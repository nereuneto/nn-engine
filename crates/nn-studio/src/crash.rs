//! Log em arquivo e aviso de travamento.
//!
//! O NN Studio grava o log em `nn-studio.log`, na pasta das preferências (o
//! log da vez anterior fica em `nn-studio.log.1`). Se o editor fechar por um
//! erro (um *panic*), a mensagem vai para `nn-studio-erro.txt`; na próxima
//! vez que abrir, ele avisa e mostra onde está o log.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Nome do arquivo de log.
pub const LOG_FILE: &str = "nn-studio.log";
/// Nome do arquivo com o erro que fechou o editor.
pub const CRASH_FILE: &str = "nn-studio-erro.txt";

/// Escreve no terminal e no arquivo de log.
struct Tee {
    file: Option<Mutex<std::fs::File>>,
}

impl Write for Tee {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _ = std::io::stderr().write_all(buf);
        if let Some(f) = &self.file
            && let Ok(mut f) = f.lock()
        {
            let _ = f.write_all(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(f) = &self.file
            && let Ok(mut f) = f.lock()
        {
            let _ = f.flush();
        }
        Ok(())
    }
}

/// Abre o log da pasta (guardando o anterior como `.1`).
fn open_log(dir: &Path) -> Option<std::fs::File> {
    std::fs::create_dir_all(dir).ok()?;
    let path = dir.join(LOG_FILE);
    if path.exists() {
        let _ = std::fs::rename(&path, dir.join(format!("{LOG_FILE}.1")));
    }
    std::fs::File::create(path).ok()
}

/// Liga o log (terminal e arquivo na pasta `dir`) e o aviso de travamento.
pub fn init(dir: Option<&Path>) {
    let file = dir.and_then(open_log).map(Mutex::new);
    let logger = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn,nn_studio=info"),
    )
    .format(|buf, record| {
        let secs = crate::settings::now_secs();
        writeln!(
            buf,
            "[{secs} {} {}] {}",
            record.level(),
            record.target(),
            record.args()
        )
    })
    .target(env_logger::Target::Pipe(Box::new(Tee { file })))
    .build();
    // A captura repassa ao log do editor e guarda os prints dos testes de
    // jogo (Jogo > Conferir com os testes).
    let level = logger.filter();
    let _ = nn_script::capture::install(Some(Box::new(logger)), level);
    log::info!(
        "NN Studio {} ({} {})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    if let Some(dir) = dir {
        install_hook(dir.to_path_buf());
    }
}

/// O texto gravado quando o editor fecha por erro.
pub fn crash_text(info: &std::panic::PanicHookInfo<'_>, backtrace: &str) -> String {
    let msg = info
        .payload()
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| info.payload().downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "erro desconhecido".into());
    let place = info
        .location()
        .map(|l| format!("{}:{}", l.file(), l.line()))
        .unwrap_or_default();
    format!(
        "NN Studio {} ({} {})\n{msg}\nem {place}\n\n{backtrace}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

fn install_hook(dir: PathBuf) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let bt = std::backtrace::Backtrace::force_capture().to_string();
        let text = crash_text(info, &bt);
        log::error!("o NN Studio fechou por um erro:\n{text}");
        let _ = std::fs::write(dir.join(CRASH_FILE), &text);
        previous(info);
    }));
}

/// O erro que fechou o editor da última vez (e apaga o aviso, para não
/// repetir).
pub fn take_last_crash(dir: &Path) -> Option<String> {
    let path = dir.join(CRASH_FILE);
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::rename(&path, dir.join(format!("{CRASH_FILE}.visto")));
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aviso_de_travamento_aparece_uma_vez() {
        let dir = std::env::temp_dir().join(format!("nn-crash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(take_last_crash(&dir), None);
        std::fs::write(dir.join(CRASH_FILE), "deu ruim").unwrap();
        assert_eq!(take_last_crash(&dir).as_deref(), Some("deu ruim"));
        assert_eq!(take_last_crash(&dir), None);
        // O log da vez anterior fica guardado.
        drop(open_log(&dir));
        std::fs::write(dir.join(LOG_FILE), "antes").unwrap();
        drop(open_log(&dir));
        assert_eq!(
            std::fs::read_to_string(dir.join(format!("{LOG_FILE}.1"))).unwrap(),
            "antes"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
