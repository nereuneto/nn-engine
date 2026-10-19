//! Áudio da nn-engine.
//!
//! [`Audio`] carrega sons (WAV, OGG Vorbis, MP3) e toca efeitos e músicas.
//! Se o computador não tiver dispositivo de som, a engine continua rodando
//! normalmente em modo mudo.
//!
//! O módulo [`synth`] gera sons simples por código, úteis como efeitos
//! provisórios enquanto o jogo ainda não tem arquivos de áudio.
//!
//! **Som 3D:** um som com posição ([`PlayOptions::at`]) fica mais baixo com a
//! distância até quem ouve ([`Audio::set_listener`], normalmente a câmera) e
//! vai para o lado esquerdo ou direito conforme onde ele está. O volume e o
//! lado são refeitos a cada [`Audio::set_listener`], então um som pode andar
//! junto com um objeto ([`Sound::set_position`]).

pub mod synth;

use std::io::Cursor;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use glam::Vec3;

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
    /// Som 3D: onde ele está no mundo (`None`: som comum, igual em
    /// qualquer lugar).
    pub position: Option<Vec3>,
    /// Som 3D: a partir desta distância (em metros) ele não se ouve mais.
    pub distance: f32,
}

impl Default for PlayOptions {
    fn default() -> Self {
        Self {
            volume: 1.0,
            speed: 1.0,
            pan: 0.0,
            looping: false,
            channel: Channel::Effects,
            position: None,
            distance: 30.0,
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

    /// Som 3D, tocando neste ponto do mundo.
    pub fn at(mut self, position: Vec3) -> Self {
        self.position = Some(position);
        self
    }

    /// Som 3D: a distância máxima em que ele é ouvido.
    pub fn distance(mut self, distance: f32) -> Self {
        self.distance = distance;
        self
    }
}

/// Quem ouve os sons 3D: onde está e para onde é a direita dele.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Listener {
    pub position: Vec3,
    pub right: Vec3,
}

impl Default for Listener {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            right: Vec3::X,
        }
    }
}

/// Volume (de 0 a 1) e lado (-1 esquerda a 1 direita) de um som 3D em
/// `source`, ouvido por `listener`, com a distância máxima `max`. Até um
/// quarto da distância máxima (no mínimo 1 m), o volume é cheio; depois cai
/// suave até zero na distância máxima. O lado nunca é todo de um ouvido só.
pub fn spatialize(listener: &Listener, source: Vec3, max: f32) -> (f32, f32) {
    let max = max.max(0.01);
    let near = (max * 0.25).max(1.0).min(max * 0.9);
    let to = source - listener.position;
    let d = to.length();
    let t = ((max - d) / (max - near)).clamp(0.0, 1.0);
    let gain = t * t;
    let pan = if d < 1e-4 {
        0.0
    } else {
        // Perto de quem ouve, o lado conta menos (o som "envolve").
        let side = to.normalize().dot(listener.right.normalize_or(Vec3::X));
        side * 0.8 * (d / near).min(1.0)
    };
    (gain, pan)
}

/// Um som tocando, com o que é preciso para refazer o volume e o lado de um
/// som 3D.
#[derive(Debug)]
struct Voice {
    handle: Option<StaticSoundHandle>,
    /// Volume pedido (sem a distância).
    volume: f32,
    pan: f32,
    position: Option<Vec3>,
    distance: f32,
    /// O volume da distância da última conta (1 num som comum).
    gain: f32,
}

impl Voice {
    /// Refaz o volume e o lado (devolve os dois).
    fn apply(&mut self, listener: &Listener, tween: Tween) -> (f32, f32) {
        let (volume, pan) = match self.position {
            Some(p) => {
                let (g, pan) = spatialize(listener, p, self.distance);
                self.gain = g;
                (self.volume * g, pan)
            }
            None => (self.volume, self.pan),
        };
        if let Some(h) = &mut self.handle {
            h.set_volume(to_decibels(volume), tween);
            h.set_panning(Panning(pan.clamp(-1.0, 1.0)), tween);
        }
        (volume, pan)
    }

    fn playing(&self) -> bool {
        self.handle
            .as_ref()
            .is_some_and(|h| !matches!(h.state(), PlaybackState::Stopped))
    }
}

/// Transição curta para as mudanças de volume e lado do som 3D (sem
/// estalos).
const SPATIAL_TWEEN: Tween = Tween {
    start_time: kira::StartTime::Immediate,
    duration: Duration::from_millis(40),
    easing: kira::Easing::Linear,
};

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
    voice: Option<Arc<Mutex<Voice>>>,
}

impl Sound {
    /// Som que não toca nada (modo mudo ou falha ao tocar).
    pub fn silent() -> Self {
        Self::default()
    }

    fn with<R>(&self, f: impl FnOnce(&mut Voice) -> R) -> Option<R> {
        let v = self.voice.as_ref()?;
        let mut v = v.lock().unwrap_or_else(|e| e.into_inner());
        Some(f(&mut v))
    }

    fn with_handle(&self, f: impl FnOnce(&mut StaticSoundHandle)) {
        self.with(|v| {
            if let Some(h) = &mut v.handle {
                f(h);
            }
        });
    }

    /// Ainda está tocando (ou pausado, esperando para continuar)?
    pub fn is_playing(&self) -> bool {
        self.with(|v| v.playing()).unwrap_or(false)
    }

    /// Para o som com um fade bem curto (evita estalos).
    pub fn stop(&mut self) {
        self.with_handle(|h| h.stop(Tween::default()));
    }

    /// Para o som diminuindo o volume ao longo de `duration`.
    pub fn fade_out(&mut self, duration: Duration) {
        self.with_handle(|h| {
            h.stop(Tween {
                duration,
                ..Default::default()
            })
        });
    }

    pub fn pause(&mut self) {
        self.with_handle(|h| h.pause(Tween::default()));
    }

    pub fn resume(&mut self) {
        self.with_handle(|h| h.resume(Tween::default()));
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.with(|v| {
            v.volume = volume.max(0.0);
            let vol = v.volume * v.gain;
            if let Some(h) = &mut v.handle {
                h.set_volume(to_decibels(vol), Tween::default());
            }
        });
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.with_handle(|h| {
            h.set_playback_rate(PlaybackRate(f64::from(speed.max(0.0))), Tween::default())
        });
    }

    /// Lado do som (num som 3D, o lado vem da posição).
    pub fn set_pan(&mut self, pan: f32) {
        self.with(|v| {
            v.pan = pan.clamp(-1.0, 1.0);
            if v.position.is_none()
                && let Some(h) = &mut v.handle
            {
                h.set_panning(Panning(v.pan), Tween::default());
            }
        });
    }

    /// Som 3D: muda onde ele está (vale no próximo [`Audio::set_listener`]).
    /// Num som comum, passa a ser 3D.
    pub fn set_position(&mut self, position: Vec3) {
        self.with(|v| v.position = Some(position));
    }

    /// Som 3D: onde ele está.
    pub fn position(&self) -> Option<Vec3> {
        self.with(|v| v.position).flatten()
    }

    /// O volume da distância (de 0 a 1) da última conta; 1 num som comum.
    pub fn distance_gain(&self) -> f32 {
        self.with(|v| v.gain).unwrap_or(1.0)
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
    listener: Listener,
    /// Os sons 3D, para refazer o volume e o lado quando quem ouve anda.
    spatial: Vec<Arc<Mutex<Voice>>>,
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
            listener: Listener::default(),
            spatial: Vec::new(),
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

    /// Forma de onda resumida para desenhar: o som dividido em `buckets`
    /// pedaços, com o menor e o maior valor (de -1 a 1, mono) de cada um.
    pub fn waveform(&self, id: SoundId, buckets: usize) -> Vec<(f32, f32)> {
        let frames = &self.sounds[id.0 as usize].frames;
        if frames.is_empty() || buckets == 0 {
            return Vec::new();
        }
        let buckets = buckets.min(frames.len());
        (0..buckets)
            .map(|b| {
                let start = b * frames.len() / buckets;
                let end = ((b + 1) * frames.len() / buckets).max(start + 1);
                frames[start..end]
                    .iter()
                    .map(|f| (f.left + f.right) * 0.5)
                    .fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(v), hi.max(v)))
            })
            .collect()
    }

    /// Toca o som uma vez, com as opções padrão.
    pub fn play(&mut self, id: SoundId) -> Sound {
        self.play_with(id, PlayOptions::default())
    }

    /// Toca o som com volume, velocidade, posição estéreo ou repetição. Com
    /// [`PlayOptions::at`], é um som 3D.
    pub fn play_with(&mut self, id: SoundId, options: PlayOptions) -> Sound {
        let mut voice = Voice {
            handle: None,
            volume: options.volume.max(0.0),
            pan: options.pan.clamp(-1.0, 1.0),
            position: options.position,
            distance: options.distance,
            gain: 1.0,
        };
        // O volume e o lado já certos desde o começo.
        let (volume, pan) = voice.apply(&self.listener, Tween::default());
        let mut data = self.sounds[id.0 as usize]
            .volume(to_decibels(volume))
            .playback_rate(PlaybackRate(f64::from(options.speed.max(0.0))))
            .panning(Panning(pan));
        if options.looping {
            data = data.loop_region(..);
        }
        if let Some(manager) = &mut self.manager {
            let played = match self.tracks.get_mut(options.channel.index()) {
                Some(track) => track.play(data),
                None => manager.play(data),
            };
            match played {
                Ok(handle) => voice.handle = Some(handle),
                Err(e) => log::warn!("não foi possível tocar o som: {e}"),
            }
        }
        let spatial = voice.position.is_some();
        let voice = Arc::new(Mutex::new(voice));
        if spatial {
            self.spatial.push(voice.clone());
        }
        Sound { voice: Some(voice) }
    }

    /// Quem ouve os sons 3D (normalmente a câmera): refaz o volume e o lado
    /// de todos os sons 3D tocando. Chame a cada frame.
    pub fn set_listener(&mut self, listener: Listener) {
        self.listener = listener;
        self.spatial.retain(|v| {
            let mut voice = v.lock().unwrap_or_else(|e| e.into_inner());
            // Fica enquanto toca ou enquanto o jogo guarda o som.
            let keep = voice.playing() || Arc::strong_count(v) > 1;
            if keep {
                let _ = voice.apply(&listener, SPATIAL_TWEEN);
            }
            keep
        });
    }

    pub fn listener(&self) -> Listener {
        self.listener
    }

    /// Quantos sons 3D estão sendo acompanhados.
    pub fn spatial_count(&self) -> usize {
        self.spatial.len()
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
    fn forma_de_onda_resumida() {
        let mut audio = Audio::disabled();
        // Meio segundo de silêncio e meio de onda cheia.
        let mut samples = vec![0.0; 100];
        samples.extend((0..100).map(|i| if i % 2 == 0 { 1.0 } else { -1.0 }));
        let id = audio.add_samples(200, &samples);
        let w = audio.waveform(id, 4);
        assert_eq!(w.len(), 4);
        assert_eq!(w[0], (0.0, 0.0));
        assert_eq!(w[3], (-1.0, 1.0));
        // Mais pedaços que amostras: um por amostra.
        assert_eq!(audio.waveform(id, 10_000).len(), 200);
        assert!(audio.waveform(id, 0).is_empty());
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
    fn som_3d_cai_com_a_distancia_e_vai_para_o_lado() {
        let l = Listener::default();
        // Perto: cheio e no meio.
        let (g, p) = spatialize(&l, Vec3::new(0.0, 0.0, -0.5), 30.0);
        assert_eq!(g, 1.0);
        assert!(p.abs() < 0.5);
        // À direita (+X), a 10 m: mais baixo e à direita.
        let (g, p) = spatialize(&l, Vec3::new(10.0, 0.0, 0.0), 30.0);
        assert!(g > 0.3 && g < 0.9, "{g}");
        assert!((p - 0.8).abs() < 1e-5, "{p}");
        // À esquerda.
        let (_, p) = spatialize(&l, Vec3::new(-10.0, 0.0, 0.0), 30.0);
        assert!(p < -0.7);
        // Longe demais: mudo.
        assert_eq!(spatialize(&l, Vec3::new(0.0, 0.0, 40.0), 30.0).0, 0.0);
        // Mais longe, mais baixo.
        let a = spatialize(&l, Vec3::new(0.0, 0.0, 5.0), 30.0).0;
        let b = spatialize(&l, Vec3::new(0.0, 0.0, 15.0), 30.0).0;
        assert!(a > b);
    }

    #[test]
    fn som_3d_acompanha_quem_ouve() {
        let mut audio = Audio::disabled();
        let id = audio.add_samples(1000, &[0.5; 100]);
        let mut som = audio.play_with(id, PlayOptions::default().at(Vec3::new(0.0, 0.0, 20.0)));
        assert!(som.distance_gain() < 0.3);
        assert_eq!(audio.spatial_count(), 1);
        // Quem ouve chega perto: cheio.
        audio.set_listener(Listener {
            position: Vec3::new(0.0, 0.0, 19.5),
            right: Vec3::X,
        });
        assert_eq!(som.distance_gain(), 1.0);
        // O som anda para longe.
        som.set_position(Vec3::new(0.0, 0.0, 100.0));
        audio.set_listener(audio.listener());
        assert_eq!(som.distance_gain(), 0.0);
        // Descartado (e sem tocar): sai da lista.
        drop(som);
        audio.set_listener(audio.listener());
        assert_eq!(audio.spatial_count(), 0);
        // Som comum não entra.
        let s = audio.play(id);
        assert_eq!(s.distance_gain(), 1.0);
        assert_eq!(audio.spatial_count(), 0);
    }

    #[test]
    fn conversao_de_volume() {
        assert_eq!(to_decibels(1.0), Decibels(0.0));
        assert!((to_decibels(0.5).0 + 6.02).abs() < 0.01);
        assert_eq!(to_decibels(0.0), Decibels::SILENCE);
        assert_eq!(to_decibels(-1.0), Decibels::SILENCE);
    }
}
