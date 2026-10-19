//! `nn.d3` nos scripts: formas, câmera, luz, raio do mouse e erros.

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Context, Game, Headless, Scene};
use nn_script::{LuaScene, ScriptRuntime};

fn projeto(name: &str, main: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-d3-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("main.lua"), main).unwrap();
    dir
}

struct Uma(LuaScene);
impl Game for Uma {
    fn update(&mut self, ctx: &mut Context) {
        let _ = self.0.update(ctx);
    }
    fn render(&mut self, ctx: &mut Context) {
        self.0.render(ctx);
    }
}

fn cena(dir: &Path) -> Option<Headless<Uma>> {
    let rt = ScriptRuntime::new(dir).unwrap();
    let config = AppConfig::default()
        .size(64, 64)
        .assets_dir(dir)
        .hot_reload(false)
        .clear_color(nn_core::Color::BLACK);
    match Headless::new(config, |ctx| {
        let mut s = rt.scene("main.lua");
        s.enter(ctx);
        Uma(s)
    }) {
        Ok(h) => Some(h),
        Err(e) => {
            assert!(
                !std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1"),
                "GPU obrigatória mas indisponível: {e}"
            );
            None
        }
    }
}

fn pixel(h: &Headless<Uma>, x: u32, y: u32) -> [u8; 4] {
    let p = h.pixels().unwrap();
    let i = ((y * 64 + x) * 4) as usize;
    p[i..i + 4].try_into().unwrap()
}

#[test]
fn cubo_com_hud_por_cima() {
    let dir = projeto(
        "cubo",
        r##"
        local c = {}
        function c:enter()
          self.cubo = nn.d3.cube()
          nn.d3.look_at(0, 0, 3, 0, 0, 0)
          -- Só luz ambiente branca: a cor sai igual.
          nn.d3.light({ sun_color = "#000000", sky = "#FFFFFF", ground = "#FFFFFF" })
        end
        function c:draw()
          nn.d3.draw(self.cubo, { color = "#FF0000" })
          nn.screen.rect(28, 28, 8, 8, "#00FF00")
        end
        return c
        "##,
    );
    let Some(mut h) = cena(&dir) else { return };
    h.run_frames(2);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(pixel(&h, 32, 32), [0, 255, 0, 255], "HUD por cima");
    assert_eq!(pixel(&h, 24, 24), [255, 0, 0, 255], "cubo");
    assert_eq!(pixel(&h, 1, 1), [0, 0, 0, 255], "fundo");
}

#[test]
fn formas_camera_e_raio() {
    let dir = projeto(
        "formas",
        r##"
        local c = {}
        function c:enter()
          local a, b = nn.d3.cube(), nn.d3.cube()
          self.mesmo = a.triangles == 12 and b.triangles == 12
          self.esfera = nn.d3.sphere(1).triangles
          self.cilindro = nn.d3.cylinder(8).triangles
          self.largura = nn.d3.capsule().width
          self.altura = nn.d3.capsule().height
          local m = nn.d3.mesh({
            points = { {0, 0, 0}, {1, 0, 0}, {0, 1, 0}, {0, 0, 1} },
            triangles = { {1, 3, 2}, {1, 2, 4}, {1, 4, 3}, {2, 3, 4} },
            colors = { "#FF0000", "#00FF00", "#0000FF", "#FFFFFF" },
          })
          self.minha = m.triangles
          local cam = nn.d3.camera({ x = 1, y = 2, z = 3, target = { 0, 1, 0 }, fov = 45 })
          self.cam = cam.x + cam.y + cam.z + cam.target.y + cam.fov
          local o = nn.d3.orbit({ target = { 0, 0, 0 }, distance = 10, yaw = 90, pitch = 0 })
          self.orbita_x = o.x
          -- Olhando de cima: o meio da tela cai no chão logo abaixo.
          nn.d3.look_at(2, 10, 0.001, 2, 0, 0)
          self.gx, self.gy, self.gz = nn.d3.ground_point(32, 32)
          self.sx, self.sy = nn.d3.to_screen(2, 0, 0)
          self.atras = nn.d3.to_screen(2, 20, 0)
        end
        return c
        "##,
    );
    let Some(mut h) = cena(&dir) else { return };
    h.run_frames(1);
    assert_eq!(h.game.0.error(), None);
    let f = |k: &str| h.game.0.field::<f64>(k).unwrap();
    assert_eq!(h.game.0.field::<bool>("mesmo"), Some(true));
    assert_eq!(f("esfera"), 80.0);
    assert_eq!(f("cilindro"), 32.0);
    assert!((f("largura") - 0.5).abs() < 1e-4 && (f("altura") - 1.0).abs() < 1e-4);
    assert_eq!(f("minha"), 4.0);
    assert!((f("cam") - (1.0 + 2.0 + 3.0 + 1.0 + 45.0)).abs() < 1e-4);
    assert!((f("orbita_x") - 10.0).abs() < 1e-3);
    assert!((f("gx") - 2.0).abs() < 0.01, "gx {}", f("gx"));
    assert!(f("gy").abs() < 1e-4);
    assert!(f("gz").abs() < 0.01);
    assert!((f("sx") - 32.0).abs() < 0.1 && (f("sy") - 32.0).abs() < 0.1);
    assert_eq!(h.game.0.field::<f64>("atras"), None, "atrás da câmera");
}

#[test]
fn erros_explicam_o_que_faltou() {
    for (code, esperado) in [
        ("nn.d3.draw('cubo')", "deve ser uma malha"),
        (
            "nn.d3.mesh({ points = { {0,0,0} }, triangles = { {1, 2, 3} } })",
            "o ponto 2 não existe",
        ),
        ("nn.d3.mesh({ triangles = {} })", "faltou \"points\""),
        ("nn.d3.light({ fog = { near = 10, far = 5 } })", "maior que"),
        ("nn.d3.light({ sun = {0, 0, 0} })", "não pode ser"),
        (
            "nn.d3.draw(nn.d3.cube(), { image = 3 })",
            "deve ser uma imagem",
        ),
    ] {
        let dir = projeto(
            "erro",
            &format!("local c = {{}}\nfunction c:enter() {code} end\nreturn c\n"),
        );
        let Some(mut h) = cena(&dir) else { return };
        h.run_frames(1);
        let erro = h.game.0.error().map(|e| e.to_string()).unwrap_or_default();
        assert!(erro.contains(esperado), "{code}: {erro}");
    }
}

#[test]
fn nevoa_e_contagem() {
    let dir = projeto(
        "nevoa",
        r##"
        local c = {}
        function c:enter()
          nn.d3.look_at(0, 0, 40, 0, 0, 0)
          nn.d3.light({ sun_color = "#000000", sky = "#FFFFFF", ground = "#FFFFFF",
                        fog = { color = "#0000FF", near = 1, far = 5 } })
        end
        function c:update() self.n = nn.d3.count() end
        function c:draw()
          for i = 1, 3 do nn.d3.draw(nn.d3.cube(), { scale = 30, color = "#FF0000" }) end
        end
        return c
        "##,
    );
    let Some(mut h) = cena(&dir) else { return };
    h.run_frames(3);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(pixel(&h, 32, 32), [0, 0, 255, 255]);
    assert_eq!(h.game.0.field::<f64>("n"), Some(3.0));
}

/// O exemplo `jogos/cubos3d` abre, anda e solta cubos sem erro.
#[test]
fn exemplo_cubos3d_roda() {
    use nn_input::{KeyCode, MouseButton};
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jogos/cubos3d");
    let project = nn_script::Project::open(&dir).unwrap();
    let rt = ScriptRuntime::new(&dir).unwrap();
    let config = AppConfig::default()
        .size(640, 360)
        .assets_dir(&dir)
        .hot_reload(false)
        .clear_color(nn_core::Color::hex(0x9FD3E8));
    let Ok(mut h) = Headless::new(config, |ctx| {
        let mut s = rt.scene(&project.start);
        s.enter(ctx);
        Uma(s)
    }) else {
        assert!(!std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1"));
        return;
    };
    h.ctx.input.press_key(KeyCode::KeyW);
    h.run_frames(20);
    h.ctx.input.release_key(KeyCode::KeyW);
    h.ctx.input.set_cursor(Some(glam::Vec2::new(400.0, 260.0)));
    h.ctx.input.press_mouse(MouseButton::Left);
    h.run_frames(1);
    h.ctx.input.release_mouse(MouseButton::Left);
    h.run_frames(40);
    assert_eq!(h.game.0.error(), None);
    let t = h.game.0.table().unwrap();
    let jogador: mlua::Table = t.get("jogador").unwrap();
    let z: f64 = jogador.get("z").unwrap();
    assert!(z < -1.0, "W anda para a frente da câmera: z = {z}");
    let cubos: mlua::Table = t.get("cubos").unwrap();
    assert_eq!(cubos.raw_len(), 1, "o clique soltou um cubo");
    if let Ok(out) = std::env::var("NN_CAPTURA") {
        h.save_png(out).unwrap();
    }
}

/// Copia os modelos do exemplo para um projeto de teste.
fn com_modelos(dir: &Path) {
    let origem = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jogos/cubos3d/modelos");
    std::fs::create_dir_all(dir.join("modelos")).unwrap();
    for f in std::fs::read_dir(origem).unwrap() {
        let f = f.unwrap();
        std::fs::copy(f.path(), dir.join("modelos").join(f.file_name())).unwrap();
    }
}

#[test]
fn modelos_ceu_e_sombras_pelo_lua() {
    let dir = projeto(
        "modelos",
        r##"
        local c = {}
        function c:enter()
          self.caixote = nn.assets.model("modelos/caixote.glb")
          self.casa = nn.assets.model("modelos/casa.glb", { flat = true })
          self.tri = self.caixote.triangles
          self.altura = self.casa.height
          self.de_novo = nn.assets.model("modelos/caixote.glb").triangles
          nn.d3.look_at(0, 0.5, 2, 0, 0.5, 0)
          nn.d3.light({ sun_color = "#000000", sky = "#FFFFFF", ground = "#FFFFFF" })
          nn.d3.sky({ top = "#0000FF", horizon = "#0000FF", bottom = "#0000FF", sun_size = 0 })
          self.sombra = nn.d3.shadows(false)
          self.sombra2 = nn.d3.shadows({ distance = 10, quality = "baixa" })
        end
        function c:draw()
          nn.d3.draw(self.caixote, { shadow = false })
          nn.d3.draw(self.casa, { x = 100 })
        end
        return c
        "##,
    );
    com_modelos(&dir);
    let Some(mut h) = cena(&dir) else { return };
    h.run_frames(2);
    assert_eq!(h.game.0.error(), None);
    let f = |k: &str| h.game.0.field::<f64>(k).unwrap();
    assert_eq!(f("tri"), 12.0);
    assert_eq!(f("de_novo"), 12.0);
    assert!((f("altura") - 3.225).abs() < 0.01);
    assert_eq!(h.game.0.field::<bool>("sombra"), Some(false));
    assert_eq!(h.game.0.field::<bool>("sombra2"), Some(true));
    // O caixote no meio (madeira) e o céu azul nos cantos, no lugar do fundo preto.
    let meio = pixel(&h, 32, 32);
    assert!(
        meio[0] > meio[1] && meio[1] > meio[2] && meio[0] > 90,
        "madeira: {meio:?}"
    );
    assert_eq!(pixel(&h, 1, 1), [0, 0, 255, 255], "céu");
}

#[test]
fn modelo_alterado_recarrega() {
    let dir = projeto(
        "recarga",
        r##"
        local c = {}
        function c:enter() self.m = nn.assets.model("modelos/caixote.glb") end
        function c:draw() nn.d3.draw(self.m) end
        return c
        "##,
    );
    com_modelos(&dir);
    let rt = ScriptRuntime::new(&dir).unwrap();
    let config = AppConfig::default()
        .size(64, 64)
        .assets_dir(&dir)
        .hot_reload(true);
    let Ok(mut h) = Headless::new(config, |ctx| {
        let mut s = rt.scene("main.lua");
        s.enter(ctx);
        Uma(s)
    }) else {
        assert!(!std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1"));
        return;
    };
    h.run_frames(1);
    assert!(
        h.ctx
            .model(&nn_app::model_key("modelos/caixote.glb", false))
            .unwrap()
            .triangles
            == 12
    );
    // Troca o arquivo por outro modelo (a casa) com o jogo aberto.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::copy(
        dir.join("modelos/casa.glb"),
        dir.join("modelos/caixote.glb"),
    )
    .unwrap();
    for _ in 0..30 {
        h.run_frames(1);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let m = h
        .ctx
        .model(&nn_app::model_key("modelos/caixote.glb", false))
        .unwrap();
    assert!(m.triangles > 12, "recarregou: {} triângulos", m.triangles);
    assert_eq!(h.game.0.error(), None);
}
