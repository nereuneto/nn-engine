# E3 — mlua (Lua 5.4 em C) no wasm32-unknown-unknown com a libc do wasi-sdk, rodando no Node

wasm: 409 KB (brotli 140 KB)

(node:12291) ExperimentalWarning: WASI is an experimental feature and might change at any time
(Use `node --trace-warnings ...` to show where the warning was created)
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

wasm://wasm/lua_mlua_web.wasm-00199512:1


Error: wasi.start() has not been called
    at lua_mlua_web.wasm.__wasi_clock_time_get (wasm://wasm/lua_mlua_web.wasm-00199512:wasm-function[368]:0x1ac64)
    at lua_mlua_web.wasm.time (wasm://wasm/lua_mlua_web.wasm-00199512:wasm-function[436]:0x1c5fe)
    at lua_mlua_web.wasm.lua_newstate (wasm://wasm/lua_mlua_web.wasm-00199512:wasm-function[449]:0x1cb43)
    at lua_mlua_web.wasm._RNvMs6_NtCs8mqczqN7vnY_4mlua5stateNtB5_3Lua3new (wasm://wasm/lua_mlua_web.wasm-00199512:wasm-function[94]:0x587c)
    at lua_mlua_web.wasm.rodar (wasm://wasm/lua_mlua_web.wasm-00199512:wasm-function[93]:0x4d5b)
    at lua_mlua_web.wasm.rodar.command_export (wasm://wasm/lua_mlua_web.wasm-00199512:wasm-function[1331]:0x4da4f)
    at file:///home/runner/work/nn-engine/nn-engine/estudo-web/lua-mlua-web.mjs:17:28 {
  code: 'ERR_WASI_NOT_STARTED'
}

Node.js v22.23.3
(node:12299) ExperimentalWarning: WASI is an experimental feature and might change at any time
(Use `node --trace-warnings ...` to show where the warning was created)
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

wasm://wasm/lua_mlua_web.wasm-00199512:1


Error: wasi.start() has not been called
    at lua_mlua_web.wasm.__wasi_clock_time_get (wasm://wasm/lua_mlua_web.wasm-00199512:wasm-function[368]:0x1ac64)
    at lua_mlua_web.wasm.time (wasm://wasm/lua_mlua_web.wasm-00199512:wasm-function[436]:0x1c5fe)
    at lua_mlua_web.wasm.lua_newstate (wasm://wasm/lua_mlua_web.wasm-00199512:wasm-function[449]:0x1cb43)
    at lua_mlua_web.wasm._RNvMs6_NtCs8mqczqN7vnY_4mlua5stateNtB5_3Lua3new (wasm://wasm/lua_mlua_web.wasm-00199512:wasm-function[94]:0x587c)
    at lua_mlua_web.wasm.rodar (wasm://wasm/lua_mlua_web.wasm-00199512:wasm-function[93]:0x4d5b)
    at lua_mlua_web.wasm.rodar.command_export (wasm://wasm/lua_mlua_web.wasm-00199512:wasm-function[1331]:0x4da4f)
    at file:///home/runner/work/nn-engine/nn-engine/estudo-web/lua-mlua-web.mjs:17:28 {
  code: 'ERR_WASI_NOT_STARTED'
}

Node.js v22.23.3
