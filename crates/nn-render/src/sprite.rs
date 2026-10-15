//! Descrição de um sprite a ser desenhado.

use glam::Vec2;
use nn_core::Color;

use crate::TextureId;

/// Retângulo em coordenadas de textura normalizadas (0.0 a 1.0),
/// com origem no canto superior esquerdo da imagem.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    /// A textura inteira.
    pub const FULL: Self = Self {
        x: 0.0,
        y: 0.0,
        w: 1.0,
        h: 1.0,
    };

    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// Converte um retângulo em pixels da imagem (útil para spritesheets).
    pub fn from_pixels(x: u32, y: u32, w: u32, h: u32, texture_size: glam::UVec2) -> Self {
        let ts = texture_size.as_vec2().max(Vec2::ONE);
        Self::new(
            x as f32 / ts.x,
            y as f32 / ts.y,
            w as f32 / ts.x,
            h as f32 / ts.y,
        )
    }

    pub fn to_array(self) -> [f32; 4] {
        [self.x, self.y, self.w, self.h]
    }
}

/// Um retângulo texturizado no mundo.
///
/// ```
/// # use nn_render::{Sprite, TextureId};
/// # use nn_core::Color;
/// # use glam::Vec2;
/// # fn exemplo(textura: TextureId) {
/// let sprite = Sprite::new(textura)
///     .at(Vec2::new(10.0, 20.0))
///     .size(Vec2::splat(64.0))
///     .color(Color::hex(0xFFCC00));
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sprite {
    pub texture: TextureId,
    /// Posição do ponto de origem no mundo.
    pub position: Vec2,
    /// Tamanho em unidades do mundo. `None` usa o tamanho em pixels da região da textura.
    pub size: Option<Vec2>,
    /// Ponto de origem relativo ao sprite: (0,0) é o canto inferior esquerdo,
    /// (0.5,0.5) o centro. A rotação acontece em torno desse ponto.
    pub origin: Vec2,
    /// Rotação em radianos (anti-horária).
    pub rotation: f32,
    /// Cor multiplicada pela textura (tingimento e transparência).
    pub color: Color,
    /// Região da textura usada.
    pub region: Rect,
}

impl Sprite {
    pub fn new(texture: TextureId) -> Self {
        Self {
            texture,
            position: Vec2::ZERO,
            size: None,
            origin: Vec2::splat(0.5),
            rotation: 0.0,
            color: Color::WHITE,
            region: Rect::FULL,
        }
    }

    pub fn at(mut self, position: Vec2) -> Self {
        self.position = position;
        self
    }

    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }

    pub fn origin(mut self, origin: Vec2) -> Self {
        self.origin = origin;
        self
    }

    pub fn rotation(mut self, radians: f32) -> Self {
        self.rotation = radians;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn region(mut self, region: Rect) -> Self {
        self.region = region;
        self
    }
}
