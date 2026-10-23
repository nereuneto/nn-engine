//! Os arquivos de um projeto da nn-engine, sem interface: ler, mudar e
//! gravar cenas (`.cena`), prefabs (`.prefab`), regras de eventos, o
//! `config.lua`, diálogos (`.dialogo`), imagens de pixel art e as animações
//! delas (`.anim`), mapas de tiles do Tiled (`.tmj`, `.tsj`), efeitos de
//! partículas prontos, efeitos sonoros sintetizados (`.sfx` → `.wav`), modelos
//! 3D do modelador (`.modelo`, com as peças e os modelos prontos) e conferir
//! scripts Lua.
//!
//! O NN Studio e o servidor MCP (`nn mcp`) usam este mesmo código, então o
//! que um grava o outro abre igual.

pub mod anim;
pub mod behaviors;
pub mod config_lua;
pub mod dialogue;
pub mod events;
pub mod lua_syntax;
pub mod map;
pub mod model;
pub mod model_kits;
pub mod model_rig;
pub mod model_view;
pub mod particles;
pub mod pixel;
pub mod scene_data;
pub mod sfx;
pub mod tileset;

/// Um Lua para ler arquivos do projeto (cenas, `config.lua`, comportamentos)
/// sem rodar o jogo: o `print` vai para o log e não há `io` nem `os.exit`.
/// Assim um arquivo escrito à mão não escreve na saída padrão (que o
/// servidor MCP usa para o protocolo) nem fecha o programa.
pub fn quiet_lua() -> mlua::Lua {
    let lua = mlua::Lua::new();
    let globals = lua.globals();
    if let Ok(print) = lua.create_function(|_, args: mlua::Variadic<mlua::Value>| {
        let text: Vec<String> = args
            .iter()
            .map(|v| v.to_string().unwrap_or_else(|_| format!("{v:?}")))
            .collect();
        log::info!("{}", text.join("\t"));
        Ok(())
    }) {
        let _ = globals.set("print", print);
    }
    let _ = globals.set("io", mlua::Value::Nil);
    if let Ok(os) = globals.get::<mlua::Table>("os") {
        for name in ["exit", "execute", "remove", "rename", "tmpname"] {
            let _ = os.set(name, mlua::Value::Nil);
        }
    }
    lua
}
