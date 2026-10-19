//! `nn.d3.third_person`: câmera em terceira pessoa (atrás do personagem),
//! girada pelo mouse preso e pelo analógico direito, que não atravessa
//! paredes.

use glam::{Vec2, Vec3};
use mlua::{AnyUserData, Lua, Result, Table, UserData, UserDataFields, UserDataMethods, Value};
use nn_input::PadAxis;
use nn_render::d3::Camera3D;

use crate::api::with_ctx;
use crate::convert::{err, opt};

#[derive(Debug, Clone)]
pub(crate) struct LThirdPerson {
    /// Graus em volta do alvo (0 = câmera em +Z, olhando para -Z).
    yaw: f32,
    /// Graus acima do horizonte.
    pitch: f32,
    distance: f32,
    min_distance: f32,
    max_distance: f32,
    min_pitch: f32,
    max_pitch: f32,
    /// Altura do ponto olhado acima da posição do alvo (os pés).
    height: f32,
    /// Graus por pixel do mouse.
    sensitivity: f32,
    /// Graus por segundo com o analógico no fim.
    stick_speed: f32,
    invert_y: bool,
    /// Rodinha aproxima e afasta.
    zoom: bool,
    /// Quão rápido acompanha o alvo (maior = mais grudado).
    follow: f32,
    /// Distância atual (encurta na hora quando algo fica no caminho).
    current: f32,
    target: Option<Vec3>,
    position: Vec3,
}

impl LThirdPerson {
    fn from_opts(o: Option<&Table>) -> Result<Self> {
        let n = |k: &str, d: f32| -> Result<f32> { Ok(opt::<f32>(o, k)?.unwrap_or(d)) };
        let distance = n("distance", 6.0)?.max(0.5);
        Ok(Self {
            yaw: n("yaw", 0.0)?,
            pitch: n("pitch", 20.0)?,
            distance,
            min_distance: n("min_distance", 2.0)?.max(0.3),
            max_distance: n("max_distance", 15.0)?,
            min_pitch: n("min_pitch", -30.0)?.max(-89.0),
            max_pitch: n("max_pitch", 70.0)?.min(89.0),
            height: n("height", 1.5)?,
            sensitivity: n("sensitivity", 0.15)?,
            stick_speed: n("stick_speed", 150.0)?,
            invert_y: opt::<bool>(o, "invert_y")?.unwrap_or(false),
            zoom: opt::<bool>(o, "zoom")?.unwrap_or(true),
            follow: n("follow", 12.0)?.max(0.0),
            current: distance,
            target: None,
            position: Vec3::ZERO,
        })
    }

    /// Para onde a câmera olha, no chão (X, Z), normalizado.
    fn forward(&self) -> Vec2 {
        let y = self.yaw.to_radians();
        Vec2::new(-y.sin(), -y.cos())
    }

    fn right(&self) -> Vec2 {
        let f = self.forward();
        Vec2::new(-f.y, f.x)
    }

    fn offset_dir(&self) -> Vec3 {
        let (y, p) = (self.yaw.to_radians(), self.pitch.to_radians());
        Vec3::new(y.sin() * p.cos(), p.sin(), y.cos() * p.cos())
    }

    fn rotate(&mut self, dyaw: f32, dpitch: f32) {
        self.yaw = (self.yaw + dyaw).rem_euclid(360.0);
        self.pitch = (self.pitch + dpitch).clamp(self.min_pitch, self.max_pitch);
    }
}

impl UserData for LThirdPerson {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("yaw", |_, this| Ok(this.yaw));
        f.add_field_method_set("yaw", |_, this, v: f32| {
            this.yaw = v.rem_euclid(360.0);
            Ok(())
        });
        f.add_field_method_get("pitch", |_, this| Ok(this.pitch));
        f.add_field_method_set("pitch", |_, this, v: f32| {
            this.pitch = v.clamp(this.min_pitch, this.max_pitch);
            Ok(())
        });
        f.add_field_method_get("distance", |_, this| Ok(this.distance));
        f.add_field_method_set("distance", |_, this, v: f32| {
            this.distance = v.clamp(this.min_distance, this.max_distance.max(this.min_distance));
            Ok(())
        });
        f.add_field_method_get("sensitivity", |_, this| Ok(this.sensitivity));
        f.add_field_method_set("sensitivity", |_, this, v: f32| {
            this.sensitivity = v.max(0.0);
            Ok(())
        });
        f.add_field_method_get("invert_y", |_, this| Ok(this.invert_y));
        f.add_field_method_set("invert_y", |_, this, v: bool| {
            this.invert_y = v;
            Ok(())
        });
        f.add_field_method_get("x", |_, this| Ok(this.position.x));
        f.add_field_method_get("y", |_, this| Ok(this.position.y));
        f.add_field_method_get("z", |_, this| Ok(this.position.z));
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method_mut("rotate", |_, this, (dy, dp): (f32, f32)| {
            this.rotate(dy, dp);
            Ok(())
        });
        // Direção de andar no chão a partir da entrada (X para a direita, Y
        // para a frente): devolve vx, vz, com o mesmo comprimento.
        m.add_method("direction", |_, this, (ix, iy): (f32, f32)| {
            let v = this.right() * ix + this.forward() * iy;
            Ok((v.x, v.y))
        });
        m.add_method("forward", |_, this, ()| {
            let f = this.forward();
            Ok((f.x, f.y))
        });
        m.add_method_mut(
            "update",
            |lua, this, (dt, x, y, z, opts): (f32, f32, f32, f32, Option<Table>)| {
                let o = opts.as_ref();
                let world: Option<AnyUserData> = match opt::<Value>(o, "world")? {
                    None | Some(Value::Nil) => None,
                    Some(Value::UserData(ud)) => Some(ud),
                    Some(other) => {
                        return Err(err(format!(
                            "\"world\" deve ser um nn.physics3d.world, recebeu {}",
                            other.type_name()
                        )));
                    }
                };
                let ignore: Option<AnyUserData> =
                    opt::<Option<AnyUserData>>(o, "ignore")?.flatten();
                let control = opt::<bool>(o, "control")?.unwrap_or(true);
                let dt = dt.clamp(0.0, 0.1);

                // Entrada: mouse preso, analógico direito e rodinha.
                let (mouse, stick, scroll) = with_ctx(lua, |c| {
                    let mouse = if c.mouse_captured() {
                        c.input.mouse_delta()
                    } else {
                        Vec2::ZERO
                    };
                    let mut stick = Vec2::new(
                        c.input.pad_axis(PadAxis::RightX),
                        c.input.pad_axis(PadAxis::RightY),
                    );
                    if stick.length() < 0.15 {
                        stick = Vec2::ZERO;
                    }
                    Ok((mouse, stick, c.input.scroll().y))
                })?;
                if control {
                    let inv = if this.invert_y { -1.0 } else { 1.0 };
                    let dyaw = -mouse.x * this.sensitivity - stick.x * this.stick_speed * dt;
                    let dpitch =
                        (mouse.y * this.sensitivity - stick.y * this.stick_speed * dt) * inv;
                    this.rotate(dyaw, dpitch);
                    if this.zoom && scroll != 0.0 {
                        this.distance = (this.distance - scroll * 0.8)
                            .clamp(this.min_distance, this.max_distance.max(this.min_distance));
                    }
                }

                // Segue o alvo com suavidade (no primeiro quadro, vai direto).
                let goal = Vec3::new(x, y + this.height, z);
                let k = 1.0 - (-this.follow * dt).exp();
                let target = match this.target {
                    Some(t) if this.follow > 0.0 => t + (goal - t) * k,
                    _ => goal,
                };
                this.target = Some(target);

                // Não atravessa paredes: encurta na hora, volta devagar.
                let dir = this.offset_dir();
                let mut wanted = this.distance;
                if let Some(w) = &world
                    && let Some(d) = crate::physics3d::ray_distance(
                        w,
                        target,
                        dir,
                        this.distance,
                        ignore.as_ref(),
                    )?
                {
                    wanted = (d - 0.3).max(0.3);
                }
                this.current = if wanted < this.current {
                    wanted
                } else {
                    this.current + (wanted - this.current) * (1.0 - (-4.0 * dt).exp())
                };
                this.position = target + dir * this.current;

                with_ctx(lua, |c| {
                    let mut cam: Camera3D = c.renderer.camera3d();
                    cam.position = this.position;
                    cam.target = target;
                    cam.up = Vec3::Y;
                    c.renderer.set_camera3d(cam);
                    Ok(())
                })
            },
        );
    }
}

pub(crate) fn constructor(lua: &Lua) -> Result<mlua::Function> {
    lua.create_function(|_, opts: Option<Table>| LThirdPerson::from_opts(opts.as_ref()))
}
