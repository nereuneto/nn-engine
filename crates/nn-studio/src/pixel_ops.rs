//! Operações do editor de pixel art, sem interface: pincel, linha,
//! retângulo, círculo, balde, quadros da folha (com as animações
//! acompanhando a numeração) e paletas.

use image::{Rgba, RgbaImage};

use crate::anim_file::AnimFile;

pub type Color = [u8; 4];

pub const CLEAR: Color = [0, 0, 0, 0];

/// Retângulo em pixels da imagem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl PixRect {
    pub fn new(x: u32, y: u32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    pub fn of(img: &RgbaImage) -> Self {
        Self::new(0, 0, img.width(), img.height())
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x as i32
            && y >= self.y as i32
            && x < (self.x + self.w) as i32
            && y < (self.y + self.h) as i32
    }

    /// Retângulo entre dois cantos (inclusive), cortado em `clip`.
    pub fn between(a: (i32, i32), b: (i32, i32), clip: PixRect) -> Option<Self> {
        let x0 = a.0.min(b.0).max(clip.x as i32);
        let y0 = a.1.min(b.1).max(clip.y as i32);
        let x1 = a.0.max(b.0).min((clip.x + clip.w) as i32 - 1);
        let y1 = a.1.max(b.1).min((clip.y + clip.h) as i32 - 1);
        (x1 >= x0 && y1 >= y0).then(|| Self::new(x0 as u32, y0 as u32, (x1 - x0 + 1) as u32, (y1 - y0 + 1) as u32))
    }
}

pub fn get(img: &RgbaImage, x: i32, y: i32) -> Option<Color> {
    (x >= 0 && y >= 0 && (x as u32) < img.width() && (y as u32) < img.height())
        .then(|| img.get_pixel(x as u32, y as u32).0)
}

pub fn put(img: &mut RgbaImage, x: i32, y: i32, c: Color, clip: PixRect) {
    if clip.contains(x, y) && (x as u32) < img.width() && (y as u32) < img.height() {
        img.put_pixel(x as u32, y as u32, Rgba(c));
    }
}

/// Pincel quadrado de `size` pixels centrado em (x, y). Com `mirror`, pinta
/// também o ponto espelhado na horizontal dentro de `clip`.
pub fn stamp(img: &mut RgbaImage, x: i32, y: i32, size: u32, c: Color, clip: PixRect, mirror: bool) {
    let size = size.max(1) as i32;
    let start = -(size - 1) / 2;
    for dy in 0..size {
        for dx in 0..size {
            let (px, py) = (x + start + dx, y + start + dy);
            put(img, px, py, c, clip);
            if mirror {
                put(img, mirror_x(px, clip), py, c, clip);
            }
        }
    }
}

/// X espelhado dentro do retângulo.
pub fn mirror_x(x: i32, clip: PixRect) -> i32 {
    clip.x as i32 + clip.w as i32 - 1 - (x - clip.x as i32)
}

/// Pontos de uma linha (Bresenham), dos dois pontas inclusive.
pub fn line(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
    let (mut x, mut y) = a;
    let dx = (b.0 - a.0).abs();
    let dy = -(b.1 - a.1).abs();
    let sx = if a.0 < b.0 { 1 } else { -1 };
    let sy = if a.1 < b.1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut out = Vec::with_capacity((dx - dy) as usize + 1);
    loop {
        out.push((x, y));
        if x == b.0 && y == b.1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    out
}

/// Pontos de um retângulo entre dois cantos.
pub fn rect(a: (i32, i32), b: (i32, i32), filled: bool) -> Vec<(i32, i32)> {
    let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
    let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
    let mut out = Vec::new();
    for y in y0..=y1 {
        for x in x0..=x1 {
            if filled || x == x0 || x == x1 || y == y0 || y == y1 {
                out.push((x, y));
            }
        }
    }
    out
}

/// Pontos de uma elipse que cabe no retângulo entre os dois cantos.
pub fn ellipse(a: (i32, i32), b: (i32, i32), filled: bool) -> Vec<(i32, i32)> {
    let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
    let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
    let (w, h) = ((x1 - x0 + 1) as f64, (y1 - y0 + 1) as f64);
    let (cx, cy) = (x0 as f64 + w / 2.0, y0 as f64 + h / 2.0);
    let (rx, ry) = (w / 2.0, h / 2.0);
    let inside = |x: i32, y: i32| {
        let nx = (x as f64 + 0.5 - cx) / rx;
        let ny = (y as f64 + 0.5 - cy) / ry;
        nx * nx + ny * ny <= 1.0
    };
    let mut out = Vec::new();
    for y in y0..=y1 {
        for x in x0..=x1 {
            if !inside(x, y) {
                continue;
            }
            let edge = !inside(x - 1, y) || !inside(x + 1, y) || !inside(x, y - 1) || !inside(x, y + 1);
            if filled || edge {
                out.push((x, y));
            }
        }
    }
    out
}

/// Balde: pinta a área da mesma cor ligada a (x, y), sem sair de `clip`.
/// Devolve quantos pixels mudaram.
pub fn flood_fill(img: &mut RgbaImage, x: i32, y: i32, c: Color, clip: PixRect) -> usize {
    if !clip.contains(x, y) {
        return 0;
    }
    let Some(target) = get(img, x, y) else {
        return 0;
    };
    if target == c {
        return 0;
    }
    let mut stack = vec![(x, y)];
    let mut n = 0;
    while let Some((x, y)) = stack.pop() {
        if !clip.contains(x, y) || get(img, x, y) != Some(target) {
            continue;
        }
        img.put_pixel(x as u32, y as u32, Rgba(c));
        n += 1;
        stack.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]);
    }
    n
}

pub fn crop(img: &RgbaImage, r: PixRect) -> RgbaImage {
    image::imageops::crop_imm(img, r.x, r.y, r.w, r.h).to_image()
}

/// Cola `src` com o canto em (x, y); pixels transparentes não apagam o que
/// está embaixo, a não ser com `replace`.
pub fn paste(img: &mut RgbaImage, src: &RgbaImage, x: i32, y: i32, clip: PixRect, replace: bool) {
    for (sx, sy, p) in src.enumerate_pixels() {
        if !replace && p.0[3] == 0 {
            continue;
        }
        put(img, x + sx as i32, y + sy as i32, p.0, clip);
    }
}

pub fn clear_rect(img: &mut RgbaImage, r: PixRect) {
    for y in r.y..(r.y + r.h).min(img.height()) {
        for x in r.x..(r.x + r.w).min(img.width()) {
            img.put_pixel(x, y, Rgba(CLEAR));
        }
    }
}

// ---------------------------------------------------------------------------
// Quadros da folha

/// Colunas e linhas de quadros `fw`×`fh` que cabem na imagem (sem quadros,
/// a imagem inteira é um quadro só).
pub fn grid(img: &RgbaImage, fw: u32, fh: u32) -> (u32, u32) {
    if fw == 0 || fh == 0 || fw > img.width() || fh > img.height() {
        return (1, 1);
    }
    (img.width() / fw, img.height() / fh)
}

/// Retângulo do quadro `i` (a partir de 0, linha por linha).
pub fn cell(img: &RgbaImage, fw: u32, fh: u32, i: u32) -> PixRect {
    let (cols, rows) = grid(img, fw, fh);
    if cols * rows == 1 && (fw == 0 || fh == 0 || fw > img.width() || fh > img.height()) {
        return PixRect::of(img);
    }
    let i = i.min(cols * rows - 1);
    PixRect::new((i % cols) * fw, (i / cols) * fh, fw, fh)
}

pub fn frame_count(img: &RgbaImage, fw: u32, fh: u32) -> u32 {
    let (c, r) = grid(img, fw, fh);
    c * r
}

fn cells(img: &RgbaImage, fw: u32, fh: u32) -> Vec<RgbaImage> {
    (0..frame_count(img, fw, fh))
        .map(|i| crop(img, cell(img, fw, fh, i)))
        .collect()
}

/// Monta a folha com os quadros. Uma linha só continua numa linha só (a
/// imagem cresce para o lado); com várias linhas, ganha linhas novas.
fn rebuild(img: &RgbaImage, fw: u32, fh: u32, list: &[RgbaImage]) -> RgbaImage {
    let (cols, rows) = grid(img, fw, fh);
    let n = list.len().max(1) as u32;
    let (cols, rows) = if rows == 1 { (n, 1) } else { (cols, n.div_ceil(cols).max(rows)) };
    // Sobras à direita e embaixo (que não formam um quadro) ficam.
    let extra_w = img.width() - grid(img, fw, fh).0 * fw;
    let extra_h = img.height() - grid(img, fw, fh).1 * fh;
    let mut out = RgbaImage::new(cols * fw + extra_w, rows * fh + extra_h);
    for (i, c) in list.iter().enumerate() {
        let i = i as u32;
        image::imageops::replace(&mut out, c, ((i % cols) * fw) as i64, ((i / cols) * fh) as i64);
    }
    out
}

/// Muda os números de quadro das animações: `map(n)` dá o novo número do
/// quadro `n` (a partir de 1), ou `None` para tirar da animação.
fn renumber(anim: &mut AnimFile, map: impl Fn(u32) -> Option<u32>) {
    for a in &mut anim.anims {
        a.frames.retain_mut(|f| match map(f.index) {
            Some(n) => {
                f.index = n;
                true
            }
            None => false,
        });
    }
}

/// Põe um quadro novo na posição `at` (a partir de 0): uma cópia de `copy`
/// ou vazio. Os quadros seguintes andam um para a frente (e as animações
/// acompanham).
pub fn insert_frame(img: &mut RgbaImage, anim: &mut AnimFile, at: u32, copy: Option<u32>) {
    let (fw, fh) = (anim.frame_w, anim.frame_h);
    if frame_count(img, fw, fh) <= 1 && grid(img, fw, fh) == (1, 1) && (fw == 0 || fh == 0) {
        // Sem quadros: a imagem inteira vira o primeiro quadro.
        anim.frame_w = img.width();
        anim.frame_h = img.height();
    }
    let (fw, fh) = (anim.frame_w, anim.frame_h);
    let mut list = cells(img, fw, fh);
    let at = at.min(list.len() as u32);
    let new = match copy.and_then(|c| list.get(c as usize)) {
        Some(c) => c.clone(),
        None => RgbaImage::new(fw, fh),
    };
    list.insert(at as usize, new);
    *img = rebuild(img, fw, fh, &list);
    renumber(anim, |n| Some(if n > at { n + 1 } else { n }));
}

/// Tira o quadro `at` (os seguintes voltam um). Se a folha for uma linha só,
/// ela encolhe.
pub fn remove_frame(img: &mut RgbaImage, anim: &mut AnimFile, at: u32) {
    let (fw, fh) = (anim.frame_w, anim.frame_h);
    let mut list = cells(img, fw, fh);
    if list.len() <= 1 || at as usize >= list.len() {
        return;
    }
    list.remove(at as usize);
    let one_row = grid(img, fw, fh).1 == 1;
    if !one_row {
        list.push(RgbaImage::new(fw, fh));
    }
    *img = rebuild(img, fw, fh, &list);
    let gone = at + 1;
    renumber(anim, |n| match n.cmp(&gone) {
        std::cmp::Ordering::Less => Some(n),
        std::cmp::Ordering::Equal => None,
        std::cmp::Ordering::Greater => Some(n - 1),
    });
}

/// Troca dois quadros de lugar (as animações acompanham).
pub fn swap_frames(img: &mut RgbaImage, anim: &mut AnimFile, a: u32, b: u32) {
    let (fw, fh) = (anim.frame_w, anim.frame_h);
    let n = frame_count(img, fw, fh);
    if a >= n || b >= n || a == b {
        return;
    }
    let (ra, rb) = (cell(img, fw, fh, a), cell(img, fw, fh, b));
    let (ca, cb) = (crop(img, ra), crop(img, rb));
    image::imageops::replace(img, &cb, ra.x as i64, ra.y as i64);
    image::imageops::replace(img, &ca, rb.x as i64, rb.y as i64);
    let (fa, fb) = (a + 1, b + 1);
    renumber(anim, |n| {
        Some(if n == fa {
            fb
        } else if n == fb {
            fa
        } else {
            n
        })
    });
}

/// Muda o tamanho da imagem, mantendo o desenho no canto de cima à esquerda.
pub fn resize_canvas(img: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    let mut out = RgbaImage::new(w.max(1), h.max(1));
    image::imageops::replace(&mut out, img, 0, 0);
    out
}

/// Cores usadas na imagem (na ordem em que aparecem), sem as transparentes.
pub fn used_colors(img: &RgbaImage, max: usize) -> Vec<Color> {
    let mut out: Vec<Color> = Vec::new();
    for p in img.pixels() {
        if p.0[3] == 0 || out.contains(&p.0) {
            continue;
        }
        out.push(p.0);
        if out.len() >= max {
            break;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Paletas

/// Paletas prontas (cores de uso livre, de lospec.com).
pub const PALETTES: &[(&str, &str)] = &[
    (
        "Sweetie 16",
        "1a1c2c 5d275d b13e53 ef7d57 ffcd75 a7f070 38b764 257179 29366f 3b5dc9 41a6f6 73eff7 f4f4f4 94b0c2 566c86 333c57",
    ),
    (
        "PICO-8",
        "000000 1d2b53 7e2553 008751 ab5236 5f574f c2c3c7 fff1e8 ff004d ffa300 ffec27 00e436 29adff 83769c ff77a8 ffccaa",
    ),
    (
        "Endesga 32",
        "be4a2f d77643 ead4aa e4a672 b86f50 733e39 3e2731 a22633 e43b44 f77622 feae34 fee761 63c74d 3e8948 265c42 193c3e 124e89 0099db 2ce8f5 ffffff c0cbdc 8b9bb4 5a6988 3a4466 262b44 181425 ff0044 68386c b55088 f6757a e8b796 c28569",
    ),
];

/// Lê cores em hexadecimal (`ff0044`, `#ff0044`, uma por linha ou separadas
/// por espaço), como os arquivos `.hex` do lospec.
pub fn parse_hex_list(text: &str) -> Vec<Color> {
    text.split(|c: char| c.is_whitespace() || c == ',')
        .filter_map(|s| {
            let s = s.trim().trim_start_matches('#');
            let v = |i: usize| u8::from_str_radix(s.get(i..i + 2)?, 16).ok();
            match s.len() {
                6 => Some([v(0)?, v(2)?, v(4)?, 255]),
                8 => Some([v(0)?, v(2)?, v(4)?, v(6)?]),
                _ => None,
            }
        })
        .collect()
}

pub fn to_hex(c: Color) -> String {
    if c[3] == 255 {
        format!("{:02x}{:02x}{:02x}", c[0], c[1], c[2])
    } else {
        format!("{:02x}{:02x}{:02x}{:02x}", c[0], c[1], c[2], c[3])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anim_file::Anim;

    fn img(w: u32, h: u32) -> RgbaImage {
        RgbaImage::new(w, h)
    }

    const R: Color = [255, 0, 0, 255];
    const G: Color = [0, 255, 0, 255];
    const B: Color = [0, 0, 255, 255];

    #[test]
    fn linha_sem_buracos() {
        let l = line((0, 0), (5, 2));
        assert_eq!(l.first(), Some(&(0, 0)));
        assert_eq!(l.last(), Some(&(5, 2)));
        assert_eq!(l.len(), 6);
        for w in l.windows(2) {
            assert!((w[0].0 - w[1].0).abs() <= 1 && (w[0].1 - w[1].1).abs() <= 1);
        }
        assert_eq!(line((3, 3), (3, 3)), vec![(3, 3)]);
    }

    #[test]
    fn retangulo_e_circulo() {
        assert_eq!(rect((0, 0), (2, 2), false).len(), 8);
        assert_eq!(rect((2, 2), (0, 0), true).len(), 9);
        let e = ellipse((0, 0), (6, 6), false);
        assert!(e.contains(&(3, 0)) && e.contains(&(0, 3)) && !e.contains(&(3, 3)));
        assert!(ellipse((0, 0), (6, 6), true).contains(&(3, 3)));
        assert!(!e.contains(&(0, 0)), "canto fora do círculo");
    }

    #[test]
    fn pincel_espelhado_e_cortado() {
        let mut i = img(8, 4);
        let clip = PixRect::new(0, 0, 4, 4);
        stamp(&mut i, 0, 1, 1, R, clip, true);
        assert_eq!(get(&i, 0, 1), Some(R));
        assert_eq!(get(&i, 3, 1), Some(R), "espelho dentro do quadro");
        assert_eq!(get(&i, 7, 1), Some(CLEAR), "não sai do quadro");
        stamp(&mut i, 5, 1, 3, G, clip, false);
        assert_eq!(get(&i, 4, 1), Some(CLEAR));
        let all = PixRect::of(&i);
        stamp(&mut i, 1, 1, 3, G, all, false);
        assert_eq!(get(&i, 0, 0), Some(G));
        assert_eq!(get(&i, 2, 2), Some(G));
    }

    #[test]
    fn balde_fica_no_quadro() {
        let mut i = img(8, 4);
        let n = flood_fill(&mut i, 1, 1, B, PixRect::new(0, 0, 4, 4));
        assert_eq!(n, 16);
        assert_eq!(get(&i, 5, 1), Some(CLEAR));
        let all = PixRect::of(&i);
        assert_eq!(flood_fill(&mut i, 1, 1, B, all), 0);
    }

    fn folha(cores: &[Color]) -> (RgbaImage, AnimFile) {
        let mut i = img(4 * cores.len() as u32, 4);
        for (k, c) in cores.iter().enumerate() {
            let all = PixRect::of(&i);
            stamp(&mut i, k as i32 * 4 + 1, 1, 2, *c, all, false);
        }
        let anim = AnimFile {
            frame_w: 4,
            frame_h: 4,
            anims: vec![Anim::new("a", [1, 2, 3])],
        };
        (i, anim)
    }

    fn cor(i: &RgbaImage, frame: u32) -> Color {
        get(i, frame as i32 * 4 + 1, 1).unwrap()
    }

    #[test]
    fn duplicar_e_excluir_quadro_renumeram_as_animacoes() {
        let (mut i, mut a) = folha(&[R, G, B]);
        insert_frame(&mut i, &mut a, 1, Some(0));
        assert_eq!(i.width(), 16);
        assert_eq!([cor(&i, 0), cor(&i, 1), cor(&i, 2), cor(&i, 3)], [R, R, G, B]);
        let idx: Vec<u32> = a.anims[0].frames.iter().map(|f| f.index).collect();
        assert_eq!(idx, [1, 3, 4]);
        remove_frame(&mut i, &mut a, 2);
        assert_eq!(i.width(), 12);
        assert_eq!([cor(&i, 0), cor(&i, 1), cor(&i, 2)], [R, R, B]);
        let idx: Vec<u32> = a.anims[0].frames.iter().map(|f| f.index).collect();
        assert_eq!(idx, [1, 3], "o quadro excluído sai da animação");
        swap_frames(&mut i, &mut a, 0, 2);
        assert_eq!([cor(&i, 0), cor(&i, 2)], [B, R]);
        let idx: Vec<u32> = a.anims[0].frames.iter().map(|f| f.index).collect();
        assert_eq!(idx, [3, 1]);
    }

    #[test]
    fn folha_de_varias_linhas_ganha_linha() {
        let mut i = img(8, 8);
        let mut a = AnimFile {
            frame_w: 4,
            frame_h: 4,
            anims: vec![],
        };
        insert_frame(&mut i, &mut a, 4, None);
        assert_eq!((i.width(), i.height()), (8, 12));
        assert_eq!(frame_count(&i, 4, 4), 6);
    }

    #[test]
    fn imagem_sem_quadros_vira_folha_ao_ganhar_quadro() {
        let mut i = img(6, 5);
        let mut a = AnimFile::default();
        insert_frame(&mut i, &mut a, 1, Some(0));
        assert_eq!((a.frame_w, a.frame_h), (6, 5));
        assert_eq!((i.width(), i.height()), (12, 5));
    }

    #[test]
    fn cores_em_hex() {
        assert_eq!(parse_hex_list("ff0000\n#00ff0080, zzz 12"), vec![R, [0, 255, 0, 128]]);
        assert_eq!(to_hex([1, 2, 3, 255]), "010203");
        for (_, p) in PALETTES {
            assert!(parse_hex_list(p).len() >= 16);
        }
    }
}
