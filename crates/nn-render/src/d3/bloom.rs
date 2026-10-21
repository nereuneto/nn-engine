//! O halo do brilho (bloom): o 3D desenha, num segundo alvo, só o que brilha
//! (o brilho próprio, a chama das tochas, o fogo, o sol). Aqui ele é
//! reduzido a 1/4, borrado duas vezes e somado por cima da imagem.

use glam::UVec2;

use super::gpu::GLOW_FORMAT;

/// O halo do brilho no mundo 3D.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bloom {
    pub enabled: bool,
    /// Quanto o halo soma (1 = o normal; 0 = nada).
    pub strength: f32,
}

impl Default for Bloom {
    fn default() -> Self {
        Self {
            enabled: true,
            strength: 1.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    dir: [f32; 2],
    strength: f32,
    _pad: f32,
}

struct Targets {
    size: UVec2,
    /// O que brilha, do tamanho do alvo (o 3D desenha aqui).
    glow: wgpu::TextureView,
    /// 1/4 do tamanho: reduzido e borrado (ida e volta entre os dois).
    a: wgpu::TextureView,
    b: wgpu::TextureView,
    down: wgpu::BindGroup,
    blur_h: wgpu::BindGroup,
    blur_v: wgpu::BindGroup,
    add: wgpu::BindGroup,
}

pub(crate) struct BloomPass {
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    down: wgpu::RenderPipeline,
    blur: wgpu::RenderPipeline,
    add: wgpu::RenderPipeline,
    params_h: wgpu::Buffer,
    params_v: wgpu::Buffer,
    params_add: wgpu::Buffer,
    targets: Option<Targets>,
}

impl BloomPass {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("bloom.wgsl"));
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nn 3d bloom"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nn 3d bloom"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |label: &str,
                        fs: &str,
                        format: wgpu::TextureFormat,
                        blend: Option<wgpu::BlendState>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pl),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_full"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let down = pipeline("nn 3d bloom down", "fs_down", GLOW_FORMAT, None);
        let blur = pipeline("nn 3d bloom blur", "fs_blur", GLOW_FORMAT, None);
        let add = pipeline(
            "nn 3d bloom add",
            "fs_add",
            format,
            Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Zero,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            }),
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nn 3d bloom"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let buffer = |label: &str| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size_of::<Params>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        Self {
            layout,
            sampler,
            down,
            blur,
            add,
            params_h: buffer("nn 3d bloom h"),
            params_v: buffer("nn 3d bloom v"),
            params_add: buffer("nn 3d bloom add"),
            targets: None,
        }
    }

    /// Prepara os alvos para o tamanho `size` (recriados quando muda).
    pub(crate) fn resize(&mut self, device: &wgpu::Device, size: UVec2) {
        let size = size.max(UVec2::ONE);
        if self.targets.as_ref().is_none_or(|t| t.size != size) {
            self.targets = Some(self.make_targets(device, size));
        }
    }

    /// O alvo com o que brilha (depois de [`BloomPass::resize`]).
    pub(crate) fn glow_view(&self) -> &wgpu::TextureView {
        &self.targets.as_ref().expect("BloomPass::resize antes").glow
    }

    fn make_targets(&self, device: &wgpu::Device, size: UVec2) -> Targets {
        let texture = |label: &str, s: UVec2| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: s.x.max(1),
                        height: s.y.max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: GLOW_FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let small = (size / 4).max(UVec2::ONE);
        let glow = texture("nn 3d glow", size);
        let a = texture("nn 3d bloom a", small);
        let b = texture("nn 3d bloom b", small);
        let group = |src: &wgpu::TextureView, params: &wgpu::Buffer| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("nn 3d bloom"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(src),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params.as_entire_binding(),
                    },
                ],
            })
        };
        Targets {
            size,
            down: group(&glow, &self.params_h),
            blur_h: group(&a, &self.params_h),
            blur_v: group(&b, &self.params_v),
            add: group(&a, &self.params_add),
            glow,
            a,
            b,
        }
    }

    /// Reduz, borra e soma o halo em `target` (só na área do jogo).
    pub(crate) fn apply(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        area: (f32, f32, f32, f32),
        bloom: Bloom,
    ) {
        let Some(t) = &self.targets else {
            return;
        };
        let p = |dir: [f32; 2], strength: f32| Params {
            dir,
            strength,
            _pad: 0.0,
        };
        queue.write_buffer(&self.params_h, 0, bytemuck::bytes_of(&p([1.0, 0.0], 0.0)));
        queue.write_buffer(&self.params_v, 0, bytemuck::bytes_of(&p([0.0, 1.0], 0.0)));
        queue.write_buffer(
            &self.params_add,
            0,
            bytemuck::bytes_of(&p(
                [t.size.x as f32, t.size.y as f32],
                bloom.strength.clamp(0.0, 4.0) * 1.6,
            )),
        );
        let mut run = |pipeline: &wgpu::RenderPipeline,
                       group: &wgpu::BindGroup,
                       view: &wgpu::TextureView,
                       load: wgpu::LoadOp<wgpu::Color>,
                       viewport: Option<(f32, f32, f32, f32)>| {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("nn 3d bloom"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
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
            if let Some((x, y, w, h)) = viewport {
                pass.set_viewport(x, y, w, h, 0.0, 1.0);
            }
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.draw(0..3, 0..1);
        };
        let clear = wgpu::LoadOp::Clear(wgpu::Color::BLACK);
        // Reduz para 1/4 e borra duas vezes (o halo fica largo e suave).
        run(&self.down, &t.down, &t.a, clear, None);
        for _ in 0..2 {
            run(&self.blur, &t.blur_h, &t.b, clear, None);
            run(&self.blur, &t.blur_v, &t.a, clear, None);
        }
        // Soma na imagem, só na área do jogo.
        let (x, y, w, h) = area;
        if w >= 1.0 && h >= 1.0 {
            run(
                &self.add,
                &t.add,
                target,
                wgpu::LoadOp::Load,
                Some((x, y, w, h)),
            );
        }
    }
}
