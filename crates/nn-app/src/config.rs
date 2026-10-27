//! Configuração inicial da aplicação.

use std::path::PathBuf;
use std::sync::Arc;

use nn_core::Color;

/// Configuração inicial da aplicação.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Título da janela.
    pub title: String,
    /// Identificador do jogo, usado no nome da pasta onde o progresso é
    /// salvo. `None` usa o título (veja [`AppConfig::save_id`]).
    pub id: Option<String>,
    /// Começa em tela cheia.
    pub fullscreen: bool,
    /// Pasta onde gravar os dados salvos, no lugar da pasta de dados do
    /// usuário. Útil em testes.
    pub save_dir: Option<PathBuf>,
    /// Tamanho inicial da janela em pixels lógicos (respeita a escala do SO).
    pub size: (u32, u32),
    pub resizable: bool,
    /// Sincroniza com a taxa de atualização do monitor.
    pub vsync: bool,
    /// Limite de quadros por segundo: `None` = o automático (sem limite com
    /// [`AppConfig::vsync`], que já segue o monitor; 240 sem ele), `Some(0)`
    /// = sem limite (o laço roda o mais rápido que der e ocupa um núcleo).
    pub fps_max: Option<u32>,
    /// Passos de [`crate::Game::fixed_update`] por segundo.
    pub fixed_hz: u32,
    /// Cor de fundo inicial.
    pub clear_color: Color,
    /// Pasta de assets. Caminhos relativos são procurados primeiro a partir da
    /// pasta atual e depois ao lado do executável.
    pub assets_dir: PathBuf,
    /// Ferramentas de desenvolvimento no jogo (F3: a tela de desempenho).
    /// Ligado no jogo aberto da pasta do projeto; desligado no exportado.
    pub dev_tools: bool,
    /// Recarrega imagens, sons e scripts quando o arquivo é salvo.
    /// Ligado por padrão em builds de desenvolvimento (debug).
    pub hot_reload: bool,
    /// Origem dos assets. `None` usa a pasta [`AppConfig::assets_dir`] no disco;
    /// jogos exportados usam o pacote dentro do executável.
    pub asset_source: Option<Arc<dyn crate::AssetSource>>,
    /// Tamanho fixo do jogo (largura, altura): a câmera e a tela enxergam
    /// sempre este tamanho, ajustado à janela por [`AppConfig::scale_mode`].
    /// `None` usa o tamanho da janela.
    pub resolution: Option<(u32, u32)>,
    /// Como o tamanho fixo se ajusta à janela.
    pub scale_mode: nn_render::ScaleMode,
    /// Antisserrilhado no 3D (MSAA 4x).
    pub antialias: bool,
    /// Tonemap no 3D: a luz forte não estoura em branco.
    pub tonemap: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "nn-engine".into(),
            id: None,
            fullscreen: false,
            save_dir: None,
            size: (1280, 720),
            resizable: true,
            vsync: true,
            fps_max: None,
            fixed_hz: 60,
            clear_color: Color::hex(0x1E1E28),
            assets_dir: PathBuf::from("assets"),
            hot_reload: cfg!(debug_assertions),
            dev_tools: cfg!(debug_assertions),
            asset_source: None,
            resolution: None,
            scale_mode: nn_render::ScaleMode::Fit,
            antialias: false,
            tonemap: false,
        }
    }
}

impl AppConfig {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }

    /// Tamanho fixo do jogo, ajustado à janela (veja [`nn_render::ScaleMode`]).
    pub fn resolution(mut self, width: u32, height: u32) -> Self {
        self.resolution = Some((width.max(1), height.max(1)));
        self
    }

    pub fn scale_mode(mut self, mode: nn_render::ScaleMode) -> Self {
        self.scale_mode = mode;
        self
    }

    pub fn vsync(mut self, vsync: bool) -> Self {
        self.vsync = vsync;
        self
    }

    pub fn fps_max(mut self, fps: Option<u32>) -> Self {
        self.fps_max = fps;
        self
    }

    /// O intervalo mínimo entre dois quadros, se houver limite.
    pub fn frame_period(&self) -> Option<std::time::Duration> {
        let fps = match self.fps_max {
            Some(0) => return None,
            Some(f) => f,
            None if self.vsync => return None,
            None => 240,
        };
        Some(std::time::Duration::from_secs_f64(
            1.0 / f64::from(fps.max(1)),
        ))
    }

    pub fn fixed_hz(mut self, hz: u32) -> Self {
        self.fixed_hz = hz;
        self
    }

    pub fn clear_color(mut self, color: Color) -> Self {
        self.clear_color = color;
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn assets_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.assets_dir = dir.into();
        self
    }

    pub fn asset_source(mut self, source: Arc<dyn crate::AssetSource>) -> Self {
        self.asset_source = Some(source);
        self
    }

    pub fn hot_reload(mut self, enabled: bool) -> Self {
        self.hot_reload = enabled;
        self
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn fullscreen(mut self, fullscreen: bool) -> Self {
        self.fullscreen = fullscreen;
        self
    }

    pub fn save_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.save_dir = Some(dir.into());
        self
    }

    /// Identificador usado para a pasta de dados: o [`AppConfig::id`] ou,
    /// sem ele, o título em minúsculas e sem acentos.
    pub fn save_id(&self) -> String {
        crate::storage::slug(self.id.as_deref().unwrap_or(&self.title))
    }

    /// Dados salvos para esta configuração: em [`AppConfig::save_dir`] ou na
    /// pasta de dados do usuário.
    pub fn open_storage(&self) -> crate::Storage {
        match &self.save_dir {
            Some(dir) => crate::Storage::open(dir.join(crate::storage::SAVE_FILE)),
            None => crate::Storage::for_game(&self.save_id()),
        }
    }
}
