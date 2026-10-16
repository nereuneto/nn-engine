//! Painel Tiles: ferramentas de pintar, camadas do mapa, o tileset (para
//! escolher os tiles e marcar quais bloqueiam, o tipo, as animações e os
//! terrenos do autotile).

use std::collections::HashMap;
use std::path::Path;

use eframe::egui::{self, Color32, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, Vec2};

use crate::map_doc::{FLIP_BITS, MapDoc, Stamp, TsKind};
use crate::paint::{PaintState, PaintTool, tileset_data};
use crate::tileset::{TileInfo, TilesetData, TilesetDoc, WangColor, WangSet, corners_of};

/// O que o clique no tileset faz.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mark {
    /// Escolher os tiles do pincel.
    #[default]
    Pick,
    Solid,
    OneWay,
    Kind,
    Anim,
    Terrain,
}

impl Mark {
    const ALL: [Mark; 6] = [
        Mark::Pick,
        Mark::Solid,
        Mark::OneWay,
        Mark::Kind,
        Mark::Anim,
        Mark::Terrain,
    ];

    fn label(self) -> &'static str {
        match self {
            Mark::Pick => "Escolher",
            Mark::Solid => "Sólido",
            Mark::OneWay => "Plataforma",
            Mark::Kind => "Tipo",
            Mark::Anim => "Animação",
            Mark::Terrain => "Terreno",
        }
    }

    fn help(self) -> &'static str {
        match self {
            Mark::Pick => "Clique num tile (ou arraste para pegar vários) para pintar com ele.",
            Mark::Solid => {
                "Clique nos tiles que bloqueiam (chão, paredes). Clique de novo para desmarcar."
            }
            Mark::OneWay => {
                "Plataforma de mão única: só bloqueia quem vem de cima (dá para pular por baixo)."
            }
            Mark::Kind => {
                "Dê um tipo aos tiles (lava, agua, tijolo...). Os eventos usam: \"Encostar num tile lava\"."
            }
            Mark::Anim => {
                "Clique num tile para escolher; com Ctrl+clique, os outros tiles viram os quadros da animação dele (água, tochas, lava)."
            }
            Mark::Terrain => {
                "Escolha um terreno embaixo e clique nos cantos de cada tile para dizer qual terreno está em cada canto. Botão direito limpa o canto."
            }
        }
    }
}

/// O que o painel pede ao espaço de trabalho.
#[derive(Debug, Clone, PartialEq)]
pub enum TilesAction {
    /// Ligar ou desligar o modo Pintar na cena ativa.
    TogglePaint,
    /// Criar um mapa novo na cena ativa.
    NewMap,
    OpenFile(String),
    /// O tileset ganhou um molde de terreno: a imagem mudou no disco.
    ImageChanged(String),
}

/// Nome e cor de um terreno novo.
#[derive(Debug, Clone)]
struct NewTerrain {
    name: String,
    color: [u8; 3],
    /// Contra qual terreno (0 = nada/transparente).
    against: u8,
    /// Num conjunto que já existe (ou num conjunto novo).
    set: Option<usize>,
}

#[derive(Default)]
pub struct TilesPanel {
    textures: crate::map_editor::ImageCache,
    pub mark: Mark,
    kind: String,
    anim_tile: Option<u32>,
    renaming: Option<(usize, String)>,
    pick_start: Option<(u32, u32)>,
    new_terrain: Option<NewTerrain>,
    zoom: f32,
    error: Option<String>,
}

impl std::fmt::Debug for TilesPanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TilesPanel")
            .field("mark", &self.mark)
            .finish_non_exhaustive()
    }
}

/// O que o painel mexe.
pub struct TilesEnv<'a> {
    pub root: &'a Path,
    /// O mapa (arquivo), se houver um escolhido.
    pub map: Option<&'a str>,
    /// O modo Pintar está ligado na cena.
    pub painting: bool,
    pub maps: &'a mut HashMap<String, MapDoc>,
    pub tilesets: &'a mut HashMap<String, TilesetDoc>,
    pub paint: &'a mut PaintState,
}

impl std::fmt::Debug for TilesEnv<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TilesEnv")
            .field("map", &self.map)
            .finish_non_exhaustive()
    }
}

fn hex(c: &str) -> Color32 {
    crate::scene_render::egui_color(c)
}

/// Cores para os terrenos novos.
const TERRAIN_COLORS: [[u8; 3]; 6] = [
    [0x5A, 0xB5, 0x52],
    [0x8A, 0x5A, 0x34],
    [0x3B, 0x8E, 0xD8],
    [0xD8, 0xB0, 0x58],
    [0x9A, 0x9A, 0xA8],
    [0xE0, 0x5A, 0x40],
];

impl TilesPanel {
    /// As imagens dos tilesets (também para quem escolhe tiles fora daqui).
    pub fn images(&mut self) -> &mut crate::map_editor::ImageCache {
        &mut self.textures
    }

    fn texture(
        &mut self,
        ctx: &egui::Context,
        root: &Path,
        rel: &str,
    ) -> Option<(TextureHandle, [u32; 2])> {
        self.textures.get(ctx, root, rel)
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, env: &mut TilesEnv) -> Vec<TilesAction> {
        let mut actions = Vec::new();
        let Some(rel) = env.map.map(str::to_string) else {
            ui.add_space(6.0);
            ui.label(
                "Para pintar tiles, selecione na cena um objeto com Mapa e clique em ✏ Pintar.",
            );
            ui.add_space(4.0);
            let r = ui
                .button("➕ Novo mapa de tiles")
                .on_hover_text("Cria um mapa (.tmj) e põe na cena aberta");
            crate::tutorial::anchor_in(ui, "tiles.novo", r.rect);
            if r.clicked() {
                actions.push(TilesAction::NewMap);
            }
            return actions;
        };
        let Some(map) = env.maps.get(&rel) else {
            ui.weak(format!("Abrindo {rel}..."));
            return actions;
        };
        ui.horizontal_wrapped(|ui| {
            ui.strong(rel.rsplit('/').next().unwrap_or(&rel));
            ui.weak(format!("{} × {} tiles", map.width, map.height));
            let r = ui
                .selectable_label(env.painting, "✏ Pintar")
                .on_hover_text("Liga o modo Pintar na cena (Esc sai)");
            crate::tutorial::anchor_in(ui, "tiles.pintar", r.rect);
            if r.clicked() {
                actions.push(TilesAction::TogglePaint);
            }
        });
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.tools(ui, env, &rel);
                ui.separator();
                self.tileset(ui, env, &rel, &mut actions);
                ui.separator();
                self.layers(ui, env, &rel);
            });
        actions
    }

    fn tools(&mut self, ui: &mut egui::Ui, env: &mut TilesEnv, rel: &str) {
        if let Some(map) = env.maps.get_mut(rel) {
            let layers = map.tile_layers();
            if layers.len() > 1 {
                ui.horizontal(|ui| {
                    ui.label("Camada:");
                    let current = map
                        .tile_layer(map.layer)
                        .map(|l| l.name.clone())
                        .unwrap_or_default();
                    let mut chosen = map.layer;
                    let r = egui::ComboBox::from_id_salt("tiles-camada")
                        .selected_text(current)
                        .show_ui(ui, |ui| {
                            for &i in layers.iter().rev() {
                                let name = map
                                    .tile_layer(i)
                                    .map(|l| l.name.clone())
                                    .unwrap_or_default();
                                ui.selectable_value(&mut chosen, i, name);
                            }
                        });
                    crate::tutorial::anchor_in(ui, "tiles.camada-atual", r.response.rect);
                    map.layer = chosen;
                });
            }
        }
        ui.horizontal_wrapped(|ui| {
            for t in PaintTool::ALL {
                let r = ui
                    .selectable_label(env.paint.tool == t, t.label())
                    .on_hover_text(format!("{} ({:?})\n{}", t.label(), t.key(), t.help()));
                crate::tutorial::anchor_in(ui, &format!("tiles:{}", t.label()), r.rect);
                if r.clicked() {
                    env.paint.tool = t;
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            let s = &env.paint.stamp;
            ui.weak(format!("Pincel: {} × {}", s.w, s.h));
            if ui.small_button("Girar (R)").clicked() {
                env.paint.stamp = env.paint.stamp.rotated();
            }
            if ui
                .small_button("Espelhar (X)")
                .on_hover_text("Shift+X: na vertical")
                .clicked()
            {
                env.paint.stamp = env.paint.stamp.flipped_h();
            }
            ui.checkbox(&mut env.paint.show_collision, "Colisão")
                .on_hover_text("Mostra na cena o que bloqueia (vermelho) e as plataformas (azul)");
            ui.checkbox(&mut env.paint.show_grid, "Grade");
        });
    }

    fn layers(&mut self, ui: &mut egui::Ui, env: &mut TilesEnv, rel: &str) {
        let map = env.maps.get_mut(rel).expect("mapa aberto");
        ui.horizontal(|ui| {
            ui.strong("Camadas");
            let r = ui
                .small_button("+ Camada")
                .on_hover_text("Uma camada nova por cima da atual (enfeites, frente...)");
            crate::tutorial::anchor_in(ui, "tiles.camada", r.rect);
            if r.clicked() {
                map.add_layer("camada");
            }
        });
        // De cima (desenhada por último) para baixo.
        let list = map.tile_layers();
        let mut action: Option<(usize, &str)> = None;
        for &i in list.iter().rev() {
            let l = map.tile_layer(i).cloned().unwrap();
            ui.horizontal(|ui| {
                let mut visible = l.visible;
                if ui.checkbox(&mut visible, "").on_hover_text("Visível").changed() {
                    map.edit_layer(i, |x| x.visible = visible);
                }
                match &mut self.renaming {
                    Some((ri, text)) if *ri == i => {
                        let r = ui.add(egui::TextEdit::singleline(text).desired_width(100.0));
                        if r.lost_focus() {
                            let name = text.trim().to_string();
                            if !name.is_empty() {
                                map.edit_layer(i, |x| x.name = name);
                            }
                            self.renaming = None;
                        } else {
                            r.request_focus();
                        }
                    }
                    _ => {
                        let text = if l.locked {
                            RichText::new(&l.name).weak()
                        } else {
                            RichText::new(&l.name)
                        };
                        let r = ui
                            .selectable_label(map.layer == i, text)
                            .on_hover_text("Clique para pintar nela; clique duas vezes para renomear");
                        if r.clicked() {
                            map.layer = i;
                        }
                        if r.double_clicked() {
                            self.renaming = Some((i, l.name.clone()));
                        }
                    }
                }
                let mut locked = l.locked;
                if ui.checkbox(&mut locked, "trava").on_hover_text("Não deixa pintar nesta camada").changed() {
                    map.edit_layer(i, |x| x.locked = locked);
                }
                let mut col = l.collision;
                if ui
                    .checkbox(&mut col, "colisão")
                    .on_hover_text("Todos os tiles desta camada bloqueiam (bom para uma camada invisível só de colisão)")
                    .changed()
                {
                    map.edit_layer(i, |x| x.collision = col);
                }
            });
        }
        let cur = map.layer;
        ui.horizontal(|ui| {
            if ui
                .small_button("Subir")
                .on_hover_text("Desenha por cima")
                .clicked()
            {
                action = Some((cur, "up"));
            }
            if ui.small_button("Descer").clicked() {
                action = Some((cur, "down"));
            }
            if ui
                .add_enabled(list.len() > 1, egui::Button::new("Excluir").small())
                .clicked()
            {
                action = Some((cur, "del"));
            }
        });
        if let Some(l) = map.tile_layer(cur) {
            let mut op = l.opacity;
            ui.horizontal(|ui| {
                ui.label("Opacidade");
                if ui
                    .add(egui::Slider::new(&mut op, 0.0..=1.0).show_value(false))
                    .changed()
                {
                    map.edit_layer(cur, |x| x.opacity = op);
                }
            });
        }
        match action {
            Some((i, "up")) => map.move_layer(i, true),
            Some((i, "down")) => map.move_layer(i, false),
            Some((i, "del")) => map.remove_layer(i),
            _ => {}
        }
    }

    fn tileset(
        &mut self,
        ui: &mut egui::Ui,
        env: &mut TilesEnv,
        rel: &str,
        actions: &mut Vec<TilesAction>,
    ) {
        let map = env.maps.get(rel).expect("mapa aberto");
        if map.tilesets.is_empty() {
            ui.weak("O mapa não tem tileset.");
            return;
        }
        let n = map.tilesets.len();
        env.paint.tileset = env.paint.tileset.min(n - 1);
        if n > 1 {
            let names: Vec<String> = (0..n)
                .map(|i| match &map.tilesets[i].kind {
                    TsKind::External(r) => r.rsplit('/').next().unwrap_or(r).to_string(),
                    TsKind::Embedded(d) => d.name.clone(),
                })
                .collect();
            egui::ComboBox::from_id_salt("tileset-do-mapa")
                .selected_text(names[env.paint.tileset].clone())
                .show_ui(ui, |ui| {
                    for (i, name) in names.iter().enumerate() {
                        ui.selectable_value(&mut env.paint.tileset, i, name);
                    }
                });
        }
        let ts = env.paint.tileset;
        let first_gid = map.tilesets[ts].first_gid;
        let external = match &map.tilesets[ts].kind {
            TsKind::External(r) => Some(r.clone()),
            TsKind::Embedded(_) => None,
        };
        let Some(data) = tileset_data(map, ts, env.tilesets).cloned() else {
            ui.colored_label(crate::theme::ERROR, "Não foi possível ler o tileset.");
            return;
        };
        self.terrains(ui, env, rel, external.as_deref(), &data, actions);
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.strong("Tileset");
            ui.weak(format!(
                "{} · tiles de {} × {}",
                data.image.rsplit('/').next().unwrap_or(""),
                data.tile_w,
                data.tile_h
            ));
            if ui
                .small_button("Abrir a imagem")
                .on_hover_text("No editor de pixel art")
                .clicked()
            {
                actions.push(TilesAction::OpenFile(data.image.clone()));
            }
        });
        ui.horizontal_wrapped(|ui| {
            for m in Mark::ALL {
                let r = ui
                    .selectable_label(self.mark == m, m.label())
                    .on_hover_text(m.help());
                crate::tutorial::anchor_in(ui, &format!("tiles.marcar:{}", m.label()), r.rect);
                if r.clicked() {
                    self.mark = m;
                }
            }
        });
        ui.weak(self.mark.help());
        if self.mark != Mark::Pick && external.is_none() {
            ui.colored_label(
                crate::theme::WARNING,
                "Este tileset está dentro do mapa (feito no Tiled): para marcar tiles aqui, separe num arquivo .tsj.",
            );
            if ui.button("Separar num arquivo .tsj").clicked() {
                self.separate(env, rel);
            }
            return;
        }
        if self.mark == Mark::Kind {
            ui.horizontal(|ui| {
                ui.label("Tipo:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.kind)
                        .hint_text("lava")
                        .desired_width(100.0),
                );
                for k in data.kinds() {
                    if ui.small_button(&k).clicked() {
                        self.kind = k;
                    }
                }
            });
        }
        let Some((tex, size)) = self.texture(ui.ctx(), env.root, &data.image) else {
            ui.colored_label(
                crate::theme::ERROR,
                format!("Imagem não encontrada: {}", data.image),
            );
            return;
        };
        let changed = self.grid(ui, env, &data, first_gid, external.as_deref(), &tex, size);
        if let (Some(r), Some(d)) = (&external, changed)
            && let Some(doc) = env.tilesets.get_mut(r)
        {
            doc.data = d;
            doc.touch();
        }
        if self.mark == Mark::Anim {
            self.anim_editor(ui, env, external.as_deref(), &data);
        }
    }

    /// A grade do tileset. Devolve o tileset mudado (marcações).
    #[allow(clippy::too_many_arguments)]
    fn grid(
        &mut self,
        ui: &mut egui::Ui,
        env: &mut TilesEnv,
        data: &TilesetData,
        first_gid: u32,
        external: Option<&str>,
        tex: &TextureHandle,
        size: [u32; 2],
    ) -> Option<TilesetData> {
        let avail = ui.available_width().max(60.0);
        if self.zoom <= 0.0 {
            self.zoom = ((avail - 4.0) / size[0] as f32)
                .clamp(1.0, 4.0)
                .floor()
                .max(1.0);
        }
        ui.horizontal(|ui| {
            if ui.small_button("−").clicked() {
                self.zoom = (self.zoom - 1.0).max(1.0);
            }
            ui.weak(format!("{:.0}×", self.zoom));
            if ui.small_button("+").clicked() {
                self.zoom = (self.zoom + 1.0).min(8.0);
            }
        });
        let z = self.zoom;
        let disp = Vec2::new(size[0] as f32 * z, size[1] as f32 * z);
        let mut out = None;
        egui::ScrollArea::both()
            .id_salt("tileset-grade")
            .max_height(320.0)
            .show(ui, |ui| {
                let (rect, resp) = ui.allocate_exact_size(disp, Sense::click_and_drag());
                crate::tutorial::anchor_in(ui, "tiles.tileset", rect);
                let p = ui.painter_at(rect);
                p.rect_filled(rect, 0.0, Color32::from_gray(0x40));
                p.image(
                    tex.id(),
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
                let tile_rect = |id: u32| {
                    let (x, y) = data.tile_origin(id);
                    Rect::from_min_size(
                        rect.min + Vec2::new(x as f32, y as f32) * z,
                        Vec2::new(data.tile_w as f32, data.tile_h as f32) * z,
                    )
                };
                let grid = Stroke::new(1.0, Color32::from_white_alpha(40));
                for id in 0..data.count {
                    p.rect_stroke(tile_rect(id), 0.0, grid, egui::StrokeKind::Inside);
                }
                // Marcações.
                for id in 0..data.count {
                    let r = tile_rect(id);
                    let info = data.info(id);
                    if info.solid && matches!(self.mark, Mark::Solid | Mark::OneWay) {
                        p.rect_filled(
                            r,
                            0.0,
                            Color32::from_rgba_unmultiplied(0xE5, 0x48, 0x4D, 70),
                        );
                    }
                    if info.one_way && matches!(self.mark, Mark::Solid | Mark::OneWay) {
                        p.hline(
                            r.x_range(),
                            r.top() + 1.5,
                            Stroke::new(3.0, Color32::from_rgb(0x4A, 0x9C, 0xF0)),
                        );
                    }
                    if !info.kind.is_empty() && self.mark == Mark::Kind {
                        p.text(
                            r.left_bottom() + Vec2::new(2.0, -1.0),
                            egui::Align2::LEFT_BOTTOM,
                            &info.kind,
                            egui::FontId::proportional(10.0),
                            Color32::WHITE,
                        );
                    }
                    if !info.animation.is_empty() && matches!(self.mark, Mark::Anim | Mark::Pick) {
                        p.text(
                            r.right_top() + Vec2::new(-2.0, 1.0),
                            egui::Align2::RIGHT_TOP,
                            "A",
                            egui::FontId::proportional(11.0),
                            Color32::from_rgb(0xFF, 0xD1, 0x66),
                        );
                    }
                    if self.mark == Mark::Terrain {
                        for set in &data.wangsets {
                            if let Some(c) = set.tiles.get(&id) {
                                let q = r.size() / 2.0;
                                for (k, off) in [
                                    (0, Vec2::ZERO),
                                    (1, Vec2::new(q.x, 0.0)),
                                    (2, q),
                                    (3, Vec2::new(0.0, q.y)),
                                ] {
                                    if c[k] > 0
                                        && let Some(col) = set.colors.get(c[k] as usize - 1)
                                    {
                                        let qr =
                                            Rect::from_min_size(r.min + off, q).shrink(q.x * 0.25);
                                        p.rect_filled(
                                            qr,
                                            1.0,
                                            hex(&col.color).gamma_multiply(0.85),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                if self.mark == Mark::Anim
                    && let Some(t) = self.anim_tile
                {
                    p.rect_stroke(
                        tile_rect(t),
                        0.0,
                        Stroke::new(2.0, Color32::from_rgb(0xFF, 0xD1, 0x66)),
                        egui::StrokeKind::Inside,
                    );
                    for (f, _) in data.info(t).animation {
                        p.rect_stroke(
                            tile_rect(f),
                            0.0,
                            Stroke::new(1.0, Color32::from_rgb(0xFF, 0xD1, 0x66)),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
                // Tiles do pincel.
                let stamp = &env.paint.stamp;
                if self.mark == Mark::Pick {
                    for g in &stamp.tiles {
                        let g = g & !FLIP_BITS;
                        if g >= first_gid && g - first_gid < data.count {
                            p.rect_stroke(
                                tile_rect(g - first_gid),
                                0.0,
                                Stroke::new(2.0, Color32::WHITE),
                                egui::StrokeKind::Inside,
                            );
                        }
                    }
                }
                let at = |pos: Pos2| -> Option<(u32, u32, Vec2)> {
                    let v = (pos - rect.min) / z;
                    if v.x < 0.0 || v.y < 0.0 {
                        return None;
                    }
                    let step = |t: u32| (t + data.spacing) as f32;
                    let col = ((v.x - data.margin as f32) / step(data.tile_w)).floor() as i64;
                    let row = ((v.y - data.margin as f32) / step(data.tile_h)).floor() as i64;
                    if col < 0 || row < 0 || col as u32 >= data.columns || row as u32 >= data.rows()
                    {
                        return None;
                    }
                    let id = row as u32 * data.columns + col as u32;
                    if id >= data.count {
                        return None;
                    }
                    let (x, y) = data.tile_origin(id);
                    let inside = Vec2::new(
                        (v.x - x as f32) / data.tile_w as f32,
                        (v.y - y as f32) / data.tile_h as f32,
                    );
                    Some((col as u32, row as u32, inside))
                };
                if let Some(pos) = resp.hover_pos()
                    && let Some((c, r, _)) = at(pos)
                {
                    let id = r * data.columns + c;
                    let info = data.info(id);
                    let mut tip = format!("tile {}", id + 1);
                    if info.solid {
                        tip.push_str(" · sólido");
                    }
                    if info.one_way {
                        tip.push_str(" · plataforma");
                    }
                    if !info.kind.is_empty() {
                        tip.push_str(&format!(" · tipo {}", info.kind));
                    }
                    resp.clone().on_hover_text_at_pointer(tip);
                }
                match self.mark {
                    Mark::Pick => {
                        let press = resp
                            .interact_pointer_pos()
                            .filter(|_| resp.drag_started() || resp.clicked());
                        if let Some(pos) = press
                            && let Some((c, r, _)) = at(pos)
                            && self.pick_start.is_none()
                        {
                            self.pick_start = Some((c, r));
                        }
                        if let (Some((c0, r0)), Some(pos)) =
                            (self.pick_start, resp.interact_pointer_pos())
                            && let Some((c1, r1, _)) = at(pos)
                        {
                            let (ca, cb) = (c0.min(c1), c0.max(c1));
                            let (ra, rb) = (r0.min(r1), r0.max(r1));
                            let tiles: Vec<u32> = (ra..=rb)
                                .flat_map(|r| (ca..=cb).map(move |c| (r, c)))
                                .map(|(r, c)| first_gid + r * data.columns + c)
                                .collect();
                            env.paint.stamp = Stamp {
                                w: cb - ca + 1,
                                h: rb - ra + 1,
                                tiles,
                            };
                            if !matches!(
                                env.paint.tool,
                                PaintTool::Brush | PaintTool::Rect | PaintTool::Fill
                            ) {
                                env.paint.tool = PaintTool::Brush;
                            }
                        }
                        if resp.drag_stopped()
                            || resp.clicked()
                            || !ui.input(|i| i.pointer.primary_down())
                        {
                            self.pick_start = None;
                        }
                    }
                    _ if external.is_some() => {
                        let clicked = resp.clicked() || resp.secondary_clicked();
                        if clicked
                            && let Some(pos) = resp.interact_pointer_pos()
                            && let Some((c, r, inside)) = at(pos)
                        {
                            let id = r * data.columns + c;
                            let ctrl = ui.input(|i| i.modifiers.command);
                            out = self.mark_tile(
                                env,
                                data,
                                id,
                                inside,
                                resp.secondary_clicked(),
                                ctrl,
                            );
                        }
                    }
                    _ => {}
                }
            });
        out
    }

    /// Clicou num tile no modo de marcar.
    fn mark_tile(
        &mut self,
        env: &mut TilesEnv,
        data: &TilesetData,
        id: u32,
        inside: Vec2,
        secondary: bool,
        ctrl: bool,
    ) -> Option<TilesetData> {
        let mut d = data.clone();
        let mut info = d.info(id);
        match self.mark {
            Mark::Solid => {
                info.solid = !info.solid;
                if info.solid {
                    info.one_way = false;
                }
            }
            Mark::OneWay => {
                info.one_way = !info.one_way;
                if info.one_way {
                    info.solid = false;
                }
            }
            Mark::Kind => {
                let k = self.kind.trim().to_string();
                info.kind = if info.kind == k { String::new() } else { k };
            }
            Mark::Anim => {
                if ctrl && let Some(t) = self.anim_tile {
                    let mut ti = d.info(t);
                    if ti.animation.is_empty() {
                        ti.animation.push((t, 200));
                    }
                    ti.animation.push((id, 200));
                    d.set_info(t, ti);
                    return Some(d);
                }
                self.anim_tile = Some(id);
                return None;
            }
            Mark::Terrain => {
                let Some((set, color)) = env.paint.terrain else {
                    self.error = Some("Escolha um terreno embaixo (ou crie um).".into());
                    return None;
                };
                let corner = match (inside.x >= 0.5, inside.y >= 0.5) {
                    (false, false) => 0,
                    (true, false) => 1,
                    (true, true) => 2,
                    (false, true) => 3,
                };
                let ws = d.wangsets.get_mut(set)?;
                let mut c = ws.tiles.get(&id).copied().unwrap_or([0; 4]);
                c[corner] = if secondary || c[corner] == color {
                    0
                } else {
                    color
                };
                if c == [0; 4] {
                    ws.tiles.remove(&id);
                } else {
                    ws.tiles.insert(id, c);
                }
                return Some(d);
            }
            Mark::Pick => return None,
        }
        d.set_info(id, info);
        Some(d)
    }

    fn anim_editor(
        &mut self,
        ui: &mut egui::Ui,
        env: &mut TilesEnv,
        external: Option<&str>,
        data: &TilesetData,
    ) {
        let Some(t) = self.anim_tile else {
            ui.weak("Clique num tile para editar a animação dele.");
            return;
        };
        let info = data.info(t);
        ui.horizontal_wrapped(|ui| {
            ui.strong(format!("Tile {}", t + 1));
            if info.animation.is_empty() {
                ui.weak("sem animação: Ctrl+clique nos tiles que vêm depois dele.");
            }
        });
        let mut frames = info.animation.clone();
        let mut changed = false;
        let mut remove = None;
        ui.horizontal_wrapped(|ui| {
            for (k, (f, ms)) in frames.iter_mut().enumerate() {
                ui.group(|ui| {
                    ui.label(format!("{}", *f + 1));
                    changed |= ui
                        .add(
                            egui::DragValue::new(ms)
                                .range(16..=10_000)
                                .suffix(" ms")
                                .speed(5.0),
                        )
                        .changed();
                    if ui.small_button("✖").clicked() {
                        remove = Some(k);
                    }
                });
            }
        });
        if let Some(k) = remove {
            frames.remove(k);
            changed = true;
        }
        if changed
            && let Some(r) = external
            && let Some(doc) = env.tilesets.get_mut(r)
        {
            let mut i = doc.data.info(t);
            i.animation = if frames.len() <= 1 {
                Vec::new()
            } else {
                frames
            };
            doc.data.set_info(t, i);
            doc.touch();
        }
    }

    fn terrains(
        &mut self,
        ui: &mut egui::Ui,
        env: &mut TilesEnv,
        rel: &str,
        external: Option<&str>,
        data: &TilesetData,
        actions: &mut Vec<TilesAction>,
    ) {
        ui.horizontal(|ui| {
            ui.strong("Terrenos");
            ui.weak("(autotile)");
        });
        if data.wangsets.is_empty() {
            ui.weak("Nenhum ainda. Um terreno (grama, água, terra) pinta as bordas e os cantos sozinho, com 16 peças.");
        }
        for (si, set) in data.wangsets.iter().enumerate() {
            ui.horizontal_wrapped(|ui| {
                if data.wangsets.len() > 1 {
                    ui.weak(&set.name);
                }
                for (ci, c) in set.colors.iter().enumerate() {
                    let color = (ci + 1) as u8;
                    let on = env.paint.terrain == Some((si, color))
                        && env.paint.tool == PaintTool::Terrain;
                    let r = swatch_button(ui, on, &c.name, hex(&c.color)).on_hover_text(format!(
                        "Pintar {} (T). Peças prontas: {} de 16.",
                        c.name,
                        set.coverage(0, color).max(
                            (1..=set.colors.len() as u8)
                                .filter(|o| *o != color)
                                .map(|o| set.coverage(o, color))
                                .max()
                                .unwrap_or(0)
                        )
                    ));
                    crate::tutorial::anchor_in(ui, &format!("tiles.terreno:{}", c.name), r.rect);
                    if r.clicked() {
                        env.paint.terrain = Some((si, color));
                        // No modo Terreno, escolhe com qual marcar os cantos;
                        // fora dele, é para pintar.
                        if self.mark != Mark::Terrain {
                            self.mark = Mark::Pick;
                            env.paint.tool = PaintTool::Terrain;
                        }
                    }
                }
                let erase =
                    env.paint.terrain == Some((si, 0)) && env.paint.tool == PaintTool::Terrain;
                if ui
                    .selectable_label(erase, "apagar")
                    .on_hover_text("Tira o terreno (as bordas em volta se ajustam)")
                    .clicked()
                {
                    env.paint.terrain = Some((si, 0));
                    env.paint.tool = PaintTool::Terrain;
                    self.mark = Mark::Pick;
                }
            });
        }
        if external.is_none() {
            return;
        }
        match &mut self.new_terrain {
            None => {
                let r = ui
                    .small_button("+ Terreno")
                    .on_hover_text("Um terreno novo, com um molde de 16 peças para desenhar");
                crate::tutorial::anchor_in(ui, "tiles.novo-terreno", r.rect);
                if r.clicked() {
                    let n = data.wangsets.iter().map(|w| w.colors.len()).sum::<usize>();
                    self.new_terrain = Some(NewTerrain {
                        name: if n == 0 {
                            "grama".into()
                        } else {
                            format!("terreno{}", n + 1)
                        },
                        color: TERRAIN_COLORS[n % TERRAIN_COLORS.len()],
                        against: 0,
                        set: if data.wangsets.is_empty() {
                            None
                        } else {
                            Some(0)
                        },
                    });
                }
            }
            Some(nt) => {
                let mut create = false;
                let mut cancel = false;
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Nome");
                        ui.add(egui::TextEdit::singleline(&mut nt.name).desired_width(90.0));
                        ui.color_edit_button_srgb(&mut nt.color);
                    });
                    if let Some(si) = nt.set
                        && let Some(set) = data.wangsets.get(si)
                    {
                        ui.horizontal_wrapped(|ui| {
                            ui.label("Encontra com:");
                            ui.selectable_value(&mut nt.against, 0, "nada (transparente)");
                            for (ci, c) in set.colors.iter().enumerate() {
                                ui.selectable_value(&mut nt.against, (ci + 1) as u8, &c.name);
                            }
                        });
                    }
                    ui.weak("Cria as 16 peças no fim da imagem do tileset, já com os cantos marcados e pintadas como guia. Depois é só desenhar por cima no editor de pixel art.");
                    ui.horizontal(|ui| {
                        let r = ui.button("Criar com molde");
                        crate::tutorial::anchor_in(ui, "tiles.criar-terreno", r.rect);
                        create = r.clicked();
                        cancel = ui.button("Cancelar").clicked();
                    });
                });
                if cancel {
                    self.new_terrain = None;
                }
                if create {
                    let nt = self.new_terrain.take().unwrap();
                    match add_terrain(env, rel, external.unwrap(), &nt) {
                        Ok(image) => actions.push(TilesAction::ImageChanged(image)),
                        Err(e) => self.error = Some(e),
                    }
                }
            }
        }
        if let Some(e) = &self.error {
            ui.colored_label(crate::theme::ERROR, e);
        }
    }

    /// Tileset de dentro do mapa vira um `.tsj` ao lado da imagem.
    fn separate(&mut self, env: &mut TilesEnv, rel: &str) {
        let ts = env.paint.tileset;
        let Some(map) = env.maps.get_mut(rel) else {
            return;
        };
        let TsKind::Embedded(data) = map.tilesets[ts].kind.clone() else {
            return;
        };
        let tsj = crate::tileset::tsj_for(&data.image);
        let mut doc = TilesetDoc::from_data(&tsj, data);
        if let Err(e) = doc.save(env.root) {
            self.error = Some(e);
            return;
        }
        map.checkpoint();
        map.tilesets[ts].kind = TsKind::External(tsj.clone());
        map.touch();
        env.tilesets.insert(tsj, doc);
    }
}

/// Cria um terreno com molde: 16 peças novas no fim da imagem (pintadas
/// com a cor de cada canto) e os cantos já marcados. Devolve a imagem.
fn add_terrain(
    env: &mut TilesEnv,
    _rel: &str,
    tsj: &str,
    nt: &NewTerrain,
) -> Result<String, String> {
    let doc = env.tilesets.get_mut(tsj).ok_or("tileset não está aberto")?;
    let d = &mut doc.data;
    let color = format!("#{:02x}{:02x}{:02x}", nt.color[0], nt.color[1], nt.color[2]);
    let (set_i, new_color, against) = match nt.set {
        Some(si) if si < d.wangsets.len() => {
            d.wangsets[si].colors.push(WangColor {
                name: nt.name.clone(),
                color: color.clone(),
            });
            (si, d.wangsets[si].colors.len() as u8, nt.against)
        }
        _ => {
            d.wangsets.push(WangSet {
                name: "terrenos".into(),
                colors: vec![WangColor {
                    name: nt.name.clone(),
                    color: color.clone(),
                }],
                tiles: Default::default(),
            });
            (d.wangsets.len() - 1, 1, 0)
        }
    };
    // As peças: com "nada", a peça sem nenhum canto não precisa (é o vazio).
    let masks: Vec<u8> = (0..16u8).filter(|m| against != 0 || *m != 0).collect();
    let full = env.root.join(&d.image);
    let mut img = crate::viewers::load_rgba(&full)?;
    let cols = d.columns.max(1);
    let first = d.count;
    let rows_needed = (first + masks.len() as u32).div_ceil(cols);
    let new_h = d.margin * 2 + rows_needed * (d.tile_h + d.spacing)
        - d.spacing.min(rows_needed * (d.tile_h + d.spacing));
    let new_h = new_h.max(img.height());
    let mut bigger = image::RgbaImage::new(img.width(), new_h);
    image::imageops::replace(&mut bigger, &img, 0, 0);
    img = bigger;
    let other_color = if against == 0 {
        None
    } else {
        d.wangsets[set_i]
            .colors
            .get(against as usize - 1)
            .map(|c| crate::scene_render::egui_color(&c.color))
    };
    let ours = Color32::from_rgb(nt.color[0], nt.color[1], nt.color[2]);
    for (k, m) in masks.iter().enumerate() {
        let id = first + k as u32;
        let corners = corners_of(*m, against, new_color);
        let (x0, y0) = d.tile_origin(id);
        // A peça com os 4 cantos é o terreno inteiro (é o que a pessoa pinta);
        // nas outras, o terreno só aparece numa faixa na beira dos cantos dele
        // (a borda: grama por cima, areia, sombra...). Assim o que bloqueia
        // fica igual ao que foi pintado.
        let full = corners.iter().all(|c| *c == new_color);
        let band = (d.tile_w.min(d.tile_h) / 4).max(1);
        let has = |k: usize| corners[k] == new_color;
        for y in 0..d.tile_h {
            for x in 0..d.tile_w {
                let (l, r) = (x < band, x >= d.tile_w - band);
                let (t, b) = (y < band, y >= d.tile_h - band);
                let ours_here = full
                    || (t && has(0) && has(1))
                    || (b && has(3) && has(2))
                    || (l && has(0) && has(3))
                    || (r && has(1) && has(2))
                    || (t && l && has(0))
                    || (t && r && has(1))
                    || (b && r && has(2))
                    || (b && l && has(3));
                let c = if ours_here { Some(ours) } else { other_color };
                if let Some(c) = c {
                    // Borda mais escura, para ver a divisão das peças.
                    let edge = x == 0 || y == 0 || x == d.tile_w - 1 || y == d.tile_h - 1;
                    let c = if edge { c.gamma_multiply(0.7) } else { c };
                    img.put_pixel(x0 + x, y0 + y, image::Rgba([c.r(), c.g(), c.b(), 255]));
                }
            }
        }
        d.wangsets[set_i].tiles.insert(id, corners);
    }
    img.save_with_format(&full, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    d.image_resized(img.width(), img.height());
    doc.touch();
    env.paint.terrain = Some((set_i, new_color));
    env.paint.tool = PaintTool::Terrain;
    Ok(doc.data.image.clone())
}

/// Para quem pinta: as informações de um tile (para mostrar).
pub fn tile_info(map: &MapDoc, tilesets: &HashMap<String, TilesetDoc>, gid: u32) -> TileInfo {
    map.tileset_of(gid)
        .and_then(|(ts, local)| tileset_data(map, ts, tilesets).map(|d| d.info(local)))
        .unwrap_or_default()
}

/// Botão com um quadradinho da cor do terreno.
fn swatch_button(ui: &mut egui::Ui, on: bool, name: &str, color: Color32) -> egui::Response {
    let r = ui.selectable_label(on, format!("    {name}"));
    let sq = Rect::from_center_size(
        Pos2::new(r.rect.left() + 11.0, r.rect.center().y),
        Vec2::splat(10.0),
    );
    ui.painter().rect_filled(sq, 2.0, color);
    ui.painter().rect_stroke(
        sq,
        2.0,
        Stroke::new(1.0, Color32::from_black_alpha(120)),
        egui::StrokeKind::Inside,
    );
    r
}
