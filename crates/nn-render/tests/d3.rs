//! Testes do 3D: desenham malhas numa textura fora da tela e conferem os
//! pixels. Sem GPU, pulam (a menos que `NN_REQUIRE_GPU=1`).

use glam::{Mat4, UVec2, Vec2, Vec3};
use nn_core::Color;
use nn_render::d3::{shapes, Camera3D, Light3D, Transform3};
use nn_render::{Camera2D, GpuContext, OffscreenTarget, Presentation, Renderer, ScaleMode};

const SIZE: UVec2 = UVec2::new(64, 64);

struct Harness {
    gpu: GpuContext,
    renderer: Renderer,
    target: OffscreenTarget,
}

impl Harness {
    fn new() -> Option<Self> {
        match pollster::block_on(GpuContext::new_headless()) {
            Ok(gpu) => {
                let mut renderer = Renderer::new(&gpu, OffscreenTarget::FORMAT);
                // Luz "neutra": só ambiente branco, a cor sai igual à da malha.
                renderer.set_light3d(flat_light());
                let target = OffscreenTarget::new(&gpu, SIZE);
                Some(Self {
                    gpu,
                    renderer,
                    target,
                })
            }
            Err(e) => {
                if std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1") {
                    panic!("GPU obrigatória mas indisponível: {e}");
                }
                eprintln!("pulando teste: {e}");
                None
            }
        }
    }

    fn render(&mut self, clear: Color) -> Vec<u8> {
        self.renderer
            .render(self.target.view(), SIZE, &Camera2D::default(), Some(clear));
        self.target.read_rgba(&self.gpu).expect("leitura dos pixels")
    }
}

fn flat_light() -> Light3D {
    Light3D {
        sun_color: Color::BLACK,
        sky: Color::WHITE,
        ground: Color::WHITE,
        ..Light3D::default()
    }
}

fn pixel(img: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * SIZE.x + x) * 4) as usize;
    img[i..i + 4].try_into().unwrap()
}

fn assert_color(actual: [u8; 4], expected: [u8; 4], where_: &str) {
    let close = actual
        .iter()
        .zip(expected)
        .all(|(a, e)| (i16::from(*a) - i16::from(e)).abs() <= 3);
    assert!(close, "{where_}: esperado {expected:?}, obtido {actual:?}");
}

fn brightness(p: [u8; 4]) -> u32 {
    u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])
}

#[test]
fn cubo_aparece_no_meio() {
    let Some(mut h) = Harness::new() else { return };
    let cube = h.renderer.create_mesh(&shapes::cube());
    h.renderer
        .set_camera3d(Camera3D::looking_at(Vec3::new(0.0, 0.0, 3.0), Vec3::ZERO));
    h.renderer.draw_mesh(cube, Mat4::IDENTITY, Color::RED);
    let img = h.render(Color::BLACK);
    assert_color(pixel(&img, 32, 32), [255, 0, 0, 255], "centro");
    assert_color(pixel(&img, 1, 1), [0, 0, 0, 255], "canto");
    assert_eq!(h.renderer.last_mesh_count(), 1);
}

#[test]
fn o_mais_perto_fica_na_frente_em_qualquer_ordem() {
    let Some(mut h) = Harness::new() else { return };
    let cube = h.renderer.create_mesh(&shapes::cube());
    h.renderer
        .set_camera3d(Camera3D::looking_at(Vec3::new(0.0, 0.0, 4.0), Vec3::ZERO));
    for near_first in [true, false] {
        let near = (Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0)), Color::RED);
        let far = (
            Mat4::from_scale_rotation_translation(
                Vec3::splat(3.0),
                glam::Quat::IDENTITY,
                Vec3::new(0.0, 0.0, -3.0),
            ),
            Color::BLUE,
        );
        let order = if near_first { [near, far] } else { [far, near] };
        for (m, c) in order {
            h.renderer.draw_mesh(cube, m, c);
        }
        let img = h.render(Color::BLACK);
        assert_color(pixel(&img, 32, 32), [255, 0, 0, 255], "o de perto");
        // O de trás é maior e aparece em volta.
        assert_color(pixel(&img, 32, 19), [0, 0, 255, 255], "o de trás");
    }
}

#[test]
fn sol_ilumina_o_lado_virado_para_ele() {
    let Some(mut h) = Harness::new() else { return };
    let plane = h.renderer.create_mesh(&shapes::plane());
    h.renderer
        .set_camera3d(Camera3D::looking_at(Vec3::new(0.0, 2.0, 0.01), Vec3::ZERO));
    let model = Transform3::default().scaled(Vec3::splat(4.0)).matrix();
    let mut shot = |sun: Vec3| {
        h.renderer.set_light3d(Light3D {
            sun_direction: sun,
            sun_color: Color::rgb(0.6, 0.6, 0.6),
            sky: Color::rgb(0.2, 0.2, 0.2),
            ground: Color::rgb(0.2, 0.2, 0.2),
            fog: None,
        });
        h.renderer.draw_mesh(plane, model, Color::WHITE);
        pixel(&h.render(Color::BLACK), 32, 32)
    };
    let lit = shot(Vec3::NEG_Y);
    let grazing = shot(Vec3::new(1.0, -0.3, 0.0));
    let below = shot(Vec3::Y);
    assert!(
        brightness(lit) > brightness(grazing) && brightness(grazing) > brightness(below),
        "de cima {lit:?}, de lado {grazing:?}, de baixo {below:?}"
    );
    // Sem sol, sobra a luz ambiente: cinza 0,2 (51), como as cores do 2D.
    assert_color(below, [51, 51, 51, 255], "só ambiente");
}

#[test]
fn faces_do_cubo_recebem_luz_diferente() {
    // O visual low-poly: cada face com um tom só, e tons diferentes entre si.
    let Some(mut h) = Harness::new() else { return };
    let cube = h.renderer.create_mesh(&shapes::cube());
    h.renderer
        .set_camera3d(Camera3D::looking_at(Vec3::new(2.0, 2.0, 2.0), Vec3::ZERO));
    h.renderer.set_light3d(Light3D::default());
    h.renderer.draw_mesh(cube, Mat4::IDENTITY, Color::WHITE);
    let img = h.render(Color::BLACK);
    // Topo (em cima do meio), frente-direita e frente-esquerda (embaixo).
    let top = pixel(&img, 32, 22);
    let left = pixel(&img, 25, 38);
    let right = pixel(&img, 39, 38);
    assert_ne!(top, left);
    assert_ne!(left, right);
    assert_ne!(top, right);
    // Pontos da mesma face têm a mesma cor.
    assert_color(pixel(&img, 32, 20), top, "mesma face");
}

#[test]
fn hud_fica_por_cima_do_3d() {
    let Some(mut h) = Harness::new() else { return };
    let cube = h.renderer.create_mesh(&shapes::cube());
    h.renderer
        .set_camera3d(Camera3D::looking_at(Vec3::new(0.0, 0.0, 2.0), Vec3::ZERO));
    h.renderer.draw_mesh(cube, Mat4::IDENTITY, Color::RED);
    // Sprite do mundo 2D também fica por cima do 3D.
    h.renderer
        .draw_rect(Vec2::new(-16.0, 0.0), Vec2::splat(8.0), Color::BLUE);
    h.renderer
        .screen()
        .draw_rect(Vec2::new(28.0, 28.0), Vec2::splat(8.0), Color::GREEN);
    let img = h.render(Color::BLACK);
    assert_color(pixel(&img, 32, 32), [0, 255, 0, 255], "HUD");
    assert_color(pixel(&img, 16, 32), [0, 0, 255, 255], "sprite");
    assert_color(pixel(&img, 40, 40), [255, 0, 0, 255], "cubo em volta");
}

#[test]
fn nevoa_some_com_o_que_esta_longe() {
    let Some(mut h) = Harness::new() else { return };
    let cube = h.renderer.create_mesh(&shapes::cube());
    h.renderer
        .set_camera3d(Camera3D::looking_at(Vec3::new(0.0, 0.0, 30.0), Vec3::ZERO));
    h.renderer.set_light3d(Light3D {
        fog: Some((Color::rgb(0.0, 1.0, 0.0), 1.0, 10.0)),
        ..flat_light()
    });
    let big = Mat4::from_scale(Vec3::splat(20.0));
    h.renderer.draw_mesh(cube, big, Color::RED);
    let img = h.render(Color::BLACK);
    assert_color(pixel(&img, 32, 32), [0, 255, 0, 255], "tudo névoa");
    // Fora do cubo, a cor de fundo (a névoa só vale nas malhas).
    assert_color(pixel(&img, 0, 0), [0, 0, 0, 255], "fundo");
}

#[test]
fn tamanho_fixo_desenha_so_na_area_do_jogo() {
    let Some(mut h) = Harness::new() else { return };
    // Jogo 2:1 numa janela quadrada: faixas pretas em cima e embaixo.
    h.renderer.set_presentation(Some(Presentation::new(
        SIZE,
        UVec2::new(128, 64),
        ScaleMode::Fit,
    )));
    let cube = h.renderer.create_mesh(&shapes::cube());
    h.renderer
        .set_camera3d(Camera3D::looking_at(Vec3::new(0.0, 0.0, 0.8), Vec3::ZERO));
    h.renderer.draw_mesh(cube, Mat4::IDENTITY, Color::RED);
    let img = h.render(Color::BLUE);
    // Bem perto do cubo: ele cobre toda a área do jogo (16..48 em y).
    assert_color(pixel(&img, 32, 32), [255, 0, 0, 255], "meio");
    assert_color(pixel(&img, 2, 18), [255, 0, 0, 255], "canto do jogo");
    assert_color(pixel(&img, 32, 4), [0, 0, 0, 255], "faixa de cima");
    assert_color(pixel(&img, 32, 60), [0, 0, 0, 255], "faixa de baixo");
}

#[test]
fn sem_3d_o_quadro_fica_como_antes() {
    let Some(mut h) = Harness::new() else { return };
    h.renderer.create_mesh(&shapes::sphere(2));
    h.renderer
        .draw_rect(Vec2::ZERO, Vec2::splat(16.0), Color::RED);
    let img = h.render(Color::BLUE);
    assert_color(pixel(&img, 32, 32), [255, 0, 0, 255], "sprite");
    assert_color(pixel(&img, 2, 2), [0, 0, 255, 255], "fundo");
    assert_eq!(h.renderer.last_mesh_count(), 0);
}

#[test]
fn varias_formas_e_muitas_copias() {
    let Some(mut h) = Harness::new() else { return };
    let meshes = [
        shapes::cube(),
        shapes::sphere(2),
        shapes::cylinder(12),
        shapes::cone(12),
        shapes::capsule(12),
        shapes::plane(),
    ]
    .map(|m| h.renderer.create_mesh(&m));
    h.renderer
        .set_camera3d(Camera3D::looking_at(Vec3::new(0.0, 30.0, 30.0), Vec3::ZERO));
    for i in 0..600 {
        let x = (i % 30) as f32 - 15.0;
        let z = (i / 30) as f32 - 10.0;
        h.renderer.draw_mesh(
            meshes[i % meshes.len()],
            Mat4::from_translation(Vec3::new(x, 0.0, z)),
            Color::WHITE,
        );
    }
    let img = h.render(Color::BLACK);
    assert_eq!(h.renderer.last_mesh_count(), 600);
    assert!(brightness(pixel(&img, 32, 32)) > 0, "algo no meio");
}
