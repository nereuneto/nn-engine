//! Cenas: menu, fases, pausa... empilhadas e trocadas por transições.

use nn_core::Color;

use crate::{Context, Game};

/// Efeito de uma transição: a tela escurece até `color`, a cena troca e a
/// tela clareia de novo. Metade da duração para cada lado.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fade {
    /// Duração total, em segundos.
    pub duration: f32,
    pub color: Color,
}

/// O que fazer com a pilha de cenas depois de um [`Scene::update`].
#[derive(Default)]
pub enum Transition {
    /// Continua na mesma cena.
    #[default]
    None,
    /// Abre uma cena por cima da atual (ex.: pausa, inventário).
    Push(Box<dyn Scene>),
    /// Fecha a cena atual e volta para a de baixo.
    Pop,
    /// Troca a cena atual por outra.
    Replace(Box<dyn Scene>),
    /// Fecha todas as cenas e começa só com esta (ex.: voltar ao menu).
    Switch(Box<dyn Scene>),
    /// Fecha o jogo.
    Quit,
    /// Outra transição, feita com a tela escurecendo e clareando
    /// (crie com [`Transition::fade`]).
    Fade(Box<Transition>, Fade),
}

impl Transition {
    /// A mesma transição com a tela escurecendo até o preto e clareando,
    /// em `seconds` no total.
    ///
    /// ```
    /// # use nn_app::{Scene, Transition, Context};
    /// # struct Fase;
    /// # impl Scene for Fase { fn render(&mut self, _: &mut Context) {} }
    /// let t = Transition::Switch(Box::new(Fase)).fade(0.6);
    /// ```
    pub fn fade(self, seconds: f32) -> Self {
        self.fade_with(seconds, Color::BLACK)
    }

    /// Como [`Transition::fade`], escurecendo até `color`.
    pub fn fade_with(self, seconds: f32, color: Color) -> Self {
        let fade = Fade {
            duration: seconds.max(0.0),
            color,
        };
        match self {
            Transition::None => Transition::None,
            Transition::Fade(inner, _) => Transition::Fade(inner, fade),
            other => Transition::Fade(Box::new(other), fade),
        }
    }
}

impl std::fmt::Debug for Transition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::None => "None",
            Self::Push(_) => "Push",
            Self::Pop => "Pop",
            Self::Replace(_) => "Replace",
            Self::Switch(_) => "Switch",
            Self::Quit => "Quit",
            Self::Fade(inner, fade) => {
                return write!(f, "Fade({inner:?}, {}s)", fade.duration);
            }
        };
        f.write_str(name)
    }
}

/// Uma tela do jogo. Só a cena do topo da pilha recebe `update`.
pub trait Scene: 'static {
    /// A cena entrou na pilha.
    fn enter(&mut self, _ctx: &mut Context) {}

    /// A cena saiu da pilha.
    fn exit(&mut self, _ctx: &mut Context) {}

    /// A cena voltou a ficar no topo (a de cima foi fechada).
    fn resume(&mut self, _ctx: &mut Context) {}

    /// Passo fixo da simulação (veja [`Game::fixed_update`]).
    fn fixed_update(&mut self, _ctx: &mut Context) {}

    /// Uma vez por frame. Devolva uma [`Transition`] para trocar de cena.
    fn update(&mut self, _ctx: &mut Context) -> Transition {
        Transition::None
    }

    /// Desenha a cena.
    fn render(&mut self, ctx: &mut Context);

    /// Se verdadeiro, a cena de baixo continua sendo desenhada por trás
    /// (útil para pausa e janelas). Ela só não recebe `update`.
    fn is_overlay(&self) -> bool {
        false
    }
}

/// Pilha de cenas. Implementa [`Game`], então pode ser passada para [`crate::run`].
///
/// ```no_run
/// use nn_app::{AppConfig, Context, Scene, SceneStack, Transition};
///
/// struct Menu;
/// impl Scene for Menu {
///     fn render(&mut self, _ctx: &mut Context) {}
/// }
///
/// nn_app::run(AppConfig::new("Jogo"), |ctx| SceneStack::new(ctx, Menu)).unwrap();
/// ```
pub struct SceneStack {
    stack: Vec<Box<dyn Scene>>,
    fade: Option<FadeState>,
}

/// Transição com escurecimento em andamento.
struct FadeState {
    fade: Fade,
    elapsed: f32,
    /// Transição esperando a tela escurecer. `None` = já trocou, clareando.
    pending: Option<Transition>,
}

impl FadeState {
    fn half(&self) -> f32 {
        self.fade.duration / 2.0
    }

    /// Quanto a tela está coberta, de 0 a 1.
    fn cover(&self) -> f32 {
        let half = self.half();
        if half <= 0.0 {
            return 0.0;
        }
        let k = if self.pending.is_some() {
            self.elapsed / half
        } else {
            1.0 - (self.elapsed - half) / half
        };
        // Suaviza o começo e o fim.
        let k = k.clamp(0.0, 1.0);
        k * k * (3.0 - 2.0 * k)
    }
}

impl std::fmt::Debug for SceneStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SceneStack")
            .field("len", &self.stack.len())
            .field("fading", &self.fade.is_some())
            .finish()
    }
}

impl SceneStack {
    /// Cria a pilha com a cena inicial (que recebe `enter` imediatamente).
    pub fn new(ctx: &mut Context, first: impl Scene) -> Self {
        Self::from_boxed(ctx, Box::new(first))
    }

    pub fn from_boxed(ctx: &mut Context, mut first: Box<dyn Scene>) -> Self {
        first.enter(ctx);
        Self {
            stack: vec![first],
            fade: None,
        }
    }

    /// Quantas cenas estão na pilha.
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Há uma transição com escurecimento em andamento?
    pub fn is_fading(&self) -> bool {
        self.fade.is_some()
    }

    /// A tela está escurecendo para trocar de cena (as cenas ficam paradas).
    fn covering(&self) -> bool {
        self.fade.as_ref().is_some_and(|f| f.pending.is_some())
    }

    /// Avança o escurecimento. Na metade, aplica a transição que esperava.
    fn advance_fade(&mut self, ctx: &mut Context) {
        let Some(mut state) = self.fade.take() else {
            ctx.effects.fade = Color::TRANSPARENT;
            return;
        };
        state.elapsed += ctx.time.delta_secs();
        if state.pending.is_some() && state.elapsed >= state.half() {
            let pending = state.pending.take().expect("conferido acima");
            state.elapsed = state.elapsed.max(state.half());
            self.apply(ctx, pending);
            if self.fade.is_some() {
                // A transição que esperava começou outro escurecimento.
                return;
            }
        }
        let done = state.pending.is_none() && state.elapsed >= state.fade.duration;
        ctx.effects.fade = if done {
            Color::TRANSPARENT
        } else {
            state
                .fade
                .color
                .with_alpha(state.fade.color.a * state.cover())
        };
        if !done {
            self.fade = Some(state);
        }
    }

    /// Aplica uma transição como se a cena do topo a tivesse pedido.
    pub fn apply(&mut self, ctx: &mut Context, transition: Transition) {
        match transition {
            Transition::None => {}
            Transition::Push(mut scene) => {
                scene.enter(ctx);
                self.stack.push(scene);
            }
            Transition::Pop => {
                if let Some(mut top) = self.stack.pop() {
                    top.exit(ctx);
                }
                match self.stack.last_mut() {
                    Some(scene) => scene.resume(ctx),
                    None => ctx.exit(),
                }
            }
            Transition::Replace(mut scene) => {
                if let Some(mut top) = self.stack.pop() {
                    top.exit(ctx);
                }
                scene.enter(ctx);
                self.stack.push(scene);
            }
            Transition::Switch(mut scene) => {
                while let Some(mut top) = self.stack.pop() {
                    top.exit(ctx);
                }
                scene.enter(ctx);
                self.stack.push(scene);
            }
            Transition::Quit => ctx.exit(),
            Transition::Fade(inner, fade) => {
                if fade.duration <= 0.0 {
                    self.apply(ctx, *inner);
                    return;
                }
                // Se já estava clareando, escurece a partir de onde parou.
                let elapsed = self
                    .fade
                    .as_ref()
                    .filter(|f| f.pending.is_none())
                    .map_or(0.0, |f| f.cover() * fade.duration / 2.0);
                self.fade = Some(FadeState {
                    fade,
                    elapsed,
                    pending: Some(*inner),
                });
            }
        }
    }
}

impl Game for SceneStack {
    fn fixed_update(&mut self, ctx: &mut Context) {
        // Enquanto a tela escurece para trocar, a cena fica parada.
        if self.covering() {
            return;
        }
        if let Some(top) = self.stack.last_mut() {
            top.fixed_update(ctx);
        }
    }

    fn update(&mut self, ctx: &mut Context) {
        if !self.covering() {
            let transition = match self.stack.last_mut() {
                Some(top) => top.update(ctx),
                None => Transition::None,
            };
            self.apply(ctx, transition);
        }
        self.advance_fade(ctx);
    }

    fn render(&mut self, ctx: &mut Context) {
        // Desenha a partir da cena mais alta que não é overlay.
        let mut start = self.stack.len().saturating_sub(1);
        while start > 0 && self.stack[start].is_overlay() {
            start -= 1;
        }
        let top = self.stack.len().saturating_sub(1);
        let interactive = ctx.ui.is_interactive();
        let covering = self.covering();
        for (i, scene) in self.stack.iter_mut().enumerate().skip(start) {
            // Só a cena do topo responde à interface; cada cena fecha seus
            // painéis antes da próxima desenhar por cima.
            ctx.ui.set_interactive(interactive && i == top && !covering);
            scene.render(ctx);
            ctx.finish_panel();
        }
        ctx.ui.set_interactive(interactive);
    }
}
