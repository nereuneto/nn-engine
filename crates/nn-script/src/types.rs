//! Tipos expostos ao Lua como userdata.

use mlua::{AnyUserData, Lua, Result, Table, UserData, UserDataFields, UserDataMethods, Value};
use nn_audio::{PlayOptions, Sound, SoundId};
use nn_render::{Animation, AnimationPlayer, FontId, Playback, SpriteSheet, TextureId};

use crate::api::with_ctx;
use crate::convert::{err, opt};

/// Imagem carregada.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LTexture(pub TextureId);

impl UserData for LTexture {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("width", |lua, this| {
            with_ctx(lua, |c| Ok(c.renderer.texture_size(this.0).x))
        });
        f.add_field_method_get("height", |lua, this| {
            with_ctx(lua, |c| Ok(c.renderer.texture_size(this.0).y))
        });
    }
}

/// Spritesheet: imagem dividida em quadros. Quadros numerados a partir de 1.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LSheet(pub SpriteSheet);

impl UserData for LSheet {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("count", |_, this| Ok(this.0.frame_count()));
        f.add_field_method_get("columns", |_, this| Ok(this.0.columns()));
        f.add_field_method_get("rows", |_, this| Ok(this.0.rows()));
        f.add_field_method_get("frame_width", |_, this| Ok(this.0.frame_size().x));
        f.add_field_method_get("frame_height", |_, this| Ok(this.0.frame_size().y));
    }
}

/// Fonte carregada.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LFont(pub FontId);

impl UserData for LFont {}

/// Som carregado (ainda não tocando).
#[derive(Debug, Clone, Copy)]
pub(crate) struct LSoundData(pub SoundId);

impl UserData for LSoundData {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("play", |lua, this, opts: Option<Table>| {
            play(lua, this.0, opts.as_ref())
        });
    }

    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("duration", |lua, this| {
            with_ctx(lua, |c| Ok(c.audio.duration(this.0).as_secs_f64()))
        });
    }
}

pub(crate) fn play_options(opts: Option<&Table>) -> Result<PlayOptions> {
    let mut o = PlayOptions::default();
    if let Some(v) = opt::<f32>(opts, "volume")? {
        o.volume = v;
    }
    if let Some(v) = opt::<f32>(opts, "speed")? {
        o.speed = v;
    }
    if let Some(v) = opt::<f32>(opts, "pan")? {
        o.pan = v;
    }
    if let Some(v) = opt::<bool>(opts, "loop")? {
        o.looping = v;
    }
    if let Some(v) = opt::<String>(opts, "channel")? {
        o.channel = crate::convert::channel(&v)?;
    }
    // Som 3D: x, y, z (qualquer um basta) e a distância máxima.
    let (x, y, z) = (
        opt::<f32>(opts, "x")?,
        opt::<f32>(opts, "y")?,
        opt::<f32>(opts, "z")?,
    );
    if x.is_some() || y.is_some() || z.is_some() {
        o.position = Some(glam::Vec3::new(
            x.unwrap_or(0.0),
            y.unwrap_or(0.0),
            z.unwrap_or(0.0),
        ));
    }
    if let Some(v) = opt::<f32>(opts, "distance")? {
        o.distance = v.max(0.1);
    }
    Ok(o)
}

pub(crate) fn play(lua: &Lua, id: SoundId, opts: Option<&Table>) -> Result<LSound> {
    let options = play_options(opts)?;
    with_ctx(lua, |c| Ok(LSound(c.audio.play_with(id, options))))
}

/// Som tocando.
#[derive(Debug)]
pub(crate) struct LSound(pub Sound);

impl UserData for LSound {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method_mut("stop", |_, this, ()| {
            this.0.stop();
            Ok(())
        });
        m.add_method_mut("fade_out", |_, this, seconds: f64| {
            this.0
                .fade_out(std::time::Duration::from_secs_f64(seconds.max(0.0)));
            Ok(())
        });
        m.add_method_mut("pause", |_, this, ()| {
            this.0.pause();
            Ok(())
        });
        m.add_method_mut("resume", |_, this, ()| {
            this.0.resume();
            Ok(())
        });
        m.add_method_mut("set_volume", |_, this, v: f32| {
            this.0.set_volume(v);
            Ok(())
        });
        m.add_method_mut("set_speed", |_, this, v: f32| {
            this.0.set_speed(v);
            Ok(())
        });
        m.add_method_mut("set_pan", |_, this, v: f32| {
            this.0.set_pan(v);
            Ok(())
        });
        m.add_method("playing", |_, this, ()| Ok(this.0.is_playing()));
        // Som 3D: muda onde ele está (o volume e o lado acompanham).
        m.add_method_mut("set_position", |_, this, (x, y, z): (f32, f32, f32)| {
            this.0.set_position(glam::Vec3::new(x, y, z));
            Ok(())
        });
        m.add_method("position", |_, this, ()| {
            let p = this.0.position();
            Ok((p.map(|p| p.x), p.map(|p| p.y), p.map(|p| p.z)))
        });
        // O volume que a distância deixa (0 a 1; 1 num som comum).
        m.add_method("distance_volume", |_, this, ()| Ok(this.0.distance_gain()));
    }
}

/// Definição de animação. Quadros numerados a partir de 1, como em Lua.
#[derive(Debug, Clone)]
pub(crate) struct LAnimation(pub Animation);

impl LAnimation {
    pub fn from_lua(
        frames: &Table,
        fps: f32,
        mode: Option<&str>,
        times: Option<&Table>,
    ) -> Result<Self> {
        let mut list = Vec::new();
        for v in frames.sequence_values::<i64>() {
            let v = v.map_err(|_| err("os quadros da animação devem ser números"))?;
            if v < 1 {
                return Err(err("quadros de animação começam em 1"));
            }
            list.push((v - 1) as u32);
        }
        if list.is_empty() {
            return Err(err("animação sem quadros"));
        }
        if fps <= 0.0 {
            return Err(err("o fps da animação precisa ser maior que zero"));
        }
        let anim = Animation::new(list, fps);
        let anim = match mode.unwrap_or("loop") {
            "loop" => anim,
            "once" => anim.once(),
            "pingpong" => anim.ping_pong(),
            other => {
                return Err(err(format!(
                    "modo de animação inválido: \"{other}\" (use loop, once ou pingpong)"
                )));
            }
        };
        let anim = match times {
            Some(t) => {
                let mut list = Vec::new();
                for v in t.sequence_values::<f32>() {
                    list.push(v.map_err(|_| {
                        err("as durações da animação devem ser números (segundos)")
                    })?);
                }
                anim.with_times(list)
            }
            None => anim,
        };
        Ok(Self(anim))
    }
}

impl UserData for LAnimation {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("duration", |_, this| Ok(this.0.duration()));
        f.add_field_method_get("length", |_, this| Ok(this.0.frames().len()));
        f.add_field_method_get("mode", |_, this| {
            Ok(match this.0.playback() {
                Playback::Loop => "loop",
                Playback::Once => "once",
                Playback::PingPong => "pingpong",
            })
        });
    }
}

/// Toca animações e guarda o tempo decorrido.
#[derive(Debug)]
pub(crate) struct LAnimator(pub AnimationPlayer);

impl UserData for LAnimator {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("speed", |_, this| Ok(this.0.speed));
        f.add_field_method_set("speed", |_, this, v: f32| {
            this.0.speed = v;
            Ok(())
        });
        f.add_field_method_get("time", |_, this| Ok(this.0.time()));
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method_mut("play", |_, this, anim: AnyUserData| {
            let anim = anim.borrow::<LAnimation>()?;
            this.0.play(&anim.0);
            Ok(())
        });
        m.add_method_mut("update", |_, this, dt: f32| {
            this.0.update(dt);
            Ok(())
        });
        m.add_method_mut("restart", |_, this, ()| {
            this.0.restart();
            Ok(())
        });
        m.add_method("frame", |_, this, ()| Ok(this.0.frame() + 1));
        m.add_method("finished", |_, this, ()| Ok(this.0.is_finished()));
    }
}

/// Converte um valor em referência a userdata do tipo `T`, com mensagem clara.
pub(crate) fn expect<T: 'static + Copy>(value: &Value, what: &str) -> Result<T> {
    match value {
        Value::UserData(ud) => ud
            .borrow::<T>()
            .map(|r| *r)
            .map_err(|_| err(format!("esperava {what}"))),
        other => Err(err(format!(
            "esperava {what}, recebeu {}",
            other.type_name()
        ))),
    }
}
