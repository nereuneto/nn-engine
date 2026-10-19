//! O jogo `jogos/estrelas3d` (também o modelo "3d"): abre, anda, pega
//! estrela, cai no mar e volta, e as telas de título e fim funcionam.

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Context, Game, Headless, Scene};
use nn_script::{LuaScene, ScriptRuntime};

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

fn pasta() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jogos/estrelas3d")
}

fn cena(path: &str, w: u32, h: u32) -> Option<(Headless<Uma>, ScriptRuntime)> {
    let dir = pasta();
    let rt = ScriptRuntime::new(&dir).unwrap();
    let rt2 = rt.clone();
    let save = std::env::temp_dir().join(format!(
        "nn-estrelas-{}-{}",
        std::process::id(),
        path.replace('/', "-")
    ));
    let _ = std::fs::remove_dir_all(&save);
    let config = AppConfig::default()
        .size(w, h)
        .assets_dir(&dir)
        .save_dir(save)
        .hot_reload(false);
    match Headless::new(config, |ctx| {
        let mut s = rt.scene(path);
        s.enter(ctx);
        Uma(s)
    }) {
        Ok(h) => {
            rt2.lua()
                .globals()
                .set("cena", h.game.0.table().unwrap())
                .unwrap();
            Some((h, rt2))
        }
        Err(e) => {
            assert!(
                !std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1"),
                "GPU obrigatória mas indisponível: {e}"
            );
            None
        }
    }
}

fn lua<T: mlua::FromLua>(rt: &ScriptRuntime, code: &str) -> T {
    rt.lua().load(code).eval().unwrap()
}

#[test]
fn partida_anda_pega_estrela_e_volta_do_mar() {
    let Some((mut h, rt)) = cena("cenas/jogo.lua", 640, 360) else {
        return;
    };
    h.run_frames(30);
    assert_eq!(h.game.0.error(), None);
    assert!(h.ctx.mouse_captured(), "o jogo prende o mouse");
    assert!(lua::<bool>(&rt, "return cena.heroi.on_ground"));

    // Anda para a frente (para onde a câmera olha: -Z).
    let z0: f64 = lua(&rt, "return cena.heroi.z");
    h.ctx.input.press_key(nn_input::KeyCode::KeyW);
    h.run_frames(30);
    h.ctx.input.release_key(nn_input::KeyCode::KeyW);
    let z1: f64 = lua(&rt, "return cena.heroi.z");
    assert!(z1 < z0 - 1.5, "andou de {z0} para {z1}");

    // Pula.
    h.ctx.input.press_key(nn_input::KeyCode::Space);
    h.run_frames(2);
    h.ctx.input.release_key(nn_input::KeyCode::Space);
    h.run_frames(10);
    assert!(lua::<f64>(&rt, "return cena.heroi.y") > 0.5, "pulou");
    h.run_frames(60);

    // Põe o herói em cima da primeira estrela: ela é pega.
    let total: f64 = lua(&rt, "return cena.total");
    assert_eq!(total, 5.0);
    rt.lua()
        .load("cena.heroi:set_position(-6, 3.2, -7)")
        .exec()
        .unwrap();
    h.run_frames(20);
    assert_eq!(lua::<f64>(&rt, "return cena.estrelas"), 1.0);
    assert_eq!(lua::<f64>(&rt, "return cena.ilha:restam('estrela')"), 4.0);

    // Cai no mar: volta para a ilha.
    rt.lua()
        .load("cena.heroi:set_position(25, 0, 0)")
        .exec()
        .unwrap();
    h.run_frames(90);
    let x: f64 = lua(&rt, "return cena.heroi.x");
    let y: f64 = lua(&rt, "return cena.heroi.y");
    assert!(x.abs() < 14.0 && y > -1.0, "voltou para a ilha: {x}, {y}");
    assert_eq!(h.game.0.error(), None);

    // As plataformas que andam, andam.
    let y0: f64 = lua(&rt, "return cena.ilha.moveis[2].corpo.y");
    h.run_frames(60);
    let y1: f64 = lua(&rt, "return cena.ilha.moveis[2].corpo.y");
    assert!((y1 - y0).abs() > 0.5, "o elevador subiu: {y0} → {y1}");

    if let Ok(out) = std::env::var("NN_CAPTURA") {
        h.save_png(out).unwrap();
    }
}

#[test]
fn todas_as_estrelas_terminam_a_partida() {
    let Some((mut h, rt)) = cena("cenas/jogo.lua", 320, 180) else {
        return;
    };
    h.run_frames(5);
    for (x, y, z) in [
        (-6.0, 3.2, -7.0),
        (10.0, 4.5, -10.0),
        (10.5, 8.5, 1.5),
        (0.0, 8.0, 0.0),
        (-20.0, 3.0, 23.0),
    ] {
        rt.lua()
            .load(format!("cena.heroi:set_position({x}, {y}, {z})"))
            .exec()
            .unwrap();
        h.run_frames(15);
    }
    assert_eq!(lua::<f64>(&rt, "return cena.estrelas"), 5.0);
    assert!(lua::<bool>(&rt, "return cena.venceu ~= nil"));
    assert_eq!(h.game.0.error(), None);
}

#[test]
fn titulo_e_fim() {
    let Some((mut h, _)) = cena("cenas/titulo.lua", 320, 180) else {
        return;
    };
    h.run_frames(10);
    assert_eq!(h.game.0.error(), None);
    assert!(!h.ctx.mouse_captured());

    // O fim (sem argumentos: tempo 0) salva o recorde.
    let Some((mut h, _)) = cena("cenas/fim.lua", 320, 180) else {
        return;
    };
    h.run_frames(5);
    assert_eq!(h.game.0.error(), None);
    let recorde = h.ctx.save.get("recorde").and_then(|v| v.as_f64());
    assert_eq!(recorde, Some(0.0));
}
