//! O jogo aberto numa janela: abrir, ver e mexer na hora.

use std::time::{Duration, Instant};

use serde_json::{Value, json};

use nn_script::capture::{LineKind, LogLine};
use nn_script::remote::{Client, NOT_OPEN};

use super::jogar::{kind_lines, write_lines, write_objects, write_state_since};
use super::{Output, Tool, opt_bool, opt_num, opt_str, s_bool, s_int, s_list, s_obj, s_str};
use crate::Server;

/// Quanto `abrir_jogo` espera a janela abrir.
const OPEN_TIMEOUT: Duration = Duration::from_secs(30);

fn connect(s: &Server) -> Result<Client, String> {
    if !s.has_project() {
        return Err("a pasta ainda não tem um jogo; crie com criar_projeto".into());
    }
    Client::connect(&s.root).map_err(|e| {
        if e == NOT_OPEN {
            format!(
                "{NOT_OPEN}. Abra com abrir_jogo (ou peça para abrirem com ▶ Jogar no NN Studio, \
                 ou `nn .` na pasta). Para jogar sem janela, use rodar_jogo."
            )
        } else {
            e
        }
    })
}

fn string_list(a: &Value, key: &str) -> Result<Vec<String>, String> {
    match a.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(list)) => list
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("\"{key}\" é uma lista de textos"))
            })
            .collect(),
        Some(_) => Err(format!("\"{key}\" é uma lista de textos")),
    }
}

/// O que interessa do log do jogo que não abriu: as linhas de erro, ou as
/// últimas.
fn log_tail(path: &std::path::Path) -> String {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let errors: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|l| {
            let l = l.to_lowercase();
            l.contains("erro") || l.contains("error") || l.contains("panicked")
        })
        .take(8)
        .collect();
    if errors.is_empty() {
        lines[lines.len().saturating_sub(12)..].join("\n")
    } else {
        errors.join("\n")
    }
}

// abrir_jogo ------------------------------------------------------------------

fn abrir_jogo(s: &mut Server, a: &Value) -> Result<Output, String> {
    if !s.has_project() {
        return Err("a pasta ainda não tem um jogo; crie com criar_projeto".into());
    }
    let scene = opt_str(a, "cena")?.map(|c| c.trim_start_matches("./").replace('\\', "/"));
    if let Some(scene) = &scene
        && !s.root.join(scene).is_file()
    {
        return Err(format!("a cena {scene} não existe"));
    }
    let reopen = opt_bool(a, "reabrir")?.unwrap_or(false);
    if let Ok(mut c) = Client::connect(&s.root) {
        if !reopen {
            let st = c.request(json!({ "pedido": "estado" }))?;
            return Ok(Output::text(format!(
                "O jogo já está aberto (em {}). Use ver_jogo_aberto e mexer_no_jogo; para começar \
                 de novo, abrir_jogo com \"reabrir\": true.",
                st["cena"].as_str().unwrap_or("?")
            )));
        }
        let _ = c.request(json!({ "pedido": "fechar" }));
        // Espera a janela fechar.
        let started = Instant::now();
        while Client::connect(&s.root).is_ok() && started.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    let Some(play) = s.hooks.play else {
        return Err(
            "este servidor não sabe abrir o jogo; abra com ▶ Jogar no NN Studio ou `nn .` na pasta"
                .into(),
        );
    };
    let log = std::env::temp_dir().join(format!(
        "nn-jogo-{}.log",
        nn_script::remote::connection_file(&s.root)
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    ));
    let mut child = play(&s.root, scene.as_deref(), &log)
        .map_err(|e| format!("não foi possível abrir o jogo: {e}"))?;
    let started = Instant::now();
    let mut client = loop {
        if let Ok(Some(status)) = child.try_wait() {
            return Err(format!(
                "o jogo fechou logo ao abrir ({status}). O fim do log:\n{}",
                log_tail(&log)
            ));
        }
        if let Ok(c) = Client::connect(&s.root) {
            break c;
        }
        if started.elapsed() > OPEN_TIMEOUT {
            return Err(format!(
                "o jogo não abriu em {} s. O fim do log:\n{}",
                OPEN_TIMEOUT.as_secs(),
                log_tail(&log)
            ));
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    s.game = Some(child);
    s.live_last = None;
    let st = client.request(json!({ "pedido": "estado" }))?;
    Ok(Output::text(format!(
        "Abri o jogo numa janela, em {}. Quem está no computador vê e pode jogar junto. \
         Use ver_jogo_aberto para ver o que acontece (e a tela) e mexer_no_jogo para mudar \
         valores na hora, pausar e avançar quadros. O que muda aqui não é gravado: para ficar, \
         mude o arquivo (mudar_objeto...), e o jogo aberto recarrega sozinho.",
        st["cena"].as_str().unwrap_or("?")
    )))
}

// ver_jogo_aberto -------------------------------------------------------------

fn lines_of(st: &Value) -> Vec<LogLine> {
    st["linhas"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|l| LogLine {
                    time: l["tempo"].as_f64().unwrap_or(0.0),
                    kind: match l["tipo"].as_str() {
                        Some("erro") => LineKind::Error,
                        Some("aviso") => LineKind::Warning,
                        _ => LineKind::Print,
                    },
                    text: l["texto"].as_str().unwrap_or("").to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn ver_jogo_aberto(s: &mut Server, a: &Value) -> Result<Output, String> {
    let mut client = connect(s)?;
    let look = string_list(a, "olhar")?;
    let objects = string_list(a, "objetos")?;
    let capture = opt_bool(a, "captura")?.unwrap_or(true);
    let st = client.request(json!({ "pedido": "estado", "olhar": look }))?;
    let scene = st["cena"].as_str().unwrap_or("?").to_string();

    let mut out = format!(
        "Jogo aberto em {scene}: {:.2} s de jogo (quadro {}), {}.\n",
        st["tempo"].as_f64().unwrap_or(0.0),
        st["quadro"].as_u64().unwrap_or(0),
        if st["pausado"] == true {
            "PAUSADO (mexer_no_jogo com \"pausar\": false continua)"
        } else {
            "rodando"
        }
    );
    if let Some(e) = st["erro_na_tela"].as_str() {
        out.push_str(&format!(
            "\nA CENA ESTÁ PARADA NUM ERRO (o jogo mostra a tela de erro):\n  {}\n\
             Conserte o arquivo e salve: a cena recarrega sozinha.\n",
            e.replace('\n', "\n  ")
        ));
    }
    let since = if s.live_last.is_some() {
        "desde a última olhada"
    } else {
        "desde que o jogo abriu"
    };
    let errors = st["erros"].as_array().cloned().unwrap_or_default();
    if !errors.is_empty() {
        out.push_str(&format!("\nErros {since}:\n"));
        for e in &errors {
            out.push_str(&format!(
                "  em {}:\n    {}\n",
                e["cena"].as_str().unwrap_or("?"),
                e["mensagem"].as_str().unwrap_or("").replace('\n', "\n    ")
            ));
        }
    }
    let lines = lines_of(&st);
    write_lines(
        &mut out,
        "Erros da engine",
        &kind_lines(&lines, LineKind::Error),
    );
    write_lines(&mut out, "Avisos", &kind_lines(&lines, LineKind::Warning));
    write_lines(&mut out, "Prints", &kind_lines(&lines, LineKind::Print));
    if let Some(n) = st["linhas_de_fora"].as_u64().filter(|n| *n > 0) {
        out.push_str(&format!("  ({n} linhas mais velhas ficaram de fora)\n"));
    }
    if let Some(rules) = st["regras"].as_array().filter(|r| !r.is_empty()) {
        out.push_str(&format!("\nRegras dos eventos que dispararam {since}:\n"));
        for r in rules.iter().take(40) {
            let n = r["vezes"].as_u64().unwrap_or(1);
            out.push_str(&format!(
                "  {:.2} s  {}{}\n",
                r["primeiro"].as_f64().unwrap_or(0.0),
                r["texto"].as_str().unwrap_or(""),
                if n > 1 {
                    format!("  ({n} vezes)")
                } else {
                    String::new()
                }
            ));
        }
    }
    let state = st.get("estado").filter(|v| !v.is_null());
    let (last_scene, last_state) = match &s.live_last {
        Some((sc, v)) => (sc.clone(), Some(v.clone())),
        None => (String::new(), None),
    };
    write_state_since(
        &mut out,
        &last_scene,
        last_state.as_ref(),
        &scene,
        state,
        "desde a última olhada",
    );
    if !objects.is_empty() {
        write_objects(&mut out, state, &objects);
    }
    if let Some(looked) = st["olhar"].as_array().filter(|l| !l.is_empty()) {
        out.push_str("\nExpressões:\n");
        for l in looked {
            let expr = l["expr"].as_str().unwrap_or("");
            let v = l["valor"].as_str().unwrap_or("");
            if l["ok"] == true {
                out.push_str(&format!("  {expr} = {v}\n"));
            } else {
                out.push_str(&format!("  {expr}: erro: {v}\n"));
            }
        }
    }
    if let Some(state) = state {
        s.live_last = Some((scene.clone(), state.clone()));
    }
    let mut output = Output::text(String::new());
    if capture {
        match client.request(json!({ "pedido": "captura" })) {
            Ok(shot) => {
                out.push_str(&format!(
                    "\nA tela agora ({}x{}):",
                    shot["largura"], shot["altura"]
                ));
                output = output.and_image_base64(shot["png"].as_str().unwrap_or(""), "image/png");
            }
            Err(e) => out.push_str(&format!("\nSem a imagem da tela: {e}\n")),
        }
    }
    output.content[0] = json!({ "type": "text", "text": out });
    Ok(output)
}

// mexer_no_jogo ---------------------------------------------------------------

fn mexer_no_jogo(s: &mut Server, a: &Value) -> Result<Output, String> {
    let mut client = connect(s)?;
    let mut done = Vec::new();
    let mut failed = Vec::new();
    if let Some(list) = a.get("mudar").filter(|v| !v.is_null()) {
        let list = list
            .as_array()
            .ok_or("\"mudar\" é uma lista de { objeto, campo, valor, componente }")?;
        for m in list {
            let req = json!({
                "pedido": "mudar",
                "objeto": m["objeto"],
                "campo": m["campo"],
                "valor": m["valor"],
                "componente": m["componente"],
            });
            match client.request(req) {
                Ok(r) => done.push(format!("✓ {}", r["valor"].as_str().unwrap_or(""))),
                Err(e) => failed.push(format!("✗ mudar {}: {e}", m)),
            }
        }
    }
    if let Some(vars) = a.get("variaveis").filter(|v| !v.is_null()) {
        let vars = vars
            .as_object()
            .ok_or("\"variaveis\" é um objeto { nome: valor }")?;
        for (k, v) in vars {
            match client.request(json!({ "pedido": "variavel", "nome": k, "valor": v })) {
                Ok(_) => done.push(format!("✓ nn.vars.{k} = {v}")),
                Err(e) => failed.push(format!("✗ nn.vars.{k}: {e}")),
            }
        }
    }
    if let Some(code) = opt_str(a, "lua")? {
        match client.request(json!({ "pedido": "lua", "codigo": code })) {
            Ok(r) => done.push(format!("✓ lua: {}", r["valor"].as_str().unwrap_or("nil"))),
            Err(e) => failed.push(format!("✗ lua: {e}")),
        }
    }
    match opt_bool(a, "pausar")? {
        Some(true) => {
            client.request(json!({ "pedido": "pausar" }))?;
            done.push("✓ jogo pausado".into());
        }
        Some(false) => {
            client.request(json!({ "pedido": "continuar" }))?;
            done.push("✓ jogo continuando".into());
        }
        None => {}
    }
    if let Some(n) = opt_num(a, "avancar")? {
        let n = n.clamp(1.0, 600.0) as u64;
        client.request(json!({ "pedido": "avancar", "quadros": n }))?;
        done.push(format!(
            "✓ avançou {n} quadro(s) ({:.2} s) e ficou pausado",
            n as f64 / 60.0
        ));
    }
    if opt_bool(a, "fechar")? == Some(true) {
        client.request(json!({ "pedido": "fechar" }))?;
        s.live_last = None;
        done.push("✓ jogo fechado".into());
    }
    if done.is_empty() && failed.is_empty() {
        return Err("diga o que fazer: mudar, variaveis, lua, pausar, avancar ou fechar".into());
    }
    let mut text = done.join("\n");
    if !failed.is_empty() {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&failed.join("\n"));
        return Err(text);
    }
    let changed = ["mudar", "variaveis", "lua"]
        .iter()
        .any(|k| a.get(*k).is_some_and(|v| !v.is_null()));
    if changed {
        text.push_str(
            "\n\nNada disso foi gravado. Veja o efeito com ver_jogo_aberto; para ficar, mude os arquivos.",
        );
    }
    Ok(Output::text(text))
}

pub static TOOLS: &[&Tool] = &[
    &Tool {
        name: "abrir_jogo",
        title: "Abrir o jogo",
        description: "Abre o jogo numa janela no computador de quem está usando (para ver e jogar junto) e liga nele: depois, ver_jogo_aberto mostra o que acontece e mexer_no_jogo muda valores na hora. Também serve o jogo aberto pelo ▶ Jogar do NN Studio ou por `nn .`: se já houver um aberto, usa ele. O jogo aberto recarrega sozinho quando os arquivos mudam.",
        params: || {
            vec![
                (
                    "cena",
                    s_str("Começar nesta cena em vez da inicial (ex.: \"cenas/fase.cena\")"),
                    false,
                ),
                (
                    "reabrir",
                    s_bool("Fechar o jogo aberto e abrir de novo (padrão false)"),
                    false,
                ),
            ]
        },
        read_only: false,
        destructive: false,
        run: abrir_jogo,
    },
    &Tool {
        name: "ver_jogo_aberto",
        title: "Ver o jogo aberto",
        description: "Olha o jogo aberto numa janela (por abrir_jogo, pelo ▶ Jogar do NN Studio ou por `nn .`), enquanto alguém joga: a cena da frente, o tempo, se está pausado, os erros, avisos e prints e as regras dos eventos que dispararam desde a última olhada, o que mudou nos objetos e nas variáveis desde a última olhada, e a tela agora (imagem).",
        params: || {
            vec![
                (
                    "objetos",
                    s_list(
                        "Nomes de objetos para ver por inteiro (posição, propriedades dos comportamentos)",
                        json!({ "type": "string" }),
                    ),
                    false,
                ),
                (
                    "olhar",
                    s_list(
                        "Expressões Lua para ver agora, com self = a cena da frente (ex.: \"self.objects.heroi.body.vy\", \"nn.vars.vidas\")",
                        json!({ "type": "string" }),
                    ),
                    false,
                ),
                (
                    "captura",
                    s_bool("Mandar a imagem da tela (padrão true)"),
                    false,
                ),
            ]
        },
        read_only: true,
        destructive: false,
        run: ver_jogo_aberto,
    },
    &Tool {
        name: "mexer_no_jogo",
        title: "Mexer no jogo aberto",
        description: "Muda o jogo aberto na hora, sem gravar nada (para experimentar um valor antes de mudar o arquivo): \"mudar\" campos de objetos e componentes ([{\"objeto\": \"heroi\", \"componente\": \"nn:plataforma\", \"campo\": \"pulo\", \"valor\": 1000}]; sem componente, o próprio objeto: x, y, visible...), \"variaveis\" do jogo ({\"vidas\": 9}), rodar \"lua\" com self = a cena da frente (\"self.objects.heroi:move_to(0, 0)\"), \"pausar\" (true/false), \"avancar\" N quadros com o jogo pausado (60 = 1 s) e \"fechar\" o jogo.",
        params: || {
            vec![
                (
                    "mudar",
                    s_list(
                        "Campos a mudar: { objeto (nome ou id), campo, valor, componente (tipo ou script, opcional) }",
                        s_obj(
                            "{ \"objeto\": ..., \"campo\": ..., \"valor\": ..., \"componente\": ... }",
                        ),
                    ),
                    false,
                ),
                (
                    "variaveis",
                    s_obj("Variáveis do jogo (nn.vars) a mudar: { nome: valor }"),
                    false,
                ),
                (
                    "lua",
                    s_str(
                        "Código Lua para rodar agora, com self = a cena da frente; volta o valor",
                    ),
                    false,
                ),
                ("pausar", s_bool("true pausa, false continua"), false),
                (
                    "avancar",
                    s_int("Avança este número de quadros e fica pausado (60 = 1 s)"),
                    false,
                ),
                ("fechar", s_bool("Fecha o jogo"), false),
            ]
        },
        read_only: false,
        destructive: false,
        run: mexer_no_jogo,
    },
];
