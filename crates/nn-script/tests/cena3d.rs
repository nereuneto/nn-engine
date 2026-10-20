//! Cenas 3D do NN Studio rodando: Corpo 3D, Jogador 3D, comportamentos e
//! eventos em 3D (engine sem janela).

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Context, Game, Headless, Scene};
use nn_input::KeyCode;
use nn_script::{LuaScene, ScriptRuntime};

fn projeto(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-cena3d-{}-{name}", std::process::id()));
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

fn segurar(h: &mut Headless<Uma>, k: KeyCode, frames: u32) {
    h.ctx.input.press_key(k);
    h.run_frames(frames);
    h.ctx.input.release_key(k);
    h.run_frames(1);
}

/// Script da cena que deixa a cena numa global (`C`), para os testes lerem.
const OBSERVA: &str = r#"
local c = {}
function c:enter() C = self end
-- Mensagens "ao vivo" do editor (mandadas durante o quadro, como no jogo).
function c:update()
  for _, m in ipairs(AO_VIVO or {}) do self:_live(m) end
  AO_VIVO = nil
end
return c
"#;

const CHAO: &str = r#"{ id = 1, name = "Chão", scale_x = 40, scale_z = 40,
  components = { { type = "shape3d", shape = "plane" }, { type = "body3d", body = "static" } } }"#;

fn mundo(objetos: &str) -> String {
    format!(
        r#"return {{ format = 1, mode = "3d", gravity = -20, script = "cenas/obs.lua",
  environment = {{ fog = false, shadows = false }},
  objects = {{
    {CHAO},
    {objetos}
  }} }}"#
    )
}

#[test]
fn corpo_3d_cai_e_para_no_chao_com_a_forma_certa() {
    let dir = projeto(
        "corpo",
        &[
            ("cenas/obs.lua", OBSERVA),
            (
                "cenas/mundo.cena",
                &mundo(
                    r#"{ id = 2, name = "Caixa", y = 3, ry = 30,
      components = { { type = "shape3d", shape = "cube" }, { type = "body3d", body = "dynamic" } } },
    { id = 3, name = "Bola", x = 3, y = 4, scale_x = 2, scale_y = 2, scale_z = 2,
      components = { { type = "shape3d", shape = "sphere" }, { type = "body3d", body = "dynamic" } } },
    { id = 4, name = "Parede", x = -3, y = 1, scale_y = 2,
      components = { { type = "shape3d", shape = "cube" } } },"#,
                ),
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    h.run_frames(120);
    assert_eq!(h.game.0.error(), None);
    // Caixa de 1 m parada no chão (centro a 0,5), sem tombar; a bola de 2 m
    // (raio 1, pela escala) com o centro a 1 m.
    let y: f64 = lua(&rt, "C.objects.Caixa.y");
    assert!((y - 0.5).abs() < 0.05, "caixa no chão: {y}");
    let ry: f64 = lua(&rt, "C.objects.Caixa.ry");
    assert!((ry - 30.0).abs() < 1.0, "não girou à toa: {ry}");
    let by: f64 = lua(&rt, "C.objects.Bola.y");
    assert!((by - 1.0).abs() < 0.05, "bola de raio 1: {by}");
    // Sem Corpo 3D, nada cai.
    let py: f64 = lua(&rt, "C.objects.Parede.y");
    assert_eq!(py, 1.0);
    // O script muda a posição: o corpo vai junto.
    rt.lua()
        .load("C.objects.Caixa.x = 6; C.objects.Caixa.y = 5")
        .exec()
        .unwrap();
    h.run_frames(90);
    let (x, y): (f64, f64) = (lua(&rt, "C.objects.Caixa.x"), lua(&rt, "C.objects.Caixa.y"));
    assert!(
        (x - 6.0).abs() < 0.1 && (y - 0.5).abs() < 0.05,
        "caiu de novo em x = 6: {x}, {y}"
    );
}

const HEROI: &str = r##"{ id = 10, name = "Herói", y = 1, z = 0,
      components = { { type = "shape3d", shape = "capsule", color = "#FF8C42" },
        { type = "behavior", script = "nn:jogador3d", prender_mouse = false } } },"##;

#[test]
fn jogador_3d_anda_para_a_frente_da_camera_pula_e_volta_se_cair() {
    let dir = projeto(
        "jogador",
        &[
            ("cenas/obs.lua", OBSERVA),
            ("cenas/mundo.cena", &mundo(HEROI)),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    h.run_frames(30);
    assert_eq!(h.game.0.error(), None);
    // A cápsula tem a origem no meio: os pés (y - 0,5) ficam no chão.
    let y0: f64 = lua(&rt, "C.objects['Herói'].y");
    assert!((y0 - 0.5).abs() < 0.08, "em pé no chão: {y0}");
    // W anda para onde a câmera olha: ela começa atrás dele. Ele olha para
    // +Z (a frente dos modelos), então a câmera também.
    segurar(&mut h, KeyCode::KeyW, 60);
    let z: f64 = lua(&rt, "C.objects['Herói'].z");
    assert!(z > 4.0, "andou para +Z: {z}");
    let x: f64 = lua(&rt, "C.objects['Herói'].x");
    assert!(x.abs() < 0.2, "reto: {x}");
    // A (esquerda, olhando para +Z, é +X) vira o herói para 90°.
    segurar(&mut h, KeyCode::KeyA, 20);
    let ry: f64 = lua(&rt, "C.objects['Herói'].ry");
    assert!(((ry.rem_euclid(360.0)) - 90.0).abs() < 8.0, "virou: {ry}");
    // Pula.
    h.ctx.input.press_key(KeyCode::Space);
    h.run_frames(12);
    h.ctx.input.release_key(KeyCode::Space);
    let y: f64 = lua(&rt, "C.objects['Herói'].y");
    assert!(y > y0 + 0.8, "pulou: {y}");
    h.run_frames(60);
    // Caiu do mundo: volta ao último chão firme e recebe "caiu".
    rt.lua()
        .load(
            "C.objects['Herói']:move_to(100, -30, 0)
             caiu = false
             C.objects['Herói'].behaviors[1].caiu = function() caiu = true end",
        )
        .exec()
        .unwrap();
    h.run_frames(5);
    assert_eq!(h.game.0.error(), None);
    let (y, z): (f64, f64) = (
        lua(&rt, "C.objects['Herói'].y"),
        lua(&rt, "C.objects['Herói'].z"),
    );
    assert!(y > 0.0 && z > 4.0, "voltou para onde estava: y {y}, z {z}");
    assert!(lua::<bool>(&rt, "caiu"));
}

#[test]
fn plataforma_que_anda_leva_o_jogador_e_coletavel_soma() {
    let dir = projeto(
        "plataforma",
        &[
            ("cenas/obs.lua", OBSERVA),
            (
                "cenas/mundo.cena",
                &mundo(&format!(
                    r#"{HEROI}
    {{ id = 20, name = "Plataforma", y = 0.25, scale_x = 3, scale_y = 0.5, scale_z = 3,
      components = {{ {{ type = "shape3d", shape = "cube" }},
        {{ type = "body3d", body = "kinematic" }},
        {{ type = "behavior", script = "nn:plataforma3d", x = 6, duracao = 2, espera = 0 }} }} }},
    {{ id = 30, name = "Estrela", x = 3, y = 1.5, z = 0,
      components = {{ {{ type = "shape3d", shape = "sphere" }},
        {{ type = "behavior", script = "nn:coletavel", variavel = "estrelas" }},
        {{ type = "behavior", script = "nn:girar3d", velocidade = 90 }} }} }},"#
                )),
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    // O herói nasce em cima da plataforma (o topo dela fica a 0,5 m).
    rt.lua()
        .load("C.objects['Herói']:move_to(0, 1.1, 0)")
        .exec()
        .unwrap();
    h.run_frames(30);
    let ry0: f64 = lua(&rt, "C.objects.Estrela.ry");
    // Em 2 s a plataforma anda 6 m em X e leva o herói, que passa pela
    // estrela e pega.
    h.run_frames(120);
    assert_eq!(h.game.0.error(), None);
    let px: f64 = lua(&rt, "C.objects.Plataforma.x");
    let x: f64 = lua(&rt, "C.objects['Herói'].x");
    assert!(px > 5.0, "plataforma andou: {px}");
    assert!(
        (x - px).abs() < 0.5,
        "herói foi junto: herói {x}, plataforma {px}"
    );
    let estrelas: f64 = lua(&rt, "nn.vars.estrelas");
    assert_eq!(estrelas, 1.0);
    assert!(lua::<bool>(&rt, "C.objects.Estrela == nil"));
    assert!(ry0 > 10.0, "a estrela girava: {ry0}");
}

#[test]
fn eventos_em_3d_encostar_perto_e_empurrar() {
    let dir = projeto(
        "eventos",
        &[
            ("cenas/obs.lua", OBSERVA),
            (
                "cenas/mundo.cena",
                &format!(
                    r#"return {{ format = 1, mode = "3d", gravity = -20, script = "cenas/obs.lua",
  events = {{
    {{ when = {{ {{ cond = "encostar", quem = "Porta", em = "@jogador" }} }},
      actions = {{ {{ act = "variavel", var = "portas", op = "somar", valor = 1 }},
        {{ act = "mover", alvo = "Porta", modo = "somar", x = 0, y = 3, z = 0 }} }} }},
    {{ when = {{ {{ cond = "perto", quem = "Sino", distancia = 2, alvo = "@jogador" }} }},
      actions = {{ {{ act = "variavel", var = "sino", op = "definir", valor = 1 }} }} }},
    {{ when = {{ {{ cond = "tecla", modo = "apertar", tecla = "e" }} }},
      actions = {{ {{ act = "velocidade", modo = "definir", alvo = "Caixa", vx = 0, vy = 8, vz = 0 }} }} }},
  }},
  objects = {{
    {CHAO},
    {HEROI}
    {{ id = 40, name = "Porta", z = 3, y = 1, components = {{ {{ type = "shape3d", shape = "cube" }} }} }},
    {{ id = 41, name = "Sino", x = 1.5, y = 0.5, components = {{ {{ type = "shape3d", shape = "sphere" }} }} }},
    {{ id = 42, name = "Caixa", x = -4, y = 0.5,
      components = {{ {{ type = "shape3d", shape = "cube" }}, {{ type = "body3d", body = "dynamic" }} }} }},
  }} }}"#
                ),
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    h.run_frames(20);
    assert_eq!(h.game.0.error(), None);
    // O sino (a 1,5 m) está perto.
    assert_eq!(lua::<f64>(&rt, "nn.vars.sino or 0"), 1.0);
    // Andar até a porta: encosta uma vez e ela sobe.
    segurar(&mut h, KeyCode::KeyW, 40);
    assert_eq!(lua::<f64>(&rt, "nn.vars.portas or 0"), 1.0);
    let py: f64 = lua(&rt, "C.objects.Porta.y");
    assert_eq!(py, 4.0);
    // E: a caixa ganha velocidade para cima (física 3D).
    h.ctx.input.press_key(KeyCode::KeyE);
    h.run_frames(2);
    h.ctx.input.release_key(KeyCode::KeyE);
    h.run_frames(10);
    let cy: f64 = lua(&rt, "C.objects.Caixa.y");
    assert!(cy > 1.0, "a caixa subiu: {cy}");
}

#[test]
fn ambiente_e_corpo_mudam_ao_vivo() {
    let dir = projeto(
        "ao-vivo",
        &[
            ("cenas/obs.lua", OBSERVA),
            (
                "cenas/mundo.cena",
                &mundo(
                    r#"{ id = 2, name = "Caixa", y = 0.5,
      components = { { type = "shape3d", shape = "cube" }, { type = "body3d", body = "static" } } },"#,
                ),
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    h.run_frames(5);
    // O editor manda o Ambiente inteiro; o sol muda na hora.
    rt.lua()
        .load(
            "AO_VIVO = { { id = 0, field = 'environment', value = { sun_height = 10, fog = false } } }",
        )
        .exec()
        .unwrap();
    h.run_frames(1);
    assert_eq!(lua::<f64>(&rt, "C._environment.sun_height"), 10.0);
    // O corpo vira dinâmico e sobe: cai.
    rt.lua()
        .load(
            "AO_VIVO = { { id = 2, component = 2, field = 'body', value = 'dynamic' },
               { id = 2, field = 'y', value = 4 } }",
        )
        .exec()
        .unwrap();
    h.run_frames(90);
    assert_eq!(h.game.0.error(), None);
    let y: f64 = lua(&rt, "C.objects.Caixa.y");
    assert!((y - 0.5).abs() < 0.05, "caiu de volta: {y}");
}

fn com_modelos(dir: &Path) {
    let origem = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jogos/estrelas3d/modelos");
    std::fs::create_dir_all(dir.join("modelos")).unwrap();
    for f in std::fs::read_dir(origem).unwrap() {
        let f = f.unwrap();
        std::fs::write(
            dir.join("modelos").join(f.file_name()),
            std::fs::read(f.path()).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn modelos_animados_jogador_anima_sozinho_e_eventos_tocam() {
    let dir = projeto(
        "animados",
        &[
            ("cenas/obs.lua", OBSERVA),
            (
                "cenas/mundo.cena",
                &format!(
                    r#"return {{ format = 1, mode = "3d", gravity = -20, script = "cenas/obs.lua",
  events = {{
    {{ when = {{ {{ cond = "tecla", modo = "apertar", tecla = "e" }} }},
      actions = {{ {{ act = "animacao", alvo = "Bicho", nome = "parado", repetir = false }} }} }},
    {{ when = {{ {{ cond = "animacao_terminou", quem = "Bicho", nome = "parado" }} }},
      actions = {{ {{ act = "variavel", var = "fim", op = "somar", valor = 1 }} }} }},
  }},
  objects = {{
    {CHAO},
    {{ id = 10, name = "Herói", y = 0.1,
      components = {{ {{ type = "model", file = "modelos/heroi.glb" }},
        {{ type = "behavior", script = "nn:jogador3d", prender_mouse = false }} }} }},
    {{ id = 11, name = "Bicho", x = 4,
      components = {{ {{ type = "model", file = "modelos/bicho.glb" }},
        {{ type = "anim3d", animation = "andar", speed = 2 }} }} }},
  }} }}"#
                ),
            ),
        ],
    );
    com_modelos(&dir);
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    let anim = |rt: &ScriptRuntime, nome: &str| -> String {
        lua(rt, &format!("C.objects['{nome}']:animator().animation"))
    };
    h.run_frames(20);
    assert_eq!(h.game.0.error(), None);
    // O corpo do herói vem do modelo (1,8 m) e os pés ficam no chão.
    let y: f64 = lua(&rt, "C.objects['Herói'].y");
    assert!(y.abs() < 0.05, "pés no chão: {y}");
    assert_eq!(anim(&rt, "Herói"), "parado");
    assert_eq!(anim(&rt, "Bicho"), "andar");
    assert_eq!(lua::<f64>(&rt, "C.objects.Bicho:animator().speed"), 2.0);
    // Andando com o teclado (velocidade 6): corre.
    h.ctx.input.press_key(KeyCode::KeyW);
    h.run_frames(15);
    assert_eq!(anim(&rt, "Herói"), "correr");
    h.ctx.input.release_key(KeyCode::KeyW);
    // Pula: "pular" (uma vez), depois "cair", e de volta ao chão, "parado".
    h.ctx.input.press_key(KeyCode::Space);
    h.run_frames(4);
    h.ctx.input.release_key(KeyCode::Space);
    assert_eq!(anim(&rt, "Herói"), "pular");
    h.run_frames(30);
    assert_eq!(anim(&rt, "Herói"), "cair");
    h.run_frames(60);
    assert_eq!(anim(&rt, "Herói"), "parado");
    // Um evento toca "parado" uma vez no bicho; ao terminar, outro evento.
    h.ctx.input.press_key(KeyCode::KeyE);
    h.run_frames(2);
    h.ctx.input.release_key(KeyCode::KeyE);
    assert_eq!(anim(&rt, "Bicho"), "parado");
    h.run_frames(60);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(lua::<f64>(&rt, "nn.vars.fim or 0"), 1.0);
    // Pelo script: obj:play.
    rt.lua()
        .load("C.objects.Bicho:play('andar')")
        .exec()
        .unwrap();
    assert_eq!(anim(&rt, "Bicho"), "andar");
}

#[test]
fn som_e_particulas_no_3d() {
    let dir = projeto(
        "efeitos",
        &[
            ("cenas/obs.lua", OBSERVA),
            (
                "cenas/mundo.cena",
                &format!(
                    r#"return {{ format = 1, mode = "3d", gravity = -20, script = "cenas/obs.lua",
  environment = {{ fog = false, shadows = false }},
  events = {{
    {{ when = {{ {{ cond = "tecla", modo = "apertar", tecla = "p" }} }},
      actions = {{ {{ act = "soltar_particulas", alvo = "Poeira", quantidade = 12 }} }} }},
    {{ when = {{ {{ cond = "tecla", modo = "apertar", tecla = "f" }} }},
      actions = {{ {{ act = "faiscas", alvo = "Fogueira" }} }} }},
    {{ when = {{ {{ cond = "tecla", modo = "apertar", tecla = "s" }} }},
      actions = {{ {{ act = "som", som = "sons/pulo.wav", onde = "Fogueira", distancia = 10 }} }} }},
  }},
  objects = {{
    {CHAO},
    {{ id = 2, name = "Câmera", y = 2, z = 6, components = {{ {{ type = "camera3d" }} }} }},
    {{ id = 3, name = "Fogueira", x = 20,
      components = {{
        {{ type = "particles", rate = 60, lifetime = 1, speed_min = 1, speed_max = 2, spread = 20,
          size_start = 0.4, size_end = 0.1, blend = "add" }},
        {{ type = "sound", sound = "sons/pulo.wav", loop = true, spatial = true, distance = 30 }},
      }} }},
    {{ id = 4, name = "Poeira", rz = 90,
      components = {{ {{ type = "particles", mode = "explosao", on_start = false, amount = 10,
        lifetime = 2, speed_min = 1, speed_max = 1, spread = 0 }} }} }},
  }} }}"#
                ),
            ),
        ],
    );
    std::fs::create_dir_all(dir.join("sons")).unwrap();
    let wav = Path::new(env!("CARGO_MANIFEST_DIR")).join("../nn-run/modelos/comum/sons/pulo.wav");
    std::fs::write(dir.join("sons/pulo.wav"), std::fs::read(wav).unwrap()).unwrap();
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    h.run_frames(30);
    assert_eq!(h.game.0.error(), None);
    // As partículas da fogueira são 3D e sobem (o "cima" do objeto).
    assert!(lua::<bool>(&rt, "C.objects.Fogueira.particles.is3d"));
    let n: i64 = lua(&rt, "C.objects.Fogueira.particles.count");
    assert!(n > 20, "fogueira acesa: {n}");
    // O som da fogueira é 3D: a câmera está a ~20 m, mais baixo.
    let longe: f64 = lua(
        &rt,
        "C.objects.Fogueira:component('sound')._playing:distance_volume()",
    );
    assert!(longe > 0.0 && longe < 0.6, "longe: {longe}");
    // A câmera vai para perto: volume cheio.
    rt.lua()
        .load("C.objects['Câmera']:move_to(20, 2, 3)")
        .exec()
        .unwrap();
    h.run_frames(3);
    let perto: f64 = lua(
        &rt,
        "C.objects.Fogueira:component('sound')._playing:distance_volume()",
    );
    assert_eq!(perto, 1.0);
    // A fogueira anda para longe: o som vai junto.
    rt.lua()
        .load("C.objects.Fogueira:move_to(-40, 0, 0)")
        .exec()
        .unwrap();
    h.run_frames(3);
    let foi: f64 = lua(
        &rt,
        "C.objects.Fogueira:component('sound')._playing:distance_volume()",
    );
    assert_eq!(foi, 0.0);
    // A poeira (girada: sai para o lado) só solta quando um evento manda.
    assert_eq!(lua::<i64>(&rt, "C.objects.Poeira.particles.count"), 0);
    super_toque(&mut h, KeyCode::KeyP);
    h.run_frames(20);
    let n: i64 = lua(&rt, "C.objects.Poeira.particles.count");
    assert_eq!(n, 12);
    // Faíscas e som num evento 3D também funcionam.
    super_toque(&mut h, KeyCode::KeyF);
    super_toque(&mut h, KeyCode::KeyS);
    h.run_frames(5);
    assert_eq!(h.game.0.error(), None);
}

fn super_toque(h: &mut Headless<Uma>, k: KeyCode) {
    h.ctx.input.press_key(k);
    h.run_frames(1);
    h.ctx.input.release_key(k);
    h.run_frames(1);
}

#[test]
fn terreno_com_morro_agua_e_jogador() {
    use glam::Vec2;
    use nn_render::d3::{BrushOp, TerrainData};
    // Um morro de ~4 m em (8, 0) e um buraco com água em (-8, 0).
    let mut t = TerrainData::flat(32, Vec2::new(32.0, 32.0), 10.0);
    for _ in 0..8 {
        t.brush(BrushOp::Raise, Vec2::new(8.0, 0.0), 6.0, 0.5);
        t.brush(BrushOp::Lower, Vec2::new(-8.0, 0.0), 4.0, 0.5);
    }
    let data = t.encode();
    let dir = projeto(
        "terreno",
        &[
            ("cenas/obs.lua", OBSERVA),
            (
                "cenas/mundo.cena",
                &format!(
                    r#"return {{ format = 1, mode = "3d", gravity = -20, script = "cenas/obs.lua",
  environment = {{ fog = false, shadows = false }},
  objects = {{
    {{ id = 1, name = "Terreno",
      components = {{ {{ type = "terrain", data = "{data}", cells = 32, width = 32, depth = 32, height = 10,
        water = true, water_level = -1 }} }} }},
    {{ id = 2, name = "Herói", y = 1,
      components = {{ {{ type = "shape3d", shape = "capsule" }},
        {{ type = "behavior", script = "nn:jogador3d", prender_mouse = false }} }},
      events = {{ {{ when = {{ {{ cond = "mensagem", quem = "@este", msg = "agua" }} }},
        actions = {{ {{ act = "variavel", var = "molhou", op = "somar", valor = 1 }} }} }} }} }},
  }} }}"#
                ),
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    h.run_frames(30);
    assert_eq!(h.game.0.error(), None);
    // (A cápsula tem o centro no meio: os pés ficam 0,5 m abaixo.)
    let y: f64 = lua(&rt, "C.objects['Herói'].y");
    assert!((y - 0.5).abs() < 0.1, "em pé no plano: {y}");
    // A altura do terreno pelo script.
    let topo: f64 = lua(&rt, "C.objects.Terreno.terrain:height_at(8, 0)");
    assert!(topo > 3.0, "morro: {topo}");
    assert_eq!(lua::<Option<f64>>(&rt, "C:water_level(-8, 0)"), Some(-1.0));
    // Em cima do morro: fica de pé lá em cima (o chão é o terreno).
    rt.lua()
        .load("C.objects['Herói']:move_to(8, 6, 0)")
        .exec()
        .unwrap();
    h.run_frames(60);
    let y: f64 = lua(&rt, "C.objects['Herói'].y");
    assert!(
        (y - 0.5 - topo).abs() < 0.3,
        "no topo do morro: {y} (topo {topo})"
    );
    // Na água do buraco: recebe "agua" e volta ao chão firme.
    rt.lua()
        .load("C.objects['Herói']:move_to(-8, 0, 0)")
        .exec()
        .unwrap();
    h.run_frames(60);
    assert_eq!(lua::<f64>(&rt, "nn.vars.molhou or 0"), 1.0);
    let x: f64 = lua(&rt, "C.objects['Herói'].x");
    assert!(x > 0.0, "voltou para o morro: {x}");
    // O script pode mexer no terreno (e o chão acompanha no desenho).
    let v0: f64 = lua(&rt, "C.objects.Terreno.terrain.version");
    rt.lua()
        .load("C.objects.Terreno.terrain:brush('subir', 0, 8, 3, 2)")
        .exec()
        .unwrap();
    h.run_frames(2);
    assert!(lua::<f64>(&rt, "C.objects.Terreno.terrain.version") > v0);
    // ...e o chão da física também: o herói fica em cima do calombo novo.
    rt.lua()
        .load("C.objects['Herói']:move_to(0, 5, 8)")
        .exec()
        .unwrap();
    h.run_frames(60);
    let alto: f64 = lua(&rt, "C.objects.Terreno.terrain:height_at(0, 8)");
    let y: f64 = lua(&rt, "C.objects['Herói'].y");
    assert!(
        alto > 1.0 && (y - 0.5 - alto).abs() < 0.3,
        "no calombo: {y} ({alto})"
    );
    // O editor manda o terreno pintado ao vivo: o chão muda (e a física).
    let mut plano = TerrainData::flat(32, Vec2::new(32.0, 32.0), 10.0);
    for _ in 0..10 {
        plano.brush(BrushOp::Raise, Vec2::new(-10.0, -10.0), 5.0, 0.5);
    }
    rt.lua()
        .load(format!(
            "AO_VIVO = {{ {{ id = 1, component = 1, field = 'data', value = '{}' }} }}",
            plano.encode()
        ))
        .exec()
        .unwrap();
    h.run_frames(2);
    let novo: f64 = lua(&rt, "C.objects.Terreno.terrain:height_at(-10, -10)");
    assert!(novo > 3.0, "ao vivo: {novo}");
    assert_eq!(
        lua::<f64>(&rt, "C.objects.Terreno.terrain:height_at(8, 0)"),
        0.0
    );
    rt.lua()
        .load("C.objects['Herói']:move_to(-10, 8, -10)")
        .exec()
        .unwrap();
    h.run_frames(60);
    let y: f64 = lua(&rt, "C.objects['Herói'].y");
    assert!((y - 0.5 - novo).abs() < 0.3, "no morro novo: {y} ({novo})");
    assert_eq!(h.game.0.error(), None);
}

#[test]
fn inimigos_vida_perigo_e_tiro_no_3d() {
    let dir = projeto(
        "inimigos",
        &[
            ("cenas/obs.lua", OBSERVA),
            (
                "prefabs/tiro.prefab",
                r#"return { format = 1, object = { id = 1, name = "tiro",
  components = {
    { type = "shape3d", shape = "sphere" },
    { type = "body3d", body = "dynamic", sensor = true, gravity_scale = 0 },
    { type = "behavior", script = "nn:perigo", quem = "Bicho", dano = 1, some = true },
  } } }"#,
            ),
            (
                "cenas/mundo.cena",
                &format!(
                    r##"return {{ format = 1, mode = "3d", gravity = -20, script = "cenas/obs.lua",
  environment = {{ fog = false, shadows = false }},
  objects = {{
    {CHAO},
    {{ id = 10, name = "Herói", y = 0.1, z = 10,
      components = {{ {{ type = "model", file = "modelos/heroi.glb" }},
        {{ type = "behavior", script = "nn:jogador3d", prender_mouse = false }},
        {{ type = "behavior", script = "nn:vida", vidas = 3, variavel = "vidas", invencivel = 1, ao_acabar = "nada" }},
        {{ type = "behavior", script = "nn:atirar", prefab = "prefabs/tiro.prefab", tecla = "x", direcao = "frente" }} }} }},
    {{ id = 11, name = "Bicho", x = -3, z = 0,
      components = {{ {{ type = "model", file = "modelos/bicho.glb" }},
        {{ type = "behavior", script = "nn:inimigo3d", andar = "patrulha", x = 6, z = 0, visao = 4 }},
        {{ type = "behavior", script = "nn:vida", vidas = 2, variavel = "", ao_acabar = "destruir", barra = true }} }} }},
    {{ id = 12, name = "Lava", x = 8, y = 0.25, z = 10, scale_y = 0.5,
      components = {{ {{ type = "shape3d", shape = "cube", color = "#FF5500" }},
        {{ type = "behavior", script = "nn:perigo", dano = 1 }} }} }},
  }} }}"##
                ),
            ),
        ],
    );
    com_modelos(&dir);
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    let pos = |rt: &ScriptRuntime, nome: &str| -> (f64, f64, f64) {
        let v: Vec<f64> = lua(rt, &format!("{{ C.objects['{nome}']:position3d() }}"));
        (v[0], v[1], v[2])
    };
    h.run_frames(10);
    assert_eq!(h.game.0.error(), None);
    let x0 = pos(&rt, "Bicho").0;
    // Patrulha: anda em X e toca "andar".
    h.run_frames(60);
    let (x1, y1, _) = pos(&rt, "Bicho");
    assert!(x1 > x0 + 1.0, "patrulhou: {x0} → {x1}");
    assert!(y1.abs() < 0.1, "no chão: {y1}");
    assert_eq!(
        lua::<String>(&rt, "C.objects.Bicho:animator().animation"),
        "andar"
    );
    // O herói chega perto: o bicho persegue e encosta. Perde uma vida e é
    // empurrado para longe.
    rt.lua()
        .load("C.objects['Herói']:move_to(C.objects.Bicho.x, 0.1, C.objects.Bicho.z + 3)")
        .exec()
        .unwrap();
    let mut levou = false;
    for _ in 0..120 {
        h.run_frames(1);
        if lua::<f64>(&rt, "nn.vars.vidas") < 3.0 {
            levou = true;
            break;
        }
    }
    assert!(levou, "o bicho alcançou o herói");
    assert_eq!(lua::<f64>(&rt, "nn.vars.vidas"), 2.0);
    h.run_frames(15);
    let (hx, _, hz) = pos(&rt, "Herói");
    let (bx, _, bz) = pos(&rt, "Bicho");
    assert!(
        ((hx - bx).powi(2) + (hz - bz).powi(2)).sqrt() > 1.2,
        "empurrado"
    );
    // Pisa em cima: o bicho perde uma vida (tem 2); pisa de novo: some.
    for vez in 0..2 {
        h.run_frames(70); // passa a invencibilidade do herói
        rt.lua()
            .load("local b = C.objects.Bicho; C.objects['Herói']:move_to(b.x, 3, b.z)")
            .exec()
            .unwrap();
        let mut pisou = false;
        for _ in 0..60 {
            h.run_frames(1);
            let vivo: bool = lua(
                &rt,
                "C.objects.Bicho ~= nil and C.objects.Bicho.alive == true",
            );
            let vidas: f64 = lua(
                &rt,
                "(C.objects.Bicho and C.objects.Bicho:behavior('vida').atual) or 0",
            );
            if (vez == 0 && vidas < 2.0) || (vez == 1 && !vivo) {
                pisou = true;
                break;
            }
        }
        assert!(pisou, "pisão {}", vez + 1);
    }
    assert_eq!(h.game.0.error(), None);
    // O herói atira para a frente (o +Z dele).
    rt.lua()
        .load("C.objects['Herói']:move_to(0, 0.1, 0); C.objects['Herói'].ry = 0")
        .exec()
        .unwrap();
    h.run_frames(5);
    let antes: i64 = lua(&rt, "#C:find_all('tiro')");
    h.ctx.input.press_key(KeyCode::KeyX);
    h.run_frames(2);
    h.ctx.input.release_key(KeyCode::KeyX);
    assert_eq!(lua::<i64>(&rt, "#C:find_all('tiro')"), antes + 1);
    h.run_frames(5);
    let (_, ty, tz) = pos(&rt, "tiro");
    assert!(tz > 1.0 && ty > 0.5, "o tiro voa para a frente: {ty} {tz}");
    // Some sozinho depois de 3 s.
    h.run_frames(200);
    assert_eq!(lua::<i64>(&rt, "#C:find_all('tiro')"), 0);
    // A lava tira vida também.
    h.run_frames(60);
    let antes: f64 = lua(&rt, "nn.vars.vidas");
    rt.lua()
        .load("C.objects['Herói']:move_to(8, 0.6, 10)")
        .exec()
        .unwrap();
    h.run_frames(10);
    assert!(lua::<f64>(&rt, "nn.vars.vidas") < antes, "a lava queima");
    assert_eq!(h.game.0.error(), None);
}

#[test]
fn portas_chaves_zonas_interagir_e_texto_no_3d() {
    let dir = projeto(
        "portas",
        &[
            ("cenas/obs.lua", OBSERVA),
            (
                "cenas/mundo.cena",
                &format!(
                    r##"return {{ format = 1, mode = "3d", gravity = -20, script = "cenas/obs.lua",
  environment = {{ fog = false, shadows = false }},
  events = {{
    {{ when = {{ {{ cond = "comecou" }} }},
      actions = {{ {{ act = "dar_item", item = "mapa", qtd = 2 }} }} }},
    {{ when = {{ {{ cond = "interagir", quem = "Placa", por = "@jogador" }} }},
      actions = {{ {{ act = "variavel", var = "lida", op = "somar", valor = 1 }},
        {{ act = "tirar_item", item = "mapa", qtd = 1 }} }} }},
    {{ when = {{ {{ cond = "entrar_zona", quem = "Zona", por = "@jogador" }} }},
      actions = {{ {{ act = "variavel", var = "entrou", op = "somar", valor = 1 }} }} }},
    {{ when = {{ {{ cond = "sair_zona", quem = "Zona", por = "@jogador" }} }},
      actions = {{ {{ act = "variavel", var = "saiu", op = "somar", valor = 1 }} }} }},
    {{ when = {{ {{ cond = "tem_item", item = "mapa", qtd = 2 }} }}, mode = "enquanto",
      actions = {{ {{ act = "variavel", var = "dois_mapas", op = "definir", valor = 1 }} }} }},
  }},
  objects = {{
    {CHAO},
    {HEROI}
    {{ id = 20, name = "Chave", x = 4, y = 0.5, scale_x = 0.3, scale_y = 0.3, scale_z = 0.3,
      components = {{ {{ type = "shape3d", shape = "cube", color = "#FFD166" }},
        {{ type = "behavior", script = "nn:coletavel", variavel = "", item = "chave", icone = "" }} }} }},
    {{ id = 21, name = "Porta", y = 1, z = -5, scale_x = 1.2, scale_y = 2, scale_z = 0.2,
      components = {{ {{ type = "shape3d", shape = "cube", color = "#9C7049" }},
        {{ type = "body3d", body = "kinematic" }},
        {{ type = "behavior", script = "nn:porta3d", chave = "chave", tempo = 0.5 }} }} }},
    {{ id = 22, name = "Zona", x = 8, y = 1, scale_x = 2, scale_y = 2, scale_z = 2,
      components = {{ {{ type = "shape3d", shape = "cube", color = "#3FA7D640" }},
        {{ type = "behavior", script = "nn:gatilho", alvo = "Portão", mensagem = "abrir", mensagem_saida = "fechar" }} }} }},
    {{ id = 23, name = "Portão", x = 8, y = 1, z = -5, scale_x = 2, scale_y = 2, scale_z = 0.2,
      components = {{ {{ type = "shape3d", shape = "cube" }}, {{ type = "body3d", body = "kinematic" }},
        {{ type = "behavior", script = "nn:porta3d", abre = "subindo", abrir_com = "mensagem", tempo = 0.3 }} }} }},
    {{ id = 24, name = "Placa", x = -6, y = 0.5,
      components = {{ {{ type = "shape3d", shape = "cube" }},
        {{ type = "text3d", text = "Mapas: {{mapa}}", facing = "placa" }},
        {{ type = "behavior", script = "nn:interagir", texto = "ler" }} }} }},
    {{ id = 25, name = "Itens", components = {{ {{ type = "behavior", script = "nn:inventario" }} }} }},
  }} }}"##
                ),
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    let apertar_e = |h: &mut Headless<Uma>| {
        h.ctx.input.press_key(KeyCode::KeyE);
        h.run_frames(2);
        h.ctx.input.release_key(KeyCode::KeyE);
        h.run_frames(1);
    };
    let ir = |rt: &ScriptRuntime, x: f64, z: f64| {
        rt.lua()
            .load(format!("C.objects['Herói']:move_to({x}, 1, {z})"))
            .exec()
            .unwrap();
    };
    h.run_frames(10);
    assert_eq!(h.game.0.error(), None);
    // O evento do começo deu 2 mapas (a regra "tem 2 mapas" vale).
    assert_eq!(lua::<i64>(&rt, "nn.inventory.count('mapa')"), 2);
    assert_eq!(lua::<f64>(&rt, "nn.vars.dois_mapas or 0"), 1.0);
    // A zona não aparece no jogo.
    assert!(!lua::<bool>(&rt, "C.objects.Zona.visible"));

    // Perto da porta trancada: o aviso pede a chave e E não abre.
    ir(&rt, 0.0, -3.6);
    h.run_frames(5);
    assert_eq!(
        lua::<String>(&rt, "C._ia and C._ia.text or ''"),
        "Precisa de: chave"
    );
    apertar_e(&mut h);
    h.run_frames(30);
    let ry: f64 = lua(&rt, "C.objects.Porta.ry");
    assert_eq!(ry, 0.0, "trancada");

    // Pega a chave (vai para o inventário, a variável não muda).
    ir(&rt, 4.0, 0.0);
    h.run_frames(5);
    assert_eq!(lua::<i64>(&rt, "nn.inventory.count('chave')"), 1);
    assert!(lua::<bool>(&rt, "C.objects.Chave == nil"));

    // Volta à porta: E abre (gira para longe do herói) e gasta a chave.
    ir(&rt, 0.0, -3.6);
    h.run_frames(5);
    assert_eq!(lua::<String>(&rt, "C._ia.text"), "abrir");
    apertar_e(&mut h);
    h.run_frames(40);
    let (ry, pz): (f64, f64) = (
        lua(&rt, "C.objects.Porta.ry"),
        lua(&rt, "C.objects.Porta.z"),
    );
    assert!(ry.abs() > 90.0, "girou: {ry}");
    assert!(pz < -5.2, "a ponta foi para longe do herói (z < -5): {pz}");
    assert_eq!(lua::<i64>(&rt, "nn.inventory.count('chave')"), 0);
    // O corpo foi junto: dá para passar pelo meio da porta.
    let (bx, bz): (f64, f64) = (
        lua(&rt, "C.objects.Porta.body.x"),
        lua(&rt, "C.objects.Porta.body.z"),
    );
    assert!((bz - pz).abs() < 0.01 && bx.abs() > 0.3, "corpo: {bx} {bz}");
    // Aberta: o aviso vira "fechar" e E fecha de novo.
    h.run_frames(2);
    assert_eq!(lua::<String>(&rt, "C._ia.text"), "fechar");
    apertar_e(&mut h);
    h.run_frames(40);
    let ry: f64 = lua(&rt, "C.objects.Porta.ry");
    assert!(ry.abs() < 0.01, "fechou: {ry}");

    // A zona abre o portão ao entrar e fecha ao sair.
    let y0: f64 = lua(&rt, "C.objects['Portão'].y");
    ir(&rt, 8.0, 0.0);
    h.run_frames(30);
    assert_eq!(lua::<f64>(&rt, "nn.vars.entrou or 0"), 1.0);
    let y1: f64 = lua(&rt, "C.objects['Portão'].y");
    assert!(y1 > y0 + 1.5, "o portão subiu: {y0} → {y1}");
    ir(&rt, 12.0, 3.0);
    h.run_frames(30);
    assert_eq!(lua::<f64>(&rt, "nn.vars.saiu or 0"), 1.0);
    let y2: f64 = lua(&rt, "C.objects['Portão'].y");
    assert!((y2 - y0).abs() < 0.01, "o portão desceu: {y2}");

    // A placa: E perto dela avisa o evento (e o texto 3D desenha).
    ir(&rt, -4.4, 0.0);
    h.run_frames(5);
    assert_eq!(lua::<String>(&rt, "C._ia.text"), "ler");
    apertar_e(&mut h);
    h.run_frames(2);
    assert_eq!(lua::<f64>(&rt, "nn.vars.lida or 0"), 1.0);
    assert_eq!(lua::<i64>(&rt, "nn.inventory.count('mapa')"), 1);
    assert_eq!(h.game.0.error(), None);
}

#[test]
fn arquivos_que_faltam_viram_avisos_e_o_jogo_continua() {
    let dir = projeto(
        "faltam",
        &[
            ("cenas/obs.lua", OBSERVA),
            ("modelos/quebrado.glb", "isto não é um glb"),
            (
                "cenas/mundo.cena",
                &mundo(
                    r#"{ id = 2, name = "Sumido", y = 1, components = { { type = "model", file = "modelos/nao.glb" } } },
    { id = 3, name = "Quebrado", x = 2, y = 1, components = { { type = "model", file = "modelos/quebrado.glb" } } },
    { id = 4, name = "Mudo", components = { { type = "sound", sound = "sons/nao.wav" } } },
    { id = 5, name = "Ana", components = { { type = "behavior", script = "nn:dialogo", arquivo = "dialogos/nao.dialogo", comecar = "cena" } } },"#,
                ),
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    h.run_frames(5);
    assert_eq!(h.game.0.error(), None);
    let avisos: Vec<String> = rt
        .lua()
        .named_registry_value::<mlua::Table>("nn.cena")
        .unwrap()
        .get::<Vec<String>>("warnings")
        .unwrap();
    let tem = |s: &str| avisos.iter().any(|a| a.contains(s));
    assert!(
        tem(
            "objeto \"Sumido\" (Modelo 3D): o arquivo modelos/nao.glb não existe; no lugar dele aparece um cubo rosa"
        ),
        "{avisos:?}"
    );
    assert!(tem("objeto \"Quebrado\" (Modelo 3D)"), "{avisos:?}");
    assert!(
        tem("objeto \"Mudo\" (Som): o arquivo sons/nao.wav não existe"),
        "{avisos:?}"
    );
    assert!(
        tem("conversa dialogos/nao.dialogo: o arquivo dialogos/nao.dialogo não existe"),
        "{avisos:?}"
    );
    // Os dois cubos rosa (e o chão) são desenhados.
    assert_eq!(h.ctx.renderer.last_mesh_count(), 3);
    assert!(lua::<bool>(&rt, "C:dialogue() == nil"));
}

/// Um clique do mouse numa cena 3D com corpos (como para prender o mouse no
/// jogo) não dá erro, e "Clique no objeto" acerta o que o mouse aponta.
#[test]
fn clique_do_mouse_no_3d_acerta_o_objeto_e_nao_da_erro() {
    let dir = projeto(
        "clique",
        &[
            ("cenas/obs.lua", OBSERVA),
            (
                "cenas/mundo.cena",
                &mundo(
                    r#"{ id = 2, name = "Cubo", y = 0.5,
      components = { { type = "shape3d", shape = "cube" }, { type = "body3d", body = "static" } } },
    { id = 3, name = "Placar", components = { { type = "text", text = "oi", hud = true } } },"#,
                )
                .replace(
                    "objects = {",
                    r#"events = {
    { when = { { cond = "clicar", quem = "Cubo" } },
      actions = { { act = "variavel", var = "clicou", op = "somar", valor = 1 } } },
    { when = { { cond = "clicar", quem = "Chão" } },
      actions = { { act = "variavel", var = "chao", op = "somar", valor = 1 } } },
  },
  objects = {"#,
                ),
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    let clicar = |h: &mut Headless<Uma>, x: f32, y: f32| {
        h.ctx.input.set_cursor(Some(glam::Vec2::new(x, y)));
        h.ctx.input.press_mouse(nn_input::MouseButton::Left);
        h.run_frames(1);
        h.ctx.input.release_mouse(nn_input::MouseButton::Left);
        h.run_frames(2);
    };
    h.run_frames(3);
    // O mouse no meio da tela: a câmera padrão olha para o centro, o cubo.
    clicar(&mut h, 32.0, 32.0);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(lua::<i64>(&rt, "nn.vars.clicou or 0"), 1);
    assert_eq!(
        lua::<i64>(&rt, "nn.vars.chao or 0"),
        0,
        "o cubo está na frente"
    );
    // Embaixo, só o chão.
    clicar(&mut h, 32.0, 62.0);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(lua::<i64>(&rt, "nn.vars.clicou or 0"), 1);
    assert_eq!(lua::<i64>(&rt, "nn.vars.chao or 0"), 1);
    // No céu, nada (e nada de erro).
    clicar(&mut h, 32.0, 1.0);
    assert_eq!(h.game.0.error(), None);
    // A caixa 2D de um objeto 3D (bounds) vem da caixa 3D.
    let (w, hh): (f64, f64) = rt
        .lua()
        .load("return C.objects.Cubo:bounds()")
        .eval()
        .unwrap();
    assert!(
        (w - 0.5).abs() < 0.01 && (hh - 0.5).abs() < 0.01,
        "{w} {hh}"
    );
    // Ctrl+clique (aberto pelo editor, para selecionar lá): o cubo, sem erro.
    nn_script::live::enable();
    h.ctx.input.press_key(KeyCode::ControlLeft);
    clicar(&mut h, 32.0, 32.0);
    h.ctx.input.release_key(KeyCode::ControlLeft);
    h.run_frames(1);
    assert_eq!(h.game.0.error(), None);
    std::fs::remove_dir_all(&dir).unwrap();
}
