# Navegador: webgpu-com-som

http://localhost:8000/com-som/index.html?webgpu

- pronto: true (0.30 s desde abrir a página)
- canvas: sim
- AudioContext: running 48000 Hz

```
0.15 [info] WebGPU is experimental on this platform. See https://github.com/gpuweb/gpuweb/wiki/Implementation-Status#implementation-status
0.16 [info] NN gpu  backend BrowserWebGpu driver  
0.16 [info] NN limites da engine servem: true
0.16 [error] Failed to load resource: the server responded with a status of 404 (File not found)
0.17 [info] NN formatos [Bgra8Unorm, Rgba8Unorm, Rgba16Float]
0.17 [info] NN superficie 800x450 Bgra8Unorm
0.22 [info] NN pronto em 82 ms
0.27 [warning] Error while parsing WGSL: :357:21 error: smoothstep called with 'low' (0.44999998807907104492) not less than 'high' (0.0)
            let k = smoothstep(0.45, 0.0, length(f)) * (0.55 + 0.45 * star_hash(cell + vec3<f32>(1.7, 3.1, 5.3)));
                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^


 - While validating [ShaderModuleDescriptor ""mesh3d.wgsl""]
 - While calling [Device "prova"].CreateShaderModule([ShaderModuleDescriptor ""mesh3d.wgsl""]).

0.27 [warning] Compilation log for [Invalid ShaderModule "mesh3d.wgsl"]:
1 error(s) generated while compiling the shader:
:357:21 error: smoothstep called with 'low' (0.44999998807907104492) not less than 'high' (0.0)
            let k = smoothstep(0.45, 0.0, length(f)) * (0.55 + 0.45 * star_hash(cell + vec3<f32>(1.7, 3.1, 5.3)));
                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

0.27 [warning] [Invalid ShaderModule "mesh3d.wgsl"] is invalid.
 - While validating vertex stage ([Invalid ShaderModule "mesh3d.wgsl"], entryPoint: "vs_main").
 - While validating vertex state.
 - While calling [Device "prova"].CreateRenderPipeline([RenderPipelineDescriptor ""nn 3d""]).

0.27 [warning] [Invalid ShaderModule "mesh3d.wgsl"] is invalid.
 - While validating vertex stage ([Invalid ShaderModule "mesh3d.wgsl"], entryPoint: "vs_shadow").
 - While validating vertex state.
 - While calling [Device "prova"].CreateRenderPipeline([RenderPipelineDescriptor ""nn 3d shadow""]).

0.27 [warning] [Invalid ShaderModule "mesh3d.wgsl"] is invalid.
 - While validating vertex stage ([Invalid ShaderModule "mesh3d.wgsl"], entryPoint: "vs_main").
 - While validating vertex state.
 - While calling [Device "prova"].CreateRenderPipeline([RenderPipelineDescriptor ""nn 3d transparent""]).

0.27 [warning] [Invalid ShaderModule "mesh3d.wgsl"] is invalid.
 - While validating vertex stage ([Invalid ShaderModule "mesh3d.wgsl"], entryPoint: "vs_skinned").
 - While validating vertex state.
 - While calling [Device "prova"].CreateRenderPipeline([RenderPipelineDescriptor ""nn 3d skinned""]).

0.27 [warning] [Invalid ShaderModule "mesh3d.wgsl"] is invalid.
 - While validating vertex stage ([Invalid ShaderModule "mesh3d.wgsl"], entryPoint: "vs_shadow_skinned").
 - While validating vertex state.
 - While calling [Device "prova"].CreateRenderPipeline([RenderPipelineDescriptor ""nn 3d skinned shadow""]).

0.28 [warning] [Invalid ShaderModule "mesh3d.wgsl"] is invalid.
 - While validating vertex stage ([Invalid ShaderModule "mesh3d.wgsl"], entryPoint: "vs_particle").
 - While validating vertex state.
 - While calling [Device "prova"].CreateRenderPipeline([RenderPipelineDescriptor ""nn 3d particles""]).

0.28 [warning] [Invalid ShaderModule "mesh3d.wgsl"] is invalid.
 - While validating vertex stage ([Invalid ShaderModule "mesh3d.wgsl"], entryPoint: "vs_particle").
 - While validating vertex state.
 - While calling [Device "prova"].CreateRenderPipeline([RenderPipelineDescriptor ""nn 3d particles additive""]).

0.28 [warning] [Invalid ShaderModule "mesh3d.wgsl"] is invalid.
 - While validating vertex stage ([Invalid ShaderModule "mesh3d.wgsl"], entryPoint: "vs_sky").
 - While validating vertex state.
 - While calling [Device "prova"].CreateRenderPipeline([RenderPipelineDescriptor ""nn 3d sky""]).

1.29 [info] NN quadro 60 pos 100 200 cpu_ms 0.72 pior 22.30 desde_inicio_s 1.08
2.29 [info] NN quadro 120 pos 100 200 cpu_ms 0.65 pior 4.70 desde_inicio_s 2.07
2.53 [info] NN clique
2.58 [info] NN som ligado true
2.58 [info] NN som tocou
2.59 [info] NN tecla ArrowRight
3.32 [info] NN quadro 180 pos 277 200 cpu_ms 0.39 pior 3.60 desde_inicio_s 3.11
3.40 [info] NN tecla ArrowDown
4.32 [info] NN quadro 240 pos 293 296 cpu_ms 0.34 pior 3.00 desde_inicio_s 4.11
5.32 [info] NN quadro 300 pos 293 296 cpu_ms 0.38 pior 3.00 desde_inicio_s 5.11
6.32 [info] NN quadro 360 pos 293 296 cpu_ms 0.47 pior 3.00 desde_inicio_s 6.11
```
