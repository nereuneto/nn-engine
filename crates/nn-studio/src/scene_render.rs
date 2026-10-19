//! Desenho da cena no editor com o mesmo renderer da engine, na GPU do
//! próprio editor: o que aparece na aba Cena é o que aparece no jogo.
//!
//! Também mede objetos (tamanho das imagens, dos mapas e dos textos) para a
//! seleção e os gizmos, mesmo sem GPU (nos testes).

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};
use glam::{UVec2, Vec2};
use nn_core::Color;
use nn_render::{
    Anchor, Blend, Camera2D, EmitArea, FontId, Panel, PanelImage, ParticleConfig, ParticleEmitter,
    Rect as UvRect, Renderer, Sprite, Text, TextureId, TextureOptions,
};

use crate::scene_data::{Component, Object, SceneData, Transform};

/// Lê um modelo 3D do projeto (com o .bin e as imagens ao lado, se for
/// .gltf).
pub fn read_model(root: &Path, rel: &str) -> Result<nn_render::d3::ModelData, String> {
    let path = root.join(rel);
    let bytes = std::fs::read(&path).map_err(|e| format!("{rel}: {e}"))?;
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    nn_render::d3::ModelData::from_gltf(&bytes, &mut |uri| std::fs::read(dir.join(uri)).ok())
        .map_err(|e| format!("{rel}: {e}"))
}

/// A caixa que envolve um objeto 3D no espaço dele (sem a transformação):
/// o modelo, a forma ou, para a câmera e objetos vazios, um cubinho.
pub fn bounds3(o: &Object, root: &Path, sizes: &mut Sizes) -> (glam::Vec3, glam::Vec3) {
    use glam::Vec3;
    let mut out: Option<(Vec3, Vec3)> = None;
    let mut add = |b: (Vec3, Vec3)| {
        out = Some(match out {
            Some((a, c)) => (a.min(b.0), c.max(b.1)),
            None => b,
        });
    };
    for c in &o.components {
        match c.kind.as_str() {
            "model" => {
                if let Some(b) = sizes.model(root, c.str("file")) {
                    add(b);
                }
            }
            "shape3d" => add(shape3_bounds(c.str("shape"))),
            _ => {}
        }
    }
    out.unwrap_or((Vec3::splat(-0.25), Vec3::splat(0.25)))
}

/// A caixa de cada forma pronta (as de `nn_render::d3::shapes`).
pub fn shape3_bounds(shape: &str) -> (glam::Vec3, glam::Vec3) {
    use glam::Vec3;
    match shape {
        // O plano é fino; um pouco de altura para dar para clicar nele.
        "plane" => (Vec3::new(-0.5, -0.01, -0.5), Vec3::new(0.5, 0.01, 0.5)),
        "capsule" => (Vec3::new(-0.25, -0.5, -0.25), Vec3::new(0.25, 0.5, 0.25)),
        _ => (Vec3::splat(-0.5), Vec3::splat(0.5)),
    }
}

/// Cor "#RRGGBB" ou "#RRGGBBAA" (como no Lua). Inválida vira branco.
pub fn parse_color(s: &str) -> Color {
    let hex = s.trim().trim_start_matches('#');
    let Ok(v) = u32::from_str_radix(hex, 16) else {
        return Color::WHITE;
    };
    match hex.len() {
        6 => Color::hex(v),
        8 => Color::hex(v >> 8).with_alpha((v & 0xFF) as f32 / 255.0),
        _ => Color::WHITE,
    }
}

/// Mesma cor para o egui (sRGB).
pub fn egui_color(s: &str) -> egui::Color32 {
    let hex = s.trim().trim_start_matches('#');
    let v = u32::from_str_radix(hex, 16).unwrap_or(0xFFFFFF);
    match hex.len() {
        8 => egui::Color32::from_rgba_unmultiplied(
            (v >> 24) as u8,
            (v >> 16) as u8,
            (v >> 8) as u8,
            v as u8,
        ),
        _ => egui::Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8),
    }
}

pub fn anchor_of(name: &str) -> Anchor {
    match name {
        "topleft" => Anchor::TopLeft,
        "top" => Anchor::TopCenter,
        "topright" => Anchor::TopRight,
        "left" => Anchor::CenterLeft,
        "right" => Anchor::CenterRight,
        "bottomleft" => Anchor::BottomLeft,
        "bottom" => Anchor::BottomCenter,
        "bottomright" => Anchor::BottomRight,
        _ => Anchor::Center,
    }
}

/// Tamanhos de imagens e mapas (lidos do disco, sem GPU) para medir objetos.
#[derive(Debug, Default)]
pub struct Sizes {
    images: HashMap<String, Option<(f32, f32)>>,
    maps: HashMap<(String, u32), Option<(f32, f32)>>,
    /// Animações (`.anim`) das imagens: nomes e tamanho do quadro.
    pub anims: crate::anim_file::AnimCache,
    /// Tamanho (em pixels, sem escala) dos mapas abertos no editor.
    pub map_px: HashMap<String, (f32, f32)>,
    /// Modelo 3D → caixa que envolve (mínimo, máximo) e os nomes das
    /// animações.
    models: HashMap<String, Option<ModelInfo>>,
}

/// O que o editor sabe de um modelo 3D: a caixa e os nomes das animações.
type ModelInfo = ((glam::Vec3, glam::Vec3), Vec<String>);

impl Sizes {
    pub fn invalidate(&mut self, rel: &str) {
        self.images.remove(rel);
        self.models.remove(rel);
        self.maps.retain(|(r, _), _| r != rel);
    }

    pub fn clear(&mut self) {
        self.images.clear();
        self.models.clear();
        self.maps.clear();
    }

    /// A caixa que envolve um modelo 3D (lido do disco, sem GPU).
    pub fn model(&mut self, root: &Path, rel: &str) -> Option<(glam::Vec3, glam::Vec3)> {
        if rel.is_empty() {
            return None;
        }
        self.model_info(root, rel).map(|(b, _)| *b)
    }

    /// As animações de um modelo 3D (vazio se não tiver ou não abrir).
    pub fn model_anims(&mut self, root: &Path, rel: &str) -> Vec<String> {
        self.model_info(root, rel)
            .map(|(_, a)| a.clone())
            .unwrap_or_default()
    }

    fn model_info(&mut self, root: &Path, rel: &str) -> Option<&ModelInfo> {
        if rel.is_empty() {
            return None;
        }
        if !self.models.contains_key(rel) {
            let info = read_model(root, rel).ok().map(|m| {
                let anims = m
                    .skeleton
                    .as_ref()
                    .map(|s| s.animations.iter().map(|a| a.name.clone()).collect())
                    .unwrap_or_default();
                (m.bounds(), anims)
            });
            self.models.insert(rel.to_string(), info);
        }
        self.models.get(rel).and_then(|i| i.as_ref())
    }

    pub fn image(&mut self, root: &Path, rel: &str) -> Option<(f32, f32)> {
        if rel.is_empty() {
            return None;
        }
        *self.images.entry(rel.to_string()).or_insert_with(|| {
            image::image_dimensions(root.join(rel))
                .ok()
                .map(|(w, h)| (w as f32, h as f32))
        })
    }

    pub fn map(&mut self, root: &Path, rel: &str, scale: f32) -> Option<(f32, f32)> {
        if rel.is_empty() {
            return None;
        }
        if let Some((w, h)) = self.map_px.get(rel) {
            return Some((w * scale.max(0.01), h * scale.max(0.01)));
        }
        *self
            .maps
            .entry((rel.to_string(), scale.to_bits()))
            .or_insert_with(|| {
                nn_tilemap::load_tiled(Path::new(rel), nn_tilemap::read_from(root), scale.max(0.01))
                    .ok()
                    .map(|m| {
                        let s = m.world_size();
                        (s.x, s.y)
                    })
            })
    }
}

/// Retângulo local (sem escala) de um objeto, relativo à posição dele.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalRect {
    pub min: (f64, f64),
    pub max: (f64, f64),
}

impl LocalRect {
    fn centered(w: f64, h: f64) -> Self {
        Self {
            min: (-w / 2.0, -h / 2.0),
            max: (w / 2.0, h / 2.0),
        }
    }

    fn union(self, o: Self) -> Self {
        Self {
            min: (self.min.0.min(o.min.0), self.min.1.min(o.min.1)),
            max: (self.max.0.max(o.max.0), self.max.1.max(o.max.1)),
        }
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.min.0 && x <= self.max.0 && y >= self.min.1 && y <= self.max.1
    }

    /// Os quatro cantos (sentido anti-horário a partir de baixo à esquerda).
    pub fn corners(&self) -> [(f64, f64); 4] {
        [
            (self.min.0, self.min.1),
            (self.max.0, self.min.1),
            (self.max.0, self.max.1),
            (self.min.0, self.max.1),
        ]
    }
}

/// Tamanho do quadro de uma imagem: o do componente ou, sem ele, o do
/// `.anim` da imagem (como a engine faz).
pub fn sprite_frame(c: &Component, anim: impl FnOnce(&str) -> Option<(u32, u32)>) -> (f64, f64) {
    let (fw, fh) = (c.num("frame_w"), c.num("frame_h"));
    if fw > 0.0 && fh > 0.0 {
        return (fw, fh);
    }
    match anim(c.str("image")) {
        Some((w, h)) if w > 0 && h > 0 => (w as f64, h as f64),
        _ => (fw, fh),
    }
}

/// Largura aproximada de um texto (sem GPU): 0,55 do tamanho por letra.
pub fn approx_text_width(text: &str, size: f64) -> f64 {
    let chars = text.lines().map(|l| l.chars().count()).max().unwrap_or(0);
    size * 0.55 * chars as f64
}

/// Número de uma variável do jogo (o valor inicial do config.lua).
pub fn var_num(name: &str) -> Option<f64> {
    use crate::scene_data::Prop;
    match crate::scene_data::var_value(name)? {
        Prop::Num(n) => Some(n),
        Prop::Bool(b) => Some(if b { 1.0 } else { 0.0 }),
        Prop::Str(s) => s.trim().parse().ok(),
    }
}

/// Ícones: quantos cheios e quantos ao todo (como `icons_count` do cena.lua).
pub fn icons_count(c: &Component) -> (usize, usize) {
    let mut n = c.num("count");
    if !c.str("var").is_empty() {
        n = var_num(c.str("var")).unwrap_or(0.0);
    }
    let n = n.max(0.0).floor() as usize;
    (n, n.max(c.num("max").max(0.0).floor() as usize))
}

/// Tamanho (sem escala) de um componente de interface, como o
/// `component_size` do cena.lua (os textos são medidos por aproximação).
pub fn ui_size(c: &Component) -> Option<(f64, f64)> {
    Some(match c.kind.as_str() {
        "panel" | "button" | "bar" | "textfield" => (c.num("w"), c.num("h")),
        "slider" => (c.num("w") + c.num("knob"), c.num("h").max(c.num("knob"))),
        "checkbox" => {
            let (box_, size) = (c.num("box"), c.num("size"));
            let tw = if c.str("text").is_empty() {
                0.0
            } else {
                approx_text_width(c.str("text"), size) + box_ * 0.4
            };
            (box_ + tw, box_.max(size * 1.2))
        }
        "icons" => {
            let (_, k) = icons_count(c);
            let k = k.max(1) as f64;
            (
                k * c.num("size") + (k - 1.0) * c.num("spacing"),
                c.num("size"),
            )
        }
        _ => return None,
    })
}

/// Tamanho de um objeto sem nada visível (ícone), em pixels da tela.
pub const ICON_SIZE: f64 = 22.0;

/// Área ocupada pelo objeto (para clicar, selecionar e escalar). `icon` é o
/// tamanho no mundo usado quando o objeto não tem nada visível.
pub fn local_bounds(o: &Object, root: &Path, sizes: &mut Sizes, icon: f64) -> (LocalRect, bool) {
    let mut rect: Option<LocalRect> = None;
    let mut add = |r: LocalRect| {
        rect = Some(match rect {
            Some(a) => a.union(r),
            None => r,
        })
    };
    for c in &o.components {
        match c.kind.as_str() {
            "sprite" => {
                let (fw, fh) = sprite_frame(c, |img| {
                    sizes.anims.get(root, img).map(|a| (a.frame_w, a.frame_h))
                });
                let natural = if fw > 0.0 && fh > 0.0 {
                    Some((fw, fh))
                } else {
                    sizes
                        .image(root, c.str("image"))
                        .map(|(w, h)| (w as f64, h as f64))
                };
                let (nw, nh) = natural.unwrap_or((32.0, 32.0));
                let w = if c.num("w") > 0.0 { c.num("w") } else { nw };
                let h = if c.num("h") > 0.0 { c.num("h") } else { nh };
                add(LocalRect::centered(w, h));
            }
            "shape" => {
                if c.str("shape") == "circle" {
                    let r = c.num("radius");
                    add(LocalRect::centered(2.0 * r, 2.0 * r));
                } else {
                    add(LocalRect::centered(c.num("w"), c.num("h")));
                }
            }
            "body" => {
                if c.str("shape") == "circle" {
                    let r = c.num("radius");
                    add(LocalRect::centered(2.0 * r, 2.0 * r));
                } else {
                    add(LocalRect::centered(c.num("w"), c.num("h")));
                }
            }
            "text" => {
                let size = c.num("size");
                let chars = c
                    .str("text")
                    .lines()
                    .map(|l| l.chars().count())
                    .max()
                    .unwrap_or(1)
                    .max(1);
                let lines = c.str("text").lines().count().max(1) as f64;
                let (w, h) = (size * 0.55 * chars as f64, size * 1.2 * lines);
                // Âncora: qual ponto do texto fica na posição do objeto.
                let a = c.str("anchor");
                let fx = if a.ends_with("left") || a == "left" {
                    0.0
                } else if a.ends_with("right") || a == "right" {
                    1.0
                } else {
                    0.5
                };
                let fy = if a.starts_with("top") {
                    1.0
                } else if a.starts_with("bottom") {
                    0.0
                } else {
                    0.5
                };
                add(LocalRect {
                    min: (-w * fx, -h * (1.0 - fy)),
                    max: (w * (1.0 - fx), h * fy),
                });
            }
            "map" => {
                if let Some((w, h)) = sizes.map(root, c.str("map"), c.num("scale") as f32) {
                    add(LocalRect {
                        min: (0.0, 0.0),
                        max: (w as f64, h as f64),
                    });
                }
            }
            "icons" => {
                if let Some((w, h)) = ui_size(c) {
                    let x0 = match c.str("align") {
                        "left" => 0.0,
                        "right" => -w,
                        _ => -w / 2.0,
                    };
                    add(LocalRect {
                        min: (x0, -h / 2.0),
                        max: (x0 + w, h / 2.0),
                    });
                }
            }
            _ => {
                if let Some((w, h)) = ui_size(c) {
                    add(LocalRect::centered(w, h));
                }
            }
        }
    }
    match rect {
        Some(r) if r.max.0 > r.min.0 && r.max.1 > r.min.1 => (r, false),
        _ => (LocalRect::centered(icon, icon), true),
    }
}

/// Grupo: põe os filhos visíveis em coluna, em linha ou em grade em volta
/// do objeto, como o `arrange` do cena.lua. Devolve as posições (id, x, y).
pub fn arrange(o: &Object, root: &Path, sizes: &mut Sizes) -> Vec<(u32, f64, f64)> {
    let Some(l) = o.component("layout") else {
        return Vec::new();
    };
    let mode = l.str("arrange");
    let gap = l.num("spacing");
    let align = l.str("align");
    let items: Vec<(u32, f64, f64)> = o
        .children
        .iter()
        .filter(|ch| ch.visible)
        .map(|ch| {
            let (r, icon) = local_bounds(ch, root, sizes, 0.0);
            let (w, h) = if icon {
                (0.0, 0.0)
            } else {
                (r.max.0 - r.min.0, r.max.1 - r.min.1)
            };
            (ch.id, w * ch.scale_x.abs(), h * ch.scale_y.abs())
        })
        .collect();
    if items.is_empty() {
        return Vec::new();
    }
    let cross = |size: f64, total: f64| match align {
        "start" => -total / 2.0 + size / 2.0,
        "end" => total / 2.0 - size / 2.0,
        _ => 0.0,
    };
    let n = items.len() as f64;
    let mut out = Vec::new();
    match mode {
        "vertical" | "horizontal" => {
            let vertical = mode == "vertical";
            let along = |it: &(u32, f64, f64)| if vertical { it.2 } else { it.1 };
            let across = |it: &(u32, f64, f64)| if vertical { it.1 } else { it.2 };
            let total: f64 = items.iter().map(along).sum::<f64>() + gap * (n - 1.0);
            let widest = items.iter().map(across).fold(0.0, f64::max);
            let mut pos = total / 2.0;
            for it in &items {
                let size = along(it);
                let a = pos - size / 2.0;
                let c = cross(across(it), widest);
                out.push(if vertical {
                    (it.0, c, a)
                } else {
                    (it.0, -a, c)
                });
                pos -= size + gap;
            }
        }
        "grid" => {
            let cols = l.num("columns").floor().max(1.0) as usize;
            let cw = items.iter().map(|i| i.1).fold(0.0, f64::max);
            let ch = items.iter().map(|i| i.2).fold(0.0, f64::max);
            let rows = items.len().div_ceil(cols) as f64;
            let k = cols.min(items.len()) as f64;
            let w = k * cw + (k - 1.0) * gap;
            let h = rows * ch + (rows - 1.0) * gap;
            for (i, it) in items.iter().enumerate() {
                let (col, row) = ((i % cols) as f64, (i / cols) as f64);
                out.push((
                    it.0,
                    -w / 2.0 + cw / 2.0 + col * (cw + gap),
                    h / 2.0 - ch / 2.0 - row * (ch + gap),
                ));
            }
        }
        _ => {}
    }
    out
}

/// Aplica os Grupos da cena toda (os filhos vão para o lugar).
pub fn arrange_all(data: &mut SceneData, root: &Path, sizes: &mut Sizes) {
    let mut moves = Vec::new();
    crate::scene_data::visit(&data.objects, &mut |o| {
        moves.extend(arrange(o, root, sizes))
    });
    for (id, x, y) in moves {
        if let Some(o) = data.find_mut(id)
            && (o.x != x || o.y != y)
        {
            o.x = x;
            o.y = y;
        }
    }
}

/// Um objeto pronto para desenhar: transformação no mundo e ordem.
#[derive(Debug, Clone, Copy)]
pub struct Placed<'a> {
    pub object: &'a Object,
    pub world: Transform,
    /// O objeto da cena que se seleciona clicando aqui: ele mesmo, ou a
    /// instância do prefab de onde este filho vem.
    pub owner: u32,
}

impl Placed<'_> {
    /// É um objeto da própria cena (não um filho vindo de um prefab)?
    pub fn is_own(&self) -> bool {
        self.owner == self.object.id
    }
}

/// Objetos visíveis em ordem de desenho (camada, depois a ordem da árvore),
/// como a engine faz. Inclui os filhos que vêm dos prefabs.
pub fn draw_order(scene: &SceneData) -> Vec<Placed<'_>> {
    fn walk<'a>(
        scene: &SceneData,
        list: &'a [Object],
        parent: Option<Transform>,
        owner: Option<u32>,
        out: &mut Vec<Placed<'a>>,
        depth: usize,
    ) {
        if depth > 16 {
            return;
        }
        for o in list {
            if !o.visible {
                continue;
            }
            let local = Transform::of(o);
            let world = match parent {
                Some(p) => p.apply(local),
                None => scene.root_place(o, local),
            };
            let me = owner.unwrap_or(o.id);
            out.push(Placed {
                object: o,
                world,
                owner: me,
            });
            walk(
                scene,
                o.prefab_children(),
                Some(world),
                Some(me),
                out,
                depth + 1,
            );
            walk(scene, &o.children, Some(world), owner, out, depth + 1);
        }
    }
    let mut out = Vec::new();
    walk(scene, &scene.objects, None, None, &mut out, 0);
    out.sort_by_key(|p| p.object.layer);
    out
}

struct Target {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: UVec2,
    id: egui::TextureId,
}

/// Desenha cenas com o renderer da engine numa textura que o egui mostra.
/// Como desenhar a cena: câmera, tamanho em pixels, fundo e o relógio das
/// animações.
#[derive(Debug, Clone)]
pub struct RenderView {
    pub camera: Camera2D,
    pub size: UVec2,
    pub background: Color,
    pub time: f32,
    /// As partículas animam (senão, ficam paradas).
    pub particles: bool,
    /// Objetos com partículas para soltar agora (botão 💥 Soltar).
    pub bursts: Vec<u32>,
}

/// As opções de partículas de um componente, como a engine monta
/// (`M.particle_options` do cena.lua). `image` é a textura e a região.
pub fn particle_config(c: &Component, image: Option<(TextureId, UvRect)>) -> ParticleConfig {
    let num = |k: &str| c.num(k) as f32;
    let burst = c.str("mode") == "explosao";
    let mut colors = vec![parse_color(c.str("color_start"))];
    if !c.str("color_mid").is_empty() {
        colors.push(parse_color(c.str("color_mid")));
    }
    colors.push(parse_color(c.str("color_end")));
    let life = num("lifetime");
    let spin = num("spin").to_radians();
    ParticleConfig {
        texture: image.map(|i| i.0),
        region: image.map_or(UvRect::FULL, |i| i.1),
        rate: if burst { 0.0 } else { num("rate") },
        lifetime: (life * (1.0 - num("lifetime_var").clamp(0.0, 1.0)), life),
        speed: (num("speed_min"), num("speed_max")),
        direction: num("direction").to_radians(),
        spread: num("spread").to_radians(),
        gravity: Vec2::new(0.0, num("gravity")),
        drag: num("drag"),
        size: (num("size_start"), num("size_end")),
        size_variation: num("size_var"),
        colors,
        spin: (-spin, spin),
        area: match c.str("area") {
            "retangulo" => EmitArea::Rect(Vec2::new(num("area_w"), num("area_h")) / 2.0),
            "circulo" => EmitArea::Circle(num("area_radius")),
            _ => EmitArea::Point,
        },
        blend: if c.str("blend") == "add" {
            Blend::Add
        } else {
            Blend::Alpha
        },
        max: (c.num("max").max(1.0) as usize).min(20_000),
        follow: c.bool("follow"),
    }
}

/// As opções de um componente Partículas numa cena 3D, como a engine monta
/// (`M.particle_options3d`): em metros, saindo para `up`.
pub fn particle_config3(
    c: &Component,
    image: Option<(TextureId, UvRect)>,
    up: glam::Vec3,
) -> nn_render::d3::ParticleConfig3 {
    use nn_render::d3::{EmitArea3, ParticleConfig3};
    let base = particle_config(c, image);
    let num = |k: &str| c.num(k) as f32;
    ParticleConfig3 {
        texture: base.texture,
        region: base.region,
        rate: base.rate,
        lifetime: base.lifetime,
        speed: base.speed,
        direction: up,
        spread: num("spread"),
        gravity: glam::Vec3::new(0.0, num("gravity"), 0.0),
        drag: base.drag,
        size: base.size,
        size_variation: base.size_variation,
        colors: base.colors,
        spin: (-num("spin"), num("spin")),
        area: match c.str("area") {
            "retangulo" => EmitArea3::Box(glam::Vec3::new(num("area_w"), 0.0, num("area_h")) / 2.0),
            "circulo" => EmitArea3::Disc(num("area_radius")),
            _ => EmitArea3::Point,
        },
        blend: base.blend,
        max: base.max,
        follow: base.follow,
    }
}

/// Prévia de um emissor 3D na cena do editor.
struct Preview3 {
    props: String,
    emitter: nn_render::d3::ParticleEmitter3,
    time: f32,
    next: f32,
}

/// Prévia de um emissor na cena do editor.
struct Preview {
    /// Os campos do componente quando o emissor foi feito.
    props: String,
    emitter: ParticleEmitter,
    /// Último relógio visto.
    time: f32,
    /// Explosão: quando soltar de novo (a prévia repete).
    next: f32,
}

pub struct SceneRenderer {
    device: wgpu::Device,
    egui: Arc<egui::mutex::RwLock<egui_wgpu::Renderer>>,
    renderer: Renderer,
    /// Imagem → textura (com e sem pixel art; `None` se não abriu).
    textures: HashMap<String, [Option<Option<TextureId>>; 2]>,
    maps: HashMap<(String, u32), Option<nn_tilemap::TileMap>>,
    targets: HashMap<String, Target>,
    anims: crate::anim_file::AnimCache,
    /// Mapas abertos no editor (ainda não salvos): mapa → (versão, arquivos).
    map_sources: HashMap<String, (u64, HashMap<String, String>)>,
    /// Fontes dos textos e botões (arquivo → fonte; `None` se não abriu).
    fonts: HashMap<String, Option<FontId>>,
    /// Partículas animando na cena: (aba, objeto, componente) → emissor.
    previews: HashMap<(String, u32, usize), Preview>,
    /// O mesmo, nas cenas 3D.
    previews3: HashMap<(String, u32, usize), Preview3>,
    /// 3D: as formas prontas já na GPU.
    meshes3: HashMap<&'static str, nn_render::d3::MeshId>,
    /// 3D: modelos (arquivo, achatado) → modelo na GPU (`None` se não abriu).
    models3: HashMap<(String, bool), Option<nn_render::d3::Model>>,
}

/// Como desenhar uma cena 3D: a câmera do editor, o tamanho e o fundo.
#[derive(Debug, Clone)]
pub struct View3 {
    pub camera: nn_render::d3::Camera3D,
    pub size: UVec2,
    /// Fundo quando a cena não tem céu.
    pub background: Color,
    /// Passo da grade no chão (0 = sem grade).
    pub grid: f32,
    /// Segundos desde que a vista abriu (para a prévia das animações e das
    /// partículas).
    pub time: f32,
    /// Objetos com partículas para soltar agora (botão 💥 Soltar).
    pub bursts: Vec<u32>,
}

/// A animação que a prévia mostra num objeto: a da Animação 3D, ou "parado"
/// num Jogador 3D. Nome e velocidade.
pub fn preview_anim3d(o: &Object) -> Option<(String, f32)> {
    if let Some(a) = o.component("anim3d") {
        let name = a.str("animation");
        return (!name.is_empty()).then(|| (name.to_string(), a.num("speed") as f32));
    }
    o.components
        .iter()
        .find(|c| c.is_behavior() && c.str("script") == "nn:jogador3d")
        .filter(|c| c.get("animar").is_none_or(|v| v.num() != 0.0))
        .map(|c| {
            let name = c.str("anim_parado");
            (
                if name.is_empty() { "parado" } else { name }.to_string(),
                1.0,
            )
        })
}

impl std::fmt::Debug for SceneRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SceneRenderer").finish_non_exhaustive()
    }
}

/// Formato da textura onde a cena é desenhada.
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
/// Como o egui enxerga a mesma textura.
const EGUI_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

impl SceneRenderer {
    pub fn new(state: &egui_wgpu::RenderState) -> Self {
        Self {
            device: state.device.clone(),
            egui: state.renderer.clone(),
            renderer: Renderer::from_device(&state.device, &state.queue, FORMAT),
            textures: HashMap::new(),
            maps: HashMap::new(),
            targets: HashMap::new(),
            anims: Default::default(),
            map_sources: HashMap::new(),
            fonts: HashMap::new(),
            previews: HashMap::new(),
            previews3: HashMap::new(),
            meshes3: HashMap::new(),
            models3: HashMap::new(),
        }
    }

    fn mesh3(&mut self, shape: &str) -> nn_render::d3::MeshId {
        use nn_render::d3::shapes;
        let (name, make): (&'static str, fn() -> nn_render::d3::Mesh) = match shape {
            "sphere" => ("sphere", || shapes::sphere(2)),
            "cylinder" => ("cylinder", || shapes::cylinder(16)),
            "cone" => ("cone", || shapes::cone(16)),
            "capsule" => ("capsule", || shapes::capsule(16)),
            "plane" => ("plane", shapes::plane),
            _ => ("cube", shapes::cube),
        };
        if let Some(id) = self.meshes3.get(name) {
            return *id;
        }
        let id = self.renderer.create_mesh(&make());
        self.meshes3.insert(name, id);
        id
    }

    fn model3(&mut self, root: &Path, rel: &str, flat: bool) -> Option<nn_render::d3::Model> {
        if rel.is_empty() {
            return None;
        }
        let key = (rel.to_string(), flat);
        if let Some(m) = self.models3.get(&key) {
            return m.clone();
        }
        let m = read_model(root, rel).ok().map(|mut data| {
            if flat {
                data.flatten();
            }
            self.renderer.create_model(&data)
        });
        self.models3.insert(key, m.clone());
        m
    }

    /// Desenha uma cena 3D: o ambiente (céu, sol, névoa, sombras), os
    /// modelos e as formas, as câmeras do jogo e a grade do chão.
    pub fn render3d(
        &mut self,
        key: &str,
        root: &Path,
        scene: &SceneData,
        view: &View3,
    ) -> egui::TextureId {
        use glam::{Mat4, Quat, Vec3};
        use nn_render::d3::{Fog, Light3D, Shadows, Sky3D};
        let size = view.size.max(UVec2::ONE);
        self.ensure_target(key, size);

        // O ambiente da cena, como o jogo monta (M.apply_environment).
        let col = |k: &str| parse_color(scene.env(k).str());
        let num = |k: &str| scene.env(k).num() as f32;
        let on = |k: &str| scene.env(k).bool();
        self.renderer.set_sky3d(on("sky").then(|| Sky3D {
            top: col("sky_top"),
            horizon: col("sky_horizon"),
            bottom: col("sky_bottom"),
            sun_size: num("sun_size"),
        }));
        let (h, t) = (num("sun_height").to_radians(), num("sun_turn").to_radians());
        self.renderer.set_light3d(Light3D {
            sun_direction: Vec3::new(-h.cos() * t.sin(), -h.sin(), -h.cos() * t.cos()),
            sun_color: col("sun_color"),
            sky: col("ambient_sky"),
            ground: col("ambient_ground"),
            fog: on("fog").then(|| Fog {
                color: None,
                near: num("fog_near"),
                far: num("fog_far"),
            }),
        });
        self.renderer.set_shadows3d(Shadows {
            enabled: on("shadows"),
            distance: num("shadow_distance").max(1.0),
            ..Shadows::default()
        });
        self.renderer.set_camera3d(view.camera);

        // Os objetos, com a transformação dos pais.
        type Draw = (Mat4, Component, Option<(String, f32)>, u32, usize);
        let mut draws: Vec<Draw> = Vec::new();
        fn walk(list: &[Object], parent: Mat4, out: &mut Vec<Draw>) {
            for o in list {
                if !o.visible {
                    continue;
                }
                let m = parent * crate::scene_data::local_matrix3(o);
                for (ci, c) in o.components.iter().enumerate() {
                    if matches!(
                        c.kind.as_str(),
                        "model" | "shape3d" | "camera3d" | "particles"
                    ) {
                        out.push((m, c.clone(), preview_anim3d(o), o.id, ci));
                    }
                }
                walk(o.prefab_children(), m, out);
                walk(&o.children, m, out);
            }
        }
        walk(&scene.objects, Mat4::IDENTITY, &mut draws);
        let mut seen = Vec::new();
        for (m, c, anim, id, ci) in draws {
            match c.kind.as_str() {
                "particles" => {
                    self.draw_particles3d(key, root, id, ci, &c, m, view, &mut seen);
                }
                "model" => {
                    if let Some(model) = self.model3(root, c.str("file"), c.bool("flat")) {
                        // Modelo animado: a prévia toca a animação, em volta.
                        let joints = model.skeleton.as_ref().and_then(|s| {
                            let (name, speed) = anim.as_ref()?;
                            let i = s.animation(name)?;
                            let d = s.animations[i].duration.max(1e-3);
                            let t = (view.time * speed).rem_euclid(d);
                            Some(s.joint_matrices(&s.pose(i, t)))
                        });
                        match joints {
                            Some(j) => self.renderer.draw_model_posed(
                                &model,
                                m,
                                parse_color(c.str("color")),
                                c.bool("shadow"),
                                &j,
                            ),
                            None => self.renderer.draw_model(
                                &model,
                                m,
                                parse_color(c.str("color")),
                                c.bool("shadow"),
                            ),
                        }
                    }
                }
                "shape3d" => {
                    let mesh = self.mesh3(c.str("shape"));
                    self.renderer.draw_mesh_with(
                        mesh,
                        m,
                        parse_color(c.str("color")),
                        None,
                        c.bool("shadow"),
                    );
                }
                "camera3d" => {
                    // A câmera do jogo: uma caixinha com a lente para a frente.
                    let body = self.mesh3("cube");
                    let lens = self.mesh3("cone");
                    let gray = Color::hex(0x4A5560);
                    let unscaled = {
                        let (_, q, p) = m.to_scale_rotation_translation();
                        Mat4::from_rotation_translation(q, p)
                    };
                    self.renderer.draw_mesh_with(
                        body,
                        unscaled * Mat4::from_scale(Vec3::new(0.35, 0.3, 0.5)),
                        gray,
                        None,
                        false,
                    );
                    self.renderer.draw_mesh_with(
                        lens,
                        unscaled
                            * Mat4::from_scale_rotation_translation(
                                Vec3::splat(0.32),
                                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                                Vec3::new(0.0, 0.0, -0.38),
                            ),
                        gray,
                        None,
                        false,
                    );
                }
                _ => {}
            }
        }
        self.previews3.retain(|k, _| k.0 != key || seen.contains(k));

        // A grade do chão (em volta de onde a câmera olha) e os eixos.
        if view.grid > 0.0 {
            let step = view.grid;
            let line = self.mesh3("cube");
            let c = view.camera.target;
            let (cx, cz) = ((c.x / step).round() * step, (c.z / step).round() * step);
            let n = 30;
            let half = n as f32 * step;
            // Linhas feitas de pedaços, cada um com cerca de 1,5 pixel de
            // largura na distância em que está (linhas finas demais somem em
            // pontinhos; grossas demais pesam perto da câmera).
            let px = 2.0 * (view.camera.fov.to_radians() * 0.5).tan() / view.size.y.max(1) as f32;
            let eye = view.camera.position;
            let pieces = n;
            let piece = half * 2.0 / pieces as f32;
            for i in -n..=n {
                let off = i as f32 * step;
                let (x, z) = (cx + off, cz + off);
                let x_axis = x.abs() < step * 0.01;
                let z_axis = z.abs() < step * 0.01;
                for k in 0..pieces {
                    let along = -half + (k as f32 + 0.5) * piece;
                    // Paralela a Z (x fixo) e paralela a X (z fixo).
                    for (center, axis, color, along_z) in [
                        (Vec3::new(x, 0.003, cz + along), x_axis, 0x4A78FF, true),
                        (Vec3::new(cx + along, 0.003, z), z_axis, 0xFF5A5A, false),
                    ] {
                        let width = (center - eye).length() * px * if axis { 2.2 } else { 1.3 };
                        let size = if along_z {
                            Vec3::new(width, 0.002, piece)
                        } else {
                            Vec3::new(piece, 0.002, width)
                        };
                        self.renderer.draw_mesh_with(
                            line,
                            Mat4::from_scale_rotation_translation(size, Quat::IDENTITY, center),
                            if axis {
                                Color::hex(color)
                            } else {
                                Color::rgba(0.0, 0.0, 0.0, 0.16)
                            },
                            None,
                            false,
                        );
                    }
                }
            }
        }

        let target = &self.targets[key];
        self.renderer.render(
            &target.view,
            size,
            &Camera2D::default(),
            Some(view.background),
        );
        target.id
    }

    /// Anima e desenha as partículas de um componente.
    #[allow(clippy::too_many_arguments)]
    fn draw_particles(
        &mut self,
        key: &str,
        root: &Path,
        id: u32,
        ci: usize,
        c: &Component,
        pos: Vec2,
        view: &RenderView,
        seen: &mut Vec<(String, u32, usize)>,
    ) {
        let image = if c.str("image").is_empty() {
            None
        } else {
            self.texture(root, c.str("image"), true).map(|tex| {
                let (fw, fh) = (c.num("frame_w") as u32, c.num("frame_h") as u32);
                let size = self.renderer.texture_size(tex);
                if fw > 0 && fh > 0 && fw <= size.x && fh <= size.y {
                    let sheet = self.renderer.sprite_sheet(tex, UVec2::new(fw, fh));
                    let frame = (c.num("frame").max(1.0) as u32).min(sheet.frame_count());
                    (tex, sheet.region(frame - 1))
                } else {
                    (tex, UvRect::FULL)
                }
            })
        };
        let k = (key.to_string(), id, ci);
        seen.push(k.clone());
        let props = format!("{:?}", c.props);
        let burst = c.str("mode") == "explosao";
        let amount = c.num("amount").max(0.0) as u32;
        let time = view.time;
        let p = self.previews.entry(k).or_insert_with(|| Preview {
            props: String::new(),
            emitter: ParticleEmitter::new(ParticleConfig::default()),
            time,
            next: time,
        });
        if p.props != props {
            // Mudou no Inspetor: o emissor começa de novo.
            p.props = props;
            p.emitter = ParticleEmitter::new(particle_config(c, image));
            p.emitter.position = pos;
            p.emitter.emitting = !burst && c.bool("emitting");
            p.next = time;
            if burst && !c.bool("on_start") {
                p.next = f32::INFINITY;
            }
        }
        p.emitter.position = pos;
        let dt = (time - p.time).clamp(0.0, 0.1);
        p.time = time;
        if view.bursts.contains(&id) {
            p.emitter.burst(amount.max(1));
        }
        if view.particles {
            // Explosão: solta e, depois que acaba, solta de novo (para ver).
            if burst && time >= p.next {
                p.emitter.burst(amount);
                let interval = c.num("interval") as f32;
                let life = c.num("lifetime") as f32;
                p.next = time + if interval > 0.0 { interval } else { life + 0.8 };
            }
            p.emitter.update(dt);
        }
        p.emitter.draw(&mut self.renderer);
    }

    /// Anima e desenha as partículas de um componente numa cena 3D.
    #[allow(clippy::too_many_arguments)]
    fn draw_particles3d(
        &mut self,
        key: &str,
        root: &Path,
        id: u32,
        ci: usize,
        c: &Component,
        m: glam::Mat4,
        view: &View3,
        seen: &mut Vec<(String, u32, usize)>,
    ) {
        let image = if c.str("image").is_empty() {
            None
        } else {
            self.texture(root, c.str("image"), true).map(|tex| {
                let (fw, fh) = (c.num("frame_w") as u32, c.num("frame_h") as u32);
                let size = self.renderer.texture_size(tex);
                if fw > 0 && fh > 0 && fw <= size.x && fh <= size.y {
                    let sheet = self.renderer.sprite_sheet(tex, UVec2::new(fw, fh));
                    let frame = (c.num("frame").max(1.0) as u32).min(sheet.frame_count());
                    (tex, sheet.region(frame - 1))
                } else {
                    (tex, UvRect::FULL)
                }
            })
        };
        let pos = m.transform_point3(glam::Vec3::ZERO);
        let up = m.transform_vector3(glam::Vec3::Y);
        let k = (key.to_string(), id, ci);
        seen.push(k.clone());
        let props = format!("{:?}", c.props);
        let burst = c.str("mode") == "explosao";
        let amount = c.num("amount").max(0.0) as u32;
        let time = view.time;
        let p = self.previews3.entry(k).or_insert_with(|| Preview3 {
            props: String::new(),
            emitter: nn_render::d3::ParticleEmitter3::new(Default::default()),
            time,
            next: time,
        });
        if p.props != props {
            p.props = props;
            p.emitter = nn_render::d3::ParticleEmitter3::new(particle_config3(c, image, up));
            p.emitter.emitting = !burst && c.bool("emitting");
            p.next = time;
            if burst && !c.bool("on_start") {
                p.next = f32::INFINITY;
            }
        }
        p.emitter.position = pos;
        p.emitter.config.direction = up;
        let dt = (time - p.time).clamp(0.0, 0.1);
        p.time = time;
        if view.bursts.contains(&id) {
            p.emitter.burst(amount.max(1));
        }
        if burst && time >= p.next {
            p.emitter.burst(amount);
            let interval = c.num("interval") as f32;
            let life = c.num("lifetime") as f32;
            p.next = time + if interval > 0.0 { interval } else { life + 0.8 };
        }
        p.emitter.update(dt);
        p.emitter.draw(&mut self.renderer);
    }

    /// Há partículas animando na prévia 3D desta aba?
    pub fn has_particles3d(&self, key: &str) -> bool {
        self.previews3.keys().any(|k| k.0 == key)
    }

    /// A fonte do arquivo (vazio: a do tema, ou a padrão).
    fn font(&mut self, root: &Path, path: &str) -> Option<FontId> {
        let path = if path.is_empty() {
            crate::scene_data::theme_value("font")
        } else {
            path.to_string()
        };
        if path.is_empty() {
            return None;
        }
        let renderer = &mut self.renderer;
        *self
            .fonts
            .entry(path.clone())
            .or_insert_with(|| renderer.load_font(root.join(&path)).ok())
    }

    fn text_of(
        &mut self,
        root: &Path,
        content: &str,
        size: f32,
        color: &str,
        font: &str,
    ) -> (String, f32, Color, Option<FontId>) {
        (
            content.to_string(),
            size,
            parse_color(color),
            self.font(root, font),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_label(
        &mut self,
        root: &Path,
        content: &str,
        at: Vec2,
        size: f32,
        color: &str,
        font: &str,
        anchor: Anchor,
    ) {
        let (content, size, color, font) = self.text_of(root, content, size, color, font);
        let mut t = Text::new(&content)
            .at(at)
            .size(size)
            .color(color)
            .anchor(anchor);
        if let Some(f) = font {
            t = t.font(f);
        }
        self.renderer.draw_text(&t);
    }

    fn measure(&mut self, root: &Path, content: &str, size: f32, font: &str) -> f32 {
        let font = self.font(root, font);
        let mut t = Text::new(content).size(size);
        if let Some(f) = font {
            t = t.font(f);
        }
        self.renderer.measure_text(&t).x
    }

    /// Desenha um componente de interface com centro em `pos` (mundo do
    /// editor, y para cima), no estado normal, como o jogo desenha.
    fn draw_ui(&mut self, root: &Path, c: &Component, pos: Vec2, scale: Vec2) {
        use crate::scene_data::themed;
        let (sx, sy) = (scale.x.abs(), scale.y.abs());
        let num = |k: &str| c.num(k) as f32;
        let panel_image = |me: &mut Self, key: &str| -> Option<PanelImage> {
            let tex = me.texture(root, c.str(key), true)?;
            Some(PanelImage {
                texture: tex,
                region: nn_render::Rect::FULL,
                slice: num("slice"),
                scale: num("slice_scale"),
            })
        };
        match c.kind.as_str() {
            "panel" => {
                let mut p = Panel::new(pos, Vec2::new(num("w") * sx, num("h") * sy))
                    .color(parse_color(&themed(c.str("color"), "panel")))
                    .radius(num("radius"))
                    .border(
                        num("border"),
                        parse_color(&themed(c.str("border_color"), "border")),
                    );
                if let Some(img) = panel_image(self, "image") {
                    p = p.image(img);
                }
                self.renderer.draw_panel(&p);
            }
            "button" => {
                let (w, h) = (num("w") * sx, num("h") * sy);
                let image = panel_image(self, "image");
                let color = if image.is_some() {
                    if c.str("color").is_empty() {
                        "#FFFFFF".to_string()
                    } else {
                        c.str("color").to_string()
                    }
                } else if !c.bool("enabled") {
                    themed("", "disabled")
                } else {
                    themed(c.str("color"), "button")
                };
                let mut p = Panel::new(pos, Vec2::new(w, h))
                    .color(parse_color(&color))
                    .radius(num("radius"))
                    .border(
                        num("border"),
                        parse_color(&themed(c.str("border_color"), "border")),
                    );
                if let Some(img) = image {
                    p = p.image(img);
                }
                self.renderer.draw_panel(&p);
                let size = num("size") * sy;
                let text = c.str("text");
                let tc = if c.bool("enabled") {
                    themed(c.str("text_color"), "text")
                } else {
                    "#FFFFFF80".to_string()
                };
                let mut dx = 0.0;
                if let Some(icon) = self.texture(root, c.str("icon"), true) {
                    let isz = h * 0.6;
                    let tw = if text.is_empty() {
                        0.0
                    } else {
                        self.measure(root, text, size, c.str("font"))
                    };
                    let gap = if text.is_empty() { 0.0 } else { size * 0.4 };
                    let total = isz + gap + tw;
                    self.renderer.draw(
                        &Sprite::new(icon)
                            .at(pos + Vec2::new(-total / 2.0 + isz / 2.0, 0.0))
                            .size(Vec2::splat(isz)),
                    );
                    dx = -total / 2.0 + isz + gap + tw / 2.0;
                }
                if !text.is_empty() {
                    self.draw_label(
                        root,
                        text,
                        pos + Vec2::new(dx, 0.0),
                        size,
                        &tc,
                        c.str("font"),
                        Anchor::Center,
                    );
                }
            }
            "bar" => {
                let (w, h) = (num("w") * sx, num("h") * sy);
                self.renderer.draw_panel(
                    &Panel::new(pos, Vec2::new(w, h))
                        .color(parse_color(c.str("back_color")))
                        .radius(num("radius"))
                        .border(
                            num("border"),
                            parse_color(&themed(c.str("border_color"), "border")),
                        ),
                );
                let mut v = c.num("value");
                if !c.str("var").is_empty() {
                    v = var_num(c.str("var")).unwrap_or(0.0);
                }
                let mut max = c.num("max");
                if !c.str("max_var").is_empty() {
                    max = var_num(c.str("max_var")).unwrap_or(max);
                }
                let f = if max > 0.0 {
                    (v / max).clamp(0.0, 1.0) as f32
                } else {
                    0.0
                };
                if f > 0.0 {
                    let (fw, fh, d) = match c.str("direction") {
                        "left" => (w * f, h, Vec2::new(w / 2.0 - w * f / 2.0, 0.0)),
                        "up" => (w, h * f, Vec2::new(0.0, -h / 2.0 + h * f / 2.0)),
                        "down" => (w, h * f, Vec2::new(0.0, h / 2.0 - h * f / 2.0)),
                        _ => (w * f, h, Vec2::new(-w / 2.0 + w * f / 2.0, 0.0)),
                    };
                    self.renderer.draw_panel(
                        &Panel::new(pos + d, Vec2::new(fw, fh))
                            .color(parse_color(&themed(c.str("color"), "accent")))
                            .radius(num("radius").min(fw / 2.0).min(fh / 2.0)),
                    );
                }
            }
            "icons" => {
                let (n, k) = icons_count(c);
                let (size, gap) = (num("size") * sy, num("spacing") * sx);
                let total = k as f32 * size + (k.max(1) - 1) as f32 * gap;
                let start = match c.str("align") {
                    "left" => 0.0,
                    "right" => -total,
                    _ => -total / 2.0,
                };
                let tex = self.texture(root, c.str("image"), c.bool("pixel_art"));
                let (fw, fh) = (c.num("frame_w") as u32, c.num("frame_h") as u32);
                for i in 0..k {
                    let at = pos + Vec2::new(start + i as f32 * (size + gap) + size / 2.0, 0.0);
                    let full = i < n;
                    match tex {
                        Some(t) => {
                            let ts = self.renderer.texture_size(t);
                            let sprite = if fw > 0 && fh > 0 && fw <= ts.x && fh <= ts.y {
                                let sheet = self.renderer.sprite_sheet(t, UVec2::new(fw, fh));
                                let frame =
                                    (c.num("frame").max(1.0) as u32).min(sheet.frame_count());
                                sheet.sprite(frame - 1)
                            } else {
                                Sprite::new(t)
                            };
                            let color = if full {
                                c.str("color")
                            } else {
                                c.str("empty_color")
                            };
                            self.renderer.draw(
                                &sprite
                                    .at(at)
                                    .size(Vec2::splat(size))
                                    .color(parse_color(color)),
                            );
                        }
                        None => {
                            let color = if full {
                                themed("", "accent")
                            } else {
                                c.str("empty_color").to_string()
                            };
                            self.renderer
                                .draw_circle(at, size / 2.0, parse_color(&color));
                        }
                    }
                }
            }
            "slider" => {
                let (w, h) = (num("w") * sx, num("h") * sy);
                self.renderer.draw_panel(
                    &Panel::new(pos, Vec2::new(w, h))
                        .color(parse_color(c.str("back_color")))
                        .radius(h / 2.0),
                );
                let v = match c.str("target") {
                    "musica" | "efeitos" | "geral" => c.num("max"),
                    _ if !c.str("var").is_empty() => var_num(c.str("var")).unwrap_or(c.num("min")),
                    _ => c.num("value"),
                };
                let span = c.num("max") - c.num("min");
                let f = if span != 0.0 {
                    ((v - c.num("min")) / span).clamp(0.0, 1.0) as f32
                } else {
                    0.0
                };
                if f > 0.0 {
                    self.renderer.draw_panel(
                        &Panel::new(
                            pos + Vec2::new(-w / 2.0 + w * f / 2.0, 0.0),
                            Vec2::new(w * f, h),
                        )
                        .color(parse_color(&themed(c.str("color"), "accent")))
                        .radius(h / 2.0),
                    );
                }
                self.renderer.draw_circle(
                    pos + Vec2::new(-w / 2.0 + w * f, 0.0),
                    num("knob") * sy / 2.0,
                    parse_color(c.str("knob_color")),
                );
            }
            "checkbox" => {
                let box_ = num("box") * sy;
                let size = num("size") * sy;
                let text = c.str("text");
                let tw = if text.is_empty() {
                    0.0
                } else {
                    self.measure(root, text, size, c.str("font")) + box_ * 0.4
                };
                let total = box_ + tw;
                let bpos = pos + Vec2::new(-total / 2.0 + box_ / 2.0, 0.0);
                self.renderer.draw_panel(
                    &Panel::new(bpos, Vec2::splat(box_))
                        .color(parse_color(&themed(c.str("color"), "panel")))
                        .radius(box_ * 0.2)
                        .border((box_ * 0.08).max(2.0), parse_color(&themed("", "border"))),
                );
                let checked = match c.str("target") {
                    "tela_cheia" => false,
                    _ if !c.str("var").is_empty() => var_num(c.str("var")).unwrap_or(0.0) != 0.0,
                    _ => c.bool("checked"),
                };
                if checked {
                    let inner = box_ * 0.5;
                    self.renderer.draw_panel(
                        &Panel::new(bpos, Vec2::splat(inner))
                            .color(parse_color(&themed(c.str("check_color"), "accent")))
                            .radius(inner * 0.25),
                    );
                }
                if !text.is_empty() {
                    let tc = themed(c.str("text_color"), "text");
                    self.draw_label(
                        root,
                        text,
                        pos + Vec2::new(-total / 2.0 + box_ + box_ * 0.4, 0.0),
                        size,
                        &tc,
                        c.str("font"),
                        Anchor::CenterLeft,
                    );
                }
            }
            "textfield" => {
                let (w, h) = (num("w") * sx, num("h") * sy);
                self.renderer.draw_panel(
                    &Panel::new(pos, Vec2::new(w, h))
                        .color(parse_color(c.str("color")))
                        .radius(num("radius"))
                        .border(
                            num("border"),
                            parse_color(&themed(c.str("border_color"), "border")),
                        ),
                );
                let value = if c.str("var").is_empty() {
                    c.str("text").to_string()
                } else {
                    match crate::scene_data::var_value(c.str("var")) {
                        Some(crate::scene_data::Prop::Str(s)) => s,
                        Some(crate::scene_data::Prop::Num(n)) => crate::scene_data::format_num(n),
                        _ => String::new(),
                    }
                };
                let pad = 12f32.min(w * 0.1);
                let at = pos + Vec2::new(-w / 2.0 + pad, 0.0);
                let size = num("size") * sy;
                if value.is_empty() {
                    self.draw_label(
                        root,
                        c.str("placeholder"),
                        at,
                        size,
                        "#FFFFFF66",
                        c.str("font"),
                        Anchor::CenterLeft,
                    );
                } else {
                    let tc = themed(c.str("text_color"), "text");
                    self.draw_label(
                        root,
                        &value,
                        at,
                        size,
                        &tc,
                        c.str("font"),
                        Anchor::CenterLeft,
                    );
                }
            }
            _ => {}
        }
    }

    /// Um arquivo mudou no disco: imagens e mapas são lidos de novo.
    pub fn invalidate(&mut self, root: &Path, rel: &str) {
        if let Some(entry) = self.textures.get(rel).copied() {
            let mut keep = [None, None];
            for (i, slot) in entry.into_iter().enumerate() {
                if let Some(Some(id)) = slot {
                    let opts = if i == 1 {
                        TextureOptions::PIXEL_ART
                    } else {
                        TextureOptions::default()
                    };
                    if let Ok(bytes) = std::fs::read(root.join(rel))
                        && self
                            .renderer
                            .replace_texture_bytes(id, &bytes, opts)
                            .is_ok()
                    {
                        keep[i] = slot;
                    }
                }
            }
            if keep == [None, None] {
                self.textures.remove(rel);
            } else {
                self.textures.insert(rel.to_string(), keep);
            }
        }
        if crate::files::FileKind::of(Path::new(rel)) == crate::files::FileKind::Anim {
            self.anims.forget(rel);
        }
        // Modelos 3D (e o .bin e as imagens deles): lidos de novo.
        self.models3.retain(|(r, _), _| r != rel);
        if !rel.to_lowercase().ends_with(".glb") && !rel.to_lowercase().ends_with(".gltf") {
            // Um .bin ou uma imagem de um .gltf: não dá para saber de qual;
            // os .gltf são lidos de novo.
            self.models3
                .retain(|(r, _), _| !r.to_lowercase().ends_with(".gltf"));
        }
        // Mapas dependem de tilesets e imagens: lidos de novo.
        self.maps.clear();
    }

    fn texture(&mut self, root: &Path, rel: &str, pixel_art: bool) -> Option<TextureId> {
        if rel.is_empty() {
            return None;
        }
        let i = usize::from(pixel_art);
        if let Some(Some(t)) = self.textures.get(rel).map(|e| e[i]) {
            return t;
        }
        let opts = if pixel_art {
            TextureOptions::PIXEL_ART
        } else {
            TextureOptions::default()
        };
        let t = self.renderer.load_texture(root.join(rel), opts).ok();
        self.textures.entry(rel.to_string()).or_default()[i] = Some(t);
        t
    }

    /// Versão do mapa aberto no editor que está sendo desenhada.
    pub fn map_source_rev(&self, rel: &str) -> Option<u64> {
        self.map_sources.get(rel).map(|(r, _)| *r)
    }

    /// Desenha o mapa `rel` a partir destes textos (o mapa e os tilesets),
    /// em vez do disco: é o mapa sendo pintado, ainda sem salvar.
    pub fn set_map_source(&mut self, rel: &str, rev: u64, files: HashMap<String, String>) {
        if self.map_source_rev(rel) == Some(rev) {
            return;
        }
        self.map_sources.insert(rel.to_string(), (rev, files));
        self.maps.retain(|(r, _), _| r != rel);
    }

    /// Volta a desenhar o mapa do disco.
    pub fn clear_map_source(&mut self, rel: &str) {
        if self.map_sources.remove(rel).is_some() {
            self.maps.retain(|(r, _), _| r != rel);
        }
    }

    /// Carrega o mapa (uma vez) e devolve a chave dele no cache.
    fn ensure_map(&mut self, root: &Path, rel: &str, scale: f32) -> (String, u32) {
        let key = (rel.to_string(), scale.to_bits());
        if !self.maps.contains_key(&key) {
            let sources = self.map_sources.get(rel).map(|(_, f)| f.clone());
            let mut disk = nn_tilemap::read_from(root);
            let read = |p: &Path| -> Result<String, String> {
                let k = p.to_string_lossy().replace('\\', "/");
                match sources.as_ref().and_then(|s| s.get(&k)) {
                    Some(text) => Ok(text.clone()),
                    None => disk(p),
                }
            };
            let mut loaded = nn_tilemap::load_tiled(Path::new(rel), read, scale.max(0.01)).ok();
            if let Some(m) = &mut loaded {
                // As imagens dos tilesets vêm do cache (o mapa é relido a cada
                // pincelada).
                let renderer = &mut self.renderer;
                let textures = &mut self.textures;
                let ok = m
                    .load_textures(|p| {
                        let r = p.to_string_lossy().replace('\\', "/");
                        match textures.get(r.as_str()).and_then(|e| e[1]) {
                            Some(Some(id)) => Ok(id),
                            _ => {
                                let id =
                                    renderer.load_texture(root.join(p), TextureOptions::PIXEL_ART);
                                if let Ok(id) = &id {
                                    textures.entry(r).or_default()[1] = Some(Some(*id));
                                }
                                id
                            }
                        }
                    })
                    .is_ok();
                if !ok {
                    loaded = None;
                }
            }
            self.maps.insert(key.clone(), loaded);
        }
        key
    }

    /// Desenha a cena como `view` pede numa textura e devolve a textura
    /// para o egui mostrar.
    pub fn render(
        &mut self,
        key: &str,
        root: &Path,
        scene: &SceneData,
        view: &RenderView,
    ) -> egui::TextureId {
        let RenderView {
            camera,
            size,
            background,
            time,
            ..
        } = view;
        let (camera, background, time) = (camera, *background, *time);
        let size = size.max(UVec2::ONE);
        self.ensure_target(key, size);

        let viewport = size.as_vec2();
        let mut seen = Vec::new();
        for placed in draw_order(scene) {
            let w = placed.world;
            let pos = Vec2::new(w.x as f32, w.y as f32);
            for (ci, c) in placed.object.components.iter().enumerate() {
                match c.kind.as_str() {
                    "particles" => {
                        self.draw_particles(
                            key,
                            root,
                            placed.object.id,
                            ci,
                            c,
                            pos,
                            view,
                            &mut seen,
                        );
                    }
                    "sprite" => {
                        let pixel_art = c.bool("pixel_art");
                        let Some(tex) = self.texture(root, c.str("image"), pixel_art) else {
                            continue;
                        };
                        let anims = &mut self.anims;
                        let (fw, fh) = sprite_frame(c, |img| {
                            anims.get(root, img).map(|a| (a.frame_w, a.frame_h))
                        });
                        let (fw, fh) = (fw as u32, fh as u32);
                        let tex_size = self.renderer.texture_size(tex);
                        let (sprite, natural) =
                            if fw > 0 && fh > 0 && fw <= tex_size.x && fh <= tex_size.y {
                                let sheet = self.renderer.sprite_sheet(tex, UVec2::new(fw, fh));
                                // Com animação, o editor já mostra ela tocando.
                                let playing = Some(c.str("animation"))
                                    .filter(|a| !a.is_empty())
                                    .and_then(|name| {
                                        let file = self.anims.get(root, c.str("image"))?;
                                        let a = file.find(name)?;
                                        let pos = a.position_at(time as f64)?;
                                        Some(a.frames[pos].index)
                                    });
                                let frame = playing.unwrap_or(c.num("frame").max(1.0) as u32);
                                if frame > sheet.frame_count() {
                                    continue;
                                }
                                (sheet.sprite(frame - 1), Vec2::new(fw as f32, fh as f32))
                            } else {
                                (Sprite::new(tex), self.renderer.texture_size(tex).as_vec2())
                            };
                        let mut sz = Vec2::new(
                            if c.num("w") > 0.0 {
                                c.num("w") as f32
                            } else {
                                natural.x
                            },
                            if c.num("h") > 0.0 {
                                c.num("h") as f32
                            } else {
                                natural.y
                            },
                        ) * Vec2::new(w.scale_x as f32, w.scale_y as f32);
                        if c.bool("flip_x") {
                            sz.x = -sz.x;
                        }
                        if c.bool("flip_y") {
                            sz.y = -sz.y;
                        }
                        // Repetir: a escala diz quantas cópias cabem.
                        let repeat = if c.str("fill") == "repeat" {
                            Vec2::new(w.scale_x.abs() as f32, w.scale_y.abs() as f32)
                        } else {
                            Vec2::ONE
                        };
                        self.renderer.draw(
                            &sprite
                                .at(pos)
                                .size(sz)
                                .rotation((w.rotation as f32).to_radians())
                                .color(parse_color(c.str("color")))
                                .repeat(repeat),
                        );
                    }
                    "shape" => {
                        let color = parse_color(c.str("color"));
                        if c.str("shape") == "circle" {
                            let s = (w.scale_x.abs()).max(w.scale_y.abs()) as f32;
                            self.renderer
                                .draw_circle(pos, c.num("radius") as f32 * s, color);
                        } else {
                            let sz = Vec2::new(
                                (c.num("w") * w.scale_x.abs()) as f32,
                                (c.num("h") * w.scale_y.abs()) as f32,
                            );
                            self.renderer.draw_rect(pos, sz, color);
                        }
                    }
                    "text" => {
                        let content = c.str("text").to_string();
                        self.draw_label(
                            root,
                            &content,
                            pos,
                            (c.num("size") * w.scale_y.abs()) as f32,
                            c.str("color"),
                            c.str("font"),
                            anchor_of(c.str("anchor")),
                        );
                    }
                    "map" => {
                        let key = self.ensure_map(root, c.str("map"), c.num("scale") as f32);
                        if let Some(Some(map)) = self.maps.get_mut(&key) {
                            map.origin = pos;
                            map.time = time;
                            map.draw(&mut self.renderer, camera, viewport);
                        }
                    }
                    _ => self.draw_ui(root, c, pos, Vec2::new(w.scale_x as f32, w.scale_y as f32)),
                }
            }
        }
        // Emissores que sumiram desta aba (objeto apagado, componente tirado).
        self.previews.retain(|k, _| k.0 != key || seen.contains(k));
        let target = &self.targets[key];
        self.renderer
            .render(&target.view, size, camera, Some(background));
        target.id
    }

    fn ensure_target(&mut self, key: &str, size: UVec2) {
        if self.targets.get(key).is_some_and(|t| t.size == size) {
            return;
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("nn-studio cena"),
            size: wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[EGUI_FORMAT],
        });
        // A engine desenha em sRGB; o egui lê os bytes como estão (gama).
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let egui_view = texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(EGUI_FORMAT),
            ..Default::default()
        });
        let mut egui = self.egui.write();
        let id = match self.targets.get(key) {
            Some(old) => {
                egui.update_egui_texture_from_wgpu_texture(
                    &self.device,
                    &egui_view,
                    wgpu::FilterMode::Nearest,
                    old.id,
                );
                old.id
            }
            None => {
                egui.register_native_texture(&self.device, &egui_view, wgpu::FilterMode::Nearest)
            }
        };
        drop(egui);
        self.targets.insert(
            key.to_string(),
            Target {
                _texture: texture,
                view,
                size,
                id,
            },
        );
    }

    /// Esquece a textura de uma aba fechada.
    pub fn forget(&mut self, key: &str) {
        if let Some(t) = self.targets.remove(key) {
            self.egui.write().free_texture(&t.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_data::{Component, Prop};

    #[test]
    fn cores() {
        assert_eq!(parse_color("#FF0000"), Color::hex(0xFF0000));
        assert_eq!(parse_color("#00000080").a, 128.0 / 255.0);
        assert_eq!(parse_color("azul"), Color::WHITE);
        assert_eq!(
            egui_color("#102030"),
            egui::Color32::from_rgb(0x10, 0x20, 0x30)
        );
    }

    #[test]
    fn medidas_dos_objetos() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jogos/plataforma");
        let mut sizes = Sizes::default();
        let mut o = Object::new(1, "a");
        // Sem nada visível: ícone.
        let (r, icon) = local_bounds(&o, &root, &mut sizes, 10.0);
        assert!(icon);
        assert_eq!(
            r,
            LocalRect {
                min: (-5.0, -5.0),
                max: (5.0, 5.0)
            }
        );
        // Sprite sem tamanho: tamanho da imagem (64×32).
        let mut s = Component::new("sprite");
        s.set("image", Prop::Str("assets/imagens/personagem.png".into()));
        o.components.push(s.clone());
        let (r, icon) = local_bounds(&o, &root, &mut sizes, 10.0);
        assert!(!icon);
        assert_eq!(r.max, (32.0, 16.0));
        // Com quadros: tamanho do quadro.
        s.set("frame_w", Prop::Num(16.0));
        s.set("frame_h", Prop::Num(16.0));
        o.components[0] = s;
        assert_eq!(local_bounds(&o, &root, &mut sizes, 10.0).0.max, (8.0, 8.0));
        // Mapa: do canto inferior esquerdo.
        let mut m = Object::new(2, "mapa");
        let mut c = Component::new("map");
        c.set("map", Prop::Str("fases/fase1.tmj".into()));
        m.components.push(c);
        let (r, _) = local_bounds(&m, &root, &mut sizes, 10.0);
        assert_eq!(r.min, (0.0, 0.0));
        assert_eq!(r.max, (60.0 * 16.0, 15.0 * 16.0));
        // Texto ancorado à esquerda começa na posição.
        let mut t = Object::new(3, "t");
        let mut c = Component::new("text");
        c.set("anchor", Prop::Str("left".into()));
        t.components.push(c);
        let (r, _) = local_bounds(&t, &root, &mut sizes, 10.0);
        assert_eq!(r.min.0, 0.0);
        assert!(r.max.0 > 0.0);
    }

    #[test]
    fn ordem_de_desenho_por_camada() {
        let mut s = SceneData::default();
        let mut a = Object::new(1, "a");
        a.layer = 2;
        let mut b = Object::new(2, "b");
        b.layer = 1;
        let mut filho = Object::new(3, "filho");
        filho.x = 5.0;
        b.children.push(filho);
        let mut escondido = Object::new(4, "escondido");
        escondido.visible = false;
        escondido.children.push(Object::new(5, "dentro"));
        a.x = 100.0;
        b.x = 10.0;
        s.objects = vec![a, b, escondido];
        let ordem: Vec<_> = draw_order(&s)
            .iter()
            .map(|p| (p.object.id, p.world.x))
            .collect();
        // A camada é de cada objeto (o filho, na camada 0, vem antes do pai);
        // o filho escondido junto com o pai não aparece.
        assert_eq!(ordem, [(3, 15.0), (2, 10.0), (1, 100.0)]);
    }
}
