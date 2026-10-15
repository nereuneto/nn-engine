//! A tabela global `nn`: tudo o que os scripts podem fazer.

use glam::Vec2;
use mlua::{AnyUserData, Function, Lua, RegistryKey, Result, Table, Value, Variadic};
use nn_app::Context;
use nn_render::{Sprite, Text, TextureOptions};

use crate::convert::{align, anchor, color, err, keys, mouse_button, opt, opt_color};
use crate::types::{LAnimation, LAnimator, LFont, LSheet, LSoundData, LTexture, expect, play};

/// Chave do registro do Lua onde fica o contexto durante um callback.
pub(crate) const CTX_KEY: &str = "nn.ctx";

/// Pedido de troca de cena feito pelo script, aplicado depois do `update`.
#[derive(Debug)]
pub(crate) enum SceneRequest {
    Push(String, Option<RegistryKey>),
    Replace(String, Option<RegistryKey>),
    Switch(String, Option<RegistryKey>),
    Pop,
    Quit,
}

/// Dados compartilhados guardados no estado Lua.
#[derive(Debug, Default)]
pub(crate) struct Shared {
    pub request: Option<SceneRequest>,
}

/// Acessa o contexto da engine. Só funciona dentro dos callbacks da cena.
pub(crate) fn with_ctx<R>(lua: &Lua, f: impl FnOnce(&mut Context) -> Result<R>) -> Result<R> {
    let ud: Option<AnyUserData> = lua.named_registry_value(CTX_KEY)?;
    let Some(ud) = ud else {
        return Err(err(
            "as funções nn.* só podem ser usadas durante a execução da cena (enter, update, draw...)",
        ));
    };
    ud.borrow_mut_scoped::<Context, _>(f)?
}

fn request(lua: &Lua, r: SceneRequest) -> Result<()> {
    let mut shared = lua
        .app_data_mut::<Shared>()
        .ok_or_else(|| err("estado interno do script ausente"))?;
    shared.request = Some(r);
    Ok(())
}

fn args_key(lua: &Lua, args: Value) -> Result<Option<RegistryKey>> {
    if args.is_nil() {
        Ok(None)
    } else {
        lua.create_registry_value(args).map(Some)
    }
}

// ----------------------------------------------------------------------
// Desenho
// ----------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Space {
    World,
    Screen,
}

fn build_sprite(
    ctx: &Context,
    target: &Value,
    x: f32,
    y: f32,
    opts: Option<&Table>,
    space: Space,
) -> Result<Sprite> {
    let (mut sprite, natural) = match target {
        Value::UserData(ud) if ud.is::<LTexture>() => {
            let id = ud.borrow::<LTexture>()?.0;
            (Sprite::new(id), ctx.renderer.texture_size(id).as_vec2())
        }
        Value::UserData(ud) if ud.is::<LSheet>() => {
            let sheet = ud.borrow::<LSheet>()?.0;
            let frame = opt::<i64>(opts, "frame")?.unwrap_or(1);
            if frame < 1 || frame as u32 > sheet.frame_count() {
                return Err(err(format!(
                    "quadro {frame} não existe (a spritesheet tem {} quadros, de 1 a {})",
                    sheet.frame_count(),
                    sheet.frame_count()
                )));
            }
            (sheet.sprite(frame as u32 - 1), sheet.frame_size().as_vec2())
        }
        other => {
            return Err(err(format!(
                "esperava uma imagem ou spritesheet, recebeu {}",
                other.type_name()
            )));
        }
    };

    let scale = opt::<f32>(opts, "scale")?.unwrap_or(1.0);
    let mut size = Vec2::new(
        opt::<f32>(opts, "w")?.unwrap_or(natural.x * scale),
        opt::<f32>(opts, "h")?.unwrap_or(natural.y * scale),
    );
    if opt::<bool>(opts, "flip_x")?.unwrap_or(false) {
        size.x = -size.x;
    }
    if opt::<bool>(opts, "flip_y")?.unwrap_or(false) {
        size.y = -size.y;
    }
    let default_origin = match space {
        Space::World => 0.5,
        Space::Screen => 0.0,
    };
    let mut tint = opt_color(opts, "color")?.unwrap_or(nn_core::Color::WHITE);
    if let Some(a) = opt::<f32>(opts, "alpha")? {
        tint.a *= a;
    }
    sprite = sprite
        .at(Vec2::new(x, y))
        .size(size)
        .origin(Vec2::new(
            opt::<f32>(opts, "origin_x")?.unwrap_or(default_origin),
            opt::<f32>(opts, "origin_y")?.unwrap_or(default_origin),
        ))
        .rotation(opt::<f32>(opts, "rotation")?.unwrap_or(0.0))
        .color(tint);
    Ok(sprite)
}

fn build_text<'a>(content: &'a str, x: f32, y: f32, opts: Option<&Table>) -> Result<Text<'a>> {
    let mut text = Text::new(content).at(Vec2::new(x, y));
    if let Some(size) = opt::<f32>(opts, "size")? {
        text = text.size(size);
    }
    if let Some(c) = opt_color(opts, "color")? {
        text = text.color(c);
    }
    if let Some(a) = opt::<f32>(opts, "alpha")? {
        text.color.a *= a;
    }
    if let Some(name) = opt::<String>(opts, "anchor")? {
        text = text.anchor(anchor(&name)?);
    }
    if let Some(name) = opt::<String>(opts, "align")? {
        text = text.align(align(&name)?);
    }
    if let Some(font) = opt::<Value>(opts, "font")? {
        text = text.font(expect::<LFont>(&font, "uma fonte (nn.assets.font)")?.0);
    }
    Ok(text)
}

fn draw_table(lua: &Lua, space: Space) -> Result<Table> {
    let t = lua.create_table()?;

    t.set(
        "sprite",
        lua.create_function(
            move |lua, (target, x, y, opts): (Value, f32, f32, Option<Table>)| {
                with_ctx(lua, |c| {
                    let sprite = build_sprite(c, &target, x, y, opts.as_ref(), space)?;
                    match space {
                        Space::World => c.renderer.draw(&sprite),
                        Space::Screen => {
                            c.renderer.screen().draw(&sprite);
                        }
                    }
                    Ok(())
                })
            },
        )?,
    )?;

    t.set(
        "rect",
        lua.create_function(move |lua, (x, y, w, h, col): (f32, f32, f32, f32, Value)| {
            let col = color(&col)?;
            with_ctx(lua, |c| {
                match space {
                    Space::World => c.renderer.draw_rect(Vec2::new(x, y), Vec2::new(w, h), col),
                    Space::Screen => {
                        c.renderer
                            .screen()
                            .draw_rect(Vec2::new(x, y), Vec2::new(w, h), col);
                    }
                }
                Ok(())
            })
        })?,
    )?;

    t.set(
        "text",
        lua.create_function(
            move |lua, (content, x, y, opts): (Value, f32, f32, Option<Table>)| {
                let content = match &content {
                    Value::String(s) => s.to_str()?.to_string(),
                    Value::Nil => String::new(),
                    other => lua
                        .coerce_string(other.clone())?
                        .map(|s| s.to_string_lossy())
                        .unwrap_or_else(|| format!("{other:?}")),
                };
                let text = build_text(&content, x, y, opts.as_ref())?;
                with_ctx(lua, |c| {
                    match space {
                        Space::World => c.renderer.draw_text(&text),
                        Space::Screen => {
                            c.renderer.screen().draw_text(&text);
                        }
                    }
                    Ok(())
                })
            },
        )?,
    )?;

    t.set(
        "circle",
        lua.create_function(move |lua, (x, y, r, col): (f32, f32, f32, Value)| {
            let col = color(&col)?;
            with_ctx(lua, |c| {
                match space {
                    Space::World => c.renderer.draw_circle(Vec2::new(x, y), r, col),
                    Space::Screen => {
                        c.renderer.screen().draw_circle(Vec2::new(x, y), r, col);
                    }
                }
                Ok(())
            })
        })?,
    )?;

    Ok(t)
}

// ----------------------------------------------------------------------
// Instalação
// ----------------------------------------------------------------------

pub(crate) fn install(lua: &Lua) -> Result<()> {
    lua.set_app_data(Shared::default());
    let nn = lua.create_table()?;
    nn.set("version", env!("CARGO_PKG_VERSION"))?;

    nn.set("draw", draw_table(lua, Space::World)?)?;
    nn.set("screen", draw_table(lua, Space::Screen)?)?;

    nn.set(
        "text_size",
        lua.create_function(|lua, (content, opts): (String, Option<Table>)| {
            let text = build_text(&content, 0.0, 0.0, opts.as_ref())?;
            with_ctx(lua, |c| {
                let size = c.renderer.measure_text(&text);
                Ok((size.x, size.y))
            })
        })?,
    )?;

    // --- Tempo ---
    let time = lua.create_table()?;
    time.set(
        "delta",
        lua.create_function(|lua, ()| with_ctx(lua, |c| Ok(c.time.delta_secs())))?,
    )?;
    time.set(
        "elapsed",
        lua.create_function(|lua, ()| with_ctx(lua, |c| Ok(c.time.elapsed_secs())))?,
    )?;
    time.set(
        "fixed_delta",
        lua.create_function(|lua, ()| with_ctx(lua, |c| Ok(c.fixed_dt())))?,
    )?;
    time.set(
        "alpha",
        lua.create_function(|lua, ()| with_ctx(lua, |c| Ok(c.alpha())))?,
    )?;
    time.set(
        "frame",
        lua.create_function(|lua, ()| with_ctx(lua, |c| Ok(c.time.frame())))?,
    )?;
    nn.set("time", time)?;

    // --- Entrada ---
    let input = lua.create_table()?;
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
                let down = |ks: &[nn_input::KeyCode]| ks.iter().any(|k| c.input.key_down(*k));
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
                use nn_input::KeyCode::*;
                let v = c.input.direction(ArrowLeft, ArrowRight, ArrowDown, ArrowUp);
                Ok((v.x, v.y))
            })
        })?,
    )?;
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
    nn.set("input", input)?;

    // --- Câmera ---
    let camera = lua.create_table()?;
    camera.set(
        "position",
        lua.create_function(|lua, ()| {
            with_ctx(lua, |c| Ok((c.camera.position.x, c.camera.position.y)))
        })?,
    )?;
    camera.set(
        "set_position",
        lua.create_function(|lua, (x, y): (f32, f32)| {
            with_ctx(lua, |c| {
                c.camera.position = Vec2::new(x, y);
                Ok(())
            })
        })?,
    )?;
    camera.set(
        "zoom",
        lua.create_function(|lua, ()| with_ctx(lua, |c| Ok(c.camera.zoom)))?,
    )?;
    camera.set(
        "set_zoom",
        lua.create_function(|lua, z: f32| {
            if z <= 0.0 {
                return Err(err("o zoom precisa ser maior que zero"));
            }
            with_ctx(lua, |c| {
                c.camera.zoom = z;
                Ok(())
            })
        })?,
    )?;
    camera.set(
        "rotation",
        lua.create_function(|lua, ()| with_ctx(lua, |c| Ok(c.camera.rotation)))?,
    )?;
    camera.set(
        "set_rotation",
        lua.create_function(|lua, r: f32| {
            with_ctx(lua, |c| {
                c.camera.rotation = r;
                Ok(())
            })
        })?,
    )?;
    camera.set(
        "to_world",
        lua.create_function(|lua, (x, y): (f32, f32)| {
            with_ctx(lua, |c| {
                let p = c
                    .camera
                    .screen_to_world(Vec2::new(x, y), c.viewport().as_vec2());
                Ok((p.x, p.y))
            })
        })?,
    )?;
    camera.set(
        "to_screen",
        lua.create_function(|lua, (x, y): (f32, f32)| {
            with_ctx(lua, |c| {
                let p = c
                    .camera
                    .world_to_screen(Vec2::new(x, y), c.viewport().as_vec2());
                Ok((p.x, p.y))
            })
        })?,
    )?;
    nn.set("camera", camera)?;

    // --- Janela ---
    let window = lua.create_table()?;
    window.set(
        "size",
        lua.create_function(|lua, ()| {
            with_ctx(lua, |c| {
                let v = c.viewport();
                Ok((v.x, v.y))
            })
        })?,
    )?;
    window.set(
        "set_clear_color",
        lua.create_function(|lua, col: Value| {
            let col = color(&col)?;
            with_ctx(lua, |c| {
                c.clear_color = col;
                Ok(())
            })
        })?,
    )?;
    nn.set("window", window)?;

    // --- Assets ---
    let assets = lua.create_table()?;
    assets.set(
        "texture",
        lua.create_function(|lua, (path, opts): (String, Option<Table>)| {
            let options = texture_options(opts.as_ref())?;
            with_ctx(lua, |c| {
                c.load_texture(&path, options)
                    .map(LTexture)
                    .map_err(|e| err(e.to_string()))
            })
        })?,
    )?;
    assets.set(
        "sheet",
        lua.create_function(
            |lua, (source, fw, fh, opts): (Value, u32, u32, Option<Table>)| {
                let options = texture_options(opts.as_ref())?;
                let margin = opt::<u32>(opts.as_ref(), "margin")?.unwrap_or(0);
                let spacing = opt::<u32>(opts.as_ref(), "spacing")?.unwrap_or(0);
                if fw == 0 || fh == 0 {
                    return Err(err("o tamanho do quadro precisa ser maior que zero"));
                }
                with_ctx(lua, |c| {
                    let texture = match &source {
                        Value::String(path) => c
                            .load_texture(path.to_str()?.as_ref() as &str, options)
                            .map_err(|e| err(e.to_string()))?,
                        other => expect::<LTexture>(other, "um caminho ou uma imagem")?.0,
                    };
                    let size = c.renderer.texture_size(texture);
                    if fw > size.x || fh > size.y {
                        return Err(err(format!(
                            "quadro de {fw}x{fh} maior que a imagem ({}x{})",
                            size.x, size.y
                        )));
                    }
                    let sheet = c
                        .renderer
                        .sprite_sheet(texture, glam::UVec2::new(fw, fh))
                        .with_margin(margin)
                        .with_spacing(spacing);
                    Ok(LSheet(sheet))
                })
            },
        )?,
    )?;
    assets.set(
        "sound",
        lua.create_function(|lua, path: String| {
            with_ctx(lua, |c| {
                c.load_sound(&path)
                    .map(LSoundData)
                    .map_err(|e| err(e.to_string()))
            })
        })?,
    )?;
    assets.set(
        "font",
        lua.create_function(|lua, path: String| {
            with_ctx(lua, |c| {
                c.load_font(&path)
                    .map(LFont)
                    .map_err(|e| err(e.to_string()))
            })
        })?,
    )?;
    assets.set(
        "read",
        lua.create_function(|lua, path: String| {
            with_ctx(lua, |c| {
                c.assets
                    .read_to_string(&path)
                    .map_err(|e| err(e.to_string()))
            })
        })?,
    )?;
    nn.set("assets", assets)?;

    // --- Áudio ---
    let audio = lua.create_table()?;
    audio.set(
        "play",
        lua.create_function(|lua, (sound, opts): (Value, Option<Table>)| {
            let id = expect::<LSoundData>(&sound, "um som (nn.assets.sound)")?.0;
            play(lua, id, opts.as_ref())
        })?,
    )?;
    audio.set(
        "volume",
        lua.create_function(|lua, ()| with_ctx(lua, |c| Ok(c.audio.master_volume())))?,
    )?;
    audio.set(
        "set_volume",
        lua.create_function(|lua, v: f32| {
            with_ctx(lua, |c| {
                c.audio.set_master_volume(v);
                Ok(())
            })
        })?,
    )?;
    nn.set("audio", audio)?;

    // --- Animação ---
    nn.set(
        "animation",
        lua.create_function(|_, (frames, fps, mode): (Table, f32, Option<String>)| {
            LAnimation::from_lua(&frames, fps, mode.as_deref())
        })?,
    )?;
    nn.set(
        "animator",
        lua.create_function(|_, anim: AnyUserData| {
            let anim = anim.borrow::<LAnimation>()?;
            Ok(LAnimator(nn_render::AnimationPlayer::new(anim.0.clone())))
        })?,
    )?;
    nn.set(
        "frames",
        lua.create_function(|lua, (first, last): (i64, i64)| {
            let step = if last >= first { 1 } else { -1 };
            let t = lua.create_table()?;
            let mut i = first;
            loop {
                t.push(i)?;
                if i == last {
                    break;
                }
                i += step;
            }
            Ok(t)
        })?,
    )?;

    // --- Cenas ---
    let scene = lua.create_table()?;
    scene.set(
        "push",
        lua.create_function(|lua, (path, args): (String, Value)| {
            let key = args_key(lua, args)?;
            request(lua, SceneRequest::Push(path, key))
        })?,
    )?;
    scene.set(
        "replace",
        lua.create_function(|lua, (path, args): (String, Value)| {
            let key = args_key(lua, args)?;
            request(lua, SceneRequest::Replace(path, key))
        })?,
    )?;
    scene.set(
        "switch",
        lua.create_function(|lua, (path, args): (String, Value)| {
            let key = args_key(lua, args)?;
            request(lua, SceneRequest::Switch(path, key))
        })?,
    )?;
    scene.set(
        "pop",
        lua.create_function(|lua, ()| request(lua, SceneRequest::Pop))?,
    )?;
    nn.set("scene", scene)?;
    nn.set(
        "quit",
        lua.create_function(|lua, ()| request(lua, SceneRequest::Quit))?,
    )?;

    nn.set("physics", crate::physics::table(lua)?)?;
    nn.set("particles", crate::particles::constructor(lua)?)?;
    nn.set("tilemap", crate::tilemap::table(lua)?)?;

    lua.globals().set("nn", nn)?;

    // print vai para o log da engine.
    let print: Function = lua.create_function(|lua, args: Variadic<Value>| {
        let parts: Vec<String> = args
            .iter()
            .map(|v| match lua.coerce_string(v.clone()) {
                Ok(Some(s)) => s.to_string_lossy(),
                _ => match v {
                    Value::Nil => "nil".into(),
                    Value::Boolean(b) => b.to_string(),
                    other => format!("{}: {:p}", other.type_name(), other.to_pointer()),
                },
            })
            .collect();
        log::info!("[lua] {}", parts.join("\t"));
        Ok(())
    })?;
    lua.globals().set("print", print)?;
    Ok(())
}

fn texture_options(opts: Option<&Table>) -> Result<TextureOptions> {
    Ok(if opt::<bool>(opts, "pixel_art")?.unwrap_or(false) {
        TextureOptions::PIXEL_ART
    } else {
        TextureOptions::default()
    })
}
