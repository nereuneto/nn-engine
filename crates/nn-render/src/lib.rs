//! Renderer 2D da nn-engine.
//!
//! - [`GpuContext`]: conexão com a GPU.
//! - [`Renderer`]: desenha [`Sprite`]s em lote, agrupando por textura.
//! - [`Camera2D`]: câmera ortográfica com Y para cima.
//! - [`OffscreenTarget`]: renderiza em memória (testes e capturas).

mod camera;
mod context;
mod offscreen;
mod renderer;
mod sprite;
mod texture;

pub use camera::Camera2D;
pub use context::GpuContext;
pub use offscreen::OffscreenTarget;
pub use renderer::{RenderStats, Renderer};
pub use sprite::{Rect, Sprite};
pub use texture::{Filter, TextureId, TextureOptions};

pub use wgpu;

/// Erros do renderer.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("nenhuma GPU compatível encontrada: {0}")]
    NoAdapter(String),
    #[error("falha ao abrir o dispositivo da GPU: {0}")]
    RequestDevice(String),
    #[error("falha ao ler pixels da GPU: {0}")]
    Readback(String),
    #[error("falha ao decodificar ou salvar imagem: {0}")]
    Image(#[from] image::ImageError),
    #[error("falha ao ler {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
}
