# E0 — as crates da engine em `wasm32-unknown-unknown`

| crate | resultado | tempo |
|---|---|---|
| nn-core | compila | 1 s |
| nn-input | compila | 1 s |
| nn-render | compila | 3 s |
| nn-audio | compila | 0 s |
| nn-physics | compila | 2 s |
| nn-physics3d | compila | 1 s |
| nn-nav | compila | 0 s |
| nn-tilemap | compila | 3 s |
| nn-script | **não compila** | 4 s |
| nn-app | compila | 4 s |
| nn-projeto | **não compila** | 1 s |
| nn-run | **não compila** | 10 s |

## nn-script

```
error: failed to run custom build command for `mlua-sys v0.13.0`

Caused by:
  process didn't exit successfully: `/home/runner/work/nn-engine/nn-engine/target/release/build/mlua-sys-d92a95accc53df11/build-script-main` (exit status: 101)
  --- stdout
  cargo:rerun-if-changed=build

  --- stderr

  thread 'main' (9167) panicked at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/lua-src-551.0.2/src/lib.rs:100:25:
  don't know how to build Lua for wasm32-unknown-unknown
  note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
warning: build failed, waiting for other jobs to finish...
```

## nn-projeto

```
error: failed to run custom build command for `mlua-sys v0.13.0`

Caused by:
  process didn't exit successfully: `/home/runner/work/nn-engine/nn-engine/target/release/build/mlua-sys-d92a95accc53df11/build-script-main` (exit status: 101)
  --- stdout
  cargo:rerun-if-changed=build

  --- stderr

  thread 'main' (9357) panicked at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/lua-src-551.0.2/src/lib.rs:100:25:
  don't know how to build Lua for wasm32-unknown-unknown
  note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
warning: build failed, waiting for other jobs to finish...
```

## nn-run

```
error: failed to run custom build command for `mlua-sys v0.13.0`

Caused by:
  process didn't exit successfully: `/home/runner/work/nn-engine/nn-engine/target/release/build/mlua-sys-d92a95accc53df11/build-script-main` (exit status: 101)
  --- stdout
  cargo:rerun-if-changed=build

  --- stderr

  thread 'main' (9570) panicked at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/lua-src-551.0.2/src/lib.rs:100:25:
  don't know how to build Lua for wasm32-unknown-unknown
  note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
warning: build failed, waiting for other jobs to finish...
```

