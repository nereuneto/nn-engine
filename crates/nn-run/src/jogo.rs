//! `nn-jogo`: executável dos jogos exportados.
//!
//! Lê o pacote anexado ao fim do próprio arquivo e roda o jogo. Sem pacote,
//! aceita uma pasta de projeto como argumento (útil para testar).
//!
//! Quando o jogo exportado dá erro, quem joga não vê a mensagem do script:
//! o erro vai para `<nome>-erro.txt` ao lado do executável (ou na pasta
//! temporária, sem permissão ali), e a tela diz onde está o arquivo para
//! mandar a quem fez o jogo. Erro de script e laço sem fim: a tela aparece no
//! próprio jogo. Erro interno (pânico) ou o jogo que não abriu: o jogo fecha e
//! a tela abre num processo novo (`nn-jogo --mostrar-erro <arquivo> <título>`).
//!
//! Para testar sem tela (no CI): `NN_JOGO_SEM_JANELA=<quadros>` roda esse
//! tanto de quadros sem janela e grava a última tela em `NN_JOGO_TELA`
//! (também a tela de erro); `NN_JOGO_TESTE_PANICO=1` entra em pânico ao
//! começar, para testar o arquivo e a tela desse caso.

// No Windows, abrir o jogo não mostra uma janela de console.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, OnceLock};

use nn_app::Pack;
use nn_script::Project;
use nn_script::report::{ErrorReport, run_error_screen, run_error_screen_headless};

/// O arquivo de erro do jogo (o do pacote, com o título; antes de ler o
/// pacote, com o nome do executável).
static REPORT: OnceLock<Arc<ErrorReport>> = OnceLock::new();

struct Paths {
    exe: PathBuf,
    name: String,
    log: PathBuf,
}

impl Paths {
    fn report(&self, title: &str) -> ErrorReport {
        let mut r = ErrorReport::new(
            self.exe.with_file_name(format!("{}-erro.txt", self.name)),
            Some(self.log.clone()),
            title,
        );
        r.show = Some(show_screen);
        r
    }
}

/// Testes sem tela: quantos quadros rodar, e onde gravar a última tela.
fn headless() -> Option<(u32, Option<PathBuf>)> {
    let frames = std::env::var("NN_JOGO_SEM_JANELA").ok()?.parse().ok()?;
    let shot = std::env::var_os("NN_JOGO_TELA").map(PathBuf::from);
    Some((frames, shot))
}

fn main() -> ExitCode {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("nn-jogo"));
    let name = exe
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "jogo".into());
    let args: Vec<String> = std::env::args().collect();

    // A tela de erro sozinha, aberta pelo jogo que fechou.
    if args.get(1).map(String::as_str) == Some("--mostrar-erro") {
        let file = args.get(2).filter(|f| !f.is_empty()).map(PathBuf::from);
        let title = args.get(3).cloned().unwrap_or(name);
        let shown = match headless() {
            Some((_, Some(shot))) => run_error_screen_headless(&title, file, &shot),
            Some((_, None)) => Ok(()),
            None => run_error_screen(&title, file).map_err(Into::into),
        };
        return match shown {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        };
    }

    // Sem console no Windows: o log vai para um arquivo na pasta temporária.
    let log = std::env::temp_dir().join(format!("{name}.log"));
    let mut logger =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    if let Ok(file) = std::fs::File::create(&log) {
        logger.target(env_logger::Target::Pipe(Box::new(file)));
    }
    logger.init();
    let paths = Arc::new(Paths { exe, name, log });

    install_panic_hook(paths.clone());

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| play(&paths, &args)));
    match result {
        Ok(Ok(())) => ExitCode::SUCCESS,
        Ok(Err(e)) => {
            log::error!("{e}");
            eprintln!("erro: {e}");
            let report = REPORT.get_or_init(|| Arc::new(paths.report(&paths.name)));
            report.write(&format!("O jogo não pôde ser iniciado:\n\n{e}"));
            show_screen(report);
            ExitCode::FAILURE
        }
        // O pânico já foi escrito no arquivo (pelo gancho).
        Err(_) => {
            if let Some(report) = REPORT.get() {
                show_screen(report);
            }
            ExitCode::FAILURE
        }
    }
}

fn play(paths: &Paths, args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut project = match Pack::read_appended(&paths.exe) {
        Ok(Some(pack)) => {
            log::info!("{} arquivos no pacote", pack.len());
            Project::from_source(Arc::new(pack), &paths.name)?
        }
        Ok(None) => {
            return match args.get(1) {
                // Uma pasta: como no desenvolvimento (a tela de erro inteira).
                Some(dir) => nn_script::run_project(dir),
                None => Err(
                    "este executável não contém um jogo; use `nn exportar <pasta>` para gerar um"
                        .into(),
                ),
            };
        }
        Err(e) => return Err(format!("pacote do jogo corrompido: {e}").into()),
    };
    let report = REPORT
        .get_or_init(|| Arc::new(paths.report(&project.config.title)))
        .clone();
    project.report = Some(report);
    if std::env::var_os("NN_JOGO_TESTE_PANICO").is_some() {
        panic!("pânico de teste (NN_JOGO_TESTE_PANICO)");
    }
    match headless() {
        Some((frames, shot)) => nn_script::run_headless(project, frames, shot.as_deref()),
        None => nn_script::run(project),
    }
}

/// Um pânico (um defeito da engine) vai para o arquivo de erro, com onde
/// aconteceu.
fn install_panic_hook(paths: Arc<Paths>) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let where_ = thread.name().unwrap_or("sem nome");
        let backtrace = std::backtrace::Backtrace::force_capture();
        let text = format!(
            "O jogo teve um erro interno (um defeito da nn-engine, não do jogo), na \
             linha de execução \"{where_}\":\n\n{info}\n\nRastro:\n{backtrace}"
        );
        log::error!("{text}");
        let report = REPORT.get_or_init(|| Arc::new(paths.report(&paths.name)));
        report.write(&text);
        previous(info);
    }));
}

/// Abre a tela de erro num processo novo (este vai fechar).
fn show_screen(report: &ErrorReport) {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let file = report
        .path()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let spawned = std::process::Command::new(&exe)
        .arg("--mostrar-erro")
        .arg(&file)
        .arg(report.title())
        .spawn();
    if let Err(e) = spawned {
        log::error!("a tela de erro não abriu: {e}");
    }
}
