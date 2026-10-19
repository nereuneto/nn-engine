//! A parte do renderer que fala com a GPU no 3D: malhas, céu, sombras.

use glam::{Mat3, Mat4, UVec2, Vec3, Vec4};
use nn_core::Color;

use super::{Camera3D, Light3D, MeshId, Shadows, Sky3D, Vertex3};
use crate::TextureId;
use crate::texture::GpuTexture;

pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ViewUniform3 {
    view_proj: [f32; 16],
    inv_view_proj: [f32; 16],
    light_view_proj: [f32; 16],
    camera: [f32; 4],
    sun_dir: [f32; 4],
    sun_color: [f32; 4],
    sky: [f32; 4],
    ground: [f32; 4],
    fog: [f32; 4],
    fog_range: [f32; 4],
    sky_top: [f32; 4],
    sky_horizon: [f32; 4],
    sky_bottom: [f32; 4],
    sky_params: [f32; 4],
    shadow: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Instance3 {
    model: [[f32; 4]; 4],
    pub(crate) normal: [[f32; 4]; 3],
    color: [f32; 4],
}

impl Instance3 {
    pub(crate) fn new(model: Mat4, color: Color, srgb: bool) -> Self {
        let color = if srgb { color.to_linear() } else { color };
        let n = Mat3::from_mat4(model).inverse().transpose();
        let col = |v: Vec3| Vec4::from((v, 0.0)).to_array();
        Self {
            model: model.to_cols_array_2d(),
            normal: [col(n.x_axis), col(n.y_axis), col(n.z_axis)],
            color: [color.r, color.g, color.b, color.a],
        }
    }
}

struct GpuMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
}

/// Uma malha para desenhar neste frame.
struct Draw3 {
    mesh: MeshId,
    texture: TextureId,
    cast_shadow: bool,
    instance: Instance3,
}

struct ShadowMap {
    resolution: u32,
    view: wgpu::TextureView,
    group: wgpu::BindGroup,
}

/// A parte 3D do renderer.
pub(crate) struct Renderer3D {
    pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    sky_pipeline: wgpu::RenderPipeline,
    view_buffer: wgpu::Buffer,
    view_group: wgpu::BindGroup,
    shadow_layout: wgpu::BindGroupLayout,
    shadow_sampler: wgpu::Sampler,
    shadow_map: ShadowMap,
    meshes: Vec<GpuMesh>,
    draws: Vec<Draw3>,
    instances: wgpu::Buffer,
    capacity: usize,
    depth: Option<(UVec2, wgpu::TextureView)>,
    pub camera: Camera3D,
    pub light: Light3D,
    pub sky: Option<Sky3D>,
    pub shadows: Shadows,
    /// Quantas malhas foram desenhadas no último frame.
    pub last_count: u32,
}

impl std::fmt::Debug for Renderer3D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Renderer3D")
            .field("meshes", &self.meshes.len())
            .field("camera", &self.camera)
            .finish_non_exhaustive()
    }
}

impl Renderer3D {
    pub(crate) fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("mesh3d.wgsl"));
        let view_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nn 3d view"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nn 3d shadow map"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });
        let view_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nn 3d view"),
            size: size_of::<ViewUniform3>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let view_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nn 3d view"),
            layout: &view_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: view_buffer.as_entire_binding(),
            }],
        });

        let vertex_attrs = wgpu::vertex_attr_array![
            0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x2
        ];
        let instance_attrs = wgpu::vertex_attr_array![
            4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4,
            8 => Float32x4, 9 => Float32x4, 10 => Float32x4, 11 => Float32x4
        ];
        let buffers = [
            Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<Vertex3>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &vertex_attrs,
            }),
            Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<Instance3>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &instance_attrs,
            }),
        ];
        let primitive = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        };

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nn 3d"),
            bind_group_layouts: &[
                Some(&view_layout),
                Some(texture_layout),
                Some(&shadow_layout),
            ],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("nn 3d"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            primitive,
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
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

        let view_only = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nn 3d view only"),
            bind_group_layouts: &[Some(&view_layout)],
            immediate_size: 0,
        });
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("nn 3d shadow"),
            layout: Some(&view_only),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_shadow"),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            primitive,
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });
        let sky_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("nn 3d sky"),
            layout: Some(&view_only),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_sky"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_sky"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nn 3d shadow"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let shadow_map = Self::shadow_map(device, &shadow_layout, &shadow_sampler, 1);
        let capacity = 256;
        let instances = Self::instance_buffer(device, capacity);
        Self {
            pipeline,
            shadow_pipeline,
            sky_pipeline,
            view_buffer,
            view_group,
            shadow_layout,
            shadow_sampler,
            shadow_map,
            meshes: Vec::new(),
            draws: Vec::new(),
            instances,
            capacity,
            depth: None,
            camera: Camera3D::default(),
            light: Light3D::default(),
            sky: None,
            shadows: Shadows::default(),
            last_count: 0,
        }
    }

    fn shadow_map(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        resolution: u32,
    ) -> ShadowMap {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("nn 3d shadow map"),
            size: wgpu::Extent3d {
                width: resolution,
                height: resolution,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = tex.create_view(&Default::default());
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nn 3d shadow map"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        });
        ShadowMap {
            resolution,
            view,
            group,
        }
    }

    fn instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nn 3d instances"),
            size: (capacity * size_of::<Instance3>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub(crate) fn create_mesh(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[Vertex3],
        indices: &[u32],
    ) -> MeshId {
        let make = |label: &str, bytes: &[u8], usage: wgpu::BufferUsages| {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: (bytes.len().max(4) as wgpu::BufferAddress).next_multiple_of(4),
                usage: usage | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            if !bytes.is_empty() {
                queue.write_buffer(&buffer, 0, bytes);
            }
            buffer
        };
        let mesh = GpuMesh {
            vertices: make(
                "nn 3d vertices",
                bytemuck::cast_slice(vertices),
                wgpu::BufferUsages::VERTEX,
            ),
            indices: make(
                "nn 3d indices",
                bytemuck::cast_slice(indices),
                wgpu::BufferUsages::INDEX,
            ),
            count: indices.len() as u32,
        };
        self.meshes.push(mesh);
        MeshId(self.meshes.len() as u32 - 1)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw(
        &mut self,
        mesh: MeshId,
        model: Mat4,
        color: Color,
        texture: TextureId,
        cast_shadow: bool,
        srgb: bool,
    ) {
        if (mesh.0 as usize) < self.meshes.len() {
            self.draws.push(Draw3 {
                mesh,
                texture,
                cast_shadow,
                instance: Instance3::new(model, color, srgb),
            });
        }
    }

    pub(crate) fn has_draws(&self) -> bool {
        !self.draws.is_empty()
    }

    /// Matriz do sol: um quadrado de lado `2 × distance` em volta do ponto
    /// para onde a câmera olha, visto da direção da luz.
    fn light_view_proj(&self) -> (Mat4, f32) {
        let r = self.shadows.distance.max(1.0);
        let dir = self.light.sun_direction.normalize_or(Vec3::NEG_Y);
        let up = if dir.cross(Vec3::Y).length() < 1e-3 {
            Vec3::Z
        } else {
            Vec3::Y
        };
        let view = glam::camera::rh::view::look_at_mat4(Vec3::ZERO, dir, up);
        // O centro anda em passos de um texel: a sombra não "treme" quando a
        // câmera se mexe.
        let texel = 2.0 * r / self.shadows.resolution.max(1) as f32;
        let c = view.transform_point3(self.camera.target);
        let c = Vec3::new(
            (c.x / texel).round() * texel,
            (c.y / texel).round() * texel,
            c.z,
        );
        let proj = glam::camera::rh::proj::directx::orthographic(
            c.x - r,
            c.x + r,
            c.y - r,
            c.y + r,
            -c.z - 4.0 * r,
            -c.z + 4.0 * r,
        );
        (proj * view, texel)
    }

    /// Desenha a fila em `target` (a cor já foi limpa) e esvazia a fila.
    /// `area` é o retângulo do jogo dentro do alvo (x, y, largura, altura);
    /// `aspect`, a proporção do jogo.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        target_size: UVec2,
        area: (f32, f32, f32, f32),
        aspect: f32,
        srgb: bool,
        textures: &[GpuTexture],
    ) {
        // Profundidade do tamanho do alvo.
        let size = target_size.max(UVec2::ONE);
        if self.depth.as_ref().is_none_or(|(s, _)| *s != size) {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("nn 3d depth"),
                size: wgpu::Extent3d {
                    width: size.x,
                    height: size.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.depth = Some((size, tex.create_view(&Default::default())));
        }

        let casters = self.draws.iter().any(|d| d.cast_shadow);
        let shadows_on = self.shadows.enabled && casters;
        let resolution = if self.shadows.enabled {
            self.shadows.resolution.clamp(256, 8192)
        } else {
            1
        };
        if self.shadow_map.resolution != resolution {
            self.shadow_map = Self::shadow_map(
                device,
                &self.shadow_layout,
                &self.shadow_sampler,
                resolution,
            );
        }

        let l = &self.light;
        let c4 = |c: Color| {
            let c = if srgb { c.to_linear() } else { c };
            [c.r, c.g, c.b, c.a]
        };
        let (fog, fog_range) = match l.fog {
            Some(f) => {
                let color = f
                    .color
                    .or(self.sky.map(|s| s.horizon))
                    .unwrap_or(l.sky);
                (c4(color), [f.near, f.far.max(f.near + 1e-3), 0.0, 0.0])
            }
            None => ([0.0; 4], [0.0; 4]),
        };
        let view_proj = self.camera.view_projection(aspect);
        let (light_view_proj, texel) = self.light_view_proj();
        let sky = self.sky.unwrap_or_default();
        let view = ViewUniform3 {
            view_proj: view_proj.to_cols_array(),
            inv_view_proj: view_proj.inverse().to_cols_array(),
            light_view_proj: light_view_proj.to_cols_array(),
            camera: self.camera.position.extend(1.0).to_array(),
            sun_dir: l.sun_direction.normalize_or(Vec3::NEG_Y).extend(0.0).to_array(),
            sun_color: c4(l.sun_color),
            sky: c4(l.sky),
            ground: c4(l.ground),
            fog,
            fog_range,
            sky_top: c4(sky.top),
            sky_horizon: c4(sky.horizon),
            sky_bottom: c4(sky.bottom),
            sky_params: [
                if sky.sun_size > 0.0 {
                    (sky.sun_size.min(45.0) * 0.5).to_radians().cos()
                } else {
                    1.0
                },
                if self.sky.is_some() { 1.0 } else { 0.0 },
                0.0,
                0.0,
            ],
            shadow: [
                if shadows_on { 1.0 } else { 0.0 },
                texel,
                self.shadows.strength.clamp(0.0, 1.0),
                0.0,
            ],
        };
        queue.write_buffer(&self.view_buffer, 0, bytemuck::bytes_of(&view));

        // Agrupa por malha, textura e sombra (uma chamada por grupo).
        self.draws
            .sort_by_key(|d| (d.mesh, d.texture.0, !d.cast_shadow));
        if self.draws.len() > self.capacity {
            self.capacity = self.draws.len().next_power_of_two();
            self.instances = Self::instance_buffer(device, self.capacity);
        }
        let data: Vec<Instance3> = self.draws.iter().map(|d| d.instance).collect();
        queue.write_buffer(&self.instances, 0, bytemuck::cast_slice(&data));
        let mut groups = Vec::new();
        let mut i = 0;
        while i < self.draws.len() {
            let d = &self.draws[i];
            let key = (d.mesh, d.texture, d.cast_shadow);
            let mut j = i + 1;
            while j < self.draws.len() {
                let e = &self.draws[j];
                if (e.mesh, e.texture, e.cast_shadow) != key {
                    break;
                }
                j += 1;
            }
            groups.push((key, i as u32..j as u32));
            i = j;
        }

        // 1. O mapa de sombras, visto do sol.
        if shadows_on {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("nn 3d shadow pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_map.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.shadow_pipeline);
            pass.set_bind_group(0, &self.view_group, &[]);
            pass.set_vertex_buffer(1, self.instances.slice(..));
            for ((mesh, _, cast), range) in &groups {
                let m = &self.meshes[mesh.0 as usize];
                if *cast && m.count > 0 {
                    pass.set_vertex_buffer(0, m.vertices.slice(..));
                    pass.set_index_buffer(m.indices.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..m.count, 0, range.clone());
                }
            }
        }

        // 2. O céu e as malhas.
        let depth = &self.depth.as_ref().expect("profundidade").1;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("nn 3d pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        let (x, y, w, h) = area;
        if w >= 1.0 && h >= 1.0 {
            pass.set_viewport(x, y, w, h, 0.0, 1.0);
            pass.set_bind_group(0, &self.view_group, &[]);
            if self.sky.is_some() {
                pass.set_pipeline(&self.sky_pipeline);
                pass.draw(0..3, 0..1);
            }
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(2, &self.shadow_map.group, &[]);
            pass.set_vertex_buffer(1, self.instances.slice(..));
            for ((mesh, tex, _), range) in &groups {
                let m = &self.meshes[mesh.0 as usize];
                if m.count > 0 {
                    pass.set_bind_group(1, &textures[tex.0 as usize].bind_group, &[]);
                    pass.set_vertex_buffer(0, m.vertices.slice(..));
                    pass.set_index_buffer(m.indices.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..m.count, 0, range.clone());
                }
            }
        }
        drop(pass);
        self.last_count = self.draws.len() as u32;
        self.draws.clear();
    }
}
