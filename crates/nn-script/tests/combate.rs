//! Combate 3D: arma presa num osso, Golpe, Atirar com mira, Projétil, os
//! números de dano e os eventos Acertar e Derrotar (engine sem janela).

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Context, Game, Headless, Scene};
use nn_input::{KeyCode, MouseButton};
use nn_script::{LuaScene, ScriptRuntime};

fn projeto(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-combate-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (path, content) in files {
        let full = dir.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }
    let origem = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jogos/estrelas3d/modelos");
    std::fs::create_dir_all(dir.join("modelos")).unwrap();
    for f in std::fs::read_dir(origem).unwrap() {
        let f = f.unwrap();
        std::fs::copy(f.path(), dir.join("modelos").join(f.file_name())).unwrap();
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

fn avisos(rt: &ScriptRuntime) -> Vec<String> {
    rt.lua()
        .named_registry_value::<mlua::Table>("nn.cena")
        .unwrap()
        .get::<Vec<String>>("warnings")
        .unwrap_or_default()
}

const OBSERVA: &str = r#"
local c = {}
function c:enter() C = self end
return c
"#;

const CHAO: &str = r#"{ id = 1, name = "Chão", scale_x = 40, scale_z = 40,
  components = { { type = "shape3d", shape = "plane" }, { type = "body3d", body = "static" } } }"#;

/// Uma cena 3D com o chão e mais estes objetos (e eventos).
fn mundo(objetos: &str, eventos: &str) -> String {
    format!(
        r#"return {{ format = 1, mode = "3d", gravity = -20, script = "cenas/obs.lua",
  environment = {{ fog = false, shadows = false }},
  events = {{ {eventos} }},
  objects = {{
    {CHAO},
    {objetos}
  }} }}"#
    )
}

fn xyz(rt: &ScriptRuntime, expr: &str) -> (f64, f64, f64) {
    let v: Vec<f64> = lua(rt, &format!("{{ {expr} }}"));
    (v[0], v[1], v[2])
}

#[test]
fn arma_presa_no_osso_acompanha_a_animacao() {
    let cena_ = mundo(
        r#"{ id = 10, name = "Herói", ry = 90,
      components = { { type = "model", file = "modelos/heroi.glb" },
        { type = "anim3d", animation = "golpe", loop = false, speed = 0 } },
      children = {
        { id = 11, name = "Espada",
          components = { { type = "model", file = "modelos/espada.glb" }, { type = "bone", bone = "mao_d" } } },
        { id = 12, name = "Solta",
          components = { { type = "model", file = "modelos/espada.glb" }, { type = "bone", bone = "pe" } } },
      } }"#,
        "",
    );
    let dir = projeto(
        "osso",
        &[("cenas/obs.lua", OBSERVA), ("cenas/mundo.cena", &cena_)],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    h.run_frames(2);
    assert_eq!(h.game.0.error(), None);
    // O osso que não existe: um aviso com os que existem.
    let av = avisos(&rt);
    assert!(
        av.iter().any(|a| a.contains(
            "objeto \"Solta\" (Preso a um osso): o modelo de \"Herói\" não tem o osso \"pe\""
        ) && a.contains("mao_d")),
        "{av:?}"
    );
    // Em descanso (o primeiro quadro do golpe), a mão direita do herói fica
    // na ponta do braço; virado 90° (para +X), o -X dele vira +Z.
    let (x, y, z) = xyz(&rt, "C.objects.Espada:position3d()");
    assert!(
        x.abs() < 0.05 && (y - 0.53).abs() < 0.05 && (z - 0.41).abs() < 0.05,
        "{x} {y} {z}"
    );
    // No meio do golpe, a mão (e a espada) estão no alto.
    rt.lua()
        .load("local a = C.objects['Herói']:animator(); a.speed = 1; a.time = 0.12; a.speed = 0")
        .exec()
        .unwrap();
    let (_, y, _) = xyz(&rt, "C.objects.Espada:position3d()");
    assert!(y > 1.35, "subiu com o braço: {y}");
    // A posição do objeto conta a partir do osso.
    rt.lua()
        .load("local a = C.objects['Herói']:animator(); a.speed = 1; a.time = 0; a.speed = 0; C.objects.Espada.y = 0.1")
        .exec()
        .unwrap();
    let (_, y, _) = xyz(&rt, "C.objects.Espada:position3d()");
    assert!((y - 0.63).abs() < 0.05, "{y}");
    // O osso que não existe: fica preso ao pai, como um filho comum.
    let (x, y, z) = xyz(&rt, "C.objects.Solta:position3d()");
    assert!(x.abs() < 1e-3 && y.abs() < 1e-3 && z.abs() < 1e-3);
    let _ = (KeyCode::KeyJ, MouseButton::Left);
}

fn segurar(h: &mut Headless<Uma>, k: KeyCode, frames: u32) {
    h.ctx.input.press_key(k);
    h.run_frames(frames);
    h.ctx.input.release_key(k);
    h.run_frames(1);
}

/// Um inimigo parado (Inimigo 3D sem andar) com Vida, numa posição.
fn alvo(id: u32, nome: &str, x: f64, z: f64, vidas: u32) -> String {
    format!(
        r#"{{ id = {id}, name = "{nome}", x = {x}, y = 0.9, z = {z}, scale_y = 1.8,
      components = {{ {{ type = "shape3d", shape = "capsule" }},
        {{ type = "behavior", script = "nn:inimigo3d", andar = "parado", perseguir = false, dano = 0, pisar = false }},
        {{ type = "behavior", script = "nn:vida", vidas = {vidas}, ao_acabar = "destruir", numeros = true, invencivel = 0 }} }} }}"#
    )
}

#[test]
fn golpe_do_jogador_acerta_quem_esta_na_frente() {
    let objetos = format!(
        r#"{{ id = 10, name = "Herói",
      components = {{ {{ type = "model", file = "modelos/heroi.glb" }},
        {{ type = "behavior", script = "nn:jogador3d", prender_mouse = false }},
        {{ type = "behavior", script = "nn:golpe" }} }} }},
    {}, {}, {},
    {{ id = 14, name = "Sem vida", x = 1.0, z = 0.8,
      components = {{ {{ type = "shape3d", shape = "capsule" }},
        {{ type = "behavior", script = "nn:inimigo3d", andar = "parado", perseguir = false, dano = 0, pisar = false }} }} }}"#,
        alvo(11, "Frente", 0.0, 1.2, 2),
        alvo(12, "Atrás", 0.0, -1.2, 2),
        alvo(13, "Longe", 0.0, 4.5, 2),
    );
    let eventos = r#"
    { when = { { cond = "acertar", quem = "@jogador", alvo = "" } },
      actions = { { act = "variavel", var = "acertos", op = "somar", valor = 1 } } },
    { when = { { cond = "derrotar", quem = "@jogador", alvo = "" } },
      actions = { { act = "variavel", var = "derrotas", op = "somar", valor = 1 } } },"#;
    let dir = projeto(
        "golpe",
        &[
            ("cenas/obs.lua", OBSERVA),
            ("cenas/mundo.cena", &mundo(&objetos, eventos)),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    h.run_frames(10);
    assert_eq!(h.game.0.error(), None);
    let vida = |rt: &ScriptRuntime, n: &str| -> f64 {
        lua(rt, &format!("C.objects['{n}']:behavior('vida').atual"))
    };
    let (_, _, z0) = xyz(&rt, "C.objects.Frente:position3d()");
    segurar(&mut h, KeyCode::KeyJ, 2);
    // O golpe começa: a animação e o herói parado.
    assert_eq!(
        lua::<String>(&rt, "C.objects['Herói']:animator().animation"),
        "golpe"
    );
    h.run_frames(14);
    assert_eq!(h.game.0.error(), None);
    // Acertou só quem está na frente, perto.
    assert_eq!(vida(&rt, "Frente"), 1.0);
    assert_eq!(vida(&rt, "Atrás"), 2.0);
    assert_eq!(vida(&rt, "Longe"), 2.0);
    // Sem Vida, o Inimigo 3D é derrotado de uma vez.
    assert!(!lua::<bool>(&rt, "C.objects['Sem vida'] ~= nil"));
    // Empurrado para a frente, vermelho e com o número do dano.
    h.run_frames(6);
    let (_, _, z1) = xyz(&rt, "C.objects.Frente:position3d()");
    assert!(z1 > z0 + 0.3, "empurrado: {z0} → {z1}");
    assert!(lua::<usize>(&rt, "#C._numbers") >= 1);
    assert_eq!(lua::<f64>(&rt, "nn.vars.acertos"), 2.0);
    assert_eq!(lua::<f64>(&rt, "nn.vars.derrotas"), 1.0);
    // De volta na frente: o segundo golpe derrota.
    rt.lua()
        .load("C.objects.Frente:move_to(0, 0.9, 1.2)")
        .exec()
        .unwrap();
    h.run_frames(20);
    segurar(&mut h, KeyCode::KeyJ, 2);
    h.run_frames(20);
    assert_eq!(h.game.0.error(), None);
    assert!(!lua::<bool>(&rt, "C.objects.Frente ~= nil"), "derrotado");
    assert_eq!(lua::<f64>(&rt, "nn.vars.derrotas"), 2.0);
    // O número do dano some sozinho.
    h.run_frames(70);
    assert_eq!(lua::<usize>(&rt, "#C._numbers"), 0);
}

#[test]
fn inimigo_chega_perto_e_golpeia_sozinho() {
    let objetos = r#"{ id = 10, name = "Herói",
      components = { { type = "model", file = "modelos/heroi.glb" },
        { type = "behavior", script = "nn:jogador3d", prender_mouse = false },
        { type = "behavior", script = "nn:vida", vidas = 3, invencivel = 0.2, tremer = 6 } } },
    { id = 11, name = "Besouro", z = 5,
      components = { { type = "model", file = "modelos/bicho.glb" },
        { type = "behavior", script = "nn:inimigo3d", andar = "parado", visao = 10, correr = 3 },
        { type = "behavior", script = "nn:golpe", quando = "perto", acerto = 0.3 } } }"#;
    // Quem bateu é o @outro do "Levar dano".
    let eventos = r#"
    { when = { { cond = "levou_dano", quem = "@jogador" } },
      actions = { { act = "variavel", var = "danos", op = "somar", valor = 1 },
        { act = "mostrar", alvo = "@outro", acao = "esconder" } } },"#;
    let dir = projeto(
        "inimigo",
        &[
            ("cenas/obs.lua", OBSERVA),
            ("cenas/mundo.cena", &mundo(objetos, eventos)),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    let mut golpeou = false;
    for _ in 0..240 {
        h.run_frames(1);
        if lua::<Option<String>>(&rt, "C.objects.Besouro:animator().animation").as_deref()
            == Some("golpe")
        {
            golpeou = true;
        }
        if lua::<f64>(&rt, "nn.vars.danos or 0") > 0.0 {
            break;
        }
    }
    assert_eq!(h.game.0.error(), None);
    assert!(golpeou, "tocou a mordida");
    assert_eq!(
        lua::<f64>(&rt, "C.objects['Herói']:behavior('vida').atual"),
        2.0
    );
    // Ele para perto (não encosta) e o tremor da câmera começou.
    let (_, _, z) = xyz(&rt, "C.objects.Besouro:position3d()");
    assert!(z > 0.6 && z < 2.2, "{z}");
    assert!(lua::<bool>(&rt, "C._shake3 ~= nil"));
    // O @outro do Levar dano é o besouro.
    assert!(!lua::<bool>(&rt, "C.objects.Besouro.visible"));
}

fn bola() -> &'static str {
    r##"return { format = 1, object = { id = 1, name = "Bola", scale_x = 0.3, scale_y = 0.3, scale_z = 0.3,
  components = { { type = "shape3d", shape = "sphere", color = "#FFD166" },
    { type = "behavior", script = "nn:projetil", dano = 1 } } } }"##
}

#[test]
fn tiro_3d_vai_para_o_meio_da_tela_acerta_e_some_na_parede() {
    let objetos = format!(
        r#"{{ id = 10, name = "Herói",
      components = {{ {{ type = "model", file = "modelos/heroi.glb" }},
        {{ type = "behavior", script = "nn:jogador3d", prender_mouse = false }},
        {{ type = "behavior", script = "nn:atirar", prefab = "prefabs/bola.prefab", osso = "mao_d", intervalo = 0.2 }} }} }},
    {},
    {{ id = 12, name = "Parede", y = 1.5, z = 9, scale_x = 20, scale_y = 3,
      components = {{ {{ type = "shape3d", shape = "cube" }}, {{ type = "body3d", body = "static" }} }} }}"#,
        alvo(11, "Alvo", -0.6, 4.0, 3),
    );
    let dir = projeto(
        "tiro",
        &[
            ("cenas/obs.lua", OBSERVA),
            ("cenas/mundo.cena", &mundo(&objetos, "")),
            ("prefabs/bola.prefab", bola()),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    h.run_frames(5);
    assert_eq!(h.game.0.error(), None);
    // A câmera mira reto (sem olhar para baixo) e foi para o ombro.
    rt.lua().load("C._player_cam.cam.pitch = 0").exec().unwrap();
    assert!((lua::<f64>(&rt, "C._player_cam.cam.shoulder") - 0.6).abs() < 1e-4);
    h.run_frames(30);
    let bolas = |rt: &ScriptRuntime| -> usize { lua(rt, "#C:find_all('Bola')") };
    h.ctx.input.press_mouse(MouseButton::Right);
    h.run_frames(1);
    h.ctx.input.release_mouse(MouseButton::Right);
    h.run_frames(1);
    assert_eq!(h.game.0.error(), None);
    // O braço sobe; o tiro sai logo depois.
    assert_eq!(bolas(&rt), 0);
    h.run_frames(5);
    assert_eq!(bolas(&rt), 1);
    // Saiu da mão direita, indo para a frente (e um pouco para a direita,
    // para o meio da tela).
    let (x, y, z) = xyz(&rt, "C:find_all('Bola')[1]:position3d()");
    assert!(y > 0.6 && y < 1.3 && z > 0.2 && x < 0.0, "{x} {y} {z}");
    assert_eq!(
        lua::<String>(&rt, "C.objects['Herói']:animator().animation"),
        "atirar"
    );
    h.run_frames(40);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(bolas(&rt), 0, "acertou e sumiu");
    assert_eq!(
        lua::<f64>(&rt, "C.objects.Alvo:behavior('vida').atual"),
        2.0
    );
    // Sem ninguém na frente, o tiro some na parede (antes de 3 s).
    rt.lua()
        .load("C.objects.Alvo:move_to(8, 0.9, 4)")
        .exec()
        .unwrap();
    h.run_frames(5);
    h.ctx.input.press_mouse(MouseButton::Right);
    h.run_frames(1);
    h.ctx.input.release_mouse(MouseButton::Right);
    h.run_frames(5);
    assert_eq!(bolas(&rt), 1);
    let (_, y0, _) = xyz(&rt, "C:find_all('Bola')[1]:position3d()");
    h.run_frames(60);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(bolas(&rt), 0, "bateu na parede");
    assert_eq!(
        lua::<f64>(&rt, "C.objects.Alvo:behavior('vida').atual"),
        2.0
    );
    assert!(y0 > 0.5);
}
