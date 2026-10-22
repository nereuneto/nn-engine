//! Peças prontas e modelos prontos do modelador.
//!
//! - **Kits:** peças que se encaixam (Casa, Natureza, Personagem, Veículo),
//!   no ➕ Peça. As do personagem já vêm com o ponto de giro na junta (o
//!   ombro, o quadril), para a S49 ligar cada uma a um osso.
//! - **Biblioteca:** modelos prontos para começar (árvore, cerca, casinha...)
//!   e os que você guarda (⭐ Guardar na biblioteca), numa pasta ao lado das
//!   preferências: valem para todos os projetos.

use std::path::{Path, PathBuf};

use glam::{Mat4, Quat, Vec3};

use crate::model_data::{DEFAULT_PALETTE, ModelData, Part};

const MADEIRA: &str = "#9C6B43";
const MADEIRA_ESCURA: &str = "#6B4423";
const FOLHA: &str = "#2E8B57";
const FOLHA_CLARA: &str = "#7BC86C";
const PEDRA: &str = "#B8B8B8";
const PELE: &str = "#F2C9A0";
const AZUL: &str = "#3FA7D6";
const AZUL_ESCURO: &str = "#2C5D9E";
const VERMELHO: &str = "#E84A4A";
const AMARELO: &str = "#FFD166";
const PRETO: &str = "#1E1E1E";
const PAREDE: &str = "#F2C9A0";
const OURO: &str = "#C9A85C";

/// Uma caixa com a base em y = 0, do tamanho dado.
pub fn boxed(name: &str, size: Vec3, color: &str) -> Part {
    Part::cube(name, color).baked(Mat4::from_scale(size))
}

/// Um prisma de `n` lados (cilindro; `top` = 0 faz um cone).
fn ring(name: &str, color: &str, n: u32, top: f32, size: Vec3) -> Part {
    Part::ring_solid(name, color, n, top).baked(Mat4::from_scale(size))
}

fn at(mut p: Part, pos: Vec3) -> Part {
    p.pos = pos;
    p
}

/// Uma parede (2 m × 1,6 m × 0,2 m), com a frente virada para +Z.
fn wall(name: &str) -> Part {
    boxed(name, Vec3::new(2.0, 1.6, 0.2), PAREDE)
}

/// Abre um buraco na frente (+Z) da parede: uma face menor, levada para
/// `y0..y1` e `-w..w`, afundada `depth` e pintada de `color`.
fn opening(mut p: Part, w: f32, y0: f32, y1: f32, depth: f32, color: &str) -> Part {
    let Some(front) = p.face_towards(Vec3::Z) else {
        return p;
    };
    p.inset(&[front], 0.3);
    let inner = p.faces[front].v.clone();
    let mid = inner.iter().map(|&k| p.verts[k as usize].y).sum::<f32>() / inner.len() as f32;
    for &k in &inner {
        let v = &mut p.verts[k as usize];
        v.y = if v.y < mid { y0 } else { y1 };
        v.x = if v.x < 0.0 { -w } else { w };
    }
    let (moved, _) = p.extrude(&[front]);
    for k in moved {
        p.verts[k as usize].z -= depth;
    }
    p.faces[front].color = color.into();
    p
}

/// Telhado de duas águas com a base de 1 × 1 m (encaixa no tamanho da face).
fn gable(name: &str) -> Part {
    let mut p = Part::new(name);
    p.verts = vec![
        Vec3::new(-0.5, 0.0, -0.5),
        Vec3::new(0.5, 0.0, -0.5),
        Vec3::new(0.5, 0.0, 0.5),
        Vec3::new(-0.5, 0.0, 0.5),
        Vec3::new(-0.5, 0.6, 0.0),
        Vec3::new(0.5, 0.6, 0.0),
    ];
    p.face(&[0, 1, 2, 3], MADEIRA_ESCURA);
    p.face(&[3, 2, 5, 4], VERMELHO);
    p.face(&[1, 0, 4, 5], VERMELHO);
    p.face(&[2, 1, 5], PAREDE);
    p.face(&[0, 3, 4], PAREDE);
    p
}

fn trunk() -> Part {
    ring("Tronco", MADEIRA_ESCURA, 6, 0.8, Vec3::new(0.3, 1.2, 0.3))
}

fn crown() -> Part {
    Part::sphere_n("Copa", FOLHA, 7, 4).baked(Mat4::from_scale(Vec3::new(1.5, 1.3, 1.5)))
}

fn pine_top() -> Part {
    let mut p = ring("Copa", FOLHA, 7, 0.0, Vec3::new(1.5, 1.1, 1.5));
    p.merge(
        &ring("", FOLHA, 7, 0.0, Vec3::new(1.1, 0.9, 1.1))
            .baked(Mat4::from_translation(Vec3::Y * 0.65)),
    );
    p.merge(
        &ring("", FOLHA, 7, 0.0, Vec3::new(0.7, 0.7, 0.7))
            .baked(Mat4::from_translation(Vec3::Y * 1.2)),
    );
    p
}

/// Uma pedra: uma esfera achatada, um pouco torta (sempre do mesmo jeito).
fn rock() -> Part {
    let mut p =
        Part::sphere_n("Pedra", PEDRA, 6, 4).baked(Mat4::from_scale(Vec3::new(0.9, 0.55, 0.7)));
    for (i, v) in p.verts.iter_mut().enumerate() {
        let k = (i as f32 * 2.399).sin() * 0.06;
        if v.y > 0.01 {
            v.x += k;
            v.z -= k * 0.7;
            v.y += k * 0.5;
        }
    }
    p
}

fn bush() -> Part {
    Part::sphere_n("Arbusto", FOLHA_CLARA, 7, 4).baked(Mat4::from_scale(Vec3::new(0.8, 0.5, 0.8)))
}

// O personagem: cada peça com o ponto de giro na junta.

fn head() -> Part {
    boxed("Cabeça", Vec3::splat(0.45), PELE)
}

fn body() -> Part {
    boxed("Corpo", Vec3::new(0.6, 0.75, 0.35), AZUL)
}

/// Um braço pendurado no ombro (a origem fica em cima).
fn arm(name: &str) -> Part {
    boxed(name, Vec3::new(0.18, 0.65, 0.18), PELE).baked(Mat4::from_translation(Vec3::Y * -0.65))
}

/// Uma perna pendurada no quadril (a origem fica em cima).
fn leg(name: &str) -> Part {
    boxed(name, Vec3::new(0.22, 0.75, 0.22), AZUL_ESCURO)
        .baked(Mat4::from_translation(Vec3::Y * -0.75))
}

/// O personagem olha para +Z (a frente dos modelos no jogo): o lado
/// esquerdo dele fica no +X.
fn character() -> Vec<Part> {
    vec![
        at(leg("Perna esquerda"), Vec3::new(0.15, 0.75, 0.0)),
        at(leg("Perna direita"), Vec3::new(-0.15, 0.75, 0.0)),
        at(body(), Vec3::new(0.0, 0.75, 0.0)),
        at(head(), Vec3::new(0.0, 1.5, 0.0)),
        at(arm("Braço esquerdo"), Vec3::new(0.39, 1.45, 0.0)),
        at(arm("Braço direito"), Vec3::new(-0.39, 1.45, 0.0)),
    ]
}

/// Uma roda deitada de lado (gira em volta de X), com o meio na altura do
/// raio.
fn wheel() -> Part {
    ring("Roda", PRETO, 10, 1.0, Vec3::new(0.6, 0.25, 0.6)).baked(
        Mat4::from_translation(Vec3::new(-0.125, 0.3, 0.0))
            * Mat4::from_quat(Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2)),
    )
}

fn car_body() -> Part {
    let mut p = boxed("Carroceria", Vec3::new(1.2, 0.5, 2.2), VERMELHO);
    if let Some(top) = p.face_towards(Vec3::Y) {
        p.inset(&[top], 0.25);
        let (moved, _) = p.extrude(&[top]);
        for k in moved {
            p.verts[k as usize].y += 0.4;
        }
        p.faces[top].color = VERMELHO.into();
        // Os vidros: as paredes da cabine.
        let n = p.faces.len();
        for f in n - 4..n {
            p.faces[f].color = AZUL.into();
        }
    }
    p
}

fn car() -> Vec<Part> {
    let mut out = vec![at(car_body(), Vec3::new(0.0, 0.3, 0.0))];
    for (i, (x, z)) in [(-0.6, -0.7), (0.6, -0.7), (-0.6, 0.7), (0.6, 0.7)]
        .into_iter()
        .enumerate()
    {
        let mut w = wheel();
        if x < 0.0 {
            w = w.baked(Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)));
        }
        w.name = format!("Roda {}", i + 1);
        out.push(at(w, Vec3::new(x, 0.0, z)));
    }
    out
}

/// Uma peça (ou um grupo de peças) de um kit.
#[derive(Debug, Clone, Copy)]
pub struct KitItem {
    pub label: &'static str,
    pub make: fn() -> Vec<Part>,
    /// Encaixa no tamanho da face onde é posta (o telhado na parede).
    pub fit: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Kit {
    pub name: &'static str,
    pub icon: &'static str,
    pub items: &'static [KitItem],
}

macro_rules! item {
    ($label:expr, $fit:expr, $body:expr) => {
        KitItem {
            label: $label,
            make: || $body,
            fit: $fit,
        }
    };
}

pub const KITS: [Kit; 4] = [
    Kit {
        name: "Casa",
        icon: "🏡",
        items: &[
            item!("Parede", false, vec![wall("Parede")]),
            item!(
                "Parede com porta",
                false,
                vec![opening(
                    wall("Parede com porta"),
                    0.4,
                    0.0,
                    1.15,
                    0.1,
                    MADEIRA_ESCURA
                )]
            ),
            item!(
                "Parede com janela",
                false,
                vec![opening(
                    wall("Parede com janela"),
                    0.4,
                    0.6,
                    1.2,
                    0.06,
                    AZUL
                )]
            ),
            item!("Telhado", true, vec![gable("Telhado")]),
            item!(
                "Chaminé",
                false,
                vec![boxed("Chaminé", Vec3::new(0.3, 0.8, 0.3), PEDRA)]
            ),
            item!(
                "Porta",
                false,
                vec![boxed("Porta", Vec3::new(0.8, 1.2, 0.08), MADEIRA)]
            ),
        ],
    },
    Kit {
        name: "Natureza",
        icon: "🌳",
        items: &[
            item!("Tronco", false, vec![trunk()]),
            item!("Copa", false, vec![crown()]),
            item!("Copa de pinheiro", false, vec![pine_top()]),
            item!("Pedra", false, vec![rock()]),
            item!("Arbusto", false, vec![bush()]),
        ],
    },
    Kit {
        name: "Personagem",
        icon: "👤",
        items: &[
            item!("Personagem inteiro", false, character()),
            item!("Cabeça", false, vec![head()]),
            item!("Corpo", false, vec![body()]),
            item!("Braço", false, vec![arm("Braço")]),
            item!("Perna", false, vec![leg("Perna")]),
        ],
    },
    Kit {
        name: "Veículo",
        icon: "🚗",
        items: &[
            item!("Carrinho inteiro", false, car()),
            item!("Carroceria", false, vec![car_body()]),
            item!("Roda", false, vec![wheel()]),
        ],
    },
];

// Biblioteca ------------------------------------------------------------------

fn model(parts: Vec<Part>) -> ModelData {
    ModelData {
        parts,
        palette: DEFAULT_PALETTE.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    }
}

fn tree() -> ModelData {
    model(vec![trunk(), at(crown(), Vec3::Y * 1.0)])
}

fn pine() -> ModelData {
    model(vec![trunk(), at(pine_top(), Vec3::Y * 0.7)])
}

fn fence() -> ModelData {
    let mut parts = Vec::new();
    for (i, x) in [-1.0, 0.0, 1.0].into_iter().enumerate() {
        parts.push(at(
            boxed(
                &format!("Estaca {}", i + 1),
                Vec3::new(0.14, 1.0, 0.14),
                MADEIRA,
            ),
            Vec3::new(x, 0.0, 0.0),
        ));
    }
    for (i, y) in [0.35, 0.75].into_iter().enumerate() {
        parts.push(at(
            boxed(
                &format!("Ripa {}", i + 1),
                Vec3::new(2.2, 0.1, 0.06),
                MADEIRA_ESCURA,
            ),
            Vec3::new(0.0, y, 0.08),
        ));
    }
    model(parts)
}

fn house() -> ModelData {
    let mut walls = boxed("Paredes", Vec3::new(2.0, 1.6, 2.0), PAREDE);
    walls = opening(walls, 0.35, 0.0, 1.1, 0.08, MADEIRA_ESCURA);
    if let Some(side) = walls.face_towards(Vec3::X) {
        walls.inset(&[side], 0.3);
        let (moved, _) = walls.extrude(&[side]);
        for k in moved {
            walls.verts[k as usize].x -= 0.05;
        }
        walls.faces[side].color = AZUL.into();
    }
    let mut roof = gable("Telhado");
    roof.scale = Vec3::new(2.4, 1.2, 2.4);
    model(vec![
        walls,
        at(roof, Vec3::Y * 1.6),
        at(
            boxed("Chaminé", Vec3::new(0.3, 0.8, 0.3), PEDRA),
            Vec3::new(0.55, 1.75, -0.45),
        ),
    ])
}

fn chest() -> ModelData {
    let mut lock = boxed("Fecho", Vec3::new(0.16, 0.2, 0.06), OURO);
    lock.pos = Vec3::new(0.0, 0.45, 0.32);
    model(vec![
        boxed("Caixa", Vec3::new(1.0, 0.55, 0.6), MADEIRA),
        at(
            boxed("Tampa", Vec3::new(1.04, 0.22, 0.64), MADEIRA_ESCURA),
            Vec3::Y * 0.55,
        ),
        lock,
    ])
}

fn lamp_post() -> ModelData {
    model(vec![
        ring("Poste", PRETO, 6, 1.0, Vec3::new(0.12, 2.0, 0.12)),
        at(
            boxed("Luz", Vec3::new(0.32, 0.4, 0.32), AMARELO),
            Vec3::Y * 2.0,
        ),
        at(
            Part::pyramid("Chapéu", PRETO).baked(Mat4::from_scale(Vec3::new(0.5, 0.25, 0.5))),
            Vec3::Y * 2.4,
        ),
    ])
}

fn rock_model() -> ModelData {
    model(vec![rock()])
}

fn character_model() -> ModelData {
    model(character())
}

fn car_model() -> ModelData {
    model(car())
}

fn campfire() -> ModelData {
    let mut parts = Vec::new();
    for i in 0..3 {
        let a = i as f32 * 60.0;
        let mut log = boxed(
            &format!("Lenha {}", i + 1),
            Vec3::new(1.0, 0.14, 0.14),
            MADEIRA_ESCURA,
        );
        log.rot = Vec3::new(0.0, a, 0.0);
        parts.push(log);
    }
    parts.push(at(
        ring("Fogo", "#FF8C42", 5, 0.0, Vec3::new(0.5, 0.7, 0.5)),
        Vec3::Y * 0.12,
    ));
    parts.push(at(
        ring("Brasa", AMARELO, 5, 0.0, Vec3::new(0.3, 0.45, 0.3)),
        Vec3::Y * 0.14,
    ));
    model(parts)
}

/// Um modelo pronto da biblioteca.
#[derive(Debug, Clone, Copy)]
pub struct Ready {
    pub name: &'static str,
    /// Nome do arquivo novo, sem extensão.
    pub file: &'static str,
    pub make: fn() -> ModelData,
}

pub const READY: [Ready; 10] = [
    Ready {
        name: "Árvore",
        file: "arvore",
        make: tree,
    },
    Ready {
        name: "Pinheiro",
        file: "pinheiro",
        make: pine,
    },
    Ready {
        name: "Pedra",
        file: "pedra",
        make: rock_model,
    },
    Ready {
        name: "Cerca",
        file: "cerca",
        make: fence,
    },
    Ready {
        name: "Casinha",
        file: "casinha",
        make: house,
    },
    Ready {
        name: "Baú",
        file: "bau",
        make: chest,
    },
    Ready {
        name: "Lampião",
        file: "lampiao",
        make: lamp_post,
    },
    Ready {
        name: "Fogueira",
        file: "fogueira",
        make: campfire,
    },
    Ready {
        name: "Personagem",
        file: "personagem",
        make: character_model,
    },
    Ready {
        name: "Carrinho",
        file: "carrinho",
        make: car_model,
    },
];

/// A pasta da biblioteca pessoal (ao lado das preferências).
pub fn user_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("biblioteca")
}

/// Os modelos guardados na biblioteca pessoal: (nome, arquivo), em ordem.
pub fn user_models(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, PathBuf)> = read
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "modelo"))
        .map(|p| {
            let name = p
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            (name, p)
        })
        .collect();
    out.sort_by_key(|a| a.0.to_lowercase());
    out
}

/// Lê um modelo da biblioteca pessoal.
pub fn read_user(path: &Path) -> Result<ModelData, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    ModelData::from_lua(&text, &path.to_string_lossy())
}

/// Guarda peças na biblioteca pessoal com o nome dado (sem repetir: "Mesa",
/// "Mesa 2"...). Devolve o arquivo.
pub fn save_user(dir: &Path, name: &str, model: &ModelData) -> Result<PathBuf, String> {
    let clean: String = name
        .trim()
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .collect();
    let clean = if clean.is_empty() {
        "peças".to_string()
    } else {
        clean
    };
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut path = dir.join(format!("{clean}.modelo"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{clean} {n}.modelo"));
        n += 1;
    }
    std::fs::write(&path, model.to_lua()).map_err(|e| e.to_string())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn closed(p: &Part) -> bool {
        let mut edges: HashMap<(u32, u32), i32> = HashMap::new();
        for f in &p.faces {
            for i in 0..f.v.len() {
                let (a, b) = (f.v[i], f.v[(i + 1) % f.v.len()]);
                *edges.entry((a, b)).or_default() += 1;
            }
        }
        edges
            .iter()
            .all(|(&(a, b), &n)| n == 1 && edges.get(&(b, a)) == Some(&1))
    }

    #[test]
    fn pecas_dos_kits_fechadas_e_em_pe() {
        for kit in KITS {
            for item in kit.items {
                let parts = (item.make)();
                assert!(!parts.is_empty(), "{}", item.label);
                for p in &parts {
                    assert!(closed(p), "{} / {} fechada", item.label, p.name);
                    assert!(!p.faces.is_empty());
                }
            }
        }
    }

    #[test]
    fn modelos_prontos_abrem_na_engine() {
        for r in READY {
            let m = (r.make)();
            let (lo, hi) = m.bounds().unwrap();
            assert!(lo.y > -0.01, "{} no chão: {lo}", r.name);
            assert!(hi.y > 0.2, "{} tem altura", r.name);
            let glb = m.to_glb();
            let data = nn_render::d3::ModelData::from_gltf(&glb, &mut |_| None).unwrap();
            assert_eq!(data.triangle_count(), m.triangle_count(), "{}", r.name);
            let back = ModelData::from_lua(&m.to_lua(), r.file).unwrap();
            assert_eq!(back.parts.len(), m.parts.len());
        }
    }

    #[test]
    fn personagem_com_giro_nas_juntas() {
        let parts = character();
        let arm = parts.iter().find(|p| p.name == "Braço direito").unwrap();
        // O ombro (a origem) fica no alto do braço.
        assert!(arm.verts.iter().all(|v| v.y <= 1e-5));
        assert!(arm.verts.iter().any(|v| v.y < -0.6));
    }

    #[test]
    fn biblioteca_pessoal_guarda_sem_repetir_o_nome() {
        let dir = std::env::temp_dir().join(format!("nn-biblioteca-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let m = tree();
        let a = save_user(&dir, "Minha árvore", &m).unwrap();
        let b = save_user(&dir, "Minha árvore", &m).unwrap();
        assert_ne!(a, b);
        let list = user_models(&dir);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].0, "Minha árvore");
        assert_eq!(read_user(&list[1].1).unwrap().parts.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
