//! Aba Cena: a cena desenhada pelo renderer da engine, com seleção,
//! gizmos para mover, girar e escalar, navegação (rolar = zoom, botão do meio
//! ou Espaço + arrastar = mover a vista) e arrastar arquivos para dentro.

use std::path::Path;

use eframe::egui::{self, Color32, Key, Modifiers, Pos2, Rect, Sense, Stroke, Vec2};

use crate::scene_data::{self, Component, History, Object, Prop, SceneData, Transform};

#[path = "scene3d.rs"]
pub mod scene3d;
#[path = "scene_repeat.rs"]
mod scene_repeat;
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
    /// Valor que não é simples (as regras de eventos).
    pub json: Option<serde_json::Value>,
}

impl LiveChange {
    pub fn to_json(&self, scene: &str) -> String {
        let mut v = serde_json::json!({
            "scene": scene,
            "id": self.id,
            "field": self.field,
            "value": self.json.clone().unwrap_or_else(|| self.value.to_json()),
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
    let events = |id: u32, rules: &[scene_data::Rule]| LiveChange {
        id,
        component: None,
        field: "events".into(),
        value: Prop::Bool(false),
        json: Some(scene_data::rules_json(rules)),
    };
    if before.events != after.events {
        out.push(events(0, &after.events));
    }
    if after.three_d && before.environment != after.environment {
        // O Ambiente vai inteiro (o jogo refaz o céu, a luz e as sombras).
        let env: serde_json::Map<String, serde_json::Value> = after
            .environment
            .iter()
            .map(|(k, v)| (k.clone(), v.to_json()))
            .collect();
        out.push(LiveChange {
            id: 0,
            component: None,
            field: "environment".into(),
            value: Prop::Bool(false),
            json: Some(serde_json::Value::Object(env)),
        });
    }
    scene_data::visit(&after.objects, &mut |o| {
        let Some(b) = before.find(o.id) else { return };
        if b.events != o.events {
            out.push(events(o.id, &o.events));
        }
        for key in [
            "name", "x", "y", "rotation", "scale_x", "scale_y", "visible", "layer", "anchor", "z",
            "rx", "ry", "rz", "scale_z",
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
                    json: None,
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
                        json: None,
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
    /// A seta da direção das partículas (componente `ci`).
    Direction {
        ci: usize,
    },
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
    /// 3D: mostra o HUD (o que fica preso à tela) por cima da vista.
    pub show_hud3: bool,
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
    /// Objeto de mapa sendo pintado (modo Pintar).
    pub painting: Option<u32>,
    /// A prévia das partículas está parada.
    pub particles_paused: bool,
    /// Objetos com partículas para soltar no próximo desenho (💥 Soltar).
    pub burst_requests: Vec<u32>,
    /// 3D: os pincéis do terreno.
    pub terrain_tool: crate::terrain_tool::TerrainTool,
    paint_stroke: Option<crate::paint::Stroke>,
    /// O mapa cresceu desde que o jogo abriu: ele só vê ao salvar.
    pub paint_grew: bool,
    /// 3D: a câmera do editor, se ela já foi mexida, o arrasto, a grade.
    pub cam3: scene3d::Cam3,
    pub cam3_set: bool,
    drag3: Option<scene3d::Drag3>,
    pub grid3: f64,
    pub show_grid3: bool,
    /// 3D: a névoa da cena na vista (desligue para ver a cena toda de longe).
    pub show_fog3: bool,
    /// Enquadrar a seleção no próximo desenho (pedido do menu ⛶ Focar).
    pub focus_request: bool,
    /// Rolar a Hierarquia até este objeto no próximo desenho.
    pub reveal: Option<u32>,
    /// O menu do botão direito na vista: em cima de quem e onde.
    view_menu: Option<ViewMenu>,
    /// Um segundo botão direito com o menu aberto: reabre no quadro seguinte.
    view_menu_reopen: Option<ViewMenu>,
    /// A busca rápida para criar (Shift+A), aberta.
    pub quick_add: Option<crate::catalog::QuickAdd>,
    /// 3D: a câmera voou (WASD) com o botão direito segurado: soltar o botão
    /// não abre o menu.
    fly_moved: bool,
    /// A última duplicação (o Ctrl+D seguinte repete o passo dela).
    last_dup: Option<LastDup>,
    /// Shift+D: as cópias seguem o mouse até o clique.
    carry: Option<Carry>,
    /// O clique que soltou as cópias do Shift+D: ignora o mouse até soltar.
    swallow_press: bool,
    /// Avisos para o canto da janela (apagou, colou...), com Desfazer.
    pub notices: Vec<String>,
    /// Sons da interface pedidos ("criar"), se ligados nas Preferências.
    pub sounds: Vec<&'static str>,
    /// Objetos que acabaram de nascer: o contorno brilha um instante. (Com a
    /// hora em que começou, depois do primeiro desenho.)
    flash: Vec<(u32, Option<f64>)>,
    /// O 🔁 Repetir aberto.
    repeat: Option<scene_repeat::SceneRepeat>,
}

/// De quem saíram as últimas cópias (Ctrl+D, Alt+arrastar, Shift+D).
#[derive(Debug, Clone, PartialEq)]
struct LastDup {
    sources: Vec<u32>,
    copies: Vec<u32>,
}

/// Cópias do Shift+D, seguindo o mouse.
#[derive(Debug, Clone)]
struct Carry {
    /// Cada cópia e onde ela estava ao começar.
    objects: Vec<(u32, WorldPos)>,
    /// Onde o mouse estava no mundo 3D (no chão, na altura das cópias) e no
    /// 2D (ou no HUD da cena 3D), ao começar.
    anchor3: Option<glam::Vec3>,
    anchor2: Option<(f64, f64)>,
    /// A altura do plano em que as cópias do 3D andam.
    y: f32,
}

/// Onde o botão direito foi clicado na vista.
#[derive(Debug, Clone, Copy)]
struct ViewMenu {
    /// O objeto embaixo do mouse (`None`: o vazio).
    target: Option<u32>,
    /// O ponto no mundo 2D.
    at2: (f64, f64),
    /// O ponto no mundo 3D (no objeto, ou no chão).
    at3: glam::Vec3,
    /// O alvo é um chão (o menu é de criar em cima dele).
    ground: bool,
}

/// Onde um objeto está no mundo (no 2D e no HUD, em pixels e z = 0). Também
/// é a posição copiada no menu do botão direito.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldPos {
    /// No mundo 3D (metros); senão, no 2D ou no HUD (pixels, z = 0).
    pub three_d: bool,
    pub pos: [f64; 3],
}

fn copied_id() -> egui::Id {
    egui::Id::new("posicao-copiada")
}

/// Os objetos copiados (Ctrl+C), para o Colar aqui do menu.
fn clip_id() -> egui::Id {
    egui::Id::new("objetos-copiados")
}

/// A cor do brilho dos objetos que acabaram de nascer.
pub(crate) const FLASH: Color32 = Color32::from_rgb(0xFF, 0xE0, 0x66);

/// O começo do texto de objetos copiados no NN Studio.
const CLIP_MARK: &str = "-- nn-studio: objetos copiados";

/// O que a aba pede ao espaço de trabalho.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneAction {
    TestHere,
    Reload,
    OpenAsText,
    /// Comando para o jogo aberto: "pause", "resume", "step",
    /// "show_paths" ou "hide_paths".
    Game(&'static str),
    /// Criar um mapa de tiles (abre a janela).
    NewMap,
    /// Um pedido do menu do objeto (o mesmo da Hierarquia).
    Object(crate::hierarchy::HierarchyAction),
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
    /// Onde os objetos estão no jogo aberto (se ele mostra esta cena).
    pub game: Option<&'a GameState>,
    /// Mapas abertos para pintar, os tilesets deles e o pincel.
    pub maps: &'a mut std::collections::HashMap<String, crate::map_doc::MapDoc>,
    pub tilesets: &'a mut std::collections::HashMap<String, crate::tileset::TilesetDoc>,
    pub paint: &'a mut crate::paint::PaintState,
    /// As animações estão ligadas (Preferências): o brilho dos objetos novos.
    pub animate: bool,
}

impl std::fmt::Debug for SceneEnv<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SceneEnv").finish_non_exhaustive()
    }
}

/// Um objeto da cena como está no jogo aberto.
#[derive(Debug, Clone, PartialEq)]
pub struct GameObject {
    /// Posição, rotação e escala (relativas ao pai), como no arquivo.
    pub local: Transform,
    /// No mundo.
    pub world: Transform,
    pub visible: bool,
    /// Propriedades dos comportamentos: (componente, campos).
    pub props: Vec<(usize, Vec<(String, Prop)>)>,
}

/// O que o jogo aberto pelo editor conta da cena da frente.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GameState {
    pub scene: String,
    pub paused: bool,
    /// O jogo está mostrando os caminhos (botão Caminhos).
    pub paths: bool,
    pub objects: std::collections::HashMap<u32, GameObject>,
    pub vars: Vec<(String, Prop)>,
    /// Regras que dispararam desde o último estado ("c:2" = 2ª regra da
    /// cena, "7:1" = 1ª regra do objeto 7).
    pub fired: Vec<String>,
}

fn json_prop(v: &serde_json::Value) -> Option<Prop> {
    Some(match v {
        serde_json::Value::Number(n) => Prop::Num(n.as_f64()?),
        serde_json::Value::Bool(b) => Prop::Bool(*b),
        serde_json::Value::String(s) => Prop::Str(s.clone()),
        _ => return None,
    })
}

impl GameState {
    /// Lê a mensagem `estado` do jogo.
    pub fn parse(v: &serde_json::Value) -> Option<Self> {
        let num = |o: &serde_json::Value, k: &str| o.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0);
        let mut state = GameState {
            scene: v.get("scene")?.as_str()?.to_string(),
            paused: v.get("paused").and_then(|p| p.as_bool()).unwrap_or(false),
            paths: v.get("paths").and_then(|p| p.as_bool()).unwrap_or(false),
            ..Default::default()
        };
        // Lista vazia chega como {} (tabela Lua vazia).
        if let Some(list) = v.get("objects").and_then(|o| o.as_array()) {
            for o in list {
                let Some(id) = o.get("id").and_then(|i| i.as_u64()) else {
                    continue;
                };
                let mut props = Vec::new();
                if let Some(map) = o.get("props").and_then(|p| p.as_object()) {
                    for (ci, fields) in map {
                        let Ok(ci) = ci.parse::<usize>() else {
                            continue;
                        };
                        let mut list: Vec<(String, Prop)> = fields
                            .as_object()
                            .map(|f| {
                                f.iter()
                                    .filter_map(|(k, v)| json_prop(v).map(|p| (k.clone(), p)))
                                    .collect()
                            })
                            .unwrap_or_default();
                        list.sort_by(|a, b| a.0.cmp(&b.0));
                        props.push((ci.saturating_sub(1), list));
                    }
                }
                props.sort_by_key(|(ci, _)| *ci);
                state.objects.insert(
                    id as u32,
                    GameObject {
                        local: Transform {
                            x: num(o, "x"),
                            y: num(o, "y"),
                            rotation: num(o, "rotation"),
                            scale_x: num(o, "scale_x"),
                            scale_y: num(o, "scale_y"),
                        },
                        world: Transform {
                            x: num(o, "wx"),
                            y: num(o, "wy"),
                            rotation: num(o, "wr"),
                            scale_x: num(o, "wsx"),
                            scale_y: num(o, "wsy"),
                        },
                        visible: o.get("visible").and_then(|b| b.as_bool()).unwrap_or(true),
                        props,
                    },
                );
            }
        }
        if let Some(list) = v.get("fired").and_then(|o| o.as_array()) {
            state.fired = list
                .iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect();
        }
        if let Some(map) = v.get("vars").and_then(|o| o.as_object()) {
            state.vars = map
                .iter()
                .filter_map(|(k, v)| json_prop(v).map(|p| (k.clone(), p)))
                .collect();
            state.vars.sort_by(|a, b| a.0.cmp(&b.0));
        }
        Some(state)
    }

    /// Valor de uma propriedade de comportamento no jogo.
    pub fn prop(&self, id: u32, component: usize, key: &str) -> Option<&Prop> {
        self.objects
            .get(&id)?
            .props
            .iter()
            .find(|(c, _)| *c == component)?
            .1
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }
}

/// Tamanho de um quadro de uma imagem que parece uma spritesheet: pixel
/// art pequena em quadros de 16×16 (a mais comum), ou uma tira com quadros
/// quadrados. Senão, a imagem toda.
pub fn sheet_frame(w: f64, h: f64) -> (f64, f64) {
    if w % 16.0 == 0.0 && h % 16.0 == 0.0 && (w > 16.0 || h > 16.0) && w.max(h) <= 128.0 {
        (16.0, 16.0)
    } else if w > h && h > 0.0 && (w % h) == 0.0 {
        (h, h)
    } else {
        (w, h)
    }
}

/// O caminho é de um prefab?
pub fn is_prefab_path(rel: &str) -> bool {
    rel.to_lowercase().ends_with(".prefab")
}

/// Lê uma cena ou um prefab (que vira uma cena com um objeto só), com as
/// instâncias de prefabs ligadas.
fn read_doc(root: &Path, rel: &str) -> Result<SceneData, String> {
    let text = std::fs::read_to_string(root.join(rel)).map_err(|e| e.to_string())?;
    let mut data = if is_prefab_path(rel) {
        SceneData {
            objects: vec![scene_data::read_prefab(&text, rel)?],
            ..Default::default()
        }
    } else {
        scene_data::read(&text, rel)?
    };
    data.link_prefabs(root);
    Ok(data)
}

const GIZMO_LEN: f32 = 64.0;
/// Comprimento da seta da direção das partículas.
const FX_LEN: f32 = 90.0;
const FX: Color32 = Color32::from_rgb(0xFF, 0x8A, 0x3D);
const HANDLE: f32 = 7.0;
const RED: Color32 = Color32::from_rgb(0xE5, 0x48, 0x4D);
const GREEN: Color32 = Color32::from_rgb(0x46, 0xC2, 0x6A);
const BLUE: Color32 = Color32::from_rgb(0x4A, 0x9C, 0xF0);
const SELECT: Color32 = Color32::from_rgb(0xFF, 0xA9, 0x2E);

impl SceneDoc {
    /// É um `.prefab` (um objeto só) e não uma cena?
    pub fn is_prefab(&self) -> bool {
        is_prefab_path(&self.rel)
    }

    pub fn open(root: &Path, rel: &str) -> Self {
        let (data, error) = match read_doc(root, rel) {
            Ok(d) => (d, None),
            Err(e) => (SceneData::default(), Some(e)),
        };
        let three_d = data.three_d;
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
            // No 3D, as caixas dos corpos cobririam a cena: ficam desligadas
            // até pedir (Colisões, na barra).
            show_colliders: !three_d,
            show_hud3: true,
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
            painting: None,
            particles_paused: false,
            burst_requests: Vec::new(),
            terrain_tool: Default::default(),
            paint_stroke: None,
            paint_grew: false,
            cam3: scene3d::Cam3::default(),
            cam3_set: false,
            drag3: None,
            grid3: 1.0,
            show_grid3: true,
            show_fog3: true,
            focus_request: false,
            reveal: None,
            view_menu: None,
            view_menu_reopen: None,
            quick_add: None,
            fly_moved: false,
            last_dup: None,
            carry: None,
            swallow_press: false,
            notices: Vec::new(),
            sounds: Vec::new(),
            flash: Vec::new(),
            repeat: None,
        }
    }

    /// Abre os pais do objeto na Hierarquia e rola até ele.
    pub fn reveal(&mut self, id: u32) {
        let mut cur = self.data.parent_of(id).flatten();
        while let Some(p) = cur {
            self.collapsed.remove(&p);
            cur = self.data.parent_of(p).flatten();
        }
        self.reveal = Some(id);
    }

    /// Começa a renomear o objeto (na Hierarquia).
    pub fn start_rename(&mut self, id: u32) {
        if let Some(o) = self.data.find(id) {
            self.renaming = Some((id, o.name.clone(), true));
            self.reveal(id);
        }
    }

    /// Onde o objeto está no mundo (no 2D e no HUD, z = 0).
    fn world_position(&self, id: u32) -> Option<WorldPos> {
        if self.data.three_d && !self.on_screen3(id) {
            let p = self
                .data
                .world_matrix3(id)?
                .transform_point3(glam::Vec3::ZERO);
            let r = |v: f32| round2(f64::from(v));
            return Some(WorldPos {
                three_d: true,
                pos: [r(p.x), r(p.y), r(p.z)],
            });
        }
        let t = self.data.world_transform(id)?;
        Some(WorldPos {
            three_d: false,
            pos: [t.x, t.y, 0.0],
        })
    }

    /// Guarda a posição do objeto para colar em outro.
    pub fn copy_position(&self, ctx: &egui::Context, id: u32) {
        if let Some(p) = self.world_position(id) {
            ctx.data_mut(|d| d.insert_temp(copied_id(), p));
        }
    }

    /// Dá para colar a posição copiada no objeto? (Do 3D para o 3D, do 2D
    /// para o 2D.)
    pub fn can_paste_position(&self, ctx: &egui::Context, id: u32) -> bool {
        let copied: Option<WorldPos> = ctx.data(|d| d.get_temp(copied_id()));
        match (copied, self.world_position(id)) {
            (Some(c), Some(p)) => c.three_d == p.three_d,
            _ => false,
        }
    }

    /// Põe a seleção na posição copiada.
    pub fn paste_position(&mut self, ctx: &egui::Context) {
        let Some(c) = ctx.data(|d| d.get_temp::<WorldPos>(copied_id())) else {
            return;
        };
        let ids: Vec<u32> = self
            .selection_roots()
            .into_iter()
            .filter(|id| {
                self.world_position(*id)
                    .is_some_and(|p| p.three_d == c.three_d)
            })
            .collect();
        if ids.is_empty() {
            return;
        }
        self.record(None);
        for id in ids {
            self.move_to(id, c);
        }
    }

    /// Leva o objeto para a posição do mundo (no 3D, sem mudar a rotação e
    /// a escala).
    fn move_to(&mut self, id: u32, p: WorldPos) {
        if p.three_d {
            if let Some(m0) = self.data.world_matrix3(id) {
                let p0 = m0.transform_point3(glam::Vec3::ZERO);
                let to = glam::Vec3::new(p.pos[0] as f32, p.pos[1] as f32, p.pos[2] as f32);
                self.data
                    .set_world_matrix3(id, glam::Mat4::from_translation(to - p0) * m0);
            }
        } else {
            self.set_world_position(id, p.pos[0], p.pos[1]);
        }
    }

    /// Cria um objeto dentro de outro (na posição dele, mais o que o objeto
    /// já traz) e seleciona.
    pub fn add_child(&mut self, parent: u32, mut o: Object) -> Option<u32> {
        self.data.find(parent)?;
        self.record(None);
        let id = self.data.next_id();
        let mut next = id;
        renumber(&mut o, &mut next);
        let n = self
            .data
            .find(parent)
            .map(|p| p.children.len())
            .unwrap_or(0);
        self.data.insert(Some(parent), n, o);
        self.collapsed.remove(&parent);
        self.selection = vec![id];
        self.flash_new(&[id]);
        Some(id)
    }

    /// Duplica a seleção (o mesmo que [`Self::duplicate_selection`]).
    pub fn duplicate_any(&mut self) {
        self.duplicate_selection();
    }

    /// Duplica a seleção: cada cópia anda `step3` (no mundo 3D, em metros)
    /// ou `step2` (no 2D e no HUD) a partir do original. Seleciona e
    /// devolve as cópias.
    fn duplicate_by(&mut self, step3: [f64; 3], step2: [f64; 3]) -> Vec<u32> {
        let keep = self.prefab_root();
        let roots: Vec<u32> = self
            .selection_roots()
            .into_iter()
            .filter(|id| Some(*id) != keep)
            .collect();
        if roots.is_empty() {
            return Vec::new();
        }
        self.record(None);
        let mut sources = Vec::new();
        let mut copies = Vec::new();
        for id in roots {
            let Some(n) = self.data.duplicate(id) else {
                continue;
            };
            if let Some(mut p) = self.world_position(id) {
                let step = if p.three_d { step3 } else { step2 };
                if step != [0.0; 3] {
                    for (v, s) in p.pos.iter_mut().zip(step) {
                        *v = round2(*v + s);
                    }
                    self.move_to(n, p);
                }
            }
            sources.push(id);
            copies.push(n);
        }
        self.selection = copies.clone();
        self.flash_new(&copies);
        self.last_dup = Some(LastDup {
            sources,
            copies: copies.clone(),
        });
        copies
    }

    /// O passo da última duplicação, se a seleção são as cópias dela
    /// (duplicou e moveu a cópia: o próximo Ctrl+D anda o mesmo tanto).
    fn learned_step(&self) -> Option<WorldPos> {
        let d = self.last_dup.as_ref()?;
        if d.copies.is_empty() || d.copies != self.selection_roots() {
            return None;
        }
        let a = self.world_position(d.sources[0])?;
        let b = self.world_position(d.copies[0])?;
        let step = [
            b.pos[0] - a.pos[0],
            b.pos[1] - a.pos[1],
            b.pos[2] - a.pos[2],
        ];
        (a.three_d == b.three_d && step.iter().any(|v| v.abs() > 1e-6)).then_some(WorldPos {
            three_d: b.three_d,
            pos: step,
        })
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
        let text = if self.is_prefab() {
            let Some(obj) = self.data.objects.first() else {
                return Err("o prefab precisa de um objeto".into());
            };
            scene_data::write_prefab(obj)
        } else {
            scene_data::write(&self.data)
        };
        crate::files::write_atomic(&root.join(&self.rel), &text).map_err(|e| e.to_string())?;
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
        if !root.join(&self.rel).is_file() {
            return;
        }
        if read_doc(root, &self.rel).ok().as_ref() == Some(&self.saved) {
            return;
        }
        if self.is_dirty() {
            self.external_change = true;
        } else {
            self.reload(root);
        }
    }

    /// A versão salva usa este prefab?
    pub fn saved_uses_prefab(&self, rel: &str) -> bool {
        self.saved.uses_prefab(rel)
    }

    /// Um prefab mudou no disco: as instâncias passam a usar o novo, sem
    /// perder o que cada uma mudou (e sem a cena ficar "não salva" por isso).
    pub fn relink_prefabs(&mut self, root: &Path) {
        self.data.link_prefabs(root);
        self.saved.link_prefabs(root);
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
        self.finish_repeat(false);
        let ok = self.history.undo(&mut self.data);
        self.selection.retain(|id| self.data.find(*id).is_some());
        ok
    }

    pub fn redo(&mut self) -> bool {
        self.finish_repeat(false);
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
        // O objeto principal de um prefab não sai (o prefab é ele).
        let keep = self.prefab_root();
        let ids: Vec<u32> = self
            .selection_roots()
            .into_iter()
            .filter(|id| Some(*id) != keep)
            .collect();
        if ids.is_empty() {
            return;
        }
        self.record(None);
        let n = ids.len();
        for id in ids {
            self.data.remove(id);
        }
        self.selection.clear();
        self.notices
            .push(count_text(n, "objeto apagado", "objetos apagados"));
    }

    /// Num prefab, o objeto principal.
    pub fn prefab_root(&self) -> Option<u32> {
        if self.is_prefab() {
            self.data.objects.first().map(|o| o.id)
        } else {
            None
        }
    }

    /// Ctrl+D: duplica a seleção um passo ao lado (no 2D, um quadrado da
    /// grade para a direita e para baixo; no 3D, um passo da grade em X). Se
    /// a seleção são as cópias da última duplicação, repete o passo dela.
    pub fn duplicate_selection(&mut self) {
        let mut step3 = [self.grid3.max(0.5), 0.0, 0.0];
        let mut step2 = [self.grid, -self.grid, 0.0];
        match self.learned_step() {
            Some(s) if s.three_d => step3 = s.pos,
            Some(s) => step2 = s.pos,
            None => {}
        }
        self.duplicate_by(step3, step2);
    }

    /// Duplica no mesmo lugar (Alt+arrastar e Shift+D levam as cópias).
    fn duplicate_in_place(&mut self) -> Vec<u32> {
        self.duplicate_by([0.0; 3], [0.0; 3])
    }

    /// A seleção como texto para a área de transferência: um pedaço de cena
    /// (dá para colar em outra cena, ou noutro NN Studio aberto). Os filhos
    /// copiados sozinhos vão com a posição do mundo.
    pub fn selection_text(&self) -> Option<String> {
        let keep = self.prefab_root();
        let mut objects = Vec::new();
        for id in self.selection_roots() {
            if Some(id) == keep {
                continue;
            }
            let Some(mut o) = self.data.find(id).cloned() else {
                continue;
            };
            if self.data.parent_of(id).flatten().is_some() {
                if self.data.three_d && !self.on_screen3(id) {
                    if let Some(m) = self.data.world_matrix3(id) {
                        scene_data::set_local_matrix3(&mut o, m);
                    }
                } else if !self.data.three_d
                    && let Some(t) = self.data.world_transform(id)
                {
                    (o.x, o.y, o.rotation) = (t.x, t.y, t.rotation);
                    (o.scale_x, o.scale_y) = (t.scale_x, t.scale_y);
                }
            }
            objects.push(o);
        }
        if objects.is_empty() {
            return None;
        }
        let piece = SceneData {
            three_d: self.data.three_d,
            objects,
            ..SceneData::default()
        };
        Some(format!("{CLIP_MARK}\n{}", scene_data::write(&piece)))
    }

    /// Ctrl+C: copia a seleção (para o sistema e para o Colar aqui do menu).
    pub fn copy_selection(&self, ctx: &egui::Context) -> bool {
        let Some(text) = self.selection_text() else {
            return false;
        };
        ctx.copy_text(text.clone());
        ctx.data_mut(|d| d.insert_temp(clip_id(), text));
        true
    }

    /// Ctrl+X: copia e apaga.
    pub fn cut_selection(&mut self, ctx: &egui::Context) {
        if self.copy_selection(ctx) {
            let before = self.notices.len();
            self.delete_selection();
            if self.notices.len() > before
                && let Some(last) = self.notices.last_mut()
            {
                *last = last.replace("apagado", "recortado");
            }
        }
    }

    /// Os objetos copiados neste NN Studio (para o Colar aqui).
    pub fn copied_objects(ctx: &egui::Context) -> Option<String> {
        ctx.data(|d| d.get_temp::<String>(clip_id()))
    }

    /// Cola objetos copiados (o texto do Ctrl+C). Com `at`, o grupo vai para
    /// lá (o último copiado no ponto; no 3D, só em X e Z: a altura fica).
    /// Devolve os objetos colados (selecionados), ou nada se o texto não é
    /// de objetos desta cena (2D com 2D, 3D com 3D).
    pub fn paste_objects(&mut self, text: &str, root: &Path, at: Option<WorldPos>) -> Vec<u32> {
        let Some(body) = text.strip_prefix(CLIP_MARK) else {
            return Vec::new();
        };
        let Ok(mut piece) = scene_data::read(body, "colado") else {
            return Vec::new();
        };
        if piece.three_d != self.data.three_d || piece.objects.is_empty() {
            return Vec::new();
        }
        piece.link_prefabs(root);
        self.record(None);
        let parent = self.prefab_root();
        let mut ids = Vec::new();
        for mut o in piece.objects {
            let id = self.data.next_id();
            let mut next = id;
            renumber(&mut o, &mut next);
            let n = match parent {
                Some(p) => self.data.find(p).map(|x| x.children.len()).unwrap_or(0),
                None => self.data.objects.len(),
            };
            self.data.insert(parent, n, o);
            ids.push(id);
        }
        if let Some(at) = at
            && let Some(lead) = ids.last().and_then(|id| self.world_position(*id))
            && lead.three_d == at.three_d
        {
            let mut d = [
                at.pos[0] - lead.pos[0],
                at.pos[1] - lead.pos[1],
                at.pos[2] - lead.pos[2],
            ];
            if at.three_d {
                d[1] = 0.0;
            }
            for id in ids.clone() {
                if let Some(mut p) = self.world_position(id)
                    && p.three_d == at.three_d
                {
                    for (v, dv) in p.pos.iter_mut().zip(d) {
                        *v = round2(*v + dv);
                    }
                    self.move_to(id, p);
                }
            }
        }
        self.selection = ids.clone();
        self.notices
            .push(count_text(ids.len(), "objeto colado", "objetos colados"));
        self.flash_new(&ids);
        ids
    }

    /// Faz o contorno destes objetos brilhar um instante (acabaram de nascer).
    pub fn flash_new(&mut self, ids: &[u32]) {
        if !ids.is_empty() {
            self.sounds.push("criar");
        }
        self.flash.retain(|(id, _)| !ids.contains(id));
        self.flash.extend(ids.iter().map(|id| (*id, None)));
    }

    /// Quanto o contorno de cada objeto novo ainda brilha (1 = começo, some
    /// em 0,7 s). Esquece os que já acabaram.
    pub(super) fn flash_levels(&mut self, now: f64, animate: bool) -> Vec<(u32, f32)> {
        if !animate {
            self.flash.clear();
            return Vec::new();
        }
        const LEN: f64 = 0.7;
        for f in &mut self.flash {
            f.1.get_or_insert(now);
        }
        self.flash.retain(|(_, t)| t.is_some_and(|t| now - t < LEN));
        self.flash
            .iter()
            .map(|(id, t)| (*id, (1.0 - (now - t.unwrap_or(now)) / LEN) as f32))
            .collect()
    }

    /// Ctrl+C, Ctrl+X e Ctrl+V com o mouse na vista ou na Hierarquia. `at`:
    /// onde colar (o mouse na vista); sem ele, no mesmo lugar.
    pub fn clipboard_keys(&mut self, ui: &egui::Ui, root: &Path, at: Option<WorldPos>) {
        let events = ui.input(|i| i.events.clone());
        for e in events {
            match e {
                egui::Event::Copy => {
                    self.copy_selection(ui.ctx());
                }
                egui::Event::Cut => self.cut_selection(ui.ctx()),
                egui::Event::Paste(text) => {
                    self.paste_objects(&text, root, at);
                }
                _ => {}
            }
        }
    }

    /// Shift+D: duplica a seleção e as cópias passam a seguir o mouse.
    pub fn start_carry(&mut self) -> bool {
        let copies = self.duplicate_in_place();
        let objects: Vec<(u32, WorldPos)> = copies
            .iter()
            .filter_map(|id| Some((*id, self.world_position(*id)?)))
            .collect();
        if objects.is_empty() {
            return false;
        }
        let y = objects
            .iter()
            .rev()
            .find(|(_, p)| p.three_d)
            .map_or(0.0, |(_, p)| p.pos[1] as f32);
        self.carry = Some(Carry {
            objects,
            anchor3: None,
            anchor2: None,
            y,
        });
        true
    }

    /// As cópias do Shift+D estão seguindo o mouse?
    pub fn carrying(&self) -> bool {
        self.carry.is_some()
    }

    /// Um quadro do Shift+D: `p2` é o mouse no 2D (ou no HUD da cena 3D);
    /// `p3`, o mouse no plano das cópias do 3D. O primeiro ponto de cada um
    /// é a âncora: as cópias andam o quanto o mouse andou dali.
    fn carry_to(&mut self, p2: Option<(f64, f64)>, p3: Option<glam::Vec3>) {
        let Some(mut c) = self.carry.take() else {
            return;
        };
        c.anchor2 = c.anchor2.or(p2);
        c.anchor3 = c.anchor3.or(p3);
        let mut d2 = match (c.anchor2, p2) {
            (Some(a), Some(b)) => (b.0 - a.0, b.1 - a.1),
            _ => (0.0, 0.0),
        };
        let mut d3 = match (c.anchor3, p3) {
            (Some(a), Some(b)) => (f64::from(b.x - a.x), f64::from(b.z - a.z)),
            _ => (0.0, 0.0),
        };
        // Encaixar: a cópia principal (a última) vai para a grade; as outras
        // acompanham.
        if self.snap
            && let Some((_, lead)) = c.objects.last()
        {
            let snap = |v: f64, g: f64| (v / g).round() * g;
            if lead.three_d {
                let g = self.grid3;
                d3 = (
                    snap(lead.pos[0] + d3.0, g) - lead.pos[0],
                    snap(lead.pos[2] + d3.1, g) - lead.pos[2],
                );
            } else {
                let g = self.grid;
                d2 = (
                    snap(lead.pos[0] + d2.0, g) - lead.pos[0],
                    snap(lead.pos[1] + d2.1, g) - lead.pos[1],
                );
            }
        }
        for (id, p0) in c.objects.clone() {
            let mut p = p0;
            if p.three_d {
                p.pos[0] = round2(p.pos[0] + d3.0);
                p.pos[2] = round2(p.pos[2] + d3.1);
            } else {
                p.pos[0] = round2(p.pos[0] + d2.0);
                p.pos[1] = round2(p.pos[1] + d2.1);
            }
            self.move_to(id, p);
        }
        self.carry = Some(c);
    }

    /// Termina o Shift+D: `keep` solta as cópias onde estão; senão, elas
    /// somem (desfaz a duplicação).
    fn finish_carry(&mut self, keep: bool) {
        if self.carry.take().is_some() {
            if keep {
                self.history.seal();
            } else {
                self.undo();
            }
        }
    }

    /// O Shift+D num quadro da vista (2D e 3D): Esc ou o botão direito
    /// desistem, o clique solta. `p3` só no 3D. Devolve `true` enquanto as
    /// cópias seguem o mouse (a vista não faz mais nada com o mouse).
    fn carry_input(
        &mut self,
        ui: &mut egui::Ui,
        p2: Option<(f64, f64)>,
        p3: Option<glam::Vec3>,
    ) -> bool {
        if self.swallow_press {
            // Até soltar o botão (e no quadro em que solta, que seria um
            // clique na vista).
            if !ui.input(|i| i.pointer.any_down()) {
                self.swallow_press = false;
            }
            return true;
        }
        if self.carry.is_none() {
            return false;
        }
        let (esc, primary, secondary) = ui.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::Escape),
                i.pointer.primary_pressed(),
                i.pointer.secondary_pressed(),
            )
        });
        if esc || secondary {
            self.finish_carry(false);
            self.swallow_press = secondary;
            return true;
        }
        self.carry_to(p2, p3);
        if primary {
            self.finish_carry(true);
            self.swallow_press = true;
        }
        true
    }

    /// O brilho em volta dos objetos que acabaram de nascer (2D).
    fn draw_flash2(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        env: &mut SceneEnv,
        flashes: &[(u32, f32)],
    ) {
        for &(id, level) in flashes {
            let Some((corners, _, _)) = self.screen_corners(env, rect, id) else {
                continue;
            };
            let c = corners.iter().fold(Vec2::ZERO, |a, p| a + p.to_vec2()) / 4.0;
            // O contorno cresce e some.
            let grow = 1.0 + (1.0 - level) * 0.25;
            let mut pts: Vec<Pos2> = corners
                .iter()
                .map(|p| (c + (p.to_vec2() - c) * grow).to_pos2())
                .collect();
            pts.push(pts[0]);
            painter.add(egui::Shape::line(
                pts,
                Stroke::new(1.0 + 3.0 * level, FLASH.gamma_multiply(level)),
            ));
        }
        if !flashes.is_empty() {
            painter.ctx().request_repaint();
        }
    }

    /// A cena está vazia? (No 3D, a câmera sozinha não conta.)
    pub fn is_empty_scene(&self) -> bool {
        self.data.objects.iter().all(|o| {
            self.data.three_d && o.component("camera3d").is_some() && o.children.is_empty()
        })
    }

    /// Abre a busca rápida (Shift+A) em `pos` na tela, para criar no meio da
    /// vista (2D) ou onde a câmera olha (3D).
    pub fn open_quick_add(&mut self, pos: Pos2, root: &Path) {
        let mut q = if self.data.three_d {
            crate::catalog::QuickAdd::new(pos, scene3d::scatter_files(root), "3d")
        } else {
            let files = crate::catalog::project_files(root, &["prefab", "png", "jpg", "jpeg"]);
            crate::catalog::QuickAdd::new(pos, files, "2d")
        };
        q.at = if self.data.three_d {
            let p = self.spawn_point3();
            [f64::from(p.x), f64::from(p.y), f64::from(p.z)]
        } else {
            let (x, y) = self.view_center();
            [x, y, 0.0]
        };
        self.quick_add = Some(q);
    }

    /// O cartão no meio de uma cena vazia: como começar.
    fn empty_card(&mut self, ui: &mut egui::Ui, rect: Rect, env: &mut SceneEnv) {
        if !self.is_empty_scene()
            || self.quick_add.is_some()
            || self.view_menu.is_some()
            || self.prefab_root().is_some()
            || self.painting.is_some()
        {
            return;
        }
        let card = Rect::from_center_size(rect.center(), Vec2::new(360.0, 170.0)).intersect(rect);
        if card.width() < 200.0 || card.height() < 120.0 {
            return;
        }
        let three_d = self.data.three_d;
        let mut search = None;
        ui.scope_builder(egui::UiBuilder::new().max_rect(card), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.label(
                    egui::RichText::new("Sua cena está vazia")
                        .size(16.0)
                        .strong(),
                );
                ui.add_space(4.0);
                ui.label("• Clique com o botão direito aqui: o objeto nasce no ponto do clique.");
                ui.label("• Shift+A abre a busca: luz, cubo, inimigo...");
                ui.label(if three_d {
                    "• Arraste modelos (.glb) e prefabs do painel Arquivos."
                } else {
                    "• Arraste imagens, mapas e prefabs do painel Arquivos."
                });
                ui.add_space(6.0);
                let r = ui.button("🔍 Buscar o que criar");
                crate::tutorial::anchor(ui.ctx(), "cena.vazia", r.rect);
                if r.clicked() {
                    search = Some(r.rect.left_bottom());
                }
            });
        });
        if let Some(p) = search {
            self.open_quick_add(p, env.root);
        }
    }

    /// A dica embaixo da vista enquanto as cópias seguem o mouse.
    fn carry_hint(&self, painter: &egui::Painter, rect: Rect) {
        if self.carry.is_none() {
            return;
        }
        let text = "Clique para soltar a cópia · Esc ou o botão direito desistem";
        let pos = rect.center_bottom() - Vec2::new(0.0, 14.0);
        let font = egui::FontId::proportional(14.0);
        let galley = painter.layout_no_wrap(text.into(), font, Color32::WHITE);
        let bg = Rect::from_center_size(pos, galley.size() + Vec2::new(16.0, 8.0));
        painter.rect_filled(bg, 6.0, Color32::from_black_alpha(170));
        painter.galley(bg.min + Vec2::new(8.0, 4.0), galley, Color32::WHITE);
    }

    /// Cria um objeto na posição `(x, y)` do mundo (na raiz) e seleciona.
    pub fn add_object(&mut self, name: &str, x: f64, y: f64, components: Vec<Component>) -> u32 {
        let mut o = Object::new(0, name);
        o.components = components;
        self.add_prepared(o, x, y)
    }

    /// Põe um objeto pronto na posição `(x, y)` do mundo e seleciona. Num
    /// prefab, ele vira filho do objeto principal.
    pub fn add_prepared(&mut self, mut o: Object, x: f64, y: f64) -> u32 {
        self.record(None);
        let id = self.data.next_id();
        // Ele e os filhos ganham ids novos.
        fn number(o: &mut Object, next: &mut u32) {
            o.id = *next;
            *next += 1;
            for c in &mut o.children {
                number(c, next);
            }
        }
        let mut next = id;
        number(&mut o, &mut next);
        o.x = x.round();
        o.y = y.round();
        // Na tela (HUD), a posição conta a partir do ponto da tela.
        if self.prefab_root().is_none() && self.data.is_screen(&o) {
            let local = self.data.root_unplace(
                &o,
                Transform {
                    x,
                    y,
                    rotation: 0.0,
                    scale_x: 1.0,
                    scale_y: 1.0,
                },
            );
            o.x = local.x.round();
            o.y = local.y.round();
        }
        match self.prefab_root() {
            Some(root) => {
                if let Some(t) = self.data.world_transform(root) {
                    let local = t.to_local(Transform {
                        x: o.x,
                        y: o.y,
                        rotation: 0.0,
                        scale_x: 1.0,
                        scale_y: 1.0,
                    });
                    o.x = local.x.round();
                    o.y = local.y.round();
                }
                let n = self.data.find(root).map(|r| r.children.len()).unwrap_or(0);
                self.data.insert(Some(root), n, o);
            }
            None => self.data.objects.push(o),
        }
        self.selection = vec![id];
        self.flash_new(&[id]);
        id
    }

    /// Algum objeto tem partículas?
    pub fn has_particles(&self) -> bool {
        let mut any = false;
        scene_data::visit(&self.data.objects, &mut |o| {
            any |= o.component("particles").is_some();
        });
        any
    }

    /// Centro da vista atual (onde objetos novos aparecem).
    pub fn view_center(&self) -> (f64, f64) {
        (self.camera.x, self.camera.y)
    }

    /// Enquadra a seleção (ou a cena toda).
    pub fn frame_selection(&mut self, rect: Rect, root: &Path, sizes: &mut Sizes) {
        // Sem seleção: tudo o que aparece (objetos escondidos não contam).
        let items: Vec<(Object, Transform)> = if self.selection.is_empty() {
            scene_render::draw_order(&self.data)
                .iter()
                .map(|p| (p.object.clone(), p.world))
                .collect()
        } else {
            self.selection
                .iter()
                .filter_map(|id| {
                    Some((
                        self.data.find(*id)?.clone(),
                        self.data.world_transform(*id)?,
                    ))
                })
                .collect()
        };
        let mut min = (f64::MAX, f64::MAX);
        let mut max = (f64::MIN, f64::MIN);
        for (o, t) in &items {
            let (r, _) = local_bounds(o, root, sizes, 32.0);
            for (cx, cy) in r.corners() {
                let (wx, wy) = t.point_to_world(cx * t.scale_x, cy * t.scale_y);
                min = (min.0.min(wx), min.1.min(wy));
                max = (max.0.max(wx), max.1.max(wy));
            }
        }
        // A cena toda (sem seleção) também mostra a tela do jogo inteira.
        if self.selection.is_empty() {
            let (cx, cy, zoom) = self.data.camera_start();
            let (sw, sh) = scene_data::screen_size();
            let (hw, hh) = (sw / zoom / 2.0, sh / zoom / 2.0);
            min = (min.0.min(cx - hw), min.1.min(cy - hh));
            max = (max.0.max(cx + hw), max.1.max(cy + hh));
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
        self.hit_among(env, rect, p, &scene_render::draw_order(&self.data))
    }

    /// O objeto de `list` (em ordem de desenho) debaixo de `p`.
    fn hit_among(
        &self,
        env: &mut SceneEnv,
        rect: Rect,
        p: Pos2,
        list: &[scene_render::Placed<'_>],
    ) -> Option<u32> {
        let (wx, wy) = self.camera.to_world(rect, p);
        let icon = scene_render::ICON_SIZE / self.camera.zoom;
        for placed in list.iter().rev() {
            let o = placed.object;
            let owner_locked =
                !placed.is_own() && self.data.find(placed.owner).is_some_and(|w| w.locked);
            if o.locked || owner_locked {
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
                return Some(placed.owner);
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
            None => match self.data.find(id) {
                Some(o) => {
                    let t = self.data.root_unplace(
                        o,
                        Transform {
                            x: wx,
                            y: wy,
                            rotation: 0.0,
                            scale_x: 1.0,
                            scale_y: 1.0,
                        },
                    );
                    (t.x, t.y)
                }
                None => (wx, wy),
            },
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
        if self.data.three_d {
            actions.extend(self.ui_3d(ui, env));
            self.repeat_window(ui, env);
            return actions;
        }
        self.toolbar(ui, env, &mut actions);
        // Grupos: os filhos ficam onde o jogo vai pôr.
        scene_render::arrange_all(&mut self.data, env.root, env.sizes);

        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Other, true, "Vista da cena")
        });
        self.view_rect = Some(rect);
        crate::tutorial::anchor(ui.ctx(), "cena", rect);
        if !self.framed {
            self.framed = true;
            self.frame_selection(rect, env.root, env.sizes);
        }
        if std::mem::take(&mut self.focus_request) {
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
                    particles: !self.particles_paused,
                    bursts: std::mem::take(&mut self.burst_requests),
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

        self.draw_guides(&painter, rect);
        if self.painting.is_some() {
            self.paint_input(ui, &response, rect, env, &mut actions);
            self.draw_overlays(&painter, rect, env);
            self.paint_overlay(&painter, ui, &response, rect, env);
        } else {
            self.handle_input(ui, &response, rect, env, &mut actions);
            self.draw_overlays(&painter, rect, env);
            let flashes = self.flash_levels(ui.input(|i| i.time), env.animate);
            self.draw_flash2(&painter, rect, env, &flashes);
            self.carry_hint(&painter, rect);
            self.empty_card(ui, rect, env);
            self.view_menu2(&response, rect, env, &mut actions);
        }
        if let Some(mut q) = self.quick_add.take() {
            use crate::catalog::{QuickResult, catalog2, quick_add_ui, with_favorites};
            let entries = with_favorites(catalog2(), env.root, false);
            match quick_add_ui(ui.ctx(), &mut q, &entries) {
                QuickResult::Open => self.quick_add = Some(q),
                QuickResult::Close => {}
                QuickResult::Pick(p) => {
                    let snap = |v: f64| {
                        if self.snap {
                            (v / self.grid).round() * self.grid
                        } else {
                            v
                        }
                    };
                    let at = (snap(q.at[0]), snap(q.at[1]));
                    self.place_picked(env, p, Some(at), &mut actions);
                }
            }
        }

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
        self.repeat_window(ui, env);
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
                    let (fw, fh) = sheet_frame(w, h);
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
            crate::files::FileKind::Prefab => {
                // Um prefab não pode ficar dentro dele mesmo.
                if rel == self.rel {
                    return None;
                }
                let base = scene_data::load_prefab(env.root, rel).ok()?;
                let mut o = Object::new(0, &base.name);
                o.layer = base.layer;
                o.prefab = Some(scene_data::PrefabLink {
                    path: rel.to_string(),
                    base: None,
                    raw: Vec::new(),
                });
                o.link_prefab(base);
                return Some(self.add_prepared(o, wx, wy));
            }
            _ => return None,
        };
        Some(self.add_object(&name, wx, wy, vec![comp]))
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, env: &mut SceneEnv, actions: &mut Vec<SceneAction>) {
        ui.horizontal_wrapped(|ui| {
            let r = ui
                .add(egui::Button::new("▶ Testar daqui").fill(crate::theme::PLAY.gamma_multiply(0.55)))
                .on_hover_text("Roda o jogo começando nesta cena (F6)");
            crate::tutorial::anchor(ui.ctx(), "cena.testar", r.rect);
            if r.clicked() {
                actions.push(SceneAction::TestHere);
            }
            if env.connected {
                ui.colored_label(GREEN, "• Ao vivo").on_hover_text(
                    "O jogo aberto está nesta cena: o que você mudar aparece nele na hora, sem salvar. Os contornos tracejados mostram onde os objetos estão no jogo. Ctrl+clique num objeto na janela do jogo seleciona ele aqui.",
                );
                let paused = env.game.is_some_and(|g| g.paused);
                if paused {
                    if ui.button("Continuar").on_hover_text("Volta a rodar o jogo").clicked() {
                        actions.push(SceneAction::Game("resume"));
                    }
                    if ui.button("Avançar 1 quadro").clicked() {
                        actions.push(SceneAction::Game("step"));
                    }
                } else if ui
                    .button("Pausar")
                    .on_hover_text("Congela o jogo aberto (para ver onde as coisas estão)")
                    .clicked()
                {
                    actions.push(SceneAction::Game("pause"));
                }
                paths_button(ui, env, actions);
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
            if ui.available_size_before_wrap().x < 70.0 {
                ui.end_row();
            }
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
            // Pintar o mapa do objeto selecionado.
            let map_obj = match (self.painting, self.selection.as_slice()) {
                (Some(id), _) => Some(id),
                (None, [id]) => self
                    .data
                    .find(*id)
                    .filter(|o| o.component("map").is_some_and(|c| !c.str("map").is_empty()))
                    .map(|o| o.id),
                _ => None,
            };
            if let Some(id) = map_obj {
                let on = self.painting.is_some();
                let r = ui
                    .selectable_label(on, "✏ Pintar")
                    .on_hover_text("Pinta os tiles do mapa direto na cena (Esc sai). As ferramentas e os tiles ficam no painel Tiles.");
                crate::tutorial::anchor(ui.ctx(), "cena.pintar", r.rect);
                if r.clicked() {
                    self.painting = if on { None } else { Some(id) };
                    self.paint_stroke = None;
                }
                if on && self.paint_grew && env.connected {
                    ui.colored_label(crate::theme::WARNING, "O mapa cresceu: salve para o jogo ver")
                        .on_hover_text("O que você pinta dentro do mapa aparece na hora; quando o mapa cresce, o jogo aberto recarrega ao salvar (Ctrl+S).");
                }
                ui.separator();
            }
            let menu = ui.menu_button("➕ Objeto", |ui| {
                if let Some(p) = add_object_menu(ui, env.root) {
                    self.place_picked(env, p, None, actions);
                }
            });
            crate::tutorial::anchor(ui.ctx(), "cena.objeto", menu.response.rect);
            menu.response.on_hover_text(
                "Criar um objeto. Com o mouse na cena: o botão direito cria onde você clicar, e Shift+A abre a busca.",
            );
            // Ver o HUD em telas de outras proporções. (A caixa de escolha
            // não pula sozinha para a linha de baixo: sem espaço, ela ficaria
            // cortada na beirada.)
            if ui.available_size_before_wrap().x < 130.0 {
                ui.end_row();
            }
            let ratio = scene_data::preview();
            let current = scene_data::PREVIEWS
                .iter()
                .find(|p| (p.1 - ratio).abs() < 1e-6)
                .map_or("do jogo", |p| p.0);
            let r = egui::ComboBox::from_id_salt("tela-previa")
                .selected_text(format!("📐 {current}"))
                .width(90.0)
                .show_ui(ui, |ui| {
                    for (label, r) in scene_data::PREVIEWS {
                        if ui.selectable_label((r - ratio).abs() < 1e-6, label).clicked() {
                            scene_data::set_preview(r);
                        }
                    }
                })
                .response
                .on_hover_text("Ver como o HUD e os menus ficam em telas de outros formatos (só no editor)");
            crate::tutorial::anchor(ui.ctx(), "cena.tela", r.rect);
            if self.has_particles() {
                let r = ui
                    .selectable_label(!self.particles_paused, "✨ Partículas")
                    .on_hover_text("Prévia das partículas animando na cena (clique para pausar)");
                if r.clicked() {
                    self.particles_paused = !self.particles_paused;
                }
            }
            ui.label(format!("{:.0}%", self.camera.zoom * 100.0))
                .on_hover_text("Zoom: role o mouse. Mover a vista: botão do meio, ou Espaço + arrastar.");
            if ui.button("⛶ Enquadrar").on_hover_text("F").clicked()
                && let Some(rect) = self.view_rect
            {
                self.frame_selection(rect, env.root, env.sizes);
            }
        });
    }

    fn draw_guides(&self, painter: &egui::Painter, rect: Rect) {
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
        let (sw, sh) = scene_data::screen_size();
        let (w, h) = (sw / zoom, sh / zoom);
        let a = cam.to_screen(rect, cx - w / 2.0, cy + h / 2.0);
        let b = cam.to_screen(rect, cx + w / 2.0, cy - h / 2.0);
        painter.rect_stroke(
            Rect::from_two_pos(a, b),
            0.0,
            Stroke::new(1.5, Color32::from_white_alpha(120)),
            egui::StrokeKind::Middle,
        );
        // O nome fica por fora da moldura, para não cobrir o HUD no canto.
        painter.text(
            a + Vec2::new(2.0, -2.0),
            egui::Align2::LEFT_BOTTOM,
            &if scene_data::preview() > 0.0 {
                format!("tela do jogo ({sw:.0}×{sh:.0})")
            } else {
                "tela do jogo".to_string()
            },
            egui::FontId::proportional(11.0),
            Color32::from_white_alpha(150),
        );
    }

    /// Onde a câmera começa: no objeto com Câmera (ou em quem ele segue).
    fn camera_start(&self) -> (f64, f64, f64) {
        self.data.camera_start()
    }

    fn draw_overlays(&self, painter: &egui::Painter, rect: Rect, env: &mut SceneEnv) {
        let cam = self.camera;
        let order = scene_render::draw_order(&self.data);
        let icon_size = scene_render::ICON_SIZE / cam.zoom;
        for placed in &order {
            let (o, t) = (placed.object, &placed.world);
            // Ícone de objetos sem nada visível (vazio, som, câmera, partículas).
            // (Medir só quem pode não ter nada visível: cenas grandes.)
            let visual = o.components.iter().any(|c| {
                matches!(c.kind.as_str(), "sprite" | "shape" | "text" | "map")
                    || crate::scene_data::is_ui_kind(&c.kind)
            });
            let is_icon = !visual && local_bounds(o, env.root, env.sizes, icon_size).1;
            let center = cam.to_screen(rect, t.x, t.y);
            // (Partículas animando já aparecem: o ícone só com a prévia parada.)
            let fx_icon = self.particles_paused && o.component("particles").is_some();
            let only_fx = is_icon
                && !self.particles_paused
                && o.component("particles").is_some()
                && o.components.iter().all(|c| c.kind == "particles");
            if (is_icon && !only_fx)
                || fx_icon
                || o.components
                    .iter()
                    .any(|c| matches!(c.kind.as_str(), "sound" | "camera"))
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
                // O corpo pode ficar deslocado do objeto (ox, oy).
                let (bx, by) = (t.x + b.num("ox") * t.scale_x, t.y + b.num("oy") * t.scale_y);
                if b.str("shape") == "circle" {
                    let r = b.num("radius") * t.scale_x.abs().max(t.scale_y.abs()) * cam.zoom;
                    painter.circle_stroke(cam.to_screen(rect, bx, by), r as f32, stroke);
                } else {
                    let (w, h) = (b.num("w") * t.scale_x.abs(), b.num("h") * t.scale_y.abs());
                    let a = cam.to_screen(rect, bx - w / 2.0, by + h / 2.0);
                    let bb = cam.to_screen(rect, bx + w / 2.0, by - h / 2.0);
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
        // Onde os objetos estão no jogo aberto (quando estão em outro lugar).
        if let Some(game) = env.game {
            let icon = scene_render::ICON_SIZE / cam.zoom;
            let color = Color32::from_rgb(0x5A, 0xD8, 0xE8);
            for (id, g) in &game.objects {
                let (Some(o), Some(t)) = (self.data.find(*id), self.data.world_transform(*id))
                else {
                    continue;
                };
                let w = g.world;
                let moved = (w.x - t.x).abs() > 0.5
                    || (w.y - t.y).abs() > 0.5
                    || (w.rotation - t.rotation).abs() > 0.5;
                if !moved || !g.visible {
                    continue;
                }
                let (r, _) = local_bounds(o, env.root, env.sizes, icon);
                let pts: Vec<Pos2> = r
                    .corners()
                    .iter()
                    .map(|(cx, cy)| {
                        let (wx, wy) = w.point_to_world(cx * w.scale_x, cy * w.scale_y);
                        cam.to_screen(rect, wx, wy)
                    })
                    .collect();
                for i in 0..pts.len() {
                    let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
                    // Sombra escura embaixo: aparece em qualquer fundo.
                    painter.add(egui::Shape::dashed_line(
                        &[a, b],
                        Stroke::new(3.5, Color32::from_black_alpha(150)),
                        6.0,
                        4.0,
                    ));
                    painter.add(egui::Shape::dashed_line(
                        &[a, b],
                        Stroke::new(1.5, color),
                        6.0,
                        4.0,
                    ));
                }
                let from = cam.to_screen(rect, t.x, t.y);
                let to = cam.to_screen(rect, w.x, w.y);
                painter.line_segment([from, to], Stroke::new(1.0, color.gamma_multiply(0.5)));
            }
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
        // Partículas do objeto selecionado: a área e a direção (a seta
        // arrasta).
        if let Some(&id) = self.selection.last()
            && let (Some(o), Some(t)) = (self.data.find(id), self.data.world_transform(id))
            && let Some(c) = o.component("particles")
        {
            let center = cam.to_screen(rect, t.x, t.y);
            let stroke = Stroke::new(1.5, FX);
            match c.str("area") {
                "retangulo" => {
                    let (w, h) = (c.num("area_w"), c.num("area_h"));
                    let a = cam.to_screen(rect, t.x - w / 2.0, t.y + h / 2.0);
                    let b = cam.to_screen(rect, t.x + w / 2.0, t.y - h / 2.0);
                    let r = Rect::from_two_pos(a, b);
                    let pts = [
                        r.left_top(),
                        r.right_top(),
                        r.right_bottom(),
                        r.left_bottom(),
                        r.left_top(),
                    ];
                    painter.add(egui::Shape::dashed_line(&pts, stroke, 6.0, 4.0));
                }
                "circulo" => {
                    let r = (c.num("area_radius") * cam.zoom) as f32;
                    painter.circle_stroke(center, r, stroke);
                }
                _ => {}
            }
            let dir = c.num("direction").to_radians() as f32;
            let half = (c.num("spread").clamp(0.0, 360.0).to_radians() / 2.0) as f32;
            let unit = |a: f32| Vec2::new(a.cos(), -a.sin());
            if half < std::f32::consts::PI - 0.01 {
                for a in [dir - half, dir + half] {
                    painter.line_segment(
                        [center, center + unit(a) * FX_LEN * 0.7],
                        Stroke::new(1.0, FX.gamma_multiply(0.6)),
                    );
                }
            } else {
                painter.circle_stroke(
                    center,
                    FX_LEN * 0.7,
                    Stroke::new(1.0, FX.gamma_multiply(0.6)),
                );
            }
            let tip = center + unit(dir) * FX_LEN;
            arrow(painter, center, tip, FX);
            painter.circle_filled(tip, 5.0, FX);
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
        // A ponta da seta das partículas vem antes das alças.
        if let Some(o) = self.data.find(id)
            && let Some(ci) = o.components.iter().position(|k| k.kind == "particles")
        {
            let a = o.components[ci].num("direction").to_radians() as f32;
            let tip = c + Vec2::new(a.cos(), -a.sin()) * FX_LEN;
            if (tip - p).length() < HANDLE * 1.5 {
                return Some(DragKind::Direction { ci });
            }
        }
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

    /// Cria o que foi escolhido num menu ou na busca rápida, em `at` (ou
    /// no meio da vista).
    fn place_picked(
        &mut self,
        env: &mut SceneEnv,
        picked: crate::catalog::Picked,
        at: Option<(f64, f64)>,
        actions: &mut Vec<SceneAction>,
    ) -> Option<u32> {
        use crate::catalog::Picked;
        let (x, y) = at.unwrap_or_else(|| self.view_center());
        match picked {
            Picked::Object(o) => Some(self.add_prepared(o, x, y)),
            Picked::File(rel) => self.drop_file(env, &rel, x, y),
            Picked::NewMap => {
                actions.push(SceneAction::NewMap);
                None
            }
            Picked::Remove(rel) => {
                actions.push(SceneAction::Object(
                    crate::hierarchy::HierarchyAction::AskDelete(rel),
                ));
                None
            }
        }
    }

    /// O menu do botão direito na vista 2D: clicar e soltar abre (arrastar
    /// continua movendo a vista).
    fn view_menu2(
        &mut self,
        response: &egui::Response,
        rect: Rect,
        env: &mut SceneEnv,
        actions: &mut Vec<SceneAction>,
    ) {
        let open = if response.secondary_clicked() {
            let p = response.interact_pointer_pos().unwrap_or(rect.center());
            Some(ViewMenu {
                target: self.hit(env, rect, p),
                at2: self.camera.to_world(rect, p),
                at3: glam::Vec3::ZERO,
                ground: false,
            })
        } else {
            None
        };
        self.view_menu(response, open, env, actions);
    }

    /// Um chão: plano, terreno, mapa de tiles, ou (no 3D) algo grande e
    /// baixo, como uma ilha (pelo menos 12 × 12 m, e menos alto que a metade
    /// da largura). O botão direito nele é para criar em cima.
    pub(super) fn is_ground(&self, id: u32, env: &mut SceneEnv) -> bool {
        let Some(o) = self.data.find(id) else {
            return false;
        };
        if o.component("terrain").is_some()
            || o.component("map").is_some()
            || o.component("shape3d")
                .is_some_and(|c| c.str("shape") == "plane")
        {
            return true;
        }
        if !self.data.three_d || self.on_screen3(id) {
            return false;
        }
        let Some(m) = self.data.world_matrix3(id) else {
            return false;
        };
        let (a, b) = scene_render::bounds3(o, env.root, env.sizes);
        let size = (b - a) * m.to_scale_rotation_translation().0.abs();
        size.x >= 12.0 && size.z >= 12.0 && size.y < size.x.min(size.z) * 0.5
    }

    /// O menu do botão direito na vista (2D e 3D). `open`: o clique que abre
    /// (em cima de quem e onde). Em cima de um objeto, o menu dele e Criar
    /// aqui; no vazio ou num chão, o que dá para criar ali.
    fn view_menu(
        &mut self,
        response: &egui::Response,
        open: Option<ViewMenu>,
        env: &mut SceneEnv,
        actions: &mut Vec<SceneAction>,
    ) {
        let popup_id = egui::Popup::default_response_id(response);
        let was_open = egui::Popup::is_id_open(&response.ctx, popup_id);
        // Botão direito em outro lugar com o menu aberto: o egui fecha o
        // menu nesse clique; ele reabre no quadro seguinte, no ponto novo.
        let open = match open {
            Some(m) if was_open => {
                self.view_menu_reopen = Some(m);
                response.ctx.request_repaint();
                None
            }
            Some(m) => Some(m),
            None => self.view_menu_reopen.take(),
        };
        let command = if let Some(mut m) = open {
            m.ground = m.target.is_some_and(|id| self.is_ground(id, env));
            if let Some(id) = m.target
                && !m.ground
                && !self.selection.contains(&id)
            {
                self.selection = vec![id];
            }
            self.view_menu = Some(m);
            Some(egui::containers::SetOpenCommand::Bool(true))
        } else if response.clicked() {
            Some(egui::containers::SetOpenCommand::Bool(false))
        } else {
            None
        };
        let three_d = self.data.three_d;
        let root = env.root;
        let add = |ui: &mut egui::Ui| {
            if three_d {
                scene3d::add_menu3(ui, root)
            } else {
                add_object_menu(ui, root)
            }
        };
        let mut picked = None;
        let mut paste = None;
        let mut out = Vec::new();
        let shown = egui::Popup::context_menu(response)
            .open_memory(command)
            .show(|ui| {
                let Some(m) = self.view_menu else {
                    ui.close();
                    return;
                };
                ui.set_min_width(170.0);
                let target = m.target.and_then(|id| {
                    let name = self.data.find(id)?.name.clone();
                    Some((id, name, m.ground))
                });
                match target {
                    Some((id, name, false)) => {
                        ui.label(egui::RichText::new(name).strong());
                        ui.separator();
                        crate::hierarchy::object_menu(ui, self, id, root, &mut out);
                        ui.separator();
                        let r = ui.menu_button("➕ Criar aqui", |ui| picked = add(ui));
                        crate::tutorial::anchor(ui.ctx(), "cena.menu.criar", r.response.rect);
                    }
                    Some((id, name, true)) => {
                        ui.weak("Criar aqui");
                        picked = add(ui);
                        ui.separator();
                        ui.menu_button(name, |ui| {
                            crate::hierarchy::object_menu(ui, self, id, root, &mut out);
                        });
                    }
                    None => {
                        ui.weak("Criar aqui");
                        picked = add(ui);
                    }
                }
                if let Some(text) = Self::copied_objects(ui.ctx()) {
                    ui.separator();
                    if crate::hierarchy::menu_item(ui, "📋 Colar aqui", "Ctrl+V").clicked() {
                        paste = Some(text);
                        ui.close();
                    }
                }
            });
        if shown.is_none() {
            self.view_menu = None;
        }
        if let (Some(text), Some(m)) = (paste, self.view_menu) {
            let at = if three_d {
                WorldPos {
                    three_d: true,
                    pos: [f64::from(m.at3.x), f64::from(m.at3.y), f64::from(m.at3.z)],
                }
            } else {
                WorldPos {
                    three_d: false,
                    pos: [m.at2.0, m.at2.1, 0.0],
                }
            };
            self.paste_objects(&text, env.root, Some(at));
        }
        if let (Some(p), Some(m)) = (picked, self.view_menu) {
            if three_d {
                self.place_picked3(env, p, Some(m.at3), actions);
            } else {
                let snap = |v: f64| {
                    if self.snap {
                        (v / self.grid).round() * self.grid
                    } else {
                        v
                    }
                };
                let at = (snap(m.at2.0), snap(m.at2.1));
                self.place_picked(env, p, Some(at), actions);
            }
        }
        actions.extend(out.into_iter().map(SceneAction::Object));
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
        // Shift+D: as cópias seguem o mouse até o clique.
        let p2 = pointer.map(|p| self.camera.to_world(rect, p));
        if self.carry_input(ui, p2, None) {
            return;
        }
        if response.hovered()
            && !busy(ui)
            && pointer.is_some()
            && ui.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::D))
        {
            self.start_carry();
            self.carry_to(p2, None);
            return;
        }
        // Ctrl+C, Ctrl+X e Ctrl+V (colar onde o mouse está).
        if response.hovered() && !busy(ui) && self.quick_add.is_none() {
            let at = p2.map(|(x, y)| WorldPos {
                three_d: false,
                pos: [x, y, 0.0],
            });
            self.clipboard_keys(ui, env.root, at);
        }
        // Shift+A: a busca rápida para criar, onde o mouse está.
        if response.hovered()
            && !busy(ui)
            && let Some(p) = pointer
            && ui.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::A))
        {
            let (x, y) = self.camera.to_world(rect, p);
            let files = crate::catalog::project_files(env.root, &["prefab", "png", "jpg", "jpeg"]);
            let mut q = crate::catalog::QuickAdd::new(p, files, "2d");
            q.at = [x, y, 0.0];
            self.quick_add = Some(q);
        }
        if self.quick_add.is_none() && !busy(ui) && (response.hovered() || response.has_focus()) {
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
                // Alt + arrastar a seta: leva uma cópia.
                let copy = modifiers.alt && matches!(kind, DragKind::Move(_));
                if copy {
                    self.duplicate_in_place();
                }
                self.start_drag(kind, start);
                if copy && let Some(d) = &mut self.drag {
                    d.recorded = true;
                }
            } else if let Some(id) = self.hit(env, rect, start) {
                if !self.selection.contains(&id) {
                    if modifiers.shift || modifiers.command {
                        self.selection.push(id);
                    } else {
                        self.selection = vec![id];
                    }
                }
                // Alt + arrastar o objeto: leva uma cópia (o original fica).
                if modifiers.alt && !self.duplicate_in_place().is_empty() {
                    self.start_drag(DragKind::Move(None), start);
                    if let Some(d) = &mut self.drag {
                        d.recorded = true;
                    }
                    return;
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
                    .filter(|p| p.is_own() && !p.object.locked)
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

    /// O mapa do objeto sendo pintado: (arquivo, escala, x e y do canto de
    /// baixo à esquerda no mundo).
    fn paint_target(&self) -> Option<(u32, String, f64, f64, f64)> {
        let id = self.painting?;
        let o = self.data.find(id)?;
        let c = o.component("map")?;
        let t = self.data.world_transform(id)?;
        Some((
            id,
            c.str("map").to_string(),
            c.num("scale").max(0.01),
            t.x,
            t.y,
        ))
    }

    /// Célula do mapa sob um ponto da tela (pode estar fora do mapa).
    fn paint_cell(
        &self,
        rect: Rect,
        p: Pos2,
        map: &crate::map_doc::MapDoc,
        scale: f64,
        ox: f64,
        oy: f64,
    ) -> (i64, i64) {
        let (wx, wy) = self.camera.to_world(rect, p);
        let (tw, th) = (map.tile_w as f64 * scale, map.tile_h as f64 * scale);
        let col = ((wx - ox) / tw).floor() as i64;
        let from_bottom = ((wy - oy) / th).floor() as i64;
        (col, map.height as i64 - 1 - from_bottom)
    }

    /// Modo Pintar: o mouse pinta tiles em vez de mexer nos objetos.
    fn paint_input(
        &mut self,
        ui: &mut egui::Ui,
        response: &egui::Response,
        rect: Rect,
        env: &mut SceneEnv,
        actions: &mut Vec<SceneAction>,
    ) {
        let Some((id, rel, scale, _, _)) = self.paint_target() else {
            self.painting = None;
            return;
        };
        if !env.maps.contains_key(&rel) {
            // O espaço de trabalho abre o mapa no próximo quadro.
            return;
        }
        let pointer = response.hover_pos();
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
        if (response.hovered() || response.has_focus()) && !ui.ctx().text_edit_focused() {
            let pressed =
                |ui: &mut egui::Ui, m: Modifiers, k: Key| ui.input_mut(|i| i.consume_key(m, k));
            for tool in crate::paint::PaintTool::ALL {
                if pressed(ui, Modifiers::NONE, tool.key()) {
                    env.paint.tool = tool;
                }
            }
            if pressed(ui, Modifiers::NONE, Key::R) {
                env.paint.stamp = env.paint.stamp.rotated();
            }
            if pressed(ui, Modifiers::NONE, Key::X) {
                env.paint.stamp = env.paint.stamp.flipped_h();
            }
            if pressed(ui, Modifiers::SHIFT, Key::X) {
                env.paint.stamp = env.paint.stamp.flipped_v();
            }
            if pressed(ui, Modifiers::NONE, Key::F) {
                self.frame_selection(rect, env.root, env.sizes);
            }
            if pressed(ui, Modifiers::NONE, Key::Escape) {
                self.painting = None;
                self.paint_stroke = None;
                return;
            }
            if pressed(ui, Modifiers::NONE, Key::F6) {
                actions.push(SceneAction::TestHere);
            }
        }
        // Mover a vista: botão do meio ou Espaço + arrastar.
        let (space, middle) = ui.input(|i| (i.key_down(Key::Space), i.pointer.middle_down()));
        if response.drag_started() && (middle || space) {
            let start = ui
                .input(|i| i.pointer.press_origin())
                .unwrap_or(rect.center());
            self.start_drag(DragKind::Pan { start: self.camera }, start);
        }
        if let (Some(drag), Some(now)) = (self.drag.clone(), response.interact_pointer_pos()) {
            self.continue_drag(&drag, now, rect, Modifiers::NONE, env);
        }
        if response.drag_stopped() || !(middle || space || ui.input(|i| i.pointer.any_down())) {
            self.drag = None;
        }
        if space || middle || self.drag.is_some() {
            return;
        }
        // Pintar: os eventos do mouse em ordem (traços rápidos saem inteiros).
        let events: Vec<egui::Event> = ui.input(|i| i.events.clone());
        let mut changes: Vec<crate::paint::Change> = Vec::new();
        let mut grew = false;
        for e in events {
            let Some(map) = env.maps.get_mut(&rel) else {
                return;
            };
            let Some((_, _, _, ox, oy)) = self.paint_target() else {
                return;
            };
            match e {
                egui::Event::PointerButton {
                    pos,
                    button,
                    pressed: true,
                    modifiers,
                } if rect.contains(pos)
                    && response.contains_pointer()
                    && self.paint_stroke.is_none()
                    && matches!(
                        button,
                        egui::PointerButton::Primary | egui::PointerButton::Secondary
                    ) =>
                {
                    response.request_focus();
                    let cell = self.paint_cell(rect, pos, map, scale, ox, oy);
                    // Botão direito ou Alt: conta-gotas. Ctrl: copiar um pedaço.
                    let pick = button == egui::PointerButton::Secondary || modifiers.alt;
                    let (stroke, out) =
                        env.paint
                            .begin(map, env.tilesets, cell, modifiers.command, pick);
                    grew |= out.grew;
                    changes.extend(out.changes);
                    self.paint_stroke = Some(stroke);
                }
                egui::Event::PointerMoved(pos) => {
                    if let Some(mut stroke) = self.paint_stroke.take() {
                        let cell = self.paint_cell(rect, pos, map, scale, ox, oy);
                        let out = env.paint.drag(map, env.tilesets, &mut stroke, cell);
                        grew |= out.grew;
                        changes.extend(out.changes);
                        self.paint_stroke = Some(stroke);
                    }
                }
                egui::Event::PointerButton {
                    pos,
                    pressed: false,
                    ..
                } => {
                    if let Some(stroke) = self.paint_stroke.take() {
                        let cell = self.paint_cell(rect, pos, map, scale, ox, oy);
                        env.paint.end(map, stroke, cell);
                    }
                }
                _ => {}
            }
            // O mapa cresceu à esquerda ou embaixo: o objeto anda junto.
            self.apply_map_shift(id, &rel, env, scale);
        }
        if self.paint_stroke.is_some() && !ui.input(|i| i.pointer.any_down()) {
            let stroke = self.paint_stroke.take().unwrap();
            if let Some(map) = env.maps.get_mut(&rel) {
                let last = stroke.last;
                env.paint.end(map, stroke, last);
            }
        }
        if grew {
            self.paint_grew = true;
        }
        if !changes.is_empty() && !grew {
            let list: Vec<serde_json::Value> = changes
                .iter()
                .map(|(l, c, r, g)| serde_json::json!([l, c + 1, r + 1, g]))
                .collect();
            self.live.push(LiveChange {
                id,
                component: None,
                field: "tiles".into(),
                value: Prop::Num(0.0),
                json: Some(serde_json::Value::Array(list)),
            });
        }
    }

    /// Leva para o objeto o quanto o mapa cresceu (ou encolheu, ao desfazer)
    /// à esquerda e embaixo, para o desenho ficar no mesmo lugar.
    pub fn apply_map_shift(&mut self, id: u32, rel: &str, env: &mut SceneEnv, scale: f64) {
        let Some(map) = env.maps.get_mut(rel) else {
            return;
        };
        let (dc, dr) = map.take_shift();
        if (dc, dr) == (0, 0) {
            return;
        }
        let (tw, th) = (map.tile_w as f64 * scale, map.tile_h as f64 * scale);
        if let Some(o) = self.data.find_mut(id) {
            o.x -= dc as f64 * tw;
            o.y -= dr as f64 * th;
        }
    }

    /// Por cima da cena, no modo Pintar: o contorno do mapa, a grade, o que
    /// bloqueia e onde o pincel vai pintar.
    fn paint_overlay(
        &self,
        painter: &egui::Painter,
        ui: &egui::Ui,
        response: &egui::Response,
        rect: Rect,
        env: &mut SceneEnv,
    ) {
        let Some((_, rel, scale, ox, oy)) = self.paint_target() else {
            return;
        };
        let Some(map) = env.maps.get(&rel) else {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Abrindo o mapa...",
                egui::FontId::proportional(16.0),
                Color32::WHITE,
            );
            return;
        };
        let cam = self.camera;
        let (tw, th) = (map.tile_w as f64 * scale, map.tile_h as f64 * scale);
        // Canto de cima à esquerda da célula (col, row) na tela.
        let corner = |c: i64, r: i64| {
            cam.to_screen(
                rect,
                ox + c as f64 * tw,
                oy + (map.height as i64 - r) as f64 * th,
            )
        };
        let outline = Rect::from_two_pos(corner(0, 0), corner(map.width as i64, map.height as i64));
        painter.rect_stroke(
            outline,
            0.0,
            Stroke::new(1.5, Color32::from_rgb(0xFF, 0x9F, 0x1C)),
            egui::StrokeKind::Outside,
        );
        let cell_px = (tw * cam.zoom) as f32;
        // Células visíveis.
        let vis = outline.intersect(rect);
        if vis.width() <= 0.0 || vis.height() <= 0.0 {
            return;
        }
        let c0 = (((vis.left() - outline.left()) / cell_px).floor() as i64).max(0);
        let c1 = (((vis.right() - outline.left()) / cell_px).ceil() as i64).min(map.width as i64);
        let rpx = (th * cam.zoom) as f32;
        let r0 = (((vis.top() - outline.top()) / rpx).floor() as i64).max(0);
        let r1 = (((vis.bottom() - outline.top()) / rpx).ceil() as i64).min(map.height as i64);
        if env.paint.show_grid && cell_px >= 8.0 {
            let stroke = Stroke::new(1.0, Color32::from_white_alpha(28));
            for c in c0..=c1 {
                let x = corner(c, 0).x;
                painter.vline(x, vis.y_range(), stroke);
            }
            for r in r0..=r1 {
                let y = corner(0, r).y;
                painter.hline(vis.x_range(), y, stroke);
            }
        }
        // O que bloqueia: vermelho (sólido) e azul em cima (plataforma).
        if env.paint.show_collision && (c1 - c0) * (r1 - r0) <= 40_000 {
            for r in r0..r1 {
                for c in c0..c1 {
                    let mut solid = false;
                    let mut one_way = false;
                    for li in map.tile_layers() {
                        let gid = map.get(li, c, r);
                        if gid == 0 {
                            continue;
                        }
                        let l = map.tile_layer(li).unwrap();
                        if l.collision {
                            solid = true;
                        }
                        if let Some((ts, local)) = map.tileset_of(gid)
                            && let Some(data) = crate::paint::tileset_data(map, ts, env.tilesets)
                        {
                            let info = data.info(local);
                            solid |= info.solid;
                            one_way |= info.one_way;
                        }
                    }
                    let cr = Rect::from_two_pos(corner(c, r), corner(c + 1, r + 1));
                    if solid {
                        painter.rect_filled(
                            cr,
                            0.0,
                            Color32::from_rgba_unmultiplied(0xE5, 0x48, 0x4D, 60),
                        );
                    } else if one_way {
                        painter.hline(
                            cr.x_range(),
                            cr.top() + 1.0,
                            Stroke::new(2.5, Color32::from_rgb(0x4A, 0x9C, 0xF0)),
                        );
                    }
                }
            }
        }
        // Onde o pincel vai pintar.
        if let Some(p) = response.hover_pos() {
            let (c, r) = self.paint_cell(rect, p, map, scale, ox, oy);
            let copying = ui.input(|i| i.modifiers.command);
            let (w, h) = match (env.paint.tool, copying) {
                (_, true)
                | (
                    crate::paint::PaintTool::Eraser
                    | crate::paint::PaintTool::Picker
                    | crate::paint::PaintTool::Fill
                    | crate::paint::PaintTool::Terrain,
                    _,
                ) => (1, 1),
                _ => (env.paint.stamp.w as i64, env.paint.stamp.h as i64),
            };
            let (sc, sr) = match &self.paint_stroke {
                Some(s) if s.copy || env.paint.tool == crate::paint::PaintTool::Rect => (
                    (s.start.0.min(c), s.start.1.min(r)),
                    (s.start.0.max(c), s.start.1.max(r)),
                ),
                _ => ((c, r), (c + w - 1, r + h - 1)),
            };
            let hr = Rect::from_two_pos(corner(sc.0, sc.1), corner(sr.0 + 1, sr.1 + 1));
            let color = if copying || self.paint_stroke.as_ref().is_some_and(|s| s.copy) {
                Color32::from_rgb(0x41, 0xA6, 0xF6)
            } else {
                Color32::WHITE
            };
            painter.rect_stroke(hr, 0.0, Stroke::new(2.0, color), egui::StrokeKind::Inside);
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            let layer = map
                .tile_layer(map.layer)
                .map(|l| l.name.as_str())
                .unwrap_or("");
            painter.text(
                rect.left_bottom() + Vec2::new(8.0, -6.0),
                egui::Align2::LEFT_BOTTOM,
                format!(
                    "{} · camada {layer} · coluna {} · linha {}",
                    env.paint.tool.label(),
                    c + 1,
                    r + 1
                ),
                egui::FontId::monospace(12.0),
                Color32::WHITE,
            );
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
            DragKind::Direction { ci } => {
                let Some(&(id, t)) = drag.objects.last() else {
                    return;
                };
                let c = self.camera.to_screen(rect, t.x, t.y);
                let mut a = angle_of(c, now);
                if snap {
                    a = (a / 15.0).round() * 15.0;
                }
                if a < 0.0 {
                    a += 360.0;
                }
                self.record_drag(drag, "direcao");
                if let Some(k) = self
                    .data
                    .find_mut(id)
                    .and_then(|o| o.components.get_mut(*ci))
                {
                    k.set("direction", Prop::Num(round2(a)));
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
        if pressed(ui, Modifiers::NONE, Key::F2)
            && let Some(&id) = self.selection.last()
        {
            self.start_rename(id);
            actions.push(SceneAction::Object(
                crate::hierarchy::HierarchyAction::ShowHierarchy,
            ));
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

/// Itens do menu "Adicionar objeto": devolve o objeto pronto (com id 0;
/// [`SceneDoc::add_prepared`] numera ele e os filhos).
pub fn add_object_menu(ui: &mut egui::Ui, root: &Path) -> Option<crate::catalog::Picked> {
    let entries = crate::catalog::with_favorites(crate::catalog::catalog2(), root, false);
    crate::catalog::menu(ui, &entries, "2d")
}

/// O botão Caminhos (com o jogo aberto): mostra no jogo por onde os
/// inimigos podem andar e o caminho de cada um.
pub(crate) fn paths_button(ui: &mut egui::Ui, env: &SceneEnv, actions: &mut Vec<SceneAction>) {
    let on = env.game.is_some_and(|g| g.paths);
    let r = ui
        .selectable_label(on, "🗺 Caminhos")
        .on_hover_text("Mostra no jogo aberto por onde os inimigos podem andar (a grade de caminhos) e o caminho que cada um está seguindo");
    crate::tutorial::anchor(ui.ctx(), "cena.caminhos", r.rect);
    if r.clicked() {
        actions.push(SceneAction::Game(if on {
            "hide_paths"
        } else {
            "show_paths"
        }));
    }
}

/// Um menu pronto: painel que arruma 3 botões em coluna.
pub fn menu_with_buttons() -> Object {
    let mut menu = Object::new(0, "menu");
    let mut panel = Component::new("panel");
    panel.set("w", Prop::Num(320.0));
    panel.set("h", Prop::Num(276.0));
    menu.components.push(panel);
    menu.components.push(Component::new("layout"));
    for (name, text, action, focus) in [
        ("jogar", "Jogar", "cena", true),
        ("opcoes", "Opções", "por_cima", false),
        ("sair", "Sair", "sair", false),
    ] {
        let mut b = Object::new(0, name);
        let mut c = Component::new("button");
        c.set("text", Prop::Str(text.into()));
        c.set("action", Prop::Str(action.into()));
        if focus {
            c.set("focus", Prop::Bool(true));
        }
        b.components.push(c);
        menu.children.push(b);
    }
    menu
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

/// Dá ids novos ao objeto e aos filhos, a partir de `next`.
fn renumber(o: &mut Object, next: &mut u32) {
    o.id = *next;
    *next += 1;
    for c in &mut o.children {
        renumber(c, next);
    }
}

/// Um campo de texto com o foco ou um menu aberto: os atalhos da vista
/// esperam (o Esc é do menu, as letras são do campo).
pub(crate) fn busy(ui: &egui::Ui) -> bool {
    ui.ctx().text_edit_focused() || egui::Popup::is_any_open(ui.ctx())
}

/// "1 objeto apagado", "3 objetos apagados".
fn count_text(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
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
    fn quadros_de_spritesheets() {
        assert_eq!(
            sheet_frame(64.0, 32.0),
            (16.0, 16.0),
            "herói: 8 quadros em 2 linhas"
        );
        assert_eq!(sheet_frame(64.0, 16.0), (16.0, 16.0), "moeda: tira");
        assert_eq!(sheet_frame(16.0, 16.0), (16.0, 16.0), "imagem de um quadro");
        assert_eq!(sheet_frame(8.0, 8.0), (8.0, 8.0));
        assert_eq!(sheet_frame(256.0, 64.0), (64.0, 64.0), "tira grande");
        assert_eq!(sheet_frame(300.0, 200.0), (300.0, 200.0), "imagem comum");
    }

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
    fn ambiente_vai_inteiro_para_o_jogo() {
        let a = SceneData::new_3d();
        let mut b = a.clone();
        b.set_env("sun_height", Prop::Num(12.0));
        let d = diff(&a, &b);
        assert_eq!(d.len(), 1);
        assert_eq!((d[0].id, d[0].field.as_str()), (0, "environment"));
        let json: serde_json::Value = serde_json::from_str(&d[0].to_json("m.cena")).unwrap();
        assert_eq!(json["value"]["sun_height"], 12.0);
        assert_eq!(json["value"]["fog"], true);
        // Campos 3D dos objetos também vão.
        let mut c = a.clone();
        c.objects[2].z = 3.0;
        c.objects[2].ry = 45.0;
        let d = diff(&a, &c);
        let fields: Vec<&str> = d.iter().map(|x| x.field.as_str()).collect();
        assert_eq!(fields, ["z", "ry"]);
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
