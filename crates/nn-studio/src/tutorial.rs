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

pub fn anchor_rect(ctx: &egui::Context, key: &str) -> Option<Rect> {
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

fn with_model<'a>(ws: &'a ProjectWorkspace, file: &str) -> Vec<&'a Object> {
    objects(ws)
        .into_iter()
        .filter(|o| {
            o.components
                .iter()
                .any(|c| c.kind == "model" && c.str("file").ends_with(file))
        })
        .collect()
}

/// Objetos com partículas cujo efeito começa com esta cor.
fn with_particles<'a>(ws: &'a ProjectWorkspace, color_start: &str) -> Vec<&'a Object> {
    objects(ws)
        .into_iter()
        .filter(|o| {
            o.component("particles")
                .is_some_and(|c| color_start.is_empty() || c.str("color_start") == color_start)
        })
        .collect()
}

/// Todas as regras da cena: as da cena e as dos objetos.
fn all_rules(ws: &ProjectWorkspace) -> Vec<&scene_data::Rule> {
    let mut out = scene_rules(ws);
    for o in objects(ws) {
        out.extend(o.events.iter());
    }
    out
}

/// O terreno da cena aberta (o primeiro).
fn terrain(ws: &ProjectWorkspace) -> Option<nn_render::d3::TerrainData> {
    objects(ws)
        .into_iter()
        .find_map(|o| o.component("terrain"))
        .map(crate::terrain_tool::terrain_of)
}

fn doc_of(ws: &ProjectWorkspace) -> Option<&crate::scene_view::SceneDoc> {
    ws.active_scene.as_deref().and_then(|r| ws.scenes.get(r))
}

fn body3d_is(o: &Object, kind: &str) -> bool {
    o.component("body3d").is_some_and(|b| b.str("body") == kind)
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
    LIST.get_or_init(|| {
        vec![
            primeiro_jogo(),
            mundo_3d(),
            efeitos_3d(),
            terreno(),
            inimigos_3d(),
            um_chefe(),
            uma_caverna(),
            dia_e_noite(),
            um_lago(),
            portas_e_chaves(),
            uma_conversa(),
            eventos(),
            desenhe(),
            tiles(),
            menu(),
            particulas(),
        ]
    })
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
                text: "Com o bloco selecionado, no Inspetor, em Transformação > Escala, arraste o número do x para a direita até uns 20. Depois, em Imagem > Preencher, escolha Repetir: em vez de esticar, o bloco se repete e vira uma faixa de grama.",
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
                text: "➕ Adicionar componente > Aparência > Animação automática. Em cada estado, clique no ☰ e escolha a animação: Parado: parado, Andando: andar, Pulando: pular (a imagem já vem com elas, em heroi.anim). Teste de novo se quiser ver.",
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
                text: "Arraste moeda.png para a cena, num lugar que o herói alcance. Adicione Itens > Coletável (ela some ao ser pega e soma 1 na variável moedas) e, em Imagem > Animação, escolha girar no ☰.",
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

fn mundo_3d() -> Tutorial {
    Tutorial {
        id: "primeiro-mundo-3d",
        title: "Seu primeiro mundo 3D",
        summary: "Monte um mundo 3D sem código: um herói animado que anda e pula com a câmera atrás dele, árvores, uma caixa que cai, um bicho, uma estrela, uma plataforma que anda, o sol da tarde e um evento de vitória.",
        minutes: 15,
        template: Template::Vazio3D,
        project: "Primeiro mundo 3D",
        steps: vec![
            Step {
                title: "Um mundo 3D",
                text: "Esta é uma cena 3D (cenas/mundo.cena): um chão verde, o céu e uma câmera. A grade do chão mede 1 metro; a linha vermelha é o eixo X e a azul, o Z. Y é a altura.\n\nEste painel acompanha você e avança sozinho quando você faz o que cada passo pede.",
                target: Some("cena"),
                show: None,
                done: None,
            },
            Step {
                title: "Olhe em volta",
                text: "Segure o botão direito do mouse em cima da cena e mova o mouse para virar a cabeça. Ainda segurando, W, A, S e D andam e Q e E descem e sobem. A rodinha aproxima e afasta. Perdeu-se? Aperte F (⛶ Enquadrar).",
                target: Some("cena"),
                show: None,
                done: None,
            },
            Step {
                title: "Criar com o botão direito",
                text: "Agora clique e solte o botão direito no chão, sem arrastar: abre o menu de criar. Escolha Cubo e ele aparece ali, em pé.\n\nOutro jeito: com o mouse na cena, Shift+A abre uma busca. Digite \"esfera\" e aperte Enter. Num objeto, o botão direito mostra o menu dele (duplicar, focar, excluir...).",
                target: Some("cena"),
                show: None,
                done: Some(|ws, _| {
                    objects(ws).iter().any(|o| {
                        o.component("shape3d")
                            .is_some_and(|c| c.str("shape") != "plane")
                    })
                }),
            },
            Step {
                title: "O herói",
                text: "No painel Arquivos, abra a pasta modelos e arraste heroi.glb para o meio do chão. Os modelos caem em pé onde o mouse aponta.",
                target: Some("arquivo:modelos/heroi.glb"),
                show: Some(Tab::Files),
                done: Some(|ws, _| !with_model(ws, "heroi.glb").is_empty()),
            },
            Step {
                title: "Andar e pular",
                text: "Com o herói selecionado: ➕ Adicionar componente (no fim do Inspetor) > Jogador > Jogador 3D. Ele anda, pula e leva a câmera atrás dele.",
                target: Some("inspetor.adicionar"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    with_model(ws, "heroi.glb")
                        .iter()
                        .any(|o| has_behavior(o, "nn:jogador3d"))
                }),
            },
            Step {
                title: "Teste!",
                text: "Aperte ▶ Testar daqui (F6). WASD anda para onde a câmera olha, espaço pula e o mouse gira a câmera. Esc solta o mouse. O herói tem animações: ele fica parado respirando, corre e pula sozinho. Depois feche a janela do jogo.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Árvores que bloqueiam",
                text: "Arraste arvore.glb para a cena. Para o herói não atravessar a árvore: ➕ Adicionar componente > Corpo 3D. O Tipo padrão é Parado, e a forma automática usa a malha exata do modelo. Ctrl+D duplica a árvore (com o corpo).",
                target: Some("arquivo:modelos/arvore.glb"),
                show: Some(Tab::Files),
                done: Some(|ws, _| {
                    with_model(ws, "arvore.glb")
                        .iter()
                        .any(|o| body3d_is(o, "static"))
                }),
            },
            Step {
                title: "Uma fileira de árvores",
                text: "Com a árvore selecionada, segure Alt e arraste a seta vermelha (X): sai uma cópia, e a original fica. Agora aperte Ctrl+D duas vezes: cada árvore nova anda o mesmo tanto que a cópia andou. Uma fileira em segundos!\n\nOutro jeito: Shift+D faz uma cópia que segue o mouse até você clicar.",
                target: Some("cena"),
                show: None,
                done: Some(|ws, _| with_model(ws, "arvore.glb").len() >= 4),
            },
            Step {
                title: "Uma caixa que cai",
                text: "Arraste caixote.glb para a cena e, com a seta verde (Y), suba ele uns 3 metros. Adicione um Corpo 3D e, no Tipo, escolha \"Cai e é empurrado\". Teste com F6: ele cai, e o herói empurra.",
                target: Some("arquivo:modelos/caixote.glb"),
                show: Some(Tab::Files),
                done: Some(|ws, _| {
                    with_model(ws, "caixote.glb")
                        .iter()
                        .any(|o| body3d_is(o, "dynamic"))
                }),
            },
            Step {
                title: "Um bicho que se mexe",
                text: "Arraste bicho.glb para a cena. Ele também tem animações: ➕ Adicionar componente > Animação 3D e, em Animação, clique no ☰ e escolha andar. Ele já anda na cena, aqui no editor.",
                target: Some("arquivo:modelos/bicho.glb"),
                show: Some(Tab::Files),
                done: Some(|ws, _| {
                    with_model(ws, "bicho.glb").iter().any(|o| {
                        o.component("anim3d")
                            .is_some_and(|a| !a.str("animation").is_empty())
                    })
                }),
            },
            Step {
                title: "Uma estrela",
                text: "Arraste estrela.glb para a cena, suba um pouco do chão e adicione Itens > Coletável: ela some quando o herói encosta. Na Variável, escreva estrelas. Para brilhar mais, adicione também Movimento > Girar e flutuar.",
                target: Some("arquivo:modelos/estrela.glb"),
                show: Some(Tab::Files),
                done: Some(|ws, _| {
                    with_model(ws, "estrela.glb")
                        .iter()
                        .any(|o| has_behavior(o, "nn:coletavel"))
                }),
            },
            Step {
                title: "Uma plataforma que anda",
                text: "Arraste plataforma.glb para a cena: ela fica em cima do chão, com o topo a 1 m de altura. Escale em X e Z (R, Escalar) para ficar maior e adicione Movimento > Plataforma que anda: ela já ganha um Corpo 3D que se move sozinho e leva o herói junto.",
                target: Some("arquivo:modelos/plataforma.glb"),
                show: Some(Tab::Files),
                done: Some(|ws, _| {
                    with_model(ws, "plataforma.glb")
                        .iter()
                        .any(|o| has_behavior(o, "nn:plataforma3d"))
                }),
            },
            Step {
                title: "Fim de tarde",
                text: "Clique em ☀ Ambiente, na barra da cena. No Inspetor, baixe a altura do Sol para uns 15°: a luz fica dourada e as sombras, compridas. Mexa nas cores do céu à vontade.",
                target: Some("cena.ambiente"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    ws.active_scene
                        .as_deref()
                        .and_then(|r| ws.scenes.get(r))
                        .is_some_and(|d| d.data.env("sun_height").num() <= 30.0)
                }),
            },
            Step {
                title: "Um evento de vitória",
                text: "No painel Eventos: ➕ Regra da cena > Objetos > Não sobrar nenhum, e escolha a sua estrela. Em Então, ➕ ação > Som > Tocar som (vitoria.wav) e Cenas > Reiniciar a cena.",
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
                title: "Pegue a estrela!",
                text: "F6 de novo: suba na plataforma, pegue a estrela e veja o seu evento acontecer.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Pronto: um mundo seu!",
                text: "Você montou um mundo 3D com um herói animado, física, um bicho que anda, uma plataforma que anda, o Ambiente e um evento. Próximos passos: o modelo 3D (Novo projeto), uma ilha inteira sem código, para ver como tudo se junta; e Colisões, na barra da cena, para ver os corpos.",
                target: None,
                show: None,
                done: None,
            },
        ],
    }
}

fn efeitos_3d() -> Tutorial {
    Tutorial {
        id: "efeitos-3d",
        title: "Efeitos no 3D",
        summary: "Uma fogueira com fogo, fumaça e o som estalando (mais alto de perto), poeira nos pés do herói ao pular e uma chuva caindo.",
        minutes: 10,
        template: Template::Vazio3D,
        project: "Efeitos 3D",
        steps: vec![
            Step {
                title: "Fogo, fumaça e som",
                text: "Numa cena 3D, as partículas e os sons ficam no mundo: as partículas aparecem atrás e na frente das coisas, e o som de um objeto fica mais alto quando você chega perto dele.\n\nVamos montar uma fogueira.",
                target: Some("cena"),
                show: None,
                done: None,
            },
            Step {
                title: "O fogo",
                text: "➕ Objeto > Partículas > Fogo. Ele já anima aqui na vista. As partículas saem para o \"cima\" do objeto: com ⟲ Girar dá para apontar para outro lado.",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| !with_particles(ws, "#FFE066").is_empty()),
            },
            Step {
                title: "A fumaça",
                text: "➕ Objeto > Partículas > Fumaça, e com a seta verde (Y) suba ela uns 1,5 m, para sair por cima do fogo.",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| with_particles(ws, "#9A9A9AAA").iter().any(|o| o.y >= 0.8)),
            },
            Step {
                title: "O estalar",
                text: "No painel Arquivos, abra assets > sons e arraste fogueira.wav para perto do fogo. No Inspetor, marque Repetir. \"Som no espaço 3D\" já vem ligado: o som sai dali.",
                target: Some("arquivo:assets/sons/fogueira.wav"),
                show: Some(Tab::Files),
                done: Some(|ws, _| {
                    objects(ws).iter().any(|o| {
                        o.component("sound").is_some_and(|c| {
                            c.str("sound").ends_with("fogueira.wav")
                                && c.bool("loop")
                                && c.bool("spatial")
                        })
                    })
                }),
            },
            Step {
                title: "Alguém para ouvir",
                text: "➕ Objeto > Jogador 3D. Ele anda, pula e leva a câmera: quem ouve os sons é a câmera.",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| objects(ws).iter().any(|o| has_behavior(o, "nn:jogador3d"))),
            },
            Step {
                title: "Teste!",
                text: "F6 (▶ Testar daqui). Ande para perto da fogueira e para longe: o estalar sobe e desce, e passa de um ouvido para o outro quando você vira.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Poeira nos pés",
                text: "➕ Objeto > Partículas > Poeira. Na Hierarquia, arraste a poeira para cima do Jogador: ela vira filha dele e vai junto. Deixe ela nos pés (y = 0).",
                target: Some("hierarquia"),
                show: Some(Tab::Hierarchy),
                done: Some(|ws, _| {
                    objects(ws).iter().any(|o| {
                        has_behavior(o, "nn:jogador3d")
                            && o.children.iter().any(|c| {
                                c.component("particles")
                                    .is_some_and(|p| p.str("color_start") == "#E8DCC4CC")
                            })
                    })
                }),
            },
            Step {
                title: "Soltar ao pular",
                text: "A poeira só sai quando pedem. Selecione o Jogador e, no painel Eventos: ➕ Regra de Jogador > Mensagens > Receber mensagem, e troque \"abrir\" por pulou (o Jogador 3D manda essa mensagem a cada pulo). Em Então: ➕ ação > Aparência > Soltar partículas, de poeira.",
                target: Some("eventos.regra_cena"),
                show: Some(Tab::Events),
                done: Some(|ws, _| {
                    all_rules(ws).iter().any(|r| {
                        r.when
                            .iter()
                            .any(|c| c.id == "mensagem" && c.str("msg") == "pulou")
                            && r.actions.iter().any(|a| a.id == "soltar_particulas")
                    })
                }),
            },
            Step {
                title: "Chuva",
                text: "➕ Objeto > Partículas > Chuva, e suba ela uns 10 m: a chuva nasce numa área deitada e cai. Quer neve? Partículas > Neve.",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| {
                    with_particles(ws, "#C8DDF0AA")
                        .iter()
                        .chain(with_particles(ws, "#FFFFFF").iter())
                        .any(|o| {
                            o.y >= 4.0
                                && o.component("particles")
                                    .is_some_and(|c| c.str("area") == "retangulo")
                        })
                }),
            },
            Step {
                title: "Teste de novo",
                text: "F6: pule para ver a poeira, ande até a fogueira na chuva.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Pronto: um mundo com vida!",
                text: "Você usou partículas 3D, som no espaço 3D e um evento com mensagem. O mar.wav (em assets/sons) fica bom na beira de um lago, com Repetir, e o modelo 3D (Novo projeto) tem uma ilha inteira com fogueira, ondas e poeira.",
                target: None,
                show: None,
                done: None,
            },
        ],
    }
}

fn terreno() -> Tutorial {
    use crate::terrain_tool::TerrainBrush;
    Tutorial {
        id: "pinte-um-terreno",
        title: "Pinte um terreno",
        summary: "Um chão de verdade para o seu mundo 3D: morros, um lago com água, cores de grama, pedra e areia, árvores espalhadas com o pincel e um herói para passear.",
        minutes: 12,
        template: Template::Vazio3D,
        project: "Meu terreno",
        steps: vec![
            Step {
                title: "Um terreno",
                text: "Um Terreno é um chão grande que se pinta com o mouse: você sobe morros, cava vales, pinta cores e põe água, como num programa de desenho.",
                target: Some("cena"),
                show: None,
                done: None,
            },
            Step {
                title: "Crie o terreno",
                text: "➕ Objeto > Terreno. Ele vem plano, com 64 × 64 m. O chão verde do modelo pode ficar (ou apague: Del).",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| terrain(ws).is_some()),
            },
            Step {
                title: "O pincel",
                text: "Com o Terreno selecionado, clique em ✏ Pintar terreno, na barra da vista. Agora o botão esquerdo pinta em vez de selecionar, e um círculo mostra o pincel. O botão direito continua voando.",
                target: Some("cena.terreno"),
                show: None,
                done: Some(|ws, _| doc_of(ws).is_some_and(|d| d.terrain_tool.active)),
            },
            Step {
                title: "Um morro",
                text: "Escolha ⬆ Subir e segure o botão esquerdo em cima do terreno: o chão sobe enquanto você segura. Faça um morro de uns 3 m. Tamanho e Força mudam o pincel; Ctrl+Z desfaz.",
                target: Some("terreno:Raise"),
                show: None,
                done: Some(|ws, _| {
                    terrain(ws).is_some_and(|t| t.heights.iter().any(|&h| h >= 2.5))
                }),
            },
            Step {
                title: "Um buraco para o lago",
                text: "Agora ⬇ Descer, num lugar plano: cave um buraco de uns 2 m. (Segurar Ctrl faz o contrário do pincel.)",
                target: Some("terreno:Lower"),
                show: None,
                done: Some(|ws, _| {
                    terrain(ws).is_some_and(|t| t.heights.iter().any(|&h| h <= -1.5))
                }),
            },
            Step {
                title: "Suave",
                text: "Alisar tira os bicos e os degraus. Passe em volta do morro e do buraco. Aplainar puxa para a altura de onde você começou: bom para uma estrada.",
                target: Some("terreno:Smooth"),
                show: None,
                done: Some(|ws, _| {
                    doc_of(ws).is_some_and(|d| {
                        matches!(
                            d.terrain_tool.brush,
                            TerrainBrush::Smooth | TerrainBrush::Flatten
                        ) && d.terrain_tool.stroke >= 3
                    })
                }),
            },
            Step {
                title: "Cores",
                text: "Pintar cor, escolha 3 Pedra e pinte o alto do morro; depois 4 Areia em volta do buraco. As cores mudam no Inspetor (Cor 1 a 4).",
                target: Some("terreno:Paint"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    terrain(ws).is_some_and(|t| t.layers.iter().any(|l| l[2] >= 128))
                }),
            },
            Step {
                title: "Água",
                text: "No Inspetor, no Terreno, marque Água. Ajuste o Nível da água (uns -0,5 m) para encher o buraco: virou um lago.",
                target: Some("inspetor"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    objects(ws)
                        .iter()
                        .any(|o| o.component("terrain").is_some_and(|c| c.bool("water")))
                }),
            },
            Step {
                title: "Árvores",
                text: "Espalhar, escolha modelos/arvore.glb e passe o pincel pela grama: as árvores nascem no chão, com tamanho e giro sorteados, já com Corpo 3D. Ctrl apaga. Ponha umas 5.",
                target: Some("terreno:Scatter"),
                show: None,
                done: Some(|ws, _| with_model(ws, "arvore.glb").len() >= 5),
            },
            Step {
                title: "Alguém para passear",
                text: "Clique de novo em ✏ Pintar terreno para desligar o pincel. ➕ Objeto > Jogador 3D, e com a seta verde (Y) deixe ele um pouco acima do chão.",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| objects(ws).iter().any(|o| has_behavior(o, "nn:jogador3d"))),
            },
            Step {
                title: "Teste!",
                text: "F6: suba o morro, ande entre as árvores e entre no lago (o Jogador 3D volta para a margem e recebe a mensagem \"agua\").",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Pronto: um lugar seu!",
                text: "Você pintou morros, um lago, cores e árvores. O modelo Vale 3D (Novo projeto) é um vale inteiro feito assim, com trilha, estrelas e uma fogueira.",
                target: None,
                show: None,
                done: None,
            },
        ],
    }
}

/// O comportamento `script` do objeto, se tiver.
fn behavior_of<'a>(o: &'a Object, script: &str) -> Option<&'a scene_data::Component> {
    o.components
        .iter()
        .find(|c| c.is_behavior() && c.str("script") == script)
}

fn inimigos_3d() -> Tutorial {
    Tutorial {
        id: "inimigos-3d",
        title: "Inimigos no 3D",
        summary: "Um herói com vida e corações na tela, um besouro que persegue e machuca (e morre se você pular em cima), uma barra de vida e uma poça de lava.",
        minutes: 12,
        template: Template::Vazio3D,
        project: "Inimigos 3D",
        steps: vec![
            Step {
                title: "Perigo!",
                text: "Um jogo fica bom quando dá para perder. Neste tutorial: vida, corações na tela, um inimigo que persegue e uma poça de lava. Tudo sem código.",
                target: Some("cena"),
                show: None,
                done: None,
            },
            Step {
                title: "O herói",
                text: "Arraste modelos/heroi.glb para a cena e adicione Jogador > Jogador 3D (ou ➕ Objeto > Jogador 3D).",
                target: Some("arquivo:modelos/heroi.glb"),
                show: Some(Tab::Files),
                done: Some(|ws, _| objects(ws).iter().any(|o| has_behavior(o, "nn:jogador3d"))),
            },
            Step {
                title: "Vida",
                text: "Com o herói selecionado: ➕ Adicionar componente > Jogador > Vida. Ele tem 3 vidas, guardadas na variável vidas. Quando acabam, a cena começa de novo.",
                target: Some("inspetor.adicionar"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    objects(ws)
                        .iter()
                        .any(|o| has_behavior(o, "nn:jogador3d") && has_behavior(o, "nn:vida"))
                }),
            },
            Step {
                title: "Corações na tela",
                text: "➕ Objeto > Na tela (HUD) > Ícones. O que é da tela aparece por cima da cena, na moldura da tela do jogo: arraste os ícones para o canto de cima. No Inspetor, em Imagem, escolha assets/imagens/coracao.png e, em Variável, vidas: um coração por vida.",
                target: Some("cena.objeto"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    objects(ws).iter().any(|o| {
                        o.component("icons")
                            .is_some_and(|c| c.str("var") == "vidas")
                    })
                }),
            },
            Step {
                title: "Um besouro",
                text: "Arraste modelos/bicho.glb para a cena, longe do herói, e adicione Inimigos > Inimigo 3D. Ele anda indo e voltando e, quando o herói chega perto, persegue.",
                target: Some("arquivo:modelos/bicho.glb"),
                show: Some(Tab::Files),
                done: Some(|ws, _| {
                    with_model(ws, "bicho.glb")
                        .iter()
                        .any(|o| has_behavior(o, "nn:inimigo3d"))
                }),
            },
            Step {
                title: "Teste!",
                text: "F6. Deixe o besouro te pegar: um coração some, você é empurrado e pisca um pouco. Agora pule em cima dele: ele some.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Um besouro mais forte",
                text: "Selecione o besouro e adicione Jogador > Vida também: 3 vidas, Quando acabar: some, e marque Barra de vida em cima. Agora ele aguenta três pisões, e a barra mostra quanto falta.",
                target: Some("inspetor.adicionar"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    objects(ws).iter().any(|o| {
                        has_behavior(o, "nn:inimigo3d")
                            && behavior_of(o, "nn:vida").is_some_and(|c| {
                                c.str("ao_acabar") == "destruir" && c.bool("barra")
                            })
                    })
                }),
            },
            Step {
                title: "Lava",
                text: "➕ Objeto > Cubo. Achate (escala Y uns 0,1), alargue em X e Z, pinte de laranja e adicione Inimigos > Perigo: quem encosta perde uma vida e é empurrado.",
                target: Some("cena.objeto"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    objects(ws)
                        .iter()
                        .any(|o| o.component("shape3d").is_some() && has_behavior(o, "nn:perigo"))
                }),
            },
            Step {
                title: "Teste de novo",
                text: "F6: três pisões no besouro, e cuidado com a lava. Se as vidas acabarem, a cena começa de novo.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Pronto: um desafio!",
                text: "Você usou Vida, Ícones, Inimigo 3D e Perigo. Ideias: um Checkpoint no meio do caminho (Fases > Checkpoint), mais besouros com Vagar, ou Atirar no herói (com um prefab de bola) para derrubar os inimigos de longe.",
                target: None,
                show: None,
                done: None,
            },
        ],
    }
}

/// O herói (com Jogador 3D) segura algo num osso (um filho com Preso a um osso)?
fn segura_algo(ws: &ProjectWorkspace) -> bool {
    objects(ws).iter().any(|o| {
        has_behavior(o, "nn:jogador3d")
            && o.children.iter().any(|ch| {
                ch.component("bone")
                    .is_some_and(|b| !b.str("bone").is_empty())
            })
    })
}

/// Algum prefab do projeto tem um Inimigo 3D?
fn prefab_de_inimigo(ws: &ProjectWorkspace) -> bool {
    ws.files.entries().iter().any(|e| {
        e.rel.ends_with(".prefab")
            && std::fs::read_to_string(ws.files.root().join(&e.rel))
                .is_ok_and(|t| t.contains("nn:inimigo3d"))
    })
}

fn um_chefe() -> Tutorial {
    Tutorial {
        id: "um-chefe",
        title: "Um chefe",
        summary: "Um herói com espada, ondas de besouros com o placar da onda e um chefe, o besouro-rei, com a barra de vida no alto da tela e um evento de vitória.",
        minutes: 15,
        template: Template::Vazio3D,
        project: "Um chefe",
        steps: vec![
            Step {
                title: "A luta final",
                text: "Neste tutorial você monta uma arena: o herói com uma espada, ondas de besouros e, no fim, um chefe com vários ataques. Tudo sem código.",
                target: Some("cena"),
                show: None,
                done: None,
            },
            Step {
                title: "O herói",
                text: "Arraste modelos/heroi.glb para a cena e adicione Jogador > Jogador 3D e Jogador > Vida.",
                target: Some("arquivo:modelos/heroi.glb"),
                show: Some(Tab::Files),
                done: Some(|ws, _| {
                    objects(ws)
                        .iter()
                        .any(|o| has_behavior(o, "nn:jogador3d") && has_behavior(o, "nn:vida"))
                }),
            },
            Step {
                title: "A espada na mão",
                text: "Arraste modelos/espada.glb para a cena. Na Hierarquia, arraste a espada para cima do herói (ela vira filha dele) e, no Inspetor da espada, ➕ Adicionar componente > Preso a um osso. Ela vai para a mão direita (mao_d).",
                target: Some("arquivo:modelos/espada.glb"),
                show: Some(Tab::Files),
                done: Some(|ws, _| segura_algo(ws)),
            },
            Step {
                title: "Golpear",
                text: "Selecione o herói e adicione Ações > Golpe. No jogo, o clique (ou J) golpeia: quem está na frente perde vida e é empurrado.",
                target: Some("inspetor.adicionar"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    objects(ws)
                        .iter()
                        .any(|o| has_behavior(o, "nn:jogador3d") && has_behavior(o, "nn:golpe"))
                }),
            },
            Step {
                title: "Um besouro",
                text: "Arraste modelos/bicho.glb para a cena e adicione Inimigos > Inimigo 3D e Jogador > Vida (2 vidas, Quando acabar: some, e marque Mostrar o dano). Para ele morder em vez de só encostar, adicione também Ações > Golpe com Golpeia: sozinho, quando o alvo chega perto.",
                target: Some("arquivo:modelos/bicho.glb"),
                show: Some(Tab::Files),
                done: Some(|ws, _| {
                    with_model(ws, "bicho.glb")
                        .iter()
                        .any(|o| has_behavior(o, "nn:inimigo3d") && has_behavior(o, "nn:vida"))
                }),
            },
            Step {
                title: "Um prefab de besouro",
                text: "As ondas criam cópias do besouro. Clique com o botão direito nele na Hierarquia e escolha 📦 Criar prefab. Depois, apague o besouro da cena (Delete): as ondas trazem os outros.",
                target: Some("hierarquia"),
                show: Some(Tab::Hierarchy),
                done: Some(|ws, _| prefab_de_inimigo(ws)),
            },
            Step {
                title: "As ondas",
                text: "➕ Objeto > Ondas de inimigos. No Inspetor, em Inimigo (prefab), escolha o besouro. Os quatro filhos (Saída) são de onde eles saem: espalhe-os em volta. Começa quando o jogador chega perto.",
                target: Some("cena.objeto"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    objects(ws).iter().any(|o| {
                        behavior_of(o, "nn:ondas").is_some_and(|c| !c.str("prefab").is_empty())
                    })
                }),
            },
            Step {
                title: "O placar da onda",
                text: "➕ Objeto > Na tela (HUD) > Texto, no canto de cima, e adicione Aparência > Mostrar valor com o texto: Onda {onda}   Faltam {restantes}.",
                target: Some("cena.objeto"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    objects(ws).iter().any(|o| {
                        behavior_of(o, "nn:mostrar_valor")
                            .is_some_and(|c| c.str("texto").contains("{onda}"))
                    })
                }),
            },
            Step {
                title: "O chefe",
                text: "Arraste modelos/besourao.glb para o meio da arena e aumente a escala para 1,5. Adicione Jogador > Vida (12 vidas, Quando acabar: nada) e Inimigos > Chefe. No Chefe, em A luta começa, escolha com a mensagem \"comecar\". Ele já vem com o golpe, a investida e o pulo.",
                target: Some("arquivo:modelos/besourao.glb"),
                show: Some(Tab::Files),
                done: Some(|ws, _| {
                    objects(ws)
                        .iter()
                        .any(|o| has_behavior(o, "nn:chefe") && has_behavior(o, "nn:vida"))
                }),
            },
            Step {
                title: "Acorde o chefe",
                text: "No painel Eventos, uma regra da cena: Quando terminarem as ondas de Ondas (Mensagens > Vencer todas as ondas), Então Mandar mensagem comecar para o besouro (o nome dele na cena).",
                target: Some("eventos"),
                show: Some(Tab::Events),
                done: Some(|ws, _| {
                    all_rules(ws).iter().any(|r| {
                        r.when.iter().any(|c| c.id == "ondas_terminaram")
                            && r.actions
                                .iter()
                                .any(|a| a.id == "mensagem" && a.str("msg") == "comecar")
                    })
                }),
            },
            Step {
                title: "Vitória",
                text: "Mais uma regra: Quando a vida do besouro acabar (Mensagens > Vida acabar), Então Tocar som assets/sons/vitoria.wav (ou o que quiser: mostrar um texto, ir para outra cena).",
                target: Some("eventos"),
                show: Some(Tab::Events),
                done: Some(|ws, _| {
                    all_rules(ws).iter().any(|r| {
                        r.when
                            .iter()
                            .any(|c| c.id == "vida_acabou" || c.id == "derrotar")
                            && !r.actions.is_empty()
                    })
                }),
            },
            Step {
                title: "Lute!",
                text: "F6. Chegue perto das Saídas: as ondas começam. Depois delas, o chefe acorda. Fique de olho no aviso (ele pisca e se prepara) e desvie; quando ele bater numa parede na investida, fica tonto: é a hora de golpear.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Pronto: um chefe!",
                text: "Você usou Golpe, Preso a um osso, Ondas de inimigos, Chefe e os eventos de onda. Ideias: no Chefe, ligue os tiros em leque (com um prefab de Projétil) ou os ajudantes; ponha uma Zona de gatilho que fecha uma Porta atrás do herói; ou deixe o chefe soltar uma estrela ao ser derrotado.",
                target: None,
                show: None,
                done: None,
            },
        ],
    }
}

/// As Luzes da cena (os componentes).
fn lights(ws: &ProjectWorkspace) -> Vec<&scene_data::Component> {
    objects(ws)
        .into_iter()
        .filter_map(|o| o.component("light3d"))
        .collect()
}

fn uma_caverna() -> Tutorial {
    Tutorial {
        id: "uma-caverna",
        title: "Uma caverna iluminada",
        summary: "Uma caverna escura com uma tocha que tremula, um cristal que brilha, um holofote que você acende e apaga com uma tecla e o herói andando com a luz.",
        minutes: 10,
        template: Template::Vazio3D,
        project: "Uma caverna",
        steps: vec![
            Step {
                title: "Luzes no escuro",
                text: "De dia, o sol ilumina tudo. Numa caverna (ou à noite), quem ilumina são as luzes que você põe: tochas, lâmpadas, holofotes e coisas que brilham.\n\nVamos apagar o sol e acender a caverna.",
                target: Some("cena"),
                show: None,
                done: None,
            },
            Step {
                title: "Apague o dia",
                text: "Clique num lugar vazio da cena (sem objeto escolhido) e, no Inspetor, em ☀ Ambiente > Pronto, clique em Caverna: o céu e o sol somem, e fica só uma luz fraquinha.",
                target: Some("inspetor:ambiente:caverna"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    doc_of(ws).is_some_and(|d| {
                        d.data.env("sky").num() == 0.0
                            && d.data
                                .env("sun_color")
                                .str()
                                .eq_ignore_ascii_case("#000000")
                    })
                }),
            },
            Step {
                title: "Uma tocha",
                text: "➕ Objeto > Luz > Tocha. A chama tremula como fogo e acende o chão em volta. A luz fica na Chama (na Hierarquia, dentro da Tocha).",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| lights(ws).iter().any(|c| c.num("flicker") > 0.0)),
            },
            Step {
                title: "Mais longe",
                text: "Na Hierarquia, abra a Tocha e escolha a Chama. No Inspetor, na Luz, aumente o Alcance para 10 m (até onde a luz chega) e, se quiser, troque a Cor.",
                target: Some("hierarquia"),
                show: Some(Tab::Hierarchy),
                done: Some(|ws, _| lights(ws).iter().any(|c| c.num("range") >= 9.5)),
            },
            Step {
                title: "Um cristal que brilha",
                text: "No painel Arquivos, abra modelos e arraste cristal.glb para a cena, longe da tocha. Ele aparece aceso mesmo no escuro: o brilho vem no arquivo do modelo.",
                target: Some("arquivo:modelos/cristal.glb"),
                show: Some(Tab::Files),
                done: Some(|ws, _| !with_model(ws, "cristal.glb").is_empty()),
            },
            Step {
                title: "Uma forma que brilha",
                text: "➕ Objeto > Esfera e, no Inspetor da Forma 3D, ponha Brilho = 1. Ela fica acesa com a própria cor, mas não ilumina em volta: para isso, uma Luz.",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| {
                    objects(ws).iter().any(|o| {
                        // (A brasa da tocha também brilha, mas é um cone.)
                        o.component("shape3d")
                            .is_some_and(|c| c.str("shape") == "sphere" && c.num("glow") >= 0.5)
                    })
                }),
            },
            Step {
                title: "Um holofote",
                text: "➕ Objeto > Luz > Holofote. Ele ilumina num cone, para a frente do objeto (a seta azul). Com ⟲ Girar, aponte para o cristal ou para a esfera.",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| lights(ws).iter().any(|c| c.str("kind") == "spot")),
            },
            Step {
                title: "Alguém para ver",
                text: "➕ Objeto > Jogador 3D, para andar pela caverna.",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| objects(ws).iter().any(|o| has_behavior(o, "nn:jogador3d"))),
            },
            Step {
                title: "Acender e apagar",
                text: "Selecione o Holofote e, no painel Eventos: ➕ Regra de Holofote > Controles > Tecla ou botão, com a tecla F. Em Então: ➕ ação > Aparência > Acender ou apagar luz, com \"acender ou apagar (o contrário)\".",
                target: Some("eventos.regra_cena"),
                show: Some(Tab::Events),
                done: Some(|ws, _| {
                    all_rules(ws).iter().any(|r| {
                        r.when.iter().any(|c| c.id == "tecla")
                            && r.actions
                                .iter()
                                .any(|a| a.id == "luz" && a.str("acao") == "alternar")
                    })
                }),
            },
            Step {
                title: "Teste!",
                text: "F6 (▶ Testar daqui). Ande perto da tocha e do cristal, e aperte F para apagar e acender o holofote.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Pronto: uma caverna iluminada!",
                text: "Você usou o ambiente Caverna, a Luz (tocha e holofote), o Brilho e a ação de acender e apagar. O Vale 3D (Novo projeto) tem uma caverna inteira, com o herói levando uma tocha na mão (Preso a um osso, mao_e). Para uma noite, experimente o ambiente pronto Noite.",
                target: None,
                show: None,
                done: None,
            },
        ],
    }
}

fn dia_e_noite() -> Tutorial {
    Tutorial {
        id: "dia-e-noite",
        title: "Dia e noite",
        summary: "O sol andando no céu, a noite com a lua e as estrelas, um poste que acende sozinho, uma janela que brilha à noite, o relógio no placar e uma regra para o anoitecer.",
        minutes: 10,
        template: Template::Vazio3D,
        project: "Dia e noite",
        steps: vec![
            Step {
                title: "O tempo passando",
                text: "Com Dia e noite, a hora anda: o sol nasce, cruza o céu e se põe, e de noite vêm a lua e as estrelas. As luzes podem acender sozinhas ao anoitecer.",
                target: Some("cena"),
                show: None,
                done: None,
            },
            Step {
                title: "Ligue o Dia e noite",
                text: "Clique num lugar vazio da cena e, no Inspetor, em ☀ Ambiente, marque Dia e noite. Ponha um dia de 2 minutos (no jogo, a noite chega rápido).",
                target: Some("inspetor:ambiente:ciclo"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    doc_of(ws).is_some_and(|d| {
                        d.data.env("cycle").bool() && d.data.env("day_length").num() <= 3.0
                    })
                }),
            },
            Step {
                title: "Veja a noite",
                text: "Em Começa às, arraste para umas 21:00. A vista mostra a cena nessa hora: escura, com a lua e as estrelas. (O dia, com as cores de sempre, é o desta cena.)",
                target: Some("inspetor:ambiente:hora"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    doc_of(ws).is_some_and(|d| {
                        let h = d.data.env("start_hour").num();
                        !(5.0..19.0).contains(&h)
                    })
                }),
            },
            Step {
                title: "Um poste",
                text: "➕ Objeto > Luz > Lâmpada. No Inspetor da Luz, marque Só à noite: ela acende sozinha ao anoitecer e apaga ao amanhecer.",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| lights(ws).iter().any(|c| c.bool("night"))),
            },
            Step {
                title: "Uma janela que acende",
                text: "➕ Objeto > Cubo, achatado como uma janela (Escala). No Inspetor da Forma 3D, ponha Brilho = 1 e marque Brilha só à noite.",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| {
                    objects(ws).iter().any(|o| {
                        o.component("shape3d")
                            .is_some_and(|c| c.num("glow") >= 0.5 && c.bool("glow_night"))
                    })
                }),
            },
            Step {
                title: "Alguém para ver",
                text: "➕ Objeto > Jogador 3D.",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| objects(ws).iter().any(|o| has_behavior(o, "nn:jogador3d"))),
            },
            Step {
                title: "O relógio",
                text: "➕ Objeto > Na tela (HUD) > Texto, no canto de cima, e adicione Aparência > Mostrar valor com o texto {relogio}. A variável relogio tem a hora do jogo (\"21:30\"); hora é só o número (21) e dia conta os dias.",
                target: Some("cena.objeto"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    objects(ws).iter().any(|o| {
                        behavior_of(o, "nn:mostrar_valor")
                            .is_some_and(|c| c.str("texto").contains("{relogio}"))
                    })
                }),
            },
            Step {
                title: "Quando anoitecer",
                text: "No painel Eventos, uma regra da cena: Quando Cena e tempo > Amanhecer ou anoitecer (anoitecer), Então o que quiser: Tocar som, Mostrar um texto, Criar um prefab de inimigo...",
                target: Some("eventos"),
                show: Some(Tab::Events),
                done: Some(|ws, _| {
                    all_rules(ws).iter().any(|r| {
                        r.when.iter().any(|c| c.id == "amanhecer") && !r.actions.is_empty()
                    })
                }),
            },
            Step {
                title: "Teste!",
                text: "F6. Ponha Começa às perto das 17:00 para ver logo o pôr do sol, o poste acendendo e a janela brilhando.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Pronto: o dia passa!",
                text: "Você usou o Dia e noite, a Luz e o Brilho só à noite, a variável relogio e o evento de anoitecer. Também dá para Mudar a hora (dormir numa cama), Parar o tempo e Mudar o ambiente (uma tempestade) nas ações. A ilha do modelo 3D tem postes no caminho e o pescador com um lampião de noite.",
                target: None,
                show: None,
                done: None,
            },
        ],
    }
}

/// Modelos `file` que balançam com o vento.
fn swaying<'a>(ws: &'a ProjectWorkspace, file: &str) -> Vec<&'a Object> {
    with_model(ws, file)
        .into_iter()
        .filter(|o| o.component("model").is_some_and(|c| c.num("sway") >= 0.5))
        .collect()
}

fn um_lago() -> Tutorial {
    Tutorial {
        id: "um-lago-com-vida",
        title: "Um lago com vida",
        summary: "A beira de um lago: água com ondinhas que refletem o céu, árvores e grama balançando com o vento, vaga-lumes levados pela brisa e o halo do brilho num fim de tarde.",
        minutes: 10,
        template: Template::Vazio3D,
        project: "Um lago",
        steps: vec![
            Step {
                title: "Um lugar vivo",
                text: "Um mundo parece vivo quando as coisas se mexem sozinhas: a água ondula e reflete o céu, o vento balança as árvores e a grama, e o que brilha ganha um halo.\n\nVamos fazer a beira de um lago.",
                target: Some("cena"),
                show: None,
                done: None,
            },
            Step {
                title: "O lago",
                text: "➕ Objeto > Água (lago). É um Plano azul com o Material Água (no Inspetor da Forma 3D): ondinhas, o céu refletido (mais forte olhando de lado) e o brilho do sol. Aumente com a Escala, se quiser.",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| {
                    objects(ws).iter().any(|o| {
                        o.component("shape3d")
                            .is_some_and(|c| c.str("material") == "agua")
                    })
                }),
            },
            Step {
                title: "O vento",
                text: "Clique num lugar vazio da cena e, no Inspetor, em ☀ Ambiente, ponha Vento = 1 (uma brisa; 3 é uma ventania). O lado diz para onde ele sopra.",
                target: Some("inspetor:ambiente:vento"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| doc_of(ws).is_some_and(|d| d.data.env("wind").num() >= 0.5)),
            },
            Step {
                title: "Uma árvore",
                text: "No painel Arquivos, abra modelos e arraste arvore.glb para a beira do lago.",
                target: Some("arquivo:modelos/arvore.glb"),
                show: Some(Tab::Files),
                done: Some(|ws, _| !with_model(ws, "arvore.glb").is_empty()),
            },
            Step {
                title: "Balançando",
                text: "Com a árvore escolhida, no Inspetor, no Modelo 3D, ponha Balança com o vento = 1: o pé fica parado e a copa balança, com rajadas. Ctrl+D duplica a árvore (com o balanço): faça mais umas.",
                target: Some("inspetor"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| !swaying(ws, "arvore.glb").is_empty()),
            },
            Step {
                title: "Grama",
                text: "Arraste modelos/grama.glb para perto da água e ponha Balança com o vento = 1,5 (a grama é leve). Com Ctrl+D e a seta, espalhe uns 3 tufos.",
                target: Some("arquivo:modelos/grama.glb"),
                show: Some(Tab::Files),
                done: Some(|ws, _| swaying(ws, "grama.glb").len() >= 3),
            },
            Step {
                title: "Vaga-lumes",
                text: "➕ Objeto > Partículas > Brilho, em cima do lago (suba 1 m com a seta verde). O vento também leva as partículas: no Inspetor, Levadas pelo vento diz quanto (0 = nada).",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| !with_particles(ws, "").is_empty()),
            },
            Step {
                title: "Fim de tarde",
                text: "Clique num lugar vazio e, em ☀ Ambiente > Pronto, escolha Entardecer: o lago reflete o céu laranja. (O vento fica como estava.)",
                target: Some("inspetor:ambiente:entardecer"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    doc_of(ws).is_some_and(|d| {
                        d.data
                            .env("sky_horizon")
                            .str()
                            .eq_ignore_ascii_case("#F7A35C")
                    })
                }),
            },
            Step {
                title: "O halo",
                text: "O que brilha (os vaga-lumes, o sol, uma forma com Brilho) tem um halo suave em volta. Em ☀ Ambiente, mude o Halo do brilho: 2 é mais forte, 0 tira.",
                target: Some("inspetor:ambiente:halo"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    doc_of(ws).is_some_and(|d| (d.data.env("bloom").num() - 1.0).abs() >= 0.2)
                }),
            },
            Step {
                title: "Alguém para ver",
                text: "➕ Objeto > Jogador 3D, para passear na beira do lago.",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| objects(ws).iter().any(|o| has_behavior(o, "nn:jogador3d"))),
            },
            Step {
                title: "Teste!",
                text: "F6. Olhe a água de lado (o reflexo fica mais forte) e na direção do sol (o brilho dele na água), e veja a grama e as árvores balançando.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Pronto: um lago com vida!",
                text: "Você usou a Água, o Vento, o Balança com o vento, as partículas no vento e o Halo do brilho. Num Terreno, a Água também tem ondas. A ilha do modelo 3D tem o mar, grama e uma bandeira na arena.",
                target: None,
                show: None,
                done: None,
            },
        ],
    }
}

/// As portas (comportamento Porta 3D) da cena.
fn doors(ws: &ProjectWorkspace) -> Vec<&Object> {
    objects(ws)
        .into_iter()
        .filter(|o| has_behavior(o, "nn:porta3d"))
        .collect()
}

fn portas_e_chaves() -> Tutorial {
    Tutorial {
        id: "portas-e-chaves",
        title: "Portas e chaves",
        summary: "Uma porta que abre com E, trancada até você achar a chave; o inventário na tela, uma placa de Texto 3D e uma zona que fecha a porta atrás de você.",
        minutes: 10,
        template: Template::Vazio3D,
        project: "Portas e chaves",
        steps: vec![
            Step {
                title: "Portas, chaves e zonas",
                text: "Neste tutorial: uma porta que abre com a tecla E, trancada até o herói achar a chave, os itens na tela e uma zona que fecha a porta quando você passa. Tudo sem código.",
                target: Some("cena"),
                show: None,
                done: None,
            },
            Step {
                title: "O herói",
                text: "➕ Objeto > Jogador 3D (ou arraste modelos/heroi.glb para a cena e adicione Jogador > Jogador 3D).",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| objects(ws).iter().any(|o| has_behavior(o, "nn:jogador3d"))),
            },
            Step {
                title: "Uma porta",
                text: "➕ Objeto > Portas e itens > Porta. Ela já vem com o comportamento Porta 3D e um Corpo 3D que se move: fechada, ninguém passa. (Troque a forma por modelos/porta.glb no Inspetor, se quiser.)",
                target: Some("cena.objeto"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| !doors(ws).is_empty()),
            },
            Step {
                title: "Teste a porta",
                text: "F6. Chegue perto da porta: aparece \"E abrir\". Aperte E: ela gira para longe de você. E de novo fecha.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Tranque",
                text: "Selecione a porta. No Inspetor, em Porta 3D > Precisa do item, escreva chave. Agora ela só abre para quem tiver uma chave no inventário.",
                target: Some("inspetor"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    doors(ws).iter().any(|o| {
                        behavior_of(o, "nn:porta3d").is_some_and(|c| c.str("chave") == "chave")
                    })
                }),
            },
            Step {
                title: "A chave",
                text: "➕ Objeto > Portas e itens > Chave, longe da porta. Ela é um Coletável com Guardar no inventário = chave: some quando o herói encosta e vai para o inventário.",
                target: Some("cena.objeto"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    objects(ws).iter().any(|o| {
                        behavior_of(o, "nn:coletavel").is_some_and(|c| c.str("item") == "chave")
                    })
                }),
            },
            Step {
                title: "Os itens na tela",
                text: "➕ Objeto > Portas e itens > Mostrar inventário: os itens que o herói carrega aparecem num canto da tela. Na chave, em Ícone do item, escolha assets/imagens/chave.png.",
                target: Some("cena.objeto"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| objects(ws).iter().any(|o| has_behavior(o, "nn:inventario"))),
            },
            Step {
                title: "Teste a chave",
                text: "F6: perto da porta, o aviso diz \"Precisa de: chave\". Pegue a chave (ela aparece no canto) e volte: agora E abre, e a chave é gasta.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Uma placa",
                text: "➕ Objeto > Portas e itens > Texto 3D. No Inspetor, escreva \"Cuidado!\" e ponha a placa em cima da porta. Em Virado para, \"A frente do objeto\" a prende como uma placa de verdade.",
                target: Some("cena.objeto"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| objects(ws).iter().any(|o| o.component("text3d").is_some())),
            },
            Step {
                title: "Uma armadilha",
                text: "➕ Objeto > Portas e itens > Zona de gatilho, do outro lado da porta. Em Avisa o objeto, escolha a porta e, em Mensagem ao entrar, escreva fechar: quem passa fica preso! A zona aparece no editor, mas não no jogo.",
                target: Some("cena.objeto"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    let names: Vec<&str> = doors(ws).iter().map(|o| o.name.as_str()).collect();
                    objects(ws).iter().any(|o| {
                        behavior_of(o, "nn:gatilho").is_some_and(|c| {
                            names.contains(&c.str("alvo")) && c.str("mensagem") == "fechar"
                        })
                    })
                }),
            },
            Step {
                title: "Teste de novo",
                text: "F6: pegue a chave, abra a porta e passe por ela. Quando entrar na zona, a porta fecha atrás de você (e abre com E de novo: ela já está destrancada).",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Pronto: uma fase com segredo!",
                text: "Você usou Porta 3D, Coletável com inventário, Mostrar inventário, Texto 3D e Zona de gatilho. Ideias: Fases > Interagir numa placa (\"E ler\") com um evento que dá um item (Itens > Ganhar item), a regra Itens > Ter um item, ou uma porta que abre sozinha quando o herói chega (Abre com: o jogador chegando perto).",
                target: None,
                show: None,
                done: None,
            },
        ],
    }
}

/// Os diálogos abertos no editor de conversa.
fn dialogues(ws: &ProjectWorkspace) -> Vec<&crate::dialogue::DialogueDoc> {
    let mut v: Vec<_> = ws.dialogues.values().collect();
    v.sort_by(|a, b| a.rel.cmp(&b.rel));
    v
}

fn uma_conversa() -> Tutorial {
    Tutorial {
        id: "uma-conversa",
        title: "Uma conversa",
        summary: "Monte uma conversa com falas, escolhas e personagens no editor de conversa, ponha um personagem que fala na cena e faça um evento para a escolha do jogador.",
        minutes: 12,
        template: Template::Vazio3D,
        project: "Uma conversa",
        steps: vec![
            Step {
                title: "Conversas",
                text: "Personagens que falam deixam o jogo vivo: dão dicas, pedem ajuda, dão itens. Neste tutorial você monta uma conversa no editor de conversa e põe quem fala na cena. Funciona igual no 2D.",
                target: Some("cena"),
                show: None,
                done: None,
            },
            Step {
                title: "Um diálogo novo",
                text: "No painel Arquivos: ➕ Novo > 💬 Diálogo (conversa), e Criar. Ele abre no editor de conversa, já com um exemplo.",
                target: Some("arquivos.novo"),
                show: Some(Tab::Files),
                done: Some(|ws, _| !dialogues(ws).is_empty()),
            },
            Step {
                title: "As falas",
                text: "Cada cartão é uma fala: quem fala, o texto e, na bolinha da direita, para onde a conversa vai depois. A fala com a borda verde é o início. Clique nela e mude o texto no painel da direita.",
                target: Some("dialogo.tela"),
                show: None,
                done: Some(|ws, _| {
                    dialogues(ws).iter().any(|d| {
                        d.data
                            .node(d.data.start)
                            .is_some_and(|n| n.text != "Olá! Que bom te ver por aqui.")
                    })
                }),
            },
            Step {
                title: "Uma fala nova",
                text: "Arraste da bolinha da fala 3 (a última) para um espaço vazio e solte: nasce uma fala nova, já ligada. Escreva o texto dela.",
                target: Some("dialogo.saida:3"),
                show: None,
                done: Some(|ws, _| {
                    dialogues(ws).iter().any(|d| {
                        d.data.nodes.len() >= 4 && d.data.nodes.iter().any(|n| n.next >= 4)
                    })
                }),
            },
            Step {
                title: "Escolhas",
                text: "Com a fala nova escolhida, clique em ➕ Escolha duas vezes: o jogador escolhe o que responder. Escreva as respostas e, em \"Vai para\", para onde cada uma leva (ou o fim).",
                target: Some("dialogo.escolha"),
                show: None,
                done: Some(|ws, _| {
                    dialogues(ws)
                        .iter()
                        .any(|d| d.data.nodes.iter().filter(|n| n.choices.len() >= 2).count() >= 2)
                }),
            },
            Step {
                title: "Um personagem",
                text: "Clique num espaço vazio da tela: o painel mostra a conversa. ➕ Personagem, dê um nome e uma cor. Depois escolha uma fala e, em Quem fala, o personagem novo.",
                target: Some("dialogo.personagem"),
                show: None,
                done: Some(|ws, _| {
                    dialogues(ws).iter().any(|d| {
                        d.data.characters.len() >= 2
                            && d.data.nodes.iter().any(|n| {
                                !n.who.is_empty()
                                    && n.who != "Ana"
                                    && d.data.character(&n.who).is_some()
                            })
                    })
                }),
            },
            Step {
                title: "Salve",
                text: "Ctrl+S salva a conversa (a aba perde o •).",
                target: Some("dialogo.tela"),
                show: None,
                done: Some(|ws, _| {
                    let d = dialogues(ws);
                    !d.is_empty()
                        && d.iter().all(|d| !d.is_dirty())
                        && d.iter().any(|d| d.data.nodes.len() >= 4)
                }),
            },
            Step {
                title: "Na cena",
                text: "Abra cenas/mundo.cena (painel Arquivos) e ponha um herói: ➕ Objeto > Jogador 3D.",
                target: Some("arquivo:cenas/mundo.cena"),
                show: Some(Tab::Files),
                done: Some(|ws, _| objects(ws).iter().any(|o| has_behavior(o, "nn:jogador3d"))),
            },
            Step {
                title: "Quem fala",
                text: "➕ Objeto > Portas e itens > Personagem que fala, perto do herói. No Inspetor, em Diálogo > Conversa, escolha o seu diálogo. (O botão 💬 Editar a conversa volta para o editor.)",
                target: Some("cena.objeto"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    objects(ws).iter().any(|o| {
                        behavior_of(o, "nn:dialogo").is_some_and(|c| !c.str("arquivo").is_empty())
                    })
                }),
            },
            Step {
                title: "Converse!",
                text: "F6. Chegue perto do personagem: \"E falar\". E (ou Enter) passa as falas; as setas e os números escolhem. Enquanto a conversa está aberta, o herói fica parado.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "A escolha faz algo",
                text: "Feche o jogo. No painel Eventos: ➕ Regra da cena > Conversas > Escolher numa conversa, e escreva o texto de uma escolha (igual ao do editor). Em Então, por exemplo, Itens > Ganhar item ou Aparência > Mudar cor.",
                target: Some("eventos.regra_cena"),
                show: Some(Tab::Events),
                done: Some(|ws, _| {
                    all_rules(ws)
                        .iter()
                        .any(|r| r.when.iter().any(|c| c.id == "escolheu") && !r.actions.is_empty())
                }),
            },
            Step {
                title: "Teste de novo",
                text: "F6 e escolha a resposta do evento: ele acontece na hora. A conversa também pode mudar variáveis e dar itens sozinha: no editor, abra \"Ao aparecer\" numa fala, ou \"Mais\" numa escolha.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Pronto: alguém para conversar!",
                text: "Você usou o editor de conversa, personagens, escolhas, o comportamento Diálogo e o evento Escolher numa conversa. Ideias: uma escolha que só aparece com um item (Mais > Só aparece com o item), um retrato para o personagem, ou a conversa começando sozinha quando a cena começa.",
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

/// A imagem criada no tutorial (a última feita com "Nova imagem").
fn drawing(ws: &ProjectWorkspace) -> Option<&crate::pixel_editor::PixelDoc> {
    let rel = ws.created_images.last()?;
    ws.pixels.get(rel)
}

fn painted(doc: &crate::pixel_editor::PixelDoc) -> usize {
    doc.img.pixels().filter(|p| p.0[3] > 0).count()
}

fn desenhe() -> Tutorial {
    Tutorial {
        id: "desenhe-seu-heroi",
        title: "Desenhe seu herói",
        summary: "Desenhe um personagem pixel a pixel no editor de pixel art, faça ele andar com dois quadros e ponha na cena.",
        minutes: 10,
        template: Template::Vazio,
        project: "Desenhe seu heroi",
        steps: vec![
            Step {
                title: "Pixel art",
                text: "Neste tutorial você desenha um personagem no editor de pixel art do NN Studio, anima ele com dois quadros e põe na cena.\n\nPixel art é desenhar ponto por ponto, em imagens pequenas: 16 × 16 pixels já dão um herói.",
                target: None,
                show: None,
                done: None,
            },
            Step {
                title: "Uma imagem nova",
                text: "No painel Arquivos, clique em ➕ Novo > 🎨 Imagem (pixel art). Chame de meu_heroi, deixe o tamanho 16 × 16 e 1 quadro, e clique em Criar.",
                target: Some("arquivos.novo"),
                show: Some(Tab::Files),
                done: Some(|ws, _| drawing(ws).is_some()),
            },
            Step {
                title: "Desenhe!",
                text: "Esta é a aba da imagem. Com o Lápis, segure o botão esquerdo e desenhe na tela do meio: uma cabeça, um corpo e duas pernas. O botão direito pinta com a cor secundária (transparente: apaga).\n\nDica: role o mouse para aumentar e marque Espelhado (à direita) para desenhar os dois lados de uma vez.",
                target: Some("pixel:tela"),
                show: None,
                done: Some(|ws, _| drawing(ws).is_some_and(|d| painted(d) >= 25)),
            },
            Step {
                title: "Cores",
                text: "Clique nas cores da paleta (à direita) para trocar a cor do lápis: pele, roupa, cabelo. O Balde (G) pinta de uma vez uma área fechada. Use pelo menos três cores.",
                target: Some("pixel:paleta"),
                show: None,
                done: Some(|ws, _| {
                    drawing(ws).is_some_and(|d| crate::pixel_ops::used_colors(&d.img, 8).len() >= 3)
                }),
            },
            Step {
                title: "Um segundo quadro",
                text: "Uma animação é uma sequência de desenhos. Embaixo, no painel Quadros, clique em Duplicar: o quadro 2 começa igual ao 1.",
                target: Some("pixel:duplicar"),
                show: None,
                done: Some(|ws, _| drawing(ws).is_some_and(|d| d.frame_count() >= 2)),
            },
            Step {
                title: "O passo",
                text: "Agora você está no quadro 2. Mude as pernas: apague com a Borracha (E) e redesenhe uma perna mais para a frente. Marque Papel vegetal para ver o quadro 1 clarinho por baixo.",
                target: Some("pixel:papel"),
                show: None,
                done: Some(|ws, _| {
                    drawing(ws).is_some_and(|d| {
                        if d.frame_count() < 2 {
                            return false;
                        }
                        let a = crate::pixel_ops::crop(&d.img, d.cell(0));
                        let b = crate::pixel_ops::crop(&d.img, d.cell(1));
                        a.pixels().zip(b.pixels()).filter(|(x, y)| x != y).count() >= 2
                    })
                }),
            },
            Step {
                title: "Animar",
                text: "Clique em Animações (embaixo) > + Nova animação: ela já usa os dois quadros e toca na Prévia, à direita. Mude o nome para andar e, se quiser, o Ritmo (quadros por segundo).",
                target: Some("pixel:aba-animacoes"),
                show: None,
                done: Some(|ws, _| {
                    drawing(ws).is_some_and(|d| {
                        d.anim
                            .anims
                            .iter()
                            .any(|a| a.name == "andar" && a.frames.len() >= 2)
                    })
                }),
            },
            Step {
                title: "Salvar",
                text: "Clique em Salvar (ou Ctrl+S). A imagem vira um PNG comum e as animações ficam em meu_heroi.anim, ao lado dela.",
                target: Some("pixel:salvar"),
                show: None,
                done: Some(|ws, _| {
                    drawing(ws).is_some_and(|d| !d.is_dirty() && !d.anim.anims.is_empty())
                }),
            },
            Step {
                title: "Na cena",
                text: "Volte para a aba fase.cena e arraste meu_heroi.png do painel Arquivos para a cena.",
                target: Some("arquivo:assets/imagens/meu_heroi.png"),
                show: Some(Tab::Files),
                done: Some(|ws, _| {
                    ws.created_images.last().is_some_and(|rel| {
                        !with_image(ws, rel.rsplit('/').next().unwrap_or(rel)).is_empty()
                    })
                }),
            },
            Step {
                title: "Andando",
                text: "Com ele selecionado, no Inspetor, em Imagem > Animação, clique no ☰ e escolha andar. Ele já anda ali mesmo, na cena!",
                target: Some("inspetor.animacao"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    let Some(rel) = ws.created_images.last() else {
                        return false;
                    };
                    let name = rel.rsplit('/').next().unwrap_or(rel);
                    with_image(ws, name).iter().any(|o| {
                        o.component("sprite")
                            .is_some_and(|c| c.str("animation") == "andar")
                    })
                }),
            },
            Step {
                title: "Teste!",
                text: "Aperte ▶ Testar daqui (F6): seu herói aparece no jogo, andando. Com o jogo aberto, mude um pixel no editor e salve: o jogo mostra o desenho novo na hora.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Pronto: um herói seu!",
                text: "Próximos passos: desenhe animações parado, andar e pular e escolha os nomes na Animação automática (Aparência), que troca sozinha conforme o herói anda ou pula. Os eventos também têm Tocar animação. Para montar a fase, faça o tutorial Seu primeiro jogo.",
                target: None,
                show: None,
                done: None,
            },
        ],
    }
}

/// O mapa do primeiro objeto com Mapa da cena (aberto para pintar).
fn scene_map(ws: &ProjectWorkspace) -> Option<&crate::map_doc::MapDoc> {
    objects(ws)
        .into_iter()
        .filter_map(|o| o.component("map"))
        .find_map(|c| ws.map_docs.get(c.str("map")))
}

/// Quantas células do mapa (em qualquer camada) têm um tile assim.
fn count_tiles(ws: &ProjectWorkspace, f: impl Fn(&crate::tileset::TileInfo) -> bool) -> usize {
    let Some(map) = scene_map(ws) else {
        return 0;
    };
    let mut n = 0;
    for li in map.tile_layers() {
        for &gid in &map.tile_layer(li).unwrap().data {
            let gid = gid & !crate::map_doc::FLIP_BITS;
            if gid != 0 && f(&crate::tiles_view::tile_info(map, &ws.tilesets, gid)) {
                n += 1;
            }
        }
    }
    n
}

fn tiles() -> Tutorial {
    Tutorial {
        id: "fase-com-tiles",
        title: "Monte uma fase com tiles",
        summary: "Pinte o chão com um terreno que se encaixa sozinho, ponha plataformas, enfeites numa camada e lava que queima, com um evento.",
        minutes: 12,
        template: Template::Vazio,
        project: "Fase com tiles",
        steps: vec![
            Step {
                title: "Tiles",
                text: "Uma fase de tiles é montada com quadradinhos (tiles) pintados numa grade, como num jogo de montar. Os tiles vêm de uma imagem só (aqui, assets/imagens/terreno.png), e o mapa guarda qual tile vai em cada casa.\n\nNeste tutorial você pinta uma fase inteira, sem posicionar bloco por bloco.",
                target: None,
                show: None,
                done: None,
            },
            Step {
                title: "Um mapa novo",
                text: "Na barra da cena, clique em ➕ Objeto > Mapa de tiles... Os tiles já vêm do terreno.tsj: deixe como está e clique em Criar. O modo ✏ Pintar liga sozinho e o painel Tiles abre à direita.",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| scene_map(ws).is_some()),
            },
            Step {
                title: "O chão",
                text: "No painel Tiles, em Terrenos, clique em terra. Agora segure o botão esquerdo e pinte uma faixa larga na parte de baixo da cena, com duas ou três linhas de altura. As bordas e a grama se encaixam sozinhas!\n\nErrou? Ctrl+Z desfaz; apagar (ao lado de terra) tira o terreno.",
                target: Some("tiles.terreno:terra"),
                show: Some(Tab::Tiles),
                done: Some(|ws, _| count_tiles(ws, |t| t.solid) >= 15),
            },
            Step {
                title: "Uma plataforma",
                text: "Em Tileset, clique na tábua (o último tile da segunda linha) e pinte uma linha no ar, acima do chão. A tábua é uma plataforma de mão única: dá para pular por baixo dela e parar em cima.\n\nDica: role o mouse na cena para afastar ou aproximar; Espaço + arrastar move a vista.",
                target: Some("tiles.tileset"),
                show: Some(Tab::Tiles),
                done: Some(|ws, _| count_tiles(ws, |t| t.one_way) >= 3),
            },
            Step {
                title: "Enfeites numa camada",
                text: "No fim do painel Tiles, em Camadas, clique em + Camada. Escolha no tileset a flor ou o mato e pinte alguns em cima da grama.\n\nNuma camada separada, os enfeites ficam por cima do chão sem trocar os tiles dele (e a borracha só apaga a camada escolhida).",
                target: Some("tiles.camada"),
                show: Some(Tab::Tiles),
                done: Some(|ws, _| {
                    scene_map(ws).is_some_and(|m| {
                        m.tile_layers().iter().skip(1).any(|&i| {
                            m.tile_layer(i)
                                .unwrap()
                                .data
                                .iter()
                                .filter(|g| **g != 0)
                                .count()
                                >= 2
                        })
                    })
                }),
            },
            Step {
                title: "Lava",
                text: "No alto do painel Tiles, em Camada, volte para chao. Escolha a lava no tileset (o tile laranja da quinta linha) e pinte uma poça no chão, trocando um pedaço da terra. Ela já se mexe: é um tile animado.",
                target: Some("tiles.tileset"),
                show: Some(Tab::Tiles),
                done: Some(|ws, _| count_tiles(ws, |t| t.animation.len() > 1) >= 2),
            },
            Step {
                title: "Marque a lava",
                text: "O jogo ainda não sabe que aquilo é lava. Em Tileset, clique em Tipo (acima dos tiles), escreva lava no campo Tipo e clique no tile de lava que você usou: ele ganha o nome lava.\n\nDo mesmo jeito, Sólido marca o que bloqueia e Plataforma marca o que é de mão única.",
                target: Some("tiles.tileset"),
                show: Some(Tab::Tiles),
                done: Some(|ws, _| count_tiles(ws, |t| t.kind == "lava") >= 1),
            },
            Step {
                title: "O herói",
                text: "Aperte Esc (com o mouse na cena) para sair do modo Pintar. Arraste heroi.png, em assets > imagens, para a cena, acima do chão.",
                target: Some("arquivo:assets/imagens/heroi.png"),
                show: Some(Tab::Files),
                done: Some(|ws, _| !with_image(ws, "heroi.png").is_empty()),
            },
            Step {
                title: "Andar e pular",
                text: "Com o herói selecionado, no Inspetor: ➕ Adicionar componente > Jogador > Jogador de plataforma. O corpo físico vem junto, e os tiles sólidos do mapa seguram ele.",
                target: Some("inspetor.adicionar"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    with_image(ws, "heroi.png")
                        .iter()
                        .any(|o| has_behavior(o, "nn:plataforma"))
                }),
            },
            Step {
                title: "Lava queima",
                text: "No painel Eventos: ➕ Regra da cena > Toques > Encostar num tile. Deixe o jogador e o tipo lava. Em Então, ➕ ação > Cenas > Reiniciar a cena.",
                target: Some("eventos.regra_cena"),
                show: Some(Tab::Events),
                done: Some(|ws, _| {
                    scene_rules(ws).iter().any(|r| {
                        r.when
                            .iter()
                            .any(|c| c.id == "tile" && c.str("tipo") == "lava")
                            && !r.actions.is_empty()
                    })
                }),
            },
            Step {
                title: "Teste!",
                text: "Aperte ▶ Testar daqui (F6): ande pelo chão, suba na tábua e pule na lava. Com o jogo aberto, volte ao ✏ Pintar e pinte: os tiles aparecem no jogo na hora.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Pronto: uma fase de tiles!",
                text: "Testar salvou tudo: a cena e o mapa, em mapas/fase.tmj (no formato do Tiled; dá para abrir nele também). Ctrl+S salva os dois juntos.\n\nPróximos passos: no painel Tiles, + Terreno cria um terreno seu com um molde de 16 peças para desenhar no editor de pixel art. Ctrl + arrastar na cena copia um pedaço do mapa para o pincel; R gira e X espelha. O evento Trocar o tile tocado faz blocos que quebram.",
                target: None,
                show: None,
                done: None,
            },
        ],
    }
}

fn all_objects(doc: &crate::scene_view::SceneDoc) -> Vec<&Object> {
    fn walk<'a>(list: &'a [Object], out: &mut Vec<&'a Object>) {
        for o in list {
            out.push(o);
            walk(&o.children, out);
        }
    }
    let mut out = Vec::new();
    walk(&doc.data.objects, &mut out);
    out
}

/// O objeto da cena aberta com os corações (componente Ícones).
fn hearts(ws: &ProjectWorkspace) -> Option<&Object> {
    objects(ws)
        .into_iter()
        .find(|o| o.component("icons").is_some())
}

/// A cena da pausa (a primeira cena aberta que abre por cima).
fn pause_scene(ws: &ProjectWorkspace) -> Option<&crate::scene_view::SceneDoc> {
    ws.scenes.values().find(|d| d.data.overlay)
}

fn menu() -> Tutorial {
    Tutorial {
        id: "monte-um-menu",
        title: "Monte um menu",
        summary: "Corações no HUD, presos no canto da tela, e um menu de pausa que abre por cima do jogo com Esc, com botões e um volume.",
        minutes: 10,
        template: Template::Vazio,
        project: "Menu e HUD",
        steps: vec![
            Step {
                title: "Interface",
                text: "Menus e HUD (vidas, placar, botões) são montados com os componentes de Interface: Painel, Botão, Barra, Ícones, Deslizador, Caixa de marcar e Campo de texto.\n\nOs botões funcionam com o mouse, com as setas e com o controle, sem nenhum código. Neste tutorial você põe corações no HUD e faz um menu de pausa.",
                target: None,
                show: None,
                done: None,
            },
            Step {
                title: "Corações",
                text: "Na barra da cena, clique em ➕ Objeto > Interface > Ícones. Os Ícones mostram uma imagem para cada ponto de uma variável: um coração para cada vida.",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| hearts(ws).is_some()),
            },
            Step {
                title: "Imagem e variável",
                text: "No Inspetor, em Ícones: arraste coracao.png (assets > imagens) para o campo Imagem. Em Variável, clique no … e escolha vidas: o jogo começa com 3 vidas (Configurações do projeto). Mude Máximo para 3: quando perder uma vida, o coração que falta fica apagado.",
                target: Some("inspetor:componente:icons"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    hearts(ws)
                        .and_then(|o| o.component("icons"))
                        .is_some_and(|c| {
                            c.str("image").ends_with("coracao.png") && c.str("var") == "vidas"
                        })
                }),
            },
            Step {
                title: "Preso no canto",
                text: "Em Transformação, Preso em mostra a tela com 9 pontos. Clique no ponto de cima à esquerda: os corações vão para esse canto, e a posição passa a contar a partir dele. Assim eles ficam no canto em qualquer tamanho de janela.\n\nDá para arrastar na cena para ajustar. O 📐 na barra da cena mostra como fica em telas de outros formatos.",
                target: Some("inspetor:ancora"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| hearts(ws).is_some_and(|o| o.anchor == "top_left")),
            },
            Step {
                title: "A cena da pausa",
                text: "No painel Arquivos, clique em ➕ Novo > 🖼 Cena visual. Chame de pausa, marque \"Abre por cima de outra cena\" e clique em Criar.\n\nEla já vem com um painel, um título e dois botões: Continuar (fecha a pausa) e Sair do jogo. A fase continua aparecendo embaixo, parada.",
                target: Some("arquivos.novo"),
                show: Some(Tab::Files),
                done: Some(|ws, _| pause_scene(ws).is_some()),
            },
            Step {
                title: "Um volume",
                text: "Na cena da pausa: ➕ Objeto > Interface > Deslizador. No Inspetor, em Muda, escolha Volume da música.\n\nNa Hierarquia, arraste o deslizador para dentro de botoes: botoes é um Grupo, que arruma os filhos em coluna sozinho.",
                target: Some("cena.objeto"),
                show: Some(Tab::Hierarchy),
                done: Some(|ws, _| {
                    pause_scene(ws).is_some_and(|d| {
                        all_objects(d).iter().any(|o| {
                            o.component("layout").is_some()
                                && o.children.iter().any(|c| {
                                    c.component("slider")
                                        .is_some_and(|s| s.str("target") != "variavel")
                                })
                        })
                    })
                }),
            },
            Step {
                title: "Esc abre a pausa",
                text: "Volte para a aba fase.cena. No painel Eventos: ➕ Regra da cena > Controles > Tecla ou botão, e escolha a tecla voltar (Esc, ou o B do controle). Em Então: ➕ ação > Cenas > Abrir cena por cima, e escolha cenas/pausa.cena.",
                target: Some("eventos.regra_cena"),
                show: Some(Tab::Events),
                done: Some(|ws, _| {
                    ws.scenes.values().filter(|d| !d.data.overlay).any(|d| {
                        d.data.events.iter().any(|r| {
                            r.when.iter().any(|c| c.id == "tecla")
                                && r.actions.iter().any(|a| {
                                    a.id == "por_cima" && a.str("cena").ends_with("pausa.cena")
                                })
                        })
                    })
                }),
            },
            Step {
                title: "Teste!",
                text: "Com a aba fase.cena aberta, aperte ▶ Testar daqui (F6) e aperte Esc no jogo. Escolha os botões com o mouse ou com as setas e Enter; no deslizador, a esquerda e a direita mudam o volume (e ele fica guardado para a próxima vez).",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "As cores do tema",
                text: "As cores dos botões e painéis vêm do tema da interface: em Configurações do projeto (Ctrl+Shift+P > configurações), abra Tema da interface. Mude a cor do Botão e salve: todos os botões do jogo mudam juntos.\n\nNo Inspetor, uma cor marcada \"do tema\" segue o tema; escolher outra cor muda só aquele componente (↺ tema volta).",
                target: None,
                show: None,
                done: None,
            },
            Step {
                title: "Pronto: HUD e menu!",
                text: "Você fez um HUD preso no canto e uma pausa por cima do jogo, com botões e um volume, sem código.\n\nPróximos passos: ➕ Objeto > Interface > Menu com botões faz um título com Jogar, Opções e Sair. Um botão com Ao apertar: Abrir cena por cima abre uma tela de opções. A Barra mostra vida ou tempo; a Caixa de marcar liga a tela cheia; o Campo de texto pede o nome do jogador. O modelo Plataforma (visual) tem tudo isso pronto para ver.",
                target: None,
                show: None,
                done: None,
            },
        ],
    }
}

/// Os componentes de partículas da cena aberta.
fn emitters(ws: &ProjectWorkspace) -> Vec<&scene_data::Component> {
    objects(ws)
        .into_iter()
        .filter_map(|o| o.component("particles"))
        .collect()
}

fn particulas() -> Tutorial {
    Tutorial {
        id: "efeitos-com-particulas",
        title: "Efeitos com partículas",
        summary: "Fogo e uma explosão prontos, a explosão num prefab e um evento que solta ela com uma tecla.",
        minutes: 6,
        template: Template::Vazio,
        project: "Efeitos",
        steps: vec![
            Step {
                title: "Partículas",
                text: "Fogo, fumaça, faíscas, chuva e explosões são feitos de partículas: muitas bolinhas (ou imagens) que nascem, andam, mudam de cor e somem.\n\nO NN Studio tem efeitos prontos, e eles já animam na cena enquanto você monta.",
                target: None,
                show: None,
                done: None,
            },
            Step {
                title: "Fogo",
                text: "Na barra da cena, clique em ➕ Objeto > Partículas > Fogo. O fogo aparece no meio da vista, já queimando.",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| emitters(ws).iter().any(|c| c.str("mode") == "continuo")),
            },
            Step {
                title: "A direção",
                text: "Com o fogo selecionado, aparece uma seta laranja: é para onde as partículas vão. Arraste a ponta dela para o lado: o fogo passa a soprar para lá, como se tivesse vento.\n\nNo Inspetor, em Partículas, ficam todos os ajustes: quantas por segundo, vida, velocidade, tamanho e as cores.",
                target: Some("cena"),
                show: Some(Tab::Inspector),
                done: Some(|ws, _| {
                    emitters(ws).iter().any(|c| {
                        c.str("mode") == "continuo" && (c.num("direction") - 90.0).abs() > 10.0
                    })
                }),
            },
            Step {
                title: "Uma explosão",
                text: "Agora ➕ Objeto > Partículas > Explosão. Ela solta tudo de uma vez e, no jogo, some sozinha quando termina (Ao terminar: Destruir o objeto). No editor ela repete para você ver; o 💥 Soltar, no Inspetor, solta mais uma.",
                target: Some("cena.objeto"),
                show: None,
                done: Some(|ws, _| emitters(ws).iter().any(|c| c.str("mode") == "explosao")),
            },
            Step {
                title: "Explosão em prefab",
                text: "Uma explosão aparece quando algo acontece: por isso ela vira um prefab. Na Hierarquia, clique com o botão direito em explosao > 📦 Criar prefab. Depois apague a explosão da cena (Delete): o prefab fica guardado em prefabs/explosao.prefab.",
                target: Some("hierarquia"),
                show: Some(Tab::Hierarchy),
                done: Some(|ws, _| ws.files.exists("prefabs/explosao.prefab")),
            },
            Step {
                title: "Explodir com uma tecla",
                text: "No painel Eventos: ➕ Regra da cena > Controles > Tecla ou botão, e escolha a tecla ação (X ou J). Em Então: ➕ ação > Objetos > Criar prefab, escolha prefabs/explosao.prefab e, em Onde, o fogo.",
                target: Some("eventos.regra_cena"),
                show: Some(Tab::Events),
                done: Some(|ws, _| {
                    scene_rules(ws).iter().any(|r| {
                        r.when.iter().any(|c| c.id == "tecla")
                            && r.actions.iter().any(|a| {
                                a.id == "criar" && a.str("prefab").ends_with("explosao.prefab")
                            })
                    })
                }),
            },
            Step {
                title: "Teste!",
                text: "Aperte ▶ Testar daqui (F6) e aperte X no jogo: cada aperto cria uma explosão no fogo, que estoura e some.",
                target: Some("cena.testar"),
                show: None,
                done: Some(|_, st| st.played),
            },
            Step {
                title: "Pronto: efeitos!",
                text: "Próximos passos: Chuva e Neve cobrem a tela (ponha no alto da cena). A Poeira não solta sozinha: a ação Soltar partículas solta quando você quiser (ao pular, ao cair). Acompanham o objeto, no Inspetor, faz o jato de uma nave andar junto com ela. A condição Partículas terminarem avisa quando uma explosão acabou. Os modelos Nave, Plataforma e Aventura têm exemplos.",
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
