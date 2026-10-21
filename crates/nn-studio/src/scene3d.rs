//! A vista 3D da aba Cena (cenas com `mode = "3d"`).
//!
//! Navegação no jeito da Unity e da Godot:
//! - botão direito segurado + WASD (Q/E desce e sobe) voa pela cena;
//! - rodinha aproxima, e com o botão direito muda a velocidade do voo;
//! - Alt + arrastar gira em volta da seleção;
//! - botão do meio arrasta a vista;
//! - F enquadra a seleção.
//!
//! Clicar seleciona (Shift ou Ctrl junta). As alças movem nos eixos (ou no
//! chão, arrastando o objeto), giram em volta dos eixos e escalam. Arquivos
//! `.glb` soltos na vista viram objetos onde o mouse aponta.

use std::path::Path;

use eframe::egui::{self, Color32, Key, Modifiers, Pos2, Rect, Sense, Stroke};
use glam::{Mat4, Quat, Vec2, Vec3};

use super::{FileDrag, SceneAction, SceneDoc, SceneEnv, Tool};
use crate::scene_data::{self, Component, Object, Prop};
use crate::scene_render::{self, Sizes};

const RED: Color32 = Color32::from_rgb(0xE8, 0x4A, 0x4A);
const GREEN: Color32 = Color32::from_rgb(0x5C, 0xC8, 0x5C);
const BLUE: Color32 = Color32::from_rgb(0x4A, 0x82, 0xF0);
const ORANGE: Color32 = Color32::from_rgb(0xFF, 0xA5, 0x2E);

/// A câmera do editor na vista 3D (não é a câmera do jogo).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cam3 {
    pub pos: Vec3,
    /// Graus em volta de Y (0 = olhando para -Z).
    pub yaw: f32,
    /// Graus acima do horizonte.
    pub pitch: f32,
    /// Metros por segundo voando.
    pub speed: f32,
}

impl Default for Cam3 {
    fn default() -> Self {
        let mut c = Self {
            pos: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            speed: 8.0,
        };
        c.look_at(Vec3::new(8.0, 6.0, 10.0), Vec3::ZERO);
        c
    }
}

impl Cam3 {
    pub fn forward(&self) -> Vec3 {
        let (y, p) = (self.yaw.to_radians(), self.pitch.to_radians());
        Vec3::new(-y.sin() * p.cos(), p.sin(), -y.cos() * p.cos())
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize_or(Vec3::X)
    }

    pub fn up(&self) -> Vec3 {
        self.right().cross(self.forward())
    }

    /// Põe a câmera em `pos` olhando para `target`.
    pub fn look_at(&mut self, pos: Vec3, target: Vec3) {
        self.pos = pos;
        let d = (target - pos).normalize_or(Vec3::NEG_Z);
        self.yaw = (-d.x).atan2(-d.z).to_degrees();
        self.pitch = d.y.clamp(-1.0, 1.0).asin().to_degrees();
    }

    pub fn camera(&self) -> nn_render::d3::Camera3D {
        nn_render::d3::Camera3D {
            position: self.pos,
            target: self.pos + self.forward(),
            up: Vec3::Y,
            fov: 60.0,
            near: 0.05,
            far: 2000.0,
        }
    }

    fn rotate(&mut self, dyaw: f32, dpitch: f32) {
        self.yaw = (self.yaw + dyaw).rem_euclid(360.0);
        self.pitch = (self.pitch + dpitch).clamp(-89.0, 89.0);
    }
}

/// Ponto da tela onde um ponto 3D aparece (ou `None`, atrás da câmera).
fn to_screen(cam: &Cam3, rect: Rect, p: Vec3) -> Option<Pos2> {
    let size = Vec2::new(rect.width(), rect.height());
    let s = cam.camera().screen_point(p, size)?;
    Some(rect.min + egui::vec2(s.x, s.y))
}

/// O raio que sai da câmera pelo ponto da tela.
fn ray(cam: &Cam3, rect: Rect, p: Pos2) -> (Vec3, Vec3) {
    let size = Vec2::new(rect.width(), rect.height());
    let local = p - rect.min;
    cam.camera().ray(Vec2::new(local.x, local.y), size)
}

/// Onde o raio corta o plano horizontal na altura `y`.
fn ray_plane(o: Vec3, d: Vec3, y: f32) -> Option<Vec3> {
    if d.y.abs() < 1e-5 {
        return None;
    }
    let t = (y - o.y) / d.y;
    (t > 0.0).then(|| o + d * t)
}

/// Distância ao longo do raio até a caixa (no espaço da caixa, pela matriz),
/// se acertar.
pub fn ray_box(o: Vec3, d: Vec3, m: Mat4, bounds: (Vec3, Vec3)) -> Option<f32> {
    let inv = m.inverse();
    if !inv.is_finite() {
        return None;
    }
    let lo = inv.transform_point3(o);
    let ld = inv.transform_vector3(d);
    let (mut tmin, mut tmax) = (f32::NEG_INFINITY, f32::INFINITY);
    for i in 0..3 {
        if ld[i].abs() < 1e-8 {
            if lo[i] < bounds.0[i] || lo[i] > bounds.1[i] {
                return None;
            }
        } else {
            let a = (bounds.0[i] - lo[i]) / ld[i];
            let b = (bounds.1[i] - lo[i]) / ld[i];
            tmin = tmin.max(a.min(b));
            tmax = tmax.min(a.max(b));
        }
    }
    (tmax >= tmin.max(0.0)).then_some(tmin.max(0.0))
}

/// Parâmetro (ao longo do eixo) do ponto do eixo mais perto do raio.
fn axis_param(o: Vec3, d: Vec3, c: Vec3, a: Vec3) -> Option<f32> {
    let w0 = o - c;
    let b = d.dot(a);
    let denom = 1.0 - b * b;
    if denom.abs() < 1e-4 {
        return None;
    }
    let e = d.dot(w0);
    let f = a.dot(w0);
    Some((f - b * e) / denom)
}

fn dist_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
    (a + ab * t - p).length()
}

fn snap(v: f32, step: f32) -> f32 {
    if step > 0.0 {
        (v / step).round() * step
    } else {
        v
    }
}

/// O que está sendo arrastado na vista 3D.
#[derive(Debug, Clone)]
pub enum Drag3Kind {
    /// Mover ao longo de um eixo do mundo.
    Axis {
        axis: Vec3,
        s0: f32,
    },
    /// Mover no chão (na altura em que o objeto estava).
    Plane {
        y: f32,
        hit0: Vec3,
    },
    /// Girar em volta de um eixo do mundo.
    Rotate {
        axis: Vec3,
        a0: f32,
        center: Pos2,
    },
    /// Escalar num eixo do objeto (0, 1, 2) ou em todos (3).
    Scale {
        axis: usize,
        s0: f32,
        len: f32,
    },
    Look,
    Orbit {
        pivot: Vec3,
    },
    Pan {
        dist: f32,
    },
    /// Arrastar objetos do HUD na tela do jogo: de onde o mouse saiu (no
    /// mundo 2D do HUD) e onde cada um estava.
    Screen {
        from: (f64, f64),
        starts: Vec<(u32, f64, f64)>,
    },
}

#[derive(Debug, Clone)]
pub struct Drag3 {
    kind: Drag3Kind,
    start: Pos2,
    /// Os objetos e as matrizes deles no mundo no começo.
    objects: Vec<(u32, Mat4)>,
    center: Vec3,
    recorded: bool,
}

impl SceneDoc {
    /// Matriz no mundo do objeto (a da raiz para objetos de prefab).
    fn matrix3(&self, id: u32) -> Option<Mat4> {
        self.data.world_matrix3(id)
    }

    /// O centro da seleção (o último selecionado) no mundo. `None` também
    /// quando ele é do HUD (não está no mundo 3D).
    fn selection_center3(&self) -> Option<Vec3> {
        let id = *self.selection.last()?;
        if self.on_screen3(id) {
            return None;
        }
        Some(self.matrix3(id)?.transform_point3(Vec3::ZERO))
    }

    /// Preso à tela (HUD, menus): ele ou um dos pais. Na cena 3D, esses
    /// objetos não estão no mundo; aparecem por cima da vista.
    fn on_screen3(&self, id: u32) -> bool {
        let mut cur = Some(id);
        while let Some(i) = cur {
            let Some(o) = self.data.find(i) else {
                return false;
            };
            let parent = self.data.parent_of(i).flatten();
            if o.is_hud() || (parent.is_none() && self.data.is_screen(o)) {
                return true;
            }
            cur = parent;
        }
        false
    }

    /// A cena tem algo preso à tela?
    fn has_screen3(&self) -> bool {
        self.data.objects.iter().any(|o| self.data.is_screen(o))
    }

    /// A câmera 2D que põe a tela do jogo inteira dentro da vista (para o
    /// HUD que aparece por cima do mundo).
    fn hud_camera(&self, rect: Rect) -> super::ViewCamera {
        let (cx, cy, zoom) = self.data.camera_start();
        let (sw, sh) = scene_data::screen_size();
        let fit = (f64::from(rect.width()) * zoom / sw).min(f64::from(rect.height()) * zoom / sh);
        super::ViewCamera {
            x: cx,
            y: cy,
            zoom: fit.max(1e-3),
        }
    }

    /// O objeto do HUD debaixo do mouse (com o HUD à mostra).
    fn hit_screen3(&self, env: &mut SceneEnv, rect: Rect, p: Pos2) -> Option<u32> {
        if !self.show_hud3 {
            return None;
        }
        let list = scene_render::screen_order(&self.data);
        self.hit_among(env, rect, p, &list)
    }

    /// Para onde girar e aproximar: a seleção ou um ponto à frente.
    fn pivot3(&self) -> Vec3 {
        self.selection_center3()
            .unwrap_or(self.cam3.pos + self.cam3.forward() * 10.0)
    }

    /// O terreno selecionado: id, índice do componente e onde ele está no
    /// mundo (o terreno só usa a posição).
    pub fn selected_terrain(&self) -> Option<(u32, usize, Vec3)> {
        let [id] = self.selection[..] else {
            return None;
        };
        let o = self.data.find(id)?;
        let ci = o.components.iter().position(|c| c.kind == "terrain")?;
        Some((id, ci, self.matrix3(id)?.transform_point3(Vec3::ZERO)))
    }

    /// Onde o raio (no mundo) bate no terreno selecionado.
    fn terrain_hit(&self, o: Vec3, d: Vec3) -> Option<(nn_render::d3::TerrainData, Vec3, Vec3)> {
        let (id, ci, origin) = self.selected_terrain()?;
        let c = self.data.find(id)?.components.get(ci)?;
        let t = crate::terrain_tool::terrain_of(c);
        let hit = t.raycast(o - origin, d, 2000.0)? + origin;
        Some((t, origin, hit))
    }

    /// Um quadro do pincel do terreno em `p` (botão segurado).
    fn terrain_stroke(&mut self, rect: Rect, p: Pos2, dt: f32, invert: bool) {
        use crate::terrain_tool::{BrushResult, apply};
        let (ro, rd) = ray(&self.cam3, rect, p);
        let Some((t, origin, hit)) = self.terrain_hit(ro, rd) else {
            return;
        };
        let Some((id, ci, _)) = self.selected_terrain() else {
            return;
        };
        // Os já espalhados com o mesmo arquivo (para não amontoar).
        let file = self.terrain_tool.scatter.clone();
        let mut placed = Vec::new();
        scene_data::visit(&self.data.objects, &mut |o| {
            let same = o.prefab.as_ref().is_some_and(|l| l.path == file)
                || o.component("model").is_some_and(|m| m.str("file") == file);
            if same && !file.is_empty() {
                placed.push((o.id, Vec3::new(o.x as f32, o.y as f32, o.z as f32)));
            }
        });
        let key = format!("terreno:{}", self.terrain_tool.stroke);
        match apply(&mut self.terrain_tool, &t, origin, hit, dt, invert, &placed) {
            BrushResult::Nothing => {}
            BrushResult::Terrain(new) => {
                self.record(Some(&key));
                if let Some(c) = self
                    .data
                    .find_mut(id)
                    .and_then(|o| o.components.get_mut(ci))
                {
                    c.set("data", Prop::Str(new.encode()));
                }
            }
            BrushResult::Scatter { add, remove } => {
                if add.is_empty() && remove.is_empty() {
                    return;
                }
                self.record(Some(&key));
                for r in remove {
                    self.data.remove(r);
                }
                for mut o in add {
                    o.id = self.data.next_id();
                    self.data.objects.push(o);
                }
            }
        }
    }

    /// O objeto (selecionável) que o raio acerta primeiro.
    fn pick3(&self, root: &Path, sizes: &mut Sizes, o: Vec3, d: Vec3) -> Option<(u32, f32)> {
        let mut best: Option<(u32, f32)> = None;
        let mut ids = Vec::new();
        scene_data::visit(&self.data.objects, &mut |obj| {
            if obj.visible && !obj.locked {
                ids.push(obj.id);
            }
        });
        ids.retain(|id| !self.on_screen3(*id));
        for id in ids {
            // Invisível por causa de um pai: não acerta.
            let mut shown = true;
            let mut cur = self.data.parent_of(id).flatten();
            while let Some(p) = cur {
                if self.data.find(p).is_some_and(|x| !x.visible) {
                    shown = false;
                }
                cur = self.data.parent_of(p).flatten();
            }
            if !shown {
                continue;
            }
            let (Some(obj), Some(m)) = (self.data.find(id), self.matrix3(id)) else {
                continue;
            };
            let b = scene_render::bounds3(obj, root, sizes);
            if let Some(t) = ray_box(o, d, m, b)
                && best.is_none_or(|(_, bt)| t < bt)
            {
                best = Some((id, t));
            }
        }
        best
    }

    /// Enquadra a seleção (ou a cena toda).
    pub fn frame3(&mut self, root: &Path, sizes: &mut Sizes) {
        let ids: Vec<u32> = if self.selection.is_empty() {
            let mut all = Vec::new();
            scene_data::visit(&self.data.objects, &mut |o| {
                if o.component("camera3d").is_none() {
                    all.push(o.id);
                }
            });
            all
        } else {
            self.selection.clone()
        };
        // O HUD não está no mundo 3D (as posições dele são pixels da tela).
        let ids: Vec<u32> = ids.into_iter().filter(|id| !self.on_screen3(*id)).collect();
        let mut lo = Vec3::splat(f32::MAX);
        let mut hi = Vec3::splat(f32::MIN);
        for id in ids {
            let (Some(o), Some(m)) = (self.data.find(id), self.matrix3(id)) else {
                continue;
            };
            let (a, b) = scene_render::bounds3(o, root, sizes);
            for i in 0..8 {
                let c = Vec3::new(
                    if i & 1 == 0 { a.x } else { b.x },
                    if i & 2 == 0 { a.y } else { b.y },
                    if i & 4 == 0 { a.z } else { b.z },
                );
                let w = m.transform_point3(c);
                lo = lo.min(w);
                hi = hi.max(w);
            }
        }
        if lo.x > hi.x {
            self.cam3 = Cam3::default();
            return;
        }
        let center = (lo + hi) / 2.0;
        let radius = ((hi - lo).length() / 2.0).max(0.5);
        let dist = radius * 1.6 + 1.0;
        // De longe, a névoa da cena esconderia tudo: a vista desliga a dela
        // (o botão Névoa liga de novo).
        let far = self.data.env("fog_far").num() as f32;
        if self.data.env("fog").bool() && dist + radius * 0.5 > far {
            self.show_fog3 = false;
        }
        let dir = if self.cam3.forward().y > -0.2 {
            Vec3::new(-0.55, -0.45, -0.7).normalize()
        } else {
            self.cam3.forward()
        };
        let speed = self.cam3.speed;
        self.cam3.look_at(center - dir * dist, center);
        self.cam3.speed = speed;
    }

    /// Põe na cena um objeto do menu ➕: os do mundo onde a câmera olha (em
    /// pé no chão), os do HUD no meio da tela do jogo.
    pub fn add_from_menu3(&mut self, o: Object) -> u32 {
        if self.data.is_screen(&o) {
            let (x, y) = self.view_center();
            return self.add_prepared(o, x, y);
        }
        let p = self.spawn_point3();
        // Formas em pé no chão (a origem delas é o meio).
        let lift = if o
            .component("shape3d")
            .is_some_and(|c| c.str("shape") != "plane")
        {
            0.5 * o.scale_y as f32
        } else if o.component("text3d").is_some() {
            1.5
        } else {
            0.0
        };
        self.add_object3(o, p + Vec3::Y * lift)
    }

    /// Ao abrir a cena: olha para o Jogador 3D de perto, como no começo do
    /// jogo (numa cena grande, enquadrar tudo deixaria tudo pequeno e na
    /// névoa). Devolve `false` se a cena não tem Jogador 3D.
    fn frame_player3(&mut self) -> bool {
        let mut player = None;
        scene_data::visit(&self.data.objects, &mut |o| {
            if player.is_none()
                && o.components
                    .iter()
                    .any(|c| c.is_behavior() && c.str("script") == "nn:jogador3d")
            {
                player = Some(o.id);
            }
        });
        let Some(m) = player.and_then(|id| self.matrix3(id)) else {
            return false;
        };
        let center = m.transform_point3(Vec3::ZERO) + Vec3::Y;
        let dir = Vec3::new(-0.55, -0.45, -0.7).normalize();
        let speed = self.cam3.speed;
        self.cam3.look_at(center - dir * 12.0, center);
        self.cam3.speed = speed;
        true
    }

    /// Cria um objeto 3D em `pos` (na raiz, ou dentro do prefab) e seleciona.
    pub fn add_object3(&mut self, mut o: Object, pos: Vec3) -> u32 {
        self.record(None);
        let id = self.data.next_id();
        fn number(o: &mut Object, next: &mut u32) {
            o.id = *next;
            *next += 1;
            for c in &mut o.children {
                number(c, next);
            }
        }
        let mut next = id;
        number(&mut o, &mut next);
        let r = |v: f32| (f64::from(v) * 100.0).round() / 100.0;
        (o.x, o.y, o.z) = (r(pos.x), r(pos.y), r(pos.z));
        match self.prefab_root() {
            Some(root) => {
                let n = self.data.find(root).map(|x| x.children.len()).unwrap_or(0);
                self.data.insert(Some(root), n, o);
                // Dentro do prefab, a posição dada é a do mundo.
                self.data.set_world_matrix3(id, Mat4::from_translation(pos));
            }
            None => self.data.objects.push(o),
        }
        self.selection = vec![id];
        id
    }

    /// Onde pôr um objeto novo: o chão à frente da câmera.
    pub fn spawn_point3(&self) -> Vec3 {
        let o = self.cam3.pos;
        let d = self.cam3.forward();
        match ray_plane(o, d, 0.0) {
            Some(p) if (p - o).length() < 40.0 => p,
            _ => {
                let p = o + d * 8.0;
                Vec3::new(p.x, p.y.max(0.0), p.z)
            }
        }
    }

    /// Cria o objeto certo para um arquivo solto na vista 3D, em `pos`.
    pub fn drop_file3(&mut self, env: &mut SceneEnv, rel: &str, pos: Vec3) -> Option<u32> {
        let name = Path::new(rel)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "objeto".into());
        let kind = crate::files::FileKind::of(Path::new(rel));
        let mut o = Object::new(0, &name);
        match kind {
            crate::files::FileKind::Model => {
                let mut c = Component::new("model");
                c.set("file", Prop::Str(rel.into()));
                o.components.push(c);
                // Apoiado no chão: a base do modelo na altura do ponto.
                let lift = env
                    .sizes
                    .model(env.root, rel)
                    .map(|(lo, _)| -lo.y)
                    .unwrap_or(0.0);
                return Some(self.add_object3(o, pos + Vec3::Y * lift));
            }
            crate::files::FileKind::Sound => {
                let mut c = Component::new("sound");
                c.set("sound", Prop::Str(rel.into()));
                o.components.push(c);
            }
            crate::files::FileKind::Prefab => {
                if rel == self.rel {
                    return None;
                }
                let base = scene_data::load_prefab(env.root, rel).ok()?;
                let mut p = Object::new(0, &base.name);
                p.prefab = Some(scene_data::PrefabLink {
                    path: rel.to_string(),
                    base: None,
                    raw: Vec::new(),
                });
                p.link_prefab(base);
                return Some(self.add_object3(p, pos));
            }
            _ => return None,
        }
        Some(self.add_object3(o, pos))
    }

    pub(super) fn ui_3d(&mut self, ui: &mut egui::Ui, env: &mut SceneEnv) -> Vec<SceneAction> {
        let mut actions = Vec::new();
        self.toolbar3d(ui, env, &mut actions);
        self.terrain_bar(ui, env);

        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Vista da cena 3D")
        });
        self.view_rect = Some(rect);
        // A câmera 2D do HUD (para desenhar, clicar e arrastar o HUD).
        self.camera = self.hud_camera(rect);
        crate::tutorial::anchor(ui.ctx(), "cena", rect);
        if !self.framed {
            self.framed = true;
            if !self.cam3_set && !self.frame_player3() {
                self.frame3(env.root, env.sizes);
            }
        }
        let painter = ui.painter_at(rect);

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
                // Animações na prévia: a vista se redesenha enquanto houver.
                let mut animated = false;
                scene_data::visit(&self.data.objects, &mut |o| {
                    animated |= (o.component("model").is_some()
                        && scene_render::preview_anim3d(o).is_some())
                        || o.component("particles").is_some();
                });
                if animated {
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(33));
                }
                let view = scene_render::View3 {
                    camera: self.cam3.camera(),
                    size,
                    background: scene_render::parse_color(bg),
                    grid: if self.show_grid3 {
                        self.grid3 as f32
                    } else {
                        0.0
                    },
                    time: ui.input(|i| i.time) as f32,
                    bursts: std::mem::take(&mut self.burst_requests),
                    fog: self.show_fog3,
                    hud: (self.show_hud3 && self.has_screen3()).then(|| {
                        let mut c = nn_render::Camera2D::new(Vec2::new(
                            self.camera.x as f32,
                            self.camera.y as f32,
                        ));
                        c.zoom = self.camera.zoom as f32 * ppp;
                        c
                    }),
                };
                let tex = renderer.render3d(&self.rel, env.root, &self.data, &view);
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

        self.input3(ui, &response, rect, env, &mut actions);
        self.overlays3(&painter, rect, env);

        // Arquivo solto em cima da vista: vira objeto onde o mouse aponta.
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
            let (o, d) = ray(&self.cam3, rect, p);
            let hit = self
                .pick3(env.root, env.sizes, o, d)
                .map(|(_, t)| o + d * t)
                .or_else(|| ray_plane(o, d, 0.0))
                .unwrap_or_else(|| self.spawn_point3());
            self.drop_file3(env, &payload.0, hit);
        }
        actions
    }

    /// A linha dos pincéis do terreno (com o ✏ ligado).
    fn terrain_bar(&mut self, ui: &mut egui::Ui, env: &mut SceneEnv) {
        use crate::terrain_tool::TerrainBrush;
        if !(self.terrain_tool.active && self.selected_terrain().is_some()) {
            return;
        }
        let colors = self
            .selected_terrain()
            .and_then(|(id, ci, _)| self.data.find(id)?.components.get(ci).cloned())
            .map(|c| crate::terrain_tool::colors_of(&c));
        ui.horizontal_wrapped(|ui| {
            for b in TerrainBrush::ALL {
                let r = ui
                    .selectable_label(self.terrain_tool.brush == b, b.label())
                    .on_hover_text(b.tip());
                crate::tutorial::anchor(ui.ctx(), &format!("terreno:{b:?}"), r.rect);
                if r.clicked() {
                    self.terrain_tool.brush = b;
                }
            }
            ui.separator();
            // Cada nome junto do seu valor (não quebram a linha no meio).
            if ui.available_size_before_wrap().x < 120.0 {
                ui.end_row();
            }
            ui.horizontal(|ui| {
                ui.label("Tamanho");
                ui.add(
                    egui::DragValue::new(&mut self.terrain_tool.radius)
                        .range(0.5..=60.0)
                        .speed(0.1)
                        .suffix(" m"),
                )
                .on_hover_text("O raio do pincel");
            });
            if ui.available_size_before_wrap().x < 100.0 {
                ui.end_row();
            }
            ui.horizontal(|ui| {
                ui.label("Força");
                ui.add(
                    egui::DragValue::new(&mut self.terrain_tool.strength)
                        .range(0.1..=30.0)
                        .speed(0.05),
                )
                .on_hover_text("Quanto muda por segundo (Ctrl faz o contrário)");
            });
            match self.terrain_tool.brush {
                TerrainBrush::Paint => {
                    ui.separator();
                    let names = ["Grama", "Terra", "Pedra", "Areia"];
                    for (k, name) in names.iter().enumerate() {
                        let color = colors.map(|c| c[k]).unwrap_or(nn_core::Color::WHITE);
                        let c32 = Color32::from_rgb(
                            (color.r * 255.0) as u8,
                            (color.g * 255.0) as u8,
                            (color.b * 255.0) as u8,
                        );
                        let text = egui::RichText::new(format!("■ {} {name}", k + 1)).color(c32);
                        let r = ui
                            .selectable_label(self.terrain_tool.layer == k, text)
                            .on_hover_text("A cor fica no Inspetor (Cor 1 a 4)");
                        crate::tutorial::anchor(ui.ctx(), &format!("terreno:cor{}", k + 1), r.rect);
                        if r.clicked() {
                            self.terrain_tool.layer = k;
                        }
                    }
                }
                TerrainBrush::Scatter => {
                    ui.separator();
                    let root = env.root.to_path_buf();
                    let shown = if self.terrain_tool.scatter.is_empty() {
                        "(escolha)".to_string()
                    } else {
                        stem_of(&self.terrain_tool.scatter)
                    };
                    let r = egui::ComboBox::from_id_salt(("espalhar", &self.rel))
                        .selected_text(shown)
                        .show_ui(ui, |ui| {
                            let list = scatter_files(&root);
                            if list.is_empty() {
                                ui.weak("Nenhum modelo (.glb) ou prefab no projeto");
                            }
                            for f in &list {
                                ui.selectable_value(&mut self.terrain_tool.scatter, f.clone(), f);
                            }
                        });
                    crate::tutorial::anchor(ui.ctx(), "terreno:espalhar", r.response.rect);
                    ui.label("Por segundo");
                    ui.add(
                        egui::DragValue::new(&mut self.terrain_tool.density)
                            .range(0.5..=60.0)
                            .speed(0.1),
                    );
                }
                _ => {}
            }
        });
    }

    fn toolbar3d(&mut self, ui: &mut egui::Ui, env: &mut SceneEnv, actions: &mut Vec<SceneAction>) {
        ui.horizontal_wrapped(|ui| {
            let r = ui
                .add(
                    egui::Button::new("▶ Testar daqui")
                        .fill(crate::theme::PLAY.gamma_multiply(0.55)),
                )
                .on_hover_text("Roda o jogo começando nesta cena (F6)");
            crate::tutorial::anchor(ui.ctx(), "cena.testar", r.rect);
            if r.clicked() {
                actions.push(SceneAction::TestHere);
            }
            if env.connected {
                ui.colored_label(GREEN, "• Ao vivo").on_hover_text(
                    "O jogo aberto está nesta cena: o que você mudar aparece nele na hora, sem salvar.",
                );
                super::paths_button(ui, env, actions);
            }
            ui.separator();
            for (tool, label, tip) in [
                (
                    Tool::Move,
                    "✋ Mover",
                    "Mover (W). Arraste as setas, ou o objeto para mover no chão.",
                ),
                (
                    Tool::Rotate,
                    "⟲ Girar",
                    "Girar (E). Arraste um dos círculos; com Ctrl, de 15 em 15 graus.",
                ),
                (
                    Tool::Scale,
                    "⬈ Escalar",
                    "Escalar (R). Arraste a ponta de um eixo, ou o quadrado do meio para todos.",
                ),
            ] {
                if ui
                    .selectable_label(self.tool == tool, label)
                    .on_hover_text(tip)
                    .clicked()
                {
                    self.tool = tool;
                }
            }
            ui.separator();
            ui.checkbox(&mut self.snap, "Encaixar").on_hover_text(
                "Encaixa na grade ao mover, de 15 em 15 graus ao girar e de 0,1 em 0,1 ao escalar. Segurar Ctrl troca.",
            );
            if ui.available_size_before_wrap().x < 70.0 {
                ui.end_row();
            }
            egui::ComboBox::from_id_salt(("grade3d", &self.rel))
                .width(56.0)
                .selected_text(scene_data::format_num(self.grid3))
                .show_ui(ui, |ui| {
                    for g in [0.1, 0.25, 0.5, 1.0, 2.0, 5.0] {
                        ui.selectable_value(&mut self.grid3, g, scene_data::format_num(g));
                    }
                })
                .response
                .on_hover_text("Tamanho da grade, em metros");
            ui.checkbox(&mut self.show_grid3, "Grade");
            ui.checkbox(&mut self.show_colliders, "Colisões")
                .on_hover_text("Mostra os Corpos 3D (verde) e o Jogador 3D (azul)");
            if self.data.env("fog").bool() {
                ui.checkbox(&mut self.show_fog3, "Névoa").on_hover_text(
                    "A névoa da cena, só aqui na vista (no jogo ela continua). Desligue para ver a cena toda de longe.",
                );
            }
            if self.has_screen3() {
                ui.checkbox(&mut self.show_hud3, "HUD").on_hover_text(
                    "Mostra por cima da vista o que fica preso à tela (placar, corações, menus), na moldura da tela do jogo. Dá para clicar e arrastar.",
                );
            }
            ui.separator();
            let menu = ui.menu_button("➕ Objeto", |ui| {
                if let Some(o) = add_menu3(ui) {
                    self.add_from_menu3(o);
                    ui.close();
                }
            });
            crate::tutorial::anchor(ui.ctx(), "cena.objeto", menu.response.rect);
            let r = ui
                .button("☀ Ambiente")
                .on_hover_text("Céu, sol, névoa e sombras da cena (no Inspetor)");
            crate::tutorial::anchor(ui.ctx(), "cena.ambiente", r.rect);
            if r.clicked() {
                self.selection.clear();
            }
            if ui.button("⛶ Enquadrar").on_hover_text("F").clicked() {
                self.frame3(env.root, env.sizes);
            }
            if self.selected_terrain().is_some() {
                let r = ui
                    .selectable_label(self.terrain_tool.active, "✏ Pintar terreno")
                    .on_hover_text("O botão esquerdo pinta o terreno selecionado (Esc para)");
                crate::tutorial::anchor(ui.ctx(), "cena.terreno", r.rect);
                if r.clicked() {
                    self.terrain_tool.active = !self.terrain_tool.active;
                }
            }
            ui.label(format!("voo {:.0} m/s", self.cam3.speed)).on_hover_text(
                "Segure o botão direito e use WASD para voar (Q/E desce e sobe, Shift corre). A rodinha com o botão direito muda a velocidade. Alt + arrastar gira em volta da seleção; o botão do meio arrasta a vista.",
            );
        });
    }

    /// O tamanho das alças no mundo (para ficarem do mesmo tamanho na tela).
    fn gizmo_len(&self, center: Vec3) -> f32 {
        (center - self.cam3.pos).length().max(0.5) * 0.16
    }

    /// Qual alça está embaixo do mouse.
    fn gizmo_hit3(&self, rect: Rect, p: Pos2) -> Option<Drag3Kind> {
        let center = self.selection_center3()?;
        let len = self.gizmo_len(center);
        let c = to_screen(&self.cam3, rect, center)?;
        let (o, d) = ray(&self.cam3, rect, p);
        match self.tool {
            Tool::Move => {
                for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
                    let Some(tip) = to_screen(&self.cam3, rect, center + axis * len) else {
                        continue;
                    };
                    if dist_to_segment(p, c, tip) < 8.0 {
                        let s0 = axis_param(o, d, center, axis)?;
                        return Some(Drag3Kind::Axis { axis, s0 });
                    }
                }
                if (p - c).length() < 10.0 {
                    let hit0 = ray_plane(o, d, center.y)?;
                    return Some(Drag3Kind::Plane { y: center.y, hit0 });
                }
                None
            }
            Tool::Rotate => {
                let mut best: Option<(f32, Vec3)> = None;
                for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
                    let pts = ring(center, axis, len);
                    for w in pts.windows(2) {
                        if let (Some(a), Some(b)) = (
                            to_screen(&self.cam3, rect, w[0]),
                            to_screen(&self.cam3, rect, w[1]),
                        ) {
                            let dd = dist_to_segment(p, a, b);
                            if dd < 7.0 && best.is_none_or(|(bd, _)| dd < bd) {
                                best = Some((dd, axis));
                            }
                        }
                    }
                }
                let (_, axis) = best?;
                let v = p - c;
                Some(Drag3Kind::Rotate {
                    axis,
                    a0: v.y.atan2(v.x),
                    center: c,
                })
            }
            Tool::Scale => {
                if (p - c).length() < 10.0 {
                    return Some(Drag3Kind::Scale {
                        axis: 3,
                        s0: p.y,
                        len,
                    });
                }
                let q = self.selection_rotation3();
                for (i, axis) in [Vec3::X, Vec3::Y, Vec3::Z].into_iter().enumerate() {
                    let a = q * axis;
                    let Some(tip) = to_screen(&self.cam3, rect, center + a * len) else {
                        continue;
                    };
                    if dist_to_segment(p, c, tip) < 8.0 {
                        let s0 = axis_param(o, d, center, a)?;
                        return Some(Drag3Kind::Scale { axis: i, s0, len });
                    }
                }
                None
            }
        }
    }

    /// A rotação no mundo do último selecionado.
    fn selection_rotation3(&self) -> Quat {
        self.selection
            .last()
            .and_then(|id| self.matrix3(*id))
            .map(|m| m.to_scale_rotation_translation().1)
            .unwrap_or(Quat::IDENTITY)
    }

    fn start_drag3(&mut self, kind: Drag3Kind, start: Pos2) {
        let objects = self
            .selection_roots()
            .into_iter()
            .filter_map(|id| Some((id, self.matrix3(id)?)))
            .collect();
        self.drag3 = Some(Drag3 {
            kind,
            start,
            objects,
            center: self.selection_center3().unwrap_or(Vec3::ZERO),
            recorded: false,
        });
    }

    fn input3(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        rect: Rect,
        env: &mut SceneEnv,
        actions: &mut Vec<SceneAction>,
    ) {
        let (mods, dt, scroll, secondary, delta) = ui.input(|i| {
            (
                i.modifiers,
                i.stable_dt.min(0.1),
                i.smooth_scroll_delta.y,
                i.pointer.secondary_down(),
                i.pointer.delta(),
            )
        });
        let hovered = response.hovered() || response.dragged();

        // Voar: botão direito segurado + WASD / QE.
        let flying = secondary
            && (hovered || matches!(self.drag3.as_ref().map(|d| &d.kind), Some(Drag3Kind::Look)));
        if response.drag_started_by(egui::PointerButton::Secondary) {
            self.drag3 = Some(Drag3 {
                kind: Drag3Kind::Look,
                start: response.interact_pointer_pos().unwrap_or_default(),
                objects: Vec::new(),
                center: Vec3::ZERO,
                recorded: false,
            });
        }
        if flying {
            if matches!(self.drag3.as_ref().map(|d| &d.kind), Some(Drag3Kind::Look)) {
                self.cam3.rotate(-delta.x * 0.25, -delta.y * 0.25);
            }
            let mut mv = Vec3::ZERO;
            ui.input(|i| {
                let k = |key| f32::from(u8::from(i.key_down(key)));
                mv += self.cam3.forward() * (k(Key::W) - k(Key::S));
                mv += self.cam3.right() * (k(Key::D) - k(Key::A));
                mv += Vec3::Y * (k(Key::E) - k(Key::Q));
            });
            if mv != Vec3::ZERO {
                let fast = if mods.shift { 3.0 } else { 1.0 };
                self.cam3.pos += mv.normalize() * self.cam3.speed * fast * dt;
                self.cam3_set = true;
                ui.ctx().request_repaint();
            }
            if scroll != 0.0 && hovered {
                self.cam3.speed = (self.cam3.speed * (1.0 + scroll * 0.002)).clamp(0.5, 200.0);
            }
        } else if scroll != 0.0 && response.hovered() {
            // Rodinha: aproxima de onde o mouse aponta.
            let p = response.hover_pos().unwrap_or(rect.center());
            let (_, d) = ray(&self.cam3, rect, p);
            let dist = (self.pivot3() - self.cam3.pos).length().max(1.0);
            self.cam3.pos += d * scroll * 0.004 * dist;
            self.cam3_set = true;
        }

        // Começo de um arrasto com o botão esquerdo ou o do meio.
        if response.drag_started_by(egui::PointerButton::Middle) {
            let dist = (self.pivot3() - self.cam3.pos).length().max(1.0);
            self.drag3 = Some(Drag3 {
                kind: Drag3Kind::Pan { dist },
                start: response.interact_pointer_pos().unwrap_or_default(),
                objects: Vec::new(),
                center: Vec3::ZERO,
                recorded: false,
            });
        }
        // Pincel do terreno: o botão esquerdo pinta (e não seleciona).
        let painting_terrain = self.terrain_tool.active && self.selected_terrain().is_some();
        if !painting_terrain {
            self.terrain_tool.active = false;
            self.terrain_tool.cursor = None;
        }
        if painting_terrain {
            let hover = response.hover_pos();
            self.terrain_tool.cursor = hover.and_then(|p| {
                let (o, d) = ray(&self.cam3, rect, p);
                self.terrain_hit(o, d).map(|(_, _, hit)| hit)
            });
            let down = ui.input(|i| i.pointer.primary_down());
            if down
                && !mods.alt
                && (response.is_pointer_button_down_on() || self.terrain_tool.painting)
            {
                if !self.terrain_tool.painting {
                    self.terrain_tool.painting = true;
                    self.terrain_tool.stroke += 1;
                    self.terrain_tool.flatten_to = None;
                    self.terrain_tool.carry = 1.0;
                }
                if let Some(p) = ui.input(|i| i.pointer.latest_pos()) {
                    self.terrain_stroke(rect, p, dt.max(1.0 / 120.0), mods.command);
                }
                ui.ctx().request_repaint();
            } else {
                self.terrain_tool.painting = false;
            }
            if self.terrain_tool.cursor.is_some() {
                ui.ctx().request_repaint();
            }
        }
        if !painting_terrain
            && response.drag_started_by(egui::PointerButton::Primary)
            && let Some(p) = response.interact_pointer_pos()
        {
            if mods.alt {
                let pivot = self.pivot3();
                self.drag3 = Some(Drag3 {
                    kind: Drag3Kind::Orbit { pivot },
                    start: p,
                    objects: Vec::new(),
                    center: pivot,
                    recorded: false,
                });
            } else if let Some(kind) = self.gizmo_hit3(rect, p) {
                self.start_drag3(kind, p);
            } else if let Some(id) = self.hit_screen3(env, rect, p) {
                // Um objeto do HUD: seleciona e arrasta na tela do jogo.
                if !self.selection.contains(&id) {
                    self.selection = vec![id];
                }
                let starts = self
                    .selection_roots()
                    .into_iter()
                    .filter(|id| self.on_screen3(*id))
                    .filter_map(|id| {
                        let t = self.data.world_transform(id)?;
                        Some((id, t.x, t.y))
                    })
                    .collect();
                self.drag3 = Some(Drag3 {
                    kind: Drag3Kind::Screen {
                        from: self.camera.to_world(rect, p),
                        starts,
                    },
                    start: p,
                    objects: Vec::new(),
                    center: Vec3::ZERO,
                    recorded: false,
                });
            } else {
                // Arrastar um objeto: seleciona e move no chão.
                let (o, d) = ray(&self.cam3, rect, p);
                if let Some((id, _)) = self.pick3(env.root, env.sizes, o, d) {
                    if !self.selection.contains(&id) {
                        self.selection = vec![id];
                    }
                    let y = self.selection_center3().map_or(0.0, |c| c.y);
                    if let Some(hit0) = ray_plane(o, d, y) {
                        self.start_drag3(Drag3Kind::Plane { y, hit0 }, p);
                    }
                }
            }
        }

        if let Some(mut drag) = self.drag3.take() {
            let p = ui.input(|i| i.pointer.latest_pos()).unwrap_or(drag.start);
            let still = response.dragged() || ui.input(|i| i.pointer.any_down());
            self.continue_drag3(&mut drag, p, rect, mods);
            if still {
                self.drag3 = Some(drag);
            }
        }

        // Clique: seleciona (Shift ou Ctrl junta / tira).
        if response.clicked()
            && !painting_terrain
            && !mods.alt
            && let Some(p) = response.interact_pointer_pos()
        {
            let (o, d) = ray(&self.cam3, rect, p);
            let hit = match self.hit_screen3(env, rect, p) {
                Some(id) => Some((id, 0.0)),
                None => self.pick3(env.root, env.sizes, o, d),
            };
            match hit {
                Some((id, _)) if mods.shift || mods.command => {
                    if let Some(i) = self.selection.iter().position(|s| *s == id) {
                        self.selection.remove(i);
                    } else {
                        self.selection.push(id);
                    }
                }
                Some((id, _)) => self.selection = vec![id],
                None if !(mods.shift || mods.command) => self.selection.clear(),
                None => {}
            }
        }
        // Passando o mouse por cima: destaca.
        self.hover = response.hover_pos().and_then(|p| {
            if let Some(id) = self.hit_screen3(env, rect, p) {
                return Some(id);
            }
            let (o, d) = ray(&self.cam3, rect, p);
            self.pick3(env.root, env.sizes, o, d).map(|(id, _)| id)
        });

        if response.hovered() || response.has_focus() || response.dragged() {
            self.keyboard3(ui, env, actions, secondary);
        }
    }

    fn continue_drag3(&mut self, drag: &mut Drag3, p: Pos2, rect: Rect, mods: Modifiers) {
        let snapping = self.snap != mods.command;
        let (o, d) = ray(&self.cam3, rect, p);
        let record = |doc: &mut Self, drag: &mut Drag3| {
            if !drag.recorded {
                drag.recorded = true;
                doc.record(None);
            }
        };
        match drag.kind.clone() {
            Drag3Kind::Look => {}
            Drag3Kind::Screen { from, starts } => {
                let (wx, wy) = self.camera.to_world(rect, p);
                let (dx, dy) = (wx - from.0, wy - from.1);
                if dx.abs() + dy.abs() < 1e-9 && !drag.recorded {
                    return;
                }
                record(self, drag);
                for (id, x0, y0) in starts {
                    self.set_world_position(id, (x0 + dx).round(), (y0 + dy).round());
                }
            }
            Drag3Kind::Pan { dist } => {
                let delta = p - drag.start;
                drag.start = p;
                let k = dist * 0.0018;
                self.cam3.pos += (-self.cam3.right() * delta.x + self.cam3.up() * delta.y) * k;
                self.cam3_set = true;
            }
            Drag3Kind::Orbit { pivot } => {
                let delta = p - drag.start;
                drag.start = p;
                let dist = (self.cam3.pos - pivot).length().max(0.5);
                self.cam3.rotate(-delta.x * 0.3, -delta.y * 0.3);
                self.cam3.pos = pivot - self.cam3.forward() * dist;
                self.cam3_set = true;
            }
            Drag3Kind::Axis { axis, s0 } => {
                let Some(s) = axis_param(o, d, drag.center, axis) else {
                    return;
                };
                if (s - s0).abs() < 1e-6 && !drag.recorded {
                    return;
                }
                record(self, drag);
                for (id, m0) in drag.objects.clone() {
                    let p0 = m0.transform_point3(Vec3::ZERO);
                    let mut pos = p0 + axis * (s - s0);
                    if snapping {
                        let k = axis.abs().max_position();
                        pos[k] = snap(pos[k], self.grid3 as f32);
                    }
                    self.data
                        .set_world_matrix3(id, Mat4::from_translation(pos - p0) * m0);
                }
            }
            Drag3Kind::Plane { y, hit0 } => {
                let Some(hit) = ray_plane(o, d, y) else {
                    return;
                };
                let delta = hit - hit0;
                if delta.length() < 1e-6 && !drag.recorded {
                    return;
                }
                record(self, drag);
                for (id, m0) in drag.objects.clone() {
                    let p0 = m0.transform_point3(Vec3::ZERO);
                    let mut pos = p0 + Vec3::new(delta.x, 0.0, delta.z);
                    if snapping {
                        pos.x = snap(pos.x, self.grid3 as f32);
                        pos.z = snap(pos.z, self.grid3 as f32);
                    }
                    self.data
                        .set_world_matrix3(id, Mat4::from_translation(pos - p0) * m0);
                }
            }
            Drag3Kind::Rotate { axis, a0, center } => {
                let v = p - center;
                let mut angle = v.y.atan2(v.x) - a0;
                // Na tela, o Y desce: girar "anti-horário" visto de frente
                // para o eixo é o sentido positivo.
                let facing = axis.dot(self.cam3.pos - drag.center) > 0.0;
                if facing {
                    angle = -angle;
                }
                let mut deg = angle.to_degrees();
                deg = (deg + 540.0).rem_euclid(360.0) - 180.0;
                if snapping {
                    deg = snap(deg, 15.0);
                }
                if deg.abs() < 1e-4 && !drag.recorded {
                    return;
                }
                record(self, drag);
                let r = Quat::from_axis_angle(axis, deg.to_radians());
                for (id, m0) in drag.objects.clone() {
                    let (s, q, t) = m0.to_scale_rotation_translation();
                    self.data
                        .set_world_matrix3(id, Mat4::from_scale_rotation_translation(s, r * q, t));
                }
            }
            Drag3Kind::Scale { axis, s0, len } => {
                let factor = if axis == 3 {
                    1.0 + (s0 - p.y) / 120.0
                } else {
                    let q = drag
                        .objects
                        .last()
                        .map(|(_, m)| m.to_scale_rotation_translation().1)
                        .unwrap_or(Quat::IDENTITY);
                    let a = q * [Vec3::X, Vec3::Y, Vec3::Z][axis];
                    let Some(s) = axis_param(o, d, drag.center, a) else {
                        return;
                    };
                    1.0 + (s - s0) / len.max(0.01)
                };
                if (factor - 1.0).abs() < 1e-5 && !drag.recorded {
                    return;
                }
                record(self, drag);
                let factor = factor.max(0.01);
                for (id, m0) in drag.objects.clone() {
                    let (s, q, t) = m0.to_scale_rotation_translation();
                    let mut ns = s;
                    for k in 0..3 {
                        if axis == 3 || axis == k {
                            ns[k] = s[k] * factor;
                            if snapping {
                                ns[k] = snap(ns[k], 0.1).max(0.1);
                            }
                        }
                    }
                    self.data
                        .set_world_matrix3(id, Mat4::from_scale_rotation_translation(ns, q, t));
                }
            }
        }
    }

    fn keyboard3(
        &mut self,
        ui: &mut egui::Ui,
        env: &mut SceneEnv,
        actions: &mut Vec<SceneAction>,
        flying: bool,
    ) {
        if ui.ctx().text_edit_focused() {
            return;
        }
        let pressed =
            |ui: &mut egui::Ui, m: Modifiers, k: Key| ui.input_mut(|i| i.consume_key(m, k));
        if !flying {
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
                self.frame3(env.root, env.sizes);
            }
        }
        if pressed(ui, Modifiers::NONE, Key::Delete) {
            self.delete_selection();
        }
        if pressed(ui, Modifiers::COMMAND, Key::D) {
            self.duplicate_selection3();
        }
        if pressed(ui, Modifiers::NONE, Key::Escape) {
            self.selection.clear();
        }
        if pressed(ui, Modifiers::NONE, Key::F6) {
            actions.push(SceneAction::TestHere);
        }
    }

    /// Duplica a seleção, com a cópia um passo da grade ao lado.
    pub fn duplicate_selection3(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        self.record(None);
        let mut new = Vec::new();
        let keep = self.prefab_root();
        for id in self.selection_roots() {
            if Some(id) == keep {
                continue;
            }
            if let Some(n) = self.data.duplicate(id) {
                if let Some(o) = self.data.find_mut(n) {
                    o.x += self.grid3.max(0.5);
                }
                new.push(n);
            }
        }
        self.selection = new;
    }

    fn overlays3(&self, painter: &egui::Painter, rect: Rect, env: &mut SceneEnv) {
        // Caixas de seleção e de destaque (e das colisões, se pedido).
        let mut boxes: Vec<(u32, Color32)> = Vec::new();
        if self.show_colliders {
            scene_data::visit(&self.data.objects, &mut |o| {
                let player = o
                    .components
                    .iter()
                    .any(|c| c.kind == "behavior" && c.str("script") == "nn:jogador3d");
                if player {
                    boxes.push((o.id, Color32::from_rgb(90, 160, 255)));
                } else if o.component("body3d").is_some() {
                    boxes.push((o.id, Color32::from_rgb(90, 230, 120)));
                }
            });
        }
        boxes.extend(self.selection.iter().map(|id| (*id, ORANGE)));
        if let Some(h) = self.hover
            && !self.selection.contains(&h)
        {
            boxes.push((h, Color32::from_white_alpha(120)));
        }
        // A moldura da tela do jogo, com o HUD à mostra.
        let hud = self.show_hud3 && self.has_screen3();
        if hud {
            let (cx, cy, zoom) = self.data.camera_start();
            let (sw, sh) = scene_data::screen_size();
            let (hw, hh) = (sw / zoom / 2.0, sh / zoom / 2.0);
            let a = self.camera.to_screen(rect, cx - hw, cy + hh);
            let b = self.camera.to_screen(rect, cx + hw, cy - hh);
            painter.rect_stroke(
                Rect::from_two_pos(a, b),
                0.0,
                Stroke::new(1.0, Color32::from_white_alpha(70)),
                egui::StrokeKind::Inside,
            );
        }
        for (id, color) in boxes {
            if self.on_screen3(id) {
                // Do HUD: o contorno na tela do jogo.
                if hud && let Some((corners, _, _)) = self.screen_corners(env, rect, id) {
                    let mut pts = corners.to_vec();
                    pts.push(corners[0]);
                    painter.add(egui::Shape::line(pts, Stroke::new(1.5, color)));
                }
                continue;
            }
            let (Some(o), Some(m)) = (self.data.find(id), self.matrix3(id)) else {
                continue;
            };
            let (a, b) = scene_render::bounds3(o, env.root, env.sizes);
            let corner = |i: usize| {
                m.transform_point3(Vec3::new(
                    if i & 1 == 0 { a.x } else { b.x },
                    if i & 2 == 0 { a.y } else { b.y },
                    if i & 4 == 0 { a.z } else { b.z },
                ))
            };
            for (i, j) in [
                (0, 1),
                (2, 3),
                (4, 5),
                (6, 7),
                (0, 2),
                (1, 3),
                (4, 6),
                (5, 7),
                (0, 4),
                (1, 5),
                (2, 6),
                (3, 7),
            ] {
                if let (Some(p), Some(q)) = (
                    to_screen(&self.cam3, rect, corner(i)),
                    to_screen(&self.cam3, rect, corner(j)),
                ) {
                    painter.line_segment([p, q], Stroke::new(1.5, color));
                }
            }
        }

        // O círculo do pincel do terreno.
        if self.terrain_tool.active
            && let Some(hit) = self.terrain_tool.cursor
            && let Some((id, ci, origin)) = self.selected_terrain()
            && let Some(c) = self.data.find(id).and_then(|o| o.components.get(ci))
        {
            let t = crate::terrain_tool::terrain_of(c);
            let pts: Vec<Pos2> =
                crate::terrain_tool::ring(&t, origin, hit, self.terrain_tool.radius)
                    .into_iter()
                    .filter_map(|p| to_screen(&self.cam3, rect, p))
                    .collect();
            painter.add(egui::Shape::line(pts, Stroke::new(2.0, Color32::WHITE)));
            if let Some(c) = to_screen(&self.cam3, rect, hit) {
                painter.circle_filled(c, 3.0, Color32::WHITE);
            }
            return;
        }

        // As alças da ferramenta.
        let Some(center) = self.selection_center3() else {
            return;
        };
        let Some(c) = to_screen(&self.cam3, rect, center) else {
            return;
        };
        let len = self.gizmo_len(center);
        let axes = [(Vec3::X, RED), (Vec3::Y, GREEN), (Vec3::Z, BLUE)];
        match self.tool {
            Tool::Move => {
                for (axis, color) in axes {
                    if let Some(tip) = to_screen(&self.cam3, rect, center + axis * len) {
                        arrow(painter, c, tip, color);
                    }
                }
                painter.rect_filled(
                    Rect::from_center_size(c, egui::vec2(10.0, 10.0)),
                    2.0,
                    Color32::from_rgba_unmultiplied(255, 220, 80, 200),
                );
            }
            Tool::Rotate => {
                for (axis, color) in axes {
                    let pts: Vec<Pos2> = ring(center, axis, len)
                        .into_iter()
                        .filter_map(|w| to_screen(&self.cam3, rect, w))
                        .collect();
                    painter.add(egui::Shape::line(pts, Stroke::new(2.0, color)));
                }
            }
            Tool::Scale => {
                let q = self.selection_rotation3();
                for (axis, color) in axes {
                    if let Some(tip) = to_screen(&self.cam3, rect, center + (q * axis) * len) {
                        painter.line_segment([c, tip], Stroke::new(2.5, color));
                        painter.rect_filled(
                            Rect::from_center_size(tip, egui::vec2(9.0, 9.0)),
                            1.0,
                            color,
                        );
                    }
                }
                painter.rect_filled(
                    Rect::from_center_size(c, egui::vec2(11.0, 11.0)),
                    1.0,
                    Color32::from_gray(230),
                );
            }
        }
    }
}

/// Pontos de um círculo de raio `r` em volta de `center`, perpendicular ao
/// eixo.
fn ring(center: Vec3, axis: Vec3, r: f32) -> Vec<Vec3> {
    let a = if axis.y.abs() > 0.9 { Vec3::X } else { Vec3::Y };
    let u = axis.cross(a).normalize();
    let v = axis.cross(u);
    (0..=48)
        .map(|i| {
            let t = i as f32 / 48.0 * std::f32::consts::TAU;
            center + (u * t.cos() + v * t.sin()) * r
        })
        .collect()
}

fn arrow(painter: &egui::Painter, from: Pos2, to: Pos2, color: Color32) {
    painter.line_segment([from, to], Stroke::new(2.5, color));
    let dir = (to - from).normalized();
    let side = egui::vec2(-dir.y, dir.x);
    let head = [
        to + dir * 6.0,
        to - dir * 6.0 + side * 5.0,
        to - dir * 6.0 - side * 5.0,
    ];
    painter.add(egui::Shape::convex_polygon(
        head.to_vec(),
        color,
        Stroke::NONE,
    ));
}

/// Itens do menu "➕ Objeto" de uma cena 3D.
/// O menu ➕ da cena 3D: os objetos do mundo e, num submenu, os da tela
/// (HUD e menus, que ficam por cima do mundo).
pub fn add_menu3(ui: &mut egui::Ui) -> Option<Object> {
    let mut out = add_object3_menu(ui);
    ui.separator();
    let r = ui.menu_button("Na tela (HUD)", |ui| {
        let r = ui
            .button("Texto na tela")
            .on_hover_text("Um texto preso à tela: placar, dicas, avisos");
        if r.clicked() {
            let mut o = Object::new(0, "texto");
            let mut c = Component::new("text");
            c.set("hud", Prop::Bool(true));
            o.components.push(c);
            out = Some(o);
            ui.close();
        }
        ui.separator();
        if let Some(o) = super::interface_menu(ui) {
            out = Some(o);
            ui.close();
        }
    });
    crate::tutorial::anchor(ui.ctx(), "cena.objeto:hud", r.response.rect);
    out
}

pub fn add_object3_menu(ui: &mut egui::Ui) -> Option<Object> {
    let mut out = None;
    let shape = |name: &str, shape: &str, color: &str| {
        let mut o = Object::new(0, name);
        let mut c = Component::new("shape3d");
        c.set("shape", Prop::Str(shape.into()));
        c.set("color", Prop::Str(color.into()));
        o.components.push(c);
        o
    };
    for (label, name, s, color) in [
        ("Cubo", "Cubo", "cube", "#FF8C42"),
        ("Esfera", "Esfera", "sphere", "#3FA7D6"),
        ("Cilindro", "Cilindro", "cylinder", "#C77DFF"),
        ("Cone", "Cone", "cone", "#FFD166"),
        ("Cápsula", "Cápsula", "capsule", "#06D6A0"),
        ("Plano (chão)", "Plano", "plane", "#7BC86C"),
    ] {
        if ui.button(label).clicked() {
            let mut o = shape(name, s, color);
            if s == "plane" {
                o.scale_x = 10.0;
                o.scale_z = 10.0;
                // Um chão: já vem com um Corpo 3D parado.
                o.components.push(Component::new("body3d"));
            }
            out = Some(o);
        }
    }
    ui.separator();
    if ui
        .button("Modelo (.glb)")
        .on_hover_text("Um objeto com um modelo 3D: escolha o arquivo no Inspetor, ou arraste um .glb do painel Arquivos para a cena.")
        .clicked()
    {
        let mut o = Object::new(0, "Modelo");
        o.components.push(Component::new("model"));
        out = Some(o);
    }
    if ui
        .button("Câmera 3D")
        .on_hover_text("A câmera do jogo: ele mostra o que ela vê.")
        .clicked()
    {
        let mut o = Object::new(0, "Câmera");
        o.components.push(Component::new("camera3d"));
        out = Some(o);
    }
    if ui
        .button("Vazio")
        .on_hover_text("Um objeto sem nada, para agrupar outros")
        .clicked()
    {
        out = Some(Object::new(0, "Grupo"));
    }
    ui.separator();
    if ui
        .button("Terreno")
        .on_hover_text(
            "Um chão de 64 × 64 m para pintar morros, vales, cores e água (✏ Pintar terreno)",
        )
        .clicked()
    {
        let mut o = Object::new(0, "Terreno");
        o.components.push(Component::new("terrain"));
        out = Some(o);
    }
    ui.menu_button("Partículas", |ui| {
        let r = ui
            .button("Vazias")
            .on_hover_text("Um emissor para ajustar do zero no Inspetor");
        if r.clicked() {
            let mut o = Object::new(0, "particulas");
            o.components.push(crate::particle_presets::component_3d());
            out = Some(o);
            ui.close();
        }
        ui.separator();
        for p in crate::particle_presets::PRESETS_3D {
            let r = ui.button(p.label).on_hover_text(p.tip);
            crate::tutorial::anchor(ui.ctx(), &format!("cena.objeto:fx:{}", p.id), r.rect);
            if r.clicked() {
                let mut o = Object::new(0, p.id);
                o.components.push(p.component());
                out = Some(o);
                ui.close();
            }
        }
    });
    ui.menu_button("Portas e itens", |ui| {
        let behavior = |script: &str, set: &[(&str, Prop)]| {
            let mut c = match crate::behaviors::builtins()
                .iter()
                .find(|b| b.script == script)
            {
                Some(info) => Component::behavior(script, &info.defaults_for(true)),
                None => Component::behavior(script, &[]),
            };
            for (k, v) in set {
                c.set(k, v.clone());
            }
            c
        };
        let items: [(&str, &str, &str); 6] = [
            (
                "porta",
                "Porta",
                "Uma porta que abre com E (girando na dobradiça). Ponha um nome de item em \"Precisa do item\" para trancar.",
            ),
            (
                "chave",
                "Chave",
                "Um item que vai para o inventário (\"chave\") quando o jogador encosta. Abre portas trancadas.",
            ),
            (
                "zona",
                "Zona de gatilho",
                "Uma área invisível no jogo que avisa quando o jogador entra ou sai (abre uma porta, liga uma armadilha...).",
            ),
            (
                "texto3d",
                "Texto 3D",
                "Um texto no mundo: placa, nome, aviso. Sempre virado para a câmera (ou preso como placa).",
            ),
            (
                "inventario",
                "Mostrar inventário",
                "Os itens que o jogador carrega, num canto da tela.",
            ),
            (
                "personagem",
                "Personagem que fala",
                "Alguém para conversar: perto dele aparece \"E falar\" e abre uma conversa (.dialogo) numa caixa de fala.",
            ),
        ];
        for (id, label, tip) in items {
            let r = ui.button(label).on_hover_text(tip);
            crate::tutorial::anchor(ui.ctx(), &format!("cena.objeto:{id}"), r.rect);
            if !r.clicked() {
                continue;
            }
            let mut o;
            match id {
                "porta" => {
                    o = shape("Porta", "cube", "#9C6B43");
                    (o.scale_x, o.scale_y, o.scale_z) = (1.2, 2.2, 0.15);
                    let mut body = Component::new("body3d");
                    body.set("body", Prop::Str("kinematic".into()));
                    o.components.push(body);
                    o.components.push(behavior("nn:porta3d", &[]));
                }
                "chave" => {
                    o = shape("Chave", "cube", "#FFD166");
                    (o.scale_x, o.scale_y, o.scale_z) = (0.35, 0.35, 0.35);
                    o.components.push(behavior(
                        "nn:coletavel",
                        &[
                            ("variavel", Prop::Str(String::new())),
                            ("item", Prop::Str("chave".into())),
                        ],
                    ));
                    o.components.push(behavior("nn:girar3d", &[]));
                }
                "zona" => {
                    o = shape("Zona", "cube", "#3FA7D640");
                    (o.scale_x, o.scale_y, o.scale_z) = (3.0, 2.0, 3.0);
                    if let Some(c) = o.components.first_mut() {
                        c.set("shadow", Prop::Bool(false));
                    }
                    o.components.push(behavior("nn:gatilho", &[]));
                }
                "texto3d" => {
                    o = Object::new(0, "Texto");
                    let mut c = Component::new("text3d");
                    c.set("text", Prop::Str("Olá!".into()));
                    o.components.push(c);
                }
                "personagem" => {
                    o = shape("Personagem", "capsule", "#4CC9F0");
                    o.components.push(behavior("nn:dialogo", &[]));
                }
                _ => {
                    o = Object::new(0, "Inventário");
                    o.components.push(behavior("nn:inventario", &[]));
                }
            }
            out = Some(o);
            ui.close();
        }
    });
    let r = ui
        .button("Ondas de inimigos")
        .on_hover_text("Cria inimigos (um prefab) em ondas: onda 1, 2, 3... Os quatro filhos são de onde eles saem: arraste-os pela arena.");
    crate::tutorial::anchor(ui.ctx(), "cena.objeto:ondas", r.rect);
    if r.clicked() {
        let mut o = Object::new(0, "Ondas");
        let defaults = crate::behaviors::builtins()
            .iter()
            .find(|b| b.script == "nn:ondas")
            .map(|info| info.defaults_for(true))
            .unwrap_or_default();
        o.components
            .push(Component::behavior("nn:ondas", &defaults));
        for (x, z) in [(5.0, 0.0), (-5.0, 0.0), (0.0, 5.0), (0.0, -5.0)] {
            let mut p = Object::new(0, "Saída");
            (p.x, p.z) = (x, z);
            o.children.push(p);
        }
        out = Some(o);
        ui.close();
    }
    ui.separator();
    if ui
        .button("Jogador 3D")
        .on_hover_text("Uma cápsula que anda, pula e leva a câmera atrás dela. Troque a forma por um modelo .glb no Inspetor.")
        .clicked()
    {
        let mut o = shape("Jogador", "capsule", "#FF8C42");
        if let Some(info) = crate::behaviors::builtins()
            .iter()
            .find(|b| b.script == "nn:jogador3d")
        {
            o.components
                .push(Component::behavior(&info.script, &info.defaults_for(true)));
        }
        out = Some(o);
    }
    out
}

fn stem_of(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Os modelos (.glb, .gltf) e prefabs do projeto, para o 🌲 Espalhar.
fn scatter_files(root: &Path) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, depth: u32, out: &mut Vec<String>) {
        let Ok(list) = std::fs::read_dir(dir) else {
            return;
        };
        for e in list.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "target" || name == "exportado" {
                continue;
            }
            if p.is_dir() {
                if depth < 4 {
                    walk(root, &p, depth + 1, out);
                }
            } else if matches!(
                p.extension().and_then(|x| x.to_str()),
                Some("glb" | "gltf" | "prefab")
            ) && let Ok(rel) = p.strip_prefix(root)
            {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, 0, &mut out);
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camera_olha_para_onde_mandam() {
        let mut c = Cam3::default();
        c.look_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO);
        assert!((c.forward() - Vec3::NEG_Z).length() < 1e-5);
        assert!((c.right() - Vec3::X).length() < 1e-5);
        c.look_at(Vec3::new(3.0, 4.0, 0.0), Vec3::ZERO);
        let f = c.forward();
        assert!(
            (f - Vec3::new(-3.0, -4.0, 0.0).normalize()).length() < 1e-4,
            "{f}"
        );
    }

    #[test]
    fn raio_acerta_caixa_girada_e_escalada() {
        let m = Mat4::from_scale_rotation_translation(
            Vec3::new(4.0, 1.0, 1.0),
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::new(0.0, 0.0, -10.0),
        );
        let b = (Vec3::splat(-0.5), Vec3::splat(0.5));
        // Girada 90°, a caixa comprida (4 em X) fica comprida em Z: a frente
        // dela fica em z = -8.
        let t = ray_box(Vec3::ZERO, Vec3::NEG_Z, m, b).unwrap();
        assert!((t - 8.0).abs() < 1e-3, "{t}");
        // Ao lado (x = 1), não acerta (ela tem 1 de largura em X agora).
        assert!(ray_box(Vec3::new(1.0, 0.0, 0.0), Vec3::NEG_Z, m, b).is_none());
        // Para trás, não acerta.
        assert!(ray_box(Vec3::ZERO, Vec3::Z, m, b).is_none());
    }

    #[test]
    fn ponto_do_eixo_mais_perto_do_raio() {
        // Raio vindo de cima, em x = 3: o ponto do eixo X mais perto é 3.
        let s = axis_param(Vec3::new(3.0, 10.0, 0.0), Vec3::NEG_Y, Vec3::ZERO, Vec3::X).unwrap();
        assert!((s - 3.0).abs() < 1e-4);
        // Paralelo ao eixo: não dá.
        assert!(axis_param(Vec3::Y, Vec3::X, Vec3::ZERO, Vec3::X).is_none());
    }
}
