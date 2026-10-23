//! Painéis para menus e HUD: retângulos com cantos arredondados e borda, ou
//! uma imagem em 9 fatias (os cantos não esticam, as bordas esticam num
//! sentido só e o meio estica nos dois).

use glam::{UVec2, Vec2};
use nn_core::Color;

use crate::{Rect, Sprite, TextureId};

/// Imagem de um painel em 9 fatias.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelImage {
    pub texture: TextureId,
    /// Região da textura (normalizada), para usar um quadro de spritesheet.
    pub region: Rect,
    /// Margem das fatias, em pixels da imagem (os cantos têm esse tamanho).
    pub slice: f32,
    /// Quantas unidades do mundo (ou pixels da tela) cada pixel da margem
    /// ocupa: com pixel art pequena, 3 ou 4.
    pub scale: f32,
}

/// Um painel retangular.
///
/// ```no_run
/// # fn exemplo(renderer: &mut nn_render::Renderer) {
/// use glam::Vec2;
/// use nn_core::Color;
/// use nn_render::Panel;
/// renderer.draw_panel(
///     &Panel::new(Vec2::ZERO, Vec2::new(300.0, 200.0))
///         .color(Color::hex(0x1E2A3A))
///         .radius(12.0)
///         .border(3.0, Color::WHITE),
/// );
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Panel {
    /// Centro do painel.
    pub center: Vec2,
    pub size: Vec2,
    /// Cor do fundo (ou tingimento da imagem).
    pub color: Color,
    /// Raio dos cantos.
    pub radius: f32,
    /// Largura da borda (0 = sem borda).
    pub border: f32,
    pub border_color: Color,
    /// Com imagem, ela é desenhada em 9 fatias no lugar do retângulo.
    pub image: Option<PanelImage>,
}

impl Panel {
    pub fn new(center: Vec2, size: Vec2) -> Self {
        Self {
            center,
            size,
            color: Color::WHITE,
            radius: 0.0,
            border: 0.0,
            border_color: Color::WHITE,
            image: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border = width;
        self.border_color = color;
        self
    }

    pub fn image(mut self, image: PanelImage) -> Self {
        self.image = Some(image);
        self
    }
}

/// Maior margem gerada em textura (raios maiores esticam essa).
pub(crate) const MAX_ROUND: u32 = 96;

/// Margem (em pixels da textura) das texturas de canto arredondado.
pub(crate) fn round_margin(radius: f32, border: f32) -> u32 {
    (radius.max(border).ceil() as u32).clamp(1, MAX_ROUND)
}

/// Texturas de um retângulo arredondado com raio `radius` e borda `border`
/// (em pixels): o fundo (por dentro da borda) e a borda, sem se sobrepor,
/// para cores com transparência ficarem certas. Cada uma tem `2m+1` pixels
/// de lado, com `m` = [`round_margin`]: os cantos têm `m` pixels e o pixel
/// do meio estica.
pub(crate) fn rounded_pixels(radius: f32, border: f32) -> (UVec2, Vec<u8>, Vec<u8>) {
    let m = round_margin(radius, border);
    // Raio e borda na escala da textura.
    let k = m as f32 / radius.max(border).max(1.0);
    let (r, b) = ((radius * k).min(m as f32), border * k);
    let side = 2 * m + 1;
    let half = side as f32 / 2.0;
    let mut fill = vec![0u8; (side * side * 4) as usize];
    let mut ring = fill.clone();
    for y in 0..side {
        for x in 0..side {
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) - Vec2::splat(half);
            let q = p.abs() - Vec2::splat(half - r);
            let sd = q.max(Vec2::ZERO).length() + q.x.max(q.y).min(0.0) - r;
            let outer = (0.5 - sd).clamp(0.0, 1.0);
            let inner = (0.5 - (sd + b)).clamp(0.0, 1.0);
            let i = ((y * side + x) * 4) as usize;
            let a_fill = if b > 0.0 { inner } else { outer };
            let a_ring = (outer - a_fill).max(0.0);
            fill[i..i + 4].copy_from_slice(&[255, 255, 255, (a_fill * 255.0).round() as u8]);
            ring[i..i + 4].copy_from_slice(&[255, 255, 255, (a_ring * 255.0).round() as u8]);
        }
    }
    (UVec2::splat(side), fill, ring)
}

/// Os 9 pedaços de um retângulo em 9 fatias: para cada um, o centro, o
/// tamanho e a região da textura. `margin` é a margem na textura
/// (normalizada, dentro de `region`) e `dest` a margem desenhada. Com
/// `y_up`, a linha de cima fica em y maior (mundo); senão, menor (tela).
pub(crate) fn nine_slice(
    center: Vec2,
    size: Vec2,
    region: Rect,
    margin: Vec2,
    dest: Vec2,
    y_up: bool,
) -> NineSlice {
    let size = size.abs();
    // Painel menor que as duas margens: elas encolhem.
    let dest = dest.min(size * 0.5).max(Vec2::ZERO);
    let mid = (size - dest * 2.0).max(Vec2::ZERO);
    let cols = [
        (-size.x / 2.0 + dest.x / 2.0, dest.x, region.x, margin.x),
        (0.0, mid.x, region.x + margin.x, region.w - margin.x * 2.0),
        (
            size.x / 2.0 - dest.x / 2.0,
            dest.x,
            region.x + region.w - margin.x,
            margin.x,
        ),
    ];
    let sign = if y_up { 1.0 } else { -1.0 };
    let rows = [
        (
            sign * (size.y / 2.0 - dest.y / 2.0),
            dest.y,
            region.y,
            margin.y,
        ),
        (0.0, mid.y, region.y + margin.y, region.h - margin.y * 2.0),
        (
            -sign * (size.y / 2.0 - dest.y / 2.0),
            dest.y,
            region.y + region.h - margin.y,
            margin.y,
        ),
    ];
    let mut out = NineSlice {
        pieces: [(Vec2::ZERO, Vec2::ZERO, Rect::FULL); 9],
        len: 0,
    };
    for (oy, h, v, vh) in rows {
        for (ox, w, u, uw) in cols {
            if w <= 0.0 || h <= 0.0 {
                continue;
            }
            out.pieces[out.len] = (
                center + Vec2::new(ox, oy),
                Vec2::new(w, h),
                Rect::new(u, v, uw, vh),
            );
            out.len += 1;
        }
    }
    out
}

/// Os pedaços de [`nine_slice`] (até 9, sem alocar).
#[derive(Debug, Clone, Copy)]
pub(crate) struct NineSlice {
    pieces: [(Vec2, Vec2, Rect); 9],
    len: usize,
}

impl NineSlice {
    pub fn as_slice(&self) -> &[(Vec2, Vec2, Rect)] {
        &self.pieces[..self.len]
    }
}

/// Os sprites que desenham o painel, postos em `out`. `rounded` dá as
/// texturas (fundo, borda) para um raio e uma borda; `image_size` o tamanho
/// de uma textura.
pub(crate) fn panel_sprites(
    panel: &Panel,
    y_up: bool,
    rounded: &mut dyn FnMut(f32, f32) -> (TextureId, TextureId, u32),
    image_size: &dyn Fn(TextureId) -> UVec2,
    out: &mut Vec<Sprite>,
) {
    if panel.size.x.abs() < 0.01 || panel.size.y.abs() < 0.01 {
        return;
    }
    let sprite = |tex: TextureId, (c, s, r): (Vec2, Vec2, Rect), color: Color| {
        Sprite::new(tex).at(c).size(s).region(r).color(color)
    };
    if let Some(img) = panel.image {
        let ts = image_size(img.texture).as_vec2().max(Vec2::ONE);
        let margin = (Vec2::splat(img.slice) / ts).min(Vec2::new(img.region.w, img.region.h) * 0.5);
        let dest = Vec2::splat(img.slice * img.scale.max(0.0));
        for &piece in
            nine_slice(panel.center, panel.size, img.region, margin, dest, y_up).as_slice()
        {
            out.push(sprite(img.texture, piece, panel.color));
        }
        return;
    }
    let radius = panel.radius.max(0.0);
    let border = panel.border.max(0.0);
    let (fill, ring, m) = rounded(radius, border);
    let side = (2 * m + 1) as f32;
    let margin = Vec2::splat(m as f32 / side);
    // A margem da textura (m pixels) vale o raio (ou a borda) no desenho.
    let dest = Vec2::splat(radius.max(border).max(1.0));
    let pieces = nine_slice(panel.center, panel.size, Rect::FULL, margin, dest, y_up);
    for &piece in pieces.as_slice() {
        out.push(sprite(fill, piece, panel.color));
    }
    if border > 0.0 && panel.border_color.a > 0.0 {
        for &piece in pieces.as_slice() {
            out.push(sprite(ring, piece, panel.border_color));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fundo_e_borda_nao_se_sobrepoem() {
        let (size, fill, ring) = rounded_pixels(8.0, 2.0);
        assert_eq!(size, UVec2::splat(17));
        let a = |v: &[u8], x: u32, y: u32| v[((y * 17 + x) * 4 + 3) as usize];
        // Centro: só fundo. Meio da borda de cima: só borda. Canto de fora: nada.
        assert_eq!((a(&fill, 8, 8), a(&ring, 8, 8)), (255, 0));
        assert_eq!((a(&fill, 8, 0), a(&ring, 8, 0)), (0, 255));
        assert_eq!((a(&fill, 0, 0), a(&ring, 0, 0)), (0, 0));
        for i in 0..(17 * 17) as usize {
            assert!(fill[i * 4 + 3] as u32 + ring[i * 4 + 3] as u32 <= 256);
        }
    }

    #[test]
    fn nove_fatias_cobrem_o_painel() {
        let pieces = nine_slice(
            Vec2::ZERO,
            Vec2::new(100.0, 40.0),
            Rect::FULL,
            Vec2::splat(0.25),
            Vec2::splat(10.0),
            true,
        );
        let pieces = pieces.as_slice();
        assert_eq!(pieces.len(), 9);
        let area: f32 = pieces.iter().map(|(_, s, _)| s.x * s.y).sum();
        assert!((area - 4000.0).abs() < 0.01);
        // O canto de cima à esquerda (mundo: y para cima) pega o canto da imagem.
        let (c, s, r) = pieces[0];
        assert_eq!((c, s), (Vec2::new(-45.0, 15.0), Vec2::splat(10.0)));
        assert_eq!((r.x, r.y), (0.0, 0.0));
        // Painel menor que as margens: os cantos encolhem e o meio some.
        let small = nine_slice(
            Vec2::ZERO,
            Vec2::splat(8.0),
            Rect::FULL,
            Vec2::splat(0.25),
            Vec2::splat(10.0),
            false,
        );
        assert_eq!(small.as_slice().len(), 4);
    }
}
