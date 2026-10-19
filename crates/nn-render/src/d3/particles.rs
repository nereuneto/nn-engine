//! Partículas 3D: fogo, fumaça, poeira, faíscas, chuva, neve, explosões.
//!
//! Como as do 2D ([`crate::ParticleEmitter`]), em metros: cada partícula é um
//! quadrado sempre virado para a câmera (*billboard*), que nasce numa área em
//! volta do emissor, sai num cone em volta da direção, sofre gravidade e
//! arrasto e muda de tamanho e cor ao longo da vida. Elas aparecem atrás e na
//! frente dos objetos certos e pegam a névoa, mas não fazem nem recebem
//! sombra.

use glam::Vec3;
use nn_core::Color;

use crate::{Blend, Rect, Renderer, TextureId};

/// Região onde as partículas nascem, em volta da posição do emissor.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum EmitArea3 {
    /// Todas no mesmo ponto.
    #[default]
    Point,
    /// Numa caixa (metade do tamanho em cada eixo).
    Box(Vec3),
    /// Numa esfera.
    Sphere(f32),
    /// Num disco deitado (no chão: chuva, poeira em volta).
    Disc(f32),
}

/// Como as partículas 3D nascem e se comportam (distâncias em metros).
#[derive(Debug, Clone, PartialEq)]
pub struct ParticleConfig3 {
    /// Imagem de cada partícula. `None` usa um círculo suave.
    pub texture: Option<TextureId>,
    /// Região da imagem (um quadro de spritesheet).
    pub region: Rect,
    /// Partículas por segundo enquanto o emissor estiver emitindo.
    pub rate: f32,
    /// Vida em segundos (mínimo, máximo).
    pub lifetime: (f32, f32),
    /// Velocidade inicial em m/s (mínimo, máximo).
    pub speed: (f32, f32),
    /// Direção central (não precisa ser unitária; zero = para cima).
    pub direction: Vec3,
    /// Abertura do cone em volta da direção, em graus (0 = um raio; 180 =
    /// meia esfera; 360 = todas as direções).
    pub spread: f32,
    pub gravity: Vec3,
    /// Perda de velocidade por segundo (0 = nenhuma; 3 = freia rápido).
    pub drag: f32,
    /// Tamanho (lado do quadrado, em metros) no nascimento e no fim.
    pub size: (f32, f32),
    /// Variação aleatória do tamanho (0.3 = até 30% maior ou menor).
    pub size_variation: f32,
    /// Cores ao longo da vida (a primeira no nascimento, a última no fim).
    pub colors: Vec<Color>,
    /// Giro em graus por segundo (mínimo, máximo).
    pub spin: (f32, f32),
    pub area: EmitArea3,
    pub blend: Blend,
    /// Limite de partículas vivas.
    pub max: usize,
    /// As partículas acompanham o emissor quando ele anda (o jato de um
    /// foguete). Sem isso, ficam no mundo onde nasceram (um rastro).
    pub follow: bool,
}

impl Default for ParticleConfig3 {
    fn default() -> Self {
        Self {
            texture: None,
            region: Rect::FULL,
            rate: 0.0,
            lifetime: (0.5, 1.0),
            speed: (1.0, 3.0),
            direction: Vec3::Y,
            spread: 360.0,
            gravity: Vec3::ZERO,
            drag: 0.0,
            size: (0.2, 0.0),
            size_variation: 0.0,
            colors: vec![Color::WHITE, Color::WHITE.with_alpha(0.0)],
            spin: (0.0, 0.0),
            area: EmitArea3::Point,
            blend: Blend::Alpha,
            max: 1000,
            follow: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Particle3 {
    position: Vec3,
    velocity: Vec3,
    age: f32,
    life: f32,
    scale: f32,
    rotation: f32,
    spin: f32,
}

/// Emissor de partículas 3D.
#[derive(Debug, Clone)]
pub struct ParticleEmitter3 {
    pub config: ParticleConfig3,
    /// Onde as partículas nascem.
    pub position: Vec3,
    /// Emite continuamente (de acordo com `config.rate`).
    pub emitting: bool,
    particles: Vec<Particle3>,
    carry: f32,
    seed: u32,
}

impl ParticleEmitter3 {
    pub fn new(config: ParticleConfig3) -> Self {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            config,
            position: Vec3::ZERO,
            emitting: true,
            particles: Vec::new(),
            carry: 0.0,
            seed: 1,
        }
        .with_seed(0x2545_F491 ^ n.wrapping_mul(0x85EB_CA6B))
    }

    /// Define a semente do sorteio (para resultados repetíveis).
    pub fn with_seed(mut self, seed: u32) -> Self {
        self.seed = seed.max(1);
        self
    }

    pub fn count(&self) -> usize {
        self.particles.len()
    }

    /// Não emite mais e não há partículas vivas.
    pub fn is_finished(&self) -> bool {
        !self.emitting && self.particles.is_empty()
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.carry = 0.0;
    }

    /// Solta `n` partículas de uma vez.
    pub fn burst(&mut self, n: u32) {
        for _ in 0..n {
            self.spawn();
        }
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

    /// Uma direção dentro do cone de abertura `spread` (graus) em volta de
    /// `dir`.
    fn cone(&mut self, dir: Vec3, spread: f32) -> Vec3 {
        let dir = dir.normalize_or(Vec3::Y);
        let half = (spread.clamp(0.0, 360.0) * 0.5).to_radians();
        // Uniforme na calota: cos do ângulo entre 1 e cos(meia abertura).
        let cos_max = half.cos();
        let cos_t = 1.0 - self.random() * (1.0 - cos_max);
        let sin_t = (1.0 - cos_t * cos_t).max(0.0).sqrt();
        let phi = self.random() * std::f32::consts::TAU;
        let (a, b) = dir.any_orthonormal_pair();
        (dir * cos_t + (a * phi.cos() + b * phi.sin()) * sin_t).normalize_or(dir)
    }

    fn spawn(&mut self) {
        if self.particles.len() >= self.config.max {
            return;
        }
        let offset = match self.config.area {
            EmitArea3::Point => Vec3::ZERO,
            EmitArea3::Box(half) => {
                Vec3::new(
                    self.random() * 2.0 - 1.0,
                    self.random() * 2.0 - 1.0,
                    self.random() * 2.0 - 1.0,
                ) * half
            }
            EmitArea3::Sphere(r) => {
                let d = self.cone(Vec3::Y, 360.0);
                d * r * self.random().cbrt()
            }
            EmitArea3::Disc(r) => {
                let a = self.random() * std::f32::consts::TAU;
                let k = r * self.random().sqrt();
                Vec3::new(a.cos() * k, 0.0, a.sin() * k)
            }
        };
        let dir = self.cone(self.config.direction, self.config.spread);
        let speed = self.range(self.config.speed);
        let life = self.range(self.config.lifetime).max(0.001);
        let scale = 1.0 + (self.random() * 2.0 - 1.0) * self.config.size_variation;
        let rotation = self.random() * 360.0;
        let spin = self.range(self.config.spin);
        let origin = if self.config.follow {
            Vec3::ZERO
        } else {
            self.position
        };
        self.particles.push(Particle3 {
            position: origin + offset,
            velocity: dir * speed,
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

    /// Posições das partículas vivas.
    pub fn positions(&self) -> impl Iterator<Item = Vec3> + '_ {
        let origin = if self.config.follow {
            self.position
        } else {
            Vec3::ZERO
        };
        self.particles.iter().map(move |p| origin + p.position)
    }

    /// Desenha no mundo 3D (vale no próximo `render`).
    pub fn draw(&self, renderer: &mut Renderer) {
        let texture = self
            .config
            .texture
            .unwrap_or_else(|| renderer.circle_texture());
        let origin = if self.config.follow {
            self.position
        } else {
            Vec3::ZERO
        };
        for p in &self.particles {
            let t = p.age / p.life;
            let size =
                (self.config.size.0 + (self.config.size.1 - self.config.size.0) * t) * p.scale;
            if size <= 0.0 {
                continue;
            }
            renderer.draw_billboard(
                texture,
                origin + p.position,
                size,
                p.rotation.to_radians(),
                self.color_at(t),
                self.config.region,
                self.config.blend,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ParticleConfig3 {
        ParticleConfig3 {
            lifetime: (1.0, 1.0),
            speed: (2.0, 2.0),
            ..Default::default()
        }
    }

    #[test]
    fn burst_taxa_e_fim() {
        let mut e = ParticleEmitter3::new(config());
        e.emitting = false;
        e.burst(20);
        e.update(0.5);
        assert_eq!(e.count(), 20);
        e.update(0.6);
        assert!(e.is_finished());
        let mut c = ParticleEmitter3::new(ParticleConfig3 {
            rate: 30.0,
            lifetime: (10.0, 10.0),
            ..config()
        });
        for _ in 0..60 {
            c.update(1.0 / 60.0);
        }
        assert!((29..=30).contains(&c.count()));
    }

    #[test]
    fn cone_de_direcao_e_gravidade() {
        // Para cima, abertura de 30°: todas sobem, nenhuma sai do cone.
        let mut e = ParticleEmitter3::new(ParticleConfig3 {
            spread: 30.0,
            ..config()
        })
        .with_seed(7);
        e.burst(200);
        e.update(0.1);
        let cos15 = 15f32.to_radians().cos();
        for p in e.positions() {
            let d = p.normalize();
            assert!(d.y >= cos15 - 1e-3, "fora do cone: {p}");
        }
        // Espalham para os lados (não é um raio só).
        let largura = e.positions().map(|p| p.x).fold(f32::MIN, f32::max)
            - e.positions().map(|p| p.x).fold(f32::MAX, f32::min);
        assert!(largura > 0.05, "{largura}");
        // Para o lado (+X), sem abertura, com gravidade: sobe nada e cai.
        let mut g = ParticleEmitter3::new(ParticleConfig3 {
            direction: Vec3::X,
            spread: 0.0,
            gravity: Vec3::new(0.0, -10.0, 0.0),
            ..config()
        });
        g.burst(1);
        for _ in 0..5 {
            g.update(0.1);
        }
        let p = g.positions().next().unwrap();
        assert!(p.x > 0.9 && p.y < -0.5 && p.z.abs() < 1e-4, "{p}");
    }

    #[test]
    fn areas_e_acompanhar() {
        let mut d = ParticleEmitter3::new(ParticleConfig3 {
            area: EmitArea3::Disc(2.0),
            speed: (0.0, 0.0),
            ..config()
        })
        .with_seed(3);
        d.position = Vec3::new(10.0, 1.0, 0.0);
        d.burst(100);
        for p in d.positions() {
            assert!((p.y - 1.0).abs() < 1e-5);
            assert!((p - Vec3::new(10.0, 1.0, 0.0)).length() <= 2.0 + 1e-4);
        }
        let mut s = ParticleEmitter3::new(ParticleConfig3 {
            area: EmitArea3::Sphere(1.0),
            speed: (0.0, 0.0),
            follow: true,
            ..config()
        });
        s.burst(50);
        s.position = Vec3::new(0.0, 5.0, 0.0);
        for p in s.positions() {
            assert!((p - Vec3::new(0.0, 5.0, 0.0)).length() <= 1.0 + 1e-4);
        }
    }
}
