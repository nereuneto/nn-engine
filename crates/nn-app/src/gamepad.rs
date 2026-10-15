//! Controles (gamepads) lidos com a biblioteca gilrs e repassados ao [`Input`].

use std::collections::HashMap;

use gilrs::{Axis, Button, EventType, GamepadId, Gilrs};
use nn_input::{Input, PadAxis, PadButton};

/// Leitor de controles. Sem suporte no sistema, simplesmente não lê nada.
pub(crate) struct Gamepads {
    gilrs: Option<Gilrs>,
    slots: HashMap<GamepadId, usize>,
}

impl std::fmt::Debug for Gamepads {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gamepads")
            .field("enabled", &self.gilrs.is_some())
            .field("slots", &self.slots)
            .finish()
    }
}

impl Gamepads {
    pub(crate) fn new(input: &mut Input) -> Self {
        let gilrs = match Gilrs::new() {
            Ok(g) => Some(g),
            Err(gilrs::Error::NotImplemented(_)) => {
                log::warn!("controles não são suportados neste sistema");
                None
            }
            Err(e) => {
                log::warn!("não foi possível ler controles: {e}");
                None
            }
        };
        let mut pads = Self {
            gilrs,
            slots: HashMap::new(),
        };
        // Controles que já estavam conectados ao abrir o jogo.
        let connected: Vec<(GamepadId, String)> = pads
            .gilrs
            .iter()
            .flat_map(|g| g.gamepads())
            .map(|(id, pad)| (id, pad.name().to_owned()))
            .collect();
        for (id, name) in connected {
            pads.connect(input, id, name);
        }
        pads
    }

    fn connect(&mut self, input: &mut Input, id: GamepadId, name: String) {
        if self.slots.contains_key(&id) {
            return;
        }
        let slot = input.connect_pad(name.clone());
        log::info!("controle conectado no lugar {}: {name}", slot + 1);
        self.slots.insert(id, slot);
    }

    /// Repassa os eventos pendentes ao `input`. Com a janela sem foco, só
    /// conexões e desconexões são consideradas.
    pub(crate) fn poll(&mut self, input: &mut Input, focused: bool) {
        let Some(gilrs) = self.gilrs.as_mut() else {
            return;
        };
        let mut connected = Vec::new();
        while let Some(event) = gilrs.next_event() {
            let id = event.id;
            match event.event {
                EventType::Connected => {
                    connected.push((id, gilrs.gamepad(id).name().to_owned()));
                    continue;
                }
                EventType::Disconnected => {
                    if let Some(slot) = self.slots.remove(&id) {
                        log::info!("controle desconectado do lugar {}", slot + 1);
                        input.disconnect_pad(slot);
                    }
                    continue;
                }
                _ => {}
            }
            let Some(&slot) = self.slots.get(&id) else {
                continue;
            };
            if !focused {
                continue;
            }
            match event.event {
                EventType::ButtonPressed(b, _) => {
                    if let Some(b) = map_button(b) {
                        input.press_pad(slot, b);
                    }
                }
                EventType::ButtonReleased(b, _) => {
                    if let Some(b) = map_button(b) {
                        input.release_pad(slot, b);
                    }
                }
                // Gatilhos analógicos chegam como "botão com valor".
                EventType::ButtonChanged(Button::LeftTrigger2, v, _) => {
                    input.set_pad_axis(slot, PadAxis::LeftTrigger, v);
                }
                EventType::ButtonChanged(Button::RightTrigger2, v, _) => {
                    input.set_pad_axis(slot, PadAxis::RightTrigger, v);
                }
                EventType::AxisChanged(axis, v, _) => {
                    if let Some(axis) = map_axis(axis) {
                        input.set_pad_axis(slot, axis, v);
                    }
                }
                _ => {}
            }
        }
        for (id, name) in connected {
            self.connect(input, id, name);
        }
    }
}

fn map_button(b: Button) -> Option<PadButton> {
    Some(match b {
        Button::South => PadButton::South,
        Button::East => PadButton::East,
        Button::West => PadButton::West,
        Button::North => PadButton::North,
        Button::LeftTrigger => PadButton::LeftShoulder,
        Button::RightTrigger => PadButton::RightShoulder,
        Button::LeftTrigger2 => PadButton::LeftTrigger,
        Button::RightTrigger2 => PadButton::RightTrigger,
        Button::Select => PadButton::Select,
        Button::Start => PadButton::Start,
        Button::LeftThumb => PadButton::LeftStick,
        Button::RightThumb => PadButton::RightStick,
        Button::DPadUp => PadButton::DPadUp,
        Button::DPadDown => PadButton::DPadDown,
        Button::DPadLeft => PadButton::DPadLeft,
        Button::DPadRight => PadButton::DPadRight,
        _ => return None,
    })
}

fn map_axis(a: Axis) -> Option<PadAxis> {
    Some(match a {
        Axis::LeftStickX => PadAxis::LeftX,
        Axis::LeftStickY => PadAxis::LeftY,
        Axis::RightStickX => PadAxis::RightX,
        Axis::RightStickY => PadAxis::RightY,
        // Controles sem mapeamento conhecido às vezes mandam os gatilhos como eixo Z.
        Axis::LeftZ => PadAxis::LeftTrigger,
        Axis::RightZ => PadAxis::RightTrigger,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapeamento_cobre_todos_os_botoes() {
        let mapped: std::collections::HashSet<PadButton> = [
            Button::South,
            Button::East,
            Button::West,
            Button::North,
            Button::LeftTrigger,
            Button::RightTrigger,
            Button::LeftTrigger2,
            Button::RightTrigger2,
            Button::Select,
            Button::Start,
            Button::LeftThumb,
            Button::RightThumb,
            Button::DPadUp,
            Button::DPadDown,
            Button::DPadLeft,
            Button::DPadRight,
        ]
        .into_iter()
        .filter_map(map_button)
        .collect();
        assert_eq!(mapped.len(), PadButton::ALL.len());
        assert_eq!(map_button(Button::Mode), None);
    }
}
