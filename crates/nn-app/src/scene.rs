//! Cenas: menu, fases, pausa... empilhadas e trocadas por transições.

use crate::{Context, Game};

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
}

impl std::fmt::Debug for SceneStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SceneStack")
            .field("len", &self.stack.len())
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
        Self { stack: vec![first] }
    }

    /// Quantas cenas estão na pilha.
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
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
        }
    }
}

impl Game for SceneStack {
    fn fixed_update(&mut self, ctx: &mut Context) {
        if let Some(top) = self.stack.last_mut() {
            top.fixed_update(ctx);
        }
    }

    fn update(&mut self, ctx: &mut Context) {
        let transition = match self.stack.last_mut() {
            Some(top) => top.update(ctx),
            None => Transition::None,
        };
        self.apply(ctx, transition);
    }

    fn render(&mut self, ctx: &mut Context) {
        // Desenha a partir da cena mais alta que não é overlay.
        let mut start = self.stack.len().saturating_sub(1);
        while start > 0 && self.stack[start].is_overlay() {
            start -= 1;
        }
        for scene in &mut self.stack[start..] {
            scene.render(ctx);
        }
    }
}
