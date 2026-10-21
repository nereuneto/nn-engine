//! Água, vento e halo no Lua e nas cenas: o Vento e o Halo do brilho do
//! ambiente, a Água e o Balança com o vento nas formas e modelos, as
//! partículas no vento e as funções `nn.d3.wind`/`nn.d3.bloom`, engine sem
//! janela.

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Context, Game, Headless, Scene};
use nn_script::{LuaScene, ScriptRuntime};

fn projeto(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-agua-vento-{}-{name}", std::process::id()));
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

fn cena(dir: &Path) -> Option<(Headless<Uma>, ScriptRuntime)> {
    let rt = ScriptRuntime::new(dir).unwrap();
    let rt2 = rt.clone();
    let config = AppConfig::default()
        .size(64, 64)
        .assets_dir(dir)
        .hot_reload(false)
        .clear_color(nn_core::Color::BLACK);
    match Headless::new(config, |ctx| {
        let mut s = rt.scene("cenas/c.cena");
        s.enter(ctx);
        Uma(s)
    }) {
        Ok(h) => Some((h, rt2)),
        Err(e) => {
            if std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1") {
                panic!("GPU obrigatória mas indisponível: {e}");
            }
            eprintln!("pulando teste: {e}");
            None
        }
    }
}

fn lua<T: mlua::FromLua>(rt: &ScriptRuntime, expr: &str) -> T {
    rt.lua().load(format!("return {expr}")).eval().unwrap()
}

fn exec(rt: &ScriptRuntime, code: &str) {
    rt.lua().load(code).exec().unwrap();
}

const OBSERVA: &str = r#"
local c = {}
function c:enter() C = self end
function c:draw()
  -- A água e o balanço também pelo Lua (como uma forma desenhada à mão).
  nn.d3.draw(nn.d3.plane(), { y = 0.2, scale = 3, material = "water", sway = 1 })
  if TROCA_VENTO then VENTO = nn.d3.wind(TROCA_VENTO) TROCA_VENTO = nil end
  if TROCA_HALO ~= nil then HALO = nn.d3.bloom(TROCA_HALO) TROCA_HALO = nil end
  if ERRADO then
    ERRO = select(2, pcall(nn.d3.draw, nn.d3.plane(), { material = "lava" }))
    ERRADO = nil
  end
end
return c
"#;

const CENA: &str = r##"return { format = 1, mode = "3d", gravity = -20, script = "cenas/obs.lua",
  environment = { wind = 2, wind_turn = 90, bloom = 0.5 },
  objects = {
    { id = 1, name = "Lago", scale_x = 10, scale_z = 10,
      components = { { type = "shape3d", shape = "plane", color = "#2E86C1D9", material = "agua" } } },
    { id = 2, name = "Árvore", x = 3,
      components = { { type = "shape3d", shape = "cone", color = "#2E7D32", sway = 1 } } },
    { id = 3, name = "Folhas", y = 1,
      components = { { type = "particles", rate = 20, speed_min = 0, speed_max = 0, wind = 0.5 } } }
  } }"##;

#[test]
fn o_vento_e_o_halo_do_ambiente_chegam_no_desenho() {
    let dir = projeto(
        "ambiente",
        &[("cenas/obs.lua", OBSERVA), ("cenas/c.cena", CENA)],
    );
    let Some((mut h, rt)) = cena(&dir) else {
        return;
    };
    h.run_frames(3);
    assert_eq!(h.game.0.error(), None);
    // Vento 2 soprando para +X (90°); o halo pela metade.
    let w = h.ctx.renderer.wind3d();
    assert!((w.strength - 2.0).abs() < 1e-4, "{w:?}");
    assert!(w.direction.x > 0.99 && w.direction.y.abs() < 1e-3, "{w:?}");
    let b = h.ctx.renderer.bloom3d();
    assert!(b.enabled && (b.strength - 0.5).abs() < 1e-4, "{b:?}");
    // As partículas são levadas pela metade do vento.
    assert_eq!(
        lua::<f64>(&rt, "C:find('Folhas'):component('particles').wind"),
        0.5
    );
    // Mudar o ambiente (uma noite) não leva o vento nem o halo da cena.
    exec(&rt, "C:environment('noite', 0)");
    h.run_frames(3);
    assert!((h.ctx.renderer.wind3d().strength - 2.0).abs() < 1e-4);
    assert!((h.ctx.renderer.bloom3d().strength - 0.5).abs() < 1e-4);
    assert_eq!(h.game.0.error(), None);
}

#[test]
fn nn_d3_wind_e_bloom_no_lua() {
    let dir = projeto(
        "lua",
        &[
            ("cenas/obs.lua", OBSERVA),
            (
                "cenas/c.cena",
                r#"return { format = 1, mode = "3d", script = "cenas/obs.lua", objects = {} }"#,
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir) else {
        return;
    };
    h.run_frames(2);
    // Sem vento no ambiente: parado. O halo vem ligado.
    assert_eq!(h.ctx.renderer.wind3d().strength, 0.0);
    assert!(h.ctx.renderer.bloom3d().enabled);
    // Uma ventania para -Z (0°).
    exec(&rt, "TROCA_VENTO = { strength = 3, turn = 0 }");
    h.run_frames(1);
    assert_eq!(lua::<f64>(&rt, "VENTO.strength"), 3.0);
    assert!(lua::<f64>(&rt, "VENTO.z") < -0.99);
    assert_eq!(h.ctx.renderer.wind3d().strength, 3.0);
    // Desliga e liga o halo; a força fica entre 0 e 4.
    exec(&rt, "TROCA_HALO = false");
    h.run_frames(1);
    assert!(!lua::<bool>(&rt, "HALO"));
    assert!(!h.ctx.renderer.bloom3d().enabled);
    exec(&rt, "TROCA_HALO = { enabled = true, strength = 9 }");
    h.run_frames(1);
    let b = h.ctx.renderer.bloom3d();
    assert!(b.enabled && b.strength == 4.0, "{b:?}");
    // Um material que não existe: o erro diz quais existem.
    exec(&rt, "ERRADO = true");
    h.run_frames(1);
    let erro: String = lua(&rt, "tostring(ERRO)");
    assert!(erro.contains("\"normal\" ou \"water\""), "{erro}");
    assert_eq!(h.game.0.error(), None);
}
