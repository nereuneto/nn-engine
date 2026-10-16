//! Tileset do Tiled (`.tsj`, JSON): a imagem dividida em tiles e o que o
//! NN Studio sabe de cada tile (sólido, plataforma, tipo, animação) e os
//! terrenos do autotile (conjuntos Wang de cantos, com 16 peças por par de
//! terrenos). O que o editor não conhece fica intocado no arquivo.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Caminhos relativos (com "/").

/// Pasta de um caminho relativo ("mapas/fase.tmj" → "mapas").
pub fn dir_of(rel: &str) -> &str {
    rel.rfind('/').map_or("", |i| &rel[..i])
}

/// Junta e resolve `..` ("mapas" + "../tiles/a.tsj" → "tiles/a.tsj").
pub fn join(dir: &str, path: &str) -> String {
    let mut parts: Vec<&str> = if path.starts_with('/') {
        Vec::new()
    } else {
        dir.split('/').filter(|s| !s.is_empty()).collect()
    };
    for p in path.split(['/', '\\']) {
        match p {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// Caminho de `target` visto da pasta `from_dir` ("mapas", "tiles/a.tsj" →
/// "../tiles/a.tsj").
pub fn relative(from_dir: &str, target: &str) -> String {
    let a: Vec<&str> = from_dir.split('/').filter(|s| !s.is_empty()).collect();
    let b: Vec<&str> = target.split('/').filter(|s| !s.is_empty()).collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let mut out: Vec<&str> = std::iter::repeat_n("..", a.len() - common).collect();
    out.extend(&b[common..]);
    out.join("/")
}

// ---------------------------------------------------------------------------

/// Um quadro de animação de tile: (id do tile, duração em milissegundos).
pub type AnimFrame = (u32, u32);

/// O que o editor guarda de um tile.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TileInfo {
    pub solid: bool,
    pub one_way: bool,
    /// Tipo livre ("lava", "agua"...), para os eventos.
    pub kind: String,
    /// Chance de ser sorteado entre variações (o Tiled usa 1 como padrão).
    pub probability: Option<f64>,
    pub animation: Vec<AnimFrame>,
}

impl TileInfo {
    fn is_empty(&self) -> bool {
        *self == TileInfo::default()
    }
}

/// Uma cor (terreno) de um conjunto de terrenos.
#[derive(Debug, Clone, PartialEq)]
pub struct WangColor {
    pub name: String,
    /// "#RRGGBB" (só para mostrar no editor).
    pub color: String,
}

/// Conjunto de terrenos de cantos (Wang "corner" no Tiled): cada tile diz
/// qual terreno está em cada canto (cima-esquerda, cima-direita,
/// baixo-direita, baixo-esquerda); 0 = nenhum.
#[derive(Debug, Clone, PartialEq)]
pub struct WangSet {
    pub name: String,
    pub colors: Vec<WangColor>,
    /// Tile → cantos [TL, TR, BR, BL] (cores a partir de 1; 0 = nenhuma).
    pub tiles: BTreeMap<u32, [u8; 4]>,
}

impl WangSet {
    /// Tiles com exatamente estes cantos.
    pub fn matching(&self, corners: [u8; 4]) -> Vec<u32> {
        self.tiles
            .iter()
            .filter(|(_, c)| **c == corners)
            .map(|(t, _)| *t)
            .collect()
    }

    /// Quantas das 16 combinações de dois terrenos (`a` e `b`) têm tile.
    pub fn coverage(&self, a: u8, b: u8) -> usize {
        (0..16u8)
            .filter(|m| {
                let c = corners_of(*m, a, b);
                self.tiles.values().any(|v| *v == c)
            })
            .count()
    }
}

/// Combinação `mask` (bits TL=8, TR=4, BR=2, BL=1: terreno `b`; senão `a`).
pub fn corners_of(mask: u8, a: u8, b: u8) -> [u8; 4] {
    let bit = |m: u8| if mask & m != 0 { b } else { a };
    [bit(8), bit(4), bit(2), bit(1)]
}

#[derive(Debug, Clone, PartialEq)]
pub struct TilesetData {
    /// O JSON original (para não perder o que o editor não usa).
    json: Value,
    pub name: String,
    /// Imagem, relativa à pasta do projeto.
    pub image: String,
    pub image_w: u32,
    pub image_h: u32,
    pub tile_w: u32,
    pub tile_h: u32,
    pub columns: u32,
    pub count: u32,
    pub margin: u32,
    pub spacing: u32,
    pub tiles: BTreeMap<u32, TileInfo>,
    pub wangsets: Vec<WangSet>,
}

fn u(v: &Value, k: &str) -> u32 {
    v.get(k).and_then(Value::as_u64).unwrap_or(0) as u32
}

fn prop_list(v: &Value) -> Vec<Value> {
    v.get("properties")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn prop_bool(list: &[Value], name: &str) -> bool {
    list.iter()
        .find(|p| p.get("name").and_then(Value::as_str) == Some(name))
        .and_then(|p| p.get("value"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn prop_str(list: &[Value], name: &str) -> String {
    list.iter()
        .find(|p| p.get("name").and_then(Value::as_str) == Some(name))
        .and_then(|p| p.get("value"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

/// Propriedades que o editor controla (as outras ficam como estão).
const OWN_PROPS: [&str; 3] = ["solid", "one_way", "tipo"];

impl TilesetData {
    /// Um tileset novo para uma imagem.
    pub fn new(name: &str, image: &str, image_size: (u32, u32), tile_w: u32, tile_h: u32) -> Self {
        let tile_w = tile_w.max(1);
        let tile_h = tile_h.max(1);
        let columns = (image_size.0 / tile_w).max(1);
        let rows = (image_size.1 / tile_h).max(1);
        Self {
            json: json!({ "type": "tileset", "version": "1.10", "tiledversion": "1.10.2" }),
            name: name.to_string(),
            image: image.to_string(),
            image_w: image_size.0,
            image_h: image_size.1,
            tile_w,
            tile_h,
            columns,
            count: columns * rows,
            margin: 0,
            spacing: 0,
            tiles: BTreeMap::new(),
            wangsets: Vec::new(),
        }
    }

    /// Lê o JSON de um tileset. `dir` é a pasta onde ele está (para achar a
    /// imagem).
    pub fn from_json(v: Value, dir: &str) -> Result<Self, String> {
        let image = v
            .get("image")
            .and_then(Value::as_str)
            .ok_or("o tileset não tem imagem (tilesets de várias imagens não são suportados)")?;
        let mut tiles = BTreeMap::new();
        for t in v
            .get("tiles")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let id = u(t, "id");
            let props = prop_list(t);
            let info = TileInfo {
                solid: prop_bool(&props, "solid"),
                one_way: prop_bool(&props, "one_way"),
                kind: prop_str(&props, "tipo"),
                probability: t.get("probability").and_then(Value::as_f64),
                animation: t
                    .get("animation")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .map(|f| (u(f, "tileid"), u(f, "duration")))
                    .collect(),
            };
            if !info.is_empty() {
                tiles.insert(id, info);
            }
        }
        let mut wangsets = Vec::new();
        for w in v
            .get("wangsets")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let colors = w
                .get("colors")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|c| WangColor {
                    name: c
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    color: c
                        .get("color")
                        .and_then(Value::as_str)
                        .unwrap_or("#ffffff")
                        .to_string(),
                })
                .collect();
            let mut map = BTreeMap::new();
            for wt in w
                .get("wangtiles")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let id = u(wt, "tileid");
                let ids: Vec<u8> = wt
                    .get("wangid")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .map(|x| x.as_u64().unwrap_or(0) as u8)
                    .collect();
                if ids.len() == 8 {
                    // Ordem do Tiled: cima, cima-dir., dir., baixo-dir., baixo,
                    // baixo-esq., esq., cima-esq. Os cantos são 7, 1, 3, 5.
                    map.insert(id, [ids[7], ids[1], ids[3], ids[5]]);
                }
            }
            wangsets.push(WangSet {
                name: w
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                colors,
                tiles: map,
            });
        }
        Ok(Self {
            name: v
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            image: join(dir, image),
            image_w: u(&v, "imagewidth"),
            image_h: u(&v, "imageheight"),
            tile_w: u(&v, "tilewidth").max(1),
            tile_h: u(&v, "tileheight").max(1),
            columns: u(&v, "columns").max(1),
            count: u(&v, "tilecount"),
            margin: u(&v, "margin"),
            spacing: u(&v, "spacing"),
            tiles,
            wangsets,
            json: v,
        })
    }

    pub fn load(root: &Path, rel: &str) -> Result<Self, String> {
        let text = std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
        let v: Value = serde_json::from_str(&text).map_err(|e| format!("{rel}: {e}"))?;
        Self::from_json(v, dir_of(rel)).map_err(|e| format!("{rel}: {e}"))
    }

    /// O JSON, com o que o editor mudou. `dir` é a pasta onde ele vai ficar.
    pub fn to_json(&self, dir: &str) -> Value {
        let mut v = self.json.clone();
        if !v.is_object() {
            v = json!({});
        }
        let o = v.as_object_mut().unwrap();
        o.insert("name".into(), json!(self.name));
        o.insert("image".into(), json!(relative(dir, &self.image)));
        o.insert("imagewidth".into(), json!(self.image_w));
        o.insert("imageheight".into(), json!(self.image_h));
        o.insert("tilewidth".into(), json!(self.tile_w));
        o.insert("tileheight".into(), json!(self.tile_h));
        o.insert("columns".into(), json!(self.columns));
        o.insert("tilecount".into(), json!(self.count));
        o.insert("margin".into(), json!(self.margin));
        o.insert("spacing".into(), json!(self.spacing));
        // Tiles: junta o que já estava (propriedades de outros, classe...) com
        // o que o editor sabe.
        let old: BTreeMap<u32, Value> = self
            .json
            .get("tiles")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|t| (u(t, "id"), t.clone()))
            .collect();
        let mut ids: Vec<u32> = old
            .keys()
            .copied()
            .chain(self.tiles.keys().copied())
            .collect();
        ids.sort();
        ids.dedup();
        let mut tiles = Vec::new();
        for id in ids {
            let mut t = old.get(&id).cloned().unwrap_or_else(|| json!({ "id": id }));
            let info = self.tiles.get(&id).cloned().unwrap_or_default();
            let to = t.as_object_mut().unwrap();
            let mut props: Vec<Value> = prop_list(&Value::Object(to.clone()))
                .into_iter()
                .filter(|p| {
                    !OWN_PROPS.contains(&p.get("name").and_then(Value::as_str).unwrap_or(""))
                })
                .collect();
            if info.solid {
                props.push(json!({ "name": "solid", "type": "bool", "value": true }));
            }
            if info.one_way {
                props.push(json!({ "name": "one_way", "type": "bool", "value": true }));
            }
            if !info.kind.is_empty() {
                props.push(json!({ "name": "tipo", "type": "string", "value": info.kind }));
            }
            if props.is_empty() {
                to.remove("properties");
            } else {
                to.insert("properties".into(), Value::Array(props));
            }
            match info.probability {
                Some(p) => to.insert("probability".into(), json!(p)),
                None => to.remove("probability"),
            };
            if info.animation.is_empty() {
                to.remove("animation");
            } else {
                to.insert(
                    "animation".into(),
                    Value::Array(
                        info.animation
                            .iter()
                            .map(|(t, d)| json!({ "tileid": t, "duration": d }))
                            .collect(),
                    ),
                );
            }
            // Só o id: não precisa ficar no arquivo.
            if to.len() > 1 {
                tiles.push(t);
            }
        }
        if tiles.is_empty() {
            o.remove("tiles");
        } else {
            o.insert("tiles".into(), Value::Array(tiles));
        }
        let old_ws: Vec<Value> = self
            .json
            .get("wangsets")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let wangsets: Vec<Value> = self
            .wangsets
            .iter()
            .enumerate()
            .map(|(i, w)| {
                let mut wv = old_ws.get(i).cloned().unwrap_or_else(|| json!({}));
                let wo = wv.as_object_mut().unwrap();
                wo.insert("name".into(), json!(w.name));
                wo.insert("type".into(), json!("corner"));
                wo.entry("tile").or_insert(json!(-1));
                wo.insert(
                    "colors".into(),
                    Value::Array(
                        w.colors
                            .iter()
                            .map(|c| {
                                json!({ "name": c.name, "color": c.color, "tile": -1, "probability": 1 })
                            })
                            .collect(),
                    ),
                );
                wo.insert(
                    "wangtiles".into(),
                    Value::Array(
                        w.tiles
                            .iter()
                            .map(|(t, c)| {
                                json!({ "tileid": t, "wangid": [0, c[1], 0, c[2], 0, c[3], 0, c[0]] })
                            })
                            .collect(),
                    ),
                );
                wv
            })
            .collect();
        if wangsets.is_empty() {
            o.remove("wangsets");
        } else {
            o.insert("wangsets".into(), Value::Array(wangsets));
        }
        v
    }

    pub fn text(&self, dir: &str) -> String {
        serde_json::to_string_pretty(&self.to_json(dir)).unwrap_or_default() + "\n"
    }

    pub fn rows(&self) -> u32 {
        self.count.div_ceil(self.columns.max(1))
    }

    /// Pixels do tile `id` na imagem: (x, y).
    pub fn tile_origin(&self, id: u32) -> (u32, u32) {
        let col = id % self.columns.max(1);
        let row = id / self.columns.max(1);
        (
            self.margin + col * (self.tile_w + self.spacing),
            self.margin + row * (self.tile_h + self.spacing),
        )
    }

    pub fn info(&self, id: u32) -> TileInfo {
        self.tiles.get(&id).cloned().unwrap_or_default()
    }

    pub fn set_info(&mut self, id: u32, info: TileInfo) {
        if info.is_empty() {
            self.tiles.remove(&id);
        } else {
            self.tiles.insert(id, info);
        }
    }

    /// A imagem mudou de tamanho (ganhou linhas): recalcula as contas.
    pub fn image_resized(&mut self, w: u32, h: u32) {
        self.image_w = w;
        self.image_h = h;
        let usable = |total: u32, tile: u32| {
            (total.saturating_sub(2 * self.margin) + self.spacing) / (tile + self.spacing).max(1)
        };
        self.columns = usable(w, self.tile_w).max(1);
        self.count = self.columns * usable(h, self.tile_h);
    }

    /// Tipos usados nos tiles ("lava", "agua"...), para as listas do editor.
    pub fn kinds(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .tiles
            .values()
            .filter(|t| !t.kind.is_empty())
            .map(|t| t.kind.clone())
            .collect();
        out.sort();
        out.dedup();
        out
    }
}

/// Caminho do `.tsj` de uma imagem (`tiles/terreno.png` → `tiles/terreno.tsj`).
pub fn tsj_for(image: &str) -> String {
    let slash = image.rfind('/').map_or(0, |i| i + 1);
    match image[slash..].rfind('.') {
        Some(d) => format!("{}.tsj", &image[..slash + d]),
        None => format!("{image}.tsj"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tileset_dos_modelos_ja_esta_no_formato_do_editor() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../nn-run/modelos");
        let rel = "comum/imagens/terreno.tsj";
        let text = std::fs::read_to_string(root.join(rel)).unwrap();
        let doc = TilesetDoc::open(&root, rel).unwrap();
        assert_eq!(doc.text(), text);
    }

    #[test]
    fn caminhos() {
        assert_eq!(dir_of("mapas/fase.tmj"), "mapas");
        assert_eq!(dir_of("fase.tmj"), "");
        assert_eq!(join("mapas", "../tiles/a.tsj"), "tiles/a.tsj");
        assert_eq!(join("", "./a.png"), "a.png");
        assert_eq!(relative("mapas", "tiles/a.tsj"), "../tiles/a.tsj");
        assert_eq!(relative("tiles", "tiles/a.png"), "a.png");
        assert_eq!(relative("", "a/b.png"), "a/b.png");
        assert_eq!(tsj_for("tiles/terreno.png"), "tiles/terreno.tsj");
    }

    #[test]
    fn le_e_grava_sem_perder_o_resto() {
        let v: Value = serde_json::from_str(
            r##"{ "name": "t", "image": "t.png", "imagewidth": 32, "imageheight": 16,
              "tilewidth": 8, "tileheight": 8, "columns": 4, "tilecount": 8, "margin": 0, "spacing": 0,
              "class": "meu", "tiles": [
                { "id": 1, "properties": [ { "name": "solid", "type": "bool", "value": true },
                                           { "name": "dano", "type": "int", "value": 2 } ] },
                { "id": 3, "animation": [ { "tileid": 3, "duration": 100 }, { "tileid": 4, "duration": 200 } ] } ],
              "wangsets": [ { "name": "chão", "type": "corner", "tile": -1, "properties": [ { "name": "x", "type": "int", "value": 1 } ],
                "colors": [ { "name": "grama", "color": "#00ff00", "tile": -1, "probability": 1 } ],
                "wangtiles": [ { "tileid": 0, "wangid": [0, 1, 0, 1, 0, 1, 0, 1] },
                               { "tileid": 2, "wangid": [0, 0, 0, 1, 0, 1, 0, 0] } ] } ] }"##,
        )
        .unwrap();
        let mut t = TilesetData::from_json(v, "tiles").unwrap();
        assert_eq!(t.image, "tiles/t.png");
        assert!(t.info(1).solid);
        assert_eq!(t.info(3).animation, vec![(3, 100), (4, 200)]);
        assert_eq!(t.wangsets[0].tiles[&0], [1, 1, 1, 1]);
        assert_eq!(t.wangsets[0].tiles[&2], [0, 0, 1, 1], "embaixo: grama");
        t.set_info(
            5,
            TileInfo {
                kind: "lava".into(),
                one_way: true,
                ..Default::default()
            },
        );
        let mut i1 = t.info(1);
        i1.solid = false;
        t.set_info(1, i1);
        let out = t.to_json("tiles");
        assert_eq!(out["class"], "meu");
        assert_eq!(out["image"], "t.png");
        let tiles = out["tiles"].as_array().unwrap();
        let t1 = tiles.iter().find(|x| x["id"] == 1).unwrap();
        // A propriedade "dano" (que o editor não conhece) continua; "solid" saiu.
        assert_eq!(t1["properties"].as_array().unwrap().len(), 1);
        assert_eq!(t1["properties"][0]["name"], "dano");
        assert!(tiles.iter().any(|x| x["id"] == 5));
        assert_eq!(out["wangsets"][0]["properties"][0]["name"], "x");
        let back = TilesetData::from_json(out, "tiles").unwrap();
        assert_eq!(back.tiles, t.tiles);
        assert_eq!(back.wangsets, t.wangsets);
        assert_eq!(back.kinds(), ["lava"]);
    }

    #[test]
    fn combinacoes_de_cantos() {
        assert_eq!(corners_of(0, 1, 2), [1, 1, 1, 1]);
        assert_eq!(corners_of(15, 1, 2), [2, 2, 2, 2]);
        assert_eq!(corners_of(0b1001, 0, 1), [1, 0, 0, 1]);
    }
}

/// Um tileset `.tsj` aberto no editor (para marcar tiles e terrenos).
#[derive(Debug, Clone)]
pub struct TilesetDoc {
    pub rel: String,
    pub data: TilesetData,
    saved: TilesetData,
    /// Muda a cada alteração.
    pub rev: u64,
}

impl TilesetDoc {
    pub fn open(root: &Path, rel: &str) -> Result<Self, String> {
        let data = TilesetData::load(root, rel)?;
        Ok(Self::from_data(rel, data))
    }

    pub fn from_data(rel: &str, data: TilesetData) -> Self {
        Self {
            rel: rel.to_string(),
            saved: data.clone(),
            data,
            rev: 1,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.data != self.saved
    }

    /// Avisa que mudou (para desenhar de novo).
    pub fn touch(&mut self) {
        self.rev += 1;
    }

    pub fn save(&mut self, root: &Path) -> Result<(), String> {
        let path = root.join(&self.rel);
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, self.data.text(dir_of(&self.rel))).map_err(|e| e.to_string())?;
        self.saved = self.data.clone();
        Ok(())
    }

    /// O texto como vai para o disco (para desenhar o mapa sem salvar).
    pub fn text(&self) -> String {
        self.data.text(dir_of(&self.rel))
    }
}
