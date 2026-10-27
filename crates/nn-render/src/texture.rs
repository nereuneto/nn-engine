//! Texturas na GPU.

use glam::UVec2;

/// Identificador leve de uma textura registrada no [`crate::Renderer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextureId(pub(crate) u32);

/// Como a textura é amostrada quando aparece maior ou menor que o original.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TextureOptions {
    pub filter: Filter,
    /// Gera os mipmaps (versões menores da imagem): desenhada menor que o
    /// tamanho dela, a imagem fica lisa em vez de serrilhada. Custa um terço
    /// a mais de memória e só vale com [`Filter::Linear`].
    pub mipmaps: bool,
}

impl TextureOptions {
    /// Atalho para pixel art.
    pub const PIXEL_ART: Self = Self {
        filter: Filter::Nearest,
        mipmaps: false,
    };

    /// Imagem lisa com mipmaps: o padrão das imagens dos jogos em Lua.
    pub const SMOOTH: Self = Self {
        filter: Filter::Linear,
        mipmaps: true,
    };

    fn mip_levels(&self, size: UVec2) -> u32 {
        if self.mipmaps && self.filter == Filter::Linear {
            32 - size.x.max(size.y).max(1).leading_zeros()
        } else {
            1
        }
    }
}

/// Bytes de uma textura RGBA8 com `levels` níveis de mipmap.
pub(crate) fn texture_bytes(size: UVec2, levels: u32) -> u64 {
    let mut total = 0;
    let mut s = size;
    for _ in 0..levels {
        total += u64::from(s.x) * u64::from(s.y) * 4;
        s = (s / 2).max(UVec2::ONE);
    }
    total
}

/// sRGB (0..255) para linear, numa tabela.
fn srgb_to_linear_table() -> [f32; 256] {
    let mut t = [0.0; 256];
    for (i, v) in t.iter_mut().enumerate() {
        let c = i as f32 / 255.0;
        *v = if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        };
    }
    t
}

fn linear_to_srgb(v: f32) -> u8 {
    let c = if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// O próximo nível de mipmap: cada pixel é a média de até 4 do nível de
/// cima, em cor linear e com o alpha pesando (a borda de uma imagem com
/// transparência não escurece).
pub(crate) fn next_mip(size: UVec2, rgba: &[u8], table: &[f32; 256]) -> (UVec2, Vec<u8>) {
    let out_size = (size / 2).max(UVec2::ONE);
    let mut out = vec![0u8; (out_size.x * out_size.y * 4) as usize];
    for y in 0..out_size.y {
        for x in 0..out_size.x {
            let mut acc = [0.0f32; 4];
            let mut n = 0.0;
            for dy in 0..2 {
                for dx in 0..2 {
                    let sx = (x * 2 + dx).min(size.x - 1);
                    let sy = (y * 2 + dy).min(size.y - 1);
                    let i = ((sy * size.x + sx) * 4) as usize;
                    let a = f32::from(rgba[i + 3]) / 255.0;
                    acc[0] += table[rgba[i] as usize] * a;
                    acc[1] += table[rgba[i + 1] as usize] * a;
                    acc[2] += table[rgba[i + 2] as usize] * a;
                    acc[3] += a;
                    n += 1.0;
                }
            }
            let o = ((y * out_size.x + x) * 4) as usize;
            let a = acc[3] / n;
            if acc[3] > 0.0 {
                out[o] = linear_to_srgb(acc[0] / acc[3]);
                out[o + 1] = linear_to_srgb(acc[1] / acc[3]);
                out[o + 2] = linear_to_srgb(acc[2] / acc[3]);
            }
            out[o + 3] = (a * 255.0 + 0.5) as u8;
        }
    }
    (out_size, out)
}

#[derive(Debug)]
pub(crate) struct GpuTexture {
    pub(crate) size: UVec2,
    pub(crate) bind_group: wgpu::BindGroup,
    texture: wgpu::Texture,
    /// Níveis de mipmap (1 = sem).
    pub(crate) levels: u32,
    /// Num canvas, a vista em que se desenha.
    pub(crate) target: Option<wgpu::TextureView>,
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
        let levels = options.mip_levels(size);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: extent,
            mip_level_count: levels,
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
        if levels > 1 {
            let table = srgb_to_linear_table();
            let (mut s, mut data) = (size, rgba.to_vec());
            for level in 1..levels {
                (s, data) = next_mip(s, &data, &table);
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: level,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &data,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(4 * s.x),
                        rows_per_image: Some(s.y),
                    },
                    wgpu::Extent3d {
                        width: s.x,
                        height: s.y,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let filter = options.filter.into();
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: filter,
            min_filter: filter,
            mipmap_filter: if levels > 1 {
                wgpu::MipmapFilterMode::Linear
            } else {
                wgpu::MipmapFilterMode::Nearest
            },
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
            texture,
            levels,
            target: None,
        }
    }

    /// Um canvas: uma textura em que o renderer desenha (no formato dele) e
    /// que se usa como imagem. Começa transparente.
    pub(crate) fn render_target(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        size: UVec2,
        format: wgpu::TextureFormat,
        filter: Filter,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("nn canvas"),
            size: wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let filter = filter.into();
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nn canvas"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: filter,
            min_filter: filter,
            ..Default::default()
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nn canvas"),
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
            texture,
            levels: 1,
            target: Some(view),
        }
    }

    /// Substitui os pixels de uma região da textura (só o nível 0: numa
    /// textura com mipmaps, os menores ficam como estavam).
    pub(crate) fn write(&self, queue: &wgpu::Queue, origin: UVec2, size: UVec2, rgba: &[u8]) {
        if size.x == 0 || size.y == 0 {
            return;
        }
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: origin.x,
                    y: origin.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * size.x),
                rows_per_image: Some(size.y),
            },
            wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
        );
    }
}
