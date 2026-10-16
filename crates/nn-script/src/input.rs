//! `nn.input`: teclado, mouse, controles e ações.

use mlua::{Lua, Result, Table, Value};
use nn_input::{Device, Input, KeyCode};

use crate::api::with_ctx;
use crate::convert::{bindings, err, keys, mouse_button, pad_axis, pad_button};

/// Lugar do controle (1, 2, ...) vindo do Lua.
fn slot(pad: Option<i64>) -> Result<Option<usize>> {
    match pad {
        None => Ok(None),
        Some(n) if n >= 1 => Ok(Some(n as usize - 1)),
        Some(n) => Err(err(format!(
            "controle {n} não existe (os controles contam a partir de 1)"
        ))),
    }
}

fn check_action(input: &Input, name: &str) -> Result<()> {
    if input.has_action(name) {
        Ok(())
    } else {
        Err(err(format!(
            "ação desconhecida: \"{name}\" (crie com nn.input.bind(\"{name}\", {{...}}))"
        )))
    }
}

pub(crate) fn table(lua: &Lua) -> Result<Table> {
    let input = lua.create_table()?;

    // --- Teclado ---
    input.set(
        "down",
        lua.create_function(|lua, name: String| {
            let ks = keys(&name)?;
            with_ctx(lua, |c| Ok(ks.iter().any(|k| c.input.key_down(*k))))
        })?,
    )?;
    input.set(
        "pressed",
        lua.create_function(|lua, name: String| {
            let ks = keys(&name)?;
            with_ctx(lua, |c| Ok(ks.iter().any(|k| c.input.key_pressed(*k))))
        })?,
    )?;
    input.set(
        "released",
        lua.create_function(|lua, name: String| {
            let ks = keys(&name)?;
            with_ctx(lua, |c| Ok(ks.iter().any(|k| c.input.key_released(*k))))
        })?,
    )?;
    input.set(
        "axis",
        lua.create_function(|lua, (neg, pos): (String, String)| {
            let (n, p) = (keys(&neg)?, keys(&pos)?);
            with_ctx(lua, |c| {
                let down = |ks: &[KeyCode]| ks.iter().any(|k| c.input.key_down(*k));
                Ok(f32::from(i8::from(down(p)) - i8::from(down(n))))
            })
        })?,
    )?;
    input.set(
        "wasd",
        lua.create_function(|lua, ()| {
            with_ctx(lua, |c| {
                let v = c.input.wasd();
                Ok((v.x, v.y))
            })
        })?,
    )?;
    input.set(
        "arrows",
        lua.create_function(|lua, ()| {
            with_ctx(lua, |c| {
                let v = c.input.arrows();
                Ok((v.x, v.y))
            })
        })?,
    )?;
    input.set(
        "move",
        lua.create_function(|lua, ()| {
            with_ctx(lua, |c| {
                let v = c.input.move_vector();
                Ok((v.x, v.y))
            })
        })?,
    )?;
    input.set(
        "any_pressed",
        lua.create_function(|lua, ()| {
            with_ctx(lua, |c| {
                let i = &c.input;
                Ok(i.any_key_pressed()
                    || i.any_pad_pressed()
                    || [
                        nn_input::MouseButton::Left,
                        nn_input::MouseButton::Right,
                        nn_input::MouseButton::Middle,
                    ]
                    .iter()
                    .any(|b| i.mouse_pressed(*b)))
            })
        })?,
    )?;

    // --- Mouse ---
    input.set(
        "mouse_down",
        lua.create_function(|lua, name: Option<String>| {
            let b = mouse_button(name.as_deref().unwrap_or("left"))?;
            with_ctx(lua, |c| Ok(c.input.mouse_down(b)))
        })?,
    )?;
    input.set(
        "mouse_pressed",
        lua.create_function(|lua, name: Option<String>| {
            let b = mouse_button(name.as_deref().unwrap_or("left"))?;
            with_ctx(lua, |c| Ok(c.input.mouse_pressed(b)))
        })?,
    )?;
    input.set(
        "mouse_released",
        lua.create_function(|lua, name: Option<String>| {
            let b = mouse_button(name.as_deref().unwrap_or("left"))?;
            with_ctx(lua, |c| Ok(c.input.mouse_released(b)))
        })?,
    )?;
    input.set(
        "mouse",
        lua.create_function(|lua, ()| {
            with_ctx(lua, |c| {
                Ok(match c.input.cursor_position() {
                    Some(p) => (Some(p.x), Some(p.y)),
                    None => (None, None),
                })
            })
        })?,
    )?;
    input.set(
        "mouse_world",
        lua.create_function(|lua, ()| {
            with_ctx(lua, |c| {
                Ok(match c.cursor_world() {
                    Some(p) => (Some(p.x), Some(p.y)),
                    None => (None, None),
                })
            })
        })?,
    )?;
    input.set(
        "scroll",
        lua.create_function(|lua, ()| {
            with_ctx(lua, |c| {
                let s = c.input.scroll();
                Ok((s.x, s.y))
            })
        })?,
    )?;

    // --- Controles ---
    input.set(
        "pad_down",
        lua.create_function(|lua, (name, pad): (String, Option<i64>)| {
            let b = pad_button(&name)?;
            let slot = slot(pad)?;
            with_ctx(lua, |c| {
                Ok(match slot {
                    Some(s) => c.input.pad(s).is_some_and(|p| p.down(b)),
                    None => c.input.pad_down(b),
                })
            })
        })?,
    )?;
    input.set(
        "pad_pressed",
        lua.create_function(|lua, (name, pad): (String, Option<i64>)| {
            let b = pad_button(&name)?;
            let slot = slot(pad)?;
            with_ctx(lua, |c| {
                Ok(match slot {
                    Some(s) => c.input.pad(s).is_some_and(|p| p.pressed(b)),
                    None => c.input.pad_pressed(b),
                })
            })
        })?,
    )?;
    input.set(
        "pad_released",
        lua.create_function(|lua, (name, pad): (String, Option<i64>)| {
            let b = pad_button(&name)?;
            let slot = slot(pad)?;
            with_ctx(lua, |c| {
                Ok(match slot {
                    Some(s) => c.input.pad(s).is_some_and(|p| p.released(b)),
                    None => c.input.pad_released(b),
                })
            })
        })?,
    )?;
    input.set(
        "pad_axis",
        lua.create_function(|lua, (name, pad): (String, Option<i64>)| {
            let a = pad_axis(&name)?;
            let slot = slot(pad)?;
            with_ctx(lua, |c| {
                Ok(match slot {
                    Some(s) => c.input.pad(s).map_or(0.0, |p| p.axis(a)),
                    None => c.input.pad_axis(a),
                })
            })
        })?,
    )?;
    input.set(
        "stick",
        lua.create_function(|lua, (side, pad): (Option<String>, Option<i64>)| {
            let right = match side.as_deref().unwrap_or("left") {
                "left" => false,
                "right" => true,
                other => {
                    return Err(err(format!(
                        "analógico inválido: \"{other}\" (use left ou right)"
                    )));
                }
            };
            let slot = slot(pad)?;
            with_ctx(lua, |c| {
                let v = match (slot, right) {
                    (Some(s), false) => c.input.pad(s).map(|p| p.left_stick()),
                    (Some(s), true) => c.input.pad(s).map(|p| p.right_stick()),
                    (None, false) => Some(c.input.left_stick()),
                    (None, true) => Some(c.input.right_stick()),
                }
                .unwrap_or_default();
                Ok((v.x, v.y))
            })
        })?,
    )?;
    input.set(
        "pads",
        lua.create_function(|lua, ()| {
            let list: Vec<(usize, String)> = with_ctx(lua, |c| {
                Ok(c.input
                    .pads()
                    .map(|(i, p)| (i + 1, p.name().to_owned()))
                    .collect())
            })?;
            let t = lua.create_table()?;
            for (slot, name) in list {
                let pad = lua.create_table()?;
                pad.set("slot", slot)?;
                pad.set("name", name)?;
                t.push(pad)?;
            }
            Ok(t)
        })?,
    )?;
    input.set(
        "pad_count",
        lua.create_function(|lua, ()| with_ctx(lua, |c| Ok(c.input.pad_count())))?,
    )?;
    input.set(
        "set_deadzone",
        lua.create_function(|lua, v: f32| {
            with_ctx(lua, |c| {
                c.input.set_deadzone(v);
                Ok(())
            })
        })?,
    )?;
    input.set(
        "text",
        lua.create_function(|lua, ()| with_ctx(lua, |c| Ok(c.input.typed().to_string())))?,
    )?;
    input.set(
        "last_device",
        lua.create_function(|lua, ()| {
            with_ctx(lua, |c| {
                Ok(match c.input.last_device() {
                    Device::Keyboard => "keyboard",
                    Device::Mouse => "mouse",
                    Device::Gamepad => "gamepad",
                })
            })
        })?,
    )?;

    // --- Ações ---
    input.set(
        "bind",
        lua.create_function(|lua, (name, specs): (String, Value)| {
            let specs: Vec<String> = match specs {
                Value::String(s) => vec![s.to_str()?.to_string()],
                Value::Table(t) => t
                    .sequence_values::<String>()
                    .collect::<Result<_>>()
                    .map_err(|_| {
                        err("as entradas da ação devem ser textos, como \"space\" ou \"pad:a\"")
                    })?,
                other => {
                    return Err(err(format!(
                        "nn.input.bind espera uma lista de entradas, recebeu {}",
                        other.type_name()
                    )));
                }
            };
            let mut all = Vec::new();
            for spec in &specs {
                all.extend(bindings(spec)?);
            }
            with_ctx(lua, |c| {
                c.input.bind(name, all);
                Ok(())
            })
        })?,
    )?;
    input.set(
        "action",
        lua.create_function(|lua, name: String| {
            with_ctx(lua, |c| {
                check_action(&c.input, &name)?;
                Ok(c.input.action_down(&name))
            })
        })?,
    )?;
    input.set(
        "action_pressed",
        lua.create_function(|lua, name: String| {
            with_ctx(lua, |c| {
                check_action(&c.input, &name)?;
                Ok(c.input.action_pressed(&name))
            })
        })?,
    )?;
    input.set(
        "action_released",
        lua.create_function(|lua, name: String| {
            with_ctx(lua, |c| {
                check_action(&c.input, &name)?;
                Ok(c.input.action_released(&name))
            })
        })?,
    )?;
    input.set(
        "action_value",
        lua.create_function(|lua, name: String| {
            with_ctx(lua, |c| {
                check_action(&c.input, &name)?;
                Ok(c.input.action_value(&name))
            })
        })?,
    )?;
    input.set(
        "action_axis",
        lua.create_function(|lua, (neg, pos): (String, String)| {
            with_ctx(lua, |c| {
                check_action(&c.input, &neg)?;
                check_action(&c.input, &pos)?;
                Ok(c.input.action_axis(&neg, &pos))
            })
        })?,
    )?;

    Ok(input)
}
