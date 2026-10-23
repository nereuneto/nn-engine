//! Esqueletos prontos e animações prontas do modelador.
//!
//! O esqueleto se ajusta ao tamanho do modelo (e às peças do personagem do
//! kit, quando elas estão lá) e liga cada peça a um osso: pelo nome ("Braço
//! esquerdo" no braço esquerdo) ou, se o nome não diz, pelo osso mais perto.
//! Cada peça segue o osso inteira, sem entortar: o jeito dos bonecos de
//! peças.
//!
//! A frente dos modelos é o +Z (como o Jogador 3D vira o modelo), então o
//! lado esquerdo do personagem fica no +X. Girar em X com graus negativos
//! leva um braço (ou uma perna) para a frente.

use glam::Vec3;

use crate::model::{Anim, Bone, ModelData};

pub const QUADRIL: &str = "Quadril";
pub const TRONCO: &str = "Tronco";
pub const CABECA: &str = "Cabeça";
pub const BRACO_E: &str = "Braço esquerdo";
pub const BRACO_D: &str = "Braço direito";
pub const PERNA_E: &str = "Perna esquerda";
pub const PERNA_D: &str = "Perna direita";
pub const CORPO: &str = "Corpo";
pub const RABO: &str = "Rabo";
pub const PATA_DE: &str = "Pata dianteira esquerda";
pub const PATA_DD: &str = "Pata dianteira direita";
pub const PATA_TE: &str = "Pata traseira esquerda";
pub const PATA_TD: &str = "Pata traseira direita";
pub const OSSO: &str = "Osso";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RigKind {
    Humanoid,
    Quadruped,
    Simple,
}

/// Os esqueletos prontos: (tipo, nome, para quê).
pub const RIGS: [(RigKind, &str, &str); 3] = [
    (
        RigKind::Humanoid,
        "🚶 Humanoide",
        "Quadril, tronco, cabeça, braços e pernas: heróis, inimigos, gente",
    ),
    (
        RigKind::Quadruped,
        "🐕 Quadrúpede",
        "Corpo, cabeça, rabo e quatro patas: bichos",
    ),
    (
        RigKind::Simple,
        "📌 Simples",
        "Um osso só: porta, tampa de baú, hélice, bandeira",
    ),
];

/// Sem acento e em minúsculas (para achar "braço" em "Braco_Esq").
fn plain(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' => 'a',
            'é' | 'ê' => 'e',
            'í' => 'i',
            'ó' | 'ô' | 'õ' => 'o',
            'ú' => 'u',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

/// O lado de um nome: esquerdo (+1), direito (-1) ou nenhum (0).
fn side(name: &str) -> i32 {
    let n = plain(name);
    if n.contains("esq") || n.ends_with(".l") || n.contains("left") {
        1
    } else if n.contains("dir") || n.ends_with(".r") || n.contains("right") {
        -1
    } else {
        0
    }
}

/// O osso que um nome de peça pede (pelas palavras dele).
fn bone_for_name(kind: RigKind, part: &str) -> Option<&'static str> {
    let n = plain(part);
    let has = |w: &[&str]| w.iter().any(|w| n.contains(w));
    let s = side(part);
    match kind {
        RigKind::Humanoid => {
            if has(&["cabec", "head", "chapeu", "cabelo", "olho"]) {
                Some(CABECA)
            } else if has(&["braco", "mao", "arm", "hand"]) {
                match s {
                    1 => Some(BRACO_E),
                    -1 => Some(BRACO_D),
                    _ => None,
                }
            } else if has(&["perna", "pe ", "leg", "foot", "bota"]) || n == "pe" {
                match s {
                    1 => Some(PERNA_E),
                    -1 => Some(PERNA_D),
                    _ => None,
                }
            } else if has(&["corpo", "tronco", "peito", "barriga", "camisa", "body"]) {
                Some(TRONCO)
            } else if has(&["quadril", "cintura", "cinto", "hip"]) {
                Some(QUADRIL)
            } else {
                None
            }
        }
        RigKind::Quadruped => {
            if has(&["cabec", "head", "orelha", "focinho"]) {
                Some(CABECA)
            } else if has(&["rabo", "cauda", "tail"]) {
                Some(RABO)
            } else if has(&["pata", "perna", "leg"]) {
                let front = has(&["diant", "front", "frente"]);
                let back = has(&["tras", "back"]);
                match (front, back, s) {
                    (true, _, 1) => Some(PATA_DE),
                    (true, _, -1) => Some(PATA_DD),
                    (_, true, 1) => Some(PATA_TE),
                    (_, true, -1) => Some(PATA_TD),
                    _ => None,
                }
            } else if has(&["corpo", "tronco", "body"]) {
                Some(CORPO)
            } else {
                None
            }
        }
        RigKind::Simple => Some(OSSO),
    }
}

/// A caixa (no modelo) das peças cujo nome pede este osso, se houver.
fn part_box(data: &ModelData, kind: RigKind, bone: &str) -> Option<(Vec3, Vec3)> {
    let mut out: Option<(Vec3, Vec3)> = None;
    for p in data
        .parts
        .iter()
        .filter(|p| bone_for_name(kind, &p.name) == Some(bone))
    {
        let m = p.matrix();
        for (pts, ..) in p.all_faces() {
            for q in pts {
                let q = m.transform_point3(q);
                out = Some(match out {
                    Some((a, b)) => (a.min(q), b.max(q)),
                    None => (q, q),
                });
            }
        }
    }
    out
}

/// O meio de cima (a junta de um braço ou de uma perna) e a altura da peça.
fn limb_top(data: &ModelData, kind: RigKind, bone: &str) -> Option<(Vec3, f32)> {
    part_box(data, kind, bone).map(|(a, b)| {
        let c = (a + b) / 2.0;
        (Vec3::new(c.x, b.y, c.z), (b.y - a.y).max(0.05))
    })
}

fn bone(name: &str, parent: Option<usize>, head: Vec3, tail: Vec3) -> Bone {
    // Em décimos de milímetro (o .modelo guarda assim).
    let r = |v: Vec3| (v * 10000.0).round() / 10000.0;
    Bone {
        name: name.into(),
        parent,
        head: r(head),
        tail: r(tail),
    }
}

/// Um esqueleto pronto, do tamanho do modelo.
pub fn make_rig(kind: RigKind, data: &ModelData) -> Vec<Bone> {
    let (lo, hi) = data
        .bounds()
        .unwrap_or((Vec3::new(-0.5, 0.0, -0.5), Vec3::new(0.5, 1.0, 0.5)));
    let size = (hi - lo).max(Vec3::splat(0.1));
    let c = (lo + hi) / 2.0;
    let at = |fy: f32| lo.y + size.y * fy;
    match kind {
        RigKind::Humanoid => {
            let leg = |b: &str, sx: f32| {
                limb_top(data, kind, b).map(|(p, _)| p).unwrap_or(Vec3::new(
                    c.x + sx * size.x * 0.12,
                    at(0.47),
                    c.z,
                ))
            };
            let leg_l = leg(PERNA_E, 1.0);
            let leg_r = leg(PERNA_D, -1.0);
            let hip = Vec3::new(c.x, (leg_l.y + leg_r.y) / 2.0, c.z);
            let neck = part_box(data, kind, CABECA)
                .map(|(a, b)| Vec3::new((a.x + b.x) / 2.0, a.y, (a.z + b.z) / 2.0))
                .unwrap_or(Vec3::new(c.x, at(0.77), c.z));
            let arm = |b: &str, sx: f32| {
                limb_top(data, kind, b).unwrap_or((
                    Vec3::new(c.x + sx * size.x * 0.3, at(0.74), c.z),
                    size.y * 0.33,
                ))
            };
            let (arm_l, len_l) = arm(BRACO_E, 1.0);
            let (arm_r, len_r) = arm(BRACO_D, -1.0);
            vec![
                bone(QUADRIL, None, hip, hip + Vec3::Y * size.y * 0.1),
                bone(TRONCO, Some(0), hip, neck),
                bone(CABECA, Some(1), neck, Vec3::new(neck.x, hi.y, neck.z)),
                bone(BRACO_E, Some(1), arm_l, arm_l - Vec3::Y * len_l),
                bone(BRACO_D, Some(1), arm_r, arm_r - Vec3::Y * len_r),
                bone(PERNA_E, Some(0), leg_l, Vec3::new(leg_l.x, lo.y, leg_l.z)),
                bone(PERNA_D, Some(0), leg_r, Vec3::new(leg_r.x, lo.y, leg_r.z)),
            ]
        }
        RigKind::Quadruped => {
            // O corpo: do meio da peça "Corpo" para a frente.
            let (body, body_tip) = part_box(data, kind, CORPO)
                .map(|(a, b)| {
                    let m = (a + b) / 2.0;
                    (m, Vec3::new(m.x, m.y, b.z))
                })
                .unwrap_or((
                    Vec3::new(c.x, at(0.55), c.z),
                    Vec3::new(c.x, at(0.55), hi.z - size.z * 0.25),
                ));
            // As patas: do meio de cima da peça com o nome dela (ou de um
            // lugar estimado pelo tamanho do modelo) até o chão.
            let leg = |b: &str, sx: f32, sz: f32| {
                limb_top(data, kind, b).map(|(p, _)| p).unwrap_or(Vec3::new(
                    c.x + sx * size.x * 0.3,
                    at(0.5),
                    c.z + sz * size.z * 0.33,
                ))
            };
            let (de, dd, te, td) = (
                leg(PATA_DE, 1.0, 1.0),
                leg(PATA_DD, -1.0, 1.0),
                leg(PATA_TE, 1.0, -1.0),
                leg(PATA_TD, -1.0, -1.0),
            );
            // O pescoço: o meio de trás da cabeça, embaixo.
            let head = part_box(data, kind, CABECA)
                .map(|(a, b)| Vec3::new((a.x + b.x) / 2.0, a.y, a.z))
                .unwrap_or(Vec3::new(c.x, at(0.7), hi.z - size.z * 0.2));
            let tail = part_box(data, kind, RABO)
                .map(|(a, b)| Vec3::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0, b.z))
                .unwrap_or(Vec3::new(c.x, at(0.6), lo.z + size.z * 0.1));
            let foot = |p: Vec3| Vec3::new(p.x, lo.y, p.z);
            vec![
                bone(CORPO, None, body, body_tip),
                bone(CABECA, Some(0), head, Vec3::new(head.x, hi.y, hi.z)),
                bone(
                    RABO,
                    Some(0),
                    tail,
                    Vec3::new(tail.x, tail.y + size.y * 0.1, lo.z),
                ),
                bone(PATA_DE, Some(0), de, foot(de)),
                bone(PATA_DD, Some(0), dd, foot(dd)),
                bone(PATA_TE, Some(0), te, foot(te)),
                bone(PATA_TD, Some(0), td, foot(td)),
            ]
        }
        RigKind::Simple => vec![bone(
            OSSO,
            None,
            Vec3::new(c.x, lo.y, c.z),
            Vec3::new(c.x, hi.y, c.z),
        )],
    }
}

/// O tipo do esqueleto do modelo (pelos nomes dos ossos).
pub fn kind_of(bones: &[Bone]) -> Option<RigKind> {
    let has = |n: &str| bones.iter().any(|b| b.name == n);
    if has(BRACO_E) && has(PERNA_E) {
        Some(RigKind::Humanoid)
    } else if has(PATA_DE) {
        Some(RigKind::Quadruped)
    } else if !bones.is_empty() {
        Some(RigKind::Simple)
    } else {
        None
    }
}

fn dist_to_segment(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
    (a + ab * t - p).length()
}

/// Liga cada peça a um osso: pelo nome, ou pelo osso mais perto do meio
/// dela. Devolve quantas ligou.
pub fn auto_bind(data: &mut ModelData) -> usize {
    let Some(kind) = kind_of(&data.bones) else {
        return 0;
    };
    let mut n = 0;
    for i in 0..data.parts.len() {
        let p = &data.parts[i];
        let by_name = bone_for_name(kind, &p.name).filter(|b| data.bone_index(b).is_some());
        let chosen = by_name.map(str::to_string).or_else(|| {
            // O meio da peça, no modelo.
            let m = p.matrix();
            let mut sum = Vec3::ZERO;
            let mut k = 0.0;
            for (pts, ..) in p.all_faces() {
                for q in pts {
                    sum += m.transform_point3(q);
                    k += 1.0;
                }
            }
            let center = if k > 0.0 { sum / k } else { p.pos };
            data.bones
                .iter()
                .min_by(|a, b| {
                    dist_to_segment(center, a.head, a.tail)
                        .total_cmp(&dist_to_segment(center, b.head, b.tail))
                })
                .map(|b| b.name.clone())
        });
        if let Some(b) = chosen {
            data.parts[i].bone = b;
            n += 1;
        }
    }
    n
}

// Animações prontas ----------------------------------------------------------

/// A altura que as animações usam para os deslocamentos (o tamanho da perna,
/// ou do modelo).
fn leg_len(bones: &[Bone]) -> f32 {
    bones
        .iter()
        .find(|b| b.name == PERNA_E || b.name == PATA_DE)
        .map(|b| (b.head - b.tail).length())
        .unwrap_or_else(|| {
            bones
                .first()
                .map(|b| (b.head - b.tail).length())
                .unwrap_or(1.0)
        })
        .max(0.1)
}

fn x(deg: f32) -> Vec3 {
    Vec3::new(deg, 0.0, 0.0)
}

fn y(deg: f32) -> Vec3 {
    Vec3::new(0.0, deg, 0.0)
}

fn z(deg: f32) -> Vec3 {
    Vec3::new(0.0, 0.0, deg)
}

fn up(v: f32) -> Vec3 {
    Vec3::new(0.0, v, 0.0)
}

const O: Vec3 = Vec3::ZERO;

/// Uma volta de andar (ou correr): pernas e braços para a frente e para
/// trás, o quadril sobe e desce, o tronco inclina `lean`.
fn stride(name: &str, dur: f32, legs: f32, arms: f32, bob: f32, lean: f32) -> Anim {
    let mut a = Anim::new(name, dur, true);
    for (t, s) in [(0.0, 1.0), (0.5, -1.0), (1.0, 1.0)] {
        let t = t * dur;
        a.key(PERNA_E, t, x(-legs * s), O);
        a.key(PERNA_D, t, x(legs * s), O);
        a.key(BRACO_E, t, x(arms * s), O);
        a.key(BRACO_D, t, x(-arms * s), O);
    }
    for (t, h) in [(0.0, 0.0), (0.25, 1.0), (0.5, 0.0), (0.75, 1.0), (1.0, 0.0)] {
        a.key(QUADRIL, t * dur, O, up(bob * h));
    }
    a.key(TRONCO, 0.0, x(lean), O);
    a.key(TRONCO, dur, x(lean), O);
    a
}

fn humanoid(name: &str, l: f32) -> Option<Anim> {
    Some(match name {
        "parado" => {
            let mut a = Anim::new("parado", 2.0, true);
            for (t, k) in [(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)] {
                a.key(TRONCO, t, x(-2.0 * k), O);
                a.key(QUADRIL, t, O, up(-0.015 * l * k));
                a.key(BRACO_E, t, z(3.0 + 2.0 * k), O);
                a.key(BRACO_D, t, z(-3.0 - 2.0 * k), O);
                a.key(CABECA, t, x(2.0 * k), O);
            }
            a
        }
        "andar" => stride("andar", 1.0, 25.0, 20.0, 0.06 * l, 3.0),
        "correr" => {
            let mut a = stride("correr", 0.6, 45.0, 40.0, 0.12 * l, 12.0);
            // Os braços dobrados para a frente.
            for tr in &mut a.tracks {
                if tr.bone == BRACO_E || tr.bone == BRACO_D {
                    for k in &mut tr.keys {
                        k.rot.x -= 15.0;
                    }
                }
            }
            a
        }
        "pular" => {
            let mut a = Anim::new("pular", 0.8, false);
            // Agacha, estica no ar com os braços para cima, encolhe as pernas.
            for (t, hip, legs, arms) in [
                (0.0, -0.12, -25.0, 20.0),
                (0.2, 0.0, 10.0, -150.0),
                (0.5, 0.05, -40.0, -160.0),
                (0.8, 0.0, -15.0, -60.0),
            ] {
                a.key(QUADRIL, t, O, up(hip * l));
                a.key(PERNA_E, t, x(legs), O);
                a.key(PERNA_D, t, x(legs * 0.7), O);
                a.key(BRACO_E, t, x(arms), O);
                a.key(BRACO_D, t, x(arms), O);
            }
            a
        }
        "cair" => {
            let mut a = Anim::new("cair", 0.6, true);
            for (t, k) in [(0.0, 0.0), (0.3, 1.0), (0.6, 0.0)] {
                a.key(BRACO_E, t, z(70.0 + 15.0 * k), O);
                a.key(BRACO_D, t, z(-70.0 - 15.0 * k), O);
                a.key(PERNA_E, t, x(-15.0 - 10.0 * k), O);
                a.key(PERNA_D, t, x(10.0 + 10.0 * k), O);
            }
            a
        }
        "golpe" => {
            let mut a = Anim::new("golpe", 0.5, false);
            for (t, arm, body) in [
                (0.0, 0.0, 0.0),
                (0.15, -160.0, 25.0),
                (0.3, -50.0, -20.0),
                (0.5, 0.0, 0.0),
            ] {
                a.key(BRACO_D, t, x(arm), O);
                a.key(TRONCO, t, y(body), O);
            }
            a
        }
        "acenar" => {
            let mut a = Anim::new("acenar", 1.2, true);
            for (t, arm) in [
                (0.0, -150.0),
                (0.3, -120.0),
                (0.6, -150.0),
                (0.9, -120.0),
                (1.2, -150.0),
            ] {
                a.key(BRACO_D, t, z(arm), O);
            }
            a.key(CABECA, 0.0, z(-6.0), O);
            a.key(CABECA, 1.2, z(-6.0), O);
            a
        }
        _ => return None,
    })
}

fn quadruped(name: &str, l: f32) -> Option<Anim> {
    let walk = |name: &str, dur: f32, legs: f32, bob: f32| {
        let mut a = Anim::new(name, dur, true);
        for (t, s) in [(0.0, 1.0), (0.5, -1.0), (1.0, 1.0)] {
            let t = t * dur;
            // As patas em X: dianteira esquerda com a traseira direita.
            a.key(PATA_DE, t, x(-legs * s), O);
            a.key(PATA_TD, t, x(-legs * s), O);
            a.key(PATA_DD, t, x(legs * s), O);
            a.key(PATA_TE, t, x(legs * s), O);
            a.key(RABO, t, y(15.0 * s), O);
        }
        for (t, h) in [(0.0, 0.0), (0.25, 1.0), (0.5, 0.0), (0.75, 1.0), (1.0, 0.0)] {
            a.key(CORPO, t * dur, O, up(bob * h));
        }
        a
    };
    Some(match name {
        "parado" => {
            let mut a = Anim::new("parado", 2.0, true);
            for (t, k) in [(0.0, 0.0), (1.0, 1.0), (2.0, 0.0)] {
                a.key(CABECA, t, x(4.0 * k), O);
                a.key(RABO, t, y(-12.0 + 24.0 * k), O);
                a.key(CORPO, t, O, up(-0.01 * l * k));
            }
            a
        }
        "andar" => walk("andar", 0.9, 22.0, 0.04 * l),
        "correr" => walk("correr", 0.5, 40.0, 0.1 * l),
        _ => return None,
    })
}

fn simple(name: &str) -> Option<Anim> {
    Some(match name {
        "girar" => {
            let mut a = Anim::new("girar", 2.0, true);
            for i in 0..=4 {
                a.key(OSSO, i as f32 * 0.5, y(i as f32 * 90.0), O);
            }
            a
        }
        "balançar" => {
            let mut a = Anim::new("balançar", 1.6, true);
            for (t, d) in [(0.0, -10.0), (0.8, 10.0), (1.6, -10.0)] {
                a.key(OSSO, t, z(d), O);
            }
            a
        }
        "abrir" => {
            let mut a = Anim::new("abrir", 0.8, false);
            a.key(OSSO, 0.0, O, O);
            a.key(OSSO, 0.8, x(-100.0), O);
            a
        }
        "abrir_porta" => {
            let mut a = Anim::new("abrir_porta", 0.8, false);
            a.key(OSSO, 0.0, O, O);
            a.key(OSSO, 0.8, y(-90.0), O);
            a
        }
        _ => return None,
    })
}

/// O osso que uma animação pronta do esqueleto simples move, num modelo sem
/// o osso "Osso": o do nome que pede (uma "Tampa", uma "Porta", uma
/// "Dobradiça", uma "Hélice"); senão, para abrir, o último osso sem filhos
/// (a tampa presa na base) e, para girar e balançar, o primeiro sem pai.
pub fn simple_bone<'a>(anim: &str, bones: &'a [Bone]) -> Option<&'a str> {
    let opening = anim.starts_with("abrir");
    let hints: &[&str] = if opening {
        &[
            "tampa", "porta", "dobradic", "portao", "janela", "lid", "door", "hinge",
        ]
    } else {
        &["helice", "roda", "moinho", "lamina"]
    };
    if let Some(b) = bones
        .iter()
        .find(|b| hints.iter().any(|h| plain(&b.name).contains(h)))
    {
        return Some(&b.name);
    }
    let leaf = |i: usize| !bones.iter().any(|b| b.parent == Some(i));
    let pick = if opening {
        (0..bones.len()).rev().find(|&i| leaf(i))
    } else {
        bones.iter().position(|b| b.parent.is_none())
    };
    pick.or(if bones.is_empty() { None } else { Some(0) })
        .map(|i| bones[i].name.as_str())
}

/// Os nomes das animações prontas de cada esqueleto (os que o Jogador 3D,
/// o Inimigo 3D e o Chefe procuram vêm primeiro).
pub fn ready_names(kind: RigKind) -> &'static [&'static str] {
    match kind {
        RigKind::Humanoid => &[
            "parado", "andar", "correr", "pular", "cair", "golpe", "acenar",
        ],
        RigKind::Quadruped => &["parado", "andar", "correr"],
        RigKind::Simple => &["girar", "balançar", "abrir", "abrir_porta"],
    }
}

/// Uma animação pronta para o esqueleto do modelo.
pub fn ready(kind: RigKind, name: &str, bones: &[Bone]) -> Option<Anim> {
    let l = leg_len(bones);
    match kind {
        RigKind::Humanoid => humanoid(name, l),
        RigKind::Quadruped => quadruped(name, l),
        RigKind::Simple => {
            let mut a = simple(name)?;
            if !bones.iter().any(|b| b.name == OSSO)
                && let Some(b) = simple_bone(name, bones)
            {
                for tr in &mut a.tracks {
                    if tr.bone == OSSO {
                        tr.bone = b.to_string();
                    }
                }
            }
            Some(a)
        }
    }
}

/// Mais forte (ou mais fraca, `f` < 1): os giros e os deslocamentos.
pub fn scale_intensity(a: &mut Anim, f: f32) {
    for tr in &mut a.tracks {
        for k in &mut tr.keys {
            k.rot *= f;
            k.offset *= f;
        }
    }
}

/// Mais rápida (`f` < 1 encurta) ou mais lenta.
pub fn scale_time(a: &mut Anim, f: f32) {
    a.duration = (a.duration * f).max(0.05);
    for tr in &mut a.tracks {
        for k in &mut tr.keys {
            k.t *= f;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_kits::READY;

    fn personagem() -> ModelData {
        (READY.iter().find(|r| r.name == "Personagem").unwrap().make)()
    }

    #[test]
    fn humanoide_liga_o_personagem_pelo_nome() {
        let mut m = personagem();
        m.bones = make_rig(RigKind::Humanoid, &m);
        assert_eq!(auto_bind(&mut m), m.parts.len());
        let bone_of = |p: &str| m.parts.iter().find(|x| x.name == p).unwrap().bone.clone();
        assert_eq!(bone_of("Braço esquerdo"), BRACO_E);
        assert_eq!(bone_of("Perna direita"), PERNA_D);
        assert_eq!(bone_of("Cabeça"), CABECA);
        assert_eq!(bone_of("Corpo"), TRONCO);
        // As juntas ficam no ombro e no quadril das peças.
        let b = &m.bones[m.bone_index(BRACO_E).unwrap()];
        assert!((b.head - Vec3::new(0.39, 1.45, 0.0)).length() < 1e-4);
    }

    #[test]
    fn sem_nome_liga_pelo_mais_perto() {
        let mut m = personagem();
        for p in &mut m.parts {
            p.name = format!("peça {}", p.pos.x);
        }
        m.bones = make_rig(RigKind::Humanoid, &m);
        auto_bind(&mut m);
        let arm = m.parts.iter().find(|p| p.pos.x > 0.3).unwrap();
        assert_eq!(arm.bone, BRACO_E, "o braço da esquerda (+X)");
    }

    #[test]
    fn andar_balanca_o_braco_e_volta_ao_comeco() {
        let mut m = personagem();
        m.bones = make_rig(RigKind::Humanoid, &m);
        auto_bind(&mut m);
        let a = ready(RigKind::Humanoid, "andar", &m.bones).unwrap();
        let rest = m.posed_matrices(None, 0.0);
        let p0 = m.posed_matrices(Some(&a), 0.0);
        let p_mid = m.posed_matrices(Some(&a), 0.5);
        let p_end = m.posed_matrices(Some(&a), 1.0);
        let i = m
            .parts
            .iter()
            .position(|p| p.name == "Braço direito")
            .unwrap();
        // A mão (o fim do braço) vai para a frente e para trás.
        let hand = Vec3::new(-0.39, 0.85, 0.0);
        let at = |mats: &[glam::Mat4]| mats[i] * rest[i].inverse() * hand.extend(1.0);
        assert!(at(&p0).z > 0.1, "para a frente: {}", at(&p0));
        assert!(at(&p_mid).z < -0.1, "para trás: {}", at(&p_mid));
        assert!((at(&p0) - at(&p_end)).length() < 1e-4, "a volta fecha");
    }

    #[test]
    fn todas_as_prontas_existem_e_mudam_algo() {
        for (kind, ..) in RIGS {
            let mut m = personagem();
            m.bones = make_rig(kind, &m);
            for name in ready_names(kind) {
                let a = ready(kind, name, &m.bones).unwrap();
                assert!(!a.tracks.is_empty(), "{name}");
                for tr in &a.tracks {
                    assert!(m.bone_index(&tr.bone).is_some(), "{name}: {}", tr.bone);
                    assert!(tr.keys.iter().all(|k| k.t <= a.duration + 1e-4));
                }
            }
        }
    }

    #[test]
    fn glb_com_esqueleto_toca_na_engine() {
        let mut m = personagem();
        m.bones = make_rig(RigKind::Humanoid, &m);
        auto_bind(&mut m);
        for name in ready_names(RigKind::Humanoid) {
            m.anims
                .push(ready(RigKind::Humanoid, name, &m.bones).unwrap());
        }
        let glb = m.to_glb();
        let data = nn_render::d3::ModelData::from_gltf(&glb, &mut |_| None).unwrap();
        assert_eq!(data.triangle_count(), m.triangle_count());
        let (lo, hi) = data.bounds();
        let (elo, ehi) = m.bounds().unwrap();
        assert!((lo - elo).length() < 1e-3 && (hi - ehi).length() < 1e-3);
        let s = data.skeleton.as_ref().expect("esqueleto");
        let names: Vec<&str> = s.animations.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ready_names(RigKind::Humanoid));
        let andar = s.animation("andar").unwrap();
        assert!((s.animations[andar].duration - 1.0).abs() < 1e-4);
        // Os ossos posados no meio do andar não estão parados.
        let rest = s.joint_matrices(&s.rest_pose());
        let mid = s.joint_matrices(&s.pose(andar, 0.5));
        assert!(rest.iter().zip(&mid).any(|(a, b)| !a.abs_diff_eq(*b, 1e-3)));
        // E o .modelo guarda tudo.
        let back = ModelData::from_lua(&m.to_lua(), "p.modelo").unwrap();
        assert_eq!(back.bones, m.bones);
        assert_eq!(back.anims.len(), m.anims.len());
        assert_eq!(back.anims[1].tracks.len(), m.anims[1].tracks.len());
        assert_eq!(back.parts[0].bone, m.parts[0].bone);
    }

    #[test]
    fn intensidade_e_velocidade() {
        let mut a = humanoid("andar", 1.0).unwrap();
        scale_intensity(&mut a, 2.0);
        assert_eq!(a.track(PERNA_E).unwrap().keys[0].rot.x, -50.0);
        scale_time(&mut a, 0.5);
        assert!((a.duration - 0.5).abs() < 1e-6);
        assert!((a.track(PERNA_E).unwrap().keys[1].t - 0.25).abs() < 1e-6);
    }

    #[test]
    fn patas_de_cubos_giram_em_cima_delas() {
        use crate::model::Part;
        let mut m = ModelData::default();
        let mut corpo = Part::cube("Corpo", "#a0522d");
        corpo.pos = Vec3::new(0.0, 0.4, 0.0);
        corpo.scale = Vec3::new(0.4, 0.35, 0.9);
        m.parts.push(corpo);
        for (nome, x, z) in [
            ("Pata dianteira esquerda", 0.15, 0.35),
            ("Pata dianteira direita", -0.15, 0.35),
            ("Pata traseira esquerda", 0.15, -0.35),
            ("Pata traseira direita", -0.15, -0.35),
        ] {
            let mut p = Part::cube(nome, "#888888");
            p.pos = Vec3::new(x, 0.0, z);
            p.scale = Vec3::new(0.1, 0.4, 0.1);
            m.parts.push(p);
        }
        m.bones = make_rig(RigKind::Quadruped, &m);
        let b = &m.bones[m.bone_index(PATA_TD).unwrap()];
        assert!(
            (b.head - Vec3::new(-0.15, 0.4, -0.35)).length() < 0.02,
            "{}",
            b.head
        );
        assert!(b.tail.y.abs() < 1e-4);
    }

    #[test]
    fn abrir_porta_vai_para_o_osso_da_porta() {
        let bones = vec![bone(
            "Dobradiça",
            None,
            Vec3::new(-0.5, 0.0, 0.0),
            Vec3::new(-0.5, 2.0, 0.0),
        )];
        let a = ready(RigKind::Simple, "abrir_porta", &bones).unwrap();
        assert_eq!(a.tracks.len(), 1);
        assert_eq!(a.tracks[0].bone, "Dobradiça");
        assert!(!a.looping);
        assert!(ready_names(RigKind::Simple).contains(&"abrir_porta"));
        // Um baú: a base e a tampa (filha dela). Abrir vai para a tampa.
        let bau = vec![
            bone("Base", None, Vec3::ZERO, Vec3::Y * 0.5),
            bone(
                "Cima",
                Some(0),
                Vec3::new(0.0, 0.5, -0.3),
                Vec3::new(0.0, 0.5, 0.3),
            ),
        ];
        assert_eq!(simple_bone("abrir", &bau), Some("Cima"));
        assert_eq!(simple_bone("girar", &bau), Some("Base"));
        let a = ready(RigKind::Simple, "abrir", &bau).unwrap();
        assert_eq!(a.tracks[0].bone, "Cima");
    }
}
