//! Tremor da câmera e flash de tela.

mod comum;

use glam::Vec2;
use nn_app::{AppConfig, Context, Game};
use nn_core::Color;

/// Um quadrado vermelho no centro do mundo e um "HUD" azul fixo na tela.
struct Cena;

impl Game for Cena {
    fn render(&mut self, ctx: &mut Context) {
        ctx.renderer
            .draw_rect(Vec2::ZERO, Vec2::splat(20.0), Color::rgb(1.0, 0.0, 0.0));
        ctx.renderer
            .screen()
            .draw_rect(Vec2::ZERO, Vec2::splat(8.0), Color::rgb(0.0, 0.0, 1.0));
    }
}

fn config() -> AppConfig {
    AppConfig::default().size(64, 64).clear_color(Color::BLACK)
}

fn pixel(h: &nn_app::Headless<Cena>, p: Vec2) -> [u8; 4] {
    let px = h.pixels().unwrap();
    let i = ((p.y.round() as u32 * 64 + p.x.round() as u32) * 4) as usize;
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}

#[test]
fn tremor_move_o_mundo_mas_nao_o_hud() {
    let Some(mut h) = comum::headless(config(), |_| Cena) else {
        return;
    };
    h.run_frames(1);
    assert_eq!(pixel(&h, Vec2::new(32.0, 32.0))[0], 255);
    assert_eq!(h.ctx.shake_offset(), Vec2::ZERO);

    h.ctx.shake(12.0, 1.0);
    h.run_frames(3);
    let off = h.ctx.shake_offset();
    assert!(off.length() > 1.0, "{off}");
    // A câmera foi para +off, então o quadrado aparece deslocado ao contrário
    // (na tela, Y é para baixo).
    let centro = Vec2::new(32.0 - off.x, 32.0 + off.y);
    assert_eq!(pixel(&h, centro)[0], 255);
    let borda_antiga = Vec2::new(32.0 + 9.5 * off.x.signum(), 32.0);
    if off.x.abs() > 2.0 {
        assert_eq!(pixel(&h, borda_antiga)[0], 0, "a borda saiu do lugar");
    }
    // O HUD continua no mesmo lugar.
    assert_eq!(pixel(&h, Vec2::new(3.0, 3.0))[2], 255);
    // A posição da câmera do jogo não muda.
    assert_eq!(h.ctx.camera.position, Vec2::ZERO);

    h.run_frames(60);
    assert_eq!(h.ctx.shake_offset(), Vec2::ZERO);
    assert_eq!(pixel(&h, Vec2::new(32.0, 32.0))[0], 255);

    h.ctx.shake(30.0, 5.0);
    h.run_frames(2);
    h.ctx.stop_shake();
    assert_eq!(h.ctx.shake_offset(), Vec2::ZERO);
}

#[test]
fn flash_cobre_a_tela_e_some() {
    let Some(mut h) = comum::headless(config(), |_| Cena) else {
        return;
    };
    h.ctx.flash(Color::WHITE, 0.5);
    h.run_frames(1);
    let canto = Vec2::new(60.0, 60.0);
    let [r, g, b, _] = pixel(&h, canto);
    assert!(r > 200 && g > 200 && b > 200, "{r} {g} {b}");
    h.run_frames(15);
    let [r, ..] = pixel(&h, canto);
    assert!(r > 0 && r < 200, "sumindo: {r}");
    h.run_frames(20);
    assert_eq!(pixel(&h, canto)[0], 0);
}
