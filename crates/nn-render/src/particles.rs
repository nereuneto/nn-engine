//! Partículas: fumaça, faíscas, poeira, explosões...
//!
//! Um [`ParticleEmitter`] solta partículas continuamente (`rate`) ou de uma
//! vez (`burst`). Cada partícula nasce com velocidade, tamanho e vida
//! sorteados dentro das faixas da [`ParticleConfig`], sofre gravidade e
//! arrasto, e muda de tamanho e de cor ao longo da vida. Todas são sprites
//! comuns, então saem num único lote por emissor.

use glam::Vec2;
use nn_core::Color;

use crate::{Blend, Rect, Renderer, Sprite, TextureId};

/// Região onde as partículas nascem, em volta da posição do emissor.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum EmitArea {
    /// Todas no mesmo ponto.
    #[default]
    Point,
    /// Num retângulo (metade do tamanho em cada eixo).
    Rect(Vec2),
    /// Num círculo.
    Circle(f32),
}

/// Como as partículas nascem e se comportam.
#[derive(Debug, Clone, PartialEq)]
pub struct ParticleConfig {
    /// Imagem de cada partícula. `None` usa um círculo suave.
    pub texture: Option<TextureId>,
    /// Região da imagem (para usar um quadro de spritesheet).
    pub region: Rect,
    /// Partículas por segundo enquanto o emissor estiver emitindo.
    pub rate: f32,
    /// Vida em segundos (mínimo, máximo).
    pub lifetime: (f32, f32),
    /// Velocidade inicial (mínimo, máximo).
    pub speed: (f32, f32),
    /// Direção central de lançamento, em radianos (0 = direita, π/2 = cima).
    pub direction: f32,
    /// Abertura total em volta da direção, em radianos (2π = todas as direções).
    pub spread: f32,
    pub gravity: Vec2,
    /// Perda de velocidade por segundo (0 = nenhuma; 3 = freia rápido).
    pub drag: f32,
    /// Tamanho no nascimento e no fim da vida.
    pub size: (f32, f32),
    /// Variação aleatória do tamanho (0.3 = até 30% maior ou menor).
    pub size_variation: f32,
    /// Cores ao longo da vida (a primeira no nascimento, a última no fim).
    pub colors: Vec<Color>,
    /// Velocidade de giro em radianos por segundo (mínimo, máximo).
    pub spin: (f32, f32),
    pub area: EmitArea,
    pub blend: Blend,
    /// Limite de partículas vivas.
    pub max: usize,
    /// As partículas acompanham o emissor quando ele anda (o jato de uma
    /// nave). Sem isso, ficam no mundo onde nasceram (um rastro).
    pub follow: bool,
}

impl Default for ParticleConfig {
    fn default() -> Self {
        Self {
            texture: None,
            region: Rect::FULL,
            rate: 0.0,
            lifetime: (0.5, 1.0),
            speed: (50.0, 150.0),
            direction: std::f32::consts::FRAC_PI_2,
            spread: std::f32::consts::TAU,
            gravity: Vec2::ZERO,
            drag: 0.0,
            size: (8.0, 0.0),
            size_variation: 0.0,
            colors: vec![Color::WHITE, Color::WHITE.with_alpha(0.0)],
            spin: (0.0, 0.0),
            area: EmitArea::Point,
            blend: Blend::Alpha,
            max: 1000,
            follow: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Particle {
    position: Vec2,
    velocity: Vec2,
    age: f32,
    life: f32,
    scale: f32,
    rotation: f32,
    spin: f32,
}

/// Emissor de partículas.
#[derive(Debug, Clone)]
pub struct ParticleEmitter {
    pub config: ParticleConfig,
    /// Onde as partículas nascem.
    pub position: Vec2,
    /// Emite continuamente (de acordo com `config.rate`).
    pub emitting: bool,
    particles: Vec<Particle>,
    carry: f32,
    seed: u32,
}

impl ParticleEmitter {
    pub fn new(config: ParticleConfig) -> Self {
        // Cada emissor sorteia diferente (duas tochas não ficam iguais).
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            config,
            position: Vec2::ZERO,
            emitting: true,
            particles: Vec::new(),
            carry: 0.0,
            seed: 1,
        }
        .with_seed(0x9E37_79B9 ^ n.wrapping_mul(0x85EB_CA6B))
    }

    /// Define a semente do sorteio (para resultados repetíveis).
    pub fn with_seed(mut self, seed: u32) -> Self {
        self.seed = seed.max(1);
        self
    }

    /// Partículas vivas.
    pub fn count(&self) -> usize {
        self.particles.len()
    }

    /// Não emite mais e não há partículas vivas.
    pub fn is_finished(&self) -> bool {
        !self.emitting && self.particles.is_empty()
    }

    /// Remove todas as partículas.
    pub fn clear(&mut self) {
        self.particles.clear();
        self.carry = 0.0;
    }

    /// Solta `n` partículas de uma vez, na posição do emissor.
    pub fn burst(&mut self, n: u32) {
        for _ in 0..n {
            self.spawn();
        }
    }

    /// Solta `n` partículas de uma vez em `position` (o emissor fica nela).
    pub fn burst_at(&mut self, n: u32, position: Vec2) {
        self.position = position;
        self.burst(n);
    }

    /// Avança `dt` segundos.
    pub fn update(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let c = &self.config;
        let damping = (1.0 - c.drag * dt).clamp(0.0, 1.0);
        let gravity = c.gravity;
        self.particles.retain_mut(|p| {
            p.age += dt;
            if p.age >= p.life {
                return false;
            }
            p.velocity = (p.velocity + gravity * dt) * damping;
            p.position += p.velocity * dt;
            p.rotation += p.spin * dt;
            true
        });

        if self.emitting && self.config.rate > 0.0 {
            self.carry += self.config.rate * dt;
            while self.carry >= 1.0 {
                self.carry -= 1.0;
                self.spawn();
            }
        }
    }

    fn random(&mut self) -> f32 {
        // xorshift32: rápido e suficiente para efeitos visuais.
        let mut x = self.seed;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.seed = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }

    fn range(&mut self, (min, max): (f32, f32)) -> f32 {
        min + (max - min) * self.random()
    }

    fn spawn(&mut self) {
        if self.particles.len() >= self.config.max {
            return;
        }
        let offset = match self.config.area {
            EmitArea::Point => Vec2::ZERO,
            EmitArea::Rect(half) => {
                Vec2::new(self.random() * 2.0 - 1.0, self.random() * 2.0 - 1.0) * half
            }
            EmitArea::Circle(radius) => {
                let a = self.random() * std::f32::consts::TAU;
                let r = radius * self.random().sqrt();
                Vec2::new(a.cos(), a.sin()) * r
            }
        };
        let angle = self.config.direction + (self.random() - 0.5) * self.config.spread;
        let speed = self.range(self.config.speed);
        let life = self.range(self.config.lifetime).max(0.001);
        let variation = self.config.size_variation;
        let scale = 1.0 + (self.random() * 2.0 - 1.0) * variation;
        let rotation = self.random() * std::f32::consts::TAU;
        let spin = self.range(self.config.spin);
        let origin = if self.config.follow {
            Vec2::ZERO
        } else {
            self.position
        };
        self.particles.push(Particle {
            position: origin + offset,
            velocity: Vec2::new(angle.cos(), angle.sin()) * speed,
            age: 0.0,
            life,
            scale,
            rotation,
            spin,
        });
    }

    fn color_at(&self, t: f32) -> Color {
        let colors = &self.config.colors;
        match colors.len() {
            0 => Color::WHITE,
            1 => colors[0],
            n => {
                let x = t.clamp(0.0, 1.0) * (n - 1) as f32;
                let i = (x.floor() as usize).min(n - 2);
                let k = x - i as f32;
                let (a, b) = (colors[i], colors[i + 1]);
                Color::rgba(
                    a.r + (b.r - a.r) * k,
                    a.g + (b.g - a.g) * k,
                    a.b + (b.b - a.b) * k,
                    a.a + (b.a - a.a) * k,
                )
            }
        }
    }

    fn sprites(&self, renderer: &Renderer) -> impl Iterator<Item = Sprite> + '_ {
        let texture = self
            .config
            .texture
            .unwrap_or_else(|| renderer.circle_texture());
        let origin = if self.config.follow {
            self.position
        } else {
            Vec2::ZERO
        };
        self.particles.iter().map(move |p| {
            let t = p.age / p.life;
            let size =
                (self.config.size.0 + (self.config.size.1 - self.config.size.0) * t) * p.scale;
            Sprite::new(texture)
                .at(origin + p.position)
                .size(Vec2::splat(size.max(0.0)))
                .rotation(p.rotation)
                .color(self.color_at(t))
                .region(self.config.region)
                .blend(self.config.blend)
        })
    }

    /// Desenha no espaço do mundo.
    pub fn draw(&self, renderer: &mut Renderer) {
        let sprites: Vec<Sprite> = self.sprites(renderer).collect();
        for s in &sprites {
            renderer.draw(s);
        }
    }

    /// Desenha no espaço da tela.
    pub fn draw_screen(&self, renderer: &mut Renderer) {
        let sprites: Vec<Sprite> = self.sprites(renderer).collect();
        let mut screen = renderer.screen();
        for s in &sprites {
            screen.draw(s);
        }
    }

    /// Posições das partículas vivas (útil em testes e efeitos próprios).
    pub fn positions(&self) -> impl Iterator<Item = Vec2> + '_ {
        let origin = if self.config.follow {
            self.position
        } else {
            Vec2::ZERO
        };
        self.particles.iter().map(move |p| origin + p.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ParticleConfig {
        ParticleConfig {
            lifetime: (1.0, 1.0),
            speed: (100.0, 100.0),
            ..Default::default()
        }
    }

    #[test]
    fn burst_e_fim_da_vida() {
        let mut e = ParticleEmitter::new(config());
        e.emitting = false;
        e.burst(25);
        assert_eq!(e.count(), 25);
        e.update(0.5);
        assert_eq!(e.count(), 25);
        e.update(0.6);
        assert_eq!(e.count(), 0);
        assert!(e.is_finished());
    }

    #[test]
    fn emissao_continua_respeita_a_taxa() {
        let mut e = ParticleEmitter::new(ParticleConfig {
            rate: 30.0,
            lifetime: (10.0, 10.0),
            ..config()
        });
        for _ in 0..60 {
            e.update(1.0 / 60.0);
        }
        assert!((29..=30).contains(&e.count()), "{}", e.count());
        e.emitting = false;
        e.update(1.0);
        assert!((29..=30).contains(&e.count()));
    }

    #[test]
    fn limite_de_particulas() {
        let mut e = ParticleEmitter::new(ParticleConfig {
            max: 10,
            ..config()
        });
        e.burst(50);
        assert_eq!(e.count(), 10);
    }

    #[test]
    fn direcao_e_gravidade() {
        // Para cima, sem abertura, com gravidade forte para baixo.
        let mut e = ParticleEmitter::new(ParticleConfig {
            direction: std::f32::consts::FRAC_PI_2,
            spread: 0.0,
            gravity: Vec2::new(0.0, -400.0),
            ..config()
        });
        e.burst(1);
        e.update(0.1);
        let p = e.positions().next().unwrap();
        assert!(p.x.abs() < 1e-3 && p.y > 0.0);
        for _ in 0..5 {
            e.update(0.1);
        }
        let depois = e.positions().next().unwrap();
        // Subiu e já começou a cair (velocidade 100 vs. gravidade 400).
        assert!(depois.y < 15.0, "{depois}");
    }

    #[test]
    fn area_de_nascimento() {
        let mut e = ParticleEmitter::new(ParticleConfig {
            area: EmitArea::Rect(Vec2::new(50.0, 5.0)),
            speed: (0.0, 0.0),
            ..config()
        })
        .with_seed(42);
        e.position = Vec2::new(100.0, 100.0);
        e.burst(200);
        for p in e.positions() {
            assert!((p.x - 100.0).abs() <= 50.0 && (p.y - 100.0).abs() <= 5.0);
        }
        let largura = e.positions().map(|p| p.x).fold(f32::MIN, f32::max)
            - e.positions().map(|p| p.x).fold(f32::MAX, f32::min);
        assert!(largura > 80.0, "espalhou pouco: {largura}");
    }

    #[test]
    fn acompanhar_o_emissor() {
        let mut e = ParticleEmitter::new(ParticleConfig {
            speed: (0.0, 0.0),
            follow: true,
            ..config()
        });
        e.burst(1);
        e.position = Vec2::new(30.0, -10.0);
        assert_eq!(e.positions().next(), Some(Vec2::new(30.0, -10.0)));
        // No mundo, a partícula fica onde nasceu.
        let mut w = ParticleEmitter::new(ParticleConfig {
            speed: (0.0, 0.0),
            ..config()
        });
        w.burst(1);
        w.position = Vec2::new(30.0, -10.0);
        assert_eq!(w.positions().next(), Some(Vec2::ZERO));
    }

    #[test]
    fn emissores_sorteiam_diferente() {
        let mut a = ParticleEmitter::new(config());
        let mut b = ParticleEmitter::new(config());
        a.burst(5);
        b.burst(5);
        a.update(0.1);
        b.update(0.1);
        let pa: Vec<Vec2> = a.positions().collect();
        let pb: Vec<Vec2> = b.positions().collect();
        assert_ne!(pa, pb);
    }

    #[test]
    fn gradiente_de_cor() {
        let e = ParticleEmitter::new(ParticleConfig {
            colors: vec![Color::RED, Color::GREEN, Color::BLUE],
            ..config()
        });
        assert_eq!(e.color_at(0.0), Color::RED);
        assert_eq!(e.color_at(0.5), Color::GREEN);
        assert_eq!(e.color_at(1.0), Color::BLUE);
        let q = e.color_at(0.25);
        assert!((q.r - 0.5).abs() < 1e-5 && (q.g - 0.5).abs() < 1e-5);
    }
}
