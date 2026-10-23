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

/// Palavras que não ajudam a achar nada.
const STOP_WORDS: &[&str] = &[
    "a", "o", "as", "os", "de", "do", "da", "dos", "das", "e", "em", "no", "na", "nos", "nas",
    "um", "uma", "com", "para", "por", "que", "como", "se", "ao", "the", "of",
];

/// As palavras da busca, sem acento e sem as que não ajudam.
fn query_words(query: &str) -> Vec<String> {
    let all: Vec<String> = fold(query).split_whitespace().map(str::to_string).collect();
    let useful: Vec<String> = all
        .iter()
        .filter(|w| !STOP_WORDS.contains(&w.as_str()))
        .cloned()
        .collect();
    if useful.is_empty() { all } else { useful }
}

/// Quantas das palavras aparecem no texto.
fn hits(hay: &str, words: &[String]) -> usize {
    words.iter().filter(|w| hay.contains(w.as_str())).count()
}

/// O tamanho de uma seção que vai inteira na busca.
const WHOLE_SECTION: usize = 5_000;
/// O máximo de trechos de uma seção grande.
const MAX_EXCERPT: usize = 6_000;

/// Os trechos de uma seção grande que citam as palavras: os parágrafos e,
/// nas tabelas, o cabeçalho e as linhas que citam.
fn excerpt(body: &str, words: &[String]) -> (String, bool) {
    if body.len() <= WHOLE_SECTION {
        return (body.to_string(), false);
    }
    let mut out = String::new();
    let mut first = true;
    for block in body.split("\n\n") {
        let block = block.trim_matches('\n');
        if block.is_empty() {
            continue;
        }
        let lines: Vec<&str> = block.lines().collect();
        let piece = if lines.first().is_some_and(|l| l.starts_with('|')) && lines.len() > 2 {
            let rows: Vec<&str> = lines[2..]
                .iter()
                .copied()
                .filter(|l| hits(&fold(l), words) > 0)
                .collect();
            if rows.is_empty() {
                continue;
            }
            format!("{}\n{}\n{}", lines[0], lines[1], rows.join("\n"))
        } else if first || hits(&fold(block), words) > 0 {
            // O primeiro parágrafo (o título) vai sempre.
            block.to_string()
        } else {
            continue;
        };
        first = false;
        if out.len() + piece.len() > MAX_EXCERPT {
            if out.len() < MAX_EXCERPT / 2 {
                let cut = (0..=MAX_EXCERPT - out.len())
                    .rev()
                    .find(|i| piece.is_char_boundary(*i))
                    .unwrap_or(0);
                out.push_str(&piece[..cut]);
                out.push_str(" [...]\n\n");
            }
            break;
        }
        out.push_str(&piece);
        out.push_str("\n\n");
    }
    (out, true)
}

fn buscar_documentacao(_: &mut Server, a: &Value) -> Result<Output, String> {
    // Uma seção inteira, pelo título.
    if let Some(want) = opt_str(a, "secao")? {
        let want = fold(want);
        let found = sections()
            .into_iter()
            .find(|(_, t, _)| fold(t) == want)
            .or_else(|| {
                sections()
                    .into_iter()
                    .find(|(_, t, _)| fold(t).contains(&want))
            });
        return match found {
            Some((file, title, mut body)) => {
                if body.len() > 20_000 {
                    let cut = (0..=20_000)
                        .rev()
                        .find(|i| body.is_char_boundary(*i))
                        .unwrap_or(0);
                    body.truncate(cut);
                    body.push_str(&format!(
                        "\n[...] (o resto está no recurso nn://docs/{file})\n"
                    ));
                }
                Ok(Output::text(format!("--- {file} › {title} ---\n{body}")))
            }
            None => Err(format!(
                "não há seção \"{}\"; use o título que aparece depois de › numa busca",
                opt_str(a, "secao")?.unwrap_or("")
            )),
        };
    }
    let query = arg_str(a, "texto")?;
    let words = query_words(query);
    if words.is_empty() {
        return Err("diga o que buscar (ex.: \"nn.draw.sprite\", \"pular\", \"prefab\")".into());
    }
    let max_sections = opt_num(a, "secoes")?.unwrap_or(3.0).clamp(0.0, 10.0) as usize;
    // Primeiro o que cita todas as palavras; se nada citar, o que cita mais.
    let mut out = String::new();
    let mut partial = false;
    for need in [words.len(), 1] {
        out = search(&words, need, max_sections);
        if !out.is_empty() {
            partial = need < words.len();
            break;
        }
    }
    if out.is_empty() {
        return Err(format!(
            "nada sobre \"{query}\". Tente outra palavra (os documentos são em português: \"pular\", \"colisão\", \"câmera\")"
        ));
    }
    if partial {
        out = format!(
            "Nada cita todas as palavras de \"{query}\"; estes citam parte delas.\n\n{out}"
        );
    }
    Ok(Output::text(out))
}

/// A busca com pelo menos `need` das palavras em cada resultado.
fn search(words: &[String], need: usize, max_sections: usize) -> String {
    let mut out = String::new();
    // API Lua (nn.*).
    let mut api: Vec<(usize, &api_docs::ApiDoc)> = api_docs::API_DOCS
        .iter()
        .filter_map(|d| {
            let n = hits(&fold(&format!("{} {}", d.name, d.doc)), words);
            (n >= need).then_some((n, d))
        })
        .collect();
    api.sort_by_key(|(n, _)| std::cmp::Reverse(*n));
    let mut members: Vec<(usize, &str, &api_docs::Member)> = [
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
    .filter_map(|(owner, m)| {
        let n = hits(&fold(&format!("{owner} {} {}", m.name, m.doc)), words);
        (n >= need).then_some((n, owner, m))
    })
    .collect();
    members.sort_by_key(|(n, _, _)| std::cmp::Reverse(*n));
    if !api.is_empty() || !members.is_empty() {
        out.push_str("API Lua:\n");
        for (_, d) in api.iter().take(20) {
            out.push_str(&format!("- {}{} — {}\n", d.name, d.args, d.doc));
        }
        for (_, owner, m) in members.iter().take(15) {
            let sep = if m.args.is_empty() { "." } else { ":" };
            out.push_str(&format!("- {owner}{sep}{}{} — {}\n", m.name, m.args, m.doc));
        }
    }
    // Seções da documentação: as que citam mais palavras, e mais vezes.
    let mut scored: Vec<(usize, (&str, String, String))> = sections()
        .into_iter()
        .filter_map(|sec| {
            let title = fold(&sec.1);
            let body = fold(&sec.2);
            let n = hits(&body, words);
            if n < need {
                return None;
            }
            let count: usize = words
                .iter()
                .map(|w| body.matches(w.as_str()).count() + 20 * title.matches(w.as_str()).count())
                .sum();
            // Seções enormes citam tudo: valem um pouco menos.
            let count = count * 4000 / body.len().max(4000);
            Some((n * 100_000 + count, sec))
        })
        .collect();
    scored.sort_by_key(|s| std::cmp::Reverse(s.0));
    let total = scored.len();
    for (_, (file, title, body)) in scored.into_iter().take(max_sections) {
        let (text, cut) = excerpt(&body, words);
        out.push_str(&format!("\n--- {file} › {title} ---\n{}", text.trim_end()));
        if cut {
            out.push_str(&format!(
                "\n[só os trechos que citam a busca; a seção inteira: buscar_documentacao com \"secao\": \"{title}\"]"
            ));
        }
        out.push('\n');
    }
    if total > max_sections && !out.is_empty() {
        out.push_str(&format!(
            "\n({total} seções citam isso; mostrei {max_sections}. Use secoes para ver mais.)"
        ));
    }
    out
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
        description: "Busca na documentação da nn-engine: a API Lua (nn.draw, nn.input, nn.physics, nn.d3, nn.scene...), os campos dos objetos nos scripts, e as seções dos guias (Lua, 3D, NN Studio): cenas, comportamentos, prefabs, eventos, física, animação, interface. Mostra os trechos que citam as palavras (se nada citar todas, os que citam parte); \"secao\" mostra uma seção inteira. Use antes de escrever código Lua.",
        params: || {
            vec![
                (
                    "texto",
                    s_str(
                        "Palavras ou um nome da API (ex.: \"nn.input\", \"comportamento props\", \"pular\")",
                    ),
                    false,
                ),
                (
                    "secao",
                    s_str(
                        "Em vez de buscar, mostra uma seção inteira pelo título (o que vem depois de › numa busca)",
                    ),
                    false,
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
