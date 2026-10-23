//! Pixel art: criar e desenhar imagens, os quadros da folha e as animações
//! (`.anim`), com a imagem ampliada para o agente ver o que fez. As
//! operações são as do editor de pixel art do NN Studio
//! ([`nn_projeto::pixel`]), então o que um faz o outro abre igual.

use image::RgbaImage;
use serde_json::{Map, Value, json};

use nn_projeto::anim::{Anim, AnimFile, Frame, Mode, anim_path};
use nn_projeto::pixel::{self as px, CLEAR, Color, PixRect};

use super::{
    Output, Tool, arg_str, opt_bool, opt_num, opt_str, s_bool, s_enum, s_int, s_list, s_num, s_obj,
    s_str,
};
use crate::Server;

/// O maior lado de uma imagem que o agente cria ou muda.
const MAX_SIDE: u32 = 1024;
/// O maior lado da prévia (em pixels da tela).
const PREVIEW_SIDE: u32 = 512;
/// O maior quadro que vira texto (`texto: true`).
const MAX_TEXT_SIDE: u32 = 64;

/// A imagem do projeto e as animações dela.
struct Sheet {
    rel: String,
    img: RgbaImage,
    anim: AnimFile,
    /// O `.anim` existia.
    anim_on_disk: bool,
}

impl Sheet {
    fn frames(&self) -> u32 {
        px::frame_count(&self.img, self.anim.frame_w, self.anim.frame_h)
    }

    /// A área do quadro `n` (a partir de 1), ou da imagem inteira.
    fn area(&self, n: Option<u32>) -> Result<PixRect, String> {
        let total = self.frames();
        match n {
            None => Ok(if total > 1 {
                px::cell(&self.img, self.anim.frame_w, self.anim.frame_h, 0)
            } else {
                PixRect::of(&self.img)
            }),
            Some(n) if n >= 1 && n <= total => Ok(px::cell(
                &self.img,
                self.anim.frame_w,
                self.anim.frame_h,
                n - 1,
            )),
            Some(n) => Err(format!(
                "{} tem {total} quadro(s); não há o quadro {n}",
                self.rel
            )),
        }
    }

    fn frame_size(&self) -> (u32, u32) {
        if self.frames() > 1 {
            (self.anim.frame_w, self.anim.frame_h)
        } else {
            self.img.dimensions()
        }
    }
}

fn is_image(rel: &str) -> bool {
    rel.to_lowercase().ends_with(".png")
}

impl Server {
    fn load_sheet(&self, rel: &str) -> Result<Sheet, String> {
        let (rel, full) = crate::paths::resolve(&self.root, rel)?;
        if !is_image(&rel) {
            return Err(format!("{rel} não é uma imagem .png"));
        }
        if !full.is_file() {
            return Err(format!("{rel} não existe (crie com criar_imagem)"));
        }
        let img = image::open(&full)
            .map_err(|e| format!("não deu para abrir {rel}: {e}"))?
            .to_rgba8();
        let anim_rel = anim_path(&rel);
        let anim_on_disk = self.root.join(&anim_rel).is_file();
        let anim = if anim_on_disk {
            AnimFile::load(&self.root, &rel)?
        } else {
            AnimFile::default()
        };
        Ok(Sheet {
            rel,
            img,
            anim,
            anim_on_disk,
        })
    }

    /// Grava a imagem e, se precisar, o `.anim` (como o NN Studio grava).
    fn save_sheet(&mut self, sheet: &Sheet) -> Result<(), String> {
        let mut png = Vec::new();
        sheet
            .img
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        self.write_file(&sheet.rel, &png)?;
        let wants_anim = !sheet.anim.anims.is_empty()
            || (sheet.anim.frame_w > 0 && sheet.frames() > 1)
            || sheet.anim_on_disk;
        if wants_anim {
            let text = sheet.anim.write(&sheet.rel);
            self.write_file(&anim_path(&sheet.rel), text.as_bytes())?;
        }
        Ok(())
    }

    /// A paleta do projeto (`paleta.hex`), se houver.
    fn project_palette(&self) -> Vec<Color> {
        std::fs::read_to_string(self.root.join("paleta.hex"))
            .map(|t| px::parse_hex_list(&t))
            .unwrap_or_default()
    }
}

/// A paleta escolhida pelo nome ("projeto", "Sweetie 16", "PICO-8"...).
fn palette(s: &Server, name: Option<&str>) -> Result<Vec<Color>, String> {
    let names = || {
        let mut v = vec!["projeto".to_string()];
        v.extend(px::PALETTES.iter().map(|(n, _)| n.to_string()));
        v.join(", ")
    };
    match name {
        None => {
            let p = s.project_palette();
            Ok(if p.is_empty() {
                px::parse_hex_list(px::PALETTES[0].1)
            } else {
                p
            })
        }
        Some(n) if crate::paths::fold(n) == "projeto" => {
            let p = s.project_palette();
            if p.is_empty() {
                Err("o projeto não tem paleta (paleta.hex); use outra ou cores #rrggbb".into())
            } else {
                Ok(p)
            }
        }
        Some(n) => px::PALETTES
            .iter()
            .find(|(k, _)| crate::paths::fold(k) == crate::paths::fold(n))
            .map(|(_, hex)| px::parse_hex_list(hex))
            .ok_or_else(|| format!("paleta \"{n}\" não existe; tem: {}", names())),
    }
}

/// Uma cor: "#rrggbb", "#rrggbbaa", "transparente" ou o número de uma cor da
/// paleta (a partir de 1).
fn color(v: &Value, pal: &[Color]) -> Result<Color, String> {
    match v {
        Value::String(s) if crate::paths::fold(s) == "transparente" => Ok(CLEAR),
        Value::String(s) => {
            let t = s.trim().trim_start_matches('#');
            if !matches!(t.len(), 6 | 8) {
                return Err(format!(
                    "cor \"{s}\": use \"#rrggbb\" (ex.: \"#ff0044\"), \"transparente\" ou o número de uma cor da paleta"
                ));
            }
            px::parse_hex_list(t)
                .first()
                .copied()
                .ok_or_else(|| format!("cor \"{s}\" não é um hexadecimal válido"))
        }
        Value::Number(n) => {
            let i = n.as_u64().unwrap_or(0) as usize;
            pal.get(i.wrapping_sub(1))
                .copied()
                .ok_or_else(|| format!("a paleta tem {} cores; não há a cor {n}", pal.len()))
        }
        _ => Err("a cor é um texto (\"#ff0044\", \"transparente\") ou um número da paleta".into()),
    }
}

fn hex(c: Color) -> String {
    if c[3] == 0 {
        "transparente".into()
    } else {
        format!("#{}", px::to_hex(c))
    }
}

/// Números de uma lista JSON.
fn ints(v: &Value, n: usize, what: &str) -> Result<Vec<i32>, String> {
    let list = v
        .as_array()
        .filter(|l| l.len() == n)
        .ok_or_else(|| format!("{what} é uma lista de {n} números"))?;
    list.iter()
        .map(|x| {
            x.as_f64()
                .map(|f| f.round() as i32)
                .ok_or_else(|| format!("{what}: só números"))
        })
        .collect()
}

/// A imagem ampliada para mandar ao agente: o quadro (`fw`×`fh`, ou a
/// imagem) com uns 512 pixels, sem a folha passar de uns 1600 de largura.
fn preview_png(img: &RgbaImage, fw: u32, fh: u32, scale: Option<u32>) -> Result<Vec<u8>, String> {
    let side = if fw > 0 && fh > 0 {
        fw.max(fh)
    } else {
        img.width().max(img.height())
    }
    .max(1);
    let fit = (PREVIEW_SIDE / side)
        .min(1600 / img.width().max(1))
        .min(1200 / img.height().max(1));
    let scale = scale.unwrap_or(fit.clamp(1, 16)).clamp(1, 32);
    let p = px::preview(img, scale, fw, fh, true);
    let mut png = Vec::new();
    p.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(png)
}

// criar_imagem ----------------------------------------------------------------

fn criar_imagem(s: &mut Server, a: &Value) -> Result<Output, String> {
    let (rel, full) = crate::paths::resolve(&s.root, arg_str(a, "caminho")?)?;
    if !is_image(&rel) {
        return Err("o caminho termina em .png (ex.: \"assets/imagens/heroi.png\")".into());
    }
    if full.exists() && !opt_bool(a, "substituir")?.unwrap_or(false) {
        return Err(format!(
            "{rel} já existe; mude com desenhar, ou use \"substituir\": true para começar de novo"
        ));
    }
    let w = opt_num(a, "largura")?.unwrap_or(16.0) as u32;
    let h = opt_num(a, "altura")?.unwrap_or(w as f64) as u32;
    let frames = opt_num(a, "quadros")?.unwrap_or(1.0).max(1.0) as u32;
    if w == 0 || h == 0 || w * frames > MAX_SIDE || h > MAX_SIDE {
        return Err(format!(
            "o tamanho vai de 1 a {MAX_SIDE} pixels (com os quadros lado a lado)"
        ));
    }
    let pal = palette(s, opt_str(a, "paleta")?)?;
    let fill = match a.get("fundo") {
        None | Some(Value::Null) => CLEAR,
        Some(v) => color(v, &pal)?,
    };
    let mut img = RgbaImage::from_pixel(w * frames, h, image::Rgba(fill));
    if fill == CLEAR {
        img = RgbaImage::new(w * frames, h);
    }
    let anim = if frames > 1 {
        AnimFile {
            frame_w: w,
            frame_h: h,
            anims: Vec::new(),
        }
    } else {
        AnimFile::default()
    };
    let sheet = Sheet {
        rel: rel.clone(),
        img,
        anim,
        anim_on_disk: full.exists() && s.root.join(anim_path(&rel)).is_file(),
    };
    s.save_sheet(&sheet)?;
    let mut msg = format!(
        "Criei {rel}: {w}x{h} pixels{}. Desenhe com desenhar (x para a direita, y para baixo, (0, 0) no canto de cima à esquerda{}).",
        if frames > 1 {
            format!(
                ", {frames} quadros lado a lado (folha de {}x{h})",
                w * frames
            )
        } else {
            String::new()
        },
        if frames > 1 { ", em cada quadro" } else { "" }
    );
    if w <= 32 && h <= 32 {
        let res = s.config().and_then(|(_, c)| c.resolution);
        if res.is_none_or(|(rw, _)| rw > 640) {
            msg.push_str(
                "\nNum jogo de 1280x720, um sprite de 16x16 fica pequeno. Para pixel art, use uma tela pequena com os pixels inteiros (mudar_config com \"resolution\": [320, 180] e \"scale\": \"pixel\"), ou aumente w e h do sprite (64x64 = 4 vezes). Numa tela pequena, diminua também a gravidade da cena, o pulo e a velocidade (os padrões são para 1280x720).",
            );
        }
    }
    msg.push_str(&format!(
        "\nNa cena: {{\"type\": \"sprite\", \"image\": \"{rel}\"{}}} (o sprite fica do tamanho da imagem; w e h mudam).",
        if frames > 1 {
            format!(", \"frame_w\": {w}, \"frame_h\": {h}")
        } else {
            String::new()
        }
    ));
    Ok(Output::text(msg))
}

// desenhar --------------------------------------------------------------------

/// Faz uma operação. Devolve quantos pixels mudaram.
fn draw_op(
    sheet: &mut Sheet,
    clip: PixRect,
    op: &Map<String, Value>,
    pal: &[Color],
    shared: &[(char, Color)],
    n: usize,
) -> Result<(), String> {
    let err = |e: String| format!("operação {n}: {e}");
    let c = || -> Result<Color, String> { color(op.get("cor").ok_or("falta \"cor\"")?, pal) };
    // Pontos relativos ao quadro.
    let (ox, oy) = (clip.x as i32, clip.y as i32);
    let before = sheet.img.clone();
    let img = &mut sheet.img;
    let paint = |img: &mut RgbaImage, pts: Vec<(i32, i32)>, c: Color| {
        for (x, y) in pts {
            px::put(img, x + ox, y + oy, c, clip);
        }
    };
    if let Some(v) = op.get("pontos") {
        let c = c().map_err(err)?;
        let list = v
            .as_array()
            .ok_or_else(|| err("\"pontos\" é uma lista de [x, y]".into()))?;
        let mut pts = Vec::new();
        for p in list {
            let p = ints(p, 2, "cada ponto").map_err(err)?;
            pts.push((p[0], p[1]));
        }
        paint(img, pts, c);
    } else if let Some(v) = op.get("linha") {
        let c = c().map_err(err)?;
        let p = ints(v, 4, "\"linha\" ([x1, y1, x2, y2])").map_err(err)?;
        paint(img, px::line((p[0], p[1]), (p[2], p[3])), c);
    } else if let Some(v) = op.get("retangulo").or_else(|| op.get("elipse")) {
        let c = c().map_err(err)?;
        let ellipse = op.contains_key("elipse");
        let p = ints(v, 4, "[x, y, largura, altura]").map_err(err)?;
        if p[2] <= 0 || p[3] <= 0 {
            return Err(err("largura e altura precisam ser maiores que 0".into()));
        }
        let filled = op.get("cheio").and_then(Value::as_bool).unwrap_or(true);
        let (a, b) = ((p[0], p[1]), (p[0] + p[2] - 1, p[1] + p[3] - 1));
        let pts = if ellipse {
            px::ellipse(a, b, filled)
        } else {
            px::rect(a, b, filled)
        };
        paint(img, pts, c);
    } else if let Some(v) = op.get("balde") {
        let c = c().map_err(err)?;
        let p = ints(v, 2, "\"balde\" ([x, y])").map_err(err)?;
        px::flood_fill(img, p[0] + ox, p[1] + oy, c, clip);
    } else if let Some(v) = op.get("apagar") {
        let r = match v {
            Value::Bool(true) => clip,
            v => {
                let p = ints(v, 4, "\"apagar\" ([x, y, largura, altura] ou true)").map_err(err)?;
                PixRect::between(
                    (p[0] + ox, p[1] + oy),
                    (p[0] + ox + p[2] - 1, p[1] + oy + p[3] - 1),
                    clip,
                )
                .unwrap_or(PixRect::new(0, 0, 0, 0))
            }
        };
        px::clear_rect(img, r);
    } else if let Some(v) = op.get("trocar_cor") {
        let from = color(v, pal).map_err(err)?;
        px::replace_color(img, clip, from, c().map_err(err)?);
    } else if let Some(v) = op.get("espelhar") {
        match v.as_str() {
            Some("horizontal") => px::flip(img, clip, true),
            Some("vertical") => px::flip(img, clip, false),
            _ => return Err(err("\"espelhar\" é \"horizontal\" ou \"vertical\"".into())),
        }
    } else if let Some(v) = op.get("girar") {
        let turns = match v.as_i64() {
            Some(90) => 1,
            Some(180) => 2,
            Some(270) | Some(-90) => 3,
            _ => {
                return Err(err(
                    "\"girar\" é 90, 180 ou 270 (graus, no sentido do relógio)".into(),
                ));
            }
        };
        if !px::rotate(img, clip, turns) {
            return Err(err(format!(
                "só dá para girar um quadro quadrado (este é {}x{})",
                clip.w, clip.h
            )));
        }
    } else if let Some(v) = op.get("simetria") {
        // Copia uma metade espelhada na outra (personagens simétricos).
        let left = match v.as_str() {
            Some("esquerda") => true,
            Some("direita") => false,
            _ => {
                return Err(err(
                    "\"simetria\" é \"esquerda\" (copia a esquerda na direita) ou \"direita\""
                        .into(),
                ));
            }
        };
        for y in clip.y..clip.y + clip.h {
            for dx in 0..clip.w / 2 {
                let (a, b) = (clip.x + dx, clip.x + clip.w - 1 - dx);
                let (from, to) = if left { (a, b) } else { (b, a) };
                let p = *img.get_pixel(from, y);
                img.put_pixel(to, y, p);
            }
        }
    } else if let Some(v) = op.get("deslocar") {
        let p = ints(v, 2, "\"deslocar\" ([dx, dy])").map_err(err)?;
        px::shift(img, clip, p[0], p[1]);
    } else if op.get("contorno").is_some_and(|v| v == true) {
        px::outline(img, clip, c().map_err(err)?);
    } else if let Some(v) = op.get("desenho") {
        let rows: Vec<&str> = v
            .as_array()
            .ok_or_else(|| err("\"desenho\" é uma lista de linhas de texto".into()))?
            .iter()
            .map(|r| {
                r.as_str()
                    .ok_or_else(|| err("cada linha do desenho é um texto".into()))
            })
            .collect::<Result<_, _>>()?;
        let mut colors = Vec::new();
        if let Some(m) = op.get("cores") {
            let m = m
                .as_object()
                .ok_or_else(|| err("\"cores\" é um objeto {\"R\": \"#ff0044\", ...}".into()))?;
            for (k, v) in m {
                let mut it = k.chars();
                let (Some(ch), None) = (it.next(), it.next()) else {
                    return Err(err(format!("\"{k}\" em cores: use um caractere só")));
                };
                colors.push((ch, color(v, pal).map_err(err)?));
            }
        }
        // As cores da chamada (as da operação valem primeiro).
        for (ch, c) in shared {
            if !colors.iter().any(|(k, _)| k == ch) {
                colors.push((*ch, *c));
            }
        }
        // Sem cores: os dígitos 1 a 9 são as cores da paleta.
        for (i, c) in pal.iter().enumerate().take(9) {
            let ch = char::from_digit(i as u32 + 1, 10).unwrap_or('?');
            if !colors.iter().any(|(k, _)| *k == ch) {
                colors.push((ch, *c));
            }
        }
        let at = match op.get("em") {
            Some(v) => ints(v, 2, "\"em\" ([x, y])").map_err(err)?,
            None => vec![0, 0],
        };
        px::draw_text_art(img, &rows, &colors, at[0] + ox, at[1] + oy, clip).map_err(|ch| {
            err(format!(
                "o caractere '{ch}' do desenho não tem cor: ponha em \"cores\" (\".\" e espaço são transparentes; 1 a 9, as cores da paleta)"
            ))
        })?;
    } else if let Some(v) = op.get("copiar_quadro") {
        let from = v
            .as_u64()
            .ok_or_else(|| err("\"copiar_quadro\" é o número do quadro".into()))?
            as u32;
        let src_rect = sheet_cell(&before, sheet.anim.frame_w, sheet.anim.frame_h, from)
            .ok_or_else(|| err(format!("não há o quadro {from}")))?;
        let src = px::crop(&before, src_rect);
        px::paste(img, &src, clip.x as i32, clip.y as i32, clip, true);
    } else {
        let keys: Vec<&String> = op.keys().collect();
        return Err(err(format!(
            "não sei fazer {keys:?}; use uma de: pontos, linha, retangulo, elipse, balde, apagar, trocar_cor, espelhar, girar, simetria, deslocar, contorno, desenho, copiar_quadro"
        )));
    }
    Ok(())
}

/// As cores dos caracteres de um desenho: {"R": "#b13e53", ...}.
fn text_colors(v: Option<&Value>, pal: &[Color]) -> Result<Vec<(char, Color)>, String> {
    let mut out = Vec::new();
    let Some(v) = v.filter(|v| !v.is_null()) else {
        return Ok(out);
    };
    let m = v
        .as_object()
        .ok_or("\"cores\" é um objeto {\"R\": \"#ff0044\", ...}")?;
    for (k, v) in m {
        let mut it = k.chars();
        let (Some(ch), None) = (it.next(), it.next()) else {
            return Err(format!("\"{k}\" em cores: use um caractere só"));
        };
        out.push((ch, color(v, pal)?));
    }
    Ok(out)
}

fn sheet_cell(img: &RgbaImage, fw: u32, fh: u32, n: u32) -> Option<PixRect> {
    let total = px::frame_count(img, fw, fh);
    (n >= 1 && n <= total).then(|| px::cell(img, fw, fh, n - 1))
}

fn count_changes(a: &RgbaImage, b: &RgbaImage) -> usize {
    a.pixels().zip(b.pixels()).filter(|(x, y)| x != y).count()
}

fn desenhar(s: &mut Server, a: &Value) -> Result<Output, String> {
    let mut sheet = s.load_sheet(arg_str(a, "caminho")?)?;
    let frame = opt_num(a, "quadro")?.map(|n| n as u32);
    let clip = sheet.area(frame)?;
    let pal = palette(s, opt_str(a, "paleta")?)?;
    let ops = a
        .get("operacoes")
        .and_then(Value::as_array)
        .ok_or("falta \"operacoes\": uma lista, como [{\"retangulo\": [0, 0, 16, 4], \"cor\": \"#3b5dc9\"}]")?;
    if ops.is_empty() {
        return Err("\"operacoes\" está vazia".into());
    }
    let shared = text_colors(a.get("cores"), &pal)?;
    let before = sheet.img.clone();
    let mut touched = vec![clip];
    for (i, op) in ops.iter().enumerate() {
        let op = op
            .as_object()
            .ok_or_else(|| format!("operação {}: precisa ser um objeto", i + 1))?;
        // Uma operação pode ir para outro quadro.
        let clip = match op.get("quadro") {
            Some(v) => {
                let n = v
                    .as_u64()
                    .ok_or_else(|| format!("operação {}: \"quadro\" é um número", i + 1))?;
                sheet
                    .area(Some(n as u32))
                    .map_err(|e| format!("operação {}: {e}", i + 1))?
            }
            None => clip,
        };
        if !touched.contains(&clip) {
            touched.push(clip);
        }
        draw_op(&mut sheet, clip, op, &pal, &shared, i + 1)?;
    }
    let changed = count_changes(&before, &sheet.img);
    s.save_sheet(&sheet)?;
    let where_ = match frame {
        Some(n) => format!(" (quadro {n})"),
        None if sheet.frames() > 1 => " (quadro 1)".into(),
        None => String::new(),
    };
    let mut out = format!(
        "Desenhei em {}{where_}: {} operação(ões), {changed} pixel(s) mudaram.",
        sheet.rel,
        ops.len()
    );
    if opt_bool(a, "previa")?.unwrap_or(true) {
        let (fw, fh) = (clip.w, clip.h);
        let mut strip = RgbaImage::new(fw * touched.len() as u32, fh);
        for (i, r) in touched.iter().enumerate() {
            let part = px::crop(&sheet.img, *r);
            image::imageops::replace(&mut strip, &part, (i as u32 * fw) as i64, 0);
        }
        if touched.len() > 1 {
            out.push_str(&format!(" Os {} quadros mexidos, na ordem:", touched.len()));
        } else {
            out.push_str(&format!(" O quadro agora ({fw}x{fh}):"));
        }
        let png = preview_png(&strip, fw, fh, None)?;
        return Ok(Output::text(out).and_image(&png, "image/png"));
    }
    Ok(Output::text(out))
}

// ver_imagem ------------------------------------------------------------------

fn ver_imagem(s: &mut Server, a: &Value) -> Result<Output, String> {
    let sheet = s.load_sheet(arg_str(a, "caminho")?)?;
    let frame = opt_num(a, "quadro")?.map(|n| n as u32);
    let (w, h) = sheet.img.dimensions();
    let total = sheet.frames();
    let (fw, fh) = sheet.frame_size();
    let mut out = format!("{}: {w}x{h} pixels", sheet.rel);
    if total > 1 {
        let (cols, rows) = px::grid(&sheet.img, fw, fh);
        out.push_str(&format!(
            ", {total} quadros de {fw}x{fh} ({cols} por linha, {rows} linha(s); numerados da esquerda para a direita, a partir de 1)"
        ));
    }
    out.push_str(".\n");
    out.push_str(&format!(
        "Num sprite, o centro do quadro fica na posição do objeto; o tamanho na cena é {fw}x{fh} vezes a escala (ou w e h do sprite).\n"
    ));
    if sheet.anim.anims.is_empty() {
        out.push_str("Animações: nenhuma (crie com animar).\n");
    } else {
        out.push_str("Animações:\n");
        for an in &sheet.anim.anims {
            out.push_str(&format!("  {}\n", anim_line(an)));
        }
    }
    let area = match frame {
        Some(_) => sheet.area(frame)?,
        None => PixRect::of(&sheet.img),
    };
    let part = px::crop(&sheet.img, area);
    let colors = color_counts(&part);
    if colors.is_empty() {
        out.push_str("Cores: nenhuma (tudo transparente).\n");
    } else {
        let list: Vec<String> = colors
            .iter()
            .take(32)
            .map(|(c, n)| format!("{} ({n})", hex(*c)))
            .collect();
        out.push_str(&format!(
            "Cores ({}, com quantos pixels){}: {}\n",
            colors.len(),
            if colors.len() > 32 {
                ", as 32 mais usadas"
            } else {
                ""
            },
            list.join(", ")
        ));
    }
    if opt_bool(a, "texto")?.unwrap_or(false) {
        if part.width() > MAX_TEXT_SIDE || part.height() > MAX_TEXT_SIDE {
            out.push_str(&format!(
                "(Como texto, só até {MAX_TEXT_SIDE}x{MAX_TEXT_SIDE}: escolha um quadro.)\n"
            ));
        } else {
            let (rows, legend) = px::as_text_art(&part, PixRect::of(&part));
            // Com "cores", os mesmos caracteres de quem desenhou.
            let given = text_colors(a.get("cores"), &palette(s, opt_str(a, "paleta")?)?)?;
            let (rows, legend) = if given.is_empty() {
                (rows, legend)
            } else {
                rename_keys(rows, legend, &given)
            };
            out.push_str("\nComo texto (um caractere por pixel; \".\" é transparente):\n");
            for r in rows {
                out.push_str(&r);
                out.push('\n');
            }
            let l: Vec<String> = legend
                .iter()
                .map(|(k, c)| format!("{k} = {}", hex(*c)))
                .collect();
            out.push_str(&format!("{}\n", l.join(", ")));
        }
    }
    let scale = opt_num(a, "escala")?.map(|n| n as u32);
    if let Some(v) = a.get("repetir").filter(|v| !v.is_null()) {
        // Lado a lado, para ver as emendas de um tile.
        let p = ints(v, 2, "\"repetir\" ([colunas, linhas])")?;
        let (nx, ny) = (p[0].clamp(1, 8) as u32, p[1].clamp(1, 8) as u32);
        let tile = if frame.is_some() || total <= 1 {
            part.clone()
        } else {
            px::crop(&sheet.img, sheet.area(Some(1))?)
        };
        let (tw, th) = tile.dimensions();
        let mut big = RgbaImage::new(tw * nx, th * ny);
        for j in 0..ny {
            for i in 0..nx {
                image::imageops::replace(&mut big, &tile, (i * tw) as i64, (j * th) as i64);
            }
        }
        let png = preview_png(&big, 0, 0, scale)?;
        out.push_str(&format!(
            "\nO quadro repetido {nx}x{ny} (para ver as emendas):"
        ));
        return Ok(Output::text(out).and_image(&png, "image/png"));
    }
    let png = match frame {
        Some(_) => preview_png(&part, 0, 0, scale)?,
        None => preview_png(&sheet.img, fw, fh, scale)?,
    };
    out.push_str(
        "\nA imagem ampliada (o xadrez é o transparente; a linha rosa separa os quadros):",
    );
    Ok(Output::text(out).and_image(&png, "image/png"))
}

/// Troca os caracteres automáticos pelos de `given` (mesma cor), sem repetir.
fn rename_keys(
    rows: Vec<String>,
    legend: Vec<(char, Color)>,
    given: &[(char, Color)],
) -> (Vec<String>, Vec<(char, Color)>) {
    let mut map: Vec<(char, char)> = Vec::new();
    let mut used: Vec<char> = given.iter().map(|(k, _)| *k).collect();
    let spare = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789#@$%&*+=?!";
    let mut new_legend = Vec::new();
    for (k, c) in &legend {
        let to = match given.iter().find(|(_, g)| g == c) {
            Some((g, _)) => *g,
            None => {
                let ch = spare.chars().find(|x| !used.contains(x)).unwrap_or('?');
                used.push(ch);
                ch
            }
        };
        map.push((*k, to));
        new_legend.push((to, *c));
    }
    let rows = rows
        .into_iter()
        .map(|r| {
            r.chars()
                .map(|ch| {
                    map.iter()
                        .find(|(k, _)| *k == ch)
                        .map(|(_, t)| *t)
                        .unwrap_or(ch)
                })
                .collect()
        })
        .collect();
    (rows, new_legend)
}

fn color_counts(img: &RgbaImage) -> Vec<(Color, usize)> {
    let mut v: Vec<(Color, usize)> = Vec::new();
    for p in img.pixels() {
        if p.0[3] == 0 {
            continue;
        }
        match v.iter_mut().find(|(c, _)| *c == p.0) {
            Some((_, n)) => *n += 1,
            None => v.push((p.0, 1)),
        }
    }
    v.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    v
}

fn mode_label(m: Mode) -> &'static str {
    match m {
        Mode::Loop => "repetir",
        Mode::Once => "uma_vez",
        Mode::PingPong => "ida_e_volta",
    }
}

fn anim_line(an: &Anim) -> String {
    let frames: Vec<String> = an.frames.iter().map(|f| f.index.to_string()).collect();
    format!(
        "\"{}\": quadros [{}], {} fps, {} ({:.2} s)",
        an.name,
        frames.join(", "),
        nn_projeto::scene_data::format_num(an.fps),
        mode_label(an.mode),
        an.duration()
    )
}

// quadros ---------------------------------------------------------------------

fn quadros(s: &mut Server, a: &Value) -> Result<Output, String> {
    let mut sheet = s.load_sheet(arg_str(a, "caminho")?)?;
    let mut done = Vec::new();
    if let Some(v) = a.get("tamanho_quadro").filter(|v| !v.is_null()) {
        let p = ints(v, 2, "\"tamanho_quadro\" ([largura, altura])")?;
        let (w, h) = (p[0].max(0) as u32, p[1].max(0) as u32);
        if w == 0 || h == 0 || w > sheet.img.width() || h > sheet.img.height() {
            return Err(format!(
                "o quadro vai de 1x1 até o tamanho da imagem ({}x{})",
                sheet.img.width(),
                sheet.img.height()
            ));
        }
        sheet.anim.frame_w = w;
        sheet.anim.frame_h = h;
        done.push(format!("quadros de {w}x{h}: agora são {}", sheet.frames()));
    }
    if let Some(v) = a.get("tamanho_imagem").filter(|v| !v.is_null()) {
        let p = ints(v, 2, "\"tamanho_imagem\" ([largura, altura])")?;
        let (w, h) = (p[0].max(1) as u32, p[1].max(1) as u32);
        if w > MAX_SIDE || h > MAX_SIDE {
            return Err(format!("a imagem vai até {MAX_SIDE}x{MAX_SIDE}"));
        }
        sheet.img = px::resize_canvas(&sheet.img, w, h);
        done.push(format!(
            "imagem com {w}x{h} (o desenho fica no canto de cima à esquerda)"
        ));
    }
    if let Some(list) = a.get("operacoes").and_then(Value::as_array) {
        for (i, op) in list.iter().enumerate() {
            let n = i + 1;
            let total = sheet.frames();
            let num = |k: &str| op.get(k).and_then(Value::as_u64).map(|x| x as u32);
            if let Some(at) = num("inserir") {
                if at == 0 || at > total + 1 {
                    return Err(format!(
                        "operação {n}: \"inserir\" vai de 1 a {}",
                        total + 1
                    ));
                }
                let copy = num("copiar").map(|c| c.saturating_sub(1));
                px::insert_frame(&mut sheet.img, &mut sheet.anim, at - 1, copy);
                done.push(match num("copiar") {
                    Some(c) => format!("quadro novo {at}, cópia do {c}"),
                    None => format!("quadro novo {at}, vazio"),
                });
            } else if let Some(at) = num("tirar") {
                if at == 0 || at > total || total <= 1 {
                    return Err(format!(
                        "operação {n}: não dá para tirar o quadro {at} (tem {total})"
                    ));
                }
                px::remove_frame(&mut sheet.img, &mut sheet.anim, at - 1);
                done.push(format!("tirei o quadro {at}"));
            } else if let Some(v) = op.get("trocar") {
                let p = ints(v, 2, "\"trocar\" ([a, b])")?;
                let (x, y) = (p[0].max(0) as u32, p[1].max(0) as u32);
                if x == 0 || y == 0 || x > total || y > total {
                    return Err(format!("operação {n}: os quadros vão de 1 a {total}"));
                }
                px::swap_frames(&mut sheet.img, &mut sheet.anim, x - 1, y - 1);
                done.push(format!("troquei os quadros {x} e {y}"));
            } else {
                return Err(format!(
                    "operação {n}: use {{\"inserir\": posição, \"copiar\": quadro}}, {{\"tirar\": quadro}} ou {{\"trocar\": [a, b]}}"
                ));
            }
        }
    }
    if done.is_empty() {
        return Err("diga o que fazer: operacoes, tamanho_quadro ou tamanho_imagem".into());
    }
    s.save_sheet(&sheet)?;
    let (fw, fh) = sheet.frame_size();
    let out = format!(
        "{}: {}. Agora: {}x{} pixels, {} quadro(s) de {fw}x{fh}. As animações acompanharam a numeração.",
        sheet.rel,
        done.join("; "),
        sheet.img.width(),
        sheet.img.height(),
        sheet.frames()
    );
    let png = preview_png(&sheet.img, fw, fh, None)?;
    Ok(Output::text(out).and_image(&png, "image/png"))
}

// animar ----------------------------------------------------------------------

fn animar(s: &mut Server, a: &Value) -> Result<Output, String> {
    let mut sheet = s.load_sheet(arg_str(a, "caminho")?)?;
    let name = arg_str(a, "nome")?.trim().to_string();
    if name.is_empty() {
        return Err("a animação precisa de um nome (\"andar\", \"pular\"...)".into());
    }
    let total = sheet.frames();
    if opt_bool(a, "apagar")?.unwrap_or(false) {
        let before = sheet.anim.anims.len();
        sheet.anim.anims.retain(|x| x.name != name);
        if sheet.anim.anims.len() == before {
            return Err(format!("{} não tem a animação \"{name}\"", sheet.rel));
        }
        s.save_sheet(&sheet)?;
        return Ok(Output::text(format!(
            "Apaguei a animação \"{name}\" de {}.",
            sheet.rel
        )));
    }
    if total <= 1 {
        return Err(format!(
            "{} tem um quadro só; para animar, divida em quadros (quadros com tamanho_quadro) ou crie a imagem com \"quadros\"",
            sheet.rel
        ));
    }
    let existing = sheet.anim.anims.iter().position(|x| x.name == name);
    let mut an = existing
        .map(|i| sheet.anim.anims[i].clone())
        .unwrap_or_else(|| Anim::new(&name, 1..=total));
    if let Some(v) = a.get("quadros").filter(|v| !v.is_null()) {
        let list = v.as_array().filter(|l| !l.is_empty()).ok_or(
            "\"quadros\" é uma lista com os números dos quadros, na ordem (ex.: [1, 2, 3, 2])",
        )?;
        let mut frames = Vec::new();
        for f in list {
            let i = f.as_u64().ok_or("cada quadro é um número")? as u32;
            if i == 0 || i > total {
                return Err(format!(
                    "{} tem os quadros 1 a {total}; não há o {i}",
                    sheet.rel
                ));
            }
            frames.push(Frame {
                index: i,
                time: None,
            });
        }
        an.frames = frames;
    }
    if let Some(fps) = opt_num(a, "fps")? {
        if !(0.1..=60.0).contains(&fps) {
            return Err("\"fps\" vai de 0,1 a 60".into());
        }
        an.fps = fps;
    }
    if let Some(m) = opt_str(a, "modo")? {
        an.mode = match m {
            "repetir" => Mode::Loop,
            "uma_vez" => Mode::Once,
            "ida_e_volta" => Mode::PingPong,
            _ => return Err("\"modo\" é \"repetir\", \"uma_vez\" ou \"ida_e_volta\"".into()),
        };
    }
    if let Some(v) = a.get("tempos").filter(|v| !v.is_null()) {
        let list = v
            .as_array()
            .ok_or("\"tempos\" é uma lista de segundos, um por quadro")?;
        if list.len() != an.frames.len() {
            return Err(format!(
                "\"tempos\" precisa de um tempo para cada quadro ({})",
                an.frames.len()
            ));
        }
        for (f, t) in an.frames.iter_mut().zip(list) {
            f.time = t.as_f64().filter(|t| *t > 0.0);
        }
    }
    match existing {
        Some(i) => sheet.anim.anims[i] = an.clone(),
        None => sheet.anim.anims.push(an.clone()),
    }
    s.save_sheet(&sheet)?;
    // A tira: os quadros na ordem em que tocam.
    let (fw, fh) = sheet.frame_size();
    let mut strip = RgbaImage::new(fw * an.frames.len() as u32, fh);
    for (i, f) in an.frames.iter().enumerate() {
        if let Some(r) = sheet_cell(&sheet.img, sheet.anim.frame_w, sheet.anim.frame_h, f.index) {
            image::imageops::replace(
                &mut strip,
                &px::crop(&sheet.img, r),
                (i as u32 * fw) as i64,
                0,
            );
        }
    }
    let png = preview_png(&strip, fw, fh, None)?;
    let out = format!(
        "{} \"{name}\" em {}: {}.\nNum objeto: {{\"type\": \"sprite\", \"image\": \"{}\", \"animation\": \"{name}\"}} toca desde o começo; com o comportamento nn:animacao, ela troca sozinha (parado, andar, pular); em Lua, obj.sprite:play(\"{name}\"). A tira abaixo mostra os quadros na ordem em que tocam:",
        if existing.is_some() {
            "Mudei a animação"
        } else {
            "Criei a animação"
        },
        anim_path(&sheet.rel),
        anim_line(&an),
        sheet.rel
    );
    Ok(Output::text(out).and_image(&png, "image/png"))
}

const CAMINHO: &str = "A imagem (.png), ex.: \"assets/imagens/heroi.png\"";

pub static TOOLS: &[&Tool] = &[
    &Tool {
        name: "criar_imagem",
        title: "Criar imagem",
        description: "Cria uma imagem de pixel art (.png) vazia (ou com um fundo), com um ou mais quadros do mesmo tamanho lado a lado (para animar). Depois, desenhe com desenhar, veja com ver_imagem e anime com animar. Tamanhos comuns: 16x16 ou 32x32 para personagens e itens, 16x16 para tiles.",
        params: || {
            vec![
                ("caminho", s_str(CAMINHO), true),
                (
                    "largura",
                    s_int("Largura de um quadro, em pixels (padrão 16)"),
                    false,
                ),
                ("altura", s_int("Altura (padrão: igual à largura)"), false),
                (
                    "quadros",
                    s_int("Quantos quadros lado a lado (padrão 1)"),
                    false,
                ),
                ("fundo", s_str("Cor de fundo (padrão transparente)"), false),
                (
                    "paleta",
                    s_str(
                        "Para cores pelo número: \"projeto\" (paleta.hex), \"Sweetie 16\", \"PICO-8\" ou \"Endesga 32\"",
                    ),
                    false,
                ),
                (
                    "substituir",
                    s_bool("Se a imagem já existe, começar de novo (padrão false)"),
                    false,
                ),
            ]
        },
        read_only: false,
        destructive: false,
        run: criar_imagem,
    },
    &Tool {
        name: "desenhar",
        title: "Desenhar",
        description: "Desenha numa imagem do projeto, num quadro (x para a direita, y para baixo, (0, 0) no canto de cima à esquerda do quadro), com várias operações numa chamada, e devolve o quadro ampliado. Operações (cada uma com \"cor\" quando pinta: \"#rrggbb\", \"transparente\" ou o número da cor na paleta): {\"pontos\": [[x, y], ...]}, {\"linha\": [x1, y1, x2, y2]}, {\"retangulo\": [x, y, largura, altura], \"cheio\": true}, {\"elipse\": [x, y, largura, altura]}, {\"balde\": [x, y]}, {\"apagar\": [x, y, largura, altura]} ou {\"apagar\": true}, {\"trocar_cor\": \"#de\", \"cor\": \"#para\"}, {\"espelhar\": \"horizontal\"}, {\"girar\": 90}, {\"simetria\": \"esquerda\"} (copia a metade esquerda espelhada na direita), {\"deslocar\": [dx, dy]}, {\"contorno\": true, \"cor\": \"#000000\"} (em volta do desenho), {\"copiar_quadro\": 2}, e o mais prático para sprites: {\"desenho\": [\"..RR..\", \".RBBR.\", ...], \"cores\": {\"R\": \"#b13e53\", \"B\": \"#41a6f6\"}, \"em\": [x, y]} (um caractere por pixel; \".\" não pinta; sem \"cores\", 1 a 9 são as cores da paleta).",
        params: || {
            vec![
                ("caminho", s_str(CAMINHO), true),
                (
                    "operacoes",
                    s_list(
                        "As operações, na ordem (veja a descrição)",
                        s_obj("Uma operação"),
                    ),
                    true,
                ),
                (
                    "quadro",
                    s_int(
                        "Em que quadro (a partir de 1; padrão o 1º, ou a imagem toda se não tiver quadros). Uma operação também pode ter o seu \"quadro\"",
                    ),
                    false,
                ),
                (
                    "cores",
                    s_obj(
                        "As cores dos caracteres de todos os \"desenho\" desta chamada: {\"R\": \"#b13e53\", ...}",
                    ),
                    false,
                ),
                (
                    "paleta",
                    s_str(
                        "Para cores pelo número: \"projeto\", \"Sweetie 16\" (padrão sem paleta.hex), \"PICO-8\" ou \"Endesga 32\"",
                    ),
                    false,
                ),
                (
                    "previa",
                    s_bool("Devolver o quadro ampliado (padrão true)"),
                    false,
                ),
            ]
        },
        read_only: false,
        destructive: false,
        run: desenhar,
    },
    &Tool {
        name: "ver_imagem",
        title: "Ver imagem",
        description: "Mostra uma imagem do projeto ampliada (pixels grandes, xadrez no transparente, linha rosa entre os quadros), com o tamanho, os quadros, as animações e as cores usadas. Com \"texto\": true, também o quadro como texto, um caractere por pixel, com a cor de cada caractere: bom para conferir pixel a pixel e para copiar num desenhar.",
        params: || {
            vec![
                ("caminho", s_str(CAMINHO), true),
                ("quadro", s_int("Só este quadro (a partir de 1)"), false),
                ("texto", s_bool("Também como texto (até 64x64)"), false),
                (
                    "cores",
                    s_obj(
                        "Com texto: o caractere de cada cor ({\"R\": \"#b13e53\"}), como no desenhar",
                    ),
                    false,
                ),
                (
                    "repetir",
                    s_list(
                        "[colunas, linhas]: o quadro repetido lado a lado, para ver as emendas de um tile",
                        json!({ "type": "integer" }),
                    ),
                    false,
                ),
                (
                    "escala",
                    s_int("Quantos pixels da tela por pixel (padrão: o que couber em 512)"),
                    false,
                ),
            ]
        },
        read_only: true,
        destructive: false,
        run: ver_imagem,
    },
    &Tool {
        name: "quadros",
        title: "Quadros da imagem",
        description: "Mexe nos quadros de uma folha de sprites: {\"inserir\": posição, \"copiar\": quadro} (um quadro novo, vazio ou cópia de outro, bom para a próxima pose de uma animação), {\"tirar\": quadro}, {\"trocar\": [a, b]}; e \"tamanho_quadro\" (divide a imagem em quadros desse tamanho) ou \"tamanho_imagem\" (aumenta ou corta a imagem). As animações acompanham a numeração.",
        params: || {
            vec![
                ("caminho", s_str(CAMINHO), true),
                (
                    "operacoes",
                    s_list(
                        "O que fazer, na ordem",
                        s_obj(
                            "{\"inserir\": n, \"copiar\": m}, {\"tirar\": n} ou {\"trocar\": [a, b]}",
                        ),
                    ),
                    false,
                ),
                (
                    "tamanho_quadro",
                    s_list(
                        "[largura, altura] de cada quadro",
                        json!({ "type": "integer" }),
                    ),
                    false,
                ),
                (
                    "tamanho_imagem",
                    s_list("[largura, altura] da imagem", json!({ "type": "integer" })),
                    false,
                ),
            ]
        },
        read_only: false,
        destructive: false,
        run: quadros,
    },
    &Tool {
        name: "animar",
        title: "Animar",
        description: "Cria ou muda uma animação com nome de uma folha de sprites (o arquivo .anim ao lado da imagem, o mesmo do NN Studio): os quadros na ordem, a velocidade, como repete e o tempo de cada quadro. Devolve os quadros na ordem em que tocam. Nomes que o comportamento nn:animacao usa sozinho: parado, andar, pular, cair.",
        params: || {
            vec![
                ("caminho", s_str(CAMINHO), true),
                (
                    "nome",
                    s_str("O nome da animação (\"andar\", \"pular\"...)"),
                    true,
                ),
                (
                    "quadros",
                    s_list(
                        "Os quadros, na ordem (padrão: todos)",
                        json!({ "type": "integer" }),
                    ),
                    false,
                ),
                ("fps", s_num("Quadros por segundo (padrão 10)"), false),
                (
                    "modo",
                    s_enum(
                        "Como toca (padrão repetir)",
                        &["repetir", "uma_vez", "ida_e_volta"],
                    ),
                    false,
                ),
                (
                    "tempos",
                    s_list(
                        "Segundos de cada quadro (em vez do fps)",
                        json!({ "type": "number" }),
                    ),
                    false,
                ),
                ("apagar", s_bool("Apagar a animação"), false),
            ]
        },
        read_only: false,
        destructive: true,
        run: animar,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cores() {
        let pal = px::parse_hex_list(px::PALETTES[1].1);
        assert_eq!(color(&json!("#ff0044"), &pal).unwrap(), [255, 0, 68, 255]);
        assert_eq!(color(&json!("ff004480"), &pal).unwrap(), [255, 0, 68, 128]);
        assert_eq!(color(&json!("Transparente"), &pal).unwrap(), CLEAR);
        assert_eq!(color(&json!(1), &pal).unwrap(), [0, 0, 0, 255]);
        assert!(color(&json!(99), &pal).unwrap_err().contains("16 cores"));
        assert!(color(&json!("vermelho"), &pal).is_err());
    }
}
