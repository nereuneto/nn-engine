# E3 — piccolo (Rust puro) no wasm32-unknown-unknown, rodando no Node

wasm: 810 KB (brotli 165 KB)

## piccolo 0.3

```
FALHA inteiros e //: type error, expected function, found nil
FALHA goto (via load): type error, expected function, found nil
FALHA string.format: type error, expected function, found nil
FALHA padroes (gsub/find/match): type error, expected table, found string
FALHA table.sort/insert/remove/concat: type error, expected function, found nil
ok    table.unpack/pack
FALHA corrotinas: type error, expected function, found nil
FALHA metatabelas: cannot add values
ok    pcall/error com tabela
FALHA load com <const>: type error, expected function, found nil
FALHA utf8: type error, expected table, found nil
FALHA string.pack: type error, expected function, found nil
ok    math
FALHA os.clock/os.time: type error, expected table, found nil
ok    select e ...
FALHA tostring: assertion failed!
FALHA string.rep/sub/byte/upper: type error, expected table, found string
ok    ipairs/pairs/next
FALHA <close> (sintaxe 5.4, via load): type error, expected function, found nil
FALHA rawget/rawset/rawequal/rawlen: type error, expected function, found nil

```

bench: 1139.4 ms (resultado 163919)

