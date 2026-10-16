//! Aba Cena: a cena desenhada pelo renderer da engine, com seleção,
//! gizmos para mover, girar e escalar, navegação (rolar = zoom, botão do meio
//! ou Espaço + arrastar = mover a vista) e arrastar arquivos para dentro.

use std::path::Path;

use eframe::egui::{self, Color32, Key, Modifiers, Pos2, Rect, Sense, Stroke, Vec2};

use crate::scene_data::{self, Component, History, Object, Prop, SceneData, Transform};
use crate::scene_render::{self, LocalRect, SceneRenderer, Sizes, local_bounds};

/// Arquivo do painel Arquivos sendo arrastado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDrag(pub String);

/// Objeto da Hierarquia sendo arrastado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectDrag(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Move,
    Rotate,
    Scale,
}

/// Mudança feita no editor que o jogo aberto pode aplicar na hora.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveChange {
    pub id: u32,
    /// Índice do componente (a partir de 1, como no Lua).
    pub component: Option<usize>,
    pub field: String,
    pub value: Prop,
}

impl LiveChange {
    pub fn to_json(&self, scene: &str) -> String {
        let mut v = serde_json::json!({
            "scene": scene,
            "id": self.id,
            "field": self.field,
            "value": self.value.to_json(),
        });
        if let Some(c) = self.component {
            v["component"] = serde_json::json!(c);
        }
        v.to_string()
    }
}

/// O que mudou de `before` para `after` que dá para aplicar no jogo sem
/// recarregar (campos de objetos e componentes que já existiam).
pub fn diff(before: &SceneData, after: &SceneData) -> Vec<LiveChange> {
    let mut out = Vec::new();
    scene_data::visit(&after.objects, &mut |o| {
        let Some(b) = before.find(o.id) else { return };
        for key in [
            "name", "x", "y", "rotation", "scale_x", "scale_y", "visible", "layer",
        ] {
            let (old, new) = (b.field(key), o.field(key));
            if old != new
                && let Some(value) = new
            {
                out.push(LiveChange {
                    id: o.id,
                    component: None,
                    field: key.into(),
                    value,
                });
            }
        }
        if b.components.len() != o.components.len() {
            return;
        }
        for (i, (cb, ca)) in b.components.iter().zip(&o.components).enumerate() {
            if cb.kind != ca.kind {
                return;
            }
            for (k, v) in &ca.props {
                if cb.get(k) != Some(v) {
                    out.push(LiveChange {
                        id: o.id,
                        component: Some(i + 1),
                        field: k.clone(),
                        value: v.clone(),
                    });
                }
            }
        }
    });
    out
}

/// Vista do editor: centro (mundo) e zoom (pontos da tela por unidade).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewCamera {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

impl Default for ViewCamera {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 0.5,
        }
    }
}

impl ViewCamera {
    pub fn to_screen(&self, rect: Rect, x: f64, y: f64) -> Pos2 {
        let c = rect.center();
        Pos2::new(
            c.x + ((x - self.x) * self.zoom) as f32,
            c.y - ((y - self.y) * self.zoom) as f32,
        )
    }

    pub fn to_world(&self, rect: Rect, p: Pos2) -> (f64, f64) {
        let c = rect.center();
        (
            self.x + (p.x - c.x) as f64 / self.zoom,
            self.y - (p.y - c.y) as f64 / self.zoom,
        )
    }

    /// Zoom mantendo fixo o ponto do mundo sob `p`.
    pub fn zoom_at(&mut self, rect: Rect, p: Pos2, factor: f64) {
        let before = self.to_world(rect, p);
        self.zoom = (self.zoom * factor).clamp(0.02, 64.0);
        let after = self.to_world(rect, p);
        self.x += before.0 - after.0;
        self.y += before.1 - after.1;
    }
}

#[derive(Debug, Clone)]
enum DragKind {
    /// Mover; `None` = livre, `Some(0)` = só X, `Some(1)` = só Y.
    Move(Option<u8>),
    Rotate {
        start_angle: f64,
    },
    /// Canto ou lado que está sendo puxado (-1, 0 ou 1 em cada eixo).
    Scale {
        handle: (i8, i8),
        rect: LocalRect,
    },
    Pan {
        start: ViewCamera,
    },
    Marquee,
}

#[derive(Debug, Clone)]
struct Drag {
    kind: DragKind,
    start: Pos2,
    /// Objetos e onde estavam no mundo.
    objects: Vec<(u32, Transform)>,
    recorded: bool,
}

/// Uma cena aberta no editor.
#[derive(Debug)]
pub struct SceneDoc {
    pub rel: String,
    pub data: SceneData,
    saved: SceneData,
    pub history: History,
    pub selection: Vec<u32>,
    pub camera: ViewCamera,
    pub tool: Tool,
    pub snap: bool,
    pub grid: f64,
    pub show_colliders: bool,
    /// Erro ao ler o arquivo (a cena fica só para leitura).
    pub error: Option<String>,
    /// O arquivo mudou no disco enquanto havia alterações aqui.
    pub external_change: bool,
    /// Mudanças para mandar ao jogo aberto.
    pub live: Vec<LiveChange>,
    /// Objetos fechados na Hierarquia.
    pub collapsed: std::collections::HashSet<u32>,
    /// Objeto sendo renomeado na Hierarquia.
    pub renaming: Option<(u32, String, bool)>,
    framed: bool,
    drag: Option<Drag>,
    hover: Option<u32>,
    /// Cópia da cena no começo do quadro (para achar o que mudou).
    frame_start: Option<SceneData>,
    /// Onde a vista estava no último quadro.
    view_rect: Option<Rect>,
}

/// O que a aba pede ao espaço de trabalho.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneAction {
    TestHere,
    Reload,
    OpenAsText,
}

/// O que a aba precisa do espaço de trabalho.
pub struct SceneEnv<'a> {
    pub root: &'a Path,
    pub sizes: &'a mut Sizes,
    pub renderer: Option<&'a mut SceneRenderer>,
    /// Tamanho da janela do jogo (config.lua), para a moldura.
    pub design: (f64, f64),
    /// Cor de fundo do projeto (config.lua).
    pub clear_color: &'a str,
    /// O jogo aberto está mostrando esta cena.
    pub connected: bool,
    pub time: f32,
}

impl std::fmt::Debug for SceneEnv<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SceneEnv").finish_non_exhaustive()
    }
}

const GIZMO_LEN: f32 = 64.0;
const HANDLE: f32 = 7.0;
const RED: Color32 = Color32::from_rgb(0xE5, 0x48, 0x4D);
const GREEN: Color32 = Color32::from_rgb(0x46, 0xC2, 0x6A);
const BLUE: Color32 = Color32::from_rgb(0x4A, 0x9C, 0xF0);
const SELECT: Color32 = Color32::from_rgb(0xFF, 0xA9, 0x2E);

impl SceneDoc {
    pub fn open(root: &Path, rel: &str) -> Self {
        let (data, error) = match std::fs::read_to_string(root.join(rel))
            .map_err(|e| e.to_string())
            .and_then(|t| scene_data::read(&t, rel))
        {
            Ok(d) => (d, None),
            Err(e) => (SceneData::default(), Some(e)),
        };
        Self {
            rel: rel.to_string(),
            saved: data.clone(),
            data,
            history: History::default(),
            selection: Vec::new(),
            camera: ViewCamera::default(),
            tool: Tool::Move,
            snap: false,
            grid: 16.0,
            show_colliders: true,
            error,
            external_change: false,
            live: Vec::new(),
            collapsed: Default::default(),
            renaming: None,
            framed: false,
            drag: None,
            hover: None,
            frame_start: None,
            view_rect: None,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.error.is_none() && self.data != self.saved
    }

    pub fn save(&mut self, root: &Path) -> Result<(), String> {
        if let Some(e) = &self.error {
            return Err(format!(
                "a cena tem um erro e não pode ser salva pelo editor: {e}"
            ));
        }
        crate::files::write_atomic(&root.join(&self.rel), &scene_data::write(&self.data))
            .map_err(|e| e.to_string())?;
        self.saved = self.data.clone();
        self.external_change = false;
        Ok(())
    }

    /// Lê de novo do disco (descarta alterações).
    pub fn reload(&mut self, root: &Path) {
        let fresh = Self::open(root, &self.rel);
        self.data = fresh.data;
        self.saved = fresh.saved;
        self.error = fresh.error;
        self.external_change = false;
        self.selection.retain(|id| self.data.find(*id).is_some());
    }

    /// O arquivo mudou no disco.
    pub fn disk_changed(&mut self, root: &Path) {
        let Ok(text) = std::fs::read_to_string(root.join(&self.rel)) else {
            return;
        };
        if scene_data::read(&text, &self.rel).ok().as_ref() == Some(&self.saved) {
            return;
        }
        if self.is_dirty() {
            self.external_change = true;
        } else {
            self.reload(root);
        }
    }

    /// Guarda o estado antes de uma mudança (para desfazer).
    pub fn record(&mut self, key: Option<&str>) {
        let before = self
            .frame_start
            .clone()
            .unwrap_or_else(|| self.data.clone());
        self.history.record(&before, key);
    }

    pub fn undo(&mut self) -> bool {
        let ok = self.history.undo(&mut self.data);
        self.selection.retain(|id| self.data.find(*id).is_some());
        ok
    }

    pub fn redo(&mut self) -> bool {
        let ok = self.history.redo(&mut self.data);
        self.selection.retain(|id| self.data.find(*id).is_some());
        ok
    }

    /// Começo de um quadro: guarda a cena para achar o que mudou no fim.
    pub fn begin_frame(&mut self) {
        self.frame_start = Some(self.data.clone());
    }

    /// Fim do quadro: o que mudou vai para o jogo aberto.
    pub fn end_frame(&mut self) {
        if let Some(before) = self.frame_start.take()
            && before != self.data
        {
            self.live.extend(diff(&before, &self.data));
        }
    }

    /// Selecionados sem os que já vêm junto com um pai selecionado.
    pub fn selection_roots(&self) -> Vec<u32> {
        self.selection
            .iter()
            .copied()
            .filter(|id| {
                !self
                    .selection
                    .iter()
                    .any(|p| *p != *id && self.data.is_ancestor(*p, *id))
            })
            .collect()
    }

    pub fn delete_selection(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        self.record(None);
        for id in self.selection_roots() {
            self.data.remove(id);
        }
        self.selection.clear();
    }

    pub fn duplicate_selection(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        self.record(None);
        let mut new = Vec::new();
        for id in self.selection_roots() {
            if let Some(n) = self.data.duplicate(id) {
                // A cópia aparece um pouco ao lado, para dar para ver.
                if let Some(o) = self.data.find_mut(n) {
                    o.x += self.grid;
                    o.y -= self.grid;
                }
                new.push(n);
            }
        }
        self.selection = new;
    }

    /// Cria um objeto na posição `(x, y)` do mundo (na raiz) e seleciona.
    pub fn add_object(&mut self, name: &str, x: f64, y: f64, components: Vec<Component>) -> u32 {
        self.record(None);
        let id = self.data.next_id();
        let mut o = Object::new(id, name);
        o.x = x.round();
        o.y = y.round();
        o.components = components;
        self.data.objects.push(o);
        self.selection = vec![id];
        id
    }

    /// Centro da vista atual (onde objetos novos aparecem).
    pub fn view_center(&self) -> (f64, f64) {
        (self.camera.x, self.camera.y)
    }

    /// Enquadra a seleção (ou a cena toda).
    pub fn frame_selection(&mut self, rect: Rect, root: &Path, sizes: &mut Sizes) {
        let ids = if self.selection.is_empty() {
            self.data.ids()
        } else {
            self.selection.clone()
        };
        let mut min = (f64::MAX, f64::MAX);
        let mut max = (f64::MIN, f64::MIN);
        for id in ids {
            let (Some(o), Some(t)) = (self.data.find(id), self.data.world_transform(id)) else {
                continue;
            };
            let (r, _) = local_bounds(o, root, sizes, 32.0);
            for (cx, cy) in r.corners() {
                let (wx, wy) = t.point_to_world(cx * t.scale_x, cy * t.scale_y);
                min = (min.0.min(wx), min.1.min(wy));
                max = (max.0.max(wx), max.1.max(wy));
            }
        }
        if min.0 > max.0 {
            self.camera = ViewCamera::default();
            return;
        }
        let w = (max.0 - min.0).max(64.0);
        let h = (max.1 - min.1).max(64.0);
        self.camera.x = (min.0 + max.0) / 2.0;
        self.camera.y = (min.1 + max.1) / 2.0;
        self.camera.zoom = ((rect.width() as f64 * 0.8) / w)
            .min((rect.height() as f64 * 0.8) / h)
            .clamp(0.05, 8.0);
    }

    fn hit(&self, env: &mut SceneEnv, rect: Rect, p: Pos2) -> Option<u32> {
        let (wx, wy) = self.camera.to_world(rect, p);
        let icon = scene_render::ICON_SIZE / self.camera.zoom;
        for placed in scene_render::draw_order(&self.data).iter().rev() {
            let o = placed.object;
            if o.locked {
                continue;
            }
            let t = placed.world;
            let (r, _) = local_bounds(o, env.root, env.sizes, icon);
            let (lx, ly) = t.point_to_local(wx, wy);
            let sx = if t.scale_x.abs() < 1e-9 {
                1e-9
            } else {
                t.scale_x
            };
            let sy = if t.scale_y.abs() < 1e-9 {
                1e-9
            } else {
                t.scale_y
            };
            if r.contains(lx / sx, ly / sy) {
                return Some(o.id);
            }
        }
        None
    }

    /// Cantos do objeto na tela.
    fn screen_corners(
        &self,
        env: &mut SceneEnv,
        rect: Rect,
        id: u32,
    ) -> Option<([Pos2; 4], LocalRect, bool)> {
        let o = self.data.find(id)?;
        let t = self.data.world_transform(id)?;
        let icon = scene_render::ICON_SIZE / self.camera.zoom;
        let (r, is_icon) = local_bounds(o, env.root, env.sizes, icon);
        let corners = r.corners().map(|(cx, cy)| {
            let (wx, wy) = t.point_to_world(cx * t.scale_x, cy * t.scale_y);
            self.camera.to_screen(rect, wx, wy)
        });
        Some((corners, r, is_icon))
    }

    fn set_world_position(&mut self, id: u32, wx: f64, wy: f64) {
        let parent = self.data.parent_of(id).flatten();
        let (lx, ly) = match parent.and_then(|p| self.data.world_transform(p)) {
            Some(pw) => {
                let l = pw.to_local(Transform {
                    x: wx,
                    y: wy,
                    rotation: 0.0,
                    scale_x: 1.0,
                    scale_y: 1.0,
                });
                (l.x, l.y)
            }
            None => (wx, wy),
        };
        if let Some(o) = self.data.find_mut(id) {
            o.x = round2(lx);
            o.y = round2(ly);
        }
    }

    /// Desenha a aba. Devolve pedidos para o espaço de trabalho.
    pub fn ui(&mut self, ui: &mut egui::Ui, env: &mut SceneEnv) -> Vec<SceneAction> {
        let mut actions = Vec::new();
        if let Some(e) = &self.error {
            ui.add_space(12.0);
            ui.colored_label(
                crate::theme::ERROR,
                format!("Não foi possível abrir a cena: {e}"),
            );
            ui.horizontal(|ui| {
                if ui.button("Abrir como texto").clicked() {
                    actions.push(SceneAction::OpenAsText);
                }
                if ui.button("Tentar de novo").clicked() {
                    actions.push(SceneAction::Reload);
                }
            });
            return actions;
        }
        if self.external_change {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(
                    crate::theme::WARNING,
                    "⚠ Este arquivo foi alterado fora do NN Studio.",
                );
                if ui.button("Usar a versão do disco").clicked() {
                    actions.push(SceneAction::Reload);
                }
                if ui.button("Manter a minha").clicked() {
                    self.external_change = false;
                }
            });
        }
        self.toolbar(ui, env, &mut actions);

        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Vista da cena")
        });
        self.view_rect = Some(rect);
        if !self.framed {
            self.framed = true;
            self.frame_selection(rect, env.root, env.sizes);
        }
        let painter = ui.painter_at(rect);

        // A cena, pelo renderer da engine.
        let bg = if self.data.background.is_empty() {
            env.clear_color
        } else {
            self.data.background.as_str()
        };
        match env.renderer.as_deref_mut() {
            Some(renderer) => {
                let ppp = ui.ctx().pixels_per_point();
                let size = glam::UVec2::new(
                    (rect.width() * ppp).round().max(1.0) as u32,
                    (rect.height() * ppp).round().max(1.0) as u32,
                );
                let mut cam = nn_render::Camera2D::new(glam::Vec2::new(
                    self.camera.x as f32,
                    self.camera.y as f32,
                ));
                cam.zoom = self.camera.zoom as f32 * ppp;
                let view = scene_render::RenderView {
                    camera: cam,
                    size,
                    background: scene_render::parse_color(bg),
                    time: env.time,
                };
                let tex = renderer.render(&self.rel, env.root, &self.data, &view);
                painter.image(
                    tex,
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            None => {
                painter.rect_filled(rect, 0.0, scene_render::egui_color(bg));
            }
        }

        self.draw_guides(&painter, rect, env);
        self.handle_input(ui, &response, rect, env, &mut actions);
        self.draw_overlays(&painter, rect, env);

        // Arquivo solto em cima da cena: vira objeto.
        if response.dnd_hover_payload::<FileDrag>().is_some() {
            painter.rect_stroke(
                rect.shrink(2.0),
                4.0,
                Stroke::new(2.0, BLUE),
                egui::StrokeKind::Inside,
            );
        }
        if let Some(payload) = response.dnd_release_payload::<FileDrag>()
            && let Some(p) = response
                .hover_pos()
                .or_else(|| ui.input(|i| i.pointer.latest_pos()))
        {
            let (wx, wy) = self.camera.to_world(rect, p);
            self.drop_file(env, &payload.0, wx, wy);
        }
        actions
    }

    /// Cria o objeto certo para um arquivo solto na cena.
    pub fn drop_file(&mut self, env: &mut SceneEnv, rel: &str, wx: f64, wy: f64) -> Option<u32> {
        let name = Path::new(rel)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "objeto".into());
        let kind = crate::files::FileKind::of(Path::new(rel));
        let comp = match kind {
            crate::files::FileKind::Image => {
                let mut c = Component::new("sprite");
                c.set("image", Prop::Str(rel.into()));
                if let Some((w, h)) = env.sizes.image(env.root, rel) {
                    let (w, h) = (w as f64, h as f64);
                    // Tira de quadros (largura múltipla da altura): um quadro.
                    let (fw, fh) = if w > h && h > 0.0 && (w % h) == 0.0 {
                        (h, h)
                    } else {
                        (w, h)
                    };
                    if fw < w {
                        c.set("frame_w", Prop::Num(fw));
                        c.set("frame_h", Prop::Num(fh));
                    }
                    // Pixel art pequena fica maior para dar para ver.
                    let k = if fw.max(fh) <= 32.0 {
                        (64.0 / fw.max(fh)).floor().max(1.0)
                    } else {
                        1.0
                    };
                    c.set("w", Prop::Num(fw * k));
                    c.set("h", Prop::Num(fh * k));
                }
                c
            }
            crate::files::FileKind::Sound => {
                let mut c = Component::new("sound");
                c.set("sound", Prop::Str(rel.into()));
                c
            }
            crate::files::FileKind::Map if rel.to_lowercase().ends_with(".tmj") => {
                let mut c = Component::new("map");
                c.set("map", Prop::Str(rel.into()));
                c.set("scale", Prop::Num(3.0));
                c
            }
            _ => return None,
        };
        Some(self.add_object(&name, wx, wy, vec![comp]))
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, env: &mut SceneEnv, actions: &mut Vec<SceneAction>) {
        ui.horizontal_wrapped(|ui| {
            if ui
                .add(egui::Button::new("▶ Testar daqui").fill(crate::theme::PLAY.gamma_multiply(0.55)))
                .on_hover_text("Roda o jogo começando nesta cena (F6)")
                .clicked()
            {
                actions.push(SceneAction::TestHere);
            }
            if env.connected {
                ui.colored_label(GREEN, "• Ao vivo").on_hover_text(
                    "O jogo aberto está nesta cena: o que você mudar aparece nele na hora, sem salvar.",
                );
            }
            ui.separator();
            for (tool, label, tip) in [
                (Tool::Move, "✋ Mover", "Mover (W). Arraste o objeto ou as setas."),
                (Tool::Rotate, "⟲ Girar", "Girar (E). Arraste o círculo; com Ctrl, de 15 em 15 graus."),
                (Tool::Scale, "⬈ Escalar", "Escalar (R). Arraste os cantos ou os lados; com Shift, proporcional."),
            ] {
                if ui.selectable_label(self.tool == tool, label).on_hover_text(tip).clicked() {
                    self.tool = tool;
                }
            }
            ui.separator();
            ui.checkbox(&mut self.snap, "Encaixar")
                .on_hover_text("Encaixa na grade ao mover. Segurar Ctrl também encaixa.");
            egui::ComboBox::from_id_salt(("grade", &self.rel))
                .width(56.0)
                .selected_text(format!("{}", self.grid))
                .show_ui(ui, |ui| {
                    for g in [1.0, 4.0, 8.0, 16.0, 32.0, 48.0, 64.0] {
                        ui.selectable_value(&mut self.grid, g, format!("{g}"));
                    }
                });
            ui.checkbox(&mut self.show_colliders, "Colisões");
            ui.separator();
            ui.menu_button("➕ Objeto", |ui| {
                if let Some((name, comps)) = add_object_menu(ui) {
                    let (x, y) = self.view_center();
                    self.add_object(name, x, y, comps);
                    ui.close();
                }
            });
            ui.label(format!("{:.0}%", self.camera.zoom * 100.0))
                .on_hover_text("Zoom: role o mouse. Mover a vista: botão do meio, ou Espaço + arrastar.");
            if ui.button("⛶ Enquadrar").on_hover_text("F").clicked()
                && let Some(rect) = self.view_rect
            {
                self.frame_selection(rect, env.root, env.sizes);
            }
        });
    }

    fn draw_guides(&self, painter: &egui::Painter, rect: Rect, env: &SceneEnv) {
        let cam = &self.camera;
        // Grade (só quando dá para ver as linhas).
        let step_px = self.grid * cam.zoom;
        if self.snap && step_px >= 6.0 {
            let (x0, y1) = cam.to_world(rect, rect.left_top());
            let (x1, y0) = cam.to_world(rect, rect.right_bottom());
            let color = Color32::from_white_alpha(18);
            let mut x = (x0 / self.grid).floor() * self.grid;
            let mut n = 0;
            while x <= x1 && n < 2000 {
                let sx = cam.to_screen(rect, x, 0.0).x;
                painter.line_segment(
                    [Pos2::new(sx, rect.top()), Pos2::new(sx, rect.bottom())],
                    Stroke::new(1.0, color),
                );
                x += self.grid;
                n += 1;
            }
            let mut y = (y0 / self.grid).floor() * self.grid;
            while y <= y1 && n < 4000 {
                let sy = cam.to_screen(rect, 0.0, y).y;
                painter.line_segment(
                    [Pos2::new(rect.left(), sy), Pos2::new(rect.right(), sy)],
                    Stroke::new(1.0, color),
                );
                y += self.grid;
                n += 1;
            }
        }
        // Eixos.
        let o = cam.to_screen(rect, 0.0, 0.0);
        painter.line_segment(
            [Pos2::new(rect.left(), o.y), Pos2::new(rect.right(), o.y)],
            Stroke::new(1.0, RED.gamma_multiply(0.35)),
        );
        painter.line_segment(
            [Pos2::new(o.x, rect.top()), Pos2::new(o.x, rect.bottom())],
            Stroke::new(1.0, GREEN.gamma_multiply(0.35)),
        );
        // Moldura da tela do jogo: onde a câmera começa.
        let (cx, cy, zoom) = self.camera_start();
        let (w, h) = (env.design.0 / zoom, env.design.1 / zoom);
        let a = cam.to_screen(rect, cx - w / 2.0, cy + h / 2.0);
        let b = cam.to_screen(rect, cx + w / 2.0, cy - h / 2.0);
        painter.rect_stroke(
            Rect::from_two_pos(a, b),
            0.0,
            Stroke::new(1.5, Color32::from_white_alpha(120)),
            egui::StrokeKind::Middle,
        );
        painter.text(
            a + Vec2::new(4.0, 2.0),
            egui::Align2::LEFT_TOP,
            "tela do jogo",
            egui::FontId::proportional(11.0),
            Color32::from_white_alpha(150),
        );
    }

    /// Onde a câmera começa: no objeto com Câmera (ou em quem ele segue).
    fn camera_start(&self) -> (f64, f64, f64) {
        let mut out = (0.0, 0.0, 1.0);
        scene_data::visit(&self.data.objects, &mut |o| {
            if out != (0.0, 0.0, 1.0) {
                return;
            }
            if let Some(c) = o.component("camera") {
                let zoom = c.num("zoom").max(0.01);
                let target = c.str("follow");
                let mut pos = self.data.world_transform(o.id).map(|t| (t.x, t.y));
                if !target.is_empty() {
                    scene_data::visit(&self.data.objects, &mut |t| {
                        if t.name == target {
                            pos = self.data.world_transform(t.id).map(|w| (w.x, w.y));
                        }
                    });
                }
                if let Some((x, y)) = pos {
                    out = (x, y, zoom);
                }
            }
        });
        out
    }

    fn draw_overlays(&self, painter: &egui::Painter, rect: Rect, env: &mut SceneEnv) {
        let cam = self.camera;
        let ids: Vec<(u32, Transform)> = scene_render::draw_order(&self.data)
            .iter()
            .map(|p| (p.object.id, p.world))
            .collect();
        for (id, t) in &ids {
            let Some(o) = self.data.find(*id) else {
                continue;
            };
            // Ícone de objetos sem nada visível (vazio, som, câmera, partículas).
            let Some((_, _, is_icon)) = self.screen_corners(env, rect, *id) else {
                continue;
            };
            let center = cam.to_screen(rect, t.x, t.y);
            if is_icon
                || o.components
                    .iter()
                    .any(|c| matches!(c.kind.as_str(), "sound" | "camera" | "particles"))
            {
                let letter = icon_letter(o);
                painter.circle(
                    center,
                    9.0,
                    Color32::from_black_alpha(160),
                    Stroke::new(1.5, Color32::from_white_alpha(200)),
                );
                painter.text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    letter,
                    egui::FontId::proportional(11.0),
                    Color32::WHITE,
                );
            }
            // Colisões.
            if self.show_colliders
                && let Some(b) = o.component("body")
            {
                let color = if b.bool("sensor") {
                    Color32::from_rgb(0xF0, 0xD0, 0x40)
                } else {
                    GREEN
                };
                let stroke = Stroke::new(1.5, color);
                if b.str("shape") == "circle" {
                    let r = b.num("radius") * t.scale_x.abs().max(t.scale_y.abs()) * cam.zoom;
                    painter.circle_stroke(center, r as f32, stroke);
                } else {
                    let (w, h) = (b.num("w") * t.scale_x.abs(), b.num("h") * t.scale_y.abs());
                    let a = cam.to_screen(rect, t.x - w / 2.0, t.y + h / 2.0);
                    let bb = cam.to_screen(rect, t.x + w / 2.0, t.y - h / 2.0);
                    painter.rect_stroke(
                        Rect::from_two_pos(a, bb),
                        0.0,
                        stroke,
                        egui::StrokeKind::Middle,
                    );
                }
            }
        }
        // Passando o mouse.
        if let Some(h) = self.hover
            && !self.selection.contains(&h)
            && let Some((c, _, _)) = self.screen_corners(env, rect, h)
        {
            painter.add(egui::Shape::closed_line(
                c.to_vec(),
                Stroke::new(1.0, SELECT.gamma_multiply(0.6)),
            ));
        }
        // Selecionados.
        for id in &self.selection {
            if let Some((c, _, _)) = self.screen_corners(env, rect, *id) {
                painter.add(egui::Shape::closed_line(
                    c.to_vec(),
                    Stroke::new(1.5, SELECT),
                ));
            }
        }
        // Gizmo no objeto principal.
        if let Some(&id) = self.selection.last()
            && let Some(t) = self.data.world_transform(id)
        {
            let p = cam.to_screen(rect, t.x, t.y);
            match self.tool {
                Tool::Move => {
                    arrow(painter, p, p + Vec2::new(GIZMO_LEN, 0.0), RED);
                    arrow(painter, p, p - Vec2::new(0.0, GIZMO_LEN), GREEN);
                    painter.rect_filled(
                        Rect::from_center_size(p + Vec2::new(12.0, -12.0), Vec2::splat(12.0)),
                        2.0,
                        BLUE.gamma_multiply(0.8),
                    );
                }
                Tool::Rotate => {
                    painter.circle_stroke(p, GIZMO_LEN, Stroke::new(2.0, BLUE));
                    let a = t.rotation.to_radians();
                    let tip = p + Vec2::new(a.cos() as f32, -(a.sin() as f32)) * GIZMO_LEN;
                    painter.line_segment([p, tip], Stroke::new(2.0, BLUE));
                    painter.circle_filled(tip, 5.0, BLUE);
                }
                Tool::Scale => {
                    if let Some((corners, r, _)) = self.screen_corners(env, rect, id) {
                        for (_, pos) in scale_handles(&corners, &r) {
                            painter.rect_filled(
                                Rect::from_center_size(pos, Vec2::splat(HANDLE * 1.6)),
                                1.0,
                                SELECT,
                            );
                        }
                    }
                }
            }
            painter.circle_filled(p, 3.5, Color32::WHITE);
        }
        // Seleção por retângulo.
        if let Some(Drag {
            kind: DragKind::Marquee,
            start,
            ..
        }) = &self.drag
            && let Some(now) = painter.ctx().input(|i| i.pointer.latest_pos())
        {
            let r = Rect::from_two_pos(*start, now);
            painter.rect_filled(r, 0.0, BLUE.gamma_multiply(0.12));
            painter.rect_stroke(r, 0.0, Stroke::new(1.0, BLUE), egui::StrokeKind::Middle);
        }
    }

    /// Qual alça do gizmo está sob o ponteiro.
    fn gizmo_hit(&self, env: &mut SceneEnv, rect: Rect, p: Pos2) -> Option<DragKind> {
        let id = *self.selection.last()?;
        let t = self.data.world_transform(id)?;
        let c = self.camera.to_screen(rect, t.x, t.y);
        match self.tool {
            Tool::Move => {
                if Rect::from_center_size(c + Vec2::new(12.0, -12.0), Vec2::splat(14.0)).contains(p)
                {
                    return Some(DragKind::Move(None));
                }
                if dist_to_segment(p, c, c + Vec2::new(GIZMO_LEN, 0.0)) < HANDLE {
                    return Some(DragKind::Move(Some(0)));
                }
                if dist_to_segment(p, c, c - Vec2::new(0.0, GIZMO_LEN)) < HANDLE {
                    return Some(DragKind::Move(Some(1)));
                }
                None
            }
            Tool::Rotate => {
                let d = (p - c).length();
                ((d - GIZMO_LEN).abs() < HANDLE * 1.5).then(|| DragKind::Rotate {
                    start_angle: angle_of(c, p),
                })
            }
            Tool::Scale => {
                let (corners, r, _) = self.screen_corners(env, rect, id)?;
                scale_handles(&corners, &r)
                    .into_iter()
                    .find(|(_, pos)| (*pos - p).length() < HANDLE * 1.5)
                    .map(|(handle, _)| DragKind::Scale { handle, rect: r })
            }
        }
    }

    fn start_drag(&mut self, kind: DragKind, start: Pos2) {
        let ids = match kind {
            DragKind::Pan { .. } | DragKind::Marquee => Vec::new(),
            _ => self.selection_roots(),
        };
        let objects = ids
            .into_iter()
            .filter_map(|id| self.data.world_transform(id).map(|t| (id, t)))
            .collect();
        self.drag = Some(Drag {
            kind,
            start,
            objects,
            recorded: false,
        });
    }

    fn handle_input(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        rect: Rect,
        env: &mut SceneEnv,
        actions: &mut Vec<SceneAction>,
    ) {
        let pointer = response.hover_pos();
        self.hover = pointer.and_then(|p| self.hit(env, rect, p));
        if let Some(p) = pointer {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.camera
                    .zoom_at(rect, p, (1.0015f64).powf(scroll as f64));
            }
            let zoom = ui.input(|i| i.zoom_delta());
            if zoom != 1.0 {
                self.camera.zoom_at(rect, p, zoom as f64);
            }
        }
        if response.has_focus() {
            // As setas mexem nos objetos, não trocam o foco de lugar.
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    response.id,
                    egui::EventFilter {
                        tab: false,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                    },
                );
            });
        }
        if response.hovered() || response.has_focus() {
            self.keyboard(ui, rect, env, actions);
        }

        let (modifiers, space, middle) =
            ui.input(|i| (i.modifiers, i.key_down(Key::Space), i.pointer.middle_down()));

        if response.drag_started() {
            response.request_focus();
            // O arrasto só "começa" depois de o mouse andar um pouco: o que
            // conta é onde o botão foi apertado.
            let start = ui
                .input(|i| i.pointer.press_origin())
                .or(response.interact_pointer_pos())
                .unwrap_or(rect.center());
            if middle || space || response.dragged_by(egui::PointerButton::Secondary) {
                self.start_drag(DragKind::Pan { start: self.camera }, start);
            } else if let Some(kind) = self.gizmo_hit(env, rect, start) {
                self.start_drag(kind, start);
            } else if let Some(id) = self.hit(env, rect, start) {
                if !self.selection.contains(&id) {
                    if modifiers.shift || modifiers.command {
                        self.selection.push(id);
                    } else {
                        self.selection = vec![id];
                    }
                }
                let kind = match self.tool {
                    Tool::Rotate => {
                        let t = self
                            .data
                            .world_transform(id)
                            .unwrap_or(Transform::of(&Object::new(0, "")));
                        DragKind::Rotate {
                            start_angle: angle_of(self.camera.to_screen(rect, t.x, t.y), start),
                        }
                    }
                    _ => DragKind::Move(None),
                };
                self.start_drag(kind, start);
            } else {
                if !(modifiers.shift || modifiers.command) {
                    self.selection.clear();
                }
                self.start_drag(DragKind::Marquee, start);
            }
        }

        if let (Some(drag), Some(now)) = (self.drag.clone(), response.interact_pointer_pos()) {
            self.continue_drag(&drag, now, rect, modifiers, env);
        }

        if response.drag_stopped() {
            if let Some(Drag {
                kind: DragKind::Marquee,
                start,
                ..
            }) = &self.drag
                && let Some(end) = response.interact_pointer_pos().or(pointer)
            {
                let sel = Rect::from_two_pos(*start, end);
                let ids: Vec<u32> = scene_render::draw_order(&self.data)
                    .iter()
                    .filter(|p| !p.object.locked)
                    .map(|p| p.object.id)
                    .collect();
                for id in ids {
                    if let Some((c, _, _)) = self.screen_corners(env, rect, id)
                        && c.iter().all(|p| sel.contains(*p))
                        && !self.selection.contains(&id)
                    {
                        self.selection.push(id);
                    }
                }
            }
            self.drag = None;
            self.history.seal();
        }

        if response.clicked() {
            response.request_focus();
            let p = response.interact_pointer_pos().unwrap_or(rect.center());
            match self.hit(env, rect, p) {
                Some(id) if modifiers.shift || modifiers.command => {
                    if let Some(i) = self.selection.iter().position(|s| *s == id) {
                        self.selection.remove(i);
                    } else {
                        self.selection.push(id);
                    }
                }
                Some(id) => self.selection = vec![id],
                None if !(modifiers.shift || modifiers.command) => self.selection.clear(),
                None => {}
            }
        }
    }

    fn continue_drag(
        &mut self,
        drag: &Drag,
        now: Pos2,
        rect: Rect,
        modifiers: Modifiers,
        _env: &mut SceneEnv,
    ) {
        let snap = self.snap || modifiers.command;
        match &drag.kind {
            DragKind::Pan { start } => {
                let d = now - drag.start;
                self.camera.x = start.x - d.x as f64 / start.zoom;
                self.camera.y = start.y + d.y as f64 / start.zoom;
            }
            DragKind::Marquee => {}
            DragKind::Move(axis) => {
                if drag.objects.is_empty() {
                    return;
                }
                let (sx, sy) = self.camera.to_world(rect, drag.start);
                let (nx, ny) = self.camera.to_world(rect, now);
                let (mut dx, mut dy) = (nx - sx, ny - sy);
                match axis {
                    Some(0) => dy = 0.0,
                    Some(1) => dx = 0.0,
                    _ => {}
                }
                if dx == 0.0 && dy == 0.0 {
                    return;
                }
                if snap {
                    // Encaixa o objeto principal na grade; os outros acompanham.
                    let (_, t0) = drag.objects[drag.objects.len() - 1];
                    let g = self.grid;
                    dx = ((t0.x + dx) / g).round() * g - t0.x;
                    dy = ((t0.y + dy) / g).round() * g - t0.y;
                    if axis == &Some(0) {
                        dy = 0.0;
                    }
                    if axis == &Some(1) {
                        dx = 0.0;
                    }
                }
                self.record_drag(drag, "mover");
                for (id, t) in &drag.objects {
                    self.set_world_position(*id, t.x + dx, t.y + dy);
                }
            }
            DragKind::Rotate { start_angle } => {
                let Some(&(_, t)) = drag.objects.last() else {
                    return;
                };
                let c = self.camera.to_screen(rect, t.x, t.y);
                let mut delta = angle_of(c, now) - start_angle;
                if snap {
                    delta = ((t.rotation + delta) / 15.0).round() * 15.0 - t.rotation;
                }
                self.record_drag(drag, "girar");
                // Aplica sobre a rotação que cada objeto tinha no começo.
                for (oid, ot) in &drag.objects {
                    let parent_world_rot = self
                        .data
                        .parent_of(*oid)
                        .flatten()
                        .and_then(|p| self.data.world_transform(p))
                        .map(|p| p.rotation)
                        .unwrap_or(0.0);
                    if let Some(o) = self.data.find_mut(*oid) {
                        o.rotation = round2(normalize_deg(ot.rotation + delta - parent_world_rot));
                    }
                }
            }
            DragKind::Scale { handle, rect: r } => {
                let Some(&(id, t)) = drag.objects.last() else {
                    return;
                };
                let (wx, wy) = self.camera.to_world(rect, now);
                let (lx, ly) = t.point_to_local(wx, wy);
                // Ponto da alça no retângulo sem escala.
                let hx = if handle.0 < 0 { r.min.0 } else { r.max.0 };
                let hy = if handle.1 < 0 { r.min.1 } else { r.max.1 };
                let parent_scale = self
                    .data
                    .parent_of(id)
                    .flatten()
                    .and_then(|p| self.data.world_transform(p))
                    .map(|p| (p.scale_x, p.scale_y))
                    .unwrap_or((1.0, 1.0));
                let mut sx = if handle.0 != 0 && hx.abs() > 1e-9 {
                    lx / hx
                } else {
                    t.scale_x
                };
                let mut sy = if handle.1 != 0 && hy.abs() > 1e-9 {
                    ly / hy
                } else {
                    t.scale_y
                };
                if modifiers.shift && handle.0 != 0 && handle.1 != 0 {
                    let k = (sx / t.scale_x).abs().max((sy / t.scale_y).abs());
                    sx = t.scale_x * k;
                    sy = t.scale_y * k;
                }
                if snap {
                    sx = (sx * 10.0).round() / 10.0;
                    sy = (sy * 10.0).round() / 10.0;
                }
                self.record_drag(drag, "escalar");
                if let Some(o) = self.data.find_mut(id) {
                    o.scale_x = round2(sx / parent_scale.0);
                    o.scale_y = round2(sy / parent_scale.1);
                }
            }
        }
    }

    fn record_drag(&mut self, drag: &Drag, key: &str) {
        if !drag.recorded {
            self.history.seal();
            self.record(Some(key));
            if let Some(d) = &mut self.drag {
                d.recorded = true;
            }
        }
    }

    fn keyboard(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        env: &mut SceneEnv,
        actions: &mut Vec<SceneAction>,
    ) {
        if ui.ctx().text_edit_focused() {
            return;
        }
        let pressed =
            |ui: &mut egui::Ui, m: Modifiers, k: Key| ui.input_mut(|i| i.consume_key(m, k));
        if pressed(ui, Modifiers::NONE, Key::W) {
            self.tool = Tool::Move;
        }
        if pressed(ui, Modifiers::NONE, Key::E) {
            self.tool = Tool::Rotate;
        }
        if pressed(ui, Modifiers::NONE, Key::R) {
            self.tool = Tool::Scale;
        }
        if pressed(ui, Modifiers::NONE, Key::F) {
            self.frame_selection(rect, env.root, env.sizes);
        }
        if pressed(ui, Modifiers::NONE, Key::Delete) {
            self.delete_selection();
        }
        if pressed(ui, Modifiers::COMMAND, Key::D) {
            self.duplicate_selection();
        }
        if pressed(ui, Modifiers::NONE, Key::Escape) {
            self.selection.clear();
        }
        if pressed(ui, Modifiers::NONE, Key::F6) {
            actions.push(SceneAction::TestHere);
        }
        // Setas: 1 unidade (10 com Shift).
        let mut nudge = (0.0, 0.0);
        ui.input(|i| {
            // Contando cada aperto: com o editor lento, vários chegam juntos.
            let step = if i.modifiers.shift { 10.0 } else { 1.0 };
            let n = |k: Key| f64::from(i.num_presses(k) as u32) * step;
            nudge.0 += n(Key::ArrowRight) - n(Key::ArrowLeft);
            nudge.1 += n(Key::ArrowUp) - n(Key::ArrowDown);
        });
        if nudge != (0.0, 0.0) && !self.selection.is_empty() {
            self.record(Some("setas"));
            for id in self.selection_roots() {
                if let Some(t) = self.data.world_transform(id) {
                    self.set_world_position(id, t.x + nudge.0, t.y + nudge.1);
                }
            }
        }
    }
}

/// Itens do menu "Adicionar objeto": devolve o nome e os componentes.
pub fn add_object_menu(ui: &mut egui::Ui) -> Option<(&'static str, Vec<Component>)> {
    let items: [(&str, &str, &[&str]); 8] = [
        ("Vazio", "objeto", &[]),
        ("Imagem (sprite)", "sprite", &["sprite"]),
        ("Retângulo", "retangulo", &["shape"]),
        ("Círculo", "circulo", &["shape"]),
        ("Texto", "texto", &["text"]),
        ("Câmera", "camera", &["camera"]),
        ("Som", "som", &["sound"]),
        ("Partículas", "particulas", &["particles"]),
    ];
    for (label, name, comps) in items {
        if ui.button(label).clicked() {
            let mut list: Vec<Component> = comps.iter().map(|k| Component::new(k)).collect();
            if label == "Círculo"
                && let Some(c) = list.first_mut()
            {
                c.set("shape", Prop::Str("circle".into()));
            }
            return Some((name, list));
        }
    }
    None
}

fn icon_letter(o: &Object) -> &'static str {
    for (kind, letter) in [("camera", "C"), ("sound", "♪"), ("particles", "*")] {
        if o.component(kind).is_some() {
            return letter;
        }
    }
    "+"
}

fn arrow(painter: &egui::Painter, from: Pos2, to: Pos2, color: Color32) {
    painter.line_segment([from, to], Stroke::new(2.5, color));
    let dir = (to - from).normalized();
    let side = Vec2::new(-dir.y, dir.x);
    painter.add(egui::Shape::convex_polygon(
        vec![to + dir * 10.0, to + side * 5.0, to - side * 5.0],
        color,
        Stroke::NONE,
    ));
}

fn dist_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

/// Ângulo (graus, anti-horário com Y para cima) de `p` em volta de `c` na tela.
fn angle_of(c: Pos2, p: Pos2) -> f64 {
    let d = p - c;
    (-(d.y as f64)).atan2(d.x as f64).to_degrees()
}

fn normalize_deg(a: f64) -> f64 {
    let mut a = a % 360.0;
    if a > 180.0 {
        a -= 360.0;
    }
    if a <= -180.0 {
        a += 360.0;
    }
    a
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Alças de escala: cantos e meios dos lados, com a direção de cada uma.
fn scale_handles(corners: &[Pos2; 4], _r: &LocalRect) -> Vec<((i8, i8), Pos2)> {
    // Cantos na ordem: (-,-) (+,-) (+,+) (-,+).
    let mid = |a: Pos2, b: Pos2| a + (b - a) * 0.5;
    vec![
        ((-1, -1), corners[0]),
        ((1, -1), corners[1]),
        ((1, 1), corners[2]),
        ((-1, 1), corners[3]),
        ((0, -1), mid(corners[0], corners[1])),
        ((1, 0), mid(corners[1], corners[2])),
        ((0, 1), mid(corners[2], corners[3])),
        ((-1, 0), mid(corners[3], corners[0])),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_ida_e_volta() {
        let rect = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(400.0, 300.0));
        let mut cam = ViewCamera {
            x: 100.0,
            y: 50.0,
            zoom: 2.0,
        };
        let p = cam.to_screen(rect, 110.0, 60.0);
        assert_eq!(p, Pos2::new(210.0 + 20.0, 170.0 - 20.0));
        let (x, y) = cam.to_world(rect, p);
        assert!((x - 110.0).abs() < 1e-9 && (y - 60.0).abs() < 1e-9);
        // Zoom mantém o ponto sob o mouse.
        cam.zoom_at(rect, p, 2.0);
        let (x2, y2) = cam.to_world(rect, p);
        assert!((x2 - 110.0).abs() < 1e-6 && (y2 - 60.0).abs() < 1e-6);
        assert_eq!(cam.zoom, 4.0);
    }

    #[test]
    fn angulos() {
        let c = Pos2::new(0.0, 0.0);
        assert!((angle_of(c, Pos2::new(10.0, 0.0)) - 0.0).abs() < 1e-9);
        // Para cima na tela = 90 graus (Y do mundo para cima).
        assert!((angle_of(c, Pos2::new(0.0, -10.0)) - 90.0).abs() < 1e-9);
        assert_eq!(normalize_deg(270.0), -90.0);
        assert_eq!(normalize_deg(-190.0), 170.0);
    }

    #[test]
    fn diferencas_para_o_jogo() {
        let mut a = SceneData::default();
        let mut o = Object::new(1, "a");
        o.components.push(Component::new("shape"));
        a.objects.push(o);
        let mut b = a.clone();
        b.objects[0].x = 5.0;
        b.objects[0].components[0].set("color", Prop::Str("#FF0000".into()));
        b.objects.push(Object::new(2, "novo"));
        let d = diff(&a, &b);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].field, "x");
        assert_eq!(d[1].component, Some(1));
        let json: serde_json::Value = serde_json::from_str(&d[1].to_json("c.cena")).unwrap();
        assert_eq!(json["scene"], "c.cena");
        assert_eq!(json["component"], 1);
        assert_eq!(json["value"], "#FF0000");
        // A engine aceita a mensagem.
        assert!(nn_script::live::push(&d[0].to_json("teste-diff.cena")));
    }

    #[test]
    fn selecao_sem_filhos_repetidos() {
        let dir = std::env::temp_dir().join(format!("nn-studio-scene-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut doc = SceneDoc::open(&dir, "nao-existe.cena");
        assert!(doc.error.is_some());
        doc.error = None;
        let mut p = Object::new(1, "pai");
        p.children.push(Object::new(2, "filho"));
        doc.data.objects.push(p);
        doc.data.objects.push(Object::new(3, "outro"));
        doc.selection = vec![2, 1, 3];
        assert_eq!(doc.selection_roots(), vec![1, 3]);
        doc.duplicate_selection();
        assert_eq!(doc.selection.len(), 2);
        assert_eq!(doc.data.ids().len(), 6);
        doc.delete_selection();
        assert_eq!(doc.data.ids().len(), 3);
        assert!(doc.undo());
        assert_eq!(doc.data.ids().len(), 6);
        assert!(doc.undo());
        assert_eq!(doc.data.ids().len(), 3);
        // Salvar e reabrir.
        doc.rel = "a.cena".into();
        doc.save(&dir).unwrap();
        assert!(!doc.is_dirty());
        let again = SceneDoc::open(&dir, "a.cena");
        assert_eq!(again.data, doc.data);
    }
}
