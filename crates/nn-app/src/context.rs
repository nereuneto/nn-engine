//! Contexto da engine e a trait [`Game`].

use std::time::Duration;

use glam::{UVec2, Vec2};
use nn_audio::Audio;
use nn_core::{Color, FixedTimestep, Time};
use nn_input::Input;
use nn_render::{Camera2D, Renderer};

use crate::{AppConfig, Assets};

/// Tudo o que o jogo pode acessar da engine durante os callbacks.
#[derive(Debug)]
pub struct Context {
    /// Estado do teclado e do mouse.
    pub input: Input,
    /// Tempo real do frame.
    pub time: Time,
    /// Renderer: crie texturas e desenhe sprites por aqui.
    pub renderer: Renderer,
    /// Efeitos sonoros e música.
    pub audio: Audio,
    /// Pasta de assets. Para carregar, use [`Context::load_texture`],
    /// [`Context::load_sound`] e [`Context::load_font`].
    pub assets: Assets,
    /// Câmera usada para desenhar o frame.
    pub camera: Camera2D,
    /// Cor de fundo.
    pub clear_color: Color,
    pub(crate) timestep: FixedTimestep,
    pub(crate) viewport: UVec2,
    pub(crate) exit_requested: bool,
}

impl Context {
    pub(crate) fn new(
        config: &AppConfig,
        renderer: Renderer,
        audio: Audio,
        viewport: UVec2,
    ) -> Self {
        Self {
            input: Input::new(),
            time: Time::default(),
            renderer,
            audio,
            assets: Assets::new(Assets::locate(&config.assets_dir), config.hot_reload),
            camera: Camera2D::default(),
            clear_color: config.clear_color,
            timestep: FixedTimestep::from_hz(config.fixed_hz),
            viewport,
            exit_requested: false,
        }
    }

    /// Tamanho de cada passo de [`Game::fixed_update`], em segundos.
    pub fn fixed_dt(&self) -> f32 {
        self.timestep.step_secs()
    }

    /// Fração (0.0 a 1.0) entre o último passo fixo e o próximo. Use em
    /// [`Game::render`] para interpolar posições e deixar o movimento suave
    /// mesmo quando a taxa de quadros é diferente da taxa de simulação.
    pub fn alpha(&self) -> f32 {
        self.timestep.alpha()
    }

    /// Tamanho da área de desenho em pixels físicos.
    pub fn viewport(&self) -> UVec2 {
        self.viewport
    }

    /// Posição do cursor no mundo, segundo a câmera atual.
    pub fn cursor_world(&self) -> Option<Vec2> {
        let viewport = self.viewport.as_vec2();
        self.input
            .cursor_position()
            .map(|p| self.camera.screen_to_world(p, viewport))
    }

    /// Fecha a aplicação ao final do frame atual.
    pub fn exit(&mut self) {
        self.exit_requested = true;
    }

    /// A aplicação vai fechar ao final deste frame?
    pub fn is_exiting(&self) -> bool {
        self.exit_requested
    }
}

/// Um jogo (ou qualquer aplicação) rodando na nn-engine.
///
/// Para jogos com várias telas (menu, fases, pausa), veja [`crate::SceneStack`].
pub trait Game: 'static {
    /// Simulação em passos fixos de [`Context::fixed_dt`] segundos.
    /// Pode rodar zero ou várias vezes por frame. Para entrada, prefira
    /// `key_down` aqui; transições como `key_pressed` vão em [`Game::update`].
    fn fixed_update(&mut self, _ctx: &mut Context) {}

    /// Chamado uma vez por frame, depois dos passos fixos.
    fn update(&mut self, _ctx: &mut Context) {}

    /// Desenha o frame usando `ctx.renderer`.
    fn render(&mut self, ctx: &mut Context);
}

/// Simulação de um frame: recarga de assets, passos fixos e `update`.
pub(crate) fn simulate<G: Game>(ctx: &mut Context, game: &mut G, dt: Duration) {
    ctx.poll_assets();
    ctx.time.tick(dt);
    let steps = ctx.timestep.advance(dt);
    for _ in 0..steps {
        game.fixed_update(ctx);
    }
    game.update(ctx);
}

/// Desenho de um frame em `target`.
pub(crate) fn draw<G: Game>(
    ctx: &mut Context,
    game: &mut G,
    target: &nn_render::wgpu::TextureView,
) {
    game.render(ctx);
    let clear = ctx.clear_color;
    let camera = ctx.camera;
    let viewport = ctx.viewport;
    ctx.renderer.render(target, viewport, &camera, Some(clear));
}
