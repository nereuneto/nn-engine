//! `nn.d3`: o mundo 3D para os scripts — formas, câmera, luz e névoa.

use std::collections::HashMap;

use glam::{Vec2, Vec3};
use mlua::{Lua, Result, Table, UserData, UserDataFields, Value};
use nn_render::d3::{self, Camera3D, Fog, Mesh, MeshId, Sky3D, Transform3};

use crate::api::with_ctx;
use crate::convert::{color, err, opt, opt_color};
use crate::types::LTexture;

/// Uma malha 3D já enviada para a GPU.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LMesh {
    pub id: MeshId,
    triangles: usize,
    size: Vec3,
}

impl UserData for LMesh {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("triangles", |_, this| Ok(this.triangles));
        f.add_field_method_get("width", |_, this| Ok(this.size.x));
        f.add_field_method_get("height", |_, this| Ok(this.size.y));
        f.add_field_method_get("depth", |_, this| Ok(this.size.z));
    }
}

/// Uma matriz de transformação 3D (`nn.d3.matrix`): posição, rotação e
/// escala. Multiplicar junta duas (pai * filho).
#[derive(Debug, Clone, Copy)]
pub(crate) struct LMat4(pub glam::Mat4);

impl UserData for LMat4 {
    fn add_methods<M: mlua::UserDataMethods<Self>>(m: &mut M) {
        m.add_meta_method(
            mlua::MetaMethod::Mul,
            |_, this, other: mlua::AnyUserData| Ok(LMat4(this.0 * other.borrow::<LMat4>()?.0)),
        );
        m.add_method("position", |_, this, ()| {
            let p = this.0.w_axis;
            Ok((p.x, p.y, p.z))
        });
        // Para onde a frente (-Z) aponta, normalizado.
        m.add_method("forward", |_, this, ()| {
            let f = this.0.transform_vector3(Vec3::NEG_Z).normalize_or_zero();
            Ok((f.x, f.y, f.z))
        });
        m.add_method("transform_point", |_, this, (x, y, z): (f32, f32, f32)| {
            let p = this.0.transform_point3(Vec3::new(x, y, z));
            Ok((p.x, p.y, p.z))
        });
        m.add_method("inverse", |_, this, ()| Ok(LMat4(this.0.inverse())));
        // Posição, rotação (graus, como no draw) e escala.
        m.add_method("decompose", |_, this, ()| {
            let (s, q, t) = this.0.to_scale_rotation_translation();
            let (y, x, z) = q.to_euler(glam::EulerRot::YXZ);
            Ok((
                t.x,
                t.y,
                t.z,
                x.to_degrees(),
                y.to_degrees(),
                z.to_degrees(),
                s.x,
                s.y,
                s.z,
            ))
        });
    }
}

/// Um modelo 3D de arquivo (`nn.assets.model`). Guarda só a chave: ao
/// desenhar, usa a versão mais nova (recarregada quando o arquivo muda).
#[derive(Debug, Clone)]
pub(crate) struct LModel {
    key: String,
    triangles: usize,
    size: Vec3,
    /// Caixa que envolve o modelo e os triângulos (para a física).
    pub bounds: (Vec3, Vec3),
    pub collision: std::sync::Arc<d3::CollisionMesh>,
    /// Esqueleto e animações (modelos animados).
    pub skeleton: Option<std::sync::Arc<d3::Skeleton>>,
}

impl UserData for LModel {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("triangles", |_, this| Ok(this.triangles));
        f.add_field_method_get("width", |_, this| Ok(this.size.x));
        f.add_field_method_get("height", |_, this| Ok(this.size.y));
        f.add_field_method_get("depth", |_, this| Ok(this.size.z));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(m: &mut M) {
        // A caixa em volta do modelo: min x, y, z e max x, y, z.
        m.add_method("bounds", |_, this, ()| {
            let (a, b) = this.bounds;
            Ok((a.x, a.y, a.z, b.x, b.y, b.z))
        });
        // Os nomes das animações do modelo.
        m.add_method("animations", |_, this, ()| {
            Ok(this
                .skeleton
                .as_ref()
                .map(|s| s.animations.iter().map(|a| a.name.clone()).collect())
                .unwrap_or_else(Vec::<String>::new))
        });
        // Quem toca as animações de um objeto (cada objeto tem o seu).
        m.add_method("animator", |_, this, ()| {
            Ok(crate::anim3d::LAnimator::new(
                this.key.trim_end_matches("|flat"),
                this.skeleton.clone(),
            ))
        });
    }
}

/// `nn.assets.model(caminho, opts)`.
pub(crate) fn load_model(lua: &Lua, path: &str, opts: Option<&Table>) -> Result<LModel> {
    let flat = opt::<bool>(opts, "flat")?.unwrap_or(false);
    with_ctx(lua, |c| {
        let model = c.load_model(path, flat).map_err(|e| err(e.to_string()))?;
        Ok(LModel {
            key: nn_app::model_key(path, flat),
            triangles: model.triangles,
            size: model.size(),
            bounds: model.bounds,
            collision: model.collision.clone(),
            skeleton: model.skeleton.clone(),
        })
    })
}

/// As formas prontas já criadas (uma de cada tipo e detalhe basta).
#[derive(Debug, Default)]
struct ShapeCache(HashMap<(&'static str, u32), LMesh>);

/// Troca o conteúdo de uma malha já enviada (o mesmo id).
pub(crate) fn reupload(lua: &Lua, target: &mut LMesh, mesh: &Mesh) -> Result<()> {
    let (min, max) = mesh.bounds();
    with_ctx(lua, |c| {
        c.renderer.update_mesh(target.id, mesh);
        Ok(())
    })?;
    target.triangles = mesh.triangle_count();
    target.size = max - min;
    Ok(())
}

pub(crate) fn upload(lua: &Lua, mesh: &Mesh) -> Result<LMesh> {
    let (min, max) = mesh.bounds();
    with_ctx(lua, |c| {
        Ok(LMesh {
            id: c.renderer.create_mesh(mesh),
            triangles: mesh.triangle_count(),
            size: max - min,
        })
    })
}

pub(crate) fn shape(
    lua: &Lua,
    name: &'static str,
    detail: u32,
    make: fn(u32) -> Mesh,
) -> Result<LMesh> {
    if let Some(m) = lua
        .app_data_ref::<ShapeCache>()
        .and_then(|c| c.0.get(&(name, detail)).copied())
    {
        return Ok(m);
    }
    let mesh = upload(lua, &make(detail))?;
    if let Some(mut cache) = lua.app_data_mut::<ShapeCache>() {
        cache.0.insert((name, detail), mesh);
    }
    Ok(mesh)
}

/// Um ponto `{x, y, z}` ou `{x =, y =, z =}`.
fn vec3(v: &Value, what: &str) -> Result<Vec3> {
    let Value::Table(t) = v else {
        return Err(err(format!(
            "{what}: use um ponto {{x, y, z}}, recebeu {}",
            v.type_name()
        )));
    };
    let get = |i: i64, k: &str| -> Result<f32> {
        let named: Option<f32> = t.get(k)?;
        let indexed: Option<f32> = t.get(i)?;
        Ok(named.or(indexed).unwrap_or(0.0))
    };
    Ok(Vec3::new(get(1, "x")?, get(2, "y")?, get(3, "z")?))
}

fn opt_vec3(opts: Option<&Table>, key: &str) -> Result<Option<Vec3>> {
    match opt::<Value>(opts, key)? {
        None | Some(Value::Nil) => Ok(None),
        Some(v) => vec3(&v, key).map(Some),
    }
}

fn point_table(lua: &Lua, p: Vec3) -> Result<Table> {
    let t = lua.create_table()?;
    t.set("x", p.x)?;
    t.set("y", p.y)?;
    t.set("z", p.z)?;
    Ok(t)
}

fn detail(n: Option<u32>, default: u32, min: u32, max: u32) -> u32 {
    n.unwrap_or(default).clamp(min, max)
}

/// Posição, rotação (graus) e escala das opções de desenho.
fn transform(opts: Option<&Table>) -> Result<Transform3> {
    let n = |k: &str, d: f32| -> Result<f32> { Ok(opt::<f32>(opts, k)?.unwrap_or(d)) };
    let scale = n("scale", 1.0)?;
    Ok(
        Transform3::at(Vec3::new(n("x", 0.0)?, n("y", 0.0)?, n("z", 0.0)?))
            .rotation_degrees(n("rx", 0.0)?, n("ry", 0.0)?, n("rz", 0.0)?)
            .scaled(Vec3::new(n("sx", scale)?, n("sy", scale)?, n("sz", scale)?)),
    )
}

fn camera_table(lua: &Lua, cam: &Camera3D) -> Result<Table> {
    let t = point_table(lua, cam.position)?;
    t.set("target", point_table(lua, cam.target)?)?;
    t.set("fov", cam.fov)?;
    t.set("near", cam.near)?;
    t.set("far", cam.far)?;
    Ok(t)
}

/// Uma malha feita no script: pontos e triângulos (cada um com três índices,
/// a partir de 1, no sentido anti-horário vistos de fora).
fn custom_mesh(opts: &Table) -> Result<Mesh> {
    let points: Table = opts
        .get::<Option<Table>>("points")?
        .ok_or_else(|| err("nn.d3.mesh: faltou \"points\" (a lista de pontos {x, y, z})"))?;
    let tris: Table = opts
        .get::<Option<Table>>("triangles")?
        .ok_or_else(|| err("nn.d3.mesh: faltou \"triangles\" (a lista de triângulos {1, 2, 3})"))?;
    let pts: Vec<Vec3> = points
        .sequence_values::<Value>()
        .enumerate()
        .map(|(i, v)| vec3(&v?, &format!("ponto {}", i + 1)))
        .collect::<Result<_>>()?;
    let colors: Option<Table> = opts.get("colors")?;
    let base = match opts.get::<Value>("color")? {
        Value::Nil => nn_core::Color::WHITE,
        v => color(&v)?,
    };
    let mut mesh = Mesh::new();
    for (i, t) in tris.sequence_values::<Table>().enumerate() {
        let t = t.map_err(|_| err(format!("triângulo {}: use {{a, b, c}}", i + 1)))?;
        let mut p = [Vec3::ZERO; 3];
        for (k, slot) in p.iter_mut().enumerate() {
            let idx: i64 = t
                .get::<Option<i64>>(k as i64 + 1)?
                .ok_or_else(|| err(format!("triângulo {}: precisa de três pontos", i + 1)))?;
            *slot = *pts
                .get((idx - 1).max(0) as usize)
                .filter(|_| idx >= 1)
                .ok_or_else(|| {
                    err(format!(
                        "triângulo {}: o ponto {idx} não existe (há {} pontos)",
                        i + 1,
                        pts.len()
                    ))
                })?;
        }
        let c = match &colors {
            Some(list) => match list.get::<Value>(i as i64 + 1)? {
                Value::Nil => base,
                v => color(&v)?,
            },
            None => base,
        };
        mesh.flat_triangle(p[0], p[1], p[2], c);
    }
    Ok(mesh)
}

pub(crate) fn table(lua: &Lua) -> Result<Table> {
    lua.set_app_data(ShapeCache::default());
    let t = lua.create_table()?;

    t.set(
        "cube",
        lua.create_function(|lua, ()| shape(lua, "cube", 0, |_| d3::shapes::cube()))?,
    )?;
    t.set(
        "plane",
        lua.create_function(|lua, ()| shape(lua, "plane", 0, |_| d3::shapes::plane()))?,
    )?;
    t.set(
        "sphere",
        lua.create_function(|lua, n: Option<u32>| {
            shape(lua, "sphere", detail(n, 2, 0, 5), d3::shapes::sphere)
        })?,
    )?;
    t.set(
        "cylinder",
        lua.create_function(|lua, n: Option<u32>| {
            shape(lua, "cylinder", detail(n, 16, 3, 128), d3::shapes::cylinder)
        })?,
    )?;
    t.set(
        "cone",
        lua.create_function(|lua, n: Option<u32>| {
            shape(lua, "cone", detail(n, 16, 3, 128), d3::shapes::cone)
        })?,
    )?;
    t.set(
        "capsule",
        lua.create_function(|lua, n: Option<u32>| {
            shape(lua, "capsule", detail(n, 16, 3, 128), d3::shapes::capsule)
        })?,
    )?;
    t.set(
        "mesh",
        lua.create_function(|lua, opts: Table| upload(lua, &custom_mesh(&opts)?))?,
    )?;
    t.set("terrain", crate::terrain3d::constructor(lua)?)?;

    t.set(
        "draw",
        lua.create_function(|lua, (what, opts): (Value, Option<Table>)| {
            enum Thing {
                Mesh(LMesh),
                Model(String),
            }
            let thing = match &what {
                Value::UserData(ud) if ud.is::<LMesh>() => Thing::Mesh(*ud.borrow::<LMesh>()?),
                Value::UserData(ud) if ud.is::<crate::terrain3d::LTerrain>() => {
                    Thing::Mesh(ud.borrow_mut::<crate::terrain3d::LTerrain>()?.mesh(lua)?)
                }
                Value::UserData(ud) if ud.is::<LModel>() => {
                    Thing::Model(ud.borrow::<LModel>()?.key.clone())
                }
                other => {
                    return Err(err(format!(
                        "nn.d3.draw: o primeiro valor deve ser uma malha (nn.d3.cube(), ...) ou um modelo (nn.assets.model), recebeu {}",
                        other.type_name()
                    )));
                }
            };
            let o = opts.as_ref();
            let model = match opt::<Value>(o, "body")? {
                None | Some(Value::Nil) => match opt::<Value>(o, "matrix")? {
                    Some(Value::UserData(ud)) if ud.is::<LMat4>() => ud.borrow::<LMat4>()?.0,
                    None | Some(Value::Nil) => transform(o)?.matrix(),
                    Some(other) => {
                        return Err(err(format!(
                            "\"matrix\" deve ser uma nn.d3.matrix, recebeu {}",
                            other.type_name()
                        )));
                    }
                },
                Some(Value::UserData(ud)) if ud.is::<crate::physics3d::LBody3>() => {
                    // Posição e rotação do corpo da física; a escala das opções.
                    let (p, q) = ud.borrow::<crate::physics3d::LBody3>()?.pose()?;
                    let s = transform(o)?.scale;
                    glam::Mat4::from_scale_rotation_translation(s, q, p)
                }
                Some(other) => {
                    return Err(err(format!(
                        "\"body\" deve ser um corpo de nn.physics3d, recebeu {}",
                        other.type_name()
                    )));
                }
            };
            let col = opt_color(o, "color")?.unwrap_or(nn_core::Color::WHITE);
            let shadow = opt::<bool>(o, "shadow")?.unwrap_or(true);
            let joints = match opt::<Value>(o, "animator")? {
                None | Some(Value::Nil) => None,
                Some(Value::UserData(ud)) if ud.is::<crate::anim3d::LAnimator>() => {
                    Some(ud.borrow::<crate::anim3d::LAnimator>()?.joints())
                }
                Some(other) => {
                    return Err(err(format!(
                        "\"animator\" deve vir de modelo:animator(), recebeu {}",
                        other.type_name()
                    )));
                }
            };
            let image = match opt::<Value>(o, "image")? {
                None | Some(Value::Nil) => None,
                Some(Value::UserData(ud)) if ud.is::<LTexture>() => Some(ud.borrow::<LTexture>()?.0),
                Some(other) => {
                    return Err(err(format!(
                        "\"image\" deve ser uma imagem (nn.assets.texture), recebeu {}",
                        other.type_name()
                    )));
                }
            };
            let glow = opt::<f32>(o, "glow")?.unwrap_or(0.0).max(0.0);
            // emissive = false apaga o brilho que o modelo já traz no arquivo.
            let emissive = opt::<bool>(o, "emissive")?.unwrap_or(true);
            let water = match opt::<String>(o, "material")?.as_deref() {
                None | Some("" | "normal") => false,
                Some("water" | "agua" | "água") => true,
                Some(other) => {
                    return Err(err(format!(
                        "\"material\": use \"normal\" ou \"water\", recebeu \"{other}\""
                    )));
                }
            };
            let sway = opt::<f32>(o, "sway")?.unwrap_or(0.0).max(0.0);
            with_ctx(lua, |c| {
                match thing {
                    Thing::Mesh(mesh) => c.renderer.draw_mesh_material(
                        mesh.id,
                        model,
                        col,
                        image,
                        shadow,
                        d3::Material3D {
                            glow: d3::Glow::own(glow),
                            water,
                            sway,
                        },
                    ),
                    Thing::Model(key) => {
                        let g = d3::Glow {
                            amount: glow,
                            color: if emissive {
                                nn_core::Color::WHITE
                            } else {
                                nn_core::Color::BLACK
                            },
                        };
                        let m = d3::Material3D {
                            glow: g,
                            water,
                            sway,
                        };
                        c.draw_model_key_material(&key, model, col, shadow, m, joints.as_deref());
                    }
                }
                Ok(())
            })
        })?,
    )?;

    t.set(
        "text",
        lua.create_function(|lua, (content, opts): (Value, Option<Table>)| {
            let content = match &content {
                Value::String(s) => s.to_str()?.to_string(),
                Value::Nil => String::new(),
                other => lua
                    .coerce_string(other.clone())?
                    .map(|s| s.to_string_lossy())
                    .unwrap_or_else(|| format!("{other:?}")),
            };
            let o = opts.as_ref();
            let mut text = crate::api::build_text(&content, 0.0, 0.0, o)?;
            if opt::<String>(o, "anchor")?.is_none() {
                text = text.anchor(nn_render::Anchor::Center);
            }
            let size = opt::<f32>(o, "size")?.unwrap_or(0.5);
            let mut t3 = d3::Text3D::at(Vec3::ZERO, size);
            match opt::<Value>(o, "matrix")? {
                Some(Value::UserData(ud)) if ud.is::<LMat4>() => {
                    // Uma placa: os eixos X e Y da matriz (com a escala).
                    let m = ud.borrow::<LMat4>()?.0;
                    t3.position = m.transform_point3(Vec3::ZERO);
                    if !opt::<bool>(o, "face_camera")?.unwrap_or(false) {
                        t3.axes = Some((m.x_axis.truncate(), m.y_axis.truncate()));
                    } else {
                        t3.size *= m.y_axis.truncate().length();
                    }
                }
                None | Some(Value::Nil) => {
                    t3.position = Vec3::new(
                        opt::<f32>(o, "x")?.unwrap_or(0.0),
                        opt::<f32>(o, "y")?.unwrap_or(0.0),
                        opt::<f32>(o, "z")?.unwrap_or(0.0),
                    );
                }
                Some(other) => {
                    return Err(err(format!(
                        "\"matrix\" deve ser uma nn.d3.matrix, recebeu {}",
                        other.type_name()
                    )));
                }
            }
            t3.background = opt_color(o, "background")?;
            t3.outline = opt_color(o, "outline")?;
            with_ctx(lua, |c| {
                c.renderer.draw_text3d(&text, &t3);
                Ok(())
            })
        })?,
    )?;

    t.set(
        "camera",
        lua.create_function(|lua, opts: Option<Table>| {
            with_ctx(lua, |c| {
                let mut cam = c.renderer.camera3d();
                if let Some(o) = opts.as_ref() {
                    let o = Some(o);
                    let n = |k: &str, d: f32| -> Result<f32> { Ok(opt::<f32>(o, k)?.unwrap_or(d)) };
                    cam.position = Vec3::new(
                        n("x", cam.position.x)?,
                        n("y", cam.position.y)?,
                        n("z", cam.position.z)?,
                    );
                    if let Some(target) = opt_vec3(o, "target")? {
                        cam.target = target;
                    }
                    cam.fov = n("fov", cam.fov)?.clamp(1.0, 179.0);
                    cam.near = n("near", cam.near)?.max(0.001);
                    cam.far = n("far", cam.far)?.max(cam.near + 0.01);
                    c.renderer.set_camera3d(cam);
                }
                camera_table(lua, &cam)
            })
        })?,
    )?;
    t.set(
        "look_at",
        lua.create_function(
            |lua, (x, y, z, tx, ty, tz): (f32, f32, f32, Option<f32>, Option<f32>, Option<f32>)| {
                with_ctx(lua, |c| {
                    let mut cam = c.renderer.camera3d();
                    cam.position = Vec3::new(x, y, z);
                    cam.target = Vec3::new(tx.unwrap_or(0.0), ty.unwrap_or(0.0), tz.unwrap_or(0.0));
                    c.renderer.set_camera3d(cam);
                    Ok(())
                })
            },
        )?,
    )?;
    t.set(
        "orbit",
        lua.create_function(|lua, opts: Table| {
            let o = Some(&opts);
            let target = opt_vec3(o, "target")?.unwrap_or(Vec3::ZERO);
            let distance = opt::<f32>(o, "distance")?.unwrap_or(8.0).max(0.01);
            let yaw = opt::<f32>(o, "yaw")?.unwrap_or(0.0).to_radians();
            let pitch = opt::<f32>(o, "pitch")?
                .unwrap_or(25.0)
                .clamp(-89.0, 89.0)
                .to_radians();
            let offset = Vec3::new(
                yaw.sin() * pitch.cos(),
                pitch.sin(),
                yaw.cos() * pitch.cos(),
            ) * distance;
            with_ctx(lua, |c| {
                let mut cam = c.renderer.camera3d();
                cam.target = target;
                cam.position = target + offset;
                c.renderer.set_camera3d(cam);
                camera_table(lua, &cam)
            })
        })?,
    )?;

    t.set("third_person", crate::camera3d::constructor(lua)?)?;
    t.set(
        "matrix",
        lua.create_function(|_, opts: Option<Table>| {
            Ok(LMat4(transform(opts.as_ref())?.matrix()))
        })?,
    )?;
    t.set(
        "light",
        lua.create_function(|lua, opts: Option<Table>| {
            with_ctx(lua, |c| {
                let mut l = c.renderer.light3d();
                if let Some(o) = opts.as_ref() {
                    let o = Some(o);
                    if let Some(dir) = opt_vec3(o, "sun")? {
                        if dir.length_squared() < 1e-8 {
                            return Err(err("\"sun\": a direção do sol não pode ser {0, 0, 0}"));
                        }
                        l.sun_direction = dir.normalize();
                    }
                    if let Some(col) = opt_color(o, "sun_color")? {
                        l.sun_color = col;
                    }
                    if let Some(col) = opt_color(o, "sky")? {
                        l.sky = col;
                    }
                    if let Some(col) = opt_color(o, "ground")? {
                        l.ground = col;
                    }
                    match opt::<Value>(o, "fog")? {
                        None | Some(Value::Nil) => {}
                        Some(Value::Boolean(false)) => l.fog = None,
                        Some(Value::Table(f)) => {
                            let f = Some(&f);
                            let near = opt::<f32>(f, "near")?.unwrap_or(10.0);
                            let far = opt::<f32>(f, "far")?.unwrap_or(near + 40.0);
                            if far <= near {
                                return Err(err("névoa: \"far\" precisa ser maior que \"near\""));
                            }
                            l.fog = Some(Fog {
                                color: opt_color(f, "color")?,
                                near: near.max(0.0),
                                far,
                            });
                        }
                        Some(other) => {
                            return Err(err(format!(
                                "\"fog\": use {{color =, near =, far =}} ou false, recebeu {}",
                                other.type_name()
                            )));
                        }
                    }
                    c.renderer.set_light3d(l);
                }
                Ok(())
            })
        })?,
    )?;

    t.set(
        "lamp",
        lua.create_function(|lua, opts: Table| {
            let o = Some(&opts);
            let position = Vec3::new(
                opt::<f32>(o, "x")?.unwrap_or(0.0),
                opt::<f32>(o, "y")?.unwrap_or(0.0),
                opt::<f32>(o, "z")?.unwrap_or(0.0),
            );
            let mut lamp = d3::Lamp3D::at(position);
            if let Some(c) = opt_color(o, "color")? {
                lamp.color = c;
            }
            if let Some(k) = opt::<f32>(o, "intensity")? {
                lamp.intensity = k.max(0.0);
            }
            if let Some(r) = opt::<f32>(o, "range")? {
                lamp.range = r.max(0.0);
            }
            if let Some(dir) = opt_vec3(o, "direction")? {
                if dir.length_squared() < 1e-8 {
                    return Err(err("\"direction\": a direção não pode ser {0, 0, 0}"));
                }
                lamp.cone = Some(d3::Cone3D {
                    direction: dir.normalize(),
                    angle: opt::<f32>(o, "angle")?.unwrap_or(30.0).clamp(1.0, 89.0),
                    softness: opt::<f32>(o, "soft")?.unwrap_or(0.25).clamp(0.0, 1.0),
                });
            }
            with_ctx(lua, |c| {
                c.renderer.add_lamp3d(lamp);
                Ok(())
            })
        })?,
    )?;

    t.set(
        "bloom",
        lua.create_function(|lua, v: Value| {
            with_ctx(lua, |c| {
                let mut b = c.renderer.bloom3d();
                match &v {
                    Value::Nil => {}
                    Value::Boolean(on) => b.enabled = *on,
                    Value::Table(t) => {
                        let o = Some(t);
                        if let Some(k) = opt::<f32>(o, "strength")? {
                            b.strength = k.clamp(0.0, 4.0);
                        }
                        if let Some(on) = opt::<bool>(o, "enabled")? {
                            b.enabled = on;
                        }
                    }
                    other => {
                        return Err(err(format!(
                            "nn.d3.bloom: use true, false ou {{strength =, enabled =}}, recebeu {}",
                            other.type_name()
                        )));
                    }
                }
                c.renderer.set_bloom3d(b);
                Ok(b.enabled)
            })
        })?,
    )?;

    t.set(
        "wind",
        lua.create_function(|lua, opts: Option<Table>| {
            with_ctx(lua, |c| {
                let mut w = c.renderer.wind3d();
                if let Some(o) = opts.as_ref() {
                    let o = Some(o);
                    if let Some(k) = opt::<f32>(o, "strength")? {
                        w.strength = k.max(0.0);
                    }
                    if let Some(t) = opt::<f32>(o, "turn")? {
                        // Graus: 0 = para -Z (a frente do mundo), 90 = para +X.
                        let r = t.to_radians();
                        w.direction = Vec2::new(r.sin(), -r.cos());
                    }
                    if let Some(d) = opt_vec3(o, "direction")? {
                        w.direction = Vec2::new(d.x, d.z);
                    }
                }
                c.renderer.set_wind3d(w);
                let out = lua.create_table()?;
                out.set("strength", w.strength)?;
                out.set("x", w.direction.x)?;
                out.set("z", w.direction.y)?;
                Ok(out)
            })
        })?,
    )?;

    t.set(
        "sky",
        lua.create_function(|lua, v: Value| {
            let sky = match &v {
                Value::Nil | Value::Boolean(false) => None,
                Value::Boolean(true) => Some(Sky3D::default()),
                Value::Table(t) => {
                    let o = Some(t);
                    let d = Sky3D::default();
                    Some(Sky3D {
                        top: opt_color(o, "top")?.unwrap_or(d.top),
                        horizon: opt_color(o, "horizon")?.unwrap_or(d.horizon),
                        bottom: opt_color(o, "bottom")?.unwrap_or(d.bottom),
                        sun_size: opt::<f32>(o, "sun_size")?.unwrap_or(d.sun_size).clamp(0.0, 45.0),
                        stars: opt::<f32>(o, "stars")?.unwrap_or(0.0).clamp(0.0, 1.0),
                    })
                }
                other => {
                    return Err(err(format!(
                        "nn.d3.sky: use {{top =, horizon =, bottom =, sun_size =}}, true ou false, recebeu {}",
                        other.type_name()
                    )));
                }
            };
            with_ctx(lua, |c| {
                c.renderer.set_sky3d(sky);
                Ok(())
            })
        })?,
    )?;
    t.set(
        "shadows",
        lua.create_function(|lua, v: Value| {
            with_ctx(lua, |c| {
                let mut s = c.renderer.shadows3d();
                match &v {
                    Value::Nil => {}
                    Value::Boolean(on) => s.enabled = *on,
                    Value::Table(t) => {
                        let o = Some(t);
                        s.enabled = opt::<bool>(o, "enabled")?.unwrap_or(true);
                        if let Some(d) = opt::<f32>(o, "distance")? {
                            s.distance = d.clamp(1.0, 1000.0);
                        }
                        if let Some(q) = opt::<String>(o, "quality")? {
                            s.resolution = match q.as_str() {
                                "baixa" | "low" => 1024,
                                "media" | "média" | "medium" => 2048,
                                "alta" | "high" => 4096,
                                other => {
                                    return Err(err(format!(
                                        "qualidade de sombra inválida: \"{other}\" (use baixa, media ou alta)"
                                    )));
                                }
                            };
                        }
                        if let Some(f) = opt::<f32>(o, "strength")? {
                            s.strength = f.clamp(0.0, 1.0);
                        }
                    }
                    other => {
                        return Err(err(format!(
                            "nn.d3.shadows: use true, false ou {{distance =, quality =, strength =}}, recebeu {}",
                            other.type_name()
                        )));
                    }
                }
                c.renderer.set_shadows3d(s);
                Ok(s.enabled)
            })
        })?,
    )?;

    t.set(
        "to_screen",
        lua.create_function(|lua, (x, y, z): (f32, f32, f32)| {
            with_ctx(lua, |c| {
                let size = c.viewport().as_vec2();
                Ok(
                    match c.renderer.camera3d().screen_point(Vec3::new(x, y, z), size) {
                        Some(p) => (Some(p.x), Some(p.y)),
                        None => (None, None),
                    },
                )
            })
        })?,
    )?;
    t.set(
        "ray",
        lua.create_function(|lua, (sx, sy): (f32, f32)| {
            with_ctx(lua, |c| {
                let size = c.viewport().as_vec2();
                let (o, d) = c.renderer.camera3d().ray(Vec2::new(sx, sy), size);
                Ok((o.x, o.y, o.z, d.x, d.y, d.z))
            })
        })?,
    )?;
    t.set(
        "ground_point",
        lua.create_function(|lua, (sx, sy, height): (f32, f32, Option<f32>)| {
            with_ctx(lua, |c| {
                let size = c.viewport().as_vec2();
                let (o, d) = c.renderer.camera3d().ray(Vec2::new(sx, sy), size);
                let h = height.unwrap_or(0.0);
                if d.y.abs() < 1e-6 {
                    return Ok((None, None, None));
                }
                let t = (h - o.y) / d.y;
                if t < 0.0 {
                    return Ok((None, None, None));
                }
                let p = o + d * t;
                Ok((Some(p.x), Some(p.y), Some(p.z)))
            })
        })?,
    )?;
    t.set(
        "count",
        lua.create_function(|lua, ()| with_ctx(lua, |c| Ok(c.renderer.last_mesh_count())))?,
    )?;
    t.set(
        "stats",
        lua.create_function(|lua, ()| {
            let s = with_ctx(lua, |c| Ok(c.renderer.last_stats3()))?;
            let t = lua.create_table()?;
            t.set("meshes", s.meshes)?;
            t.set("visible", s.visible)?;
            t.set("shadow", s.shadow)?;
            t.set("draw_calls", s.draw_calls)?;
            t.set("lamps", s.lamps)?;
            t.set("halo", s.halo)?;
            Ok(t)
        })?,
    )?;

    Ok(t)
}
