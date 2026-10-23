//! Partículas: um efeito pronto (fogo, fumaça, faíscas...) num objeto, os
//! ajustes e uma prévia do efeito sozinho, em quadros ao longo do tempo.

use std::sync::atomic::{AtomicU32, Ordering};

use image::RgbaImage;
use serde_json::{Map, Value, json};

use nn_projeto::scene_data::{self, Component, Object, Prop, SceneData};
use nn_projeto::{config_lua, particles};
use nn_script::game_run::{Action, ActionKind, GameRun, Plan, RunOptions};

use super::cenas::find_object;
use super::valores;
use super::{Output, Tool, arg_str, opt_bool, opt_num, opt_str, s_bool, s_num, s_obj, s_str};
use crate::Server;

/// Os campos que crescem com a escala (tamanhos, velocidades, áreas).
const SCALED: &[&str] = &[
    "size_start",
    "size_end",
    "speed_min",
    "speed_max",
    "gravity",
    "area_w",
    "area_h",
    "area_radius",
];

fn effects(three_d: bool) -> String {
    particles::presets(three_d)
        .iter()
        .map(|p| format!("{} ({})", p.id, p.tip))
        .collect::<Vec<_>>()
        .join("; ")
}

fn summary(c: &Component) -> String {
    let n = |k: &str| {
        let v = c.num(k);
        if v.fract() == 0.0 {
            format!("{v:.0}")
        } else {
            format!("{v:.2}")
        }
    };
    let mut out = if c.str("mode") == "explosao" {
        format!("de uma vez: {} partículas", n("amount"))
    } else {
        format!("contínuo: {} por segundo", n("rate"))
    };
    out.push_str(&format!(
        ", vida {} s, velocidade {} a {}, direção {}° (abertura {}°), gravidade {}, tamanho {} → {}, cores {} → {}{}{}",
        n("lifetime"),
        n("speed_min"),
        n("speed_max"),
        n("direction"),
        n("spread"),
        n("gravity"),
        n("size_start"),
        n("size_end"),
        c.str("color_start"),
        c.str("color_end"),
        if c.bool("pixel") { ", pixel art (quadradinhos)" } else { "" },
        if c.str("image").is_empty() { String::new() } else { format!(", imagem {}", c.str("image")) },
    ));
    out
}

/// O retângulo da tela em que algum quadro difere do fundo (o efeito),
/// com uma folga; `None` se nada apareceu.
fn effect_area(frames: &[RgbaImage]) -> Option<(u32, u32, u32, u32)> {
    let first = frames.first()?;
    let bg = *first.get_pixel(0, 0);
    let (w, h) = first.dimensions();
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for f in frames {
        for (x, y, p) in f.enumerate_pixels() {
            let d: u32 = (0..3).map(|i| u32::from(p[i].abs_diff(bg[i]))).sum();
            if d > 30 {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
    }
    if x1 < x0 {
        return None;
    }
    // Folga de 15% (no mínimo um décimo da tela) e pelo menos 1/6 da tela.
    let min_w = (w / 6).max(16);
    let min_h = (h / 6).max(16);
    let pad_x = ((x1 - x0) * 15 / 100).max(w / 40);
    let pad_y = ((y1 - y0) * 15 / 100).max(h / 40);
    let (mut x0, mut x1) = (x0.saturating_sub(pad_x), (x1 + pad_x).min(w - 1));
    let (mut y0, mut y1) = (y0.saturating_sub(pad_y), (y1 + pad_y).min(h - 1));
    while x1 - x0 + 1 < min_w && (x0 > 0 || x1 < w - 1) {
        x0 = x0.saturating_sub(1);
        x1 = (x1 + 1).min(w - 1);
    }
    while y1 - y0 + 1 < min_h && (y0 > 0 || y1 < h - 1) {
        y0 = y0.saturating_sub(1);
        y1 = (y1 + 1).min(h - 1);
    }
    Some((x0, y0, x1 - x0 + 1, y1 - y0 + 1))
}

/// Junta as capturas lado a lado, recortadas em volta do efeito e
/// ampliadas (com pixels inteiros, se for para crescer). Devolve a imagem e
/// a parte da tela mostrada.
fn strip(pngs: &[Vec<u8>]) -> Result<(Vec<u8>, String), String> {
    let frames: Vec<RgbaImage> = pngs
        .iter()
        .filter_map(|p| image::load_from_memory(p).ok().map(|i| i.to_rgba8()))
        .collect();
    let Some(first) = frames.first() else {
        return Err("a prévia não capturou nada".into());
    };
    let (w, h) = first.dimensions();
    let (cx, cy, cw, ch) = effect_area(&frames).unwrap_or((0, 0, w, h));
    let shown = if (cw, ch) == (w, h) {
        format!("a tela toda ({w}x{h})")
    } else {
        format!("um recorte de {cw}x{ch} da tela de {w}x{h}, em volta do efeito")
    };
    // Cada quadro com uns 240 de altura (e no máximo 480 de largura).
    const H: u32 = 240;
    const W: u32 = 480;
    let (fw, fh, filter) = if ch < H && cw < W {
        let k = (H / ch).min(W / cw).clamp(1, 8);
        (cw * k, ch * k, image::imageops::FilterType::Nearest)
    } else {
        let k = (f64::from(H) / f64::from(ch)).min(f64::from(W) / f64::from(cw));
        (
            ((f64::from(cw) * k) as u32).max(1),
            ((f64::from(ch) * k) as u32).max(1),
            image::imageops::FilterType::Triangle,
        )
    };
    let gap = 4;
    let n = frames.len() as u32;
    let mut out = RgbaImage::from_pixel(
        fw * n + gap * (n - 1),
        fh,
        image::Rgba([0xff, 0x2e, 0x88, 255]),
    );
    for (i, f) in frames.iter().enumerate() {
        let part = image::imageops::crop_imm(f, cx, cy, cw, ch).to_image();
        let big = image::imageops::resize(&part, fw, fh, filter);
        image::imageops::replace(&mut out, &big, i64::from(i as u32 * (fw + gap)), 0);
    }
    let mut bytes = Vec::new();
    out.write_to(
        &mut std::io::Cursor::new(&mut bytes),
        image::ImageFormat::Png,
    )
    .map_err(|e| e.to_string())?;
    Ok((bytes, shown))
}

/// O efeito sozinho numa cena de teste (numa pasta temporária, com o
/// config.lua e a imagem do projeto): capturas ao longo da vida dele.
fn preview(s: &Server, obj: &Object, c: &Component) -> Result<(String, Vec<u8>), String> {
    static N: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "nn-mcp-fx-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let result = (|| {
        // O config.lua do projeto (a tela, o fundo), começando na cena de teste.
        let config = std::fs::read_to_string(s.root.join("config.lua"))
            .map_err(|e| format!("config.lua: {e}"))?;
        let old = config_lua::read(&config, "")?;
        let mut new = old.clone();
        new.start = "fx.cena".into();
        let config = config_lua::update(&config, &config_lua::changes(&old, &new), &new);
        std::fs::write(dir.join("config.lua"), config).map_err(|e| e.to_string())?;
        let image = c.str("image");
        if !image.is_empty() {
            let to = dir.join(image);
            if let Some(p) = to.parent() {
                std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let _ = std::fs::copy(s.root.join(image), to);
        }
        let (_, h) = s
            .config()
            .map(|(_, cfg)| cfg.game_size())
            .unwrap_or((1280, 720));
        let h = f64::from(h);
        // O efeito no meio; se ele cai (chuva, neve), no alto da tela; se
        // sobe (fogo, fumaça), mais para baixo.
        let life = c.num("lifetime").max(0.05);
        let speed = (c.num("speed_min") + c.num("speed_max")) / 2.0;
        let rise =
            c.num("direction").to_radians().sin() * speed * life * (1.0 - c.num("spread") / 360.0)
                + 0.5 * c.num("gravity") * life * life;
        let mut fx = Object::new(1, &obj.name);
        let mut comp = c.clone();
        comp.set("on_start", Prop::Bool(true));
        comp.set("emitting", Prop::Bool(true));
        comp.set("when_done", Prop::Str("nada".into()));
        fx.components.push(comp);
        let mut cam = Object::new(2, "camera");
        cam.y = if rise < -h * 0.2 {
            -h * 0.4
        } else if rise > h * 0.2 {
            h * 0.25
        } else {
            0.0
        };
        cam.components.push(Component::new("camera"));
        let data = SceneData {
            objects: vec![fx, cam],
            ..Default::default()
        };
        std::fs::write(dir.join("fx.cena"), scene_data::write(&data)).map_err(|e| e.to_string())?;

        let burst = c.str("mode") == "explosao";
        let marks: &[f64] = if burst {
            &[0.04, 0.25, 0.55, 0.9]
        } else {
            &[0.25, 0.6, 1.0, 1.8]
        };
        let times: Vec<f64> = marks.iter().map(|k| (k * life).clamp(0.05, 12.0)).collect();
        let actions = times
            .iter()
            .map(|t| Action {
                at: *t,
                kind: ActionKind::Capture(format!("{t:.2} s")),
            })
            .collect();
        let mut run = GameRun::open(
            &dir,
            RunOptions {
                scene: Some("fx.cena".into()),
                ..RunOptions::default()
            },
        )?;
        let plan = Plan {
            duration: times.last().copied().unwrap_or(1.0) + 0.05,
            actions,
        };
        run.run_plan(&plan)?;
        let report = run.take_report();
        if let Some(e) = report.errors.first() {
            return Err(format!("o efeito deu erro no jogo: {}", e.error.message));
        }
        let pngs: Vec<Vec<u8>> = report.captures.into_iter().map(|c| c.png).collect();
        let (png, shown) = strip(&pngs)?;
        let label = times
            .iter()
            .map(|t| format!("{t:.2} s"))
            .collect::<Vec<_>>()
            .join(", ");
        Ok((format!("{label} ({shown})"), png))
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn particulas(s: &mut Server, a: &Value) -> Result<Output, String> {
    let mut doc = s.open_doc(arg_str(a, "cena")?)?;
    let three_d = doc.three_d();
    let name = arg_str(a, "objeto")?;
    let effect =
        match opt_str(a, "efeito")? {
            Some(id) => Some(particles::find_in(id, three_d).ok_or_else(|| {
                format!("não há o efeito \"{id}\". Efeitos: {}", effects(three_d))
            })?),
            None => None,
        };
    let mut m = Map::new();
    m.insert("type".into(), json!("particles"));
    let target = s.target(&m)?;
    let mut notes = Vec::new();
    let id = match find_object(&doc.data, &json!(name)) {
        Ok(id) => id,
        Err(_) if !name.trim().is_empty() && name.parse::<u32>().is_err() => {
            // Objeto novo, onde for dito.
            if effect.is_none() && a.get("mudar").is_none() {
                return Err(format!(
                    "não há objeto \"{name}\": para criar, diga o \"efeito\". Efeitos: {}",
                    effects(three_d)
                ));
            }
            let id = doc.data.next_id();
            let mut o = Object::new(id, name);
            o.x = opt_num(a, "x")?.unwrap_or(0.0);
            o.y = opt_num(a, "y")?.unwrap_or(0.0);
            if three_d {
                o.z = opt_num(a, "z")?.unwrap_or(0.0);
            }
            let len = doc.data.objects.len();
            doc.data.insert(None, len, o);
            notes.push(format!("Criei o objeto \"{name}\" (id {id})."));
            id
        }
        Err(e) => return Err(e),
    };
    let o = doc.data.find_mut(id).expect("objeto");
    if o.prefab.is_some() && valores::find_component(o, &target).is_none() {
        return Err(format!(
            "\"{}\" é uma cópia de prefab sem partículas; ponha no próprio prefab",
            o.name
        ));
    }
    let i = match valores::find_component(o, &target) {
        Some(i) => i,
        None => {
            o.components.push(if three_d {
                particles::component_3d()
            } else {
                Component::new("particles")
            });
            o.components.len() - 1
        }
    };
    let comp = &mut o.components[i];
    if let Some(p) = effect {
        for (k, v) in p.component().props {
            comp.set(&k, v);
        }
    }
    if let Some(on) = opt_bool(a, "pixel")? {
        if three_d {
            return Err("\"pixel\" é só para 2D".into());
        }
        comp.set("pixel", Prop::Bool(on));
    }
    // A escala: a pedida ou, em pixel art, a da tela (os efeitos prontos são
    // para 720 de altura).
    let scale = match opt_num(a, "escala")? {
        Some(k) if k > 0.0 && k <= 20.0 => Some(k),
        Some(_) => return Err("\"escala\" vai de 0 a 20 (0.5 = metade)".into()),
        None if effect.is_some() && comp.bool("pixel") => {
            let (_, h) = s
                .config()
                .map(|(_, cfg)| cfg.game_size())
                .unwrap_or((1280, 720));
            (h < 540).then(|| (f64::from(h) / 720.0).max(0.15))
        }
        None => None,
    };
    if let Some(k) = scale {
        for key in SCALED {
            let v = comp.num(key) * k;
            let v = if comp.bool("pixel") && key.starts_with("size") {
                v.round()
            } else {
                (v * 100.0).round() / 100.0
            };
            comp.set(key, Prop::Num(v));
        }
        notes.push(format!("Tamanhos, velocidades e áreas × {k:.2}."));
    }
    if let Some(m) = a.get("mudar").filter(|v| !v.is_null()) {
        let m = m
            .as_object()
            .ok_or("\"mudar\" é { campo: valor } (veja listar_componentes particles)")?;
        valores::apply_fields(comp, &target, m, three_d)?;
        // No contínuo, quem liga e desliga é "emitting" ("on_start" é da
        // explosão): começar desligado é emitting = false.
        if comp.str("mode") == "continuo"
            && m.get("on_start").and_then(Value::as_bool) == Some(false)
            && !m.contains_key("emitting")
        {
            comp.set("emitting", Prop::Bool(false));
            notes.push("Contínuo começando desligado: \"emitting\": false (\"on_start\" é da explosão). Liga com a ação \"Ligar ou desligar partículas\" (id emissor) dos eventos (ou, em Lua, objeto.particles.emitting = true).".into());
        }
    }
    let comp = comp.clone();
    let obj = o.clone();
    s.save_doc(&doc)?;
    let mut text = format!("\"{}\" em {}: {}.", obj.name, doc.rel, summary(&comp));
    for n in notes {
        text.push_str(&format!("\n- {n}"));
    }
    if comp.str("mode") == "explosao" && !comp.bool("on_start") {
        text.push_str("\n- Solta só com a ação \"Soltar partículas\" dos eventos (ou, em Lua, nn.burst_particles).");
    }
    if three_d || !opt_bool(a, "previa")?.unwrap_or(true) {
        if three_d {
            text.push_str("\nPara ver no 3D: rodar_jogo.");
        }
        return Ok(Output::text(text));
    }
    match preview(s, &obj, &comp) {
        Ok((times, png)) => {
            text.push_str(&format!(
                "\nO efeito sozinho, em {times} (da esquerda para a direita, no fundo do jogo):"
            ));
            Ok(Output::text(text).and_image(&png, "image/png"))
        }
        Err(e) => {
            text.push_str(&format!("\n(Sem prévia: {e}.)"));
            Ok(Output::text(text))
        }
    }
}

pub static TOOLS: &[&Tool] = &[&Tool {
    name: "particulas",
    title: "Partículas",
    description: "Põe um efeito de partículas pronto num objeto (cria o objeto se ele não existir), ajusta e mostra o efeito sozinho em 4 momentos. Efeitos 2D: fogo, fumaca, faiscas, explosao, poeira, brilho, bolhas, chuva, neve, confete (3D: fogo, fumaca, faiscas, explosao, poeira, brilho, chuva, neve). \"pixel\": quadradinhos de tamanho inteiro (pixel art; numa tela pequena o efeito já vem na escala dela). \"escala\" multiplica tamanhos, velocidades e áreas. \"mudar\" troca campos do componente (veja listar_componentes particles), ex.: {\"color_start\": \"#7ec4ff\", \"mode\": \"explosao\", \"on_start\": false} (uma explosão que só sai quando pedir); um contínuo começa desligado com \"emitting\": false.",
    params: || {
        vec![
            ("cena", s_str("A cena (.cena) ou o prefab"), true),
            (
                "objeto",
                s_str("O objeto (nome ou id); um nome novo cria o objeto"),
                true,
            ),
            (
                "efeito",
                s_str("O efeito pronto (fogo, fumaca, faiscas, explosao...)"),
                false,
            ),
            (
                "pixel",
                s_bool("Pixel art: quadradinhos de tamanho inteiro (2D)"),
                false,
            ),
            (
                "escala",
                s_num("Multiplica tamanhos, velocidades e áreas (0.25 = um quarto)"),
                false,
            ),
            (
                "mudar",
                s_obj("Campos do componente: {\"rate\": 30, \"color_start\": \"#ff0000\"}"),
                false,
            ),
            ("x", s_num("Objeto novo: x"), false),
            ("y", s_num("Objeto novo: y"), false),
            ("z", s_num("Objeto novo, no 3D: z"), false),
            (
                "previa",
                s_bool("Mostrar o efeito sozinho (padrão true; só 2D)"),
                false,
            ),
        ]
    },
    read_only: false,
    destructive: false,
    run: particulas,
}];
