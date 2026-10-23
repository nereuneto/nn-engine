//! Diálogos (`.dialogo`): o agente escreve a conversa inteira em JSON (as
//! falas, quem fala, as escolhas e o que cada uma faz) e lê de volta no mesmo
//! formato. É o arquivo do editor de conversa do NN Studio
//! ([`nn_projeto::dialogue`]); as posições dos cartões ficam ou são
//! arrumadas sozinhas.

use std::collections::{BTreeMap, HashMap, VecDeque};

use serde_json::{Map, Value, json};

use nn_projeto::dialogue::{self, Character, Choice, DialogueData, Node};

use super::{Output, Tool, arg_str, s_list, s_num, s_obj, s_str};
use crate::Server;

/// Cores para personagens novos sem cor.
const COLORS: &[&str] = &[
    "#FFD166", "#7EC4FF", "#A7F070", "#FF8FA3", "#C9A7FF", "#FFB570",
];

fn open(s: &Server, rel: &str) -> Result<(String, Option<DialogueData>), String> {
    let (rel, full) = crate::paths::resolve(&s.root, rel)?;
    if !rel.to_lowercase().ends_with(".dialogo") {
        return Err(format!("{rel} não é um diálogo (.dialogo)"));
    }
    if !full.is_file() {
        return Ok((rel, None));
    }
    let (_, text) = s.read_text(&rel)?;
    Ok((rel.clone(), Some(dialogue::read(&text, &rel)?)))
}

/// Um id de fala: número ou nome ("oi", "pergunta").
fn key(v: &Value) -> Option<String> {
    match v {
        Value::Number(n) => n.as_u64().map(|n| n.to_string()),
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        _ => None,
    }
}

/// Texto de um valor (número ou texto) para var/value.
fn plain(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

fn str_field(m: &Map<String, Value>, k: &str) -> String {
    m.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

/// Arruma os cartões (para o editor): cada fala uma coluna depois de quem
/// leva a ela.
fn layout(d: &mut DialogueData, keep: &HashMap<u32, (f32, f32)>) {
    let mut depth: BTreeMap<u32, usize> = BTreeMap::new();
    let mut queue = VecDeque::from([(d.start, 0usize)]);
    while let Some((id, k)) = queue.pop_front() {
        if id == 0 || depth.contains_key(&id) {
            continue;
        }
        let Some(n) = d.node(id) else { continue };
        depth.insert(id, k);
        let mut next: Vec<u32> = n.choices.iter().map(|c| c.next).collect();
        next.push(n.next);
        for x in next {
            queue.push_back((x, k + 1));
        }
    }
    let max = depth.values().copied().max().unwrap_or(0);
    let mut rows: HashMap<usize, usize> = HashMap::new();
    for n in &mut d.nodes {
        if let Some((x, y)) = keep.get(&n.id) {
            (n.x, n.y) = (*x, *y);
            continue;
        }
        let col = depth.get(&n.id).copied().unwrap_or(max + 1);
        let row = rows.entry(col).or_default();
        n.x = 30.0 + col as f32 * 300.0;
        n.y = 30.0 + *row as f32 * 190.0;
        *row += 1;
    }
}

/// Lê a conversa do JSON do agente. Devolve a conversa e os avisos.
fn from_json(a: &Value, old: Option<&DialogueData>) -> Result<(DialogueData, Vec<String>), String> {
    let mut notes = Vec::new();
    let falas = a
        .get("falas")
        .and_then(Value::as_array)
        .filter(|l| !l.is_empty())
        .ok_or("\"falas\" é a lista das falas (pelo menos uma)")?;
    // Os ids: números ficam; nomes ganham números depois do maior.
    let mut ids: HashMap<String, u32> = HashMap::new();
    let mut order = Vec::new();
    let mut top = falas
        .iter()
        .filter_map(|f| f.get("id").and_then(Value::as_u64))
        .max()
        .unwrap_or(0) as u32;
    for (i, f) in falas.iter().enumerate() {
        let f = f
            .as_object()
            .ok_or_else(|| format!("fala {}: precisa ser um objeto", i + 1))?;
        let k =
            match f.get("id") {
                None => None,
                Some(v) => Some(key(v).filter(|k| k != "0").ok_or_else(|| {
                    format!("fala {}: \"id\" é um número (não 0) ou um nome", i + 1)
                })?),
            };
        let id = match &k {
            Some(k) => {
                if ids.contains_key(k) {
                    return Err(format!("fala {}: o id \"{k}\" se repete", i + 1));
                }
                match k.parse::<u32>() {
                    Ok(n) => n,
                    Err(_) => {
                        top += 1;
                        top
                    }
                }
            }
            None => {
                top += 1;
                top
            }
        };
        if let Some(k) = k {
            ids.insert(k, id);
        }
        order.push(id);
    }
    let link = |v: Option<&Value>, after: Option<u32>, what: &str| -> Result<u32, String> {
        match v {
            None | Some(Value::Null) => Ok(after.unwrap_or(0)),
            Some(Value::String(s)) if s == "fim" => Ok(0),
            Some(Value::Number(n)) if n.as_u64() == Some(0) => Ok(0),
            Some(v) => {
                let k = key(v)
                    .ok_or_else(|| format!("{what}: o destino é o id de uma fala ou \"fim\""))?;
                ids.get(&k).copied().ok_or_else(|| {
                    format!("{what}: não há a fala \"{k}\" (as falas: {})", {
                        let mut all: Vec<&String> = ids.keys().collect();
                        all.sort();
                        all.iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                })
            }
        }
    };

    let mut d = DialogueData {
        start: 0,
        speed: a
            .get("velocidade")
            .and_then(Value::as_f64)
            .or(old.map(|o| o.speed))
            .unwrap_or(40.0)
            .clamp(0.0, 1000.0),
        characters: Vec::new(),
        nodes: Vec::new(),
    };
    // Os personagens: os dados; os que só aparecem nas falas entram sozinhos.
    if let Some(list) = a.get("personagens") {
        for (i, p) in list
            .as_array()
            .ok_or("\"personagens\" é uma lista de {\"nome\", \"cor\", \"retrato\", \"voz\"}")?
            .iter()
            .enumerate()
        {
            let p = p
                .as_object()
                .ok_or_else(|| format!("personagem {}: precisa ser um objeto", i + 1))?;
            let name = str_field(p, "nome");
            if name.trim().is_empty() {
                return Err(format!("personagem {}: falta o \"nome\"", i + 1));
            }
            let color = match str_field(p, "cor") {
                c if c.is_empty() => COLORS[d.characters.len() % COLORS.len()].to_string(),
                c => c,
            };
            d.characters.push(Character {
                name,
                color,
                image: str_field(p, "retrato"),
                sound: str_field(p, "voz"),
            });
        }
    } else if let Some(o) = old {
        d.characters = o.characters.clone();
    }

    for (i, f) in falas.iter().enumerate() {
        let f = f.as_object().expect("conferido");
        let n = i + 1;
        let what = format!("fala {n}");
        let after = order.get(i + 1).copied();
        let who = str_field(f, "quem");
        let text = str_field(f, "texto");
        if text.trim().is_empty() {
            return Err(format!("{what}: falta o \"texto\""));
        }
        let mut node = Node {
            id: order[i],
            who: who.clone(),
            text,
            var: str_field(f, "variavel"),
            value: plain(f.get("valor")),
            item: str_field(f, "item"),
            ..Node::default()
        };
        if let Some(list) = f.get("escolhas").filter(|v| !v.is_null()) {
            let list = list
                .as_array()
                .ok_or_else(|| format!("{what}: \"escolhas\" é uma lista"))?;
            for (j, c) in list.iter().enumerate() {
                let c = c
                    .as_object()
                    .ok_or_else(|| format!("{what}, escolha {}: precisa ser um objeto", j + 1))?;
                let cw = format!("{what}, escolha {}", j + 1);
                let text = str_field(c, "texto");
                if text.trim().is_empty() {
                    return Err(format!("{cw}: falta o \"texto\""));
                }
                node.choices.push(Choice {
                    text,
                    next: link(c.get("vai"), None, &cw)?,
                    var: str_field(c, "variavel"),
                    value: plain(c.get("valor")),
                    need: str_field(c, "precisa"),
                });
            }
            if f.contains_key("depois") {
                notes.push(format!(
                    "{what}: com escolhas, \"depois\" não vale (cada escolha diz para onde \"vai\")"
                ));
            }
        } else {
            node.next = link(f.get("depois"), after, &what)?;
        }
        if !who.is_empty() && d.character(&who).is_none() {
            let color = COLORS[d.characters.len() % COLORS.len()].to_string();
            d.characters.push(Character {
                name: who.clone(),
                color,
                ..Character::default()
            });
            notes.push(format!("personagem \"{who}\" criado (sem retrato nem voz)"));
        }
        d.nodes.push(node);
    }
    d.start = match a.get("comeco").filter(|v| !v.is_null()) {
        Some(v) => link(Some(v), None, "\"comeco\"")?,
        None => order[0],
    };
    if d.start == 0 {
        return Err("\"comeco\" é a primeira fala (não \"fim\")".into());
    }
    let keep: HashMap<u32, (f32, f32)> = old
        .map(|o| o.nodes.iter().map(|n| (n.id, (n.x, n.y))).collect())
        .unwrap_or_default();
    layout(&mut d, &keep);
    Ok((d, notes))
}

/// Um valor de variável: número quando é um número (como a engine lê).
fn value_json(v: &str) -> Value {
    match v.trim().parse::<f64>() {
        Ok(n) if n.is_finite() && !v.trim().is_empty() => {
            if n.fract() == 0.0 && n.abs() < 1e15 {
                json!(n as i64)
            } else {
                json!(n)
            }
        }
        _ => json!(v),
    }
}

/// A conversa no formato que `escrever_dialogo` recebe.
fn to_json(d: &DialogueData) -> Value {
    let chars: Vec<Value> = d
        .characters
        .iter()
        .map(|c| {
            let mut m = Map::new();
            m.insert("nome".into(), json!(c.name));
            m.insert("cor".into(), json!(c.color));
            if !c.image.is_empty() {
                m.insert("retrato".into(), json!(c.image));
            }
            if !c.sound.is_empty() {
                m.insert("voz".into(), json!(c.sound));
            }
            Value::Object(m)
        })
        .collect();
    let mut nodes: Vec<&Node> = d.nodes.iter().collect();
    // O começo primeiro.
    nodes.sort_by_key(|n| n.id != d.start);
    let falas: Vec<Value> = nodes
        .iter()
        .map(|n| {
            let mut m = Map::new();
            m.insert("id".into(), json!(n.id));
            m.insert("quem".into(), json!(n.who));
            m.insert("texto".into(), json!(n.text));
            if n.choices.is_empty() {
                m.insert(
                    "depois".into(),
                    if n.next == 0 {
                        json!("fim")
                    } else {
                        json!(n.next)
                    },
                );
            } else {
                let cs: Vec<Value> = n
                    .choices
                    .iter()
                    .map(|c| {
                        let mut cm = Map::new();
                        cm.insert("texto".into(), json!(c.text));
                        cm.insert(
                            "vai".into(),
                            if c.next == 0 {
                                json!("fim")
                            } else {
                                json!(c.next)
                            },
                        );
                        if !c.var.is_empty() {
                            cm.insert("variavel".into(), json!(c.var));
                            cm.insert("valor".into(), value_json(&c.value));
                        }
                        if !c.need.is_empty() {
                            cm.insert("precisa".into(), json!(c.need));
                        }
                        Value::Object(cm)
                    })
                    .collect();
                m.insert("escolhas".into(), Value::Array(cs));
            }
            if !n.var.is_empty() {
                m.insert("variavel".into(), json!(n.var));
                m.insert("valor".into(), value_json(&n.value));
            }
            if !n.item.is_empty() {
                m.insert("item".into(), json!(n.item));
            }
            Value::Object(m)
        })
        .collect();
    json!({
        "comeco": d.start,
        "velocidade": d.speed,
        "personagens": chars,
        "falas": falas,
    })
}

/// A conversa contada: os caminhos, a partir do começo.
fn outline(d: &DialogueData) -> String {
    let mut out = String::new();
    let mut seen = Vec::new();
    let mut queue = VecDeque::from([d.start]);
    while let Some(id) = queue.pop_front() {
        if id == 0 || seen.contains(&id) {
            continue;
        }
        seen.push(id);
        let Some(n) = d.node(id) else { continue };
        let who = if n.who.is_empty() {
            "(narrador)"
        } else {
            n.who.as_str()
        };
        out.push_str(&format!(
            "[{}] {who}: {}\n",
            n.id,
            n.text.replace('\n', " / ")
        ));
        if !n.var.is_empty() {
            out.push_str(&format!("      faz {} = {}\n", n.var, n.value));
        }
        if !n.item.is_empty() {
            out.push_str(&format!("      dá o item {}\n", n.item));
        }
        let to = |x: u32| {
            if x == 0 {
                "fim".to_string()
            } else {
                format!("[{x}]")
            }
        };
        if n.choices.is_empty() {
            out.push_str(&format!("      → {}\n", to(n.next)));
            queue.push_back(n.next);
        } else {
            for (i, c) in n.choices.iter().enumerate() {
                let mut extra = String::new();
                if !c.need.is_empty() {
                    extra.push_str(&format!(" (só com o item {})", c.need));
                }
                if !c.var.is_empty() {
                    extra.push_str(&format!(" (faz {} = {})", c.var, c.value));
                }
                out.push_str(&format!(
                    "      {}) \"{}\"{extra} → {}\n",
                    i + 1,
                    c.text,
                    to(c.next)
                ));
                queue.push_back(c.next);
            }
        }
    }
    let lost: Vec<String> = d
        .nodes
        .iter()
        .filter(|n| !seen.contains(&n.id))
        .map(|n| format!("[{}]", n.id))
        .collect();
    if !lost.is_empty() {
        out.push_str(&format!(
            "AVISO: nenhum caminho chega em {} (nunca aparecem).\n",
            lost.join(", ")
        ));
    }
    out
}

/// Arquivos dos personagens que não existem.
fn missing_files(s: &Server, d: &DialogueData) -> Vec<String> {
    let mut out = Vec::new();
    for c in &d.characters {
        for (what, f) in [("retrato", &c.image), ("voz", &c.sound)] {
            if !f.is_empty() && !s.root.join(f).is_file() {
                out.push(format!("{} de {}: {f} não existe", what, c.name));
            }
        }
    }
    out
}

fn usage(rel: &str) -> String {
    format!(
        "Na cena: um personagem com o comportamento nn:dialogo, {{\"type\": \"behavior\", \"script\": \"nn:dialogo\", \"arquivo\": \"{rel}\"}} (com a tecla E perto dele; \"comecar\": \"chegar\", \"cena\" ou \"mensagem\" muda isso). O texto pode ter {{variavel}}."
    )
}

fn escrever_dialogo(s: &mut Server, a: &Value) -> Result<Output, String> {
    let (rel, old) = open(s, arg_str(a, "caminho")?)?;
    let (d, mut notes) = from_json(a, old.as_ref())?;
    s.write_file(&rel, dialogue::write(&d).as_bytes())?;
    notes.extend(missing_files(s, &d));
    let mut text = format!(
        "{} {rel}: {} fala(s), {} personagem(ns). A conversa:\n{}",
        if old.is_some() { "Reescrevi" } else { "Criei" },
        d.nodes.len(),
        d.characters.len(),
        outline(&d)
    );
    for n in notes {
        text.push_str(&format!("- {n}\n"));
    }
    text.push_str(&usage(&rel));
    Ok(Output::text(text))
}

fn ver_dialogo(s: &mut Server, a: &Value) -> Result<Output, String> {
    let (rel, d) = open(s, arg_str(a, "caminho")?)?;
    let d = d.ok_or_else(|| format!("{rel} não existe (crie com escrever_dialogo)"))?;
    let mut text = format!(
        "{rel}: {} fala(s). A conversa:\n{}",
        d.nodes.len(),
        outline(&d)
    );
    for n in missing_files(s, &d) {
        text.push_str(&format!("- {n}\n"));
    }
    text.push_str(&format!(
        "\nNo formato de escrever_dialogo (mude e mande de volta inteiro):\n{}\n\n{}",
        serde_json::to_string_pretty(&to_json(&d)).unwrap_or_default(),
        usage(&rel)
    ));
    Ok(Output::text(text))
}

pub static TOOLS: &[&Tool] = &[
    &Tool {
        name: "escrever_dialogo",
        title: "Escrever diálogo",
        description: "Cria ou reescreve uma conversa (.dialogo, o arquivo do editor de conversa do NN Studio) inteira. Cada fala: {\"id\": \"oi\" (um nome ou número; opcional), \"quem\": \"Ana\" (vazio = narrador), \"texto\": \"Oi, {nome}!\", \"depois\": id da próxima ou \"fim\" (sem \"depois\", vai para a próxima da lista e a última termina; o fim de um ramo de escolha precisa de \"depois\": \"fim\"), \"escolhas\": [{\"texto\": \"Sim\", \"vai\": id ou \"fim\", \"variavel\": \"ajudou\", \"valor\": 1, \"precisa\": \"chave\" (só aparece com o item)}], \"variavel\"/\"valor\" (muda uma variável do jogo ao aparecer), \"item\" (dá um item)}. A conversa começa na primeira fala (ou em \"comeco\"). Personagens que aparecem nas falas entram sozinhos; em \"personagens\" vão cor, retrato (imagem) e voz (som que toca enquanto o texto aparece). Para mudar uma conversa, leia com ver_dialogo e mande de volta inteira.",
        params: || {
            vec![
                (
                    "caminho",
                    s_str("O diálogo, ex.: \"dialogos/ana.dialogo\""),
                    true,
                ),
                (
                    "falas",
                    s_list("As falas, na ordem (veja a descrição)", s_obj("Uma fala")),
                    true,
                ),
                (
                    "personagens",
                    s_list(
                        "[{\"nome\": \"Ana\", \"cor\": \"#FFD166\", \"retrato\": \"assets/imagens/ana.png\", \"voz\": \"assets/sons/blip.wav\"}]",
                        s_obj("Um personagem"),
                    ),
                    false,
                ),
                (
                    "comeco",
                    s_str("O id da primeira fala (padrão: a primeira da lista)"),
                    false,
                ),
                (
                    "velocidade",
                    s_num("Letras por segundo (0 = o texto aparece de uma vez; padrão 40)"),
                    false,
                ),
            ]
        },
        read_only: false,
        destructive: false,
        run: escrever_dialogo,
    },
    &Tool {
        name: "ver_dialogo",
        title: "Ver diálogo",
        description: "Mostra uma conversa (.dialogo) (os ids com nome viram números no arquivo): os caminhos a partir do começo (quem fala, o texto, as escolhas e para onde vão), falas que nunca aparecem e arquivos que faltam, e a conversa no formato de escrever_dialogo, para mudar e mandar de volta.",
        params: || vec![("caminho", s_str("O diálogo (.dialogo)"), true)],
        read_only: true,
        destructive: false,
        run: ver_dialogo,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falas_com_nomes_e_sequencia() {
        let a = json!({ "falas": [
            { "id": "oi", "quem": "Ana", "texto": "Oi!" },
            { "id": "pergunta", "quem": "Ana", "texto": "Ajuda?", "escolhas": [
                { "texto": "Sim", "vai": "obrigada", "variavel": "ajudou", "valor": 1 },
                { "texto": "Não", "vai": "fim" },
            ] },
            { "id": "obrigada", "quem": "Ana", "texto": "Valeu!", "item": "moeda" },
        ] });
        let (d, notes) = from_json(&a, None).unwrap();
        assert_eq!(d.start, 1);
        assert_eq!(d.nodes[0].next, 2, "sem \"depois\": a próxima da lista");
        assert_eq!(d.nodes[1].choices[0].next, 3);
        assert_eq!(d.nodes[1].choices[1].next, 0);
        assert_eq!(d.nodes[2].next, 0, "a última termina");
        assert_eq!(d.characters.len(), 1);
        assert!(notes[0].contains("\"Ana\" criado"));
        // Volta igual pelo formato de leitura.
        let (e, _) = from_json(&to_json(&d), Some(&d)).unwrap();
        assert_eq!(e, d);
        let bad = json!({ "falas": [{ "texto": "a", "depois": "x" }] });
        assert!(
            from_json(&bad, None)
                .unwrap_err()
                .contains("não há a fala \"x\"")
        );
    }
}
