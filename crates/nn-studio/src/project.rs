//! Um projeto aberto: painéis encaixáveis (Arquivos, Console, abas de
//! código, imagem, som e mapa), o jogo rodando, diálogos e comandos.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;

use eframe::egui::{self, Color32, RichText};
use egui_dock::tab_viewer::OnCloseResponse;
use egui_dock::{DockArea, DockState, NodeIndex, TabViewer};
use serde::{Deserialize, Serialize};

use crate::code::{CodeAction, CodeDoc};
use crate::commands::Command;
use crate::config_lua::{self, ConfigValues};
use crate::console::{Console, ConsoleAction};
use crate::file_panel::FilePanel;
use crate::files::{self, FileKind, FileTree};
use crate::game::GameProcess;
use crate::hierarchy::HierarchyPanel;
use crate::inspector::{Inspector, InspectorAction};
use crate::scene_render::{SceneRenderer, Sizes};
use crate::scene_view::{SceneAction, SceneDoc, SceneEnv};
use crate::settings::{Mode, Settings};
use crate::theme;
use crate::viewers::{ImageView, MapView, SoundView, ViewerAction};

/// Uma aba.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tab {
    Home,
    Files,
    Console,
    Settings,
    Code(String),
    Image(String),
    Sound(String),
    Map(String),
    /// Cena visual (`.cena`).
    Scene(String),
    Hierarchy,
    Inspector,
}

impl Tab {
    /// A aba certa para abrir um arquivo (`None`: abrir fora).
    pub fn for_file(rel: &str) -> Option<Tab> {
        let lower = rel.to_lowercase();
        Some(match FileKind::of(Path::new(rel)) {
            FileKind::Lua | FileKind::Text | FileKind::Tileset => Tab::Code(rel.into()),
            FileKind::Scene => Tab::Scene(rel.into()),
            FileKind::Map if lower.ends_with(".tmj") => Tab::Map(rel.into()),
            FileKind::Map => Tab::Code(rel.into()),
            FileKind::Image => Tab::Image(rel.into()),
            FileKind::Sound => Tab::Sound(rel.into()),
            FileKind::Folder | FileKind::Font | FileKind::Other => return None,
        })
    }

    pub fn rel(&self) -> Option<&str> {
        match self {
            Tab::Code(r) | Tab::Image(r) | Tab::Sound(r) | Tab::Map(r) | Tab::Scene(r) => Some(r),
            _ => None,
        }
    }

    fn set_rel(&mut self, new: String) {
        match self {
            Tab::Code(r) | Tab::Image(r) | Tab::Sound(r) | Tab::Map(r) | Tab::Scene(r) => *r = new,
            _ => {}
        }
    }

    /// Abas que ficam na área central (documentos).
    pub fn is_document(&self) -> bool {
        !matches!(
            self,
            Tab::Files | Tab::Console | Tab::Hierarchy | Tab::Inspector
        )
    }
}

/// Layout inicial: Arquivos e Hierarquia à esquerda, Início no centro,
/// Inspetor à direita e Console embaixo.
pub fn default_layout() -> DockState<Tab> {
    let mut state = DockState::new(vec![Tab::Home]);
    let tree = state.main_surface_mut();
    let [center, files] = tree.split_left(NodeIndex::root(), 0.2, vec![Tab::Files]);
    let [_files, _hierarchy] = tree.split_below(files, 0.5, vec![Tab::Hierarchy]);
    let [center, _inspector] = tree.split_right(center, 0.72, vec![Tab::Inspector]);
    let [_center, _console] = tree.split_below(center, 0.70, vec![Tab::Console]);
    state
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewKind {
    Scene,
    VisualScene,
    Module,
    Folder,
}

impl NewKind {
    fn title(self) -> &'static str {
        match self {
            NewKind::Scene => "Nova cena em código",
            NewKind::VisualScene => "Nova cena visual",
            NewKind::Module => "Novo script (módulo)",
            NewKind::Folder => "Nova pasta",
        }
    }

    fn default_name(self) -> &'static str {
        match self {
            NewKind::Scene | NewKind::VisualScene => "nova_cena",
            NewKind::Module => "util",
            NewKind::Folder => "nova_pasta",
        }
    }
}

/// O que os painéis pedem; tratado depois de desenhar.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Command(Command),
    OpenFile {
        rel: String,
        line: Option<usize>,
    },
    OpenLocation {
        path: String,
        line: usize,
    },
    PlaySound(String),
    NewItem {
        kind: NewKind,
        parent: String,
    },
    Import {
        dest: Option<String>,
    },
    ImportPaths {
        paths: Vec<PathBuf>,
        dest: Option<String>,
    },
    Rename {
        rel: String,
        new_name: String,
    },
    AskDelete(String),
    AskCloseTab(String),
    Reveal(String),
    OpenExternal(String),
    CopyText(String),
    SaveDoc(String),
    ReloadDoc(String),
    /// Rodar o jogo começando nesta cena.
    TestScene(String),
    /// Criar o script da cena visual.
    CreateSceneScript(String),
    /// Abrir a cena visual como texto.
    OpenAsText(String),
}

enum Dialog {
    NewItem {
        kind: NewKind,
        parent: String,
        name: String,
        error: Option<String>,
        focus: bool,
    },
    ConfirmDelete(String),
    CloseTab(String),
    Export(ExportDialog),
}

struct ExportDialog {
    output: String,
    name: String,
    job: Option<Receiver<Result<nn_run::ExportReport, String>>>,
    result: Option<Result<nn_run::ExportReport, String>>,
}

/// Formulário das configurações do projeto (`config.lua`).
#[derive(Debug, Default)]
struct SettingsForm {
    original: Option<ConfigValues>,
    values: ConfigValues,
    error: Option<String>,
    loaded: bool,
}

/// O que o espaço de trabalho precisa do aplicativo.
pub struct Env<'a> {
    pub settings: &'a mut Settings,
    pub audio: &'a mut Option<nn_audio::Audio>,
    /// Executável usado para rodar o jogo (o próprio NN Studio).
    pub exe: &'a Path,
    /// GPU do editor (para desenhar as cenas com o renderer da engine).
    pub render_state: Option<&'a eframe::egui_wgpu::RenderState>,
}

impl std::fmt::Debug for Env<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Env")
            .field("exe", &self.exe)
            .field("gpu", &self.render_state.is_some())
            .finish_non_exhaustive()
    }
}

pub struct ProjectWorkspace {
    pub root: PathBuf,
    pub title: String,
    pub dock: DockState<Tab>,
    pub files: FileTree,
    pub file_panel: FilePanel,
    pub docs: HashMap<String, CodeDoc>,
    images: HashMap<String, ImageView>,
    sounds: HashMap<String, SoundView>,
    maps: HashMap<String, MapView>,
    pub console: Console,
    pub game: Option<GameProcess>,
    form: SettingsForm,
    lua: mlua::Lua,
    dialog: Option<Dialog>,
    actions: Vec<Action>,
    /// Último editor de código com o foco (destino de Salvar, Buscar...).
    focused_code: Option<String>,
    console_drawn: u64,
    frame: u64,
    /// Cenas visuais abertas.
    pub scenes: HashMap<String, SceneDoc>,
    /// Cena que a Hierarquia e o Inspetor mostram.
    pub active_scene: Option<String>,
    sizes: Sizes,
    scene_renderer: Option<SceneRenderer>,
    hierarchy: HierarchyPanel,
    inspector: Inspector,
    /// Cena que o jogo aberto está mostrando (para o ajuste ao vivo).
    pub game_scene: Option<String>,
    /// Valores do config.lua (tamanho da tela e cor de fundo).
    config: ConfigValues,
    started: std::time::Instant,
}

impl std::fmt::Debug for ProjectWorkspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectWorkspace")
            .field("root", &self.root)
            .field("title", &self.title)
            .finish_non_exhaustive()
    }
}

/// Título do projeto: o do `config.lua` ou o nome da pasta.
pub fn project_title(root: &Path) -> String {
    let folder = root
        .canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "Projeto".into());
    std::fs::read_to_string(root.join("config.lua"))
        .ok()
        .and_then(|t| config_lua::read(&t, &folder).ok())
        .map(|v| v.title)
        .filter(|t| !t.trim().is_empty())
        .unwrap_or(folder)
}

/// Confere se a pasta parece um projeto da nn-engine.
pub fn looks_like_project(dir: &Path) -> Result<(), String> {
    if !dir.is_dir() {
        return Err(format!(
            "A pasta {} não existe.",
            crate::settings::display_path(dir)
        ));
    }
    if dir.join("config.lua").is_file() || dir.join("main.lua").is_file() {
        Ok(())
    } else {
        Err(format!(
            "A pasta {} não parece um projeto da nn-engine: não tem config.lua nem main.lua.",
            crate::settings::display_path(dir)
        ))
    }
}

impl ProjectWorkspace {
    pub fn open(root: PathBuf, layout: Option<DockState<Tab>>) -> Self {
        let title = project_title(&root);
        let files = FileTree::new(&root);
        let first_time = layout.is_none();
        let mut dock = layout.unwrap_or_else(default_layout);
        // Abas de arquivos que sumiram desde a última vez saem do layout.
        dock.retain_tabs(|t| t.rel().is_none_or(|r| root.join(r).exists()));
        if dock.iter_all_tabs().next().is_none() {
            dock = default_layout();
        }
        let mut ws = Self {
            root,
            title,
            dock,
            files,
            file_panel: FilePanel::default(),
            docs: HashMap::new(),
            images: HashMap::new(),
            sounds: HashMap::new(),
            maps: HashMap::new(),
            console: Console::default(),
            game: None,
            form: SettingsForm::default(),
            lua: mlua::Lua::new(),
            dialog: None,
            actions: Vec::new(),
            focused_code: None,
            console_drawn: 0,
            frame: 0,
            scenes: HashMap::new(),
            active_scene: None,
            sizes: Sizes::default(),
            scene_renderer: None,
            hierarchy: HierarchyPanel::default(),
            inspector: Inspector::default(),
            game_scene: None,
            config: ConfigValues::default(),
            started: std::time::Instant::now(),
        };
        ws.reload_config();
        ws.console.info(format!(
            "Projeto aberto: {}",
            crate::settings::display_path(&ws.root)
        ));
        // Primeira vez com um jogo que começa numa cena visual: já abre ela.
        let start = ws.config.start.clone();
        if first_time
            && FileKind::of(Path::new(&start)) == FileKind::Scene
            && ws.root.join(&start).is_file()
        {
            ws.open_file(&start, None);
        }
        ws
    }

    /// Arquivos com alterações não salvas.
    pub fn dirty_files(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .docs
            .values()
            .filter(|d| d.is_dirty())
            .map(|d| d.rel.clone())
            .chain(
                self.scenes
                    .values()
                    .filter(|s| s.is_dirty())
                    .map(|s| s.rel.clone()),
            )
            .collect();
        v.sort();
        v
    }

    /// O arquivo tem alterações não salvas (código ou cena).
    pub fn is_dirty(&self, rel: &str) -> bool {
        self.docs.get(rel).is_some_and(CodeDoc::is_dirty)
            || self.scenes.get(rel).is_some_and(SceneDoc::is_dirty)
    }

    /// Esquece as alterações não salvas (antes de fechar sem salvar).
    pub fn discard_changes(&mut self) {
        self.docs.clear();
        self.scenes.clear();
    }

    /// Relê o config.lua (tamanho da tela e cor de fundo da cena).
    fn reload_config(&mut self) {
        let text = std::fs::read_to_string(self.root.join("config.lua")).unwrap_or_default();
        self.config = config_lua::read(&text, &self.title).unwrap_or_default();
    }

    pub fn save_all(&mut self) -> Result<(), String> {
        let mut errors = Vec::new();
        let mut saved = Vec::new();
        for doc in self.docs.values_mut().filter(|d| d.is_dirty()) {
            match doc.save(&self.root) {
                Ok(()) => {
                    doc.check_now(&self.lua);
                    saved.push(doc.rel.clone());
                }
                Err(e) => errors.push(format!("{}: {e}", doc.rel)),
            }
        }
        for scene in self.scenes.values_mut().filter(|s| s.is_dirty()) {
            match scene.save(&self.root) {
                Ok(()) => saved.push(scene.rel.clone()),
                Err(e) => errors.push(format!("{}: {e}", scene.rel)),
            }
        }
        saved.sort();
        for rel in saved {
            self.console.info(format!("Salvo: {rel}"));
        }
        self.files.rescan();
        if errors.is_empty() {
            Ok(())
        } else {
            let msg = format!("Não foi possível salvar:\n{}", errors.join("\n"));
            self.console.error(&msg);
            Err(msg)
        }
    }

    pub fn is_playing(&self) -> bool {
        self.game.is_some()
    }

    /// Abre (ou mostra) uma aba.
    pub fn show_tab(&mut self, tab: Tab) {
        if let Some(path) = self.dock.find_tab(&tab) {
            let _ = self.dock.set_active_tab(path);
            self.dock.set_focused_node_and_surface(path.node_path());
            return;
        }
        match tab {
            Tab::Files => {
                self.dock
                    .main_surface_mut()
                    .split_left(NodeIndex::root(), 0.22, vec![Tab::Files]);
            }
            Tab::Console => {
                self.dock.main_surface_mut().split_below(
                    NodeIndex::root(),
                    0.70,
                    vec![Tab::Console],
                );
            }
            Tab::Hierarchy => {
                // Embaixo do painel Arquivos, se ele estiver aberto.
                match self.dock.find_tab(&Tab::Files) {
                    Some(path) if path.surface == egui_dock::SurfaceIndex::main() => {
                        self.dock.main_surface_mut().split_below(
                            path.node,
                            0.5,
                            vec![Tab::Hierarchy],
                        );
                    }
                    _ => {
                        self.dock.main_surface_mut().split_left(
                            NodeIndex::root(),
                            0.2,
                            vec![Tab::Hierarchy],
                        );
                    }
                }
            }
            Tab::Inspector => {
                self.dock.main_surface_mut().split_right(
                    NodeIndex::root(),
                    0.78,
                    vec![Tab::Inspector],
                );
            }
            _ => {
                // Vai para onde já estão os outros documentos.
                match self.dock.find_tab_from(Tab::is_document) {
                    Some(path) => {
                        if let Ok(leaf) = self.dock.leaf_mut(path.node_path()) {
                            leaf.append_tab(tab.clone());
                        }
                    }
                    None => {
                        let has_files = self.dock.find_tab(&Tab::Files).is_some();
                        if has_files {
                            self.dock.main_surface_mut().split_right(
                                NodeIndex::root(),
                                0.22,
                                vec![tab.clone()],
                            );
                        } else {
                            self.dock.push_to_first_leaf(tab.clone());
                        }
                    }
                }
                if let Some(path) = self.dock.find_tab(&tab) {
                    let _ = self.dock.set_active_tab(path);
                    self.dock.set_focused_node_and_surface(path.node_path());
                }
            }
        }
    }

    /// Fecha a aba (sem perguntar).
    fn close_tab(&mut self, tab: &Tab) {
        if let Some(path) = self.dock.find_tab(tab) {
            self.dock.remove_tab(path);
        }
        self.forget(tab);
    }

    fn forget(&mut self, tab: &Tab) {
        match tab {
            Tab::Code(r) => {
                self.docs.remove(r);
                if self.focused_code.as_deref() == Some(r.as_str()) {
                    self.focused_code = None;
                }
            }
            Tab::Image(r) => {
                self.images.remove(r);
            }
            Tab::Sound(r) => {
                if let Some(mut s) = self.sounds.remove(r) {
                    s.stop();
                }
            }
            Tab::Map(r) => {
                self.maps.remove(r);
            }
            Tab::Scene(r) => {
                self.scenes.remove(r);
                if let Some(sr) = &mut self.scene_renderer {
                    sr.forget(r);
                }
                if self.active_scene.as_deref() == Some(r.as_str()) {
                    self.active_scene = self.scenes.keys().next().cloned();
                }
            }
            _ => {}
        }
    }

    /// Abre a Hierarquia e o Inspetor, se estiverem fechados (sem roubar o foco).
    fn ensure_scene_panels(&mut self) {
        for tab in [Tab::Hierarchy, Tab::Inspector] {
            if self.dock.find_tab(&tab).is_none() {
                self.show_tab(tab);
            }
        }
    }

    /// Abre um arquivo na aba certa (ou no programa padrão).
    pub fn open_file(&mut self, rel: &str, line: Option<usize>) {
        let Some(tab) = Tab::for_file(rel) else {
            self.open_external(rel);
            return;
        };
        if let Tab::Code(r) = &tab
            && !self.docs.contains_key(r)
        {
            match CodeDoc::open(&self.root, r) {
                Ok(mut doc) => {
                    doc.check_now(&self.lua);
                    self.docs.insert(r.clone(), doc);
                }
                Err(e) => {
                    self.console.error(format!("{r}: {e}"));
                    return;
                }
            }
        }
        if let Tab::Scene(r) = &tab {
            if !self.scenes.contains_key(r) {
                let doc = SceneDoc::open(&self.root, r);
                if let Some(e) = &doc.error {
                    self.console.error(format!("{r}: {e}"));
                }
                self.scenes.insert(r.clone(), doc);
            }
            self.active_scene = Some(r.clone());
            self.ensure_scene_panels();
        }
        self.file_panel.reveal(rel);
        self.show_tab(tab);
        if let (Some(line), Some(doc)) = (line, self.docs.get_mut(rel)) {
            doc.go_to_line(line);
        }
        if rel.ends_with(".lua") {
            self.focused_code = Some(rel.to_string());
        }
    }

    fn open_external(&mut self, rel: &str) {
        let full = self.files.full(rel);
        if let Err(e) = opener::open(&full) {
            self.console
                .error(format!("Não foi possível abrir {rel}: {e}"));
        }
    }

    /// Resolve um caminho de mensagem de erro para um arquivo do projeto.
    pub fn resolve(&self, path: &str) -> Option<String> {
        let p = Path::new(path);
        if p.is_absolute() {
            let rel = files::relative(&self.root, p);
            return self.files.exists(&rel).then_some(rel);
        }
        let clean = path.trim_start_matches("./").replace('\\', "/");
        if self.files.exists(&clean) || self.root.join(&clean).is_file() {
            return Some(clean);
        }
        // Só o nome do arquivo: procura no projeto.
        self.files
            .entries()
            .iter()
            .find(|e| !e.is_dir && (e.rel.ends_with(&format!("/{clean}")) || e.rel == clean))
            .map(|e| e.rel.clone())
    }

    /// Cena que recebe Salvar, desfazer etc.: a aba com foco, se for uma
    /// cena (ou a Hierarquia ou o Inspetor dela).
    fn focused_scene(&mut self) -> Option<String> {
        let active = self.active_scene.clone();
        match self.dock.find_active_focused() {
            Some((_, Tab::Scene(r))) => Some(r.clone()),
            Some((_, Tab::Hierarchy | Tab::Inspector)) => active,
            _ => None,
        }
    }

    /// Código que recebe Salvar, Buscar etc.: a aba com foco, se for de código.
    fn active_code(&mut self) -> Option<String> {
        if let Some((_, Tab::Code(r))) = self.dock.find_active_focused() {
            return Some(r.clone());
        }
        self.focused_code
            .clone()
            .filter(|r| self.docs.contains_key(r))
    }

    // ------------------------------------------------------------------
    // Jogo

    pub fn play(&mut self, ctx: &egui::Context, env: &mut Env) {
        self.play_from(ctx, env, None);
    }

    /// Roda o jogo; com `start`, começando nessa cena em vez da inicial.
    pub fn play_from(&mut self, ctx: &egui::Context, env: &mut Env, start: Option<&str>) {
        let _ = self.save_all();
        if let Some(mut g) = self.game.take() {
            g.stop();
        }
        if env.settings.clear_console_on_play {
            self.console.clear();
        }
        let wake_ctx = ctx.clone();
        self.game_scene = None;
        match GameProcess::start(env.exe, &self.root, start, move || {
            wake_ctx.request_repaint()
        }) {
            Ok(g) => {
                match start {
                    Some(scene) => self.console.info(format!("▶ Testando a partir de {scene}.")),
                    None => self.console.info("▶ Jogo iniciado. Salve um arquivo com o jogo aberto e a mudança aparece na hora."),
                }
                self.game = Some(g);
            }
            Err(e) => self
                .console
                .error(format!("Não foi possível iniciar o jogo: {e}")),
        }
    }

    pub fn stop(&mut self) {
        if let Some(mut g) = self.game.take() {
            g.stop();
            self.console.info("⏹ Jogo parado.");
        }
    }

    fn poll_game(&mut self) {
        let Some(game) = &mut self.game else {
            return;
        };
        let lines = game.drain();
        let exit = game.exited();
        let errors_before = self.console.count(crate::console::Category::Error);
        for line in lines {
            if let Some(scene) = crate::console::live_scene(&line) {
                self.game_scene = Some(scene);
                continue;
            }
            self.console.push_game_line(&line);
        }
        if let Some(status) = exit {
            // O que ainda estava no caminho.
            std::thread::sleep(std::time::Duration::from_millis(30));
            if let Some(g) = &self.game {
                for line in g.drain() {
                    self.console.push_game_line(&line);
                }
            }
            match crate::game::describe_exit(status) {
                Some(msg) => self.console.error(msg),
                None => self.console.info("O jogo foi fechado."),
            }
            self.game = None;
            self.game_scene = None;
        }
        // Erro novo com o console escondido: mostra o console.
        if self.console.count(crate::console::Category::Error) > errors_before
            && self.console_drawn + 1 < self.frame
        {
            self.show_tab(Tab::Console);
        }
    }

    fn watch_files(&mut self) {
        let changes = self.files.poll();
        if changes.is_empty() {
            return;
        }
        let root = self.root.clone();
        for rel in &changes.changed {
            if let Some(doc) = self.docs.get_mut(rel) {
                doc.disk_changed(&root);
                doc.check_now(&self.lua);
            }
            if let Some(v) = self.images.get_mut(rel) {
                v.reload();
            }
            if let Some(v) = self.sounds.get_mut(rel) {
                v.reload();
            }
            if rel == "config.lua" {
                self.form.loaded = false;
                self.title = project_title(&self.root);
                self.reload_config();
            }
            if let Some(scene) = self.scenes.get_mut(rel) {
                scene.disk_changed(&root);
            }
            if matches!(
                FileKind::of(Path::new(rel)),
                FileKind::Image | FileKind::Map | FileKind::Tileset
            ) {
                self.sizes.invalidate(rel);
                if FileKind::of(Path::new(rel)) != FileKind::Image {
                    self.sizes.clear();
                }
                if let Some(sr) = &mut self.scene_renderer {
                    sr.invalidate(&root, rel);
                }
            }
        }
        // Mapas dependem de tilesets e imagens: recarrega todos.
        if changes.changed.iter().any(|r| {
            matches!(
                FileKind::of(Path::new(r)),
                FileKind::Map | FileKind::Tileset | FileKind::Image
            )
        }) {
            for m in self.maps.values_mut() {
                m.reload();
            }
        }
        // Arquivo apagado fora: fecha a aba se não houver nada a perder.
        for rel in &changes.removed {
            let dirty = self.is_dirty(rel);
            if !dirty
                && let Some(tab) = Tab::for_file(rel)
                && self.dock.find_tab(&tab).is_some()
            {
                self.close_tab(&tab);
            }
        }
    }

    // ------------------------------------------------------------------
    // Comandos e ações

    /// Executa um comando do projeto. Devolve `false` se não for daqui.
    pub fn run(&mut self, cmd: Command, ctx: &egui::Context, env: &mut Env) -> bool {
        match cmd {
            Command::Play => self.play(ctx, env),
            Command::TestHere => match self.test_target() {
                Some(rel) => self.play_from(ctx, env, Some(&rel)),
                None => self
                    .console
                    .warn("Abra uma cena visual (.cena) para testar o jogo a partir dela."),
            },
            Command::Stop => self.stop(),
            Command::Undo | Command::Redo => {
                if let Some(rel) = self.focused_scene()
                    && let Some(doc) = self.scenes.get_mut(&rel)
                {
                    doc.begin_frame();
                    if cmd == Command::Undo {
                        doc.undo();
                    } else {
                        doc.redo();
                    }
                    doc.end_frame();
                    self.flush_live();
                }
            }
            Command::Save => {
                if let Some(rel) = self.focused_scene() {
                    self.save_doc(&rel);
                } else if let Some(rel) = self.active_code() {
                    self.save_doc(&rel);
                } else {
                    let _ = self.save_all();
                }
            }
            Command::SaveAll => {
                let _ = self.save_all();
            }
            Command::NewScene | Command::NewCodeScene => self.actions.push(Action::NewItem {
                kind: if cmd == Command::NewScene {
                    NewKind::VisualScene
                } else {
                    NewKind::Scene
                },
                parent: if self.files.get("cenas").is_some_and(|e| e.is_dir) {
                    "cenas".into()
                } else {
                    String::new()
                },
            }),
            Command::NewScript => self.actions.push(Action::NewItem {
                kind: NewKind::Module,
                parent: if self.files.get("lib").is_some_and(|e| e.is_dir) {
                    "lib".into()
                } else {
                    String::new()
                },
            }),
            Command::NewFolder => self.actions.push(Action::NewItem {
                kind: NewKind::Folder,
                parent: self.file_panel.target_dir(&self.files).unwrap_or_default(),
            }),
            Command::ImportFiles => self.actions.push(Action::Import {
                dest: self.file_panel.target_dir(&self.files),
            }),
            Command::ProjectSettings => {
                self.form.loaded = false;
                self.show_tab(Tab::Settings);
            }
            Command::Export => {
                let name = self
                    .root
                    .canonicalize()
                    .ok()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                    .unwrap_or_else(|| "jogo".into());
                self.dialog = Some(Dialog::Export(ExportDialog {
                    output: "dist".into(),
                    name,
                    job: None,
                    result: None,
                }));
            }
            Command::Find | Command::Replace => {
                if let Some(rel) = self.active_code() {
                    let sel = self.docs.get(&rel).and_then(|d| d.selected_text(ctx));
                    if let Some(doc) = self.docs.get_mut(&rel) {
                        doc.open_find(ctx, cmd == Command::Replace, sel);
                    }
                }
            }
            Command::GoToLine => {
                if let Some(rel) = self.active_code()
                    && let Some(doc) = self.docs.get_mut(&rel)
                {
                    doc.open_goto(ctx);
                }
            }
            Command::CloseTab => {
                let tab = self.dock.find_active_focused().map(|(_, t)| t.clone());
                if let Some(tab) = tab {
                    match &tab {
                        Tab::Code(r) | Tab::Scene(r) if self.is_dirty(r) => {
                            self.dialog = Some(Dialog::CloseTab(r.clone()));
                        }
                        _ => self.close_tab(&tab),
                    }
                }
            }
            Command::OpenExternal => {
                let target = self
                    .dock
                    .find_active_focused()
                    .and_then(|(_, t)| t.rel().map(str::to_string))
                    .or_else(|| self.file_panel.selected.clone());
                match target {
                    Some(rel) if !rel.is_empty() => self.open_external(&rel),
                    _ => self.console.warn("Escolha um arquivo primeiro."),
                }
            }
            Command::RevealProject => self.actions.push(Action::Reveal(String::new())),
            Command::ShowFiles => self.show_tab(Tab::Files),
            Command::ShowConsole => self.show_tab(Tab::Console),
            Command::ShowHome => self.show_tab(Tab::Home),
            Command::ShowHierarchy => self.show_tab(Tab::Hierarchy),
            Command::ShowInspector => self.show_tab(Tab::Inspector),
            Command::ClearConsole => self.console.clear(),
            Command::ResetLayout => {
                self.dock = default_layout();
                env.settings.remove_layout(&self.root);
            }
            _ => return false,
        }
        true
    }

    /// Cena para "Testar daqui": a que tem o foco ou a última usada.
    fn test_target(&mut self) -> Option<String> {
        self.focused_scene()
            .or_else(|| self.active_scene.clone())
            .filter(|r| self.scenes.contains_key(r))
    }

    /// Manda para o jogo aberto os ajustes ao vivo da cena que ele mostra.
    fn flush_live(&mut self) {
        let target = self.game_scene.clone();
        for (rel, doc) in &mut self.scenes {
            if doc.live.is_empty() {
                continue;
            }
            let changes = std::mem::take(&mut doc.live);
            if target.as_deref() != Some(rel.as_str()) {
                continue;
            }
            if let Some(game) = &mut self.game {
                for c in changes {
                    if !game.send(&c.to_json(rel)) {
                        break;
                    }
                }
            }
        }
    }

    /// Cria (ou liga, se já existir) o script `<cena>.lua` da cena visual.
    fn create_scene_script(&mut self, scene_rel: &str) {
        let script = match scene_rel.rsplit_once('.') {
            Some((stem, _)) => format!("{stem}.lua"),
            None => format!("{scene_rel}.lua"),
        };
        let full = self.root.join(&script);
        if !full.exists() {
            if let Some(dir) = full.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Err(e) = std::fs::write(&full, files::scene_script_template(scene_rel)) {
                self.console
                    .error(format!("Não foi possível criar {script}: {e}"));
                return;
            }
            self.console.info(format!("Criado: {script}"));
            self.files.rescan();
        }
        if let Some(doc) = self.scenes.get_mut(scene_rel) {
            doc.begin_frame();
            doc.record(None);
            doc.data.script = script.clone();
            doc.end_frame();
            // O script é lido ao carregar a cena: salva para o jogo já usar.
            self.save_doc(scene_rel);
        }
        self.open_file(&script, None);
    }

    fn save_doc(&mut self, rel: &str) {
        if let Some(scene) = self.scenes.get_mut(rel) {
            match scene.save(&self.root) {
                Ok(()) => {
                    self.console.info(format!("Salvo: {rel}"));
                    self.files.rescan();
                }
                Err(e) => self
                    .console
                    .error(format!("Não foi possível salvar {rel}: {e}")),
            }
            return;
        }
        let Some(doc) = self.docs.get_mut(rel) else {
            return;
        };
        match doc.save(&self.root) {
            Ok(()) => {
                doc.check_now(&self.lua);
                let issue = doc.issue.clone();
                self.console.info(format!("Salvo: {rel}"));
                if let Some(i) = issue {
                    self.console
                        .warn(format!("{rel}:{}: {}", i.line, i.message));
                }
                self.files.rescan();
            }
            Err(e) => self
                .console
                .error(format!("Não foi possível salvar {rel}: {e}")),
        }
    }

    fn handle_actions(&mut self, ctx: &egui::Context, env: &mut Env) -> Vec<Command> {
        let mut app_commands = Vec::new();
        let actions = std::mem::take(&mut self.actions);
        for action in actions {
            match action {
                Action::Command(c) => {
                    if !self.run(c, ctx, env) {
                        app_commands.push(c);
                    }
                }
                Action::OpenFile { rel, line } => self.open_file(&rel, line),
                Action::OpenLocation { path, line } => match self.resolve(&path) {
                    Some(rel) => self.open_file(&rel, Some(line)),
                    None => self
                        .console
                        .warn(format!("Arquivo não encontrado no projeto: {path}")),
                },
                Action::PlaySound(rel) => {
                    let audio = env.audio.get_or_insert_with(nn_audio::Audio::new);
                    let full = self.files.full(&rel);
                    // Só um som de prévia por vez.
                    for s in self.sounds.values_mut() {
                        s.stop();
                    }
                    self.sounds.entry(rel).or_default().play(audio, &full);
                }
                Action::NewItem { kind, parent } => {
                    self.dialog = Some(Dialog::NewItem {
                        kind,
                        parent,
                        name: kind.default_name().into(),
                        error: None,
                        focus: true,
                    });
                }
                Action::Import { dest } => {
                    let picked = rfd::FileDialog::new()
                        .set_title("Importar arquivos para o projeto")
                        .pick_files();
                    if let Some(paths) = picked {
                        self.import_paths(&paths, dest.as_deref());
                    }
                }
                Action::ImportPaths { paths, dest } => self.import_paths(&paths, dest.as_deref()),
                Action::Rename { rel, new_name } => self.rename(&rel, &new_name),
                Action::AskDelete(rel) => self.dialog = Some(Dialog::ConfirmDelete(rel)),
                Action::AskCloseTab(rel) => self.dialog = Some(Dialog::CloseTab(rel)),
                Action::Reveal(rel) => {
                    let full = if rel.is_empty() {
                        self.root.clone()
                    } else {
                        self.files.full(&rel)
                    };
                    let result = if full.is_dir() {
                        opener::open(&full)
                    } else {
                        opener::reveal(&full)
                    };
                    if let Err(e) = result {
                        self.console
                            .error(format!("Não foi possível mostrar a pasta: {e}"));
                    }
                }
                Action::OpenExternal(rel) => self.open_external(&rel),
                Action::CopyText(text) => {
                    ctx.copy_text(text.clone());
                    self.console.info(format!("Copiado: {text}"));
                }
                Action::SaveDoc(rel) => self.save_doc(&rel),
                Action::TestScene(rel) => self.play_from(ctx, env, Some(&rel)),
                Action::CreateSceneScript(rel) => self.create_scene_script(&rel),
                Action::OpenAsText(rel) => {
                    self.file_panel.reveal(&rel);
                    self.show_tab(Tab::Code(rel));
                }
                Action::ReloadDoc(rel) => {
                    if let Some(doc) = self.docs.get_mut(&rel) {
                        match doc.reload(&self.root) {
                            Ok(()) => doc.check_now(&self.lua),
                            Err(e) => self.console.error(format!("{rel}: {e}")),
                        }
                    }
                }
            }
        }
        app_commands
    }

    pub fn import_paths(&mut self, paths: &[PathBuf], dest: Option<&str>) {
        let mut last = None;
        for r in files::import(&self.root, paths, dest) {
            match r {
                Ok(rel) => {
                    self.console.info(format!("Importado: {rel}"));
                    last = Some(rel);
                }
                Err(e) => self.console.warn(format!("Não importado: {e}")),
            }
        }
        self.files.rescan();
        if let Some(rel) = last {
            self.file_panel.reveal(&rel);
            self.show_tab(Tab::Files);
        }
    }

    fn rename(&mut self, rel: &str, new_name: &str) {
        match files::rename(&self.root, rel, new_name) {
            Ok(new_rel) => {
                self.console.info(format!("Renomeado: {rel} → {new_rel}"));
                let fix = |p: &str| -> Option<String> {
                    if p == rel {
                        Some(new_rel.clone())
                    } else {
                        p.strip_prefix(&format!("{rel}/"))
                            .map(|rest| format!("{new_rel}/{rest}"))
                    }
                };
                for (_, tab) in self.dock.iter_all_tabs_mut() {
                    if let Some(n) = tab.rel().and_then(fix) {
                        tab.set_rel(n);
                    }
                }
                let docs: Vec<String> = self.docs.keys().cloned().collect();
                for k in docs {
                    if let Some(n) = fix(&k)
                        && let Some(mut d) = self.docs.remove(&k)
                    {
                        d.rel = n.clone();
                        self.docs.insert(n, d);
                    }
                }
                let scenes: Vec<String> = self.scenes.keys().cloned().collect();
                for k in scenes {
                    if let Some(n) = fix(&k)
                        && let Some(mut d) = self.scenes.remove(&k)
                    {
                        d.rel = n.clone();
                        if let Some(sr) = &mut self.scene_renderer {
                            sr.forget(&k);
                        }
                        self.scenes.insert(n, d);
                    }
                }
                for slot in [&mut self.active_scene, &mut self.game_scene] {
                    if let Some(n) = slot.as_deref().and_then(fix) {
                        *slot = Some(n);
                    }
                }
                // Visualizadores abrem de novo com o caminho novo.
                self.images.retain(|k, _| fix(k).is_none());
                self.maps.retain(|k, _| fix(k).is_none());
                self.sounds.retain(|k, _| fix(k).is_none());
                if let Some(f) = &self.focused_code
                    && let Some(n) = fix(f)
                {
                    self.focused_code = Some(n);
                }
                self.file_panel.renamed(rel, &new_rel);
                self.files.rescan();
            }
            Err(e) => self
                .console
                .error(format!("Não foi possível renomear {rel}: {e}")),
        }
    }

    fn delete(&mut self, rel: &str) {
        match files::move_to_trash(&self.root, rel) {
            Ok(()) => {
                self.console.info(format!("Mandado para a lixeira: {rel}"));
                let tabs: Vec<Tab> = self
                    .dock
                    .iter_all_tabs()
                    .map(|(_, t)| t.clone())
                    .filter(|t| {
                        t.rel()
                            .is_some_and(|r| r == rel || r.starts_with(&format!("{rel}/")))
                    })
                    .collect();
                for t in tabs {
                    self.close_tab(&t);
                }
                if self.file_panel.selected.as_deref() == Some(rel) {
                    self.file_panel.selected = None;
                }
                self.files.rescan();
            }
            Err(e) => self
                .console
                .error(format!("Não foi possível mandar {rel} para a lixeira: {e}")),
        }
    }

    // ------------------------------------------------------------------
    // Interface

    /// Desenha a área do projeto. Devolve comandos que são do aplicativo.
    pub fn ui(&mut self, ui: &mut egui::Ui, env: &mut Env) -> Vec<Command> {
        self.frame += 1;
        let ctx = ui.ctx().clone();
        self.watch_files();
        self.poll_game();
        if self.game.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        } else {
            ctx.request_repaint_after(files::POLL_INTERVAL);
        }

        self.status_bar(ui);

        // Arquivos arrastados para a janela.
        let (hovering, dropped) = ctx.input(|i| {
            (
                !i.raw.hovered_files.is_empty(),
                i.raw
                    .dropped_files
                    .iter()
                    .map(|f| f.path().to_path_buf())
                    .collect::<Vec<_>>(),
            )
        });
        if !dropped.is_empty() {
            let dest = self
                .file_panel
                .target_dir(&self.files)
                .filter(|d| !d.is_empty());
            self.actions.push(Action::ImportPaths {
                paths: dropped,
                dest,
            });
        }

        let mode = env.settings.mode;
        let mut clear_on_play = env.settings.clear_console_on_play;
        if self.scene_renderer.is_none()
            && let Some(rs) = env.render_state
        {
            self.scene_renderer = Some(SceneRenderer::new(rs));
        }
        for doc in self.scenes.values_mut() {
            doc.begin_frame();
        }
        {
            let mut viewer = Viewer {
                root: &self.root,
                title: &self.title,
                files: &self.files,
                file_panel: &mut self.file_panel,
                docs: &mut self.docs,
                images: &mut self.images,
                sounds: &mut self.sounds,
                maps: &mut self.maps,
                console: &mut self.console,
                form: &mut self.form,
                lua: &self.lua,
                audio: env.audio,
                mode,
                zoom_code: theme::CODE_SIZE,
                clear_on_play: &mut clear_on_play,
                actions: &mut self.actions,
                playing: self.game.is_some(),
                focused_code: &mut self.focused_code,
                console_drawn: &mut self.console_drawn,
                frame: self.frame,
                scenes: &mut self.scenes,
                active_scene: &mut self.active_scene,
                sizes: &mut self.sizes,
                scene_renderer: &mut self.scene_renderer,
                hierarchy: &mut self.hierarchy,
                inspector: &mut self.inspector,
                game_scene: self.game_scene.as_deref(),
                design: (f64::from(self.config.width), f64::from(self.config.height)),
                clear_color: &self.config.clear_color,
                time: self.started.elapsed().as_secs_f32(),
            };
            let mut style = egui_dock::Style::from_egui(ui.style().as_ref());
            style.tab_bar.fill_tab_bar = false;
            DockArea::new(&mut self.dock)
                .id(egui::Id::new("nn-studio-dock"))
                .style(style)
                .show_leaf_collapse_buttons(false)
                .show_leaf_close_all_buttons(false)
                .show_inside(ui, &mut viewer);
        }
        env.settings.clear_console_on_play = clear_on_play;
        // A Hierarquia e o Inspetor seguem a cena com o foco.
        if let Some((_, Tab::Scene(r))) = self.dock.find_active_focused() {
            let r = r.clone();
            if self.scenes.contains_key(&r) {
                self.active_scene = Some(r);
            }
        }
        for doc in self.scenes.values_mut() {
            doc.end_frame();
        }
        self.flush_live();

        if hovering {
            drop_overlay(&ctx, self.file_panel.target_dir(&self.files));
        }
        self.dialogs(&ctx);
        self.handle_actions(&ctx, env)
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("barra-de-status")
            .exact_size(24.0)
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.weak(format!("📂 {}", crate::settings::display_path(&self.root)));
                    let dirty = self.dirty_files();
                    if !dirty.is_empty() {
                        ui.separator();
                        ui.label(
                            RichText::new(if dirty.len() == 1 {
                                "• 1 arquivo não salvo".to_string()
                            } else {
                                format!("• {} arquivos não salvos", dirty.len())
                            })
                            .color(theme::WARNING),
                        )
                        .on_hover_text(dirty.join("\n"));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let errors = self.console.count(crate::console::Category::Error);
                        if errors > 0
                            && ui
                                .link(
                                    RichText::new(format!(
                                        "⛔ {errors} {}",
                                        if errors == 1 { "erro" } else { "erros" }
                                    ))
                                    .color(theme::ERROR),
                                )
                                .on_hover_text("Mostrar o console")
                                .clicked()
                        {
                            self.show_tab(Tab::Console);
                        }
                        if let Some(g) = &self.game {
                            ui.label(
                                RichText::new(format!(
                                    "• Jogo rodando há {}",
                                    crate::game::format_duration(g.running_for())
                                ))
                                .color(theme::PLAY),
                            );
                        }
                    });
                });
            });
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(dialog) = &mut self.dialog else {
            return;
        };
        let mut close = false;
        let mut after: Option<Action> = None;
        let mut created: Option<String> = None;
        let mut export_job = None;
        match dialog {
            Dialog::NewItem {
                kind,
                parent,
                name,
                error,
                focus,
            } => {
                let kind = *kind;
                let dirs: Vec<String> = std::iter::once(String::new())
                    .chain(
                        self.files
                            .entries()
                            .iter()
                            .filter(|e| e.is_dir)
                            .map(|e| e.rel.clone()),
                    )
                    .collect();
                let response = egui::Modal::new(egui::Id::new("dialogo-novo")).show(ctx, |ui| {
                    ui.set_width(380.0);
                    ui.heading(kind.title());
                    ui.add_space(6.0);
                    ui.label("Nome:");
                    let r = ui.add(egui::TextEdit::singleline(name).desired_width(f32::INFINITY));
                    if *focus {
                        r.request_focus();
                        *focus = false;
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Na pasta:");
                        egui::ComboBox::from_id_salt("pasta-novo")
                            .selected_text(if parent.is_empty() { "(raiz do projeto)".to_string() } else { parent.clone() })
                            .show_ui(ui, |ui| {
                                for d in &dirs {
                                    let label = if d.is_empty() { "(raiz do projeto)" } else { d.as_str() };
                                    ui.selectable_value(parent, d.clone(), label);
                                }
                            });
                    });
                    let mut file_name = name.trim().to_string();
                    if kind != NewKind::Folder && Path::new(&file_name).extension().is_none() {
                        file_name.push_str(if kind == NewKind::VisualScene { ".cena" } else { ".lua" });
                    }
                    let rel = files::join_rel(parent, &file_name);
                    ui.weak(format!("Será criado: {rel}"));
                    if kind == NewKind::Scene {
                        ui.weak("A cena já vem com um quadrado que anda com WASD, as setas ou o controle.");
                    }
                    if kind == NewKind::VisualScene {
                        ui.weak("Monte a cena arrastando imagens, mapas e sons para ela. Se precisar de código, crie o script dela no Inspetor.");
                    }
                    if let Some(e) = error {
                        ui.colored_label(theme::ERROR, e.as_str());
                    }
                    ui.add_space(8.0);
                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    ui.horizontal(|ui| {
                        let create = ui.add(egui::Button::new("Criar").fill(theme::ACCENT.gamma_multiply(0.7))).clicked();
                        if create || enter {
                            let result = match kind {
                                NewKind::Folder => files::create_folder(&self.root, parent, name),
                                NewKind::Scene => files::create_file(&self.root, parent, name, &files::scene_template(&rel)),
                                NewKind::VisualScene => files::create_file(&self.root, parent, &file_name, &files::visual_scene_template()),
                                NewKind::Module => files::create_file(&self.root, parent, name, &files::module_template(&rel)),
                            };
                            match result {
                                Ok(rel) => created = Some(rel),
                                Err(e) => *error = Some(e),
                            }
                        }
                        if ui.button("Cancelar").clicked() {
                            close = true;
                        }
                    });
                });
                if response.should_close() {
                    close = true;
                }
                if let Some(rel) = &created {
                    close = true;
                    self.console.info(format!("Criado: {rel}"));
                    self.files.rescan();
                    if kind == NewKind::Folder {
                        self.file_panel.reveal(rel);
                    } else {
                        after = Some(Action::OpenFile {
                            rel: rel.clone(),
                            line: None,
                        });
                    }
                }
            }
            Dialog::ConfirmDelete(rel) => {
                let rel = rel.clone();
                let is_dir = self.files.get(&rel).is_some_and(|e| e.is_dir);
                let dirty = self.is_dirty(&rel);
                let mut confirm = false;
                let response = egui::Modal::new(egui::Id::new("dialogo-excluir")).show(ctx, |ui| {
                    ui.set_width(360.0);
                    ui.heading("Mandar para a lixeira?");
                    ui.add_space(6.0);
                    ui.label(format!(
                        "{} \"{rel}\" vai para a lixeira do sistema. Dá para recuperar de lá.",
                        if is_dir { "A pasta" } else { "O arquivo" }
                    ));
                    if dirty {
                        ui.colored_label(
                            theme::WARNING,
                            "As alterações não salvas dele serão perdidas.",
                        );
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new("🗑 Mandar para a lixeira")
                                    .fill(theme::ERROR.gamma_multiply(0.6)),
                            )
                            .clicked()
                        {
                            confirm = true;
                        }
                        if ui.button("Cancelar").clicked() {
                            close = true;
                        }
                    });
                });
                if response.should_close() {
                    close = true;
                }
                if confirm {
                    self.dialog = None;
                    self.delete(&rel);
                    return;
                }
            }
            Dialog::CloseTab(rel) => {
                let rel = rel.clone();
                let mut choice = None;
                let response =
                    egui::Modal::new(egui::Id::new("dialogo-fechar-aba")).show(ctx, |ui| {
                        ui.set_width(360.0);
                        ui.heading("Salvar as alterações?");
                        ui.add_space(6.0);
                        ui.label(format!("O arquivo {rel} tem alterações não salvas."));
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add(
                                    egui::Button::new("Salvar")
                                        .fill(theme::ACCENT.gamma_multiply(0.7)),
                                )
                                .clicked()
                            {
                                choice = Some(true);
                            }
                            if ui.button("Não salvar").clicked() {
                                choice = Some(false);
                            }
                            if ui.button("Cancelar").clicked() {
                                close = true;
                            }
                        });
                    });
                if response.should_close() {
                    close = true;
                }
                if let Some(save) = choice {
                    if save {
                        self.save_doc(&rel);
                    }
                    if !save || !self.is_dirty(&rel) {
                        if !save && let Some(scene) = self.scenes.get_mut(&rel) {
                            scene.reload(&self.root);
                        }
                        if let Some(tab) = Tab::for_file(&rel) {
                            self.close_tab(&tab);
                        }
                    }
                    close = true;
                }
            }
            Dialog::Export(ex) => {
                if let Some(rx) = &ex.job
                    && let Ok(result) = rx.try_recv()
                {
                    match &result {
                        Ok(r) => self.console.info(format!(
                            "Jogo exportado: {} ({})",
                            crate::settings::display_path(&r.output),
                            files::human_size(r.total_bytes)
                        )),
                        Err(e) => self.console.error(format!("A exportação falhou: {e}")),
                    }
                    ex.result = Some(result);
                    ex.job = None;
                }
                let running = ex.job.is_some();
                let mut start = false;
                let mut open_folder = None;
                let response = egui::Modal::new(egui::Id::new("dialogo-exportar")).show(ctx, |ui| {
                    ui.set_width(440.0);
                    ui.heading("Exportar jogo");
                    ui.add_space(4.0);
                    ui.label("Gera um executável único com o jogo, para mandar para outras pessoas. Quem for jogar não precisa instalar nada.");
                    ui.add_space(6.0);
                    egui::Grid::new("grade-exportar").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                        ui.label("Nome do arquivo:");
                        ui.add_enabled(!running, egui::TextEdit::singleline(&mut ex.name).desired_width(240.0));
                        ui.end_row();
                        ui.label("Pasta de saída:");
                        ui.horizontal(|ui| {
                            ui.add_enabled(!running, egui::TextEdit::singleline(&mut ex.output).desired_width(180.0));
                            if ui.add_enabled(!running, egui::Button::new("Escolher...")).clicked()
                                && let Some(dir) = rfd::FileDialog::new().set_title("Pasta de saída").pick_folder()
                            {
                                ex.output = dir.to_string_lossy().into_owned();
                            }
                        });
                        ui.end_row();
                    });
                    ui.add_space(6.0);
                    if running {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Exportando... (na primeira vez pode demorar alguns minutos)");
                        });
                        ctx.request_repaint_after(std::time::Duration::from_millis(200));
                    }
                    match &ex.result {
                        Some(Ok(r)) => {
                            ui.colored_label(theme::PLAY, format!(
                                "✔ Pronto! {} ({}, {} arquivos do projeto)",
                                crate::settings::display_path(&r.output),
                                files::human_size(r.total_bytes),
                                r.files
                            ));
                            if ui.button("📂 Abrir a pasta").clicked() {
                                open_folder = Some(r.output.clone());
                            }
                        }
                        Some(Err(e)) => {
                            ui.colored_label(theme::ERROR, e.as_str());
                        }
                        None => {}
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.add_enabled(!running, egui::Button::new("Exportar").fill(theme::ACCENT.gamma_multiply(0.7))).clicked() {
                            start = true;
                        }
                        if ui.add_enabled(!running, egui::Button::new("Fechar")).clicked() {
                            close = true;
                        }
                    });
                });
                if response.should_close() && !running {
                    close = true;
                }
                if let Some(out) = open_folder {
                    let _ = opener::reveal(&out);
                }
                if start {
                    let out = PathBuf::from(&ex.output);
                    let output_dir = if out.is_absolute() {
                        out
                    } else {
                        self.root.join(out)
                    };
                    let opts = nn_run::ExportOptions {
                        project: self.root.clone(),
                        output_dir,
                        runtime: None,
                        name: Some(ex.name.trim().to_string()).filter(|n| !n.is_empty()),
                    };
                    let (tx, rx) = std::sync::mpsc::channel();
                    ex.job = Some(rx);
                    ex.result = None;
                    export_job = Some((opts, tx));
                }
            }
        }
        if close {
            self.dialog = None;
        }
        if let Some(a) = after {
            self.actions.push(a);
        }
        if let Some((opts, tx)) = export_job {
            let _ = self.save_all();
            self.console.info("Exportando o jogo...");
            let wake = ctx.clone();
            std::thread::spawn(move || {
                let _ = tx.send(nn_run::export(&opts));
                wake.request_repaint();
            });
        }
    }
}

fn drop_overlay(ctx: &egui::Context, dest: Option<String>) {
    let screen = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("soltar-arquivos"),
    ));
    painter.rect_filled(screen, 0.0, Color32::from_black_alpha(150));
    let text = match dest.filter(|d| !d.is_empty()) {
        Some(d) => format!("Solte para importar para {d}"),
        None => {
            "Solte para importar\nimagens vão para assets/imagens, sons para assets/sons".into()
        }
    };
    painter.text(
        screen.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(22.0),
        Color32::WHITE,
    );
}

/// Desenha cada aba.
struct Viewer<'a> {
    root: &'a Path,
    title: &'a str,
    files: &'a FileTree,
    file_panel: &'a mut FilePanel,
    docs: &'a mut HashMap<String, CodeDoc>,
    images: &'a mut HashMap<String, ImageView>,
    sounds: &'a mut HashMap<String, SoundView>,
    maps: &'a mut HashMap<String, MapView>,
    console: &'a mut Console,
    form: &'a mut SettingsForm,
    lua: &'a mlua::Lua,
    audio: &'a mut Option<nn_audio::Audio>,
    mode: Mode,
    zoom_code: f32,
    clear_on_play: &'a mut bool,
    actions: &'a mut Vec<Action>,
    playing: bool,
    focused_code: &'a mut Option<String>,
    console_drawn: &'a mut u64,
    frame: u64,
    scenes: &'a mut HashMap<String, SceneDoc>,
    active_scene: &'a mut Option<String>,
    sizes: &'a mut Sizes,
    scene_renderer: &'a mut Option<SceneRenderer>,
    hierarchy: &'a mut HierarchyPanel,
    inspector: &'a mut Inspector,
    game_scene: Option<&'a str>,
    design: (f64, f64),
    clear_color: &'a str,
    time: f32,
}

impl TabViewer for Viewer<'_> {
    type Tab = Tab;

    fn id(&mut self, tab: &mut Tab) -> egui::Id {
        egui::Id::new(("aba", &*tab))
    }

    fn title(&mut self, tab: &mut Tab) -> egui::WidgetText {
        let name = match tab {
            Tab::Home => "Início".to_string(),
            Tab::Files => "Arquivos".to_string(),
            Tab::Console => {
                if self.console.unseen_errors > 0 {
                    format!("Console ({})", self.console.unseen_errors)
                } else {
                    "Console".to_string()
                }
            }
            Tab::Settings => "Configurações".to_string(),
            Tab::Hierarchy => "Hierarquia".to_string(),
            Tab::Inspector => "Inspetor".to_string(),
            Tab::Code(r) | Tab::Image(r) | Tab::Sound(r) | Tab::Map(r) | Tab::Scene(r) => {
                let name = r.rsplit('/').next().unwrap_or(r).to_string();
                if self.docs.get(r.as_str()).is_some_and(CodeDoc::is_dirty)
                    || self.scenes.get(r.as_str()).is_some_and(SceneDoc::is_dirty)
                {
                    format!("{name} •")
                } else {
                    name
                }
            }
        };
        name.into()
    }

    fn on_tab_button(&mut self, tab: &mut Tab, response: &egui::Response) {
        if let Some(r) = tab.rel() {
            response.clone().on_hover_text(r);
        }
    }

    fn allowed_in_windows(&self, _tab: &mut Tab) -> bool {
        false
    }

    fn on_close(&mut self, tab: &mut Tab) -> OnCloseResponse {
        match tab {
            Tab::Code(r) if self.docs.get(r.as_str()).is_some_and(CodeDoc::is_dirty) => {
                self.actions.push(Action::AskCloseTab(r.clone()));
                OnCloseResponse::Ignore
            }
            Tab::Scene(r) if self.scenes.get(r.as_str()).is_some_and(SceneDoc::is_dirty) => {
                self.actions.push(Action::AskCloseTab(r.clone()));
                OnCloseResponse::Ignore
            }
            Tab::Scene(r) => {
                self.scenes.remove(r.as_str());
                if let Some(sr) = self.scene_renderer {
                    sr.forget(r);
                }
                if self.active_scene.as_deref() == Some(r.as_str()) {
                    *self.active_scene = self.scenes.keys().next().cloned();
                }
                OnCloseResponse::Close
            }
            Tab::Code(r) => {
                self.docs.remove(r.as_str());
                OnCloseResponse::Close
            }
            Tab::Image(r) => {
                self.images.remove(r.as_str());
                OnCloseResponse::Close
            }
            Tab::Sound(r) => {
                if let Some(mut s) = self.sounds.remove(r.as_str()) {
                    s.stop();
                }
                OnCloseResponse::Close
            }
            Tab::Map(r) => {
                self.maps.remove(r.as_str());
                OnCloseResponse::Close
            }
            _ => OnCloseResponse::Close,
        }
    }

    fn context_menu(&mut self, ui: &mut egui::Ui, tab: &mut Tab, _path: egui_dock::NodePath) {
        if let Some(rel) = tab.rel() {
            if ui.button("Mostrar no painel Arquivos").clicked() {
                self.file_panel.reveal(rel);
                self.actions.push(Action::Command(Command::ShowFiles));
                ui.close();
            }
            if ui.button("📋 Copiar caminho").clicked() {
                self.actions.push(Action::CopyText(format!("\"{rel}\"")));
                ui.close();
            }
            if ui.button("Abrir no programa padrão").clicked() {
                self.actions.push(Action::OpenExternal(rel.to_string()));
                ui.close();
            }
        }
    }

    fn scroll_bars(&self, tab: &Tab) -> [bool; 2] {
        match tab {
            Tab::Home | Tab::Settings => [false, true],
            _ => [false, false],
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Tab) {
        match tab {
            Tab::Home => self.home(ui),
            Tab::Files => {
                self.file_panel
                    .ui(ui, self.files, self.mode, self.title, self.actions);
            }
            Tab::Console => {
                *self.console_drawn = self.frame;
                if let Some(ConsoleAction::OpenLocation { path, line }) =
                    self.console.ui(ui, self.mode.is_full(), self.clear_on_play)
                {
                    self.actions.push(Action::OpenLocation { path, line });
                }
            }
            Tab::Settings => self.settings(ui),
            Tab::Code(rel) => {
                if !self.docs.contains_key(rel.as_str()) {
                    match CodeDoc::open(self.root, rel) {
                        Ok(mut d) => {
                            d.check_now(self.lua);
                            self.docs.insert(rel.clone(), d);
                        }
                        Err(e) => {
                            ui.colored_label(
                                theme::ERROR,
                                format!("Não foi possível abrir {rel}: {e}"),
                            );
                            return;
                        }
                    }
                }
                let doc = self.docs.get_mut(rel.as_str()).expect("documento aberto");
                for a in doc.ui(ui, self.lua, self.zoom_code) {
                    match a {
                        CodeAction::Save => self.actions.push(Action::SaveDoc(rel.clone())),
                        CodeAction::Reload => self.actions.push(Action::ReloadDoc(rel.clone())),
                    }
                }
                if ui.memory(|m| m.has_focus(doc.editor_id())) {
                    *self.focused_code = Some(rel.clone());
                }
            }
            Tab::Image(rel) => {
                let path = self.root.join(rel.as_str());
                let view = self.images.entry(rel.clone()).or_default();
                if let Some(a) = view.ui(ui, &path, rel) {
                    self.viewer_action(a);
                }
            }
            Tab::Sound(rel) => {
                let path = self.root.join(rel.as_str());
                let audio = self.audio.get_or_insert_with(nn_audio::Audio::new);
                let view = self.sounds.entry(rel.clone()).or_default();
                if let Some(a) = view.ui(ui, audio, &path, rel) {
                    self.viewer_action(a);
                }
            }
            Tab::Map(rel) => {
                let view = self.maps.entry(rel.clone()).or_default();
                if let Some(a) = view.ui(ui, self.root, rel) {
                    self.viewer_action(a);
                }
            }
            Tab::Scene(rel) => self.scene(ui, rel),
            Tab::Hierarchy => {
                let doc = self
                    .active_scene
                    .as_deref()
                    .and_then(|r| self.scenes.get_mut(r));
                self.hierarchy.ui(ui, doc);
            }
            Tab::Inspector => {
                let active = self.active_scene.clone();
                let doc = active.as_deref().and_then(|r| self.scenes.get_mut(r));
                for a in self
                    .inspector
                    .ui(ui, doc, self.files, self.sizes, self.mode)
                {
                    match a {
                        InspectorAction::OpenFile(rel) => {
                            self.actions.push(Action::OpenFile { rel, line: None });
                        }
                        InspectorAction::CreateScript => {
                            if let Some(r) = &active {
                                self.actions.push(Action::CreateSceneScript(r.clone()));
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Viewer<'_> {
    fn scene(&mut self, ui: &mut egui::Ui, rel: &str) {
        if !self.scenes.contains_key(rel) {
            // Aba que veio do layout salvo: abre agora.
            let mut doc = SceneDoc::open(self.root, rel);
            doc.begin_frame();
            self.scenes.insert(rel.to_string(), doc);
        }
        if self.active_scene.is_none() {
            *self.active_scene = Some(rel.to_string());
        }
        let doc = self.scenes.get_mut(rel).expect("cena aberta");
        let mut env = SceneEnv {
            root: self.root,
            sizes: self.sizes,
            renderer: self.scene_renderer.as_mut(),
            design: self.design,
            clear_color: self.clear_color,
            connected: self.game_scene == Some(rel),
            time: self.time,
        };
        let actions = doc.ui(ui, &mut env);
        for a in actions {
            match a {
                SceneAction::TestHere => self.actions.push(Action::TestScene(rel.to_string())),
                SceneAction::Reload => doc.reload(self.root),
                SceneAction::OpenAsText => self.actions.push(Action::OpenAsText(rel.to_string())),
            }
        }
        // Clicou nesta cena: a Hierarquia e o Inspetor passam a mostrá-la.
        if ui.ui_contains_pointer() && ui.input(|i| i.pointer.any_pressed()) {
            *self.active_scene = Some(rel.to_string());
        }
    }

    fn viewer_action(&mut self, a: ViewerAction) {
        self.actions.push(match a {
            ViewerAction::CopyText(t) => Action::CopyText(t),
            ViewerAction::OpenExternal(r) => Action::OpenExternal(r),
        });
    }

    fn home(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(self.title).size(28.0).strong());
                ui.weak(crate::settings::display_path(self.root));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let play = if self.playing { "⟳ Reiniciar o jogo" } else { "▶ Jogar" };
                    if ui.add(theme::big_button(play, theme::PLAY)).on_hover_text("F5").clicked() {
                        self.actions.push(Action::Command(Command::Play));
                    }
                    if ui.button("⚙ Configurações do projeto").clicked() {
                        self.actions.push(Action::Command(Command::ProjectSettings));
                    }
                    if ui.button("📦 Exportar jogo").clicked() {
                        self.actions.push(Action::Command(Command::Export));
                    }
                });
                ui.add_space(18.0);

                // Cena inicial.
                let start = std::fs::read_to_string(self.root.join("config.lua"))
                    .ok()
                    .and_then(|t| config_lua::read(&t, "").ok())
                    .map(|v| v.start)
                    .unwrap_or_else(|| "main.lua".into());
                ui.label(RichText::new("Primeiros passos").size(17.0).strong());
                ui.add_space(4.0);
                let step = |ui: &mut egui::Ui, n: &str, text: &str| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(n).strong().color(theme::ACCENT));
                        ui.label(text);
                    });
                };
                step(ui, "1.", "Clique em ▶ Jogar (ou aperte F5): o jogo abre na janela dele.");
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("2.").strong().color(theme::ACCENT));
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label("Abra a cena inicial, ");
                    if ui.link(&start).clicked() {
                        self.actions.push(Action::OpenFile { rel: start.clone(), line: None });
                    }
                    ui.label(", mude um número ou uma cor e salve (Ctrl+S). Com o jogo aberto, a mudança aparece na hora.");
                });
                step(ui, "3.", "Arraste imagens e sons do seu computador para esta janela: eles entram no projeto.");
                step(ui, "4.", "Clique com o botão direito num arquivo para copiar o caminho e usar no código.");
                step(ui, "5.", "Ctrl+Shift+P abre a paleta com todos os comandos. Ctrl+P acha qualquer arquivo.");
                ui.add_space(18.0);

                let count = |k: FileKind| self.files.files_of(k).count();
                let lua = self.files.files_of(FileKind::Lua).filter(|e| e.rel != "config.lua").count();
                ui.label(RichText::new("O projeto").size(17.0).strong());
                ui.add_space(4.0);
                ui.label(format!(
                    "📜 {lua} scripts   🖼 {} imagens   🔊 {} sons   🗺 {} mapas",
                    count(FileKind::Image),
                    count(FileKind::Sound),
                    count(FileKind::Map)
                ));
                ui.add_space(18.0);
                ui.label(RichText::new("Aprender").size(17.0).strong());
                ui.add_space(4.0);
                ui.hyperlink_to("📖 Guia do Lua na nn-engine (todas as funções nn.*)", DOCS_URL);
                ui.hyperlink_to("🎓 Lua em 15 minutos (em inglês)", "https://learnxinyminutes.com/docs/lua/");
            });
        });
    }

    fn settings(&mut self, ui: &mut egui::Ui) {
        let path = self.root.join("config.lua");
        if !self.form.loaded {
            self.form.loaded = true;
            let folder = self
                .root
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            match config_lua::read(&text, &folder) {
                Ok(v) => {
                    self.form.values = v.clone();
                    self.form.original = Some(v);
                    self.form.error = None;
                }
                Err(e) => {
                    self.form.original = None;
                    self.form.error = Some(e);
                }
            }
        }
        ui.add_space(8.0);
        ui.heading("Configurações do projeto");
        ui.weak("Ficam no arquivo config.lua. Valem na próxima vez que o jogo abrir.");
        ui.add_space(8.0);
        if let Some(e) = &self.form.error {
            ui.colored_label(theme::ERROR, format!("O config.lua tem um erro: {e}"));
            if ui.button("Abrir config.lua").clicked() {
                self.actions.push(Action::OpenFile {
                    rel: "config.lua".into(),
                    line: None,
                });
            }
            return;
        }
        let full = self.mode.is_full();
        let scenes: Vec<String> = self
            .files
            .files_of(FileKind::Lua)
            .filter(|e| e.rel != "config.lua")
            .map(|e| e.rel.clone())
            .collect();
        let v = &mut self.form.values;
        egui::Grid::new("grade-config")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                ui.label("Título do jogo:");
                ui.add(egui::TextEdit::singleline(&mut v.title).desired_width(280.0));
                ui.end_row();

                ui.label("Tamanho da janela:");
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut v.width).range(160..=7680).suffix(" px"));
                    ui.label("×");
                    ui.add(egui::DragValue::new(&mut v.height).range(120..=4320).suffix(" px"));
                    egui::ComboBox::from_id_salt("tamanhos")
                        .selected_text("Padrões")
                        .show_ui(ui, |ui| {
                            for (w, h, name) in [
                                (1280, 720, "1280 × 720 (HD)"),
                                (1920, 1080, "1920 × 1080 (Full HD)"),
                                (960, 540, "960 × 540"),
                                (640, 360, "640 × 360 (pixel art)"),
                                (800, 600, "800 × 600 (4:3)"),
                            ] {
                                if ui.selectable_label(v.width == w && v.height == h, name).clicked() {
                                    v.width = w;
                                    v.height = h;
                                }
                            }
                        });
                });
                ui.end_row();

                ui.label("Tela cheia:");
                ui.checkbox(&mut v.fullscreen, "Começar em tela cheia");
                ui.end_row();

                ui.label("Cor de fundo:");
                ui.horizontal(|ui| {
                    let mut rgb = parse_hex(&v.clear_color).unwrap_or([0x1E, 0x1E, 0x28]);
                    if ui.color_edit_button_srgb(&mut rgb).changed() {
                        v.clear_color = format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
                    }
                    ui.weak(&v.clear_color);
                });
                ui.end_row();

                ui.label("Cena inicial:");
                egui::ComboBox::from_id_salt("cena-inicial")
                    .selected_text(v.start.clone())
                    .width(280.0)
                    .show_ui(ui, |ui| {
                        for s in &scenes {
                            ui.selectable_value(&mut v.start, s.clone(), s);
                        }
                    });
                ui.end_row();

                if full {
                    ui.label("Identificador:");
                    ui.add(egui::TextEdit::singleline(&mut v.id).desired_width(280.0))
                        .on_hover_text("Nome da pasta onde o progresso é salvo. Não mude depois de distribuir o jogo.");
                    ui.end_row();
                    ui.label("Janela:");
                    ui.checkbox(&mut v.resizable, "Pode mudar de tamanho");
                    ui.end_row();
                    ui.label("Sincronia vertical:");
                    ui.checkbox(&mut v.vsync, "VSync (evita cortes na imagem)");
                    ui.end_row();
                    ui.label("Recarregar ao salvar:");
                    ui.checkbox(&mut v.hot_reload, "Atualiza o jogo aberto quando um .lua é salvo");
                    ui.end_row();
                    ui.label("Passos fixos por segundo:");
                    ui.add(egui::DragValue::new(&mut v.fixed_hz).range(1..=1000).suffix(" Hz"));
                    ui.end_row();
                }
            });
        if !full {
            ui.add_space(4.0);
            ui.weak("Mais opções no modo Completo (botão Completo, no alto à direita).");
        }
        ui.add_space(12.0);
        let changed = self
            .form
            .original
            .as_ref()
            .map(|o| config_lua::changes(o, &self.form.values))
            .unwrap_or_default();
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !changed.is_empty(),
                    egui::Button::new("💾 Salvar configurações")
                        .fill(theme::ACCENT.gamma_multiply(0.7)),
                )
                .clicked()
            {
                let text = std::fs::read_to_string(&path).unwrap_or_default();
                let new = config_lua::update(&text, &changed, &self.form.values);
                match files::write_atomic(&path, &new) {
                    Ok(()) => {
                        self.form.original = Some(self.form.values.clone());
                        self.console.info("Configurações salvas em config.lua.");
                    }
                    Err(e) => self
                        .console
                        .error(format!("Não foi possível salvar o config.lua: {e}")),
                }
            }
            if ui
                .add_enabled(!changed.is_empty(), egui::Button::new("Desfazer"))
                .clicked()
                && let Some(o) = &self.form.original
            {
                self.form.values = o.clone();
            }
            if full && ui.button("Abrir config.lua").clicked() {
                self.actions.push(Action::OpenFile {
                    rel: "config.lua".into(),
                    line: None,
                });
            }
        });
    }
}

/// Endereço do guia do Lua.
pub const DOCS_URL: &str = "https://github.com/nereuneto/nn-engine/blob/main/docs/lua.md";

fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let h = s.trim().strip_prefix('#')?;
    if h.len() < 6 {
        return None;
    }
    let b = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    Some([b(0)?, b(2)?, b(4)?])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aba_certa_para_cada_arquivo() {
        assert_eq!(
            Tab::for_file("cenas/a.lua"),
            Some(Tab::Code("cenas/a.lua".into()))
        );
        assert_eq!(
            Tab::for_file("LEIAME.md"),
            Some(Tab::Code("LEIAME.md".into()))
        );
        assert_eq!(
            Tab::for_file("fases/f.tmj"),
            Some(Tab::Map("fases/f.tmj".into()))
        );
        assert_eq!(
            Tab::for_file("fases/f.tmx"),
            Some(Tab::Code("fases/f.tmx".into()))
        );
        assert_eq!(Tab::for_file("a/h.PNG"), Some(Tab::Image("a/h.PNG".into())));
        assert_eq!(Tab::for_file("s.wav"), Some(Tab::Sound("s.wav".into())));
        assert_eq!(Tab::for_file("fonte.ttf"), None);
    }

    #[test]
    fn cor_hex() {
        assert_eq!(parse_hex("#14202E"), Some([0x14, 0x20, 0x2E]));
        assert_eq!(parse_hex("#14202EFF"), Some([0x14, 0x20, 0x2E]));
        assert_eq!(parse_hex("azul"), None);
    }

    #[test]
    fn titulo_e_projeto_valido() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jogos/plataforma");
        assert_eq!(project_title(&root), "Plataforma — nn-engine");
        assert!(looks_like_project(&root).is_ok());
        assert!(looks_like_project(&root.join("cenas")).is_err());
        assert!(looks_like_project(&root.join("nao-existe")).is_err());
    }

    #[test]
    fn layout_padrao_tem_os_paineis() {
        let d = default_layout();
        let tabs: Vec<_> = d.iter_all_tabs().map(|(_, t)| t.clone()).collect();
        assert!(tabs.contains(&Tab::Files));
        assert!(tabs.contains(&Tab::Console));
        assert!(tabs.contains(&Tab::Home));
    }
}
