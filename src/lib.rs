//! # nn-engine
//!
//! Engine de jogos 2D em Rust.
//!
//! ```no_run
//! use nn_engine::prelude::*;
//!
//! struct MeuJogo;
//!
//! impl Game for MeuJogo {
//!     fn render(&mut self, ctx: &mut Context) {
//!         ctx.renderer.draw_rect(Vec2::ZERO, Vec2::splat(100.0), Color::hex(0x3FA7D6));
//!     }
//! }
//!
//! fn main() -> Result<(), AppError> {
//!     nn_engine::run(AppConfig::new("Meu jogo"), |_ctx| MeuJogo)
//! }
//! ```
//!
//! Os módulos também podem ser usados separadamente: [`core`], [`input`],
//! [`render`], [`audio`], [`physics`], [`tilemap`], [`script`] (Lua) e [`app`].

pub use nn_app as app;
pub use nn_audio as audio;
pub use nn_core as core;
pub use nn_input as input;
pub use nn_physics as physics;
pub use nn_render as render;
pub use nn_script as script;
pub use nn_tilemap as tilemap;

pub use glam as math;

pub use nn_app::{AppConfig, AppError, Context, Game, run};

/// Importa de uma vez os tipos mais usados.
pub mod prelude {
    pub use glam::{UVec2, Vec2};
    pub use nn_app::{
        AppConfig, AppError, Context, Fade, Game, Panel, PanelOptions, Scene, SceneStack, Storage,
        Theme, Transition,
    };
    pub use nn_audio::{Audio, Channel, PlayOptions, Sound, SoundId};
    pub use nn_core::{Color, Time};
    pub use nn_input::{AxisDir, Binding, Device, Input, KeyCode, MouseButton, PadAxis, PadButton};
    pub use nn_render::{
        Align, Anchor, Animation, AnimationPlayer, Blend, Bounds, Camera2D, EmitArea, Filter,
        Follow, FontId, Parallax, ParticleConfig, ParticleEmitter, Playback, Rect, Sprite,
        SpriteSheet, Text, TextureId, TextureOptions,
    };
}

/// Garante que os exemplos do README continuam compilando.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
