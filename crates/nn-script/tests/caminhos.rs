//! Caminhos: a grade da cena (nav), scene:path e os inimigos que contornam
//! paredes e buracos, no 2D e no 3D (engine sem janela).

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Context, Game, Headless, Scene};
use nn_script::{LuaScene, ScriptRuntime};

fn projeto(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-caminhos-{}-{name}", std::process::id()));
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
        .size(64, 64)
        .assets_dir(dir)
        .hot_reload(false)
        .clear_color(nn_core::Color::BLACK);
    match Headless::new(config, |ctx| {
        let mut s = rt.scene(path);
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

const OBSERVA: &str = r#"
local c = {}
function c:enter() C = self end
return c
"#;

/// Um chão de 30 × 30 com um muro de 16 m em z = 0 (de x = -8 a 8), um herói
/// parado atrás dele e um bicho na frente, que persegue (com ou sem
/// `contornar`). Do lado direito, longe do muro, um buraco no chão.
fn muro(contornar: bool) -> String {
    format!(
        r#"return {{ format = 1, mode = "3d", gravity = -20, script = "cenas/obs.lua",
  environment = {{ fog = false, shadows = false }},
  objects = {{
    {{ id = 1, name = "Chão", x = -3, scale_x = 24, scale_z = 30,
      components = {{ {{ type = "shape3d", shape = "plane" }}, {{ type = "body3d", body = "static" }} }} }},
    {{ id = 2, name = "Chão 2", x = 13, scale_x = 4, scale_z = 30,
      components = {{ {{ type = "shape3d", shape = "plane" }}, {{ type = "body3d", body = "static" }} }} }},
    {{ id = 3, name = "Muro", y = 1, scale_x = 16, scale_y = 2, scale_z = 0.4,
      components = {{ {{ type = "shape3d", shape = "cube" }}, {{ type = "body3d", body = "static" }} }} }},
    {{ id = 10, name = "Herói", y = 0.1, z = 5,
      components = {{ {{ type = "shape3d", shape = "capsule" }},
        {{ type = "behavior", script = "nn:jogador3d", prender_mouse = false, animar = false }} }} }},
    {{ id = 11, name = "Bicho", x = 2, y = 0.1, z = -5,
      components = {{ {{ type = "model", file = "modelos/bicho.glb" }},
        {{ type = "behavior", script = "nn:inimigo3d", andar = "parado", visao = 30, correr = 4,
           dano = 0, contornar = {contornar} }} }} }},
  }} }}"#
    )
}

/// Os modelos do exemplo estrelas3d (o bicho).
fn com_modelos(dir: &Path) {
    let origem = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jogos/estrelas3d/modelos");
    std::fs::create_dir_all(dir.join("modelos")).unwrap();
    for f in std::fs::read_dir(origem).unwrap() {
        let f = f.unwrap();
        std::fs::copy(f.path(), dir.join("modelos").join(f.file_name())).unwrap();
    }
}

fn pos(rt: &ScriptRuntime, nome: &str) -> (f64, f64, f64) {
    let v: Vec<f64> = lua(rt, &format!("{{ C.objects['{nome}']:position3d() }}"));
    (v[0], v[1], v[2])
}

#[test]
fn no_3d_o_inimigo_contorna_o_muro_e_nao_cai_no_buraco() {
    for contornar in [true, false] {
        let dir = projeto(
            &format!("muro-{contornar}"),
            &[
                ("cenas/obs.lua", OBSERVA),
                ("cenas/mundo.cena", &muro(contornar)),
            ],
        );
        com_modelos(&dir);
        let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
            return;
        };
        h.run_frames(5);
        assert_eq!(h.game.0.error(), None);
        if contornar {
            // A grade: o muro e o buraco (entre x = 9 e 11) não são andáveis.
            assert!(lua::<bool>(&rt, "C:nav():walkable(0, -3)"));
            assert!(!lua::<bool>(&rt, "C:nav():walkable(0, 0)"), "o muro");
            assert!(!lua::<bool>(&rt, "C:nav():walkable(10, -5)"), "o buraco");
            assert!(lua::<bool>(&rt, "C:nav().is_3d"));
            assert!(lua::<f64>(&rt, "C:nav():floor(0, -3)").abs() < 0.05);
            // scene:path dá a volta pela ponta do muro.
            let (n, completo): (usize, bool) = rt
                .lua()
                .load("local p, ok = C:path(C.objects['Bicho'], C.objects['Herói']) return #p, ok")
                .eval()
                .unwrap();
            assert!(completo);
            assert!(n >= 3, "{n} pontos");
            let maior_x: f64 = lua(
                &rt,
                "(function() local m = 0 for _, q in ipairs((C:path({x = 2, z = -5}, {x = 0, z = 5}))) do m = math.max(m, math.abs(q.x)) end return m end)()",
            );
            assert!(maior_x > 8.0, "passa pela ponta: {maior_x}");
        }
        // O bicho vai atrás do herói por 6 segundos.
        let mut menor_y = f64::MAX;
        for _ in 0..36 {
            h.run_frames(10);
            menor_y = menor_y.min(pos(&rt, "Bicho").1);
        }
        assert_eq!(h.game.0.error(), None);
        let (bx, _, bz) = pos(&rt, "Bicho");
        let (hx, _, hz) = pos(&rt, "Herói");
        let d = ((bx - hx).powi(2) + (bz - hz).powi(2)).sqrt();
        if contornar {
            assert!(d < 2.0, "chegou perto do herói: {d} ({bx}, {bz})");
            assert!(menor_y > -0.5, "não caiu no buraco: {menor_y}");
        } else {
            assert!(bz < 0.0, "sem contornar, fica preso no muro: ({bx}, {bz})");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[test]
fn no_2d_visto_de_cima_quem_persegue_contorna_a_parede() {
    let dir = projeto(
        "parede-2d",
        &[
            ("cenas/obs.lua", OBSERVA),
            (
                "cenas/fase.cena",
                r##"return { gravity = 0, script = "cenas/obs.lua", objects = {
  { id = 1, name = "parede", x = 0, y = 0, components = {
    { type = "shape", w = 400, h = 20, color = "#888888" },
    { type = "body", body = "static", w = 400, h = 20 },
  } },
  { id = 2, name = "heroi", x = 0, y = 120, components = {
    { type = "shape", w = 16, h = 16, color = "#FF0000" },
    { type = "body", body = "dynamic", w = 16, h = 16, gravity_scale = 0 },
    { type = "behavior", script = "nn:topdown", velocidade = 0 },
  } },
  { id = 3, name = "morcego", x = 30, y = -120, components = {
    { type = "shape", w = 16, h = 16, color = "#8844FF" },
    { type = "body", body = "dynamic", w = 16, h = 16, gravity_scale = 0 },
    { type = "behavior", script = "nn:perseguir", voar = true, velocidade = 200, visao = 0 },
  } },
  { id = 4, name = "camera", components = { { type = "camera" } } },
} }"##,
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/fase.cena") else {
        return;
    };
    h.run_frames(3);
    assert_eq!(h.game.0.error(), None);
    assert!(!lua::<bool>(&rt, "C:nav():walkable(0, 0)"), "a parede");
    assert!(lua::<bool>(&rt, "C:nav():walkable(0, -60)"));
    assert!(!lua::<bool>(&rt, "C:nav().is_3d"));
    h.run_frames(300);
    assert_eq!(h.game.0.error(), None);
    let p = |n: &str| -> (f64, f64) {
        let v: Vec<f64> = lua(&rt, &format!("{{ C.objects['{n}']:world_transform() }}"));
        (v[0], v[1])
    };
    let (mx, my) = p("morcego");
    let (hx, hy) = p("heroi");
    let d = ((mx - hx).powi(2) + (my - hy).powi(2)).sqrt();
    assert!(
        d < 40.0,
        "chegou: morcego em ({mx}, {my}), herói em ({hx}, {hy})"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn caminhos_a_mostra_e_grade_refeita_quando_um_corpo_parado_muda() {
    let dir = projeto(
        "mostrar",
        &[
            ("cenas/obs.lua", OBSERVA),
            ("cenas/mundo.cena", &muro(true)),
        ],
    );
    com_modelos(&dir);
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    h.run_frames(5);
    // Pedido pelo editor: a grade e o caminho do bicho são desenhados.
    nn_script::live::push(r#"{"cmd":"paths","on":true}"#);
    let antes = h.ctx.renderer.last_mesh_count();
    h.run_frames(30);
    assert_eq!(h.game.0.error(), None);
    assert!(
        h.ctx.renderer.last_mesh_count() > antes,
        "{antes} → {}",
        h.ctx.renderer.last_mesh_count()
    );
    nn_script::live::push(r#"{"cmd":"paths","on":false}"#);
    // O muro some: a grade é refeita e o lugar dele vira andável.
    assert!(!lua::<bool>(&rt, "C:nav():walkable(0, 0)"));
    rt.lua().load("C.objects['Muro']:destroy()").exec().unwrap();
    h.run_frames(2);
    assert!(lua::<bool>(&rt, "C:nav():walkable(0, 0)"));
    std::fs::remove_dir_all(&dir).unwrap();
}
