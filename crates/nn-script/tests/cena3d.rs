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
