//! Lua para o editor de código: cores (tokens), verificação de sintaxe
//! enquanto se digita, posições `arquivo.lua:12` em mensagens e o recuo
//! automático da próxima linha.

use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Keyword,
    /// `nn`, `self`, `print`, `math`... (nomes que já existem).
    Builtin,
    /// `true`, `false`, `nil`.
    Literal,
    Number,
    String,
    Comment,
    /// Nome logo depois de `function` ou antes de `(`.
    Function,
    Operator,
    Name,
    Space,
}

const KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "for", "function", "goto", "if", "in", "local",
    "not", "or", "repeat", "return", "then", "until", "while",
];

const BUILTINS: &[&str] = &[
    "nn",
    "self",
    "print",
    "pairs",
    "ipairs",
    "math",
    "string",
    "table",
    "os",
    "require",
    "tostring",
    "tonumber",
    "type",
    "pcall",
    "xpcall",
    "error",
    "assert",
    "select",
    "next",
    "setmetatable",
    "getmetatable",
    "rawget",
    "rawset",
    "rawequal",
    "rawlen",
    "unpack",
    "utf8",
    "coroutine",
];

/// Divide o código em pedaços com o tipo de cada um. Os pedaços cobrem o
/// texto todo, em ordem.
pub fn tokenize(src: &str) -> Vec<(Range<usize>, TokenKind)> {
    let b = src.as_bytes();
    let mut out: Vec<(Range<usize>, TokenKind)> = Vec::new();
    let mut i = 0;
    let mut after_function = false;
    while i < b.len() {
        let start = i;
        let c = b[i];
        let kind = if c.is_ascii_whitespace() {
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1;
            }
            TokenKind::Space
        } else if c == b'-' && b.get(i + 1) == Some(&b'-') {
            i += 2;
            if let Some(level) = long_bracket_level(b, i) {
                i = long_bracket_end(b, i, level);
            } else {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            TokenKind::Comment
        } else if c == b'[' && long_bracket_level(b, i).is_some() {
            let level = long_bracket_level(b, i).unwrap_or(0);
            i = long_bracket_end(b, i, level);
            TokenKind::String
        } else if c == b'"' || c == b'\'' {
            i += 1;
            while i < b.len() && b[i] != c && b[i] != b'\n' {
                if b[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            i = (i + 1).min(b.len());
            TokenKind::String
        } else if c.is_ascii_digit() || (c == b'.' && b.get(i + 1).is_some_and(u8::is_ascii_digit))
        {
            let hex = c == b'0' && matches!(b.get(i + 1), Some(b'x' | b'X'));
            if hex {
                i += 2;
            }
            while i < b.len() {
                let d = b[i];
                let exp = if hex {
                    matches!(d, b'p' | b'P')
                } else {
                    matches!(d, b'e' | b'E')
                };
                if exp && matches!(b.get(i + 1), Some(b'+' | b'-')) {
                    i += 2;
                } else if d.is_ascii_alphanumeric() || d == b'.' {
                    i += 1;
                } else {
                    break;
                }
            }
            TokenKind::Number
        } else if c.is_ascii_alphabetic() || c == b'_' || c >= 0x80 {
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] >= 0x80) {
                i += 1;
            }
            let word = &src[start..i];
            if word == "true" || word == "false" || word == "nil" {
                TokenKind::Literal
            } else if KEYWORDS.contains(&word) {
                TokenKind::Keyword
            } else if BUILTINS.contains(&word) {
                TokenKind::Builtin
            } else if after_function || next_non_space(b, i) == Some(b'(') {
                TokenKind::Function
            } else {
                TokenKind::Name
            }
        } else {
            i += 1;
            // Caracteres de vários bytes (fora de nomes) ficam inteiros.
            while i < b.len() && !src.is_char_boundary(i) {
                i += 1;
            }
            TokenKind::Operator
        };
        match kind {
            TokenKind::Space => {}
            TokenKind::Keyword => after_function = &src[start..i] == "function",
            // `function a.b:c()`: o nome inteiro é a função.
            TokenKind::Operator if after_function && matches!(c, b'.' | b':') => {}
            TokenKind::Name | TokenKind::Builtin | TokenKind::Function if after_function => {
                out.push((start..i, TokenKind::Function));
                continue;
            }
            _ => after_function = false,
        }
        out.push((start..i, kind));
    }
    out
}

fn next_non_space(b: &[u8], mut i: usize) -> Option<u8> {
    while i < b.len() && (b[i] == b' ' || b[i] == b'\t') {
        i += 1;
    }
    b.get(i).copied()
}

/// `[[`, `[=[`, `[==[`... em `i`: devolve o número de `=`.
fn long_bracket_level(b: &[u8], i: usize) -> Option<usize> {
    if b.get(i) != Some(&b'[') {
        return None;
    }
    let mut j = i + 1;
    while b.get(j) == Some(&b'=') {
        j += 1;
    }
    (b.get(j) == Some(&b'[')).then_some(j - i - 1)
}

/// Fim (exclusivo) de um texto/comentário longo que começa em `i`.
fn long_bracket_end(b: &[u8], i: usize, level: usize) -> usize {
    let mut close = vec![b']'];
    close.extend(std::iter::repeat_n(b'=', level));
    close.push(b']');
    let body = i + level + 2;
    b.get(body..)
        .and_then(|rest| {
            rest.windows(close.len())
                .position(|w| w == close.as_slice())
        })
        .map(|p| body + p + close.len())
        .unwrap_or(b.len())
}

/// Erro de sintaxe encontrado no código.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxIssue {
    /// Linha (a partir de 1).
    pub line: usize,
    /// Mensagem em português, sem o "arquivo:linha:" do começo.
    pub message: String,
}

/// Compila o código (sem executar) e devolve o erro de sintaxe, se houver.
pub fn check(lua: &mlua::Lua, name: &str, code: &str) -> Option<SyntaxIssue> {
    let err = lua
        .load(code)
        .set_name(format!("@{name}"))
        .into_function()
        .err()?;
    let raw = match &err {
        mlua::Error::SyntaxError { message, .. } => message.clone(),
        other => other.to_string(),
    };
    let msg = nn_script::translate_error(raw.trim());
    let (line, message) = match find_locations(&msg).first() {
        Some(loc) if loc.range.start == 0 => (
            loc.line,
            msg[loc.range.end..]
                .trim_start_matches(':')
                .trim()
                .to_string(),
        ),
        _ => (1, msg.clone()),
    };
    Some(SyntaxIssue { line, message })
}

/// Uma posição `arquivo.lua:12` dentro de um texto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    /// Trecho do texto com "arquivo.lua:12".
    pub range: Range<usize>,
    /// Caminho como aparece (barras normalizadas para `/`).
    pub path: String,
    pub line: usize,
}

/// Acha todas as posições `algo.lua:N` num texto (mensagens de erro, log).
pub fn find_locations(text: &str) -> Vec<Location> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(off) = text[from..].find(".lua:") {
        let dot = from + off;
        let after = dot + ".lua:".len();
        let digits: String = text[after..]
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        from = after;
        if digits.is_empty() {
            continue;
        }
        // Volta até o começo do caminho.
        let start = text[..dot]
            .char_indices()
            .rev()
            .take_while(|(_, c)| c.is_alphanumeric() || matches!(c, '_' | '-' | '/' | '\\' | '.'))
            .last()
            .map(|(i, _)| i)
            .unwrap_or(dot);
        // Letra de unidade do Windows ("C:\\jogo\\main.lua").
        let start = match text[..start].strip_suffix(':') {
            Some(before)
                if before
                    .chars()
                    .last()
                    .is_some_and(|c| c.is_ascii_alphabetic())
                    && before[..before.len() - 1]
                        .chars()
                        .last()
                        .is_none_or(|c| !c.is_alphanumeric())
                    && text[start..].starts_with(['/', '\\']) =>
            {
                start - 2
            }
            _ => start,
        };
        let mut path = text[start..dot + 4].replace('\\', "/");
        let mut range_start = start;
        while let Some(rest) = path.strip_prefix("./") {
            path = rest.to_string();
            range_start += 2;
        }
        if path == ".lua" || path.is_empty() {
            continue;
        }
        let end = after + digits.len();
        out.push(Location {
            range: range_start..end,
            path,
            line: digits.parse().unwrap_or(1),
        });
        from = end;
    }
    out
}

/// Recuo da linha seguinte a `line` (a linha atual inteira): mantém o recuo
/// e soma um nível depois de `then`, `do`, `function(...)`, `{`, `else`,
/// `repeat`...
pub fn next_indent(line: &str, unit: &str) -> String {
    let indent: String = line
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    let code = strip_comment(line).trim_end();
    let opens = code.ends_with(" then")
        || code == "then"
        || code.ends_with(" do")
        || code == "do"
        || code.ends_with('{')
        || code.ends_with('(')
        || code.trim_start() == "else"
        || code.trim_start() == "repeat"
        || (code.contains("function") && code.ends_with(')') && !code.contains(" end"));
    if opens {
        format!("{indent}{unit}")
    } else {
        indent
    }
}

/// O código da linha sem um comentário `--` no fim (ignora `--` dentro de texto).
fn strip_comment(line: &str) -> &str {
    let b = line.as_bytes();
    let mut quote = None;
    let mut i = 0;
    while i < b.len() {
        match (quote, b[i]) {
            (None, b'"' | b'\'') => quote = Some(b[i]),
            (Some(q), c) if c == q => quote = None,
            (Some(_), b'\\') => i += 1,
            (None, b'-') if b.get(i + 1) == Some(&b'-') => return &line[..i],
            _ => {}
        }
        i += 1;
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<(&str, TokenKind)> {
        tokenize(src)
            .into_iter()
            .filter(|(_, k)| *k != TokenKind::Space)
            .map(|(r, k)| (&src[r], k))
            .collect()
    }

    #[test]
    fn tokens_basicos() {
        use TokenKind::*;
        assert_eq!(
            kinds("local x = 10 -- dez\nif x > 0.5e3 then print(\"oi\") end"),
            vec![
                ("local", Keyword),
                ("x", Name),
                ("=", Operator),
                ("10", Number),
                ("-- dez", Comment),
                ("if", Keyword),
                ("x", Name),
                (">", Operator),
                ("0.5e3", Number),
                ("then", Keyword),
                ("print", Builtin),
                ("(", Operator),
                ("\"oi\"", String),
                (")", Operator),
                ("end", Keyword),
            ]
        );
    }

    #[test]
    fn funcoes_textos_longos_e_comentarios_longos() {
        use TokenKind::*;
        let src = "function cena:update(dt) end\n--[[ várias\nlinhas ]] local s = [==[ a ]] b ]==] nn.draw.rect(0x1F, true, nil)";
        let k = kinds(src);
        assert!(k.contains(&("cena", Function)));
        assert!(k.contains(&("update", Function)));
        assert!(k.contains(&("--[[ várias\nlinhas ]]", Comment)));
        assert!(k.contains(&("[==[ a ]] b ]==]", String)));
        assert!(k.contains(&("nn", Builtin)));
        assert!(k.contains(&("rect", Function)));
        assert!(k.contains(&("0x1F", Number)));
        assert!(k.contains(&("true", Literal)));
        assert!(k.contains(&("nil", Literal)));
        assert!(k.contains(&("dt", Name)));
    }

    #[test]
    fn tokens_cobrem_tudo_mesmo_com_texto_aberto_e_acentos() {
        for src in [
            "local s = \"sem fim",
            "x = 'ok' -- ç ã é",
            "[[ aberto",
            "a = b ~= c ç",
        ] {
            let toks = tokenize(src);
            let mut pos = 0;
            for (r, _) in &toks {
                assert_eq!(r.start, pos, "{src}");
                pos = r.end;
            }
            assert_eq!(pos, src.len(), "{src}");
        }
    }

    #[test]
    fn verifica_sintaxe_em_portugues() {
        let lua = mlua::Lua::new();
        assert_eq!(check(&lua, "main.lua", "local x = 1\nreturn x"), None);
        let e = check(
            &lua,
            "main.lua",
            "local c = {}\nfunction c:draw()\n  x = \nend",
        )
        .unwrap();
        assert_eq!(e.line, 4);
        assert!(e.message.contains("perto de 'end'"), "{e:?}");
        let e = check(&lua, "cenas/a.lua", "function f()\n  return 1\n").unwrap();
        assert_eq!(e.line, 3);
        assert!(e.message.starts_with("falta 'end'"), "{e:?}");
    }

    #[test]
    fn acha_posicoes_em_mensagens() {
        let msg = "erro em cenas/fase.lua:\ncenas/fase.lua:12: tentou chamar\n\t[C]: em ?\n\t./lib/util.lua:3: na função 'x'\nC:\\jogo\\main.lua:7: oi";
        let locs = find_locations(msg);
        let simples: Vec<_> = locs.iter().map(|l| (l.path.as_str(), l.line)).collect();
        assert_eq!(
            simples,
            vec![
                ("cenas/fase.lua", 12),
                ("lib/util.lua", 3),
                ("C:/jogo/main.lua", 7)
            ]
        );
        assert_eq!(&msg[locs[0].range.clone()], "cenas/fase.lua:12");
        assert_eq!(&msg[locs[1].range.clone()], "lib/util.lua:3");
        assert!(find_locations("nada aqui.lua: e x.lua:").is_empty());
    }

    #[test]
    fn recuo_automatico() {
        assert_eq!(next_indent("  if x then", "  "), "    ");
        assert_eq!(next_indent("function cena:update(dt)", "  "), "  ");
        assert_eq!(next_indent("  local t = {", "  "), "    ");
        assert_eq!(next_indent("  for i = 1, 3 do -- laço", "  "), "    ");
        assert_eq!(next_indent("    x = 1", "  "), "    ");
        assert_eq!(next_indent("  else", "  "), "    ");
        assert_eq!(next_indent("  local f = function() end", "  "), "  ");
        assert_eq!(next_indent("  print(\"-- then\")", "  "), "  ");
    }
}
