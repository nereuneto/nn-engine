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
        ["parado", "andar", "correr", "pular", "cair", "acenar"]
    );
    assert_eq!(s.joints.len(), 10);
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
    // O bicho também.
    let b = modelo_ilha("bicho.glb");
    let names: Vec<String> = b
        .skeleton
        .unwrap()
        .animations
        .into_iter()
        .map(|a| a.name)
        .collect();
    assert_eq!(names, ["parado", "andar"]);
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
