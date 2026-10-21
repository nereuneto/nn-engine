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
    /// código: comportamentos prontos, eventos e um prefab. O jeito de
    /// começar pelo editor.
    Visual,
    /// Aventura vista de cima, sem código: chave, porta, inimigos e tesouro.
    Aventura,
    /// Jogo de nave, sem código: tiros, asteroides, pontos e recorde salvo.
    Nave,
    /// Cena visual vazia com um pacote de imagens e sons (o tutorial começa
    /// daqui).
    Vazio,
    /// Jogo 3D em terceira pessoa (o "Ilha das Estrelas"): câmera, personagem,
    /// física, plataformas que andam, menu, pausa e recorde.
    TresD,
    /// A ilha das estrelas montada no NN Studio, sem código: Jogador 3D,
    /// corpos 3D, plataformas que andam, prefabs e eventos.
    Visual3D,
    /// Mundo 3D vazio (câmera e chão) com modelos e sons (o tutorial 3D
    /// começa daqui).
    Vazio3D,
    /// Um vale num Terreno pintado (morros, lago, trilha), sem código.
    Vale3D,
}

impl Template {
    pub const ALL: [Template; 10] = [
        Template::Padrao,
        Template::Visual,
        Template::Aventura,
        Template::Nave,
        Template::Visual3D,
        Template::Vale3D,
        Template::TresD,
        Template::Vazio,
        Template::Vazio3D,
        Template::Minimo,
    ];

    /// Cria um jogo 3D?
    pub fn is_3d(self) -> bool {
        matches!(
            self,
            Template::TresD | Template::Visual3D | Template::Vazio3D | Template::Vale3D
        )
    }

    pub fn name(self) -> &'static str {
        match self {
            Template::Padrao => "padrao",
            Template::Minimo => "minimo",
            Template::Visual => "visual",
            Template::Aventura => "aventura",
            Template::Nave => "nave",
            Template::Vazio => "vazio",
            Template::TresD => "3d",
            Template::Visual3D => "visual3d",
            Template::Vazio3D => "vazio3d",
            Template::Vale3D => "vale3d",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "padrao" | "padrão" => Some(Template::Padrao),
            "minimo" | "mínimo" => Some(Template::Minimo),
            "visual" => Some(Template::Visual),
            "aventura" => Some(Template::Aventura),
            "nave" => Some(Template::Nave),
            "vazio" => Some(Template::Vazio),
            "3d" | "3D" => Some(Template::TresD),
            "visual3d" | "3d-visual" => Some(Template::Visual3D),
            "vazio3d" | "3d-vazio" => Some(Template::Vazio3D),
            "vale3d" | "vale" => Some(Template::Vale3D),
            _ => None,
        }
    }

    fn files(self) -> &'static [(&'static str, &'static [u8])] {
        // Cada arquivo: "caminho" (do próprio modelo) ou ="caminho" "origem"
        // (de outra pasta de modelos: as imagens e sons são compartilhados).
        macro_rules! modelo {
            ($dir:literal: $($(=$dest:literal)? $path:literal),* $(,)?) => {
                &[$(modelo!(@one $dir, $(=$dest)? $path)),*]
            };
            (@one $dir:literal, $path:literal) => {
                ($path, include_bytes!(concat!("../modelos/", $dir, "/", $path)))
            };
            (@one $dir:literal, =$dest:literal $src:literal) => {
                ($dest, include_bytes!(concat!("../modelos/", $src)))
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
            Template::Visual => modelo!("visual":
                "config.lua",
                "LEIAME.md",
                "cenas/titulo.cena",
                "cenas/fase.cena",
                "cenas/fim.cena",
                "cenas/pausa.cena",
                "cenas/opcoes.cena",
                "prefabs/moeda.prefab",
                "prefabs/brilho.prefab",
                "mapas/fase.tmj",
                ="assets/imagens/coracao.png" "comum/imagens/coracao.png",
                ="assets/imagens/terreno.png" "comum/imagens/terreno.png",
                ="assets/imagens/terreno.tsj" "comum/imagens/terreno.tsj",
                ="assets/imagens/heroi.png" "padrao/assets/imagens/heroi.png",
                ="assets/imagens/heroi.anim" "padrao/assets/imagens/heroi.anim",
                ="assets/imagens/moeda.png" "padrao/assets/imagens/moeda.png",
                ="assets/imagens/moeda.anim" "padrao/assets/imagens/moeda.anim",
                ="assets/sons/moeda.wav" "padrao/assets/sons/moeda.wav",
            ),
            Template::Aventura => modelo!("aventura":
                "config.lua",
                "LEIAME.md",
                "cenas/titulo.cena",
                "cenas/mapa.cena",
                "cenas/fim.cena",
                "dialogos/velho.dialogo",
                "mapas/mapa.tmj",
                ="assets/imagens/coracao.png" "comum/imagens/coracao.png",
                ="assets/imagens/terreno.png" "comum/imagens/terreno.png",
                ="assets/imagens/terreno.tsj" "comum/imagens/terreno.tsj",
                ="assets/imagens/heroi.png" "padrao/assets/imagens/heroi.png",
                ="assets/imagens/heroi.anim" "padrao/assets/imagens/heroi.anim",
                ="assets/imagens/chave.png" "comum/imagens/chave.png",
                ="assets/imagens/porta.png" "comum/imagens/porta.png",
                ="assets/imagens/bau.png" "comum/imagens/bau.png",
                ="assets/imagens/inimigos.png" "comum/imagens/inimigos.png",
                ="assets/sons/chave.wav" "comum/sons/chave.wav",
                ="assets/sons/porta.wav" "comum/sons/porta.wav",
                ="assets/sons/dano.wav" "comum/sons/dano.wav",
                ="assets/sons/vitoria.wav" "comum/sons/vitoria.wav",
            ),
            Template::Nave => modelo!("nave":
                "config.lua",
                "LEIAME.md",
                "cenas/titulo.cena",
                "cenas/jogo.cena",
                "cenas/fim.cena",
                "prefabs/asteroide.prefab",
                "prefabs/tiro.prefab",
                "prefabs/estrela.prefab",
                "prefabs/explosao.prefab",
                ="assets/imagens/nave.png" "comum/imagens/nave.png",
                ="assets/imagens/asteroide.png" "comum/imagens/asteroide.png",
                ="assets/imagens/tiro.png" "comum/imagens/tiro.png",
                ="assets/sons/tiro.wav" "comum/sons/tiro.wav",
                ="assets/sons/explosao.wav" "comum/sons/explosao.wav",
            ),
            // As cenas, os modelos e os sons vêm do jogo de exemplo
            // (jogos/estrelas3d), para não haver duas cópias.
            Template::TresD => modelo!("3d":
                "config.lua",
                "LEIAME.md",
                ="cenas/titulo.lua" "../../../jogos/estrelas3d/cenas/titulo.lua",
                ="cenas/jogo.lua" "../../../jogos/estrelas3d/cenas/jogo.lua",
                ="cenas/pausa.lua" "../../../jogos/estrelas3d/cenas/pausa.lua",
                ="cenas/fim.lua" "../../../jogos/estrelas3d/cenas/fim.lua",
                ="lib/jogo.lua" "../../../jogos/estrelas3d/lib/jogo.lua",
                ="lib/ilha.lua" "../../../jogos/estrelas3d/lib/ilha.lua",
                ="modelos/arvore.glb" "../../../jogos/estrelas3d/modelos/arvore.glb",
                ="modelos/caixote.glb" "../../../jogos/estrelas3d/modelos/caixote.glb",
                ="modelos/estrela.glb" "../../../jogos/estrelas3d/modelos/estrela.glb",
                ="modelos/heroi.glb" "../../../jogos/estrelas3d/modelos/heroi.glb",
                ="modelos/moeda.glb" "../../../jogos/estrelas3d/modelos/moeda.glb",
                ="modelos/pedra.glb" "../../../jogos/estrelas3d/modelos/pedra.glb",
                ="modelos/plataforma.glb" "../../../jogos/estrelas3d/modelos/plataforma.glb",
                ="assets/sons/moeda.wav" "padrao/assets/sons/moeda.wav",
                ="assets/sons/menu.wav" "padrao/assets/sons/menu.wav",
                ="assets/sons/confirmar.wav" "padrao/assets/sons/confirmar.wav",
                ="assets/sons/pulo.wav" "comum/sons/pulo.wav",
                ="assets/sons/vitoria.wav" "comum/sons/vitoria.wav",
                ="assets/sons/dano.wav" "comum/sons/dano.wav",
            ),
            Template::Visual3D => modelo!("visual3d":
                "config.lua",
                ="assets/imagens/coracao.png" "comum/imagens/coracao.png",
                ="assets/imagens/chave.png" "comum/imagens/chave.png",
                "LEIAME.md",
                "cenas/ilha.cena",
                "cenas/fim.cena",
                "dialogos/pescador.dialogo",
                "prefabs/moeda.prefab",
                "prefabs/estrela.prefab",
                ="modelos/arvore.glb" "../../../jogos/estrelas3d/modelos/arvore.glb",
                ="modelos/caixote.glb" "../../../jogos/estrelas3d/modelos/caixote.glb",
                ="modelos/estrela.glb" "../../../jogos/estrelas3d/modelos/estrela.glb",
                ="modelos/heroi.glb" "../../../jogos/estrelas3d/modelos/heroi.glb",
                ="modelos/moeda.glb" "../../../jogos/estrelas3d/modelos/moeda.glb",
                ="modelos/pedra.glb" "../../../jogos/estrelas3d/modelos/pedra.glb",
                ="modelos/plataforma.glb" "../../../jogos/estrelas3d/modelos/plataforma.glb",
                ="modelos/bicho.glb" "../../../jogos/estrelas3d/modelos/bicho.glb",
                ="modelos/chave.glb" "../../../jogos/estrelas3d/modelos/chave.glb",
                ="modelos/porta.glb" "../../../jogos/estrelas3d/modelos/porta.glb",
                ="modelos/espada.glb" "../../../jogos/estrelas3d/modelos/espada.glb",
                ="modelos/cajado.glb" "../../../jogos/estrelas3d/modelos/cajado.glb",
                ="assets/sons/moeda.wav" "padrao/assets/sons/moeda.wav",
                ="assets/sons/pulo.wav" "comum/sons/pulo.wav",
                ="assets/sons/vitoria.wav" "comum/sons/vitoria.wav",
                ="assets/sons/dano.wav" "comum/sons/dano.wav",
                ="assets/sons/fogueira.wav" "comum/sons/fogueira.wav",
                ="assets/sons/mar.wav" "comum/sons/mar.wav",
                ="assets/sons/chave.wav" "comum/sons/chave.wav",
                ="assets/sons/porta.wav" "comum/sons/porta.wav",
            ),
            Template::Vazio3D => modelo!("vazio3d":
                "config.lua",
                ="assets/imagens/coracao.png" "comum/imagens/coracao.png",
                ="assets/imagens/chave.png" "comum/imagens/chave.png",
                "LEIAME.md",
                "cenas/mundo.cena",
                ="modelos/arvore.glb" "../../../jogos/estrelas3d/modelos/arvore.glb",
                ="modelos/caixote.glb" "../../../jogos/estrelas3d/modelos/caixote.glb",
                ="modelos/estrela.glb" "../../../jogos/estrelas3d/modelos/estrela.glb",
                ="modelos/heroi.glb" "../../../jogos/estrelas3d/modelos/heroi.glb",
                ="modelos/moeda.glb" "../../../jogos/estrelas3d/modelos/moeda.glb",
                ="modelos/pedra.glb" "../../../jogos/estrelas3d/modelos/pedra.glb",
                ="modelos/plataforma.glb" "../../../jogos/estrelas3d/modelos/plataforma.glb",
                ="modelos/bicho.glb" "../../../jogos/estrelas3d/modelos/bicho.glb",
                ="modelos/chave.glb" "../../../jogos/estrelas3d/modelos/chave.glb",
                ="modelos/porta.glb" "../../../jogos/estrelas3d/modelos/porta.glb",
                ="modelos/espada.glb" "../../../jogos/estrelas3d/modelos/espada.glb",
                ="modelos/cajado.glb" "../../../jogos/estrelas3d/modelos/cajado.glb",
                ="assets/sons/moeda.wav" "padrao/assets/sons/moeda.wav",
                ="assets/sons/pulo.wav" "comum/sons/pulo.wav",
                ="assets/sons/vitoria.wav" "comum/sons/vitoria.wav",
                ="assets/sons/dano.wav" "comum/sons/dano.wav",
                ="assets/sons/fogueira.wav" "comum/sons/fogueira.wav",
                ="assets/sons/mar.wav" "comum/sons/mar.wav",
                ="assets/sons/chave.wav" "comum/sons/chave.wav",
                ="assets/sons/porta.wav" "comum/sons/porta.wav",
            ),
            Template::Vale3D => modelo!("vale3d":
                "config.lua",
                ="assets/imagens/coracao.png" "comum/imagens/coracao.png",
                ="assets/imagens/chave.png" "comum/imagens/chave.png",
                "LEIAME.md",
                "cenas/vale.cena",
                "cenas/fim.cena",
                "prefabs/magia.prefab",
                ="prefabs/moeda.prefab" "visual3d/prefabs/moeda.prefab",
                ="prefabs/estrela.prefab" "visual3d/prefabs/estrela.prefab",
                ="modelos/arvore.glb" "../../../jogos/estrelas3d/modelos/arvore.glb",
                ="modelos/caixote.glb" "../../../jogos/estrelas3d/modelos/caixote.glb",
                ="modelos/estrela.glb" "../../../jogos/estrelas3d/modelos/estrela.glb",
                ="modelos/heroi.glb" "../../../jogos/estrelas3d/modelos/heroi.glb",
                ="modelos/moeda.glb" "../../../jogos/estrelas3d/modelos/moeda.glb",
                ="modelos/pedra.glb" "../../../jogos/estrelas3d/modelos/pedra.glb",
                ="modelos/plataforma.glb" "../../../jogos/estrelas3d/modelos/plataforma.glb",
                ="modelos/bicho.glb" "../../../jogos/estrelas3d/modelos/bicho.glb",
                ="modelos/chave.glb" "../../../jogos/estrelas3d/modelos/chave.glb",
                ="modelos/porta.glb" "../../../jogos/estrelas3d/modelos/porta.glb",
                ="modelos/espada.glb" "../../../jogos/estrelas3d/modelos/espada.glb",
                ="modelos/cajado.glb" "../../../jogos/estrelas3d/modelos/cajado.glb",
                ="assets/sons/moeda.wav" "padrao/assets/sons/moeda.wav",
                ="assets/sons/pulo.wav" "comum/sons/pulo.wav",
                ="assets/sons/vitoria.wav" "comum/sons/vitoria.wav",
                ="assets/sons/dano.wav" "comum/sons/dano.wav",
                ="assets/sons/tiro.wav" "comum/sons/tiro.wav",
                ="assets/sons/fogueira.wav" "comum/sons/fogueira.wav",
                ="assets/sons/mar.wav" "comum/sons/mar.wav",
                ="assets/sons/chave.wav" "comum/sons/chave.wav",
                ="assets/sons/porta.wav" "comum/sons/porta.wav",
            ),
            Template::Vazio => modelo!("vazio":
                "config.lua",
                "LEIAME.md",
                "cenas/fase.cena",
                ="assets/imagens/coracao.png" "comum/imagens/coracao.png",
                ="assets/imagens/heroi.png" "padrao/assets/imagens/heroi.png",
                ="assets/imagens/heroi.anim" "padrao/assets/imagens/heroi.anim",
                ="assets/imagens/moeda.png" "padrao/assets/imagens/moeda.png",
                ="assets/imagens/moeda.anim" "padrao/assets/imagens/moeda.anim",
                ="assets/imagens/bloco.png" "comum/imagens/bloco.png",
                ="assets/imagens/bandeira.png" "comum/imagens/bandeira.png",
                ="assets/imagens/inimigos.png" "comum/imagens/inimigos.png",
                ="assets/imagens/chave.png" "comum/imagens/chave.png",
                ="assets/imagens/terreno.png" "comum/imagens/terreno.png",
                ="assets/imagens/terreno.tsj" "comum/imagens/terreno.tsj",
                ="assets/sons/moeda.wav" "padrao/assets/sons/moeda.wav",
                ="assets/sons/pulo.wav" "comum/sons/pulo.wav",
                ="assets/sons/dano.wav" "comum/sons/dano.wav",
                ="assets/sons/vitoria.wav" "comum/sons/vitoria.wav",
            ),
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
    let is_lua = [".lua", ".cena", ".prefab"]
        .iter()
        .any(|e| path.ends_with(e));
    let is_text = is_lua || path.ends_with(".md");
    let Ok(text) = std::str::from_utf8(bytes) else {
        return bytes.to_vec();
    };
    if !is_text {
        return bytes.to_vec();
    }
    let title = if is_lua {
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
