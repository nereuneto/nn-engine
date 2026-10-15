//! Testes do mundo físico.

use super::*;

const DT: f32 = 1.0 / 60.0;

fn run(world: &mut World, seconds: f32) -> Vec<Event> {
    let mut events = Vec::new();
    for _ in 0..(seconds / DT).round() as u32 {
        events.extend_from_slice(world.step(DT));
    }
    events
}

/// Chão de -500 a 500, com o topo em y = 0.
fn with_floor() -> World {
    let mut w = World::default();
    w.add(Body::fixed(Shape::rect(1000.0, 20.0)).at(Vec2::new(0.0, -10.0)));
    w
}

#[test]
fn cai_e_para_no_chao() {
    let mut w = with_floor();
    let p = w.add(Body::dynamic(Shape::rect(20.0, 40.0)).at(Vec2::new(0.0, 200.0)));
    run(&mut w, 2.0);
    let b = w.get(p).unwrap();
    assert!((b.position.y - 20.0).abs() < 0.01, "y = {}", b.position.y);
    assert!(b.on_ground());
    assert_eq!(b.velocity.y, 0.0);
}

#[test]
fn anda_sobre_varios_tiles_sem_enroscar() {
    let mut w = World::default();
    let mut grid = CollisionGrid::new(50, 4, Vec2::splat(16.0), Vec2::ZERO);
    for c in 0..50 {
        grid.set(c, 0, Cell::Solid);
    }
    w.add_grid(grid);
    let p = w.add(Body::dynamic(Shape::rect(12.0, 20.0)).at(Vec2::new(20.0, 40.0)));
    run(&mut w, 0.5);
    w.get_mut(p).unwrap().velocity.x = 200.0;
    for _ in 0..120 {
        w.get_mut(p).unwrap().velocity.x = 200.0;
        w.step(DT);
        assert!(
            !w.get(p).unwrap().touching().right,
            "enroscou na junção dos tiles"
        );
    }
    let b = w.get(p).unwrap();
    assert!(b.position.x > 400.0, "x = {}", b.position.x);
    assert!(b.on_ground());
}

#[test]
fn parede_bloqueia() {
    let mut w = with_floor();
    w.add(Body::fixed(Shape::rect(20.0, 200.0)).at(Vec2::new(100.0, 100.0)));
    let p = w.add(Body::dynamic(Shape::rect(20.0, 20.0)).at(Vec2::new(0.0, 10.0)));
    for _ in 0..120 {
        w.get_mut(p).unwrap().velocity.x = 300.0;
        w.step(DT);
    }
    let b = w.get(p).unwrap();
    assert!((b.position.x - 80.0).abs() < 0.01, "x = {}", b.position.x);
    assert!(b.touching().right);
}

#[test]
fn alta_velocidade_nao_atravessa_parede_fina() {
    let mut w = World::new(Vec2::ZERO);
    w.add(Body::fixed(Shape::rect(4.0, 200.0)).at(Vec2::new(100.0, 0.0)));
    let p = w.add(Body::dynamic(Shape::rect(10.0, 10.0)).with_velocity(Vec2::new(30_000.0, 0.0)));
    w.step(DT);
    assert!(w.get(p).unwrap().position.x < 100.0);
}

#[test]
fn plataforma_de_mao_unica() {
    let mut w = with_floor();
    w.add(
        Body::fixed(Shape::rect(100.0, 10.0))
            .at(Vec2::new(0.0, 100.0))
            .as_one_way(),
    );
    // Pulando de baixo: atravessa.
    let p = w.add(
        Body::dynamic(Shape::rect(20.0, 20.0))
            .at(Vec2::new(0.0, 10.0))
            .with_velocity(Vec2::new(0.0, 800.0)),
    );
    let mut passou = false;
    for _ in 0..60 {
        w.step(DT);
        if w.get(p).unwrap().position.y > 115.0 {
            passou = true;
        }
    }
    assert!(passou, "não atravessou por baixo");
    // Depois cai e para em cima dela.
    run(&mut w, 1.0);
    let b = w.get(p).unwrap();
    assert!((b.position.y - 115.0).abs() < 0.01, "y = {}", b.position.y);
    assert!(b.on_ground());
}

#[test]
fn grade_de_mao_unica() {
    let mut w = World::default();
    let mut grid = CollisionGrid::new(10, 10, Vec2::splat(16.0), Vec2::ZERO);
    for c in 0..10 {
        grid.set(c, 0, Cell::Solid);
        grid.set(c, 3, Cell::OneWay);
    }
    w.add_grid(grid);
    let p = w.add(Body::dynamic(Shape::rect(10.0, 10.0)).at(Vec2::new(40.0, 100.0)));
    run(&mut w, 1.0);
    let b = w.get(p).unwrap();
    assert!((b.position.y - 69.0).abs() < 0.01, "y = {}", b.position.y);
    assert!(b.on_ground());
}

#[test]
fn sensores_avisam_entrada_e_saida() {
    let mut w = World::new(Vec2::ZERO);
    let moeda = w.add(
        Body::fixed(Shape::circle(10.0))
            .at(Vec2::new(50.0, 0.0))
            .as_sensor(),
    );
    let p = w.add(Body::dynamic(Shape::rect(10.0, 10.0)).with_velocity(Vec2::new(60.0, 0.0)));
    let eventos = run(&mut w, 2.0);
    assert_eq!(eventos, [Event::Enter(moeda, p), Event::Exit(moeda, p)]);
    // O sensor não bloqueou.
    assert!(w.get(p).unwrap().position.x > 100.0);
}

#[test]
fn camadas_filtram_colisoes() {
    let mut w = with_floor();
    let fantasma = w.add(
        Body::dynamic(Shape::rect(10.0, 10.0))
            .at(Vec2::new(0.0, 100.0))
            .with_layer(2, 2),
    );
    run(&mut w, 1.0);
    assert!(
        w.get(fantasma).unwrap().position.y < -50.0,
        "deveria atravessar o chão"
    );
}

#[test]
fn plataforma_movel_carrega_o_jogador() {
    let mut w = World::default();
    let plat = w.add(Body::kinematic(Shape::rect(100.0, 10.0)));
    let p = w.add(Body::dynamic(Shape::rect(10.0, 10.0)).at(Vec2::new(0.0, 30.0)));
    run(&mut w, 0.5); // pousa
    w.get_mut(plat).unwrap().velocity = Vec2::new(50.0, 0.0);
    run(&mut w, 2.0);
    let b = w.get(p).unwrap();
    let pl = w.get(plat).unwrap();
    assert!(b.on_ground());
    assert_eq!(b.ground_body(), Some(plat));
    // Andou junto: continua no mesmo ponto da plataforma.
    assert!(
        (b.position.x - pl.position.x).abs() < 0.01,
        "{} vs {}",
        b.position.x,
        pl.position.x
    );
    assert!(pl.position.x > 99.0);
}

#[test]
fn elevador_sobe_com_o_jogador() {
    let mut w = World::default();
    let elev = w.add(Body::kinematic(Shape::rect(100.0, 10.0)));
    let p = w.add(Body::dynamic(Shape::rect(10.0, 10.0)).at(Vec2::new(0.0, 10.0)));
    run(&mut w, 0.5);
    w.get_mut(elev).unwrap().velocity = Vec2::new(0.0, 100.0);
    run(&mut w, 1.0);
    let b = w.get(p).unwrap();
    let e = w.get(elev).unwrap();
    assert!(
        (b.position.y - (e.position.y + 10.0)).abs() < 1.0,
        "{} vs {}",
        b.position.y,
        e.position.y
    );
    assert!(e.position.y > 90.0);
}

#[test]
fn circulo_rola_no_chao_e_quica() {
    let mut w = with_floor();
    let bola = w.add(
        Body::dynamic(Shape::circle(10.0))
            .at(Vec2::new(0.0, 200.0))
            .with_bounce(0.6),
    );
    let mut quicou = false;
    for _ in 0..240 {
        w.step(DT);
        if w.get(bola).unwrap().velocity.y > 50.0 {
            quicou = true;
        }
    }
    assert!(quicou);
    let b = w.get(bola).unwrap();
    assert!((b.position.y - 10.0).abs() < 0.5, "y = {}", b.position.y);
    assert!(b.on_ground());
}

#[test]
fn caixas_nao_se_atravessam() {
    let mut w = with_floor();
    let a = w.add(Body::dynamic(Shape::rect(20.0, 20.0)).at(Vec2::new(0.0, 10.0)));
    let b = w.add(Body::dynamic(Shape::rect(20.0, 20.0)).at(Vec2::new(0.0, 60.0)));
    run(&mut w, 2.0);
    let (pa, pb) = (w.get(a).unwrap().position, w.get(b).unwrap().position);
    assert!(pb.y - pa.y > 19.0, "{pa} {pb}");
    assert!(w.get(b).unwrap().on_ground());
}

#[test]
fn raycast_acha_o_mais_proximo() {
    let mut w = with_floor();
    let caixa = w.add(Body::fixed(Shape::rect(20.0, 20.0)).at(Vec2::new(0.0, 50.0)));
    w.add(
        Body::fixed(Shape::rect(20.0, 20.0))
            .at(Vec2::new(0.0, 30.0))
            .as_sensor(),
    );
    let hit = w
        .raycast(Vec2::new(0.0, 200.0), Vec2::NEG_Y, 500.0, u32::MAX)
        .unwrap();
    assert_eq!(hit.body, Some(caixa));
    assert!((hit.distance - 140.0).abs() < 0.01);
    assert_eq!(hit.normal, Vec2::Y);
    // Sem alcance suficiente.
    assert!(
        w.raycast(Vec2::new(0.0, 200.0), Vec2::NEG_Y, 100.0, u32::MAX)
            .is_none()
    );
    // Ao lado da caixa, chega no chão.
    let hit = w
        .raycast(Vec2::new(300.0, 200.0), Vec2::NEG_Y, 500.0, u32::MAX)
        .unwrap();
    assert!((hit.point.y).abs() < 0.01);
}

#[test]
fn consultas() {
    let mut w = World::new(Vec2::ZERO);
    let a = w.add(Body::fixed(Shape::rect(10.0, 10.0)));
    let b = w.add(Body::fixed(Shape::circle(5.0)).at(Vec2::new(20.0, 0.0)));
    assert_eq!(w.query_point(Vec2::new(1.0, 1.0)), [a]);
    assert_eq!(w.query_point(Vec2::new(22.0, 0.0)), [b]);
    let mut todos = w.query_aabb(Aabb::new(Vec2::new(-20.0, -5.0), Vec2::new(30.0, 5.0)));
    todos.sort();
    assert_eq!(todos, [a, b]);
}

#[test]
fn ids_continuam_validos_depois_de_remover() {
    let mut w = World::new(Vec2::ZERO);
    let a = w.add(Body::fixed(Shape::rect(1.0, 1.0)));
    w.remove(a);
    let b = w.add(Body::fixed(Shape::rect(1.0, 1.0)));
    assert!(w.get(a).is_none());
    assert!(w.get(b).is_some());
    assert_ne!(a, b);
    assert_eq!(BodyId::from_bits(b.to_bits()), b);
    assert_eq!(w.len(), 1);
}

#[test]
fn sobreposicao_entre_dois_corpos_ignora_camadas() {
    let mut w = World::default();
    // Jogador e inimigo em camadas que não interagem: não se empurram...
    let jogador = w.add(
        Body::dynamic(Shape::rect(20.0, 40.0))
            .at(Vec2::new(0.0, 0.0))
            .with_layer(2, u32::MAX),
    );
    let inimigo = w.add(
        Body::dynamic(Shape::circle(10.0))
            .at(Vec2::new(25.0, 0.0))
            .with_gravity_scale(0.0)
            .with_layer(4, 1),
    );
    w.get_mut(jogador).unwrap().gravity_scale = 0.0;
    assert!(!w.intersects(jogador, inimigo), "ainda separados");
    w.get_mut(inimigo).unwrap().position.x = 15.0;
    w.step(DT);
    // ...mas a sobreposição é detectada.
    assert!(w.intersects(jogador, inimigo));
    assert!(w.intersects(inimigo, jogador));
    assert_eq!(
        w.get(inimigo).unwrap().position.x,
        15.0,
        "não foi empurrado"
    );
    assert!(
        w.overlapping(jogador).is_empty(),
        "overlapping respeita as máscaras"
    );
    assert!(!w.intersects(jogador, jogador));
    w.remove(inimigo);
    assert!(!w.intersects(jogador, inimigo));
}
