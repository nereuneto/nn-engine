//! Configuração inicial da aplicação.

use std::path::PathBuf;
use std::sync::Arc;

use nn_core::Color;

/// Configuração inicial da aplicação.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Título da janela.
    pub title: String,
    /// Tamanho inicial da janela em pixels lógicos (respeita a escala do SO).
    pub size: (u32, u32),
    pub resizable: bool,
    /// Sincroniza com a taxa de atualização do monitor.
    pub vsync: bool,
    /// Passos de [`crate::Game::fixed_update`] por segundo.
    pub fixed_hz: u32,
    /// Cor de fundo inicial.
    pub clear_color: Color,
    /// Pasta de assets. Caminhos relativos são procurados primeiro a partir da
    /// pasta atual e depois ao lado do executável.
    pub assets_dir: PathBuf,
    /// Recarrega imagens, sons e scripts quando o arquivo é salvo.
    /// Ligado por padrão em builds de desenvolvimento (debug).
    pub hot_reload: bool,
    /// Origem dos assets. `None` usa a pasta [`AppConfig::assets_dir`] no disco;
    /// jogos exportados usam o pacote dentro do executável.
    pub asset_source: Option<Arc<dyn crate::AssetSource>>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "nn-engine".into(),
            size: (1280, 720),
            resizable: true,
            vsync: true,
            fixed_hz: 60,
            clear_color: Color::hex(0x1E1E28),
            assets_dir: PathBuf::from("assets"),
            hot_reload: cfg!(debug_assertions),
            asset_source: None,
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

    pub fn vsync(mut self, vsync: bool) -> Self {
        self.vsync = vsync;
        self
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
}
