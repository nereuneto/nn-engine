# E0 — as crates da engine em `wasm32-unknown-emscripten`

| crate | resultado | tempo |
|---|---|---|
| nn-core | compila | 0 s |
| nn-input | **não compila** | 0 s |
| nn-render | compila | 4 s |
| nn-audio | compila | 0 s |
| nn-physics | compila | 1 s |
| nn-physics3d | compila | 1 s |
| nn-nav | compila | 0 s |
| nn-tilemap | compila | 4 s |
| nn-script | **não compila** | 4 s |
| nn-app | **não compila** | 12 s |
| nn-projeto | **não compila** | 1 s |
| nn-run | **não compila** | 19 s |

## nn-input

```
error: The platform you're compiling for is not supported by winit
  --> /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/mod.rs:78:1
   |
78 | compile_error!("The platform you're compiling for is not supported by winit");
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error: could not compile `winit` (lib) due to 1 previous error
```

## nn-script

```
error: The platform you're compiling for is not supported by winit
  --> /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/mod.rs:78:1
   |
78 | compile_error!("The platform you're compiling for is not supported by winit");
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error: could not compile `winit` (lib) due to 1 previous error
warning: build failed, waiting for other jobs to finish...
```

## nn-app

```
error: The platform you're compiling for is not supported by winit
  --> /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/mod.rs:78:1
   |
78 | compile_error!("The platform you're compiling for is not supported by winit");
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error: could not compile `winit` (lib) due to 1 previous error
warning: build failed, waiting for other jobs to finish...
```

## nn-projeto

```
error: The platform you're compiling for is not supported by winit
  --> /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/mod.rs:78:1
   |
78 | compile_error!("The platform you're compiling for is not supported by winit");
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error: could not compile `winit` (lib) due to 1 previous error
warning: build failed, waiting for other jobs to finish...
```

## nn-run

```
error: The platform you're compiling for is not supported by winit
  --> /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/mod.rs:78:1
   |
78 | compile_error!("The platform you're compiling for is not supported by winit");
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

   Compiling wgpu-hal v30.0.1
   Compiling gilrs v0.11.2
error: could not compile `winit` (lib) due to 1 previous error
warning: build failed, waiting for other jobs to finish...
```

