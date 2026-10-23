//! Modelos 3D (`.modelo`, os do modelador do NN Studio): criar, modelar com
//! operações de peças e faces, e ver (também qualquer `.glb`), desenhados
//! sem placa de vídeo em quatro vistas. Grava o `.modelo` e o `.glb` que o
//! jogo usa.

use glam::{Mat4, Vec3};
use serde_json::{Map, Value};

use nn_projeto::model::{self, ModelData, Part, face_normal, glb_path};
use nn_projeto::model_kits::{KITS, READY};
use nn_projeto::model_view::{self, View};

use super::{
    Output, Tool, arg_str, opt_bool, opt_num, opt_str, s_bool, s_list, s_num, s_obj, s_str,
};
use crate::Server;

/// Um número de metros com até 2 casas.
fn m(v: f32) -> String {
    let r = (v * 100.0).round() / 100.0;
    if r == r.trunc() {
        format!("{r:.0}")
    } else {
        format!("{r}")
    }
}

fn vec3(v: &Value, what: &str) -> Result<Vec3, String> {
    match v {
        Value::Number(n) => {
            let f = n.as_f64().unwrap_or(0.0) as f32;
            Ok(Vec3::splat(f))
        }
        Value::Array(a) if a.len() == 3 => {
            let mut out = [0.0f32; 3];
            for (o, x) in out.iter_mut().zip(a) {
                *o = x.as_f64().ok_or_else(|| format!("{what}: só números"))? as f32;
            }
            Ok(Vec3::from_array(out))
        }
        _ => Err(format!("{what} é [x, y, z] (metros)")),
    }
}

fn png(img: &image::RgbaImage) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

/// As direções com nome.
fn side(name: &str) -> Result<Vec3, String> {
    Ok(match crate::paths::fold(name).as_str() {
        "cima" | "topo" => Vec3::Y,
        "baixo" | "base" => Vec3::NEG_Y,
        "frente" => Vec3::Z,
        "tras" | "atras" | "costas" => Vec3::NEG_Z,
        "direita" => Vec3::X,
        "esquerda" => Vec3::NEG_X,
        _ => {
            return Err(format!(
                "lado \"{name}\": use cima, baixo, frente (+Z), tras, direita (+X) ou esquerda"
            ));
        }
    })
}

/// As faces da peça viradas para `dir` (no espaço do modelo): as quase tão
/// viradas quanto a mais virada (o topo de um cubo; todas as faces de uma
/// ponta de cone).
fn faces_towards(p: &Part, dir: Vec3) -> Vec<usize> {
    let rot = p.rotation();
    let dots: Vec<f32> = (0..p.faces.len())
        .map(|i| (rot * face_normal(&p.verts, &p.faces[i].v)).dot(dir))
        .collect();
    let best = dots.iter().copied().fold(f32::MIN, f32::max);
    (0..dots.len())
        .filter(|&i| dots[i] > 0.05 && dots[i] >= best - 0.15)
        .collect()
}

/// Uma forma pronta, do tamanho pedido, com a base em y = 0.
fn shape(
    name: &str,
    forma: &str,
    color: &str,
    size: Vec3,
    sides: Option<u32>,
) -> Result<Part, String> {
    let n = sides.unwrap_or(8).clamp(3, 48);
    let p = match crate::paths::fold(forma).as_str() {
        "cubo" | "caixa" => Part::cube(name, color),
        "cilindro" => Part::ring_solid(name, color, n, 1.0),
        "cone" => Part::ring_solid(name, color, n, 0.0),
        "piramide" => Part::pyramid(name, color),
        "esfera" | "bola" => Part::sphere_n(name, color, n.max(6), (n / 2).max(3)),
        "plano" => Part::plane(name, color),
        "rampa" | "cunha" => Part::ramp(name, color),
        _ => {
            return Err(format!(
                "forma \"{forma}\": use cubo, cilindro, cone, piramide, esfera, plano ou rampa"
            ));
        }
    };
    Ok(p.baked(Mat4::from_scale(size)))
}

/// A peça de um kit ("Casa/Parede" ou só "Parede").
fn kit_parts(name: &str) -> Result<Vec<Part>, String> {
    let wanted = crate::paths::fold(name);
    let (kit, item) = match wanted.split_once('/') {
        Some((k, i)) => (Some(k.trim().to_string()), i.trim().to_string()),
        None => (None, wanted.trim().to_string()),
    };
    for k in KITS {
        if kit
            .as_ref()
            .is_some_and(|w| *w != crate::paths::fold(k.name))
        {
            continue;
        }
        for it in k.items {
            if crate::paths::fold(it.label) == item {
                return Ok((it.make)());
            }
        }
    }
    Err(format!(
        "não há a peça \"{name}\" nos kits. Peças: {}",
        kit_list()
    ))
}

fn kit_list() -> String {
    KITS.iter()
        .map(|k| {
            let items: Vec<&str> = k.items.iter().map(|i| i.label).collect();
            format!("{}: {}", k.name, items.join(", "))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn ready_list() -> String {
    READY.iter().map(|r| r.file).collect::<Vec<_>>().join(", ")
}

/// A peça pelo nome (sem ligar para acentos e maiúsculas).
fn part_index(md: &ModelData, name: &str) -> Result<usize, String> {
    if let Some(i) = md.parts.iter().position(|p| p.name == name) {
        return Ok(i);
    }
    let f = crate::paths::fold(name);
    md.parts
        .iter()
        .position(|p| crate::paths::fold(&p.name) == f)
        .ok_or_else(|| {
            let names: Vec<&str> = md.parts.iter().map(|p| p.name.as_str()).collect();
            format!("não há a peça \"{name}\" (tem: {})", names.join(", "))
        })
}

/// Um nome que ainda não existe ("Braço", "Braço 2"...).
fn free_name(md: &ModelData, base: &str) -> String {
    let mut name = base.to_string();
    let mut n = 1;
    while md.parts.iter().any(|p| p.name == name) {
        n += 1;
        name = format!("{base} {n}");
    }
    name
}

/// A peça espelhada em X (pontos, posição e giro).
fn mirrored(p: &Part) -> Part {
    let mut q = p.clone();
    for v in &mut q.verts {
        v.x = -v.x;
    }
    for f in &mut q.faces {
        f.v.reverse();
    }
    q.pos.x = -q.pos.x;
    q.rot.y = -q.rot.y;
    q.rot.z = -q.rot.z;
    q
}

/// Leva os pontos para o espaço do modelo (com o espelho feito) e zera a
/// posição, o giro e a escala da peça.
fn baked_part(p: &Part) -> Part {
    let mut q = p.clone();
    if q.mirror {
        q.mirror_apply();
    }
    let mtx = q.matrix();
    let mut out = q.baked(mtx);
    out.pos = Vec3::ZERO;
    out.rot = Vec3::ZERO;
    out.scale = Vec3::ONE;
    out
}

/// A caixa da peça no espaço do modelo.
fn part_bounds(p: &Part) -> Option<(Vec3, Vec3)> {
    let mtx = p.matrix();
    let mut it = p
        .all_faces()
        .into_iter()
        .flat_map(|(pts, ..)| pts)
        .map(|v| mtx.transform_point3(v));
    let first = it.next()?;
    Some(it.fold((first, first), |(a, b), v| (a.min(v), b.max(v))))
}

// O texto de um modelo -------------------------------------------------------

fn describe(rel: &str, md: &ModelData) -> String {
    let mut out = String::new();
    match md.bounds() {
        Some((a, b)) => {
            let s = b - a;
            out.push_str(&format!(
                "{rel}: {} peça(s), {} triângulos. Tamanho {} x {} x {} m (largura em X, altura em Y, fundo em Z); vai de y = {} a {} (a origem, o ponto que fica na posição do objeto, é o 0).\n",
                md.parts.len(),
                md.triangle_count(),
                m(s.x),
                m(s.y),
                m(s.z),
                m(a.y),
                m(b.y)
            ));
            if a.y < -0.05 {
                out.push_str("  Aviso: parte do modelo fica abaixo de y = 0 (afunda no chão quando o objeto está no chão).\n");
            } else if a.y > 0.05 {
                out.push_str("  Aviso: a base fica acima de y = 0 (o modelo flutua quando o objeto está no chão).\n");
            }
        }
        None => out.push_str(&format!("{rel}: vazio (sem peças).\n")),
    }
    out.push_str("Peças (posição da base, giro em graus, tamanho):\n");
    for p in &md.parts {
        let size = part_bounds(p).map(|(a, b)| b - a).unwrap_or(Vec3::ZERO);
        let mut colors: Vec<&str> = p.faces.iter().map(|f| f.color.as_str()).collect();
        colors.sort_unstable();
        colors.dedup();
        out.push_str(&format!(
            "  \"{}\": em [{}, {}, {}], giro [{}, {}, {}], {} x {} x {} m, cores {}{}{}\n",
            p.name,
            m(p.pos.x),
            m(p.pos.y),
            m(p.pos.z),
            m(p.rot.x),
            m(p.rot.y),
            m(p.rot.z),
            m(size.x),
            m(size.y),
            m(size.z),
            colors.join(" "),
            if p.mirror { ", com simetria em X" } else { "" },
            if p.bone.is_empty() {
                String::new()
            } else {
                format!(", segue o osso {}", p.bone)
            }
        ));
    }
    if !md.bones.is_empty() {
        let names: Vec<&str> = md.bones.iter().map(|b| b.name.as_str()).collect();
        out.push_str(&format!("Ossos: {}.\n", names.join(", ")));
    }
    if !md.anims.is_empty() {
        let list: Vec<String> = md
            .anims
            .iter()
            .map(|a| {
                format!(
                    "{} ({} s{})",
                    a.name,
                    m(a.duration),
                    if a.looping { ", repete" } else { "" }
                )
            })
            .collect();
        out.push_str(&format!("Animações: {}.\n", list.join(", ")));
    }
    out
}

const VIEWS_NOTE: &str = "A imagem tem 4 vistas: em cima, a frente (de +Z) e o lado (de +X); embaixo, de cima (a frente para baixo) e na diagonal. As três retas têm a mesma escala, com uma grade de 1 m (o chão, y = 0, é a linha mais clara) e a origem em vermelho.";

fn views(md: &ModelData) -> Result<Vec<u8>, String> {
    let glb = md.to_glb();
    let data =
        nn_render::d3::ModelData::from_gltf(&glb, &mut |_| None).map_err(|e| e.to_string())?;
    png(&model_view::render(
        &model_view::triangles(&data, None),
        300,
    ))
}

impl Server {
    fn load_model(&self, rel: &str) -> Result<(String, ModelData), String> {
        let (rel, full) = crate::paths::resolve(&self.root, rel)?;
        if !rel.to_lowercase().ends_with(".modelo") {
            return Err(format!(
                "{rel} não é um modelo do modelador (.modelo); para um .glb, use ver_modelo"
            ));
        }
        if !full.is_file() {
            return Err(format!("{rel} não existe (crie com criar_modelo)"));
        }
        let text = std::fs::read_to_string(&full).map_err(|e| format!("{rel}: {e}"))?;
        let md = ModelData::from_lua(&text, &rel)?;
        Ok((rel, md))
    }

    /// Grava o `.modelo` e o `.glb` (o que o jogo usa).
    fn save_model(&mut self, rel: &str, md: &ModelData) -> Result<(), String> {
        self.write_file(rel, md.to_lua().as_bytes())?;
        self.write_file(&glb_path(rel), &md.to_glb())
    }
}

fn usage(rel: &str) -> String {
    format!(
        "Na cena 3D: um objeto com {{\"type\": \"model\", \"file\": \"{}\"}} (o .glb que acompanha o .modelo).",
        glb_path(rel)
    )
}

// criar_modelo ---------------------------------------------------------------

fn criar_modelo(s: &mut Server, a: &Value) -> Result<Output, String> {
    let (rel, full) = crate::paths::resolve(&s.root, arg_str(a, "caminho")?)?;
    if !rel.to_lowercase().ends_with(".modelo") {
        return Err("o caminho termina em .modelo (ex.: \"modelos/torre.modelo\")".into());
    }
    if full.exists() && !opt_bool(a, "substituir")?.unwrap_or(false) {
        return Err(format!(
            "{rel} já existe; mude com modelar (ou \"substituir\": true)"
        ));
    }
    let from = opt_str(a, "a_partir")?.unwrap_or("vazio");
    let md = if crate::paths::fold(from) == "vazio" {
        ModelData {
            parts: Vec::new(),
            palette: model::DEFAULT_PALETTE
                .iter()
                .map(|c| c.to_string())
                .collect(),
            ..Default::default()
        }
    } else if from.ends_with(".modelo") {
        s.load_model(from)?.1
    } else {
        let f = crate::paths::fold(from);
        let r = READY
            .iter()
            .find(|r| r.file == f || crate::paths::fold(r.name) == f)
            .ok_or_else(|| {
                format!(
                    "\"a_partir\": \"vazio\", um modelo pronto ({}) ou outro .modelo",
                    ready_list()
                )
            })?;
        (r.make)()
    };
    s.save_model(&rel, &md)?;
    let mut text = format!(
        "Criei {rel} (e {}). {}\n{}",
        glb_path(&rel),
        if md.parts.is_empty() {
            "Ainda sem peças: ponha com modelar."
        } else {
            ""
        },
        describe(&rel, &md)
    );
    text.push_str(&usage(&rel));
    if md.parts.is_empty() {
        return Ok(Output::text(text));
    }
    text.push_str(&format!("\n{VIEWS_NOTE}"));
    Ok(Output::text(text).and_image(&views(&md)?, "image/png"))
}

// modelar --------------------------------------------------------------------

fn op_err(n: usize, e: impl std::fmt::Display) -> String {
    format!("operação {n}: {e}")
}

fn get_str<'a>(op: &'a Map<String, Value>, k: &str, n: usize) -> Result<Option<&'a str>, String> {
    match op.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(_) => Err(op_err(n, format!("\"{k}\" é um texto"))),
    }
}

fn get_vec(op: &Map<String, Value>, k: &str, n: usize) -> Result<Option<Vec3>, String> {
    match op.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => vec3(v, &format!("\"{k}\""))
            .map(Some)
            .map_err(|e| op_err(n, e)),
    }
}

fn get_num(op: &Map<String, Value>, k: &str, n: usize) -> Result<Option<f32>, String> {
    match op.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_f64()
            .map(|f| Some(f as f32))
            .ok_or_else(|| op_err(n, format!("\"{k}\" é um número"))),
    }
}

/// Faz uma operação; devolve o que fez, em poucas palavras.
fn model_op(md: &mut ModelData, op: &Map<String, Value>, n: usize) -> Result<String, String> {
    let color = get_str(op, "cor", n)?;
    // Uma peça nova, de uma forma.
    if let Some(name) = get_str(op, "criar", n)? {
        let forma = get_str(op, "forma", n)?.unwrap_or("cubo");
        let size = get_vec(op, "tamanho", n)?.unwrap_or(Vec3::ONE);
        if size.min_element() <= 0.0 {
            return Err(op_err(n, "\"tamanho\" precisa ser maior que 0"));
        }
        let sides = get_num(op, "lados", n)?.map(|v| v.round() as u32);
        let name = if md.parts.iter().any(|p| p.name == name) {
            return Err(op_err(
                n,
                format!("já existe a peça \"{name}\"; use outro nome"),
            ));
        } else {
            name.to_string()
        };
        let mut p = shape(&name, forma, color.unwrap_or("#B8B8B8"), size, sides)
            .map_err(|e| op_err(n, e))?;
        p.pos = get_vec(op, "em", n)?.unwrap_or(Vec3::ZERO);
        p.rot = get_vec(op, "giro", n)?.unwrap_or(Vec3::ZERO);
        if op.get("simetria").and_then(Value::as_bool) == Some(true) {
            p.mirror_on();
        }
        md.parts.push(p);
        return Ok(format!("peça \"{name}\" ({forma})"));
    }
    // Peças de um kit.
    if let Some(kit) = get_str(op, "kit", n)? {
        let parts = kit_parts(kit).map_err(|e| op_err(n, e))?;
        let at = get_vec(op, "em", n)?.unwrap_or(Vec3::ZERO);
        let rot = get_vec(op, "giro", n)?;
        let single = parts.len() == 1;
        let mut names = Vec::new();
        for mut p in parts {
            let base = match (single, get_str(op, "nome", n)?) {
                (true, Some(nm)) => nm.to_string(),
                _ => p.name.clone(),
            };
            p.name = free_name(md, &base);
            p.pos += at;
            if let Some(r) = rot {
                p.rot = r;
            }
            if let Some(c) = color {
                for f in &mut p.faces {
                    f.color = c.to_string();
                }
            }
            names.push(format!("\"{}\"", p.name));
            md.parts.push(p);
        }
        return Ok(format!("do kit {kit}: {}", names.join(", ")));
    }
    if let Some(name) = get_str(op, "apagar", n)? {
        let i = part_index(md, name).map_err(|e| op_err(n, e))?;
        md.parts.remove(i);
        return Ok(format!("apaguei \"{name}\""));
    }
    if let Some(name) = get_str(op, "copiar", n)? {
        let i = part_index(md, name).map_err(|e| op_err(n, e))?;
        let mut p = md.parts[i].clone();
        let espelhar = get_str(op, "espelhar", n)?;
        if let Some(e) = espelhar {
            if crate::paths::fold(e) != "x" {
                return Err(op_err(
                    n,
                    "\"espelhar\" é \"x\" (o espelho do lado esquerdo e direito)",
                ));
            }
            p = mirrored(&p);
        }
        let base = match get_str(op, "nome", n)? {
            Some(nm) => nm.to_string(),
            None => p.name.clone(),
        };
        p.name = free_name(md, &base);
        if let Some(at) = get_vec(op, "em", n)? {
            p.pos = at;
        }
        if let Some(d) = get_vec(op, "andar", n)? {
            p.pos += d;
        }
        let name = p.name.clone();
        md.parts.push(p);
        return Ok(format!(
            "copiei \"{}\" em \"{name}\"{}",
            md.parts[i].name,
            if espelhar.is_some() {
                " (espelhada)"
            } else {
                ""
            }
        ));
    }
    if let Some(list) = op.get("juntar") {
        let names: Vec<&str> = list
            .as_array()
            .ok_or_else(|| op_err(n, "\"juntar\" é uma lista de peças"))?
            .iter()
            .filter_map(Value::as_str)
            .collect();
        if names.len() < 2 {
            return Err(op_err(n, "\"juntar\" precisa de pelo menos duas peças"));
        }
        let mut idx = Vec::new();
        for nm in &names {
            idx.push(part_index(md, nm).map_err(|e| op_err(n, e))?);
        }
        let mut merged = baked_part(&md.parts[idx[0]]);
        for &i in &idx[1..] {
            merged.merge(&baked_part(&md.parts[i]));
        }
        merged.name = get_str(op, "nome", n)?.unwrap_or(names[0]).to_string();
        idx.sort_unstable();
        let at = idx[0];
        for &i in idx.iter().rev() {
            md.parts.remove(i);
        }
        let name = merged.name.clone();
        md.parts.insert(at.min(md.parts.len()), merged);
        return Ok(format!("juntei {} em \"{name}\"", names.join(", ")));
    }
    // O resto mexe numa peça.
    let Some(name) = get_str(op, "peca", n)? else {
        return Err(op_err(
            n,
            "diga o que fazer: criar, kit, peca (com em, andar, giro, escala, tamanho, cor, extrudar, recuar, afinar, simetria, renomear), copiar, apagar ou juntar",
        ));
    };
    let i = part_index(md, name).map_err(|e| op_err(n, e))?;
    let mut done = Vec::new();
    {
        let p = &mut md.parts[i];
        if let Some(v) = get_vec(op, "em", n)? {
            p.pos = v;
            done.push("posição");
        }
        if let Some(v) = get_vec(op, "andar", n)? {
            p.pos += v;
            done.push("andou");
        }
        if let Some(v) = get_vec(op, "giro", n)? {
            p.rot = v;
            done.push("giro");
        }
        if let Some(v) = get_vec(op, "escala", n)? {
            if v.min_element() <= 0.0 {
                return Err(op_err(n, "\"escala\" precisa ser maior que 0"));
            }
            p.scale *= v;
            done.push("escala");
        }
        if let Some(v) = get_vec(op, "tamanho", n)? {
            if v.min_element() <= 0.0 {
                return Err(op_err(n, "\"tamanho\" precisa ser maior que 0"));
            }
            // A escala que deixa a peça (girada ou não) desse tamanho.
            let (a, b) = part_bounds(p).ok_or_else(|| op_err(n, "a peça está vazia"))?;
            let now = (b - a).max(Vec3::splat(1e-4));
            p.scale *= v / now;
            done.push("tamanho");
        }
    }
    let p = &mut md.parts[i];
    // As faces de um lado (para pintar, extrudar, recuar e afinar).
    let lado = get_str(op, "lado", n)?;
    if let Some(c) = color {
        match lado {
            Some(l) => {
                let dir = side(l).map_err(|e| op_err(n, e))?;
                for f in faces_towards(p, dir) {
                    p.faces[f].color = c.to_string();
                }
                done.push("pintei um lado");
            }
            None => {
                for f in &mut p.faces {
                    f.color = c.to_string();
                }
                done.push("pintei");
            }
        }
    }
    if let Some(l) = get_str(op, "extrudar", n)? {
        let dir = side(l).map_err(|e| op_err(n, e))?;
        let dist = get_num(op, "distancia", n)?.unwrap_or(0.5);
        let faces = faces_towards(p, dir);
        let (verts, normal) = p.extrude(&faces);
        // A distância em metros do modelo (a peça pode ter escala).
        let local = normal * dist / p.scale.max(Vec3::splat(1e-4));
        let k = op
            .get("afinar")
            .and_then(Value::as_f64)
            .unwrap_or(1.0)
            .max(0.0) as f32;
        let c =
            verts.iter().map(|&v| p.verts[v as usize]).sum::<Vec3>() / verts.len().max(1) as f32;
        for &v in &verts {
            let q = &mut p.verts[v as usize];
            *q = c + (*q - c) * k + local;
        }
        done.push("extrudei");
    }
    if let Some(l) = get_str(op, "recuar", n)? {
        let dir = side(l).map_err(|e| op_err(n, e))?;
        let t = get_num(op, "quanto", n)?.unwrap_or(0.2);
        let faces = faces_towards(p, dir);
        let inner = p.inset(&faces, t);
        if let Some(depth) = get_num(op, "fundo", n)? {
            for f in &inner {
                let nrm = face_normal(&p.verts, &p.faces[*f].v);
                let local = nrm * depth / p.scale.max(Vec3::splat(1e-4));
                let vs = p.faces[*f].v.clone();
                for v in vs {
                    p.verts[v as usize] += local;
                }
            }
        }
        if let Some(c) = get_str(op, "cor_dentro", n)? {
            for f in &inner {
                p.faces[*f].color = c.to_string();
            }
        }
        done.push("recuei");
    }
    if let Some(Value::String(l)) = op.get("afinar")
        && get_str(op, "extrudar", n)?.is_none()
    {
        let dir = side(l).map_err(|e| op_err(n, e))?;
        let k = get_num(op, "quanto", n)?.unwrap_or(0.5).max(0.0);
        let faces = faces_towards(p, dir);
        let mut vs: Vec<u32> = faces.iter().flat_map(|&f| p.faces[f].v.clone()).collect();
        vs.sort_unstable();
        vs.dedup();
        let c = vs.iter().map(|&v| p.verts[v as usize]).sum::<Vec3>() / vs.len().max(1) as f32;
        for v in vs {
            let q = &mut p.verts[v as usize];
            *q = c + (*q - c) * k;
        }
        done.push("afinei");
    }
    if let Some(on) = op.get("simetria").and_then(Value::as_bool) {
        if on && !p.mirror {
            p.mirror_on();
        } else if !on && p.mirror {
            p.mirror_apply();
        }
        done.push("simetria");
    }
    if let Some(new) = get_str(op, "renomear", n)? {
        if md
            .parts
            .iter()
            .enumerate()
            .any(|(j, q)| j != i && q.name == new)
        {
            return Err(op_err(n, format!("já existe a peça \"{new}\"")));
        }
        md.parts[i].name = new.to_string();
        done.push("nome novo");
    }
    if done.is_empty() {
        return Err(op_err(
            n,
            "com \"peca\", diga o que mudar: em, andar, giro, escala, tamanho, cor (com \"lado\"), extrudar, recuar, afinar, simetria ou renomear",
        ));
    }
    Ok(format!("\"{}\": {}", md.parts[i].name, done.join(", ")))
}

fn modelar(s: &mut Server, a: &Value) -> Result<Output, String> {
    let (rel, mut md) = s.load_model(arg_str(a, "caminho")?)?;
    let ops = a
        .get("operacoes")
        .and_then(Value::as_array)
        .filter(|l| !l.is_empty())
        .ok_or("falta \"operacoes\": uma lista, como [{\"criar\": \"tronco\", \"forma\": \"cilindro\", \"tamanho\": [0.4, 2, 0.4], \"cor\": \"#9C6B43\"}]")?;
    let mut done = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        let op = op
            .as_object()
            .ok_or_else(|| op_err(i + 1, "precisa ser um objeto"))?;
        done.push(model_op(&mut md, op, i + 1)?);
    }
    s.save_model(&rel, &md)?;
    let mut text = format!("{rel}: {}.\n{}", done.join("; "), describe(&rel, &md));
    if md.parts.is_empty() || !opt_bool(a, "previa")?.unwrap_or(true) {
        return Ok(Output::text(text));
    }
    text.push_str(VIEWS_NOTE);
    Ok(Output::text(text).and_image(&views(&md)?, "image/png"))
}

// ver_modelo -----------------------------------------------------------------

fn view_of(name: &str) -> Result<Option<View>, String> {
    Ok(Some(match crate::paths::fold(name).as_str() {
        "todas" | "" => return Ok(None),
        "frente" => View::Front,
        "lado" => View::Side,
        "cima" => View::Top,
        "diagonal" | "3/4" => View::ThreeQuarter,
        _ => return Err("\"vista\": todas, frente, lado, cima ou diagonal".into()),
    }))
}

fn ver_modelo(s: &mut Server, a: &Value) -> Result<Output, String> {
    let (rel, full) = crate::paths::resolve(&s.root, arg_str(a, "caminho")?)?;
    if !full.is_file() {
        return Err(format!("{rel} não existe"));
    }
    let view = view_of(opt_str(a, "vista")?.unwrap_or("todas"))?;
    let lower = rel.to_lowercase();
    let (mut text, data) = if lower.ends_with(".modelo") {
        let (_, md) = s.load_model(&rel)?;
        let glb = md.to_glb();
        let data =
            nn_render::d3::ModelData::from_gltf(&glb, &mut |_| None).map_err(|e| e.to_string())?;
        let mut t = describe(&rel, &md);
        if !s.root.join(glb_path(&rel)).is_file() {
            t.push_str(&format!(
                "Aviso: falta o {} (o jogo usa ele); grave com modelar.\n",
                glb_path(&rel)
            ));
        }
        (t, data)
    } else if lower.ends_with(".glb") || lower.ends_with(".gltf") {
        let bytes = std::fs::read(&full).map_err(|e| format!("{rel}: {e}"))?;
        let dir = full
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_default();
        let data = nn_render::d3::ModelData::from_gltf(&bytes, &mut |uri| {
            std::fs::read(dir.join(uri)).ok()
        })
        .map_err(|e| format!("{rel}: {e}"))?;
        (describe_glb(&rel, &data), data)
    } else {
        return Err(format!("{rel}: ver_modelo abre .modelo, .glb e .gltf"));
    };
    // Uma animação, num tempo (modelos com esqueleto).
    let mut anim = None;
    if let Some(name) = opt_str(a, "animacao")? {
        let sk = data
            .skeleton
            .as_ref()
            .ok_or_else(|| format!("{rel} não tem animações"))?;
        let i = sk.animation(name).ok_or_else(|| {
            let names: Vec<&str> = sk.animations.iter().map(|x| x.name.as_str()).collect();
            format!(
                "{rel} não tem a animação \"{name}\" (tem: {})",
                names.join(", ")
            )
        })?;
        let t = opt_num(a, "tempo")?.unwrap_or(0.0) as f32;
        anim = Some((i, t));
        text.push_str(&format!("Na animação \"{name}\", em {} s.\n", m(t)));
    }
    let tris = model_view::triangles(&data, anim);
    if tris.is_empty() {
        return Ok(Output::text(format!(
            "{text}(Sem triângulos para desenhar.)"
        )));
    }
    let img = match view {
        None => {
            text.push_str(VIEWS_NOTE);
            model_view::render(&tris, 300)
        }
        Some(v) => model_view::render_one(&tris, v, 600),
    };
    Ok(Output::text(text).and_image(&png(&img)?, "image/png"))
}

/// O que um `.glb` tem (de qualquer lugar: Blender, Mixamo, o modelador).
fn describe_glb(rel: &str, d: &nn_render::d3::ModelData) -> String {
    let mut out = String::new();
    let tris = model_view::triangles(d, None);
    match model_view::bounds(&tris) {
        Some((a, b)) => {
            let s = b - a;
            out.push_str(&format!(
                "{rel}: {} triângulos, {} parte(s) de material. Tamanho {} x {} x {} m (X, Y, Z); vai de y = {} a {}; o meio fica em x = {}, z = {} (a origem, o ponto que fica na posição do objeto, é o 0).\n",
                d.triangle_count(),
                d.parts.len(),
                m(s.x),
                m(s.y),
                m(s.z),
                m(a.y),
                m(b.y),
                m((a.x + b.x) / 2.0),
                m((a.z + b.z) / 2.0)
            ));
            let big = s.max_element();
            if big > 50.0 {
                out.push_str(&format!(
                    "  Aviso: {} m é enorme (o arquivo deve estar em centímetros). No objeto, use \"scale\" {} (1 m = 1 unidade).\n",
                    m(big),
                    m(1.0 / 100.0)
                ));
            } else if big < 0.05 {
                out.push_str("  Aviso: o modelo é minúsculo (menos de 5 cm); aumente com \"scale\" no objeto.\n");
            }
            if a.y < -0.05 {
                out.push_str("  Aviso: parte fica abaixo de y = 0: com o objeto no chão, ele afunda (suba o objeto, ou use um filho).\n");
            } else if a.y > 0.05 {
                out.push_str(
                    "  Aviso: a base fica acima de y = 0: com o objeto no chão, ele flutua.\n",
                );
            }
        }
        None => out.push_str(&format!("{rel}: sem triângulos.\n")),
    }
    if !d.images.is_empty() {
        out.push_str(&format!("Imagens (texturas): {}.\n", d.images.len()));
    }
    let glow: Vec<String> = d
        .parts
        .iter()
        .filter(|p| p.emissive.r + p.emissive.g + p.emissive.b > 0.01)
        .map(|p| {
            format!(
                "#{:02x}{:02x}{:02x}",
                (p.emissive.r * 255.0) as u8,
                (p.emissive.g * 255.0) as u8,
                (p.emissive.b * 255.0) as u8
            )
        })
        .collect();
    if !glow.is_empty() {
        out.push_str(&format!(
            "Brilho próprio (emissive): {}.\n",
            glow.join(", ")
        ));
    }
    if let Some(sk) = &d.skeleton {
        let joints: Vec<&str> = sk
            .joints
            .iter()
            .filter_map(|(node, _)| sk.nodes.get(*node).map(|n| n.name.as_str()))
            .collect();
        out.push_str(&format!(
            "Esqueleto: {} osso(s){}.\n",
            joints.len(),
            if joints.is_empty() {
                String::new()
            } else {
                let shown: Vec<&str> = joints.iter().take(30).copied().collect();
                format!(
                    " ({}{})",
                    shown.join(", "),
                    if joints.len() > 30 { ", ..." } else { "" }
                )
            }
        ));
        if sk.animations.is_empty() {
            out.push_str("Animações: nenhuma.\n");
        } else {
            let list: Vec<String> = sk
                .animations
                .iter()
                .map(|a| format!("{} ({} s)", a.name, m(a.duration)))
                .collect();
            out.push_str(&format!(
                "Animações: {}. No objeto, o componente anim3d toca uma pelo nome; ver_modelo com \"animacao\" e \"tempo\" mostra um quadro.\n",
                list.join(", ")
            ));
        }
    }
    out
}

const MODELO: &str = "O modelo (.modelo), ex.: \"modelos/torre.modelo\"";

pub static TOOLS: &[&Tool] = &[
    &Tool {
        name: "criar_modelo",
        title: "Criar modelo 3D",
        description: "Cria um modelo 3D do modelador do NN Studio (.modelo, low-poly: peças de faces coloridas), vazio, a partir de um pronto (arvore, pinheiro, pedra, cerca, casinha, bau, lampiao, fogueira, personagem, carrinho, muro, borda, escada, ponte, placa) ou de outro .modelo. Grava junto o .glb, que o jogo usa. Depois: modelar e ver_modelo. Medidas em metros: Y para cima, a frente é +Z, e a base do modelo fica em y = 0 (a origem é o ponto que fica na posição do objeto).",
        params: || {
            vec![
                ("caminho", s_str(MODELO), true),
                (
                    "a_partir",
                    s_str("\"vazio\" (padrão), um modelo pronto ou outro .modelo"),
                    false,
                ),
                ("substituir", s_bool("Se já existe, começar de novo"), false),
            ]
        },
        read_only: false,
        destructive: false,
        run: criar_modelo,
    },
    &Tool {
        name: "modelar",
        title: "Modelar",
        description: "Muda um modelo 3D (.modelo) com várias operações numa chamada e devolve o modelo em 4 vistas. Medidas em metros, Y para cima, a frente é +Z, cores \"#rrggbb\". Operações: {\"criar\": \"tronco\", \"forma\": \"cubo|cilindro|cone|piramide|esfera|plano|rampa\", \"tamanho\": [x, y, z], \"em\": [x, y, z] (o meio da base da peça), \"giro\": [x, y, z] (graus), \"cor\": \"#9C6B43\", \"lados\": 8, \"simetria\": true}; {\"kit\": \"Casa/Parede\", \"em\": [...], \"nome\": ...} (peças prontas: Casa, Natureza, Personagem, Veículo); {\"peca\": \"tronco\", e: \"em\", \"andar\" (soma), \"giro\", \"escala\", \"tamanho\" (a escala para ficar desse tamanho), \"cor\" (com \"lado\": cima, baixo, frente, tras, direita ou esquerda, só as faces daquele lado), \"extrudar\": lado (+ \"distancia\" em m e \"afinar\": 0.5 deixa a ponta menor), \"recuar\": lado (+ \"quanto\" de 0 a 1, \"fundo\" em m, negativo afunda: janelas e portas, e \"cor_dentro\"), \"afinar\": lado (+ \"quanto\": encolhe aquelas faces), \"simetria\": true/false (o lado -X copia o +X), \"renomear\"}; {\"copiar\": \"braco\", \"nome\": \"braco_d\", \"espelhar\": \"x\", \"em\"/\"andar\"}; {\"apagar\": \"peca\"}; {\"juntar\": [\"a\", \"b\"], \"nome\": \"c\"}.",
        params: || {
            vec![
                ("caminho", s_str(MODELO), true),
                (
                    "operacoes",
                    s_list(
                        "As operações, na ordem (veja a descrição)",
                        s_obj("Uma operação"),
                    ),
                    true,
                ),
                ("previa", s_bool("Devolver as vistas (padrão true)"), false),
            ]
        },
        read_only: false,
        destructive: false,
        run: modelar,
    },
    &Tool {
        name: "ver_modelo",
        title: "Ver modelo 3D",
        description: "Mostra um modelo 3D (.modelo do modelador ou qualquer .glb/.gltf, do Blender, do Mixamo...): o tamanho em metros, onde ficam a base e a origem (e se ele afunda ou flutua), as peças, os triângulos, as cores, o brilho, os ossos e as animações, e o modelo desenhado em 4 vistas (frente, lado, cima e diagonal) ou numa só, maior (\"vista\"). Num modelo com animações, \"animacao\" e \"tempo\" mostram um quadro dela.",
        params: || {
            vec![
                ("caminho", s_str("O modelo (.modelo, .glb ou .gltf)"), true),
                (
                    "vista",
                    s_str("todas (padrão), frente, lado, cima ou diagonal"),
                    false,
                ),
                (
                    "animacao",
                    s_str("Uma animação do modelo, para ver um quadro"),
                    false,
                ),
                ("tempo", s_num("O tempo na animação (segundos)"), false),
            ]
        },
        read_only: true,
        destructive: false,
        run: ver_modelo,
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn op(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn operacoes_nas_pecas() {
        let mut md = ModelData::default();
        model_op(&mut md, &op(json!({ "criar": "tronco", "forma": "cilindro", "tamanho": [0.4, 2, 0.4], "cor": "#9C6B43" })), 1).unwrap();
        let (a, b) = part_bounds(&md.parts[0]).unwrap();
        assert!(
            ((b - a) - Vec3::new(0.4, 2.0, 0.4)).length() < 0.05,
            "{a} {b}"
        );
        model_op(
            &mut md,
            &op(json!({ "peca": "tronco", "extrudar": "cima", "distancia": 1, "afinar": 0 })),
            2,
        )
        .unwrap();
        let (_, b) = part_bounds(&md.parts[0]).unwrap();
        assert!((b.y - 3.0).abs() < 1e-3, "ponta em 3 m: {b}");
        model_op(
            &mut md,
            &op(json!({ "peca": "tronco", "cor": "#FF0000", "lado": "baixo" })),
            3,
        )
        .unwrap();
        assert_eq!(
            md.parts[0]
                .faces
                .iter()
                .filter(|f| f.color == "#FF0000")
                .count(),
            1
        );
        model_op(
            &mut md,
            &op(json!({ "criar": "braco", "tamanho": [0.2, 0.6, 0.2], "em": [0.5, 1, 0] })),
            4,
        )
        .unwrap();
        model_op(
            &mut md,
            &op(json!({ "copiar": "braco", "nome": "braco_d", "espelhar": "x" })),
            5,
        )
        .unwrap();
        assert_eq!(md.parts[2].pos.x, -0.5);
        let e = model_op(&mut md, &op(json!({ "peca": "perna", "em": [0, 0, 0] })), 6).unwrap_err();
        assert!(e.contains("tem: tronco, braco, braco_d"), "{e}");
        model_op(
            &mut md,
            &op(json!({ "juntar": ["braco", "braco_d"], "nome": "bracos" })),
            7,
        )
        .unwrap();
        assert_eq!(md.parts.len(), 2);
        let (a, b) = part_bounds(&md.parts[1]).unwrap();
        assert!(
            (a.x + 0.6).abs() < 1e-3 && (b.x - 0.6).abs() < 1e-3,
            "{a} {b}"
        );
        model_op(
            &mut md,
            &op(json!({ "kit": "Casa/Parede", "em": [0, 0, 2] })),
            8,
        )
        .unwrap();
        assert!(md.parts.iter().any(|p| p.name == "Parede"));
        let e = model_op(&mut md, &op(json!({ "kit": "Torre" })), 9).unwrap_err();
        assert!(e.contains("Casa: Parede"), "{e}");
        model_op(&mut md, &op(json!({ "peca": "Parede", "recuar": "frente", "quanto": 0.4, "fundo": -0.1, "cor_dentro": "#222222" })), 10).unwrap();
        model_op(
            &mut md,
            &op(json!({ "peca": "tronco", "tamanho": [1, 1, 1] })),
            11,
        )
        .unwrap();
        let (a, b) = part_bounds(&md.parts[0]).unwrap();
        assert!(((b - a) - Vec3::ONE).length() < 1e-3, "{a} {b}");
    }
}
