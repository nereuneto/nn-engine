//! `nn.d3.terrain`: terrenos com morros e cores, para desenhar, pisar e
//! pintar pelo script.

use glam::{Vec2, Vec3};
use mlua::{Lua, Result, Table, UserData, UserDataFields, UserDataMethods, Value};
use nn_core::Color;
use nn_render::d3::{BrushOp, DEFAULT_LAYER_COLORS, TerrainData};

use crate::convert::{color, err, opt};
use crate::d3::LMesh;

/// Um terreno no Lua.
#[derive(Debug, Clone)]
pub(crate) struct LTerrain {
    pub data: TerrainData,
    pub colors: [Color; 4],
    mesh: Option<LMesh>,
    /// Mudou desde a última malha.
    dirty: bool,
    /// Conta as mudanças (para quem guarda o corpo da física saber).
    pub version: u64,
}

impl LTerrain {
    /// A malha atual (refeita se o terreno mudou).
    pub(crate) fn mesh(&mut self, lua: &Lua) -> Result<LMesh> {
        // A malha só é montada de novo quando o terreno muda (não a cada
        // quadro).
        match &mut self.mesh {
            Some(m) if self.dirty => {
                let mesh = self.data.mesh(&self.colors);
                crate::d3::reupload(lua, m, &mesh)?;
            }
            Some(_) => {}
            None => {
                let mesh = self.data.mesh(&self.colors);
                self.mesh = Some(crate::d3::upload(lua, &mesh)?);
            }
        }
        self.dirty = false;
        Ok(self.mesh.expect("malha"))
    }

    fn changed(&mut self) {
        self.dirty = true;
        self.version += 1;
    }
}

fn layer_colors(opts: Option<&Table>) -> Result<[Color; 4]> {
    let mut out = DEFAULT_LAYER_COLORS;
    if let Some(list) = opt::<Table>(opts, "colors")? {
        for (k, c) in out.iter_mut().enumerate() {
            let v: Value = list.get(k + 1)?;
            if !v.is_nil() {
                *c = color(&v)?;
            }
        }
    }
    Ok(out)
}

impl UserData for LTerrain {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("cells", |_, this| Ok(this.data.cells));
        f.add_field_method_get("width", |_, this| Ok(this.data.size.x));
        f.add_field_method_get("depth", |_, this| Ok(this.data.size.y));
        f.add_field_method_get("max_height", |_, this| Ok(this.data.max_height));
        f.add_field_method_get("version", |_, this| Ok(this.version));
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        // A altura do chão em (x, z), no espaço do terreno (nil fora dele).
        m.add_method("height_at", |_, this, (x, z): (f32, f32)| {
            Ok(this.data.height_at(x, z))
        });
        // Onde um raio bate no chão: x, y, z (ou nil).
        m.add_method(
            "raycast",
            |_, this, (ox, oy, oz, dx, dy, dz, max): (f32, f32, f32, f32, f32, f32, Option<f32>)| {
                let hit = this.data.raycast(
                    Vec3::new(ox, oy, oz),
                    Vec3::new(dx, dy, dz),
                    max.unwrap_or(500.0),
                );
                Ok((hit.map(|p| p.x), hit.map(|p| p.y), hit.map(|p| p.z)))
            },
        );
        // brush("subir" | "descer" | "alisar" | "aplainar" | "ruido", x, z, raio, força, altura)
        m.add_method_mut(
            "brush",
            |_, this, (op, x, z, radius, strength, target): (String, f32, f32, f32, f32, Option<f32>)| {
                let op = match op.as_str() {
                    "subir" | "raise" => BrushOp::Raise,
                    "descer" | "lower" => BrushOp::Lower,
                    "alisar" | "smooth" => BrushOp::Smooth,
                    "aplainar" | "flatten" => {
                        BrushOp::Flatten(target.or(this.data.height_at(x, z)).unwrap_or(0.0))
                    }
                    "ruido" | "noise" => BrushOp::Noise,
                    other => {
                        return Err(err(format!(
                            "pincel inválido: \"{other}\" (use subir, descer, alisar, aplainar ou ruido)"
                        )));
                    }
                };
                let changed = this.data.brush(op, Vec2::new(x, z), radius, strength);
                if changed {
                    this.changed();
                }
                Ok(changed)
            },
        );
        // paint(camada 1 a 4, x, z, raio, força 0 a 1)
        m.add_method_mut(
            "paint",
            |_, this, (layer, x, z, radius, strength): (usize, f32, f32, f32, Option<f32>)| {
                if !(1..=4).contains(&layer) {
                    return Err(err("a camada vai de 1 a 4"));
                }
                let changed =
                    this.data
                        .paint(layer - 1, Vec2::new(x, z), radius, strength.unwrap_or(1.0));
                if changed {
                    this.changed();
                }
                Ok(changed)
            },
        );
        // As cores das quatro camadas ({grama, terra, pedra, areia}).
        m.add_method_mut("set_colors", |lua, this, list: Table| {
            let opts = lua.create_table()?;
            opts.set("colors", list)?;
            this.colors = layer_colors(Some(&opts))?;
            this.dirty = true;
            Ok(())
        });
        m.add_method("encode", |_, this, ()| Ok(this.data.encode()));
        // Troca tudo (como nn.d3.terrain), aproveitando a malha da GPU.
        m.add_method_mut("replace", |_, this, opts: Table| {
            let new = from_opts(Some(&opts))?;
            this.data = new.data;
            this.colors = new.colors;
            this.changed();
            Ok(())
        });
        // A malha para nn.d3.draw (refeita quando o terreno muda).
        m.add_method_mut("mesh", |lua, this, ()| this.mesh(lua));
    }
}

fn from_opts(o: Option<&Table>) -> Result<LTerrain> {
    let cells = opt::<u32>(o, "cells")?.unwrap_or(64).clamp(2, 256);
    let width = opt::<f32>(o, "width")?.unwrap_or(64.0);
    let depth = opt::<f32>(o, "depth")?.unwrap_or(width);
    let height = opt::<f32>(o, "height")?.unwrap_or(20.0);
    let data = opt::<String>(o, "data")?.unwrap_or_default();
    Ok(LTerrain {
        data: TerrainData::decode(&data, cells, Vec2::new(width, depth), height),
        colors: layer_colors(o)?,
        mesh: None,
        dirty: true,
        version: 0,
    })
}

/// `nn.d3.terrain(opts)`
pub(crate) fn constructor(lua: &Lua) -> Result<mlua::Function> {
    lua.create_function(|_, opts: Option<Table>| from_opts(opts.as_ref()))
}
