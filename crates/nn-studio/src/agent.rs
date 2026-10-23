//! O agente de IA no NN Studio.
//!
//! - **Ligar um agente** (Jogo > Ligar um agente de IA…): mostra como ligar o
//!   Claude Code, o Claude Desktop, o Cursor ou outro cliente MCP ao servidor
//!   do projeto (`nn mcp`), e grava a configuração para quem pedir.
//! - **O editor atende o servidor MCP** ([`EditorLink`]), pela porta local
//!   (como o jogo aberto, veja `nn_script::remote`): o agente vê a aba da
//!   frente, os objetos escolhidos e o que não foi gravado (`ver_editor`), e
//!   avisa o que mudou (vira uma linha ✨ no Console).

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui;
use serde_json::{Map, Value, json};

use nn_script::remote;

// Configuração dos clientes ---------------------------------------------------

/// Os clientes MCP que a janela sabe configurar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum McpClient {
    #[default]
    ClaudeCode,
    ClaudeDesktop,
    Cursor,
    Other,
}

impl McpClient {
    pub const ALL: [McpClient; 4] = [
        McpClient::ClaudeCode,
        McpClient::ClaudeDesktop,
        McpClient::Cursor,
        McpClient::Other,
    ];

    pub fn label(self) -> &'static str {
        match self {
            McpClient::ClaudeCode => "Claude Code",
            McpClient::ClaudeDesktop => "Claude Desktop",
            McpClient::Cursor => "Cursor",
            McpClient::Other => "Outro",
        }
    }
}

/// O `nn` que veio com o NN Studio (ao lado dele), ou só `nn` (do PATH).
pub fn nn_exe(studio_exe: &Path) -> PathBuf {
    let file = format!("nn{}", std::env::consts::EXE_SUFFIX);
    studio_exe
        .parent()
        .map(|d| d.join(&file))
        .filter(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from(file))
}

/// O nome do servidor na configuração de um cliente que vale para vários
/// projetos: `nn-` e o nome da pasta, só com letras simples.
pub fn server_name(project: &Path) -> String {
    let folder = project
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut slug = String::new();
    for c in crate::commands::fold(&folder).chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "nn-jogo".into()
    } else {
        format!("nn-{slug}")
    }
}

/// A entrada `mcpServers` de um servidor: `nn mcp <pasta>`.
pub fn server_entry(nn: &Path, project_arg: &str) -> Value {
    json!({
        "command": nn.to_string_lossy(),
        "args": ["mcp", project_arg],
    })
}

/// Um argumento de linha de comando entre aspas, se precisar.
fn quoted(s: &str) -> String {
    if !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "/\\._-:".contains(c))
    {
        s.to_string()
    } else {
        format!("\"{s}\"")
    }
}

/// O comando que liga o servidor no Claude Code (só para quem rodar).
pub fn claude_code_command(nn: &Path, project: &Path) -> String {
    format!(
        "claude mcp add {} -- {} mcp {}",
        server_name(project),
        quoted(&nn.to_string_lossy()),
        quoted(&project.to_string_lossy())
    )
}

/// Põe (ou troca) o servidor `name` na configuração JSON `existing` (vazia
/// ou ausente: uma nova), mantendo o resto. `Err` se o arquivo tem um erro.
pub fn merge_config(existing: Option<&str>, name: &str, entry: Value) -> Result<String, String> {
    let mut root = match existing.map(str::trim).filter(|t| !t.is_empty()) {
        None => Value::Object(Map::new()),
        Some(text) => serde_json::from_str::<Value>(text)
            .map_err(|e| format!("o arquivo tem um erro de JSON ({e}); não mexi nele"))?,
    };
    let Some(obj) = root.as_object_mut() else {
        return Err("o arquivo não é um objeto JSON; não mexi nele".into());
    };
    let servers = obj
        .entry("mcpServers")
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(servers) = servers.as_object_mut() else {
        return Err("\"mcpServers\" no arquivo não é um objeto; não mexi nele".into());
    };
    servers.insert(name.to_string(), entry);
    Ok(serde_json::to_string_pretty(&root).unwrap_or_default() + "\n")
}

/// O arquivo de configuração do Claude Desktop neste sistema.
pub fn desktop_config_path() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("Claude"))
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Library/Application Support/Claude"))
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|d| d.join("Claude"))
    }
    .map(|d| d.join("claude_desktop_config.json"))
}

/// Grava o servidor no arquivo de configuração `file` (com uma cópia do
/// arquivo de antes em `<arquivo>.bak`). Devolve o que fez.
pub fn write_config(file: &Path, name: &str, entry: Value) -> Result<String, String> {
    let existing = std::fs::read_to_string(file).ok();
    let text = merge_config(existing.as_deref(), name, entry)?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    if let Some(old) = &existing {
        let mut bak = file.as_os_str().to_owned();
        bak.push(".bak");
        std::fs::write(PathBuf::from(bak), old).map_err(|e| e.to_string())?;
    }
    std::fs::write(file, text).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(if existing.is_some() {
        format!(
            "Pronto: \"{name}\" está em {} (o arquivo de antes ficou em .bak).",
            crate::settings::display_path(file)
        )
    } else {
        format!(
            "Pronto: criei {} com \"{name}\".",
            crate::settings::display_path(file)
        )
    })
}

// A janela ---------------------------------------------------------------------

/// Jogo > Ligar um agente de IA…
#[derive(Debug, Default)]
pub struct AgentDialog {
    pub client: McpClient,
    /// O resultado do último botão (texto, deu certo).
    pub result: Option<(String, bool)>,
}

/// Desenha a janela. Devolve `false` quando ela fecha.
pub fn dialog_ui(
    ctx: &egui::Context,
    dialog: &mut AgentDialog,
    project: &Path,
    studio_exe: &Path,
    editor_connected: bool,
) -> bool {
    let nn = nn_exe(studio_exe);
    let nn_found = nn.is_absolute();
    let mut open = true;
    let response = egui::Modal::new(egui::Id::new("dialogo-agente")).show(ctx, |ui| {
        ui.set_width(560.0);
        ui.heading("✨ Ligar um agente de IA");
        ui.add_space(4.0);
        ui.label(
            "Um agente de IA (como o Claude) pode fazer o jogo junto com você: montar cenas, \
             pôr comportamentos e eventos, escrever Lua, jogar sem janela, rodar os testes e \
             acompanhar o jogo aberto. Ele fala com o projeto pelo servidor MCP da nn-engine \
             (o nn que veio com o NN Studio). Escolha o programa do agente:",
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            for c in McpClient::ALL {
                if ui
                    .selectable_label(dialog.client == c, c.label())
                    .clicked()
                {
                    dialog.client = c;
                    dialog.result = None;
                }
            }
        });
        ui.separator();
        let project_s = project.to_string_lossy().into_owned();
        match dialog.client {
            McpClient::ClaudeCode => {
                ui.label("No terminal, na pasta do projeto, rode uma vez:");
                copy_block(ui, &claude_code_command(&nn, project));
                ui.add_space(4.0);
                ui.label(
                    "Ou grave um .mcp.json no projeto: o Claude Code oferece o servidor sozinho \
                     ao abrir na pasta (e pergunta antes de usar).",
                );
                if ui.button("📄 Gravar .mcp.json no projeto").clicked() {
                    let entry = server_entry(&nn, "${CLAUDE_PROJECT_DIR:-.}");
                    dialog.result = Some(match write_config(&project.join(".mcp.json"), "nn-engine", entry) {
                        Ok(m) => (m, true),
                        Err(e) => (format!("Não deu: {e}"), false),
                    });
                }
            }
            McpClient::ClaudeDesktop => {
                let file = desktop_config_path();
                ui.label("O NN Studio acrescenta o jogo na configuração do Claude Desktop (o resto fica como está):");
                if let Some(f) = &file {
                    ui.monospace(crate::settings::display_path(f));
                }
                if ui
                    .add_enabled(file.is_some(), egui::Button::new("🔧 Pôr no Claude Desktop"))
                    .clicked()
                    && let Some(f) = &file
                {
                    let entry = server_entry(&nn, &project_s);
                    dialog.result = Some(match write_config(f, &server_name(project), entry) {
                        Ok(m) => (format!("{m} Feche e abra o Claude Desktop para ele ver o jogo."), true),
                        Err(e) => (format!("Não deu: {e}"), false),
                    });
                }
                ui.add_space(4.0);
                ui.label("Ou acrescente à mão, em \"mcpServers\":");
                copy_block(ui, &mcp_json_snippet(&server_name(project), &nn, &project_s));
            }
            McpClient::Cursor => {
                ui.label("O Cursor lê os servidores do projeto em .cursor/mcp.json:");
                if ui.button("📄 Gravar .cursor/mcp.json no projeto").clicked() {
                    let entry = server_entry(&nn, &project_s);
                    dialog.result = Some(
                        match write_config(&project.join(".cursor").join("mcp.json"), "nn-engine", entry) {
                            Ok(m) => (m, true),
                            Err(e) => (format!("Não deu: {e}"), false),
                        },
                    );
                }
                ui.add_space(4.0);
                ui.label("Ou acrescente à mão:");
                copy_block(ui, &mcp_json_snippet("nn-engine", &nn, &project_s));
            }
            McpClient::Other => {
                ui.label("O servidor é um programa que fala MCP pela entrada e saída padrão (stdio). O comando:");
                copy_block(
                    ui,
                    &format!("{} mcp {}", quoted(&nn.to_string_lossy()), quoted(&project_s)),
                );
                ui.label("Na configuração JSON da maioria dos clientes:");
                copy_block(ui, &mcp_json_snippet("nn-engine", &nn, &project_s));
            }
        }
        if !nn_found {
            ui.colored_label(
                crate::theme::WARNING,
                "⚠ Não achei o nn ao lado do NN Studio: os comandos usam o \"nn\" do PATH.",
            );
        }
        if let Some((msg, ok)) = &dialog.result {
            ui.add_space(4.0);
            ui.colored_label(if *ok { crate::theme::PLAY } else { crate::theme::ERROR }, msg);
        }
        ui.add_space(6.0);
        ui.label(if editor_connected {
            "Com o NN Studio aberto, o agente também vê a aba da frente e os objetos escolhidos \
             (\"deixa este inimigo mais rápido\"), e não grava por cima do que você ainda não salvou. \
             O que ele mudar aparece no Console com ✨."
        } else {
            "O agente funciona com o NN Studio aberto ou fechado."
        });
        ui.horizontal(|ui| {
            if ui.button("📖 Como usar (documentação)").clicked() {
                ui.ctx().open_url(egui::OpenUrl::new_tab(
                    "https://github.com/nereuneto/nn-engine/blob/main/docs/mcp.md",
                ));
            }
            if ui.button("Fechar").clicked() {
                open = false;
            }
        });
    });
    if response.should_close() {
        open = false;
    }
    open
}

fn mcp_json_snippet(name: &str, nn: &Path, project: &str) -> String {
    let v = json!({ "mcpServers": { name: server_entry(nn, project) } });
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

/// Um texto para copiar, com o botão.
fn copy_block(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        egui::Frame::new()
            .fill(ui.visuals().extreme_bg_color)
            .inner_margin(6.0)
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.set_max_width(460.0);
                ui.add(egui::Label::new(egui::RichText::new(text).monospace()).wrap());
            });
        if ui
            .button("📋 Copiar")
            .on_hover_text("Copia para a área de transferência")
            .clicked()
        {
            ui.ctx().copy_text(text.to_string());
        }
    });
}

// O editor atende o servidor MCP ----------------------------------------------

/// O que o servidor MCP contou que o agente fez.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentNote {
    pub tool: String,
    pub text: String,
    pub files: Vec<String>,
}

#[derive(Default)]
struct Shared {
    /// O estado do editor (refeito a cada quadro).
    snapshot: Value,
    notes: Vec<AgentNote>,
    /// Quem espera a imagem da janela.
    shots: Vec<Sender<Value>>,
    shot_requested: bool,
    ctx: Option<egui::Context>,
    /// O projeto fechou: quem ainda está ligado ouve que acabou.
    closed: bool,
}

/// O NN Studio atendendo o servidor MCP pela porta local, enquanto o
/// projeto está aberto.
pub struct EditorLink {
    shared: Arc<Mutex<Shared>>,
    _listener: Option<remote::Listener>,
}

impl std::fmt::Debug for EditorLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditorLink")
            .field("listening", &self._listener.is_some())
            .finish()
    }
}

/// Quanto o pedido da imagem espera a janela.
const SHOT_TIMEOUT: Duration = Duration::from_secs(5);

impl EditorLink {
    /// Começa a atender o servidor MCP para o projeto em `root`.
    pub fn start(root: &Path, ctx: &egui::Context) -> Self {
        let shared = Arc::new(Mutex::new(Shared {
            ctx: Some(ctx.clone()),
            ..Shared::default()
        }));
        let s = shared.clone();
        let handler: remote::Handler = Arc::new(move |req: Value| answer(&s, &req));
        let listener = match remote::listen_with(root, remote::EDITOR, handler) {
            Ok(l) => Some(l),
            Err(e) => {
                log::warn!("o NN Studio não vai atender o agente de IA: {e}");
                None
            }
        };
        Self {
            shared,
            _listener: listener,
        }
    }

    pub fn listening(&self) -> bool {
        self._listener.is_some()
    }

    /// A cada quadro: guarda o estado do editor, pede a imagem da janela se
    /// alguém espera e devolve o que o agente avisou.
    pub fn update(&mut self, ctx: &egui::Context, snapshot: Value) -> Vec<AgentNote> {
        let shot = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        let mut s = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        s.snapshot = snapshot;
        if let Some(image) = shot {
            let [w, h] = image.size;
            let pixels: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
            let reply = match nn_script::game_run::encode_png(w as u32, h as u32, pixels) {
                Ok(png) => json!({ "largura": w, "altura": h, "png": remote::base64(&png) }),
                Err(e) => json!({ "erro": e }),
            };
            for tx in s.shots.drain(..) {
                let _ = tx.send(reply.clone());
            }
            s.shot_requested = false;
        }
        if !s.shots.is_empty() && !s.shot_requested {
            s.shot_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            ctx.request_repaint();
        }
        std::mem::take(&mut s.notes)
    }
}

impl Drop for EditorLink {
    fn drop(&mut self) {
        let mut s = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        s.closed = true;
        s.ctx = None;
        for tx in s.shots.drain(..) {
            let _ = tx.send(json!({ "erro": "o projeto foi fechado no NN Studio" }));
        }
    }
}

/// Responde um pedido do servidor MCP (numa thread da ligação).
fn answer(shared: &Arc<Mutex<Shared>>, req: &Value) -> Value {
    if shared.lock().unwrap_or_else(|e| e.into_inner()).closed {
        return json!({ "erro": "o projeto foi fechado no NN Studio" });
    }
    match req["pedido"].as_str().unwrap_or("") {
        "oi" => json!({ "ok": true, "pid": std::process::id(), "editor": true }),
        "editor" => {
            let s = shared.lock().unwrap_or_else(|e| e.into_inner());
            json!({ "ok": true, "editor": s.snapshot })
        }
        "mudou" => {
            let note = AgentNote {
                tool: req["ferramenta"].as_str().unwrap_or("").to_string(),
                text: req["texto"].as_str().unwrap_or("").to_string(),
                files: req["arquivos"]
                    .as_array()
                    .map(|l| {
                        l.iter()
                            .filter_map(|f| f.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
            };
            let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
            s.notes.push(note);
            if let Some(ctx) = &s.ctx {
                ctx.request_repaint();
            }
            json!({ "ok": true })
        }
        "captura" => {
            let (tx, rx) = channel();
            {
                let mut s = shared.lock().unwrap_or_else(|e| e.into_inner());
                s.shots.push(tx);
                if let Some(ctx) = &s.ctx {
                    ctx.request_repaint();
                }
            }
            rx.recv_timeout(SHOT_TIMEOUT).unwrap_or_else(
                |_| json!({ "erro": "o NN Studio não mandou a imagem da janela (minimizada?)" }),
            )
        }
        other => json!({ "erro": format!("pedido desconhecido: {other}") }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nome_do_servidor_pela_pasta() {
        assert_eq!(
            server_name(Path::new("/home/ana/jogos/Ilha Mágica")),
            "nn-ilha-magica"
        );
        assert_eq!(server_name(Path::new("/x/meu_jogo!")), "nn-meu-jogo");
        assert_eq!(server_name(Path::new("/")), "nn-jogo");
    }

    #[test]
    fn junta_na_configuracao_sem_perder_o_resto() {
        let nn = Path::new("/opt/nn/nn");
        let e = server_entry(nn, "/jogos/ilha");
        let fresh = merge_config(None, "nn-ilha", e.clone()).unwrap();
        let v: Value = serde_json::from_str(&fresh).unwrap();
        assert_eq!(v["mcpServers"]["nn-ilha"]["command"], "/opt/nn/nn");
        assert_eq!(
            v["mcpServers"]["nn-ilha"]["args"],
            json!(["mcp", "/jogos/ilha"])
        );
        let other = r#"{ "theme": "dark", "mcpServers": { "outro": { "command": "x" } } }"#;
        let v: Value =
            serde_json::from_str(&merge_config(Some(other), "nn-ilha", e.clone()).unwrap())
                .unwrap();
        assert_eq!(v["theme"], "dark");
        assert_eq!(v["mcpServers"]["outro"]["command"], "x");
        assert_eq!(v["mcpServers"]["nn-ilha"]["args"][0], "mcp");
        assert!(
            merge_config(Some("{ quebrado"), "nn", e.clone())
                .unwrap_err()
                .contains("não mexi")
        );
        assert!(merge_config(Some(r#"{"mcpServers": 3}"#), "nn", e).is_err());
    }

    #[test]
    fn grava_com_copia_do_de_antes() {
        let dir = std::env::temp_dir().join(format!("nn-agente-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let file = dir.join("Claude").join("claude_desktop_config.json");
        let e = server_entry(Path::new("nn"), "/j");
        assert!(
            write_config(&file, "nn-j", e.clone())
                .unwrap()
                .contains("criei")
        );
        std::fs::write(&file, r#"{"a": 1}"#).unwrap();
        assert!(write_config(&file, "nn-j", e).unwrap().contains(".bak"));
        assert_eq!(
            std::fs::read_to_string(dir.join("Claude/claude_desktop_config.json.bak")).unwrap(),
            r#"{"a": 1}"#
        );
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(v["a"], 1);
        assert!(v["mcpServers"]["nn-j"].is_object());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn comando_do_claude_code() {
        let c = claude_code_command(Path::new("/opt/nn Studio/nn"), Path::new("/jogos/ilha"));
        assert_eq!(
            c,
            "claude mcp add nn-ilha -- \"/opt/nn Studio/nn\" mcp /jogos/ilha"
        );
    }
}
