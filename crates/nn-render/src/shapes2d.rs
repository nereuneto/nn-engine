//! Linhas, contornos e polígonos 2D, em triângulos.
//!
//! O renderer desenha os triângulos no mesmo lote das imagens (uma instância
//! de sprite por triângulo, com a textura branca). Cada triângulo diz quais
//! dos lados dele são a borda da forma: só esses ganham a borda suave
//! (antialias); os lados de dentro (entre dois triângulos) ficam sem, para não
//! aparecer costura.

use glam::Vec2;

/// Um triângulo de uma forma: os três cantos e quais lados são borda.
///
/// Os bits de `edges`: 1 = do canto 0 ao 1, 2 = do 1 ao 2, 4 = do 2 ao 0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Tri {
    pub points: [Vec2; 3],
    pub edges: u8,
}

/// O ângulo mais fechado em que a junta de uma linha ainda fica em ponta
/// (miter): o comprimento da ponta, em meias espessuras. Mais fechado que
/// isso, a junta é cortada (bevel), como no SVG.
const MITER_LIMIT: f32 = 4.0;

/// Tira os pontos repetidos seguidos (e o último, se igual ao primeiro numa
/// linha fechada).
fn clean(points: &[Vec2], closed: bool) -> Vec<Vec2> {
    let mut out: Vec<Vec2> = Vec::with_capacity(points.len());
    for &p in points {
        if !p.is_finite() {
            continue;
        }
        if out.last().is_none_or(|q| q.distance_squared(p) > 1e-10) {
            out.push(p);
        }
    }
    if closed && out.len() > 2 && out[0].distance_squared(out[out.len() - 1]) <= 1e-10 {
        out.pop();
    }
    out
}

/// Uma linha com espessura, passando pelos pontos. `closed` liga o último ao
/// primeiro (um contorno).
pub(crate) fn polyline(points: &[Vec2], width: f32, closed: bool, out: &mut Vec<Tri>) {
    let pts = clean(points, closed);
    let hw = width.max(0.0) / 2.0;
    if pts.len() < 2 || hw <= 0.0 {
        return;
    }
    let n = pts.len();
    let segments = if closed && n > 2 { n } else { n - 1 };
    let closed = segments == n;
    let normal = |i: usize| {
        let d = (pts[(i + 1) % n] - pts[i]).normalize_or_zero();
        Vec2::new(-d.y, d.x)
    };
    // Os dois cantos (esquerda, direita) de cada ponta de cada segmento.
    // Numa junta em ponta, o fim de um e o começo do outro são os mesmos
    // pontos; numa junta cortada, o lado de fora ganha um triângulo.
    let mut starts = vec![[Vec2::ZERO; 2]; segments];
    let mut ends = vec![[Vec2::ZERO; 2]; segments];
    let mut bevels = Vec::new();
    for (s, start) in starts.iter_mut().enumerate() {
        let nrm = normal(s);
        *start = [pts[s] + nrm * hw, pts[s] - nrm * hw];
        ends[s] = [pts[(s + 1) % n] + nrm * hw, pts[(s + 1) % n] - nrm * hw];
    }
    let joints: Vec<usize> = if closed {
        (0..n).collect()
    } else {
        (1..n - 1).collect()
    };
    for j in joints {
        // A junta entre o segmento que chega (a) e o que sai (b) no ponto j.
        let a = (j + segments - 1) % segments;
        let b = j % segments;
        let (na, nb) = (normal(a), normal(b));
        let sum = na + nb;
        let cos = na.dot(nb);
        if sum.length_squared() < 1e-8 {
            // Volta para trás: fica cada um com a ponta dele.
            continue;
        }
        let miter = sum.normalize();
        let len = hw / miter.dot(nb).max(1e-4);
        if len <= hw * MITER_LIMIT {
            let left = pts[j] + miter * len;
            let right = pts[j] - miter * len;
            ends[a] = [left, right];
            starts[b] = [left, right];
        } else {
            // Cortada: o lado de dentro se encontra, o de fora ganha um
            // triângulo entre as duas pontas.
            let turn = na.perp_dot(nb);
            let inner_len = (hw / miter.dot(nb).abs().max(1e-4)).min(
                (pts[j] - pts[a])
                    .length()
                    .min((pts[(j + 1) % n] - pts[j]).length())
                    + hw,
            );
            if turn > 0.0 || (turn == 0.0 && cos < 0.0) {
                // Vira para a esquerda: o lado de fora é o da direita.
                let inner = pts[j] + miter * inner_len;
                ends[a][0] = inner;
                starts[b][0] = inner;
                bevels.push(Tri {
                    points: [pts[j], ends[a][1], starts[b][1]],
                    edges: 2,
                });
            } else {
                let inner = pts[j] - miter * inner_len;
                ends[a][1] = inner;
                starts[b][1] = inner;
                bevels.push(Tri {
                    points: [pts[j], ends[a][0], starts[b][0]],
                    edges: 2,
                });
            }
        }
    }
    for s in 0..segments {
        let [a0, a1] = starts[s];
        let [b0, b1] = ends[s];
        // As pontas soltas (começo e fim de uma linha aberta) são borda.
        let start_cap = !closed && s == 0;
        let end_cap = !closed && s == segments - 1;
        out.push(Tri {
            points: [a0, b0, a1],
            edges: 1 | if start_cap { 4 } else { 0 },
        });
        out.push(Tri {
            points: [b1, a1, b0],
            edges: 1 | if end_cap { 4 } else { 0 },
        });
    }
    out.extend(bevels);
}

/// Área com sinal (positiva no sentido anti-horário com Y para cima).
fn signed_area(pts: &[Vec2]) -> f32 {
    let mut a = 0.0;
    for i in 0..pts.len() {
        let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
        a += p.perp_dot(q);
    }
    a / 2.0
}

/// `p` dentro do triângulo abc (ou na borda), com abc no sentido positivo.
fn inside(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    (b - a).perp_dot(p - a) >= 0.0
        && (c - b).perp_dot(p - b) >= 0.0
        && (a - c).perp_dot(p - c) >= 0.0
}

/// Um polígono preenchido (também côncavo), cortado em triângulos pelas
/// orelhas. Os lados do polígono são borda; os cortes, não.
pub(crate) fn polygon(points: &[Vec2], out: &mut Vec<Tri>) {
    let pts = clean(points, true);
    let n = pts.len();
    if n < 3 {
        return;
    }
    let area = signed_area(&pts);
    if area.abs() < 1e-8 {
        return;
    }
    // Os índices no sentido positivo.
    let mut idx: Vec<usize> = if area > 0.0 {
        (0..n).collect()
    } else {
        (0..n).rev().collect()
    };
    // Um lado entre dois pontos vizinhos no polígono original é borda.
    let side = |i: usize, j: usize| (i + 1) % n == j || (j + 1) % n == i;
    let mut guard = 0;
    while idx.len() > 3 && guard < n * n {
        guard += 1;
        let m = idx.len();
        let mut cut = None;
        for k in 0..m {
            let (ia, ib, ic) = (idx[(k + m - 1) % m], idx[k], idx[(k + 1) % m]);
            let (a, b, c) = (pts[ia], pts[ib], pts[ic]);
            // Uma orelha: o canto é convexo e nenhum outro ponto fica dentro.
            if (b - a).perp_dot(c - b) <= 0.0 {
                continue;
            }
            let free = idx
                .iter()
                .all(|&o| o == ia || o == ib || o == ic || !inside(pts[o], a, b, c));
            if free {
                cut = Some(k);
                break;
            }
        }
        // Sem orelha (polígono que se cruza): corta o canto que sobrar.
        let k = cut.unwrap_or(0);
        let m = idx.len();
        let (ia, ib, ic) = (idx[(k + m - 1) % m], idx[k], idx[(k + 1) % m]);
        push_split(
            out,
            [pts[ia], pts[ib], pts[ic]],
            [side(ia, ib), side(ib, ic), side(ic, ia)],
            3,
        );
        idx.remove(k);
    }
    if idx.len() == 3 {
        let (ia, ib, ic) = (idx[0], idx[1], idx[2]);
        push_split(
            out,
            [pts[ia], pts[ib], pts[ic]],
            [side(ia, ib), side(ib, ic), side(ic, ia)],
            3,
        );
    }
}

/// Põe o triângulo, cortando antes os que têm um ângulo obtuso junto de um
/// lado de borda. A borda suave mede a distância até a reta do lado, e num
/// triângulo assim a reta passa perto de pontos que estão longe do lado (no
/// prolongamento dele): eles ficariam meio apagados no meio do polígono. O
/// corte pela perpendicular na ponta do lado separa essa parte, que fica sem
/// borda.
fn push_split(out: &mut Vec<Tri>, p: [Vec2; 3], border: [bool; 3], depth: u32) {
    if depth > 0 {
        for e in 0..3 {
            if !border[e] {
                continue;
            }
            // O lado e vai de p[e] a p[e+1]; k é o canto oposto.
            let (i0, i1, k) = (e, (e + 1) % 3, (e + 2) % 3);
            let (e0, e1, kp) = (p[i0], p[i1], p[k]);
            let d = (e1 - e0).normalize_or_zero();
            if d == Vec2::ZERO {
                continue;
            }
            // Obtuso em e1 (o canto oposto passa da ponta) ou em e0.
            let (a, b, beyond_end) = if (kp - e1).dot(d) > 1e-3 {
                (e0, e1, true)
            } else if (kp - e0).dot(d) < -1e-3 {
                (e1, e0, false)
            } else {
                continue;
            };
            // Q: onde a perpendicular por b corta o lado de a até kp.
            let along = (kp - a).dot(d);
            let t = if along.abs() > 1e-6 {
                (b - a).dot(d) / along
            } else {
                continue;
            };
            let q = a + (kp - a) * t;
            // As bordas dos lados de a até kp e de b até kp.
            let (a_k, b_k) = if beyond_end {
                (border[k], border[i1])
            } else {
                (border[i1], border[k])
            };
            push_split(out, [a, b, q], [true, false, a_k], depth - 1);
            push_split(out, [q, b, kp], [false, b_k, a_k], depth - 1);
            return;
        }
    }
    out.push(Tri {
        points: p,
        edges: u8::from(border[0]) | (u8::from(border[1]) << 1) | (u8::from(border[2]) << 2),
    });
}

/// Os pontos de um círculo, com lados curtos o bastante para parecer redondo.
pub(crate) fn circle_points(center: Vec2, radius: f32, pixels_per_unit: f32) -> Vec<Vec2> {
    let r = radius.abs() * pixels_per_unit.max(0.01);
    // Uns 3 px por lado, de 12 a 256 lados.
    let sides = ((std::f32::consts::TAU * r / 3.0).ceil() as usize).clamp(12, 256);
    (0..sides)
        .map(|i| {
            let a = i as f32 / sides as f32 * std::f32::consts::TAU;
            center + Vec2::new(a.cos(), a.sin()) * radius.abs()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(tris: &[Tri]) -> f32 {
        tris.iter()
            .map(|t| {
                let [a, b, c] = t.points;
                ((b - a).perp_dot(c - a) / 2.0).abs()
            })
            .sum()
    }

    #[test]
    fn linha_reta_vira_um_retangulo() {
        let mut t = Vec::new();
        polyline(&[Vec2::ZERO, Vec2::new(10.0, 0.0)], 2.0, false, &mut t);
        assert_eq!(t.len(), 2);
        assert!((area(&t) - 20.0).abs() < 1e-4);
        // Os lados compridos e as duas pontas são borda.
        assert_eq!(t[0].edges, 1 | 4);
        assert_eq!(t[1].edges, 1 | 4);
    }

    #[test]
    fn quadrado_fechado_tem_cantos_em_ponta() {
        let mut t = Vec::new();
        let q = [
            Vec2::ZERO,
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(0.0, 10.0),
        ];
        polyline(&q, 2.0, true, &mut t);
        assert_eq!(t.len(), 8);
        // Moldura de 12x12 menos 8x8.
        assert!((area(&t) - (144.0 - 64.0)).abs() < 1e-3, "{}", area(&t));
        // O último ponto igual ao primeiro não muda nada.
        let mut u = Vec::new();
        polyline(&[q[0], q[1], q[2], q[3], q[0]], 2.0, true, &mut u);
        assert_eq!(t, u);
    }

    #[test]
    fn angulo_fechado_corta_a_ponta() {
        let mut t = Vec::new();
        polyline(
            &[
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                Vec2::new(0.0, 1.0),
            ],
            2.0,
            false,
            &mut t,
        );
        // Dois segmentos e o triângulo do corte.
        assert_eq!(t.len(), 5);
        let far = t
            .iter()
            .flat_map(|t| t.points)
            .map(|p| p.x)
            .fold(f32::MIN, f32::max);
        assert!(far < 12.0, "a ponta não vai longe: {far}");
    }

    #[test]
    fn poligono_concavo_cobre_a_area_certa() {
        // Um L: 3x3 menos o quadrado de cima à direita (2x2): área 5.
        let l = [
            Vec2::new(0.0, 0.0),
            Vec2::new(3.0, 0.0),
            Vec2::new(3.0, 1.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 3.0),
            Vec2::new(0.0, 3.0),
        ];
        for pts in [l.to_vec(), l.iter().rev().copied().collect()] {
            let mut t = Vec::new();
            polygon(&pts, &mut t);
            assert!((area(&t) - 5.0).abs() < 1e-4);
            // A borda soma o perímetro do L (12).
            let borda: f32 = t
                .iter()
                .flat_map(|t| {
                    (0..3)
                        .filter(|e| t.edges & (1 << e) != 0)
                        .map(|e| t.points[e].distance(t.points[(e + 1) % 3]))
                })
                .sum();
            assert!((borda - 12.0).abs() < 1e-3, "{borda}");
            // Nenhum triângulo com ângulo obtuso junto de um lado de borda.
            for t in &t {
                for e in 0..3 {
                    if t.edges & (1 << e) == 0 {
                        continue;
                    }
                    let (a, b, k) = (t.points[e], t.points[(e + 1) % 3], t.points[(e + 2) % 3]);
                    let d = (b - a).normalize();
                    assert!((k - a).dot(d) >= -1e-3 && (k - b).dot(d) <= 1e-3, "{t:?}");
                }
            }
        }
    }

    #[test]
    fn formas_vazias_nao_desenham() {
        let mut t = Vec::new();
        polygon(&[Vec2::ZERO, Vec2::ONE], &mut t);
        polygon(&[Vec2::ZERO, Vec2::ONE, Vec2::splat(2.0)], &mut t);
        polyline(&[Vec2::ZERO, Vec2::ZERO], 2.0, false, &mut t);
        polyline(&[Vec2::ZERO, Vec2::ONE], 0.0, false, &mut t);
        polyline(&[Vec2::ZERO, Vec2::new(f32::NAN, 0.0)], 1.0, false, &mut t);
        assert!(t.is_empty());
    }

    #[test]
    fn circulo_tem_mais_lados_quando_maior() {
        assert_eq!(circle_points(Vec2::ZERO, 2.0, 1.0).len(), 12);
        let grande = circle_points(Vec2::ZERO, 100.0, 1.0).len();
        assert!(grande > 150, "{grande}");
        assert!(circle_points(Vec2::ZERO, 1e6, 1.0).len() <= 256);
    }
}
