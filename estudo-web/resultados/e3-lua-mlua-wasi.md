# E3 — mlua (Lua 5.4 em C) no wasm32-wasip1 (wasi-sdk), rodando no wasmtime

não compila:
```
error: linking with `rust-lld` failed: exit status: 1
  |
  = note:  "rust-lld" "-flavor" "wasm" "--export" "__main_void" "-z" "stack-size=1048576" "--stack-first" "--no-demangle" "<sysroot>/lib/rustlib/wasm32-wasip1/lib/self-contained/crt1-command.o" "<1 object files omitted>" "/home/runner/work/nn-engine/nn-engine/estudo-web/target/wasm32-wasip1/release/deps/rustcLZlyGv/libmlua_sys-d7e9d66b2a372337.rlib" "-l" "wasi-emulated-signal" "-l" "setjmp" "-l" "c" "<sysroot>/lib/rustlib/wasm32-wasip1/lib/libcompiler_builtins-*.rlib" "-L" "/home/runner/wasi-sdk-27.0-x86_64-linux/share/wasi-sysroot/lib/wasm32-wasip1" "-L" "/home/runner/work/nn-engine/nn-engine/estudo-web/target/wasm32-wasip1/release/build/mlua-sys-532419c5f39e777d/out/lib" "-L" "<sysroot>/lib/rustlib/wasm32-wasip1/lib/self-contained" "-o" "/home/runner/work/nn-engine/nn-engine/estudo-web/target/wasm32-wasip1/release/deps/lua_mlua-6eb325cefd49fb7e.wasm" "--gc-sections" "-O2" "--strip-debug"
  = note: some arguments are omitted. use `--verbose` to show all linker arguments
  = note: rust-lld: error: /home/runner/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/wasm32-wasip1/lib/self-contained/crt1-command.o: undefined symbol: __wasi_init_tp
          

error: could not compile `lua-mlua` (bin "lua-mlua") due to 1 previous error
```
