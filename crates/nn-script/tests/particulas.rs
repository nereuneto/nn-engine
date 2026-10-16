//! Partículas das cenas visuais: contínuas, explosão que some sozinha,
//! soltar e desligar pelos eventos e efeitos em prefab.

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Context, Game, Headless, Scene};
use nn_input::KeyCode;
use nn_script::{LuaScene, ScriptRuntime};

fn projeto(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-particulas-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (path, content) in files {
        let full = dir.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }
    dir
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
        .size(320, 240)
        .assets_dir(dir)
        .save_dir(dir.join("dados"))
        .hot_reload(false);
    let h = match Headless::new(config, |ctx| {
        let mut s = rt.scene(path);
        s.enter(ctx);
        Uma(s)
    }) {
        Ok(h) => h,
        Err(e) => {
            assert!(
                !std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1"),
                "GPU obrigatória mas indisponível: {e}"
            );
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

fn apertar(h: &mut Headless<impl Game>, k: KeyCode) {
    h.ctx.input.press_key(k);
    h.run_frames(1);
    h.ctx.input.release_key(k);
    h.run_frames(1);
}

const CONFIG: &str = r#"return { title = "Fx", width = 320, height = 240, vars = { fim = 0 } }"#;

// Uma imagem de 2 × 2 pixels (para as partículas com imagem).
fn png() -> Vec<u8> {
    let mut out = Vec::new();
    let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 255, 255, 255]));
    img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .unwrap();
    out
}

const CENA: &str = r##"return { format = 1,
  events = {
    { when = { { cond = "particulas_terminaram", quem = "explosao" } },
      actions = { { act = "variavel", var = "fim", op = "somar", valor = 1 } } },
    { when = { { cond = "tecla", modo = "apertar", tecla = "x" } },
      actions = { { act = "soltar_particulas", alvo = "poeira", quantidade = 12 } } },
    { when = { { cond = "tecla", modo = "apertar", tecla = "d" } },
      actions = { { act = "emissor", acao = "desligar", alvo = "fogo" } } },
    { when = { { cond = "tecla", modo = "apertar", tecla = "c" } },
      actions = { { act = "criar", prefab = "fx.prefab", onde = "fogo" } } },
  },
  objects = {
    { id = 1, name = "fogo", components = { { type = "particles", rate = 60, lifetime = 0.5,
        area = "retangulo", area_w = 40, area_h = 10, color_mid = "#FF8800",
        image = "p.png", spin = 90, drag = 2 } } },
    { id = 2, name = "explosao", components = { { type = "particles", mode = "explosao",
        amount = 40, lifetime = 0.3, when_done = "destruir", area = "circulo" } } },
    { id = 3, name = "poeira", components = { { type = "particles", mode = "explosao",
        on_start = false, amount = 5, lifetime = 0.3 } } },
    { id = 4, name = "jato", x = 50, components = { { type = "particles", rate = 30,
        follow = true, lifetime = 2, speed_min = 0, speed_max = 0 } } },
  },
}"##;

#[test]
fn continuas_explosao_soltar_desligar_e_prefab() {
    let img = png();
    let dir = projeto("tudo", &[("config.lua", CONFIG), ("cena.cena", CENA)]);
    std::fs::write(dir.join("p.png"), &img).unwrap();
    std::fs::write(
        dir.join("fx.prefab"),
        r#"return { format = 1, object = { id = 1, name = "estouro", components = {
          { type = "particles", mode = "explosao", amount = 10, lifetime = 0.2, when_done = "destruir" } } } }"#,
    )
    .unwrap();
    let Some((mut h, rt)) = cena(&dir, "cena.cena") else {
        return;
    };
    h.run_frames(3);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(lua::<i64>(&rt, "cena.objects.explosao.particles.count"), 40);
    // A explosão solta tudo ao começar, termina, avisa e some.
    h.run_frames(30);
    assert_eq!(h.game.0.error(), None);
    assert!(lua::<i64>(&rt, "cena.objects.fogo.particles.count") > 10);
    assert!(lua::<bool>(&rt, "cena:find('explosao') == nil"));
    assert_eq!(lua::<i64>(&rt, "nn.vars.fim"), 1);
    // A poeira não solta sozinha nem "termina" sem ter soltado.
    assert_eq!(lua::<i64>(&rt, "cena.objects.poeira.particles.count"), 0);
    apertar(&mut h, KeyCode::KeyX);
    assert_eq!(lua::<i64>(&rt, "cena.objects.poeira.particles.count"), 12);
    // Um efeito em prefab: criado pelo evento, some sozinho.
    apertar(&mut h, KeyCode::KeyC);
    assert!(lua::<bool>(&rt, "cena:find('estouro') ~= nil"));
    h.run_frames(30);
    assert!(lua::<bool>(&rt, "cena:find('estouro') == nil"));
    // Desligar: as que saíram terminam a vida.
    apertar(&mut h, KeyCode::KeyD);
    h.run_frames(60);
    assert_eq!(lua::<i64>(&rt, "cena.objects.fogo.particles.count"), 0);
    assert!(lua::<bool>(&rt, "cena.objects.fogo.particles.finished"));
    // O jato acompanha o objeto: anda junto com ele.
    let antes: f64 = lua(&rt, "cena.objects.jato.particles.count");
    assert!(antes > 10.0);
    rt.lua().load("cena.objects.jato.x = 150").exec().unwrap();
    h.run_frames(1);
    assert_eq!(h.game.0.error(), None);
}
