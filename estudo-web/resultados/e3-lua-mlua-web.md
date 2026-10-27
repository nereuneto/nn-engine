# E3 — mlua (Lua 5.4 em C) no wasm32-unknown-unknown com a libc do wasi-sdk, rodando no Node

wasm: 409 KB (brotli 140 KB)

imports do wasm:
- env.agora_ms
- wasi_snapshot_preview1.environ_get
- wasi_snapshot_preview1.environ_sizes_get
- wasi_snapshot_preview1.clock_time_get
- wasi_snapshot_preview1.fd_close
- wasi_snapshot_preview1.fd_fdstat_get
- wasi_snapshot_preview1.fd_fdstat_set_flags
- wasi_snapshot_preview1.fd_prestat_get
- wasi_snapshot_preview1.fd_prestat_dir_name
- wasi_snapshot_preview1.fd_read
- wasi_snapshot_preview1.fd_renumber
- wasi_snapshot_preview1.fd_seek
- wasi_snapshot_preview1.fd_write
- wasi_snapshot_preview1.path_open
- wasi_snapshot_preview1.path_remove_directory
- wasi_snapshot_preview1.path_rename
- wasi_snapshot_preview1.path_unlink_file
- wasi_snapshot_preview1.proc_exit
- wasi_snapshot_preview1.random_get

## mlua (Lua 5.4 em C) no wasm32-unknown-unknown

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

bench: 124.1 ms (resultado 163919)

