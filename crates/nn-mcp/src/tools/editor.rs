//! O NN Studio aberto no mesmo projeto: o que a pessoa vê (`ver_editor`),
//! não gravar por cima do que ela ainda não salvou e avisar no editor o que
//! o agente mudou.

use serde_json::{Value, json};

use nn_script::remote::{Client, EDITOR};

use super::{Output, Tool, opt_bool, s_bool};
use crate::Server;

impl Server {
    /// Um pedido ao NN Studio com este projeto aberto (`None` se ele não está
    /// aberto). Liga de novo uma vez se a ligação caiu.
    pub(crate) fn editor_request(&mut self, req: Value) -> Option<Value> {
        for _ in 0..2 {
            if self.editor.is_none() {
                self.editor = Client::connect_to(&self.root, EDITOR).ok();
            }
            let client = self.editor.as_mut()?;
            match client.request(req.clone()) {
                Ok(v) => return Some(v),
                Err(_) => self.editor = None,
            }
        }
        None
    }

    /// Antes de uma ferramenta que grava: os arquivos que o NN Studio tem
    /// com mudanças não gravadas.
    pub(crate) fn fetch_unsaved(&mut self) {
        self.unsaved = self
            .editor_request(json!({ "pedido": "editor" }))
            .and_then(|v| v["editor"]["nao_gravados"].as_array().cloned())
            .map(|l| {
                l.iter()
                    .filter_map(|f| f.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
    }

    /// `Err` se o NN Studio tem mudanças não gravadas em `rel`: gravar por
    /// cima perderia o que a pessoa fez.
    pub(crate) fn check_unsaved(&self, rel: &str) -> Result<(), String> {
        if self.unsaved.iter().any(|f| f == rel) {
            return Err(format!(
                "o NN Studio está com mudanças ainda não gravadas em {rel}: gravar agora apagaria o \
                 que a pessoa fez. Peça para ela gravar (Ctrl+S) ou descartar essas mudanças e \
                 tente de novo."
            ));
        }
        Ok(())
    }

    /// Depois de uma ferramenta que gravou: conta ao NN Studio (vira uma
    /// linha ✨ no Console).
    pub(crate) fn notify_editor(&mut self, tool: &str, files: Vec<String>, text: &str) {
        if self.editor.is_none()
            && !nn_script::remote::connection_file_for(&self.root, EDITOR).is_file()
        {
            return;
        }
        let first = text
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(200)
            .collect::<String>();
        let _ = self.editor_request(json!({
            "pedido": "mudou",
            "ferramenta": tool,
            "arquivos": files,
            "texto": first,
        }));
    }
}

fn ver_editor(s: &mut Server, a: &Value) -> Result<Output, String> {
    let capture = opt_bool(a, "captura")?.unwrap_or(true);
    let Some(v) = s.editor_request(json!({ "pedido": "editor" })) else {
        return Err(
            "o NN Studio não está aberto com este projeto. Sem ele, o agente trabalha só pelos \
             arquivos (ver_projeto, ver_cena); para o jogo numa janela, abrir_jogo."
                .into(),
        );
    };
    let e = &v["editor"];
    let mut out = format!(
        "NN Studio aberto com \"{}\".\n",
        e["projeto"].as_str().unwrap_or("?")
    );
    match e["aba"].as_object() {
        Some(tab) => out.push_str(&format!(
            "Aba da frente: {} ({}).\n",
            tab.get("arquivo").and_then(Value::as_str).unwrap_or("?"),
            tab.get("tipo").and_then(Value::as_str).unwrap_or("?")
        )),
        None => out.push_str("Nenhum arquivo na frente (o Início ou um painel).\n"),
    }
    if let Some(scene) = e["cena"].as_object() {
        let rel = scene.get("arquivo").and_then(Value::as_str).unwrap_or("?");
        if let Some(err) = scene.get("erro").and_then(Value::as_str) {
            out.push_str(&format!("A cena {rel} tem um erro no editor: {err}\n"));
        }
        let chosen = scene
            .get("escolhidos")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if chosen.is_empty() {
            out.push_str(&format!("Na cena {rel}, nenhum objeto escolhido.\n"));
        } else {
            out.push_str(&format!(
                "Escolhidos na cena {rel} (é deles que a pessoa fala quando diz \"este\"):\n"
            ));
            for o in &chosen {
                let mut line = format!(
                    "  {} (id {}): x {}, y {}",
                    o["nome"].as_str().unwrap_or("?"),
                    o["id"],
                    o["x"],
                    o["y"]
                );
                if !o["z"].is_null() {
                    line.push_str(&format!(", z {}", o["z"]));
                }
                if let Some(p) = o["prefab"].as_str() {
                    line.push_str(&format!(", cópia de {p}"));
                }
                let comps: Vec<&str> = o["componentes"]
                    .as_array()
                    .map(|l| l.iter().filter_map(Value::as_str).collect())
                    .unwrap_or_default();
                if !comps.is_empty() {
                    line.push_str(&format!(" — {}", comps.join(", ")));
                }
                out.push_str(&line);
                out.push('\n');
            }
            out.push_str("  (ver_cena com \"objeto\" mostra tudo de um deles.)\n");
        }
    }
    let unsaved: Vec<&str> = e["nao_gravados"]
        .as_array()
        .map(|l| l.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if unsaved.is_empty() {
        out.push_str("Tudo gravado.\n");
    } else {
        out.push_str(&format!(
            "Mudanças NÃO gravadas (o servidor não grava por cima delas): {}\n",
            unsaved.join(", ")
        ));
    }
    let tabs: Vec<&str> = e["abas"]
        .as_array()
        .map(|l| l.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if !tabs.is_empty() {
        out.push_str(&format!("Abas abertas: {}\n", tabs.join(", ")));
    }
    if e["jogo_aberto"] == true {
        out.push_str("O jogo está aberto pelo ▶ Jogar (veja com ver_jogo_aberto).\n");
    }
    let mut output = Output::default();
    if capture {
        match s.editor_request(json!({ "pedido": "captura" })) {
            Some(shot) if shot["png"].is_string() => {
                out.push_str(&format!(
                    "\nA janela do NN Studio agora ({}x{}):",
                    shot["largura"], shot["altura"]
                ));
                output = output
                    .and_text(out)
                    .and_image_base64(shot["png"].as_str().unwrap_or(""), "image/png");
                return Ok(output);
            }
            Some(shot) => out.push_str(&format!(
                "\n(Sem a imagem da janela: {}.)",
                shot["erro"].as_str().unwrap_or("não veio")
            )),
            None => out.push_str("\n(Sem a imagem da janela.)"),
        }
    }
    Ok(output.and_text(out))
}

pub static TOOLS: &[&Tool] = &[&Tool {
    name: "ver_editor",
    title: "Ver o NN Studio",
    description: "Olha o NN Studio aberto com este projeto, para trabalhar junto com a pessoa: a aba da frente, os objetos escolhidos na cena (\"deixa este inimigo mais rápido\" fala deles), os arquivos com mudanças ainda não gravadas (o servidor não grava por cima delas) e a imagem da janela. Sem o NN Studio aberto, diz isso.",
    params: || {
        vec![(
            "captura",
            s_bool("Mandar a imagem da janela do NN Studio (padrão true)"),
            false,
        )]
    },
    read_only: true,
    destructive: false,
    run: ver_editor,
}];
