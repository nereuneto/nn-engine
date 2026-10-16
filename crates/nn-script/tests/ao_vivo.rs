//! Editor ao vivo, no sentido jogo → editor: estado da cena, pausar e
//! avançar um quadro. (Arquivo próprio: a pausa vale para o processo todo.)

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Context, Game, Headless, Scene};
use nn_script::{LuaScene, ScriptRuntime, live};

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

fn projeto(files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-ao-vivo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (path, content) in files {
        let full = dir.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }
    dir
}

fn headless(dir: &Path, rt: ScriptRuntime) -> Option<Headless<Uma>> {
    let config = AppConfig::default()
        .size(64, 64)
        .assets_dir(dir)
        .hot_reload(false);
    match Headless::new(config, move |ctx| {
        let mut s = rt.scene("cenas/fase.cena");
        s.enter(ctx);
        Uma(s)
    }) {
        Ok(h) => Some(h),
        Err(e) => {
            assert!(
                !std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1"),
                "GPU obrigatória: {e}"
            );
            None
        }
    }
}

/// Guarda o que o jogo escreve para o editor.
struct Captura(std::sync::Mutex<Vec<String>>);
impl log::Log for Captura {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }
    fn log(&self, r: &log::Record) {
        if r.target() == live::LOG_TARGET {
            self.0.lock().unwrap().push(r.args().to_string());
        }
    }
    fn flush(&self) {}
}
static CAPTURA: Captura = Captura(std::sync::Mutex::new(Vec::new()));

fn ultimo_estado() -> serde_json::Value {
    let linhas = CAPTURA.0.lock().unwrap();
    let l = linhas
        .iter()
        .rev()
        .find_map(|l| l.strip_prefix("estado: "))
        .expect("nenhum estado");
    serde_json::from_str(l).unwrap()
}

#[test]
fn estado_da_cena_pausar_e_avancar() {
    log::set_logger(&CAPTURA).unwrap();
    log::set_max_level(log::LevelFilter::Info);
    let dir = projeto(&[
        ("config.lua", "return { vars = { moedas = 4 } }"),
        (
            "cenas/fase.cena",
            r##"return { objects = {
  { id = 3, name = "roda", x = 10, y = 20, components = {
    { type = "shape", w = 10, h = 10 },
    { type = "behavior", script = "nn:movimento", tipo = "girar", velocidade = 600 },
  } },
} }"##,
        ),
    ]);
    let rt = ScriptRuntime::new(&dir).unwrap();
    let Some(mut h) = headless(&dir, rt) else {
        return;
    };
    live::enable();
    h.run_frames(3);
    // O estado que vai para o editor.
    let json = ultimo_estado();
    assert_eq!(json["scene"], "cenas/fase.cena");
    assert_eq!(json["paused"], false);
    assert_eq!(json["vars"]["moedas"], 4);
    let roda = &json["objects"][0];
    assert_eq!(roda["id"], 3);
    assert_eq!(roda["x"], 10);
    assert_eq!(roda["props"]["2"]["velocidade"], 600);
    // Pausado: a roda para de girar; "avançar" anda um quadro.
    let rot = |h: &Headless<Uma>| -> f64 {
        h.game
            .0
            .table()
            .unwrap()
            .get::<mlua::Table>("objects")
            .unwrap()
            .get::<mlua::Table>("roda")
            .unwrap()
            .get("rotation")
            .unwrap()
    };
    assert!(live::push(r#"{"cmd":"pause"}"#));
    h.run_frames(2);
    let parada = rot(&h);
    std::thread::sleep(std::time::Duration::from_millis(120));
    h.run_frames(10);
    assert_eq!(rot(&h), parada, "pausado");
    assert_eq!(ultimo_estado()["paused"], true, "o editor sabe que pausou");
    assert!(live::push(r#"{"cmd":"step"}"#));
    h.run_frames(3);
    let passo = rot(&h);
    assert!(
        passo > parada && passo - parada < 30.0,
        "um passo: {parada} -> {passo}"
    );
    assert!(live::push(r#"{"cmd":"resume"}"#));
    h.run_frames(10);
    assert!(rot(&h) > passo + 50.0, "continuou");
    assert_eq!(h.game.0.error(), None);
}
