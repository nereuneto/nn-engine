//! `nn.tilemap`: mapas de tiles para os scripts.

use std::cell::RefCell;
use std::path::Path;
use std::rc::{Rc, Weak};

use glam::Vec2;
use mlua::{AnyUserData, Lua, Result, Table, UserData, UserDataFields, UserDataMethods, Value};
use nn_physics::{Cell, GridId, World};
use nn_render::TextureOptions;
use nn_tilemap::{AsciiTile, MapObject, Properties, Property, TileMap, load_tiled};

use crate::api::with_ctx;
use crate::convert::{err, opt};
use crate::types::LSheet;

#[derive(Debug)]
struct MapState {
    map: TileMap,
    /// Mundos em que o mapa foi adicionado, para atualizar a colisão ao trocar tiles.
    grids: Vec<(Weak<RefCell<World>>, GridId)>,
}

/// Mapa de tiles no Lua.
#[derive(Debug, Clone)]
pub(crate) struct LTileMap(Rc<RefCell<MapState>>);

impl LTileMap {
    /// Retângulo do mapa no mundo.
    pub(crate) fn bounds(&self) -> nn_render::Bounds {
        self.0.borrow().map.bounds()
    }

    fn new(map: TileMap) -> Self {
        Self(Rc::new(RefCell::new(MapState {
            map,
            grids: Vec::new(),
        })))
    }

    /// Adiciona a colisão do mapa a um mundo físico.
    pub(crate) fn attach(&self, world: &Rc<RefCell<World>>) {
        let mut state = self.0.borrow_mut();
        let grid = state.map.collision_grid();
        let id = world.borrow_mut().add_grid(grid);
        state.grids.push((Rc::downgrade(world), id));
    }

    fn layer_index(map: &TileMap, layer: &Value) -> Result<usize> {
        match layer {
            Value::Nil => Ok(0),
            Value::Integer(i) if *i >= 1 && (*i as usize) <= map.layers.len() => {
                Ok(*i as usize - 1)
            }
            Value::String(s) => {
                let name = s.to_str()?;
                map.layer_index(&name)
                    .ok_or_else(|| err(format!("camada \"{}\" não existe", &*name)))
            }
            other => Err(err(format!("camada inválida: {other:?}"))),
        }
    }
}

fn cell_from_lua(col: i64, row: i64, map: &TileMap) -> Result<(u32, u32)> {
    if col < 1 || row < 1 || col > map.width as i64 || row > map.height as i64 {
        return Err(err(format!(
            "célula ({col}, {row}) fora do mapa ({}x{})",
            map.width, map.height
        )));
    }
    Ok((col as u32 - 1, row as u32 - 1))
}

fn properties_table(lua: &Lua, props: &Properties) -> Result<Table> {
    let t = lua.create_table()?;
    for (k, v) in props {
        match v {
            Property::Bool(b) => t.set(k.as_str(), *b)?,
            Property::Int(i) => t.set(k.as_str(), *i)?,
            Property::Float(f) => t.set(k.as_str(), *f)?,
            Property::String(s) => t.set(k.as_str(), s.as_str())?,
        }
    }
    Ok(t)
}

fn object_table(lua: &Lua, o: &MapObject) -> Result<Table> {
    let t = lua.create_table()?;
    t.set("id", o.id)?;
    t.set("name", o.name.as_str())?;
    t.set("type", o.kind.as_str())?;
    t.set("x", o.position.x)?;
    t.set("y", o.position.y)?;
    t.set("w", o.size.x)?;
    t.set("h", o.size.y)?;
    t.set("point", o.point)?;
    t.set("layer", o.layer.as_str())?;
    t.set("properties", properties_table(lua, &o.properties)?)?;
    Ok(t)
}

impl UserData for LTileMap {
    fn add_fields<F: UserDataFields<Self>>(f: &mut F) {
        f.add_field_method_get("width", |_, this| Ok(this.0.borrow().map.width));
        f.add_field_method_get("height", |_, this| Ok(this.0.borrow().map.height));
        f.add_field_method_get("tile_width", |_, this| Ok(this.0.borrow().map.tile_size.x));
        f.add_field_method_get("tile_height", |_, this| Ok(this.0.borrow().map.tile_size.y));
        f.add_field_method_get("world_width", |_, this| {
            Ok(this.0.borrow().map.world_size().x)
        });
        f.add_field_method_get("world_height", |_, this| {
            Ok(this.0.borrow().map.world_size().y)
        });
        f.add_field_method_get("x", |_, this| Ok(this.0.borrow().map.origin.x));
        f.add_field_method_get("y", |_, this| Ok(this.0.borrow().map.origin.y));
        f.add_field_method_get("properties", |lua, this| {
            properties_table(lua, &this.0.borrow().map.properties)
        });
        f.add_field_method_get("layers", |_, this| {
            Ok(this
                .0
                .borrow()
                .map
                .layers
                .iter()
                .filter(|l| !l.properties.contains_key("nn_collision_codes"))
                .map(|l| l.name.clone())
                .collect::<Vec<_>>())
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(m: &mut M) {
        m.add_method("draw", |lua, this, ()| {
            let mut state = this.0.borrow_mut();
            with_ctx(lua, |c| {
                // Tiles animados acompanham o relógio do jogo.
                state.map.time = c.time.elapsed_secs();
                let camera = c.camera;
                let viewport = c.viewport().as_vec2();
                state.map.draw(&mut c.renderer, &camera, viewport);
                Ok(())
            })
        });

        m.add_method("draw_layer", |lua, this, layer: Value| {
            let mut state = this.0.borrow_mut();
            let index = Self::layer_index(&state.map, &layer)?;
            with_ctx(lua, |c| {
                state.map.time = c.time.elapsed_secs();
                let camera = c.camera;
                let viewport = c.viewport().as_vec2();
                state
                    .map
                    .draw_layer(index, &mut c.renderer, &camera, viewport);
                Ok(())
            })
        });

        m.add_method(
            "animate",
            |_, this, (tile, frames, fps): (u32, Vec<u32>, Option<f32>)| {
                let mut state = this.0.borrow_mut();
                if state.map.animate(tile, &frames, fps.unwrap_or(8.0)) {
                    Ok(())
                } else {
                    Err(err(format!(
                        "não foi possível animar o tile {tile}: os quadros precisam ser tiles \
                         do mesmo tileset que ele"
                    )))
                }
            },
        );

        m.add_method("objects", |lua, this, kind: Option<String>| {
            let state = this.0.borrow();
            let list = lua.create_table()?;
            for o in &state.map.objects {
                if kind.as_ref().is_none_or(|k| &o.kind == k) {
                    list.push(object_table(lua, o)?)?;
                }
            }
            Ok(list)
        });

        m.add_method("object", |lua, this, name: String| {
            let state = this.0.borrow();
            match state.map.object(&name) {
                Some(o) => Ok(Value::Table(object_table(lua, o)?)),
                None => Ok(Value::Nil),
            }
        });

        m.add_method("tile", |_, this, (col, row, layer): (i64, i64, Value)| {
            let state = this.0.borrow();
            let (c, r) = cell_from_lua(col, row, &state.map)?;
            let index = Self::layer_index(&state.map, &layer)?;
            Ok(state.map.tile(index, c, r))
        });

        m.add_method(
            "set_tile",
            |_, this, (col, row, tile, layer): (i64, i64, u32, Value)| {
                let mut state = this.0.borrow_mut();
                let (c, r) = cell_from_lua(col, row, &state.map)?;
                let index = Self::layer_index(&state.map, &layer)?;
                state.map.set_tile(index, c, r, tile);
                // Em mapas montados com nn.tilemap.new, apagar o tile apaga a colisão dele.
                if tile == 0
                    && let Some(ci) = code_layer(&state.map)
                    && ci != index
                {
                    state.map.set_tile(ci, c, r, 0);
                }
                sync_cell(&mut state, c, r);
                Ok(())
            },
        );

        m.add_method(
            "set_solid",
            |_, this, (col, row, kind): (i64, i64, Option<String>)| {
                let mut state = this.0.borrow_mut();
                let (c, r) = cell_from_lua(col, row, &state.map)?;
                let code = match kind.as_deref() {
                    None => 0,
                    Some("solid") => 1,
                    Some("one_way") => 2,
                    Some(other) => {
                        return Err(err(format!(
                            "tipo de colisão inválido: \"{other}\" (use \"solid\", \"one_way\" ou nil)"
                        )));
                    }
                };
                let ci = match code_layer(&state.map) {
                    Some(ci) => ci,
                    None => {
                        let (w, h) = (state.map.width, state.map.height);
                        let mut properties = Properties::new();
                        properties.insert("nn_collision_codes".into(), Property::Bool(true));
                        state.map.layers.push(nn_tilemap::Layer {
                            name: "colisao".into(),
                            tiles: vec![0; (w * h) as usize],
                            visible: false,
                            opacity: 1.0,
                            properties,
                        });
                        state.map.layers.len() - 1
                    }
                };
                state.map.set_tile(ci, c, r, code);
                sync_cell(&mut state, c, r);
                Ok(())
            },
        );

        m.add_method(
            "tile_properties",
            |lua, this, (col, row, layer): (i64, i64, Value)| {
                let state = this.0.borrow();
                let (c, r) = cell_from_lua(col, row, &state.map)?;
                let index = Self::layer_index(&state.map, &layer)?;
                match state.map.tile_properties(state.map.tile(index, c, r)) {
                    Some(p) => Ok(Value::Table(properties_table(lua, p)?)),
                    None => Ok(Value::Nil),
                }
            },
        );

        // Tiles (não vazios) das células que o retângulo do mundo toca, em
        // todas as camadas: { col, row, layer, tile, properties }.
        m.add_method(
            "tiles_in",
            |lua, this, (x0, y0, x1, y1): (f32, f32, f32, f32)| {
                let state = this.0.borrow();
                let map = &state.map;
                let out = lua.create_table()?;
                if map.width == 0 || map.height == 0 {
                    return Ok(out);
                }
                let (lo, hi) = (
                    Vec2::new(x0.min(x1), y0.min(y1)),
                    Vec2::new(x0.max(x1), y0.max(y1)),
                );
                let a = (lo - map.origin) / map.tile_size;
                let b = (hi - map.origin) / map.tile_size;
                let clamp = |v: f32, n: u32| (v.floor() as i64).clamp(0, n as i64 - 1) as u32;
                if b.x < 0.0 || b.y < 0.0 || a.x >= map.width as f32 || a.y >= map.height as f32 {
                    return Ok(out);
                }
                let (c0, c1) = (clamp(a.x, map.width), clamp(b.x, map.width));
                let (y0b, y1b) = (clamp(a.y, map.height), clamp(b.y, map.height));
                let mut n = 0;
                for (li, layer) in map.layers.iter().enumerate() {
                    if layer.properties.contains_key("nn_collision_codes") {
                        continue;
                    }
                    for yb in y0b..=y1b {
                        let row = map.height - 1 - yb;
                        for col in c0..=c1 {
                            let gid = map.tile(li, col, row);
                            if gid == 0 {
                                continue;
                            }
                            let t = lua.create_table()?;
                            t.set("col", col + 1)?;
                            t.set("row", row + 1)?;
                            t.set("layer", li + 1)?;
                            t.set("tile", gid)?;
                            if let Some(p) = map.tile_properties(gid) {
                                t.set("properties", properties_table(lua, p)?)?;
                            }
                            n += 1;
                            out.set(n, t)?;
                        }
                    }
                }
                Ok(out)
            },
        );

        m.add_method("solid", |_, this, (col, row): (i64, i64)| {
            let state = this.0.borrow();
            let (c, r) = cell_from_lua(col, row, &state.map)?;
            Ok(match state.map.collision_cell(c, r) {
                Cell::Solid => Some("solid"),
                Cell::OneWay => Some("one_way"),
                Cell::Empty => None,
            })
        });

        m.add_method("cell_at", |_, this, (x, y): (f32, f32)| {
            Ok(match this.0.borrow().map.cell_at(Vec2::new(x, y)) {
                Some((c, r)) => (Some(c + 1), Some(r + 1)),
                None => (None, None),
            })
        });

        m.add_method("cell_center", |_, this, (col, row): (i64, i64)| {
            let state = this.0.borrow();
            let (c, r) = cell_from_lua(col, row, &state.map)?;
            let p = state.map.cell_center(c, r);
            Ok((p.x, p.y))
        });
    }
}

/// Camada que guarda tipos de colisão diretamente (criada por `nn.tilemap.new` ou `set_solid`).
fn code_layer(map: &TileMap) -> Option<usize> {
    map.layers
        .iter()
        .position(|l| l.properties.contains_key("nn_collision_codes"))
}

/// Leva a colisão atual de uma célula para os mundos físicos.
fn sync_cell(state: &mut MapState, c: u32, r: u32) {
    let cell = state.map.collision_cell(c, r);
    let height = state.map.height;
    state.grids.retain(|(w, _)| w.strong_count() > 0);
    for (world, grid) in &state.grids {
        if let Some(world) = world.upgrade()
            && let Some(g) = world.borrow_mut().grid_mut(*grid)
        {
            g.set(c as i32, (height - 1 - r) as i32, cell);
        }
    }
}

fn texture_options(opts: Option<&Table>) -> Result<TextureOptions> {
    // Mapas de tiles costumam ser pixel art: nítido por padrão.
    Ok(if opt::<bool>(opts, "pixel_art")?.unwrap_or(true) {
        TextureOptions::PIXEL_ART
    } else {
        TextureOptions::default()
    })
}

fn apply_origin(map: &mut TileMap, opts: Option<&Table>) -> Result<()> {
    map.origin = Vec2::new(
        opt::<f32>(opts, "x")?.unwrap_or(0.0),
        opt::<f32>(opts, "y")?.unwrap_or(0.0),
    );
    // Objetos acompanham a origem.
    for o in &mut map.objects {
        o.position += map.origin;
    }
    Ok(())
}

pub(crate) fn table(lua: &Lua) -> Result<Table> {
    let t = lua.create_table()?;

    // nn.tilemap.load("fases/1.tmj", { scale = 2, pixel_art = true, x = 0, y = 0 })
    t.set(
        "load",
        lua.create_function(|lua, (path, opts): (String, Option<Table>)| {
            let o = opts.as_ref();
            let scale = opt::<f32>(o, "scale")?.unwrap_or(1.0);
            let options = texture_options(o)?;
            with_ctx(lua, |c| {
                let mut map = load_tiled(
                    Path::new(&path),
                    |p| c.assets.read_to_string(p).map_err(|e| e.to_string()),
                    scale,
                )
                .map_err(|e| err(e.to_string()))?;
                map.load_textures(|p| c.load_texture(p, options))
                    .map_err(|e| err(e.to_string()))?;
                apply_origin(&mut map, o)?;
                Ok(LTileMap::new(map))
            })
        })?,
    )?;

    // nn.tilemap.new({ sheet = ..., tile_size = 32, rows = {...}, legend = {...} })
    t.set(
        "new",
        lua.create_function(|lua, opts: Table| {
            let o = Some(&opts);
            let rows: Vec<String> = opts.get::<Option<Vec<String>>>("rows")?.ok_or_else(|| {
                err("nn.tilemap.new precisa de `rows` (lista de linhas de texto)")
            })?;
            let legend: Table = opts
                .get::<Option<Table>>("legend")?
                .ok_or_else(|| err("nn.tilemap.new precisa de `legend`"))?;
            let tile_w = opt::<f32>(o, "tile_size")?
                .or(opt::<f32>(o, "tile_width")?)
                .ok_or_else(|| err("nn.tilemap.new precisa de `tile_size`"))?;
            let tile_h = opt::<f32>(o, "tile_height")?.unwrap_or(tile_w);

            // Lê a legenda antes de montar o mapa (para relatar erros com clareza).
            let mut defs = std::collections::HashMap::new();
            for pair in legend.pairs::<String, Table>() {
                let (key, def) = pair.map_err(|e| err(format!("legenda inválida: {e}")))?;
                let mut chars = key.chars();
                let (Some(ch), None) = (chars.next(), chars.next()) else {
                    return Err(err(format!(
                        "as chaves da legenda devem ter um caractere só (recebeu \"{key}\")"
                    )));
                };
                let d = Some(&def);
                let cell = if opt::<bool>(d, "one_way")?.unwrap_or(false) {
                    Cell::OneWay
                } else if opt::<bool>(d, "solid")?.unwrap_or(false) {
                    Cell::Solid
                } else {
                    Cell::Empty
                };
                let tile = if let Some(kind) = opt::<String>(d, "object")? {
                    AsciiTile::Object(kind)
                } else if let Some(frame) = opt::<i64>(d, "tile")? {
                    if frame < 1 {
                        return Err(err("tiles da legenda começam em 1"));
                    }
                    AsciiTile::Tile {
                        frame: frame as u32 - 1,
                        cell,
                    }
                } else {
                    AsciiTile::Collision(cell)
                };
                defs.insert(ch, tile);
            }

            let row_refs: Vec<&str> = rows.iter().map(String::as_str).collect();
            let mut map = TileMap::from_ascii(&row_refs, Vec2::new(tile_w, tile_h), |c| {
                defs.get(&c).cloned()
            });
            if let Some(sheet) = opt::<AnyUserData>(o, "sheet")? {
                let sheet = sheet
                    .borrow::<LSheet>()
                    .map_err(|_| err("`sheet` deve ser uma spritesheet (nn.assets.sheet)"))?
                    .0;
                let size = with_ctx(lua, |c| Ok(c.renderer.texture_size(sheet.texture())))?;
                map.add_sheet(&sheet, size);
            }
            apply_origin(&mut map, o)?;
            Ok(LTileMap::new(map))
        })?,
    )?;

    Ok(t)
}
