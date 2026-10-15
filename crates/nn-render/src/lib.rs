//! Renderer 2D da nn-engine.
//!
//! - [`GpuContext`]: conexão com a GPU.
//! - [`Renderer`]: desenha [`Sprite`]s e [`Text`]os em lote, agrupando por
//!   textura, no espaço do mundo ou da tela (HUD).
//! - [`SpriteSheet`], [`Animation`] e [`AnimationPlayer`]: animação quadro a quadro.
//! - [`ParticleEmitter`]: partículas (poeira, faíscas, explosões).
//! - [`Camera2D`]: câmera ortográfica com Y para cima.
//! - [`OffscreenTarget`]: renderiza em memória (testes e capturas).

mod animation;
mod camera;
mod context;
mod offscreen;
mod parallax;
mod particles;
mod renderer;
mod sprite;
mod text;
mod texture;

pub use animation::{Animation, AnimationPlayer, Playback, SpriteSheet};
pub use camera::{Bounds, Camera2D, Follow};
pub use context::GpuContext;
pub use offscreen::OffscreenTarget;
pub use parallax::Parallax;
pub use particles::{EmitArea, ParticleConfig, ParticleEmitter};
pub use renderer::{RenderStats, Renderer, ScreenPainter};
pub use sprite::{Blend, Rect, Sprite};
pub use text::{Align, Anchor, FontId, Text};
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
    #[error("fonte inválida: {0}")]
    Font(String),
    #[error("falha ao decodificar ou salvar imagem: {0}")]
    Image(#[from] image::ImageError),
    #[error("falha ao ler {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
}
