# E1 — prova (com-som)

Compilou em 20 s (sem cache: inclui as dependências).

| arquivo | tamanho | gzip -9 | brotli 11 |
|---|---|---|---|
| wasm (do wasm-bindgen) | 3939 KB | | |
| wasm (wasm-opt -Oz) | 2557 KB | 1054 KB | 841 KB |
| js | 147 KB | 22 KB | 19 KB |

Os maiores pedaços do wasm final (twiggy; sem nomes, então só os tamanhos por seção e as maiores funções):

```
 Shallow Bytes │ Shallow % │ Item
───────────────┼───────────┼─────────────────────
        129977 ┊     4.96% ┊ data[121]
         88735 ┊     3.39% ┊ code[30]
         84639 ┊     3.23% ┊ data[2]
         61001 ┊     2.33% ┊ code[4]
         47531 ┊     1.82% ┊ code[5]
         38922 ┊     1.49% ┊ code[1]
         31931 ┊     1.22% ┊ data[90]
         31056 ┊     1.19% ┊ code[0]
         28734 ┊     1.10% ┊ code[150]
         27581 ┊     1.05% ┊ code[8]
         27314 ┊     1.04% ┊ code[2345]
         27024 ┊     1.03% ┊ data[122]
       1994144 ┊    76.15% ┊ ... and 7268 more.
       2618589 ┊   100.00% ┊ Σ [7280 Total Rows]
```

As maiores crates (twiggy no wasm antes do wasm-opt, que ainda tem os nomes):

```
     1188 KB  (seção) "function names" subsection
      732 KB  (seção) custom section '__wasm_bindgen_unstable'
      611 KB  naga
      547 KB  (seção) data segment ".rodata"
      345 KB  wgpu_core
      263 KB  core
      143 KB  wgpu
      123 KB  wgpu_hal
      113 KB  zune_jpeg
      102 KB  alloc
       97 KB  winit
       74 KB  hashbrown
       54 KB  (outros)
       48 KB  nn_render
       46 KB  png
       33 KB  fontdue
       32 KB  wgpu_types
       32 KB  compiler_builtins
       30 KB  prova
       25 KB  image
       23 KB  ttf_parser
       22 KB  js_sys
       20 KB  kira
       20 KB  (seção) export "__wbindgen_describe___wbg_set_on
       19 KB  codespan_reporting
```
