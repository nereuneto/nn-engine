//! Captura do log enquanto um jogo roda sem janela ([`crate::game_run`]):
//! os `print` do Lua, os avisos e os erros da engine, com o tempo do jogo em
//! que aconteceram.
//!
//! O log do Rust é um só para o programa inteiro, então quem quer a captura
//! instala o [`Logger`] daqui no lugar do seu (ele repassa tudo para o
//! logger de antes):
//!
//! ```ignore
//! let env = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).build();
//! let level = env.filter();
//! nn_script::capture::install(Some(Box::new(env)), level).unwrap();
//! ```
//!
//! Sem isso, o [`crate::game_run::GameRun`] instala um [`Logger`] sozinho, se
//! ainda não houver nenhum. A captura vale para a thread que roda o jogo, então
//! testes em paralelo não misturam as mensagens.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use log::{LevelFilter, Log, Metadata, Record};

/// O tipo de uma linha capturada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    /// `print` do Lua.
    Print,
    /// Aviso da engine (um arquivo que falta...): o jogo continua.
    Warning,
    /// Erro da engine.
    Error,
}

/// Uma linha capturada.
#[derive(Debug, Clone, PartialEq)]
pub struct LogLine {
    /// Segundos de jogo.
    pub time: f64,
    pub kind: LineKind,
    pub text: String,
}

/// Quantas linhas uma captura guarda (as primeiras).
const MAX_LINES: usize = 2000;

/// Onde as linhas de uma captura vão parar.
#[derive(Debug, Default)]
pub struct Sink {
    time: Cell<f64>,
    lines: RefCell<Vec<LogLine>>,
    dropped: Cell<usize>,
}

impl Sink {
    /// O tempo de jogo das próximas linhas.
    pub fn set_time(&self, time: f64) {
        self.time.set(time);
    }

    /// Tira as linhas capturadas até agora.
    pub fn take(&self) -> Vec<LogLine> {
        std::mem::take(&mut *self.lines.borrow_mut())
    }

    /// Quantas linhas ficaram de fora por passar do limite.
    pub fn dropped(&self) -> usize {
        self.dropped.get()
    }

    fn push(&self, kind: LineKind, text: String) {
        let Ok(mut lines) = self.lines.try_borrow_mut() else {
            return;
        };
        if lines.len() >= MAX_LINES {
            self.dropped.set(self.dropped.get() + 1);
            return;
        }
        lines.push(LogLine {
            time: self.time.get(),
            kind,
            text,
        });
    }
}

thread_local! {
    static SINKS: RefCell<Vec<Rc<Sink>>> = const { RefCell::new(Vec::new()) };
}

/// Começa a capturar nesta thread. As linhas vão para a captura mais nova
/// até o [`Guard`] sair de cena.
pub fn start() -> (Rc<Sink>, Guard) {
    let sink = Rc::new(Sink::default());
    SINKS.with(|s| s.borrow_mut().push(sink.clone()));
    (sink.clone(), Guard(sink))
}

/// Termina a captura ao sair de cena.
#[derive(Debug)]
pub struct Guard(Rc<Sink>);

impl Drop for Guard {
    fn drop(&mut self) {
        let _ = SINKS.try_with(|s| {
            if let Ok(mut s) = s.try_borrow_mut() {
                s.retain(|x| !Rc::ptr_eq(x, &self.0));
            }
        });
    }
}

fn capturing() -> bool {
    SINKS
        .try_with(|s| s.try_borrow().map(|s| !s.is_empty()).unwrap_or(false))
        .unwrap_or(false)
}

/// O que uma mensagem do log vira na captura (ou nada).
fn classify(record: &Record) -> Option<(LineKind, String)> {
    let target = record.target();
    // Os erros que param uma cena chegam pelo `ScriptRuntime`.
    if target == crate::SCENE_ERROR_TARGET || !target.starts_with("nn_") {
        return None;
    }
    let text = record.args().to_string();
    if let Some(print) = text.strip_prefix("[lua] ") {
        return Some((LineKind::Print, print.to_string()));
    }
    match record.level() {
        log::Level::Error => Some((LineKind::Error, text)),
        log::Level::Warn => Some((LineKind::Warning, text)),
        _ => None,
    }
}

/// O logger que captura: repassa tudo ao logger de antes (se houver) e
/// guarda o que interessa na captura da thread.
pub struct Logger {
    inner: Option<Box<dyn Log>>,
}

impl std::fmt::Debug for Logger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Logger")
            .field("inner", &self.inner.is_some())
            .finish()
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        (capturing() && metadata.level() <= log::Level::Info)
            || self.inner.as_ref().is_some_and(|l| l.enabled(metadata))
    }

    fn log(&self, record: &Record) {
        if let Some(inner) = &self.inner
            && inner.enabled(record.metadata())
        {
            inner.log(record);
        }
        if !capturing() {
            return;
        }
        if let Some((kind, text)) = classify(record) {
            let _ = SINKS.try_with(|s| {
                if let Ok(s) = s.try_borrow()
                    && let Some(sink) = s.last()
                {
                    sink.push(kind, text);
                }
            });
        }
    }

    fn flush(&self) {
        if let Some(inner) = &self.inner {
            inner.flush();
        }
    }
}

/// Instala o logger da captura, repassando para `inner` (que mostra as
/// mensagens até o nível `level`). Falha se já houver um logger.
pub fn install(inner: Option<Box<dyn Log>>, level: LevelFilter) -> Result<(), log::SetLoggerError> {
    TRIED.store(true, Ordering::Relaxed);
    // Fica para sempre: o log é do programa inteiro (e só é instalado uma vez).
    let logger: &'static Logger = Box::leak(Box::new(Logger { inner }));
    log::set_logger(logger)?;
    // Os prints do Lua são `info`: precisam passar mesmo com o log mais quieto.
    log::set_max_level(level.max(LevelFilter::Info));
    Ok(())
}

static TRIED: AtomicBool = AtomicBool::new(false);

/// Instala o logger da captura sem repassar para ninguém, se ainda não
/// houver um logger.
pub fn ensure_installed() {
    if !TRIED.load(Ordering::Relaxed) {
        let _ = install(None, LevelFilter::Off);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captura_so_na_thread_e_so_o_que_interessa() {
        ensure_installed();
        let (sink, guard) = start();
        sink.set_time(1.5);
        log::info!(target: "nn_script::api", "[lua] olá\tmundo");
        log::warn!(target: "nn_app::assets", "imagem não encontrada: x.png");
        log::error!(target: "nn_app", "falhou");
        log::warn!(target: "wgpu_core", "detalhe da GPU");
        log::info!(target: "nn_script", "recarregando");
        log::error!(target: crate::SCENE_ERROR_TARGET, "erro em a.lua");
        std::thread::spawn(|| log::info!(target: "nn_script::api", "[lua] outra thread"))
            .join()
            .unwrap();
        let lines = sink.take();
        let got: Vec<_> = lines.iter().map(|l| (l.kind, l.text.as_str())).collect();
        assert_eq!(
            got,
            [
                (LineKind::Print, "olá\tmundo"),
                (LineKind::Warning, "imagem não encontrada: x.png"),
                (LineKind::Error, "falhou"),
            ]
        );
        assert!(lines.iter().all(|l| l.time == 1.5));
        drop(guard);
        log::info!(target: "nn_script::api", "[lua] depois");
        assert!(sink.take().is_empty());
    }
}
