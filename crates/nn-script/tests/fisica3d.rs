//! `nn.physics3d` nos scripts: corpos, personagem, eventos, raios e erros.

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Context, Game, Headless, Scene};
use nn_script::{LuaScene, ScriptRuntime};

fn projeto(name: &str, main: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-fisica3d-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("modelos")).unwrap();
    std::fs::write(dir.join("main.lua"), main).unwrap();
    let origem = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jogos/cubos3d/modelos");
    for f in ["caixote.glb", "casa.glb"] {
        std::fs::copy(origem.join(f), dir.join("modelos").join(f)).unwrap();
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
fn corpos_caem_e_eventos_chegam() {
    let dir = projeto(
        "corpos",
        r##"
        local c = {}
        function c:enter()
          self.mundo = nn.physics3d.world({ gravity = -10 })
          self.chao = self.mundo:body({ type = "static", w = 20, h = 1, d = 20, y = -0.5, tag = "chao" })
          self.caixa = self.mundo:body({ x = 0, y = 4, z = 0, tag = "caixa" })
          self.bola = self.mundo:body({ shape = "sphere", radius = 0.5, x = 3, y = 2, z = 0, bounce = 0.5 })
          self.zona = self.mundo:body({ type = "static", sensor = true, w = 2, h = 1, d = 2, y = 2.5, tag = "zona" })
          self.entrou, self.saiu, self.tocou_chao = 0, 0, false
        end
        function c:fixed_update(dt)
          for _, e in ipairs(self.mundo:step(dt)) do
            if e.a == self.zona and e.b == self.caixa then
              if e.type == "enter" then self.entrou = self.entrou + 1 else self.saiu = self.saiu + 1 end
            end
            if e.type == "enter" and (e.a.tag == "chao" or e.b.tag == "chao") then self.tocou_chao = true end
          end
          self.y = self.caixa.y
          self.parada = self.caixa.sleeping
          self.n_caixas = #self.mundo:bodies("caixa")
          self.contagem = self.mundo.count
          local hit = self.mundo:raycast(0, 10, 0, 0, -1, 0)
          self.raio_y = hit and hit.y
          self.raio_e_caixa = hit and hit.body == self.caixa
          self.raio_ny = hit and hit.ny
          self.tocando = #self.caixa:touching()
        end
        return c
        "##,
    );
    let Some(mut h) = cena(&dir) else { return };
    h.run_frames(180);
    assert_eq!(h.game.0.error(), None);
    let f = |k: &str| h.game.0.field::<f64>(k).unwrap();
    assert!((f("y") - 0.5).abs() < 0.05, "y = {}", f("y"));
    assert_eq!(f("entrou"), 1.0);
    assert_eq!(f("saiu"), 1.0);
    assert_eq!(h.game.0.field::<bool>("tocou_chao"), Some(true));
    assert_eq!(f("n_caixas"), 1.0);
    assert_eq!(f("contagem"), 4.0);
    assert!(
        (f("raio_y") - 1.0).abs() < 0.05,
        "o raio para em cima da caixa"
    );
    assert_eq!(h.game.0.field::<bool>("raio_e_caixa"), Some(true));
    assert!((f("raio_ny") - 1.0).abs() < 1e-3);
    assert_eq!(f("tocando"), 1.0);
}

#[test]
fn personagem_anda_pula_e_empurra() {
    let dir = projeto(
        "personagem",
        r##"
        local c = {}
        function c:enter()
          self.mundo = nn.physics3d.world({ gravity = -20 })
          self.mundo:body({ type = "static", w = 40, h = 1, d = 40, y = -0.5 })
          self.heroi = self.mundo:character({ x = 0, y = 0.2, z = 0 })
          self.caixa = self.mundo:body({ model = nn.assets.model("modelos/caixote.glb"), x = 3, y = 0, z = 0 })
          self.t = 0
        end
        function c:fixed_update(dt)
          self.t = self.t + dt
          if self.t > 0.5 then self.heroi:walk(3, 0) end
          if self.t > 0.3 and not self.pulou and self.heroi.on_ground then
            self.pulou = self.heroi:jump(7)
            self.no_ar = not self.heroi.on_ground
          end
          self.mundo:step(dt)
          self.alto = math.max(self.alto or 0, self.heroi.y)
          self.x = self.heroi.x
          self.chao = self.heroi.on_ground
          self.caixa_x = self.caixa.x
          self.e_personagem = self.heroi.is_character
        end
        return c
        "##,
    );
    let Some(mut h) = cena(&dir) else { return };
    h.run_frames(150);
    assert_eq!(h.game.0.error(), None);
    let f = |k: &str| h.game.0.field::<f64>(k).unwrap();
    assert_eq!(h.game.0.field::<bool>("pulou"), Some(true));
    assert!(f("alto") > 1.0, "pulou {}", f("alto"));
    assert_eq!(h.game.0.field::<bool>("chao"), Some(true));
    assert_eq!(h.game.0.field::<bool>("e_personagem"), Some(true));
    // Andou e empurrou o caixote (que começa em x = 3).
    assert!(f("x") > 2.0, "x = {}", f("x"));
    assert!(f("caixa_x") > 3.3, "caixote em {}", f("caixa_x"));
}

#[test]
fn modelo_com_forma_exata_e_desenho_pelo_corpo() {
    let dir = projeto(
        "malha",
        r##"
        local c = {}
        function c:enter()
          self.mundo = nn.physics3d.world()
          self.casa = nn.assets.model("modelos/casa.glb")
          -- O telhado da casa (forma exata) segura a bola lá em cima.
          self.mundo:body({ type = "static", model = self.casa, shape = "mesh" })
          self.bola = self.mundo:body({ shape = "sphere", radius = 0.2, x = 0, y = 6, z = 0 })
          -- Um cubo que cai na frente da câmera e é desenhado pelo corpo.
          self.cubo = self.mundo:body({ size = 1, x = 0, y = 2, z = 20, gravity_scale = 0 })
          self.cubo:set_rotation(0, 45, 0)
          self.malha = nn.d3.cube()
          nn.d3.look_at(0, 2, 23, 0, 2, 20)
          nn.d3.light({ sun_color = "#000000", sky = "#FFFFFF", ground = "#FFFFFF" })
        end
        function c:fixed_update(dt)
          self.mundo:step(dt)
          self.bola_y = self.bola.y
          self.ry = self.cubo.ry
        end
        function c:draw()
          nn.d3.draw(self.malha, { body = self.cubo, color = "#FF0000" })
          self.mundo:debug_draw()
        end
        return c
        "##,
    );
    let Some(mut h) = cena(&dir) else { return };
    h.run_frames(120);
    assert_eq!(h.game.0.error(), None);
    let f = |k: &str| h.game.0.field::<f64>(k).unwrap();
    // A cumeeira do telhado fica em 2 × 1,5 = 3 (a chaminé é mais alta, mas fica de lado).
    assert!(
        f("bola_y") > 2.5 && f("bola_y") < 3.5,
        "bola em {}",
        f("bola_y")
    );
    assert!((f("ry") - 45.0).abs() < 0.1);
    let p = pixel(&h, 32, 32);
    assert!(
        p[0] > 200 && p[1] < 120,
        "o cubo vermelho (com o debug por cima): {p:?}"
    );
}

#[test]
fn erros_explicam_o_que_faltou() {
    for (code, esperado) in [
        (
            "local m = nn.physics3d.world(); m:body({ type = 'voador' })",
            "tipo de corpo inválido",
        ),
        (
            "local m = nn.physics3d.world(); m:body({ shape = 'cone' })",
            "forma inválida",
        ),
        (
            "local m = nn.physics3d.world(); m:body({ shape = 'mesh' })",
            "passe model",
        ),
        (
            "local m = nn.physics3d.world(); m:body({ model = nn.assets.model('modelos/casa.glb'), shape = 'mesh' })",
            "só vale para corpos static",
        ),
        (
            "local m = nn.physics3d.world(); m:body({}):walk(1, 0)",
            "só vale para personagens",
        ),
        (
            "local m = nn.physics3d.world(); local b = m:body({}); b:remove(); return b.x",
            "já foi removido",
        ),
        (
            "nn.d3.draw(nn.d3.cube(), { body = 3 })",
            "deve ser um corpo",
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
