//! Assets do jogo: carregamento com cache e recarga quando o arquivo muda.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use web_time::Instant;

use nn_audio::{AudioError, SoundId};
use nn_render::{FontId, RenderError, TextureId, TextureOptions};

use crate::Context;
use crate::source::{AssetSource, DiskSource, normalize};

/// Erros ao carregar assets.
#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("asset não encontrado: {0}")]
    NotFound(PathBuf),
    #[error(
        "{0}: fica fora da pasta do jogo (use um caminho a partir dela, como \"imagens/heroi.png\"; o jogo exportado só leva o que está nela)"
    )]
    Outside(PathBuf),
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

/// Quem usa um asset carregado. Enquanto houver um vivo (ou o asset estiver
/// guardado com [`Context::keep_texture`] e afins), [`Context::unload_unused`]
/// não o descarrega. Guarde junto de quem usa o id (a imagem de um sprite, o
/// mapa com o tileset...).
#[derive(Debug, Clone)]
pub struct AssetHandle(#[allow(dead_code)] Rc<()>);

#[derive(Debug, Clone)]
struct Tracked<T> {
    id: T,
    modified: Option<SystemTime>,
    /// Os [`AssetHandle`] deste asset.
    users: Weak<()>,
    /// Pedido sem um [`AssetHandle`] (`load_texture` e afins): quem pediu
    /// pode guardar o id por quanto quiser, então nunca é descarregado.
    pinned: bool,
    /// Guardado de propósito (`nn.assets.keep`): nunca é descarregado.
    kept: bool,
}

impl<T> Tracked<T> {
    fn new(id: T, modified: Option<SystemTime>) -> Self {
        Self {
            id,
            modified,
            users: Weak::new(),
            pinned: false,
            kept: false,
        }
    }

    fn handle(&mut self) -> AssetHandle {
        match self.users.upgrade() {
            Some(rc) => AssetHandle(rc),
            None => {
                let rc = Rc::new(());
                self.users = Rc::downgrade(&rc);
                AssetHandle(rc)
            }
        }
    }

    fn unused(&self) -> bool {
        !self.pinned && !self.kept && self.users.strong_count() == 0
    }
}

/// O que [`Context::unload_unused`] descarregou.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Unloaded {
    pub textures: usize,
    pub sounds: usize,
    pub models: usize,
}

impl Unloaded {
    pub fn is_empty(&self) -> bool {
        self.textures + self.sounds + self.models == 0
    }
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
    models: HashMap<String, Tracked<nn_render::d3::Model>>,
    last_poll: Option<Instant>,
    reloaded: Vec<PathBuf>,
    /// Só caminhos dentro da pasta do jogo (veja [`Assets::set_confined`]).
    confined: bool,
    /// Os canvas criados, com quem ainda os usa (descarregam como as
    /// imagens).
    canvases: Vec<(TextureId, Weak<()>)>,
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
            canvases: Vec::new(),
            sounds: HashMap::new(),
            fonts: HashMap::new(),
            models: HashMap::new(),
            last_poll: None,
            reloaded: Vec::new(),
            confined: false,
        }
    }

    /// Só aceita caminhos dentro da pasta do jogo: um caminho absoluto ou que
    /// sai dela (`../`) dá [`AssetError::Outside`], igual no desenvolvimento e
    /// no jogo exportado (que só leva o que está na pasta). Os jogos em Lua
    /// ligam isto.
    pub fn set_confined(&mut self, confined: bool) {
        self.confined = confined;
    }

    fn outside(&self, path: &Path) -> bool {
        self.confined && !crate::source::inside_game(path)
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
        if self.outside(path) {
            return Err(AssetError::Outside(path.to_path_buf()));
        }
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
        !self.outside(path.as_ref()) && self.source.read(path.as_ref()).is_ok()
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
        let entry = self.model_entry(path, flat)?;
        entry.pinned = true;
        Ok(entry.id.clone())
    }

    /// Como [`Context::load_model`], com quem usa: sem nenhum
    /// [`AssetHandle`] vivo, [`Context::unload_unused`] descarrega o modelo.
    pub fn model_handle(
        &mut self,
        path: impl AsRef<Path>,
        flat: bool,
    ) -> Result<(nn_render::d3::Model, AssetHandle), AssetError> {
        let entry = self.model_entry(path, flat)?;
        Ok((entry.id.clone(), entry.handle()))
    }

    fn model_entry(
        &mut self,
        path: impl AsRef<Path>,
        flat: bool,
    ) -> Result<&mut Tracked<nn_render::d3::Model>, AssetError> {
        let key = model_key(&path, flat);
        if !self.assets.models.contains_key(&key) {
            let t = Instant::now();
            let file = normalize(path.as_ref());
            let model = self.read_model(&file, flat)?;
            self.perf.add_load_file(t.elapsed(), &file);
            let modified = self.assets.source.modified(Path::new(&file));
            self.assets
                .models
                .insert(key.clone(), Tracked::new(model, modified));
        }
        Ok(self.assets.models.get_mut(&key).expect("inserido acima"))
    }

    /// Um modelo já carregado, pela chave de [`model_key`].
    pub fn model(&self, key: &str) -> Option<&nn_render::d3::Model> {
        self.assets.models.get(key).map(|t| &t.id)
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
        match self.assets.models.get(key).map(|t| &t.id) {
            Some(model) => {
                self.renderer.draw_model(model, matrix, color, cast_shadow);
                true
            }
            None => false,
        }
    }

    /// Como [`Context::draw_model_key`], numa pose: as matrizes dos ossos de
    /// [`Skeleton::joint_matrices`](nn_render::d3::Skeleton::joint_matrices).
    pub fn draw_model_key_posed(
        &mut self,
        key: &str,
        matrix: glam::Mat4,
        color: nn_core::Color,
        cast_shadow: bool,
        joints: &[glam::Mat4],
    ) -> bool {
        match self.assets.models.get(key).map(|t| &t.id) {
            Some(model) => {
                self.renderer
                    .draw_model_posed(model, matrix, color, cast_shadow, joints);
                true
            }
            None => false,
        }
    }

    /// Como [`Context::draw_model_key`], com brilho próprio (veja
    /// [`Renderer::draw_model_glow`](nn_render::Renderer::draw_model_glow))
    /// e, num modelo animado, na pose `joints`.
    pub fn draw_model_key_glow(
        &mut self,
        key: &str,
        matrix: glam::Mat4,
        color: nn_core::Color,
        cast_shadow: bool,
        glow: nn_render::d3::Glow,
        joints: Option<&[glam::Mat4]>,
    ) -> bool {
        self.draw_model_key_material(
            key,
            matrix,
            color,
            cast_shadow,
            nn_render::d3::Material3D::glow(glow),
            joints,
        )
    }

    /// Como [`Context::draw_model_key`], com o material inteiro (brilho,
    /// água, balançar com o vento).
    pub fn draw_model_key_material(
        &mut self,
        key: &str,
        matrix: glam::Mat4,
        color: nn_core::Color,
        cast_shadow: bool,
        material: nn_render::d3::Material3D,
        joints: Option<&[glam::Mat4]>,
    ) -> bool {
        match self.assets.models.get(key).map(|t| &t.id) {
            Some(model) => {
                self.renderer.draw_model_material(
                    model,
                    matrix,
                    color,
                    cast_shadow,
                    material,
                    joints,
                );
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
        let entry = self.texture_entry(path, options)?;
        entry.pinned = true;
        Ok(entry.id)
    }

    /// Como [`Context::load_texture`], com quem usa: sem nenhum
    /// [`AssetHandle`] vivo, [`Context::unload_unused`] libera a textura.
    pub fn texture_handle(
        &mut self,
        path: impl AsRef<Path>,
        options: TextureOptions,
    ) -> Result<(TextureId, AssetHandle), AssetError> {
        let entry = self.texture_entry(path, options)?;
        Ok((entry.id, entry.handle()))
    }

    fn texture_entry(
        &mut self,
        path: impl AsRef<Path>,
        options: TextureOptions,
    ) -> Result<&mut Tracked<TextureId>, AssetError> {
        let key = (normalize(path.as_ref()), options);
        if !self.assets.textures.contains_key(&key) {
            let t = Instant::now();
            let bytes = self.assets.read(&key.0)?;
            let id = self
                .renderer
                .load_texture_bytes(&bytes, options)
                .map_err(|source| AssetError::Render {
                    path: self.assets.resolve(&key.0),
                    source,
                })?;
            let modified = self.assets.source.modified(Path::new(&key.0));
            self.assets
                .textures
                .insert(key.clone(), Tracked::new(id, modified));
            self.perf.add_load_file(t.elapsed(), &key.0);
        }
        Ok(self.assets.textures.get_mut(&key).expect("inserido acima"))
    }

    /// Carrega um som (WAV, OGG Vorbis ou MP3) dos assets, com cache.
    pub fn load_sound(&mut self, path: impl AsRef<Path>) -> Result<SoundId, AssetError> {
        let entry = self.sound_entry(path)?;
        entry.pinned = true;
        Ok(entry.id)
    }

    /// Como [`Context::load_sound`], com quem usa: sem nenhum
    /// [`AssetHandle`] vivo, [`Context::unload_unused`] libera o som.
    pub fn sound_handle(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<(SoundId, AssetHandle), AssetError> {
        let entry = self.sound_entry(path)?;
        Ok((entry.id, entry.handle()))
    }

    fn sound_entry(&mut self, path: impl AsRef<Path>) -> Result<&mut Tracked<SoundId>, AssetError> {
        let key = normalize(path.as_ref());
        if !self.assets.sounds.contains_key(&key) {
            let t = Instant::now();
            let bytes = self.assets.read(&key)?;
            let id = self
                .audio
                .load_bytes(bytes)
                .map_err(|source| AssetError::Audio {
                    path: self.assets.resolve(&key),
                    source,
                })?;
            let modified = self.assets.source.modified(Path::new(&key));
            self.assets
                .sounds
                .insert(key.clone(), Tracked::new(id, modified));
            self.perf.add_load_file(t.elapsed(), &key);
        }
        Ok(self.assets.sounds.get_mut(&key).expect("inserido acima"))
    }

    /// Guarda a textura para sempre (nunca descarregada). `false` se ela não
    /// veio dos assets.
    pub fn keep_texture(&mut self, id: TextureId) -> bool {
        let mut found = false;
        for t in self.assets.textures.values_mut().filter(|t| t.id == id) {
            t.kept = true;
            found = true;
        }
        found
    }

    /// Guarda o som para sempre (a música que toca em todas as cenas).
    pub fn keep_sound(&mut self, id: SoundId) -> bool {
        let mut found = false;
        for t in self.assets.sounds.values_mut().filter(|t| t.id == id) {
            t.kept = true;
            found = true;
        }
        found
    }

    /// Guarda o modelo (pela chave de [`model_key`]) para sempre.
    pub fn keep_model(&mut self, key: &str) -> bool {
        match self.assets.models.get_mut(key) {
            Some(t) => {
                t.kept = true;
                true
            }
            None => false,
        }
    }

    /// Um canvas de `size` pixels (veja [`nn_render::Renderer::create_canvas`]).
    /// Some da memória quando o último [`AssetHandle`] dele cai, na próxima
    /// troca de cena, como as imagens.
    pub fn create_canvas(
        &mut self,
        size: glam::UVec2,
        options: TextureOptions,
    ) -> (TextureId, AssetHandle) {
        let id = self.renderer.create_canvas(size, options);
        let rc = Rc::new(());
        self.assets.canvases.push((id, Rc::downgrade(&rc)));
        (id, AssetHandle(rc))
    }

    /// Descarrega as texturas, os sons e os modelos que ninguém usa mais:
    /// pedidos só com um [`AssetHandle`], sem nenhum vivo, e não guardados.
    /// Pedir de novo carrega outra vez. A engine chama isto ao trocar de
    /// cena (depois de o Lua soltar o que a cena anterior usava).
    pub fn unload_unused(&mut self) -> Unloaded {
        let mut out = Unloaded::default();
        let textures: Vec<_> = self
            .assets
            .textures
            .iter()
            .filter(|(_, t)| t.unused())
            .map(|(k, t)| (k.clone(), t.id))
            .collect();
        for (key, id) in textures {
            self.renderer.free_texture(id);
            self.assets.textures.remove(&key);
            out.textures += 1;
        }
        let renderer = &mut self.renderer;
        self.assets.canvases.retain(|(id, users)| {
            if users.strong_count() > 0 {
                return true;
            }
            renderer.free_texture(*id);
            out.textures += 1;
            false
        });
        let sounds: Vec<_> = self
            .assets
            .sounds
            .iter()
            .filter(|(_, t)| t.unused())
            .map(|(k, t)| (k.clone(), t.id))
            .collect();
        for (key, id) in sounds {
            self.audio.free(id);
            self.assets.sounds.remove(&key);
            out.sounds += 1;
        }
        let models: Vec<String> = self
            .assets
            .models
            .iter()
            .filter(|(_, t)| t.unused())
            .map(|(k, _)| k.clone())
            .collect();
        for key in models {
            if let Some(t) = self.assets.models.remove(&key) {
                self.renderer.free_model(&t.id);
                out.models += 1;
            }
        }
        out
    }

    /// Carrega uma fonte (.ttf, .otf) dos assets, com cache.
    pub fn load_font(&mut self, path: impl AsRef<Path>) -> Result<FontId, AssetError> {
        let key = normalize(path.as_ref());
        // A fonte de pixel art embutida.
        if key == "pixel" {
            return Ok(self.renderer.pixel_font());
        }
        if let Some(id) = self.assets.fonts.get(&key) {
            return Ok(*id);
        }
        let t = Instant::now();
        let bytes = self.assets.read(&key)?;
        let id = self
            .renderer
            .load_font_bytes(&bytes)
            .map_err(|source| AssetError::Render {
                path: self.assets.resolve(&key),
                source,
            })?;
        self.perf.add_load_file(t.elapsed(), &key);
        self.assets.fonts.insert(key, id);
        Ok(id)
    }

    /// Carrega uma fonte de pixel art: `em` é o tamanho em que cada pixel do
    /// desenho dela vira um pixel da tela (o texto fica em múltiplos dele).
    pub fn load_pixel_font(
        &mut self,
        path: impl AsRef<Path>,
        em: f32,
    ) -> Result<FontId, AssetError> {
        let key = normalize(path.as_ref());
        let cache = format!("{key}#pixel{em}");
        if let Some(id) = self.assets.fonts.get(&cache) {
            return Ok(*id);
        }
        let t = Instant::now();
        let bytes = self.assets.read(&key)?;
        let id = self
            .renderer
            .load_pixel_font_bytes(&bytes, em)
            .map_err(|source| AssetError::Render {
                path: self.assets.resolve(&key),
                source,
            })?;
        self.perf.add_load_file(t.elapsed(), &key);
        self.assets.fonts.insert(cache, id);
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
            .map(|(k, v)| (k.clone(), v.clone()))
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
            .map(|(k, v)| (k.clone(), v.clone()))
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
            .map(|(k, t)| (k.clone(), t.modified))
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
                    if let Some(t) = self.assets.models.get_mut(&key) {
                        // Quem desenha pela chave passa a desenhar o novo. O
                        // velho sai da GPU, a não ser que alguém tenha pedido
                        // o modelo sem um AssetHandle (e guardado um clone).
                        let old = std::mem::replace(&mut t.id, model);
                        t.modified = current;
                        if !t.pinned {
                            self.renderer.free_model(&old);
                        }
                    }
                    self.assets.reloaded.push(PathBuf::from(file));
                }
                Err(e) => log::warn!("não foi possível recarregar: {e}"),
            }
        }
    }
}
