# E3 — mlua (Lua 5.4 em C) no wasm32-unknown-unknown

não compila:
```
error: failed to run custom build command for `mlua-sys v0.13.0`

Caused by:
  process didn't exit successfully: `/home/runner/work/nn-engine/nn-engine/estudo-web/target/release/build/mlua-sys-a3fa1e5fe20d2e39/build-script-main` (exit status: 101)
  --- stdout
  cargo:rerun-if-changed=build

  --- stderr

  thread 'main' (18104) panicked at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/lua-src-551.0.2/src/lib.rs:100:25:
  don't know how to build Lua for wasm32-unknown-unknown
  note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
warning: build failed, waiting for other jobs to finish...
```
