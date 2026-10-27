//! A tabela global `nn`: tudo o que os scripts podem fazer.

use glam::Vec2;
use mlua::{AnyUserData, Function, Lua, RegistryKey, Result, Table, Value, Variadic};
use nn_app::Context;
use nn_render::{Sprite, Text, TextureOptions};

use crate::convert::{align, anchor, channel, color, err, opt, opt_color};
use crate::types::{LAnimation, LAnimator, LFont, LSheet, LSoundData, LTexture, expect, play};

/// Onde fica o contexto da engine durante um callback da cena: guardado nos
/// dados do estado Lua, para cada `nn.*` não precisar buscar o contexto no
/// registro do Lua pelo nome. Junto, o vigia do laço sem fim (no jogo com
/// janela).
#[derive(Debug, Default, Clone)]
pub(crate) struct CtxSlot(std::rc::Rc<CtxInner>);

#[derive(Debug, Default)]
pub(crate) struct CtxInner {
    ud: std::cell::RefCell<Option<AnyUserData>>,
    watch: std::cell::OnceCell<std::sync::Arc<crate::watchdog::Watchdog>>,
}

impl CtxSlot {
    fn of(lua: &Lua) -> Option<Self> {
        lua.app_data_ref::<CtxSlot>().map(|s| s.clone())
    }

    /// Liga o vigia do laço sem fim (uma vez).
    pub(crate) fn watch(lua: &Lua, watch: std::sync::Arc<crate::watchdog::Watchdog>) {
        if let Some(slot) = Self::of(lua) {
            let _ = slot.0.watch.set(watch);
        }
    }

    /// Deixa `ctx` disponível para as funções `nn.*` enquanto `f` roda.
    pub(crate) fn enter<R>(
        lua: &Lua,
        ctx: &mut Context,
        f: impl FnOnce() -> Result<R>,
    ) -> Result<R> {
        let Some(slot) = Self::of(lua) else {
            return f();
        };
        lua.scope(|scope| {
            let ud = scope.create_any_userdata_ref_mut(ctx)?;
            /// Volta ao contexto de antes (também se `f` entrar em pânico).
            struct Restore(CtxSlot, Option<AnyUserData>);
            impl Drop for Restore {
                fn drop(&mut self) {
                    *self.0.0.ud.borrow_mut() = self.1.take();
                    if let Some(w) = self.0.0.watch.get() {
                        w.leave_lua();
                    }
                }
            }
            if let Some(w) = slot.0.watch.get() {
                w.enter_lua();
            }
            let previous = slot.0.ud.replace(Some(ud));
            let _restore = Restore(slot.clone(), previous);
            f()
        })
    }
}

/// Liga os dados do contexto ao estado Lua (uma vez, ao criar).
pub(crate) fn install_ctx(lua: &Lua) {
    lua.set_app_data(CtxSlot::default());
}

/// Pedido de troca de cena feito pelo script, aplicado depois do `update`.
#[derive(Debug)]
pub(crate) enum SceneRequest {
    Push(String, Option<RegistryKey>),
    Replace(String, Option<RegistryKey>),
    Switch(String, Option<RegistryKey>),
    Pop,
    Quit,
}

/// A tela que aparece enquanto a cena nova carrega (`loading` nas opções
/// da troca de cena).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LoadingScreen {
    /// A tela padrão, com uma barra.
    Default,
    /// Uma cena do jogo (ela lê o andamento em `nn.scene.progress()`).
    Scene(String),
}

/// Uma troca de cena pedida pelo script.
#[derive(Debug)]
pub(crate) struct Request {
    pub scene: SceneRequest,
    pub fade: Option<nn_app::Fade>,
    pub loading: Option<LoadingScreen>,
}

/// Dados compartilhados guardados no estado Lua.
#[derive(Debug, Default)]
pub(crate) struct Shared {
    /// Troca de cena pedida (aplicada depois do `update`).
    pub request: Option<Request>,
    /// Quanto da cena nova já carregou (de 0 a 1), enquanto uma tela de
    /// carregamento aparece.
    pub progress: Option<f32>,
    /// O arquivo da cena da frente.
    pub current: String,
}

/// Acessa o contexto da engine. Só funciona dentro dos callbacks da cena.
pub(crate) fn with_ctx<R>(lua: &Lua, f: impl FnOnce(&mut Context) -> Result<R>) -> Result<R> {
    let slot = CtxSlot::of(lua);
    let ud = slot.as_ref().and_then(|s| s.0.ud.borrow().clone());
    let Some(ud) = ud else {
        return Err(err(
            "as funções nn.* só podem ser usadas durante a execução da cena (enter, update, draw...)",
        ));
    };
    // O vigia do laço sem fim: o tempo aqui dentro não conta, e um quadro
    // que passou do limite é cortado na primeira nn.* que o Lua chamar.
    let watch = slot.as_ref().and_then(|s| s.0.watch.get());
    if let Some(w) = watch
        && !w.enter_rust()
    {
        // Onde o Lua estava (quem chamou esta função): arquivo e linha.
        let at = lua
            .inspect_stack(1, |d| {
                let line = d.current_line()?;
                let src = d.source();
                let file = src.short_src.filter(|_| src.what != "C")?;
                Some(format!("{file}:{line}: "))
            })
            .flatten()
            .unwrap_or_default();
        return Err(err(format!("{at}{}", crate::watchdog::cut_message())));
    }
    let result = ud.borrow_mut_scoped::<Context, _>(f);
    if let Some(w) = watch {
        w.leave_rust();
    }
    result?
}

/// Soma o tempo de um passo de física no medidor do quadro (se houver um
/// jogo rodando; num teste sem jogo, nada).
pub(crate) fn count_physics(lua: &Lua, d: std::time::Duration) {
    let _ = with_ctx(lua, |ctx| {
        ctx.perf.add_physics(d);
        Ok(())
    });
}

fn request(lua: &Lua, r: SceneRequest, opts: Option<&Table>) -> Result<()> {
    let fade = fade_options(opts)?;
    let loading = match opt::<Value>(opts, "loading")? {
        None | Some(Value::Nil) | Some(Value::Boolean(false)) => None,
        Some(Value::Boolean(true)) => Some(LoadingScreen::Default),
        Some(Value::String(s)) => Some(LoadingScreen::Scene(s.to_str()?.to_string())),
        Some(other) => {
            return Err(err(format!(
                "loading: use true (a tela padrão) ou o caminho de uma cena, recebeu {}",
                other.type_name()
            )));
        }
    };
    let mut shared = lua
        .app_data_mut::<Shared>()
        .ok_or_else(|| err("estado interno do script ausente"))?;
    shared.request = Some(Request {
        scene: r,
        fade,
        loading,
    });
    Ok(())
}

/// Opções de troca de cena: `{ fade = segundos, color = cor }`.
fn fade_options(opts: Option<&Table>) -> Result<Option<nn_app::Fade>> {
    let duration = match opt::<Value>(opts, "fade")? {
        None | Some(Value::Nil) | Some(Value::Boolean(false)) => return Ok(None),
        Some(Value::Boolean(true)) => 0.5,
        Some(v) => match v.as_f64() {
            Some(s) if s >= 0.0 => s as f32,
            _ => {
                return Err(err(
                    "fade: use a duração em segundos (ex.: fade = 0.5) ou true",
                ));
            }
        },
    };
    let color = opt_color(opts, "color")?.unwrap_or(nn_core::Color::BLACK);
    Ok(Some(nn_app::Fade { duration, color }))
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
    // Repetir em vez de esticar: cada cópia tem `tile_w` × `tile_h`.
    let tile_w = opt::<f32>(opts, "tile_w")?.filter(|v| *v > 0.0);
    let tile_h = opt::<f32>(opts, "tile_h")?.filter(|v| *v > 0.0);
    let repeat = Vec2::new(
        tile_w.map_or(1.0, |t| size.x.abs() / t),
        tile_h.map_or(1.0, |t| size.y.abs() / t),
    );
    // Deslocar a imagem dentro do sprite (em pixels do desenho; uma
    // repetição é `tile_w`, ou o sprite inteiro): positivo anda para +x e +y
    // do espaço (na tela, y é para baixo).
    let tile = Vec2::new(
        tile_w.unwrap_or(size.x.abs()),
        tile_h.unwrap_or(size.y.abs()),
    )
    .max(Vec2::splat(1e-3));
    let shift = Vec2::new(
        opt::<f32>(opts, "offset_x")?.unwrap_or(0.0),
        opt::<f32>(opts, "offset_y")?.unwrap_or(0.0),
    );
    let offset = Vec2::new(
        -shift.x / tile.x,
        match space {
            Space::World => shift.y / tile.y,
            Space::Screen => -shift.y / tile.y,
        },
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
        .color(tint)
        .repeat(repeat)
        .offset(offset);
    Ok(sprite)
}

pub(crate) fn build_text<'a>(
    content: &'a str,
    x: f32,
    y: f32,
    opts: Option<&Table>,
) -> Result<Text<'a>> {
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

/// Argumentos de [`component_sprite`], na ordem.
type ComponentSprite = (
    bool,
    Value,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    f32,
    Value,
    Option<bool>,
    Option<bool>,
    Option<bool>,
    Option<i64>,
);

/// O componente Imagem da cena visual, sem a tabela de opções (é o que mais
/// se desenha): `nn._sprite(hud, img, x, y, sx, sy, w, h, rotação em graus,
/// cor, flip_x, flip_y, repetir, quadro)`. `w`/`h` 0 é o tamanho natural; a
/// escala do objeto multiplica; repetir põe uma cópia por unidade de escala.
fn component_sprite(
    lua: &Lua,
    (hud, target, x, y, sx, sy, w, h, rot, col, flip_x, flip_y, repeat, frame): ComponentSprite,
) -> Result<()> {
    let tint = if col.is_nil() {
        nn_core::Color::WHITE
    } else {
        color(&col)?
    };
    with_ctx(lua, |c| {
        let (sprite, natural) = match &target {
            Value::UserData(ud) if ud.is::<LTexture>() => {
                let id = ud.borrow::<LTexture>()?.0;
                (Sprite::new(id), c.renderer.texture_size(id).as_vec2())
            }
            Value::UserData(ud) if ud.is::<LSheet>() => {
                let sheet = ud.borrow::<LSheet>()?.0;
                let count = sheet.frame_count().max(1);
                let f = frame.unwrap_or(1).clamp(1, count as i64) as u32;
                (sheet.sprite(f - 1), sheet.frame_size().as_vec2())
            }
            other => {
                return Err(err(format!(
                    "esperava uma imagem ou spritesheet, recebeu {}",
                    other.type_name()
                )));
            }
        };
        let base = Vec2::new(
            if w > 0.0 { w } else { natural.x },
            if h > 0.0 { h } else { natural.y },
        );
        let mut size = base * Vec2::new(sx, sy);
        let repeat = if repeat.unwrap_or(false) {
            Vec2::new(sx.abs(), sy.abs())
        } else {
            Vec2::ONE
        };
        if flip_x.unwrap_or(false) {
            size.x = -size.x;
        }
        if flip_y.unwrap_or(false) {
            size.y = -size.y;
        }
        // Na tela, y é para baixo: a rotação vira ao contrário.
        let rotation = if hud { -rot } else { rot }.to_radians();
        let sprite = sprite
            .at(Vec2::new(x, y))
            .size(size)
            .origin(Vec2::splat(0.5))
            .rotation(rotation)
            .color(tint)
            .repeat(repeat);
        if hud {
            c.renderer.screen().draw(&sprite);
        } else {
            c.renderer.draw(&sprite);
        }
        Ok(())
    })
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
        "panel",
        lua.create_function(
            move |lua, (x, y, w, h, opts): (f32, f32, f32, f32, Option<Table>)| {
                let o = opts.as_ref();
                with_ctx(lua, |c| {
                    // No mundo, x, y é o centro; na tela, o canto de cima à
                    // esquerda (como nn.screen.rect).
                    let center = match space {
                        Space::World => Vec2::new(x, y),
                        Space::Screen => Vec2::new(x + w / 2.0, y + h / 2.0),
                    };
                    let mut panel = nn_render::Panel::new(center, Vec2::new(w, h))
                        .color(opt_color(o, "color")?.unwrap_or(nn_core::Color::WHITE))
                        .radius(opt::<f32>(o, "radius")?.unwrap_or(0.0));
                    if let Some(b) = opt::<f32>(o, "border")? {
                        let bc = opt_color(o, "border_color")?.unwrap_or(nn_core::Color::WHITE);
                        panel = panel.border(b, bc);
                    }
                    if let Some(img) = opt::<Value>(o, "image")? {
                        let (texture, region) = match &img {
                            Value::UserData(ud) if ud.is::<LTexture>() => {
                                (ud.borrow::<LTexture>()?.0, nn_render::Rect::FULL)
                            }
                            Value::UserData(ud) if ud.is::<LSheet>() => {
                                let sheet = ud.borrow::<LSheet>()?.0;
                                let frame = opt::<i64>(o, "frame")?.unwrap_or(1).max(1) as u32;
                                let s = sheet
                                    .sprite((frame - 1).min(sheet.frame_count().saturating_sub(1)));
                                (s.texture, s.region)
                            }
                            other => {
                                return Err(err(format!(
                                    "image: esperava uma imagem ou spritesheet, recebeu {}",
                                    other.type_name()
                                )));
                            }
                        };
                        panel = panel.image(nn_render::PanelImage {
                            texture,
                            region,
                            slice: opt::<f32>(o, "slice")?.unwrap_or(4.0),
                            scale: opt::<f32>(o, "slice_scale")?.unwrap_or(1.0),
                        });
                    }
                    match space {
                        Space::World => c.renderer.draw_panel(&panel),
                        Space::Screen => {
                            c.renderer.screen().draw_panel(&panel);
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
    install_ctx(lua);
    let nn = lua.create_table()?;
    nn.set("version", env!("CARGO_PKG_VERSION"))?;

    let draw = draw_table(lua, Space::World)?;
    draw.set(
        "parallax",
        lua.create_function(|lua, (img, opts): (Value, Option<Table>)| {
            let texture = expect::<LTexture>(&img, "uma imagem (nn.assets.texture)")?.0;
            let o = opts.as_ref();
            let scale = opt::<f32>(o, "scale")?.unwrap_or(1.0);
            let f = opt::<f32>(o, "factor")?.unwrap_or(0.5);
            let factor = Vec2::new(
                opt::<f32>(o, "factor_x")?.unwrap_or(f),
                opt::<f32>(o, "factor_y")?.unwrap_or(f),
            );
            let origin = Vec2::new(
                opt::<f32>(o, "x")?.unwrap_or(0.0),
                opt::<f32>(o, "y")?.unwrap_or(0.0),
            );
            let speed = Vec2::new(
                opt::<f32>(o, "scroll_x")?.unwrap_or(0.0),
                opt::<f32>(o, "scroll_y")?.unwrap_or(0.0),
            );
            let repeat_x = opt::<bool>(o, "repeat_x")?.unwrap_or(true);
            let repeat_y = opt::<bool>(o, "repeat_y")?.unwrap_or(false);
            let mut color = opt_color(o, "color")?.unwrap_or(nn_core::Color::WHITE);
            if let Some(a) = opt::<f32>(o, "alpha")? {
                color.a *= a;
            }
            let w = opt::<f32>(o, "w")?;
            let h = opt::<f32>(o, "h")?;
            with_ctx(lua, |c| {
                let natural = c.renderer.texture_size(texture).as_vec2();
                let size = Vec2::new(
                    w.unwrap_or(natural.x * scale),
                    h.unwrap_or(natural.y * scale),
                );
                let layer = nn_render::Parallax::new(texture, size)
                    .factor(factor)
                    .at(origin)
                    .scroll(speed * c.time.elapsed_secs())
                    .repeat(repeat_x, repeat_y)
                    .color(color);
                let camera = c.camera;
                let viewport = c.viewport().as_vec2();
                layer.draw(&mut c.renderer, &camera, viewport);
                Ok(())
            })
        })?,
    )?;
    nn.set("draw", draw)?;
    let screen = draw_table(lua, Space::Screen)?;
    screen.set(
        "flash",
        lua.create_function(|lua, (col, seconds): (Value, Option<f32>)| {
            let col = color(&col)?;
            with_ctx(lua, |c| {
                c.flash(col, seconds.unwrap_or(0.3));
                Ok(())
            })
        })?,
    )?;
    nn.set("screen", screen)?;
    nn.set("_sprite", lua.create_function(component_sprite)?)?;

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

    // --- Desempenho (o mesmo da tela F3) ---
    let debug = lua.create_table()?;
    debug.set(
        "stats",
        lua.create_function(|lua, ()| {
            with_ctx(lua, |c| {
                let t = lua.create_table()?;
                let a = c.perf.average;
                let r = c.renderer.last_stats();
                t.set("fps", c.perf.fps())?;
                t.set("frame_ms", a.frame)?;
                t.set("worst_ms", c.perf.worst_frame)?;
                t.set("fixed_ms", a.fixed)?;
                t.set("physics_ms", a.physics)?;
                t.set("update_ms", a.update)?;
                t.set("draw_ms", a.draw)?;
                t.set("sprites", r.sprites)?;
                t.set("draw_calls", r.draw_calls)?;
                t.set("textures", c.renderer.texture_count())?;
                t.set(
                    "texture_mb",
                    c.renderer.texture_bytes() as f64 / (1024.0 * 1024.0),
                )?;
                t.set("lua_mb", c.perf.lua_kb / 1024.0)?;
                t.set("objects", c.perf.objects)?;
                t.set("loads", c.perf.last.loads)?;
                t.set("loaded", c.perf.last_loaded.clone())?;
                t.set("total_loads", c.perf.total_loads)?;
                t.set("sounds", c.audio.sound_count())?;
                t.set("meshes", c.renderer.mesh_count())?;
                Ok(t)
            })
        })?,
    )?;
    debug.set(
        "overlay",
        lua.create_function(|lua, on: Option<bool>| {
            with_ctx(lua, |c| {
                if let Some(on) = on {
                    c.perf.overlay = on;
                }
                Ok(c.perf.overlay)
            })
        })?,
    )?;
    nn.set("debug", debug)?;

    nn.set("input", crate::input::table(lua)?)?;

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
        "follow",
        lua.create_function(|lua, (x, y, opts): (f32, f32, Option<Table>)| {
            let o = opts.as_ref();
            let mut follow = nn_render::Follow::default();
            if let Some(s) = opt::<f32>(o, "smooth")? {
                follow.smoothing = s.max(0.0);
            }
            follow.deadzone = Vec2::new(
                opt::<f32>(o, "deadzone_w")?.unwrap_or(0.0),
                opt::<f32>(o, "deadzone_h")?.unwrap_or(0.0),
            );
            follow.offset = Vec2::new(
                opt::<f32>(o, "offset_x")?.unwrap_or(0.0),
                opt::<f32>(o, "offset_y")?.unwrap_or(0.0),
            );
            if let Some(b) = opt::<Value>(o, "bounds")?.filter(|v| !v.is_nil()) {
                follow.bounds = Some(bounds(&b)?);
            }
            with_ctx(lua, |c| {
                let dt = c.time.delta_secs();
                let viewport = c.viewport().as_vec2();
                c.camera.follow(Vec2::new(x, y), &follow, dt, viewport);
                Ok(())
            })
        })?,
    )?;
    camera.set(
        "clamp",
        lua.create_function(|lua, b: Value| {
            let b = bounds(&b)?;
            with_ctx(lua, |c| {
                let viewport = c.viewport().as_vec2();
                c.camera.clamp_to(b, viewport);
                Ok(())
            })
        })?,
    )?;
    camera.set(
        "view",
        lua.create_function(|lua, ()| {
            with_ctx(lua, |c| {
                let b = c.camera.view_bounds(c.viewport().as_vec2());
                Ok((b.min.x, b.min.y, b.max.x, b.max.y))
            })
        })?,
    )?;
    camera.set(
        "shake",
        lua.create_function(|lua, (strength, seconds): (f32, Option<f32>)| {
            with_ctx(lua, |c| {
                c.shake(strength, seconds.unwrap_or(0.3));
                Ok(())
            })
        })?,
    )?;
    camera.set(
        "stop_shake",
        lua.create_function(|lua, ()| {
            with_ctx(lua, |c| {
                c.stop_shake();
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
        "title",
        lua.create_function(|lua, ()| with_ctx(lua, |c| Ok(c.title().to_string())))?,
    )?;
    window.set(
        "capture_mouse",
        lua.create_function(|lua, on: Option<bool>| {
            with_ctx(lua, |c| {
                c.set_mouse_captured(on.unwrap_or(true));
                Ok(())
            })
        })?,
    )?;
    window.set(
        "mouse_captured",
        lua.create_function(|lua, ()| with_ctx(lua, |c| Ok(c.mouse_captured())))?,
    )?;
    window.set(
        "fullscreen",
        lua.create_function(|lua, ()| with_ctx(lua, |c| Ok(c.is_fullscreen())))?,
    )?;
    window.set(
        "set_fullscreen",
        lua.create_function(|lua, on: bool| {
            with_ctx(lua, |c| {
                c.set_fullscreen(on);
                Ok(())
            })
        })?,
    )?;
    window.set(
        "toggle_fullscreen",
        lua.create_function(|lua, ()| {
            with_ctx(lua, |c| {
                let on = !c.is_fullscreen();
                c.set_fullscreen(on);
                Ok(on)
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
                c.texture_handle(&path, options)
                    .map(|(id, h)| LTexture(id, Some(h)))
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
                    let (texture, handle) = match &source {
                        Value::String(path) => {
                            let (id, h) = c
                                .texture_handle(path.to_str()?.as_ref() as &str, options)
                                .map_err(|e| err(e.to_string()))?;
                            (id, Some(h))
                        }
                        other => {
                            let t = expect::<LTexture>(other, "um caminho ou uma imagem")?;
                            (t.0, t.1.clone())
                        }
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
                    Ok(LSheet(sheet, handle))
                })
            },
        )?,
    )?;
    assets.set(
        "sound",
        lua.create_function(|lua, path: String| {
            with_ctx(lua, |c| {
                c.sound_handle(&path)
                    .map(|(id, h)| LSoundData(id, Some(h)))
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
        "model",
        lua.create_function(|lua, (path, opts): (String, Option<Table>)| {
            crate::d3::load_model(lua, &path, opts.as_ref())
        })?,
    )?;
    assets.set(
        "keep",
        lua.create_function(|lua, v: Value| {
            let Value::UserData(ud) = &v else {
                return Err(err(format!(
                    "nn.assets.keep: esperava uma imagem, spritesheet, som, modelo ou malha, recebeu {}",
                    v.type_name()
                )));
            };
            if let Ok(m) = ud.borrow::<crate::d3::LMesh>() {
                m.keep_forever(lua);
                return Ok(v.clone());
            }
            with_ctx(lua, |c| {
                if let Ok(t) = ud.borrow::<LTexture>() {
                    c.keep_texture(t.0);
                } else if let Ok(s) = ud.borrow::<LSheet>() {
                    c.keep_texture(s.0.texture());
                } else if let Ok(s) = ud.borrow::<LSoundData>() {
                    c.keep_sound(s.0);
                } else if let Ok(m) = ud.borrow::<crate::d3::LModel>() {
                    c.keep_model(m.key());
                } else {
                    return Err(err(
                        "nn.assets.keep: esperava uma imagem, spritesheet, som, modelo ou malha",
                    ));
                }
                Ok(())
            })?;
            Ok(v.clone())
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
    assets.set(
        "exists",
        lua.create_function(|lua, path: String| with_ctx(lua, |c| Ok(c.assets.exists(&path))))?,
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
        lua.create_function(|lua, ch: Option<String>| {
            let ch = ch.as_deref().map(channel).transpose()?;
            with_ctx(lua, |c| {
                Ok(match ch {
                    Some(ch) => c.audio.channel_volume(ch),
                    None => c.audio.master_volume(),
                })
            })
        })?,
    )?;
    audio.set(
        "set_volume",
        lua.create_function(|lua, (v, ch): (f32, Option<String>)| {
            let ch = ch.as_deref().map(channel).transpose()?;
            with_ctx(lua, |c| {
                match ch {
                    Some(ch) => c.audio.set_channel_volume(ch, v),
                    None => c.audio.set_master_volume(v),
                }
                Ok(())
            })
        })?,
    )?;
    nn.set("audio", audio)?;

    // --- Animação ---
    nn.set(
        "animation",
        lua.create_function(
            |_, (frames, fps, mode, times): (Table, f32, Option<String>, Option<Table>)| {
                LAnimation::from_lua(&frames, fps, mode.as_deref(), times.as_ref())
            },
        )?,
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
        lua.create_function(|lua, (path, args, opts): (String, Value, Option<Table>)| {
            let key = args_key(lua, args)?;
            request(lua, SceneRequest::Push(path, key), opts.as_ref())
        })?,
    )?;
    scene.set(
        "replace",
        lua.create_function(|lua, (path, args, opts): (String, Value, Option<Table>)| {
            let key = args_key(lua, args)?;
            request(lua, SceneRequest::Replace(path, key), opts.as_ref())
        })?,
    )?;
    scene.set(
        "switch",
        lua.create_function(|lua, (path, args, opts): (String, Value, Option<Table>)| {
            let key = args_key(lua, args)?;
            request(lua, SceneRequest::Switch(path, key), opts.as_ref())
        })?,
    )?;
    scene.set(
        "pop",
        lua.create_function(|lua, opts: Option<Table>| {
            request(lua, SceneRequest::Pop, opts.as_ref())
        })?,
    )?;
    scene.set(
        "current",
        lua.create_function(|lua, ()| {
            Ok(lua
                .app_data_ref::<Shared>()
                .map(|s| s.current.clone())
                .filter(|c| !c.is_empty()))
        })?,
    )?;
    scene.set(
        "progress",
        lua.create_function(|lua, ()| Ok(lua.app_data_ref::<Shared>().and_then(|s| s.progress)))?,
    )?;
    nn.set("scene", scene)?;
    nn.set(
        "quit",
        lua.create_function(|lua, opts: Option<Table>| {
            request(lua, SceneRequest::Quit, opts.as_ref())
        })?,
    )?;

    nn.set("physics", crate::physics::table(lua)?)?;
    nn.set("physics3d", crate::physics3d::table(lua)?)?;
    nn.set("particles", crate::particles::constructor(lua)?)?;
    nn.set("particles3d", crate::particles::constructor3d(lua)?)?;
    nn.set("d3", crate::d3::table(lua)?)?;
    nn.set("tilemap", crate::tilemap::table(lua)?)?;
    nn.set("ui", crate::ui::table(lua)?)?;
    nn.set("save", crate::save::table(lua)?)?;

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
                    // Um erro pego com pcall: a mensagem, em português.
                    Value::Error(e) => crate::translate_error(&e.to_string()),
                    // Como o tostring (que respeita o __tostring da tabela).
                    other => lua
                        .globals()
                        .get::<Function>("tostring")
                        .and_then(|f| f.call::<String>(other.clone()))
                        .unwrap_or_else(|_| {
                            format!("{}: {:p}", other.type_name(), other.to_pointer())
                        }),
                },
            })
            .collect();
        log::info!("[lua] {}", parts.join("\t"));
        Ok(())
    })?;
    lua.globals().set("print", print)?;
    // Avisos da engine (um arquivo que falta...): o jogo continua, e o NN
    // Studio mostra no Console como aviso.
    let nn: Table = lua.globals().get("nn")?;
    nn.set(
        "_warn",
        lua.create_function(|_, msg: String| {
            log::warn!("{msg}");
            Ok(())
        })?,
    )?;
    // Mostrar os caminhos dos inimigos (botão Caminhos do NN Studio, ou
    // NN_CAMINHOS=1 ao abrir o jogo).
    nn.set(
        "_show_paths",
        lua.create_function(|_, ()| Ok(crate::live::show_paths()))?,
    )?;
    Ok(())
}

/// Limite da câmera: um mapa (nn.tilemap) ou `{ x =, y =, w =, h = }`
/// (canto inferior esquerdo e tamanho).
fn bounds(v: &Value) -> Result<nn_render::Bounds> {
    match v {
        Value::UserData(ud) if ud.is::<crate::tilemap::LTileMap>() => {
            Ok(ud.borrow::<crate::tilemap::LTileMap>()?.bounds())
        }
        Value::Table(t) => {
            let get = |k: &str| -> Result<f32> {
                t.get::<Option<f32>>(k)?.ok_or_else(|| {
                    err(format!(
                        "limite sem \"{k}\": use {{ x =, y =, w =, h = }} ou um mapa"
                    ))
                })
            };
            Ok(nn_render::Bounds::from_size(
                Vec2::new(get("x")?, get("y")?),
                Vec2::new(get("w")?, get("h")?),
            ))
        }
        other => Err(err(format!(
            "limite da câmera inválido: use um mapa (nn.tilemap) ou {{ x =, y =, w =, h = }}, recebeu {}",
            other.type_name()
        ))),
    }
}

fn texture_options(opts: Option<&Table>) -> Result<TextureOptions> {
    Ok(image_options(
        opt::<bool>(opts, "pixel_art")?.unwrap_or(false),
        opt::<bool>(opts, "mipmaps")?.unwrap_or(true),
    ))
}

/// As opções de uma imagem de jogo em Lua: pixel art nítida, ou lisa com
/// mipmaps (desenhada menor, não serrilha), que `mipmaps = false` tira.
pub(crate) fn image_options(pixel_art: bool, mipmaps: bool) -> TextureOptions {
    if pixel_art {
        TextureOptions::PIXEL_ART
    } else if mipmaps {
        TextureOptions::SMOOTH
    } else {
        TextureOptions::default()
    }
}
