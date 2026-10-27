# E3 — Lua no desktop (base para comparar)

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

bench: 45.9 ms (resultado 163919)

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

bench: 402.2 ms (resultado 163919)

