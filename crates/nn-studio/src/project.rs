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
use crate::hierarchy::{HierarchyAction, HierarchyPanel};
use crate::inspector::{Inspector, InspectorAction};
use crate::pixel_editor::{PixelAction, PixelDoc};
use crate::scene_render::{SceneRenderer, Sizes};
use crate::scene_view::{SceneAction, SceneDoc, SceneEnv};
use crate::settings::{Mode, Settings};
use crate::theme;
use crate::viewers::{MapView, SoundView, ViewerAction};

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
    /// Eventos "Quando... Então..." da cena ativa.
    Events,
    /// Painel Tiles: pincel, camadas e tileset do mapa sendo pintado.
    Tiles,
    /// Diálogo (`.dialogo`) no editor de conversa.
    Dialogue(String),
    /// Modelo 3D (`.modelo`) no modelador.
    Model(String),
}

impl Tab {
    /// A aba certa para abrir um arquivo (`None`: abrir fora).
    pub fn for_file(rel: &str) -> Option<Tab> {
        let lower = rel.to_lowercase();
        Some(match FileKind::of(Path::new(rel)) {
            FileKind::Lua | FileKind::Text | FileKind::Tileset => Tab::Code(rel.into()),
            FileKind::Scene | FileKind::Prefab => Tab::Scene(rel.into()),
            FileKind::Map if lower.ends_with(".tmj") => Tab::Map(rel.into()),
            FileKind::Map => Tab::Code(rel.into()),
            FileKind::Image => Tab::Image(rel.into()),
            // A aba da imagem (o .anim abre junto com ela, em open_file).
            FileKind::Anim => Tab::Code(rel.into()),
            FileKind::Sound => Tab::Sound(rel.into()),
            FileKind::Dialogue => Tab::Dialogue(rel.into()),
            FileKind::ModelSource => Tab::Model(rel.into()),
            // (O 3D no editor vem depois: por enquanto abre no programa padrão.)
            FileKind::Folder | FileKind::Font | FileKind::Model | FileKind::Other => return None,
        })
    }

    pub fn rel(&self) -> Option<&str> {
        match self {
            Tab::Code(r)
            | Tab::Image(r)
            | Tab::Sound(r)
            | Tab::Map(r)
            | Tab::Scene(r)
            | Tab::Dialogue(r)
            | Tab::Model(r) => Some(r),
            _ => None,
        }
    }

    fn set_rel(&mut self, new: String) {
        match self {
            Tab::Code(r)
            | Tab::Image(r)
            | Tab::Sound(r)
            | Tab::Map(r)
            | Tab::Scene(r)
            | Tab::Dialogue(r)
            | Tab::Model(r) => *r = new,
            _ => {}
        }
    }

    /// Abas que ficam na área central (documentos).
    pub fn is_document(&self) -> bool {
        !matches!(
            self,
            Tab::Files | Tab::Console | Tab::Hierarchy | Tab::Inspector | Tab::Events | Tab::Tiles
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
    let [_center, _console] = tree.split_below(center, 0.66, vec![Tab::Console, Tab::Events]);
    state
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewKind {
    Scene,
    VisualScene,
    Module,
    Folder,
    /// Imagem para o editor de pixel art.
    Image,
    /// Conversa para o editor de diálogos.
    Dialogue,
    /// Modelo 3D para o modelador.
    Model,
}

impl NewKind {
    fn title(self) -> &'static str {
        match self {
            NewKind::Scene => "Nova cena em código",
            NewKind::VisualScene => "Nova cena visual",
            NewKind::Module => "Novo script (módulo)",
            NewKind::Folder => "Nova pasta",
            NewKind::Image => "Nova imagem (pixel art)",
            NewKind::Dialogue => "Novo diálogo (conversa)",
            NewKind::Model => "Novo modelo 3D",
        }
    }

    fn default_name(self) -> &'static str {
        match self {
            NewKind::Scene | NewKind::VisualScene => "nova_cena",
            NewKind::Module => "util",
            NewKind::Folder => "nova_pasta",
            NewKind::Image => "desenho",
            NewKind::Dialogue => "conversa",
            NewKind::Model => "modelo",
        }
    }
}

/// O que os painéis pedem; tratado depois de desenhar.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Command(Command),
    /// Uma linha no Console (informação ou erro) e o aviso rápido (`toast`,
    /// ou o mesmo texto).
    Notice {
        text: String,
        toast: Option<String>,
        error: bool,
    },
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
    /// Criar um script de comportamento e pôr no objeto `id` da cena.
    NewBehaviorScript {
        scene: String,
        id: u32,
    },
    /// Transformar o objeto `id` da cena ativa num prefab, na pasta `dir`.
    CreatePrefab {
        id: u32,
        dir: Option<String>,
    },
    /// Comando para o jogo aberto ("pause", "resume", "step").
    GameCommand(&'static str),
    /// Gravar a paleta do projeto (`paleta.hex`).
    SavePalette,
    /// Dar o foco a esta aba (clicaram dentro dela).
    FocusTab(Tab),
    /// Criar um mapa de tiles novo na cena ativa.
    NewMap,
    /// Criar um efeito sonoro na pasta `parent`, ou ajustar o som `edit`
    /// (um `.wav` com o `.sfx` ao lado).
    NewSound {
        parent: String,
        edit: Option<String>,
    },
    /// Gravar uma variação do modelo `rel` (uma cópia) e abrir.
    ModelVariation {
        rel: String,
        data: crate::model_data::ModelData,
    },
    /// Criar uma imagem nova para o componente `ci` do objeto `id`.
    NewImageFor {
        scene: String,
        id: u32,
        ci: usize,
        /// Nome sugerido (o do objeto).
        name: String,
    },
}

enum Dialog {
    NewItem {
        kind: NewKind,
        parent: String,
        name: String,
        error: Option<String>,
        focus: bool,
        /// Imagem: largura e altura do quadro e quantos quadros.
        size: [u32; 3],
        /// Imagem pedida pelo Inspetor: (cena, objeto, componente) que a usa.
        assign: Option<(String, u32, usize)>,
        /// Cena visual que abre por cima de outra (pausa, opções).
        overlay: bool,
        /// Cena visual 3D.
        three_d: bool,
        /// Modelo 3D: de onde começa ("" = um cubo, "pronto:<i>" ou o
        /// caminho de um da biblioteca pessoal).
        model_from: String,
    },
    ConfirmDelete(String),
    CloseTab(String),
    Export(ExportDialog),
    /// Mapa de tiles novo, para pôr na cena (se houver).
    NewMap(crate::new_map::NewMapForm, Option<String>),
    /// Efeito sonoro novo (ou ajustar um).
    NewSound(Box<crate::new_sound::NewSoundForm>),
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
    /// Imagens abertas no editor de pixel art.
    pub pixels: HashMap<String, PixelDoc>,
    /// Diálogos abertos no editor de conversa.
    pub dialogues: HashMap<String, crate::dialogue::DialogueDoc>,
    /// Modelos 3D abertos no modelador.
    pub models: HashMap<String, crate::model_editor::ModelDoc>,
    /// A biblioteca pessoal do modelador (ao lado das preferências).
    pub library_dir: Option<PathBuf>,
    /// O modelo com o foco (o Inspetor mostra o resumo dele).
    active_model: Option<String>,
    /// Paleta do projeto (`paleta.hex`), lida na primeira vez.
    palette: Option<Vec<crate::pixel_ops::Color>>,
    sounds: HashMap<String, SoundView>,
    maps: HashMap<String, MapView>,
    /// Abas de mapas `.tmj` (o editor).
    map_editors: HashMap<String, crate::map_editor::MapEditor>,
    pub console: Console,
    pub game: Option<GameProcess>,
    /// Os testes de jogo rodando (Jogo > Conferir com os testes).
    tests: Option<std::sync::mpsc::Receiver<TestMsg>>,
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
    events: crate::events_view::EventsPanel,
    /// Cena que o jogo aberto está mostrando (para o ajuste ao vivo).
    pub game_scene: Option<String>,
    /// O que o jogo aberto conta da cena da frente.
    pub game_state: Option<crate::scene_view::GameState>,
    /// Valores do config.lua (tamanho da tela e cor de fundo).
    config: ConfigValues,
    /// Para o autocompletar: script → nomes dos objetos das cenas que o
    /// usam. Refeito quando alguma cena muda.
    script_objects: Option<HashMap<String, Vec<String>>>,
    started: std::time::Instant,
    /// Os avisos rápidos do canto da janela.
    pub toasts: crate::toast::Toasts,
    /// Quando cada arquivo foi salvo (a aba mostra um ✓ um instante).
    saved_at: HashMap<String, f64>,
    /// Sons da interface para tocar no próximo quadro (se ligados).
    pending_sounds: Vec<&'static str>,
    ui_sound_ids: HashMap<&'static str, nn_audio::SoundId>,
    playing_ui: Vec<nn_audio::Sound>,
    /// O confete de uma comemoração (há quanto tempo começou, em segundos).
    confetti: Option<f32>,
    /// A exportação terminou bem (a comemoração vem no próximo quadro).
    exported_ok: bool,
    /// Quantas vezes o jogo foi aberto (▶ Jogar ou Testar daqui).
    pub plays: u64,
    /// Imagens criadas com "Nova imagem" (o tutorial confere).
    pub created_images: Vec<String>,
    /// Mapas abertos para pintar (na cena ou na aba do mapa).
    pub map_docs: HashMap<String, crate::map_doc::MapDoc>,
    /// Tilesets `.tsj` desses mapas.
    pub tilesets: HashMap<String, crate::tileset::TilesetDoc>,
    /// O pincel (ferramenta e tiles), do painel Tiles.
    pub paint: crate::paint::PaintState,
    tiles_panel: crate::tiles_view::TilesPanel,
    /// O mapa que o painel Tiles mostra.
    pub tiles_map: Option<String>,
    /// Jogo > Ligar um agente de IA (aberta).
    pub agent_dialog: Option<crate::agent::AgentDialog>,
    /// O editor atendendo o servidor MCP (o agente de IA).
    agent_link: Option<crate::agent::EditorLink>,
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
            pixels: HashMap::new(),
            dialogues: HashMap::new(),
            models: HashMap::new(),
            library_dir: None,
            active_model: None,
            palette: None,
            sounds: HashMap::new(),
            maps: HashMap::new(),
            map_editors: HashMap::new(),
            console: Console::default(),
            game: None,
            tests: None,
            form: SettingsForm::default(),
            lua: mlua::Lua::new(),
            dialog: None,
            agent_dialog: None,
            agent_link: None,
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
            events: Default::default(),
            game_scene: None,
            game_state: None,
            config: ConfigValues::default(),
            script_objects: None,
            started: std::time::Instant::now(),
            toasts: Default::default(),
            saved_at: HashMap::new(),
            pending_sounds: Vec::new(),
            ui_sound_ids: HashMap::new(),
            playing_ui: Vec::new(),
            confetti: None,
            exported_ok: false,
            plays: 0,
            created_images: Vec::new(),
            map_docs: HashMap::new(),
            tilesets: HashMap::new(),
            paint: Default::default(),
            tiles_panel: Default::default(),
            tiles_map: None,
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
            // Começa num título: a fase também abre (e fica na frente).
            if let Some(fase) = ["cenas/fase.cena", "cenas/mapa.cena", "cenas/jogo.cena"]
                .into_iter()
                .find(|r| *r != start && ws.root.join(r).is_file())
            {
                ws.open_file(fase, None);
            }
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
            .chain(
                self.pixels
                    .values()
                    .filter(|s| s.is_dirty())
                    .map(|s| s.rel.clone()),
            )
            .chain(
                self.map_docs
                    .values()
                    .filter(|s| s.is_dirty())
                    .map(|s| s.rel.clone()),
            )
            .chain(
                self.dialogues
                    .values()
                    .filter(|s| s.is_dirty())
                    .map(|s| s.rel.clone()),
            )
            .chain(
                self.models
                    .values()
                    .filter(|s| s.is_dirty())
                    .map(|s| s.rel.clone()),
            )
            .chain(
                self.tilesets
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
            || self.pixels.get(rel).is_some_and(PixelDoc::is_dirty)
            || self
                .dialogues
                .get(rel)
                .is_some_and(crate::dialogue::DialogueDoc::is_dirty)
            || self
                .models
                .get(rel)
                .is_some_and(crate::model_editor::ModelDoc::is_dirty)
            || self
                .map_docs
                .get(rel)
                .is_some_and(crate::map_doc::MapDoc::is_dirty)
            || self
                .tilesets
                .get(rel)
                .is_some_and(crate::tileset::TilesetDoc::is_dirty)
    }

    /// O que o autocompletar precisa saber para o arquivo `rel`.
    fn completion_env(&mut self, rel: &str) -> crate::complete::Env {
        use crate::complete::{Env, ScriptKind};
        let map = self.script_objects.get_or_insert_with(|| {
            let mut map: HashMap<String, Vec<String>> = HashMap::new();
            for e in self.files.files_of(FileKind::Scene) {
                let Ok(text) = std::fs::read_to_string(self.root.join(&e.rel)) else {
                    continue;
                };
                let Ok(scene) = crate::scene_data::read(&text, &e.rel) else {
                    continue;
                };
                if scene.script.is_empty() {
                    continue;
                }
                let names = map.entry(scene.script.clone()).or_default();
                crate::scene_data::visit(&scene.objects, &mut |o| names.push(o.name.clone()));
            }
            map
        });
        let mut objects = map.get(rel).cloned().unwrap_or_default();
        let mut scene_script = map.contains_key(rel);
        // Cenas abertas (talvez ainda não salvas) também contam.
        for doc in self.scenes.values() {
            if doc.data.script == rel {
                scene_script = true;
                crate::scene_data::visit(&doc.data.objects, &mut |o| objects.push(o.name.clone()));
            }
        }
        objects.sort();
        objects.dedup();
        let text = self.docs.get(rel).map(|d| d.text.as_str()).unwrap_or("");
        let kind = if text.contains("props") && !scene_script {
            ScriptKind::Behavior
        } else if scene_script {
            ScriptKind::SceneScript
        } else {
            ScriptKind::Other
        };
        Env {
            files: self
                .files
                .entries()
                .iter()
                .filter(|e| !e.is_dir)
                .map(|e| e.rel.clone())
                .collect(),
            objects,
            vars: self.config.vars.iter().map(|(k, _)| k.clone()).collect(),
            kind,
        }
    }

    /// Esquece as alterações não salvas (antes de fechar sem salvar).
    pub fn discard_changes(&mut self) {
        self.docs.clear();
        self.scenes.clear();
        self.pixels.clear();
        self.dialogues.clear();
        self.models.clear();
        self.map_docs.clear();
        self.tilesets.clear();
    }

    /// Relê o config.lua (tamanho da tela e cor de fundo da cena).
    fn reload_config(&mut self) {
        let text = std::fs::read_to_string(self.root.join("config.lua")).unwrap_or_default();
        self.config = config_lua::read(&text, &self.title).unwrap_or_default();
        crate::scene_data::set_project_ui(self.config.theme.clone(), self.config.vars.clone());
        let (w, h) = self.config.game_size();
        crate::scene_data::set_screen_size(w as f64, h as f64);
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
        for img in self.pixels.values_mut().filter(|s| s.is_dirty()) {
            match img.save(&self.root) {
                Ok(()) => saved.push(img.rel.clone()),
                Err(e) => errors.push(format!("{}: {e}", img.rel)),
            }
        }
        for d in self.dialogues.values_mut().filter(|s| s.is_dirty()) {
            match d.save(&self.root) {
                Ok(()) => saved.push(d.rel.clone()),
                Err(e) => errors.push(format!("{}: {e}", d.rel)),
            }
        }
        let mut models = Vec::new();
        for d in self.models.values_mut().filter(|s| s.is_dirty()) {
            match d.save(&self.root) {
                Ok(()) => {
                    saved.push(d.rel.clone());
                    models.push(d.rel.clone());
                }
                Err(e) => errors.push(format!("{}: {e}", d.rel)),
            }
        }
        for rel in models {
            self.model_exported(&rel);
        }
        let (ok, err) = self.save_maps(None);
        saved.extend(ok);
        errors.extend(err);
        saved.sort();
        for rel in saved {
            self.saved(&rel);
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
            Tab::Tiles => {
                // Junto do Inspetor (na frente dele).
                match self.dock.find_tab(&Tab::Inspector) {
                    Some(path) => {
                        if let Ok(leaf) = self.dock.leaf_mut(path.node_path()) {
                            leaf.append_tab(Tab::Tiles);
                        }
                    }
                    None => {
                        self.dock.main_surface_mut().split_right(
                            NodeIndex::root(),
                            0.74,
                            vec![Tab::Tiles],
                        );
                    }
                }
                if let Some(path) = self.dock.find_tab(&Tab::Tiles) {
                    let _ = self.dock.set_active_tab(path);
                }
            }
            Tab::Events => {
                // Junto do Console, se ele estiver aberto.
                match self.dock.find_tab(&Tab::Console) {
                    Some(path) => {
                        if let Ok(leaf) = self.dock.leaf_mut(path.node_path()) {
                            leaf.append_tab(Tab::Events);
                        }
                    }
                    None => {
                        self.dock.main_surface_mut().split_below(
                            NodeIndex::root(),
                            0.66,
                            vec![Tab::Events],
                        );
                    }
                }
                if let Some(path) = self.dock.find_tab(&Tab::Events) {
                    let _ = self.dock.set_active_tab(path);
                    self.dock.set_focused_node_and_surface(path.node_path());
                }
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
                self.pixels.remove(r);
            }
            Tab::Dialogue(r) => {
                self.dialogues.remove(r);
            }
            Tab::Model(r) => {
                self.models.remove(r);
                if let Some(sr) = &mut self.scene_renderer {
                    sr.forget(r);
                }
            }
            Tab::Sound(r) => {
                if let Some(mut s) = self.sounds.remove(r) {
                    s.stop();
                }
            }
            Tab::Map(r) => {
                self.maps.remove(r);
                self.map_editors.remove(r);
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
        // Eventos: junto do Console, sem tirar o Console da frente.
        if self.dock.find_tab(&Tab::Events).is_none()
            && let Some(path) = self.dock.find_tab(&Tab::Console)
            && let Ok(leaf) = self.dock.leaf_mut(path.node_path())
        {
            let active = leaf.active;
            leaf.append_tab(Tab::Events);
            leaf.active = active;
        }
    }

    /// Abre a janela "Novo som" (como Arquivos > ➕ Novo > Som), com os sons
    /// na pasta `dir`.
    pub fn new_sound_dialog(&mut self, dir: &str) {
        self.dialog = Some(Dialog::NewSound(Box::new(
            crate::new_sound::NewSoundForm::new(&self.files, dir),
        )));
    }

    /// Abre um arquivo na aba certa (ou no programa padrão).
    pub fn open_file(&mut self, rel: &str, line: Option<usize>) {
        // As animações abrem na aba da imagem delas.
        if FileKind::of(Path::new(rel)) == FileKind::Anim
            && line.is_none()
            && let Some(img) = self.image_of_anim(rel)
        {
            self.open_file(&img, None);
            return;
        }
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
        let is_map = matches!(tab, Tab::Map(_));
        self.show_tab(tab);
        if is_map {
            self.tiles_map = Some(rel.to_string());
            if self.dock.find_tab(&Tab::Tiles).is_none() {
                self.show_tab(Tab::Tiles);
                // A aba do mapa continua com o foco.
                if let Some(path) = self.dock.find_tab(&Tab::Map(rel.to_string())) {
                    self.dock.set_focused_node_and_surface(path.node_path());
                }
            } else if let Some(path) = self.dock.find_tab(&Tab::Tiles) {
                let _ = self.dock.set_active_tab(path);
            }
        }
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
            Some((_, Tab::Hierarchy | Tab::Inspector | Tab::Events | Tab::Tiles)) => active,
            _ => None,
        }
    }

    /// Mapa que recebe desfazer: o da aba do mapa com foco, ou o que está
    /// sendo pintado na cena com foco (ou do painel Tiles).
    fn focused_map(&mut self) -> Option<String> {
        if let Some((_, Tab::Map(r))) = self.dock.find_active_focused() {
            let r = r.clone();
            return self.map_docs.contains_key(&r).then_some(r);
        }
        let scene = self.focused_scene()?;
        let doc = self.scenes.get(&scene)?;
        let id = doc.painting?;
        let rel = doc.data.find(id)?.component("map")?.str("map").to_string();
        self.map_docs.contains_key(&rel).then_some(rel)
    }

    /// Desfaz (ou refaz) no mapa; se ele tinha crescido à esquerda ou
    /// embaixo, o objeto do mapa na cena volta junto.
    fn undo_map(&mut self, rel: &str, undo: bool) {
        let Some(map) = self.map_docs.get_mut(rel) else {
            return;
        };
        if undo {
            map.undo();
        } else {
            map.redo();
        }
        let (dc, dr) = map.take_shift();
        let (tw, th) = (map.tile_w as f64, map.tile_h as f64);
        if (dc, dr) == (0, 0) {
            return;
        }
        for doc in self.scenes.values_mut() {
            let Some(id) = doc.painting else {
                continue;
            };
            let Some(scale) = doc
                .data
                .find(id)
                .and_then(|o| o.component("map"))
                .filter(|c| c.str("map") == rel)
                .map(|c| c.num("scale").max(0.01))
            else {
                continue;
            };
            doc.begin_frame();
            if let Some(o) = doc.data.find_mut(id) {
                o.x -= dc as f64 * tw * scale;
                o.y -= dr as f64 * th * scale;
            }
            doc.end_frame();
            break;
        }
        self.flush_live();
    }

    /// A imagem (PNG) de um arquivo `.anim`, se existir.
    fn image_of_anim(&self, rel: &str) -> Option<String> {
        let base = rel
            .strip_suffix(".anim")
            .or_else(|| rel.strip_suffix(".ANIM"))?;
        ["png", "PNG"]
            .iter()
            .map(|e| format!("{base}.{e}"))
            .find(|p| self.root.join(p).is_file())
    }

    /// Modelo com o foco (recebe Desfazer e Salvar).
    fn focused_model(&mut self) -> Option<String> {
        match self.dock.find_active_focused() {
            Some((_, Tab::Model(r))) => Some(r.clone()),
            _ => None,
        }
    }

    /// Grava uma cópia do modelo (`casinha 2.modelo`, com o `.glb`) e abre.
    fn model_variation(&mut self, rel: &str, data: &crate::model_data::ModelData) {
        let base = rel.strip_suffix(".modelo").unwrap_or(rel);
        // "casinha 2" vira "casinha 3", e não "casinha 2 2".
        let base = base
            .rsplit_once(' ')
            .filter(|(_, n)| n.parse::<u32>().is_ok())
            .map_or(base, |(b, _)| b);
        let new = (2..)
            .map(|n| format!("{base} {n}.modelo"))
            .find(|r| {
                !self.root.join(r).exists()
                    && !self.root.join(crate::model_data::glb_path(r)).exists()
            })
            .expect("nome livre");
        let result = std::fs::write(self.root.join(&new), data.to_lua()).and_then(|()| {
            std::fs::write(
                self.root.join(crate::model_data::glb_path(&new)),
                data.to_glb(),
            )
        });
        match result {
            Ok(()) => {
                self.files.rescan();
                let name = new.rsplit('/').next().unwrap_or(&new).to_string();
                self.console.info(format!("Variação criada: {new}"));
                self.toasts
                    .push(format!("🔀 {name} criado"), crate::toast::Kind::Success);
                self.open_file(&new, None);
            }
            Err(e) => self
                .console
                .error(format!("Não foi possível criar a variação: {e}")),
        }
    }

    /// O `.glb` de um modelo foi gravado: as cenas abertas leem de novo.
    fn model_exported(&mut self, rel: &str) {
        let glb = crate::model_data::glb_path(rel);
        self.sizes.invalidate(&glb);
        if let Some(sr) = &mut self.scene_renderer {
            sr.invalidate(&self.root, &glb);
        }
    }

    /// Diálogo com o foco (recebe Desfazer e Salvar).
    fn focused_dialogue(&mut self) -> Option<String> {
        match self.dock.find_active_focused() {
            Some((_, Tab::Dialogue(r))) => Some(r.clone()),
            _ => None,
        }
    }

    /// Imagem com o foco (recebe Desfazer e Salvar).
    fn focused_image(&mut self) -> Option<String> {
        match self.dock.find_active_focused() {
            Some((_, Tab::Image(r))) => Some(r.clone()),
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

    /// Um marco do projeto (`what`: `jogar` ou `exportar`): na primeira vez,
    /// um aviso de comemoração com confete (se as comemorações estão ligadas).
    fn celebrate(&mut self, env: &mut Env, what: &str, text: &str) {
        let key = format!("{what}:{}", self.root.display());
        if !env.settings.celebrated.insert(key) || !env.settings.celebrations {
            return;
        }
        self.toasts.push(text, crate::toast::Kind::Celebrate);
        self.confetti = Some(0.0);
        self.pending_sounds.push("comemorar");
    }

    /// Toca os sons da interface pedidos (se ligados nas Preferências).
    fn play_ui_sounds(&mut self, env: &mut Env) {
        let pending = std::mem::take(&mut self.pending_sounds);
        if !env.settings.ui_sounds || pending.is_empty() {
            return;
        }
        let audio = env.audio.get_or_insert_with(nn_audio::Audio::new);
        self.playing_ui.retain(|s| s.is_playing());
        let mut done: Vec<&str> = Vec::new();
        for kind in pending {
            // Um de cada tipo por quadro (colar 10 objetos é um clique só).
            if done.contains(&kind) {
                continue;
            }
            done.push(kind);
            let id = *self.ui_sound_ids.entry(kind).or_insert_with(|| {
                let (rate, samples) = crate::ui_sounds::samples(kind);
                audio.add_samples(rate, &samples)
            });
            self.playing_ui.push(audio.play(id));
        }
    }

    /// Roda o jogo; com `start`, começando nessa cena em vez da inicial.
    pub fn play_from(&mut self, ctx: &egui::Context, env: &mut Env, start: Option<&str>) {
        self.plays += 1;
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
                self.celebrate(
                    env,
                    "jogar",
                    "🎉 Seu jogo rodou pela primeira vez! Agora é só ir melhorando.",
                );
            }
            Err(e) => self
                .console
                .error(format!("Não foi possível iniciar o jogo: {e}")),
        }
    }

    /// Roda os testes de jogo (`testes/*.lua`) sem janela, em segundo plano;
    /// o resultado vai para o Console.
    pub fn run_tests(&mut self, ctx: &egui::Context) {
        if self.tests.is_some() {
            self.console.warn("Os testes já estão rodando.");
            return;
        }
        let _ = self.save_all();
        let root = self.root.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let wake = ctx.clone();
        self.console
            .info("🔬 Rodando os testes do jogo (os arquivos de testes/), sem janela...");
        self.show_tab(Tab::Console);
        std::thread::spawn(move || {
            let send = |m: TestMsg| {
                let _ = tx.send(m);
                wake.request_repaint();
            };
            let project = match nn_script::Project::open(&root) {
                Ok(p) => p,
                Err(e) => {
                    send(TestMsg::Done(Err(e.to_string())));
                    return;
                }
            };
            let opts = nn_script::testing::TestOptions::default();
            let result = nn_script::testing::run_tests(&project, &opts, |r| {
                send(TestMsg::Result(test_line(r)))
            });
            send(TestMsg::Done(result.map(|s| {
                let mut lines: Vec<(bool, String)> = s
                    .file_errors
                    .iter()
                    .map(|e| (true, format!("✖ {} não carregou:\n{}", e.file, e.message)))
                    .collect();
                let total = s.results.len();
                lines.push((
                    !s.ok(),
                    if total == 0 && s.file_errors.is_empty() {
                        "Este projeto ainda não tem testes. Crie um arquivo .lua em testes/ com \
                         test(\"nome\", function(t) ... end): veja \"Testes de jogo\" no Guia do Lua (F1)."
                            .to_string()
                    } else {
                        format!(
                            "🔬 {total} {}: {} {}, {} {}.",
                            if total == 1 { "teste" } else { "testes" },
                            s.passed(),
                            if s.passed() == 1 { "passou" } else { "passaram" },
                            s.failed(),
                            if s.failed() == 1 { "falhou" } else { "falharam" },
                        )
                    },
                ));
                lines
            })));
        });
        self.tests = Some(rx);
    }

    fn poll_tests(&mut self) {
        let Some(rx) = &self.tests else {
            return;
        };
        let msgs: Vec<TestMsg> = rx.try_iter().collect();
        for m in msgs {
            match m {
                TestMsg::Result((false, text)) => self.console.info(text),
                TestMsg::Result((true, text)) => self.console.error(text),
                TestMsg::Done(Ok(lines)) => {
                    for (error, text) in lines {
                        if error {
                            self.console.error(text);
                        } else {
                            self.console.info(text);
                        }
                    }
                    self.tests = None;
                }
                TestMsg::Done(Err(e)) => {
                    self.console
                        .error(format!("Não foi possível rodar os testes: {e}"));
                    self.tests = None;
                }
            }
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
            use crate::console::LiveMessage;
            match crate::console::live_message(&line) {
                Some(LiveMessage::Scene(scene)) => self.game_scene = Some(scene),
                Some(LiveMessage::State(v)) => {
                    self.game_state = crate::scene_view::GameState::parse(&v);
                    if let Some(g) = &self.game_state {
                        self.events.flash(&g.scene, &g.fired);
                    }
                }
                Some(LiveMessage::Select(id)) => {
                    // Ctrl+clique no jogo: seleciona o objeto aqui.
                    if let Some(scene) = self.game_scene.clone()
                        && let Some(doc) = self.scenes.get_mut(&scene)
                        && doc.data.find(id).is_some()
                    {
                        doc.selection = vec![id];
                        self.active_scene = Some(scene);
                    }
                }
                None => self.console.push_game_line(&line),
            }
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
            self.game_state = None;
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
            if let Some(v) = self.pixels.get_mut(rel) {
                v.disk_changed(&root);
            }
            if let Some(v) = self.dialogues.get_mut(rel) {
                v.disk_changed(&root);
            }
            if let Some(v) = self.models.get_mut(rel) {
                v.disk_changed(&root);
            }
            // O .anim mudou fora: a imagem dele relê (se não houver nada a perder).
            if FileKind::of(Path::new(rel)) == FileKind::Anim {
                self.sizes.anims.forget(rel);
                for v in self.pixels.values_mut() {
                    if crate::anim_file::anim_path(&v.rel) == *rel {
                        v.disk_changed(&root);
                    }
                }
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
            if let Some(map) = self.map_docs.get_mut(rel) {
                map.disk_changed(&root);
            }
            if let Some(ts) = self.tilesets.get(rel)
                && !ts.is_dirty()
                && let Ok(new) = crate::tileset::TilesetDoc::open(&root, rel)
                && new.text() != ts.text()
            {
                self.tilesets.insert(rel.clone(), new);
            }
            if matches!(
                FileKind::of(Path::new(rel)),
                FileKind::Scene | FileKind::Prefab
            ) {
                self.script_objects = None;
            }
            if FileKind::of(Path::new(rel)) == FileKind::Prefab {
                // As cópias do prefab nas cenas abertas passam a usar o novo.
                for (other, doc) in self.scenes.iter_mut() {
                    if other != rel && (doc.data.uses_prefab(rel) || doc.saved_uses_prefab(rel)) {
                        doc.relink_prefabs(&root);
                    }
                }
            }
            if FileKind::of(Path::new(rel)) == FileKind::Model {
                // Um .glb mudou (exportado de fora, ou pelo modelador).
                self.sizes.invalidate(rel);
                if let Some(sr) = &mut self.scene_renderer {
                    sr.invalidate(&root, rel);
                }
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
            Command::RunTests => self.run_tests(ctx),
            Command::Undo | Command::Redo => {
                if let Some(map_rel) = self.focused_map() {
                    self.undo_map(&map_rel, cmd == Command::Undo);
                } else if let Some(rel) = self.focused_dialogue()
                    && let Some(doc) = self.dialogues.get_mut(&rel)
                {
                    if cmd == Command::Undo {
                        doc.undo();
                    } else {
                        doc.redo();
                    }
                } else if let Some(rel) = self.focused_model()
                    && let Some(doc) = self.models.get_mut(&rel)
                {
                    if cmd == Command::Undo {
                        doc.undo();
                    } else {
                        doc.redo();
                    }
                } else if let Some(rel) = self.focused_image()
                    && let Some(doc) = self.pixels.get_mut(&rel)
                {
                    if cmd == Command::Undo {
                        doc.undo();
                    } else {
                        doc.redo();
                    }
                } else if let Some(rel) = self.focused_scene()
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
                if let Some((_, Tab::Map(r))) = self.dock.find_active_focused() {
                    let r = r.clone();
                    self.save_doc(&r);
                } else if let Some(rel) = self.focused_image() {
                    self.save_doc(&rel);
                } else if let Some(rel) = self.focused_dialogue() {
                    self.save_doc(&rel);
                } else if let Some(rel) = self.focused_model() {
                    self.save_doc(&rel);
                } else if let Some(rel) = self.focused_scene() {
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
                        Tab::Code(r)
                        | Tab::Scene(r)
                        | Tab::Image(r)
                        | Tab::Dialogue(r)
                        | Tab::Model(r)
                            if self.is_dirty(r) =>
                        {
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
            Command::ShowEvents => self.show_tab(Tab::Events),
            Command::ShowTiles => self.show_tab(Tab::Tiles),
            Command::ClearConsole => self.console.clear(),
            Command::ConnectAgent => self.agent_dialog = Some(Default::default()),
            Command::NewSound => self.actions.push(Action::NewSound {
                parent: String::new(),
                edit: None,
            }),
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

    /// Guarda um objeto da cena ativa (com os filhos) num `.prefab` e
    /// transforma o objeto numa instância dele.
    fn create_prefab(&mut self, id: u32, dir: Option<String>) {
        let Some(scene) = self.active_scene.clone() else {
            return;
        };
        let Some(obj) = self
            .scenes
            .get(&scene)
            .and_then(|d| d.data.find(id))
            .cloned()
        else {
            return;
        };
        if obj.prefab.is_some() {
            self.console
                .warn(format!("\"{}\" já é um prefab.", obj.name));
            return;
        }
        let dir = dir
            .filter(|d| !d.is_empty())
            .unwrap_or_else(|| "prefabs".into());
        let slug: String = crate::commands::fold(&obj.name)
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let slug = if slug.trim_matches('_').is_empty() {
            "objeto".to_string()
        } else {
            slug
        };
        let mut rel = files::join_rel(&dir, &format!("{slug}.prefab"));
        let mut n = 2;
        while self.root.join(&rel).exists() {
            rel = files::join_rel(&dir, &format!("{slug}_{n}.prefab"));
            n += 1;
        }
        // O prefab começa no (0, 0), com ids próprios.
        let mut base = obj.clone();
        base.x = 0.0;
        base.y = 0.0;
        let mut next = 1;
        fn renumber(o: &mut crate::scene_data::Object, next: &mut u32) {
            o.id = *next;
            *next += 1;
            for c in &mut o.children {
                renumber(c, next);
            }
        }
        renumber(&mut base, &mut next);
        let full = self.root.join(&rel);
        if let Some(parent) = full.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(e) = crate::files::write_atomic(&full, &crate::scene_data::write_prefab(&base)) {
            self.console
                .error(format!("Não foi possível criar {rel}: {e}"));
            return;
        }
        self.files.rescan();
        self.file_panel.reveal(&rel);
        if let Some(doc) = self.scenes.get_mut(&scene) {
            doc.begin_frame();
            doc.record(None);
            let removed: Vec<u32> = {
                let mut v = Vec::new();
                crate::scene_data::visit(&obj.children, &mut |c| v.push(c.id));
                v
            };
            if let Some(o) = doc.data.find_mut(id) {
                o.children.clear();
                o.prefab = Some(crate::scene_data::PrefabLink {
                    path: rel.clone(),
                    base: None,
                    raw: Vec::new(),
                });
                o.link_prefab(base);
            }
            doc.selection.retain(|s| !removed.contains(s));
            doc.end_frame();
        }
        self.console.info(format!(
            "Prefab criado: {rel}. Arraste-o do painel Arquivos para pôr cópias nas cenas."
        ));
        self.toasts.push(
            format!("📦 Prefab criado: {rel}"),
            crate::toast::Kind::Success,
        );
    }

    /// Cria `scripts/<nome do objeto>.lua` com o modelo de comportamento e
    /// põe no objeto.
    fn new_behavior_script(&mut self, scene: &str, id: u32) {
        let Some(name) = self
            .scenes
            .get(scene)
            .and_then(|d| d.data.find(id))
            .map(|o| o.name.clone())
        else {
            return;
        };
        let slug: String = crate::commands::fold(&name)
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let slug = if slug.trim_matches('_').is_empty() {
            "comportamento".to_string()
        } else {
            slug
        };
        let mut rel = format!("scripts/{slug}.lua");
        let mut n = 2;
        while self.root.join(&rel).exists() {
            rel = format!("scripts/{slug}_{n}.lua");
            n += 1;
        }
        let full = self.root.join(&rel);
        if let Some(dir) = full.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(&full, crate::behaviors::template(&name)) {
            self.console
                .error(format!("Não foi possível criar {rel}: {e}"));
            return;
        }
        self.console.info(format!("Criado: {rel}"));
        self.files.rescan();
        let info = self.inspector.catalog.get(&self.root, &rel);
        if let Some(doc) = self.scenes.get_mut(scene) {
            doc.begin_frame();
            doc.record(None);
            if let Some(o) = doc.data.find_mut(id) {
                o.components.push(crate::scene_data::Component::behavior(
                    &rel,
                    &info.defaults(),
                ));
            }
            doc.end_frame();
        }
        self.open_file(&rel, None);
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

    /// Salva os mapas (todos, ou só estes) com alterações e os tilesets deles.
    fn save_maps(&mut self, only: Option<Vec<String>>) -> (Vec<String>, Vec<String>) {
        let mut saved = Vec::new();
        let mut errors = Vec::new();
        let mut tilesets: Vec<String> = Vec::new();
        for map in self.map_docs.values_mut() {
            if only.as_ref().is_some_and(|o| !o.contains(&map.rel)) {
                continue;
            }
            tilesets.extend(map.external_tilesets());
            if !map.is_dirty() {
                continue;
            }
            match map.save(&self.root) {
                Ok(()) => saved.push(map.rel.clone()),
                Err(e) => errors.push(format!("{}: {e}", map.rel)),
            }
        }
        for ts in self.tilesets.values_mut() {
            if only.is_some() && !tilesets.contains(&ts.rel) || !ts.is_dirty() {
                continue;
            }
            match ts.save(&self.root) {
                Ok(()) => saved.push(ts.rel.clone()),
                Err(e) => errors.push(format!("{}: {e}", ts.rel)),
            }
        }
        if !saved.is_empty() {
            self.paint_grew_reset();
        }
        (saved, errors)
    }

    fn save_maps_logged(&mut self, only: Option<Vec<String>>) {
        let (saved, errors) = self.save_maps(only);
        for r in &saved {
            self.console.info(format!("Salvo: {r}"));
        }
        for e in errors {
            self.console.error(format!("Não foi possível salvar {e}"));
        }
        if !saved.is_empty() {
            self.files.rescan();
        }
    }

    /// Um arquivo foi salvo: o Console, o aviso rápido, o ✓ na aba e o som.
    fn saved(&mut self, rel: &str) {
        self.console.info(format!("Salvo: {rel}"));
        let name = rel.rsplit('/').next().unwrap_or(rel);
        self.toasts
            .push(format!("Salvo: {name}"), crate::toast::Kind::Success);
        self.saved_at
            .insert(rel.to_string(), self.started.elapsed().as_secs_f64());
        self.pending_sounds.push("salvar");
    }

    /// Salvou os mapas: o jogo aberto relê, o aviso "salve para o jogo ver" sai.
    fn paint_grew_reset(&mut self) {
        for s in self.scenes.values_mut() {
            s.paint_grew = false;
        }
    }

    fn save_doc(&mut self, rel: &str) {
        if let Some(d) = self.models.get_mut(rel) {
            match d.save(&self.root) {
                Ok(()) => {
                    self.saved(rel);
                    self.model_exported(rel);
                    self.files.rescan();
                }
                Err(e) => self
                    .console
                    .error(format!("Não foi possível salvar {rel}: {e}")),
            }
            return;
        }
        if let Some(d) = self.dialogues.get_mut(rel) {
            match d.save(&self.root) {
                Ok(()) => {
                    self.saved(rel);
                    self.files.rescan();
                }
                Err(e) => self
                    .console
                    .error(format!("Não foi possível salvar {rel}: {e}")),
            }
            return;
        }
        if let Some(img) = self.pixels.get_mut(rel) {
            match img.save(&self.root) {
                Ok(()) => {
                    self.saved(rel);
                    self.files.rescan();
                }
                Err(e) => self
                    .console
                    .error(format!("Não foi possível salvar {rel}: {e}")),
            }
            return;
        }
        if self.map_docs.contains_key(rel) && !self.scenes.contains_key(rel) {
            self.save_maps_logged(Some(vec![rel.to_string()]));
            return;
        }
        if let Some(scene) = self.scenes.get_mut(rel) {
            // Os mapas pintados nesta cena salvam junto.
            let maps = scene_maps(&scene.data);
            match scene.save(&self.root) {
                Ok(()) => {
                    self.saved(rel);
                    self.files.rescan();
                }
                Err(e) => self
                    .console
                    .error(format!("Não foi possível salvar {rel}: {e}")),
            }
            self.save_maps_logged(Some(maps));
            return;
        }
        let Some(doc) = self.docs.get_mut(rel) else {
            return;
        };
        match doc.save(&self.root) {
            Ok(()) => {
                doc.check_now(&self.lua);
                let issue = doc.issue.clone();
                self.saved(rel);
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
                        size: [16, 16, 1],
                        assign: None,
                        overlay: false,
                        three_d: false,
                        model_from: String::new(),
                    });
                }
                Action::NewImageFor {
                    scene,
                    id,
                    ci,
                    name,
                } => {
                    self.dialog = Some(Dialog::NewItem {
                        kind: NewKind::Image,
                        parent: crate::file_panel::image_dir(&self.files, ""),
                        name: files::safe_name(&name),
                        error: None,
                        focus: true,
                        size: [16, 16, 1],
                        assign: Some((scene, id, ci)),
                        overlay: false,
                        three_d: false,
                        model_from: String::new(),
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
                Action::Notice { text, toast, error } => {
                    let short = toast.unwrap_or_else(|| text.clone());
                    if error {
                        self.console.error(&text);
                        self.toasts.push(short, crate::toast::Kind::Error);
                    } else {
                        self.console.info(&text);
                        self.toasts.push(short, crate::toast::Kind::Success);
                    }
                }
                Action::NewSound { parent, edit } => match edit {
                    Some(rel) => match crate::new_sound::NewSoundForm::open(&self.root, &rel) {
                        Ok(f) => self.dialog = Some(Dialog::NewSound(Box::new(f))),
                        Err(e) => self
                            .console
                            .error(format!("Não deu para abrir os números do som: {e}")),
                    },
                    None => {
                        self.dialog = Some(Dialog::NewSound(Box::new(
                            crate::new_sound::NewSoundForm::new(&self.files, &parent),
                        )));
                    }
                },
                Action::NewMap => {
                    self.dialog = Some(Dialog::NewMap(
                        crate::new_map::NewMapForm::new(&self.files),
                        self.active_scene.clone(),
                    ));
                }
                Action::ModelVariation { rel, data } => self.model_variation(&rel, &data),
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
                Action::NewBehaviorScript { scene, id } => self.new_behavior_script(&scene, id),
                Action::CreatePrefab { id, dir } => self.create_prefab(id, dir),
                Action::FocusTab(tab) => {
                    if let Some(path) = self.dock.find_tab(&tab) {
                        self.dock.set_focused_node_and_surface(path.node_path());
                    }
                }
                Action::SavePalette => {
                    let text: String = self
                        .palette
                        .iter()
                        .flatten()
                        .map(|c| crate::pixel_ops::to_hex(*c) + "\n")
                        .collect();
                    let path = self.root.join(crate::pixel_editor::PROJECT_PALETTE);
                    match std::fs::write(&path, text) {
                        Ok(()) => {
                            self.files.rescan();
                        }
                        Err(e) => self.console.error(format!(
                            "Não foi possível salvar {}: {e}",
                            crate::pixel_editor::PROJECT_PALETTE
                        )),
                    }
                }
                Action::GameCommand(cmd) => {
                    if let Some(g) = &mut self.game {
                        g.send(&serde_json::json!({ "cmd": cmd }).to_string());
                    }
                }
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
                let models: Vec<String> = self.models.keys().cloned().collect();
                for k in models {
                    if let Some(n) = fix(&k)
                        && let Some(mut d) = self.models.remove(&k)
                    {
                        d.rel = n.clone();
                        self.models.insert(n, d);
                    }
                }
                let dialogues: Vec<String> = self.dialogues.keys().cloned().collect();
                for k in dialogues {
                    if let Some(n) = fix(&k)
                        && let Some(mut d) = self.dialogues.remove(&k)
                    {
                        d.rel = n.clone();
                        self.dialogues.insert(n, d);
                    }
                }
                let pixels: Vec<String> = self.pixels.keys().cloned().collect();
                for k in pixels {
                    if let Some(n) = fix(&k)
                        && let Some(mut d) = self.pixels.remove(&k)
                    {
                        d.rel = n.clone();
                        self.pixels.insert(n, d);
                    }
                }
                // Visualizadores abrem de novo com o caminho novo.
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
        self.poll_tests();
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
        // Autocompletar: só para o código com o foco.
        let mut code_env = None;
        if let Some(rel) = self.focused_code.clone()
            && rel.ends_with(".lua")
            && self.docs.contains_key(&rel)
        {
            code_env = Some((rel.clone(), self.completion_env(&rel)));
        }
        {
            let vars: Vec<String> = self.config.vars.iter().map(|(k, _)| k.clone()).collect();
            let mut viewer = Viewer {
                root: &self.root,
                title: &self.title,
                files: &self.files,
                file_panel: &mut self.file_panel,
                docs: &mut self.docs,
                pixels: &mut self.pixels,
                dialogues: &mut self.dialogues,
                models: &mut self.models,
                library: self.library_dir.as_deref(),
                active_model: self.active_model.as_deref(),
                map_docs: &mut self.map_docs,
                tilesets: &mut self.tilesets,
                paint: &mut self.paint,
                tiles_panel: &mut self.tiles_panel,
                tiles_map: &mut self.tiles_map,
                palette: self.palette.get_or_insert_with(|| {
                    std::fs::read_to_string(self.root.join(crate::pixel_editor::PROJECT_PALETTE))
                        .map(|t| crate::pixel_ops::parse_hex_list(&t))
                        .unwrap_or_default()
                }),
                sounds: &mut self.sounds,
                maps: &mut self.maps,
                map_editors: &mut self.map_editors,
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
                events: &mut self.events,
                vars: &vars,
                game_scene: self.game_scene.as_deref(),
                design: {
                    let (w, h) = self.config.game_size();
                    (f64::from(w), f64::from(h))
                },
                clear_color: &self.config.clear_color,
                time: self.started.elapsed().as_secs_f32(),
                code_env: code_env.as_ref(),
                game_state: self.game_state.as_ref().filter(|g| {
                    self.game.is_some() && self.game_scene.as_deref() == Some(g.scene.as_str())
                }),
                saved_at: &self.saved_at,
                now: self.started.elapsed().as_secs_f64(),
                animate: env.settings.animations,
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
        // A Hierarquia e o Inspetor seguem a cena (ou o modelo) com o foco.
        match self.dock.find_active_focused() {
            Some((_, Tab::Scene(r))) => {
                let r = r.clone();
                if self.scenes.contains_key(&r) {
                    self.active_scene = Some(r);
                }
                self.active_model = None;
            }
            Some((_, Tab::Model(r))) => self.active_model = Some(r.clone()),
            _ => {}
        }
        if self
            .active_model
            .as_ref()
            .is_some_and(|r| !self.models.contains_key(r))
        {
            self.active_model = None;
        }
        // Avisos das cenas (apagou, colou...), com Desfazer, e os sons.
        for (rel, doc) in self.scenes.iter_mut() {
            let version = doc.history.version();
            for text in doc.notices.drain(..) {
                self.toasts.push_undo(
                    text,
                    crate::toast::Undo {
                        rel: rel.clone(),
                        version,
                    },
                );
            }
            self.pending_sounds.append(&mut doc.sounds);
        }
        if std::mem::take(&mut self.exported_ok) {
            self.celebrate(
                env,
                "exportar",
                "🎉 Seu primeiro jogo exportado! Agora dá para mandar para os amigos.",
            );
            self.pending_sounds.push("salvar");
        }
        let scenes = &self.scenes;
        if let Some(u) = self.toasts.ui(
            &ctx,
            |rel| scenes.get(rel).map(|s| s.history.version()),
            env.settings.animations,
        ) && let Some(doc) = self.scenes.get_mut(&u.rel)
        {
            doc.undo();
        }
        if env.settings.animations {
            draw_confetti(&ctx, &mut self.confetti);
        } else {
            self.confetti = None;
        }
        self.play_ui_sounds(env);
        for doc in self.scenes.values_mut() {
            doc.end_frame();
        }
        self.flush_live();

        if hovering {
            drop_overlay(&ctx, self.file_panel.target_dir(&self.files));
        }
        self.dialogs(&ctx, env.audio);
        if let Some(d) = &mut self.agent_dialog {
            let connected = self.agent_link.as_ref().is_some_and(|l| l.listening());
            if !crate::agent::dialog_ui(&ctx, d, &self.root, env.exe, connected) {
                self.agent_dialog = None;
            }
        }
        self.serve_agent(&ctx);
        self.handle_actions(&ctx, env)
    }

    /// Atende o servidor MCP (o agente de IA): o estado do editor e o que
    /// ele avisa que mudou.
    fn serve_agent(&mut self, ctx: &egui::Context) {
        if self.agent_link.is_none() {
            self.agent_link = Some(crate::agent::EditorLink::start(&self.root, ctx));
        }
        let snapshot = self.agent_snapshot();
        let notes = match &mut self.agent_link {
            Some(link) => link.update(ctx, snapshot),
            None => Vec::new(),
        };
        for n in notes {
            let text = match n.text.lines().next() {
                Some(first) if !first.is_empty() => format!("Agente ({}): {first}", n.tool),
                _ => format!("Agente: {}", n.tool),
            };
            self.console.agent(text, n.files);
        }
    }

    /// O que o agente de IA vê do editor (`ver_editor`).
    pub fn agent_snapshot(&mut self) -> serde_json::Value {
        use serde_json::json;
        let focused = self.dock.find_active_focused().map(|(_, t)| t.clone());
        // A aba de documento com o foco; com um painel em foco (Hierarquia,
        // Inspetor...), a cena que ele mostra.
        let front = match &focused {
            Some(t) if t.rel().is_some() => t.rel().map(str::to_string),
            _ => self
                .active_scene
                .clone()
                .or_else(|| self.focused_code.clone()),
        };
        let kind = |rel: &str| match FileKind::of(Path::new(rel)) {
            FileKind::Scene => "cena",
            FileKind::Prefab => "prefab",
            FileKind::Lua => "código Lua",
            FileKind::Image => "imagem",
            FileKind::Map => "mapa",
            FileKind::Sound => "som",
            FileKind::Dialogue => "diálogo",
            FileKind::ModelSource => "modelo 3D",
            _ => "arquivo",
        };
        let tabs: Vec<String> = self
            .dock
            .iter_all_tabs()
            .filter_map(|(_, t)| t.rel().map(str::to_string))
            .collect();
        let scene_rel = front
            .as_ref()
            .filter(|r| self.scenes.contains_key(r.as_str()))
            .cloned()
            .or_else(|| self.active_scene.clone());
        let scene = scene_rel.as_ref().and_then(|r| self.scenes.get(r)).map(|doc| {
            let selected: Vec<serde_json::Value> = doc
                .selection
                .iter()
                .filter_map(|id| doc.data.find(*id))
                .map(|o| {
                    let mut v = json!({
                        "id": o.id, "nome": o.name, "x": o.x, "y": o.y,
                        "componentes": o.components.iter().map(|c| {
                            if c.kind == crate::scene_data::BEHAVIOR { c.str("script").to_string() } else { c.kind.clone() }
                        }).collect::<Vec<_>>(),
                    });
                    if doc.data.three_d {
                        v["z"] = json!(o.z);
                    }
                    if let Some(p) = &o.prefab {
                        v["prefab"] = json!(p.path);
                    }
                    v
                })
                .collect();
            json!({
                "arquivo": doc.rel,
                "3d": doc.data.three_d,
                "objetos": doc.data.objects.len(),
                "escolhidos": selected,
                "erro": doc.error,
            })
        });
        json!({
            "projeto": self.title,
            "pasta": self.root,
            "aba": front.as_ref().map(|r| json!({ "arquivo": r, "tipo": kind(r) })),
            "abas": tabs,
            "cena": scene,
            "nao_gravados": self.dirty_files(),
            "jogo_aberto": self.game.is_some(),
        })
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

    fn dialogs(&mut self, ctx: &egui::Context, audio: &mut Option<nn_audio::Audio>) {
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
                size,
                assign,
                overlay,
                three_d,
                model_from,
            } => {
                let kind = *kind;
                let library = self
                    .library_dir
                    .as_deref()
                    .map(crate::model_kits::user_models)
                    .unwrap_or_default();
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
                        file_name.push_str(match kind {
                            NewKind::VisualScene => ".cena",
                            NewKind::Image => ".png",
                            NewKind::Dialogue => ".dialogo",
                            NewKind::Model => ".modelo",
                            _ => ".lua",
                        });
                    }
                    if kind == NewKind::Image {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label("Tamanho:");
                            ui.add(egui::DragValue::new(&mut size[0]).range(1..=1024).suffix(" px"));
                            ui.label("×");
                            ui.add(egui::DragValue::new(&mut size[1]).range(1..=1024).suffix(" px"));
                            for s in crate::pixel_editor::FRAME_SIZES {
                                if ui.small_button(s.to_string()).clicked() {
                                    *size = [s, s, size[2]];
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Quadros:");
                            ui.add(egui::DragValue::new(&mut size[2]).range(1..=64));
                            ui.weak("(mais de um: uma folha para animar; dá para mudar depois)");
                        });
                    }
                    let rel = files::join_rel(parent, &file_name);
                    ui.weak(format!("Será criado: {rel}"));
                    if kind == NewKind::Scene {
                        ui.weak("A cena já vem com um quadrado que anda com WASD, as setas ou o controle.");
                    }
                    if kind == NewKind::VisualScene {
                        ui.horizontal(|ui| {
                            ui.label("Tipo:");
                            let r2 = ui.radio_value(three_d, false, "2D");
                            let r3 = ui
                                .radio_value(three_d, true, "3D")
                                .on_hover_text("Um mundo 3D: modelos .glb, formas, céu, sol e sombras. Já vem com câmera, chão e um cubo.");
                            crate::tutorial::anchor_in(ui, "novo:2d", r2.rect);
                            crate::tutorial::anchor_in(ui, "novo:3d", r3.rect);
                        });
                    }
                    if kind == NewKind::VisualScene && *three_d {
                        ui.weak("Arraste modelos .glb do painel Arquivos para a cena, ou use ➕ Objeto. Botão direito + WASD voa pela cena.");
                    } else if kind == NewKind::VisualScene {
                        let r = ui
                            .checkbox(overlay, "Abre por cima de outra cena (pausa, opções)")
                            .on_hover_text("A cena de baixo continua aparecendo e fica parada até esta fechar. Já vem com um menu: Continuar e Sair.");
                        crate::tutorial::anchor_in(ui, "novo:por-cima", r.rect);
                        if *overlay {
                            ui.weak("Já vem com um painel e botões. Abra ela com o evento Abrir cena por cima, ou com um botão.");
                        } else {
                            ui.weak("Monte a cena arrastando imagens, mapas e sons para ela. Se precisar de código, crie o script dela no Inspetor.");
                        }
                    }
                    if kind == NewKind::Image {
                        ui.weak("Abre no editor de pixel art. 16 × 16 é um bom tamanho para personagens e itens.");
                    }
                    if kind == NewKind::Model {
                        let label = |from: &str| -> String {
                            if let Some(i) = from.strip_prefix("pronto:").and_then(|i| i.parse::<usize>().ok()) {
                                crate::model_kits::READY.get(i).map_or("?", |r| r.name).to_string()
                            } else if from.is_empty() {
                                "Um cubo".to_string()
                            } else {
                                format!("⭐ {}", Path::new(from).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default())
                            }
                        };
                        ui.horizontal(|ui| {
                            ui.label("Começar com:");
                            let before = model_from.clone();
                            let r = egui::ComboBox::from_id_salt("novo-modelo-de")
                                .height(420.0)
                                .selected_text(label(model_from))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(model_from, String::new(), "Um cubo");
                                    ui.separator();
                                    for (i, r) in crate::model_kits::READY.iter().enumerate() {
                                        ui.selectable_value(model_from, format!("pronto:{i}"), r.name);
                                    }
                                    if !library.is_empty() {
                                        ui.separator();
                                    }
                                    for (n, p) in &library {
                                        ui.selectable_value(model_from, p.to_string_lossy().into_owned(), format!("⭐ {n}"));
                                    }
                                });
                            crate::tutorial::anchor_in(ui, "novo:modelo-de", r.response.rect);
                            // O nome acompanha o modelo escolhido (se ainda não foi mudado).
                            if *model_from != before {
                                let was = if before.is_empty() { "modelo".to_string() } else { files::safe_name(&label(&before).replace("⭐ ", "")) };
                                if name.trim() == was || name.trim() == "modelo" {
                                    *name = if model_from.is_empty() { "modelo".into() } else { files::safe_name(&label(model_from).replace("⭐ ", "")) };
                                }
                            }
                        });
                        ui.weak("Abre no modelador. Ao salvar, o .glb com o mesmo nome sai junto: é ele que vai para as cenas.");
                    }
                    if kind == NewKind::Dialogue {
                        ui.weak("Abre no editor de conversa, já com um exemplo: falas, escolhas e um personagem. Na cena, use o comportamento Diálogo.");
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
                                NewKind::VisualScene => {
                                    let text = if *three_d {
                                        files::scene3d_template()
                                    } else if *overlay {
                                        files::overlay_scene_template()
                                    } else {
                                        files::visual_scene_template()
                                    };
                                    files::create_file(&self.root, parent, &file_name, &text)
                                }
                                NewKind::Module => files::create_file(&self.root, parent, name, &files::module_template(&rel)),
                                NewKind::Dialogue => files::create_file(&self.root, parent, &file_name, &crate::dialogue::template()),
                                NewKind::Model => {
                                    let model = if let Some(i) = model_from.strip_prefix("pronto:").and_then(|i| i.parse::<usize>().ok()) {
                                        crate::model_kits::READY.get(i).map(|r| (r.make)())
                                    } else if model_from.is_empty() {
                                        None
                                    } else {
                                        crate::model_kits::read_user(Path::new(model_from.as_str())).ok()
                                    };
                                    create_model(&self.root, parent, &file_name, model.unwrap_or_else(crate::model_data::ModelData::new_default))
                                }
                                NewKind::Image => {
                                    let [w, h, n] = *size;
                                    create_image(&self.root, &rel, w, h, n)
                                }
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
                    if kind == NewKind::Image {
                        self.created_images.push(rel.clone());
                    }
                    if let Some((scene, id, ci)) = assign.take() {
                        let [w, h, n] = *size;
                        if let Some(doc) = self.scenes.get_mut(&scene) {
                            doc.begin_frame();
                            doc.record(None);
                            if let Some(c) =
                                doc.data.find_mut(id).and_then(|o| o.components.get_mut(ci))
                            {
                                c.set("image", crate::scene_data::Prop::Str(rel.clone()));
                                c.set(
                                    "frame_w",
                                    crate::scene_data::Prop::Num(if n > 1 {
                                        w as f64
                                    } else {
                                        0.0
                                    }),
                                );
                                c.set(
                                    "frame_h",
                                    crate::scene_data::Prop::Num(if n > 1 {
                                        h as f64
                                    } else {
                                        0.0
                                    }),
                                );
                                c.set("frame", crate::scene_data::Prop::Num(1.0));
                                // Pixel art pequena aparece maior na cena.
                                if c.num("w") <= 0.0 && w <= 64 {
                                    c.set("w", crate::scene_data::Prop::Num(w as f64 * 4.0));
                                    c.set("h", crate::scene_data::Prop::Num(h as f64 * 4.0));
                                }
                            }
                            doc.end_frame();
                        }
                    }
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
            Dialog::NewSound(form) => {
                let mut done = None;
                let response =
                    egui::Modal::new(egui::Id::new("dialogo-novo-som")).show(ctx, |ui| {
                        let (c, cancel) = form.ui(ui, &self.files, audio);
                        done = c;
                        close |= cancel;
                    });
                close |= response.should_close();
                if let Some(c) = done {
                    close = true;
                    if c.replaced {
                        self.console.info(format!("Som ajustado: {}", c.wav));
                        if let Some(v) = self.sounds.get_mut(&c.wav) {
                            v.reload();
                        }
                    } else {
                        self.console
                            .info(format!("Criado: {} (os números em {})", c.wav, c.sfx));
                        after = Some(Action::OpenFile {
                            rel: c.wav.clone(),
                            line: None,
                        });
                    }
                    self.files.rescan();
                }
            }
            Dialog::NewMap(form, scene) => {
                let mut done = None;
                let response =
                    egui::Modal::new(egui::Id::new("dialogo-novo-mapa")).show(ctx, |ui| {
                        let (c, cancel) = form.ui(ui, &self.files);
                        done = c;
                        close |= cancel;
                    });
                close |= response.should_close();
                if let Some(c) = done {
                    close = true;
                    for f in &c.files {
                        self.console.info(format!("Criado: {f}"));
                    }
                    self.files.rescan();
                    match scene.as_deref().and_then(|r| self.scenes.get_mut(r)) {
                        Some(doc) => {
                            let name = c
                                .map
                                .rsplit('/')
                                .next()
                                .unwrap_or("mapa")
                                .trim_end_matches(".tmj")
                                .to_string();
                            let (cx, cy) = doc.view_center();
                            let mut comp = crate::scene_data::Component::new("map");
                            comp.set("map", crate::scene_data::Prop::Str(c.map.clone()));
                            comp.set("scale", crate::scene_data::Prop::Num(c.scale));
                            doc.begin_frame();
                            let id = doc.add_object(
                                &name,
                                cx - c.world.0 / 2.0,
                                cy - c.world.1 / 2.0,
                                vec![comp],
                            );
                            if let Some(o) = doc.data.find_mut(id) {
                                o.layer = -1;
                            }
                            doc.end_frame();
                            doc.painting = Some(id);
                            self.tiles_map = Some(c.map.clone());
                            after = Some(Action::Command(Command::ShowTiles));
                        }
                        None => {
                            after = Some(Action::OpenFile {
                                rel: c.map.clone(),
                                line: None,
                            });
                        }
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
                        Ok(r) => {
                            self.console.info(format!(
                                "Jogo exportado: {} ({})",
                                crate::settings::display_path(&r.output),
                                files::human_size(r.total_bytes)
                            ));
                            self.exported_ok = true;
                        }
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

/// Lista das variáveis do jogo (`vars` do config.lua). Devolve `false` se
/// algum nome for inválido ou repetido.
/// Os nomes das variáveis são válidos e não se repetem?
fn vars_valid(vars: &[(String, crate::scene_data::Prop)]) -> bool {
    vars.iter().all(|(name, _)| {
        config_lua::valid_var_name(name) && vars.iter().filter(|(n, _)| n == name).count() == 1
    })
}

fn vars_editor(ui: &mut egui::Ui, vars: &mut Vec<(String, crate::scene_data::Prop)>) -> bool {
    use crate::scene_data::Prop;
    ui.label(RichText::new("Variáveis do jogo").strong());
    ui.weak("Valem de uma cena para outra (nn.vars.nome). Coletável e Vida mudam elas; Mostrar valor mostra na tela.");
    ui.add_space(4.0);
    let mut ok = true;
    let mut remove = None;
    let names: Vec<String> = vars.iter().map(|(k, _)| k.clone()).collect();
    egui::Grid::new("grade-vars")
        .num_columns(4)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            for (i, (name, value)) in vars.iter_mut().enumerate() {
                let valid = config_lua::valid_var_name(name)
                    && names.iter().filter(|n| *n == name).count() == 1;
                ok &= valid;
                let r = ui.add(
                    egui::TextEdit::singleline(name)
                        .desired_width(140.0)
                        .text_color_opt((!valid).then_some(theme::ERROR)),
                );
                if !valid {
                    r.on_hover_text("Use letras, números e _ (sem espaço, sem começar com número), sem repetir.");
                }
                let kind = match value {
                    Prop::Num(_) => 0,
                    Prop::Str(_) => 1,
                    Prop::Bool(_) => 2,
                };
                let mut new_kind = kind;
                egui::ComboBox::from_id_salt(("tipo-var", i))
                    .width(90.0)
                    .selected_text(["número", "texto", "sim/não"][kind])
                    .show_ui(ui, |ui| {
                        for (k, l) in ["número", "texto", "sim/não"].iter().enumerate() {
                            ui.selectable_value(&mut new_kind, k, *l);
                        }
                    });
                if new_kind != kind {
                    *value = match new_kind {
                        0 => Prop::Num(0.0),
                        1 => Prop::Str(String::new()),
                        _ => Prop::Bool(false),
                    };
                }
                match value {
                    Prop::Num(n) => {
                        ui.add(egui::DragValue::new(n).speed(1.0));
                    }
                    Prop::Str(t) => {
                        ui.add(egui::TextEdit::singleline(t).desired_width(140.0));
                    }
                    Prop::Bool(b) => {
                        ui.checkbox(b, "");
                    }
                }
                if ui.small_button("✖").on_hover_text("Tirar").clicked() {
                    remove = Some(i);
                }
                ui.end_row();
            }
        });
    if let Some(i) = remove {
        vars.remove(i);
    }
    if ui.button("➕ Variável").clicked() {
        let mut n = 1;
        let mut name = "pontos".to_string();
        while vars.iter().any(|(k, _)| *k == name) {
            n += 1;
            name = format!("pontos_{n}");
        }
        vars.push((name, Prop::Num(0.0)));
    }
    ok
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

/// O tamanho de quadro que as cenas e prefabs usam para a imagem `rel`.
fn frame_hint(
    root: &Path,
    files: &FileTree,
    open: &HashMap<String, SceneDoc>,
    rel: &str,
) -> Option<(u32, u32)> {
    fn find_in(objects: &[crate::scene_data::Object], rel: &str) -> Option<(u32, u32)> {
        let mut found = None;
        crate::scene_data::visit(objects, &mut |o| {
            for c in &o.components {
                if found.is_none()
                    && c.kind == "sprite"
                    && c.str("image") == rel
                    && c.num("frame_w") > 0.0
                    && c.num("frame_h") > 0.0
                {
                    found = Some((c.num("frame_w") as u32, c.num("frame_h") as u32));
                }
            }
        });
        found
    }
    if let Some(f) = open.values().find_map(|d| find_in(&d.data.objects, rel)) {
        return Some(f);
    }
    for e in files
        .files_of(FileKind::Scene)
        .chain(files.files_of(FileKind::Prefab))
    {
        let Ok(text) = std::fs::read_to_string(root.join(&e.rel)) else {
            continue;
        };
        if !text.contains(rel) {
            continue;
        }
        let found = if e.rel.ends_with(".prefab") {
            crate::scene_data::read_prefab(&text, &e.rel)
                .ok()
                .and_then(|o| find_in(std::slice::from_ref(&o), rel))
        } else {
            crate::scene_data::read(&text, &e.rel)
                .ok()
                .and_then(|s| find_in(&s.objects, rel))
        };
        if found.is_some() {
            return found;
        }
    }
    None
}

/// Cria um PNG transparente (e o `.anim` com o tamanho do quadro, se tiver
/// mais de um quadro).
/// O Inspetor com um modelo do modelador na frente: o resumo dele.
fn model_summary(ui: &mut egui::Ui, doc: &crate::model_editor::ModelDoc) {
    let name = doc.rel.rsplit('/').next().unwrap_or(&doc.rel);
    ui.heading(name);
    ui.weak(&doc.rel);
    ui.add_space(6.0);
    let d = &doc.data;
    egui::Grid::new("inspetor-modelo")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            ui.label("Peças");
            ui.label(d.parts.len().to_string());
            ui.end_row();
            ui.label("Triângulos");
            ui.label(d.triangle_count().to_string());
            ui.end_row();
            if let Some((lo, hi)) = d.bounds() {
                let s = hi - lo;
                let f =
                    |v: f32| crate::scene_data::format_num(f64::from((v * 100.0).round() / 100.0));
                ui.label("Tamanho");
                ui.label(format!("{} × {} × {} m", f(s.x), f(s.y), f(s.z)));
                ui.end_row();
            }
            ui.label("Ossos");
            ui.label(if d.bones.is_empty() {
                "nenhum".to_string()
            } else {
                d.bones.len().to_string()
            });
            ui.end_row();
            ui.label("Animações");
            ui.label(if d.anims.is_empty() {
                "nenhuma".to_string()
            } else {
                d.anims
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            });
            ui.end_row();
        });
    ui.add_space(8.0);
    ui.label(format!(
        "Ao salvar, o {} sai junto: é ele que vai para as cenas e para o jogo.",
        crate::model_data::glb_path(name)
    ));
    ui.add_space(4.0);
    ui.weak("A peça escolhida (posição, rotação, escala e osso) fica no painel da direita do modelador.");
}

/// Um modelo novo (um cubo), já com o `.glb` dele.
fn create_model(
    root: &Path,
    parent: &str,
    name: &str,
    model: crate::model_data::ModelData,
) -> Result<String, String> {
    let glb = crate::model_data::glb_path(&files::join_rel(parent, name.trim()));
    if root.join(&glb).exists() {
        return Err(format!("Já existe {glb}."));
    }
    let rel = files::create_file(root, parent, name, &model.to_lua())?;
    std::fs::write(root.join(&glb), model.to_glb()).map_err(|e| e.to_string())?;
    Ok(rel)
}

fn create_image(root: &Path, rel: &str, w: u32, h: u32, frames: u32) -> Result<String, String> {
    let path = root.join(rel);
    if path.exists() {
        return Err(format!("Já existe um arquivo {rel}."));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let (img, anim) = crate::pixel_editor::blank(w, h, frames);
    img.save_with_format(&path, image::ImageFormat::Png)
        .map_err(|e| format!("Não foi possível criar a imagem: {e}"))?;
    if frames > 1 {
        let anim_rel = crate::anim_file::anim_path(rel);
        std::fs::write(root.join(&anim_rel), anim.write(rel)).map_err(|e| e.to_string())?;
    }
    Ok(rel.to_string())
}

/// Desenha cada aba.
/// Um pedido da Hierarquia (ou do menu do objeto na vista da cena).
fn hierarchy_action(actions: &mut Vec<Action>, a: HierarchyAction) {
    match a {
        HierarchyAction::CreatePrefab { id, dir } => {
            actions.push(Action::CreatePrefab { id, dir });
        }
        HierarchyAction::OpenFile(rel) => {
            actions.push(Action::OpenFile { rel, line: None });
        }
        HierarchyAction::NewMap => actions.push(Action::NewMap),
        HierarchyAction::ShowHierarchy => {
            actions.push(Action::Command(Command::ShowHierarchy));
        }
        HierarchyAction::Saved(rel) => {
            let name = std::path::Path::new(&rel)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            actions.push(Action::Notice {
                text: format!(
                    "⭐ Guardado nos favoritos: {rel}. Ele aparece no topo do ➕ Objeto e no Shift+A."
                ),
                toast: Some(format!("{name} guardado nos favoritos ⭐")),
                error: false,
            });
        }
        HierarchyAction::Error(text) => actions.push(Action::Notice {
            text,
            toast: None,
            error: true,
        }),
        HierarchyAction::AskDelete(rel) => actions.push(Action::AskDelete(rel)),
    }
}

struct Viewer<'a> {
    root: &'a Path,
    title: &'a str,
    files: &'a FileTree,
    file_panel: &'a mut FilePanel,
    docs: &'a mut HashMap<String, CodeDoc>,
    pixels: &'a mut HashMap<String, PixelDoc>,
    dialogues: &'a mut HashMap<String, crate::dialogue::DialogueDoc>,
    models: &'a mut HashMap<String, crate::model_editor::ModelDoc>,
    library: Option<&'a Path>,
    active_model: Option<&'a str>,
    palette: &'a mut Vec<crate::pixel_ops::Color>,
    map_docs: &'a mut HashMap<String, crate::map_doc::MapDoc>,
    tilesets: &'a mut HashMap<String, crate::tileset::TilesetDoc>,
    paint: &'a mut crate::paint::PaintState,
    tiles_panel: &'a mut crate::tiles_view::TilesPanel,
    tiles_map: &'a mut Option<String>,
    sounds: &'a mut HashMap<String, SoundView>,
    maps: &'a mut HashMap<String, MapView>,
    map_editors: &'a mut HashMap<String, crate::map_editor::MapEditor>,
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
    events: &'a mut crate::events_view::EventsPanel,
    /// Variáveis do jogo (config.lua).
    vars: &'a [String],
    game_scene: Option<&'a str>,
    design: (f64, f64),
    clear_color: &'a str,
    time: f32,
    /// Autocompletar do código com o foco: (arquivo, o que ele sabe).
    code_env: Option<&'a (String, crate::complete::Env)>,
    /// O que o jogo aberto conta da cena da frente.
    game_state: Option<&'a crate::scene_view::GameState>,
    /// Quando cada arquivo foi salvo, e a hora agora (a aba mostra um ✓).
    saved_at: &'a HashMap<String, f64>,
    now: f64,
    /// As animações estão ligadas (Preferências).
    animate: bool,
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
            Tab::Events => "Eventos".to_string(),
            Tab::Tiles => "Tiles".to_string(),
            Tab::Code(r)
            | Tab::Image(r)
            | Tab::Sound(r)
            | Tab::Map(r)
            | Tab::Scene(r)
            | Tab::Dialogue(r)
            | Tab::Model(r) => {
                let name = r.rsplit('/').next().unwrap_or(r).to_string();
                if self.docs.get(r.as_str()).is_some_and(CodeDoc::is_dirty)
                    || self.scenes.get(r.as_str()).is_some_and(SceneDoc::is_dirty)
                    || self.pixels.get(r.as_str()).is_some_and(PixelDoc::is_dirty)
                    || self
                        .dialogues
                        .get(r.as_str())
                        .is_some_and(crate::dialogue::DialogueDoc::is_dirty)
                    || self
                        .models
                        .get(r.as_str())
                        .is_some_and(crate::model_editor::ModelDoc::is_dirty)
                    || self
                        .map_docs
                        .get(r.as_str())
                        .is_some_and(crate::map_doc::MapDoc::is_dirty)
                    || self.scenes.get(r.as_str()).is_some_and(|s| {
                        scene_maps(&s.data).iter().any(|m| {
                            self.map_docs.get(m).is_some_and(|d| {
                                d.is_dirty()
                                    || d.external_tilesets()
                                        .iter()
                                        .any(|t| self.tilesets.get(t).is_some_and(|t| t.is_dirty()))
                            })
                        })
                    })
                {
                    format!("{name} •")
                } else if self
                    .saved_at
                    .get(r.as_str())
                    .is_some_and(|t| self.now - t < 1.2)
                {
                    // Acabou de salvar: um ✔ no lugar do •.
                    format!("{name} ✔")
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
            Tab::Dialogue(r)
                if self
                    .dialogues
                    .get(r.as_str())
                    .is_some_and(crate::dialogue::DialogueDoc::is_dirty) =>
            {
                self.actions.push(Action::AskCloseTab(r.clone()));
                OnCloseResponse::Ignore
            }
            Tab::Dialogue(r) => {
                self.dialogues.remove(r.as_str());
                OnCloseResponse::Close
            }
            Tab::Model(r)
                if self
                    .models
                    .get(r.as_str())
                    .is_some_and(crate::model_editor::ModelDoc::is_dirty) =>
            {
                self.actions.push(Action::AskCloseTab(r.clone()));
                OnCloseResponse::Ignore
            }
            Tab::Model(r) => {
                self.models.remove(r.as_str());
                if let Some(sr) = self.scene_renderer.as_mut() {
                    sr.forget(r);
                }
                OnCloseResponse::Close
            }
            Tab::Image(r) if self.pixels.get(r.as_str()).is_some_and(PixelDoc::is_dirty) => {
                self.actions.push(Action::AskCloseTab(r.clone()));
                OnCloseResponse::Ignore
            }
            Tab::Image(r) => {
                self.pixels.remove(r.as_str());
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
                self.map_editors.remove(r.as_str());
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
                match self.console.ui(ui, self.mode.is_full(), self.clear_on_play) {
                    Some(ConsoleAction::OpenLocation { path, line }) => {
                        self.actions.push(Action::OpenLocation { path, line });
                    }
                    Some(ConsoleAction::Play) => self.actions.push(Action::Command(Command::Play)),
                    Some(ConsoleAction::OpenFile(rel)) => {
                        self.actions.push(Action::OpenFile { rel, line: None });
                    }
                    None => {}
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
                let env = self
                    .code_env
                    .filter(|(r, _)| r == rel.as_str())
                    .map(|(_, e)| e);
                for a in doc.ui(ui, self.lua, self.zoom_code, env) {
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
                if !self.pixels.contains_key(rel.as_str()) {
                    let mut doc = PixelDoc::open(self.root, rel);
                    if let Some((w, h)) = frame_hint(self.root, self.files, self.scenes, rel) {
                        doc.hint_frame_size(w, h);
                    }
                    self.pixels.insert(rel.clone(), doc);
                }
                let doc = self.pixels.get_mut(rel.as_str()).expect("imagem aberta");
                // Clicar na imagem dá o foco à aba (Ctrl+Z, Ctrl+S vêm para cá).
                if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect())
                {
                    self.actions.push(Action::FocusTab(Tab::Image(rel.clone())));
                }
                for a in doc.ui(ui, self.palette) {
                    match a {
                        PixelAction::Save => self.actions.push(Action::SaveDoc(rel.clone())),
                        PixelAction::CopyText(t) => self.actions.push(Action::CopyText(t)),
                        PixelAction::SavePalette => self.actions.push(Action::SavePalette),
                    }
                }
            }
            Tab::Dialogue(rel) => {
                if !self.dialogues.contains_key(rel.as_str()) {
                    let doc = crate::dialogue::DialogueDoc::open(self.root, rel);
                    self.dialogues.insert(rel.clone(), doc);
                }
                if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect())
                {
                    self.actions
                        .push(Action::FocusTab(Tab::Dialogue(rel.clone())));
                }
                let env = crate::dialogue::DialogueEnv {
                    images: self
                        .files
                        .files_of(FileKind::Image)
                        .map(|e| e.rel.clone())
                        .collect(),
                    sounds: self
                        .files
                        .files_of(FileKind::Sound)
                        .map(|e| e.rel.clone())
                        .collect(),
                };
                let doc = self
                    .dialogues
                    .get_mut(rel.as_str())
                    .expect("diálogo aberto");
                for a in doc.ui(ui, &env) {
                    match a {
                        crate::dialogue::DialogueAction::Save => {
                            self.actions.push(Action::SaveDoc(rel.clone()))
                        }
                    }
                }
            }
            Tab::Model(rel) => {
                if !self.models.contains_key(rel.as_str()) {
                    let doc = crate::model_editor::ModelDoc::open(self.root, rel);
                    self.models.insert(rel.clone(), doc);
                }
                if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect())
                {
                    self.actions.push(Action::FocusTab(Tab::Model(rel.clone())));
                }
                let mut env = crate::model_editor::ModelEnv {
                    renderer: self.scene_renderer.as_mut(),
                    root: self.root,
                    library: self.library,
                };
                let doc = self.models.get_mut(rel.as_str()).expect("modelo aberto");
                for a in doc.ui(ui, &mut env) {
                    match a {
                        crate::model_editor::ModelAction::Save => {
                            self.actions.push(Action::SaveDoc(rel.clone()))
                        }
                        crate::model_editor::ModelAction::Notice(text) => {
                            self.actions.push(Action::Notice {
                                text,
                                toast: None,
                                error: false,
                            })
                        }
                        crate::model_editor::ModelAction::Variation(data) => {
                            self.actions.push(Action::ModelVariation {
                                rel: rel.clone(),
                                data,
                            })
                        }
                    }
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
                // O editor; se o mapa não abrir aqui (base64, infinito...),
                // fica a prévia com o motivo.
                match crate::paint::ensure_open(self.map_docs, self.tilesets, self.root, rel) {
                    Ok(()) => {
                        let ed = self.map_editors.entry(rel.clone()).or_default();
                        for a in ed.ui(ui, self.root, rel, self.map_docs, self.tilesets, self.paint)
                        {
                            match a {
                                crate::map_editor::MapAction::CopyText(t) => {
                                    self.actions.push(Action::CopyText(t))
                                }
                                crate::map_editor::MapAction::OpenExternal(r) => {
                                    self.actions.push(Action::OpenExternal(r))
                                }
                                crate::map_editor::MapAction::Focus => {
                                    *self.tiles_map = Some(rel.clone());
                                    self.actions.push(Action::FocusTab(Tab::Map(rel.clone())));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        ui.colored_label(
                            theme::WARNING,
                            format!("Não dá para editar este mapa aqui: {e}"),
                        );
                        let view = self.maps.entry(rel.clone()).or_default();
                        if let Some(a) = view.ui(ui, self.root, rel) {
                            self.viewer_action(a);
                        }
                    }
                }
            }
            Tab::Scene(rel) => self.scene(ui, rel),
            Tab::Hierarchy => {
                let doc = self
                    .active_scene
                    .as_deref()
                    .and_then(|r| self.scenes.get_mut(r));
                for a in self.hierarchy.ui(ui, doc, self.root) {
                    hierarchy_action(self.actions, a);
                }
            }
            Tab::Events => {
                let doc = self
                    .active_scene
                    .as_deref()
                    .and_then(|r| self.scenes.get_mut(r));
                let anim_files: Vec<String> = self
                    .files
                    .files_of(FileKind::Anim)
                    .map(|e| e.rel.clone())
                    .collect();
                let mut anims = self.sizes.anims.all_names(self.root, &anim_files);
                // Numa cena 3D: as animações dos modelos dela.
                if let Some(d) = doc.as_ref().filter(|d| d.data.three_d) {
                    let mut files: Vec<String> = Vec::new();
                    crate::scene_data::visit(&d.data.objects, &mut |o| {
                        if let Some(m) = o.component("model")
                            && !files.contains(&m.str("file").to_string())
                        {
                            files.push(m.str("file").to_string());
                        }
                    });
                    anims.clear();
                    for f in files {
                        for a in self.sizes.model_anims(self.root, &f) {
                            if !anims.contains(&a) {
                                anims.push(a);
                            }
                        }
                    }
                }
                // Os tiles dos mapas da cena (para as regras de tiles).
                let maps = doc
                    .as_ref()
                    .map(|d| scene_maps(&d.data))
                    .unwrap_or_default();
                let mut tile_kinds: Vec<String> = Vec::new();
                let mut tile_sheet = None;
                for m in &maps {
                    if crate::paint::ensure_open(self.map_docs, self.tilesets, self.root, m)
                        .is_err()
                    {
                        continue;
                    }
                    let map = &self.map_docs[m];
                    for i in 0..map.tilesets.len() {
                        let Some(data) = crate::paint::tileset_data(map, i, self.tilesets) else {
                            continue;
                        };
                        for k in data.kinds() {
                            if !tile_kinds.contains(&k) {
                                tile_kinds.push(k);
                            }
                        }
                        if tile_sheet.is_none()
                            && let Some((texture, size)) =
                                self.tiles_panel
                                    .images()
                                    .get(ui.ctx(), self.root, &data.image)
                        {
                            tile_sheet = Some(crate::events_view::TileSheet {
                                texture,
                                size,
                                data: data.clone(),
                                first_gid: map.tilesets[i].first_gid,
                            });
                        }
                    }
                }
                tile_kinds.sort();
                let env = crate::events_view::EventsEnv {
                    files: self.files,
                    vars: self.vars,
                    anims: &anims,
                    tile_kinds: &tile_kinds,
                    tile_sheet: tile_sheet.as_ref(),
                };
                for a in self.events.ui(ui, doc, &env) {
                    match a {
                        crate::events_view::EventsAction::OpenFile(rel) => {
                            self.actions.push(Action::OpenFile { rel, line: None });
                        }
                    }
                }
            }
            Tab::Tiles => self.tiles(ui),
            Tab::Inspector
                if self
                    .active_model
                    .is_some_and(|r| self.models.contains_key(r)) =>
            {
                let rel = self.active_model.unwrap_or_default();
                if let Some(doc) = self.models.get(rel) {
                    model_summary(ui, doc);
                }
            }
            Tab::Inspector => {
                let active = self.active_scene.clone();
                let doc = active.as_deref().and_then(|r| self.scenes.get_mut(r));
                let painting_before = doc.as_ref().and_then(|d| d.painting);
                let game = self
                    .game_state
                    .filter(|g| active.as_deref() == Some(g.scene.as_str()));
                for a in self
                    .inspector
                    .ui(ui, doc, self.files, self.sizes, self.mode, game)
                {
                    match a {
                        InspectorAction::OpenFile(rel) => {
                            self.actions.push(Action::OpenFile { rel, line: None });
                        }
                        InspectorAction::ShowEvents => {
                            self.actions.push(Action::Command(Command::ShowEvents));
                        }
                        InspectorAction::CreateScript => {
                            if let Some(r) = &active {
                                self.actions.push(Action::CreateSceneScript(r.clone()));
                            }
                        }
                        InspectorAction::NewBehaviorScript(id) => {
                            if let Some(r) = &active {
                                self.actions.push(Action::NewBehaviorScript {
                                    scene: r.clone(),
                                    id,
                                });
                            }
                        }
                        InspectorAction::NewImage { id, ci, name } => {
                            if let Some(r) = &active {
                                self.actions.push(Action::NewImageFor {
                                    scene: r.clone(),
                                    id,
                                    ci,
                                    name,
                                });
                            }
                        }
                    }
                }
                // Ligou o Pintar pelo Inspetor: o painel Tiles aparece.
                let painting_after = active
                    .as_deref()
                    .and_then(|r| self.scenes.get(r))
                    .and_then(|d| d.painting);
                if painting_before.is_none() && painting_after.is_some() {
                    self.actions.push(Action::Command(Command::ShowTiles));
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
        // Pintando: o mapa (e os tilesets) abertos no editor.
        if let Some(id) = doc.painting {
            let map_rel = doc
                .data
                .find(id)
                .and_then(|o| o.component("map"))
                .map(|c| c.str("map").to_string());
            match map_rel {
                Some(m) => {
                    if let Err(e) =
                        crate::paint::ensure_open(self.map_docs, self.tilesets, self.root, &m)
                    {
                        self.console
                            .error(format!("Não foi possível abrir o mapa {m}: {e}"));
                        doc.painting = None;
                    }
                }
                None => doc.painting = None,
            }
        }
        // Mapas abertos (talvez sem salvar) são desenhados como estão aqui.
        for (m, map) in self.map_docs.iter() {
            self.sizes.map_px.insert(
                m.clone(),
                (
                    (map.width * map.tile_w) as f32,
                    (map.height * map.tile_h) as f32,
                ),
            );
            if let Some(sr) = self.scene_renderer.as_mut() {
                let rev = crate::paint::combined_rev(map, self.tilesets);
                if sr.map_source_rev(m) != Some(rev) {
                    sr.set_map_source(m, rev, crate::paint::source_files(map, self.tilesets));
                }
            }
        }
        let mut env = SceneEnv {
            root: self.root,
            sizes: self.sizes,
            renderer: self.scene_renderer.as_mut(),
            design: self.design,
            clear_color: self.clear_color,
            connected: self.game_scene == Some(rel),
            time: self.time,
            game: self.game_state.filter(|g| g.scene == rel),
            maps: self.map_docs,
            tilesets: self.tilesets,
            paint: self.paint,
            animate: self.animate,
        };
        let was_painting = doc.painting;
        let actions = doc.ui(ui, &mut env);
        if let Some(id) = doc.painting {
            if let Some(m) = doc.data.find(id).and_then(|o| o.component("map")) {
                *self.tiles_map = Some(m.str("map").to_string());
            }
            if was_painting.is_none() {
                self.actions.push(Action::Command(Command::ShowTiles));
            }
        }
        for a in actions {
            match a {
                SceneAction::TestHere => self.actions.push(Action::TestScene(rel.to_string())),
                SceneAction::Reload => doc.reload(self.root),
                SceneAction::OpenAsText => self.actions.push(Action::OpenAsText(rel.to_string())),
                SceneAction::Game(cmd) => self.actions.push(Action::GameCommand(cmd)),
                SceneAction::NewMap => self.actions.push(Action::NewMap),
                SceneAction::Object(a) => hierarchy_action(self.actions, a),
            }
        }
        // Objetos com animação tocam na cena do editor também.
        let mut animated = false;
        crate::scene_data::visit(&doc.data.objects, &mut |o| {
            animated |= o
                .components
                .iter()
                .any(|c| c.kind == "sprite" && !c.str("animation").is_empty());
        });
        // As partículas também animam (a não ser com a prévia parada), mais
        // lisas; com a janela sem foco, nada anima.
        let fx = !doc.particles_paused && doc.has_particles();
        if fx {
            crate::theme::animate(ui.ctx(), std::time::Duration::from_millis(16));
        } else if animated {
            crate::theme::animate(ui.ctx(), std::time::Duration::from_millis(33));
        }
        // Clicou nesta cena: a Hierarquia e o Inspetor passam a mostrá-la.
        if ui.ui_contains_pointer() && ui.input(|i| i.pointer.any_pressed()) {
            *self.active_scene = Some(rel.to_string());
        }
    }

    /// O painel Tiles: o mapa sendo pintado (ou o último).
    fn tiles(&mut self, ui: &mut egui::Ui) {
        let active = self.active_scene.clone();
        // O mapa sendo pintado na cena ativa vem primeiro.
        let painting = active
            .as_deref()
            .and_then(|r| self.scenes.get(r))
            .and_then(|d| d.painting.and_then(|id| d.data.find(id)))
            .and_then(|o| o.component("map"))
            .map(|c| c.str("map").to_string());
        if let Some(m) = &painting {
            *self.tiles_map = Some(m.clone());
        }
        let map = self.tiles_map.clone().filter(|m| {
            crate::paint::ensure_open(self.map_docs, self.tilesets, self.root, m).is_ok()
        });
        let mut env = crate::tiles_view::TilesEnv {
            root: self.root,
            map: map.as_deref(),
            painting: painting.is_some() && painting == map,
            maps: self.map_docs,
            tilesets: self.tilesets,
            paint: self.paint,
        };
        for a in self.tiles_panel.ui(ui, &mut env) {
            match a {
                crate::tiles_view::TilesAction::TogglePaint => {
                    let Some(doc) = active.as_deref().and_then(|r| self.scenes.get_mut(r)) else {
                        if let Some(m) = &map {
                            self.actions.push(Action::OpenFile {
                                rel: m.clone(),
                                line: None,
                            });
                        }
                        continue;
                    };
                    if doc.painting.is_some() {
                        doc.painting = None;
                        continue;
                    }
                    let Some(m) = &map else { continue };
                    // O objeto selecionado com esse mapa, ou o primeiro que tiver.
                    let uses = |o: &crate::scene_data::Object| {
                        o.component("map")
                            .is_some_and(|c| c.str("map") == m.as_str())
                    };
                    let mut found = doc
                        .selection
                        .iter()
                        .copied()
                        .find(|id| doc.data.find(*id).is_some_and(uses));
                    if found.is_none() {
                        crate::scene_data::visit(&doc.data.objects, &mut |o| {
                            if found.is_none() && uses(o) {
                                found = Some(o.id);
                            }
                        });
                    }
                    match found {
                        Some(id) => {
                            doc.painting = Some(id);
                            doc.selection = vec![id];
                        }
                        None => self.actions.push(Action::OpenFile {
                            rel: m.clone(),
                            line: None,
                        }),
                    }
                }
                crate::tiles_view::TilesAction::NewMap => self.actions.push(Action::NewMap),
                crate::tiles_view::TilesAction::OpenFile(rel) => {
                    self.actions.push(Action::OpenFile { rel, line: None });
                }
                crate::tiles_view::TilesAction::ImageChanged(_) => {}
            }
        }
    }

    fn viewer_action(&mut self, a: ViewerAction) {
        self.actions.push(match a {
            ViewerAction::CopyText(t) => Action::CopyText(t),
            ViewerAction::OpenExternal(r) => Action::OpenExternal(r),
            ViewerAction::EditSound(r) => Action::NewSound {
                parent: String::new(),
                edit: Some(r),
            },
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
                        ui.label(RichText::new(n).strong().color(ui.visuals().hyperlink_color));
                        ui.label(text);
                    });
                };
                step(ui, "1.", "Clique em ▶ Jogar (ou aperte F5): o jogo abre na janela dele.");
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("2.").strong().color(ui.visuals().hyperlink_color));
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
                crate::tips::tip_ui(ui);
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
        let changed = self
            .form
            .original
            .as_ref()
            .map(|o| config_lua::changes(o, &self.form.values))
            .unwrap_or_default();
        let vars_ok = vars_valid(&self.form.values.vars);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    !changed.is_empty() && vars_ok,
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
            if self.mode.is_full() && ui.button("Abrir config.lua").clicked() {
                self.actions.push(Action::OpenFile {
                    rel: "config.lua".into(),
                    line: None,
                });
            }
        });
        ui.add_space(8.0);
        let full = self.mode.is_full();
        let scenes: Vec<String> = self
            .files
            .files_of(FileKind::Scene)
            .chain(self.files.files_of(FileKind::Lua))
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

                ui.label("Tamanho do jogo:")
                    .on_hover_text("Com tamanho fixo, o jogo enxerga sempre o mesmo tamanho e se ajusta à janela (maior ou menor), sem cortar nada.");
                ui.horizontal_wrapped(|ui| {
                    let mut fixed = v.resolution.is_some();
                    if ui
                        .checkbox(&mut fixed, "Fixo")
                        .on_hover_text("Desligado: o jogo enxerga o tamanho da janela (janela maior mostra mais coisa).")
                        .changed()
                    {
                        v.resolution = fixed.then_some((v.width, v.height));
                    }
                    if let Some((w, h)) = &mut v.resolution {
                        ui.add(egui::DragValue::new(w).range(16..=7680));
                        ui.label("×");
                        ui.add(egui::DragValue::new(h).range(16..=4320));
                        let names = [
                            ("fit", "Ajustar (faixas pretas)"),
                            ("stretch", "Esticar (ocupa tudo)"),
                            ("pixel", "Pixel art (só 2×, 3×...)"),
                        ];
                        let current = names
                            .iter()
                            .find(|(k, _)| *k == v.scale)
                            .map(|(_, l)| *l)
                            .unwrap_or("Ajustar (faixas pretas)");
                        egui::ComboBox::from_id_salt("escala")
                            .selected_text(current)
                            .show_ui(ui, |ui| {
                                for (k, l) in names {
                                    ui.selectable_value(&mut v.scale, k.to_string(), l);
                                }
                            });
                    }
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
        ui.add_space(14.0);
        vars_editor(ui, &mut self.form.values.vars);
        ui.add_space(14.0);
        let fonts: Vec<String> = self
            .files
            .files_of(FileKind::Font)
            .map(|e| e.rel.clone())
            .collect();
        let sounds: Vec<String> = self
            .files
            .files_of(FileKind::Sound)
            .map(|e| e.rel.clone())
            .collect();
        theme_editor(ui, &mut self.form.values, &fonts, &sounds);
    }
}

/// Tema da interface: as cores, a fonte e o som que os menus e o HUD usam
/// quando o componente não escolhe outros.
fn theme_editor(ui: &mut egui::Ui, v: &mut ConfigValues, fonts: &[String], sounds: &[String]) {
    let r = egui::CollapsingHeader::new(RichText::new("Tema da interface").strong())
        .id_salt("config-tema")
        .show(ui, |ui| theme_form(ui, v, fonts, sounds));
    crate::tutorial::anchor(ui.ctx(), "config:tema", r.header_response.rect);
}

fn theme_form(ui: &mut egui::Ui, v: &mut ConfigValues, fonts: &[String], sounds: &[String]) {
    ui.weak("Cores, fonte e som dos menus e do HUD. Botões, painéis e barras usam estas, a não ser que você escolha outra no Inspetor.");
    ui.add_space(4.0);
    let set = |v: &mut ConfigValues, key: &str, value: String| {
        let default = crate::scene_data::schema()
            .theme
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, d)| d.clone())
            .unwrap_or_default();
        v.theme.retain(|(k, _)| k != key);
        if value != default {
            v.theme.push((key.to_string(), value));
            v.theme.sort();
        }
    };
    let colors = [
        ("text", "Texto"),
        ("button", "Botão"),
        ("hover", "Botão com foco"),
        ("press", "Botão apertado"),
        ("disabled", "Botão desligado"),
        ("panel", "Painel"),
        ("border", "Bordas"),
        ("accent", "Destaque (barras, ✔)"),
    ];
    ui.horizontal_top(|ui| {
        egui::Grid::new("grade-tema")
            .num_columns(2)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                for (key, label) in colors {
                    ui.label(format!("{label}:"));
                    ui.horizontal(|ui| {
                        let current = v.theme_value(key);
                        let mut c = crate::scene_render::egui_color(&current).to_srgba_unmultiplied();
                        if ui.color_edit_button_srgba_unmultiplied(&mut c).changed() {
                            let hex = if c[3] == 255 {
                                format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
                            } else {
                                format!("#{:02X}{:02X}{:02X}{:02X}", c[0], c[1], c[2], c[3])
                            };
                            set(v, key, hex);
                        }
                        ui.weak(&current);
                    });
                    ui.end_row();
                }
                for (key, label, list, tip) in [
                    ("font", "Fonte", fonts, "Arquivo .ttf ou .otf (ponha na pasta assets/fontes). Vazio = a fonte da engine."),
                    ("sound", "Som dos botões", sounds, "Toca quando um botão é apertado."),
                ] {
                    ui.label(format!("{label}:")).on_hover_text(tip);
                    let current = v.theme_value(key);
                    let mut chosen = current.clone();
                    egui::ComboBox::from_id_salt(("tema", key))
                        .selected_text(if current.is_empty() {
                            "(nenhum)".to_string()
                        } else {
                            current.clone()
                        })
                        .width(220.0)
                        .truncate()
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut chosen, String::new(), "(nenhum)");
                            for f in list {
                                ui.selectable_value(&mut chosen, f.clone(), f);
                            }
                        });
                    if chosen != current {
                        set(v, key, chosen);
                    }
                    ui.end_row();
                }
            });
        ui.add_space(16.0);
        theme_preview(ui, v);
    });
    if !v.theme.is_empty() && ui.button("Voltar ao tema padrão").clicked() {
        v.theme.clear();
    }
}

/// Um painel com um botão em cada estado e uma barra, nas cores do tema.
fn theme_preview(ui: &mut egui::Ui, v: &ConfigValues) {
    let color = |k: &str| crate::scene_render::egui_color(&v.theme_value(k));
    let (rect, _) = ui.allocate_exact_size(egui::vec2(220.0, 230.0), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 4.0, egui::Color32::from_rgb(0x30, 0x40, 0x30));
    let panel = rect.shrink(12.0);
    p.rect_filled(panel, 10.0, color("panel"));
    p.rect_stroke(
        panel,
        10.0,
        egui::Stroke::new(1.5, color("border")),
        egui::StrokeKind::Inside,
    );
    let text = color("text");
    for (i, (key, label)) in [
        ("button", "Normal"),
        ("hover", "Com foco"),
        ("press", "Apertado"),
        ("disabled", "Desligado"),
    ]
    .into_iter()
    .enumerate()
    {
        let b = egui::Rect::from_center_size(
            egui::pos2(panel.center().x, panel.top() + 26.0 + i as f32 * 40.0),
            egui::vec2(150.0, 32.0),
        );
        p.rect_filled(b, 8.0, color(key));
        p.text(
            b.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(14.0),
            text,
        );
    }
    let bar = egui::Rect::from_min_size(
        egui::pos2(panel.center().x - 75.0, panel.bottom() - 28.0),
        egui::vec2(150.0, 12.0),
    );
    p.rect_filled(bar, 4.0, egui::Color32::from_black_alpha(128));
    p.rect_filled(
        egui::Rect::from_min_size(bar.min, egui::vec2(bar.width() * 0.65, bar.height())),
        4.0,
        color("accent"),
    );
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

/// Os mapas (`.tmj`) que os objetos da cena usam.
fn scene_maps(data: &crate::scene_data::SceneData) -> Vec<String> {
    let mut out = Vec::new();
    crate::scene_data::visit(&data.objects, &mut |o| {
        if let Some(c) = o.component("map") {
            let m = c.str("map");
            if !m.is_empty() && !out.iter().any(|x: &String| x == m) {
                out.push(m.to_string());
            }
        }
    });
    out
}

/// O confete de uma comemoração: papeizinhos que sobem do alto da janela e
/// caem, por 1,8 s. `start` vira `None` quando acaba.
fn draw_confetti(ctx: &egui::Context, start: &mut Option<f32>) {
    const LEN: f32 = 1.8;
    let Some(t) = *start else {
        return;
    };
    if t > LEN {
        *start = None;
        return;
    }
    *start = Some(t + ctx.input(|i| i.stable_dt).min(0.1));
    // Números "aleatórios" fixos para cada papel.
    let rnd = |i: u32, k: u32| -> f32 {
        let mut x = i.wrapping_mul(0x9E37_79B9) ^ k.wrapping_mul(0x85EB_CA6B);
        x ^= x >> 15;
        x = x.wrapping_mul(0x2C1B_3C6D);
        x ^= x >> 12;
        (x % 10_000) as f32 / 10_000.0
    };
    let colors = [
        egui::Color32::from_rgb(0xFF, 0xC8, 0x3D),
        egui::Color32::from_rgb(0x4C, 0xC9, 0xF0),
        egui::Color32::from_rgb(0xF2, 0x6D, 0x6D),
        egui::Color32::from_rgb(0x6F, 0xD6, 0x96),
        egui::Color32::from_rgb(0xC7, 0x7D, 0xFF),
    ];
    let rect = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("confete"),
    ));
    let fade = 1.0 - (t / LEN).powi(3);
    for i in 0..80u32 {
        let x0 = rect.center().x + (rnd(i, 1) - 0.5) * rect.width() * 0.35;
        let y0 = rect.top() + rect.height() * 0.2;
        let vx = (rnd(i, 2) - 0.5) * 520.0;
        let vy = -220.0 - rnd(i, 3) * 380.0;
        let p = egui::pos2(x0 + vx * t, y0 + vy * t + 0.5 * 900.0 * t * t);
        let spin = (t * (4.0 + rnd(i, 4) * 8.0) + rnd(i, 5) * 6.0).sin();
        let size = egui::vec2(3.0 + 4.0 * spin.abs(), 6.0);
        let color = colors[(i as usize) % colors.len()].gamma_multiply(fade);
        painter.rect_filled(egui::Rect::from_center_size(p, size), 1.0, color);
    }
    ctx.request_repaint();
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

/// O que a thread dos testes manda: cada teste (falhou?, texto) e o fim.
#[derive(Debug)]
enum TestMsg {
    Result((bool, String)),
    Done(Result<Vec<(bool, String)>, String>),
}

/// A linha do Console de um teste: ✔, ou ✖ com o motivo (o arquivo:linha vira
/// link) e o que o jogo escreveu.
fn test_line(r: &nn_script::testing::TestResult) -> (bool, String) {
    let secs = |t: f64| format!("{t:.2} s").replace('.', ",");
    match &r.failure {
        None => (
            false,
            format!("✔ {} › {} ({})", r.case.file, r.case.name, secs(r.time)),
        ),
        Some(f) => {
            let mut text = format!("✖ {} › {}: {f}", r.case.file, r.case.name);
            for l in r.report.lines.iter().take(20) {
                let kind = match l.kind {
                    nn_script::capture::LineKind::Print => "print",
                    nn_script::capture::LineKind::Warning => "aviso",
                    nn_script::capture::LineKind::Error => "erro",
                };
                text.push_str(&format!("\n  {kind} em {}: {}", secs(l.time), l.text));
            }
            (true, text)
        }
    }
}
