//! Malhas 3D na memória: vértices com posição, normal e cor, e índices.

use glam::{Vec2, Vec3};
use nn_core::Color;

/// Um vértice: posição, normal (para a luz), cor e coordenada de textura.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex3 {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

impl Vertex3 {
    pub fn new(position: Vec3, normal: Vec3, color: Color) -> Self {
        Self {
            position: position.to_array(),
            normal: normal.normalize_or_zero().to_array(),
            color: [color.r, color.g, color.b, color.a],
            uv: [0.0, 0.0],
        }
    }

    pub fn with_uv(mut self, uv: Vec2) -> Self {
        self.uv = uv.to_array();
        self
    }
}

/// Uma malha: triângulos (três índices cada, no sentido anti-horário vistos
/// de fora).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<Vertex3>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn new() -> Self {
        Self::default()
    }

    /// Acrescenta um triângulo com a normal da face (sombreado "low-poly":
    /// cada face tem uma cor de luz só).
    pub fn flat_triangle(&mut self, a: Vec3, b: Vec3, c: Vec3, color: Color) {
        let n = (b - a).cross(c - a).normalize_or_zero();
        let base = self.vertices.len() as u32;
        for p in [a, b, c] {
            self.vertices.push(Vertex3::new(p, n, color));
        }
        self.indices.extend([base, base + 1, base + 2]);
    }

    /// Acrescenta um quadrilátero (a, b, c, d no sentido anti-horário).
    pub fn flat_quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3, color: Color) {
        let n = (b - a).cross(c - a).normalize_or_zero();
        let base = self.vertices.len() as u32;
        let uvs = [
            Vec2::new(0.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(0.0, 0.0),
        ];
        for (p, uv) in [a, b, c, d].into_iter().zip(uvs) {
            self.vertices.push(Vertex3::new(p, n, color).with_uv(uv));
        }
        self.indices
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    /// Junta outra malha a esta.
    pub fn append(&mut self, other: &Mesh) {
        let base = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&other.vertices);
        self.indices.extend(other.indices.iter().map(|i| i + base));
    }

    /// Quantos triângulos.
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Caixa que envolve a malha (mínimo, máximo).
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for v in &self.vertices {
            let p = Vec3::from_array(v.position);
            min = min.min(p);
            max = max.max(p);
        }
        if self.vertices.is_empty() {
            (Vec3::ZERO, Vec3::ZERO)
        } else {
            (min, max)
        }
    }
}
