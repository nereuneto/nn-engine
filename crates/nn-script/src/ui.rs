//! `nn.ui`: menus com botões, opções e barras, por mouse, teclado e controle.

use glam::Vec2;
use mlua::{Lua, Result, Table, UserData, UserDataMethods, Value};
use nn_app::{Context, Panel, PanelOptions};
use nn_core::Color;

use crate::api::with_ctx;
use crate::convert::{anchor, color, err, opt, opt_color};
use crate::types::{LFont, LSoundData, expect};

/// Um painel aberto com `nn.ui.panel`.
#[derive(Debug)]
pub(crate) struct LPanel {
    id: String,
}

impl LPanel {
    /// Roda `f` com o painel, se ele ainda estiver aberto.
    fn with<R>(&self, lua: &Lua, f: impl FnOnce(&mut Panel<'_>) -> Result<R>) -> Result<R> {
        with_ctx(lua, |c: &mut Context| {
            if c.ui.open_id() != Some(self.id.as_str()) {
                return Err(err(format!(
                    "o painel \"{}\" já foi fechado (finish ou outro nn.ui.panel); \
                     crie o painel de novo a cada frame, dentro de draw",
                    self.id
                )));
            }
            let mut panel = c.open_panel().expect("painel aberto conferido acima");
            f(&mut panel)
        })
    }
}

impl UserData for LPanel {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("button", |lua, this, label: String| {
            this.with(lua, |p| Ok(p.button(&label)))
        });

        m.add_method(
            "toggle",
            |lua, this, (label, value, opts): (String, bool, Option<Table>)| {
                let on = opt::<String>(opts.as_ref(), "on")?;
                let off = opt::<String>(opts.as_ref(), "off")?;
                this.with(lua, |p| {
                    let mut value = value;
                    let theme = p.ctx().ui.theme();
                    let on = on.unwrap_or_else(|| theme.on_text.clone());
                    let off = off.unwrap_or_else(|| theme.off_text.clone());
                    let changed = p.toggle_with(&label, &mut value, &on, &off);
                    Ok((value, changed))
                })
            },
        );

        m.add_method(
            "choice",
            |lua, this, (label, options, index): (String, Table, Option<i64>)| {
                let options: Vec<String> = options
                    .sequence_values::<Value>()
                    .map(|v| {
                        let v = v?;
                        Ok(match lua.coerce_string(v.clone())? {
                            Some(s) => s.to_string_lossy(),
                            None => {
                                return Err(err(format!(
                                    "as opções devem ser textos ou números, recebeu {}",
                                    v.type_name()
                                )));
                            }
                        })
                    })
                    .collect::<Result<_>>()?;
                if options.is_empty() {
                    return Err(err("ui:choice precisa de pelo menos uma opção"));
                }
                let n = options.len() as i64;
                let mut i = (index.unwrap_or(1).clamp(1, n) - 1) as usize;
                this.with(lua, |p| {
                    let changed = p.choice(&label, &options, &mut i);
                    Ok((i as i64 + 1, changed))
                })
            },
        );

        m.add_method(
            "slider",
            |lua, this, (label, value, opts): (String, f32, Option<Table>)| {
                let min = opt::<f32>(opts.as_ref(), "min")?.unwrap_or(0.0);
                let max = opt::<f32>(opts.as_ref(), "max")?.unwrap_or(1.0);
                let step = opt::<f32>(opts.as_ref(), "step")?.unwrap_or((max - min) / 10.0);
                this.with(lua, |p| {
                    let mut value = value;
                    let changed = p.slider(&label, &mut value, min, max, step);
                    // Sem resíduos de f32 (0.6 em vez de 0.6000000238...), o que
                    // deixa comparações e o arquivo salvo limpos.
                    let value = (f64::from(value) * 1e6).round() / 1e6;
                    Ok((value, changed))
                })
            },
        );

        m.add_method(
            "label",
            |lua, this, (text, opts): (Value, Option<Table>)| {
                let text = match lua.coerce_string(text.clone())? {
                    Some(s) => s.to_string_lossy(),
                    None => String::new(),
                };
                let col = opt_color(opts.as_ref(), "color")?;
                let size = opt::<f32>(opts.as_ref(), "size")?.unwrap_or(1.0);
                this.with(lua, |p| {
                    let col = col.unwrap_or(p.ctx().ui.theme().label_color);
                    p.label_with(&text, col, size);
                    Ok(())
                })
            },
        );

        m.add_method("space", |lua, this, h: f32| {
            this.with(lua, |p| {
                p.space(h);
                Ok(())
            })
        });

        m.add_method("back", |lua, this, ()| this.with(lua, |p| Ok(p.back())));

        m.add_method("focus", |lua, this, ()| {
            this.with(lua, |p| Ok(p.focus() as i64 + 1))
        });

        m.add_method("finish", |lua, this, ()| {
            with_ctx(lua, |c| {
                if c.ui.open_id() == Some(this.id.as_str()) {
                    c.finish_panel();
                }
                Ok(())
            })
        });
    }
}

fn panel_options(opts: Option<&Table>) -> Result<PanelOptions> {
    let mut o = PanelOptions::new();
    o.title = opt::<String>(opts, "title")?;
    o.width = opt::<f32>(opts, "width")?;
    let x = opt::<f32>(opts, "x")?;
    let y = opt::<f32>(opts, "y")?;
    match (x, y) {
        (Some(x), Some(y)) => o.position = Some(Vec2::new(x, y)),
        (None, None) => {}
        _ => return Err(err("informe x e y juntos para posicionar o painel")),
    }
    if let Some(name) = opt::<String>(opts, "anchor")? {
        o.anchor = Some(anchor(&name)?);
    }
    o.background = opt_color(opts, "background")?;
    Ok(o)
}

fn set_color(t: &Table, key: &str, target: &mut Color) -> Result<()> {
    let v: Value = t.get(key)?;
    if !v.is_nil() {
        *target = color(&v).map_err(|e| err(format!("{key}: {e}")))?;
    }
    Ok(())
}

fn apply_theme(lua: &Lua, t: &Table) -> Result<()> {
    with_ctx(lua, |c| {
        let mut theme = c.ui.theme().clone();
        let font: Value = t.get("font")?;
        match font {
            Value::Nil => {}
            Value::Boolean(false) => theme.font = None,
            other => theme.font = Some(expect::<LFont>(&other, "uma fonte (nn.assets.font)")?.0),
        }
        for (key, target) in [
            ("text_size", &mut theme.text_size),
            ("title_size", &mut theme.title_size),
            ("width", &mut theme.width),
            ("padding", &mut theme.padding),
            ("spacing", &mut theme.spacing),
            ("row_height", &mut theme.row_height),
        ] {
            if let Some(v) = t.get::<Option<f32>>(key)? {
                *target = v.max(0.0);
            }
        }
        set_color(t, "text_color", &mut theme.text_color)?;
        set_color(t, "label_color", &mut theme.label_color)?;
        set_color(t, "panel_color", &mut theme.panel_color)?;
        set_color(t, "button_color", &mut theme.button_color)?;
        set_color(t, "focus_color", &mut theme.focus_color)?;
        set_color(t, "focus_text_color", &mut theme.focus_text_color)?;
        set_color(t, "accent_color", &mut theme.accent_color)?;
        match t.get::<Value>("scale")? {
            Value::Nil => {}
            Value::String(s) if s.to_str()? == "auto" => theme.scale = None,
            v => match v.as_f64() {
                Some(s) if s > 0.0 => theme.scale = Some(s as f32),
                _ => return Err(err("scale: use um número maior que zero ou \"auto\"")),
            },
        }
        if let Some(s) = t.get::<Option<String>>("on_text")? {
            theme.on_text = s;
        }
        if let Some(s) = t.get::<Option<String>>("off_text")? {
            theme.off_text = s;
        }
        for (key, target) in [
            ("move_sound", &mut theme.move_sound),
            ("confirm_sound", &mut theme.confirm_sound),
        ] {
            match t.get::<Value>(key)? {
                Value::Nil => {}
                Value::Boolean(false) => *target = None,
                v => *target = Some(expect::<LSoundData>(&v, "um som (nn.assets.sound)")?.0),
            }
        }
        c.ui.set_theme(theme);
        Ok(())
    })
}

pub(crate) fn table(lua: &Lua) -> Result<Table> {
    let ui = lua.create_table()?;
    ui.set(
        "panel",
        lua.create_function(|lua, (id, opts): (String, Option<Table>)| {
            let options = panel_options(opts.as_ref())?;
            with_ctx(lua, |c| {
                c.panel(id.clone(), options);
                Ok(LPanel { id })
            })
        })?,
    )?;
    ui.set(
        "theme",
        lua.create_function(|lua, t: Table| apply_theme(lua, &t))?,
    )?;
    ui.set(
        "set_focus",
        lua.create_function(|lua, (id, index): (String, i64)| {
            with_ctx(lua, |c| {
                c.ui.set_focus(&id, (index.max(1) - 1) as usize);
                Ok(())
            })
        })?,
    )?;
    ui.set(
        "reset",
        lua.create_function(|lua, id: String| {
            with_ctx(lua, |c| {
                c.ui.reset(&id);
                Ok(())
            })
        })?,
    )?;
    Ok(ui)
}
