//! Efeitos sonoros sintetizados (como o sfxr): moeda, pulo, tiro,
//! explosão... Um punhado de números vira um `.wav`. Os números ficam ao lado,
//! num `.sfx` (JSON), para ajustar depois.

use serde_json::{Map, Value, json};

/// Amostras por segundo do `.wav`.
pub const RATE: u32 = 44_100;
/// O som mais longo.
pub const MAX_SECONDS: f32 = 5.0;

/// A forma da onda.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wave {
    /// Quadrada (com o pulso): o som de videogame antigo.
    Square,
    /// Dente de serra: mais áspera (tiros, motores).
    Saw,
    Triangle,
    /// Seno: limpa e suave.
    Sine,
    /// Ruído: explosões, batidas, dano.
    Noise,
}

impl Wave {
    pub const ALL: [Wave; 5] = [
        Wave::Square,
        Wave::Saw,
        Wave::Triangle,
        Wave::Sine,
        Wave::Noise,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Wave::Square => "quadrada",
            Wave::Saw => "serra",
            Wave::Triangle => "triangulo",
            Wave::Sine => "seno",
            Wave::Noise => "ruido",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Wave::Square => "Quadrada",
            Wave::Saw => "Serra",
            Wave::Triangle => "Triângulo",
            Wave::Sine => "Seno",
            Wave::Noise => "Ruído",
        }
    }

    pub fn from_id(id: &str) -> Option<Wave> {
        Wave::ALL.into_iter().find(|w| w.id() == id)
    }
}

/// Os tipos prontos.
pub const KINDS: &[(&str, &str, &str)] = &[
    (
        "moeda",
        "Moeda",
        "Pegar uma moeda ou um item: dois tons subindo",
    ),
    ("pulo", "Pulo", "Um tom que sobe rápido"),
    ("tiro", "Tiro", "Um disparo que cai de agudo para grave"),
    ("laser", "Laser", "Um zap fino descendo"),
    ("explosao", "Explosão", "Ruído grave que vai sumindo"),
    ("powerup", "Power-up", "Subindo e repetindo, com vibrato"),
    ("dano", "Dano", "Uma batida curta e áspera"),
    ("clique", "Clique", "Clique curto de interface"),
    ("blip", "Blip", "Um bipe de menu ou de texto"),
];

/// Um efeito: os números que viram o som.
#[derive(Debug, Clone, PartialEq)]
pub struct Sfx {
    pub wave: Wave,
    /// Frequência no começo (Hz).
    pub freq: f32,
    /// Frequência no fim do deslize (Hz).
    pub freq_end: f32,
    /// Tempo para ir de `freq` a `freq_end` (s; 0 = não desliza).
    pub slide: f32,
    /// Vibrato em semitons e a velocidade dele (Hz).
    pub vibrato: f32,
    pub vibrato_speed: f32,
    /// Depois de `arp_time` segundos, a frequência é multiplicada por `arp`
    /// (1 = não muda; 1.5 = uma quinta acima).
    pub arp: f32,
    pub arp_time: f32,
    /// Recomeça a frequência a cada tanto (s; 0 = não repete).
    pub repeat: f32,
    /// Largura do pulso da onda quadrada (0.5 = cheia; 0.1 = fina), no
    /// começo e no fim.
    pub duty: f32,
    pub duty_end: f32,
    /// Volume ao longo do tempo: sobe em `attack`, fica em `sustain` (com
    /// um soco no começo, `punch`) e some em `decay` (segundos).
    pub attack: f32,
    pub sustain: f32,
    pub punch: f32,
    pub decay: f32,
    /// Filtros (Hz; 0 = desligado): passa-baixa abafa, passa-alta tira o
    /// grave.
    pub lowpass: f32,
    pub highpass: f32,
    pub volume: f32,
}

impl Default for Sfx {
    fn default() -> Self {
        Self {
            wave: Wave::Square,
            freq: 440.0,
            freq_end: 440.0,
            slide: 0.0,
            vibrato: 0.0,
            vibrato_speed: 8.0,
            arp: 1.0,
            arp_time: 0.1,
            repeat: 0.0,
            duty: 0.5,
            duty_end: 0.5,
            attack: 0.0,
            sustain: 0.1,
            punch: 0.0,
            decay: 0.2,
            lowpass: 0.0,
            highpass: 0.0,
            volume: 0.5,
        }
    }
}

/// Um campo ajustável: (chave no `.sfx` e no MCP, nome, dica, mínimo, máximo).
#[derive(Debug)]
pub struct Field {
    pub key: &'static str,
    pub label: &'static str,
    pub tip: &'static str,
    pub min: f32,
    pub max: f32,
}

const fn field(
    key: &'static str,
    label: &'static str,
    tip: &'static str,
    min: f32,
    max: f32,
) -> Field {
    Field {
        key,
        label,
        tip,
        min,
        max,
    }
}

/// Os campos numéricos (a onda é à parte: `onda`).
pub const FIELDS: &[Field] = &[
    field(
        "freq",
        "Frequência",
        "Hz no começo (440 = lá; mais alto = mais agudo)",
        30.0,
        4000.0,
    ),
    field(
        "freq_fim",
        "Frequência no fim",
        "Hz no fim do deslize",
        30.0,
        4000.0,
    ),
    field(
        "deslize",
        "Deslize",
        "Segundos para ir de freq a freq_fim (0 = não desliza)",
        0.0,
        2.0,
    ),
    field(
        "vibrato",
        "Vibrato",
        "Semitons para cima e para baixo",
        0.0,
        4.0,
    ),
    field(
        "vibrato_vel",
        "Velocidade do vibrato",
        "Vezes por segundo",
        0.5,
        30.0,
    ),
    field(
        "arpejo",
        "Arpejo",
        "Multiplica a frequência depois de arpejo_tempo (1 = nada; 1.5 = uma quinta)",
        0.5,
        3.0,
    ),
    field(
        "arpejo_tempo",
        "Tempo do arpejo",
        "Segundos até o arpejo",
        0.0,
        1.0,
    ),
    field(
        "repetir",
        "Repetir",
        "Recomeça a frequência a cada tanto (s; 0 = não)",
        0.0,
        1.0,
    ),
    field(
        "pulso",
        "Pulso",
        "Só na quadrada: 0.5 cheia, 0.1 fina e nasal",
        0.05,
        0.5,
    ),
    field(
        "pulso_fim",
        "Pulso no fim",
        "O pulso no fim do som",
        0.05,
        0.5,
    ),
    field(
        "ataque",
        "Ataque",
        "Segundos para chegar no volume",
        0.0,
        1.0,
    ),
    field(
        "sustentar",
        "Sustentar",
        "Segundos no volume cheio",
        0.0,
        2.0,
    ),
    field(
        "impacto",
        "Impacto",
        "Um soco de volume no começo (0 a 1)",
        0.0,
        1.0,
    ),
    field("queda", "Queda", "Segundos sumindo", 0.0, 3.0),
    field(
        "passa_baixa",
        "Passa-baixa",
        "Abafa acima destes Hz (0 = desligado)",
        0.0,
        20000.0,
    ),
    field(
        "passa_alta",
        "Passa-alta",
        "Tira o grave abaixo destes Hz (0 = desligado)",
        0.0,
        5000.0,
    ),
    field("volume", "Volume", "0 a 1", 0.0, 1.0),
];

/// Gerador simples e repetível (xorshift).
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }

    /// `v` com até `k` (fração) a mais ou a menos.
    fn jitter(&mut self, v: f32, k: f32) -> f32 {
        v * (1.0 + (self.next() * 2.0 - 1.0) * k)
    }

    fn pick<T: Copy>(&mut self, list: &[T]) -> T {
        list[((self.next() * list.len() as f32) as usize).min(list.len() - 1)]
    }
}

impl Sfx {
    /// Um tipo pronto (`KINDS`). A `variation` 0 é o som de base; as outras
    /// mudam um pouco (cada número dá sempre o mesmo som).
    pub fn preset(kind: &str, variation: u32) -> Option<Sfx> {
        let d = Sfx::default();
        let mut s = match kind {
            "moeda" => Sfx {
                freq: 988.0,
                freq_end: 988.0,
                arp: 1.335,
                arp_time: 0.07,
                sustain: 0.06,
                punch: 0.4,
                decay: 0.3,
                ..d
            },
            "pulo" => Sfx {
                duty: 0.3,
                duty_end: 0.3,
                freq: 220.0,
                freq_end: 520.0,
                slide: 0.2,
                sustain: 0.08,
                decay: 0.18,
                highpass: 80.0,
                ..d
            },
            "tiro" => Sfx {
                wave: Wave::Saw,
                freq: 1000.0,
                freq_end: 180.0,
                slide: 0.18,
                sustain: 0.03,
                punch: 0.3,
                decay: 0.15,
                highpass: 200.0,
                ..d
            },
            "laser" => Sfx {
                duty: 0.15,
                duty_end: 0.4,
                freq: 1600.0,
                freq_end: 300.0,
                slide: 0.12,
                sustain: 0.05,
                decay: 0.1,
                ..d
            },
            "explosao" => Sfx {
                wave: Wave::Noise,
                freq: 180.0,
                freq_end: 50.0,
                slide: 0.6,
                sustain: 0.1,
                punch: 0.6,
                decay: 0.7,
                lowpass: 3000.0,
                volume: 0.7,
                ..d
            },
            "powerup" => Sfx {
                freq: 330.0,
                freq_end: 900.0,
                slide: 0.12,
                repeat: 0.12,
                sustain: 0.25,
                decay: 0.25,
                vibrato: 0.3,
                vibrato_speed: 12.0,
                ..d
            },
            "dano" => Sfx {
                wave: Wave::Noise,
                freq: 900.0,
                freq_end: 200.0,
                slide: 0.15,
                sustain: 0.02,
                punch: 0.3,
                decay: 0.18,
                lowpass: 4000.0,
                ..d
            },
            "clique" => Sfx {
                freq: 1400.0,
                freq_end: 1400.0,
                sustain: 0.005,
                decay: 0.03,
                highpass: 300.0,
                volume: 0.4,
                ..d
            },
            "blip" => Sfx {
                freq: 660.0,
                freq_end: 660.0,
                sustain: 0.04,
                decay: 0.06,
                volume: 0.4,
                ..d
            },
            _ => return None,
        };
        if variation > 0 {
            let mut r = Rng::new(u64::from(variation) ^ (kind.len() as u64 * 7919));
            let k = r.jitter(1.0, 0.25);
            s.freq *= k;
            s.freq_end *= r.jitter(k, 0.12);
            s.slide = r.jitter(s.slide, 0.25);
            s.sustain = r.jitter(s.sustain, 0.25);
            s.decay = r.jitter(s.decay, 0.25);
            s.punch = (s.punch + (r.next() - 0.5) * 0.3).clamp(0.0, 1.0);
            if s.wave == Wave::Square {
                s.duty = (s.duty + (r.next() - 0.5) * 0.2).clamp(0.1, 0.5);
                s.duty_end = (s.duty_end + (r.next() - 0.5) * 0.2).clamp(0.1, 0.5);
            }
            match kind {
                "moeda" => {
                    s.arp = r.pick(&[1.25, 1.335, 1.5, 2.0]);
                    s.arp_time = r.jitter(s.arp_time, 0.3);
                }
                "tiro" => s.wave = r.pick(&[Wave::Saw, Wave::Square, Wave::Saw]),
                "powerup" => {
                    s.repeat = r.jitter(s.repeat, 0.3);
                    s.vibrato = r.pick(&[0.0, 0.3, 0.6]);
                }
                "dano" => s.wave = r.pick(&[Wave::Noise, Wave::Noise, Wave::Square]),
                _ => {}
            }
        }
        for f in FIELDS {
            let v = s.get(f.key).unwrap_or(0.0);
            let _ = s.set(f.key, v);
        }
        Some(s)
    }

    /// A duração em segundos.
    pub fn duration(&self) -> f32 {
        (self.attack + self.sustain + self.decay).clamp(0.01, MAX_SECONDS)
    }

    pub fn get(&self, key: &str) -> Option<f32> {
        Some(match key {
            "freq" => self.freq,
            "freq_fim" => self.freq_end,
            "deslize" => self.slide,
            "vibrato" => self.vibrato,
            "vibrato_vel" => self.vibrato_speed,
            "arpejo" => self.arp,
            "arpejo_tempo" => self.arp_time,
            "repetir" => self.repeat,
            "pulso" => self.duty,
            "pulso_fim" => self.duty_end,
            "ataque" => self.attack,
            "sustentar" => self.sustain,
            "impacto" => self.punch,
            "queda" => self.decay,
            "passa_baixa" => self.lowpass,
            "passa_alta" => self.highpass,
            "volume" => self.volume,
            _ => return None,
        })
    }

    /// Muda um campo (dentro dos limites dele). `Err` se o campo não existe.
    pub fn set(&mut self, key: &str, v: f32) -> Result<(), String> {
        let f = FIELDS.iter().find(|f| f.key == key).ok_or_else(|| {
            let all: Vec<&str> = FIELDS.iter().map(|f| f.key).collect();
            format!("não há o campo \"{key}\" (tem: onda, {})", all.join(", "))
        })?;
        if !v.is_finite() {
            return Err(format!("\"{key}\" precisa ser um número"));
        }
        // Três casas, como no `.sfx`: o som gravado é o que o arquivo diz.
        let v = (v.clamp(f.min, f.max) * 1000.0).round() / 1000.0;
        let slot = match key {
            "freq" => &mut self.freq,
            "freq_fim" => &mut self.freq_end,
            "deslize" => &mut self.slide,
            "vibrato" => &mut self.vibrato,
            "vibrato_vel" => &mut self.vibrato_speed,
            "arpejo" => &mut self.arp,
            "arpejo_tempo" => &mut self.arp_time,
            "repetir" => &mut self.repeat,
            "pulso" => &mut self.duty,
            "pulso_fim" => &mut self.duty_end,
            "ataque" => &mut self.attack,
            "sustentar" => &mut self.sustain,
            "impacto" => &mut self.punch,
            "queda" => &mut self.decay,
            "passa_baixa" => &mut self.lowpass,
            "passa_alta" => &mut self.highpass,
            _ => &mut self.volume,
        };
        *slot = v;
        Ok(())
    }

    /// Os números como JSON (o `.sfx`), com o tipo e a variação de origem.
    pub fn to_json(&self, kind: &str, variation: u32) -> Value {
        let mut m = Map::new();
        m.insert("tipo".into(), json!(kind));
        m.insert("variacao".into(), json!(variation));
        m.insert("onda".into(), json!(self.wave.id()));
        for f in FIELDS {
            let v = self.get(f.key).unwrap_or(0.0);
            m.insert(
                f.key.into(),
                json!((f64::from(v) * 1000.0).round() / 1000.0),
            );
        }
        Value::Object(m)
    }

    pub fn text(&self, kind: &str, variation: u32) -> String {
        let mut t =
            serde_json::to_string_pretty(&self.to_json(kind, variation)).unwrap_or_default();
        t.push('\n');
        t
    }

    /// Lê um `.sfx`: (efeito, tipo, variação). Campos que faltam ficam no
    /// padrão.
    pub fn from_json(v: &Value) -> Result<(Sfx, String, u32), String> {
        let m = v.as_object().ok_or("um .sfx é um objeto JSON")?;
        let kind = m
            .get("tipo")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let variation = m.get("variacao").and_then(Value::as_u64).unwrap_or(0) as u32;
        let mut s = Sfx::default();
        s.apply(m)?;
        Ok((s, kind, variation))
    }

    /// Aplica campos de um objeto JSON (`onda` e os de `FIELDS`); ignora
    /// `tipo` e `variacao`.
    pub fn apply(&mut self, m: &Map<String, Value>) -> Result<(), String> {
        for (k, v) in m {
            match k.as_str() {
                "tipo" | "variacao" => {}
                "onda" => {
                    let id = v.as_str().unwrap_or("");
                    self.wave = Wave::from_id(id).ok_or_else(|| {
                        let all: Vec<&str> = Wave::ALL.iter().map(|w| w.id()).collect();
                        format!("onda \"{id}\" não existe (use {})", all.join(", "))
                    })?;
                }
                _ => {
                    let n = v
                        .as_f64()
                        .ok_or_else(|| format!("\"{k}\" precisa ser um número"))?;
                    self.set(k, n as f32)?;
                }
            }
        }
        Ok(())
    }

    /// A frequência (Hz) no segundo `t`: deslize, arpejo, repetição e
    /// vibrato.
    pub fn freq_at(&self, t: f32) -> f32 {
        let tr = if self.repeat > 0.005 {
            t % self.repeat
        } else {
            t
        };
        let mut f = if self.slide > 0.0 {
            let k = (tr / self.slide).min(1.0);
            self.freq * (self.freq_end / self.freq.max(1.0)).powf(k)
        } else {
            self.freq
        };
        if (self.arp - 1.0).abs() > 0.001 && tr >= self.arp_time {
            f *= self.arp;
        }
        if self.vibrato > 0.0 {
            let st = self.vibrato * (std::f32::consts::TAU * self.vibrato_speed * t).sin();
            f *= (st / 12.0).exp2();
        }
        f.clamp(20.0, 20_000.0)
    }

    /// As amostras (mono, -1 a 1, [`RATE`] por segundo).
    pub fn render(&self) -> Vec<f32> {
        let rate = RATE as f32;
        let total = self.duration();
        let n = (total * rate) as usize;
        let mut out = Vec::with_capacity(n);
        let mut rng = Rng::new(0x5EED);
        let mut phase = 0.0f32;
        let mut noise = rng.next() * 2.0 - 1.0;
        let mut noise_step = 0u32;
        let (mut lp, mut hp_lp) = (0.0f32, 0.0f32);
        let a_lp = if self.lowpass > 0.0 {
            1.0 - (-std::f32::consts::TAU * self.lowpass / rate).exp()
        } else {
            1.0
        };
        let a_hp = if self.highpass > 0.0 {
            1.0 - (-std::f32::consts::TAU * self.highpass / rate).exp()
        } else {
            0.0
        };
        for i in 0..n {
            let t = i as f32 / rate;
            let f = self.freq_at(t);
            let duty = self.duty + (self.duty_end - self.duty) * (t / total);
            phase += f / rate;
            if phase >= 1.0 {
                phase -= phase.floor();
            }
            let wave = match self.wave {
                Wave::Square => {
                    if phase < duty {
                        1.0
                    } else {
                        -1.0
                    }
                }
                Wave::Saw => 2.0 * phase - 1.0,
                Wave::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
                Wave::Sine => (std::f32::consts::TAU * phase).sin(),
                Wave::Noise => {
                    // Um valor novo 8 vezes por ciclo: o ruído tem "altura".
                    let step = (phase * 8.0) as u32;
                    if step != noise_step {
                        noise_step = step;
                        noise = rng.next() * 2.0 - 1.0;
                    }
                    noise
                }
            };
            // O volume ao longo do tempo.
            let env = if t < self.attack {
                t / self.attack
            } else if t < self.attack + self.sustain {
                let k = (t - self.attack) / self.sustain.max(1e-4);
                1.0 + self.punch * (1.0 - k)
            } else {
                let k = (t - self.attack - self.sustain) / self.decay.max(1e-4);
                (1.0 - k).max(0.0).powf(1.5)
            };
            let mut x = wave * env;
            lp += a_lp * (x - lp);
            x = lp;
            if a_hp > 0.0 {
                hp_lp += a_hp * (x - hp_lp);
                x -= hp_lp;
            }
            out.push((x * self.volume * 0.8).clamp(-1.0, 1.0));
        }
        // Sem estalo no fim.
        let fade = (RATE as usize / 200).min(out.len());
        let len = out.len();
        for (j, s) in out[len - fade..].iter_mut().enumerate() {
            *s *= 1.0 - j as f32 / fade as f32;
        }
        out
    }
}

/// Um `.wav` PCM de 16 bits, mono.
pub fn wav_bytes(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&RATE.to_le_bytes());
    b.extend_from_slice(&(RATE * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        b.extend_from_slice(&v.to_le_bytes());
    }
    b
}

/// O `.sfx` ao lado de um `.wav` (`sons/moeda.wav` → `sons/moeda.sfx`).
pub fn sfx_for(wav: &str) -> String {
    let name_at = wav.rfind('/').map_or(0, |i| i + 1);
    match wav[name_at..].rfind('.') {
        Some(dot) => format!("{}.sfx", &wav[..name_at + dot]),
        None => format!("{wav}.sfx"),
    }
}

/// Mínimo e máximo de cada pedaço (para desenhar a onda).
pub fn peaks(samples: &[f32], buckets: usize) -> Vec<(f32, f32)> {
    let buckets = buckets.max(1);
    (0..buckets)
        .map(|b| {
            let a = b * samples.len() / buckets;
            let z = ((b + 1) * samples.len() / buckets)
                .max(a + 1)
                .min(samples.len());
            samples
                .get(a..z)
                .unwrap_or(&[])
                .iter()
                .fold((0.0f32, 0.0f32), |(lo, hi), s| (lo.min(*s), hi.max(*s)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todos_os_tipos_tocam() {
        for (k, _, _) in KINDS {
            for v in [0, 1, 7] {
                let s = Sfx::preset(k, v).unwrap();
                let samples = s.render();
                assert!(!samples.is_empty(), "{k}");
                let peak = samples.iter().fold(0.0f32, |m, x| m.max(x.abs()));
                assert!(peak > 0.05 && peak <= 1.0, "{k} {v}: pico {peak}");
                assert!(samples.last().unwrap().abs() < 0.01, "{k}: sem estalo");
            }
        }
        assert!(Sfx::preset("nada", 0).is_none());
        // A mesma variação dá o mesmo som; outra, outro.
        assert_eq!(Sfx::preset("moeda", 3), Sfx::preset("moeda", 3));
        assert_ne!(Sfx::preset("moeda", 3), Sfx::preset("moeda", 4));
    }

    #[test]
    fn json_e_wav() {
        let s = Sfx::preset("pulo", 2).unwrap();
        let (back, kind, v) = Sfx::from_json(&s.to_json("pulo", 2)).unwrap();
        assert_eq!((kind.as_str(), v), ("pulo", 2));
        assert!((back.freq - s.freq).abs() < 0.01 && back.wave == s.wave);
        let mut m = Map::new();
        m.insert("onda".into(), json!("seno"));
        m.insert("freq".into(), json!(99999));
        let mut t = s.clone();
        t.apply(&m).unwrap();
        assert_eq!((t.wave, t.freq), (Wave::Sine, 4000.0), "dentro do limite");
        m.insert("cor".into(), json!(1));
        assert!(t.apply(&m).unwrap_err().contains("não há o campo \"cor\""));
        let wav = wav_bytes(&[0.0, 0.5, -1.0]);
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(wav.len(), 44 + 6);
        assert_eq!(sfx_for("sons/moeda.wav"), "sons/moeda.sfx");
        assert_eq!(sfx_for("a.b/som"), "a.b/som.sfx");
    }
}
