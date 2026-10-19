//! Testes do 3D: desenham malhas numa textura fora da tela e conferem os
//! pixels. Sem GPU, pulam (a menos que `NN_REQUIRE_GPU=1`).

use glam::{Mat4, UVec2, Vec2, Vec3};
use nn_core::Color;
use nn_render::d3::{Camera3D, Fog, Light3D, Transform3, shapes};
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
        self.target
            .read_rgba(&self.gpu)
            .expect("leitura dos pixels")
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
        fog: Some(Fog {
            color: Some(Color::rgb(0.0, 1.0, 0.0)),
            near: 1.0,
            far: 10.0,
        }),
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

// --- Modelos, sombras e céu (Sprint 21) ----------------------------------

use nn_render::d3::{ModelData, Shadows, Sky3D};

fn modelo(nome: &str) -> ModelData {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jogos/cubos3d/modelos");
    let bytes = std::fs::read(dir.join(nome)).unwrap();
    ModelData::from_gltf(&bytes, &mut |_| None).unwrap()
}

#[test]
fn modelos_gerados_carregam_com_as_partes_e_as_cores() {
    let arvore = modelo("arvore.glb");
    // Tronco e duas copas (cada uma com o seu material).
    assert_eq!(arvore.parts.len(), 1, "sem textura: uma parte só");
    let cores: std::collections::HashSet<[u8; 3]> = arvore.parts[0]
        .mesh
        .vertices
        .iter()
        .map(|v| v.color.map(|c| (c * 255.0).round() as u8)[..3].try_into().unwrap())
        .collect();
    assert!(cores.contains(&[0x8B, 0x5A, 0x3C]), "tronco: {cores:?}");
    assert!(cores.contains(&[0x3E, 0x9B, 0x4F]), "copa");
    // As copas sobem pelo nó: a árvore vai até ~2,25 de altura.
    let (min, max) = arvore.bounds();
    assert!(min.y.abs() < 1e-4 && (max.y - 2.25).abs() < 0.01, "{min} {max}");

    // A casa tem um nó pai com escala 1,5 (a chaminé vai a 2,15 → 3,225).
    let (_, max) = modelo("casa.glb").bounds();
    assert!((max.y - 3.225).abs() < 0.01, "{max}");

    let caixote = modelo("caixote.glb");
    assert_eq!(caixote.images.len(), 1);
    assert_eq!(caixote.images[0].size, UVec2::new(16, 16));
    assert!(caixote.images[0].nearest, "pixel art");
    assert_eq!(caixote.parts[0].image, Some(0));
    assert_eq!(caixote.triangle_count(), 12);
}

#[test]
fn modelos_gerados_tem_as_faces_para_fora() {
    // Volume com sinal: positivo quando os triângulos estão no sentido
    // anti-horário vistos de fora (o lado de fora é o que aparece).
    for nome in ["arvore.glb", "casa.glb", "pedra.glb", "heroi.glb", "caixote.glb"] {
        let m = modelo(nome);
        let mut volume = 0.0;
        for p in &m.parts {
            for t in p.mesh.indices.chunks(3) {
                let v = |i: u32| Vec3::from_array(p.mesh.vertices[i as usize].position);
                volume += v(t[0]).dot(v(t[1]).cross(v(t[2]))) / 6.0;
            }
        }
        assert!(volume > 0.05, "{nome}: volume {volume}");
    }
    // O caixote é um cubo de lado 1.
    let caixote = modelo("caixote.glb");
    let p = &caixote.parts[0];
    let mut volume = 0.0;
    for t in p.mesh.indices.chunks(3) {
        let v = |i: u32| Vec3::from_array(p.mesh.vertices[i as usize].position);
        volume += v(t[0]).dot(v(t[1]).cross(v(t[2]))) / 6.0;
    }
    assert!((volume - 1.0).abs() < 1e-4, "{volume}");
}

#[test]
fn gltf_com_arquivos_separados() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/modelos");
    let bytes = std::fs::read(dir.join("caixote.gltf")).unwrap();
    let mut pedidos = Vec::new();
    let m = ModelData::from_gltf(&bytes, &mut |uri| {
        pedidos.push(uri.to_string());
        std::fs::read(dir.join(uri)).ok()
    })
    .unwrap();
    assert_eq!(pedidos, ["caixote.bin", "madeira.png"]);
    assert_eq!(m.triangle_count(), 12);
    // Faltando um arquivo: o erro diz qual.
    let err = ModelData::from_gltf(&bytes, &mut |_| None).unwrap_err();
    assert!(err.to_string().contains("caixote.bin"), "{err}");
    // Lixo no lugar do modelo.
    assert!(ModelData::from_gltf(b"nada", &mut |_| None).is_err());
}

#[test]
fn modelo_aparece_com_textura() {
    let Some(mut h) = Harness::new() else { return };
    let data = modelo("caixote.glb");
    let model = h.renderer.create_model(&data);
    h.renderer
        .set_camera3d(Camera3D::looking_at(Vec3::new(0.0, 0.5, 2.0), Vec3::new(0.0, 0.5, 0.0)));
    h.renderer
        .draw_model(&model, Mat4::IDENTITY, Color::WHITE, true);
    let img = h.render(Color::BLACK);
    // A borda da tábua é escura e o meio é claro.
    let meio = pixel(&img, 33, 33);
    assert!(brightness(meio) > 200, "madeira clara: {meio:?}");
    assert!(meio[0] > meio[2], "cor de madeira: {meio:?}");
    assert_eq!(h.renderer.last_mesh_count(), 1);
}

/// Cubo no ar sobre um chão, sol de cima: a sombra cai bem embaixo dele.
fn cena_com_sombra(h: &mut Harness, cast: bool) -> Vec<u8> {
    let plane = h.renderer.create_mesh(&shapes::plane());
    let cube = h.renderer.create_mesh(&shapes::cube());
    // Câmera de lado e um pouco acima: o chão embaixo do cubo aparece na frente dele.
    h.renderer
        .set_camera3d(Camera3D::looking_at(Vec3::new(0.0, 6.0, 6.0), Vec3::ZERO));
    h.renderer.set_light3d(Light3D {
        sun_direction: Vec3::NEG_Y,
        sun_color: Color::rgb(0.7, 0.7, 0.7),
        sky: Color::rgb(0.3, 0.3, 0.3),
        ground: Color::rgb(0.3, 0.3, 0.3),
        fog: None,
    });
    h.renderer
        .draw_mesh(plane, Mat4::from_scale(Vec3::splat(20.0)), Color::WHITE);
    h.renderer.draw_mesh_with(
        cube,
        Mat4::from_translation(Vec3::new(0.0, 1.5, 2.0)),
        Color::RED,
        None,
        cast,
    );
    h.render(Color::BLACK)
}

#[test]
fn cubo_projeta_sombra_no_chao() {
    let Some(mut h) = Harness::new() else { return };
    // Onde o chão bem embaixo do cubo aparece (em z = 2, na frente da origem).
    let onde = Camera3D::looking_at(Vec3::new(0.0, 6.0, 6.0), Vec3::ZERO)
        .screen_point(Vec3::new(0.0, 0.0, 2.0), SIZE.as_vec2())
        .unwrap();
    let (x, y) = (onde.x as u32, onde.y as u32);
    let longe = pixel(&cena_com_sombra(&mut h, true), 4, 60);
    let sombra = pixel(&cena_com_sombra(&mut h, true), x, y);
    let sem = pixel(&cena_com_sombra(&mut h, false), x, y);
    assert!(
        brightness(sombra) + 60 < brightness(sem),
        "na sombra {sombra:?}, sem sombra {sem:?}"
    );
    assert_color(sem, longe, "sem sombra o chão é igual em toda parte");

    // Sombras desligadas no renderer: nada de sombra.
    h.renderer.set_shadows3d(Shadows {
        enabled: false,
        ..Shadows::default()
    });
    let desligada = pixel(&cena_com_sombra(&mut h, true), x, y);
    assert_color(desligada, sem, "sombras desligadas");
}

#[test]
fn ceu_em_degrade_com_sol() {
    let Some(mut h) = Harness::new() else { return };
    let cube = h.renderer.create_mesh(&shapes::cube());
    h.renderer.set_sky3d(Some(Sky3D {
        top: Color::rgb(0.0, 0.0, 1.0),
        horizon: Color::rgb(1.0, 1.0, 1.0),
        bottom: Color::rgb(0.0, 1.0, 0.0),
        sun_size: 0.0,
    }));
    h.renderer.set_light3d(Light3D {
        fog: Some(Fog {
            color: None,
            near: 1.0,
            far: 2.0,
        }),
        ..flat_light()
    });
    // Olhando para o horizonte.
    h.renderer.set_camera3d(Camera3D::looking_at(
        Vec3::new(0.0, 0.0, 10.0),
        Vec3::new(0.0, 0.0, 0.0),
    ));
    h.renderer
        .draw_mesh(cube, Mat4::from_translation(Vec3::new(0.0, 0.0, -30.0)), Color::RED);
    let img = h.render(Color::BLACK);
    let cima = pixel(&img, 32, 0);
    let baixo = pixel(&img, 32, 63);
    assert!(cima[2] > cima[0] + 40, "em cima puxa para o azul: {cima:?}");
    assert!(baixo[1] > baixo[2] + 40, "embaixo puxa para o verde: {baixo:?}");
    // O cubo longe, coberto de névoa, fica da cor do horizonte.
    assert_color(pixel(&img, 32, 32), [255, 255, 255, 255], "névoa = horizonte");

    // O disco do sol, na direção da luz.
    h.renderer.set_sky3d(Some(Sky3D {
        sun_size: 20.0,
        ..Sky3D::default()
    }));
    h.renderer.set_light3d(Light3D {
        sun_direction: Vec3::new(0.0, 0.0, 1.0), // vindo de -Z, em frente à câmera
        sun_color: Color::rgb(1.0, 0.9, 0.5),
        ..Light3D::default()
    });
    h.renderer
        .draw_mesh(cube, Mat4::from_translation(Vec3::new(0.0, -50.0, 0.0)), Color::RED);
    let img = h.render(Color::BLACK);
    let sol = pixel(&img, 32, 32);
    assert!(brightness(sol) > 650, "sol no meio: {sol:?}");
    assert!(brightness(pixel(&img, 2, 2)) < brightness(sol), "o resto do céu é mais escuro");
}
