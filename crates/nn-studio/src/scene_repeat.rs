//! Repetir objetos da cena em linha ou em círculo (o 🔁 Repetir do menu do
//! objeto): uma janelinha com a prévia, como no modelador.

use eframe::egui::{self, Key, Modifiers};
use glam::{Mat4, Vec3};

use super::{SceneDoc, SceneEnv};
use crate::repeat::{self, RepeatSpec};
use crate::scene_data::SceneData;
use crate::scene_render;

/// O Repetir aberto: a cena antes da prévia, de quem são as cópias e como.
#[derive(Debug, Clone)]
pub struct SceneRepeat {
    /// `None` até o primeiro desenho (as medidas precisam dos arquivos).
    before: Option<SceneData>,
    sources: Vec<u32>,
    pub spec: RepeatSpec,
    /// No mundo 3D (fora do HUD).
    three_d: bool,
    origin3: Vec3,
    origin2: (f64, f64),
    copies: Vec<u32>,
}

impl SceneDoc {
    /// Abre o Repetir com a seleção.
    pub fn start_repeat(&mut self) -> bool {
        let keep = self.prefab_root();
        let sources: Vec<u32> = self
            .selection_roots()
            .into_iter()
            .filter(|id| Some(*id) != keep)
            .collect();
        if sources.is_empty() {
            return false;
        }
        self.finish_repeat(false);
        let three_d = self.data.three_d && sources.iter().all(|id| !self.on_screen3(*id));
        self.repeat = Some(SceneRepeat {
            before: None,
            sources,
            spec: RepeatSpec::line(3, 1.0),
            three_d,
            origin3: Vec3::ZERO,
            origin2: (0.0, 0.0),
            copies: Vec::new(),
        });
        true
    }

    pub fn repeat_open(&self) -> bool {
        self.repeat.is_some()
    }

    /// Mede a seleção (o tamanho dá a distância e o raio de começo) e faz a
    /// primeira prévia.
    fn begin_repeat(&mut self, env: &mut SceneEnv) {
        let Some(r) = &self.repeat else {
            return;
        };
        if r.before.is_some() {
            return;
        }
        let (sources, three_d) = (r.sources.clone(), r.three_d);
        let mut spec;
        let (mut origin3, mut origin2) = (Vec3::ZERO, (0.0, 0.0));
        if three_d {
            let mut lo = Vec3::splat(f32::MAX);
            let mut hi = Vec3::splat(f32::MIN);
            for &id in &sources {
                let (Some(o), Some(m)) = (self.data.find(id), self.data.world_matrix3(id)) else {
                    continue;
                };
                let (a, b) = scene_render::bounds3(o, env.root, env.sizes);
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
                origin3 += m.transform_point3(Vec3::ZERO);
            }
            origin3 /= sources.len().max(1) as f32;
            let size = if lo.x <= hi.x { hi - lo } else { Vec3::ONE };
            let step = (self.grid3 as f32).max(0.25);
            // Uma encostada na outra (a cerca continua, a parede fecha); o
            // campo Distância afasta.
            let spacing = (size.x * 100.0).round() / 100.0;
            spec = RepeatSpec::line(3, spacing.max(0.1));
            spec.radius = ((size.x.max(size.z) * 2.0 / step).round() * step).max(1.0);
        } else {
            let mut w = 0.0f64;
            for &id in &sources {
                if let Some(o) = self.data.find(id) {
                    let (lr, _) = scene_render::local_bounds(o, env.root, env.sizes, 32.0);
                    let s = self
                        .data
                        .world_transform(id)
                        .map_or(1.0, |t| t.scale_x.abs());
                    w = w.max((lr.max.0 - lr.min.0) * s);
                }
                if let Some(t) = self.data.world_transform(id) {
                    origin2.0 += t.x;
                    origin2.1 += t.y;
                }
            }
            let n = sources.len().max(1) as f64;
            origin2 = (origin2.0 / n, origin2.1 / n);
            let g = self.grid.max(1.0);
            let spacing = w.max(g).round();
            spec = RepeatSpec::line(3, spacing as f32);
            spec.radius = (spacing * 2.0) as f32;
        }
        if let Some(r) = &mut self.repeat {
            r.spec = spec;
            r.origin3 = origin3;
            r.origin2 = origin2;
            r.before = Some(self.data.clone());
        }
        self.apply_repeat();
    }

    /// Refaz as cópias da prévia.
    fn apply_repeat(&mut self) {
        let Some(r) = self.repeat.clone() else {
            return;
        };
        let Some(before) = r.before.clone() else {
            return;
        };
        self.data = before;
        let mut copies = Vec::new();
        // Cada cópia sai da anterior: os nomes seguem (cerca 2, cerca 3...) e
        // ficam em ordem na Hierarquia.
        let mut last: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
        for i in 1..=r.spec.count {
            for &id in &r.sources {
                let from = last.get(&id).copied().unwrap_or(id);
                let Some(n) = self.data.duplicate(from) else {
                    continue;
                };
                last.insert(id, n);
                if r.three_d {
                    if let Some(m0) = self.data.world_matrix3(id) {
                        let t: Mat4 = r.spec.transform(i, r.origin3);
                        self.data.set_world_matrix3(n, t * m0);
                    }
                } else if let Some(t) = self.data.world_transform(id) {
                    let ((x, y), turn) = r.spec.transform2(i, r.origin2, (t.x, t.y));
                    self.set_world_position(n, super::round2(x), super::round2(y));
                    if turn != 0.0
                        && let Some(o) = self.data.find_mut(n)
                    {
                        o.rotation = super::round2(super::normalize_deg(o.rotation + turn));
                    }
                }
                copies.push(n);
            }
        }
        self.selection = r
            .sources
            .iter()
            .copied()
            .chain(copies.iter().copied())
            .collect();
        if let Some(rr) = &mut self.repeat {
            rr.copies = copies;
        }
    }

    /// Fecha o Repetir: fica com as cópias (`apply`) ou volta como estava.
    pub fn finish_repeat(&mut self, apply: bool) {
        let Some(r) = self.repeat.take() else {
            return;
        };
        let Some(before) = r.before else {
            return;
        };
        if apply {
            if before != self.data {
                self.history.record(&before, None);
                let n = r.copies.len();
                self.notices.push(if n == 1 {
                    "1 cópia criada".into()
                } else {
                    format!("{n} cópias criadas")
                });
                self.flash_new(&r.copies);
            }
        } else {
            self.data = before;
            self.selection = r.sources;
            self.selection.retain(|id| self.data.find(*id).is_some());
        }
    }

    /// A janelinha do Repetir (no fim do desenho da aba).
    pub(super) fn repeat_window(&mut self, ui: &egui::Ui, env: &mut SceneEnv) {
        if self.repeat.is_none() {
            return;
        }
        self.begin_repeat(env);
        let view = self.view_rect.unwrap_or(ui.max_rect());
        let Some(r) = &mut self.repeat else {
            return;
        };
        let (three_d, n) = (r.three_d, r.sources.len());
        let mut changed = false;
        let mut done = None;
        // Uma janela que segura o resto (a prévia não muda por baixo), sem
        // escurecer a cena.
        let id = egui::Id::new(("cena-repetir", &self.rel));
        let area = egui::Area::new(id)
            .kind(egui::UiKind::Modal)
            .sense(egui::Sense::hover())
            .order(egui::Order::Foreground)
            .interactable(true)
            .fixed_pos(view.left_top() + egui::vec2(12.0, 12.0));
        let shown = egui::Modal::new(id)
            .area(area)
            .backdrop_color(egui::Color32::TRANSPARENT)
            .show(ui.ctx(), |ui| {
                ui.set_width(238.0);
                ui.strong("🔁 Repetir");
                ui.weak(if n == 1 {
                    "1 objeto escolhido".to_string()
                } else {
                    format!("{n} objetos escolhidos")
                });
                changed = repeat::spec_ui(
                    ui,
                    &mut r.spec,
                    three_d,
                    if three_d { "m" } else { "px" },
                    false,
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let ok = ui.add(
                        egui::Button::new("✔ Aplicar")
                            .fill(crate::theme::ACCENT.gamma_multiply(0.7)),
                    );
                    crate::tutorial::anchor(ui.ctx(), "repetir.aplicar", ok.rect);
                    if ok.on_hover_text("Enter").clicked() {
                        done = Some(true);
                    }
                    if ui.button("Cancelar").on_hover_text("Esc").clicked() {
                        done = Some(false);
                    }
                });
            });
        crate::tutorial::anchor(ui.ctx(), "repetir", shown.response.rect);
        if done.is_none() && !ui.ctx().text_edit_focused() {
            ui.input_mut(|i| {
                if i.consume_key(Modifiers::NONE, Key::Enter) {
                    done = Some(true);
                } else if i.consume_key(Modifiers::NONE, Key::Escape) {
                    done = Some(false);
                }
            });
        }
        if changed {
            self.apply_repeat();
        }
        if let Some(apply) = done {
            self.finish_repeat(apply);
        }
    }
}
