//! Painel Arquivos: a árvore do projeto com ícones e miniaturas, menus de
//! contexto, renomear no lugar e filtro.

use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

use eframe::egui::{self, Key, RichText, TextureHandle, Vec2};

use crate::files::{FileEntry, FileKind, FileTree};
use crate::project::{Action, NewKind};
use crate::settings::Mode;

/// Novas miniaturas carregadas por quadro (o resto vem nos próximos).
const THUMBS_PER_FRAME: usize = 6;
const THUMB: f32 = 16.0;

#[derive(Default)]
pub struct FilePanel {
    pub selected: Option<String>,
    expanded: HashSet<String>,
    initialized: bool,
    renaming: Option<Rename>,
    /// Arquivo para rolar até ele no próximo quadro.
    scroll_to: Option<String>,
    pub filter: String,
    thumbs: HashMap<String, (Option<SystemTime>, Option<TextureHandle>)>,
}

impl std::fmt::Debug for FilePanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilePanel")
            .field("selected", &self.selected)
            .field("filter", &self.filter)
            .finish()
    }
}

#[derive(Debug)]
struct Rename {
    rel: String,
    name: String,
    focus: bool,
}

impl FilePanel {
    /// Pasta (relativa) da seleção: onde coisas novas são criadas.
    pub fn target_dir(&self, tree: &FileTree) -> Option<String> {
        let sel = self.selected.as_ref()?;
        let e = tree.get(sel)?;
        Some(crate::files::parent_dir(&e.rel, e.is_dir))
    }

    /// Começa a renomear um arquivo (F2).
    pub fn start_rename(&mut self, rel: &str) {
        let name = rel.rsplit('/').next().unwrap_or(rel).to_string();
        self.renaming = Some(Rename {
            rel: rel.to_string(),
            name,
            focus: true,
        });
    }

    /// Mostra um arquivo: abre as pastas acima dele e o seleciona.
    pub fn reveal(&mut self, rel: &str) {
        let mut dir = rel;
        while let Some((parent, _)) = dir.rsplit_once('/') {
            self.expanded.insert(parent.to_string());
            dir = parent;
        }
        self.selected = Some(rel.to_string());
        self.scroll_to = Some(rel.to_string());
    }

    /// Um arquivo mudou de nome: atualiza a seleção e as pastas abertas.
    pub fn renamed(&mut self, from: &str, to: &str) {
        let fix = |p: &str| -> Option<String> {
            if p == from {
                Some(to.to_string())
            } else {
                p.strip_prefix(&format!("{from}/"))
                    .map(|rest| format!("{to}/{rest}"))
            }
        };
        if let Some(sel) = &self.selected
            && let Some(n) = fix(sel)
        {
            self.selected = Some(n);
        }
        self.expanded = self
            .expanded
            .iter()
            .map(|p| fix(p).unwrap_or_else(|| p.clone()))
            .collect();
        self.thumbs.retain(|k, _| fix(k).is_none());
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        tree: &FileTree,
        mode: Mode,
        title: &str,
        actions: &mut Vec<Action>,
    ) {
        if !self.initialized {
            self.initialized = true;
            // Projetos pequenos abrem com todas as pastas à mostra.
            let small = tree.entries().len() <= 80;
            for e in tree.entries().iter().filter(|e| e.is_dir) {
                if small || e.depth == 0 {
                    self.expanded.insert(e.rel.clone());
                }
            }
        }

        ui.horizontal(|ui| {
            let novo = ui.menu_button("➕ Novo", |ui| {
                let dir = self.target_dir(tree).unwrap_or_default();
                if ui.button("🖼 Cena visual").on_hover_text("Uma tela do jogo montada com o mouse: arraste imagens, mapas e sons para ela").clicked() {
                    actions.push(Action::NewItem { kind: NewKind::VisualScene, parent: scene_dir(tree, &dir) });
                    ui.close();
                }
                if ui.button("📜 Cena em código").on_hover_text("Uma tela do jogo escrita em Lua").clicked() {
                    actions.push(Action::NewItem { kind: NewKind::Scene, parent: scene_dir(tree, &dir) });
                    ui.close();
                }
                let r = ui.button("🎨 Imagem (pixel art)").on_hover_text("Uma imagem nova para desenhar no editor de pixel art (personagem, item, cenário...)");
                crate::tutorial::anchor_in(ui, "arquivos:nova-imagem", r.rect);
                if r.clicked() {
                    actions.push(Action::NewItem { kind: NewKind::Image, parent: image_dir(tree, &dir) });
                    ui.close();
                }
                let r = ui.button("🎲 Modelo 3D").on_hover_text("Um objeto, personagem ou cenário 3D montado no modelador, com peças e cores. Ao salvar, o .glb sai junto para usar nas cenas");
                crate::tutorial::anchor_in(ui, "arquivos:novo-modelo", r.rect);
                if r.clicked() {
                    actions.push(Action::NewItem { kind: NewKind::Model, parent: model_dir(&dir) });
                    ui.close();
                }
                let r = ui.button("💬 Diálogo (conversa)").on_hover_text("Uma conversa com falas e escolhas, montada no editor de conversa. Na cena, use o comportamento Diálogo");
                crate::tutorial::anchor_in(ui, "arquivos:novo-dialogo", r.rect);
                if r.clicked() {
                    actions.push(Action::NewItem { kind: NewKind::Dialogue, parent: dialogue_dir(&dir) });
                    ui.close();
                }
                if ui.button("🔧 Script (módulo)").on_hover_text("Código usado por várias cenas, com require").clicked() {
                    actions.push(Action::NewItem { kind: NewKind::Module, parent: module_dir(tree, &dir) });
                    ui.close();
                }
                if ui.button("📁 Pasta").clicked() {
                    actions.push(Action::NewItem { kind: NewKind::Folder, parent: dir.clone() });
                    ui.close();
                }
            });
            crate::tutorial::anchor_in(ui, "arquivos.novo", novo.response.rect);
            if ui
                .button("📥 Importar")
                .on_hover_text("Copiar imagens, sons e outros arquivos para o projeto.\nVocê também pode arrastar arquivos para esta janela.")
                .clicked()
            {
                actions.push(Action::Import { dest: self.target_dir(tree) });
            }
            ui.add(
                egui::TextEdit::singleline(&mut self.filter)
                    .hint_text("🔍 Filtrar")
                    .desired_width(ui.available_width().max(40.0)),
            );
        });
        ui.separator();

        let mut to_load = THUMBS_PER_FRAME;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                // Raiz do projeto.
                let root_sel = self.selected.as_deref() == Some("");
                let r =
                    ui.selectable_label(root_sel, RichText::new(format!("🎮 {title}")).strong());
                if r.clicked() {
                    self.selected = Some(String::new());
                }
                accept_object(ui, &r, "", actions);
                r.context_menu(|ui| self.folder_menu(ui, "", mode, actions));

                if !self.filter.trim().is_empty() {
                    self.filtered(ui, tree, actions, &mut to_load);
                    return;
                }
                let mut hidden_under: Option<String> = None;
                for e in tree.entries() {
                    if let Some(h) = &hidden_under {
                        if e.rel.starts_with(&format!("{h}/")) {
                            continue;
                        }
                        hidden_under = None;
                    }
                    // Pastas de saída e de build ficam escondidas no modo Simples.
                    if !mode.is_full()
                        && e.depth == 0
                        && e.is_dir
                        && matches!(e.name.as_str(), "dist" | "target")
                    {
                        hidden_under = Some(e.rel.clone());
                        continue;
                    }
                    self.row(ui, tree, e, mode, actions, &mut to_load);
                    if e.is_dir && !self.expanded.contains(&e.rel) {
                        hidden_under = Some(e.rel.clone());
                    }
                }
                if tree.truncated {
                    ui.weak(format!(
                        "(mostrando só os primeiros {} itens)",
                        crate::files::MAX_ENTRIES
                    ));
                }
            });

        // Atalhos com o mouse sobre o painel.
        if ui.ui_contains_pointer()
            && self.renaming.is_none()
            && let Some(sel) = self.selected.clone().filter(|s| !s.is_empty())
        {
            if ui.input(|i| i.key_pressed(Key::F2)) {
                self.start_rename(&sel);
            }
            if ui.input(|i| i.key_pressed(Key::Delete)) {
                actions.push(Action::AskDelete(sel));
            }
        }
    }

    fn filtered(
        &mut self,
        ui: &mut egui::Ui,
        tree: &FileTree,
        actions: &mut Vec<Action>,
        to_load: &mut usize,
    ) {
        let mut found: Vec<(i32, &FileEntry)> = tree
            .entries()
            .iter()
            .filter(|e| !e.is_dir)
            .filter_map(|e| {
                crate::commands::fuzzy_score(&self.filter, &e.name)
                    .map(|s| s + 20)
                    .max(crate::commands::fuzzy_score(&self.filter, &e.rel))
                    .map(|s| (s, e))
            })
            .collect();
        found.sort_by_key(|f| std::cmp::Reverse(f.0));
        if found.is_empty() {
            ui.weak("Nenhum arquivo com esse nome.");
        }
        for (_, e) in found.into_iter().take(200) {
            ui.horizontal(|ui| {
                self.icon(ui, tree, e, to_load);
                let sel = self.selected.as_deref() == Some(e.rel.as_str());
                let r = ui.selectable_label(sel, &e.rel);
                if r.clicked() {
                    self.selected = Some(e.rel.clone());
                    actions.push(Action::OpenFile {
                        rel: e.rel.clone(),
                        line: None,
                    });
                }
            });
        }
    }

    fn icon(&mut self, ui: &mut egui::Ui, tree: &FileTree, e: &FileEntry, to_load: &mut usize) {
        if e.kind == FileKind::Image && e.rel.to_lowercase().ends_with(".png") {
            let stale = self
                .thumbs
                .get(&e.rel)
                .is_none_or(|(m, _)| *m != e.modified);
            if stale && *to_load > 0 {
                *to_load -= 1;
                let tex = thumbnail(ui.ctx(), &tree.full(&e.rel), &e.rel);
                self.thumbs.insert(e.rel.clone(), (e.modified, tex));
            } else if stale {
                ui.ctx().request_repaint();
            }
            if let Some((_, Some(tex))) = self.thumbs.get(&e.rel) {
                let size = tex.size_vec2();
                let scale = (THUMB / size.x.max(size.y)).min(4.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(THUMB), egui::Sense::hover());
                ui.painter().image(
                    tex.id(),
                    egui::Rect::from_center_size(rect.center(), size * scale),
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                return;
            }
        }
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(THUMB), egui::Sense::hover());
        paint_icon(ui.painter(), rect, e.kind, false);
    }

    fn row(
        &mut self,
        ui: &mut egui::Ui,
        tree: &FileTree,
        e: &FileEntry,
        mode: Mode,
        actions: &mut Vec<Action>,
        to_load: &mut usize,
    ) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            ui.add_space(e.depth as f32 * 14.0);
            if e.is_dir {
                let open = self.expanded.contains(&e.rel);
                let arrow = if open { "⏷" } else { "⏵" };
                if ui
                    .add(egui::Button::new(arrow).frame(false).small())
                    .clicked()
                {
                    self.toggle(&e.rel);
                }
            } else {
                ui.add_space(14.0);
            }
            if e.is_dir {
                let open = self.expanded.contains(&e.rel);
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(THUMB), egui::Sense::hover());
                paint_icon(ui.painter(), rect, FileKind::Folder, open);
            } else {
                self.icon(ui, tree, e, to_load);
            }

            if let Some(rn) = &mut self.renaming
                && rn.rel == e.rel
            {
                let r = ui.add(
                    egui::TextEdit::singleline(&mut rn.name)
                        .desired_width(ui.available_width() - 8.0),
                );
                if rn.focus {
                    rn.focus = false;
                    r.request_focus();
                    // Seleciona o nome sem a extensão.
                    let stem = rn
                        .name
                        .rfind('.')
                        .filter(|i| *i > 0 && !e.is_dir)
                        .unwrap_or(rn.name.len());
                    let stem_chars = rn.name[..stem].chars().count();
                    if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), r.id) {
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::two(
                                egui::text::CCursor::new(0),
                                egui::text::CCursor::new(stem_chars),
                            )));
                        state.store(ui.ctx(), r.id);
                    }
                }
                let enter = ui.input(|i| i.key_pressed(Key::Enter));
                let esc = ui.input(|i| i.key_pressed(Key::Escape));
                if esc {
                    self.renaming = None;
                } else if r.lost_focus() || enter {
                    let rn = self.renaming.take().expect("renomeando");
                    let old = rn.rel.rsplit('/').next().unwrap_or(&rn.rel);
                    if rn.name.trim() != old {
                        actions.push(Action::Rename {
                            rel: rn.rel,
                            new_name: rn.name.trim().to_string(),
                        });
                    }
                }
                return;
            }

            let selected = self.selected.as_deref() == Some(e.rel.as_str());
            let mut text = RichText::new(&e.name);
            if e.is_dir {
                text = text.strong();
            }
            let r = ui.selectable_label(selected, text);
            crate::tutorial::anchor_in(ui, &format!("arquivo:{}", e.rel), r.rect);
            let r = if e.is_dir {
                r
            } else {
                // Arrastar o arquivo para a cena ou para um campo do Inspetor.
                let r = r.interact(egui::Sense::drag());
                r.dnd_set_drag_payload(crate::scene_view::FileDrag(e.rel.clone()));
                if r.dragged()
                    && let Some(p) = ui.ctx().pointer_interact_pos()
                {
                    egui::Area::new(egui::Id::new("arrastando-arquivo"))
                        .order(egui::Order::Tooltip)
                        .fixed_pos(p + Vec2::new(14.0, 8.0))
                        .interactable(false)
                        .show(ui.ctx(), |ui| {
                            egui::Frame::popup(ui.style()).show(ui, |ui| {
                                ui.label(&e.name);
                            });
                        });
                }
                r.on_hover_text(format!(
                    "{}\n{} · {}",
                    e.rel,
                    e.kind.label(),
                    crate::files::human_size(e.size)
                ))
            };
            if e.is_dir {
                accept_object(ui, &r, &e.rel, actions);
            }
            if self.scroll_to.as_deref() == Some(e.rel.as_str()) {
                r.scroll_to_me(Some(egui::Align::Center));
                self.scroll_to = None;
            }
            if r.clicked() {
                self.selected = Some(e.rel.clone());
                if e.is_dir {
                    self.toggle(&e.rel);
                } else {
                    actions.push(Action::OpenFile {
                        rel: e.rel.clone(),
                        line: None,
                    });
                }
            }
            if e.kind == FileKind::Sound
                && ui
                    .add(egui::Button::new("▶").small().frame(false))
                    .on_hover_text("Ouvir")
                    .clicked()
            {
                actions.push(Action::PlaySound(e.rel.clone()));
            }
            r.context_menu(|ui| {
                self.selected = Some(e.rel.clone());
                if e.is_dir {
                    self.folder_menu(ui, &e.rel, mode, actions);
                    ui.separator();
                } else {
                    if ui.button("Abrir").clicked() {
                        actions.push(Action::OpenFile {
                            rel: e.rel.clone(),
                            line: None,
                        });
                        ui.close();
                    }
                    if e.kind == FileKind::Sound && ui.button("▶ Ouvir").clicked() {
                        actions.push(Action::PlaySound(e.rel.clone()));
                        ui.close();
                    }
                    if ui
                        .button("📋 Copiar caminho")
                        .on_hover_text(format!("Copia \"{}\", para usar no código", e.rel))
                        .clicked()
                    {
                        actions.push(Action::CopyText(format!("\"{}\"", e.rel)));
                        ui.close();
                    }
                    if e.kind == FileKind::Model && ui.button("📋 Copiar nn.assets.model").clicked()
                    {
                        actions.push(Action::CopyText(format!("nn.assets.model(\"{}\")", e.rel)));
                        ui.close();
                    }
                    if e.kind == FileKind::Lua && ui.button("📋 Copiar require").clicked() {
                        actions.push(Action::CopyText(format!(
                            "require(\"{}\")",
                            crate::files::module_name(&e.rel)
                        )));
                        ui.close();
                    }
                    if ui.button("Abrir no programa padrão").clicked() {
                        actions.push(Action::OpenExternal(e.rel.clone()));
                        ui.close();
                    }
                    ui.separator();
                }
                if crate::hierarchy::menu_item(ui, "✏ Renomear", "F2").clicked() {
                    self.start_rename(&e.rel);
                    ui.close();
                }
                if crate::hierarchy::menu_item(ui, "🗑 Mandar para a lixeira", "Del").clicked() {
                    actions.push(Action::AskDelete(e.rel.clone()));
                    ui.close();
                }
                if ui.button("Mostrar na pasta do sistema").clicked() {
                    actions.push(Action::Reveal(e.rel.clone()));
                    ui.close();
                }
            });
        });
    }

    fn folder_menu(
        &mut self,
        ui: &mut egui::Ui,
        dir: &str,
        _mode: Mode,
        actions: &mut Vec<Action>,
    ) {
        if ui.button("🖼 Nova cena visual aqui").clicked() {
            actions.push(Action::NewItem {
                kind: NewKind::VisualScene,
                parent: dir.to_string(),
            });
            ui.close();
        }
        if ui.button("📜 Nova cena em código aqui").clicked() {
            actions.push(Action::NewItem {
                kind: NewKind::Scene,
                parent: dir.to_string(),
            });
            ui.close();
        }
        if ui.button("🎲 Novo modelo 3D aqui").clicked() {
            actions.push(Action::NewItem {
                kind: NewKind::Model,
                parent: dir.to_string(),
            });
            ui.close();
        }
        if ui.button("💬 Novo diálogo aqui").clicked() {
            actions.push(Action::NewItem {
                kind: NewKind::Dialogue,
                parent: dir.to_string(),
            });
            ui.close();
        }
        if ui.button("🎨 Nova imagem aqui").clicked() {
            actions.push(Action::NewItem {
                kind: NewKind::Image,
                parent: dir.to_string(),
            });
            ui.close();
        }
        if ui.button("🔧 Novo script aqui").clicked() {
            actions.push(Action::NewItem {
                kind: NewKind::Module,
                parent: dir.to_string(),
            });
            ui.close();
        }
        if ui.button("📁 Nova pasta aqui").clicked() {
            actions.push(Action::NewItem {
                kind: NewKind::Folder,
                parent: dir.to_string(),
            });
            ui.close();
        }
        if ui.button("📥 Importar para cá...").clicked() {
            actions.push(Action::Import {
                dest: Some(dir.to_string()),
            });
            ui.close();
        }
        if dir.is_empty() && ui.button("Mostrar na pasta do sistema").clicked() {
            actions.push(Action::Reveal(String::new()));
            ui.close();
        }
    }

    fn toggle(&mut self, rel: &str) {
        if !self.expanded.remove(rel) {
            self.expanded.insert(rel.to_string());
        }
    }
}

/// Pasta sugerida para cenas novas: a escolhida, ou `cenas`, se existir.
fn scene_dir(tree: &FileTree, chosen: &str) -> String {
    if !chosen.is_empty() || !tree.get("cenas").is_some_and(|e| e.is_dir) {
        chosen.to_string()
    } else {
        "cenas".into()
    }
}

/// Pasta sugerida para imagens novas: a escolhida (se for de assets), ou
/// `assets/imagens`.
pub fn image_dir(tree: &FileTree, chosen: &str) -> String {
    let default = ["assets/imagens", "assets"]
        .iter()
        .any(|d| tree.get(d).is_some_and(|e| e.is_dir));
    if !chosen.is_empty() && (chosen.starts_with("assets") || !default) {
        return chosen.to_string();
    }
    ["assets/imagens", "assets"]
        .iter()
        .find(|d| tree.get(d).is_some_and(|e| e.is_dir))
        .map(|d| d.to_string())
        .unwrap_or_default()
}

/// Pasta sugerida para modelos 3D novos: `assets/modelos` (ou uma pasta
/// dentro dela).
fn model_dir(chosen: &str) -> String {
    if chosen == "assets/modelos" || chosen.starts_with("assets/modelos/") {
        chosen.to_string()
    } else {
        "assets/modelos".into()
    }
}

/// Pasta sugerida para diálogos novos: `dialogos` (ou uma pasta dentro
/// dela, se for a escolhida).
fn dialogue_dir(chosen: &str) -> String {
    if chosen == "dialogos" || chosen.starts_with("dialogos/") {
        chosen.to_string()
    } else {
        "dialogos".into()
    }
}

/// Pasta sugerida para módulos novos: a escolhida, ou `lib`, se existir.
fn module_dir(tree: &FileTree, chosen: &str) -> String {
    if !chosen.is_empty() || !tree.get("lib").is_some_and(|e| e.is_dir) {
        chosen.to_string()
    } else {
        "lib".into()
    }
}

/// Miniatura de uma imagem (pequena, sem suavizar a pixel art).
fn thumbnail(ctx: &egui::Context, path: &std::path::Path, rel: &str) -> Option<TextureHandle> {
    let img = crate::viewers::load_rgba(path).ok()?;
    let max = img.width().max(img.height());
    let img = if max > 64 {
        let s = 64.0 / max as f32;
        image::imageops::resize(
            &img,
            ((img.width() as f32 * s) as u32).max(1),
            ((img.height() as f32 * s) as u32).max(1),
            image::imageops::FilterType::Nearest,
        )
    } else {
        img
    };
    Some(ctx.load_texture(
        format!("miniatura:{rel}"),
        crate::viewers::to_color_image(&img),
        egui::TextureOptions::NEAREST,
    ))
}

/// Objeto da Hierarquia solto numa pasta: vira prefab nela.
fn accept_object(ui: &egui::Ui, r: &egui::Response, dir: &str, actions: &mut Vec<Action>) {
    if r.dnd_hover_payload::<crate::scene_view::ObjectDrag>()
        .is_some()
    {
        ui.painter().rect_stroke(
            r.rect,
            3.0,
            egui::Stroke::new(1.5, ui.visuals().selection.bg_fill),
            egui::StrokeKind::Inside,
        );
        r.clone().on_hover_text("Solte para criar um prefab aqui");
    }
    if let Some(p) = r.dnd_release_payload::<crate::scene_view::ObjectDrag>() {
        actions.push(Action::CreatePrefab {
            id: p.0,
            dir: Some(dir.to_string()),
        });
    }
}

/// Desenha o ícone de um tipo de arquivo (sem depender de fontes de emoji).
pub fn paint_icon(painter: &egui::Painter, rect: egui::Rect, kind: FileKind, open: bool) {
    use egui::{Color32, Pos2, Rect, Stroke, vec2};
    let c = rect.center();
    let rgb = Color32::from_rgb;
    if kind == FileKind::Folder {
        let color = rgb(0xE0, 0xA8, 0x3A);
        let body = Rect::from_center_size(c + vec2(0.0, 1.5), vec2(14.0, 10.0));
        let tab = Rect::from_min_size(body.left_top() - vec2(0.0, 2.5), vec2(6.0, 4.0));
        painter.rect_filled(tab, 1.0, color);
        painter.rect_filled(body, 1.5, color);
        if open {
            let front = Rect::from_min_max(body.left_top() + vec2(1.5, 3.0), body.right_bottom());
            painter.rect_filled(front, 1.5, rgb(0xF5, 0xCB, 0x6A));
        }
        return;
    }
    let (fill, mark) = match kind {
        FileKind::Lua => (rgb(0x4A, 0x86, 0xD8), "lines"),
        FileKind::Scene => (rgb(0xD9, 0x5B, 0x8A), "picture"),
        FileKind::Prefab => (rgb(0x4A, 0x9C, 0xF0), "box"),
        FileKind::Text => (rgb(0x8A, 0x93, 0x9E), "lines"),
        FileKind::Map => (rgb(0x4C, 0xA8, 0x5A), "grid"),
        FileKind::Tileset => (rgb(0x3E, 0x8E, 0x7E), "grid"),
        FileKind::Sound => (rgb(0xA0, 0x6C, 0xD5), "note"),
        FileKind::Image => (rgb(0xD8, 0x7A, 0x4A), "picture"),
        FileKind::Font => (rgb(0x8A, 0x93, 0x9E), "letter"),
        FileKind::Model => (rgb(0x2F, 0xA8, 0xA0), "box"),
        FileKind::ModelSource => (rgb(0x2F, 0x86, 0xC8), "box"),
        FileKind::Dialogue => (rgb(0xE0, 0x9F, 0x3E), "bubble"),
        _ => (rgb(0x6E, 0x76, 0x80), "none"),
    };
    let page = Rect::from_center_size(c, vec2(11.0, 14.0));
    painter.rect_filled(page, 1.5, fill);
    // Canto dobrado.
    let corner = [
        Pos2::new(page.right() - 4.0, page.top()),
        Pos2::new(page.right(), page.top() + 4.0),
        Pos2::new(page.right() - 4.0, page.top() + 4.0),
    ];
    painter.add(egui::Shape::convex_polygon(
        corner.to_vec(),
        Color32::from_white_alpha(110),
        Stroke::NONE,
    ));
    let white = Color32::from_white_alpha(230);
    let s = Stroke::new(1.2, white);
    match mark {
        "lines" => {
            for (i, w) in [6.0, 4.0, 5.5].iter().enumerate() {
                let y = page.top() + 6.0 + i as f32 * 2.6;
                painter.line_segment(
                    [
                        Pos2::new(page.left() + 2.0, y),
                        Pos2::new(page.left() + 2.0 + w, y),
                    ],
                    s,
                );
            }
        }
        "grid" => {
            let g = Rect::from_center_size(page.center() + vec2(0.0, 1.0), vec2(7.0, 7.0));
            painter.rect_stroke(g, 0.0, s, egui::StrokeKind::Middle);
            painter.line_segment(
                [
                    Pos2::new(g.center().x, g.top()),
                    Pos2::new(g.center().x, g.bottom()),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(g.left(), g.center().y),
                    Pos2::new(g.right(), g.center().y),
                ],
                s,
            );
        }
        "note" => {
            let x = page.center().x + 1.0;
            painter.line_segment(
                [
                    Pos2::new(x, page.top() + 4.0),
                    Pos2::new(x, page.bottom() - 4.0),
                ],
                s,
            );
            painter.line_segment(
                [
                    Pos2::new(x, page.top() + 4.0),
                    Pos2::new(x + 2.5, page.top() + 5.5),
                ],
                s,
            );
            painter.circle_filled(Pos2::new(x - 1.5, page.bottom() - 4.0), 1.8, white);
        }
        "picture" => {
            let base = page.bottom() - 3.0;
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(page.left() + 1.5, base),
                    Pos2::new(page.left() + 4.5, page.center().y),
                    Pos2::new(page.left() + 7.5, base),
                ],
                white,
                Stroke::NONE,
            ));
            painter.circle_filled(
                Pos2::new(page.right() - 3.0, page.center().y - 1.0),
                1.3,
                white,
            );
        }
        "box" => {
            // Um cubo: o prefab é um objeto pronto.
            let b = Rect::from_center_size(page.center() + vec2(0.0, 1.0), vec2(6.0, 6.0));
            painter.rect_stroke(b, 0.0, s, egui::StrokeKind::Middle);
            painter.line_segment([b.left_top(), b.left_top() + vec2(1.5, -1.5)], s);
            painter.line_segment([b.right_top(), b.right_top() + vec2(1.5, -1.5)], s);
            painter.line_segment(
                [
                    b.left_top() + vec2(1.5, -1.5),
                    b.right_top() + vec2(1.5, -1.5),
                ],
                s,
            );
        }
        "bubble" => {
            // Um balão de fala.
            let b = Rect::from_center_size(page.center() + vec2(0.0, 0.5), vec2(7.5, 5.0));
            painter.rect_filled(b, 1.5, white);
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(b.left() + 1.5, b.bottom() - 0.5),
                    Pos2::new(b.left() + 4.0, b.bottom() - 0.5),
                    Pos2::new(b.left() + 1.0, b.bottom() + 2.5),
                ],
                white,
                Stroke::NONE,
            ));
        }
        "letter" => {
            painter.text(
                page.center() + vec2(0.0, 1.0),
                egui::Align2::CENTER_CENTER,
                "A",
                egui::FontId::proportional(9.0),
                white,
            );
        }
        _ => {}
    }
}
