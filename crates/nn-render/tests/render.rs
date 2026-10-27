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

#[test]
fn circulo_redondo() {
    let Some(mut h) = Harness::new() else { return };
    h.renderer.draw_circle(Vec2::ZERO, 30.0, Color::RED);
    let img = h.render(&Camera2D::default(), Color::BLACK);
    assert_color(img.pixel(32, 32), RED, "centro");
    assert_color(img.pixel(32, 5), RED, "perto do topo");
    // Cantos do quadrado que envolve o círculo ficam vazios.
    assert_color(img.pixel(5, 5), BLACK, "canto");
    assert_color(img.pixel(58, 58), BLACK, "canto oposto");
}

#[test]
fn mistura_aditiva_soma_as_cores() {
    use nn_render::Blend;
    let Some(mut h) = Harness::new() else { return };
    let white = h.renderer.white_texture();
    // Fundo azul + vermelho aditivo = magenta; com alpha normal seria vermelho.
    h.renderer.draw(
        &Sprite::new(white)
            .at(Vec2::new(-16.0, 0.0))
            .size(Vec2::splat(32.0))
            .color(Color::RED)
            .blend(Blend::Add),
    );
    h.renderer.draw(
        &Sprite::new(white)
            .at(Vec2::new(16.0, 0.0))
            .size(Vec2::splat(32.0))
            .color(Color::RED),
    );
    let img = h.render(&Camera2D::default(), Color::BLUE);
    assert_color(img.pixel(16, 32), [255, 0, 255, 255], "aditivo");
    assert_color(img.pixel(48, 32), RED, "normal");
    // Mesma textura, mistura diferente: duas chamadas de desenho.
    assert_eq!(h.renderer.last_stats().draw_calls, 2);
}

#[test]
fn particulas_aparecem_na_tela() {
    use nn_render::{ParticleConfig, ParticleEmitter};
    let Some(mut h) = Harness::new() else { return };
    let mut fx = ParticleEmitter::new(ParticleConfig {
        lifetime: (1.0, 1.0),
        speed: (0.0, 0.0),
        size: (20.0, 20.0),
        colors: vec![Color::GREEN],
        ..Default::default()
    });
    fx.burst(3);
    fx.update(0.1);
    fx.draw(&mut h.renderer);
    let img = h.render(&Camera2D::default(), Color::BLACK);
    assert_color(img.pixel(32, 32), GREEN, "partícula no centro");
    assert_eq!(h.renderer.last_stats().draw_calls, 1);
}

#[test]
fn painel_arredondado_com_borda() {
    let Some(mut h) = Harness::new() else { return };
    // Painel de 48×48 no meio (a câmera mostra 64×64 com centro no 0, 0),
    // fundo verde, borda vermelha de 4 e cantos de raio 12.
    h.renderer.draw_panel(
        &nn_render::Panel::new(Vec2::ZERO, Vec2::splat(48.0))
            .color(Color::rgb(0.0, 1.0, 0.0))
            .radius(12.0)
            .border(4.0, Color::rgb(1.0, 0.0, 0.0)),
    );
    let img = h.render(&Camera2D::default(), Color::BLACK);
    assert_color(img.pixel(32, 32), GREEN, "meio");
    assert_color(img.pixel(32, 9), RED, "borda de cima");
    assert_color(img.pixel(54, 32), RED, "borda da direita");
    assert_color(img.pixel(9, 9), BLACK, "canto de fora (arredondado)");
    assert_color(img.pixel(2, 2), BLACK, "fora");
}

#[test]
fn painel_de_imagem_em_nove_fatias() {
    let Some(mut h) = Harness::new() else { return };
    // Imagem 3×3: borda azul, meio branco. Fatias de 1 pixel, cada uma com 8.
    let mut px = Vec::new();
    for y in 0..3 {
        for x in 0..3 {
            px.extend_from_slice(if x == 1 && y == 1 { &WHITE } else { &BLUE });
        }
    }
    let tex = h
        .renderer
        .create_texture(UVec2::splat(3), &px, TextureOptions::PIXEL_ART);
    h.renderer.screen().draw_panel(
        &nn_render::Panel::new(Vec2::splat(32.0), Vec2::new(56.0, 40.0)).image(
            nn_render::PanelImage {
                texture: tex,
                region: nn_render::Rect::FULL,
                slice: 1.0,
                scale: 8.0,
            },
        ),
    );
    let img = h.render(&Camera2D::default(), Color::BLACK);
    assert_color(img.pixel(32, 32), WHITE, "meio estica");
    assert_color(img.pixel(8, 32), BLUE, "borda da esquerda (8 px)");
    assert_color(img.pixel(14, 32), WHITE, "logo depois da borda");
    assert_color(img.pixel(32, 15), BLUE, "borda de cima");
    assert_color(img.pixel(2, 32), BLACK, "fora");
}

#[test]
fn textura_liberada_some_e_o_lugar_e_reaproveitado() {
    let Some(mut h) = Harness::new() else {
        return;
    };
    let antes = (h.renderer.texture_count(), h.renderer.texture_bytes());
    let a = h.renderer.create_texture(
        UVec2::new(4, 4),
        &[255; 4 * 4 * 4],
        TextureOptions::PIXEL_ART,
    );
    assert_eq!(h.renderer.texture_count(), antes.0 + 1);
    h.renderer.free_texture(a);
    h.renderer.free_texture(a);
    assert_eq!(
        (h.renderer.texture_count(), h.renderer.texture_bytes()),
        antes
    );
    // Desenhar o id liberado não quebra: não aparece nada.
    h.renderer
        .draw(&Sprite::new(a).at(Vec2::ZERO).size(Vec2::splat(64.0)));
    let img = h.render(&Camera2D::new(Vec2::ZERO), Color::BLACK);
    assert_color(img.pixel(32, 32), [0, 0, 0, 255], "centro");
    // A próxima textura usa o mesmo lugar.
    let b = h.renderer.create_texture(
        UVec2::new(2, 2),
        &[255; 2 * 2 * 4],
        TextureOptions::PIXEL_ART,
    );
    assert_eq!(a, b);
    // As da engine não são liberadas.
    let branco = h.renderer.white_texture();
    h.renderer.free_texture(branco);
    assert_eq!(h.renderer.texture_count(), antes.0 + 1);
}

#[test]
fn o_atlas_de_letras_reaproveita_as_paginas_menos_usadas() {
    let Some(mut h) = Harness::new() else {
        return;
    };
    // Muitos tamanhos de letra, um por quadro: sem limite, cada um ficaria
    // ocupando o atlas para sempre.
    for i in 0..80 {
        let text = nn_render::Text::new(
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ abcdefghijklmnopqrstuvwxyz 0123456789",
        )
        .at(Vec2::ZERO)
        .size(60.0 + i as f32 * 2.0);
        h.renderer.screen().draw_text(&text);
        h.render(&Camera2D::new(Vec2::ZERO), Color::BLACK);
    }
    let pages = h.renderer.glyph_atlas_pages();
    assert!(pages <= 4, "{pages} páginas");
    // Depois de reaproveitar páginas, as letras ainda aparecem.
    h.renderer.screen().draw_text(
        &nn_render::Text::new("█")
            .at(Vec2::new(8.0, 8.0))
            .size(40.0)
            .color(Color::WHITE),
    );
    let img = h.render(&Camera2D::new(Vec2::ZERO), Color::BLACK);
    let lit = (0..SIZE.y)
        .flat_map(|y| (0..SIZE.x).map(move |x| (x, y)))
        .filter(|&(x, y)| img.pixel(x, y)[0] > 128)
        .count();
    assert!(lit > 50, "{lit} pixels acesos");
}

/// Um xadrez de 1 pixel (256x256) desenhado em 37x37 na tela (uma escala
/// quebrada: sem mipmaps, cada pixel da tela cai num lugar do xadrez).
fn xadrez_reduzido(h: &mut Harness, options: TextureOptions) -> Vec<u8> {
    let n = 256u32;
    let pixels: Vec<u8> = (0..n * n)
        .flat_map(|i| {
            let v = if (i % n + i / n).is_multiple_of(2) {
                255
            } else {
                0
            };
            [v, v, v, 255]
        })
        .collect();
    let tex = h.renderer.create_texture(UVec2::splat(n), &pixels, options);
    h.renderer.screen().draw(
        &Sprite::new(tex)
            .at(Vec2::new(16.0, 16.0))
            .origin(Vec2::ZERO)
            .size(Vec2::splat(37.0)),
    );
    let img = h.render(&Camera2D::default(), Color::BLACK);
    let mut out = Vec::new();
    for y in 20..48 {
        for x in 20..48 {
            out.push(img.pixel(x, y)[0]);
        }
    }
    out
}

/// Com mipmaps, uma imagem desenhada bem menor fica lisa (um cinza parelho);
/// sem, ela serrilha (cada pixel pega um quadrado preto ou branco).
#[test]
fn mipmaps_deixam_a_imagem_reduzida_lisa() {
    let Some(mut h) = Harness::new() else {
        return;
    };
    let lisa = xadrez_reduzido(&mut h, TextureOptions::SMOOTH);
    let (min, max) = (lisa.iter().min().unwrap(), lisa.iter().max().unwrap());
    // Metade preto e metade branco, em cor linear: uns 188 em sRGB.
    assert!(max - min <= 6, "lisa: de {min} a {max}");
    assert!((175..=200).contains(min), "lisa: {min}");
    let serrilhada = xadrez_reduzido(&mut h, TextureOptions::default());
    let (min, max) = (
        serrilhada.iter().min().unwrap(),
        serrilhada.iter().max().unwrap(),
    );
    assert!(max - min > 100, "sem mipmaps: de {min} a {max}");
}

#[test]
fn mipmaps_contam_na_memoria_e_pixel_art_nao_tem() {
    let Some(mut h) = Harness::new() else {
        return;
    };
    let antes = h.renderer.texture_bytes();
    h.renderer.create_texture(
        UVec2::new(64, 32),
        &[0; 64 * 32 * 4],
        TextureOptions::SMOOTH,
    );
    // 64x32 + 32x16 + 16x8 + 8x4 + 4x2 + 2x1 + 1x1, 4 bytes cada.
    let niveis = (2048 + 512 + 128 + 32 + 8 + 2 + 1) * 4;
    assert_eq!(h.renderer.texture_bytes() - antes, niveis);
    let antes = h.renderer.texture_bytes();
    h.renderer.create_texture(
        UVec2::new(64, 32),
        &[0; 64 * 32 * 4],
        TextureOptions::PIXEL_ART,
    );
    assert_eq!(h.renderer.texture_bytes() - antes, 64 * 32 * 4);
}

/// O deslocamento anda a imagem repetida: metade vermelha e metade azul,
/// deslocada meia imagem, troca de lado.
#[test]
fn deslocar_a_imagem_repetida() {
    let Some(mut h) = Harness::new() else {
        return;
    };
    let pixels: Vec<u8> = (0..8 * 8)
        .flat_map(|i| {
            if i % 8 < 4 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 255]
            }
        })
        .collect();
    let tex = h
        .renderer
        .create_texture(UVec2::splat(8), &pixels, TextureOptions::PIXEL_ART);
    let sprite = Sprite::new(tex)
        .at(Vec2::ZERO)
        .origin(Vec2::ZERO)
        .size(Vec2::new(64.0, 32.0));
    h.renderer.screen().draw(&sprite);
    h.renderer
        .screen()
        .draw(&sprite.at(Vec2::new(0.0, 32.0)).offset(Vec2::new(0.5, 0.0)));
    let img = h.render(&Camera2D::default(), Color::BLACK);
    assert_color(
        img.pixel(8, 16),
        [255, 0, 0, 255],
        "sem deslocar, à esquerda",
    );
    assert_color(
        img.pixel(56, 16),
        [0, 0, 255, 255],
        "sem deslocar, à direita",
    );
    assert_color(img.pixel(8, 48), [0, 0, 255, 255], "deslocada, à esquerda");
    assert_color(img.pixel(56, 48), [255, 0, 0, 255], "deslocada, à direita");
}
