//! Catálogo dos eventos "Quando... Então..." (condições e ações), lido do
//! próprio módulo da engine (`nn_script::EVENTS_RUNTIME`), e o que o editor
//! faz com ele: frases, valores padrão e o "Ver como código".

use std::sync::OnceLock;

use mlua::{Lua, Table, Value};

use crate::behaviors::{PropSpec, PropType, read_props};
use crate::scene_data::{EventItem, Prop, Rule, format_num};

/// Gatilho (acontece num instante) ou estado (vale por um tempo)?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    No,
    Yes,
    /// Depende de um campo (tecla apertada é gatilho; segurando, não).
    Depends,
}

/// Uma condição ou ação do catálogo.
#[derive(Debug, Clone, PartialEq)]
pub struct EventDef {
    pub id: String,
    /// Nome curto, para os menus ("Encostar em").
    pub name: String,
    pub category: String,
    /// Frase com `{campos}`.
    pub text: String,
    pub help: String,
    /// Modelo do Lua equivalente.
    pub lua: String,
    pub trigger: Trigger,
    pub props: Vec<PropSpec>,
    /// Só numa cena "2d" ou "3d" (vazio = nas duas).
    pub mode: String,
    /// Frase e ajuda numa cena 3D, quando são outras.
    pub text3d: String,
    pub help3d: String,
}

thread_local! {
    static THREE_D: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A cena cujos eventos estão sendo mostrados é 3D? Muda as frases, os
/// padrões e o que aparece nos menus. (Quem desenha uma cena chama antes.)
pub fn set_three_d(on: bool) {
    THREE_D.with(|c| c.set(on));
}

pub fn three_d() -> bool {
    THREE_D.with(|c| c.get())
}

/// Pedaço de uma frase: texto ou o campo que vai no meio dela.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    Text(String),
    Field(String),
}

impl EventDef {
    pub fn prop(&self, key: &str) -> Option<&PropSpec> {
        self.props.iter().find(|p| p.key == key)
    }

    /// Vale na cena aberta (2D ou 3D)?
    pub fn fits(&self) -> bool {
        crate::behaviors::mode_fits(&self.mode, three_d())
    }

    /// A ajuda (a do 3D numa cena 3D, se houver).
    pub fn help_text(&self) -> &str {
        if three_d() && !self.help3d.is_empty() {
            &self.help3d
        } else {
            &self.help
        }
    }

    /// As propriedades que valem na cena aberta.
    pub fn props_now(&self) -> impl Iterator<Item = &PropSpec> {
        let three_d = three_d();
        self.props.iter().filter(move |p| p.fits(three_d))
    }

    /// A frase em pedaços.
    pub fn pieces(&self) -> Vec<Piece> {
        let mut out = Vec::new();
        let mut rest = if three_d() && !self.text3d.is_empty() {
            self.text3d.as_str()
        } else {
            self.text.as_str()
        };
        while let Some(i) = rest.find('{') {
            let Some(j) = rest[i..].find('}').map(|j| j + i) else {
                break;
            };
            if i > 0 {
                out.push(Piece::Text(rest[..i].to_string()));
            }
            out.push(Piece::Field(rest[i + 1..j].to_string()));
            rest = &rest[j + 1..];
        }
        if !rest.is_empty() {
            out.push(Piece::Text(rest.to_string()));
        }
        out
    }

    /// A frase com "…" no lugar dos campos (para os menus).
    pub fn menu_text(&self) -> String {
        self.pieces()
            .iter()
            .map(|p| match p {
                Piece::Text(t) => t.clone(),
                Piece::Field(_) => "…".to_string(),
            })
            .collect()
    }

    /// Item novo, com os valores padrão. Numa regra da cena, "este objeto"
    /// vira "o jogador".
    pub fn new_item(&self, in_object: bool) -> EventItem {
        EventItem {
            id: self.id.clone(),
            not: false,
            props: self
                .props_now()
                .map(|p| {
                    let v = match (p.default_for(three_d()), p.kind) {
                        (Prop::Str(s), PropType::Target) if s == "@este" && !in_object => {
                            Prop::Str("@jogador".into())
                        }
                        (d, _) => d,
                    };
                    (p.key.clone(), v)
                })
                .collect(),
        }
    }

    /// É gatilho com estes valores?
    pub fn is_trigger(&self, item: &EventItem) -> bool {
        match self.trigger {
            Trigger::Yes => true,
            Trigger::No => false,
            Trigger::Depends => item.str("modo") != "segurar",
        }
    }
}

/// Condições e ações.
#[derive(Debug, Default)]
pub struct Catalog {
    pub conditions: Vec<EventDef>,
    pub actions: Vec<EventDef>,
}

impl Catalog {
    pub fn condition(&self, id: &str) -> Option<&EventDef> {
        self.conditions.iter().find(|d| d.id == id)
    }

    pub fn action(&self, id: &str) -> Option<&EventDef> {
        self.actions.iter().find(|d| d.id == id)
    }

    pub fn get(&self, condition: bool, id: &str) -> Option<&EventDef> {
        if condition {
            self.condition(id)
        } else {
            self.action(id)
        }
    }

    /// Categorias, na ordem do catálogo.
    pub fn categories(list: &[EventDef]) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for d in list {
            if !out.contains(&d.category.as_str()) {
                out.push(&d.category);
            }
        }
        out
    }
}

fn opt_str(t: &Table, key: &str) -> String {
    t.get::<Option<String>>(key)
        .ok()
        .flatten()
        .unwrap_or_default()
}

fn read_list(t: &Table) -> Vec<EventDef> {
    let mut out = Vec::new();
    for d in t.sequence_values::<Table>().filter_map(Result::ok) {
        let trigger = match d.get::<Value>("trigger") {
            Ok(Value::Boolean(true)) => Trigger::Yes,
            Ok(Value::String(_)) => Trigger::Depends,
            _ => Trigger::No,
        };
        out.push(EventDef {
            id: opt_str(&d, "id"),
            name: opt_str(&d, "name"),
            category: opt_str(&d, "category"),
            text: opt_str(&d, "text"),
            help: opt_str(&d, "help"),
            lua: opt_str(&d, "lua"),
            trigger,
            props: d
                .get::<Option<Table>>("props")
                .ok()
                .flatten()
                .map(|p| read_props(&p))
                .unwrap_or_default(),
            mode: opt_str(&d, "mode"),
            text3d: opt_str(&d, "text3d"),
            help3d: opt_str(&d, "help3d"),
        });
    }
    out
}

/// O catálogo, lido uma vez do módulo da engine.
pub fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let lua = Lua::new();
        let module: Table = lua
            .load(nn_script::SCENE_RUNTIME)
            .eval()
            .expect("módulo das cenas da engine");
        let events: Table = lua
            .load(nn_script::EVENTS_RUNTIME)
            .call(module)
            .expect("módulo dos eventos da engine");
        let cat: Table = events
            .get::<mlua::Function>("catalog")
            .and_then(|f| f.call(()))
            .expect("catálogo dos eventos");
        Catalog {
            conditions: cat
                .get::<Table>("conditions")
                .map(|t| read_list(&t))
                .unwrap_or_default(),
            actions: cat
                .get::<Table>("actions")
                .map(|t| read_list(&t))
                .unwrap_or_default(),
        }
    })
}

/// Põe os campos na ordem do catálogo (os desconhecidos vão no fim, em
/// ordem alfabética), para o arquivo ficar estável.
pub fn sort_props(id: &str, condition: bool, props: &mut [(String, Prop)]) {
    let order: Vec<String> = catalog()
        .get(condition, id)
        .map(|d| d.props.iter().map(|p| p.key.clone()).collect())
        .unwrap_or_default();
    props.sort_by(|a, b| {
        let ia = order.iter().position(|k| *k == a.0).unwrap_or(usize::MAX);
        let ib = order.iter().position(|k| *k == b.0).unwrap_or(usize::MAX);
        ia.cmp(&ib).then_with(|| a.0.cmp(&b.0))
    });
}

/// Texto de um "quem/alvo".
pub fn target_label(v: &str, in_object: bool) -> String {
    match v {
        "@este" if in_object => "este objeto".into(),
        "@este" | "@jogador" | "" => "o jogador".into(),
        "@outro" => "o outro".into(),
        name => name.to_string(),
    }
}

/// Valor de um campo como texto curto, para frases.
pub fn value_text(spec: Option<&PropSpec>, v: &Prop, in_object: bool) -> String {
    let Some(spec) = spec else {
        return match v {
            Prop::Num(n) => format_num(*n),
            Prop::Bool(b) => if *b { "sim" } else { "não" }.into(),
            Prop::Str(s) => s.clone(),
        };
    };
    match spec.kind {
        PropType::Target => target_label(v.str(), in_object),
        PropType::Choice => {
            let s = match v {
                Prop::Str(s) => s.clone(),
                other => value_text(None, other, in_object),
            };
            spec.options
                .iter()
                .find(|(k, _)| *k == s)
                .map(|(_, l)| l.clone())
                .unwrap_or(s)
        }
        PropType::Scene | PropType::Prefab | PropType::Sound | PropType::Image => {
            let s = v.str();
            if s.is_empty() {
                "(nenhum)".into()
            } else {
                s.rsplit('/').next().unwrap_or(s).to_string()
            }
        }
        PropType::Tile => match v.num() as i64 {
            0 => "nenhum (apaga)".into(),
            n => n.to_string(),
        },
        PropType::Var | PropType::Object | PropType::TileType if v.str().is_empty() => spec
            .allow_empty
            .clone()
            .unwrap_or_else(|| "(nenhum)".into()),
        _ => value_text(None, v, in_object),
    }
}

/// A frase de um item com os valores ("o jogador encostar em chave").
pub fn sentence(def: &EventDef, item: &EventItem, in_object: bool) -> String {
    let mut s = String::new();
    for p in def.pieces() {
        match p {
            Piece::Text(t) => s.push_str(&t),
            Piece::Field(k) => {
                let v = item.get(&k).cloned().unwrap_or(Prop::Str(String::new()));
                s.push_str(&value_text(def.prop(&k), &v, in_object));
            }
        }
    }
    s
}

/// Uma regra numa linha ("Quando ..., então ...").
pub fn rule_summary(rule: &Rule, in_object: bool) -> String {
    if let Some(c) = &rule.comment
        && rule.when.is_empty()
        && rule.actions.is_empty()
    {
        return format!("# {c}");
    }
    let cat = catalog();
    let when: Vec<String> = rule
        .when
        .iter()
        .map(|i| match cat.condition(&i.id) {
            Some(d) => {
                let s = sentence(d, i, in_object);
                if i.not { format!("não: {s}") } else { s }
            }
            None => format!("({})", i.id),
        })
        .collect();
    let actions: Vec<String> = rule
        .actions
        .iter()
        .map(|i| match cat.action(&i.id) {
            Some(d) => sentence(d, i, in_object),
            None => format!("({})", i.id),
        })
        .collect();
    format!(
        "{} {}, então {}",
        if rule.repeat { "Enquanto" } else { "Quando" },
        when.join(" e "),
        actions.join(" e ")
    )
}

// Ver como código ---------------------------------------------------------

fn lua_target(v: &str, in_object: bool) -> String {
    match v {
        "@este" if in_object => "self.obj".into(),
        "@este" | "@jogador" | "" => "jogador".into(),
        "@outro" => "outro".into(),
        name => {
            let scene = if in_object { "self.scene" } else { "self" };
            format!("{scene}:find({})", crate::config_lua::lua_string(name))
        }
    }
}

fn fill_template(def: &EventDef, item: &EventItem, in_object: bool) -> String {
    let mut out = String::new();
    let mut rest = def.lua.as_str();
    while let Some(i) = rest.find('{') {
        let Some(j) = rest[i..].find('}').map(|j| j + i) else {
            break;
        };
        out.push_str(&rest[..i]);
        let key = &rest[i + 1..j];
        let v = item.get(key).cloned().unwrap_or(Prop::Str(String::new()));
        let text = match def.prop(key).map(|p| p.kind) {
            Some(PropType::Target) => lua_target(v.str(), in_object),
            _ => match &v {
                Prop::Num(n) => format_num(*n),
                Prop::Bool(b) => b.to_string(),
                Prop::Str(s) => s.clone(),
            },
        };
        out.push_str(&text);
        rest = &rest[j + 1..];
    }
    out.push_str(rest);
    out
}

/// Lua aproximado equivalente às regras (para aprender; não é usado pelo
/// jogo). `owner` é o nome do objeto, ou `None` para as regras da cena.
pub fn to_lua(rules: &[Rule], owner: Option<&str>) -> String {
    let in_object = owner.is_some();
    let cat = catalog();
    let mut s = String::new();
    match owner {
        Some(name) => {
            s.push_str(&format!(
                "-- Eventos de \"{name}\" em Lua (aproximado: mostra a ideia).\n\
                 -- Num comportamento, self.obj é o objeto e self.scene é a cena.\n\
                 local B = {{}}\n\nfunction B:update(dt)\n"
            ));
            s.push_str("  local jogador = self.scene:targets(\"\")[1]\n");
        }
        None => {
            s.push_str(
                "-- Eventos da cena em Lua (aproximado: mostra a ideia).\n\
                 -- No script da cena, self é a cena e self.objects tem os objetos.\n\
                 local cena = {}\n\nfunction cena:update(dt)\n",
            );
            s.push_str("  local jogador = self:targets(\"\")[1]\n");
        }
    }
    for rule in rules {
        s.push('\n');
        if let Some(c) = &rule.comment {
            s.push_str(&format!("  -- {c}\n"));
            if rule.when.is_empty() && rule.actions.is_empty() {
                continue;
            }
        }
        let mut block = String::new();
        block.push_str(&format!("-- {}\n", rule_summary(rule, in_object)));
        if rule.repeat {
            block.push_str("-- (Enquanto: a cada quadro)\n");
        } else if !rule
            .when
            .iter()
            .any(|i| cat.condition(&i.id).is_some_and(|d| d.is_trigger(i)))
        {
            block.push_str("-- (Quando: só no momento em que passa a valer)\n");
        }
        let mut conds = Vec::new();
        for c in &rule.when {
            let Some(d) = cat.condition(&c.id) else {
                continue;
            };
            let code = fill_template(d, c, in_object);
            if code.trim_start().starts_with("--") {
                block.push_str(&format!("{code}\n"));
                continue;
            }
            conds.push(if c.not { format!("not ({code})") } else { code });
        }
        let cond = if conds.is_empty() {
            "true".to_string()
        } else {
            conds.join(" and ")
        };
        block.push_str(&format!("if {cond} then\n"));
        for a in &rule.actions {
            let Some(d) = cat.action(&a.id) else {
                continue;
            };
            block.push_str(&format!("  {}\n", fill_template(d, a, in_object)));
        }
        block.push_str("end\n");
        for line in block.lines() {
            if rule.off {
                s.push_str(&format!("  -- (desligada) {line}\n"));
            } else {
                s.push_str(&format!("  {line}\n"));
            }
        }
    }
    s.push_str("end\n\n");
    s.push_str(if in_object {
        "return B\n"
    } else {
        "return cena\n"
    });
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numa_cena_3d_frases_padroes_e_menus_mudam() {
        let cat = catalog();
        set_three_d(true);
        // Só do 2D: some do menu.
        assert!(!cat.condition("tile").unwrap().fits());
        assert!(cat.condition("perto").unwrap().fits());
        // Velocidade: com z, em metros, e a frase mostra os três.
        let vel = cat.action("velocidade").unwrap();
        let item = vel.new_item(false);
        assert_eq!(item.get("vy"), Some(&Prop::Num(10.0)));
        assert_eq!(item.get("vz"), Some(&Prop::Num(0.0)));
        assert!(vel.pieces().contains(&Piece::Field("vz".into())));
        assert_eq!(
            cat.condition("perto")
                .unwrap()
                .new_item(false)
                .get("distancia"),
            Some(&Prop::Num(3.0))
        );
        set_three_d(false);
        let item = vel.new_item(false);
        assert_eq!(item.get("vy"), Some(&Prop::Num(600.0)));
        assert_eq!(item.get("vz"), None);
        assert!(!vel.pieces().contains(&Piece::Field("vz".into())));
        assert!(cat.condition("tile").unwrap().fits());
    }

    #[test]
    fn catalogo_tem_condicoes_e_acoes_com_frases() {
        let cat = catalog();
        assert!(cat.conditions.len() >= 15);
        assert!(cat.actions.len() >= 20);
        let enc = cat.condition("encostar").unwrap();
        assert_eq!(enc.trigger, Trigger::Yes);
        assert_eq!(
            enc.pieces(),
            vec![
                Piece::Field("quem".into()),
                Piece::Text(" encostar em ".into()),
                Piece::Field("em".into())
            ]
        );
        assert_eq!(enc.menu_text(), "… encostar em …");
        assert_eq!(enc.prop("quem").unwrap().kind, PropType::Target);
        let tecla = cat.condition("tecla").unwrap();
        assert_eq!(tecla.trigger, Trigger::Depends);
        let mut item = tecla.new_item(false);
        assert!(tecla.is_trigger(&item));
        item.set("modo", Prop::Str("segurar".into()));
        assert!(!tecla.is_trigger(&item));
        // Na regra da cena, "este objeto" vira "o jogador".
        assert_eq!(enc.new_item(false).str("quem"), "@jogador");
        assert_eq!(enc.new_item(true).str("quem"), "@este");
    }

    #[test]
    fn frase_resumo_e_codigo() {
        let cat = catalog();
        let mut c = cat.condition("encostar").unwrap().new_item(false);
        c.set("em", Prop::Str("chave".into()));
        let mut a = cat.action("variavel").unwrap().new_item(false);
        a.set("var", Prop::Str("chaves".into()));
        let d = cat.action("destruir").unwrap();
        let mut b = d.new_item(false);
        b.set("alvo", Prop::Str("@outro".into()));
        let rule = Rule {
            when: vec![c],
            actions: vec![a, b],
            ..Default::default()
        };
        assert_eq!(
            rule_summary(&rule, false),
            "Quando o jogador encostar em chave, então variável chaves: somar 1 e destruir o outro"
        );
        let code = to_lua(&[rule], None);
        assert!(
            code.contains("if jogador:touching(self:find(\"chave\"))"),
            "{code}"
        );
        assert!(code.contains("outro:destroy()"), "{code}");
        assert!(code.ends_with("return cena\n"));
    }
}
