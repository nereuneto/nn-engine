//! Um modelo 3D desenhado sem placa de vídeo, para quem não abre o NN Studio
//! (o servidor MCP): os triângulos do `.glb` (o mesmo que o jogo usa), cada
//! face de uma cor com uma luz de cima, em quatro vistas lado a lado.

use glam::{Mat4, Vec3};
use image::{Rgba, RgbaImage};

/// Um triângulo pronto para desenhar: os pontos (no espaço do modelo) e a cor
/// (sRGB, 0 a 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tri {
    pub p: [Vec3; 3],
    pub color: [f32; 3],
}

/// Os triângulos de um modelo lido de um `.glb`, na pose de descanso ou num
/// quadro de uma animação (`(índice, segundos)`).
pub fn triangles(data: &nn_render::d3::ModelData, anim: Option<(usize, f32)>) -> Vec<Tri> {
    let joints: Vec<Mat4> = match &data.skeleton {
        Some(s) => {
            let pose = match anim {
                Some((i, t)) => s.pose(i, t),
                None => s.rest_pose(),
            };
            s.joint_matrices(&pose)
        }
        None => Vec::new(),
    };
    let mut out = Vec::new();
    for part in &data.parts {
        let image = part.image.and_then(|i| data.images.get(i));
        let place = |k: usize| -> Vec3 {
            let v = &part.mesh.vertices[k];
            let p = Vec3::from_array(v.position);
            match part.skin.get(k) {
                Some(s) if !joints.is_empty() => {
                    let mut acc = Vec3::ZERO;
                    let mut total = 0.0;
                    for (j, w) in s.joints.iter().zip(s.weights) {
                        if w > 0.0
                            && let Some(m) = joints.get(*j as usize)
                        {
                            acc += m.transform_point3(p) * w;
                            total += w;
                        }
                    }
                    if total > 0.0 { acc / total } else { p }
                }
                _ => p,
            }
        };
        for tri in part.mesh.indices.as_chunks::<3>().0 {
            let k = tri.map(|i| i as usize);
            if k.iter().any(|&i| i >= part.mesh.vertices.len()) {
                continue;
            }
            let v = k.map(|i| part.mesh.vertices[i]);
            let mut c = [0.0f32; 3];
            for vert in &v {
                for (a, b) in c.iter_mut().zip(vert.color) {
                    *a += b / 3.0;
                }
            }
            // A imagem do modelo: a cor no meio do triângulo.
            if let Some(img) = image
                && img.size.x > 0
                && img.size.y > 0
            {
                let u = (v[0].uv[0] + v[1].uv[0] + v[2].uv[0]) / 3.0;
                let w = (v[0].uv[1] + v[1].uv[1] + v[2].uv[1]) / 3.0;
                let x = ((u.rem_euclid(1.0)) * img.size.x as f32) as u32 % img.size.x;
                let y = ((w.rem_euclid(1.0)) * img.size.y as f32) as u32 % img.size.y;
                let i = ((y * img.size.x + x) * 4) as usize;
                if let Some(px) = img.rgba.get(i..i + 3) {
                    for (a, b) in c.iter_mut().zip(px) {
                        *a *= f32::from(*b) / 255.0;
                    }
                }
            }
            out.push(Tri {
                p: k.map(place),
                color: c,
            });
        }
    }
    out
}

/// A caixa que envolve os triângulos (mínimo, máximo).
pub fn bounds(tris: &[Tri]) -> Option<(Vec3, Vec3)> {
    let mut it = tris.iter().flat_map(|t| t.p);
    let first = it.next()?;
    Some(it.fold((first, first), |(a, b), p| (a.min(p), b.max(p))))
}

/// De onde se olha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// De +Z (a frente dos modelos), olhando para -Z.
    Front,
    /// De +X, olhando para -X.
    Side,
    /// De cima, com o +Z (a frente) embaixo.
    Top,
    /// Na diagonal, de cima, da frente e da direita.
    ThreeQuarter,
    /// De -Z (as costas), olhando para +Z.
    Back,
}

impl View {
    /// (direita, cima, para a câmera) na tela.
    fn basis(self) -> (Vec3, Vec3, Vec3) {
        match self {
            View::Front => (Vec3::X, Vec3::Y, Vec3::Z),
            View::Side => (Vec3::NEG_Z, Vec3::Y, Vec3::X),
            View::Top => (Vec3::X, Vec3::NEG_Z, Vec3::Y),
            View::Back => (Vec3::NEG_X, Vec3::Y, Vec3::NEG_Z),
            View::ThreeQuarter => {
                let d = Vec3::new(1.0, 0.75, 1.3).normalize();
                let right = Vec3::Y.cross(d).normalize();
                (right, d.cross(right), d)
            }
        }
    }

    fn ortho(self) -> bool {
        self != View::ThreeQuarter
    }
}

const BG: Rgba<u8> = Rgba([0x26, 0x2b, 0x3a, 255]);
const GRID: Rgba<u8> = Rgba([0x33, 0x3a, 0x4d, 255]);
const GROUND: Rgba<u8> = Rgba([0x6d, 0x7f, 0x9a, 255]);
const ORIGIN: Rgba<u8> = Rgba([0xff, 0x6b, 0x6b, 255]);
const FRONT: Rgba<u8> = Rgba([0xff, 0xd1, 0x66, 255]);

/// Um quadro de `cell` x `cell` com o modelo visto de `view`. `span` é
/// quantos metros cabem na largura (o mesmo nas vistas retas, para comparar).
fn draw_view(tris: &[Tri], view: View, cell: u32, span: f32) -> RgbaImage {
    match bounds(tris) {
        Some(area) => draw_view_in(tris, view, cell, span, area, &[]),
        None => RgbaImage::from_pixel(cell, cell, BG),
    }
}

/// Um osso desenhado: da junta até a ponta.
pub type BoneLine = (Vec3, Vec3);

const BONE: Rgba<u8> = Rgba([0x5c, 0xff, 0x9d, 255]);
const JOINT: Rgba<u8> = Rgba([0xff, 0xff, 0xff, 255]);

/// Como [`draw_view`], enquadrando `area` (o mesmo em todos os quadros de
/// uma animação) e com os ossos por cima.
fn draw_view_in(
    tris: &[Tri],
    view: View,
    cell: u32,
    span: f32,
    area: (Vec3, Vec3),
    bones: &[BoneLine],
) -> RgbaImage {
    let mut img = RgbaImage::from_pixel(cell, cell, BG);
    let (right, up, toward) = view.basis();
    let (min, max) = area;
    // O meio do que se vê, na tela.
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, max.y, max.z),
    ];
    let (mut lo, mut hi) = (glam::Vec2::splat(f32::MAX), glam::Vec2::splat(f32::MIN));
    for c in corners {
        let s = glam::Vec2::new(c.dot(right), c.dot(up));
        lo = lo.min(s);
        hi = hi.max(s);
    }
    let span = if view.ortho() {
        span
    } else {
        (hi - lo).max_element() * 1.15
    }
    .max(1e-3);
    let k = cell as f32 / span;
    let mid = (lo + hi) / 2.0;
    let half = cell as f32 / 2.0;
    let to_screen = |p: Vec3| -> (f32, f32, f32) {
        (
            half + (p.dot(right) - mid.x) * k,
            half - (p.dot(up) - mid.y) * k,
            p.dot(toward),
        )
    };
    // A grade de 1 m e o chão (y = 0), nas vistas retas.
    if view.ortho() {
        let step = if span > 40.0 {
            10.0
        } else if span > 12.0 {
            2.0
        } else {
            1.0
        };
        let (a, b) = (
            mid - glam::Vec2::splat(span / 2.0),
            mid + glam::Vec2::splat(span / 2.0),
        );
        let mut x = (a.x / step).floor() * step;
        while x <= b.x {
            let sx = half + (x - mid.x) * k;
            if (0.0..cell as f32).contains(&sx) {
                for y in 0..cell {
                    img.put_pixel(sx as u32, y, GRID);
                }
            }
            x += step;
        }
        let mut y = (a.y / step).floor() * step;
        while y <= b.y {
            let sy = half - (y - mid.y) * k;
            if (0.0..cell as f32).contains(&sy) {
                let color = if y.abs() < 1e-4 && view != View::Top {
                    GROUND
                } else {
                    GRID
                };
                for x in 0..cell {
                    img.put_pixel(x, sy as u32, color);
                }
            }
            y += step;
        }
    }
    // Os triângulos, com a luz de cima (o lado de dentro também aparece).
    let light = Vec3::new(-0.35, 0.85, 0.45).normalize();
    let mut depth = vec![f32::MIN; (cell * cell) as usize];
    for t in tris {
        let mut n = (t.p[1] - t.p[0]).cross(t.p[2] - t.p[0]).normalize_or_zero();
        if n.dot(toward) < 0.0 {
            n = -n;
        }
        let shade = 0.55 + 0.45 * n.dot(light).max(0.0);
        let px = Rgba([
            (t.color[0] * shade * 255.0).clamp(0.0, 255.0) as u8,
            (t.color[1] * shade * 255.0).clamp(0.0, 255.0) as u8,
            (t.color[2] * shade * 255.0).clamp(0.0, 255.0) as u8,
            255,
        ]);
        let s = t.p.map(to_screen);
        let area = (s[1].0 - s[0].0) * (s[2].1 - s[0].1) - (s[1].1 - s[0].1) * (s[2].0 - s[0].0);
        if area.abs() < 1e-6 {
            continue;
        }
        let x0 = s
            .iter()
            .map(|p| p.0)
            .fold(f32::MAX, f32::min)
            .floor()
            .max(0.0) as u32;
        let x1 = s
            .iter()
            .map(|p| p.0)
            .fold(f32::MIN, f32::max)
            .ceil()
            .min(cell as f32 - 1.0);
        let y0 = s
            .iter()
            .map(|p| p.1)
            .fold(f32::MAX, f32::min)
            .floor()
            .max(0.0) as u32;
        let y1 = s
            .iter()
            .map(|p| p.1)
            .fold(f32::MIN, f32::max)
            .ceil()
            .min(cell as f32 - 1.0);
        if x1 < 0.0 || y1 < 0.0 {
            continue;
        }
        for y in y0..=y1 as u32 {
            for x in x0..=x1 as u32 {
                let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
                let w0 = ((s[1].0 - fx) * (s[2].1 - fy) - (s[1].1 - fy) * (s[2].0 - fx)) / area;
                let w1 = ((s[2].0 - fx) * (s[0].1 - fy) - (s[2].1 - fy) * (s[0].0 - fx)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                    continue;
                }
                let z = w0 * s[0].2 + w1 * s[1].2 + w2 * s[2].2;
                let i = (y * cell + x) as usize;
                if z > depth[i] {
                    depth[i] = z;
                    img.put_pixel(x, y, px);
                }
            }
        }
    }
    // A frente (+Z): um traço amarelo saindo da origem, nas vistas em que
    // ela não aponta para quem olha.
    if !matches!(view, View::Front | View::Back) {
        let (x0, y0, _) = to_screen(Vec3::ZERO);
        let (x1, y1, _) = to_screen(Vec3::Z * (span * 0.12).max(0.3));
        let steps = ((x1 - x0).abs().max((y1 - y0).abs()) as i32).max(1);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let (x, y) = (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1)] {
                let (px, py) = (x as i32 + dx, y as i32 + dy);
                if px >= 0 && py >= 0 && (px as u32) < cell && (py as u32) < cell {
                    img.put_pixel(px as u32, py as u32, FRONT);
                }
            }
        }
    }
    // A origem (o ponto que fica na posição do objeto na cena).
    let (ox, oy, _) = to_screen(Vec3::ZERO);
    for d in -4i32..=4 {
        for (x, y) in [(ox as i32 + d, oy as i32), (ox as i32, oy as i32 + d)] {
            if x >= 0 && y >= 0 && (x as u32) < cell && (y as u32) < cell {
                img.put_pixel(x as u32, y as u32, ORIGIN);
            }
        }
    }
    // Os ossos por cima de tudo: a linha verde e a junta branca.
    let dot = |img: &mut RgbaImage, x: f32, y: f32, r: i32, c: Rgba<u8>| {
        for dy in -r..=r {
            for dx in -r..=r {
                let (px, py) = (x as i32 + dx, y as i32 + dy);
                if dx * dx + dy * dy <= r * r
                    && px >= 0
                    && py >= 0
                    && (px as u32) < cell
                    && (py as u32) < cell
                {
                    img.put_pixel(px as u32, py as u32, c);
                }
            }
        }
    };
    for (a, b) in bones {
        let (x0, y0, _) = to_screen(*a);
        let (x1, y1, _) = to_screen(*b);
        let steps = ((x1 - x0).abs().max((y1 - y0).abs()) as i32).max(1);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            dot(&mut img, x0 + (x1 - x0) * t, y0 + (y1 - y0) * t, 1, BONE);
        }
        dot(&mut img, x0, y0, 3, JOINT);
    }
    img
}

/// As quatro vistas com os ossos por cima.
pub fn render_bones(tris: &[Tri], cell: u32, bones: &[BoneLine]) -> RgbaImage {
    let gap = 2;
    let mut out = RgbaImage::from_pixel(
        cell * 2 + gap,
        cell * 2 + gap,
        Rgba([0x10, 0x12, 0x18, 255]),
    );
    let Some(mut area) = bounds(tris) else {
        return out;
    };
    for (a, b) in bones {
        area = (area.0.min(a.min(*b)), area.1.max(a.max(*b)));
    }
    let span = ((area.1 - area.0).max_element() * 1.2).max(0.5);
    for (i, view) in [View::Front, View::Side, View::Top, View::ThreeQuarter]
        .into_iter()
        .enumerate()
    {
        let img = draw_view_in(tris, view, cell, span, area, bones);
        let (x, y) = ((i as u32 % 2) * (cell + gap), (i as u32 / 2) * (cell + gap));
        image::imageops::replace(&mut out, &img, i64::from(x), i64::from(y));
    }
    out
}

/// Os quadros de uma animação lado a lado, todos no mesmo enquadramento (o
/// que mexe, mexe na tira), cada um com os seus ossos.
pub fn render_strip(frames: &[(Vec<Tri>, Vec<BoneLine>)], view: View, cell: u32) -> RgbaImage {
    let gap = 2;
    let n = frames.len().max(1) as u32;
    let mut out = RgbaImage::from_pixel(
        cell * n + gap * (n - 1),
        cell,
        Rgba([0x10, 0x12, 0x18, 255]),
    );
    let mut area: Option<(Vec3, Vec3)> = None;
    for (tris, _) in frames {
        if let Some(b) = bounds(tris) {
            area = Some(match area {
                Some(a) => (a.0.min(b.0), a.1.max(b.1)),
                None => b,
            });
        }
    }
    let Some(area) = area else {
        return out;
    };
    let span = ((area.1 - area.0).max_element() * 1.2).max(0.5);
    for (i, (tris, bones)) in frames.iter().enumerate() {
        let img = draw_view_in(tris, view, cell, span, area, bones);
        image::imageops::replace(&mut out, &img, i64::from(i as u32 * (cell + gap)), 0);
    }
    out
}

/// Quantos metros cabem numa vista: o modelo com uma folga.
fn auto_span(tris: &[Tri]) -> f32 {
    bounds(tris)
        .map(|(a, b)| (b - a).max_element() * 1.2)
        .unwrap_or(2.0)
        .max(0.5)
}

/// As quatro vistas, 2 x 2: frente e lado em cima; de cima e 3/4 embaixo.
/// As três retas têm a mesma escala (dá para comparar os tamanhos).
pub fn render(tris: &[Tri], cell: u32) -> RgbaImage {
    render_span(tris, cell, None)
}

/// Como [`render`], com quantos metros cabem em cada vista (o mesmo para
/// vários modelos, para comparar).
pub fn render_span(tris: &[Tri], cell: u32, span: Option<f32>) -> RgbaImage {
    let gap = 2;
    let mut out = RgbaImage::from_pixel(
        cell * 2 + gap,
        cell * 2 + gap,
        Rgba([0x10, 0x12, 0x18, 255]),
    );
    let span = span.unwrap_or_else(|| auto_span(tris));
    for (i, view) in [View::Front, View::Side, View::Top, View::ThreeQuarter]
        .into_iter()
        .enumerate()
    {
        let img = draw_view(tris, view, cell, span);
        let (x, y) = ((i as u32 % 2) * (cell + gap), (i as u32 / 2) * (cell + gap));
        image::imageops::replace(&mut out, &img, i64::from(x), i64::from(y));
    }
    out
}

/// Uma vista só, maior.
pub fn render_one(tris: &[Tri], view: View, cell: u32, span: Option<f32>) -> RgbaImage {
    let span = span.unwrap_or_else(|| auto_span(tris));
    draw_view(tris, view, cell, span)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ModelData, Part};

    #[test]
    fn um_cubo_vermelho_nas_quatro_vistas() {
        let mut m = ModelData::new_default();
        m.parts = vec![Part::cube("caixa", "#E84A4A")];
        let glb = m.to_glb();
        let data = nn_render::d3::ModelData::from_gltf(&glb, &mut |_| None).unwrap();
        let tris = triangles(&data, None);
        assert_eq!(tris.len(), 12);
        let (a, b) = bounds(&tris).unwrap();
        assert!((b - a - Vec3::ONE).length() < 1e-4, "1 m de lado: {a} {b}");
        let img = render(&tris, 100);
        assert_eq!(img.dimensions(), (202, 202));
        // O meio de cada vista é vermelho (com a luz).
        if let Ok(path) = std::env::var("NN_VISTA") {
            img.save(path).unwrap();
        }
        for (x, y) in [(50, 50), (152, 50), (40, 140), (152, 152)] {
            let p = img.get_pixel(x, y).0;
            assert!(
                p[0] > 100 && p[0] > p[1] * 2 && p[1] == p[2],
                "({x}, {y}): {p:?}"
            );
        }
        // O canto é o fundo.
        assert_eq!(*img.get_pixel(1, 1), BG);
    }
}

#[cfg(test)]
mod amostras {
    use super::*;

    /// `NN_VISTAS=pasta cargo test -p nn-projeto amostras`: as vistas de
    /// cada modelo pronto, para olhar.
    #[test]
    fn modelos_prontos() {
        let Ok(dir) = std::env::var("NN_VISTAS") else {
            return;
        };
        for r in crate::model_kits::READY {
            let glb = (r.make)().to_glb();
            let data = nn_render::d3::ModelData::from_gltf(&glb, &mut |_| None).unwrap();
            render(&triangles(&data, None), 240)
                .save(format!("{dir}/{}.png", r.file))
                .unwrap();
        }
    }
}
