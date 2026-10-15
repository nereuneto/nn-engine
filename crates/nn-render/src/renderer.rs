//! Renderer de sprites em lote.

use std::path::Path;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, UVec2, Vec2};
use nn_core::Color;

use crate::text::{FontId, Text, TextSystem};
use crate::texture::GpuTexture;
use crate::{Camera2D, GpuContext, RenderError, Sprite, SpriteSheet, TextureId, TextureOptions};

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
}

impl SpriteInstance {
    const ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x2,
        3 => Float32,
        4 => Float32x4,
        5 => Float32x4,
    ];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
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

/// Sequência de instâncias consecutivas com a mesma textura: vira uma chamada de desenho.
#[derive(Debug, Clone, Copy)]
struct Batch {
    texture: TextureId,
    start: u32,
    end: u32,
}

#[derive(Debug, Default)]
struct DrawQueue {
    instances: Vec<SpriteInstance>,
    batches: Vec<Batch>,
}

impl DrawQueue {
    fn push(&mut self, texture: TextureId, instance: SpriteInstance) {
        let index = self.instances.len() as u32;
        self.instances.push(instance);
        match self.batches.last_mut() {
            Some(batch) if batch.texture == texture => batch.end = index + 1,
            _ => self.batches.push(Batch {
                texture,
                start: index,
                end: index + 1,
            }),
        }
    }

    fn clear(&mut self) {
        self.instances.clear();
        self.batches.clear();
    }
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
    world_view: View,
    screen_view: View,
    texture_layout: wgpu::BindGroupLayout,
    textures: Vec<GpuTexture>,
    white: TextureId,
    text: TextSystem,
    world: DrawQueue,
    screen: DrawQueue,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    last_stats: RenderStats,
}

impl Renderer {
    const INITIAL_CAPACITY: usize = 1024;

    /// Cria o renderer para desenhar em alvos com o formato `format`.
    pub fn new(gpu: &GpuContext, format: wgpu::TextureFormat) -> Self {
        let device = gpu.device.clone();
        let queue = gpu.queue.clone();

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
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("nn sprite"),
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
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let instance_buffer = Self::create_instance_buffer(&device, Self::INITIAL_CAPACITY);

        let mut renderer = Self {
            device,
            queue,
            format,
            pipeline,
            world_view,
            screen_view,
            texture_layout,
            textures: Vec::new(),
            white: TextureId(0),
            text: TextSystem::new(),
            world: DrawQueue::default(),
            screen: DrawQueue::default(),
            instance_buffer,
            instance_capacity: Self::INITIAL_CAPACITY,
            last_stats: RenderStats::default(),
        };
        renderer.white =
            renderer.create_texture(UVec2::ONE, &[255, 255, 255, 255], TextureOptions::default());
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
        let id = TextureId(self.textures.len() as u32);
        self.textures.push(GpuTexture::from_rgba8(
            &self.device,
            &self.queue,
            &self.texture_layout,
            size,
            rgba,
            options,
            None,
        ));
        id
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
        self.text.layout(text).size
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
        };
        self.queue_mut(space).push(sprite.texture, instance);
    }

    fn push_text(&mut self, space: Space, text: &Text<'_>) {
        let layout = self.text.layout(text);
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
                // Pixels inteiros deixam as letras nítidas.
                Space::Screen => ((text.position + local).round(), Vec2::ZERO),
                Space::World => (
                    (text.position + Vec2::new(local.x, -local.y)).round(),
                    Vec2::new(0.0, 1.0),
                ),
            };
            let instance = SpriteInstance {
                position: position.to_array(),
                size: placed.size.to_array(),
                origin: origin.to_array(),
                rotation: 0.0,
                region: glyph.region.to_array(),
                color,
            };
            self.queue_mut(space).push(glyph.texture, instance);
        }
    }

    fn queue_mut(&mut self, space: Space) -> &mut DrawQueue {
        match space {
            Space::World => &mut self.world,
            Space::Screen => &mut self.screen,
        }
    }

    fn gpu_color(&self, color: Color) -> Color {
        if self.format.is_srgb() {
            color.to_linear()
        } else {
            color
        }
    }

    /// Desenha tudo o que foi enfileirado em `target` e esvazia as filas.
    ///
    /// `viewport` é o tamanho de `target` em pixels. Se `clear` for `Some`,
    /// o alvo é limpo com essa cor antes; caso contrário o conteúdo é mantido.
    pub fn render(
        &mut self,
        target: &wgpu::TextureView,
        viewport: UVec2,
        camera: &Camera2D,
        clear: Option<Color>,
    ) -> RenderStats {
        let world = ViewUniform {
            view_proj: camera.view_projection(viewport.as_vec2()).to_cols_array(),
            y_down: 0.0,
            _padding: [0.0; 3],
        };
        let size = viewport.max(UVec2::ONE).as_vec2();
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

        let load = match clear {
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
            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, self.instance_buffer.slice(..));

            let passes = [
                (&self.world_view, &self.world, 0u32),
                (&self.screen_view, &self.screen, world_count as u32),
            ];
            for (view, queue, base) in passes {
                if queue.batches.is_empty() {
                    continue;
                }
                pass.set_bind_group(0, &view.bind_group, &[]);
                for batch in &queue.batches {
                    let texture = &self.textures[batch.texture.0 as usize];
                    pass.set_bind_group(1, &texture.bind_group, &[]);
                    pass.draw(0..6, base + batch.start..base + batch.end);
                }
            }
        }
        self.queue.submit([encoder.finish()]);

        self.last_stats = RenderStats {
            sprites: total as u32,
            draw_calls: (self.world.batches.len() + self.screen.batches.len()) as u32,
        };
        self.world.clear();
        self.screen.clear();
        self.last_stats
    }

    /// Números do último [`Renderer::render`].
    pub fn last_stats(&self) -> RenderStats {
        self.last_stats
    }

    /// Quantas páginas (texturas de 1024×1024) o atlas de glifos já usa.
    pub fn glyph_atlas_pages(&self) -> usize {
        self.text.page_count()
    }
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

    pub fn draw_text(&mut self, text: &Text<'_>) -> &mut Self {
        self.renderer.push_text(Space::Screen, text);
        self
    }
}
