//! Formas, caixas alinhadas aos eixos e testes geométricos.

use glam::Vec2;

/// Tolerância usada nos testes de sobreposição. Evita que um corpo apoiado
/// exatamente sobre o chão seja tratado como dentro da parede ao andar.
pub(crate) const EPS: f32 = 1e-3;

/// Forma de colisão de um corpo. Corpos não giram.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    /// Retângulo alinhado aos eixos, guardado pela metade do tamanho.
    Rect {
        half: Vec2,
    },
    Circle {
        radius: f32,
    },
}

impl Shape {
    /// Retângulo com largura e altura.
    pub fn rect(width: f32, height: f32) -> Self {
        Self::Rect {
            half: Vec2::new(width.abs(), height.abs()) / 2.0,
        }
    }

    pub fn circle(radius: f32) -> Self {
        Self::Circle {
            radius: radius.abs(),
        }
    }

    /// Metade do tamanho da caixa que envolve a forma.
    pub fn half_extents(&self) -> Vec2 {
        match *self {
            Self::Rect { half } => half,
            Self::Circle { radius } => Vec2::splat(radius),
        }
    }

    /// Caixa que envolve a forma centrada em `center`.
    pub fn aabb(&self, center: Vec2) -> Aabb {
        Aabb::from_center(center, self.half_extents())
    }
}

/// Caixa alinhada aos eixos.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec2,
    pub max: Vec2,
}

impl Aabb {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self {
            min: min.min(max),
            max: min.max(max),
        }
    }

    pub fn from_center(center: Vec2, half: Vec2) -> Self {
        Self {
            min: center - half,
            max: center + half,
        }
    }

    pub fn center(&self) -> Vec2 {
        (self.min + self.max) / 2.0
    }

    pub fn half(&self) -> Vec2 {
        (self.max - self.min) / 2.0
    }

    /// Sobreposição com folga: só encostar não conta.
    pub fn overlaps(&self, other: &Aabb) -> bool {
        self.max.x > other.min.x + EPS
            && self.min.x < other.max.x - EPS
            && self.max.y > other.min.y + EPS
            && self.min.y < other.max.y - EPS
    }

    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    /// Ponto da caixa mais próximo de `p`.
    pub fn closest_point(&self, p: Vec2) -> Vec2 {
        p.clamp(self.min, self.max)
    }

    /// Interseção de um raio com a caixa: distância ao longo do raio e normal.
    pub fn raycast(&self, origin: Vec2, dir: Vec2, max: f32) -> Option<(f32, Vec2)> {
        let mut t_min = 0.0f32;
        let mut t_max = max;
        let mut normal = Vec2::ZERO;
        for axis in 0..2 {
            let (o, d, lo, hi) = if axis == 0 {
                (origin.x, dir.x, self.min.x, self.max.x)
            } else {
                (origin.y, dir.y, self.min.y, self.max.y)
            };
            if d.abs() < 1e-9 {
                if o < lo || o > hi {
                    return None;
                }
                continue;
            }
            let inv = 1.0 / d;
            let (mut t1, mut t2) = ((lo - o) * inv, (hi - o) * inv);
            let mut n = if axis == 0 { Vec2::NEG_X } else { Vec2::NEG_Y };
            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
                n = -n;
            }
            if t1 > t_min {
                t_min = t1;
                normal = n;
            }
            t_max = t_max.min(t2);
            if t_min > t_max {
                return None;
            }
        }
        // Raio começando dentro da caixa não conta como acerto.
        if normal == Vec2::ZERO {
            return None;
        }
        Some((t_min, normal))
    }
}

/// Os dois corpos se sobrepõem?
pub(crate) fn overlap(pa: Vec2, sa: &Shape, pb: Vec2, sb: &Shape) -> bool {
    match (sa, sb) {
        (Shape::Rect { .. }, Shape::Rect { .. }) => sa.aabb(pa).overlaps(&sb.aabb(pb)),
        (Shape::Circle { radius: ra }, Shape::Circle { radius: rb }) => {
            pa.distance_squared(pb) < (ra + rb - EPS).max(0.0).powi(2)
        }
        (Shape::Circle { radius }, Shape::Rect { .. }) => {
            sb.aabb(pb).closest_point(pa).distance_squared(pa) < (radius - EPS).max(0.0).powi(2)
        }
        (Shape::Rect { .. }, Shape::Circle { .. }) => overlap(pb, sb, pa, sa),
    }
}

/// Vetor mínimo para separar `a` de `b`: normal (apontando de `b` para `a`)
/// e profundidade. `None` se não há sobreposição.
pub(crate) fn separation(pa: Vec2, sa: &Shape, pb: Vec2, sb: &Shape) -> Option<(Vec2, f32)> {
    match (sa, sb) {
        (Shape::Rect { .. }, Shape::Rect { .. }) => {
            let (a, b) = (sa.aabb(pa), sb.aabb(pb));
            if !a.overlaps(&b) {
                return None;
            }
            let dx = (a.max.x.min(b.max.x)) - (a.min.x.max(b.min.x));
            let dy = (a.max.y.min(b.max.y)) - (a.min.y.max(b.min.y));
            if dx < dy {
                let n = if pa.x < pb.x { Vec2::NEG_X } else { Vec2::X };
                Some((n, dx))
            } else {
                let n = if pa.y < pb.y { Vec2::NEG_Y } else { Vec2::Y };
                Some((n, dy))
            }
        }
        (Shape::Circle { radius: ra }, Shape::Circle { radius: rb }) => {
            let d = pa - pb;
            let dist = d.length();
            let depth = ra + rb - dist;
            if depth <= EPS {
                return None;
            }
            let n = if dist > 1e-6 { d / dist } else { Vec2::Y };
            Some((n, depth))
        }
        (Shape::Circle { radius }, Shape::Rect { .. }) => {
            let b = sb.aabb(pb);
            let closest = b.closest_point(pa);
            let d = pa - closest;
            let dist = d.length();
            if dist > 1e-6 {
                let depth = radius - dist;
                (depth > EPS).then(|| (d / dist, depth))
            } else {
                // Centro dentro do retângulo: sai pelo lado mais próximo.
                let to_min = pa - b.min;
                let to_max = b.max - pa;
                let candidates = [
                    (Vec2::NEG_X, to_min.x),
                    (Vec2::X, to_max.x),
                    (Vec2::NEG_Y, to_min.y),
                    (Vec2::Y, to_max.y),
                ];
                let (n, d) = candidates
                    .into_iter()
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .unwrap_or((Vec2::Y, 0.0));
                Some((n, d + radius))
            }
        }
        (Shape::Rect { .. }, Shape::Circle { .. }) => {
            separation(pb, sb, pa, sa).map(|(n, d)| (-n, d))
        }
    }
}

/// Interseção de um raio com um círculo.
pub(crate) fn raycast_circle(
    center: Vec2,
    radius: f32,
    origin: Vec2,
    dir: Vec2,
    max: f32,
) -> Option<(f32, Vec2)> {
    let m = origin - center;
    let b = m.dot(dir);
    let c = m.length_squared() - radius * radius;
    if c <= 0.0 {
        return None; // começa dentro
    }
    if b > 0.0 {
        return None;
    }
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    (t >= 0.0 && t <= max).then(|| (t, (origin + dir * t - center).normalize_or(Vec2::Y)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encostar_nao_e_sobrepor() {
        let a = Aabb::from_center(Vec2::ZERO, Vec2::splat(1.0));
        let b = Aabb::from_center(Vec2::new(2.0, 0.0), Vec2::splat(1.0));
        assert!(!a.overlaps(&b));
        let c = Aabb::from_center(Vec2::new(1.9, 0.0), Vec2::splat(1.0));
        assert!(a.overlaps(&c));
    }

    #[test]
    fn raio_na_caixa() {
        let b = Aabb::new(Vec2::new(2.0, -1.0), Vec2::new(4.0, 1.0));
        let (t, n) = b.raycast(Vec2::ZERO, Vec2::X, 10.0).unwrap();
        assert!((t - 2.0).abs() < 1e-5);
        assert_eq!(n, Vec2::NEG_X);
        assert!(b.raycast(Vec2::ZERO, Vec2::X, 1.5).is_none());
        assert!(b.raycast(Vec2::ZERO, Vec2::NEG_X, 10.0).is_none());
        let (t, n) = b.raycast(Vec2::new(3.0, 5.0), Vec2::NEG_Y, 10.0).unwrap();
        assert!((t - 4.0).abs() < 1e-5);
        assert_eq!(n, Vec2::Y);
    }

    #[test]
    fn raio_no_circulo() {
        let (t, n) = raycast_circle(Vec2::new(5.0, 0.0), 1.0, Vec2::ZERO, Vec2::X, 10.0).unwrap();
        assert!((t - 4.0).abs() < 1e-5);
        assert!((n - Vec2::NEG_X).length() < 1e-5);
        assert!(raycast_circle(Vec2::new(5.0, 3.0), 1.0, Vec2::ZERO, Vec2::X, 10.0).is_none());
    }

    #[test]
    fn separacao_retangulos_pelo_menor_eixo() {
        let s = Shape::rect(2.0, 2.0);
        let (n, d) = separation(Vec2::new(0.0, 1.5), &s, Vec2::ZERO, &s).unwrap();
        assert_eq!(n, Vec2::Y);
        assert!((d - 0.5).abs() < 1e-5);
    }

    #[test]
    fn separacao_circulo_retangulo() {
        let c = Shape::circle(1.0);
        let r = Shape::rect(4.0, 2.0);
        let (n, d) = separation(Vec2::new(0.0, 1.5), &c, Vec2::ZERO, &r).unwrap();
        assert!((n - Vec2::Y).length() < 1e-5);
        assert!((d - 0.5).abs() < 1e-5);
        assert!(overlap(Vec2::new(0.0, 1.5), &c, Vec2::ZERO, &r));
        assert!(!overlap(Vec2::new(0.0, 2.5), &c, Vec2::ZERO, &r));
    }
}
