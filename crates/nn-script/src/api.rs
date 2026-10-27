//! A tabela global `nn`: tudo o que os scripts podem fazer.

use glam::Vec2;
use mlua::{AnyUserData, Function, Lua, RegistryKey, Result, Table, Value, Variadic};
use nn_app::Context;
use nn_render::{Sprite, Text, TextureOptions};

use crate::convert::{align, anchor, channel, color, err, opt, opt_color};
use crate::types::{
    LAnimation, LAnimator, LEffect, LFont, LSheet, LSoundData, LTexture, expect, play,
};

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

/// As opções de `nn.draw.sprite`: da tabela, ou do jeito curto
/// `(img, x, y, quadro, escala, cor)` (sem criar uma tabela a cada quadro).
#[derive(Default)]
struct SpriteOpts {
    frame: Option<i64>,
    scale: Option<f32>,
    w: Option<f32>,
    h: Option<f32>,
    tile_w: Option<f32>,
    tile_h: Option<f32>,
    offset_x: Option<f32>,
    offset_y: Option<f32>,
    flip_x: bool,
    flip_y: bool,
    color: Option<nn_core::Color>,
    alpha: Option<f32>,
    origin_x: Option<f32>,
    origin_y: Option<f32>,
    rotation: Option<f32>,
    effect: Option<nn_render::effects::EffectId>,
}

impl SpriteOpts {
    fn from_table(o: Option<&Table>) -> Result<Self> {
        Ok(Self {
            frame: opt(o, "frame")?,
            scale: opt(o, "scale")?,
            w: opt(o, "w")?,
            h: opt(o, "h")?,
            tile_w: opt(o, "tile_w")?,
            tile_h: opt(o, "tile_h")?,
            offset_x: opt(o, "offset_x")?,
            offset_y: opt(o, "offset_y")?,
            flip_x: opt::<bool>(o, "flip_x")?.unwrap_or(false),
            flip_y: opt::<bool>(o, "flip_y")?.unwrap_or(false),
            color: opt_color(o, "color")?,
            alpha: opt(o, "alpha")?,
            origin_x: opt(o, "origin_x")?,
            origin_y: opt(o, "origin_y")?,
            rotation: opt(o, "rotation")?,
            effect: match opt::<Value>(o, "effect")? {
                None => None,
                Some(v) => Some(expect::<LEffect>(&v, "um efeito (nn.effect)")?.0),
            },
        })
    }

    /// O 4º argumento em diante: uma tabela de opções, ou quadro, escala e
    /// cor.
    fn from_args(args: &[Value]) -> Result<Self> {
        match args.first() {
            Some(Value::Table(t)) => Self::from_table(Some(t)),
            _ => {
                let num = |i: usize, name: &str| -> Result<Option<f64>> {
                    match args.get(i) {
                        None | Some(Value::Nil) => Ok(None),
                        Some(Value::Integer(n)) => Ok(Some(*n as f64)),
                        Some(Value::Number(n)) => Ok(Some(*n)),
                        Some(other) => Err(err(format!(
                            "{name}: esperava um número, recebeu {}",
                            other.type_name()
                        ))),
                    }
                };
                Ok(Self {
                    frame: num(0, "quadro (4º argumento)")?.map(|f| f as i64),
                    scale: num(1, "escala (5º argumento)")?.map(|f| f as f32),
                    color: match args.get(2) {
                        None | Some(Value::Nil) => None,
                        Some(v) => Some(color(v)?),
                    },
                    ..Self::default()
                })
            }
        }
    }
}

fn build_sprite(
    ctx: &Context,
    target: &Value,
    x: f32,
    y: f32,
    o: &SpriteOpts,
    space: Space,
) -> Result<Sprite> {
    let (mut sprite, natural) = match target {
        Value::UserData(ud) if ud.is::<LTexture>() => {
            let id = ud.borrow::<LTexture>()?.0;
            (Sprite::new(id), ctx.renderer.texture_size(id).as_vec2())
        }
        Value::UserData(ud) if ud.is::<LSheet>() => {
            let sheet = ud.borrow::<LSheet>()?.0;
            let frame = o.frame.unwrap_or(1);
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

    let scale = o.scale.unwrap_or(1.0);
    let mut size = Vec2::new(
        o.w.unwrap_or(natural.x * scale),
        o.h.unwrap_or(natural.y * scale),
    );
    // Repetir em vez de esticar: cada cópia tem `tile_w` × `tile_h`.
    let tile_w = o.tile_w.filter(|v| *v > 0.0);
    let tile_h = o.tile_h.filter(|v| *v > 0.0);
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
    let shift = Vec2::new(o.offset_x.unwrap_or(0.0), o.offset_y.unwrap_or(0.0));
    let offset = Vec2::new(
        -shift.x / tile.x,
        match space {
            Space::World => shift.y / tile.y,
            Space::Screen => -shift.y / tile.y,
        },
    );
    if o.flip_x {
        size.x = -size.x;
    }
    if o.flip_y {
        size.y = -size.y;
    }
    let default_origin = match space {
        Space::World => 0.5,
        Space::Screen => 0.0,
    };
    let mut tint = o.color.unwrap_or(nn_core::Color::WHITE);
    if let Some(a) = o.alpha {
        tint.a *= a;
    }
    sprite = sprite
        .at(Vec2::new(x, y))
        .size(size)
        .origin(Vec2::new(
            o.origin_x.unwrap_or(default_origin),
            o.origin_y.unwrap_or(default_origin),
        ))
        .rotation(o.rotation.unwrap_or(0.0))
        .color(tint)
        .repeat(repeat)
        .offset(offset);
    if let Some(e) = o.effect {
        sprite = sprite.effect(e);
    }
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
    match opt::<Value>(opts, "font")? {
        None => {}
        // A fonte de pixel art embutida.
        Some(Value::String(s)) if s.to_str().is_ok_and(|s| &*s == "pixel") => {
            text = text.font(nn_render::FontId::PIXEL);
        }
        Some(font) => {
            text = text.font(expect::<LFont>(&font, "uma fonte (nn.assets.font, ou \"pixel\")")?.0);
        }
    }
    if let Some(w) = opt::<f32>(opts, "width")? {
        text = text.width(w);
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
    Option<mlua::AnyUserData>,
);

/// O componente Imagem da cena visual, sem a tabela de opções (é o que mais
/// se desenha): `nn._sprite(hud, img, x, y, sx, sy, w, h, rotação em graus,
/// cor, flip_x, flip_y, repetir, quadro)`. `w`/`h` 0 é o tamanho natural; a
/// escala do objeto multiplica; repetir põe uma cópia por unidade de escala.
fn component_sprite(
    lua: &Lua,
    (hud, target, x, y, sx, sy, w, h, rot, col, flip_x, flip_y, repeat, frame, effect): ComponentSprite,
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
        let mut sprite = sprite
            .at(Vec2::new(x, y))
            .size(size)
            .origin(Vec2::splat(0.5))
            .rotation(rotation)
            .color(tint)
            .repeat(repeat);
        if let Some(e) = &effect
            && let Ok(e) = e.borrow::<LEffect>()
        {
            sprite = sprite.effect(e.0);
        }
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
            move |lua, (target, x, y, a, b, c): (Value, f32, f32, Value, Value, Value)| {
                let o = SpriteOpts::from_args(&[a, b, c])?;
                with_ctx(lua, |c| {
                    let sprite = build_sprite(c, &target, x, y, &o, space)?;
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
        lua.create_function(
            move |lua, (x, y, w, h, col, line): (f32, f32, f32, f32, Value, Option<f32>)| {
                let col = color(&col)?;
                let (at, size) = (Vec2::new(x, y), Vec2::new(w, h));
                with_ctx(lua, |c| {
                    // Um número no fim: só o contorno, com essa espessura.
                    match (space, line) {
                        (Space::World, None) => c.renderer.draw_rect(at, size, col),
                        (Space::World, Some(l)) => c.renderer.draw_rect_outline(at, size, l, col),
                        (Space::Screen, None) => {
                            c.renderer.screen().draw_rect(at, size, col);
                        }
                        (Space::Screen, Some(l)) => {
                            c.renderer.screen().draw_rect_outline(at, size, l, col);
                        }
                    }
                    Ok(())
                })
            },
        )?,
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
                // Contorno: o texto na cor dele, em volta (lê bem em cima de
                // qualquer fundo), como o do componente Texto.
                let outline = opt_color(opts.as_ref(), "outline")?;
                with_ctx(lua, |c| {
                    let mut draw = |t: &nn_render::Text<'_>| match space {
                        Space::World => c.renderer.draw_text(t),
                        Space::Screen => {
                            c.renderer.screen().draw_text(t);
                        }
                    };
                    if let Some(col) = outline {
                        let d = (text.size / 16.0).max(1.0);
                        let mut o = text;
                        o.color = col.with_alpha(col.a * text.color.a);
                        for (dx, dy) in [
                            (-1.0, -1.0),
                            (0.0, -1.0),
                            (1.0, -1.0),
                            (-1.0, 0.0),
                            (1.0, 0.0),
                            (-1.0, 1.0),
                            (0.0, 1.0),
                            (1.0, 1.0),
                        ] {
                            o.position = text.position + Vec2::new(dx, dy) * d;
                            draw(&o);
                        }
                    }
                    draw(&text);
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
        lua.create_function(
            move |lua, (x, y, r, col, line): (f32, f32, f32, Value, Option<f32>)| {
                let col = color(&col)?;
                let at = Vec2::new(x, y);
                with_ctx(lua, |c| {
                    match (space, line) {
                        (Space::World, None) => c.renderer.draw_circle(at, r, col),
                        (Space::World, Some(l)) => c.renderer.draw_circle_outline(at, r, l, col),
                        (Space::Screen, None) => {
                            c.renderer.screen().draw_circle(at, r, col);
                        }
                        (Space::Screen, Some(l)) => {
                            c.renderer.screen().draw_circle_outline(at, r, l, col);
                        }
                    }
                    Ok(())
                })
            },
        )?,
    )?;

    t.set(
        "clip",
        lua.create_function(
            move |lua, (x, y, w, h, f): (f32, f32, f32, f32, mlua::Function)| {
                // No mundo, x, y é o centro; na tela, o canto de cima à
                // esquerda (como o rect).
                let (a, b) = match space {
                    Space::World => (
                        Vec2::new(x - w.abs() / 2.0, y - h.abs() / 2.0),
                        Vec2::new(x + w.abs() / 2.0, y + h.abs() / 2.0),
                    ),
                    Space::Screen => (Vec2::new(x, y), Vec2::new(x + w, y + h)),
                };
                let rect = nn_render::ClipRect::new(a, b);
                let before = with_ctx(lua, |c| {
                    let before = match space {
                        Space::World => c.renderer.clip(),
                        Space::Screen => c.renderer.screen().clip(),
                    };
                    // Um recorte dentro do outro: só o pedaço em comum.
                    let clip = Some(before.map_or(rect, |b| b.intersect(rect)));
                    match space {
                        Space::World => c.renderer.set_clip(clip),
                        Space::Screen => {
                            c.renderer.screen().set_clip(clip);
                        }
                    }
                    Ok(before)
                })?;
                let result = f.call::<mlua::MultiValue>(());
                with_ctx(lua, |c| {
                    match space {
                        Space::World => c.renderer.set_clip(before),
                        Space::Screen => {
                            c.renderer.screen().set_clip(before);
                        }
                    }
                    Ok(())
                })?;
                result
            },
        )?,
    )?;

    t.set(
        "line",
        lua.create_function(
            move |lua, (x1, y1, x2, y2, col, width): (f32, f32, f32, f32, Value, Option<f32>)| {
                let col = color(&col)?;
                let (a, b, w) = (Vec2::new(x1, y1), Vec2::new(x2, y2), width.unwrap_or(1.0));
                with_ctx(lua, |c| {
                    match space {
                        Space::World => c.renderer.draw_line(a, b, w, col),
                        Space::Screen => {
                            c.renderer.screen().draw_line(a, b, w, col);
                        }
                    }
                    Ok(())
                })
            },
        )?,
    )?;

    t.set(
        "polyline",
        lua.create_function(
            move |lua, (pts, col, width, closed): (Table, Value, Option<f32>, Option<bool>)| {
                let col = color(&col)?;
                let w = width.unwrap_or(1.0);
                let closed = closed.unwrap_or(false);
                with_ctx(lua, |c| {
                    let pts = points(lua, &pts)?;
                    match space {
                        Space::World => c.renderer.draw_polyline(&pts, w, closed, col),
                        Space::Screen => {
                            c.renderer.screen().draw_polyline(&pts, w, closed, col);
                        }
                    }
                    put_points(lua, pts);
                    Ok(())
                })
            },
        )?,
    )?;

    t.set(
        "polygon",
        lua.create_function(move |lua, (pts, col, line): (Table, Value, Option<f32>)| {
            let col = color(&col)?;
            with_ctx(lua, |c| {
                let pts = points(lua, &pts)?;
                match (space, line) {
                    (Space::World, None) => c.renderer.draw_polygon(&pts, col),
                    (Space::World, Some(l)) => c.renderer.draw_polyline(&pts, l, true, col),
                    (Space::Screen, None) => {
                        c.renderer.screen().draw_polygon(&pts, col);
                    }
                    (Space::Screen, Some(l)) => {
                        c.renderer.screen().draw_polyline(&pts, l, true, col);
                    }
                }
                put_points(lua, pts);
                Ok(())
            })
        })?,
    )?;

    Ok(t)
}

/// Um vetor de pontos reaproveitado entre as chamadas (sem alocar a cada
/// linha desenhada).
#[derive(Default)]
struct PointBuf(Vec<Vec2>);

/// Os pontos de uma lista: `{ x1, y1, x2, y2, ... }` ou `{ {x, y}, ... }`
/// (também `{ {x = 1, y = 2}, ... }`).
fn points(lua: &Lua, t: &Table) -> Result<Vec<Vec2>> {
    let mut out = lua
        .app_data_mut::<PointBuf>()
        .map(|mut b| std::mem::take(&mut b.0))
        .unwrap_or_default();
    out.clear();
    let n = t.raw_len();
    match t.raw_get::<Value>(1)? {
        Value::Table(_) => {
            for i in 1..=n {
                let p: Table = t
                    .raw_get(i)
                    .map_err(|_| err(format!("ponto {i}: esperava {{x, y}}")))?;
                let x = match p.raw_get::<Option<f32>>(1)? {
                    Some(x) => x,
                    None => p
                        .raw_get::<Option<f32>>("x")?
                        .ok_or_else(|| err(format!("ponto {i}: falta o x")))?,
                };
                let y = match p.raw_get::<Option<f32>>(2)? {
                    Some(y) => y,
                    None => p
                        .raw_get::<Option<f32>>("y")?
                        .ok_or_else(|| err(format!("ponto {i}: falta o y")))?,
                };
                out.push(Vec2::new(x, y));
            }
        }
        Value::Nil => {}
        _ => {
            if !n.is_multiple_of(2) {
                return Err(err(format!(
                    "os pontos vêm aos pares (x, y): a lista tem {n} números"
                )));
            }
            for i in (1..=n).step_by(2) {
                out.push(Vec2::new(t.raw_get(i)?, t.raw_get(i + 1)?));
            }
        }
    }
    Ok(out)
}

fn put_points(lua: &Lua, pts: Vec<Vec2>) {
    if let Some(mut b) = lua.app_data_mut::<PointBuf>() {
        b.0 = pts;
    } else {
        lua.set_app_data(PointBuf(pts));
    }
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
    // O efeito na tela inteira (nil tira).
    screen.set(
        "effect",
        lua.create_function(|lua, e: Value| {
            let id = match &e {
                Value::Nil => None,
                Value::UserData(ud) if ud.is::<LEffect>() => Some(ud.borrow::<LEffect>()?.0),
                Value::String(name) => {
                    return Err(err(format!(
                        "nn.screen.effect: esperava um efeito, recebeu o texto \"{}\"; crie com nn.effect: nn.screen.effect(nn.effect(\"{}\"))",
                        name.to_string_lossy(),
                        name.to_string_lossy()
                    )));
                }
                other => {
                    return Err(err(format!(
                        "nn.screen.effect: esperava um efeito (nn.effect) ou nil, recebeu {}",
                        other.type_name()
                    )));
                }
            };
            // Guarda o efeito enquanto ele estiver na tela.
            lua.set_named_registry_value("nn_efeito_tela", e)?;
            with_ctx(lua, |c| {
                c.renderer.set_screen_effect(id);
                Ok(())
            })
        })?,
    )?;
    nn.set("screen", screen)?;
    nn.set("_sprite", lua.create_function(component_sprite)?)?;
    // O recorte das cenas visuais (as listas que rolam): `_clip(tela, x0,
    // y0, x1, y1)` põe, `_clip(tela)` tira.
    nn.set(
        "_clip",
        lua.create_function(
            |lua,
             (screen, x0, y0, x1, y1): (
                bool,
                Option<f32>,
                Option<f32>,
                Option<f32>,
                Option<f32>,
            )| {
                let clip = match (x0, y0, x1, y1) {
                    (Some(x0), Some(y0), Some(x1), Some(y1)) => Some(nn_render::ClipRect::new(
                        Vec2::new(x0, y0),
                        Vec2::new(x1, y1),
                    )),
                    _ => None,
                };
                with_ctx(lua, |c| {
                    if screen {
                        c.renderer.screen().set_clip(clip);
                    } else {
                        c.renderer.set_clip(clip);
                    }
                    Ok(())
                })
            },
        )?,
    )?;

    nn.set(
        "effect",
        lua.create_function(|lua, (name, params): (String, Option<Table>)| {
            let (id, handle) = with_ctx(lua, |c| Ok(c.effect(&name)))?;
            if let Some(p) = params {
                for pair in p.pairs::<String, Value>() {
                    let (k, v) = pair?;
                    LEffect::set(lua, id, &k, &v)?;
                }
            }
            Ok(LEffect(id, handle))
        })?,
    )?;
    nn.set(
        "canvas",
        lua.create_function(|lua, (w, h, opts): (u32, u32, Option<Table>)| {
            if w == 0 || h == 0 || w > 8192 || h > 8192 {
                return Err(err(format!(
                    "nn.canvas: o tamanho precisa ser de 1 a 8192 pixels (pediu {w} x {h})"
                )));
            }
            // Nítido ampliado (pixel art), a não ser que peça smooth.
            let smooth = opt::<bool>(opts.as_ref(), "smooth")?.unwrap_or(false);
            let options = if smooth {
                nn_render::TextureOptions {
                    filter: nn_render::Filter::Linear,
                    mipmaps: false,
                }
            } else {
                nn_render::TextureOptions::PIXEL_ART
            };
            with_ctx(lua, |c| {
                let (id, handle) = c.create_canvas(glam::UVec2::new(w, h), options);
                Ok(LTexture(id, Some(handle)))
            })
        })?,
    )?;
    nn.set(
        "text_lines",
        lua.create_function(|lua, (content, opts): (String, Option<Table>)| {
            let text = build_text(&content, 0.0, 0.0, opts.as_ref())?;
            with_ctx(lua, |c| Ok(c.renderer.text_lines(&text)))
        })?,
    )?;
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
        lua.create_function(|lua, (path, opts): (String, Option<Table>)| {
            // `{ pixel = 8 }`: uma fonte de pixel art, nítida em múltiplos de 8.
            let pixel = opt::<f32>(opts.as_ref(), "pixel")?.filter(|v| *v > 0.0);
            with_ctx(lua, |c| {
                match pixel {
                    Some(em) => c.load_pixel_font(&path, em),
                    None => c.load_font(&path),
                }
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
    // pcall e xpcall devolvem a mensagem do erro em português (como a tela
    // de erro mostra; no xpcall, o que a função de tratar devolve). Só textos e erros da engine; uma tabela jogada com
    // error({...}) volta como veio. Em Lua, para continuar podendo pausar
    // (coroutine.yield) dentro.
    let nn_table: Table = lua.globals().get("nn")?;
    nn_table.set(
        "_translate_only",
        lua.create_function(|_, msg: String| Ok(crate::translate_error(&msg)))?,
    )?;
    nn_table.set(
        "_translate_error",
        lua.create_function(|lua, msg: String| {
            // O começo da pasta do jogo sai dos nomes dos arquivos.
            let root = with_ctx(lua, |c| Ok(c.assets.root().to_string_lossy().into_owned())).ok();
            Ok(crate::clean_error(&msg, root.as_deref()))
        })?,
    )?;
    lua.load(
        r#"
local raw_pcall, raw_xpcall, tr, tr_only = pcall, xpcall, nn._translate_error, nn._translate_only
local type, tostring, select = type, tostring, select
local function pt(e)
  local t = type(e)
  if t == "string" then return tr(e) end
  if t == "userdata" then
    local ok, s = raw_pcall(tostring, e)
    if ok and type(s) == "string" then return tr(s) end
  end
  return e
end
local function fix(ok, ...)
  if ok then return ok, ... end
  return false, pt((...)), select(2, ...)
end
function pcall(f, ...)
  return fix(raw_pcall(f, ...))
end
-- No xpcall, o que a função de tratar devolveu (pode ter o caminho até o
-- erro): só traduzido.
local function fix_x(ok, ...)
  if ok then return ok, ... end
  local e = ...
  if type(e) == "string" then e = tr_only(e) end
  return false, e, select(2, ...)
end
function xpcall(f, handler, ...)
  return fix_x(raw_xpcall(f, handler, ...))
end
-- O Lua dos jogos não tem a biblioteca debug; fica o traceback, que é o que
-- se usa com o xpcall: xpcall(f, debug.traceback).
debug = {
  traceback = function(msg)
    if type(msg) ~= "string" and msg ~= nil and type(msg) ~= "userdata" then return msg end
    local tb = rawget(nn, "_traceback")
    if not tb then return msg end
    return tb(msg == nil and "" or msg)
  end,
}
"#,
    )
    .set_name("=nn")
    .exec()?;
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
