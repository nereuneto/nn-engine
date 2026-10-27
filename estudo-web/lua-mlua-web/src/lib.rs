//! Roda `teste.lua` e `bench.lua` e devolve o relatório. Sem wasm-bindgen: a
//! página chama `rodar()` direto e lê o texto da memória.

/// Devolve `(tamanho << 32) | ponteiro` do texto (UTF-8) na memória do wasm.
#[unsafe(no_mangle)]
pub extern "C" fn rodar() -> u64 {
    rodar_com(relogio)
}

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    /// O relógio da página (performance.now()).
    #[link_name = "agora_ms"]
    fn agora_js() -> f64;
}

fn relogio() -> f64 {
    unsafe { agora_js() }
}

fn rodar_com(agora: fn() -> f64) -> u64 {
    let mut out = String::from("## mlua (Lua 5.4 em C) no wasm32-unknown-unknown\n\n```\n");
    let lua = mlua::Lua::new();
    match lua.load(include_str!("../../teste.lua")).set_name("teste.lua").eval::<String>() {
        Ok(s) => out.push_str(&s),
        Err(e) => out.push_str(&format!("ERRO {e}\n")),
    }
    out.push_str("```\n\n");
    let lua = mlua::Lua::new();
    let t = agora();
    match lua.load(include_str!("../../bench.lua")).eval::<mlua::Value>() {
        Ok(v) => out.push_str(&format!(
            "bench: {:.1} ms (resultado {})\n",
            agora() - t,
            v.to_string().unwrap_or_default()
        )),
        Err(e) => out.push_str(&format!("bench: ERRO {e}\n")),
    }
    let s = out.leak();
    ((s.len() as u64) << 32) | (s.as_ptr() as u64)
}
