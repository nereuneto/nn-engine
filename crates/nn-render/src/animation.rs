//! Spritesheets e animação quadro a quadro.

use glam::UVec2;

use crate::{Rect, Sprite, TextureId};

/// Uma textura dividida em quadros do mesmo tamanho, em grade.
///
/// Os quadros são numerados da esquerda para a direita e de cima para baixo,
/// começando em 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteSheet {
    texture: TextureId,
    texture_size: UVec2,
    frame_size: UVec2,
    margin: u32,
    spacing: u32,
    columns: u32,
    rows: u32,
}

impl SpriteSheet {
    /// Cria a grade. Prefira [`crate::Renderer::sprite_sheet`], que já sabe o
    /// tamanho da textura.
    ///
    /// # Panics
    /// Se o quadro tiver tamanho zero ou for maior que a textura.
    pub fn new(texture: TextureId, texture_size: UVec2, frame_size: UVec2) -> Self {
        assert!(
            frame_size.x > 0 && frame_size.y > 0,
            "quadro da spritesheet com tamanho zero"
        );
        assert!(
            frame_size.x <= texture_size.x && frame_size.y <= texture_size.y,
            "quadro maior que a textura"
        );
        let mut sheet = Self {
            texture,
            texture_size,
            frame_size,
            margin: 0,
            spacing: 0,
            columns: 0,
            rows: 0,
        };
        sheet.recount();
        sheet
    }

    /// Borda em pixels ao redor de toda a grade.
    pub fn with_margin(mut self, margin: u32) -> Self {
        self.margin = margin;
        self.recount();
        self
    }

    /// Espaço em pixels entre quadros vizinhos.
    pub fn with_spacing(mut self, spacing: u32) -> Self {
        self.spacing = spacing;
        self.recount();
        self
    }

    fn recount(&mut self) {
        let fit = |total: u32, frame: u32| {
            let usable = total.saturating_sub(2 * self.margin) + self.spacing;
            usable / (frame + self.spacing)
        };
        self.columns = fit(self.texture_size.x, self.frame_size.x);
        self.rows = fit(self.texture_size.y, self.frame_size.y);
    }

    pub fn texture(&self) -> TextureId {
        self.texture
    }

    pub fn frame_size(&self) -> UVec2 {
        self.frame_size
    }

    pub fn columns(&self) -> u32 {
        self.columns
    }

    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// Total de quadros na grade.
    pub fn frame_count(&self) -> u32 {
        self.columns * self.rows
    }

    /// Região da textura ocupada pelo quadro `index`.
    ///
    /// # Panics
    /// Se `index` estiver fora da grade.
    pub fn region(&self, index: u32) -> Rect {
        assert!(
            index < self.frame_count(),
            "quadro {index} fora da spritesheet ({} quadros)",
            self.frame_count()
        );
        let col = index % self.columns;
        let row = index / self.columns;
        let step = self.frame_size + UVec2::splat(self.spacing);
        Rect::from_pixels(
            self.margin + col * step.x,
            self.margin + row * step.y,
            self.frame_size.x,
            self.frame_size.y,
            self.texture_size,
        )
    }

    /// Sprite do quadro `index`, no tamanho original do quadro.
    pub fn sprite(&self, index: u32) -> Sprite {
        Sprite::new(self.texture).region(self.region(index))
    }
}

/// Como a animação se comporta ao chegar no último quadro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Playback {
    /// Volta ao primeiro quadro.
    #[default]
    Loop,
    /// Para no último quadro.
    Once,
    /// Vai e volta: 0 1 2 3 2 1 0 1 ...
    PingPong,
}

/// Sequência de quadros de uma [`SpriteSheet`] tocada a uma taxa fixa.
#[derive(Debug, Clone, PartialEq)]
pub struct Animation {
    frames: Vec<u32>,
    fps: f32,
    playback: Playback,
    /// Duração de cada quadro, em segundos (`None`: todos duram `1 / fps`).
    times: Option<Vec<f32>>,
}

impl Animation {
    /// Cria a animação com os quadros indicados, a `fps` quadros por segundo.
    ///
    /// ```
    /// use nn_render::Animation;
    /// let andar = Animation::new(0..4, 10.0);
    /// let piscar = Animation::new([4, 5, 4], 6.0).once();
    /// ```
    ///
    /// # Panics
    /// Se a lista de quadros estiver vazia ou `fps` não for positivo.
    pub fn new(frames: impl IntoIterator<Item = u32>, fps: f32) -> Self {
        let frames: Vec<u32> = frames.into_iter().collect();
        assert!(!frames.is_empty(), "animação sem quadros");
        assert!(fps > 0.0, "fps da animação precisa ser positivo");
        Self {
            frames,
            fps,
            playback: Playback::Loop,
            times: None,
        }
    }

    /// Dá a cada quadro a sua duração, em segundos (um valor por quadro, na
    /// ordem de `frames`). Quadros sem valor, ou com valor que não é positivo,
    /// usam `1 / fps`.
    ///
    /// ```
    /// use nn_render::Animation;
    /// // O segundo quadro fica parado meio segundo.
    /// let golpe = Animation::new([0, 1, 2], 10.0).with_times([0.1, 0.5, 0.1]);
    /// assert!((golpe.duration() - 0.7).abs() < 1e-6);
    /// ```
    pub fn with_times(mut self, times: impl IntoIterator<Item = f32>) -> Self {
        let base = 1.0 / self.fps;
        let mut list: Vec<f32> = times
            .into_iter()
            .map(|t| if t > 0.0 && t.is_finite() { t } else { base })
            .collect();
        list.resize(self.frames.len(), base);
        list.truncate(self.frames.len());
        self.times = if list.iter().all(|t| (t - base).abs() < 1e-6) {
            None
        } else {
            Some(list)
        };
        self
    }

    /// Duração do quadro `i` da lista (em segundos).
    pub fn frame_time(&self, i: usize) -> f32 {
        match &self.times {
            Some(t) => t[i],
            None => 1.0 / self.fps,
        }
    }

    /// Durações próprias de cada quadro, se houver.
    pub fn times(&self) -> Option<&[f32]> {
        self.times.as_deref()
    }

    pub fn once(mut self) -> Self {
        self.playback = Playback::Once;
        self
    }

    pub fn ping_pong(mut self) -> Self {
        self.playback = Playback::PingPong;
        self
    }

    pub fn playback(&self) -> Playback {
        self.playback
    }

    pub fn frames(&self) -> &[u32] {
        &self.frames
    }

    pub fn fps(&self) -> f32 {
        self.fps
    }

    /// Duração de uma passada completa, em segundos.
    pub fn duration(&self) -> f32 {
        match &self.times {
            None => self.cycle_len() as f32 / self.fps,
            Some(_) => (0..self.cycle_len())
                .map(|s| self.frame_time(self.cycle_index(s)))
                .sum(),
        }
    }

    /// Posição na lista de quadros do passo `s` de uma passada.
    fn cycle_index(&self, s: usize) -> usize {
        let n = self.frames.len();
        if s < n { s } else { self.cycle_len() - s }
    }

    fn cycle_len(&self) -> usize {
        match self.playback {
            Playback::PingPong if self.frames.len() > 1 => 2 * self.frames.len() - 2,
            _ => self.frames.len(),
        }
    }

    /// Quadro (índice da spritesheet) no instante `time`, em segundos.
    pub fn frame_at(&self, time: f32) -> u32 {
        if self.times.is_some() {
            return self.frames[self.timed_index(time)];
        }
        let n = self.frames.len();
        let step = (time.max(0.0) * self.fps) as usize;
        let i = match self.playback {
            Playback::Loop => step % n,
            Playback::Once => step.min(n - 1),
            Playback::PingPong => {
                let cycle = self.cycle_len();
                let t = step % cycle;
                if t < n { t } else { cycle - t }
            }
        };
        self.frames[i]
    }

    /// Com durações por quadro: anda pela passada somando as durações.
    fn timed_index(&self, time: f32) -> usize {
        let total = self.duration();
        let mut t = time.max(0.0);
        if self.playback == Playback::Once {
            if t >= total {
                return self.frames.len() - 1;
            }
        } else {
            t %= total;
        }
        for s in 0..self.cycle_len() {
            let i = self.cycle_index(s);
            let d = self.frame_time(i);
            if t < d {
                return i;
            }
            t -= d;
        }
        self.cycle_index(self.cycle_len() - 1)
    }

    /// A animação já terminou no instante `time`? Só acontece com [`Playback::Once`].
    pub fn is_finished_at(&self, time: f32) -> bool {
        self.playback == Playback::Once
            && match self.times {
                None => time * self.fps >= self.frames.len() as f32,
                Some(_) => time >= self.duration() - 1e-6,
            }
    }
}

/// Toca uma [`Animation`] e guarda o tempo decorrido.
///
/// ```
/// use nn_render::{Animation, AnimationPlayer};
/// let mut player = AnimationPlayer::new(Animation::new(0..4, 10.0));
/// player.update(0.25);
/// assert_eq!(player.frame(), 2);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationPlayer {
    animation: Animation,
    time: f32,
    /// Multiplicador de velocidade (1.0 = normal, 0.0 = pausado).
    pub speed: f32,
}

impl AnimationPlayer {
    pub fn new(animation: Animation) -> Self {
        Self {
            animation,
            time: 0.0,
            speed: 1.0,
        }
    }

    /// Troca a animação. Se for a mesma que já está tocando, continua de onde
    /// estava; assim dá para chamar a cada frame sem reiniciar.
    pub fn play(&mut self, animation: &Animation) {
        if &self.animation != animation {
            self.animation = animation.clone();
            self.time = 0.0;
        }
    }

    /// Volta ao primeiro quadro.
    pub fn restart(&mut self) {
        self.time = 0.0;
    }

    /// Avança `dt` segundos.
    pub fn update(&mut self, dt: f32) {
        self.time += dt * self.speed;
        // Evita perder precisão em animações em loop tocando por horas.
        if self.animation.playback != Playback::Once {
            self.time %= self.animation.duration();
        }
    }

    /// Quadro atual (índice da spritesheet).
    pub fn frame(&self) -> u32 {
        self.animation.frame_at(self.time)
    }

    pub fn is_finished(&self) -> bool {
        self.animation.is_finished_at(self.time)
    }

    pub fn animation(&self) -> &Animation {
        &self.animation
    }

    /// Tempo decorrido na animação atual, em segundos.
    pub fn time(&self) -> f32 {
        self.time
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet(tex: UVec2, frame: UVec2) -> SpriteSheet {
        SpriteSheet::new(TextureId(0), tex, frame)
    }

    #[test]
    fn grade_simples() {
        let s = sheet(UVec2::new(64, 32), UVec2::new(16, 16));
        assert_eq!((s.columns(), s.rows(), s.frame_count()), (4, 2, 8));
        assert_eq!(s.region(0), Rect::new(0.0, 0.0, 0.25, 0.5));
        assert_eq!(s.region(5), Rect::new(0.25, 0.5, 0.25, 0.5));
    }

    #[test]
    fn grade_com_margem_e_espaco() {
        // 2 + 16 + 1 + 16 + 2 = 37 pixels de largura para duas colunas.
        let s = sheet(UVec2::new(37, 20), UVec2::new(16, 16))
            .with_margin(2)
            .with_spacing(1);
        assert_eq!((s.columns(), s.rows()), (2, 1));
        let r = s.region(1);
        assert!((r.x - 19.0 / 37.0).abs() < 1e-6);
        assert!((r.y - 2.0 / 20.0).abs() < 1e-6);
    }

    #[test]
    #[should_panic]
    fn quadro_fora_da_grade() {
        sheet(UVec2::new(32, 16), UVec2::new(16, 16)).region(2);
    }

    #[test]
    fn loop_volta_ao_inicio() {
        let a = Animation::new([7, 8, 9], 10.0);
        let quadros: Vec<u32> = (0..7).map(|i| a.frame_at(i as f32 * 0.1 + 0.01)).collect();
        assert_eq!(quadros, [7, 8, 9, 7, 8, 9, 7]);
        assert!(!a.is_finished_at(100.0));
    }

    #[test]
    fn once_para_no_ultimo() {
        let a = Animation::new(0..3, 10.0).once();
        assert_eq!(a.frame_at(0.05), 0);
        assert_eq!(a.frame_at(5.0), 2);
        assert!(!a.is_finished_at(0.25));
        assert!(a.is_finished_at(0.3));
    }

    #[test]
    fn ping_pong_vai_e_volta() {
        let a = Animation::new(0..4, 1.0).ping_pong();
        let quadros: Vec<u32> = (0..9).map(|i| a.frame_at(i as f32 + 0.5)).collect();
        assert_eq!(quadros, [0, 1, 2, 3, 2, 1, 0, 1, 2]);
        assert!((a.duration() - 6.0).abs() < 1e-6);
    }

    #[test]
    fn duracao_por_quadro() {
        let a = Animation::new([7, 8, 9], 10.0).with_times([0.1, 0.5, 0.1]);
        let at = |t: f32| a.frame_at(t);
        assert_eq!([at(0.05), at(0.15), at(0.55), at(0.65), at(0.75)], [7, 8, 8, 9, 7]);
        let once = Animation::new([1, 2], 10.0).once().with_times([0.2, 0.3]);
        assert_eq!(once.frame_at(0.25), 2);
        assert!(!once.is_finished_at(0.4));
        assert!(once.is_finished_at(0.5));
        let pp = Animation::new([0, 1, 2], 10.0).ping_pong().with_times([1.0, 0.1, 0.1]);
        // 0 (1 s), 1, 2, 1 e volta ao 0.
        assert!((pp.duration() - 1.3).abs() < 1e-5);
        assert_eq!([pp.frame_at(0.5), pp.frame_at(1.05), pp.frame_at(1.15), pp.frame_at(1.25)], [0, 1, 2, 1]);
        // Tudo igual a 1/fps: sem lista própria.
        assert!(Animation::new([0, 1], 10.0).with_times([0.1, 0.1]).times().is_none());
    }

    #[test]
    fn ping_pong_de_um_quadro() {
        let a = Animation::new([5], 4.0).ping_pong();
        assert_eq!(a.frame_at(3.3), 5);
    }

    #[test]
    fn player_nao_reinicia_a_mesma_animacao() {
        let andar = Animation::new(0..4, 10.0);
        let parado = Animation::new([4, 5], 2.0);
        let mut p = AnimationPlayer::new(andar.clone());
        p.update(0.2);
        p.play(&andar);
        assert_eq!(p.frame(), 2);
        p.play(&parado);
        assert_eq!(p.frame(), 4);
        assert_eq!(p.time(), 0.0);
    }

    #[test]
    fn player_velocidade_e_fim() {
        let mut p = AnimationPlayer::new(Animation::new(0..4, 10.0).once());
        p.speed = 2.0;
        p.update(0.1);
        assert_eq!(p.frame(), 2);
        p.update(1.0);
        assert!(p.is_finished());
        assert_eq!(p.frame(), 3);
    }
}
