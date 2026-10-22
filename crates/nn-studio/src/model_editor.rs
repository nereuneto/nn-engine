//! O modelador: a aba Modelo, onde se monta um modelo 3D low-poly (`.modelo`).
//!
//! A vista anda como a da cena 3D (botão direito + WASD, Alt gira em volta,
//! rodinha aproxima, F enquadra). Escolhe-se pontos (1), faces (2) ou peças
//! (3); as alças movem, giram e escalam. Puxar (P) tira um bloco das faces,
//! Encolher (I) faz uma face menor dentro dela, Juntar (M) junta pontos e o
//! Pincel (B) pinta faces com a cor escolhida. Ao salvar, o `.glb` com o
//! mesmo nome é gravado junto: é ele que vai para as cenas e para o jogo.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use eframe::egui::{self, Color32, Key, Modifiers, Pos2, Rect, Sense, Stroke};
use glam::{Mat4, Quat, Vec3};

use crate::model_data::{self, MID, ModelData, Part, SHAPES, face_center, face_normal};
use crate::repeat::{self, RepeatSpec};

#[path = "model_animator.rs"]
mod animator;
use crate::scene_render::{self, SceneRenderer, View3};
use crate::scene_view::scene3d::{
    BLUE, Cam3, GREEN, ORANGE, RED, arrow, axis_param, dist_to_segment, ray, ray_plane, ring, snap,
    to_screen,
};
pub use animator::RigState;

/// O que o modelador pede ao espaço de trabalho.
#[derive(Debug, Clone, PartialEq)]
pub enum ModelAction {
    Save,
    /// Gravar uma cópia deste modelo (uma variação) e abrir.
    Variation(ModelData),
    /// Um aviso rápido.
    Notice(String),
}

/// O que o modelador usa do espaço de trabalho.
#[derive(Debug)]
pub struct ModelEnv<'a> {
    pub renderer: Option<&'a mut SceneRenderer>,
    pub root: &'a Path,
    /// A biblioteca pessoal (ao lado das preferências), se houver.
    pub library: Option<&'a Path>,
}

/// O que o clique escolhe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Select {
    Points,
    Faces,
    Parts,
}

/// A ferramenta do botão esquerdo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MTool {
    Move,
    Rotate,
    Scale,
    /// Pintar faces com a cor escolhida.
    Paint,
}

#[derive(Debug, Clone)]
enum DragKind {
    Axis {
        axis: Vec3,
        s0: f32,
    },
    /// Arrastar o que está escolhido no plano de frente para a câmera.
    Free {
        normal: Vec3,
        hit0: Vec3,
    },
    Rotate {
        axis: Vec3,
        a0: f32,
        center: Pos2,
    },
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
    /// Caixa de seleção.
    Box {
        from: Pos2,
        add: bool,
    },
}

#[derive(Debug, Clone)]
struct MDrag {
    kind: DragKind,
    start: Pos2,
    before: ModelData,
    center: Vec3,
    recorded: bool,
}

/// Uma operação que segue o mouse até o clique (Esc ou o botão direito
/// desfazem).
#[derive(Debug, Clone)]
enum Modal {
    Extrude {
        before: ModelData,
        base: ModelData,
        /// Os pontos novos de cada peça.
        moved: Vec<(usize, Vec<u32>)>,
        axis: Vec3,
        origin: Vec3,
        /// Onde o mouse estava no começo (no primeiro quadro).
        start: Option<Pos2>,
    },
    Inset {
        before: ModelData,
        faces: Vec<(usize, Vec<usize>)>,
        center: Pos2,
        /// A distância do mouse ao meio no começo (no primeiro quadro).
        r0: Option<f32>,
    },
}

/// Repetindo peças (a janelinha Repetir aberta): como estava antes, quais
/// peças, como e de onde.
#[derive(Debug, Clone)]
struct PartRepeat {
    before: ModelData,
    parts: Vec<usize>,
    spec: RepeatSpec,
    origin: Vec3,
}

/// Uma face acertada pelo raio.
#[derive(Debug, Clone, Copy)]
struct FaceHit {
    part: usize,
    face: usize,
    t: f32,
}

/// Onde uma peça nova vai: no chão, ou apoiada numa face (virada para fora
/// dela).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Place {
    pub pos: Vec3,
    pub rot: Quat,
    /// O tamanho da face (para as peças que encaixam nela, como o telhado).
    pub size: Option<(f32, f32)>,
}

impl Place {
    pub fn ground(pos: Vec3) -> Self {
        Self {
            pos,
            rot: Quat::IDENTITY,
            size: None,
        }
    }
}

/// O que escolheram para pôr no modelo.
#[derive(Debug, Clone, PartialEq)]
enum AddPick {
    Shape(usize),
    Kit(usize, usize),
    Ready(usize),
    User(std::path::PathBuf),
}

/// Onde o menu do botão direito abriu.
#[derive(Debug, Clone, Copy)]
struct MenuAt {
    place: Place,
}

static NEXT_REV: AtomicU64 = AtomicU64::new(1);

pub struct ModelDoc {
    pub rel: String,
    pub data: ModelData,
    saved: ModelData,
    /// O arquivo não abriu (o texto fica intocado até corrigir).
    pub error: Option<String>,
    undo: Vec<ModelData>,
    redo: Vec<ModelData>,
    pub mode: Select,
    pub tool: MTool,
    pub sel_parts: Vec<usize>,
    pub sel_faces: Vec<(usize, usize)>,
    pub sel_verts: Vec<(usize, u32)>,
    /// A cor do pincel.
    pub color: String,
    pub cam: Cam3,
    framed: bool,
    pub snap: bool,
    pub grid: f32,
    pub show_grid: bool,
    drag: Option<MDrag>,
    modal: Option<Modal>,
    hover_face: Option<(usize, usize)>,
    hover_vert: Option<(usize, u32)>,
    fly_moved: bool,
    menu: Option<MenuAt>,
    menu_reopen: Option<MenuAt>,
    /// Abrir o menu de peças onde o mouse está (Shift+A).
    add_menu: Option<(Pos2, Place)>,
    /// Guardando na biblioteca: o nome sendo escrito.
    saving: Option<String>,
    /// Pintando com o botão segurado (o passo de desfazer já foi guardado).
    stroke: bool,
    renaming: Option<(usize, String)>,
    /// O clique que terminou de puxar ou encolher não escolhe nada.
    swallow: bool,
    rev: u64,
    /// As peças com que as malhas foram montadas.
    built: Option<Vec<Part>>,
    /// O esqueleto e o animador.
    pub rig: RigState,
    stage: crate::scene_data::SceneData,
    pub view_rect: Option<Rect>,
    actions: Vec<ModelAction>,
    repeat: Option<PartRepeat>,
}

impl std::fmt::Debug for ModelDoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelDoc")
            .field("rel", &self.rel)
            .finish_non_exhaustive()
    }
}

/// Distância (pixels) para pegar um ponto com o mouse.
const POINT_PICK: f32 = 10.0;

impl ModelDoc {
    pub fn open(root: &Path, rel: &str) -> Self {
        let (data, error) = match std::fs::read_to_string(root.join(rel)) {
            Ok(text) => match ModelData::from_lua(&text, rel) {
                Ok(d) => (d, None),
                Err(e) => (ModelData::default(), Some(e)),
            },
            Err(e) => (ModelData::default(), Some(e.to_string())),
        };
        Self::with(rel, data, error)
    }

    pub fn with(rel: &str, data: ModelData, error: Option<String>) -> Self {
        let color = data
            .parts
            .first()
            .and_then(|p| p.faces.first())
            .map(|f| f.color.clone())
            .unwrap_or_else(|| "#FF8C42".into());
        Self {
            rel: rel.to_string(),
            saved: data.clone(),
            data,
            error,
            undo: Vec::new(),
            redo: Vec::new(),
            mode: Select::Faces,
            tool: MTool::Move,
            sel_parts: Vec::new(),
            sel_faces: Vec::new(),
            sel_verts: Vec::new(),
            color,
            cam: Cam3::default(),
            framed: false,
            snap: true,
            grid: 0.25,
            show_grid: true,
            drag: None,
            modal: None,
            hover_face: None,
            hover_vert: None,
            fly_moved: false,
            menu: None,
            menu_reopen: None,
            add_menu: None,
            saving: None,
            stroke: false,
            renaming: None,
            swallow: false,
            rev: 0,
            built: None,
            rig: RigState::default(),
            stage: crate::scene_data::SceneData {
                three_d: true,
                ..Default::default()
            },
            view_rect: None,
            actions: Vec::new(),
            repeat: None,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.error.is_none() && self.data != self.saved
    }

    /// O arquivo mudou fora: relê (se não houver nada a perder).
    pub fn disk_changed(&mut self, root: &Path) {
        if self.is_dirty() {
            return;
        }
        let fresh = Self::open(root, &self.rel);
        if fresh.data != self.data || fresh.error != self.error {
            self.data = fresh.data.clone();
            self.saved = fresh.data;
            self.error = fresh.error;
            self.fix_selection();
        }
    }

    /// Grava o `.modelo` e o `.glb` dele.
    pub fn save(&mut self, root: &Path) -> Result<(), String> {
        if let Some(e) = &self.error {
            return Err(format!("o arquivo tem um erro: {e}"));
        }
        std::fs::write(root.join(&self.rel), self.data.to_lua()).map_err(|e| e.to_string())?;
        std::fs::write(
            root.join(model_data::glb_path(&self.rel)),
            self.data.to_glb(),
        )
        .map_err(|e| e.to_string())?;
        self.saved = self.data.clone();
        Ok(())
    }

    /// Guarda o modelo como está, antes de uma mudança.
    pub fn record(&mut self) {
        self.undo.push(self.data.clone());
        if self.undo.len() > crate::scene_data::MAX_HISTORY {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn record_before(&mut self, before: ModelData) {
        if before != self.data {
            self.undo.push(before);
            self.redo.clear();
        }
    }

    pub fn undo(&mut self) {
        self.cancel_modal();
        self.finish_repeat(false);
        if let Some(d) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.data, d));
            self.fix_selection();
        }
    }

    pub fn redo(&mut self) {
        self.cancel_modal();
        self.finish_repeat(false);
        if let Some(d) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.data, d));
            self.fix_selection();
        }
    }

    fn fix_selection(&mut self) {
        if self.rig.anim.is_some_and(|i| i >= self.data.anims.len()) {
            self.rig.anim = (!self.data.anims.is_empty()).then_some(0);
        }
        if self
            .rig
            .sel_bone
            .is_some_and(|i| i >= self.data.bones.len())
        {
            self.rig.sel_bone = None;
        }
        if self.data.bones.is_empty() {
            self.rig.animating = false;
            self.rig.bones_mode = false;
        }
        let parts = &self.data.parts;
        self.sel_parts.retain(|&i| i < parts.len());
        self.sel_faces
            .retain(|&(p, f)| parts.get(p).is_some_and(|p| f < p.faces.len()));
        self.sel_verts
            .retain(|&(p, v)| parts.get(p).is_some_and(|p| (v as usize) < p.verts.len()));
    }

    pub fn clear_selection(&mut self) {
        self.sel_parts.clear();
        self.sel_faces.clear();
        self.sel_verts.clear();
    }

    pub fn set_mode(&mut self, mode: Select) {
        self.rig.bones_mode = false;
        if mode == self.mode {
            return;
        }
        // O que estava escolhido passa para o novo jeito, quando dá.
        let parts: Vec<usize> = self.touched_parts();
        let verts = self.selected_verts();
        match mode {
            Select::Parts => self.sel_parts = parts,
            Select::Points => self.sel_verts = verts,
            Select::Faces => {
                if self.mode == Select::Points {
                    self.sel_faces = self.faces_of_verts(&verts);
                } else {
                    self.sel_faces.clear();
                }
            }
        }
        if mode != Select::Parts {
            self.sel_parts.clear();
        }
        if mode != Select::Faces {
            self.sel_faces.clear();
        }
        if mode != Select::Points {
            self.sel_verts.clear();
        }
        self.mode = mode;
    }

    fn faces_of_verts(&self, verts: &[(usize, u32)]) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (pi, p) in self.data.parts.iter().enumerate() {
            for (fi, f) in p.faces.iter().enumerate() {
                if f.v.iter().all(|k| verts.contains(&(pi, *k))) {
                    out.push((pi, fi));
                }
            }
        }
        out
    }

    /// As peças com algo escolhido.
    pub fn touched_parts(&self) -> Vec<usize> {
        let mut out: Vec<usize> = match self.mode {
            Select::Parts => self.sel_parts.clone(),
            Select::Faces => self.sel_faces.iter().map(|f| f.0).collect(),
            Select::Points => self.sel_verts.iter().map(|v| v.0).collect(),
        };
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Os pontos escolhidos (os das faces, no modo Faces).
    pub fn selected_verts(&self) -> Vec<(usize, u32)> {
        let mut out: Vec<(usize, u32)> = match self.mode {
            Select::Points => self.sel_verts.clone(),
            Select::Faces => self
                .sel_faces
                .iter()
                .flat_map(|&(p, f)| self.data.parts[p].faces[f].v.iter().map(move |&k| (p, k)))
                .collect(),
            Select::Parts => Vec::new(),
        };
        out.sort_unstable();
        out.dedup();
        out
    }

    fn has_selection(&self) -> bool {
        match self.mode {
            Select::Parts => !self.sel_parts.is_empty(),
            Select::Faces => !self.sel_faces.is_empty(),
            Select::Points => !self.sel_verts.is_empty(),
        }
    }

    fn part_bounds(&self, i: usize) -> Option<(Vec3, Vec3)> {
        let p = self.data.parts.get(i)?;
        let m = p.matrix();
        let mut out: Option<(Vec3, Vec3)> = None;
        for (pts, ..) in p.all_faces() {
            for q in pts {
                let w = m.transform_point3(q);
                out = Some(match out {
                    Some((a, b)) => (a.min(w), b.max(w)),
                    None => (w, w),
                });
            }
        }
        out.or(Some((p.pos, p.pos)))
    }

    /// O meio do que está escolhido (no mundo).
    pub fn selection_center(&self) -> Option<Vec3> {
        match self.mode {
            Select::Parts => {
                let mut sum = Vec3::ZERO;
                let mut n = 0.0;
                for &i in &self.sel_parts {
                    let (a, b) = self.part_bounds(i)?;
                    sum += (a + b) / 2.0;
                    n += 1.0;
                }
                (n > 0.0).then(|| sum / n)
            }
            _ => {
                let verts = self.selected_verts();
                if verts.is_empty() {
                    return None;
                }
                let sum: Vec3 = verts
                    .iter()
                    .map(|&(p, k)| {
                        let part = &self.data.parts[p];
                        part.matrix().transform_point3(part.verts[k as usize])
                    })
                    .sum();
                Some(sum / verts.len() as f32)
            }
        }
    }

    // Raio e escolha ---------------------------------------------------------

    /// A face mais perto que o raio acerta (nas peças à mostra).
    fn pick_face(&self, o: Vec3, d: Vec3) -> Option<FaceHit> {
        let mut best: Option<FaceHit> = None;
        for (pi, p) in self.data.parts.iter().enumerate() {
            if !p.visible {
                continue;
            }
            let m = p.matrix();
            for (pts, _, _, fi) in p.all_faces() {
                let w: Vec<Vec3> = pts.iter().map(|q| m.transform_point3(*q)).collect();
                for k in 1..w.len().saturating_sub(1) {
                    if let Some(t) = ray_triangle(o, d, w[0], w[k], w[k + 1])
                        && best.is_none_or(|b| t < b.t)
                    {
                        best = Some(FaceHit {
                            part: pi,
                            face: fi,
                            t,
                        });
                    }
                }
            }
        }
        best
    }

    /// O ponto do mundo está à mostra (nenhuma face na frente)?
    fn sees(&self, w: Vec3) -> bool {
        let v = w - self.cam.pos;
        let dist = v.length();
        if dist < 1e-4 {
            return true;
        }
        match self.pick_face(self.cam.pos, v / dist) {
            Some(h) => h.t >= dist - 0.002 - dist * 0.002,
            None => true,
        }
    }

    /// Os pontos das peças à mostra, onde aparecem na tela: (peça, ponto,
    /// é do espelho, tela, mundo).
    fn screen_points(&self, rect: Rect) -> Vec<(usize, u32, bool, Pos2, Vec3)> {
        let mut out = Vec::new();
        for (pi, p) in self.data.parts.iter().enumerate() {
            if !p.visible {
                continue;
            }
            let m = p.matrix();
            for (k, v) in p.verts.iter().enumerate() {
                let mut sides = vec![(false, *v)];
                if p.mirror && v.x > MID {
                    sides.push((true, Vec3::new(-v.x, v.y, v.z)));
                }
                for (mirrored, q) in sides {
                    let w = m.transform_point3(q);
                    if let Some(s) = to_screen(&self.cam, rect, w) {
                        out.push((pi, k as u32, mirrored, s, w));
                    }
                }
            }
        }
        out
    }

    fn pick_point(&self, rect: Rect, p: Pos2) -> Option<(usize, u32)> {
        let mut best: Option<(f32, usize, u32)> = None;
        for (pi, k, _, s, w) in self.screen_points(rect) {
            let d = (s - p).length();
            if d < POINT_PICK && best.is_none_or(|b| d < b.0) && self.sees(w) {
                best = Some((d, pi, k));
            }
        }
        best.map(|(_, p, k)| (p, k))
    }

    /// Onde o mouse aponta: na face embaixo dele ou no chão.
    fn point_under(&self, rect: Rect, p: Pos2) -> Vec3 {
        let (o, d) = ray(&self.cam, rect, p);
        if let Some(h) = self.pick_face(o, d) {
            return o + d * h.t;
        }
        let dist = (self.pivot() - self.cam.pos).length().max(1.0);
        ray_plane(o, d, 0.0)
            .filter(|h| (*h - self.cam.pos).length() < dist * 3.0)
            .unwrap_or(self.cam.pos + d * dist)
    }

    fn click_select(&mut self, rect: Rect, p: Pos2, add: bool) {
        let (o, d) = ray(&self.cam, rect, p);
        match self.mode {
            Select::Points => {
                let hit = self.pick_point(rect, p);
                toggle(&mut self.sel_verts, hit, add);
            }
            Select::Faces => {
                let hit = self.pick_face(o, d).map(|h| (h.part, h.face));
                toggle(&mut self.sel_faces, hit, add);
            }
            Select::Parts => {
                let hit = self.pick_face(o, d).map(|h| h.part);
                toggle(&mut self.sel_parts, hit, add);
            }
        }
    }

    fn box_select(&mut self, rect: Rect, area: Rect, add: bool) {
        if !add {
            self.clear_selection();
        }
        match self.mode {
            Select::Points => {
                for (pi, k, _, s, w) in self.screen_points(rect) {
                    if area.contains(s) && self.sees(w) && !self.sel_verts.contains(&(pi, k)) {
                        self.sel_verts.push((pi, k));
                    }
                }
            }
            Select::Faces => {
                for (pi, p) in self.data.parts.iter().enumerate() {
                    if !p.visible {
                        continue;
                    }
                    let m = p.matrix();
                    for (pts, _, _, fi) in p.all_faces() {
                        let idx: Vec<u32> = (0..pts.len() as u32).collect();
                        let c = m.transform_point3(face_center(&pts, &idx));
                        let n = m.transform_vector3(face_normal(&pts, &idx));
                        let front = n.dot(c - self.cam.pos) < 0.0;
                        let inside =
                            to_screen(&self.cam, rect, c).is_some_and(|s| area.contains(s));
                        if front && inside && self.sees(c) && !self.sel_faces.contains(&(pi, fi)) {
                            self.sel_faces.push((pi, fi));
                        }
                    }
                }
            }
            Select::Parts => {
                for i in 0..self.data.parts.len() {
                    if !self.data.parts[i].visible {
                        continue;
                    }
                    let Some((a, b)) = self.part_bounds(i) else {
                        continue;
                    };
                    let inside =
                        to_screen(&self.cam, rect, (a + b) / 2.0).is_some_and(|s| area.contains(s));
                    if inside && !self.sel_parts.contains(&i) {
                        self.sel_parts.push(i);
                    }
                }
            }
        }
    }

    pub fn select_all(&mut self) {
        self.clear_selection();
        for (pi, p) in self.data.parts.iter().enumerate() {
            match self.mode {
                Select::Parts => self.sel_parts.push(pi),
                Select::Faces => self.sel_faces.extend((0..p.faces.len()).map(|f| (pi, f))),
                Select::Points => self
                    .sel_verts
                    .extend((0..p.verts.len() as u32).map(|k| (pi, k))),
            }
        }
    }

    // Câmera -----------------------------------------------------------------

    pub fn frame(&mut self) {
        let bounds = match self.mode {
            Select::Parts if !self.sel_parts.is_empty() => self
                .sel_parts
                .iter()
                .filter_map(|&i| self.part_bounds(i))
                .fold(None, |acc: Option<(Vec3, Vec3)>, (a, b)| {
                    Some(acc.map_or((a, b), |(x, y)| (x.min(a), y.max(b))))
                }),
            Select::Faces | Select::Points if self.has_selection() => {
                let mut out: Option<(Vec3, Vec3)> = None;
                for (p, k) in self.selected_verts() {
                    let part = &self.data.parts[p];
                    let w = part.matrix().transform_point3(part.verts[k as usize]);
                    out = Some(out.map_or((w, w), |(a, b)| (a.min(w), b.max(w))));
                }
                out
            }
            _ => self.data.bounds(),
        };
        let (lo, hi) = bounds.unwrap_or((Vec3::splat(-0.5), Vec3::splat(0.5)));
        let center = (lo + hi) / 2.0;
        let radius = ((hi - lo).length() / 2.0).max(0.4);
        let dist = radius * 2.2 + 0.5;
        let dir = if self.cam.forward().y > -0.2 {
            Vec3::new(-0.55, -0.45, -0.7).normalize()
        } else {
            self.cam.forward()
        };
        let speed = (radius * 2.0).clamp(1.0, 20.0);
        self.cam.look_at(center - dir * dist, center);
        self.cam.speed = speed;
    }

    fn pivot(&self) -> Vec3 {
        self.selection_center()
            .or_else(|| self.data.bounds().map(|(a, b)| (a + b) / 2.0))
            .unwrap_or(Vec3::ZERO)
    }

    // Edição -----------------------------------------------------------------

    /// Uma peça nova (forma pronta) no chão, em `at`, escolhida.
    pub fn add_part(&mut self, shape: usize, at: Vec3) -> usize {
        self.add_shape(shape, self.ground_place(at))
    }

    /// Onde fica algo posto no chão em `at` (na grade).
    fn ground_place(&self, at: Vec3) -> Place {
        let step = self.grid.max(0.01);
        Place::ground(Vec3::new(
            snap(at.x, step),
            snap(at.y.max(0.0), step),
            snap(at.z, step),
        ))
    }

    /// Uma forma pronta em `place`, escolhida.
    pub fn add_shape(&mut self, shape: usize, place: Place) -> usize {
        let (label, _, make) = SHAPES[shape.min(SHAPES.len() - 1)];
        let p = make(label, &self.color);
        self.insert_parts(vec![p], place, false)[0]
    }

    /// Põe peças no modelo em `place` (com `fit`, uma peça de 1 × 1 m fica do
    /// tamanho da face). Escolhe e devolve as novas.
    pub fn insert_parts(&mut self, parts: Vec<Part>, place: Place, fit: bool) -> Vec<usize> {
        if parts.is_empty() {
            return Vec::new();
        }
        self.cancel_modal();
        self.finish_repeat(false);
        self.record();
        let stretch = match (fit, place.size) {
            (true, Some((w, d))) => Vec3::new(w.max(0.05), 1.0, d.max(0.05)),
            _ => Vec3::ONE,
        };
        let t = Mat4::from_rotation_translation(place.rot, place.pos) * Mat4::from_scale(stretch);
        let mut new = Vec::new();
        for mut p in parts {
            let (scale, q, pos) = (t * p.matrix()).to_scale_rotation_translation();
            p.pos = round3(pos);
            p.rot = model_data::euler_degrees(q);
            p.scale = round3(scale);
            p.name = unique_name(&self.data, &p.name);
            self.data.parts.push(p);
            new.push(self.data.parts.len() - 1);
        }
        self.set_mode(Select::Parts);
        self.sel_parts = new.clone();
        new
    }

    /// Onde vai uma peça posta com o mouse em `p`: apoiada na face embaixo
    /// dele, ou no chão.
    fn place_at(&self, rect: Rect, p: Pos2) -> Place {
        let (o, d) = ray(&self.cam, rect, p);
        if let Some(h) = self.pick_face(o, d)
            && let Some(place) = self.face_place(h.part, h.face)
        {
            return place;
        }
        self.ground_place(self.point_under(rect, p))
    }

    /// Apoiado no meio de uma face, virado para fora dela.
    pub fn face_place(&self, part: usize, face: usize) -> Option<Place> {
        let p = self.data.parts.get(part)?;
        let f = p.faces.get(face)?;
        let m = p.matrix();
        let w: Vec<Vec3> =
            f.v.iter()
                .map(|&k| m.transform_point3(p.verts[k as usize]))
                .collect();
        let idx: Vec<u32> = (0..w.len() as u32).collect();
        let c = face_center(&w, &idx);
        let n = face_normal(&w, &idx);
        if n == Vec3::ZERO {
            return None;
        }
        let rot = Quat::from_rotation_arc(Vec3::Y, n);
        let inv = rot.inverse();
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for q in &w {
            let l = inv * (*q - c);
            lo = lo.min(l);
            hi = hi.max(l);
        }
        Some(Place {
            pos: round3(c),
            rot,
            size: Some((hi.x - lo.x, hi.z - lo.z)),
        })
    }

    /// Põe o que foi escolhido num menu de peças.
    fn add_pick(&mut self, pick: AddPick, place: Place) {
        match pick {
            AddPick::Shape(i) => {
                self.add_shape(i, place);
            }
            AddPick::Kit(k, i) => {
                if let Some(item) = crate::model_kits::KITS.get(k).and_then(|k| k.items.get(i)) {
                    self.insert_parts((item.make)(), place, item.fit);
                }
            }
            AddPick::Ready(i) => {
                if let Some(r) = crate::model_kits::READY.get(i) {
                    let m = (r.make)();
                    self.add_palette(&m);
                    self.insert_parts(m.parts, place, false);
                }
            }
            AddPick::User(path) => match crate::model_kits::read_user(&path) {
                Ok(m) => {
                    self.add_palette(&m);
                    self.insert_parts(m.parts, place, false);
                }
                Err(e) => self
                    .actions
                    .push(ModelAction::Notice(format!("Não deu para abrir: {e}"))),
            },
        }
    }

    /// As cores de outro modelo que faltam na paleta entram nela.
    fn add_palette(&mut self, m: &ModelData) {
        for p in &m.parts {
            for f in &p.faces {
                if !self.data.palette.contains(&f.color) {
                    self.data.palette.push(f.color.clone());
                }
            }
        }
    }

    /// Troca uma cor por outra no modelo todo.
    pub fn replace_color(&mut self, from: &str, to: &str) -> usize {
        let n: usize = self
            .data
            .parts
            .iter()
            .flat_map(|p| &p.faces)
            .filter(|f| f.color == from)
            .count();
        if n == 0 || from == to {
            return 0;
        }
        self.record();
        for p in &mut self.data.parts {
            for f in &mut p.faces {
                if f.color == from {
                    f.color = to.to_string();
                }
            }
        }
        n
    }

    /// Escolhe as faces de uma cor.
    pub fn select_color(&mut self, color: &str) {
        self.set_mode(Select::Faces);
        self.sel_faces = self
            .data
            .parts
            .iter()
            .enumerate()
            .filter(|(_, p)| p.visible)
            .flat_map(|(pi, p)| {
                p.faces
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.color == color)
                    .map(move |(fi, _)| (pi, fi))
            })
            .collect();
    }

    /// As peças para guardar na biblioteca: as escolhidas (ou todas).
    pub fn parts_to_save(&self) -> ModelData {
        let chosen = self.touched_parts();
        if chosen.is_empty() {
            // O modelo todo vai com o esqueleto e as animações.
            return self.data.clone();
        }
        ModelData {
            parts: chosen.iter().map(|&i| self.data.parts[i].clone()).collect(),
            palette: self.data.palette.clone(),
            ..Default::default()
        }
    }

    /// Apaga o que está escolhido.
    pub fn delete_selection(&mut self) {
        match self.mode {
            Select::Parts => {
                if self.sel_parts.is_empty() {
                    return;
                }
                self.record();
                let mut idx = std::mem::take(&mut self.sel_parts);
                idx.sort_unstable();
                for i in idx.into_iter().rev() {
                    self.data.parts.remove(i);
                }
            }
            Select::Faces => {
                if self.sel_faces.is_empty() {
                    return;
                }
                self.record();
                for pi in self.touched_parts() {
                    let faces: Vec<usize> = self
                        .sel_faces
                        .iter()
                        .filter(|f| f.0 == pi)
                        .map(|f| f.1)
                        .collect();
                    self.data.parts[pi].delete_faces(&faces);
                }
                self.sel_faces.clear();
            }
            Select::Points => {
                if self.sel_verts.is_empty() {
                    return;
                }
                self.record();
                for pi in self.touched_parts() {
                    let verts: Vec<u32> = self
                        .sel_verts
                        .iter()
                        .filter(|v| v.0 == pi)
                        .map(|v| v.1)
                        .collect();
                    let p = &mut self.data.parts[pi];
                    let faces: Vec<usize> = p
                        .faces
                        .iter()
                        .enumerate()
                        .filter(|(_, f)| f.v.iter().any(|k| verts.contains(k)))
                        .map(|(i, _)| i)
                        .collect();
                    p.delete_faces(&faces);
                }
                self.sel_verts.clear();
            }
        }
        self.fix_selection();
    }

    /// Junta os pontos escolhidos (de cada peça) num só.
    pub fn merge_selection(&mut self) -> bool {
        let verts = self.selected_verts();
        let parts = self.touched_parts();
        if verts.len() < 2 || self.mode == Select::Parts {
            return false;
        }
        self.record();
        for pi in parts {
            let ks: Vec<u32> = verts.iter().filter(|v| v.0 == pi).map(|v| v.1).collect();
            self.data.parts[pi].merge_verts(&ks);
        }
        self.sel_faces.clear();
        self.sel_verts.clear();
        self.fix_selection();
        true
    }

    /// Pinta as faces escolhidas (ou as peças inteiras) com a cor.
    pub fn paint_selection(&mut self, color: &str) {
        let targets: Vec<(usize, usize)> = match self.mode {
            Select::Faces => self.sel_faces.clone(),
            Select::Parts => self
                .sel_parts
                .iter()
                .flat_map(|&p| (0..self.data.parts[p].faces.len()).map(move |f| (p, f)))
                .collect(),
            Select::Points => self.faces_of_verts(&self.sel_verts),
        };
        if targets.is_empty() {
            return;
        }
        self.record();
        for (p, f) in targets {
            self.data.parts[p].faces[f].color = color.to_string();
        }
    }

    /// Duplica as peças escolhidas (ao lado).
    pub fn duplicate_parts(&mut self) {
        let parts = self.touched_parts();
        if parts.is_empty() {
            return;
        }
        self.record();
        let mut new = Vec::new();
        for i in parts {
            let mut p = self.data.parts[i].clone();
            let (a, b) = self.part_bounds(i).unwrap_or((Vec3::ZERO, Vec3::ONE));
            p.pos.x += snap((b.x - a.x).max(self.grid), self.grid.max(0.01));
            p.name = unique_name(&self.data, &self.data.parts[i].name);
            self.data.parts.push(p);
            new.push(self.data.parts.len() - 1);
        }
        self.set_mode(Select::Parts);
        self.sel_parts = new;
    }

    /// Liga ou desliga o espelho em X nas peças escolhidas.
    pub fn toggle_mirror(&mut self) {
        let parts = self.touched_parts();
        let parts = if parts.is_empty() && self.data.parts.len() == 1 {
            vec![0]
        } else {
            parts
        };
        if parts.is_empty() {
            return;
        }
        self.record();
        let on = !parts.iter().all(|&i| self.data.parts[i].mirror);
        for i in parts {
            let p = &mut self.data.parts[i];
            if on {
                p.mirror_on();
            } else {
                p.mirror_apply();
            }
        }
        self.sel_faces.clear();
        self.sel_verts.clear();
        self.fix_selection();
    }

    /// Abre o Repetir com as peças escolhidas (em linha, ao lado delas).
    pub fn start_repeat(&mut self) -> bool {
        let parts = self.touched_parts();
        if parts.is_empty() {
            return false;
        }
        self.cancel_modal();
        self.finish_repeat(false);
        let (lo, hi) = parts.iter().filter_map(|&i| self.part_bounds(i)).fold(
            (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
            |(a, b), (c, d)| (a.min(c), b.max(d)),
        );
        let size = (hi - lo).max(Vec3::splat(0.1));
        let step = self.grid.max(0.05);
        // Uma encostada na outra; o campo Distância afasta.
        let mut spec = RepeatSpec::line(3, ((size.x * 100.0).round() / 100.0).max(0.05));
        spec.radius = snap(size.x.max(size.z) * 1.5, step).max(0.5);
        self.set_mode(Select::Parts);
        self.sel_parts = parts.clone();
        self.repeat = Some(PartRepeat {
            before: self.data.clone(),
            parts,
            spec,
            origin: (lo + hi) / 2.0,
        });
        self.apply_repeat();
        true
    }

    /// Refaz as cópias da prévia do Repetir.
    fn apply_repeat(&mut self) {
        let Some(r) = self.repeat.clone() else {
            return;
        };
        let mut data = r.before.clone();
        let mut sel = r.parts.clone();
        for i in 1..=r.spec.count {
            let t = r.spec.transform(i, r.origin);
            for &pi in &r.parts {
                let Some(base) = r.before.parts.get(pi) else {
                    continue;
                };
                let mut p = base.clone();
                let (_, q, pos) = (t * base.matrix()).to_scale_rotation_translation();
                p.pos = round3(pos);
                p.rot = model_data::euler_degrees(q);
                p.name = unique_name(&data, &base.name);
                data.parts.push(p);
                sel.push(data.parts.len() - 1);
            }
        }
        self.data = data;
        self.sel_parts = sel;
    }

    /// Muda quantas cópias o Repetir aberto faz (como no campo Cópias).
    pub fn set_repeat_count(&mut self, n: u32) {
        if let Some(r) = &mut self.repeat {
            r.spec.count = n.clamp(1, 200);
            self.apply_repeat();
        }
    }

    /// Fecha o Repetir: fica com as cópias (`apply`) ou volta como estava.
    pub fn finish_repeat(&mut self, apply: bool) {
        if let Some(r) = self.repeat.take() {
            if apply {
                self.record_before(r.before);
            } else {
                self.data = r.before;
                self.sel_parts = r.parts;
                self.fix_selection();
            }
        }
    }

    /// A janelinha do Repetir.
    fn repeat_window(&mut self, ui: &egui::Ui) {
        let Some(view) = self.view_rect else {
            return;
        };
        let Some(r) = &mut self.repeat else {
            return;
        };
        let mut changed = false;
        let mut done = None;
        let n = r.parts.len();
        let shown = egui::Window::new("🔁 Repetir")
            .id(egui::Id::new(("modelo-repetir", &self.rel)))
            .collapsible(false)
            .resizable(false)
            .default_pos(view.left_top() + egui::vec2(12.0, 12.0))
            .show(ui.ctx(), |ui| {
                ui.weak(if n == 1 {
                    "1 peça escolhida".to_string()
                } else {
                    format!("{n} peças escolhidas")
                });
                changed = repeat::spec_ui(ui, &mut r.spec, true, "m", true);
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
        if let Some(w) = shown {
            crate::tutorial::anchor(ui.ctx(), "repetir", w.response.rect);
        }
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

    /// A janelinha do ⭐ Guardar na biblioteca (o nome).
    fn save_window(&mut self, ui: &egui::Ui, library: Option<&Path>) {
        let Some(name) = &mut self.saving else {
            return;
        };
        let mut done = None;
        let r =
            egui::Modal::new(egui::Id::new(("modelo-guardar", &self.rel))).show(ui.ctx(), |ui| {
                ui.set_width(320.0);
                ui.heading("⭐ Guardar na biblioteca");
                ui.label("Nome:");
                let r = ui.add(egui::TextEdit::singleline(name).desired_width(f32::INFINITY));
                if !r.has_focus() && !r.lost_focus() {
                    r.request_focus();
                }
                ui.weak("Fica em ➕ Peça > 📚 Biblioteca, em todos os modelos e projetos.");
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let ok = ui.add(
                        egui::Button::new("Guardar").fill(crate::theme::ACCENT.gamma_multiply(0.7)),
                    );
                    if ok.clicked() || ui.input(|i| i.key_pressed(Key::Enter)) {
                        done = Some(true);
                    }
                    if ui.button("Cancelar").clicked() {
                        done = Some(false);
                    }
                });
            });
        if r.should_close() && done.is_none() {
            done = Some(false);
        }
        let Some(ok) = done else {
            return;
        };
        let name = self.saving.take().unwrap_or_default();
        if !ok {
            return;
        }
        let Some(dir) = library else {
            self.actions.push(ModelAction::Notice(
                "Sem pasta de preferências: não dá para guardar na biblioteca.".into(),
            ));
            return;
        };
        match crate::model_kits::save_user(dir, &name, &self.parts_to_save()) {
            Ok(path) => {
                let file = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.actions.push(ModelAction::Notice(format!(
                    "⭐ {file} guardado na biblioteca"
                )));
            }
            Err(e) => self.actions.push(ModelAction::Notice(format!(
                "Não deu para guardar na biblioteca: {e}"
            ))),
        }
    }

    /// Começa a puxar as faces escolhidas.
    pub fn start_extrude(&mut self, mouse: Pos2) -> bool {
        if self.mode != Select::Faces || self.sel_faces.is_empty() {
            return false;
        }
        let before = self.data.clone();
        let mut moved = Vec::new();
        let mut axis = Vec3::ZERO;
        for pi in self.touched_parts() {
            let faces: Vec<usize> = self
                .sel_faces
                .iter()
                .filter(|f| f.0 == pi)
                .map(|f| f.1)
                .collect();
            let p = &mut self.data.parts[pi];
            let (ks, n) = p.extrude(&faces);
            axis += p.matrix().transform_vector3(n).normalize_or_zero();
            moved.push((pi, ks));
        }
        let axis = axis.normalize_or(Vec3::Y);
        let origin = self.selection_center().unwrap_or(Vec3::ZERO);
        self.modal = Some(Modal::Extrude {
            before,
            base: self.data.clone(),
            moved,
            axis,
            origin,
            start: None,
        });
        let _ = mouse;
        true
    }

    /// Começa a encolher as faces escolhidas.
    pub fn start_inset(&mut self, rect: Rect, mouse: Pos2) -> bool {
        if self.mode != Select::Faces || self.sel_faces.is_empty() {
            return false;
        }
        let Some(c) = self
            .selection_center()
            .and_then(|c| to_screen(&self.cam, rect, c))
        else {
            return false;
        };
        let faces = self
            .touched_parts()
            .into_iter()
            .map(|pi| {
                let f: Vec<usize> = self
                    .sel_faces
                    .iter()
                    .filter(|f| f.0 == pi)
                    .map(|f| f.1)
                    .collect();
                (pi, f)
            })
            .collect();
        self.modal = Some(Modal::Inset {
            before: self.data.clone(),
            faces,
            center: c,
            r0: None,
        });
        let _ = mouse;
        true
    }

    /// Puxa ou encolhe conforme o mouse.
    fn update_modal(&mut self, rect: Rect, p: Pos2, mods: Modifiers) {
        let snapping = self.snap != mods.command;
        let grid = self.grid;
        let Some(modal) = &mut self.modal else {
            return;
        };
        match modal {
            Modal::Extrude {
                base,
                moved,
                axis,
                origin,
                start,
                ..
            } => {
                let start = *start.get_or_insert(p);
                // O mouse anda junto com a direção de puxar, como ela aparece
                // na tela (olhando bem de frente para ela: pela altura).
                let a = to_screen(&self.cam, rect, *origin);
                let b = to_screen(&self.cam, rect, *origin + *axis);
                let moved_px = p - start;
                let dist = match (a, b) {
                    (Some(a), Some(b)) if (b - a).length() > 8.0 => {
                        let dir = b - a;
                        moved_px.dot(dir) / dir.length_sq()
                    }
                    _ => -moved_px.y / 100.0,
                };
                let dist = if snapping { snap(dist, grid) } else { dist };
                let mut data = base.clone();
                for (pi, ks) in moved.iter() {
                    let part = &mut data.parts[*pi];
                    let local = part.matrix().inverse().transform_vector3(*axis * dist);
                    for &k in ks {
                        let v = &mut part.verts[k as usize];
                        let mid = part.mirror && v.x.abs() <= MID;
                        *v += local;
                        if mid {
                            v.x = 0.0;
                        } else if part.mirror {
                            v.x = v.x.max(0.0);
                        }
                    }
                }
                self.data = data;
            }
            Modal::Inset {
                before,
                faces,
                center,
                r0,
            } => {
                let r0 = *r0.get_or_insert((p - *center).length().max(60.0));
                let t = (1.0 - (p - *center).length() / r0).clamp(0.0, 0.9);
                let t = if snapping { snap(t, 0.05) } else { t };
                let mut data = before.clone();
                for (pi, f) in faces.iter() {
                    data.parts[*pi].inset(f, t.max(0.02));
                }
                self.data = data;
            }
        }
    }

    fn confirm_modal(&mut self) {
        if let Some(m) = self.modal.take() {
            let before = match m {
                Modal::Extrude { before, .. } | Modal::Inset { before, .. } => before,
            };
            self.record_before(before);
        }
    }

    fn cancel_modal(&mut self) {
        if let Some(m) = self.modal.take() {
            self.data = match m {
                Modal::Extrude { before, .. } | Modal::Inset { before, .. } => before,
            };
            self.fix_selection();
        }
    }

    /// Aplica um movimento (no mundo) ao que está escolhido, a partir de
    /// como estava no começo do arrasto.
    fn transform_selection(&mut self, before: &ModelData, t: Mat4, keep_scale: bool) {
        match self.mode {
            Select::Parts => {
                for &i in &self.sel_parts.clone() {
                    let Some(b) = before.parts.get(i) else {
                        continue;
                    };
                    let (s, q, pos) = (t * b.matrix()).to_scale_rotation_translation();
                    let p = &mut self.data.parts[i];
                    p.pos = round3(pos);
                    p.rot = model_data::euler_degrees(q);
                    p.scale = if keep_scale { b.scale } else { round3(s) };
                }
            }
            _ => {
                for (pi, k) in self.selected_verts() {
                    let Some(b) = before.parts.get(pi) else {
                        continue;
                    };
                    let m = b.matrix();
                    let v0 = b.verts[k as usize];
                    let w = t.transform_point3(m.transform_point3(v0));
                    let mut v = round3(m.inverse().transform_point3(w));
                    if b.mirror {
                        if v0.x.abs() <= MID {
                            v.x = 0.0;
                        } else {
                            v.x = v.x.max(0.0);
                        }
                    }
                    self.data.parts[pi].verts[k as usize] = v;
                }
            }
        }
    }

    // Alças ------------------------------------------------------------------

    fn gizmo_len(&self, center: Vec3) -> f32 {
        (center - self.cam.pos).length().max(0.3) * 0.16
    }

    /// A rotação das alças de escalar (a da peça, no modo Peças).
    fn gizmo_rotation(&self) -> Quat {
        match self.mode {
            Select::Parts => self
                .sel_parts
                .last()
                .and_then(|&i| self.data.parts.get(i))
                .map(|p| p.rotation())
                .unwrap_or(Quat::IDENTITY),
            _ => Quat::IDENTITY,
        }
    }

    fn gizmo_hit(&self, rect: Rect, p: Pos2) -> Option<DragKind> {
        let center = self.selection_center()?;
        self.gizmo_hit_at(rect, p, center, self.tool)
    }

    /// Qual alça (da ferramenta `tool`, com o meio em `center`) está embaixo
    /// do mouse.
    fn gizmo_hit_at(&self, rect: Rect, p: Pos2, center: Vec3, tool: MTool) -> Option<DragKind> {
        let len = self.gizmo_len(center);
        let c = to_screen(&self.cam, rect, center)?;
        let (o, d) = ray(&self.cam, rect, p);
        match tool {
            MTool::Paint => None,
            MTool::Move => {
                for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
                    let Some(tip) = to_screen(&self.cam, rect, center + axis * len) else {
                        continue;
                    };
                    if dist_to_segment(p, c, tip) < 8.0 {
                        let s0 = axis_param(o, d, center, axis)?;
                        return Some(DragKind::Axis { axis, s0 });
                    }
                }
                if (p - c).length() < 10.0 {
                    let normal = self.cam.forward();
                    let hit0 = ray_plane_n(o, d, center, normal)?;
                    return Some(DragKind::Free { normal, hit0 });
                }
                None
            }
            MTool::Rotate => {
                let mut best: Option<(f32, Vec3)> = None;
                for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
                    let pts = ring(center, axis, len);
                    for w in pts.windows(2) {
                        if let (Some(a), Some(b)) = (
                            to_screen(&self.cam, rect, w[0]),
                            to_screen(&self.cam, rect, w[1]),
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
                Some(DragKind::Rotate {
                    axis,
                    a0: v.y.atan2(v.x),
                    center: c,
                })
            }
            MTool::Scale => {
                if (p - c).length() < 10.0 {
                    return Some(DragKind::Scale {
                        axis: 3,
                        s0: p.y,
                        len,
                    });
                }
                let q = self.gizmo_rotation();
                for (i, axis) in [Vec3::X, Vec3::Y, Vec3::Z].into_iter().enumerate() {
                    let a = q * axis;
                    let Some(tip) = to_screen(&self.cam, rect, center + a * len) else {
                        continue;
                    };
                    if dist_to_segment(p, c, tip) < 8.0 {
                        let s0 = axis_param(o, d, center, a)?;
                        return Some(DragKind::Scale { axis: i, s0, len });
                    }
                }
                None
            }
        }
    }

    fn start_drag(&mut self, kind: DragKind, start: Pos2) {
        self.drag = Some(MDrag {
            kind,
            start,
            before: self.data.clone(),
            center: self.selection_center().unwrap_or(Vec3::ZERO),
            recorded: false,
        });
    }

    fn continue_drag(&mut self, drag: &mut MDrag, p: Pos2, rect: Rect, mods: Modifiers) {
        let snapping = self.snap != mods.command;
        let step = self.grid.max(0.01);
        let (o, d) = ray(&self.cam, rect, p);
        let c = drag.center;
        let moved = |doc: &mut Self, drag: &mut MDrag, t: Mat4, keep: bool| {
            drag.recorded = true;
            let before = drag.before.clone();
            doc.transform_selection(&before, t, keep);
        };
        match drag.kind.clone() {
            DragKind::Look | DragKind::Box { .. } => {}
            DragKind::Pan { dist } => {
                let delta = p - drag.start;
                drag.start = p;
                let k = dist * 0.0018;
                self.cam.pos += (-self.cam.right() * delta.x + self.cam.up() * delta.y) * k;
            }
            DragKind::Orbit { pivot } => {
                let delta = p - drag.start;
                drag.start = p;
                let dist = (self.cam.pos - pivot).length().max(0.3);
                let (yaw, pitch) = (self.cam.yaw, self.cam.pitch);
                self.cam.yaw = (yaw - delta.x * 0.3).rem_euclid(360.0);
                self.cam.pitch = (pitch - delta.y * 0.3).clamp(-89.0, 89.0);
                self.cam.pos = pivot - self.cam.forward() * dist;
            }
            DragKind::Axis { axis, s0 } => {
                let Some(s) = axis_param(o, d, c, axis) else {
                    return;
                };
                let mut dist = s - s0;
                if snapping {
                    dist = snap(dist, step);
                }
                if dist.abs() < 1e-6 && !drag.recorded {
                    return;
                }
                moved(self, drag, Mat4::from_translation(axis * dist), true);
            }
            DragKind::Free { normal, hit0 } => {
                let Some(hit) = ray_plane_n(o, d, c, normal) else {
                    return;
                };
                let mut delta = hit - hit0;
                if snapping {
                    delta = Vec3::new(
                        snap(delta.x, step),
                        snap(delta.y, step),
                        snap(delta.z, step),
                    );
                }
                if delta.length() < 1e-6 && !drag.recorded {
                    return;
                }
                moved(self, drag, Mat4::from_translation(delta), true);
            }
            DragKind::Rotate { axis, a0, center } => {
                let v = p - center;
                let mut angle = v.y.atan2(v.x) - a0;
                if axis.dot(self.cam.pos - c) > 0.0 {
                    angle = -angle;
                }
                let mut deg = (angle.to_degrees() + 540.0).rem_euclid(360.0) - 180.0;
                if snapping {
                    deg = snap(deg, 15.0);
                }
                if deg.abs() < 1e-4 && !drag.recorded {
                    return;
                }
                let t = Mat4::from_translation(c)
                    * Mat4::from_quat(Quat::from_axis_angle(axis, deg.to_radians()))
                    * Mat4::from_translation(-c);
                moved(self, drag, t, true);
            }
            DragKind::Scale { axis, s0, len } => {
                let q = self.gizmo_rotation();
                let factor = if axis == 3 {
                    1.0 + (s0 - p.y) / 120.0
                } else {
                    let a = q * [Vec3::X, Vec3::Y, Vec3::Z][axis];
                    let Some(s) = axis_param(o, d, c, a) else {
                        return;
                    };
                    1.0 + (s - s0) / len.max(0.01)
                };
                let mut factor = factor.max(0.0);
                if snapping {
                    factor = snap(factor, 0.1);
                }
                if (factor - 1.0).abs() < 1e-5 && !drag.recorded {
                    return;
                }
                if self.mode == Select::Parts {
                    drag.recorded = true;
                    for &i in &self.sel_parts.clone() {
                        let Some(b) = drag.before.parts.get(i) else {
                            continue;
                        };
                        let mut s = b.scale;
                        for k in 0..3 {
                            if axis == 3 || axis == k {
                                s[k] = (b.scale[k] * factor).max(0.01);
                            }
                        }
                        // Escalar em volta do meio da seleção.
                        let off = b.pos - c;
                        let mut ns = Vec3::ONE;
                        for k in 0..3 {
                            if axis == 3 || axis == k {
                                ns[k] = factor.max(0.01);
                            }
                        }
                        let p = &mut self.data.parts[i];
                        p.scale = round3(s);
                        p.pos = round3(c + if axis == 3 { off * ns } else { off });
                    }
                } else {
                    let mut v = Vec3::ONE;
                    for k in 0..3 {
                        if axis == 3 || axis == k {
                            v[k] = factor;
                        }
                    }
                    let t = Mat4::from_translation(c)
                        * Mat4::from_scale(v)
                        * Mat4::from_translation(-c);
                    moved(self, drag, t, true);
                }
            }
        }
    }

    // Interface --------------------------------------------------------------

    pub fn ui(&mut self, ui: &mut egui::Ui, env: &mut ModelEnv) -> Vec<ModelAction> {
        if let Some(e) = &self.error {
            ui.colored_label(
                crate::theme::ERROR,
                format!("Não foi possível abrir o modelo: {e}"),
            );
            ui.label("Corrija o arquivo (ele é um texto em Lua) e abra de novo.");
            return Vec::new();
        }
        if self.rig.animating {
            self.anim_toolbar(ui);
            egui::Panel::bottom(egui::Id::new(("modelo-tempo", &self.rel)))
                .resizable(false)
                .show(ui, |ui| self.timeline(ui));
        } else {
            self.toolbar(ui, env.library);
        }
        egui::Panel::right(egui::Id::new(("modelo-lado", &self.rel)))
            .resizable(true)
            .default_size(190.0)
            .min_size(150.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("modelo-lado-rolar")
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.side_panel(ui));
            });
        self.view(ui, env);
        self.repeat_window(ui);
        self.save_window(ui, env.library);
        std::mem::take(&mut self.actions)
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, library: Option<&Path>) {
        ui.horizontal_wrapped(|ui| {
            for (mode, label, tip, key) in [
                (Select::Points, "• Pontos", "Escolher pontos (1)", "modelo.pontos"),
                (Select::Faces, "◼ Faces", "Escolher faces (2)", "modelo.faces"),
                (Select::Parts, "📦 Peças", "Escolher peças inteiras (3)", "modelo.pecas"),
            ] {
                let r = ui
                    .selectable_label(self.mode == mode && !self.rig.bones_mode, label)
                    .on_hover_text(tip);
                crate::tutorial::anchor(ui.ctx(), key, r.rect);
                if r.clicked() {
                    self.set_mode(mode);
                }
            }
            if self.has_bones() {
                let r = ui
                    .selectable_label(self.rig.bones_mode, "💀 Ossos")
                    .on_hover_text("Escolher ossos e mover as juntas (4)");
                crate::tutorial::anchor(ui.ctx(), "modelo.ossos", r.rect);
                if r.clicked() {
                    self.rig.bones_mode = true;
                }
            }
            ui.separator();
            self.skeleton_menu(ui);
            let r = ui
                .add_enabled(self.has_bones(), egui::Button::new("🎬 Animar"))
                .on_hover_text("A linha do tempo e as animações: parado, andar, pular...")
                .on_disabled_hover_text("Para animar, ponha um esqueleto (💀 Esqueleto)");
            crate::tutorial::anchor(ui.ctx(), "modelo.animar", r.rect);
            if r.clicked() {
                self.set_animating(true);
            }
            ui.separator();
            for (tool, label, tip) in [
                (MTool::Move, "✋ Mover", "Mover (W). Arraste as setas, ou o que está escolhido."),
                (MTool::Rotate, "⟲ Girar", "Girar (E). Arraste um dos círculos."),
                (MTool::Scale, "⬈ Escalar", "Escalar (R). A ponta de um eixo, ou o quadrado do meio."),
                (MTool::Paint, "🎨 Pintar", "Pintar (B). Clique nas faces para pintar com a cor escolhida; com Shift, a peça toda."),
            ] {
                let r = ui
                    .selectable_label(self.tool == tool, label)
                    .on_hover_text(tip);
                if tool == MTool::Paint {
                    crate::tutorial::anchor(ui.ctx(), "modelo.pintar", r.rect);
                }
                if r.clicked() {
                    self.tool = tool;
                }
            }
            ui.separator();
            let faces = self.mode == Select::Faces && !self.sel_faces.is_empty();
            let r = ui
                .add_enabled(faces, egui::Button::new("⬆ Puxar"))
                .on_hover_text("Puxar as faces escolhidas (P): mexa o mouse e clique para terminar.")
                .on_disabled_hover_text("Escolha faces (2) para puxar.");
            crate::tutorial::anchor(ui.ctx(), "modelo.puxar", r.rect);
            if r.clicked() {
                let rect = self.view_rect.unwrap_or(ui.max_rect());
                self.start_extrude(rect.center());
            }
            let r = ui
                .add_enabled(faces, egui::Button::new("▣ Encolher"))
                .on_hover_text("Fazer uma face menor dentro de cada face escolhida (I).")
                .on_disabled_hover_text("Escolha faces (2) para encolher.");
            crate::tutorial::anchor(ui.ctx(), "modelo.encolher", r.rect);
            if r.clicked() {
                let rect = self.view_rect.unwrap_or(ui.max_rect());
                let c = self
                    .selection_center()
                    .and_then(|c| to_screen(&self.cam, rect, c))
                    .unwrap_or(rect.center());
                self.start_inset(rect, c + egui::vec2(80.0, 0.0));
            }
            let can_merge = self.mode != Select::Parts && self.selected_verts().len() >= 2;
            if ui
                .add_enabled(can_merge, egui::Button::new("🔗 Juntar"))
                .on_hover_text("Juntar os pontos escolhidos num só (M).")
                .on_disabled_hover_text("Escolha dois ou mais pontos (1).")
                .clicked()
            {
                self.merge_selection();
            }
            if ui
                .add_enabled(self.has_selection(), egui::Button::new("🗑 Apagar"))
                .on_hover_text("Apagar o que está escolhido (Delete).")
                .clicked()
            {
                self.delete_selection();
            }
            ui.separator();
            let parts = self.touched_parts();
            let mirror = !parts.is_empty() && parts.iter().all(|&i| self.data.parts[i].mirror);
            let r = ui
                .selectable_label(mirror, "↔ Espelho X")
                .on_hover_text("Modelar só uma metade: o lado esquerdo copia o direito. Desligar transforma a cópia em pontos e faces de verdade.");
            crate::tutorial::anchor(ui.ctx(), "modelo.espelho", r.rect);
            if r.clicked() {
                self.toggle_mirror();
            }
            ui.separator();
            ui.checkbox(&mut self.snap, "Encaixar").on_hover_text(
                "Encaixa na grade ao mover e puxar, de 15 em 15 graus ao girar. Segurar Ctrl troca.",
            );
            egui::ComboBox::from_id_salt(("modelo-grade", &self.rel))
                .width(56.0)
                .selected_text(crate::scene_data::format_num(f64::from(self.grid)))
                .show_ui(ui, |ui| {
                    for g in [0.05f32, 0.1, 0.25, 0.5, 1.0] {
                        ui.selectable_value(
                            &mut self.grid,
                            g,
                            crate::scene_data::format_num(f64::from(g)),
                        );
                    }
                })
                .response
                .on_hover_text("Tamanho da grade, em metros");
            ui.checkbox(&mut self.show_grid, "Grade");
            ui.separator();
            let mut pick = None;
            let r = ui.menu_button("➕ Peça", |ui| pick = add_menu_ui(ui, library));
            crate::tutorial::anchor(ui.ctx(), "modelo.peca", r.response.rect);
            r.response.on_hover_text(
                "Uma forma, uma peça de kit ou um modelo pronto. Com o mouse na vista: o botão direito põe em cima da face (ou no chão) onde você clicou, e o Shift+A também.",
            );
            if let Some(pick) = pick {
                self.add_pick(pick, Place::ground(Vec3::ZERO));
            }
            let r = ui.menu_button("📚 Biblioteca", |ui| {
                ui.set_min_width(200.0);
                let n = self.touched_parts().len();
                let r = ui
                    .button(if n == 0 {
                        "⭐ Guardar o modelo na biblioteca…".to_string()
                    } else if n == 1 {
                        "⭐ Guardar a peça na biblioteca…".to_string()
                    } else {
                        format!("⭐ Guardar as {n} peças na biblioteca…")
                    })
                    .on_hover_text("Para usar em outros modelos, em qualquer projeto (➕ Peça > 📚 Biblioteca)");
                crate::tutorial::anchor(ui.ctx(), "modelo.guardar", r.rect);
                if r.clicked() {
                    let name = match self.touched_parts()[..] {
                        [i] => self.data.parts[i].name.clone(),
                        _ => self
                            .rel
                            .rsplit('/')
                            .next()
                            .and_then(|f| f.strip_suffix(".modelo"))
                            .unwrap_or("peças")
                            .to_string(),
                    };
                    self.saving = Some(name);
                    ui.close();
                }
                let r = ui
                    .button("🔀 Criar variação")
                    .on_hover_text("Uma cópia deste modelo (com o .glb), aberta numa aba nova, para mudar as cores ou o jeito sem perder o original");
                crate::tutorial::anchor(ui.ctx(), "modelo.variacao", r.rect);
                if r.clicked() {
                    self.actions.push(ModelAction::Variation(self.data.clone()));
                    ui.close();
                }
            });
            crate::tutorial::anchor(ui.ctx(), "modelo.biblioteca", r.response.rect);
            let r = ui
                .add_enabled(
                    !self.touched_parts().is_empty() && self.repeat.is_none(),
                    egui::Button::new("🔁 Repetir"),
                )
                .on_hover_text("Cópias das peças escolhidas em linha (cerca, escada) ou em círculo (raios de roda, pétalas)")
                .on_disabled_hover_text("Escolha uma ou mais peças para repetir");
            crate::tutorial::anchor(ui.ctx(), "modelo.repetir", r.rect);
            if r.clicked() {
                self.start_repeat();
            }
            if ui.button("⛶ Enquadrar").on_hover_text("F").clicked() {
                self.frame();
            }
            ui.weak(format!("{} triângulos", self.data.triangle_count()))
                .on_hover_text("Quanto menos, mais leve no jogo. Até uns 2.000 por modelo é tranquilo.");
        });
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.strong("Peças");
        let mut toggle_vis = None;
        let mut pick = None;
        let mut rename_done = None;
        let touched = self.touched_parts();
        let r = egui::ScrollArea::vertical()
            .id_salt("modelo-pecas")
            .max_height(ui.available_height() * 0.45)
            .show(ui, |ui| {
                for (i, p) in self.data.parts.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let eye = if p.visible { "👁" } else { "–" };
                        if ui
                            .small_button(eye)
                            .on_hover_text("Mostrar ou esconder aqui (no jogo ela aparece)")
                            .clicked()
                        {
                            toggle_vis = Some(i);
                        }
                        if let Some((ri, text)) = &mut self.renaming
                            && *ri == i
                        {
                            let r = ui.text_edit_singleline(text);
                            r.request_focus();
                            if r.lost_focus() {
                                rename_done = Some((i, text.clone()));
                            }
                            return;
                        }
                        let selected = touched.contains(&i);
                        let mut label = p.name.clone();
                        if p.mirror {
                            label.push_str("  ↔");
                        }
                        let r = ui.selectable_label(selected, label);
                        if r.double_clicked() {
                            pick = Some((i, true));
                        } else if r.clicked() {
                            pick = Some((i, false));
                        }
                        r.context_menu(|ui| {
                            if ui.button("✏ Renomear").clicked() {
                                pick = Some((i, true));
                                ui.close();
                            }
                        });
                    });
                }
            });
        crate::tutorial::anchor(ui.ctx(), "modelo.lista", r.inner_rect);
        if let Some(i) = toggle_vis {
            self.record();
            self.data.parts[i].visible = !self.data.parts[i].visible;
        }
        if let Some((i, text)) = rename_done {
            let name = text.trim().to_string();
            if !name.is_empty() && name != self.data.parts[i].name {
                self.record();
                self.data.parts[i].name = name;
            }
            self.renaming = None;
        }
        if let Some((i, rename)) = pick {
            let add = ui.input(|i| i.modifiers.shift || i.modifiers.command);
            self.set_mode(Select::Parts);
            if add {
                toggle(&mut self.sel_parts, Some(i), true);
            } else {
                self.sel_parts = vec![i];
            }
            if rename {
                self.renaming = Some((i, self.data.parts[i].name.clone()));
            }
        }
        if ui
            .small_button("➕ Cubo")
            .on_hover_text("Uma peça nova; outras formas no ➕ Peça")
            .clicked()
        {
            self.add_part(0, Vec3::ZERO);
        }

        // A peça escolhida, em números.
        if self.mode == Select::Parts
            && let [i] = self.sel_parts[..]
            && i < self.data.parts.len()
        {
            ui.separator();
            let before = self.data.clone();
            let mut record = false;
            let p = &mut self.data.parts[i];
            egui::Grid::new(("modelo-peca-numeros", &self.rel))
                .num_columns(4)
                .spacing(egui::vec2(3.0, 3.0))
                .show(ui, |ui| {
                    for (label, v, speed, suffix) in [
                        ("Posição", &mut p.pos, 0.05, ""),
                        ("Rotação", &mut p.rot, 1.0, "°"),
                        ("Escala", &mut p.scale, 0.05, ""),
                    ] {
                        ui.label(label);
                        for k in 0..3 {
                            let r = ui.add(
                                egui::DragValue::new(&mut v[k])
                                    .speed(speed)
                                    .max_decimals(2)
                                    .suffix(suffix),
                            );
                            record |= r.drag_started() || (r.changed() && !r.dragged());
                        }
                        ui.end_row();
                    }
                });
            if record {
                self.undo.push(before);
                self.redo.clear();
            }
            self.part_bone_ui(ui, i);
        }
        self.bones_list(ui);

        ui.separator();
        ui.strong("Cores");
        ui.weak("Clique numa cor para pintar o que está escolhido.");
        let mut paint = None;
        let mut remove = None;
        let mut swap: Option<(String, String)> = None;
        let mut select = None;
        let palette = self.data.palette.clone();
        let r = ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
            for (i, c) in self.data.palette.iter().enumerate() {
                let color = scene_render::egui_color(c);
                let (rect, r) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), Sense::click());
                ui.painter().rect_filled(rect, 3.0, color);
                if *c == self.color {
                    ui.painter().rect_stroke(
                        rect.expand(1.0),
                        3.0,
                        Stroke::new(2.0, ui.visuals().strong_text_color()),
                        egui::StrokeKind::Outside,
                    );
                }
                r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, c));
                let r = r.on_hover_text(c);
                if r.clicked() {
                    paint = Some(c.clone());
                }
                r.context_menu(|ui| {
                    ui.menu_button("🎨 Trocar por…", |ui| {
                        ui.weak("No modelo todo");
                        ui.horizontal_wrapped(|ui| {
                            ui.set_max_width(150.0);
                            ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
                            for to in &palette {
                                let (rect, r) =
                                    ui.allocate_exact_size(egui::vec2(20.0, 20.0), Sense::click());
                                ui.painter()
                                    .rect_filled(rect, 3.0, scene_render::egui_color(to));
                                r.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::Button,
                                        true,
                                        format!("trocar por {to}"),
                                    )
                                });
                                if r.on_hover_text(to).clicked() {
                                    swap = Some((c.clone(), to.clone()));
                                    ui.close();
                                }
                            }
                        });
                    });
                    if ui.button("Escolher as faces desta cor").clicked() {
                        select = Some(c.clone());
                        ui.close();
                    }
                    if ui.button("Tirar da paleta").clicked() {
                        remove = Some(i);
                        ui.close();
                    }
                });
            }
        });
        crate::tutorial::anchor(ui.ctx(), "modelo.cores", r.response.rect);
        if let Some(c) = paint {
            self.color = c.clone();
            self.paint_selection(&c);
        }
        if let Some(i) = remove {
            self.record();
            self.data.palette.remove(i);
        }
        if let Some((from, to)) = swap {
            let n = self.replace_color(&from, &to);
            self.actions.push(ModelAction::Notice(if n == 1 {
                "1 face trocou de cor".into()
            } else {
                format!("{n} faces trocaram de cor")
            }));
        }
        if let Some(c) = select {
            self.select_color(&c);
        }
        ui.horizontal(|ui| {
            let mut c = scene_render::egui_color(&self.color);
            if ui.color_edit_button_srgba(&mut c).changed() {
                self.color = format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b());
            }
            ui.label(&self.color);
            if !self.data.palette.contains(&self.color)
                && ui
                    .small_button("➕")
                    .on_hover_text("Guardar a cor na paleta do modelo")
                    .clicked()
            {
                self.record();
                self.data.palette.push(self.color.clone());
            }
        });

        ui.separator();
        ui.weak(match (self.mode, self.tool) {
            (_, MTool::Paint) => "Clique numa face para pintar; Shift pinta a peça toda.",
            (Select::Faces, _) => "Clique numa face (Shift junta). P puxa, I encolhe.",
            (Select::Points, _) => "Clique num ponto (Shift junta). M junta os escolhidos.",
            (Select::Parts, _) => "Clique numa peça. Ctrl+D duplica.",
        });
    }

    fn view(&mut self, ui: &mut egui::Ui, env: &mut ModelEnv) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Vista do modelo")
        });
        self.view_rect = Some(rect);
        crate::tutorial::anchor(ui.ctx(), "modelo", rect);
        if !self.framed {
            self.framed = true;
            self.frame();
        }
        let painter = ui.painter_at(rect);

        self.tick(ui);
        if self.built.as_ref() != Some(&self.data.parts) {
            self.built = Some(self.data.parts.clone());
            self.rev = NEXT_REV.fetch_add(1, Ordering::Relaxed);
        }
        match env.renderer.as_deref_mut() {
            Some(renderer) => {
                let ppp = ui.ctx().pixels_per_point();
                let size = glam::UVec2::new(
                    (rect.width() * ppp).round().max(1.0) as u32,
                    (rect.height() * ppp).round().max(1.0) as u32,
                );
                let mats = self.shown_matrices();
                let shown: Vec<(usize, &Part)> = self
                    .data
                    .parts
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.visible && !p.faces.is_empty())
                    .collect();
                let ids = renderer.model_meshes(&self.rel, self.rev, || {
                    shown.iter().map(|(_, p)| p.mesh()).collect()
                });
                let extra: Vec<_> = ids
                    .into_iter()
                    .zip(shown.iter().map(|(i, _)| mats[*i]))
                    .collect();
                let view = View3 {
                    camera: {
                        let mut c = self.cam.camera();
                        c.near = 0.02;
                        c
                    },
                    size,
                    background: nn_core::Color::hex(0x2B3038),
                    grid: if self.show_grid {
                        self.grid.max(0.05)
                    } else {
                        0.0
                    },
                    time: 0.0,
                    bursts: Vec::new(),
                    fog: false,
                    hud: None,
                };
                let tex = renderer.render3d_with(&self.rel, env.root, &self.stage, &view, &extra);
                painter.image(
                    tex,
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            None => {
                painter.rect_filled(rect, 0.0, Color32::from_rgb(0x2B, 0x30, 0x38));
            }
        }

        self.input(ui, &response, rect);
        if self.rig.animating || self.rig.bones_mode {
            self.rig_overlays(&painter, rect);
        } else {
            self.overlays(&painter, rect);
        }
        self.view_menu(ui, &response, rect, env.library);
        self.hint(&painter, rect);
    }

    fn input(&mut self, ui: &mut egui::Ui, response: &egui::Response, rect: Rect) {
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
        let busy = crate::scene_view::busy(ui);

        // Puxando ou encolhendo: o mouse manda até o clique.
        if self.modal.is_some() {
            if let Some(p) = ui.input(|i| i.pointer.latest_pos()) {
                self.update_modal(rect, p, mods);
            }
            let (confirm, cancel) = ui.input_mut(|i| {
                (
                    i.pointer.primary_pressed() || i.consume_key(Modifiers::NONE, Key::Enter),
                    i.pointer.secondary_pressed() || i.consume_key(Modifiers::NONE, Key::Escape),
                )
            });
            if cancel {
                self.cancel_modal();
            } else if confirm {
                self.confirm_modal();
            }
            // O clique que confirmou não seleciona nada.
            self.drag = None;
            self.fly_moved = true;
            self.swallow = true;
            return;
        }
        if self.swallow {
            if !ui.input(|i| i.pointer.any_down()) {
                self.swallow = false;
            }
            return;
        }

        if ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary)) {
            self.fly_moved = false;
        }
        let looking = matches!(self.drag.as_ref().map(|d| &d.kind), Some(DragKind::Look));
        let flying = secondary && (hovered || looking);
        if response.drag_started_by(egui::PointerButton::Secondary) {
            self.drag = Some(MDrag {
                kind: DragKind::Look,
                start: response.interact_pointer_pos().unwrap_or_default(),
                before: ModelData::default(),
                center: Vec3::ZERO,
                recorded: false,
            });
        }
        if flying {
            if looking {
                self.cam.yaw = (self.cam.yaw - delta.x * 0.25).rem_euclid(360.0);
                self.cam.pitch = (self.cam.pitch - delta.y * 0.25).clamp(-89.0, 89.0);
                if delta != egui::Vec2::ZERO {
                    self.fly_moved = true;
                }
            }
            let mut mv = Vec3::ZERO;
            ui.input(|i| {
                let k = |key| f32::from(u8::from(i.key_down(key)));
                mv += self.cam.forward() * (k(Key::W) - k(Key::S));
                mv += self.cam.right() * (k(Key::D) - k(Key::A));
                mv += Vec3::Y * (k(Key::E) - k(Key::Q));
            });
            if mv != Vec3::ZERO {
                self.fly_moved = true;
                let fast = if mods.shift { 3.0 } else { 1.0 };
                self.cam.pos += mv.normalize() * self.cam.speed * fast * dt;
                ui.ctx().request_repaint();
            }
            if scroll != 0.0 && hovered {
                self.cam.speed = (self.cam.speed * (1.0 + scroll * 0.002)).clamp(0.2, 50.0);
            }
        } else if scroll != 0.0 && response.hovered() {
            let p = response.hover_pos().unwrap_or(rect.center());
            let (_, d) = ray(&self.cam, rect, p);
            let dist = (self.pivot() - self.cam.pos).length().max(0.5);
            self.cam.pos += d * scroll * 0.004 * dist;
        }

        if response.drag_started_by(egui::PointerButton::Middle) {
            let dist = (self.pivot() - self.cam.pos).length().max(0.5);
            self.drag = Some(MDrag {
                kind: DragKind::Pan { dist },
                start: response.interact_pointer_pos().unwrap_or_default(),
                before: ModelData::default(),
                center: Vec3::ZERO,
                recorded: false,
            });
        }
        // Ossos e Animar: escolher ossos e posar.
        if self.rig.animating || self.rig.bones_mode {
            self.rig_input(ui, response, rect, mods, flying);
            return;
        }

        // Pincel: pinta as faces por onde o mouse passa com o botão apertado.
        let painting = self.tool == MTool::Paint;
        // Com o Repetir aberto, a vista só anda (a prévia não muda por baixo).
        let locked = self.repeat.is_some();
        if painting && !locked {
            let down = ui.input(|i| i.pointer.primary_down());
            if down && !mods.alt && (response.is_pointer_button_down_on() || self.stroke) {
                if let Some(p) = ui.input(|i| i.pointer.latest_pos())
                    && rect.contains(p)
                {
                    let (o, d) = ray(&self.cam, rect, p);
                    if let Some(h) = self.pick_face(o, d) {
                        let faces: Vec<usize> = if mods.shift {
                            (0..self.data.parts[h.part].faces.len()).collect()
                        } else {
                            vec![h.face]
                        };
                        let part = &self.data.parts[h.part];
                        if faces.iter().any(|&f| part.faces[f].color != self.color) {
                            if !self.stroke {
                                self.record();
                            }
                            for f in faces {
                                self.data.parts[h.part].faces[f].color = self.color.clone();
                            }
                        }
                    }
                }
                self.stroke = true;
            } else {
                self.stroke = false;
            }
        }

        if !painting
            && (!locked || mods.alt)
            && response.drag_started_by(egui::PointerButton::Primary)
            && let Some(p) = ui
                .input(|i| i.pointer.press_origin())
                .or(response.interact_pointer_pos())
        {
            if mods.alt {
                let pivot = self.pivot();
                self.drag = Some(MDrag {
                    kind: DragKind::Orbit { pivot },
                    start: p,
                    before: ModelData::default(),
                    center: pivot,
                    recorded: false,
                });
            } else if let Some(kind) = self.gizmo_hit(rect, p) {
                self.start_drag(kind, p);
            } else {
                // Arrastar algo: escolhe e move (de frente para a câmera);
                // no vazio, a caixa de seleção.
                let (o, d) = ray(&self.cam, rect, p);
                let under = match self.mode {
                    Select::Points => self.pick_point(rect, p).map(|v| {
                        if !self.sel_verts.contains(&v) {
                            self.sel_verts = vec![v];
                        }
                    }),
                    Select::Faces => self.pick_face(o, d).map(|h| {
                        if !self.sel_faces.contains(&(h.part, h.face)) {
                            self.sel_faces = vec![(h.part, h.face)];
                        }
                    }),
                    Select::Parts => self.pick_face(o, d).map(|h| {
                        if !self.sel_parts.contains(&h.part) {
                            self.sel_parts = vec![h.part];
                        }
                    }),
                };
                if under.is_some() && self.tool == MTool::Move {
                    let c = self.selection_center().unwrap_or(Vec3::ZERO);
                    let normal = self.cam.forward();
                    if let Some(hit0) = ray_plane_n(o, d, c, normal) {
                        self.start_drag(DragKind::Free { normal, hit0 }, p);
                    }
                } else if under.is_none() {
                    self.start_drag(
                        DragKind::Box {
                            from: p,
                            add: mods.shift || mods.command,
                        },
                        p,
                    );
                }
            }
        }

        if let Some(mut drag) = self.drag.take() {
            let p = ui.input(|i| i.pointer.latest_pos()).unwrap_or(drag.start);
            let still = response.dragged() || ui.input(|i| i.pointer.any_down());
            self.continue_drag(&mut drag, p, rect, mods);
            if still {
                self.drag = Some(drag);
            } else {
                if let DragKind::Box { from, add } = drag.kind {
                    let area = Rect::from_two_pos(from, p);
                    if area.width() > 3.0 || area.height() > 3.0 {
                        self.box_select(rect, area, add);
                    }
                }
                if drag.recorded {
                    let before = drag.before;
                    self.record_before(before);
                }
            }
        }

        if response.clicked()
            && !painting
            && !locked
            && !mods.alt
            && let Some(p) = response.interact_pointer_pos()
        {
            self.click_select(rect, p, mods.shift || mods.command);
        }

        // Passando o mouse por cima: destaca.
        self.hover_face = None;
        self.hover_vert = None;
        if let Some(p) = response.hover_pos()
            && self.drag.is_none()
        {
            match self.mode {
                Select::Points if !painting => self.hover_vert = self.pick_point(rect, p),
                _ => {
                    let (o, d) = ray(&self.cam, rect, p);
                    self.hover_face = self.pick_face(o, d).map(|h| (h.part, h.face));
                }
            }
        }

        if response.hovered()
            && !secondary
            && !busy
            && !locked
            && let Some(p) = response.hover_pos()
            && ui.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::A))
        {
            self.add_menu = Some((p, self.place_at(rect, p)));
        }

        if !busy && !locked && (response.hovered() || response.has_focus() || response.dragged()) {
            self.keyboard(ui, rect, flying);
        } else if locked
            && response.hovered()
            && !flying
            && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F))
        {
            self.frame();
        }
    }

    fn keyboard(&mut self, ui: &mut egui::Ui, rect: Rect, flying: bool) {
        if ui.ctx().text_edit_focused() {
            return;
        }
        let pressed =
            |ui: &mut egui::Ui, m: Modifiers, k: Key| ui.input_mut(|i| i.consume_key(m, k));
        let mouse = ui.input(|i| i.pointer.hover_pos()).unwrap_or(rect.center());
        if !flying {
            for (key, mode) in [
                (Key::Num1, Select::Points),
                (Key::Num2, Select::Faces),
                (Key::Num3, Select::Parts),
            ] {
                if pressed(ui, Modifiers::NONE, key) {
                    self.set_mode(mode);
                }
            }
            if self.has_bones() && pressed(ui, Modifiers::NONE, Key::Num4) {
                self.rig.bones_mode = true;
            }
            for (key, tool) in [
                (Key::W, MTool::Move),
                (Key::E, MTool::Rotate),
                (Key::R, MTool::Scale),
                (Key::B, MTool::Paint),
            ] {
                if pressed(ui, Modifiers::NONE, key) {
                    self.tool = tool;
                }
            }
            if pressed(ui, Modifiers::NONE, Key::F) {
                self.frame();
            }
            if pressed(ui, Modifiers::NONE, Key::P) && !self.start_extrude(mouse) {
                self.actions.push(ModelAction::Notice(
                    "Para puxar, escolha faces (2) e aperte P.".into(),
                ));
            }
            if pressed(ui, Modifiers::NONE, Key::I) && !self.start_inset(rect, mouse) {
                self.actions.push(ModelAction::Notice(
                    "Para encolher, escolha faces (2) e aperte I.".into(),
                ));
            }
            if pressed(ui, Modifiers::NONE, Key::M) && !self.merge_selection() {
                self.actions.push(ModelAction::Notice(
                    "Para juntar, escolha dois ou mais pontos (1) e aperte M.".into(),
                ));
            }
        }
        if pressed(ui, Modifiers::NONE, Key::Delete) || pressed(ui, Modifiers::NONE, Key::X) {
            self.delete_selection();
        }
        if pressed(ui, Modifiers::COMMAND, Key::D) {
            self.duplicate_parts();
        }
        if pressed(ui, Modifiers::COMMAND, Key::A) {
            self.select_all();
        }
        if pressed(ui, Modifiers::NONE, Key::F2) {
            let parts = self.touched_parts();
            if let Some(&i) = parts.last() {
                self.renaming = Some((i, self.data.parts[i].name.clone()));
            }
        }
        if pressed(ui, Modifiers::NONE, Key::Escape) {
            self.clear_selection();
        }
    }

    /// O menu do botão direito (clicar e soltar, sem voar) e o do Shift+A.
    fn view_menu(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        rect: Rect,
        library: Option<&Path>,
    ) {
        let popup_id = egui::Popup::default_response_id(response);
        let was_open = egui::Popup::is_id_open(&response.ctx, popup_id);
        let open = if response.secondary_clicked()
            && !self.fly_moved
            && self.repeat.is_none()
            && !self.rig.animating
            && !self.rig.bones_mode
        {
            let p = response.interact_pointer_pos().unwrap_or(rect.center());
            // Clicar em algo fora da seleção escolhe ele.
            let (o, d) = ray(&self.cam, rect, p);
            match self.mode {
                Select::Points => {
                    if let Some(v) = self.pick_point(rect, p)
                        && !self.sel_verts.contains(&v)
                    {
                        self.sel_verts = vec![v];
                    }
                }
                Select::Faces => {
                    if let Some(h) = self.pick_face(o, d)
                        && !self.sel_faces.contains(&(h.part, h.face))
                    {
                        self.sel_faces = vec![(h.part, h.face)];
                    }
                }
                Select::Parts => {
                    if let Some(h) = self.pick_face(o, d)
                        && !self.sel_parts.contains(&h.part)
                    {
                        self.sel_parts = vec![h.part];
                    }
                }
            }
            Some(MenuAt {
                place: self.place_at(rect, p),
            })
        } else {
            None
        };
        let open = match open {
            Some(m) if was_open => {
                self.menu_reopen = Some(m);
                response.ctx.request_repaint();
                None
            }
            Some(m) => Some(m),
            None => self.menu_reopen.take(),
        };
        let command = if let Some(m) = open {
            self.menu = Some(m);
            Some(egui::containers::SetOpenCommand::Bool(true))
        } else if response.clicked() {
            Some(egui::containers::SetOpenCommand::Bool(false))
        } else {
            None
        };
        let mut add = None;
        let mut op = None;
        let has = self.has_selection();
        let (mode, verts) = (self.mode, self.selected_verts().len());
        let shown = egui::Popup::context_menu(response)
            .open_memory(command)
            .show(|ui| {
                ui.set_min_width(170.0);
                if has {
                    match mode {
                        Select::Faces => {
                            if crate::hierarchy::menu_item(ui, "⬆ Puxar", "P").clicked() {
                                op = Some(Key::P);
                            }
                            if crate::hierarchy::menu_item(ui, "▣ Encolher", "I").clicked() {
                                op = Some(Key::I);
                            }
                            if ui.button("🎨 Pintar com a cor escolhida").clicked() {
                                op = Some(Key::B);
                            }
                        }
                        Select::Points => {
                            if verts >= 2
                                && crate::hierarchy::menu_item(ui, "🔗 Juntar pontos", "M")
                                    .clicked()
                            {
                                op = Some(Key::M);
                            }
                        }
                        Select::Parts => {
                            if crate::hierarchy::menu_item(ui, "Duplicar", "Ctrl+D").clicked() {
                                op = Some(Key::D);
                            }
                            if crate::hierarchy::menu_item(ui, "✏ Renomear", "F2").clicked() {
                                op = Some(Key::F2);
                            }
                            if ui.button("🔁 Repetir…").clicked() {
                                op = Some(Key::Num0);
                            }
                            if ui.button("🎨 Pintar com a cor escolhida").clicked() {
                                op = Some(Key::B);
                            }
                        }
                    }
                    if ui.button("↔ Espelho X").clicked() {
                        op = Some(Key::X);
                    }
                    if crate::hierarchy::menu_item(ui, "🗑 Apagar", "Delete").clicked() {
                        op = Some(Key::Delete);
                    }
                    ui.separator();
                }
                let r = ui.menu_button("➕ Peça aqui", |ui| add = add_menu_ui(ui, library));
                crate::tutorial::anchor(ui.ctx(), "modelo.menu.peca", r.response.rect);
                if crate::hierarchy::menu_item(ui, "⛶ Enquadrar", "F").clicked() {
                    op = Some(Key::F);
                }
                if add.is_some() || op.is_some() {
                    ui.close();
                }
            });
        if shown.is_none() {
            self.menu = None;
        }
        // Shift+A: o mesmo menu de formas, onde o mouse estava.
        if let Some((p, at)) = self.add_menu {
            let mut close = false;
            let mut picked = None;
            egui::Area::new(egui::Id::new(("modelo-shift-a", &self.rel)))
                .order(egui::Order::Foreground)
                .fixed_pos(p)
                .show(ui.ctx(), |ui| {
                    egui::Frame::menu(ui.style()).show(ui, |ui| {
                        ui.set_min_width(140.0);
                        ui.weak("Peça nova");
                        if let Some(pick) = add_menu_ui(ui, library) {
                            picked = Some(pick);
                            close = true;
                        }
                    });
                });
            let outside = ui.input(|i| i.pointer.any_pressed())
                && !close
                && ui
                    .ctx()
                    .layer_id_at(ui.input(|i| i.pointer.latest_pos()).unwrap_or_default())
                    .is_none_or(|l| l.id != egui::Id::new(("modelo-shift-a", &self.rel)));
            // (Um submenu aberto fica numa camada própria: clicar nele não
            // é clicar fora.)
            let outside = outside && !egui::Popup::is_any_open(ui.ctx());
            if close || outside || ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                self.add_menu = None;
            }
            if let Some(pick) = picked {
                self.add_pick(pick, at);
            }
        }
        let at = self
            .menu
            .map(|m| m.place)
            .unwrap_or(Place::ground(Vec3::ZERO));
        if let Some(pick) = add {
            self.add_pick(pick, at);
        }
        let mouse = self
            .selection_center()
            .and_then(|c| to_screen(&self.cam, rect, c))
            .unwrap_or(rect.center());
        match op {
            Some(Key::P) => {
                self.start_extrude(mouse);
            }
            Some(Key::I) => {
                self.start_inset(rect, mouse + egui::vec2(80.0, 0.0));
            }
            Some(Key::B) => {
                let c = self.color.clone();
                self.paint_selection(&c);
            }
            Some(Key::M) => {
                self.merge_selection();
            }
            Some(Key::D) => self.duplicate_parts(),
            Some(Key::F2) => {
                if let Some(&i) = self.touched_parts().last() {
                    self.renaming = Some((i, self.data.parts[i].name.clone()));
                }
            }
            Some(Key::X) => self.toggle_mirror(),
            Some(Key::Num0) => {
                self.start_repeat();
            }
            Some(Key::Delete) => self.delete_selection(),
            Some(Key::F) => self.frame(),
            _ => {}
        }
    }

    fn overlays(&self, painter: &egui::Painter, rect: Rect) {
        let edge = Stroke::new(1.0, Color32::from_black_alpha(110));
        let sel_parts = self.touched_parts();
        for (pi, p) in self.data.parts.iter().enumerate() {
            if !p.visible {
                continue;
            }
            let m = p.matrix();
            let part_selected = self.mode == Select::Parts && sel_parts.contains(&pi);
            let part_hover = self.mode == Select::Parts
                && self.hover_face.is_some_and(|h| h.0 == pi)
                && !part_selected;
            for (pts, _, _, fi) in p.all_faces() {
                let w: Vec<Vec3> = pts.iter().map(|q| m.transform_point3(*q)).collect();
                let idx: Vec<u32> = (0..w.len() as u32).collect();
                let c = face_center(&w, &idx);
                let n = face_normal(&w, &idx);
                let front = n.dot(c - self.cam.pos) < 0.0;
                let s: Vec<Option<Pos2>> =
                    w.iter().map(|q| to_screen(&self.cam, rect, *q)).collect();
                if s.iter().any(Option::is_none) {
                    continue;
                }
                let s: Vec<Pos2> = s.into_iter().flatten().collect();
                let selected = self.mode == Select::Faces && self.sel_faces.contains(&(pi, fi));
                let hover = self.hover_face == Some((pi, fi));
                if selected {
                    painter.add(polygon(
                        &s,
                        ORANGE.gamma_multiply(if front { 0.35 } else { 0.15 }),
                    ));
                }
                if self.tool == MTool::Paint && hover {
                    painter.add(polygon(
                        &s,
                        scene_render::egui_color(&self.color).gamma_multiply(0.7),
                    ));
                }
                let stroke = if selected || part_selected {
                    Some(Stroke::new(if front { 2.0 } else { 1.0 }, ORANGE))
                } else if part_hover {
                    Some(Stroke::new(1.5, Color32::from_white_alpha(170)))
                } else if hover && self.mode == Select::Faces && self.tool != MTool::Paint {
                    Some(Stroke::new(2.0, Color32::from_white_alpha(200)))
                } else if front && self.mode != Select::Parts {
                    Some(edge)
                } else {
                    None
                };
                if let Some(stroke) = stroke {
                    let mut line = s.clone();
                    line.push(s[0]);
                    painter.add(egui::Shape::line(line, stroke));
                }
            }
        }

        // Os pontos.
        if self.mode == Select::Points {
            for (pi, k, mirrored, s, w) in self.screen_points(rect) {
                if !rect.contains(s) || !self.sees(w) {
                    continue;
                }
                let selected = self.sel_verts.contains(&(pi, k));
                let hover = self.hover_vert == Some((pi, k));
                let (r, color) = if selected {
                    (4.0, ORANGE)
                } else if hover {
                    (4.0, Color32::WHITE)
                } else {
                    (2.5, Color32::from_gray(25))
                };
                let color = if mirrored {
                    color.gamma_multiply(0.5)
                } else {
                    color
                };
                painter.circle(
                    s,
                    r,
                    color,
                    Stroke::new(1.0, Color32::from_white_alpha(140)),
                );
            }
        }

        // A caixa de seleção.
        if let Some(MDrag {
            kind: DragKind::Box { from, .. },
            ..
        }) = &self.drag
            && let Some(p) = painter.ctx().input(|i| i.pointer.latest_pos())
        {
            let area = Rect::from_two_pos(*from, p);
            painter.rect(
                area,
                0.0,
                Color32::from_rgba_unmultiplied(255, 165, 46, 30),
                Stroke::new(1.0, ORANGE),
                egui::StrokeKind::Inside,
            );
        }

        // As alças.
        if self.modal.is_some() || self.tool == MTool::Paint {
            return;
        }
        let Some(center) = self.selection_center() else {
            return;
        };
        self.draw_gizmo(painter, rect, center, self.tool);
    }

    /// Desenha as alças da ferramenta `tool` em `center`.
    fn draw_gizmo(&self, painter: &egui::Painter, rect: Rect, center: Vec3, tool: MTool) {
        let Some(c) = to_screen(&self.cam, rect, center) else {
            return;
        };
        let len = self.gizmo_len(center);
        let axes = [(Vec3::X, RED), (Vec3::Y, GREEN), (Vec3::Z, BLUE)];
        match tool {
            MTool::Move => {
                for (axis, color) in axes {
                    if let Some(tip) = to_screen(&self.cam, rect, center + axis * len) {
                        arrow(painter, c, tip, color);
                    }
                }
                painter.rect_filled(
                    Rect::from_center_size(c, egui::vec2(10.0, 10.0)),
                    2.0,
                    Color32::from_rgba_unmultiplied(255, 220, 80, 200),
                );
            }
            MTool::Rotate => {
                for (axis, color) in axes {
                    let pts: Vec<Pos2> = ring(center, axis, len)
                        .into_iter()
                        .filter_map(|w| to_screen(&self.cam, rect, w))
                        .collect();
                    painter.add(egui::Shape::line(pts, Stroke::new(2.0, color)));
                }
            }
            MTool::Scale => {
                let q = self.gizmo_rotation();
                for (axis, color) in axes {
                    if let Some(tip) = to_screen(&self.cam, rect, center + (q * axis) * len) {
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
            MTool::Paint => {}
        }
    }

    /// A dica embaixo da vista (o que fazer agora).
    fn hint(&self, painter: &egui::Painter, rect: Rect) {
        let text = match &self.modal {
            Some(Modal::Extrude { .. }) => {
                "Puxando: mexa o mouse e clique para terminar (Esc cancela, Ctrl solta da grade)"
            }
            Some(Modal::Inset { .. }) => {
                "Encolhendo: aproxime o mouse do meio e clique para terminar (Esc cancela)"
            }
            None if self.data.parts.is_empty() => {
                "Modelo vazio: botão direito na vista (ou Shift+A) para pôr uma peça"
            }
            None => return,
        };
        let pos = rect.center_bottom() - egui::vec2(0.0, 18.0);
        let galley = painter.layout_no_wrap(
            text.to_string(),
            egui::FontId::proportional(14.0),
            Color32::WHITE,
        );
        let bg = Rect::from_center_size(pos, galley.size() + egui::vec2(18.0, 10.0));
        painter.rect_filled(bg, 6.0, Color32::from_black_alpha(170));
        painter.galley(bg.min + egui::vec2(9.0, 5.0), galley, Color32::WHITE);
    }
}

/// Um polígono preenchido (em leque, para aceitar faces côncavas na tela).
fn polygon(s: &[Pos2], color: Color32) -> egui::Shape {
    let mut mesh = egui::Mesh::default();
    for p in s {
        mesh.colored_vertex(*p, color);
    }
    for k in 1..s.len().saturating_sub(1) as u32 {
        mesh.add_triangle(0, k, k + 1);
    }
    egui::Shape::mesh(mesh)
}

/// O menu do que dá para pôr no modelo: as formas, os kits e a biblioteca.
fn add_menu_ui(ui: &mut egui::Ui, library: Option<&Path>) -> Option<AddPick> {
    let mut out = shape_menu(ui).map(AddPick::Shape);
    ui.separator();
    for (k, kit) in crate::model_kits::KITS.iter().enumerate() {
        let r = ui.menu_button(format!("{} {}", kit.icon, kit.name), |ui| {
            for (i, item) in kit.items.iter().enumerate() {
                if ui.button(item.label).clicked() {
                    out = Some(AddPick::Kit(k, i));
                    ui.close();
                }
            }
        });
        crate::tutorial::anchor(
            ui.ctx(),
            &format!("modelo.kit:{}", kit.name),
            r.response.rect,
        );
    }
    ui.separator();
    let r = ui.menu_button("📚 Biblioteca", |ui| {
        ui.weak("Modelos prontos");
        for (i, m) in crate::model_kits::READY.iter().enumerate() {
            if ui.button(m.name).clicked() {
                out = Some(AddPick::Ready(i));
                ui.close();
            }
        }
        let mine = library
            .map(crate::model_kits::user_models)
            .unwrap_or_default();
        ui.separator();
        ui.weak("Minhas peças");
        if mine.is_empty() {
            ui.weak("(nada guardado ainda: 📚 > ⭐ Guardar)");
        }
        for (name, path) in mine {
            if ui.button(format!("⭐ {name}")).clicked() {
                out = Some(AddPick::User(path));
                ui.close();
            }
        }
    });
    crate::tutorial::anchor(ui.ctx(), "modelo.menu.biblioteca", r.response.rect);
    out
}

/// O menu das formas prontas: devolve qual foi escolhida.
fn shape_menu(ui: &mut egui::Ui) -> Option<usize> {
    let mut out = None;
    for (i, (label, words, _)) in SHAPES.iter().enumerate() {
        let icon = match i {
            0 => "⬛",
            1 => "◎",
            2 => "🔺",
            3 => "🏠",
            4 => "⚫",
            5 => "⬜",
            _ => "📐",
        };
        if ui
            .button(format!("{icon} {label}"))
            .on_hover_text(*words)
            .clicked()
        {
            out = Some(i);
            ui.close();
        }
    }
    out
}

fn toggle<T: PartialEq + Copy>(list: &mut Vec<T>, hit: Option<T>, add: bool) {
    match hit {
        Some(h) if add => {
            if let Some(i) = list.iter().position(|x| *x == h) {
                list.remove(i);
            } else {
                list.push(h);
            }
        }
        Some(h) => *list = vec![h],
        None if !add => list.clear(),
        None => {}
    }
}

/// "Cubo", "Cubo 2", "Cubo 3"...
fn unique_name(data: &ModelData, base: &str) -> String {
    let base = base
        .rsplit_once(' ')
        .filter(|(_, n)| n.parse::<u32>().is_ok())
        .map_or(base, |(b, _)| b);
    if !data.parts.iter().any(|p| p.name == base) {
        return base.to_string();
    }
    (2..)
        .map(|n| format!("{base} {n}"))
        .find(|n| !data.parts.iter().any(|p| p.name == *n))
        .expect("nome livre")
}

fn round3(v: Vec3) -> Vec3 {
    let r = |x: f32| {
        let y = (x * 10_000.0).round() / 10_000.0;
        if y == -0.0 { 0.0 } else { y }
    };
    Vec3::new(r(v.x), r(v.y), r(v.z))
}

/// Onde o raio corta o plano que passa por `c` com a normal `n`.
fn ray_plane_n(o: Vec3, d: Vec3, c: Vec3, n: Vec3) -> Option<Vec3> {
    let den = d.dot(n);
    if den.abs() < 1e-5 {
        return None;
    }
    let t = (c - o).dot(n) / den;
    (t > 0.0).then(|| o + d * t)
}

/// Distância até o triângulo, pelos dois lados (Möller–Trumbore).
fn ray_triangle(o: Vec3, d: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let e1 = b - a;
    let e2 = c - a;
    let p = d.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-9 {
        return None;
    }
    let inv = 1.0 / det;
    let s = o - a;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = d.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 1e-5).then_some(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> ModelDoc {
        ModelDoc::with("casa.modelo", ModelData::new_default(), None)
    }

    #[test]
    fn raio_acerta_a_face_de_cima() {
        let d = doc();
        let h = d.pick_face(Vec3::new(0.1, 5.0, 0.2), Vec3::NEG_Y).unwrap();
        let n = face_normal(&d.data.parts[0].verts, &d.data.parts[0].faces[h.face].v);
        assert!(n.y > 0.9);
        assert!((h.t - 4.0).abs() < 1e-4);
    }

    #[test]
    fn mover_pontos_com_espelho_fica_no_meio() {
        let mut d = doc();
        d.data.parts[0].mirror_on();
        d.set_mode(Select::Points);
        d.select_all();
        let before = d.data.clone();
        d.transform_selection(
            &before,
            Mat4::from_translation(Vec3::new(-2.0, 1.0, 0.0)),
            true,
        );
        for v in &d.data.parts[0].verts {
            assert!(v.x >= 0.0, "nada passa para o outro lado");
            assert!(v.y >= 1.0);
        }
    }

    #[test]
    fn mover_e_girar_pecas() {
        let mut d = doc();
        d.set_mode(Select::Parts);
        d.sel_parts = vec![0];
        let before = d.data.clone();
        let t = Mat4::from_quat(Quat::from_rotation_y(90f32.to_radians()));
        d.transform_selection(&before, Mat4::from_translation(Vec3::X) * t, true);
        let p = &d.data.parts[0];
        assert!((p.rot.y - 90.0).abs() < 1e-3, "{}", p.rot);
        assert!((p.pos - Vec3::X).length() < 1e-4);
        assert_eq!(p.scale, Vec3::ONE);
    }

    #[test]
    fn puxar_segue_e_desfaz() {
        let mut d = doc();
        d.set_mode(Select::Faces);
        let top = d.data.parts[0]
            .faces
            .iter()
            .position(|f| face_normal(&d.data.parts[0].verts, &f.v).y > 0.9)
            .unwrap();
        d.sel_faces = vec![(0, top)];
        assert!(d.start_extrude(Pos2::ZERO));
        assert_eq!(d.data.parts[0].faces.len(), 10);
        d.cancel_modal();
        assert_eq!(d.data.parts[0].faces.len(), 6, "Esc volta como estava");
        assert!(d.start_extrude(Pos2::ZERO));
        d.confirm_modal();
        assert_eq!(d.data.parts[0].faces.len(), 10);
        d.undo();
        assert_eq!(d.data.parts[0].faces.len(), 6);
    }

    #[test]
    fn nomes_sem_repetir() {
        let mut d = doc();
        assert_eq!(unique_name(&d.data, "Cubo"), "Cubo 2");
        d.add_part(0, Vec3::ZERO);
        assert_eq!(d.data.parts[1].name, "Cubo 2");
        assert_eq!(unique_name(&d.data, "Cubo 2"), "Cubo 3");
        assert_eq!(unique_name(&d.data, "Telhado"), "Telhado");
    }

    #[test]
    fn pintar_e_apagar() {
        let mut d = doc();
        d.set_mode(Select::Faces);
        d.sel_faces = vec![(0, 0), (0, 1)];
        d.paint_selection("#3FA7D6");
        assert_eq!(d.data.parts[0].faces[1].color, "#3FA7D6");
        d.delete_selection();
        assert_eq!(d.data.parts[0].faces.len(), 4);
        d.undo();
        assert_eq!(d.data.parts[0].faces.len(), 6);
        d.undo();
        assert_eq!(d.data.parts[0].faces[1].color, "#FF8C42");
    }

    #[test]
    fn trocar_o_modo_leva_a_selecao() {
        let mut d = doc();
        d.set_mode(Select::Faces);
        d.sel_faces = vec![(0, 1)];
        d.set_mode(Select::Points);
        assert_eq!(d.sel_verts.len(), 4);
        d.set_mode(Select::Faces);
        assert_eq!(d.sel_faces, vec![(0, 1)]);
        d.set_mode(Select::Parts);
        assert_eq!(d.sel_parts, vec![0]);
    }
}
