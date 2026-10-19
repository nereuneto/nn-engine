//! `nn.physics3d`: física 3D para os scripts (corpos, personagem, eventos).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use glam::{EulerRot, Mat4, Quat, Vec3};
use mlua::{
    AnyUserData, Lua, MetaMethod, Result, Table, UserData, UserDataFields, UserDataMethods, Value,
};
use nn_core::Color;
use nn_physics3d::{BodyDesc, BodyId3, BodyKind3, CharacterDesc, Event3, Shape3, World3};

use crate::api::with_ctx;
use crate::convert::{err, opt};
use crate::d3::LModel;

/// O mundo e os rótulos dos corpos.
#[derive(Debug)]
pub(crate) struct Shared3 {
    world: World3,
    tags: HashMap<BodyId3, String>,
}

type SharedWorld3 = Rc<RefCell<Shared3>>;

/// `nn.physics3d.world(...)`
#[derive(Debug, Clone)]
pub(crate) struct LWorld3(SharedWorld3);

/// Um corpo do mundo 3D.
#[derive(Debug, Clone)]
pub(crate) struct LBody3 {
    world: SharedWorld3,
    id: BodyId3,
}

const REMOVED: &str = "este corpo já foi removido do mundo";

fn degrees(q: Quat) -> (f32, f32, f32) {
    let (y, x, z) = q.to_euler(EulerRot::YXZ);
    (x.to_degrees(), y.to_degrees(), z.to_degrees())
}

fn from_degrees(x: f32, y: f32, z: f32) -> Quat {
    Quat::from_euler(
        EulerRot::YXZ,
        y.to_radians(),
        x.to_radians(),
        z.to_radians(),
    )
}

impl LBody3 {
    fn new(world: &SharedWorld3, id: BodyId3) -> Self {
        Self {
            world: world.clone(),
            id,
        }
    }

    /// Posição e rotação, para desenhar.
    pub(crate) fn pose(&self) -> Result<(Vec3, Quat)> {
        let s = self.world.borrow();
        let p = s.world.position(self.id).ok_or_else(|| err(REMOVED))?;
        Ok((p, s.world.rotation(self.id).unwrap_or(Quat::IDENTITY)))
    }

    fn get<R>(&self, f: impl FnOnce(&World3) -> Option<R>) -> Result<R> {
        f(&self.world.borrow().world).ok_or_else(|| err(REMOVED))
    }

    fn set(&self, f: impl FnOnce(&mut World3)) -> Result<()> {
        let mut s = self.world.borrow_mut();
        if !s.world.contains(self.id) {
            return Err(err(REMOVED));
        }
        f(&mut s.world);
        Ok(())
    }

    fn set_pos_axis(&self, axis: usize, v: f32) -> Result<()> {
        let mut p = self.get(|w| w.position(self.id))?;
        p[axis] = v;
        self.set(|w| w.set_position(self.id, p))
    }

    fn set_vel_axis(&self, axis: usize, v: f32) -> Result<()> {
        let mut p = self.get(|w| w.velocity(self.id))?;
        p[axis] = v;
        self.set(|w| w.set_velocity(self.id, p))
    }

    fn set_rot_axis(&self, axis: usize, v: f32) -> Result<()> {
        let q = self.get(|w| w.rotation(self.id))?;
        let (x, y, z) = degrees(q);
        let mut a = [x, y, z];
        a[axis] = v;
        self.set(|w| w.set_rotation(self.id, from_degrees(a[0], a[1], a[2])))
    }
}

fn kind_name(kind: BodyKind3) -> &'static str {
    match kind {
        BodyKind3::Static => "static",
        BodyKind3::Dynamic => "dynamic",
        BodyKind3::Kinematic => "kinematic",
    }
}

impl UserData for LBody3 {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("id", |_, this| Ok(this.id.to_bits()));
        f.add_field_method_get("alive", |_, this| {
            Ok(this.world.borrow().world.contains(this.id))
        });
        f.add_field_method_get("type", |_, this| {
            this.get(|w| w.kind(this.id).map(kind_name))
        });
        f.add_field_method_get("is_character", |_, this| {
            Ok(this.world.borrow().world.is_character(this.id))
        });
        f.add_field_method_get("tag", |_, this| {
            Ok(this.world.borrow().tags.get(&this.id).cloned())
        });
        f.add_field_method_set("tag", |_, this, v: Option<String>| {
            let mut s = this.world.borrow_mut();
            match v {
                Some(t) => s.tags.insert(this.id, t),
                None => s.tags.remove(&this.id),
            };
            Ok(())
        });
        for (i, name) in ["x", "y", "z"].into_iter().enumerate() {
            f.add_field_method_get(name, move |_, this| {
                this.get(|w| w.position(this.id).map(|p| p[i]))
            });
            f.add_field_method_set(name, move |_, this, v: f32| this.set_pos_axis(i, v));
        }
        for (i, name) in ["vx", "vy", "vz"].into_iter().enumerate() {
            f.add_field_method_get(name, move |_, this| {
                this.get(|w| w.velocity(this.id).map(|p| p[i]))
            });
            f.add_field_method_set(name, move |_, this, v: f32| this.set_vel_axis(i, v));
        }
        for (i, name) in ["rx", "ry", "rz"].into_iter().enumerate() {
            f.add_field_method_get(name, move |_, this| {
                this.get(|w| {
                    w.rotation(this.id).map(|q| {
                        let (x, y, z) = degrees(q);
                        [x, y, z][i]
                    })
                })
            });
            f.add_field_method_set(name, move |_, this, v: f32| this.set_rot_axis(i, v));
        }
        f.add_field_method_get("mass", |_, this| this.get(|w| w.mass(this.id)));
        f.add_field_method_get("gravity_scale", |_, this| {
            this.get(|w| w.gravity_scale(this.id))
        });
        f.add_field_method_set("gravity_scale", |_, this, v: f32| {
            this.set(|w| w.set_gravity_scale(this.id, v))
        });
        f.add_field_method_get("sleeping", |_, this| {
            Ok(this.world.borrow().world.is_sleeping(this.id))
        });
        f.add_field_method_get("sensor", |_, this| {
            Ok(this.world.borrow().world.is_sensor(this.id))
        });
        f.add_field_method_get("on_ground", |_, this| {
            Ok(this.world.borrow().world.on_ground(this.id))
        });
        f.add_field_method_get("hit_wall", |_, this| {
            Ok(this.world.borrow().world.hit_wall(this.id))
        });
        f.add_field_method_get("hit_ceiling", |_, this| {
            Ok(this.world.borrow().world.hit_ceiling(this.id))
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("remove", |_, this, ()| {
            let mut s = this.world.borrow_mut();
            s.tags.remove(&this.id);
            Ok(s.world.remove(this.id))
        });
        m.add_method("set_position", |_, this, (x, y, z): (f32, f32, f32)| {
            this.set(|w| w.set_position(this.id, Vec3::new(x, y, z)))
        });
        m.add_method("set_velocity", |_, this, (x, y, z): (f32, f32, f32)| {
            this.set(|w| w.set_velocity(this.id, Vec3::new(x, y, z)))
        });
        m.add_method("set_rotation", |_, this, (x, y, z): (f32, f32, f32)| {
            this.set(|w| w.set_rotation(this.id, from_degrees(x, y, z)))
        });
        m.add_method("set_spin", |_, this, (x, y, z): (f32, f32, f32)| {
            let v = Vec3::new(x, y, z) * std::f32::consts::PI / 180.0;
            this.set(|w| w.set_angular_velocity(this.id, v))
        });
        m.add_method("impulse", |_, this, (x, y, z): (f32, f32, f32)| {
            this.set(|w| w.apply_impulse(this.id, Vec3::new(x, y, z)))
        });
        m.add_method(
            "impulse_at",
            |_, this, (x, y, z, px, py, pz): (f32, f32, f32, f32, f32, f32)| {
                this.set(|w| w.apply_impulse_at(this.id, Vec3::new(x, y, z), Vec3::new(px, py, pz)))
            },
        );
        m.add_method("torque", |_, this, (x, y, z): (f32, f32, f32)| {
            this.set(|w| w.apply_torque_impulse(this.id, Vec3::new(x, y, z)))
        });
        m.add_method("walk", |_, this, (vx, vz): (f32, f32)| {
            if !this.world.borrow().world.is_character(this.id) {
                return Err(err(
                    ":walk só vale para personagens (mundo:character); num corpo, use :set_velocity",
                ));
            }
            this.set(|w| w.walk(this.id, Vec3::new(vx, 0.0, vz)))
        });
        m.add_method("jump", |_, this, speed: Option<f32>| {
            let mut s = this.world.borrow_mut();
            if !s.world.is_character(this.id) {
                return Err(err(
                    ":jump só vale para personagens (mundo:character); num corpo, use :impulse",
                ));
            }
            Ok(s.world.jump(this.id, speed.unwrap_or(6.0)))
        });
        m.add_method("touching", |_, this, ()| {
            let ids = this.world.borrow().world.touching(this.id);
            Ok(ids
                .into_iter()
                .map(|id| LBody3::new(&this.world, id))
                .collect::<Vec<_>>())
        });
        m.add_meta_method(MetaMethod::Eq, |_, this, other: AnyUserData| {
            Ok(other
                .borrow::<LBody3>()
                .is_ok_and(|o| o.id == this.id && Rc::ptr_eq(&o.world, &this.world)))
        });
        m.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            let tag = this.world.borrow().tags.get(&this.id).cloned();
            Ok(format!(
                "corpo 3D {} ({})",
                this.id.to_bits(),
                tag.unwrap_or_default()
            ))
        });
    }
}

fn kind_from(o: Option<&Table>) -> Result<BodyKind3> {
    Ok(
        match opt::<String>(o, "type")?
            .unwrap_or_else(|| "dynamic".into())
            .as_str()
        {
            "dynamic" => BodyKind3::Dynamic,
            "static" => BodyKind3::Static,
            "kinematic" => BodyKind3::Kinematic,
            other => {
                return Err(err(format!(
                    "tipo de corpo inválido: \"{other}\" (use dynamic, static ou kinematic)"
                )));
            }
        },
    )
}

fn body_from_opts(opts: &Table) -> Result<BodyDesc> {
    let o = Some(opts);
    let n = |k: &str, d: f32| -> Result<f32> { Ok(opt::<f32>(o, k)?.unwrap_or(d)) };
    let scale = n("scale", 1.0)?;
    let model = match opt::<Value>(o, "model")? {
        None | Some(Value::Nil) => None,
        Some(Value::UserData(ud)) if ud.is::<LModel>() => Some(ud.borrow::<LModel>()?.clone()),
        Some(other) => {
            return Err(err(format!(
                "\"model\" deve ser um modelo (nn.assets.model), recebeu {}",
                other.type_name()
            )));
        }
    };
    let shape_name = opt::<String>(o, "shape")?;
    let mut offset = Vec3::new(
        n("offset_x", 0.0)?,
        n("offset_y", 0.0)?,
        n("offset_z", 0.0)?,
    );
    let shape = match (shape_name.as_deref(), &model) {
        (Some("mesh"), Some(m)) => Shape3::Mesh {
            vertices: m.collision.vertices.iter().map(|v| *v * scale).collect(),
            triangles: m.collision.triangles.clone(),
        },
        (Some("mesh"), None) => {
            return Err(err(
                "a forma \"mesh\" usa os triângulos de um modelo: passe model = nn.assets.model(...)",
            ));
        }
        (None | Some("box"), Some(m)) if opts.get::<Value>("w")?.is_nil() => {
            // A caixa em volta do modelo (que pode não estar centrado).
            let (min, max) = m.bounds;
            offset += (min + max) / 2.0 * scale;
            Shape3::Box((max - min).max(Vec3::splat(0.01)) * scale)
        }
        (None | Some("box"), _) => {
            let size = n("size", 1.0)?;
            Shape3::Box(Vec3::new(n("w", size)?, n("h", size)?, n("d", size)?) * scale)
        }
        (Some("sphere"), _) => Shape3::Sphere {
            radius: n("radius", 0.5)? * scale,
        },
        (Some("capsule"), _) => Shape3::Capsule {
            radius: n("radius", 0.25)? * scale,
            height: n("height", 1.0)? * scale,
        },
        (Some("cylinder"), _) => Shape3::Cylinder {
            radius: n("radius", 0.5)? * scale,
            height: n("height", 1.0)? * scale,
        },
        (Some(other), _) => {
            return Err(err(format!(
                "forma inválida: \"{other}\" (use box, sphere, capsule, cylinder ou mesh)"
            )));
        }
    };
    let d = BodyDesc::default();
    Ok(BodyDesc {
        kind: kind_from(o)?,
        shape,
        offset,
        position: Vec3::new(n("x", 0.0)?, n("y", 0.0)?, n("z", 0.0)?),
        rotation: from_degrees(n("rx", 0.0)?, n("ry", 0.0)?, n("rz", 0.0)?),
        velocity: Vec3::new(n("vx", 0.0)?, n("vy", 0.0)?, n("vz", 0.0)?),
        density: n("density", d.density)?,
        mass: opt::<f32>(o, "mass")?,
        friction: n("friction", d.friction)?,
        bounce: n("bounce", d.bounce)?,
        sensor: opt::<bool>(o, "sensor")?.unwrap_or(false),
        lock_rotation: opt::<bool>(o, "lock_rotation")?.unwrap_or(false),
        gravity_scale: n("gravity_scale", 1.0)?,
        linear_damping: n("damping", d.linear_damping)?,
        angular_damping: n("angular_damping", d.angular_damping)?,
        ccd: opt::<bool>(o, "ccd")?.unwrap_or(false),
    })
}

impl UserData for LWorld3 {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("gravity", |_, this| Ok(this.0.borrow().world.gravity().y));
        f.add_field_method_set("gravity", |_, this, v: f32| {
            this.0
                .borrow_mut()
                .world
                .set_gravity(Vec3::new(0.0, v, 0.0));
            Ok(())
        });
        f.add_field_method_get("count", |_, this| Ok(this.0.borrow().world.len()));
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("body", |_, this, opts: Table| {
            let desc = body_from_opts(&opts)?;
            let tag = opt::<String>(Some(&opts), "tag")?;
            let mut s = this.0.borrow_mut();
            let id = s.world.add(&desc).map_err(|e| err(e.to_string()))?;
            if let Some(t) = tag {
                s.tags.insert(id, t);
            }
            Ok(LBody3::new(&this.0, id))
        });
        m.add_method("character", |_, this, opts: Option<Table>| {
            let o = opts.as_ref();
            let n = |k: &str, d: f32| -> Result<f32> { Ok(opt::<f32>(o, k)?.unwrap_or(d)) };
            let d = CharacterDesc::default();
            let desc = CharacterDesc {
                position: Vec3::new(n("x", 0.0)?, n("y", 0.0)?, n("z", 0.0)?),
                radius: n("radius", d.radius)?,
                height: n("height", d.height)?,
                step: n("step", d.step)?,
                max_slope: n("slope", d.max_slope)?,
                mass: n("mass", d.mass)?,
            };
            let tag = opt::<String>(o, "tag")?;
            let mut s = this.0.borrow_mut();
            let id = s.world.add_character(&desc);
            if let Some(t) = tag {
                s.tags.insert(id, t);
            }
            Ok(LBody3::new(&this.0, id))
        });
        m.add_method("step", |lua, this, dt: f32| {
            let events: Vec<Event3> = {
                let mut s = this.0.borrow_mut();
                s.world.step(dt);
                s.world.events().to_vec()
            };
            let out = lua.create_table()?;
            for (i, e) in events.into_iter().enumerate() {
                let (kind, a, b) = match e {
                    Event3::Enter(a, b) => ("enter", a, b),
                    Event3::Exit(a, b) => ("exit", a, b),
                };
                let t = lua.create_table()?;
                t.set("type", kind)?;
                t.set("a", LBody3::new(&this.0, a))?;
                t.set("b", LBody3::new(&this.0, b))?;
                out.set(i + 1, t)?;
            }
            Ok(out)
        });
        m.add_method(
            "raycast",
            |lua,
             this,
             (x, y, z, dx, dy, dz, max, ignore): (
                f32,
                f32,
                f32,
                f32,
                f32,
                f32,
                Option<f32>,
                Option<AnyUserData>,
            )| {
                let ignore = match ignore {
                    Some(ud) => Some(ud.borrow::<LBody3>()?.id),
                    None => None,
                };
                let hit = this.0.borrow().world.raycast(
                    Vec3::new(x, y, z),
                    Vec3::new(dx, dy, dz),
                    max.unwrap_or(1000.0),
                    ignore,
                );
                let Some(h) = hit else { return Ok(None) };
                let t = lua.create_table()?;
                t.set("x", h.point.x)?;
                t.set("y", h.point.y)?;
                t.set("z", h.point.z)?;
                t.set("nx", h.normal.x)?;
                t.set("ny", h.normal.y)?;
                t.set("nz", h.normal.z)?;
                t.set("distance", h.distance)?;
                t.set("body", LBody3::new(&this.0, h.body))?;
                Ok(Some(t))
            },
        );
        m.add_method(
            "query_sphere",
            |_, this, (x, y, z, r): (f32, f32, f32, f32)| {
                let ids = this.0.borrow().world.query_sphere(Vec3::new(x, y, z), r);
                Ok(ids
                    .into_iter()
                    .map(|id| LBody3::new(&this.0, id))
                    .collect::<Vec<_>>())
            },
        );
        m.add_method("bodies", |_, this, tag: Option<String>| {
            let s = this.0.borrow();
            Ok(s.world
                .ids()
                .into_iter()
                .filter(|id| tag.is_none() || s.tags.get(id) == tag.as_ref())
                .map(|id| LBody3::new(&this.0, id))
                .collect::<Vec<_>>())
        });
        m.add_method("debug_draw", |lua, this, ()| debug_draw(lua, &this.0));
    }
}

/// Distância até o primeiro corpo sólido no raio (para a câmera não
/// atravessar paredes), ignorando `ignore` e os personagens.
pub(crate) fn ray_distance(
    world: &AnyUserData,
    origin: Vec3,
    dir: Vec3,
    max: f32,
    ignore: Option<&AnyUserData>,
) -> Result<Option<f32>> {
    let w = world.borrow::<LWorld3>().map_err(|_| {
        err("o mundo deve ser um nn.physics3d.world")
    })?;
    let ignore = match ignore {
        Some(ud) => Some(
            ud.borrow::<LBody3>()
                .map_err(|_| err("\"ignore\" deve ser um corpo de nn.physics3d"))?
                .id,
        ),
        None => None,
    };
    let s = w.0.borrow();
    // Personagens (o herói, inimigos) não empurram a câmera.
    let mut from = origin;
    let mut left = max;
    for _ in 0..8 {
        let Some(hit) = s.world.raycast(from, dir, left, ignore) else {
            return Ok(None);
        };
        if !s.world.is_character(hit.body) {
            return Ok(Some(max - left + hit.distance));
        }
        let step = hit.distance + 0.05;
        from += dir * step;
        left -= step;
        if left <= 0.0 {
            return Ok(None);
        }
    }
    Ok(None)
}

/// Mostra as formas de colisão por cima do mundo (meio transparentes).
fn debug_draw(lua: &Lua, world: &SharedWorld3) -> Result<()> {
    use crate::d3::shape;
    use nn_render::d3::shapes;
    let cube = shape(lua, "cube", 0, |_| shapes::cube())?.id;
    let sphere = shape(lua, "sphere", 1, shapes::sphere)?.id;
    let cylinder = shape(lua, "cylinder", 12, shapes::cylinder)?.id;
    let capsule = shape(lua, "capsule", 12, shapes::capsule)?.id;
    let s = world.borrow();
    let mut draws = Vec::new();
    for id in s.world.ids() {
        let (Some(p), Some(q), Some(sh), Some(off), Some(kind)) = (
            s.world.position(id),
            s.world.rotation(id),
            s.world.shape(id),
            s.world.shape_offset(id),
            s.world.kind(id),
        ) else {
            continue;
        };
        // Personagens: a posição é a dos pés; a cápsula fica no meio.
        let center = if s.world.is_character(id) {
            match sh {
                Shape3::Capsule { height, .. } => Vec3::Y * height / 2.0,
                _ => Vec3::ZERO,
            }
        } else {
            off
        };
        let (mesh, size) = match sh {
            Shape3::Box(size) => (cube, *size),
            Shape3::Sphere { radius } => (sphere, Vec3::splat(radius * 2.0)),
            Shape3::Cylinder { radius, height } => {
                (cylinder, Vec3::new(radius * 2.0, *height, radius * 2.0))
            }
            Shape3::Capsule { radius, height } => {
                (capsule, Vec3::new(radius * 4.0, *height, radius * 4.0))
            }
            Shape3::Mesh { .. } => continue,
        };
        let color = if s.world.is_sensor(id) {
            Color::rgba(0.3, 1.0, 0.4, 0.3)
        } else {
            match kind {
                BodyKind3::Static => Color::rgba(0.6, 0.6, 0.7, 0.3),
                BodyKind3::Dynamic => Color::rgba(1.0, 0.6, 0.2, 0.35),
                BodyKind3::Kinematic => Color::rgba(0.3, 0.6, 1.0, 0.35),
            }
        };
        let m = Mat4::from_rotation_translation(q, p)
            * Mat4::from_translation(center)
            * Mat4::from_scale(size * 1.01);
        draws.push((mesh, m, color));
    }
    drop(s);
    with_ctx(lua, |c| {
        for (mesh, m, color) in draws {
            c.renderer.draw_mesh_with(mesh, m, color, None, false);
        }
        Ok(())
    })
}

pub(crate) fn table(lua: &Lua) -> Result<Table> {
    let t = lua.create_table()?;
    t.set(
        "world",
        lua.create_function(|_, opts: Option<Table>| {
            let g = opt::<f32>(opts.as_ref(), "gravity")?.unwrap_or(-9.81);
            Ok(LWorld3(Rc::new(RefCell::new(Shared3 {
                world: World3::new(Vec3::new(0.0, g, 0.0)),
                tags: HashMap::new(),
            }))))
        })?,
    )?;
    Ok(t)
}
