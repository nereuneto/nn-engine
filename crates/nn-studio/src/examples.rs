//! Jogos de exemplo que vão dentro do executável. Abrir um exemplo cria uma
//! cópia numa pasta do usuário, para ele mexer à vontade.

use std::path::{Path, PathBuf};

use include_dir::{Dir, include_dir};

static PLATAFORMA: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../jogos/plataforma");
static DEMO: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../jogos/demo");
static CUBOS3D: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../jogos/cubos3d");

#[derive(Debug, Clone, Copy)]
pub struct Example {
    /// Nome da pasta criada.
    pub folder: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    dir: &'static Dir<'static>,
}

pub const EXAMPLES: &[Example] = &[
    Example {
        folder: "plataforma",
        title: "Plataforma",
        description: "Jogo de plataforma com duas fases, inimigos, moedas, menus, \
                      opções e recordes salvos. Mapas feitos no Tiled.",
        dir: &PLATAFORMA,
    },
    Example {
        folder: "demonstracao",
        title: "Demonstração",
        description: "Um passeio pelos recursos da engine: sprites, animação, \
                      partículas, física, som e cenas.",
        dir: &DEMO,
    },
    Example {
        folder: "cubos3d",
        title: "Cubos 3D",
        description: "Um campo low-poly em 3D, todo em código Lua: modelos .glb, \
                      sombras, céu, um herói que anda e câmera que gira com o mouse.",
        dir: &CUBOS3D,
    },
];

impl Example {
    /// Quantidade de arquivos do exemplo.
    pub fn file_count(&self) -> usize {
        count(self.dir)
    }

    /// Copia o exemplo para dentro de `parent`, numa pasta nova (se já
    /// existir, usa "plataforma-2", "plataforma-3"...). Devolve a pasta criada.
    pub fn copy_to(&self, parent: &Path) -> std::io::Result<PathBuf> {
        let mut target = parent.join(self.folder);
        let mut n = 2;
        while target.exists() {
            target = parent.join(format!("{}-{n}", self.folder));
            n += 1;
        }
        std::fs::create_dir_all(&target)?;
        write_dir(self.dir, &target)?;
        Ok(target)
    }
}

fn count(dir: &Dir<'_>) -> usize {
    dir.files().count() + dir.dirs().map(count).sum::<usize>()
}

fn write_dir(dir: &Dir<'_>, target: &Path) -> std::io::Result<()> {
    for file in dir.files() {
        // Os caminhos do include_dir são relativos à raiz do exemplo.
        let path = target.join(file.path());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, file.contents())?;
    }
    for sub in dir.dirs() {
        write_dir(sub, target)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copia_exemplos_que_abrem() {
        let parent = std::env::temp_dir().join(format!("nn-studio-ex-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&parent);
        for ex in EXAMPLES {
            assert!(ex.file_count() >= 2, "{}", ex.folder);
            let a = ex.copy_to(&parent).unwrap();
            let b = ex.copy_to(&parent).unwrap();
            assert_eq!(a, parent.join(ex.folder));
            assert_eq!(b, parent.join(format!("{}-2", ex.folder)));
            // A cópia é um projeto válido.
            nn_script::Project::open(&b).unwrap();
            assert_eq!(nn_app::list_dir(&a).len(), ex.file_count(), "{}", ex.folder);
        }
    }
}
