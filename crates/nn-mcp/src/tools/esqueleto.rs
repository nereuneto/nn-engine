//! Ossos e animações dos modelos 3D (`.modelo`): o agente põe um esqueleto
//! (pronto ou com os ossos dele), liga as peças aos ossos e faz animações
//! (as prontas ou com quadros-chave), vendo o movimento numa tira de
//! quadros. É o mesmo esqueleto e animador do modelador do NN Studio.

use glam::Vec3;
use serde_json::{Map, Value};

use nn_projeto::model::{Anim, Bone, Key, ModelData};
use nn_projeto::model_rig::{self, RigKind};
use nn_projeto::model_view::{self, View};

use super::modelos::{m, part_index, png, usage, vec3};
use super::{
    Output, Tool, arg_str, opt_bool, opt_num, opt_str, s_bool, s_int, s_list, s_num, s_obj, s_str,
};
use crate::Server;

fn rig_kind(name: &str) -> Result<RigKind, String> {
    Ok(match crate::paths::fold(name).as_str() {
        "humanoide" | "pessoa" | "personagem" => RigKind::Humanoid,
        "quadrupede" | "bicho" | "animal" => RigKind::Quadruped,
        "simples" | "um" => RigKind::Simple,
        _ => {
            return Err(format!(
                "\"tipo\" \"{name}\": humanoide (quadril, tronco, cabeça, braços e pernas), quadrupede (corpo, cabeça, rabo e quatro patas) ou simples (um osso: porta, tampa, hélice)"
            ));
        }
    })
}

fn bone_index(md: &ModelData, name: &str) -> Result<usize, String> {
    if let Some(i) = md.bone_index(name) {
        return Ok(i);
    }
    let f = crate::paths::fold(name);
    md.bones
        .iter()
        .position(|b| crate::paths::fold(&b.name) == f)
        .ok_or_else(|| {
            let names: Vec<&str> = md.bones.iter().map(|b| b.name.as_str()).collect();
            if names.is_empty() {
                "o modelo ainda não tem ossos (use esqueleto)".to_string()
            } else {
                format!("não há o osso \"{name}\" (tem: {})", names.join(", "))
            }
        })
}

/// Os ossos e quem segue cada um.
fn describe_bones(md: &ModelData) -> String {
    if md.bones.is_empty() {
        return "Sem ossos.\n".into();
    }
    let v = |a: Vec3| format!("[{}, {}, {}]", m(a.x), m(a.y), m(a.z));
    let mut out = String::from("Ossos (a junta, onde gira, e a ponta):\n");
    for b in &md.bones {
        let parts: Vec<&str> = md
            .parts
            .iter()
            .filter(|p| p.bone == b.name)
            .map(|p| p.name.as_str())
            .collect();
        out.push_str(&format!(
            "  \"{}\"{}: de {} a {}; peças: {}\n",
            b.name,
            b.parent
                .and_then(|p| md.bones.get(p))
                .map(|p| format!(" (filho de \"{}\")", p.name))
                .unwrap_or_default(),
            v(b.head),
            v(b.tail),
            if parts.is_empty() {
                "nenhuma".into()
            } else {
                parts.join(", ")
            }
        ));
    }
    let loose: Vec<&str> = md
        .parts
        .iter()
        .filter(|p| p.bone.is_empty() || md.bone_index(&p.bone).is_none())
        .map(|p| p.name.as_str())
        .collect();
    if !loose.is_empty() {
        out.push_str(&format!(
            "Peças sem osso (ficam paradas nas animações): {}.\n",
            loose.join(", ")
        ));
    }
    out
}

fn describe_anims(md: &ModelData) -> String {
    if md.anims.is_empty() {
        return "Sem animações.\n".into();
    }
    let list: Vec<String> = md
        .anims
        .iter()
        .map(|a| {
            let keys: usize = a.tracks.iter().map(|t| t.keys.len()).sum();
            format!(
                "\"{}\" ({} s, {}, {} osso(s), {} quadro(s)-chave)",
                a.name,
                m(a.duration),
                if a.looping { "repete" } else { "uma vez" },
                a.tracks.len(),
                keys
            )
        })
        .collect();
    format!("Animações: {}.\n", list.join(", "))
}

/// As vistas com os ossos (na pose de descanso).
fn bone_views(md: &ModelData) -> Result<Vec<u8>, String> {
    let data = nn_render::d3::ModelData::from_gltf(&md.to_glb(), &mut |_| None)
        .map_err(|e| e.to_string())?;
    let tris = model_view::triangles(&data, None);
    png(&model_view::render_bones(
        &tris,
        300,
        &md.bone_lines(None, 0.0),
    ))
}

// esqueleto ------------------------------------------------------------------

fn esqueleto(s: &mut Server, a: &Value) -> Result<Output, String> {
    let (rel, mut md) = s.load_model(arg_str(a, "caminho")?)?;
    let mut done = Vec::new();
    if opt_bool(a, "apagar")?.unwrap_or(false) {
        md.bones.clear();
        md.anims.clear();
        for p in &mut md.parts {
            p.bone.clear();
        }
        s.save_model(&rel, &md)?;
        return Ok(Output::text(format!(
            "{rel}: tirei os ossos e as animações."
        )));
    }
    if let Some(t) = opt_str(a, "tipo")? {
        let kind = rig_kind(t)?;
        if md.parts.is_empty() {
            return Err("o modelo não tem peças: o esqueleto se ajusta ao tamanho delas".into());
        }
        md.bones = model_rig::make_rig(kind, &md);
        for p in &mut md.parts {
            p.bone.clear();
        }
        let n = model_rig::auto_bind(&mut md);
        // As animações com ossos que não existem mais perdem essas trilhas.
        let names: Vec<String> = md.bones.iter().map(|b| b.name.clone()).collect();
        for an in &mut md.anims {
            an.tracks.retain(|t| names.contains(&t.bone));
        }
        done.push(format!(
            "esqueleto {} com {} osso(s), {n} peça(s) ligadas (pelo nome ou pelo osso mais perto)",
            t,
            md.bones.len()
        ));
    }
    if let Some(list) = a.get("ossos").filter(|v| !v.is_null()) {
        let list = list.as_array().ok_or(
            "\"ossos\" é uma lista de {\"nome\", \"pai\", \"de\": [x, y, z], \"ate\": [x, y, z]}",
        )?;
        for (i, b) in list.iter().enumerate() {
            let b = b
                .as_object()
                .ok_or_else(|| format!("osso {}: precisa ser um objeto", i + 1))?;
            let name = b
                .get("nome")
                .and_then(Value::as_str)
                .filter(|n| !n.trim().is_empty())
                .ok_or_else(|| format!("osso {}: falta o \"nome\"", i + 1))?
                .to_string();
            let head = vec3(
                b.get("de")
                    .ok_or_else(|| format!("osso \"{name}\": falta \"de\" (a junta)"))?,
                "\"de\"",
            )?;
            let tail = vec3(
                b.get("ate")
                    .ok_or_else(|| format!("osso \"{name}\": falta \"ate\" (a ponta)"))?,
                "\"ate\"",
            )?;
            let parent = match b
                .get("pai")
                .and_then(Value::as_str)
                .filter(|p| !p.is_empty())
            {
                Some(p) => Some(bone_index(&md, p).map_err(|e| format!("osso \"{name}\": {e}"))?),
                None => None,
            };
            match md.bone_index(&name) {
                Some(k) => {
                    // Muda um que já existe (o pai continua antes dele).
                    if parent.is_some_and(|p| p >= k) {
                        return Err(format!("osso \"{name}\": o pai precisa vir antes dele"));
                    }
                    md.bones[k] = Bone {
                        name: name.clone(),
                        parent,
                        head,
                        tail,
                    };
                    done.push(format!("osso \"{name}\" mudou"));
                }
                None => {
                    md.bones.push(Bone {
                        name: name.clone(),
                        parent,
                        head,
                        tail,
                    });
                    done.push(format!("osso \"{name}\""));
                }
            }
        }
    }
    if let Some(map) = a.get("ligar").filter(|v| !v.is_null()) {
        let map = map
            .as_object()
            .ok_or("\"ligar\" é {\"peça\": \"osso\"} (\"\" desliga)")?;
        for (part, bone) in map {
            let pi = part_index(&md, part)?;
            let bone = bone.as_str().ok_or("em \"ligar\", o osso é um texto")?;
            if bone.is_empty() {
                md.parts[pi].bone.clear();
            } else {
                let bi = bone_index(&md, bone)?;
                md.parts[pi].bone = md.bones[bi].name.clone();
            }
        }
        done.push(format!("{} peça(s) ligadas", map.len()));
    }
    if opt_bool(a, "ligar_sozinho")?.unwrap_or(false) {
        if md.bones.is_empty() {
            return Err("sem ossos para ligar (ponha com \"tipo\" ou \"ossos\")".into());
        }
        // Só as que ainda não têm osso.
        let had: Vec<String> = md.parts.iter().map(|p| p.bone.clone()).collect();
        model_rig::auto_bind(&mut md);
        for (p, old) in md.parts.iter_mut().zip(had) {
            if !old.is_empty() {
                p.bone = old;
            }
        }
        done.push("as peças sem osso foram ligadas ao mais perto".into());
    }
    if done.is_empty() {
        return Err(
            "diga o que fazer: tipo (um esqueleto pronto), ossos, ligar, ligar_sozinho ou apagar"
                .into(),
        );
    }
    s.save_model(&rel, &md)?;
    let mut text = format!(
        "{rel}: {}.\n{}{}{}",
        done.join("; "),
        describe_bones(&md),
        describe_anims(&md),
        usage(&rel)
    );
    if let Some(kind) = model_rig::kind_of(&md.bones) {
        text.push_str(&format!(
            "\nAnimações prontas para este esqueleto (animar_modelo com \"pronta\"): {}.",
            model_rig::ready_names(kind).join(", ")
        ));
    }
    text.push_str(
        "\nAs vistas com os ossos: em verde, da junta (o ponto branco, onde gira) até a ponta.",
    );
    Ok(Output::text(text).and_image(&bone_views(&md)?, "image/png"))
}

// animar_modelo --------------------------------------------------------------

fn animar_modelo(s: &mut Server, a: &Value) -> Result<Output, String> {
    let (rel, mut md) = s.load_model(arg_str(a, "caminho")?)?;
    if md.bones.is_empty() {
        return Err(format!(
            "{rel} ainda não tem ossos: ponha com esqueleto (\"tipo\": \"humanoide\", por exemplo)"
        ));
    }
    let name = arg_str(a, "nome")?.trim().to_string();
    if name.is_empty() {
        return Err("falta o \"nome\" da animação (\"andar\", \"parado\"...)".into());
    }
    if opt_bool(a, "apagar")?.unwrap_or(false) {
        let before = md.anims.len();
        md.anims.retain(|x| x.name != name);
        if md.anims.len() == before {
            return Err(format!("{rel} não tem a animação \"{name}\""));
        }
        s.save_model(&rel, &md)?;
        return Ok(Output::text(format!(
            "{rel}: apaguei a animação \"{name}\".\n{}",
            describe_anims(&md)
        )));
    }
    let kind = model_rig::kind_of(&md.bones).unwrap_or(RigKind::Simple);
    // Uma animação nova com chaves próprias dura até a última chave (se não
    // vier a "duracao").
    let mut fresh = false;
    let mut anim = match opt_str(a, "pronta")? {
        Some(p) => {
            let mut an = model_rig::ready(kind, p, &md.bones).ok_or_else(|| {
                format!(
                    "não há a animação pronta \"{p}\" para este esqueleto (tem: {})",
                    model_rig::ready_names(kind).join(", ")
                )
            })?;
            an.name = name.clone();
            an
        }
        None => match md.anims.iter().find(|x| x.name == name) {
            Some(x) if !opt_bool(a, "limpar")?.unwrap_or(false) => x.clone(),
            _ => {
                fresh = true;
                Anim::new(&name, 1.0, true)
            }
        },
    };
    if let Some(f) = opt_num(a, "forca")? {
        if !(-5.0..=5.0).contains(&f) {
            return Err("\"forca\" vai de -5 a 5 (1 = como é; 0.5 = metade do movimento; -1 = para o outro lado)".into());
        }
        model_rig::scale_intensity(&mut anim, f as f32);
    }
    if let Some(v) = opt_num(a, "velocidade")? {
        if !(0.1..=10.0).contains(&v) {
            return Err("\"velocidade\" vai de 0.1 a 10 (2 = duas vezes mais rápida)".into());
        }
        model_rig::scale_time(&mut anim, 1.0 / v as f32);
    }
    let duration = opt_num(a, "duracao")?;
    if let Some(d) = duration {
        if !(0.05..=60.0).contains(&d) {
            return Err("\"duracao\" vai de 0.05 a 60 segundos".into());
        }
        anim.duration = d as f32;
    }
    if let Some(r) = opt_bool(a, "repete")? {
        anim.looping = r;
    }
    if let Some(list) = a.get("chaves").filter(|v| !v.is_null()) {
        let list = list
            .as_array()
            .ok_or("\"chaves\" é uma lista de {\"osso\", \"t\", \"giro\": [x, y, z], \"andar\": [x, y, z]}")?;
        for (i, k) in list.iter().enumerate() {
            let k: &Map<String, Value> = k
                .as_object()
                .ok_or_else(|| format!("chave {}: precisa ser um objeto", i + 1))?;
            let bone = k
                .get("osso")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("chave {}: falta o \"osso\"", i + 1))?;
            let bi = bone_index(&md, bone).map_err(|e| format!("chave {}: {e}", i + 1))?;
            let t = k
                .get("t")
                .and_then(Value::as_f64)
                .ok_or_else(|| format!("chave {}: falta \"t\" (o tempo, em segundos)", i + 1))?
                as f32;
            if t < 0.0 {
                return Err(format!("chave {}: \"t\" não pode ser negativo", i + 1));
            }
            let rot = match k.get("giro") {
                Some(v) => vec3(v, "\"giro\"").map_err(|e| format!("chave {}: {e}", i + 1))?,
                None => Vec3::ZERO,
            };
            let off = match k.get("andar") {
                Some(v) => vec3(v, "\"andar\"").map_err(|e| format!("chave {}: {e}", i + 1))?,
                None => Vec3::ZERO,
            };
            let bname = md.bones[bi].name.clone();
            anim.track_mut(&bname).set(Key {
                t,
                rot,
                offset: off,
            });
            if t > anim.duration {
                anim.duration = t;
            }
        }
    }
    if fresh && duration.is_none() {
        let last = anim
            .tracks
            .iter()
            .flat_map(|t| &t.keys)
            .map(|k| k.t)
            .fold(0.0, f32::max);
        if last > 0.0 {
            anim.duration = last;
        }
    }
    if anim.tracks.iter().all(|t| t.keys.is_empty()) {
        return Err("a animação não tem nenhum quadro-chave: use \"pronta\" ou \"chaves\"".into());
    }
    match md.anims.iter().position(|x| x.name == name) {
        Some(i) => md.anims[i] = anim.clone(),
        None => md.anims.push(anim.clone()),
    }
    s.save_model(&rel, &md)?;

    // A tira: o modelo em vários momentos (pela engine: o .glb que o jogo usa).
    let n = opt_num(a, "quadros")?.unwrap_or(6.0).clamp(2.0, 12.0) as usize;
    let view = match crate::paths::fold(opt_str(a, "vista")?.unwrap_or("lado")).as_str() {
        "lado" => View::Side,
        "frente" => View::Front,
        "cima" => View::Top,
        "diagonal" | "3/4" => View::ThreeQuarter,
        "tras" | "costas" => View::Back,
        _ => return Err("\"vista\": lado (padrão), frente, cima, diagonal ou tras".into()),
    };
    let data = nn_render::d3::ModelData::from_gltf(&md.to_glb(), &mut |_| None)
        .map_err(|e| e.to_string())?;
    let gi = data
        .skeleton
        .as_ref()
        .and_then(|sk| sk.animation(&name))
        .ok_or("a animação não foi para o .glb")?;
    let times: Vec<f32> = (0..n)
        .map(|i| {
            if anim.looping {
                anim.duration * i as f32 / n as f32
            } else {
                anim.duration * i as f32 / (n - 1) as f32
            }
        })
        .collect();
    let frames: Vec<_> = times
        .iter()
        .map(|t| {
            (
                model_view::triangles(&data, Some((gi, *t))),
                md.bone_lines(Some(&anim), *t),
            )
        })
        .collect();
    let strip = model_view::render_strip(&frames, view, 200);
    let keys: usize = anim.tracks.iter().map(|t| t.keys.len()).sum();
    let mut text = format!(
        "{rel}: animação \"{name}\", {} s, {}, {} osso(s), {keys} quadro(s)-chave.\n{}",
        m(anim.duration),
        if anim.looping {
            "repete"
        } else {
            "toca uma vez"
        },
        anim.tracks.len(),
        describe_anims(&md)
    );
    text.push_str(&format!(
        "No jogo: {{\"type\": \"model\", \"file\": \"{}\"}}. O Jogador 3D toca sozinho parado, andar, correr, pular e cair (e o Golpe, golpe); o Inimigo 3D, parado e andar. Outra animação: a ação \"Tocar animação\" (id animacao) numa regra, ou o componente anim3d com \"animation\".\n",
        nn_projeto::model::glb_path(&rel)
    ));
    text.push_str(&format!(
        "A tira: {n} momentos (em {} s), vistos {}, no mesmo enquadramento; os ossos em verde.",
        times.iter().map(|t| m(*t)).collect::<Vec<_>>().join(", "),
        match view {
            View::Side => "de lado (de +X: a frente fica à esquerda)",
            View::Front => "de frente",
            View::Top => "de cima",
            View::ThreeQuarter => "na diagonal",
            View::Back => "de trás",
        }
    ));
    Ok(Output::text(text).and_image(&png(&strip)?, "image/png"))
}

const MODELO: &str = "O modelo (.modelo), ex.: \"modelos/heroi.modelo\"";

pub static TOOLS: &[&Tool] = &[
    &Tool {
        name: "esqueleto",
        title: "Esqueleto do modelo",
        description: "Põe ossos num modelo 3D (.modelo) para animar: um esqueleto pronto (\"tipo\": humanoide, quadrupede ou simples) que se ajusta ao tamanho do modelo e liga cada peça a um osso (pelo nome da peça, ex.: \"Braço esquerdo\", ou pelo osso mais perto); ossos próprios (\"ossos\": [{\"nome\": \"Asa\", \"pai\": \"Corpo\", \"de\": [x, y, z] (a junta, onde gira), \"ate\": [x, y, z] (a ponta)}]; numa porta, um osso só com a junta na dobradiça); \"ligar\": {\"Espada\": \"Braço direito\"} (a peça segue o osso inteira; \"\" desliga); \"ligar_sozinho\": true (as peças sem osso vão para o mais perto); \"apagar\": true. Cada peça segue um osso sem entortar (o jeito dos bonecos de peças). Volta os ossos e as peças de cada um, e as vistas com os ossos desenhados. Depois: animar_modelo.",
        params: || {
            vec![
                ("caminho", s_str(MODELO), true),
                (
                    "tipo",
                    s_str("humanoide, quadrupede ou simples (troca o esqueleto todo)"),
                    false,
                ),
                (
                    "ossos",
                    s_list("Ossos próprios (ou mudar um pelo nome)", s_obj("Um osso")),
                    false,
                ),
                ("ligar", s_obj("{\"peça\": \"osso\"}"), false),
                (
                    "ligar_sozinho",
                    s_bool("Ligar as peças sem osso ao mais perto"),
                    false,
                ),
                ("apagar", s_bool("Tirar os ossos e as animações"), false),
            ]
        },
        read_only: false,
        destructive: true,
        run: esqueleto,
    },
    &Tool {
        name: "animar_modelo",
        title: "Animar modelo 3D",
        description: "Cria ou muda uma animação de um modelo 3D com ossos (.modelo; ponha os ossos com esqueleto) e devolve uma tira com o modelo em vários momentos dela. \"pronta\": uma animação pronta do esqueleto (humanoide: parado, andar, correr, pular, cair, golpe, acenar; quadrupede: parado, andar, correr; simples: girar, balançar, abrir (uma tampa, em X), abrir_porta (em Y, na junta do osso)), com \"forca\" (1 = como é, 0.5 = metade do movimento, -1 = para o outro lado) e \"velocidade\" (2 = duas vezes mais rápida). Ou quadros-chave próprios: \"chaves\": [{\"osso\": \"Braço esquerdo\", \"t\": 0.5 (segundos), \"giro\": [x, y, z] (graus na junta; num personagem virado para +Z, giro negativo em X leva o braço ou a perna para a frente), \"andar\": [x, y, z] (metros que a junta anda, ex.: o quadril subindo)}], entre os quadros o movimento é suave; \"duracao\" (s) e \"repete\" (true: andar, parado; false: pular, golpe). Com o mesmo \"nome\", muda a animação (as chaves se somam; \"limpar\": true começa do zero). Os nomes que o Jogador 3D toca sozinho: parado, andar, correr, pular, cair.",
        params: || {
            vec![
                ("caminho", s_str(MODELO), true),
                (
                    "nome",
                    s_str("O nome da animação (\"andar\", \"parado\", \"acenar\"...)"),
                    true,
                ),
                ("pronta", s_str("Uma animação pronta do esqueleto"), false),
                ("forca", s_num("Quanto mexe (1 = como é)"), false),
                (
                    "velocidade",
                    s_num("Mais rápida (2) ou mais lenta (0.5)"),
                    false,
                ),
                (
                    "chaves",
                    s_list(
                        "Quadros-chave: {\"osso\", \"t\", \"giro\", \"andar\"}",
                        s_obj("Um quadro-chave"),
                    ),
                    false,
                ),
                ("duracao", s_num("Segundos"), false),
                (
                    "repete",
                    s_bool("Repete sem parar (padrão true numa nova)"),
                    false,
                ),
                ("limpar", s_bool("Começar a animação do zero"), false),
                ("apagar", s_bool("Apagar a animação"), false),
                (
                    "vista",
                    s_str("A tira vista de: lado (padrão), frente, cima, diagonal ou tras"),
                    false,
                ),
                (
                    "quadros",
                    s_int("Quantos momentos na tira (padrão 6)"),
                    false,
                ),
            ]
        },
        read_only: false,
        destructive: false,
        run: animar_modelo,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tipos_de_esqueleto() {
        assert_eq!(rig_kind("Humanoide").unwrap(), RigKind::Humanoid);
        assert_eq!(rig_kind("quadrúpede").unwrap(), RigKind::Quadruped);
        assert!(rig_kind("aranha").unwrap_err().contains("humanoide"));
        let md = ModelData::default();
        assert!(
            bone_index(&md, "x")
                .unwrap_err()
                .contains("ainda não tem ossos")
        );
    }
}
