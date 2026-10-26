# E1 — prova (sem-som)

Compilou em 72 s (sem cache: inclui as dependências).

| arquivo | tamanho | gzip -9 | brotli 11 |
|---|---|---|---|
| wasm (do wasm-bindgen) | 3234 KB | | |
| wasm (wasm-opt -Oz) | 2007 KB | 814 KB | 643 KB |
| js | 140 KB | 20 KB | 17 KB |

Os maiores pedaços do wasm (twiggy):

```
 Shallow Bytes │ Shallow % │ Item
───────────────┼───────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
       1092575 ┊    23.36% ┊ "function names" subsection
        725937 ┊    15.52% ┊ custom section '__wasm_bindgen_unstable'
        338794 ┊     7.24% ┊ data segment ".rodata"
         47669 ┊     1.02% ┊ <wgpu_core[6f0373605e7e6da7]::command::CommandEncoder>::encode_commands
         41067 ┊     0.88% ┊ wgpu_core[6f0373605e7e6da7]::command::render::encode_render_pass
         31214 ┊     0.67% ┊ <naga[7885fd17e26ecd4b]::valid::Validator>::validate_impl
         30430 ┊     0.65% ┊ <naga[7885fd17e26ecd4b]::front::wgsl::lower::Lowerer>::call_builtin
         28760 ┊     0.61% ┊ <wgpu_hal[8c87abe60c623157]::gles::Device>::create_pipeline
         28246 ┊     0.60% ┊ <naga[7885fd17e26ecd4b]::back::glsl::writer::Writer<&mut alloc[42d2b33bd937efdb]::string::String>>::write_expr
         25480 ┊     0.54% ┊ <naga[7885fd17e26ecd4b]::front::wgsl::error::Error>::as_parse_error
         19371 ┊     0.41% ┊ <prova[c31d915f8e3b8027]::App as winit[e3bb41461ffdc95]::application::ApplicationHandler<prova[c31d915f8e3b8027]::Evento>>::resumed::{closure#0}
         17964 ┊     0.38% ┊ <nn_render[d7f7b0733ae050cc]::renderer::Renderer>::render
         17266 ┊     0.37% ┊ <naga[7885fd17e26ecd4b]::front::wgsl::Frontend>::inner
         16725 ┊     0.36% ┊ <naga[7885fd17e26ecd4b]::valid::Validator>::validate_block_impl
         16301 ┊     0.35% ┊ <wgpu_hal[8c87abe60c623157]::gles::web::Instance as wgpu_hal[8c87abe60c623157]::Instance>::enumerate_adapters
         15919 ┊     0.34% ┊ <naga[7885fd17e26ecd4b]::proc::constant_evaluator::ConstantEvaluator>::try_eval_and_append_impl
         15667 ┊     0.33% ┊ <wgpu_hal[8c87abe60c623157]::gles::Queue>::process
         15594 ┊     0.33% ┊ <naga[7885fd17e26ecd4b]::valid::Validator>::validate_function
         15566 ┊     0.33% ┊ <wgpu_core[6f0373605e7e6da7]::device::queue::Queue>::submit
         15519 ┊     0.33% ┊ <wgpu_core[6f0373605e7e6da7]::global::Global>::adapter_request_device
         14698 ┊     0.31% ┊ <wgpu_core[6f0373605e7e6da7]::device::resource::Device>::create_render_pipeline_inner
         14677 ┊     0.31% ┊ <naga[7885fd17e26ecd4b]::valid::Validator>::validate_expression
         14508 ┊     0.31% ┊ naga[7885fd17e26ecd4b]::compact::compact
         14334 ┊     0.31% ┊ <naga[7885fd17e26ecd4b]::back::glsl::writer::Writer<&mut alloc[42d2b33bd937efdb]::string::String>>::write_stmt
         12174 ┊     0.26% ┊ naga[7885fd17e26ecd4b]::back::pipeline_constants::process_overrides
       2050388 ┊    43.84% ┊ ... and 14253 more.
       4676843 ┊   100.00% ┊ Σ [14278 Total Rows]
```
