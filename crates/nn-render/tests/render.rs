//! Testes de renderização de verdade: desenham em uma textura fora da tela e
//! conferem os pixels.
//!
//! Sem GPU disponível os testes são pulados, a menos que a variável de
//! ambiente `NN_REQUIRE_GPU=1` esteja definida (como no CI), caso em que falham.

use glam::{UVec2, Vec2};
use nn_core::Color;
use nn_render::{Camera2D, GpuContext, OffscreenTarget, Renderer, Sprite, TextureOptions};

const SIZE: UVec2 = UVec2::new(64, 64);

struct Harness {
    gpu: GpuContext,
    renderer: Renderer,
    target: OffscreenTarget,
}

impl Harness {
    fn new() -> Option<Self> {
        match pollster::block_on(GpuContext::new_headless()) {
            Ok(gpu) => {
                let renderer = Renderer::new(&gpu, OffscreenTarget::FORMAT);
                let target = OffscreenTarget::new(&gpu, SIZE);
                Some(Self {
                    gpu,
                    renderer,
                    target,
                })
            }
            Err(e) => {
                if std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1") {
                    panic!("GPU obrigatória mas indisponível: {e}");
                }
                eprintln!("pulando teste: {e}");
                None
            }
        }
    }

    fn render(&mut self, camera: &Camera2D, clear: Color) -> Image {
        self.renderer
            .render(self.target.view(), SIZE, camera, Some(clear));
        Image(
            self.target
                .read_rgba(&self.gpu)
                .expect("leitura dos pixels"),
        )
    }
}

struct Image(Vec<u8>);

impl Image {
    fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * SIZE.x + x) * 4) as usize;
        self.0[i..i + 4].try_into().unwrap()
    }
}

fn assert_color(actual: [u8; 4], expected: [u8; 4], where_: &str) {
    let close = actual
        .iter()
        .zip(expected)
        .all(|(a, e)| (i16::from(*a) - i16::from(e)).abs() <= 2);
    assert!(close, "{where_}: esperado {expected:?}, obtido {actual:?}");
}

const RED: [u8; 4] = [255, 0, 0, 255];
const GREEN: [u8; 4] = [0, 255, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];
const BLACK: [u8; 4] = [0, 0, 0, 255];

/// Textura 2×2: vermelho, verde / azul, branco (de cima para baixo).
fn checker(h: &mut Harness) -> nn_render::TextureId {
    let pixels = [RED, GREEN, BLUE, WHITE].concat();
    h.renderer
        .create_texture(UVec2::new(2, 2), &pixels, TextureOptions::PIXEL_ART)
}

#[test]
fn limpa_com_a_cor_de_fundo() {
    let Some(mut h) = Harness::new() else { return };
    let img = h.render(&Camera2D::default(), Color::rgba8(10, 20, 30, 255));
    assert_color(img.pixel(0, 0), [10, 20, 30, 255], "canto");
    assert_color(img.pixel(40, 50), [10, 20, 30, 255], "meio");
}

#[test]
fn retangulo_solido_no_centro() {
    let Some(mut h) = Harness::new() else { return };
    h.renderer
        .draw_rect(Vec2::ZERO, Vec2::splat(32.0), Color::RED);
    let img = h.render(&Camera2D::default(), Color::BLACK);
    assert_color(img.pixel(32, 32), RED, "centro");
    assert_color(img.pixel(17, 17), RED, "dentro, perto da borda");
    assert_color(img.pixel(14, 14), BLACK, "fora");
    assert_color(img.pixel(50, 32), BLACK, "fora à direita");
}

#[test]
fn textura_aparece_na_orientacao_certa() {
    let Some(mut h) = Harness::new() else { return };
    let tex = checker(&mut h);
    h.renderer.draw(&Sprite::new(tex).size(Vec2::splat(64.0)));
    let img = h.render(&Camera2D::default(), Color::BLACK);
    assert_color(img.pixel(16, 16), RED, "superior esquerdo");
    assert_color(img.pixel(48, 16), GREEN, "superior direito");
    assert_color(img.pixel(16, 48), BLUE, "inferior esquerdo");
    assert_color(img.pixel(48, 48), WHITE, "inferior direito");
}

#[test]
fn rotacao_e_anti_horaria() {
    let Some(mut h) = Harness::new() else { return };
    let tex = checker(&mut h);
    h.renderer.draw(
        &Sprite::new(tex)
            .size(Vec2::splat(64.0))
            .rotation(std::f32::consts::FRAC_PI_2),
    );
    let img = h.render(&Camera2D::default(), Color::BLACK);
    // Girando 90° no sentido anti-horário, o canto superior esquerdo vai para baixo.
    assert_color(
        img.pixel(16, 48),
        RED,
        "vermelho foi para o inferior esquerdo",
    );
    assert_color(
        img.pixel(16, 16),
        GREEN,
        "verde foi para o superior esquerdo",
    );
}

#[test]
fn tamanho_padrao_vem_da_textura() {
    let Some(mut h) = Harness::new() else { return };
    let tex = checker(&mut h);
    // Sem tamanho explícito: 2×2 pixels no centro.
    h.renderer.draw(&Sprite::new(tex));
    let img = h.render(&Camera2D::default(), Color::BLACK);
    assert_color(img.pixel(31, 31), RED, "pixel superior esquerdo do sprite");
    assert_color(img.pixel(32, 32), WHITE, "pixel inferior direito do sprite");
    assert_color(img.pixel(29, 29), BLACK, "fora do sprite");
}

#[test]
fn ultimo_desenhado_fica_por_cima() {
    let Some(mut h) = Harness::new() else { return };
    h.renderer
        .draw_rect(Vec2::ZERO, Vec2::splat(40.0), Color::RED);
    h.renderer
        .draw_rect(Vec2::ZERO, Vec2::splat(20.0), Color::BLUE);
    let img = h.render(&Camera2D::default(), Color::BLACK);
    assert_color(img.pixel(32, 32), BLUE, "centro");
    assert_color(img.pixel(15, 32), RED, "anel externo");
}

#[test]
fn camera_desloca_a_cena() {
    let Some(mut h) = Harness::new() else { return };
    h.renderer
        .draw_rect(Vec2::ZERO, Vec2::splat(16.0), Color::GREEN);
    let camera = Camera2D::new(Vec2::new(16.0, 0.0));
    let img = h.render(&camera, Color::BLACK);
    // A câmera foi para a direita, então o objeto aparece à esquerda do centro.
    assert_color(img.pixel(16, 32), GREEN, "objeto deslocado");
    assert_color(img.pixel(32, 32), BLACK, "centro vazio");
}

#[test]
fn transparencia_mistura_com_o_fundo() {
    let Some(mut h) = Harness::new() else { return };
    h.renderer
        .draw_rect(Vec2::ZERO, Vec2::splat(64.0), Color::WHITE.with_alpha(0.5));
    let img = h.render(&Camera2D::default(), Color::BLACK);
    let [r, g, b, _] = img.pixel(32, 32);
    // Mistura em espaço linear: 50% de branco sobre preto vira ~188 em sRGB.
    assert!((180..=195).contains(&r) && r == g && g == b, "obtido {r}");
}

#[test]
fn agrupa_sprites_por_textura() {
    let Some(mut h) = Harness::new() else { return };
    let tex = checker(&mut h);
    let white = h.renderer.white_texture();
    for t in [tex, tex, white, white, tex] {
        h.renderer.draw(&Sprite::new(t));
    }
    h.render(&Camera2D::default(), Color::BLACK);
    let stats = h.renderer.last_stats();
    assert_eq!(stats.sprites, 5);
    assert_eq!(stats.draw_calls, 3);
}

#[test]
fn milhares_de_sprites_aumentam_o_buffer() {
    let Some(mut h) = Harness::new() else { return };
    for i in 0..5000 {
        let x = (i % 64) as f32 - 31.5;
        let y = (i / 64 % 64) as f32 - 31.5;
        h.renderer
            .draw_rect(Vec2::new(x, y), Vec2::ONE, Color::BLUE);
    }
    let img = h.render(&Camera2D::default(), Color::BLACK);
    assert_eq!(h.renderer.last_stats().draw_calls, 1);
    assert_color(img.pixel(0, 63), BLUE, "canto inferior esquerdo preenchido");
}

#[test]
fn carrega_png() {
    let Some(mut h) = Harness::new() else { return };
    let mut png = Vec::new();
    image::RgbaImage::from_pixel(3, 2, image::Rgba(GREEN))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let tex = h
        .renderer
        .load_texture_bytes(&png, TextureOptions::default())
        .unwrap();
    assert_eq!(h.renderer.texture_size(tex), UVec2::new(3, 2));
}

// ----------------------------------------------------------------------
// Sprint 2: espaço da tela, spritesheets e texto
// ----------------------------------------------------------------------

use nn_render::{Anchor, Text};

impl Image {
    /// Conta pixels "acesos" (diferentes do fundo preto) numa região.
    fn lit_in(&self, x0: u32, y0: u32, x1: u32, y1: u32) -> u32 {
        let mut n = 0;
        for y in y0..y1 {
            for x in x0..x1 {
                let [r, g, b, _] = self.pixel(x, y);
                if u32::from(r) + u32::from(g) + u32::from(b) > 60 {
                    n += 1;
                }
            }
        }
        n
    }
}

#[test]
fn tela_usa_canto_superior_esquerdo() {
    let Some(mut h) = Harness::new() else { return };
    h.renderer
        .screen()
        .draw_rect(Vec2::new(0.0, 0.0), Vec2::new(16.0, 8.0), Color::RED);
    let img = h.render(&Camera2D::default(), Color::BLACK);
    assert_color(img.pixel(0, 0), RED, "canto superior esquerdo");
    assert_color(img.pixel(15, 7), RED, "fim do retângulo");
    assert_color(img.pixel(16, 0), BLACK, "à direita");
    assert_color(img.pixel(0, 8), BLACK, "abaixo");
}

#[test]
fn tela_ignora_a_camera_e_fica_por_cima() {
    let Some(mut h) = Harness::new() else { return };
    let camera = Camera2D {
        position: Vec2::new(500.0, 500.0),
        zoom: 3.0,
        ..Default::default()
    };
    // O mundo inteiro coberto de azul...
    h.renderer
        .draw_rect(camera.position, Vec2::splat(1000.0), Color::BLUE);
    // ...e a tela, que deve ficar por cima mesmo sendo um retângulo pequeno.
    h.renderer
        .screen()
        .draw_rect(Vec2::new(32.0, 32.0), Vec2::splat(8.0), Color::GREEN);
    let img = h.render(&camera, Color::BLACK);
    assert_color(img.pixel(35, 35), GREEN, "HUD por cima");
    assert_color(img.pixel(10, 10), BLUE, "mundo ao redor");
}

#[test]
fn textura_na_tela_tem_a_mesma_orientacao() {
    let Some(mut h) = Harness::new() else { return };
    let tex = checker(&mut h);
    h.renderer.screen().draw(
        &Sprite::new(tex)
            .at(Vec2::ZERO)
            .origin(Vec2::ZERO)
            .size(Vec2::splat(64.0)),
    );
    let img = h.render(&Camera2D::default(), Color::BLACK);
    assert_color(img.pixel(16, 16), RED, "superior esquerdo");
    assert_color(img.pixel(48, 16), GREEN, "superior direito");
    assert_color(img.pixel(16, 48), BLUE, "inferior esquerdo");
    assert_color(img.pixel(48, 48), WHITE, "inferior direito");
}

#[test]
fn quadro_da_spritesheet() {
    let Some(mut h) = Harness::new() else { return };
    let tex = checker(&mut h);
    let sheet = h.renderer.sprite_sheet(tex, UVec2::ONE);
    assert_eq!(sheet.frame_count(), 4);
    // Quadro 2 = pixel inferior esquerdo da textura (azul).
    h.renderer.draw(&sheet.sprite(2).size(Vec2::splat(64.0)));
    let img = h.render(&Camera2D::default(), Color::BLACK);
    assert_color(img.pixel(32, 32), BLUE, "quadro 2");
}

#[test]
fn texto_na_tela_acende_pixels_onde_deveria() {
    let Some(mut h) = Harness::new() else { return };
    h.renderer
        .screen()
        .draw_text(&Text::new("HI").at(Vec2::new(4.0, 4.0)).size(24.0));
    let img = h.render(&Camera2D::default(), Color::BLACK);
    let dentro = img.lit_in(4, 4, 40, 32);
    assert!(
        dentro > 40,
        "texto deveria acender pixels, acendeu {dentro}"
    );
    assert_eq!(img.lit_in(0, 40, 64, 64), 0, "nada abaixo do texto");
    assert_eq!(img.lit_in(44, 0, 64, 64), 0, "nada à direita do texto");
    assert_eq!(h.renderer.glyph_atlas_pages(), 1);
}

#[test]
fn texto_respeita_a_ancora_e_a_cor() {
    let Some(mut h) = Harness::new() else { return };
    let text = Text::new("O")
        .at(Vec2::new(32.0, 32.0))
        .size(32.0)
        .color(Color::GREEN)
        .anchor(Anchor::Center);
    let tamanho = h.renderer.measure_text(&text);
    h.renderer.screen().draw_text(&text);
    let img = h.render(&Camera2D::default(), Color::BLACK);
    // O "O" centralizado é simétrico: metades esquerda e direita parecidas.
    let esquerda = img.lit_in(0, 0, 32, 64);
    let direita = img.lit_in(32, 0, 64, 64);
    assert!(esquerda > 20 && direita > 20);
    let diff = (esquerda as i32 - direita as i32).abs();
    assert!(diff < esquerda as i32 / 3, "esq {esquerda} dir {direita}");
    // Só verde.
    for y in 0..64 {
        for x in 0..64 {
            let [r, _, b, _] = img.pixel(x, y);
            assert!(r < 3 && b < 3);
        }
    }
    assert!(tamanho.x > 10.0 && tamanho.y > 30.0);
}

#[test]
fn texto_no_mundo_acompanha_a_camera() {
    let Some(mut h) = Harness::new() else { return };
    let text = Text::new("X").size(20.0).anchor(Anchor::Center);
    // Câmera deslocada: o texto na origem deve aparecer à esquerda do centro.
    h.renderer.draw_text(&text);
    let img = h.render(&Camera2D::new(Vec2::new(20.0, 0.0)), Color::BLACK);
    let esquerda = img.lit_in(0, 0, 24, 64);
    let direita = img.lit_in(24, 0, 64, 64);
    assert!(
        esquerda > 10 && direita == 0,
        "esq {esquerda} dir {direita}"
    );
}

#[test]
fn glifos_sao_reaproveitados_entre_frames() {
    let Some(mut h) = Harness::new() else { return };
    for _ in 0..3 {
        h.renderer
            .screen()
            .draw_text(&Text::new("abcabc").size(18.0));
        h.render(&Camera2D::default(), Color::BLACK);
    }
    // 6 letras, mas tudo numa página só e em uma chamada de desenho.
    assert_eq!(h.renderer.last_stats().sprites, 6);
    assert_eq!(h.renderer.last_stats().draw_calls, 1);
    assert_eq!(h.renderer.glyph_atlas_pages(), 1);
}
