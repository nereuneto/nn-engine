//! O Lua de hoje (mlua com o Lua 5.4 em C): no desktop é a base para
//! comparar; no wasm32-unknown-emscripten mostra se o mesmo Lua roda na web.

use std::time::Instant;

fn main() {
    let lua = mlua::Lua::new();
    println!("## mlua (Lua 5.4 em C)\n\n```");
    match lua
        .load(include_str!("../../teste.lua"))
        .set_name("teste.lua")
        .eval::<String>()
    {
        Ok(s) => println!("{s}"),
        Err(e) => println!("ERRO {e}"),
    }
    println!("```\n");
    let lua = mlua::Lua::new();
    let t = Instant::now();
    match lua
        .load(include_str!("../../bench.lua"))
        .eval::<mlua::Value>()
    {
        Ok(v) => println!(
            "bench: {:.1} ms (resultado {})",
            t.elapsed().as_secs_f64() * 1000.0,
            v.to_string().unwrap_or_default()
        ),
        Err(e) => println!("bench: ERRO {e}"),
    }
}
