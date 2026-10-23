//! Eventos "Quando... Então..." das cenas e dos objetos.

use serde_json::Value;

use nn_projeto::events::{self, EventDef, Trigger};

use super::cenas::find_object;
use super::{Output, Tool, arg_str, opt_num, opt_str, s_enum, s_int, s_obj, s_ref, s_str};
use crate::Server;

fn def_text(d: &EventDef, detail: bool) -> String {
    let mut s = format!("{}: \"{}\"", d.id, d.text);
    if !d.text3d.is_empty() && d.text3d != d.text {
        s.push_str(&format!(" (3D: \"{}\")", d.text3d));
    }
    if !d.mode.is_empty() && d.mode != "both" {
        s.push_str(&format!(" [só {}]", d.mode.to_uppercase()));
    }
    match d.trigger {
        Trigger::Yes => s.push_str(" [gatilho]"),
        Trigger::Depends => s.push_str(" [gatilho, menos com modo = segurar]"),
        Trigger::No => {}
    }
    if detail && !d.help.is_empty() {
        s.push_str(&format!(" — {}", d.help));
    }
    let props: Vec<String> = d
        .props
        .iter()
        .map(|p| {
            let mut t = format!("{} = {}", p.key, p.default.to_lua());
            if !p.options.is_empty() {
                let o: Vec<&str> = p.options.iter().map(|(k, _)| k.as_str()).collect();
                t.push_str(&format!(" ({})", o.join("|")));
            }
            let kind = format!("{:?}", p.kind).to_lowercase();
            if kind == "target" {
                t.push_str(" (quem: @este, @outro, @jogador ou o nome de um objeto)");
            }
            if detail && !p.help.is_empty() {
                t.push_str(&format!(" — {}", p.help));
            }
            t
        })
        .collect();
    if !props.is_empty() {
        s.push_str(&format!("\n    campos: {}", props.join("; ")));
    }
    s
}

fn listar_eventos(_: &mut Server, a: &Value) -> Result<Output, String> {
    let cat = events::catalog();
    let which = opt_str(a, "tipo")?.unwrap_or("tudo");
    let search = opt_str(a, "busca")?.map(crate::paths::fold);
    let three_d = match opt_str(a, "cena_tipo")? {
        Some("3d") => Some(true),
        Some("2d") => Some(false),
        _ => None,
    };
    let detail = search.is_some();
    let mut out = String::from(
        "Regras \"Quando... Então...\": { \"when\": [ { \"cond\": \"id\", ...campos } ], \"actions\": [ { \"act\": \"id\", ...campos } ] }. \
         Com mode = \"enquanto\", as ações rodam a cada quadro enquanto as condições valem; sem, rodam uma vez quando elas passam a valer. \
         Uma condição com \"nao\": true vale ao contrário. Campos que faltam ficam com o padrão. Gatilhos acontecem num instante (apertar, encostar, pegar).\n",
    );
    let mut count = 0;
    for (title, list, kind) in [
        ("CONDIÇÕES (\"cond\")", &cat.conditions, "condicoes"),
        ("AÇÕES (\"act\")", &cat.actions, "acoes"),
    ] {
        if which != "tudo" && which != kind {
            continue;
        }
        out.push_str(&format!("\n{title}\n"));
        for category in events::Catalog::categories(list) {
            let items: Vec<&EventDef> = list
                .iter()
                .filter(|d| d.category == category)
                .filter(|d| three_d.is_none_or(|t| nn_projeto::behaviors::mode_fits(&d.mode, t)))
                .filter(|d| {
                    search.as_ref().is_none_or(|q| {
                        let hay = crate::paths::fold(&format!(
                            "{} {} {} {} {}",
                            d.id, d.name, d.text, d.help, d.category
                        ));
                        q.split_whitespace().all(|w| hay.contains(w))
                    })
                })
                .collect();
            if items.is_empty() {
                continue;
            }
            out.push_str(&format!("[{category}]\n"));
            for d in items {
                count += 1;
                out.push_str(&format!("- {}\n", def_text(d, detail)));
            }
        }
    }
    if count == 0 {
        return Err("nenhuma condição ou ação com isso; tente outra palavra ou sem busca".into());
    }
    Ok(Output::text(out))
}

/// A lista de regras do alvo: a da cena ou a de um objeto.
fn rules_of<'a>(
    doc: &'a mut super::cenas::Doc,
    a: &Value,
) -> Result<(&'a mut Vec<nn_projeto::scene_data::Rule>, bool, String), String> {
    match a.get("objeto") {
        None | Some(Value::Null) => {
            if doc.prefab {
                let o = &mut doc.data.objects[0];
                let name = o.name.clone();
                return Ok((&mut o.events, true, format!("\"{name}\"")));
            }
            Ok((&mut doc.data.events, false, "a cena".into()))
        }
        Some(r) => {
            let id = find_object(&doc.data, r)?;
            let o = doc.data.find_mut(id).expect("objeto");
            if o.prefab.is_some() {
                return Err(format!(
                    "\"{}\" é cópia de um prefab: as regras dele são as do prefab (mude o .prefab)",
                    o.name
                ));
            }
            let name = o.name.clone();
            Ok((&mut o.events, true, format!("\"{name}\"")))
        }
    }
}

fn por_regra(s: &mut Server, a: &Value) -> Result<Output, String> {
    let mut doc = s.open_doc(arg_str(a, "cena")?)?;
    let three_d = doc.three_d();
    let rule_json = a.get("regra").ok_or("falta \"regra\"")?;
    let index = opt_num(a, "posicao")?;
    let replace = opt_num(a, "trocar")?;
    let (rules, in_object, owner) = rules_of(&mut doc, a)?;
    let rule = super::valores::rule_from_json(rule_json, in_object, three_d)?;
    let mut summary = events::rule_summary(&rule, in_object);
    if rule.off {
        summary.push_str(" (DESLIGADA: off = true)");
    }
    if let Some(c) = rule.comment.as_ref().filter(|_| !rule.when.is_empty()) {
        summary = format!("[{c}] {summary}");
    }
    let notes = super::valores::rule_missing_files(&s.root, &rule);
    let at = if let Some(n) = replace {
        let i = n as usize;
        if i == 0 || i > rules.len() {
            return Err(format!("{owner} não tem a regra {i} (tem {})", rules.len()));
        }
        rules[i - 1] = rule;
        i
    } else {
        let i = index
            .map(|p| (p.max(1.0) as usize - 1).min(rules.len()))
            .unwrap_or(rules.len());
        rules.insert(i, rule);
        i + 1
    };
    let total = rules.len();
    s.save_doc(&doc)?;
    let mut msg = format!("Regra {at} de {total} em {owner}: {summary}");
    for n in notes {
        msg.push_str(&format!("\n- {n}"));
    }
    Ok(Output::text(msg))
}

fn tirar_regra(s: &mut Server, a: &Value) -> Result<Output, String> {
    let mut doc = s.open_doc(arg_str(a, "cena")?)?;
    let n =
        opt_num(a, "indice")?.ok_or("falta \"indice\" (o número da regra em ver_cena)")? as usize;
    let (rules, in_object, owner) = rules_of(&mut doc, a)?;
    if n == 0 || n > rules.len() {
        return Err(format!("{owner} não tem a regra {n} (tem {})", rules.len()));
    }
    let r = rules.remove(n - 1);
    s.save_doc(&doc)?;
    Ok(Output::text(format!(
        "Tirei a regra {n} de {owner}: {}",
        events::rule_summary(&r, in_object)
    )))
}

const CENA: &str = "Arquivo da cena (.cena) ou do prefab (.prefab)";

pub static TOOLS: &[&Tool] = &[
    &Tool {
        name: "listar_eventos",
        title: "Listar eventos",
        description: "O catálogo dos eventos \"Quando... Então...\": as condições (tecla, encostar, pegar, acabaram, tempo, variável...) e as ações (trocar de cena, criar, destruir, tocar som, mudar variável, cor, partículas...), com os campos e valores padrão. Com \"busca\", mostra também a ajuda de cada um.",
        params: || {
            vec![
                (
                    "tipo",
                    s_enum("O que listar", &["tudo", "condicoes", "acoes"]),
                    false,
                ),
                (
                    "busca",
                    s_str("Palavras para filtrar (ex.: \"tecla\", \"som\", \"cena\")"),
                    false,
                ),
                (
                    "cena_tipo",
                    s_enum("Só os que valem em cenas 2D ou 3D", &["2d", "3d"]),
                    false,
                ),
            ]
        },
        read_only: true,
        destructive: false,
        run: listar_eventos,
    },
    &Tool {
        name: "por_regra",
        title: "Pôr regra de evento",
        description: "Põe uma regra \"Quando... Então...\" na cena ou num objeto (sem código Lua). Exemplo: { \"when\": [ { \"cond\": \"pegar\", \"quem\": \"@jogador\", \"item\": \"moeda\" } ], \"actions\": [ { \"act\": \"som\", \"som\": \"assets/sons/moeda.wav\" } ] }. Numa regra de objeto, \"@este\" é o próprio objeto. Os ids e campos estão em listar_eventos; tudo é conferido. Com \"trocar\", substitui a regra daquele número.",
        params: || {
            vec![
                ("cena", s_str(CENA), true),
                (
                    "regra",
                    s_obj("A regra: when, actions, mode (\"enquanto\"), off, comment"),
                    true,
                ),
                (
                    "objeto",
                    s_ref("No objeto (id ou nome); sem, a regra é da cena"),
                    false,
                ),
                (
                    "posicao",
                    s_int("Número que a regra vai ter (1 = primeira); sem, vai no fim"),
                    false,
                ),
                ("trocar", s_int("Substitui a regra com este número"), false),
            ]
        },
        read_only: false,
        destructive: false,
        run: por_regra,
    },
    &Tool {
        name: "tirar_regra",
        title: "Tirar regra de evento",
        description: "Tira uma regra pelo número (o \"indice\" que ver_cena mostra).",
        params: || {
            vec![
                ("cena", s_str(CENA), true),
                ("indice", s_int("Número da regra (1 = primeira)"), true),
                (
                    "objeto",
                    s_ref("Do objeto (id ou nome); sem, da cena"),
                    false,
                ),
            ]
        },
        read_only: false,
        destructive: true,
        run: tirar_regra,
    },
];
