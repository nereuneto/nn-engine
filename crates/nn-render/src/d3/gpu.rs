//! A parte do renderer que fala com a GPU no 3D: malhas, céu, sombras.

use glam::{Mat3, Mat4, UVec2, Vec3, Vec4};
use nn_core::Color;

use super::{
    Camera3D, Glow, Lamp3D, Light3D, MAX_JOINTS, MAX_LAMPS, Material3D, MeshId, Shadows,
    SkinVertex, Sky3D, Vertex3, Wind3D,
};
use crate::TextureId;
use crate::texture::GpuTexture;

pub(crate) const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// O alvo com o que brilha (o halo do brilho é feito dele).
pub(crate) const GLOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Bytes das matrizes dos ossos de um desenho (o shader lê `MAX_JOINTS`).
const JOINT_SLOT: u64 = (MAX_JOINTS * 64) as u64;

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
    /// Direita e cima da câmera (para as partículas virarem para ela).
    cam_right: [f32; 4],
    cam_up: [f32; 4],
    /// x = segundos (ondas, vento), y = força do vento, zw = direção dele.
    time_wind: [f32; 4],
}

/// Uma lâmpada como o shader lê.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
struct LampGpu {
    /// Posição e alcance.
    pos_range: [f32; 4],
    /// Cor × força.
    color: [f32; 4],
    /// Direção do holofote e o cosseno da borda de fora.
    dir_cos: [f32; 4],
    /// x = cosseno de onde começa a apagar, y = 1 num holofote.
    cone: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct LampsUniform {
    count: [u32; 4],
    items: [LampGpu; MAX_LAMPS],
}

/// Uma partícula 3D (um quadrado virado para a câmera) ou um pedaço de
/// texto 3D (um retângulo deslocado do centro, virado para a câmera ou com
/// os eixos dados).
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Billboard3 {
    center: [f32; 3],
    /// Altura (metros).
    size: f32,
    color: [f32; 4],
    region: [f32; 4],
    rotation: f32,
    /// Largura (metros).
    width: f32,
    /// Deslocamento do centro, nos eixos do retângulo (metros).
    offset: [f32; 2],
    /// Eixos do retângulo (w = 1); w = 0 usa os da câmera.
    right: [f32; 4],
    up: [f32; 4],
}

/// Um retângulo 3D desenhado como as partículas (os pedaços do
/// [`Renderer::draw_text3d`](crate::Renderer::draw_text3d)).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Quad3 {
    pub center: Vec3,
    pub size: glam::Vec2,
    pub offset: glam::Vec2,
    pub axes: Option<(Vec3, Vec3)>,
}

struct BillboardDraw {
    texture: TextureId,
    additive: bool,
    data: Billboard3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Instance3 {
    model: [[f32; 4]; 4],
    pub(crate) normal: [[f32; 4]; 3],
    color: [f32; 4],
    /// Brilho: rgb = a cor que brilha por cima, a = quanto da própria cor.
    glow: [f32; 4],
    /// x = 1 na água, y = quanto balança com o vento.
    material: [f32; 4],
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
            glow: [0.0; 4],
            material: [0.0; 4],
        }
    }

    pub(crate) fn with_material(self, m: Material3D, srgb: bool) -> Self {
        let mut i = self.with_glow(m.glow, srgb);
        i.material = [if m.water { 1.0 } else { 0.0 }, m.sway.max(0.0), 0.0, 0.0];
        i
    }

    pub(crate) fn with_glow(mut self, glow: Glow, srgb: bool) -> Self {
        let c = if srgb {
            glow.color.to_linear()
        } else {
            glow.color
        };
        self.glow = [c.r, c.g, c.b, glow.amount.max(0.0)];
        self
    }
}

struct GpuMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    count: u32,
    /// A caixa da malha (para não desenhar o que está fora da câmera).
    bounds: Option<(Vec3, Vec3)>,
    /// Os ossos de cada vértice (malhas de modelos animados).
    skin: Option<wgpu::Buffer>,
}

/// Uma malha para desenhar neste frame.
struct Draw3 {
    mesh: MeshId,
    texture: TextureId,
    cast_shadow: bool,
    instance: Instance3,
    /// A caixa no mundo (None: desenha sempre).
    bounds: Option<(Vec3, Vec3)>,
    /// Malha com ossos: onde estão as matrizes dela em `joint_data`.
    joints: Option<u32>,
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
    skinned_pipeline: wgpu::RenderPipeline,
    skinned_shadow_pipeline: wgpu::RenderPipeline,
    billboard_pipeline: wgpu::RenderPipeline,
    /// Malhas meio transparentes (água, vidro): depois das outras, de trás
    /// para a frente, sem escrever a profundidade.
    transparent_pipeline: wgpu::RenderPipeline,
    transparent: Vec<Draw3>,
    billboard_add_pipeline: wgpu::RenderPipeline,
    billboards: Vec<BillboardDraw>,
    billboard_buffer: wgpu::Buffer,
    billboard_capacity: usize,
    joint_layout: wgpu::BindGroupLayout,
    /// As matrizes dos ossos de todos os desenhos com ossos deste frame.
    joint_data: Vec<[f32; 16]>,
    joint_buffer: wgpu::Buffer,
    joint_group: wgpu::BindGroup,
    joint_capacity: u64,
    sky_pipeline: wgpu::RenderPipeline,
    view_buffer: wgpu::Buffer,
    lamp_buffer: wgpu::Buffer,
    view_group: wgpu::BindGroup,
    /// As lâmpadas deste frame.
    lamps: Vec<Lamp3D>,
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
    pub wind: Wind3D,
    pub bloom: super::Bloom,
    bloom_pass: super::bloom::BloomPass,
    /// Os segundos das ondas e do vento (`None`: o relógio desde o começo).
    pub time: Option<f32>,
    clock: std::time::Instant,
    /// Quantas malhas foram desenhadas no último frame.
    pub last_count: u32,
    /// O que o último frame desenhou de fato (veja [`Stats3`]).
    pub last_stats: Stats3,
}

/// Números do último frame 3D (para medir o desempenho).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats3 {
    /// Malhas pedidas (cada parte de um modelo conta uma).
    pub meshes: u32,
    /// Malhas que estavam na frente da câmera (as outras não foram
    /// desenhadas).
    pub visible: u32,
    /// Malhas desenhadas no mapa de sombras (as perto da câmera).
    pub shadow: u32,
    /// Chamadas de desenho enviadas à GPU (sombras incluídas).
    pub draw_calls: u32,
    /// Lâmpadas que valeram (as que a câmera pode ver, até
    /// [`MAX_LAMPS`](super::MAX_LAMPS)).
    pub lamps: u32,
    /// Se o halo do brilho foi desenhado (ligado e com algo brilhando na
    /// tela: sem nada brilhando, o passe do halo é pulado).
    pub halo: bool,
}

/// Os seis planos de uma câmera (ou da luz): x · n + d ≥ 0 é dentro.
#[derive(Debug, Clone, Copy)]
struct Frustum([glam::Vec4; 6]);

impl Frustum {
    /// Planos de uma matriz de projeção × vista (profundidade de 0 a 1).
    fn of(m: Mat4) -> Self {
        let (r0, r1, r2, r3) = (m.row(0), m.row(1), m.row(2), m.row(3));
        let planes = [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r2, r3 - r2].map(|p| {
            let n = p.truncate().length();
            if n > 0.0 { p / n } else { p }
        });
        Self(planes)
    }

    /// A esfera encosta (ou está dentro)?
    fn touches_sphere(&self, center: Vec3, radius: f32) -> bool {
        self.0
            .iter()
            .all(|p| p.truncate().dot(center) + p.w >= -radius)
    }

    /// A caixa encosta (ou está dentro)?
    fn touches(&self, (min, max): (Vec3, Vec3)) -> bool {
        self.0.iter().all(|p| {
            let n = p.truncate();
            let far = Vec3::select(n.cmpge(Vec3::ZERO), max, min);
            n.dot(far) + p.w >= 0.0
        })
    }
}

/// A caixa `bounds` (do espaço da malha) vista no mundo pela matriz `m`.
fn world_bounds(m: &Mat4, (min, max): (Vec3, Vec3)) -> (Vec3, Vec3) {
    let c = m.transform_point3((min + max) * 0.5);
    let e = (max - min) * 0.5;
    let ext = Vec3::new(
        m.x_axis.x.abs() * e.x + m.y_axis.x.abs() * e.y + m.z_axis.x.abs() * e.z,
        m.x_axis.y.abs() * e.x + m.y_axis.y.abs() * e.y + m.z_axis.y.abs() * e.z,
        m.x_axis.z.abs() * e.x + m.y_axis.z.abs() * e.y + m.z_axis.z.abs() * e.z,
    );
    (c - ext, c + ext)
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
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
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
        let lamp_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nn 3d lamps"),
            size: size_of::<LampsUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let view_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nn 3d view"),
            layout: &view_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: view_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: lamp_buffer.as_entire_binding(),
                },
            ],
        });

        let vertex_attrs = wgpu::vertex_attr_array![
            0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x2
        ];
        let instance_attrs = wgpu::vertex_attr_array![
            4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4,
            8 => Float32x4, 9 => Float32x4, 10 => Float32x4, 11 => Float32x4,
            14 => Float32x4, 15 => Float32x4
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
        // Malhas com ossos: mais um buffer com os ossos de cada vértice e as
        // matrizes dos ossos num uniform (um pedaço por desenho).
        let skin_attrs = wgpu::vertex_attr_array![12 => Uint32x4, 13 => Float32x4];
        let skinned_buffers = [
            buffers[0].clone(),
            buffers[1].clone(),
            Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<SkinVertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &skin_attrs,
            }),
        ];
        let joint_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nn 3d joints"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(JOINT_SLOT),
                },
                count: None,
            }],
        });

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
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    // O que brilha (para o halo).
                    Some(wgpu::ColorTargetState {
                        format: GLOW_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
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
        let transparent_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("nn 3d transparent"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &buffers,
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..primitive
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    // O que brilha (para o halo).
                    Some(wgpu::ColorTargetState {
                        format: GLOW_FORMAT,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
            }),
            multiview_mask: None,
            cache: None,
        });
        let skinned_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nn 3d skinned"),
            bind_group_layouts: &[
                Some(&view_layout),
                Some(texture_layout),
                Some(&shadow_layout),
                Some(&joint_layout),
            ],
            immediate_size: 0,
        });
        let skinned_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("nn 3d skinned"),
            layout: Some(&skinned_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_skinned"),
                compilation_options: Default::default(),
                buffers: &skinned_buffers,
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
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    // O que brilha (para o halo).
                    Some(wgpu::ColorTargetState {
                        format: GLOW_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
            }),
            multiview_mask: None,
            cache: None,
        });
        let skinned_shadow_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("nn 3d skinned shadow"),
                bind_group_layouts: &[Some(&view_layout), None, None, Some(&joint_layout)],
                immediate_size: 0,
            });
        let skinned_shadow_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("nn 3d skinned shadow"),
                layout: Some(&skinned_shadow_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_shadow_skinned"),
                    compilation_options: Default::default(),
                    buffers: &skinned_buffers,
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
        // Partículas: quadrados virados para a câmera, sem escrever a
        // profundidade (as de trás aparecem pelas da frente).
        let billboard_attrs = wgpu::vertex_attr_array![
            0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4,
            4 => Float32x4, 5 => Float32x4
        ];
        let billboard_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nn 3d particles"),
            bind_group_layouts: &[Some(&view_layout), Some(texture_layout)],
            immediate_size: 0,
        });
        let make_billboard = |label: &str, fs: &str, blend: wgpu::BlendState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&billboard_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_particle"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: size_of::<Billboard3>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &billboard_attrs,
                    })],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[
                        Some(wgpu::ColorTargetState {
                            format,
                            blend: Some(blend),
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                        // O que brilha (para o halo).
                        Some(wgpu::ColorTargetState {
                            format: GLOW_FORMAT,
                            blend: Some(blend),
                            write_mask: wgpu::ColorWrites::ALL,
                        }),
                    ],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let billboard_pipeline = make_billboard(
            "nn 3d particles",
            "fs_particle",
            wgpu::BlendState::ALPHA_BLENDING,
        );
        let billboard_add_pipeline = make_billboard(
            "nn 3d particles additive",
            "fs_particle_add",
            wgpu::BlendState {
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
            },
        );
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
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    // O que brilha (para o halo).
                    Some(wgpu::ColorTargetState {
                        format: GLOW_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
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
        let joint_capacity = 4;
        let (joint_buffer, joint_group) = Self::joint_buffer(device, &joint_layout, joint_capacity);
        let billboard_capacity = 256;
        let billboard_buffer = Self::billboard_buffer(device, billboard_capacity);
        Self {
            pipeline,
            shadow_pipeline,
            skinned_pipeline,
            skinned_shadow_pipeline,
            billboard_pipeline,
            billboard_add_pipeline,
            transparent_pipeline,
            transparent: Vec::new(),
            billboards: Vec::new(),
            billboard_buffer,
            billboard_capacity,
            joint_layout,
            joint_data: Vec::new(),
            joint_buffer,
            joint_group,
            joint_capacity,
            sky_pipeline,
            view_buffer,
            lamp_buffer,
            view_group,
            lamps: Vec::new(),
            shadow_layout,
            shadow_sampler,
            shadow_map,
            meshes: Vec::new(),
            last_stats: Stats3::default(),
            draws: Vec::new(),
            instances,
            capacity,
            depth: None,
            camera: Camera3D::default(),
            light: Light3D::default(),
            sky: None,
            shadows: Shadows::default(),
            wind: Wind3D::default(),
            bloom: super::Bloom::default(),
            bloom_pass: super::bloom::BloomPass::new(device, format),
            time: None,
            clock: std::time::Instant::now(),
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

    /// Buffer das matrizes dos ossos com lugar para `slots` desenhos.
    fn joint_buffer(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        slots: u64,
    ) -> (wgpu::Buffer, wgpu::BindGroup) {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nn 3d joints"),
            size: slots * JOINT_SLOT,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nn 3d joints"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(JOINT_SLOT),
                }),
            }],
        });
        (buffer, group)
    }

    fn billboard_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nn 3d particles"),
            size: (capacity * size_of::<Billboard3>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Uma partícula 3D para este frame.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn billboard(
        &mut self,
        texture: TextureId,
        center: Vec3,
        size: f32,
        rotation: f32,
        color: Color,
        region: crate::Rect,
        additive: bool,
        srgb: bool,
    ) {
        let quad = Quad3 {
            center,
            size: glam::Vec2::splat(size),
            offset: glam::Vec2::ZERO,
            axes: None,
        };
        self.quad(texture, quad, rotation, color, region, additive, srgb);
        // Partículas (fumaça, poeira) recebem a luz do mundo: de noite, mais
        // escuras. (Os textos 3D, não.)
        if let Some(b) = self.billboards.last_mut() {
            b.data.up[3] = 2.0;
        }
    }

    /// Um retângulo 3D (partícula ou pedaço de texto) para este frame.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn quad(
        &mut self,
        texture: TextureId,
        quad: Quad3,
        rotation: f32,
        color: Color,
        region: crate::Rect,
        additive: bool,
        srgb: bool,
    ) {
        let color = if srgb { color.to_linear() } else { color };
        let (right, up) = match quad.axes {
            Some((r, u)) => (r.extend(1.0), u.extend(1.0)),
            None => (glam::Vec4::ZERO, glam::Vec4::ZERO),
        };
        self.billboards.push(BillboardDraw {
            texture,
            additive,
            data: Billboard3 {
                center: quad.center.to_array(),
                size: quad.size.y,
                color: [color.r, color.g, color.b, color.a],
                region: [region.x, region.y, region.w, region.h],
                rotation,
                width: quad.size.x,
                offset: quad.offset.to_array(),
                right: right.to_array(),
                up: up.to_array(),
            },
        });
    }

    fn instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("nn 3d instances"),
            size: (capacity * size_of::<Instance3>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Troca o conteúdo de uma malha (mesmo id), sem gastar mais memória a
    /// cada troca (um terreno sendo pintado).
    pub(crate) fn replace_mesh(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        id: MeshId,
        vertices: &[Vertex3],
        indices: &[u32],
    ) {
        if (id.0 as usize) >= self.meshes.len() {
            return;
        }
        let new = self.create_mesh(device, queue, vertices, indices, None);
        let mesh = self.meshes.pop().expect("criada agora");
        debug_assert_eq!(new.0 as usize, self.meshes.len());
        self.meshes[id.0 as usize] = mesh;
    }

    pub(crate) fn create_mesh(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[Vertex3],
        indices: &[u32],
        skin: Option<&[SkinVertex]>,
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
            bounds: vertices
                .iter()
                .map(|v| Vec3::from_array(v.position))
                .fold(None, |acc: Option<(Vec3, Vec3)>, p| {
                    Some(match acc {
                        Some((a, b)) => (a.min(p), b.max(p)),
                        None => (p, p),
                    })
                })
                .map(|(a, b)| {
                    if skin.is_some() {
                        // Animada: os ossos mexem os vértices; uma folga.
                        let pad = (b - a) * 0.5 + Vec3::splat(0.5);
                        (a - pad, b + pad)
                    } else {
                        (a, b)
                    }
                }),
            skin: skin.map(|s| {
                make(
                    "nn 3d skin",
                    bytemuck::cast_slice(s),
                    wgpu::BufferUsages::VERTEX,
                )
            }),
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
        material: Material3D,
    ) {
        if let Some(m) = self.meshes.get(mesh.0 as usize) {
            // Balançando, a caixa cresce um pouco (para não sumir da câmera).
            let pad = material.sway.min(4.0) * 0.6;
            let draw = Draw3 {
                mesh,
                texture,
                cast_shadow,
                instance: Instance3::new(model, color, srgb).with_material(material, srgb),
                bounds: m.bounds.map(|b| {
                    let (a, c) = world_bounds(&model, b);
                    (a - Vec3::splat(pad), c + Vec3::splat(pad))
                }),
                joints: None,
            };
            if color.a < 0.999 {
                self.transparent.push(Draw3 {
                    cast_shadow: false,
                    ..draw
                });
            } else {
                self.draws.push(draw);
            }
        }
    }

    /// Desenha uma malha com ossos nas posições `joints` (uma matriz por
    /// osso; as que faltam ficam paradas).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_skinned(
        &mut self,
        mesh: MeshId,
        model: Mat4,
        color: Color,
        texture: TextureId,
        cast_shadow: bool,
        srgb: bool,
        joints: &[Mat4],
        material: Material3D,
    ) {
        let Some(m) = self.meshes.get(mesh.0 as usize) else {
            return;
        };
        if m.skin.is_none() {
            self.draw(mesh, model, color, texture, cast_shadow, srgb, material);
            return;
        }
        let slot = (self.joint_data.len() / MAX_JOINTS) as u32;
        for i in 0..MAX_JOINTS {
            let j = joints.get(i).copied().unwrap_or(Mat4::IDENTITY);
            self.joint_data.push(j.to_cols_array());
        }
        let bounds = m.bounds.map(|b| world_bounds(&model, b));
        self.draws.push(Draw3 {
            mesh,
            texture,
            cast_shadow,
            instance: Instance3::new(model, color, srgb).with_material(material, srgb),
            bounds,
            joints: Some(slot),
        });
    }

    /// Uma lâmpada para este frame.
    pub(crate) fn lamp(&mut self, lamp: Lamp3D) {
        if lamp.range > 0.0 && lamp.intensity > 0.0 {
            self.lamps.push(lamp);
        }
    }

    /// As lâmpadas que vão para a GPU: as que a câmera pode ver (a esfera
    /// do alcance encosta no que ela vê), as mais perto primeiro.
    fn lamps_uniform(&self, camera: &Frustum, srgb: bool) -> LampsUniform {
        let eye = self.camera.position;
        let mut list: Vec<(f32, &Lamp3D)> = self
            .lamps
            .iter()
            .filter(|l| camera.touches_sphere(l.position, l.range))
            .map(|l| ((l.position.distance(eye) - l.range).max(0.0), l))
            .collect();
        list.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut out = LampsUniform {
            count: [0; 4],
            items: [LampGpu::default(); MAX_LAMPS],
        };
        for (slot, (_, l)) in out.items.iter_mut().zip(list.iter()) {
            let c = if srgb { l.color.to_linear() } else { l.color };
            let k = l.intensity.max(0.0);
            let (dir, cos_out, cos_in, spot) = match l.cone {
                Some(cone) => {
                    let outer = cone.angle.clamp(1.0, 89.0).to_radians();
                    let inner = outer * (1.0 - cone.softness.clamp(0.0, 1.0));
                    (
                        cone.direction.normalize_or(Vec3::NEG_Y),
                        outer.cos(),
                        inner.cos().max(outer.cos() + 1e-4),
                        1.0,
                    )
                }
                None => (Vec3::NEG_Y, -1.0, -1.0, 0.0),
            };
            *slot = LampGpu {
                pos_range: l.position.extend(l.range.max(0.01)).to_array(),
                color: [c.r * k, c.g * k, c.b * k, 1.0],
                dir_cos: dir.extend(cos_out).to_array(),
                cone: [cos_in, spot, 0.0, 0.0],
            };
        }
        out.count[0] = list.len().min(MAX_LAMPS) as u32;
        out
    }

    pub(crate) fn has_draws(&self) -> bool {
        !self.draws.is_empty() || !self.billboards.is_empty() || !self.transparent.is_empty()
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
                let color = f.color.or(self.sky.map(|s| s.horizon)).unwrap_or(l.sky);
                (c4(color), [f.near, f.far.max(f.near + 1e-3), 0.0, 0.0])
            }
            None => ([0.0; 4], [0.0; 4]),
        };
        let view_proj = self.camera.view_projection(aspect);
        let forward = (self.camera.target - self.camera.position).normalize_or(Vec3::NEG_Z);
        let cam_right = forward.cross(self.camera.up).normalize_or(Vec3::X);
        let cam_up = cam_right.cross(forward).normalize_or(Vec3::Y);
        let (light_view_proj, texel) = self.light_view_proj();
        let sky = self.sky.unwrap_or_default();
        let view = ViewUniform3 {
            view_proj: view_proj.to_cols_array(),
            inv_view_proj: view_proj.inverse().to_cols_array(),
            light_view_proj: light_view_proj.to_cols_array(),
            camera: self.camera.position.extend(1.0).to_array(),
            sun_dir: l
                .sun_direction
                .normalize_or(Vec3::NEG_Y)
                .extend(0.0)
                .to_array(),
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
                sky.stars.clamp(0.0, 1.0),
                0.0,
            ],
            shadow: [
                if shadows_on { 1.0 } else { 0.0 },
                texel,
                self.shadows.strength.clamp(0.0, 1.0),
                0.0,
            ],
            cam_right: cam_right.extend(0.0).to_array(),
            cam_up: cam_up.extend(0.0).to_array(),
            time_wind: {
                let d = self.wind.direction.normalize_or(glam::Vec2::X);
                [
                    self.time
                        .unwrap_or_else(|| self.clock.elapsed().as_secs_f32()),
                    self.wind.strength.max(0.0),
                    d.x,
                    d.y,
                ]
            },
        };
        queue.write_buffer(&self.view_buffer, 0, bytemuck::bytes_of(&view));
        let lamps = self.lamps_uniform(&Frustum::of(view_proj), srgb);
        queue.write_buffer(&self.lamp_buffer, 0, bytemuck::bytes_of(&lamps));

        // Agrupa por malha, textura e sombra (uma chamada por grupo); os com
        // ossos vão no fim, um por chamada.
        self.draws
            .sort_by_key(|d| (d.joints.is_some(), d.mesh, d.texture.0, !d.cast_shadow));
        if !self.joint_data.is_empty() {
            let slots = (self.joint_data.len() / MAX_JOINTS) as u64;
            if slots > self.joint_capacity {
                self.joint_capacity = slots.next_power_of_two();
                let (b, g) = Self::joint_buffer(device, &self.joint_layout, self.joint_capacity);
                self.joint_buffer = b;
                self.joint_group = g;
            }
            queue.write_buffer(
                &self.joint_buffer,
                0,
                bytemuck::cast_slice(&self.joint_data),
            );
        }
        // Só o que a câmera vê (e, para as sombras, o que está na caixa do
        // sol) vai para a GPU.
        let camera = Frustum::of(view_proj);
        let sun = Frustum::of(light_view_proj);
        let seen = |f: &Frustum, d: &Draw3| d.bounds.is_none_or(|b| f.touches(b));
        let visible: Vec<usize> = (0..self.draws.len())
            .filter(|&i| seen(&camera, &self.draws[i]))
            .collect();
        let shadow: Vec<usize> = if shadows_on {
            (0..self.draws.len())
                .filter(|&i| self.draws[i].cast_shadow && seen(&sun, &self.draws[i]))
                .collect()
        } else {
            Vec::new()
        };
        // As transparentes vão depois de todas, de trás para a frente.
        let eye3 = self.camera.position;
        let center = |d: &Draw3| {
            Vec3::new(
                d.instance.model[3][0],
                d.instance.model[3][1],
                d.instance.model[3][2],
            )
        };
        let before = self.transparent.len();
        self.transparent
            .retain(|d| d.bounds.is_none_or(|b| camera.touches(b)));
        let culled_transparent = (before - self.transparent.len()) as u32;
        self.transparent.sort_by(|a, b| {
            center(b)
                .distance_squared(eye3)
                .total_cmp(&center(a).distance_squared(eye3))
        });
        // As instâncias: as vistas, as das sombras e as transparentes.
        let total = visible.len() + shadow.len() + self.transparent.len();
        if total > self.capacity {
            self.capacity = total.next_power_of_two();
            self.instances = Self::instance_buffer(device, self.capacity);
        }
        let data: Vec<Instance3> = visible
            .iter()
            .chain(&shadow)
            .map(|&i| self.draws[i].instance)
            .chain(self.transparent.iter().map(|d| d.instance))
            .collect();
        queue.write_buffer(&self.instances, 0, bytemuck::cast_slice(&data));
        // Grupos (malha, textura, sombra) seguidos numa lista; `base` é onde
        // a lista começa nas instâncias. Os com ossos ficam de fora (um por
        // chamada).
        type Group = ((MeshId, TextureId, bool), std::ops::Range<u32>);
        let group = |list: &[usize], base: u32| -> (Vec<Group>, Vec<(usize, u32)>) {
            let mut groups: Vec<Group> = Vec::new();
            let mut skinned = Vec::new();
            for (k, &i) in list.iter().enumerate() {
                let d = &self.draws[i];
                let at = base + k as u32;
                if d.joints.is_some() {
                    skinned.push((i, at));
                    continue;
                }
                let key = (d.mesh, d.texture, d.cast_shadow);
                match groups.last_mut() {
                    Some((k2, r)) if *k2 == key && r.end == at => r.end = at + 1,
                    _ => groups.push((key, at..at + 1)),
                }
            }
            (groups, skinned)
        };
        let (groups, skinned) = group(&visible, 0);
        let (shadow_groups, shadow_skinned) = group(&shadow, visible.len() as u32);
        let mut calls = 0u32;

        // As partículas: as transparentes de trás para a frente, depois as
        // que somam luz (a ordem não importa), agrupadas por imagem.
        let eye = self.camera.position;
        let dist =
            |b: &BillboardDraw| -> f32 { Vec3::from_array(b.data.center).distance_squared(eye) };
        self.billboards.sort_by(|a, b| {
            a.additive.cmp(&b.additive).then_with(|| {
                if a.additive {
                    a.texture.0.cmp(&b.texture.0)
                } else {
                    dist(b).total_cmp(&dist(a))
                }
            })
        });
        if self.billboards.len() > self.billboard_capacity {
            self.billboard_capacity = self.billboards.len().next_power_of_two();
            self.billboard_buffer = Self::billboard_buffer(device, self.billboard_capacity);
        }
        if !self.billboards.is_empty() {
            let data: Vec<Billboard3> = self.billboards.iter().map(|b| b.data).collect();
            queue.write_buffer(&self.billboard_buffer, 0, bytemuck::cast_slice(&data));
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
            for ((mesh, _, _), range) in &shadow_groups {
                let m = &self.meshes[mesh.0 as usize];
                if m.count > 0 {
                    pass.set_vertex_buffer(0, m.vertices.slice(..));
                    pass.set_index_buffer(m.indices.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..m.count, 0, range.clone());
                    calls += 1;
                }
            }
            pass.set_pipeline(&self.skinned_shadow_pipeline);
            for &(i, at) in &shadow_skinned {
                let d = &self.draws[i];
                let m = &self.meshes[d.mesh.0 as usize];
                if let (Some(slot), Some(skin)) = (d.joints, &m.skin)
                    && m.count > 0
                {
                    let offset = (slot as u64 * JOINT_SLOT) as u32;
                    pass.set_bind_group(3, &self.joint_group, &[offset]);
                    pass.set_vertex_buffer(0, m.vertices.slice(..));
                    pass.set_vertex_buffer(2, skin.slice(..));
                    pass.set_index_buffer(m.indices.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..m.count, 0, at..at + 1);
                    calls += 1;
                }
            }
        }

        // 2. O céu e as malhas (e, no segundo alvo, o que brilha).
        self.bloom_pass.resize(device, size);
        let depth = &self.depth.as_ref().expect("profundidade").1;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("nn 3d pass"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                }),
                Some(wgpu::RenderPassColorAttachment {
                    view: self.bloom_pass.glow_view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                }),
            ],
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
                    calls += 1;
                }
            }
            if !skinned.is_empty() {
                pass.set_pipeline(&self.skinned_pipeline);
                for &(i, at) in &skinned {
                    let d = &self.draws[i];
                    let m = &self.meshes[d.mesh.0 as usize];
                    if let (Some(slot), Some(skin)) = (d.joints, &m.skin)
                        && m.count > 0
                    {
                        let offset = (slot as u64 * JOINT_SLOT) as u32;
                        pass.set_bind_group(1, &textures[d.texture.0 as usize].bind_group, &[]);
                        pass.set_bind_group(3, &self.joint_group, &[offset]);
                        pass.set_vertex_buffer(0, m.vertices.slice(..));
                        pass.set_vertex_buffer(2, skin.slice(..));
                        pass.set_index_buffer(m.indices.slice(..), wgpu::IndexFormat::Uint32);
                        pass.draw_indexed(0..m.count, 0, at..at + 1);
                        calls += 1;
                    }
                }
            }
            // 3. As transparentes (água, vidro).
            if !self.transparent.is_empty() {
                pass.set_pipeline(&self.transparent_pipeline);
                pass.set_vertex_buffer(1, self.instances.slice(..));
                let base = (visible.len() + shadow.len()) as u32;
                for (k, d) in self.transparent.iter().enumerate() {
                    let m = &self.meshes[d.mesh.0 as usize];
                    if m.count > 0 {
                        let i = base + k as u32;
                        pass.set_bind_group(1, &textures[d.texture.0 as usize].bind_group, &[]);
                        pass.set_vertex_buffer(0, m.vertices.slice(..));
                        pass.set_index_buffer(m.indices.slice(..), wgpu::IndexFormat::Uint32);
                        pass.draw_indexed(0..m.count, 0, i..i + 1);
                    }
                }
            }
            // 4. As partículas, por cima do que já tem profundidade.
            let mut i = 0;
            while i < self.billboards.len() {
                let (tex, add) = (self.billboards[i].texture, self.billboards[i].additive);
                let mut j = i + 1;
                while j < self.billboards.len()
                    && self.billboards[j].texture == tex
                    && self.billboards[j].additive == add
                {
                    j += 1;
                }
                pass.set_pipeline(if add {
                    &self.billboard_add_pipeline
                } else {
                    &self.billboard_pipeline
                });
                pass.set_bind_group(1, &textures[tex.0 as usize].bind_group, &[]);
                pass.set_vertex_buffer(0, self.billboard_buffer.slice(..));
                pass.draw(0..6, i as u32..j as u32);
                i = j;
            }
        }
        drop(pass);
        // 5. O halo do brilho, por cima: só se algo na tela brilha (o
        // brilho das malhas, a água, o fogo, o disco do sol ou da lua).
        let glows = |i: &Instance3| i.glow.iter().any(|&g| g > 0.0) || i.material[0] > 0.5;
        let sun_seen = self.sky.is_some_and(|s| s.sun_size > 0.0) && {
            let d = -self.light.sun_direction.normalize_or(Vec3::NEG_Y);
            let c = view_proj * d.extend(0.0);
            c.w > 0.0 && (c.x / c.w).abs() < 1.25 && (c.y / c.w).abs() < 1.25
        };
        let halo = self.bloom.enabled
            && self.bloom.strength > 0.0
            && (sun_seen
                || self.billboards.iter().any(|b| b.additive)
                || visible.iter().any(|&i| glows(&self.draws[i].instance))
                || self.transparent.iter().any(|d| glows(&d.instance)));
        if halo {
            self.bloom_pass
                .apply(queue, encoder, target, area, self.bloom);
        }
        self.last_count = (self.draws.len() + self.transparent.len()) as u32 + culled_transparent;
        self.last_stats = Stats3 {
            meshes: self.last_count,
            visible: (visible.len() + self.transparent.len()) as u32,
            shadow: shadow.len() as u32,
            draw_calls: calls + self.transparent.len() as u32,
            lamps: lamps.count[0],
            halo,
        };
        self.draws.clear();
        self.joint_data.clear();
        self.billboards.clear();
        self.transparent.clear();
        self.lamps.clear();
    }
}
