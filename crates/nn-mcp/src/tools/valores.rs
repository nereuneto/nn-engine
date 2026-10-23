//! Objetos, componentes e regras em JSON, no mesmo formato do arquivo
//! `.cena` (o que `ver_cena` mostra, `criar_objeto` aceita), conferidos
//! contra o esquema da engine.

use serde_json::{Map, Value, json};

use nn_projeto::behaviors::{BehaviorInfo, PropSpec, PropType};
use nn_projeto::events;
use nn_projeto::scene_data::{
    self, BEHAVIOR, Component, EventItem, Object, PrefabLink, Prop, Rule, SceneData,
};

use crate::Server;

/// Componentes que só existem numa cena 3D.
pub const KINDS_3D: [&str; 9] = [
    "model", "anim3d", "shape3d", "light3d", "camera3d", "body3d", "terrain", "text3d", "bone",
];

/// Campos do próprio objeto, como no arquivo.
pub const OBJECT_FIELDS: [&str; 15] = [
    "name", "x", "y", "z", "rotation", "rx", "ry", "rz", "scale_x", "scale_y", "scale_z", "layer",
    "visible", "locked", "anchor",
];

/// "; parecidas: ..." com os nomes que lembram `wanted` (ou nada).
pub fn suggest(wanted: &str, names: &[&str]) -> String {
    let w = crate::paths::fold(wanted);
    let parts: Vec<&str> = w
        .split(|c: char| !c.is_alphanumeric())
        .filter(|p| p.len() >= 3)
        .collect();
    let close: Vec<&str> = names
        .iter()
        .copied()
        .filter(|n| {
            let n = crate::paths::fold(n);
            n.contains(&w) || w.contains(&n) || parts.iter().any(|p| n.contains(p))
        })
        .take(6)
        .collect();
    if close.is_empty() {
        String::new()
    } else {
        format!("; parecidos: {}", close.join(", "))
    }
}

/// Lista curta para mensagens de erro.
pub fn names<S: AsRef<str>>(list: &[S]) -> String {
    list.iter()
        .map(|s| s.as_ref())
        .collect::<Vec<_>>()
        .join(", ")
}

fn is_color(s: &str) -> bool {
    let Some(hex) = s.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 4 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit())
}

fn kind_name(p: &Prop) -> &'static str {
    match p {
        Prop::Num(_) => "um número",
        Prop::Bool(_) => "true ou false",
        Prop::Str(_) => "um texto",
    }
}

/// Converte para o tipo do padrão (`expected`). Texto aceita número (vira
/// texto); cores são conferidas.
fn convert(key: &str, v: &Value, expected: &Prop, color: bool) -> Result<Prop, String> {
    let p = match (expected, v) {
        (Prop::Num(_), Value::Number(n)) => Prop::Num(n.as_f64().unwrap_or(0.0)),
        (Prop::Bool(_), Value::Bool(b)) => Prop::Bool(*b),
        (Prop::Str(_), Value::String(s)) => Prop::Str(s.clone()),
        (Prop::Str(_), Value::Number(n)) => Prop::Str(n.to_string()),
        _ => {
            return Err(format!(
                "\"{key}\" precisa ser {} (veio {v})",
                kind_name(expected)
            ));
        }
    };
    if color
        && let Prop::Str(s) = &p
        && !s.is_empty()
        && !is_color(s)
    {
        return Err(format!(
            "\"{key}\": a cor vai como \"#RRGGBB\" ou \"#RRGGBBAA\" (veio \"{s}\")"
        ));
    }
    Ok(p)
}

/// Confere e converte um valor de propriedade de comportamento ou evento.
pub fn spec_value(spec: &PropSpec, v: &Value, three_d: bool) -> Result<Prop, String> {
    let key = &spec.key;
    let p = match (spec.kind, v) {
        // Número, texto ou {variavel}: vale como veio.
        (PropType::Value, Value::Number(n)) => Prop::Num(n.as_f64().unwrap_or(0.0)),
        (PropType::Value, Value::String(s)) => Prop::Str(s.clone()),
        (PropType::Value, Value::Bool(b)) => Prop::Bool(*b),
        _ => convert(
            key,
            v,
            &spec.default_for(three_d),
            spec.kind == PropType::Color,
        )?,
    };
    if !spec.options.is_empty() {
        let s = match &p {
            Prop::Str(s) => s.clone(),
            other => other.to_lua(),
        };
        let empty_ok = spec.allow_empty.is_some() && s.is_empty();
        if !empty_ok && !spec.options.iter().any(|(k, _)| *k == s) {
            let opts: Vec<String> = spec
                .options
                .iter()
                .map(|(k, l)| {
                    if k == l {
                        format!("\"{k}\"")
                    } else {
                        format!("\"{k}\" ({l})")
                    }
                })
                .collect();
            return Err(format!(
                "\"{key}\" não aceita \"{s}\". Opções: {}",
                opts.join(", ")
            ));
        }
    }
    if let Prop::Num(n) = p {
        if let Some(min) = spec.min
            && n < min
        {
            return Err(format!("\"{key}\" vai de {min} para cima (veio {n})"));
        }
        if let Some(max) = spec.max
            && n > max
        {
            return Err(format!("\"{key}\" vai até {max} (veio {n})"));
        }
        if spec.integer && n.fract() != 0.0 {
            return Err(format!(
                "\"{key}\" precisa ser um número inteiro (veio {n})"
            ));
        }
    }
    Ok(p)
}

/// O que um `type` de componente quer dizer.
pub enum Target {
    Kind(String),
    Behavior(Box<BehaviorInfo>),
}

impl Server {
    /// Informações de um comportamento, com erro se o script não existe ou
    /// não é um comportamento.
    pub(crate) fn behavior(&mut self, script: &str) -> Result<BehaviorInfo, String> {
        if !script.starts_with("nn:") {
            let (rel, full) = crate::paths::resolve(&self.root, script)?;
            if !full.is_file() {
                return Err(format!(
                    "o script {rel} não existe. Crie-o com escrever_arquivo (veja \"comportamentos\" em buscar_documentacao) ou use um pronto (listar_comportamentos)"
                ));
            }
        }
        let info = self.catalog.get(&self.root, script);
        match &info.error {
            Some(e) if script.starts_with("nn:") => {
                Err(format!("{e}. Os prontos estão em listar_comportamentos"))
            }
            Some(e) => Err(format!("{script} não é um comportamento válido: {e}")),
            None => Ok(info),
        }
    }

    /// Lê o `type` (e o `script`) de um componente em JSON.
    pub(crate) fn target(&mut self, c: &Map<String, Value>) -> Result<Target, String> {
        let kind = match c.get("type") {
            Some(Value::String(s)) => s.trim().to_string(),
            _ => {
                return Err(
                    "cada componente precisa de \"type\": \"sprite\", \"body\"... ou um comportamento (\"nn:plataforma\", \"scripts/x.lua\")"
                        .into(),
                );
            }
        };
        let script = if kind == BEHAVIOR {
            match c.get("script") {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("um \"behavior\" precisa de \"script\" (\"nn:plataforma\" ou o caminho do .lua)".into()),
            }
        } else if kind.starts_with("nn:") || kind.ends_with(".lua") {
            kind.clone()
        } else if scene_data::schema().fields(&kind).is_some() {
            return Ok(Target::Kind(kind));
        } else if nn_projeto::behaviors::builtins()
            .iter()
            .any(|b| b.script == format!("nn:{kind}"))
        {
            format!("nn:{kind}")
        } else {
            let behaviors: Vec<&str> = nn_projeto::behaviors::builtins()
                .iter()
                .map(|b| b.script.as_str())
                .collect();
            return Err(format!(
                "componente desconhecido: \"{kind}\"{}. Componentes: {}. Comportamentos: \"nn:...\" (listar_comportamentos) ou o caminho de um script",
                suggest(&kind, &behaviors),
                names(&scene_data::schema().order)
            ));
        };
        Ok(Target::Behavior(Box::new(self.behavior(&script)?)))
    }
}

/// Muda os campos de um componente (todos os de `c`, menos `type` e
/// `script`). `null` volta ao padrão.
pub fn apply_fields(
    comp: &mut Component,
    target: &Target,
    c: &Map<String, Value>,
    three_d: bool,
) -> Result<(), String> {
    for (key, v) in c {
        if key == "type" || (key == "script" && matches!(target, Target::Behavior(_))) {
            continue;
        }
        match target {
            Target::Kind(kind) => {
                let fields = scene_data::schema().fields(kind).unwrap_or(&[]);
                let Some((_, default)) = fields.iter().find(|(k, _)| k == key) else {
                    let keys: Vec<&str> = fields.iter().map(|(k, _)| k.as_str()).collect();
                    return Err(format!(
                        "\"{kind}\" não tem o campo \"{key}\". Campos: {}",
                        names(&keys)
                    ));
                };
                let p = if v.is_null() {
                    default.clone()
                } else {
                    convert(key, v, default, key.contains("color"))
                        .map_err(|e| format!("{kind}: {e}"))?
                };
                if let (Prop::Str(text), Some(choices)) = (&p, scene_data::field_choices(kind, key))
                    && !choices.contains(&text.as_str())
                {
                    return Err(format!(
                        "{kind}: \"{key}\" = \"{text}\" não existe; use {}",
                        choices
                            .iter()
                            .map(|c| format!("\"{c}\""))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                comp.set(key, p);
            }
            Target::Behavior(info) => {
                let Some(spec) = info.prop(key) else {
                    let keys: Vec<&str> = info.props.iter().map(|p| p.key.as_str()).collect();
                    return Err(format!(
                        "{} não tem a propriedade \"{key}\". Propriedades: {}",
                        info.script,
                        if keys.is_empty() {
                            "(nenhuma)".into()
                        } else {
                            names(&keys)
                        }
                    ));
                };
                let p = if v.is_null() {
                    spec.default_for(three_d)
                } else {
                    spec_value(spec, v, three_d).map_err(|e| format!("{}: {e}", info.script))?
                };
                comp.set(key, p);
            }
        }
    }
    Ok(())
}

/// Componente novo, com os padrões e os campos de `c`.
pub fn new_component(
    target: &Target,
    c: &Map<String, Value>,
    three_d: bool,
) -> Result<Component, String> {
    let mut comp = match target {
        Target::Kind(kind) => Component::new(kind),
        Target::Behavior(info) => Component::behavior(&info.script, &info.defaults_for(three_d)),
    };
    apply_fields(&mut comp, target, c, three_d)?;
    Ok(comp)
}

/// O componente do objeto que corresponde a `target` (o mesmo tipo; num
/// comportamento, o mesmo script).
pub fn find_component(o: &Object, target: &Target) -> Option<usize> {
    o.components.iter().position(|comp| match target {
        Target::Kind(k) => comp.kind == *k,
        Target::Behavior(info) => comp.is_behavior() && comp.str("script") == info.script,
    })
}

/// Põe no objeto o que o comportamento precisa (como o NN Studio faz) e
/// devolve avisos do que falta.
pub fn add_requirements(
    root: &std::path::Path,
    o: &mut Object,
    info: &BehaviorInfo,
    three_d: bool,
) -> Vec<String> {
    let mut notes = Vec::new();
    for req in &info.requires {
        let req = if req == "text" && three_d {
            "text3d"
        } else {
            req.as_str()
        };
        if o.component(req).is_some() || (req == "text3d" && o.component("text").is_some()) {
            continue;
        }
        match req {
            "body3d" => {
                let mut body = Component::new("body3d");
                if !info.body.is_empty() {
                    body.set("body", Prop::Str(info.body.clone()));
                }
                o.components.insert(0, body);
                notes.push(format!(
                    "\"{}\": ganhou um body3d ({}), que {} precisa",
                    o.name,
                    if info.body.is_empty() {
                        "padrão"
                    } else {
                        &info.body
                    },
                    info.script
                ));
            }
            "body" => {
                let mut body = Component::new("body");
                // Do tamanho do que o objeto mostra: o w e o h do sprite
                // ou, com 0, o quadro (ou a imagem toda).
                let sprite_size = o.component("sprite").and_then(|c| {
                    let (w, h) = (c.num("w"), c.num("h"));
                    if w > 0.0 && h > 0.0 {
                        return Some((w, h));
                    }
                    let (fw, fh) = (c.num("frame_w"), c.num("frame_h"));
                    if fw > 0.0 && fh > 0.0 {
                        return Some((fw, fh));
                    }
                    let img = c.str("image");
                    if img.is_empty() {
                        return None;
                    }
                    if let Ok(a) = nn_projeto::anim::AnimFile::load(root, img)
                        && a.frame_w > 0
                        && a.frame_h > 0
                    {
                        return Some((f64::from(a.frame_w), f64::from(a.frame_h)));
                    }
                    image::image_dimensions(root.join(img))
                        .ok()
                        .map(|(w, h)| (f64::from(w), f64::from(h)))
                });
                if let Some(size) = sprite_size
                    .or_else(|| o.component("shape").map(|c| (c.num("w"), c.num("h"))))
                    .filter(|(w, h)| *w > 0.0 && *h > 0.0)
                {
                    body.set("w", Prop::Num(size.0.round()));
                    body.set("h", Prop::Num(size.1.round()));
                }
                let at = o
                    .components
                    .iter()
                    .position(Component::is_behavior)
                    .unwrap_or(o.components.len());
                o.components.insert(at, body);
                notes.push(format!(
                    "\"{}\": ganhou um body de {}x{}, do tamanho do que ele mostra, que {} precisa (para a colisão ficar só nos pés ou no corpo, diminua w e h e desloque com ox e oy)",
                    o.name,
                    body_size(o).0,
                    body_size(o).1,
                    info.script
                ));
            }
            other => notes.push(format!(
                "aviso: \"{}\" usa {}, que precisa de um componente \"{other}\"",
                o.name, info.script
            )),
        }
    }
    if !info.fits(three_d) {
        notes.push(format!(
            "aviso: {} é para cenas {}, e esta cena é {}",
            info.script,
            info.mode.to_uppercase(),
            if three_d { "3D" } else { "2D" }
        ));
    }
    notes
}

/// Numa tela pequena (pixel art), os números pensados para 720 de altura
/// ficam grandes: velocidades, pulos e letras que não mudaram do padrão
/// ganham uma sugestão na escala da tela.
pub fn small_screen_notes(
    o: &Object,
    screen: (u32, u32),
    lookup: &mut impl FnMut(&str) -> Option<BehaviorInfo>,
) -> Vec<String> {
    let (w, h) = screen;
    let k = f64::from(h) / 720.0;
    if k >= 0.75 {
        return Vec::new();
    }
    // (comportamento ou componente, campo, como escala).
    const FIELDS: &[(&str, &str, bool)] = &[
        ("nn:plataforma", "velocidade", false),
        ("nn:plataforma", "pulo", true),
        ("nn:topdown", "velocidade", false),
        ("nn:patrulha", "velocidade", false),
        ("nn:perseguir", "velocidade", false),
        ("nn:atirar", "velocidade", false),
        ("nn:inventario", "tamanho", false),
        ("nn:inventario", "margem", false),
        ("text", "size", false),
        ("button", "size", false),
        ("button", "w", false),
        ("button", "h", false),
    ];
    let mut out = Vec::new();
    scene_data::visit(std::slice::from_ref(o), &mut |o| {
        let mut found = Vec::new();
        for c in &o.components {
            let kind = if c.is_behavior() {
                c.str("script").to_string()
            } else {
                c.kind.clone()
            };
            for (who, key, sqrt) in FIELDS {
                if *who != kind {
                    continue;
                }
                let default = if c.is_behavior() {
                    lookup(&kind).and_then(|i| i.prop(key).map(|p| p.default.num()))
                } else {
                    scene_data::schema().default_of(&kind, key).map(Prop::num)
                };
                let Some(d) = default.filter(|d| *d > 0.0) else {
                    continue;
                };
                let now = c.get(key).map(Prop::num).unwrap_or(d);
                if (now - d).abs() > 1e-9 {
                    continue;
                }
                let f = if *sqrt { k.sqrt() } else { k };
                found.push(format!(
                    "{who}.{key} {} → uns {}",
                    scene_data::format_num(d),
                    (d * f).round()
                ));
            }
        }
        if !found.is_empty() {
            out.push(format!(
                "aviso: \"{}\": a tela é de {w}x{h}, e estes números (padrão) são para uma tela de 720 de altura; nesta, algo como: {}",
                o.name,
                found.join(", ")
            ));
        }
    });
    out
}

fn body_size(o: &Object) -> (String, String) {
    let b = o.component("body");
    let f = |k: &str| {
        b.map(|b| scene_data::format_num(b.num(k)))
            .unwrap_or_default()
    };
    (f("w"), f("h"))
}

/// Muda um campo do próprio objeto.
pub fn set_object_field(o: &mut Object, key: &str, v: &Value) -> Result<(), String> {
    let num = || {
        v.as_f64()
            .ok_or_else(|| format!("\"{key}\" precisa ser um número"))
    };
    match key {
        "name" => {
            let s = v
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .ok_or("\"name\" precisa ser um texto (não vazio)")?;
            o.name = s.to_string();
        }
        "x" => o.x = num()?,
        "y" => o.y = num()?,
        "z" => o.z = num()?,
        "rotation" => o.rotation = num()?,
        "rx" => o.rx = num()?,
        "ry" => o.ry = num()?,
        "rz" => o.rz = num()?,
        "scale_x" => o.scale_x = num()?,
        "scale_y" => o.scale_y = num()?,
        "scale_z" => o.scale_z = num()?,
        "layer" => o.layer = num()?.round() as i64,
        "visible" => o.visible = v.as_bool().ok_or("\"visible\" precisa ser true ou false")?,
        "locked" => o.locked = v.as_bool().ok_or("\"locked\" precisa ser true ou false")?,
        "anchor" => {
            let s = v.as_str().ok_or("\"anchor\" precisa ser um texto")?;
            let ok = s.is_empty() || scene_data::SCREEN_ANCHORS.iter().any(|a| a.0 == s);
            if !ok {
                let all: Vec<&str> = scene_data::SCREEN_ANCHORS.iter().map(|a| a.0).collect();
                return Err(format!(
                    "\"anchor\": use \"\" ou um destes: {}",
                    names(&all)
                ));
            }
            o.anchor = s.to_string();
        }
        other => {
            return Err(format!(
                "o objeto não tem o campo \"{other}\". Campos: {}, components, children, prefab, events",
                names(&OBJECT_FIELDS)
            ));
        }
    }
    Ok(())
}

impl Server {
    /// Objeto (com filhos) a partir do JSON. Os ids vêm de `next_id`.
    pub(crate) fn object_from_json(
        &mut self,
        v: &Value,
        three_d: bool,
        next_id: &mut u32,
        notes: &mut Vec<String>,
    ) -> Result<Object, String> {
        let Some(map) = v.as_object() else {
            return Err(
                "o objeto vai como { \"name\": ..., \"x\": ..., \"components\": [...] }".into(),
            );
        };
        let name = map
            .get("name")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
            .ok_or("todo objeto precisa de \"name\"")?;
        let mut o = Object::new(*next_id, name);
        *next_id += 1;
        if let Some(path) = map.get("prefab") {
            let path = path.as_str().ok_or("\"prefab\" é o caminho do .prefab")?;
            let (rel, full) = crate::paths::resolve(&self.root, path)?;
            if !full.is_file() {
                return Err(format!("o prefab {rel} não existe"));
            }
            let base = scene_data::load_prefab(&self.root, &rel)?;
            o.prefab = Some(PrefabLink {
                path: rel,
                base: None,
                raw: Vec::new(),
            });
            o.layer = base.layer;
            o.link_prefab(base);
        }
        for (key, value) in map {
            match key.as_str() {
                "prefab" | "id" | "events" | "children" | "components" => {}
                _ => set_object_field(&mut o, key, value).map_err(|e| format!("{name}: {e}"))?,
            }
        }
        if let Some(list) = map.get("components") {
            let list = list.as_array().ok_or("\"components\" é uma lista")?;
            for c in list {
                let c = c
                    .as_object()
                    .ok_or("cada componente é um objeto com \"type\"")?;
                let target = self.target(c)?;
                if o.prefab.is_some() {
                    // Numa cópia de prefab só se mudam valores.
                    let Some(i) = find_component(&o, &target) else {
                        return Err(format!(
                            "{name}: é uma cópia do prefab e ele não tem esse componente; mude o próprio prefab (cena = o arquivo .prefab)"
                        ));
                    };
                    apply_fields(&mut o.components[i], &target, c, three_d)?;
                    continue;
                }
                let comp =
                    new_component(&target, c, three_d).map_err(|e| format!("{name}: {e}"))?;
                o.components.push(comp);
                if let Target::Behavior(info) = &target {
                    notes.extend(add_requirements(&self.root, &mut o, info, three_d));
                }
            }
        }
        if let Some(list) = map.get("events") {
            if o.prefab.is_some() {
                return Err(format!(
                    "{name}: as regras de uma cópia são as do prefab; mude o prefab"
                ));
            }
            let list = list.as_array().ok_or("\"events\" é uma lista de regras")?;
            for r in list {
                o.events.push(rule_from_json(r, true, three_d)?);
            }
        }
        if let Some(list) = map.get("children") {
            let list = list
                .as_array()
                .ok_or("\"children\" é uma lista de objetos")?;
            for c in list {
                let child = self.object_from_json(c, three_d, next_id, notes)?;
                o.children.push(child);
            }
        }
        Ok(o)
    }
}

/// O objeto em JSON, no formato do arquivo. `full`: com todos os campos
/// (senão, só o que difere do padrão).
pub fn object_json(
    o: &Object,
    three_d: bool,
    full: bool,
    catalog: &mut impl FnMut(&str) -> Option<BehaviorInfo>,
) -> Value {
    let mut m = Map::new();
    m.insert("id".into(), json!(o.id));
    m.insert("name".into(), json!(o.name));
    let fields: &[&str] = if three_d {
        &[
            "x", "y", "z", "rx", "ry", "rz", "scale_x", "scale_y", "scale_z",
        ]
    } else {
        &["x", "y", "rotation", "scale_x", "scale_y"]
    };
    let blank = Object::new(0, "");
    for key in fields
        .iter()
        .chain(&["layer", "visible", "locked", "anchor"])
    {
        let v = o.field(key);
        if (full || v != blank.field(key))
            && let Some(v) = v
        {
            m.insert(key.to_string(), v.to_json());
        }
    }
    if let Some(link) = &o.prefab {
        m.insert("prefab".into(), json!(link.path));
    }
    let base = o.prefab.as_ref().and_then(|l| l.base.as_deref());
    let comps: Vec<Value> = o
        .components
        .iter()
        .enumerate()
        .map(|(i, c)| {
            // Numa cópia de prefab, o "padrão" é o valor do prefab.
            let from_prefab = base.and_then(|b| b.components.get(i));
            component_json(c, three_d, full, from_prefab, catalog)
        })
        // Numa cópia, só os componentes que ela muda.
        .filter(|c| full || base.is_none() || c.as_object().is_some_and(|m| m.len() > 2))
        .collect();
    if !comps.is_empty() {
        m.insert("components".into(), Value::Array(comps));
    }
    if !o.events.is_empty() {
        m.insert(
            "events".into(),
            Value::Array(rules_json(&o.events, true, o.prefab.is_some())),
        );
    }
    if !o.children.is_empty() {
        m.insert(
            "children".into(),
            Value::Array(
                o.children
                    .iter()
                    .map(|c| object_json(c, three_d, full, catalog))
                    .collect(),
            ),
        );
    }
    Value::Object(m)
}

fn component_json(
    c: &Component,
    three_d: bool,
    full: bool,
    from_prefab: Option<&Component>,
    catalog: &mut impl FnMut(&str) -> Option<BehaviorInfo>,
) -> Value {
    let mut m = Map::new();
    m.insert("type".into(), json!(c.kind));
    let info = if c.is_behavior() {
        catalog(c.str("script"))
    } else {
        None
    };
    for (k, v) in &c.props {
        let default = match (from_prefab, &info) {
            (Some(p), _) => p.get(k).cloned(),
            (None, Some(info)) => info.prop(k).map(|p| p.default_for(three_d)),
            (None, None) => scene_data::schema().default_of(&c.kind, k).cloned(),
        };
        if full || k == "script" || default.as_ref() != Some(v) {
            m.insert(k.clone(), v.to_json());
        }
    }
    Value::Object(m)
}

/// Regras em JSON, com o número de cada uma e a frase.
pub fn rules_json(rules: &[Rule], in_object: bool, from_prefab: bool) -> Vec<Value> {
    rules
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut v = r.to_json();
            if let Value::Object(m) = &mut v {
                m.insert("indice".into(), json!(i + 1));
                m.insert("frase".into(), json!(events::rule_summary(r, in_object)));
                if from_prefab {
                    m.insert("do_prefab".into(), json!(true));
                }
            }
            v
        })
        .collect()
}

/// Uma regra a partir do JSON (o formato do arquivo: `when`, `actions`,
/// `mode = "enquanto"`, `off`, `comment`), conferida contra o catálogo.
pub fn rule_from_json(v: &Value, in_object: bool, three_d: bool) -> Result<Rule, String> {
    let m = v
        .as_object()
        .ok_or("a regra vai como { \"when\": [...], \"actions\": [...] }")?;
    events::set_three_d(three_d);
    let mut rule = Rule::default();
    for (key, value) in m {
        match key.as_str() {
            "when" | "actions" => {}
            "mode" => {
                rule.repeat = match value.as_str() {
                    Some("enquanto") => true,
                    Some("quando") | None => false,
                    Some(other) => {
                        return Err(format!(
                            "\"mode\" é \"quando\" (uma vez, ao acontecer) ou \"enquanto\" (a cada quadro); veio \"{other}\""
                        ));
                    }
                }
            }
            "off" => rule.off = value.as_bool().unwrap_or(false),
            "comment" => rule.comment = value.as_str().map(str::to_string),
            // O que ver_cena acrescenta.
            "indice" | "frase" | "do_prefab" => {}
            other => {
                return Err(format!(
                    "a regra não tem \"{other}\". Use when, actions, mode, off, comment"
                ));
            }
        }
    }
    let cat = events::catalog();
    for (key, cond) in [("when", true), ("actions", false)] {
        let Some(list) = m.get(key) else {
            continue;
        };
        let list = list
            .as_array()
            .ok_or_else(|| format!("\"{key}\" é uma lista"))?;
        for item in list {
            let im = item.as_object().ok_or_else(|| {
                format!(
                    "cada item de \"{key}\" é {{ \"{}\": \"id\", ...campos }}",
                    if cond { "cond" } else { "act" }
                )
            })?;
            let id_key = if cond { "cond" } else { "act" };
            let id = im
                .get(id_key)
                .and_then(Value::as_str)
                .ok_or_else(|| format!("falta \"{id_key}\" num item de \"{key}\""))?;
            let def = cat.get(cond, id).ok_or_else(|| {
                let list = if cond { &cat.conditions } else { &cat.actions };
                let ids: Vec<&str> = list.iter().map(|d| d.id.as_str()).collect();
                format!(
                    "{} desconhecida: \"{id}\"{} (veja listar_eventos)",
                    if cond { "condição" } else { "ação" },
                    suggest(id, &ids)
                )
            })?;
            if !def.fits() {
                return Err(format!(
                    "\"{id}\" só funciona em cenas {}",
                    def.mode.to_uppercase()
                ));
            }
            let mut it: EventItem = def.new_item(in_object);
            for (k, value) in im {
                if k == id_key {
                    continue;
                }
                if k == "nao" && cond {
                    it.not = value.as_bool().unwrap_or(false);
                    continue;
                }
                let Some(spec) = def.prop(k) else {
                    let keys: Vec<&str> = def.props_now().map(|p| p.key.as_str()).collect();
                    return Err(format!(
                        "\"{id}\" não tem o campo \"{k}\". Campos: {}",
                        if keys.is_empty() {
                            "(nenhum)".into()
                        } else {
                            names(&keys)
                        }
                    ));
                };
                let p = spec_value(spec, value, three_d).map_err(|e| format!("{id}: {e}"))?;
                it.set(k, p);
            }
            if cond {
                rule.when.push(it);
            } else {
                rule.actions.push(it);
            }
        }
    }
    if rule.comment.is_none() && rule.actions.is_empty() {
        return Err("a regra precisa de pelo menos uma ação em \"actions\" (ou só \"comment\", para uma linha de comentário)".into());
    }
    Ok(rule)
}

/// A cena (ou o prefab) tem coisa de 3D?
pub fn looks_3d(scene: &SceneData) -> bool {
    if scene.three_d {
        return true;
    }
    let mut found = false;
    scene_data::visit(&scene.objects, &mut |o| {
        found |= o
            .components
            .iter()
            .any(|c| KINDS_3D.contains(&c.kind.as_str()));
    });
    found
}

/// Extensões de arquivos que as cenas citam.
const ASSET_EXTS: [&str; 18] = [
    ".png", ".jpg", ".jpeg", ".wav", ".ogg", ".mp3", ".flac", ".cena", ".prefab", ".tmj", ".tsj",
    ".glb", ".gltf", ".dialogo", ".anim", ".lua", ".ttf", ".otf",
];

pub fn looks_like_file(v: &str) -> bool {
    let lower = v.to_lowercase();
    !v.starts_with("nn:") && !v.contains(' ') && ASSET_EXTS.iter().any(|e| lower.ends_with(e))
}

/// Arquivos que uma regra cita e que não existem (uma cena que ainda vai
/// ser criada, um som com o nome errado).
pub fn rule_missing_files(root: &std::path::Path, rule: &scene_data::Rule) -> Vec<String> {
    let mut out = Vec::new();
    for item in rule.when.iter().chain(&rule.actions) {
        for (k, v) in &item.props {
            if let Prop::Str(path) = v
                && looks_like_file(path)
                && !root.join(path).is_file()
            {
                out.push(format!(
                    "aviso: {}.{k} = \"{path}\" não existe (ainda?): crie antes de rodar, ou confira em listar_arquivos",
                    item.id
                ));
            }
        }
    }
    out
}

/// Arquivos que o objeto (e os filhos) cita e que não existem.
pub fn missing_files(root: &std::path::Path, o: &Object) -> Vec<String> {
    let mut out = Vec::new();
    scene_data::visit(std::slice::from_ref(o), &mut |o| {
        for c in &o.components {
            for (k, v) in &c.props {
                if let Prop::Str(path) = v
                    && looks_like_file(path)
                    && !root.join(path).is_file()
                {
                    out.push(format!(
                        "aviso: \"{}\": {}.{k} = \"{path}\" não existe (veja listar_arquivos)",
                        o.name, c.kind
                    ));
                }
            }
        }
    });
    out
}

/// Avisa quando o corpo da física de um objeto 2D com escala fica bem
/// maior ou menor que o sprite: o `w`/`h` dos dois é multiplicado pela
/// escala, e é fácil pôr no corpo o tamanho já multiplicado.
pub fn size_notes(o: &Object) -> Vec<String> {
    let mut out = Vec::new();
    scene_data::visit(std::slice::from_ref(o), &mut |o| {
        let (sx, sy) = (o.scale_x, o.scale_y);
        if (sx - 1.0).abs() < 1e-9 && (sy - 1.0).abs() < 1e-9 {
            return;
        }
        let (Some(sprite), Some(body)) = (o.component("sprite"), o.component("body")) else {
            return;
        };
        if body.str("shape") == "circle" {
            return;
        }
        let num = |c: &scene_data::Component, k: &str, d: f64| c.get(k).map(Prop::num).unwrap_or(d);
        let (spw, sph) = (num(sprite, "w", 0.0), num(sprite, "h", 0.0));
        let (bw, bh) = (num(body, "w", 32.0), num(body, "h", 32.0));
        if spw <= 0.0 || sph <= 0.0 {
            return;
        }
        let off = |b: f64, s: f64| b > s * 2.0 || b < s / 2.0;
        if off(bw, spw) || off(bh, sph) {
            out.push(format!(
                "aviso: \"{}\": o corpo (body) fica {:.0}×{:.0} px no jogo e o sprite {:.0}×{:.0}: o w e o h dos dois são multiplicados pela escala ({sx}×{sy}). Para o corpo cobrir o sprite, use o mesmo w e h dele ({spw}×{sph}).",
                o.name,
                bw * sx.abs(),
                bh * sy.abs(),
                spw * sx.abs(),
                sph * sy.abs(),
            ));
        }
    });
    out
}

/// Um sprite com `fill = "repeat"` e w/h bem maiores que a imagem: quase
/// sempre era para ser a escala (w e h são o tamanho de cada cópia).
pub fn repeat_notes(root: &std::path::Path, o: &Object) -> Vec<String> {
    let mut out = Vec::new();
    scene_data::visit(std::slice::from_ref(o), &mut |o| {
        let Some(sprite) = o.component("sprite") else {
            return;
        };
        if sprite.str("fill") != "repeat" {
            return;
        }
        let Some((iw, ih)) = std::fs::read(root.join(sprite.str("image")))
            .ok()
            .and_then(|d| super::arquivos::image_size(&d))
        else {
            return;
        };
        let fw = match sprite.num("frame_w") {
            w if w > 0.0 => w,
            _ => f64::from(iw),
        };
        let fh = match sprite.num("frame_h") {
            h if h > 0.0 => h,
            _ => f64::from(ih),
        };
        let (w, h) = (sprite.num("w"), sprite.num("h"));
        if w > fw * 2.0 || h > fh * 2.0 {
            out.push(format!(
                "aviso: \"{}\": com fill = \"repeat\", w e h são o tamanho de CADA cópia; {w}x{h} estica o tile de {fw}x{fh}. Para repetir, use w {fw}, h {fh} e a escala do objeto para quantas cópias (scale_x {:.0} = {:.0} cópias na largura); o corpo (body) segue a mesma conta.",
                o.name,
                (w / fw).round(),
                (w / fw).round(),
            ));
        }
    });
    out
}

/// Ordem das chaves ao mostrar: a do arquivo `.cena`.
fn key_order(m: &Map<String, Value>) -> Vec<&String> {
    const FIRST: [&str; 31] = [
        "indice",
        "frase",
        "do_prefab",
        "id",
        "name",
        "type",
        "script",
        "cond",
        "act",
        "nao",
        "comment",
        "mode",
        "off",
        "prefab",
        "x",
        "y",
        "z",
        "rotation",
        "rx",
        "ry",
        "rz",
        "scale_x",
        "scale_y",
        "scale_z",
        "layer",
        "visible",
        "locked",
        "anchor",
        "when",
        "actions",
        "environment",
    ];
    const LAST: [&str; 4] = ["components", "events", "children", "objects"];
    let schema: Vec<&str> = m
        .get("type")
        .and_then(Value::as_str)
        .and_then(|k| scene_data::schema().fields(k))
        .map(|f| f.iter().map(|(k, _)| k.as_str()).collect())
        .unwrap_or_default();
    let rank = |k: &str| {
        if let Some(i) = FIRST.iter().position(|f| *f == k) {
            (0, i)
        } else if let Some(i) = LAST.iter().position(|f| *f == k) {
            (3, i)
        } else if let Some(i) = schema.iter().position(|f| *f == k) {
            (1, i)
        } else {
            (2, 0)
        }
    };
    let mut keys: Vec<&String> = m.keys().collect();
    keys.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.cmp(b)));
    keys
}

/// JSON legível e curto: objetos sem nada dentro (um componente, um item
/// de regra) numa linha só, e as chaves na ordem do arquivo.
pub fn pretty(v: &Value) -> String {
    fn flat(v: &Value) -> bool {
        match v {
            Value::Object(m) => m.values().all(|x| !x.is_object() && !x.is_array()),
            Value::Array(a) => a.iter().all(|x| !x.is_object() && !x.is_array()),
            _ => true,
        }
    }
    fn inline(v: &Value, out: &mut String) {
        match v {
            Value::Object(m) => {
                out.push('{');
                for (i, k) in key_order(m).into_iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&Value::String(k.clone()).to_string());
                    out.push_str(": ");
                    inline(&m[k], out);
                }
                out.push('}');
            }
            Value::Array(a) => {
                out.push('[');
                for (i, x) in a.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    inline(x, out);
                }
                out.push(']');
            }
            other => out.push_str(&other.to_string()),
        }
    }
    fn go(v: &Value, depth: usize, out: &mut String) {
        if flat(v) {
            inline(v, out);
            return;
        }
        let pad = "  ".repeat(depth + 1);
        let end = "  ".repeat(depth);
        match v {
            Value::Object(m) => {
                out.push_str("{\n");
                let keys = key_order(m);
                for (i, k) in keys.iter().enumerate() {
                    out.push_str(&pad);
                    out.push_str(&Value::String((*k).clone()).to_string());
                    out.push_str(": ");
                    go(&m[*k], depth + 1, out);
                    if i + 1 < keys.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&end);
                out.push('}');
            }
            Value::Array(a) => {
                out.push_str("[\n");
                for (i, x) in a.iter().enumerate() {
                    out.push_str(&pad);
                    go(x, depth + 1, out);
                    if i + 1 < a.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&end);
                out.push(']');
            }
            _ => inline(v, out),
        }
    }
    let mut out = String::new();
    go(v, 0, &mut out);
    out
}
