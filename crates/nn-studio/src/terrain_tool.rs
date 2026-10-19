//! Os pincéis do terreno na vista 3D: subir, descer, alisar, aplainar,
//! ruído, pintar as cores e espalhar árvores (ou qualquer modelo ou prefab).

use glam::{Vec2, Vec3};
use nn_core::Color;
use nn_render::d3::{BrushOp, TerrainData};

use crate::scene_data::{Component, Object, Prop};
use crate::scene_render::parse_color;

/// Qual pincel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerrainBrush {
    #[default]
    Raise,
    Lower,
    Smooth,
    Flatten,
    Noise,
    Paint,
    Scatter,
}

impl TerrainBrush {
    pub const ALL: [TerrainBrush; 7] = [
        TerrainBrush::Raise,
        TerrainBrush::Lower,
        TerrainBrush::Smooth,
        TerrainBrush::Flatten,
        TerrainBrush::Noise,
        TerrainBrush::Paint,
        TerrainBrush::Scatter,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TerrainBrush::Raise => "⬆ Subir",
            TerrainBrush::Lower => "⬇ Descer",
            TerrainBrush::Smooth => "Alisar",
            TerrainBrush::Flatten => "Aplainar",
            TerrainBrush::Noise => "Ruído",
            TerrainBrush::Paint => "Pintar cor",
            TerrainBrush::Scatter => "Espalhar",
        }
    }

    pub fn tip(self) -> &'static str {
        match self {
            TerrainBrush::Raise => "Sobe o chão onde você passa (Ctrl desce). Morros, montanhas.",
            TerrainBrush::Lower => "Desce o chão (Ctrl sobe). Vales, buracos, lagos.",
            TerrainBrush::Smooth => "Deixa mais suave: tira os bicos e degraus.",
            TerrainBrush::Flatten => {
                "Puxa para a altura de onde você começou a arrastar: estradas, lugares para construir."
            }
            TerrainBrush::Noise => "Calombos sorteados: o chão fica menos liso.",
            TerrainBrush::Paint => {
                "Pinta uma das quatro cores do terreno (as cores ficam no Inspetor)."
            }
            TerrainBrush::Scatter => {
                "Espalha cópias de um modelo ou prefab no chão, com tamanho e giro sorteados (Ctrl apaga)."
            }
        }
    }
}

/// O estado dos pincéis de uma aba de cena.
#[derive(Debug, Clone)]
pub struct TerrainTool {
    /// Pintando (o botão esquerdo pinta em vez de selecionar).
    pub active: bool,
    pub brush: TerrainBrush,
    /// Raio, em metros.
    pub radius: f32,
    /// Força (metros por segundo nos pincéis de altura).
    pub strength: f32,
    /// A cor que o Pintar cor usa (0 a 3).
    pub layer: usize,
    /// O que o 🌲 Espalhar põe (um .glb ou um .prefab).
    pub scatter: String,
    /// Quantos por segundo o 🌲 Espalhar põe.
    pub density: f32,
    /// Onde o pincel está agora (no mundo), para desenhar o círculo.
    pub cursor: Option<Vec3>,
    /// Altura do Aplainar (a do começo do arrasto, no terreno).
    pub flatten_to: Option<f32>,
    /// Conta os arrastos (cada um é um passo para desfazer).
    pub stroke: u32,
    pub painting: bool,
    /// O Espalhar: tempo acumulado até o próximo.
    pub carry: f32,
    seed: u32,
}

impl Default for TerrainTool {
    fn default() -> Self {
        Self {
            active: false,
            brush: TerrainBrush::Raise,
            radius: 5.0,
            strength: 3.0,
            layer: 1,
            scatter: String::new(),
            density: 6.0,
            cursor: None,
            flatten_to: None,
            stroke: 0,
            painting: false,
            carry: 0.0,
            seed: 0x1234_5679,
        }
    }
}

impl TerrainTool {
    fn random(&mut self) -> f32 {
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }
}

/// O terreno guardado num componente `terrain`.
pub fn terrain_of(c: &Component) -> TerrainData {
    TerrainData::decode(
        c.str("data"),
        c.num("cells").clamp(2.0, 256.0) as u32,
        Vec2::new(c.num("width") as f32, c.num("depth") as f32),
        c.num("height") as f32,
    )
}

/// As cores das quatro camadas do componente.
pub fn colors_of(c: &Component) -> [Color; 4] {
    ["color1", "color2", "color3", "color4"].map(|k| parse_color(c.str(k)))
}

/// O nome que o objeto espalhado ganha (o do arquivo).
fn stem(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "objeto".into())
}

/// Um passo do pincel em `hit` (no mundo), com o terreno em `origin`.
/// Devolve o terreno novo (pincéis de altura e cor) ou os objetos para pôr
/// e os ids para tirar (Espalhar).
#[derive(Debug)]
pub enum BrushResult {
    Nothing,
    Terrain(TerrainData),
    Scatter { add: Vec<Object>, remove: Vec<u32> },
}

/// Os objetos já espalhados (com o mesmo arquivo), para não amontoar e para
/// o Ctrl apagar: (id, posição no mundo).
pub type Placed = Vec<(u32, Vec3)>;

#[allow(clippy::too_many_arguments)]
pub fn apply(
    tool: &mut TerrainTool,
    terrain: &TerrainData,
    origin: Vec3,
    hit: Vec3,
    dt: f32,
    invert: bool,
    placed: &Placed,
) -> BrushResult {
    let local = hit - origin;
    let center = Vec2::new(local.x, local.z);
    let mut t = terrain.clone();
    let per_frame = tool.strength * dt;
    let changed = match tool.brush {
        TerrainBrush::Raise | TerrainBrush::Lower => {
            let up = (tool.brush == TerrainBrush::Raise) != invert;
            let op = if up { BrushOp::Raise } else { BrushOp::Lower };
            t.brush(op, center, tool.radius, per_frame)
        }
        TerrainBrush::Smooth => t.brush(
            BrushOp::Smooth,
            center,
            tool.radius,
            (per_frame * 2.0).min(1.0),
        ),
        TerrainBrush::Flatten => {
            let target = *tool
                .flatten_to
                .get_or_insert_with(|| terrain.height_at(center.x, center.y).unwrap_or(0.0));
            t.brush(
                BrushOp::Flatten(target),
                center,
                tool.radius,
                (per_frame * 2.0).min(1.0),
            )
        }
        TerrainBrush::Noise => t.brush(BrushOp::Noise, center, tool.radius, per_frame * 0.5),
        TerrainBrush::Paint => t.paint(tool.layer, center, tool.radius, (per_frame * 2.0).min(1.0)),
        TerrainBrush::Scatter => {
            if invert {
                let remove: Vec<u32> = placed
                    .iter()
                    .filter(|(_, p)| Vec2::new(p.x - hit.x, p.z - hit.z).length() <= tool.radius)
                    .map(|(id, _)| *id)
                    .collect();
                return BrushResult::Scatter {
                    add: Vec::new(),
                    remove,
                };
            }
            if tool.scatter.is_empty() {
                return BrushResult::Nothing;
            }
            tool.carry += dt * tool.density;
            let mut add = Vec::new();
            let mut taken: Vec<Vec3> = placed.iter().map(|(_, p)| *p).collect();
            let gap = (tool.radius * 0.35).clamp(0.8, 4.0);
            while tool.carry >= 1.0 {
                tool.carry -= 1.0;
                let a = tool.random() * std::f32::consts::TAU;
                let r = tool.radius * tool.random().sqrt();
                let p = Vec2::new(center.x + a.cos() * r, center.y + a.sin() * r);
                let Some(h) = terrain.height_at(p.x, p.y) else {
                    continue;
                };
                let world = origin + Vec3::new(p.x, h, p.y);
                if taken
                    .iter()
                    .any(|q| Vec2::new(q.x - world.x, q.z - world.z).length() < gap)
                {
                    continue;
                }
                taken.push(world);
                let scale = 0.8 + tool.random() * 0.45;
                let mut o = Object::new(0, &stem(&tool.scatter));
                o.x = world.x as f64;
                o.y = world.y as f64;
                o.z = world.z as f64;
                o.ry = (tool.random() * 360.0).round() as f64;
                o.scale_x = scale as f64;
                o.scale_y = scale as f64;
                o.scale_z = scale as f64;
                if tool.scatter.ends_with(".prefab") {
                    o.prefab = Some(crate::scene_data::PrefabLink {
                        path: tool.scatter.clone(),
                        base: None,
                        raw: Vec::new(),
                    });
                } else {
                    let mut m = Component::new("model");
                    m.set("file", Prop::Str(tool.scatter.clone()));
                    o.components.push(m);
                    o.components.push(Component::new("body3d"));
                }
                add.push(o);
            }
            return BrushResult::Scatter {
                add,
                remove: Vec::new(),
            };
        }
    };
    if changed {
        BrushResult::Terrain(t)
    } else {
        BrushResult::Nothing
    }
}

/// Os pontos do círculo do pincel no chão (no mundo), para desenhar.
pub fn ring(terrain: &TerrainData, origin: Vec3, hit: Vec3, radius: f32) -> Vec<Vec3> {
    let local = hit - origin;
    (0..=40)
        .map(|k| {
            let a = k as f32 / 40.0 * std::f32::consts::TAU;
            let x = local.x + a.cos() * radius;
            let z = local.z + a.sin() * radius;
            let y = terrain.height_at(x, z).unwrap_or(local.y);
            origin + Vec3::new(x, y + 0.05, z)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comp() -> Component {
        let mut c = Component::new("terrain");
        c.set("cells", Prop::Num(16.0));
        c.set("width", Prop::Num(32.0));
        c.set("depth", Prop::Num(32.0));
        c.set("height", Prop::Num(10.0));
        c
    }

    #[test]
    fn pinceis_e_espalhar() {
        let c = comp();
        let t = terrain_of(&c);
        assert_eq!(t.cells, 16);
        let mut tool = TerrainTool::default();
        let origin = Vec3::new(100.0, 2.0, 0.0);
        // Subir em (100, 0): o morro fica no meio do terreno.
        let BrushResult::Terrain(up) = apply(
            &mut tool,
            &t,
            origin,
            Vec3::new(100.0, 2.0, 0.0),
            0.5,
            false,
            &Vec::new(),
        ) else {
            panic!("subiu");
        };
        assert!(up.height_at(0.0, 0.0).unwrap() > 1.0);
        // Ctrl: desce.
        let BrushResult::Terrain(down) =
            apply(&mut tool, &t, origin, origin, 0.5, true, &Vec::new())
        else {
            panic!("desceu");
        };
        assert!(down.height_at(0.0, 0.0).unwrap() < -1.0);
        // Espalhar: põe alguns, no chão, sem amontoar; Ctrl tira.
        tool.brush = TerrainBrush::Scatter;
        tool.scatter = "modelos/arvore.glb".into();
        tool.radius = 10.0;
        let BrushResult::Scatter { add, .. } =
            apply(&mut tool, &t, origin, origin, 2.0, false, &Vec::new())
        else {
            panic!("espalhou");
        };
        assert!(!add.is_empty() && add.len() <= 12, "{}", add.len());
        for o in &add {
            assert_eq!(o.name, "arvore");
            assert!((o.y - 2.0).abs() < 1e-4, "no chão: {}", o.y);
            assert!(o.component("body3d").is_some());
        }
        let placed: Placed = add
            .iter()
            .enumerate()
            .map(|(i, o)| (i as u32, Vec3::new(o.x as f32, o.y as f32, o.z as f32)))
            .collect();
        let BrushResult::Scatter { remove, .. } =
            apply(&mut tool, &t, origin, origin, 0.1, true, &placed)
        else {
            panic!("apagou");
        };
        assert_eq!(remove.len(), placed.len());
        // O círculo segue o chão.
        assert_eq!(ring(&t, origin, origin, 3.0).len(), 41);
    }
}
