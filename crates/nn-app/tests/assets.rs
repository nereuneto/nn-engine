//! Testes da pasta de assets e da recarga.

mod comum;

use std::time::Duration;

use nn_app::{AppConfig, AssetError, Context, Game};
use nn_render::{Sprite, TextureId, TextureOptions};

fn png(color: [u8; 4], w: u32, h: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::RgbaImage::from_pixel(w, h, image::Rgba(color))
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    bytes
}

/// Jogo que desenha uma textura cobrindo a tela toda.
struct Tela(TextureId);
impl Game for Tela {
    fn render(&mut self, ctx: &mut Context) {
        let size = ctx.viewport().as_vec2();
        ctx.renderer.draw(&Sprite::new(self.0).size(size));
    }
}

fn config(dir: &std::path::Path) -> AppConfig {
    AppConfig::default()
        .size(16, 16)
        .assets_dir(dir)
        .hot_reload(true)
}

#[test]
fn carrega_com_cache() {
    let dir = comum::temp_dir("cache");
    std::fs::write(dir.join("a.png"), png([255, 0, 0, 255], 2, 3)).unwrap();
    let Some(mut h) = comum::headless(config(&dir), |_| Nada) else {
        return;
    };
    let ctx = &mut h.ctx;
    let a = ctx
        .load_texture("a.png", TextureOptions::default())
        .unwrap();
    let b = ctx
        .load_texture("a.png", TextureOptions::default())
        .unwrap();
    assert_eq!(a, b);
    assert_eq!(ctx.renderer.texture_size(a).to_array(), [2, 3]);
    // Opções diferentes geram outra textura.
    let c = ctx
        .load_texture("a.png", TextureOptions::PIXEL_ART)
        .unwrap();
    assert_ne!(a, c);
}

#[test]
fn erro_claro_para_arquivo_que_nao_existe() {
    let dir = comum::temp_dir("faltando");
    let Some(mut h) = comum::headless(config(&dir), |_| Nada) else {
        return;
    };
    let err = h
        .ctx
        .load_texture("nao/existe.png", TextureOptions::default())
        .unwrap_err();
    assert!(matches!(err, AssetError::NotFound(_)));
    assert!(err.to_string().contains("existe.png"));
    assert!(h.ctx.load_sound("nada.ogg").is_err());
    assert!(h.ctx.load_font("nada.ttf").is_err());
}

#[test]
fn imagem_salva_e_recarregada_mantendo_o_id() {
    let dir = comum::temp_dir("recarga");
    let file = dir.join("cor.png");
    std::fs::write(&file, png([255, 0, 0, 255], 1, 1)).unwrap();

    let Some(mut h) = comum::headless(config(&dir), |ctx| {
        Tela(
            ctx.load_texture("cor.png", TextureOptions::default())
                .unwrap(),
        )
    }) else {
        return;
    };
    h.run_frames(1);
    assert_eq!(&h.pixels().unwrap()[..3], [255, 0, 0]);

    // Garante um horário de modificação diferente mesmo em sistemas de arquivos
    // com resolução de 1 s, e espera o intervalo de verificação.
    std::thread::sleep(Duration::from_millis(1100));
    std::fs::write(&file, png([0, 0, 255, 255], 4, 4)).unwrap();
    std::thread::sleep(nn_app::Assets::POLL_INTERVAL);
    h.run_frames(1);

    assert_eq!(&h.pixels().unwrap()[..3], [0, 0, 255]);
    assert_eq!(h.ctx.renderer.texture_size(h.game.0).to_array(), [4, 4]);
    assert_eq!(h.ctx.assets.reloaded().len(), 1);
}

#[test]
fn le_arquivos_de_texto() {
    let dir = comum::temp_dir("texto");
    std::fs::create_dir_all(dir.join("fases")).unwrap();
    std::fs::write(dir.join("fases/1.txt"), "olá\nmundo").unwrap();
    let Some(h) = comum::headless(config(&dir), |_| Nada) else {
        return;
    };
    assert_eq!(
        h.ctx.assets.read_to_string("fases/1.txt").unwrap(),
        "olá\nmundo"
    );
}

/// Jogo vazio, só para ter um contexto.
struct Nada;
impl Game for Nada {
    fn render(&mut self, _: &mut Context) {}
}
