//! `nn.physics`: mundo físico e corpos para os scripts.

use std::cell::RefCell;
use std::rc::Rc;

use glam::Vec2;
use mlua::{
    AnyUserData, Lua, MetaMethod, Result, Table, UserData, UserDataFields, UserDataMethods, Value,
};
use nn_core::Color;
use nn_physics::{Aabb, Body, BodyId, BodyKind, Cell, Event, Shape, World};

use crate::api::with_ctx;
use crate::convert::{err, opt};
use crate::tilemap::LTileMap;

/// Mundo físico compartilhado entre o script e os corpos.
pub(crate) type SharedWorld = Rc<RefCell<World>>;

/// `nn.physics.world(...)`
#[derive(Debug, Clone)]
pub(crate) struct LWorld(pub SharedWorld);

/// Um corpo do mundo.
#[derive(Debug, Clone)]
pub(crate) struct LBody {
    world: SharedWorld,
    id: BodyId,
}

impl LBody {
    fn with<R>(&self, f: impl FnOnce(&Body) -> R) -> Result<R> {
        let world = self.world.borrow();
        world
            .get(self.id)
            .map(f)
            .ok_or_else(|| err("este corpo já foi removido do mundo"))
    }

    fn with_mut<R>(&self, f: impl FnOnce(&mut Body) -> R) -> Result<R> {
        let mut world = self.world.borrow_mut();
        world
            .get_mut(self.id)
            .map(f)
            .ok_or_else(|| err("este corpo já foi removido do mundo"))
    }
}

fn kind_name(kind: BodyKind) -> &'static str {
    match kind {
        BodyKind::Static => "static",
        BodyKind::Dynamic => "dynamic",
        BodyKind::Kinematic => "kinematic",
    }
}

impl UserData for LBody {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("id", |_, this| Ok(this.id.to_bits()));
        f.add_field_method_get("alive", |_, this| Ok(this.world.borrow().contains(this.id)));
        f.add_field_method_get("type", |_, this| this.with(|b| kind_name(b.kind)));

        f.add_field_method_get("x", |_, this| this.with(|b| b.position.x));
        f.add_field_method_set("x", |_, this, v: f32| this.with_mut(|b| b.position.x = v));
        f.add_field_method_get("y", |_, this| this.with(|b| b.position.y));
        f.add_field_method_set("y", |_, this, v: f32| this.with_mut(|b| b.position.y = v));
        f.add_field_method_get("vx", |_, this| this.with(|b| b.velocity.x));
        f.add_field_method_set("vx", |_, this, v: f32| this.with_mut(|b| b.velocity.x = v));
        f.add_field_method_get("vy", |_, this| this.with(|b| b.velocity.y));
        f.add_field_method_set("vy", |_, this, v: f32| this.with_mut(|b| b.velocity.y = v));

        f.add_field_method_get("gravity_scale", |_, this| this.with(|b| b.gravity_scale));
        f.add_field_method_set("gravity_scale", |_, this, v: f32| {
            this.with_mut(|b| b.gravity_scale = v)
        });
        f.add_field_method_get("bounce", |_, this| this.with(|b| b.bounce));
        f.add_field_method_set("bounce", |_, this, v: f32| this.with_mut(|b| b.bounce = v));
        f.add_field_method_get("sensor", |_, this| this.with(|b| b.sensor));
        f.add_field_method_set("sensor", |_, this, v: bool| this.with_mut(|b| b.sensor = v));
        f.add_field_method_get("layer", |_, this| this.with(|b| b.layer));
        f.add_field_method_set("layer", |_, this, v: u32| this.with_mut(|b| b.layer = v));
        f.add_field_method_get("mask", |_, this| this.with(|b| b.mask));
        f.add_field_method_set("mask", |_, this, v: u32| this.with_mut(|b| b.mask = v));
        f.add_field_method_get("tag", |_, this| this.with(|b| b.tag.clone()));
        f.add_field_method_set("tag", |_, this, v: String| this.with_mut(|b| b.tag = v));

        f.add_field_method_get("width", |_, this| {
            this.with(|b| b.shape.half_extents().x * 2.0)
        });
        f.add_field_method_get("height", |_, this| {
            this.with(|b| b.shape.half_extents().y * 2.0)
        });
        f.add_field_method_get("radius", |_, this| {
            this.with(|b| match b.shape {
                Shape::Circle { radius } => Some(radius),
                Shape::Rect { .. } => None,
            })
        });

        f.add_field_method_get("on_ground", |_, this| this.with(|b| b.on_ground()));
        f.add_field_method_get("touching_left", |_, this| this.with(|b| b.touching().left));
        f.add_field_method_get("touching_right", |_, this| {
            this.with(|b| b.touching().right)
        });
        f.add_field_method_get("touching_top", |_, this| this.with(|b| b.touching().top));
        f.add_field_method_get("touching_bottom", |_, this| {
            this.with(|b| b.touching().bottom)
        });
        f.add_field_method_get("ground", |_, this| {
            let ground = this.with(|b| b.ground_body())?;
            Ok(ground.map(|id| LBody {
                world: this.world.clone(),
                id,
            }))
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("remove", |_, this, ()| {
            this.world.borrow_mut().remove(this.id);
            Ok(())
        });
        m.add_method("set_position", |_, this, (x, y): (f32, f32)| {
            this.with_mut(|b| b.position = Vec2::new(x, y))
        });
        m.add_method("set_velocity", |_, this, (x, y): (f32, f32)| {
            this.with_mut(|b| b.velocity = Vec2::new(x, y))
        });
        m.add_method("intersects", |_, this, other: AnyUserData| {
            let other = other.borrow::<LBody>()?;
            if !Rc::ptr_eq(&this.world, &other.world) {
                return Err(err("os dois corpos precisam ser do mesmo mundo"));
            }
            Ok(this.world.borrow().intersects(this.id, other.id))
        });
        m.add_method("overlaps", |_, this, ()| {
            let ids = this.world.borrow().overlapping(this.id);
            Ok(ids
                .into_iter()
                .map(|id| LBody {
                    world: this.world.clone(),
                    id,
                })
                .collect::<Vec<_>>())
        });
        m.add_meta_method(MetaMethod::Eq, |_, this, other: AnyUserData| {
            Ok(other
                .borrow::<LBody>()
                .is_ok_and(|o| o.id == this.id && Rc::ptr_eq(&o.world, &this.world)))
        });
        m.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            let tag = this
                .with(|b| b.tag.clone())
                .unwrap_or_else(|_| "removido".into());
            Ok(format!("corpo {} ({tag})", this.id.to_bits()))
        });
    }
}

fn body_from_opts(opts: &Table) -> Result<Body> {
    let o = Some(opts);
    let shape = match (
        opt::<f32>(o, "radius")?,
        opt::<f32>(o, "w")?,
        opt::<f32>(o, "h")?,
    ) {
        (Some(r), _, _) => Shape::circle(r),
        (None, Some(w), Some(h)) => Shape::rect(w, h),
        _ => return Err(err("o corpo precisa de `radius` ou de `w` e `h`")),
    };
    let kind = opt::<String>(o, "type")?.unwrap_or_else(|| "dynamic".into());
    let mut body = match kind.as_str() {
        "dynamic" => Body::dynamic(shape),
        "static" => Body::fixed(shape),
        "kinematic" => Body::kinematic(shape),
        other => {
            return Err(err(format!(
                "tipo de corpo inválido: \"{other}\" (use dynamic, static ou kinematic)"
            )));
        }
    };
    body.position = Vec2::new(
        opt::<f32>(o, "x")?.unwrap_or(0.0),
        opt::<f32>(o, "y")?.unwrap_or(0.0),
    );
    body.velocity = Vec2::new(
        opt::<f32>(o, "vx")?.unwrap_or(0.0),
        opt::<f32>(o, "vy")?.unwrap_or(0.0),
    );
    if let Some(v) = opt::<f32>(o, "gravity_scale")? {
        body.gravity_scale = v;
    }
    if let Some(v) = opt::<bool>(o, "sensor")? {
        body.sensor = v;
    }
    if let Some(v) = opt::<bool>(o, "one_way")? {
        body.one_way = v;
    }
    if let Some(v) = opt::<f32>(o, "bounce")? {
        body.bounce = v;
    }
    if let Some(v) = opt::<u32>(o, "layer")? {
        body.layer = v;
    }
    if let Some(v) = opt::<u32>(o, "mask")? {
        body.mask = v;
    }
    if let Some(v) = opt::<String>(o, "tag")? {
        body.tag = v;
    }
    Ok(body)
}

impl UserData for LWorld {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("gravity_x", |_, this| Ok(this.0.borrow().gravity.x));
        f.add_field_method_set("gravity_x", |_, this, v: f32| {
            this.0.borrow_mut().gravity.x = v;
            Ok(())
        });
        f.add_field_method_get("gravity_y", |_, this| Ok(this.0.borrow().gravity.y));
        f.add_field_method_set("gravity_y", |_, this, v: f32| {
            this.0.borrow_mut().gravity.y = v;
            Ok(())
        });
        f.add_field_method_get("count", |_, this| Ok(this.0.borrow().len()));
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("body", |_, this, opts: Table| {
            let body = body_from_opts(&opts)?;
            let id = this.0.borrow_mut().add(body);
            Ok(LBody {
                world: this.0.clone(),
                id,
            })
        });

        m.add_method("step", |lua, this, dt: f32| {
            let events: Vec<Event> = this.0.borrow_mut().step(dt).to_vec();
            let list = lua.create_table()?;
            for e in events {
                let (kind, a, b) = match e {
                    Event::Enter(a, b) => ("enter", a, b),
                    Event::Exit(a, b) => ("exit", a, b),
                };
                let t = lua.create_table()?;
                t.set("type", kind)?;
                t.set(
                    "a",
                    LBody {
                        world: this.0.clone(),
                        id: a,
                    },
                )?;
                t.set(
                    "b",
                    LBody {
                        world: this.0.clone(),
                        id: b,
                    },
                )?;
                list.push(t)?;
            }
            Ok(list)
        });

        m.add_method("add_tilemap", |_, this, map: AnyUserData| {
            let map = map.borrow::<LTileMap>()?;
            map.attach(&this.0);
            Ok(())
        });

        m.add_method(
            "raycast",
            |lua, this, (x, y, dx, dy, max, mask): (f32, f32, f32, f32, f32, Option<u32>)| {
                let hit = this.0.borrow().raycast(
                    Vec2::new(x, y),
                    Vec2::new(dx, dy),
                    max,
                    mask.unwrap_or(u32::MAX),
                );
                let Some(hit) = hit else {
                    return Ok(Value::Nil);
                };
                let t = lua.create_table()?;
                t.set("x", hit.point.x)?;
                t.set("y", hit.point.y)?;
                t.set("nx", hit.normal.x)?;
                t.set("ny", hit.normal.y)?;
                t.set("distance", hit.distance)?;
                if let Some(id) = hit.body {
                    t.set(
                        "body",
                        LBody {
                            world: this.0.clone(),
                            id,
                        },
                    )?;
                }
                Ok(Value::Table(t))
            },
        );

        m.add_method("query_point", |_, this, (x, y): (f32, f32)| {
            let ids = this.0.borrow().query_point(Vec2::new(x, y));
            Ok(ids
                .into_iter()
                .map(|id| LBody {
                    world: this.0.clone(),
                    id,
                })
                .collect::<Vec<_>>())
        });

        m.add_method(
            "query_rect",
            |_, this, (x, y, w, h): (f32, f32, f32, f32)| {
                let ids = this.0.borrow().query_aabb(Aabb::from_center(
                    Vec2::new(x, y),
                    Vec2::new(w, h).abs() / 2.0,
                ));
                Ok(ids
                    .into_iter()
                    .map(|id| LBody {
                        world: this.0.clone(),
                        id,
                    })
                    .collect::<Vec<_>>())
            },
        );

        m.add_method("bodies", |_, this, tag: Option<String>| {
            let world = this.0.borrow();
            Ok(world
                .bodies()
                .filter(|(_, b)| tag.as_ref().is_none_or(|t| &b.tag == t))
                .map(|(id, _)| LBody {
                    world: this.0.clone(),
                    id,
                })
                .collect::<Vec<_>>())
        });

        m.add_method("debug_draw", |lua, this, ()| {
            let world = this.0.borrow();
            with_ctx(lua, |c| {
                debug_draw(&world, c);
                Ok(())
            })
        });
    }
}

/// Desenha corpos e grades para depuração.
fn debug_draw(world: &World, ctx: &mut nn_app::Context) {
    let zoom = ctx.camera.zoom.max(0.01);
    let line = 1.5 / zoom;
    let view = ctx.viewport().as_vec2() / (2.0 * zoom);
    let visible = Aabb::from_center(ctx.camera.position, view + Vec2::splat(64.0));

    for grid in world.grids() {
        for (c, r, cell) in grid.cells_in(&visible) {
            let b = grid.cell_aabb(c, r);
            let color = match cell {
                Cell::Solid => Color::rgba(1.0, 0.3, 0.3, 0.25),
                Cell::OneWay => Color::rgba(1.0, 0.8, 0.2, 0.3),
                Cell::Empty => continue,
            };
            if cell == Cell::OneWay {
                ctx.renderer.draw_rect(
                    Vec2::new(b.center().x, b.max.y - line),
                    Vec2::new(b.max.x - b.min.x, line * 2.0),
                    color.with_alpha(0.8),
                );
            } else {
                ctx.renderer.draw_rect(b.center(), b.max - b.min, color);
                outline(ctx, &b, line, color.with_alpha(0.6));
            }
        }
    }

    for (_, body) in world.bodies() {
        let color = if body.sensor {
            Color::rgba(1.0, 0.85, 0.2, 1.0)
        } else {
            match body.kind {
                BodyKind::Static => Color::rgba(0.7, 0.7, 0.75, 1.0),
                BodyKind::Dynamic => Color::rgba(0.3, 1.0, 0.45, 1.0),
                BodyKind::Kinematic => Color::rgba(0.35, 0.65, 1.0, 1.0),
            }
        };
        match body.shape {
            Shape::Rect { .. } => {
                let b = body.aabb();
                ctx.renderer
                    .draw_rect(b.center(), b.max - b.min, color.with_alpha(0.15));
                outline(ctx, &b, line, color);
            }
            Shape::Circle { radius } => {
                ctx.renderer
                    .draw_circle(body.position, radius, color.with_alpha(0.25));
            }
        }
        if body.on_ground() {
            let b = body.aabb();
            ctx.renderer.draw_rect(
                Vec2::new(b.center().x, b.min.y),
                Vec2::new(b.max.x - b.min.x, line * 3.0),
                Color::WHITE,
            );
        }
    }
}

fn outline(ctx: &mut nn_app::Context, b: &Aabb, t: f32, color: Color) {
    let size = b.max - b.min;
    let c = b.center();
    ctx.renderer.draw_rect(
        Vec2::new(c.x, b.min.y + t / 2.0),
        Vec2::new(size.x, t),
        color,
    );
    ctx.renderer.draw_rect(
        Vec2::new(c.x, b.max.y - t / 2.0),
        Vec2::new(size.x, t),
        color,
    );
    ctx.renderer.draw_rect(
        Vec2::new(b.min.x + t / 2.0, c.y),
        Vec2::new(t, size.y),
        color,
    );
    ctx.renderer.draw_rect(
        Vec2::new(b.max.x - t / 2.0, c.y),
        Vec2::new(t, size.y),
        color,
    );
}

pub(crate) fn table(lua: &Lua) -> Result<Table> {
    let t = lua.create_table()?;
    t.set(
        "world",
        lua.create_function(|_, opts: Option<Table>| {
            let o = opts.as_ref();
            let gravity = Vec2::new(
                opt::<f32>(o, "gravity_x")?.unwrap_or(0.0),
                opt::<f32>(o, "gravity")?
                    .or(opt::<f32>(o, "gravity_y")?)
                    .unwrap_or(-1500.0),
            );
            Ok(LWorld(Rc::new(RefCell::new(World::new(gravity)))))
        })?,
    )?;
    Ok(t)
}
