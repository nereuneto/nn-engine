//! Rodar o jogo sem janela e os testes de jogo.

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use nn_script::capture::{LineKind, LogLine};
use nn_script::game_run::{
    Action, ActionKind, Capture, GameRun, Outcome, Plan, Report, RunOptions, check_key,
};
use nn_script::testing::{self, TestOptions};

use super::{Output, Tool, opt_bool, opt_num, opt_str, s_bool, s_int, s_list, s_num, s_obj, s_str};
use crate::Server;

/// Quanto tempo de jogo uma chamada pode rodar.
const MAX_SECONDS: f64 = 120.0;
/// Imagens por resposta.
const MAX_IMAGES: usize = 5;
/// Linhas de cada tipo (prints, avisos...) por resposta.
const MAX_LINES: usize = 40;

fn need_project(s: &Server) -> Result<(), String> {
    if s.has_project() {
        Ok(())
    } else {
        Err("a pasta ainda não tem um jogo; crie com criar_projeto".into())
    }
}

fn open_run(s: &Server, scene: Option<&str>) -> Result<GameRun, String> {
    let opts = RunOptions {
        scene: scene.map(|c| c.trim_start_matches("./").replace('\\', "/")),
        ..RunOptions::default()
    };
    GameRun::open(&s.root, opts)
}

// rodar_jogo ------------------------------------------------------------------

/// Lê as ações do JSON.
fn parse_actions(list: &Value, duration: f64) -> Result<Vec<Action>, String> {
    let Some(list) = list.as_array() else {
        return Err("\"acoes\" precisa ser uma lista".into());
    };
    let mut out = Vec::new();
    for (i, a) in list.iter().enumerate() {
        let n = i + 1;
        let Some(obj) = a.as_object() else {
            return Err(format!(
                "ação {n}: precisa ser um objeto, como {{\"em\": 1, \"apertar\": \"space\"}}"
            ));
        };
        let at = match obj.get("em") {
            None | Some(Value::Null) => 0.0,
            Some(v) => v
                .as_f64()
                .filter(|t| *t >= 0.0)
                .ok_or_else(|| format!("ação {n}: \"em\" precisa ser os segundos (0 ou mais)"))?,
        };
        let key = |k: &str| -> Result<String, String> {
            let v = obj[k]
                .as_str()
                .ok_or_else(|| format!("ação {n}: \"{k}\" precisa ser o nome da tecla"))?;
            check_key(v).map_err(|e| format!("ação {n}: {e}"))?;
            Ok(v.to_string())
        };
        let button = match obj.get("botao") {
            None | Some(Value::Null) => "left".to_string(),
            Some(Value::String(b)) if ["left", "right", "middle"].contains(&b.as_str()) => {
                b.clone()
            }
            Some(_) => {
                return Err(format!(
                    "ação {n}: \"botao\" é \"left\", \"right\" ou \"middle\""
                ));
            }
        };
        let point = |v: &Value| -> Option<(f32, f32)> {
            let arr = v.as_array()?;
            match arr.as_slice() {
                [x, y] => Some((x.as_f64()? as f32, y.as_f64()? as f32)),
                _ => None,
            }
        };
        let kinds = [
            "apertar",
            "segurar",
            "soltar",
            "clicar",
            "mouse",
            "rolar",
            "girar_mouse",
            "digitar",
            "capturar",
            "lua",
        ];
        let found: Vec<&str> = kinds
            .iter()
            .copied()
            .filter(|k| obj.contains_key(*k))
            .collect();
        let kind = match found.as_slice() {
            ["apertar"] => ActionKind::Press(key("apertar")?),
            ["segurar"] => {
                let k = key("segurar")?;
                match obj.get("por") {
                    None | Some(Value::Null) => ActionKind::Hold(k, (duration - at).max(0.0)),
                    Some(v) => ActionKind::Hold(
                        k,
                        v.as_f64()
                            .filter(|s| *s >= 0.0)
                            .ok_or_else(|| format!("ação {n}: \"por\" precisa ser os segundos"))?,
                    ),
                }
            }
            ["soltar"] => ActionKind::Up(key("soltar")?),
            ["clicar"] => {
                match &obj["clicar"] {
                    Value::String(name) => ActionKind::ClickObject {
                        name: name.clone(),
                        button,
                    },
                    v => {
                        let (x, y) = point(v).ok_or_else(|| {
                        format!("ação {n}: \"clicar\" é [x, y] (pixels da tela) ou o nome de um objeto")
                    })?;
                        ActionKind::Click { x, y, button }
                    }
                }
            }
            ["mouse"] => {
                let (x, y) = point(&obj["mouse"])
                    .ok_or_else(|| format!("ação {n}: \"mouse\" é [x, y] (pixels da tela)"))?;
                ActionKind::Move { x, y }
            }
            ["rolar"] => ActionKind::Scroll(obj["rolar"].as_f64().ok_or_else(|| {
                format!(
                    "ação {n}: \"rolar\" são as linhas da roda do mouse (negativo desce a lista)"
                )
            })? as f32),
            ["girar_mouse"] => {
                let (dx, dy) = point(&obj["girar_mouse"]).ok_or_else(|| {
                    format!("ação {n}: \"girar_mouse\" é [dx, dy] (pixels que o mouse anda)")
                })?;
                let seconds = match obj.get("por") {
                    None | Some(Value::Null) => 0.0,
                    Some(v) => v
                        .as_f64()
                        .filter(|s| *s >= 0.0)
                        .ok_or_else(|| format!("ação {n}: \"por\" precisa ser os segundos"))?
                        as f32,
                };
                ActionKind::MouseMotion { dx, dy, seconds }
            }
            ["digitar"] => ActionKind::Type(
                obj["digitar"]
                    .as_str()
                    .ok_or_else(|| format!("ação {n}: \"digitar\" precisa ser um texto"))?
                    .to_string(),
            ),
            ["capturar"] => ActionKind::Capture(match &obj["capturar"] {
                Value::String(s) => s.clone(),
                Value::Bool(true) => String::new(),
                _ => return Err(format!("ação {n}: \"capturar\" é true ou um nome")),
            }),
            ["lua"] => ActionKind::Lua(
                obj["lua"]
                    .as_str()
                    .ok_or_else(|| format!("ação {n}: \"lua\" precisa ser o código"))?
                    .to_string(),
            ),
            [] => {
                return Err(format!("ação {n}: diga o que fazer: {}", kinds.join(", ")));
            }
            many => {
                return Err(format!(
                    "ação {n}: uma coisa por ação (veio {})",
                    many.join(" e ")
                ));
            }
        };
        for k in obj.keys() {
            if !kinds.contains(&k.as_str()) && !["em", "por", "botao"].contains(&k.as_str()) {
                return Err(format!(
                    "ação {n}: \"{k}\" não existe (use em, {}, por, botao)",
                    kinds.join(", ")
                ));
            }
        }
        out.push(Action { at, kind });
    }
    Ok(out)
}

fn string_list(a: &Value, key: &str) -> Result<Vec<String>, String> {
    match a.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(list)) => list
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("\"{key}\" é uma lista de textos"))
            })
            .collect(),
        Some(_) => Err(format!("\"{key}\" é uma lista de textos")),
    }
}

fn rodar_jogo(s: &mut Server, a: &Value) -> Result<Output, String> {
    need_project(s)?;
    let duration = opt_num(a, "segundos")?.unwrap_or(3.0);
    if !(0.0..=MAX_SECONDS).contains(&duration) {
        return Err(format!("\"segundos\" vai de 0 a {MAX_SECONDS}"));
    }
    let actions = match a.get("acoes") {
        None | Some(Value::Null) => Vec::new(),
        Some(v) => parse_actions(v, duration)?,
    };
    let look = string_list(a, "olhar")?;
    let objects = string_list(a, "objetos")?;
    let final_capture = opt_bool(a, "captura_final")?.unwrap_or(true);
    let show_bodies = opt_bool(a, "ver_corpos")?.unwrap_or(false);
    let zoom = opt_num(a, "ampliar")?.map(|n| n.clamp(1.0, 8.0) as u32);

    let mut run = open_run(s, opt_str(a, "cena")?)?;
    if show_bodies {
        run.runtime()
            .set_show_bodies(true)
            .map_err(|e| e.to_string())?;
    }
    let start_scene = run.front().to_string();
    let before = run.snapshot();
    let plan = Plan { duration, actions };
    let outcome = run.run_plan(&plan)?;
    let end_scene = run.front().to_string();
    let after = run.snapshot();
    let mut looked = Vec::new();
    for expr in &look {
        looked.push((expr.clone(), run.inspect(expr)));
    }
    let last = if final_capture && !run.ended() {
        run.screenshot(&format!("fim ({:.2} s)", run.time())).ok()
    } else {
        None
    };
    let report = run.take_report();

    let mut out = String::new();
    out.push_str(&format!(
        "Rodou {:.2} s de jogo ({} quadros), começando em {start_scene}.\n",
        run.time(),
        run.frames()
    ));
    match &outcome {
        Outcome::Done => {}
        Outcome::GameError(_) => {
            out.push_str("O JOGO PAROU COM ERRO (veja Erros); o resto do roteiro não rodou.\n");
        }
        Outcome::Failed(msg) => out.push_str(&format!("O roteiro parou: {msg}\n")),
        Outcome::TimedOut(msg) => out.push_str(&format!("O roteiro parou: {msg}\n")),
    }
    if let Some(t) = report.quit {
        out.push_str(&format!("O jogo fechou (nn.quit) em {t:.2} s.\n"));
    }
    write_report(&mut out, &report);
    if report.scenes.len() > 1 {
        out.push_str("\nTrocas de cena:\n");
        for c in report.scenes.iter().skip(1) {
            out.push_str(&format!("  {:.2} s  {}\n", c.time, c.scene));
        }
    }
    if !report.restarts.is_empty() {
        let list: Vec<String> = report
            .restarts
            .iter()
            .map(|t| format!("{t:.2} s"))
            .collect();
        out.push_str(&format!(
            "\nA cena reiniciou (começou de novo) em: {}. Por isso, abaixo vai o estado no fim, não a diferença desde o começo.\n",
            list.join(", ")
        ));
    }
    write_state(
        &mut out,
        &start_scene,
        if report.restarts.is_empty() {
            before.as_ref()
        } else {
            None
        },
        &end_scene,
        after.as_ref(),
    );
    if !objects.is_empty() {
        write_objects(&mut out, after.as_ref(), &objects);
    }
    if !looked.is_empty() {
        out.push_str("\nExpressões (no fim):\n");
        for (expr, v) in looked {
            match v {
                Ok(v) => out.push_str(&format!("  {expr} = {v}\n")),
                Err(e) => out.push_str(&format!("  {expr}: erro: {e}\n")),
            }
        }
    }
    let mut captures = report.captures;
    captures.extend(last);
    enlarge(&mut captures, zoom);
    Ok(with_images(out, captures))
}

/// Junta o texto e as capturas (as primeiras e a última, até o limite).
/// Amplia as capturas de um jogo de tela pequena (pixel art em 320x180, por
/// exemplo), com pixels inteiros: `scale` vezes, ou o que der uns 960 de
/// largura.
pub(super) fn enlarge(captures: &mut [Capture], scale: Option<u32>) {
    for c in captures.iter_mut() {
        let k = scale
            .unwrap_or(if c.width < 640 {
                960 / c.width.max(1)
            } else {
                1
            })
            .clamp(1, 8);
        if k <= 1 {
            continue;
        }
        let Ok(img) = image::load_from_memory(&c.png) else {
            continue;
        };
        let big = image::imageops::resize(
            &img.to_rgba8(),
            c.width * k,
            c.height * k,
            image::imageops::FilterType::Nearest,
        );
        let mut png = Vec::new();
        if big
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .is_ok()
        {
            c.png = png;
            c.label = format!("{} (ampliada {k}x)", c.label);
        }
    }
}

pub(super) fn with_images(mut text: String, mut captures: Vec<Capture>) -> Output {
    if captures.len() > MAX_IMAGES {
        let extra = captures.len() - MAX_IMAGES;
        let last = captures.pop();
        captures.truncate(MAX_IMAGES - 1);
        captures.extend(last);
        text.push_str(&format!(
            "\n({extra} capturas não foram mostradas: no máximo {MAX_IMAGES} por vez.)\n"
        ));
    }
    if !captures.is_empty() {
        text.push_str("\nCapturas (na ordem):");
        for c in &captures {
            text.push_str(&format!(
                "\n  {} — {:.2} s, {}x{}",
                c.label, c.time, c.width, c.height
            ));
        }
        text.push('\n');
    }
    let mut out = Output::text(text);
    for c in &captures {
        out = out.and_image(&c.png, "image/png");
    }
    out
}

pub(super) fn kind_lines(lines: &[LogLine], kind: LineKind) -> Vec<&LogLine> {
    lines.iter().filter(|l| l.kind == kind).collect()
}

pub(super) fn write_lines(out: &mut String, title: &str, lines: &[&LogLine]) {
    if lines.is_empty() {
        return;
    }
    out.push_str(&format!("\n{title} ({}):\n", lines.len()));
    for l in lines.iter().take(MAX_LINES) {
        let text = l.text.replace('\n', "\n          ");
        out.push_str(&format!("  {:.2} s  {text}\n", l.time));
    }
    if lines.len() > MAX_LINES {
        out.push_str(&format!("  ... e mais {}\n", lines.len() - MAX_LINES));
    }
}

/// Erros, avisos, prints e regras.
fn write_report(out: &mut String, report: &Report) {
    if report.errors.is_empty() && kind_lines(&report.lines, LineKind::Error).is_empty() {
        out.push_str("\nErros: nenhum.\n");
    } else {
        out.push_str("\nErros:\n");
        for e in &report.errors {
            let msg = e.error.message.replace('\n', "\n    ");
            let seguiu = if e.error.isolated {
                " (só esse comportamento foi desligado, e o jogo seguiu)"
            } else {
                ""
            };
            out.push_str(&format!(
                "  {:.2} s  em {}{seguiu}:\n    {msg}\n",
                e.time, e.error.scene
            ));
        }
        for l in kind_lines(&report.lines, LineKind::Error) {
            out.push_str(&format!("  {:.2} s  {}\n", l.time, l.text));
        }
    }
    write_lines(out, "Avisos", &kind_lines(&report.lines, LineKind::Warning));
    write_lines(out, "Prints", &kind_lines(&report.lines, LineKind::Print));
    if report.dropped_lines > 0 {
        out.push_str(&format!(
            "  ({} linhas do log ficaram de fora: muitas)\n",
            report.dropped_lines
        ));
    }
    if !report.sounds.is_empty() {
        let list: Vec<String> = report
            .sounds
            .iter()
            .map(|(p, n, t)| {
                if *n > 1 {
                    format!("{p} ({n} vezes, a primeira em {t:.2} s)")
                } else {
                    format!("{p} (em {t:.2} s)")
                }
            })
            .collect();
        out.push_str(&format!(
            "\nSons que tocaram (sem som aqui; só a lista): {}\n",
            list.join(", ")
        ));
    }
    if report.perf.frames > 0 {
        let p = &report.perf;
        let n = p.frames as f32;
        out.push_str(&format!(
            "\nDesempenho (média por quadro, nesta máquina e sem placa de vídeo): passos fixos {:.2} ms (física {:.2} ms), update {:.2} ms, draw do jogo {:.2} ms; o quadro mais pesado: {:.1} ms em {:.2} s. Aqui o Lua roda mais devagar que no jogo de verdade (até uns 40%), pela checagem de laço sem fim deste modo.\n",
            p.total.fixed / n,
            p.total.physics / n,
            p.total.update / n,
            p.total.draw / n,
            p.worst.0,
            p.worst.1
        ));
        out.push_str(&format!(
            "No fim: {} texturas na memória ({:.1} MB){}. Sem placa de vídeo, as imagens não são desenhadas de verdade: o tempo do desenho na placa e o número de chamadas de desenho só aparecem no jogo aberto (F3).\n",
            p.textures.0,
            p.textures.1 as f64 / (1024.0 * 1024.0),
            if p.objects > 0 {
                format!(", {} objetos na cena", p.objects)
            } else {
                String::new()
            }
        ));
    }
    if !report.mid_loads.is_empty() {
        out.push_str("\nCarregou no meio da cena (uma travada para quem joga; o que a cena vai usar pode ir na lista `preload` dela, ou num componente):\n");
        for m in report.mid_loads.iter().take(10) {
            let files = if m.files.is_empty() {
                "(arquivos sem nome)".to_string()
            } else {
                m.files.join(", ")
            };
            out.push_str(&format!(
                "  {:.2} s em {} ({:.1} ms): {files}\n",
                m.time, m.scene, m.ms
            ));
        }
        if report.mid_loads.len() > 10 {
            out.push_str(&format!(
                "  ... e mais {} vezes\n",
                report.mid_loads.len() - 10
            ));
        }
    }
    if !report.animations.is_empty() {
        // Por modelo: as animações na ordem em que começaram.
        let mut models: Vec<(&str, Vec<String>)> = Vec::new();
        for (m, name, n, t) in &report.animations {
            let item = if *n > 1 {
                format!("{name} ({n} vezes, a primeira em {t:.2} s)")
            } else {
                format!("{name} (em {t:.2} s)")
            };
            match models.iter_mut().find(|(x, _)| x == m) {
                Some((_, v)) => v.push(item),
                None => models.push((m, vec![item])),
            }
        }
        out.push_str("\nAnimações 3D que começaram a tocar:\n");
        for (m, v) in models {
            out.push_str(&format!("  {m}: {}\n", v.join(", ")));
        }
    }
    if !report.fired.is_empty() {
        out.push_str("\nRegras dos eventos que dispararam:\n");
        for f in report.fired.iter().take(MAX_LINES) {
            let times = if f.count > 1 {
                let list: Vec<String> = f.times.iter().map(|t| format!("{t:.2}")).collect();
                let more = if f.count as usize > f.times.len() {
                    ", ..."
                } else {
                    ""
                };
                format!("  ({} vezes: {} s{more})", f.count, list.join(", "))
            } else {
                String::new()
            };
            out.push_str(&format!(
                "  {:.2} s  {}: {}{times}\n",
                f.first, f.scene, f.text
            ));
        }
    }
}

fn show(v: &Value) -> String {
    match v {
        Value::String(s) => format!("\"{s}\""),
        Value::Null => "nada".into(),
        // 480.0 e 480 são o mesmo número (um veio do arquivo, o outro da física).
        Value::Number(n) => match n.as_f64() {
            Some(f) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", f as i64),
            Some(f) => format!("{}", (f * 100.0).round() / 100.0),
            None => n.to_string(),
        },
        other => other.to_string(),
    }
}

/// Valores iguais (números comparados como números).
fn same(a: &Value, b: &Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => (x - y).abs() < 1e-9,
        _ => a == b,
    }
}

/// As propriedades de um objeto do estado, como "comportamento.campo" →
/// valor.
fn flat_props(o: &Value) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    if let Some(props) = o.get("props").and_then(Value::as_object) {
        for (b, fields) in props {
            if let Some(fields) = fields.as_object() {
                for (k, v) in fields {
                    out.insert(format!("{b}.{k}"), v.clone());
                }
            }
        }
    }
    out
}

const FIELDS: [&str; 8] = ["x", "y", "z", "rotation", "rx", "ry", "rz", "visible"];

fn object_label(o: &Value) -> String {
    format!(
        "{} ({})",
        o["name"].as_str().unwrap_or("?"),
        o["id"].as_i64().unwrap_or(0)
    )
}

fn objects_by_id(state: &Value) -> Vec<(i64, &Value)> {
    state
        .get("objects")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .map(|o| (o["id"].as_i64().unwrap_or(0), o))
                .collect()
        })
        .unwrap_or_default()
}

fn map_of(v: Option<&Value>) -> Map<String, Value> {
    v.and_then(Value::as_object).cloned().unwrap_or_default()
}

fn diff_maps(before: &Map<String, Value>, after: &Map<String, Value>) -> Vec<String> {
    let mut out = Vec::new();
    for (k, v) in after {
        match before.get(k) {
            Some(old) if same(old, v) => {}
            Some(old) => out.push(format!("{k} {} → {}", show(old), show(v))),
            None => out.push(format!("{k} = {} (nova)", show(v))),
        }
    }
    out
}

/// O que mudou do começo para o fim (ou o estado no fim, se a cena mudou).
fn write_state(
    out: &mut String,
    start_scene: &str,
    before: Option<&Value>,
    end_scene: &str,
    after: Option<&Value>,
) {
    write_state_since(
        out,
        start_scene,
        before,
        end_scene,
        after,
        "do começo ao fim",
    );
}

/// Como [`write_state`], dizendo desde quando (`since`).
pub(super) fn write_state_since(
    out: &mut String,
    start_scene: &str,
    before: Option<&Value>,
    end_scene: &str,
    after: Option<&Value>,
    since: &str,
) {
    let Some(after) = after else {
        return;
    };
    let vars_after = map_of(after.get("vars"));
    let same_scene = start_scene == end_scene && before.is_some();
    if same_scene && let Some(before) = before {
        let mut lines = Vec::new();
        let after_objs = objects_by_id(after);
        for (id, b) in objects_by_id(before) {
            let Some((_, a)) = after_objs.iter().find(|(i, _)| *i == id) else {
                lines.push(format!("{}: sumiu (destruído)", object_label(b)));
                continue;
            };
            let mut changes = Vec::new();
            for f in FIELDS {
                if let (Some(x), Some(y)) = (b.get(f), a.get(f))
                    && !same(x, y)
                {
                    changes.push(format!("{f} {} → {}", show(x), show(y)));
                }
            }
            let (pb, pa) = (flat_props(b), flat_props(a));
            for (k, v) in &pa {
                match pb.get(k) {
                    Some(old) if same(old, v) => {}
                    Some(old) => changes.push(format!("{k} {} → {}", show(old), show(v))),
                    None => changes.push(format!("{k} = {}", show(v))),
                }
            }
            if !changes.is_empty() {
                lines.push(format!("{}: {}", object_label(a), changes.join(", ")));
            }
        }
        let before_objs = objects_by_id(before);
        for (id, a) in &after_objs {
            if !before_objs.iter().any(|(i, _)| i == id) {
                lines.push(format!("{}: apareceu", object_label(a)));
            }
        }
        if after.get("visual").is_none() {
            lines.extend(diff_maps(
                &map_of(before.get("fields")),
                &map_of(after.get("fields")),
            ));
        }
        let (sb, sa) = (
            before["spawned"].as_u64().unwrap_or(0),
            after["spawned"].as_u64().unwrap_or(0),
        );
        if sb != sa {
            lines.push(format!("objetos criados pelo jogo: {sb} → {sa}"));
        }
        out.push_str(&format!("\nO que mudou em {end_scene} {since}:\n"));
        if lines.is_empty() {
            out.push_str("  nada (nenhum objeto se mexeu ou mudou)\n");
        }
        let total = lines.len();
        for l in lines.into_iter().take(MAX_LINES) {
            out.push_str(&format!("  {l}\n"));
        }
        if total > MAX_LINES {
            out.push_str(&format!("  ... e mais {}\n", total - MAX_LINES));
        }
        let vars = diff_maps(&map_of(before.get("vars")), &vars_after);
        if !vars.is_empty() {
            out.push_str(&format!("Variáveis que mudaram: {}\n", vars.join(", ")));
        }
    } else {
        out.push_str(&format!("\nOs objetos de {end_scene}:\n"));
        let objs = objects_by_id(after);
        for (_, o) in objs.iter().take(MAX_LINES) {
            let mut fields = Vec::new();
            for f in FIELDS {
                if let Some(v) = o.get(f) {
                    fields.push(format!("{f} {}", show(v)));
                }
            }
            out.push_str(&format!("  {}: {}\n", object_label(o), fields.join(", ")));
        }
        if objs.len() > MAX_LINES {
            out.push_str(&format!("  ... e mais {}\n", objs.len() - MAX_LINES));
        }
        let fields = map_of(after.get("fields"));
        if !fields.is_empty() {
            let list: Vec<String> = fields
                .iter()
                .take(MAX_LINES)
                .map(|(k, v)| format!("{k} = {}", show(v)))
                .collect();
            out.push_str(&format!("  self: {}\n", list.join(", ")));
        }
    }
    if !vars_after.is_empty() {
        let list: Vec<String> = vars_after
            .iter()
            .map(|(k, v)| format!("{k} = {}", show(v)))
            .collect();
        let when = if since == "do começo ao fim" {
            "no fim"
        } else {
            "agora"
        };
        out.push_str(&format!("Variáveis {when}: {}\n", list.join(", ")));
    }
}

/// O estado completo dos objetos pedidos.
pub(super) fn write_objects(out: &mut String, after: Option<&Value>, names: &[String]) {
    out.push_str("\nObjetos pedidos:\n");
    let objs = after.map(objects_by_id).unwrap_or_default();
    for name in names {
        let found: Vec<&&Value> = objs
            .iter()
            .map(|(_, o)| o)
            .filter(|o| o["name"].as_str() == Some(name))
            .collect();
        if found.is_empty() {
            out.push_str(&format!("  {name}: não está na cena (ou foi destruído)\n"));
        }
        for o in found {
            let mut o = (*o).clone();
            if let Some(m) = o.as_object_mut() {
                m.remove("name");
            }
            out.push_str(&format!("  {name}: {o}\n"));
        }
    }
}

// rodar_testes ----------------------------------------------------------------

fn rodar_testes(s: &mut Server, a: &Value) -> Result<Output, String> {
    need_project(s)?;
    let project = nn_script::Project::open(&s.root).map_err(|e| e.to_string())?;
    let opts = TestOptions {
        filter: opt_str(a, "filtro")?.map(str::to_string),
        file: opt_str(a, "arquivo")?.map(str::to_string),
        capture_failures: true,
        ..TestOptions::default()
    };
    let prints = opt_bool(a, "prints")?.unwrap_or(false);
    let summary = testing::run_tests(&project, &opts, |_| {})?;
    let mut out = String::new();
    if summary.results.is_empty() && summary.file_errors.is_empty() {
        out.push_str(
            "Nenhum teste encontrado. Escreva um arquivo em testes/ (ex.: testes/jogo.lua) com \
             test(\"o que confere\", function(t) ... end); veja buscar_documentacao \
             \"testes de jogo\".\n",
        );
        return Ok(Output::text(out));
    }
    out.push_str(&format!(
        "{} testes: {} passaram, {} falharam ({:.1} s).\n",
        summary.results.len(),
        summary.passed(),
        summary.failed(),
        summary.wall.as_secs_f64()
    ));
    for e in &summary.file_errors {
        out.push_str(&format!("\n✗ {} não carregou:\n  {}\n", e.file, e.message));
    }
    let mut images = Vec::new();
    out.push('\n');
    for r in &summary.results {
        match &r.failure {
            None => {
                out.push_str(&format!(
                    "✓ {} › {} ({:.2} s de jogo)\n",
                    r.case.file, r.case.name, r.time
                ));
                if prints {
                    for l in kind_lines(&r.report.lines, LineKind::Print) {
                        out.push_str(&format!("  print em {:.2} s: {}\n", l.time, l.text));
                    }
                }
            }
            Some(f) => {
                out.push_str(&format!(
                    "✗ {} › {} (falhou em {:.2} s de jogo):\n  {}\n",
                    r.case.file,
                    r.case.name,
                    r.time,
                    f.replace('\n', "\n  ")
                ));
                // Os erros do jogo já estão na falha; o que mais ajuda é o
                // que o jogo escreveu.
                let mut detail = String::new();
                write_lines(
                    &mut detail,
                    "Erros da engine",
                    &kind_lines(&r.report.lines, LineKind::Error),
                );
                write_lines(
                    &mut detail,
                    "Avisos",
                    &kind_lines(&r.report.lines, LineKind::Warning),
                );
                write_lines(
                    &mut detail,
                    "Prints",
                    &kind_lines(&r.report.lines, LineKind::Print),
                );
                if !detail.is_empty() {
                    out.push_str(detail.trim_end().replace('\n', "\n  ").trim_start());
                    out.push('\n');
                }
                if let Some(c) = &r.last_frame {
                    let mut c = c.clone();
                    c.label = format!("{} (quando falhou)", r.case.name);
                    images.push(c);
                }
            }
        }
        for c in &r.report.captures {
            let mut c = c.clone();
            c.label = format!("{} › {}", r.case.name, c.label);
            images.push(c);
        }
    }
    enlarge(&mut images, None);
    Ok(with_images(out, images))
}

pub static TOOLS: &[&Tool] = &[
    &Tool {
        name: "rodar_jogo",
        title: "Rodar o jogo",
        description: "Roda o jogo sem janela (60 quadros por segundo de jogo, o mais rápido que der, sem som, salvo só na memória) e mostra o que aconteceu: erros (com arquivo e linha), avisos, prints, regras dos eventos que dispararam, trocas de cena, o que mudou nos objetos e nas variáveis, e capturas da tela (a do fim e as pedidas). Simule o jogador com \"acoes\", cada uma no seu tempo \"em\" (segundos): {\"em\": 0.5, \"apertar\": \"space\"} (um toque de um quadro: num pulo que fica mais alto segurando, dá um pulinho), {\"em\": 1, \"segurar\": \"right\", \"por\": 2}, {\"soltar\": \"right\"}, {\"clicar\": [x, y]} (pixels da tela, y para baixo) ou {\"clicar\": \"nome do objeto\"}, {\"mouse\": [x, y]}, {\"rolar\": -3} (a roda do mouse: negativo desce a lista; ponha o mouse em cima antes), {\"girar_mouse\": [300, 0], \"por\": 0.5} (o mouse preso andando: gira a câmera do Jogador 3D), {\"digitar\": \"texto\"}, {\"capturar\": true} ou {\"capturar\": \"nome\"}, {\"lua\": \"self.objects.heroi:move_to(0, 0)\"} (código com self = a cena da frente: para preparar a situação; self.objects.nome é o primeiro com esse nome, e self:find_all(\"nome\") devolve todos). Teclas: os nomes do nn.input (a-z, 0-9, space, enter, escape, left, right, up, down, shift, ctrl...). Com \"segundos\": 0, só mostra a primeira tela.",
        params: || {
            vec![
                (
                    "segundos",
                    s_num("Quanto tempo de jogo rodar (padrão 3, até 120)"),
                    false,
                ),
                (
                    "cena",
                    s_str("Começar nesta cena em vez da inicial (ex.: \"cenas/fase.cena\")"),
                    false,
                ),
                (
                    "acoes",
                    s_list(
                        "O que o jogador faz, cada ação no seu tempo (veja a descrição)",
                        s_obj(
                            "{\"em\": segundos, e uma de: apertar, segurar (+ por), soltar, clicar (+ botao), mouse, digitar, capturar, lua}",
                        ),
                    ),
                    false,
                ),
                (
                    "olhar",
                    s_list(
                        "Expressões Lua para ver no fim, com self = a cena da frente (ex.: \"self.objects.heroi.body.vx\", \"nn.vars.moedas\")",
                        json!({ "type": "string" }),
                    ),
                    false,
                ),
                (
                    "objetos",
                    s_list(
                        "Nomes de objetos para ver por inteiro no fim (posição, propriedades dos comportamentos)",
                        json!({ "type": "string" }),
                    ),
                    false,
                ),
                (
                    "captura_final",
                    s_bool(
                        "Mandar a captura da tela do fim (padrão true). Vão no máximo 5 imagens por chamada, contando esta: com 5 \"capturar\" no roteiro, use false",
                    ),
                    false,
                ),
                (
                    "ampliar",
                    s_int(
                        "Ampliar as capturas N vezes, com pixels inteiros (padrão: num jogo de tela pequena, como 320x180 de pixel art, o que der uns 960 de largura; 1 = sem ampliar)",
                    ),
                    false,
                ),
                (
                    "ver_corpos",
                    s_bool(
                        "Desenhar nas capturas os corpos da física (o que colide de verdade): cinza parado, verde que cai, azul que se move sozinho, amarelo sensor. Bom quando algo não encosta ou bate no ar.",
                    ),
                    false,
                ),
            ]
        },
        read_only: true,
        destructive: false,
        run: rodar_jogo,
    },
    &Tool {
        name: "rodar_testes",
        title: "Rodar os testes",
        description: "Roda os testes de jogo do projeto (arquivos Lua em testes/, cada teste num jogo novo, sem janela) e mostra quais passaram e por que os outros falharam: a linha do teste, o erro do jogo, os prints e a tela quando falhou. Um teste: test(\"o herói pega a moeda\", function(t) t:open(\"cenas/fase.cena\"); t:hold(\"right\", 1); t:check(nn.vars.moedas == 1, \"devia ter pegado\") end). Veja buscar_documentacao \"testes de jogo\" para tudo o que o t faz.",
        params: || {
            vec![
                (
                    "arquivo",
                    s_str("Só os testes deste arquivo (ex.: \"testes/fase.lua\")"),
                    false,
                ),
                (
                    "filtro",
                    s_str("Só os testes cujo nome tem este texto"),
                    false,
                ),
                (
                    "prints",
                    s_bool(
                        "Mostra os prints também dos testes que passaram (para medir alguma coisa)",
                    ),
                    false,
                ),
            ]
        },
        read_only: true,
        destructive: false,
        run: rodar_testes,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acoes_conferidas() {
        let ok = parse_actions(
            &json!([
                { "em": 0.5, "apertar": "space" },
                { "segurar": "right" },
                { "em": 1, "segurar": "a", "por": 0.25 },
                { "clicar": [10, 20], "botao": "right" },
                { "clicar": "botao_jogar" },
                { "capturar": true },
                { "capturar": "pulo" },
                { "lua": "self.x = 1" },
                { "digitar": "Nereu" },
                { "mouse": [1, 2] },
                { "soltar": "a" },
            ]),
            3.0,
        )
        .unwrap();
        assert_eq!(ok.len(), 11);
        assert_eq!(ok[1].kind, ActionKind::Hold("right".into(), 3.0));
        assert_eq!(ok[2].kind, ActionKind::Hold("a".into(), 0.25));
        assert_eq!(
            ok[3].kind,
            ActionKind::Click {
                x: 10.0,
                y: 20.0,
                button: "right".into()
            }
        );
        assert_eq!(ok[5].kind, ActionKind::Capture(String::new()));
        let err = |v: Value| parse_actions(&json!([v]), 3.0).unwrap_err();
        assert!(err(json!({ "apertar": "pulo" })).contains("tecla desconhecida"));
        assert!(err(json!({ "apertar": "pulo" })).contains("space"));
        assert!(err(json!({ "em": -1, "apertar": "a" })).contains("\"em\""));
        assert!(err(json!({ "apertar": "a", "clicar": [1, 2] })).contains("uma coisa por ação"));
        assert!(err(json!({ "em": 1 })).contains("diga o que fazer"));
        assert!(err(json!({ "apertar": "a", "tempo": 1 })).contains("\"tempo\" não existe"));
        assert!(err(json!({ "clicar": [1] })).contains("[x, y]"));
        assert!(err(json!({ "clicar": [1, 2], "botao": "meio" })).contains("middle"));
        assert!(err(json!("apertar")).contains("objeto"));
    }

    #[test]
    fn diferencas_do_estado() {
        let before = json!({ "visual": true, "vars": { "moedas": 0 }, "spawned": 0, "objects": [
            { "id": 1, "name": "heroi", "x": 0, "y": 0, "rotation": 0, "visible": true,
              "props": { "nn:plataforma": { "no_chao": false } } },
            { "id": 2, "name": "moeda", "x": 50, "y": 0, "rotation": 0, "visible": true, "props": {} },
        ] });
        let after = json!({ "visual": true, "vars": { "moedas": 1, "vidas": 3 }, "spawned": 2, "objects": [
            { "id": 1, "name": "heroi", "x": 40.5, "y": 0, "rotation": 0, "visible": true,
              "props": { "nn:plataforma": { "no_chao": true } } },
        ] });
        let mut out = String::new();
        write_state(&mut out, "a.cena", Some(&before), "a.cena", Some(&after));
        assert!(
            out.contains("heroi (1): x 0 → 40.5, nn:plataforma.no_chao false → true"),
            "{out}"
        );
        assert!(out.contains("moeda (2): sumiu"), "{out}");
        assert!(out.contains("objetos criados pelo jogo: 0 → 2"), "{out}");
        assert!(out.contains("moedas 0 → 1, vidas = 3 (nova)"), "{out}");
        let mut out = String::new();
        write_state(&mut out, "a.cena", Some(&before), "b.cena", Some(&after));
        assert!(
            out.contains("Os objetos de b.cena:\n  heroi (1): x 40.5"),
            "{out}"
        );
        let mut out = String::new();
        write_objects(&mut out, Some(&after), &["heroi".into(), "dragao".into()]);
        assert!(out.contains("heroi: {\"id\":1"), "{out}");
        assert!(out.contains("dragao: não está"), "{out}");
    }
}
