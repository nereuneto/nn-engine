//! Conexão com a GPU: instância, adaptador, dispositivo e fila de comandos.

use crate::RenderError;

/// Tudo o que é necessário para conversar com a GPU.
///
/// Um único `GpuContext` é criado pela aplicação e compartilhado por todo o
/// renderer. Em testes e ferramentas, [`GpuContext::new_headless`] cria um
/// contexto sem janela.
#[derive(Debug)]
pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl GpuContext {
    /// Cria o contexto escolhendo um adaptador compatível com `surface`, se informada.
    pub async fn new(
        instance: wgpu::Instance,
        compatible_surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<Self, RenderError> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface,
                ..Default::default()
            })
            .await
            .map_err(|e| RenderError::NoAdapter(e.to_string()))?;

        let info = adapter.get_info();
        log::info!(
            "GPU: {} ({:?}, driver: {} {})",
            info.name,
            info.backend,
            info.driver,
            info.driver_info
        );

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("nn-engine"),
                required_limits: limits_for(&adapter.limits()),
                ..Default::default()
            })
            .await
            .map_err(|e| RenderError::RequestDevice(e.to_string()))?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }

    /// Cria um contexto sem janela, útil para testes e para renderizar em imagem.
    pub async fn new_headless() -> Result<Self, RenderError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        Self::new(instance, None).await
    }
}

/// Os limites que a engine pede à GPU: conservadores, para rodar em qualquer
/// placa com wgpu (inclusive antigas e drivers por software). Onde nem esses
/// existem (WebGL2, OpenGL ES 3.0), os do WebGL2: o renderer só usa uniforms,
/// sem buffers de armazenamento nem compute.
fn limits_for(adapter: &wgpu::Limits) -> wgpu::Limits {
    let downlevel = wgpu::Limits::downlevel_defaults();
    if downlevel.check_limits(adapter) {
        downlevel
    } else {
        wgpu::Limits::downlevel_webgl2_defaults()
    }
    .using_resolution(adapter.clone())
}

#[cfg(test)]
mod tests {
    use super::limits_for;

    #[test]
    fn placa_sem_os_limites_comuns_usa_os_do_webgl2() {
        let webgl = wgpu::Limits::downlevel_webgl2_defaults();
        let l = limits_for(&webgl);
        assert!(l.check_limits(&webgl));
        assert_eq!(l.max_storage_buffers_per_shader_stage, 0);
        let comum = wgpu::Limits::default();
        let l = limits_for(&comum);
        assert_eq!(
            l.max_storage_buffers_per_shader_stage,
            wgpu::Limits::downlevel_defaults().max_storage_buffers_per_shader_stage
        );
        assert_eq!(l.max_texture_dimension_2d, comum.max_texture_dimension_2d);
    }
}
