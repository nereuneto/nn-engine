//! Ações com nome ("pular", "atirar") ligadas a teclas, mouse e controle.

use crate::{KeyCode, MouseButton, PadAxis, PadButton};

/// Sentido de um eixo usado como botão.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxisDir {
    Negative,
    Positive,
}

impl AxisDir {
    pub(crate) fn sign(self) -> f32 {
        match self {
            AxisDir::Negative => -1.0,
            AxisDir::Positive => 1.0,
        }
    }
}

/// Uma entrada que aciona uma ação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Binding {
    Key(KeyCode),
    Mouse(MouseButton),
    /// Botão de qualquer controle conectado.
    Pad(PadButton),
    /// Eixo de qualquer controle, num sentido: conta como apertado a partir
    /// de [`AXIS_THRESHOLD`].
    PadAxis(PadAxis, AxisDir),
}

impl From<KeyCode> for Binding {
    fn from(key: KeyCode) -> Self {
        Binding::Key(key)
    }
}

impl From<MouseButton> for Binding {
    fn from(button: MouseButton) -> Self {
        Binding::Mouse(button)
    }
}

impl From<PadButton> for Binding {
    fn from(button: PadButton) -> Self {
        Binding::Pad(button)
    }
}

/// A partir de quanto um eixo usado como botão conta como apertado.
pub const AXIS_THRESHOLD: f32 = 0.5;

#[derive(Debug, Clone, Default)]
pub(crate) struct Action {
    pub(crate) bindings: Vec<Binding>,
    /// Estado no fim do frame anterior, para saber se apertou ou soltou agora.
    pub(crate) was_down: bool,
}
