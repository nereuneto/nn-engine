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
use nn_render::{Anchor, Camera2D, Renderer, Sprite, Text, TextureId, TextureOptions};

use crate::scene_data::{Component, Object, SceneData, Transform};

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
}

impl Sizes {
    pub fn invalidate(&mut self, rel: &str) {
        self.images.remove(rel);
        self.maps.retain(|(r, _), _| r != rel);
    }

    pub fn clear(&mut self) {
        self.images.clear();
        self.maps.clear();
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
            _ => {}
        }
    }
    match rect {
        Some(r) if r.max.0 > r.min.0 && r.max.1 > r.min.1 => (r, false),
        _ => (LocalRect::centered(icon, icon), true),
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
}

pub struct SceneRenderer {
    device: wgpu::Device,
    egui: Arc<egui::mutex::RwLock<egui_wgpu::Renderer>>,
    renderer: Renderer,
    textures: HashMap<(String, bool), Option<TextureId>>,
    maps: HashMap<(String, u32), Option<nn_tilemap::TileMap>>,
    targets: HashMap<String, Target>,
    anims: crate::anim_file::AnimCache,
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
        }
    }

    /// Um arquivo mudou no disco: imagens e mapas são lidos de novo.
    pub fn invalidate(&mut self, root: &Path, rel: &str) {
        let keys: Vec<(String, bool)> = self
            .textures
            .keys()
            .filter(|(r, _)| r == rel)
            .cloned()
            .collect();
        for key in keys {
            if let Some(Some(id)) = self.textures.get(&key).copied() {
                let opts = if key.1 {
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
                    continue;
                }
            }
            self.textures.remove(&key);
        }
        // Mapas dependem de tilesets e imagens: lidos de novo.
        self.maps.clear();
    }

    fn texture(&mut self, root: &Path, rel: &str, pixel_art: bool) -> Option<TextureId> {
        if rel.is_empty() {
            return None;
        }
        let renderer = &mut self.renderer;
        *self
            .textures
            .entry((rel.to_string(), pixel_art))
            .or_insert_with(|| {
                let opts = if pixel_art {
                    TextureOptions::PIXEL_ART
                } else {
                    TextureOptions::default()
                };
                renderer.load_texture(root.join(rel), opts).ok()
            })
    }

    /// Carrega o mapa (uma vez) e devolve a chave dele no cache.
    fn ensure_map(&mut self, root: &Path, rel: &str, scale: f32) -> (String, u32) {
        let key = (rel.to_string(), scale.to_bits());
        if !self.maps.contains_key(&key) {
            let mut loaded = nn_tilemap::load_tiled(
                Path::new(rel),
                nn_tilemap::read_from(root),
                scale.max(0.01),
            )
            .ok();
            if let Some(m) = &mut loaded {
                let renderer = &mut self.renderer;
                let ok = m
                    .load_textures(|p| {
                        renderer.load_texture(root.join(p), TextureOptions::PIXEL_ART)
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
        } = view;
        let (camera, background, time) = (camera, *background, *time);
        let size = size.max(UVec2::ONE);
        self.ensure_target(key, size);

        let viewport = size.as_vec2();
        for placed in draw_order(scene) {
            let w = placed.world;
            let pos = Vec2::new(w.x as f32, w.y as f32);
            for c in &placed.object.components {
                match c.kind.as_str() {
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
                        self.renderer.draw_text(
                            &Text::new(&content)
                                .at(pos)
                                .size((c.num("size") * w.scale_y.abs()) as f32)
                                .color(parse_color(c.str("color")))
                                .anchor(anchor_of(c.str("anchor"))),
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
                    _ => {}
                }
            }
        }
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
