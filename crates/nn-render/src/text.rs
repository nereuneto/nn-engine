//! Texto: fontes, layout e atlas de glifos.
//!
//! Cada glifo é rasterizado uma única vez por tamanho e guardado em uma
//! textura-atlas. Depois disso, desenhar texto é desenhar sprites comuns, que
//! entram no mesmo lote dos demais.

use std::collections::HashMap;

use glam::{UVec2, Vec2};
use nn_core::Color;

use crate::texture::{GpuTexture, TextureId, TextureOptions};
use crate::{Rect, RenderError};

/// Fonte padrão: Poppins Regular (SIL Open Font License 1.1).
const DEFAULT_FONT: &[u8] = include_bytes!("../assets/fonts/Poppins-Regular.ttf");

/// Identificador de uma fonte carregada no [`crate::Renderer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontId(pub(crate) u32);

/// Qual ponto da caixa do texto fica em [`Text::position`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Anchor {
    #[default]
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl Anchor {
    /// Fração da caixa (x para a direita, y para baixo).
    fn fraction(self) -> Vec2 {
        match self {
            Self::TopLeft => Vec2::new(0.0, 0.0),
            Self::TopCenter => Vec2::new(0.5, 0.0),
            Self::TopRight => Vec2::new(1.0, 0.0),
            Self::CenterLeft => Vec2::new(0.0, 0.5),
            Self::Center => Vec2::new(0.5, 0.5),
            Self::CenterRight => Vec2::new(1.0, 0.5),
            Self::BottomLeft => Vec2::new(0.0, 1.0),
            Self::BottomCenter => Vec2::new(0.5, 1.0),
            Self::BottomRight => Vec2::new(1.0, 1.0),
        }
    }
}

/// Alinhamento das linhas dentro da caixa, para texto com várias linhas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
}

/// Um trecho de texto a ser desenhado.
///
/// ```
/// # use nn_render::{Anchor, Text};
/// # use nn_core::Color;
/// # use glam::Vec2;
/// let titulo = Text::new("Fim de jogo")
///     .at(Vec2::new(400.0, 300.0))
///     .size(48.0)
///     .color(Color::hex(0xFFD24A))
///     .anchor(Anchor::Center);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Text<'a> {
    pub content: &'a str,
    pub position: Vec2,
    /// Altura da fonte em pixels (unidades do mundo, no espaço do mundo).
    pub size: f32,
    pub color: Color,
    /// `None` usa a fonte padrão.
    pub font: Option<FontId>,
    pub anchor: Anchor,
    pub align: Align,
}

impl<'a> Text<'a> {
    pub fn new(content: &'a str) -> Self {
        Self {
            content,
            position: Vec2::ZERO,
            size: 20.0,
            color: Color::WHITE,
            font: None,
            anchor: Anchor::TopLeft,
            align: Align::Left,
        }
    }

    pub fn at(mut self, position: Vec2) -> Self {
        self.position = position;
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }

    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    pub(crate) fn anchor_fraction(&self) -> Vec2 {
        self.anchor.fraction()
    }
}

/// Glifo posicionado dentro da caixa do texto (pixels, origem no canto
/// superior esquerdo da caixa, Y para baixo).
#[derive(Debug, Clone, Copy)]
pub(crate) struct PlacedGlyph {
    pub key: GlyphKey,
    pub top_left: Vec2,
    pub size: Vec2,
}

#[derive(Debug, Default)]
pub(crate) struct TextLayout {
    pub glyphs: Vec<PlacedGlyph>,
    /// Tamanho da caixa que envolve o texto.
    pub size: Vec2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct GlyphKey {
    font: u32,
    glyph: u16,
    /// Tamanho em pixels, arredondado.
    px: u32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AtlasGlyph {
    pub texture: TextureId,
    pub region: Rect,
}

#[derive(Debug)]
struct AtlasPage {
    texture: TextureId,
    cursor: UVec2,
    shelf_height: u32,
}

/// Fontes carregadas e cache de glifos rasterizados.
#[derive(Debug)]
pub(crate) struct TextSystem {
    fonts: Vec<fontdue::Font>,
    glyphs: HashMap<GlyphKey, Option<AtlasGlyph>>,
    pages: Vec<AtlasPage>,
}

impl TextSystem {
    pub const PAGE_SIZE: u32 = 1024;
    const PADDING: u32 = 1;

    pub fn new() -> Self {
        let mut system = Self {
            fonts: Vec::new(),
            glyphs: HashMap::new(),
            pages: Vec::new(),
        };
        system
            .load_font(DEFAULT_FONT)
            .expect("a fonte padrão embutida é válida");
        system
    }

    pub fn default_font(&self) -> FontId {
        FontId(0)
    }

    pub fn load_font(&mut self, bytes: &[u8]) -> Result<FontId, RenderError> {
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(|e| RenderError::Font(e.to_string()))?;
        self.fonts.push(font);
        Ok(FontId(self.fonts.len() as u32 - 1))
    }

    /// Posiciona os glifos do texto, sem tocar na GPU.
    pub fn layout(&self, text: &Text<'_>) -> TextLayout {
        let font_id = text.font.unwrap_or(self.default_font());
        let font = &self.fonts[font_id.0 as usize];
        let px = text.size.max(1.0).round();

        let (ascent, descent, line_step) = match font.horizontal_line_metrics(px) {
            Some(m) => (m.ascent, m.descent, m.new_line_size),
            None => (px * 0.8, -px * 0.2, px * 1.2),
        };

        let mut lines: Vec<(Vec<PlacedGlyph>, f32)> = Vec::new();
        for (line_index, line) in text.content.split('\n').enumerate() {
            let baseline = ascent + line_index as f32 * line_step;
            let mut pen = 0.0f32;
            let mut previous: Option<u16> = None;
            let mut placed = Vec::new();
            for ch in line.chars() {
                if ch == '\r' {
                    continue;
                }
                let (ch, repeat) = if ch == '\t' { (' ', 4) } else { (ch, 1) };
                let glyph = font.lookup_glyph_index(ch);
                for _ in 0..repeat {
                    if let Some(prev) = previous {
                        pen += font.horizontal_kern_indexed(prev, glyph, px).unwrap_or(0.0);
                    }
                    let m = font.metrics_indexed(glyph, px);
                    if m.width > 0 && m.height > 0 {
                        placed.push(PlacedGlyph {
                            key: GlyphKey {
                                font: font_id.0,
                                glyph,
                                px: px as u32,
                            },
                            top_left: Vec2::new(
                                (pen + m.xmin as f32).round(),
                                (baseline - m.ymin as f32 - m.height as f32).round(),
                            ),
                            size: Vec2::new(m.width as f32, m.height as f32),
                        });
                    }
                    pen += m.advance_width;
                    previous = Some(glyph);
                }
            }
            lines.push((placed, pen));
        }

        let width = lines.iter().map(|(_, w)| *w).fold(0.0, f32::max).ceil();
        let height = (ascent - descent + (lines.len() - 1) as f32 * line_step).ceil();

        let mut glyphs = Vec::new();
        for (placed, line_width) in lines {
            let shift = match text.align {
                Align::Left => 0.0,
                Align::Center => ((width - line_width) / 2.0).round(),
                Align::Right => (width - line_width).round(),
            };
            glyphs.extend(placed.into_iter().map(|mut g| {
                g.top_left.x += shift;
                g
            }));
        }

        TextLayout {
            glyphs,
            size: Vec2::new(width, height),
        }
    }

    /// Garante que o glifo está no atlas e devolve onde ele está.
    /// `None` para glifos que não cabem em uma página.
    pub fn glyph(
        &mut self,
        key: GlyphKey,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        textures: &mut Vec<GpuTexture>,
    ) -> Option<AtlasGlyph> {
        if let Some(cached) = self.glyphs.get(&key) {
            return *cached;
        }

        let font = &self.fonts[key.font as usize];
        let (m, coverage) = font.rasterize_indexed(key.glyph, key.px as f32);
        let size = UVec2::new(m.width as u32, m.height as u32);
        let padded = size + UVec2::splat(Self::PADDING);

        let entry = if padded.x > Self::PAGE_SIZE || padded.y > Self::PAGE_SIZE {
            log::warn!("glifo de {} px grande demais para o atlas", key.px);
            None
        } else {
            let (page, origin) = self.allocate(padded, device, queue, layout, textures);
            let rgba: Vec<u8> = coverage.iter().flat_map(|&a| [255, 255, 255, a]).collect();
            textures[page.0 as usize].write(queue, origin, size, &rgba);
            Some(AtlasGlyph {
                texture: page,
                region: Rect::from_pixels(
                    origin.x,
                    origin.y,
                    size.x,
                    size.y,
                    UVec2::splat(Self::PAGE_SIZE),
                ),
            })
        };
        self.glyphs.insert(key, entry);
        entry
    }

    /// Reserva espaço em uma página do atlas (empacotamento em prateleiras).
    fn allocate(
        &mut self,
        size: UVec2,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        textures: &mut Vec<GpuTexture>,
    ) -> (TextureId, UVec2) {
        if let Some(page) = self.pages.last_mut() {
            if page.cursor.x + size.x > Self::PAGE_SIZE {
                page.cursor = UVec2::new(0, page.cursor.y + page.shelf_height);
                page.shelf_height = 0;
            }
            if page.cursor.y + size.y <= Self::PAGE_SIZE {
                let origin = page.cursor;
                page.cursor.x += size.x;
                page.shelf_height = page.shelf_height.max(size.y);
                return (page.texture, origin);
            }
        }

        // Página nova.
        let texture = TextureId(textures.len() as u32);
        let blank = vec![0u8; (Self::PAGE_SIZE * Self::PAGE_SIZE * 4) as usize];
        textures.push(GpuTexture::from_rgba8(
            device,
            queue,
            layout,
            UVec2::splat(Self::PAGE_SIZE),
            &blank,
            TextureOptions::default(),
            Some("nn glyph atlas"),
        ));
        log::debug!("nova página do atlas de glifos ({})", self.pages.len() + 1);
        self.pages.push(AtlasPage {
            texture,
            cursor: UVec2::new(size.x, 0),
            shelf_height: size.y,
        });
        (texture, UVec2::ZERO)
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn system() -> TextSystem {
        TextSystem::new()
    }

    #[test]
    fn texto_vazio_tem_altura_de_uma_linha() {
        let s = system();
        let l = s.layout(&Text::new("").size(20.0));
        assert!(l.glyphs.is_empty());
        assert_eq!(l.size.x, 0.0);
        assert!(l.size.y >= 20.0 && l.size.y < 32.0, "altura {}", l.size.y);
    }

    #[test]
    fn espacos_nao_geram_glifos_mas_ocupam_largura() {
        let s = system();
        let l = s.layout(&Text::new("a b"));
        assert_eq!(l.glyphs.len(), 2);
        let sem_espaco = s.layout(&Text::new("ab"));
        assert!(l.size.x > sem_espaco.size.x);
    }

    #[test]
    fn largura_cresce_com_o_tamanho() {
        let s = system();
        let pequeno = s.layout(&Text::new("Olá, mundo").size(16.0)).size;
        let grande = s.layout(&Text::new("Olá, mundo").size(32.0)).size;
        assert!((grande.x / pequeno.x - 2.0).abs() < 0.15);
        assert!((grande.y / pequeno.y - 2.0).abs() < 0.15);
    }

    #[test]
    fn quebra_de_linha_aumenta_a_altura() {
        let s = system();
        let uma = s.layout(&Text::new("abc")).size;
        let duas = s.layout(&Text::new("abc\nabc")).size;
        assert_eq!(uma.x, duas.x);
        assert!(duas.y > uma.y * 1.5);
    }

    #[test]
    fn alinhamento_a_direita_encosta_linhas_curtas() {
        let s = system();
        let l = s.layout(&Text::new("iiiiiiii\nW").align(Align::Right));
        let w = l.glyphs.last().unwrap();
        assert!(w.top_left.x + w.size.x > l.size.x - 4.0);
        let centro = s.layout(&Text::new("iiiiiiii\nW").align(Align::Center));
        let wc = centro.glyphs.last().unwrap();
        assert!(wc.top_left.x > 2.0 && wc.top_left.x < w.top_left.x);
    }

    #[test]
    fn glifos_ficam_dentro_da_caixa() {
        let s = system();
        let l = s.layout(&Text::new("Ágil Jogo gy").size(30.0));
        for g in &l.glyphs {
            assert!(g.top_left.y >= -1.0, "glifo acima da caixa");
            assert!(
                g.top_left.y + g.size.y <= l.size.y + 1.0,
                "glifo abaixo da caixa"
            );
        }
    }

    #[test]
    fn fonte_invalida_e_rejeitada() {
        let mut s = system();
        assert!(s.load_font(b"isto nao e uma fonte").is_err());
    }
}
