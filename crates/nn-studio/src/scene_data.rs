//! Cena visual (`.cena`) do jeito que o editor a enxerga: objetos com
//! posição, rotação, escala, componentes e filhos. Lê e grava o arquivo em
//! tabela Lua legível, um objeto por bloco e um componente por linha.
//!
//! Os componentes e seus campos padrão vêm do próprio módulo da engine que
//! roda as cenas (`nn_script::SCENE_RUNTIME`), então editor e jogo nunca
//! discordam sobre um padrão.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use mlua::{Lua, Table, Value};

/// Valor de um campo de componente.
#[derive(Debug, Clone, PartialEq)]
pub enum Prop {
    Num(f64),
    Bool(bool),
    Str(String),
}

impl Prop {
    pub fn num(&self) -> f64 {
        match self {
            Prop::Num(n) => *n,
            Prop::Bool(b) => f64::from(u8::from(*b)),
            Prop::Str(s) => s.parse().unwrap_or(0.0),
        }
    }

    pub fn bool(&self) -> bool {
        match self {
            Prop::Bool(b) => *b,
            Prop::Num(n) => *n != 0.0,
            Prop::Str(s) => !s.is_empty(),
        }
    }

    pub fn str(&self) -> &str {
        match self {
            Prop::Str(s) => s,
            _ => "",
        }
    }

    /// Como o valor aparece no arquivo.
    pub fn to_lua(&self) -> String {
        match self {
            Prop::Num(n) => format_num(*n),
            Prop::Bool(b) => b.to_string(),
            Prop::Str(s) => crate::config_lua::lua_string(s),
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Prop::Num(n) if n.fract() == 0.0 && n.abs() < 1e15 => serde_json::json!(*n as i64),
            Prop::Num(n) => serde_json::json!(n),
            Prop::Bool(b) => serde_json::json!(b),
            Prop::Str(s) => serde_json::json!(s),
        }
    }
}

/// Número sem ".0" quando é inteiro, e com até 4 casas quando não é.
pub fn format_num(n: f64) -> String {
    if !n.is_finite() {
        return "0".into();
    }
    if n.fract() == 0.0 && n.abs() < 1e15 {
        return format!("{}", n as i64);
    }
    let s = format!("{n:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.to_string() }
}

/// Tipo do componente de comportamento (script com propriedades).
pub const BEHAVIOR: &str = "behavior";

#[derive(Debug, Clone, PartialEq)]
pub struct Component {
    pub kind: String,
    /// Campos na ordem do arquivo. Num comportamento: `script` e depois as
    /// propriedades em ordem alfabética.
    pub props: Vec<(String, Prop)>,
}

impl Component {
    /// Componente novo, com os valores padrão.
    pub fn new(kind: &str) -> Self {
        if kind == BEHAVIOR {
            return Self::behavior("", &[]);
        }
        let props = schema()
            .fields(kind)
            .map(|f| f.to_vec())
            .unwrap_or_default();
        Self {
            kind: kind.to_string(),
            props,
        }
    }

    /// Comportamento com o script e os valores das propriedades.
    pub fn behavior(script: &str, props: &[(String, Prop)]) -> Self {
        let mut c = Self {
            kind: BEHAVIOR.to_string(),
            props: vec![("script".to_string(), Prop::Str(script.to_string()))],
        };
        for (k, v) in props {
            c.set(k, v.clone());
        }
        c
    }

    pub fn is_behavior(&self) -> bool {
        self.kind == BEHAVIOR
    }

    /// Mantém `script` primeiro e o resto em ordem alfabética (num
    /// comportamento a ordem não importa, e assim o arquivo fica estável).
    fn normalize(&mut self) {
        if self.is_behavior() {
            self.props.sort_by(|(a, _), (b, _)| {
                (a != "script").cmp(&(b != "script")).then_with(|| a.cmp(b))
            });
        }
    }

    /// Tira um campo (propriedade que o script não tem mais).
    pub fn remove(&mut self, key: &str) {
        self.props.retain(|(k, _)| k != key);
    }

    pub fn get(&self, key: &str) -> Option<&Prop> {
        self.props.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn num(&self, key: &str) -> f64 {
        self.get(key).map(Prop::num).unwrap_or(0.0)
    }

    pub fn bool(&self, key: &str) -> bool {
        self.get(key).is_some_and(Prop::bool)
    }

    pub fn str(&self, key: &str) -> &str {
        self.get(key).map(Prop::str).unwrap_or("")
    }

    pub fn set(&mut self, key: &str, value: Prop) {
        match self.props.iter_mut().find(|(k, _)| k == key) {
            Some((_, v)) => *v = value,
            None => {
                self.props.push((key.to_string(), value));
                self.normalize();
            }
        }
    }

    /// Algum campo `hud` ligado (preso à tela)?
    pub fn is_hud(&self) -> bool {
        matches!(self.kind.as_str(), "sprite" | "text" | "shape") && self.bool("hud")
    }
}

/// Ligação de uma instância com o prefab.
#[derive(Debug, Clone, PartialEq)]
pub struct PrefabLink {
    /// Arquivo `.prefab`.
    pub path: String,
    /// O objeto do prefab como está no disco (para saber o que a instância
    /// mudou e para desenhar os filhos dele). `None` enquanto não foi lido ou
    /// se o arquivo sumiu.
    pub base: Option<Box<Object>>,
    /// Mudanças lidas do arquivo, guardadas enquanto o prefab não foi lido.
    pub raw: Vec<(usize, Vec<(String, Prop)>)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub id: u32,
    pub name: String,
    pub x: f64,
    pub y: f64,
    /// Graus, sentido anti-horário (Y para cima).
    pub rotation: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub visible: bool,
    /// Travado no editor (não seleciona clicando na cena).
    pub locked: bool,
    pub layer: i64,
    /// Numa instância de prefab, os componentes já com as mudanças dela.
    pub components: Vec<Component>,
    /// Filhos do próprio objeto (numa instância, além dos do prefab).
    pub children: Vec<Object>,
    /// Instância de um prefab.
    pub prefab: Option<PrefabLink>,
}

impl Object {
    pub fn new(id: u32, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            visible: true,
            locked: false,
            layer: 0,
            components: Vec::new(),
            children: Vec::new(),
            prefab: None,
        }
    }

    pub fn component(&self, kind: &str) -> Option<&Component> {
        self.components.iter().find(|c| c.kind == kind)
    }

    /// Preso à tela (HUD): algum componente visual com `hud`.
    pub fn is_hud(&self) -> bool {
        self.components.iter().any(Component::is_hud)
    }

    /// Filhos que vêm do prefab (só para desenhar; não são editados aqui).
    pub fn prefab_children(&self) -> &[Object] {
        match &self.prefab {
            Some(PrefabLink {
                base: Some(base), ..
            }) => &base.children,
            _ => &[],
        }
    }

    /// O que a instância mudou em relação ao prefab: (componente, campos).
    pub fn overrides(&self) -> Vec<(usize, Vec<(String, Prop)>)> {
        let Some(link) = &self.prefab else {
            return Vec::new();
        };
        let Some(base) = &link.base else {
            return link.raw.clone();
        };
        let mut out = Vec::new();
        for (i, c) in self.components.iter().enumerate() {
            let Some(b) = base.components.get(i) else {
                continue;
            };
            let changed: Vec<(String, Prop)> = c
                .props
                .iter()
                .filter(|(k, v)| b.get(k) != Some(v))
                .cloned()
                .collect();
            if !changed.is_empty() {
                out.push((i, changed));
            }
        }
        out
    }

    /// Campo `key` do componente `index` difere do prefab?
    pub fn is_overridden(&self, index: usize, key: &str) -> bool {
        let Some(PrefabLink {
            base: Some(base), ..
        }) = &self.prefab
        else {
            return false;
        };
        let mine = self.components.get(index).and_then(|c| c.get(key));
        let theirs = base.components.get(index).and_then(|c| c.get(key));
        mine != theirs
    }

    /// Liga (ou religa) a instância ao prefab `base`: os componentes passam a
    /// ser os do prefab, com as mudanças que a instância já tinha.
    pub fn link_prefab(&mut self, base: Object) {
        let overrides = self.overrides();
        let mut comps = base.components.clone();
        for (i, fields) in overrides {
            if let Some(c) = comps.get_mut(i) {
                for (k, v) in fields {
                    c.set(&k, v);
                }
            }
        }
        self.components = comps;
        if let Some(link) = &mut self.prefab {
            link.base = Some(Box::new(base));
            link.raw.clear();
        }
    }

    /// Valor de um campo do objeto (para o ajuste ao vivo).
    pub fn field(&self, key: &str) -> Option<Prop> {
        Some(match key {
            "name" => Prop::Str(self.name.clone()),
            "x" => Prop::Num(self.x),
            "y" => Prop::Num(self.y),
            "rotation" => Prop::Num(self.rotation),
            "scale_x" => Prop::Num(self.scale_x),
            "scale_y" => Prop::Num(self.scale_y),
            "visible" => Prop::Bool(self.visible),
            "locked" => Prop::Bool(self.locked),
            "layer" => Prop::Num(self.layer as f64),
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneData {
    pub script: String,
    pub background: String,
    pub gravity: f64,
    pub overlay: bool,
    pub objects: Vec<Object>,
}

impl Default for SceneData {
    fn default() -> Self {
        Self {
            script: String::new(),
            background: String::new(),
            gravity: -1800.0,
            overlay: false,
            objects: Vec::new(),
        }
    }
}

/// Os componentes que existem e os campos de cada um.
#[derive(Debug)]
pub struct Schema {
    components: BTreeMap<String, Vec<(String, Prop)>>,
    /// Ordem para mostrar no "Adicionar componente".
    pub order: Vec<String>,
}

impl Schema {
    pub fn fields(&self, kind: &str) -> Option<&[(String, Prop)]> {
        self.components.get(kind).map(Vec::as_slice)
    }

    pub fn default_of(&self, kind: &str, key: &str) -> Option<&Prop> {
        self.fields(kind)?
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }
}

/// O esquema dos componentes, lido do módulo da engine.
pub fn schema() -> &'static Schema {
    static SCHEMA: OnceLock<Schema> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        let lua = Lua::new();
        let module: Table = lua
            .load(nn_script::SCENE_RUNTIME)
            .eval()
            .expect("módulo das cenas da engine");
        let fields: Table = module.get("FIELDS").expect("FIELDS");
        let mut components = BTreeMap::new();
        for pair in fields.pairs::<String, Table>() {
            let (kind, list) = pair.expect("componente");
            let mut props = Vec::new();
            for f in list.sequence_values::<Table>() {
                let f = f.expect("campo");
                let key: String = f.get(1).expect("nome");
                let value = to_prop(&f.get::<Value>(2).expect("padrão")).expect("valor simples");
                props.push((key, value));
            }
            components.insert(kind, props);
        }
        let order = [
            "sprite",
            "shape",
            "text",
            "body",
            "map",
            "camera",
            "sound",
            "particles",
        ]
        .into_iter()
        .filter(|k| components.contains_key(*k))
        .map(str::to_string)
        .collect();
        Schema { components, order }
    })
}

fn to_prop(v: &Value) -> Option<Prop> {
    Some(match v {
        Value::Integer(i) => Prop::Num(*i as f64),
        Value::Number(n) => Prop::Num(*n),
        Value::Boolean(b) => Prop::Bool(*b),
        Value::String(s) => Prop::Str(s.to_string_lossy()),
        _ => return None,
    })
}

fn num(t: &Table, key: &str, default: f64) -> f64 {
    match t.get::<Value>(key) {
        Ok(Value::Integer(i)) => i as f64,
        Ok(Value::Number(n)) => n,
        _ => default,
    }
}

fn boolean(t: &Table, key: &str, default: bool) -> bool {
    match t.get::<Value>(key) {
        Ok(Value::Boolean(b)) => b,
        _ => default,
    }
}

fn string(t: &Table, key: &str) -> String {
    match t.get::<Value>(key) {
        Ok(Value::String(s)) => s.to_string_lossy(),
        _ => String::new(),
    }
}

/// Lê um `.cena`. Ids que faltam ou se repetem são corrigidos.
pub fn read(text: &str, name: &str) -> Result<SceneData, String> {
    let lua = Lua::new();
    let value: Value = lua
        .load(text)
        .set_name(format!("@{name}"))
        .eval()
        .map_err(|e| nn_script::translate_error(e.to_string().trim()))?;
    let Value::Table(t) = value else {
        return Err(format!(
            "{name}: o arquivo da cena deve devolver uma tabela"
        ));
    };
    let mut scene = SceneData {
        script: string(&t, "script"),
        background: string(&t, "background"),
        gravity: num(&t, "gravity", -1800.0),
        overlay: boolean(&t, "overlay", false),
        objects: Vec::new(),
    };
    if let Ok(Value::Table(list)) = t.get::<Value>("objects") {
        scene.objects = read_objects(&list, name)?;
    }
    fix_ids(&mut scene);
    Ok(scene)
}

fn read_component(c: &Table, name: &str, obj: &str) -> Result<Component, String> {
    let kind = string(c, "type");
    if kind == BEHAVIOR {
        let mut comp = Component::behavior(&string(c, "script"), &[]);
        for pair in c.pairs::<String, Value>() {
            let Ok((k, v)) = pair else { continue };
            if k == "type" || k == "script" {
                continue;
            }
            if let Some(p) = to_prop(&v) {
                comp.set(&k, p);
            }
        }
        return Ok(comp);
    }
    let Some(fields) = schema().fields(&kind) else {
        return Err(format!(
            "{name}: componente desconhecido \"{kind}\" no objeto \"{obj}\""
        ));
    };
    let mut comp = Component {
        kind: kind.clone(),
        props: Vec::new(),
    };
    for (key, default) in fields {
        let v = c
            .get::<Value>(key.as_str())
            .ok()
            .and_then(|v| to_prop(&v))
            .unwrap_or_else(|| default.clone());
        comp.props.push((key.clone(), v));
    }
    Ok(comp)
}

fn read_objects(list: &Table, name: &str) -> Result<Vec<Object>, String> {
    let mut out = Vec::new();
    for o in list.sequence_values::<Table>() {
        let o = o.map_err(|e| format!("{name}: objeto inválido: {e}"))?;
        out.push(read_object(&o, name)?);
    }
    Ok(out)
}

fn read_object(o: &Table, name: &str) -> Result<Object, String> {
    let mut obj = Object::new(num(o, "id", 0.0).max(0.0) as u32, "Objeto");
    let n = string(o, "name");
    if !n.is_empty() {
        obj.name = n;
    }
    obj.x = num(o, "x", 0.0);
    obj.y = num(o, "y", 0.0);
    obj.rotation = num(o, "rotation", 0.0);
    obj.scale_x = num(o, "scale_x", 1.0);
    obj.scale_y = num(o, "scale_y", 1.0);
    obj.visible = boolean(o, "visible", true);
    obj.locked = boolean(o, "locked", false);
    obj.layer = num(o, "layer", 0.0) as i64;
    let prefab = string(o, "prefab");
    if !prefab.is_empty() {
        let mut raw = Vec::new();
        if let Ok(Value::Table(list)) = o.get::<Value>("overrides") {
            for ov in list.sequence_values::<Table>() {
                let ov = ov.map_err(|e| format!("{name}: mudança inválida: {e}"))?;
                let index = num(&ov, "component", 0.0) as usize;
                if index == 0 {
                    continue;
                }
                let mut fields: Vec<(String, Prop)> = ov
                    .pairs::<String, Value>()
                    .filter_map(Result::ok)
                    .filter(|(k, _)| k != "component")
                    .filter_map(|(k, v)| to_prop(&v).map(|p| (k, p)))
                    .collect();
                fields.sort_by(|a, b| a.0.cmp(&b.0));
                raw.push((index - 1, fields));
            }
        }
        obj.prefab = Some(PrefabLink {
            path: prefab,
            base: None,
            raw,
        });
    } else if let Ok(Value::Table(comps)) = o.get::<Value>("components") {
        for c in comps.sequence_values::<Table>() {
            let c = c.map_err(|e| format!("{name}: componente inválido: {e}"))?;
            obj.components.push(read_component(&c, name, &obj.name)?);
        }
    }
    if let Ok(Value::Table(children)) = o.get::<Value>("children") {
        obj.children = read_objects(&children, name)?;
    }
    Ok(obj)
}

/// Lê um `.prefab`: um objeto (com filhos).
pub fn read_prefab(text: &str, name: &str) -> Result<Object, String> {
    let lua = Lua::new();
    let value: Value = lua
        .load(text)
        .set_name(format!("@{name}"))
        .eval()
        .map_err(|e| nn_script::translate_error(e.to_string().trim()))?;
    let Value::Table(t) = value else {
        return Err(format!("{name}: o prefab deve devolver uma tabela"));
    };
    let Ok(Value::Table(o)) = t.get::<Value>("object") else {
        return Err(format!("{name}: o prefab precisa de `object = {{ ... }}`"));
    };
    let mut obj = read_object(&o, name)?;
    let mut scene = SceneData {
        objects: vec![obj],
        ..Default::default()
    };
    fix_ids(&mut scene);
    obj = scene.objects.remove(0);
    Ok(obj)
}

/// Lê um prefab do disco, já com os prefabs que ele usa ligados.
pub fn load_prefab(root: &std::path::Path, rel: &str) -> Result<Object, String> {
    load_prefab_at(root, rel, 0)
}

fn load_prefab_at(root: &std::path::Path, rel: &str, depth: usize) -> Result<Object, String> {
    if depth > 8 {
        return Err(format!("{rel}: prefabs demais um dentro do outro"));
    }
    let text = std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
    let mut obj = read_prefab(&text, rel)?;
    link_tree(std::slice::from_mut(&mut obj), root, depth + 1);
    Ok(obj)
}

fn link_tree(list: &mut [Object], root: &std::path::Path, depth: usize) {
    for o in list {
        if let Some(link) = &o.prefab
            && let Ok(base) = load_prefab_at(root, &link.path.clone(), depth)
        {
            o.link_prefab(base);
        }
        link_tree(&mut o.children, root, depth);
    }
}

/// Grava um `.prefab`.
pub fn write_prefab(obj: &Object) -> String {
    let mut s = String::from(
        "-- Prefab do NN Studio: um objeto pronto para usar em várias cenas.\n\
         -- Dá para editar à mão; ao salvar pelo editor, comentários se perdem.\n\
         return {\n  format = 1,\n  object = ",
    );
    let mut body = String::new();
    write_objects(&mut body, std::slice::from_ref(obj), 1);
    s.push_str(body.trim_start());
    s.push_str("}\n");
    s
}

/// Garante ids únicos e maiores que zero.
pub fn fix_ids(scene: &mut SceneData) {
    let mut seen = std::collections::HashSet::new();
    let mut max = 0;
    visit(&scene.objects, &mut |o| max = max.max(o.id));
    fn walk(list: &mut [Object], seen: &mut std::collections::HashSet<u32>, max: &mut u32) {
        for o in list {
            if o.id == 0 || !seen.insert(o.id) {
                *max += 1;
                o.id = *max;
                seen.insert(o.id);
            }
            walk(&mut o.children, seen, max);
        }
    }
    walk(&mut scene.objects, &mut seen, &mut max);
}

/// Grava a cena como tabela Lua.
pub fn write(scene: &SceneData) -> String {
    let mut s = String::from(
        "-- Cena visual do NN Studio. Dá para editar à mão; ao salvar pelo editor,\n\
         -- comentários escritos aqui se perdem.\n\
         return {\n  format = 1,\n",
    );
    if !scene.script.is_empty() {
        s.push_str(&format!(
            "  script = {},\n",
            crate::config_lua::lua_string(&scene.script)
        ));
    }
    if !scene.background.is_empty() {
        s.push_str(&format!(
            "  background = {},\n",
            crate::config_lua::lua_string(&scene.background)
        ));
    }
    s.push_str(&format!("  gravity = {},\n", format_num(scene.gravity)));
    if scene.overlay {
        s.push_str("  overlay = true,\n");
    }
    s.push_str("  objects = {\n");
    write_objects(&mut s, &scene.objects, 2);
    s.push_str("  },\n}\n");
    s
}

fn write_objects(s: &mut String, list: &[Object], depth: usize) {
    let pad = "  ".repeat(depth);
    for o in list {
        s.push_str(&format!("{pad}{{\n"));
        let inner = "  ".repeat(depth + 1);
        s.push_str(&format!(
            "{inner}id = {}, name = {},\n",
            o.id,
            crate::config_lua::lua_string(&o.name)
        ));
        s.push_str(&format!(
            "{inner}x = {}, y = {}, rotation = {}, scale_x = {}, scale_y = {},\n",
            format_num(o.x),
            format_num(o.y),
            format_num(o.rotation),
            format_num(o.scale_x),
            format_num(o.scale_y)
        ));
        s.push_str(&format!(
            "{inner}visible = {}, layer = {},",
            o.visible, o.layer
        ));
        if o.locked {
            s.push_str(" locked = true,");
        }
        s.push('\n');
        if let Some(link) = &o.prefab {
            s.push_str(&format!(
                "{inner}prefab = {},\n",
                crate::config_lua::lua_string(&link.path)
            ));
            let overrides = o.overrides();
            if !overrides.is_empty() {
                s.push_str(&format!("{inner}overrides = {{\n"));
                for (i, fields) in overrides {
                    let mut line = format!("component = {}", i + 1);
                    for (k, v) in fields {
                        line.push_str(&format!(", {k} = {}", v.to_lua()));
                    }
                    s.push_str(&format!("{inner}  {{ {line} }},\n"));
                }
                s.push_str(&format!("{inner}}},\n"));
            }
        } else if !o.components.is_empty() {
            s.push_str(&format!("{inner}components = {{\n"));
            for c in &o.components {
                let mut line = format!("type = {}", crate::config_lua::lua_string(&c.kind));
                for (k, v) in &c.props {
                    line.push_str(&format!(", {k} = {}", v.to_lua()));
                }
                s.push_str(&format!("{inner}  {{ {line} }},\n"));
            }
            s.push_str(&format!("{inner}}},\n"));
        }
        if !o.children.is_empty() {
            s.push_str(&format!("{inner}children = {{\n"));
            write_objects(s, &o.children, depth + 2);
            s.push_str(&format!("{inner}}},\n"));
        }
        s.push_str(&format!("{pad}}},\n"));
    }
}

/// Chama `f` para todos os objetos, em ordem (pais antes dos filhos).
pub fn visit(list: &[Object], f: &mut impl FnMut(&Object)) {
    for o in list {
        f(o);
        visit(&o.children, f);
    }
}

impl SceneData {
    /// Liga (ou religa, depois que um prefab mudou) as instâncias aos
    /// prefabs do disco, mantendo o que cada instância mudou.
    pub fn link_prefabs(&mut self, root: &std::path::Path) {
        link_tree(&mut self.objects, root, 0);
    }

    /// Tem alguma instância deste prefab?
    pub fn uses_prefab(&self, rel: &str) -> bool {
        let mut found = false;
        visit(&self.objects, &mut |o| {
            found |= o.prefab.as_ref().is_some_and(|l| l.path == rel);
        });
        found
    }

    pub fn next_id(&self) -> u32 {
        let mut max = 0;
        visit(&self.objects, &mut |o| max = max.max(o.id));
        max + 1
    }

    pub fn find(&self, id: u32) -> Option<&Object> {
        fn go(list: &[Object], id: u32) -> Option<&Object> {
            for o in list {
                if o.id == id {
                    return Some(o);
                }
                if let Some(f) = go(&o.children, id) {
                    return Some(f);
                }
            }
            None
        }
        go(&self.objects, id)
    }

    pub fn find_mut(&mut self, id: u32) -> Option<&mut Object> {
        fn go(list: &mut [Object], id: u32) -> Option<&mut Object> {
            for o in list {
                if o.id == id {
                    return Some(o);
                }
                if let Some(f) = go(&mut o.children, id) {
                    return Some(f);
                }
            }
            None
        }
        go(&mut self.objects, id)
    }

    /// Pai de um objeto (`Some(None)` = está na raiz).
    pub fn parent_of(&self, id: u32) -> Option<Option<u32>> {
        fn go(list: &[Object], id: u32, parent: Option<u32>) -> Option<Option<u32>> {
            for o in list {
                if o.id == id {
                    return Some(parent);
                }
                if let Some(p) = go(&o.children, id, Some(o.id)) {
                    return Some(p);
                }
            }
            None
        }
        go(&self.objects, id, None)
    }

    /// `ancestor` é pai, avô... de `id`?
    pub fn is_ancestor(&self, ancestor: u32, id: u32) -> bool {
        let mut cur = self.parent_of(id).flatten();
        while let Some(p) = cur {
            if p == ancestor {
                return true;
            }
            cur = self.parent_of(p).flatten();
        }
        false
    }

    fn list_mut(&mut self, parent: Option<u32>) -> Option<&mut Vec<Object>> {
        match parent {
            None => Some(&mut self.objects),
            Some(p) => self.find_mut(p).map(|o| &mut o.children),
        }
    }

    /// Tira um objeto (com os filhos). Devolve o objeto, o pai e a posição.
    pub fn remove(&mut self, id: u32) -> Option<(Object, Option<u32>, usize)> {
        let parent = self.parent_of(id)?;
        let list = self.list_mut(parent)?;
        let index = list.iter().position(|o| o.id == id)?;
        Some((list.remove(index), parent, index))
    }

    /// Põe um objeto como filho de `parent` (ou na raiz), na posição `index`.
    pub fn insert(&mut self, parent: Option<u32>, index: usize, obj: Object) {
        if let Some(list) = self.list_mut(parent) {
            let i = index.min(list.len());
            list.insert(i, obj);
        } else {
            self.objects.push(obj);
        }
    }

    /// Muda um objeto de lugar na árvore. Não deixa um objeto virar filho
    /// dele mesmo. Mantém a posição no mundo.
    pub fn move_object(&mut self, id: u32, parent: Option<u32>, index: usize) -> bool {
        if parent == Some(id) || parent.is_some_and(|p| self.is_ancestor(id, p)) {
            return false;
        }
        let world = self.world_transform(id);
        let Some((mut obj, old_parent, old_index)) = self.remove(id) else {
            return false;
        };
        // Dentro da mesma lista, a posição anda um para trás depois de tirar.
        let index = if old_parent == parent && index > old_index {
            index - 1
        } else {
            index
        };
        if let Some(w) = world {
            let local = match parent.and_then(|p| self.world_transform(p)) {
                Some(pw) => pw.to_local(w),
                None => self.root_unplace(&obj, w),
            };
            obj.x = local.x;
            obj.y = local.y;
            obj.rotation = local.rotation;
            obj.scale_x = local.scale_x;
            obj.scale_y = local.scale_y;
        }
        self.insert(parent, index, obj);
        true
    }

    /// Copia o objeto (com filhos e ids novos) logo depois do original.
    pub fn duplicate(&mut self, id: u32) -> Option<u32> {
        let parent = self.parent_of(id)?;
        let obj = self.find(id)?.clone();
        let mut next = self.next_id();
        fn renumber(o: &mut Object, next: &mut u32) {
            o.id = *next;
            *next += 1;
            for c in &mut o.children {
                renumber(c, next);
            }
        }
        let mut copy = obj;
        renumber(&mut copy, &mut next);
        copy.name = copy_name(&copy.name);
        let new_id = copy.id;
        let index = self
            .list_mut(parent)
            .and_then(|l| l.iter().position(|o| o.id == id))
            .map(|i| i + 1)
            .unwrap_or(0);
        self.insert(parent, index, copy);
        Some(new_id)
    }

    /// Posição, rotação e escala no mundo. Objetos presos à tela (HUD)
    /// aparecem no editor dentro da moldura da tela do jogo.
    pub fn world_transform(&self, id: u32) -> Option<Transform> {
        let o = self.find(id)?;
        let local = Transform::of(o);
        match self.parent_of(id)? {
            None => Some(self.root_place(o, local)),
            Some(p) => Some(self.world_transform(p)?.apply(local)),
        }
    }

    /// Como [`SceneData::world_transform`], sem o ajuste do HUD.
    fn plain_world_transform(&self, id: u32) -> Option<Transform> {
        let o = self.find(id)?;
        let local = Transform::of(o);
        match self.parent_of(id)? {
            None => Some(local),
            Some(p) => Some(self.plain_world_transform(p)?.apply(local)),
        }
    }

    /// Onde fica um objeto da raiz no mundo do editor.
    pub fn root_place(&self, o: &Object, local: Transform) -> Transform {
        if !o.is_hud() {
            return local;
        }
        // No jogo, o HUD fica onde estaria com a câmera no começo, sem zoom.
        let (cx, cy, zoom) = self.camera_start();
        Transform {
            x: cx + local.x / zoom,
            y: cy + local.y / zoom,
            rotation: local.rotation,
            scale_x: local.scale_x / zoom,
            scale_y: local.scale_y / zoom,
        }
    }

    /// O contrário de [`SceneData::root_place`].
    pub fn root_unplace(&self, o: &Object, world: Transform) -> Transform {
        if !o.is_hud() {
            return world;
        }
        let (cx, cy, zoom) = self.camera_start();
        Transform {
            x: (world.x - cx) * zoom,
            y: (world.y - cy) * zoom,
            rotation: world.rotation,
            scale_x: world.scale_x * zoom,
            scale_y: world.scale_y * zoom,
        }
    }

    /// Onde a câmera começa (x, y) e o zoom dela: a primeira câmera da cena,
    /// ou quem ela segue.
    pub fn camera_start(&self) -> (f64, f64, f64) {
        let mut out = None;
        visit(&self.objects, &mut |o| {
            if out.is_some() {
                return;
            }
            if let Some(c) = o.component("camera") {
                let zoom = c.num("zoom").max(0.01);
                let target = c.str("follow");
                let mut pos = self.plain_world_transform(o.id).map(|t| (t.x, t.y));
                if !target.is_empty() {
                    visit(&self.objects, &mut |t| {
                        if t.name == target {
                            pos = self.plain_world_transform(t.id).map(|w| (w.x, w.y));
                        }
                    });
                }
                if let Some((x, y)) = pos {
                    out = Some((x, y, zoom));
                }
            }
        });
        out.unwrap_or((0.0, 0.0, 1.0))
    }

    /// Todos os ids, em ordem de desenho do arquivo (pais antes dos filhos).
    pub fn ids(&self) -> Vec<u32> {
        let mut v = Vec::new();
        visit(&self.objects, &mut |o| v.push(o.id));
        v
    }
}

/// "moeda" → "moeda 2", "moeda 2" → "moeda 3".
pub fn copy_name(name: &str) -> String {
    if let Some((base, n)) = name.rsplit_once(' ')
        && let Ok(n) = n.parse::<u32>()
    {
        return format!("{base} {}", n + 1);
    }
    format!("{name} 2")
}

/// Posição (x, y), rotação (graus) e escala.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub x: f64,
    pub y: f64,
    pub rotation: f64,
    pub scale_x: f64,
    pub scale_y: f64,
}

impl Transform {
    pub fn of(o: &Object) -> Self {
        Self {
            x: o.x,
            y: o.y,
            rotation: o.rotation,
            scale_x: o.scale_x,
            scale_y: o.scale_y,
        }
    }

    /// `child` (relativo a este) no mundo. Igual ao `world_transform` do Lua.
    pub fn apply(&self, child: Transform) -> Transform {
        let lx = child.x * self.scale_x;
        let ly = child.y * self.scale_y;
        let a = self.rotation.to_radians();
        let (s, c) = a.sin_cos();
        Transform {
            x: self.x + lx * c - ly * s,
            y: self.y + lx * s + ly * c,
            rotation: self.rotation + child.rotation,
            scale_x: self.scale_x * child.scale_x,
            scale_y: self.scale_y * child.scale_y,
        }
    }

    /// O contrário de [`Transform::apply`]: `world` relativo a este.
    pub fn to_local(&self, world: Transform) -> Transform {
        let a = (-self.rotation).to_radians();
        let (s, c) = a.sin_cos();
        let dx = world.x - self.x;
        let dy = world.y - self.y;
        let sx = if self.scale_x.abs() < 1e-9 {
            1.0
        } else {
            self.scale_x
        };
        let sy = if self.scale_y.abs() < 1e-9 {
            1.0
        } else {
            self.scale_y
        };
        Transform {
            x: (dx * c - dy * s) / sx,
            y: (dx * s + dy * c) / sy,
            rotation: world.rotation - self.rotation,
            scale_x: world.scale_x / sx,
            scale_y: world.scale_y / sy,
        }
    }

    /// Ponto do mundo em coordenadas locais (sem escala).
    pub fn point_to_local(&self, x: f64, y: f64) -> (f64, f64) {
        let a = (-self.rotation).to_radians();
        let (s, c) = a.sin_cos();
        let dx = x - self.x;
        let dy = y - self.y;
        (dx * c - dy * s, dx * s + dy * c)
    }

    /// Ponto local (já com escala) no mundo.
    pub fn point_to_world(&self, lx: f64, ly: f64) -> (f64, f64) {
        let a = self.rotation.to_radians();
        let (s, c) = a.sin_cos();
        (self.x + lx * c - ly * s, self.y + lx * s + ly * c)
    }
}

/// Desfazer e refazer: guarda a cena inteira antes de cada mudança (cenas
/// são pequenas, então isso é simples e seguro).
#[derive(Debug, Default)]
pub struct History {
    undo: Vec<(SceneData, Option<String>)>,
    redo: Vec<SceneData>,
    /// Hora da última mudança guardada, para juntar mudanças seguidas.
    last: Option<std::time::Instant>,
}

/// Mudanças com a mesma chave e com menos que isso de intervalo viram um
/// passo só (ex.: arrastar um número no inspetor).
pub const MERGE_WINDOW: std::time::Duration = std::time::Duration::from_millis(1200);

/// Quantos passos ficam guardados.
pub const MAX_HISTORY: usize = 300;

impl History {
    /// Guarda `before` (a cena como estava) antes de uma mudança. Com a mesma
    /// `key` da mudança anterior, e logo em seguida, junta as duas.
    pub fn record(&mut self, before: &SceneData, key: Option<&str>) {
        let now = std::time::Instant::now();
        let merge = key.is_some()
            && self.undo.last().and_then(|(_, k)| k.as_deref()) == key
            && self.last.is_some_and(|t| now - t < MERGE_WINDOW);
        self.last = Some(now);
        self.redo.clear();
        if merge {
            return;
        }
        self.undo.push((before.clone(), key.map(str::to_string)));
        if self.undo.len() > MAX_HISTORY {
            self.undo.remove(0);
        }
    }

    /// Faz a próxima mudança começar um passo novo, mesmo com a mesma chave.
    pub fn seal(&mut self) {
        self.last = None;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self, scene: &mut SceneData) -> bool {
        let Some((prev, _)) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(scene, prev));
        self.last = None;
        true
    }

    pub fn redo(&mut self, scene: &mut SceneData) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push((std::mem::replace(scene, next), None));
        self.last = None;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exemplo() -> SceneData {
        let mut s = SceneData {
            script: "cenas/fase.lua".into(),
            background: "#14202E".into(),
            ..Default::default()
        };
        let mut j = Object::new(1, "jogador \"um\"");
        j.x = 10.5;
        j.y = -3.0;
        j.rotation = 45.0;
        j.layer = 2;
        let mut sprite = Component::new("sprite");
        sprite.set("image", Prop::Str("assets/heroi.png".into()));
        sprite.set("frame", Prop::Num(3.0));
        j.components.push(sprite);
        j.components.push(Component::new("body"));
        let mut chapeu = Object::new(2, "chapéu");
        chapeu.y = 20.0;
        chapeu.locked = true;
        j.children.push(chapeu);
        s.objects.push(j);
        s.objects.push(Object::new(3, "vazio"));
        s
    }

    #[test]
    fn esquema_vem_da_engine() {
        let s = schema();
        assert!(s.order.contains(&"sprite".to_string()));
        assert_eq!(s.order.len(), 8);
        assert_eq!(s.default_of("body", "gravity_scale"), Some(&Prop::Num(1.0)));
        assert_eq!(s.default_of("sprite", "pixel_art"), Some(&Prop::Bool(true)));
        assert_eq!(Component::new("text").str("anchor"), "center");
    }

    #[test]
    fn grava_e_le_igual() {
        let s = exemplo();
        let text = write(&s);
        let lido = read(&text, "a.cena").unwrap();
        assert_eq!(lido, s);
        // Gravar de novo dá o mesmo texto (bom para o Git).
        assert_eq!(write(&lido), text);
        assert!(text.contains("x = 10.5, y = -3, rotation = 45"), "{text}");
        assert!(text.contains("locked = true"), "{text}");
    }

    #[test]
    fn a_engine_le_o_que_o_editor_grava() {
        // O arquivo gravado é Lua válido com os campos que a engine espera.
        let text = write(&exemplo());
        let lua = Lua::new();
        let t: Table = lua.load(&text).eval().unwrap();
        let objs: Table = t.get("objects").unwrap();
        let j: Table = objs.get(1).unwrap();
        assert_eq!(j.get::<String>("name").unwrap(), "jogador \"um\"");
        let comps: Table = j.get("components").unwrap();
        let c: Table = comps.get(1).unwrap();
        assert_eq!(c.get::<String>("type").unwrap(), "sprite");
        assert_eq!(c.get::<i64>("frame").unwrap(), 3);
    }

    #[test]
    fn le_com_padroes_e_corrige_ids() {
        let s = read(
            "return { objects = { { name = 'a', components = { { type = 'shape' } } }, { id = 5 }, { id = 5 } } }",
            "x.cena",
        )
        .unwrap();
        let ids: Vec<u32> = s.objects.iter().map(|o| o.id).collect();
        assert_eq!(ids, [6, 5, 7]);
        assert_eq!(s.objects[0].components[0].num("w"), 64.0);
        assert_eq!(s.objects[1].name, "Objeto");
        assert_eq!(s.gravity, -1800.0);
        let e = read(
            "return { objects = { { components = { { type = 'nave' } } } } }",
            "x.cena",
        )
        .unwrap_err();
        assert!(e.contains("componente desconhecido \"nave\""), "{e}");
        assert!(read("return 3", "x.cena").is_err());
        assert!(read("return {", "x.cena").unwrap_err().contains("x.cena"));
    }

    #[test]
    fn numeros() {
        assert_eq!(format_num(3.0), "3");
        assert_eq!(format_num(-0.5), "-0.5");
        assert_eq!(format_num(1.0 / 3.0), "0.3333");
        assert_eq!(format_num(-0.00001), "0");
        assert_eq!(format_num(f64::NAN), "0");
    }

    #[test]
    fn arvore() {
        let mut s = exemplo();
        assert_eq!(s.parent_of(2), Some(Some(1)));
        assert_eq!(s.parent_of(1), Some(None));
        assert_eq!(s.parent_of(99), None);
        assert!(s.is_ancestor(1, 2));
        // Não vira filho do próprio filho.
        assert!(!s.move_object(1, Some(2), 0));
        // O chapéu vai para a raiz e mantém a posição no mundo.
        let antes = s.world_transform(2).unwrap();
        assert!(s.move_object(2, None, 0));
        let depois = s.world_transform(2).unwrap();
        assert!((antes.x - depois.x).abs() < 1e-9 && (antes.y - depois.y).abs() < 1e-9);
        assert!((depois.rotation - 45.0).abs() < 1e-9);
        assert_eq!(s.objects[0].id, 2);
        // Duplicar: ids novos, nome com número.
        let novo = s.duplicate(1).unwrap();
        assert_eq!(novo, 4);
        assert_eq!(s.find(4).unwrap().name, "jogador \"um\" 2");
        assert_eq!(copy_name("moeda 2"), "moeda 3");
        let (o, p, i) = s.remove(4).unwrap();
        assert_eq!((o.id, p, i), (4, None, 2));
    }

    #[test]
    fn modelos_ja_vem_no_formato_do_editor() {
        // Abrir e salvar pelo editor não muda nada nas cenas que vêm prontas.
        for text in [
            // (No Windows, o git pode trazer o arquivo com CRLF.)
            include_str!("../../nn-run/modelos/visual/cenas/fase.cena").replace("\r\n", "\n"),
            include_str!("../../nn-run/modelos/visual/cenas/fim.cena").replace("\r\n", "\n"),
            crate::files::visual_scene_template(),
        ] {
            let scene = read(&text, "modelo").unwrap();
            assert_eq!(write(&scene), text);
        }
        let prefab =
            include_str!("../../nn-run/modelos/visual/prefabs/moeda.prefab").replace("\r\n", "\n");
        let obj = read_prefab(&prefab, "moeda").unwrap();
        assert_eq!(write_prefab(&obj), prefab);
    }

    #[test]
    fn transformacoes_ida_e_volta() {
        let pai = Transform {
            x: 10.0,
            y: 5.0,
            rotation: 90.0,
            scale_x: 2.0,
            scale_y: 2.0,
        };
        let filho = Transform {
            x: 3.0,
            y: 0.0,
            rotation: 10.0,
            scale_x: 1.0,
            scale_y: 1.0,
        };
        let w = pai.apply(filho);
        assert!(
            (w.x - 10.0).abs() < 1e-9 && (w.y - 11.0).abs() < 1e-9,
            "{w:?}"
        );
        let l = pai.to_local(w);
        assert!((l.x - 3.0).abs() < 1e-9 && l.y.abs() < 1e-9 && (l.rotation - 10.0).abs() < 1e-9);
    }

    #[test]
    fn desfazer_refazer_e_juntar() {
        let mut h = History::default();
        let mut s = SceneData::default();
        // Três mudanças no mesmo campo, seguidas: um passo só.
        for x in 1..=3 {
            h.record(&s, Some("x"));
            s.gravity = -100.0 * x as f64;
        }
        // Outra mudança: passo novo.
        h.record(&s, Some("y"));
        s.overlay = true;
        assert!(h.undo(&mut s));
        assert!(!s.overlay);
        assert!(h.undo(&mut s));
        assert_eq!(s.gravity, -1800.0);
        assert!(!h.undo(&mut s));
        assert!(h.redo(&mut s));
        assert_eq!(s.gravity, -300.0);
        // Mudança nova apaga o "refazer".
        h.record(&s, None);
        assert!(!h.can_redo());
        // seal: mesma chave, mas passo novo.
        h.seal();
        h.record(&s, Some("x"));
        h.seal();
        h.record(&s, Some("x"));
        let mut passos = 0;
        while h.undo(&mut s) {
            passos += 1;
        }
        assert_eq!(passos, 4);
    }
}
