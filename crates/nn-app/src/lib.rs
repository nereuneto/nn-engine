//! Aplicação da nn-engine: abre a janela, conecta a GPU e roda o loop principal.
//!
//! O jogo implementa [`Game`] e chama [`run`], passando uma função que cria o
//! jogo depois que a GPU está pronta (para já poder carregar texturas).
//! A cada frame a engine:
//!
//! 1. mede o tempo real que passou;
//! 2. executa [`Game::fixed_update`] zero ou mais vezes (timestep fixo);
//! 3. executa [`Game::update`] uma vez;
//! 4. executa [`Game::render`] e apresenta a imagem;
//! 5. limpa as transições de entrada ("apertou neste frame").

use std::sync::Arc;
use std::time::Instant;

use glam::{UVec2, Vec2};
use nn_audio::Audio;
use nn_core::{Color, FixedTimestep, Time};
use nn_input::Input;
use nn_render::{Camera2D, GpuContext, RenderError, Renderer};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

/// Configuração inicial da aplicação.
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Título da janela.
    pub title: String,
    /// Tamanho inicial da janela em pixels lógicos (respeita a escala do SO).
    pub size: (u32, u32),
    pub resizable: bool,
    /// Sincroniza com a taxa de atualização do monitor.
    pub vsync: bool,
    /// Passos de [`Game::fixed_update`] por segundo.
    pub fixed_hz: u32,
    /// Cor de fundo inicial.
    pub clear_color: Color,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            title: "nn-engine".into(),
            size: (1280, 720),
            resizable: true,
            vsync: true,
            fixed_hz: 60,
            clear_color: Color::hex(0x1E1E28),
        }
    }
}

impl AppConfig {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Self::default()
        }
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }

    pub fn vsync(mut self, vsync: bool) -> Self {
        self.vsync = vsync;
        self
    }

    pub fn fixed_hz(mut self, hz: u32) -> Self {
        self.fixed_hz = hz;
        self
    }

    pub fn clear_color(mut self, color: Color) -> Self {
        self.clear_color = color;
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }
}

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
    /// Câmera usada para desenhar o frame.
    pub camera: Camera2D,
    /// Cor de fundo.
    pub clear_color: Color,
    timestep: FixedTimestep,
    viewport: UVec2,
    exit_requested: bool,
}

impl Context {
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
}

/// Um jogo (ou qualquer aplicação) rodando na nn-engine.
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

/// Erros fatais da aplicação.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("falha no loop de eventos: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),
    #[error("falha ao criar a janela: {0}")]
    Window(#[from] winit::error::OsError),
    #[error("falha ao criar a superfície de desenho: {0}")]
    Surface(#[from] nn_render::wgpu::CreateSurfaceError),
    #[error(transparent)]
    Render(#[from] RenderError),
    #[error("a GPU não suporta desenhar nesta janela")]
    UnsupportedSurface,
}

/// Abre a janela e roda o jogo até o usuário fechar ou [`Context::exit`] ser chamado.
///
/// `create` é chamada uma vez, com a GPU pronta, e devolve o jogo. Carregue
/// texturas e prepare o estado inicial nela.
pub fn run<G, F>(config: AppConfig, create: F) -> Result<(), AppError>
where
    G: Game,
    F: FnOnce(&mut Context) -> G,
{
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let instance = nn_render::wgpu::Instance::new(
        nn_render::wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(
            event_loop.owned_display_handle(),
        )),
    );
    let mut app = App {
        config,
        create: Some(create),
        game: None,
        instance: Some(instance),
        state: None,
        error: None,
    };
    event_loop.run_app(&mut app)?;
    match app.error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

struct App<G, F> {
    config: AppConfig,
    create: Option<F>,
    game: Option<G>,
    instance: Option<nn_render::wgpu::Instance>,
    state: Option<State>,
    error: Option<AppError>,
}

struct State {
    window: Arc<Window>,
    surface: nn_render::wgpu::Surface<'static>,
    surface_config: nn_render::wgpu::SurfaceConfiguration,
    gpu: GpuContext,
    ctx: Context,
    last_frame: Instant,
}

impl<G: Game, F: FnOnce(&mut Context) -> G> App<G, F> {
    fn init(&mut self, event_loop: &ActiveEventLoop) -> Result<(), AppError> {
        use nn_render::wgpu;

        let instance = self
            .instance
            .take()
            .expect("init só roda uma vez por execução");
        let attrs = Window::default_attributes()
            .with_title(self.config.title.clone())
            .with_inner_size(LogicalSize::new(self.config.size.0, self.config.size.1))
            .with_resizable(self.config.resizable);
        let window = Arc::new(event_loop.create_window(attrs)?);
        let surface = instance.create_surface(window.clone())?;
        let gpu = pollster::block_on(GpuContext::new(instance, Some(&surface)))?;

        let caps = surface.get_capabilities(&gpu.adapter);
        // Formato sRGB: as cores saem iguais às do editor de imagem.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .or_else(|| caps.formats.first().copied())
            .ok_or(AppError::UnsupportedSurface)?;
        let size = window.inner_size();
        let mut surface_config = surface
            .get_default_config(&gpu.adapter, size.width.max(1), size.height.max(1))
            .ok_or(AppError::UnsupportedSurface)?;
        surface_config.format = format;
        surface_config.present_mode = if self.config.vsync {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        };
        surface.configure(&gpu.device, &surface_config);

        let renderer = Renderer::new(&gpu, format);
        let mut ctx = Context {
            input: Input::new(),
            time: Time::default(),
            renderer,
            audio: Audio::new(),
            camera: Camera2D::default(),
            clear_color: self.config.clear_color,
            timestep: FixedTimestep::from_hz(self.config.fixed_hz),
            viewport: UVec2::new(surface_config.width, surface_config.height),
            exit_requested: false,
        };
        let create = self.create.take().expect("o jogo só é criado uma vez");
        self.game = Some(create(&mut ctx));

        self.state = Some(State {
            window,
            surface,
            surface_config,
            gpu,
            ctx,
            last_frame: Instant::now(),
        });
        Ok(())
    }
}

impl State {
    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            // Janela minimizada: mantém a última configuração.
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface
            .configure(&self.gpu.device, &self.surface_config);
        self.ctx.viewport = UVec2::new(width, height);
    }

    fn frame<G: Game>(&mut self, game: &mut G) {
        use nn_render::wgpu::CurrentSurfaceTexture;

        let now = Instant::now();
        let dt = now - self.last_frame;
        self.last_frame = now;

        self.ctx.time.tick(dt);
        let steps = self.ctx.timestep.advance(dt);
        for _ in 0..steps {
            game.fixed_update(&mut self.ctx);
        }
        game.update(&mut self.ctx);

        let size = self.window.inner_size();
        if size.width > 0 && size.height > 0 {
            let (texture, reconfigure) = match self.surface.get_current_texture() {
                CurrentSurfaceTexture::Success(t) => (Some(t), false),
                CurrentSurfaceTexture::Suboptimal(t) => (Some(t), true),
                CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => (None, true),
                CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => (None, false),
                other => {
                    log::error!("falha ao obter a imagem da janela: {other:?}");
                    (None, false)
                }
            };

            if let Some(texture) = texture {
                let view = texture
                    .texture
                    .create_view(&nn_render::wgpu::TextureViewDescriptor::default());
                game.render(&mut self.ctx);
                let clear = self.ctx.clear_color;
                let camera = self.ctx.camera;
                let viewport = self.ctx.viewport;
                self.ctx
                    .renderer
                    .render(&view, viewport, &camera, Some(clear));
                self.window.pre_present_notify();
                self.gpu.queue.present(texture);
            }
            if reconfigure {
                self.resize(size.width, size.height);
            }
        }

        self.ctx.input.end_frame();
    }
}

impl<G: Game, F: FnOnce(&mut Context) -> G> ApplicationHandler for App<G, F> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() || self.error.is_some() {
            return;
        }
        if let Err(e) = self.init(event_loop) {
            log::error!("{e}");
            self.error = Some(e);
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let (Some(state), Some(game)) = (self.state.as_mut(), self.game.as_mut()) else {
            return;
        };
        state.ctx.input.process_event(&event);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state.frame(game);
                if state.ctx.exit_requested {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
}
