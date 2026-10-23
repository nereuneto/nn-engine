//! `nn.particles` e `nn.particles3d`: emissores de partículas para os
//! scripts.

use glam::{Vec2, Vec3};
use mlua::{Lua, Result, Table, UserData, UserDataFields, UserDataMethods, Value};
use nn_render::d3::{EmitArea3, ParticleConfig3, ParticleEmitter3};
use nn_render::{Blend, EmitArea, ParticleConfig, ParticleEmitter};

use crate::api::with_ctx;
use crate::convert::{color, err, opt};
use crate::types::{LSheet, LTexture};

/// Emissor de partículas no Lua (2D ou 3D).
#[derive(Debug)]
pub(crate) enum LParticles {
    D2(ParticleEmitter),
    D3(ParticleEmitter3),
}

impl LParticles {
    fn emitting(&self) -> bool {
        match self {
            Self::D2(e) => e.emitting,
            Self::D3(e) => e.emitting,
        }
    }

    fn set_emitting(&mut self, v: bool) {
        match self {
            Self::D2(e) => e.emitting = v,
            Self::D3(e) => e.emitting = v,
        }
    }

    fn rate(&self) -> f32 {
        match self {
            Self::D2(e) => e.config.rate,
            Self::D3(e) => e.config.rate,
        }
    }

    fn set_rate(&mut self, v: f32) {
        match self {
            Self::D2(e) => e.config.rate = v.max(0.0),
            Self::D3(e) => e.config.rate = v.max(0.0),
        }
    }

    fn count(&self) -> usize {
        match self {
            Self::D2(e) => e.count(),
            Self::D3(e) => e.count(),
        }
    }

    fn finished(&self) -> bool {
        match self {
            Self::D2(e) => e.is_finished(),
            Self::D3(e) => e.is_finished(),
        }
    }

    fn position(&self) -> Vec3 {
        match self {
            Self::D2(e) => e.position.extend(0.0),
            Self::D3(e) => e.position,
        }
    }

    fn set_position(&mut self, x: Option<f32>, y: Option<f32>, z: Option<f32>) {
        let p = self.position();
        let p = Vec3::new(x.unwrap_or(p.x), y.unwrap_or(p.y), z.unwrap_or(p.z));
        match self {
            Self::D2(e) => e.position = p.truncate(),
            Self::D3(e) => e.position = p,
        }
    }
}

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
    c.follow = opt::<bool>(o, "follow")?.unwrap_or(false);
    c.pixel = opt::<bool>(o, "pixel")?.unwrap_or(false);

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

/// Opções do 3D (em metros; ângulos em graus).
fn config3_from(lua: &Lua, opts: &Table) -> Result<ParticleConfig3> {
    // As partes iguais às do 2D (imagem, cores, mistura...) vêm da leitura do
    // 2D, sem os campos que no 3D são outra coisa.
    let shared = lua.create_table()?;
    for pair in opts.pairs::<Value, Value>() {
        let (k, v) = pair?;
        let skip = match &k {
            Value::String(s) => matches!(
                s.to_str()?.as_ref(),
                "gravity" | "direction" | "area" | "spread"
            ),
            _ => false,
        };
        if !skip {
            shared.raw_set(k, v)?;
        }
    }
    let base = config_from(&shared)?;
    let o = Some(opts);
    let mut c = ParticleConfig3 {
        texture: base.texture,
        region: base.region,
        rate: base.rate,
        colors: base.colors,
        blend: base.blend,
        drag: base.drag,
        size_variation: base.size_variation,
        max: base.max,
        follow: base.follow,
        ..Default::default()
    };
    c.lifetime = range(opts, "lifetime", c.lifetime)?;
    c.speed = range(opts, "speed", c.speed)?;
    c.size = range(opts, "size", c.size)?;
    c.spin = range(opts, "spin", c.spin)?;
    c.spread = opt::<f32>(o, "spread")?.unwrap_or(c.spread);
    c.direction = vec3(opts, "direction", Vec3::Y, false)?;
    c.gravity = vec3(opts, "gravity", Vec3::ZERO, true)?;
    if let Some(area) = opt::<Table>(o, "area")? {
        let a = Some(&area);
        let sphere = opt::<bool>(a, "sphere")?.unwrap_or(false);
        c.area = match (
            opt::<f32>(a, "radius")?,
            opt::<f32>(a, "w")?,
            opt::<f32>(a, "h")?,
            opt::<f32>(a, "d")?,
        ) {
            (Some(r), ..) if sphere => EmitArea3::Sphere(r),
            (Some(r), ..) => EmitArea3::Disc(r),
            (None, w, h, d) if w.is_some() || h.is_some() || d.is_some() => EmitArea3::Box(
                Vec3::new(w.unwrap_or(0.0), h.unwrap_or(0.0), d.unwrap_or(0.0)) / 2.0,
            ),
            _ => EmitArea3::Point,
        };
    }
    Ok(c)
}

/// Um vetor 3D: `{x, y, z}` ou, com `number_is_y`, só um número (o y).
fn vec3(opts: &Table, key: &str, default: Vec3, number_is_y: bool) -> Result<Vec3> {
    let v: Value = opts.get(key)?;
    match v {
        Value::Nil => Ok(default),
        Value::Integer(i) if number_is_y => Ok(Vec3::new(0.0, i as f32, 0.0)),
        Value::Number(n) if number_is_y => Ok(Vec3::new(0.0, n as f32, 0.0)),
        Value::Table(t) => Ok(Vec3::new(
            t.get::<Option<f32>>(1)?.unwrap_or(0.0),
            t.get::<Option<f32>>(2)?.unwrap_or(0.0),
            t.get::<Option<f32>>(3)?.unwrap_or(0.0),
        )),
        other => Err(err(format!(
            "\"{key}\" deve ser {{x, y, z}}{}, recebeu {}",
            if number_is_y { " ou um número" } else { "" },
            other.type_name()
        ))),
    }
}

impl UserData for LParticles {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("x", |_, this| Ok(this.position().x));
        f.add_field_method_set("x", |_, this, v: f32| {
            this.set_position(Some(v), None, None);
            Ok(())
        });
        f.add_field_method_get("y", |_, this| Ok(this.position().y));
        f.add_field_method_set("y", |_, this, v: f32| {
            this.set_position(None, Some(v), None);
            Ok(())
        });
        f.add_field_method_get("z", |_, this| Ok(this.position().z));
        f.add_field_method_set("z", |_, this, v: f32| {
            this.set_position(None, None, Some(v));
            Ok(())
        });
        f.add_field_method_get("emitting", |_, this| Ok(this.emitting()));
        f.add_field_method_set("emitting", |_, this, v: bool| {
            this.set_emitting(v);
            Ok(())
        });
        f.add_field_method_get("rate", |_, this| Ok(this.rate()));
        f.add_field_method_set("rate", |_, this, v: f32| {
            this.set_rate(v);
            Ok(())
        });
        f.add_field_method_get("count", |_, this| Ok(this.count()));
        f.add_field_method_get("finished", |_, this| Ok(this.finished()));
        f.add_field_method_get("is3d", |_, this| Ok(matches!(this, Self::D3(_))));
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method_mut(
            "burst",
            |_, this, (n, x, y, z): (u32, Option<f32>, Option<f32>, Option<f32>)| {
                if x.is_some() || y.is_some() || z.is_some() {
                    this.set_position(x, y, z);
                }
                match this {
                    Self::D2(e) => e.burst(n),
                    Self::D3(e) => e.burst(n),
                }
                Ok(())
            },
        );
        m.add_method_mut(
            "set_position",
            |_, this, (x, y, z): (f32, f32, Option<f32>)| {
                this.set_position(Some(x), Some(y), z);
                Ok(())
            },
        );
        // 3D: para onde as partículas saem ({x, y, z}).
        m.add_method_mut("set_direction", |_, this, (x, y, z): (f32, f32, f32)| {
            match this {
                Self::D2(e) => e.config.direction = y.atan2(x),
                Self::D3(e) => e.config.direction = Vec3::new(x, y, z),
            }
            Ok(())
        });
        // 3D: o vento que leva as partículas (m/s).
        m.add_method_mut("set_wind", |_, this, (x, y, z): (f32, f32, f32)| {
            if let Self::D3(e) = this {
                e.config.wind = Vec3::new(x, y, z);
            }
            Ok(())
        });
        m.add_method_mut("update", |_, this, dt: f32| {
            match this {
                Self::D2(e) => e.update(dt),
                Self::D3(e) => e.update(dt),
            }
            Ok(())
        });
        m.add_method_mut("clear", |_, this, ()| {
            match this {
                Self::D2(e) => e.clear(),
                Self::D3(e) => e.clear(),
            }
            Ok(())
        });
        m.add_method("draw", |lua, this, ()| {
            with_ctx(lua, |c| {
                match this {
                    Self::D2(e) => e.draw(&mut c.renderer),
                    Self::D3(e) => e.draw(&mut c.renderer),
                }
                Ok(())
            })
        });
        m.add_method("draw_screen", |lua, this, ()| {
            with_ctx(lua, |c| {
                match this {
                    Self::D2(e) => e.draw_screen(&mut c.renderer),
                    Self::D3(_) => {
                        return Err(err("partículas 3D são desenhadas no mundo: use draw()"));
                    }
                }
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
        Ok(LParticles::D2(emitter))
    })
}

/// `nn.particles3d(opções)`
pub(crate) fn constructor3d(lua: &Lua) -> Result<mlua::Function> {
    lua.create_function(|lua, opts: Option<Table>| {
        let opts = match opts {
            Some(t) => t,
            None => lua.create_table()?,
        };
        let mut emitter = ParticleEmitter3::new(config3_from(lua, &opts)?);
        let o = Some(&opts);
        emitter.position = Vec3::new(
            opt::<f32>(o, "x")?.unwrap_or(0.0),
            opt::<f32>(o, "y")?.unwrap_or(0.0),
            opt::<f32>(o, "z")?.unwrap_or(0.0),
        );
        emitter.emitting = opt::<bool>(o, "emitting")?.unwrap_or(true);
        Ok(LParticles::D3(emitter))
    })
}
