//! Gera imagens de referência para conferência visual.
//!
//! Ignorado por padrão. Rode com:
//! `NN_CAPTURE_DIR=capturas cargo test -p nn-render --test captura -- --ignored`

use glam::{UVec2, Vec2};
use nn_core::Color;
use nn_render::{Align, Anchor, Camera2D, GpuContext, OffscreenTarget, Renderer, Text};

#[test]
#[ignore = "gera arquivos; rode manualmente"]
fn captura_de_texto() {
    let dir = std::env::var("NN_CAPTURE_DIR").unwrap_or_else(|_| "capturas".into());
    std::fs::create_dir_all(&dir).unwrap();
    let gpu = pollster::block_on(GpuContext::new_headless()).expect("GPU");
    let size = UVec2::new(640, 360);
    let mut renderer = Renderer::new(&gpu, OffscreenTarget::FORMAT);
    let target = OffscreenTarget::new(&gpu, size);

    let mut y = 16.0;
    for px in [12.0, 16.0, 24.0, 36.0, 56.0] {
        let label = format!("Poppins {px} px — Olá, ação! 0123 ÁÉÍÓÚ çãõ");
        renderer
            .screen()
            .draw_text(&Text::new(&label).at(Vec2::new(16.0, y)).size(px));
        y += px * 1.5;
    }
    renderer.screen().draw_text(
        &Text::new("Centro\nalinhado ao centro\nfim")
            .at(Vec2::new(620.0, 340.0))
            .anchor(Anchor::BottomRight)
            .align(Align::Center)
            .color(Color::hex(0xFFD24A)),
    );
    renderer.render(
        target.view(),
        size,
        &Camera2D::default(),
        Some(Color::hex(0x1E1E28)),
    );
    target
        .save_png(&gpu, format!("{dir}/texto.png"))
        .expect("salvar PNG");
}
