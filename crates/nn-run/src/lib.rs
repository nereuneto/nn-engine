//! Ferramentas da linha de comando `nn`: criar projetos ([`create_project`])
//! e exportar jogos em Lua para um executável único ([`export`]).
//!
//! O executável exportado é o `nn-jogo` com todos os arquivos do projeto
//! anexados no fim (veja [`nn_app::Pack`]). Ao abrir, ele lê o pacote de si
//! mesmo e roda o jogo, sem precisar de Rust, da pasta do projeto ou de
//! qualquer instalação.

use std::path::{Path, PathBuf};
use std::process::Command;

use nn_app::{Pack, list_dir, normalize};
use nn_script::{Project, ScriptRuntime};

mod novo;

pub use novo::{NewOptions, NewReport, Template, agents_md, create_project};

/// O que o servidor MCP (`nn mcp`) precisa da ferramenta: os modelos de
/// projeto e como criar um.
pub fn mcp_hooks() -> nn_mcp::Hooks {
    fn create(dir: &Path, template: &str, title: Option<&str>) -> Result<Vec<String>, String> {
        let template = Template::from_name(template)
            .ok_or_else(|| format!("modelo desconhecido: {template}"))?;
        create_project(&NewOptions {
            dir: dir.to_path_buf(),
            title: title.map(str::to_string),
            template,
        })
        .map(|r| r.files)
    }
    nn_mcp::Hooks {
        templates: Template::ALL
            .iter()
            .map(|t| nn_mcp::TemplateInfo {
                name: t.name().to_string(),
                description: t.description().to_string(),
                three_d: t.is_3d(),
            })
            .collect(),
        create_project: Some(create),
        agents_md: agents_md(Template::Visual, "Seu jogo"),
        play: Some(play),
    }
}

/// Abre o jogo da pasta numa janela com o próprio `nn` (`nn <pasta> --cena`).
fn play(dir: &Path, scene: Option<&str>, log: &Path) -> std::io::Result<std::process::Child> {
    let exe = own_exe()?;
    let mut cmd = Command::new(&exe);
    cmd.arg(dir);
    if let Some(scene) = scene {
        cmd.args(["--cena", scene]);
    }
    let log = std::fs::File::create(log)?;
    cmd.current_dir(dir)
        .stdin(std::process::Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .spawn()
        .map_err(|e| std::io::Error::new(e.kind(), format!("{} ({e})", exe.display())))
}

/// O próprio executável. No Linux, se ele foi trocado com o programa aberto
/// (uma atualização, ou compilado de novo), o caminho ganha " (deleted)":
/// vale o arquivo novo no mesmo lugar.
fn own_exe() -> std::io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    if exe.exists() {
        return Ok(exe);
    }
    let text = exe.to_string_lossy();
    if let Some(path) = text.strip_suffix(" (deleted)").map(PathBuf::from)
        && path.exists()
    {
        return Ok(path);
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!(
            "o executável {} não existe mais (foi atualizado ou apagado com o servidor aberto); reinicie o servidor MCP",
            exe.display()
        ),
    ))
}

/// Sufixo dos executáveis nesta plataforma (`.exe` no Windows).
pub const EXE_SUFFIX: &str = std::env::consts::EXE_SUFFIX;

/// Opções de [`export`].
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// Pasta do projeto.
    pub project: PathBuf,
    /// Pasta onde o executável será criado.
    pub output_dir: PathBuf,
    /// Executável `nn-jogo` a usar. `None` procura (ou compila) o da engine.
    /// Para exportar para outro sistema, passe o `nn-jogo` daquele sistema.
    pub runtime: Option<PathBuf>,
    /// Nome do executável (sem extensão). Padrão: nome da pasta do projeto.
    pub name: Option<String>,
}

/// Resultado de [`export`].
#[derive(Debug, Clone)]
pub struct ExportReport {
    pub output: PathBuf,
    pub files: usize,
    pub assets_bytes: u64,
    pub total_bytes: u64,
    pub runtime: PathBuf,
}

/// Gera o executável do jogo.
pub fn export(opts: &ExportOptions) -> Result<ExportReport, String> {
    let project_dir = &opts.project;
    // Valida config.lua e a cena inicial.
    Project::open(project_dir).map_err(|e| e.to_string())?;

    // Erros de sintaxe aparecem agora, e não no computador de quem for jogar.
    let runtime = ScriptRuntime::new(project_dir).map_err(|e| e.to_string())?;
    let errors = runtime.check_syntax();
    if !errors.is_empty() {
        return Err(format!(
            "o projeto tem erros nos scripts:\n\n{}",
            errors.join("\n\n")
        ));
    }

    // Arquivos do projeto, sem a pasta de saída (se ela estiver dentro dele).
    let skip = opts
        .output_dir
        .canonicalize()
        .ok()
        .and_then(|out| {
            let proj = project_dir.canonicalize().ok()?;
            out.strip_prefix(&proj).ok().map(normalize)
        })
        .filter(|s| !s.is_empty());
    let mut files = Vec::new();
    let tests = format!("{}/", nn_script::testing::TEST_DIR);
    for (rel, full) in list_dir(project_dir) {
        if skip
            .as_ref()
            .is_some_and(|s| rel == *s || rel.starts_with(&format!("{s}/")))
        {
            continue;
        }
        // Os testes de jogo e o guia do agente de IA não vão para quem joga.
        if rel.starts_with(&tests) || rel == "AGENTS.md" {
            continue;
        }
        let data = std::fs::read(&full).map_err(|e| format!("{}: {e}", full.display()))?;
        files.push((rel, data));
    }
    let assets_bytes: u64 = files.iter().map(|(_, d)| d.len() as u64).sum();
    let pack = Pack::build(files.iter().map(|(p, d)| (p.as_str(), d.as_slice())));

    let runtime_path = match &opts.runtime {
        Some(p) => p.clone(),
        None => find_runtime()?,
    };
    let runtime_bytes =
        std::fs::read(&runtime_path).map_err(|e| format!("{}: {e}", runtime_path.display()))?;
    if Pack::read_appended(&runtime_path).ok().flatten().is_some() {
        return Err(format!(
            "{} já é um jogo exportado; use o nn-jogo original",
            runtime_path.display()
        ));
    }

    let name = match &opts.name {
        Some(n) => n.clone(),
        None => project_dir
            .canonicalize()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "jogo".into()),
    };
    let ext = if runtime_path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
    {
        ".exe"
    } else {
        ""
    };
    std::fs::create_dir_all(&opts.output_dir)
        .map_err(|e| format!("{}: {e}", opts.output_dir.display()))?;
    let output = opts.output_dir.join(format!("{name}{ext}"));
    let bytes = Pack::append_to(&runtime_bytes, &pack);
    std::fs::write(&output, &bytes).map_err(|e| format!("{}: {e}", output.display()))?;
    copy_permissions(&runtime_path, &output);

    Ok(ExportReport {
        output,
        files: files.len(),
        assets_bytes,
        total_bytes: bytes.len() as u64,
        runtime: runtime_path,
    })
}

#[cfg(unix)]
fn copy_permissions(from: &Path, to: &Path) {
    if let Ok(meta) = std::fs::metadata(from) {
        let _ = std::fs::set_permissions(to, meta.permissions());
    }
}

#[cfg(not(unix))]
fn copy_permissions(_from: &Path, _to: &Path) {}

/// Encontra o `nn-jogo` da engine, compilando em modo release se preciso.
pub fn find_runtime() -> Result<PathBuf, String> {
    let file = format!("nn-jogo{EXE_SUFFIX}");
    // A própria ferramenta em release: o nn-jogo fica ao lado.
    if !cfg!(debug_assertions)
        && let Some(sibling) = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join(&file)))
        && sibling.exists()
    {
        return Ok(sibling);
    }

    // Rodando a partir do código da engine: compila o nn-jogo otimizado.
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let workspace = workspace.canonicalize().unwrap_or(workspace);
    if workspace.join("Cargo.toml").exists() {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        eprintln!(
            "compilando o executável do jogo (release); a primeira vez demora alguns minutos..."
        );
        let status = Command::new(cargo)
            .current_dir(&workspace)
            .args(["build", "--release", "-p", "nn-run", "--bin", "nn-jogo"])
            .status()
            .map_err(|e| format!("não foi possível rodar o cargo: {e}"))?;
        if !status.success() {
            return Err("a compilação do nn-jogo falhou".into());
        }
        let target = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| workspace.join("target"));
        let built = target.join("release").join(&file);
        if built.exists() {
            return Ok(built);
        }
    }
    Err(format!(
        "não encontrei o {file}; coloque-o ao lado do nn ou informe com --runtime"
    ))
}

/// Tamanho legível ("1,5 MB").
pub fn human_size(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1024.0 * 1024.0 {
        format!("{:.1} MB", b / (1024.0 * 1024.0)).replace('.', ",")
    } else if b >= 1024.0 {
        format!("{:.0} KB", b / 1024.0)
    } else {
        format!("{bytes} bytes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("nn-export-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn write(dir: &Path, path: &str, content: &str) {
        let p = dir.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    #[test]
    fn exporta_com_runtime_informado() {
        let dir = temp("ok");
        let proj = dir.join("meu-jogo");
        write(&proj, "main.lua", "return {}");
        write(&proj, "lib/a.lua", "return 1");
        write(&proj, ".git/config", "ignorado");
        write(&proj, "testes/jogo.lua", "test(\"x\", function(t) end)");
        write(&proj, "AGENTS.md", "# para o agente");
        write(&proj, ".mcp.json", "{}");
        let runtime = dir.join("nn-jogo.exe");
        std::fs::write(&runtime, b"RUNTIME").unwrap();
        // Saída dentro do projeto: não pode entrar no pacote.
        let out = proj.join("dist");

        let r = export(&ExportOptions {
            project: proj.clone(),
            output_dir: out.clone(),
            runtime: Some(runtime),
            name: None,
        })
        .unwrap();
        assert_eq!(r.output, out.join("meu-jogo.exe"));
        assert_eq!(r.files, 2);

        let bytes = std::fs::read(&r.output).unwrap();
        assert!(bytes.starts_with(b"RUNTIME"));
        let pack = Pack::read_appended(&r.output).unwrap().unwrap();
        let mut paths: Vec<_> = pack.paths().collect();
        paths.sort();
        assert_eq!(paths, ["lib/a.lua", "main.lua"]);

        // Exportar de novo não inclui o executável anterior.
        let r2 = export(&ExportOptions {
            project: proj,
            output_dir: out,
            runtime: Some(dir.join("nn-jogo.exe")),
            name: Some("outro".into()),
        })
        .unwrap();
        assert_eq!(r2.files, 2);
    }

    #[test]
    fn recusa_projeto_com_erro_de_sintaxe() {
        let dir = temp("sintaxe");
        write(&dir, "main.lua", "return {");
        let runtime = dir.join("rt");
        std::fs::write(&runtime, b"RT").unwrap();
        let err = export(&ExportOptions {
            project: dir.clone(),
            output_dir: dir.join("dist"),
            runtime: Some(runtime),
            name: None,
        })
        .unwrap_err();
        assert!(err.contains("main.lua"), "{err}");
    }

    #[test]
    fn recusa_runtime_que_ja_e_um_jogo() {
        let dir = temp("duplo");
        let proj = dir.join("p");
        write(&proj, "main.lua", "return {}");
        let runtime = dir.join("rt");
        std::fs::write(
            &runtime,
            Pack::append_to(b"RT", &Pack::build([("x", b"1".as_slice())])),
        )
        .unwrap();
        let err = export(&ExportOptions {
            project: proj,
            output_dir: dir.join("dist"),
            runtime: Some(runtime),
            name: None,
        })
        .unwrap_err();
        assert!(err.contains("já é um jogo exportado"), "{err}");
    }

    #[test]
    fn tamanhos_legiveis() {
        assert_eq!(human_size(512), "512 bytes");
        assert_eq!(human_size(2048), "2 KB");
        assert_eq!(human_size(3 * 1024 * 1024 / 2), "1,5 MB");
    }
}
