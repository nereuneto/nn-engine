//! `nn`: cria, roda e exporta jogos em Lua.
//!
//! ```text
//! nn novo <pasta> [opções]       cria um jogo novo a partir de um modelo
//! nn [pasta]                     roda o jogo da pasta (padrão: pasta atual)
//! nn exportar <pasta> [opções]   gera um executável único para distribuir
//! nn mcp [pasta]                 servidor MCP para agentes de IA
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use nn_run::{ExportOptions, NewOptions, Template, create_project, export, human_size};

const HELP: &str = "\
uso:
  nn novo <pasta> [opções]       cria um jogo novo, pronto para rodar
  nn [pasta]                     roda o jogo em Lua da pasta (padrão: pasta atual)
  nn exportar <pasta> [opções]   gera um executável único com o jogo
  nn mcp [pasta]                 servidor MCP: liga um agente de IA (Claude Code,
                                 Claude Desktop, Cursor...) ao jogo da pasta
  nn --versao                    mostra a versão

opções de novo:
  --titulo <título>    título do jogo (padrão: nome da pasta)
  --modelo <nome>      padrao: título, partida, pausa e recorde (padrão)
                       visual: fase de plataforma montada no NN Studio
                       aventura: aventura vista de cima (NN Studio)
                       nave: jogo de nave (NN Studio)
                       3d: plataforma 3D em terceira pessoa (código Lua)
                       vazio: cena vazia com imagens e sons
                       minimo: só uma cena com um quadrado que anda

opções de exportar:
  --saida <pasta>      onde criar o executável (padrão: dist)
  --nome <nome>        nome do executável (padrão: nome da pasta do projeto)
  --runtime <arquivo>  nn-jogo a usar; para exportar para outro sistema,
                       passe o nn-jogo daquele sistema (ex.: nn-jogo.exe)";

fn main() -> ExitCode {
    // O servidor MCP usa a saída padrão para o protocolo: o log vai para a
    // saída de erros (o padrão do env_logger) e só os avisos.
    let level = if std::env::args().nth(1).as_deref() == Some("mcp") {
        "warn"
    } else {
        "info"
    };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level)).init();
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("-h" | "--help" | "ajuda") => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        Some("-V" | "--version" | "--versao") => {
            println!("nn {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("exportar") => exportar(&args[1..]),
        Some("novo") => novo(&args[1..]),
        Some("mcp") => mcp(&args[1..]),
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
                    .ok_or_else(|| {
                        format!("modelo desconhecido: {v} (use padrao, visual, aventura, nave, 3d, vazio ou minimo)")
                    })
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

/// `nn mcp [pasta]`: servidor MCP pela entrada e saída padrão.
fn mcp(args: &[String]) -> ExitCode {
    let dir = match args {
        [] => PathBuf::from("."),
        [dir] if !dir.starts_with("--") => PathBuf::from(dir),
        _ => {
            eprintln!("uso: nn mcp [pasta do jogo]");
            return ExitCode::FAILURE;
        }
    };
    if !dir.is_dir() {
        eprintln!("erro: a pasta {} não existe", dir.display());
        return ExitCode::FAILURE;
    }
    match nn_mcp::serve_stdio(nn_mcp::Server::new(dir, nn_run::mcp_hooks())) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("erro: {e}");
            ExitCode::FAILURE
        }
    }
}
