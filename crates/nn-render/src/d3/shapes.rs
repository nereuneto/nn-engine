//! Formas prontas, com as faces lisas (visual low-poly) e tamanho 1: cubo
//! de lado 1, esfera de diâmetro 1, plano 1 × 1, cilindro, cone e cápsula de
//! altura 1. Todas centradas na origem (o plano fica deitado no chão, com a
//! face para cima).

use std::f32::consts::{PI, TAU};

use glam::Vec3;
use nn_core::Color;

use super::Mesh;

const W: Color = Color::WHITE;

/// Cubo de lado 1.
pub fn cube() -> Mesh {
    let mut m = Mesh::new();
    let h = 0.5;
    let p = |x: f32, y: f32, z: f32| Vec3::new(x * h, y * h, z * h);
    // Cada face no sentido anti-horário vista de fora.
    m.flat_quad(
        p(-1., -1., 1.),
        p(1., -1., 1.),
        p(1., 1., 1.),
        p(-1., 1., 1.),
        W,
    ); // frente (+z)
    m.flat_quad(
        p(1., -1., -1.),
        p(-1., -1., -1.),
        p(-1., 1., -1.),
        p(1., 1., -1.),
        W,
    ); // trás
    m.flat_quad(
        p(1., -1., 1.),
        p(1., -1., -1.),
        p(1., 1., -1.),
        p(1., 1., 1.),
        W,
    ); // direita
    m.flat_quad(
        p(-1., -1., -1.),
        p(-1., -1., 1.),
        p(-1., 1., 1.),
        p(-1., 1., -1.),
        W,
    ); // esquerda
    m.flat_quad(
        p(-1., 1., 1.),
        p(1., 1., 1.),
        p(1., 1., -1.),
        p(-1., 1., -1.),
        W,
    ); // cima
    m.flat_quad(
        p(-1., -1., -1.),
        p(1., -1., -1.),
        p(1., -1., 1.),
        p(-1., -1., 1.),
        W,
    ); // baixo
    m
}

/// Plano 1 × 1 deitado (normal para cima).
pub fn plane() -> Mesh {
    let mut m = Mesh::new();
    m.flat_quad(
        Vec3::new(-0.5, 0.0, 0.5),
        Vec3::new(0.5, 0.0, 0.5),
        Vec3::new(0.5, 0.0, -0.5),
        Vec3::new(-0.5, 0.0, -0.5),
        W,
    );
    m
}

/// Esfera de diâmetro 1: um icosaedro subdividido `detail` vezes (0 = 20
/// faces, 1 = 80, 2 = 320).
pub fn sphere(detail: u32) -> Mesh {
    let t = (1.0 + 5f32.sqrt()) / 2.0;
    let mut verts: Vec<Vec3> = [
        (-1., t, 0.),
        (1., t, 0.),
        (-1., -t, 0.),
        (1., -t, 0.),
        (0., -1., t),
        (0., 1., t),
        (0., -1., -t),
        (0., 1., -t),
        (t, 0., -1.),
        (t, 0., 1.),
        (-t, 0., -1.),
        (-t, 0., 1.),
    ]
    .iter()
    .map(|&(x, y, z)| Vec3::new(x, y, z).normalize())
    .collect();
    let mut faces: Vec<[usize; 3]> = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    for _ in 0..detail.min(4) {
        let mut next = Vec::with_capacity(faces.len() * 4);
        let mut mid = std::collections::HashMap::new();
        let mut middle = |a: usize, b: usize, verts: &mut Vec<Vec3>| {
            let key = (a.min(b), a.max(b));
            *mid.entry(key).or_insert_with(|| {
                verts.push(((verts[a] + verts[b]) * 0.5).normalize());
                verts.len() - 1
            })
        };
        for [a, b, c] in faces {
            let ab = middle(a, b, &mut verts);
            let bc = middle(b, c, &mut verts);
            let ca = middle(c, a, &mut verts);
            next.extend([[a, ab, ca], [b, bc, ab], [c, ca, bc], [ab, bc, ca]]);
        }
        faces = next;
    }
    let mut m = Mesh::new();
    for [a, b, c] in faces {
        m.flat_triangle(verts[a] * 0.5, verts[b] * 0.5, verts[c] * 0.5, W);
    }
    m
}

/// Cilindro de altura 1 e diâmetro 1, com `segments` lados.
pub fn cylinder(segments: u32) -> Mesh {
    cone_like(segments, 0.5, 0.5)
}

/// Cone de altura 1 e base de diâmetro 1 (a ponta para cima).
pub fn cone(segments: u32) -> Mesh {
    cone_like(segments, 0.5, 0.0)
}

fn ring(segments: u32, r: f32, y: f32) -> Vec<Vec3> {
    (0..segments)
        .map(|i| {
            let a = i as f32 / segments as f32 * TAU;
            Vec3::new(a.cos() * r, y, -a.sin() * r)
        })
        .collect()
}

fn cone_like(segments: u32, bottom: f32, top: f32) -> Mesh {
    let n = segments.max(3);
    let lo = ring(n, bottom, -0.5);
    let hi = ring(n, top, 0.5);
    let mut m = Mesh::new();
    for i in 0..n as usize {
        let j = (i + 1) % n as usize;
        if top > 0.0 {
            m.flat_quad(lo[i], lo[j], hi[j], hi[i], W);
            // Tampa de cima.
            m.flat_triangle(Vec3::new(0.0, 0.5, 0.0), hi[i], hi[j], W);
        } else {
            m.flat_triangle(lo[i], lo[j], Vec3::new(0.0, 0.5, 0.0), W);
        }
        // Tampa de baixo.
        m.flat_triangle(Vec3::new(0.0, -0.5, 0.0), lo[j], lo[i], W);
    }
    m
}

/// Cápsula de altura 1 e diâmetro 0,5 (um cilindro com meias esferas nas
/// pontas): o formato de um personagem simples.
pub fn capsule(segments: u32) -> Mesh {
    let n = segments.max(4);
    let r = 0.25;
    let rings = (n / 4).max(2);
    // Anéis de cima para baixo: meia esfera de cima, cilindro, de baixo.
    let mut profile: Vec<(f32, f32)> = Vec::new();
    for k in 0..=rings {
        let a = k as f32 / rings as f32 * (PI / 2.0);
        profile.push((r * a.sin(), 0.25 + r * a.cos()));
    }
    for k in 0..=rings {
        let a = k as f32 / rings as f32 * (PI / 2.0);
        profile.push((r * a.cos(), -0.25 - r * a.sin()));
    }
    let mut m = Mesh::new();
    for w in profile.windows(2) {
        let (r0, y0) = w[0];
        let (r1, y1) = w[1];
        let a = ring(n, r0, y0);
        let b = ring(n, r1, y1);
        for i in 0..n as usize {
            let j = (i + 1) % n as usize;
            if r0 < 1e-4 {
                m.flat_triangle(a[i], b[i], b[j], W);
            } else if r1 < 1e-4 {
                m.flat_triangle(a[i], b[i], a[j], W);
            } else {
                m.flat_quad(b[i], b[j], a[j], a[i], W);
            }
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    /// As normais apontam para fora (o centro está atrás de todas as faces).
    fn normals_out(m: &Mesh) {
        for v in &m.vertices {
            let p = Vec3::from_array(v.position);
            let n = Vec3::from_array(v.normal);
            assert!(n.length() > 0.99, "normal zerada");
            // Para formas convexas centradas: a normal aponta para o mesmo
            // lado que o ponto.
            assert!(p.dot(n) > -1e-4, "normal para dentro em {p} ({n})");
        }
    }

    #[test]
    fn formas_fechadas_e_normais_para_fora() {
        for (nome, m) in [
            ("cubo", cube()),
            ("esfera", sphere(1)),
            ("cilindro", cylinder(12)),
            ("cone", cone(12)),
            ("capsula", capsule(12)),
        ] {
            assert!(m.triangle_count() > 0, "{nome}");
            normals_out(&m);
            let (min, max) = m.bounds();
            let size = max - min;
            assert!(size.y > 0.99 && size.y < 1.01, "{nome}: altura {size}");
        }
        assert_eq!(cube().triangle_count(), 12);
        assert_eq!(sphere(0).triangle_count(), 20);
        assert_eq!(sphere(1).triangle_count(), 80);
        let p = plane();
        assert_eq!(p.vertices[0].normal, [0.0, 1.0, 0.0]);
    }
}
