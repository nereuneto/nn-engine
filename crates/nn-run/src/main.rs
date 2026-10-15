//! `nn`: roda e exporta jogos em Lua.
//!
//! ```text
//! nn [pasta]                     roda o jogo da pasta (padrão: pasta atual)
//! nn exportar <pasta> [opções]   gera um executável único para distribuir
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use nn_run::{ExportOptions, export, human_size};

const HELP: &str = "\
uso:
  nn [pasta]                     roda o jogo em Lua da pasta (padrão: pasta atual)
  nn exportar <pasta> [opções]   gera um executável único com o jogo

opções de exportar:
  --saida <pasta>      onde criar o executável (padrão: dist)
  --nome <nome>        nome do executável (padrão: nome da pasta do projeto)
  --runtime <arquivo>  nn-jogo a usar; para exportar para outro sistema,
                       passe o nn-jogo daquele sistema (ex.: nn-jogo.exe)";

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("-h" | "--help" | "ajuda") => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        Some("exportar") => exportar(&args[1..]),
        Some(dir) => rodar(dir),
        None => rodar("."),
    }
}

fn rodar(dir: &str) -> ExitCode {
    match nn_script::run_project(dir) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log::error!("{e}");
            eprintln!("erro: {e}");
            ExitCode::FAILURE
        }
    }
}

fn exportar(args: &[String]) -> ExitCode {
    let mut project = None;
    let mut opts = ExportOptions {
        project: PathBuf::new(),
        output_dir: PathBuf::from("dist"),
        runtime: None,
        name: None,
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |flag: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{flag} precisa de um valor"))
        };
        let result = match arg.as_str() {
            "--saida" => value("--saida").map(|v| opts.output_dir = v.into()),
            "--nome" => value("--nome").map(|v| opts.name = Some(v)),
            "--runtime" => value("--runtime").map(|v| opts.runtime = Some(v.into())),
            other if other.starts_with("--") => Err(format!("opção desconhecida: {other}")),
            other => {
                project = Some(PathBuf::from(other));
                Ok(())
            }
        };
        if let Err(e) = result {
            eprintln!("erro: {e}\n\n{HELP}");
            return ExitCode::FAILURE;
        }
    }
    let Some(project) = project else {
        eprintln!("erro: informe a pasta do projeto\n\n{HELP}");
        return ExitCode::FAILURE;
    };
    opts.project = project;

    match export(&opts) {
        Ok(r) => {
            println!(
                "jogo exportado: {}\n  {} arquivos ({}), executável com {}\n  runtime: {}",
                r.output.display(),
                r.files,
                human_size(r.assets_bytes),
                human_size(r.total_bytes),
                r.runtime.display()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("erro: {e}");
            ExitCode::FAILURE
        }
    }
}
