//! Exemplo da Sprint 2: colete todas as moedas.
//!
//! Mostra spritesheets e animação, texto no mundo e na tela (HUD) e áudio.
//! Todos os sons e imagens são gerados no código, sem arquivos externos.
//!
//! Controles:
//! - WASD ou setas: mover
//! - Shift: correr
//! - M: liga/desliga a música
//! - R: recomeçar
//! - Esc: sair
//!
//! Rode com: `cargo run --example coletor`

use nn_engine::audio::synth::{self, Tone, Wave};
use nn_engine::prelude::*;

const ARENA: Vec2 = Vec2::new(1600.0, 1000.0);
const TILE: f32 = 64.0;
const SPEED: f32 = 260.0;
const SPRINT: f32 = 1.7;
const COINS: usize = 20;
const PICKUP_RADIUS: f32 = 30.0;
const PLAYER_SIZE: f32 = 64.0;
const COIN_SIZE: f32 = 40.0;

struct Coin {
    position: Vec2,
    anim: AnimationPlayer,
    collected: bool,
}

/// "+1" que sobe e some quando uma moeda é coletada.
struct Popup {
    position: Vec2,
    age: f32,
}

struct Sounds {
    coin: SoundId,
    fanfare: SoundId,
    music: SoundId,
}

struct Coletor {
    player_sheet: SpriteSheet,
    coin_sheet: SpriteSheet,
    idle: Animation,
    walk: Animation,
    player_anim: AnimationPlayer,
    previous: Vec2,
    current: Vec2,
    facing_left: bool,
    moving: bool,
    coins: Vec<Coin>,
    popups: Vec<Popup>,
    score: usize,
    sounds: Sounds,
    music: Option<Sound>,
    fps: f32,
    seed: u32,
}

impl Coletor {
    fn new(ctx: &mut Context) -> Self {
        let (size, pixels) = player_sheet_pixels();
        let texture = ctx
            .renderer
            .create_texture(size, &pixels, TextureOptions::PIXEL_ART);
        let player_sheet = ctx.renderer.sprite_sheet(texture, UVec2::splat(16));

        let (size, pixels) = coin_sheet_pixels();
        let texture = ctx
            .renderer
            .create_texture(size, &pixels, TextureOptions::PIXEL_ART);
        let coin_sheet = ctx.renderer.sprite_sheet(texture, UVec2::splat(16));

        let idle = Animation::new([0, 1], 2.0);
        let walk = Animation::new(4..8, 10.0);

        let sounds = Sounds {
            coin: ctx.audio.add_samples(synth::SAMPLE_RATE, &coin_sound()),
            fanfare: ctx.audio.add_samples(synth::SAMPLE_RATE, &fanfare_sound()),
            music: ctx.audio.add_samples(synth::SAMPLE_RATE, &music_loop()),
        };
        let music = Some(
            ctx.audio
                .play_with(sounds.music, PlayOptions::default().looped().volume(0.35)),
        );

        let mut game = Self {
            player_sheet,
            coin_sheet,
            player_anim: AnimationPlayer::new(idle.clone()),
            idle,
            walk,
            previous: Vec2::ZERO,
            current: Vec2::ZERO,
            facing_left: false,
            moving: false,
            coins: Vec::new(),
            popups: Vec::new(),
            score: 0,
            sounds,
            music,
            fps: 60.0,
            seed: 2026,
        };
        game.reset();
        game
    }

    fn reset(&mut self) {
        self.previous = Vec2::ZERO;
        self.current = Vec2::ZERO;
        self.score = 0;
        self.popups.clear();
        self.coins.clear();
        // Moedas pulam em ping-pong pelos quadros 0..4 (girando).
        let spin = Animation::new(0..4, 9.0).ping_pong();
        while self.coins.len() < COINS {
            let half = ARENA / 2.0 - Vec2::splat(TILE);
            let p = Vec2::new(
                (self.random() * 2.0 - 1.0) * half.x,
                (self.random() * 2.0 - 1.0) * half.y,
            );
            let longe_do_inicio = p.length() > 150.0;
            let longe_das_outras = self.coins.iter().all(|c| c.position.distance(p) > 90.0);
            if longe_do_inicio && longe_das_outras {
                let mut anim = AnimationPlayer::new(spin.clone());
                // Cada moeda começa num ponto diferente do giro.
                anim.update(self.random() * spin.duration());
                self.coins.push(Coin {
                    position: p,
                    anim,
                    collected: false,
                });
            }
        }
    }

    /// Número pseudoaleatório em [0, 1). Determinístico para o exemplo.
    fn random(&mut self) -> f32 {
        self.seed = self
            .seed
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        (self.seed >> 8) as f32 / (1u32 << 24) as f32
    }
}

impl Game for Coletor {
    fn fixed_update(&mut self, ctx: &mut Context) {
        let input = &ctx.input;
        let dir = (input.wasd()
            + input.direction(
                KeyCode::ArrowLeft,
                KeyCode::ArrowRight,
                KeyCode::ArrowDown,
                KeyCode::ArrowUp,
            ))
        .normalize_or_zero();
        let sprint = input.key_down(KeyCode::ShiftLeft) || input.key_down(KeyCode::ShiftRight);
        let speed = if sprint { SPEED * SPRINT } else { SPEED };

        self.previous = self.current;
        let limit = ARENA / 2.0 - Vec2::splat(PLAYER_SIZE / 2.0);
        self.current = (self.current + dir * speed * ctx.fixed_dt()).clamp(-limit, limit);
        self.moving = dir != Vec2::ZERO;
        if dir.x != 0.0 {
            self.facing_left = dir.x < 0.0;
        }
        self.player_anim.speed = if sprint { SPRINT } else { 1.0 };

        // Coleta.
        let camera_x = ctx.camera.position.x;
        let half_width = ctx.viewport().x as f32 / 2.0;
        let mut collected_now = Vec::new();
        for coin in self.coins.iter_mut().filter(|c| !c.collected) {
            if coin.position.distance(self.current) < PICKUP_RADIUS + COIN_SIZE / 2.0 {
                coin.collected = true;
                collected_now.push(coin.position);
            }
        }
        for position in collected_now {
            self.score += 1;
            self.popups.push(Popup { position, age: 0.0 });
            // Tom levemente diferente a cada moeda e som posicionado no estéreo.
            let pitch = 0.95 + self.random() * 0.15;
            let pan = ((position.x - camera_x) / half_width).clamp(-1.0, 1.0) * 0.7;
            ctx.audio.play_with(
                self.sounds.coin,
                PlayOptions::default().speed(pitch).pan(pan),
            );
            if self.score == COINS {
                ctx.audio.play(self.sounds.fanfare);
            }
        }
    }

    fn update(&mut self, ctx: &mut Context) {
        let dt = ctx.time.delta_secs();
        if dt > 0.0 {
            self.fps += (1.0 / dt - self.fps) * 0.05;
        }

        if ctx.input.key_pressed(KeyCode::Escape) {
            ctx.exit();
        }
        if ctx.input.key_pressed(KeyCode::KeyR) {
            self.reset();
        }
        if ctx.input.key_pressed(KeyCode::KeyM) {
            match self.music.take() {
                Some(mut music) => music.fade_out(std::time::Duration::from_millis(300)),
                None => {
                    self.music = Some(ctx.audio.play_with(
                        self.sounds.music,
                        PlayOptions::default().looped().volume(0.35),
                    ));
                }
            }
        }

        let anim = if self.moving { &self.walk } else { &self.idle };
        self.player_anim.play(anim);
        self.player_anim.update(dt);
        for coin in &mut self.coins {
            coin.anim.update(dt);
        }
        for popup in &mut self.popups {
            popup.age += dt;
        }
        self.popups.retain(|p| p.age < 0.8);
    }

    fn render(&mut self, ctx: &mut Context) {
        let pos = self.previous.lerp(self.current, ctx.alpha());
        let t = 1.0 - (-6.0 * ctx.time.delta_secs()).exp();
        ctx.camera.position = ctx.camera.position.lerp(pos, t);

        draw_arena(ctx);

        // Moedas: todas usam a mesma textura, então viram uma única chamada de desenho.
        for coin in self.coins.iter().filter(|c| !c.collected) {
            let bob = (ctx.time.elapsed_secs() * 3.0 + coin.position.x).sin() * 3.0;
            ctx.renderer.draw_rect(
                coin.position - Vec2::new(0.0, COIN_SIZE / 2.0 + 2.0),
                Vec2::new(COIN_SIZE * 0.6, 6.0),
                Color::BLACK.with_alpha(0.25),
            );
            ctx.renderer.draw(
                &self
                    .coin_sheet
                    .sprite(coin.anim.frame())
                    .at(coin.position + Vec2::new(0.0, bob))
                    .size(Vec2::splat(COIN_SIZE)),
            );
        }

        // Personagem.
        ctx.renderer.draw_rect(
            pos - Vec2::new(0.0, 30.0),
            Vec2::new(40.0, 8.0),
            Color::BLACK.with_alpha(0.3),
        );
        let flip = if self.facing_left { -1.0 } else { 1.0 };
        ctx.renderer.draw(
            &self
                .player_sheet
                .sprite(self.player_anim.frame())
                .at(pos)
                .size(Vec2::new(PLAYER_SIZE * flip, PLAYER_SIZE)),
        );

        // "+1" no mundo, subindo e sumindo.
        for popup in &self.popups {
            let k = popup.age / 0.8;
            ctx.renderer.draw_text(
                &Text::new("+1")
                    .at(popup.position + Vec2::new(0.0, 30.0 + 50.0 * k))
                    .size(26.0)
                    .color(Color::hex(0xFFE27A).with_alpha(1.0 - k * k))
                    .anchor(Anchor::Center),
            );
        }

        draw_hud(self, ctx);
    }
}

fn draw_arena(ctx: &mut Context) {
    let light = Color::hex(0x3B4A3F);
    let dark = Color::hex(0x334037);
    let half = ARENA / 2.0;
    let tiles = (ARENA / TILE).as_ivec2();
    for y in 0..tiles.y {
        for x in 0..tiles.x {
            let color = if (x + y) % 2 == 0 { light } else { dark };
            let center = -half + (Vec2::new(x as f32, y as f32) + 0.5) * TILE;
            ctx.renderer.draw_rect(center, Vec2::splat(TILE), color);
        }
    }
    // Bordas.
    let wall = Color::hex(0x5A4632);
    let t = 12.0;
    ctx.renderer.draw_rect(
        Vec2::new(0.0, half.y + t / 2.0),
        Vec2::new(ARENA.x + 2.0 * t, t),
        wall,
    );
    ctx.renderer.draw_rect(
        Vec2::new(0.0, -half.y - t / 2.0),
        Vec2::new(ARENA.x + 2.0 * t, t),
        wall,
    );
    ctx.renderer.draw_rect(
        Vec2::new(half.x + t / 2.0, 0.0),
        Vec2::new(t, ARENA.y),
        wall,
    );
    ctx.renderer.draw_rect(
        Vec2::new(-half.x - t / 2.0, 0.0),
        Vec2::new(t, ARENA.y),
        wall,
    );
}

/// Texto com sombra, no espaço da tela.
fn shadowed(ctx: &mut Context, text: Text<'_>) {
    let shadow = Text {
        position: text.position + Vec2::splat(2.0),
        color: Color::BLACK.with_alpha(0.6 * text.color.a),
        ..text
    };
    ctx.renderer.screen().draw_text(&shadow).draw_text(&text);
}

fn draw_hud(game: &Coletor, ctx: &mut Context) {
    let view = ctx.viewport().as_vec2();
    let margin = 20.0;

    // Painel do placar.
    let score = format!("Moedas: {} / {COINS}", game.score);
    let size = ctx.renderer.measure_text(&Text::new(&score).size(28.0));
    ctx.renderer.screen().draw_rect(
        Vec2::splat(margin - 10.0),
        size + Vec2::new(20.0, 12.0),
        Color::BLACK.with_alpha(0.35),
    );
    shadowed(
        ctx,
        Text::new(&score)
            .at(Vec2::new(margin, margin - 4.0))
            .size(28.0)
            .color(Color::hex(0xFFD24A)),
    );

    let fps = format!("{:.0} FPS", game.fps);
    shadowed(
        ctx,
        Text::new(&fps)
            .at(Vec2::new(view.x - margin, margin))
            .size(18.0)
            .color(Color::hex(0xB8C4BB))
            .anchor(Anchor::TopRight),
    );

    let music = if game.music.is_some() {
        "ligada"
    } else {
        "desligada"
    };
    let help = format!(
        "WASD/setas: andar   Shift: correr   M: música ({music})   R: recomeçar   Esc: sair"
    );
    shadowed(
        ctx,
        Text::new(&help)
            .at(Vec2::new(margin, view.y - margin))
            .size(16.0)
            .color(Color::hex(0xDDE4DE))
            .anchor(Anchor::BottomLeft),
    );

    if game.score == COINS {
        ctx.renderer
            .screen()
            .draw_rect(Vec2::ZERO, view, Color::BLACK.with_alpha(0.4));
        shadowed(
            ctx,
            Text::new("Você pegou todas!")
                .at(view / 2.0)
                .size(56.0)
                .color(Color::hex(0xFFD24A))
                .anchor(Anchor::BottomCenter),
        );
        shadowed(
            ctx,
            Text::new("Aperte R para jogar de novo")
                .at(view / 2.0 + Vec2::new(0.0, 12.0))
                .size(22.0)
                .anchor(Anchor::TopCenter),
        );
    }
}

// ----------------------------------------------------------------------
// Arte gerada no código
// ----------------------------------------------------------------------

/// Spritesheet do personagem: 4×2 quadros de 16×16.
/// Linha de cima: parado (0, 1). Linha de baixo: andando (4 a 7).
fn player_sheet_pixels() -> (UVec2, Vec<u8>) {
    const HEAD: [&str; 10] = [
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
    ];
    const BODY: [&str; 3] = ["....#bbbbbb#....", "...#bbbbbbbb#...", "...#bbbbbbbb#..."];
    const LEGS_STAND: [&str; 3] = ["....#bb##bb#....", "....#bb#.#bb#...", ".....##...##...."];
    const LEGS_A: [&str; 3] = ["...#bb#..#bb#...", "..#bb#....#bb#..", "..###......###.."];
    const LEGS_B: [&str; 3] = ["....#bbbbbb#....", ".....#bbbb#.....", ".....######....."];
    const LEGS_C: [&str; 3] = ["....#bb##bb#....", "...#bb#..#bb#...", "...##.....###..."];

    let frame = |legs: [&str; 3], lift: bool| -> Vec<String> {
        let mut rows: Vec<String> = HEAD
            .iter()
            .chain(&BODY)
            .chain(&legs)
            .map(|s| s.to_string())
            .collect();
        if lift {
            // Corpo um pixel acima: dá o "pulinho" do passo / respiração.
            rows.remove(0);
            rows.push("................".into());
        }
        rows
    };

    let frames = [
        frame(LEGS_STAND, false),
        frame(LEGS_STAND, true),
        frame(LEGS_STAND, false), // não usados
        frame(LEGS_STAND, false),
        frame(LEGS_A, false),
        frame(LEGS_B, true),
        frame(LEGS_C, false),
        frame(LEGS_B, true),
    ];

    let size = UVec2::new(64, 32);
    let mut pixels = vec![0u8; (size.x * size.y * 4) as usize];
    for (i, rows) in frames.iter().enumerate() {
        let ox = (i as u32 % 4) * 16;
        let oy = (i as u32 / 4) * 16;
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                let rgba = match ch {
                    '#' => [24, 20, 37, 255],
                    'o' => [255, 205, 117, 255],
                    'b' => [59, 93, 201, 255],
                    _ => continue,
                };
                let idx = (((oy + y as u32) * size.x + ox + x as u32) * 4) as usize;
                pixels[idx..idx + 4].copy_from_slice(&rgba);
            }
        }
    }
    (size, pixels)
}

/// Spritesheet da moeda girando: 4 quadros de 16×16, da frente até de lado.
fn coin_sheet_pixels() -> (UVec2, Vec<u8>) {
    let size = UVec2::new(64, 16);
    let mut pixels = vec![0u8; (size.x * size.y * 4) as usize];
    let half_widths = [7.0f32, 5.2, 3.2, 1.6];
    for (i, &hw) in half_widths.iter().enumerate() {
        for y in 0..16 {
            for x in 0..16 {
                let dx = (x as f32 + 0.5 - 8.0) / hw;
                let dy = (y as f32 + 0.5 - 8.0) / 7.5;
                let d = dx * dx + dy * dy;
                if d > 1.0 {
                    continue;
                }
                let inner = {
                    let ix = (x as f32 + 0.5 - 8.0) / (hw - 1.5).max(0.5);
                    let iy = (y as f32 + 0.5 - 8.0) / 6.0;
                    ix * ix + iy * iy <= 1.0
                };
                let shine = hw > 2.0 && x as f32 + 0.5 < 8.0 - hw * 0.3 && y < 7 && inner;
                let rgba = if !inner {
                    [178, 112, 24, 255]
                } else if shine {
                    [255, 246, 196, 255]
                } else {
                    [255, 200, 60, 255]
                };
                let idx = ((y * size.x + i as u32 * 16 + x) * 4) as usize;
                pixels[idx..idx + 4].copy_from_slice(&rgba);
            }
        }
    }
    (size, pixels)
}

// ----------------------------------------------------------------------
// Sons gerados no código
// ----------------------------------------------------------------------

fn coin_sound() -> Vec<f32> {
    synth::sequence([
        Tone::new(Wave::Square, 988.0)
            .duration(0.06)
            .volume(0.22)
            .envelope(0.002, 0.01)
            .render(),
        Tone::new(Wave::Square, 1319.0)
            .duration(0.22)
            .volume(0.22)
            .envelope(0.002, 0.18)
            .render(),
    ])
}

fn fanfare_sound() -> Vec<f32> {
    let note = |n: u8, d: f32| {
        Tone::new(Wave::Triangle, synth::midi_to_hz(n))
            .duration(d)
            .volume(0.4)
            .envelope(0.005, d * 0.5)
            .render()
    };
    synth::sequence([
        note(72, 0.12),
        note(76, 0.12),
        note(79, 0.12),
        note(84, 0.5),
    ])
}

/// Arpejo em lá menor, para tocar em loop.
fn music_loop() -> Vec<f32> {
    const NOTES: [u8; 16] = [
        57, 60, 64, 60, 55, 59, 62, 59, 53, 57, 60, 57, 52, 56, 59, 56,
    ];
    synth::sequence(NOTES.iter().map(|&n| {
        Tone::new(Wave::Triangle, synth::midi_to_hz(n))
            .duration(0.2)
            .volume(0.25)
            .envelope(0.01, 0.12)
            .render()
    }))
}

fn main() -> Result<(), AppError> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let config = AppConfig::new("nn-engine — coletor de moedas").clear_color(Color::hex(0x1A1F1C));
    nn_engine::run(config, Coletor::new)
}
