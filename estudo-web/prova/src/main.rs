//! Prova da nn-engine no navegador: a janela (um canvas), o desenho 2D do
//! nn-render (retângulos, imagem, texto), o teclado e (com a feature `som`)
//! o som do nn-audio. Também roda no desktop, para comparar.
//!
//! Na web, o endereço escolhe o modo: `?webgpu` usa WebGPU (senão, WebGL2) e
//! `?carga=2000` desenha mais 2000 imagens por quadro. Tudo o que o teste do
//! navegador precisa ler sai no console, começando com "NN ".

use std::collections::HashSet;
use std::sync::Arc;

use glam::{UVec2, Vec2};
use nn_core::Color;
use nn_render::wgpu;
use nn_render::{Camera2D, GpuContext, Renderer, Sprite, Text, TextureId, TextureOptions};
use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

struct Estado {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    gpu: GpuContext,
    renderer: Renderer,
    moeda: TextureId,
    pos: Vec2,
    teclas: HashSet<KeyCode>,
    quadros: u64,
    inicio: Instant,
    ultimo: Instant,
    /// Tempo de CPU para montar e enviar os últimos quadros (ms).
    tempos: Vec<f32>,
    carga: u32,
    desenho: wgpu::TextureFormat,
    cubo: Option<nn_render::d3::MeshId>,
    #[cfg(feature = "som")]
    audio: Option<(nn_audio::Audio, nn_audio::SoundId)>,
}

enum Evento {
    Pronto(Box<Estado>),
    Erro(String),
}

struct App {
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    proxy: EventLoopProxy<Evento>,
    estado: Option<Box<Estado>>,
    pediu: bool,
    backends: wgpu::Backends,
    carga: u32,
    comeco: Instant,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    limite: Option<u64>,
}

async fn iniciar(
    window: Arc<Window>,
    backends: wgpu::Backends,
    carga: u32,
) -> Result<Estado, String> {
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = backends;
    let instance = wgpu::Instance::new(desc);
    let surface = instance
        .create_surface(window.clone())
        .map_err(|e| format!("create_surface: {e}"))?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
            ..Default::default()
        })
        .await
        .map_err(|e| format!("request_adapter: {e}"))?;
    let info = adapter.get_info();
    log::info!(
        "NN gpu {} backend {:?} driver {} {}",
        info.name,
        info.backend,
        info.driver,
        info.driver_info
    );
    // Os limites que a engine pede hoje (GpuContext::new) servem aqui?
    let engine = wgpu::Limits::downlevel_defaults();
    let servem = engine.check_limits(&adapter.limits());
    log::info!("NN limites da engine servem: {servem}");
    let limites = if servem {
        engine
    } else {
        wgpu::Limits::downlevel_webgl2_defaults()
    }
    .using_resolution(adapter.limits());
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("prova"),
            required_limits: limites,
            ..Default::default()
        })
        .await
        .map_err(|e| format!("request_device: {e}"))?;
    let gpu = GpuContext {
        instance,
        adapter,
        device,
        queue,
    };
    let caps = surface.get_capabilities(&gpu.adapter);
    log::info!("NN formatos {:?}", caps.formats);
    let format = caps
        .formats
        .iter()
        .copied()
        .find(wgpu::TextureFormat::is_srgb)
        .or_else(|| caps.formats.first().copied())
        .ok_or("a superfície não tem formatos")?;
    let size = window.inner_size();
    let mut config = surface
        .get_default_config(&gpu.adapter, size.width.max(1), size.height.max(1))
        .ok_or("a superfície não tem configuração")?;
    config.format = format;
    // O canvas do WebGPU não tem formato sRGB: desenha numa vista sRGB dele.
    let desenho = format.add_srgb_suffix();
    if desenho != format {
        config.view_formats = vec![desenho];
    }
    surface.configure(&gpu.device, &config);
    log::info!(
        "NN superficie {}x{} {format:?}",
        config.width,
        config.height
    );
    let mut renderer = Renderer::new(&gpu, desenho);
    let moeda = renderer
        .load_texture_bytes(include_bytes!("moeda.png"), TextureOptions::PIXEL_ART)
        .map_err(|e| format!("textura: {e}"))?;
    // `?3d`: um cubo girando com o céu estrelado (as pipelines 3D).
    let cubo = busca().contains("3d").then(|| {
        renderer.set_sky3d(Some(nn_render::d3::Sky3D {
            stars: 1.0,
            ..Default::default()
        }));
        renderer.create_mesh(&nn_render::d3::shapes::cube())
    });
    let agora = Instant::now();
    Ok(Estado {
        window,
        surface,
        config,
        gpu,
        renderer,
        moeda,
        desenho,
        cubo,
        pos: Vec2::new(100.0, 200.0),
        teclas: HashSet::new(),
        quadros: 0,
        inicio: agora,
        ultimo: agora,
        tempos: Vec::new(),
        carga,
        #[cfg(feature = "som")]
        audio: None,
    })
}

impl Estado {
    fn redimensionar(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.gpu.device, &self.config);
    }

    fn quadro(&mut self) {
        use wgpu::CurrentSurfaceTexture as C;
        let agora = Instant::now();
        let dt = (agora - self.ultimo).as_secs_f32().min(0.1);
        self.ultimo = agora;
        let mut v = Vec2::ZERO;
        for (k, d) in [
            (KeyCode::ArrowRight, Vec2::X),
            (KeyCode::ArrowLeft, -Vec2::X),
            (KeyCode::ArrowDown, Vec2::Y),
            (KeyCode::ArrowUp, -Vec2::Y),
        ] {
            if self.teclas.contains(&k) {
                v += d;
            }
        }
        self.pos += v * 240.0 * dt;

        let texture = match self.surface.get_current_texture() {
            C::Success(t) | C::Suboptimal(t) => t,
            C::Outdated | C::Lost => {
                let (w, h) = (self.config.width, self.config.height);
                self.redimensionar(w, h);
                return;
            }
            other => {
                log::warn!("NN sem imagem da janela: {other:?}");
                return;
            }
        };
        let view = texture.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.desenho),
            ..Default::default()
        });
        if let Some(cubo) = self.cubo {
            let t = self.inicio.elapsed().as_secs_f32();
            let modelo = glam::Mat4::from_rotation_y(t) * glam::Mat4::from_rotation_x(t * 0.6);
            self.renderer.draw_mesh(
                cubo,
                modelo * glam::Mat4::from_scale(glam::Vec3::splat(2.0)),
                Color::hex(0x4D9BE5),
            );
        }
        let tamanho = UVec2::new(self.config.width, self.config.height);
        let t = self.inicio.elapsed().as_secs_f32();
        let info = format!(
            "quadro {}  setas: mover  clique: som  carga: {}",
            self.quadros, self.carga
        );
        {
            let mut s = self.renderer.screen();
            for i in 0..self.carga {
                let f = i as f32;
                s.draw(
                    &Sprite::new(self.moeda)
                        .at(Vec2::new((f * 37.0 + t * 50.0) % 780.0, (f * 13.0) % 420.0))
                        .size(Vec2::new(32.0, 8.0)),
                );
            }
            s.draw_rect(
                Vec2::new(16.0, 16.0),
                Vec2::new(330.0, 44.0),
                Color::hex(0x2A3446),
            );
            s.draw_text(
                &Text::new("nn-engine no navegador")
                    .at(Vec2::new(28.0, 26.0))
                    .size(24.0)
                    .color(Color::WHITE),
            );
            s.draw(
                &Sprite::new(self.moeda)
                    .at(Vec2::new(420.0 + (t * 2.0).sin() * 60.0, 120.0))
                    .size(Vec2::new(192.0, 48.0)),
            );
            s.draw_rect(self.pos, Vec2::splat(48.0), Color::hex(0xE5A84D));
            s.draw_text(
                &Text::new(&info)
                    .at(Vec2::new(16.0, 410.0))
                    .size(16.0)
                    .color(Color::hex(0xC8D0E0)),
            );
        }
        self.renderer.render(
            &view,
            tamanho,
            &Camera2D::default(),
            Some(Color::hex(0x141A26)),
        );
        self.window.pre_present_notify();
        self.gpu.queue.present(texture);
        self.tempos.push(agora.elapsed().as_secs_f32() * 1000.0);
        self.quadros += 1;
        if self.quadros % 60 == 0 {
            let n = self.tempos.len() as f32;
            let media = self.tempos.iter().sum::<f32>() / n;
            let pior = self.tempos.iter().copied().fold(0.0, f32::max);
            log::info!(
                "NN quadro {} pos {:.0} {:.0} cpu_ms {media:.2} pior {pior:.2} desde_inicio_s {:.2}",
                self.quadros,
                self.pos.x,
                self.pos.y,
                self.inicio.elapsed().as_secs_f32()
            );
            self.tempos.clear();
        }
    }

    #[cfg(feature = "som")]
    fn tocar(&mut self) {
        use nn_audio::synth::{SAMPLE_RATE, Tone, Wave};
        let (audio, id) = self.audio.get_or_insert_with(|| {
            let mut audio = nn_audio::Audio::new();
            let amostras = Tone::new(Wave::Square, 660.0)
                .slide_to(990.0)
                .duration(0.2)
                .volume(0.3)
                .render();
            let id = audio.add_samples(SAMPLE_RATE, &amostras);
            log::info!("NN som ligado {}", audio.is_enabled());
            (audio, id)
        });
        let _ = audio.play(*id);
        log::info!("NN som tocou");
    }

    #[cfg(not(feature = "som"))]
    fn tocar(&mut self) {
        log::info!("NN som fora deste build");
    }
}

impl ApplicationHandler<Evento> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.pediu {
            return;
        }
        self.pediu = true;
        let attrs = Window::default_attributes()
            .with_title("nn-engine web")
            .with_inner_size(LogicalSize::new(800.0, 450.0));
        #[cfg(target_arch = "wasm32")]
        let attrs = {
            use winit::platform::web::WindowAttributesExtWebSys;
            attrs.with_append(true)
        };
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                log::error!("NN erro janela: {e}");
                return;
            }
        };
        let (backends, carga) = (self.backends, self.carga);
        #[cfg(target_arch = "wasm32")]
        {
            let proxy = self.proxy.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let ev = match iniciar(window, backends, carga).await {
                    Ok(e) => Evento::Pronto(Box::new(e)),
                    Err(e) => Evento::Erro(e),
                };
                let _ = proxy.send_event(ev);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let ev = match pollster::block_on(iniciar(window, backends, carga)) {
                Ok(e) => Evento::Pronto(Box::new(e)),
                Err(e) => Evento::Erro(e),
            };
            self.user_event(event_loop, ev);
        }
    }

    fn user_event(&mut self, _: &ActiveEventLoop, ev: Evento) {
        match ev {
            Evento::Pronto(e) => {
                log::info!(
                    "NN pronto em {:.0} ms",
                    self.comeco.elapsed().as_secs_f32() * 1000.0
                );
                e.window.request_redraw();
                self.estado = Some(e);
            }
            Evento::Erro(e) => log::error!("NN erro {e}"),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(e) = &mut self.estado else {
            if matches!(event, WindowEvent::CloseRequested) {
                event_loop.exit();
            }
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(s) => e.redimensionar(s.width, s.height),
            WindowEvent::KeyboardInput { event: k, .. } => {
                if let PhysicalKey::Code(code) = k.physical_key {
                    if k.state == ElementState::Pressed {
                        if e.teclas.insert(code) {
                            log::info!("NN tecla {code:?}");
                        }
                    } else {
                        e.teclas.remove(&code);
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                log::info!("NN clique");
                e.tocar();
            }
            WindowEvent::RedrawRequested => {
                e.quadro();
                #[cfg(not(target_arch = "wasm32"))]
                if self.limite.is_some_and(|n| e.quadros >= n) {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(e) = &self.estado {
            e.window.request_redraw();
        }
    }
}

/// O endereço da página (na web) ou a variável PROVA (no desktop).
fn busca() -> String {
    #[cfg(target_arch = "wasm32")]
    let busca = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default();
    #[cfg(not(target_arch = "wasm32"))]
    let busca = std::env::var("PROVA").unwrap_or_default();
    busca
}

fn opcoes() -> (wgpu::Backends, u32) {
    let busca = busca();
    let backends = if busca.contains("webgpu") {
        wgpu::Backends::BROWSER_WEBGPU
    } else if cfg!(target_arch = "wasm32") {
        wgpu::Backends::GL
    } else {
        wgpu::Backends::PRIMARY | wgpu::Backends::GL
    };
    let carga = busca
        .split(['?', '&'])
        .find_map(|p| p.strip_prefix("carga="))
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    (backends, carga)
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        let _ = console_log::init_with_level(log::Level::Info);
    }
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let event_loop = match EventLoop::<Evento>::with_user_event().build() {
        Ok(e) => e,
        Err(e) => {
            log::error!("NN erro event loop: {e}");
            return;
        }
    };
    let (backends, carga) = opcoes();
    let app = App {
        proxy: event_loop.create_proxy(),
        estado: None,
        pediu: false,
        backends,
        carga,
        comeco: Instant::now(),
        limite: std::env::var("PROVA_QUADROS")
            .ok()
            .and_then(|n| n.parse().ok()),
    };
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(app);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = app;
        if let Err(e) = event_loop.run_app(&mut app) {
            log::error!("NN erro {e}");
        }
    }
}
