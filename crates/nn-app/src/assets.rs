//! Assets do jogo: carregamento com cache e recarga quando o arquivo muda.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use nn_audio::{AudioError, SoundId};
use nn_render::{FontId, RenderError, TextureId, TextureOptions};

use crate::Context;
use crate::source::{AssetSource, DiskSource, normalize};

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
    #[error("{path}: {source}")]
    Model {
        path: PathBuf,
        source: nn_render::d3::ModelError,
    },
}

#[derive(Debug, Clone, Copy)]
struct Tracked<T> {
    id: T,
    modified: Option<SystemTime>,
}

/// Assets carregados a partir da pasta do jogo (ou do pacote, num jogo exportado).
///
/// Cada arquivo é carregado uma única vez: pedir o mesmo caminho de novo
/// devolve o mesmo id. Com a recarga ligada, imagens e sons alterados no
/// disco são recarregados sozinhos, mantendo o id.
#[derive(Debug)]
pub struct Assets {
    source: Arc<dyn AssetSource>,
    hot_reload: bool,
    textures: HashMap<(String, TextureOptions), Tracked<TextureId>>,
    sounds: HashMap<String, Tracked<SoundId>>,
    fonts: HashMap<String, FontId>,
    /// Modelos 3D, pela chave de [`model_key`].
    models: HashMap<String, (nn_render::d3::Model, Option<SystemTime>)>,
    last_poll: Option<Instant>,
    reloaded: Vec<PathBuf>,
}

impl Assets {
    /// Intervalo entre verificações de arquivos alterados.
    pub const POLL_INTERVAL: Duration = Duration::from_millis(250);

    /// Assets lidos de `source`. Origens empacotadas nunca recarregam.
    pub fn new(source: Arc<dyn AssetSource>, hot_reload: bool) -> Self {
        let hot_reload = hot_reload && !source.is_packed();
        Self {
            source,
            hot_reload,
            textures: HashMap::new(),
            sounds: HashMap::new(),
            fonts: HashMap::new(),
            models: HashMap::new(),
            last_poll: None,
            reloaded: Vec::new(),
        }
    }

    /// Assets lidos de uma pasta do disco.
    pub fn from_dir(root: impl Into<PathBuf>, hot_reload: bool) -> Self {
        Self::new(Arc::new(DiskSource::new(root)), hot_reload)
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

    /// Origem dos arquivos (para quem precisa ler da mesma forma, como os scripts).
    pub fn source(&self) -> Arc<dyn AssetSource> {
        self.source.clone()
    }

    /// Onde estão os assets (para mensagens).
    pub fn root(&self) -> PathBuf {
        self.source.describe(Path::new(""))
    }

    /// Nome completo de um asset, para mensagens.
    pub fn resolve(&self, path: impl AsRef<Path>) -> PathBuf {
        self.source.describe(path.as_ref())
    }

    pub fn hot_reload(&self) -> bool {
        self.hot_reload
    }

    pub fn set_hot_reload(&mut self, enabled: bool) {
        self.hot_reload = enabled && !self.source.is_packed();
    }

    /// Arquivos recarregados na última verificação (para avisos na tela, logs...).
    pub fn reloaded(&self) -> &[PathBuf] {
        &self.reloaded
    }

    /// Lê os bytes de um arquivo dos assets.
    pub fn read(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, AssetError> {
        let path = path.as_ref();
        self.source.read(path).map_err(|source| {
            let path = self.source.describe(path);
            if source.kind() == std::io::ErrorKind::NotFound {
                AssetError::NotFound(path)
            } else {
                AssetError::Io { path, source }
            }
        })
    }

    /// O arquivo existe nos assets?
    pub fn exists(&self, path: impl AsRef<Path>) -> bool {
        self.source.read(path.as_ref()).is_ok()
    }

    /// Lê um arquivo de texto (UTF-8) dos assets.
    pub fn read_to_string(&self, path: impl AsRef<Path>) -> Result<String, AssetError> {
        let path = path.as_ref();
        let bytes = self.read(path)?;
        String::from_utf8(bytes).map_err(|e| AssetError::Io {
            path: self.source.describe(path),
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

/// Chave de um modelo no cache: o caminho e se ele foi achatado.
pub fn model_key(path: impl AsRef<Path>, flat: bool) -> String {
    let key = normalize(path.as_ref());
    if flat { format!("{key}|flat") } else { key }
}

impl Context {
    /// Carrega um modelo 3D (`.glb` ou `.gltf`) dos assets, com cache. Com
    /// `flat`, cada face ganha um tom só (o visual low-poly), mesmo que o
    /// arquivo tenha sombreado suave. Com a recarga ligada, o modelo é lido
    /// de novo quando o arquivo muda: busque-o com [`Context::model`] a cada
    /// desenho para ver a mudança.
    pub fn load_model(
        &mut self,
        path: impl AsRef<Path>,
        flat: bool,
    ) -> Result<nn_render::d3::Model, AssetError> {
        let key = model_key(&path, flat);
        if let Some((m, _)) = self.assets.models.get(&key) {
            return Ok(m.clone());
        }
        let file = normalize(path.as_ref());
        let model = self.read_model(&file, flat)?;
        let modified = self.assets.source.modified(Path::new(&file));
        self.assets.models.insert(key, (model.clone(), modified));
        Ok(model)
    }

    /// Um modelo já carregado, pela chave de [`model_key`].
    pub fn model(&self, key: &str) -> Option<&nn_render::d3::Model> {
        self.assets.models.get(key).map(|(m, _)| m)
    }

    /// Desenha o modelo carregado com a chave `key` (de [`model_key`]), na
    /// versão mais nova do arquivo. `false` se ele não estiver carregado.
    pub fn draw_model_key(
        &mut self,
        key: &str,
        matrix: glam::Mat4,
        color: nn_core::Color,
        cast_shadow: bool,
    ) -> bool {
        match self.assets.models.get(key) {
            Some((model, _)) => {
                self.renderer.draw_model(model, matrix, color, cast_shadow);
                true
            }
            None => false,
        }
    }

    fn read_model(&mut self, file: &str, flat: bool) -> Result<nn_render::d3::Model, AssetError> {
        let bytes = self.assets.read(file)?;
        let dir = Path::new(file)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let assets = &self.assets;
        let mut data = nn_render::d3::ModelData::from_gltf(&bytes, &mut |uri| {
            assets.read(normalize(&dir.join(uri))).ok()
        })
        .map_err(|source| AssetError::Model {
            path: self.assets.resolve(file),
            source,
        })?;
        if flat {
            data.flatten();
        }
        Ok(self.renderer.create_model(&data))
    }

    /// Carrega uma imagem (PNG) dos assets. Chamadas repetidas com o mesmo
    /// caminho e opções devolvem a mesma textura.
    pub fn load_texture(
        &mut self,
        path: impl AsRef<Path>,
        options: TextureOptions,
    ) -> Result<TextureId, AssetError> {
        let key = normalize(path.as_ref());
        if let Some(t) = self.assets.textures.get(&(key.clone(), options)) {
            return Ok(t.id);
        }
        let bytes = self.assets.read(&key)?;
        let id = self
            .renderer
            .load_texture_bytes(&bytes, options)
            .map_err(|source| AssetError::Render {
                path: self.assets.resolve(&key),
                source,
            })?;
        let modified = self.assets.source.modified(Path::new(&key));
        self.assets
            .textures
            .insert((key, options), Tracked { id, modified });
        Ok(id)
    }

    /// Carrega um som (WAV, OGG Vorbis ou MP3) dos assets, com cache.
    pub fn load_sound(&mut self, path: impl AsRef<Path>) -> Result<SoundId, AssetError> {
        let key = normalize(path.as_ref());
        if let Some(t) = self.assets.sounds.get(&key) {
            return Ok(t.id);
        }
        let bytes = self.assets.read(&key)?;
        let id = self
            .audio
            .load_bytes(bytes)
            .map_err(|source| AssetError::Audio {
                path: self.assets.resolve(&key),
                source,
            })?;
        let modified = self.assets.source.modified(Path::new(&key));
        self.assets.sounds.insert(key, Tracked { id, modified });
        Ok(id)
    }

    /// Carrega uma fonte (.ttf, .otf) dos assets, com cache.
    pub fn load_font(&mut self, path: impl AsRef<Path>) -> Result<FontId, AssetError> {
        let key = normalize(path.as_ref());
        if let Some(id) = self.assets.fonts.get(&key) {
            return Ok(*id);
        }
        let bytes = self.assets.read(&key)?;
        let id = self
            .renderer
            .load_font_bytes(&bytes)
            .map_err(|source| AssetError::Render {
                path: self.assets.resolve(&key),
                source,
            })?;
        self.assets.fonts.insert(key, id);
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
        let source = self.assets.source.clone();

        let textures: Vec<_> = self
            .assets
            .textures
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        for ((path, options), tracked) in textures {
            let current = source.modified(Path::new(&path));
            if current.is_none() || current == tracked.modified {
                continue;
            }
            let result = self.assets.read(&path).and_then(|bytes| {
                self.renderer
                    .replace_texture_bytes(tracked.id, &bytes, options)
                    .map_err(|e| AssetError::Render {
                        path: source.describe(Path::new(&path)),
                        source: e,
                    })
            });
            match result {
                Ok(()) => log::info!("recarregado: {path}"),
                // Arquivo pela metade (editor ainda salvando) ou inválido: tenta de novo depois.
                Err(e) => {
                    log::warn!("não foi possível recarregar: {e}");
                    continue;
                }
            }
            if let Some(t) = self.assets.textures.get_mut(&(path.clone(), options)) {
                t.modified = current;
            }
            self.assets.reloaded.push(PathBuf::from(path));
        }

        let sounds: Vec<_> = self
            .assets
            .sounds
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        for (path, tracked) in sounds {
            let current = source.modified(Path::new(&path));
            if current.is_none() || current == tracked.modified {
                continue;
            }
            let result = self.assets.read(&path).and_then(|bytes| {
                self.audio
                    .replace_bytes(tracked.id, bytes)
                    .map_err(|e| AssetError::Audio {
                        path: source.describe(Path::new(&path)),
                        source: e,
                    })
            });
            match result {
                Ok(()) => log::info!("recarregado: {path}"),
                Err(e) => {
                    log::warn!("não foi possível recarregar: {e}");
                    continue;
                }
            }
            if let Some(t) = self.assets.sounds.get_mut(&path) {
                t.modified = current;
            }
            self.assets.reloaded.push(PathBuf::from(path));
        }

        let models: Vec<_> = self
            .assets
            .models
            .iter()
            .map(|(k, (_, m))| (k.clone(), *m))
            .collect();
        for (key, modified) in models {
            let (file, flat) = match key.strip_suffix("|flat") {
                Some(f) => (f.to_string(), true),
                None => (key.clone(), false),
            };
            let current = source.modified(Path::new(&file));
            if current.is_none() || current == modified {
                continue;
            }
            match self.read_model(&file, flat) {
                Ok(model) => {
                    log::info!("recarregado: {file}");
                    self.assets.models.insert(key, (model, current));
                    self.assets.reloaded.push(PathBuf::from(file));
                }
                Err(e) => log::warn!("não foi possível recarregar: {e}"),
            }
        }
    }
}
