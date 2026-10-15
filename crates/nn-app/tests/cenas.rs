//! Testes da pilha de cenas.

mod comum;

use std::cell::RefCell;
use std::rc::Rc;

use nn_app::{AppConfig, Context, Scene, SceneStack, Transition};

type Log = Rc<RefCell<Vec<String>>>;
/// Transição que a cena vai pedir no próximo `update`.
type Next = Box<dyn FnOnce(&Log) -> Transition>;

/// Cena que registra o que acontece com ela e executa a transição programada.
struct Probe {
    name: &'static str,
    log: Log,
    next: Option<Next>,
    overlay: bool,
}

impl Probe {
    fn new(name: &'static str, log: &Log) -> Self {
        Self {
            name,
            log: log.clone(),
            next: None,
            overlay: false,
        }
    }

    fn then(mut self, f: impl FnOnce(&Log) -> Transition + 'static) -> Self {
        self.next = Some(Box::new(f));
        self
    }

    fn overlay(mut self) -> Self {
        self.overlay = true;
        self
    }

    fn rec(&self, event: &str) {
        self.log.borrow_mut().push(format!("{}.{event}", self.name));
    }
}

impl Scene for Probe {
    fn enter(&mut self, _: &mut Context) {
        self.rec("enter");
    }
    fn exit(&mut self, _: &mut Context) {
        self.rec("exit");
    }
    fn resume(&mut self, _: &mut Context) {
        self.rec("resume");
    }
    fn update(&mut self, _: &mut Context) -> Transition {
        self.rec("update");
        match self.next.take() {
            Some(f) => f(&self.log),
            None => Transition::None,
        }
    }
    fn render(&mut self, _: &mut Context) {
        self.rec("render");
    }
    fn is_overlay(&self) -> bool {
        self.overlay
    }
}

fn take(log: &Log) -> Vec<String> {
    std::mem::take(&mut *log.borrow_mut())
}

#[test]
fn push_pop_e_resume() {
    let log = Log::default();
    let l = log.clone();
    let Some(mut h) = comum::headless(AppConfig::default(), move |ctx| {
        let menu = Probe::new("menu", &l).then(|log| {
            Transition::Push(Box::new(Probe::new("pausa", log).then(|_| Transition::Pop)))
        });
        SceneStack::new(ctx, menu)
    }) else {
        return;
    };
    assert_eq!(take(&log), ["menu.enter"]);

    h.run_frames(1);
    assert_eq!(take(&log), ["menu.update", "pausa.enter", "pausa.render"]);
    assert_eq!(h.game.len(), 2);

    h.run_frames(1);
    assert_eq!(
        take(&log),
        ["pausa.update", "pausa.exit", "menu.resume", "menu.render"]
    );
    assert_eq!(h.game.len(), 1);
}

#[test]
fn overlay_desenha_a_cena_de_baixo() {
    let log = Log::default();
    let l = log.clone();
    let Some(mut h) = comum::headless(AppConfig::default(), move |ctx| {
        let jogo = Probe::new("jogo", &l)
            .then(|log| Transition::Push(Box::new(Probe::new("pausa", log).overlay())));
        SceneStack::new(ctx, jogo)
    }) else {
        return;
    };
    h.run_frames(2);
    let eventos = take(&log);
    // No segundo frame: só a pausa recebe update, mas as duas são desenhadas, de baixo para cima.
    assert_eq!(
        &eventos[eventos.len() - 3..],
        ["pausa.update", "jogo.render", "pausa.render"]
    );
}

#[test]
fn replace_e_switch() {
    let log = Log::default();
    let l = log.clone();
    let Some(mut h) = comum::headless(AppConfig::default(), move |ctx| {
        let a = Probe::new("a", &l).then(|log| {
            Transition::Push(Box::new(Probe::new("b", log).then(|log| {
                Transition::Replace(Box::new(
                    Probe::new("c", log)
                        .then(|log| Transition::Switch(Box::new(Probe::new("menu", log)))),
                ))
            })))
        });
        SceneStack::new(ctx, a)
    }) else {
        return;
    };
    h.run_frames(3);
    let eventos = take(&log);
    let sem_render: Vec<_> = eventos
        .iter()
        .filter(|e| !e.ends_with("render"))
        .cloned()
        .collect();
    assert_eq!(
        sem_render,
        [
            "a.enter",
            "a.update",
            "b.enter",
            "b.update",
            "b.exit",
            "c.enter",
            "c.update",
            "c.exit",
            "a.exit",
            "menu.enter"
        ]
    );
    assert_eq!(h.game.len(), 1);
}

#[test]
fn pop_da_ultima_cena_fecha_o_jogo() {
    let log = Log::default();
    let l = log.clone();
    let Some(mut h) = comum::headless(AppConfig::default(), move |ctx| {
        SceneStack::new(ctx, Probe::new("unica", &l).then(|_| Transition::Pop))
    }) else {
        return;
    };
    assert!(!h.ctx.is_exiting());
    h.run_frames(1);
    assert!(h.ctx.is_exiting());
    assert!(h.game.is_empty());
}

#[test]
fn quit_fecha_o_jogo() {
    let log = Log::default();
    let l = log.clone();
    let Some(mut h) = comum::headless(AppConfig::default(), move |ctx| {
        SceneStack::new(ctx, Probe::new("x", &l).then(|_| Transition::Quit))
    }) else {
        return;
    };
    h.run_frames(1);
    assert!(h.ctx.is_exiting());
}

fn pixel_central<G: nn_app::Game>(h: &nn_app::Headless<G>) -> [u8; 4] {
    let px = h.pixels().unwrap();
    let i = ((32 * 64 + 32) * 4) as usize;
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}

#[test]
fn transicao_com_escurecimento() {
    let log = Log::default();
    let l = log.clone();
    let config = AppConfig::default()
        .size(64, 64)
        .clear_color(nn_core::Color::WHITE);
    let Some(mut h) = comum::headless(config, move |ctx| {
        let a = Probe::new("a", &l)
            .then(|log| Transition::Switch(Box::new(Probe::new("b", log))).fade(0.5));
        SceneStack::new(ctx, a)
    }) else {
        return;
    };
    h.run_frames(1);
    assert!(h.game.is_fading());
    take(&log);

    // Escurecendo: a cena antiga só é desenhada (não recebe update) e a
    // nova ainda não entrou.
    h.run_frames(10);
    let eventos = take(&log);
    assert!(eventos.iter().all(|e| e == "a.render"), "{eventos:?}");
    let [r, ..] = pixel_central(&h);
    assert!(r > 20 && r < 250, "meio escuro: {r}");

    // Na metade, troca com a tela coberta.
    h.run_frames(6);
    let eventos = take(&log);
    assert!(eventos.contains(&"a.exit".to_string()), "{eventos:?}");
    assert!(eventos.contains(&"b.enter".to_string()), "{eventos:?}");
    let [r, ..] = pixel_central(&h);
    assert!(r < 100, "coberta: {r}");

    // Clareando: a cena nova já roda.
    h.run_frames(20);
    let eventos = take(&log);
    assert!(eventos.contains(&"b.update".to_string()));
    assert!(!h.game.is_fading());
    assert_eq!(pixel_central(&h)[0], 255);
}

#[test]
fn sair_com_escurecimento() {
    let log = Log::default();
    let l = log.clone();
    let Some(mut h) = comum::headless(AppConfig::default(), move |ctx| {
        SceneStack::new(
            ctx,
            Probe::new("x", &l).then(|_| Transition::Quit.fade(0.2)),
        )
    }) else {
        return;
    };
    h.run_frames(3);
    assert!(!h.ctx.is_exiting(), "espera escurecer");
    h.run_frames(5);
    assert!(h.ctx.is_exiting());
}

#[test]
fn fade_de_nada_e_nada_e_duracao_zero_e_imediata() {
    assert!(matches!(Transition::None.fade(1.0), Transition::None));
    let t = Transition::Pop
        .fade(1.0)
        .fade_with(0.3, nn_core::Color::WHITE);
    match t {
        Transition::Fade(inner, f) => {
            assert!(matches!(*inner, Transition::Pop));
            assert_eq!(f.duration, 0.3);
        }
        other => panic!("{other:?}"),
    }

    let log = Log::default();
    let l = log.clone();
    let Some(mut h) = comum::headless(AppConfig::default(), move |ctx| {
        SceneStack::new(
            ctx,
            Probe::new("a", &l)
                .then(|log| Transition::Push(Box::new(Probe::new("b", log))).fade(0.0)),
        )
    }) else {
        return;
    };
    h.run_frames(1);
    assert_eq!(h.game.len(), 2);
    assert!(!h.game.is_fading());
}
