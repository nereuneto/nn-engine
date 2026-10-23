//! O jogo aberto atende quem está no mesmo computador (o servidor MCP, para
//! um agente de IA) por uma porta local: mostra o estado, muda valores na
//! hora, roda Lua, pausa, avança quadros e manda a imagem da tela.
//!
//! Ao abrir um jogo de uma pasta (`nn .`, o ▶ Jogar do NN Studio), o jogo
//! escuta numa porta de `127.0.0.1` e escreve a porta e uma senha num arquivo
//! que só o usuário lê ([`connection_file`]). Só quem lê esse arquivo (um
//! programa do mesmo usuário) consegue falar com o jogo. Um jogo exportado
//! não escuta.
//!
//! O protocolo é JSON, uma mensagem por linha. Cada pedido leva a senha:
//!
//! ```json
//! {"token": "...", "pedido": "estado", "olhar": ["nn.vars.moedas"]}
//! {"token": "...", "pedido": "mudar", "objeto": "heroi", "campo": "velocidade", "componente": "nn:plataforma", "valor": 500}
//! {"token": "...", "pedido": "variavel", "nome": "vidas", "valor": 9}
//! {"token": "...", "pedido": "lua", "codigo": "self.objects.heroi:move_to(0, 0)"}
//! {"token": "...", "pedido": "teclas", "teclas": [{"em": 0, "tecla": "right", "baixo": true}, {"em": 1, "tecla": "right", "baixo": false}]}
//! {"token": "...", "pedido": "pausar"}            (e "continuar", "avancar" com "quadros")
//! {"token": "...", "pedido": "captura"}
//! {"token": "...", "pedido": "fechar"}
//! ```
//!
//! e volta uma linha com a resposta (`{"erro": "..."}` quando não deu). Use
//! [`Client`] para falar com um jogo aberto.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mlua::{Function, LuaSerdeExt, Table, Value as LuaValue};
use serde_json::{Value, json};

use crate::{ScriptRuntime, capture, live};

/// Quanto o pedido espera o jogo responder.
const REPLY_TIMEOUT: Duration = Duration::from_secs(10);

static SERVING: AtomicBool = AtomicBool::new(false);

/// Um pedido esperando o próximo quadro do jogo.
struct Pending {
    request: Value,
    reply: Sender<Value>,
}

static QUEUE: Mutex<Vec<Pending>> = Mutex::new(Vec::new());
/// Quem pediu a imagem da tela e espera ela sair.
static CAPTURES: Mutex<Vec<Sender<Value>>> = Mutex::new(Vec::new());

/// Uma tecla para apertar ou soltar daqui a `wait` segundos de jogo (só
/// contam os quadros que andam: pausado, o tempo para).
struct KeyEvent {
    wait: f64,
    keys: &'static [nn_input::KeyCode],
    down: bool,
}

static KEYS: Mutex<Vec<KeyEvent>> = Mutex::new(Vec::new());

/// O jogo está atendendo pela porta local?
pub fn serving() -> bool {
    SERVING.load(Ordering::Relaxed)
}

/// O arquivo com a porta e a senha do jogo aberto de uma pasta.
pub fn connection_file(project_dir: &Path) -> PathBuf {
    connection_file_for(project_dir, GAME)
}

/// Quem atende pela porta local: o jogo aberto (`GAME`) ou o NN Studio com o
/// projeto aberto (`EDITOR`).
pub const GAME: &str = "";
pub const EDITOR: &str = "editor";

/// O arquivo de conexão de `kind` ([`GAME`] ou [`EDITOR`]) de uma pasta.
pub fn connection_file_for(project_dir: &Path, kind: &str) -> PathBuf {
    let dir = project_dir
        .canonicalize()
        .unwrap_or_else(|_| project_dir.to_path_buf());
    // FNV-1a: o mesmo número para a mesma pasta em qualquer programa.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in dir.to_string_lossy().as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    if kind.is_empty() {
        base_dir().join(format!("{h:016x}.json"))
    } else {
        base_dir().join(format!("{h:016x}-{kind}.json"))
    }
}

/// Onde ficam os arquivos de conexão: uma pasta só do usuário (a senha deixa
/// rodar Lua no jogo). No Linux, a de `XDG_RUNTIME_DIR` (ou `~/.cache`), e
/// não o `/tmp`, que é de todos; no Windows e no macOS, a pasta temporária
/// já é do usuário.
fn base_dir() -> PathBuf {
    if cfg!(target_os = "linux") {
        if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR").filter(|d| !d.is_empty()) {
            return PathBuf::from(dir).join("nn-ao-vivo");
        }
        if let Some(home) = std::env::var_os("HOME").filter(|d| !d.is_empty()) {
            return PathBuf::from(home).join(".cache").join("nn-ao-vivo");
        }
    }
    std::env::temp_dir().join("nn-ao-vivo")
}

/// Grava o arquivo de conexão só para o usuário ler.
fn write_private(file: &Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
        }
    }
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(file)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    f.write_all(text.as_bytes())
}

fn random_token() -> String {
    let mut out = String::new();
    for i in 0..2u8 {
        let mut h = RandomState::new().build_hasher();
        h.write_u8(i);
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        h.write_u32(std::process::id());
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

/// Quem atende pela porta: recebe o pedido (já com a senha conferida) e
/// devolve a resposta.
pub type Handler = Arc<dyn Fn(Value) -> Value + Send + Sync>;

/// Atendendo pela porta: tira o arquivo de conexão ao sair de cena.
#[derive(Debug)]
pub struct Listener {
    file: PathBuf,
    port: u16,
    game: bool,
}

impl Listener {
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        if self.game {
            SERVING.store(false, Ordering::Relaxed);
        }
        // Só se ainda for o nosso (outro jogo da mesma pasta pode ter aberto depois).
        if let Ok(text) = std::fs::read_to_string(&self.file)
            && let Ok(v) = serde_json::from_str::<Value>(&text)
            && v["porta"].as_u64() == Some(u64::from(self.port))
        {
            let _ = std::fs::remove_file(&self.file);
        }
    }
}

/// Começa a atender pela porta local o jogo da pasta `project_dir`.
pub fn listen(project_dir: &Path) -> std::io::Result<Listener> {
    capture::ensure_installed();
    capture::enable_live();
    let mut listener = listen_with(project_dir, GAME, Arc::new(answer))?;
    listener.game = true;
    SERVING.store(true, Ordering::Relaxed);
    log::info!(
        "jogo aberto atendendo na porta {} (servidor MCP)",
        listener.port
    );
    Ok(listener)
}

/// Atende pela porta local, para quem é `kind` na pasta `project_dir`, com
/// `handler` respondendo os pedidos (numa thread de cada ligação).
pub fn listen_with(project_dir: &Path, kind: &str, handler: Handler) -> std::io::Result<Listener> {
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))?;
    let port = listener.local_addr()?.port();
    let token = random_token();
    let file = connection_file_for(project_dir, kind);
    let info = json!({
        "porta": port,
        "token": token,
        "pid": std::process::id(),
        "pasta": project_dir.canonicalize().unwrap_or_else(|_| project_dir.to_path_buf()),
    });
    write_private(&file, &info.to_string())?;
    let connections = Arc::new(AtomicUsize::new(0));
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let token = token.clone();
            let handler = handler.clone();
            let connections = connections.clone();
            std::thread::spawn(move || handle_connection(stream, &token, &handler, &connections));
        }
    });
    Ok(Listener {
        file,
        port,
        game: false,
    })
}

/// O maior pedido aceito (um script Lua grande cabe com folga).
const MAX_REQUEST: u64 = 1 << 20;
/// Quanto uma ligação nova tem para mandar o primeiro pedido, com a senha.
const HELLO_TIMEOUT: Duration = Duration::from_secs(5);
/// Quantas ligações ao mesmo tempo (o servidor MCP usa uma).
const MAX_CONNECTIONS: usize = 8;

/// Compara a senha sem deixar o tempo da comparação dizer quanto acertou.
fn same_token(given: Option<&str>, token: &str) -> bool {
    let Some(given) = given else {
        return false;
    };
    let (a, b) = (given.as_bytes(), token.as_bytes());
    let mut diff = u8::from(a.len() != b.len());
    for (i, x) in b.iter().enumerate() {
        diff |= x ^ a.get(i).copied().unwrap_or(0);
    }
    diff == 0
}

/// Lê uma linha de no máximo `MAX_REQUEST` bytes. `Ok(None)` no fim da
/// ligação; `Err` se a linha passou do limite ou a leitura falhou.
fn read_request<R: BufRead>(reader: &mut R) -> std::io::Result<Option<String>> {
    let mut buf = Vec::new();
    let n = std::io::Read::take(&mut *reader, MAX_REQUEST + 1).read_until(b'\n', &mut buf)?;
    if n == 0 {
        return Ok(None);
    }
    if buf.len() as u64 > MAX_REQUEST {
        return Err(std::io::Error::other("pedido grande demais"));
    }
    Ok(Some(String::from_utf8_lossy(&buf).into_owned()))
}

fn handle_connection(stream: TcpStream, token: &str, handler: &Handler, count: &AtomicUsize) {
    if count.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
        count.fetch_sub(1, Ordering::SeqCst);
        let mut out = stream;
        let _ = writeln!(out, "{}", json!({ "erro": "ligações demais" }));
        return;
    }
    serve_connection(stream, token, handler);
    count.fetch_sub(1, Ordering::SeqCst);
}

fn serve_connection(stream: TcpStream, token: &str, handler: &Handler) {
    let Ok(read) = stream.try_clone() else {
        return;
    };
    // Quem liga e não diz nada (ou não sabe a senha) não fica ocupando o jogo.
    let _ = stream.set_read_timeout(Some(HELLO_TIMEOUT));
    let mut out = stream;
    let mut reader = BufReader::new(read);
    let mut known = false;
    loop {
        let line = match read_request(&mut reader) {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(e) => {
                let _ = writeln!(out, "{}", json!({ "erro": e.to_string() }));
                break;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(req) if !same_token(req["token"].as_str(), token) => {
                let _ = writeln!(out, "{}", json!({ "erro": "senha errada" }));
                break;
            }
            Err(_) if !known => break,
            Err(e) => json!({ "erro": format!("JSON inválido: {e}") }),
            Ok(req) => {
                if !known {
                    known = true;
                    let _ = out.set_read_timeout(None);
                }
                handler(req)
            }
        };
        if writeln!(out, "{reply}").and_then(|()| out.flush()).is_err() {
            break;
        }
    }
}

/// Responde um pedido: o que não precisa do jogo, aqui; o resto, no próximo
/// quadro.
fn answer(req: Value) -> Value {
    match req["pedido"].as_str().unwrap_or("") {
        "oi" => json!({ "ok": true, "pid": std::process::id() }),
        "pausar" => {
            live::push(r#"{"cmd":"pause"}"#);
            json!({ "ok": true, "pausado": true })
        }
        "continuar" => {
            live::push(r#"{"cmd":"resume"}"#);
            json!({ "ok": true, "pausado": false })
        }
        "teclas" => match parse_keys(&req["teclas"]) {
            Ok(events) => {
                let n = events.len();
                KEYS.lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .extend(events);
                json!({ "ok": true, "teclas": n })
            }
            Err(e) => json!({ "erro": e }),
        },
        "avancar" => {
            let n = req["quadros"].as_u64().unwrap_or(1).clamp(1, 600);
            for _ in 0..n {
                live::push(r#"{"cmd":"step"}"#);
            }
            // Espera os quadros andarem (o jogo pausado anda um por quadro).
            let started = std::time::Instant::now();
            while live::pending_steps() > 0 && started.elapsed() < REPLY_TIMEOUT {
                std::thread::sleep(Duration::from_millis(5));
            }
            json!({ "ok": true, "pausado": true, "quadros": n })
        }
        _ => {
            let (tx, rx) = channel();
            QUEUE
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(Pending {
                    request: req,
                    reply: tx,
                });
            rx.recv_timeout(REPLY_TIMEOUT).unwrap_or_else(|_| {
                json!({ "erro": "o jogo não respondeu (está travado, ou a janela minimizada?)" })
            })
        }
    }
}

fn parse_keys(list: &Value) -> Result<Vec<KeyEvent>, String> {
    let list = list.as_array().ok_or("\"teclas\" é uma lista")?;
    let mut out = Vec::new();
    for t in list {
        let name = t["tecla"]
            .as_str()
            .ok_or("cada tecla precisa de \"tecla\"")?;
        let keys = crate::convert::keys(name).map_err(|e| crate::format_error(&e))?;
        out.push(KeyEvent {
            wait: t["em"].as_f64().unwrap_or(0.0).max(0.0),
            keys,
            down: t["baixo"].as_bool().unwrap_or(true),
        });
    }
    Ok(out)
}

/// Aperta e solta as teclas pedidas que chegaram na hora, num quadro que
/// vai andar (pausado, espera o "avancar").
fn press_keys(ctx: &mut nn_app::Context) {
    let mut keys = KEYS.lock().unwrap_or_else(|e| e.into_inner());
    if keys.is_empty() || (live::paused() && live::pending_steps() == 0) {
        return;
    }
    let dt = f64::from(ctx.time.delta_secs());
    keys.retain_mut(|k| {
        if k.wait <= 1e-6 {
            crate::game_run::set_key(&mut ctx.input, k.keys, k.down);
            false
        } else {
            k.wait -= dt;
            true
        }
    });
}

/// Atende os pedidos que chegaram (chamado a cada quadro pela cena da frente).
pub(crate) fn serve(
    runtime: &ScriptRuntime,
    path: &str,
    scene: Option<Table>,
    error: Option<&str>,
    ctx: &mut nn_app::Context,
) {
    capture::set_live_time(f64::from(ctx.time.elapsed_secs()));
    press_keys(ctx);
    // A imagem pedida no quadro anterior.
    {
        let mut waiting = CAPTURES.lock().unwrap_or_else(|e| e.into_inner());
        if !waiting.is_empty()
            && let Some(shot) = ctx.take_screenshot()
        {
            let reply = match shot.and_then(|s| {
                crate::game_run::encode_png(s.width, s.height, s.pixels)
                    .map(|png| (s.width, s.height, png))
            }) {
                Ok((w, h, png)) => json!({ "largura": w, "altura": h, "png": base64(&png) }),
                Err(e) => json!({ "erro": e }),
            };
            for tx in waiting.drain(..) {
                let _ = tx.send(reply.clone());
            }
        }
    }
    let pending: Vec<Pending> =
        std::mem::take(&mut *QUEUE.lock().unwrap_or_else(|e| e.into_inner()));
    if pending.is_empty() {
        return;
    }
    let driver = match runtime.driver() {
        Ok(d) => d,
        Err(e) => {
            for p in pending {
                let _ = p.reply.send(json!({ "erro": crate::format_error(&e) }));
            }
            return;
        }
    };
    for p in pending {
        let req = &p.request;
        let reply = match req["pedido"].as_str().unwrap_or("") {
            "estado" => state(runtime, &driver, path, scene.clone(), error, req, ctx),
            "mudar" => call_driver(runtime, &driver, ctx, "live_set", |lua| {
                Ok((
                    scene.clone(),
                    lua.to_value(&req["objeto"])?,
                    req["campo"].as_str().unwrap_or("").to_string(),
                    lua.to_value(&req["valor"])?,
                    // Sem componente: o próprio objeto (nil, e não o null do JSON).
                    match &req["componente"] {
                        Value::Null => LuaValue::Nil,
                        c => lua.to_value(c)?,
                    },
                ))
            }),
            "variavel" => match req["nome"].as_str().filter(|n| !n.is_empty()) {
                None => json!({ "erro": "falta o nome da variável" }),
                Some(name) => {
                    let r = runtime.with_ctx(ctx, |lua| {
                        let vars: Table = lua.globals().get::<Table>("nn")?.get("vars")?;
                        vars.set(name, lua.to_value(&req["valor"])?)
                    });
                    match r {
                        Ok(()) => json!({ "ok": true }),
                        Err(e) => json!({ "erro": crate::format_error(&e) }),
                    }
                }
            },
            "lua" => call_driver(runtime, &driver, ctx, "inspect", |_| {
                Ok((
                    scene.clone(),
                    req["codigo"].as_str().unwrap_or("").to_string(),
                    "lua",
                ))
            }),
            "captura" => {
                ctx.request_screenshot();
                CAPTURES
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(p.reply);
                continue;
            }
            "fechar" => {
                ctx.exit();
                json!({ "ok": true })
            }
            other => json!({ "erro": format!("pedido desconhecido: {other}") }),
        };
        let _ = p.reply.send(reply);
    }
}

/// Chama `D.<name>(args)` (que devolve ok, texto) com o contexto da engine.
fn call_driver<A: mlua::IntoLuaMulti>(
    runtime: &ScriptRuntime,
    driver: &Table,
    ctx: &mut nn_app::Context,
    name: &str,
    args: impl FnOnce(&mlua::Lua) -> mlua::Result<A>,
) -> Value {
    let r = runtime.with_ctx(ctx, |lua| {
        let f: Function = driver.get(name)?;
        f.call::<(bool, String)>(args(lua)?)
    });
    match r {
        Ok((true, v)) => json!({ "ok": true, "valor": v }),
        Ok((false, e)) => json!({ "erro": crate::translate_error(&e) }),
        Err(e) => json!({ "erro": crate::format_error(&e) }),
    }
}

fn state(
    runtime: &ScriptRuntime,
    driver: &Table,
    path: &str,
    scene: Option<Table>,
    error: Option<&str>,
    req: &Value,
    ctx: &mut nn_app::Context,
) -> Value {
    let lua = runtime.lua().clone();
    let r = runtime.with_ctx(ctx, |_| {
        let snapshot: LuaValue = driver.get::<Function>("snapshot")?.call(scene.clone())?;
        let fired: LuaValue = driver.get::<Function>("live_fired")?.call(scene.clone())?;
        let mut looked = Vec::new();
        if let Some(list) = req["olhar"].as_array() {
            let inspect: Function = driver.get("inspect")?;
            for e in list.iter().filter_map(Value::as_str) {
                let (ok, v): (bool, String) = inspect.call((scene.clone(), e))?;
                looked.push(json!({ "expr": e, "ok": ok, "valor": if ok { v } else { crate::translate_error(&v) } }));
            }
        }
        Ok((snapshot, fired, looked))
    });
    let (snapshot, fired, looked) = match r {
        Ok(x) => x,
        Err(e) => return json!({ "erro": crate::format_error(&e) }),
    };
    let to_json = |v: LuaValue| lua.from_value::<Value>(v).unwrap_or(Value::Null);
    let (lines, dropped) = capture::take_live();
    let lines: Vec<Value> = lines
        .into_iter()
        .map(|l| {
            json!({
                "tempo": l.time,
                "tipo": match l.kind {
                    capture::LineKind::Print => "print",
                    capture::LineKind::Warning => "aviso",
                    capture::LineKind::Error => "erro",
                },
                "texto": l.text,
            })
        })
        .collect();
    let errors: Vec<Value> = runtime
        .take_errors()
        .into_iter()
        .map(|e| json!({ "cena": e.scene, "mensagem": e.message }))
        .collect();
    json!({
        "cena": path,
        "tempo": ctx.time.elapsed_secs(),
        "quadro": ctx.time.frame(),
        "pausado": live::paused(),
        "erro_na_tela": error,
        "erros": errors,
        "linhas": lines,
        "linhas_de_fora": dropped,
        "regras": to_json(fired),
        "estado": to_json(snapshot),
        "olhar": looked,
        "tela": [ctx.viewport().x, ctx.viewport().y],
    })
}

/// Base64 padrão (para mandar a imagem em JSON).
pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Fala com o jogo aberto de uma pasta.
#[derive(Debug)]
pub struct Client {
    stream: TcpStream,
    reader: BufReader<TcpStream>,
    token: String,
}

/// O texto de quando não há jogo aberto.
pub const NOT_OPEN: &str = "o jogo desta pasta não está aberto";

impl Client {
    /// Liga no jogo aberto da pasta (`Err` com o motivo, se não houver).
    pub fn connect(project_dir: &Path) -> Result<Self, String> {
        Self::connect_to(project_dir, GAME)
    }

    /// Liga em quem atende `kind` ([`GAME`] ou [`EDITOR`]) na pasta.
    pub fn connect_to(project_dir: &Path, kind: &str) -> Result<Self, String> {
        let file = connection_file_for(project_dir, kind);
        let text = std::fs::read_to_string(&file).map_err(|_| NOT_OPEN.to_string())?;
        let info: Value = serde_json::from_str(&text).map_err(|_| NOT_OPEN.to_string())?;
        let port = info["porta"].as_u64().unwrap_or(0) as u16;
        let token = info["token"].as_str().unwrap_or("").to_string();
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).map_err(|_| {
            // Sobrou de um jogo que não fechou direito.
            let _ = std::fs::remove_file(&file);
            NOT_OPEN.to_string()
        })?;
        let _ = stream.set_read_timeout(Some(REPLY_TIMEOUT + Duration::from_secs(5)));
        let reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
        let mut client = Self {
            stream,
            reader,
            token,
        };
        let hello = client.request(json!({ "pedido": "oi" }))?;
        if hello["ok"] != true {
            return Err(NOT_OPEN.into());
        }
        Ok(client)
    }

    /// Manda um pedido e espera a resposta. `Err` se a ligação caiu ou o
    /// jogo respondeu `{"erro": ...}`.
    pub fn request(&mut self, mut req: Value) -> Result<Value, String> {
        req["token"] = json!(self.token);
        writeln!(self.stream, "{req}")
            .and_then(|()| self.stream.flush())
            .map_err(|_| "o jogo fechou".to_string())?;
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) | Err(_) => return Err("o jogo fechou ou não respondeu".into()),
            Ok(_) => {}
        }
        let v: Value =
            serde_json::from_str(&line).map_err(|e| format!("resposta inválida do jogo: {e}"))?;
        match v.get("erro").and_then(Value::as_str) {
            Some(e) => Err(e.to_string()),
            None => Ok(v),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_padrao() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[cfg(unix)]
    #[test]
    fn arquivo_so_do_usuario() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("nn-remote-{}", std::process::id()));
        let file = dir.join("sub").join("x.json");
        write_private(&file, "{}").unwrap();
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&file), 0o600);
        assert_eq!(mode(file.parent().unwrap()), 0o700);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn senha_comparada_inteira() {
        assert!(same_token(Some("abc123"), "abc123"));
        assert!(!same_token(Some("abc12"), "abc123"));
        assert!(!same_token(Some("abc1234"), "abc123"));
        assert!(!same_token(Some("abc124"), "abc123"));
        assert!(!same_token(Some(""), "abc123"));
        assert!(!same_token(None, "abc123"));
    }

    #[test]
    fn pedido_grande_demais() {
        let big = format!("{}\n", "x".repeat(MAX_REQUEST as usize + 10));
        let mut r = std::io::Cursor::new(big.into_bytes());
        assert!(read_request(&mut r).is_err());
        let mut r = std::io::Cursor::new(b"{\"a\":1}\n{\"b\":2}".to_vec());
        assert_eq!(read_request(&mut r).unwrap().unwrap(), "{\"a\":1}\n");
        assert_eq!(read_request(&mut r).unwrap().unwrap(), "{\"b\":2}");
        assert_eq!(read_request(&mut r).unwrap(), None);
    }

    #[test]
    fn arquivo_por_pasta() {
        let a = connection_file(Path::new("/tmp/um"));
        let b = connection_file(Path::new("/tmp/outro"));
        assert_ne!(a, b);
        assert_eq!(a, connection_file(Path::new("/tmp/um")));
        assert_ne!(random_token(), random_token());
    }
}
