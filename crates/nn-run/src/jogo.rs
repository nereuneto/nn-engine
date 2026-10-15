//! `nn-jogo`: executável dos jogos exportados.
//!
//! Lê o pacote anexado ao fim do próprio arquivo e roda o jogo. Sem pacote,
//! aceita uma pasta de projeto como argumento (útil para testar).

// No Windows, abrir o jogo não mostra uma janela de console.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use nn_app::Pack;
use nn_script::Project;

fn main() -> ExitCode {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("nn-jogo"));
    let name = exe
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "jogo".into());

    // Sem console no Windows: o log vai para um arquivo na pasta temporária.
    let log_path = std::env::temp_dir().join(format!("{name}.log"));
    let mut logger =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
    if let Ok(file) = std::fs::File::create(&log_path) {
        logger.target(env_logger::Target::Pipe(Box::new(file)));
    }
    logger.init();

    let result: Result<(), Box<dyn std::error::Error>> = match Pack::read_appended(&exe) {
        Ok(Some(pack)) => {
            log::info!("{} arquivos no pacote", pack.len());
            Project::from_source(Arc::new(pack), &name)
                .map_err(Into::into)
                .and_then(nn_script::run)
        }
        Ok(None) => match std::env::args().nth(1) {
            Some(dir) => nn_script::run_project(dir),
            None => Err(
                "este executável não contém um jogo; use `nn exportar <pasta>` para gerar um"
                    .into(),
            ),
        },
        Err(e) => Err(format!("pacote do jogo corrompido: {e}").into()),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log::error!("{e}");
            eprintln!("erro: {e}");
            // Deixa a mensagem num arquivo fácil de achar, ao lado do executável.
            let report = exe.with_file_name(format!("{name}-erro.txt"));
            let _ = std::fs::write(
                &report,
                format!(
                    "O jogo não pôde ser iniciado:\n\n{e}\n\nLog completo: {}\n",
                    log_path.display()
                ),
            );
            ExitCode::FAILURE
        }
    }
}
