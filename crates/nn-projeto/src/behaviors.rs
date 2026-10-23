//! Comportamentos para o Inspetor: os prontos da engine e os scripts do
//! jogo. As informações (título, descrição, propriedades) vêm do próprio
//! script, lidas pelo mesmo código da engine (`describe` do módulo das cenas
//! visuais), sem rodar nada do jogo.

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;
use std::time::SystemTime;

use mlua::{Lua, Table, Value};

use crate::scene_data::Prop;

/// Tipo de uma propriedade (decide o controle no Inspetor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropType {
    Number,
    Bool,
    Text,
    Color,
    Choice,
    /// Nome de um objeto da cena.
    Object,
    Scene,
    Prefab,
    Sound,
    Image,
    /// Quem/alvo de um evento: "@este", "@outro", "@jogador" ou um nome.
    Target,
    /// Variável do jogo.
    Var,
    /// Número, texto ou `{variavel}`.
    Value,
    /// Comportamento ("plataforma", ou o caminho do script).
    Behavior,
    /// Nome de uma animação da imagem (`.anim`).
    Anim,
    /// Um osso do modelo 3D do objeto ("mao_d").
    Bone,
    /// Um tile do mapa (número; 0 = nenhum).
    Tile,
    /// Um arquivo de diálogo (`.dialogo`).
    Dialogue,
    /// O tipo de um tile (`tipo` no tileset: "lava", "agua"...).
    TileType,
}

impl PropType {
    fn parse(s: &str) -> Self {
        match s {
            "number" => Self::Number,
            "bool" => Self::Bool,
            "color" => Self::Color,
            "choice" => Self::Choice,
            "object" => Self::Object,
            "scene" => Self::Scene,
            "prefab" => Self::Prefab,
            "sound" => Self::Sound,
            "image" => Self::Image,
            "target" => Self::Target,
            "var" => Self::Var,
            "value" => Self::Value,
            "behavior" => Self::Behavior,
            "anim" => Self::Anim,
            "bone" => Self::Bone,
            "tile" => Self::Tile,
            "tiletype" => Self::TileType,
            "dialogue" => Self::Dialogue,
            _ => Self::Text,
        }
    }
}

/// Uma propriedade declarada pelo script (`props = { { "velocidade", 120 } }`).
#[derive(Debug, Clone, PartialEq)]
pub struct PropSpec {
    pub key: String,
    pub default: Prop,
    pub kind: PropType,
    pub label: String,
    pub help: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub integer: bool,
    /// Valor e texto de cada opção.
    pub options: Vec<(String, String)>,
    pub advanced: bool,
    /// Só aparece quando outra propriedade tem um destes valores.
    pub show_if: Option<(String, Vec<String>)>,
    /// Pode ficar vazio; o texto é o que mostrar nesse caso ("todas").
    pub allow_empty: Option<String>,
    /// Só numa cena "2d" ou "3d" (vazio = nas duas).
    pub mode: String,
    /// Padrão numa cena 3D, quando é outro (metros em vez de pixels).
    pub default3d: Option<Prop>,
}

impl PropSpec {
    /// A propriedade vale numa cena 3D (`true`) ou 2D?
    pub fn fits(&self, three_d: bool) -> bool {
        mode_fits(&self.mode, three_d)
    }

    pub fn default_for(&self, three_d: bool) -> Prop {
        match (&self.default3d, three_d) {
            (Some(d), true) => d.clone(),
            _ => self.default.clone(),
        }
    }
}

/// `mode` de um comportamento, evento ou propriedade: "2d", "3d" ou nas
/// duas ("both" ou vazio).
pub fn mode_fits(mode: &str, three_d: bool) -> bool {
    match mode {
        "2d" => !three_d,
        "3d" => three_d,
        _ => true,
    }
}

/// Um comportamento: o script e o que ele declara.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BehaviorInfo {
    /// `"nn:plataforma"` ou o caminho do script.
    pub script: String,
    pub title: String,
    pub description: String,
    pub category: String,
    /// Componentes de que precisa ("body", "sprite", "text").
    pub requires: Vec<String>,
    /// É um comportamento de jogador (os outros procuram por ele).
    pub player: bool,
    /// Cena em que funciona: "2d", "3d" ou "both". Os prontos sem `mode`
    /// são do 2D; os do jogo valem nas duas.
    pub mode: String,
    /// Tipo do Corpo 3D que ganha ao ser adicionado (se precisar de um).
    pub body: String,
    pub props: Vec<PropSpec>,
    /// Erro ao ler o script.
    pub error: Option<String>,
}

impl BehaviorInfo {
    pub fn is_builtin(&self) -> bool {
        self.script.starts_with("nn:")
    }

    pub fn prop(&self, key: &str) -> Option<&PropSpec> {
        self.props.iter().find(|p| p.key == key)
    }

    /// Valores padrão de todas as propriedades.
    /// Funciona numa cena 3D (`true`) ou 2D?
    pub fn fits(&self, three_d: bool) -> bool {
        mode_fits(&self.mode, three_d)
    }

    pub fn defaults(&self) -> Vec<(String, Prop)> {
        self.props
            .iter()
            .map(|p| (p.key.clone(), p.default.clone()))
            .collect()
    }

    /// Os valores iniciais numa cena 2D ou 3D (só as propriedades que valem
    /// nela, com o padrão dela).
    pub fn defaults_for(&self, three_d: bool) -> Vec<(String, Prop)> {
        self.props
            .iter()
            .filter(|p| p.fits(three_d))
            .map(|p| (p.key.clone(), p.default_for(three_d)))
            .collect()
    }
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

fn opt_num(t: &Table, key: &str) -> Option<f64> {
    match t.get::<Value>(key).ok()? {
        Value::Integer(i) => Some(i as f64),
        Value::Number(n) => Some(n),
        _ => None,
    }
}

fn opt_str(t: &Table, key: &str) -> String {
    t.get::<Option<String>>(key)
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// Lê a lista de propriedades já normalizada pela engine
/// (`normalize_props`): `{ key, default, type, label, ... }`.
pub fn read_props(props: &Table) -> Vec<PropSpec> {
    let mut out = Vec::new();
    for p in props.sequence_values::<Table>().filter_map(Result::ok) {
        let key = opt_str(&p, "key");
        let default = p
            .get::<Value>("default")
            .ok()
            .and_then(|v| to_prop(&v))
            .unwrap_or(Prop::Str(String::new()));
        let mut options = Vec::new();
        if let Ok(Some(list)) = p.get::<Option<Table>>("options") {
            for o in list.sequence_values::<Value>().filter_map(Result::ok) {
                match o {
                    Value::String(s) => {
                        let s = s.to_string_lossy();
                        options.push((s.clone(), s));
                    }
                    Value::Table(pair) => {
                        let v: String = pair.get(1).unwrap_or_default();
                        let l: String = pair.get(2).unwrap_or_else(|_| v.clone());
                        options.push((v, l));
                    }
                    _ => {}
                }
            }
        }
        let show_if = p
            .get::<Option<Table>>("show_if")
            .ok()
            .flatten()
            .and_then(|t| {
                // Os valores como o Inspetor compara (true, 2, "atras").
                let mut it = t
                    .sequence_values::<Value>()
                    .filter_map(Result::ok)
                    .map(|v| match v {
                        Value::String(s) => s.to_string_lossy(),
                        Value::Boolean(b) => b.to_string(),
                        Value::Integer(n) => n.to_string(),
                        Value::Number(n) => crate::scene_data::format_num(n),
                        _ => String::new(),
                    });
                let key = it.next()?;
                Some((key, it.collect()))
            });
        let label = opt_str(&p, "label");
        let flag = |k: &str| p.get::<Option<bool>>(k).ok().flatten().unwrap_or(false);
        let allow_empty = match p.get::<Value>("allow_empty") {
            Ok(Value::String(s)) => Some(s.to_string_lossy()),
            _ => None,
        };
        let default3d = p.get::<Value>("default3d").ok().and_then(|v| to_prop(&v));
        out.push(PropSpec {
            mode: opt_str(&p, "mode"),
            default3d,
            label: if label.is_empty() { key.clone() } else { label },
            kind: PropType::parse(&opt_str(&p, "type")),
            help: opt_str(&p, "help"),
            min: opt_num(&p, "min"),
            max: opt_num(&p, "max"),
            integer: flag("integer"),
            advanced: flag("advanced"),
            options,
            show_if,
            allow_empty,
            key,
            default,
        });
    }
    out
}

fn describe_with(describe: &mlua::Function, src: &str, script: &str) -> BehaviorInfo {
    let mut info = BehaviorInfo {
        script: script.to_string(),
        ..Default::default()
    };
    let result: mlua::Result<Table> = describe.call((src, script));
    let t = match result {
        Ok(t) => t,
        Err(e) => {
            info.error = Some(nn_script::translate_error(e.to_string().trim()));
            return info;
        }
    };
    if let Some(e) = t.get::<Option<String>>("error").ok().flatten() {
        info.error = Some(nn_script::translate_error(e.trim()));
        return info;
    }
    info.title = opt_str(&t, "title");
    info.description = opt_str(&t, "description");
    info.category = opt_str(&t, "category");
    info.player = t
        .get::<Option<bool>>("player")
        .ok()
        .flatten()
        .unwrap_or(false);
    info.mode = opt_str(&t, "mode");
    if info.mode.is_empty() && script.starts_with("nn:") {
        info.mode = "2d".into();
    }
    info.body = opt_str(&t, "body");
    if let Ok(Some(req)) = t.get::<Option<Table>>("requires") {
        info.requires = req
            .sequence_values::<String>()
            .filter_map(Result::ok)
            .collect();
    }
    if let Ok(Some(props)) = t.get::<Option<Table>>("props") {
        info.props = read_props(&props);
    }
    info
}

fn describer() -> (Lua, mlua::Function) {
    let lua = crate::quiet_lua();
    let module: Table = lua
        .load(nn_script::SCENE_RUNTIME)
        .eval()
        .expect("módulo das cenas da engine");
    let describe = module.get("describe").expect("describe");
    (lua, describe)
}

/// Os comportamentos prontos, na ordem da engine.
pub fn builtins() -> &'static [BehaviorInfo] {
    static LIST: OnceLock<Vec<BehaviorInfo>> = OnceLock::new();
    LIST.get_or_init(|| {
        let (_lua, describe) = describer();
        nn_script::BEHAVIORS
            .iter()
            .map(|(id, src)| describe_with(&describe, src, &format!("nn:{id}")))
            .collect()
    })
}

/// Categorias dos prontos, na ordem em que aparecem no menu.
pub fn categories() -> Vec<&'static str> {
    let mut out: Vec<&str> = Vec::new();
    for b in builtins() {
        if !out.contains(&b.category.as_str()) {
            out.push(&b.category);
        }
    }
    out
}

/// Lê (e guarda) as informações dos comportamentos usados no projeto.
#[derive(Default)]
pub struct Catalog {
    user: HashMap<String, (Option<SystemTime>, BehaviorInfo)>,
    lua: Option<(Lua, mlua::Function)>,
}

impl std::fmt::Debug for Catalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Catalog")
            .field("user", &self.user.len())
            .finish()
    }
}

impl Catalog {
    /// Informações de um comportamento (`"nn:..."` ou caminho no projeto).
    /// Scripts do jogo são relidos quando mudam.
    pub fn get(&mut self, root: &Path, script: &str) -> BehaviorInfo {
        if let Some(id) = script.strip_prefix("nn:") {
            return builtins()
                .iter()
                .find(|b| b.script == script)
                .cloned()
                .unwrap_or_else(|| BehaviorInfo {
                    script: script.to_string(),
                    title: id.to_string(),
                    error: Some(format!("comportamento pronto desconhecido: {script}")),
                    ..Default::default()
                });
        }
        let path = root.join(script);
        let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        if let Some((m, info)) = self.user.get(script)
            && *m == modified
        {
            return info.clone();
        }
        let info = match std::fs::read_to_string(&path) {
            Ok(src) => {
                let (_, describe) = self.lua.get_or_insert_with(describer);
                let mut info = describe_with(describe, &src, script);
                if info.title.is_empty() {
                    info.title = Path::new(script)
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| script.to_string());
                }
                info
            }
            Err(e) => BehaviorInfo {
                script: script.to_string(),
                title: script.to_string(),
                error: Some(format!("não foi possível ler {script}: {e}")),
                ..Default::default()
            },
        };
        self.user
            .insert(script.to_string(), (modified, info.clone()));
        info
    }
}

/// Script novo de comportamento.
pub fn template(name: &str) -> String {
    format!(
        "-- Comportamento \"{name}\". Ponha em objetos pelo Inspetor:\n\
         -- Adicionar componente > Script do jogo.\n\
         local B = {{\n\
         \x20 title = \"{name}\",\n\
         \x20 description = \"O que este comportamento faz.\",\n\
         \x20 -- Propriedades: aparecem no Inspetor e ficam em self.<nome>.\n\
         \x20 props = {{\n\
         \x20   {{ \"velocidade\", 100, label = \"Velocidade\", min = 0 }},\n\
         \x20 }},\n\
         }}\n\
         \n\
         -- Começou (todos os objetos da cena já existem). self.obj é o objeto.\n\
         function B:start()\n\
         end\n\
         \n\
         -- Uma vez por quadro.\n\
         function B:update(dt)\n\
         \x20 -- Exemplo: girar.\n\
         \x20 self.obj.rotation = self.obj.rotation + self.velocidade * dt\n\
         end\n\
         \n\
         -- Passos fixos (física): function B:fixed_update(dt) end\n\
         -- Tocou outro corpo físico: function B:contact(outro, e) end\n\
         -- Vai sumir: function B:destroyed() end\n\
         \n\
         return B\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prontos_tem_titulo_categoria_e_propriedades() {
        let list = builtins();
        assert_eq!(list.len(), nn_script::BEHAVIORS.len());
        for b in list {
            assert_eq!(b.error, None, "{}", b.script);
            assert!(
                !b.title.is_empty() && !b.category.is_empty(),
                "{}",
                b.script
            );
        }
        let plataforma = list.iter().find(|b| b.script == "nn:plataforma").unwrap();
        assert!(plataforma.player);
        assert_eq!(plataforma.requires, vec!["body".to_string()]);
        let pulos = plataforma.prop("pulos_no_ar").unwrap();
        assert!(pulos.integer);
        assert_eq!(pulos.default, Prop::Num(0.0));
        let vida = list.iter().find(|b| b.script == "nn:vida").unwrap();
        let acabar = vida.prop("ao_acabar").unwrap();
        assert_eq!(acabar.kind, PropType::Choice);
        assert_eq!(acabar.options[0].0, "reiniciar");
        let cena = vida.prop("cena").unwrap();
        assert_eq!(cena.kind, PropType::Scene);
        assert_eq!(
            cena.show_if,
            Some(("ao_acabar".to_string(), vec!["cena".to_string()]))
        );
        // Mostrar quando um sim/não está ligado (true vira "true").
        let j3 = list.iter().find(|b| b.script == "nn:jogador3d").unwrap();
        assert_eq!(
            j3.prop("anim_andar").unwrap().show_if,
            Some(("animar".to_string(), vec!["true".to_string()]))
        );
        let perigo = list.iter().find(|b| b.script == "nn:perigo").unwrap();
        assert_eq!(perigo.prop("cena").unwrap().kind, PropType::Scene);
        assert!(categories().contains(&"Jogador"));
    }

    #[test]
    fn script_do_jogo_e_relido_quando_muda() {
        let dir = std::env::temp_dir().join(format!("nn-studio-comp-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("scripts")).unwrap();
        std::fs::write(dir.join("scripts/gira.lua"), template("gira")).unwrap();
        let mut cat = Catalog::default();
        let info = cat.get(&dir, "scripts/gira.lua");
        assert_eq!(info.error, None);
        assert_eq!(info.title, "gira");
        assert_eq!(info.prop("velocidade").unwrap().default, Prop::Num(100.0));
        // Forma curta (tabela simples) e erro de sintaxe.
        std::fs::write(
            dir.join("scripts/curto.lua"),
            "return { props = { vida = 3, nome = 'x', cor = '#FF0000', ativo = true } }",
        )
        .unwrap();
        let curto = cat.get(&dir, "scripts/curto.lua");
        let keys: Vec<_> = curto
            .props
            .iter()
            .map(|p| (p.key.as_str(), p.kind))
            .collect();
        assert_eq!(
            keys,
            vec![
                ("ativo", PropType::Bool),
                ("cor", PropType::Color),
                ("nome", PropType::Text),
                ("vida", PropType::Number)
            ]
        );
        std::fs::write(dir.join("scripts/ruim.lua"), "return {").unwrap();
        assert!(cat.get(&dir, "scripts/ruim.lua").error.is_some());
        // Usar a engine fora das funções: erro claro (o editor não roda o jogo).
        std::fs::write(
            dir.join("scripts/nn.lua"),
            "local x = nn.assets.texture('a.png') return {}",
        )
        .unwrap();
        assert!(cat.get(&dir, "scripts/nn.lua").error.is_some());
    }
}
