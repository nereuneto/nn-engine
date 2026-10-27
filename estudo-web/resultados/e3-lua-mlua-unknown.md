# E3 — mlua (Lua 5.4 em C) no wasm32-unknown-unknown

não compila:
```
error: failed to run custom build command for `mlua-sys v0.13.0`

Caused by:
  process didn't exit successfully: `/home/runner/work/nn-engine/nn-engine/estudo-web/target/release/build/mlua-sys-c9d5f716f63a4b6b/build-script-main` (exit status: 101)
  --- stdout
  cargo:rerun-if-changed=build
  cargo:warning=clang (LLVM option parsing): Unknown command line argument '-wasm-use-legacy-eh=false'.  Try: 'clang (LLVM option parsing) --help'
  cargo:warning=clang (LLVM option parsing): Did you mean '--wasm-enable-eh=false'?

  --- stderr

  thread 'main' (12181) panicked at lua-src-web/src/lib.rs:100:25:
  ToolExecError: command did not execute successfully (status code exit status: 1): "clang" "-Oz" "-ffunction-sections" "-fdata-sections" "--target=wasm32-unknown-unknown" "-I" "/home/runner/work/nn-engine/nn-engine/estudo-web/lua-src-web/lua-5.4.9" "-w" "-mllvm" "-wasm-enable-eh" "-mllvm" "-wasm-use-legacy-eh=false" "-mllvm" "-wasm-enable-sjlj" "-fno-common" "-DLUA_USE_POSIX" "-D_WASI_EMULATED_SIGNAL" "-DLUA_COMPAT_5_3" "-o" "/home/runner/work/nn-engine/nn-engine/estudo-web/target/wasm32-unknown-unknown/release/build/mlua-sys-c1461f46a26062ad/out/lib/fb6722624dfb7324-lbaselib.o" "-c" "/home/runner/work/nn-engine/nn-engine/estudo-web/lua-src-web/lua-5.4.9/lbaselib.c"
  note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```
