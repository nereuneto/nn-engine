//! `nn.particles`: emissores de partículas para os scripts.

use glam::Vec2;
use mlua::{Lua, Result, Table, UserData, UserDataFields, UserDataMethods, Value};
use nn_render::{Blend, EmitArea, ParticleConfig, ParticleEmitter};

use crate::api::with_ctx;
use crate::convert::{color, err, opt};
use crate::types::{LSheet, LTexture};

/// Emissor de partículas no Lua.
#[derive(Debug)]
pub(crate) struct LParticles(ParticleEmitter);

/// Lê um número ou uma faixa `{min, max}`.
fn range(opts: &Table, key: &str, default: (f32, f32)) -> Result<(f32, f32)> {
    let v: Value = opts.get(key)?;
    match v {
        Value::Nil => Ok(default),
        Value::Integer(i) => Ok((i as f32, i as f32)),
        Value::Number(n) => Ok((n as f32, n as f32)),
        Value::Table(t) => {
            let a: f32 = t.get(1).map_err(|_| {
                err(format!(
                    "\"{key}\" deve ser um número ou {{mínimo, máximo}}"
                ))
            })?;
            let b: Option<f32> = t.get(2)?;
            Ok((a, b.unwrap_or(a)))
        }
        _ => Err(err(format!(
            "\"{key}\" deve ser um número ou {{mínimo, máximo}}"
        ))),
    }
}

fn config_from(opts: &Table) -> Result<ParticleConfig> {
    let o = Some(opts);
    let mut c = ParticleConfig::default();

    let texture: Value = opts.get("texture")?;
    match &texture {
        Value::Nil => {}
        Value::UserData(ud) if ud.is::<LTexture>() => c.texture = Some(ud.borrow::<LTexture>()?.0),
        Value::UserData(ud) if ud.is::<LSheet>() => {
            let sheet = ud.borrow::<LSheet>()?.0;
            let frame = opt::<i64>(o, "frame")?.unwrap_or(1);
            if frame < 1 || frame as u32 > sheet.frame_count() {
                return Err(err(format!("quadro {frame} não existe na spritesheet")));
            }
            c.texture = Some(sheet.texture());
            c.region = sheet.region(frame as u32 - 1);
        }
        other => {
            return Err(err(format!(
                "\"texture\" deve ser uma imagem ou spritesheet, recebeu {}",
                other.type_name()
            )));
        }
    }

    c.rate = opt::<f32>(o, "rate")?.unwrap_or(0.0);
    c.lifetime = range(opts, "lifetime", c.lifetime)?;
    c.speed = range(opts, "speed", c.speed)?;
    c.direction = opt::<f32>(o, "direction")?.unwrap_or(c.direction);
    c.spread = opt::<f32>(o, "spread")?.unwrap_or(c.spread);
    c.drag = opt::<f32>(o, "drag")?.unwrap_or(0.0);
    c.size = range(opts, "size", c.size)?;
    c.size_variation = opt::<f32>(o, "size_variation")?.unwrap_or(0.0);
    c.spin = range(opts, "spin", c.spin)?;
    c.max = opt::<usize>(o, "max")?.unwrap_or(c.max);

    let gravity: Value = opts.get("gravity")?;
    c.gravity = match gravity {
        Value::Nil => Vec2::ZERO,
        Value::Integer(i) => Vec2::new(0.0, i as f32),
        Value::Number(n) => Vec2::new(0.0, n as f32),
        Value::Table(t) => Vec2::new(
            t.get::<Option<f32>>(1)?.unwrap_or(0.0),
            t.get::<Option<f32>>(2)?.unwrap_or(0.0),
        ),
        other => return Err(err(format!("\"gravity\" inválida: {}", other.type_name()))),
    };

    let colors: Value = opts.get("colors")?;
    if let Value::Table(list) = colors {
        let mut out = Vec::new();
        for v in list.sequence_values::<Value>() {
            out.push(color(&v?)?);
        }
        if out.is_empty() {
            return Err(err("\"colors\" precisa de pelo menos uma cor"));
        }
        c.colors = out;
    } else if let Some(single) = opts.get::<Option<Value>>("color")? {
        // Uma cor só: some aos poucos até o fim da vida.
        let base = color(&single)?;
        c.colors = vec![base, base.with_alpha(0.0)];
    }

    if let Some(area) = opt::<Table>(o, "area")? {
        let a = Some(&area);
        c.area = match (
            opt::<f32>(a, "radius")?,
            opt::<f32>(a, "w")?,
            opt::<f32>(a, "h")?,
        ) {
            (Some(r), _, _) => EmitArea::Circle(r),
            (None, w, h) if w.is_some() || h.is_some() => {
                EmitArea::Rect(Vec2::new(w.unwrap_or(0.0), h.unwrap_or(0.0)) / 2.0)
            }
            _ => EmitArea::Point,
        };
    }

    c.blend = match opt::<String>(o, "blend")?.as_deref() {
        None | Some("alpha") => Blend::Alpha,
        Some("add") => Blend::Add,
        Some(other) => {
            return Err(err(format!(
                "mistura inválida: \"{other}\" (use alpha ou add)"
            )));
        }
    };
    Ok(c)
}

impl UserData for LParticles {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("x", |_, this| Ok(this.0.position.x));
        f.add_field_method_set("x", |_, this, v: f32| {
            this.0.position.x = v;
            Ok(())
        });
        f.add_field_method_get("y", |_, this| Ok(this.0.position.y));
        f.add_field_method_set("y", |_, this, v: f32| {
            this.0.position.y = v;
            Ok(())
        });
        f.add_field_method_get("emitting", |_, this| Ok(this.0.emitting));
        f.add_field_method_set("emitting", |_, this, v: bool| {
            this.0.emitting = v;
            Ok(())
        });
        f.add_field_method_get("rate", |_, this| Ok(this.0.config.rate));
        f.add_field_method_set("rate", |_, this, v: f32| {
            this.0.config.rate = v.max(0.0);
            Ok(())
        });
        f.add_field_method_get("count", |_, this| Ok(this.0.count()));
        f.add_field_method_get("finished", |_, this| Ok(this.0.is_finished()));
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method_mut(
            "burst",
            |_, this, (n, x, y): (u32, Option<f32>, Option<f32>)| {
                if let (Some(x), Some(y)) = (x, y) {
                    this.0.position = Vec2::new(x, y);
                }
                this.0.burst(n);
                Ok(())
            },
        );
        m.add_method_mut("set_position", |_, this, (x, y): (f32, f32)| {
            this.0.position = Vec2::new(x, y);
            Ok(())
        });
        m.add_method_mut("update", |_, this, dt: f32| {
            this.0.update(dt);
            Ok(())
        });
        m.add_method_mut("clear", |_, this, ()| {
            this.0.clear();
            Ok(())
        });
        m.add_method("draw", |lua, this, ()| {
            with_ctx(lua, |c| {
                this.0.draw(&mut c.renderer);
                Ok(())
            })
        });
        m.add_method("draw_screen", |lua, this, ()| {
            with_ctx(lua, |c| {
                this.0.draw_screen(&mut c.renderer);
                Ok(())
            })
        });
    }
}

/// `nn.particles(opções)`
pub(crate) fn constructor(lua: &Lua) -> Result<mlua::Function> {
    lua.create_function(|lua, opts: Option<Table>| {
        let opts = match opts {
            Some(t) => t,
            None => lua.create_table()?,
        };
        let mut emitter = ParticleEmitter::new(config_from(&opts)?);
        emitter.position = Vec2::new(
            opt::<f32>(Some(&opts), "x")?.unwrap_or(0.0),
            opt::<f32>(Some(&opts), "y")?.unwrap_or(0.0),
        );
        // Sem taxa contínua, o emissor só solta partículas com burst.
        emitter.emitting = opt::<bool>(Some(&opts), "emitting")?.unwrap_or(true);
        Ok(LParticles(emitter))
    })
}
