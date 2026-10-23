//! Cenas, objetos, componentes, comportamentos e prefabs.

use serde_json::{Map, Value, json};

use nn_projeto::behaviors::{self, BehaviorInfo, PropSpec};
use nn_projeto::scene_data::{self, Object, PrefabLink, Prop, SceneData};

use super::valores::{self, Target, names};
use super::{
    Output, Tool, arg_str, opt_bool, opt_num, opt_str, s_bool, s_enum, s_int, s_list, s_obj, s_ref,
    s_str,
};
use crate::Server;

/// Uma cena ou um prefab aberto para mudar. Um prefab vira uma cena com um
/// objeto só na raiz.
pub struct Doc {
    pub rel: String,
    pub prefab: bool,
    pub data: SceneData,
}

impl Doc {
    pub fn three_d(&self) -> bool {
        valores::looks_3d(&self.data)
    }
}

impl Server {
    /// Lê uma cena (`.cena`) ou um prefab (`.prefab`) do projeto.
    pub(crate) fn open_doc(&self, rel: &str) -> Result<Doc, String> {
        let (rel, text) = self.read_text(rel)?;
        self.prepare_screen();
        let prefab = rel.ends_with(".prefab");
        let mut data = if prefab {
            let obj = scene_data::read_prefab(&text, &rel)?;
            SceneData {
                objects: vec![obj],
                ..Default::default()
            }
        } else if rel.ends_with(".cena") {
            scene_data::read(&text, &rel)?
        } else {
            return Err(format!(
                "{rel} não é uma cena visual (.cena) nem um prefab (.prefab). Scripts Lua mudam com escrever_arquivo"
            ));
        };
        data.link_prefabs(&self.root);
        if prefab {
            data.three_d = valores::looks_3d(&data);
        }
        Ok(Doc { rel, prefab, data })
    }

    /// Grava a cena ou o prefab no formato do NN Studio.
    pub(crate) fn save_doc(&mut self, doc: &Doc) -> Result<(), String> {
        let text = if doc.prefab {
            if doc.data.objects.len() != 1 {
                return Err("um prefab tem um objeto só na raiz".into());
            }
            scene_data::write_prefab(&doc.data.objects[0])
        } else {
            scene_data::write(&doc.data)
        };
        self.write_file(&doc.rel, text.as_bytes())
    }

    /// O tamanho da tela do jogo (as posições dos objetos presos à tela
    /// dependem dele).
    fn prepare_screen(&self) {
        if let Ok(text) = std::fs::read_to_string(self.root.join("config.lua"))
            && let Ok(cfg) = nn_projeto::config_lua::read(&text, "")
        {
            let (w, h) = cfg.game_size();
            scene_data::set_screen_size(f64::from(w), f64::from(h));
        }
    }

    pub(crate) fn behavior_lookup(&mut self) -> impl FnMut(&str) -> Option<BehaviorInfo> + '_ {
        let root = self.root.clone();
        move |script: &str| {
            let info = self.catalog.get(&root, script);
            info.error.is_none().then_some(info)
        }
    }
}

/// O id de um objeto pelo id ou pelo nome.
pub fn find_object(data: &SceneData, r: &Value) -> Result<u32, String> {
    let all = || {
        let mut v = Vec::new();
        scene_data::visit(&data.objects, &mut |o| {
            v.push(format!("{} ({})", o.name, o.id))
        });
        if v.len() > 40 {
            v.truncate(40);
            v.push("...".into());
        }
        v.join(", ")
    };
    match r {
        Value::Number(n) => {
            let id = n.as_u64().unwrap_or(0) as u32;
            data.find(id)
                .map(|o| o.id)
                .ok_or_else(|| format!("não há objeto com id {id}. Objetos: {}", all()))
        }
        Value::String(s) => {
            if let Ok(id) = s.trim().parse::<u32>()
                && data.find(id).is_some()
            {
                return Ok(id);
            }
            let mut found = Vec::new();
            scene_data::visit(&data.objects, &mut |o| {
                if o.name == *s {
                    found.push(o.id);
                }
            });
            if found.is_empty() {
                let wanted = crate::paths::fold(s);
                scene_data::visit(&data.objects, &mut |o| {
                    if crate::paths::fold(&o.name) == wanted {
                        found.push(o.id);
                    }
                });
            }
            match found.as_slice() {
                [id] => Ok(*id),
                [] => Err(format!("não há objeto \"{s}\". Objetos: {}", all())),
                ids => Err(format!(
                    "há {} objetos \"{s}\" (ids {}); use o id",
                    ids.len(),
                    ids.iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            }
        }
        _ => Err("\"objeto\" é o id (número) ou o nome".into()),
    }
}

fn header(doc: &Doc) -> String {
    let d = &doc.data;
    let mut count = 0;
    scene_data::visit(&d.objects, &mut |_| count += 1);
    if doc.prefab {
        format!(
            "Prefab {} ({}): {count} objeto(s).",
            doc.rel,
            if doc.three_d() { "3D" } else { "2D" }
        )
    } else {
        let mut s = format!(
            "Cena {} ({}{}): {count} objeto(s), {} regra(s) da cena. background = {}, gravity = {}",
            doc.rel,
            if d.three_d { "3D" } else { "2D" },
            if d.overlay { ", abre por cima" } else { "" },
            d.events.len(),
            if d.background.is_empty() {
                "(nenhum)".to_string()
            } else {
                d.background.clone()
            },
            scene_data::format_num(d.gravity),
        );
        if !d.script.is_empty() {
            s.push_str(&format!(", script = {}", d.script));
        }
        s.push('.');
        s
    }
}

fn scene_json(server: &mut Server, doc: &Doc, only: Option<u32>, full: bool) -> Value {
    let three_d = doc.three_d();
    let mut lookup = server.behavior_lookup();
    if let Some(id) = only {
        let o = doc.data.find(id).expect("objeto existe");
        return valores::object_json(o, three_d, full, &mut lookup);
    }
    let mut m = Map::new();
    if doc.data.three_d && !doc.prefab {
        let env: Map<String, Value> = doc
            .data
            .environment
            .iter()
            .filter(|(k, v)| {
                full || scene_data::schema()
                    .environment
                    .iter()
                    .find(|(dk, _)| dk == k)
                    .map(|(_, dv)| dv)
                    != Some(v)
            })
            .map(|(k, v)| (k.clone(), v.to_json()))
            .collect();
        m.insert("environment".into(), Value::Object(env));
    }
    if !doc.data.events.is_empty() {
        m.insert(
            "events".into(),
            Value::Array(valores::rules_json(&doc.data.events, false, false)),
        );
    }
    m.insert(
        "objects".into(),
        Value::Array(
            doc.data
                .objects
                .iter()
                .map(|o| valores::object_json(o, three_d, full, &mut lookup))
                .collect(),
        ),
    );
    Value::Object(m)
}

use valores::pretty;

// ver_cena ---------------------------------------------------------------

fn ver_cena(s: &mut Server, a: &Value) -> Result<Output, String> {
    let doc = s.open_doc(arg_str(a, "cena")?)?;
    let full = opt_bool(a, "completo")?.unwrap_or(false);
    let only = match a.get("objeto") {
        None | Some(Value::Null) => None,
        Some(r) => Some(find_object(&doc.data, r)?),
    };
    let json = scene_json(s, &doc, only, full);
    let mut text = header(&doc);
    if !full {
        text.push_str(
            " (Só os campos diferentes do padrão; completo = true mostra todos. Numa cópia de prefab, os diferentes do prefab.)",
        );
    }
    Ok(Output::text(format!("{text}\n{}", pretty(&json))))
}

// criar_cena -------------------------------------------------------------

fn criar_cena(s: &mut Server, a: &Value) -> Result<Output, String> {
    let rel = crate::paths::clean(arg_str(a, "caminho")?)?;
    if !rel.ends_with(".cena") {
        return Err("a cena precisa terminar em .cena (ex.: cenas/fase2.cena)".into());
    }
    if s.root.join(&rel).exists() {
        return Err(format!(
            "{rel} já existe; mude-a com mudar_cena e as ferramentas de objeto"
        ));
    }
    let three_d = match opt_str(a, "tipo")? {
        Some("3d") => true,
        Some("2d") => false,
        Some(other) => return Err(format!("\"tipo\" é \"2d\" ou \"3d\" (veio \"{other}\")")),
        None => s.project_is_3d(),
    };
    let mut data = if three_d {
        let mut d = SceneData::new_3d();
        // Só a câmera e o chão.
        d.objects.retain(|o| o.name != "Cubo");
        d
    } else {
        SceneData::default()
    };
    if let Some(bg) = opt_str(a, "background")? {
        data.background = bg.to_string();
    }
    data.overlay = opt_bool(a, "por_cima")?.unwrap_or(false);
    if data.overlay && data.background.is_empty() {
        data.background = "#00000099".into();
    }
    let doc = Doc {
        rel: rel.clone(),
        prefab: false,
        data,
    };
    s.save_doc(&doc)?;
    let mut msg = format!(
        "Criei {rel} ({}). Ponha objetos com criar_objeto.",
        if three_d {
            "3D, com câmera e chão"
        } else {
            "2D, vazia"
        }
    );
    if opt_bool(a, "inicial")?.unwrap_or(false) {
        super::projeto::set_config(s, &json!({ "start": rel }))?;
        msg.push_str(" Ela agora é a cena inicial do jogo.");
    }
    Ok(Output::text(msg))
}

// mudar_cena -------------------------------------------------------------

fn mudar_cena(s: &mut Server, a: &Value) -> Result<Output, String> {
    let mut doc = s.open_doc(arg_str(a, "cena")?)?;
    if doc.prefab {
        return Err("um prefab não tem campos de cena; use mudar_objeto".into());
    }
    let d = &mut doc.data;
    if let Some(bg) = opt_str(a, "background")? {
        if !bg.is_empty() && !bg.starts_with('#') {
            return Err("\"background\" é uma cor \"#RRGGBB\" (ou vazio)".into());
        }
        d.background = bg.to_string();
    }
    if let Some(g) = opt_num(a, "gravity")? {
        d.gravity = g;
    }
    if let Some(o) = opt_bool(a, "por_cima")? {
        d.overlay = o;
    }
    if let Some(script) = opt_str(a, "script")? {
        if !script.is_empty() {
            let (rel, full) = crate::paths::resolve(&s.root, script)?;
            if !full.is_file() {
                return Err(format!("o script {rel} não existe"));
            }
        }
        d.script = script.to_string();
    }
    if let Some(env) = a.get("environment") {
        let env = env
            .as_object()
            .ok_or("\"environment\" é um objeto { campo: valor }")?;
        if !d.three_d {
            return Err("\"environment\" (céu, sol, névoa) é só de cenas 3D".into());
        }
        let schema = scene_data::schema();
        for (k, v) in env {
            let Some((_, default)) = schema.environment.iter().find(|(dk, _)| dk == k) else {
                let keys: Vec<&str> = schema.environment.iter().map(|(k, _)| k.as_str()).collect();
                return Err(format!(
                    "o ambiente não tem \"{k}\". Campos: {}",
                    names(&keys)
                ));
            };
            let p = match (default, v) {
                (Prop::Num(_), Value::Number(n)) => Prop::Num(n.as_f64().unwrap_or(0.0)),
                (Prop::Bool(_), Value::Bool(b)) => Prop::Bool(*b),
                (Prop::Str(_), Value::String(t)) => Prop::Str(t.clone()),
                _ => {
                    return Err(format!(
                        "\"{k}\": tipo errado (o padrão é {})",
                        default.to_lua()
                    ));
                }
            };
            d.set_env(k, p);
        }
    }
    s.save_doc(&doc)?;
    Ok(Output::text(format!("Pronto. {}", header(&doc))))
}

// criar_objeto -----------------------------------------------------------

fn parent_arg(doc: &Doc, a: &Value) -> Result<Option<Option<u32>>, String> {
    match a.get("pai") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) if s.is_empty() => Ok(Some(None)),
        Some(r) => Ok(Some(Some(find_object(&doc.data, r)?))),
    }
}

fn criar_objeto(s: &mut Server, a: &Value) -> Result<Output, String> {
    let mut doc = s.open_doc(arg_str(a, "cena")?)?;
    let parent = match parent_arg(&doc, a)? {
        Some(p) => p,
        // Num prefab, tudo fica dentro do objeto dele.
        None if doc.prefab => Some(doc.data.objects[0].id),
        None => None,
    };
    if doc.prefab && parent.is_none() {
        return Err("num prefab, o objeto novo precisa ficar dentro do objeto do prefab".into());
    }
    let objects: Vec<Value> = match (a.get("objeto"), a.get("objetos")) {
        (Some(o), None) => vec![o.clone()],
        (None, Some(Value::Array(list))) => list.clone(),
        _ => return Err("passe \"objeto\" (um) ou \"objetos\" (uma lista)".into()),
    };
    let three_d = doc.three_d();
    let mut next = doc.data.next_id();
    let mut notes = Vec::new();
    let mut created = Vec::new();
    let mut index = opt_num(a, "posicao")?.map(|p| p.max(0.0) as usize);
    for v in &objects {
        let obj = s.object_from_json(v, three_d, &mut next, &mut notes)?;
        notes.extend(valores::missing_files(&s.root, &obj));
        if !three_d {
            notes.extend(valores::size_notes(&obj));
            notes.extend(valores::repeat_notes(&s.root, &obj));
        }
        created.push(format!("\"{}\" (id {})", obj.name, obj.id));
        let len = match parent {
            Some(p) => doc.data.find(p).map(|o| o.children.len()).unwrap_or(0),
            None => doc.data.objects.len(),
        };
        let at = index.unwrap_or(len);
        doc.data.insert(parent, at, obj);
        index = index.map(|i| i + 1);
    }
    s.save_doc(&doc)?;
    let mut msg = format!("Criei em {}: {}.", doc.rel, created.join(", "));
    for n in notes {
        msg.push_str(&format!("\n- {n}"));
    }
    Ok(Output::text(msg))
}

// mudar_objeto -----------------------------------------------------------

/// Muda o pai mantendo o lugar no mundo (como arrastar na Hierarquia).
fn reparent(
    data: &mut SceneData,
    id: u32,
    parent: Option<u32>,
    index: Option<usize>,
) -> Result<(), String> {
    if parent == Some(id) || parent.is_some_and(|p| data.is_ancestor(id, p)) {
        return Err("um objeto não pode ficar dentro dele mesmo".into());
    }
    let len = match parent {
        Some(p) => data.find(p).map(|o| o.children.len()).unwrap_or(0),
        None => data.objects.len(),
    };
    let index = index.unwrap_or(len + 1);
    if data.three_d {
        let world = data.world_matrix3(id);
        let (obj, _, _) = data.remove(id).ok_or("objeto sumiu")?;
        data.insert(parent, index, obj);
        if let Some(w) = world {
            data.set_world_matrix3(id, w);
        }
        Ok(())
    } else if data.move_object(id, parent, index) {
        Ok(())
    } else {
        Err("não deu para mudar o objeto de lugar".into())
    }
}

fn mudar_objeto(s: &mut Server, a: &Value) -> Result<Output, String> {
    let mut doc = s.open_doc(arg_str(a, "cena")?)?;
    let id = find_object(&doc.data, a.get("objeto").ok_or("falta \"objeto\"")?)?;
    let three_d = doc.three_d();
    let mut notes = Vec::new();
    // Os alvos dos componentes primeiro (precisam do servidor).
    let mut comps = Vec::new();
    if let Some(list) = a.get("componentes") {
        for c in list.as_array().ok_or("\"componentes\" é uma lista")? {
            let c = c
                .as_object()
                .ok_or("cada componente é um objeto com \"type\"")?;
            comps.push((s.target(c)?, c.clone()));
        }
    }
    let mut remove = Vec::new();
    if let Some(list) = a.get("tirar_componentes") {
        for t in list
            .as_array()
            .ok_or("\"tirar_componentes\" é uma lista de tipos")?
        {
            let t = t
                .as_str()
                .ok_or("cada item de \"tirar_componentes\" é um tipo (\"body\", \"nn:vida\")")?;
            let mut m = Map::new();
            m.insert("type".into(), json!(t));
            remove.push((t.to_string(), s.target(&m)?));
        }
    }
    let o = doc.data.find_mut(id).expect("objeto existe");
    let name = o.name.clone();
    if let Some(m) = a.get("mudar") {
        let m = m
            .as_object()
            .ok_or("\"mudar\" é um objeto { campo: valor }")?;
        for (k, v) in m {
            valores::set_object_field(o, k, v).map_err(|e| format!("{name}: {e}"))?;
        }
    }
    for (label, target) in &remove {
        if o.prefab.is_some() {
            return Err(format!(
                "\"{name}\" é uma cópia de prefab: tire o componente do próprio prefab"
            ));
        }
        let i = valores::find_component(o, target)
            .ok_or_else(|| format!("\"{name}\" não tem {label}"))?;
        o.components.remove(i);
    }
    for (target, c) in &comps {
        match valores::find_component(o, target) {
            Some(i) => valores::apply_fields(&mut o.components[i], target, c, three_d)
                .map_err(|e| format!("{name}: {e}"))?,
            None if o.prefab.is_some() => {
                return Err(format!(
                    "\"{name}\" é uma cópia de prefab e o prefab não tem esse componente; ponha no próprio prefab"
                ));
            }
            None => {
                let comp = valores::new_component(target, c, three_d)
                    .map_err(|e| format!("{name}: {e}"))?;
                o.components.push(comp);
                if let Target::Behavior(info) = target {
                    notes.extend(valores::add_requirements(o, info, three_d));
                }
            }
        }
    }
    if let Some(parent) = parent_arg(&doc, a)? {
        if doc.prefab && parent.is_none() {
            return Err("num prefab, tudo fica dentro do objeto do prefab".into());
        }
        let index = opt_num(a, "posicao")?.map(|p| p.max(0.0) as usize);
        reparent(&mut doc.data, id, parent, index)?;
    } else if let Some(p) = opt_num(a, "posicao")? {
        let parent = doc.data.parent_of(id).flatten();
        reparent(&mut doc.data, id, parent, Some(p.max(0.0) as usize))?;
    }
    s.save_doc(&doc)?;
    notes.extend(valores::missing_files(
        &s.root,
        doc.data.find(id).expect("objeto"),
    ));
    if !three_d {
        let o = doc.data.find(id).expect("objeto");
        notes.extend(valores::size_notes(o));
        notes.extend(valores::repeat_notes(&s.root, o));
    }
    let mut lookup = s.behavior_lookup();
    let now = valores::object_json(
        doc.data.find(id).expect("objeto"),
        three_d,
        false,
        &mut lookup,
    );
    let mut msg = format!("Pronto. \"{name}\" agora:\n{}", pretty(&now));
    for n in notes {
        msg.push_str(&format!("\n- {n}"));
    }
    Ok(Output::text(msg))
}

// apagar_objeto, copiar_objeto ------------------------------------------

fn apagar_objeto(s: &mut Server, a: &Value) -> Result<Output, String> {
    let mut doc = s.open_doc(arg_str(a, "cena")?)?;
    let id = find_object(&doc.data, a.get("objeto").ok_or("falta \"objeto\"")?)?;
    if doc.prefab && doc.data.objects[0].id == id {
        return Err("este é o objeto do próprio prefab; para tirar o prefab, apague o arquivo (apagar_arquivo)".into());
    }
    let (obj, _, _) = doc.data.remove(id).ok_or("objeto não encontrado")?;
    let mut count = 0;
    scene_data::visit(std::slice::from_ref(&obj), &mut |_| count += 1);
    s.save_doc(&doc)?;
    Ok(Output::text(format!(
        "Apaguei \"{}\" (id {id}){}.",
        obj.name,
        if count > 1 {
            format!(" e {} filho(s)", count - 1)
        } else {
            String::new()
        }
    )))
}

fn copiar_objeto(s: &mut Server, a: &Value) -> Result<Output, String> {
    let mut doc = s.open_doc(arg_str(a, "cena")?)?;
    let id = find_object(&doc.data, a.get("objeto").ok_or("falta \"objeto\"")?)?;
    if doc.prefab && doc.data.objects[0].id == id {
        return Err(
            "o objeto do próprio prefab não se copia; use criar_objeto com \"prefab\" numa cena"
                .into(),
        );
    }
    let count = opt_num(a, "copias")?.unwrap_or(1.0).clamp(1.0, 500.0) as usize;
    let step = (
        opt_num(a, "passo_x")?.unwrap_or(0.0),
        opt_num(a, "passo_y")?.unwrap_or(0.0),
        opt_num(a, "passo_z")?.unwrap_or(0.0),
    );
    let mut created = Vec::new();
    let mut last = id;
    for n in 1..=count {
        let new_id = doc.data.duplicate(last).ok_or("não deu para copiar")?;
        let o = doc.data.find_mut(new_id).expect("cópia");
        o.x += step.0;
        o.y += step.1;
        o.z += step.2;
        if let Some(m) = a.get("mudar") {
            let m = m
                .as_object()
                .ok_or("\"mudar\" é um objeto { campo: valor }")?;
            for (k, v) in m {
                valores::set_object_field(o, k, v)?;
            }
            if count > 1 && m.contains_key("name") {
                o.name = format!("{}{n}", o.name);
            }
        }
        created.push(format!("\"{}\" (id {new_id})", o.name));
        last = new_id;
    }
    s.save_doc(&doc)?;
    Ok(Output::text(format!("Copiei: {}.", created.join(", "))))
}

// criar_prefab -----------------------------------------------------------

fn criar_prefab(s: &mut Server, a: &Value) -> Result<Output, String> {
    let mut doc = s.open_doc(arg_str(a, "cena")?)?;
    if doc.prefab {
        return Err("crie o prefab a partir de um objeto de uma cena".into());
    }
    let id = find_object(&doc.data, a.get("objeto").ok_or("falta \"objeto\"")?)?;
    let obj = doc.data.find(id).cloned().expect("objeto");
    if obj.prefab.is_some() {
        return Err(format!("\"{}\" já é cópia de um prefab", obj.name));
    }
    let rel = match opt_str(a, "caminho")? {
        Some(p) => {
            let rel = crate::paths::clean(p)?;
            if !rel.ends_with(".prefab") {
                return Err("o prefab precisa terminar em .prefab".into());
            }
            if s.root.join(&rel).exists() {
                return Err(format!("{rel} já existe"));
            }
            rel
        }
        None => {
            let slug: String = crate::paths::fold(&obj.name)
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                .collect();
            let slug = if slug.trim_matches('_').is_empty() {
                "objeto".to_string()
            } else {
                slug
            };
            let mut rel = format!("prefabs/{slug}.prefab");
            let mut n = 2;
            while s.root.join(&rel).exists() {
                rel = format!("prefabs/{slug}_{n}.prefab");
                n += 1;
            }
            rel
        }
    };
    // O prefab começa no (0, 0), com ids próprios.
    let mut base = obj.clone();
    base.x = 0.0;
    base.y = 0.0;
    base.z = 0.0;
    let mut next = 1;
    fn renumber(o: &mut Object, next: &mut u32) {
        o.id = *next;
        *next += 1;
        for c in &mut o.children {
            renumber(c, next);
        }
    }
    renumber(&mut base, &mut next);
    s.write_file(&rel, scene_data::write_prefab(&base).as_bytes())?;
    let keep = opt_bool(a, "trocar_pelo_prefab")?.unwrap_or(true);
    if keep {
        let o = doc.data.find_mut(id).expect("objeto");
        o.children.clear();
        o.prefab = Some(PrefabLink {
            path: rel.clone(),
            base: None,
            raw: Vec::new(),
        });
        o.link_prefab(base);
        s.save_doc(&doc)?;
    }
    Ok(Output::text(format!(
        "Criei {rel}.{} Ponha cópias com criar_objeto: {{ \"name\": ..., \"prefab\": \"{rel}\", \"x\": ..., \"y\": ... }}. Mudar o prefab (cena = \"{rel}\") muda todas as cópias.",
        if keep {
            format!(" \"{}\" virou uma cópia dele.", obj.name)
        } else {
            String::new()
        }
    )))
}

// listar_componentes, listar_comportamentos ------------------------------

fn listar_componentes(_: &mut Server, a: &Value) -> Result<Output, String> {
    let schema = scene_data::schema();
    let only = opt_str(a, "tipo")?;
    let mut out = String::from(
        "Componentes (\"type\" no arquivo). Os campos vêm com o valor padrão; numa cena 2D a unidade é o pixel, numa 3D o metro.\n",
    );
    let mut any = false;
    for kind in &schema.order {
        if only.is_some_and(|o| o != kind) {
            continue;
        }
        any = true;
        let (title, desc) = scene_data::component_info(kind);
        let mode = if valores::KINDS_3D.contains(&kind.as_str()) {
            " [3D]"
        } else if matches!(
            kind.as_str(),
            "sprite" | "shape" | "body" | "map" | "camera"
        ) {
            " [2D]"
        } else {
            ""
        };
        let fields: Vec<String> = schema
            .fields(kind)
            .unwrap_or(&[])
            .iter()
            .map(|(k, v)| format!("{k} = {}", v.to_lua()))
            .collect();
        out.push_str(&format!(
            "\n{kind}{mode} — {title}: {desc}\n  {}\n",
            fields.join(", ")
        ));
    }
    if !any {
        return Err(format!(
            "não há componente \"{}\". Componentes: {}",
            only.unwrap_or_default(),
            names(&schema.order)
        ));
    }
    out.push_str("\nComportamentos (\"type\": \"behavior\", \"script\": \"nn:...\") estão em listar_comportamentos.");
    Ok(Output::text(out))
}

fn spec_line(p: &PropSpec) -> String {
    let mut s = format!("{} = {}", p.key, p.default.to_lua());
    if let Some(d) = &p.default3d {
        s.push_str(&format!(" (3D: {})", d.to_lua()));
    }
    let mut extra = Vec::new();
    if !p.options.is_empty() {
        let opts: Vec<&str> = p.options.iter().map(|(k, _)| k.as_str()).collect();
        extra.push(format!("opções: {}", opts.join("|")));
    }
    if let Some(min) = p.min {
        extra.push(format!("mín {}", scene_data::format_num(min)));
    }
    if let Some(max) = p.max {
        extra.push(format!("máx {}", scene_data::format_num(max)));
    }
    if !p.mode.is_empty() && p.mode != "both" {
        extra.push(format!("só {}", p.mode.to_uppercase()));
    }
    if let Some((k, vals)) = &p.show_if {
        extra.push(format!("vale quando {k} = {}", vals.join("|")));
    }
    let kind = format!("{:?}", p.kind).to_lowercase();
    if !matches!(kind.as_str(), "number" | "bool" | "text") {
        extra.push(kind);
    }
    if !extra.is_empty() {
        s.push_str(&format!(" [{}]", extra.join("; ")));
    }
    let label = if p.label != p.key {
        p.label.as_str()
    } else {
        ""
    };
    match (label.is_empty(), p.help.is_empty()) {
        (true, true) => {}
        (false, true) => s.push_str(&format!(" — {label}")),
        (true, false) => s.push_str(&format!(" — {}", p.help)),
        (false, false) => s.push_str(&format!(" — {label}: {}", p.help)),
    }
    s
}

fn behavior_text(b: &BehaviorInfo, detail: bool) -> String {
    let mode = match b.mode.as_str() {
        "2d" => "2D",
        "3d" => "3D",
        _ => "2D e 3D",
    };
    let mut s = format!("{} — {} ({mode}", b.script, b.title);
    if !b.category.is_empty() {
        s.push_str(&format!(", {}", b.category));
    }
    s.push_str(&format!("): {}", b.description));
    if !b.requires.is_empty() {
        s.push_str(&format!(" Precisa de: {}.", b.requires.join(", ")));
    }
    if b.player {
        s.push_str(" É o jogador (os inimigos e itens procuram por ele).");
    }
    if detail {
        for p in &b.props {
            s.push_str(&format!("\n  {}", spec_line(p)));
        }
    } else if !b.props.is_empty() {
        let keys: Vec<&str> = b.props.iter().map(|p| p.key.as_str()).collect();
        s.push_str(&format!(" Propriedades: {}.", keys.join(", ")));
    }
    s
}

/// Scripts de comportamento do projeto: os usados nas cenas e os da pasta
/// `scripts/`.
fn project_behaviors(s: &mut Server) -> Vec<BehaviorInfo> {
    let files = crate::paths::list_files(&s.root, "");
    let mut scripts: Vec<String> = files
        .iter()
        .filter(|f| f.starts_with("scripts/") && f.ends_with(".lua"))
        .cloned()
        .collect();
    for f in files
        .iter()
        .filter(|f| f.ends_with(".cena") || f.ends_with(".prefab"))
    {
        if let Ok(doc) = s.open_doc(f) {
            scene_data::visit(&doc.data.objects, &mut |o| {
                for c in &o.components {
                    let sc = c.str("script");
                    if c.is_behavior() && !sc.starts_with("nn:") && !scripts.iter().any(|x| x == sc)
                    {
                        scripts.push(sc.to_string());
                    }
                }
            });
        }
    }
    let root = s.root.clone();
    scripts
        .into_iter()
        .map(|sc| s.catalog.get(&root, &sc))
        .collect()
}

fn listar_comportamentos(s: &mut Server, a: &Value) -> Result<Output, String> {
    if let Some(script) = opt_str(a, "script")? {
        let script = if script.starts_with("nn:") || script.ends_with(".lua") {
            script.to_string()
        } else {
            format!("nn:{script}")
        };
        let info = s.behavior(&script)?;
        return Ok(Output::text(format!(
            "{}\n\nNo objeto: {{ \"type\": \"behavior\", \"script\": \"{}\", ...propriedades }} (as que faltarem ficam com o padrão).",
            behavior_text(&info, true),
            info.script
        )));
    }
    let three_d = match opt_str(a, "tipo")? {
        Some("3d") => Some(true),
        Some("2d") => Some(false),
        _ => None,
    };
    let detail = opt_bool(a, "detalhes")?.unwrap_or(false);
    let mut out = String::from(
        "Comportamentos prontos: no objeto, { \"type\": \"behavior\", \"script\": \"nn:...\", ...propriedades }. Detalhes de um: listar_comportamentos com script.\n",
    );
    for cat in behaviors::categories() {
        let list: Vec<&BehaviorInfo> = behaviors::builtins()
            .iter()
            .filter(|b| b.category == cat && three_d.is_none_or(|t| b.fits(t)))
            .collect();
        if list.is_empty() {
            continue;
        }
        out.push_str(&format!("\n{cat}:\n"));
        for b in list {
            out.push_str(&format!("- {}\n", behavior_text(b, detail)));
        }
    }
    let mine = project_behaviors(s);
    out.push_str("\nScripts do jogo:\n");
    if mine.is_empty() {
        out.push_str("(nenhum ainda; um script de comportamento é um .lua em scripts/ — veja buscar_documentacao \"comportamentos\")\n");
    }
    for b in mine {
        match &b.error {
            Some(e) => out.push_str(&format!("- {}: ERRO: {e}\n", b.script)),
            None => out.push_str(&format!("- {}\n", behavior_text(&b, detail))),
        }
    }
    Ok(Output::text(out))
}

// As ferramentas -----------------------------------------------------------

const CENA: &str = "Arquivo da cena (.cena) ou do prefab (.prefab), relativo à pasta do projeto";

pub static TOOLS: &[&Tool] = &[
    &Tool {
        name: "ver_cena",
        title: "Ver cena",
        description: "Mostra uma cena (.cena) ou um prefab (.prefab) em JSON, no mesmo formato do arquivo: objetos com id, posição, componentes (\"type\" e campos), filhos e regras de eventos (com o número e a frase de cada uma). Por padrão só vêm os campos diferentes do padrão. O JSON de um objeto serve de modelo para criar_objeto.",
        params: || {
            vec![
                ("cena", s_str(CENA), true),
                ("objeto", s_ref("Só este objeto (id ou nome)"), false),
                (
                    "completo",
                    s_bool("Mostrar todos os campos, até os que estão no padrão"),
                    false,
                ),
            ]
        },
        read_only: true,
        destructive: false,
        run: ver_cena,
    },
    &Tool {
        name: "criar_cena",
        title: "Criar cena",
        description: "Cria uma cena visual nova (.cena). 2D: vazia. 3D: com câmera e chão. Sem \"tipo\", segue o projeto (2D ou 3D).",
        params: || {
            vec![
                (
                    "caminho",
                    s_str("Arquivo novo, ex.: cenas/fase2.cena"),
                    true,
                ),
                ("tipo", s_enum("2D ou 3D", &["2d", "3d"]), false),
                ("background", s_str("Cor do fundo, \"#RRGGBB\""), false),
                (
                    "por_cima",
                    s_bool("Abre por cima de outra (pausa, menu): fundo transparente"),
                    false,
                ),
                (
                    "inicial",
                    s_bool("Vira a cena inicial do jogo (start no config.lua)"),
                    false,
                ),
            ]
        },
        read_only: false,
        destructive: false,
        run: criar_cena,
    },
    &Tool {
        name: "mudar_cena",
        title: "Mudar cena",
        description: "Muda os campos da própria cena: cor do fundo, gravidade, abrir por cima, o script da cena e, no 3D, o ambiente (céu, sol, névoa, sombras: veja ver_cena com completo = true).",
        params: || {
            vec![
                ("cena", s_str(CENA), true),
                ("background", s_str("Cor do fundo, \"#RRGGBB\""), false),
                (
                    "gravity",
                    s_num_desc("Gravidade (2D: pixels/s², padrão -1800; 3D: m/s², padrão -20)"),
                    false,
                ),
                ("por_cima", s_bool("Abre por cima de outra cena"), false),
                ("script", s_str("Script Lua da cena (\"\" tira)"), false),
                (
                    "environment",
                    s_obj(
                        "3D: campos do ambiente a mudar, { \"sky_top\": \"#...\", \"fog\": true, ... }",
                    ),
                    false,
                ),
            ]
        },
        read_only: false,
        destructive: false,
        run: mudar_cena,
    },
    &Tool {
        name: "criar_objeto",
        title: "Criar objeto",
        description: "Põe um ou mais objetos numa cena ou prefab. O objeto vai no formato do arquivo: { \"name\": \"heroi\", \"x\": 0, \"y\": -180, \"layer\": 2, \"components\": [ { \"type\": \"sprite\", \"image\": \"assets/imagens/heroi.png\", \"w\": 64, \"h\": 64 }, { \"type\": \"behavior\", \"script\": \"nn:plataforma\", \"velocidade\": 320 } ], \"children\": [...] }. Para uma cópia de prefab: { \"name\": \"moeda2\", \"prefab\": \"prefabs/moeda.prefab\", \"x\": 100, \"y\": 0 }. Campos que faltam ficam com o padrão; cada campo é conferido (listar_componentes, listar_comportamentos). Comportamentos que precisam de corpo físico já ganham um. 2D: pixels, y para cima, (0,0) no meio da tela. 3D: metros, y é a altura.",
        params: || {
            vec![
                ("cena", s_str(CENA), true),
                ("objeto", s_obj("O objeto (com filhos, se quiser)"), false),
                (
                    "objetos",
                    s_list("Vários objetos de uma vez", json!({ "type": "object" })),
                    false,
                ),
                (
                    "pai",
                    s_ref("Dentro de qual objeto (id ou nome); sem pai, vai na raiz"),
                    false,
                ),
                (
                    "posicao",
                    s_int(
                        "Posição na lista (0 = primeiro, desenha atrás na mesma camada); sem, vai no fim",
                    ),
                    false,
                ),
            ]
        },
        read_only: false,
        destructive: false,
        run: criar_objeto,
    },
    &Tool {
        name: "mudar_objeto",
        title: "Mudar objeto",
        description: "Muda um objeto: campos do próprio objeto em \"mudar\" ({ \"x\": 10, \"name\": \"chefe\", \"visible\": false }); componentes em \"componentes\" (cada um com \"type\": se o objeto já tem, muda só os campos passados; se não tem, põe um novo; null volta ao padrão); \"tirar_componentes\" (tipos: [\"body\", \"nn:vida\"]); e \"pai\" para mudar de lugar na árvore (mantém a posição no mundo; null = raiz). Numa cópia de prefab só dá para mudar valores.",
        params: || {
            vec![
                ("cena", s_str(CENA), true),
                ("objeto", s_ref("O objeto (id ou nome)"), true),
                (
                    "mudar",
                    s_obj(
                        "Campos do objeto: name, x, y, z, rotation, rx, ry, rz, scale_x, scale_y, scale_z, layer, visible, locked, anchor",
                    ),
                    false,
                ),
                (
                    "componentes",
                    s_list(
                        "Componentes a mudar ou pôr: { \"type\": ..., campos }",
                        json!({ "type": "object" }),
                    ),
                    false,
                ),
                (
                    "tirar_componentes",
                    s_list("Tipos dos componentes a tirar", json!({ "type": "string" })),
                    false,
                ),
                (
                    "pai",
                    s_ref("Novo pai (id ou nome; null ou \"\" = raiz)"),
                    false,
                ),
                ("posicao", s_int("Posição na lista do pai"), false),
            ]
        },
        read_only: false,
        destructive: false,
        run: mudar_objeto,
    },
    &Tool {
        name: "apagar_objeto",
        title: "Apagar objeto",
        description: "Tira um objeto (e os filhos dele) da cena ou do prefab.",
        params: || {
            vec![
                ("cena", s_str(CENA), true),
                ("objeto", s_ref("O objeto (id ou nome)"), true),
            ]
        },
        read_only: false,
        destructive: true,
        run: apagar_objeto,
    },
    &Tool {
        name: "copiar_objeto",
        title: "Copiar objeto",
        description: "Copia um objeto (com os filhos) logo depois dele. Com \"copias\" e \"passo_x\"/\"passo_y\"/\"passo_z\", faz uma fileira (moedas, cercas, plataformas). \"mudar\" muda campos de cada cópia.",
        params: || {
            vec![
                ("cena", s_str(CENA), true),
                ("objeto", s_ref("O objeto (id ou nome)"), true),
                ("copias", s_int("Quantas cópias (padrão 1)"), false),
                (
                    "passo_x",
                    s_num_desc("Cada cópia anda isto em x a partir da anterior"),
                    false,
                ),
                ("passo_y", s_num_desc("... em y"), false),
                ("passo_z", s_num_desc("... em z (3D)"), false),
                ("mudar", s_obj("Campos a mudar em cada cópia"), false),
            ]
        },
        read_only: false,
        destructive: false,
        run: copiar_objeto,
    },
    &Tool {
        name: "criar_prefab",
        title: "Criar prefab",
        description: "Transforma um objeto de uma cena num prefab (.prefab): um molde para pôr cópias em várias cenas e no jogo (ação \"criar\" dos eventos, nn.scene). O objeto vira uma cópia do prefab.",
        params: || {
            vec![
                ("cena", s_str("A cena (.cena) onde o objeto está"), true),
                ("objeto", s_ref("O objeto (id ou nome)"), true),
                (
                    "caminho",
                    s_str("Arquivo do prefab (padrão: prefabs/<nome>.prefab)"),
                    false,
                ),
                (
                    "trocar_pelo_prefab",
                    s_bool("O objeto vira cópia do prefab (padrão true)"),
                    false,
                ),
            ]
        },
        read_only: false,
        destructive: false,
        run: criar_prefab,
    },
    &Tool {
        name: "listar_componentes",
        title: "Listar componentes",
        description: "Os componentes que um objeto pode ter (sprite, shape, text, body, camera, particles, button, model, body3d...) com todos os campos e valores padrão.",
        params: || vec![("tipo", s_str("Só este componente"), false)],
        read_only: true,
        destructive: false,
        run: listar_componentes,
    },
    &Tool {
        name: "listar_comportamentos",
        title: "Listar comportamentos",
        description: "Os comportamentos prontos da engine (jogador de plataforma, inimigo que patrulha, coletável, vida, porta, Jogador 3D...) e os scripts de comportamento do projeto. Com \"script\", mostra todas as propriedades de um, com padrão, opções e ajuda.",
        params: || {
            vec![
                (
                    "script",
                    s_str(
                        "Um comportamento: \"nn:plataforma\" (ou só \"plataforma\") ou \"scripts/x.lua\"",
                    ),
                    false,
                ),
                (
                    "tipo",
                    s_enum("Só os que funcionam em cenas 2D ou 3D", &["2d", "3d"]),
                    false,
                ),
                (
                    "detalhes",
                    s_bool("Todas as propriedades de todos (longo)"),
                    false,
                ),
            ]
        },
        read_only: true,
        destructive: false,
        run: listar_comportamentos,
    },
];

fn s_num_desc(desc: &str) -> Value {
    super::s_num(desc)
}
