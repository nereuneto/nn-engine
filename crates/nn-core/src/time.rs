//! Controle de tempo: relógio do frame e timestep fixo para a simulação.

use std::time::Duration;

/// Informações de tempo do frame atual, atualizadas pela engine a cada frame.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Time {
    delta: Duration,
    elapsed: Duration,
    frame: u64,
}

impl Time {
    /// Registra o tempo real que passou desde o frame anterior.
    pub fn tick(&mut self, delta: Duration) {
        self.delta = delta;
        self.elapsed += delta;
        self.frame += 1;
    }

    /// Tempo real do último frame.
    pub fn delta(&self) -> Duration {
        self.delta
    }

    /// Tempo real do último frame, em segundos.
    pub fn delta_secs(&self) -> f32 {
        self.delta.as_secs_f32()
    }

    /// Tempo total desde o início da execução.
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Tempo total desde o início da execução, em segundos.
    pub fn elapsed_secs(&self) -> f32 {
        self.elapsed.as_secs_f32()
    }

    /// Quantos frames já foram processados.
    pub fn frame(&self) -> u64 {
        self.frame
    }
}

/// Acumulador de timestep fixo.
///
/// A simulação (física, lógica do jogo) avança sempre em passos do mesmo
/// tamanho, independentemente da taxa de quadros. Isso deixa o comportamento
/// determinístico e igual em máquinas rápidas e lentas. A renderização usa
/// [`FixedTimestep::alpha`] para interpolar entre o estado anterior e o atual.
///
/// ```
/// use std::time::Duration;
/// use nn_core::FixedTimestep;
///
/// let mut ts = FixedTimestep::from_hz(60);
/// // Um frame lento de 34 ms rende dois passos de simulação de ~16,7 ms.
/// assert_eq!(ts.advance(Duration::from_millis(34)), 2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedTimestep {
    step: Duration,
    accumulator: Duration,
    max_frame: Duration,
}

impl FixedTimestep {
    /// Limite padrão de tempo considerado por frame. Evita a "espiral da morte":
    /// depois de uma pausa longa (janela arrastada, breakpoint), a engine não
    /// tenta recuperar segundos inteiros de simulação de uma vez.
    pub const DEFAULT_MAX_FRAME: Duration = Duration::from_millis(250);

    /// Cria um timestep com o tamanho de passo informado.
    ///
    /// # Panics
    /// Se `step` for zero.
    pub fn new(step: Duration) -> Self {
        assert!(
            !step.is_zero(),
            "o passo do timestep fixo não pode ser zero"
        );
        Self {
            step,
            accumulator: Duration::ZERO,
            max_frame: Self::DEFAULT_MAX_FRAME,
        }
    }

    /// Cria um timestep com `hz` passos por segundo.
    ///
    /// # Panics
    /// Se `hz` for zero.
    pub fn from_hz(hz: u32) -> Self {
        assert!(
            hz > 0,
            "a frequência do timestep fixo precisa ser maior que zero"
        );
        Self::new(Duration::from_secs_f64(1.0 / f64::from(hz)))
    }

    /// Define o tempo máximo considerado em um único frame.
    pub fn with_max_frame(mut self, max_frame: Duration) -> Self {
        self.max_frame = max_frame.max(self.step);
        self
    }

    /// Tamanho de cada passo de simulação.
    pub fn step(&self) -> Duration {
        self.step
    }

    /// Tamanho de cada passo de simulação, em segundos.
    pub fn step_secs(&self) -> f32 {
        self.step.as_secs_f32()
    }

    /// Acrescenta o tempo real do frame e devolve quantos passos de simulação
    /// devem ser executados agora. O resto fica acumulado para o próximo frame.
    pub fn advance(&mut self, frame_time: Duration) -> u32 {
        self.accumulator += frame_time.min(self.max_frame);
        let mut steps = 0;
        while self.accumulator >= self.step {
            self.accumulator -= self.step;
            steps += 1;
        }
        steps
    }

    /// Fração (de 0.0 a 1.0) do próximo passo que já foi acumulada.
    /// Use para interpolar o estado renderizado entre dois passos.
    pub fn alpha(&self) -> f32 {
        (self.accumulator.as_secs_f64() / self.step.as_secs_f64()) as f32
    }
}

impl Default for FixedTimestep {
    /// 60 passos por segundo.
    fn default() -> Self {
        Self::from_hz(60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(v: u64) -> Duration {
        Duration::from_millis(v)
    }

    #[test]
    fn frame_curto_acumula_sem_passo() {
        let mut ts = FixedTimestep::new(ms(10));
        assert_eq!(ts.advance(ms(4)), 0);
        assert!((ts.alpha() - 0.4).abs() < 1e-6);
    }

    #[test]
    fn frame_longo_gera_varios_passos_e_guarda_resto() {
        let mut ts = FixedTimestep::new(ms(10));
        assert_eq!(ts.advance(ms(35)), 3);
        assert!((ts.alpha() - 0.5).abs() < 1e-6);
        // O resto de 5 ms soma com o próximo frame.
        assert_eq!(ts.advance(ms(5)), 1);
        assert!(ts.alpha().abs() < 1e-6);
    }

    #[test]
    fn total_de_passos_independe_da_taxa_de_quadros() {
        let mut rapido = FixedTimestep::new(ms(10));
        let mut lento = FixedTimestep::new(ms(10));
        let passos_rapido: u32 = (0..100).map(|_| rapido.advance(ms(2))).sum();
        let passos_lento: u32 = (0..10).map(|_| lento.advance(ms(20))).sum();
        assert_eq!(passos_rapido, 20);
        assert_eq!(passos_lento, 20);
    }

    #[test]
    fn frame_gigante_e_limitado() {
        let mut ts = FixedTimestep::new(ms(10)).with_max_frame(ms(50));
        assert_eq!(ts.advance(Duration::from_secs(5)), 5);
    }

    #[test]
    fn from_hz_calcula_o_passo() {
        let ts = FixedTimestep::from_hz(50);
        assert_eq!(ts.step(), ms(20));
    }

    #[test]
    #[should_panic]
    fn passo_zero_e_rejeitado() {
        let _ = FixedTimestep::new(Duration::ZERO);
    }

    #[test]
    fn time_acumula() {
        let mut t = Time::default();
        t.tick(ms(16));
        t.tick(ms(17));
        assert_eq!(t.frame(), 2);
        assert_eq!(t.elapsed(), ms(33));
        assert_eq!(t.delta(), ms(17));
    }
}
