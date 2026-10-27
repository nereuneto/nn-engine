# E1 — prova (sem-som)

Compilou em 19 s (sem cache: inclui as dependências).

| arquivo | tamanho | gzip -9 | brotli 11 |
|---|---|---|---|
| wasm (do wasm-bindgen) | 3821 KB | | |
| wasm (wasm-opt -Oz) | 2491 KB | 1023 KB | 817 KB |
| js | 140 KB | 20 KB | 17 KB |

Os maiores pedaços do wasm final (twiggy; sem nomes, então só os tamanhos por seção e as maiores funções):

```
 Shallow Bytes │ Shallow % │ Item
───────────────┼───────────┼─────────────────────
        129977 ┊     5.09% ┊ data[112]
         88735 ┊     3.48% ┊ code[29]
         82299 ┊     3.23% ┊ data[2]
         61066 ┊     2.39% ┊ code[4]
         38924 ┊     1.53% ┊ code[1]
         36190 ┊     1.42% ┊ code[28]
         31655 ┊     1.24% ┊ data[81]
         31056 ┊     1.22% ┊ code[0]
         28734 ┊     1.13% ┊ code[147]
         27581 ┊     1.08% ┊ code[7]
         27314 ┊     1.07% ┊ code[2246]
         27024 ┊     1.06% ┊ data[113]
       1940652 ┊    76.07% ┊ ... and 6947 more.
       2551207 ┊   100.00% ┊ Σ [6959 Total Rows]
```

As maiores crates (twiggy no wasm antes do wasm-opt, que ainda tem os nomes):

```
     1144 KB  (seção) "function names" subsection
      709 KB  (seção) custom section '__wasm_bindgen_unstable'
      611 KB  naga
      540 KB  (seção) data segment ".rodata"
      345 KB  wgpu_core
      255 KB  core
      143 KB  wgpu
      123 KB  wgpu_hal
      113 KB  zune_jpeg
       98 KB  alloc
       85 KB  winit
       74 KB  hashbrown
       52 KB  (outros)
       48 KB  nn_render
       46 KB  png
       33 KB  fontdue
       32 KB  wgpu_types
       32 KB  compiler_builtins
       30 KB  prova
       25 KB  image
       23 KB  ttf_parser
       22 KB  js_sys
       20 KB  (seção) export "__wbindgen_describe___wbg_set_on
       19 KB  codespan_reporting
       14 KB  (seção) export "__wbindgen_describe___wbg_unifor
```
