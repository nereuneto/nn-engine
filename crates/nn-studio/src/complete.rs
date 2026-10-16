//! Autocompletar do editor de código: o que pode vir no cursor (API `nn.*`,
//! objetos da cena, campos de corpos e objetos, propriedades do
//! comportamento, variáveis do arquivo, teclas, arquivos e cenas dentro de
//! strings) e a dica da função em que o cursor está.

use std::ops::Range;

use nn_script::api_docs::{
    self, API_DOCS, BEHAVIOR_MEMBERS, BODY_MEMBERS, Member, OBJECT_MEMBERS, SCENE_MEMBERS,
    SHAPE_MEMBERS, SPRITE_MEMBERS, TEXT_MEMBERS,
};

use crate::lua_syntax::{TokenKind, tokenize};

/// Tipo de sugestão (decide o ícone e o que entra no texto).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Function,
    Field,
    Namespace,
    Keyword,
    Variable,
    File,
    Value,
}

impl ItemKind {
    /// Letra que aparece antes da sugestão.
    pub fn badge(self) -> &'static str {
        match self {
            ItemKind::Function => "ƒ",
            ItemKind::Field => "•",
            ItemKind::Namespace => "{}",
            ItemKind::Keyword => "k",
            ItemKind::Variable => "x",
            ItemKind::File => "/",
            ItemKind::Value => "\"",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub label: String,
    /// O que entra no lugar do que já foi digitado.
    pub insert: String,
    /// Assinatura (`(img, x, y, opts)`), se houver.
    pub detail: String,
    pub doc: String,
    pub kind: ItemKind,
}

impl Item {
    fn new(label: impl Into<String>, kind: ItemKind) -> Self {
        let label = label.into();
        Self {
            insert: label.clone(),
            label,
            detail: String::new(),
            doc: String::new(),
            kind,
        }
    }

    fn doc(mut self, detail: &str, doc: &str) -> Self {
        self.detail = detail.to_string();
        self.doc = doc.to_string();
        self
    }
}

/// Que tipo de script é o arquivo aberto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScriptKind {
    /// Comportamento (tem `props`): `self` é o comportamento.
    Behavior,
    /// Script de uma cena visual: `self` é a cena com os objetos.
    SceneScript,
    #[default]
    Other,
}

/// O que o autocompletar sabe do projeto.
#[derive(Debug, Clone, Default)]
pub struct Env {
    /// Arquivos do projeto (caminhos relativos).
    pub files: Vec<String>,
    /// Nomes dos objetos das cenas que usam este script.
    pub objects: Vec<String>,
    /// Variáveis do jogo (`nn.vars`).
    pub vars: Vec<String>,
    pub kind: ScriptKind,
}

/// Sugestões para o cursor: o trecho que será trocado e as opções.
#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    pub range: Range<usize>,
    pub items: Vec<Item>,
}

const KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in", "local",
    "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

const GLOBALS: &[(&str, &str)] = &[
    ("nn", "A engine."),
    ("self", "A cena (ou o comportamento) deste arquivo."),
    ("print", "Escreve no console."),
    ("pairs", "Percorre uma tabela (chave, valor)."),
    ("ipairs", "Percorre uma lista (1, 2, 3...)."),
    ("math", "Funções de matemática."),
    ("string", "Funções de texto."),
    ("table", "Funções de tabela."),
    ("require", "Usa um módulo: require(\"lib.util\")."),
    ("tostring", "Valor como texto."),
    ("tonumber", "Texto como número."),
    ("type", "Tipo de um valor."),
];

const MATH: &[(&str, &str, &str)] = &[
    ("abs", "(x)", "Valor absoluto."),
    ("floor", "(x)", "Arredonda para baixo."),
    ("ceil", "(x)", "Arredonda para cima."),
    ("min", "(a, b, ...)", "O menor."),
    ("max", "(a, b, ...)", "O maior."),
    ("sqrt", "(x)", "Raiz quadrada."),
    ("sin", "(x)", "Seno (radianos)."),
    ("cos", "(x)", "Cosseno (radianos)."),
    ("atan", "(y, x)", "Ângulo de um vetor."),
    ("random", "(a, b)", "Número sorteado."),
    ("pi", "", "3,14159..."),
    ("huge", "", "Infinito."),
];

/// Nomes de teclas aceitos pelo `nn.input`.
pub fn key_names() -> Vec<String> {
    let mut v: Vec<String> = ('a'..='z').map(|c| c.to_string()).collect();
    v.extend(('0'..='9').map(|c| c.to_string()));
    for k in [
        "space",
        "enter",
        "escape",
        "tab",
        "backspace",
        "delete",
        "shift",
        "ctrl",
        "alt",
        "left",
        "right",
        "up",
        "down",
    ] {
        v.push(k.into());
    }
    v.extend((1..=12).map(|n| format!("f{n}")));
    v
}

const PAD_BUTTONS: &[&str] = &[
    "a", "b", "x", "y", "lb", "rb", "lt", "rt", "back", "start", "ls", "rs", "up", "down", "left",
    "right",
];

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Começo da palavra que termina em `end`.
fn word_start(text: &str, end: usize) -> usize {
    text[..end]
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_ident(*c))
        .last()
        .map(|(i, _)| i)
        .unwrap_or(end)
}

/// Expressão `a.b:c` que termina em `end` (sem parênteses).
fn chain_before(text: &str, end: usize) -> &str {
    let start = text[..end]
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_ident(*c) || *c == '.' || *c == ':')
        .last()
        .map(|(i, _)| i)
        .unwrap_or(end);
    &text[start..end]
}

/// O cursor está dentro de uma string? Devolve o começo do conteúdo dela e
/// a função chamada com ela (`nn.input.down` em `nn.input.down("`).
fn string_context(text: &str, cursor: usize) -> Option<(usize, String)> {
    for (r, kind) in tokenize(text) {
        if r.start >= cursor {
            break;
        }
        if kind != TokenKind::String || cursor > r.end {
            continue;
        }
        let quote = text[r.start..].chars().next()?;
        if quote != '"' && quote != '\'' {
            return None;
        }
        let closed = r.end - r.start >= 2 && text[..r.end].ends_with(quote);
        if cursor == r.end && closed {
            continue;
        }
        let before = text[..r.start].trim_end();
        let call = match before.strip_suffix('(') {
            Some(b) => chain_before(b, b.trim_end().len()).to_string(),
            None => String::new(),
        };
        return Some((r.start + 1, call));
    }
    None
}

/// Está num comentário?
fn in_comment(text: &str, cursor: usize) -> bool {
    tokenize(text)
        .iter()
        .any(|(r, k)| *k == TokenKind::Comment && r.start < cursor && cursor <= r.end)
}

/// Variáveis e funções locais do arquivo, e os campos `self.x` usados nele.
fn file_names(text: &str) -> (Vec<String>, Vec<String>) {
    let mut locals = Vec::new();
    let mut fields = Vec::new();
    let tokens: Vec<_> = tokenize(text)
        .into_iter()
        .filter(|(_, k)| !matches!(k, TokenKind::Space | TokenKind::Comment))
        .collect();
    for (i, (r, _)) in tokens.iter().enumerate() {
        let s = &text[r.clone()];
        if s == "local" || s == "function" || s == "for" {
            let mut j = i + 1;
            while let Some((r2, k2)) = tokens.get(j) {
                let w = &text[r2.clone()];
                if matches!(k2, TokenKind::Name | TokenKind::Function) {
                    if !w.contains(['.', ':']) {
                        locals.push(w.to_string());
                    }
                } else if w != "," {
                    break;
                }
                j += 1;
            }
        }
        if s == "self"
            && let (Some((dot, _)), Some((name, _))) = (tokens.get(i + 1), tokens.get(i + 2))
            && &text[dot.clone()] == "."
        {
            fields.push(text[name.clone()].to_string());
        }
    }
    // Parâmetros de funções: `function nome(a, b)`.
    for (i, (r, _)) in tokens.iter().enumerate() {
        if &text[r.clone()] != "function" {
            continue;
        }
        let Some(open) = tokens[i + 1..]
            .iter()
            .position(|(r, _)| &text[r.clone()] == "(")
        else {
            continue;
        };
        for (r, k) in &tokens[i + 1 + open + 1..] {
            let w = &text[r.clone()];
            if w == ")" {
                break;
            }
            if *k == TokenKind::Name {
                locals.push(w.to_string());
            }
        }
    }
    for list in [&mut locals, &mut fields] {
        list.sort();
        list.dedup();
    }
    (locals, fields)
}

fn members(list: &[Member], colon: bool) -> Vec<Item> {
    list.iter()
        .filter(|m| !colon || !m.args.is_empty())
        .map(|m| {
            let kind = if m.args.is_empty() {
                ItemKind::Field
            } else {
                ItemKind::Function
            };
            Item::new(m.name, kind).doc(m.args, m.doc)
        })
        .collect()
}

/// O que vem depois de `chain.` ou `chain:`.
fn after_dot(chain: &str, colon: bool, text: &str, env: &Env) -> Vec<Item> {
    let segs: Vec<&str> = chain.split(['.', ':']).collect();
    let last = segs.last().copied().unwrap_or("");
    let prev = if segs.len() >= 2 {
        segs[segs.len() - 2]
    } else {
        ""
    };
    if segs.first() == Some(&"nn") {
        if chain == "nn.vars" {
            return env
                .vars
                .iter()
                .map(|v| Item::new(v, ItemKind::Variable).doc("", "Variável do jogo."))
                .collect();
        }
        let base = format!("{chain}.");
        let mut out: Vec<Item> = Vec::new();
        for d in API_DOCS {
            let Some(rest) = d.name.strip_prefix(&base) else {
                continue;
            };
            if rest.contains('.') {
                continue;
            }
            let kind = if d.is_function() {
                ItemKind::Function
            } else if API_DOCS
                .iter()
                .any(|x| x.name.starts_with(&format!("{}.", d.name)))
            {
                ItemKind::Namespace
            } else {
                ItemKind::Field
            };
            out.push(Item::new(rest, kind).doc(d.args, d.doc));
        }
        return out;
    }
    match last {
        "body" => return members(BODY_MEMBERS, colon),
        "sprite" => return members(SPRITE_MEMBERS, colon),
        "text" => return members(TEXT_MEMBERS, colon),
        "shape" => return members(SHAPE_MEMBERS, colon),
        "math" if segs.len() == 1 => {
            return MATH
                .iter()
                .map(|(n, a, d)| {
                    let kind = if a.is_empty() {
                        ItemKind::Field
                    } else {
                        ItemKind::Function
                    };
                    Item::new(*n, kind).doc(a, d)
                })
                .collect();
        }
        "objects" => {
            return env
                .objects
                .iter()
                .map(|o| Item::new(o, ItemKind::Field).doc("", "Objeto da cena."))
                .collect();
        }
        "scene" => return members(SCENE_MEMBERS, colon),
        _ => {}
    }
    let is_object = matches!(
        last,
        "obj" | "object_a" | "object_b" | "outro" | "other" | "quem" | "alvo" | "tiro" | "o"
    ) || prev == "objects";
    if is_object {
        return members(OBJECT_MEMBERS, colon);
    }
    if chain == "self" {
        let (_, fields) = file_names(text);
        let mut out = match env.kind {
            ScriptKind::Behavior => members(BEHAVIOR_MEMBERS, colon),
            ScriptKind::SceneScript => members(SCENE_MEMBERS, colon),
            ScriptKind::Other => Vec::new(),
        };
        if !colon {
            for f in fields {
                if !out.iter().any(|i| i.label == f) {
                    out.push(Item::new(f, ItemKind::Field));
                }
            }
        }
        return out;
    }
    // Sem saber o tipo: os nomes usados depois de um ponto neste arquivo.
    let mut seen: Vec<String> = Vec::new();
    for (i, _) in text.match_indices(['.', ':']) {
        let rest = &text[i + 1..];
        let w: String = rest.chars().take_while(|c| is_ident(*c)).collect();
        if !w.is_empty() && !w.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            seen.push(w);
        }
    }
    seen.sort();
    seen.dedup();
    seen.into_iter()
        .map(|w| Item::new(w, ItemKind::Field))
        .collect()
}

/// Sugestões dentro de uma string, conforme a função chamada.
fn in_string(call: &str, env: &Env) -> Vec<Item> {
    let last = call.rsplit(['.', ':']).next().unwrap_or("");
    let files = |pred: &dyn Fn(&str) -> bool| -> Vec<Item> {
        env.files
            .iter()
            .filter(|f| pred(f))
            .map(|f| Item::new(f, ItemKind::File))
            .collect()
    };
    let ext = |f: &str, list: &[&str]| {
        let lower = f.to_lowercase();
        list.iter().any(|e| lower.ends_with(e))
    };
    match call {
        "nn.input.down" | "nn.input.pressed" | "nn.input.released" | "nn.input.axis" => {
            return key_names()
                .into_iter()
                .map(|k| Item::new(k, ItemKind::Value))
                .collect();
        }
        "nn.input.pad_down" | "nn.input.pad_pressed" | "nn.input.pad_released" => {
            return PAD_BUTTONS
                .iter()
                .map(|k| Item::new(*k, ItemKind::Value))
                .collect();
        }
        "nn.scene.switch" | "nn.scene.push" | "nn.scene.replace" => {
            return files(&|f| {
                ext(f, &[".cena"])
                    || (ext(f, &[".lua"]) && f != "config.lua" && !f.starts_with("lib/"))
            });
        }
        "require" => {
            return env
                .files
                .iter()
                .filter(|f| ext(f, &[".lua"]) && *f != "config.lua" && *f != "main.lua")
                .map(|f| {
                    let module = f.trim_end_matches(".lua").replace('/', ".");
                    Item::new(module, ItemKind::File)
                })
                .collect();
        }
        "nn.assets.texture" | "nn.assets.sheet" | "nn.draw.parallax" => {
            return files(&|f| ext(f, &[".png", ".jpg", ".jpeg", ".bmp", ".gif", ".webp"]));
        }
        "nn.assets.sound" => return files(&|f| ext(f, &[".wav", ".ogg", ".mp3", ".flac"])),
        "nn.assets.font" => return files(&|f| ext(f, &[".ttf", ".otf"])),
        "nn.tilemap.load" => return files(&|f| ext(f, &[".tmj"])),
        _ => {}
    }
    match last {
        "spawn" => files(&|f| ext(f, &[".prefab"])),
        "find" | "find_all" | "targets" => env
            .objects
            .iter()
            .map(|o| Item::new(o, ItemKind::Value))
            .collect(),
        "component" => [
            "sprite",
            "text",
            "shape",
            "body",
            "map",
            "camera",
            "sound",
            "particles",
            "behavior",
        ]
        .iter()
        .map(|k| Item::new(*k, ItemKind::Value))
        .collect(),
        _ => files(&|f| !f.ends_with('/')),
    }
}

fn rank(items: Vec<Item>, prefix: &str) -> Vec<Item> {
    let p = prefix.to_lowercase();
    let mut scored: Vec<(u8, Item)> = items
        .into_iter()
        .filter_map(|i| {
            let l = i.label.to_lowercase();
            let score = if p.is_empty() || l.starts_with(&p) {
                0
            } else if l.contains(&p) {
                1
            } else {
                return None;
            };
            Some((score, i))
        })
        .collect();
    // Se algo começa com o que foi digitado, só esses; senão, os que contêm.
    if scored.iter().any(|(s, _)| *s == 0) {
        scored.retain(|(s, _)| *s == 0);
    }
    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.label.cmp(&b.1.label)));
    let mut out: Vec<Item> = Vec::new();
    for (_, i) in scored {
        if !out.iter().any(|o| o.label == i.label) {
            out.push(i);
        }
    }
    // Já está escrito por inteiro e é a única opção: nada a sugerir.
    if out.len() == 1 && out[0].label == prefix {
        out.clear();
    }
    out.truncate(60);
    out
}

/// Sugestões para o cursor em `cursor` (bytes). Com `forced` (Ctrl+Espaço),
/// mostra as opções mesmo sem nada digitado.
pub fn complete(text: &str, cursor: usize, env: &Env, forced: bool) -> Option<Completion> {
    let cursor = cursor.min(text.len());
    if !text.is_char_boundary(cursor) || in_comment(text, cursor) {
        return None;
    }
    if let Some((start, call)) = string_context(text, cursor) {
        let typed = &text[start..cursor];
        if call.is_empty() && !forced && !typed.contains('/') {
            return None;
        }
        let items = rank(in_string(&call, env), typed);
        return (!items.is_empty()).then_some(Completion {
            range: start..cursor,
            items,
        });
    }
    let start = word_start(text, cursor);
    let prefix = &text[start..cursor];
    if prefix.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return None;
    }
    let sep = text[..start].chars().last();
    let items = if matches!(sep, Some('.') | Some(':')) {
        let chain = chain_before(text, start - 1);
        if chain.is_empty() {
            return None;
        }
        after_dot(chain, sep == Some(':'), text, env)
    } else {
        if prefix.is_empty() && !forced {
            return None;
        }
        if !forced && prefix.chars().count() < 2 {
            return None;
        }
        let (locals, _) = file_names(text);
        let mut v: Vec<Item> = KEYWORDS
            .iter()
            .map(|k| Item::new(*k, ItemKind::Keyword))
            .collect();
        v.extend(
            GLOBALS
                .iter()
                .map(|(g, d)| Item::new(*g, ItemKind::Variable).doc("", d)),
        );
        v.extend(locals.into_iter().map(|l| Item::new(l, ItemKind::Variable)));
        v
    };
    let items = rank(items, prefix);
    (!items.is_empty()).then_some(Completion {
        range: start..cursor,
        items,
    })
}

/// A função da API em que o cursor está (para mostrar os parâmetros
/// enquanto se digita os argumentos).
pub fn signature_at(text: &str, cursor: usize) -> Option<&'static api_docs::ApiDoc> {
    let cursor = cursor.min(text.len());
    let mut depth = 0i32;
    let bytes = text.as_bytes();
    let mut i = cursor;
    // Volta até o "(" que ainda está aberto (sem passar de uma linha vazia).
    while i > 0 {
        i -= 1;
        match bytes[i] {
            b')' => depth += 1,
            b'(' if depth == 0 => {
                let chain = chain_before(text, i);
                return api_docs::api_doc(chain).filter(|d| d.is_function());
            }
            b'(' => depth -= 1,
            b'\n' if i > 0 && bytes[i - 1] == b'\n' => return None,
            _ => {}
        }
    }
    None
}

/// Documentação do nome da API sob a posição `at` (para a dica ao passar o
/// mouse).
pub fn doc_at(text: &str, at: usize) -> Option<&'static api_docs::ApiDoc> {
    let at = at.min(text.len());
    if !text.is_char_boundary(at) {
        return None;
    }
    let end = at
        + text[at..]
            .char_indices()
            .find(|(_, c)| !is_ident(*c))
            .map(|(i, _)| i)
            .unwrap_or(text.len() - at);
    let chain = chain_before(text, end);
    if chain.is_empty() {
        return None;
    }
    api_docs::api_doc(chain)
}

/// O texto com a sugestão aplicada e onde o cursor fica (bytes).
pub fn apply(text: &str, c: &Completion, item: &Item) -> (String, usize) {
    let mut out = text.to_string();
    let mut insert = item.insert.clone();
    // Função da API sem "(" logo depois: abre o parêntese.
    let next = text[c.range.end..].chars().next();
    let call = item.kind == ItemKind::Function && next != Some('(');
    if call {
        insert.push('(');
    }
    out.replace_range(c.range.clone(), &insert);
    (out, c.range.start + insert.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> (String, usize) {
        let i = text.find('|').expect("cursor");
        (text.replacen('|', "", 1), i)
    }

    fn labels(text: &str, env: &Env) -> Vec<String> {
        let (t, c) = at(text);
        complete(&t, c, env, false)
            .map(|c| c.items.into_iter().map(|i| i.label).collect())
            .unwrap_or_default()
    }

    fn env() -> Env {
        Env {
            files: vec![
                "assets/imagens/heroi.png".into(),
                "assets/sons/pulo.wav".into(),
                "cenas/fase.cena".into(),
                "cenas/menu.lua".into(),
                "lib/util.lua".into(),
                "prefabs/tiro.prefab".into(),
                "config.lua".into(),
            ],
            objects: vec!["heroi".into(), "moeda".into()],
            vars: vec!["moedas".into(), "vidas".into()],
            kind: ScriptKind::SceneScript,
        }
    }

    #[test]
    fn api_depois_do_ponto() {
        let e = env();
        let l = labels("nn.dr|", &e);
        assert_eq!(l, vec!["draw"]);
        let l = labels("nn.draw.|", &e);
        assert!(l.contains(&"sprite".to_string()) && l.contains(&"rect".to_string()));
        let l = labels("x = nn.input.pre|", &e);
        assert_eq!(l, vec!["pressed"]);
        let l = labels("nn.vars.|", &e);
        assert_eq!(l, vec!["moedas", "vidas"]);
        // Item com a assinatura e a descrição.
        let (t, c) = at("nn.draw.spr|");
        let comp = complete(&t, c, &e, false).unwrap();
        assert_eq!(comp.items[0].detail, "(img, x, y, opts)");
        let (new, cur) = apply(&t, &comp, &comp.items[0]);
        assert_eq!(new, "nn.draw.sprite(");
        assert_eq!(cur, new.len());
    }

    #[test]
    fn objetos_campos_e_self() {
        let e = env();
        assert_eq!(labels("self.objects.he|", &e), vec!["heroi"]);
        let l = labels("self.objects.heroi.|", &e);
        assert!(l.contains(&"body".to_string()) && l.contains(&"destroy".to_string()));
        let l = labels("local b = self.objects.heroi.body.v|", &e);
        assert_eq!(l, vec!["vx", "vy"]);
        let l = labels("self.objects.heroi.body:|", &e);
        assert!(l.contains(&"set_position".to_string()) && !l.contains(&"vx".to_string()));
        let l = labels("function c:update() self.pontos = 1 end\nself.|", &e);
        assert!(l.contains(&"objects".to_string()) && l.contains(&"pontos".to_string()));
        let mut b = e.clone();
        b.kind = ScriptKind::Behavior;
        let l = labels("self.|", &b);
        assert!(l.contains(&"obj".to_string()) && l.contains(&"scene".to_string()));
        let l = labels("self.obj.sprite.fl|", &b);
        assert_eq!(l, vec!["flip_x", "flip_y"]);
    }

    #[test]
    fn palavras_locais_e_palavras_chave() {
        let e = env();
        let l = labels("local velocidade = 3\nlocal function pular() end\nve|", &e);
        assert_eq!(l, vec!["velocidade"]);
        let l = labels("local velocidade = 3\npu|", &e);
        assert!(l.is_empty(), "pular ainda não existe");
        assert_eq!(labels("ret|", &e), vec!["return"]);
        // Uma letra só não abre sozinho; Ctrl+Espaço abre.
        assert!(labels("r|", &e).is_empty());
        let (t, c) = at("r|");
        assert!(complete(&t, c, &e, true).is_some());
        // Comentários e números não.
        assert!(labels("-- nn.dr|", &e).is_empty());
        assert!(labels("x = 12|", &e).is_empty());
    }

    #[test]
    fn dentro_de_strings() {
        let e = env();
        let l = labels("if nn.input.pressed(\"sp|\") then", &e);
        assert_eq!(l, vec!["space"]);
        let l = labels("nn.scene.switch(\"|", &e);
        assert_eq!(l, vec!["cenas/fase.cena", "cenas/menu.lua"]);
        let l = labels("local u = require(\"li|\")", &e);
        assert_eq!(l, vec!["lib.util"]);
        let l = labels("nn.assets.texture('|')", &e);
        assert_eq!(l, vec!["assets/imagens/heroi.png"]);
        let l = labels("self:spawn(\"|\", 0, 0)", &e);
        assert_eq!(l, vec!["prefabs/tiro.prefab"]);
        let l = labels("self:find(\"m|\")", &e);
        assert_eq!(l, vec!["moeda"]);
        // String solta: só com caminho.
        assert!(labels("local s = \"ola|\"", &e).is_empty());
        assert_eq!(
            labels("local s = \"assets/sons/|\"", &e),
            vec!["assets/sons/pulo.wav"]
        );
        // Depois da string fechada não sugere arquivos.
        assert!(labels("nn.scene.switch(\"cenas/fase.cena\"|)", &e).is_empty());
    }

    #[test]
    fn assinatura_e_dica() {
        let (t, c) = at("nn.draw.rect(10, math.max(1, 2), |");
        assert_eq!(signature_at(&t, c).unwrap().name, "nn.draw.rect");
        let (t, c) = at("print(|");
        assert!(signature_at(&t, c).is_none());
        let t = "nn.camera.shake(4)";
        assert_eq!(doc_at(t, 12).unwrap().name, "nn.camera.shake");
        assert_eq!(doc_at(t, 4).unwrap().name, "nn.camera");
        assert!(doc_at("local x = 1", 7).is_none());
    }
}
