//! A página: desenha a cena `?cena=N` num canvas do tamanho das cenas.

use std::sync::Arc;

use nn_render::wgpu;
use nn_render::{Camera2D, GpuContext, Renderer};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::platform::web::{EventLoopExtWebSys, WindowAttributesExtWebSys};
use winit::window::{Window, WindowId};

use crate::cenas::{self, Recursos, SIZE};

struct Estado {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    gpu: GpuContext,
    renderer: Renderer,
    recursos: Recursos,
    view_format: wgpu::TextureFormat,
    quadros: u32,
}

enum Evento {
    Pronto(Box<Estado>),
    Erro(String),
}

struct App {
    proxy: EventLoopProxy<Evento>,
    estado: Option<Box<Estado>>,
    pediu: bool,
    cena: usize,
    backends: wgpu::Backends,
}

async fn iniciar(window: Arc<Window>, backends: wgpu::Backends) -> Result<Estado, String> {
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = backends;
    let instance = wgpu::Instance::new(desc);
    let surface = instance
        .create_surface(window.clone())
        .map_err(|e| format!("create_surface: {e}"))?;
    let gpu = GpuContext::new(instance, Some(&surface))
        .await
        .map_err(|e| e.to_string())?;
    let info = gpu.adapter.get_info();
    log::info!("NN gpu {} {:?}", info.name, info.backend);
    let caps = surface.get_capabilities(&gpu.adapter);
    let format = caps
        .formats
        .iter()
        .copied()
        .find(wgpu::TextureFormat::is_srgb)
        .or_else(|| caps.formats.first().copied())
        .ok_or("a superfície não tem formatos")?;
    let mut config = surface
        .get_default_config(&gpu.adapter, SIZE.x, SIZE.y)
        .ok_or("a superfície não tem configuração")?;
    config.format = format;
    // O canvas do WebGPU não tem formato sRGB: desenha numa vista sRGB dele.
    let view_format = format.add_srgb_suffix();
    if view_format != format {
        config.view_formats = vec![view_format];
    }
    surface.configure(&gpu.device, &config);
    let mut renderer = Renderer::new(&gpu, view_format);
    let recursos = Recursos::new(&mut renderer);
    Ok(Estado {
        window,
        surface,
        config,
        gpu,
        renderer,
        recursos,
        view_format,
        quadros: 0,
    })
}

impl Estado {
    fn quadro(&mut self, cena: usize) {
        use wgpu::CurrentSurfaceTexture as C;
        let size = self.window.inner_size();
        if size.width != SIZE.x || size.height != SIZE.y {
            // Ainda não está no tamanho: espera o Resized.
            return;
        }
        let texture = match self.surface.get_current_texture() {
            C::Success(t) | C::Suboptimal(t) => t,
            _ => {
                self.surface.configure(&self.gpu.device, &self.config);
                return;
            }
        };
        let view = texture.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.view_format),
            ..Default::default()
        });
        cenas::desenhar(cena, &mut self.renderer, &self.recursos);
        self.renderer
            .render(&view, SIZE, &Camera2D::default(), Some(cenas::fundo(cena)));
        self.window.pre_present_notify();
        self.gpu.queue.present(texture);
        self.quadros += 1;
        if self.quadros == 5 {
            log::info!("NN pronto {cena}");
        }
    }
}

impl ApplicationHandler<Evento> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.pediu {
            return;
        }
        self.pediu = true;
        let attrs = Window::default_attributes()
            .with_title("render-web")
            .with_inner_size(LogicalSize::new(SIZE.x as f64, SIZE.y as f64))
            .with_append(true);
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                log::error!("NN erro janela: {e}");
                return;
            }
        };
        let proxy = self.proxy.clone();
        let backends = self.backends;
        wasm_bindgen_futures::spawn_local(async move {
            let ev = match iniciar(window, backends).await {
                Ok(e) => Evento::Pronto(Box::new(e)),
                Err(e) => Evento::Erro(e),
            };
            let _ = proxy.send_event(ev);
        });
    }

    fn user_event(&mut self, _: &ActiveEventLoop, ev: Evento) {
        match ev {
            Evento::Pronto(e) => {
                e.window.request_redraw();
                self.estado = Some(e);
            }
            Evento::Erro(e) => log::error!("NN erro {e}"),
        }
    }

    fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(e) = &mut self.estado else {
            return;
        };
        if let WindowEvent::RedrawRequested = event {
            e.quadro(self.cena);
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(e) = &self.estado {
            e.window.request_redraw();
        }
    }
}

pub fn main() {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);
    let busca = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default();
    let cena = busca
        .split(['?', '&'])
        .find_map(|p| p.strip_prefix("cena="))
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    let backends = if busca.contains("webgpu") {
        wgpu::Backends::BROWSER_WEBGPU
    } else {
        wgpu::Backends::GL
    };
    let event_loop = match EventLoop::<Evento>::with_user_event().build() {
        Ok(e) => e,
        Err(e) => {
            log::error!("NN erro event loop: {e}");
            return;
        }
    };
    let app = App {
        proxy: event_loop.create_proxy(),
        estado: None,
        pediu: false,
        cena,
        backends,
    };
    event_loop.spawn_app(app);
}
