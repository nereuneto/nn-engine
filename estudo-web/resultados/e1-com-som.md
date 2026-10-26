# E1 — prova (com-som)

Compilou em 54 s (sem cache: inclui as dependências).

| arquivo | tamanho | gzip -9 | brotli 11 |
|---|---|---|---|
| wasm (do wasm-bindgen) | 3358 KB | | |
| wasm (wasm-opt -Oz) | 2076 KB | 845 KB | 669 KB |
| js | 147 KB | 22 KB | 19 KB |

Os maiores pedaços do wasm (twiggy):

```
 Shallow Bytes │ Shallow % │ Item
───────────────┼───────────┼───────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
       1139606 ┊    23.54% ┊ "function names" subsection
        749924 ┊    15.49% ┊ custom section '__wasm_bindgen_unstable'
        346618 ┊     7.16% ┊ data segment ".rodata"
         47669 ┊     0.98% ┊ <wgpu_core[5e1a0d8057df918d]::command::CommandEncoder>::encode_commands
         41067 ┊     0.85% ┊ wgpu_core[5e1a0d8057df918d]::command::render::encode_render_pass
         31214 ┊     0.64% ┊ <naga[8eb75bf1d9ad936c]::valid::Validator>::validate_impl
         30430 ┊     0.63% ┊ <naga[8eb75bf1d9ad936c]::front::wgsl::lower::Lowerer>::call_builtin
         28760 ┊     0.59% ┊ <wgpu_hal[922b0f73e8853165]::gles::Device>::create_pipeline
         28246 ┊     0.58% ┊ <naga[8eb75bf1d9ad936c]::back::glsl::writer::Writer<&mut alloc[42d2b33bd937efdb]::string::String>>::write_expr
         25480 ┊     0.53% ┊ <naga[8eb75bf1d9ad936c]::front::wgsl::error::Error>::as_parse_error
         20086 ┊     0.41% ┊ <winit[9e86a71fe5eb6452]::platform_impl::web::event_loop::EventLoop<prova[a1d4ea92127974c3]::Evento>>::spawn::<<winit[9e86a71fe5eb6452]::event_loop::EventLoop<prova[a1d4ea92127974c3]::Evento> as winit[9e86a71fe5eb6452]::platform::web::EventLoopExtWebSys>::spawn_app<prova[a1d4ea92127974c3]::App>::{closure#0}>::{closure#0}
         19367 ┊     0.40% ┊ <prova[a1d4ea92127974c3]::App as winit[9e86a71fe5eb6452]::application::ApplicationHandler<prova[a1d4ea92127974c3]::Evento>>::resumed::{closure#0}
         17968 ┊     0.37% ┊ <nn_render[ca5335774ecedc78]::renderer::Renderer>::render
         17266 ┊     0.36% ┊ <naga[8eb75bf1d9ad936c]::front::wgsl::Frontend>::inner
         16725 ┊     0.35% ┊ <naga[8eb75bf1d9ad936c]::valid::Validator>::validate_block_impl
         16148 ┊     0.33% ┊ <wgpu_hal[922b0f73e8853165]::gles::web::Instance as wgpu_hal[922b0f73e8853165]::Instance>::enumerate_adapters
         15919 ┊     0.33% ┊ <naga[8eb75bf1d9ad936c]::proc::constant_evaluator::ConstantEvaluator>::try_eval_and_append_impl
         15667 ┊     0.32% ┊ <wgpu_hal[922b0f73e8853165]::gles::Queue>::process
         15594 ┊     0.32% ┊ <naga[8eb75bf1d9ad936c]::valid::Validator>::validate_function
         15566 ┊     0.32% ┊ <wgpu_core[5e1a0d8057df918d]::device::queue::Queue>::submit
         15519 ┊     0.32% ┊ <wgpu_core[5e1a0d8057df918d]::global::Global>::adapter_request_device
         14698 ┊     0.30% ┊ <wgpu_core[5e1a0d8057df918d]::device::resource::Device>::create_render_pipeline_inner
         14677 ┊     0.30% ┊ <naga[8eb75bf1d9ad936c]::valid::Validator>::validate_expression
         14508 ┊     0.30% ┊ naga[8eb75bf1d9ad936c]::compact::compact
         14334 ┊     0.30% ┊ <naga[8eb75bf1d9ad936c]::back::glsl::writer::Writer<&mut alloc[42d2b33bd937efdb]::string::String>>::write_stmt
       2127080 ┊    43.95% ┊ ... and 14739 more.
       4840136 ┊   100.00% ┊ Σ [14764 Total Rows]
```
