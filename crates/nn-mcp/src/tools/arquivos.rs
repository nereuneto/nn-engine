//! Arquivos do projeto: listar, ler, escrever (conferindo) e desfazer.

use serde_json::Value;

use super::{Output, Tool, arg_str, opt_bool, opt_num, opt_str, s_bool, s_int, s_str};
use crate::Server;

/// Maior arquivo de texto devolvido de uma vez.
const MAX_TEXT: usize = 200_000;
/// Maior imagem devolvida.
const MAX_IMAGE: u64 = 4_000_000;

fn listar_arquivos(s: &mut Server, a: &Value) -> Result<Output, String> {
    let under = match opt_str(a, "pasta")? {
        Some(p) if !p.trim().is_empty() && p.trim() != "." => {
            let (rel, full) = crate::paths::resolve(&s.root, p)?;
            if !full.is_dir() {
                return Err(format!("{rel} não é uma pasta"));
            }
            rel
        }
        _ => String::new(),
    };
    let files = crate::paths::list_files(&s.root, &under);
    if files.is_empty() {
        return Ok(Output::text("(nenhum arquivo)"));
    }
    let lines: Vec<String> = files
        .iter()
        .map(|f| {
            let size = std::fs::metadata(s.root.join(f))
                .map(|m| m.len())
                .unwrap_or(0);
            format!("{f} ({})", human(size))
        })
        .collect();
    Ok(Output::text(format!(
        "{} arquivo(s):\n{}",
        files.len(),
        lines.join("\n")
    )))
}

fn human(n: u64) -> String {
    if n < 1024 {
        format!("{n} B")
    } else if n < 1024 * 1024 {
        format!("{:.1} KB", n as f64 / 1024.0)
    } else {
        format!("{:.1} MB", n as f64 / 1024.0 / 1024.0)
    }
}

fn ler_arquivo(s: &mut Server, a: &Value) -> Result<Output, String> {
    let (rel, full) = crate::paths::resolve(&s.root, arg_str(a, "caminho")?)?;
    if !full.is_file() {
        return Err(format!("{rel} não existe (veja listar_arquivos)"));
    }
    let lower = rel.to_lowercase();
    let size = std::fs::metadata(&full).map(|m| m.len()).unwrap_or(0);
    if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        if size > MAX_IMAGE {
            return Err(format!(
                "{rel} é grande demais para mostrar ({})",
                human(size)
            ));
        }
        let data = std::fs::read(&full).map_err(|e| format!("{rel}: {e}"))?;
        let mime = if lower.ends_with(".png") {
            "image/png"
        } else {
            "image/jpeg"
        };
        let dims = image_size(&data)
            .map(|(w, h)| format!(", {w}x{h} pixels"))
            .unwrap_or_default();
        return Ok(Output::text(format!("{rel} ({}{dims})", human(size))).and_image(&data, mime));
    }
    let data = std::fs::read(&full).map_err(|e| format!("{rel}: {e}"))?;
    let Ok(text) = String::from_utf8(data) else {
        return Ok(Output::text(format!(
            "{rel} é um arquivo binário ({}); não dá para mostrar como texto",
            human(size)
        )));
    };
    let from = opt_num(a, "linha")?.unwrap_or(1.0).max(1.0) as usize;
    let count = opt_num(a, "linhas")?.map(|n| n.max(1.0) as usize);
    let total = text.lines().count();
    let mut out = String::new();
    let mut shown = 0;
    for line in text
        .lines()
        .skip(from - 1)
        .take(count.unwrap_or(usize::MAX))
    {
        if out.len() + line.len() > MAX_TEXT {
            break;
        }
        out.push_str(line);
        out.push('\n');
        shown += 1;
    }
    let last = from - 1 + shown;
    if from > 1 || last < total {
        out = format!("[{rel}: linhas {from} a {last} de {total}]\n{out}");
    }
    Ok(Output::text(out))
}

/// Largura e altura de um PNG (do cabeçalho).
pub(super) fn image_size(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() > 24 && &data[..8] == b"\x89PNG\r\n\x1a\n" {
        let w = u32::from_be_bytes(data[16..20].try_into().ok()?);
        let h = u32::from_be_bytes(data[20..24].try_into().ok()?);
        return Some((w, h));
    }
    None
}

/// Confere o conteúdo antes de gravar, pelo tipo do arquivo.
fn check_content(rel: &str, text: &str) -> Result<Option<String>, String> {
    if rel.ends_with(".lua") {
        let lua = mlua::Lua::new();
        if let Some(issue) = nn_projeto::lua_syntax::check(&lua, rel, text) {
            return Err(format!(
                "não gravei {rel}: erro de sintaxe na linha {}: {}",
                issue.line, issue.message
            ));
        }
        if rel == "config.lua" {
            nn_projeto::config_lua::read(text, "")
                .map_err(|e| format!("não gravei config.lua: {e}"))?;
        }
    } else if rel.ends_with(".cena") {
        nn_projeto::scene_data::read(text, rel)
            .map_err(|e| format!("não gravei {rel}: a cena não abre: {e}"))?;
        return Ok(Some(
            "Dica: criar_objeto, mudar_objeto e por_regra mudam cenas conferindo cada campo."
                .into(),
        ));
    } else if rel.ends_with(".prefab") {
        nn_projeto::scene_data::read_prefab(text, rel)
            .map_err(|e| format!("não gravei {rel}: o prefab não abre: {e}"))?;
    } else if rel.ends_with(".json") || rel.ends_with(".tmj") || rel.ends_with(".tsj") {
        serde_json::from_str::<Value>(text)
            .map_err(|e| format!("não gravei {rel}: JSON inválido: {e}"))?;
    }
    Ok(None)
}

fn escrever_arquivo(s: &mut Server, a: &Value) -> Result<Output, String> {
    let (rel, full) = crate::paths::resolve(&s.root, arg_str(a, "caminho")?)?;
    let text = arg_str(a, "conteudo")?;
    if full.is_dir() {
        return Err(format!("{rel} é uma pasta"));
    }
    let lower = rel.to_lowercase();
    if [
        ".png", ".jpg", ".jpeg", ".wav", ".ogg", ".mp3", ".glb", ".ttf",
    ]
    .iter()
    .any(|e| lower.ends_with(e))
    {
        return Err(format!(
            "{rel} é um arquivo binário; escrever_arquivo grava só texto"
        ));
    }
    let existed = full.exists();
    if existed && opt_bool(a, "so_se_novo")?.unwrap_or(false) {
        return Err(format!("{rel} já existe"));
    }
    let tip = check_content(&rel, text)?;
    s.write_file(&rel, text.as_bytes())?;
    let mut msg = format!(
        "{} {rel} ({} linhas).",
        if existed { "Gravei" } else { "Criei" },
        text.lines().count()
    );
    if rel.ends_with(".lua") && !rel.ends_with("config.lua") {
        msg.push_str(" Sintaxe ok.");
        // Um comportamento: confere o que ele declara.
        if text.contains("props") || text.contains("title") {
            let info = s.catalog.get(&s.root.clone(), &rel);
            match &info.error {
                None if !info.title.is_empty() => msg.push_str(&format!(
                    " Comportamento \"{}\" com {} propriedade(s); ponha num objeto com {{ \"type\": \"behavior\", \"script\": \"{rel}\" }}.",
                    info.title,
                    info.props.len()
                )),
                Some(e) if text.contains("props") => {
                    msg.push_str(&format!(" Aviso: como comportamento, ele dá erro: {e}"));
                }
                _ => {}
            }
        }
    }
    if let Some(t) = tip {
        msg.push(' ');
        msg.push_str(&t);
    }
    Ok(Output::text(msg))
}

fn apagar_arquivo(s: &mut Server, a: &Value) -> Result<Output, String> {
    let (rel, full) = crate::paths::resolve(&s.root, arg_str(a, "caminho")?)?;
    if !full.exists() {
        return Err(format!("{rel} não existe"));
    }
    if rel == "config.lua" {
        return Err("o config.lua é o jogo; não apago".into());
    }
    s.delete_file(&rel)?;
    // Quem usa o arquivo?
    let mut users = Vec::new();
    for f in crate::paths::list_files(&s.root, "") {
        if (f.ends_with(".cena") || f.ends_with(".prefab") || f.ends_with(".lua"))
            && std::fs::read_to_string(s.root.join(&f)).is_ok_and(|t| t.contains(&rel))
        {
            users.push(f);
        }
    }
    let mut msg = format!("Apaguei {rel} (dá para desfazer).");
    if !users.is_empty() {
        msg.push_str(&format!(
            " Atenção: ele ainda é citado em {}.",
            users.join(", ")
        ));
    }
    Ok(Output::text(msg))
}

fn desfazer(s: &mut Server, a: &Value) -> Result<Output, String> {
    if opt_bool(a, "listar")?.unwrap_or(false) {
        let list = s.history.list();
        if list.is_empty() {
            return Ok(Output::text("Nada feito por aqui ainda."));
        }
        let lines: Vec<String> = list
            .iter()
            .enumerate()
            .map(|(i, (label, files))| format!("{}. {label}: {}", i + 1, files.join(", ")))
            .collect();
        return Ok(Output::text(format!(
            "Mudanças, da mais nova para a mais antiga:\n{}",
            lines.join("\n")
        )));
    }
    let steps = opt_num(a, "passos")?.unwrap_or(1.0).clamp(1.0, 100.0) as usize;
    let force = opt_bool(a, "forcar")?.unwrap_or(false);
    let mut done = Vec::new();
    for _ in 0..steps {
        if s.history.is_empty() {
            break;
        }
        if let Some((_, files)) = s.history.list().into_iter().next() {
            for f in &files {
                s.check_unsaved(f)?;
            }
        }
        done.push(s.history.undo(&s.root.clone(), force)?);
    }
    if done.is_empty() {
        return Err("não há nada feito por aqui para desfazer".into());
    }
    Ok(Output::text(done.join("\n")))
}

pub static TOOLS: &[&Tool] = &[
    &Tool {
        name: "listar_arquivos",
        title: "Listar arquivos",
        description: "Lista os arquivos do projeto (ou de uma pasta dele), com o tamanho.",
        params: || vec![("pasta", s_str("Só esta pasta (ex.: assets/imagens)"), false)],
        read_only: true,
        destructive: false,
        run: listar_arquivos,
    },
    &Tool {
        name: "ler_arquivo",
        title: "Ler arquivo",
        description: "Lê um arquivo do projeto. Texto (Lua, cenas, mapas .tmj, diálogos): devolve o conteúdo, com \"linha\" e \"linhas\" para um pedaço. Imagens .png/.jpg: devolve a imagem, para você ver (e o tamanho em pixels, útil para frame_w/frame_h de spritesheets).",
        params: || {
            vec![
                (
                    "caminho",
                    s_str("Arquivo, relativo à pasta do projeto"),
                    true,
                ),
                ("linha", s_int("Primeira linha (texto)"), false),
                ("linhas", s_int("Quantas linhas (texto)"), false),
            ]
        },
        read_only: true,
        destructive: false,
        run: ler_arquivo,
    },
    &Tool {
        name: "escrever_arquivo",
        title: "Escrever arquivo",
        description: "Cria ou substitui um arquivo de texto do projeto (scripts Lua, mapas, diálogos, LEIAME). Confere antes de gravar: Lua com erro de sintaxe não é gravado (vem a linha e o erro em português); cenas e prefabs precisam abrir; JSON precisa ser válido. Num script de comportamento, diz o que ele declara. Para mudar cenas, prefira criar_objeto/mudar_objeto/por_regra.",
        params: || {
            vec![
                (
                    "caminho",
                    s_str("Arquivo, relativo à pasta do projeto (ex.: scripts/inimigo.lua)"),
                    true,
                ),
                ("conteudo", s_str("O conteúdo inteiro do arquivo"), true),
                (
                    "so_se_novo",
                    s_bool("Não substitui um arquivo que já existe"),
                    false,
                ),
            ]
        },
        read_only: false,
        destructive: true,
        run: escrever_arquivo,
    },
    &Tool {
        name: "apagar_arquivo",
        title: "Apagar arquivo",
        description: "Apaga um arquivo do projeto (dá para desfazer) e avisa se alguma cena ou script ainda o cita.",
        params: || {
            vec![(
                "caminho",
                s_str("Arquivo, relativo à pasta do projeto"),
                true,
            )]
        },
        read_only: false,
        destructive: true,
        run: apagar_arquivo,
    },
    &Tool {
        name: "desfazer",
        title: "Desfazer",
        description: "Desfaz as últimas mudanças feitas por estas ferramentas (cada chamada que mudou arquivos é um passo; vale enquanto o servidor estiver aberto). Se o arquivo mudou depois por fora (no NN Studio, à mão), só desfaz com forcar = true. Com listar = true, só mostra os passos.",
        params: || {
            vec![
                ("passos", s_int("Quantos passos desfazer (padrão 1)"), false),
                (
                    "forcar",
                    s_bool("Desfazer mesmo se o arquivo mudou por fora"),
                    false,
                ),
                ("listar", s_bool("Só listar o que dá para desfazer"), false),
            ]
        },
        read_only: false,
        destructive: true,
        run: desfazer,
    },
];
