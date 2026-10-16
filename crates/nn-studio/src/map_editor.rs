//! A aba de um mapa `.tmj`: o mapa desenhado (com as camadas, giros e
//! animações dos tiles) e pintado com o mesmo pincel do painel Tiles.

use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

use eframe::egui::{self, Color32, Key, Modifiers, Pos2, Rect, Sense, Stroke, TextureHandle, Vec2};

use crate::map_doc::{FLIP_BITS, MapDoc};
use crate::paint::{PaintState, PaintTool, tileset_data};
use crate::tileset::TilesetDoc;

/// Imagens dos tilesets como texturas do egui (relidas quando mudam no disco).
#[derive(Default)]
pub struct ImageCache {
    map: HashMap<String, (Option<SystemTime>, TextureHandle, [u32; 2])>,
}

impl std::fmt::Debug for ImageCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageCache")
            .field("images", &self.map.len())
            .finish()
    }
}

impl ImageCache {
    pub fn get(
        &mut self,
        ctx: &egui::Context,
        root: &Path,
        rel: &str,
    ) -> Option<(TextureHandle, [u32; 2])> {
        let full = root.join(rel);
        let modified = std::fs::metadata(&full).and_then(|m| m.modified()).ok();
        if self.map.get(rel).is_none_or(|(m, _, _)| *m != modified) {
            let img = crate::viewers::load_rgba(&full).ok()?;
            let size = [img.width(), img.height()];
            let tex = ctx.load_texture(
                format!("tiles:{rel}"),
                crate::viewers::to_color_image(&img),
                egui::TextureOptions::NEAREST,
            );
            self.map.insert(rel.to_string(), (modified, tex, size));
        }
        self.map.get(rel).map(|(_, t, s)| (t.clone(), *s))
    }
}

/// Os cantos da imagem do tile para cada canto da tela (cima-esq., cima-dir.,
/// baixo-dir., baixo-esq.), com os giros do Tiled (diagonal, depois
/// horizontal, depois vertical).
pub fn flip_uv(raw: u32) -> [(f32, f32); 4] {
    let (d, h, v) = (
        raw & 0x2000_0000 != 0,
        raw & 0x8000_0000 != 0,
        raw & 0x4000_0000 != 0,
    );
    [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)].map(|(x, y)| {
        let (mut a, mut b) = (x, y);
        if v {
            b = 1.0 - b;
        }
        if h {
            a = 1.0 - a;
        }
        if d {
            std::mem::swap(&mut a, &mut b);
        }
        (a, b)
    })
}

/// Desenha as camadas visíveis do mapa com o canto de cima à esquerda em
/// `origin` e `px` pontos da tela por pixel do mapa.
#[allow(clippy::too_many_arguments)]
pub fn draw_map(
    painter: &egui::Painter,
    ctx: &egui::Context,
    root: &Path,
    images: &mut ImageCache,
    map: &MapDoc,
    tilesets: &HashMap<String, TilesetDoc>,
    origin: Pos2,
    px: f32,
    clip: Rect,
    time_ms: u64,
) {
    let (tw, th) = (map.tile_w as f32 * px, map.tile_h as f32 * px);
    let c0 = (((clip.left() - origin.x) / tw).floor() as i64).max(0);
    let c1 = (((clip.right() - origin.x) / tw).ceil() as i64).min(map.width as i64);
    let r0 = (((clip.top() - origin.y) / th).floor() as i64).max(0);
    let r1 = (((clip.bottom() - origin.y) / th).ceil() as i64).min(map.height as i64);
    if c0 >= c1 || r0 >= r1 {
        return;
    }
    // Uma malha por imagem de tileset.
    let mut meshes: HashMap<usize, egui::Mesh> = HashMap::new();
    let mut textures: HashMap<usize, Option<(TextureHandle, [u32; 2])>> = HashMap::new();
    for li in map.tile_layers() {
        let Some(layer) = map.tile_layer(li) else {
            continue;
        };
        if !layer.visible || layer.opacity <= 0.0 {
            continue;
        }
        let tint = Color32::from_white_alpha((layer.opacity.clamp(0.0, 1.0) * 255.0) as u8);
        for r in r0..r1 {
            for c in c0..c1 {
                let raw = map.get(li, c, r);
                let gid = raw & !FLIP_BITS;
                if gid == 0 {
                    continue;
                }
                let Some((ts, mut local)) = map.tileset_of(gid) else {
                    continue;
                };
                let Some(data) = tileset_data(map, ts, tilesets) else {
                    continue;
                };
                let anim = data.info(local).animation;
                if anim.len() > 1 {
                    let total: u64 = anim.iter().map(|(_, ms)| (*ms).max(1) as u64).sum();
                    let mut t = time_ms % total.max(1);
                    for (f, ms) in &anim {
                        let ms = (*ms).max(1) as u64;
                        if t < ms {
                            local = *f;
                            break;
                        }
                        t -= ms;
                    }
                }
                let tex = textures
                    .entry(ts)
                    .or_insert_with(|| images.get(ctx, root, &data.image))
                    .clone();
                let Some((tex, size)) = tex else { continue };
                let (sx, sy) = data.tile_origin(local);
                let (iw, ih) = (size[0].max(1) as f32, size[1].max(1) as f32);
                let mesh = meshes
                    .entry(ts)
                    .or_insert_with(|| egui::Mesh::with_texture(tex.id()));
                let min = origin + Vec2::new(c as f32 * tw, r as f32 * th);
                let corners = [
                    min,
                    min + Vec2::new(tw, 0.0),
                    min + Vec2::new(tw, th),
                    min + Vec2::new(0.0, th),
                ];
                let uv = flip_uv(raw);
                let base = mesh.vertices.len() as u32;
                for (p, (a, b)) in corners.iter().zip(uv) {
                    mesh.vertices.push(egui::epaint::Vertex {
                        pos: *p,
                        uv: Pos2::new(
                            (sx as f32 + a * data.tile_w as f32) / iw,
                            (sy as f32 + b * data.tile_h as f32) / ih,
                        ),
                        color: tint,
                    });
                }
                mesh.indices
                    .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
            }
        }
        // Cada camada por cima da anterior.
        let mut keys: Vec<usize> = meshes.keys().copied().collect();
        keys.sort();
        for k in keys {
            if let Some(m) = meshes.remove(&k) {
                painter.add(egui::Shape::mesh(m));
            }
        }
    }
}

/// O que a aba pede ao espaço de trabalho.
#[derive(Debug, Clone, PartialEq)]
pub enum MapAction {
    CopyText(String),
    OpenExternal(String),
    /// Clicaram na aba: o painel Tiles passa a mostrar este mapa.
    Focus,
}

#[derive(Default)]
pub struct MapEditor {
    zoom: f32,
    /// Canto de cima à esquerda do mapa, relativo ao centro da vista.
    offset: Vec2,
    framed: bool,
    stroke: Option<crate::paint::Stroke>,
    pan: Option<(Pos2, Vec2)>,
    grown_seen: (i64, i64),
    images: ImageCache,
}

impl std::fmt::Debug for MapEditor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MapEditor")
            .field("zoom", &self.zoom)
            .finish_non_exhaustive()
    }
}

impl MapEditor {
    fn frame(&mut self, map: &MapDoc, rect: Rect) {
        let (w, h) = (
            (map.width * map.tile_w) as f32,
            (map.height * map.tile_h) as f32,
        );
        let z = ((rect.width() - 24.0) / w.max(1.0)).min((rect.height() - 24.0) / h.max(1.0));
        // Pixel art fica melhor em zoom inteiro.
        self.zoom = if z >= 1.0 { z.floor() } else { z.max(0.05) };
        self.offset = -Vec2::new(w, h) * self.zoom / 2.0;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        root: &Path,
        rel: &str,
        maps: &mut HashMap<String, MapDoc>,
        tilesets: &mut HashMap<String, TilesetDoc>,
        paint: &mut PaintState,
    ) -> Vec<MapAction> {
        let mut actions = Vec::new();
        let Some(map) = maps.get(rel) else {
            return actions;
        };
        let layer_name = map
            .tile_layer(map.layer)
            .map(|l| l.name.clone())
            .unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            ui.label(format!(
                "{} × {} tiles de {}×{} px · camada {}",
                map.width, map.height, map.tile_w, map.tile_h, layer_name
            ));
            ui.separator();
            ui.checkbox(&mut paint.show_grid, "Grade");
            ui.checkbox(&mut paint.show_collision, "Colisão");
            ui.label(format!("{:.0}%", self.zoom * 100.0))
                .on_hover_text("Zoom: role o mouse. Mover a vista: botão do meio, ou Espaço + arrastar.");
            if ui.button("⛶ Enquadrar").on_hover_text("F").clicked() {
                self.framed = false;
            }
            ui.separator();
            if ui
                .button("Abrir no Tiled")
                .on_hover_text("Abre o mapa no programa padrão (o Tiled, se estiver instalado). Ao salvar lá, o editor e o jogo atualizam.")
                .clicked()
            {
                actions.push(MapAction::OpenExternal(rel.to_string()));
            }
            let code = format!("nn.tilemap.load(\"{rel}\")");
            if ui.button("📋 Copiar código").on_hover_text(format!("Copia: {code}")).clicked() {
                actions.push(MapAction::CopyText(code));
            }
        });
        ui.weak("Pinte aqui ou na cena (✏ Pintar). O pincel, as camadas e os tiles ficam no painel Tiles.");
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        crate::tutorial::anchor_in(ui, "mapa.vista", rect);
        if !self.framed {
            self.framed = true;
            self.frame(map, rect);
            self.grown_seen = map.grown_left_top;
        }
        if response.is_pointer_button_down_on() || response.clicked() {
            actions.push(MapAction::Focus);
        }
        self.input(ui, &response, rect, rel, maps, tilesets, paint);
        let Some(map) = maps.get(rel) else {
            return actions;
        };
        // Cresceu à esquerda ou em cima: o desenho fica no lugar.
        let (gl, gt) = (
            map.grown_left_top.0 - self.grown_seen.0,
            map.grown_left_top.1 - self.grown_seen.1,
        );
        if (gl, gt) != (0, 0) {
            self.offset -=
                Vec2::new(gl as f32 * map.tile_w as f32, gt as f32 * map.tile_h as f32) * self.zoom;
            self.grown_seen = map.grown_left_top;
        }
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_gray(0x2A));
        let origin = rect.center() + self.offset;
        let z = self.zoom;
        let size = Vec2::new(
            (map.width * map.tile_w) as f32,
            (map.height * map.tile_h) as f32,
        ) * z;
        let area = Rect::from_min_size(origin, size);
        painter.rect_filled(
            area,
            0.0,
            Color32::from_rgb(0x5F, 0xB3, 0xE0).gamma_multiply(0.45),
        );
        let time_ms = (ui.input(|i| i.time) * 1000.0) as u64;
        draw_map(
            &painter,
            ui.ctx(),
            root,
            &mut self.images,
            map,
            tilesets,
            origin,
            z,
            rect,
            time_ms,
        );
        if has_animation(map, tilesets) {
            crate::theme::animate(ui.ctx(), std::time::Duration::from_millis(50));
        }
        painter.rect_stroke(
            area,
            0.0,
            Stroke::new(1.5, Color32::from_rgb(0xFF, 0x9F, 0x1C)),
            egui::StrokeKind::Outside,
        );
        let (tw, th) = (map.tile_w as f32 * z, map.tile_h as f32 * z);
        let vis = area.intersect(rect);
        if vis.is_positive() {
            let c0 = (((vis.left() - origin.x) / tw).floor() as i64).max(0);
            let c1 = (((vis.right() - origin.x) / tw).ceil() as i64).min(map.width as i64);
            let r0 = (((vis.top() - origin.y) / th).floor() as i64).max(0);
            let r1 = (((vis.bottom() - origin.y) / th).ceil() as i64).min(map.height as i64);
            if paint.show_grid && tw >= 8.0 {
                let stroke = Stroke::new(1.0, Color32::from_white_alpha(28));
                for c in c0..=c1 {
                    painter.vline(origin.x + c as f32 * tw, vis.y_range(), stroke);
                }
                for r in r0..=r1 {
                    painter.hline(vis.x_range(), origin.y + r as f32 * th, stroke);
                }
            }
            if paint.show_collision && (c1 - c0) * (r1 - r0) <= 40_000 {
                for r in r0..r1 {
                    for c in c0..c1 {
                        let (solid, one_way) = cell_collision(map, tilesets, c, r);
                        let cell = Rect::from_min_size(
                            origin + Vec2::new(c as f32 * tw, r as f32 * th),
                            Vec2::new(tw, th),
                        );
                        if solid {
                            painter.rect_filled(
                                cell,
                                0.0,
                                Color32::from_rgba_unmultiplied(0xE5, 0x48, 0x4D, 45),
                            );
                        } else if one_way {
                            painter.hline(
                                cell.x_range(),
                                cell.top() + 1.5,
                                Stroke::new(3.0, Color32::from_rgb(0x4A, 0x9C, 0xF0)),
                            );
                        }
                    }
                }
            }
        }
        // Onde o pincel vai pintar.
        if let Some(p) = response.hover_pos() {
            let (c, r) = self.cell_at(map, rect, p);
            let (w, h) = match paint.tool {
                PaintTool::Brush => (paint.stamp.w as i64, paint.stamp.h as i64),
                _ => (1, 1),
            };
            let hover = Rect::from_min_size(
                origin + Vec2::new(c as f32 * tw, r as f32 * th),
                Vec2::new(w as f32 * tw, h as f32 * th),
            );
            painter.rect_stroke(
                hover,
                0.0,
                Stroke::new(2.0, Color32::WHITE),
                egui::StrokeKind::Inside,
            );
            painter.text(
                rect.left_bottom() + Vec2::new(8.0, -8.0),
                egui::Align2::LEFT_BOTTOM,
                format!(
                    "{} · camada {} · coluna {} · linha {}",
                    paint.tool.label(),
                    layer_name,
                    c + 1,
                    r + 1
                ),
                egui::FontId::monospace(12.0),
                Color32::WHITE,
            );
        }
        actions
    }

    fn cell_at(&self, map: &MapDoc, rect: Rect, p: Pos2) -> (i64, i64) {
        let origin = rect.center() + self.offset;
        let v = (p - origin) / self.zoom;
        (
            (v.x / map.tile_w as f32).floor() as i64,
            (v.y / map.tile_h as f32).floor() as i64,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn input(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        rect: Rect,
        rel: &str,
        maps: &mut HashMap<String, MapDoc>,
        tilesets: &mut HashMap<String, TilesetDoc>,
        paint: &mut PaintState,
    ) {
        if let Some(p) = response.hover_pos() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            let pinch = ui.input(|i| i.zoom_delta());
            let k = (1.0015f32).powf(scroll) * pinch;
            if k != 1.0 {
                let new = (self.zoom * k).clamp(0.05, 32.0);
                // O ponto sob o mouse fica parado.
                let before = p - (rect.center() + self.offset);
                self.offset += before - before * (new / self.zoom);
                self.zoom = new;
            }
        }
        if (response.hovered() || response.has_focus()) && !ui.ctx().text_edit_focused() {
            let pressed =
                |ui: &mut egui::Ui, m: Modifiers, k: Key| ui.input_mut(|i| i.consume_key(m, k));
            for tool in PaintTool::ALL {
                if pressed(ui, Modifiers::NONE, tool.key()) {
                    paint.tool = tool;
                }
            }
            if pressed(ui, Modifiers::NONE, Key::R) {
                paint.stamp = paint.stamp.rotated();
            }
            if pressed(ui, Modifiers::NONE, Key::X) {
                paint.stamp = paint.stamp.flipped_h();
            }
            if pressed(ui, Modifiers::SHIFT, Key::X) {
                paint.stamp = paint.stamp.flipped_v();
            }
            if pressed(ui, Modifiers::NONE, Key::F) {
                self.framed = false;
            }
        }
        let (space, middle) = ui.input(|i| (i.key_down(Key::Space), i.pointer.middle_down()));
        if response.drag_started() && (middle || space) {
            let start = ui
                .input(|i| i.pointer.press_origin())
                .unwrap_or(rect.center());
            self.pan = Some((start, self.offset));
        }
        if let (Some((start, off)), Some(now)) = (self.pan, response.interact_pointer_pos()) {
            self.offset = off + (now - start);
        }
        if response.drag_stopped() || !(middle || space || ui.input(|i| i.pointer.any_down())) {
            self.pan = None;
        }
        if space || middle || self.pan.is_some() {
            return;
        }
        let events: Vec<egui::Event> = ui.input(|i| i.events.clone());
        for e in events {
            let Some(map) = maps.get_mut(rel) else { return };
            match e {
                egui::Event::PointerButton {
                    pos,
                    button,
                    pressed: true,
                    modifiers,
                } if rect.contains(pos)
                    && response.contains_pointer()
                    && self.stroke.is_none()
                    && matches!(
                        button,
                        egui::PointerButton::Primary | egui::PointerButton::Secondary
                    ) =>
                {
                    response.request_focus();
                    let cell = self.cell_at(map, rect, pos);
                    let pick = button == egui::PointerButton::Secondary || modifiers.alt;
                    let (stroke, _) = paint.begin(map, tilesets, cell, modifiers.command, pick);
                    self.stroke = Some(stroke);
                }
                egui::Event::PointerMoved(pos) => {
                    if let Some(mut stroke) = self.stroke.take() {
                        let cell = self.cell_at(map, rect, pos);
                        // O desenho fica no lugar se o mapa crescer no meio do traço.
                        let cell = (
                            cell.0 + map.grown_left_top.0 - self.grown_seen.0,
                            cell.1 + map.grown_left_top.1 - self.grown_seen.1,
                        );
                        paint.drag(map, tilesets, &mut stroke, cell);
                        self.stroke = Some(stroke);
                    }
                }
                egui::Event::PointerButton {
                    pos,
                    pressed: false,
                    ..
                } => {
                    if let Some(stroke) = self.stroke.take() {
                        let cell = self.cell_at(map, rect, pos);
                        let cell = (
                            cell.0 + map.grown_left_top.0 - self.grown_seen.0,
                            cell.1 + map.grown_left_top.1 - self.grown_seen.1,
                        );
                        paint.end(map, stroke, cell);
                    }
                }
                _ => {}
            }
        }
        if self.stroke.is_some() && !ui.input(|i| i.pointer.any_down()) {
            let stroke = self.stroke.take().unwrap();
            if let Some(map) = maps.get_mut(rel) {
                let last = stroke.last;
                paint.end(map, stroke, last);
            }
        }
        // A aba do mapa não tem objeto para mover: o deslocamento é só daqui.
        if let Some(map) = maps.get_mut(rel) {
            let _ = map.take_shift();
        }
    }
}

/// A célula bloqueia (algum tile sólido ou camada de colisão) ou é plataforma.
pub fn cell_collision(
    map: &MapDoc,
    tilesets: &HashMap<String, TilesetDoc>,
    c: i64,
    r: i64,
) -> (bool, bool) {
    let mut solid = false;
    let mut one_way = false;
    for li in map.tile_layers() {
        let gid = map.get(li, c, r) & !FLIP_BITS;
        if gid == 0 {
            continue;
        }
        if map.tile_layer(li).is_some_and(|l| l.collision) {
            solid = true;
            continue;
        }
        let info = crate::tiles_view::tile_info(map, tilesets, gid);
        solid |= info.solid;
        one_way |= info.one_way;
    }
    (solid, one_way)
}

fn has_animation(map: &MapDoc, tilesets: &HashMap<String, TilesetDoc>) -> bool {
    (0..map.tilesets.len()).any(|i| {
        tileset_data(map, i, tilesets)
            .is_some_and(|d| d.tiles.values().any(|t| t.animation.len() > 1))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn giros_como_no_tiled() {
        // Sem giro: cada canto da tela pega o mesmo canto da imagem.
        assert_eq!(flip_uv(0), [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);
        // Espelho horizontal.
        assert_eq!(flip_uv(0x8000_0000)[0], (1.0, 0.0));
        // Girar 90° no sentido horário (diagonal + horizontal): o canto de
        // cima à esquerda da tela mostra o de baixo à esquerda da imagem.
        assert_eq!(flip_uv(0xA000_0000)[0], (0.0, 1.0));
        assert_eq!(flip_uv(0xA000_0000)[1], (0.0, 0.0));
    }
}
