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

#[test]
fn grade_de_caminhos_com_paredes_tiles_e_o_que_nao_bloqueia() {
    let mut w = World::new(Vec2::ZERO);
    w.add(Body::fixed(Shape::rect(200.0, 20.0)).at(Vec2::new(0.0, 0.0)));
    w.add(Body::fixed(Shape::circle(20.0)).at(Vec2::new(0.0, 100.0)));
    w.add(
        Body::fixed(Shape::rect(40.0, 40.0))
            .at(Vec2::new(-100.0, 100.0))
            .as_sensor(),
    );
    w.add(
        Body::fixed(Shape::rect(40.0, 10.0))
            .at(Vec2::new(100.0, 100.0))
            .as_one_way(),
    );
    w.add(Body::dynamic(Shape::rect(16.0, 16.0)).at(Vec2::new(0.0, -100.0)));
    let mut g = CollisionGrid::new(4, 4, Vec2::splat(16.0), Vec2::new(-200.0, -200.0));
    g.set(1, 1, Cell::Solid);
    w.add_grid(g);
    let nav = w.nav_grid(None, 8.0, 6.0);
    let anda = |x: f32, y: f32| nav.walkable_at(Vec2::new(x, y));
    assert!(!anda(0.0, 0.0), "a parede");
    assert!(!anda(0.0, 13.0), "perto da parede (o raio de quem anda)");
    assert!(anda(0.0, 20.0));
    assert!(!anda(0.0, 100.0), "o círculo");
    assert!(anda(-100.0, 100.0), "sensor não bloqueia");
    assert!(anda(100.0, 100.0), "mão única não bloqueia");
    assert!(anda(0.0, -100.0), "quem se move não bloqueia");
    assert!(!anda(-200.0 + 24.0, -200.0 + 24.0), "o tile sólido");
    assert!(anda(-200.0 + 56.0, -200.0 + 24.0));
    let p = nav
        .path(Vec2::new(0.0, -40.0), Vec2::new(0.0, 40.0))
        .unwrap();
    assert!(p.complete);
    assert!(p.points.iter().any(|q| q.x.abs() > 100.0), "{:?}", p.points);
}

/// Um mundo cheio e bagunçado: chão, tiles, plataformas móveis, sensores,
/// mão única, círculos, camadas e corpos nascendo uns dentro dos outros.
fn crowd(n: usize, spread: f32, seed: u64) -> World {
    let mut r = seed;
    let mut f = move || {
        r = r
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((r >> 33) as f32) / (1u64 << 31) as f32
    };
    let mut w = World::default();
    let mut g = CollisionGrid::new(100, 10, Vec2::splat(32.0), Vec2::new(-1600.0, -800.0));
    for c in 0..100 {
        g.set(c, 0, Cell::Solid);
        if c % 7 == 0 {
            g.set(c, 4, Cell::OneWay);
        }
    }
    w.add_grid(g);
    w.add(Body::fixed(Shape::rect(4000.0, 40.0)).at(Vec2::new(0.0, -400.0)));
    for i in 0..n {
        let at = Vec2::new(
            (f() * 3000.0 - 1500.0) * spread,
            (f() * 1200.0 - 300.0) * spread,
        );
        let s = if i % 5 == 0 {
            Shape::circle(4.0 + f() * 12.0)
        } else {
            Shape::rect(6.0 + f() * 30.0, 6.0 + f() * 30.0)
        };
        let b = match i % 11 {
            0 => Body::fixed(s).at(at),
            1 => Body::kinematic(Shape::rect(80.0, 12.0))
                .at(at)
                .with_velocity(Vec2::new(f() * 100.0 - 50.0, f() * 60.0 - 30.0)),
            2 => Body::fixed(s).at(at).as_sensor(),
            3 => Body::fixed(Shape::rect(60.0, 8.0)).at(at).as_one_way(),
            4 => Body::dynamic(s).at(at).as_sensor(),
            5 => Body::dynamic(s)
                .at(at)
                .with_bounce(0.5)
                .with_velocity(Vec2::new(f() * 600.0 - 300.0, 0.0)),
            6 => Body::dynamic(s).at(at).with_layer(2, 2),
            _ => Body::dynamic(s)
                .at(at)
                .with_velocity(Vec2::new(f() * 400.0 - 200.0, f() * 400.0 - 200.0)),
        };
        w.add(b);
    }
    w
}

fn snapshot(w: &World) -> Vec<(BodyId, Vec2, Vec2, Touching, Option<BodyId>)> {
    w.bodies()
        .map(|(id, b)| (id, b.position, b.velocity, b.touching(), b.ground_body()))
        .collect()
}

#[test]
fn a_grade_da_o_mesmo_resultado_que_conferir_todos() {
    for (n, spread, seed) in [(300, 1.0, 42), (500, 0.2, 7), (200, 0.05, 3)] {
        let mut a = crowd(n, spread, seed);
        let mut b = crowd(n, spread, seed);
        b.cell_override = Some(1.0e9);
        for step in 0..180 {
            let ea = a.step(DT).to_vec();
            let eb = b.step(DT).to_vec();
            assert_eq!(ea, eb, "eventos no passo {step} ({n} corpos)");
        }
        assert_eq!(snapshot(&a), snapshot(&b), "{n} corpos");
        // As consultas também.
        for k in 0..40 {
            let o = Vec2::new(k as f32 * 70.0 - 1400.0, 300.0);
            let d = Vec2::new((k as f32).sin(), -1.0);
            assert_eq!(
                a.raycast(o, d, 2000.0, u32::MAX),
                b.raycast(o, d, 2000.0, u32::MAX)
            );
            assert_eq!(a.raycast(o, d, 30.0, 2), b.raycast(o, d, 30.0, 2));
            let p = Vec2::new(k as f32 * 50.0 - 1000.0, -380.0 + k as f32 * 9.0);
            assert_eq!(a.query_point(p), b.query_point(p));
            let area = Aabb::from_center(p, Vec2::splat(60.0));
            assert_eq!(a.query_aabb(area), b.query_aabb(area));
        }
        for (id, _) in a.bodies().take(50).collect::<Vec<_>>() {
            assert_eq!(a.overlapping(id), b.overlapping(id));
        }
    }
}

#[test]
fn o_passo_nao_depende_de_ordem_escondida() {
    // Dois mundos iguais dão o mesmo resultado (antes, a ordem das
    // plataformas móveis vinha de um HashMap e mudava de um para o outro).
    let mut a = crowd(400, 0.3, 11);
    let mut b = crowd(400, 0.3, 11);
    run(&mut a, 3.0);
    run(&mut b, 3.0);
    assert_eq!(snapshot(&a), snapshot(&b));
}

#[test]
fn a_grade_confere_so_quem_esta_perto() {
    // 2.000 caixas paradas espalhadas e 200 caindo: sem a grade seriam
    // milhões de conferências por passo.
    let mut w = with_floor();
    for i in 0..2000 {
        let x = (i % 100) as f32 * 40.0 - 2000.0;
        let y = (i / 100) as f32 * 40.0 + 400.0;
        w.add(Body::fixed(Shape::rect(16.0, 16.0)).at(Vec2::new(x, y)));
    }
    for i in 0..200 {
        let x = (i % 20) as f32 * 30.0 - 300.0;
        let y = (i / 20) as f32 * 30.0 + 50.0;
        w.add(Body::dynamic(Shape::rect(12.0, 12.0)).at(Vec2::new(x, y)));
    }
    run(&mut w, 1.0);
    let s = w.stats();
    assert_eq!(s.bodies, 2201);
    assert!(s.checks < 20_000, "{s:?}");
}

/// Só no modo otimizado (`cargo test --release`): o tempo do passo.
#[cfg(not(debug_assertions))]
#[test]
fn quinhentos_corpos_caindo_em_menos_de_4_ms() {
    let mut w = with_floor();
    for i in 0..500 {
        let x = (i % 50) as f32 * 18.0 - 450.0;
        let y = (i / 50) as f32 * 30.0 + 2000.0;
        w.add(Body::dynamic(Shape::rect(8.0, 8.0)).at(Vec2::new(x, y)));
    }
    let t = std::time::Instant::now();
    run(&mut w, 5.0);
    let ms = t.elapsed().as_secs_f64() * 1000.0 / 300.0;
    assert!(ms < 4.0, "{ms:.2} ms por passo");
}
