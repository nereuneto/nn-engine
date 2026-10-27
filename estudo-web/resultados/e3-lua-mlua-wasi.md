# E3 — mlua (Lua 5.4 em C) no wasm32-wasip1 (wasi-sdk), rodando no wasmtime

wasm: 419 KB (brotli 143 KB)

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
FALHA os.clock/os.time: [string "teste.lua"]:71: not supported on WASI
ok    select e ...
ok    tostring
ok    string.rep/sub/byte/upper
ok    ipairs/pairs/next
ok    <close> (sintaxe 5.4, via load)
ok    rawget/rawset/rawequal/rawlen

```

bench: 81.8 ms (resultado 163919)
