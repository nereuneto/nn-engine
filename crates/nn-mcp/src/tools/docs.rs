//! Documentação e conferência de Lua.

use serde_json::Value;

use nn_script::api_docs;

use super::{Output, Tool, arg_str, opt_num, opt_str, s_int, s_str};
use crate::Server;
use crate::paths::fold;

/// Uma seção de um documento: (arquivo, título, texto).
fn sections() -> Vec<(&'static str, String, String)> {
    let mut out = Vec::new();
    for (name, text) in crate::docs::DOCS.iter().copied() {
        let mut title = String::new();
        let mut body = String::new();
        for line in text.lines() {
            if line.starts_with("## ") || line.starts_with("### ") {
                if !body.trim().is_empty() {
                    out.push((name, title.clone(), std::mem::take(&mut body)));
                }
                body.clear();
                title = line.trim_start_matches('#').trim().to_string();
            }
            body.push_str(line);
            body.push('\n');
        }
        if !body.trim().is_empty() {
            out.push((name, title, body));
        }
    }
    out
}

fn buscar_documentacao(_: &mut Server, a: &Value) -> Result<Output, String> {
    let query = arg_str(a, "texto")?;
    let words: Vec<String> = fold(query).split_whitespace().map(str::to_string).collect();
    if words.is_empty() {
        return Err("diga o que buscar (ex.: \"nn.draw.sprite\", \"pular\", \"prefab\")".into());
    }
    let max_sections = opt_num(a, "secoes")?.unwrap_or(3.0).clamp(0.0, 10.0) as usize;
    let mut out = String::new();
    // API Lua (nn.*).
    let api: Vec<&api_docs::ApiDoc> = api_docs::API_DOCS
        .iter()
        .filter(|d| {
            let hay = fold(&format!("{} {}", d.name, d.doc));
            words.iter().all(|w| hay.contains(w.as_str()))
        })
        .take(30)
        .collect();
    let members: Vec<(&str, &api_docs::Member)> = [
        ("objeto", api_docs::OBJECT_MEMBERS),
        ("corpo (objeto.body)", api_docs::BODY_MEMBERS),
        ("cena (self.scene)", api_docs::SCENE_MEMBERS),
        ("comportamento (self)", api_docs::BEHAVIOR_MEMBERS),
        ("sprite", api_docs::SPRITE_MEMBERS),
        ("texto", api_docs::TEXT_MEMBERS),
        ("forma", api_docs::SHAPE_MEMBERS),
    ]
    .into_iter()
    .flat_map(|(owner, list)| list.iter().map(move |m| (owner, m)))
    .filter(|(owner, m)| {
        let hay = fold(&format!("{owner} {} {}", m.name, m.doc));
        words.iter().all(|w| hay.contains(w.as_str()))
    })
    .take(20)
    .collect();
    if !api.is_empty() || !members.is_empty() {
        out.push_str("API Lua:\n");
        for d in &api {
            out.push_str(&format!("- {}{} — {}\n", d.name, d.args, d.doc));
        }
        for (owner, m) in &members {
            let sep = if m.args.is_empty() { "." } else { ":" };
            out.push_str(&format!("- {owner}{sep}{}{} — {}\n", m.name, m.args, m.doc));
        }
    }
    // Seções da documentação, das que mais citam as palavras.
    let mut scored: Vec<(usize, (&str, String, String))> = sections()
        .into_iter()
        .filter_map(|sec| {
            let title = fold(&sec.1);
            let body = fold(&sec.2);
            if !words.iter().all(|w| body.contains(w.as_str())) {
                return None;
            }
            let score: usize = words
                .iter()
                .map(|w| body.matches(w.as_str()).count() + 20 * title.matches(w.as_str()).count())
                .sum();
            Some((score, sec))
        })
        .collect();
    scored.sort_by_key(|s| std::cmp::Reverse(s.0));
    let total = scored.len();
    for (_, (file, title, body)) in scored.into_iter().take(max_sections) {
        let mut body = body;
        if body.len() > 12_000 {
            let cut = (0..=12_000)
                .rev()
                .find(|i| body.is_char_boundary(*i))
                .unwrap_or(0);
            body.truncate(cut);
            body.push_str("\n[...] (seção longa: leia o recurso inteiro)\n");
        }
        out.push_str(&format!("\n--- {file} › {title} ---\n{body}"));
    }
    if out.is_empty() {
        return Err(format!(
            "nada sobre \"{query}\". Tente outra palavra (os documentos são em português: \"pular\", \"colisão\", \"câmera\")"
        ));
    }
    if total > max_sections {
        out.push_str(&format!(
            "\n({total} seções citam isso; mostrei {max_sections}. Use secoes para ver mais.)"
        ));
    }
    Ok(Output::text(out))
}

fn conferir_lua(s: &mut Server, a: &Value) -> Result<Output, String> {
    let (name, code) = match (opt_str(a, "codigo")?, opt_str(a, "caminho")?) {
        (Some(code), name) => (name.unwrap_or("trecho.lua").to_string(), code.to_string()),
        (None, Some(path)) => s.read_text(path)?,
        (None, None) => return Err("passe \"codigo\" ou \"caminho\"".into()),
    };
    let lua = mlua::Lua::new();
    match nn_projeto::lua_syntax::check(&lua, &name, &code) {
        Some(issue) => Err(format!("{name}:{}: {}", issue.line, issue.message)),
        None => Ok(Output::text(format!(
            "{name}: sintaxe ok ({} linhas). (Erros que só aparecem rodando o jogo não aparecem aqui.)",
            code.lines().count()
        ))),
    }
}

pub static TOOLS: &[&Tool] = &[
    &Tool {
        name: "buscar_documentacao",
        title: "Buscar na documentação",
        description: "Busca na documentação da nn-engine: a API Lua (nn.draw, nn.input, nn.physics, nn.d3, nn.scene...), os campos dos objetos nos scripts, e as seções dos guias (Lua, 3D, NN Studio): cenas, comportamentos, prefabs, eventos, física, animação, interface. Use antes de escrever código Lua.",
        params: || {
            vec![
                (
                    "texto",
                    s_str(
                        "Palavras ou um nome da API (ex.: \"nn.input\", \"comportamento props\", \"pular\")",
                    ),
                    true,
                ),
                (
                    "secoes",
                    s_int("Quantas seções dos guias mostrar (padrão 3)"),
                    false,
                ),
            ]
        },
        read_only: true,
        destructive: false,
        run: buscar_documentacao,
    },
    &Tool {
        name: "conferir_lua",
        title: "Conferir Lua",
        description: "Confere a sintaxe de um código Lua (ou de um arquivo do projeto) sem gravar nem rodar; o erro vem com a linha, em português.",
        params: || {
            vec![
                ("codigo", s_str("O código"), false),
                ("caminho", s_str("Ou um arquivo .lua do projeto"), false),
            ]
        },
        read_only: true,
        destructive: false,
        run: conferir_lua,
    },
];
