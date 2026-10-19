use super::*;

const DT: f32 = 1.0 / 60.0;

fn chao(w: &mut World3) -> BodyId3 {
    w.add(&BodyDesc {
        kind: BodyKind3::Static,
        shape: Shape3::Box(Vec3::new(40.0, 1.0, 40.0)),
        position: Vec3::new(0.0, -0.5, 0.0),
        ..Default::default()
    })
    .unwrap()
}

fn caixa(w: &mut World3, p: Vec3) -> BodyId3 {
    w.add(&BodyDesc {
        position: p,
        ..Default::default()
    })
    .unwrap()
}

fn passos(w: &mut World3, segundos: f32) {
    for _ in 0..(segundos / DT) as usize {
        w.step(DT);
    }
}

#[test]
fn caixa_cai_e_para_no_chao() {
    let mut w = World3::default();
    chao(&mut w);
    let c = caixa(&mut w, Vec3::new(0.0, 5.0, 0.0));
    passos(&mut w, 3.0);
    let p = w.position(c).unwrap();
    assert!((p.y - 0.5).abs() < 0.05, "{p}");
    assert!(w.velocity(c).unwrap().length() < 0.05);
}

#[test]
fn caixas_empilham() {
    let mut w = World3::default();
    chao(&mut w);
    let pilha: Vec<_> = (0..4)
        .map(|i| caixa(&mut w, Vec3::new(0.0, 0.5 + i as f32 * 1.05, 0.0)))
        .collect();
    passos(&mut w, 4.0);
    for (i, c) in pilha.iter().enumerate() {
        let p = w.position(*c).unwrap();
        assert!((p.y - (0.5 + i as f32)).abs() < 0.1, "caixa {i}: {p}");
        assert!(p.x.abs() < 0.1 && p.z.abs() < 0.1, "caixa {i} escorregou: {p}");
    }
}

#[test]
fn bola_quica() {
    let mut w = World3::default();
    chao(&mut w);
    let b = w
        .add(&BodyDesc {
            shape: Shape3::Sphere { radius: 0.5 },
            position: Vec3::new(0.0, 5.0, 0.0),
            bounce: 0.8,
            ..Default::default()
        })
        .unwrap();
    // Cai até o chão e sobe de novo.
    let mut tocou = false;
    let mut subiu = 0.0f32;
    for _ in 0..180 {
        w.step(DT);
        let y = w.position(b).unwrap().y;
        if y < 0.6 {
            tocou = true;
        } else if tocou {
            subiu = subiu.max(y);
        }
    }
    assert!(tocou && subiu > 2.5, "subiu até {subiu}");
}

#[test]
fn sensor_avisa_quem_entra_e_sai() {
    let mut w = World3::default();
    let sensor = w
        .add(&BodyDesc {
            kind: BodyKind3::Static,
            shape: Shape3::Box(Vec3::new(2.0, 1.0, 2.0)),
            position: Vec3::new(0.0, 3.0, 0.0),
            sensor: true,
            ..Default::default()
        })
        .unwrap();
    let c = caixa(&mut w, Vec3::new(0.0, 6.0, 0.0));
    let mut eventos = Vec::new();
    for _ in 0..120 {
        w.step(DT);
        eventos.extend_from_slice(w.events());
    }
    assert_eq!(eventos, [Event3::Enter(sensor, c), Event3::Exit(sensor, c)]);
}

#[test]
fn toque_entre_corpos_solidos() {
    let mut w = World3::default();
    let g = chao(&mut w);
    let c = caixa(&mut w, Vec3::new(0.0, 2.0, 0.0));
    let mut entrou = false;
    for _ in 0..120 {
        w.step(DT);
        entrou |= w
            .events()
            .iter()
            .any(|e| matches!(e, Event3::Enter(a, b) if (*a, *b) == (g, c) || (*a, *b) == (c, g)));
    }
    assert!(entrou);
    assert_eq!(w.touching(c), [g]);
}

#[test]
fn raio_acerta_o_chao_e_ignora_quem_atira() {
    let mut w = World3::default();
    let g = chao(&mut w);
    let c = caixa(&mut w, Vec3::new(0.0, 3.0, 0.0));
    w.step(DT);
    let hit = w
        .raycast(Vec3::new(0.0, 10.0, 0.0), Vec3::NEG_Y, 100.0, None)
        .unwrap();
    assert_eq!(hit.body, c);
    let hit = w
        .raycast(Vec3::new(0.0, 10.0, 0.0), Vec3::NEG_Y, 100.0, Some(c))
        .unwrap();
    assert_eq!(hit.body, g);
    assert!((hit.point.y).abs() < 0.01 && (hit.distance - 10.0).abs() < 0.01);
    assert!((hit.normal - Vec3::Y).length() < 1e-3);
    assert!(w.raycast(Vec3::new(0.0, 10.0, 0.0), Vec3::Y, 100.0, None).is_none());
    assert!(w.query_sphere(Vec3::new(0.0, 3.0, 0.0), 0.2).contains(&c));
}

#[test]
fn malha_exata_segura_e_so_vale_parada() {
    let mut w = World3::default();
    let vertices = vec![
        Vec3::new(-5.0, 0.0, -5.0),
        Vec3::new(5.0, 0.0, -5.0),
        Vec3::new(5.0, 0.0, 5.0),
        Vec3::new(-5.0, 0.0, 5.0),
    ];
    let triangles = vec![[0, 2, 1], [0, 3, 2]];
    w.add(&BodyDesc {
        kind: BodyKind3::Static,
        shape: Shape3::Mesh {
            vertices: vertices.clone(),
            triangles: triangles.clone(),
        },
        ..Default::default()
    })
    .unwrap();
    let c = caixa(&mut w, Vec3::new(1.0, 3.0, 1.0));
    passos(&mut w, 2.0);
    assert!((w.position(c).unwrap().y - 0.5).abs() < 0.05);

    let err = w
        .add(&BodyDesc {
            shape: Shape3::Mesh {
                vertices,
                triangles,
            },
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err, PhysicsError::MeshNotStatic);
    let err = w
        .add(&BodyDesc {
            kind: BodyKind3::Static,
            shape: Shape3::Mesh {
                vertices: vec![Vec3::ZERO],
                triangles: vec![[0, 1, 2]],
            },
            ..Default::default()
        })
        .unwrap_err();
    assert!(err.to_string().contains("inválida"), "{err}");
}

#[test]
fn remover_e_ids() {
    let mut w = World3::default();
    let a = caixa(&mut w, Vec3::ZERO);
    let b = caixa(&mut w, Vec3::X * 3.0);
    assert_eq!(w.len(), 2);
    assert_eq!(BodyId3::from_bits(a.to_bits()), a);
    assert!(w.remove(a));
    assert!(!w.remove(a));
    assert!(!w.contains(a) && w.contains(b));
    assert_eq!(w.ids(), [b]);
    assert!(w.position(a).is_none());
}

// --- Personagem --------------------------------------------------------------

fn personagem(w: &mut World3, p: Vec3) -> BodyId3 {
    w.add_character(&CharacterDesc {
        position: p,
        ..Default::default()
    })
}

#[test]
fn personagem_cai_anda_e_para_na_parede() {
    let mut w = World3::default();
    chao(&mut w);
    let p = personagem(&mut w, Vec3::new(0.0, 2.0, 0.0));
    passos(&mut w, 1.5);
    assert!(w.on_ground(p));
    let pes = w.position(p).unwrap();
    assert!(pes.y.abs() < 0.08, "os pés no chão: {pes}");

    w.walk(p, Vec3::new(2.0, 0.0, 0.0));
    passos(&mut w, 1.0);
    let x = w.position(p).unwrap().x;
    assert!((x - 2.0).abs() < 0.15, "andou {x}");
    assert!(w.on_ground(p));

    // Uma parede em x = 4.
    w.add(&BodyDesc {
        kind: BodyKind3::Static,
        shape: Shape3::Box(Vec3::new(1.0, 3.0, 10.0)),
        position: Vec3::new(4.5, 1.5, 0.0),
        ..Default::default()
    })
    .unwrap();
    passos(&mut w, 2.0);
    let x = w.position(p).unwrap().x;
    assert!(x < 4.0 - 0.3 && x > 3.4, "parou na parede: {x}");
    assert!(w.hit_wall(p));
}

#[test]
fn personagem_sobe_degrau_baixo_mas_nao_o_alto() {
    for (altura, sobe) in [(0.25, true), (0.8, false)] {
        let mut w = World3::default();
        chao(&mut w);
        w.add(&BodyDesc {
            kind: BodyKind3::Static,
            shape: Shape3::Box(Vec3::new(4.0, altura, 4.0)),
            position: Vec3::new(3.0, altura / 2.0, 0.0),
            ..Default::default()
        })
        .unwrap();
        let p = personagem(&mut w, Vec3::new(0.0, 0.1, 0.0));
        passos(&mut w, 0.5);
        w.walk(p, Vec3::new(2.0, 0.0, 0.0));
        passos(&mut w, 1.5);
        let pos = w.position(p).unwrap();
        if sobe {
            assert!(pos.x > 1.5 && (pos.y - altura).abs() < 0.1, "subiu: {pos}");
        } else {
            assert!(pos.x < 1.0 && pos.y.abs() < 0.1, "barrou: {pos}");
        }
    }
}

#[test]
fn personagem_pula_e_volta_ao_chao() {
    let mut w = World3::default();
    chao(&mut w);
    let p = personagem(&mut w, Vec3::new(0.0, 0.5, 0.0));
    passos(&mut w, 1.0);
    assert!(w.on_ground(p));
    assert!(w.jump(p, 6.0));
    assert!(!w.jump(p, 6.0), "no ar não pula de novo");
    let mut alto = 0.0f32;
    for _ in 0..120 {
        w.step(DT);
        alto = alto.max(w.position(p).unwrap().y);
    }
    // v² / 2g = 36 / 19.62 ≈ 1,83
    assert!((alto - 1.83).abs() < 0.2, "pulou {alto}");
    assert!(w.on_ground(p));
}

#[test]
fn personagem_empurra_caixa() {
    let mut w = World3::default();
    chao(&mut w);
    let c = caixa(&mut w, Vec3::new(2.0, 0.5, 0.0));
    let p = personagem(&mut w, Vec3::new(0.0, 0.05, 0.0));
    passos(&mut w, 0.5);
    w.walk(p, Vec3::new(2.0, 0.0, 0.0));
    passos(&mut w, 2.0);
    let x = w.position(c).unwrap().x;
    assert!(x > 2.5, "a caixa andou: {x}");
}

#[test]
fn personagem_entra_no_sensor() {
    let mut w = World3::default();
    chao(&mut w);
    let s = w
        .add(&BodyDesc {
            kind: BodyKind3::Static,
            shape: Shape3::Box(Vec3::new(1.0, 2.0, 1.0)),
            position: Vec3::new(2.0, 1.0, 0.0),
            sensor: true,
            ..Default::default()
        })
        .unwrap();
    let p = personagem(&mut w, Vec3::new(0.0, 0.05, 0.0));
    w.walk(p, Vec3::new(2.0, 0.0, 0.0));
    let mut entrou = false;
    for _ in 0..90 {
        w.step(DT);
        entrou |= w.events().contains(&Event3::Enter(s, p));
    }
    assert!(entrou);
    // O sensor não barra.
    assert!(w.position(p).unwrap().x > 2.5);
}
