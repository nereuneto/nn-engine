//! Texto: fontes, layout e atlas de glifos.
//!
//! Cada glifo é rasterizado uma única vez por tamanho e guardado em uma
//! textura-atlas. Depois disso, desenhar texto é desenhar sprites comuns, que
//! entram no mesmo lote dos demais.

use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHasher};

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
    /// A página do atlas (índice em `TextSystem::pages`).
    page: usize,
}

impl AtlasPage {
    /// Lugar para um glifo de `size` (com a margem) nesta página, se couber.
    fn fit(&mut self, size: UVec2) -> Option<UVec2> {
        let side = TextSystem::PAGE_SIZE;
        if self.cursor.x + size.x > side {
            self.cursor = UVec2::new(0, self.cursor.y + self.shelf_height);
            self.shelf_height = 0;
        }
        if self.cursor.y + size.y > side {
            return None;
        }
        let origin = self.cursor;
        self.cursor.x += size.x;
        self.shelf_height = self.shelf_height.max(size.y);
        Some(origin)
    }
}

#[derive(Debug)]
struct AtlasPage {
    texture: TextureId,
    cursor: UVec2,
    shelf_height: u32,
    /// O último quadro em que um glifo desta página foi desenhado.
    used: u64,
}

/// Layouts já feitos, pelo texto, fonte, tamanho e alinhamento. Guarda os do
/// quadro atual e os do anterior: um texto que aparece em todo quadro não é
/// refeito, e um que sumiu sai da memória dois quadros depois.
#[derive(Debug, Default)]
struct LayoutCache {
    current: FxHashMap<u64, Vec<CachedLayout>>,
    previous: FxHashMap<u64, Vec<CachedLayout>>,
}

#[derive(Debug)]
struct CachedLayout {
    content: Box<str>,
    font: u32,
    px: u32,
    align: Align,
    layout: Rc<TextLayout>,
}

impl CachedLayout {
    fn is(&self, content: &str, font: u32, px: u32, align: Align) -> bool {
        self.font == font && self.px == px && self.align == align && &*self.content == content
    }
}

/// Fontes carregadas e cache de glifos rasterizados.
#[derive(Debug)]
pub(crate) struct TextSystem {
    fonts: Vec<fontdue::Font>,
    glyphs: FxHashMap<GlyphKey, Option<AtlasGlyph>>,
    pages: Vec<AtlasPage>,
    /// A página onde os glifos novos entram.
    current: usize,
    layouts: RefCell<LayoutCache>,
    /// Conta os quadros (para saber qual página foi usada há mais tempo).
    frame: std::cell::Cell<u64>,
}

impl TextSystem {
    pub const PAGE_SIZE: u32 = 1024;
    const PADDING: u32 = 1;
    /// Páginas do atlas antes de reaproveitar a menos usada (4 MB cada). Um
    /// jogo com muitos tamanhos de letra passa disso; textos que aparecem
    /// por um tempo e somem deixam de ocupar memória para sempre.
    pub const MAX_PAGES: usize = 4;

    pub fn new() -> Self {
        let mut system = Self {
            fonts: Vec::new(),
            glyphs: FxHashMap::default(),
            pages: Vec::new(),
            current: 0,
            layouts: RefCell::default(),
            frame: std::cell::Cell::new(1),
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

    /// Como [`TextSystem::layout`], mas reaproveita o layout de um texto igual
    /// pedido neste quadro ou no anterior.
    pub fn cached_layout(&self, text: &Text<'_>) -> Rc<TextLayout> {
        let font = text.font.unwrap_or(self.default_font()).0;
        let px = text.size.max(1.0).round() as u32;
        let mut h = FxHasher::default();
        text.content.hash(&mut h);
        (font, px, text.align as u8).hash(&mut h);
        let key = h.finish();
        let mut cache = self.layouts.borrow_mut();
        let found = |list: &Vec<CachedLayout>| {
            list.iter()
                .position(|c| c.is(text.content, font, px, text.align))
        };
        if let Some(list) = cache.current.get(&key)
            && let Some(i) = found(list)
        {
            return list[i].layout.clone();
        }
        let entry = match cache
            .previous
            .get_mut(&key)
            .and_then(|l| found(l).map(|i| (l, i)))
        {
            Some((list, i)) => list.swap_remove(i),
            None => CachedLayout {
                content: text.content.into(),
                font,
                px,
                align: text.align,
                layout: Rc::new(self.layout(text)),
            },
        };
        let layout = entry.layout.clone();
        cache.current.entry(key).or_default().push(entry);
        layout
    }

    /// Fim do quadro: os layouts que não foram usados neste nem no anterior
    /// saem da memória.
    pub fn end_frame(&self) {
        self.frame.set(self.frame.get() + 1);
        let mut cache = self.layouts.borrow_mut();
        let current = std::mem::take(&mut cache.current);
        cache.previous = current;
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
            if let Some(g) = cached
                && let Some(page) = self.pages.get_mut(g.page)
            {
                page.used = self.frame.get();
            }
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
            let texture = self.pages[page].texture;
            self.pages[page].used = self.frame.get();
            let rgba: Vec<u8> = coverage.iter().flat_map(|&a| [255, 255, 255, a]).collect();
            textures[texture.0 as usize].write(queue, origin, size, &rgba);
            Some(AtlasGlyph {
                page,
                texture,
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
    /// Devolve a página e onde, nela. Com [`TextSystem::MAX_PAGES`] cheias,
    /// esvazia a usada há mais tempo (nunca uma usada neste quadro, que pode
    /// estar na fila de desenho).
    fn allocate(
        &mut self,
        size: UVec2,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        textures: &mut Vec<GpuTexture>,
    ) -> (usize, UVec2) {
        if let Some(page) = self.pages.get_mut(self.current)
            && let Some(origin) = page.fit(size)
        {
            return (self.current, origin);
        }
        let blank = || vec![0u8; (Self::PAGE_SIZE * Self::PAGE_SIZE * 4) as usize];
        let now = self.frame.get();
        let oldest = self
            .pages
            .iter()
            .enumerate()
            .filter(|(_, p)| p.used < now)
            .min_by_key(|(_, p)| p.used)
            .map(|(i, _)| i);
        match oldest {
            Some(i) if self.pages.len() >= Self::MAX_PAGES => {
                // Esvazia a página: os glifos dela são refeitos quando voltarem.
                self.glyphs
                    .retain(|_, g| g.as_ref().is_none_or(|g| g.page != i));
                let page = &mut self.pages[i];
                textures[page.texture.0 as usize].write(
                    queue,
                    UVec2::ZERO,
                    UVec2::splat(Self::PAGE_SIZE),
                    &blank(),
                );
                page.cursor = UVec2::ZERO;
                page.shelf_height = 0;
                log::debug!("página {i} do atlas de glifos reaproveitada");
                self.current = i;
            }
            _ => {
                if self.pages.len() >= Self::MAX_PAGES {
                    log::debug!(
                        "atlas de glifos com {} páginas: todas em uso neste quadro",
                        self.pages.len() + 1
                    );
                }
                let texture = TextureId(textures.len() as u32);
                textures.push(GpuTexture::from_rgba8(
                    device,
                    queue,
                    layout,
                    UVec2::splat(Self::PAGE_SIZE),
                    &blank(),
                    TextureOptions::default(),
                    Some("nn glyph atlas"),
                ));
                log::debug!("nova página do atlas de glifos ({})", self.pages.len() + 1);
                self.pages.push(AtlasPage {
                    texture,
                    cursor: UVec2::ZERO,
                    shelf_height: 0,
                    used: now,
                });
                self.current = self.pages.len() - 1;
            }
        }
        let origin = self.pages[self.current]
            .fit(size)
            .expect("cabe numa página vazia (conferido em glyph)");
        (self.current, origin)
    }

    /// A textura é de uma página do atlas?
    pub fn owns(&self, id: TextureId) -> bool {
        self.pages.iter().any(|p| p.texture == id)
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
