//! Áudio da nn-engine.
//!
//! [`Audio`] carrega sons (WAV, OGG Vorbis, MP3) e toca efeitos e músicas.
//! Se o computador não tiver dispositivo de som, a engine continua rodando
//! normalmente em modo mudo.
//!
//! O módulo [`synth`] gera sons simples por código, úteis como efeitos
//! provisórios enquanto o jogo ainda não tem arquivos de áudio.

pub mod synth;

use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings};
use kira::track::{TrackBuilder, TrackHandle};
use kira::{
    AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Frame, Panning, PlaybackRate,
    Tween,
};

/// Identificador de um som carregado em [`Audio`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundId(u32);

/// Erros ao carregar sons.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("falha ao ler {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("formato de áudio inválido ou não suportado: {0}")]
    Decode(#[from] kira::sound::FromFileError),
}

/// Canal de mixagem: cada um tem seu próprio volume, para o jogador
/// ajustar música e efeitos separadamente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Channel {
    /// Efeitos sonoros (padrão).
    #[default]
    Effects,
    /// Música de fundo.
    Music,
}

impl Channel {
    pub const ALL: [Channel; 2] = [Channel::Effects, Channel::Music];

    fn index(self) -> usize {
        self as usize
    }
}

/// Opções de reprodução de um som.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayOptions {
    /// Volume linear: 1.0 é o original, 0.5 é metade, 0.0 é mudo.
    pub volume: f32,
    /// Velocidade (e tom): 1.0 normal, 2.0 uma oitava acima.
    pub speed: f32,
    /// Posição estéreo: -1.0 esquerda, 0.0 centro, 1.0 direita.
    pub pan: f32,
    /// Repete até ser parado. Use para música e sons contínuos.
    pub looping: bool,
    /// Canal de mixagem (efeitos ou música).
    pub channel: Channel,
}

impl Default for PlayOptions {
    fn default() -> Self {
        Self {
            volume: 1.0,
            speed: 1.0,
            pan: 0.0,
            looping: false,
            channel: Channel::Effects,
        }
    }
}

impl PlayOptions {
    pub fn volume(mut self, volume: f32) -> Self {
        self.volume = volume;
        self
    }

    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    pub fn pan(mut self, pan: f32) -> Self {
        self.pan = pan;
        self
    }

    pub fn looped(mut self) -> Self {
        self.looping = true;
        self
    }

    pub fn channel(mut self, channel: Channel) -> Self {
        self.channel = channel;
        self
    }
}

/// Converte volume linear em decibéis.
fn to_decibels(volume: f32) -> Decibels {
    if volume <= 0.001 {
        Decibels::SILENCE
    } else {
        Decibels(20.0 * volume.log10())
    }
}

/// Um som tocando. Pode ser descartado sem parar o som.
#[derive(Debug, Default)]
pub struct Sound {
    handle: Option<StaticSoundHandle>,
}

impl Sound {
    /// Som que não toca nada (modo mudo ou falha ao tocar).
    pub fn silent() -> Self {
        Self::default()
    }

    /// Ainda está tocando (ou pausado, esperando para continuar)?
    pub fn is_playing(&self) -> bool {
        self.handle
            .as_ref()
            .is_some_and(|h| !matches!(h.state(), PlaybackState::Stopped))
    }

    /// Para o som com um fade bem curto (evita estalos).
    pub fn stop(&mut self) {
        if let Some(h) = &mut self.handle {
            h.stop(Tween::default());
        }
    }

    /// Para o som diminuindo o volume ao longo de `duration`.
    pub fn fade_out(&mut self, duration: Duration) {
        if let Some(h) = &mut self.handle {
            h.stop(Tween {
                duration,
                ..Default::default()
            });
        }
    }

    pub fn pause(&mut self) {
        if let Some(h) = &mut self.handle {
            h.pause(Tween::default());
        }
    }

    pub fn resume(&mut self) {
        if let Some(h) = &mut self.handle {
            h.resume(Tween::default());
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        if let Some(h) = &mut self.handle {
            h.set_volume(to_decibels(volume), Tween::default());
        }
    }

    pub fn set_speed(&mut self, speed: f32) {
        if let Some(h) = &mut self.handle {
            h.set_playback_rate(PlaybackRate(f64::from(speed.max(0.0))), Tween::default());
        }
    }

    pub fn set_pan(&mut self, pan: f32) {
        if let Some(h) = &mut self.handle {
            h.set_panning(Panning(pan.clamp(-1.0, 1.0)), Tween::default());
        }
    }
}

/// Sistema de áudio: guarda os sons carregados e toca.
pub struct Audio {
    manager: Option<AudioManager<DefaultBackend>>,
    /// Uma trilha do mixer por canal (vazio no modo mudo).
    tracks: Vec<TrackHandle>,
    sounds: Vec<StaticSoundData>,
    master_volume: f32,
    channel_volumes: [f32; 2],
}

impl std::fmt::Debug for Audio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Audio")
            .field("enabled", &self.is_enabled())
            .field("sounds", &self.sounds.len())
            .field("master_volume", &self.master_volume)
            .field("channel_volumes", &self.channel_volumes)
            .finish()
    }
}

impl Default for Audio {
    fn default() -> Self {
        Self::new()
    }
}

impl Audio {
    /// Abre o dispositivo de som padrão. Se não houver, entra em modo mudo.
    pub fn new() -> Self {
        match AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()) {
            Ok(mut manager) => {
                let tracks = Channel::ALL
                    .iter()
                    .map(|_| manager.add_sub_track(TrackBuilder::new()))
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap_or_else(|e| {
                        log::warn!("canais de áudio indisponíveis: {e}");
                        Vec::new()
                    });
                Self {
                    manager: Some(manager),
                    tracks,
                    ..Self::disabled()
                }
            }
            Err(e) => {
                log::warn!("sem dispositivo de áudio, seguindo em modo mudo: {e}");
                Self::disabled()
            }
        }
    }

    /// Áudio em modo mudo: tudo funciona, mas nada é tocado. Útil em testes.
    pub fn disabled() -> Self {
        Self {
            manager: None,
            tracks: Vec::new(),
            sounds: Vec::new(),
            master_volume: 1.0,
            channel_volumes: [1.0; 2],
        }
    }

    /// Há um dispositivo de som ativo?
    pub fn is_enabled(&self) -> bool {
        self.manager.is_some()
    }

    /// Carrega um som de um arquivo (WAV, OGG Vorbis ou MP3).
    ///
    /// O arquivo inteiro é decodificado na memória. Para efeitos e músicas
    /// curtas isso é o ideal.
    pub fn load(&mut self, path: impl AsRef<Path>) -> Result<SoundId, AudioError> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|source| AudioError::Io {
            path: path.to_owned(),
            source,
        })?;
        self.load_bytes(bytes)
    }

    /// Carrega um som a partir dos bytes de um arquivo (WAV, OGG Vorbis ou MP3).
    /// Funciona bem com `include_bytes!`.
    pub fn load_bytes(&mut self, bytes: impl Into<Vec<u8>>) -> Result<SoundId, AudioError> {
        let data = StaticSoundData::from_cursor(Cursor::new(bytes.into()))?;
        Ok(self.add(data))
    }

    /// Cria um som a partir de amostras mono (de -1.0 a 1.0), por exemplo
    /// geradas com [`synth`].
    pub fn add_samples(&mut self, sample_rate: u32, samples: &[f32]) -> SoundId {
        let frames: Arc<[Frame]> = samples.iter().map(|&s| Frame::from_mono(s)).collect();
        self.add(StaticSoundData {
            sample_rate,
            frames,
            settings: StaticSoundSettings::default(),
            slice: None,
        })
    }

    /// Troca o conteúdo de um som já carregado, mantendo o mesmo [`SoundId`].
    /// Sons que já estão tocando continuam com a versão antiga.
    pub fn replace_bytes(
        &mut self,
        id: SoundId,
        bytes: impl Into<Vec<u8>>,
    ) -> Result<(), AudioError> {
        let data = StaticSoundData::from_cursor(Cursor::new(bytes.into()))?;
        self.sounds[id.0 as usize] = data;
        Ok(())
    }

    fn add(&mut self, data: StaticSoundData) -> SoundId {
        self.sounds.push(data);
        SoundId(self.sounds.len() as u32 - 1)
    }

    /// Duração do som.
    pub fn duration(&self, id: SoundId) -> Duration {
        self.sounds[id.0 as usize].duration()
    }

    /// Toca o som uma vez, com as opções padrão.
    pub fn play(&mut self, id: SoundId) -> Sound {
        self.play_with(id, PlayOptions::default())
    }

    /// Toca o som com volume, velocidade, posição estéreo ou repetição.
    pub fn play_with(&mut self, id: SoundId, options: PlayOptions) -> Sound {
        let mut data = self.sounds[id.0 as usize]
            .volume(to_decibels(options.volume))
            .playback_rate(PlaybackRate(f64::from(options.speed.max(0.0))))
            .panning(Panning(options.pan.clamp(-1.0, 1.0)));
        if options.looping {
            data = data.loop_region(..);
        }
        let Some(manager) = &mut self.manager else {
            return Sound::silent();
        };
        let played = match self.tracks.get_mut(options.channel.index()) {
            Some(track) => track.play(data),
            None => manager.play(data),
        };
        match played {
            Ok(handle) => Sound {
                handle: Some(handle),
            },
            Err(e) => {
                log::warn!("não foi possível tocar o som: {e}");
                Sound::silent()
            }
        }
    }

    /// Volume geral, aplicado a todos os sons (linear, 1.0 = normal).
    pub fn master_volume(&self) -> f32 {
        self.master_volume
    }

    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.max(0.0);
        if let Some(manager) = &mut self.manager {
            manager
                .main_track()
                .set_volume(to_decibels(self.master_volume), Tween::default());
        }
    }

    /// Volume de um canal (linear, 1.0 = normal), multiplicado pelo volume geral.
    pub fn channel_volume(&self, channel: Channel) -> f32 {
        self.channel_volumes[channel.index()]
    }

    /// Muda o volume de um canal, inclusive dos sons que já estão tocando nele.
    pub fn set_channel_volume(&mut self, channel: Channel, volume: f32) {
        let volume = volume.max(0.0);
        self.channel_volumes[channel.index()] = volume;
        if let Some(track) = self.tracks.get_mut(channel.index()) {
            track.set_volume(to_decibels(volume), Tween::default());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WAV PCM 16 bits mono mínimo, montado à mão.
    fn wav(sample_rate: u32, samples: &[i16]) -> Vec<u8> {
        let data_len = (samples.len() * 2) as u32;
        let mut v = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + data_len).to_le_bytes());
        v.extend_from_slice(b"WAVEfmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes()); // PCM
        v.extend_from_slice(&1u16.to_le_bytes()); // mono
        v.extend_from_slice(&sample_rate.to_le_bytes());
        v.extend_from_slice(&(sample_rate * 2).to_le_bytes());
        v.extend_from_slice(&2u16.to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_len.to_le_bytes());
        for s in samples {
            v.extend_from_slice(&s.to_le_bytes());
        }
        v
    }

    #[test]
    fn carrega_wav_da_memoria() {
        let mut audio = Audio::disabled();
        let id = audio.load_bytes(wav(8000, &[0; 4000])).unwrap();
        assert_eq!(audio.duration(id), Duration::from_millis(500));
    }

    #[test]
    fn rejeita_arquivo_invalido() {
        let mut audio = Audio::disabled();
        assert!(audio.load_bytes(b"nao sou audio".to_vec()).is_err());
        assert!(matches!(
            audio.load("nao/existe.ogg"),
            Err(AudioError::Io { .. })
        ));
    }

    #[test]
    fn amostras_geradas_viram_som() {
        let mut audio = Audio::disabled();
        let id = audio.add_samples(1000, &[0.0; 250]);
        assert_eq!(audio.duration(id), Duration::from_millis(250));
    }

    #[test]
    fn modo_mudo_aceita_tudo() {
        let mut audio = Audio::disabled();
        let id = audio.add_samples(1000, &[0.5; 100]);
        let mut som = audio.play_with(id, PlayOptions::default().looped().volume(0.3));
        assert!(!som.is_playing());
        som.set_volume(0.1);
        som.pause();
        som.resume();
        som.stop();
        audio.set_master_volume(0.5);
        assert_eq!(audio.master_volume(), 0.5);
    }

    #[test]
    fn volume_por_canal() {
        let mut audio = Audio::disabled();
        assert_eq!(audio.channel_volume(Channel::Music), 1.0);
        audio.set_channel_volume(Channel::Music, 0.25);
        audio.set_channel_volume(Channel::Effects, -1.0);
        assert_eq!(audio.channel_volume(Channel::Music), 0.25);
        assert_eq!(audio.channel_volume(Channel::Effects), 0.0);
        let id = audio.add_samples(1000, &[0.5; 100]);
        let som = audio.play_with(id, PlayOptions::default().channel(Channel::Music));
        assert!(!som.is_playing());
    }

    #[test]
    fn conversao_de_volume() {
        assert_eq!(to_decibels(1.0), Decibels(0.0));
        assert!((to_decibels(0.5).0 + 6.02).abs() < 0.01);
        assert_eq!(to_decibels(0.0), Decibels::SILENCE);
        assert_eq!(to_decibels(-1.0), Decibels::SILENCE);
    }
}
