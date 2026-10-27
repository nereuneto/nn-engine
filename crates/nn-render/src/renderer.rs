//! Renderer de sprites em lote.

use std::path::Path;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, UVec2, Vec2, Vec3};

use crate::effects::{EffectError, EffectGpu, EffectId, EffectShader};
use nn_core::Color;

use crate::text::{FontId, Text, TextSystem};
use crate::texture::GpuTexture;
use crate::{
    Blend, Camera2D, GpuContext, RenderError, Sprite, SpriteSheet, TextureId, TextureOptions,
};

/// Dados de um sprite como a GPU os lê (um por instância).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct SpriteInstance {
    position: [f32; 2],
    size: [f32; 2],
    origin: [f32; 2],
    rotation: f32,
    region: [f32; 4],
    color: [f32; 4],
    /// Quantas vezes a região se repete em cada eixo (1 = uma vez, esticada).
    repeat: [f32; 2],
    /// O deslocamento dentro das repetições.
    offset: [f32; 2],
    /// 1 = a textura é um canvas (as cores guardadas com o alpha já
    /// multiplicado): o shader divide pelo alpha ao ler.
    flags: f32,
}

impl SpriteInstance {
    const ATTRIBUTES: [wgpu::VertexAttribute; 9] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x2,
        3 => Float32,
        4 => Float32x4,
        5 => Float32x4,
        6 => Float32x2,
        7 => Float32x2,
        8 => Float32,
    ];

    pub(crate) fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

/// Uniform de cada espaço de desenho (mundo ou tela).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ViewUniform {
    view_proj: [f32; 16],
    /// 1.0 quando o Y cresce para baixo (espaço da tela).
    y_down: f32,
    _padding: [f32; 3],
}

/// Sequência de instâncias consecutivas com a mesma textura e mistura: vira
/// uma chamada de desenho.
#[derive(Debug, Clone, Copy)]
struct Batch {
    texture: TextureId,
    blend: Blend,
    effect: Option<EffectId>,
    /// O recorte (no espaço da fila), se houver.
    clip: Option<ClipRect>,
    start: u32,
    end: u32,
}

/// Um retângulo de recorte: os cantos de menor e de maior x e y, no espaço
/// em que se desenha (mundo ou tela).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipRect {
    pub min: Vec2,
    pub max: Vec2,
}

impl ClipRect {
    /// O retângulo com um canto em `a` e o oposto em `b` (em qualquer ordem).
    pub fn new(a: Vec2, b: Vec2) -> Self {
        Self {
            min: a.min(b),
            max: a.max(b),
        }
    }

    /// O pedaço em comum com `other` (pode ficar vazio).
    pub fn intersect(self, other: ClipRect) -> ClipRect {
        let min = self.min.max(other.min);
        Self {
            min,
            max: self.max.max(min).min(other.max).max(min),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.max.x <= self.min.x || self.max.y <= self.min.y
    }
}

#[derive(Debug, Default)]
struct DrawQueue {
    instances: Vec<SpriteInstance>,
    batches: Vec<Batch>,
    /// O recorte do que entra agora.
    clip: Option<ClipRect>,
}

impl DrawQueue {
    fn push(&mut self, texture: TextureId, blend: Blend, instance: SpriteInstance) {
        self.push_with(texture, blend, None, instance);
    }

    fn push_with(
        &mut self,
        texture: TextureId,
        blend: Blend,
        effect: Option<EffectId>,
        instance: SpriteInstance,
    ) {
        let index = self.instances.len() as u32;
        self.instances.push(instance);
        match self.batches.last_mut() {
            Some(batch)
                if batch.texture == texture
                    && batch.blend == blend
                    && batch.effect == effect
                    && batch.clip == self.clip =>
            {
                batch.end = index + 1;
            }
            _ => self.batches.push(Batch {
                texture,
                blend,
                effect,
                clip: self.clip,
                start: index,
                end: index + 1,
            }),
        }
    }

    fn clear(&mut self) {
        self.instances.clear();
        self.batches.clear();
        self.clip = None;
    }
}

fn color_bits(c: Color) -> [u32; 4] {
    [c.r.to_bits(), c.g.to_bits(), c.b.to_bits(), c.a.to_bits()]
}

/// Para onde vai um passo de desenho.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pass {
    /// O quadro, no alvo dele.
    Main,
    /// Um canvas do jogo (só o 2D).
    Canvas,
    /// O quadro inteiro num canvas, antes do efeito da tela.
    Offscreen,
}

/// Mistura aditiva: soma a cor ao fundo.
const ADDITIVE_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

/// Num canvas: a cor como na mistura normal (com o alpha multiplicado, sobre
/// o transparente) e o alpha somado, para guardar a cor com o alpha já
/// multiplicado.
const CANVAS_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
};

/// Aditiva num canvas: soma a cor e o alpha.
const CANVAS_ADDITIVE_BLEND: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

/// O quadro guardado enquanto se desenha num canvas.
#[derive(Debug)]
struct CanvasFrame {
    canvas: TextureId,
    world: DrawQueue,
    screen: DrawQueue,
    presentation: Option<crate::Presentation>,
}

/// Espaço de coordenadas em que algo é desenhado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Space {
    /// Coordenadas do mundo, vistas pela câmera. Y para cima.
    World,
    /// Pixels da tela, origem no canto superior esquerdo. Y para baixo.
    Screen,
}

/// Números do último frame, úteis para medir desempenho.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RenderStats {
    /// Sprites desenhados (cada letra de um texto conta como um).
    pub sprites: u32,
    /// Chamadas de desenho enviadas à GPU (uma por troca de textura).
    pub draw_calls: u32,
}

#[derive(Debug)]
struct View {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

/// Renderer 2D: acumula sprites e textos durante o frame e desenha tudo de uma vez.
///
/// Há dois espaços de desenho:
/// - **mundo** (métodos diretos, como [`Renderer::draw`]): coordenadas do jogo,
///   vistas pela câmera, com Y para cima;
/// - **tela** ([`Renderer::screen`]): pixels da janela, origem no canto
///   superior esquerdo, Y para baixo. Ideal para HUD e menus. É desenhado por
///   cima do mundo e não é afetado pela câmera.
///
/// Dentro de cada espaço, o que é enviado por último fica por cima. Itens
/// consecutivos com a mesma textura são agrupados em uma única chamada de
/// desenho, então agrupe por textura sempre que possível.
#[derive(Debug)]
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    /// Os mesmos, para desenhar num canvas (veja [`CANVAS_BLEND`]).
    canvas_pipeline: wgpu::RenderPipeline,
    canvas_additive_pipeline: wgpu::RenderPipeline,
    additive_pipeline: wgpu::RenderPipeline,
    world_view: View,
    screen_view: View,
    texture_layout: wgpu::BindGroupLayout,
    textures: Vec<GpuTexture>,
    /// Texturas liberadas ([`Renderer::free_texture`]): o lugar delas fica
    /// com uma de 1×1 transparente e é reaproveitado pela próxima criada.
    freed: std::collections::BTreeSet<u32>,
    white: TextureId,
    /// Os triângulos de uma forma, reaproveitados de uma para outra.
    tri_buf: Vec<crate::shapes2d::Tri>,
    /// Os canvas sendo desenhados (o de cima é o atual), com o que foi
    /// guardado do quadro.
    canvas_stack: Vec<CanvasFrame>,
    /// O que os canvas desenharam neste quadro (vai para as estatísticas).
    canvas_stats: RenderStats,
    /// Os efeitos (shaders) e as camadas de que eles precisam.
    effects: EffectGpu,
    view_layout: wgpu::BindGroupLayout,
    /// O efeito na tela inteira e o canvas em que o quadro é desenhado antes.
    screen_effect: Option<EffectId>,
    screen_canvas: Option<(TextureId, UVec2)>,
    screen_effect_skipped: bool,
    /// Para montar os sprites de um painel sem alocar a cada um.
    panel_buf: Vec<Sprite>,
    /// A última cor convertida para a GPU (veja `gpu_color`).
    last_color: std::cell::Cell<([u32; 4], Color)>,
    circle: TextureId,
    /// Texturas dos painéis arredondados: (raio, borda) → (fundo, borda, margem).
    rounded: std::collections::HashMap<(u32, u32), (TextureId, TextureId, u32)>,
    text: TextSystem,
    world: DrawQueue,
    screen: DrawQueue,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    last_stats: RenderStats,
    /// Tamanho fixo do jogo dentro do alvo (ou `None`: o alvo inteiro).
    presentation: Option<crate::Presentation>,
    /// O mundo 3D, desenhado antes do 2D.
    d3: crate::d3::Renderer3D,
}

/// Um modelo como ele é: sem brilho a mais, com o brilho do arquivo.
const MODEL_GLOW: crate::d3::Glow = crate::d3::Glow {
    amount: 0.0,
    color: Color::WHITE,
};

impl Renderer {
    const INITIAL_CAPACITY: usize = 1024;

    /// Cria o renderer para desenhar em alvos com o formato `format`.
    pub fn new(gpu: &GpuContext, format: wgpu::TextureFormat) -> Self {
        let independent_blend = gpu
            .adapter
            .get_downlevel_capabilities()
            .flags
            .contains(wgpu::DownlevelFlags::INDEPENDENT_BLEND);
        // O antisserrilhado do 3D: 4 amostras na cor, no brilho e na
        // profundidade, e resolver as duas cores.
        let msaa = |f: wgpu::TextureFormat, resolve: bool| {
            let flags = gpu.adapter.get_texture_format_features(f).flags;
            flags.sample_count_supported(4)
                && (!resolve
                    || flags.contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_RESOLVE))
        };
        let msaa_ok = msaa(format, true)
            && msaa(crate::d3::GLOW_FORMAT, true)
            && msaa(crate::d3::DEPTH_FORMAT, false);
        Self::build(&gpu.device, &gpu.queue, format, independent_blend, msaa_ok)
    }

    /// Cria o renderer sobre um dispositivo que já existe (por exemplo, o de
    /// outra biblioteca de interface, como o editor faz para mostrar a cena).
    pub fn from_device(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        Self::build(device, queue, format, true, true)
    }

    /// `independent_blend`: a placa mistura cada alvo de um jeito (o WebGL2
    /// e o OpenGL ES 3.0 não).
    fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        independent_blend: bool,
        msaa_ok: bool,
    ) -> Self {
        let device = device.clone();
        let queue = queue.clone();

        let shader = device.create_shader_module(wgpu::include_wgsl!("sprite.wgsl"));

        let view_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nn view"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nn sprite texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let make_view = |label: &str| {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size_of::<ViewUniform>() as wgpu::BufferAddress,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &view_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });
            View { buffer, bind_group }
        };
        let world_view = make_view("nn world view");
        let screen_view = make_view("nn screen view");

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nn sprite"),
            bind_group_layouts: &[Some(&view_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let make_pipeline = |label: &str, blend: wgpu::BlendState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(SpriteInstance::layout())],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    // Sem descarte de faces: tamanho negativo espelha o sprite.
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let pipeline = make_pipeline("nn sprite", wgpu::BlendState::ALPHA_BLENDING);
        // Aditiva: soma a cor ao fundo (brilho, fogo, faíscas).
        let additive_pipeline = make_pipeline("nn sprite aditivo", ADDITIVE_BLEND);
        // Num canvas, a cor fica com o alpha multiplicado (o translúcido
        // sobre o transparente sai certo depois).
        let canvas_pipeline = make_pipeline("nn sprite canvas", CANVAS_BLEND);
        let canvas_additive_pipeline =
            make_pipeline("nn sprite canvas aditivo", CANVAS_ADDITIVE_BLEND);

        let instance_buffer = Self::create_instance_buffer(&device, Self::INITIAL_CAPACITY);

        let d3 = crate::d3::Renderer3D::new(
            &device,
            format,
            &texture_layout,
            independent_blend,
            msaa_ok,
        );
        let effects = EffectGpu::new(&device);
        let mut renderer = Self {
            device,
            queue,
            format,
            pipeline,
            additive_pipeline,
            canvas_pipeline,
            canvas_additive_pipeline,
            world_view,
            screen_view,
            texture_layout,
            textures: Vec::new(),
            white: TextureId(0),
            tri_buf: Vec::new(),
            canvas_stack: Vec::new(),
            canvas_stats: RenderStats::default(),
            effects,
            view_layout,
            screen_effect: None,
            screen_canvas: None,
            screen_effect_skipped: false,
            panel_buf: Vec::new(),
            freed: Default::default(),
            last_color: std::cell::Cell::new((color_bits(Color::WHITE), Color::WHITE)),
            circle: TextureId(0),
            rounded: std::collections::HashMap::new(),
            text: TextSystem::new(),
            world: DrawQueue::default(),
            screen: DrawQueue::default(),
            instance_buffer,
            instance_capacity: Self::INITIAL_CAPACITY,
            last_stats: RenderStats::default(),
            presentation: None,
            d3,
        };
        renderer.white =
            renderer.create_texture(UVec2::ONE, &[255, 255, 255, 255], TextureOptions::default());
        let (size, pixels) = circle_pixels(128);
        renderer.circle = renderer.create_texture(size, &pixels, TextureOptions::default());
        renderer
    }

    fn create_instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nn sprite instances"),
            size: (capacity * size_of::<SpriteInstance>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Formato dos alvos em que este renderer desenha.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    // ------------------------------------------------------------------
    // Texturas
    // ------------------------------------------------------------------

    /// Textura de um círculo branco com borda suave (usada por `draw_circle`
    /// e como padrão das partículas).
    pub fn circle_texture(&self) -> TextureId {
        self.circle
    }

    /// Textura branca de 1×1, para desenhar retângulos de cor sólida.
    pub fn white_texture(&self) -> TextureId {
        self.white
    }

    /// Cria uma textura a partir de pixels RGBA (8 bits por canal, sRGB),
    /// linha a linha a partir do topo da imagem.
    ///
    /// # Panics
    /// Se `rgba` não tiver exatamente `size.x * size.y * 4` bytes ou se o
    /// tamanho for zero.
    pub fn create_texture(
        &mut self,
        size: UVec2,
        rgba: &[u8],
        options: TextureOptions,
    ) -> TextureId {
        assert!(size.x > 0 && size.y > 0, "textura com tamanho zero");
        assert_eq!(
            rgba.len(),
            (size.x * size.y * 4) as usize,
            "quantidade de bytes não confere com o tamanho da textura"
        );
        let texture = GpuTexture::from_rgba8(
            &self.device,
            &self.queue,
            &self.texture_layout,
            size,
            rgba,
            options,
            None,
        );
        if let Some(i) = self.freed.pop_first() {
            self.textures[i as usize] = texture;
            return TextureId(i);
        }
        self.textures.push(texture);
        TextureId(self.textures.len() as u32 - 1)
    }

    /// Libera a memória de uma textura (de uma imagem que ninguém usa mais).
    /// O id passa a desenhar nada (uma textura de 1×1 transparente) até ser
    /// reaproveitado por uma textura nova. As da engine (o branco, o círculo,
    /// as letras) não são liberadas.
    pub fn free_texture(&mut self, id: TextureId) {
        let i = id.0 as usize;
        if i >= self.textures.len() || self.freed.contains(&id.0) || self.is_builtin(id) {
            return;
        }
        self.textures[i] = GpuTexture::from_rgba8(
            &self.device,
            &self.queue,
            &self.texture_layout,
            UVec2::ONE,
            &[0, 0, 0, 0],
            TextureOptions::default(),
            None,
        );
        self.freed.insert(id.0);
    }

    /// Texturas da própria engine (que [`Renderer::free_texture`] não libera).
    fn is_builtin(&self, id: TextureId) -> bool {
        id == self.white
            || id == self.circle
            || self.text.owns(id)
            || self.rounded.values().any(|(a, b, _)| *a == id || *b == id)
    }

    /// Substitui os pixels de uma região de uma textura existente.
    ///
    /// # Panics
    /// Se a região sair da textura ou `rgba` não tiver `size.x * size.y * 4` bytes.
    pub fn update_texture(&mut self, id: TextureId, origin: UVec2, size: UVec2, rgba: &[u8]) {
        let texture = &self.textures[id.0 as usize];
        assert!(
            origin.x + size.x <= texture.size.x && origin.y + size.y <= texture.size.y,
            "região fora da textura"
        );
        assert_eq!(rgba.len(), (size.x * size.y * 4) as usize);
        texture.write(&self.queue, origin, size, rgba);
    }

    /// Troca todo o conteúdo de uma textura (inclusive o tamanho), mantendo o
    /// mesmo [`TextureId`]. Quem já usa o id passa a desenhar a imagem nova.
    /// É assim que funciona a recarga de imagens ao salvar o arquivo.
    ///
    /// # Panics
    /// Nas mesmas condições de [`Renderer::create_texture`].
    pub fn replace_texture(
        &mut self,
        id: TextureId,
        size: UVec2,
        rgba: &[u8],
        options: TextureOptions,
    ) {
        assert!(size.x > 0 && size.y > 0, "textura com tamanho zero");
        assert_eq!(rgba.len(), (size.x * size.y * 4) as usize);
        self.textures[id.0 as usize] = GpuTexture::from_rgba8(
            &self.device,
            &self.queue,
            &self.texture_layout,
            size,
            rgba,
            options,
            None,
        );
    }

    /// Versão de [`Renderer::replace_texture`] que decodifica um arquivo de imagem.
    pub fn replace_texture_bytes(
        &mut self,
        id: TextureId,
        bytes: &[u8],
        options: TextureOptions,
    ) -> Result<(), RenderError> {
        let img = image::load_from_memory(bytes)?.into_rgba8();
        self.replace_texture(
            id,
            UVec2::new(img.width(), img.height()),
            img.as_raw(),
            options,
        );
        Ok(())
    }

    /// Cria uma textura a partir de um arquivo de imagem em memória (PNG).
    pub fn load_texture_bytes(
        &mut self,
        bytes: &[u8],
        options: TextureOptions,
    ) -> Result<TextureId, RenderError> {
        let img = image::load_from_memory(bytes)?.into_rgba8();
        let size = UVec2::new(img.width(), img.height());
        Ok(self.create_texture(size, img.as_raw(), options))
    }

    /// Carrega uma textura de um arquivo de imagem (PNG).
    pub fn load_texture(
        &mut self,
        path: impl AsRef<Path>,
        options: TextureOptions,
    ) -> Result<TextureId, RenderError> {
        let bytes = read_file(path.as_ref())?;
        self.load_texture_bytes(&bytes, options)
    }

    /// Tamanho da textura em pixels.
    pub fn texture_size(&self, id: TextureId) -> UVec2 {
        self.textures[id.0 as usize].size
    }

    /// Divide a textura em uma grade de quadros de `frame_size` pixels.
    pub fn sprite_sheet(&self, texture: TextureId, frame_size: UVec2) -> SpriteSheet {
        SpriteSheet::new(texture, self.texture_size(texture), frame_size)
    }

    // ------------------------------------------------------------------
    // Fontes e texto
    // ------------------------------------------------------------------

    /// Fonte usada quando [`Text::font`] é `None` (Poppins Regular).
    pub fn default_font(&self) -> FontId {
        self.text.default_font()
    }

    /// A fonte de pixel art embutida (Press Start 2P): nítida, em múltiplos
    /// inteiros do tamanho dela (8 px).
    pub fn pixel_font(&self) -> FontId {
        self.text.pixel_font()
    }

    /// Carrega uma fonte de pixel art: `em` é o tamanho (pixels por em) em que
    /// cada pixel do desenho dela vira um pixel da tela.
    pub fn load_pixel_font_bytes(&mut self, bytes: &[u8], em: f32) -> Result<FontId, RenderError> {
        self.text.load_pixel_font(bytes, em)
    }

    /// As linhas em que o texto é desenhado (as do `\n` e, com
    /// [`Text::width`], as quebradas para caber).
    pub fn text_lines(&self, text: &Text<'_>) -> Vec<String> {
        let k = self.text_scale();
        let mut scaled = *text;
        scaled.size = text.size * k;
        scaled.width = text.width.map(|w| w * k);
        self.text.lines(&scaled)
    }

    /// Carrega uma fonte TrueType/OpenType a partir dos bytes do arquivo.
    pub fn load_font_bytes(&mut self, bytes: &[u8]) -> Result<FontId, RenderError> {
        self.text.load_font(bytes)
    }

    /// Carrega uma fonte TrueType/OpenType (.ttf, .otf).
    pub fn load_font(&mut self, path: impl AsRef<Path>) -> Result<FontId, RenderError> {
        let bytes = read_file(path.as_ref())?;
        self.load_font_bytes(&bytes)
    }

    /// Tamanho da caixa que o texto ocupa (largura, altura), sem desenhá-lo.
    pub fn measure_text(&self, text: &Text<'_>) -> Vec2 {
        let k = self.text_scale();
        if k == 1.0 {
            return self.text.cached_layout(text).size;
        }
        let mut scaled = *text;
        scaled.size = text.size * k;
        scaled.width = text.width.map(|w| w * k);
        self.text.cached_layout(&scaled).size / k
    }

    /// Desenha o jogo num tamanho fixo ajustado ao alvo (veja
    /// [`crate::Presentation`]), ou no alvo inteiro com `None`. Vale para os
    /// próximos [`Renderer::render`]; o texto é rasterizado no tamanho da
    /// tela de verdade, para ficar nítido.
    pub fn set_presentation(&mut self, presentation: Option<crate::Presentation>) {
        self.presentation = presentation;
    }

    pub fn presentation(&self) -> Option<crate::Presentation> {
        self.presentation
    }

    /// Pixels de verdade por unidade do jogo, para rasterizar o texto.
    fn text_scale(&self) -> f32 {
        match &self.presentation {
            Some(p) => {
                let s = p.scale();
                let k = s.x.min(s.y);
                if (k - 1.0).abs() < 1e-3 {
                    1.0
                } else {
                    k.clamp(0.25, 16.0)
                }
            }
            None => 1.0,
        }
    }

    // ------------------------------------------------------------------
    // Desenho
    // ------------------------------------------------------------------

    /// Enfileira um sprite no espaço do mundo.
    pub fn draw(&mut self, sprite: &Sprite) {
        self.push_sprite(Space::World, sprite);
    }

    /// Retângulo de cor sólida centrado em `center`, no espaço do mundo.
    pub fn draw_rect(&mut self, center: Vec2, size: Vec2, color: Color) {
        let sprite = Sprite::new(self.white).at(center).size(size).color(color);
        self.push_sprite(Space::World, &sprite);
    }

    /// Círculo de cor sólida, no espaço do mundo.
    pub fn draw_circle(&mut self, center: Vec2, radius: f32, color: Color) {
        let sprite = Sprite::new(self.circle)
            .at(center)
            .size(Vec2::splat(radius * 2.0))
            .color(color);
        self.push_sprite(Space::World, &sprite);
    }

    /// O recorte do desenho no mundo: o que vier depois só aparece dentro
    /// dele (`None`: sem recorte). Cada mudança custa uma chamada de desenho a
    /// mais, no máximo.
    pub fn set_clip(&mut self, clip: Option<ClipRect>) {
        self.world.clip = clip;
    }

    /// O recorte do desenho no mundo agora.
    pub fn clip(&self) -> Option<ClipRect> {
        self.world.clip
    }

    /// Linha de `a` até `b`, com `width` de espessura, no espaço do mundo.
    pub fn draw_line(&mut self, a: Vec2, b: Vec2, width: f32, color: Color) {
        self.push_polyline(Space::World, &[a, b], width, false, color);
    }

    /// Linha passando pelos pontos (`closed`: volta ao primeiro), no espaço do
    /// mundo. As juntas ficam em ponta, ou cortadas nos ângulos fechados.
    pub fn draw_polyline(&mut self, points: &[Vec2], width: f32, closed: bool, color: Color) {
        self.push_polyline(Space::World, points, width, closed, color);
    }

    /// Polígono preenchido (também côncavo), no espaço do mundo.
    pub fn draw_polygon(&mut self, points: &[Vec2], color: Color) {
        self.push_polygon(Space::World, points, color);
    }

    /// Contorno de um retângulo centrado em `center`, por dentro dele, no
    /// espaço do mundo.
    pub fn draw_rect_outline(&mut self, center: Vec2, size: Vec2, width: f32, color: Color) {
        self.push_rect_outline(Space::World, center - size.abs() / 2.0, size, width, color);
    }

    fn push_rect_outline(&mut self, space: Space, min: Vec2, size: Vec2, width: f32, color: Color) {
        let size = size.abs();
        if width <= 0.0 || size.min_element() <= 0.0 {
            return;
        }
        // Mais grosso que o retângulo: fica cheio.
        let pts = if width * 2.0 >= size.min_element() {
            [
                min,
                Vec2::new(min.x + size.x, min.y),
                min + size,
                Vec2::new(min.x, min.y + size.y),
            ]
        } else {
            let h = Vec2::splat(width / 2.0);
            let (a, b) = (min + h, min + size - h);
            return self.push_polyline(
                space,
                &[a, Vec2::new(b.x, a.y), b, Vec2::new(a.x, b.y)],
                width,
                true,
                color,
            );
        };
        self.push_polygon(space, &pts, color);
    }

    /// Contorno de um círculo, por dentro dele, no espaço do mundo.
    pub fn draw_circle_outline(&mut self, center: Vec2, radius: f32, width: f32, color: Color) {
        self.push_circle_outline(Space::World, center, radius, width, color);
    }

    fn push_polyline(
        &mut self,
        space: Space,
        points: &[Vec2],
        width: f32,
        closed: bool,
        color: Color,
    ) {
        let mut tris = std::mem::take(&mut self.tri_buf);
        tris.clear();
        crate::shapes2d::polyline(points, width, closed, &mut tris);
        self.push_tris(space, &tris, color);
        self.tri_buf = tris;
    }

    fn push_polygon(&mut self, space: Space, points: &[Vec2], color: Color) {
        let mut tris = std::mem::take(&mut self.tri_buf);
        tris.clear();
        crate::shapes2d::polygon(points, &mut tris);
        self.push_tris(space, &tris, color);
        self.tri_buf = tris;
    }

    fn push_circle_outline(
        &mut self,
        space: Space,
        center: Vec2,
        radius: f32,
        width: f32,
        color: Color,
    ) {
        // A linha fica por dentro do círculo, como a borda de um painel.
        let r = (radius.abs() - width / 2.0).max(width / 4.0);
        let pts = crate::shapes2d::circle_points(center, r, self.text_scale());
        self.push_polyline(space, &pts, width.min(radius.abs() * 2.0), true, color);
    }

    /// Os triângulos de uma forma, na mesma fila das imagens (com a textura
    /// branca: linhas e retângulos seguidos ficam numa chamada só).
    fn push_tris(&mut self, space: Space, tris: &[crate::shapes2d::Tri], color: Color) {
        let color = self.gpu_color(color).to_array();
        let white = self.white;
        let queue = self.queue_mut(space);
        for t in tris {
            let [p0, p1, p2] = t.points;
            queue.push(
                white,
                Blend::Alpha,
                SpriteInstance {
                    position: p0.to_array(),
                    size: p1.to_array(),
                    origin: p2.to_array(),
                    rotation: 0.0,
                    region: [0.0, 0.0, 1.0, 1.0],
                    color,
                    repeat: [-1.0, f32::from(t.edges)],
                    offset: [0.0, 0.0],
                    flags: 0.0,
                },
            );
        }
    }

    /// Painel (retângulo arredondado com borda, ou imagem em 9 fatias) no
    /// espaço do mundo, com centro em `panel.center`.
    pub fn draw_panel(&mut self, panel: &crate::Panel) {
        self.push_panel(Space::World, panel, true);
    }

    /// Os sprites de um painel, num vetor reaproveitado de quadro em quadro.
    fn push_panel(&mut self, space: Space, panel: &crate::Panel, y_up: bool) {
        let rounded = match panel.image {
            None => self.rounded_textures(panel.radius.max(0.0), panel.border.max(0.0)),
            Some(_) => (self.white, self.white, 1),
        };
        let mut sprites = std::mem::take(&mut self.panel_buf);
        sprites.clear();
        let textures = &self.textures;
        crate::panel::panel_sprites(
            panel,
            y_up,
            &mut |_, _| rounded,
            &|id| {
                textures
                    .get(id.0 as usize)
                    .map(|t| t.size)
                    .unwrap_or(UVec2::ONE)
            },
            &mut sprites,
        );
        for sprite in &sprites {
            self.push_sprite(space, sprite);
        }
        self.panel_buf = sprites;
    }

    fn rounded_textures(&mut self, radius: f32, border: f32) -> (TextureId, TextureId, u32) {
        let key = ((radius * 2.0).round() as u32, (border * 2.0).round() as u32);
        if let Some(t) = self.rounded.get(&key) {
            return *t;
        }
        let (r, b) = (key.0 as f32 / 2.0, key.1 as f32 / 2.0);
        let (size, fill, ring) = crate::panel::rounded_pixels(r, b);
        let opts = TextureOptions::default();
        let t = (
            self.create_texture(size, &fill, opts),
            self.create_texture(size, &ring, opts),
            crate::panel::round_margin(r, b),
        );
        self.rounded.insert(key, t);
        t
    }

    /// Texto no espaço do mundo (acompanha a câmera e o zoom).
    pub fn draw_text(&mut self, text: &Text<'_>) {
        self.push_text(Space::World, text);
    }

    /// Desenho no espaço da tela (HUD), por cima do mundo.
    ///
    /// ```no_run
    /// # fn exemplo(renderer: &mut nn_render::Renderer) {
    /// use nn_render::Text;
    /// use glam::Vec2;
    /// renderer.screen().draw_text(&Text::new("Vidas: 3").at(Vec2::new(16.0, 16.0)));
    /// # }
    /// ```
    pub fn screen(&mut self) -> ScreenPainter<'_> {
        ScreenPainter { renderer: self }
    }

    fn push_sprite(&mut self, space: Space, sprite: &Sprite) {
        let texture = self
            .textures
            .get(sprite.texture.0 as usize)
            .expect("TextureId de outro renderer ou inválido");
        let size = sprite.size.unwrap_or_else(|| {
            texture.size.as_vec2() * Vec2::new(sprite.region.w, sprite.region.h)
        });
        let instance = SpriteInstance {
            position: sprite.position.to_array(),
            size: size.to_array(),
            origin: sprite.origin.to_array(),
            rotation: sprite.rotation,
            region: sprite.region.to_array(),
            color: self.gpu_color(sprite.color).to_array(),
            repeat: sprite.repeat.max(Vec2::splat(0.001)).to_array(),
            offset: sprite.offset.to_array(),
            flags: if texture.target.is_some() { 1.0 } else { 0.0 },
        };
        self.queue_mut(space)
            .push_with(sprite.texture, sprite.blend, sprite.effect, instance);
    }

    fn push_text(&mut self, space: Space, text: &Text<'_>) {
        // Com tamanho fixo do jogo, as letras são rasterizadas no tamanho em
        // que aparecem na tela (`k` pixels por unidade) e encolhidas de volta.
        let k = self.text_scale();
        let mut scaled = *text;
        scaled.size = text.size * k;
        scaled.width = text.width.map(|w| w * k);
        let layout = self.text.cached_layout(&scaled);
        let color = self.gpu_color(text.color).to_array();
        // Canto superior esquerdo da caixa, relativo a `position`, com Y para baixo.
        let box_offset = -text.anchor_fraction() * layout.size;

        for placed in &layout.glyphs {
            let Some(glyph) = self.text.glyph(
                placed.key,
                &self.device,
                &self.queue,
                &self.texture_layout,
                &mut self.textures,
            ) else {
                continue;
            };
            let local = box_offset + placed.top_left;
            let (position, origin) = match space {
                // Pixels inteiros (da tela de verdade) deixam as letras nítidas.
                Space::Screen => ((text.position * k + local).round() / k, Vec2::ZERO),
                Space::World => (
                    (text.position * k + Vec2::new(local.x, -local.y)).round() / k,
                    Vec2::new(0.0, 1.0),
                ),
            };
            let instance = SpriteInstance {
                position: position.to_array(),
                size: (placed.size / k).to_array(),
                origin: origin.to_array(),
                rotation: 0.0,
                region: glyph.region.to_array(),
                color,
                repeat: [1.0, 1.0],
                offset: [0.0, 0.0],
                flags: 0.0,
            };
            self.queue_mut(space)
                .push(glyph.texture, Blend::Alpha, instance);
        }
    }

    fn queue_mut(&mut self, space: Space) -> &mut DrawQueue {
        match space {
            Space::World => &mut self.world,
            Space::Screen => &mut self.screen,
        }
    }

    fn gpu_color(&self, color: Color) -> Color {
        if !self.format.is_srgb() {
            return color;
        }
        // A mesma cor seguida (o branco das imagens, a cor de um texto) não
        // refaz a conta (três potências).
        let bits = color_bits(color);
        let (last, linear) = self.last_color.get();
        if last == bits {
            return linear;
        }
        let linear = color.to_linear();
        self.last_color.set((bits, linear));
        linear
    }

    /// Desenha tudo o que foi enfileirado em `target` e esvazia as filas.
    ///
    /// `viewport` é o tamanho de `target` em pixels. Se `clear` for `Some`,
    /// o alvo é limpo com essa cor antes; caso contrário o conteúdo é mantido.
    /// Com [`Renderer::set_presentation`], o jogo ocupa só a área dele (o
    /// resto fica preto) e a câmera e a tela usam o tamanho do jogo.
    pub fn render(
        &mut self,
        target: &wgpu::TextureView,
        viewport: UVec2,
        camera: &Camera2D,
        clear: Option<Color>,
    ) -> RenderStats {
        let skipped = std::mem::take(&mut self.screen_effect_skipped);
        if let Some(effect) = self
            .screen_effect
            .filter(|e| !skipped && self.effects.get(*e).is_some())
        {
            return self.render_with_screen_effect(target, viewport, camera, clear, effect);
        }
        self.render_inner(target, viewport, camera, clear, Pass::Main)
    }

    /// O quadro num canvas do tamanho em que ele aparece na janela, e o
    /// canvas na tela com o efeito.
    fn render_with_screen_effect(
        &mut self,
        target: &wgpu::TextureView,
        viewport: UVec2,
        camera: &Camera2D,
        clear: Option<Color>,
        effect: EffectId,
    ) -> RenderStats {
        let present = self.presentation;
        let (size, inner) = match present {
            Some(p) => {
                let size = p.size.round().max(Vec2::ONE);
                (
                    size.as_uvec2(),
                    Some(crate::Presentation {
                        logical: p.logical,
                        origin: Vec2::ZERO,
                        size,
                    }),
                )
            }
            None => (viewport.max(UVec2::ONE), None),
        };
        let canvas = match self.screen_canvas {
            Some((id, s)) if s == size => id,
            other => {
                if let Some((old, _)) = other {
                    self.free_texture(old);
                }
                let id = self.create_canvas(size, TextureOptions::PIXEL_ART);
                self.screen_canvas = Some((id, size));
                id
            }
        };
        let Some(view) = self.textures[canvas.0 as usize].target.clone() else {
            return self.render_inner(target, viewport, camera, clear, Pass::Main);
        };
        self.presentation = inner;
        self.render_inner(&view, size, camera, clear, Pass::Offscreen);
        self.presentation = present;
        let logical = present.map_or(viewport, |p| p.logical).as_vec2();
        self.screen().draw(
            &Sprite::new(canvas)
                .at(Vec2::ZERO)
                .origin(Vec2::ZERO)
                .size(logical)
                .effect(effect),
        );
        self.render_inner(target, viewport, &Camera2D::default(), clear, Pass::Main)
    }

    /// Desenha o que foi pedido em `target`. Num canvas, sem o 3D; fora do
    /// passo principal, sem fechar o quadro (o cache das letras segue até o
    /// fim dele).
    fn render_inner(
        &mut self,
        target: &wgpu::TextureView,
        viewport: UVec2,
        camera: &Camera2D,
        clear: Option<Color>,
        pass_kind: Pass,
    ) -> RenderStats {
        self.effects.upload(&self.queue);
        let present = self.presentation;
        let logical = present.map(|p| p.logical).unwrap_or(viewport);
        let world = ViewUniform {
            view_proj: camera.view_projection(logical.as_vec2()).to_cols_array(),
            y_down: 0.0,
            _padding: [0.0; 3],
        };
        let size = logical.max(UVec2::ONE).as_vec2();
        let screen = ViewUniform {
            view_proj: screen_projection(size).to_cols_array(),
            y_down: 1.0,
            _padding: [0.0; 3],
        };
        self.queue
            .write_buffer(&self.world_view.buffer, 0, bytemuck::bytes_of(&world));
        self.queue
            .write_buffer(&self.screen_view.buffer, 0, bytemuck::bytes_of(&screen));

        let world_count = self.world.instances.len();
        let total = world_count + self.screen.instances.len();
        // Com tamanho fixo, o fundo do jogo é um retângulo (a janela toda é
        // limpa de preto, para as faixas).
        let background = match (present, clear) {
            (Some(_), Some(c)) => Some(SpriteInstance {
                position: [0.0, 0.0],
                size: size.to_array(),
                origin: [0.0, 0.0],
                rotation: 0.0,
                region: crate::Rect::FULL.to_array(),
                color: self.gpu_color(c).to_array(),
                repeat: [1.0, 1.0],
                offset: [0.0, 0.0],
                flags: 0.0,
            }),
            _ => None,
        };
        let total = total + usize::from(background.is_some());
        if total > self.instance_capacity {
            self.instance_capacity = total.next_power_of_two();
            self.instance_buffer =
                Self::create_instance_buffer(&self.device, self.instance_capacity);
        }
        if world_count > 0 {
            self.queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&self.world.instances),
            );
        }
        if !self.screen.instances.is_empty() {
            self.queue.write_buffer(
                &self.instance_buffer,
                (world_count * size_of::<SpriteInstance>()) as wgpu::BufferAddress,
                bytemuck::cast_slice(&self.screen.instances),
            );
        }
        let background_index = (world_count + self.screen.instances.len()) as u32;
        if let Some(bg) = &background {
            self.queue.write_buffer(
                &self.instance_buffer,
                (background_index as usize * size_of::<SpriteInstance>()) as wgpu::BufferAddress,
                bytemuck::bytes_of(bg),
            );
        }

        let load = match clear {
            Some(_) if present.is_some() => wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            Some(c) => {
                let c = self.gpu_color(c);
                wgpu::LoadOp::Clear(wgpu::Color {
                    r: f64::from(c.r),
                    g: f64::from(c.g),
                    b: f64::from(c.b),
                    a: f64::from(c.a),
                })
            }
            None => wgpu::LoadOp::Load,
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("nn sprite frame"),
            });
        // Com 3D, o quadro vira três passos: limpar (e o fundo do jogo), as
        // malhas com profundidade e por fim o 2D por cima.
        let has_3d = pass_kind != Pass::Canvas && self.d3.has_draws();
        let mut load = load;
        if has_3d {
            let area = match present {
                Some(p) => (p.origin.x, p.origin.y, p.size.x, p.size.y),
                None => (0.0, 0.0, viewport.x as f32, viewport.y as f32),
            };
            if background.is_some() || matches!(load, wgpu::LoadOp::Clear(_)) {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("nn clear pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                if let (Some(p), Some(_)) = (present, &background)
                    && let Some((x, y, w, h)) = scissor(p, viewport)
                {
                    pass.set_viewport(p.origin.x, p.origin.y, p.size.x, p.size.y, 0.0, 1.0);
                    pass.set_scissor_rect(x, y, w, h);
                    pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
                    pass.set_pipeline(&self.pipeline);
                    pass.set_bind_group(0, &self.screen_view.bind_group, &[]);
                    let white = &self.textures[self.white.0 as usize];
                    pass.set_bind_group(1, &white.bind_group, &[]);
                    pass.draw(0..6, background_index..background_index + 1);
                }
            }
            let aspect = logical.x.max(1) as f32 / logical.y.max(1) as f32;
            self.d3.render(
                &self.device,
                &self.queue,
                &mut encoder,
                target,
                viewport,
                area,
                aspect,
                self.format.is_srgb(),
                &self.textures,
            );
            load = wgpu::LoadOp::Load;
        }
        let background = if has_3d { None } else { background };
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("nn sprite pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
            let mut current_pipeline: Option<(Blend, Option<usize>)> = None;
            if let Some(p) = present {
                pass.set_viewport(p.origin.x, p.origin.y, p.size.x, p.size.y, 0.0, 1.0);
                let Some((x, y, w, h)) = scissor(p, viewport) else {
                    drop(pass);
                    self.queue.submit([encoder.finish()]);
                    self.world.clear();
                    self.screen.clear();
                    self.text.end_frame();
                    return self.last_stats;
                };
                pass.set_scissor_rect(x, y, w, h);
                if background.is_some() {
                    pass.set_pipeline(&self.pipeline);
                    current_pipeline = Some((Blend::Alpha, None));
                    pass.set_bind_group(0, &self.screen_view.bind_group, &[]);
                    let white = &self.textures[self.white.0 as usize];
                    pass.set_bind_group(1, &white.bind_group, &[]);
                    pass.draw(0..6, background_index..background_index + 1);
                }
            }

            // O recorte de fora (a área do jogo, com tamanho fixo) e onde o
            // jogo fica no alvo, para levar os recortes do desenho a pixels.
            let base_scissor = match present {
                Some(p) => scissor(p, viewport),
                None => Some((0, 0, viewport.x.max(1), viewport.y.max(1))),
            };
            let (vp_origin, vp_size) = match present {
                Some(p) => (p.origin, p.size),
                None => (Vec2::ZERO, viewport.as_vec2()),
            };
            let world_matrix = Mat4::from_cols_array(&world.view_proj);
            let screen_matrix = Mat4::from_cols_array(&screen.view_proj);
            // O recorte que está valendo no passo de desenho.
            let mut current = base_scissor;
            let passes = [
                (&self.world_view, &self.world, 0u32, world_matrix),
                (
                    &self.screen_view,
                    &self.screen,
                    world_count as u32,
                    screen_matrix,
                ),
            ];
            for (view, queue, base, matrix) in passes {
                if queue.batches.is_empty() {
                    continue;
                }
                pass.set_bind_group(0, &view.bind_group, &[]);
                for batch in &queue.batches {
                    let rect = match batch.clip {
                        None => base_scissor,
                        Some(c) => clip_scissor(c, matrix, vp_origin, vp_size, base_scissor),
                    };
                    // Recortado até sumir: não desenha.
                    let Some((x, y, w, h)) = rect else {
                        continue;
                    };
                    if rect != current {
                        pass.set_scissor_rect(x, y, w, h);
                        current = rect;
                    }
                    // Com efeito, o pipeline dele (e os parâmetros no grupo 2).
                    let effect = batch.effect.and_then(|e| self.effects.get(e));
                    let key = (batch.blend, effect.map(|i| i as *const _ as usize));
                    if current_pipeline != Some(key) {
                        let canvas = pass_kind != Pass::Main;
                        let pipeline = match (effect, batch.blend, canvas) {
                            (Some(i), Blend::Alpha, false) => &self.effects.shaders[i.shader].alpha,
                            (Some(i), Blend::Add, false) => &self.effects.shaders[i.shader].add,
                            (Some(i), Blend::Alpha, true) => {
                                &self.effects.shaders[i.shader].canvas_alpha
                            }
                            (Some(i), Blend::Add, true) => {
                                &self.effects.shaders[i.shader].canvas_add
                            }
                            (None, Blend::Alpha, false) => &self.pipeline,
                            (None, Blend::Add, false) => &self.additive_pipeline,
                            (None, Blend::Alpha, true) => &self.canvas_pipeline,
                            (None, Blend::Add, true) => &self.canvas_additive_pipeline,
                        };
                        pass.set_pipeline(pipeline);
                        if let Some(i) = effect {
                            pass.set_bind_group(2, &i.bind_group, &[]);
                        }
                        current_pipeline = Some(key);
                    }
                    let texture = &self.textures[batch.texture.0 as usize];
                    pass.set_bind_group(1, &texture.bind_group, &[]);
                    pass.draw(0..6, base + batch.start..base + batch.end);
                }
            }
        }
        self.queue.submit([encoder.finish()]);

        let stats = RenderStats {
            sprites: total as u32,
            draw_calls: (self.world.batches.len() + self.screen.batches.len()) as u32,
        };
        self.world.clear();
        self.screen.clear();
        if pass_kind != Pass::Main {
            // Somado ao quadro, que mostra o total.
            self.canvas_stats.sprites += stats.sprites;
            self.canvas_stats.draw_calls += stats.draw_calls;
            return stats;
        }
        self.last_stats = RenderStats {
            sprites: stats.sprites + self.canvas_stats.sprites,
            draw_calls: stats.draw_calls + self.canvas_stats.draw_calls,
        };
        self.canvas_stats = RenderStats::default();
        self.text.end_frame();
        self.last_stats
    }

    // ------------------------------------------------------------------
    // Canvas: desenhar numa imagem
    // ------------------------------------------------------------------

    /// Uma imagem em que se desenha ([`Renderer::begin_canvas`]) e que se
    /// usa como qualquer outra (num sprite, num painel...). Começa
    /// transparente.
    pub fn create_canvas(&mut self, size: UVec2, options: TextureOptions) -> TextureId {
        let size = size.max(UVec2::ONE);
        let texture = GpuTexture::render_target(
            &self.device,
            &self.texture_layout,
            size,
            self.format,
            options.filter,
        );
        if let Some(i) = self.freed.pop_first() {
            self.textures[i as usize] = texture;
            return TextureId(i);
        }
        self.textures.push(texture);
        TextureId(self.textures.len() as u32 - 1)
    }

    /// É um canvas (de [`Renderer::create_canvas`])?
    pub fn is_canvas(&self, id: TextureId) -> bool {
        self.textures
            .get(id.0 as usize)
            .is_some_and(|t| t.target.is_some())
    }

    /// Começa a desenhar no canvas: o que for pedido até
    /// [`Renderer::end_canvas`] vai para ele (o mundo e a tela, em pixels do
    /// canvas), não para o quadro. Pode ficar um dentro do outro. Devolve
    /// `false` (e nada muda) se `id` não for um canvas.
    pub fn begin_canvas(&mut self, id: TextureId) -> bool {
        if !self.is_canvas(id) {
            return false;
        }
        let frame = CanvasFrame {
            canvas: id,
            world: std::mem::take(&mut self.world),
            screen: std::mem::take(&mut self.screen),
            presentation: self.presentation.take(),
        };
        self.canvas_stack.push(frame);
        true
    }

    /// Desenha no canvas o que foi pedido desde [`Renderer::begin_canvas`],
    /// com a câmera dada (o mundo) e, com `clear`, limpando antes (`None`
    /// mantém o que já estava: rastros). Volta a desenhar onde estava.
    pub fn end_canvas(&mut self, camera: &Camera2D, clear: Option<Color>) {
        let Some(frame) = self.canvas_stack.pop() else {
            return;
        };
        let id = frame.canvas;
        let texture = &self.textures[id.0 as usize];
        let (view, size) = (texture.target.clone(), texture.size);
        // Um canvas não pode se desenhar nele mesmo: essas partes ficam de
        // fora.
        for q in [&mut self.world, &mut self.screen] {
            q.batches.retain(|b| b.texture != id);
        }
        if let Some(view) = view {
            self.render_inner(&view, size, camera, clear, Pass::Canvas);
        }
        self.world = frame.world;
        self.screen = frame.screen;
        self.presentation = frame.presentation;
    }

    // ------------------------------------------------------------------
    // Efeitos (shaders)
    // ------------------------------------------------------------------

    /// Compila um efeito em WGSL (veja [`crate::effects`]). O erro diz a
    /// linha e o que houve, em português; nada muda se der erro.
    pub fn compile_effect(&mut self, source: &str) -> Result<EffectShader, EffectError> {
        self.effects.compile(
            &self.device,
            source,
            self.format,
            &self.view_layout,
            &self.texture_layout,
            SpriteInstance::layout(),
            [
                wgpu::BlendState::ALPHA_BLENDING,
                ADDITIVE_BLEND,
                CANVAS_BLEND,
                CANVAS_ADDITIVE_BLEND,
            ],
        )
    }

    /// Um efeito pronto ("cinza", "pixelar", "crt", "onda", "contorno",
    /// "brilho"), compilado na primeira vez.
    pub fn builtin_effect(&mut self, name: &str) -> Option<EffectShader> {
        if let Some(s) = self.effects.builtin(name) {
            return Some(s);
        }
        let source = crate::effects::builtin_source(name)?;
        let shader = self
            .compile_effect(source)
            .unwrap_or_else(|e| panic!("o efeito pronto {name} não compila: {e}"));
        self.effects.remember_builtin(name, shader);
        Some(shader)
    }

    /// Um efeito para usar (num sprite, num canvas, na tela), com os valores
    /// padrão dos parâmetros.
    pub fn create_effect(&mut self, shader: EffectShader) -> EffectId {
        self.effects.create(&self.device, shader)
    }

    /// Os parâmetros do efeito, com os valores padrão.
    pub fn effect_params(&self, effect: EffectId) -> &[(String, f32)] {
        self.effects.params(effect)
    }

    /// Muda um parâmetro. Devolve `false` se o efeito não tem esse parâmetro.
    pub fn set_effect_param(&mut self, effect: EffectId, name: &str, value: f32) -> bool {
        self.effects.set(effect, name, value)
    }

    /// O valor de um parâmetro.
    pub fn effect_param(&self, effect: EffectId, name: &str) -> Option<f32> {
        self.effects.value(effect, name)
    }

    /// Muda uma cor: os parâmetros `nome_r`, `nome_g`, `nome_b` (e `nome_a`,
    /// se houver), em cor linear, como o shader trabalha. Devolve `false`
    /// (sem mudar nada) se faltar algum.
    pub fn set_effect_color(&mut self, effect: EffectId, name: &str, color: Color) -> bool {
        let has = |s: &Self, k: &str| s.effects.value(effect, &format!("{name}_{k}")).is_some();
        if !(has(self, "r") && has(self, "g") && has(self, "b")) {
            return false;
        }
        let c = self.gpu_color(color);
        for (suffix, v) in [("r", c.r), ("g", c.g), ("b", c.b), ("a", c.a)] {
            self.effects.set(effect, &format!("{name}_{suffix}"), v);
        }
        true
    }

    /// A cor `nome` do efeito (de `nome_r`, `nome_g`, `nome_b` e `nome_a`),
    /// de volta em sRGB.
    pub fn effect_color(&self, effect: EffectId, name: &str) -> Option<Color> {
        let v = |k: &str| self.effects.value(effect, &format!("{name}_{k}"));
        let (r, g, b) = (v("r")?, v("g")?, v("b")?);
        let a = v("a").unwrap_or(1.0);
        let to_srgb = |x: f32| {
            let x = x.clamp(0.0, 1.0);
            if !self.format.is_srgb() {
                x
            } else if x <= 0.003_130_8 {
                x * 12.92
            } else {
                1.055 * x.powf(1.0 / 2.4) - 0.055
            }
        };
        Some(Color::rgba(to_srgb(r), to_srgb(g), to_srgb(b), a))
    }

    /// As cores do efeito (os nomes com `_r`, `_g` e `_b`).
    pub fn effect_colors(&self, effect: EffectId) -> Vec<String> {
        let params = self.effects.params(effect);
        params
            .iter()
            .filter_map(|(n, _)| n.strip_suffix("_r"))
            .filter(|base| {
                ["g", "b"]
                    .iter()
                    .all(|k| params.iter().any(|(n, _)| *n == format!("{base}_{k}")))
            })
            .map(str::to_string)
            .collect()
    }

    /// Troca o shader de um efeito (ao recarregar o arquivo), mantendo os
    /// valores dos parâmetros de mesmo nome.
    pub fn replace_effect_shader(&mut self, effect: EffectId, shader: EffectShader) {
        self.effects.replace_shader(effect, shader);
    }

    /// Libera o efeito (o id passa a desenhar sem efeito).
    pub fn free_effect(&mut self, effect: EffectId) {
        if self.screen_effect == Some(effect) {
            self.screen_effect = None;
        }
        self.effects.free(effect);
    }

    /// O tempo dos efeitos (`tempo()` no shader), em segundos.
    pub fn set_time(&mut self, seconds: f32) {
        self.effects.time = seconds;
    }

    /// Um efeito na tela inteira (o quadro é desenhado num canvas e mostrado
    /// com o efeito), ou `None`.
    pub fn set_screen_effect(&mut self, effect: Option<EffectId>) {
        self.screen_effect = effect;
    }

    pub fn screen_effect(&self) -> Option<EffectId> {
        self.screen_effect
    }

    /// Mostra este quadro sem o efeito da tela (a tela de erro precisa ser
    /// lida, com qualquer efeito).
    pub fn skip_screen_effect_this_frame(&mut self) {
        self.screen_effect_skipped = true;
    }

    /// Joga fora o que foi pedido neste quadro, sem desenhar nada (para
    /// rodar a lógica sem gastar a GPU, como nos testes sem janela).
    pub fn discard_frame(&mut self) {
        self.screen_effect_skipped = false;
        self.world.clear();
        self.screen.clear();
        self.canvas_stats = RenderStats::default();
        self.text.end_frame();
        self.d3.discard();
    }

    // ------------------------------------------------------------------
    // 3D
    // ------------------------------------------------------------------

    /// Envia uma malha para a GPU. Guarde o [`MeshId`](crate::d3::MeshId) e
    /// desenhe com [`Renderer::draw_mesh`] quantas vezes quiser.
    pub fn create_mesh(&mut self, mesh: &crate::d3::Mesh) -> crate::d3::MeshId {
        let srgb = self.format.is_srgb();
        let vertices: Vec<crate::d3::Vertex3> = mesh
            .vertices
            .iter()
            .map(|v| {
                let mut v = *v;
                if srgb {
                    let [r, g, b, a] = v.color;
                    let c = Color::rgba(r, g, b, a).to_linear();
                    v.color = [c.r, c.g, c.b, c.a];
                }
                v
            })
            .collect();
        self.d3
            .create_mesh(&self.device, &self.queue, &vertices, &mesh.indices, None)
    }

    /// Troca o conteúdo de uma malha já criada (o mesmo id continua valendo).
    pub fn update_mesh(&mut self, id: crate::d3::MeshId, mesh: &crate::d3::Mesh) {
        let srgb = self.format.is_srgb();
        let vertices: Vec<crate::d3::Vertex3> = mesh
            .vertices
            .iter()
            .map(|v| {
                let mut v = *v;
                if srgb {
                    let [r, g, b, a] = v.color;
                    let c = Color::rgba(r, g, b, a).to_linear();
                    v.color = [c.r, c.g, c.b, c.a];
                }
                v
            })
            .collect();
        self.d3
            .replace_mesh(&self.device, &self.queue, id, &vertices, &mesh.indices);
    }

    /// Libera a memória de uma malha que ninguém desenha mais. O id passa a
    /// desenhar nada até ser reaproveitado por uma malha nova.
    pub fn free_mesh(&mut self, id: crate::d3::MeshId) {
        self.d3.free_mesh(&self.device, &self.queue, id);
    }

    /// Libera um modelo (as malhas e as texturas dele). Clones do modelo
    /// param de desenhar.
    pub fn free_model(&mut self, model: &crate::d3::Model) {
        for part in &model.parts {
            self.free_mesh(part.mesh);
            if let Some(t) = part.texture {
                self.free_texture(t);
            }
        }
    }

    /// Quantas malhas 3D estão na GPU.
    pub fn mesh_count(&self) -> usize {
        self.d3.mesh_count()
    }

    /// Como [`Renderer::create_mesh`], com os ossos de cada vértice (para
    /// desenhar com [`Renderer::draw_mesh_skinned`]).
    pub fn create_mesh_skinned(
        &mut self,
        mesh: &crate::d3::Mesh,
        skin: &[crate::d3::SkinVertex],
    ) -> crate::d3::MeshId {
        let srgb = self.format.is_srgb();
        let vertices: Vec<crate::d3::Vertex3> = mesh
            .vertices
            .iter()
            .map(|v| {
                let mut v = *v;
                if srgb {
                    let [r, g, b, a] = v.color;
                    let c = Color::rgba(r, g, b, a).to_linear();
                    v.color = [c.r, c.g, c.b, c.a];
                }
                v
            })
            .collect();
        let mut skin = skin.to_vec();
        skin.resize(vertices.len(), crate::d3::SkinVertex::rigid(0));
        self.d3.create_mesh(
            &self.device,
            &self.queue,
            &vertices,
            &mesh.indices,
            Some(&skin),
        )
    }

    /// Desenha uma malha com ossos, com as matrizes dos ossos em `joints`
    /// (veja [`Skeleton::joint_matrices`](crate::d3::Skeleton::joint_matrices)).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_mesh_skinned(
        &mut self,
        mesh: crate::d3::MeshId,
        model: Mat4,
        color: Color,
        texture: Option<TextureId>,
        cast_shadow: bool,
        joints: &[Mat4],
    ) {
        self.draw_mesh_skinned_glow(
            mesh,
            model,
            color,
            texture,
            cast_shadow,
            joints,
            crate::d3::Glow::NONE,
        );
    }

    /// Como [`Renderer::draw_mesh_skinned`], com brilho próprio.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_mesh_skinned_glow(
        &mut self,
        mesh: crate::d3::MeshId,
        model: Mat4,
        color: Color,
        texture: Option<TextureId>,
        cast_shadow: bool,
        joints: &[Mat4],
        glow: crate::d3::Glow,
    ) {
        self.draw_mesh_skinned_material(
            mesh,
            model,
            color,
            texture,
            cast_shadow,
            joints,
            crate::d3::Material3D::glow(glow),
        );
    }

    /// Como [`Renderer::draw_mesh_skinned`], com o material (brilho, água,
    /// balançar com o vento).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_mesh_skinned_material(
        &mut self,
        mesh: crate::d3::MeshId,
        model: Mat4,
        color: Color,
        texture: Option<TextureId>,
        cast_shadow: bool,
        joints: &[Mat4],
        material: crate::d3::Material3D,
    ) {
        let srgb = self.format.is_srgb();
        let texture = texture.unwrap_or(self.white);
        self.d3.draw_skinned(
            mesh,
            model,
            color,
            texture,
            cast_shadow,
            srgb,
            joints,
            material,
        );
    }

    /// Desenha a malha no mundo 3D com a matriz `model` (posição, rotação e
    /// escala; veja [`Transform3`](crate::d3::Transform3)) e a cor `color`
    /// multiplicando a dos vértices. Projeta sombra.
    pub fn draw_mesh(&mut self, mesh: crate::d3::MeshId, model: Mat4, color: Color) {
        self.draw_mesh_with(mesh, model, color, None, true);
    }

    /// Como [`Renderer::draw_mesh`], com uma textura (pelas coordenadas `uv`
    /// dos vértices).
    pub fn draw_mesh_textured(
        &mut self,
        mesh: crate::d3::MeshId,
        model: Mat4,
        color: Color,
        texture: TextureId,
    ) {
        self.draw_mesh_with(mesh, model, color, Some(texture), true);
    }

    /// Desenha a malha escolhendo a textura e se ela projeta sombra.
    pub fn draw_mesh_with(
        &mut self,
        mesh: crate::d3::MeshId,
        model: Mat4,
        color: Color,
        texture: Option<TextureId>,
        cast_shadow: bool,
    ) {
        self.draw_mesh_glow(
            mesh,
            model,
            color,
            texture,
            cast_shadow,
            crate::d3::Glow::NONE,
        );
    }

    /// Como [`Renderer::draw_mesh_with`], com brilho próprio: a malha
    /// aparece acesa mesmo no escuro (veja [`Glow`](crate::d3::Glow)).
    pub fn draw_mesh_glow(
        &mut self,
        mesh: crate::d3::MeshId,
        model: Mat4,
        color: Color,
        texture: Option<TextureId>,
        cast_shadow: bool,
        glow: crate::d3::Glow,
    ) {
        self.draw_mesh_material(
            mesh,
            model,
            color,
            texture,
            cast_shadow,
            crate::d3::Material3D::glow(glow),
        );
    }

    /// Como [`Renderer::draw_mesh_with`], com o material: brilho, água (ondas
    /// e o céu refletido) e balançar com o vento (veja
    /// [`Material3D`](crate::d3::Material3D)).
    pub fn draw_mesh_material(
        &mut self,
        mesh: crate::d3::MeshId,
        model: Mat4,
        color: Color,
        texture: Option<TextureId>,
        cast_shadow: bool,
        material: crate::d3::Material3D,
    ) {
        let srgb = self.format.is_srgb();
        let texture = texture.unwrap_or(self.white);
        self.d3
            .draw(mesh, model, color, texture, cast_shadow, srgb, material);
    }

    /// O halo do brilho (bloom) no mundo 3D: o que brilha ganha um halo
    /// suave em volta. Ligado por padrão.
    pub fn set_bloom3d(&mut self, bloom: crate::d3::Bloom) {
        self.d3.bloom = bloom;
    }

    pub fn bloom3d(&self) -> crate::d3::Bloom {
        self.d3.bloom
    }

    /// O vento do mundo 3D (vale até ser trocado).
    pub fn set_wind3d(&mut self, wind: crate::d3::Wind3D) {
        self.d3.wind = wind;
    }

    pub fn wind3d(&self) -> crate::d3::Wind3D {
        self.d3.wind
    }

    /// Os segundos que as ondas e o vento usam (`None`: o relógio desde que
    /// o renderer foi criado). Fixe para tirar imagens iguais.
    pub fn set_time3d(&mut self, seconds: Option<f32>) {
        self.d3.time = seconds;
    }

    /// Uma lâmpada ou um holofote no mundo 3D, só para este frame (chame a
    /// cada frame, como os desenhos). Veja [`Lamp3D`](crate::d3::Lamp3D).
    pub fn add_lamp3d(&mut self, lamp: crate::d3::Lamp3D) {
        self.d3.lamp(lamp);
    }

    /// Uma partícula 3D: um quadrado de lado `size` (metros) sempre virado
    /// para a câmera, girado `rotation` radianos. Aparece na frente e atrás
    /// dos objetos certos, com névoa, sem sombra. Veja
    /// [`ParticleEmitter3`](crate::d3::ParticleEmitter3).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_billboard(
        &mut self,
        texture: TextureId,
        center: Vec3,
        size: f32,
        rotation: f32,
        color: Color,
        region: crate::Rect,
        blend: crate::Blend,
    ) {
        let srgb = self.format.is_srgb();
        self.d3.billboard(
            texture,
            center,
            size,
            rotation,
            color,
            region,
            blend == crate::Blend::Add,
            srgb,
        );
    }

    /// Escreve um texto no mundo 3D: virado para a câmera (nomes, avisos) ou
    /// preso em dois eixos (placas). Usa o conteúdo, a cor, a fonte, a âncora
    /// e o alinhamento de `text`; o tamanho vem de `opts.size` (metros).
    /// Aparece atrás das paredes e com névoa, como as partículas.
    ///
    /// ```no_run
    /// # fn exemplo(r: &mut nn_render::Renderer) {
    /// use glam::Vec3;
    /// use nn_render::{Anchor, Text, d3::Text3D};
    /// let placa = Text::new("Saída").anchor(Anchor::Center);
    /// r.draw_text3d(&placa, &Text3D::at(Vec3::new(0.0, 2.0, 0.0), 0.5));
    /// # }
    /// ```
    pub fn draw_text3d(&mut self, text: &Text<'_>, opts: &crate::d3::Text3D) {
        // As letras são rasterizadas sempre neste tamanho (nítidas de perto,
        // e um tamanho só no atlas) e encolhidas para `size` metros.
        const PX: f32 = 64.0;
        let mut raster = *text;
        raster.size = PX;
        let layout = self.text.cached_layout(&raster);
        if opts.size <= 0.0 || (layout.glyphs.is_empty() && opts.background.is_none()) {
            return;
        }
        let k = opts.size / PX;
        let srgb = self.format.is_srgb();
        // Visto de trás, a placa espelha (continua dando para ler).
        let axes = opts.axes.map(|(r, u)| {
            let eye = self.d3.camera.position;
            if r.cross(u).dot(eye - opts.position) < 0.0 {
                (-r, u)
            } else {
                (r, u)
            }
        });
        let quad = |center_px: Vec2, size_px: Vec2| crate::d3::Quad3 {
            center: opts.position,
            size: size_px * k,
            offset: Vec2::new(center_px.x, -center_px.y) * k,
            axes,
        };
        // Canto de cima à esquerda da caixa, relativo à âncora (Y para baixo).
        let box_offset = -text.anchor_fraction() * layout.size;
        // Tudo do mesmo texto tem o mesmo centro: a ordem de desenho é a de
        // envio (fundo, contorno, letras).
        if let Some(bg) = opts.background {
            let pad = Vec2::new(PX * 0.3, PX * 0.15);
            let q = quad(box_offset + layout.size / 2.0, layout.size + pad * 2.0);
            self.d3
                .quad(self.white, q, 0.0, bg, crate::Rect::FULL, false, srgb);
        }
        let mut glyphs = Vec::with_capacity(layout.glyphs.len());
        for placed in &layout.glyphs {
            if let Some(g) = self.text.glyph(
                placed.key,
                &self.device,
                &self.queue,
                &self.texture_layout,
                &mut self.textures,
            ) {
                glyphs.push((
                    g,
                    box_offset + placed.top_left + placed.size / 2.0,
                    placed.size,
                ));
            }
        }
        if let Some(oc) = opts.outline {
            let r = PX * 0.06;
            for (dx, dy) in [
                (1.0, 0.0),
                (-1.0, 0.0),
                (0.0, 1.0),
                (0.0, -1.0),
                (0.7, 0.7),
                (-0.7, 0.7),
                (0.7, -0.7),
                (-0.7, -0.7),
            ] {
                for (g, c, size) in &glyphs {
                    let q = quad(*c + Vec2::new(dx, dy) * r, *size);
                    self.d3.quad(g.texture, q, 0.0, oc, g.region, false, srgb);
                }
            }
        }
        for (g, c, size) in &glyphs {
            let q = quad(*c, *size);
            self.d3
                .quad(g.texture, q, 0.0, text.color, g.region, false, srgb);
        }
    }

    /// Envia um modelo (lido de um `.glb`) para a GPU.
    pub fn create_model(&mut self, data: &crate::d3::ModelData) -> crate::d3::Model {
        let textures: Vec<TextureId> = data
            .images
            .iter()
            .map(|img| {
                let options = if img.nearest {
                    TextureOptions::PIXEL_ART
                } else {
                    TextureOptions::default()
                };
                self.create_texture(img.size, &img.rgba, options)
            })
            .collect();
        let skinned = data.skeleton.is_some();
        let parts = data
            .parts
            .iter()
            .map(|p| {
                let mesh = if skinned {
                    self.create_mesh_skinned(&p.mesh, &p.skin)
                } else {
                    self.create_mesh(&p.mesh)
                };
                crate::d3::GpuPart {
                    mesh,
                    texture: p.image.and_then(|i| textures.get(i).copied()),
                    emissive: p.emissive,
                    cutout: p.cutout,
                    blend: p.blend,
                    double_sided: p.double_sided,
                }
            })
            .collect();
        let skeleton = data.skeleton.clone().map(std::sync::Arc::new);
        let rest_joints = skeleton
            .as_ref()
            .map(|s| s.joint_matrices(&s.rest_pose()))
            .unwrap_or_default();
        crate::d3::Model {
            parts,
            bounds: data.bounds(),
            triangles: data.triangle_count(),
            collision: std::sync::Arc::new(data.collision_mesh()),
            skeleton,
            rest_joints: std::sync::Arc::new(rest_joints),
        }
    }

    /// Desenha um modelo, como [`Renderer::draw_mesh_with`].
    pub fn draw_model(
        &mut self,
        model: &crate::d3::Model,
        matrix: Mat4,
        color: Color,
        cast_shadow: bool,
    ) {
        self.draw_model_glow(model, matrix, color, cast_shadow, MODEL_GLOW, None);
    }

    /// Desenha um modelo com brilho próprio e, num modelo animado, na pose
    /// `joints` (`None`: parado). No `glow`, `amount` é quanto da própria cor
    /// brilha (0 a 1) e `color` multiplica o brilho que o arquivo já tem
    /// (branco = como está; preto = apagado, como uma lanterna de dia).
    pub fn draw_model_glow(
        &mut self,
        model: &crate::d3::Model,
        matrix: Mat4,
        color: Color,
        cast_shadow: bool,
        glow: crate::d3::Glow,
        joints: Option<&[Mat4]>,
    ) {
        self.draw_model_material(
            model,
            matrix,
            color,
            cast_shadow,
            crate::d3::Material3D::glow(glow),
            joints,
        );
    }

    /// Como [`Renderer::draw_model_glow`], com o material inteiro (brilho,
    /// água, balançar com o vento).
    pub fn draw_model_material(
        &mut self,
        model: &crate::d3::Model,
        matrix: Mat4,
        color: Color,
        cast_shadow: bool,
        material: crate::d3::Material3D,
        joints: Option<&[Mat4]>,
    ) {
        let rest = model.rest_joints.clone();
        let joints = joints.unwrap_or(&rest);
        // O brilho do arquivo também pega a cor do objeto (tingir um modelo
        // que brilha muda a cor do brilho).
        let k = material.glow.color;
        for part in &model.parts {
            let (mesh, texture, emissive) = (part.mesh, part.texture, part.emissive);
            // O recorte e as duas faces: o do arquivo, ou o que o jogo pedir.
            let cutout = if material.cutout > 0.0 {
                material.cutout
            } else {
                part.cutout
            };
            // Meio transparente no arquivo (BLEND): vai com as transparentes.
            let color = if part.blend && cutout <= 0.0 {
                Color::rgba(color.r, color.g, color.b, color.a.min(0.998))
            } else {
                color
            };
            let m = crate::d3::Material3D {
                cutout,
                double_sided: material.double_sided || part.double_sided,
                glow: crate::d3::Glow {
                    amount: material.glow.amount,
                    color: Color::rgba(
                        emissive.r * k.r * color.r,
                        emissive.g * k.g * color.g,
                        emissive.b * k.b * color.b,
                        1.0,
                    ),
                },
                ..material
            };
            if model.skeleton.is_some() {
                self.draw_mesh_skinned_material(
                    mesh,
                    matrix,
                    color,
                    texture,
                    cast_shadow,
                    joints,
                    m,
                );
            } else {
                self.draw_mesh_material(mesh, matrix, color, texture, cast_shadow, m);
            }
        }
    }

    /// Desenha um modelo animado numa pose: as matrizes dos ossos de
    /// [`Skeleton::joint_matrices`](crate::d3::Skeleton::joint_matrices). Num
    /// modelo sem esqueleto, é o mesmo que [`Renderer::draw_model`].
    pub fn draw_model_posed(
        &mut self,
        model: &crate::d3::Model,
        matrix: Mat4,
        color: Color,
        cast_shadow: bool,
        joints: &[Mat4],
    ) {
        self.draw_model_glow(model, matrix, color, cast_shadow, MODEL_GLOW, Some(joints));
    }

    /// A câmera 3D (vale até ser trocada).
    pub fn set_camera3d(&mut self, camera: crate::d3::Camera3D) {
        self.d3.camera = camera;
    }

    pub fn camera3d(&self) -> crate::d3::Camera3D {
        self.d3.camera
    }

    /// O sol, a luz ambiente e a névoa do mundo 3D.
    pub fn set_light3d(&mut self, light: crate::d3::Light3D) {
        self.d3.light = light;
    }

    pub fn light3d(&self) -> crate::d3::Light3D {
        self.d3.light
    }

    /// O céu em degradê atrás do mundo 3D (`None`: a cor de fundo).
    pub fn set_sky3d(&mut self, sky: Option<crate::d3::Sky3D>) {
        self.d3.sky = sky;
    }

    pub fn sky3d(&self) -> Option<crate::d3::Sky3D> {
        self.d3.sky
    }

    /// Antisserrilhado no 3D (MSAA 4x): as bordas das malhas ficam lisas.
    /// Custa mais memória e tempo da placa; numa placa que não consegue, não
    /// faz nada.
    pub fn set_antialias3d(&mut self, on: bool) {
        self.d3.antialias = on;
    }

    pub fn antialias3d(&self) -> bool {
        self.d3.antialias
    }

    /// Tonemap no 3D: as cores acima do branco (uma luz forte, o sol na
    /// água) são comprimidas em vez de estourar; as do meio quase não mudam.
    pub fn set_tonemap3d(&mut self, on: bool) {
        self.d3.tonemap = on;
    }

    pub fn tonemap3d(&self) -> bool {
        self.d3.tonemap
    }

    /// As sombras do sol.
    pub fn set_shadows3d(&mut self, shadows: crate::d3::Shadows) {
        self.d3.shadows = shadows;
    }

    pub fn shadows3d(&self) -> crate::d3::Shadows {
        self.d3.shadows
    }

    /// Quantas malhas foram pedidas no último frame (as fora da câmera
    /// contam, mas não vão para a GPU; veja [`Renderer::last_stats3`]).
    pub fn last_mesh_count(&self) -> u32 {
        self.d3.last_count
    }

    /// Números do último [`Renderer::render`].
    /// O que o último frame 3D desenhou: malhas pedidas, as que estavam na
    /// frente da câmera, as do mapa de sombras e as chamadas de desenho.
    pub fn last_stats3(&self) -> crate::d3::Stats3 {
        self.d3.last_stats
    }

    pub fn last_stats(&self) -> RenderStats {
        self.last_stats
    }

    /// Memória das texturas na placa de vídeo, em bytes (as imagens e as
    /// páginas de letras; 4 bytes por pixel).
    pub fn texture_bytes(&self) -> u64 {
        self.textures
            .iter()
            .enumerate()
            .filter(|(i, _)| !self.freed.contains(&(*i as u32)))
            .map(|(_, t)| crate::texture::texture_bytes(t.size, t.levels))
            .sum()
    }

    /// Quantas texturas estão carregadas.
    pub fn texture_count(&self) -> usize {
        self.textures.len() - self.freed.len()
    }

    /// Quantas páginas (texturas de 1024×1024) o atlas de glifos já usa.
    pub fn glyph_atlas_pages(&self) -> usize {
        self.text.page_count()
    }
}

/// Círculo branco com borda suavizada, para `draw_circle`.
fn circle_pixels(size: u32) -> (UVec2, Vec<u8>) {
    let r = size as f32 / 2.0;
    let mut px = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let d = Vec2::new(x as f32 + 0.5 - r, y as f32 + 0.5 - r).length();
            let a = (r - d).clamp(0.0, 1.0);
            px.extend_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    (UVec2::splat(size), px)
}

/// Projeção da tela: pixels, origem no canto superior esquerdo, Y para baixo.
fn screen_projection(size: Vec2) -> Mat4 {
    glam::camera::rh::proj::directx::orthographic(0.0, size.x, size.y, 0.0, -1.0, 1.0)
}

fn read_file(path: &Path) -> Result<Vec<u8>, RenderError> {
    std::fs::read(path).map_err(|source| RenderError::Io {
        path: path.to_owned(),
        source,
    })
}

/// Desenha no espaço da tela. Obtido com [`Renderer::screen`].
///
/// Coordenadas em pixels físicos da janela, origem no canto superior
/// esquerdo e Y para baixo. Aqui, a origem (0, 0) de um sprite é o seu canto
/// superior esquerdo e rotações positivas giram no sentido horário.
#[derive(Debug)]
pub struct ScreenPainter<'a> {
    renderer: &'a mut Renderer,
}

impl ScreenPainter<'_> {
    pub fn draw(&mut self, sprite: &Sprite) -> &mut Self {
        self.renderer.push_sprite(Space::Screen, sprite);
        self
    }

    /// Retângulo de cor sólida a partir do canto superior esquerdo.
    pub fn draw_rect(&mut self, top_left: Vec2, size: Vec2, color: Color) -> &mut Self {
        let sprite = Sprite::new(self.renderer.white)
            .at(top_left)
            .size(size)
            .origin(Vec2::ZERO)
            .color(color);
        self.renderer.push_sprite(Space::Screen, &sprite);
        self
    }

    /// Círculo de cor sólida a partir do centro.
    pub fn draw_circle(&mut self, center: Vec2, radius: f32, color: Color) -> &mut Self {
        let sprite = Sprite::new(self.renderer.circle)
            .at(center)
            .size(Vec2::splat(radius * 2.0))
            .color(color);
        self.renderer.push_sprite(Space::Screen, &sprite);
        self
    }

    pub fn draw_text(&mut self, text: &Text<'_>) -> &mut Self {
        self.renderer.push_text(Space::Screen, text);
        self
    }

    /// Painel com centro em `panel.center` (pixels da tela, Y para baixo).
    pub fn draw_panel(&mut self, panel: &crate::Panel) -> &mut Self {
        self.renderer.push_panel(Space::Screen, panel, false);
        self
    }

    /// O recorte do desenho na tela (pixels do jogo, Y para baixo).
    pub fn set_clip(&mut self, clip: Option<ClipRect>) -> &mut Self {
        self.renderer.screen.clip = clip;
        self
    }

    /// O recorte do desenho na tela agora.
    pub fn clip(&self) -> Option<ClipRect> {
        self.renderer.screen.clip
    }

    /// Linha de `a` até `b`, com `width` de espessura.
    pub fn draw_line(&mut self, a: Vec2, b: Vec2, width: f32, color: Color) -> &mut Self {
        self.renderer
            .push_polyline(Space::Screen, &[a, b], width, false, color);
        self
    }

    /// Linha passando pelos pontos (`closed`: volta ao primeiro).
    pub fn draw_polyline(
        &mut self,
        points: &[Vec2],
        width: f32,
        closed: bool,
        color: Color,
    ) -> &mut Self {
        self.renderer
            .push_polyline(Space::Screen, points, width, closed, color);
        self
    }

    /// Polígono preenchido (também côncavo).
    pub fn draw_polygon(&mut self, points: &[Vec2], color: Color) -> &mut Self {
        self.renderer.push_polygon(Space::Screen, points, color);
        self
    }

    /// Contorno de um retângulo a partir do canto superior esquerdo, por
    /// dentro dele.
    pub fn draw_rect_outline(
        &mut self,
        top_left: Vec2,
        size: Vec2,
        width: f32,
        color: Color,
    ) -> &mut Self {
        self.renderer
            .push_rect_outline(Space::Screen, top_left, size, width, color);
        self
    }

    /// Contorno de um círculo, por dentro dele.
    pub fn draw_circle_outline(
        &mut self,
        center: Vec2,
        radius: f32,
        width: f32,
        color: Color,
    ) -> &mut Self {
        self.renderer
            .push_circle_outline(Space::Screen, center, radius, width, color);
        self
    }
}

/// Um recorte do desenho em pixels do alvo, dentro de `base` (ou `None` se
/// ficou sem área). Com a câmera girada, fica a caixa em volta.
fn clip_scissor(
    clip: ClipRect,
    matrix: Mat4,
    vp_origin: Vec2,
    vp_size: Vec2,
    base: Option<(u32, u32, u32, u32)>,
) -> Option<(u32, u32, u32, u32)> {
    let (bx, by, bw, bh) = base?;
    let to_px = |p: Vec2| {
        let ndc = matrix.project_point3(p.extend(0.0));
        vp_origin + Vec2::new((ndc.x + 1.0) * 0.5, (1.0 - ndc.y) * 0.5) * vp_size
    };
    let corners = [
        to_px(clip.min),
        to_px(clip.max),
        to_px(Vec2::new(clip.min.x, clip.max.y)),
        to_px(Vec2::new(clip.max.x, clip.min.y)),
    ];
    let min = corners
        .iter()
        .fold(Vec2::INFINITY, |a, c| a.min(*c))
        .round();
    let max = corners
        .iter()
        .fold(Vec2::NEG_INFINITY, |a, c| a.max(*c))
        .round();
    let lo = min.max(Vec2::new(bx as f32, by as f32));
    let hi = max.min(Vec2::new((bx + bw) as f32, (by + bh) as f32));
    if !(hi.x > lo.x && hi.y > lo.y) {
        return None;
    }
    Some((
        lo.x as u32,
        lo.y as u32,
        (hi.x - lo.x) as u32,
        (hi.y - lo.y) as u32,
    ))
}

/// O retângulo do jogo dentro do alvo, em pixels inteiros (ou `None` se ficou
/// sem área).
fn scissor(p: crate::Presentation, viewport: UVec2) -> Option<(u32, u32, u32, u32)> {
    let vw = viewport.max(UVec2::ONE).as_vec2();
    let min = p.origin.clamp(Vec2::ZERO, vw);
    let max = (p.origin + p.size).clamp(Vec2::ZERO, vw);
    let (x, y) = (min.x as u32, min.y as u32);
    let (w, h) = ((max.x - min.x) as u32, (max.y - min.y) as u32);
    (w > 0 && h > 0).then_some((x, y, w, h))
}
