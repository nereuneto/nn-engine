//! Sintetizador simples para gerar efeitos sonoros por código.
//!
//! ```
//! use nn_audio::synth::{Tone, Wave};
//! // "Pling" de moeda: onda quadrada subindo de tom.
//! let amostras = Tone::new(Wave::Square, 988.0)
//!     .slide_to(1319.0)
//!     .duration(0.12)
//!     .volume(0.3)
//!     .render();
//! assert_eq!(amostras.len(), (0.12 * 44_100.0) as usize);
//! ```

use std::f32::consts::TAU;

/// Taxa de amostragem usada pelo sintetizador.
pub const SAMPLE_RATE: u32 = 44_100;

/// Forma de onda.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wave {
    /// Suave, como um apito.
    Sine,
    /// Áspera, clássica de videogame antigo.
    Square,
    /// Entre a senoidal e a quadrada.
    Triangle,
    /// Brilhante e "zumbida".
    Saw,
    /// Ruído branco: explosões, passos, chiados.
    Noise,
}

/// Um som simples: uma onda com tom (que pode deslizar), envelope e volume.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tone {
    pub wave: Wave,
    /// Frequência inicial em Hz.
    pub frequency: f32,
    /// Frequência final em Hz (igual à inicial se não houver deslize).
    pub end_frequency: f32,
    /// Duração em segundos.
    pub duration: f32,
    /// Volume de 0.0 a 1.0.
    pub volume: f32,
    /// Tempo de subida do volume, em segundos.
    pub attack: f32,
    /// Tempo de descida do volume no fim, em segundos.
    pub release: f32,
}

impl Tone {
    pub fn new(wave: Wave, frequency: f32) -> Self {
        Self {
            wave,
            frequency,
            end_frequency: frequency,
            duration: 0.2,
            volume: 0.5,
            attack: 0.005,
            release: 0.05,
        }
    }

    pub fn slide_to(mut self, frequency: f32) -> Self {
        self.end_frequency = frequency;
        self
    }

    pub fn duration(mut self, seconds: f32) -> Self {
        self.duration = seconds;
        self
    }

    pub fn volume(mut self, volume: f32) -> Self {
        self.volume = volume;
        self
    }

    pub fn envelope(mut self, attack: f32, release: f32) -> Self {
        self.attack = attack;
        self.release = release;
        self
    }

    /// Gera as amostras mono a [`SAMPLE_RATE`].
    pub fn render(&self) -> Vec<f32> {
        let count = (self.duration.max(0.0) * SAMPLE_RATE as f32) as usize;
        let dt = 1.0 / SAMPLE_RATE as f32;
        let mut phase = 0.0f32;
        let mut noise = 0x1234_5678u32;
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            let t = i as f32 * dt;
            let progress = if count > 1 {
                i as f32 / (count - 1) as f32
            } else {
                0.0
            };
            let freq = self.frequency + (self.end_frequency - self.frequency) * progress;
            phase = (phase + freq * dt).fract();

            let sample = match self.wave {
                Wave::Sine => (phase * TAU).sin(),
                Wave::Square => {
                    if phase < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                Wave::Triangle => 1.0 - 4.0 * (phase - 0.5).abs(),
                Wave::Saw => 2.0 * phase - 1.0,
                Wave::Noise => {
                    // xorshift: barato e determinístico.
                    noise ^= noise << 13;
                    noise ^= noise >> 17;
                    noise ^= noise << 5;
                    noise as f32 / u32::MAX as f32 * 2.0 - 1.0
                }
            };

            let attack = if self.attack > 0.0 {
                (t / self.attack).min(1.0)
            } else {
                1.0
            };
            let remaining = self.duration - t;
            let release = if self.release > 0.0 {
                (remaining / self.release).clamp(0.0, 1.0)
            } else {
                1.0
            };
            out.push(sample * self.volume * attack * release);
        }
        out
    }
}

/// Junta vários trechos em sequência (por exemplo, as notas de uma melodia).
pub fn sequence<I: IntoIterator<Item = Vec<f32>>>(parts: I) -> Vec<f32> {
    parts.into_iter().flatten().collect()
}

/// Silêncio com a duração indicada, em segundos.
pub fn silence(seconds: f32) -> Vec<f32> {
    vec![0.0; (seconds.max(0.0) * SAMPLE_RATE as f32) as usize]
}

/// Frequência em Hz de uma nota MIDI (69 = Lá 440 Hz, 60 = Dó central).
pub fn midi_to_hz(note: u8) -> f32 {
    440.0 * 2f32.powf((f32::from(note) - 69.0) / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duracao_e_limites() {
        for wave in [
            Wave::Sine,
            Wave::Square,
            Wave::Triangle,
            Wave::Saw,
            Wave::Noise,
        ] {
            let s = Tone::new(wave, 440.0).duration(0.1).volume(0.8).render();
            assert_eq!(s.len(), 4410);
            assert!(
                s.iter().all(|v| v.abs() <= 0.8 + 1e-6),
                "{wave:?} passou do volume"
            );
            assert!(s.iter().any(|v| v.abs() > 0.3), "{wave:?} está mudo");
        }
    }

    #[test]
    fn envelope_comeca_e_termina_em_silencio() {
        let s = Tone::new(Wave::Square, 220.0).duration(0.2).render();
        assert!(s[0].abs() < 0.01);
        assert!(s.last().unwrap().abs() < 0.01);
    }

    #[test]
    fn senoide_tem_a_frequencia_certa() {
        let s = Tone::new(Wave::Sine, 100.0)
            .duration(1.0)
            .envelope(0.0, 0.0)
            .render();
        let cruzamentos = s.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        assert!((99..=101).contains(&cruzamentos), "{cruzamentos}");
    }

    #[test]
    fn notas_midi() {
        assert!((midi_to_hz(69) - 440.0).abs() < 1e-3);
        assert!((midi_to_hz(81) - 880.0).abs() < 1e-2);
        assert!((midi_to_hz(60) - 261.63).abs() < 0.01);
    }

    #[test]
    fn sequencia_e_silencio() {
        let s = sequence([
            silence(0.5),
            Tone::new(Wave::Sine, 440.0).duration(0.5).render(),
        ]);
        assert_eq!(s.len(), SAMPLE_RATE as usize);
    }
}
