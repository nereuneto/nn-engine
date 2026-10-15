//! Importação de mapas do Tiled.

use std::path::Path;

use glam::Vec2;
use nn_physics::Cell;
use nn_tilemap::{FLIP_H, Property, TilemapError, load_tiled, read_from};

fn fixtures() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn carregar() -> nn_tilemap::TileMap {
    load_tiled(Path::new("fases/teste.tmj"), read_from(fixtures()), 1.0).unwrap()
}

#[test]
fn le_tamanho_tilesets_e_caminhos() {
    let m = carregar();
    assert_eq!((m.width, m.height), (4, 3));
    assert_eq!(m.tile_size, Vec2::splat(16.0));
    assert_eq!(m.tilesets.len(), 2);
    assert_eq!(m.tilesets[0].image, Path::new("imagens/terreno.png"));
    // Tileset externo: imagem relativa ao .tsj.
    assert_eq!(m.tilesets[1].first_gid, 5);
    assert_eq!(m.tilesets[1].image, Path::new("imagens/decoracao.png"));
    assert_eq!(
        m.properties.get("musica"),
        Some(&Property::String("fase1.ogg".into()))
    );
}

#[test]
fn camadas_grupos_e_espelhamento() {
    let m = carregar();
    let nomes: Vec<_> = m.layers.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(nomes, ["fundo", "chao", "paredes_invisiveis"]);
    assert_eq!(m.layers[0].opacity, 0.5);
    assert!(!m.layers[2].visible);
    // Bit de espelhamento horizontal preservado na camada, removido na consulta.
    assert_eq!(m.layers[0].tiles[3] & FLIP_H, FLIP_H);
    assert_eq!(m.tile(0, 3, 0), 6);
}

#[test]
fn colisao_por_propriedade_de_tile_e_de_camada() {
    let m = carregar();
    assert_eq!(m.collision_cell(0, 2), Cell::Solid); // tile com solid = true
    assert_eq!(m.collision_cell(1, 1), Cell::OneWay); // tile com one_way = true
    assert_eq!(m.collision_cell(0, 0), Cell::Solid); // camada com collision = true
    assert_eq!(m.collision_cell(3, 0), Cell::Empty); // decoração
    let g = m.collision_grid();
    assert_eq!(g.get(0, 0), Cell::Solid);
    assert_eq!(g.get(1, 1), Cell::OneWay);
}

#[test]
fn objetos_em_coordenadas_do_mundo() {
    let m = carregar();
    let inicio = m.object("inicio").unwrap();
    assert!(inicio.point);
    // Mapa com 48 px de altura: y = 24 no Tiled vira 24 no mundo (meio).
    assert_eq!(inicio.position, Vec2::new(8.0, 24.0));
    let moeda = m.objects_of("moeda").next().unwrap();
    assert_eq!(moeda.position, Vec2::new(40.0, 40.0));
    assert_eq!(moeda.properties.get("valor"), Some(&Property::Int(5)));
    // Objeto de tile: y do Tiled é a base.
    let bau = m.object("bau").unwrap();
    assert_eq!(bau.kind, "item");
    assert_eq!(bau.position, Vec2::new(56.0, 8.0));
}

#[test]
fn escala_multiplica_tudo() {
    let m = load_tiled(Path::new("fases/teste.tmj"), read_from(fixtures()), 2.0).unwrap();
    assert_eq!(m.tile_size, Vec2::splat(32.0));
    assert_eq!(m.object("inicio").unwrap().position, Vec2::new(16.0, 48.0));
}

#[test]
fn erros_explicam_o_que_fazer() {
    let ler = |conteudo: &'static str| move |_: &Path| Ok::<_, String>(conteudo.to_string());

    let tmx = load_tiled(Path::new("a.tmx"), ler(""), 1.0).unwrap_err();
    assert!(tmx.to_string().contains("JSON"), "{tmx}");

    let infinito = load_tiled(
        Path::new("a.tmj"),
        ler(r#"{"width":1,"height":1,"tilewidth":8,"tileheight":8,"infinite":true}"#),
        1.0,
    )
    .unwrap_err();
    assert!(infinito.to_string().contains("infinitos"), "{infinito}");

    let base64 = load_tiled(
        Path::new("a.tmj"),
        ler(r#"{"width":1,"height":1,"tilewidth":8,"tileheight":8,
               "layers":[{"type":"tilelayer","name":"c","data":"AAAA","encoding":"base64"}]}"#),
        1.0,
    )
    .unwrap_err();
    assert!(base64.to_string().contains("CSV"), "{base64}");

    let tsx = load_tiled(
        Path::new("a.tmj"),
        ler(r#"{"width":1,"height":1,"tilewidth":8,"tileheight":8,
               "tilesets":[{"firstgid":1,"source":"t.tsx"}]}"#),
        1.0,
    )
    .unwrap_err();
    assert!(tsx.to_string().contains(".tsj"), "{tsx}");

    let json = load_tiled(Path::new("a.tmj"), ler("{ isto não é json"), 1.0).unwrap_err();
    assert!(matches!(json, TilemapError::Json { .. }));

    let faltando = load_tiled(Path::new("nao_existe.tmj"), read_from(fixtures()), 1.0).unwrap_err();
    assert!(matches!(faltando, TilemapError::Io { .. }));
}

#[test]
fn tiles_animados_do_tiled() {
    let m = carregar();
    let deco = &m.tilesets[1];
    let anim = deco.animations.get(&1).expect("tile 1 animado");
    assert_eq!(anim.frames, [(1, 0.15), (2, 0.35)]);
    assert!((anim.duration() - 0.5).abs() < 1e-6);
    assert_eq!(anim.frame_at(0.0), Some(1));
    assert_eq!(anim.frame_at(0.2), Some(2));
    assert_eq!(anim.frame_at(0.55), Some(1), "repete");
    assert!(!deco.animations.contains_key(&0));
}
