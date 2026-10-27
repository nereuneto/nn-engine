# E3 — mlua (Lua 5.4 em C) no wasm32-wasip1 (wasi-sdk), rodando no wasmtime

não compila:
```
error: linking with `rust-lld` failed: exit status: 1
  |
  = note:  "rust-lld" "-flavor" "wasm" "--export" "__main_void" "-z" "stack-size=1048576" "--stack-first" "--no-demangle" "<1 object files omitted>" "/home/runner/work/nn-engine/nn-engine/estudo-web/target/wasm32-wasip1/release/deps/rustcqqeSce/libmlua_sys-c1353c324b3474c1.rlib" "-l" "wasi-emulated-signal" "-l" "setjmp" "-l" "c" "<sysroot>/lib/rustlib/wasm32-wasip1/lib/libcompiler_builtins-*.rlib" "-L" "/home/runner/wasi-sdk-27.0-x86_64-linux/share/wasi-sysroot/lib/wasm32-wasip1" "-L" "/home/runner/work/nn-engine/nn-engine/estudo-web/target/wasm32-wasip1/release/build/mlua-sys-1bfce7dfca678b32/out/lib" "-o" "/home/runner/work/nn-engine/nn-engine/estudo-web/target/wasm32-wasip1/release/deps/lua_mlua-07274556c912aee9.wasm" "--gc-sections" "-O2" "--strip-debug"
  = note: some arguments are omitted. use `--verbose` to show all linker arguments
  = note: rust-lld: error: entry symbol not defined (pass --no-entry to suppress): _start
          

error: could not compile `lua-mlua` (bin "lua-mlua") due to 1 previous error
```
