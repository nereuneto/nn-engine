# Navegador: webgpu-3d

http://localhost:8000/com-som/index.html?webgpu&3d

- pronto: true (0.26 s desde abrir a página)
- canvas: sim
- AudioContext: running 48000 Hz

```
0.06 [error] Failed to load resource: the server responded with a status of 404 (File not found)
0.13 [info] WebGPU is experimental on this platform. See https://github.com/gpuweb/gpuweb/wiki/Implementation-Status#implementation-status
0.14 [info] NN gpu  backend BrowserWebGpu driver  
0.14 [info] NN limites da engine servem: true
0.14 [info] NN formatos [Bgra8Unorm, Rgba8Unorm, Rgba16Float]
0.14 [info] NN superficie 800x450 Bgra8Unorm
0.18 [info] NN pronto em 82 ms
0.24 [warning] Error while parsing WGSL: :224:29 error: 'textureSample' must only be called from uniform control flow
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

0.24 [warning] Compilation log for [Invalid ShaderModule "mesh3d.wgsl"]:
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

0.24 [warning] [Invalid ShaderModule "mesh3d.wgsl"] is invalid.
 - While validating vertex stage ([Invalid ShaderModule "mesh3d.wgsl"], entryPoint: "vs_main").
 - While validating vertex state.
 - While calling [Device "prova"].CreateRenderPipeline([RenderPipelineDescriptor ""nn 3d""]).

0.24 [warning] [Invalid ShaderModule "mesh3d.wgsl"] is invalid.
 - While validating vertex stage ([Invalid ShaderModule "mesh3d.wgsl"], entryPoint: "vs_shadow").
 - While validating vertex state.
 - While calling [Device "prova"].CreateRenderPipeline([RenderPipelineDescriptor ""nn 3d shadow""]).

0.24 [warning] [Invalid ShaderModule "mesh3d.wgsl"] is invalid.
 - While validating vertex stage ([Invalid ShaderModule "mesh3d.wgsl"], entryPoint: "vs_main").
 - While validating vertex state.
 - While calling [Device "prova"].CreateRenderPipeline([RenderPipelineDescriptor ""nn 3d transparent""]).

0.24 [warning] [Invalid ShaderModule "mesh3d.wgsl"] is invalid.
 - While validating vertex stage ([Invalid ShaderModule "mesh3d.wgsl"], entryPoint: "vs_skinned").
 - While validating vertex state.
 - While calling [Device "prova"].CreateRenderPipeline([RenderPipelineDescriptor ""nn 3d skinned""]).

0.24 [warning] [Invalid ShaderModule "mesh3d.wgsl"] is invalid.
 - While validating vertex stage ([Invalid ShaderModule "mesh3d.wgsl"], entryPoint: "vs_shadow_skinned").
 - While validating vertex state.
 - While calling [Device "prova"].CreateRenderPipeline([RenderPipelineDescriptor ""nn 3d skinned shadow""]).

0.24 [warning] [Invalid ShaderModule "mesh3d.wgsl"] is invalid.
 - While validating vertex stage ([Invalid ShaderModule "mesh3d.wgsl"], entryPoint: "vs_particle").
 - While validating vertex state.
 - While calling [Device "prova"].CreateRenderPipeline([RenderPipelineDescriptor ""nn 3d particles""]).

0.24 [warning] [Invalid ShaderModule "mesh3d.wgsl"] is invalid.
 - While validating vertex stage ([Invalid ShaderModule "mesh3d.wgsl"], entryPoint: "vs_particle").
 - While validating vertex state.
 - While calling [Device "prova"].CreateRenderPipeline([RenderPipelineDescriptor ""nn 3d particles additive""]).

0.24 [warning] [Invalid ShaderModule "mesh3d.wgsl"] is invalid.
 - While validating vertex stage ([Invalid ShaderModule "mesh3d.wgsl"], entryPoint: "vs_sky").
 - While validating vertex state.
 - While calling [Device "prova"].CreateRenderPipeline([RenderPipelineDescriptor ""nn 3d sky""]).

0.24 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.24 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.28 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.28 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.29 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.29 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.31 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.31 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.31 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.31 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.32 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.32 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.34 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.34 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.35 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.35 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.37 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.37 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.39 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.39 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.40 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.40 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.42 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.42 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.43 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.43 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.45 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.45 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.47 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.47 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.48 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.48 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.50 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.50 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.52 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.52 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.53 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.53 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.55 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.55 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.57 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.57 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.58 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.58 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.60 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.60 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.62 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.62 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.64 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.64 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.65 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.65 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.67 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.67 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.68 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.68 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.70 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.70 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.72 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.72 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.74 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.74 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.75 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.75 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.77 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.77 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.79 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.79 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.80 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.80 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.82 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.82 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.83 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.83 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.85 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.85 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.87 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.87 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.89 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.89 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.90 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.90 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.92 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.92 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.94 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.94 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.95 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.95 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.97 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.97 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

0.98 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

0.98 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.00 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.00 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.01 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.01 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.03 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.03 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.05 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.05 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.07 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.07 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.08 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.08 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.10 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.10 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.12 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.12 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.13 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.13 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.15 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.15 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.17 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.17 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.18 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.18 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.20 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.20 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.21 [info] NN quadro 60 pos 100 200 cpu_ms 0.74 pior 11.60 desde_inicio_s 1.03
1.22 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.22 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.23 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.23 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.25 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.25 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.27 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.27 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.28 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.28 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.30 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.30 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.32 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.32 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.33 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.33 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.35 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.35 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.37 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.37 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.39 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.39 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.40 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.40 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.42 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.42 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.43 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.44 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.45 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.45 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.47 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.47 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.49 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.49 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.50 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.50 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.52 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.52 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.53 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.53 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.55 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.55 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.57 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.57 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.58 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.58 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.60 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.60 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.62 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.62 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.64 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.64 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.65 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.65 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.67 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.67 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.68 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.68 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.70 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.70 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.72 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.72 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.73 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.73 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.75 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.75 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.77 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.77 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.78 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.78 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.80 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.80 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.82 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.82 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.83 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.83 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.85 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.85 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.87 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.87 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.89 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.89 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.90 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.90 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.92 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.92 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.94 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.94 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.95 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.95 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.97 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.97 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

1.99 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

1.99 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.00 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.00 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.02 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.02 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.03 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.04 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.05 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.05 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.07 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.07 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.08 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.08 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.10 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.10 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.12 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.12 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.13 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.13 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.15 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.15 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.17 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.17 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.19 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.19 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.20 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.20 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.21 [info] NN quadro 120 pos 100 200 cpu_ms 0.43 pior 3.30 desde_inicio_s 2.03
2.22 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.22 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.23 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.23 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.25 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.25 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.27 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.27 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.29 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.29 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.30 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.30 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.32 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.32 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.35 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.35 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.37 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.37 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.38 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.38 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.38 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.38 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.40 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.40 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.41 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.41 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.43 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.43 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.44 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.44 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.46 [info] NN clique
2.49 [info] NN som ligado true
2.49 [info] NN som tocou
2.50 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.50 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.51 [info] NN tecla ArrowRight
2.51 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.51 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.51 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.51 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.52 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.52 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.53 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.53 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.55 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.55 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.57 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.57 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.59 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.59 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.60 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.60 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.62 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.62 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.64 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.64 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.65 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.65 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.67 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.67 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.69 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.69 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.70 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.70 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.72 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.72 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.73 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.73 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.75 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.75 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.77 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.77 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.78 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.78 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.80 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.80 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.82 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.82 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.83 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.83 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.85 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.85 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.87 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.87 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.88 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.88 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.90 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.90 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.92 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.92 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.94 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.94 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.95 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.95 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.97 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.97 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

2.98 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

2.98 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.00 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.00 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.02 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.02 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.03 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.04 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.05 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.05 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.07 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.07 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.09 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.09 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.10 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.10 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.12 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.12 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.13 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.13 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.15 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.15 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.17 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.17 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.18 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.18 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.20 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.20 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.21 [info] NN quadro 180 pos 271 200 cpu_ms 0.53 pior 3.10 desde_inicio_s 3.03
3.22 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.22 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.23 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.23 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.25 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.25 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.27 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.27 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.28 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.28 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.30 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.30 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.32 [info] NN tecla ArrowDown
3.32 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.32 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.33 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.33 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.35 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.35 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.37 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.37 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.38 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.39 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.40 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.40 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.42 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.42 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.43 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.43 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.45 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.45 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.47 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.47 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.48 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.48 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.50 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.50 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.52 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.52 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.53 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.53 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.55 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.55 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.56 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.56 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.58 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.58 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.60 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.60 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.62 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.62 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.63 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.63 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.65 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.65 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.67 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.67 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.68 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.68 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.70 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.70 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.71 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.71 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.73 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.73 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.75 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.75 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.77 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.77 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.79 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.79 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.80 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.80 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.82 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.82 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.84 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.84 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.85 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.85 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.87 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.87 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.88 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.88 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.90 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.90 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.92 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.92 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.93 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.93 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.95 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.95 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.97 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.97 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

3.98 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

3.98 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.00 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.00 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.02 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.02 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.03 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.03 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.05 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.05 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.07 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.07 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.08 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.08 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.10 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.10 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.12 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.12 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.13 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.13 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.15 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.15 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.17 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.17 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.18 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.18 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.20 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.20 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.21 [info] NN quadro 240 pos 291 296 cpu_ms 0.54 pior 4.60 desde_inicio_s 4.03
4.21 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.21 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.24 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.24 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.25 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.25 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.27 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.27 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.28 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.28 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.30 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.30 [warning] [Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"] is invalid.
 - While calling [Queue].Submit([[Invalid CommandBuffer "nn sprite frame" from CommandEncoder "nn sprite frame"]])

4.32 [warning] [Invalid RenderPipeline "nn 3d shadow"] is invalid.
 - While encoding [RenderPassEncoder "nn 3d shadow pass"].SetPipeline([Invalid RenderPipeline "nn 3d shadow"]).
 - While finishing [CommandEncoder "nn sprite frame"].

4.32 [warning] WebGPU: too many warnings, no more warnings will be reported to the console for this GPUDevice.
5.21 [info] NN quadro 300 pos 291 296 cpu_ms 0.65 pior 3.70 desde_inicio_s 5.03
6.21 [info] NN quadro 360 pos 291 296 cpu_ms 0.61 pior 3.30 desde_inicio_s 6.03
```
