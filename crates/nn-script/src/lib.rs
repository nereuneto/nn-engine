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

mod anim3d;
mod api;
pub mod api_docs;
mod camera3d;
mod convert;
mod d3;
mod errors;
mod input;
pub mod live;
mod particles;
mod physics;
mod physics3d;
mod save;
mod tilemap;
mod types;
mod ui;

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use glam::Vec2;
use mlua::{Function, IntoLuaMulti, Lua, RegistryKey, Table, Value};
use nn_app::{AssetSource, Context, DiskSource, Scene, Transition};
use nn_core::Color;
use nn_render::Text;

use crate::api::{CTX_KEY, SceneRequest, Shared};

pub use errors::translate_error;

/// Código Lua que roda as cenas visuais (`.cena`). Também traz a lista de
/// componentes e seus campos padrão (`FIELDS`), que o NN Studio usa.
pub const SCENE_RUNTIME: &str = include_str!("cena.lua");

/// Eventos "Quando... Então..." das cenas visuais. Recebe o módulo das cenas
/// (`SCENE_RUNTIME`) e devolve o módulo dos eventos, com o catálogo de
/// condições e ações (`catalog()`), que o NN Studio também lê.
pub const EVENTS_RUNTIME: &str = include_str!("eventos.lua");

/// Código Lua da interação dos menus das cenas visuais (foco, mouse,
/// teclado, controle e digitação).
pub const INTERFACE_RUNTIME: &str = include_str!("interface.lua");

/// Extensão dos arquivos de cena visual.
pub const SCENE_EXTENSION: &str = "cena";

/// Extensão dos prefabs (um objeto pronto para usar em várias cenas).
pub const PREFAB_EXTENSION: &str = "prefab";

/// Extensão das animações de uma imagem (`heroi.png` → `heroi.anim`).
pub const ANIM_EXTENSION: &str = "anim";

/// Comportamentos prontos (`"nn:<id>"` no componente `behavior`): id e
/// código Lua. O NN Studio lê as propriedades de cada um.
pub const BEHAVIORS: &[(&str, &str)] = &[
    ("plataforma", include_str!("comportamentos/plataforma.lua")),
    ("topdown", include_str!("comportamentos/topdown.lua")),
    ("patrulha", include_str!("comportamentos/patrulha.lua")),
    ("perseguir", include_str!("comportamentos/perseguir.lua")),
    ("coletavel", include_str!("comportamentos/coletavel.lua")),
    ("perigo", include_str!("comportamentos/perigo.lua")),
    ("vida", include_str!("comportamentos/vida.lua")),
    ("checkpoint", include_str!("comportamentos/checkpoint.lua")),
    (
        "ir_para_cena",
        include_str!("comportamentos/ir_para_cena.lua"),
    ),
    ("animacao", include_str!("comportamentos/animacao.lua")),
    ("movimento", include_str!("comportamentos/movimento.lua")),
    ("atirar", include_str!("comportamentos/atirar.lua")),
    ("gerador", include_str!("comportamentos/gerador.lua")),
    ("temporario", include_str!("comportamentos/temporario.lua")),
    (
        "mostrar_valor",
        include_str!("comportamentos/mostrar_valor.lua"),
    ),
    ("botao", include_str!("comportamentos/botao.lua")),
    ("jogador3d", include_str!("comportamentos/jogador3d.lua")),
    ("girar3d", include_str!("comportamentos/girar3d.lua")),
    (
        "plataforma3d",
        include_str!("comportamentos/plataforma3d.lua"),
    ),
];

/// Variáveis do jogo (`nn.vars`): começam com o `vars` do config.lua e valem
/// entre as cenas.
const VARS_LUA: &str = r#"
local initial = ...
local function copy(v)
  if type(v) ~= "table" then return v end
  local t = {}
  for k, x in pairs(v) do t[k] = copy(x) end
  return t
end
nn.vars = copy(initial)
--- Volta as variáveis do jogo aos valores do config.lua.
function nn.reset_vars()
  for k in pairs(nn.vars) do nn.vars[k] = nil end
  for k, v in pairs(copy(initial)) do nn.vars[k] = v end
end
-- O config.lua mudou com o jogo aberto: variáveis novas aparecem.
function nn._merge_vars(new)
  initial = new
  for k, v in pairs(new) do
    if nn.vars[k] == nil then nn.vars[k] = copy(v) end
  end
end
"#;
/// Chave do registro Lua com o módulo das cenas visuais.
const SCENE_RUNTIME_KEY: &str = "nn.cena";

/// `nn.assets.animations(imagem)`: as animações do `.anim` da imagem, prontas
/// para `nn.animator` (usa a mesma leitura das cenas visuais).
const ANIMATIONS_LUA: &str = r#"
local cena = ...
function nn.assets.animations(image)
  local path = cena.anim_path(image)
  local data = cena.parse_anims(cena.load_data(path, "arquivo de animações"), path)
  local out = {}
  for _, a in ipairs(data.list) do
    out[a.name] = nn.animation(a.frames, a.fps, a.mode, a.times)
  end
  return out
end
"#;

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
    source: Arc<dyn AssetSource>,
    /// Módulos padrão do Lua, que nunca são descarregados na recarga.
    builtin_modules: HashSet<String>,
    generation: Cell<u64>,
    watch: RefCell<Watch>,
}

#[derive(Default)]
struct Watch {
    files: Vec<(String, Option<SystemTime>)>,
    last_poll: Option<Instant>,
}

impl std::fmt::Debug for ScriptRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptRuntime")
            .field("source", &self.inner.source)
            .finish()
    }
}

impl ScriptRuntime {
    /// Intervalo entre verificações de scripts alterados.
    pub const POLL_INTERVAL: Duration = Duration::from_millis(250);

    /// Cria o ambiente com os scripts lidos da pasta `root` (geralmente a
    /// mesma dos assets). `require "lib.util"` busca `root/lib/util.lua`.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, ScriptError> {
        Self::with_source(Arc::new(DiskSource::new(root)))
    }

    /// Cria o ambiente com os scripts lidos de `source` (pasta ou pacote).
    pub fn with_source(source: Arc<dyn AssetSource>) -> Result<Self, ScriptError> {
        let lua = Lua::new();
        api::install(&lua)?;
        install_require(&lua, source.clone())?;
        let scene_runtime: Table = lua.load(SCENE_RUNTIME).set_name("@nn/cena.lua").eval()?;
        let builtin: Table = scene_runtime.get("BUILTIN")?;
        for (id, src) in BEHAVIORS {
            builtin.set(*id, *src)?;
        }
        let events: Table = lua
            .load(EVENTS_RUNTIME)
            .set_name("@nn/eventos.lua")
            .call(scene_runtime.clone())?;
        scene_runtime.set("events", events)?;
        let interface: Table = lua
            .load(INTERFACE_RUNTIME)
            .set_name("@nn/interface.lua")
            .call(scene_runtime.clone())?;
        scene_runtime.set("ui", interface)?;
        lua.load(ANIMATIONS_LUA)
            .set_name("@nn/animacoes.lua")
            .call::<()>(scene_runtime.clone())?;
        lua.set_named_registry_value(SCENE_RUNTIME_KEY, scene_runtime)?;
        set_theme(&lua, source.as_ref());
        let vars = config_vars(&lua, source.as_ref());
        lua.load(VARS_LUA)
            .set_name("@nn/vars.lua")
            .call::<()>(vars)?;

        let loaded: Table = lua.globals().get::<Table>("package")?.get("loaded")?;
        let builtin_modules = loaded
            .pairs::<String, Value>()
            .filter_map(|p| p.ok().map(|(k, _)| k))
            .collect();

        let runtime = Self {
            inner: Rc::new(Inner {
                lua,
                source,
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

    /// De onde vêm os scripts.
    pub fn source(&self) -> &Arc<dyn AssetSource> {
        &self.inner.source
    }

    /// O estado Lua, para quem quiser registrar funções próprias.
    pub fn lua(&self) -> &Lua {
        &self.inner.lua
    }

    /// Cria uma cena a partir de um arquivo, relativo à pasta dos scripts.
    pub fn scene(&self, path: impl Into<String>) -> LuaScene {
        LuaScene::new(self.clone(), path, None)
    }

    /// Verifica a sintaxe de todos os `.lua`, sem executar nada.
    /// Devolve as mensagens de erro (vazio se tudo estiver certo).
    pub fn check_syntax(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let mut files = self.inner.source.files("lua");
        files.extend(self.inner.source.files(SCENE_EXTENSION));
        files.extend(self.inner.source.files(PREFAB_EXTENSION));
        files.extend(self.inner.source.files(ANIM_EXTENSION));
        for (path, _) in files {
            let result = self
                .inner
                .source
                .read(Path::new(&path))
                .map_err(|e| e.to_string())
                .and_then(|bytes| {
                    self.inner
                        .lua
                        .load(&bytes)
                        .set_name(format!("@{path}"))
                        .into_function()
                        .map(|_| ())
                        .map_err(|e| format_error(&e))
                });
            if let Err(e) = result {
                errors.push(e);
            }
        }
        errors
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

    fn take_request(&self) -> Option<(SceneRequest, Option<nn_app::Fade>)> {
        self.inner
            .lua
            .app_data_mut::<Shared>()
            .and_then(|mut s| s.request.take())
    }

    /// Carrega (executa) o arquivo de uma cena e devolve a tabela que ele retorna.
    fn load_module(&self, ctx: &mut Context, path: &str) -> mlua::Result<Table> {
        let bytes = self.inner.source.read(Path::new(path)).map_err(|e| {
            mlua::Error::RuntimeError(format!(
                "não foi possível ler {}: {e}",
                self.inner.source.describe(Path::new(path)).display()
            ))
        })?;
        if path.ends_with(&format!(".{SCENE_EXTENSION}")) {
            // Cena visual: os dados viram uma cena pelo módulo embutido.
            return self.with_ctx(ctx, |lua| {
                let data: Value = lua.load(&bytes).set_name(format!("@{path}")).eval()?;
                let runtime: Table = lua.named_registry_value(SCENE_RUNTIME_KEY)?;
                runtime.get::<Function>("new")?.call((data, path))
            });
        }
        let value: Value = self.with_ctx(ctx, |lua| {
            lua.load(&bytes).set_name(format!("@{path}")).eval()
        })?;
        match value {
            Value::Table(t) => Ok(t),
            other => Err(mlua::Error::RuntimeError(format!(
                "{path}: o script de cena deve terminar com `return` de uma tabela (recebeu {})",
                other.type_name()
            ))),
        }
    }

    /// Variáveis novas no config.lua passam a existir em `nn.vars`.
    fn merge_config_vars(&self) {
        let lua = &self.inner.lua;
        set_theme(lua, self.inner.source.as_ref());
        let vars = config_vars(lua, self.inner.source.as_ref());
        let merge = lua
            .globals()
            .get::<Table>("nn")
            .and_then(|nn| nn.get::<Function>("_merge_vars"));
        if let Ok(f) = merge
            && let Err(e) = f.call::<()>(vars)
        {
            log::warn!("variáveis do config.lua: {e}");
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

    fn scan(&self) -> Vec<(String, Option<SystemTime>)> {
        if self.inner.source.is_packed() {
            return Vec::new();
        }
        let mut files = self.inner.source.files("lua");
        files.extend(self.inner.source.files(SCENE_EXTENSION));
        files.extend(self.inner.source.files(PREFAB_EXTENSION));
        files.extend(self.inner.source.files(ANIM_EXTENSION));
        // Mapas e tilesets: salvar no editor recarrega a cena.
        files.extend(self.inner.source.files("tmj"));
        files.extend(self.inner.source.files("tsj"));
        files.sort();
        files
    }

    /// Verifica se algum `.lua` mudou. Devolve a geração atual; ela aumenta a
    /// cada mudança detectada.
    fn poll(&self, hot_reload: bool) -> u64 {
        if hot_reload && !self.inner.source.is_packed() {
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
                        .filter(|f| !w.files.contains(f))
                        .map(|(p, _)| p.clone())
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

/// A tabela `vars` do config.lua (vazia se não houver).
/// `nn.theme`: o tema da interface (`theme` do config.lua).
fn set_theme(lua: &Lua, source: &dyn AssetSource) {
    let read = || -> Option<Table> {
        let bytes = source.read(Path::new("config.lua")).ok()?;
        let config: Table = lua.load(&bytes).set_name("@config.lua").eval().ok()?;
        config.get::<Option<Table>>("theme").ok().flatten()
    };
    let theme = read().unwrap_or_else(|| lua.create_table().expect("tabela Lua"));
    if let Ok(nn) = lua.globals().get::<Table>("nn") {
        let _ = nn.set("theme", theme);
    }
}

fn config_vars(lua: &Lua, source: &dyn AssetSource) -> Table {
    let read = || -> Option<Table> {
        let bytes = source.read(Path::new("config.lua")).ok()?;
        let config: Table = lua.load(&bytes).set_name("@config.lua").eval().ok()?;
        config.get::<Option<Table>>("vars").ok().flatten()
    };
    read().unwrap_or_else(|| lua.create_table().expect("tabela Lua"))
}

/// Troca a busca de módulos do `require` por uma que lê da mesma origem dos
/// assets (pasta ou pacote): `require "lib.util"` procura `lib/util.lua` e
/// `lib/util/init.lua`.
fn install_require(lua: &Lua, source: Arc<dyn AssetSource>) -> mlua::Result<()> {
    let package: Table = lua.globals().get("package")?;
    let searchers: Table = package.get("searchers")?;
    let preload: Value = searchers.get(1)?;
    let searcher = lua.create_function(move |lua, name: String| {
        let base = name.replace('.', "/");
        let candidates = [format!("{base}.lua"), format!("{base}/init.lua")];
        for path in &candidates {
            if let Ok(bytes) = source.read(Path::new(path)) {
                let chunk = lua
                    .load(&bytes)
                    .set_name(format!("@{path}"))
                    .into_function()?;
                return Ok((
                    Value::Function(chunk),
                    Value::String(lua.create_string(path)?),
                ));
            }
        }
        let tried: String = candidates
            .iter()
            .map(|c| format!("\n\tarquivo '{c}' não encontrado"))
            .collect();
        Ok((Value::String(lua.create_string(&tried)?), Value::Nil))
    })?;
    let list = lua.create_table()?;
    list.push(preload)?;
    list.push(searcher)?;
    package.set("searchers", list)?;
    package.set("path", "")?;
    package.set("cpath", "")?;
    Ok(())
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
    /// Pausado pelo editor e pediram para avançar: roda um passo fixo.
    step_fixed: bool,
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
            step_fixed: false,
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

    /// A tabela `self` da cena (para ferramentas e testes).
    pub fn table(&self) -> Option<Table> {
        self.instance.clone()
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
            live::announce(&self.path);
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
            self.runtime.merge_config_vars();
            log::info!("recarregando cena {}", self.path);
            self.load(ctx);
        }
    }

    /// Cena da frente, aberta pelo editor: manda o estado dela e o Ctrl+clique.
    fn live_tick(&mut self, ctx: &mut Context) {
        if self.error.is_some() {
            return;
        }
        let Some(instance) = self.instance.clone() else {
            return;
        };
        let lua = self.runtime.lua().clone();
        let result = self.runtime.with_ctx(ctx, |_| {
            let Some(f) = instance.get::<Option<Function>>("_live_tick")? else {
                return Ok((Value::Nil, None));
            };
            f.call::<(Value, Option<i64>)>(instance.clone())
        });
        match result {
            Ok((state, picked)) => {
                if let Value::Table(_) = &state
                    && let Ok(mut json) =
                        mlua::LuaSerdeExt::from_value::<serde_json::Value>(&lua, state)
                {
                    json["paused"] = serde_json::Value::Bool(live::paused());
                    live::report(&json);
                }
                if let Some(id) = picked {
                    live::select(id);
                }
            }
            Err(e) => log::debug!("estado ao vivo: {}", format_error(&e)),
        }
    }

    /// Ajustes ao vivo vindos do editor (só cenas visuais os recebem).
    fn apply_live(&mut self, ctx: &mut Context) {
        if self.error.is_some() {
            return;
        }
        let messages = live::take(&self.path);
        if messages.is_empty() {
            return;
        }
        let Some(instance) = self.instance.clone() else {
            return;
        };
        let lua = self.runtime.lua().clone();
        for msg in messages {
            let result = self.runtime.with_ctx(ctx, |_| {
                let Some(f) = instance.get::<Option<Function>>("_live")? else {
                    return Ok(());
                };
                let value = mlua::LuaSerdeExt::to_value(&lua, &msg)?;
                f.call::<()>((instance.clone(), value))
            });
            if let Err(e) = result {
                log::warn!("ajuste ao vivo não aplicado: {}", format_error(&e));
            }
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
        live::announce(&self.path);
        self.call(ctx, "resume", ());
    }

    fn fixed_update(&mut self, ctx: &mut Context) {
        // Pausado pelo editor: só anda o passo pedido com "avançar".
        if live::paused() {
            if !self.step_fixed {
                return;
            }
            self.step_fixed = false;
        }
        let dt = ctx.fixed_dt();
        self.call(ctx, "fixed_update", dt);
    }

    fn update(&mut self, ctx: &mut Context) -> Transition {
        self.reload_if_changed(ctx);
        if live::enabled() {
            self.live_tick(ctx);
        }
        let run = !live::paused() || {
            let step = live::take_step();
            self.step_fixed = step;
            step
        };
        let dt = ctx.time.delta_secs();
        if run {
            self.call(ctx, "update", dt);
        }

        // Com erro na tela, Esc fecha o jogo (não há outra saída).
        if self.error.is_some() && ctx.input.key_pressed(nn_input::KeyCode::Escape) {
            return Transition::Quit;
        }

        let Some((request, fade)) = self.runtime.take_request() else {
            return Transition::None;
        };
        let scene = |path, args| Box::new(LuaScene::new(self.runtime.clone(), path, args));
        let transition = match request {
            SceneRequest::Pop => Transition::Pop,
            SceneRequest::Quit => Transition::Quit,
            SceneRequest::Push(path, args) => Transition::Push(scene(path, args)),
            SceneRequest::Replace(path, args) => Transition::Replace(scene(path, args)),
            SceneRequest::Switch(path, args) => Transition::Switch(scene(path, args)),
        };
        match fade {
            Some(f) => transition.fade_with(f.duration, f.color),
            None => transition,
        }
    }

    fn render(&mut self, ctx: &mut Context) {
        // Cenas cobertas por uma pausa não recebem update, mas recarregam aqui.
        self.reload_if_changed(ctx);
        self.apply_live(ctx);
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
    translate_error(msg.trim())
}

fn root_cause(e: &mlua::Error) -> String {
    match e {
        mlua::Error::CallbackError { cause, .. } => root_cause(cause),
        mlua::Error::RuntimeError(m) => m.clone(),
        other => other.to_string(),
    }
}

/// Um projeto de jogo em Lua: uma pasta (ou pacote) com `main.lua` (ou a
/// cena indicada em `config.lua`) e os assets.
#[derive(Debug, Clone)]
pub struct Project {
    /// Configuração da janela e da engine.
    pub config: nn_app::AppConfig,
    /// Cena inicial, relativa à raiz do projeto.
    pub start: String,
}

impl Project {
    /// Lê a pasta do projeto. O arquivo `config.lua` é opcional e devolve uma
    /// tabela com `title`, `id`, `width`, `height`, `resolution`, `scale`,
    /// `fullscreen`, `vsync`, `fixed_hz`, `clear_color`, `resizable`,
    /// `hot_reload` e `start`.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self, ScriptError> {
        let dir = dir.as_ref();
        if !dir.is_dir() {
            return Err(ScriptError(format!(
                "pasta do projeto não encontrada: {}",
                dir.display()
            )));
        }
        let title = dir
            .canonicalize()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "nn-engine".into());
        // Projeto em pasta = jogo em desenvolvimento: recarrega ao salvar,
        // a não ser que o config.lua diga `hot_reload = false`.
        let mut project = Self::read(Arc::new(DiskSource::new(dir)), &title, true)?;
        project.config.assets_dir = dir.to_path_buf();
        Ok(project)
    }

    /// Lê o projeto de uma origem qualquer (pasta ou pacote de um jogo exportado).
    pub fn from_source(
        source: Arc<dyn AssetSource>,
        default_title: &str,
    ) -> Result<Self, ScriptError> {
        let hot_reload = nn_app::AppConfig::default().hot_reload;
        Self::read(source, default_title, hot_reload)
    }

    fn read(
        source: Arc<dyn AssetSource>,
        default_title: &str,
        hot_reload: bool,
    ) -> Result<Self, ScriptError> {
        let mut config = nn_app::AppConfig::new(default_title)
            .asset_source(source.clone())
            .hot_reload(hot_reload);
        let mut start = "main.lua".to_string();

        match source.read(Path::new("config.lua")) {
            Ok(bytes) => {
                let lua = Lua::new();
                let t: Table = lua
                    .load(&bytes)
                    .set_name("@config.lua")
                    .eval()
                    .map_err(|e| ScriptError(format_error(&e)))?;
                let get = |k: &str| -> Result<Value, ScriptError> {
                    t.get(k)
                        .map_err(|e| ScriptError(format!("config.lua: {k}: {e}")))
                };
                // Lua 5.4 separa inteiros (1280) de números com vírgula (1280.0).
                let num = |v: &Value| match v {
                    Value::Integer(i) => Some((*i).max(0) as u32),
                    Value::Number(n) => Some(n.max(0.0) as u32),
                    _ => None,
                };
                if let Value::String(s) = get("title")? {
                    config.title = s.to_string_lossy();
                }
                if let Value::String(s) = get("id")? {
                    config.id = Some(s.to_string_lossy());
                }
                if let Some(b) = get("fullscreen")?.as_boolean() {
                    config.fullscreen = b;
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
                // Tamanho fixo do jogo: resolution = { 640, 360 }.
                match get("resolution")? {
                    Value::Nil | Value::Boolean(false) => {}
                    Value::Table(r) => {
                        let w = num(&r.get::<Value>(1).unwrap_or(Value::Nil));
                        let h = num(&r.get::<Value>(2).unwrap_or(Value::Nil));
                        match (w, h) {
                            (Some(w), Some(h)) if w > 0 && h > 0 => {
                                config = config.resolution(w, h);
                            }
                            _ => {
                                return Err(ScriptError(
                                    "config.lua: resolution deve ser { largura, altura }, \
                                     por exemplo { 640, 360 }"
                                        .into(),
                                ));
                            }
                        }
                    }
                    _ => {
                        return Err(ScriptError(
                            "config.lua: resolution deve ser { largura, altura }, por exemplo { 640, 360 }"
                                .into(),
                        ));
                    }
                }
                if let Value::String(s) = get("scale")? {
                    let s = s.to_string_lossy();
                    config.scale_mode = nn_render::ScaleMode::from_name(&s).ok_or_else(|| {
                        ScriptError(format!(
                            "config.lua: scale = \"{s}\" não existe (use \"fit\", \"stretch\" ou \"pixel\")"
                        ))
                    })?;
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
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(ScriptError(format!("config.lua: {e}"))),
        }

        if source.read(Path::new(&start)).is_err() {
            return Err(ScriptError(format!(
                "cena inicial não encontrada: {}",
                source.describe(Path::new(&start)).display()
            )));
        }
        Ok(Self { config, start })
    }

    /// Origem dos arquivos do projeto.
    pub fn source(&self) -> Arc<dyn AssetSource> {
        self.config
            .asset_source
            .clone()
            .unwrap_or_else(|| Arc::new(DiskSource::new(&self.config.assets_dir)))
    }
}

/// Abre a janela e roda o projeto Lua da pasta `dir`.
pub fn run_project(dir: impl AsRef<Path>) -> Result<(), Box<dyn std::error::Error>> {
    run(Project::open(dir.as_ref())?)
}

/// Abre a janela e roda um projeto já aberto (de pasta ou pacote).
pub fn run(project: Project) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = ScriptRuntime::with_source(project.source())?;
    let start = project.start.clone();
    nn_app::run(project.config, move |ctx| {
        nn_app::SceneStack::new(ctx, runtime.scene(start))
    })?;
    Ok(())
}
