//! Scripting em Lua para a nn-engine.
//!
//! Cada arquivo `.lua` de cena devolve uma tabela com callbacks opcionais:
//!
//! ```lua
//! local cena = {}
//!
//! function cena:enter(args)  -- a cena começou
//!   self.x = 0
//! end
//!
//! function cena:update(dt)   -- uma vez por frame
//!   local dx, dy = nn.input.wasd()
//!   self.x = self.x + dx * 200 * dt
//! end
//!
//! function cena:draw()
//!   nn.draw.rect(self.x, 0, 50, 50, "#3FA7D6")
//! end
//!
//! return cena
//! ```
//!
//! [`LuaScene`] transforma esse arquivo numa [`nn_app::Scene`]. Ao salvar
//! qualquer `.lua` do projeto, as cenas abertas são recarregadas sem perder o
//! estado (`self`). Erros aparecem na própria janela e somem quando o arquivo
//! corrigido é salvo.

mod api;
mod convert;
mod physics;
mod tilemap;
mod types;

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime};

use glam::Vec2;
use mlua::{Function, IntoLuaMulti, Lua, RegistryKey, Table, Value};
use nn_app::{Context, Scene, Transition};
use nn_core::Color;
use nn_render::Text;

use crate::api::{CTX_KEY, SceneRequest, Shared};

/// Erro ao preparar o ambiente Lua.
#[derive(Debug)]
pub struct ScriptError(pub String);

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ScriptError {}

impl From<mlua::Error> for ScriptError {
    fn from(e: mlua::Error) -> Self {
        Self(e.to_string())
    }
}

/// Ambiente Lua compartilhado por todas as cenas de um projeto.
///
/// Barato de clonar: os clones compartilham o mesmo estado Lua, então
/// variáveis globais e módulos (`require`) são comuns a todas as cenas.
#[derive(Clone)]
pub struct ScriptRuntime {
    inner: Rc<Inner>,
}

struct Inner {
    lua: Lua,
    root: PathBuf,
    /// Módulos padrão do Lua, que nunca são descarregados na recarga.
    builtin_modules: HashSet<String>,
    generation: Cell<u64>,
    watch: RefCell<Watch>,
}

#[derive(Default)]
struct Watch {
    files: HashMap<PathBuf, SystemTime>,
    last_poll: Option<Instant>,
}

impl std::fmt::Debug for ScriptRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptRuntime")
            .field("root", &self.inner.root)
            .finish()
    }
}

impl ScriptRuntime {
    /// Intervalo entre verificações de scripts alterados.
    pub const POLL_INTERVAL: Duration = Duration::from_millis(250);

    /// Cria o ambiente com os scripts procurados em `root` (geralmente a
    /// mesma pasta dos assets). `require "modulo"` busca `root/modulo.lua`.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, ScriptError> {
        let root = root.into();
        let lua = Lua::new();
        api::install(&lua)?;

        let package: Table = lua.globals().get("package")?;
        let base = root.to_string_lossy().replace('\\', "/");
        let current: String = package.get("path")?;
        package.set("path", format!("{base}/?.lua;{base}/?/init.lua;{current}"))?;

        let loaded: Table = package.get("loaded")?;
        let builtin_modules = loaded
            .pairs::<String, Value>()
            .filter_map(|p| p.ok().map(|(k, _)| k))
            .collect();

        let runtime = Self {
            inner: Rc::new(Inner {
                lua,
                root,
                builtin_modules,
                generation: Cell::new(0),
                watch: RefCell::new(Watch::default()),
            }),
        };
        // Registra o estado atual dos arquivos, para só recarregar o que mudar depois.
        let files = runtime.scan();
        runtime.inner.watch.borrow_mut().files = files;
        Ok(runtime)
    }

    /// Pasta dos scripts.
    pub fn root(&self) -> &Path {
        &self.inner.root
    }

    /// O estado Lua, para quem quiser registrar funções próprias.
    pub fn lua(&self) -> &Lua {
        &self.inner.lua
    }

    /// Cria uma cena a partir de um arquivo, relativo à pasta dos scripts.
    pub fn scene(&self, path: impl Into<String>) -> LuaScene {
        LuaScene::new(self.clone(), path, None)
    }

    /// Executa `f` com o contexto da engine disponível para as funções `nn.*`.
    fn with_ctx<R>(
        &self,
        ctx: &mut Context,
        f: impl FnOnce(&Lua) -> mlua::Result<R>,
    ) -> mlua::Result<R> {
        let lua = &self.inner.lua;
        lua.scope(|scope| {
            let ud = scope.create_any_userdata_ref_mut(ctx)?;
            lua.set_named_registry_value(CTX_KEY, ud)?;
            let result = f(lua);
            lua.unset_named_registry_value(CTX_KEY)?;
            result
        })
    }

    fn take_request(&self) -> Option<SceneRequest> {
        self.inner
            .lua
            .app_data_mut::<Shared>()
            .and_then(|mut s| s.request.take())
    }

    /// Carrega (executa) o arquivo de uma cena e devolve a tabela que ele retorna.
    fn load_module(&self, ctx: &mut Context, path: &str) -> mlua::Result<Table> {
        let full = self.inner.root.join(path);
        let source = std::fs::read_to_string(&full).map_err(|e| {
            mlua::Error::RuntimeError(format!("não foi possível ler {}: {e}", full.display()))
        })?;
        let value: Value = self.with_ctx(ctx, |lua| {
            lua.load(&source).set_name(format!("@{path}")).eval()
        })?;
        match value {
            Value::Table(t) => Ok(t),
            other => Err(mlua::Error::RuntimeError(format!(
                "{path}: o script de cena deve terminar com `return` de uma tabela (recebeu {})",
                other.type_name()
            ))),
        }
    }

    /// Descarrega os módulos do projeto (`require`) para que sejam lidos de novo.
    fn forget_project_modules(&self) -> mlua::Result<()> {
        let loaded: Table = self
            .inner
            .lua
            .globals()
            .get::<Table>("package")?
            .get("loaded")?;
        let keys: Vec<String> = loaded
            .pairs::<String, Value>()
            .filter_map(|p| p.ok().map(|(k, _)| k))
            .filter(|k| !self.inner.builtin_modules.contains(k))
            .collect();
        for k in keys {
            loaded.set(k, Value::Nil)?;
        }
        Ok(())
    }

    fn scan(&self) -> HashMap<PathBuf, SystemTime> {
        let mut files = HashMap::new();
        scan_dir(&self.inner.root, 0, &mut files);
        files
    }

    /// Verifica se algum `.lua` mudou. Devolve a geração atual; ela aumenta a
    /// cada mudança detectada.
    fn poll(&self, hot_reload: bool) -> u64 {
        if hot_reload {
            let now = Instant::now();
            let due = {
                let mut w = self.inner.watch.borrow_mut();
                match w.last_poll {
                    Some(last) if now - last < Self::POLL_INTERVAL => false,
                    _ => {
                        w.last_poll = Some(now);
                        true
                    }
                }
            };
            if due {
                let files = self.scan();
                let mut w = self.inner.watch.borrow_mut();
                if files != w.files {
                    let changed: Vec<_> = files
                        .iter()
                        .filter(|(p, t)| w.files.get(*p) != Some(*t))
                        .map(|(p, _)| p.display().to_string())
                        .collect();
                    log::info!("scripts alterados: {}", changed.join(", "));
                    w.files = files;
                    self.inner.generation.set(self.inner.generation.get() + 1);
                }
            }
        }
        self.inner.generation.get()
    }
}

fn scan_dir(dir: &Path, depth: u32, out: &mut HashMap<PathBuf, SystemTime>) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "target" {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            scan_dir(&path, depth + 1, out);
        } else if path.extension().is_some_and(|e| e == "lua")
            && let Ok(modified) = meta.modified()
        {
            out.insert(path, modified);
        }
    }
}

/// Uma cena escrita em Lua.
pub struct LuaScene {
    runtime: ScriptRuntime,
    path: String,
    args: Option<RegistryKey>,
    instance: Option<Table>,
    entered: bool,
    error: Option<String>,
    generation: u64,
}

impl std::fmt::Debug for LuaScene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LuaScene")
            .field("path", &self.path)
            .field("error", &self.error)
            .finish()
    }
}

impl LuaScene {
    fn new(runtime: ScriptRuntime, path: impl Into<String>, args: Option<RegistryKey>) -> Self {
        let generation = runtime.inner.generation.get();
        Self {
            runtime,
            path: path.into(),
            args,
            instance: None,
            entered: false,
            error: None,
            generation,
        }
    }

    /// Caminho do arquivo da cena.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Mensagem do erro atual, se o script estiver com erro.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Valor de um campo do `self` da cena (útil em testes e ferramentas).
    pub fn field<T: mlua::FromLua>(&self, name: &str) -> Option<T> {
        self.instance
            .as_ref()?
            .get::<Option<T>>(name)
            .ok()
            .flatten()
    }

    fn set_error(&mut self, e: &mlua::Error) {
        let msg = format_error(e);
        log::error!("erro em {}:\n{msg}", self.path);
        self.error = Some(msg);
    }

    /// Carrega o arquivo e cria (ou atualiza) a instância da cena.
    fn load(&mut self, ctx: &mut Context) {
        let module = match self.runtime.load_module(ctx, &self.path) {
            Ok(m) => m,
            Err(e) => {
                self.set_error(&e);
                return;
            }
        };
        self.error = None;
        let lua = self.runtime.lua().clone();
        let result = (|| -> mlua::Result<()> {
            let meta = lua.create_table()?;
            meta.set("__index", module)?;
            match &self.instance {
                // Recarga: troca as funções, mantém o estado.
                Some(instance) if self.entered => {
                    instance.set_metatable(Some(meta))?;
                }
                _ => {
                    let instance = lua.create_table()?;
                    instance.set_metatable(Some(meta))?;
                    self.instance = Some(instance);
                }
            }
            Ok(())
        })();
        if let Err(e) = result {
            self.set_error(&e);
            return;
        }

        if self.entered {
            self.call(ctx, "reload", ());
        } else {
            self.entered = true;
            let args: Value = match &self.args {
                Some(key) => lua.registry_value(key).unwrap_or(Value::Nil),
                None => Value::Nil,
            };
            self.call(ctx, "enter", args);
        }
    }

    /// Chama um método da cena, se existir. Erros viram a tela de erro.
    fn call(&mut self, ctx: &mut Context, method: &str, args: impl IntoLuaMulti) {
        if self.error.is_some() {
            return;
        }
        let Some(instance) = self.instance.clone() else {
            return;
        };
        let result =
            self.runtime
                .with_ctx(ctx, |_| match instance.get::<Option<Function>>(method)? {
                    Some(f) => f.call::<()>((instance.clone(), args)),
                    None => Ok(()),
                });
        if let Err(e) = result {
            self.set_error(&e);
        }
    }

    fn reload_if_changed(&mut self, ctx: &mut Context) {
        let generation = self.runtime.poll(ctx.assets.hot_reload());
        if generation != self.generation {
            self.generation = generation;
            if let Err(e) = self.runtime.forget_project_modules() {
                self.set_error(&e);
                return;
            }
            log::info!("recarregando cena {}", self.path);
            self.load(ctx);
        }
    }

    fn draw_error(&self, ctx: &mut Context, message: &str) {
        let view = ctx.viewport().as_vec2();
        let mut screen = ctx.renderer.screen();
        screen.draw_rect(Vec2::ZERO, view, Color::hex(0x1A0D10).with_alpha(0.92));
        screen.draw_rect(Vec2::ZERO, Vec2::new(view.x, 6.0), Color::hex(0xE5484D));
        screen.draw_text(
            &Text::new(&format!("Erro no script {}", self.path))
                .at(Vec2::new(24.0, 24.0))
                .size(26.0)
                .color(Color::hex(0xFF8A8A)),
        );
        let lines: Vec<String> = message
            .lines()
            .take(22)
            .map(|l| {
                let l = l.replace('\t', "    ");
                if l.chars().count() > 140 {
                    let mut s: String = l.chars().take(137).collect();
                    s.push_str("...");
                    s
                } else {
                    l
                }
            })
            .collect();
        screen.draw_text(
            &Text::new(&lines.join("\n"))
                .at(Vec2::new(24.0, 70.0))
                .size(17.0)
                .color(Color::hex(0xF2E6E6)),
        );
        screen.draw_text(
            &Text::new("Corrija o arquivo e salve: a cena recarrega sozinha.")
                .at(Vec2::new(24.0, view.y - 24.0))
                .size(17.0)
                .color(Color::hex(0xC9A0A0))
                .anchor(nn_render::Anchor::BottomLeft),
        );
    }
}

impl Scene for LuaScene {
    fn enter(&mut self, ctx: &mut Context) {
        self.load(ctx);
    }

    fn exit(&mut self, ctx: &mut Context) {
        self.call(ctx, "exit", ());
    }

    fn resume(&mut self, ctx: &mut Context) {
        self.call(ctx, "resume", ());
    }

    fn fixed_update(&mut self, ctx: &mut Context) {
        let dt = ctx.fixed_dt();
        self.call(ctx, "fixed_update", dt);
    }

    fn update(&mut self, ctx: &mut Context) -> Transition {
        self.reload_if_changed(ctx);
        let dt = ctx.time.delta_secs();
        self.call(ctx, "update", dt);

        // Com erro na tela, Esc fecha o jogo (não há outra saída).
        if self.error.is_some() && ctx.input.key_pressed(nn_input::KeyCode::Escape) {
            return Transition::Quit;
        }

        match self.runtime.take_request() {
            None => Transition::None,
            Some(SceneRequest::Pop) => Transition::Pop,
            Some(SceneRequest::Quit) => Transition::Quit,
            Some(SceneRequest::Push(path, args)) => {
                Transition::Push(Box::new(LuaScene::new(self.runtime.clone(), path, args)))
            }
            Some(SceneRequest::Replace(path, args)) => {
                Transition::Replace(Box::new(LuaScene::new(self.runtime.clone(), path, args)))
            }
            Some(SceneRequest::Switch(path, args)) => {
                Transition::Switch(Box::new(LuaScene::new(self.runtime.clone(), path, args)))
            }
        }
    }

    fn render(&mut self, ctx: &mut Context) {
        // Cenas cobertas por uma pausa não recebem update, mas recarregam aqui.
        self.reload_if_changed(ctx);
        self.call(ctx, "draw", ());
        if let Some(message) = self.error.clone() {
            self.draw_error(ctx, &message);
        }
    }

    fn is_overlay(&self) -> bool {
        self.instance
            .as_ref()
            .and_then(|i| i.get::<Option<bool>>("overlay").ok().flatten())
            .unwrap_or(false)
    }
}

/// Deixa a mensagem de erro do Lua mais legível.
fn format_error(e: &mlua::Error) -> String {
    let mut msg = match e {
        mlua::Error::CallbackError { traceback, cause } => {
            format!("{}\n{traceback}", root_cause(cause))
        }
        other => other.to_string(),
    };
    for prefix in ["runtime error: ", "syntax error: "] {
        if let Some(rest) = msg.strip_prefix(prefix) {
            msg = rest.to_string();
        }
    }
    msg.trim().to_string()
}

fn root_cause(e: &mlua::Error) -> String {
    match e {
        mlua::Error::CallbackError { cause, .. } => root_cause(cause),
        mlua::Error::RuntimeError(m) => m.clone(),
        other => other.to_string(),
    }
}

/// Um projeto de jogo em Lua: uma pasta com `main.lua` (ou a cena indicada em
/// `config.lua`) e os assets.
#[derive(Debug, Clone)]
pub struct Project {
    /// Configuração da janela e da engine.
    pub config: nn_app::AppConfig,
    /// Cena inicial, relativa à pasta do projeto.
    pub start: String,
}

impl Project {
    /// Lê a pasta do projeto. O arquivo `config.lua` é opcional e devolve uma
    /// tabela com `title`, `width`, `height`, `vsync`, `fixed_hz`,
    /// `clear_color`, `resizable`, `hot_reload` e `start`.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self, ScriptError> {
        let dir = dir.as_ref();
        if !dir.is_dir() {
            return Err(ScriptError(format!(
                "pasta do projeto não encontrada: {}",
                dir.display()
            )));
        }
        let mut config = nn_app::AppConfig::new(
            dir.canonicalize()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "nn-engine".into()),
        )
        .assets_dir(dir);
        let mut start = "main.lua".to_string();

        let config_file = dir.join("config.lua");
        if config_file.exists() {
            let source = std::fs::read_to_string(&config_file)
                .map_err(|e| ScriptError(format!("{}: {e}", config_file.display())))?;
            let lua = Lua::new();
            let t: Table = lua
                .load(&source)
                .set_name("@config.lua")
                .eval()
                .map_err(|e| ScriptError(format_error(&e)))?;
            let get = |k: &str| -> Result<Value, ScriptError> {
                t.get(k)
                    .map_err(|e| ScriptError(format!("config.lua: {k}: {e}")))
            };
            let num = |v: &Value| v.as_f64().map(|n| n as u32);
            if let Value::String(s) = get("title")? {
                config.title = s.to_string_lossy();
            }
            if let (Some(w), Some(h)) = (num(&get("width")?), num(&get("height")?)) {
                config.size = (w, h);
            }
            if let Some(b) = get("vsync")?.as_boolean() {
                config.vsync = b;
            }
            if let Some(b) = get("resizable")?.as_boolean() {
                config.resizable = b;
            }
            if let Some(b) = get("hot_reload")?.as_boolean() {
                config.hot_reload = b;
            }
            if let Some(hz) = num(&get("fixed_hz")?) {
                config.fixed_hz = hz.max(1);
            }
            let clear = get("clear_color")?;
            if !clear.is_nil() {
                config.clear_color = convert::color(&clear)
                    .map_err(|e| ScriptError(format!("config.lua: clear_color: {e}")))?;
            }
            if let Value::String(s) = get("start")? {
                start = s.to_string_lossy();
            }
        }

        if !dir.join(&start).exists() {
            return Err(ScriptError(format!(
                "cena inicial não encontrada: {}",
                dir.join(&start).display()
            )));
        }
        Ok(Self { config, start })
    }
}

/// Abre a janela e roda o projeto Lua da pasta `dir`.
pub fn run_project(dir: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    let project = Project::open(dir.as_ref())?;
    let root = project.config.assets_dir.clone();
    let runtime = ScriptRuntime::new(root)?;
    let start = project.start.clone();
    nn_app::run(project.config, move |ctx| {
        nn_app::SceneStack::new(ctx, runtime.scene(start))
    })?;
    Ok(())
}
