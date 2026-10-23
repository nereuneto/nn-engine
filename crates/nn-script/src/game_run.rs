//! Rodar um jogo sem janela, com teclas e cliques simulados, e ver o que
//! aconteceu: os erros, os `print`, os avisos, as regras dos eventos que
//! dispararam, as trocas de cena, o estado dos objetos e capturas da tela.
//!
//! É a base dos testes de jogo ([`crate::testing`], `nn testar`) e da
//! ferramenta `rodar_jogo` do servidor MCP.
//!
//! ```no_run
//! use nn_script::game_run::{GameRun, Plan, Action, RunOptions};
//! let mut run = GameRun::open("meu-jogo", RunOptions::default()).unwrap();
//! let plan = Plan {
//!     duration: 2.0,
//!     actions: vec![Action::hold(0.0, "right", 1.0), Action::press(1.0, "space")],
//! };
//! run.run_plan(&plan).unwrap();
//! let report = run.take_report();
//! println!("{} erros", report.errors.len());
//! ```
//!
//! O jogo roda a 60 quadros por segundo de jogo, o mais rápido que der (não
//! no tempo do relógio), sem som, com o `math.random` sempre na mesma
//! sequência e com o salvo (`nn.save`) só na memória: rodar não mexe nos
//! dados de quem joga.

use std::cell::Cell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::{Rc, Weak};
use std::time::{Duration, Instant};

use mlua::{Function, HookTriggers, IntoLuaMulti, LuaSerdeExt, MultiValue, Table, Value, VmState};
use nn_app::{Headless, SceneStack};

use crate::capture::{self, Guard, LogLine, Sink};
use crate::{Inner, Project, SCENE_RUNTIME_KEY, SceneError, ScriptRuntime, format_error};

/// O código Lua do jogo sem janela (a API dos testes, o roteiro e o estado).
pub const DRIVER_RUNTIME: &str = include_str!("testes.lua");

/// Quadros por segundo de jogo.
pub const FPS: f64 = 60.0;

/// Como rodar.
#[derive(Debug, Clone)]
pub struct RunOptions {
    /// Cena para começar (padrão: a cena inicial do `config.lua`).
    pub scene: Option<String>,
    /// Tamanho da tela em pixels (padrão: o tamanho fixo do jogo, ou o da
    /// janela do `config.lua`).
    pub size: Option<(u32, u32)>,
    /// Quanto o Lua pode ficar rodando sem voltar (um laço sem fim) antes de
    /// o quadro parar com erro.
    pub frame_limit: Duration,
    /// Semente do `math.random`.
    pub seed: i64,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            scene: None,
            size: None,
            frame_limit: Duration::from_secs(5),
            seed: 1,
        }
    }
}

/// Uma captura da tela.
#[derive(Debug, Clone)]
pub struct Capture {
    pub label: String,
    /// Segundos de jogo.
    pub time: f64,
    pub width: u32,
    pub height: u32,
    /// A imagem em PNG.
    pub png: Vec<u8>,
}

/// Uma regra dos eventos que disparou.
#[derive(Debug, Clone, PartialEq)]
pub struct FiredRule {
    /// A cena da regra.
    pub scene: String,
    /// "regra 2 da cena: quando pegar → somar, som".
    pub text: String,
    /// Quando disparou pela primeira vez (segundos de jogo).
    pub first: f64,
    /// Quantas vezes disparou (somando as cópias do mesmo prefab).
    pub count: u32,
    /// Quando disparou (as primeiras vezes).
    pub times: Vec<f64>,
}

/// A cena da frente mudou.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneChange {
    pub time: f64,
    pub scene: String,
}

/// Um erro que parou uma cena.
#[derive(Debug, Clone, PartialEq)]
pub struct RunError {
    pub time: f64,
    pub error: SceneError,
}

/// O que aconteceu desde o último [`GameRun::take_report`].
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// Prints do Lua, avisos e erros da engine, na ordem.
    pub lines: Vec<LogLine>,
    /// Linhas que ficaram de fora por serem muitas.
    pub dropped_lines: usize,
    /// Erros que pararam cenas.
    pub errors: Vec<RunError>,
    pub fired: Vec<FiredRule>,
    /// A cena inicial e as trocas.
    pub scenes: Vec<SceneChange>,
    pub captures: Vec<Capture>,
    /// Os sons que tocaram: (arquivo, quantas vezes, a primeira vez).
    pub sounds: Vec<(String, u32, f64)>,
    /// Quando a cena da frente reiniciou (a mesma cena começou de novo).
    pub restarts: Vec<f64>,
    /// As animações 3D que começaram a tocar: (modelo, animação, quantas
    /// vezes, a primeira vez).
    pub animations: Vec<(String, String, u32, f64)>,
    /// O tempo gasto: a soma dos quadros e o pior (veja [`Perf`]).
    pub perf: Perf,
    /// O jogo fechou (`nn.quit`) neste tempo.
    pub quit: Option<f64>,
}

/// Quanto o jogo gastou, somando os quadros rodados (sem desenhar na placa
/// de vídeo: o desenho aqui é só o `draw` do jogo).
#[derive(Debug, Clone, Copy, Default)]
pub struct Perf {
    pub frames: u32,
    /// Somas, em ms.
    pub total: nn_app::FrameTimes,
    /// O quadro mais pesado (passos fixos + update + desenho), em ms, e quando.
    pub worst: (f32, f64),
}

/// Uma ação do roteiro de [`GameRun::run_plan`].
#[derive(Debug, Clone, PartialEq)]
pub struct Action {
    /// Quando (segundos desde o começo do jogo).
    pub at: f64,
    pub kind: ActionKind,
}

/// O que a ação faz.
#[derive(Debug, Clone, PartialEq)]
pub enum ActionKind {
    /// Aperta e solta a tecla.
    Press(String),
    /// Segura a tecla por um tempo (segundos).
    Hold(String, f64),
    /// Aperta a tecla (até um `Up`).
    Down(String),
    Up(String),
    /// Clica num ponto da tela (pixels, y para baixo).
    Click {
        x: f32,
        y: f32,
        button: String,
    },
    /// Clica no objeto com esse nome (na cena da frente).
    ClickObject {
        name: String,
        button: String,
    },
    /// Põe o mouse num ponto da tela.
    Move {
        x: f32,
        y: f32,
    },
    /// Escreve um texto.
    Type(String),
    /// Captura a tela.
    Capture(String),
    /// Roda código Lua com `self` = a cena da frente.
    Lua(String),
}

impl Action {
    pub fn press(at: f64, key: &str) -> Self {
        Self {
            at,
            kind: ActionKind::Press(key.into()),
        }
    }

    pub fn hold(at: f64, key: &str, seconds: f64) -> Self {
        Self {
            at,
            kind: ActionKind::Hold(key.into(), seconds),
        }
    }
}

/// Um roteiro: ações em certos tempos, e quanto tempo rodar no total.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Plan {
    pub duration: f64,
    pub actions: Vec<Action>,
}

/// Como terminou um roteiro ou um teste.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Chegou ao fim.
    Done,
    /// O código do teste (ou uma ação `lua`) deu erro: a mensagem.
    Failed(String),
    /// O jogo deu erro (está em [`Report::errors`]) e parou aqui.
    GameError(RunError),
    /// Passou do tempo: a mensagem.
    TimedOut(String),
}

/// Limites de tempo de um roteiro ou teste.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Segundos de jogo.
    pub game: f64,
    /// Tempo de relógio.
    pub wall: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            game: 600.0,
            wall: Duration::from_secs(120),
        }
    }
}

/// Um jogo rodando sem janela.
pub struct GameRun {
    // Ordem de saída: o jogo (as cenas) e a tabela do Lua antes do ambiente Lua.
    headless: Headless<SceneStack>,
    driver: Table,
    runtime: ScriptRuntime,
    sink: Rc<Sink>,
    _guard: Guard,
    deadline: Rc<Cell<Option<Instant>>>,
    frame_limit: Duration,
    frames: u64,
    report: Report,
    front: String,
    /// A tabela da cena da frente (muda quando a cena reinicia).
    front_ptr: usize,
    fired_index: HashMap<(String, String), usize>,
    ended: bool,
}

impl std::fmt::Debug for GameRun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameRun")
            .field("frames", &self.frames)
            .field("front", &self.front)
            .finish()
    }
}

impl GameRun {
    /// Abre o projeto da pasta e começa o jogo (a cena inicial já começou).
    pub fn open(dir: impl AsRef<Path>, opts: RunOptions) -> Result<Self, String> {
        let project = Project::open(dir.as_ref()).map_err(|e| e.to_string())?;
        Self::new(project, opts)
    }

    /// Começa o jogo de um projeto já aberto.
    pub fn new(project: Project, opts: RunOptions) -> Result<Self, String> {
        capture::ensure_installed();
        let (sink, guard) = capture::start();
        let source = project.source();
        let start = opts.scene.clone().unwrap_or_else(|| project.start.clone());
        if source.read(Path::new(&start)).is_err() {
            return Err(format!("a cena {start} não existe"));
        }
        let runtime = ScriptRuntime::with_source(source).map_err(|e| e.to_string())?;
        let lua = runtime.lua().clone();
        lua.load(format!("math.randomseed({})", opts.seed))
            .set_name("=semente")
            .exec()
            .map_err(|e| e.to_string())?;

        crate::anim3d::record_plays(true);
        // Os sons que tocam (sem placa de som não se ouve nada; assim o
        // relatório diz o que tocou).
        lua.load(
            r#"
            local names = setmetatable({}, { __mode = "k" })
            local load = nn.assets.sound
            nn.assets.sound = function(path, ...)
              local s = load(path, ...)
              if s ~= nil then names[s] = path end
              return s
            end
            local play = nn.audio.play
            _nn_tocados = {}
            nn.audio.play = function(s, ...)
              table.insert(_nn_tocados, names[s] or "(um som)")
              return play(s, ...)
            end
            "#,
        )
        .set_name("=sons")
        .exec()
        .map_err(|e| e.to_string())?;

        // Um laço sem fim no Lua para o quadro com erro, em vez de travar.
        let deadline: Rc<Cell<Option<Instant>>> = Rc::new(Cell::new(None));
        let limit = opts.frame_limit;
        let dl = deadline.clone();
        lua.set_global_hook(
            HookTriggers::new().every_nth_instruction(20_000),
            move |_, _| match dl.get() {
                Some(d) if Instant::now() > d => Err(mlua::Error::RuntimeError(format!(
                    "o jogo travou: o Lua rodou mais de {} s sem voltar (um laço sem fim?)",
                    limit.as_secs_f32()
                ))),
                _ => Ok(VmState::Continue),
            },
        )
        .map_err(|e| e.to_string())?;

        let driver = runtime.driver().map_err(|e| format_error(&e))?;

        let mut config = project.config.clone();
        config.hot_reload = false;
        config.save_dir = None;
        config.fullscreen = false;
        config.size = opts.size.or(config.resolution).unwrap_or(config.size);

        deadline.set(Some(Instant::now() + limit));
        let rt = runtime.clone();
        let first = start.clone();
        let headless = Headless::new(config, move |ctx| SceneStack::new(ctx, rt.scene(first)))
            .map_err(|e| {
                format!("não foi possível rodar o jogo sem janela (sem placa de vídeo?): {e}")
            })?;
        deadline.set(None);

        let mut run = Self {
            headless,
            runtime,
            driver,
            sink,
            _guard: guard,
            deadline,
            frame_limit: limit,
            frames: 0,
            report: Report::default(),
            front: String::new(),
            front_ptr: 0,
            fired_index: HashMap::new(),
            ended: false,
        };
        run.after_frame(0.0);
        Ok(run)
    }

    /// O ambiente Lua do jogo.
    pub fn runtime(&self) -> &ScriptRuntime {
        &self.runtime
    }

    /// O contexto da engine (teclado, mouse, câmera...).
    pub fn ctx(&mut self) -> &mut nn_app::Context {
        &mut self.headless.ctx
    }

    /// Quadros rodados.
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// Segundos de jogo.
    pub fn time(&self) -> f64 {
        self.frames as f64 / FPS
    }

    /// O jogo fechou (`nn.quit`)?
    pub fn ended(&self) -> bool {
        self.ended
    }

    /// Alguma cena parou com erro?
    pub fn has_error(&self) -> bool {
        !self.report.errors.is_empty()
    }

    /// O primeiro erro que parou uma cena (desde o último
    /// [`GameRun::take_report`]).
    pub fn first_error(&self) -> Option<&RunError> {
        self.report.errors.first()
    }

    /// A cena da frente (o arquivo).
    pub fn front(&self) -> &str {
        &self.front
    }

    fn arm(&self) {
        self.deadline.set(Some(
            Instant::now() + self.frame_limit.max(Duration::from_millis(10)),
        ));
    }

    fn disarm(&self) {
        self.deadline.set(None);
    }

    /// Roda um quadro.
    pub fn frame(&mut self) {
        if self.ended {
            return;
        }
        // A cena que sai neste quadro ainda pode ter regras que dispararam
        // (a que trocou de cena, por exemplo).
        let before = self.runtime.front();
        let t = self.time();
        self.sink.set_time(t);
        self.arm();
        // Sem desenhar na GPU: só quando alguém pede a tela (bem mais rápido).
        self.headless.frame_undrawn(Headless::<SceneStack>::FRAME);
        self.disarm();
        self.frames += 1;
        if let (path, Some(scene)) = before
            && path != self.runtime.front().0
        {
            self.collect_fired(&path, scene, t);
        }
        self.after_frame(t);
    }

    /// Roda `n` quadros; para antes se o jogo fechar ou der erro. Devolve
    /// quantos rodou.
    pub fn run_frames(&mut self, n: u64) -> u64 {
        let errors = self.report.errors.len();
        let mut ran = 0;
        while ran < n && !self.ended && self.report.errors.len() == errors {
            self.frame();
            ran += 1;
        }
        ran
    }

    /// O que olhar depois de cada quadro: erros, a cena da frente, as
    /// regras que dispararam e se o jogo fechou.
    fn after_frame(&mut self, t: f64) {
        let last = self.headless.ctx.perf.last;
        let p = &mut self.report.perf;
        p.frames += 1;
        p.total.fixed += last.fixed;
        p.total.physics += last.physics;
        p.total.update += last.update;
        p.total.draw += last.draw;
        p.total.steps += last.steps;
        let work = last.fixed + last.update + last.draw;
        if work > p.worst.0 {
            p.worst = (work, t);
        }
        for error in self.runtime.take_errors() {
            self.report.errors.push(RunError { time: t, error });
        }
        let (path, scene) = self.runtime.front();
        let ptr = scene.as_ref().map_or(0, |s| s.to_pointer() as usize);
        if path != self.front {
            self.report.scenes.push(SceneChange {
                time: t,
                scene: path.clone(),
            });
            self.front = path.clone();
        } else if ptr != self.front_ptr && self.front_ptr != 0 && ptr != 0 {
            // A mesma cena, de novo: reiniciou.
            self.report.restarts.push(t);
        }
        if ptr != 0 {
            self.front_ptr = ptr;
        }
        if let Some(scene) = scene {
            self.collect_fired(&path, scene, t);
        }
        self.collect_sounds(t);
        for (model, name) in crate::anim3d::take_plays() {
            match self
                .report
                .animations
                .iter_mut()
                .find(|(m, n, ..)| *m == model && *n == name)
            {
                Some(e) => e.2 += 1,
                None => self.report.animations.push((model, name, 1, t)),
            }
        }
        if self.headless.ctx.is_exiting() || self.headless.game.is_empty() {
            self.ended = true;
            self.report.quit.get_or_insert(t);
        }
    }

    fn collect_sounds(&mut self, t: f64) {
        let lua = self.runtime.lua();
        let Ok(list) = lua.globals().get::<Table>("_nn_tocados") else {
            return;
        };
        if list.raw_len() == 0 {
            return;
        }
        for path in list.sequence_values::<String>().flatten() {
            match self.report.sounds.iter_mut().find(|(p, _, _)| *p == path) {
                Some(e) => e.1 += 1,
                None => self.report.sounds.push((path, 1, t)),
            }
        }
        let _ = lua.globals().set("_nn_tocados", lua.create_table().ok());
    }

    fn collect_fired(&mut self, path: &str, scene: Table, t: f64) {
        let Ok(f) = self.driver.get::<Function>("fired") else {
            return;
        };
        self.arm();
        let list = f.call::<Option<Table>>(scene);
        self.disarm();
        let Ok(Some(list)) = list else {
            return;
        };
        for item in list.sequence_values::<Table>().flatten() {
            // Pelo texto: a mesma regra de várias cópias de um prefab (as
            // moedas, os tiros) é uma linha só.
            let text: String = item.get("text").unwrap_or_default();
            let id = (path.to_string(), text.clone());
            match self.fired_index.get(&id) {
                Some(&i) => {
                    let f = &mut self.report.fired[i];
                    f.count += 1;
                    if f.times.len() < 8 {
                        f.times.push(t);
                    }
                }
                None => {
                    self.fired_index.insert(id, self.report.fired.len());
                    self.report.fired.push(FiredRule {
                        scene: path.to_string(),
                        text,
                        first: t,
                        count: 1,
                        times: vec![t],
                    });
                }
            }
        }
    }

    /// Aperta (`down = true`) ou solta uma tecla, pelo nome do `nn.input`
    /// (`"space"`, `"left"`, `"a"`...).
    pub fn key(&mut self, name: &str, down: bool) -> Result<(), String> {
        let keys = crate::convert::keys(name).map_err(|e| format_error(&e))?;
        set_key(&mut self.headless.ctx.input, keys, down);
        Ok(())
    }

    /// O estado da cena da frente: objetos (do arquivo) e variáveis; numa
    /// cena em código, os campos simples do `self`.
    pub fn snapshot(&mut self) -> Option<serde_json::Value> {
        let (_, scene) = self.runtime.front();
        let scene = scene?;
        let f: Function = self.driver.get("snapshot").ok()?;
        self.arm();
        let value = self
            .runtime
            .with_ctx(&mut self.headless.ctx, |_| f.call::<Value>(scene));
        self.disarm();
        let value = value.ok()?;
        self.runtime.lua().from_value(value).ok()
    }

    /// O valor de uma expressão Lua (com `self` = a cena da frente), como
    /// texto. `Err` traz a mensagem do erro.
    pub fn inspect(&mut self, code: &str) -> Result<String, String> {
        let (_, scene) = self.runtime.front();
        let f: Function = self.driver.get("inspect").map_err(|e| e.to_string())?;
        self.arm();
        let r = self.runtime.with_ctx(&mut self.headless.ctx, |_| {
            f.call::<(bool, String)>((scene, code))
        });
        self.disarm();
        match r {
            Ok((true, v)) => Ok(v),
            Ok((false, e)) => Err(crate::translate_error(&e)),
            Err(e) => Err(format_error(&e)),
        }
    }

    /// A tela do último quadro, em PNG (antes do primeiro quadro, roda um).
    pub fn screenshot(&mut self, label: &str) -> Result<Capture, String> {
        if self.frames == 0 {
            self.frame();
        }
        if !self.headless.drawn() {
            self.arm();
            self.headless.redraw();
            self.disarm();
        }
        let pixels = self
            .headless
            .pixels()
            .map_err(|e| format!("não foi possível ler a tela: {e}"))?;
        let size = self.headless.ctx.window_size();
        let png = encode_png(size.x, size.y, pixels)?;
        Ok(Capture {
            label: label.to_string(),
            time: self.time(),
            width: size.x,
            height: size.y,
            png,
        })
    }

    /// Tira o que aconteceu até agora (as linhas do log, os erros, as
    /// regras...).
    pub fn take_report(&mut self) -> Report {
        let mut report = std::mem::take(&mut self.report);
        report.lines = self.sink.take();
        report.dropped_lines = self.sink.dropped();
        self.fired_index.clear();
        report
    }

    /// Roda o roteiro: as ações nos seus tempos, até `plan.duration`. Para
    /// antes se o jogo der erro ou fechar.
    pub fn run_plan(&mut self, plan: &Plan) -> Result<Outcome, String> {
        // A cena já começou com erro: não há o que rodar.
        if let Some(e) = self.report.errors.first() {
            return Ok(Outcome::GameError(e.clone()));
        }
        let lua = self.runtime.lua().clone();
        let table = plan_table(&lua, plan).map_err(|e| e.to_string())?;
        let f: Function = self.driver.get("plan").map_err(|e| e.to_string())?;
        let limits = Limits {
            game: plan.duration + 5.0,
            ..Limits::default()
        };
        self.run_with_t(f, table, limits)
    }

    /// Roda `f(t, extra)` numa corrotina, com o `t` dos testes, fazendo o
    /// jogo andar quando ela pede.
    pub fn run_with_t(
        &mut self,
        f: Function,
        extra: impl IntoLuaMulti,
        limits: Limits,
    ) -> Result<Outcome, String> {
        let lua = self.runtime.lua().clone();
        let new_t: Function = self.driver.get("new_t").map_err(|e| e.to_string())?;
        let t: Table = new_t.call(self.frames).map_err(|e| e.to_string())?;
        let run: Function = self.driver.get("run").map_err(|e| e.to_string())?;
        // D.run(f, t, extra...) roda f(t, extra...) dentro de um xpcall (em
        // Lua: a corrotina não pode parar dentro de uma função do Rust).
        let mut args = MultiValue::new();
        args.push_back(Value::Function(f));
        args.push_back(Value::Table(t));
        args.extend(extra.into_lua_multi(&lua).map_err(|e| e.to_string())?);
        let thread = lua.create_thread(run).map_err(|e| e.to_string())?;
        self.drive(thread, args, limits)
    }

    /// Faz a corrotina andar até ela terminar.
    fn drive(
        &mut self,
        thread: mlua::Thread,
        first: impl IntoLuaMulti,
        limits: Limits,
    ) -> Result<Outcome, String> {
        let lua = self.runtime.lua().clone();
        let started = Instant::now();
        let last_frame = self.frames + (limits.game * FPS).ceil() as u64;
        let errors_before = self.report.errors.len();
        let mut args = first.into_lua_multi(&lua).map_err(|e| e.to_string())?;
        loop {
            self.sink.set_time(self.time());
            self.arm();
            let r = self.runtime.with_ctx(&mut self.headless.ctx, |_| {
                thread.resume::<MultiValue>(args.clone())
            });
            self.disarm();
            let values = match r {
                Ok(v) => v,
                Err(e) => return Ok(Outcome::Failed(format_error(&e))),
            };
            if thread.status() != mlua::thread::ThreadStatus::Resumable {
                // Terminou: D.run devolve true, ou false e o erro.
                let mut it = values.into_iter();
                return Ok(match it.next() {
                    Some(Value::Boolean(false)) => {
                        let msg = it
                            .next()
                            .and_then(|v| lua.coerce_string(v).ok().flatten())
                            .map(|s| s.to_string_lossy())
                            .unwrap_or_else(|| "erro".into());
                        Outcome::Failed(crate::translate_error(&msg))
                    }
                    _ => Outcome::Done,
                });
            }
            // Um pedido da corrotina: ("frames", n) ou ("capture", nome).
            let mut it = values.into_iter();
            let cmd = it
                .next()
                .and_then(|v| v.as_string().map(|s| s.to_string_lossy()))
                .unwrap_or_default();
            match cmd.as_str() {
                "frames" => {
                    let n = it.next().and_then(|v| v.as_i64()).unwrap_or(1).max(0) as u64;
                    for _ in 0..n {
                        if self.ended {
                            break;
                        }
                        if self.frames >= last_frame {
                            return Ok(Outcome::TimedOut(format!(
                                "passou de {} s de jogo",
                                limits.game
                            )));
                        }
                        if started.elapsed() > limits.wall {
                            return Ok(Outcome::TimedOut(format!(
                                "demorou mais de {} s de relógio",
                                limits.wall.as_secs()
                            )));
                        }
                        self.frame();
                        if self.report.errors.len() > errors_before {
                            let e = self.report.errors[errors_before].clone();
                            return Ok(Outcome::GameError(e));
                        }
                    }
                }
                "capture" => {
                    let label = it
                        .next()
                        .and_then(|v| v.as_string().map(|s| s.to_string_lossy()))
                        .unwrap_or_else(|| "captura".into());
                    let capture = self.screenshot(&label)?;
                    self.report.captures.push(capture);
                }
                other => {
                    return Ok(Outcome::Failed(format!(
                        "coroutine.yield não pode ser usado direto aqui (recebeu \"{other}\"): para esperar quadros, use t:frames(n) num teste ou \"em\" nas ações do rodar_jogo"
                    )));
                }
            }
            args = (self.frames, self.ended)
                .into_lua_multi(&lua)
                .map_err(|e| e.to_string())?;
        }
    }

    /// Carrega um arquivo de testes: os testes que ele declara com
    /// `test(nome, função)`.
    pub fn load_tests(&mut self, path: &str) -> Result<Vec<(String, Function)>, String> {
        let bytes = self
            .runtime
            .source()
            .read(Path::new(path))
            .map_err(|e| format!("não foi possível ler {path}: {e}"))?;
        let lua = self.runtime.lua().clone();
        let collector: Function = self.driver.get("collector").map_err(|e| e.to_string())?;
        let list = lua.create_table().map_err(|e| e.to_string())?;
        let test: Function = collector.call(list.clone()).map_err(|e| e.to_string())?;
        let globals = lua.globals();
        let old: Value = globals.get("test").unwrap_or(Value::Nil);
        globals.set("test", test).map_err(|e| e.to_string())?;
        self.arm();
        let r = self.runtime.with_ctx(&mut self.headless.ctx, |lua| {
            lua.load(&bytes).set_name(format!("@{path}")).exec()
        });
        self.disarm();
        let _ = globals.set("test", old);
        r.map_err(|e| format_error(&e))?;
        let mut out = Vec::new();
        for item in list.sequence_values::<Table>() {
            let item = item.map_err(|e| e.to_string())?;
            out.push((
                item.get::<String>("name").map_err(|e| e.to_string())?,
                item.get::<Function>("fn").map_err(|e| e.to_string())?,
            ));
        }
        Ok(out)
    }
}

/// Confere o nome de uma tecla (os do `nn.input`: `"space"`, `"left"`,
/// `"a"`...). `Err` traz a mensagem com os nomes que valem.
pub fn check_key(name: &str) -> Result<(), String> {
    crate::convert::keys(name)
        .map(|_| ())
        .map_err(|e| format_error(&e))
}

/// Aperta ou solta as teclas de um nome (`"shift"` são as duas).
pub(crate) fn set_key(input: &mut nn_input::Input, keys: &[nn_input::KeyCode], down: bool) {
    if down {
        if let Some(k) = keys.first() {
            input.press_key(*k);
        }
    } else {
        for k in keys {
            input.release_key(*k);
        }
    }
}

/// O roteiro como tabela Lua para o `D.plan`.
fn plan_table(lua: &mlua::Lua, plan: &Plan) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    t.set("duration", plan.duration.max(0.0))?;
    let actions = lua.create_table()?;
    for a in &plan.actions {
        let x = lua.create_table()?;
        x.set("at", a.at.max(0.0))?;
        match &a.kind {
            ActionKind::Press(k) => {
                x.set("op", "press")?;
                x.set("key", k.as_str())?;
            }
            ActionKind::Hold(k, s) => {
                x.set("op", "hold")?;
                x.set("key", k.as_str())?;
                x.set("dur", s.max(0.0))?;
            }
            ActionKind::Down(k) => {
                x.set("op", "down")?;
                x.set("key", k.as_str())?;
            }
            ActionKind::Up(k) => {
                x.set("op", "up")?;
                x.set("key", k.as_str())?;
            }
            ActionKind::Click {
                x: px,
                y: py,
                button,
            } => {
                x.set("op", "click")?;
                x.set("x", *px)?;
                x.set("y", *py)?;
                x.set("button", button.as_str())?;
            }
            ActionKind::ClickObject { name, button } => {
                x.set("op", "click")?;
                x.set("name", name.as_str())?;
                x.set("button", button.as_str())?;
            }
            ActionKind::Move { x: px, y: py } => {
                x.set("op", "move")?;
                x.set("x", *px)?;
                x.set("y", *py)?;
            }
            ActionKind::Type(text) => {
                x.set("op", "type")?;
                x.set("text", text.as_str())?;
            }
            ActionKind::Capture(label) => {
                x.set("op", "capture")?;
                if !label.is_empty() {
                    x.set("label", label.as_str())?;
                }
            }
            ActionKind::Lua(code) => {
                x.set("op", "lua")?;
                x.set("code", code.as_str())?;
            }
        }
        actions.push(x)?;
    }
    t.set("actions", actions)?;
    Ok(t)
}

/// As funções do Rust que o `testes.lua` usa e o módulo dele.
pub(crate) fn install_driver(runtime: &ScriptRuntime) -> mlua::Result<Table> {
    let lua = runtime.lua();
    let driver = lua.create_table()?;
    driver.set(
        "key",
        lua.create_function(|lua, (name, down): (String, bool)| {
            let keys = crate::convert::keys(&name)?;
            crate::api::with_ctx(lua, |c| {
                set_key(&mut c.input, keys, down);
                Ok(())
            })
        })?,
    )?;
    driver.set(
        "mouse_move",
        lua.create_function(|lua, (x, y): (f32, f32)| {
            crate::api::with_ctx(lua, |c| {
                c.input.set_cursor(Some(glam::Vec2::new(x, y)));
                Ok(())
            })
        })?,
    )?;
    driver.set(
        "mouse_button",
        lua.create_function(|lua, (name, down): (String, bool)| {
            let b = crate::convert::mouse_button(&name)?;
            crate::api::with_ctx(lua, |c| {
                if down {
                    c.input.press_mouse(b);
                } else {
                    c.input.release_mouse(b);
                }
                Ok(())
            })
        })?,
    )?;
    driver.set(
        "type",
        lua.create_function(|lua, text: String| {
            crate::api::with_ctx(lua, |c| {
                c.input.type_text(&text);
                Ok(())
            })
        })?,
    )?;
    // Fraco: a tabela vive dentro do Lua, que vive dentro do runtime.
    let weak: Weak<Inner> = Rc::downgrade(&runtime.inner);
    driver.set(
        "front",
        lua.create_function(move |_, ()| {
            let Some(inner) = weak.upgrade() else {
                return Ok((String::new(), None));
            };
            let front = inner.front.borrow().clone();
            Ok(front)
        })?,
    )?;
    // As linhas (arquivo:linha) das funções Lua na pilha, de dentro para fora.
    driver.set(
        "traceback",
        lua.create_function(|lua, ()| {
            let mut out = Vec::new();
            for level in 1..60 {
                let Some(item) = lua.inspect_stack(level, |d| {
                    let src = d.source();
                    let file = src.short_src.map(|s| s.into_owned());
                    (src.what, file, d.current_line())
                }) else {
                    break;
                };
                if let ("Lua" | "main", Some(file), Some(line)) = item {
                    out.push(format!("{file}:{line}"));
                }
            }
            Ok(out)
        })?,
    )?;
    let scene_runtime: Table = lua.named_registry_value(SCENE_RUNTIME_KEY)?;
    lua.load(DRIVER_RUNTIME)
        .set_name("@nn/testes.lua")
        .call((driver, scene_runtime))
}

/// Codifica os pixels (RGBA) em PNG.
pub fn encode_png(width: u32, height: u32, pixels: Vec<u8>) -> Result<Vec<u8>, String> {
    let img = image::RgbaImage::from_raw(width, height, pixels)
        .ok_or("a imagem da tela veio com o tamanho errado")?;
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| format!("não foi possível gerar o PNG: {e}"))?;
    Ok(out.into_inner())
}
