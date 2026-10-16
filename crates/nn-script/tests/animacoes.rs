//! Animações com nome (arquivo .anim ao lado da imagem) e imagens que se
//! repetem, nas cenas visuais e em código.

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Context, Game, Headless, Scene};
use nn_script::{LuaScene, ScriptRuntime};

fn projeto(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-anim-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (path, content) in files {
        let full = dir.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }
    dir
}

/// Folha de `n` quadros de `w`×`h`, cada um de uma cor.
fn folha(dir: &Path, rel: &str, w: u32, h: u32, cores: &[[u8; 4]]) {
    let mut img = image::RgbaImage::new(w * cores.len() as u32, h);
    for (i, c) in cores.iter().enumerate() {
        for y in 0..h {
            for x in 0..w {
                img.put_pixel(i as u32 * w + x, y, image::Rgba(*c));
            }
        }
    }
    let full = dir.join(rel);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    img.save(full).unwrap();
}

struct Uma(LuaScene);
impl Game for Uma {
    fn fixed_update(&mut self, ctx: &mut Context) {
        self.0.fixed_update(ctx);
    }
    fn update(&mut self, ctx: &mut Context) {
        let _ = self.0.update(ctx);
    }
    fn render(&mut self, ctx: &mut Context) {
        self.0.render(ctx);
    }
}

fn cena(dir: &Path, path: &str) -> Option<(Headless<Uma>, ScriptRuntime)> {
    let rt = ScriptRuntime::new(dir).unwrap();
    let rt2 = rt.clone();
    let config = AppConfig::default()
        .size(64, 64)
        .assets_dir(dir)
        .hot_reload(false);
    let h = match Headless::new(config, |ctx| {
        let mut s = rt.scene(path);
        s.enter(ctx);
        Uma(s)
    }) {
        Ok(h) => h,
        Err(e) => {
            if std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1") {
                panic!("GPU obrigatória mas indisponível: {e}");
            }
            eprintln!("pulando teste: {e}");
            return None;
        }
    };
    rt2.lua()
        .globals()
        .set("cena", h.game.0.table().unwrap())
        .unwrap();
    Some((h, rt2))
}

fn lua<T: mlua::FromLua>(rt: &ScriptRuntime, expr: &str) -> T {
    rt.lua().load(format!("return {expr}")).eval().unwrap()
}

const RGBW: [[u8; 4]; 4] = [
    [255, 0, 0, 255],
    [0, 255, 0, 255],
    [0, 0, 255, 255],
    [255, 255, 255, 255],
];

const ANIM: &str = r#"return {
  format = 1, frame_w = 8, frame_h = 8,
  animations = {
    { name = "parado", frames = { 1 }, fps = 4 },
    { name = "andar", frames = { 2, 3 }, fps = 10 },
    { name = "golpe", frames = { 3, 4 }, fps = 10, mode = "once", times = { 0.1, 0.3 } },
  },
}"#;

#[test]
fn anim_ao_lado_da_imagem_toca_por_nome_e_avisa_o_fim() {
    let dir = projeto(
        "cena",
        &[
            ("img/heroi.anim", ANIM),
            (
                "cenas/fase.cena",
                r#"return {
  events = {
    { when = { { cond = "tecla", modo = "apertar", tecla = "x" } },
      actions = { { act = "animacao", nome = "golpe", alvo = "heroi" } } },
    { when = { { cond = "animacao_terminou", quem = "heroi", nome = "golpe" } },
      actions = { { act = "variavel", var = "golpes", op = "somar", valor = 1 },
                  { act = "animacao", nome = "parado", alvo = "heroi" } } },
  },
  objects = {
    { id = 1, name = "heroi", components = {
      { type = "sprite", image = "img/heroi.png", animation = "andar" } } },
  },
}"#,
            ),
        ],
    );
    folha(&dir, "img/heroi.png", 8, 8, &RGBW);
    let Some((mut h, rt)) = cena(&dir, "cenas/fase.cena") else {
        return;
    };
    // O tamanho do quadro veio do .anim.
    assert_eq!(lua::<u32>(&rt, "cena.objects.heroi.sprite.frame_w"), 8);
    h.run_frames(1);
    assert_eq!(lua::<u32>(&rt, "cena.objects.heroi.sprite.frame"), 2);
    // 60 quadros por segundo: 0,1 s depois, o segundo quadro da animação.
    h.run_frames(7);
    assert_eq!(lua::<u32>(&rt, "cena.objects.heroi.sprite.frame"), 3);
    let nomes: Vec<String> = lua(&rt, "cena.objects.heroi.sprite:animations()");
    assert_eq!(nomes, ["parado", "andar", "golpe"]);

    // Evento: tocar "golpe" (uma vez, 0,4 s) e saber quando terminou.
    h.ctx.input.press_key(nn_input::KeyCode::KeyX);
    h.run_frames(1);
    h.ctx.input.release_key(nn_input::KeyCode::KeyX);
    h.run_frames(1);
    assert_eq!(
        lua::<String>(&rt, "cena.objects.heroi.sprite.animation"),
        "golpe"
    );
    h.run_frames(12);
    assert_eq!(lua::<u32>(&rt, "cena.objects.heroi.sprite.frame"), 4);
    assert_eq!(lua::<Option<f64>>(&rt, "nn.vars.golpes"), None);
    h.run_frames(20);
    assert_eq!(lua::<f64>(&rt, "nn.vars.golpes"), 1.0, "terminou uma vez");
    assert_eq!(
        lua::<String>(&rt, "cena.objects.heroi.sprite.animation"),
        "parado"
    );
    h.run_frames(30);
    assert_eq!(lua::<f64>(&rt, "nn.vars.golpes"), 1.0);
    assert_eq!(lua::<u32>(&rt, "cena.objects.heroi.sprite.frame"), 1);

    // Pelo script: play, mudar o campo e o erro com os nomes que existem.
    rt.lua()
        .load(r#"cena.objects.heroi.sprite.animation = "andar""#)
        .exec()
        .unwrap();
    h.run_frames(1);
    assert_eq!(lua::<u32>(&rt, "cena.objects.heroi.sprite.frame"), 2);
    let e = rt
        .lua()
        .load(r#"cena.objects.heroi.sprite:play("voar")"#)
        .exec()
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("\"voar\" não existe") && e.contains("parado, andar, golpe"),
        "{e}"
    );
}

#[test]
fn animacao_automatica_usa_nomes_do_anim() {
    let dir = projeto(
        "auto",
        &[
            ("img/heroi.anim", ANIM),
            (
                "cenas/fase.cena",
                r#"return {
  gravity = 0,
  objects = {
    { id = 1, name = "heroi", components = {
      { type = "sprite", image = "img/heroi.png" },
      { type = "behavior", script = "nn:animacao", parado = "parado", andando = "andar" } } },
  },
}"#,
            ),
        ],
    );
    folha(&dir, "img/heroi.png", 8, 8, &RGBW);
    let Some((mut h, rt)) = cena(&dir, "cenas/fase.cena") else {
        return;
    };
    h.run_frames(2);
    assert_eq!(
        lua::<String>(&rt, "cena.objects.heroi.sprite.animation"),
        "parado"
    );
    assert_eq!(lua::<u32>(&rt, "cena.objects.heroi.sprite.frame"), 1);
    // Sem corpo, andando pela posição (visto de cima).
    rt.lua().load("cena.objects.heroi.vx = 100").exec().unwrap();
    h.run_frames(3);
    assert_eq!(
        lua::<String>(&rt, "cena.objects.heroi.sprite.animation"),
        "andar"
    );
    assert!([2, 3].contains(&lua::<u32>(&rt, "cena.objects.heroi.sprite.frame")));
}

#[test]
fn anim_errado_explica() {
    let dir = projeto(
        "erro",
        &[
            (
                "img/heroi.anim",
                r#"return { frame_w = 8, frame_h = 8, animations = { { name = "a", frames = { 1 }, mode = "vaivem" } } }"#,
            ),
            (
                "cenas/fase.cena",
                r#"return { objects = { { id = 1, name = "heroi", components = { { type = "sprite", image = "img/heroi.png" } } } } }"#,
            ),
        ],
    );
    folha(&dir, "img/heroi.png", 8, 8, &RGBW);
    let Some((h, _rt)) = cena(&dir, "cenas/fase.cena") else {
        return;
    };
    let msg = h.game.0.error().expect("deveria haver erro").to_string();
    assert!(
        msg.contains("img/heroi.anim") && msg.contains("\"vaivem\" não existe"),
        "{msg}"
    );
}

#[test]
fn animacoes_em_codigo_com_nn_assets_animations() {
    let dir = projeto(
        "codigo",
        &[
            ("img/heroi.anim", ANIM),
            (
                "main.lua",
                r#"
local c = {}
function c:enter()
  self.anims = nn.assets.animations("img/heroi.png")
  self.anim = nn.animator(self.anims.golpe)
  duracao = self.anims.golpe.duration
  modo = self.anims.golpe.mode
end
function c:update(dt)
  self.anim:update(dt)
  quadro, fim = self.anim:frame(), self.anim:finished()
end
return c
"#,
            ),
        ],
    );
    folha(&dir, "img/heroi.png", 8, 8, &RGBW);
    let Some((mut h, rt)) = cena(&dir, "main.lua") else {
        return;
    };
    assert!((lua::<f64>(&rt, "duracao") - 0.4).abs() < 1e-6);
    assert_eq!(lua::<String>(&rt, "modo"), "once");
    h.run_frames(3);
    assert_eq!(lua::<u32>(&rt, "quadro"), 3);
    h.run_frames(12);
    assert_eq!(lua::<u32>(&rt, "quadro"), 4);
    assert!(!lua::<bool>(&rt, "fim"));
    h.run_frames(20);
    assert!(lua::<bool>(&rt, "fim"));
}

#[test]
fn imagem_que_repete_em_vez_de_esticar() {
    // Imagem 4×4: metade da esquerda vermelha, a da direita azul.
    let cena_txt = |fill: &str| {
        format!(
            r##"return {{ background = "#000000", objects = {{
  {{ id = 1, name = "chao", x = 0, y = 0, scale_x = 4, scale_y = 1, components = {{
    {{ type = "sprite", image = "img/bloco.png", w = 16, h = 16, fill = "{fill}" }} }} }},
}} }}"##
        )
    };
    let dir = projeto(
        "repetir",
        &[
            ("cenas/repetir.cena", &cena_txt("repeat")),
            ("cenas/esticar.cena", &cena_txt("stretch")),
        ],
    );
    folha(&dir, "img/bloco.png", 2, 4, &[[255, 0, 0, 255], [0, 0, 255, 255]]);
    for (path, esperado) in [
        // Repetir: 4 blocos de 16 px, cada um com 8 px vermelhos e 8 azuis.
        ("cenas/repetir.cena", ["r", "b", "r", "b", "r", "b", "r", "b"]),
        // Esticar: um bloco só, de 64 px.
        ("cenas/esticar.cena", ["r", "r", "r", "r", "b", "b", "b", "b"]),
    ] {
        let Some((mut h, _rt)) = cena(&dir, path) else {
            return;
        };
        h.run_frames(2);
        let px = h.pixels().unwrap();
        let cor = |x: usize| -> &str {
            let i = (32 * 64 + x) * 4;
            match px[i..i + 3] {
                [255, 0, 0] => "r",
                [0, 0, 255] => "b",
                _ => "?",
            }
        };
        let linha: Vec<&str> = (0..8).map(|i| cor(i * 8 + 4)).collect();
        assert_eq!(linha, esperado, "{path}");
    }
}
