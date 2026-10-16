//! O aplicativo NN Studio: tela inicial ou projeto aberto, menus, barra de
//! ferramentas, atalhos, paleta de comandos e preferências.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{self, Key, RichText, ViewportCommand};

use crate::commands::{Command, PaletteItem};
use crate::files::FileKind;
use crate::launcher::{Launcher, LauncherAction};
use crate::project::{self, Env, ProjectWorkspace};
use crate::settings::{self, Mode, Settings, ThemeChoice};
use crate::theme;

/// Ícone do NN Studio (janela, barra de tarefas e tela inicial).
pub const ICON_PNG: &[u8] = include_bytes!("../assets/icone.png");

/// Como abrir o NN Studio.
#[derive(Debug, Clone, Default)]
pub struct StudioOptions {
    /// Arquivo de preferências. `None`: não grava (testes).
    pub settings_path: Option<PathBuf>,
    /// Projeto para abrir direto.
    pub open: Option<PathBuf>,
    /// Executável que roda os jogos (`--jogar`). Padrão: o próprio programa.
    pub exe: Option<PathBuf>,
}

enum Screen {
    Launcher,
    Project(Box<ProjectWorkspace>),
}

/// O que fazer depois de perguntar sobre alterações não salvas.
#[derive(Debug, Clone, PartialEq)]
enum Pending {
    Quit,
    CloseProject,
    Open(PathBuf),
    PickAndOpen,
    NewProject,
}

#[derive(Debug, Default)]
struct PaletteState {
    query: String,
    selected: usize,
    files_only: bool,
    focus: bool,
}

pub struct StudioApp {
    pub settings: Settings,
    settings_path: Option<PathBuf>,
    saved_snapshot: String,
    screen: Screen,
    launcher: Launcher,
    palette: Option<PaletteState>,
    audio: Option<nn_audio::Audio>,
    exe: PathBuf,
    pending: Option<Pending>,
    allow_close: bool,
    about: bool,
    shortcuts: bool,
    logo: Option<egui::TextureHandle>,
    window_title: String,
    /// GPU do editor (as cenas visuais são desenhadas com ela).
    render_state: Option<eframe::egui_wgpu::RenderState>,
    /// Tutorial em andamento.
    pub tutorial: Option<crate::tutorial::TutorialRun>,
}

impl std::fmt::Debug for StudioApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StudioApp").finish_non_exhaustive()
    }
}

/// Abre a janela do NN Studio.
pub fn run(options: StudioOptions) -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("NN Studio")
        .with_app_id("nn-studio")
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([720.0, 480.0])
        .with_drag_and_drop(true);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(ICON_PNG) {
        viewport = viewport.with_icon(Arc::new(icon));
    }
    let native = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "NN Studio",
        native,
        Box::new(move |cc| {
            let mut app = StudioApp::new(&cc.egui_ctx, options);
            app.set_render_state(cc.wgpu_render_state.clone());
            Ok(Box::new(app))
        }),
    )
}

impl StudioApp {
    /// GPU do editor, para desenhar as cenas visuais (sem ela, a aba da
    /// cena mostra só os contornos).
    pub fn set_render_state(&mut self, state: Option<eframe::egui_wgpu::RenderState>) {
        self.render_state = state;
    }

    pub fn new(ctx: &egui::Context, options: StudioOptions) -> Self {
        let settings = options
            .settings_path
            .as_deref()
            .map(Settings::load)
            .unwrap_or_default();
        let exe = options
            .exe
            .clone()
            .or_else(|| std::env::current_exe().ok())
            .unwrap_or_else(|| PathBuf::from("nn-studio"));
        let logo = image::load_from_memory(ICON_PNG).ok().map(|img| {
            ctx.load_texture(
                "logo",
                crate::viewers::to_color_image(&img.to_rgba8()),
                egui::TextureOptions::LINEAR,
            )
        });
        let mut app = Self {
            saved_snapshot: snapshot(&settings),
            settings,
            settings_path: options.settings_path,
            screen: Screen::Launcher,
            launcher: Launcher::default(),
            palette: None,
            audio: None,
            exe,
            pending: None,
            allow_close: false,
            about: false,
            shortcuts: false,
            logo,
            window_title: String::new(),
            render_state: None,
            tutorial: None,
        };
        theme::apply(ctx, app.settings.theme, app.settings.zoom);
        if let Some(dir) = options.open {
            app.open_project(&dir);
        }
        app
    }

    /// O projeto aberto, se houver.
    pub fn project(&self) -> Option<&ProjectWorkspace> {
        match &self.screen {
            Screen::Project(p) => Some(p),
            Screen::Launcher => None,
        }
    }

    pub fn project_mut(&mut self) -> Option<&mut ProjectWorkspace> {
        match &mut self.screen {
            Screen::Project(p) => Some(p),
            Screen::Launcher => None,
        }
    }

    pub fn launcher_error(&self) -> Option<&str> {
        self.launcher.error.as_deref()
    }

    /// Grava as preferências (e o layout do projeto aberto).
    pub fn persist(&mut self) {
        if let Screen::Project(ws) = &self.screen {
            self.settings.set_layout(&ws.root, ws.dock.clone());
        }
        if let Some(path) = &self.settings_path
            && let Err(e) = self.settings.save(path)
        {
            log::warn!("não foi possível gravar as preferências: {e}");
        }
        self.saved_snapshot = snapshot(&self.settings);
    }

    pub fn open_project(&mut self, dir: &Path) {
        if let Err(e) = project::looks_like_project(dir) {
            self.launcher.error = Some(e);
            return;
        }
        self.close_project();
        let dir = settings::absolute(dir);
        let layout = self.settings.layout(&dir).cloned();
        let ws = ProjectWorkspace::open(dir.clone(), layout);
        self.settings
            .touch_recent(&dir, &ws.title, settings::now_secs());
        self.launcher.error = None;
        self.screen = Screen::Project(Box::new(ws));
        self.persist();
    }

    /// Cria o projeto do tutorial (numa pasta nova) e começa.
    pub fn start_tutorial(&mut self, id: &str) {
        let Some(tutorial) = crate::tutorial::find(id) else {
            return;
        };
        let parent = self.settings.default_projects_dir();
        let dir = crate::tutorial::project_dir(&parent, tutorial);
        let result = std::fs::create_dir_all(&parent)
            .map_err(|e| e.to_string())
            .and_then(|_| {
                nn_run::create_project(&nn_run::NewOptions {
                    dir: dir.clone(),
                    title: Some(tutorial.project.to_string()),
                    template: tutorial.template,
                })
            });
        match result {
            Ok(report) => {
                self.open_project(&report.dir);
                if self.project().is_some() {
                    self.tutorial = Some(crate::tutorial::TutorialRun::new(tutorial));
                }
            }
            Err(e) => {
                self.launcher.error =
                    Some(format!("Não foi possível criar o projeto do tutorial: {e}"))
            }
        }
    }

    pub fn close_project(&mut self) {
        if let Screen::Project(ws) = &mut self.screen {
            ws.stop();
            self.settings.set_layout(&ws.root, ws.dock.clone());
            self.screen = Screen::Launcher;
            self.persist();
        }
    }

    fn dirty(&self) -> bool {
        self.project().is_some_and(|p| !p.dirty_files().is_empty())
    }

    /// Faz `p` agora ou, havendo alterações não salvas, pergunta antes.
    fn request(&mut self, ctx: &egui::Context, p: Pending) {
        if self.dirty() {
            self.pending = Some(p);
        } else {
            self.proceed(ctx, p);
        }
    }

    fn proceed(&mut self, ctx: &egui::Context, p: Pending) {
        match p {
            Pending::Quit => {
                self.allow_close = true;
                if let Some(ws) = self.project_mut() {
                    ws.stop();
                }
                self.persist();
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            Pending::CloseProject => {
                self.close_project();
                self.tutorial = None;
            }
            Pending::Open(dir) => self.open_project(&dir),
            Pending::PickAndOpen => {
                let mut dialog =
                    rfd::FileDialog::new().set_title("Abrir projeto (escolha a pasta do jogo)");
                if let Some(r) = self.settings.recent.first()
                    && let Some(parent) = r.path.parent()
                {
                    dialog = dialog.set_directory(parent);
                }
                if let Some(dir) = dialog.pick_folder() {
                    self.open_project(&dir);
                }
            }
            Pending::NewProject => {
                self.close_project();
                self.launcher.open_new_project(&self.settings);
            }
        }
    }

    /// Comandos do aplicativo (os do projeto ficam com o projeto).
    fn run_command(&mut self, ctx: &egui::Context, cmd: Command) {
        if let Screen::Project(ws) = &mut self.screen {
            let mut env = Env {
                settings: &mut self.settings,
                audio: &mut self.audio,
                exe: &self.exe,
                render_state: self.render_state.as_ref(),
            };
            if ws.run(cmd, ctx, &mut env) {
                return;
            }
        } else if cmd.needs_project() {
            return;
        }
        match cmd {
            Command::NewProject => self.request(ctx, Pending::NewProject),
            Command::OpenProject => self.request(ctx, Pending::PickAndOpen),
            Command::CloseProject => self.request(ctx, Pending::CloseProject),
            Command::ToggleTheme => {
                self.settings.theme = match self.settings.theme {
                    ThemeChoice::Escuro => ThemeChoice::Claro,
                    ThemeChoice::Claro => ThemeChoice::Escuro,
                };
            }
            Command::ToggleMode => {
                self.settings.mode = match self.settings.mode {
                    Mode::Simples => Mode::Completo,
                    Mode::Completo => Mode::Simples,
                };
            }
            Command::ZoomIn => self.settings.zoom = theme::zoom_step(self.settings.zoom, true),
            Command::ZoomOut => self.settings.zoom = theme::zoom_step(self.settings.zoom, false),
            Command::ZoomReset => self.settings.zoom = 1.0,
            Command::Palette => {
                self.palette = Some(PaletteState {
                    focus: true,
                    ..Default::default()
                })
            }
            Command::QuickOpen => {
                self.palette = Some(PaletteState {
                    focus: true,
                    files_only: true,
                    ..Default::default()
                })
            }
            Command::Docs => ctx.open_url(egui::OpenUrl::new_tab(project::DOCS_URL)),
            Command::Shortcuts => self.shortcuts = true,
            Command::About => self.about = true,
            Command::Tutorials => {
                // Os tutoriais criam um projeto novo: volta ao início.
                self.launcher.show_tutorials();
                self.request(ctx, Pending::CloseProject);
            }
            Command::Quit => self.request(ctx, Pending::Quit),
            _ => {}
        }
    }

    /// Atalhos de teclado → comandos.
    fn shortcuts_pressed(&self, ctx: &egui::Context) -> Vec<Command> {
        if self.palette.is_some() || self.pending.is_some() {
            return Vec::new();
        }
        let mut cmds: Vec<(Command, egui::KeyboardShortcut)> = Command::ALL
            .iter()
            .filter_map(|c| c.shortcut().map(|s| (*c, s)))
            .collect();
        // Atalhos com mais teclas primeiro (Ctrl+Shift+S antes de Ctrl+S).
        let weight =
            |m: egui::Modifiers| m.shift as u8 + m.alt as u8 + m.command as u8 + m.ctrl as u8;
        cmds.sort_by_key(|(_, s)| std::cmp::Reverse(weight(s.modifiers)));
        // Ctrl+Z / Ctrl+Y num campo de texto ficam com o campo.
        let typing = ctx.text_edit_focused();
        let mut out = Vec::new();
        ctx.input_mut(|i| {
            if !typing
                && i.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::Z,
                ))
            {
                out.push(Command::Redo);
            }
            for (c, s) in &cmds {
                if typing && matches!(c, Command::Undo | Command::Redo) {
                    continue;
                }
                if i.consume_shortcut(s) {
                    out.push(*c);
                }
            }
        });
        out
    }

    fn command_button(&self, ui: &mut egui::Ui, cmd: Command) -> bool {
        let enabled = !cmd.needs_project() || self.project().is_some();
        let mut b = egui::Button::new(cmd.title());
        if let Some(s) = cmd.shortcut() {
            b = b.shortcut_text(ui.ctx().format_shortcut(&s));
        }
        let clicked = ui.add_enabled(enabled, b).clicked();
        if clicked {
            ui.close();
        }
        clicked
    }

    fn menu_bar(&mut self, ui: &mut egui::Ui) -> Vec<Command> {
        let mut out = Vec::new();
        let full = self.settings.mode.is_full();
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("Arquivo", |ui| {
                for c in [Command::NewProject, Command::OpenProject] {
                    if self.command_button(ui, c) {
                        out.push(c);
                    }
                }
                ui.menu_button("Projetos recentes", |ui| {
                    if self.settings.recent.is_empty() {
                        ui.weak("Nenhum");
                    }
                    for r in self.settings.recent.clone().iter().take(10) {
                        if ui
                            .button(format!(
                                "{}  —  {}",
                                r.title,
                                settings::display_path(&r.path)
                            ))
                            .clicked()
                        {
                            self.pending_open(ui.ctx(), r.path.clone());
                            ui.close();
                        }
                    }
                });
                if self.command_button(ui, Command::CloseProject) {
                    out.push(Command::CloseProject);
                }
                ui.separator();
                for c in [
                    Command::NewScene,
                    Command::NewCodeScene,
                    Command::NewScript,
                    Command::NewFolder,
                    Command::ImportFiles,
                ] {
                    if self.command_button(ui, c) {
                        out.push(c);
                    }
                }
                ui.separator();
                for c in [Command::Save, Command::SaveAll] {
                    if self.command_button(ui, c) {
                        out.push(c);
                    }
                }
                ui.separator();
                if self.command_button(ui, Command::Quit) {
                    out.push(Command::Quit);
                }
            });
            ui.menu_button("Editar", |ui| {
                for c in [Command::Undo, Command::Redo] {
                    if self.command_button(ui, c) {
                        out.push(c);
                    }
                }
                ui.separator();
                for c in [Command::Find, Command::Replace, Command::GoToLine] {
                    if self.command_button(ui, c) {
                        out.push(c);
                    }
                }
                ui.separator();
                if self.command_button(ui, Command::CloseTab) {
                    out.push(Command::CloseTab);
                }
                if full && self.command_button(ui, Command::OpenExternal) {
                    out.push(Command::OpenExternal);
                }
                ui.separator();
                if self.command_button(ui, Command::ProjectSettings) {
                    out.push(Command::ProjectSettings);
                }
            });
            ui.menu_button("Jogo", |ui| {
                for c in [Command::Play, Command::TestHere, Command::Stop] {
                    if self.command_button(ui, c) {
                        out.push(c);
                    }
                }
                ui.separator();
                if self.command_button(ui, Command::Export) {
                    out.push(Command::Export);
                }
                if full && self.command_button(ui, Command::RevealProject) {
                    out.push(Command::RevealProject);
                }
            });
            ui.menu_button("Exibir", |ui| {
                for c in [
                    Command::ShowHome,
                    Command::ShowFiles,
                    Command::ShowHierarchy,
                    Command::ShowInspector,
                    Command::ShowEvents,
                    Command::ShowConsole,
                ] {
                    if self.command_button(ui, c) {
                        out.push(c);
                    }
                }
                if self.command_button(ui, Command::ClearConsole) {
                    out.push(Command::ClearConsole);
                }
                ui.separator();
                if self.command_button(ui, Command::Palette) {
                    out.push(Command::Palette);
                }
                if self.command_button(ui, Command::QuickOpen) {
                    out.push(Command::QuickOpen);
                }
                ui.separator();
                ui.label(RichText::new("Tema").weak());
                ui.radio_value(&mut self.settings.theme, ThemeChoice::Escuro, "🌙 Escuro");
                ui.radio_value(&mut self.settings.theme, ThemeChoice::Claro, "☀ Claro");
                ui.separator();
                ui.label(RichText::new("Modo").weak());
                ui.radio_value(
                    &mut self.settings.mode,
                    Mode::Simples,
                    "Simples (o essencial)",
                );
                ui.radio_value(
                    &mut self.settings.mode,
                    Mode::Completo,
                    "Completo (todas as opções)",
                );
                ui.separator();
                for c in [Command::ZoomIn, Command::ZoomOut, Command::ZoomReset] {
                    if self.command_button(ui, c) {
                        out.push(c);
                    }
                }
                ui.separator();
                if self.command_button(ui, Command::ResetLayout) {
                    out.push(Command::ResetLayout);
                }
            });
            ui.menu_button("Ajuda", |ui| {
                for c in [
                    Command::Tutorials,
                    Command::Docs,
                    Command::Shortcuts,
                    Command::About,
                ] {
                    if self.command_button(ui, c) {
                        out.push(c);
                    }
                }
            });
        });
        out
    }

    fn pending_open(&mut self, ctx: &egui::Context, dir: PathBuf) {
        self.request(ctx, Pending::Open(dir));
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) -> Vec<Command> {
        let mut out = Vec::new();
        let Some(ws) = self.project() else {
            return out;
        };
        let playing = ws.is_playing();
        let title = ws.title.clone();
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            let play = if playing {
                "⟳ Reiniciar"
            } else {
                "▶ Jogar"
            };
            if ui
                .add(theme::big_button(play, theme::PLAY))
                .on_hover_text(if playing {
                    "Fecha e abre o jogo de novo (F5)"
                } else {
                    "Roda o jogo na janela dele (F5)"
                })
                .clicked()
            {
                out.push(Command::Play);
            }
            if ui
                .add_enabled(
                    playing,
                    theme::big_button("⏹ Parar", egui::Color32::from_rgb(0xB0, 0x40, 0x40)),
                )
                .on_hover_text("Shift+F5")
                .clicked()
            {
                out.push(Command::Stop);
            }
            ui.add_space(8.0);
            ui.label(RichText::new(&title).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(4.0);
                if ui
                    .button("🔎 Comandos")
                    .on_hover_text("Paleta de comandos (Ctrl+Shift+P)")
                    .clicked()
                {
                    out.push(Command::Palette);
                }
                ui.selectable_value(&mut self.settings.mode, Mode::Completo, "Completo")
                    .on_hover_text("Mostra todas as opções");
                ui.selectable_value(&mut self.settings.mode, Mode::Simples, "Simples")
                    .on_hover_text("Mostra só o essencial");
            });
        });
        out
    }

    fn palette_ui(&mut self, ctx: &egui::Context) -> Option<Command> {
        let state = self.palette.as_mut()?;
        let mut items: Vec<PaletteItem> = Vec::new();
        let has_project = matches!(self.screen, Screen::Project(_));
        if !state.files_only {
            for c in Command::ALL {
                if *c == Command::Palette
                    || (c.needs_project() && !has_project)
                    || (c.advanced() && !self.settings.mode.is_full())
                {
                    continue;
                }
                items.push(PaletteItem::Command(*c));
            }
        }
        if let Screen::Project(ws) = &self.screen {
            for e in ws.files.entries().iter().filter(|e| !e.is_dir) {
                items.push(PaletteItem::File(e.rel.clone()));
            }
        }
        let results = crate::commands::search(&state.query, &items);
        let up = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowUp));
        let down = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowDown));
        let enter = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter));
        let esc = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape));
        if down {
            state.selected = (state.selected + 1).min(results.len().saturating_sub(1));
        }
        if up {
            state.selected = state.selected.saturating_sub(1);
        }
        let mut chosen: Option<PaletteItem> = None;
        if enter {
            chosen = results.get(state.selected).cloned();
        }
        let id = egui::Id::new("paleta");
        let area =
            egui::Modal::default_area(id).anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 70.0));
        let response = egui::Modal::new(id).area(area).show(ctx, |ui| {
            ui.set_width(520.0);
            let hint = if state.files_only {
                "Nome do arquivo..."
            } else {
                "Digite um comando ou o nome de um arquivo..."
            };
            let r = ui.add(
                egui::TextEdit::singleline(&mut state.query)
                    .hint_text(hint)
                    .desired_width(f32::INFINITY),
            );
            if state.focus {
                r.request_focus();
                state.focus = false;
            }
            if r.changed() {
                state.selected = 0;
            }
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .max_height(360.0)
                .show(ui, |ui| {
                    if results.is_empty() {
                        ui.weak("Nada encontrado.");
                    }
                    for (i, item) in results.iter().take(60).enumerate() {
                        let selected = i == state.selected;
                        let (text, shortcut) = match item {
                            PaletteItem::Command(c) => (
                                c.title().to_string(),
                                c.shortcut()
                                    .map(|s| ctx.format_shortcut(&s))
                                    .unwrap_or_default(),
                            ),
                            PaletteItem::File(rel) => (
                                rel.clone(),
                                FileKind::of(Path::new(rel)).label().to_string(),
                            ),
                        };
                        let r = ui.add(
                            egui::Button::selectable(selected, text)
                                .shortcut_text(shortcut)
                                .min_size(egui::vec2(ui.available_width(), 24.0)),
                        );
                        if selected && (up || down) {
                            r.scroll_to_me(None);
                        }
                        if r.clicked() {
                            chosen = Some(item.clone());
                        }
                    }
                });
        });
        let mut close = esc || response.should_close();
        let mut cmd = None;
        if let Some(item) = chosen {
            close = true;
            match item {
                PaletteItem::Command(c) => cmd = Some(c),
                PaletteItem::File(rel) => {
                    if let Some(ws) = self.project_mut() {
                        ws.open_file(&rel, None);
                    }
                }
            }
        }
        if close {
            self.palette = None;
        }
        cmd
    }

    fn pending_dialog(&mut self, ctx: &egui::Context) {
        let Some(p) = self.pending.clone() else {
            return;
        };
        let files = self.project().map(|w| w.dirty_files()).unwrap_or_default();
        let mut choice: Option<bool> = None;
        let mut cancel = false;
        let response = egui::Modal::new(egui::Id::new("salvar-antes")).show(ctx, |ui| {
            ui.set_width(400.0);
            ui.heading("Salvar as alterações?");
            ui.add_space(6.0);
            ui.label(if files.len() == 1 {
                "Este arquivo tem alterações não salvas:".to_string()
            } else {
                format!("{} arquivos têm alterações não salvas:", files.len())
            });
            for f in &files {
                ui.label(format!("  • {f}"));
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add(egui::Button::new("Salvar tudo").fill(theme::ACCENT.gamma_multiply(0.75)))
                    .clicked()
                {
                    choice = Some(true);
                }
                if ui.button("Não salvar").clicked() {
                    choice = Some(false);
                }
                if ui.button("Cancelar").clicked() {
                    cancel = true;
                }
            });
        });
        if response.should_close() {
            cancel = true;
        }
        if cancel {
            self.pending = None;
            return;
        }
        if let Some(save) = choice {
            if save
                && let Some(ws) = self.project_mut()
                && ws.save_all().is_err()
            {
                // Não fecha: o erro está no console.
                self.pending = None;
                return;
            }
            if !save && let Some(ws) = self.project_mut() {
                // Descarta: os documentos voltam ao que está no disco.
                ws.discard_changes();
            }
            self.pending = None;
            self.proceed(ctx, p);
        }
    }

    fn info_dialogs(&mut self, ctx: &egui::Context) {
        if self.about {
            let r = egui::Modal::new(egui::Id::new("sobre")).show(ctx, |ui| {
                ui.set_width(360.0);
                ui.horizontal(|ui| {
                    if let Some(tex) = &self.logo {
                        ui.add(egui::Image::new(tex).fit_to_exact_size(egui::vec2(48.0, 48.0)));
                    }
                    ui.vertical(|ui| {
                        ui.heading("NN Studio");
                        ui.weak(format!("versão {}", env!("CARGO_PKG_VERSION")));
                    });
                });
                ui.add_space(6.0);
                ui.label("O editor da nn-engine, para criar jogos 2D em Lua.");
                ui.hyperlink_to(
                    "github.com/nereuneto/nn-engine",
                    "https://github.com/nereuneto/nn-engine",
                );
                ui.add_space(6.0);
                ui.button("Fechar").clicked()
            });
            if r.inner || r.should_close() {
                self.about = false;
            }
        }
        if self.shortcuts {
            let r = egui::Modal::new(egui::Id::new("atalhos")).show(ctx, |ui| {
                ui.set_width(420.0);
                ui.heading("Atalhos do teclado");
                ui.add_space(6.0);
                egui::Grid::new("grade-atalhos")
                    .striped(true)
                    .num_columns(2)
                    .show(ui, |ui| {
                        for c in Command::ALL {
                            if let Some(s) = c.shortcut() {
                                ui.label(c.title());
                                ui.monospace(ctx.format_shortcut(&s));
                                ui.end_row();
                            }
                        }
                        for (what, keys) in [
                            ("Comentar/descomentar linhas", "Ctrl+/"),
                            ("Recuar / desfazer recuo", "Tab / Shift+Tab"),
                            ("Desfazer / refazer", "Ctrl+Z / Ctrl+Y"),
                            ("Renomear arquivo (no painel)", "F2"),
                            ("Mandar para a lixeira (no painel)", "Del"),
                        ] {
                            ui.label(what);
                            ui.monospace(keys);
                            ui.end_row();
                        }
                    });
                ui.add_space(6.0);
                ui.button("Fechar").clicked()
            });
            if r.inner || r.should_close() {
                self.shortcuts = false;
            }
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let title = match &self.screen {
            Screen::Launcher => "NN Studio".to_string(),
            Screen::Project(ws) => {
                let dirty = if ws.dirty_files().is_empty() {
                    ""
                } else {
                    "● "
                };
                format!("{dirty}{} — NN Studio", ws.title)
            }
        };
        if title != self.window_title {
            ctx.send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }

    /// Um quadro da interface (separado de `eframe::App` para os testes).
    pub fn frame(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        theme::apply(&ctx, self.settings.theme, self.settings.zoom);

        // Fechar a janela com alterações não salvas: pergunta antes.
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            if self.dirty() {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                self.pending = Some(Pending::Quit);
            } else {
                if let Some(ws) = self.project_mut() {
                    ws.stop();
                }
                self.persist();
            }
        }

        crate::tutorial::begin_frame(&ctx);
        let mut commands = self.shortcuts_pressed(&ctx);
        match &self.screen {
            Screen::Launcher => {
                // Pasta arrastada para a janela: abre como projeto.
                let dropped = ctx.input(|i| {
                    i.raw
                        .dropped_files
                        .iter()
                        .map(|f| f.path().to_path_buf())
                        .find(|p| p.is_dir())
                });
                if let Some(dir) = dropped {
                    self.open_project(&dir);
                }
                let logo = self.logo.clone();
                if let Some(a) = self.launcher.ui(ui, &mut self.settings, logo.as_ref()) {
                    match a {
                        LauncherAction::Open(dir) => self.open_project(&dir),
                        LauncherAction::Command(c) => commands.push(c),
                        LauncherAction::StartTutorial(id) => self.start_tutorial(id),
                    }
                }
            }
            Screen::Project(_) => {
                egui::Panel::top("menu-e-ferramentas").show(ui, |ui| {
                    commands.extend(self.menu_bar(ui));
                    ui.add_space(2.0);
                    commands.extend(self.toolbar(ui));
                    ui.add_space(4.0);
                });
                egui::CentralPanel::default()
                    .frame(egui::Frame::central_panel(ui.style()).inner_margin(2.0))
                    .show(ui, |ui| {
                        if let Screen::Project(ws) = &mut self.screen {
                            let mut env = Env {
                                settings: &mut self.settings,
                                audio: &mut self.audio,
                                exe: &self.exe,
                                render_state: self.render_state.as_ref(),
                            };
                            commands.extend(ws.ui(ui, &mut env));
                        }
                    });
            }
        }
        // Tutorial: confere o projeto e mostra o passo.
        if let (Some(run), Screen::Project(ws)) = (&mut self.tutorial, &mut self.screen) {
            run.update(ws);
            if run.ui(&ctx) == Some(crate::tutorial::TutorialAction::Close) {
                self.tutorial = None;
            }
        }
        if let Some(c) = self.palette_ui(&ctx) {
            commands.push(c);
        }
        for c in commands {
            self.run_command(&ctx, c);
        }
        self.pending_dialog(&ctx);
        self.info_dialogs(&ctx);
        self.update_title(&ctx);

        // Preferências mudaram (tema, modo, recentes...): grava.
        if snapshot(&self.settings) != self.saved_snapshot {
            self.persist();
        }
    }
}

/// Resumo das preferências para saber quando gravar.
fn snapshot(s: &Settings) -> String {
    format!(
        "{:?}{:?}{}{}{:?}{}",
        s.theme,
        s.mode,
        s.zoom,
        s.clear_console_on_play,
        s.projects_dir,
        s.recent
            .iter()
            .map(|r| r.path.to_string_lossy())
            .collect::<Vec<_>>()
            .join("|")
    )
}

impl eframe::App for StudioApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frame(ui);
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.persist();
    }

    fn on_exit(&mut self) {
        if let Some(ws) = self.project_mut() {
            ws.stop();
        }
        self.persist();
    }
}
