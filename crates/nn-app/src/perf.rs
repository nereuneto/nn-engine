//! O medidor de desempenho: quanto cada parte do quadro levou, e a tela F3
//! que mostra isso no jogo em desenvolvimento.

use std::time::Duration;

use glam::Vec2;
use nn_core::Color;
use nn_input::KeyCode;
use nn_render::{Anchor, Text};

use crate::Context;

/// Os tempos de um quadro, em milissegundos.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FrameTimes {
    /// O quadro inteiro (de um começo ao outro).
    pub frame: f32,
    /// Os passos fixos (`fixed_update`), com a física dentro.
    pub fixed: f32,
    /// A física (contada por quem a roda, dentro dos passos fixos).
    pub physics: f32,
    /// O `update`.
    pub update: f32,
    /// O desenho: o `render` do jogo e o envio para a placa de vídeo.
    pub draw: f32,
    /// Passos fixos neste quadro.
    pub steps: u32,
    /// Arquivos carregados neste quadro (imagens, sons, modelos, fontes).
    pub loads: u32,
    /// O tempo carregando esses arquivos.
    pub load: f32,
}

impl FrameTimes {
    fn blend(&mut self, new: &FrameTimes, k: f32) {
        let m = |a: &mut f32, b: f32| *a += (b - *a) * k;
        m(&mut self.frame, new.frame);
        m(&mut self.fixed, new.fixed);
        m(&mut self.physics, new.physics);
        m(&mut self.update, new.update);
        m(&mut self.draw, new.draw);
        self.steps = new.steps;
        self.loads = new.loads;
        self.load = new.load;
    }
}

/// O medidor (em [`Context::perf`]).
#[derive(Debug, Clone, Default)]
pub struct Perf {
    /// O último quadro.
    pub last: FrameTimes,
    /// A média dos últimos quadros (suavizada, como numa tela de jogo).
    pub average: FrameTimes,
    /// O maior tempo de quadro nos últimos segundos.
    pub worst_frame: f32,
    /// Objetos na cena (quem roda o jogo conta; 0 = não conta).
    pub objects: u32,
    /// Memória do Lua, em KB (idem).
    pub lua_kb: f32,
    /// A tela F3 está aberta.
    pub overlay: bool,
    /// F3 abre a tela (só no jogo em desenvolvimento).
    pub(crate) allowed: bool,
    current: FrameTimes,
    /// Um quadro começou e ainda não fechou.
    open: bool,
    worst_timer: f32,
    frames: u64,
    /// Arquivos carregados desde o começo, e o último quadro que carregou.
    pub total_loads: u64,
    pub last_load: FrameTimes,
    /// Os arquivos que o último quadro carregou (até 16).
    pub last_loaded: Vec<String>,
    loaded_now: Vec<String>,
    /// Sem janela (testes, `rodar_jogo`): o tempo do quadro é o trabalho
    /// dele (passos fixos, `update` e desenho), não o relógio simulado.
    pub(crate) simulated: bool,
}

impl Perf {
    pub(crate) fn new(allowed: bool) -> Self {
        Self {
            allowed,
            ..Self::default()
        }
    }

    /// Conta um arquivo carregado (imagem, som, modelo, fonte) no quadro de
    /// agora, com o tempo que levou.
    pub fn add_load(&mut self, d: Duration) {
        self.total_loads += 1;
        if self.open {
            self.current.loads += 1;
            self.current.load += ms(d);
        } else {
            // Fora de um quadro (a primeira cena, ao abrir o jogo).
            self.last_load.loads += 1;
            self.last_load.load += ms(d);
        }
    }

    /// Como [`Perf::add_load`], guardando o nome do arquivo.
    pub fn add_load_file(&mut self, d: Duration, file: &str) {
        self.add_load(d);
        if self.open && self.loaded_now.len() < 16 {
            self.loaded_now.push(file.to_string());
        }
    }

    /// Soma um tempo de física ao quadro de agora.
    pub fn add_physics(&mut self, d: Duration) {
        self.current.physics += ms(d);
    }

    pub(crate) fn begin(&mut self, frame: Duration) {
        self.open = true;
        self.current = FrameTimes {
            frame: ms(frame),
            ..FrameTimes::default()
        };
        self.loaded_now.clear();
    }

    pub(crate) fn fixed(&mut self, d: Duration, steps: u32) {
        self.current.fixed = ms(d);
        self.current.steps = steps;
    }

    pub(crate) fn update(&mut self, d: Duration) {
        self.current.update = ms(d);
    }

    pub(crate) fn draw(&mut self, d: Duration) {
        if self.open {
            self.current.draw = ms(d);
            self.end();
        }
    }

    fn end(&mut self) {
        self.open = false;
        if self.simulated {
            let c = &mut self.current;
            c.frame = c.fixed + c.update + c.draw;
        }
        self.last = self.current;
        std::mem::swap(&mut self.last_loaded, &mut self.loaded_now);
        if self.last.loads > 0 {
            self.last_load = self.last;
        }
        self.frames += 1;
        let k = if self.frames == 1 { 1.0 } else { 0.1 };
        self.average.blend(&self.last, k);
        self.worst_timer += self.last.frame / 1000.0;
        if self.worst_timer > 2.0 {
            self.worst_timer = 0.0;
            self.worst_frame = self.last.frame;
        } else {
            self.worst_frame = self.worst_frame.max(self.last.frame);
        }
    }

    /// Quadros por segundo (pela média).
    pub fn fps(&self) -> f32 {
        if self.average.frame > 0.0 {
            1000.0 / self.average.frame
        } else {
            0.0
        }
    }
}

fn ms(d: Duration) -> f32 {
    d.as_secs_f32() * 1000.0
}

impl Context {
    /// F3 abre e fecha a tela de desempenho (no jogo em desenvolvimento).
    pub(crate) fn perf_keys(&mut self) {
        if self.perf.allowed && self.input.key_pressed(KeyCode::F3) {
            self.perf.overlay = !self.perf.overlay;
        }
    }

    /// As linhas da tela F3.
    pub fn perf_lines(&self) -> Vec<String> {
        let a = &self.perf.average;
        let r = self.renderer.last_stats();
        let r3 = self.renderer.last_stats3();
        let mut lines = vec![
            format!(
                "{:.0} quadros/s  ({:.1} ms; pior {:.1} ms)",
                self.perf.fps(),
                a.frame,
                self.perf.worst_frame
            ),
            format!(
                "passos fixos {:.2} ms ({} por quadro), física {:.2} ms",
                a.fixed, a.steps, a.physics
            ),
            format!("update {:.2} ms, desenho {:.2} ms", a.update, a.draw),
            format!("desenho: {} sprites, {} chamadas", r.sprites, r.draw_calls),
        ];
        if r3.meshes > 0 {
            lines.push(format!(
                "3D: {} malhas, {} visíveis, {} chamadas",
                r3.meshes, r3.visible, r3.draw_calls
            ));
        }
        let mut mem = format!(
            "texturas: {} ({:.1} MB)",
            self.renderer.texture_count(),
            self.renderer.texture_bytes() as f64 / (1024.0 * 1024.0)
        );
        if self.perf.lua_kb > 0.0 {
            mem.push_str(&format!(", Lua {:.1} MB", self.perf.lua_kb / 1024.0));
        }
        lines.push(mem);
        if self.perf.objects > 0 {
            lines.push(format!("objetos: {}", self.perf.objects));
        }
        if self.perf.total_loads > 0 {
            let l = &self.perf.last_load;
            lines.push(format!(
                "carregados: {} arquivos; o último quadro que carregou: {} em {:.1} ms",
                self.perf.total_loads, l.loads, l.load
            ));
        }
        lines
    }

    /// Desenha a tela F3 por cima de tudo, no canto de cima à esquerda.
    pub(crate) fn draw_perf(&mut self) {
        if !self.perf.overlay {
            return;
        }
        let lines = self.perf_lines();
        let scale = (self.viewport.y as f32 / 720.0).clamp(0.5, 2.0);
        let size = 16.0 * scale;
        let pad = 8.0 * scale;
        let h = lines.len() as f32 * size * 1.25 + pad * 2.0;
        let w = 470.0 * scale;
        let mut screen = self.renderer.screen();
        screen.draw_rect(
            Vec2::splat(4.0),
            Vec2::new(w, h),
            Color::rgba(0.0, 0.0, 0.0, 0.7),
        );
        for (i, line) in lines.iter().enumerate() {
            let at = Vec2::new(4.0 + pad, 4.0 + pad + i as f32 * size * 1.25);
            let mut t = Text::new(line).at(at);
            t.size = size;
            t.color = Color::rgba(0.85, 1.0, 0.85, 1.0);
            t.anchor = Anchor::TopLeft;
            screen.draw_text(&t);
        }
    }
}
