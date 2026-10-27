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
        self.get("hud") == Some(&Prop::Bool(true))
    }
}

/// Uma condição ou ação de um evento: o id do catálogo e os campos.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventItem {
    pub id: String,
    /// Condição invertida ("não").
    pub not: bool,
    pub props: Vec<(String, Prop)>,
}

impl EventItem {
    pub fn get(&self, key: &str) -> Option<&Prop> {
        self.props.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn str(&self, key: &str) -> &str {
        self.get(key).map(Prop::str).unwrap_or("")
    }

    pub fn set(&mut self, key: &str, value: Prop) {
        match self.props.iter_mut().find(|(k, _)| k == key) {
            Some((_, v)) => *v = value,
            None => self.props.push((key.to_string(), value)),
        }
    }
}

/// Uma regra "Quando... Então..." (ou uma linha de comentário).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rule {
    pub when: Vec<EventItem>,
    pub actions: Vec<EventItem>,
    /// "Enquanto": roda a cada quadro enquanto as condições valem.
    pub repeat: bool,
    /// Desligada.
    pub off: bool,
    /// Sem condições nem ações, uma linha de comentário; numa regra, o
    /// nome dela.
    pub comment: Option<String>,
}

impl Rule {
    pub fn to_json(&self) -> serde_json::Value {
        let item = |i: &EventItem, key: &str| {
            let mut m = serde_json::Map::new();
            m.insert(key.into(), serde_json::json!(i.id));
            if i.not {
                m.insert("nao".into(), serde_json::json!(true));
            }
            for (k, v) in &i.props {
                m.insert(k.clone(), v.to_json());
            }
            serde_json::Value::Object(m)
        };
        let mut m = serde_json::Map::new();
        if let Some(c) = &self.comment {
            m.insert("comment".into(), serde_json::json!(c));
        }
        if self.repeat {
            m.insert("mode".into(), serde_json::json!("enquanto"));
        }
        if self.off {
            m.insert("off".into(), serde_json::json!(true));
        }
        m.insert(
            "when".into(),
            serde_json::Value::Array(self.when.iter().map(|i| item(i, "cond")).collect()),
        );
        m.insert(
            "actions".into(),
            serde_json::Value::Array(self.actions.iter().map(|i| item(i, "act")).collect()),
        );
        serde_json::Value::Object(m)
    }
}

/// Regras como JSON (para o ajuste ao vivo).
pub fn rules_json(rules: &[Rule]) -> serde_json::Value {
    serde_json::Value::Array(rules.iter().map(Rule::to_json).collect())
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
    /// 3D: profundidade (Y é a altura), rotação em graus em volta de X, Y e Z
    /// e a escala em Z. Numa cena 2D ficam no padrão.
    pub z: f64,
    pub rx: f64,
    pub ry: f64,
    pub rz: f64,
    pub scale_z: f64,
    pub visible: bool,
    /// Travado no editor (não seleciona clicando na cena).
    pub locked: bool,
    pub layer: i64,
    /// HUD: ponto da tela em que o objeto fica preso ("" = o centro).
    pub anchor: String,
    /// Numa instância de prefab, os componentes já com as mudanças dela.
    pub components: Vec<Component>,
    /// Filhos do próprio objeto (numa instância, além dos do prefab).
    pub children: Vec<Object>,
    /// Instância de um prefab.
    pub prefab: Option<PrefabLink>,
    /// Eventos do objeto (numa instância, os do prefab).
    pub events: Vec<Rule>,
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
            z: 0.0,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            scale_z: 1.0,
            visible: true,
            locked: false,
            layer: 0,
            anchor: String::new(),
            components: Vec::new(),
            children: Vec::new(),
            prefab: None,
            events: Vec::new(),
        }
    }

    pub fn component(&self, kind: &str) -> Option<&Component> {
        self.components.iter().find(|c| c.kind == kind)
    }

    /// Preso à tela (HUD): algum componente visual com `hud`, ou um Grupo
    /// que arruma filhos presos à tela.
    pub fn is_hud(&self) -> bool {
        self.components.iter().any(Component::is_hud)
            || (self.component("layout").is_some() && self.children.iter().any(Object::is_hud))
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
        self.events = base.events.clone();
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
            "z" => Prop::Num(self.z),
            "rx" => Prop::Num(self.rx),
            "ry" => Prop::Num(self.ry),
            "rz" => Prop::Num(self.rz),
            "scale_z" => Prop::Num(self.scale_z),
            "visible" => Prop::Bool(self.visible),
            "locked" => Prop::Bool(self.locked),
            "layer" => Prop::Num(self.layer as f64),
            "anchor" => Prop::Str(self.anchor.clone()),
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
    /// Cena 3D (`mode = "3d"` no arquivo).
    pub three_d: bool,
    /// Céu, sol, névoa e sombras de uma cena 3D (os campos de
    /// `M.ENVIRONMENT`, na mesma ordem).
    pub environment: Vec<(String, Prop)>,
    pub objects: Vec<Object>,
    /// Eventos da cena.
    pub events: Vec<Rule>,
}

impl Default for SceneData {
    fn default() -> Self {
        Self {
            script: String::new(),
            background: String::new(),
            gravity: -1800.0,
            overlay: false,
            three_d: false,
            environment: schema().environment.clone(),
            objects: Vec::new(),
            events: Vec::new(),
        }
    }
}

impl SceneData {
    /// Uma cena 3D nova: câmera, chão e o ambiente padrão.
    pub fn new_3d() -> Self {
        let mut scene = Self {
            three_d: true,
            gravity: -20.0,
            ..Default::default()
        };
        let mut camera = Object::new(1, "Câmera");
        camera.y = 5.0;
        camera.z = 10.0;
        camera.rx = -22.0;
        camera.components.push(Component::new("camera3d"));
        let mut chao = Object::new(2, "Chão");
        chao.scale_x = 20.0;
        chao.scale_z = 20.0;
        let mut forma = Component::new("shape3d");
        forma.set("shape", Prop::Str("plane".into()));
        chao.components.push(forma);
        chao.components.push(Component::new("body3d"));
        let mut cubo = Object::new(3, "Cubo");
        cubo.y = 0.5;
        let mut c = Component::new("shape3d");
        c.set("color", Prop::Str("#FF8C42".into()));
        cubo.components.push(c);
        scene.objects = vec![camera, chao, cubo];
        scene
    }

    /// 3D: a matriz do objeto no mundo (com as dos pais).
    pub fn world_matrix3(&self, id: u32) -> Option<glam::Mat4> {
        let o = self.find(id)?;
        let local = local_matrix3(o);
        match self.parent_of(id)? {
            None => Some(local),
            Some(p) => Some(self.world_matrix3(p)? * self.bone_matrix3(p, o) * local),
        }
    }

    /// 3D: a matriz do pai no mundo (identidade na raiz). Para um objeto
    /// preso a um osso, a do osso (na pose de descanso).
    pub fn parent_matrix3(&self, id: u32) -> glam::Mat4 {
        match (self.parent_of(id).flatten(), self.find(id)) {
            (Some(p), Some(o)) => {
                self.world_matrix3(p).unwrap_or(glam::Mat4::IDENTITY) * self.bone_matrix3(p, o)
            }
            _ => glam::Mat4::IDENTITY,
        }
    }

    /// Objeto preso a um osso (componente "bone"): onde o osso fica no
    /// modelo do pai, na pose de descanso. Identidade sem osso (ou com um
    /// modelo que o editor ainda não leu).
    pub fn bone_matrix3(&self, parent: u32, o: &Object) -> glam::Mat4 {
        let Some(bone) = o
            .component("bone")
            .map(|c| c.str("bone"))
            .filter(|b| !b.is_empty())
        else {
            return glam::Mat4::IDENTITY;
        };
        self.find(parent)
            .and_then(|p| p.component("model"))
            .and_then(|m| bone_rest(m.str("file"), bone))
            .unwrap_or(glam::Mat4::IDENTITY)
    }

    /// 3D: põe o objeto na posição, rotação e escala do mundo dadas pela
    /// matriz (convertendo para o espaço do pai).
    pub fn set_world_matrix3(&mut self, id: u32, world: glam::Mat4) {
        let local = self.parent_matrix3(id).inverse() * world;
        if let Some(o) = self.find_mut(id) {
            set_local_matrix3(o, local);
        }
    }

    /// O ambiente como o jogo vai mostrar ao começar: com Dia e noite, o da
    /// hora em que ele começa (o mesmo `M.environment_at` do jogo), e se é
    /// de noite (para as luzes "Só à noite").
    pub fn environment_now(&self) -> EnvironmentNow {
        thread_local! {
            static LUA: (Lua, Table) = {
                let lua = crate::quiet_lua();
                let module: Table = lua
                    .load(nn_script::SCENE_RUNTIME)
                    .eval()
                    .expect("módulo das cenas da engine");
                (lua, module)
            };
        }
        let keys: Vec<String> = schema()
            .environment
            .iter()
            .map(|(k, _)| k.clone())
            .collect();
        LUA.with(|(lua, module)| {
            let run = || -> mlua::Result<EnvironmentNow> {
                let env = lua.create_table()?;
                for k in &keys {
                    let v = match self.env(k) {
                        Prop::Num(n) => Value::Number(n),
                        Prop::Bool(b) => Value::Boolean(b),
                        Prop::Str(s) => Value::String(lua.create_string(&s)?),
                    };
                    env.set(k.as_str(), v)?;
                }
                let at: mlua::Function = module.get("environment_at")?;
                let hour = self.env("start_hour").num();
                let e: Table = at.call((env, hour))?;
                let mut values = BTreeMap::new();
                for k in keys.iter().map(String::as_str).chain(["stars"]) {
                    if let Some(p) = e.get::<Value>(k).ok().as_ref().and_then(to_prop) {
                        values.insert(k.to_string(), p);
                    }
                }
                let night = if self.env("cycle").bool() {
                    let f: mlua::Function = module.get("is_night_hour")?;
                    f.call(hour)?
                } else {
                    let c = values
                        .get("sun_color")
                        .map(|p| p.str().to_string())
                        .unwrap_or_default();
                    let hex = u32::from_str_radix(
                        c.trim_start_matches('#').get(..6).unwrap_or("FFFFFF"),
                        16,
                    )
                    .unwrap_or(0xFFFFFF);
                    let (r, g, b) = (
                        ((hex >> 16) & 255) as f64 / 255.0,
                        ((hex >> 8) & 255) as f64 / 255.0,
                        (hex & 255) as f64 / 255.0,
                    );
                    0.3 * r + 0.59 * g + 0.11 * b < 0.5
                };
                Ok(EnvironmentNow { values, night })
            };
            run().unwrap_or_else(|e| {
                log::warn!("ambiente: {e}");
                EnvironmentNow {
                    values: keys.iter().map(|k| (k.clone(), self.env(k))).collect(),
                    night: false,
                }
            })
        })
    }

    pub fn env(&self, key: &str) -> Prop {
        self.environment
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .or_else(|| {
                schema()
                    .environment
                    .iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, v)| v.clone())
            })
            .unwrap_or(Prop::Num(0.0))
    }

    pub fn set_env(&mut self, key: &str, value: Prop) {
        match self.environment.iter_mut().find(|(k, _)| k == key) {
            Some((_, v)) => *v = value,
            None => self.environment.push((key.to_string(), value)),
        }
    }
}

thread_local! {
    /// Os ossos dos modelos que o editor já leu: (modelo, osso) → onde o
    /// osso fica na pose de descanso. Preenchido por `Sizes` ao ler um
    /// modelo; vale para todas as cenas abertas.
    static BONES: std::cell::RefCell<std::collections::HashMap<(String, String), glam::Mat4>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Guarda os ossos de um modelo (na pose de descanso) para
/// [`SceneData::bone_matrix3`].
pub fn remember_bones(model: &str, bones: &[(String, glam::Mat4)]) {
    BONES.with(|b| {
        let mut b = b.borrow_mut();
        b.retain(|(m, _), _| m != model);
        for (name, m) in bones {
            b.insert((model.to_string(), name.clone()), *m);
        }
    });
}

/// Onde um osso do modelo fica na pose de descanso (se o editor já leu o
/// modelo).
pub fn bone_rest(model: &str, bone: &str) -> Option<glam::Mat4> {
    BONES.with(|b| {
        b.borrow()
            .get(&(model.to_string(), bone.to_string()))
            .copied()
    })
}

/// A rotação 3D do objeto (graus em volta de X, Y e Z, aplicados Y, X, Z:
/// virar, inclinar e rolar, como a engine).
pub fn rotation3(o: &Object) -> glam::Quat {
    glam::Quat::from_euler(
        glam::EulerRot::YXZ,
        (o.ry as f32).to_radians(),
        (o.rx as f32).to_radians(),
        (o.rz as f32).to_radians(),
    )
}

/// Graus (x, y, z) de uma rotação, como [`rotation3`] lê.
pub fn degrees3(q: glam::Quat) -> (f64, f64, f64) {
    let (y, x, z) = q.normalize().to_euler(glam::EulerRot::YXZ);
    let r = |a: f32| round_to(f64::from(a.to_degrees()), 1e-3);
    (r(x), r(y), r(z))
}

fn round_to(v: f64, step: f64) -> f64 {
    let r = (v / step).round() * step;
    if r == 0.0 { 0.0 } else { r }
}

/// A matriz 3D do objeto em relação ao pai.
pub fn local_matrix3(o: &Object) -> glam::Mat4 {
    glam::Mat4::from_scale_rotation_translation(
        glam::Vec3::new(o.scale_x as f32, o.scale_y as f32, o.scale_z as f32),
        rotation3(o),
        glam::Vec3::new(o.x as f32, o.y as f32, o.z as f32),
    )
}

/// Põe posição, rotação e escala do objeto a partir de uma matriz local.
pub fn set_local_matrix3(o: &mut Object, m: glam::Mat4) {
    let (s, q, t) = m.to_scale_rotation_translation();
    let r = |v: f32| round_to(f64::from(v), 1e-4);
    (o.x, o.y, o.z) = (r(t.x), r(t.y), r(t.z));
    (o.scale_x, o.scale_y, o.scale_z) = (r(s.x), r(s.y), r(s.z));
    (o.rx, o.ry, o.rz) = degrees3(q);
}

/// Os componentes que existem e os campos de cada um.
#[derive(Debug)]
pub struct Schema {
    components: BTreeMap<String, Vec<(String, Prop)>>,
    /// Ordem para mostrar no "Adicionar componente".
    pub order: Vec<String>,
    /// Tema padrão da interface (`M.THEME`): (chave, valor).
    pub theme: Vec<(String, String)>,
    /// Ambiente 3D padrão (`M.ENVIRONMENT`).
    pub environment: Vec<(String, Prop)>,
    /// Formas do componente shape3d (`M.SHAPES3D`).
    pub shapes3d: Vec<String>,
    /// Ambientes prontos (`M.ENVIRONMENT_PRESETS`): id, nome e o ambiente
    /// inteiro (o padrão com os campos dele por cima).
    pub environment_presets: Vec<EnvironmentPreset>,
}

/// O ambiente de uma cena 3D na hora em que o jogo começa (veja
/// [`SceneData::environment_now`]).
#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentNow {
    pub values: BTreeMap<String, Prop>,
    /// De noite: as luzes "Só à noite" acendem.
    pub night: bool,
}

impl EnvironmentNow {
    pub fn get(&self, key: &str) -> Prop {
        self.values.get(key).cloned().unwrap_or(Prop::Num(0.0))
    }
}

/// Um ambiente pronto: id, nome e o ambiente inteiro.
pub type EnvironmentPreset = (String, String, Vec<(String, Prop)>);

/// Chaves do tema da interface, na ordem do formulário.
pub const THEME_KEYS: [&str; 10] = [
    "font", "text", "button", "hover", "press", "disabled", "panel", "border", "accent", "sound",
];

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
        let lua = crate::quiet_lua();
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
            "panel",
            "button",
            "bar",
            "icons",
            "slider",
            "checkbox",
            "textfield",
            "layout",
            "model",
            "anim3d",
            "shape3d",
            "light3d",
            "camera3d",
            "body3d",
            "terrain",
            "text3d",
            "bone",
        ]
        .into_iter()
        .filter(|k| components.contains_key(*k))
        .map(str::to_string)
        .collect();
        let theme_table: Table = module.get("THEME").expect("THEME");
        let theme = THEME_KEYS
            .iter()
            .map(|k| {
                (
                    k.to_string(),
                    theme_table.get::<String>(*k).unwrap_or_default(),
                )
            })
            .collect();
        let env_list: Table = module.get("ENVIRONMENT").expect("ENVIRONMENT");
        let environment: Vec<(String, Prop)> = env_list
            .sequence_values::<Table>()
            .map(|f| {
                let f = f.expect("campo do ambiente");
                let key: String = f.get(1).expect("nome");
                let value = to_prop(&f.get::<Value>(2).expect("padrão")).expect("valor simples");
                (key, value)
            })
            .collect();
        let shapes: Table = module.get("SHAPES3D").expect("SHAPES3D");
        let shapes3d = shapes
            .sequence_values::<String>()
            .map(|s| s.expect("forma"))
            .collect();
        let presets: Table = module
            .get("ENVIRONMENT_PRESETS")
            .expect("ENVIRONMENT_PRESETS");
        let environment_presets = presets
            .sequence_values::<Table>()
            .map(|p| {
                let p = p.expect("ambiente pronto");
                let id: String = p.get(1).expect("id");
                let name: String = p.get(2).expect("nome");
                let set: Table = p.get(3).expect("campos");
                let full = environment
                    .iter()
                    .map(|(k, v): &(String, Prop)| {
                        let v = set
                            .get::<Value>(k.as_str())
                            .ok()
                            .and_then(|x| to_prop(&x))
                            .unwrap_or_else(|| v.clone());
                        (k.clone(), v)
                    })
                    .collect();
                (id, name, full)
            })
            .collect();
        Schema {
            components,
            order,
            theme,
            environment,
            shapes3d,
            environment_presets,
        }
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
    let lua = crate::quiet_lua();
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
        three_d: string(&t, "mode") == "3d",
        environment: schema().environment.clone(),
        objects: Vec::new(),
        events: Vec::new(),
    };
    if let Ok(Value::Table(env)) = t.get::<Value>("environment") {
        for (key, default) in schema().environment.clone() {
            let v = env
                .get::<Value>(key.as_str())
                .ok()
                .and_then(|v| to_prop(&v))
                .unwrap_or(default);
            scene.set_env(&key, v);
        }
    }
    if let Ok(Value::Table(list)) = t.get::<Value>("objects") {
        scene.objects = read_objects(&list, name)?;
    }
    scene.events = read_rules(&t, name, "cena")?;
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
    obj.z = num(o, "z", 0.0);
    obj.rx = num(o, "rx", 0.0);
    obj.ry = num(o, "ry", 0.0);
    obj.rz = num(o, "rz", 0.0);
    obj.scale_z = num(o, "scale_z", 1.0);
    obj.visible = boolean(o, "visible", true);
    obj.locked = boolean(o, "locked", false);
    obj.layer = num(o, "layer", 0.0) as i64;
    obj.anchor = string(o, "anchor");
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
    if obj.prefab.is_none() {
        obj.events = read_rules(o, name, &obj.name)?;
    }
    Ok(obj)
}

/// Lê o campo `events` (lista de regras) de uma cena ou objeto.
fn read_rules(t: &Table, name: &str, owner: &str) -> Result<Vec<Rule>, String> {
    let Ok(Value::Table(list)) = t.get::<Value>("events") else {
        return Ok(Vec::new());
    };
    let item = |it: &Table, key: &str| -> EventItem {
        let mut e = EventItem {
            id: string(it, key),
            not: boolean(it, "nao", false),
            props: Vec::new(),
        };
        let mut props: Vec<(String, Prop)> = it
            .pairs::<String, Value>()
            .filter_map(Result::ok)
            .filter(|(k, _)| k != key && k != "nao")
            .filter_map(|(k, v)| to_prop(&v).map(|p| (k, p)))
            .collect();
        crate::events::sort_props(&e.id, key == "cond", &mut props);
        e.props = props;
        e
    };
    let mut out = Vec::new();
    for (i, r) in list.sequence_values::<Table>().enumerate() {
        let r = r.map_err(|e| format!("{name}: evento {} de {owner} inválido: {e}", i + 1))?;
        let mut rule = Rule {
            repeat: string(&r, "mode") == "enquanto",
            off: boolean(&r, "off", false),
            comment: match r.get::<Value>("comment") {
                Ok(Value::String(s)) => Some(s.to_string_lossy()),
                _ => None,
            },
            ..Default::default()
        };
        if let Ok(Value::Table(w)) = r.get::<Value>("when") {
            rule.when = w
                .sequence_values::<Table>()
                .filter_map(Result::ok)
                .map(|c| item(&c, "cond"))
                .collect();
        }
        if let Ok(Value::Table(a)) = r.get::<Value>("actions") {
            rule.actions = a
                .sequence_values::<Table>()
                .filter_map(Result::ok)
                .map(|c| item(&c, "act"))
                .collect();
        }
        out.push(rule);
    }
    Ok(out)
}

/// Grava uma lista de regras (`events = { ... },`).
fn write_rules(s: &mut String, rules: &[Rule], depth: usize) {
    if rules.is_empty() {
        return;
    }
    let pad = "  ".repeat(depth);
    let inner = "  ".repeat(depth + 1);
    let item = |i: &EventItem, key: &str| {
        let mut line = format!("{key} = {}", crate::config_lua::lua_string(&i.id));
        if i.not {
            line.push_str(", nao = true");
        }
        for (k, v) in &i.props {
            line.push_str(&format!(", {k} = {}", v.to_lua()));
        }
        format!("{{ {line} }}")
    };
    s.push_str(&format!("{pad}events = {{\n"));
    for r in rules {
        if let Some(c) = &r.comment
            && r.when.is_empty()
            && r.actions.is_empty()
        {
            s.push_str(&format!(
                "{inner}{{ comment = {} }},\n",
                crate::config_lua::lua_string(c)
            ));
            continue;
        }
        let mut head = String::new();
        if let Some(c) = &r.comment {
            head.push_str(&format!("comment = {}, ", crate::config_lua::lua_string(c)));
        }
        if r.repeat {
            head.push_str("mode = \"enquanto\", ");
        }
        if r.off {
            head.push_str("off = true, ");
        }
        s.push_str(&format!("{inner}{{ {head}when = {{\n"));
        for c in &r.when {
            s.push_str(&format!("{inner}    {},\n", item(c, "cond")));
        }
        s.push_str(&format!("{inner}  }}, actions = {{\n"));
        for a in &r.actions {
            s.push_str(&format!("{inner}    {},\n", item(a, "act")));
        }
        s.push_str(&format!("{inner}  }} }},\n"));
    }
    s.push_str(&format!("{pad}}},\n"));
}

/// Lê um `.prefab`: um objeto (com filhos).
pub fn read_prefab(text: &str, name: &str) -> Result<Object, String> {
    let lua = crate::quiet_lua();
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
    if scene.three_d {
        s.push_str("  mode = \"3d\",\n");
        let fields: Vec<String> = scene
            .environment
            .iter()
            .map(|(k, v)| format!("{k} = {}", v.to_lua()))
            .collect();
        s.push_str(&format!("  environment = {{ {} }},\n", fields.join(", ")));
    }
    write_rules(&mut s, &scene.events, 1);
    s.push_str("  objects = {\n");
    write_objects(&mut s, &scene.objects, 2);
    s.push_str("  },\n}\n");
    s
}

/// Campos que vieram depois da 0.7: no valor padrão, não vão para o
/// arquivo. Assim um projeto da 0.7 aberto e gravado sai igual, e o arquivo
/// continua abrindo no `nn` da 0.7. (A largura e o alinhamento do texto
/// vieram na 0.10.)
const LATE_FIELDS: &[(&str, &str)] = &[
    ("body", "ox"),
    ("body", "oy"),
    ("particles", "pixel"),
    ("text", "width"),
    ("text", "align"),
];

fn late_default(kind: &str, key: &str, v: &Prop) -> bool {
    LATE_FIELDS.contains(&(kind, key)) && schema().default_of(kind, key) == Some(v)
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
        if o.z != 0.0 || o.rx != 0.0 || o.ry != 0.0 || o.rz != 0.0 || o.scale_z != 1.0 {
            s.push_str(&format!(
                "{inner}z = {}, rx = {}, ry = {}, rz = {}, scale_z = {},\n",
                format_num(o.z),
                format_num(o.rx),
                format_num(o.ry),
                format_num(o.rz),
                format_num(o.scale_z)
            ));
        }
        s.push_str(&format!(
            "{inner}visible = {}, layer = {},",
            o.visible, o.layer
        ));
        if o.locked {
            s.push_str(" locked = true,");
        }
        if !o.anchor.is_empty() {
            s.push_str(&format!(
                " anchor = {},",
                crate::config_lua::lua_string(&o.anchor)
            ));
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
                    if late_default(&c.kind, k, v) {
                        continue;
                    }
                    line.push_str(&format!(", {k} = {}", v.to_lua()));
                }
                s.push_str(&format!("{inner}  {{ {line} }},\n"));
            }
            s.push_str(&format!("{inner}}},\n"));
        }
        if o.prefab.is_none() {
            write_rules(s, &o.events, depth + 1);
        }
        if !o.children.is_empty() {
            s.push_str(&format!("{inner}children = {{\n"));
            write_objects(s, &o.children, depth + 2);
            s.push_str(&format!("{inner}}},\n"));
        }
        s.push_str(&format!("{pad}}},\n"));
    }
}

// O que o editor sabe do projeto aberto para desenhar como o jogo: o tamanho
// da tela (para as âncoras do HUD), a proporção escolhida para ver, o tema
// da interface e as variáveis do jogo (do config.lua). Fica na thread da
// interface (cada janela de teste tem a sua).
#[derive(Default)]
struct ProjectView {
    screen: (f64, f64),
    preview: f64,
    theme: Vec<(String, String)>,
    vars: Vec<(String, Prop)>,
}

thread_local! {
    static VIEW: std::cell::RefCell<ProjectView> = std::cell::RefCell::new(ProjectView::default());
}

/// Muda o tamanho da tela usado nas âncoras do HUD.
pub fn set_screen_size(w: f64, h: f64) {
    VIEW.with_borrow_mut(|v| v.screen = (w, h));
}

/// Proporções de tela para ver como o HUD fica (largura ÷ altura; 0 = a
/// do jogo).
pub const PREVIEWS: [(&str, f64); 5] = [
    ("do jogo", 0.0),
    ("16:9", 16.0 / 9.0),
    ("4:3", 4.0 / 3.0),
    ("21:9", 21.0 / 9.0),
    ("9:16 (celular em pé)", 9.0 / 16.0),
];

/// Vê a cena numa tela de outra proporção (0 = a do jogo).
pub fn set_preview(ratio: f64) {
    VIEW.with_borrow_mut(|v| v.preview = ratio);
}

pub fn preview() -> f64 {
    VIEW.with_borrow(|v| v.preview)
}

/// Tamanho da tela usado nas âncoras do HUD (padrão 1280 × 720). Com uma
/// proporção escolhida para ver, a altura fica e a largura muda.
pub fn screen_size() -> (f64, f64) {
    let ((w, h), ratio) = VIEW.with_borrow(|v| (v.screen, v.preview));
    let (w, h) = if w > 0.0 && h > 0.0 {
        (w, h)
    } else {
        (1280.0, 720.0)
    };
    if ratio > 0.0 {
        ((h * ratio).round(), h)
    } else {
        (w, h)
    }
}

/// O editor leu o config.lua: o tema e as variáveis do jogo.
pub fn set_project_ui(theme: Vec<(String, String)>, vars: Vec<(String, Prop)>) {
    VIEW.with_borrow_mut(|v| {
        v.theme = theme;
        v.vars = vars;
    });
}

/// Valor do tema da interface (o do projeto, ou o padrão da engine).
pub fn theme_value(key: &str) -> String {
    let mine = VIEW.with_borrow(|v| {
        v.theme
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .filter(|v| !v.is_empty())
    });
    mine.unwrap_or_else(|| {
        schema()
            .theme
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    })
}

/// A cor do componente, ou a do tema quando vazia.
pub fn themed(value: &str, key: &str) -> String {
    if value.is_empty() {
        theme_value(key)
    } else {
        value.to_string()
    }
}

/// Valor inicial de uma variável do jogo (config.lua).
pub fn var_value(name: &str) -> Option<Prop> {
    VIEW.with_borrow(|v| {
        v.vars
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, p)| p.clone())
    })
}

/// Nomes das variáveis do jogo (config.lua).
pub fn var_names() -> Vec<String> {
    VIEW.with_borrow(|v| v.vars.iter().map(|(k, _)| k.clone()).collect())
}

/// Componente de interface (menus e HUD)?
pub fn is_ui_kind(kind: &str) -> bool {
    matches!(
        kind,
        "panel" | "button" | "bar" | "icons" | "slider" | "checkbox" | "textfield"
    )
}

/// Pontos da tela para prender o HUD: (nome, rótulo, fração x, fração y a
/// partir do canto de cima à esquerda). Como `M.ANCHORS` do cena.lua.
pub const SCREEN_ANCHORS: [(&str, &str, f64, f64); 9] = [
    ("top_left", "em cima, à esquerda", 0.0, 0.0),
    ("top", "em cima", 0.5, 0.0),
    ("top_right", "em cima, à direita", 1.0, 0.0),
    ("left", "à esquerda", 0.0, 0.5),
    ("center", "no meio", 0.5, 0.5),
    ("right", "à direita", 1.0, 0.5),
    ("bottom_left", "embaixo, à esquerda", 0.0, 1.0),
    ("bottom", "embaixo", 0.5, 1.0),
    ("bottom_right", "embaixo, à direita", 1.0, 1.0),
];

/// Quanto o ponto de âncora está do centro da tela (y para cima).
pub fn anchor_offset(anchor: &str) -> (f64, f64) {
    let (w, h) = screen_size();
    SCREEN_ANCHORS
        .iter()
        .find(|a| a.0 == anchor)
        .map(|a| ((a.2 - 0.5) * w, (0.5 - a.3) * h))
        .unwrap_or((0.0, 0.0))
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
        // Um nome que ainda não existe na cena ("Árvore 2" já existe: "Árvore 3").
        let mut used = Vec::new();
        visit(&self.objects, &mut |o| used.push(o.name.clone()));
        let mut name = copy_name(&copy.name);
        while used.contains(&name) {
            name = copy_name(&name);
        }
        copy.name = name;
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

    /// O objeto fica preso à tela (HUD, ou a cena abre por cima de outra)?
    pub fn is_screen(&self, o: &Object) -> bool {
        self.overlay || o.is_hud()
    }

    /// Onde fica um objeto da raiz no mundo do editor.
    pub fn root_place(&self, o: &Object, local: Transform) -> Transform {
        if !self.is_screen(o) {
            return local;
        }
        // No jogo, o HUD fica onde estaria com a câmera no começo, sem zoom,
        // mais o ponto da tela em que ele está preso.
        let (cx, cy, zoom) = self.camera_start();
        let (ax, ay) = anchor_offset(&o.anchor);
        Transform {
            x: cx + (local.x + ax) / zoom,
            y: cy + (local.y + ay) / zoom,
            rotation: local.rotation,
            scale_x: local.scale_x / zoom,
            scale_y: local.scale_y / zoom,
        }
    }

    /// O contrário de [`SceneData::root_place`].
    pub fn root_unplace(&self, o: &Object, world: Transform) -> Transform {
        if !self.is_screen(o) {
            return world;
        }
        let (cx, cy, zoom) = self.camera_start();
        let (ax, ay) = anchor_offset(&o.anchor);
        Transform {
            x: (world.x - cx) * zoom - ax,
            y: (world.y - cy) * zoom - ay,
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
    /// Conta cada passo guardado, desfeito ou refeito (para saber se a cena
    /// mudou depois de um aviso com Desfazer).
    version: u64,
}

/// Mudanças com a mesma chave e com menos que isso de intervalo viram um
/// passo só (ex.: arrastar um número no inspetor).
pub const MERGE_WINDOW: std::time::Duration = std::time::Duration::from_millis(1200);

/// Quantos passos ficam guardados.
pub const MAX_HISTORY: usize = 300;

impl History {
    /// Guarda `before` (a cena como estava) antes de uma mudança. Com a mesma
    /// `key` da mudança anterior, e logo em seguida, junta as duas.
    /// O passo atual: muda a cada mudança guardada, desfeita ou refeita.
    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn record(&mut self, before: &SceneData, key: Option<&str>) {
        self.version += 1;
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
        self.version += 1;
        self.redo.push(std::mem::replace(scene, prev));
        self.last = None;
        true
    }

    pub fn redo(&mut self, scene: &mut SceneData) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.version += 1;
        self.undo.push((std::mem::replace(scene, next), None));
        self.last = None;
        true
    }
}

/// Nome e descrição de cada componente.
/// Os valores que um campo de texto de um componente aceita (os que o
/// Inspetor oferece), ou `None` se ele é livre.
pub fn field_choices(kind: &str, key: &str) -> Option<&'static [&'static str]> {
    Some(match (kind, key) {
        ("sprite", "fill") => &["stretch", "repeat"],
        ("button", "action") => &["nada", "cena", "por_cima", "fechar", "reiniciar", "sair"],
        ("text", "anchor") => &[
            "topleft",
            "top",
            "topright",
            "left",
            "center",
            "right",
            "bottomleft",
            "bottom",
            "bottomright",
        ],
        ("shape", "shape") => &["rect", "circle"],
        ("body", "body") => &["dynamic", "static", "kinematic"],
        ("body", "shape") => &["box", "circle"],
        ("sound", "channel") => &["effects", "music"],
        ("particles", "mode") => &["continuo", "explosao"],
        ("particles", "when_done") => &["nada", "destruir"],
        ("particles", "area") => &["ponto", "retangulo", "circulo"],
        ("particles", "blend") => &["alpha", "add"],
        ("bar", "direction") => &["right", "left", "up", "down"],
        ("icons", "align") => &["left", "center", "right"],
        ("slider", "target") => &["variavel", "musica", "efeitos", "geral"],
        ("checkbox", "target") => &["variavel", "tela_cheia"],
        ("layout", "arrange") => &["vertical", "horizontal", "grid", "none"],
        ("layout", "align") => &["start", "center", "end"],
        ("shape3d", "shape") => &["cube", "sphere", "cylinder", "cone", "capsule", "plane"],
        ("shape3d", "material") => &["normal", "agua"],
        ("light3d", "kind") => &["point", "spot"],
        ("text3d", "facing") => &["camera", "placa"],
        ("body3d", "body") => &["static", "dynamic", "kinematic"],
        ("body3d", "shape") => &["auto", "box", "sphere", "capsule", "cylinder", "mesh"],
        _ => return None,
    })
}

/// O valor certo de um campo de escolha escrito de outro jeito: sem ligar
/// para maiúsculas, espaços, hífens e sublinhados (`"top_left"` vira o
/// `"topleft"` do texto; `"Top Left"` também).
pub fn choice_like(kind: &str, key: &str, text: &str) -> Option<&'static str> {
    let squash = |s: &str| {
        s.chars()
            .filter(|c| !matches!(c, '_' | '-' | ' '))
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    let want = squash(text);
    field_choices(kind, key)?
        .iter()
        .copied()
        .find(|c| squash(c) == want)
}

/// O nome certo de uma âncora do objeto (`"topleft"` vira `"top_left"`).
pub fn screen_anchor_like(text: &str) -> Option<&'static str> {
    let squash = |s: &str| {
        s.chars()
            .filter(|c| !matches!(c, '_' | '-' | ' '))
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    let want = squash(text);
    SCREEN_ANCHORS
        .iter()
        .map(|a| a.0)
        .find(|a| squash(a) == want)
}

pub fn component_info(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "sprite" => (
            "Imagem (sprite)",
            "Mostra uma imagem ou um quadro de spritesheet. w e h (0 = o tamanho da imagem) são multiplicados pela escala do objeto. Com fill = \"repeat\", a imagem se repete lado a lado: w e h são o tamanho de CADA cópia (o tile) e a escala do objeto diz quantas cabem (um chão de 20 blocos de 16x16: w 16, h 16, scale_x 20).",
        ),
        "text" => ("Texto", "Escreve um texto no mundo."),
        "shape" => ("Forma", "Retângulo ou círculo de uma cor."),
        "body" => (
            "Corpo físico",
            "Cai, colide, para no chão. Mexa nele pelo script: objeto.body.vx. A caixa (w, h) fica centrada no objeto; ox e oy a deslocam (na escala do objeto; oy positivo sobe), para a colisão ficar nos pés de um sprite com espaço vazio embaixo.",
        ),
        "map" => (
            "Mapa de tiles",
            "Desenha um mapa .tmj (do editor ou do Tiled), com a colisão dos tiles.",
        ),
        "camera" => ("Câmera", "Onde a câmera começa, ou quem ela segue."),
        "sound" => ("Som", "Toca um som ou música quando a cena começa."),
        "particles" => ("Partículas", "Fumaça, faíscas, poeira, chuva..."),
        "panel" => (
            "Painel",
            "Fundo de menu ou de HUD: cantos arredondados, borda ou uma moldura.",
        ),
        "button" => (
            "Botão",
            "Aperta com o mouse, o teclado ou o controle. Troca de cena, abre por cima, fecha...",
        ),
        "bar" => (
            "Barra",
            "Vida, energia, tempo: mostra uma variável do jogo.",
        ),
        "icons" => (
            "Ícones",
            "Corações, vidas, moedas: um ícone para cada ponto da variável.",
        ),
        "slider" => (
            "Deslizador",
            "Escolhe um valor arrastando: volume, dificuldade...",
        ),
        "checkbox" => (
            "Caixa de marcar",
            "Liga e desliga: tela cheia, uma opção do jogo.",
        ),
        "textfield" => (
            "Campo de texto",
            "Para digitar: o nome do jogador, um código.",
        ),
        "layout" => (
            "Grupo (arrumar)",
            "Arruma os filhos sozinho, em coluna, em linha ou em grade.",
        ),
        "model" => (
            "Modelo 3D",
            "Um modelo .glb ou .gltf (do Blender e outros).",
        ),
        "shape3d" => (
            "Forma 3D",
            "Cubo, esfera, cilindro, cone, cápsula ou plano, de uma cor.",
        ),
        "light3d" => (
            "Luz",
            "Uma lâmpada, uma tocha ou um holofote: ilumina o que está em volta (as luzes não fazem sombra; só o sol).",
        ),
        "camera3d" => (
            "Câmera 3D",
            "A câmera do jogo: ele mostra o que ela vê, de onde ela está.",
        ),
        "terrain" => (
            "Terreno",
            "Um chão com morros, vales, cores e água, pintado com o ✏ Pintar terreno da vista 3D.",
        ),
        "text3d" => (
            "Texto 3D",
            "Escreve no mundo 3D: placas, nomes em cima dos personagens, avisos.",
        ),
        "anim3d" => (
            "Animação 3D",
            "Toca uma animação do modelo 3D (andar, acenar...), com velocidade e repetir.",
        ),
        "bone" => (
            "Preso a um osso",
            "Para segurar: uma espada ou um cajado na mão do personagem, que acompanha a animação. Ponha o objeto como filho do personagem.",
        ),
        "body3d" => (
            "Corpo 3D",
            "Física 3D: chão e paredes que bloqueiam, caixas que caem, plataformas que andam.",
        ),
        "behavior" => (
            "Comportamento",
            "Um script com propriedades: pronto (andar, pular, coletar...) ou do jogo.",
        ),
        _ => ("Componente", ""),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrizes_3d_de_pai_e_filho() {
        let mut scene = SceneData::new_3d();
        let mut pai = Object::new(10, "pai");
        pai.x = 5.0;
        pai.ry = 90.0;
        let mut filho = Object::new(11, "filho");
        filho.z = -2.0; // 2 à frente do pai
        pai.children.push(filho);
        scene.objects.push(pai);
        // Virado 90° em Y, a frente do pai (-Z) aponta para -X.
        let m = scene.world_matrix3(11).unwrap();
        let p = m.transform_point3(glam::Vec3::ZERO);
        assert!((p - glam::Vec3::new(3.0, 0.0, 0.0)).length() < 1e-4, "{p}");
        // Pondo o filho num lugar do mundo, a posição local é a certa.
        scene.set_world_matrix3(
            11,
            glam::Mat4::from_translation(glam::Vec3::new(5.0, 1.0, 4.0)),
        );
        let f = scene.find(11).unwrap();
        // (0, 1, 4) do mundo em volta do pai, desfazendo o giro de 90°: x = -4.
        assert!(
            (f.x + 4.0).abs() < 1e-3 && (f.y - 1.0).abs() < 1e-3 && f.z.abs() < 1e-3,
            "{f:?}"
        );
        assert!(
            (f.ry + 90.0).abs() < 1e-2,
            "o filho desfaz o giro do pai: {}",
            f.ry
        );
        // Graus voltam iguais.
        let mut o = Object::new(1, "o");
        (o.rx, o.ry, o.rz) = (20.0, -35.0, 10.0);
        assert_eq!(degrees3(rotation3(&o)), (20.0, -35.0, 10.0));
    }

    #[test]
    fn cena_3d_ida_e_volta() {
        let mut scene = SceneData::new_3d();
        scene.set_env("fog", Prop::Bool(false));
        scene.set_env("sun_height", Prop::Num(30.0));
        let text = write(&scene);
        assert!(text.contains("mode = \"3d\""), "{text}");
        assert!(text.contains("z = 10, rx = -22"), "{text}");
        let back = read(&text, "3d.cena").unwrap();
        assert!(back.three_d);
        assert_eq!(back, scene);
        assert_eq!(back.env("fog"), Prop::Bool(false));
        assert_eq!(back.env("sun_height"), Prop::Num(30.0));
        assert_eq!(write(&back), text);
        // Uma cena 2D não grava nada de 3D.
        let two = SceneData::default();
        let t2 = write(&two);
        assert!(!t2.contains("mode") && !t2.contains("environment"), "{t2}");
        assert!(!read(&t2, "2d").unwrap().three_d);
    }

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
        assert_eq!(s.order.len(), 25);
        assert_eq!(
            s.default_of("text3d", "facing"),
            Some(&Prop::Str("camera".into()))
        );
        assert_eq!(s.shapes3d.len(), 6);
        assert_eq!(s.environment[0], ("sky".to_string(), Prop::Bool(true)));
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
    fn uma_cena_da_0_7_regravada_sai_igual() {
        // Como o NN Studio 0.7.0 gravava um corpo e partículas (sem os
        // campos que vieram depois: ox, oy, pixel).
        let text = r#"return {
  objects = {
    {
      id = 1, name = "chao",
      x = 0, y = 0, rotation = 0, scale_x = 1, scale_y = 1,
      visible = true, layer = 0,
      components = {
        { type = "body", body = "static", shape = "box", w = 32, h = 32, radius = 16, sensor = false, one_way = false, bounce = 0, gravity_scale = 1, tag = "" },
      },
    },
  },
}
"#;
        let s = read(text, "a.cena").unwrap();
        let back = write(&s);
        assert!(!back.contains("ox ="), "{back}");
        let body = s.objects[0].components[0].clone();
        assert_eq!(body.num("ox"), 0.0);
        // Mudado, vai para o arquivo.
        let mut s2 = s.clone();
        s2.objects[0].components[0].set("oy", Prop::Num(4.0));
        assert!(write(&s2).contains("oy = 4"));
        let line = |t: &str| {
            t.lines()
                .find(|l| l.contains("type = \"body\""))
                .unwrap()
                .trim()
                .to_string()
        };
        assert_eq!(line(&back), line(text));
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
    fn eventos_da_cena_e_do_objeto_ida_e_volta() {
        let cat = crate::events::catalog();
        let mut enc = cat.condition("encostar").unwrap().new_item(false);
        enc.set("em", Prop::Str("chave".into()));
        let mut tecla = cat.condition("tecla").unwrap().new_item(false);
        tecla.not = true;
        let mut var = cat.action("variavel").unwrap().new_item(false);
        var.set("valor", Prop::Str("{pontos}".into()));
        let mut s = SceneData {
            events: vec![
                Rule {
                    comment: Some("Chaves abrem a porta".into()),
                    ..Default::default()
                },
                Rule {
                    when: vec![enc, tecla],
                    actions: vec![var],
                    repeat: true,
                    off: true,
                    comment: None,
                },
            ],
            ..Default::default()
        };
        let mut o = Object::new(1, "porta");
        o.events = vec![Rule {
            when: vec![cat.condition("comecou").unwrap().new_item(true)],
            actions: vec![cat.action("destruir").unwrap().new_item(true)],
            ..Default::default()
        }];
        s.objects.push(o);
        let text = write(&s);
        assert!(
            text.contains("{ comment = \"Chaves abrem a porta\" },"),
            "{text}"
        );
        assert!(
            text.contains("mode = \"enquanto\", off = true, when = {"),
            "{text}"
        );
        assert!(
            text.contains(
                "{ cond = \"tecla\", nao = true, modo = \"apertar\", tecla = \"pular\" },"
            ),
            "{text}"
        );
        let back = read(&text, "teste").unwrap();
        assert_eq!(back, s);
        assert_eq!(write(&back), text);
        // O JSON do ajuste ao vivo tem o formato que a engine lê.
        let json = rules_json(&s.events).to_string();
        assert!(json.contains("\"mode\":\"enquanto\""), "{json}");
        assert!(json.contains("\"cond\":\"encostar\""), "{json}");
    }

    #[test]
    fn instancia_de_prefab_usa_as_regras_do_prefab() {
        let mut base = Object::new(1, "moeda");
        base.events = vec![Rule {
            comment: Some("do prefab".into()),
            ..Default::default()
        }];
        let mut inst = Object::new(5, "moeda");
        inst.prefab = Some(PrefabLink {
            path: "prefabs/moeda.prefab".into(),
            base: None,
            raw: Vec::new(),
        });
        inst.link_prefab(base);
        assert_eq!(inst.events.len(), 1);
        let s = SceneData {
            objects: vec![inst],
            ..Default::default()
        };
        // A instância não grava as regras (elas são do prefab).
        assert!(!write(&s).contains("events"));
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
