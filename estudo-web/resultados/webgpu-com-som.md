# Navegador: webgpu-com-som

http://localhost:8000/com-som/index.html?webgpu

- pronto: true (0.28 s desde abrir a página)
- canvas: sim
- AudioContext: running 48000 Hz

```
0.14 [info] WebGPU is experimental on this platform. See https://github.com/gpuweb/gpuweb/wiki/Implementation-Status#implementation-status
0.15 [info] NN gpu  backend BrowserWebGpu driver  
0.15 [info] NN limites da engine servem: true
0.15 [error] Failed to load resource: the server responded with a status of 404 (File not found)
0.16 [info] NN formatos [Bgra8Unorm, Rgba8Unorm, Rgba16Float]
0.16 [info] NN superficie 800x450 Bgra8Unorm
0.21 [info] NN pronto em 87 ms
0.26 [warning] Error while parsing WGSL: :224:29 error: 'textureSample' must only be called from uniform control flow
    let albedo = in.color * textureSample(tex, samp, in.uv);
                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:193:16 note: called by 'water' from 'fs_main'
        return water(in);
               ^^^^^^^^^

:192:5 note: control flow depends on possibly non-uniform value
    if (in.material.x > 0.5) {
    ^^

:192:9 note: parameter 'in' of 'fs_main' may be non-uniform
    if (in.material.x > 0.5) {
        ^^


 - While validating [ShaderModuleDescriptor ""mesh3d.wgsl""]
 - While calling [Device "prova"].CreateShaderModule([ShaderModuleDescriptor ""mesh3d.wgsl""]).

0.26 [warning] Compilation log for [Invalid ShaderModule "mesh3d.wgsl"]:
1 error(s) generated while compiling the shader:
:224:29 error: 'textureSample' must only be called from uniform control flow
    let albedo = in.color * textureSample(tex, samp, in.uv);
                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

:193:16 note: called by 'water' from 'fs_main'
        return water(in);
               ^^^^^^^^^

:192:5 note: control flow depends on possibly non-uniform value
    if (in.material.x > 0.5) {
    ^^

:192:9 note: parameter 'in' of 'fs_main' may be non-uniform
    if (in.material.x > 0.5) {
        ^^

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

0.27 [warning] [Invalid ShaderModule "mesh3d.wgsl"] is invalid.
 - While validating vertex stage ([Invalid ShaderModule "mesh3d.wgsl"], entryPoint: "vs_particle").
 - While validating vertex state.
 - While calling [Device "prova"].CreateRenderPipeline([RenderPipelineDescriptor ""nn 3d particles""]).

0.27 [warning] [Invalid ShaderModule "mesh3d.wgsl"] is invalid.
 - While validating vertex stage ([Invalid ShaderModule "mesh3d.wgsl"], entryPoint: "vs_particle").
 - While validating vertex state.
 - While calling [Device "prova"].CreateRenderPipeline([RenderPipelineDescriptor ""nn 3d particles additive""]).

0.27 [warning] [Invalid ShaderModule "mesh3d.wgsl"] is invalid.
 - While validating vertex stage ([Invalid ShaderModule "mesh3d.wgsl"], entryPoint: "vs_sky").
 - While validating vertex state.
 - While calling [Device "prova"].CreateRenderPipeline([RenderPipelineDescriptor ""nn 3d sky""]).

1.29 [info] NN quadro 60 pos 100 200 cpu_ms 0.62 pior 11.70 desde_inicio_s 1.09
2.30 [info] NN quadro 120 pos 100 200 cpu_ms 0.32 pior 2.50 desde_inicio_s 2.09
2.51 [info] NN clique
2.55 [info] NN som ligado true
2.55 [info] NN som tocou
2.57 [info] NN tecla ArrowRight
3.33 [info] NN quadro 180 pos 286 200 cpu_ms 0.33 pior 3.30 desde_inicio_s 3.12
3.37 [info] NN tecla ArrowDown
4.33 [info] NN quadro 240 pos 294 300 cpu_ms 0.23 pior 1.20 desde_inicio_s 4.12
5.33 [info] NN quadro 300 pos 294 300 cpu_ms 0.24 pior 2.50 desde_inicio_s 5.12
6.33 [info] NN quadro 360 pos 294 300 cpu_ms 0.30 pior 4.00 desde_inicio_s 6.12
```
