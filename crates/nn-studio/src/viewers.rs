//! Abas de visualização: imagem (com zoom, grade de pixels e de quadros),
//! som (forma de onda e botão de tocar) e mapa do Tiled (prévia montada com
//! os tiles de verdade e os objetos marcados).

use std::path::Path;
use std::time::Instant;

use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, Vec2};

/// Ação pedida por um visualizador.
#[derive(Debug, Clone, PartialEq)]
pub enum ViewerAction {
    CopyText(String),
    OpenExternal(String),
}

// ---------------------------------------------------------------------------
// Imagem

pub struct ImageView {
    texture: Option<TextureHandle>,
    pub size: [usize; 2],
    pub error: Option<String>,
    /// Zoom; `None` = ajustar à aba.
    zoom: Option<f32>,
    pub frames: bool,
    pub frame_size: [u32; 2],
    needs_load: bool,
    bytes: u64,
}

impl std::fmt::Debug for ImageView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageView")
            .field("size", &self.size)
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl Default for ImageView {
    fn default() -> Self {
        Self {
            texture: None,
            size: [0, 0],
            error: None,
            zoom: None,
            frames: false,
            frame_size: [16, 16],
            needs_load: true,
            bytes: 0,
        }
    }
}

/// Lê uma imagem como RGBA.
pub fn load_rgba(path: &Path) -> Result<image::RgbaImage, String> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if ext != "png" {
        return Err(format!(
            "A engine lê imagens PNG; este arquivo é .{ext}. Converta para PNG para usar no jogo."
        ));
    }
    image::open(path)
        .map(|i| i.to_rgba8())
        .map_err(|e| format!("Não foi possível abrir a imagem: {e}"))
}

pub fn to_color_image(img: &image::RgbaImage) -> egui::ColorImage {
    egui::ColorImage::from_rgba_unmultiplied(
        [img.width() as usize, img.height() as usize],
        img.as_raw(),
    )
}

impl ImageView {
    /// Lê de novo na próxima vez que a aba aparecer.
    pub fn reload(&mut self) {
        self.needs_load = true;
    }

    fn load(&mut self, ctx: &egui::Context, path: &Path, rel: &str) {
        self.needs_load = false;
        self.bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        match load_rgba(path) {
            Ok(img) => {
                self.size = [img.width() as usize, img.height() as usize];
                self.texture = Some(ctx.load_texture(
                    format!("imagem:{rel}"),
                    to_color_image(&img),
                    egui::TextureOptions::NEAREST,
                ));
                self.error = None;
                // Spritesheet de quadros quadrados: sugere o tamanho do quadro.
                if self.size[0] > self.size[1]
                    && self.size[1] > 0
                    && self.size[0].is_multiple_of(self.size[1])
                {
                    self.frame_size = [self.size[1] as u32; 2];
                }
            }
            Err(e) => {
                self.texture = None;
                self.error = Some(e);
            }
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, path: &Path, rel: &str) -> Option<ViewerAction> {
        if self.needs_load {
            self.load(ui.ctx(), path, rel);
        }
        let mut action = None;
        if let Some(e) = &self.error {
            ui.colored_label(Color32::from_rgb(0xF2, 0x6D, 0x6D), e);
            return None;
        }
        let tex = self.texture.clone()?;
        let img_size = Vec2::new(self.size[0] as f32, self.size[1] as f32);

        ui.horizontal_wrapped(|ui| {
            ui.label(format!(
                "{} × {} px · {}",
                self.size[0],
                self.size[1],
                crate::files::human_size(self.bytes)
            ));
            ui.separator();
            let zoom_now = self.zoom.unwrap_or(0.0);
            if ui
                .button("−")
                .on_hover_text("Diminuir (Ctrl + rolar)")
                .clicked()
            {
                self.zoom = Some(step_zoom(zoom_now.max(0.25), false));
            }
            let label = match self.zoom {
                None => "Ajustado".to_string(),
                Some(z) => format!("{:.0}%", z * 100.0),
            };
            ui.label(label);
            if ui
                .button("+")
                .on_hover_text("Aumentar (Ctrl + rolar)")
                .clicked()
            {
                self.zoom = Some(step_zoom(zoom_now.max(0.25), true));
            }
            if ui
                .selectable_label(self.zoom.is_none(), "Ajustar")
                .clicked()
            {
                self.zoom = None;
            }
            if ui.selectable_label(self.zoom == Some(1.0), "1:1").clicked() {
                self.zoom = Some(1.0);
            }
            ui.separator();
            ui.checkbox(&mut self.frames, "Quadros")
                .on_hover_text("Mostra a divisão em quadros, como no nn.assets.sheet");
            if self.frames {
                ui.add(
                    egui::DragValue::new(&mut self.frame_size[0])
                        .range(1..=4096)
                        .suffix(" px"),
                );
                ui.label("×");
                ui.add(
                    egui::DragValue::new(&mut self.frame_size[1])
                        .range(1..=4096)
                        .suffix(" px"),
                );
            }
            ui.separator();
            let code = if self.frames {
                format!(
                    "nn.assets.sheet(\"{rel}\", {}, {}, {{ pixel_art = true }})",
                    self.frame_size[0], self.frame_size[1]
                )
            } else {
                format!("nn.assets.texture(\"{rel}\", {{ pixel_art = true }})")
            };
            if ui
                .button("📋 Copiar código")
                .on_hover_text(format!("Copia: {code}"))
                .clicked()
            {
                action = Some(ViewerAction::CopyText(code));
            }
        });
        ui.separator();

        let avail = ui.available_size();
        let fit = ((avail.x - 16.0) / img_size.x)
            .min((avail.y - 16.0) / img_size.y)
            .clamp(0.05, 64.0);
        // Pixel art fica mais nítida em zoom inteiro.
        let fit = if fit > 1.0 { fit.floor() } else { fit };
        let zoom = self.zoom.unwrap_or(fit);
        let display = img_size * zoom;

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let canvas = display.max(ui.available_size());
                let (rect, response) = ui.allocate_exact_size(canvas, Sense::hover());
                let img_rect = Rect::from_center_size(rect.center(), display);
                let painter = ui.painter_at(rect);
                checkerboard(&painter, img_rect);
                painter.image(
                    tex.id(),
                    img_rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
                let grid = ui.visuals().text_color().gamma_multiply(0.18);
                if zoom >= 8.0 {
                    pixel_grid(&painter, img_rect, self.size, zoom, Stroke::new(1.0, grid));
                }
                if self.frames {
                    self.frame_overlay(&painter, img_rect, zoom);
                }
                if response.hovered() {
                    if let Some(p) = response.hover_pos()
                        && img_rect.contains(p)
                    {
                        let px = ((p - img_rect.min) / zoom).floor();
                        let mut tip = format!("x {} · y {}", px.x, px.y);
                        if self.frames {
                            let cols = (self.size[0] as u32 / self.frame_size[0].max(1)).max(1);
                            let n = (px.y as u32 / self.frame_size[1].max(1)) * cols
                                + px.x as u32 / self.frame_size[0].max(1)
                                + 1;
                            tip.push_str(&format!(" · quadro {n}"));
                        }
                        response.on_hover_text_at_pointer(tip);
                    }
                    // Ctrl + rolar (ou o gesto de pinça) muda o zoom.
                    let z = ui.input(|i| i.zoom_delta());
                    if !(0.999..=1.001).contains(&z) {
                        self.zoom = Some(step_zoom(zoom, z > 1.0));
                    }
                }
            });
        action
    }

    fn frame_overlay(&self, painter: &egui::Painter, img_rect: Rect, zoom: f32) {
        let [fw, fh] = self.frame_size;
        let (fw, fh) = (fw.max(1) as f32 * zoom, fh.max(1) as f32 * zoom);
        let stroke = Stroke::new(1.0, Color32::from_rgb(0xFF, 0x9F, 0x1C));
        let cols = (img_rect.width() / fw).floor() as usize;
        let rows = (img_rect.height() / fh).floor() as usize;
        for c in 0..=cols {
            let x = img_rect.left() + c as f32 * fw;
            painter.line_segment(
                [
                    Pos2::new(x, img_rect.top()),
                    Pos2::new(x, img_rect.top() + rows as f32 * fh),
                ],
                stroke,
            );
        }
        for r in 0..=rows {
            let y = img_rect.top() + r as f32 * fh;
            painter.line_segment(
                [
                    Pos2::new(img_rect.left(), y),
                    Pos2::new(img_rect.left() + cols as f32 * fw, y),
                ],
                stroke,
            );
        }
        if cols * rows <= 400 && fw >= 14.0 && fh >= 12.0 {
            for r in 0..rows {
                for c in 0..cols {
                    let p = img_rect.min + Vec2::new(c as f32 * fw + 2.0, r as f32 * fh + 1.0);
                    painter.text(
                        p,
                        egui::Align2::LEFT_TOP,
                        (r * cols + c + 1).to_string(),
                        egui::FontId::monospace(10.0),
                        Color32::from_rgb(0xFF, 0xC8, 0x6B),
                    );
                }
            }
        }
    }
}

fn step_zoom(z: f32, up: bool) -> f32 {
    const STEPS: [f32; 13] = [
        0.25, 0.5, 0.75, 1.0, 2.0, 3.0, 4.0, 6.0, 8.0, 12.0, 16.0, 24.0, 32.0,
    ];
    if up {
        STEPS
            .iter()
            .copied()
            .find(|s| *s > z + 0.001)
            .unwrap_or(32.0)
    } else {
        STEPS
            .iter()
            .rev()
            .copied()
            .find(|s| *s < z - 0.001)
            .unwrap_or(0.25)
    }
}

fn checkerboard(painter: &egui::Painter, rect: Rect) {
    let a = Color32::from_gray(0x5A);
    let b = Color32::from_gray(0x46);
    painter.rect_filled(rect, 0.0, a);
    let cell = 8.0;
    let cols = (rect.width() / cell).ceil() as usize;
    let rows = (rect.height() / cell).ceil() as usize;
    if cols * rows > 40_000 {
        return;
    }
    for r in 0..rows {
        for c in 0..cols {
            if (r + c) % 2 == 1 {
                let min = rect.min + Vec2::new(c as f32 * cell, r as f32 * cell);
                let cell_rect = Rect::from_min_size(min, Vec2::splat(cell)).intersect(rect);
                painter.rect_filled(cell_rect, 0.0, b);
            }
        }
    }
}

fn pixel_grid(painter: &egui::Painter, rect: Rect, size: [usize; 2], zoom: f32, stroke: Stroke) {
    if size[0] + size[1] > 2048 {
        return;
    }
    for x in 0..=size[0] {
        let px = rect.left() + x as f32 * zoom;
        painter.line_segment(
            [Pos2::new(px, rect.top()), Pos2::new(px, rect.bottom())],
            stroke,
        );
    }
    for y in 0..=size[1] {
        let py = rect.top() + y as f32 * zoom;
        painter.line_segment(
            [Pos2::new(rect.left(), py), Pos2::new(rect.right(), py)],
            stroke,
        );
    }
}

// ---------------------------------------------------------------------------
// Som

#[derive(Debug, Default)]
pub struct SoundView {
    id: Option<nn_audio::SoundId>,
    duration: f32,
    wave: Vec<(f32, f32)>,
    pub error: Option<String>,
    playing: Option<(nn_audio::Sound, Instant)>,
    needs_load: bool,
    loaded_once: bool,
    bytes: u64,
}

impl SoundView {
    pub fn reload(&mut self) {
        self.needs_load = true;
    }

    fn load(&mut self, audio: &mut nn_audio::Audio, path: &Path) {
        self.needs_load = false;
        self.loaded_once = true;
        self.stop();
        let result = std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                self.bytes = bytes.len() as u64;
                match self.id {
                    Some(id) => audio.replace_bytes(id, bytes).map(|_| id),
                    None => audio.load_bytes(bytes),
                }
                .map_err(|e| e.to_string())
            });
        match result {
            Ok(id) => {
                self.id = Some(id);
                self.duration = audio.duration(id).as_secs_f32();
                self.wave = audio.waveform(id, 600);
                self.error = None;
            }
            Err(e) => self.error = Some(format!("Não foi possível abrir o som: {e}")),
        }
    }

    pub fn is_playing(&self) -> bool {
        self.playing.as_ref().is_some_and(|(s, _)| s.is_playing())
    }

    pub fn play(&mut self, audio: &mut nn_audio::Audio, path: &Path) {
        if self.needs_load || !self.loaded_once {
            self.load(audio, path);
        }
        if let Some(id) = self.id {
            self.stop();
            self.playing = Some((audio.play(id), Instant::now()));
        }
    }

    pub fn stop(&mut self) {
        if let Some((mut s, _)) = self.playing.take() {
            s.stop();
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        audio: &mut nn_audio::Audio,
        path: &Path,
        rel: &str,
    ) -> Option<ViewerAction> {
        if self.needs_load || !self.loaded_once {
            self.load(audio, path);
        }
        let mut action = None;
        if let Some(e) = &self.error {
            ui.colored_label(Color32::from_rgb(0xF2, 0x6D, 0x6D), e);
            return None;
        }
        let playing = self.is_playing();
        ui.horizontal(|ui| {
            if playing {
                if ui.button(RichText::new("⏹ Parar").size(16.0)).clicked() {
                    self.stop();
                }
            } else if ui.button(RichText::new("▶ Tocar").size(16.0)).clicked() {
                self.play(audio, path);
            }
            ui.label(format!(
                "{:.2} s · {}",
                self.duration,
                crate::files::human_size(self.bytes)
            ));
            if !audio.is_enabled() {
                ui.weak("(sem saída de som neste computador)");
            }
            ui.separator();
            let code = format!("nn.assets.sound(\"{rel}\")");
            if ui
                .button("📋 Copiar código")
                .on_hover_text(format!("Copia: {code}"))
                .clicked()
            {
                action = Some(ViewerAction::CopyText(code));
            }
        });
        ui.add_space(8.0);
        let height = 140.0_f32.min(ui.available_height());
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 6.0, ui.visuals().extreme_bg_color);
        let color = ui.visuals().selection.bg_fill;
        let n = self.wave.len().max(1) as f32;
        let mid = rect.center().y;
        let half = rect.height() * 0.45;
        for (i, (lo, hi)) in self.wave.iter().enumerate() {
            let x = rect.left() + (i as f32 + 0.5) / n * rect.width();
            painter.line_segment(
                [
                    Pos2::new(x, mid - hi * half),
                    Pos2::new(x, mid - lo * half + 1.0),
                ],
                Stroke::new((rect.width() / n).max(1.0), color),
            );
        }
        if let Some((_, start)) = &self.playing
            && playing
            && self.duration > 0.0
        {
            let t = (start.elapsed().as_secs_f32() / self.duration).min(1.0);
            let x = rect.left() + t * rect.width();
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(2.0, ui.visuals().strong_text_color()),
            );
            ui.ctx().request_repaint();
        }
        if response.on_hover_text("Clique para tocar").clicked() {
            self.play(audio, path);
        }
        action
    }
}

// ---------------------------------------------------------------------------
// Mapa

/// Prévia de um mapa do Tiled.
#[derive(Debug, Clone)]
pub struct MapPreview {
    pub image: image::RgbaImage,
    pub columns: u32,
    pub rows: u32,
    pub tile_px: [u32; 2],
    pub layers: Vec<String>,
    /// Objetos: (tipo ou nome, retângulo em pixels da imagem, é ponto).
    pub objects: Vec<(String, [f32; 4], bool)>,
}

/// Monta a imagem do mapa (camadas visíveis, sem a camada de colisão).
pub fn compose_map(root: &Path, rel: &str) -> Result<MapPreview, String> {
    let map = nn_tilemap::load_tiled(Path::new(rel), nn_tilemap::read_from(root), 1.0)
        .map_err(|e| e.to_string())?;
    let tw = map.tile_px.x.max(1.0) as u32;
    let th = map.tile_px.y.max(1.0) as u32;
    let (w, h) = (map.width * tw, map.height * th);
    if (w as u64) * (h as u64) > 64_000_000 {
        return Err("Mapa grande demais para a prévia.".into());
    }
    let mut out = image::RgbaImage::new(w, h);
    let sheets: Vec<Option<image::RgbaImage>> = map
        .tilesets
        .iter()
        .map(|ts| load_rgba(&root.join(&ts.image)).ok())
        .collect();
    const MASK: u32 = nn_tilemap::FLIP_H | nn_tilemap::FLIP_V | nn_tilemap::FLIP_D | 0x1000_0000;
    for layer in &map.layers {
        if !layer.visible || layer.properties.contains_key("nn_collision_codes") {
            continue;
        }
        for row in 0..map.height {
            for col in 0..map.width {
                let raw = layer.tiles[(row * map.width + col) as usize];
                let gid = raw & !MASK;
                if gid == 0 {
                    continue;
                }
                let Some((i, ts)) = map
                    .tilesets
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(_, ts)| gid >= ts.first_gid && gid < ts.first_gid + ts.tile_count)
                else {
                    continue;
                };
                let Some(sheet) = &sheets[i] else { continue };
                let local = gid - ts.first_gid;
                let cols = ts.columns.max(1);
                let sx = ts.margin + (local % cols) * (ts.tile_size.x + ts.spacing);
                let sy = ts.margin + (local / cols) * (ts.tile_size.y + ts.spacing);
                // Tiles maiores que a célula ficam alinhados pela base, como no Tiled.
                let dx = col * tw;
                let dy = (row + 1) * th;
                for y in 0..ts.tile_size.y {
                    for x in 0..ts.tile_size.x {
                        let (mut ux, mut uy) = (x, y);
                        if raw & nn_tilemap::FLIP_H != 0 {
                            ux = ts.tile_size.x - 1 - x;
                        }
                        if raw & nn_tilemap::FLIP_V != 0 {
                            uy = ts.tile_size.y - 1 - y;
                        }
                        let (px, py) = (sx + ux, sy + uy);
                        if px >= sheet.width() || py >= sheet.height() {
                            continue;
                        }
                        let src = *sheet.get_pixel(px, py);
                        let alpha = src[3] as f32 / 255.0 * layer.opacity;
                        if alpha <= 0.0 {
                            continue;
                        }
                        let ox = dx + x;
                        let Some(oy) = (dy + y).checked_sub(ts.tile_size.y) else {
                            continue;
                        };
                        if ox >= w || oy >= h {
                            continue;
                        }
                        let dst = out.get_pixel_mut(ox, oy);
                        for c in 0..3 {
                            dst[c] = (src[c] as f32 * alpha + dst[c] as f32 * (1.0 - alpha)) as u8;
                        }
                        dst[3] = (alpha * 255.0 + dst[3] as f32 * (1.0 - alpha)).min(255.0) as u8;
                    }
                }
            }
        }
    }
    let height_px = h as f32;
    let objects = map
        .objects
        .iter()
        .map(|o| {
            let label = if o.kind.is_empty() {
                o.name.clone()
            } else {
                o.kind.clone()
            };
            let cx = o.position.x;
            let cy = height_px - o.position.y;
            (
                [cx - o.size.x / 2.0, cy - o.size.y / 2.0, o.size.x, o.size.y],
                label,
                o.point,
            )
        })
        .map(|(r, l, p)| (l, r, p))
        .collect();
    Ok(MapPreview {
        image: out,
        columns: map.width,
        rows: map.height,
        tile_px: [tw, th],
        layers: map.layers.iter().map(|l| l.name.clone()).collect(),
        objects,
    })
}

pub struct MapView {
    preview: Option<MapPreview>,
    texture: Option<TextureHandle>,
    pub error: Option<String>,
    needs_load: bool,
    zoom: Option<f32>,
    pub show_objects: bool,
    pub show_grid: bool,
}

impl std::fmt::Debug for MapView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MapView")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

impl Default for MapView {
    fn default() -> Self {
        Self {
            preview: None,
            texture: None,
            error: None,
            needs_load: true,
            zoom: None,
            show_objects: true,
            show_grid: false,
        }
    }
}

impl MapView {
    pub fn reload(&mut self) {
        self.needs_load = true;
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, root: &Path, rel: &str) -> Option<ViewerAction> {
        if self.needs_load {
            self.needs_load = false;
            match compose_map(root, rel) {
                Ok(p) => {
                    self.texture = Some(ui.ctx().load_texture(
                        format!("mapa:{rel}"),
                        to_color_image(&p.image),
                        egui::TextureOptions::NEAREST,
                    ));
                    self.preview = Some(p);
                    self.error = None;
                }
                Err(e) => {
                    self.preview = None;
                    self.texture = None;
                    self.error = Some(e);
                }
            }
        }
        let mut action = None;
        ui.horizontal_wrapped(|ui| {
            if let Some(p) = &self.preview {
                ui.label(format!(
                    "{} × {} tiles de {}×{} px · {} camadas · {} objetos",
                    p.columns,
                    p.rows,
                    p.tile_px[0],
                    p.tile_px[1],
                    p.layers.len(),
                    p.objects.len()
                ));
                ui.separator();
                ui.checkbox(&mut self.show_objects, "Objetos");
                ui.checkbox(&mut self.show_grid, "Grade");
                if ui.selectable_label(self.zoom.is_none(), "Ajustar").clicked() {
                    self.zoom = None;
                }
                if ui.selectable_label(self.zoom == Some(1.0), "1:1").clicked() {
                    self.zoom = Some(1.0);
                }
                if ui.selectable_label(self.zoom == Some(2.0), "2:1").clicked() {
                    self.zoom = Some(2.0);
                }
                ui.separator();
            }
            if ui
                .button("✏ Editar no Tiled")
                .on_hover_text("Abre o mapa no programa padrão (o Tiled, se estiver instalado). Ao salvar lá, a prévia e o jogo atualizam.")
                .clicked()
            {
                action = Some(ViewerAction::OpenExternal(rel.to_string()));
            }
            let code = format!("nn.tilemap.load(\"{rel}\")");
            if ui.button("📋 Copiar código").on_hover_text(format!("Copia: {code}")).clicked() {
                action = Some(ViewerAction::CopyText(code));
            }
        });
        ui.separator();
        if let Some(e) = &self.error {
            ui.colored_label(Color32::from_rgb(0xF2, 0x6D, 0x6D), e);
            return action;
        }
        let (Some(p), Some(tex)) = (&self.preview, &self.texture) else {
            return action;
        };
        let size = Vec2::new(p.image.width() as f32, p.image.height() as f32);
        let avail = ui.available_size();
        let fit = ((avail.x - 8.0) / size.x)
            .min((avail.y - 8.0) / size.y)
            .clamp(0.05, 8.0);
        let zoom = self.zoom.unwrap_or(fit);
        let show_objects = self.show_objects;
        let show_grid = self.show_grid;
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let display = size * zoom;
                let (rect, response) =
                    ui.allocate_exact_size(display.max(ui.available_size()), Sense::hover());
                let r = Rect::from_min_size(rect.min, display);
                let painter = ui.painter_at(rect);
                painter.rect_filled(
                    r,
                    0.0,
                    Color32::from_rgb(0x7E, 0xC8, 0xE3).gamma_multiply(0.35),
                );
                painter.image(
                    tex.id(),
                    r,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
                if show_grid {
                    let stroke = Stroke::new(1.0, Color32::from_white_alpha(40));
                    let (tw, th) = (p.tile_px[0] as f32 * zoom, p.tile_px[1] as f32 * zoom);
                    for c in 0..=p.columns {
                        let x = r.left() + c as f32 * tw;
                        painter.line_segment(
                            [Pos2::new(x, r.top()), Pos2::new(x, r.bottom())],
                            stroke,
                        );
                    }
                    for row in 0..=p.rows {
                        let y = r.top() + row as f32 * th;
                        painter.line_segment(
                            [Pos2::new(r.left(), y), Pos2::new(r.right(), y)],
                            stroke,
                        );
                    }
                }
                let mut hovered = None;
                if show_objects {
                    let color = Color32::from_rgb(0xFF, 0x9F, 0x1C);
                    for (label, [x, y, w, h], point) in &p.objects {
                        let min = r.min + Vec2::new(*x, *y) * zoom;
                        let obj = if *point || *w == 0.0 {
                            Rect::from_center_size(
                                min + Vec2::new(*w, *h) * zoom / 2.0,
                                Vec2::splat(8.0),
                            )
                        } else {
                            Rect::from_min_size(min, Vec2::new(*w, *h) * zoom)
                        };
                        painter.rect_stroke(
                            obj,
                            2.0,
                            Stroke::new(1.5, color),
                            egui::StrokeKind::Middle,
                        );
                        if response
                            .hover_pos()
                            .is_some_and(|hp| obj.expand(3.0).contains(hp))
                        {
                            hovered = Some(label.clone());
                        }
                        if zoom >= 2.0 {
                            painter.text(
                                obj.left_top() - Vec2::new(0.0, 2.0),
                                egui::Align2::LEFT_BOTTOM,
                                label,
                                egui::FontId::proportional(11.0),
                                color,
                            );
                        }
                    }
                }
                if let Some(label) = hovered {
                    response.on_hover_text_at_pointer(label);
                }
            });
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prevista_do_mapa_da_fase_1() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jogos/plataforma");
        let p = compose_map(&root, "fases/fase1.tmj").unwrap();
        assert_eq!(p.image.width(), p.columns * p.tile_px[0]);
        assert_eq!(p.image.height(), p.rows * p.tile_px[1]);
        // A fase tem chão: a última linha não é toda transparente.
        let y = p.image.height() - 2;
        let opaque = (0..p.image.width())
            .filter(|&x| p.image.get_pixel(x, y)[3] > 0)
            .count();
        assert!(opaque > p.image.width() as usize / 2, "{opaque}");
        assert!(p.objects.iter().any(|(k, _, _)| k == "moeda"));
        assert!(p.objects.iter().any(|(k, _, _)| k == "lesma"));
        // Objetos dentro da imagem.
        for (_, [x, y, _, _], _) in &p.objects {
            assert!(*x >= -64.0 && *x <= p.image.width() as f32 + 64.0);
            assert!(*y >= -64.0 && *y <= p.image.height() as f32 + 64.0);
        }
        assert!(compose_map(&root, "fases/nao-existe.tmj").is_err());
    }

    #[test]
    fn so_png() {
        let e = load_rgba(Path::new("x.jpg")).unwrap_err();
        assert!(e.contains("PNG"), "{e}");
    }

    #[test]
    fn passos_de_zoom() {
        assert_eq!(step_zoom(1.0, true), 2.0);
        assert_eq!(step_zoom(1.0, false), 0.75);
        assert_eq!(step_zoom(2.7, true), 3.0);
        assert_eq!(step_zoom(32.0, true), 32.0);
        assert_eq!(step_zoom(0.25, false), 0.25);
    }
}
