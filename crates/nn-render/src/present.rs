//! Tamanho fixo do jogo: o jogo é desenhado num tamanho escolhido (por
//! exemplo 640×360) e ajustado à janela, do jeito que [`ScaleMode`] manda.

use glam::{UVec2, Vec2};

/// Como o tamanho do jogo se ajusta à janela.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScaleMode {
    /// Aumenta ou diminui mantendo a proporção; sobra faixa preta nos lados
    /// (ou em cima e embaixo).
    #[default]
    Fit,
    /// Ocupa a janela toda, mesmo que distorça.
    Stretch,
    /// Só aumenta em números inteiros (2×, 3×...): pixel art nítida.
    Pixel,
}

impl ScaleMode {
    /// Pelo nome usado no `config.lua` ("fit", "stretch", "pixel").
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "fit" => Self::Fit,
            "stretch" => Self::Stretch,
            "pixel" => Self::Pixel,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Fit => "fit",
            Self::Stretch => "stretch",
            Self::Pixel => "pixel",
        }
    }
}

/// Onde o jogo aparece dentro da janela.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Presentation {
    /// Tamanho do jogo (o que a câmera e a tela enxergam).
    pub logical: UVec2,
    /// Canto superior esquerdo da área do jogo na janela, em pixels.
    pub origin: Vec2,
    /// Tamanho da área do jogo na janela, em pixels.
    pub size: Vec2,
}

impl Presentation {
    /// Ajusta `logical` dentro de uma janela de `window` pixels.
    pub fn new(window: UVec2, logical: UVec2, mode: ScaleMode) -> Self {
        let win = window.max(UVec2::ONE).as_vec2();
        let log = logical.max(UVec2::ONE).as_vec2();
        let ratio = win / log;
        let size = match mode {
            ScaleMode::Stretch => win,
            ScaleMode::Fit => log * ratio.x.min(ratio.y),
            ScaleMode::Pixel => {
                // Escala inteira; se a janela for menor que o jogo, encolhe
                // (melhor que cortar).
                let s = ratio.x.min(ratio.y);
                let s = if s >= 1.0 { s.floor() } else { s };
                log * s
            }
        };
        let origin = ((win - size) * 0.5).round();
        Self {
            logical: logical.max(UVec2::ONE),
            origin,
            size: size.round().max(Vec2::ONE),
        }
    }

    /// Pixels da janela por unidade do jogo, em cada eixo.
    pub fn scale(&self) -> Vec2 {
        self.size / self.logical.as_vec2()
    }

    /// Ponto da janela (pixels) → ponto do jogo.
    pub fn to_logical(&self, window_point: Vec2) -> Vec2 {
        (window_point - self.origin) / self.scale()
    }

    /// Ponto do jogo → ponto da janela (pixels).
    pub fn to_window(&self, logical_point: Vec2) -> Vec2 {
        self.origin + logical_point * self.scale()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ajustar_com_barras_esticar_e_inteiro() {
        let logical = UVec2::new(640, 360);
        // Janela mais larga: barras dos lados.
        let p = Presentation::new(UVec2::new(1600, 720), logical, ScaleMode::Fit);
        assert_eq!(p.size, Vec2::new(1280.0, 720.0));
        assert_eq!(p.origin, Vec2::new(160.0, 0.0));
        assert_eq!(p.to_logical(Vec2::new(160.0, 0.0)), Vec2::ZERO);
        assert_eq!(
            p.to_logical(Vec2::new(800.0, 360.0)),
            Vec2::new(320.0, 180.0)
        );
        // Esticar: a janela toda.
        let p = Presentation::new(UVec2::new(1600, 720), logical, ScaleMode::Stretch);
        assert_eq!(p.size, Vec2::new(1600.0, 720.0));
        assert_eq!(p.scale(), Vec2::new(2.5, 2.0));
        // Inteiro: 1000×700 cabe 1× (2× não cabe), centralizado.
        let p = Presentation::new(UVec2::new(1000, 700), logical, ScaleMode::Pixel);
        assert_eq!(p.size, Vec2::new(640.0, 360.0));
        assert_eq!(p.origin, Vec2::new(180.0, 170.0));
        let p = Presentation::new(UVec2::new(1920, 1080), logical, ScaleMode::Pixel);
        assert_eq!(p.scale(), Vec2::splat(3.0));
        // Janela menor que o jogo: encolhe em vez de cortar.
        let p = Presentation::new(UVec2::new(320, 180), logical, ScaleMode::Pixel);
        assert_eq!(p.size, Vec2::new(320.0, 180.0));
        assert_eq!(
            p.to_window(Vec2::new(640.0, 360.0)),
            Vec2::new(320.0, 180.0)
        );
    }

    #[test]
    fn nomes() {
        for m in [ScaleMode::Fit, ScaleMode::Stretch, ScaleMode::Pixel] {
            assert_eq!(ScaleMode::from_name(m.name()), Some(m));
        }
        assert_eq!(ScaleMode::from_name("x"), None);
    }
}
