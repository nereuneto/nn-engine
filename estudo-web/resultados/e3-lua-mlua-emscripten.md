# E3 — mlua (Lua 5.4 em C) no wasm32-unknown-emscripten, rodando no Node

emcc: emcc (Emscripten gcc/clang-like replacement + linker emulating GNU ld) 6.0.13 (e13ee988ce3d87a738dd8f3661d02223308c412a)

- lua_mlua.wasm: 314 KB (brotli 123 KB)
- lua-mlua.js: 62 KB (brotli 15 KB)

## mlua (Lua 5.4 em C)

```
ok    inteiros e //
ok    goto (via load)
ok    string.format
ok    padroes (gsub/find/match)
ok    table.sort/insert/remove/concat
ok    table.unpack/pack
ok    corrotinas
ok    metatabelas
ok    pcall/error com tabela
ok    load com <const>
ok    utf8
ok    string.pack
ok    math
ok    os.clock/os.time
ok    select e ...
ok    tostring
ok    string.rep/sub/byte/upper
ok    ipairs/pairs/next
ok    <close> (sintaxe 5.4, via load)
ok    rawget/rawset/rawequal/rawlen

```

bench: 131.0 ms (resultado 163919)
