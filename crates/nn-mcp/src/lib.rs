//! Servidor MCP (Model Context Protocol) da nn-engine.
//!
//! `nn mcp <pasta>` liga um agente de IA (Claude Code, Claude Desktop,
//! Cursor...) a um projeto de jogo. O agente conversa com o servidor pela
//! entrada e saída padrão (JSON-RPC 2.0, uma mensagem por linha) e usa as
//! ferramentas para conhecer o projeto, mudar cenas, objetos, componentes,
//! prefabs, eventos e scripts, e conferir o que fez. Tudo passa pelo mesmo
//! código do NN Studio ([`nn_projeto`]), então o que o agente grava o editor
//! abre igual.
//!
//! O servidor só mexe em arquivos dentro da pasta do projeto, e cada mudança
//! pode ser desfeita (ferramenta `desfazer`).
//!
//! ```no_run
//! let server = nn_mcp::Server::new("meu-jogo".into(), nn_mcp::Hooks::default());
//! nn_mcp::serve_stdio(server).unwrap();
//! ```

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

mod docs;
mod history;
mod paths;
mod tools;

pub use history::History;

/// Versão da engine (a do `nn`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Versões do protocolo MCP que o servidor entende (a mais nova primeiro).
pub const PROTOCOL_VERSIONS: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

/// Um modelo de projeto que `criar_projeto` oferece.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateInfo {
    pub name: String,
    pub description: String,
    pub three_d: bool,
}

/// Cria um projeto: pasta, modelo e título. Devolve os arquivos criados.
pub type CreateProject = fn(&Path, &str, Option<&str>) -> Result<Vec<String>, String>;

/// Abre o jogo de uma pasta numa janela (começando na cena pedida), com o
/// log indo para o arquivo dado. Devolve o processo.
pub type PlayGame = fn(&Path, Option<&str>, &Path) -> std::io::Result<std::process::Child>;

/// O que o servidor recebe de quem o abre (a ferramenta `nn`): os modelos de
/// projeto, como criar um e como abrir o jogo.
#[derive(Debug, Clone, Default)]
pub struct Hooks {
    pub templates: Vec<TemplateInfo>,
    pub create_project: Option<CreateProject>,
    /// Texto do `AGENTS.md` dos projetos novos (também vira um recurso).
    pub agents_md: String,
    /// Abre o jogo numa janela (ferramenta `abrir_jogo`).
    pub play: Option<PlayGame>,
}

/// O servidor: a pasta do projeto e o que ele lembra entre as chamadas.
#[derive(Debug)]
pub struct Server {
    root: PathBuf,
    hooks: Hooks,
    history: History,
    catalog: nn_projeto::behaviors::Catalog,
    client: String,
    /// O estado do jogo aberto na última olhada (cena e estado), para dizer
    /// o que mudou.
    live_last: Option<(String, Value)>,
    /// Quando foi a última olhada e em que quadro o jogo estava (para dizer
    /// quantos quadros por segundo ele roda).
    live_clock: Option<(std::time::Instant, u64)>,
    /// O jogo que `abrir_jogo` abriu.
    game: Option<std::process::Child>,
    /// A ligação com o NN Studio aberto no projeto (se houver).
    editor: Option<nn_script::remote::Client>,
    /// Os arquivos que o NN Studio tem com mudanças não gravadas (lido antes
    /// de cada ferramenta que grava).
    unsaved: Vec<String>,
}

/// Erro de JSON-RPC: código e mensagem.
struct RpcError(i64, String);

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;

impl Server {
    /// Servidor para o projeto em `root` (a pasta pode ainda estar vazia:
    /// `criar_projeto` cria o jogo nela).
    pub fn new(root: PathBuf, hooks: Hooks) -> Self {
        let root = paths::canonical(&root);
        Self {
            root,
            hooks,
            history: History::default(),
            catalog: Default::default(),
            client: String::new(),
            live_last: None,
            live_clock: None,
            game: None,
            editor: None,
            unsaved: Vec::new(),
        }
    }

    /// A pasta do projeto.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Trata uma linha recebida. Devolve a resposta (uma linha de JSON) ou
    /// nada (notificações).
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }
        let response = match serde_json::from_str::<Value>(line) {
            Ok(Value::Array(batch)) => {
                let out: Vec<Value> = batch.into_iter().filter_map(|m| self.handle(m)).collect();
                (!out.is_empty()).then_some(Value::Array(out))
            }
            Ok(msg) => self.handle(msg),
            Err(e) => Some(error_response(
                Value::Null,
                RpcError(PARSE_ERROR, format!("JSON inválido: {e}")),
            )),
        };
        response.map(|r| r.to_string())
    }

    /// Trata uma mensagem JSON-RPC. Devolve a resposta (ou nada, para
    /// notificações e respostas).
    pub fn handle(&mut self, msg: Value) -> Option<Value> {
        let id = msg.get("id").cloned();
        let Some(method) = msg.get("method").and_then(Value::as_str) else {
            // Uma resposta do cliente (o servidor não faz pedidos) ou lixo.
            return match id {
                Some(id) if msg.get("result").is_none() && msg.get("error").is_none() => Some(
                    error_response(id, RpcError(INVALID_REQUEST, "falta \"method\"".into())),
                ),
                _ => None,
            };
        };
        let params = msg.get("params").cloned().unwrap_or(json!({}));
        let Some(id) = id else {
            // Notificação: nada a responder.
            if method == "notifications/initialized" {
                log::info!("cliente MCP pronto");
            }
            return None;
        };
        let result = match method {
            "initialize" => Ok(self.initialize(&params)),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools::list() })),
            "tools/call" => self.call_tool(&params),
            "resources/list" => Ok(json!({ "resources": docs::resources(self) })),
            "resources/templates/list" => Ok(json!({ "resourceTemplates": [] })),
            "resources/read" => self.read_resource(&params),
            "prompts/list" => Ok(json!({ "prompts": docs::prompts() })),
            "prompts/get" => docs::prompt(&params).map_err(|e| RpcError(INVALID_PARAMS, e)),
            "logging/setLevel" => Ok(json!({})),
            other => Err(RpcError(
                METHOD_NOT_FOUND,
                format!("método desconhecido: {other}"),
            )),
        };
        Some(match result {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err(e) => error_response(id, e),
        })
    }

    fn initialize(&mut self, params: &Value) -> Value {
        let asked = params
            .get("protocolVersion")
            .and_then(Value::as_str)
            .unwrap_or("");
        let version = PROTOCOL_VERSIONS
            .iter()
            .find(|v| **v == asked)
            .copied()
            .unwrap_or(PROTOCOL_VERSIONS[0]);
        self.client = params
            .pointer("/clientInfo/name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        log::info!(
            "cliente MCP: {} (protocolo {version})",
            if self.client.is_empty() {
                "?"
            } else {
                &self.client
            }
        );
        json!({
            "protocolVersion": version,
            "capabilities": {
                "tools": { "listChanged": false },
                "resources": { "listChanged": false, "subscribe": false },
                "prompts": { "listChanged": false },
                "logging": {},
            },
            "serverInfo": {
                "name": "nn-engine",
                "title": "nn-engine",
                "version": VERSION,
            },
            "instructions": docs::instructions(self),
        })
    }

    fn call_tool(&mut self, params: &Value) -> Result<Value, RpcError> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| RpcError(INVALID_PARAMS, "falta o nome da ferramenta".into()))?;
        let args = params.get("arguments").cloned().unwrap_or(json!({}));
        let Some(tool) = tools::find(name) else {
            return Err(RpcError(
                INVALID_PARAMS,
                format!("ferramenta desconhecida: {name}"),
            ));
        };
        if !args.is_object() {
            return Err(RpcError(
                INVALID_PARAMS,
                "os argumentos precisam ser um objeto".into(),
            ));
        }
        // Um parâmetro que a ferramenta não tem é erro (o esquema diz
        // additionalProperties: false): um nome errado não pode passar como
        // "Pronto." sem ter feito nada.
        let known: Vec<&str> = (tool.params)().into_iter().map(|(n, _, _)| n).collect();
        if let Some(bad) = args
            .as_object()
            .and_then(|o| o.keys().find(|k| !known.contains(&k.as_str())))
        {
            let mut list = known.clone();
            list.sort_unstable();
            return Ok(json!({
                "content": [{ "type": "text", "text": format!(
                    "{name}: o parâmetro \"{bad}\" não existe. Os que valem: {}.",
                    if list.is_empty() { "nenhum".to_string() } else { list.join(", ") }
                ) }],
                "isError": true,
            }));
        }
        log::info!("ferramenta {name}");
        // Tudo que uma ferramenta grava vira um passo do `desfazer`.
        if !tool.read_only {
            self.fetch_unsaved();
            self.history.begin(&step_label(name, &args));
        }
        // Um defeito numa ferramenta vira uma resposta de erro: o agente
        // continua com o servidor.
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (tool.run)(self, &args)))
                .unwrap_or_else(|panic| {
                    let why = panic
                        .downcast_ref::<&str>()
                        .map(|s| s.to_string())
                        .or_else(|| panic.downcast_ref::<String>().cloned())
                        .unwrap_or_default();
                    log::error!("a ferramenta {name} quebrou: {why}");
                    Err(format!(
                        "A ferramenta {name} quebrou por um defeito da nn-engine ({why}). \
                 O servidor continua; tente de outro jeito e, se puder, conte o defeito em \
                 https://github.com/nereuneto/nn-engine/issues."
                    ))
                });
        if !tool.read_only {
            let files = self.history.current_files();
            self.history.end();
            self.unsaved.clear();
            // Conta ao NN Studio aberto o que mudou.
            if let Ok(out) = &result
                && (!files.is_empty() || name == "desfazer")
            {
                let text = out
                    .content
                    .iter()
                    .find_map(|c| c["text"].as_str())
                    .unwrap_or("");
                self.notify_editor(name, files, text);
            }
        }
        Ok(match result {
            Ok(out) => json!({ "content": out.content, "isError": false }),
            Err(e) => json!({
                "content": [{ "type": "text", "text": e }],
                "isError": true,
            }),
        })
    }

    fn read_resource(&mut self, params: &Value) -> Result<Value, RpcError> {
        let uri = params
            .get("uri")
            .and_then(Value::as_str)
            .ok_or_else(|| RpcError(INVALID_PARAMS, "falta \"uri\"".into()))?;
        let text = docs::read(self, uri).ok_or_else(|| {
            RpcError(
                // O MCP pede este código para recurso que não existe.
                -32002,
                format!("recurso desconhecido: {uri}"),
            )
        })?;
        Ok(json!({
            "contents": [{ "uri": uri, "mimeType": "text/markdown", "text": text }],
        }))
    }
}

/// Nome de um passo do desfazer: a ferramenta e o que ela mexeu.
fn step_label(name: &str, args: &Value) -> String {
    let mut label = name.to_string();
    for key in ["cena", "caminho", "objeto"] {
        if let Some(v) = args.get(key) {
            let v = match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            label.push_str(&format!(" {v}"));
        }
    }
    label
}

fn error_response(id: Value, e: RpcError) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": e.0, "message": e.1 } })
}

/// Atende pela entrada e saída padrão até a entrada fechar. Só as respostas
/// vão para a saída padrão; o log vai para a saída de erros.
pub fn serve_stdio(mut server: Server) -> std::io::Result<()> {
    log::info!(
        "servidor MCP da nn-engine {VERSION} em {}",
        server.root.display()
    );
    let stdin = std::io::stdin();
    let mut stdin = stdin.lock();
    let mut stdout = std::io::stdout();
    let mut buf = Vec::new();
    loop {
        buf.clear();
        if stdin.read_until(b'\n', &mut buf)? == 0 {
            break;
        }
        // Bytes que não são UTF-8 viram um erro de JSON, e não o fim do servidor.
        let line = String::from_utf8_lossy(&buf);
        if let Some(response) = server.handle_line(&line) {
            // Uma mensagem por linha: o JSON compacto não tem quebras.
            stdout.write_all(response.as_bytes())?;
            stdout.write_all(b"\n")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server() -> Server {
        Server::new(std::env::temp_dir(), Hooks::default())
    }

    fn rpc(s: &mut Server, method: &str, params: Value) -> Value {
        s.handle(json!({ "jsonrpc": "2.0", "id": 7, "method": method, "params": params }))
            .unwrap()
    }

    #[test]
    fn combina_a_versao_do_protocolo() {
        let mut s = server();
        let r = rpc(
            &mut s,
            "initialize",
            json!({ "protocolVersion": "2025-03-26" }),
        );
        assert_eq!(r["id"], 7);
        assert_eq!(r["result"]["protocolVersion"], "2025-03-26");
        assert!(r["result"]["capabilities"]["tools"].is_object());
        assert!(
            r["result"]["instructions"]
                .as_str()
                .unwrap()
                .contains("ver_projeto")
        );
        // Versão que não conhece: responde com a mais nova.
        let r = rpc(
            &mut s,
            "initialize",
            json!({ "protocolVersion": "1999-01-01" }),
        );
        assert_eq!(r["result"]["protocolVersion"], PROTOCOL_VERSIONS[0]);
    }

    #[test]
    fn erros_do_protocolo() {
        let mut s = server();
        assert_eq!(
            rpc(&mut s, "voar", json!({}))["error"]["code"],
            METHOD_NOT_FOUND
        );
        let r: Value = serde_json::from_str(&s.handle_line("{ não é json").unwrap()).unwrap();
        assert_eq!(r["error"]["code"], PARSE_ERROR);
        assert_eq!(r["id"], Value::Null);
        // Notificação não tem resposta.
        assert!(
            s.handle(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
                .is_none()
        );
        assert!(s.handle_line("   ").is_none());
        let r = rpc(
            &mut s,
            "tools/call",
            json!({ "name": "voar", "arguments": {} }),
        );
        assert_eq!(r["error"]["code"], INVALID_PARAMS);
        // Erro de uma ferramenta volta como resultado com isError.
        let r = rpc(
            &mut s,
            "tools/call",
            json!({ "name": "ler_arquivo", "arguments": {} }),
        );
        assert_eq!(r["result"]["isError"], true);
        assert!(
            r["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("caminho")
        );
        // Lote: uma resposta por pedido.
        let r: Value = serde_json::from_str(
            &s.handle_line(r#"[{"jsonrpc":"2.0","id":1,"method":"ping"},{"jsonrpc":"2.0","id":2,"method":"ping"}]"#)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(r.as_array().unwrap().len(), 2);
    }

    #[test]
    fn ferramentas_bem_descritas() {
        let mut s = server();
        let tools = rpc(&mut s, "tools/list", json!({}))["result"]["tools"].clone();
        let tools = tools.as_array().unwrap();
        assert!(tools.len() >= 20);
        let mut names = std::collections::HashSet::new();
        for t in tools {
            let name = t["name"].as_str().unwrap();
            assert!(names.insert(name), "{name} repetida");
            assert!(
                name.len() <= 64 && name.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{name}"
            );
            assert!(t["description"].as_str().unwrap().len() > 40, "{name}");
            let schema = &t["inputSchema"];
            assert_eq!(schema["type"], "object");
            let props = schema["properties"].as_object().unwrap();
            for p in props.values() {
                assert!(p["description"].is_string(), "{name}: {p}");
            }
            for r in schema["required"].as_array().unwrap() {
                assert!(props.contains_key(r.as_str().unwrap()), "{name}: {r}");
            }
        }
    }

    #[test]
    fn documentacao_e_prompts() {
        let mut s = server();
        let list = rpc(&mut s, "resources/list", json!({}));
        let list = list["result"]["resources"].as_array().unwrap();
        for r in list {
            let uri = r["uri"].as_str().unwrap();
            let read = rpc(&mut s, "resources/read", json!({ "uri": uri }));
            let text = read["result"]["contents"][0]["text"].as_str();
            // O AGENTS.md só existe com um projeto (ou nos hooks).
            assert!(text.is_some() || uri.contains("AGENTS"), "{uri}");
        }
        let r = rpc(
            &mut s,
            "resources/read",
            json!({ "uri": "nn://docs/nada.md" }),
        );
        assert_eq!(r["error"]["code"], -32002);
        let p = rpc(&mut s, "prompts/list", json!({}));
        for p in p["result"]["prompts"].as_array().unwrap() {
            let r = rpc(
                &mut s,
                "prompts/get",
                json!({ "name": p["name"], "arguments": { "ideia": "x", "pedido": "y" } }),
            );
            assert_eq!(r["result"]["messages"][0]["role"], "user");
        }
    }
}
