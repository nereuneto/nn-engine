//! Estado de um controle (gamepad).

use glam::Vec2;

use crate::ButtonSet;

/// Botões de um controle, pela posição (layout de Xbox como referência).
///
/// Os nomes valem para qualquer controle: [`PadButton::South`] é o "A" do
/// Xbox, o "X" (cruz) do PlayStation e o "B" do Nintendo Switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PadButton {
    /// Botão de baixo (A no Xbox, cruz no PlayStation).
    South,
    /// Botão da direita (B no Xbox, círculo no PlayStation).
    East,
    /// Botão da esquerda (X no Xbox, quadrado no PlayStation).
    West,
    /// Botão de cima (Y no Xbox, triângulo no PlayStation).
    North,
    /// Botão de ombro esquerdo (LB, L1).
    LeftShoulder,
    /// Botão de ombro direito (RB, R1).
    RightShoulder,
    /// Gatilho esquerdo (LT, L2), como botão.
    LeftTrigger,
    /// Gatilho direito (RT, R2), como botão.
    RightTrigger,
    /// Select, Back, View ou Share.
    Select,
    /// Start, Menu ou Options.
    Start,
    /// Apertar o analógico esquerdo (L3).
    LeftStick,
    /// Apertar o analógico direito (R3).
    RightStick,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

impl PadButton {
    /// Todos os botões, na ordem da declaração.
    pub const ALL: [PadButton; 16] = [
        PadButton::South,
        PadButton::East,
        PadButton::West,
        PadButton::North,
        PadButton::LeftShoulder,
        PadButton::RightShoulder,
        PadButton::LeftTrigger,
        PadButton::RightTrigger,
        PadButton::Select,
        PadButton::Start,
        PadButton::LeftStick,
        PadButton::RightStick,
        PadButton::DPadUp,
        PadButton::DPadDown,
        PadButton::DPadLeft,
        PadButton::DPadRight,
    ];
}

/// Eixos analógicos de um controle.
///
/// Analógicos vão de -1.0 a 1.0, com **Y para cima** (como o mundo do jogo).
/// Gatilhos vão de 0.0 (solto) a 1.0 (apertado até o fim).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PadAxis {
    LeftX,
    LeftY,
    RightX,
    RightY,
    LeftTrigger,
    RightTrigger,
}

impl PadAxis {
    pub const ALL: [PadAxis; 6] = [
        PadAxis::LeftX,
        PadAxis::LeftY,
        PadAxis::RightX,
        PadAxis::RightY,
        PadAxis::LeftTrigger,
        PadAxis::RightTrigger,
    ];

    fn index(self) -> usize {
        self as usize
    }
}

/// Zona morta padrão dos analógicos: valores menores que isto viram zero,
/// para o personagem não andar sozinho com o analógico gasto.
pub const DEFAULT_DEADZONE: f32 = 0.2;

/// Um controle conectado (ou que já esteve conectado) num "lugar" (slot).
#[derive(Debug, Clone)]
pub struct Gamepad {
    name: String,
    connected: bool,
    pub(crate) buttons: ButtonSet<PadButton>,
    axes: [f32; 6],
    pub(crate) deadzone: f32,
}

impl Gamepad {
    pub(crate) fn new(name: String, deadzone: f32) -> Self {
        Self {
            name,
            connected: true,
            buttons: ButtonSet::default(),
            axes: [0.0; 6],
            deadzone,
        }
    }

    /// Nome informado pelo sistema (ex.: "Xbox Wireless Controller").
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Botão está segurado neste momento.
    pub fn down(&self, button: PadButton) -> bool {
        self.buttons.down.contains(&button)
    }

    /// Botão foi apertado neste frame.
    pub fn pressed(&self, button: PadButton) -> bool {
        self.buttons.pressed.contains(&button)
    }

    /// Botão foi solto neste frame.
    pub fn released(&self, button: PadButton) -> bool {
        self.buttons.released.contains(&button)
    }

    /// Valor de um eixo já com a zona morta aplicada.
    pub fn axis(&self, axis: PadAxis) -> f32 {
        apply_deadzone(self.axes[axis.index()], self.deadzone)
    }

    /// Valor de um eixo como veio do controle, sem zona morta.
    pub fn raw_axis(&self, axis: PadAxis) -> f32 {
        self.axes[axis.index()]
    }

    /// Analógico esquerdo como vetor (Y para cima), com zona morta circular.
    pub fn left_stick(&self) -> Vec2 {
        self.stick(PadAxis::LeftX, PadAxis::LeftY)
    }

    /// Analógico direito como vetor (Y para cima), com zona morta circular.
    pub fn right_stick(&self) -> Vec2 {
        self.stick(PadAxis::RightX, PadAxis::RightY)
    }

    fn stick(&self, x: PadAxis, y: PadAxis) -> Vec2 {
        let raw = Vec2::new(self.raw_axis(x), self.raw_axis(y));
        let len = raw.length();
        if len <= self.deadzone {
            return Vec2::ZERO;
        }
        // Reescala para começar em 0 na borda da zona morta e chegar a 1.
        let scaled = ((len - self.deadzone) / (1.0 - self.deadzone)).min(1.0);
        raw / len * scaled
    }

    pub(crate) fn set_axis(&mut self, axis: PadAxis, value: f32) {
        let value = if value.is_finite() { value } else { 0.0 };
        let (min, max) = match axis {
            PadAxis::LeftTrigger | PadAxis::RightTrigger => (0.0, 1.0),
            _ => (-1.0, 1.0),
        };
        self.axes[axis.index()] = value.clamp(min, max);
    }

    pub(crate) fn disconnect(&mut self) {
        self.connected = false;
        self.buttons.release_all();
        self.axes = [0.0; 6];
    }

    pub(crate) fn reconnect(&mut self, name: String) {
        self.name = name;
        self.connected = true;
    }
}

/// Zona morta num eixo isolado, reescalando o resto para 0..1.
fn apply_deadzone(value: f32, deadzone: f32) -> f32 {
    let magnitude = value.abs();
    if magnitude <= deadzone {
        0.0
    } else {
        value.signum() * ((magnitude - deadzone) / (1.0 - deadzone)).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zona_morta_zera_e_reescala() {
        assert_eq!(apply_deadzone(0.1, 0.2), 0.0);
        assert_eq!(apply_deadzone(-0.2, 0.2), 0.0);
        assert!((apply_deadzone(0.6, 0.2) - 0.5).abs() < 1e-6);
        assert!((apply_deadzone(-1.0, 0.2) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn analogico_tem_zona_morta_circular() {
        let mut pad = Gamepad::new("teste".into(), 0.2);
        pad.set_axis(PadAxis::LeftX, 0.15);
        pad.set_axis(PadAxis::LeftY, 0.1);
        assert_eq!(pad.left_stick(), Vec2::ZERO);

        pad.set_axis(PadAxis::LeftX, 0.0);
        pad.set_axis(PadAxis::LeftY, 1.0);
        assert!((pad.left_stick() - Vec2::Y).length() < 1e-6);

        // Diagonal no limite físico (~0.707 em cada eixo) tem comprimento 1.
        pad.set_axis(PadAxis::LeftX, std::f32::consts::FRAC_1_SQRT_2);
        pad.set_axis(PadAxis::LeftY, std::f32::consts::FRAC_1_SQRT_2);
        assert!((pad.left_stick().length() - 1.0).abs() < 1e-3);
    }

    #[test]
    fn gatilho_fica_entre_zero_e_um() {
        let mut pad = Gamepad::new("teste".into(), 0.2);
        pad.set_axis(PadAxis::LeftTrigger, -0.5);
        assert_eq!(pad.raw_axis(PadAxis::LeftTrigger), 0.0);
        pad.set_axis(PadAxis::RightTrigger, 2.0);
        assert_eq!(pad.raw_axis(PadAxis::RightTrigger), 1.0);
        pad.set_axis(PadAxis::LeftX, f32::NAN);
        assert_eq!(pad.raw_axis(PadAxis::LeftX), 0.0);
    }
}
