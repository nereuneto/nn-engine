//! Lê e altera o `config.lua` de um projeto sem perder os comentários nem a
//! organização do arquivo: só os valores mudados são reescritos.

use mlua::{Lua, Table, Value};

use crate::scene_data::Prop;

/// As opções do `config.lua` que o formulário edita.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigValues {
    pub title: String,
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
    pub resizable: bool,
    pub vsync: bool,
    pub hot_reload: bool,
    pub fixed_hz: u32,
    /// Cor de fundo, `#RRGGBB`.
    pub clear_color: String,
    pub start: String,
    /// Variáveis do jogo (`vars`), em ordem alfabética.
    pub vars: Vec<(String, Prop)>,
    /// Tamanho fixo do jogo (`resolution = { 640, 360 }`).
    pub resolution: Option<(u32, u32)>,
    /// Como o tamanho fixo se ajusta à janela: "fit", "stretch" ou "pixel".
    pub scale: String,
}

impl ConfigValues {
    /// O tamanho que a câmera e a tela do jogo enxergam.
    pub fn game_size(&self) -> (u32, u32) {
        self.resolution.unwrap_or((self.width, self.height))
    }
}

impl Default for ConfigValues {
    fn default() -> Self {
        Self {
            title: String::new(),
            id: String::new(),
            width: 1280,
            height: 720,
            fullscreen: false,
            resizable: true,
            vsync: true,
            hot_reload: true,
            fixed_hz: 60,
            clear_color: "#1E1E28".into(),
            start: "main.lua".into(),
            vars: Vec::new(),
            resolution: None,
            scale: "fit".into(),
        }
    }
}

/// Um valor a gravar no `config.lua`.
#[derive(Debug, Clone, PartialEq)]
pub enum LuaValue {
    Str(String),
    Int(i64),
    Bool(bool),
    /// Código Lua pronto (uma tabela, por exemplo).
    Raw(String),
}

impl LuaValue {
    /// Como o valor é escrito no código Lua.
    pub fn to_lua(&self) -> String {
        match self {
            LuaValue::Str(s) => lua_string(s),
            LuaValue::Int(n) => n.to_string(),
            LuaValue::Bool(b) => b.to_string(),
            LuaValue::Raw(s) => s.clone(),
        }
    }
}

/// Texto Lua entre aspas, com escapes.
pub fn lua_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Lê os valores do `config.lua`. Opções ausentes ficam com o padrão
/// (`default_title` para o título).
pub fn read(text: &str, default_title: &str) -> Result<ConfigValues, String> {
    let mut v = ConfigValues {
        title: default_title.to_string(),
        ..ConfigValues::default()
    };
    if text.trim().is_empty() {
        return Ok(v);
    }
    let lua = Lua::new();
    let t: Table = lua
        .load(text)
        .set_name("@config.lua")
        .eval()
        .map_err(|e| nn_script::translate_error(e.to_string().trim()))?;
    let get = |k: &str| t.get::<Value>(k).unwrap_or(Value::Nil);
    if let Value::String(s) = get("title") {
        v.title = s.to_string_lossy();
    }
    if let Value::String(s) = get("id") {
        v.id = s.to_string_lossy();
    }
    if let Value::String(s) = get("start") {
        v.start = s.to_string_lossy();
    }
    if let Value::String(s) = get("clear_color") {
        v.clear_color = s.to_string_lossy();
    }
    // Lua 5.4 separa inteiros (1280) de números com vírgula (1280.0).
    let num = |val: Value| match val {
        Value::Integer(i) => Some(i.max(0) as u32),
        Value::Number(n) => Some(n.max(0.0) as u32),
        _ => None,
    };
    if let Some(n) = num(get("width")) {
        v.width = n;
    }
    if let Some(n) = num(get("height")) {
        v.height = n;
    }
    if let Some(n) = num(get("fixed_hz")) {
        v.fixed_hz = n.max(1);
    }
    if let Value::Table(r) = get("resolution")
        && let (Some(w), Some(h)) = (
            num(r.get::<Value>(1).unwrap_or(Value::Nil)),
            num(r.get::<Value>(2).unwrap_or(Value::Nil)),
        )
        && w > 0
        && h > 0
    {
        v.resolution = Some((w, h));
    }
    if let Value::String(s) = get("scale") {
        v.scale = s.to_string_lossy();
    }
    if let Value::Table(vars) = get("vars") {
        for (k, val) in vars.pairs::<String, Value>().filter_map(Result::ok) {
            let p = match val {
                Value::Integer(i) => Prop::Num(i as f64),
                Value::Number(n) => Prop::Num(n),
                Value::Boolean(b) => Prop::Bool(b),
                Value::String(s) => Prop::Str(s.to_string_lossy()),
                _ => continue,
            };
            v.vars.push((k, p));
        }
        v.vars.sort_by(|a, b| a.0.cmp(&b.0));
    }
    for (key, field) in [
        ("fullscreen", &mut v.fullscreen),
        ("resizable", &mut v.resizable),
        ("vsync", &mut v.vsync),
        ("hot_reload", &mut v.hot_reload),
    ] {
        if let Some(b) = get(key).as_boolean() {
            *field = b;
        }
    }
    Ok(v)
}

/// Diferenças entre dois conjuntos de valores, como (chave, valor novo).
pub fn changes(old: &ConfigValues, new: &ConfigValues) -> Vec<(&'static str, LuaValue)> {
    let mut out = Vec::new();
    let mut s = |k, a: &String, b: &String| {
        if a != b {
            out.push((k, LuaValue::Str(b.clone())));
        }
    };
    s("title", &old.title, &new.title);
    s("id", &old.id, &new.id);
    s("clear_color", &old.clear_color, &new.clear_color);
    s("start", &old.start, &new.start);
    s("scale", &old.scale, &new.scale);
    if old.resolution != new.resolution {
        out.push(("resolution", resolution_lua(new.resolution)));
    }
    let mut n = |k, a: u32, b: u32| {
        if a != b {
            out.push((k, LuaValue::Int(b as i64)));
        }
    };
    n("width", old.width, new.width);
    n("height", old.height, new.height);
    n("fixed_hz", old.fixed_hz, new.fixed_hz);
    let mut b = |k, a: bool, b: bool| {
        if a != b {
            out.push((k, LuaValue::Bool(b)));
        }
    };
    b("fullscreen", old.fullscreen, new.fullscreen);
    b("resizable", old.resizable, new.resizable);
    b("vsync", old.vsync, new.vsync);
    b("hot_reload", old.hot_reload, new.hot_reload);
    if old.vars != new.vars {
        out.push(("vars", LuaValue::Raw(vars_lua(&new.vars))));
    }
    out
}

/// `{ 640, 360 }`, ou `false` sem tamanho fixo.
fn resolution_lua(r: Option<(u32, u32)>) -> LuaValue {
    match r {
        Some((w, h)) => LuaValue::Raw(format!("{{ {w}, {h} }}")),
        None => LuaValue::Bool(false),
    }
}

/// Nome de variável válido no Lua (`moedas`, `vidas_extra`).
pub fn valid_var_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// `{ moedas = 0, vidas = 3 }`.
pub fn vars_lua(vars: &[(String, Prop)]) -> String {
    if vars.is_empty() {
        return "{}".into();
    }
    let mut sorted: Vec<_> = vars.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let items: Vec<String> = sorted
        .iter()
        .map(|(k, v)| format!("{k} = {}", v.to_lua()))
        .collect();
    format!("{{ {} }}", items.join(", "))
}

/// Reescreve o `config.lua` com os valores novos. Cada `chave = valor` que já
/// existe na tabela principal tem só o valor trocado; chaves novas entram no
/// fim da tabela. Se o arquivo for complicado demais para isso (valores
/// calculados, por exemplo), ele é gerado de novo a partir de `all`.
pub fn update(text: &str, changes: &[(&str, LuaValue)], all: &ConfigValues) -> String {
    if changes.is_empty() {
        return text.to_string();
    }
    if let Some(updated) = patch(text, changes) {
        // Confere: o arquivo novo precisa ler exatamente os valores pedidos.
        if let Ok(v) = read(&updated, &all.title)
            && changes
                .iter()
                .all(|(k, val)| value_of(&v, k) == Some(val.clone()))
        {
            return updated;
        }
    }
    generate(all)
}

fn value_of(v: &ConfigValues, key: &str) -> Option<LuaValue> {
    Some(match key {
        "title" => LuaValue::Str(v.title.clone()),
        "id" => LuaValue::Str(v.id.clone()),
        "clear_color" => LuaValue::Str(v.clear_color.clone()),
        "start" => LuaValue::Str(v.start.clone()),
        "width" => LuaValue::Int(v.width as i64),
        "height" => LuaValue::Int(v.height as i64),
        "fixed_hz" => LuaValue::Int(v.fixed_hz as i64),
        "fullscreen" => LuaValue::Bool(v.fullscreen),
        "resizable" => LuaValue::Bool(v.resizable),
        "vsync" => LuaValue::Bool(v.vsync),
        "hot_reload" => LuaValue::Bool(v.hot_reload),
        "vars" => LuaValue::Raw(vars_lua(&v.vars)),
        "resolution" => resolution_lua(v.resolution),
        "scale" => LuaValue::Str(v.scale.clone()),
        _ => return None,
    })
}

/// Um `config.lua` novo com todos os valores.
pub fn generate(v: &ConfigValues) -> String {
    let mut s = String::from(
        "-- Configuração do jogo. Todas as opções estão na documentação da nn-engine\n\
         -- (docs/lua.md, seção \"config.lua\").\nreturn {\n",
    );
    let d = ConfigValues::default();
    let mut line = |k: &str, val: LuaValue| s.push_str(&format!("  {k} = {},\n", val.to_lua()));
    line("title", LuaValue::Str(v.title.clone()));
    if !v.id.is_empty() {
        line("id", LuaValue::Str(v.id.clone()));
    }
    line("width", LuaValue::Int(v.width as i64));
    line("height", LuaValue::Int(v.height as i64));
    if v.resolution.is_some() {
        line("resolution", resolution_lua(v.resolution));
        line("scale", LuaValue::Str(v.scale.clone()));
    }
    line("clear_color", LuaValue::Str(v.clear_color.clone()));
    line("start", LuaValue::Str(v.start.clone()));
    if v.fullscreen != d.fullscreen {
        line("fullscreen", LuaValue::Bool(v.fullscreen));
    }
    if v.resizable != d.resizable {
        line("resizable", LuaValue::Bool(v.resizable));
    }
    if v.vsync != d.vsync {
        line("vsync", LuaValue::Bool(v.vsync));
    }
    if v.hot_reload != d.hot_reload {
        line("hot_reload", LuaValue::Bool(v.hot_reload));
    }
    if v.fixed_hz != d.fixed_hz {
        line("fixed_hz", LuaValue::Int(v.fixed_hz as i64));
    }
    if !v.vars.is_empty() {
        line("vars", LuaValue::Raw(vars_lua(&v.vars)));
    }
    s.push_str("}\n");
    s
}

/// Troca os valores no texto. `None` se o arquivo não tiver o formato
/// esperado (`return { ... }`).
fn patch(text: &str, changes: &[(&str, LuaValue)]) -> Option<String> {
    let table = main_table(text)?;
    let mut out = text.to_string();
    // Substitui de trás para frente, para as posições continuarem valendo.
    let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    let mut missing = Vec::new();
    for (key, value) in changes {
        match find_value(text, table.clone(), key) {
            Some(range) => edits.push((range, value.to_lua())),
            None => missing.push((key, value)),
        }
    }
    if !missing.is_empty() {
        // Chaves novas: antes do `}` final, com o mesmo recuo das outras.
        let close = table.end - 1;
        let before = text[..close].trim_end();
        let needs_comma = !before.ends_with(',') && !before.ends_with('{');
        let indent = indent_of_entries(text, table.clone()).unwrap_or_else(|| "  ".into());
        let mut add = String::new();
        if needs_comma {
            add.push(',');
        }
        for (key, value) in &missing {
            add.push_str(&format!("\n{indent}{key} = {},", value.to_lua()));
        }
        add.push('\n');
        edits.push((before.len()..close, add));
    }
    edits.sort_by_key(|(r, _)| std::cmp::Reverse(r.start));
    for (range, new) in edits {
        out.replace_range(range, &new);
    }
    Some(out)
}

/// Posição da tabela de `return { ... }` (do `{` até depois do `}`).
fn main_table(text: &str) -> Option<std::ops::Range<usize>> {
    let tokens = crate::lua_syntax::tokenize(text);
    let mut iter = tokens.iter().filter(|(_, k)| {
        !matches!(
            k,
            crate::lua_syntax::TokenKind::Space | crate::lua_syntax::TokenKind::Comment
        )
    });
    let mut start = None;
    while let Some((r, _)) = iter.next() {
        if &text[r.clone()] == "return" {
            let (open, _) = iter.next()?;
            if &text[open.clone()] != "{" {
                return None;
            }
            start = Some(open.start);
            break;
        }
    }
    let start = start?;
    let mut depth = 0i32;
    for (r, k) in &tokens {
        if r.start < start || *k != crate::lua_syntax::TokenKind::Operator {
            continue;
        }
        match &text[r.clone()] {
            "{" => depth += 1,
            "}" => {
                depth -= 1;
                if depth == 0 {
                    return Some(start..r.end);
                }
            }
            _ => {}
        }
    }
    None
}

/// Trecho do valor de `key = valor` no primeiro nível da tabela.
fn find_value(
    text: &str,
    table: std::ops::Range<usize>,
    key: &str,
) -> Option<std::ops::Range<usize>> {
    use crate::lua_syntax::TokenKind;
    let tokens: Vec<_> = crate::lua_syntax::tokenize(text)
        .into_iter()
        .filter(|(r, k)| {
            r.start > table.start
                && r.end < table.end
                && !matches!(k, TokenKind::Space | TokenKind::Comment)
        })
        .collect();
    let mut depth = 0;
    for (i, (r, k)) in tokens.iter().enumerate() {
        let s = &text[r.clone()];
        if *k == TokenKind::Operator {
            match s {
                "{" | "(" | "[" => depth += 1,
                "}" | ")" | "]" => depth -= 1,
                _ => {}
            }
            continue;
        }
        if depth != 0 || s != key {
            continue;
        }
        let Some((eq, _)) = tokens.get(i + 1) else {
            continue;
        };
        if &text[eq.clone()] != "=" {
            continue;
        }
        // O valor vai até a próxima vírgula (ou `}`) do mesmo nível.
        let first = tokens.get(i + 2)?;
        let mut end = first.0.end;
        let mut d = 0;
        for (r, k) in &tokens[i + 2..] {
            let s = &text[r.clone()];
            if *k == TokenKind::Operator {
                match s {
                    "{" | "(" | "[" => d += 1,
                    "}" | ")" | "]" => d -= 1,
                    "," | ";" if d == 0 => break,
                    _ => {}
                }
            }
            end = r.end;
        }
        return Some(first.0.start..end);
    }
    None
}

fn indent_of_entries(text: &str, table: std::ops::Range<usize>) -> Option<String> {
    text[table]
        .lines()
        .skip(1)
        .find(|l| l.contains('=') && !l.trim_start().starts_with("--"))
        .map(|l| l.chars().take_while(|c| c.is_whitespace()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PADRAO: &str = "-- Configuração do jogo.\nreturn {\n  title = \"Meu jogo\",\n  -- Não mude depois de distribuir.\n  id = \"meu-jogo\",\n  width = 1280,\n  height = 720,\n  clear_color = \"#14202E\",\n  start = \"cenas/titulo.lua\",\n}\n";

    #[test]
    fn tamanho_fixo_do_jogo() {
        let text = "return {\n  title = \"X\",\n  width = 1280,\n  height = 720,\n}\n";
        let old = read(text, "x").unwrap();
        assert_eq!(old.resolution, None);
        assert_eq!(old.game_size(), (1280, 720));
        let mut new = old.clone();
        new.resolution = Some((640, 360));
        new.scale = "pixel".into();
        let out = update(text, &changes(&old, &new), &new);
        assert!(out.contains("resolution = { 640, 360 },"), "{out}");
        assert!(out.contains("scale = \"pixel\","), "{out}");
        let back = read(&out, "x").unwrap();
        assert_eq!(back.game_size(), (640, 360));
        assert_eq!(back.scale, "pixel");
        // Desligar volta ao tamanho da janela.
        let mut off = back.clone();
        off.resolution = None;
        let out2 = update(&out, &changes(&back, &off), &off);
        assert!(out2.contains("resolution = false,"), "{out2}");
        assert_eq!(read(&out2, "x").unwrap().resolution, None);
    }

    #[test]
    fn variaveis_do_jogo() {
        let text =
            "return {\n  title = \"X\",\n  -- placar\n  vars = { vidas = 3, moedas = 0 },\n}\n";
        let old = read(text, "x").unwrap();
        assert_eq!(
            old.vars,
            vec![
                ("moedas".to_string(), Prop::Num(0.0)),
                ("vidas".to_string(), Prop::Num(3.0))
            ]
        );
        let mut new = old.clone();
        new.vars.push(("nome".into(), Prop::Str("Ana".into())));
        new.vars.retain(|(k, _)| k != "vidas");
        let out = update(text, &changes(&old, &new), &new);
        assert!(out.contains("-- placar"), "{out}");
        assert!(
            out.contains("vars = { moedas = 0, nome = \"Ana\" },"),
            "{out}"
        );
        assert_eq!(read(&out, "x").unwrap().vars.len(), 2);
        // Sem `vars` no arquivo: entra no fim.
        let out = update(PADRAO, &changes(&read(PADRAO, "x").unwrap(), &new), &new);
        assert!(out.contains("vars = {"), "{out}");
        assert!(valid_var_name("vidas_2") && !valid_var_name("2vidas") && !valid_var_name("a b"));
    }

    #[test]
    fn le_valores_e_padroes() {
        let v = read(PADRAO, "x").unwrap();
        assert_eq!(v.title, "Meu jogo");
        assert_eq!(v.id, "meu-jogo");
        assert_eq!((v.width, v.height), (1280, 720));
        assert_eq!(v.start, "cenas/titulo.lua");
        assert!(v.vsync && v.resizable && !v.fullscreen);
        let vazio = read("", "Pasta").unwrap();
        assert_eq!(vazio.title, "Pasta");
        assert_eq!(vazio.start, "main.lua");
        assert!(read("return {", "x").is_err());
    }

    #[test]
    fn muda_so_o_valor_e_mantem_comentarios() {
        let old = read(PADRAO, "x").unwrap();
        let mut new = old.clone();
        new.title = "Aventura \"Épica\"".into();
        new.width = 960;
        new.fullscreen = true;
        let c = changes(&old, &new);
        assert_eq!(c.len(), 3);
        let out = update(PADRAO, &c, &new);
        assert!(out.contains("-- Configuração do jogo."), "{out}");
        assert!(out.contains("  -- Não mude depois de distribuir."), "{out}");
        assert!(
            out.contains("  title = \"Aventura \\\"Épica\\\"\","),
            "{out}"
        );
        assert!(out.contains("  width = 960,"), "{out}");
        assert!(out.contains("  fullscreen = true,\n}"), "{out}");
        // Lê de volta exatamente o que foi pedido.
        assert_eq!(read(&out, "x").unwrap(), new);
    }

    #[test]
    fn tabela_sem_virgula_no_fim_e_em_uma_linha() {
        let src = "return { title = \"A\", width = 800 }";
        let old = read(src, "x").unwrap();
        let mut new = old.clone();
        new.width = 1024;
        new.vsync = false;
        let out = update(src, &changes(&old, &new), &new);
        assert_eq!(read(&out, "x").unwrap(), new);
        assert!(
            out.starts_with("return { title = \"A\", width = 1024"),
            "{out}"
        );
    }

    #[test]
    fn arquivo_complicado_e_gerado_de_novo() {
        let src = "local w = 640\nreturn { title = \"A\", width = w * 2, height = w }";
        let old = read(src, "x").unwrap();
        assert_eq!(old.width, 1280);
        let mut new = old.clone();
        new.height = 500;
        let out = update(src, &changes(&old, &new), &new);
        assert_eq!(read(&out, "x").unwrap(), new);
        // Arquivo que não é `return {...}`: também gera de novo.
        let src = "local c = {}\nc.title = \"B\"\nreturn c";
        let old = read(src, "x").unwrap();
        let mut new = old.clone();
        new.width = 300;
        let out = update(src, &changes(&old, &new), &new);
        assert_eq!(read(&out, "x").unwrap(), new);
    }

    #[test]
    fn valores_aninhados_nao_confundem() {
        let src = "return {\n  extra = { width = 1 },\n  width = 2,\n}\n";
        let old = read(src, "x").unwrap();
        assert_eq!(old.width, 2);
        let mut new = old.clone();
        new.width = 3;
        let out = update(src, &changes(&old, &new), &new);
        assert!(out.contains("extra = { width = 1 }"), "{out}");
        assert!(out.contains("  width = 3,"), "{out}");
    }
}
