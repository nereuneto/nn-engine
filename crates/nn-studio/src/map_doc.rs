//! Mapa do Tiled (`.tmj`) aberto no editor: camadas de tiles para pintar,
//! crescer o mapa, desfazer e autotile de terrenos. O resto do arquivo
//! (objetos, grupos, propriedades) fica como estava.

use std::path::Path;
use std::time::SystemTime;

use serde_json::{Value, json};

use crate::tileset::{TilesetData, WangSet, dir_of, join, relative};

pub use nn_tilemap::{FLIP_D, FLIP_H, FLIP_V};

/// Bits que não fazem parte do número do tile.
pub const FLIP_BITS: u32 = FLIP_H | FLIP_V | FLIP_D | 0x1000_0000;

/// Maior tamanho (em tiles) que o mapa pode crescer pintando.
pub const MAX_SIZE: u32 = 1000;

#[derive(Debug, Clone, PartialEq)]
pub enum TsKind {
    /// Arquivo `.tsj` (caminho relativo ao projeto).
    External(String),
    /// Tileset dentro do próprio mapa (mapas feitos no Tiled).
    Embedded(TilesetData),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TsRef {
    pub first_gid: u32,
    pub kind: TsKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TileLayer {
    pub name: String,
    pub data: Vec<u32>,
    pub visible: bool,
    pub opacity: f64,
    /// Só no editor: não deixa pintar.
    pub locked: bool,
    /// Camada de colisão: todos os tiles bloqueiam (propriedade `collision`).
    pub collision: bool,
    json: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    Tiles(TileLayer),
    /// Camada que o editor não mexe (objetos, imagem, grupo).
    Other(Value),
}

#[derive(Debug, Clone)]
struct Snap {
    width: u32,
    height: u32,
    entries: Vec<Entry>,
    shift: (i32, i32),
    grown: (i64, i64),
    rev: u64,
}

fn next_rev() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    N.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone)]
pub struct MapDoc {
    pub rel: String,
    json: Value,
    pub width: u32,
    pub height: u32,
    pub tile_w: u32,
    pub tile_h: u32,
    pub entries: Vec<Entry>,
    pub tilesets: Vec<TsRef>,
    /// Camada em que se pinta (índice em `entries`).
    pub layer: usize,
    history: Vec<Snap>,
    future: Vec<Snap>,
    rev: u64,
    saved_rev: u64,
    /// Quantas colunas (à esquerda) e linhas (embaixo) o mapa ganhou desde o
    /// começo: o objeto do mapa na cena anda o mesmo tanto para trás.
    shift: (i32, i32),
    /// Parte de `shift` que a cena ainda não aplicou.
    pending: (i32, i32),
    disk_mtime: Option<SystemTime>,
    /// Número para sortear variações (muda a cada pincelada).
    seed: u64,
    /// Colunas e linhas ganhas à esquerda e em cima no último `grow_to`.
    pub last_grow: (i64, i64),
    /// Total ganho à esquerda e em cima (para a aba do mapa manter o desenho
    /// no lugar).
    pub grown_left_top: (i64, i64),
}

fn u(v: &Value, k: &str) -> u32 {
    v.get(k).and_then(Value::as_u64).unwrap_or(0) as u32
}

fn layer_collision(v: &Value) -> bool {
    v.get("properties")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|p| {
            p.get("name").and_then(Value::as_str) == Some("collision")
                && p.get("value") == Some(&json!(true))
        })
}

impl MapDoc {
    /// Mapa novo, vazio, com uma camada.
    pub fn new(
        rel: &str,
        width: u32,
        height: u32,
        tile_w: u32,
        tile_h: u32,
        tileset: &str,
    ) -> Self {
        let json = json!({
            "type": "map", "version": "1.10", "tiledversion": "1.10.2",
            "orientation": "orthogonal", "renderorder": "right-down", "infinite": false,
            "width": width, "height": height, "tilewidth": tile_w, "tileheight": tile_h,
            "nextlayerid": 1, "nextobjectid": 1, "layers": [],
            "tilesets": [ { "firstgid": 1, "source": relative(dir_of(rel), tileset) } ],
        });
        let mut doc = Self::from_json(json, rel).expect("mapa novo");
        doc.add_layer("chao");
        doc.history.clear();
        doc.saved_rev = doc.rev;
        doc
    }

    pub fn from_json(json: Value, rel: &str) -> Result<Self, String> {
        if json.get("infinite").and_then(Value::as_bool) == Some(true) {
            return Err("mapas infinitos do Tiled não são suportados (desmarque \"Infinito\" nas propriedades do mapa)".into());
        }
        let width = u(&json, "width");
        let height = u(&json, "height");
        let mut entries = Vec::new();
        for l in json
            .get("layers")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if l.get("type").and_then(Value::as_str) == Some("tilelayer") {
                let data: Vec<u32> = match l.get("data") {
                    Some(Value::Array(a)) => {
                        a.iter().map(|v| v.as_u64().unwrap_or(0) as u32).collect()
                    }
                    Some(Value::String(_)) => {
                        return Err(format!(
                            "a camada \"{}\" está compactada (base64); no Tiled, mude o formato da camada para CSV",
                            l.get("name").and_then(Value::as_str).unwrap_or("")
                        ));
                    }
                    _ => vec![0; (width * height) as usize],
                };
                let mut data = data;
                data.resize((width * height) as usize, 0);
                entries.push(Entry::Tiles(TileLayer {
                    name: l
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    data,
                    visible: l.get("visible").and_then(Value::as_bool).unwrap_or(true),
                    opacity: l.get("opacity").and_then(Value::as_f64).unwrap_or(1.0),
                    locked: l.get("locked").and_then(Value::as_bool).unwrap_or(false),
                    collision: layer_collision(l),
                    json: l.clone(),
                }));
            } else {
                entries.push(Entry::Other(l.clone()));
            }
        }
        let mut tilesets = Vec::new();
        for t in json
            .get("tilesets")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let first_gid = u(t, "firstgid").max(1);
            let kind = match t.get("source").and_then(Value::as_str) {
                Some(src) => {
                    if src.to_lowercase().ends_with(".tsx") {
                        return Err(
                            "tilesets .tsx (XML) não são suportados; exporte como JSON (.tsj)"
                                .into(),
                        );
                    }
                    TsKind::External(join(dir_of(rel), src))
                }
                None => TsKind::Embedded(TilesetData::from_json(t.clone(), dir_of(rel))?),
            };
            tilesets.push(TsRef { first_gid, kind });
        }
        tilesets.sort_by_key(|t| t.first_gid);
        let rev = next_rev();
        let layer = entries
            .iter()
            .position(|e| matches!(e, Entry::Tiles(_)))
            .unwrap_or(0);
        Ok(Self {
            rel: rel.to_string(),
            width,
            height,
            tile_w: u(&json, "tilewidth").max(1),
            tile_h: u(&json, "tileheight").max(1),
            entries,
            tilesets,
            layer,
            history: Vec::new(),
            future: Vec::new(),
            rev,
            saved_rev: rev,
            shift: (0, 0),
            pending: (0, 0),
            disk_mtime: None,
            seed: 1,
            last_grow: (0, 0),
            grown_left_top: (0, 0),
            json,
        })
    }

    pub fn open(root: &Path, rel: &str) -> Result<Self, String> {
        let path = root.join(rel);
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{rel}: {e}"))?;
        let v: Value =
            serde_json::from_str(&text).map_err(|e| format!("{rel}: JSON inválido: {e}"))?;
        let mut doc = Self::from_json(v, rel).map_err(|e| format!("{rel}: {e}"))?;
        doc.disk_mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        Ok(doc)
    }

    pub fn is_dirty(&self) -> bool {
        self.rev != self.saved_rev
    }

    /// Muda a cada alteração (para quem desenha o mapa saber que mudou).
    pub fn rev(&self) -> u64 {
        self.rev
    }

    /// O arquivo mudou fora (no Tiled): relê, se não houver nada a perder.
    pub fn disk_changed(&mut self, root: &Path) -> bool {
        if self.is_dirty() {
            return false;
        }
        let m = std::fs::metadata(root.join(&self.rel))
            .and_then(|m| m.modified())
            .ok();
        if m == self.disk_mtime {
            return false;
        }
        match Self::open(root, &self.rel) {
            Ok(mut d) => {
                d.layer = self.layer.min(d.entries.len().saturating_sub(1));
                *self = d;
                true
            }
            Err(_) => false,
        }
    }

    pub fn to_json(&self) -> Value {
        let mut v = self.json.clone();
        let o = v.as_object_mut().expect("mapa é um objeto");
        o.insert("width".into(), json!(self.width));
        o.insert("height".into(), json!(self.height));
        let layers: Vec<Value> = self
            .entries
            .iter()
            .map(|e| match e {
                Entry::Other(v) => v.clone(),
                Entry::Tiles(l) => {
                    let mut lv = l.json.clone();
                    let lo = lv.as_object_mut().expect("camada é um objeto");
                    lo.insert("type".into(), json!("tilelayer"));
                    lo.insert("name".into(), json!(l.name));
                    lo.insert("width".into(), json!(self.width));
                    lo.insert("height".into(), json!(self.height));
                    lo.insert("visible".into(), json!(l.visible));
                    // Como o Tiled: 1, não 1.0.
                    let op = if l.opacity.fract() == 0.0 {
                        json!(l.opacity as i64)
                    } else {
                        json!(l.opacity)
                    };
                    lo.insert("opacity".into(), op);
                    if l.locked {
                        lo.insert("locked".into(), json!(true));
                    } else {
                        lo.remove("locked");
                    }
                    lo.insert("data".into(), json!(l.data));
                    lo.remove("encoding");
                    lo.remove("compression");
                    let mut props: Vec<Value> = lo
                        .get("properties")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|p| p.get("name").and_then(Value::as_str) != Some("collision"))
                        .collect();
                    if l.collision {
                        props.push(json!({ "name": "collision", "type": "bool", "value": true }));
                    }
                    if props.is_empty() {
                        lo.remove("properties");
                    } else {
                        lo.insert("properties".into(), Value::Array(props));
                    }
                    lv
                }
            })
            .collect();
        o.insert("layers".into(), Value::Array(layers));
        let dir = dir_of(&self.rel);
        let tilesets: Vec<Value> = self
            .tilesets
            .iter()
            .map(|t| match &t.kind {
                TsKind::External(rel) => {
                    json!({ "firstgid": t.first_gid, "source": relative(dir, rel) })
                }
                TsKind::Embedded(d) => {
                    let mut tv = d.to_json(dir);
                    tv.as_object_mut()
                        .unwrap()
                        .insert("firstgid".into(), json!(t.first_gid));
                    tv
                }
            })
            .collect();
        o.insert("tilesets".into(), Value::Array(tilesets));
        v
    }

    /// O JSON como vai para o disco: os dados de cada camada ficam uma linha
    /// do mapa por linha do arquivo (fácil de ler e de comparar).
    pub fn text(&self) -> String {
        let mut v = self.to_json();
        let mut rows: Vec<Vec<u32>> = Vec::new();
        if let Some(layers) = v.get_mut("layers").and_then(Value::as_array_mut) {
            for l in layers {
                if let Some(Value::Array(d)) = l.get("data") {
                    rows.push(d.iter().map(|x| x.as_u64().unwrap_or(0) as u32).collect());
                    l["data"] = json!(format!("@@dados{}@@", rows.len() - 1));
                }
            }
        }
        let mut text = serde_json::to_string_pretty(&v).unwrap_or_default();
        let w = self.width.max(1) as usize;
        for (i, data) in rows.iter().enumerate() {
            let key = format!("\"@@dados{i}@@\"");
            let Some(at) = text.find(&key) else { continue };
            let line_start = text[..at].rfind('\n').map_or(0, |p| p + 1);
            let indent: String = text[line_start..]
                .chars()
                .take_while(|c| *c == ' ')
                .collect();
            let lines: Vec<String> = data
                .chunks(w)
                .map(|r| {
                    format!(
                        "{indent}  {}",
                        r.iter().map(u32::to_string).collect::<Vec<_>>().join(",")
                    )
                })
                .collect();
            let body = if lines.is_empty() {
                "[]".to_string()
            } else {
                format!("[\n{}\n{indent}]", lines.join(",\n"))
            };
            text.replace_range(at..at + key.len(), &body);
        }
        text + "\n"
    }

    pub fn save(&mut self, root: &Path) -> Result<(), String> {
        let path = root.join(&self.rel);
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, self.text()).map_err(|e| e.to_string())?;
        self.disk_mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        self.saved_rev = self.rev;
        Ok(())
    }

    // ------------------------------------------------------------------
    // Histórico

    /// Guarda o estado antes de uma mudança.
    pub fn checkpoint(&mut self) {
        self.history.push(Snap {
            width: self.width,
            height: self.height,
            entries: self.entries.clone(),
            shift: self.shift,
            grown: self.grown_left_top,
            rev: self.rev,
        });
        if self.history.len() > 100 {
            self.history.remove(0);
        }
        self.future.clear();
        self.seed = self
            .seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
    }

    pub fn touch(&mut self) {
        self.rev = next_rev();
    }

    /// Tira o último passo do histórico se nada mudou desde ele.
    pub fn drop_checkpoint_if_unchanged(&mut self) {
        if let Some(s) = self.history.last()
            && s.entries == self.entries
            && s.width == self.width
            && s.height == self.height
        {
            self.rev = s.rev;
            self.history.pop();
        }
    }

    fn snap(&self) -> Snap {
        Snap {
            width: self.width,
            height: self.height,
            entries: self.entries.clone(),
            shift: self.shift,
            grown: self.grown_left_top,
            rev: self.rev,
        }
    }

    fn restore(&mut self, s: Snap) {
        self.pending.0 += s.shift.0 - self.shift.0;
        self.pending.1 += s.shift.1 - self.shift.1;
        self.width = s.width;
        self.height = s.height;
        self.entries = s.entries;
        self.shift = s.shift;
        self.grown_left_top = s.grown;
        self.rev = s.rev;
        self.layer = self.layer.min(self.entries.len().saturating_sub(1));
    }

    pub fn undo(&mut self) -> bool {
        let Some(s) = self.history.pop() else {
            return false;
        };
        self.future.push(self.snap());
        self.restore(s);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(s) = self.future.pop() else {
            return false;
        };
        self.history.push(self.snap());
        self.restore(s);
        true
    }

    /// Colunas (à esquerda) e linhas (embaixo) que o mapa ganhou (ou perdeu,
    /// ao desfazer) desde a última vez: o objeto na cena precisa andar
    /// `-colunas × largura do tile` e `-linhas × altura do tile`.
    pub fn take_shift(&mut self) -> (i32, i32) {
        std::mem::take(&mut self.pending)
    }

    // ------------------------------------------------------------------
    // Camadas

    pub fn tile_layer(&self, i: usize) -> Option<&TileLayer> {
        match self.entries.get(i) {
            Some(Entry::Tiles(l)) => Some(l),
            _ => None,
        }
    }

    pub fn tile_layer_mut(&mut self, i: usize) -> Option<&mut TileLayer> {
        match self.entries.get_mut(i) {
            Some(Entry::Tiles(l)) => Some(l),
            _ => None,
        }
    }

    /// Índices (em `entries`) das camadas de tiles.
    pub fn tile_layers(&self) -> Vec<usize> {
        (0..self.entries.len())
            .filter(|i| matches!(self.entries[*i], Entry::Tiles(_)))
            .collect()
    }

    fn next_layer_id(&mut self) -> u64 {
        let id = self
            .json
            .get("nextlayerid")
            .and_then(Value::as_u64)
            .unwrap_or(1)
            .max(1);
        // Ids que já existem não se repetem.
        let used = self
            .entries
            .iter()
            .filter_map(|e| match e {
                Entry::Tiles(l) => l.json.get("id").and_then(Value::as_u64),
                Entry::Other(v) => v.get("id").and_then(Value::as_u64),
            })
            .max()
            .unwrap_or(0);
        let id = id.max(used + 1);
        self.json
            .as_object_mut()
            .unwrap()
            .insert("nextlayerid".into(), json!(id + 1));
        id
    }

    /// Camada nova, por cima da atual. Devolve o índice.
    pub fn add_layer(&mut self, name: &str) -> usize {
        self.checkpoint();
        let id = self.next_layer_id();
        let mut name = name.to_string();
        // A primeira camada a mais costuma ser de enfeites.
        if name == "camada" && self.tile_layers().len() == 1 {
            name = "enfeites".into();
        }
        let names: Vec<String> = self
            .tile_layers()
            .iter()
            .filter_map(|i| self.tile_layer(*i).map(|l| l.name.clone()))
            .collect();
        if names.contains(&name) {
            name = (2..)
                .map(|n| format!("{name}{n}"))
                .find(|n| !names.contains(n))
                .unwrap();
        }
        let layer = TileLayer {
            name,
            data: vec![0; (self.width * self.height) as usize],
            visible: true,
            opacity: 1.0,
            locked: false,
            collision: false,
            json: json!({ "id": id, "type": "tilelayer", "x": 0, "y": 0 }),
        };
        let at = if self.entries.is_empty() {
            0
        } else {
            (self.layer + 1).min(self.entries.len())
        };
        self.entries.insert(at, Entry::Tiles(layer));
        self.layer = at;
        self.touch();
        at
    }

    pub fn remove_layer(&mut self, i: usize) {
        if self.tile_layers().len() <= 1 || self.tile_layer(i).is_none() {
            return;
        }
        self.checkpoint();
        self.entries.remove(i);
        self.layer = self
            .tile_layers()
            .into_iter()
            .rev()
            .find(|l| *l < i)
            .or_else(|| self.tile_layers().first().copied())
            .unwrap_or(0);
        self.touch();
    }

    /// Sobe (desenha por cima) ou desce a camada.
    pub fn move_layer(&mut self, i: usize, up: bool) {
        let to = if up { i + 1 } else { i.wrapping_sub(1) };
        if to >= self.entries.len() {
            return;
        }
        self.checkpoint();
        self.entries.swap(i, to);
        if self.layer == i {
            self.layer = to;
        } else if self.layer == to {
            self.layer = i;
        }
        self.touch();
    }

    /// Muda uma camada (nome, visível, cadeado, colisão, opacidade).
    pub fn edit_layer(&mut self, i: usize, f: impl FnOnce(&mut TileLayer)) {
        let Some(before) = self.tile_layer(i).cloned() else {
            return;
        };
        let mut after = before.clone();
        f(&mut after);
        if after != before {
            self.checkpoint();
            *self.tile_layer_mut(i).unwrap() = after;
            self.touch();
        }
    }

    // ------------------------------------------------------------------
    // Tiles

    pub fn index(&self, col: i64, row: i64) -> Option<usize> {
        (col >= 0 && row >= 0 && (col as u32) < self.width && (row as u32) < self.height)
            .then(|| (row as u32 * self.width + col as u32) as usize)
    }

    pub fn get(&self, layer: usize, col: i64, row: i64) -> u32 {
        match (self.tile_layer(layer), self.index(col, row)) {
            (Some(l), Some(i)) => l.data[i],
            _ => 0,
        }
    }

    /// Troca um tile (sem checkpoint). Devolve se mudou.
    pub fn set(&mut self, layer: usize, col: i64, row: i64, gid: u32) -> bool {
        let Some(i) = self.index(col, row) else {
            return false;
        };
        match self.tile_layer_mut(layer) {
            Some(l) if l.data[i] != gid => {
                l.data[i] = gid;
                true
            }
            _ => false,
        }
    }

    /// Faz o mapa crescer para caber a célula (col, row), que pode estar fora
    /// (negativa = à esquerda ou em cima). Devolve a célula no mapa novo.
    pub fn grow_to(&mut self, col: i64, row: i64) -> Option<(i64, i64)> {
        let left = (-col).max(0) as u32;
        let top = (-row).max(0) as u32;
        let right = (col + 1 - self.width as i64).max(0) as u32;
        let bottom = (row + 1 - self.height as i64).max(0) as u32;
        self.last_grow = (0, 0);
        if left + right + top + bottom == 0 {
            return Some((col, row));
        }
        let (w, h) = (self.width + left + right, self.height + top + bottom);
        if w > MAX_SIZE || h > MAX_SIZE {
            return None;
        }
        for e in &mut self.entries {
            match e {
                Entry::Tiles(l) => {
                    let mut data = vec![0; (w * h) as usize];
                    for r in 0..self.height {
                        for c in 0..self.width {
                            data[((r + top) * w + c + left) as usize] =
                                l.data[(r * self.width + c) as usize];
                        }
                    }
                    l.data = data;
                }
                Entry::Other(v) => {
                    // Objetos do Tiled ficam no mesmo lugar do desenho.
                    let (dx, dy) = ((left * self.tile_w) as f64, (top * self.tile_h) as f64);
                    if let Some(objs) = v.get_mut("objects").and_then(Value::as_array_mut) {
                        for o in objs {
                            for (k, d) in [("x", dx), ("y", dy)] {
                                if let Some(n) = o.get(k).and_then(Value::as_f64) {
                                    o[k] = json!(n + d);
                                }
                            }
                        }
                    }
                }
            }
        }
        self.width = w;
        self.height = h;
        self.shift.0 += left as i32;
        self.shift.1 += bottom as i32;
        self.pending.0 += left as i32;
        self.pending.1 += bottom as i32;
        self.last_grow = (left as i64, top as i64);
        self.grown_left_top.0 += left as i64;
        self.grown_left_top.1 += top as i64;
        Some((col + left as i64, row + top as i64))
    }

    /// Balde: pinta a área ligada com o mesmo tile. Devolve as células.
    pub fn flood(&mut self, layer: usize, col: i64, row: i64, gid: u32) -> Vec<(i64, i64)> {
        let mut out = Vec::new();
        if self.index(col, row).is_none() {
            return out;
        }
        let target = self.get(layer, col, row);
        if target == gid {
            return out;
        }
        let mut stack = vec![(col, row)];
        while let Some((c, r)) = stack.pop() {
            if self.index(c, r).is_none() || self.get(layer, c, r) != target {
                continue;
            }
            self.set(layer, c, r, gid);
            out.push((c, r));
            stack.extend([(c + 1, r), (c - 1, r), (c, r + 1), (c, r - 1)]);
        }
        out
    }

    /// O tileset de um tile: (índice em `tilesets`, id local).
    pub fn tileset_of(&self, gid: u32) -> Option<(usize, u32)> {
        let gid = gid & !FLIP_BITS;
        if gid == 0 {
            return None;
        }
        self.tilesets
            .iter()
            .enumerate()
            .rev()
            .find(|(_, t)| t.first_gid <= gid)
            .map(|(i, t)| (i, gid - t.first_gid))
    }

    /// Caminhos dos tilesets `.tsj` do mapa.
    pub fn external_tilesets(&self) -> Vec<String> {
        self.tilesets
            .iter()
            .filter_map(|t| match &t.kind {
                TsKind::External(r) => Some(r.clone()),
                TsKind::Embedded(_) => None,
            })
            .collect()
    }

    /// Autotile: pinta o terreno `color` (0 = nenhum) nos quatro cantos da
    /// célula e escolhe, para ela e as vizinhas, os tiles do conjunto que
    /// combinam com os cantos. Devolve as células que mudaram.
    #[allow(clippy::too_many_arguments)]
    pub fn paint_terrain(
        &mut self,
        layer: usize,
        ts: usize,
        data: &TilesetData,
        set: &WangSet,
        color: u8,
        col: i64,
        row: i64,
    ) -> Vec<(i64, i64)> {
        let first = self.tilesets.get(ts).map_or(1, |t| t.first_gid);
        let corners_at = |doc: &MapDoc, c: i64, r: i64| -> Option<[u8; 4]> {
            let gid = doc.get(layer, c, r);
            if gid == 0 {
                return Some([0; 4]);
            }
            let (t, local) = doc.tileset_of(gid)?;
            if t != ts {
                return None;
            }
            set.tiles.get(&local).copied()
        };
        // Vértices da célula: (0,0) cima-esquerda ... (1,1) baixo-direita.
        let mut changed = Vec::new();
        for dr in -1..=1i64 {
            for dc in -1..=1i64 {
                let (c, r) = (col + dc, row + dr);
                if self.index(c, r).is_none() {
                    continue;
                }
                let Some(mut corners) = corners_at(self, c, r) else {
                    continue;
                };
                // Cantos desta célula que são vértices da célula pintada.
                // Cantos: [TL, TR, BR, BL] → deslocamento do vértice.
                let offs = [(0, 0), (1, 0), (1, 1), (0, 1)];
                let mut touched = false;
                for (k, (ox, oy)) in offs.iter().enumerate() {
                    let (vx, vy) = (c + ox, r + oy);
                    if (col..=col + 1).contains(&vx) && (row..=row + 1).contains(&vy) {
                        corners[k] = color;
                        touched = true;
                    }
                }
                if !touched {
                    continue;
                }
                let gid = if corners == [0; 4] {
                    0
                } else {
                    let mut options = set.matching(corners);
                    if options.is_empty() {
                        // Sem a peça exata: a que mais combina.
                        let best = set
                            .tiles
                            .values()
                            .map(|v| (0..4).filter(|i| v[*i] == corners[*i]).count())
                            .max()
                            .unwrap_or(0);
                        options = set
                            .tiles
                            .iter()
                            .filter(|(_, v)| {
                                (0..4).filter(|i| v[*i] == corners[*i]).count() == best
                            })
                            .map(|(t, _)| *t)
                            .collect();
                    }
                    match self.pick(&options, data, c, r) {
                        Some(t) => first + t,
                        None => continue,
                    }
                };
                // Mantém a variação que já estava, se ela ainda serve.
                let current = self.get(layer, c, r);
                let keep = current != 0
                    && self
                        .tileset_of(current)
                        .is_some_and(|(t, l)| t == ts && set.tiles.get(&l) == Some(&corners));
                if !keep && self.set(layer, c, r, gid) {
                    changed.push((c, r));
                }
            }
        }
        changed
    }

    /// Sorteia entre variações (pela chance de cada uma), sempre igual para a
    /// mesma célula na mesma pincelada.
    fn pick(&self, options: &[u32], data: &TilesetData, c: i64, r: i64) -> Option<u32> {
        if options.len() <= 1 {
            return options.first().copied();
        }
        let weight = |t: u32| -> f64 { data.info(t).probability.unwrap_or(1.0).max(0.0) };
        let total: f64 = options.iter().map(|t| weight(*t)).sum();
        let mut h = self.seed
            ^ (c as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (r as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
        h ^= h >> 33;
        h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
        h ^= h >> 33;
        let mut x = (h as f64 / u64::MAX as f64) * total;
        for t in options {
            x -= weight(*t);
            if x <= 0.0 {
                return Some(*t);
            }
        }
        options.last().copied()
    }
}

// ---------------------------------------------------------------------------
// Giros e espelhos dos tiles (bits do Tiled).

type M = [[i32; 2]; 2];

fn mul(a: M, b: M) -> M {
    [
        [
            a[0][0] * b[0][0] + a[0][1] * b[1][0],
            a[0][0] * b[0][1] + a[0][1] * b[1][1],
        ],
        [
            a[1][0] * b[0][0] + a[1][1] * b[1][0],
            a[1][0] * b[0][1] + a[1][1] * b[1][1],
        ],
    ]
}

fn matrix(flags: u32) -> M {
    let mut m: M = [[1, 0], [0, 1]];
    if flags & FLIP_D != 0 {
        m = mul([[0, 1], [1, 0]], m);
    }
    if flags & FLIP_H != 0 {
        m = mul([[-1, 0], [0, 1]], m);
    }
    if flags & FLIP_V != 0 {
        m = mul([[1, 0], [0, -1]], m);
    }
    m
}

fn flags_of(m: M) -> u32 {
    (0..8u32)
        .map(|i| {
            (if i & 1 != 0 { FLIP_H } else { 0 })
                | (if i & 2 != 0 { FLIP_V } else { 0 })
                | (if i & 4 != 0 { FLIP_D } else { 0 })
        })
        .find(|f| matrix(*f) == m)
        .unwrap_or(0)
}

/// O tile girado 90° no sentido horário.
pub fn rotate_cw(gid: u32) -> u32 {
    if gid & !FLIP_BITS == 0 {
        return gid;
    }
    let m = mul([[0, -1], [1, 0]], matrix(gid));
    (gid & !(FLIP_H | FLIP_V | FLIP_D)) | flags_of(m)
}

/// O tile espelhado na horizontal.
pub fn flip_h(gid: u32) -> u32 {
    if gid & !FLIP_BITS == 0 {
        return gid;
    }
    let m = mul([[-1, 0], [0, 1]], matrix(gid));
    (gid & !(FLIP_H | FLIP_V | FLIP_D)) | flags_of(m)
}

/// O tile espelhado na vertical.
pub fn flip_v(gid: u32) -> u32 {
    if gid & !FLIP_BITS == 0 {
        return gid;
    }
    let m = mul([[1, 0], [0, -1]], matrix(gid));
    (gid & !(FLIP_H | FLIP_V | FLIP_D)) | flags_of(m)
}

/// Um carimbo: tiles em linhas (de cima para baixo).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stamp {
    pub w: u32,
    pub h: u32,
    pub tiles: Vec<u32>,
}

impl Stamp {
    pub fn single(gid: u32) -> Self {
        Self {
            w: 1,
            h: 1,
            tiles: vec![gid],
        }
    }

    pub fn get(&self, c: u32, r: u32) -> u32 {
        self.tiles[(r * self.w + c) as usize]
    }

    /// Gira o carimbo (e cada tile) 90° no sentido horário.
    pub fn rotated(&self) -> Self {
        let (w, h) = (self.h, self.w);
        let mut tiles = vec![0; (w * h) as usize];
        for r in 0..self.h {
            for c in 0..self.w {
                // (c, r) vai para (h - 1 - r, c).
                tiles[(c * w + (self.h - 1 - r)) as usize] = rotate_cw(self.get(c, r));
            }
        }
        Self { w, h, tiles }
    }

    pub fn flipped_h(&self) -> Self {
        let mut tiles = vec![0; self.tiles.len()];
        for r in 0..self.h {
            for c in 0..self.w {
                tiles[(r * self.w + (self.w - 1 - c)) as usize] = flip_h(self.get(c, r));
            }
        }
        Self {
            w: self.w,
            h: self.h,
            tiles,
        }
    }

    pub fn flipped_v(&self) -> Self {
        let mut tiles = vec![0; self.tiles.len()];
        for r in 0..self.h {
            for c in 0..self.w {
                tiles[((self.h - 1 - r) * self.w + c) as usize] = flip_v(self.get(c, r));
            }
        }
        Self {
            w: self.w,
            h: self.h,
            tiles,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tileset::{WangColor, corners_of};

    fn mapa() -> MapDoc {
        MapDoc::new("mapas/fase.tmj", 4, 3, 16, 16, "tiles/t.tsj")
    }

    #[test]
    fn mapa_novo_grava_no_formato_do_tiled() {
        let m = mapa();
        let v = m.to_json();
        assert_eq!(v["tilesets"][0]["source"], "../tiles/t.tsj");
        assert_eq!(v["layers"][0]["type"], "tilelayer");
        assert_eq!(v["layers"][0]["data"].as_array().unwrap().len(), 12);
        // A engine lê o que o editor grava.
        let tsj = r#"{ "name": "t", "image": "t.png", "imagewidth": 32, "imageheight": 16, "tilewidth": 16, "tileheight": 16, "columns": 2, "tilecount": 2 }"#;
        let map_text = m.text();
        let loaded = nn_tilemap::load_tiled(
            Path::new("mapas/fase.tmj"),
            |p: &Path| {
                if p.ends_with("fase.tmj") {
                    Ok(map_text.clone())
                } else {
                    Ok(tsj.to_string())
                }
            },
            1.0,
        )
        .unwrap();
        assert_eq!((loaded.width, loaded.height), (4, 3));
        assert!(!m.is_dirty());
        // Uma linha do mapa por linha do arquivo.
        assert!(
            map_text.contains("\n        0,0,0,0,\n        0,0,0,0,\n        0,0,0,0\n      ]"),
            "{map_text}"
        );
        let back: Value = serde_json::from_str(&map_text).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn mapas_dos_modelos_ja_estao_no_formato_do_editor() {
        // Salvar sem mudar nada não muda o arquivo (nada de diferenças à toa).
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../nn-run/modelos");
        for rel in ["visual/mapas/fase.tmj", "aventura/mapas/mapa.tmj"] {
            let text = std::fs::read_to_string(root.join(rel)).unwrap();
            let m = MapDoc::open(&root, rel).unwrap();
            assert_eq!(m.text(), text, "{rel}");
        }
    }

    #[test]
    fn preserva_o_que_nao_conhece() {
        let v: Value = serde_json::from_str(
            r#"{ "width": 2, "height": 1, "tilewidth": 8, "tileheight": 8, "meu": 7,
            "tilesets": [ { "firstgid": 1, "source": "t.tsj" } ],
            "layers": [ { "type": "objectgroup", "name": "obj", "objects": [ { "id": 1, "x": 4, "y": 4 } ] },
                        { "type": "tilelayer", "name": "a", "data": [1, 2], "width": 2, "height": 1, "class": "x" } ] }"#,
        )
        .unwrap();
        let mut m = MapDoc::from_json(v, "a.tmj").unwrap();
        assert_eq!(m.layer, 1);
        m.checkpoint();
        m.set(1, 0, 0, 5);
        m.touch();
        let out = m.to_json();
        assert_eq!(out["meu"], 7);
        assert_eq!(out["layers"][0]["type"], "objectgroup");
        assert_eq!(out["layers"][1]["class"], "x");
        assert_eq!(out["layers"][1]["data"], json!([5, 2]));
    }

    #[test]
    fn crescer_para_a_esquerda_e_para_baixo_move_o_objeto() {
        let mut m = mapa();
        m.set(0, 0, 0, 7);
        m.checkpoint();
        // Uma célula à esquerda e uma abaixo do mapa.
        assert_eq!(m.grow_to(-2, 3), Some((0, 3)));
        assert_eq!((m.width, m.height), (6, 4));
        assert_eq!(m.get(0, 2, 0), 7, "o tile ficou no mesmo lugar do mundo");
        assert_eq!(m.take_shift(), (2, 1));
        assert_eq!(m.take_shift(), (0, 0));
        // Crescer para cima não mexe na origem (que é o canto de baixo).
        m.checkpoint();
        m.grow_to(0, -1);
        assert_eq!(m.take_shift(), (0, 0));
        assert_eq!(m.get(0, 2, 1), 7);
        m.undo();
        assert_eq!((m.width, m.height), (6, 4));
        m.undo();
        assert_eq!((m.width, m.height), (4, 3));
        assert_eq!(m.take_shift(), (-2, -1), "desfazer devolve o objeto");
        assert!(m.grow_to(5000, 0).is_none());
    }

    #[test]
    fn camadas() {
        let mut m = mapa();
        let a = m.add_layer("chao");
        assert_eq!(m.tile_layer(a).unwrap().name, "chao2");
        m.move_layer(a, false);
        assert_eq!(m.tile_layer(0).unwrap().name, "chao2");
        assert_eq!(m.layer, 0);
        m.edit_layer(0, |l| l.collision = true);
        assert_eq!(
            m.to_json()["layers"][0]["properties"][0]["name"],
            "collision"
        );
        m.remove_layer(0);
        assert_eq!(m.tile_layers().len(), 1);
        m.remove_layer(0);
        assert_eq!(m.tile_layers().len(), 1, "a última camada fica");
        let ids: Vec<u64> = m.to_json()["layers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l["id"].as_u64().unwrap())
            .collect();
        assert_eq!(ids, [1]);
    }

    #[test]
    fn balde() {
        let mut m = mapa();
        m.set(0, 1, 1, 3);
        let n = m.flood(0, 0, 0, 2);
        assert_eq!(n.len(), 11);
        assert_eq!(m.get(0, 1, 1), 3);
    }

    #[test]
    fn giros_e_carimbo() {
        let g = 5;
        assert_eq!(rotate_cw(rotate_cw(rotate_cw(rotate_cw(g)))), g);
        assert_eq!(rotate_cw(g), g | FLIP_D | FLIP_H);
        assert_eq!(flip_h(flip_h(g | FLIP_D)), g | FLIP_D);
        assert_eq!(rotate_cw(0), 0);
        let s = Stamp {
            w: 2,
            h: 1,
            tiles: vec![1, 2],
        };
        let r = s.rotated();
        assert_eq!((r.w, r.h), (1, 2));
        assert_eq!(r.tiles[0] & !FLIP_BITS, 1);
        assert_eq!(r.tiles[1] & !FLIP_BITS, 2);
        let f = s.flipped_h();
        assert_eq!(f.tiles, vec![2 | FLIP_H, 1 | FLIP_H]);
    }

    /// Conjunto completo de 16 peças: grama (1) contra nada (0), peça = máscara.
    fn grama() -> WangSet {
        WangSet {
            name: "chao".into(),
            colors: vec![WangColor {
                name: "grama".into(),
                color: "#00ff00".into(),
            }],
            tiles: (1..16u8).map(|m| (m as u32, corners_of(m, 0, 1))).collect(),
        }
    }

    #[test]
    fn autotile_encaixa_bordas_e_cantos() {
        let mut m = MapDoc::new("a.tmj", 5, 5, 16, 16, "t.tsj");
        let set = grama();
        let data = TilesetData::new("t", "t.png", (64, 64), 16, 16);
        // Uma célula pintada: ela fica com os 4 cantos de grama (peça 15) e
        // as 8 vizinhas ganham as bordas.
        m.paint_terrain(0, 0, &data, &set, 1, 2, 2);
        let local = |m: &MapDoc, c, r| m.get(0, c, r).saturating_sub(1);
        assert_eq!(local(&m, 2, 2), 15);
        // Vizinha da esquerda: grama nos cantos da direita (TR=4, BR=2).
        assert_eq!(local(&m, 1, 2), 4 | 2);
        // Em cima à esquerda: só o canto de baixo-direita (BR=2).
        assert_eq!(local(&m, 1, 1), 2);
        assert_eq!(m.get(0, 0, 0), 0);
        // Pintar a vizinha da direita junta as duas.
        m.paint_terrain(0, 0, &data, &set, 1, 3, 2);
        assert_eq!(local(&m, 2, 2), 15);
        assert_eq!(local(&m, 3, 2), 15);
        assert_eq!(
            local(&m, 2, 1),
            2 | 1,
            "em cima do meio: os dois cantos de baixo"
        );
        // Apagar (terreno 0) volta tudo ao vazio.
        m.paint_terrain(0, 0, &data, &set, 0, 2, 2);
        m.paint_terrain(0, 0, &data, &set, 0, 3, 2);
        assert!(m.tile_layer(0).unwrap().data.iter().all(|t| *t == 0));
    }
}
