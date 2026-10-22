//! Um modelo 3D do modelador (`.modelo`): peças feitas de pontos e faces
//! coloridas, no estilo low-poly (cada face de uma cor, sombreada por face).
//!
//! O `.modelo` é um arquivo Lua legível, como as cenas. Ao salvar, o NN Studio
//! grava junto o `.glb` (glTF), que é o que o jogo usa e que abre no Blender:
//! cada peça vira um nó, e cada cor, um material.

use std::collections::HashMap;

use glam::{EulerRot, Mat4, Quat, Vec3};
use mlua::{Lua, Table, Value};

/// As cores que um modelo novo traz na paleta.
pub const DEFAULT_PALETTE: [&str; 16] = [
    "#FFFFFF", "#B8B8B8", "#5C5C5C", "#1E1E1E", "#E84A4A", "#FF8C42", "#FFD166", "#7BC86C",
    "#2E8B57", "#3FA7D6", "#2C5D9E", "#C77DFF", "#9C6B43", "#6B4423", "#F2C9A0", "#C9A85C",
];

/// Uma face: os pontos em volta (no sentido anti-horário, visto de fora) e a cor.
#[derive(Debug, Clone, PartialEq)]
pub struct Face {
    pub v: Vec<u32>,
    pub color: String,
}

/// Uma peça do modelo (a cabeça, o telhado...): pontos e faces, com posição,
/// rotação (graus, como nos objetos 3D) e escala próprias.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub name: String,
    pub verts: Vec<Vec3>,
    pub faces: Vec<Face>,
    pub pos: Vec3,
    pub rot: Vec3,
    pub scale: Vec3,
    /// Espelho em X: só a metade com x ≥ 0 é guardada; a outra é desenhada
    /// (e exportada) espelhada.
    pub mirror: bool,
    /// Escondida só no editor (o `.glb` leva todas).
    pub visible: bool,
}

/// Um modelo inteiro.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ModelData {
    pub parts: Vec<Part>,
    pub palette: Vec<String>,
}

/// Pontos com |x| menor que isso estão no meio (no espelho).
pub const MID: f32 = 1e-4;

/// O vetor normal de uma face (método de Newell: vale para faces com mais de
/// três pontos, mesmo um pouco tortas).
pub fn face_normal(verts: &[Vec3], face: &[u32]) -> Vec3 {
    let mut n = Vec3::ZERO;
    for i in 0..face.len() {
        let a = verts[face[i] as usize];
        let b = verts[face[(i + 1) % face.len()] as usize];
        n.x += (a.y - b.y) * (a.z + b.z);
        n.y += (a.z - b.z) * (a.x + b.x);
        n.z += (a.x - b.x) * (a.y + b.y);
    }
    n.normalize_or_zero()
}

/// O meio de uma face.
pub fn face_center(verts: &[Vec3], face: &[u32]) -> Vec3 {
    face.iter().map(|&i| verts[i as usize]).sum::<Vec3>() / face.len().max(1) as f32
}

impl Part {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            verts: Vec::new(),
            faces: Vec::new(),
            pos: Vec3::ZERO,
            rot: Vec3::ZERO,
            scale: Vec3::ONE,
            mirror: false,
            visible: true,
        }
    }

    /// A rotação da peça.
    pub fn rotation(&self) -> Quat {
        Quat::from_euler(
            EulerRot::YXZ,
            self.rot.y.to_radians(),
            self.rot.x.to_radians(),
            self.rot.z.to_radians(),
        )
    }

    /// A matriz da peça no modelo.
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation(), self.pos)
    }

    pub fn face(&mut self, v: &[u32], color: &str) {
        self.faces.push(Face {
            v: v.to_vec(),
            color: color.into(),
        });
    }

    // Formas prontas (com 1 m de lado, em pé no chão: a base em y = 0) ----

    pub fn cube(name: &str, color: &str) -> Self {
        let mut p = Self::new(name);
        for &(x, y, z) in &[
            (-0.5, 0.0, -0.5),
            (0.5, 0.0, -0.5),
            (0.5, 0.0, 0.5),
            (-0.5, 0.0, 0.5),
            (-0.5, 1.0, -0.5),
            (0.5, 1.0, -0.5),
            (0.5, 1.0, 0.5),
            (-0.5, 1.0, 0.5),
        ] {
            p.verts.push(Vec3::new(x, y, z));
        }
        for f in [
            [0, 1, 2, 3],
            [4, 7, 6, 5],
            [3, 2, 6, 7],
            [1, 0, 4, 5],
            [2, 1, 5, 6],
            [0, 3, 7, 4],
        ] {
            p.face(&f, color);
        }
        p
    }

    /// A rampa (cunha): o lado de trás alto, a frente no chão.
    pub fn ramp(name: &str, color: &str) -> Self {
        let mut p = Self::new(name);
        for &(x, y, z) in &[
            (-0.5, 0.0, -0.5),
            (0.5, 0.0, -0.5),
            (0.5, 0.0, 0.5),
            (-0.5, 0.0, 0.5),
            (-0.5, 1.0, -0.5),
            (0.5, 1.0, -0.5),
        ] {
            p.verts.push(Vec3::new(x, y, z));
        }
        p.face(&[0, 1, 2, 3], color);
        p.face(&[3, 2, 5, 4], color);
        p.face(&[1, 0, 4, 5], color);
        p.face(&[2, 1, 5], color);
        p.face(&[0, 3, 4], color);
        p
    }

    pub fn plane(name: &str, color: &str) -> Self {
        let mut p = Self::new(name);
        for &(x, z) in &[(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)] {
            p.verts.push(Vec3::new(x, 0.0, z));
        }
        p.face(&[3, 2, 1, 0], color);
        p
    }

    /// Cilindro com `n` lados (ou cone, com `top` = 0).
    pub fn ring_solid(name: &str, color: &str, n: u32, top: f32) -> Self {
        let mut p = Self::new(name);
        let n = n.max(3);
        for i in 0..n {
            let a = std::f32::consts::TAU * i as f32 / n as f32;
            p.verts.push(Vec3::new(a.sin() * 0.5, 0.0, a.cos() * 0.5));
        }
        let base: Vec<u32> = (0..n).rev().collect();
        if top > 0.0 {
            for i in 0..n {
                let a = std::f32::consts::TAU * i as f32 / n as f32;
                p.verts
                    .push(Vec3::new(a.sin() * 0.5 * top, 1.0, a.cos() * 0.5 * top));
            }
            p.face(&base, color);
            p.face(&(n..2 * n).collect::<Vec<_>>(), color);
            for i in 0..n {
                let j = (i + 1) % n;
                p.face(&[i, j, n + j, n + i], color);
            }
        } else {
            p.verts.push(Vec3::new(0.0, 1.0, 0.0));
            p.face(&base, color);
            for i in 0..n {
                let j = (i + 1) % n;
                p.face(&[i, j, n], color);
            }
        }
        p
    }

    pub fn cylinder(name: &str, color: &str) -> Self {
        Self::ring_solid(name, color, 8, 1.0)
    }

    pub fn cone(name: &str, color: &str) -> Self {
        Self::ring_solid(name, color, 8, 0.0)
    }

    /// Pirâmide de base quadrada (do tamanho do cubo): um telhado.
    pub fn pyramid(name: &str, color: &str) -> Self {
        let mut p = Self::new(name);
        for &(x, z) in &[(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)] {
            p.verts.push(Vec3::new(x, 0.0, z));
        }
        p.verts.push(Vec3::new(0.0, 1.0, 0.0));
        p.face(&[0, 1, 2, 3], color);
        for i in 0..4 {
            p.face(&[(i + 1) % 4, i, 4], color);
        }
        p
    }

    /// Esfera low-poly: 8 fatias e 5 anéis.
    pub fn sphere(name: &str, color: &str) -> Self {
        Self::sphere_n(name, color, 8, 5)
    }

    /// Esfera com `seg` fatias e `rings` anéis (1 m, em pé no chão).
    pub fn sphere_n(name: &str, color: &str, seg: u32, rings: u32) -> Self {
        let mut p = Self::new(name);
        let (seg, rings) = (seg.max(3), rings.max(3));
        p.verts.push(Vec3::new(0.0, 0.0, 0.0));
        for r in 1..rings {
            let t = std::f32::consts::PI * r as f32 / rings as f32;
            let (y, rad) = (0.5 - 0.5 * t.cos(), 0.5 * t.sin());
            for s in 0..seg {
                let a = std::f32::consts::TAU * s as f32 / seg as f32;
                p.verts.push(Vec3::new(a.sin() * rad, y, a.cos() * rad));
            }
        }
        let top = p.verts.len() as u32;
        p.verts.push(Vec3::new(0.0, 1.0, 0.0));
        let at = |r: u32, s: u32| 1 + (r - 1) * seg + s % seg;
        for s in 0..seg {
            p.face(&[0, at(1, s + 1), at(1, s)], color);
        }
        for r in 1..rings - 1 {
            for s in 0..seg {
                p.face(
                    &[at(r, s), at(r, s + 1), at(r + 1, s + 1), at(r + 1, s)],
                    color,
                );
            }
        }
        for s in 0..seg {
            p.face(&[at(rings - 1, s), at(rings - 1, s + 1), top], color);
        }
        p
    }

    /// Os pontos passam pela matriz (a forma muda; a posição, a rotação e a
    /// escala da peça, não).
    pub fn baked(mut self, m: Mat4) -> Self {
        for v in &mut self.verts {
            *v = m.transform_point3(*v);
        }
        // Uma matriz que espelha vira as faces do avesso: desvira.
        if m.determinant() < 0.0 {
            for f in &mut self.faces {
                f.v.reverse();
            }
        }
        self
    }

    /// Junta os pontos e as faces de outra peça (no mesmo espaço) nesta.
    pub fn merge(&mut self, other: &Part) {
        let base = self.verts.len() as u32;
        self.verts.extend_from_slice(&other.verts);
        for f in &other.faces {
            self.faces.push(Face {
                v: f.v.iter().map(|k| k + base).collect(),
                color: f.color.clone(),
            });
        }
    }

    /// A face mais virada para `dir` (ex.: a de cima, com `Vec3::Y`).
    pub fn face_towards(&self, dir: Vec3) -> Option<usize> {
        (0..self.faces.len()).max_by(|&a, &b| {
            let na = face_normal(&self.verts, &self.faces[a].v).dot(dir);
            let nb = face_normal(&self.verts, &self.faces[b].v).dot(dir);
            na.total_cmp(&nb)
        })
    }

    /// As faces com os pontos já espelhados (as de verdade e, com espelho, as
    /// do outro lado): (pontos, cor, é do espelho, índice da face).
    pub fn all_faces(&self) -> Vec<(Vec<Vec3>, &str, bool, usize)> {
        let mut out = Vec::new();
        for (i, f) in self.faces.iter().enumerate() {
            let pts: Vec<Vec3> = f.v.iter().map(|&k| self.verts[k as usize]).collect();
            if self.mirror {
                let mut m: Vec<Vec3> = pts.iter().map(|p| Vec3::new(-p.x, p.y, p.z)).collect();
                m.reverse();
                out.push((pts, f.color.as_str(), false, i));
                out.push((m, f.color.as_str(), true, i));
            } else {
                out.push((pts, f.color.as_str(), false, i));
            }
        }
        out
    }

    /// A malha da peça (no espaço dela), cada face com a cor e a normal dela.
    pub fn mesh(&self) -> nn_render::d3::Mesh {
        let mut mesh = nn_render::d3::Mesh::new();
        for (pts, color, _, _) in self.all_faces() {
            let c = crate::scene_render::parse_color(color);
            for k in 1..pts.len().saturating_sub(1) {
                mesh.flat_triangle(pts[0], pts[k], pts[k + 1], c);
            }
        }
        mesh
    }

    // Edição -----------------------------------------------------------------

    /// Puxa as faces para fora (como um bloco): os pontos delas viram novos,
    /// ligados aos antigos por paredes nas bordas do grupo. Devolve os pontos
    /// novos e a direção média (para andar com o mouse).
    pub fn extrude(&mut self, faces: &[usize]) -> (Vec<u32>, Vec3) {
        let sel: Vec<usize> = faces
            .iter()
            .copied()
            .filter(|&f| f < self.faces.len())
            .collect();
        if sel.is_empty() {
            return (Vec::new(), Vec3::Y);
        }
        // Quantas faces do grupo usam cada aresta (a, b) num sentido ou no outro.
        let mut edges: HashMap<(u32, u32), usize> = HashMap::new();
        for &f in &sel {
            let v = &self.faces[f].v;
            for i in 0..v.len() {
                let (a, b) = (v[i], v[(i + 1) % v.len()]);
                *edges.entry((a.min(b), a.max(b))).or_default() += 1;
            }
        }
        let mut normal = Vec3::ZERO;
        let mut map: HashMap<u32, u32> = HashMap::new();
        let mut walls: Vec<(u32, u32, String)> = Vec::new();
        for &f in &sel {
            normal += face_normal(&self.verts, &self.faces[f].v);
            let v = self.faces[f].v.clone();
            for i in 0..v.len() {
                let (a, b) = (v[i], v[(i + 1) % v.len()]);
                // Com o espelho, a borda no meio não ganha parede: a outra
                // metade continua dali.
                let mid = self.mirror
                    && self.verts[a as usize].x.abs() <= MID
                    && self.verts[b as usize].x.abs() <= MID;
                if edges[&(a.min(b), a.max(b))] == 1 && !mid {
                    walls.push((a, b, self.faces[f].color.clone()));
                }
            }
            for &k in &v {
                map.entry(k).or_insert_with(|| {
                    self.verts.push(self.verts[k as usize]);
                    self.verts.len() as u32 - 1
                });
            }
        }
        for &f in &sel {
            for k in &mut self.faces[f].v {
                *k = map[k];
            }
        }
        for (a, b, color) in walls {
            self.faces.push(Face {
                v: vec![a, b, map[&b], map[&a]],
                color,
            });
        }
        let mut moved: Vec<u32> = map.values().copied().collect();
        moved.sort_unstable();
        (moved, normal.normalize_or(Vec3::Y))
    }

    /// Encolhe cada face para dentro (`t` de 0 a 1 do caminho até o meio): a
    /// face fica no meio, ligada à borda por um anel de faces. Devolve as
    /// faces do meio.
    pub fn inset(&mut self, faces: &[usize], t: f32) -> Vec<usize> {
        let t = t.clamp(0.0, 0.95);
        let mut inner = Vec::new();
        for &f in faces {
            let Some(face) = self.faces.get(f).cloned() else {
                continue;
            };
            let c = face_center(&self.verts, &face.v);
            let new: Vec<u32> = face
                .v
                .iter()
                .map(|&k| {
                    let p = self.verts[k as usize];
                    self.verts.push(p + (c - p) * t);
                    self.verts.len() as u32 - 1
                })
                .collect();
            let n = face.v.len();
            for i in 0..n {
                let j = (i + 1) % n;
                self.faces.push(Face {
                    v: vec![face.v[i], face.v[j], new[j], new[i]],
                    color: face.color.clone(),
                });
            }
            self.faces[f].v = new;
            inner.push(f);
        }
        inner
    }

    /// Tira as faces e os pontos que ficarem sem uso.
    pub fn delete_faces(&mut self, faces: &[usize]) {
        let mut i = 0;
        self.faces.retain(|_| {
            let keep = !faces.contains(&i);
            i += 1;
            keep
        });
        self.cleanup();
    }

    /// Junta os pontos num só (no meio deles). Faces que ficam sem área saem.
    pub fn merge_verts(&mut self, verts: &[u32]) {
        if verts.len() < 2 {
            return;
        }
        let c = verts.iter().map(|&k| self.verts[k as usize]).sum::<Vec3>() / verts.len() as f32;
        let keep = verts[0];
        self.verts[keep as usize] = c;
        for f in &mut self.faces {
            for k in &mut f.v {
                if verts.contains(k) {
                    *k = keep;
                }
            }
            f.v.dedup();
            while f.v.len() > 1 && f.v.first() == f.v.last() {
                f.v.pop();
            }
        }
        self.faces.retain(|f| f.v.len() >= 3);
        self.cleanup();
    }

    /// Tira os pontos sem face e renumera.
    pub fn cleanup(&mut self) {
        let mut used = vec![false; self.verts.len()];
        for f in &self.faces {
            for &k in &f.v {
                used[k as usize] = true;
            }
        }
        let mut map = vec![u32::MAX; self.verts.len()];
        let mut verts = Vec::new();
        for (i, p) in self.verts.iter().enumerate() {
            if used[i] {
                map[i] = verts.len() as u32;
                verts.push(*p);
            }
        }
        self.verts = verts;
        for f in &mut self.faces {
            for k in &mut f.v {
                *k = map[*k as usize];
            }
        }
    }

    /// Liga o espelho em X: corta a peça em x = 0 e fica só com a metade
    /// x ≥ 0 (a outra passa a ser o espelho dela).
    pub fn mirror_on(&mut self) {
        if self.mirror {
            return;
        }
        let mut cut: HashMap<(u32, u32), u32> = HashMap::new();
        let mut faces = Vec::new();
        let old = std::mem::take(&mut self.faces);
        for f in old {
            let n = f.v.len();
            let mut out: Vec<u32> = Vec::new();
            for i in 0..n {
                let (a, b) = (f.v[i], f.v[(i + 1) % n]);
                let (pa, pb) = (self.verts[a as usize], self.verts[b as usize]);
                let (ina, inb) = (pa.x >= -MID, pb.x >= -MID);
                if ina {
                    out.push(a);
                }
                if ina != inb && (pa.x.abs() > MID && pb.x.abs() > MID) {
                    let key = (a.min(b), a.max(b));
                    let k = *cut.entry(key).or_insert_with(|| {
                        let t = pa.x / (pa.x - pb.x);
                        let mut p = pa + (pb - pa) * t;
                        p.x = 0.0;
                        self.verts.push(p);
                        self.verts.len() as u32 - 1
                    });
                    out.push(k);
                }
            }
            out.dedup();
            if out.len() >= 3 {
                // Uma face toda em x = 0 (a parede do corte) não é desenhada:
                // as duas metades se encontram ali.
                let flat = out.iter().all(|&k| self.verts[k as usize].x.abs() <= MID);
                if !flat {
                    faces.push(Face {
                        v: out,
                        color: f.color,
                    });
                }
            }
        }
        self.faces = faces;
        for p in &mut self.verts {
            if p.x.abs() <= MID {
                p.x = 0.0;
            }
        }
        self.cleanup();
        self.mirror = true;
    }

    /// Desliga o espelho: a outra metade vira pontos e faces de verdade.
    pub fn mirror_apply(&mut self) {
        if !self.mirror {
            return;
        }
        let mut map: HashMap<u32, u32> = HashMap::new();
        let n = self.verts.len() as u32;
        for k in 0..n {
            let p = self.verts[k as usize];
            if p.x.abs() <= MID {
                map.insert(k, k);
            } else {
                self.verts.push(Vec3::new(-p.x, p.y, p.z));
                map.insert(k, self.verts.len() as u32 - 1);
            }
        }
        let copies: Vec<Face> = self
            .faces
            .iter()
            .map(|f| {
                let mut v: Vec<u32> = f.v.iter().map(|k| map[k]).collect();
                v.reverse();
                Face {
                    v,
                    color: f.color.clone(),
                }
            })
            .collect();
        self.faces.extend(copies);
        self.mirror = false;
    }
}

/// Os graus (x, y, z) de uma rotação, na ordem dos objetos 3D (Y, X, Z).
pub fn euler_degrees(q: Quat) -> Vec3 {
    let (y, x, z) = q.to_euler(EulerRot::YXZ);
    let r = |v: f32| {
        let d = v.to_degrees();
        let r = (d * 1000.0).round() / 1000.0;
        if r == -0.0 { 0.0 } else { r }
    };
    Vec3::new(r(x), r(y), r(z))
}

/// Formas prontas para começar uma peça: (rótulo, palavras para a busca, como faz).
pub type Maker = fn(&str, &str) -> Part;
pub const SHAPES: [(&str, &str, Maker); 7] = [
    ("Cubo", "caixa bloco quadrado", Part::cube),
    ("Cilindro", "tubo coluna tronco roda", Part::cylinder),
    ("Cone", "ponta chapeu", Part::cone),
    ("Pirâmide", "telhado ponta", Part::pyramid),
    ("Esfera", "bola cabeca", Part::sphere),
    ("Plano", "chao piso placa", Part::plane),
    ("Rampa", "cunha telhado inclinado", Part::ramp),
];

impl ModelData {
    /// Um modelo novo: um cubo.
    pub fn new_default() -> Self {
        Self {
            parts: vec![Part::cube("Cubo", "#FF8C42")],
            palette: DEFAULT_PALETTE.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// A caixa que envolve o modelo (com as peças no lugar).
    pub fn bounds(&self) -> Option<(Vec3, Vec3)> {
        let mut out: Option<(Vec3, Vec3)> = None;
        for p in &self.parts {
            let m = p.matrix();
            for (pts, ..) in p.all_faces() {
                for q in pts {
                    let w = m.transform_point3(q);
                    out = Some(match out {
                        Some((a, b)) => (a.min(w), b.max(w)),
                        None => (w, w),
                    });
                }
            }
        }
        out
    }

    pub fn triangle_count(&self) -> usize {
        self.parts
            .iter()
            .flat_map(|p| p.all_faces())
            .map(|(pts, ..)| pts.len().saturating_sub(2))
            .sum()
    }

    // Arquivo .modelo --------------------------------------------------------

    pub fn to_lua(&self) -> String {
        let n =
            |v: f32| crate::scene_data::format_num(f64::from((v * 10_000.0).round() / 10_000.0));
        let v3 = |v: Vec3| format!("{{ {}, {}, {} }}", n(v.x), n(v.y), n(v.z));
        let mut s = String::from(
            "-- Modelo 3D do NN Studio (abra no NN Studio para editar). Ao salvar, o\n\
             -- NN Studio grava junto o .glb com o mesmo nome, que é o que o jogo usa.\n\
             return {\n  format = 1,\n",
        );
        s.push_str("  palette = { ");
        for c in &self.palette {
            s.push_str(&format!("\"{c}\", "));
        }
        s.push_str("},\n  parts = {\n");
        for p in &self.parts {
            s.push_str(&format!(
                "    {{ name = {}, pos = {}, rot = {}, scale = {},\n",
                crate::config_lua::lua_string(&p.name),
                v3(p.pos),
                v3(p.rot),
                v3(p.scale)
            ));
            if p.mirror {
                s.push_str("      mirror = true,\n");
            }
            if !p.visible {
                s.push_str("      visible = false,\n");
            }
            s.push_str("      verts = {\n");
            for q in &p.verts {
                s.push_str(&format!("        {},\n", v3(*q)));
            }
            s.push_str("      },\n      faces = {\n");
            for f in &p.faces {
                let idx: Vec<String> = f.v.iter().map(|k| (k + 1).to_string()).collect();
                s.push_str(&format!(
                    "        {{ {}, color = \"{}\" }},\n",
                    idx.join(", "),
                    f.color
                ));
            }
            s.push_str("      },\n    },\n");
        }
        s.push_str("  },\n}\n");
        s
    }

    pub fn from_lua(text: &str, name: &str) -> Result<Self, String> {
        let lua = Lua::new();
        let value: Value = lua
            .load(text)
            .set_name(format!("@{name}"))
            .eval()
            .map_err(|e| nn_script::translate_error(e.to_string().trim()))?;
        let Value::Table(t) = value else {
            return Err(format!("{name}: o modelo deve devolver uma tabela"));
        };
        let num = |v: Value| -> f32 {
            match v {
                Value::Integer(i) => i as f32,
                Value::Number(n) => n as f32,
                _ => 0.0,
            }
        };
        let v3 = |t: &Table, k: &str, d: Vec3| -> Vec3 {
            match t.get::<Value>(k) {
                Ok(Value::Table(v)) => Vec3::new(
                    num(v.get(1).unwrap_or(Value::Nil)),
                    num(v.get(2).unwrap_or(Value::Nil)),
                    num(v.get(3).unwrap_or(Value::Nil)),
                ),
                _ => d,
            }
        };
        let mut palette = Vec::new();
        if let Ok(Value::Table(p)) = t.get::<Value>("palette") {
            for c in p.sequence_values::<String>().flatten() {
                palette.push(c);
            }
        }
        if palette.is_empty() {
            palette = DEFAULT_PALETTE.iter().map(|s| s.to_string()).collect();
        }
        let mut parts = Vec::new();
        if let Ok(Value::Table(list)) = t.get::<Value>("parts") {
            for (i, pt) in list.sequence_values::<Table>().enumerate() {
                let pt = pt.map_err(|e| format!("{name}: peça {}: {e}", i + 1))?;
                let mut p = Part::new(
                    &pt.get::<Option<String>>("name")
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| format!("peça {}", i + 1)),
                );
                p.pos = v3(&pt, "pos", Vec3::ZERO);
                p.rot = v3(&pt, "rot", Vec3::ZERO);
                p.scale = v3(&pt, "scale", Vec3::ONE);
                p.mirror = pt
                    .get::<Option<bool>>("mirror")
                    .ok()
                    .flatten()
                    .unwrap_or(false);
                p.visible = pt
                    .get::<Option<bool>>("visible")
                    .ok()
                    .flatten()
                    .unwrap_or(true);
                if let Ok(Value::Table(vs)) = pt.get::<Value>("verts") {
                    for v in vs.sequence_values::<Table>().flatten() {
                        p.verts.push(Vec3::new(
                            num(v.get(1).unwrap_or(Value::Nil)),
                            num(v.get(2).unwrap_or(Value::Nil)),
                            num(v.get(3).unwrap_or(Value::Nil)),
                        ));
                    }
                }
                if let Ok(Value::Table(fs)) = pt.get::<Value>("faces") {
                    for f in fs.sequence_values::<Table>().flatten() {
                        let color = f
                            .get::<Option<String>>("color")
                            .ok()
                            .flatten()
                            .unwrap_or_else(|| "#FFFFFF".into());
                        let v: Vec<u32> = f
                            .sequence_values::<i64>()
                            .flatten()
                            .filter(|k| *k >= 1 && (*k as usize) <= p.verts.len())
                            .map(|k| (k - 1) as u32)
                            .collect();
                        if v.len() >= 3 {
                            p.faces.push(Face { v, color });
                        }
                    }
                }
                parts.push(p);
            }
        }
        Ok(Self { parts, palette })
    }

    // Arquivo .glb -----------------------------------------------------------

    /// O `.glb` do modelo: um nó por peça (com posição, rotação e escala), um
    /// material por cor e as faces já com o espelho.
    pub fn to_glb(&self) -> Vec<u8> {
        use serde_json::json;
        let mut bin: Vec<u8> = Vec::new();
        let mut views = Vec::new();
        let mut accessors = Vec::new();
        let mut materials: Vec<serde_json::Value> = Vec::new();
        let mut material_of: HashMap<String, usize> = HashMap::new();
        let mut meshes = Vec::new();
        let mut nodes = Vec::new();
        let mut push_view = |bin: &mut Vec<u8>, data: &[u8], target: u32| -> usize {
            while !bin.len().is_multiple_of(4) {
                bin.push(0);
            }
            let offset = bin.len();
            bin.extend_from_slice(data);
            views.push(json!({
                "buffer": 0, "byteOffset": offset, "byteLength": data.len(), "target": target
            }));
            views.len() - 1
        };
        for p in &self.parts {
            // As faces de cada cor juntas (uma "primitiva" por cor).
            let mut by_color: Vec<(String, Vec<Vec3>, Vec<Vec3>)> = Vec::new();
            for (pts, color, ..) in p.all_faces() {
                let n = {
                    let idx: Vec<u32> = (0..pts.len() as u32).collect();
                    face_normal(&pts, &idx)
                };
                let slot = match by_color.iter().position(|(c, ..)| c == color) {
                    Some(i) => i,
                    None => {
                        by_color.push((color.to_string(), Vec::new(), Vec::new()));
                        by_color.len() - 1
                    }
                };
                for k in 1..pts.len().saturating_sub(1) {
                    for q in [pts[0], pts[k], pts[k + 1]] {
                        by_color[slot].1.push(q);
                        by_color[slot].2.push(n);
                    }
                }
            }
            let mut prims = Vec::new();
            for (color, pos, nor) in by_color {
                if pos.is_empty() {
                    continue;
                }
                let mat = *material_of.entry(color.clone()).or_insert_with(|| {
                    let c = crate::scene_render::parse_color(&color);
                    let lin = |v: f32| {
                        if v <= 0.04045 {
                            v / 12.92
                        } else {
                            ((v + 0.055) / 1.055).powf(2.4)
                        }
                    };
                    materials.push(json!({
                        "name": color,
                        "pbrMetallicRoughness": {
                            "baseColorFactor": [lin(c.r), lin(c.g), lin(c.b), 1.0],
                            "metallicFactor": 0.0,
                            "roughnessFactor": 1.0
                        }
                    }));
                    materials.len() - 1
                });
                let (mut lo, mut hi) = (pos[0], pos[0]);
                let mut pbytes = Vec::with_capacity(pos.len() * 12);
                for q in &pos {
                    lo = lo.min(*q);
                    hi = hi.max(*q);
                    for v in [q.x, q.y, q.z] {
                        pbytes.extend_from_slice(&v.to_le_bytes());
                    }
                }
                let mut nbytes = Vec::with_capacity(nor.len() * 12);
                for q in &nor {
                    for v in [q.x, q.y, q.z] {
                        nbytes.extend_from_slice(&v.to_le_bytes());
                    }
                }
                let pv = push_view(&mut bin, &pbytes, 34962);
                let nv = push_view(&mut bin, &nbytes, 34962);
                accessors.push(json!({
                    "bufferView": pv, "componentType": 5126, "count": pos.len(), "type": "VEC3",
                    "min": [lo.x, lo.y, lo.z], "max": [hi.x, hi.y, hi.z]
                }));
                accessors.push(json!({
                    "bufferView": nv, "componentType": 5126, "count": nor.len(), "type": "VEC3"
                }));
                prims.push(json!({
                    "attributes": { "POSITION": accessors.len() - 2, "NORMAL": accessors.len() - 1 },
                    "material": mat
                }));
            }
            let q = p.rotation();
            let mut node = json!({
                "name": p.name,
                "translation": [p.pos.x, p.pos.y, p.pos.z],
                "rotation": [q.x, q.y, q.z, q.w],
                "scale": [p.scale.x, p.scale.y, p.scale.z]
            });
            if !prims.is_empty() {
                meshes.push(json!({ "name": p.name, "primitives": prims }));
                node["mesh"] = json!(meshes.len() - 1);
            }
            nodes.push(node);
        }
        while !bin.len().is_multiple_of(4) {
            bin.push(0);
        }
        let doc = json!({
            "asset": { "version": "2.0", "generator": "NN Studio (modelador)" },
            "scene": 0,
            "scenes": [{ "nodes": (0..nodes.len()).collect::<Vec<_>>() }],
            "nodes": nodes,
            "meshes": meshes,
            "materials": materials,
            "accessors": accessors,
            "bufferViews": views,
            "buffers": [{ "byteLength": bin.len() }]
        });
        let mut text = serde_json::to_vec(&doc).expect("json");
        while !text.len().is_multiple_of(4) {
            text.push(b' ');
        }
        let total = 12 + 8 + text.len() + 8 + bin.len();
        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(b"glTF");
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&(total as u32).to_le_bytes());
        out.extend_from_slice(&(text.len() as u32).to_le_bytes());
        out.extend_from_slice(b"JSON");
        out.extend_from_slice(&text);
        out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        out.extend_from_slice(b"BIN\0");
        out.extend_from_slice(&bin);
        out
    }
}

/// O `.glb` que acompanha um `.modelo` (o mesmo nome).
pub fn glb_path(rel: &str) -> String {
    match rel.strip_suffix(".modelo") {
        Some(base) => format!("{base}.glb"),
        None => format!("{rel}.glb"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn closed(p: &Part) -> bool {
        // Toda aresta usada por duas faces, uma em cada sentido.
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
    fn formas_fechadas_e_viradas_para_fora() {
        for (label, _, make) in SHAPES {
            let p = make(label, "#FF0000");
            if label != "Plano" {
                assert!(closed(&p), "{label} fechado");
            }
            // As normais apontam para fora do meio da forma.
            let c = p.verts.iter().sum::<Vec3>() / p.verts.len() as f32;
            for f in &p.faces {
                let n = face_normal(&p.verts, &f.v);
                let out = face_center(&p.verts, &f.v) - c;
                assert!(n.dot(out) > -1e-4, "{label}: face virada para dentro");
            }
        }
    }

    #[test]
    fn puxar_uma_face_faz_um_bloco() {
        let mut p = Part::cube("c", "#FF0000");
        let top = 1; // a face de cima
        let (moved, dir) = p.extrude(&[top]);
        assert_eq!(moved.len(), 4);
        assert!((dir - Vec3::Y).length() < 1e-5);
        for k in &moved {
            p.verts[*k as usize].y += 1.0;
        }
        assert_eq!(p.faces.len(), 6 + 4);
        assert!(closed(&p), "continua fechado");
        let (_, hi) = ModelData {
            parts: vec![p],
            palette: vec![],
        }
        .bounds()
        .unwrap();
        assert!((hi.y - 2.0).abs() < 1e-5);
    }

    #[test]
    fn puxar_duas_faces_vizinhas_juntas() {
        let mut p = Part::cube("c", "#FF0000");
        p.extrude(&[1, 2]);
        // Cada face puxada ganha paredes só nas bordas do grupo: 3 + 3 (a
        // aresta comum não ganha parede).
        assert_eq!(p.faces.len(), 6 + 6);
        assert!(closed(&p));
    }

    #[test]
    fn encolher_e_apagar() {
        let mut p = Part::cube("c", "#FF0000");
        let inner = p.inset(&[1], 0.3);
        assert_eq!(inner, vec![1]);
        assert_eq!(p.faces.len(), 6 + 4);
        assert!(closed(&p));
        p.delete_faces(&[1]);
        assert_eq!(p.faces.len(), 9);
        assert_eq!(p.verts.len(), 12);
    }

    #[test]
    fn juntar_pontos() {
        let mut p = Part::cube("c", "#FF0000");
        // Os quatro de cima num só: vira uma pirâmide.
        p.merge_verts(&[4, 5, 6, 7]);
        assert_eq!(p.verts.len(), 5);
        assert_eq!(p.faces.len(), 5);
        assert!(closed(&p));
    }

    #[test]
    fn espelho_corta_e_aplica() {
        let mut p = Part::cube("c", "#FF0000");
        p.mirror_on();
        assert!(p.verts.iter().all(|v| v.x >= 0.0));
        // A metade: cima, baixo, frente, trás cortados ao meio, e o lado +x.
        assert_eq!(p.faces.len(), 5);
        let full = Part::cube("c", "#FF0000").mesh().triangle_count();
        // Com o espelho, a malha tem os dois lados (os cortes viram 2 quadrados).
        assert_eq!(p.mesh().triangle_count(), full + 8);
        p.mirror_apply();
        assert!(!p.mirror);
        assert_eq!(p.faces.len(), 10);
        assert!(closed(&p), "as metades se encontram no meio");
    }

    #[test]
    fn puxar_com_espelho_continua_fechado() {
        let mut p = Part::cube("c", "#FF0000");
        p.mirror_on();
        let top = p
            .faces
            .iter()
            .position(|f| face_normal(&p.verts, &f.v).y > 0.9)
            .unwrap();
        let (moved, _) = p.extrude(&[top]);
        for k in moved {
            p.verts[k as usize].y += 0.5;
        }
        p.mirror_apply();
        // Os pontos do meio são dos dois lados: as metades se juntam.
        assert!(closed(&p), "fechado depois de aplicar o espelho");
    }

    #[test]
    fn graus_da_rotacao() {
        let p = Part {
            rot: Vec3::new(10.0, 45.0, -30.0),
            ..Part::cube("c", "#FFFFFF")
        };
        let back = euler_degrees(p.rotation());
        assert!((back - p.rot).length() < 1e-3, "{back}");
    }

    #[test]
    fn arquivo_modelo_ida_e_volta() {
        let mut m = ModelData::new_default();
        m.parts.push(Part::cylinder("Tronco", "#9C6B43"));
        m.parts[1].pos = Vec3::new(1.0, 2.0, -0.5);
        m.parts[1].rot = Vec3::new(0.0, 45.0, 0.0);
        m.parts[1].mirror = false;
        m.parts[0].faces[2].color = "#3FA7D6".into();
        let text = m.to_lua();
        let back = ModelData::from_lua(&text, "teste.modelo").unwrap();
        assert_eq!(back.parts.len(), 2);
        assert_eq!(back.parts[1].name, "Tronco");
        assert_eq!(back.parts[1].pos, Vec3::new(1.0, 2.0, -0.5));
        assert_eq!(back.parts[0].faces[2].color, "#3FA7D6");
        assert_eq!(back.parts[0].verts, m.parts[0].verts);
        assert_eq!(back.palette.len(), DEFAULT_PALETTE.len());
        assert!(ModelData::from_lua("return 3", "x").is_err());
    }

    #[test]
    fn glb_abre_na_engine_com_as_cores() {
        let mut m = ModelData::new_default();
        m.parts[0].faces[1].color = "#3FA7D6".into();
        let mut t = Part::cone("Telhado", "#E84A4A");
        t.pos.y = 1.0;
        t.mirror_on();
        m.parts.push(t);
        let bytes = m.to_glb();
        let data = nn_render::d3::ModelData::from_gltf(&bytes, &mut |_| None).unwrap();
        assert_eq!(data.triangle_count(), m.triangle_count());
        let (lo, hi) = data.bounds();
        let (elo, ehi) = m.bounds().unwrap();
        assert!(
            (lo - elo).length() < 1e-4 && (hi - ehi).length() < 1e-4,
            "{lo} {hi}"
        );
        // As cores voltam em sRGB, como estão no modelo.
        let azul = crate::scene_render::parse_color("#3FA7D6");
        let achou = data.parts.iter().any(|p| {
            p.mesh
                .vertices
                .iter()
                .any(|v| (v.color[2] - azul.b).abs() < 0.01 && (v.color[0] - azul.r).abs() < 0.01)
        });
        assert!(achou, "a face azul chegou azul");
    }

    #[test]
    fn caminho_do_glb() {
        assert_eq!(glb_path("modelos/casa.modelo"), "modelos/casa.glb");
    }
}
