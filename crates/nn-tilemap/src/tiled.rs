//! Importação de mapas do editor Tiled (formato JSON: `.tmj` e `.tsj`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use glam::{UVec2, Vec2};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    Layer, MapObject, Properties, Property, TileAnimation, TileMap, TilemapError, Tileset,
    join_normalized,
};

#[derive(Deserialize)]
struct JMap {
    width: u32,
    height: u32,
    tilewidth: u32,
    tileheight: u32,
    #[serde(default)]
    orientation: String,
    #[serde(default)]
    infinite: bool,
    #[serde(default)]
    layers: Vec<JLayer>,
    #[serde(default)]
    tilesets: Vec<JTileset>,
    #[serde(default)]
    properties: Vec<JProperty>,
}

fn one() -> f32 {
    1.0
}

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
struct JLayer {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    data: Option<Value>,
    #[serde(default = "one")]
    opacity: f32,
    #[serde(default = "yes")]
    visible: bool,
    #[serde(default)]
    objects: Vec<JObject>,
    #[serde(default)]
    layers: Vec<JLayer>,
    #[serde(default)]
    properties: Vec<JProperty>,
}

#[derive(Deserialize)]
struct JObject {
    #[serde(default)]
    id: u32,
    #[serde(default)]
    name: String,
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    class: String,
    x: f32,
    y: f32,
    #[serde(default)]
    width: f32,
    #[serde(default)]
    height: f32,
    #[serde(default)]
    point: bool,
    #[serde(default)]
    gid: Option<u32>,
    #[serde(default)]
    properties: Vec<JProperty>,
}

#[derive(Deserialize)]
struct JTileset {
    #[serde(default)]
    firstgid: u32,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    name: String,
    #[serde(default)]
    image: Option<String>,
    #[serde(default)]
    imagewidth: u32,
    #[serde(default)]
    imageheight: u32,
    #[serde(default)]
    tilewidth: u32,
    #[serde(default)]
    tileheight: u32,
    #[serde(default)]
    columns: u32,
    #[serde(default)]
    tilecount: u32,
    #[serde(default)]
    margin: u32,
    #[serde(default)]
    spacing: u32,
    #[serde(default)]
    tiles: Vec<JTile>,
}

#[derive(Deserialize)]
struct JTile {
    id: u32,
    #[serde(default)]
    properties: Vec<JProperty>,
    #[serde(default)]
    animation: Vec<JFrame>,
}

#[derive(Deserialize)]
struct JFrame {
    tileid: u32,
    /// Milissegundos.
    duration: u32,
}

#[derive(Deserialize)]
struct JProperty {
    name: String,
    #[serde(default, rename = "type")]
    kind: String,
    value: Value,
}

fn properties(list: &[JProperty]) -> Properties {
    list.iter()
        .map(|p| {
            let value = match (p.kind.as_str(), &p.value) {
                ("bool", Value::Bool(b)) => Property::Bool(*b),
                ("int", v) if v.is_i64() => Property::Int(v.as_i64().unwrap_or_default()),
                ("float", v) if v.is_number() => Property::Float(v.as_f64().unwrap_or_default()),
                (_, Value::Bool(b)) => Property::Bool(*b),
                (_, Value::String(s)) => Property::String(s.clone()),
                (_, v) if v.is_i64() => Property::Int(v.as_i64().unwrap_or_default()),
                (_, v) if v.is_number() => Property::Float(v.as_f64().unwrap_or_default()),
                (_, v) => Property::String(v.to_string()),
            };
            (p.name.clone(), value)
        })
        .collect()
}

fn unsupported(path: &Path, message: impl Into<String>) -> TilemapError {
    TilemapError::Unsupported {
        path: path.to_owned(),
        message: message.into(),
    }
}

fn parse<T: for<'de> Deserialize<'de>>(path: &Path, text: &str) -> Result<T, TilemapError> {
    serde_json::from_str(text).map_err(|source| TilemapError::Json {
        path: path.to_owned(),
        source,
    })
}

/// Carrega um mapa do Tiled salvo em JSON.
///
/// - `map_path`: caminho do `.tmj`, relativo à pasta de assets;
/// - `read`: lê um arquivo de texto da pasta de assets (o mapa e tilesets externos);
/// - `scale`: multiplica o tamanho em pixels (2.0 = tiles com o dobro do tamanho no mundo).
///
/// As imagens dos tilesets não são carregadas aqui; use
/// [`TileMap::load_textures`] depois.
pub fn load_tiled(
    map_path: &Path,
    mut read: impl FnMut(&Path) -> Result<String, String>,
    scale: f32,
) -> Result<TileMap, TilemapError> {
    let io = |path: &Path, message: String| TilemapError::Io {
        path: path.to_owned(),
        message,
    };
    if map_path.extension().is_some_and(|e| e == "tmx") {
        return Err(unsupported(
            map_path,
            "mapas .tmx (XML) não são suportados; no Tiled, use Arquivo > Exportar como... > JSON (.tmj)",
        ));
    }
    let text = read(map_path).map_err(|e| io(map_path, e))?;
    let j: JMap = parse(map_path, &text)?;
    if j.infinite {
        return Err(unsupported(
            map_path,
            "mapas infinitos não são suportados; desmarque \"Infinito\" nas propriedades do mapa",
        ));
    }
    if !j.orientation.is_empty() && j.orientation != "orthogonal" {
        return Err(unsupported(
            map_path,
            format!(
                "orientação \"{}\" não suportada (use ortogonal)",
                j.orientation
            ),
        ));
    }
    if j.tilewidth == 0 || j.tileheight == 0 {
        return Err(TilemapError::Invalid(format!(
            "{}: tamanho de tile inválido",
            map_path.display()
        )));
    }

    let map_dir = map_path.parent().unwrap_or(Path::new("")).to_path_buf();
    let scale = if scale > 0.0 { scale } else { 1.0 };

    let mut tilesets = Vec::new();
    for t in j.tilesets {
        let first_gid = t.firstgid;
        let (t, base_dir, source_path) = match &t.source {
            Some(source) => {
                let path = join_normalized(&map_dir, Path::new(source));
                if path.extension().is_some_and(|e| e == "tsx") {
                    return Err(unsupported(
                        &path,
                        "tilesets .tsx (XML) não são suportados; exporte o tileset como JSON (.tsj) ou embuta-o no mapa",
                    ));
                }
                let text = read(&path).map_err(|e| io(&path, e))?;
                let ext: JTileset = parse(&path, &text)?;
                let dir = path.parent().unwrap_or(Path::new("")).to_path_buf();
                (ext, dir, path)
            }
            None => (t, map_dir.clone(), map_path.to_path_buf()),
        };
        let Some(image) = &t.image else {
            return Err(unsupported(
                &source_path,
                format!(
                    "o tileset \"{}\" é uma coleção de imagens; use um tileset baseado em uma única imagem",
                    t.name
                ),
            ));
        };
        if t.tilewidth == 0 || t.tileheight == 0 {
            return Err(TilemapError::Invalid(format!(
                "{}: tileset \"{}\" sem tamanho de tile",
                source_path.display(),
                t.name
            )));
        }
        let columns = if t.columns > 0 {
            t.columns
        } else {
            (t.imagewidth.saturating_sub(2 * t.margin) + t.spacing) / (t.tilewidth + t.spacing)
        };
        let tile_count = if t.tilecount > 0 {
            t.tilecount
        } else {
            let rows = (t.imageheight.saturating_sub(2 * t.margin) + t.spacing)
                / (t.tileheight + t.spacing);
            columns * rows
        };
        tilesets.push(Tileset {
            name: t.name.clone(),
            first_gid,
            image: join_normalized(&base_dir, Path::new(image)),
            image_size: UVec2::new(t.imagewidth, t.imageheight),
            tile_size: UVec2::new(t.tilewidth, t.tileheight),
            columns,
            tile_count,
            margin: t.margin,
            spacing: t.spacing,
            tile_properties: t
                .tiles
                .iter()
                .filter(|tile| !tile.properties.is_empty())
                .map(|tile| (tile.id, properties(&tile.properties)))
                .collect::<BTreeMap<_, _>>(),
            animations: t
                .tiles
                .iter()
                .filter(|tile| !tile.animation.is_empty())
                .map(|tile| {
                    let frames = tile
                        .animation
                        .iter()
                        .map(|f| (f.tileid, f.duration as f32 / 1000.0))
                        .collect();
                    (tile.id, TileAnimation { frames })
                })
                .collect(),
            texture: None,
        });
    }
    tilesets.sort_by_key(|t| t.first_gid);

    let mut map = TileMap {
        width: j.width,
        height: j.height,
        tile_size: Vec2::new(j.tilewidth as f32, j.tileheight as f32) * scale,
        tile_px: Vec2::new(j.tilewidth as f32, j.tileheight as f32),
        origin: Vec2::ZERO,
        layers: Vec::new(),
        tilesets,
        objects: Vec::new(),
        properties: properties(&j.properties),
        time: 0.0,
    };

    let height_px = (j.height * j.tileheight) as f32;
    let mut flat = Vec::new();
    flatten(j.layers, true, &mut flat);
    for (layer, visible) in flat {
        match layer.kind.as_str() {
            "tilelayer" => {
                let data = match layer.data {
                    Some(Value::Array(values)) => values
                        .iter()
                        .map(|v| v.as_u64().unwrap_or(0) as u32)
                        .collect::<Vec<u32>>(),
                    Some(Value::String(_)) => {
                        return Err(unsupported(
                            map_path,
                            format!(
                                "a camada \"{}\" está compactada (base64); nas propriedades do mapa no Tiled, mude o formato da camada de tiles para CSV",
                                layer.name
                            ),
                        ));
                    }
                    _ => vec![0; (j.width * j.height) as usize],
                };
                if data.len() != (j.width * j.height) as usize {
                    return Err(TilemapError::Invalid(format!(
                        "{}: a camada \"{}\" tem {} tiles, esperava {}",
                        map_path.display(),
                        layer.name,
                        data.len(),
                        j.width * j.height
                    )));
                }
                map.layers.push(Layer {
                    name: layer.name,
                    tiles: data,
                    visible,
                    opacity: layer.opacity,
                    properties: properties(&layer.properties),
                });
            }
            "objectgroup" => {
                for o in layer.objects {
                    // Objetos de tile têm y na base; os demais, no topo.
                    let top = if o.gid.is_some() { o.y - o.height } else { o.y };
                    let center_px =
                        Vec2::new(o.x + o.width / 2.0, height_px - (top + o.height / 2.0));
                    map.objects.push(MapObject {
                        id: o.id,
                        name: o.name,
                        kind: if o.class.is_empty() { o.kind } else { o.class },
                        position: center_px * scale,
                        size: Vec2::new(o.width, o.height) * scale,
                        point: o.point,
                        layer: layer.name.clone(),
                        properties: properties(&o.properties),
                    });
                }
            }
            other => log::debug!("camada \"{}\" do tipo {other} ignorada", layer.name),
        }
    }
    Ok(map)
}

/// Abre grupos de camadas, mantendo a ordem e a visibilidade herdada.
fn flatten(layers: Vec<JLayer>, parent_visible: bool, out: &mut Vec<(JLayer, bool)>) {
    for mut layer in layers {
        let visible = parent_visible && layer.visible;
        if layer.kind == "group" {
            let children = std::mem::take(&mut layer.layers);
            flatten(children, visible, out);
        } else {
            out.push((layer, visible));
        }
    }
}

/// Lê um arquivo da pasta `root` (para usar com [`load_tiled`] fora da engine).
pub fn read_from(root: impl Into<PathBuf>) -> impl FnMut(&Path) -> Result<String, String> {
    let root = root.into();
    move |p: &Path| std::fs::read_to_string(root.join(p)).map_err(|e| e.to_string())
}
