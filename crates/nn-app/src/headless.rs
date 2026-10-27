//! Execução sem janela: roda frames de um jogo desenhando em memória.
//! Serve para testes automatizados e para gerar capturas de tela.

use std::path::Path;
use std::time::Duration;

use glam::UVec2;
use nn_audio::Audio;
use nn_render::{GpuContext, OffscreenTarget, RenderError, Renderer};

use crate::context::{draw, simulate};
use crate::{AppConfig, AppError, Context, Game};

/// Um jogo rodando sem janela e sem som.
///
/// ```no_run
/// # use nn_app::{AppConfig, Context, Game, Headless};
/// # struct Jogo;
/// # impl Game for Jogo { fn render(&mut self, _: &mut Context) {} }
/// let mut h = Headless::new(AppConfig::default(), |_ctx| Jogo).unwrap();
/// h.ctx.input.press_key(nn_input::KeyCode::KeyD);
/// h.run_frames(30);
/// h.save_png("captura.png").unwrap();
/// ```
#[derive(Debug)]
pub struct Headless<G: Game> {
    pub ctx: Context,
    pub game: G,
    gpu: GpuContext,
    target: OffscreenTarget,
    /// O último quadro foi desenhado no alvo?
    drawn: bool,
}

impl<G: Game> Headless<G> {
    /// Duração de frame usada por [`Headless::run_frames`] (60 quadros por segundo).
    pub const FRAME: Duration = Duration::from_nanos(16_666_667);

    /// Cria o contexto com GPU sem janela, no tamanho `config.size`.
    pub fn new(
        config: AppConfig,
        create: impl FnOnce(&mut Context) -> G,
    ) -> Result<Self, AppError> {
        let gpu = pollster::block_on(GpuContext::new_headless())?;
        let size = UVec2::new(config.size.0.max(1), config.size.1.max(1));
        let renderer = Renderer::new(&gpu, OffscreenTarget::FORMAT);
        let target = OffscreenTarget::new(&gpu, size);
        // Sem `save_dir`, nada é gravado: testes não mexem nos dados do usuário.
        let save = match &config.save_dir {
            Some(_) => config.open_storage(),
            None => crate::Storage::in_memory(),
        };
        let mut ctx = Context::new(&config, renderer, Audio::disabled(), size, save);
        ctx.perf.simulated = true;
        let game = create(&mut ctx);
        Ok(Self {
            ctx,
            game,
            gpu,
            target,
            drawn: false,
        })
    }

    /// Roda um frame completo que durou `dt`: simulação, desenho e fim de frame.
    pub fn frame(&mut self, dt: Duration) {
        self.frame_with(dt, true);
    }

    /// Como [`Headless::frame`], mas sem a GPU: o `render` do jogo roda (os
    /// erros dele aparecem), e o que ele pediu é jogado fora. Bem mais
    /// rápido sem placa de vídeo. Antes de ler a tela, [`Headless::redraw`].
    pub fn frame_undrawn(&mut self, dt: Duration) {
        self.frame_with(dt, false);
    }

    /// Desenha de novo o quadro atual, sem avançar o jogo (se o último foi
    /// sem desenho). Pedidos de imagem da tela são atendidos aqui.
    pub fn redraw(&mut self) {
        if !self.drawn {
            draw(&mut self.ctx, &mut self.game, Some(self.target.view()));
            self.drawn = true;
        }
    }

    /// O último quadro foi desenhado (a tela que [`Headless::pixels`] lê)?
    pub fn drawn(&self) -> bool {
        self.drawn
    }

    fn frame_with(&mut self, dt: Duration, gpu: bool) {
        simulate(&mut self.ctx, &mut self.game, dt);
        // Pedir a imagem da tela força o desenho.
        let gpu = gpu || self.ctx.screenshot_request;
        draw(
            &mut self.ctx,
            &mut self.game,
            gpu.then(|| self.target.view()),
        );
        self.drawn = gpu;
        if std::mem::take(&mut self.ctx.screenshot_request) {
            let size = self.target.size();
            let shot = self
                .target
                .read_rgba(&self.gpu)
                .map(|pixels| crate::Screenshot {
                    width: size.x,
                    height: size.y,
                    pixels,
                })
                .map_err(|e| e.to_string());
            self.ctx.screenshot = Some(shot);
        }
        if let Some(fullscreen) = self.ctx.fullscreen_request.take() {
            self.ctx.fullscreen = fullscreen;
        }
        self.ctx.save.auto_flush();
        self.ctx.input.end_frame();
    }

    /// Roda `n` frames de 1/60 s (para antes se o jogo pedir para fechar).
    pub fn run_frames(&mut self, n: u32) {
        for _ in 0..n {
            if self.ctx.exit_requested {
                return;
            }
            self.frame(Self::FRAME);
        }
    }

    /// Pixels do último frame desenhado (RGBA, de cima para baixo).
    pub fn pixels(&self) -> Result<Vec<u8>, RenderError> {
        self.target.read_rgba(&self.gpu)
    }

    /// Salva o último frame como PNG.
    pub fn save_png(&self, path: impl AsRef<Path>) -> Result<(), RenderError> {
        self.target.save_png(&self.gpu, path)
    }
}
