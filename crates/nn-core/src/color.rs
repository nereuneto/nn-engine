//! Cor RGBA em ponto flutuante.

use bytemuck::{Pod, Zeroable};

/// Cor RGBA com componentes de 0.0 a 1.0 no espaço sRGB (o mesmo de editores
/// de imagem e de códigos hexadecimais). O renderer converte para linear.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    /// Cria a partir de componentes de 0 a 255.
    pub fn rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::rgba(
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            f32::from(a) / 255.0,
        )
    }

    /// Cria a partir de um código hexadecimal: `0xRRGGBB`.
    pub fn hex(rgb: u32) -> Self {
        Self::rgba8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255)
    }

    /// Mesma cor com outra opacidade.
    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    /// Converte para o espaço linear, usado nos cálculos de iluminação e mistura.
    pub fn to_linear(self) -> Self {
        fn conv(c: f32) -> f32 {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        Self::rgba(conv(self.r), conv(self.g), conv(self.b), self.a)
    }

    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

impl From<[f32; 4]> for Color {
    fn from([r, g, b, a]: [f32; 4]) -> Self {
        Self::rgba(r, g, b, a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_decodifica_componentes() {
        let c = Color::hex(0xFF8000);
        assert_eq!(c, Color::rgba8(255, 128, 0, 255));
        assert!((c.g - 128.0 / 255.0).abs() < 1e-6);
    }

    #[test]
    fn linear_preserva_extremos() {
        assert_eq!(Color::WHITE.to_linear(), Color::WHITE);
        assert_eq!(Color::BLACK.to_linear(), Color::BLACK);
        let meio = Color::rgb(0.5, 0.5, 0.5).to_linear();
        assert!((meio.r - 0.214).abs() < 0.001);
    }
}
