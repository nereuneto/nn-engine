//! Terreno: uma grade de alturas (morros, vales) com quatro camadas de cor
//! pintadas por cima (grama, terra, pedra, areia, ou as cores que quiser).
//!
//! [`TerrainData`] guarda as alturas e as cores e sabe se pintar: os pincéis
//! ([`TerrainData::brush`]) sobem, descem, alisam, aplainam e fazem ruído; o
//! [`TerrainData::paint`] pinta uma camada. [`TerrainData::mesh`] monta a
//! malha (low-poly: cada triângulo com a sua cor) e
//! [`TerrainData::collision`] os triângulos da física, os mesmos do desenho.
//!
//! O terreno fica centrado na origem: vai de `-width/2` a `width/2` em X e de
//! `-depth/2` a `depth/2` em Z. As alturas vão de `-max_height` a
//! `max_height`, e um terreno novo é plano na altura 0.
//!
//! [`TerrainData::encode`] e [`TerrainData::decode`] guardam tudo num texto
//! curto (para a cena do NN Studio).

use glam::{Vec2, Vec3};
use nn_core::Color;

use super::{CollisionMesh, Mesh, Vertex3};

/// O que um pincel faz nas alturas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BrushOp {
    /// Sobe (com força negativa, desce).
    Raise,
    /// Desce.
    Lower,
    /// Puxa cada ponto para a média dos vizinhos.
    Smooth,
    /// Puxa para uma altura.
    Flatten(f32),
    /// Calombos sorteados (sempre os mesmos no mesmo lugar).
    Noise,
}

/// Uma grade de alturas com quatro camadas de cor.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainData {
    /// Quadrados de cada lado (a grade tem `cells + 1` pontos por lado).
    pub cells: u32,
    /// Largura (X) e fundo (Z), em metros.
    pub size: Vec2,
    /// A altura vai de `-max_height` a `max_height`.
    pub max_height: f32,
    /// Altura de cada ponto (linha por linha, de -Z para +Z; em cada linha,
    /// de -X para +X).
    pub heights: Vec<f32>,
    /// Quanto de cada camada em cada ponto (0 a 255; a soma não precisa dar
    /// 255, as cores são misturadas na proporção).
    pub layers: Vec<[u8; 4]>,
}

/// As cores padrão das quatro camadas: grama, terra, pedra e areia.
pub const DEFAULT_LAYER_COLORS: [Color; 4] = [
    Color::rgba(0.48, 0.78, 0.42, 1.0),
    Color::rgba(0.61, 0.44, 0.29, 1.0),
    Color::rgba(0.62, 0.64, 0.66, 1.0),
    Color::rgba(0.95, 0.83, 0.61, 1.0),
];

impl TerrainData {
    /// Um terreno plano, todo da primeira camada.
    pub fn flat(cells: u32, size: Vec2, max_height: f32) -> Self {
        let cells = cells.clamp(2, 512);
        let n = ((cells + 1) * (cells + 1)) as usize;
        Self {
            cells,
            size: size.max(Vec2::splat(0.1)),
            max_height: max_height.max(0.1),
            heights: vec![0.0; n],
            layers: vec![[255, 0, 0, 0]; n],
        }
    }

    /// Pontos por lado.
    pub fn side(&self) -> u32 {
        self.cells + 1
    }

    fn index(&self, i: u32, j: u32) -> usize {
        (j * self.side() + i) as usize
    }

    /// Distância entre dois pontos da grade (X, Z).
    pub fn spacing(&self) -> Vec2 {
        self.size / self.cells as f32
    }

    /// Onde fica o ponto (i, j) no terreno (com a altura).
    pub fn point(&self, i: u32, j: u32) -> Vec3 {
        let s = self.spacing();
        Vec3::new(
            -self.size.x / 2.0 + i as f32 * s.x,
            self.heights[self.index(i, j)],
            -self.size.y / 2.0 + j as f32 * s.y,
        )
    }

    /// A altura do chão em (x, z), no espaço do terreno (entre os pontos, a
    /// altura dos triângulos). `None` fora do terreno.
    pub fn height_at(&self, x: f32, z: f32) -> Option<f32> {
        let s = self.spacing();
        let fx = (x + self.size.x / 2.0) / s.x;
        let fz = (z + self.size.y / 2.0) / s.y;
        let c = self.cells as f32;
        if !(0.0..=c).contains(&fx) || !(0.0..=c).contains(&fz) {
            return None;
        }
        let i = (fx.floor() as u32).min(self.cells - 1);
        let j = (fz.floor() as u32).min(self.cells - 1);
        let (u, v) = (fx - i as f32, fz - j as f32);
        let h = |a: u32, b: u32| self.heights[self.index(a, b)];
        let (h00, h10, h01, h11) = (h(i, j), h(i + 1, j), h(i, j + 1), h(i + 1, j + 1));
        // Os dois triângulos de cada quadrado: a diagonal vai de (0,0) a (1,1).
        Some(if u >= v {
            h00 + (h10 - h00) * u + (h11 - h10) * v
        } else {
            h00 + (h11 - h01) * u + (h01 - h00) * v
        })
    }

    /// Onde um raio (no espaço do terreno) bate no chão. Anda em passos
    /// pequenos e refina no fim.
    pub fn raycast(&self, origin: Vec3, dir: Vec3, max: f32) -> Option<Vec3> {
        let dir = dir.normalize_or_zero();
        if dir == Vec3::ZERO {
            return None;
        }
        let step = self.spacing().min_element().max(0.05) * 0.5;
        let above = |p: Vec3| self.height_at(p.x, p.z).map(|h| p.y - h);
        let mut t = 0.0;
        let mut prev: Option<(f32, f32)> = above(origin).map(|d| (0.0, d));
        while t < max {
            t += step;
            let p = origin + dir * t;
            match above(p) {
                Some(d) => {
                    if let Some((t0, d0)) = prev
                        && d0 > 0.0
                        && d <= 0.0
                    {
                        // Entre t0 e t: busca binária.
                        let (mut a, mut b) = (t0, t);
                        for _ in 0..20 {
                            let m = (a + b) / 2.0;
                            if above(origin + dir * m).unwrap_or(1.0) > 0.0 {
                                a = m;
                            } else {
                                b = m;
                            }
                        }
                        return Some(origin + dir * b);
                    }
                    prev = Some((t, d));
                }
                None => prev = None,
            }
        }
        None
    }

    /// Quanto o pincel pega no ponto (0 a 1): cheio no meio, suave na borda.
    fn falloff(d: f32, radius: f32) -> f32 {
        if d >= radius {
            return 0.0;
        }
        let t = 1.0 - d / radius;
        t * t * (3.0 - 2.0 * t)
    }

    /// Os pontos da grade dentro de um círculo (x, z, raio), com o peso de
    /// cada um.
    fn under(&self, center: Vec2, radius: f32) -> Vec<(u32, u32, f32)> {
        let s = self.spacing();
        let half = self.size / 2.0;
        let i0 = (((center.x - radius + half.x) / s.x).floor().max(0.0)) as u32;
        let i1 =
            (((center.x + radius + half.x) / s.x).ceil() as i64).clamp(0, self.cells as i64) as u32;
        let j0 = (((center.y - radius + half.y) / s.y).floor().max(0.0)) as u32;
        let j1 =
            (((center.y + radius + half.y) / s.y).ceil() as i64).clamp(0, self.cells as i64) as u32;
        let mut out = Vec::new();
        for j in j0..=j1.min(self.cells) {
            for i in i0..=i1.min(self.cells) {
                let p = self.point(i, j);
                let w = Self::falloff(Vec2::new(p.x, p.z).distance(center), radius);
                if w > 0.0 {
                    out.push((i, j, w));
                }
            }
        }
        out
    }

    /// Um toque do pincel de altura em (x, z): `strength` em metros (no meio
    /// do pincel) por toque. Devolve se mudou alguma coisa.
    pub fn brush(&mut self, op: BrushOp, center: Vec2, radius: f32, strength: f32) -> bool {
        let radius = radius.max(1e-3);
        let pts = self.under(center, radius);
        if pts.is_empty() {
            return false;
        }
        let old = self.heights.clone();
        let side = self.side() as i64;
        let avg = |i: u32, j: u32| -> f32 {
            let mut sum = 0.0;
            let mut n = 0.0;
            for dj in -1i64..=1 {
                for di in -1i64..=1 {
                    let (a, b) = (i as i64 + di, j as i64 + dj);
                    if a >= 0 && b >= 0 && a < side && b < side {
                        sum += old[(b * side + a) as usize];
                        n += 1.0;
                    }
                }
            }
            sum / n
        };
        let m = self.max_height;
        for (i, j, w) in pts {
            let k = self.index(i, j);
            let h = old[k];
            let new = match op {
                BrushOp::Raise => h + strength * w,
                BrushOp::Lower => h - strength * w,
                BrushOp::Smooth => {
                    let t = (strength * w).clamp(0.0, 1.0);
                    h + (avg(i, j) - h) * t
                }
                BrushOp::Flatten(target) => {
                    let t = (strength * w).clamp(0.0, 1.0);
                    h + (target - h) * t
                }
                BrushOp::Noise => {
                    let n = hash(i, j) * 2.0 - 1.0;
                    h + n * strength * w
                }
            };
            self.heights[k] = new.clamp(-m, m);
        }
        true
    }

    /// Pinta a camada `layer` (0 a 3) em volta de (x, z). `strength` de 0 a 1
    /// por toque (1 = toda de uma vez no meio).
    pub fn paint(&mut self, layer: usize, center: Vec2, radius: f32, strength: f32) -> bool {
        let layer = layer.min(3);
        let pts = self.under(center, radius.max(1e-3));
        for &(i, j, w) in &pts {
            let k = self.index(i, j);
            let t = (strength * w).clamp(0.0, 1.0);
            let cur = self.layers[k];
            let mut out = [0u8; 4];
            for (c, v) in out.iter_mut().enumerate() {
                let target = if c == layer { 255.0 } else { 0.0 };
                let x = cur[c] as f32 + (target - cur[c] as f32) * t;
                *v = x.round().clamp(0.0, 255.0) as u8;
            }
            self.layers[k] = out;
        }
        !pts.is_empty()
    }

    fn color_at(&self, k: usize, colors: &[Color; 4]) -> Color {
        let l = self.layers[k];
        let total: f32 = l.iter().map(|&v| v as f32).sum();
        if total <= 0.0 {
            return colors[0];
        }
        let mut c = [0.0f32; 4];
        for (n, &w) in l.iter().enumerate() {
            let w = w as f32 / total;
            c[0] += colors[n].r * w;
            c[1] += colors[n].g * w;
            c[2] += colors[n].b * w;
            c[3] += colors[n].a * w;
        }
        Color::rgba(c[0], c[1], c[2], c[3])
    }

    /// Os dois triângulos de cada quadrado (índices dos pontos), na mesma
    /// diagonal que [`TerrainData::height_at`].
    fn triangles(&self) -> impl Iterator<Item = [usize; 3]> + '_ {
        let side = self.side();
        (0..self.cells).flat_map(move |j| {
            (0..self.cells).flat_map(move |i| {
                let a = (j * side + i) as usize;
                let b = a + 1;
                let c = ((j + 1) * side + i) as usize;
                let d = c + 1;
                // Anti-horário visto de cima (+Y).
                [[a, d, b], [a, c, d]]
            })
        })
    }

    /// A malha para desenhar: low-poly, cada triângulo com a mistura das
    /// cores dos seus pontos (com uma leve variação, para não ficar chapado).
    pub fn mesh(&self, colors: &[Color; 4]) -> Mesh {
        let mut mesh = Mesh::new();
        let pts: Vec<Vec3> = (0..self.side())
            .flat_map(|j| (0..self.side()).map(move |i| (i, j)))
            .map(|(i, j)| self.point(i, j))
            .collect();
        for (n, t) in self.triangles().enumerate() {
            let [a, b, c] = t;
            let (pa, pb, pc) = (pts[a], pts[b], pts[c]);
            let ca = self.color_at(a, colors);
            let cb = self.color_at(b, colors);
            let cc = self.color_at(c, colors);
            let jitter = 1.0 + (hash(n as u32, 7) - 0.5) * 0.08;
            let col = Color::rgba(
                ((ca.r + cb.r + cc.r) / 3.0 * jitter).clamp(0.0, 1.0),
                ((ca.g + cb.g + cc.g) / 3.0 * jitter).clamp(0.0, 1.0),
                ((ca.b + cb.b + cc.b) / 3.0 * jitter).clamp(0.0, 1.0),
                1.0,
            );
            let normal = (pb - pa).cross(pc - pa).normalize_or(Vec3::Y);
            let base = mesh.vertices.len() as u32;
            for p in [pa, pb, pc] {
                mesh.vertices.push(Vertex3::new(p, normal, col));
            }
            mesh.indices.extend([base, base + 1, base + 2]);
        }
        mesh
    }

    /// Os triângulos para a física (os mesmos do desenho).
    pub fn collision(&self) -> CollisionMesh {
        let vertices: Vec<Vec3> = (0..self.side())
            .flat_map(|j| (0..self.side()).map(move |i| (i, j)))
            .map(|(i, j)| self.point(i, j))
            .collect();
        CollisionMesh {
            vertices,
            triangles: self.triangles().map(|t| t.map(|v| v as u32)).collect(),
        }
    }

    /// O mesmo terreno com outra quantidade de quadrados (as alturas
    /// interpoladas, as cores do ponto mais perto).
    pub fn resample(&self, cells: u32) -> Self {
        let mut out = Self::flat(cells, self.size, self.max_height);
        let side = out.side();
        for j in 0..side {
            for i in 0..side {
                let p = out.point(i, j);
                let k = out.index(i, j);
                out.heights[k] = self.height_at(p.x, p.z).unwrap_or(0.0);
                let s = self.spacing();
                let oi = (((p.x + self.size.x / 2.0) / s.x).round() as u32).min(self.cells);
                let oj = (((p.z + self.size.y / 2.0) / s.y).round() as u32).min(self.cells);
                out.layers[k] = self.layers[self.index(oi, oj)];
            }
        }
        out
    }

    /// Guarda as alturas e as camadas num texto (`t1:` + base64): as
    /// alturas em 16 bits, de `-max_height` a `max_height`.
    pub fn encode(&self) -> String {
        let n = self.heights.len();
        let mut bytes = Vec::with_capacity(n * 6);
        for &h in &self.heights {
            let v = ((h / self.max_height).clamp(-1.0, 1.0) * 32767.0).round() as i16;
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        for l in &self.layers {
            bytes.extend_from_slice(l);
        }
        format!("t1:{}", base64_encode(&bytes))
    }

    /// Lê o texto de [`TerrainData::encode`]. Vazio (ou de outro tamanho):
    /// um terreno plano com esse tamanho.
    pub fn decode(text: &str, cells: u32, size: Vec2, max_height: f32) -> Self {
        let mut t = Self::flat(cells, size, max_height);
        let n = t.heights.len();
        let Some(data) = text.strip_prefix("t1:").and_then(base64_decode) else {
            return t;
        };
        if data.len() != n * 6 {
            return t;
        }
        for (k, h) in t.heights.iter_mut().enumerate() {
            let v = i16::from_le_bytes([data[2 * k], data[2 * k + 1]]);
            *h = v as f32 / 32767.0 * t.max_height;
        }
        for (k, l) in t.layers.iter_mut().enumerate() {
            let o = 2 * n + 4 * k;
            l.copy_from_slice(&data[o..o + 4]);
        }
        t
    }
}

/// Um número de 0 a 1 sorteado pelo ponto (sempre o mesmo).
fn hash(i: u32, j: u32) -> f32 {
    let mut x = i.wrapping_mul(0x8DA6_B343) ^ j.wrapping_mul(0xD816_3841) ^ 0x9E37_79B9;
    x ^= x >> 13;
    x = x.wrapping_mul(0x5BD1_E995);
    x ^= x >> 15;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        for k in 0..4 {
            if k <= c.len() {
                out.push(B64[(n >> (18 - 6 * k) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| -> Option<u32> { B64.iter().position(|&b| b == c).map(|p| p as u32) };
    let clean: Vec<u8> = s
        .bytes()
        .filter(|c| !c.is_ascii_whitespace() && *c != b'=')
        .collect();
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    for chunk in clean.chunks(4) {
        let mut n = 0u32;
        for (k, &c) in chunk.iter().enumerate() {
            n |= val(c)? << (18 - 6 * k);
        }
        let b = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        match chunk.len() {
            4 => out.extend_from_slice(&b),
            3 => out.extend_from_slice(&b[..2]),
            2 => out.push(b[0]),
            _ => return None,
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terreno() -> TerrainData {
        TerrainData::flat(32, Vec2::new(64.0, 64.0), 20.0)
    }

    #[test]
    fn plano_altura_e_raio() {
        let t = terreno();
        assert_eq!(t.heights.len(), 33 * 33);
        assert_eq!(t.height_at(0.0, 0.0), Some(0.0));
        assert_eq!(t.height_at(40.0, 0.0), None, "fora");
        let hit = t
            .raycast(Vec3::new(3.0, 10.0, -2.0), Vec3::new(0.0, -1.0, 0.0), 100.0)
            .unwrap();
        assert!(hit.abs_diff_eq(Vec3::new(3.0, 0.0, -2.0), 1e-3), "{hit}");
        assert!(
            t.raycast(Vec3::new(0.0, 10.0, 0.0), Vec3::Y, 100.0)
                .is_none()
        );
    }

    #[test]
    fn pinceis_sobem_alisam_aplainam() {
        let mut t = terreno();
        for _ in 0..10 {
            t.brush(BrushOp::Raise, Vec2::ZERO, 6.0, 0.5);
        }
        let topo = t.height_at(0.0, 0.0).unwrap();
        assert!((topo - 5.0).abs() < 0.01, "morro de 5 m: {topo}");
        let borda = t.height_at(5.0, 0.0).unwrap();
        assert!(borda > 0.0 && borda < topo, "mais baixo na borda: {borda}");
        assert_eq!(t.height_at(10.0, 0.0), Some(0.0), "fora do pincel");
        // O raio bate no morro.
        let hit = t
            .raycast(Vec3::new(-20.0, 3.0, 0.0), Vec3::X, 100.0)
            .unwrap();
        assert!(hit.x < 0.0 && hit.x > -6.0, "{hit}");
        // Alisar baixa o topo.
        for _ in 0..20 {
            t.brush(BrushOp::Smooth, Vec2::ZERO, 8.0, 1.0);
        }
        assert!(t.height_at(0.0, 0.0).unwrap() < topo);
        // Aplainar puxa para a altura escolhida.
        for _ in 0..30 {
            t.brush(BrushOp::Flatten(2.0), Vec2::ZERO, 10.0, 0.5);
        }
        assert!((t.height_at(0.0, 0.0).unwrap() - 2.0).abs() < 0.01);
        // Não passa do máximo.
        for _ in 0..100 {
            t.brush(BrushOp::Lower, Vec2::ZERO, 4.0, 5.0);
        }
        assert_eq!(t.height_at(0.0, 0.0), Some(-20.0));
        // Ruído é sempre igual.
        let mut a = terreno();
        let mut b = terreno();
        a.brush(BrushOp::Noise, Vec2::ZERO, 8.0, 1.0);
        b.brush(BrushOp::Noise, Vec2::ZERO, 8.0, 1.0);
        assert_eq!(a, b);
        assert_ne!(a, terreno());
    }

    #[test]
    fn pintar_malha_fisica_e_texto() {
        let mut t = terreno();
        t.brush(BrushOp::Raise, Vec2::new(10.0, 10.0), 5.0, 3.0);
        t.paint(3, Vec2::new(-10.0, 0.0), 4.0, 1.0);
        let k = t.index(11, 16);
        assert_eq!(t.layers[k], [0, 0, 0, 255], "areia no meio do pincel");
        let mesh = t.mesh(&DEFAULT_LAYER_COLORS);
        assert_eq!(mesh.triangle_count(), 32 * 32 * 2);
        // As normais apontam para cima (anti-horário visto de cima).
        assert!(mesh.vertices.iter().all(|v| v.normal[1] > 0.0));
        let col = t.collision();
        assert_eq!(col.triangles.len(), 32 * 32 * 2);
        assert_eq!(col.vertices.len(), 33 * 33);
        // Ida e volta pelo texto (as alturas com 16 bits).
        let text = t.encode();
        let back = TerrainData::decode(&text, 32, Vec2::new(64.0, 64.0), 20.0);
        assert_eq!(back.layers, t.layers);
        for (a, b) in back.heights.iter().zip(&t.heights) {
            assert!((a - b).abs() < 1e-3);
        }
        // Texto de outro tamanho: plano.
        let outro = TerrainData::decode(&text, 16, Vec2::new(64.0, 64.0), 20.0);
        assert!(outro.heights.iter().all(|&h| h == 0.0));
        assert_eq!(base64_decode(&base64_encode(b"oi!?")).unwrap(), b"oi!?");
        // Mais quadrados: o morro continua no mesmo lugar.
        let fino = t.resample(64);
        assert_eq!(fino.cells, 64);
        let (a, b) = (
            t.height_at(10.0, 10.0).unwrap(),
            fino.height_at(10.0, 10.0).unwrap(),
        );
        assert!((a - b).abs() < 0.05, "{a} {b}");
    }
}
