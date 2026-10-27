//! Testes da interface (painéis, foco, mouse, controle e cenas).

mod comum;

use glam::Vec2;
use nn_app::{AppConfig, Context, Game, Headless, PanelOptions, Scene, SceneStack, Transition};
use nn_core::Color;
use nn_input::{KeyCode, MouseButton, PadButton};

const W: u32 = 640;
const H: u32 = 360;

fn config() -> AppConfig {
    AppConfig::new("ui").size(W, H).clear_color(Color::BLACK)
}

/// Menu com três itens: dois botões e uma barra.
#[derive(Default)]
struct Menu {
    show: bool,
    log: Vec<String>,
    volume: f32,
    back: bool,
}

impl Game for Menu {
    fn render(&mut self, ctx: &mut Context) {
        if !self.show {
            return;
        }
        let mut p = ctx.panel("menu", PanelOptions::new());
        if p.button("Jogar") {
            self.log.push("jogar".into());
        }
        if p.button("Sair") {
            self.log.push("sair".into());
        }
        p.slider("Volume", &mut self.volume, 0.0, 1.0, 0.1);
        self.back |= p.back();
        p.finish();
    }
}

fn menu() -> Option<Headless<Menu>> {
    let mut h = comum::headless(config(), |ctx| {
        // Escala fixa para os testes saberem onde ficam os itens.
        ctx.ui.theme_mut().scale = Some(1.0);
        Menu {
            show: true,
            volume: 0.5,
            ..Default::default()
        }
    })?;
    h.run_frames(1);
    Some(h)
}

/// Centro do item `i` do menu (tema padrão, escala 1, painel no centro).
fn item_center(i: usize) -> Vec2 {
    let (padding, row, spacing) = (24.0, 52.0, 10.0);
    let height = padding * 2.0 + row * 3.0 + spacing * 2.0;
    let top = (H as f32 - height) / 2.0;
    Vec2::new(
        W as f32 / 2.0,
        top + padding + i as f32 * (row + spacing) + row / 2.0,
    )
}

fn tap(h: &mut Headless<impl Game>, key: KeyCode) {
    h.ctx.input.press_key(key);
    h.run_frames(1);
    h.ctx.input.release_key(key);
    h.run_frames(1);
}

fn pixel(h: &Headless<impl Game>, p: Vec2) -> [u8; 4] {
    let px = h.pixels().unwrap();
    let i = ((p.y as u32 * W + p.x as u32) * 4) as usize;
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}

fn close(a: [u8; 4], b: Color) -> bool {
    let b = b.to_array().map(|c| (c * 255.0).round() as u8);
    a.iter()
        .zip(b.iter())
        .take(3)
        .all(|(x, y)| x.abs_diff(*y) <= 3)
}

#[test]
fn teclado_navega_e_confirma() {
    let Some(mut h) = menu() else { return };
    assert_eq!(h.ctx.ui.focus("menu"), Some(0));

    tap(&mut h, KeyCode::ArrowDown);
    assert_eq!(h.ctx.ui.focus("menu"), Some(1));
    tap(&mut h, KeyCode::Enter);
    assert_eq!(h.game.log, ["sair"]);

    // Para cima a partir do primeiro volta ao último.
    tap(&mut h, KeyCode::KeyW);
    tap(&mut h, KeyCode::ArrowUp);
    assert_eq!(h.ctx.ui.focus("menu"), Some(2));
    tap(&mut h, KeyCode::ArrowRight);
    assert!((h.game.volume - 0.6).abs() < 1e-5, "{}", h.game.volume);
    tap(&mut h, KeyCode::KeyA);
    tap(&mut h, KeyCode::KeyA);
    assert!((h.game.volume - 0.4).abs() < 1e-5, "{}", h.game.volume);
}

#[test]
fn toque_rapido_dentro_do_frame_conta() {
    let Some(mut h) = menu() else { return };
    // Apertou e soltou antes do frame ser processado (comum com FPS baixo).
    h.ctx.input.press_key(KeyCode::ArrowDown);
    h.ctx.input.release_key(KeyCode::ArrowDown);
    h.run_frames(1);
    assert_eq!(h.ctx.ui.focus("menu"), Some(1));
    h.ctx.input.press_key(KeyCode::ArrowDown);
    h.ctx.input.release_key(KeyCode::ArrowDown);
    h.run_frames(1);
    assert_eq!(h.ctx.ui.focus("menu"), Some(2));
    h.ctx.input.press_key(KeyCode::ArrowRight);
    h.ctx.input.release_key(KeyCode::ArrowRight);
    h.run_frames(1);
    assert!((h.game.volume - 0.6).abs() < 1e-5, "{}", h.game.volume);
}

#[test]
fn segurar_a_direcao_repete() {
    let Some(mut h) = menu() else { return };
    h.ctx.input.press_key(KeyCode::ArrowDown);
    h.run_frames(1);
    assert_eq!(h.ctx.ui.focus("menu"), Some(1));
    // Durante o atraso inicial não anda.
    h.run_frames(15);
    assert_eq!(h.ctx.ui.focus("menu"), Some(1));
    // Depois anda sozinho.
    h.run_frames(15);
    assert_ne!(h.ctx.ui.focus("menu"), Some(1));
}

#[test]
fn controle_navega_e_confirma() {
    let Some(mut h) = menu() else { return };
    let pad = h.ctx.input.connect_pad("controle de teste");
    h.ctx.input.press_pad(pad, PadButton::DPadDown);
    h.run_frames(1);
    h.ctx.input.release_pad(pad, PadButton::DPadDown);
    h.ctx.input.press_pad(pad, PadButton::South);
    h.run_frames(1);
    assert_eq!(h.game.log, ["sair"]);

    // Analógico também navega.
    h.ctx
        .input
        .set_pad_axis(pad, nn_input::PadAxis::LeftY, -0.9);
    h.run_frames(1);
    h.ctx.input.set_pad_axis(pad, nn_input::PadAxis::LeftY, 0.0);
    assert_eq!(h.ctx.ui.focus("menu"), Some(2));

    h.ctx.input.press_pad(pad, PadButton::East);
    h.run_frames(1);
    assert!(h.game.back);
}

#[test]
fn mouse_foca_e_clica() {
    let Some(mut h) = menu() else { return };
    h.ctx.input.set_cursor(Some(Vec2::new(5.0, 5.0)));
    h.run_frames(1);
    h.ctx.input.set_cursor(Some(item_center(1)));
    h.run_frames(1);
    assert_eq!(h.ctx.ui.focus("menu"), Some(1));

    h.ctx.input.set_cursor(Some(item_center(0)));
    h.ctx.input.press_mouse(MouseButton::Left);
    h.run_frames(1);
    h.ctx.input.release_mouse(MouseButton::Left);
    h.run_frames(1);
    assert_eq!(h.game.log, ["jogar"]);

    // Clique no texto da barra só foca; na barra, muda o valor; arrastar continua.
    let row = item_center(2);
    h.ctx
        .input
        .set_cursor(Some(Vec2::new(row.x - 150.0, row.y)));
    h.ctx.input.press_mouse(MouseButton::Left);
    h.run_frames(1);
    h.ctx.input.release_mouse(MouseButton::Left);
    h.run_frames(1);
    assert!((h.game.volume - 0.5).abs() < 1e-5);

    let (bar_start, bar_end) = (110.0 + 24.0 + 372.0 * 0.45, 110.0 + 24.0 + 372.0 - 86.0);
    h.ctx.input.set_cursor(Some(Vec2::new(bar_start, row.y)));
    h.ctx.input.press_mouse(MouseButton::Left);
    h.run_frames(1);
    assert!(h.game.volume.abs() < 1e-5, "{}", h.game.volume);
    h.ctx
        .input
        .set_cursor(Some(Vec2::new(bar_end + 40.0, row.y)));
    h.run_frames(1);
    assert!((h.game.volume - 1.0).abs() < 1e-5, "{}", h.game.volume);
    h.ctx.input.release_mouse(MouseButton::Left);
    h.ctx.input.set_cursor(Some(Vec2::new(bar_start, row.y)));
    h.run_frames(1);
    assert!((h.game.volume - 1.0).abs() < 1e-5, "{}", h.game.volume);
}

#[test]
fn painel_ignora_o_aperto_que_o_abriu() {
    let Some(mut h) = comum::headless(config(), |_| Menu::default()) else {
        return;
    };
    h.run_frames(2);
    // O mesmo Enter que "abre" o menu não pode acionar o primeiro botão.
    h.game.show = true;
    h.ctx.input.press_key(KeyCode::Enter);
    h.ctx.input.press_key(KeyCode::Escape);
    h.run_frames(1);
    assert!(h.game.log.is_empty());
    assert!(!h.game.back);
    h.ctx.input.release_key(KeyCode::Enter);
    h.ctx.input.release_key(KeyCode::Escape);
    h.run_frames(1);
    tap(&mut h, KeyCode::Escape);
    assert!(h.game.back);
}

#[test]
fn painel_e_foco_sao_desenhados() {
    let Some(mut h) = menu() else { return };
    h.run_frames(1);
    let theme = h.ctx.ui.theme().clone();
    // Fora do painel: fundo da tela.
    assert!(close(pixel(&h, Vec2::new(5.0, 5.0)), Color::BLACK));
    // Canto do item com foco (longe do texto): cor de foco.
    let first = item_center(0);
    assert!(close(
        pixel(&h, Vec2::new(110.0 + 24.0 + 4.0, first.y)),
        theme.focus_color
    ));
    // Margem do painel: fundo do painel misturado ao preto.
    let margin = pixel(&h, Vec2::new(110.0 + 8.0, first.y));
    assert!(!close(margin, Color::BLACK) && !close(margin, theme.focus_color));

    tap(&mut h, KeyCode::ArrowDown);
    assert!(!close(
        pixel(&h, Vec2::new(110.0 + 24.0 + 4.0, first.y)),
        theme.focus_color
    ));
}

/// Cena com um painel, que registra os botões acionados.
struct Tela {
    name: &'static str,
    overlay: bool,
    log: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}

impl Scene for Tela {
    fn render(&mut self, ctx: &mut Context) {
        let mut p = ctx.panel(self.name, PanelOptions::new().title(self.name));
        if p.button("ok") {
            self.log.borrow_mut().push(self.name.into());
        }
        // Sem finish: a pilha de cenas fecha o painel.
    }

    fn is_overlay(&self) -> bool {
        self.overlay
    }

    fn update(&mut self, ctx: &mut Context) -> Transition {
        if self.name == "jogo" && ctx.input.key_pressed(KeyCode::KeyP) {
            return Transition::Push(Box::new(Tela {
                name: "pausa",
                overlay: true,
                log: self.log.clone(),
            }));
        }
        Transition::None
    }
}

#[test]
fn so_a_cena_do_topo_responde() {
    let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let log2 = log.clone();
    let Some(mut h) = comum::headless(config(), move |ctx| {
        SceneStack::new(
            ctx,
            Tela {
                name: "jogo",
                overlay: false,
                log: log2,
            },
        )
    }) else {
        return;
    };
    h.run_frames(2);
    tap(&mut h, KeyCode::KeyP);
    assert_eq!(h.game.len(), 2);
    tap(&mut h, KeyCode::Enter);
    assert_eq!(*log.borrow(), ["pausa"]);
    assert!(!h.ctx.ui.is_open());
    assert!(h.ctx.ui.is_interactive());
}

/// Uma lista de 200 botões num painel de 300 de altura.
#[derive(Default)]
struct Lista {
    log: Vec<usize>,
}

impl Game for Lista {
    fn render(&mut self, ctx: &mut Context) {
        let mut p = ctx.panel("lista", PanelOptions::new().height(300.0));
        for i in 0..200 {
            if p.button(&format!("Item {i}")) {
                self.log.push(i);
            }
        }
        p.finish();
    }
}

fn lista() -> Option<Headless<Lista>> {
    let mut h = comum::headless(config(), |ctx| {
        ctx.ui.theme_mut().scale = Some(1.0);
        Lista::default()
    })?;
    h.run_frames(2);
    Some(h)
}

/// Topo do painel da lista (300 de altura, no centro).
const LISTA_TOPO: f32 = (H as f32 - 300.0) / 2.0;

#[test]
fn lista_longa_desenha_so_o_que_aparece() {
    let Some(h) = lista() else { return };
    // Uns 5 botões aparecem (cada um: fundo e texto), não 200.
    let sprites = h.ctx.renderer.last_stats().sprites;
    assert!(sprites < 200, "{sprites}");
    // Nada do painel abaixo dele.
    let fundo = Color::BLACK;
    assert!(close(
        pixel(&h, Vec2::new(W as f32 / 2.0, LISTA_TOPO + 304.0)),
        fundo
    ));
    assert!(!close(
        pixel(&h, Vec2::new(W as f32 / 2.0, LISTA_TOPO + 290.0)),
        fundo
    ));
}

#[test]
fn lista_rola_com_o_teclado_a_roda_e_o_arrastar() {
    let Some(mut h) = lista() else { return };
    // O teclado: descer 10 itens leva a lista junto e o item 10 fica visível.
    for _ in 0..10 {
        tap(&mut h, KeyCode::ArrowDown);
    }
    assert_eq!(h.ctx.ui.focus("lista"), Some(10));
    tap(&mut h, KeyCode::Enter);
    assert_eq!(h.game.log, [10]);

    // A roda: rola para baixo (y negativo) com o mouse em cima.
    let meio = Vec2::new(W as f32 / 2.0, LISTA_TOPO + 150.0);
    h.ctx.input.set_cursor(Some(meio));
    h.run_frames(1);
    let antes = h.ctx.ui.focus("lista").unwrap();
    h.ctx.input.add_scroll(Vec2::new(0.0, -10.0));
    h.run_frames(2);
    let depois = h.ctx.ui.focus("lista").unwrap();
    assert!(depois > antes + 5, "{antes} → {depois}");

    // Arrastar para cima rola para baixo, e não aperta o item.
    h.ctx.input.press_mouse(MouseButton::Left);
    h.run_frames(1);
    for k in 1..=10 {
        h.ctx
            .input
            .set_cursor(Some(meio - Vec2::new(0.0, k as f32 * 10.0)));
        h.run_frames(1);
    }
    h.ctx.input.release_mouse(MouseButton::Left);
    h.run_frames(2);
    assert_eq!(h.game.log, [10], "arrastar não é clique");
    // O item embaixo do cursor andou junto com ele; no meio agora há outro.
    assert_eq!(h.ctx.ui.focus("lista"), Some(depois));
    h.ctx.input.set_cursor(Some(meio));
    h.run_frames(1);
    let embaixo = h.ctx.ui.focus("lista").unwrap();
    assert!(embaixo > depois, "{depois} → {embaixo}");

    // Um clique (apertar e soltar sem arrastar) aperta o item embaixo.
    h.ctx.input.press_mouse(MouseButton::Left);
    h.run_frames(1);
    assert_eq!(h.game.log, [10], "vale ao soltar");
    h.ctx.input.release_mouse(MouseButton::Left);
    h.run_frames(1);
    assert_eq!(h.game.log, [10, embaixo]);

    // Rolar até o fim para no último.
    h.ctx.input.add_scroll(Vec2::new(0.0, -1000.0));
    h.run_frames(2);
    h.ctx
        .input
        .set_cursor(Some(Vec2::new(W as f32 / 2.0, LISTA_TOPO + 260.0)));
    h.run_frames(1);
    assert_eq!(h.ctx.ui.focus("lista"), Some(199));
}
