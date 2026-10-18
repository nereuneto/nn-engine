//! 3D: malhas com luz, câmera em perspectiva e formas prontas.
//!
//! O mundo 3D é desenhado antes de tudo o que é 2D no mesmo
//! [`Renderer::render`](crate::Renderer::render): sprites, texto e
//! `screen()` (HUD, menus) ficam por cima. O visual mira o *low-poly*: cada
//! face recebe uma cor de luz só (normais por face), com o sol (luz
//! direcional) e uma luz ambiente que vem do céu e do chão.
//!
//! ```no_run
//! # fn exemplo(r: &mut nn_render::Renderer) {
//! use glam::{Mat4, Vec3};
//! use nn_core::Color;
//! use nn_render::d3::{self, Camera3D};
//! let cubo = r.create_mesh(&d3::shapes::cube());
//! r.set_camera3d(Camera3D::looking_at(Vec3::new(3.0, 2.0, 4.0), Vec3::ZERO));
//! r.draw_mesh(cubo, Mat4::IDENTITY, Color::hex(0x3FA7D6));
//! # }
//! ```

mod mesh;
pub mod shapes;

use glam::{Mat3, Mat4, Quat, Vec2, Vec3, Vec4};
use nn_core::Color;

pub use mesh::{Mesh, Vertex3};

use crate::TextureId;

/// Câmera em perspectiva: de onde olha, para onde e o campo de visão.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera3D {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    /// Campo de visão vertical, em graus.
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self::looking_at(Vec3::new(0.0, 4.0, 8.0), Vec3::ZERO)
    }
}

impl Camera3D {
    /// Câmera em `position` olhando para `target` (Y para cima, 60°).
    pub fn looking_at(position: Vec3, target: Vec3) -> Self {
        Self {
            position,
            target,
            up: Vec3::Y,
            fov: 60.0,
            near: 0.1,
            far: 1000.0,
        }
    }

    pub fn view(&self) -> Mat4 {
        let up = if (self.target - self.position).normalize_or_zero().cross(self.up).length() < 1e-4
        {
            // Olhando reto para cima ou para baixo: outro "cima".
            Vec3::Z
        } else {
            self.up
        };
        glam::camera::rh::view::look_at_mat4(self.position, self.target, up)
    }

    pub fn projection(&self, aspect: f32) -> Mat4 {
        glam::camera::rh::proj::directx::perspective(
            self.fov.clamp(1.0, 179.0).to_radians(),
            aspect.max(1e-4),
            self.near.max(1e-4),
            self.far.max(self.near + 1e-3),
        )
    }

    /// Matriz que leva do mundo para a tela (com a proporção largura ÷ altura).
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        self.projection(aspect) * self.view()
    }

    /// Onde o ponto `world` aparece na tela de tamanho `size` (pixels, origem
    /// no canto de cima à esquerda). `None` se estiver atrás da câmera.
    pub fn screen_point(&self, world: Vec3, size: Vec2) -> Option<Vec2> {
        let clip = self.view_projection(size.x / size.y.max(1.0)) * world.extend(1.0);
        if clip.w <= 1e-6 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        Some(Vec2::new(
            (ndc.x * 0.5 + 0.5) * size.x,
            (0.5 - ndc.y * 0.5) * size.y,
        ))
    }

    /// O raio que sai da câmera passando pelo ponto `screen` da tela (para
    /// clicar em coisas 3D): origem e direção (normalizada).
    pub fn ray(&self, screen: Vec2, size: Vec2) -> (Vec3, Vec3) {
        let inv = self.view_projection(size.x / size.y.max(1.0)).inverse();
        let ndc = Vec2::new(
            screen.x / size.x.max(1.0) * 2.0 - 1.0,
            1.0 - screen.y / size.y.max(1.0) * 2.0,
        );
        let near = inv.project_point3(ndc.extend(0.0));
        let far = inv.project_point3(ndc.extend(1.0));
        (near, (far - near).normalize_or_zero())
    }
}

/// A luz do mundo 3D: o sol, a luz ambiente e a névoa.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light3D {
    /// Direção para onde a luz do sol vai (de cima para baixo: Y negativo).
    pub sun_direction: Vec3,
    pub sun_color: Color,
    /// Luz ambiente que vem de cima (céu) e de baixo (chão).
    pub sky: Color,
    pub ground: Color,
    /// Névoa: cor, distância em que começa e em que fica total.
    pub fog: Option<(Color, f32, f32)>,
}

impl Default for Light3D {
    fn default() -> Self {
        Self {
            sun_direction: Vec3::new(-0.4, -1.0, -0.6).normalize(),
            sun_color: Color::rgb(1.0, 0.96, 0.88),
            sky: Color::rgb(0.42, 0.47, 0.58),
            ground: Color::rgb(0.22, 0.2, 0.18),
            fog: None,
        }
    }
}

/// Posição, rotação e escala de algo no mundo 3D.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform3 {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform3 {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform3 {
    pub fn at(position: Vec3) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }

    /// Rotação em graus em volta de X, Y e Z (aplicada Y, depois X, depois Z:
    /// virar, inclinar e rolar).
    pub fn rotation_degrees(mut self, x: f32, y: f32, z: f32) -> Self {
        self.rotation = Quat::from_euler(
            glam::EulerRot::YXZ,
            y.to_radians(),
            x.to_radians(),
            z.to_radians(),
        );
        self
    }

    pub fn scaled(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

/// Uma malha enviada à GPU por [`Renderer::create_mesh`](crate::Renderer::create_mesh).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MeshId(pub(crate) u32);

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct ViewUniform3 {
    view_proj: [f32; 16],
    camera: [f32; 4],
    sun_dir: [f32; 4],
    sun_color: [f32; 4],
    sky: [f32; 4],
    ground: [f32; 4],
    fog: [f32; 4],
    fog_range: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Instance3 {
    model: [[f32; 4]; 4],
    normal: [[f32; 4]; 3],
    color: [f32; 4],
}

impl Instance3 {
    fn new(model: Mat4, color: Color, srgb: bool) -> Self {
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
    instance: Instance3,
}

/// A parte 3D do renderer.
pub(crate) struct Renderer3D {
    pipeline: wgpu::RenderPipeline,
    view_buffer: wgpu::Buffer,
    view_group: wgpu::BindGroup,
    meshes: Vec<GpuMesh>,
    draws: Vec<Draw3>,
    instances: wgpu::Buffer,
    capacity: usize,
    depth: Option<(glam::UVec2, wgpu::TextureView)>,
    pub camera: Camera3D,
    pub light: Light3D,
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

pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

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
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nn 3d"),
            bind_group_layouts: &[Some(&view_layout), Some(texture_layout)],
            immediate_size: 0,
        });
        let vertex_attrs = wgpu::vertex_attr_array![
            0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x2
        ];
        let instance_attrs = wgpu::vertex_attr_array![
            4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4,
            8 => Float32x4, 9 => Float32x4, 10 => Float32x4, 11 => Float32x4
        ];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("nn 3d"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
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
                ],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
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
        let capacity = 256;
        let instances = Self::instance_buffer(device, capacity);
        Self {
            pipeline,
            view_buffer,
            view_group,
            meshes: Vec::new(),
            draws: Vec::new(),
            instances,
            capacity,
            depth: None,
            camera: Camera3D::default(),
            light: Light3D::default(),
            last_count: 0,
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

    pub(crate) fn draw(
        &mut self,
        mesh: MeshId,
        model: Mat4,
        color: Color,
        texture: TextureId,
        srgb: bool,
    ) {
        if (mesh.0 as usize) < self.meshes.len() {
            self.draws.push(Draw3 {
                mesh,
                texture,
                instance: Instance3::new(model, color, srgb),
            });
        }
    }

    pub(crate) fn has_draws(&self) -> bool {
        !self.draws.is_empty()
    }

    /// Desenha as malhas da fila em `target` (a cor já foi limpa) e esvazia a
    /// fila. `area` é o retângulo do jogo dentro do alvo (x, y, largura,
    /// altura); `aspect`, a proporção do jogo.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        target_size: glam::UVec2,
        area: (f32, f32, f32, f32),
        aspect: f32,
        srgb: bool,
        textures: &[crate::texture::GpuTexture],
    ) {
        // Profundidade do tamanho do alvo.
        let size = target_size.max(glam::UVec2::ONE);
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
        let l = &self.light;
        let c4 = |c: Color| {
            let c = if srgb { c.to_linear() } else { c };
            [c.r, c.g, c.b, c.a]
        };
        let (fog, fog_range) = match l.fog {
            Some((c, a, b)) => (c4(c), [a, b, 0.0, 0.0]),
            None => ([0.0; 4], [0.0; 4]),
        };
        let view = ViewUniform3 {
            view_proj: self.camera.view_projection(aspect).to_cols_array(),
            camera: self.camera.position.extend(1.0).to_array(),
            sun_dir: l.sun_direction.normalize_or_zero().extend(0.0).to_array(),
            sun_color: c4(l.sun_color),
            sky: c4(l.sky),
            ground: c4(l.ground),
            fog,
            fog_range,
        };
        queue.write_buffer(&self.view_buffer, 0, bytemuck::bytes_of(&view));

        // Agrupa por malha e textura (uma chamada por grupo).
        self.draws
            .sort_by_key(|d| (d.mesh, d.texture.0));
        if self.draws.len() > self.capacity {
            self.capacity = self.draws.len().next_power_of_two();
            self.instances = Self::instance_buffer(device, self.capacity);
        }
        let data: Vec<Instance3> = self.draws.iter().map(|d| d.instance).collect();
        queue.write_buffer(&self.instances, 0, bytemuck::cast_slice(&data));

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
        if w < 1.0 || h < 1.0 {
            drop(pass);
            self.draws.clear();
            return;
        }
        pass.set_viewport(x, y, w, h, 0.0, 1.0);
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.view_group, &[]);
        pass.set_vertex_buffer(1, self.instances.slice(..));
        let mut i = 0;
        while i < self.draws.len() {
            let (mesh, tex) = (self.draws[i].mesh, self.draws[i].texture);
            let mut j = i + 1;
            while j < self.draws.len() && self.draws[j].mesh == mesh && self.draws[j].texture == tex {
                j += 1;
            }
            let m = &self.meshes[mesh.0 as usize];
            if m.count > 0 {
                pass.set_bind_group(1, &textures[tex.0 as usize].bind_group, &[]);
                pass.set_vertex_buffer(0, m.vertices.slice(..));
                pass.set_index_buffer(m.indices.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..m.count, 0, i as u32..j as u32);
            }
            i = j;
        }
        drop(pass);
        self.last_count = self.draws.len() as u32;
        self.draws.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ponto_na_frente_da_camera_cai_no_meio_da_tela() {
        let cam = Camera3D::looking_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO);
        let size = Vec2::new(800.0, 600.0);
        let p = cam.screen_point(Vec3::ZERO, size).unwrap();
        assert!((p - Vec2::new(400.0, 300.0)).length() < 0.01, "{p}");
        // Acima e à direita no mundo: acima e à direita na tela (Y para baixo).
        let q = cam.screen_point(Vec3::new(1.0, 1.0, 0.0), size).unwrap();
        assert!(q.x > 400.0 && q.y < 300.0, "{q}");
        // Atrás da câmera: nada.
        assert!(cam.screen_point(Vec3::new(0.0, 0.0, 10.0), size).is_none());
    }

    #[test]
    fn raio_do_clique_volta_ao_ponto() {
        let cam = Camera3D::looking_at(Vec3::new(3.0, 4.0, 5.0), Vec3::new(0.0, 1.0, 0.0));
        let size = Vec2::new(640.0, 360.0);
        let world = Vec3::new(0.5, 0.2, -0.7);
        let screen = cam.screen_point(world, size).unwrap();
        let (origin, dir) = cam.ray(screen, size);
        // O ponto fica sobre o raio.
        let t = (world - origin).dot(dir);
        assert!(t > 0.0);
        assert!((origin + dir * t - world).length() < 1e-3);
    }

    #[test]
    fn olhar_reto_para_baixo_nao_quebra() {
        let cam = Camera3D::looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::ZERO);
        let m = cam.view_projection(1.0);
        assert!(m.is_finite());
    }

    #[test]
    fn rotacao_em_graus() {
        let t = Transform3::default().rotation_degrees(0.0, 90.0, 0.0);
        // Virar 90° em Y leva a frente (−Z) para a esquerda (−X).
        let v = t.matrix().transform_vector3(Vec3::NEG_Z);
        assert!((v - Vec3::NEG_X).length() < 1e-5, "{v}");
    }

    #[test]
    fn normais_acompanham_a_escala() {
        // Esticado em X, a normal de um plano inclinado não pode esticar junto.
        let model = Mat4::from_scale(Vec3::new(4.0, 1.0, 1.0));
        let inst = Instance3::new(model, Color::WHITE, false);
        let n = Mat3::from_cols(
            Vec4::from(inst.normal[0]).truncate(),
            Vec4::from(inst.normal[1]).truncate(),
            Vec4::from(inst.normal[2]).truncate(),
        );
        let normal = (n * Vec3::new(1.0, 1.0, 0.0).normalize()).normalize();
        // A superfície x + y = c esticada em X vira mais "deitada": normal mais para Y.
        assert!(normal.y > normal.x, "{normal}");
    }
}
