//! Mapas de tiles (`.tmj` do Tiled): criar, pintar (com terreno
//! automático), marcar os tiles sólidos e ver o mapa desenhado. As operações
//! são as do editor de mapas do NN Studio ([`nn_projeto::map`]).

use std::collections::HashMap;

use image::RgbaImage;
use serde_json::{Map, Value, json};

use nn_projeto::map::{FLIP_BITS, FLIP_D, FLIP_H, FLIP_V, MAX_SIZE, MapDoc, TsKind};
use nn_projeto::tileset::{TilesetData, TilesetDoc, WangColor, WangSet, corners_of, tsj_for};

use super::{Output, Tool, arg_str, opt_bool, opt_str, s_bool, s_list, s_obj, s_str};
use crate::Server;

/// O maior mapa que vira texto.
const MAX_TEXT_COLS: u32 = 100;

fn ints(v: &Value, n: usize, what: &str) -> Result<Vec<i64>, String> {
    let list = v
        .as_array()
        .filter(|l| l.len() == n)
        .ok_or_else(|| format!("{what} é uma lista de {n} números"))?;
    list.iter()
        .map(|x| {
            x.as_f64()
                .map(|f| f.round() as i64)
                .ok_or_else(|| format!("{what}: só números"))
        })
        .collect()
}

fn png(img: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

/// Amplia com pixels inteiros até uns `side` pixels (ou reduz, se for
/// grande demais).
fn fit(img: &RgbaImage, side: u32) -> RgbaImage {
    let big = img.width().max(img.height()).max(1);
    let (w, h) = if big < side {
        let k = (side / big).clamp(1, 8);
        (img.width() * k, img.height() * k)
    } else if big > side * 2 {
        let k = big as f64 / (side * 2) as f64;
        (
            (img.width() as f64 / k) as u32,
            (img.height() as f64 / k) as u32,
        )
    } else {
        return img.clone();
    };
    image::imageops::resize(
        img,
        w.max(1),
        h.max(1),
        image::imageops::FilterType::Nearest,
    )
}

/// O mapa e os tilesets dele.
struct Loaded {
    map: MapDoc,
    tilesets: Vec<(String, TilesetData)>,
}

impl Server {
    fn load_map(&self, rel: &str) -> Result<Loaded, String> {
        let (rel, full) = crate::paths::resolve(&self.root, rel)?;
        if !rel.to_lowercase().ends_with(".tmj") {
            return Err(format!("{rel} não é um mapa .tmj"));
        }
        if !full.is_file() {
            return Err(format!("{rel} não existe (crie com criar_mapa)"));
        }
        let map = MapDoc::open(&self.root, &rel)?;
        let mut tilesets = Vec::new();
        for t in &map.tilesets {
            match &t.kind {
                TsKind::External(r) => {
                    tilesets.push((r.clone(), TilesetData::load(&self.root, r)?))
                }
                TsKind::Embedded(d) => tilesets.push((format!("(dentro de {rel})"), d.clone())),
            }
        }
        Ok(Loaded { map, tilesets })
    }

    fn save_map(&mut self, map: &MapDoc) -> Result<(), String> {
        self.write_file(&map.rel, map.text().as_bytes())
    }

    fn save_tileset(&mut self, rel: &str, data: &TilesetData) -> Result<(), String> {
        let text = TilesetDoc::from_data(rel, data.clone()).text();
        self.write_file(rel, text.as_bytes())
    }
}

/// As imagens dos tilesets (lidas uma vez).
fn tileset_images(s: &Server, l: &Loaded) -> Vec<Option<RgbaImage>> {
    l.tilesets
        .iter()
        .map(|(_, d)| {
            image::open(s.root.join(&d.image))
                .ok()
                .map(|i| i.to_rgba8())
        })
        .collect()
}

/// O tile `gid` (com os giros do Tiled), ou nada.
fn tile_image(l: &Loaded, images: &[Option<RgbaImage>], gid: u32) -> Option<RgbaImage> {
    let (ts, local) = l.map.tileset_of(gid)?;
    let data = &l.tilesets.get(ts)?.1;
    let img = images.get(ts)?.as_ref()?;
    let (x, y) = data.tile_origin(local);
    if x + data.tile_w > img.width() || y + data.tile_h > img.height() {
        return None;
    }
    let mut t = image::imageops::crop_imm(img, x, y, data.tile_w, data.tile_h).to_image();
    if gid & FLIP_D != 0 {
        t = image::imageops::rotate90(&t);
        t = image::imageops::flip_horizontal(&t);
    }
    if gid & FLIP_H != 0 {
        t = image::imageops::flip_horizontal(&t);
    }
    if gid & FLIP_V != 0 {
        t = image::imageops::flip_vertical(&t);
    }
    Some(t)
}

/// O mapa desenhado (as camadas visíveis, de baixo para cima).
fn render_map(s: &Server, l: &Loaded) -> RgbaImage {
    let (tw, th) = (l.map.tile_w, l.map.tile_h);
    let mut out = RgbaImage::from_pixel(
        l.map.width * tw,
        l.map.height * th,
        image::Rgba([0x30, 0x34, 0x46, 255]),
    );
    let images = tileset_images(s, l);
    let mut cache: HashMap<u32, Option<RgbaImage>> = HashMap::new();
    for i in l.map.tile_layers() {
        let Some(layer) = l.map.tile_layer(i) else {
            continue;
        };
        if !layer.visible {
            continue;
        }
        for row in 0..l.map.height {
            for col in 0..l.map.width {
                let gid = layer.data[(row * l.map.width + col) as usize];
                if gid == 0 {
                    continue;
                }
                let tile = cache
                    .entry(gid)
                    .or_insert_with(|| tile_image(l, &images, gid));
                if let Some(t) = tile {
                    image::imageops::overlay(&mut out, t, (col * tw) as i64, (row * th) as i64);
                }
            }
        }
    }
    out
}

/// O tileset com uma grade (os tiles contam da esquerda para a direita, de
/// cima para baixo).
fn render_tileset(s: &Server, data: &TilesetData) -> Option<RgbaImage> {
    let img = image::open(s.root.join(&data.image)).ok()?.to_rgba8();
    let k = (32 / data.tile_w.max(1)).clamp(1, 8);
    let mut big = image::imageops::resize(
        &img,
        img.width() * k,
        img.height() * k,
        image::imageops::FilterType::Nearest,
    );
    for id in 0..data.count {
        let (x, y) = data.tile_origin(id);
        let (x, y, w, h) = (x * k, y * k, data.tile_w * k, data.tile_h * k);
        for i in 0..w {
            for (px, py) in [(x + i, y), (x + i, y + h - 1)] {
                if px < big.width() && py < big.height() {
                    big.put_pixel(px, py, image::Rgba([0xff, 0x2e, 0x88, 255]));
                }
            }
        }
        for j in 0..h {
            for (px, py) in [(x, y + j), (x + w - 1, y + j)] {
                if px < big.width() && py < big.height() {
                    big.put_pixel(px, py, image::Rgba([0xff, 0x2e, 0x88, 255]));
                }
            }
        }
    }
    Some(big)
}

/// A camada pelo nome (ou a primeira).
fn layer_index(map: &MapDoc, name: Option<&str>) -> Result<usize, String> {
    let layers = map.tile_layers();
    match name {
        None => layers
            .first()
            .copied()
            .ok_or_else(|| "o mapa não tem camada de tiles".to_string()),
        Some(n) => layers
            .iter()
            .copied()
            .find(|i| map.tile_layer(*i).is_some_and(|l| l.name == n))
            .ok_or_else(|| {
                let names: Vec<String> = layers
                    .iter()
                    .filter_map(|i| map.tile_layer(*i).map(|l| l.name.clone()))
                    .collect();
                format!(
                    "o mapa não tem a camada \"{n}\" (tem: {})",
                    names.join(", ")
                )
            }),
    }
}

/// O que um mapa tem, em texto.
fn describe(l: &Loaded) -> String {
    let m = &l.map;
    let mut out = format!(
        "{}: {}x{} tiles de {}x{} pixels ({}x{} pixels). Coluna 0 à esquerda, linha 0 em cima.\n",
        m.rel,
        m.width,
        m.height,
        m.tile_w,
        m.tile_h,
        m.width * m.tile_w,
        m.height * m.tile_h
    );
    out.push_str("Camadas (de baixo para cima):\n");
    for i in m.tile_layers() {
        if let Some(layer) = m.tile_layer(i) {
            let used = layer.data.iter().filter(|g| **g != 0).count();
            out.push_str(&format!(
                "  \"{}\": {used} tile(s){}{}\n",
                layer.name,
                if layer.collision {
                    ", COLISÃO (todos os tiles bloqueiam)"
                } else {
                    ""
                },
                if layer.visible { "" } else { ", escondida" }
            ));
        }
    }
    for (k, (rel, d)) in l.tilesets.iter().enumerate() {
        let first = m.tilesets[k].first_gid;
        out.push_str(&format!(
            "Tileset {rel} (imagem {}): tiles {} a {} ({} por linha; o {} é o do canto de cima à esquerda).\n",
            d.image,
            first,
            first + d.count.saturating_sub(1),
            d.columns,
            first
        ));
        let pick = |f: &dyn Fn(&nn_projeto::tileset::TileInfo) -> bool| -> Vec<String> {
            d.tiles
                .iter()
                .filter(|(_, t)| f(t))
                .map(|(id, _)| (first + id).to_string())
                .collect()
        };
        let solid = pick(&|t| t.solid);
        let one_way = pick(&|t| t.one_way);
        out.push_str(&format!(
            "  Sólidos: {}. Plataformas (só por cima): {}.\n",
            if solid.is_empty() {
                "nenhum".into()
            } else {
                solid.join(", ")
            },
            if one_way.is_empty() {
                "nenhum".into()
            } else {
                one_way.join(", ")
            }
        ));
        for kind in d.kinds() {
            let list = pick(&|t| t.kind == kind);
            out.push_str(&format!("  Tipo \"{kind}\": {}.\n", list.join(", ")));
        }
        for set in &d.wangsets {
            let names: Vec<&str> = set.colors.iter().map(|c| c.name.as_str()).collect();
            out.push_str(&format!(
                "  Terrenos de \"{}\" (pinte com {{\"terreno\": nome}}): {}.\n",
                set.name,
                names.join(", ")
            ));
        }
    }
    out
}

// criar_mapa ------------------------------------------------------------------

fn criar_mapa(s: &mut Server, a: &Value) -> Result<Output, String> {
    let (rel, full) = crate::paths::resolve(&s.root, arg_str(a, "caminho")?)?;
    if !rel.to_lowercase().ends_with(".tmj") {
        return Err("o caminho termina em .tmj (ex.: \"mapas/fase.tmj\")".into());
    }
    if full.exists() {
        return Err(format!("{rel} já existe; mude com pintar_mapa"));
    }
    let (tiles_rel, tiles_full) = crate::paths::resolve(&s.root, arg_str(a, "tiles")?)?;
    if !tiles_full.is_file() {
        return Err(format!(
            "{tiles_rel} não existe: passe uma imagem de tiles (.png, crie com criar_imagem) ou um tileset .tsj"
        ));
    }
    let size = match a.get("tamanho").filter(|v| !v.is_null()) {
        Some(v) => ints(v, 2, "\"tamanho\" ([colunas, linhas])")?,
        None => vec![40, 12],
    };
    let (cols, rows) = (size[0], size[1]);
    if cols < 1 || rows < 1 || cols > MAX_SIZE as i64 || rows > MAX_SIZE as i64 {
        return Err(format!("o mapa vai de 1x1 a {MAX_SIZE}x{MAX_SIZE} tiles"));
    }
    let mut made = Vec::new();
    let tsj = if tiles_rel.to_lowercase().ends_with(".tsj") {
        tiles_rel.clone()
    } else {
        let tsj = tsj_for(&tiles_rel);
        if !s.root.join(&tsj).is_file() {
            let t = match a.get("tile").filter(|v| !v.is_null()) {
                Some(v) => ints(v, 2, "\"tile\" ([largura, altura])")?,
                None => vec![16, 16],
            };
            let (tw, th) = (t[0].max(1) as u32, t[1].max(1) as u32);
            let (w, h) =
                image::image_dimensions(&tiles_full).map_err(|e| format!("{tiles_rel}: {e}"))?;
            if w < tw || h < th {
                return Err(format!(
                    "{tiles_rel} tem {w}x{h}: menor que um tile de {tw}x{th}"
                ));
            }
            let name = tiles_rel.rsplit('/').next().unwrap_or(&tiles_rel);
            let name = name.rsplit_once('.').map_or(name, |(n, _)| n);
            let data = TilesetData::new(name, &tiles_rel, (w, h), tw, th);
            s.save_tileset(&tsj, &data)?;
            made.push(tsj.clone());
        }
        tsj
    };
    let data = TilesetData::load(&s.root, &tsj)?;
    let map = MapDoc::new(
        &rel,
        cols as u32,
        rows as u32,
        data.tile_w,
        data.tile_h,
        &tsj,
    );
    s.save_map(&map)?;
    made.push(rel.clone());
    let l = s.load_map(&rel)?;
    let mut out = format!(
        "Criei {}. {}\nPinte com pintar_mapa; marque os tiles que bloqueiam com marcar_tiles.\n\
         Na cena: um objeto com {{\"type\": \"map\", \"map\": \"{rel}\", \"scale\": 1}}: o objeto fica no canto de BAIXO à esquerda do mapa (o mapa sobe e vai para a direita dele), e scale multiplica o tamanho dos tiles.\n\n{}",
        made.join(", "),
        if made.len() > 1 {
            "(O tileset novo diz como a imagem se divide em tiles.)"
        } else {
            ""
        },
        describe(&l)
    );
    out.push_str("\nOs tiles do tileset (com a grade):");
    let mut o = Output::text(out);
    if let Some(img) = render_tileset(s, &data) {
        o = o.and_image(&png(&img)?, "image/png");
    }
    Ok(o)
}

// ver_mapa --------------------------------------------------------------------

fn ver_mapa(s: &mut Server, a: &Value) -> Result<Output, String> {
    let l = s.load_map(arg_str(a, "caminho")?)?;
    let mut out = describe(&l);
    if opt_bool(a, "texto")?.unwrap_or(false) {
        let li = layer_index(&l.map, opt_str(a, "camada")?)?;
        if l.map.width > MAX_TEXT_COLS {
            out.push_str(&format!("(Como texto, só até {MAX_TEXT_COLS} colunas.)\n"));
        } else if let Some(layer) = l.map.tile_layer(li) {
            const KEYS: &str =
                "#=ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789@$%&*+?!";
            let mut legend: Vec<(u32, char)> = Vec::new();
            out.push_str(&format!(
                "\nA camada \"{}\" como texto (um caractere por tile, \".\" vazio):\n",
                layer.name
            ));
            for row in 0..l.map.height {
                let mut line = String::new();
                for col in 0..l.map.width {
                    let gid = layer.data[(row * l.map.width + col) as usize];
                    if gid == 0 {
                        line.push('.');
                        continue;
                    }
                    let ch = match legend.iter().find(|(g, _)| *g == gid) {
                        Some((_, c)) => *c,
                        None => {
                            let c = KEYS.chars().nth(legend.len()).unwrap_or('?');
                            legend.push((gid, c));
                            c
                        }
                    };
                    line.push(ch);
                }
                out.push_str(&line);
                out.push('\n');
            }
            let l: Vec<String> = legend
                .iter()
                .map(|(g, c)| {
                    let flips = if g & FLIP_BITS != 0 { " (girado)" } else { "" };
                    format!("{c} = tile {}{flips}", g & !FLIP_BITS)
                })
                .collect();
            out.push_str(&format!("{}\n", l.join(", ")));
        }
    }
    let img = render_map(s, &l);
    out.push_str(&format!(
        "\nO mapa desenhado ({}x{} pixels; o fundo azul-escuro é o vazio):",
        img.width(),
        img.height()
    ));
    let mut o = Output::text(out).and_image(&png(&fit(&img, 800))?, "image/png");
    if opt_bool(a, "tiles")?.unwrap_or(false) {
        for (_, d) in &l.tilesets {
            if let Some(t) = render_tileset(s, d) {
                o = o.and_image(&png(&t)?, "image/png");
            }
        }
    }
    Ok(o)
}

// pintar_mapa -----------------------------------------------------------------

/// As células de uma forma: retângulo, linha ou pontos.
fn cells(op: &Map<String, Value>) -> Result<Option<Vec<(i64, i64)>>, String> {
    if let Some(v) = op.get("retangulo") {
        let p = ints(v, 4, "\"retangulo\" ([coluna, linha, largura, altura])")?;
        let mut out = Vec::new();
        for r in p[1]..p[1] + p[3].max(0) {
            for c in p[0]..p[0] + p[2].max(0) {
                out.push((c, r));
            }
        }
        return Ok(Some(out));
    }
    if let Some(v) = op.get("linha") {
        let p = ints(v, 4, "\"linha\" ([c1, l1, c2, l2])")?;
        let pts = nn_projeto::pixel::line((p[0] as i32, p[1] as i32), (p[2] as i32, p[3] as i32));
        return Ok(Some(
            pts.into_iter().map(|(c, r)| (c as i64, r as i64)).collect(),
        ));
    }
    if let Some(v) = op.get("pontos") {
        let list = v
            .as_array()
            .ok_or("\"pontos\" é uma lista de [coluna, linha]")?;
        let mut out = Vec::new();
        for p in list {
            let p = ints(p, 2, "cada ponto")?;
            out.push((p[0], p[1]));
        }
        return Ok(Some(out));
    }
    Ok(None)
}

/// O número do tile, com os giros: {"tile": 5, "espelhar": "horizontal"}.
fn gid_of(op: &Map<String, Value>, v: &Value) -> Result<u32, String> {
    let mut gid = v
        .as_u64()
        .ok_or("\"tile\" é o número do tile (veja ver_mapa; 0 = vazio)")? as u32;
    match op.get("espelhar").and_then(Value::as_str) {
        Some("horizontal") => gid |= FLIP_H,
        Some("vertical") => gid |= FLIP_V,
        Some(_) => return Err("\"espelhar\" é \"horizontal\" ou \"vertical\"".into()),
        None => {}
    }
    Ok(gid)
}

fn paint_op(
    l: &mut Loaded,
    layer: usize,
    op: &Map<String, Value>,
    n: usize,
) -> Result<usize, String> {
    let err = |e: String| format!("operação {n}: {e}");
    let (w, h) = (l.map.width as i64, l.map.height as i64);
    let check = |pts: &[(i64, i64)]| -> Result<(), String> {
        if let Some((c, r)) = pts
            .iter()
            .find(|(c, r)| *c < 0 || *r < 0 || *c >= w || *r >= h)
        {
            return Err(err(format!(
                "a célula ({c}, {r}) fica fora do mapa ({w}x{h}); aumente com \"tamanho\""
            )));
        }
        Ok(())
    };
    let mut changed = 0;
    if let Some(name) = op.get("terreno") {
        let name = name
            .as_str()
            .ok_or_else(|| err("\"terreno\" é o nome do terreno".into()))?;
        let pts = cells(op)
            .map_err(err)?
            .ok_or_else(|| err("com \"terreno\", diga onde: retangulo, linha ou pontos".into()))?;
        check(&pts)?;
        // O conjunto que tem esse terreno (ou "nenhum", para apagar).
        let mut found = None;
        'out: for (ts, (_, data)) in l.tilesets.iter().enumerate() {
            for set in &data.wangsets {
                if name == "nenhum" {
                    found = Some((ts, set.clone(), 0u8));
                    break 'out;
                }
                if let Some(i) = set.colors.iter().position(|c| c.name == name) {
                    found = Some((ts, set.clone(), i as u8 + 1));
                    break 'out;
                }
            }
        }
        let Some((ts, set, color)) = found else {
            let all: Vec<String> = l
                .tilesets
                .iter()
                .flat_map(|(_, d)| {
                    d.wangsets
                        .iter()
                        .flat_map(|s| s.colors.iter().map(|c| c.name.clone()))
                })
                .collect();
            return Err(err(if all.is_empty() {
                "os tilesets deste mapa não têm terrenos; pinte com \"tile\"".into()
            } else {
                format!(
                    "não há o terreno \"{name}\"; tem: {} (ou \"nenhum\")",
                    all.join(", ")
                )
            }));
        };
        let data = l.tilesets[ts].1.clone();
        for (c, r) in pts {
            changed += l
                .map
                .paint_terrain(layer, ts, &data, &set, color, c, r)
                .len();
        }
        return Ok(changed);
    }
    if let Some(v) = op.get("apagar") {
        let pts = match v {
            Value::Bool(true) => (0..h).flat_map(|r| (0..w).map(move |c| (c, r))).collect(),
            v => {
                let p = ints(
                    v,
                    4,
                    "\"apagar\" ([coluna, linha, largura, altura] ou true)",
                )
                .map_err(err)?;
                let mut out = Vec::new();
                for r in p[1]..p[1] + p[3].max(0) {
                    for c in p[0]..p[0] + p[2].max(0) {
                        out.push((c, r));
                    }
                }
                out
            }
        };
        check(&pts)?;
        for (c, r) in pts {
            changed += usize::from(l.map.set(layer, c, r, 0));
        }
        return Ok(changed);
    }
    if let Some(v) = op.get("desenho") {
        let rows: Vec<&str> = v
            .as_array()
            .ok_or_else(|| err("\"desenho\" é uma lista de linhas de texto".into()))?
            .iter()
            .map(|r| {
                r.as_str()
                    .ok_or_else(|| err("cada linha é um texto".into()))
            })
            .collect::<Result<_, _>>()?;
        let map = op.get("tiles").and_then(Value::as_object).ok_or_else(|| {
            err(
                "com \"desenho\", diga o tile de cada caractere: \"tiles\": {\"#\": 1, \"=\": 5}"
                    .into(),
            )
        })?;
        let mut keys = Vec::new();
        for (k, v) in map {
            let mut it = k.chars();
            let (Some(ch), None) = (it.next(), it.next()) else {
                return Err(err(format!("\"{k}\" em tiles: use um caractere só")));
            };
            keys.push((ch, gid_of(op, v).map_err(err)?));
        }
        let at = match op.get("em") {
            Some(v) => ints(v, 2, "\"em\" ([coluna, linha])").map_err(err)?,
            None => vec![0, 0],
        };
        for (dr, row) in rows.iter().enumerate() {
            for (dc, ch) in row.chars().enumerate() {
                if ch == '.' || ch == ' ' {
                    continue;
                }
                let gid = keys
                    .iter()
                    .find(|(k, _)| *k == ch)
                    .map(|(_, g)| *g)
                    .ok_or_else(|| {
                        err(format!(
                            "o caractere '{ch}' não está em \"tiles\" (\".\" e espaço não pintam)"
                        ))
                    })?;
                let (c, r) = (at[0] + dc as i64, at[1] + dr as i64);
                check(&[(c, r)])?;
                changed += usize::from(l.map.set(layer, c, r, gid));
            }
        }
        return Ok(changed);
    }
    let gid = gid_of(op, op.get("tile").ok_or_else(|| {
        err("diga o que fazer: tile (com retangulo, linha, pontos ou balde), terreno, desenho ou apagar".into())
    })?)
    .map_err(err)?;
    if let Some(v) = op.get("balde") {
        let p = ints(v, 2, "\"balde\" ([coluna, linha])").map_err(err)?;
        check(&[(p[0], p[1])])?;
        return Ok(l.map.flood(layer, p[0], p[1], gid).len());
    }
    let pts = cells(op)
        .map_err(err)?
        .ok_or_else(|| err("com \"tile\", diga onde: retangulo, linha, pontos ou balde".into()))?;
    check(&pts)?;
    for (c, r) in pts {
        changed += usize::from(l.map.set(layer, c, r, gid));
    }
    Ok(changed)
}

fn pintar_mapa(s: &mut Server, a: &Value) -> Result<Output, String> {
    let mut l = s.load_map(arg_str(a, "caminho")?)?;
    let mut done = Vec::new();
    if let Some(v) = a.get("tamanho").filter(|v| !v.is_null()) {
        let p = ints(v, 2, "\"tamanho\" ([colunas, linhas])")?;
        if p[0] < l.map.width as i64 || p[1] < l.map.height as i64 {
            return Err(format!(
                "o mapa só cresce por aqui (tem {}x{}); para diminuir, apague as bordas",
                l.map.width, l.map.height
            ));
        }
        // Cresce para a direita e para baixo: o canto de cima à esquerda fica.
        if l.map.grow_to(p[0] - 1, p[1] - 1).is_none() {
            return Err(format!("o mapa vai até {MAX_SIZE}x{MAX_SIZE}"));
        }
        done.push(format!(
            "o mapa agora tem {}x{} tiles",
            l.map.width, l.map.height
        ));
    }
    let name = opt_str(a, "camada")?;
    let layer = match name {
        Some(n) if layer_index(&l.map, Some(n)).is_err() => {
            if !opt_bool(a, "criar_camada")?.unwrap_or(false) {
                return Err(format!(
                    "{}; para criar, use \"criar_camada\": true",
                    layer_index(&l.map, Some(n)).unwrap_err()
                ));
            }
            let i = l.map.add_layer(n);
            done.push(format!(
                "camada nova \"{}\"",
                l.map
                    .tile_layer(i)
                    .map(|x| x.name.clone())
                    .unwrap_or_default()
            ));
            i
        }
        n => layer_index(&l.map, n)?,
    };
    if let Some(on) = opt_bool(a, "colisao")? {
        if let Some(layer) = l.map.tile_layer_mut(layer) {
            layer.collision = on;
        }
        done.push(if on {
            "a camada agora é de colisão (todos os tiles dela bloqueiam)".into()
        } else {
            "a camada não é mais de colisão".into()
        });
    }
    let mut changed = 0;
    if let Some(ops) = a.get("operacoes").and_then(Value::as_array) {
        for (i, op) in ops.iter().enumerate() {
            let op = op
                .as_object()
                .ok_or_else(|| format!("operação {}: precisa ser um objeto", i + 1))?;
            changed += paint_op(&mut l, layer, op, i + 1)?;
        }
        done.push(format!(
            "{} operação(ões), {changed} tile(s) mudaram",
            ops.len()
        ));
    }
    if done.is_empty() {
        return Err("diga o que fazer: operacoes, tamanho ou colisao".into());
    }
    l.map.touch();
    s.save_map(&l.map)?;
    let layer_name = l
        .map
        .tile_layer(layer)
        .map(|x| x.name.clone())
        .unwrap_or_default();
    let out = format!(
        "{} (camada \"{layer_name}\"): {}.",
        l.map.rel,
        done.join("; ")
    );
    if opt_bool(a, "previa")?.unwrap_or(true) {
        let img = render_map(s, &l);
        return Ok(Output::text(format!("{out} O mapa agora:"))
            .and_image(&png(&fit(&img, 800))?, "image/png"));
    }
    Ok(Output::text(out))
}

// marcar_tiles ----------------------------------------------------------------

fn marcar_tiles(s: &mut Server, a: &Value) -> Result<Output, String> {
    let rel = arg_str(a, "caminho")?;
    // Pelo mapa (números do mapa) ou direto no tileset (o 1 é o primeiro).
    let (tsj, mut data, first) = if rel.to_lowercase().ends_with(".tmj") {
        let l = s.load_map(rel)?;
        let k = l
            .map
            .tilesets
            .iter()
            .position(|t| matches!(t.kind, TsKind::External(_)))
            .ok_or("os tilesets deste mapa estão dentro dele; edite no Tiled ou no NN Studio")?;
        let TsKind::External(r) = &l.map.tilesets[k].kind else {
            unreachable!()
        };
        (
            r.clone(),
            l.tilesets[k].1.clone(),
            l.map.tilesets[k].first_gid,
        )
    } else {
        let (r, _) = crate::paths::resolve(&s.root, rel)?;
        if !r.to_lowercase().ends_with(".tsj") {
            return Err("passe o mapa (.tmj) ou o tileset (.tsj)".into());
        }
        (r.clone(), TilesetData::load(&s.root, &r)?, 1)
    };
    let id_of = |n: &Value| -> Result<u32, String> {
        let n = n.as_u64().ok_or("os tiles são números")? as u32;
        if n < first || n >= first + data.count {
            return Err(format!(
                "o tileset tem os tiles {first} a {}; não há o {n}",
                first + data.count - 1
            ));
        }
        Ok(n - first)
    };
    let list = |k: &str| -> Result<Vec<u32>, String> {
        match a.get(k).filter(|v| !v.is_null()) {
            None => Ok(Vec::new()),
            Some(v) => v
                .as_array()
                .ok_or_else(|| format!("\"{k}\" é uma lista de tiles"))?
                .iter()
                .map(&id_of)
                .collect(),
        }
    };
    let (solid, one_way, normal) = (list("solidos")?, list("plataformas")?, list("normais")?);
    let mut kinds = Vec::new();
    if let Some(m) = a.get("tipo").and_then(Value::as_object) {
        for (k, v) in m {
            let ids: Vec<u32> = v
                .as_array()
                .ok_or("\"tipo\" é {\"lava\": [tiles]}")?
                .iter()
                .map(&id_of)
                .collect::<Result<_, _>>()?;
            kinds.push((k.clone(), ids));
        }
    }
    // Terreno: as peças e os cantos de cada uma.
    let mut terrain = None;
    if let Some(t) = a.get("terreno").filter(|v| !v.is_null()) {
        let t = t
            .as_object()
            .ok_or("\"terreno\" é {\"nome\": \"grama\", \"primeiro\": 5} ou {\"nome\": ..., \"pecas\": {\"5\": \"0011\"}}")?;
        let name = t
            .get("nome")
            .and_then(Value::as_str)
            .filter(|n| !n.trim().is_empty() && *n != "nenhum")
            .ok_or("o terreno precisa de \"nome\" (ex.: \"grama\")")?
            .trim()
            .to_string();
        let mut pieces: Vec<(u32, u8)> = Vec::new();
        if let Some(first_tile) = t.get("primeiro") {
            for m in 0..16u8 {
                let n = first_tile
                    .as_u64()
                    .ok_or("\"primeiro\" é o número de um tile")?
                    + u64::from(m);
                pieces.push((id_of(&json!(n))?, m));
            }
        }
        if let Some(p) = t.get("pecas").and_then(Value::as_object) {
            for (k, v) in p {
                let n: u64 = k
                    .parse()
                    .map_err(|_| format!("\"{k}\" em pecas: a chave é o número do tile"))?;
                let bits = v.as_str().filter(|b| b.len() == 4 && b.chars().all(|c| c == '0' || c == '1')).ok_or_else(|| {
                    format!("a peça {k}: os cantos são 4 dígitos 0/1 (cima-esq, cima-dir, baixo-dir, baixo-esq), ex.: \"0011\" = a metade de baixo")
                })?;
                pieces.push((id_of(&json!(n))?, u8::from_str_radix(bits, 2).unwrap_or(0)));
            }
        }
        if pieces.is_empty() {
            return Err(
                "o terreno precisa de \"primeiro\" (16 tiles seguidos) ou de \"pecas\"".into(),
            );
        }
        let color = t
            .get("cor")
            .and_then(Value::as_str)
            .unwrap_or("#38b764")
            .to_string();
        terrain = Some((name, color, pieces));
    }
    if solid.is_empty()
        && one_way.is_empty()
        && normal.is_empty()
        && kinds.is_empty()
        && terrain.is_none()
    {
        return Err("diga o que marcar: solidos, plataformas, normais, tipo ou terreno".into());
    }
    for id in &normal {
        let mut i = data.info(*id);
        i.solid = false;
        i.one_way = false;
        data.set_info(*id, i);
    }
    for id in &solid {
        let mut i = data.info(*id);
        i.solid = true;
        i.one_way = false;
        data.set_info(*id, i);
    }
    for id in &one_way {
        let mut i = data.info(*id);
        i.one_way = true;
        i.solid = false;
        data.set_info(*id, i);
    }
    for (k, ids) in &kinds {
        for id in ids {
            let mut i = data.info(*id);
            i.kind = k.clone();
            data.set_info(*id, i);
        }
    }
    let mut terrain_note = String::new();
    if let Some((name, color, pieces)) = terrain {
        // Um conjunto só ("terrenos"): o terreno novo entra nele, ou é trocado.
        if data.wangsets.is_empty() {
            data.wangsets.push(WangSet {
                name: "terrenos".into(),
                colors: Vec::new(),
                tiles: Default::default(),
            });
        }
        let set = &mut data.wangsets[0];
        let c = match set.colors.iter().position(|c| c.name == name) {
            Some(i) => {
                set.colors[i].color = color;
                i as u8 + 1
            }
            None => {
                set.colors.push(WangColor {
                    name: name.clone(),
                    color,
                });
                set.colors.len() as u8
            }
        };
        set.tiles.retain(|_, corners| !corners.contains(&c));
        for (id, mask) in pieces.iter().filter(|(_, m)| *m != 0) {
            set.tiles.insert(*id, corners_of(*mask, 0, c));
        }
        let have = set.coverage(0, c);
        terrain_note = format!(
            " Terreno \"{name}\": {} peça(s), {have} das 15 combinações de cantos{}. Pinte com pintar_mapa {{\"terreno\": \"{name}\", \"retangulo\": [...]}} (\"nenhum\" apaga).",
            pieces.len(),
            if have < 15 {
                " (faltam peças: as bordas que não tiverem a sua ficam com a mais parecida)"
            } else {
                ""
            }
        );
    }
    s.save_tileset(&tsj, &data)?;
    Ok(Output::text(format!(
        "{tsj}: {} sólido(s), {} plataforma(s), {} normal(is){}. No jogo, um mapa com \"collision\": true faz os sólidos bloquearem.{terrain_note}",
        solid.len(),
        one_way.len(),
        normal.len(),
        if kinds.is_empty() {
            String::new()
        } else {
            format!(
                ", tipos: {}",
                kinds
                    .iter()
                    .map(|(k, v)| format!("{k} ({})", v.len()))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    )))
}

const MAPA: &str = "O mapa (.tmj), ex.: \"mapas/fase.tmj\"";

pub static TOOLS: &[&Tool] = &[
    &Tool {
        name: "criar_mapa",
        title: "Criar mapa de tiles",
        description: "Cria um mapa de tiles (.tmj, o formato do Tiled que o NN Studio edita) a partir de uma imagem de tiles (.png, que vira um tileset .tsj) ou de um tileset .tsj, e mostra os tiles com a grade e os números. Depois: pintar_mapa, marcar_tiles (quais bloqueiam) e, na cena, um objeto com o componente map.",
        params: || {
            vec![
                ("caminho", s_str(MAPA), true),
                (
                    "tiles",
                    s_str("A imagem dos tiles (.png) ou o tileset (.tsj)"),
                    true,
                ),
                (
                    "tile",
                    s_list(
                        "[largura, altura] de cada tile na imagem (padrão [16, 16])",
                        json!({ "type": "integer" }),
                    ),
                    false,
                ),
                (
                    "tamanho",
                    s_list(
                        "[colunas, linhas] do mapa (padrão [40, 12])",
                        json!({ "type": "integer" }),
                    ),
                    false,
                ),
            ]
        },
        read_only: false,
        destructive: false,
        run: criar_mapa,
    },
    &Tool {
        name: "ver_mapa",
        title: "Ver mapa",
        description: "Mostra um mapa de tiles desenhado, com o tamanho, as camadas, os tilesets (os números dos tiles, quais são sólidos, plataformas e de que tipo) e os terrenos para pintar. Com \"texto\", uma camada como texto (um caractere por tile); com \"tiles\", também a imagem dos tilesets com a grade.",
        params: || {
            vec![
                ("caminho", s_str(MAPA), true),
                (
                    "texto",
                    s_bool("Uma camada como texto (até 100 colunas)"),
                    false,
                ),
                (
                    "camada",
                    s_str("A camada do texto (padrão a primeira)"),
                    false,
                ),
                ("tiles", s_bool("Também os tilesets com a grade"), false),
            ]
        },
        read_only: true,
        destructive: false,
        run: ver_mapa,
    },
    &Tool {
        name: "pintar_mapa",
        title: "Pintar mapa",
        description: "Pinta um mapa de tiles numa camada (coluna 0 à esquerda, linha 0 em cima), com várias operações numa chamada, e devolve o mapa desenhado. Operações: {\"tile\": 3, \"retangulo\": [coluna, linha, largura, altura]}, {\"tile\": 3, \"linha\": [c1, l1, c2, l2]}, {\"tile\": 3, \"pontos\": [[c, l], ...]}, {\"tile\": 3, \"balde\": [c, l]}, {\"tile\": 3, \"espelhar\": \"horizontal\", ...}, {\"apagar\": [c, l, largura, altura]} ou {\"apagar\": true}, {\"terreno\": \"grama\", \"retangulo\": [...]} (terreno automático: as bordas e os cantos se ajustam sozinhos, se o tileset tiver terrenos) e {\"desenho\": [\"....##....\", \"##########\"], \"tiles\": {\"#\": 1}, \"em\": [c, l]} (um caractere por tile; \".\" não pinta). Também: \"camada\" (com \"criar_camada\": true, uma nova), \"colisao\" (todos os tiles da camada bloqueiam) e \"tamanho\" (o mapa cresce para a direita e para baixo).",
        params: || {
            vec![
                ("caminho", s_str(MAPA), true),
                (
                    "operacoes",
                    s_list(
                        "As operações, na ordem (veja a descrição)",
                        s_obj("Uma operação"),
                    ),
                    false,
                ),
                ("camada", s_str("A camada (padrão a primeira)"), false),
                (
                    "criar_camada",
                    s_bool("Criar a camada se ela não existir"),
                    false,
                ),
                (
                    "colisao",
                    s_bool("A camada vira de colisão (true) ou deixa de ser (false)"),
                    false,
                ),
                (
                    "tamanho",
                    s_list(
                        "[colunas, linhas]: o mapa cresce para a direita e para baixo",
                        json!({ "type": "integer" }),
                    ),
                    false,
                ),
                (
                    "previa",
                    s_bool("Devolver o mapa desenhado (padrão true)"),
                    false,
                ),
            ]
        },
        read_only: false,
        destructive: false,
        run: pintar_mapa,
    },
    &Tool {
        name: "marcar_tiles",
        title: "Marcar tiles",
        description: "Marca no tileset quais tiles bloqueiam (sólidos), quais são plataformas de mão única (dá para subir por baixo) e o tipo livre de cada um (\"lava\", \"agua\"), que os eventos usam (\"encostar num tile\"). Os números são os do mapa (ver_mapa); com um .tsj, o 1 é o primeiro tile.",
        params: || {
            vec![
                ("caminho", s_str("O mapa (.tmj) ou o tileset (.tsj)"), true),
                (
                    "solidos",
                    s_list("Tiles que bloqueiam", json!({ "type": "integer" })),
                    false,
                ),
                (
                    "plataformas",
                    s_list("Tiles de mão única", json!({ "type": "integer" })),
                    false,
                ),
                (
                    "normais",
                    s_list(
                        "Tiles que não bloqueiam (tira sólido e plataforma)",
                        json!({ "type": "integer" }),
                    ),
                    false,
                ),
                ("tipo", s_obj("{\"lava\": [tiles], ...}"), false),
                (
                    "terreno",
                    s_obj(
                        "Um terreno para o pintar_mapa ajustar as bordas sozinho: {\"nome\": \"grama\", \"primeiro\": 5} (os 16 tiles a partir do 5: o tile 5+m tem terreno nos cantos dos bits de m, 8 = cima-esq, 4 = cima-dir, 2 = baixo-dir, 1 = baixo-esq; o 5 é o vazio e o 20 o cheio) ou {\"nome\": \"grama\", \"pecas\": {\"7\": \"0011\", ...}} (cantos cima-esq, cima-dir, baixo-dir, baixo-esq)",
                    ),
                    false,
                ),
            ]
        },
        read_only: false,
        destructive: false,
        run: marcar_tiles,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formas_em_celulas() {
        let op = json!({ "retangulo": [1, 2, 3, 2] });
        let c = cells(op.as_object().unwrap()).unwrap().unwrap();
        assert_eq!(c.len(), 6);
        assert_eq!(c[0], (1, 2));
        let op = json!({ "linha": [0, 0, 3, 0] });
        assert_eq!(cells(op.as_object().unwrap()).unwrap().unwrap().len(), 4);
        let op = json!({ "tile": 2, "espelhar": "horizontal" });
        let o = op.as_object().unwrap();
        assert_eq!(gid_of(o, &o["tile"]).unwrap(), 2 | FLIP_H);
    }
}
