//! `nn`: cria, roda e exporta jogos em Lua.
//!
//! ```text
//! nn novo <pasta> [opções]       cria um jogo novo a partir de um modelo
//! nn [pasta]                     roda o jogo da pasta (padrão: pasta atual)
//! nn exportar <pasta> [opções]   gera um executável único para distribuir
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use nn_run::{ExportOptions, NewOptions, Template, create_project, export, human_size};

const HELP: &str = "\
uso:
  nn novo <pasta> [opções]       cria um jogo novo, pronto para rodar
  nn [pasta]                     roda o jogo em Lua da pasta (padrão: pasta atual)
  nn exportar <pasta> [opções]   gera um executável único com o jogo

opções de novo:
  --titulo <título>    título do jogo (padrão: nome da pasta)
  --modelo <nome>      padrao: título, partida, pausa e recorde (padrão)
                       minimo: só uma cena com um quadrado que anda

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
        Some("novo") => novo(&args[1..]),
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

fn novo(args: &[String]) -> ExitCode {
    let mut opts = NewOptions::default();
    let mut dir = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |flag: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{flag} precisa de um valor"))
        };
        let result = match arg.as_str() {
            "--titulo" => value("--titulo").map(|v| opts.title = Some(v)),
            "--modelo" => value("--modelo").and_then(|v| {
                Template::from_name(&v)
                    .map(|t| opts.template = t)
                    .ok_or_else(|| format!("modelo desconhecido: {v} (use padrao ou minimo)"))
            }),
            other if other.starts_with("--") => Err(format!("opção desconhecida: {other}")),
            other => {
                dir = Some(PathBuf::from(other));
                Ok(())
            }
        };
        if let Err(e) = result {
            eprintln!("erro: {e}\n\n{HELP}");
            return ExitCode::FAILURE;
        }
    }
    let Some(dir) = dir else {
        eprintln!("erro: informe a pasta do novo jogo (ex.: nn novo meu-jogo)\n\n{HELP}");
        return ExitCode::FAILURE;
    };
    opts.dir = dir;

    match create_project(&opts) {
        Ok(r) => {
            // Rodando por `cargo nn`, o comando para jogar também é por ele.
            let nn = if std::env::var_os("CARGO").is_some() {
                "cargo nn"
            } else {
                "nn"
            };
            println!(
                "jogo \"{}\" criado em {} ({} arquivos, modelo {})\n\n\
                 para jogar:      {nn} {}\n\
                 para exportar:   {nn} exportar {}\n\n\
                 Com o jogo aberto, edite e salve os arquivos .lua: a mudança aparece na hora.",
                r.title,
                r.dir.display(),
                r.files.len(),
                opts.template.name(),
                r.dir.display(),
                r.dir.display(),
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
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
