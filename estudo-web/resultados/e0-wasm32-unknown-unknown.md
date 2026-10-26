# E0 — as crates da engine em `wasm32-unknown-unknown`

| crate | resultado | tempo |
|---|---|---|
| nn-core | compila | 4 s |
| nn-input | compila | 27 s |
| nn-render | compila | 43 s |
| nn-audio | compila | 11 s |
| nn-physics | compila | 6 s |
| nn-physics3d | compila | 37 s |
| nn-nav | compila | 0 s |
| nn-tilemap | compila | 10 s |
| nn-script | **não compila** | 56 s |
| nn-app | **não compila** | 34 s |
| nn-projeto | **não compila** | 1 s |
| nn-run | **não compila** | 1 s |

## nn-script

```
error: failed to run custom build command for `mlua-sys v0.13.0`

Caused by:
  process didn't exit successfully: `/home/runner/work/nn-engine/nn-engine/target/release/build/mlua-sys-d92a95accc53df11/build-script-main` (exit status: 101)
  --- stdout
  cargo:rerun-if-changed=build

  --- stderr

  thread 'main' (13344) panicked at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/lua-src-551.0.2/src/lib.rs:100:25:
  don't know how to build Lua for wasm32-unknown-unknown
  note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
warning: build failed, waiting for other jobs to finish...
```

## nn-app

```
error[E0308]: mismatched types
   --> crates/nn-app/src/window.rs:383:17
    |
382 |             event_loop.set_control_flow(ControlFlow::WaitUntil(
    |                                         ---------------------- arguments to this enum variant are incorrect
383 |                 Instant::now() + std::time::Duration::from_millis(100),
    |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ expected `web_time::time::instant::Instant`, found `Instant`
    |
    = note: `Instant` and `web_time::time::instant::Instant` have similar names, but are actually distinct types
note: `Instant` is defined in crate `std`
   --> /rustc/58cf8f9c35b355199727627d07ae34a4e2846d67/library/std/src/time.rs:156:0
note: `web_time::time::instant::Instant` is defined in crate `web_time`
   --> /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/web-time-1.1.0/src/time/instant.rs:15:1
    |
 15 | pub struct Instant(Duration);
--
error[E0308]: mismatched types
   --> crates/nn-app/src/window.rs:389:64
    |
389 |             event_loop.set_control_flow(ControlFlow::WaitUntil(state.next_frame));
    |                                         ---------------------- ^^^^^^^^^^^^^^^^ expected `web_time::time::instant::Instant`, found `Instant`
    |                                         |
    |                                         arguments to this enum variant are incorrect
    |
    = note: `Instant` and `web_time::time::instant::Instant` have similar names, but are actually distinct types
note: `Instant` is defined in crate `std`
   --> /rustc/58cf8f9c35b355199727627d07ae34a4e2846d67/library/std/src/time.rs:156:0
note: `web_time::time::instant::Instant` is defined in crate `web_time`
   --> /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/web-time-1.1.0/src/time/instant.rs:15:1
    |
 15 | pub struct Instant(Duration);
--
error: could not compile `nn-app` (lib) due to 2 previous errors
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

  thread 'main' (14126) panicked at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/lua-src-551.0.2/src/lib.rs:100:25:
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

  thread 'main' (14182) panicked at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/lua-src-551.0.2/src/lib.rs:100:25:
  don't know how to build Lua for wasm32-unknown-unknown
  note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
warning: build failed, waiting for other jobs to finish...
```

