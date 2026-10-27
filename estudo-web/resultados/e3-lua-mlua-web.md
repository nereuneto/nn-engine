# E3 — mlua (Lua 5.4 em C) no wasm32-unknown-unknown com a libc do wasi-sdk, rodando no Node

não compila:
```
error: linking with `rust-lld` failed: exit status: 1
  |
  = note:  "rust-lld" "-flavor" "wasm" "--export" "rodar" "-z" "stack-size=1048576" "--stack-first" "--no-demangle" "--no-entry" "<1 object files omitted>" "-l" "c" "-l" "clang_rt.builtins" "/home/runner/work/nn-engine/nn-engine/estudo-web/target/wasm32-unknown-unknown/release/deps/rustcu4o73j/libmlua_sys-6812b844a4425ee5.rlib" "-l" "wasi-emulated-signal" "-l" "setjmp" "<sysroot>/lib/rustlib/wasm32-unknown-unknown/lib/libcompiler_builtins-*.rlib" "-l" "c" "-l" "clang_rt.builtins" "-l" "c" "-l" "clang_rt.builtins" "-l" "c" "-l" "clang_rt.builtins" "-l" "c" "-l" "clang_rt.builtins" "-l" "c" "-l" "clang_rt.builtins" "-l" "c" "-l" "clang_rt.builtins" "-l" "c" "-l" "clang_rt.builtins" "-l" "c" "-l" "clang_rt.builtins" "-l" "c" "-l" "clang_rt.builtins" "-l" "c" "-l" "clang_rt.builtins" "-l" "c" "-l" "clang_rt.builtins" "-l" "c" "-l" "clang_rt.builtins" "-l" "c" "-l" "clang_rt.builtins" "-L" "/home/runner/wasi-sdk-27.0-x86_64-linux/share/wasi-sysroot/lib/wasm32-wasip1" "-L" "/home/runner/wasi-sdk-27.0-x86_64-linux/lib/clang/20/lib/wasm32-unknown-wasip1" "-L" "/home/runner/work/nn-engine/nn-engine/estudo-web/target/wasm32-unknown-unknown/release/build/mlua-sys-ac55b995f77c69fd/out/lib" "-L" "<sysroot>/lib/rustlib/wasm32-unknown-unknown/lib/self-contained" "-o" "/home/runner/work/nn-engine/nn-engine/estudo-web/target/wasm32-unknown-unknown/release/deps/lua_mlua_web.wasm" "--gc-sections" "--no-entry" "-O2" "--strip-debug"
  = note: some arguments are omitted. use `--verbose` to show all linker arguments
  = note: rust-lld: error: /home/runner/work/nn-engine/nn-engine/estudo-web/target/wasm32-unknown-unknown/release/deps/lua_mlua_web.lua_mlua_web.6ac6dc21f5a09854-cgu.0.rcgu.o: undefined symbol: agora_ms
          rust-lld: error: /home/runner/work/nn-engine/nn-engine/estudo-web/target/wasm32-unknown-unknown/release/deps/lua_mlua_web.lua_mlua_web.6ac6dc21f5a09854-cgu.0.rcgu.o: undefined symbol: agora_ms
          

error: could not compile `lua-mlua-web` (lib) due to 1 previous error
```
