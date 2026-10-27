//! Renderer da nn-engine: 2D e o mundo 3D ([`d3`]).
//!
//! - [`GpuContext`]: conexão com a GPU.
//! - [`Renderer`]: desenha [`Sprite`]s e [`Text`]os em lote, agrupando por
//!   textura, no espaço do mundo ou da tela (HUD).
//! - [`SpriteSheet`], [`Animation`] e [`AnimationPlayer`]: animação quadro a quadro.
//! - [`ParticleEmitter`]: partículas (poeira, faíscas, explosões).
//! - [`Camera2D`]: câmera ortográfica com Y para cima.
//! - [`OffscreenTarget`]: renderiza em memória (testes e capturas).
//! - [`Presentation`]: tamanho fixo do jogo ajustado à janela.
//! - [`Panel`]: painéis de menus (cantos arredondados, borda, 9 fatias).
//! - [`d3`]: malhas 3D com luz, câmera em perspectiva e formas prontas.

mod animation;
mod camera;
mod context;
pub mod d3;
mod offscreen;
mod panel;
mod parallax;
mod particles;
mod present;
mod renderer;
mod shapes2d;
mod sprite;
mod text;
mod texture;

pub use animation::{Animation, AnimationPlayer, Playback, SpriteSheet};
pub use camera::{Bounds, Camera2D, Follow};
pub use context::GpuContext;
pub use offscreen::{OffscreenTarget, read_texture_rgba};
pub use panel::{Panel, PanelImage};
pub use parallax::Parallax;
pub use particles::{EmitArea, ParticleConfig, ParticleEmitter};
pub use present::{Presentation, ScaleMode};
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
