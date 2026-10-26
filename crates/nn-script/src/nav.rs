//! Caminhos para os scripts: a grade que `world:nav(...)` monta (no
//! `nn.physics` e no `nn.physics3d`) e os pedidos de caminho.

use glam::{Vec2, Vec3};
use mlua::{Lua, Result, Table, UserData, UserDataFields, UserDataMethods, Value};
use nn_core::Color;
use nn_nav::NavGrid;
use nn_render::d3::Mesh;

use crate::d3::LMesh;

/// Uma grade de caminhos no Lua.
#[derive(Debug)]
pub(crate) struct LNav {
    pub grid: NavGrid,
    /// No 3D, os pontos são (x, y, z), com o y do chão; no 2D, (x, y).
    pub three_d: bool,
    /// A malha para ver a grade (3D), montada no primeiro pedido.
    mesh: Option<LMesh>,
    /// Mudou desde a malha.
    dirty: bool,
}

impl LNav {
    pub(crate) fn new(grid: NavGrid, three_d: bool) -> Self {
        Self {
            grid,
            three_d,
            mesh: None,
            dirty: true,
        }
    }

    fn point(&self, lua: &Lua, p: Vec2, h: f32) -> Result<Table> {
        let t = lua.create_table()?;
        t.set("x", p.x)?;
        if self.three_d {
            t.set("y", h)?;
            t.set("z", p.y)?;
        } else {
            t.set("y", p.y)?;
        }
        Ok(t)
    }

    /// As células andáveis como quadradinhos no chão (para ver a grade).
    fn debug_mesh(&self) -> Mesh {
        let mut m = Mesh::new();
        let s = self.grid.cell * 0.42;
        let color = Color::WHITE;
        for c in self.grid.walkable_cells() {
            let p = self.grid.center(c);
            let y = self.grid.floor(c) + 0.06;
            m.flat_quad(
                Vec3::new(p.x - s, y, p.y + s),
                Vec3::new(p.x + s, y, p.y + s),
                Vec3::new(p.x + s, y, p.y - s),
                Vec3::new(p.x - s, y, p.y - s),
                color,
            );
        }
        m
    }
}

impl UserData for LNav {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("cell", |_, this| Ok(this.grid.cell));
        f.add_field_method_get("columns", |_, this| Ok(this.grid.width()));
        f.add_field_method_get("rows", |_, this| Ok(this.grid.height()));
        f.add_field_method_get("walkable_count", |_, this| Ok(this.grid.walkable_count()));
        f.add_field_method_get("is_3d", |_, this| Ok(this.three_d));
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        // path(x1, y1, x2, y2) no 2D, path(x1, z1, x2, z2) no 3D: os pontos
        // ({x, y} ou {x, y, z}) e se chega mesmo ao destino.
        m.add_method(
            "path",
            |lua, this, (ax, ay, bx, by): (f32, f32, f32, f32)| {
                let Some(p) = this.grid.path(Vec2::new(ax, ay), Vec2::new(bx, by)) else {
                    return Ok((Value::Nil, false));
                };
                let out = lua.create_table()?;
                for (i, (q, h)) in p.points.iter().zip(&p.heights).enumerate() {
                    out.set(i + 1, this.point(lua, *q, *h)?)?;
                }
                Ok((Value::Table(out), p.complete))
            },
        );
        // O pedaço coberto: x1, y1, x2, y2 (no 3D, x e z).
        m.add_method("bounds", |_, this, ()| {
            let (a, b) = (this.grid.origin, this.grid.max());
            Ok((a.x, a.y, b.x, b.y))
        });
        // Dá para andar ali? (No 3D, x e z.)
        m.add_method("walkable", |_, this, (x, y): (f32, f32)| {
            Ok(this.grid.walkable_at(Vec2::new(x, y)))
        });
        // Dá para ir de um ponto ao outro em linha reta?
        m.add_method(
            "sight",
            |_, this, (ax, ay, bx, by): (f32, f32, f32, f32)| {
                Ok(this
                    .grid
                    .line_of_sight(Vec2::new(ax, ay), Vec2::new(bx, by)))
            },
        );
        // A altura do chão (3D).
        m.add_method("floor", |_, this, (x, z): (f32, f32)| {
            Ok(this.grid.floor_at(Vec2::new(x, z)))
        });
        // block(x1, y1, x2, y2): bloqueia um retângulo; unblock desfaz.
        m.add_method_mut(
            "block",
            |_, this, (ax, ay, bx, by): (f32, f32, f32, f32)| {
                this.grid.block_rect(Vec2::new(ax, ay), Vec2::new(bx, by));
                this.dirty = true;
                Ok(())
            },
        );
        m.add_method_mut(
            "unblock",
            |_, this, (ax, ay, bx, by): (f32, f32, f32, f32)| {
                let cells: Vec<_> = this
                    .grid
                    .cells_in(Vec2::new(ax, ay), Vec2::new(bx, by))
                    .collect();
                for c in cells {
                    this.grid.set_walkable(c, true);
                }
                this.dirty = true;
                Ok(())
            },
        );
        // block_below(x1, z1, x2, z2, nível): água e lava (3D).
        m.add_method_mut(
            "block_below",
            |_, this, (ax, ay, bx, by, level): (f32, f32, f32, f32, f32)| {
                this.grid
                    .block_below(Vec2::new(ax, ay), Vec2::new(bx, by), level);
                this.dirty = true;
                Ok(())
            },
        );
        // As células andáveis: {x, y, (z,) size} cada (para desenhar no 2D).
        m.add_method("cells", |lua, this, ()| {
            let out = lua.create_table()?;
            for (i, c) in this.grid.walkable_cells().enumerate() {
                let t = this.point(lua, this.grid.center(c), this.grid.floor(c))?;
                t.set("size", this.grid.cell)?;
                out.set(i + 1, t)?;
            }
            Ok(out)
        });
        // A malha das células andáveis (3D), para nn.d3.draw.
        m.add_method_mut("mesh", |lua, this, ()| {
            if this.dirty || this.mesh.is_none() {
                let mesh = this.debug_mesh();
                match &mut this.mesh {
                    Some(m) => crate::d3::reupload(lua, m, &mesh)?,
                    None => this.mesh = Some(crate::d3::upload(lua, &mesh)?),
                }
                this.dirty = false;
            }
            Ok(this.mesh.clone().expect("malha"))
        });
    }
}
