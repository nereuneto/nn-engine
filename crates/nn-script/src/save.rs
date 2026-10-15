//! `nn.save`: dados que continuam existindo depois de fechar o jogo.

use mlua::{Lua, LuaSerdeExt, Result, Table, Value};

use crate::api::with_ctx;
use crate::convert::err;

fn to_json(lua: &Lua, value: Value) -> Result<serde_json::Value> {
    let type_name = value.type_name();
    lua.from_value::<serde_json::Value>(value).map_err(|e| {
        err(format!(
            "não dá para salvar um valor do tipo {type_name}: use números, textos, \
             booleanos e tabelas com eles ({e})"
        ))
    })
}

pub(crate) fn table(lua: &Lua) -> Result<Table> {
    let save = lua.create_table()?;
    save.set(
        "get",
        lua.create_function(|lua, (key, default): (String, Value)| {
            let value = with_ctx(lua, |c| Ok(c.save.get(&key).cloned()))?;
            match value {
                Some(v) => lua.to_value(&v),
                None => Ok(default),
            }
        })?,
    )?;
    save.set(
        "set",
        lua.create_function(|lua, (key, value): (String, Value)| {
            let json = to_json(lua, value)?;
            with_ctx(lua, |c| {
                c.save.set(key, json);
                Ok(())
            })
        })?,
    )?;
    save.set(
        "remove",
        lua.create_function(|lua, key: String| {
            with_ctx(lua, |c| {
                c.save.remove(&key);
                Ok(())
            })
        })?,
    )?;
    save.set(
        "clear",
        lua.create_function(|lua, ()| {
            with_ctx(lua, |c| {
                c.save.clear();
                Ok(())
            })
        })?,
    )?;
    save.set(
        "keys",
        lua.create_function(|lua, ()| {
            let mut keys: Vec<String> =
                with_ctx(lua, |c| Ok(c.save.keys().map(str::to_owned).collect()))?;
            keys.sort();
            lua.create_sequence_from(keys)
        })?,
    )?;
    save.set(
        "flush",
        lua.create_function(|lua, ()| {
            with_ctx(lua, |c| {
                c.save
                    .flush()
                    .map_err(|e| err(format!("não foi possível salvar: {e}")))
            })
        })?,
    )?;
    save.set(
        "path",
        lua.create_function(|lua, ()| {
            with_ctx(lua, |c| {
                Ok(c.save.path().map(|p| p.to_string_lossy().into_owned()))
            })
        })?,
    )?;
    Ok(save)
}
