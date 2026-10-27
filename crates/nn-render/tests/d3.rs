//! Testes do 3D: desenham malhas numa textura fora da tela e conferem os
//! pixels. Sem GPU, pulam (a menos que `NN_REQUIRE_GPU=1`).

use glam::{Mat4, UVec2, Vec2, Vec3};
use nn_core::Color;
use nn_render::d3::{
    Bloom, Camera3D, Cone3D, Fog, Glow, Lamp3D, Light3D, MAX_LAMPS, Material3D, Transform3, Wind3D,
    shapes,
};
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
        .map(|v| {
            v.color.map(|c| (c * 255.0).round() as u8)[..3]
                .try_into()
                .unwrap()
        })
        .collect();
    assert!(cores.contains(&[0x8B, 0x5A, 0x3C]), "tronco: {cores:?}");
    assert!(cores.contains(&[0x3E, 0x9B, 0x4F]), "copa");
    // As copas sobem pelo nó: a árvore vai até ~2,25 de altura.
    let (min, max) = arvore.bounds();
    assert!(
        min.y.abs() < 1e-4 && (max.y - 2.25).abs() < 0.01,
        "{min} {max}"
    );

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
    for nome in [
        "arvore.glb",
        "casa.glb",
        "pedra.glb",
        "heroi.glb",
        "caixote.glb",
    ] {
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
    h.renderer.set_camera3d(Camera3D::looking_at(
        Vec3::new(0.0, 0.5, 2.0),
        Vec3::new(0.0, 0.5, 0.0),
    ));
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
        stars: 0.0,
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
    h.renderer.draw_mesh(
        cube,
        Mat4::from_translation(Vec3::new(0.0, 0.0, -30.0)),
        Color::RED,
    );
    let img = h.render(Color::BLACK);
    let cima = pixel(&img, 32, 0);
    let baixo = pixel(&img, 32, 63);
    assert!(cima[2] > cima[0] + 40, "em cima puxa para o azul: {cima:?}");
    assert!(
        baixo[1] > baixo[2] + 40,
        "embaixo puxa para o verde: {baixo:?}"
    );
    // O cubo longe, coberto de névoa, fica da cor do horizonte.
    assert_color(
        pixel(&img, 32, 32),
        [255, 255, 255, 255],
        "névoa = horizonte",
    );

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
    h.renderer.draw_mesh(
        cube,
        Mat4::from_translation(Vec3::new(0.0, -50.0, 0.0)),
        Color::RED,
    );
    let img = h.render(Color::BLACK);
    let sol = pixel(&img, 32, 32);
    assert!(brightness(sol) > 650, "sol no meio: {sol:?}");
    assert!(
        brightness(pixel(&img, 2, 2)) < brightness(sol),
        "o resto do céu é mais escuro"
    );
}

// --- Malhas com ossos (Sprint 26) -------------------------------------------

#[test]
fn malha_com_ossos_segue_as_matrizes_dos_ossos() {
    use nn_render::d3::SkinVertex;
    let Some(mut h) = Harness::new() else { return };
    // Dois cubos: o da esquerda preso ao osso 0, o da direita ao osso 1.
    let mut mesh = nn_render::d3::Mesh::new();
    let mut skin = Vec::new();
    for (x, joint, color) in [(-1.0f32, 0u32, Color::RED), (1.0, 1, Color::BLUE)] {
        let mut c = shapes::cube();
        for v in &mut c.vertices {
            v.position[0] += x;
            v.color = [color.r, color.g, color.b, 1.0];
        }
        skin.extend(std::iter::repeat_n(
            SkinVertex::rigid(joint),
            c.vertices.len(),
        ));
        mesh.append(&c);
    }
    let id = h.renderer.create_mesh_skinned(&mesh, &skin);
    h.renderer
        .set_camera3d(Camera3D::looking_at(Vec3::new(0.0, 0.0, 6.0), Vec3::ZERO));
    // Parado: vermelho à esquerda, azul à direita.
    let parado = [Mat4::IDENTITY, Mat4::IDENTITY];
    h.renderer
        .draw_mesh_skinned(id, Mat4::IDENTITY, Color::WHITE, None, true, &parado);
    let img = h.render(Color::BLACK);
    assert_color(pixel(&img, 20, 32), [255, 0, 0, 255], "vermelho à esquerda");
    assert_color(pixel(&img, 44, 32), [0, 0, 255, 255], "azul à direita");
    // O osso 1 sobe 2: o azul sai do meio; o vermelho fica.
    let subiu = [
        Mat4::IDENTITY,
        Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0)),
    ];
    h.renderer
        .draw_mesh_skinned(id, Mat4::IDENTITY, Color::WHITE, None, true, &subiu);
    let img = h.render(Color::BLACK);
    assert_color(pixel(&img, 20, 32), [255, 0, 0, 255], "vermelho parado");
    assert_color(pixel(&img, 44, 32), [0, 0, 0, 255], "o azul subiu");
    assert!(pixel(&img, 44, 12)[2] > 200, "azul lá em cima");
    // Vários desenhos com poses diferentes no mesmo frame (e a matriz do
    // modelo vale junto).
    for k in 0..6 {
        let x = Mat4::from_translation(Vec3::new(0.0, -0.3 * k as f32, 0.0));
        h.renderer
            .draw_mesh_skinned(id, x, Color::WHITE, None, true, &[Mat4::IDENTITY, x]);
    }
    let img = h.render(Color::BLACK);
    assert_eq!(h.renderer.last_mesh_count(), 6);
    assert!(brightness(pixel(&img, 44, 40)) > 0);
}

fn modelo_ilha(nome: &str) -> ModelData {
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jogos/estrelas3d/modelos");
    let bytes = std::fs::read(dir.join(nome)).unwrap();
    ModelData::from_gltf(&bytes, &mut |_| None).unwrap()
}

#[test]
fn heroi_gerado_tem_esqueleto_e_animacoes() {
    let h = modelo_ilha("heroi.glb");
    let s = h.skeleton.as_ref().expect("esqueleto");
    let nomes: Vec<&str> = s.animations.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(
        nomes,
        [
            "parado", "andar", "correr", "pular", "cair", "acenar", "golpe", "atirar"
        ]
    );
    assert_eq!(s.joints.len(), 12);
    assert!(
        h.parts
            .iter()
            .all(|p| p.skin.len() == p.mesh.vertices.len())
    );
    // Mesmo tamanho de antes: 1,8 m, os pés no chão.
    let (min, max) = h.bounds();
    assert!(
        min.y.abs() < 1e-4 && (max.y - 1.8).abs() < 1e-3,
        "{min} {max}"
    );
    // Em descanso, os ossos não mexem nos vértices.
    for m in s.joint_matrices(&s.rest_pose()) {
        assert!(m.abs_diff_eq(Mat4::IDENTITY, 1e-4), "{m}");
    }
    // Andando, no primeiro quarto do passo, o pé esquerdo vai para a frente
    // (+Z) e o direito para trás.
    let andar = s.animation("andar").unwrap();
    assert!((s.animations[andar].duration - 0.8).abs() < 1e-4);
    let pose = s.pose(andar, 0.0);
    let pe_e = s
        .node_matrix(&pose, "canela_e")
        .unwrap()
        .transform_point3(Vec3::new(0.0, -0.2, 0.0));
    let pe_d = s
        .node_matrix(&pose, "canela_d")
        .unwrap()
        .transform_point3(Vec3::new(0.0, -0.2, 0.0));
    assert!(pe_e.z > 0.1 && pe_d.z < -0.1, "pés {pe_e} {pe_d}");
    // As mãos: em descanso, na ponta dos braços; no golpe, a direita passa
    // por cima da cabeça e desce na frente do corpo.
    let mao = |pose: &nn_render::d3::Pose| {
        s.node_matrix(pose, "mao_d")
            .unwrap()
            .transform_point3(Vec3::ZERO)
    };
    let descanso = mao(&s.rest_pose());
    assert!(
        descanso.abs_diff_eq(Vec3::new(-0.41, 0.53, 0.0), 1e-4),
        "{descanso}"
    );
    let golpe = s.animation("golpe").unwrap();
    let alto = mao(&s.pose(golpe, 0.12));
    let frente = mao(&s.pose(golpe, 0.22));
    assert!(alto.y > 1.35 && alto.z < 0.1, "levanta: {alto}");
    assert!(
        frente.z > 0.3 && frente.y < 1.0,
        "desce na frente: {frente}"
    );
    let atirar = s.animation("atirar").unwrap();
    let apontando = mao(&s.pose(atirar, 0.1));
    assert!(
        apontando.z > 0.4 && (apontando.y - 1.0).abs() < 0.15,
        "{apontando}"
    );
    // O bicho também.
    let b = modelo_ilha("bicho.glb");
    let names: Vec<String> = b
        .skeleton
        .unwrap()
        .animations
        .into_iter()
        .map(|a| a.name)
        .collect();
    assert_eq!(names, ["parado", "andar", "golpe"]);
}

#[test]
fn heroi_animado_mexe_os_pixels_e_faz_sombra() {
    let Some(mut h) = Harness::new() else { return };
    let data = modelo_ilha("heroi.glb");
    let model = h.renderer.create_model(&data);
    let s = model.skeleton.clone().unwrap();
    h.renderer.set_camera3d(Camera3D::looking_at(
        Vec3::new(0.0, 1.0, 3.0),
        Vec3::new(0.0, 0.9, 0.0),
    ));
    // De frente: parado, os braços ficam juntos do corpo; acenando, o braço
    // direito sobe para o lado.
    h.renderer
        .draw_model(&model, Mat4::IDENTITY, Color::WHITE, true);
    let parado = h.render(Color::BLACK);
    let acenar = s.animation("acenar").unwrap();
    let joints = s.joint_matrices(&s.pose(acenar, 0.3));
    h.renderer
        .draw_model_posed(&model, Mat4::IDENTITY, Color::WHITE, true, &joints);
    let acenando = h.render(Color::BLACK);
    if let Some(d) = std::env::var_os("NN_TELAS") {
        for (nome, img) in [("heroi-parado", &parado), ("heroi-acenando", &acenando)] {
            image::RgbaImage::from_raw(64, 64, img.clone())
                .unwrap()
                .save(std::path::Path::new(&d).join(format!("{nome}.png")))
                .unwrap();
        }
    }
    let acesos = |img: &[u8], y0: u32, y1: u32| {
        (y0..y1)
            .flat_map(|y| (0..64).map(move |x| (x, y)))
            .filter(|&(x, y)| brightness(pixel(img, x, y)) > 0)
            .count()
    };
    // O herói inteiro aparece nas duas, e o braço levantado muda a imagem.
    assert!(acesos(&parado, 0, 64) > 200);
    let diferentes = (0..64 * 64)
        .filter(|i| {
            let (x, y) = (i % 64, i / 64);
            pixel(&parado, x, y) != pixel(&acenando, x, y)
        })
        .count();
    assert!(diferentes > 20, "o braço mexeu: {diferentes} pixels");
}

#[test]
fn parte_sem_ossos_anima_pelo_no() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/modelos");
    let bytes = std::fs::read(dir.join("roda.glb")).unwrap();
    let m = ModelData::from_gltf(&bytes, &mut |_| None).unwrap();
    let s = m.skeleton.as_ref().expect("animado");
    assert_eq!(s.joints.len(), 1, "o nó da parte vira um osso");
    // A ponta da barra (x = 1, na altura do eixo) gira 90° em Y na metade.
    let ponta = Vec3::new(1.0, 1.0, 0.0);
    let i = s.animation("girar").unwrap();
    let j = s.joint_matrices(&s.pose(i, 0.5));
    let p = j[0].transform_point3(ponta);
    assert!(p.abs_diff_eq(Vec3::new(0.0, 1.0, -1.0), 1e-4), "{p}");
    // Em descanso, parada.
    let j = s.joint_matrices(&s.rest_pose());
    assert!(j[0].transform_point3(ponta).abs_diff_eq(ponta, 1e-4));
}

// --- Partículas 3D (Sprint 27) ----------------------------------------------

#[test]
fn particulas_3d_viram_para_a_camera_e_respeitam_a_profundidade() {
    use nn_render::d3::{ParticleConfig3, ParticleEmitter3};
    use nn_render::{Blend, Rect};
    let Some(mut h) = Harness::new() else { return };
    let white = h.renderer.white_texture();
    let cube = h.renderer.create_mesh(&shapes::cube());
    // De frente e de cima: o quadrado aparece do mesmo tamanho (virado para
    // a câmera).
    for eye in [Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 5.0, 0.01)] {
        h.renderer
            .set_camera3d(Camera3D::looking_at(eye, Vec3::ZERO));
        h.renderer.draw_billboard(
            white,
            Vec3::ZERO,
            1.0,
            0.0,
            Color::RED,
            Rect::FULL,
            Blend::Alpha,
        );
        let img = h.render(Color::BLACK);
        assert_color(pixel(&img, 32, 32), [255, 0, 0, 255], "vermelho no meio");
        assert_color(pixel(&img, 4, 32), [0, 0, 0, 255], "pequeno");
    }
    // Atrás de um cubo: escondida; na frente: aparece por cima.
    h.renderer
        .set_camera3d(Camera3D::looking_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO));
    h.renderer.draw_mesh(cube, Mat4::IDENTITY, Color::BLUE);
    h.renderer.draw_billboard(
        white,
        Vec3::new(0.0, 0.0, -2.0),
        0.6,
        0.0,
        Color::RED,
        Rect::FULL,
        Blend::Alpha,
    );
    let img = h.render(Color::BLACK);
    assert_color(pixel(&img, 32, 32), [0, 0, 255, 255], "atrás do cubo");
    h.renderer.draw_mesh(cube, Mat4::IDENTITY, Color::BLUE);
    h.renderer.draw_billboard(
        white,
        Vec3::new(0.0, 0.0, 2.0),
        0.3,
        0.0,
        Color::RED,
        Rect::FULL,
        Blend::Alpha,
    );
    let img = h.render(Color::BLACK);
    assert_color(pixel(&img, 32, 32), [255, 0, 0, 255], "na frente do cubo");
    // Somar luz: duas metades de verde somam um verde cheio.
    for _ in 0..2 {
        h.renderer.draw_billboard(
            white,
            Vec3::ZERO,
            1.0,
            0.0,
            Color::rgba(0.0, 1.0, 0.0, 0.5),
            Rect::FULL,
            Blend::Add,
        );
    }
    let img = h.render(Color::BLACK);
    let g = pixel(&img, 32, 32)[1];
    assert!(g > 240, "aditivo soma: {g}");
    // Um emissor inteiro.
    let mut e = ParticleEmitter3::new(ParticleConfig3 {
        speed: (0.0, 0.0),
        size: (0.5, 0.5),
        lifetime: (5.0, 5.0),
        colors: vec![Color::WHITE],
        ..Default::default()
    });
    e.burst(10);
    e.update(0.1);
    e.draw(&mut h.renderer);
    let img = h.render(Color::BLACK);
    assert!(brightness(pixel(&img, 32, 32)) > 600);
}

// --- Terreno e água (Sprint 28) -----------------------------------------------

#[test]
fn terreno_desenha_morro_e_agua_transparente() {
    use nn_render::d3::{BrushOp, DEFAULT_LAYER_COLORS, TerrainData};
    let Some(mut h) = Harness::new() else { return };
    let mut t = TerrainData::flat(32, Vec2::new(20.0, 20.0), 10.0);
    for _ in 0..8 {
        t.brush(BrushOp::Raise, Vec2::ZERO, 5.0, 0.5);
    }
    t.paint(2, Vec2::ZERO, 2.0, 1.0);
    let mesh = h.renderer.create_mesh(&t.mesh(&DEFAULT_LAYER_COLORS));
    // De lado, na altura do chão: o morro aparece no meio, o chão plano não.
    h.renderer.set_camera3d(Camera3D::looking_at(
        Vec3::new(0.0, 0.5, 15.0),
        Vec3::new(0.0, 0.5, 0.0),
    ));
    h.renderer.draw_mesh(mesh, Mat4::IDENTITY, Color::WHITE);
    let img = h.render(Color::BLACK);
    assert!(brightness(pixel(&img, 32, 26)) > 0, "o morro sobe no meio");
    assert_eq!(brightness(pixel(&img, 32, 4)), 0, "céu vazio em cima");
    // Água meio transparente por cima de um chão vermelho: mistura as duas.
    let plane = h.renderer.create_mesh(&shapes::plane());
    h.renderer
        .set_camera3d(Camera3D::looking_at(Vec3::new(0.0, 8.0, 0.01), Vec3::ZERO));
    // A água vem antes na fila: mesmo assim o chão aparece por baixo.
    h.renderer.draw_mesh(
        plane,
        Mat4::from_scale_rotation_translation(
            Vec3::splat(20.0),
            glam::Quat::IDENTITY,
            Vec3::new(0.0, 1.0, 0.0),
        ),
        Color::rgba(0.0, 0.0, 1.0, 0.5),
    );
    h.renderer
        .draw_mesh(plane, Mat4::from_scale(Vec3::splat(20.0)), Color::RED);
    let img = h.render(Color::BLACK);
    let p = pixel(&img, 32, 32);
    assert!(p[0] > 90 && p[2] > 90, "vermelho e azul misturados: {p:?}");
}

// --- Desempenho (Sprint 32) ------------------------------------------------

#[test]
fn so_o_que_a_camera_ve_vai_para_a_gpu() {
    let Some(mut h) = Harness::new() else { return };
    let cube = h.renderer.create_mesh(&shapes::cube());
    h.renderer
        .set_camera3d(Camera3D::looking_at(Vec3::new(0.0, 2.0, 10.0), Vec3::ZERO));
    h.renderer.set_shadows3d(Shadows {
        enabled: true,
        distance: 6.0,
        ..Shadows::default()
    });
    // Uma fila de cubos de -100 a 100 em X (só os do meio aparecem) e outros
    // atrás da câmera.
    for i in -50..=50 {
        h.renderer.draw_mesh(
            cube,
            Mat4::from_translation(Vec3::new(i as f32 * 2.0, 0.0, 0.0)),
            Color::RED,
        );
    }
    for i in 0..20 {
        h.renderer.draw_mesh(
            cube,
            Mat4::from_translation(Vec3::new(i as f32, 0.0, 40.0)),
            Color::RED,
        );
    }
    let img = h.render(Color::BLACK);
    let s = h.renderer.last_stats3();
    assert_eq!(s.meshes, 121);
    assert_eq!(h.renderer.last_mesh_count(), 121);
    assert!(
        (4..=12).contains(&s.visible),
        "só os da frente da câmera: {s:?}"
    );
    // Sombras: só os perto de onde a câmera olha (6 m para cada lado).
    assert!((3..=10).contains(&s.shadow), "sombras: {s:?}");
    // Uma chamada para os vistos e uma para as sombras (a mesma malha).
    assert_eq!(s.draw_calls, 2, "{s:?}");
    assert_color(
        pixel(&img, 32, 32),
        [255, 0, 0, 255],
        "o cubo do meio aparece",
    );
    // Uma barra comprida, com o centro fora da tela mas a ponta dentro,
    // aparece.
    h.renderer.draw_mesh(
        cube,
        Mat4::from_scale_rotation_translation(
            Vec3::new(24.0, 1.0, 1.0),
            glam::Quat::IDENTITY,
            Vec3::new(14.0, 0.0, 0.0),
        ),
        Color::GREEN,
    );
    let img = h.render(Color::BLACK);
    assert_eq!(h.renderer.last_stats3().visible, 1);
    let p = Camera3D::looking_at(Vec3::new(0.0, 2.0, 10.0), Vec3::ZERO)
        .screen_point(Vec3::new(4.0, 0.0, 0.5), SIZE.as_vec2())
        .unwrap();
    assert_color(
        pixel(&img, p.x as u32, p.y as u32),
        [0, 255, 0, 255],
        "a ponta da barra",
    );
}

// --- Lâmpadas, holofotes e brilho -------------------------------------------

/// Um chão grande no escuro (sem sol nem luz ambiente), visto de cima: a
/// 10 m de altura, cada pixel tem uns 18 cm.
fn chao_no_escuro(h: &mut Harness) {
    let plane = h.renderer.create_mesh(&shapes::plane());
    h.renderer
        .set_camera3d(Camera3D::looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::ZERO));
    h.renderer.set_light3d(Light3D {
        sun_color: Color::BLACK,
        sky: Color::BLACK,
        ground: Color::BLACK,
        ..Light3D::default()
    });
    h.renderer
        .draw_mesh(plane, Mat4::from_scale(Vec3::splat(30.0)), Color::WHITE);
}

#[test]
fn lampada_ilumina_em_volta_ate_o_alcance() {
    let Some(mut h) = Harness::new() else { return };
    chao_no_escuro(&mut h);
    let mut lamp = Lamp3D::at(Vec3::new(0.0, 1.0, 0.0));
    lamp.color = Color::rgb(1.0, 0.5, 0.0);
    lamp.range = 4.0;
    h.renderer.add_lamp3d(lamp);
    let img = h.render(Color::BLACK);
    let meio = pixel(&img, 32, 32);
    // Embaixo da lâmpada: claro, com a cor dela (laranja).
    assert!(meio[0] > 150 && meio[1] > 40 && meio[2] < 10, "{meio:?}");
    // A 1,5 m: mais fraco que embaixo, mas aceso.
    let perto = pixel(&img, 32 + 8, 32);
    assert!(
        brightness(perto) > 30 && brightness(perto) < brightness(meio),
        "{perto:?}"
    );
    // Longe (mais que o alcance): escuro.
    assert_color(pixel(&img, 2, 2), [0, 0, 0, 255], "longe da lâmpada");
    assert_eq!(h.renderer.last_stats3().lamps, 1);
    // No frame seguinte, sem pedir de novo, a lâmpada some.
    chao_no_escuro(&mut h);
    let img = h.render(Color::BLACK);
    assert_color(pixel(&img, 32, 32), [0, 0, 0, 255], "sem lâmpada");
}

#[test]
fn holofote_ilumina_so_dentro_do_cone() {
    let Some(mut h) = Harness::new() else { return };
    // Um holofote a 3 m, apontando para baixo, com 20° de abertura (meia):
    // o círculo de luz no chão tem ~1,1 m de raio.
    let holofote = |h: &mut Harness, cone: bool| {
        chao_no_escuro(h);
        let mut lamp = Lamp3D::at(Vec3::new(0.0, 3.0, 0.0));
        lamp.range = 10.0;
        lamp.intensity = 2.0;
        if cone {
            lamp.cone = Some(Cone3D {
                direction: Vec3::NEG_Y,
                angle: 20.0,
                softness: 0.2,
            });
        }
        h.renderer.add_lamp3d(lamp);
        h.render(Color::BLACK)
    };
    let img = holofote(&mut h, true);
    assert!(
        brightness(pixel(&img, 32, 32)) > 300,
        "{:?}",
        pixel(&img, 32, 32)
    );
    // A 2,5 m do centro: fora do cone, escuro.
    assert_color(pixel(&img, 32 + 14, 32), [0, 0, 0, 255], "fora do cone");
    // A mesma luz sem cone acende ali.
    let img = holofote(&mut h, false);
    assert!(
        brightness(pixel(&img, 32 + 14, 32)) > 60,
        "{:?}",
        pixel(&img, 32 + 14, 32)
    );
}

#[test]
fn brilho_aparece_no_escuro() {
    let Some(mut h) = Harness::new() else { return };
    let cube = h.renderer.create_mesh(&shapes::cube());
    let desenha = |h: &mut Harness, glow: Glow| {
        chao_no_escuro(h);
        h.renderer.draw_mesh_glow(
            cube,
            Mat4::from_translation(Vec3::new(0.0, 0.5, 0.0)),
            Color::rgb(0.0, 1.0, 0.0),
            None,
            true,
            glow,
        );
        h.render(Color::BLACK)
    };
    // Sem brilho, no escuro: preto.
    let img = desenha(&mut h, Glow::NONE);
    assert_color(pixel(&img, 32, 32), [0, 0, 0, 255], "sem brilho");
    // Com a própria cor brilhando: verde, sem luz nenhuma.
    let img = desenha(&mut h, Glow::own(1.0));
    assert_color(pixel(&img, 32, 32), [0, 255, 0, 255], "brilho próprio");
    // O chão em volta continua escuro (o brilho não ilumina).
    assert_color(pixel(&img, 2, 2), [0, 0, 0, 255], "chão");
    // Uma cor que brilha por cima (a emissão de um .glb).
    let img = desenha(
        &mut h,
        Glow {
            amount: 0.0,
            color: Color::rgb(1.0, 0.0, 0.0),
        },
    );
    assert_color(pixel(&img, 32, 32), [255, 0, 0, 255], "cor emitida");
}

#[test]
fn com_muitas_lampadas_valem_as_mais_perto_que_a_camera_ve() {
    let Some(mut h) = Harness::new() else { return };
    chao_no_escuro(&mut h);
    // 50 lâmpadas na frente da câmera e 10 bem longe, atrás dela.
    for i in 0..50 {
        let mut l = Lamp3D::at(Vec3::new((i % 10) as f32 - 4.5, 0.5, (i / 10) as f32 - 2.0));
        l.range = 1.0;
        h.renderer.add_lamp3d(l);
    }
    for i in 0..10 {
        h.renderer
            .add_lamp3d(Lamp3D::at(Vec3::new(i as f32, 0.5, 200.0)));
    }
    h.render(Color::BLACK);
    assert_eq!(h.renderer.last_stats3().lamps, MAX_LAMPS as u32);
    // Só as de trás: nenhuma vale.
    chao_no_escuro(&mut h);
    for i in 0..10 {
        h.renderer
            .add_lamp3d(Lamp3D::at(Vec3::new(i as f32, 0.5, 200.0)));
    }
    h.render(Color::BLACK);
    assert_eq!(h.renderer.last_stats3().lamps, 0);
}

#[test]
fn modelos_que_brilham_trazem_a_cor_da_emissao_e_aparecem_no_escuro() {
    // A chama da tocha, o vidro do lampião e os cristais vêm do arquivo
    // com emissão (o cabo e o ferro, sem).
    for (nome, partes) in [("tocha.glb", 2), ("lampiao.glb", 1), ("cristal.glb", 2)] {
        let m = modelo_ilha(nome);
        let acesas = m
            .parts
            .iter()
            .filter(|p| p.emissive != Color::BLACK)
            .count();
        assert_eq!(acesas, partes, "{nome}");
        assert!(m.parts.iter().any(|p| p.emissive == Color::BLACK), "{nome}");
    }
    let Some(mut h) = Harness::new() else { return };
    let data = modelo_ilha("tocha.glb");
    let tocha = h.renderer.create_model(&data);
    // (Sem o halo, para ver só a emissão.)
    h.renderer.set_bloom3d(Bloom {
        enabled: false,
        ..Bloom::default()
    });
    h.renderer.set_light3d(Light3D {
        sun_color: Color::BLACK,
        sky: Color::BLACK,
        ground: Color::BLACK,
        ..Light3D::default()
    });
    h.renderer.set_camera3d(Camera3D::looking_at(
        Vec3::new(0.0, 0.7, 1.6),
        Vec3::new(0.0, 0.7, 0.0),
    ));
    h.renderer
        .draw_model(&tocha, Mat4::IDENTITY, Color::WHITE, true);
    let img = h.render(Color::BLACK);
    if let Some(d) = std::env::var_os("NN_TELAS") {
        image::RgbaImage::from_raw(64, 64, img.clone())
            .unwrap()
            .save(std::path::Path::new(&d).join("tocha-no-escuro.png"))
            .unwrap();
    }
    // A chama (em cima) acesa e laranja; o cabo (embaixo), preto.
    let (mut chama, mut cabo) = (0, 0);
    for y in 0..64 {
        for x in 0..64 {
            let p = pixel(&img, x, y);
            if p[0] > 200 && p[1] > 80 && p[2] < 80 {
                chama += 1;
                assert!(y < 32, "a chama fica em cima: {x}, {y}");
            }
            if y > 40 && brightness(p) > 0 {
                cabo += 1;
            }
        }
    }
    assert!(chama > 10, "chama: {chama}");
    assert_eq!(cabo, 0, "o cabo no escuro");
}

// --- Água e vento ------------------------------------------------------------

#[test]
fn agua_mexe_com_o_tempo_e_reflete_o_ceu_de_lado() {
    let Some(mut h) = Harness::new() else { return };
    let plane = h.renderer.create_mesh(&shapes::plane());
    h.renderer.set_sky3d(Some(Sky3D {
        top: Color::rgb(0.2, 0.4, 1.0),
        horizon: Color::rgb(1.0, 1.0, 1.0),
        bottom: Color::rgb(0.2, 0.4, 1.0),
        sun_size: 0.0,
        stars: 0.0,
    }));
    h.renderer.set_light3d(Light3D {
        sun_color: Color::rgb(0.6, 0.6, 0.6),
        sky: Color::rgb(0.3, 0.3, 0.3),
        ground: Color::rgb(0.3, 0.3, 0.3),
        ..Light3D::default()
    });
    // Baixinho, olhando a água até o horizonte.
    h.renderer.set_camera3d(Camera3D::looking_at(
        Vec3::new(0.0, 1.5, 10.0),
        Vec3::new(0.0, 0.0, -10.0),
    ));
    let desenha = |h: &mut Harness, t: f32, water: bool| {
        h.renderer.set_time3d(Some(t));
        h.renderer.draw_mesh_material(
            plane,
            Mat4::from_scale(Vec3::splat(200.0)),
            Color::rgb(0.05, 0.2, 0.3),
            None,
            false,
            Material3D {
                water,
                ..Material3D::default()
            },
        );
        h.render(Color::BLACK)
    };
    let a = desenha(&mut h, 0.0, true);
    let b = desenha(&mut h, 1.5, true);
    let diferentes = (0..64 * 64)
        .filter(|i| a[i * 4..i * 4 + 3] != b[i * 4..i * 4 + 3])
        .count();
    assert!(diferentes > 200, "as ondas mexem: {diferentes} pixels");
    // Longe (olhando de lado), a água fica mais clara: o céu refletido.
    let perto = brightness(pixel(&a, 32, 62));
    let longe = brightness(pixel(&a, 32, 36));
    assert!(longe > perto + 60, "longe {longe}, perto {perto}");
    // Sem o material de água, a mesma cor em todo canto (sem céu).
    let c = desenha(&mut h, 0.0, false);
    let (p, l) = (brightness(pixel(&c, 32, 62)), brightness(pixel(&c, 32, 36)));
    assert!(p.abs_diff(l) < 10, "{p} {l}");
}

#[test]
fn vento_balanca_o_alto_e_a_base_fica_parada() {
    let Some(mut h) = Harness::new() else { return };
    let cube = h.renderer.create_mesh(&shapes::cube());
    h.renderer.set_camera3d(Camera3D::looking_at(
        Vec3::new(0.0, 2.0, 9.0),
        Vec3::new(0.0, 2.0, 0.0),
    ));
    // Um poste alto (a base no chão, a origem do objeto no pé).
    let poste = Mat4::from_translation(Vec3::ZERO)
        * Mat4::from_scale(Vec3::new(0.6, 4.0, 0.6))
        * Mat4::from_translation(Vec3::new(0.0, 0.5, 0.0));
    let desenha = |h: &mut Harness, forca: f32| {
        h.renderer.set_time3d(Some(0.3));
        h.renderer.set_wind3d(Wind3D {
            direction: glam::Vec2::new(1.0, 0.0),
            strength: forca,
        });
        h.renderer.draw_mesh_material(
            cube,
            poste,
            Color::RED,
            None,
            false,
            Material3D {
                sway: 1.0,
                ..Material3D::default()
            },
        );
        h.render(Color::BLACK)
    };
    let parado = desenha(&mut h, 0.0);
    let ventando = desenha(&mut h, 3.0);
    let linha = |img: &[u8], y: u32| -> Vec<u32> {
        (0..64).filter(|&x| pixel(img, x, y)[0] > 100).collect()
    };
    // No alto, a ponta foi para a direita (+X, para onde o vento sopra).
    let (a, b) = (linha(&parado, 21), linha(&ventando, 21));
    assert!(!a.is_empty() && !b.is_empty());
    assert!(b[0] >= a[0] + 2, "o alto balançou: {a:?} → {b:?}");
    // Embaixo, no pé, igual.
    assert!(!linha(&parado, 43).is_empty());
    assert_eq!(linha(&parado, 43), linha(&ventando, 43));
}

#[test]
fn halo_so_em_volta_do_que_brilha() {
    let Some(mut h) = Harness::new() else { return };
    let cube = h.renderer.create_mesh(&shapes::cube());
    h.renderer
        .set_camera3d(Camera3D::looking_at(Vec3::new(0.0, 0.0, 6.0), Vec3::ZERO));
    let desenha = |h: &mut Harness, glow: f32, sol: bool, halo: bool| {
        h.renderer.set_bloom3d(Bloom {
            enabled: halo,
            strength: 1.0,
        });
        h.renderer.set_light3d(Light3D {
            sun_direction: Vec3::new(0.0, 0.0, -1.0),
            sun_color: if sol { Color::WHITE } else { Color::BLACK },
            sky: Color::BLACK,
            ground: Color::BLACK,
            ..Light3D::default()
        });
        h.renderer.draw_mesh_glow(
            cube,
            Mat4::IDENTITY,
            Color::rgb(1.0, 0.8, 0.2),
            None,
            false,
            Glow::own(glow),
        );
        h.render(Color::BLACK)
    };
    // O cubo ocupa o meio (uns 18 pixels); 4 pixels para fora da borda.
    let fora = |img: &[u8]| brightness(pixel(img, 32, 19)) + brightness(pixel(img, 19, 32));
    let com_halo = desenha(&mut h, 1.0, false, true);
    let sem_halo = desenha(&mut h, 1.0, false, false);
    assert!(
        brightness(pixel(&com_halo, 32, 32)) > 400,
        "o cubo brilhando"
    );
    assert_eq!(fora(&sem_halo), 0, "sem halo, fora fica preto");
    assert!(fora(&com_halo) > 20, "o halo em volta: {}", fora(&com_halo));
    // Um cubo claro só pela luz do sol (sem brilho próprio): sem halo.
    let claro = desenha(&mut h, 0.0, true, true);
    assert!(brightness(pixel(&claro, 32, 32)) > 400, "aceso pelo sol");
    assert_eq!(fora(&claro), 0, "o que não brilha não ganha halo");
    // Sem nada brilhando, o passe do halo nem roda.
    assert!(!h.renderer.last_stats3().halo, "pulado sem brilho");
    desenha(&mut h, 1.0, false, true);
    assert!(h.renderer.last_stats3().halo);
    // O disco do sol na tela também brilha; atrás da câmera, não.
    h.renderer.set_sky3d(Some(Sky3D::default()));
    for (sol, visto) in [
        (Vec3::new(0.0, 0.0, 1.0), true),
        (Vec3::new(0.0, 0.0, -1.0), false),
    ] {
        h.renderer.set_light3d(Light3D {
            sun_direction: sol,
            ..Light3D::default()
        });
        h.renderer.draw_mesh(cube, Mat4::IDENTITY, Color::WHITE);
        h.render(Color::BLACK);
        assert_eq!(h.renderer.last_stats3().halo, visto, "sol {sol}");
    }
}

/// O brilho do arquivo (emissive) pega a cor do objeto: um modelo branco que
/// brilha, tingido de vermelho, brilha vermelho (no escuro).
#[test]
fn cor_do_objeto_tinge_o_brilho_do_modelo() {
    let Some(mut h) = Harness::new() else { return };
    h.renderer.set_light3d(Light3D {
        sun_color: Color::BLACK,
        sky: Color::BLACK,
        ground: Color::BLACK,
        ..Light3D::default()
    });
    let data = ModelData {
        parts: vec![nn_render::d3::ModelPart {
            mesh: shapes::cube(),
            image: None,
            skin: Vec::new(),
            emissive: Color::WHITE,
            ..Default::default()
        }],
        images: Vec::new(),
        skeleton: None,
    };
    let model = h.renderer.create_model(&data);
    h.renderer
        .set_camera3d(Camera3D::looking_at(Vec3::new(0.0, 0.0, 3.0), Vec3::ZERO));
    h.renderer
        .draw_model(&model, Mat4::IDENTITY, Color::RED, false);
    let img = h.render(Color::BLACK);
    let p = pixel(&img, 32, 32);
    assert!(
        p[0] > 200 && p[1] < 40 && p[2] < 40,
        "brilho vermelho: {p:?}"
    );
}

// --- Folhagem: recorte por transparência e duas faces (Sprint 76) --------

/// Uma imagem de 2x1: a metade da esquerda branca, a da direita transparente.
fn meia_folha(h: &mut Harness) -> nn_render::TextureId {
    h.renderer.create_texture(
        UVec2::new(2, 1),
        &[255, 255, 255, 255, 255, 255, 255, 0],
        nn_render::TextureOptions::PIXEL_ART,
    )
}

#[test]
fn folha_recortada_deixa_ver_o_que_esta_atras() {
    let Some(mut h) = Harness::new() else { return };
    let folha = meia_folha(&mut h);
    let plane = h.renderer.create_mesh(&shapes::plane());
    let cube = h.renderer.create_mesh(&shapes::cube());
    h.renderer
        .set_camera3d(Camera3D::looking_at(Vec3::new(0.0, 0.0, 3.0), Vec3::ZERO));
    let cena = |h: &mut Harness, cutout: f32| {
        // A folha em pé, virada para a câmera, e um cubo vermelho atrás.
        h.renderer.draw_mesh_material(
            plane,
            Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2) * Mat4::from_scale(Vec3::splat(2.0)),
            Color::WHITE,
            Some(folha),
            false,
            Material3D {
                cutout,
                ..Material3D::default()
            },
        );
        h.renderer.draw_mesh_with(
            cube,
            Mat4::from_translation(Vec3::new(0.0, 0.0, -2.5)) * Mat4::from_scale(Vec3::splat(3.0)),
            Color::RED,
            None,
            false,
        );
        h.render(Color::BLACK)
    };
    let com = cena(&mut h, 0.5);
    assert_color(
        pixel(&com, 20, 32),
        [255, 255, 255, 255],
        "a metade opaca da folha",
    );
    assert_color(
        pixel(&com, 44, 32),
        [255, 0, 0, 255],
        "pelo buraco, o cubo atrás",
    );
    // Sem o recorte, a parte transparente esconde o cubo (como antes).
    let sem = cena(&mut h, 0.0);
    assert_color(
        pixel(&sem, 44, 32),
        [0, 0, 0, 255],
        "sem recorte, o buraco tampa o cubo",
    );
}

#[test]
fn folha_recortada_faz_sombra_com_os_buracos() {
    let Some(mut h) = Harness::new() else { return };
    let folha = meia_folha(&mut h);
    let plane = h.renderer.create_mesh(&shapes::plane());
    let floor = h.renderer.create_mesh(&shapes::plane());
    let eye = Vec3::new(0.0, 6.0, 6.0);
    h.renderer
        .set_camera3d(Camera3D::looking_at(eye, Vec3::ZERO));
    h.renderer.set_light3d(Light3D {
        sun_direction: Vec3::NEG_Y,
        sun_color: Color::rgb(0.7, 0.7, 0.7),
        sky: Color::rgb(0.3, 0.3, 0.3),
        ground: Color::rgb(0.3, 0.3, 0.3),
        fog: None,
    });
    let cena = |h: &mut Harness, cutout: f32| {
        h.renderer
            .draw_mesh(floor, Mat4::from_scale(Vec3::splat(20.0)), Color::WHITE);
        // A folha deitada no ar, sobre o ponto (0, 0, 2).
        h.renderer.draw_mesh_material(
            plane,
            Mat4::from_translation(Vec3::new(0.0, 1.5, 2.0)),
            Color::WHITE,
            Some(folha),
            true,
            Material3D {
                cutout,
                ..Material3D::default()
            },
        );
        h.render(Color::BLACK)
    };
    let onde = |x: f32| {
        let p = Camera3D::looking_at(eye, Vec3::ZERO)
            .screen_point(Vec3::new(x, 0.0, 2.0), SIZE.as_vec2())
            .unwrap();
        (p.x as u32, p.y as u32)
    };
    let ((ex, ey), (dx, dy)) = (onde(-0.25), onde(0.25));
    let com = cena(&mut h, 0.5);
    let (sombra, luz) = (pixel(&com, ex, ey), pixel(&com, dx, dy));
    assert!(
        brightness(sombra) + 60 < brightness(luz),
        "embaixo da metade opaca, sombra ({sombra:?}); embaixo do buraco, sol ({luz:?})"
    );
    // Sem o recorte, a folha inteira faz sombra.
    let sem = cena(&mut h, 0.0);
    let buraco_sem = pixel(&sem, dx, dy);
    assert!(
        brightness(buraco_sem) + 60 < brightness(luz),
        "sem recorte, o buraco também faz sombra ({buraco_sem:?})"
    );
}

#[test]
fn duas_faces_aparecem_por_baixo_com_a_luz_do_lado_delas() {
    let Some(mut h) = Harness::new() else { return };
    let plane = h.renderer.create_mesh(&shapes::plane());
    // A câmera embaixo do plano, e o sol também embaixo, iluminando para cima.
    h.renderer
        .set_camera3d(Camera3D::looking_at(Vec3::new(0.0, -3.0, 1.0), Vec3::ZERO));
    h.renderer.set_light3d(Light3D {
        sun_direction: Vec3::Y,
        sun_color: Color::WHITE,
        sky: Color::BLACK,
        ground: Color::BLACK,
        fog: None,
    });
    let cena = |h: &mut Harness, double_sided: bool| {
        h.renderer.draw_mesh_material(
            plane,
            Mat4::from_scale(Vec3::splat(4.0)),
            Color::WHITE,
            None,
            false,
            Material3D {
                double_sided,
                ..Material3D::default()
            },
        );
        h.render(Color::rgb(0.0, 0.0, 1.0))
    };
    // Uma face só: por baixo, não aparece (o fundo azul).
    assert_color(
        pixel(&cena(&mut h, false), 32, 32),
        [0, 0, 255, 255],
        "face de trás",
    );
    // Duas faces: aparece, acesa pelo sol de baixo.
    let duas = pixel(&cena(&mut h, true), 32, 32);
    assert!(
        brightness(duas) > 600,
        "a face de baixo, iluminada: {duas:?}"
    );
}

#[test]
fn gltf_traz_o_recorte_a_transparencia_e_as_duas_faces() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/modelos");
    let texto = std::fs::read_to_string(dir.join("caixote.gltf")).unwrap();
    let com = |extra: &str| {
        let t = texto.replacen(
            "\"name\": \"caixote-0\",",
            &format!("\"name\": \"caixote-0\", {extra}"),
            1,
        );
        assert_ne!(t, texto, "o material do caixote mudou de nome");
        ModelData::from_gltf(t.as_bytes(), &mut |uri| std::fs::read(dir.join(uri)).ok()).unwrap()
    };
    let m = com("\"alphaMode\": \"MASK\", \"alphaCutoff\": 0.3, \"doubleSided\": true,");
    assert!(
        (m.parts[0].cutout - 0.3).abs() < 1e-6,
        "{}",
        m.parts[0].cutout
    );
    assert!(m.parts[0].double_sided && !m.parts[0].blend);
    // MASK sem o alphaCutoff: 0.5, o padrão do glTF.
    let m = com("\"alphaMode\": \"MASK\",");
    assert!((m.parts[0].cutout - 0.5).abs() < 1e-6);
    let m = com("\"alphaMode\": \"BLEND\",");
    assert!(m.parts[0].blend && m.parts[0].cutout == 0.0 && !m.parts[0].double_sided);
    // Opaco (o padrão): nada muda.
    let m = ModelData::from_gltf(texto.as_bytes(), &mut |uri| {
        std::fs::read(dir.join(uri)).ok()
    })
    .unwrap();
    assert!(m.parts[0].cutout == 0.0 && !m.parts[0].blend && !m.parts[0].double_sided);
}

// --- Sombra dos holofotes (Sprint 77) ---------------------------------------

/// Um holofote a 4 m apontando para baixo e um cubo ao lado, a 1 m do chão:
/// a sombra do cubo cai no chão do lado de fora dele.
fn holofote_e_cubo(h: &mut Harness, shadow: bool, cast: bool) -> Vec<u8> {
    chao_no_escuro(h);
    let cube = h.renderer.create_mesh(&shapes::cube());
    h.renderer.draw_mesh_with(
        cube,
        Mat4::from_translation(Vec3::new(1.0, 1.0, 0.0)) * Mat4::from_scale(Vec3::splat(0.6)),
        Color::WHITE,
        None,
        cast,
    );
    let mut lamp = Lamp3D::at(Vec3::new(0.0, 4.0, 0.0));
    lamp.range = 12.0;
    lamp.intensity = 2.0;
    lamp.shadow = shadow;
    lamp.cone = Some(Cone3D {
        direction: Vec3::NEG_Y,
        angle: 40.0,
        softness: 0.1,
    });
    h.renderer.add_lamp3d(lamp);
    h.render(Color::BLACK)
}

#[test]
fn holofote_com_sombra_escurece_atras_do_cubo() {
    let Some(mut h) = Harness::new() else { return };
    let camera = Camera3D::looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::ZERO);
    let at = |p: Vec3| {
        let s = camera.screen_point(p, SIZE.as_vec2()).unwrap();
        (s.x as u32, s.y as u32)
    };
    // No chão, na linha do holofote ao cubo, depois dele; e do outro lado.
    let (sx, sy) = at(Vec3::new(1.65, 0.0, 0.0));
    let (lx, ly) = at(Vec3::new(-1.65, 0.0, 0.0));
    let com = holofote_e_cubo(&mut h, true, true);
    assert_eq!(h.renderer.last_stats3().spot_shadows, 1);
    let (sombra, luz) = (pixel(&com, sx, sy), pixel(&com, lx, ly));
    assert!(
        brightness(sombra) * 3 < brightness(luz),
        "atrás do cubo, sombra ({sombra:?}); do outro lado, luz ({luz:?})"
    );
    // Sem sombra no holofote, ou com o cubo sem projetar, o chão ali acende.
    for (shadow, cast) in [(false, true), (true, false)] {
        let img = holofote_e_cubo(&mut h, shadow, cast);
        let p = pixel(&img, sx, sy);
        assert!(
            brightness(p) * 4 > brightness(luz) * 3,
            "sombra {shadow}, projeta {cast}: {p:?} (luz {luz:?})"
        );
    }
    assert_eq!(h.renderer.last_stats3().spot_shadows, 1);
    // Sombras desligadas: nada de mapa.
    h.renderer.set_shadows3d(Shadows {
        enabled: false,
        ..Shadows::default()
    });
    let img = holofote_e_cubo(&mut h, true, true);
    assert!(brightness(pixel(&img, sx, sy)) * 4 > brightness(luz) * 3);
    assert_eq!(h.renderer.last_stats3().spot_shadows, 0);
}

#[test]
fn so_quatro_holofotes_fazem_sombra_os_mais_perto() {
    let Some(mut h) = Harness::new() else { return };
    chao_no_escuro(&mut h);
    let cube = h.renderer.create_mesh(&shapes::cube());
    h.renderer
        .draw_mesh(cube, Mat4::from_translation(Vec3::Y), Color::WHITE);
    for i in 0..6 {
        let mut lamp = Lamp3D::at(Vec3::new(i as f32 - 3.0, 4.0, 0.0));
        lamp.shadow = true;
        lamp.cone = Some(Cone3D {
            direction: Vec3::NEG_Y,
            angle: 30.0,
            softness: 0.2,
        });
        h.renderer.add_lamp3d(lamp);
    }
    h.render(Color::BLACK);
    let s = h.renderer.last_stats3();
    assert_eq!((s.lamps, s.spot_shadows), (6, 4));
}
