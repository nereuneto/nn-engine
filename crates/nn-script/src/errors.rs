//! Mensagens de erro do Lua em português.
//!
//! O Lua escreve os erros em inglês ("attempt to index a nil value (global
//! 'x')"). [`translate_error`] reescreve as mensagens mais comuns, de sintaxe
//! e de execução, e as linhas do caminho do erro (stack traceback). O que não
//! for reconhecido fica como está. As posições (`arquivo.lua:12:`) não mudam,
//! então editores e o console continuam achando a linha.

/// O porquê de um argumento inválido, em português.
fn argument_detail(d: &str) -> String {
    if let Some((exp, got)) = d.split_once(" expected, got ") {
        return format!("esperava {}, recebeu {}", type_name(exp), type_name(got));
    }
    if d == "number has no integer representation" {
        return "o número não é inteiro; use math.floor".to_string();
    }
    if let Some(conv) = d.strip_prefix("error converting Lua ") {
        // "nil to f32 (expected number)", "integer to u32 (out of range)".
        let (conv, why) = match conv.split_once(" (") {
            Some((c, w)) => (c, Some(w.trim_end_matches(')'))),
            None => (conv, None),
        };
        if let Some((from, to)) = conv.split_once(" to ") {
            let want = match to {
                "f32" | "f64" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "usize"
                | "isize" => "um número",
                "String" | "string" => "um texto",
                "bool" | "boolean" => "true ou false",
                "table" | "Table" => "uma tabela",
                "function" | "Function" => "uma função",
                "userdata" | "AnyUserData" => "um objeto da engine (imagem, efeito, som...)",
                other => other,
            };
            return match why {
                Some("out of range") => {
                    format!("o número {} está fora do que vale aqui", type_name(from))
                }
                _ => format!("esperava {want}, recebeu {}", type_name(from)),
            };
        }
    }
    d.to_string()
}

/// A mensagem de um erro pego com `pcall`, limpa: só o que houve (sem
/// "runtime error:", sem o caminho até o erro e sem o começo da pasta do jogo
/// nos nomes dos arquivos), em português.
pub fn clean_error(message: &str, root: Option<&str>) -> String {
    let mut m = message;
    if let Some(i) = m.rfind("caused by: ") {
        m = &m[i + "caused by: ".len()..];
    }
    if let Some(i) = m.find("stack traceback:") {
        m = &m[..i];
    }
    let m = m.trim_end();
    let m = m.strip_prefix("runtime error: ").unwrap_or(m);
    let mut m = m.to_string();
    if let Some(root) = root.filter(|r| !r.is_empty()) {
        for sep in ["/", "\\"] {
            m = m.replace(&format!("{}{sep}", root.trim_end_matches(['/', '\\'])), "");
        }
    }
    // "asset não encontrado: x.png" vira "o arquivo x.png não existe".
    if let Some(rest) = m.strip_prefix("asset não encontrado: ") {
        m = format!("o arquivo {rest} não existe");
    }
    translate_error(&m)
}

/// Traduz uma mensagem de erro do Lua (pode ter várias linhas).
pub fn translate_error(message: &str) -> String {
    message
        .lines()
        .map(translate_line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn translate_line(line: &str) -> String {
    // Linhas do caminho do erro: "\tcenas/fase.lua:12: in function 'x'".
    let indent_len = line.len() - line.trim_start().len();
    let (indent, rest) = line.split_at(indent_len);
    if rest == "stack traceback:" {
        return format!("{indent}caminho até o erro (stack traceback):");
    }
    if rest == "(...tail calls...)" {
        return format!("{indent}(...chamadas finais...)");
    }
    let (location, body) = split_location(rest);
    if !indent.is_empty()
        && let Some(t) = translate_traceback(body)
    {
        return format!("{indent}{location}{t}");
    }
    match translate_message(body) {
        Some(t) => format!("{indent}{location}{t}"),
        None => line.to_string(),
    }
}

/// Separa o começo "arquivo.lua:12: " (ou "[C]: ") do resto da linha.
fn split_location(line: &str) -> (&str, &str) {
    if let Some(rest) = line.strip_prefix("[C]: ") {
        return (&line[..line.len() - rest.len()], rest);
    }
    let bytes = line.as_bytes();
    let mut i = 0;
    while let Some(off) = line[i..].find(':') {
        let colon = i + off;
        let digits = bytes[colon + 1..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count();
        let after = colon + 1 + digits;
        if digits > 0 && line[after..].starts_with(": ") {
            return line.split_at(after + 2);
        }
        i = colon + 1;
    }
    ("", line)
}

fn translate_traceback(body: &str) -> Option<String> {
    if body == "in main chunk" {
        return Some("no corpo do arquivo".into());
    }
    if body == "in ?" {
        return Some("em ?".into());
    }
    for (en, pt) in [
        ("in function ", "na função "),
        ("in method ", "no método "),
        ("in local ", "na função local "),
        ("in field ", "na função "),
        ("in upvalue ", "na função "),
        ("in metamethod ", "no metamétodo "),
        ("in for iterator ", "no iterador do for "),
        ("in hook ", "no gancho "),
    ] {
        if let Some(rest) = body.strip_prefix(en) {
            return Some(format!("{pt}{rest}"));
        }
    }
    None
}

/// "um valor nil", "uma tabela"...
fn a_value(type_name: &str) -> String {
    match type_name {
        "nil" => "um valor nil (vazio)".into(),
        "number" => "um número".into(),
        "string" => "um texto (string)".into(),
        "boolean" => "um booleano (true/false)".into(),
        "table" => "uma tabela".into(),
        "function" => "uma função".into(),
        "userdata" => "um objeto da engine (userdata)".into(),
        "thread" => "uma corrotina".into(),
        other => format!("um valor {other}"),
    }
}

fn type_name(t: &str) -> &str {
    match t {
        "number" => "número",
        "string" => "texto",
        "boolean" => "booleano",
        "table" => "tabela",
        "function" => "função",
        "thread" => "corrotina",
        "no value" => "nada",
        other => other,
    }
}

/// " (global 'x')" → " (variável global 'x')".
fn var_info(info: &str) -> String {
    let inner = info
        .trim()
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'));
    let Some(inner) = inner else {
        return info.to_string();
    };
    for (en, pt) in [
        ("global ", "variável global "),
        ("local ", "variável local "),
        ("field ", "campo "),
        ("method ", "método "),
        ("upvalue ", "variável externa "),
        ("constant ", "constante "),
    ] {
        if let Some(name) = inner.strip_prefix(en) {
            return format!(" ({pt}{name})");
        }
    }
    info.to_string()
}

/// Dica extra para os casos mais comuns de quem está começando.
fn hint(op: &str, type_name: &str, info: &str) -> &'static str {
    if type_name != "nil" {
        return "";
    }
    let global = info.contains("(global ");
    match op {
        "index" if global => " — essa variável não existe; confira o nome (maiúsculas contam)",
        "call" if global => " — essa função não existe; confira o nome (maiúsculas contam)",
        "call" if info.contains("(method ") || info.contains("(field ") => {
            " — essa função não existe; confira o nome"
        }
        _ => "",
    }
}

/// "'x'" ou "<eof>" → como aparece em português.
fn near(token: &str) -> String {
    if token == "<eof>" || token == "'<eof>'" {
        "do fim do arquivo".into()
    } else {
        format!("de {token}")
    }
}

fn translate_message(msg: &str) -> Option<String> {
    // --- Execução -----------------------------------------------------------
    if let Some(rest) = msg.strip_prefix("attempt to compare ") {
        if let Some(t) = rest
            .strip_prefix("two ")
            .and_then(|r| r.strip_suffix(" values"))
        {
            return Some(format!(
                "tentou comparar dois valores do tipo {}",
                type_name(t)
            ));
        }
        if let Some((a, b)) = rest.split_once(" with ") {
            return Some(format!(
                "tentou comparar {} com {}",
                type_name(a),
                type_name(b)
            ));
        }
    }
    for (op, pt) in [
        ("index", "tentou acessar um campo de"),
        ("call", "tentou chamar"),
        ("perform arithmetic on", "tentou fazer uma conta com"),
        ("concatenate", "tentou juntar (..) texto com"),
        (
            "perform bitwise operation on",
            "tentou fazer uma operação de bits com",
        ),
        ("get length of", "tentou pegar o tamanho (#) de"),
    ] {
        let prefix = format!("attempt to {op} a ");
        if let Some(rest) = msg.strip_prefix(&prefix) {
            let (type_part, info) = match rest.find(" value") {
                Some(i) => (&rest[..i], &rest[i + " value".len()..]),
                None => (rest, ""),
            };
            return Some(format!(
                "{pt} {}{}{}",
                a_value(type_part),
                var_info(info),
                hint(op, type_part, info)
            ));
        }
    }
    if let Some(rest) = msg.strip_prefix("bad argument #") {
        // "bad argument #1 to 'insert' (table expected, got nil)"
        // "bad argument #3: error converting..." (sem o nome da função).
        if let Some((n, detail)) = rest.split_once(": ")
            && !n.contains(' ')
        {
            return Some(format!(
                "argumento nº {n} inválido ({})",
                argument_detail(detail)
            ));
        }
        let (n, rest) = rest.split_once(' ')?;
        let rest = rest.strip_prefix("to ")?;
        let (func, detail) = match rest.find(" (") {
            Some(i) if rest.ends_with(')') => (&rest[..i], Some(&rest[i + 2..rest.len() - 1])),
            _ => match rest.split_once(": ") {
                Some((f, d)) => (f, Some(d)),
                None => (rest, None),
            },
        };
        let detail = detail.map(argument_detail);
        return Some(match detail {
            Some(d) => format!("argumento nº {n} inválido em {func} ({d})"),
            None => format!("argumento nº {n} inválido em {func}"),
        });
    }
    let fixed = [
        (
            "stack overflow",
            "estouro de pilha: uma função chamando a si mesma sem parar?",
        ),
        ("attempt to perform 'n//0'", "divisão inteira por zero"),
        ("attempt to perform 'n%%0'", "resto de divisão por zero"),
        (
            "'for' initial value must be a number",
            "o valor inicial do 'for' precisa ser um número",
        ),
        (
            "'for' limit must be a number",
            "o limite do 'for' precisa ser um número",
        ),
        (
            "'for' step must be a number",
            "o passo do 'for' precisa ser um número",
        ),
        ("'for' step is zero", "o passo do 'for' é zero"),
        ("table index is nil", "índice de tabela é nil"),
        ("table index is NaN", "índice de tabela é NaN"),
        (
            "number has no integer representation",
            "o número não tem representação inteira",
        ),
    ];
    for (en, pt) in fixed {
        if msg == en || msg.starts_with(&format!("{en} ")) {
            return Some(format!("{pt}{}", &msg[en.len()..]));
        }
    }
    if let Some(rest) = msg.strip_prefix("attempt to assign to const variable ") {
        return Some(format!("tentou mudar a constante {rest}"));
    }
    if let Some(rest) = msg.strip_prefix("module ")
        && let Some(name) = rest.strip_suffix(" not found:")
    {
        return Some(format!("módulo {name} não encontrado:"));
    }
    if let Some(rest) = msg.strip_prefix("no field package.preload[") {
        return Some(
            format!(
                "nenhum módulo pré-carregado [{}",
                rest.trim_end_matches(']')
            ) + "]",
        );
    }

    // --- Sintaxe ------------------------------------------------------------
    let (head, tok) = match msg.rfind(" near ") {
        Some(i) => (&msg[..i], Some(&msg[i + " near ".len()..])),
        None => (msg, None),
    };
    let near_txt = tok
        .map(|t| format!(" (perto {})", near(t)))
        .unwrap_or_default();
    if let Some((what, rest)) = head.split_once(" expected (to close ") {
        // "'end' expected (to close 'function' at line 3)"
        let rest = rest.strip_suffix(')')?;
        let (opener, line) = rest.split_once(" at line ")?;
        return Some(format!(
            "falta {what} para fechar o {opener} da linha {line}{near_txt}"
        ));
    }
    let syntax = [
        ("unexpected symbol", "símbolo inesperado"),
        ("unfinished string", "texto sem as aspas de fechamento"),
        ("malformed number", "número mal escrito"),
        ("syntax error", "erro de sintaxe"),
        (
            "function arguments expected",
            "esperava os argumentos da função",
        ),
        ("invalid escape sequence", "sequência de escape inválida"),
        (
            "invalid long string delimiter",
            "delimitador de texto longo inválido",
        ),
        ("decimal escape too large", "escape decimal grande demais"),
        ("UTF-8 value too large", "valor UTF-8 grande demais"),
        (
            "hexadecimal digit expected",
            "esperava um dígito hexadecimal",
        ),
        ("chunk has too many lines", "o arquivo tem linhas demais"),
    ];
    for (en, pt) in syntax {
        if head == en {
            return Some(format!("{pt}{near_txt}"));
        }
    }
    if let Some(what) = head.strip_suffix(" expected") {
        let what = if what == "<name>" { "um nome" } else { what };
        return Some(format!("esperava {what}{near_txt}"));
    }
    for (en, pt) in [
        (
            "unfinished long string (starting at line ",
            "texto longo ([[ ]]) sem fechamento (começa na linha ",
        ),
        (
            "unfinished long comment (starting at line ",
            "comentário longo (--[[ ]]) sem fechamento (começa na linha ",
        ),
        (
            "break outside a loop at line ",
            "'break' fora de um laço, na linha ",
        ),
    ] {
        if let Some(rest) = head.strip_prefix(en) {
            return Some(format!("{pt}{rest}{near_txt}"));
        }
    }
    if let Some(rest) = head.strip_prefix("no visible label ")
        && let Some(label) = rest.strip_suffix(" for goto")
    {
        return Some(format!(
            "o rótulo {label} do goto não foi encontrado{near_txt}"
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erros_de_execucao() {
        assert_eq!(
            translate_error("main.lua:3: attempt to index a nil value (global 'jogador')"),
            "main.lua:3: tentou acessar um campo de um valor nil (vazio) (variável global 'jogador') — essa variável não existe; confira o nome (maiúsculas contam)"
        );
        assert_eq!(
            translate_error("cenas/fase.lua:12: attempt to call a nil value (method 'pular')"),
            "cenas/fase.lua:12: tentou chamar um valor nil (vazio) (método 'pular') — essa função não existe; confira o nome"
        );
        assert_eq!(
            translate_error("a.lua:1: attempt to perform arithmetic on a nil value (field 'vida')"),
            "a.lua:1: tentou fazer uma conta com um valor nil (vazio) (campo 'vida')"
        );
        assert_eq!(
            translate_error("a.lua:1: attempt to concatenate a table value (local 't')"),
            "a.lua:1: tentou juntar (..) texto com uma tabela (variável local 't')"
        );
        assert_eq!(
            translate_error("a.lua:2: attempt to compare number with nil"),
            "a.lua:2: tentou comparar número com nil"
        );
        assert_eq!(
            translate_error("a.lua:2: attempt to compare two table values"),
            "a.lua:2: tentou comparar dois valores do tipo tabela"
        );
        assert_eq!(
            translate_error("a.lua:5: bad argument #1 to 'insert' (table expected, got nil)"),
            "a.lua:5: argumento nº 1 inválido em 'insert' (esperava tabela, recebeu nil)"
        );
        assert_eq!(
            translate_error("a.lua:5: bad argument #2 to 'floor' (number expected, got no value)"),
            "a.lua:5: argumento nº 2 inválido em 'floor' (esperava número, recebeu nada)"
        );
        assert_eq!(
            translate_error(
                "a.lua:3: bad argument #1 to 'format' (number has no integer representation)"
            ),
            "a.lua:3: argumento nº 1 inválido em 'format' (o número não é inteiro; use math.floor)"
        );
    }

    #[test]
    fn erros_de_sintaxe() {
        assert_eq!(
            translate_error(
                "main.lua:9: 'end' expected (to close 'function' at line 3) near <eof>"
            ),
            "main.lua:9: falta 'end' para fechar o 'function' da linha 3 (perto do fim do arquivo)"
        );
        assert_eq!(
            translate_error("main.lua:2: '=' expected near 'x'"),
            "main.lua:2: esperava '=' (perto de 'x')"
        );
        assert_eq!(
            translate_error("main.lua:2: unexpected symbol near '+'"),
            "main.lua:2: símbolo inesperado (perto de '+')"
        );
        assert_eq!(
            translate_error("main.lua:4: unfinished string near '\"oi'"),
            "main.lua:4: texto sem as aspas de fechamento (perto de '\"oi')"
        );
        assert_eq!(
            translate_error("t.lua:16: function arguments expected near '.'"),
            "t.lua:16: esperava os argumentos da função (perto de '.')"
        );
        assert_eq!(
            translate_error("main.lua:1: <name> expected near '('"),
            "main.lua:1: esperava um nome (perto de '(')"
        );
        assert_eq!(
            translate_error("x.lua:7: unfinished long comment (starting at line 2) near '<eof>'"),
            "x.lua:7: comentário longo (--[[ ]]) sem fechamento (começa na linha 2) (perto do fim do arquivo)"
        );
    }

    #[test]
    fn caminho_do_erro() {
        let msg = "main.lua:3: attempt to call a nil value (global 'f')\n\
                   stack traceback:\n\
                   \t[C]: in ?\n\
                   \tmain.lua:3: in function 'cena.update'\n\
                   \tmain.lua:10: in main chunk\n\
                   \t(...tail calls...)";
        let t = translate_error(msg);
        let linhas: Vec<_> = t.lines().collect();
        assert!(linhas[0].starts_with("main.lua:3: tentou chamar"), "{t}");
        assert_eq!(linhas[1], "caminho até o erro (stack traceback):");
        assert_eq!(linhas[2], "\t[C]: em ?");
        assert_eq!(linhas[3], "\tmain.lua:3: na função 'cena.update'");
        assert_eq!(linhas[4], "\tmain.lua:10: no corpo do arquivo");
        assert_eq!(linhas[5], "\t(...chamadas finais...)");
    }

    #[test]
    fn mensagens_desconhecidas_ficam_iguais() {
        for msg in [
            "main.lua:3: cor inválida: \"#xyz\"",
            "não foi possível ler foo.png",
            "",
            "texto: com: dois pontos",
        ] {
            assert_eq!(translate_error(msg), msg);
        }
    }

    #[test]
    fn posicao_sem_mensagem_conhecida() {
        assert_eq!(split_location("a/b.lua:12: oi"), ("a/b.lua:12: ", "oi"));
        assert_eq!(
            split_location("C:\\x\\b.lua:3: oi"),
            ("C:\\x\\b.lua:3: ", "oi")
        );
        assert_eq!(split_location("sem posição"), ("", "sem posição"));
    }
}
