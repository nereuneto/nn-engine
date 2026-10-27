//! `nn`: cria, roda e exporta jogos em Lua.
//!
//! ```text
//! nn novo <pasta> [opções]       cria um jogo novo a partir de um modelo
//! nn [pasta] [--cena arquivo]   roda o jogo da pasta (padrão: pasta atual)
//! nn exportar <pasta> [opções]   gera um executável único para distribuir
//! nn testar [pasta] [opções]     roda os testes de jogo (testes/*.lua)
//! nn mcp [pasta]                 servidor MCP para agentes de IA
//! ```

use std::path::PathBuf;
use std::process::ExitCode;

use nn_run::{ExportOptions, NewOptions, Template, create_project, export, human_size};

const HELP: &str = "\
uso:
  nn novo <pasta> [opções]       cria um jogo novo, pronto para rodar
  nn [pasta] [--cena <arquivo>]  roda o jogo em Lua da pasta (padrão: pasta atual),
                                 começando na cena inicial ou na pedida
  nn exportar <pasta> [opções]   gera um executável único com o jogo
  nn testar [pasta] [opções]     roda os testes de jogo (arquivos Lua em testes/),
                                 sem janela; sai com erro se algum falhar
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
                       visual3d: a mesma ilha 3D montada no NN Studio
                       vale3d: um vale pintado num terreno (NN Studio)
                       vazio: cena vazia com imagens e sons
                       vazio3d: mundo 3D com chão, modelos e sons
                       minimo: só uma cena com um quadrado que anda

opções de exportar:
  --saida <pasta>      onde criar o executável (padrão: dist)
  --nome <nome>        nome do executável (padrão: nome da pasta do projeto)
  --runtime <arquivo>  nn-jogo a usar; para exportar para outro sistema,
                       passe o nn-jogo daquele sistema (ex.: nn-jogo.exe)

opções de testar:
  --filtro <texto>     só os testes cujo nome (ou arquivo) tem o texto
  --capturas <pasta>   grava a tela de quem falhar e as capturas dos testes
  --prints             mostra os prints também dos testes que passaram
  --tamanho <LxA>      tamanho da tela do jogo sem janela (ex.: 640x360; menor
                       roda mais rápido sem placa de vídeo)";

fn main() -> ExitCode {
    // O servidor MCP usa a saída padrão para o protocolo: o log vai para a
    // saída de erros (o padrão do env_logger) e só os avisos.
    // O servidor MCP e os testes só mostram os avisos. Os do driver de vídeo
    // (o jogo sem janela tenta vários) não interessam a quem faz o jogo.
    let level = match std::env::args().nth(1).as_deref() {
        Some("mcp" | "testar") => "warn,wgpu_hal=off",
        _ => "info",
    };
    // A captura do log (prints e erros do jogo sem janela: `nn testar` e o
    // servidor MCP) repassa tudo para o env_logger.
    let logger =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level)).build();
    let filter = logger.filter();
    if let Err(e) = nn_script::capture::install(Some(Box::new(logger)), filter) {
        eprintln!("log: {e}");
    }
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
        Some("testar") => testar(&args[1..]),
        Some(_) => rodar(&args),
        None => rodar(&[]),
    }
}

/// `nn [pasta] [--cena arquivo]`: abre o jogo numa janela.
fn rodar(args: &[String]) -> ExitCode {
    let mut dir = ".";
    let mut scene = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--cena" => match it.next() {
                Some(s) => scene = Some(s.as_str()),
                None => {
                    eprintln!("erro: --cena precisa de um valor\n\n{HELP}");
                    return ExitCode::FAILURE;
                }
            },
            other if other.starts_with("--") => {
                eprintln!("erro: opção desconhecida: {other}\n\n{HELP}");
                return ExitCode::FAILURE;
            }
            other => dir = other,
        }
    }
    match nn_script::run_project_from(dir, scene) {
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
        // Sem pasta: a do projeto do Claude Code (se for ele quem abriu), ou a atual.
        [] => std::env::var_os("CLAUDE_PROJECT_DIR")
            .filter(|d| !d.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".")),
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

/// Segundos com vírgula ("1,25 s").
fn segundos(t: f64) -> String {
    format!("{t:.2} s").replace('.', ",")
}

/// Um nome que dá para usar como nome de arquivo.
fn file_name(s: &str) -> String {
    let out: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    out.trim_matches('_').chars().take(60).collect()
}

/// `nn testar [pasta] [--filtro texto] [--capturas pasta]`.
fn testar(args: &[String]) -> ExitCode {
    let mut dir = None;
    let mut opts = nn_script::testing::TestOptions {
        capture_failures: false,
        ..Default::default()
    };
    let mut captures: Option<PathBuf> = None;
    let mut prints = false;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |flag: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{flag} precisa de um valor"))
        };
        let result = match arg.as_str() {
            "--filtro" => value("--filtro").map(|v| opts.filter = Some(v)),
            "--prints" => {
                prints = true;
                Ok(())
            }
            "--capturas" => value("--capturas").map(|v| captures = Some(v.into())),
            "--tamanho" => value("--tamanho").and_then(|v| {
                let size = v
                    .split_once(['x', 'X'])
                    .and_then(|(w, h)| Some((w.trim().parse().ok()?, h.trim().parse().ok()?)))
                    .filter(|(w, h): &(u32, u32)| *w > 0 && *h > 0)
                    .ok_or_else(|| {
                        format!("--tamanho é largura x altura, como 640x360 (veio {v})")
                    })?;
                opts.run.size = Some(size);
                Ok(())
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
    let dir = dir.unwrap_or_else(|| PathBuf::from("."));
    opts.capture_failures = captures.is_some();
    let project = match nn_script::Project::open(&dir) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("erro: {e}");
            return ExitCode::FAILURE;
        }
    };
    if let Some(c) = &captures
        && let Err(e) = std::fs::create_dir_all(c)
    {
        eprintln!("erro: {}: {e}", c.display());
        return ExitCode::FAILURE;
    }
    let mut saved = 0;
    let mut save = |name: &str, png: &[u8]| {
        if let Some(c) = &captures {
            saved += 1;
            let path = c.join(format!("{saved:02}-{}.png", file_name(name)));
            if let Err(e) = std::fs::write(&path, png) {
                eprintln!("erro: {}: {e}", path.display());
            }
        }
    };
    let result = nn_script::testing::run_tests(&project, &opts, |r| {
        match &r.failure {
            None => {
                println!(
                    "  ✓ {} › {} ({})",
                    r.case.file,
                    r.case.name,
                    segundos(r.time)
                );
                if prints {
                    for l in &r.report.lines {
                        if l.kind == nn_script::capture::LineKind::Print {
                            println!("      print em {}: {}", segundos(l.time), l.text);
                        }
                    }
                }
            }
            Some(f) => {
                println!(
                    "  ✗ {} › {} (falhou em {})",
                    r.case.file,
                    r.case.name,
                    segundos(r.time)
                );
                for line in f.lines() {
                    println!("      {line}");
                }
                for l in &r.report.lines {
                    let kind = match l.kind {
                        nn_script::capture::LineKind::Print => "print",
                        nn_script::capture::LineKind::Warning => "aviso",
                        nn_script::capture::LineKind::Error => "erro",
                    };
                    println!("      {kind} em {}: {}", segundos(l.time), l.text);
                }
                if let Some(c) = &r.last_frame {
                    save(&format!("{}-falhou", r.case.name), &c.png);
                }
            }
        }
        for c in &r.report.captures {
            save(&format!("{}-{}", r.case.name, c.label), &c.png);
        }
    });
    let summary = match result {
        Ok(s) => s,
        Err(e) => {
            eprintln!("erro: {e}");
            return ExitCode::FAILURE;
        }
    };
    for e in &summary.file_errors {
        println!("  ✗ {} não carregou:", e.file);
        for line in e.message.lines() {
            println!("      {line}");
        }
    }
    if summary.results.is_empty() && summary.file_errors.is_empty() {
        println!(
            "nenhum teste em {}/testes (escreva um arquivo .lua com test(\"nome\", function(t) ... end))",
            dir.display()
        );
        return ExitCode::SUCCESS;
    }
    let total = summary.results.len();
    println!(
        "\n{} {}: {} {}, {} {} ({})",
        total,
        if total == 1 { "teste" } else { "testes" },
        summary.passed(),
        if summary.passed() == 1 {
            "passou"
        } else {
            "passaram"
        },
        summary.failed(),
        if summary.failed() == 1 {
            "falhou"
        } else {
            "falharam"
        },
        segundos(summary.wall.as_secs_f64())
    );
    if let Some(c) = &captures
        && saved > 0
    {
        println!("capturas em {}", c.display());
    }
    if summary.ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
