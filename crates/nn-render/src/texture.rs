//! Texturas na GPU.

use glam::UVec2;

/// Identificador leve de uma textura registrada no [`crate::Renderer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextureId(pub(crate) u32);

/// Como a textura é amostrada quando aparece maior ou menor que o original.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filter {
    /// Suave. Bom para arte desenhada e fotos.
    #[default]
    Linear,
    /// Pixels nítidos. Ideal para pixel art.
    Nearest,
}

impl From<Filter> for wgpu::FilterMode {
    fn from(f: Filter) -> Self {
        match f {
            Filter::Linear => wgpu::FilterMode::Linear,
            Filter::Nearest => wgpu::FilterMode::Nearest,
        }
    }
}

/// Opções de criação de textura.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TextureOptions {
    pub filter: Filter,
}

impl TextureOptions {
    /// Atalho para pixel art.
    pub const PIXEL_ART: Self = Self {
        filter: Filter::Nearest,
    };
}

#[derive(Debug)]
pub(crate) struct GpuTexture {
    pub(crate) size: UVec2,
    pub(crate) bind_group: wgpu::BindGroup,
    // Mantidos vivos enquanto o bind group existir.
    _texture: wgpu::Texture,
}

impl GpuTexture {
    pub(crate) fn from_rgba8(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        size: UVec2,
        rgba: &[u8],
        options: TextureOptions,
        label: Option<&str>,
    ) -> Self {
        let extent = wgpu::Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * size.x),
                rows_per_image: Some(size.y),
            },
            extent,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let filter = options.filter.into();
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: filter,
            min_filter: filter,
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label,
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            size,
            bind_group,
            _texture: texture,
        }
    }
}
