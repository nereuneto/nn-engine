//! As cenas de teste do render: cada uma usa um pedaço do nn-render. O mesmo
//! código desenha no desktop (a referência) e no navegador.

use glam::{Mat4, UVec2, Vec2, Vec3};
use nn_core::Color;
use nn_render::d3::{Camera3D, MeshId, Sky3D, shapes};
use nn_render::{Align, ClipRect, Panel, Renderer, Sprite, Text, TextureId, TextureOptions};

/// Tamanho da tela das cenas.
pub const SIZE: UVec2 = UVec2::new(320, 180);

/// Os nomes, na ordem (o número da cena é a posição aqui).
pub const CENAS: &[&str] = &[
    "formas",
    "imagens",
    "mipmaps",
    "repetir",
    "texto",
    "painel",
    "3d",
    "fonte-pixel",
    "quebra",
    "linhas",
    "recorte",
    "canvas",
    "efeitos",
    "folhagem",
    "holofote",
    "cascatas",
    "antisserrilhado",
];

/// A cor de fundo de cada cena.
pub fn fundo(cena: usize) -> Color {
    if cena == 6 || cena == 13 || cena == 15 || cena == 16 {
        Color::hex(0x87A9D6)
    } else if cena == 14 {
        Color::hex(0x05060B)
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
    /// Um plano e uma folha com buracos (o recorte por transparência).
    plano: MeshId,
    folha: TextureId,
    canvas: TextureId,
    /// Os efeitos prontos, na ordem de nn_render::effects::BUILTIN.
    efeitos: Vec<nn_render::effects::EffectId>,
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
        let plano = r.create_mesh(&shapes::plane());
        // Uma folha redonda de 16x16, com buracos em xadrez de 4 px.
        let folha: Vec<u8> = (0u32..16 * 16)
            .flat_map(|i| {
                let (x, y) = (i % 16, i / 16);
                let (dx, dy) = (x as f32 - 7.5, y as f32 - 7.5);
                let dentro = dx * dx + dy * dy < 56.0 && !(x / 4 + y / 4).is_multiple_of(2)
                    || dx.abs() < 1.0;
                [60, 160, 90, if dentro { 255 } else { 0 }]
            })
            .collect();
        let folha = r.create_texture(UVec2::splat(16), &folha, TextureOptions::PIXEL_ART);
        let canvas = r.create_canvas(UVec2::new(80, 45), TextureOptions::PIXEL_ART);
        let efeitos = nn_render::effects::BUILTIN
            .iter()
            .map(|(nome, _)| {
                let shader = r.builtin_effect(nome).expect("efeito pronto");
                r.create_effect(shader)
            })
            .collect();
        Self {
            efeitos,
            canvas,
            pixel,
            xadrez,
            listras,
            cubo,
            plano,
            folha,
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
        7 => {
            let pixel = r.pixel_font();
            let mut s = r.screen();
            for (y, size, cor) in [
                (8.0, 8.0, 0xFFFFFF),
                (24.0, 16.0, 0xE5A84D),
                (48.0, 24.0, 0x7BC86C),
            ] {
                s.draw_text(
                    &Text::new("Ação, coração: 0123")
                        .at(Vec2::new(8.0, y))
                        .size(size)
                        .font(pixel)
                        .color(Color::hex(cor)),
                );
            }
            s.draw_text(
                &Text::new("nítida em todo múltiplo de 8 px")
                    .at(Vec2::new(8.0, 150.0))
                    .size(8.0)
                    .font(pixel)
                    .color(Color::hex(0xC8D0E0)),
            );
        }
        8 => {
            let pixel = r.pixel_font();
            let mut s = r.screen();
            let texto = "Era uma vez um herói que andava pelo mundo procurando a chave da porta.";
            for (x, align, font) in [
                (8.0, Align::Left, Some(pixel)),
                (112.0, Align::Center, Some(pixel)),
                (216.0, Align::Right, None),
            ] {
                s.draw_rect(
                    Vec2::new(x, 8.0),
                    Vec2::new(96.0, 164.0),
                    Color::hex(0x2A3446),
                );
                let mut t = Text::new(texto)
                    .at(Vec2::new(x, 8.0))
                    .size(8.0)
                    .width(96.0)
                    .align(align)
                    .color(Color::WHITE);
                if let Some(f) = font {
                    t = t.font(f);
                } else {
                    t = t.size(12.0);
                }
                s.draw_text(&t);
            }
        }
        9 => {
            let mut s = r.screen();
            // Um L côncavo, uma estrela, linhas finas e grossas, contornos.
            s.draw_polygon(
                &[
                    Vec2::new(10.0, 10.0),
                    Vec2::new(40.0, 10.0),
                    Vec2::new(40.0, 60.0),
                    Vec2::new(90.0, 60.0),
                    Vec2::new(90.0, 90.0),
                    Vec2::new(10.0, 90.0),
                ],
                Color::hex(0xE5A84D),
            );
            let estrela: Vec<Vec2> = (0..10)
                .map(|i| {
                    let a = i as f32 * std::f32::consts::PI / 5.0 - std::f32::consts::FRAC_PI_2;
                    let r = if i % 2 == 0 { 40.0 } else { 16.0 };
                    Vec2::new(150.0, 55.0) + Vec2::new(a.cos(), a.sin()) * r
                })
                .collect();
            s.draw_polygon(&estrela, Color::hex(0x7BC86C));
            s.draw_polyline(&estrela, 2.0, true, Color::WHITE);
            for i in 0..12 {
                let a = i as f32 * std::f32::consts::PI / 12.0;
                let c = Vec2::new(250.0, 55.0);
                s.draw_line(
                    c,
                    c + Vec2::new(a.cos(), -a.sin()) * 45.0,
                    1.0 + (i % 3) as f32,
                    Color::hex(0x4D9BE5),
                );
            }
            s.draw_polyline(
                &[
                    Vec2::new(20.0, 150.0),
                    Vec2::new(60.0, 110.0),
                    Vec2::new(100.0, 160.0),
                    Vec2::new(105.0, 110.0),
                ],
                6.0,
                false,
                Color::hex(0xC0405A).with_alpha(0.8),
            );
            s.draw_rect_outline(
                Vec2::new(130.0, 110.0),
                Vec2::new(80.0, 50.0),
                3.0,
                Color::WHITE,
            );
            s.draw_circle_outline(Vec2::new(260.0, 135.0), 30.0, 4.0, Color::hex(0xE5A84D));
        }
        10 => {
            let mut s = r.screen();
            // Uma lista recortada: as linhas de cima e de baixo aparecem
            // pela metade.
            s.draw_rect(
                Vec2::new(20.0, 20.0),
                Vec2::new(140.0, 140.0),
                Color::hex(0x2A3446),
            );
            s.set_clip(Some(ClipRect::new(
                Vec2::new(28.0, 28.0),
                Vec2::new(152.0, 152.0),
            )));
            for i in 0..8 {
                let y = 12.0 + i as f32 * 22.0;
                s.draw_rect(
                    Vec2::new(28.0, y),
                    Vec2::new(124.0, 18.0),
                    Color::hex(0x4D9BE5),
                );
                s.draw_text(
                    &Text::new(&format!("Item {}", i + 1))
                        .at(Vec2::new(34.0, y + 2.0))
                        .size(12.0)
                        .color(Color::WHITE),
                );
            }
            s.set_clip(None);
        }
        13 => {
            // Folhas recortadas, com as duas faces e a sombra com os buracos.
            r.set_camera3d(Camera3D::looking_at(
                Vec3::new(2.5, 2.2, 3.5),
                Vec3::new(0.0, 0.6, 0.0),
            ));
            r.set_sky3d(Some(Sky3D::default()));
            r.draw_mesh(
                rec.cubo,
                Mat4::from_translation(Vec3::new(0.0, -0.1, 0.0))
                    * Mat4::from_scale(Vec3::new(8.0, 0.2, 8.0)),
                Color::hex(0xD8C9A0),
            );
            let folha = nn_render::d3::Material3D {
                cutout: 0.5,
                double_sided: true,
                ..Default::default()
            };
            for (x, giro) in [(-1.2_f32, 0.4_f32), (0.0, 2.8), (1.2, -0.9)] {
                let m = Mat4::from_translation(Vec3::new(x, 0.9, 0.0))
                    * Mat4::from_rotation_y(giro)
                    * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2)
                    * Mat4::from_scale(Vec3::splat(1.4));
                r.draw_mesh_material(rec.plano, m, Color::WHITE, Some(rec.folha), true, folha);
            }
        }
        14 => {
            // No escuro, sem o sol: dois holofotes com sombra (um deles
            // colorido) sobre cubos e uma folha recortada.
            r.set_camera3d(Camera3D::looking_at(
                Vec3::new(0.0, 4.5, 5.0),
                Vec3::new(0.0, 0.0, 0.5),
            ));
            r.set_sky3d(None);
            r.set_light3d(nn_render::d3::Light3D {
                sun_color: Color::BLACK,
                sky: Color::hex(0x101018),
                ground: Color::hex(0x080808),
                fog: None,
                ..Default::default()
            });
            r.set_shadows3d(nn_render::d3::Shadows {
                sun: false,
                ..Default::default()
            });
            r.draw_mesh(
                rec.cubo,
                Mat4::from_translation(Vec3::new(0.0, -0.1, 0.0))
                    * Mat4::from_scale(Vec3::new(10.0, 0.2, 10.0)),
                Color::WHITE,
            );
            for x in [-1.3_f32, 0.9] {
                r.draw_mesh(
                    rec.cubo,
                    Mat4::from_translation(Vec3::new(x, 0.6, 0.3))
                        * Mat4::from_rotation_y(0.5)
                        * Mat4::from_scale(Vec3::splat(0.6)),
                    Color::hex(0xE5A84D),
                );
            }
            r.draw_mesh_material(
                rec.plano,
                Mat4::from_translation(Vec3::new(0.0, 1.4, -0.3))
                    * Mat4::from_scale(Vec3::splat(1.2)),
                Color::WHITE,
                Some(rec.folha),
                true,
                nn_render::d3::Material3D {
                    cutout: 0.5,
                    double_sided: true,
                    ..Default::default()
                },
            );
            for (pos, cor) in [
                (Vec3::new(-1.0, 3.5, 1.5), Color::WHITE),
                (Vec3::new(1.5, 3.0, 1.0), Color::hex(0x7BC8FF)),
            ] {
                let mut lamp = nn_render::d3::Lamp3D::at(pos);
                lamp.color = cor;
                lamp.intensity = 1.6;
                lamp.range = 10.0;
                lamp.shadow = true;
                lamp.cone = Some(nn_render::d3::Cone3D {
                    direction: (Vec3::new(0.0, 0.0, 0.0) - pos).normalize(),
                    angle: 35.0,
                    softness: 0.3,
                });
                r.add_lamp3d(lamp);
            }
        }
        15 => {
            // Uma estrada de 150 m com postes e casas, a sombra até 120 m:
            // as três cascatas do sol (nítida perto, e chegando longe).
            r.set_camera3d(Camera3D::looking_at(
                Vec3::new(2.0, 3.0, 4.0),
                Vec3::new(0.0, 1.0, -10.0),
            ));
            r.set_sky3d(Some(Sky3D::default()));
            r.set_light3d(nn_render::d3::Light3D {
                sun_direction: Vec3::new(0.6, -0.7, 0.35).normalize(),
                ..Default::default()
            });
            r.set_shadows3d(nn_render::d3::Shadows {
                distance: 120.0,
                ..Default::default()
            });
            r.draw_mesh(
                rec.cubo,
                Mat4::from_translation(Vec3::new(0.0, -0.1, -60.0))
                    * Mat4::from_scale(Vec3::new(60.0, 0.2, 160.0)),
                Color::hex(0x9FB57A),
            );
            for k in 0..12 {
                let z = -(k as f32) * 12.0;
                r.draw_mesh(
                    rec.cubo,
                    Mat4::from_translation(Vec3::new(-2.0, 1.5, z))
                        * Mat4::from_scale(Vec3::new(0.25, 3.0, 0.25)),
                    Color::hex(0x5A4A3A),
                );
                r.draw_mesh(
                    rec.cubo,
                    Mat4::from_translation(Vec3::new(-9.0, 2.0, z - 4.0))
                        * Mat4::from_scale(Vec3::new(5.0, 4.0, 5.0)),
                    Color::hex(0xE5A84D),
                );
            }
        }
        16 => {
            // Cubos inclinados com antisserrilhado e uma lâmpada forte com
            // tonemap (sem estourar em branco).
            r.set_antialias3d(true);
            r.set_tonemap3d(true);
            r.set_camera3d(Camera3D::looking_at(Vec3::new(0.0, 2.0, 5.0), Vec3::ZERO));
            r.set_sky3d(Some(Sky3D::default()));
            r.draw_mesh(
                rec.cubo,
                Mat4::from_translation(Vec3::new(0.0, -0.6, 0.0))
                    * Mat4::from_scale(Vec3::new(8.0, 0.2, 8.0)),
                Color::WHITE,
            );
            for (k, x) in [-1.6_f32, 0.0, 1.6].into_iter().enumerate() {
                r.draw_mesh(
                    rec.cubo,
                    Mat4::from_translation(Vec3::new(x, 0.2, 0.0))
                        * Mat4::from_rotation_y(0.4 + k as f32 * 0.5)
                        * Mat4::from_rotation_x(0.3),
                    Color::hex(0xE5A84D),
                );
            }
            let mut lamp = nn_render::d3::Lamp3D::at(Vec3::new(0.0, 0.8, 1.2));
            lamp.intensity = 5.0;
            lamp.range = 4.0;
            lamp.color = Color::hex(0xFFD9A0);
            r.add_lamp3d(lamp);
        }
        _ => {}
    }
    if cena == 11 {
        // Um jogo em 80x45 ampliado 4 vezes: o pixel art do herói, um
        // círculo, texto e uma linha, tudo nítido.
        r.begin_canvas(rec.canvas);
        r.draw_circle(Vec2::new(-20.0, 0.0), 12.0, Color::hex(0x7BC86C));
        r.draw(
            &Sprite::new(rec.pixel)
                .at(Vec2::new(14.0, 2.0))
                .size(Vec2::splat(16.0)),
        );
        r.screen()
            .draw_text(
                &Text::new("80x45")
                    .at(Vec2::new(2.0, 2.0))
                    .size(8.0)
                    .font(nn_render::FontId::PIXEL),
            )
            .draw_line(
                Vec2::new(2.0, 40.0),
                Vec2::new(78.0, 34.0),
                1.0,
                Color::hex(0xE5A84D),
            );
        r.end_canvas(&nn_render::Camera2D::default(), Some(Color::hex(0x2A3446)));
        r.screen().draw(
            &Sprite::new(rec.canvas)
                .at(Vec2::ZERO)
                .origin(Vec2::ZERO)
                .size(Vec2::new(320.0, 180.0)),
        );
    }
    if cena == 12 {
        // Os efeitos prontos, um em cada imagem (o tempo parado).
        r.set_time(1.0);
        for (i, e) in rec.efeitos.iter().enumerate() {
            let (col, row) = ((i % 3) as f32, (i / 3) as f32);
            r.screen().draw(
                &Sprite::new(rec.pixel)
                    .at(Vec2::new(12.0 + col * 100.0, 8.0 + row * 86.0))
                    .origin(Vec2::ZERO)
                    .size(Vec2::splat(72.0))
                    .effect(*e),
            );
        }
    }
    // O mundo recortado: um círculo grande, só um quadrado dele.
    if cena == 10 {
        r.set_clip(Some(ClipRect::new(
            Vec2::new(40.0, -40.0),
            Vec2::new(120.0, 40.0),
        )));
        r.draw_circle(Vec2::new(80.0, 0.0), 70.0, Color::hex(0xE5A84D));
        r.set_clip(None);
    }
}
