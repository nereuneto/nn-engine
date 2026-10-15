//! Renderer de sprites em lote.

use std::path::Path;

use bytemuck::{Pod, Zeroable};
use glam::{UVec2, Vec2};
use nn_core::Color;

use crate::texture::GpuTexture;
use crate::{Camera2D, GpuContext, RenderError, Sprite, TextureId, TextureOptions};

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

/// Sequência de instâncias consecutivas com a mesma textura: vira uma chamada de desenho.
#[derive(Debug, Clone, Copy)]
struct Batch {
    texture: TextureId,
    start: u32,
    end: u32,
}

/// Números do último frame, úteis para medir desempenho.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RenderStats {
    /// Sprites desenhados.
    pub sprites: u32,
    /// Chamadas de desenho enviadas à GPU (uma por troca de textura).
    pub draw_calls: u32,
}

/// Renderer 2D: acumula sprites durante o frame e desenha tudo de uma vez.
///
/// Sprites são desenhados na ordem em que foram enviados (os últimos ficam
/// por cima). Sprites consecutivos com a mesma textura são agrupados em uma
/// única chamada de desenho, então agrupe por textura sempre que possível.
#[derive(Debug)]
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,
    textures: Vec<GpuTexture>,
    white: TextureId,
    instances: Vec<SpriteInstance>,
    batches: Vec<Batch>,
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

        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nn camera"),
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

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nn camera"),
            size: size_of::<[[f32; 4]; 4]>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nn camera"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nn sprite"),
            bind_group_layouts: &[Some(&camera_layout), Some(&texture_layout)],
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
            camera_buffer,
            camera_bind_group,
            texture_layout,
            textures: Vec::new(),
            white: TextureId(0),
            instances: Vec::with_capacity(Self::INITIAL_CAPACITY),
            batches: Vec::new(),
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
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|source| RenderError::Io {
            path: path.to_owned(),
            source,
        })?;
        self.load_texture_bytes(&bytes, options)
    }

    /// Tamanho da textura em pixels.
    pub fn texture_size(&self, id: TextureId) -> UVec2 {
        self.textures[id.0 as usize].size
    }

    /// Enfileira um sprite para o próximo [`Renderer::render`].
    pub fn draw(&mut self, sprite: &Sprite) {
        let texture = self
            .textures
            .get(sprite.texture.0 as usize)
            .expect("TextureId de outro renderer ou inválido");
        let size = sprite.size.unwrap_or_else(|| {
            texture.size.as_vec2() * Vec2::new(sprite.region.w, sprite.region.h)
        });
        let color = if self.format.is_srgb() {
            sprite.color.to_linear()
        } else {
            sprite.color
        };

        let index = self.instances.len() as u32;
        self.instances.push(SpriteInstance {
            position: sprite.position.to_array(),
            size: size.to_array(),
            origin: sprite.origin.to_array(),
            rotation: sprite.rotation,
            region: sprite.region.to_array(),
            color: color.to_array(),
        });

        match self.batches.last_mut() {
            Some(batch) if batch.texture == sprite.texture => batch.end = index + 1,
            _ => self.batches.push(Batch {
                texture: sprite.texture,
                start: index,
                end: index + 1,
            }),
        }
    }

    /// Atalho para desenhar um retângulo de cor sólida centrado em `center`.
    pub fn draw_rect(&mut self, center: Vec2, size: Vec2, color: Color) {
        let sprite = Sprite::new(self.white).at(center).size(size).color(color);
        self.draw(&sprite);
    }

    /// Desenha todos os sprites enfileirados em `target` e esvazia a fila.
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
        let view_proj = camera.view_projection(viewport.as_vec2());
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&view_proj.to_cols_array()),
        );

        if self.instances.len() > self.instance_capacity {
            self.instance_capacity = self.instances.len().next_power_of_two();
            self.instance_buffer =
                Self::create_instance_buffer(&self.device, self.instance_capacity);
        }
        if !self.instances.is_empty() {
            self.queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&self.instances),
            );
        }

        let load = match clear {
            Some(c) => {
                let c = if self.format.is_srgb() {
                    c.to_linear()
                } else {
                    c
                };
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
            pass.set_bind_group(0, &self.camera_bind_group, &[]);
            pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
            for batch in &self.batches {
                let texture = &self.textures[batch.texture.0 as usize];
                pass.set_bind_group(1, &texture.bind_group, &[]);
                pass.draw(0..6, batch.start..batch.end);
            }
        }
        self.queue.submit([encoder.finish()]);

        self.last_stats = RenderStats {
            sprites: self.instances.len() as u32,
            draw_calls: self.batches.len() as u32,
        };
        self.instances.clear();
        self.batches.clear();
        self.last_stats
    }

    /// Números do último [`Renderer::render`].
    pub fn last_stats(&self) -> RenderStats {
        self.last_stats
    }
}
