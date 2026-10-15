//! Pasta de assets: carregamento com cache e recarga quando o arquivo muda.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use nn_audio::{AudioError, SoundId};
use nn_render::{FontId, RenderError, TextureId, TextureOptions};

use crate::Context;

/// Erros ao carregar assets.
#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("asset não encontrado: {0}")]
    NotFound(PathBuf),
    #[error("falha ao ler {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{path}: {source}")]
    Render { path: PathBuf, source: RenderError },
    #[error("{path}: {source}")]
    Audio { path: PathBuf, source: AudioError },
}

#[derive(Debug, Clone, Copy)]
struct Tracked<T> {
    id: T,
    modified: Option<SystemTime>,
}

/// Assets carregados a partir da pasta do jogo.
///
/// Cada arquivo é carregado uma única vez: pedir o mesmo caminho de novo
/// devolve o mesmo id. Com a recarga ligada, imagens e sons alterados no
/// disco são recarregados sozinhos, mantendo o id.
#[derive(Debug)]
pub struct Assets {
    root: PathBuf,
    hot_reload: bool,
    textures: HashMap<(PathBuf, TextureOptions), Tracked<TextureId>>,
    sounds: HashMap<PathBuf, Tracked<SoundId>>,
    fonts: HashMap<PathBuf, FontId>,
    last_poll: Option<Instant>,
    reloaded: Vec<PathBuf>,
}

impl Assets {
    /// Intervalo entre verificações de arquivos alterados.
    pub const POLL_INTERVAL: Duration = Duration::from_millis(250);

    pub fn new(root: impl Into<PathBuf>, hot_reload: bool) -> Self {
        Self {
            root: root.into(),
            hot_reload,
            textures: HashMap::new(),
            sounds: HashMap::new(),
            fonts: HashMap::new(),
            last_poll: None,
            reloaded: Vec::new(),
        }
    }

    /// Escolhe a pasta de assets: `dir` relativo à pasta atual, ou ao lado do
    /// executável se não existir na pasta atual.
    pub fn locate(dir: &Path) -> PathBuf {
        if dir.is_absolute() || dir.exists() {
            return dir.to_path_buf();
        }
        if let Some(exe_dir) = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
        {
            let candidate = exe_dir.join(dir);
            if candidate.exists() {
                return candidate;
            }
        }
        dir.to_path_buf()
    }

    /// Pasta raiz dos assets.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Caminho completo de um asset.
    pub fn resolve(&self, path: impl AsRef<Path>) -> PathBuf {
        self.root.join(path)
    }

    pub fn hot_reload(&self) -> bool {
        self.hot_reload
    }

    pub fn set_hot_reload(&mut self, enabled: bool) {
        self.hot_reload = enabled;
    }

    /// Arquivos recarregados na última verificação (para avisos na tela, logs...).
    pub fn reloaded(&self) -> &[PathBuf] {
        &self.reloaded
    }

    /// Lê os bytes de um arquivo da pasta de assets.
    pub fn read(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, AssetError> {
        read(&self.resolve(path))
    }

    /// Lê um arquivo de texto (UTF-8) da pasta de assets.
    pub fn read_to_string(&self, path: impl AsRef<Path>) -> Result<String, AssetError> {
        let full = self.resolve(path);
        let bytes = read(&full)?;
        String::from_utf8(bytes).map_err(|e| AssetError::Io {
            path: full,
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
        })
    }

    fn due_for_poll(&mut self, now: Instant) -> bool {
        if !self.hot_reload {
            return false;
        }
        match self.last_poll {
            Some(last) if now - last < Self::POLL_INTERVAL => false,
            _ => {
                self.last_poll = Some(now);
                true
            }
        }
    }
}

fn read(path: &Path) -> Result<Vec<u8>, AssetError> {
    std::fs::read(path).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            AssetError::NotFound(path.to_owned())
        } else {
            AssetError::Io {
                path: path.to_owned(),
                source,
            }
        }
    })
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

impl Context {
    /// Carrega uma imagem (PNG) da pasta de assets. Chamadas repetidas com o
    /// mesmo caminho e opções devolvem a mesma textura.
    pub fn load_texture(
        &mut self,
        path: impl AsRef<Path>,
        options: TextureOptions,
    ) -> Result<TextureId, AssetError> {
        let full = self.assets.resolve(path);
        if let Some(t) = self.assets.textures.get(&(full.clone(), options)) {
            return Ok(t.id);
        }
        let bytes = read(&full)?;
        let id = self
            .renderer
            .load_texture_bytes(&bytes, options)
            .map_err(|source| AssetError::Render {
                path: full.clone(),
                source,
            })?;
        let modified = modified(&full);
        self.assets
            .textures
            .insert((full, options), Tracked { id, modified });
        Ok(id)
    }

    /// Carrega um som (WAV, OGG Vorbis ou MP3) da pasta de assets, com cache.
    pub fn load_sound(&mut self, path: impl AsRef<Path>) -> Result<SoundId, AssetError> {
        let full = self.assets.resolve(path);
        if let Some(t) = self.assets.sounds.get(&full) {
            return Ok(t.id);
        }
        let bytes = read(&full)?;
        let id = self
            .audio
            .load_bytes(bytes)
            .map_err(|source| AssetError::Audio {
                path: full.clone(),
                source,
            })?;
        let modified = modified(&full);
        self.assets.sounds.insert(full, Tracked { id, modified });
        Ok(id)
    }

    /// Carrega uma fonte (.ttf, .otf) da pasta de assets, com cache.
    pub fn load_font(&mut self, path: impl AsRef<Path>) -> Result<FontId, AssetError> {
        let full = self.assets.resolve(path);
        if let Some(id) = self.assets.fonts.get(&full) {
            return Ok(*id);
        }
        let bytes = read(&full)?;
        let id = self
            .renderer
            .load_font_bytes(&bytes)
            .map_err(|source| AssetError::Render {
                path: full.clone(),
                source,
            })?;
        self.assets.fonts.insert(full, id);
        Ok(id)
    }

    /// Verifica arquivos alterados e recarrega imagens e sons.
    /// A engine chama isto a cada frame; a verificação em si é espaçada.
    pub fn poll_assets(&mut self) {
        let now = Instant::now();
        if !self.assets.due_for_poll(now) {
            return;
        }
        self.assets.reloaded.clear();

        let textures: Vec<_> = self
            .assets
            .textures
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        for ((path, options), tracked) in textures {
            let current = modified(&path);
            if current.is_none() || current == tracked.modified {
                continue;
            }
            match read(&path).and_then(|bytes| {
                self.renderer
                    .replace_texture_bytes(tracked.id, &bytes, options)
                    .map_err(|source| AssetError::Render {
                        path: path.clone(),
                        source,
                    })
            }) {
                Ok(()) => log::info!("recarregado: {}", path.display()),
                // Arquivo pela metade (editor ainda salvando) ou inválido: tenta de novo depois.
                Err(e) => {
                    log::warn!("não foi possível recarregar: {e}");
                    continue;
                }
            }
            if let Some(t) = self.assets.textures.get_mut(&(path.clone(), options)) {
                t.modified = current;
            }
            self.assets.reloaded.push(path);
        }

        let sounds: Vec<_> = self
            .assets
            .sounds
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        for (path, tracked) in sounds {
            let current = modified(&path);
            if current.is_none() || current == tracked.modified {
                continue;
            }
            match read(&path).and_then(|bytes| {
                self.audio
                    .replace_bytes(tracked.id, bytes)
                    .map_err(|source| AssetError::Audio {
                        path: path.clone(),
                        source,
                    })
            }) {
                Ok(()) => log::info!("recarregado: {}", path.display()),
                Err(e) => {
                    log::warn!("não foi possível recarregar: {e}");
                    continue;
                }
            }
            if let Some(t) = self.assets.sounds.get_mut(&path) {
                t.modified = current;
            }
            self.assets.reloaded.push(path);
        }
    }
}
