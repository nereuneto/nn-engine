//! Lua em Rust puro (piccolo): roda os testes de compatibilidade e mede a
//! carga de `bench.lua`. Compila para wasm32-unknown-unknown sem C.

use piccolo::{Closure, Executor, Lua};
use web_time::Instant;

fn rodar_script(nome: &str, codigo: &str) -> Result<String, String> {
    let mut lua = Lua::full();
    let ex = lua
        .try_enter(|ctx| {
            let closure = Closure::load(ctx, Some(nome), codigo.as_bytes())?;
            Ok(ctx.stash(Executor::start(ctx, closure.into(), ())))
        })
        .map_err(|e| format!("carregar {nome}: {e}"))?;
    lua.finish(&ex);
    lua.try_enter(|ctx| {
        let v = ctx.fetch(&ex).take_result::<piccolo::Value>(ctx)??;
        Ok(v.to_string())
    })
    .map_err(|e| format!("rodar {nome}: {e}"))
}

/// O relatório: os testes e o tempo da carga.
pub fn rodar() -> String {
    let mut out = String::from("## piccolo 0.3\n\n```\n");
    match rodar_script("teste.lua", include_str!("../../teste.lua")) {
        Ok(s) => out.push_str(&s),
        Err(e) => out.push_str(&format!("ERRO {e}")),
    }
    out.push_str("\n```\n\n");
    let t = Instant::now();
    match rodar_script("bench.lua", include_str!("../../bench.lua")) {
        Ok(v) => out.push_str(&format!(
            "bench: {:.1} ms (resultado {v})\n",
            t.elapsed().as_secs_f64() * 1000.0
        )),
        Err(e) => out.push_str(&format!("bench: ERRO {e}\n")),
    }
    out
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn rodar_js() -> String {
    rodar()
}
