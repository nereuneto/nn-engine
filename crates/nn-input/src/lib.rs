//! Estado de entrada (teclado, mouse e controles) consultável a qualquer
//! momento do frame.
//!
//! A engine alimenta [`Input`] com os eventos da janela e dos controles e
//! chama [`Input::end_frame`] ao final de cada frame. O jogo só consulta.
//!
//! Além das consultas diretas (`key_down`, `pad_pressed`...), dá para dar
//! nome a ações e ligar várias entradas a cada uma:
//!
//! ```
//! use nn_input::{Binding, Input, KeyCode, PadButton};
//!
//! let mut input = Input::new();
//! input.bind("pular", [
//!     Binding::Key(KeyCode::Space),
//!     Binding::Key(KeyCode::KeyW),
//!     Binding::Pad(PadButton::South),
//! ]);
//!
//! input.press_key(KeyCode::Space);
//! assert!(input.action_pressed("pular"));
//! ```

mod action;
mod gamepad;

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use glam::Vec2;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::keyboard::PhysicalKey;

pub use action::{AXIS_THRESHOLD, AxisDir, Binding};
pub use gamepad::{DEFAULT_DEADZONE, Gamepad, PadAxis, PadButton};
pub use winit::event::MouseButton;
pub use winit::keyboard::KeyCode;

use action::Action;

/// Conjunto de "botões" com estado contínuo e transições do frame.
#[derive(Debug, Clone)]
struct ButtonSet<T> {
    down: HashSet<T>,
    pressed: HashSet<T>,
    released: HashSet<T>,
}

impl<T> Default for ButtonSet<T> {
    fn default() -> Self {
        Self {
            down: HashSet::new(),
            pressed: HashSet::new(),
            released: HashSet::new(),
        }
    }
}

impl<T: Copy + Eq + Hash> ButtonSet<T> {
    /// Devolve `true` se é um aperto novo.
    fn press(&mut self, b: T) -> bool {
        // Repetição automática do teclado não conta como novo aperto.
        let new = self.down.insert(b);
        if new {
            self.pressed.insert(b);
        }
        new
    }

    fn release(&mut self, b: T) {
        if self.down.remove(&b) {
            self.released.insert(b);
        }
    }

    fn release_all(&mut self) {
        let down: Vec<T> = self.down.iter().copied().collect();
        for b in down {
            self.release(b);
        }
    }

    fn end_frame(&mut self) {
        self.pressed.clear();
        self.released.clear();
    }
}

/// Último tipo de dispositivo usado pelo jogador. Útil para mostrar
/// "aperte Enter" ou "aperte A", ou esconder o destaque do menu ao usar o mouse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Device {
    #[default]
    Keyboard,
    Mouse,
    Gamepad,
}

/// Estado atual do teclado, do mouse e dos controles.
#[derive(Debug, Clone)]
pub struct Input {
    keys: ButtonSet<KeyCode>,
    mouse: ButtonSet<MouseButton>,
    cursor: Option<Vec2>,
    cursor_transform: Option<(Vec2, Vec2)>,
    scroll: Vec2,
    pads: Vec<Gamepad>,
    deadzone: f32,
    actions: HashMap<String, Action>,
    last_device: Device,
    /// Texto digitado neste frame (letras, com acento e maiúsculas).
    typed: String,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            keys: ButtonSet::default(),
            mouse: ButtonSet::default(),
            cursor: None,
            cursor_transform: None,
            scroll: Vec2::ZERO,
            pads: Vec::new(),
            deadzone: DEFAULT_DEADZONE,
            actions: HashMap::new(),
            last_device: Device::default(),
            typed: String::new(),
        }
    }
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

    // ----- teclado e mouse -----

    /// Tecla está segurada neste momento.
    pub fn key_down(&self, key: KeyCode) -> bool {
        self.keys.down.contains(&key)
    }

    /// Tecla foi apertada neste frame.
    pub fn key_pressed(&self, key: KeyCode) -> bool {
        self.keys.pressed.contains(&key)
    }

    /// Tecla foi solta neste frame.
    pub fn key_released(&self, key: KeyCode) -> bool {
        self.keys.released.contains(&key)
    }

    /// Alguma tecla foi apertada neste frame ("aperte qualquer tecla").
    pub fn any_key_pressed(&self) -> bool {
        !self.keys.pressed.is_empty()
    }

    /// Botão do mouse está segurado neste momento.
    pub fn mouse_down(&self, button: MouseButton) -> bool {
        self.mouse.down.contains(&button)
    }

    /// Botão do mouse foi apertado neste frame.
    pub fn mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse.pressed.contains(&button)
    }

    /// Botão do mouse foi solto neste frame.
    pub fn mouse_released(&self, button: MouseButton) -> bool {
        self.mouse.released.contains(&button)
    }

    /// Posição do cursor na área do jogo (pixels físicos, ou unidades do
    /// tamanho fixo do jogo; origem no canto superior
    /// esquerdo, Y para baixo). `None` se o cursor está fora da janela.
    pub fn cursor_position(&self) -> Option<Vec2> {
        self.cursor
    }

    /// Rolagem acumulada neste frame, em "linhas" (aprox.).
    pub fn scroll(&self) -> Vec2 {
        self.scroll
    }

    /// Eixo de -1.0 a 1.0 a partir de duas teclas.
    pub fn axis(&self, negative: KeyCode, positive: KeyCode) -> f32 {
        f32::from(i8::from(self.key_down(positive)) - i8::from(self.key_down(negative)))
    }

    /// Direção 2D normalizada (comprimento 0 ou 1) a partir de quatro teclas,
    /// com Y para cima. Andar na diagonal não fica mais rápido.
    pub fn direction(&self, left: KeyCode, right: KeyCode, down: KeyCode, up: KeyCode) -> Vec2 {
        Vec2::new(self.axis(left, right), self.axis(down, up)).normalize_or_zero()
    }

    /// Atalho para [`Input::direction`] com WASD.
    pub fn wasd(&self) -> Vec2 {
        self.direction(KeyCode::KeyA, KeyCode::KeyD, KeyCode::KeyS, KeyCode::KeyW)
    }

    /// Atalho para [`Input::direction`] com as setas.
    pub fn arrows(&self) -> Vec2 {
        self.direction(
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
            KeyCode::ArrowDown,
            KeyCode::ArrowUp,
        )
    }

    /// Direção de movimento juntando tudo: WASD, setas, direcional e
    /// analógico esquerdo de qualquer controle. Comprimento de 0 a 1 (o
    /// analógico permite andar devagar), com Y para cima.
    pub fn move_vector(&self) -> Vec2 {
        let pad = |b| f32::from(u8::from(self.pad_down(b)));
        let digital = Vec2::new(
            self.axis(KeyCode::KeyA, KeyCode::KeyD)
                + self.axis(KeyCode::ArrowLeft, KeyCode::ArrowRight)
                + pad(PadButton::DPadRight)
                - pad(PadButton::DPadLeft),
            self.axis(KeyCode::KeyS, KeyCode::KeyW)
                + self.axis(KeyCode::ArrowDown, KeyCode::ArrowUp)
                + pad(PadButton::DPadUp)
                - pad(PadButton::DPadDown),
        )
        .clamp(Vec2::NEG_ONE, Vec2::ONE)
        .normalize_or_zero();
        let stick = self.left_stick();
        if stick.length_squared() > digital.length_squared() {
            stick
        } else {
            digital
        }
    }

    /// Registra o aperto de uma tecla. Normalmente chamado pela engine.
    pub fn press_key(&mut self, key: KeyCode) {
        if self.keys.press(key) {
            self.last_device = Device::Keyboard;
        }
    }

    /// O texto digitado neste frame, como o sistema entendeu as teclas
    /// (maiúsculas, acentos, Ç...). Sem as teclas de controle (Enter,
    /// Backspace, Tab, Esc): veja essas com [`Input::key_pressed`].
    pub fn typed(&self) -> &str {
        &self.typed
    }

    /// Registra texto digitado. Normalmente chamado pela engine.
    pub fn type_text(&mut self, text: &str) {
        self.typed.extend(text.chars().filter(|c| !c.is_control()));
        self.last_device = Device::Keyboard;
    }

    /// Registra a soltura de uma tecla. Normalmente chamado pela engine.
    pub fn release_key(&mut self, key: KeyCode) {
        self.keys.release(key);
    }

    /// Registra o aperto de um botão do mouse. Normalmente chamado pela engine.
    pub fn press_mouse(&mut self, button: MouseButton) {
        self.mouse.press(button);
        self.last_device = Device::Mouse;
    }

    /// Registra a soltura de um botão do mouse. Normalmente chamado pela engine.
    pub fn release_mouse(&mut self, button: MouseButton) {
        self.mouse.release(button);
    }

    /// Com tamanho fixo do jogo: a área do jogo começa em `origin` (pixels da
    /// janela) e cada unidade do jogo tem `scale` pixels. O cursor passa a
    /// vir em unidades do jogo. Normalmente chamado pela engine.
    pub fn set_cursor_transform(&mut self, transform: Option<(Vec2, Vec2)>) {
        self.cursor_transform = transform;
    }

    /// Move o cursor (pixels físicos, Y para baixo). Normalmente chamado pela engine.
    pub fn set_cursor(&mut self, position: Option<Vec2>) {
        if let (Some(new), Some(old)) = (position, self.cursor)
            && new.distance_squared(old) > 4.0
        {
            self.last_device = Device::Mouse;
        }
        self.cursor = position;
    }

    /// Solta tudo. Usado quando a janela perde o foco, para nenhuma tecla
    /// ficar "presa".
    pub fn release_all(&mut self) {
        self.keys.release_all();
        self.mouse.release_all();
        for pad in &mut self.pads {
            pad.buttons.release_all();
            for axis in PadAxis::ALL {
                pad.set_axis(axis, 0.0);
            }
        }
    }

    /// Atualiza o estado a partir de um evento da janela.
    pub fn process_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => self.press_key(code),
                        ElementState::Released => self.release_key(code),
                    }
                }
                if event.state == ElementState::Pressed
                    && let Some(text) = &event.text
                {
                    self.type_text(text);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed => self.press_mouse(*button),
                ElementState::Released => self.release_mouse(*button),
            },
            WindowEvent::CursorMoved { position, .. } => {
                let mut p = Vec2::new(position.x as f32, position.y as f32);
                if let Some((origin, scale)) = self.cursor_transform {
                    p = (p - origin) / scale;
                }
                self.set_cursor(Some(p));
            }
            WindowEvent::CursorLeft { .. } => self.cursor = None,
            WindowEvent::MouseWheel { delta, .. } => {
                self.scroll += match delta {
                    MouseScrollDelta::LineDelta(x, y) => Vec2::new(*x, *y),
                    // Aproximação: ~20 px por linha.
                    MouseScrollDelta::PixelDelta(p) => Vec2::new(p.x as f32, p.y as f32) / 20.0,
                };
                self.last_device = Device::Mouse;
            }
            WindowEvent::Focused(false) => self.release_all(),
            _ => {}
        }
    }

    // ----- controles -----

    /// Registra um controle conectado e devolve o lugar (slot) dele.
    /// Reaproveita o primeiro lugar vago, então um controle que cai e volta
    /// costuma ficar no mesmo lugar. Normalmente chamado pela engine.
    pub fn connect_pad(&mut self, name: impl Into<String>) -> usize {
        let name = name.into();
        if let Some(slot) = self.pads.iter().position(|p| !p.is_connected()) {
            self.pads[slot].reconnect(name);
            slot
        } else {
            self.pads.push(Gamepad::new(name, self.deadzone));
            self.pads.len() - 1
        }
    }

    /// Registra que o controle do lugar `slot` foi desconectado: os botões
    /// dele contam como soltos. Normalmente chamado pela engine.
    pub fn disconnect_pad(&mut self, slot: usize) {
        if let Some(pad) = self.pads.get_mut(slot) {
            pad.disconnect();
        }
    }

    /// Registra o aperto de um botão do controle. Normalmente chamado pela engine.
    pub fn press_pad(&mut self, slot: usize, button: PadButton) {
        if let Some(pad) = self.pads.get_mut(slot).filter(|p| p.is_connected())
            && pad.buttons.press(button)
        {
            self.last_device = Device::Gamepad;
        }
    }

    /// Registra a soltura de um botão do controle. Normalmente chamado pela engine.
    pub fn release_pad(&mut self, slot: usize, button: PadButton) {
        if let Some(pad) = self.pads.get_mut(slot) {
            pad.buttons.release(button);
        }
    }

    /// Atualiza um eixo do controle. Normalmente chamado pela engine.
    pub fn set_pad_axis(&mut self, slot: usize, axis: PadAxis, value: f32) {
        if let Some(pad) = self.pads.get_mut(slot).filter(|p| p.is_connected()) {
            pad.set_axis(axis, value);
            if pad.raw_axis(axis).abs() >= AXIS_THRESHOLD {
                self.last_device = Device::Gamepad;
            }
        }
    }

    /// Controle no lugar `slot`, se estiver conectado.
    pub fn pad(&self, slot: usize) -> Option<&Gamepad> {
        self.pads.get(slot).filter(|p| p.is_connected())
    }

    /// Controles conectados, com o lugar de cada um.
    pub fn pads(&self) -> impl Iterator<Item = (usize, &Gamepad)> {
        self.pads
            .iter()
            .enumerate()
            .filter(|(_, p)| p.is_connected())
    }

    /// Quantos controles estão conectados.
    pub fn pad_count(&self) -> usize {
        self.pads().count()
    }

    /// Botão segurado em qualquer controle.
    pub fn pad_down(&self, button: PadButton) -> bool {
        self.pads().any(|(_, p)| p.down(button))
    }

    /// Botão apertado neste frame em qualquer controle.
    pub fn pad_pressed(&self, button: PadButton) -> bool {
        self.pads().any(|(_, p)| p.pressed(button))
    }

    /// Botão solto neste frame em qualquer controle.
    pub fn pad_released(&self, button: PadButton) -> bool {
        self.pads().any(|(_, p)| p.released(button))
    }

    /// Algum botão de algum controle foi apertado neste frame.
    pub fn any_pad_pressed(&self) -> bool {
        self.pads().any(|(_, p)| !p.buttons.pressed.is_empty())
    }

    /// Eixo com zona morta, do controle que estiver mais inclinado nele.
    pub fn pad_axis(&self, axis: PadAxis) -> f32 {
        self.pads()
            .map(|(_, p)| p.axis(axis))
            .fold(0.0, |best, v| if v.abs() > best.abs() { v } else { best })
    }

    /// Analógico esquerdo do controle que estiver mais inclinado.
    pub fn left_stick(&self) -> Vec2 {
        self.strongest(Gamepad::left_stick)
    }

    /// Analógico direito do controle que estiver mais inclinado.
    pub fn right_stick(&self) -> Vec2 {
        self.strongest(Gamepad::right_stick)
    }

    fn strongest(&self, f: impl Fn(&Gamepad) -> Vec2) -> Vec2 {
        self.pads().map(|(_, p)| f(p)).fold(Vec2::ZERO, |best, v| {
            if v.length_squared() > best.length_squared() {
                v
            } else {
                best
            }
        })
    }

    /// Zona morta dos analógicos (padrão [`DEFAULT_DEADZONE`]), de 0.0 a 0.9.
    pub fn set_deadzone(&mut self, deadzone: f32) {
        self.deadzone = deadzone.clamp(0.0, 0.9);
        for pad in &mut self.pads {
            pad.deadzone = self.deadzone;
        }
    }

    pub fn deadzone(&self) -> f32 {
        self.deadzone
    }

    /// Último tipo de dispositivo usado.
    pub fn last_device(&self) -> Device {
        self.last_device
    }

    // ----- ações -----

    /// Define (ou redefine) uma ação com as entradas que a acionam.
    pub fn bind<B: Into<Binding>>(
        &mut self,
        action: impl Into<String>,
        bindings: impl IntoIterator<Item = B>,
    ) {
        let action = self.actions.entry(action.into()).or_default();
        action.bindings = bindings.into_iter().map(Into::into).collect();
    }

    /// Acrescenta uma entrada a uma ação (criando a ação se preciso).
    pub fn add_binding(&mut self, action: impl Into<String>, binding: impl Into<Binding>) {
        let binding = binding.into();
        let action = self.actions.entry(action.into()).or_default();
        if !action.bindings.contains(&binding) {
            action.bindings.push(binding);
        }
    }

    /// Remove uma ação.
    pub fn unbind(&mut self, action: &str) {
        self.actions.remove(action);
    }

    /// A ação existe (foi definida com [`Input::bind`]).
    pub fn has_action(&self, action: &str) -> bool {
        self.actions.contains_key(action)
    }

    /// Entradas ligadas a uma ação (vazio se ela não existe).
    pub fn bindings(&self, action: &str) -> &[Binding] {
        self.actions
            .get(action)
            .map(|a| a.bindings.as_slice())
            .unwrap_or_default()
    }

    /// Alguma entrada da ação está segurada.
    pub fn action_down(&self, action: &str) -> bool {
        self.actions
            .get(action)
            .is_some_and(|a| self.bindings_down(&a.bindings))
    }

    /// A ação começou neste frame (nenhuma entrada dela estava segurada antes).
    pub fn action_pressed(&self, action: &str) -> bool {
        self.actions.get(action).is_some_and(|a| {
            !a.was_down
                && (self.bindings_down(&a.bindings)
                    || a.bindings.iter().any(|b| self.binding_pressed(*b)))
        })
    }

    /// A ação terminou neste frame (todas as entradas dela foram soltas).
    pub fn action_released(&self, action: &str) -> bool {
        self.actions.get(action).is_some_and(|a| {
            if self.bindings_down(&a.bindings) {
                return false;
            }
            // Também conta um toque rápido que apertou e soltou no mesmo frame.
            a.was_down
                || (a.bindings.iter().any(|b| self.binding_pressed(*b))
                    && a.bindings.iter().any(|b| self.binding_released(*b)))
        })
    }

    /// Intensidade da ação de 0.0 a 1.0: 1.0 para teclas e botões, o quanto
    /// o eixo está inclinado para eixos.
    pub fn action_value(&self, action: &str) -> f32 {
        self.actions.get(action).map_or(0.0, |a| {
            a.bindings
                .iter()
                .map(|b| self.binding_value(*b))
                .fold(0.0, f32::max)
        })
    }

    /// Eixo de -1.0 a 1.0 a partir de duas ações (ex.: "esquerda" e "direita").
    pub fn action_axis(&self, negative: &str, positive: &str) -> f32 {
        self.action_value(positive) - self.action_value(negative)
    }

    fn bindings_down(&self, bindings: &[Binding]) -> bool {
        bindings.iter().any(|b| self.binding_down(*b))
    }

    fn binding_down(&self, binding: Binding) -> bool {
        match binding {
            Binding::Key(k) => self.key_down(k),
            Binding::Mouse(m) => self.mouse_down(m),
            Binding::Pad(b) => self.pad_down(b),
            Binding::PadAxis(axis, dir) => self
                .pads()
                .any(|(_, p)| p.raw_axis(axis) * dir.sign() >= AXIS_THRESHOLD),
        }
    }

    fn binding_pressed(&self, binding: Binding) -> bool {
        match binding {
            Binding::Key(k) => self.key_pressed(k),
            Binding::Mouse(m) => self.mouse_pressed(m),
            Binding::Pad(b) => self.pad_pressed(b),
            Binding::PadAxis(..) => false,
        }
    }

    fn binding_released(&self, binding: Binding) -> bool {
        match binding {
            Binding::Key(k) => self.key_released(k),
            Binding::Mouse(m) => self.mouse_released(m),
            Binding::Pad(b) => self.pad_released(b),
            Binding::PadAxis(..) => false,
        }
    }

    fn binding_value(&self, binding: Binding) -> f32 {
        match binding {
            Binding::PadAxis(axis, dir) => self
                .pads()
                .map(|(_, p)| (p.axis(axis) * dir.sign()).max(0.0))
                .fold(0.0, f32::max),
            other => f32::from(u8::from(self.binding_down(other))),
        }
    }

    // ----- frame -----

    /// Encerra o frame: limpa transições ("apertou/soltou neste frame") e a rolagem.
    pub fn end_frame(&mut self) {
        let states: Vec<(String, bool)> = self
            .actions
            .iter()
            .map(|(name, a)| (name.clone(), self.bindings_down(&a.bindings)))
            .collect();
        for (name, down) in states {
            if let Some(a) = self.actions.get_mut(&name) {
                a.was_down = down;
            }
        }
        self.keys.end_frame();
        self.mouse.end_frame();
        self.typed.clear();
        for pad in &mut self.pads {
            pad.buttons.end_frame();
        }
        self.scroll = Vec2::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aperto_dura_um_frame_e_segurar_continua() {
        let mut input = Input::new();
        input.press_key(KeyCode::Space);
        assert!(input.key_pressed(KeyCode::Space));
        assert!(input.key_down(KeyCode::Space));

        input.end_frame();
        assert!(!input.key_pressed(KeyCode::Space));
        assert!(input.key_down(KeyCode::Space));

        input.release_key(KeyCode::Space);
        assert!(input.key_released(KeyCode::Space));
        assert!(!input.key_down(KeyCode::Space));
        input.end_frame();
        assert!(!input.key_released(KeyCode::Space));
    }

    #[test]
    fn repeticao_automatica_nao_gera_novo_aperto() {
        let mut input = Input::new();
        input.press_key(KeyCode::KeyA);
        input.end_frame();
        input.press_key(KeyCode::KeyA); // evento de repetição do SO
        assert!(!input.key_pressed(KeyCode::KeyA));
    }

    #[test]
    fn soltar_tecla_nao_segurada_e_ignorado() {
        let mut input = Input::new();
        input.release_key(KeyCode::KeyA);
        assert!(!input.key_released(KeyCode::KeyA));
    }

    #[test]
    fn wasd_diagonal_e_normalizada() {
        let mut input = Input::new();
        input.press_key(KeyCode::KeyW);
        input.press_key(KeyCode::KeyD);
        let dir = input.wasd();
        assert!((dir.length() - 1.0).abs() < 1e-6);
        assert!(dir.x > 0.0 && dir.y > 0.0);
    }

    #[test]
    fn teclas_opostas_se_anulam() {
        let mut input = Input::new();
        input.press_key(KeyCode::KeyA);
        input.press_key(KeyCode::KeyD);
        assert_eq!(input.wasd(), Vec2::ZERO);
    }

    #[test]
    fn perder_foco_solta_tudo() {
        let mut input = Input::new();
        input.press_key(KeyCode::KeyW);
        input.press_mouse(MouseButton::Left);
        let pad = input.connect_pad("controle");
        input.press_pad(pad, PadButton::South);
        input.set_pad_axis(pad, PadAxis::LeftX, 1.0);
        input.process_event(&WindowEvent::Focused(false));
        assert!(!input.key_down(KeyCode::KeyW));
        assert!(!input.mouse_down(MouseButton::Left));
        assert!(input.key_released(KeyCode::KeyW));
        assert!(!input.pad_down(PadButton::South));
        assert_eq!(input.pad_axis(PadAxis::LeftX), 0.0);
    }

    #[test]
    fn botoes_do_controle_tem_transicoes() {
        let mut input = Input::new();
        let pad = input.connect_pad("controle");
        assert_eq!(pad, 0);
        input.press_pad(pad, PadButton::South);
        assert!(input.pad_pressed(PadButton::South));
        assert!(input.pad(0).unwrap().down(PadButton::South));
        input.end_frame();
        assert!(!input.pad_pressed(PadButton::South));
        assert!(input.pad_down(PadButton::South));
        input.release_pad(pad, PadButton::South);
        assert!(input.pad_released(PadButton::South));
        assert_eq!(input.last_device(), Device::Gamepad);
    }

    #[test]
    fn controle_desconectado_solta_botoes_e_libera_lugar() {
        let mut input = Input::new();
        let a = input.connect_pad("A");
        let b = input.connect_pad("B");
        assert_eq!((a, b), (0, 1));
        input.press_pad(a, PadButton::Start);
        input.end_frame();

        input.disconnect_pad(a);
        assert!(input.pad(a).is_none());
        assert_eq!(input.pad_count(), 1);
        assert!(!input.pad_down(PadButton::Start));

        // Eventos de um controle desconectado são ignorados.
        input.press_pad(a, PadButton::North);
        assert!(!input.pad_down(PadButton::North));

        // O próximo a conectar ocupa o lugar vago.
        let c = input.connect_pad("C");
        assert_eq!(c, 0);
        assert_eq!(input.pad(0).unwrap().name(), "C");
        assert!(!input.pad_down(PadButton::North));
    }

    #[test]
    fn eixo_pega_o_controle_mais_inclinado() {
        let mut input = Input::new();
        let a = input.connect_pad("A");
        let b = input.connect_pad("B");
        input.set_pad_axis(a, PadAxis::LeftX, 0.5);
        input.set_pad_axis(b, PadAxis::LeftX, -1.0);
        assert!((input.pad_axis(PadAxis::LeftX) + 1.0).abs() < 1e-6);
        input.set_pad_axis(b, PadAxis::LeftX, 0.0);
        assert!(input.pad_axis(PadAxis::LeftX) > 0.0);
    }

    #[test]
    fn movimento_junta_teclado_direcional_e_analogico() {
        let mut input = Input::new();
        assert_eq!(input.move_vector(), Vec2::ZERO);

        input.press_key(KeyCode::ArrowLeft);
        assert_eq!(input.move_vector(), Vec2::NEG_X);
        // Seta e A juntas não andam mais rápido.
        input.press_key(KeyCode::KeyA);
        assert_eq!(input.move_vector(), Vec2::NEG_X);
        input.release_key(KeyCode::ArrowLeft);
        input.release_key(KeyCode::KeyA);

        let pad = input.connect_pad("controle");
        input.press_pad(pad, PadButton::DPadUp);
        assert_eq!(input.move_vector(), Vec2::Y);
        input.release_pad(pad, PadButton::DPadUp);

        // Analógico pela metade anda devagar.
        input.set_pad_axis(pad, PadAxis::LeftX, 0.6);
        let v = input.move_vector();
        assert!(v.x > 0.4 && v.x < 0.6 && v.y == 0.0, "{v}");
    }

    #[test]
    fn acao_com_varias_entradas() {
        let mut input = Input::new();
        input.bind(
            "pular",
            [
                Binding::Key(KeyCode::Space),
                Binding::Key(KeyCode::KeyW),
                Binding::Pad(PadButton::South),
            ],
        );
        assert!(input.has_action("pular"));
        assert!(!input.action_down("pular"));

        input.press_key(KeyCode::Space);
        assert!(input.action_pressed("pular"));
        input.end_frame();

        // Apertar a segunda tecla com a primeira segurada não é um novo aperto.
        input.press_key(KeyCode::KeyW);
        assert!(input.action_down("pular"));
        assert!(!input.action_pressed("pular"));
        input.end_frame();

        input.release_key(KeyCode::Space);
        assert!(!input.action_released("pular"));
        input.end_frame();
        input.release_key(KeyCode::KeyW);
        assert!(input.action_released("pular"));
        input.end_frame();

        let pad = input.connect_pad("controle");
        input.press_pad(pad, PadButton::South);
        assert!(input.action_pressed("pular"));
    }

    #[test]
    fn toque_rapido_no_mesmo_frame_conta() {
        let mut input = Input::new();
        input.bind("atirar", [KeyCode::KeyJ]);
        input.press_key(KeyCode::KeyJ);
        input.release_key(KeyCode::KeyJ);
        assert!(!input.action_down("atirar"));
        assert!(input.action_pressed("atirar"));
        assert!(input.action_released("atirar"));
    }

    #[test]
    fn eixo_como_botao_e_valor_analogico() {
        let mut input = Input::new();
        input.bind(
            "direita",
            [
                Binding::Key(KeyCode::KeyD),
                Binding::PadAxis(PadAxis::LeftX, AxisDir::Positive),
            ],
        );
        input.bind(
            "esquerda",
            [
                Binding::Key(KeyCode::KeyA),
                Binding::PadAxis(PadAxis::LeftX, AxisDir::Negative),
            ],
        );
        let pad = input.connect_pad("controle");

        input.set_pad_axis(pad, PadAxis::LeftX, 0.3);
        assert!(!input.action_down("direita"));
        assert!(input.action_value("direita") > 0.0);
        input.end_frame();

        input.set_pad_axis(pad, PadAxis::LeftX, 0.9);
        assert!(input.action_pressed("direita"));
        assert!(input.action_axis("esquerda", "direita") > 0.8);
        input.end_frame();

        input.set_pad_axis(pad, PadAxis::LeftX, -1.0);
        assert!(input.action_released("direita"));
        assert!(input.action_pressed("esquerda"));
        assert!((input.action_axis("esquerda", "direita") + 1.0).abs() < 1e-6);

        input.press_key(KeyCode::KeyD);
        assert_eq!(input.action_value("direita"), 1.0);
    }

    #[test]
    fn acao_desconhecida_e_falsa() {
        let mut input = Input::new();
        assert!(!input.action_pressed("nada"));
        assert_eq!(input.bindings("nada"), &[]);
        input.add_binding("menu", KeyCode::Escape);
        input.add_binding("menu", KeyCode::Escape);
        input.add_binding("menu", PadButton::Start);
        assert_eq!(input.bindings("menu").len(), 2);
        input.unbind("menu");
        assert!(!input.has_action("menu"));
    }

    #[test]
    fn ultimo_dispositivo_acompanha_o_jogador() {
        let mut input = Input::new();
        input.set_cursor(Some(Vec2::new(10.0, 10.0)));
        assert_eq!(input.last_device(), Device::Keyboard);
        input.set_cursor(Some(Vec2::new(50.0, 10.0)));
        assert_eq!(input.last_device(), Device::Mouse);
        input.press_key(KeyCode::Enter);
        assert_eq!(input.last_device(), Device::Keyboard);
        let pad = input.connect_pad("controle");
        input.set_pad_axis(pad, PadAxis::LeftY, 0.1);
        assert_eq!(input.last_device(), Device::Keyboard);
        input.set_pad_axis(pad, PadAxis::LeftY, 0.8);
        assert_eq!(input.last_device(), Device::Gamepad);
    }
}
