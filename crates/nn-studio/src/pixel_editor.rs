//! Aba de imagem: editor de pixel art (lápis, borracha, balde, linha,
//! retângulo, círculo, conta-gotas e seleção), quadros da folha, papel
//! vegetal, paletas e o editor de animações (arquivo `.anim` ao lado da
//! imagem), com prévia.

use std::path::Path;
use std::sync::Mutex;
use std::time::{Instant, SystemTime};

use eframe::egui::{
    self, Color32, Key, PointerButton, Pos2, Rect, Sense, Stroke, TextureHandle, Vec2,
};
use image::RgbaImage;

use crate::anim_file::{Anim, AnimFile, Frame, Mode, anim_path};
use crate::pixel_ops::{self as ops, CLEAR, Color, PixRect};

/// Ferramenta de desenho.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Pencil,
    Eraser,
    Fill,
    Line,
    Rect,
    Ellipse,
    Picker,
    Select,
}

impl Tool {
    pub const ALL: [Tool; 8] = [
        Tool::Pencil,
        Tool::Eraser,
        Tool::Fill,
        Tool::Line,
        Tool::Rect,
        Tool::Ellipse,
        Tool::Picker,
        Tool::Select,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Tool::Pencil => "Lápis",
            Tool::Eraser => "Borracha",
            Tool::Fill => "Balde",
            Tool::Line => "Linha",
            Tool::Rect => "Retângulo",
            Tool::Ellipse => "Círculo",
            Tool::Picker => "Conta-gotas",
            Tool::Select => "Seleção",
        }
    }

    pub fn key(self) -> Key {
        match self {
            Tool::Pencil => Key::B,
            Tool::Eraser => Key::E,
            Tool::Fill => Key::G,
            Tool::Line => Key::L,
            Tool::Rect => Key::R,
            Tool::Ellipse => Key::O,
            Tool::Picker => Key::I,
            Tool::Select => Key::M,
        }
    }

    fn help(self) -> &'static str {
        match self {
            Tool::Pencil => "Desenha pixel a pixel. Botão esquerdo: cor principal; direito: secundária. Shift+clique: linha desde o último ponto.",
            Tool::Eraser => "Apaga (deixa transparente).",
            Tool::Fill => "Pinta a área da mesma cor (sem sair do quadro).",
            Tool::Line => "Arraste para uma linha. Shift: retas e diagonais.",
            Tool::Rect => "Arraste para um retângulo. Shift: quadrado.",
            Tool::Ellipse => "Arraste para um círculo ou elipse. Shift: círculo.",
            Tool::Picker => "Pega a cor de um pixel. (Com qualquer ferramenta: Alt+clique.)",
            Tool::Select => "Arraste para selecionar; arraste a seleção para mover. Ctrl+C, Ctrl+X, Ctrl+V, Delete.",
        }
    }

    fn uses_brush(self) -> bool {
        matches!(self, Tool::Pencil | Tool::Eraser | Tool::Line)
    }

    fn uses_fill(self) -> bool {
        matches!(self, Tool::Rect | Tool::Ellipse)
    }
}

/// O que o editor pede ao projeto.
#[derive(Debug, Clone, PartialEq)]
pub enum PixelAction {
    Save,
    CopyText(String),
    /// A paleta do projeto mudou (gravar em `paleta.hex`).
    SavePalette,
}

/// Arquivo da paleta do projeto (cores em hexadecimal, uma por linha).
pub const PROJECT_PALETTE: &str = "paleta.hex";

/// Área de transferência de imagens (compartilhada entre as abas).
static CLIPBOARD: Mutex<Option<RgbaImage>> = Mutex::new(None);

#[derive(Debug, Clone)]
struct Snap {
    img: RgbaImage,
    anim: AnimFile,
    rev: u64,
}

#[derive(Debug, Clone)]
struct StrokeState {
    tool: Tool,
    button: PointerButton,
    color: Color,
    start: (i32, i32),
    last: (i32, i32),
    before: RgbaImage,
    clip: PixRect,
}

/// Pedaço solto (colado ou selecionado e arrastado), ainda não grudado.
#[derive(Debug, Clone)]
struct Floating {
    img: RgbaImage,
    x: i32,
    y: i32,
}

/// Arrastar um quadro da fita para a linha do tempo.
#[derive(Debug, Clone, Copy)]
struct FrameDrag(u32);

/// Arrastar dentro da linha do tempo (posição na animação).
#[derive(Debug, Clone, Copy)]
struct TimelineDrag(usize);

fn next_rev() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    N.fetch_add(1, Ordering::Relaxed)
}

pub struct PixelDoc {
    pub rel: String,
    pub img: RgbaImage,
    pub anim: AnimFile,
    pub error: Option<String>,
    /// Problema ao ler o `.anim` (a imagem abre assim mesmo).
    pub anim_error: Option<String>,
    anim_on_disk: bool,
    rev: u64,
    saved_rev: u64,
    history: Vec<Snap>,
    future: Vec<Snap>,
    last_edit: Option<(String, Instant)>,
    tex: Option<TextureHandle>,
    tex_rev: u64,
    float_tex: Option<TextureHandle>,
    float_rev: u64,
    pub tool: Tool,
    pub brush: u32,
    pub filled: bool,
    pub mirror: bool,
    pub primary: Color,
    pub secondary: Color,
    /// 0 = cores desta imagem, 1 = paleta do projeto, 2.. = prontas.
    pub palette: usize,
    zoom: Option<f32>,
    pan: Vec2,
    /// Mostrar só o quadro atual (em vez da folha toda).
    pub only_frame: bool,
    /// Quadro atual (a partir de 0).
    pub current: u32,
    pub onion: bool,
    pub pixel_grid: bool,
    stroke: Option<StrokeState>,
    last_point: Option<(i32, i32)>,
    selection: Option<PixRect>,
    sel_start: Option<(i32, i32)>,
    floating: Option<Floating>,
    float_grab: Option<(i32, i32)>,
    /// Animação escolhida no painel.
    pub anim_sel: Option<usize>,
    pub playing: bool,
    play_time: f64,
    play_last: Option<Instant>,
    resize: Option<[u32; 2]>,
    disk_mtime: Option<SystemTime>,
    hover_px: Option<(i32, i32)>,
}

impl std::fmt::Debug for PixelDoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PixelDoc")
            .field("rel", &self.rel)
            .field("size", &self.img.dimensions())
            .field("dirty", &self.is_dirty())
            .finish_non_exhaustive()
    }
}

fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

impl PixelDoc {
    /// Uma imagem nova (ainda não salva).
    pub fn new_image(rel: &str, img: RgbaImage, anim: AnimFile) -> Self {
        let rev = next_rev();
        Self {
            rel: rel.to_string(),
            img,
            anim,
            error: None,
            anim_error: None,
            anim_on_disk: false,
            rev,
            saved_rev: rev,
            history: Vec::new(),
            future: Vec::new(),
            last_edit: None,
            tex: None,
            tex_rev: 0,
            float_tex: None,
            float_rev: 0,
            tool: Tool::Pencil,
            brush: 1,
            filled: false,
            mirror: false,
            primary: [0x1a, 0x1c, 0x2c, 255],
            secondary: CLEAR,
            palette: 2,
            zoom: None,
            pan: Vec2::ZERO,
            only_frame: false,
            current: 0,
            onion: false,
            pixel_grid: true,
            stroke: None,
            last_point: None,
            selection: None,
            sel_start: None,
            floating: None,
            float_grab: None,
            anim_sel: None,
            playing: true,
            play_time: 0.0,
            play_last: None,
            resize: None,
            disk_mtime: None,
            hover_px: None,
        }
    }

    pub fn open(root: &Path, rel: &str) -> Self {
        let mut doc = Self::new_image(rel, RgbaImage::new(1, 1), AnimFile::default());
        doc.load(root);
        doc
    }

    fn load(&mut self, root: &Path) {
        let path = root.join(&self.rel);
        self.disk_mtime = mtime(&path);
        match crate::viewers::load_rgba(&path) {
            Ok(img) => {
                self.img = img;
                self.error = None;
            }
            Err(e) => {
                self.error = Some(e);
                return;
            }
        }
        let anim_rel = anim_path(&self.rel);
        self.anim_on_disk = root.join(&anim_rel).is_file();
        self.anim_error = None;
        self.anim = if self.anim_on_disk {
            match AnimFile::load(root, &self.rel) {
                Ok(a) => a,
                Err(e) => {
                    self.anim_error = Some(e);
                    AnimFile::default()
                }
            }
        } else {
            AnimFile::default()
        };
        if self.anim.frame_w == 0 {
            // Tira de quadros quadrados: já sugere os quadros (só ao mostrar).
            self.only_frame = false;
        }
        self.current = self.current.min(self.frame_count() - 1);
        if self.anim_sel.is_none_or(|i| i >= self.anim.anims.len()) {
            self.anim_sel = (!self.anim.anims.is_empty()).then_some(0);
        }
        self.rev = next_rev();
        self.saved_rev = self.rev;
        self.history.clear();
        self.future.clear();
        self.floating = None;
        self.selection = None;
        self.stroke = None;
    }

    pub fn is_dirty(&self) -> bool {
        self.rev != self.saved_rev
    }

    /// O arquivo mudou fora: relê, se não houver nada a perder.
    pub fn disk_changed(&mut self, root: &Path) {
        if self.is_dirty() || self.stroke.is_some() {
            return;
        }
        let path = root.join(&self.rel);
        let anim_m = mtime(&root.join(anim_path(&self.rel)));
        if mtime(&path) != self.disk_mtime || anim_m.is_some() != self.anim_on_disk || anim_m.is_some() {
            self.load(root);
        }
    }

    /// Grava a imagem (PNG) e, se houver quadros ou animações, o `.anim`.
    pub fn save(&mut self, root: &Path) -> Result<(), String> {
        self.commit_floating();
        let path = root.join(&self.rel);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        self.img
            .save_with_format(&path, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        let anim_rel = anim_path(&self.rel);
        let wants_anim = !self.anim.anims.is_empty()
            || (self.anim.frame_w > 0 && self.frame_count() > 1)
            || self.anim_on_disk;
        if wants_anim {
            std::fs::write(root.join(&anim_rel), self.anim.write(&self.rel))
                .map_err(|e| format!("{anim_rel}: {e}"))?;
            self.anim_on_disk = true;
        }
        self.disk_mtime = mtime(&path);
        self.saved_rev = self.rev;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Histórico

    /// Guarda o estado antes de uma mudança. Mudanças seguidas com a mesma
    /// `key` (arrastar um número, digitar um nome) viram um passo só.
    fn checkpoint(&mut self, key: Option<&str>) {
        if let (Some(k), Some((last, at))) = (key, &self.last_edit)
            && k == last
            && at.elapsed().as_secs_f32() < 1.5
        {
            self.last_edit = Some((k.to_string(), Instant::now()));
            return;
        }
        self.last_edit = key.map(|k| (k.to_string(), Instant::now()));
        self.history.push(Snap {
            img: self.img.clone(),
            anim: self.anim.clone(),
            rev: self.rev,
        });
        if self.history.len() > 80 {
            self.history.remove(0);
        }
        self.future.clear();
    }

    fn touch(&mut self) {
        self.rev = next_rev();
    }

    fn restore(&mut self, s: Snap) {
        self.img = s.img;
        self.anim = s.anim;
        self.rev = s.rev;
        self.current = self.current.min(self.frame_count() - 1);
        if self.anim_sel.is_some_and(|i| i >= self.anim.anims.len()) {
            self.anim_sel = self.anim.anims.len().checked_sub(1);
        }
        self.selection = None;
        self.floating = None;
        self.last_edit = None;
    }

    pub fn undo(&mut self) {
        if self.floating.is_some() {
            self.commit_floating();
        }
        if let Some(s) = self.history.pop() {
            let now = Snap {
                img: self.img.clone(),
                anim: self.anim.clone(),
                rev: self.rev,
            };
            self.future.push(now);
            self.restore(s);
        }
    }

    pub fn redo(&mut self) {
        if let Some(s) = self.future.pop() {
            let now = Snap {
                img: self.img.clone(),
                anim: self.anim.clone(),
                rev: self.rev,
            };
            self.history.push(now);
            self.restore(s);
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    // ------------------------------------------------------------------
    // Quadros

    pub fn has_frames(&self) -> bool {
        self.anim.frame_w > 0
            && self.anim.frame_h > 0
            && self.anim.frame_w <= self.img.width()
            && self.anim.frame_h <= self.img.height()
    }

    pub fn frame_count(&self) -> u32 {
        if self.has_frames() {
            ops::frame_count(&self.img, self.anim.frame_w, self.anim.frame_h).max(1)
        } else {
            1
        }
    }

    pub fn cell(&self, i: u32) -> PixRect {
        if self.has_frames() {
            ops::cell(&self.img, self.anim.frame_w, self.anim.frame_h, i)
        } else {
            PixRect::of(&self.img)
        }
    }

    /// Quadro que contém o pixel.
    fn frame_at(&self, x: i32, y: i32) -> Option<u32> {
        (0..self.frame_count()).find(|i| self.cell(*i).contains(x, y))
    }

    pub fn set_frame_size(&mut self, w: u32, h: u32) {
        self.checkpoint(Some("tamanho-do-quadro"));
        self.anim.frame_w = w;
        self.anim.frame_h = h;
        self.current = self.current.min(self.frame_count() - 1);
        self.touch();
    }

    pub fn add_frame(&mut self, copy: bool) {
        self.commit_floating();
        self.checkpoint(None);
        let at = if self.has_frames() { self.current + 1 } else { 1 };
        ops::insert_frame(&mut self.img, &mut self.anim, at, copy.then_some(self.current));
        self.current = at;
        self.touch();
    }

    pub fn delete_frame(&mut self) {
        if self.frame_count() <= 1 {
            return;
        }
        self.commit_floating();
        self.checkpoint(None);
        ops::remove_frame(&mut self.img, &mut self.anim, self.current);
        self.current = self.current.min(self.frame_count() - 1);
        self.touch();
    }

    pub fn move_frame(&mut self, right: bool) {
        let n = self.frame_count();
        let to = if right {
            self.current + 1
        } else {
            self.current.wrapping_sub(1)
        };
        if to >= n {
            return;
        }
        self.commit_floating();
        self.checkpoint(None);
        ops::swap_frames(&mut self.img, &mut self.anim, self.current, to);
        self.current = to;
        self.touch();
    }

    // ------------------------------------------------------------------
    // Seleção e área de transferência

    fn commit_floating(&mut self) {
        if let Some(f) = self.floating.take() {
            let all = PixRect::of(&self.img);
            ops::paste(&mut self.img, &f.img, f.x, f.y, all, false);
            self.selection = PixRect::between(
                (f.x, f.y),
                (f.x + f.img.width() as i32 - 1, f.y + f.img.height() as i32 - 1),
                all,
            );
            self.touch();
        }
    }

    /// Solta a seleção do desenho (para mover ou transformar).
    fn lift_selection(&mut self) -> bool {
        let Some(r) = self.selection else {
            return false;
        };
        self.checkpoint(None);
        let piece = ops::crop(&self.img, r);
        ops::clear_rect(&mut self.img, r);
        self.floating = Some(Floating {
            img: piece,
            x: r.x as i32,
            y: r.y as i32,
        });
        self.touch();
        true
    }

    fn copy(&mut self, ctx: &egui::Context) {
        let piece = match (&self.floating, self.selection) {
            (Some(f), _) => Some(f.img.clone()),
            (None, Some(r)) => Some(ops::crop(&self.img, r)),
            _ => Some(ops::crop(&self.img, self.cell(self.current))),
        };
        *CLIPBOARD.lock().unwrap() = piece;
        // Um texto na área do sistema faz o Ctrl+V chegar até aqui.
        ctx.copy_text("[imagem copiada no NN Studio]".into());
    }

    fn cut(&mut self, ctx: &egui::Context) {
        self.copy(ctx);
        self.delete_selection();
    }

    fn delete_selection(&mut self) {
        if self.floating.take().is_some() {
            self.selection = None;
            self.touch();
            return;
        }
        if let Some(r) = self.selection {
            self.checkpoint(None);
            ops::clear_rect(&mut self.img, r);
            self.touch();
        }
    }

    pub fn paste(&mut self) {
        let Some(piece) = CLIPBOARD.lock().unwrap().clone() else {
            return;
        };
        self.commit_floating();
        self.checkpoint(None);
        let cell = self.cell(self.current);
        self.floating = Some(Floating {
            img: piece,
            x: cell.x as i32,
            y: cell.y as i32,
        });
        self.tool = Tool::Select;
        self.touch();
    }

    /// Espelha ou gira o pedaço solto, a seleção ou o quadro atual.
    pub fn transform(&mut self, how: Transform) {
        if self.floating.is_none() && self.selection.is_some() {
            self.lift_selection();
        } else if self.floating.is_some() {
            self.checkpoint(None);
        }
        if let Some(f) = &mut self.floating {
            f.img = how.apply(&f.img);
            self.touch();
            return;
        }
        let r = self.cell(self.current);
        if how == Transform::Rotate && r.w != r.h {
            return;
        }
        self.checkpoint(None);
        let piece = how.apply(&ops::crop(&self.img, r));
        image::imageops::replace(&mut self.img, &piece, r.x as i64, r.y as i64);
        self.touch();
    }

    pub fn resize_canvas(&mut self, w: u32, h: u32) {
        self.commit_floating();
        self.checkpoint(None);
        self.img = ops::resize_canvas(&self.img, w, h);
        self.current = self.current.min(self.frame_count() - 1);
        self.touch();
    }

    // ------------------------------------------------------------------
    // Animações

    pub fn new_anim(&mut self) -> usize {
        self.checkpoint(None);
        let name = self.anim.free_name(match self.anim.anims.len() {
            0 => "parado",
            1 => "andar",
            2 => "pular",
            _ => "animacao",
        });
        let frames: Vec<u32> = if self.anim.anims.is_empty() {
            (1..=self.frame_count()).collect()
        } else {
            vec![self.current + 1]
        };
        self.anim.anims.push(Anim::new(&name, frames));
        let i = self.anim.anims.len() - 1;
        self.anim_sel = Some(i);
        self.play_time = 0.0;
        self.touch();
        i
    }

    fn anim_edit(&mut self, i: usize, key: &str, f: impl FnOnce(&mut Anim)) {
        self.checkpoint(Some(&format!("{key}:{i}")));
        if let Some(a) = self.anim.anims.get_mut(i) {
            f(a);
        }
        self.touch();
    }

    // ------------------------------------------------------------------
    // Desenho

    fn clip_for(&self, x: i32, y: i32) -> PixRect {
        self.frame_at(x, y)
            .map(|i| self.cell(i))
            .unwrap_or_else(|| PixRect::of(&self.img))
    }

    fn begin_stroke(&mut self, tool: Tool, button: PointerButton, p: (i32, i32), shift: bool) {
        let color = match (tool, button) {
            (Tool::Eraser, _) => CLEAR,
            (_, PointerButton::Secondary) => self.secondary,
            _ => self.primary,
        };
        if let Some(f) = self.frame_at(p.0, p.1) {
            self.current = f;
        }
        let clip = self.clip_for(p.0, p.1);
        match tool {
            Tool::Picker => {
                if let Some(c) = ops::get(&self.img, p.0, p.1) {
                    if button == PointerButton::Secondary {
                        self.secondary = c;
                    } else {
                        self.primary = c;
                    }
                }
                return;
            }
            Tool::Fill => {
                if ops::get(&self.img, p.0, p.1).is_some_and(|c| c != color) {
                    self.checkpoint(None);
                    ops::flood_fill(&mut self.img, p.0, p.1, color, clip);
                    self.touch();
                }
                return;
            }
            Tool::Select => return,
            _ => {}
        }
        self.checkpoint(None);
        let before = self.img.clone();
        let start = match (tool, shift, self.last_point) {
            (Tool::Pencil | Tool::Eraser, true, Some(lp)) => lp,
            _ => p,
        };
        self.stroke = Some(StrokeState {
            tool,
            button,
            color,
            start,
            last: start,
            before,
            clip,
        });
        self.continue_stroke(p, shift);
    }

    fn continue_stroke(&mut self, p: (i32, i32), shift: bool) {
        let Some(s) = self.stroke.as_mut() else {
            return;
        };
        let (brush, filled, mirror) = (self.brush, self.filled, self.mirror);
        match s.tool {
            Tool::Pencil | Tool::Eraser => {
                for (x, y) in ops::line(s.last, p) {
                    ops::stamp(&mut self.img, x, y, brush, s.color, s.clip, mirror);
                }
                s.last = p;
            }
            Tool::Line | Tool::Rect | Tool::Ellipse => {
                let mut end = p;
                if shift {
                    let (dx, dy) = (p.0 - s.start.0, p.1 - s.start.1);
                    end = match s.tool {
                        Tool::Line => {
                            // Retas e diagonais.
                            if dx.abs() > 2 * dy.abs() {
                                (p.0, s.start.1)
                            } else if dy.abs() > 2 * dx.abs() {
                                (s.start.0, p.1)
                            } else {
                                let d = dx.abs().max(dy.abs());
                                (s.start.0 + d * dx.signum(), s.start.1 + d * dy.signum())
                            }
                        }
                        _ => {
                            let d = dx.abs().max(dy.abs());
                            (s.start.0 + d * dx.signum(), s.start.1 + d * dy.signum())
                        }
                    };
                }
                self.img = s.before.clone();
                let points = match s.tool {
                    Tool::Line => ops::line(s.start, end),
                    Tool::Rect => ops::rect(s.start, end, filled),
                    _ => ops::ellipse(s.start, end, filled),
                };
                let size = if s.tool == Tool::Line { brush } else { 1 };
                for (x, y) in points {
                    ops::stamp(&mut self.img, x, y, size, s.color, s.clip, mirror);
                }
                s.last = end;
            }
            _ => {}
        }
        self.touch();
    }

    fn end_stroke(&mut self) {
        if let Some(s) = self.stroke.take() {
            self.last_point = Some(s.last);
            if s.before == self.img {
                // Nada mudou: o passo do histórico não precisa ficar.
                if let Some(snap) = self.history.pop() {
                    self.rev = snap.rev;
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Interface

    fn sync_textures(&mut self, ctx: &egui::Context) {
        if self.tex.is_none() || self.tex_rev != self.rev {
            let image = crate::viewers::to_color_image(&self.img);
            match &mut self.tex {
                Some(t) => t.set(image, egui::TextureOptions::NEAREST),
                None => {
                    self.tex = Some(ctx.load_texture(
                        format!("pixel:{}", self.rel),
                        image,
                        egui::TextureOptions::NEAREST,
                    ))
                }
            }
            self.tex_rev = self.rev;
        }
        match &self.floating {
            Some(f) if self.float_tex.is_none() || self.float_rev != self.rev => {
                let image = crate::viewers::to_color_image(&f.img);
                match &mut self.float_tex {
                    Some(t) => t.set(image, egui::TextureOptions::NEAREST),
                    None => {
                        self.float_tex = Some(ctx.load_texture(
                            format!("pixel-solto:{}", self.rel),
                            image,
                            egui::TextureOptions::NEAREST,
                        ))
                    }
                }
                self.float_rev = self.rev;
            }
            None => self.float_tex = None,
            _ => {}
        }
    }

    fn uv(&self, r: PixRect) -> Rect {
        let (w, h) = (self.img.width() as f32, self.img.height() as f32);
        Rect::from_min_max(
            Pos2::new(r.x as f32 / w, r.y as f32 / h),
            Pos2::new((r.x + r.w) as f32 / w, (r.y + r.h) as f32 / h),
        )
    }

    /// Desenha o quadro `i` no retângulo (para miniaturas e prévia).
    fn paint_frame(&self, painter: &egui::Painter, rect: Rect, i: u32) {
        let Some(tex) = &self.tex else {
            return;
        };
        let cell = self.cell(i);
        // Mantém a proporção do quadro dentro do retângulo.
        let k = (rect.width() / cell.w as f32).min(rect.height() / cell.h as f32);
        let k = if k >= 1.0 { k.floor() } else { k };
        let r = Rect::from_center_size(rect.center(), Vec2::new(cell.w as f32, cell.h as f32) * k);
        checkerboard(painter, r, (k * 2.0).max(4.0));
        painter.image(tex.id(), r, self.uv(cell), Color32::WHITE);
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, project_palette: &mut Vec<Color>) -> Vec<PixelAction> {
        let mut actions = Vec::new();
        if let Some(e) = &self.error {
            ui.colored_label(crate::theme::ERROR, e);
            return actions;
        }
        let hot = ui.rect_contains_pointer(ui.max_rect());
        self.sync_textures(ui.ctx());
        self.toolbar(ui, &mut actions);
        ui.separator();

        egui::Panel::right(egui::Id::new(("pixel-cores", &self.rel)))
            .resizable(true)
            .default_size(220.0)
            .min_size(180.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.side_panel(ui, project_palette, &mut actions));
            });
        egui::Panel::bottom(egui::Id::new(("pixel-quadros", &self.rel)))
            .resizable(true)
            .default_size(210.0)
            .min_size(120.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.frames_panel(ui);
                        ui.add_space(4.0);
                        self.anims_panel(ui);
                    });
            });
        self.canvas(ui, hot);
        if hot {
            self.keyboard(ui);
        }
        self.sync_textures(ui.ctx());
        if self.playing && self.anim_sel.is_some() {
            ui.ctx().request_repaint();
        }
        actions
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, actions: &mut Vec<PixelAction>) {
        ui.horizontal_wrapped(|ui| {
            for t in Tool::ALL {
                let r = ui
                    .selectable_label(self.tool == t, t.label())
                    .on_hover_text(format!("{} ({:?})\n{}", t.label(), t.key(), t.help()));
                crate::tutorial::anchor_in(ui, &format!("pixel:{}", t.label()), r.rect);
                if r.clicked() {
                    self.set_tool(t);
                }
            }
            ui.separator();
            if self.tool.uses_brush() {
                ui.label("Pincel");
                ui.add(egui::DragValue::new(&mut self.brush).range(1..=16).suffix(" px"))
                    .on_hover_text("Tamanho do pincel ([ e ] mudam)");
            }
            if self.tool.uses_fill() {
                ui.checkbox(&mut self.filled, "Preenchido");
            }
            let r = ui
                .checkbox(&mut self.mirror, "Espelhado")
                .on_hover_text("Desenha dos dois lados ao mesmo tempo (o meio é o meio do quadro): ótimo para personagens de frente.");
            crate::tutorial::anchor_in(ui, "pixel:espelhado", r.rect);
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("{} × {} px", self.img.width(), self.img.height()));
            let r = ui.menu_button("Tamanho…", |ui| {
                let size = self
                    .resize
                    .get_or_insert([self.img.width(), self.img.height()]);
                ui.label("Novo tamanho da imagem (o desenho fica no canto de cima, à esquerda):");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut size[0]).range(1..=4096).suffix(" px"));
                    ui.label("×");
                    ui.add(egui::DragValue::new(&mut size[1]).range(1..=4096).suffix(" px"));
                });
                let [w, h] = *size;
                if ui.button("Aplicar").clicked() {
                    self.resize_canvas(w, h);
                    self.resize = None;
                    ui.close();
                }
            });
            if r.response.clicked() {
                self.resize = Some([self.img.width(), self.img.height()]);
            }
            ui.separator();
            let mut frames = self.has_frames();
            let r = ui
                .checkbox(&mut frames, "Quadros")
                .on_hover_text("Divide a imagem em quadros do mesmo tamanho (uma folha de sprites).");
            crate::tutorial::anchor_in(ui, "pixel:quadros", r.rect);
            if r.changed() {
                if frames {
                    let h = self.img.height();
                    let w = if self.img.width() % h == 0 { h } else { self.img.width() };
                    self.set_frame_size(w.min(self.img.width()), h);
                } else {
                    self.set_frame_size(0, 0);
                }
            }
            if self.has_frames() {
                let (mut w, mut h) = (self.anim.frame_w, self.anim.frame_h);
                let a = ui.add(
                    egui::DragValue::new(&mut w)
                        .range(1..=self.img.width())
                        .suffix(" px"),
                );
                ui.label("×");
                let b = ui.add(
                    egui::DragValue::new(&mut h)
                        .range(1..=self.img.height())
                        .suffix(" px"),
                );
                if a.changed() || b.changed() {
                    self.set_frame_size(w, h);
                }
                ui.separator();
                ui.selectable_value(&mut self.only_frame, false, "Folha")
                    .on_hover_text("Mostra todos os quadros");
                let r = ui
                    .selectable_value(&mut self.only_frame, true, "Quadro")
                    .on_hover_text("Mostra só o quadro atual, maior");
                crate::tutorial::anchor_in(ui, "pixel:so-quadro", r.rect);
                ui.checkbox(&mut self.onion, "Papel vegetal").on_hover_text(
                    "Mostra o quadro anterior clarinho por baixo, para desenhar o próximo passo da animação.",
                );
            }
            ui.separator();
            ui.checkbox(&mut self.pixel_grid, "Grade");
            if ui.button("−").on_hover_text("Diminuir (rolar)").clicked() {
                self.zoom = Some((self.zoom.unwrap_or(8.0) / 2.0).max(0.25));
            }
            ui.label(match self.zoom {
                None => "Ajustado".to_string(),
                Some(z) => format!("{:.0}%", z * 100.0),
            });
            if ui.button("+").on_hover_text("Aumentar (rolar)").clicked() {
                self.zoom = Some((self.zoom.unwrap_or(8.0) * 2.0).min(128.0));
            }
            if ui.selectable_label(self.zoom.is_none(), "Ajustar").clicked() {
                self.zoom = None;
                self.pan = Vec2::ZERO;
            }
            ui.separator();
            let code = if self.has_frames() {
                format!(
                    "nn.assets.sheet(\"{}\", {}, {}, {{ pixel_art = true }})",
                    self.rel, self.anim.frame_w, self.anim.frame_h
                )
            } else {
                format!("nn.assets.texture(\"{}\", {{ pixel_art = true }})", self.rel)
            };
            if ui
                .button("📋 Copiar código")
                .on_hover_text(format!("Copia: {code}"))
                .clicked()
            {
                actions.push(PixelAction::CopyText(code));
            }
            let save = ui
                .add_enabled(self.is_dirty(), egui::Button::new("Salvar"))
                .on_hover_text("Ctrl+S. O jogo aberto e as cenas mostram a imagem nova ao salvar.");
            crate::tutorial::anchor_in(ui, "pixel:salvar", save.rect);
            if save.clicked() {
                actions.push(PixelAction::Save);
            }
        });
        if let Some(e) = &self.anim_error {
            ui.colored_label(crate::theme::ERROR, format!("Animações: {e}"));
        }
    }

    fn set_tool(&mut self, t: Tool) {
        if t != Tool::Select {
            self.commit_floating();
            self.selection = None;
        }
        self.tool = t;
    }

    fn side_panel(&mut self, ui: &mut egui::Ui, project: &mut Vec<Color>, actions: &mut Vec<PixelAction>) {
        ui.strong("Cores");
        egui::Grid::new(("pixel-cores-grade", &self.rel))
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Principal").on_hover_text("Botão esquerdo do mouse");
                ui.color_edit_button_srgba_unmultiplied(&mut self.primary);
                ui.end_row();
                ui.label("Secundária").on_hover_text("Botão direito do mouse");
                ui.color_edit_button_srgba_unmultiplied(&mut self.secondary);
                ui.end_row();
            });
        if ui.small_button("Trocar (X)").clicked() {
            std::mem::swap(&mut self.primary, &mut self.secondary);
        }
        ui.add_space(6.0);
        let names: Vec<String> = ["Cores desta imagem".to_string(), "Paleta do projeto".to_string()]
            .into_iter()
            .chain(ops::PALETTES.iter().map(|(n, _)| n.to_string()))
            .collect();
        let palette = self.palette.min(names.len() - 1);
        egui::ComboBox::from_id_salt(("paleta", &self.rel))
            .width(ui.available_width().min(200.0))
            .selected_text(&names[palette])
            .show_ui(ui, |ui| {
                for (i, n) in names.iter().enumerate() {
                    ui.selectable_value(&mut self.palette, i, n);
                }
            });
        let colors: Vec<Color> = match palette {
            0 => ops::used_colors(&self.img, 64),
            1 => project.clone(),
            i => ops::parse_hex_list(ops::PALETTES[i - 2].1),
        };
        let mut picked = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(2.0);
            // Transparente primeiro: o "apagador" de qualquer ferramenta.
            for c in std::iter::once(CLEAR).chain(colors.iter().copied()) {
                let (rect, r) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
                let p = ui.painter();
                if c[3] == 0 {
                    checkerboard(p, rect, 4.0);
                } else {
                    p.rect_filled(rect, 2.0, Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]));
                }
                let selected = c == self.primary;
                let stroke = if selected {
                    Stroke::new(2.0, ui.visuals().strong_text_color())
                } else {
                    Stroke::new(1.0, Color32::from_black_alpha(90))
                };
                p.rect_stroke(rect, 2.0, stroke, egui::StrokeKind::Inside);
                let r = r.on_hover_text(if c[3] == 0 {
                    "transparente".to_string()
                } else {
                    format!("#{}", ops::to_hex(c))
                });
                if r.clicked() {
                    picked = Some((c, false));
                }
                if r.secondary_clicked() {
                    picked = Some((c, true));
                }
            }
        });
        if let Some((c, secondary)) = picked {
            if secondary {
                self.secondary = c;
            } else {
                self.primary = c;
            }
        }
        if palette == 1 {
            ui.horizontal_wrapped(|ui| {
                if ui
                    .small_button("+ Cor principal")
                    .on_hover_text(format!("Guarda em {PROJECT_PALETTE}, para usar em todas as imagens"))
                    .clicked()
                    && self.primary[3] > 0
                    && !project.contains(&self.primary)
                {
                    project.push(self.primary);
                    actions.push(PixelAction::SavePalette);
                }
                if project.contains(&self.primary) && ui.small_button("Tirar a principal").clicked() {
                    project.retain(|c| *c != self.primary);
                    actions.push(PixelAction::SavePalette);
                }
            });
        }

        ui.add_space(10.0);
        ui.separator();
        ui.strong("Prévia");
        let anim = self.anim_sel.and_then(|i| self.anim.anims.get(i)).cloned();
        let frame = match &anim {
            Some(a) if self.playing => {
                let now = Instant::now();
                if let Some(last) = self.play_last {
                    self.play_time += now.duration_since(last).as_secs_f64();
                }
                self.play_last = Some(now);
                a.position_at(self.play_time)
                    .and_then(|p| a.frames.get(p))
                    .map(|f| f.index.saturating_sub(1))
                    .unwrap_or(self.current)
            }
            _ => {
                self.play_last = None;
                self.current
            }
        };
        let w = ui.available_width().min(200.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, w.min(160.0)), Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
        self.paint_frame(&painter, rect.shrink(6.0), frame.min(self.frame_count() - 1));
        match &anim {
            Some(a) => {
                ui.horizontal(|ui| {
                    let label = if self.playing { "Pausar" } else { "▶ Tocar" };
                    if ui.button(label).clicked() {
                        self.playing = !self.playing;
                        self.play_time = 0.0;
                    }
                    ui.weak(format!("\"{}\" · {:.2} s", a.name, a.duration()));
                });
            }
            None => {
                ui.weak(format!("Quadro {} de {}", self.current + 1, self.frame_count()));
            }
        }
    }

    fn frames_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.strong("Quadros");
            ui.weak(format!("{} de {}", self.current + 1, self.frame_count()));
            ui.separator();
            let r = ui
                .button("+ Novo quadro")
                .on_hover_text("Um quadro vazio depois do atual");
            crate::tutorial::anchor_in(ui, "pixel:novo-quadro", r.rect);
            if r.clicked() {
                self.add_frame(false);
            }
            let r = ui
                .button("Duplicar")
                .on_hover_text("Uma cópia do quadro atual logo depois dele: o jeito mais rápido de fazer o próximo passo de uma animação.");
            crate::tutorial::anchor_in(ui, "pixel:duplicar", r.rect);
            if r.clicked() {
                self.add_frame(true);
            }
            if ui
                .add_enabled(self.frame_count() > 1, egui::Button::new("Excluir"))
                .clicked()
            {
                self.delete_frame();
            }
            if ui
                .add_enabled(self.current > 0, egui::Button::new("Para trás"))
                .on_hover_text("Troca de lugar com o anterior")
                .clicked()
            {
                self.move_frame(false);
            }
            if ui
                .add_enabled(self.current + 1 < self.frame_count(), egui::Button::new("Para frente"))
                .on_hover_text("Troca de lugar com o próximo")
                .clicked()
            {
                self.move_frame(true);
            }
            ui.separator();
            if ui.button("Espelhar ↔").on_hover_text("Espelha a seleção (ou o quadro) na horizontal").clicked() {
                self.transform(Transform::FlipH);
            }
            if ui.button("Espelhar ↕").on_hover_text("Espelha na vertical").clicked() {
                self.transform(Transform::FlipV);
            }
            if ui.button("Girar").on_hover_text("Gira 90° (quadros quadrados)").clicked() {
                self.transform(Transform::Rotate);
            }
        });
        if !self.has_frames() {
            ui.weak("A imagem é um quadro só. Marque \"Quadros\" na barra de cima para dividir uma folha de sprites, ou crie mais quadros com \"+ Novo quadro\" e \"Duplicar\".");
            return;
        }
        let size = 52.0;
        egui::ScrollArea::horizontal()
            .id_salt(("fita", &self.rel))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for i in 0..self.frame_count() {
                        let id = egui::Id::new(("quadro", &self.rel, i));
                        let r = ui.dnd_drag_source(id, FrameDrag(i), |ui| {
                            let (rect, r) = ui.allocate_exact_size(Vec2::new(size, size + 14.0), Sense::click());
                            let p = ui.painter_at(rect);
                            let thumb = Rect::from_min_size(rect.min, Vec2::splat(size));
                            p.rect_filled(thumb, 3.0, ui.visuals().extreme_bg_color);
                            self.paint_frame(&p, thumb.shrink(3.0), i);
                            if i == self.current {
                                p.rect_stroke(thumb, 3.0, Stroke::new(2.0, crate::theme::ACCENT), egui::StrokeKind::Inside);
                            }
                            p.text(
                                Pos2::new(thumb.center().x, thumb.bottom() + 7.0),
                                egui::Align2::CENTER_CENTER,
                                (i + 1).to_string(),
                                egui::FontId::proportional(11.0),
                                ui.visuals().text_color(),
                            );
                            r
                        });
                        let clicked = r.inner.clicked() || (r.response.clicked());
                        if clicked {
                            self.current = i;
                            self.playing = false;
                        }
                        r.response.on_hover_text(format!(
                            "Quadro {} (clique para desenhar nele; arraste para uma animação)",
                            i + 1
                        ));
                    }
                });
            });
    }

    fn anims_panel(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.strong("Animações");
            ui.weak(format!("(salvas em {})", anim_path(&self.rel).rsplit('/').next().unwrap_or("")));
            let r = ui
                .button("+ Nova animação")
                .on_hover_text("A primeira usa todos os quadros; as outras começam com o quadro atual.");
            crate::tutorial::anchor_in(ui, "pixel:nova-animacao", r.rect);
            if r.clicked() {
                self.new_anim();
                self.playing = true;
            }
        });
        if self.anim.anims.is_empty() {
            ui.weak("Nenhuma ainda. Uma animação é uma sequência de quadros com nome (\"andar\", \"pular\"), usada pela Animação automática, pelos eventos e pelo código.");
            return;
        }
        let mut remove = None;
        let mut duplicate = None;
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(150.0);
                for i in 0..self.anim.anims.len() {
                    let name = self.anim.anims[i].name.clone();
                    let r = ui.selectable_label(self.anim_sel == Some(i), &name);
                    if r.clicked() {
                        self.anim_sel = Some(i);
                        self.play_time = 0.0;
                        self.playing = true;
                    }
                    r.context_menu(|ui| {
                        if ui.button("Duplicar").clicked() {
                            duplicate = Some(i);
                            ui.close();
                        }
                        if ui.button("Excluir").clicked() {
                            remove = Some(i);
                            ui.close();
                        }
                    });
                }
            });
            ui.separator();
            if let Some(i) = self.anim_sel.filter(|i| *i < self.anim.anims.len()) {
                ui.vertical(|ui| self.anim_editor(ui, i, &mut remove, &mut duplicate));
            }
        });
        if let Some(i) = duplicate {
            self.checkpoint(None);
            let mut copy = self.anim.anims[i].clone();
            copy.name = self.anim.free_name(&copy.name);
            self.anim.anims.insert(i + 1, copy);
            self.anim_sel = Some(i + 1);
            self.touch();
        }
        if let Some(i) = remove {
            self.checkpoint(None);
            self.anim.anims.remove(i);
            self.anim_sel = if self.anim.anims.is_empty() {
                None
            } else {
                Some(i.min(self.anim.anims.len() - 1))
            };
            self.touch();
        }
    }

    fn anim_editor(&mut self, ui: &mut egui::Ui, i: usize, remove: &mut Option<usize>, duplicate: &mut Option<usize>) {
        let a = self.anim.anims[i].clone();
        ui.horizontal_wrapped(|ui| {
            ui.label("Nome");
            let mut name = a.name.clone();
            let r = ui.add(egui::TextEdit::singleline(&mut name).desired_width(110.0));
            crate::tutorial::anchor_in(ui, "pixel:nome-animacao", r.rect);
            if r.changed() {
                let taken = self.anim.anims.iter().enumerate().any(|(k, o)| k != i && o.name == name);
                if !name.trim().is_empty() && !taken {
                    self.anim_edit(i, "nome", |a| a.name = name);
                }
            }
            ui.label("Ritmo");
            let mut fps = a.fps;
            if ui
                .add(egui::DragValue::new(&mut fps).range(0.5..=60.0).speed(0.2).suffix(" quadros/s"))
                .changed()
            {
                self.anim_edit(i, "fps", |a| a.fps = fps);
            }
            let mut mode = a.mode;
            egui::ComboBox::from_id_salt(("modo", &self.rel, i))
                .selected_text(mode.label())
                .show_ui(ui, |ui| {
                    for m in Mode::ALL {
                        ui.selectable_value(&mut mode, m, m.label());
                    }
                });
            if mode != a.mode {
                self.anim_edit(i, "modo", |a| a.mode = mode);
            }
            ui.weak(format!("{:.2} s", a.duration()));
            if ui.small_button("Duplicar").clicked() {
                *duplicate = Some(i);
            }
            if ui.small_button("Excluir").clicked() {
                *remove = Some(i);
            }
        });
        ui.horizontal_wrapped(|ui| {
            let r = ui
                .small_button("+ Quadro atual")
                .on_hover_text("Põe o quadro atual no fim da animação (ou arraste quadros da fita para cá)");
            crate::tutorial::anchor_in(ui, "pixel:mais-quadro", r.rect);
            if r.clicked() {
                let f = self.current + 1;
                self.anim_edit(i, "", |a| a.frames.push(Frame { index: f, time: None }));
                self.last_edit = None;
            }
            if ui.small_button("Todos os quadros").clicked() {
                let n = self.frame_count();
                self.anim_edit(i, "", |a| {
                    a.frames = (1..=n).map(|index| Frame { index, time: None }).collect();
                });
                self.last_edit = None;
            }
        });
        // Linha do tempo.
        let size = 46.0;
        let mut drop_at: Option<(usize, Drop)> = None;
        let mut select = None;
        let mut edits: Vec<(usize, Option<Option<f64>>)> = Vec::new();
        egui::ScrollArea::horizontal()
            .id_salt(("linha-do-tempo", &self.rel, i))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let playing_pos = (self.playing).then(|| a.position_at(self.play_time)).flatten();
                    for (k, f) in a.frames.iter().enumerate() {
                        let id = egui::Id::new(("tl", &self.rel, i, k));
                        let r = ui.dnd_drag_source(id, TimelineDrag(k), |ui| {
                            let (rect, r) = ui.allocate_exact_size(Vec2::new(size, size + 26.0), Sense::click());
                            let p = ui.painter_at(rect);
                            let thumb = Rect::from_min_size(rect.min, Vec2::splat(size));
                            p.rect_filled(thumb, 3.0, ui.visuals().extreme_bg_color);
                            self.paint_frame(&p, thumb.shrink(3.0), f.index.saturating_sub(1).min(self.frame_count() - 1));
                            let on = playing_pos == Some(k);
                            if on || (!self.playing && f.index == self.current + 1) {
                                p.rect_stroke(thumb, 3.0, Stroke::new(2.0, crate::theme::ACCENT), egui::StrokeKind::Inside);
                            }
                            let ms = (a.frame_time(k) * 1000.0).round();
                            p.text(
                                Pos2::new(thumb.center().x, thumb.bottom() + 7.0),
                                egui::Align2::CENTER_CENTER,
                                format!("{}", f.index),
                                egui::FontId::proportional(11.0),
                                ui.visuals().text_color(),
                            );
                            p.text(
                                Pos2::new(thumb.center().x, thumb.bottom() + 19.0),
                                egui::Align2::CENTER_CENTER,
                                format!("{ms} ms"),
                                egui::FontId::proportional(10.0),
                                if f.time.is_some() {
                                    crate::theme::ACCENT
                                } else {
                                    ui.visuals().weak_text_color()
                                },
                            );
                            r
                        });
                        let resp = r.inner.union(r.response.clone());
                        if resp.clicked() {
                            select = Some(f.index.saturating_sub(1));
                        }
                        // Soltar quadros antes deste.
                        if let Some(d) = resp.dnd_release_payload::<FrameDrag>() {
                            drop_at = Some((k, Drop::Frame(d.0)));
                        } else if let Some(d) = resp.dnd_release_payload::<TimelineDrag>() {
                            drop_at = Some((k, Drop::Move(d.0)));
                        }
                        if resp.dnd_hover_payload::<FrameDrag>().is_some()
                            || resp.dnd_hover_payload::<TimelineDrag>().is_some()
                        {
                            let x = resp.rect.left() - 2.0;
                            ui.painter().vline(x, resp.rect.y_range(), Stroke::new(3.0, crate::theme::ACCENT));
                        }
                        let ft = a.frame_time(k);
                        resp.on_hover_text("Clique: desenhar neste quadro. Arraste para mudar a ordem. Botão direito: duração.")
                            .context_menu(|ui| {
                                ui.label(format!("Quadro {} da folha", f.index));
                                let mut ms = (ft * 1000.0).round();
                                ui.horizontal(|ui| {
                                    ui.label("Duração");
                                    if ui
                                        .add(egui::DragValue::new(&mut ms).range(10.0..=10_000.0).speed(5.0).suffix(" ms"))
                                        .changed()
                                    {
                                        edits.push((k, Some(Some(ms / 1000.0))));
                                    }
                                });
                                if f.time.is_some() && ui.button("Usar o ritmo da animação").clicked() {
                                    edits.push((k, Some(None)));
                                    ui.close();
                                }
                                if ui.button("Tirar da animação").clicked() {
                                    edits.push((k, None));
                                    ui.close();
                                }
                            });
                    }
                    // Espaço no fim para soltar.
                    let (rect, end) = ui.allocate_exact_size(Vec2::new(size, size), Sense::hover());
                    let hovering = end.dnd_hover_payload::<FrameDrag>().is_some()
                        || end.dnd_hover_payload::<TimelineDrag>().is_some();
                    ui.painter().rect_stroke(
                        rect.shrink(2.0),
                        3.0,
                        Stroke::new(1.0, if hovering { crate::theme::ACCENT } else { ui.visuals().weak_text_color() }),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "+",
                        egui::FontId::proportional(18.0),
                        ui.visuals().weak_text_color(),
                    );
                    if let Some(d) = end.dnd_release_payload::<FrameDrag>() {
                        drop_at = Some((a.frames.len(), Drop::Frame(d.0)));
                    } else if let Some(d) = end.dnd_release_payload::<TimelineDrag>() {
                        drop_at = Some((a.frames.len(), Drop::Move(d.0)));
                    }
                    end.on_hover_text("Arraste um quadro da fita para cá");
                });
            });
        if let Some(f) = select {
            self.current = f.min(self.frame_count() - 1);
            self.playing = false;
        }
        for (k, e) in edits {
            match e {
                Some(t) => self.anim_edit(i, &format!("tempo{k}"), |a| {
                    if let Some(f) = a.frames.get_mut(k) {
                        f.time = t;
                    }
                }),
                None => {
                    self.anim_edit(i, "", |a| {
                        if k < a.frames.len() {
                            a.frames.remove(k);
                        }
                    });
                    self.last_edit = None;
                }
            }
        }
        if let Some((at, what)) = drop_at {
            self.anim_edit(i, "", |a| match what {
                Drop::Frame(f) => a.frames.insert(at.min(a.frames.len()), Frame { index: f + 1, time: None }),
                Drop::Move(from) => {
                    if from < a.frames.len() {
                        let f = a.frames.remove(from);
                        let to = if at > from { at - 1 } else { at };
                        a.frames.insert(to.min(a.frames.len()), f);
                    }
                }
            });
            self.last_edit = None;
        }
    }

    fn canvas(&mut self, ui: &mut egui::Ui, hot: bool) {
        let avail = ui.available_rect_before_wrap();
        let (rect, resp) = ui.allocate_exact_size(avail.size(), Sense::click_and_drag());
        crate::tutorial::anchor_in(ui, "pixel:tela", resp.rect);
        let view = if self.only_frame && self.has_frames() {
            self.cell(self.current)
        } else {
            PixRect::of(&self.img)
        };
        let view_size = Vec2::new(view.w as f32, view.h as f32);
        let fit = ((rect.width() - 24.0) / view_size.x)
            .min((rect.height() - 24.0) / view_size.y)
            .clamp(0.1, 64.0);
        let fit = if fit > 1.0 { fit.floor() } else { fit };
        let zoom = self.zoom.unwrap_or(fit);
        let origin = rect.center() + self.pan - view_size * zoom / 2.0;
        let to_screen = |x: f32, y: f32| origin + Vec2::new(x - view.x as f32, y - view.y as f32) * zoom;
        let to_pixel = |p: Pos2| {
            let v = (p - origin) / zoom;
            ((v.x + view.x as f32).floor() as i32, (v.y + view.y as f32).floor() as i32)
        };

        // Zoom (rolar) e arrastar a vista (botão do meio ou Espaço).
        if let Some(p) = resp.hover_pos() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            let z = ui.input(|i| i.zoom_delta());
            let factor = (1.0025f32).powf(scroll) * z;
            if (factor - 1.0).abs() > 1e-4 {
                let new_zoom = (zoom * factor).clamp(0.1, 128.0);
                let fpix = (p - origin) / zoom;
                let new_origin = p - fpix * new_zoom;
                self.pan = new_origin - rect.center() + view_size * new_zoom / 2.0;
                self.zoom = Some(new_zoom);
            }
        }
        let (space, alt, shift) = ui.input(|i| (i.key_down(Key::Space), i.modifiers.alt, i.modifiers.shift));
        let panning = resp.dragged_by(PointerButton::Middle) || (space && resp.dragged_by(PointerButton::Primary));
        if panning {
            self.pan += resp.drag_delta();
        }

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
        let img_rect = Rect::from_min_size(origin, view_size * zoom);
        checkerboard(&painter, img_rect, (zoom * 4.0).clamp(6.0, 16.0));
        // Papel vegetal: o quadro anterior (na animação escolhida, se ele estiver nela).
        if self.onion && self.has_frames() && self.frame_count() > 1 {
            let prev = self.onion_frame();
            if let (Some(prev), Some(tex)) = (prev, &self.tex) {
                let cur = self.cell(self.current);
                let r = Rect::from_min_size(to_screen(cur.x as f32, cur.y as f32), Vec2::new(cur.w as f32, cur.h as f32) * zoom);
                painter.image(tex.id(), r, self.uv(self.cell(prev)), Color32::from_white_alpha(70));
            }
        }
        if let Some(tex) = &self.tex {
            painter.image(tex.id(), img_rect, self.uv(view), Color32::WHITE);
        }
        if let (Some(f), Some(tex)) = (&self.floating, &self.float_tex) {
            let r = Rect::from_min_size(to_screen(f.x as f32, f.y as f32), Vec2::new(f.img.width() as f32, f.img.height() as f32) * zoom);
            painter.image(tex.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            dashed_rect(&painter, r);
        }
        // Grades.
        if self.pixel_grid && zoom >= 6.0 && view.w + view.h <= 1024 {
            let stroke = Stroke::new(1.0, Color32::from_white_alpha(22));
            for x in 0..=view.w {
                let sx = img_rect.left() + x as f32 * zoom;
                painter.vline(sx, img_rect.y_range(), stroke);
            }
            for y in 0..=view.h {
                let sy = img_rect.top() + y as f32 * zoom;
                painter.hline(img_rect.x_range(), sy, stroke);
            }
        }
        if self.has_frames() && !self.only_frame {
            let stroke = Stroke::new(1.0, Color32::from_rgb(0xFF, 0x9F, 0x1C).gamma_multiply(0.7));
            for i in 0..self.frame_count() {
                let c = self.cell(i);
                let r = Rect::from_min_size(to_screen(c.x as f32, c.y as f32), Vec2::new(c.w as f32, c.h as f32) * zoom);
                painter.rect_stroke(r, 0.0, stroke, egui::StrokeKind::Inside);
            }
            if self.frame_count() > 1 {
                let c = self.cell(self.current);
                let r = Rect::from_min_size(to_screen(c.x as f32, c.y as f32), Vec2::new(c.w as f32, c.h as f32) * zoom);
                painter.rect_stroke(r, 0.0, Stroke::new(2.0, crate::theme::ACCENT), egui::StrokeKind::Outside);
            }
        }
        if self.mirror {
            let c = self.cell(self.current);
            let x = to_screen(c.x as f32 + c.w as f32 / 2.0, 0.0).x;
            let top = to_screen(0.0, c.y as f32).y;
            painter.vline(x, top..=top + c.h as f32 * zoom, Stroke::new(1.0, Color32::from_rgb(0x41, 0xA6, 0xF6)));
        }
        if let Some(s) = self.selection
            && self.floating.is_none()
        {
            let r = Rect::from_min_size(to_screen(s.x as f32, s.y as f32), Vec2::new(s.w as f32, s.h as f32) * zoom);
            dashed_rect(&painter, r);
        }

        // Desenhar.
        self.hover_px = resp.hover_pos().map(to_pixel);
        let pointer = ui.input(|i| i.pointer.interact_pos()).map(to_pixel);
        if !panning && !space {
            let pressed = ui.input(|i| {
                if i.pointer.button_pressed(PointerButton::Primary) {
                    Some(PointerButton::Primary)
                } else if i.pointer.button_pressed(PointerButton::Secondary) {
                    Some(PointerButton::Secondary)
                } else {
                    None
                }
            });
            if let (Some(button), true, Some(p)) = (pressed, resp.hovered(), pointer) {
                resp.request_focus();
                if alt && self.tool != Tool::Select {
                    self.begin_stroke(Tool::Picker, button, p, false);
                } else if self.tool == Tool::Select {
                    self.select_press(p);
                } else {
                    self.begin_stroke(self.tool, button, p, shift);
                }
            }
            if let Some(p) = pointer {
                if let Some(s) = &self.stroke {
                    let down = ui.input(|i| i.pointer.button_down(s.button));
                    if down {
                        if p != s.last || matches!(s.tool, Tool::Line | Tool::Rect | Tool::Ellipse) {
                            self.continue_stroke(p, shift);
                        }
                    } else {
                        self.end_stroke();
                    }
                } else if self.tool == Tool::Select && ui.input(|i| i.pointer.primary_down()) {
                    self.select_drag(p);
                }
            }
            if ui.input(|i| i.pointer.primary_released()) {
                if self.stroke.is_some() {
                    self.end_stroke();
                }
                self.sel_start = None;
                self.float_grab = None;
            }
        }

        // Cursor e coordenadas.
        if let Some((x, y)) = self.hover_px
            && hot
        {
            if PixRect::of(&self.img).contains(x, y) {
                let r = Rect::from_min_size(to_screen(x as f32, y as f32), Vec2::splat(zoom));
                painter.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::from_white_alpha(140)), egui::StrokeKind::Outside);
                let mut info = format!("x {x} · y {y}");
                if self.has_frames()
                    && let Some(f) = self.frame_at(x, y)
                {
                    info.push_str(&format!(" · quadro {}", f + 1));
                }
                painter.text(
                    rect.left_bottom() + Vec2::new(8.0, -6.0),
                    egui::Align2::LEFT_BOTTOM,
                    info,
                    egui::FontId::monospace(12.0),
                    ui.visuals().text_color(),
                );
            }
            if space || panning {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            } else {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            }
        }
    }

    /// Quadro mostrado no papel vegetal: o anterior na animação escolhida
    /// (se o atual estiver nela) ou o anterior na folha.
    fn onion_frame(&self) -> Option<u32> {
        if let Some(a) = self.anim_sel.and_then(|i| self.anim.anims.get(i))
            && let Some(pos) = a.frames.iter().position(|f| f.index == self.current + 1)
            && a.frames.len() > 1
        {
            let prev = if pos == 0 { a.frames.len() - 1 } else { pos - 1 };
            return Some(a.frames[prev].index.saturating_sub(1).min(self.frame_count() - 1));
        }
        (self.current > 0).then(|| self.current - 1)
    }

    fn select_press(&mut self, p: (i32, i32)) {
        if let Some(f) = &self.floating {
            let r = PixRect::new(f.x.max(0) as u32, f.y.max(0) as u32, f.img.width(), f.img.height());
            if p.0 >= f.x && p.1 >= f.y && p.0 < f.x + r.w as i32 && p.1 < f.y + r.h as i32 {
                self.float_grab = Some((p.0 - f.x, p.1 - f.y));
                return;
            }
            self.commit_floating();
        }
        if let Some(s) = self.selection
            && s.contains(p.0, p.1)
            && self.lift_selection()
        {
            self.float_grab = Some((p.0 - s.x as i32, p.1 - s.y as i32));
            return;
        }
        self.selection = None;
        self.sel_start = Some(p);
        if let Some(f) = self.frame_at(p.0, p.1) {
            self.current = f;
        }
    }

    fn select_drag(&mut self, p: (i32, i32)) {
        if let (Some(g), Some(f)) = (self.float_grab, self.floating.as_mut()) {
            let (nx, ny) = (p.0 - g.0, p.1 - g.1);
            if (nx, ny) != (f.x, f.y) {
                f.x = nx;
                f.y = ny;
            }
            return;
        }
        if let Some(a) = self.sel_start {
            self.selection = PixRect::between(a, p, PixRect::of(&self.img));
        }
    }

    fn keyboard(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if ctx.text_edit_focused() {
            return;
        }
        let (copy, cut, paste) = ui.input(|i| {
            let mut out = (false, false, false);
            for e in &i.events {
                match e {
                    egui::Event::Copy => out.0 = true,
                    egui::Event::Cut => out.1 = true,
                    egui::Event::Paste(_) => out.2 = true,
                    _ => {}
                }
            }
            out
        });
        if copy {
            self.copy(&ctx);
        }
        if cut {
            self.cut(&ctx);
        }
        if paste {
            self.paste();
        }
        let pressed = |k: Key| ui.input(|i| i.key_pressed(k) && i.modifiers.is_none());
        for t in Tool::ALL {
            if pressed(t.key()) {
                self.set_tool(t);
            }
        }
        if pressed(Key::P) {
            self.set_tool(Tool::Pencil);
        }
        if pressed(Key::X) {
            std::mem::swap(&mut self.primary, &mut self.secondary);
        }
        if pressed(Key::OpenBracket) {
            self.brush = self.brush.saturating_sub(1).max(1);
        }
        if pressed(Key::CloseBracket) {
            self.brush = (self.brush + 1).min(16);
        }
        if pressed(Key::Delete) || pressed(Key::Backspace) {
            self.delete_selection();
        }
        if pressed(Key::Enter) {
            self.commit_floating();
            self.selection = None;
        }
        if pressed(Key::Escape) {
            self.commit_floating();
            self.selection = None;
        }
        if pressed(Key::Comma) && self.current > 0 {
            self.current -= 1;
            self.playing = false;
        }
        if pressed(Key::Period) && self.current + 1 < self.frame_count() {
            self.current += 1;
            self.playing = false;
        }
        if pressed(Key::H) {
            self.transform(Transform::FlipH);
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Drop {
    Frame(u32),
    Move(usize),
}

/// Espelhar ou girar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transform {
    FlipH,
    FlipV,
    Rotate,
}

impl Transform {
    fn apply(self, img: &RgbaImage) -> RgbaImage {
        match self {
            Transform::FlipH => image::imageops::flip_horizontal(img),
            Transform::FlipV => image::imageops::flip_vertical(img),
            Transform::Rotate => image::imageops::rotate90(img),
        }
    }
}

fn checkerboard(painter: &egui::Painter, rect: Rect, cell: f32) {
    let clip = painter.clip_rect().intersect(rect);
    if clip.width() <= 0.0 || clip.height() <= 0.0 {
        return;
    }
    let a = Color32::from_gray(0x5A);
    let b = Color32::from_gray(0x46);
    painter.rect_filled(clip, 0.0, a);
    let c0 = ((clip.left() - rect.left()) / cell).floor() as i64;
    let c1 = ((clip.right() - rect.left()) / cell).ceil() as i64;
    let r0 = ((clip.top() - rect.top()) / cell).floor() as i64;
    let r1 = ((clip.bottom() - rect.top()) / cell).ceil() as i64;
    if (c1 - c0) * (r1 - r0) > 40_000 {
        return;
    }
    for r in r0..r1 {
        for c in c0..c1 {
            if (r + c).rem_euclid(2) == 1 {
                let min = rect.min + Vec2::new(c as f32 * cell, r as f32 * cell);
                let cell_rect = Rect::from_min_size(min, Vec2::splat(cell)).intersect(clip);
                painter.rect_filled(cell_rect, 0.0, b);
            }
        }
    }
}

fn dashed_rect(painter: &egui::Painter, r: Rect) {
    painter.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::BLACK), egui::StrokeKind::Outside);
    let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    painter.extend(egui::Shape::dashed_line(&pts, Stroke::new(1.0, Color32::WHITE), 4.0, 4.0));
}

/// Para a lista de "Nova imagem": tamanhos comuns de quadro.
pub const FRAME_SIZES: [u32; 5] = [8, 16, 24, 32, 64];

/// Cria a imagem nova (transparente) com `frames` quadros de `w`×`h`.
pub fn blank(w: u32, h: u32, frames: u32) -> (RgbaImage, AnimFile) {
    let frames = frames.max(1);
    let img = RgbaImage::new(w.max(1) * frames, h.max(1));
    let anim = if frames > 1 {
        AnimFile {
            frame_w: w,
            frame_h: h,
            anims: Vec::new(),
        }
    } else {
        AnimFile::default()
    };
    (img, anim)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> PixelDoc {
        let (img, anim) = blank(4, 4, 3);
        PixelDoc::new_image("a.png", img, anim)
    }

    #[test]
    fn desenhar_desfazer_e_salvar_marcam_alteracoes() {
        let mut d = doc();
        assert!(!d.is_dirty());
        d.begin_stroke(Tool::Pencil, PointerButton::Primary, (1, 1), false);
        d.continue_stroke((3, 1), false);
        d.end_stroke();
        assert!(d.is_dirty());
        assert_eq!(ops::get(&d.img, 2, 1), Some(d.primary));
        assert_eq!(ops::get(&d.img, 4, 1), Some(CLEAR), "não passa para o quadro 2");
        d.undo();
        assert!(!d.is_dirty(), "desfazer volta ao estado salvo");
        d.redo();
        assert!(d.is_dirty());
        let dir = std::env::temp_dir().join(format!("nn-pixel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        d.new_anim();
        d.save(&dir).unwrap();
        assert!(!d.is_dirty());
        assert!(dir.join("a.anim").is_file());
        let back = PixelDoc::open(&dir, "a.png");
        assert_eq!(back.img, d.img);
        assert_eq!(back.anim, d.anim);
        assert_eq!(back.anim.anims[0].frames.len(), 3);
    }

    #[test]
    fn clique_sem_mudar_nada_nao_fica_no_historico() {
        let mut d = doc();
        d.primary = CLEAR;
        d.begin_stroke(Tool::Pencil, PointerButton::Primary, (1, 1), false);
        d.end_stroke();
        assert!(!d.can_undo());
        assert!(!d.is_dirty());
    }

    #[test]
    fn retangulo_com_shift_e_quadrado_e_balde() {
        let mut d = doc();
        d.begin_stroke(Tool::Rect, PointerButton::Primary, (4, 0), true);
        d.continue_stroke((7, 2), true);
        d.end_stroke();
        // Shift: quadrado de 3×3, só a borda, dentro do quadro 2.
        assert_eq!(ops::get(&d.img, 7, 3), Some(d.primary));
        assert_eq!(ops::get(&d.img, 5, 1), Some(CLEAR));
        assert_eq!(d.current, 1);
        d.begin_stroke(Tool::Fill, PointerButton::Secondary, (5, 1), false);
        assert_eq!(ops::get(&d.img, 5, 1), Some(d.secondary));
        d.secondary = [9, 9, 9, 255];
        d.begin_stroke(Tool::Fill, PointerButton::Secondary, (0, 0), false);
        assert_eq!(ops::get(&d.img, 3, 3), Some([9, 9, 9, 255]));
        assert_eq!(ops::get(&d.img, 8, 0), Some(CLEAR), "o balde não sai do quadro");
    }

    #[test]
    fn selecionar_mover_copiar_e_colar() {
        let mut d = doc();
        d.begin_stroke(Tool::Pencil, PointerButton::Primary, (0, 0), false);
        d.end_stroke();
        d.tool = Tool::Select;
        d.select_press((0, 0));
        d.select_drag((1, 1));
        assert_eq!(d.selection, Some(PixRect::new(0, 0, 2, 2)));
        // Arrasta a seleção um pixel para a direita.
        d.select_press((0, 0));
        d.select_drag((1, 0));
        d.commit_floating();
        assert_eq!(ops::get(&d.img, 0, 0), Some(CLEAR));
        assert_eq!(ops::get(&d.img, 1, 0), Some(d.primary));
        let ctx = egui::Context::default();
        d.copy(&ctx);
        d.current = 2;
        d.paste();
        d.commit_floating();
        assert_eq!(ops::get(&d.img, 8, 0), Some(d.primary), "colou no quadro 3");
        d.transform(Transform::FlipH);
        d.commit_floating();
        assert_eq!(ops::get(&d.img, 9, 0), Some(d.primary), "espelhou a seleção");
        assert_eq!(ops::get(&d.img, 8, 0), Some(CLEAR));
    }

    #[test]
    fn espelhado_desenha_dos_dois_lados_do_quadro() {
        let mut d = doc();
        d.mirror = true;
        d.begin_stroke(Tool::Pencil, PointerButton::Primary, (4, 2), false);
        d.end_stroke();
        assert_eq!(ops::get(&d.img, 4, 2), Some(d.primary));
        assert_eq!(ops::get(&d.img, 7, 2), Some(d.primary));
    }

    #[test]
    fn edicoes_seguidas_da_animacao_viram_um_passo() {
        let mut d = doc();
        let i = d.new_anim();
        let steps = d.history.len();
        for fps in [11.0, 12.0, 13.0] {
            d.anim_edit(i, "fps", |a| a.fps = fps);
        }
        assert_eq!(d.history.len(), steps + 1);
        d.undo();
        assert_eq!(d.anim.anims[0].fps, 10.0);
    }
}
