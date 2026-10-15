//! Estado de entrada (teclado e mouse) consultável a qualquer momento do frame.
//!
//! A engine alimenta [`Input`] com os eventos da janela e chama
//! [`Input::end_frame`] ao final de cada frame. O jogo só consulta.

use std::collections::HashSet;
use std::hash::Hash;

use glam::Vec2;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::keyboard::PhysicalKey;

pub use winit::event::MouseButton;
pub use winit::keyboard::KeyCode;

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
    fn press(&mut self, b: T) {
        // Repetição automática do teclado não conta como novo aperto.
        if self.down.insert(b) {
            self.pressed.insert(b);
        }
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

/// Estado atual do teclado e do mouse.
#[derive(Debug, Clone, Default)]
pub struct Input {
    keys: ButtonSet<KeyCode>,
    mouse: ButtonSet<MouseButton>,
    cursor: Option<Vec2>,
    scroll: Vec2,
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

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

    /// Posição do cursor em pixels físicos da janela (origem no canto superior
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

    /// Registra o aperto de uma tecla. Normalmente chamado pela engine.
    pub fn press_key(&mut self, key: KeyCode) {
        self.keys.press(key);
    }

    /// Registra a soltura de uma tecla. Normalmente chamado pela engine.
    pub fn release_key(&mut self, key: KeyCode) {
        self.keys.release(key);
    }

    /// Registra o aperto de um botão do mouse. Normalmente chamado pela engine.
    pub fn press_mouse(&mut self, button: MouseButton) {
        self.mouse.press(button);
    }

    /// Registra a soltura de um botão do mouse. Normalmente chamado pela engine.
    pub fn release_mouse(&mut self, button: MouseButton) {
        self.mouse.release(button);
    }

    /// Solta tudo. Usado quando a janela perde o foco, para nenhuma tecla
    /// ficar "presa".
    pub fn release_all(&mut self) {
        self.keys.release_all();
        self.mouse.release_all();
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
            }
            WindowEvent::MouseInput { state, button, .. } => match state {
                ElementState::Pressed => self.press_mouse(*button),
                ElementState::Released => self.release_mouse(*button),
            },
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = Some(Vec2::new(position.x as f32, position.y as f32));
            }
            WindowEvent::CursorLeft { .. } => self.cursor = None,
            WindowEvent::MouseWheel { delta, .. } => {
                self.scroll += match delta {
                    MouseScrollDelta::LineDelta(x, y) => Vec2::new(*x, *y),
                    // Aproximação: ~20 px por linha.
                    MouseScrollDelta::PixelDelta(p) => Vec2::new(p.x as f32, p.y as f32) / 20.0,
                };
            }
            WindowEvent::Focused(false) => self.release_all(),
            _ => {}
        }
    }

    /// Encerra o frame: limpa transições ("apertou/soltou neste frame") e a rolagem.
    pub fn end_frame(&mut self) {
        self.keys.end_frame();
        self.mouse.end_frame();
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
        input.process_event(&WindowEvent::Focused(false));
        assert!(!input.key_down(KeyCode::KeyW));
        assert!(!input.mouse_down(MouseButton::Left));
        assert!(input.key_released(KeyCode::KeyW));
    }
}
