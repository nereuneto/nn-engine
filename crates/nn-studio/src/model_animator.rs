//! O esqueleto e o animador do modelador.
//!
//! - **💀 Esqueleto:** um esqueleto pronto (humanoide, quadrúpede, simples)
//!   que se ajusta ao modelo e liga as peças; o modo **Ossos** mostra os
//!   ossos e move as juntas.
//! - **🎬 Animar:** a linha do tempo embaixo, as animações do modelo
//!   (prontas ou novas), e posar: girar um osso com as alças grava um
//!   quadro-chave no instante da linha do tempo.

use eframe::egui::{self, Color32, Key, Modifiers, Pos2, Rect, Sense, Stroke};
use glam::{Mat4, Quat, Vec3};

use super::{DragKind, MTool, ModelAction, ModelDoc, Select};
use crate::model_data::{self, Anim, Key as AnimKey, ModelData};
use crate::model_rig::{self, RigKind};
use super::ray_plane_n as ray_plane_n_pub;
use crate::scene_view::scene3d::{ORANGE, axis_param, dist_to_segment, ray, snap, to_screen};

/// Arrastando uma alça de um osso: posando (Animar) ou movendo a junta
/// (Ossos).
#[derive(Debug, Clone)]
pub(super) struct RigDrag {
    kind: DragKind,
    bone: usize,
    before: ModelData,
    /// A junta (no mundo) no começo.
    origin: Vec3,
    /// O giro do osso (no pai) e o deslocamento no começo.
    base: (Quat, Vec3),
    /// O giro do pai no mundo.
    parent_rot: Quat,
    recorded: bool,
}

/// Arrastando quadros-chave na linha do tempo.
#[derive(Debug, Clone)]
pub(super) struct KeyDrag {
    from: f32,
    bone: Option<String>,
    before: ModelData,
}

/// O estado do esqueleto e do animador numa aba.
#[derive(Debug, Clone, Default)]
pub struct RigState {
    /// No modo Animar.
    pub animating: bool,
    /// No modo Ossos (escolher e mover juntas).
    pub bones_mode: bool,
    pub sel_bone: Option<usize>,
    /// A animação aberta (índice em `data.anims`).
    pub anim: Option<usize>,
    /// O instante na linha do tempo (segundos).
    pub time: f32,
    pub playing: bool,
    pub(super) drag: Option<RigDrag>,
    pub(super) key_drag: Option<KeyDrag>,
    /// A pose copiada: (osso, giro, deslocamento).
    clip: Option<Vec<(String, glam::Vec3, glam::Vec3)>>,
    renaming_anim: Option<String>,
    renaming_bone: Option<(usize, String)>,
    /// O quadro do menu do botão direito na linha do tempo.
    key_menu: Option<f32>,
}

const BONE: Color32 = Color32::from_rgb(0xE8, 0xE8, 0xF0);
const KEY: Color32 = Color32::from_rgb(0xFF, 0xD1, 0x66);

impl ModelDoc {
    pub fn has_bones(&self) -> bool {
        !self.data.bones.is_empty()
    }

    /// Põe um esqueleto pronto (trocando o que houver) e liga as peças.
    pub fn apply_rig(&mut self, kind: RigKind) {
        self.record();
        self.data.bones = model_rig::make_rig(kind, &self.data);
        for p in &mut self.data.parts {
            p.bone.clear();
        }
        let n = model_rig::auto_bind(&mut self.data);
        self.rig.sel_bone = None;
        self.rig.bones_mode = true;
        self.actions.push(ModelAction::Notice(format!(
            "Esqueleto com {} ossos; {n} {} ligada{}",
            self.data.bones.len(),
            if n == 1 { "peça" } else { "peças" },
            if n == 1 { "" } else { "s" }
        )));
    }

    /// Tira o esqueleto (e as animações, que dependem dele).
    pub fn remove_rig(&mut self) {
        if !self.has_bones() {
            return;
        }
        self.record();
        self.data.bones.clear();
        self.data.anims.clear();
        for p in &mut self.data.parts {
            p.bone.clear();
        }
        self.rig = RigState::default();
    }

    /// Liga as peças aos ossos de novo (pelo nome ou pelo mais perto).
    pub fn rebind(&mut self) -> usize {
        self.record();
        model_rig::auto_bind(&mut self.data)
    }

    pub fn current_anim(&self) -> Option<&Anim> {
        self.rig.anim.and_then(|i| self.data.anims.get(i))
    }

    fn anim_mut(&mut self) -> Option<&mut Anim> {
        self.rig.anim.and_then(|i| self.data.anims.get_mut(i))
    }

    /// Entra (ou sai) do modo Animar.
    pub fn set_animating(&mut self, on: bool) {
        if on && !self.has_bones() {
            self.actions.push(ModelAction::Notice(
                "Para animar, ponha um esqueleto primeiro (💀 Esqueleto).".into(),
            ));
            return;
        }
        self.cancel_modal();
        self.finish_repeat(false);
        self.rig.animating = on;
        self.rig.playing = false;
        self.rig.drag = None;
        if on {
            self.rig.bones_mode = false;
            // Posar é girar.
            self.tool = MTool::Rotate;
            if self.rig.anim.is_none() && !self.data.anims.is_empty() {
                self.rig.anim = Some(0);
            }
        }
    }

    /// Uma animação pronta (troca a que tiver o mesmo nome) e abre ela.
    pub fn add_ready(&mut self, name: &str) -> bool {
        let Some(kind) = model_rig::kind_of(&self.data.bones) else {
            return false;
        };
        let Some(anim) = model_rig::ready(kind, name, &self.data.bones) else {
            return false;
        };
        self.record();
        let i = match self.data.anims.iter().position(|a| a.name == name) {
            Some(i) => {
                self.data.anims[i] = anim;
                i
            }
            None => {
                self.data.anims.push(anim);
                self.data.anims.len() - 1
            }
        };
        self.rig.anim = Some(i);
        self.rig.time = 0.0;
        true
    }

    /// Todas as animações prontas que ainda faltam.
    pub fn add_all_ready(&mut self) -> usize {
        let Some(kind) = model_rig::kind_of(&self.data.bones) else {
            return 0;
        };
        self.record();
        let mut n = 0;
        for name in model_rig::ready_names(kind) {
            if self.data.anim(name).is_none()
                && let Some(a) = model_rig::ready(kind, name, &self.data.bones)
            {
                self.data.anims.push(a);
                n += 1;
            }
        }
        if self.rig.anim.is_none() && !self.data.anims.is_empty() {
            self.rig.anim = Some(0);
        }
        n
    }

    /// Uma animação nova, vazia (1 s, repetindo).
    pub fn new_anim(&mut self) -> usize {
        self.record();
        let name = (1..)
            .map(|n| format!("animação {n}"))
            .find(|n| self.data.anim(n).is_none())
            .expect("nome livre");
        self.data.anims.push(Anim::new(&name, 1.0, true));
        let i = self.data.anims.len() - 1;
        self.rig.anim = Some(i);
        self.rig.time = 0.0;
        self.rig.renaming_anim = Some(name);
        i
    }

    pub fn delete_anim(&mut self) {
        let Some(i) = self.rig.anim else {
            return;
        };
        self.record();
        self.data.anims.remove(i);
        self.rig.anim = if self.data.anims.is_empty() {
            None
        } else {
            Some(i.min(self.data.anims.len() - 1))
        };
    }

    /// A matriz de cada peça como aparece agora (posada, no Animar).
    pub fn shown_matrices(&self) -> Vec<Mat4> {
        if self.rig.animating {
            self.data
                .posed_matrices(self.current_anim(), self.rig.time)
        } else {
            self.data.parts.iter().map(|p| p.matrix()).collect()
        }
    }

    fn globals(&self) -> Vec<Mat4> {
        let anim = if self.rig.animating {
            self.current_anim()
        } else {
            None
        };
        self.data.bone_globals(anim, self.rig.time)
    }

    /// Grava um quadro-chave do osso no instante da linha do tempo.
    pub fn set_key(&mut self, bone: usize, rot: Vec3, offset: Vec3) {
        let t = self.rig.time;
        let Some(name) = self.data.bones.get(bone).map(|b| b.name.clone()) else {
            return;
        };
        if let Some(a) = self.anim_mut() {
            a.key(&name, t, rot, offset);
        }
    }

    /// Apaga os quadros no instante `t` (do osso escolhido, ou de todos).
    pub fn delete_keys_at(&mut self, t: f32) -> bool {
        let bone = self
            .rig
            .sel_bone
            .and_then(|b| self.data.bones.get(b))
            .map(|b| b.name.clone());
        let Some(a) = self.current_anim() else {
            return false;
        };
        let any = a.tracks.iter().any(|tr| {
            bone.as_ref().is_none_or(|b| *b == tr.bone)
                && tr.keys.iter().any(|k| (k.t - t).abs() < 1e-3)
        });
        if !any {
            return false;
        }
        self.record();
        if let Some(a) = self.anim_mut() {
            for tr in &mut a.tracks {
                if bone.as_ref().is_none_or(|b| *b == tr.bone) {
                    tr.keys.retain(|k| (k.t - t).abs() >= 1e-3);
                }
            }
            a.tracks.retain(|tr| !tr.keys.is_empty());
        }
        true
    }

    /// Leva os quadros de `from` para `to` (do osso, ou de todos).
    fn move_keys(&mut self, bone: Option<&str>, from: f32, to: f32) {
        let Some(a) = self.anim_mut() else {
            return;
        };
        let to = to.clamp(0.0, a.duration);
        for tr in &mut a.tracks {
            if bone.is_some_and(|b| b != tr.bone) {
                continue;
            }
            if let Some(i) = tr.keys.iter().position(|k| (k.t - from).abs() < 1e-3) {
                let mut k = tr.keys.remove(i);
                k.t = to;
                tr.set(k);
            }
        }
    }

    /// Copia a pose do instante (todos os ossos).
    pub fn copy_pose(&mut self) {
        let Some(a) = self.current_anim() else {
            return;
        };
        let t = self.rig.time;
        let clip = self
            .data
            .bones
            .iter()
            .map(|b| {
                let (q, off) = a.sample(&b.name, t);
                (b.name.clone(), model_data::euler_degrees(q), off)
            })
            .collect();
        self.rig.clip = Some(clip);
    }

    /// Cola a pose copiada no instante (um quadro em cada osso).
    pub fn paste_pose(&mut self) {
        let Some(clip) = self.rig.clip.clone() else {
            return;
        };
        if self.current_anim().is_none() {
            return;
        }
        self.record();
        let t = self.rig.time;
        if let Some(a) = self.anim_mut() {
            for (b, rot, off) in clip {
                a.key(&b, t, rot, off);
            }
        }
    }

    // Vista --------------------------------------------------------------

    /// Onde cada osso aparece na tela: (junta, ponta).
    fn bone_screen(&self, rect: Rect, globals: &[Mat4]) -> Vec<Option<(Pos2, Pos2)>> {
        self.data
            .bones
            .iter()
            .zip(globals)
            .map(|(b, g)| {
                let head = g.transform_point3(Vec3::ZERO);
                let tail = g.transform_point3(b.tail - b.head);
                Some((
                    to_screen(&self.cam, rect, head)?,
                    to_screen(&self.cam, rect, tail)?,
                ))
            })
            .collect()
    }

    /// O osso embaixo do mouse (a junta primeiro, depois o osso).
    fn pick_bone(&self, rect: Rect, p: Pos2) -> Option<usize> {
        let screen = self.bone_screen(rect, &self.globals());
        let mut best: Option<(f32, usize)> = None;
        for (i, s) in screen.iter().enumerate() {
            let Some((h, t)) = s else {
                continue;
            };
            let d = ((p - *h).length() - 4.0).min(dist_to_segment(p, *h, *t) + 2.0);
            if d < 8.0 && best.is_none_or(|b| d < b.0) {
                best = Some((d, i));
            }
        }
        best.map(|b| b.1)
    }

    /// A peça (posada) que o raio acerta primeiro.
    fn pick_part_posed(&self, o: Vec3, d: Vec3) -> Option<usize> {
        let mats = self.shown_matrices();
        let mut best: Option<(f32, usize)> = None;
        for (pi, p) in self.data.parts.iter().enumerate() {
            if !p.visible {
                continue;
            }
            for (pts, ..) in p.all_faces() {
                let w: Vec<Vec3> = pts.iter().map(|q| mats[pi].transform_point3(*q)).collect();
                for k in 1..w.len().saturating_sub(1) {
                    if let Some(t) = super::ray_triangle(o, d, w[0], w[k], w[k + 1])
                        && best.is_none_or(|b| t < b.0)
                    {
                        best = Some((t, pi));
                    }
                }
            }
        }
        best.map(|b| b.1)
    }

    /// A ferramenta das alças nos ossos: girar (posando) ou mover.
    fn rig_tool(&self) -> MTool {
        if self.rig.bones_mode {
            MTool::Move
        } else if self.tool == MTool::Move {
            MTool::Move
        } else {
            MTool::Rotate
        }
    }

    /// O mouse e o teclado na vista no modo Ossos ou Animar.
    pub(super) fn rig_input(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        rect: Rect,
        mods: Modifiers,
        flying: bool,
    ) {
        let globals = self.globals();
        let center = self
            .rig
            .sel_bone
            .and_then(|b| globals.get(b))
            .map(|g| g.transform_point3(Vec3::ZERO));
        let tool = self.rig_tool();
        if response.drag_started_by(egui::PointerButton::Primary)
            && let Some(p) = ui
                .input(|i| i.pointer.press_origin())
                .or(response.interact_pointer_pos())
        {
            if mods.alt {
                let pivot = self.pivot();
                self.drag = Some(super::MDrag {
                    kind: DragKind::Orbit { pivot },
                    start: p,
                    before: ModelData::default(),
                    center: pivot,
                    recorded: false,
                });
            } else {
                // A alça do osso escolhido; senão, o osso (ou a peça) embaixo.
                let hit = center.and_then(|c| self.gizmo_hit_at(rect, p, c, tool));
                if hit.is_none() {
                    let picked = self.pick_bone(rect, p).or_else(|| {
                        let (o, d) = ray(&self.cam, rect, p);
                        self.pick_part_posed(o, d)
                            .and_then(|pi| self.data.bone_index(&self.data.parts[pi].bone))
                    });
                    if picked.is_some() {
                        self.rig.sel_bone = picked;
                    }
                }
                let center = self
                    .rig
                    .sel_bone
                    .and_then(|b| globals.get(b))
                    .map(|g| g.transform_point3(Vec3::ZERO));
                let kind = hit.or_else(|| {
                    // Arrastar o osso: gira em volta do eixo da câmera.
                    let c = center?;
                    let cs = to_screen(&self.cam, rect, c)?;
                    if tool == MTool::Rotate {
                        let v = p - cs;
                        Some(DragKind::Rotate {
                            axis: -self.cam.forward(),
                            a0: v.y.atan2(v.x),
                            center: cs,
                        })
                    } else {
                        let (o, d) = ray(&self.cam, rect, p);
                        let normal = self.cam.forward();
                        let hit0 = ray_plane_n_pub(o, d, c, normal)?;
                        Some(DragKind::Free { normal, hit0 })
                    }
                });
                if let (Some(kind), Some(b), Some(c)) = (kind, self.rig.sel_bone, center) {
                    self.start_rig_drag(kind, b, c, &globals);
                }
            }
        }
        if let Some(mut drag) = self.rig.drag.take() {
            let p = ui.input(|i| i.pointer.latest_pos()).unwrap_or_default();
            let still = response.dragged() || ui.input(|i| i.pointer.any_down());
            self.continue_rig_drag(&mut drag, p, rect, mods);
            if still {
                self.rig.drag = Some(drag);
            } else if drag.recorded {
                self.record_before(drag.before);
            }
        }
        if let Some(mut drag) = self.drag.take() {
            let p = ui.input(|i| i.pointer.latest_pos()).unwrap_or(drag.start);
            let still = response.dragged() || ui.input(|i| i.pointer.any_down());
            self.continue_drag(&mut drag, p, rect, mods);
            if still {
                self.drag = Some(drag);
            }
        }
        if response.clicked()
            && !mods.alt
            && let Some(p) = response.interact_pointer_pos()
        {
            let picked = self.pick_bone(rect, p).or_else(|| {
                let (o, d) = ray(&self.cam, rect, p);
                self.pick_part_posed(o, d)
                    .and_then(|pi| self.data.bone_index(&self.data.parts[pi].bone))
            });
            self.rig.sel_bone = picked;
        }
        if crate::scene_view::busy(ui) || ui.ctx().text_edit_focused() {
            return;
        }
        if !(response.hovered() || response.has_focus() || response.dragged()) {
            return;
        }
        let pressed =
            |ui: &mut egui::Ui, m: Modifiers, k: Key| ui.input_mut(|i| i.consume_key(m, k));
        if !flying {
            if pressed(ui, Modifiers::NONE, Key::W) {
                self.tool = MTool::Move;
            }
            if pressed(ui, Modifiers::NONE, Key::E) {
                self.tool = MTool::Rotate;
            }
            if pressed(ui, Modifiers::NONE, Key::F) {
                self.frame();
            }
            if !self.rig.bones_mode {
                for (k, mode) in [
                    (Key::Num1, Select::Points),
                    (Key::Num2, Select::Faces),
                    (Key::Num3, Select::Parts),
                ] {
                    if pressed(ui, Modifiers::NONE, k) {
                        self.set_animating(false);
                        self.set_mode(mode);
                    }
                }
            }
        }
        if self.rig.animating {
            self.anim_keys(ui);
        }
        if pressed(ui, Modifiers::NONE, Key::Escape) {
            self.rig.sel_bone = None;
        }
    }

    /// Atalhos do Animar: Espaço toca, Del apaga o quadro, setas andam de
    /// quadro em quadro, Ctrl+C/V copiam a pose.
    fn anim_keys(&mut self, ui: &mut egui::Ui) {
        let pressed =
            |ui: &mut egui::Ui, m: Modifiers, k: Key| ui.input_mut(|i| i.consume_key(m, k));
        if pressed(ui, Modifiers::NONE, Key::Space) {
            self.rig.playing = !self.rig.playing;
        }
        if pressed(ui, Modifiers::NONE, Key::Delete) || pressed(ui, Modifiers::NONE, Key::X) {
            let t = self.rig.time;
            self.delete_keys_at(t);
        }
        if pressed(ui, Modifiers::COMMAND, Key::C) {
            self.copy_pose();
        }
        if pressed(ui, Modifiers::COMMAND, Key::V) {
            self.paste_pose();
        }
        let times = self.current_anim().map(Anim::times).unwrap_or_default();
        if pressed(ui, Modifiers::NONE, Key::ArrowRight) {
            let t = self.rig.time;
            if let Some(n) = times.iter().find(|&&k| k > t + 1e-3) {
                self.rig.time = *n;
            }
        }
        if pressed(ui, Modifiers::NONE, Key::ArrowLeft) {
            let t = self.rig.time;
            if let Some(n) = times.iter().rev().find(|&&k| k < t - 1e-3) {
                self.rig.time = *n;
            }
        }
        if pressed(ui, Modifiers::NONE, Key::Home) {
            self.rig.time = 0.0;
        }
    }

    fn start_rig_drag(&mut self, kind: DragKind, bone: usize, origin: Vec3, globals: &[Mat4]) {
        let parent_rot = self.data.bones[bone]
            .parent
            .and_then(|p| globals.get(p))
            .map(|g| g.to_scale_rotation_translation().1)
            .unwrap_or(Quat::IDENTITY);
        let base = self
            .current_anim()
            .filter(|_| self.rig.animating)
            .map(|a| a.sample(&self.data.bones[bone].name, self.rig.time))
            .unwrap_or((Quat::IDENTITY, Vec3::ZERO));
        self.rig.drag = Some(RigDrag {
            kind,
            bone,
            before: self.data.clone(),
            origin,
            base,
            parent_rot,
            recorded: false,
        });
    }

    fn continue_rig_drag(&mut self, drag: &mut RigDrag, p: Pos2, rect: Rect, mods: Modifiers) {
        let snapping = self.snap != mods.command;
        let (o, d) = ray(&self.cam, rect, p);
        let c = drag.origin;
        // O quanto andou no mundo (mover) ou girou (girar).
        let (delta, rot): (Vec3, Option<Quat>) = match drag.kind.clone() {
            DragKind::Axis { axis, s0 } => {
                let Some(s) = axis_param(o, d, c, axis) else {
                    return;
                };
                let mut v = s - s0;
                if snapping {
                    v = snap(v, self.grid.max(0.01) * 0.5);
                }
                (axis * v, None)
            }
            DragKind::Free { normal, hit0 } => {
                let Some(hit) = ray_plane_n_pub(o, d, c, normal) else {
                    return;
                };
                let mut v = hit - hit0;
                if snapping {
                    let g = self.grid.max(0.01) * 0.5;
                    v = Vec3::new(snap(v.x, g), snap(v.y, g), snap(v.z, g));
                }
                (v, None)
            }
            DragKind::Rotate { axis, a0, center } => {
                let v = p - center;
                let mut angle = v.y.atan2(v.x) - a0;
                if axis.dot(self.cam.pos - c) > 0.0 {
                    angle = -angle;
                }
                let mut deg = (angle.to_degrees() + 540.0).rem_euclid(360.0) - 180.0;
                if snapping {
                    deg = snap(deg, 5.0);
                }
                (Vec3::ZERO, Some(Quat::from_axis_angle(axis, deg.to_radians())))
            }
            _ => return,
        };
        if delta.length() < 1e-6 && rot.is_none_or(|q| q.angle_between(Quat::IDENTITY) < 1e-5) {
            if !drag.recorded {
                return;
            }
        }
        drag.recorded = true;
        let before = drag.before.clone();
        self.data = before;
        if self.rig.bones_mode {
            // Mover a junta: o osso e os filhos dele andam juntos.
            let b = drag.bone;
            let mut moving = vec![b];
            for i in 0..self.data.bones.len() {
                if self.data.bones[i]
                    .parent
                    .is_some_and(|p| moving.contains(&p))
                {
                    moving.push(i);
                }
            }
            for i in moving {
                let bone = &mut self.data.bones[i];
                bone.head = super::round3(bone.head + delta);
                bone.tail = super::round3(bone.tail + delta);
            }
            return;
        }
        // Posar: o giro (ou o deslocamento) entra num quadro-chave.
        let (q0, off0) = drag.base;
        let (q, off) = match rot {
            Some(r) => (drag.parent_rot.inverse() * r * drag.parent_rot * q0, off0),
            None => (q0, off0 + drag.parent_rot.inverse() * delta),
        };
        self.set_key(drag.bone, model_data::euler_degrees(q), super::round3(off));
    }

    /// Os ossos (e as alças do escolhido) por cima da vista.
    pub(super) fn rig_overlays(&self, painter: &egui::Painter, rect: Rect) {
        let mats = self.shown_matrices();
        // As peças do osso escolhido ficam contornadas.
        if let Some(b) = self.rig.sel_bone.and_then(|b| self.data.bones.get(b)) {
            for (pi, p) in self.data.parts.iter().enumerate() {
                if p.bone != b.name || !p.visible {
                    continue;
                }
                for (pts, ..) in p.all_faces() {
                    let s: Option<Vec<Pos2>> = pts
                        .iter()
                        .map(|q| to_screen(&self.cam, rect, mats[pi].transform_point3(*q)))
                        .collect();
                    if let Some(mut s) = s {
                        s.push(s[0]);
                        painter.add(egui::Shape::line(s, Stroke::new(1.0, ORANGE.gamma_multiply(0.7))));
                    }
                }
            }
        }
        let globals = self.globals();
        let screen = self.bone_screen(rect, &globals);
        for (i, s) in screen.iter().enumerate() {
            let Some((h, t)) = s else {
                continue;
            };
            let sel = self.rig.sel_bone == Some(i);
            let color = if sel { ORANGE } else { BONE };
            painter.line_segment([*h, *t], Stroke::new(5.0, Color32::from_black_alpha(140)));
            painter.line_segment([*h, *t], Stroke::new(3.0, color));
            painter.circle(*h, 5.0, color, Stroke::new(1.5, Color32::from_black_alpha(160)));
        }
        if self.rig.drag.is_none()
            && let Some(b) = self.rig.sel_bone
            && let Some(g) = globals.get(b)
        {
            self.draw_gizmo(painter, rect, g.transform_point3(Vec3::ZERO), self.rig_tool());
        }
        // O nome do osso escolhido.
        if let Some(b) = self.rig.sel_bone.and_then(|b| self.data.bones.get(b))
            && let Some(Some((h, _))) = self.rig.sel_bone.map(|i| screen.get(i).copied().flatten())
        {
            painter.text(
                h + egui::vec2(10.0, -14.0),
                egui::Align2::LEFT_BOTTOM,
                &b.name,
                egui::FontId::proportional(13.0),
                Color32::WHITE,
            );
        }
    }

    /// Anda o tempo enquanto toca.
    pub(super) fn tick(&mut self, ui: &egui::Ui) {
        if !(self.rig.animating && self.rig.playing) {
            return;
        }
        let dt = ui.input(|i| i.stable_dt).min(0.1);
        let Some(a) = self.current_anim() else {
            self.rig.playing = false;
            return;
        };
        let (dur, looping) = (a.duration.max(0.01), a.looping);
        let mut t = self.rig.time + dt;
        if t > dur {
            if looping {
                t = t.rem_euclid(dur);
            } else {
                t = dur;
                self.rig.playing = false;
            }
        }
        self.rig.time = t;
        ui.ctx().request_repaint();
    }

    // Painéis ------------------------------------------------------------

    /// O menu 💀 Esqueleto (na barra do modelador).
    pub(super) fn skeleton_menu(&mut self, ui: &mut egui::Ui) {
        let r = ui.menu_button("💀 Esqueleto", |ui| {
            ui.set_min_width(220.0);
            ui.weak(if self.has_bones() {
                "Trocar por um pronto:"
            } else {
                "Um esqueleto pronto, do tamanho do modelo:"
            });
            for (kind, label, help) in model_rig::RIGS {
                let r = ui.button(label).on_hover_text(help);
                crate::tutorial::anchor(ui.ctx(), &format!("modelo.esqueleto:{label}"), r.rect);
                if r.clicked() {
                    self.apply_rig(kind);
                    ui.close();
                }
            }
            if self.has_bones() {
                ui.separator();
                if ui
                    .button("🔗 Ligar as peças de novo")
                    .on_hover_text("Cada peça vai para o osso do nome dela (Braço esquerdo...) ou para o mais perto")
                    .clicked()
                {
                    let n = self.rebind();
                    self.actions
                        .push(ModelAction::Notice(format!("{n} peças ligadas aos ossos")));
                    ui.close();
                }
                if ui
                    .button("🗑 Tirar o esqueleto")
                    .on_hover_text("Tira os ossos e as animações (Ctrl+Z volta)")
                    .clicked()
                {
                    self.remove_rig();
                    ui.close();
                }
            }
        });
        crate::tutorial::anchor(ui.ctx(), "modelo.esqueleto", r.response.rect);
        r.response
            .on_hover_text("Ossos para animar: humanoide, quadrúpede ou um osso só");
    }

    /// A barra do modo Animar.
    pub(super) fn anim_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if ui
                .button("✏ Modelar")
                .on_hover_text("Voltar a modelar (as peças em repouso)")
                .clicked()
            {
                self.set_animating(false);
            }
            ui.separator();
            // A animação aberta.
            if let Some(name) = &mut self.rig.renaming_anim {
                let r = ui.add(egui::TextEdit::singleline(name).desired_width(120.0));
                if !r.has_focus() && !r.lost_focus() {
                    r.request_focus();
                }
                if r.lost_focus() {
                    let new = name.trim().to_string();
                    self.rig.renaming_anim = None;
                    if let Some(i) = self.rig.anim
                        && !new.is_empty()
                        && self.data.anims[i].name != new
                        && self.data.anim(&new).is_none()
                    {
                        self.record();
                        self.data.anims[i].name = new;
                    }
                }
            } else {
                let current = self
                    .current_anim()
                    .map(|a| a.name.clone())
                    .unwrap_or_else(|| "(nenhuma)".into());
                let mut pick = None;
                let r = egui::ComboBox::from_id_salt(("modelo-animacao", &self.rel))
                    .width(120.0)
                    .selected_text(current)
                    .show_ui(ui, |ui| {
                        for (i, a) in self.data.anims.iter().enumerate() {
                            if ui
                                .selectable_label(self.rig.anim == Some(i), &a.name)
                                .clicked()
                            {
                                pick = Some(i);
                            }
                        }
                    });
                crate::tutorial::anchor(ui.ctx(), "animar.lista", r.response.rect);
                if let Some(i) = pick {
                    self.rig.anim = Some(i);
                    self.rig.time = 0.0;
                }
            }
            let r = ui.menu_button("🎬 Prontas", |ui| {
                let Some(kind) = model_rig::kind_of(&self.data.bones) else {
                    return;
                };
                let r = ui.button("Todas as que faltam");
                if r.clicked() {
                    let n = self.add_all_ready();
                    self.actions.push(ModelAction::Notice(if n == 1 {
                        "1 animação pronta".into()
                    } else {
                        format!("{n} animações prontas")
                    }));
                    ui.close();
                }
                ui.separator();
                for name in model_rig::ready_names(kind) {
                    let has = self.data.anim(name).is_some();
                    let label = if has {
                        format!("{name} (trocar)")
                    } else {
                        name.to_string()
                    };
                    if ui.button(label).clicked() {
                        self.add_ready(name);
                        ui.close();
                    }
                }
            });
            crate::tutorial::anchor(ui.ctx(), "animar.prontas", r.response.rect);
            r.response.on_hover_text(
                "Animações prontas para o esqueleto. Os nomes (parado, andar, correr, pular, cair, golpe) são os que o Jogador 3D, o Inimigo 3D e o Chefe procuram.",
            );
            let r = ui.button("➕ Nova").on_hover_text("Uma animação vazia, para posar do zero");
            crate::tutorial::anchor(ui.ctx(), "animar.nova", r.rect);
            if r.clicked() {
                self.new_anim();
            }
            let has = self.current_anim().is_some();
            if ui
                .add_enabled(has, egui::Button::new("✏"))
                .on_hover_text("Renomear a animação")
                .clicked()
            {
                self.rig.renaming_anim = self.current_anim().map(|a| a.name.clone());
            }
            if ui
                .add_enabled(has, egui::Button::new("🗑"))
                .on_hover_text("Apagar a animação")
                .clicked()
            {
                self.delete_anim();
            }
            ui.separator();
            let play = if self.rig.playing { "⏸ Pausar" } else { "▶ Tocar" };
            let r = ui
                .add_enabled(has, egui::Button::new(play))
                .on_hover_text("Tocar ou pausar (Espaço)");
            crate::tutorial::anchor(ui.ctx(), "animar.tocar", r.rect);
            if r.clicked() {
                self.rig.playing = !self.rig.playing;
            }
            if let Some(i) = self.rig.anim {
                let before = self.data.clone();
                let a = &mut self.data.anims[i];
                let mut changed = ui
                    .checkbox(&mut a.looping, "🔁 Repetir")
                    .on_hover_text("Repete sem parar (andar), ou toca uma vez (pular)")
                    .changed();
                let r = ui
                    .add(
                        egui::DragValue::new(&mut a.duration)
                            .range(0.1..=60.0)
                            .speed(0.02)
                            .max_decimals(2)
                            .suffix(" s"),
                    )
                    .on_hover_text("Duração");
                changed |= r.drag_started() || (r.changed() && !r.dragged());
                if changed {
                    self.undo.push(before);
                    self.redo.clear();
                }
                ui.separator();
                for (label, tip, f, speed, key) in [
                    ("🐇 Mais rápida", "Encurta a animação", 0.8, true, "animar.rapida"),
                    ("🐢 Mais lenta", "Alonga a animação", 1.25, true, "animar.lenta"),
                    ("💪 Mais forte", "Balança mais (giros e saltos maiores)", 1.25, false, "animar.forte"),
                    ("Mais fraca", "Balança menos", 0.8, false, "animar.fraca"),
                ] {
                    let r = ui.button(label).on_hover_text(tip);
                    crate::tutorial::anchor(ui.ctx(), key, r.rect);
                    if r.clicked() {
                        self.record();
                        let a = &mut self.data.anims[i];
                        if speed {
                            model_rig::scale_time(a, f);
                            self.rig.time *= f;
                        } else {
                            model_rig::scale_intensity(a, f);
                        }
                    }
                }
                ui.separator();
                if ui.button("📋 Copiar pose").on_hover_text("Ctrl+C").clicked() {
                    self.copy_pose();
                }
                if ui
                    .add_enabled(self.rig.clip.is_some(), egui::Button::new("📋 Colar pose"))
                    .on_hover_text("Ctrl+V: um quadro-chave em cada osso, neste instante")
                    .clicked()
                {
                    self.paste_pose();
                }
            }
            ui.separator();
            for (tool, label, tip) in [
                (MTool::Rotate, "⟲ Girar", "Girar o osso (E): arraste um círculo, ou o osso"),
                (MTool::Move, "✋ Mover", "Mover a junta (W): para o quadril subir e descer"),
            ] {
                let on = self.rig_tool() == tool;
                if ui.selectable_label(on, label).on_hover_text(tip).clicked() {
                    self.tool = tool;
                }
            }
        });
    }

    /// A linha do tempo (embaixo, no modo Animar).
    pub(super) fn timeline(&mut self, ui: &mut egui::Ui) {
        let Some(a) = self.current_anim().cloned() else {
            ui.vertical_centered(|ui| {
                ui.add_space(14.0);
                ui.weak("Nenhuma animação: 🎬 Prontas põe as do esqueleto, ou ➕ Nova começa uma do zero.");
            });
            return;
        };
        let dur = a.duration.max(0.05);
        ui.horizontal(|ui| {
            ui.label(format!(
                "{} s / {} s",
                crate::scene_data::format_num(f64::from((self.rig.time * 100.0).round() / 100.0)),
                crate::scene_data::format_num(f64::from(dur))
            ));
            let bone = self
                .rig
                .sel_bone
                .and_then(|b| self.data.bones.get(b))
                .map(|b| b.name.clone());
            ui.weak(match &bone {
                Some(b) => format!("Osso: {b}. Gire com as alças para gravar um quadro aqui."),
                None => "Clique num osso (ou numa peça) para posar.".into(),
            });
        });
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 46.0), Sense::click_and_drag());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Linha do tempo")
        });
        crate::tutorial::anchor(ui.ctx(), "animar.tempo", rect);
        let painter = ui.painter_at(rect);
        let visuals = ui.visuals().clone();
        painter.rect_filled(rect, 4.0, visuals.extreme_bg_color);
        let pad = 10.0;
        let x_of = |t: f32| rect.left() + pad + (rect.width() - 2.0 * pad) * (t / dur);
        let t_of = |x: f32| ((x - rect.left() - pad) / (rect.width() - 2.0 * pad) * dur).clamp(0.0, dur);
        // Marcas a cada 0,1 s (com número a cada 0,5 s).
        let steps = (dur / 0.1).round() as i32;
        for i in 0..=steps {
            let t = i as f32 * 0.1;
            let x = x_of(t);
            let big = i % 5 == 0;
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.top() + if big { 10.0 } else { 5.0 })],
                Stroke::new(1.0, visuals.weak_text_color()),
            );
            if big && steps <= 120 {
                painter.text(
                    Pos2::new(x + 2.0, rect.top() + 1.0),
                    egui::Align2::LEFT_TOP,
                    crate::scene_data::format_num(f64::from((t * 10.0).round() / 10.0)),
                    egui::FontId::proportional(10.0),
                    visuals.weak_text_color(),
                );
            }
        }
        let bone = self
            .rig
            .sel_bone
            .and_then(|b| self.data.bones.get(b))
            .map(|b| b.name.clone());
        let all = a.times();
        let mine: Vec<f32> = bone
            .as_ref()
            .and_then(|b| a.track(b))
            .map(|tr| tr.keys.iter().map(|k| k.t).collect())
            .unwrap_or_default();
        let row_all = rect.top() + 22.0;
        let row_mine = rect.top() + 36.0;
        let diamond = |c: Pos2, r: f32, color: Color32| {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    c + egui::vec2(0.0, -r),
                    c + egui::vec2(r, 0.0),
                    c + egui::vec2(0.0, r),
                    c + egui::vec2(-r, 0.0),
                ],
                color,
                Stroke::new(1.0, Color32::from_black_alpha(160)),
            ));
        };
        for &t in &all {
            diamond(Pos2::new(x_of(t), row_all), 4.0, KEY.gamma_multiply(0.45));
        }
        for &t in &mine {
            diamond(Pos2::new(x_of(t), row_mine), 6.0, KEY);
        }
        // O cursor do tempo.
        let x = x_of(self.rig.time.min(dur));
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(2.0, ORANGE),
        );
        // Qual quadro está perto de um ponto (os do osso primeiro).
        let near = |p: Pos2| -> Option<(f32, bool)> {
            let pick = |list: &[f32], y: f32| {
                list.iter()
                    .copied()
                    .filter(|&t| (Pos2::new(x_of(t), y) - p).length() < 9.0)
                    .min_by(|a, b| (x_of(*a) - p.x).abs().total_cmp(&(x_of(*b) - p.x).abs()))
            };
            pick(&mine, row_mine)
                .map(|t| (t, true))
                .or_else(|| pick(&all, row_all).map(|t| (t, false)))
        };
        let snapping = self.snap != ui.input(|i| i.modifiers.command);
        let snap_t = |t: f32| if snapping { (snap(t, 0.05) * 1000.0).round() / 1000.0 } else { t };
        if response.drag_started_by(egui::PointerButton::Primary)
            && let Some(p) = ui.input(|i| i.pointer.press_origin())
        {
            if let Some((t, own)) = near(p) {
                self.rig.key_drag = Some(KeyDrag {
                    from: t,
                    bone: if own { bone.clone() } else { None },
                    before: self.data.clone(),
                });
            }
        }
        if let Some(p) = response.interact_pointer_pos()
            && (response.dragged() || response.clicked())
        {
            let t = snap_t(t_of(p.x));
            match &mut self.rig.key_drag {
                Some(kd) => {
                    let (from, b) = (kd.from, kd.bone.clone());
                    if (t - from).abs() > 1e-4 {
                        self.move_keys(b.as_deref(), from, t);
                        if let Some(kd) = &mut self.rig.key_drag {
                            kd.from = t;
                        }
                    }
                    self.rig.time = t;
                }
                None => {
                    self.rig.time = t;
                    self.rig.playing = false;
                }
            }
        }
        if !response.dragged()
            && let Some(kd) = self.rig.key_drag.take()
        {
            self.record_before(kd.before);
        }
        if response.secondary_clicked()
            && let Some(p) = response.interact_pointer_pos()
        {
            self.rig.key_menu = near(p).map(|(t, _)| t).or(Some(snap_t(t_of(p.x))));
        }
        let mut delete = None;
        response.context_menu(|ui| {
            let Some(t) = self.rig.key_menu else {
                ui.close();
                return;
            };
            if ui.button("🗑 Apagar o quadro").clicked() {
                delete = Some(t);
                ui.close();
            }
            if ui.button("📋 Copiar a pose daqui").clicked() {
                self.rig.time = t;
                self.copy_pose();
                ui.close();
            }
            if self.rig.clip.is_some() && ui.button("📋 Colar a pose aqui").clicked() {
                self.rig.time = t;
                self.paste_pose();
                ui.close();
            }
        });
        if let Some(t) = delete {
            self.delete_keys_at(t);
        }
    }

    /// A lista de ossos (no painel da direita).
    pub(super) fn bones_list(&mut self, ui: &mut egui::Ui) {
        if !self.has_bones() {
            return;
        }
        ui.separator();
        ui.strong("Ossos");
        let mut pick = None;
        let mut renamed = None;
        let counts: Vec<usize> = self
            .data
            .bones
            .iter()
            .map(|b| self.data.parts.iter().filter(|p| p.bone == b.name).count())
            .collect();
        for (i, b) in self.data.bones.iter().enumerate() {
            let depth = {
                let mut d = 0;
                let mut cur = b.parent;
                while let Some(p) = cur {
                    d += 1;
                    cur = self.data.bones.get(p).and_then(|b| b.parent);
                    if d > 32 {
                        break;
                    }
                }
                d
            };
            ui.horizontal(|ui| {
                ui.add_space(depth as f32 * 10.0);
                if let Some((ri, text)) = &mut self.rig.renaming_bone
                    && *ri == i
                {
                    let r = ui.text_edit_singleline(text);
                    if !r.has_focus() && !r.lost_focus() {
                        r.request_focus();
                    }
                    if r.lost_focus() {
                        renamed = Some((i, text.clone()));
                    }
                    return;
                }
                let r = ui
                    .selectable_label(self.rig.sel_bone == Some(i), &b.name)
                    .on_hover_text(format!(
                        "{} peça(s) seguem este osso. Duplo clique renomeia.",
                        counts[i]
                    ));
                crate::tutorial::anchor_in(ui, &format!("osso:{}", b.name), r.rect);
                if r.double_clicked() {
                    pick = Some((i, true));
                } else if r.clicked() {
                    pick = Some((i, false));
                }
            });
        }
        if let Some((i, rename)) = pick {
            self.rig.sel_bone = Some(i);
            if !self.rig.animating {
                self.rig.bones_mode = true;
            }
            if rename {
                self.rig.renaming_bone = Some((i, self.data.bones[i].name.clone()));
            }
        }
        if let Some((i, text)) = renamed {
            self.rig.renaming_bone = None;
            let new = text.trim().to_string();
            let old = self.data.bones[i].name.clone();
            if !new.is_empty() && new != old && self.data.bone_index(&new).is_none() {
                self.record();
                self.data.rename_bone(&old, &new);
            }
        }
    }

    /// O osso que a peça escolhida segue (no painel da peça).
    pub(super) fn part_bone_ui(&mut self, ui: &mut egui::Ui, part: usize) {
        if !self.has_bones() {
            return;
        }
        let current = self.data.parts[part].bone.clone();
        let mut pick = None;
        ui.horizontal(|ui| {
            ui.label("Segue");
            egui::ComboBox::from_id_salt(("modelo-osso-da-peca", &self.rel))
                .width(130.0)
                .selected_text(if current.is_empty() {
                    "(nenhum osso)".to_string()
                } else {
                    current.clone()
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_label(current.is_empty(), "(nenhum osso)").clicked() {
                        pick = Some(String::new());
                    }
                    for b in &self.data.bones {
                        if ui.selectable_label(current == b.name, &b.name).clicked() {
                            pick = Some(b.name.clone());
                        }
                    }
                })
                .response
                .on_hover_text("O osso que a peça segue inteira quando o modelo anima");
        });
        if let Some(b) = pick
            && b != current
        {
            self.record();
            self.data.parts[part].bone = b;
        }
    }

    /// Um quadro novo aqui, igual ao que já aparece (para começar a posar).
    pub fn key_here(&mut self, bone: usize) {
        let Some(a) = self.current_anim() else {
            return;
        };
        let name = self.data.bones[bone].name.clone();
        let (q, off) = a.sample(&name, self.rig.time);
        self.record();
        let rot = model_data::euler_degrees(q);
        self.set_key(bone, rot, off);
    }

    /// (Para os testes.) O quadro do osso no instante, se houver.
    pub fn key_at(&self, bone: &str, t: f32) -> Option<AnimKey> {
        self.current_anim()?
            .track(bone)?
            .keys
            .iter()
            .find(|k| (k.t - t).abs() < 1e-3)
            .copied()
    }
}
