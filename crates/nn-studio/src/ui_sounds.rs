//! Os sons da interface (ligados nas Preferências): gerados aqui mesmo, sem
//! arquivos. Curtos e baixos, para não cansar.

use std::f32::consts::TAU;

/// Taxa de amostragem dos sons.
const RATE: u32 = 44_100;

/// Uma nota com ataque rápido e queda suave, somada a partir de `start`
/// segundos.
fn note(out: &mut Vec<f32>, start: f32, freq: f32, len: f32, volume: f32) {
    let first = (start * RATE as f32) as usize;
    let n = (len * RATE as f32) as usize;
    if out.len() < first + n {
        out.resize(first + n, 0.0);
    }
    for i in 0..n {
        let t = i as f32 / RATE as f32;
        let attack = (t / 0.004).min(1.0);
        let decay = (-t * 6.0 / len).exp();
        // Um pouco da oitava de cima deixa o som mais "plim".
        let wave = (TAU * freq * t).sin() + 0.25 * (TAU * 2.0 * freq * t).sin();
        out[first + i] += wave * attack * decay * volume;
    }
}

/// O som de cada coisa: "salvar" (plim de duas notas), "criar" (um toque
/// curto) e "comemorar" (um arpejo). Devolve (taxa, amostras).
pub fn samples(kind: &str) -> (u32, Vec<f32>) {
    let mut out = Vec::new();
    match kind {
        "salvar" => {
            note(&mut out, 0.0, 880.0, 0.12, 0.18);
            note(&mut out, 0.07, 1318.5, 0.2, 0.16);
        }
        "comemorar" => {
            for (i, f) in [523.25, 659.25, 783.99, 1046.5].into_iter().enumerate() {
                note(&mut out, i as f32 * 0.085, f, 0.3, 0.15);
            }
        }
        _ => note(&mut out, 0.0, 1567.98, 0.05, 0.12),
    }
    (RATE, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sons_curtos_e_baixos() {
        for kind in ["salvar", "criar", "comemorar"] {
            let (rate, s) = samples(kind);
            let secs = s.len() as f32 / rate as f32;
            assert!(secs > 0.03 && secs < 0.7, "{kind}: {secs} s");
            let peak = s.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            assert!(peak > 0.05 && peak < 0.5, "{kind}: pico {peak}");
        }
    }
}
