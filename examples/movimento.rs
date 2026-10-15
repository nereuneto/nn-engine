//! Exemplo da Sprint 1: um personagem que anda com WASD.
//!
//! Controles:
//! - WASD ou setas: mover
//! - Shift: correr
//! - Roda do mouse: zoom
//! - Esc: sair
//!
//! Rode com: `cargo run --example movimento`

use nn_engine::prelude::*;

const TILE: f32 = 64.0;
const SPEED: f32 = 280.0;
const SPRINT: f32 = 1.8;

struct Movimento {
    player: TextureId,
    /// Posição no passo fixo anterior e no atual: o render interpola entre as duas.
    previous: Vec2,
    current: Vec2,
    facing_left: bool,
}

impl Movimento {
    fn new(ctx: &mut Context) -> Self {
        // Textura gerada no código para o exemplo não depender de arquivos.
        let (size, pixels) = player_pixels();
        let player = ctx
            .renderer
            .create_texture(size, &pixels, TextureOptions::PIXEL_ART);
        Self {
            player,
            previous: Vec2::ZERO,
            current: Vec2::ZERO,
            facing_left: false,
        }
    }
}

impl Game for Movimento {
    fn fixed_update(&mut self, ctx: &mut Context) {
        let input = &ctx.input;
        // Teclado (WASD ou setas) ou controle (direcional ou analógico).
        let dir = input.move_vector();

        let sprint = input.key_down(KeyCode::ShiftLeft)
            || input.key_down(KeyCode::ShiftRight)
            || input.pad_down(PadButton::RightShoulder);
        let speed = if sprint { SPEED * SPRINT } else { SPEED };

        self.previous = self.current;
        self.current += dir * speed * ctx.fixed_dt();
        if dir.x != 0.0 {
            self.facing_left = dir.x < 0.0;
        }
    }

    fn update(&mut self, ctx: &mut Context) {
        if ctx.input.key_pressed(KeyCode::Escape) {
            ctx.exit();
        }
        let scroll = ctx.input.scroll().y;
        if scroll != 0.0 {
            ctx.camera.zoom = (ctx.camera.zoom * 1.1f32.powf(scroll)).clamp(0.25, 4.0);
        }
    }

    fn render(&mut self, ctx: &mut Context) {
        let pos = self.previous.lerp(self.current, ctx.alpha());

        // Câmera segue o personagem com suavização.
        let t = 1.0 - (-8.0 * ctx.time.delta_secs()).exp();
        ctx.camera.position = ctx.camera.position.lerp(pos, t);

        draw_floor(ctx);

        // Sombra e personagem. Tamanho negativo em X espelha o sprite.
        ctx.renderer.draw_rect(
            pos - Vec2::new(0.0, 30.0),
            Vec2::new(44.0, 10.0),
            Color::BLACK.with_alpha(0.3),
        );
        let bob = (ctx.time.elapsed_secs() * 6.0).sin() * 2.0;
        let flip = if self.facing_left { -1.0 } else { 1.0 };
        ctx.renderer.draw(
            &Sprite::new(self.player)
                .at(pos + Vec2::new(0.0, bob))
                .size(Vec2::new(64.0 * flip, 64.0)),
        );
    }
}

/// Piso quadriculado só na área visível da câmera.
fn draw_floor(ctx: &mut Context) {
    let half = ctx.viewport().as_vec2() / (2.0 * ctx.camera.zoom);
    let min = ((ctx.camera.position - half) / TILE).floor();
    let max = ((ctx.camera.position + half) / TILE).ceil();
    let light = Color::hex(0x3B4A3F);
    let dark = Color::hex(0x334037);
    for y in min.y as i32..=max.y as i32 {
        for x in min.x as i32..=max.x as i32 {
            let color = if (x + y).rem_euclid(2) == 0 {
                light
            } else {
                dark
            };
            let center = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * TILE;
            ctx.renderer.draw_rect(center, Vec2::splat(TILE), color);
        }
    }
    // Marca a origem do mundo.
    ctx.renderer
        .draw_rect(Vec2::ZERO, Vec2::splat(8.0), Color::hex(0xE0C35A));
}

/// Personagem de 16×16 em pixel art.
fn player_pixels() -> (UVec2, Vec<u8>) {
    const ART: [&str; 16] = [
        "................",
        ".....######.....",
        "....#oooooo#....",
        "...#oooooooo#...",
        "...#oo#oo#oo#...",
        "...#oo#oo#oo#...",
        "...#oooooooo#...",
        "...#ooo##ooo#...",
        "....#oooooo#....",
        ".....######.....",
        "....#bbbbbb#....",
        "...#bbbbbbbb#...",
        "...#bbbbbbbb#...",
        "....#bb##bb#....",
        "....#bb#.#bb#...",
        ".....##...##....",
    ];
    let mut pixels = Vec::with_capacity(16 * 16 * 4);
    for row in ART {
        for ch in row.chars() {
            let rgba = match ch {
                '#' => [24, 20, 37, 255],
                'o' => [255, 205, 117, 255],
                'b' => [59, 93, 201, 255],
                _ => [0, 0, 0, 0],
            };
            pixels.extend_from_slice(&rgba);
        }
    }
    (UVec2::new(16, 16), pixels)
}

fn main() -> Result<(), AppError> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let config = AppConfig::new("nn-engine — movimento").clear_color(Color::hex(0x1E1E28));
    nn_engine::run(config, Movimento::new)
}
