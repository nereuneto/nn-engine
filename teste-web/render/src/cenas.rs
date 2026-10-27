//! As cenas de teste do render: cada uma usa um pedaço do nn-render. O mesmo
//! código desenha no desktop (a referência) e no navegador.

use glam::{Mat4, UVec2, Vec2, Vec3};
use nn_core::Color;
use nn_render::d3::{Camera3D, MeshId, Sky3D, shapes};
use nn_render::{Panel, Renderer, Sprite, Text, TextureId, TextureOptions};

/// Tamanho da tela das cenas.
pub const SIZE: UVec2 = UVec2::new(320, 180);

/// Os nomes, na ordem (o número da cena é a posição aqui).
pub const CENAS: &[&str] = &[
    "formas", "imagens", "mipmaps", "repetir", "texto", "painel", "3d",
];

/// A cor de fundo de cada cena.
pub fn fundo(cena: usize) -> Color {
    if cena == 6 {
        Color::hex(0x87A9D6)
    } else {
        Color::hex(0x1B2233)
    }
}

/// As imagens e as malhas das cenas, criadas uma vez por renderer.
pub struct Recursos {
    pixel: TextureId,
    xadrez: TextureId,
    listras: TextureId,
    cubo: MeshId,
}

/// Um personagem de 8x8 em pixel art.
const HEROI: [&str; 8] = [
    "..rrrr..", ".rrrrrr.", ".ssbssb.", ".ssssss.", "..gggg..", ".gggggg.", "..b..b..", ".bb..bb.",
];

impl Recursos {
    pub fn new(r: &mut Renderer) -> Self {
        let mut px = Vec::new();
        for row in HEROI {
            for c in row.chars() {
                px.extend_from_slice(&match c {
                    'r' => [200, 50, 60, 255],
                    's' => [240, 200, 160, 255],
                    'b' => [40, 40, 60, 255],
                    'g' => [60, 160, 90, 255],
                    _ => [0, 0, 0, 0],
                });
            }
        }
        let pixel = r.create_texture(UVec2::splat(8), &px, TextureOptions::PIXEL_ART);
        let n = 128u32;
        let xadrez: Vec<u8> = (0..n * n)
            .flat_map(|i| {
                let v = if (i % n + i / n).is_multiple_of(2) {
                    255
                } else {
                    0
                };
                [v, v, v, 255]
            })
            .collect();
        let xadrez = r.create_texture(UVec2::splat(n), &xadrez, TextureOptions::SMOOTH);
        let listras: Vec<u8> = (0..16 * 16)
            .flat_map(|i| {
                if i % 16 < 8 {
                    [230, 160, 60, 255]
                } else {
                    [60, 110, 200, 255]
                }
            })
            .collect();
        let listras = r.create_texture(UVec2::splat(16), &listras, TextureOptions::PIXEL_ART);
        let cubo = r.create_mesh(&shapes::cube());
        Self {
            pixel,
            xadrez,
            listras,
            cubo,
        }
    }
}

/// Enfileira a cena `cena` no renderer (a tela é `SIZE`, y para baixo).
pub fn desenhar(cena: usize, r: &mut Renderer, rec: &Recursos) {
    match cena {
        0 => {
            let mut s = r.screen();
            s.draw_rect(
                Vec2::new(10.0, 10.0),
                Vec2::new(90.0, 60.0),
                Color::hex(0xE5A84D),
            );
            s.draw_rect(
                Vec2::new(60.0, 40.0),
                Vec2::new(90.0, 60.0),
                Color::hex(0x4D9BE5).with_alpha(0.5),
            );
            s.draw_circle(Vec2::new(230.0, 70.0), 45.0, Color::hex(0x7BC86C));
            s.draw_circle(Vec2::new(260.0, 120.0), 25.0, Color::WHITE.with_alpha(0.7));
            s.draw_rect(
                Vec2::new(10.0, 130.0),
                Vec2::new(300.0, 30.0),
                Color::hex(0xC0405A),
            );
        }
        1 => {
            let mut s = r.screen();
            for (i, scale) in [2.0f32, 4.0, 8.0].iter().enumerate() {
                s.draw(
                    &Sprite::new(rec.pixel)
                        .at(Vec2::new(20.0 + i as f32 * 70.0, 20.0))
                        .origin(Vec2::ZERO)
                        .size(Vec2::splat(8.0 * scale)),
                );
            }
            s.draw(
                &Sprite::new(rec.pixel)
                    .at(Vec2::new(250.0, 120.0))
                    .size(Vec2::splat(48.0))
                    .rotation(0.5)
                    .color(Color::hex(0x80C0FF)),
            );
        }
        2 => {
            let mut s = r.screen();
            // O mesmo xadrez em vários tamanhos: com mipmaps, cinza parelho.
            for (x, size) in [
                (10.0f32, 128.0f32),
                (150.0, 61.0),
                (225.0, 37.0),
                (275.0, 19.0),
            ] {
                s.draw(
                    &Sprite::new(rec.xadrez)
                        .at(Vec2::new(x, 20.0))
                        .origin(Vec2::ZERO)
                        .size(Vec2::splat(size)),
                );
            }
        }
        3 => {
            let mut s = r.screen();
            let base = Sprite::new(rec.listras)
                .origin(Vec2::ZERO)
                .size(Vec2::new(300.0, 40.0))
                .repeat(Vec2::new(300.0 / 32.0, 40.0 / 32.0));
            s.draw(&base.at(Vec2::new(10.0, 10.0)));
            s.draw(&base.at(Vec2::new(10.0, 60.0)).offset(Vec2::new(0.25, 0.0)));
            s.draw(&base.at(Vec2::new(10.0, 110.0)).offset(Vec2::new(0.5, 0.5)));
        }
        4 => {
            let mut s = r.screen();
            s.draw_text(
                &Text::new("nn-engine no navegador")
                    .at(Vec2::new(10.0, 10.0))
                    .size(24.0)
                    .color(Color::WHITE),
            );
            s.draw_text(
                &Text::new("Ação, coração e pão: 0123456789")
                    .at(Vec2::new(10.0, 50.0))
                    .size(16.0)
                    .color(Color::hex(0xE5A84D)),
            );
            s.draw_text(
                &Text::new("pequeno, 11 px")
                    .at(Vec2::new(10.0, 80.0))
                    .size(11.0)
                    .color(Color::hex(0xC8D0E0)),
            );
        }
        5 => {
            let mut s = r.screen();
            s.draw_panel(
                &Panel::new(Vec2::new(90.0, 90.0), Vec2::new(150.0, 120.0))
                    .color(Color::hex(0x2A3446))
                    .radius(12.0)
                    .border(3.0, Color::hex(0xE5A84D)),
            );
            s.draw_panel(
                &Panel::new(Vec2::new(240.0, 90.0), Vec2::new(110.0, 60.0))
                    .color(Color::hex(0x4D9BE5))
                    .radius(30.0),
            );
        }
        6 => {
            r.set_camera3d(Camera3D::looking_at(Vec3::new(3.0, 2.5, 4.0), Vec3::ZERO));
            r.set_sky3d(Some(Sky3D::default()));
            let model = Mat4::from_rotation_y(0.6);
            r.draw_mesh(rec.cubo, model, Color::hex(0xE5A84D));
            r.draw_mesh(
                rec.cubo,
                Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0))
                    * Mat4::from_scale(Vec3::new(8.0, 0.2, 8.0)),
                Color::hex(0x7BC86C),
            );
        }
        _ => {}
    }
}
