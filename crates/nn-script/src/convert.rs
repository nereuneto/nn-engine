//! Conversões entre valores Lua e tipos da engine.

use mlua::{Error, Result, Table, Value};
use nn_core::Color;
use nn_input::{KeyCode, MouseButton};
use nn_render::{Align, Anchor};

pub(crate) fn err(msg: impl Into<String>) -> Error {
    Error::RuntimeError(msg.into())
}

/// Cor a partir de: número `0xRRGGBB`, texto `"#RRGGBB"` / `"#RRGGBBAA"`,
/// ou tabela `{r, g, b, a}` / `{r=, g=, b=, a=}` com valores de 0 a 1.
pub(crate) fn color(value: &Value) -> Result<Color> {
    match value {
        Value::Integer(n) => Ok(Color::hex(*n as u32)),
        Value::Number(n) => Ok(Color::hex(*n as u32)),
        Value::String(s) => {
            let s = s.to_str()?;
            let hex = s.trim_start_matches('#');
            let parsed =
                u32::from_str_radix(hex, 16).map_err(|_| err(format!("cor inválida: \"{s}\"")))?;
            match hex.len() {
                6 => Ok(Color::hex(parsed)),
                8 => Ok(Color::hex(parsed >> 8).with_alpha((parsed & 0xFF) as f32 / 255.0)),
                _ => Err(err(format!(
                    "cor inválida: \"{s}\" (use \"#RRGGBB\" ou \"#RRGGBBAA\")"
                ))),
            }
        }
        Value::Table(t) => {
            let get = |idx: i64, name: &str, default: f32| -> Result<f32> {
                let named: Option<f32> = t.get(name)?;
                let indexed: Option<f32> = t.get(idx)?;
                Ok(named.or(indexed).unwrap_or(default))
            };
            Ok(Color::rgba(
                get(1, "r", 0.0)?,
                get(2, "g", 0.0)?,
                get(3, "b", 0.0)?,
                get(4, "a", 1.0)?,
            ))
        }
        Value::Nil => Ok(Color::WHITE),
        other => Err(err(format!("cor inválida: {}", other.type_name()))),
    }
}

/// Lê um campo opcional de uma tabela de opções.
pub(crate) fn opt<T: mlua::FromLua>(opts: Option<&Table>, key: &str) -> Result<Option<T>> {
    match opts {
        Some(t) => t
            .get::<Option<T>>(key)
            .map_err(|e| err(format!("opção \"{key}\" inválida: {e}"))),
        None => Ok(None),
    }
}

pub(crate) fn opt_color(opts: Option<&Table>, key: &str) -> Result<Option<Color>> {
    match opts {
        Some(t) => {
            let v: Value = t.get(key)?;
            if v.is_nil() {
                Ok(None)
            } else {
                color(&v).map(Some)
            }
        }
        None => Ok(None),
    }
}

pub(crate) fn anchor(name: &str) -> Result<Anchor> {
    Ok(match name {
        "topleft" => Anchor::TopLeft,
        "top" => Anchor::TopCenter,
        "topright" => Anchor::TopRight,
        "left" => Anchor::CenterLeft,
        "center" => Anchor::Center,
        "right" => Anchor::CenterRight,
        "bottomleft" => Anchor::BottomLeft,
        "bottom" => Anchor::BottomCenter,
        "bottomright" => Anchor::BottomRight,
        _ => {
            return Err(err(format!(
                "âncora inválida: \"{name}\" (use topleft, top, topright, left, center, right, bottomleft, bottom ou bottomright)"
            )));
        }
    })
}

pub(crate) fn align(name: &str) -> Result<Align> {
    Ok(match name {
        "left" => Align::Left,
        "center" => Align::Center,
        "right" => Align::Right,
        _ => {
            return Err(err(format!(
                "alinhamento inválido: \"{name}\" (use left, center ou right)"
            )));
        }
    })
}

/// Tecla pelo nome. Algumas teclas têm duas versões (esquerda e direita):
/// devolvemos as duas e a consulta aceita qualquer uma.
pub(crate) fn keys(name: &str) -> Result<&'static [KeyCode]> {
    use KeyCode::*;
    let lower = name.to_ascii_lowercase();
    Ok(match lower.as_str() {
        "a" => &[KeyA],
        "b" => &[KeyB],
        "c" => &[KeyC],
        "d" => &[KeyD],
        "e" => &[KeyE],
        "f" => &[KeyF],
        "g" => &[KeyG],
        "h" => &[KeyH],
        "i" => &[KeyI],
        "j" => &[KeyJ],
        "k" => &[KeyK],
        "l" => &[KeyL],
        "m" => &[KeyM],
        "n" => &[KeyN],
        "o" => &[KeyO],
        "p" => &[KeyP],
        "q" => &[KeyQ],
        "r" => &[KeyR],
        "s" => &[KeyS],
        "t" => &[KeyT],
        "u" => &[KeyU],
        "v" => &[KeyV],
        "w" => &[KeyW],
        "x" => &[KeyX],
        "y" => &[KeyY],
        "z" => &[KeyZ],
        "0" => &[Digit0, Numpad0],
        "1" => &[Digit1, Numpad1],
        "2" => &[Digit2, Numpad2],
        "3" => &[Digit3, Numpad3],
        "4" => &[Digit4, Numpad4],
        "5" => &[Digit5, Numpad5],
        "6" => &[Digit6, Numpad6],
        "7" => &[Digit7, Numpad7],
        "8" => &[Digit8, Numpad8],
        "9" => &[Digit9, Numpad9],
        "space" => &[Space],
        "enter" | "return" => &[Enter, NumpadEnter],
        "escape" | "esc" => &[Escape],
        "tab" => &[Tab],
        "backspace" => &[Backspace],
        "delete" => &[Delete],
        "shift" => &[ShiftLeft, ShiftRight],
        "ctrl" => &[ControlLeft, ControlRight],
        "alt" => &[AltLeft, AltRight],
        "left" => &[ArrowLeft],
        "right" => &[ArrowRight],
        "up" => &[ArrowUp],
        "down" => &[ArrowDown],
        "f1" => &[F1],
        "f2" => &[F2],
        "f3" => &[F3],
        "f4" => &[F4],
        "f5" => &[F5],
        "f6" => &[F6],
        "f7" => &[F7],
        "f8" => &[F8],
        "f9" => &[F9],
        "f10" => &[F10],
        "f11" => &[F11],
        "f12" => &[F12],
        _ => return Err(err(format!("tecla desconhecida: \"{name}\""))),
    })
}

pub(crate) fn mouse_button(name: &str) -> Result<MouseButton> {
    Ok(match name {
        "left" => MouseButton::Left,
        "right" => MouseButton::Right,
        "middle" => MouseButton::Middle,
        _ => {
            return Err(err(format!(
                "botão do mouse inválido: \"{name}\" (use left, right ou middle)"
            )));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn cores_em_varios_formatos() {
        let lua = Lua::new();
        let c = |code: &str| color(&lua.load(code).eval::<Value>().unwrap()).unwrap();
        assert_eq!(c("0xFF8000"), Color::hex(0xFF8000));
        assert_eq!(c("'#FF8000'"), Color::hex(0xFF8000));
        assert_eq!(
            c("'#FF800080'"),
            Color::hex(0xFF8000).with_alpha(128.0 / 255.0)
        );
        assert_eq!(c("{1, 0.5, 0}"), Color::rgba(1.0, 0.5, 0.0, 1.0));
        assert_eq!(
            c("{r = 1, b = 1, a = 0.5}"),
            Color::rgba(1.0, 0.0, 1.0, 0.5)
        );
        assert!(color(&lua.load("'#XYZ'").eval::<Value>().unwrap()).is_err());
        assert!(color(&Value::Boolean(true)).is_err());
    }

    #[test]
    fn teclas_por_nome() {
        assert_eq!(keys("W").unwrap(), &[KeyCode::KeyW]);
        assert_eq!(keys("shift").unwrap().len(), 2);
        assert!(keys("tecla-que-nao-existe").is_err());
    }
}
