//! Janela e loop principal.

use std::sync::Arc;
use web_time::Instant;

use glam::UVec2;
use nn_audio::Audio;
use nn_render::{GpuContext, RenderError, Renderer};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Fullscreen, Window, WindowId};

use crate::context::{draw, simulate};
use crate::{AppConfig, Context, Game};

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
    focused: bool,
    /// O mouse está preso de verdade (pela janela) agora.
    grabbed: bool,
    /// A janela avisou que está minimizada (tamanho zero) ou toda coberta.
    hidden: bool,
    /// O jogo está parado por isso (o som segue).
    paused: bool,
    /// O intervalo entre quadros (o limite de quadros) e a hora marcada
    /// para o próximo. A hora anda de intervalo em intervalo, não a partir
    /// de quando o quadro começou de fato: um atraso do sistema num quadro é
    /// compensado no seguinte, e a média fica no limite pedido.
    frame_period: Option<std::time::Duration>,
    next_frame: Instant,
    #[cfg(feature = "gamepad")]
    gamepads: crate::gamepad::Gamepads,
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
            .with_resizable(self.config.resizable)
            .with_fullscreen(
                self.config
                    .fullscreen
                    .then_some(Fullscreen::Borderless(None)),
            );
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
        // Para as capturas da tela (o agente de IA vê o jogo aberto).
        if caps.usages.contains(wgpu::TextureUsages::COPY_SRC) {
            surface_config.usage |= wgpu::TextureUsages::COPY_SRC;
        }
        surface_config.present_mode = if self.config.vsync {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        };
        surface.configure(&gpu.device, &surface_config);

        let renderer = Renderer::new(&gpu, format);
        let viewport = UVec2::new(surface_config.width, surface_config.height);
        let save = self.config.open_storage();
        if let Some(path) = save.path() {
            log::info!("dados salvos em: {}", path.display());
        }
        let mut ctx = Context::new(&self.config, renderer, Audio::new(), viewport, save);
        log::info!("pasta de assets: {}", ctx.assets.root().display());
        #[cfg(feature = "gamepad")]
        let gamepads = crate::gamepad::Gamepads::new(&mut ctx.input);
        let create = self.create.take().expect("o jogo só é criado uma vez");
        self.game = Some(create(&mut ctx));

        self.state = Some(State {
            window,
            surface,
            surface_config,
            gpu,
            ctx,
            last_frame: Instant::now(),
            focused: true,
            grabbed: false,
            hidden: false,
            paused: false,
            frame_period: self.config.frame_period(),
            next_frame: Instant::now(),
            #[cfg(feature = "gamepad")]
            gamepads,
        });
        Ok(())
    }
}

impl State {
    /// Prende ou solta o mouse conforme o jogo pediu (e se a janela tem foco).
    fn apply_mouse_capture(&mut self) {
        use winit::window::CursorGrabMode;
        let want = self.ctx.mouse_captured && self.focused;
        if want == self.grabbed {
            return;
        }
        if want {
            // Preso no lugar; onde não dá (alguns sistemas), preso na janela.
            let ok = self
                .window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined));
            if let Err(e) = ok {
                log::warn!("não foi possível prender o mouse: {e}");
            }
            self.window.set_cursor_visible(false);
        } else {
            let _ = self.window.set_cursor_grab(CursorGrabMode::None);
            self.window.set_cursor_visible(true);
        }
        self.grabbed = want;
    }

    fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
    }

    /// Para o jogo com a janela minimizada (ou coberta) e o solta quando ela
    /// volta. Ao voltar, o tempo parado não conta: o jogo segue de onde
    /// estava, sem um salto.
    fn update_paused(&mut self) -> bool {
        let paused = self.hidden || self.window.is_minimized() == Some(true);
        if self.paused && !paused {
            self.last_frame = Instant::now();
            self.next_frame = self.last_frame;
        }
        self.paused = paused;
        paused
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            // Janela minimizada: mantém a última configuração.
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface
            .configure(&self.gpu.device, &self.surface_config);
        self.ctx.set_window_size(UVec2::new(width, height));
    }

    fn frame<G: Game>(&mut self, game: &mut G) {
        use nn_render::wgpu::CurrentSurfaceTexture;

        let now = Instant::now();
        let dt = now - self.last_frame;
        self.last_frame = now;
        if let Some(period) = self.frame_period {
            // Muito atrasado (uma travada): recomeça a contar daqui.
            self.next_frame = if now > self.next_frame + period {
                now + period
            } else {
                self.next_frame + period
            };
        }
        #[cfg(feature = "gamepad")]
        self.gamepads.poll(&mut self.ctx.input, self.focused);
        simulate(&mut self.ctx, game, dt);

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
                draw(&mut self.ctx, game, Some(&view));
                if std::mem::take(&mut self.ctx.screenshot_request) {
                    let shot = if self
                        .surface_config
                        .usage
                        .contains(wgpu::TextureUsages::COPY_SRC)
                    {
                        let size = UVec2::new(texture.texture.width(), texture.texture.height());
                        nn_render::read_texture_rgba(
                            &self.gpu,
                            &texture.texture,
                            size,
                            self.surface_config.format,
                        )
                        .map(|pixels| crate::Screenshot {
                            width: size.x,
                            height: size.y,
                            pixels,
                        })
                        .map_err(|e| e.to_string())
                    } else {
                        Err("esta placa de vídeo não deixa ler a imagem da janela".into())
                    };
                    self.ctx.screenshot = Some(shot);
                }
                self.window.pre_present_notify();
                self.gpu.queue.present(texture);
            }
            if reconfigure {
                self.resize(size.width, size.height);
            }
        }

        if let Some(fullscreen) = self.ctx.fullscreen_request.take() {
            self.window
                .set_fullscreen(fullscreen.then_some(Fullscreen::Borderless(None)));
        }
        self.ctx.fullscreen = self.window.fullscreen().is_some();
        self.apply_mouse_capture();
        self.ctx.save.auto_flush();
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
            WindowEvent::CloseRequested => {
                state.ctx.save.flush_logged();
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                state.set_hidden(size.width == 0 || size.height == 0);
                state.resize(size.width, size.height);
            }
            WindowEvent::Occluded(occluded) => state.set_hidden(occluded),
            WindowEvent::Focused(focused) => {
                state.focused = focused;
                // Fora da janela o mouse fica solto; de volta, preso de novo.
                state.apply_mouse_capture();
            }
            WindowEvent::RedrawRequested => {
                // Minimizada, o sistema ainda pode pedir um desenho: o jogo
                // continua parado.
                if state.update_paused() {
                    return;
                }
                state.frame(game);
                if state.ctx.exit_requested {
                    state.ctx.save.flush_logged();
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let (Some(state), winit::event::DeviceEvent::MouseMotion { delta }) =
            (self.state.as_mut(), event)
            && state.focused
        {
            state
                .ctx
                .input
                .add_mouse_motion(glam::Vec2::new(delta.0 as f32, delta.1 as f32));
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(state) = &mut self.state else {
            return;
        };
        // Minimizada: confere 10 vezes por segundo se voltou, sem gastar o
        // processador simulando e desenhando o que ninguém vê.
        if state.update_paused() {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + std::time::Duration::from_millis(100),
            ));
            return;
        }
        // O limite de quadros: dorme até a hora do próximo.
        if state.frame_period.is_some() && Instant::now() < state.next_frame {
            event_loop.set_control_flow(ControlFlow::WaitUntil(state.next_frame));
            return;
        }
        event_loop.set_control_flow(ControlFlow::Poll);
        state.window.request_redraw();
    }
}
