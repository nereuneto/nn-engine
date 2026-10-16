//! Painel Inspetor: as propriedades do objeto selecionado (ou da cena, sem
//! seleção). Cada componente tem os campos com o controle certo, e dá para
//! arrastar imagens, sons e mapas do painel Arquivos direto para os campos.

use std::collections::HashMap;
use std::time::SystemTime;

use eframe::egui::{self, Color32, RichText, Vec2};

use crate::behaviors::{self, BehaviorInfo, Catalog, PropType};
use crate::files::{FileKind, FileTree};
use crate::scene_data::{Component, Object, Prop};
use crate::scene_view::{FileDrag, GameState, SceneDoc};
use crate::settings::Mode;

/// Tipo de controle de um campo.
#[derive(Debug, Clone, Copy)]
enum W {
    Num {
        speed: f64,
        min: f64,
        max: f64,
        suffix: &'static str,
    },
    Int {
        min: f64,
        max: f64,
    },
    Bool,
    Text,
    Multiline,
    Color,
    Choice(&'static [(&'static str, &'static str)]),
    File(FileKind),
    Object,
}

#[derive(Debug, Clone, Copy)]
struct F {
    key: &'static str,
    label: &'static str,
    w: W,
    advanced: bool,
    help: &'static str,
}

const fn f(key: &'static str, label: &'static str, w: W) -> F {
    F {
        key,
        label,
        w,
        advanced: false,
        help: "",
    }
}

const fn adv(key: &'static str, label: &'static str, w: W) -> F {
    F {
        key,
        label,
        w,
        advanced: true,
        help: "",
    }
}

const fn help(mut x: F, help: &'static str) -> F {
    x.help = help;
    x
}

const SIZE: W = W::Num {
    speed: 1.0,
    min: 0.0,
    max: 100_000.0,
    suffix: "",
};
const ANY: W = W::Num {
    speed: 1.0,
    min: -100_000.0,
    max: 100_000.0,
    suffix: "",
};
const ANCHORS: &[(&str, &str)] = &[
    ("center", "Centro"),
    ("topleft", "Canto superior esquerdo"),
    ("top", "Em cima"),
    ("topright", "Canto superior direito"),
    ("left", "Esquerda"),
    ("right", "Direita"),
    ("bottomleft", "Canto inferior esquerdo"),
    ("bottom", "Embaixo"),
    ("bottomright", "Canto inferior direito"),
];

const SPRITE_FIELDS: &[F] = &[
    f("image", "Imagem", W::File(FileKind::Image)),
    help(
        f(
            "frame_w",
            "Largura do quadro",
            W::Int {
                min: 0.0,
                max: 4096.0,
            },
        ),
        "Para spritesheets: o tamanho de cada quadro. 0 = a imagem inteira.",
    ),
    f(
        "frame_h",
        "Altura do quadro",
        W::Int {
            min: 0.0,
            max: 4096.0,
        },
    ),
    f(
        "frame",
        "Quadro",
        W::Int {
            min: 1.0,
            max: 100_000.0,
        },
    ),
    help(
        f("w", "Largura", SIZE),
        "0 = o tamanho da imagem (ou do quadro).",
    ),
    f("h", "Altura", SIZE),
    help(
        f("color", "Cor", W::Color),
        "Tinge a imagem. Branco = cores originais.",
    ),
    f("flip_x", "Espelhar na horizontal", W::Bool),
    adv("flip_y", "Espelhar na vertical", W::Bool),
    adv("pixel_art", "Pixel art (sem suavizar)", W::Bool),
    HUD,
];

const HUD: F = help(
    f("hud", "Fixo na tela (HUD)", W::Bool),
    "Não anda com a câmera: placar, vidas, botões. A posição conta a partir do centro da tela.",
);

const TEXT_FIELDS: &[F] = &[
    f("text", "Texto", W::Multiline),
    f("size", "Tamanho", SIZE),
    f("color", "Cor", W::Color),
    help(
        f("anchor", "Âncora", W::Choice(ANCHORS)),
        "Qual ponto do texto fica na posição do objeto.",
    ),
    HUD,
];

const SHAPE_FIELDS: &[F] = &[
    f(
        "shape",
        "Forma",
        W::Choice(&[("rect", "Retângulo"), ("circle", "Círculo")]),
    ),
    f("w", "Largura", SIZE),
    f("h", "Altura", SIZE),
    f("radius", "Raio", SIZE),
    f("color", "Cor", W::Color),
    HUD,
];

const BODY_FIELDS: &[F] = &[
    f(
        "body",
        "Tipo",
        W::Choice(&[
            ("dynamic", "Dinâmico (cai e colide)"),
            ("static", "Estático (chão, parede)"),
            ("kinematic", "Cinemático (plataforma móvel)"),
        ]),
    ),
    f(
        "shape",
        "Forma",
        W::Choice(&[("box", "Caixa"), ("circle", "Círculo")]),
    ),
    f("w", "Largura", SIZE),
    f("h", "Altura", SIZE),
    f("radius", "Raio", SIZE),
    help(
        f("sensor", "Sensor", W::Bool),
        "Não bloqueia: só avisa quando algo entra (moedas, portas, espinhos).",
    ),
    help(
        f("one_way", "Mão única", W::Bool),
        "Só bloqueia quem vem de cima (plataformas que dá para pular por baixo).",
    ),
    help(
        f(
            "bounce",
            "Quique",
            W::Num {
                speed: 0.01,
                min: 0.0,
                max: 1.0,
                suffix: "",
            },
        ),
        "0 = para ao bater, 1 = quica sem perder velocidade.",
    ),
    help(
        adv(
            "gravity_scale",
            "Gravidade",
            W::Num {
                speed: 0.05,
                min: -10.0,
                max: 10.0,
                suffix: "×",
            },
        ),
        "Multiplica a gravidade da cena (0 = flutua).",
    ),
    help(
        adv("tag", "Rótulo", W::Text),
        "Rótulo do corpo (padrão: o nome do objeto).",
    ),
];

const MAP_FIELDS: &[F] = &[
    f("map", "Mapa (.tmj)", W::File(FileKind::Map)),
    help(
        f(
            "scale",
            "Escala",
            W::Num {
                speed: 0.1,
                min: 0.1,
                max: 32.0,
                suffix: "×",
            },
        ),
        "Tiles de 16 px com escala 3 viram 48 unidades.",
    ),
    help(
        f("collision", "Colisão dos tiles", W::Bool),
        "Os tiles marcados como sólidos no Tiled bloqueiam os corpos físicos.",
    ),
];

const CAMERA_FIELDS: &[F] = &[
    help(
        f("follow", "Seguir", W::Object),
        "Objeto que a câmera acompanha. Sem nenhum, ela fica parada aqui.",
    ),
    f(
        "zoom",
        "Zoom",
        W::Num {
            speed: 0.05,
            min: 0.05,
            max: 20.0,
            suffix: "×",
        },
    ),
    help(
        f("limit_to_map", "Não passar do mapa", W::Bool),
        "Não mostra nada fora do primeiro mapa da cena.",
    ),
    help(
        adv(
            "smooth",
            "Suavidade",
            W::Num {
                speed: 0.1,
                min: 0.0,
                max: 50.0,
                suffix: "",
            },
        ),
        "Rapidez para alcançar o alvo; 0 vai direto.",
    ),
];

const SOUND_FIELDS: &[F] = &[
    f("sound", "Som", W::File(FileKind::Sound)),
    f("autoplay", "Tocar ao começar", W::Bool),
    f("loop", "Repetir", W::Bool),
    f(
        "volume",
        "Volume",
        W::Num {
            speed: 0.01,
            min: 0.0,
            max: 2.0,
            suffix: "",
        },
    ),
    f(
        "channel",
        "Canal",
        W::Choice(&[("effects", "Efeitos"), ("music", "Música")]),
    ),
];

const PARTICLES_FIELDS: &[F] = &[
    f("rate", "Por segundo", SIZE),
    f(
        "lifetime",
        "Vida (segundos)",
        W::Num {
            speed: 0.05,
            min: 0.0,
            max: 60.0,
            suffix: " s",
        },
    ),
    f("speed_min", "Velocidade mínima", ANY),
    f("speed_max", "Velocidade máxima", ANY),
    help(
        f(
            "direction",
            "Direção",
            W::Num {
                speed: 1.0,
                min: -360.0,
                max: 360.0,
                suffix: "°",
            },
        ),
        "90 = para cima.",
    ),
    help(
        f(
            "spread",
            "Abertura",
            W::Num {
                speed: 1.0,
                min: 0.0,
                max: 360.0,
                suffix: "°",
            },
        ),
        "360 = para todos os lados.",
    ),
    f("gravity", "Gravidade", ANY),
    f("size_start", "Tamanho inicial", SIZE),
    f("size_end", "Tamanho final", SIZE),
    f("color_start", "Cor inicial", W::Color),
    f("color_end", "Cor final", W::Color),
    f(
        "blend",
        "Mistura",
        W::Choice(&[("alpha", "Normal"), ("add", "Luz (somar cores)")]),
    ),
    f("emitting", "Emitindo", W::Bool),
];

fn fields(kind: &str) -> &'static [F] {
    match kind {
        "sprite" => SPRITE_FIELDS,
        "text" => TEXT_FIELDS,
        "shape" => SHAPE_FIELDS,
        "body" => BODY_FIELDS,
        "map" => MAP_FIELDS,
        "camera" => CAMERA_FIELDS,
        "sound" => SOUND_FIELDS,
        "particles" => PARTICLES_FIELDS,
        _ => &[],
    }
}

/// Nome e descrição de cada componente.
pub fn component_info(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "sprite" => (
            "Imagem (sprite)",
            "Mostra uma imagem ou um quadro de spritesheet.",
        ),
        "text" => ("Texto", "Escreve um texto no mundo."),
        "shape" => ("Forma", "Retângulo ou círculo de uma cor."),
        "body" => (
            "Corpo físico",
            "Cai, colide, para no chão. Mexa nele pelo script: objeto.body.vx.",
        ),
        "map" => (
            "Mapa do Tiled",
            "Desenha um mapa .tmj, com a colisão dos tiles.",
        ),
        "camera" => ("Câmera", "Onde a câmera começa, ou quem ela segue."),
        "sound" => ("Som", "Toca um som ou música quando a cena começa."),
        "particles" => ("Partículas", "Fumaça, faíscas, poeira, chuva..."),
        "behavior" => (
            "Comportamento",
            "Um script com propriedades: pronto (andar, pular, coletar...) ou do jogo.",
        ),
        _ => ("Componente", ""),
    }
}

/// O que o inspetor pede ao espaço de trabalho.
#[derive(Debug, Clone, PartialEq)]
pub enum InspectorAction {
    OpenFile(String),
    /// Criar o script da cena.
    CreateScript,
    /// Criar um script de comportamento novo e pôr neste objeto.
    NewBehaviorScript(u32),
    /// Mostrar o painel Eventos.
    ShowEvents,
}

/// Resumo das regras de eventos, com o botão para editar. Devolve `true` se
/// pediu para abrir o painel Eventos.
fn events_summary(ui: &mut egui::Ui, rules: &[crate::scene_data::Rule], in_object: bool) -> bool {
    ui.add_space(8.0);
    let mut open = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Eventos").strong());
        ui.weak(match rules.len() {
            0 => "nenhuma regra".to_string(),
            1 => "1 regra".to_string(),
            n => format!("{n} regras"),
        });
        open = ui
            .button(if rules.is_empty() {
                "➕ Criar"
            } else {
                "Editar"
            })
            .on_hover_text("Regras \"Quando... Então...\": abre o painel Eventos")
            .clicked();
    });
    for r in rules.iter().take(4) {
        let text = crate::events::rule_summary(r, in_object);
        let short: String = if text.chars().count() > 70 {
            text.chars().take(69).chain(std::iter::once('…')).collect()
        } else {
            text.clone()
        };
        ui.add(egui::Label::new(RichText::new(short).small().weak()).truncate())
            .on_hover_text(text);
    }
    if rules.len() > 4 {
        ui.weak(format!("... e mais {}", rules.len() - 4));
    }
    open
}

#[derive(Default)]
pub struct Inspector {
    images: HashMap<String, (Option<SystemTime>, egui::TextureHandle)>,
    /// Informações dos comportamentos (lidas dos scripts).
    pub catalog: Catalog,
    /// Pedidos feitos enquanto desenha.
    actions_out: Vec<InspectorAction>,
    /// O jogo aberto (se mostra a cena deste inspetor).
    game: Option<GameState>,
}

/// Cor do que vem do jogo aberto.
pub const GAME: Color32 = Color32::from_rgb(0x5A, 0xD8, 0xE8);

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Valor como texto curto.
fn prop_text(p: &Prop) -> String {
    match p {
        Prop::Num(n) => crate::scene_data::format_num(round2(*n)),
        Prop::Bool(b) => if *b { "sim" } else { "não" }.to_string(),
        Prop::Str(s) => format!("\"{s}\""),
    }
}

/// Mesmo valor (números com uma folga pequena)?
fn same_value(a: &Prop, b: &Prop) -> bool {
    match (a, b) {
        (Prop::Num(x), Prop::Num(y)) => (x - y).abs() < 1e-6,
        _ => a == b,
    }
}

/// Cor das instâncias de prefab.
pub const PREFAB: Color32 = Color32::from_rgb(0x6A, 0xB0, 0xF5);

/// Um `.lua` que pode ser um comportamento (não é cena nem configuração).
fn is_behavior_candidate(rel: &str) -> bool {
    rel != "config.lua" && rel != "main.lua" && !rel.starts_with("cenas/")
}

/// Rótulo de um campo (em negrito e com ↺ quando a instância mudou o valor
/// do prefab).
fn field_row(ui: &mut egui::Ui, spec: &Spec, overridden: bool, reverts: &mut Vec<String>) -> bool {
    ui.horizontal(|ui| {
        let text = if overridden {
            RichText::new(&spec.label).strong().color(PREFAB)
        } else {
            RichText::new(&spec.label)
        };
        let r = ui.label(text);
        if !spec.help.is_empty() {
            r.on_hover_text(&spec.help);
        }
        if overridden
            && ui
                .small_button("↺")
                .on_hover_text("Voltar ao valor do prefab")
                .clicked()
        {
            reverts.push(spec.key.clone());
        }
    });
    true
}

/// Aplica as mudanças de um componente (um passo de desfazer).
fn apply_changes(
    doc: &mut SceneDoc,
    o: &Object,
    ci: usize,
    changes: Vec<(String, Prop)>,
    reverts: Vec<String>,
) {
    if changes.is_empty() && reverts.is_empty() {
        return;
    }
    let first = changes
        .first()
        .map(|c| c.0.clone())
        .or_else(|| reverts.first().cloned())
        .unwrap_or_default();
    let key = format!("{}:{ci}:{first}", o.id);
    doc.record(if reverts.is_empty() { Some(&key) } else { None });
    let base: Option<Component> = o
        .prefab
        .as_ref()
        .and_then(|l| l.base.as_ref())
        .and_then(|b| b.components.get(ci).cloned());
    if let Some(c) = doc
        .data
        .find_mut(o.id)
        .and_then(|x| x.components.get_mut(ci))
    {
        for (k, v) in changes {
            c.set(&k, v);
        }
        for k in reverts {
            match base.as_ref().and_then(|b| b.get(&k)) {
                Some(v) => c.set(&k, v.clone()),
                None => c.remove(&k),
            }
        }
    }
}

/// Um campo pronto para mostrar: do componente (fixo) ou de um
/// comportamento (vem do script).
#[derive(Debug, Clone)]
struct Spec {
    key: String,
    label: String,
    w: Wd,
    help: String,
}

#[derive(Debug, Clone)]
enum Wd {
    Num {
        speed: f64,
        min: f64,
        max: f64,
        suffix: &'static str,
    },
    Int {
        min: f64,
        max: f64,
    },
    Bool,
    Text,
    Multiline,
    Color,
    Choice(Vec<(String, String)>),
    File(FileKind),
    /// Cena (.cena ou .lua).
    Scene,
    /// Objeto da cena; o texto é o que mostrar quando vazio.
    Object(&'static str),
}

impl From<&F> for Spec {
    fn from(f: &F) -> Self {
        let w = match f.w {
            W::Num {
                speed,
                min,
                max,
                suffix,
            } => Wd::Num {
                speed,
                min,
                max,
                suffix,
            },
            W::Int { min, max } => Wd::Int { min, max },
            W::Bool => Wd::Bool,
            W::Text => Wd::Text,
            W::Multiline => Wd::Multiline,
            W::Color => Wd::Color,
            W::Choice(list) => Wd::Choice(
                list.iter()
                    .map(|(k, l)| (k.to_string(), l.to_string()))
                    .collect(),
            ),
            W::File(k) => Wd::File(k),
            W::Object => Wd::Object("(nenhum)"),
        };
        Spec {
            key: f.key.to_string(),
            label: f.label.to_string(),
            w,
            help: f.help.to_string(),
        }
    }
}

/// O campo de uma propriedade de comportamento.
fn behavior_spec(info: &BehaviorInfo, p: &behaviors::PropSpec) -> Spec {
    let w = match p.kind {
        PropType::Number if p.integer => Wd::Int {
            min: p.min.unwrap_or(-1_000_000.0),
            max: p.max.unwrap_or(1_000_000.0),
        },
        PropType::Number => {
            let range = p.max.unwrap_or(1000.0) - p.min.unwrap_or(0.0);
            let base = p.default.num().abs();
            Wd::Num {
                speed: if base < 5.0 && range <= 20.0 {
                    0.01
                } else {
                    1.0
                },
                min: p.min.unwrap_or(-1_000_000.0),
                max: p.max.unwrap_or(1_000_000.0),
                suffix: "",
            }
        }
        PropType::Bool => Wd::Bool,
        PropType::Text => Wd::Text,
        PropType::Color => Wd::Color,
        PropType::Choice => Wd::Choice(p.options.clone()),
        PropType::Object => Wd::Object(if info.is_builtin() {
            "(o jogador)"
        } else {
            "(nenhum)"
        }),
        PropType::Scene => Wd::Scene,
        PropType::Prefab => Wd::File(FileKind::Prefab),
        PropType::Sound => Wd::File(FileKind::Sound),
        PropType::Image => Wd::File(FileKind::Image),
        PropType::Target | PropType::Var | PropType::Value | PropType::Behavior => Wd::Text,
    };
    Spec {
        key: p.key.clone(),
        label: p.label.clone(),
        w,
        help: p.help.clone(),
    }
}

impl std::fmt::Debug for Inspector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inspector").finish_non_exhaustive()
    }
}

/// Cor "#RRGGBB(AA)" ↔ bytes.
fn hex_to_rgba(s: &str) -> [u8; 4] {
    let c = crate::scene_render::egui_color(s);
    c.to_srgba_unmultiplied()
}

fn rgba_to_hex(c: [u8; 4]) -> String {
    if c[3] == 255 {
        format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
    } else {
        format!("#{:02X}{:02X}{:02X}{:02X}", c[0], c[1], c[2], c[3])
    }
}

impl Inspector {
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        doc: Option<&mut SceneDoc>,
        files: &FileTree,
        sizes: &mut crate::scene_render::Sizes,
        mode: Mode,
        game: Option<&GameState>,
    ) -> Vec<InspectorAction> {
        let mut actions = Vec::new();
        self.game = game.cloned();
        let Some(doc) = doc else {
            ui.add_space(8.0);
            ui.weak("Selecione um objeto numa cena visual para ver as propriedades dele aqui.");
            return actions;
        };
        if doc.error.is_some() {
            return actions;
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match doc.selection.len() {
                0 => self.scene_ui(ui, doc, files, mode, &mut actions),
                1 => {
                    let id = doc.selection[0];
                    if doc.data.find(id).is_some() {
                        self.object_ui(ui, doc, id, files, sizes, mode);
                    }
                }
                n => {
                    ui.label(format!("{n} objetos selecionados."));
                    ui.horizontal(|ui| {
                        if ui.button("Duplicar").clicked() {
                            doc.duplicate_selection();
                        }
                        if ui.button("🗑 Excluir").clicked() {
                            doc.delete_selection();
                        }
                    });
                }
            });
        actions.append(&mut self.actions_out);
        actions
    }

    fn scene_ui(
        &mut self,
        ui: &mut egui::Ui,
        doc: &mut SceneDoc,
        files: &FileTree,
        mode: Mode,
        actions: &mut Vec<InspectorAction>,
    ) {
        ui.label(RichText::new("Cena").size(17.0).strong());
        ui.weak(format!("{} · {} objetos", doc.rel, doc.data.ids().len()));
        ui.add_space(6.0);
        egui::Grid::new(("grade-cena", &doc.rel))
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.label("Script:")
                    .on_hover_text("Código Lua da cena: recebe os objetos em self.objects.");
                ui.horizontal(|ui| {
                    let mut script = doc.data.script.clone();
                    egui::ComboBox::from_id_salt(("script", &doc.rel))
                        .width((ui.available_width() - 64.0).clamp(70.0, 180.0))
                        .truncate()
                        .selected_text(if script.is_empty() {
                            "(nenhum)".to_string()
                        } else {
                            script.clone()
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut script, String::new(), "(nenhum)");
                            for e in files
                                .files_of(FileKind::Lua)
                                .filter(|e| e.rel != "config.lua")
                            {
                                ui.selectable_value(&mut script, e.rel.clone(), &e.rel);
                            }
                        });
                    if script != doc.data.script {
                        doc.record(None);
                        doc.data.script = script;
                    }
                    if doc.data.script.is_empty() {
                        if ui
                            .button("Criar")
                            .on_hover_text("Cria um script para esta cena")
                            .clicked()
                        {
                            actions.push(InspectorAction::CreateScript);
                        }
                    } else if ui.button("Abrir").clicked() {
                        actions.push(InspectorAction::OpenFile(doc.data.script.clone()));
                    }
                });
                ui.end_row();

                ui.label("Fundo:");
                ui.horizontal(|ui| {
                    let mut own = !doc.data.background.is_empty();
                    if ui
                        .checkbox(&mut own, "Cor própria")
                        .on_hover_text("Sem cor própria, vale a do projeto (config.lua).")
                        .changed()
                    {
                        doc.record(None);
                        doc.data.background = if own { "#14202E".into() } else { String::new() };
                    }
                    if own {
                        let mut c = hex_to_rgba(&doc.data.background);
                        if ui.color_edit_button_srgba_unmultiplied(&mut c).changed() {
                            doc.record(Some("fundo"));
                            doc.data.background = rgba_to_hex([c[0], c[1], c[2], 255]);
                        }
                    }
                });
                ui.end_row();

                ui.label("Gravidade:")
                    .on_hover_text("Para corpos físicos. Negativa = cai.");
                let mut g = doc.data.gravity;
                if ui.add(egui::DragValue::new(&mut g).speed(10.0)).changed() {
                    doc.record(Some("gravidade"));
                    doc.data.gravity = g;
                }
                ui.end_row();

                if mode.is_full() {
                    ui.label("Por cima:");
                    let mut o = doc.data.overlay;
                    if ui
                        .checkbox(&mut o, "Desenha por cima da cena de baixo (pausa, diálogo)")
                        .changed()
                    {
                        doc.record(None);
                        doc.data.overlay = o;
                    }
                    ui.end_row();
                }
            });
        if !doc.is_prefab() && events_summary(ui, &doc.data.events, false) {
            self.actions_out.push(InspectorAction::ShowEvents);
        }
        if let Some(game) = &self.game {
            ui.add_space(10.0);
            ui.label(RichText::new("No jogo agora").strong().color(GAME));
            if game.vars.is_empty() {
                ui.weak("Nenhuma variável do jogo.");
            }
            egui::Grid::new("vars-no-jogo")
                .num_columns(2)
                .show(ui, |ui| {
                    for (k, v) in &game.vars {
                        ui.label(k);
                        ui.label(RichText::new(prop_text(v)).color(GAME));
                        ui.end_row();
                    }
                });
            if game.paused {
                ui.weak("O jogo está pausado.");
            }
        }
        ui.add_space(10.0);
        ui.weak("Selecione um objeto na cena ou na Hierarquia para ver as propriedades dele.");
    }

    fn object_ui(
        &mut self,
        ui: &mut egui::Ui,
        doc: &mut SceneDoc,
        id: u32,
        files: &FileTree,
        sizes: &mut crate::scene_render::Sizes,
        mode: Mode,
    ) {
        let full = mode.is_full();
        let o = doc.data.find(id).cloned().expect("objeto selecionado");
        // Nome, visível, travado.
        ui.horizontal(|ui| {
            let mut name = o.name.clone();
            let r = ui.add(
                egui::TextEdit::singleline(&mut name).desired_width(ui.available_width() - 4.0),
            );
            if r.changed() {
                doc.record(Some(&format!("{id}:name")));
                if let Some(obj) = doc.data.find_mut(id) {
                    obj.name = name;
                }
            }
        });
        ui.horizontal(|ui| {
            let mut v = o.visible;
            if ui.checkbox(&mut v, "Visível").changed() {
                doc.record(None);
                if let Some(obj) = doc.data.find_mut(id) {
                    obj.visible = v;
                }
            }
            let mut l = o.locked;
            if ui
                .checkbox(&mut l, "Travado")
                .on_hover_text("Não seleciona clicando na cena.")
                .changed()
            {
                doc.record(None);
                if let Some(obj) = doc.data.find_mut(id) {
                    obj.locked = l;
                }
            }
            if full {
                ui.weak(format!("id {id}"));
            }
        });
        ui.add_space(4.0);

        egui::CollapsingHeader::new(RichText::new("Transformação").strong())
            .id_salt(("transformacao", id))
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new(("grade-transformacao", id))
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        let mut t = (o.x, o.y, o.rotation, o.scale_x, o.scale_y, o.layer);
                        ui.label("Posição");
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut t.0).speed(1.0).prefix("x "));
                            ui.add(egui::DragValue::new(&mut t.1).speed(1.0).prefix("y "));
                        });
                        ui.end_row();
                        ui.label("Rotação");
                        ui.add(egui::DragValue::new(&mut t.2).speed(1.0).suffix("°"));
                        ui.end_row();
                        ui.label("Escala");
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut t.3).speed(0.01).prefix("x "));
                            ui.add(egui::DragValue::new(&mut t.4).speed(0.01).prefix("y "));
                        });
                        ui.end_row();
                        ui.label("Camada")
                            .on_hover_text("Objetos de camada maior são desenhados por cima.");
                        ui.add(egui::DragValue::new(&mut t.5).speed(0.1));
                        ui.end_row();
                        // Onde está no jogo aberto, se mudou.
                        if let Some(g) = self.game.as_ref().and_then(|g| g.objects.get(&id)) {
                            let l = g.local;
                            let differs = (l.x - o.x).abs() > 0.01
                                || (l.y - o.y).abs() > 0.01
                                || (l.rotation - o.rotation).abs() > 0.01
                                || (l.scale_x - o.scale_x).abs() > 0.001
                                || (l.scale_y - o.scale_y).abs() > 0.001;
                            if differs {
                                ui.label(RichText::new("No jogo").color(GAME));
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "x {}  y {}  {}°",
                                            crate::scene_data::format_num(round2(l.x)),
                                            crate::scene_data::format_num(round2(l.y)),
                                            crate::scene_data::format_num(round2(l.rotation)),
                                        ))
                                        .color(GAME),
                                    );
                                    if ui
                                        .small_button("Manter")
                                        .on_hover_text("Traz a posição, a rotação e a escala do jogo para a cena")
                                        .clicked()
                                    {
                                        t = (
                                            round2(l.x),
                                            round2(l.y),
                                            round2(l.rotation),
                                            l.scale_x,
                                            l.scale_y,
                                            t.5,
                                        );
                                    }
                                });
                                ui.end_row();
                            }
                        }
                        if t != (o.x, o.y, o.rotation, o.scale_x, o.scale_y, o.layer) {
                            doc.record(Some(&format!("{id}:transformacao")));
                            if let Some(obj) = doc.data.find_mut(id) {
                                (
                                    obj.x,
                                    obj.y,
                                    obj.rotation,
                                    obj.scale_x,
                                    obj.scale_y,
                                    obj.layer,
                                ) = t;
                            }
                        }
                    });
            });

        if o.component("body").is_some() {
            if doc.data.parent_of(id).flatten().is_some() {
                ui.colored_label(
                    crate::theme::WARNING,
                    "⚠ Objetos com corpo físico funcionam melhor na raiz (sem pai).",
                );
            }
            if o.rotation != 0.0 {
                ui.colored_label(
                    crate::theme::WARNING,
                    "⚠ O corpo físico não gira (só a imagem).",
                );
            }
        }

        // Instância de prefab: os componentes vêm dele.
        let instance = o.prefab.clone();
        if let Some(link) = &instance {
            let mut unlink = false;
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Prefab").strong().color(PREFAB));
                    ui.label(&link.path);
                });
                if link.base.is_none() {
                    ui.colored_label(
                        crate::theme::ERROR,
                        "O arquivo do prefab não foi encontrado (ou tem erro).",
                    );
                } else {
                    ui.weak("Mudanças aqui valem só para este objeto (aparecem em negrito, com ↺ para voltar ao do prefab).");
                }
                ui.horizontal(|ui| {
                    if ui.button("Abrir prefab").clicked() {
                        self.actions_out.push(InspectorAction::OpenFile(link.path.clone()));
                    }
                    if ui
                        .button("Desvincular")
                        .on_hover_text("Vira um objeto comum: os filhos do prefab passam a ser dele e mudanças no prefab não chegam mais.")
                        .clicked()
                    {
                        unlink = true;
                    }
                });
            });
            if unlink {
                doc.record(None);
                let mut next = doc.data.next_id();
                if let Some(obj) = doc.data.find_mut(id) {
                    let mut kids: Vec<Object> = obj.prefab_children().to_vec();
                    fn renumber(o: &mut Object, next: &mut u32) {
                        o.id = *next;
                        *next += 1;
                        for c in &mut o.children {
                            renumber(c, next);
                        }
                    }
                    for k in &mut kids {
                        renumber(k, &mut next);
                    }
                    kids.append(&mut obj.children);
                    obj.children = kids;
                    obj.prefab = None;
                }
                return;
            }
        }

        let names: Vec<String> = {
            let mut v = Vec::new();
            crate::scene_data::visit(&doc.data.objects, &mut |x| v.push(x.name.clone()));
            v
        };
        let root = files.root().to_path_buf();
        let infos: Vec<Option<BehaviorInfo>> = o
            .components
            .iter()
            .map(|c| {
                (c.is_behavior() && !c.str("script").is_empty())
                    .then(|| self.catalog.get(&root, c.str("script")))
            })
            .collect();
        let mut remove = None;
        let mut swap = None;
        let mut add: Vec<Component> = Vec::new();
        let count = o.components.len();
        for (ci, comp) in o.components.iter().enumerate() {
            let (mut title, mut desc) = {
                let (t, d) = component_info(&comp.kind);
                (t.to_string(), d.to_string())
            };
            if let Some(info) = &infos[ci] {
                title = info.title.clone();
                desc = info.description.clone();
            }
            let state = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                ui.id().with(("componente", id, ci, &comp.kind)),
                true,
            );
            state
                .show_header(ui, |ui| {
                    let r = ui.label(RichText::new(&title).strong());
                    if !desc.is_empty() {
                        r.on_hover_text(&desc);
                    }
                    if instance.is_some() {
                        return;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.menu_button("☰", |ui| {
                            if ci > 0 && ui.button("Subir").clicked() {
                                swap = Some((ci, ci - 1));
                                ui.close();
                            }
                            if ci + 1 < count && ui.button("Descer").clicked() {
                                swap = Some((ci, ci + 1));
                                ui.close();
                            }
                            if ui.button("🗑 Remover componente").clicked() {
                                remove = Some(ci);
                                ui.close();
                            }
                        });
                    });
                })
                .body(|ui| {
                    if comp.is_behavior() {
                        self.behavior_ui(
                            ui,
                            doc,
                            &o,
                            ci,
                            comp,
                            infos[ci].as_ref(),
                            files,
                            sizes,
                            &names,
                            full,
                            &mut add,
                        );
                    } else {
                        self.component_ui(ui, doc, id, ci, comp, files, sizes, &names, full);
                    }
                });
        }
        if let Some(ci) = remove {
            doc.record(None);
            if let Some(obj) = doc.data.find_mut(id) {
                obj.components.remove(ci);
            }
        }
        if let Some((a, b)) = swap {
            doc.record(None);
            if let Some(obj) = doc.data.find_mut(id) {
                obj.components.swap(a, b);
            }
        }
        if !add.is_empty() {
            doc.record(None);
            if let Some(obj) = doc.data.find_mut(id) {
                obj.components.extend(add);
            }
        }
        if let Some(obj) = doc.data.find(id)
            && events_summary(ui, &obj.events, true)
        {
            self.actions_out.push(InspectorAction::ShowEvents);
        }
        ui.add_space(8.0);
        if instance.is_some() {
            ui.weak("Para pôr ou tirar componentes, abra o prefab: a mudança vale para todas as cópias.");
            return;
        }
        let mut chosen: Vec<Component> = Vec::new();
        ui.menu_button("➕ Adicionar componente", |ui| {
            for kind in &crate::scene_data::schema().order {
                let (title, desc) = component_info(kind);
                let exists = o.component(kind).is_some();
                let unique = matches!(kind.as_str(), "body" | "camera" | "map");
                if ui
                    .add_enabled(!(exists && unique), egui::Button::new(title))
                    .on_hover_text(desc)
                    .clicked()
                {
                    let mut c = Component::new(kind);
                    if kind == "body" {
                        // O corpo já vem do tamanho do que aparece.
                        if let Some((w, h)) = visible_size(&o, sizes, files) {
                            c.set("w", Prop::Num(w.round()));
                            c.set("h", Prop::Num(h.round()));
                        }
                    }
                    chosen.push(c);
                    ui.close();
                }
            }
            ui.separator();
            ui.label(RichText::new("Comportamentos").weak());
            for cat in behaviors::categories() {
                ui.menu_button(cat, |ui| {
                    for b in behaviors::builtins().iter().filter(|b| b.category == cat) {
                        if ui.button(&b.title).on_hover_text(&b.description).clicked() {
                            // Quem precisa de corpo físico já ganha um.
                            if b.requires.iter().any(|r| r == "body")
                                && o.component("body").is_none()
                            {
                                let mut body = Component::new("body");
                                if let Some((w, h)) = visible_size(&o, sizes, files) {
                                    body.set("w", Prop::Num(w.round()));
                                    body.set("h", Prop::Num(h.round()));
                                }
                                chosen.push(body);
                            }
                            chosen.push(Component::behavior(&b.script, &b.defaults()));
                            ui.close();
                        }
                    }
                });
            }
            ui.menu_button("Script do jogo", |ui| {
                if ui
                    .button("➕ Novo script...")
                    .on_hover_text("Cria um arquivo em scripts/ com o modelo de comportamento")
                    .clicked()
                {
                    self.actions_out
                        .push(InspectorAction::NewBehaviorScript(id));
                    ui.close();
                }
                let scripts: Vec<String> = files
                    .files_of(FileKind::Lua)
                    .map(|e| e.rel.clone())
                    .filter(|r| is_behavior_candidate(r))
                    .collect();
                if !scripts.is_empty() {
                    ui.separator();
                }
                for rel in scripts {
                    if ui.button(&rel).clicked() {
                        let info = self.catalog.get(&root, &rel);
                        chosen.push(Component::behavior(&rel, &info.defaults()));
                        ui.close();
                    }
                }
            });
        });
        if !chosen.is_empty() {
            doc.record(None);
            if let Some(obj) = doc.data.find_mut(id) {
                obj.components.extend(chosen);
            }
        }
    }

    /// Um comportamento: escolher o script e as propriedades dele.
    #[allow(clippy::too_many_arguments)]
    fn behavior_ui(
        &mut self,
        ui: &mut egui::Ui,
        doc: &mut SceneDoc,
        o: &Object,
        ci: usize,
        comp: &Component,
        info: Option<&BehaviorInfo>,
        files: &FileTree,
        sizes: &mut crate::scene_render::Sizes,
        names: &[String],
        full: bool,
        add: &mut Vec<Component>,
    ) {
        let id = o.id;
        let _ = sizes;
        let mut changes: Vec<(String, Prop)> = Vec::new();
        let mut reverts: Vec<String> = Vec::new();
        let script = comp.str("script").to_string();
        let Some(info) = info else {
            ui.weak("Escolha um comportamento em ➕ Adicionar componente.");
            return;
        };
        if let Some(e) = &info.error {
            ui.colored_label(crate::theme::ERROR, e);
        }
        if !info.is_builtin() {
            ui.horizontal(|ui| {
                ui.weak(&script);
                if ui.small_button("Abrir script").clicked() {
                    self.actions_out
                        .push(InspectorAction::OpenFile(script.clone()));
                }
            });
        }
        // Do que ele precisa.
        for req in &info.requires {
            if o.component(req).is_none() {
                let (title, _) = component_info(req);
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(crate::theme::WARNING, format!("⚠ Precisa de: {title}"));
                    if o.prefab.is_none() && ui.small_button("Adicionar").clicked() {
                        add.push(Component::new(req));
                    }
                });
            }
        }
        let value = |key: &str| -> Prop {
            comp.get(key)
                .cloned()
                .or_else(|| info.prop(key).map(|p| p.default.clone()))
                .unwrap_or(Prop::Str(String::new()))
        };
        egui::Grid::new(("grade-comportamento", id, ci))
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                for p in &info.props {
                    if p.advanced && !full {
                        continue;
                    }
                    if let Some((other, values)) = &p.show_if {
                        let v = value(other);
                        let v = match &v {
                            Prop::Str(s) => s.clone(),
                            other => other.to_lua(),
                        };
                        if !values.contains(&v) {
                            continue;
                        }
                    }
                    let spec = behavior_spec(info, p);
                    let overridden = o.is_overridden(ci, &p.key);
                    if field_row(ui, &spec, overridden, &mut reverts) {
                        // (a linha mostrou o rótulo)
                    }
                    let current = value(&p.key);
                    ui.horizontal(|ui| {
                        if let Some(new) = field_widget(ui, &spec, &current, files, names, (id, ci))
                        {
                            changes.push((p.key.clone(), new));
                        }
                        // Valor no jogo aberto, se mudou (o jogo pode mexer nele).
                        if let Some(g) = self.game.as_ref().and_then(|g| g.prop(id, ci, &p.key))
                            && !same_value(g, &current)
                        {
                            ui.label(RichText::new(format!("jogo: {}", prop_text(g))).color(GAME));
                            if ui
                                .small_button("Manter")
                                .on_hover_text("Usa o valor do jogo na cena")
                                .clicked()
                            {
                                changes.push((p.key.clone(), g.clone()));
                            }
                        }
                    });
                    ui.end_row();
                }
            });
        if info.props.is_empty() && info.error.is_none() {
            ui.weak("Este comportamento não tem propriedades.");
        }
        // Valores guardados de propriedades que o script não tem mais.
        let stale: Vec<String> = comp
            .props
            .iter()
            .map(|(k, _)| k.clone())
            .filter(|k| k != "script" && info.prop(k).is_none() && info.error.is_none())
            .collect();
        if !stale.is_empty() && o.prefab.is_none() {
            ui.horizontal_wrapped(|ui| {
                ui.weak(format!(
                    "Valores que o script não usa mais: {}",
                    stale.join(", ")
                ));
                if ui.small_button("Limpar").clicked() {
                    doc.record(None);
                    if let Some(c) = doc.data.find_mut(id).and_then(|x| x.components.get_mut(ci)) {
                        for k in &stale {
                            c.remove(k);
                        }
                    }
                }
            });
        }
        apply_changes(doc, o, ci, changes, reverts);
    }

    #[allow(clippy::too_many_arguments)]
    fn component_ui(
        &mut self,
        ui: &mut egui::Ui,
        doc: &mut SceneDoc,
        id: u32,
        ci: usize,
        comp: &Component,
        files: &FileTree,
        sizes: &mut crate::scene_render::Sizes,
        names: &[String],
        full: bool,
    ) {
        let mut changes: Vec<(String, Prop)> = Vec::new();
        let mut reverts: Vec<String> = Vec::new();
        let object = doc.data.find(id).cloned();
        let obj = object.as_ref();
        let is_instance = obj.is_some_and(|x| x.prefab.is_some());
        let circle = comp.str("shape") == "circle";
        egui::Grid::new(("grade-componente", id, ci))
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                for spec in fields(&comp.kind) {
                    if spec.advanced && !full {
                        continue;
                    }
                    // Raio só para círculos; largura/altura só para os outros.
                    if matches!(comp.kind.as_str(), "shape" | "body") {
                        if spec.key == "radius" && !circle {
                            continue;
                        }
                        if (spec.key == "w" || spec.key == "h") && circle {
                            continue;
                        }
                    }
                    if comp.kind == "sprite" && spec.key == "frame" && comp.num("frame_w") <= 0.0 {
                        continue;
                    }
                    let spec = Spec::from(spec);
                    let overridden =
                        is_instance && obj.is_some_and(|x| x.is_overridden(ci, &spec.key));
                    field_row(ui, &spec, overridden, &mut reverts);
                    let current = comp.get(&spec.key).cloned().unwrap_or(Prop::Num(0.0));
                    if let Some(new) = field_widget(ui, &spec, &current, files, names, (id, ci)) {
                        changes.push((spec.key.clone(), new));
                    }
                    ui.end_row();
                }
            });

        // Ajudas da imagem: tamanho original, ×2, quadros.
        if comp.kind == "sprite" && !comp.str("image").is_empty() {
            let natural = sizes.image(files.root(), comp.str("image"));
            match natural {
                None => {
                    ui.colored_label(crate::theme::ERROR, "Imagem não encontrada (ou não é PNG).");
                }
                Some((iw, ih)) => {
                    let (fw, fh) = if comp.num("frame_w") > 0.0 && comp.num("frame_h") > 0.0 {
                        (comp.num("frame_w"), comp.num("frame_h"))
                    } else {
                        (iw as f64, ih as f64)
                    };
                    ui.horizontal_wrapped(|ui| {
                        ui.weak(format!("{iw}×{ih} px"));
                        if ui
                            .small_button("Original")
                            .on_hover_text("Tamanho da imagem (ou do quadro)")
                            .clicked()
                        {
                            changes.push(("w".into(), Prop::Num(0.0)));
                            changes.push(("h".into(), Prop::Num(0.0)));
                        }
                        for k in [2.0, 3.0, 4.0] {
                            if ui.small_button(format!("×{k}")).clicked() {
                                changes.push(("w".into(), Prop::Num(fw * k)));
                                changes.push(("h".into(), Prop::Num(fh * k)));
                            }
                        }
                        if comp.num("frame_w") <= 0.0
                            && iw > ih
                            && ih > 0.0
                            && (iw % ih) == 0.0
                            && ui
                                .small_button("É uma tira de quadros")
                                .on_hover_text(format!("Divide em quadros de {ih}×{ih}"))
                                .clicked()
                        {
                            changes.push(("frame_w".into(), Prop::Num(ih as f64)));
                            changes.push(("frame_h".into(), Prop::Num(ih as f64)));
                            changes.push(("frame".into(), Prop::Num(1.0)));
                        }
                    });
                    if comp.num("frame_w") > 0.0
                        && comp.num("frame_h") > 0.0
                        && let Some(frame) = self.frame_picker(ui, files, comp, (iw, ih))
                    {
                        changes.push(("frame".into(), Prop::Num(frame as f64)));
                    }
                }
            }
        }
        if comp.kind == "body"
            && let Some(o) = doc.data.find(id)
            && let Some((w, h)) = visible_size(o, sizes, files)
            && ui.small_button("Ajustar ao que aparece").clicked()
        {
            changes.push(("w".into(), Prop::Num(w.round())));
            changes.push(("h".into(), Prop::Num(h.round())));
            changes.push(("radius".into(), Prop::Num((w.max(h) / 2.0).round())));
        }

        if let Some(o) = obj {
            apply_changes(doc, o, ci, changes, reverts);
        }
    }

    /// Grade da spritesheet para escolher o quadro com um clique.
    fn frame_picker(
        &mut self,
        ui: &mut egui::Ui,
        files: &FileTree,
        comp: &Component,
        size: (f32, f32),
    ) -> Option<u32> {
        let rel = comp.str("image").to_string();
        let modified = files.get(&rel).and_then(|e| e.modified);
        let stale = self.images.get(&rel).is_none_or(|(m, _)| *m != modified);
        if stale {
            let img = crate::viewers::load_rgba(&files.full(&rel)).ok()?;
            let tex = ui.ctx().load_texture(
                format!("inspetor:{rel}"),
                crate::viewers::to_color_image(&img),
                egui::TextureOptions::NEAREST,
            );
            self.images.insert(rel.clone(), (modified, tex));
        }
        let tex = self.images.get(&rel)?.1.clone();
        let (fw, fh) = (comp.num("frame_w") as f32, comp.num("frame_h") as f32);
        let cols = (size.0 / fw).floor().max(1.0) as u32;
        let rows = (size.1 / fh).floor().max(1.0) as u32;
        let width = ui.available_width().min(260.0);
        let scale = (width / size.0).min(160.0 / size.1).max(0.25);
        let (rect, resp) = ui.allocate_exact_size(
            Vec2::new(size.0 * scale, size.1 * scale),
            egui::Sense::click(),
        );
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 0.0, Color32::from_gray(0x40));
        p.image(
            tex.id(),
            rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        let grid = egui::Stroke::new(1.0, Color32::from_white_alpha(70));
        for c in 0..=cols {
            let x = rect.left() + c as f32 * fw * scale;
            p.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                grid,
            );
        }
        for r in 0..=rows {
            let y = rect.top() + r as f32 * fh * scale;
            p.line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                grid,
            );
        }
        let current = comp.num("frame").max(1.0) as u32 - 1;
        let cell = |n: u32| {
            egui::Rect::from_min_size(
                rect.min
                    + Vec2::new(
                        (n % cols) as f32 * fw * scale,
                        (n / cols) as f32 * fh * scale,
                    ),
                Vec2::new(fw * scale, fh * scale),
            )
        };
        p.rect_stroke(
            cell(current),
            0.0,
            egui::Stroke::new(2.0, Color32::from_rgb(0xFF, 0xA9, 0x2E)),
            egui::StrokeKind::Inside,
        );
        if let Some(pos) = resp.hover_pos() {
            let col = ((pos.x - rect.left()) / (fw * scale)) as u32;
            let row = ((pos.y - rect.top()) / (fh * scale)) as u32;
            if col < cols && row < rows {
                let n = row * cols + col;
                p.rect_stroke(
                    cell(n),
                    0.0,
                    egui::Stroke::new(1.0, Color32::WHITE),
                    egui::StrokeKind::Inside,
                );
                if resp.clicked() {
                    return Some(n + 1);
                }
                resp.on_hover_text(format!("quadro {}", n + 1));
            }
        }
        None
    }
}

/// Tamanho do que aparece (imagem ou forma), já com a escala.
fn visible_size(
    o: &crate::scene_data::Object,
    sizes: &mut crate::scene_render::Sizes,
    files: &FileTree,
) -> Option<(f64, f64)> {
    let mut only_visual = o.clone();
    only_visual
        .components
        .retain(|c| matches!(c.kind.as_str(), "sprite" | "shape"));
    if only_visual.components.is_empty() {
        return None;
    }
    let (r, _) = crate::scene_render::local_bounds(&only_visual, files.root(), sizes, 32.0);
    Some((
        (r.max.0 - r.min.0) * o.scale_x.abs(),
        (r.max.1 - r.min.1) * o.scale_y.abs(),
    ))
}

/// Mostra o controle de um campo. Devolve o valor novo, se mudou.
fn field_widget(
    ui: &mut egui::Ui,
    spec: &Spec,
    current: &Prop,
    files: &FileTree,
    names: &[String],
    salt: (u32, usize),
) -> Option<Prop> {
    let width = ui.available_width().clamp(80.0, 180.0);
    match &spec.w {
        Wd::Num {
            speed,
            min,
            max,
            suffix,
        } => {
            let mut v = current.num();
            ui.add(
                egui::DragValue::new(&mut v)
                    .speed(*speed)
                    .range(*min..=*max)
                    .suffix(*suffix),
            )
            .changed()
            .then_some(Prop::Num(v))
        }
        Wd::Int { min, max } => {
            let mut v = current.num().round();
            ui.add(
                egui::DragValue::new(&mut v)
                    .speed(0.1)
                    .range(*min..=*max)
                    .max_decimals(0),
            )
            .changed()
            .then_some(Prop::Num(v.round()))
        }
        Wd::Bool => {
            let mut v = current.bool();
            ui.checkbox(&mut v, "").changed().then_some(Prop::Bool(v))
        }
        Wd::Text => {
            let mut v = current.str().to_string();
            ui.add(egui::TextEdit::singleline(&mut v).desired_width(width))
                .changed()
                .then_some(Prop::Str(v))
        }
        Wd::Multiline => {
            let mut v = current.str().to_string();
            ui.add(
                egui::TextEdit::multiline(&mut v)
                    .desired_rows(2)
                    .desired_width(width),
            )
            .changed()
            .then_some(Prop::Str(v))
        }
        Wd::Color => {
            let mut c = hex_to_rgba(current.str());
            ui.color_edit_button_srgba_unmultiplied(&mut c)
                .changed()
                .then(|| Prop::Str(rgba_to_hex(c)))
        }
        Wd::Choice(options) => {
            let mut v = current.str().to_string();
            let text = options
                .iter()
                .find(|(k, _)| *k == v)
                .map(|(_, l)| l.clone())
                .unwrap_or_else(|| v.clone());
            let mut changed = false;
            egui::ComboBox::from_id_salt(("escolha", salt, &spec.key))
                .width(width)
                .truncate()
                .selected_text(text)
                .show_ui(ui, |ui| {
                    for (k, l) in options {
                        if ui.selectable_value(&mut v, k.clone(), l).changed() {
                            changed = true;
                        }
                    }
                });
            changed.then_some(Prop::Str(v))
        }
        Wd::Object(empty) => {
            let mut v = current.str().to_string();
            let mut changed = false;
            egui::ComboBox::from_id_salt(("objeto", salt, &spec.key))
                .width(width)
                .truncate()
                .selected_text(if v.is_empty() {
                    empty.to_string()
                } else {
                    v.clone()
                })
                .show_ui(ui, |ui| {
                    changed |= ui.selectable_value(&mut v, String::new(), *empty).changed();
                    let mut seen = std::collections::HashSet::new();
                    for n in names {
                        if seen.insert(n) {
                            changed |= ui.selectable_value(&mut v, n.clone(), n).changed();
                        }
                    }
                });
            changed.then_some(Prop::Str(v))
        }
        Wd::Scene => {
            let list: Vec<String> = files
                .files_of(FileKind::Scene)
                .chain(files.files_of(FileKind::Lua))
                .map(|e| e.rel.clone())
                .filter(|r| r != "config.lua")
                .collect();
            file_combo(ui, spec, current, &list, salt, |p| {
                matches!(
                    FileKind::of(std::path::Path::new(p)),
                    FileKind::Scene | FileKind::Lua
                )
            })
        }
        Wd::File(kind) => {
            let kind = *kind;
            let list: Vec<String> = files
                .files_of(kind)
                .map(|e| e.rel.clone())
                .filter(|r| kind != FileKind::Map || r.to_lowercase().ends_with(".tmj"))
                .collect();
            file_combo(ui, spec, current, &list, salt, |p| {
                FileKind::of(std::path::Path::new(p)) == kind
            })
        }
    }
}

/// Lista de arquivos, aceitando arrastar do painel Arquivos.
fn file_combo(
    ui: &mut egui::Ui,
    spec: &Spec,
    current: &Prop,
    list: &[String],
    salt: (u32, usize),
    accepts: impl Fn(&str) -> bool,
) -> Option<Prop> {
    let mut v = current.str().to_string();
    let mut changed = false;
    // Na caixa, só o nome do arquivo (o caminho todo fica na dica).
    let short = v.rsplit('/').next().unwrap_or(&v).to_string();
    let r = egui::ComboBox::from_id_salt(("arquivo", salt, &spec.key))
        .selected_text(if v.is_empty() {
            "(nenhum)".to_string()
        } else {
            short
        })
        .width(ui.available_width().clamp(80.0, 180.0))
        .truncate()
        .show_ui(ui, |ui| {
            changed |= ui
                .selectable_value(&mut v, String::new(), "(nenhum)")
                .changed();
            for rel in list {
                changed |= ui.selectable_value(&mut v, rel.clone(), rel).changed();
            }
        })
        .response
        .on_hover_text(if v.is_empty() {
            "Escolha na lista ou arraste do painel Arquivos para cá.".to_string()
        } else {
            format!("{v}\nEscolha na lista ou arraste do painel Arquivos para cá.")
        });
    if let Some(p) = r.dnd_release_payload::<FileDrag>()
        && accepts(&p.0)
    {
        v = p.0.clone();
        changed = true;
    }
    changed.then_some(Prop::Str(v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todos_os_campos_existem_na_engine() {
        // Cada campo do inspetor precisa existir no esquema da engine (e
        // todo campo da engine aparece no inspetor).
        let schema = crate::scene_data::schema();
        for kind in &schema.order {
            let engine: Vec<&str> = schema
                .fields(kind)
                .unwrap()
                .iter()
                .map(|(k, _)| k.as_str())
                .collect();
            let ours: Vec<&str> = fields(kind).iter().map(|f| f.key).collect();
            for k in &ours {
                assert!(engine.contains(k), "{kind}.{k} não existe na engine");
            }
            for k in &engine {
                assert!(ours.contains(k), "{kind}.{k} não aparece no inspetor");
            }
            assert_ne!(component_info(kind).0, "Componente");
        }
    }

    #[test]
    fn cores_hex() {
        assert_eq!(rgba_to_hex(hex_to_rgba("#12AB34")), "#12AB34");
        assert_eq!(rgba_to_hex(hex_to_rgba("#12AB3480")), "#12AB3480");
    }
}
