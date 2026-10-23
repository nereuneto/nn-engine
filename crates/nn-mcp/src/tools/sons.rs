//! Efeitos sonoros sintetizados ([`nn_projeto::sfx`]): o agente escolhe um
//! tipo (moeda, pulo, tiro...), varia e ajusta, e recebe a forma da onda e a
//! altura ao longo do tempo (já que não ouve).

use image::{Rgba, RgbaImage};
use serde_json::Value;

use nn_projeto::sfx::{self, FIELDS, KINDS, Sfx};

use super::{Output, Tool, arg_str, opt_num, opt_str, s_int, s_obj, s_str};
use crate::Server;

const W: u32 = 720;
const WAVE_H: u32 = 120;
const PITCH_H: u32 = 60;

fn kinds() -> String {
    KINDS
        .iter()
        .map(|(id, _, tip)| format!("{id} ({tip})"))
        .collect::<Vec<_>>()
        .join("; ")
}

/// A forma da onda (em cima) e a altura (embaixo, de 30 Hz a 4 kHz, em
/// escala de notas).
fn picture(s: &Sfx, samples: &[f32]) -> Result<Vec<u8>, String> {
    let h = WAVE_H + PITCH_H + 2;
    let mut img = RgbaImage::from_pixel(W, h, Rgba([0x1a, 0x1c, 0x2c, 255]));
    let mid = WAVE_H / 2;
    for x in 0..W {
        img.put_pixel(x, mid, Rgba([0x33, 0x3c, 0x57, 255]));
        img.put_pixel(x, WAVE_H + 1, Rgba([0x56, 0x6c, 0x86, 255]));
    }
    for (x, (lo, hi)) in sfx::peaks(samples, W as usize).into_iter().enumerate() {
        let y0 = (mid as f32 - hi * (mid - 2) as f32).round() as u32;
        let y1 = (mid as f32 - lo * (mid - 2) as f32).round() as u32;
        for y in y0.min(mid)..=y1.max(mid).min(WAVE_H - 1) {
            if y >= y0 && y <= y1 {
                img.put_pixel(x as u32, y, Rgba([0x38, 0xb7, 0x64, 255]));
            }
        }
    }
    let dur = s.duration();
    let (lo, hi) = (30f32.ln(), 4000f32.ln());
    let mut last: Option<u32> = None;
    for x in 0..W {
        let t = x as f32 / W as f32 * dur;
        let f = s.freq_at(t).clamp(30.0, 4000.0);
        let k = (f.ln() - lo) / (hi - lo);
        let y = WAVE_H + 2 + ((1.0 - k) * (PITCH_H - 1) as f32).round() as u32;
        let from = last.unwrap_or(y);
        for yy in from.min(y)..=from.max(y) {
            img.put_pixel(x, yy, Rgba([0xff, 0xcd, 0x75, 255]));
        }
        last = Some(y);
    }
    let mut out = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

fn describe(s: &Sfx, kind: &str, variation: u32) -> String {
    let mut fields = vec![format!("onda: {}", s.wave.id())];
    let d = Sfx::default();
    for f in FIELDS {
        let v = s.get(f.key).unwrap_or(0.0);
        // Só o que importa para este som.
        let skip = match f.key {
            "freq_fim" | "deslize" => s.slide == 0.0,
            "vibrato_vel" => s.vibrato == 0.0,
            "arpejo_tempo" => (s.arp - 1.0).abs() < 0.001,
            "pulso" | "pulso_fim" => s.wave != sfx::Wave::Square,
            _ => false,
        };
        if !skip && (Some(v) != d.get(f.key) || matches!(f.key, "freq" | "volume")) {
            fields.push(format!("{}: {}", f.key, (v * 1000.0).round() / 1000.0));
        }
    }
    format!(
        "{}{}, {:.2} s. {}.",
        if kind.is_empty() {
            "Som".to_string()
        } else {
            format!("Tipo {kind}")
        },
        if variation > 0 {
            format!(" (variação {variation})")
        } else {
            String::new()
        },
        s.duration(),
        fields.join(", ")
    )
}

fn criar_som(s: &mut Server, a: &Value) -> Result<Output, String> {
    let (rel, full) = crate::paths::resolve(&s.root, arg_str(a, "caminho")?)?;
    if !rel.to_lowercase().ends_with(".wav") {
        return Err("o caminho termina em .wav (ex.: \"assets/sons/moeda.wav\")".into());
    }
    let side = sfx::sfx_for(&rel);
    let existing = if s.root.join(&side).is_file() {
        let text =
            std::fs::read_to_string(s.root.join(&side)).map_err(|e| format!("{side}: {e}"))?;
        let v: Value = serde_json::from_str(&text).map_err(|e| format!("{side}: {e}"))?;
        Some(Sfx::from_json(&v).map_err(|e| format!("{side}: {e}"))?)
    } else if full.exists() {
        return Err(format!(
            "{rel} já existe e não foi feito aqui (não tem {side}); escolha outro nome"
        ));
    } else {
        None
    };
    let kind_arg = opt_str(a, "tipo")?;
    let variation = match opt_num(a, "variacao")? {
        Some(v) if (0.0..=1_000_000.0).contains(&v) => Some(v as u32),
        Some(_) => return Err("\"variacao\" é um número de 0 a 1000000".into()),
        None => None,
    };
    let changes = match a.get("mudar").filter(|v| !v.is_null()) {
        Some(v) => Some(
            v.as_object()
                .ok_or("\"mudar\" é { campo: valor }, ex.: {\"freq\": 600, \"onda\": \"serra\"}")?
                .clone(),
        ),
        None => None,
    };
    let (mut fx, kind, mut var) = match (kind_arg, existing) {
        (Some(k), _) => {
            let v = variation.unwrap_or(0);
            let fx = Sfx::preset(k, v)
                .ok_or_else(|| format!("não há o tipo \"{k}\". Tipos: {}", kinds()))?;
            (fx, k.to_string(), v)
        }
        (None, Some((fx, k, v))) => match variation {
            // Outra variação do mesmo tipo.
            Some(nv) if nv != v => match Sfx::preset(&k, nv) {
                Some(fx) => (fx, k, nv),
                None => return Err(format!("{side} não diz o tipo; passe \"tipo\"")),
            },
            _ => (fx, k, v),
        },
        (None, None) => {
            return Err(format!("diga o \"tipo\" do som. Tipos: {}", kinds()));
        }
    };
    let only_look = kind_arg.is_none() && variation.is_none() && changes.is_none();
    if let Some(m) = &changes {
        fx.apply(m)?;
    }
    if fx.attack + fx.sustain + fx.decay > sfx::MAX_SECONDS {
        return Err(format!(
            "ataque + sustentar + queda passam de {} s",
            sfx::MAX_SECONDS
        ));
    }
    let samples = fx.render();
    if !only_look {
        s.write_file(&rel, &sfx::wav_bytes(&samples))?;
        s.write_file(&side, fx.text(&kind, var).as_bytes())?;
    }
    if kind.is_empty() {
        var = 0;
    }
    let mut text = if only_look {
        format!("{rel}: {}", describe(&fx, &kind, var))
    } else {
        format!(
            "Gravei {rel} (e os números em {side}). {}",
            describe(&fx, &kind, var)
        )
    };
    text.push_str(&format!(
        "\nPara tocar: nos eventos, a ação \"Tocar som\"; no componente sound (\"sound\": \"{rel}\"); em Lua, nn.audio.play(\"{rel}\").\n\
         Outra versão do mesmo tipo: \"variacao\" com outro número. Ajuste fino: \"mudar\".\n\
         A imagem: a onda (em cima, verde; a altura é o volume) e a nota ao longo do tempo (embaixo, amarelo; mais alto = mais agudo)."
    ));
    Ok(Output::text(text).and_image(&picture(&fx, &samples)?, "image/png"))
}

pub static TOOLS: &[&Tool] = &[&Tool {
    name: "criar_som",
    title: "Criar efeito sonoro",
    description: "Cria um efeito sonoro sintetizado (.wav, como o sfxr) de um tipo pronto e devolve a forma da onda e a nota ao longo do tempo. Tipos: moeda, pulo, tiro, laser, explosao, powerup, dano, clique, blip. \"variacao\" (um número) dá outra versão do mesmo tipo; \"mudar\" ajusta os números: onda (quadrada, serra, triangulo, seno, ruido), freq e freq_fim (Hz), deslize (s), vibrato (semitons) e vibrato_vel, arpejo (multiplica a nota; 1.5 = uma quinta) e arpejo_tempo, repetir (s), pulso e pulso_fim (só quadrada), ataque, sustentar, impacto (0 a 1), queda (s), passa_baixa e passa_alta (Hz; 0 = desligado), volume. Os números ficam num .sfx ao lado: chamar de novo com o mesmo caminho e só \"mudar\" ajusta o som; só o caminho mostra o som.",
    params: || {
        vec![
            (
                "caminho",
                s_str("O .wav, ex.: \"assets/sons/moeda.wav\""),
                true,
            ),
            (
                "tipo",
                s_str("moeda, pulo, tiro, laser, explosao, powerup, dano, clique ou blip"),
                false,
            ),
            (
                "variacao",
                s_int("Outra versão do tipo (0 = a de base; cada número dá sempre o mesmo som)"),
                false,
            ),
            (
                "mudar",
                s_obj("Os números: {\"freq\": 600, \"queda\": 0.4, \"onda\": \"serra\"}"),
                false,
            ),
        ]
    },
    read_only: false,
    destructive: false,
    run: criar_som,
}];
