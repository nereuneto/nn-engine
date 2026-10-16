//! `nn novo`: cria um projeto de jogo em Lua a partir de um modelo.

use std::path::{Path, PathBuf};

/// Modelo de projeto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Template {
    /// Jogo completo e pequeno: título com menu, partida, pausa, fim e
    /// recorde salvo. Mostra o jeito recomendado de organizar um jogo.
    #[default]
    Padrao,
    /// Só uma cena com um quadrado que anda.
    Minimo,
    /// Fase de plataforma montada no NN Studio (cena visual `.cena`), sem
    /// código: comportamentos prontos e um prefab. O jeito de começar pelo
    /// editor.
    Visual,
}

impl Template {
    pub const ALL: [Template; 3] = [Template::Padrao, Template::Visual, Template::Minimo];

    pub fn name(self) -> &'static str {
        match self {
            Template::Padrao => "padrao",
            Template::Minimo => "minimo",
            Template::Visual => "visual",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "padrao" | "padrão" => Some(Template::Padrao),
            "minimo" | "mínimo" => Some(Template::Minimo),
            "visual" => Some(Template::Visual),
            _ => None,
        }
    }

    fn files(self) -> &'static [(&'static str, &'static [u8])] {
        macro_rules! modelo {
            ($dir:literal: $($path:literal),* $(,)?) => {
                &[$(($path, include_bytes!(concat!("../modelos/", $dir, "/", $path)))),*]
            };
        }
        match self {
            Template::Padrao => modelo!("padrao":
                "config.lua",
                "LEIAME.md",
                "lib/jogo.lua",
                "cenas/titulo.lua",
                "cenas/jogo.lua",
                "cenas/pausa.lua",
                "cenas/fim.lua",
                "assets/imagens/heroi.png",
                "assets/imagens/moeda.png",
                "assets/sons/moeda.wav",
                "assets/sons/fim.wav",
                "assets/sons/menu.wav",
                "assets/sons/confirmar.wav",
            ),
            Template::Minimo => modelo!("minimo": "config.lua", "main.lua"),
            Template::Visual => &[
                ("config.lua", include_bytes!("../modelos/visual/config.lua")),
                ("LEIAME.md", include_bytes!("../modelos/visual/LEIAME.md")),
                (
                    "cenas/fase.cena",
                    include_bytes!("../modelos/visual/cenas/fase.cena"),
                ),
                (
                    "cenas/fim.cena",
                    include_bytes!("../modelos/visual/cenas/fim.cena"),
                ),
                (
                    "prefabs/moeda.prefab",
                    include_bytes!("../modelos/visual/prefabs/moeda.prefab"),
                ),
                // As imagens e o som são os mesmos do modelo padrão.
                (
                    "assets/imagens/heroi.png",
                    include_bytes!("../modelos/padrao/assets/imagens/heroi.png"),
                ),
                (
                    "assets/imagens/moeda.png",
                    include_bytes!("../modelos/padrao/assets/imagens/moeda.png"),
                ),
                (
                    "assets/sons/moeda.wav",
                    include_bytes!("../modelos/padrao/assets/sons/moeda.wav"),
                ),
            ],
        }
    }
}

/// Opções de [`create_project`].
#[derive(Debug, Clone, Default)]
pub struct NewOptions {
    /// Pasta a criar (não pode existir com arquivos dentro).
    pub dir: PathBuf,
    /// Título do jogo. Padrão: o nome da pasta ("meu-jogo" vira "Meu jogo").
    pub title: Option<String>,
    pub template: Template,
}

/// Resultado de [`create_project`].
#[derive(Debug, Clone)]
pub struct NewReport {
    pub dir: PathBuf,
    pub title: String,
    pub id: String,
    /// Arquivos criados, relativos à pasta.
    pub files: Vec<String>,
}

/// Cria o projeto.
pub fn create_project(opts: &NewOptions) -> Result<NewReport, String> {
    let dir = &opts.dir;
    if dir.as_os_str().is_empty() {
        return Err("informe a pasta do novo jogo".into());
    }
    if dir.exists() {
        let has_files = std::fs::read_dir(dir)
            .map_err(|e| format!("não foi possível ler {}: {e}", dir.display()))?
            .next()
            .is_some();
        if has_files {
            return Err(format!(
                "a pasta {} já existe e não está vazia; escolha outro nome",
                dir.display()
            ));
        }
    }

    let folder = folder_name(dir);
    let title = opts
        .title
        .clone()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| title_from_folder(&folder));
    let id = nn_app::slug(&folder);

    let mut files = Vec::new();
    for (path, bytes) in opts.template.files() {
        let target = dir.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("não foi possível criar {}: {e}", parent.display()))?;
        }
        let content = fill(path, bytes, &title, &id);
        std::fs::write(&target, content)
            .map_err(|e| format!("não foi possível criar {}: {e}", target.display()))?;
        files.push(path.to_string());
    }
    // Para quem usar git: a pasta da exportação não vai para o repositório.
    std::fs::write(dir.join(".gitignore"), "dist/\n")
        .map_err(|e| format!("não foi possível criar .gitignore: {e}"))?;
    files.push(".gitignore".into());

    Ok(NewReport {
        dir: dir.clone(),
        title,
        id,
        files,
    })
}

/// Troca os marcadores do modelo. Em `.lua`, o título vai dentro de uma
/// string Lua, então aspas e barras são escapadas.
fn fill(path: &str, bytes: &[u8], title: &str, id: &str) -> Vec<u8> {
    let is_text = path.ends_with(".lua") || path.ends_with(".md");
    let Ok(text) = std::str::from_utf8(bytes) else {
        return bytes.to_vec();
    };
    if !is_text {
        return bytes.to_vec();
    }
    let title = if path.ends_with(".lua") {
        title
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', " ")
    } else {
        title.to_string()
    };
    text.replace("{{TITULO}}", &title)
        .replace("{{ID}}", id)
        .into_bytes()
}

fn folder_name(dir: &Path) -> String {
    // "." ou ".." não têm nome: usa o da pasta de verdade.
    let name = dir.file_name().map(ToOwned::to_owned).or_else(|| {
        std::path::absolute(dir)
            .ok()
            .and_then(|p| p.components().next_back().map(|c| c.as_os_str().to_owned()))
    });
    name.map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty() && n != "." && n != "..")
        .unwrap_or_else(|| "meu-jogo".into())
}

/// "meu-jogo" → "Meu jogo".
fn title_from_folder(folder: &str) -> String {
    let words = folder.replace(['-', '_'], " ");
    let words = words.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => "Meu jogo".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titulo_a_partir_da_pasta() {
        assert_eq!(title_from_folder("meu-jogo"), "Meu jogo");
        assert_eq!(title_from_folder("nave_espacial  2"), "Nave espacial 2");
        assert_eq!(title_from_folder("ágil"), "Ágil");
        assert_eq!(title_from_folder("--"), "Meu jogo");
    }

    #[test]
    fn titulo_com_aspas_vira_string_lua_valida() {
        let out = fill(
            "config.lua",
            b"t = \"{{TITULO}}\"",
            "O \"Grande\" \\ Jogo",
            "x",
        );
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "t = \"O \\\"Grande\\\" \\\\ Jogo\""
        );
        let md = fill("LEIAME.md", b"# {{TITULO}}", "O \"Grande\"", "x");
        assert_eq!(md, b"# O \"Grande\"");
    }

    #[test]
    fn nomes_dos_modelos() {
        for t in Template::ALL {
            assert_eq!(Template::from_name(t.name()), Some(t));
            assert!(!t.files().is_empty());
        }
        assert_eq!(Template::from_name("padrão"), Some(Template::Padrao));
        assert_eq!(Template::from_name("rpg"), None);
    }
}
