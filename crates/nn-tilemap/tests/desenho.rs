//! Desenho de tilemaps (precisa de GPU; sem GPU o teste é pulado).

use glam::{UVec2, Vec2};
use nn_core::Color;
use nn_physics::Cell;
use nn_render::{Camera2D, GpuContext, OffscreenTarget, Renderer, TextureOptions};
use nn_tilemap::{AsciiTile, FLIP_H, TileMap};

fn gpu() -> Option<GpuContext> {
    match pollster::block_on(GpuContext::new_headless()) {
        Ok(g) => Some(g),
        Err(e) => {
            if std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1") {
                panic!("GPU obrigatória mas indisponível: {e}");
            }
            eprintln!("pulando teste: {e}");
            None
        }
    }
}

#[test]
fn desenha_tiles_na_posicao_certa() {
    let Some(gpu) = gpu() else { return };
    let mut renderer = Renderer::new(&gpu, OffscreenTarget::FORMAT);
    let target = OffscreenTarget::new(&gpu, UVec2::splat(64));

    // Tileset 2x1: tile 0 vermelho (metade esquerda branca), tile 1 azul.
    let mut px = Vec::new();
    for c in [
        [255u8, 255, 255, 255],
        [255, 0, 0, 255],
        [0, 0, 255, 255],
        [0, 0, 255, 255],
    ] {
        px.extend_from_slice(&c);
    }
    let tex = renderer.create_texture(UVec2::new(4, 1), &px, TextureOptions::PIXEL_ART);
    let sheet = renderer.sprite_sheet(tex, UVec2::new(2, 1));

    let mut mapa = TileMap::from_ascii(&["ab", "b."], Vec2::splat(32.0), |c| match c {
        'a' => Some(AsciiTile::Tile {
            frame: 0,
            cell: Cell::Solid,
        }),
        'b' => Some(AsciiTile::Tile {
            frame: 1,
            cell: Cell::Empty,
        }),
        _ => None,
    });
    mapa.add_sheet(&sheet, UVec2::new(4, 1));
    // Espelha o tile "a" horizontalmente: o branco vai para a direita.
    mapa.layers[0].tiles[0] |= FLIP_H;

    // Câmera no centro do mapa (64x64 no mundo).
    mapa.draw(
        &mut renderer,
        &Camera2D::new(Vec2::splat(32.0)),
        Vec2::splat(64.0),
    );
    renderer.render(
        target.view(),
        UVec2::splat(64),
        &Camera2D::new(Vec2::splat(32.0)),
        Some(Color::BLACK),
    );
    let img = target.read_rgba(&gpu).unwrap();
    let p =
        |x: u32, y: u32| -> [u8; 4] { img[((y * 64 + x) * 4) as usize..][..4].try_into().unwrap() };

    // Linha de cima: "a" (espelhado: vermelho à esquerda, branco à direita) e "b" (azul).
    assert_eq!(p(8, 16), [255, 0, 0, 255]);
    assert_eq!(p(24, 16), [255, 255, 255, 255]);
    assert_eq!(p(48, 16), [0, 0, 255, 255]);
    // Linha de baixo: "b" e vazio.
    assert_eq!(p(16, 48), [0, 0, 255, 255]);
    assert_eq!(p(48, 48), [0, 0, 0, 255]);
    // Tiles dividem a mesma textura: uma única chamada de desenho.
    assert_eq!(renderer.last_stats().draw_calls, 1);
}

#[test]
fn tiles_animados_trocam_com_o_tempo() {
    let Some(gpu) = gpu() else { return };
    let mut renderer = Renderer::new(&gpu, OffscreenTarget::FORMAT);
    let target = OffscreenTarget::new(&gpu, UVec2::splat(32));

    // Três tiles de 1 pixel: vermelho, verde, azul.
    let px = [255u8, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255];
    let tex = renderer.create_texture(UVec2::new(3, 1), &px, TextureOptions::PIXEL_ART);
    let sheet = renderer.sprite_sheet(tex, UVec2::new(1, 1));
    let mut mapa = TileMap::from_ascii(&["~"], Vec2::splat(32.0), |c| {
        (c == '~').then_some(AsciiTile::Tile {
            frame: 0,
            cell: Cell::Empty,
        })
    });
    mapa.add_sheet(&sheet, UVec2::new(3, 1));
    // Água: tile 1 alterna entre os tiles 2 e 3, 4 vezes por segundo.
    assert!(mapa.animate(1, &[2, 3], 4.0));
    assert!(!mapa.animate(1, &[2, 99], 4.0), "id fora do tileset");

    let mut cor = |mapa: &TileMap| {
        let camera = Camera2D::new(Vec2::splat(16.0));
        mapa.draw(&mut renderer, &camera, Vec2::splat(32.0));
        renderer.render(target.view(), UVec2::splat(32), &camera, Some(Color::BLACK));
        let img = target.read_rgba(&gpu).unwrap();
        [img[0], img[1], img[2]]
    };
    assert_eq!(cor(&mapa), [0, 255, 0]);
    mapa.update(0.3);
    assert_eq!(cor(&mapa), [0, 0, 255]);
    mapa.update(0.25);
    assert_eq!(cor(&mapa), [0, 255, 0]);
}
