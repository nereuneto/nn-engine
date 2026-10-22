//! Alvo de renderização fora da tela, com leitura dos pixels de volta para a CPU.
//! Usado em testes automatizados e para gerar capturas de tela.

use glam::UVec2;

use crate::{GpuContext, RenderError};

/// Textura que pode ser usada como alvo do [`crate::Renderer`] e lida depois.
#[derive(Debug)]
pub struct OffscreenTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: UVec2,
}

impl OffscreenTarget {
    /// Formato usado pelos alvos fora da tela.
    pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

    pub fn new(gpu: &GpuContext, size: UVec2) -> Self {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("nn offscreen"),
            size: wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            texture,
            view,
            size,
        }
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn size(&self) -> UVec2 {
        self.size
    }

    /// Copia os pixels para a CPU: RGBA sRGB, 8 bits por canal, linha a linha a
    /// partir do topo. Bloqueia até a GPU terminar.
    pub fn read_rgba(&self, gpu: &GpuContext) -> Result<Vec<u8>, RenderError> {
        read_texture_rgba(gpu, &self.texture, self.size, Self::FORMAT)
    }

    /// Salva o conteúdo atual como PNG.
    pub fn save_png(
        &self,
        gpu: &GpuContext,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), RenderError> {
        let pixels = self.read_rgba(gpu)?;
        image::save_buffer(
            path.as_ref(),
            &pixels,
            self.size.x,
            self.size.y,
            image::ExtendedColorType::Rgba8,
        )?;
        Ok(())
    }
}

/// Copia os pixels de uma textura (com `COPY_SRC`) para a CPU, como RGBA de 8
/// bits por canal, linha a linha a partir do topo. Aceita os formatos RGBA e
/// BGRA de 8 bits (o da janela costuma ser BGRA). Bloqueia até a GPU terminar.
pub fn read_texture_rgba(
    gpu: &GpuContext,
    texture: &wgpu::Texture,
    size: UVec2,
    format: wgpu::TextureFormat,
) -> Result<Vec<u8>, RenderError> {
    use wgpu::TextureFormat as F;
    let bgra = match format {
        F::Rgba8Unorm | F::Rgba8UnormSrgb => false,
        F::Bgra8Unorm | F::Bgra8UnormSrgb => true,
        other => {
            return Err(RenderError::Readback(format!(
                "formato de imagem sem leitura: {other:?}"
            )));
        }
    };
    let unpadded = 4 * size.x;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded = unpadded.div_ceil(align) * align;

    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("nn readback"),
        size: u64::from(padded) * u64::from(size.y),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("nn readback"),
        });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(size.y),
            },
        },
        wgpu::Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit([encoder.finish()]);

    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| RenderError::Readback(e.to_string()))?;
    rx.recv()
        .map_err(|e| RenderError::Readback(e.to_string()))?
        .map_err(|e| RenderError::Readback(e.to_string()))?;

    let data = slice
        .get_mapped_range()
        .map_err(|e| RenderError::Readback(e.to_string()))?;
    let mut pixels = Vec::with_capacity((unpadded * size.y) as usize);
    for row in data.chunks(padded as usize) {
        pixels.extend_from_slice(&row[..unpadded as usize]);
    }
    drop(data);
    buffer.unmap();
    if bgra {
        for px in pixels.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
    }
    Ok(pixels)
}
