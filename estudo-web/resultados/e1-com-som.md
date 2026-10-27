# E1 — prova (com-som)

Compilou em 20 s (sem cache: inclui as dependências).

| arquivo | tamanho | gzip -9 | brotli 11 |
|---|---|---|---|
| wasm (do wasm-bindgen) | 3947 KB | | |
| wasm (wasm-opt -Oz) | 2564 KB | 1057 KB | 844 KB |
| js | 147 KB | 22 KB | 19 KB |

Os maiores pedaços do wasm final (twiggy; sem nomes, então só os tamanhos por seção e as maiores funções):

```
 Shallow Bytes │ Shallow % │ Item
───────────────┼───────────┼─────────────────────
        129977 ┊     4.95% ┊ data[124]
         88735 ┊     3.38% ┊ code[30]
         84639 ┊     3.22% ┊ data[2]
         63998 ┊     2.44% ┊ code[3]
         49964 ┊     1.90% ┊ code[5]
         38922 ┊     1.48% ┊ code[1]
         31931 ┊     1.22% ┊ data[92]
         31056 ┊     1.18% ┊ code[0]
         28734 ┊     1.09% ┊ code[150]
         27581 ┊     1.05% ┊ code[8]
         27314 ┊     1.04% ┊ code[2352]
         27020 ┊     1.03% ┊ data[125]
       1996002 ┊    76.01% ┊ ... and 7283 more.
       2625873 ┊   100.00% ┊ Σ [7295 Total Rows]
```

As maiores crates (twiggy no wasm antes do wasm-opt, que ainda tem os nomes):

```
     1190 KB  (seção) "function names" subsection
      732 KB  (seção) custom section '__wasm_bindgen_unstable'
      611 KB  naga
      548 KB  (seção) data segment ".rodata"
      345 KB  wgpu_core
      263 KB  core
      143 KB  wgpu
      123 KB  wgpu_hal
      113 KB  zune_jpeg
      103 KB  alloc
       99 KB  winit
       74 KB  hashbrown
       54 KB  (outros)
       49 KB  nn_render
       46 KB  png
       33 KB  fontdue
       33 KB  prova
       32 KB  wgpu_types
       32 KB  compiler_builtins
       25 KB  image
       23 KB  ttf_parser
       22 KB  js_sys
       20 KB  kira
       20 KB  (seção) export "__wbindgen_describe___wbg_set_on
       19 KB  codespan_reporting
```
