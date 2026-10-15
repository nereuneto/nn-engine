//! Efeitos de tela: tremor da câmera, flash e o escurecimento das
//! transições de cena.

use glam::Vec2;
use nn_core::Color;
use nn_render::Camera2D;

use crate::Context;

#[derive(Debug, Clone, Copy)]
struct Shake {
    strength: f32,
    duration: f32,
    elapsed: f32,
}

#[derive(Debug, Clone, Copy)]
struct Flash {
    color: Color,
    duration: f32,
    elapsed: f32,
}

/// Estado dos efeitos de tela, guardado no [`Context`].
#[derive(Debug)]
pub(crate) struct Effects {
    shakes: Vec<Shake>,
    /// Relógio do tremor (só anda enquanto há tremor).
    clock: f32,
    flash: Option<Flash>,
    /// Cor por cima de tudo (transições de cena). Alfa zero = nada.
    pub(crate) fade: Color,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            shakes: Vec::new(),
            clock: 0.0,
            flash: None,
            // A cor padrão é branca opaca: aqui precisa ser "nada".
            fade: Color::TRANSPARENT,
        }
    }
}

impl Effects {
    pub(crate) fn advance(&mut self, dt: f32) {
        for s in &mut self.shakes {
            s.elapsed += dt;
        }
        self.shakes.retain(|s| s.elapsed < s.duration);
        if self.shakes.is_empty() {
            self.clock = 0.0;
        } else {
            self.clock += dt;
        }
        if let Some(f) = &mut self.flash {
            f.elapsed += dt;
            if f.elapsed >= f.duration {
                self.flash = None;
            }
        }
    }

    /// Intensidade atual do tremor, em pixels.
    fn shake_strength(&self) -> f32 {
        self.shakes
            .iter()
            .map(|s| {
                // Cai rápido no começo e suave no fim.
                let left = 1.0 - s.elapsed / s.duration;
                s.strength * left * left
            })
            .sum()
    }

    /// Deslocamento do tremor em pixels da tela (Y para cima).
    pub(crate) fn shake_offset(&self) -> Vec2 {
        let strength = self.shake_strength();
        if strength <= 0.0 {
            return Vec2::ZERO;
        }
        // Soma de senos com frequências que não se repetem juntas: parece
        // aleatório, mas é suave e não depende de gerador de números.
        let t = self.clock;
        let x =
            (t * 47.0).sin() * 0.5 + (t * 29.3 + 1.7).sin() * 0.3 + (t * 13.1 + 4.2).sin() * 0.2;
        let y = (t * 41.7 + 2.1).sin() * 0.5
            + (t * 31.9 + 0.4).sin() * 0.3
            + (t * 17.3 + 3.3).sin() * 0.2;
        Vec2::new(x, y) * strength
    }

    fn flash_color(&self) -> Option<Color> {
        self.flash.map(|f| {
            let k = 1.0 - f.elapsed / f.duration;
            f.color.with_alpha(f.color.a * k * k)
        })
    }
}

impl Context {
    /// Faz a câmera tremer: `strength` é o deslocamento máximo em pixels da
    /// tela e o tremor diminui até parar em `seconds`. Tremores somam.
    /// Só o mundo treme; o que é desenhado na tela (HUD, menus) fica parado.
    pub fn shake(&mut self, strength: f32, seconds: f32) {
        if strength > 0.0 && seconds > 0.0 {
            self.effects.shakes.push(Shake {
                strength,
                duration: seconds,
                elapsed: 0.0,
            });
        }
    }

    /// Para qualquer tremor na hora.
    pub fn stop_shake(&mut self) {
        self.effects.shakes.clear();
    }

    /// Deslocamento atual do tremor, em pixels da tela.
    pub fn shake_offset(&self) -> Vec2 {
        self.effects.shake_offset()
    }

    /// Pinta a tela inteira com `color`, sumindo em `seconds` (ex.: vermelho
    /// ao levar dano, branco numa explosão). Um flash novo substitui o anterior.
    pub fn flash(&mut self, color: Color, seconds: f32) {
        self.effects.flash = (seconds > 0.0).then_some(Flash {
            color,
            duration: seconds,
            elapsed: 0.0,
        });
    }

    /// Câmera usada no desenho: a do jogo com o tremor aplicado.
    pub(crate) fn render_camera(&self) -> Camera2D {
        let mut camera = self.camera;
        let offset = self.effects.shake_offset();
        if offset != Vec2::ZERO {
            let zoom = camera.zoom.max(f32::EPSILON);
            camera.position += Vec2::from_angle(camera.rotation).rotate(offset) / zoom;
        }
        camera
    }

    /// Desenha o flash e o escurecimento por cima de tudo.
    pub(crate) fn draw_overlays(&mut self) {
        let view = self.viewport.as_vec2();
        let flash = self.effects.flash_color();
        let fade = self.effects.fade;
        let mut screen = self.renderer.screen();
        if let Some(color) = flash {
            screen.draw_rect(Vec2::ZERO, view, color);
        }
        if fade.a > 0.0 {
            screen.draw_rect(Vec2::ZERO, view, fade);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tremor_diminui_e_acaba() {
        let mut e = Effects::default();
        e.shakes.push(Shake {
            strength: 10.0,
            duration: 0.5,
            elapsed: 0.0,
        });
        assert!((e.shake_strength() - 10.0).abs() < 1e-6);
        e.advance(0.25);
        assert!((e.shake_strength() - 2.5).abs() < 1e-5);
        let mut maior = 0.0f32;
        for _ in 0..10 {
            e.advance(0.01);
            maior = maior.max(e.shake_offset().length());
        }
        assert!(maior > 0.5 && maior <= 2.5 * 1.42, "{maior}");
        e.advance(0.3);
        assert_eq!(e.shake_offset(), Vec2::ZERO);
        assert!(e.shakes.is_empty());
    }

    #[test]
    fn flash_some_aos_poucos() {
        let mut e = Effects {
            flash: Some(Flash {
                color: Color::rgba(1.0, 0.0, 0.0, 0.8),
                duration: 1.0,
                elapsed: 0.0,
            }),
            ..Effects::default()
        };
        assert!((e.flash_color().unwrap().a - 0.8).abs() < 1e-6);
        e.advance(0.5);
        assert!((e.flash_color().unwrap().a - 0.2).abs() < 1e-6);
        e.advance(0.6);
        assert!(e.flash_color().is_none());
    }
}
