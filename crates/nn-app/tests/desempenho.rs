//! O medidor de desempenho e a tela F3.

mod comum;

use std::time::{Duration, Instant};

use glam::Vec2;
use nn_app::{AppConfig, Context, Game};
use nn_core::Color;
use nn_input::KeyCode;

/// Gasta ~2 ms em cada passo fixo e ~1 ms no update.
struct Pesada;

fn gastar(ms: u64) {
    let t = Instant::now();
    while t.elapsed() < Duration::from_millis(ms) {
        std::hint::spin_loop();
    }
}

impl Game for Pesada {
    fn fixed_update(&mut self, ctx: &mut Context) {
        gastar(2);
        ctx.perf.add_physics(Duration::from_millis(1));
    }
    fn update(&mut self, _: &mut Context) {
        gastar(1);
    }
    fn render(&mut self, ctx: &mut Context) {
        ctx.renderer
            .screen()
            .draw_rect(Vec2::ZERO, Vec2::splat(64.0), Color::rgb(1.0, 0.0, 0.0));
    }
}

fn config(dev: bool) -> AppConfig {
    let mut c = AppConfig::default()
        .size(320, 180)
        .clear_color(Color::BLACK);
    c.dev_tools = dev;
    c
}

fn pixel(h: &nn_app::Headless<Pesada>, x: u32, y: u32) -> [u8; 4] {
    let px = h.pixels().unwrap();
    let i = ((y * 320 + x) * 4) as usize;
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}

#[test]
fn mede_os_passos_fixos_a_fisica_e_o_update() {
    let Some(mut h) = comum::headless(config(true), |_| Pesada) else {
        return;
    };
    h.run_frames(10);
    let a = h.ctx.perf.average;
    assert_eq!(h.ctx.perf.last.steps, 1);
    assert!(a.fixed >= 1.8, "{a:?}");
    assert!((a.physics - 1.0).abs() < 0.05, "{a:?}");
    assert!(a.update >= 0.9, "{a:?}");
    assert!(h.ctx.perf.fps() > 0.0);
    let lines = h.ctx.perf_lines();
    assert!(lines[1].contains("física 1.00 ms"), "{lines:?}");
}

#[test]
fn f3_abre_a_tela_so_no_jogo_em_desenvolvimento() {
    for dev in [true, false] {
        let Some(mut h) = comum::headless(config(dev), |_| Pesada) else {
            return;
        };
        h.run_frames(1);
        assert_eq!(pixel(&h, 10, 10), [255, 0, 0, 255]);
        h.ctx.input.press_key(KeyCode::F3);
        h.run_frames(1);
        h.ctx.input.release_key(KeyCode::F3);
        h.run_frames(1);
        assert_eq!(h.ctx.perf.overlay, dev);
        // O painel escuro por cima do vermelho, no canto (a mistura é
        // linear: 30% de 255 vira ~149 em sRGB).
        let p = pixel(&h, 5, 5);
        if dev {
            assert!(p[0] < 200, "{p:?}");
        } else {
            assert_eq!(p, [255, 0, 0, 255]);
        }
    }
}
