# E3 — piccolo (Rust puro) no wasm32-unknown-unknown, rodando no Node

não compila:
```
error: the wasm*-unknown-unknown targets are not supported by default, you may need to enable the "js" feature. For more information see: https://docs.rs/getrandom/#webassembly-support
   --> /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/getrandom-0.2.17/src/lib.rs:346:9
    |
346 | /         compile_error!("the wasm*-unknown-unknown targets are not supported by \
347 | |                         default, you may need to enable the \"js\" feature. \
348 | |                         For more information see: \
349 | |                         https://docs.rs/getrandom/#webassembly-support");
    | |________________________________________________________________________^

error: The wasm32-unknown-unknown targets are not supported by default; you may need to enable the "wasm_js" configuration flag. Note that enabling the `wasm_js` feature flag alone is insufficient. For more information see: https://docs.rs/getrandom/0.3.4/#webassembly-support
   --> /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/getrandom-0.3.4/src/backends.rs:194:17
    |
194 | /                 compile_error!(concat!(
195 | |                     "The wasm32-unknown-unknown targets are not supported by default; \
196 | |                     you may need to enable the \"wasm_js\" configuration flag. Note \
197 | |                     that enabling the `wasm_js` feature flag alone is insufficient. \
198 | |                     For more information see: \
199 | |                     https://docs.rs/getrandom/", env!("CARGO_PKG_VERSION"), "/#webassembly-support"
200 | |                 ));
    | |__________________^

error: could not compile `getrandom` (lib) due to 1 previous error
warning: build failed, waiting for other jobs to finish...
error: could not compile `getrandom` (lib) due to 1 previous error
```
