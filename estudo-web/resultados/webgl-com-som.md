# Navegador: webgl-com-som

http://localhost:8000/com-som/index.html?webgl

- pronto: false (60.05 s desde abrir a página)
- canvas: sim
- AudioContext: nenhum

```
0.11 [info] NN gpu ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0x0000C0DE)), SwiftShader driver) backend Gl driver  WebGL 2.0 (OpenGL ES 3.0 Chromium)
0.11 [info] NN limites da engine servem: false
0.11 [info] NN formatos [Rgba8UnormSrgb, Rgba8Unorm, Rgba16Float]
0.11 [info] NN superficie 1x1 Rgba8UnormSrgb
0.18 [error] Handling wgpu errors as fatal by default
0.18 [error] panicked at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/wgpu-30.0.1/src/backend/wgpu_core.rs:1414:26:
wgpu error: Validation Error

Caused by:
  In Device::create_render_pipeline, label = 'nn 3d'
    Downlevel flags DownlevelFlags(INDEPENDENT_BLEND) are required but not supported on the device.
This is not an invalid use of WebGPU: the underlying API or device does not support enough features to be a fully compliant implementation. A subset of the features can still be used. If you are running this program on native and not in a browser and wish to work around this issue, call Adapter::downlevel_properties or Device::downlevel_properties to get a listing of the features the current platform supports.



Stack:

Error
    at __wbg_new_227d7c05414eb861 (http://localhost:8000/com-som/prova.js:1315:25)
    at http://localhost:8000/com-som/prova_bg.wasm:wasm-function[1676]:0x15a709
    at http://localhost:8000/com-som/prova_bg.wasm:wasm-function[3609]:0x196f28
    at http://localhost:8000/com-som/prova_bg.wasm:wasm-function[940]:0xfda66
    at http://localhost:8000/com-som/prova_bg.wasm:wasm-function[728]:0x847ad
    at http://localhost:8000/com-som/prova_bg.wasm:wasm-function[5884]:0x1b0620
    at http://localhost:8000/com-som/prova_bg.wasm:wasm-function[707]:0x38901
    at http://localhost:8000/com-som/prova_bg.wasm:wasm-function[1409]:0x140dda
    at http://localhost:8000/com-som/prova_bg.wasm:wasm-function[2874]:0x18b8f5
    at http://localhost:8000/com-som/prova_bg.wasm:wasm-function[4190]:0x19d365


0.18 [pageerror] unreachable
```
