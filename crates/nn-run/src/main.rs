//! `nn`: roda um jogo em Lua.
//!
//! ```text
//! nn [pasta-do-projeto]
//! ```
//!
//! A pasta precisa ter um `main.lua` (ou a cena indicada em `config.lua`).
//! Sem argumento, usa a pasta atual.

use std::process::ExitCode;

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let mut args = std::env::args().skip(1);
    let dir = match args.next().as_deref() {
        Some("-h" | "--help") => {
            println!(
                "uso: nn [pasta-do-projeto]\n\nRoda o jogo em Lua da pasta indicada (padrão: pasta atual)."
            );
            return ExitCode::SUCCESS;
        }
        Some(dir) => dir.to_string(),
        None => ".".to_string(),
    };

    match nn_script::run_project(&dir) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log::error!("{e}");
            eprintln!("erro: {e}");
            ExitCode::FAILURE
        }
    }
}
