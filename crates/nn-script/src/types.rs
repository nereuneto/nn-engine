//! Tipos expostos ao Lua como userdata.

use mlua::{AnyUserData, Lua, Result, Table, UserData, UserDataFields, UserDataMethods, Value};
use nn_app::AssetHandle;
use nn_audio::{PlayOptions, Sound, SoundId};
use nn_render::{Animation, AnimationPlayer, FontId, Playback, SpriteSheet, TextureId};

use crate::api::with_ctx;
use crate::convert::{err, opt};

/// Um efeito (shader) de `nn.effect`. Os parâmetros são campos:
/// `e.forca = 0.5`; uma cor (`e.cor = "#FF0000"`) muda `cor_r`, `cor_g`,
/// `cor_b` e `cor_a`.
#[derive(Debug, Clone)]
pub(crate) struct LEffect(
    pub nn_render::effects::EffectId,
    #[allow(dead_code)] pub AssetHandle,
);

impl LEffect {
    /// Muda um parâmetro: um número, ou uma cor para `nome_r`... `nome_a`.
    /// Com o arquivo do efeito com erro, não reclama (o jogo segue sem o
    /// efeito).
    pub(crate) fn set(
        lua: &Lua,
        id: nn_render::effects::EffectId,
        name: &str,
        value: &Value,
    ) -> Result<()> {
        match value {
            Value::Integer(_) | Value::Number(_) => {
                let v: f32 = lua.unpack(value.clone())?;
                with_ctx(lua, |c| {
                    if c.renderer.set_effect_param(id, name, v) || c.effect_error(id).is_some() {
                        Ok(())
                    } else {
                        Err(err(no_param(c, id, name)))
                    }
                })
            }
            Value::String(_) | Value::Table(_) => {
                let col = crate::convert::color(value).map_err(|_| {
                    let shown = lua
                        .coerce_string(value.clone())
                        .ok()
                        .flatten()
                        .map(|s| s.to_string_lossy())
                        .unwrap_or_else(|| "uma tabela".into());
                    err(format!(
                        "cor inválida para \"{name}\": \"{shown}\" (use \"#RRGGBB\" ou \"#RRGGBBAA\")"
                    ))
                })?;
                with_ctx(lua, |c| {
                    if c.renderer.set_effect_color(id, name, col) || c.effect_error(id).is_some() {
                        Ok(())
                    } else {
                        let colors = c.renderer.effect_colors(id);
                        Err(err(if colors.is_empty() {
                            format!("o efeito não tem a cor \"{name}\" (ele não tem cores)")
                        } else {
                            format!(
                                "o efeito não tem a cor \"{name}\" (tem: {})",
                                colors.join(", ")
                            )
                        }))
                    }
                })
            }
            other => Err(err(format!(
                "o parâmetro \"{name}\" do efeito precisa ser um número ou uma cor, recebeu {}",
                other.type_name()
            ))),
        }
    }
}

fn no_param(c: &nn_app::Context, id: nn_render::effects::EffectId, name: &str) -> String {
    let names: Vec<&str> = c
        .renderer
        .effect_params(id)
        .iter()
        .map(|(n, _)| n.as_str())
        .collect();
    if names.is_empty() {
        format!("o efeito não tem o parâmetro \"{name}\" (ele não tem parâmetros)")
    } else {
        format!(
            "o efeito não tem o parâmetro \"{name}\" (tem: {})",
            names.join(", ")
        )
    }
}

impl UserData for LEffect {
    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("set", |lua, this, (name, value): (String, Value)| {
            LEffect::set(lua, this.0, &name, &value)
        });
        m.add_meta_method(mlua::MetaMethod::Index, |lua, this, key: String| {
            let id = this.0;
            match key.as_str() {
                // O erro do arquivo .wgsl (com a linha), ou nil.
                "error" => with_ctx(lua, |c| Ok(c.effect_error(id))).and_then(|e| lua.pack(e)),
                "params" => with_ctx(lua, |c| {
                    Ok(c.renderer
                        .effect_params(id)
                        .iter()
                        .map(|(n, _)| n.clone())
                        .collect::<Vec<_>>())
                })
                .and_then(|v| lua.pack(v)),
                // Um número, ou uma cor ("#RRGGBBAA") para nome_r... nome_a.
                name => with_ctx(lua, |c| {
                    if let Some(v) = c.renderer.effect_param(id, name) {
                        return Ok(Value::Number(f64::from(v)));
                    }
                    Ok(match c.renderer.effect_color(id, name) {
                        Some(col) => {
                            let b = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
                            Value::String(lua.create_string(format!(
                                "#{:02X}{:02X}{:02X}{:02X}",
                                b(col.r),
                                b(col.g),
                                b(col.b),
                                b(col.a)
                            ))?)
                        }
                        None => Value::Nil,
                    })
                }),
            }
        });
        m.add_meta_method(
            mlua::MetaMethod::NewIndex,
            |lua, this, (key, value): (String, Value)| LEffect::set(lua, this.0, &key, &value),
        );
    }
}

/// Imagem carregada. O `AssetHandle` diz que a imagem está em uso: quando
/// nenhum sobra (o Lua soltou todas), a troca de cena a descarrega.
#[derive(Debug, Clone)]
pub(crate) struct LTexture(pub TextureId, pub Option<AssetHandle>);

impl UserData for LTexture {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("width", |lua, this| {
            with_ctx(lua, |c| Ok(c.renderer.texture_size(this.0).x))
        });
        f.add_field_method_get("height", |lua, this| {
            with_ctx(lua, |c| Ok(c.renderer.texture_size(this.0).y))
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        // Num canvas: desenha nele o que a função desenhar.
        m.add_method(
            "draw",
            |lua, this, (f, opts): (mlua::Function, Option<Table>)| {
                let o = opts.as_ref();
                // Limpa antes (transparente, ou a cor pedida); false mantém
                // o que já estava (rastros).
                let clear = match opt::<Value>(o, "clear")? {
                    None => Some(nn_core::Color::rgba(0.0, 0.0, 0.0, 0.0)),
                    Some(Value::Boolean(false)) => None,
                    Some(v) => Some(crate::convert::color(&v)?),
                };
                let (x, y, zoom) = (
                    opt::<f32>(o, "x")?,
                    opt::<f32>(o, "y")?,
                    opt::<f32>(o, "zoom")?,
                );
                let id = this.0;
                let camera = with_ctx(lua, |c| {
                    if !c.renderer.begin_canvas(id) {
                        return Err(err(
                            "esta imagem não é um canvas: crie com nn.canvas(largura, altura)",
                        ));
                    }
                    // O mundo, pela câmera do jogo (ou a pedida).
                    let mut cam = c.camera;
                    if let Some(x) = x {
                        cam.position.x = x;
                    }
                    if let Some(y) = y {
                        cam.position.y = y;
                    }
                    if let Some(z) = zoom.filter(|z| *z > 0.0) {
                        cam.zoom = z;
                    }
                    Ok(cam)
                })?;
                let result = f.call::<()>(());
                with_ctx(lua, |c| {
                    c.renderer.end_canvas(&camera, clear);
                    Ok(())
                })?;
                result
            },
        );
    }
}

/// Spritesheet: imagem dividida em quadros. Quadros numerados a partir de 1.
#[derive(Debug, Clone)]
pub(crate) struct LSheet(pub SpriteSheet, pub Option<AssetHandle>);

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
#[derive(Debug, Clone)]
pub(crate) struct LSoundData(pub SoundId, #[allow(dead_code)] pub Option<AssetHandle>);

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
pub(crate) fn expect<T: 'static + Clone>(value: &Value, what: &str) -> Result<T> {
    match value {
        Value::UserData(ud) => ud
            .borrow::<T>()
            .map(|r| r.clone())
            .map_err(|_| err(format!("esperava {what}"))),
        other => Err(err(format!(
            "esperava {what}, recebeu {}",
            other.type_name()
        ))),
    }
}
