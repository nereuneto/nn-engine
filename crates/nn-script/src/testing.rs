//! Testes de jogo: arquivos Lua na pasta `testes/` do projeto que jogam o
//! jogo sem janela e conferem o resultado.
//!
//! ```lua
//! -- testes/heroi.lua
//! test("o herói anda para a direita", function(t)
//!   t:open("cenas/fase.cena")
//!   local heroi = t:find("heroi")
//!   local x0 = heroi.x
//!   t:hold("right", 1)
//!   t:check(heroi.x > x0 + 100, "o herói devia ter andado")
//! end)
//! ```
//!
//! Cada teste roda num jogo novo, que começa na cena inicial do
//! `config.lua`. O `t` tem o que o teste faz com o jogo (veja `docs/lua.md`,
//! "Testes de jogo"). `nn testar` roda todos; o servidor MCP tem a
//! ferramenta `rodar_testes`.

use std::path::Path;
use std::time::{Duration, Instant};

use crate::game_run::{Capture, GameRun, Limits, Outcome, Report, RunOptions};
use crate::{Project, ScriptRuntime};

/// A pasta dos testes.
pub const TEST_DIR: &str = "testes";

/// Um teste de um arquivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCase {
    pub file: String,
    pub name: String,
    /// A posição no arquivo (0 = o primeiro).
    pub index: usize,
}

/// Um arquivo de testes que não carregou.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileError {
    pub file: String,
    pub message: String,
}

/// O resultado de um teste.
#[derive(Debug, Clone)]
pub struct TestResult {
    pub case: TestCase,
    /// `None`: passou. `Some`: o motivo da falha.
    pub failure: Option<String>,
    /// Segundos de jogo que o teste rodou.
    pub time: f64,
    /// Tempo de relógio.
    pub wall: Duration,
    /// Os prints, avisos, erros, regras e capturas (`t:screenshot`).
    pub report: Report,
    /// A tela quando o teste falhou.
    pub last_frame: Option<Capture>,
}

impl TestResult {
    pub fn passed(&self) -> bool {
        self.failure.is_none()
    }
}

/// Como rodar os testes.
#[derive(Debug, Clone, Default)]
pub struct TestOptions {
    /// Só os testes cujo nome (ou arquivo) tem este texto.
    pub filter: Option<String>,
    /// Só os testes deste arquivo.
    pub file: Option<String>,
    /// Como rodar o jogo (a cena é sempre a inicial).
    pub run: RunOptions,
    /// Limite de cada teste (padrão: 10 minutos de jogo, 2 minutos de relógio).
    pub limits: Option<Limits>,
    /// Guardar a tela de quem falhar.
    pub capture_failures: bool,
}

/// Os arquivos de testes do projeto (`testes/**/*.lua`), em ordem.
pub fn test_files(source: &dyn nn_app::AssetSource) -> Vec<String> {
    let prefix = format!("{TEST_DIR}/");
    let mut files: Vec<String> = source
        .files("lua")
        .into_iter()
        .map(|(p, _)| p)
        .filter(|p| p.starts_with(&prefix))
        .collect();
    files.sort();
    files
}

/// Os testes do projeto (sem rodar nada) e os arquivos que não carregaram.
pub fn discover(
    project: &Project,
    opts: &TestOptions,
) -> Result<(Vec<TestCase>, Vec<FileError>), String> {
    let source = project.source();
    let mut files = test_files(source.as_ref());
    if let Some(only) = &opts.file {
        let only = only.trim_start_matches("./").replace('\\', "/");
        if source.read(Path::new(&only)).is_err() {
            return Err(format!("{only} não existe"));
        }
        files = vec![only];
    }
    let runtime = ScriptRuntime::with_source(source.clone()).map_err(|e| e.to_string())?;
    let lua = runtime.lua();
    let filter = opts.filter.as_ref().map(|f| f.to_lowercase());
    let mut cases = Vec::new();
    let mut errors = Vec::new();
    for file in files {
        match names_in(lua, source.as_ref(), &file) {
            Ok(names) => {
                for (index, name) in names.into_iter().enumerate() {
                    let wanted = filter.as_ref().is_none_or(|f| {
                        name.to_lowercase().contains(f) || file.to_lowercase().contains(f)
                    });
                    if wanted {
                        cases.push(TestCase {
                            file: file.clone(),
                            name,
                            index,
                        });
                    }
                }
            }
            Err(message) => errors.push(FileError { file, message }),
        }
    }
    Ok((cases, errors))
}

/// Os nomes dos testes de um arquivo (carrega o arquivo sem o jogo).
fn names_in(
    lua: &mlua::Lua,
    source: &dyn nn_app::AssetSource,
    file: &str,
) -> Result<Vec<String>, String> {
    let bytes = source
        .read(Path::new(file))
        .map_err(|e| format!("não foi possível ler: {e}"))?;
    let list = lua.create_table().map_err(|e| e.to_string())?;
    let l = list.clone();
    let test = lua
        .create_function(
            move |_, (name, f): (mlua::Value, mlua::Value)| match (name, f) {
                (mlua::Value::String(name), mlua::Value::Function(_)) => {
                    l.push(name.to_string_lossy())?;
                    Ok(())
                }
                _ => Err(mlua::Error::RuntimeError(
                    "use test(\"nome do teste\", function(t) ... end)".into(),
                )),
            },
        )
        .map_err(|e| e.to_string())?;
    let globals = lua.globals();
    globals.set("test", test).map_err(|e| e.to_string())?;
    let r = lua.load(&bytes).set_name(format!("@{file}")).exec();
    let _ = globals.set("test", mlua::Value::Nil);
    r.map_err(|e| crate::format_error(&e))?;
    let names: Vec<String> = list.sequence_values::<String>().flatten().collect();
    Ok(names)
}

/// Roda um teste num jogo novo.
pub fn run_test(
    project: &Project,
    case: &TestCase,
    opts: &TestOptions,
) -> Result<TestResult, String> {
    let started = Instant::now();
    let mut run_opts = opts.run.clone();
    run_opts.scene = None;
    // Num teste, o erro de um comportamento é uma falha (o jogo seguiria).
    run_opts.stop_on_behavior_errors = true;
    let mut run = GameRun::new(project.clone(), run_opts)?;
    let limits = opts.limits.unwrap_or_default();
    let failure = match prepare(&mut run, case) {
        Err(msg) => Some(msg),
        Ok(f) => match run.run_with_t(f, (), limits)? {
            Outcome::Done => None,
            Outcome::Failed(msg) => Some(msg),
            Outcome::GameError(e) => Some(format!(
                "o jogo deu erro em {} s ({}):\n{}",
                seconds(e.time),
                e.error.scene,
                e.error.message
            )),
            Outcome::TimedOut(msg) => Some(format!("o teste {msg}")),
        },
    };
    let last_frame = match &failure {
        Some(_) if opts.capture_failures => run.screenshot("quando falhou").ok(),
        _ => None,
    };
    let time = run.time();
    Ok(TestResult {
        case: case.clone(),
        failure,
        time,
        wall: started.elapsed(),
        report: run.take_report(),
        last_frame,
    })
}

/// Carrega o arquivo no jogo e acha a função do teste.
fn prepare(run: &mut GameRun, case: &TestCase) -> Result<mlua::Function, String> {
    // O teste começa com a cena inicial já rodando: as regras "Quando
    // começar" e o `enter` já rodaram, como para quem joga.
    run.frame();
    if let Some(e) = run.first_error() {
        return Err(format!(
            "o jogo deu erro ao começar ({}):\n{}",
            e.error.scene, e.error.message
        ));
    }
    let tests = run
        .load_tests(&case.file)
        .map_err(|e| format!("{} não carregou: {e}", case.file))?;
    match tests.into_iter().nth(case.index) {
        Some((name, f)) if name == case.name => Ok(f),
        _ => Err(format!(
            "o teste \"{}\" sumiu de {} (o arquivo mudou?)",
            case.name, case.file
        )),
    }
}

/// Roda os testes, um jogo novo para cada, chamando `on_result` a cada um.
pub fn run_tests(
    project: &Project,
    opts: &TestOptions,
    mut on_result: impl FnMut(&TestResult),
) -> Result<Summary, String> {
    let (cases, file_errors) = discover(project, opts)?;
    let started = Instant::now();
    let mut results = Vec::new();
    for case in &cases {
        let r = run_test(project, case, opts)?;
        on_result(&r);
        results.push(r);
    }
    Ok(Summary {
        results,
        file_errors,
        wall: started.elapsed(),
    })
}

/// O resultado de [`run_tests`].
#[derive(Debug, Clone)]
pub struct Summary {
    pub results: Vec<TestResult>,
    pub file_errors: Vec<FileError>,
    pub wall: Duration,
}

impl Summary {
    pub fn passed(&self) -> usize {
        self.results.iter().filter(|r| r.passed()).count()
    }

    pub fn failed(&self) -> usize {
        self.results.len() - self.passed()
    }

    /// Tudo passou (e todos os arquivos carregaram).
    pub fn ok(&self) -> bool {
        self.failed() == 0 && self.file_errors.is_empty()
    }
}

/// Segundos com duas casas ("1.25").
fn seconds(t: f64) -> String {
    format!("{t:.2}")
}
