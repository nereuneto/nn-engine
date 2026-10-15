//! Fundos em camadas (parallax): imagens repetidas que andam mais devagar
//! que o mundo, dando sensação de profundidade.

use glam::Vec2;
use nn_core::Color;

use crate::{Camera2D, Renderer, Sprite, TextureId};

/// Máximo de cópias por eixo (protege contra imagens minúsculas).
const MAX_COPIES: i64 = 64;

/// Uma camada de fundo.
///
/// `factor` diz quanto a camada acompanha a câmera: com 1.0 ela anda junto
/// com o mundo (como o chão); com 0.0 fica parada na tela (como um céu
/// infinitamente longe); 0.3 dá a impressão de montanhas distantes.
///
/// ```no_run
/// # use nn_render::{Camera2D, Parallax, Renderer, TextureId};
/// # use glam::Vec2;
/// # fn exemplo(renderer: &mut Renderer, montanhas: TextureId, camera: &Camera2D) {
/// let camada = Parallax::new(montanhas, Vec2::new(960.0, 360.0))
///     .factor(Vec2::new(0.3, 0.6))
///     .at(Vec2::new(0.0, 300.0));
/// // Depois de posicionar a câmera e antes do resto do mundo:
/// camada.draw(renderer, camera, Vec2::new(1280.0, 720.0));
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parallax {
    pub texture: TextureId,
    /// Tamanho de uma cópia da imagem no mundo.
    pub size: Vec2,
    /// Quanto acompanha a câmera em cada eixo (0 = parada na tela, 1 = junto com o mundo).
    pub factor: Vec2,
    /// Centro da cópia de referência quando a câmera está em (0, 0).
    pub origin: Vec2,
    /// Deslocamento extra, por exemplo `velocidade * tempo` para nuvens andando.
    pub scroll: Vec2,
    /// Repetir a imagem na horizontal e na vertical.
    pub repeat_x: bool,
    pub repeat_y: bool,
    pub color: Color,
}

impl Parallax {
    /// Camada que repete na horizontal, com fator 0.5.
    pub fn new(texture: TextureId, size: Vec2) -> Self {
        Self {
            texture,
            size,
            factor: Vec2::splat(0.5),
            origin: Vec2::ZERO,
            scroll: Vec2::ZERO,
            repeat_x: true,
            repeat_y: false,
            color: Color::WHITE,
        }
    }

    pub fn factor(mut self, factor: Vec2) -> Self {
        self.factor = factor;
        self
    }

    pub fn at(mut self, origin: Vec2) -> Self {
        self.origin = origin;
        self
    }

    pub fn scroll(mut self, scroll: Vec2) -> Self {
        self.scroll = scroll;
        self
    }

    pub fn repeat(mut self, x: bool, y: bool) -> Self {
        self.repeat_x = x;
        self.repeat_y = y;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Centros das cópias que aparecem na tela, para a câmera dada.
    pub fn positions(&self, camera: &Camera2D, viewport: Vec2) -> Vec<Vec2> {
        let size = self.size.abs();
        if size.x <= 0.0 || size.y <= 0.0 {
            return Vec::new();
        }
        let base = self.origin + camera.position * (Vec2::ONE - self.factor) + self.scroll;
        // Com rotação, a área visível é o círculo que envolve a tela.
        let half = if camera.rotation == 0.0 {
            camera.view_size(viewport) / 2.0
        } else {
            Vec2::splat(camera.view_size(viewport).length() / 2.0)
        };
        let (view_min, view_max) = (camera.position - half, camera.position + half);
        let range = |axis: usize, repeat: bool| -> (i64, i64) {
            if !repeat {
                return (0, 0);
            }
            let s = size[axis];
            let first = ((view_min[axis] - base[axis] - s / 2.0) / s).ceil() as i64;
            let last = ((view_max[axis] - base[axis] + s / 2.0) / s).floor() as i64;
            (first, last.min(first + MAX_COPIES - 1))
        };
        let (x0, x1) = range(0, self.repeat_x);
        let (y0, y1) = range(1, self.repeat_y);
        let mut out = Vec::new();
        for ky in y0..=y1 {
            for kx in x0..=x1 {
                out.push(base + Vec2::new(kx as f32, ky as f32) * size);
            }
        }
        out
    }

    /// Desenha as cópias visíveis no mundo. Chame depois de posicionar a câmera.
    pub fn draw(&self, renderer: &mut Renderer, camera: &Camera2D, viewport: Vec2) {
        for center in self.positions(camera, viewport) {
            renderer.draw(
                &Sprite::new(self.texture)
                    .at(center)
                    .size(self.size)
                    .color(self.color),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEW: Vec2 = Vec2::new(800.0, 600.0);

    fn camada() -> Parallax {
        Parallax::new(TextureId(0), Vec2::new(300.0, 200.0))
    }

    #[test]
    fn copias_cobrem_a_tela_inteira() {
        let p = camada();
        for x in [-1234.0, 0.0, 77.0, 5000.0] {
            let cam = Camera2D::new(Vec2::new(x, 0.0));
            let pos = p.positions(&cam, VIEW);
            let esquerda = pos.iter().map(|c| c.x - 150.0).fold(f32::MAX, f32::min);
            let direita = pos.iter().map(|c| c.x + 150.0).fold(f32::MIN, f32::max);
            assert!(
                esquerda <= x - 400.0 && direita >= x + 400.0,
                "x = {x}: {pos:?}"
            );
            assert!(pos.len() <= 5, "{}", pos.len());
            assert!(pos.iter().all(|c| c.y == 0.0));
        }
    }

    #[test]
    fn fator_controla_quanto_a_camada_anda_na_tela() {
        let p = camada().factor(Vec2::new(0.25, 1.0)).repeat(false, false);
        let a = Camera2D::new(Vec2::ZERO);
        let b = Camera2D::new(Vec2::new(400.0, 100.0));
        let tela = |cam: &Camera2D| cam.world_to_screen(p.positions(cam, VIEW)[0], VIEW);
        let andou = tela(&b) - tela(&a);
        // A câmera andou 400 para a direita: a camada anda 100 para a esquerda na tela.
        assert!((andou.x + 100.0).abs() < 1e-3, "{andou}");
        // No vertical, fator 1: anda junto com o mundo (100 para baixo na tela).
        assert!((andou.y - 100.0).abs() < 1e-3, "{andou}");

        let parada = camada().factor(Vec2::ZERO).repeat(false, false);
        let s = |cam: &Camera2D| cam.world_to_screen(parada.positions(cam, VIEW)[0], VIEW);
        assert!(s(&a).distance(s(&b)) < 1e-3);
    }

    #[test]
    fn repetir_nos_dois_eixos_e_rolagem() {
        let p = camada().repeat(true, true).scroll(Vec2::new(1000.0, 0.0));
        let pos = p.positions(&Camera2D::default(), VIEW);
        assert!(pos.len() >= 9);
        assert!(pos.iter().any(|c| c.y != 0.0));
        let sem = camada().repeat(true, true);
        // A rolagem desloca as cópias.
        assert_ne!(pos[0], sem.positions(&Camera2D::default(), VIEW)[0]);

        let minuscula = Parallax::new(TextureId(0), Vec2::splat(0.5)).repeat(true, true);
        assert!(minuscula.positions(&Camera2D::default(), VIEW).len() <= 64 * 64);
        let vazia = Parallax::new(TextureId(0), Vec2::ZERO);
        assert!(vazia.positions(&Camera2D::default(), VIEW).is_empty());
    }
}
