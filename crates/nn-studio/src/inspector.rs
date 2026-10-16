//! Painel Inspetor: as propriedades do objeto selecionado (ou da cena, sem
//! seleção). Cada componente tem os campos com o controle certo, e dá para
//! arrastar imagens, sons e mapas do painel Arquivos direto para os campos.

use std::collections::HashMap;
use std::time::SystemTime;

use eframe::egui::{self, Color32, RichText, Vec2};

use crate::files::{FileKind, FileTree};
use crate::scene_data::{Component, Prop};
use crate::scene_view::{FileDrag, SceneDoc};
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
];

const TEXT_FIELDS: &[F] = &[
    f("text", "Texto", W::Multiline),
    f("size", "Tamanho", SIZE),
    f("color", "Cor", W::Color),
    help(
        f("anchor", "Âncora", W::Choice(ANCHORS)),
        "Qual ponto do texto fica na posição do objeto.",
    ),
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
        _ => ("Componente", ""),
    }
}

/// O que o inspetor pede ao espaço de trabalho.
#[derive(Debug, Clone, PartialEq)]
pub enum InspectorAction {
    OpenFile(String),
    /// Criar o script da cena.
    CreateScript,
}

#[derive(Default)]
pub struct Inspector {
    images: HashMap<String, (Option<SystemTime>, egui::TextureHandle)>,
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
    ) -> Vec<InspectorAction> {
        let mut actions = Vec::new();
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

        let names: Vec<String> = {
            let mut v = Vec::new();
            crate::scene_data::visit(&doc.data.objects, &mut |x| v.push(x.name.clone()));
            v
        };
        let mut remove = None;
        let mut swap = None;
        let count = o.components.len();
        for (ci, comp) in o.components.iter().enumerate() {
            let (title, _) = component_info(&comp.kind);
            let state = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                ui.id().with(("componente", id, ci, &comp.kind)),
                true,
            );
            state
                .show_header(ui, |ui| {
                    ui.label(RichText::new(title).strong());
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
                    self.component_ui(ui, doc, id, ci, comp, files, sizes, &names, full);
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
        ui.add_space(8.0);
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
                    doc.record(None);
                    if let Some(obj) = doc.data.find_mut(id) {
                        obj.components.push(c);
                    }
                    ui.close();
                }
            }
        });
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
                    let label = ui.label(spec.label);
                    if !spec.help.is_empty() {
                        label.on_hover_text(spec.help);
                    }
                    let current = comp.get(spec.key).cloned().unwrap_or(Prop::Num(0.0));
                    if let Some(new) = field_widget(ui, spec, &current, files, names, (id, ci)) {
                        changes.push((spec.key.to_string(), new));
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
            && ui.small_button("Ajustar ao tamanho da imagem").clicked()
        {
            changes.push(("w".into(), Prop::Num(w.round())));
            changes.push(("h".into(), Prop::Num(h.round())));
            changes.push(("radius".into(), Prop::Num((w.max(h) / 2.0).round())));
        }

        if !changes.is_empty() {
            let key = format!("{id}:{ci}:{}", changes[0].0);
            doc.record(Some(&key));
            if let Some(c) = doc.data.find_mut(id).and_then(|o| o.components.get_mut(ci)) {
                for (k, v) in changes {
                    c.set(&k, v);
                }
            }
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
    spec: &F,
    current: &Prop,
    files: &FileTree,
    names: &[String],
    salt: (u32, usize),
) -> Option<Prop> {
    match spec.w {
        W::Num {
            speed,
            min,
            max,
            suffix,
        } => {
            let mut v = current.num();
            ui.add(
                egui::DragValue::new(&mut v)
                    .speed(speed)
                    .range(min..=max)
                    .suffix(suffix),
            )
            .changed()
            .then_some(Prop::Num(v))
        }
        W::Int { min, max } => {
            let mut v = current.num().round();
            ui.add(
                egui::DragValue::new(&mut v)
                    .speed(0.1)
                    .range(min..=max)
                    .max_decimals(0),
            )
            .changed()
            .then_some(Prop::Num(v.round()))
        }
        W::Bool => {
            let mut v = current.bool();
            ui.checkbox(&mut v, "").changed().then_some(Prop::Bool(v))
        }
        W::Text => {
            let mut v = current.str().to_string();
            ui.add(
                egui::TextEdit::singleline(&mut v)
                    .desired_width(ui.available_width().clamp(80.0, 180.0)),
            )
            .changed()
            .then_some(Prop::Str(v))
        }
        W::Multiline => {
            let mut v = current.str().to_string();
            ui.add(
                egui::TextEdit::multiline(&mut v)
                    .desired_rows(2)
                    .desired_width(ui.available_width().clamp(80.0, 180.0)),
            )
            .changed()
            .then_some(Prop::Str(v))
        }
        W::Color => {
            let mut c = hex_to_rgba(current.str());
            ui.color_edit_button_srgba_unmultiplied(&mut c)
                .changed()
                .then(|| Prop::Str(rgba_to_hex(c)))
        }
        W::Choice(options) => {
            let mut v = current.str().to_string();
            let text = options
                .iter()
                .find(|(k, _)| *k == v)
                .map(|(_, l)| *l)
                .unwrap_or(v.as_str())
                .to_string();
            let mut changed = false;
            egui::ComboBox::from_id_salt(("escolha", salt, spec.key))
                .width(ui.available_width().clamp(80.0, 180.0))
                .truncate()
                .selected_text(text)
                .show_ui(ui, |ui| {
                    for (k, l) in options {
                        if ui.selectable_value(&mut v, k.to_string(), *l).changed() {
                            changed = true;
                        }
                    }
                });
            changed.then_some(Prop::Str(v))
        }
        W::Object => {
            let mut v = current.str().to_string();
            let mut changed = false;
            egui::ComboBox::from_id_salt(("objeto", salt, spec.key))
                .width(ui.available_width().clamp(80.0, 180.0))
                .truncate()
                .selected_text(if v.is_empty() {
                    "(nenhum)".to_string()
                } else {
                    v.clone()
                })
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(&mut v, String::new(), "(nenhum)")
                        .changed();
                    for n in names {
                        changed |= ui.selectable_value(&mut v, n.clone(), n).changed();
                    }
                });
            changed.then_some(Prop::Str(v))
        }
        W::File(kind) => {
            let mut v = current.str().to_string();
            let mut changed = false;
            // Na caixa, só o nome do arquivo (o caminho todo fica na dica).
            let short = v.rsplit('/').next().unwrap_or(&v).to_string();
            let r = egui::ComboBox::from_id_salt(("arquivo", salt, spec.key))
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
                    for e in files.files_of(kind) {
                        if kind == FileKind::Map && !e.rel.to_lowercase().ends_with(".tmj") {
                            continue;
                        }
                        changed |= ui.selectable_value(&mut v, e.rel.clone(), &e.rel).changed();
                    }
                })
                .response
                .on_hover_text(if v.is_empty() {
                    "Escolha na lista ou arraste do painel Arquivos para cá.".to_string()
                } else {
                    format!("{v}\nEscolha na lista ou arraste do painel Arquivos para cá.")
                });
            // Arrastar um arquivo do painel para o campo.
            if let Some(p) = r.dnd_release_payload::<FileDrag>()
                && crate::files::FileKind::of(std::path::Path::new(&p.0)) == kind
            {
                v = p.0.clone();
                changed = true;
            }
            changed.then_some(Prop::Str(v))
        }
    }
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
