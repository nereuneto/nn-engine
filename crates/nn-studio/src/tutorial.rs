//! Tutoriais guiados: uma janelinha com passos curtos que destaca o painel
//! ou botão de cada passo e avança sozinha quando a pessoa faz o que ele
//! pediu (o tutorial olha o projeto: objetos, componentes, regras...).
//!
//! Os painéis marcam onde estão as coisas com [`anchor`]; o tutorial usa
//! essas marcas para desenhar o destaque.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Rect, RichText, Stroke};
use nn_run::Template;

use crate::project::{ProjectWorkspace, Tab};
use crate::scene_data::{self, Object};
use crate::theme;

// Marcas -------------------------------------------------------------------

fn anchors_id() -> egui::Id {
    egui::Id::new("tutorial-marcas")
}

/// Começo do quadro: esquece as marcas do quadro anterior.
pub fn begin_frame(ctx: &egui::Context) {
    ctx.data_mut(|d| d.insert_temp(anchors_id(), HashMap::<String, Rect>::new()));
}

/// Marca onde está um painel ou botão (`"cena.testar"`, `"arquivo:<caminho>"`...).
pub fn anchor(ctx: &egui::Context, key: &str, rect: Rect) {
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<HashMap<String, Rect>>(anchors_id())
            .insert(key.to_string(), rect);
    });
}

/// Como [`anchor`], só com a parte que aparece (dentro de um painel com
/// rolagem, o que saiu da vista não conta).
pub fn anchor_in(ui: &egui::Ui, key: &str, rect: Rect) {
    let visible = rect.intersect(ui.clip_rect());
    if visible.is_positive() {
        anchor(ui.ctx(), key, visible);
    }
}

/// Como [`anchor`], mas fica a primeira marca com esse nome (o primeiro
/// objeto com um nome, na Hierarquia).
pub fn anchor_first(ui: &egui::Ui, key: &str, rect: Rect) {
    let rect = rect.intersect(ui.clip_rect());
    if !rect.is_positive() {
        return;
    }
    ui.ctx().data_mut(|d| {
        d.get_temp_mut_or_default::<HashMap<String, Rect>>(anchors_id())
            .entry(key.to_string())
            .or_insert(rect);
    });
}

fn anchor_rect(ctx: &egui::Context, key: &str) -> Option<Rect> {
    ctx.data(|d| {
        d.get_temp::<HashMap<String, Rect>>(anchors_id())
            .and_then(|m| m.get(key).copied())
    })
}

// Passos ------------------------------------------------------------------

/// O que o tutorial sabe enquanto um passo está aberto.
#[derive(Debug, Default, Clone)]
pub struct StepState {
    /// O jogo foi aberto (▶) depois que o passo começou.
    pub played: bool,
}

type Check = fn(&ProjectWorkspace, &StepState) -> bool;

/// Um passo.
pub struct Step {
    pub title: &'static str,
    pub text: &'static str,
    /// Marca a destacar.
    pub target: Option<&'static str>,
    /// Painel a mostrar quando o passo começa.
    pub show: Option<Tab>,
    /// `None`: o passo é só leitura (botão Próximo).
    pub done: Option<Check>,
}

impl std::fmt::Debug for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Step").field("title", &self.title).finish()
    }
}

/// Um tutorial: o modelo do projeto e os passos.
pub struct Tutorial {
    pub id: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub minutes: u32,
    pub template: Template,
    /// Nome do projeto que ele cria.
    pub project: &'static str,
    pub steps: Vec<Step>,
}

impl std::fmt::Debug for Tutorial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tutorial").field("id", &self.id).finish()
    }
}

// Ajudas para conferir o projeto.

fn objects(ws: &ProjectWorkspace) -> Vec<&Object> {
    fn walk<'a>(list: &'a [Object], out: &mut Vec<&'a Object>) {
        for o in list {
            out.push(o);
            walk(&o.children, out);
        }
    }
    let mut out = Vec::new();
    if let Some(doc) = ws.active_scene.as_deref().and_then(|r| ws.scenes.get(r)) {
        walk(&doc.data.objects, &mut out);
    }
    out
}

fn with_image<'a>(ws: &'a ProjectWorkspace, image: &str) -> Vec<&'a Object> {
    objects(ws)
        .into_iter()
        .filter(|o| {
            o.components
                .iter()
                .any(|c| c.kind == "sprite" && c.str("image").ends_with(image))
        })
        .collect()
}

fn has_behavior(o: &Object, script: &str) -> bool {
    o.components
        .iter()
        .any(|c| c.is_behavior() && c.str("script") == script)
}

fn scene_rules(ws: &ProjectWorkspace) -> Vec<&scene_data::Rule> {
    ws.active_scene
        .as_deref()
        .and_then(|r| ws.scenes.get(r))
        .map(|d| d.data.events.iter().collect())
        .unwrap_or_default()
}

fn selected_name(ws: &ProjectWorkspace) -> Option<String> {
    let doc = ws.active_scene.as_deref().and_then(|r| ws.scenes.get(r))?;
    let id = *doc.selection.first()?;
    doc.data.find(id).map(|o| o.name.clone())
}

fn key_rule(rule: &scene_data::Rule, modo: &str) -> bool {
    rule.when
        .iter()
        .any(|c| c.id == "tecla" && c.str("modo") == modo && c.str("tecla") == "shift")
}

/// Os tutoriais, na ordem do menu.
pub fn all() -> &'static [Tutorial] {
    static LIST: std::sync::OnceLock<Vec<Tutorial>> = std::sync::OnceLock::new();
    LIST.get_or_init(|| vec![primeiro_jogo(), eventos()])
}

pub fn find(id: &str) -> Option<&'static Tutorial> {
    all().iter().find(|t| t.id == id)
}

fn primeiro_jogo() -> Tutorial {
    Tutorial {
        id: "primeiro-jogo",
        title: "Seu primeiro jogo",
        summary: "Monte uma fase de plataforma do zero: chão, herói que anda e pula, moedas, placar e um evento para vencer. Sem escrever código.",
        minutes: 15,
        template: Template::Vazio,
        project: "Primeiro jogo",
        steps: vec![
            Step {
                title: "Bem-vindo!",
                text: "Este projeto começa vazio. No centro está a cena (cenas/fase.cena): é onde você monta a fase com o mouse. O retângulo \"tela do jogo\" é o que cabe na janela do jogo.\n\nEste painel acompanha você. Ele avança sozinho quando você faz o que cada passo pede.",
                target: Some("cena"),
                show: None,
                done: None,
            },
            Step {
                title: "O chão",
                text: "No painel Arquivos, à esquerda, abra assets > imagens e arraste bloco.png para dentro da cena, um pouco abaixo do meio.",
                target: Some("arquivo:assets/imagens/bloco.png"),
                show: Some(Tab::Files),
                done: Some(|ws, _| !with_image(ws, "bloco.png").is_empty()),
            },
            Step {
                title: "Um chão comprido",
                text: "Com o bloco selecionado, no Inspetor, em Transformação > Escala, arraste o número do x para a direita até uns 20. O bloco vira uma faixa de grama.",
                target: Some("inspetor"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| with_image(ws, "bloco.png").iter().any(|o| o.scale_x >= 4.0)),
            },
            Step {
                title: "Chão firme",
                text: "Para o herói não atravessar o chão, ele precisa de um corpo físico. Clique em ➕ Adicionar componente (no fim do Inspetor) > Corpo físico.",
                target: Some("inspetor.adicionar"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    with_image(ws, "bloco.png")
                        .iter()
                        .any(|o| o.component("body").is_some())
                }),
            },
            Step {
                title: "Um chão que não cai",
                text: "Role o Inspetor até o Corpo físico. No Tipo, escolha Estático (não cai): senão o chão também cai com a gravidade!",
                target: Some("inspetor"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    with_image(ws, "bloco.png").iter().any(|o| {
                        o.component("body")
                            .is_some_and(|b| b.str("body") == "static")
                    })
                }),
            },
            Step {
                title: "O herói",
                text: "Arraste heroi.png para a cena, acima do chão. A imagem tem vários quadros (parado, andando, pulando): o editor já entendeu e mostra só o primeiro.",
                target: Some("arquivo:assets/imagens/heroi.png"),
                show: Some(Tab::Files),
                done: Some(|ws, _| !with_image(ws, "heroi.png").is_empty()),
            },
            Step {
                title: "Andar e pular",
                text: "Com o herói selecionado: ➕ Adicionar componente > Jogador > Jogador de plataforma. O corpo físico vem junto.",
                target: Some("inspetor.adicionar"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    with_image(ws, "heroi.png")
                        .iter()
                        .any(|o| has_behavior(o, "nn:plataforma"))
                }),
            },
            Step {
                title: "Teste!",
                text: "Aperte ▶ Testar daqui (F6). Ande com as setas ou A e D e pule com espaço. Depois feche a janela do jogo (ou deixe aberta: o que você muda aqui aparece lá na hora).",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Vida ao herói",
                text: "➕ Adicionar componente > Aparência > Animação automática. Nos quadros, use parado: 1-2, andando: 5-8, pulando: 6. Teste de novo se quiser ver.",
                target: Some("inspetor.adicionar"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    with_image(ws, "heroi.png")
                        .iter()
                        .any(|o| has_behavior(o, "nn:animacao"))
                }),
            },
            Step {
                title: "Uma moeda",
                text: "Arraste moeda.png para a cena, num lugar que o herói alcance. Adicione Itens > Coletável (ela some ao ser pega e soma 1 na variável moedas) e Aparência > Animação automática com parado: 1-4.",
                target: Some("arquivo:assets/imagens/moeda.png"),
                show: Some(Tab::Files),
                done: Some(|ws, _| {
                    with_image(ws, "moeda.png")
                        .iter()
                        .any(|o| has_behavior(o, "nn:coletavel"))
                }),
            },
            Step {
                title: "Várias moedas: um prefab",
                text: "Na Hierarquia, clique com o botão direito na moeda > 📦 Criar prefab. Depois arraste prefabs/moeda.prefab do painel Arquivos para a cena mais duas vezes. Mudar o prefab muda todas as moedas.",
                target: Some("hierarquia"),
                show: Some(Tab::Hierarchy),
                done: Some(|ws, _| {
                    objects(ws)
                        .iter()
                        .filter(|o| {
                            o.prefab
                                .as_ref()
                                .is_some_and(|l| l.path.ends_with("moeda.prefab"))
                        })
                        .count()
                        >= 3
                }),
            },
            Step {
                title: "O placar",
                text: "➕ Objeto > Texto. No Inspetor, marque \"Fixo na tela (HUD)\" e adicione Aparência > Mostrar valor. O texto \"Moedas: {moedas}\" mostra a variável. Arraste o placar para o alto da tela do jogo.",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| {
                    objects(ws).iter().any(|o| {
                        o.components
                            .iter()
                            .any(|c| c.kind == "text" && c.bool("hud"))
                            && has_behavior(o, "nn:mostrar_valor")
                    })
                }),
            },
            Step {
                title: "Um evento para vencer",
                text: "No painel Eventos (embaixo): ➕ Regra da cena > Objetos > Não sobrar nenhum, e escolha moeda. Em Então, ➕ ação > Aparência > Mudar texto: o texto do seu placar para \"Você venceu!\". (Ou Cenas > Reiniciar a cena.)",
                target: Some("eventos.regra_cena"),
                show: Some(Tab::Events),
                done: Some(|ws, _| {
                    scene_rules(ws).iter().any(|r| {
                        r.when
                            .iter()
                            .any(|c| c.id == "acabaram" && !c.str("nome").is_empty())
                            && !r.actions.is_empty()
                    })
                }),
            },
            Step {
                title: "Pegue todas!",
                text: "F6 de novo: pegue todas as moedas e veja o seu evento acontecer. A regra acende no painel Eventos quando dispara.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Salvar",
                text: "O ▶ Testar daqui salva tudo antes de abrir o jogo. Enquanto você monta a cena, Ctrl+S salva; uma aba com • tem alterações que ainda não foram salvas.",
                target: None,
                show: None,
                done: None,
            },
            Step {
                title: "Pronto: um jogo seu!",
                text: "Você montou uma fase com comportamentos, um prefab, uma variável no placar e um evento. Próximos passos: o tutorial Eventos (Ajuda > Tutoriais), os modelos Aventura e Nave (em Novo projeto) e o guia em Ajuda > Guia do Lua.",
                target: None,
                show: None,
                done: None,
            },
        ],
    }
}

fn eventos() -> Tutorial {
    Tutorial {
        id: "eventos",
        title: "Eventos: Quando... Então...",
        summary: "Com o modelo Aventura: leia as regras da porta, teste e crie regras suas com Quando e Enquanto.",
        minutes: 10,
        template: Template::Aventura,
        project: "Tutorial de eventos",
        steps: vec![
            Step {
                title: "Regras sem código",
                text: "Este projeto é o modelo Aventura: pegue a chave, abra a porta e ache o tesouro. Nada dele tem código: as regras ficam no painel Eventos, escritas como frases.",
                target: Some("cena"),
                show: None,
                done: None,
            },
            Step {
                title: "A porta",
                text: "Na Hierarquia, clique em porta.",
                target: Some("hierarquia:porta"),
                show: Some(Tab::Hierarchy),
                done: Some(|ws, _| selected_name(ws).as_deref() == Some("porta")),
            },
            Step {
                title: "Lendo uma regra",
                text: "No painel Eventos estão as regras da porta. A primeira diz: Quando o jogador estiver encostando em este objeto e a variável chaves for ≥ 1, Então: tira 1 chave, toca um som, solta faíscas e destrói a porta.\n\n\"Quando\" acontece uma vez, no momento em que tudo passa a valer.",
                target: Some("eventos"),
                show: Some(Tab::Events),
                done: None,
            },
            Step {
                title: "Teste",
                text: "F6 (Testar daqui): encoste na porta sem a chave e veja o aviso (a segunda regra). Depois pegue a chave e abra a porta.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Uma regra sua: Enquanto",
                text: "Feche o jogo. Clique em ➕ Regra da cena > Controles > Tecla ou botão. Troque \"apertar\" por \"estiver segurando\" e a tecla por Shift. No alto da regra, troque Quando por Enquanto: ela vale a cada quadro enquanto você segura.",
                target: Some("eventos.regra_cena"),
                show: Some(Tab::Events),
                done: Some(|ws, _| {
                    scene_rules(ws)
                        .iter()
                        .any(|r| r.repeat && key_rule(r, "segurar"))
                }),
            },
            Step {
                title: "Então...",
                text: "Na mesma regra, ➕ ação > Aparência > Mudar cor. Deixe \"o jogador\" e escolha uma cor forte.",
                target: Some("eventos"),
                show: Some(Tab::Events),
                done: Some(|ws, _| {
                    scene_rules(ws)
                        .iter()
                        .any(|r| key_rule(r, "segurar") && r.actions.iter().any(|a| a.id == "cor"))
                }),
            },
            Step {
                title: "E quando soltar?",
                text: "A cor fica depois de soltar o Shift. Faça outra regra: ➕ Regra > Controles > Tecla ou botão, com \"soltar\" e Shift, e a ação Mudar cor de o jogador para branco (#FFFFFF): a cor normal da imagem.",
                target: Some("eventos"),
                show: Some(Tab::Events),
                done: Some(|ws, _| {
                    scene_rules(ws)
                        .iter()
                        .any(|r| key_rule(r, "soltar") && r.actions.iter().any(|a| a.id == "cor"))
                }),
            },
            Step {
                title: "Veja acontecer",
                text: "F6 e segure Shift. Repare no painel Eventos: cada regra acende quando dispara.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Como código",
                text: "Clique em Ver como código, no painel Eventos: aparece o Lua que faz o mesmo que as regras. É um bom jeito de aprender a programar quando quiser.",
                target: Some("eventos.codigo"),
                show: Some(Tab::Events),
                done: None,
            },
            Step {
                title: "É isso!",
                text: "Regras têm condições (Quando ou Enquanto) e ações (Então). \"O outro\" é com quem o objeto encostou; \"esperar\" atrasa as ações seguintes; regras de um objeto vão junto com ele (e com as cópias, se for um prefab). Abra o prefab do asteroide do modelo Nave para ver mais.",
                target: None,
                show: None,
                done: None,
            },
        ],
    }
}

// Rodando -------------------------------------------------------------------

/// O que o tutorial pede ao aplicativo.
#[derive(Debug, Clone, PartialEq)]
pub enum TutorialAction {
    /// Fechar o tutorial.
    Close,
}

/// Um tutorial em andamento.
#[derive(Debug)]
pub struct TutorialRun {
    pub tutorial: &'static Tutorial,
    pub step: usize,
    state: StepState,
    /// Quando o passo ficou feito (para mostrar "Feito!" antes de avançar).
    done_at: Option<Instant>,
    shown: bool,
    /// Tamanho da janelinha no último quadro (para não cobrir o destaque).
    size: egui::Vec2,
    /// Quanto tempo o "Feito!" fica antes do próximo passo.
    pub pause: Duration,
    /// Vezes que o jogo foi aberto quando o passo começou.
    plays_at_start: Option<u64>,
}

/// Quanto tempo o "Feito!" fica antes de ir para o próximo passo.
const DONE_PAUSE: Duration = Duration::from_millis(900);

impl TutorialRun {
    pub fn new(tutorial: &'static Tutorial) -> Self {
        Self {
            tutorial,
            step: 0,
            state: StepState::default(),
            done_at: None,
            shown: false,
            size: egui::vec2(380.0, 230.0),
            pause: DONE_PAUSE,
            plays_at_start: None,
        }
    }

    fn go(&mut self, step: usize) {
        self.step = step.min(self.tutorial.steps.len() - 1);
        self.state = StepState::default();
        self.done_at = None;
        self.shown = false;
        self.plays_at_start = None;
    }

    pub fn current(&self) -> &Step {
        &self.tutorial.steps[self.step]
    }

    /// Confere o projeto: o passo foi feito? Avança depois de um instante.
    pub fn update(&mut self, ws: &mut ProjectWorkspace) {
        if !self.shown {
            self.shown = true;
            if let Some(tab) = self.current().show.clone() {
                ws.show_tab(tab);
            }
            // Arquivo do passo: aparece (e rola até ele) no painel Arquivos.
            if let Some(rel) = self
                .current()
                .target
                .and_then(|k| k.strip_prefix("arquivo:"))
            {
                ws.file_panel.reveal(rel);
            }
        }
        // Apertou ▶ (mesmo que o jogo já tenha fechado).
        let plays = *self.plays_at_start.get_or_insert(ws.plays);
        if ws.plays > plays {
            self.state.played = true;
        }
        let done = self.current().done.is_some_and(|f| f(ws, &self.state));
        match (done, self.done_at) {
            (true, None) => self.done_at = Some(Instant::now()),
            (false, Some(_)) => self.done_at = None,
            (true, Some(t))
                if t.elapsed() >= self.pause && self.step + 1 < self.tutorial.steps.len() =>
            {
                self.go(self.step + 1);
            }
            _ => {}
        }
    }

    /// A janelinha do tutorial e o destaque do passo.
    pub fn ui(&mut self, ctx: &egui::Context) -> Option<TutorialAction> {
        let mut action = None;
        let total = self.tutorial.steps.len();
        let step = self.current();
        let manual = step.done.is_none();
        let done = self.done_at.is_some();
        let last = self.step + 1 == total;
        let mut go = None;
        // Num canto que não cubra o que o passo destaca.
        let screen = ctx.content_rect();
        let target = step.target.and_then(|k| anchor_rect(ctx, k));
        let corners = [
            (egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -36.0)),
            (egui::Align2::LEFT_BOTTOM, egui::vec2(16.0, -36.0)),
            (egui::Align2::RIGHT_TOP, egui::vec2(-16.0, 110.0)),
            (egui::Align2::LEFT_TOP, egui::vec2(16.0, 110.0)),
        ];
        let place = |(align, offset): (egui::Align2, egui::Vec2)| {
            let r = align.align_size_within_rect(self.size, screen.shrink(0.0));
            r.translate(offset)
        };
        let (align, offset) = corners
            .into_iter()
            .find(|c| target.is_none_or(|t| !place(*c).intersects(t.expand(4.0))))
            .unwrap_or(corners[0]);
        let window =
            egui::Window::new(RichText::new(format!("🎓 {}", self.tutorial.title)).strong())
                .id(egui::Id::new("tutorial"))
                .anchor(align, offset)
                .default_width(360.0)
                .max_width(360.0)
                .resizable(false)
                .collapsible(true)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.weak(format!("Passo {} de {}", self.step + 1, total));
                        ui.add(
                            egui::ProgressBar::new((self.step + 1) as f32 / total as f32)
                                .desired_width(160.0)
                                .desired_height(6.0),
                        );
                    });
                    ui.add_space(4.0);
                    ui.label(RichText::new(step.title).size(17.0).strong());
                    ui.add_space(2.0);
                    ui.label(step.text);
                    ui.add_space(6.0);
                    if done {
                        ui.label(RichText::new("✔ Feito!").strong().color(theme::PLAY));
                        ui.ctx().request_repaint_after(Duration::from_millis(100));
                    } else if !manual {
                        ui.weak("Faça o que o passo pede: ele avança sozinho.");
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(self.step > 0, egui::Button::new("Voltar"))
                            .clicked()
                        {
                            go = Some(self.step - 1);
                        }
                        if last {
                            if ui
                                .add(
                                    egui::Button::new("Concluir")
                                        .fill(theme::ACCENT.gamma_multiply(0.75)),
                                )
                                .clicked()
                            {
                                action = Some(TutorialAction::Close);
                            }
                        } else if manual || done {
                            if ui
                                .add(
                                    egui::Button::new("Próximo")
                                        .fill(theme::ACCENT.gamma_multiply(0.75)),
                                )
                                .clicked()
                            {
                                go = Some(self.step + 1);
                            }
                        } else if ui
                            .small_button("Pular")
                            .on_hover_text("Ir para o próximo passo sem fazer este")
                            .clicked()
                        {
                            go = Some(self.step + 1);
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                        .small_button("Sair do tutorial")
                        .on_hover_text(
                            "O projeto continua aberto; dá para voltar em Ajuda > Tutoriais",
                        )
                        .clicked()
                    {
                        action = Some(TutorialAction::Close);
                    }
                        });
                    });
                });
        if let Some(r) = window {
            self.size = r.response.rect.size();
        }
        if let Some(s) = go {
            self.go(s);
        }
        // Destaque do lugar do passo.
        if let Some(key) = self.current().target
            && let Some(rect) = anchor_rect(ctx, key)
        {
            let t = ctx.input(|i| i.time) as f32;
            let pulse = (t * 4.0).sin() * 0.5 + 0.5;
            let color = Color32::from_rgb(0xFF, 0xC8, 0x3D).gamma_multiply(0.55 + 0.45 * pulse);
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("tutorial-destaque"),
            ));
            painter.rect_stroke(
                rect.expand(3.0 + 2.0 * pulse),
                6.0,
                Stroke::new(2.5, color),
                egui::StrokeKind::Outside,
            );
            ctx.request_repaint_after(Duration::from_millis(33));
        }
        action
    }
}

/// Pasta nova para o projeto do tutorial (não sobrescreve nada).
pub fn project_dir(parent: &std::path::Path, tutorial: &Tutorial) -> std::path::PathBuf {
    let base = nn_app::slug(tutorial.project);
    let mut dir = parent.join(&base);
    let mut n = 2;
    while dir.exists() {
        dir = parent.join(format!("{base}-{n}"));
        n += 1;
    }
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tutoriais_tem_passos_e_o_ultimo_conclui() {
        for t in all() {
            assert!(t.steps.len() >= 8, "{}", t.id);
            assert!(t.steps.last().unwrap().done.is_none(), "{}", t.id);
            assert!(
                t.steps
                    .iter()
                    .all(|s| !s.text.is_empty() && !s.title.is_empty())
            );
        }
        assert!(find("primeiro-jogo").is_some());
    }

    #[test]
    fn pasta_do_projeto_nao_repete() {
        let dir = std::env::temp_dir().join(format!("nn-tutorial-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let t = find("primeiro-jogo").unwrap();
        let a = project_dir(&dir, t);
        assert_eq!(a.file_name().unwrap(), "primeiro-jogo");
        std::fs::create_dir_all(&a).unwrap();
        assert_eq!(project_dir(&dir, t).file_name().unwrap(), "primeiro-jogo-2");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
