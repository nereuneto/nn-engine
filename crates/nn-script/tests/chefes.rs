//! Chefes e ondas: o Gerador no 3D, as Ondas de inimigos e o Chefe (os
//! ataques com aviso, a fúria, a barra e a derrota), engine sem janela.

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Context, Game, Headless, Scene};
use nn_script::{LuaScene, ScriptRuntime};

fn projeto(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-chefes-{}-{name}", std::process::id()));
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

#[allow(dead_code)]
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

/// Um bicho parado (para as ondas e os ajudantes): Inimigo 3D sem andar e Vida 1.
const BICHO: &str = r#"return { format = 1, object = { id = 1, name = "bicho",
  components = { { type = "model", file = "modelos/bicho.glb" },
    { type = "behavior", script = "nn:inimigo3d", andar = "parado", perseguir = false, dano = 0, pisar = false },
    { type = "behavior", script = "nn:vida", vidas = 1, variavel = "", ao_acabar = "destruir", invencivel = 0 } } } }"#;

const ESTRELA: &str = r#"return { format = 1, object = { id = 1, name = "estrela",
  components = { { type = "model", file = "modelos/estrela.glb" } } } }"#;

const BOLA: &str = r##"return { format = 1, object = { id = 1, name = "bola", scale_x = 0.3, scale_y = 0.3, scale_z = 0.3,
  components = { { type = "shape3d", shape = "sphere", color = "#FFD166" },
    { type = "behavior", script = "nn:projetil", dano = 1 } } } }"##;

fn heroi(z: f64, vidas: u32) -> String {
    format!(
        r#"{{ id = 10, name = "Herói", z = {z},
      components = {{ {{ type = "model", file = "modelos/heroi.glb" }},
        {{ type = "behavior", script = "nn:jogador3d", prender_mouse = false }},
        {{ type = "behavior", script = "nn:vida", vidas = {vidas}, variavel = "", invencivel = 0.2, ao_acabar = "nada" }} }} }}"#
    )
}

fn arquivos(cena: &str) -> Vec<(&str, &str)> {
    vec![
        ("cenas/obs.lua", OBSERVA),
        ("cenas/mundo.cena", cena),
        ("prefabs/bicho.prefab", BICHO),
        ("prefabs/estrela.prefab", ESTRELA),
        ("prefabs/bola.prefab", BOLA),
    ]
}

#[test]
fn ondas_vem_uma_depois_da_outra_e_terminam() {
    let objetos = format!(
        r#"{},
    {{ id = 20, name = "Ondas", z = 10,
      components = {{ {{ type = "behavior", script = "nn:ondas", prefab = "prefabs/bicho.prefab", quando = "cena",
        ondas = 2, primeira = 2, a_mais = 1, pausa = 0.5, intervalo = 0.1, raio = 5, longe = 3 }} }} }}"#,
        heroi(0.0, 3)
    );
    let eventos = r#"
    { when = { { cond = "onda", quem = "Ondas", quando = "comecar", onda = 0 } },
      actions = { { act = "variavel", var = "comecos", op = "somar", valor = 1 } } },
    { when = { { cond = "onda", quem = "Ondas", quando = "terminar", onda = 1 } },
      actions = { { act = "variavel", var = "fim1", op = "somar", valor = 1 } } },
    { when = { { cond = "ondas_terminaram", quem = "Ondas" } },
      actions = { { act = "variavel", var = "venceu", op = "definir", valor = 1 } } },"#;
    let cena_ = mundo(&objetos, eventos);
    let dir = projeto("ondas", &arquivos(&cena_));
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    let bichos = |rt: &ScriptRuntime| -> usize { lua(rt, "#C:find_all('bicho')") };
    h.run_frames(30);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(bichos(&rt), 2);
    assert_eq!(lua::<f64>(&rt, "nn.vars.onda"), 1.0);
    assert_eq!(lua::<f64>(&rt, "nn.vars.restantes"), 2.0);
    // No chão, em volta das Ondas (até 5 m) e longe do herói.
    for i in 1..=2 {
        let (x, y, z) = xyz(&rt, &format!("C:find_all('bicho')[{i}]:position3d()"));
        let d = (x * x + (z - 10.0).powi(2)).sqrt();
        assert!(d <= 5.3 && y.abs() < 0.2, "{x} {y} {z}");
    }
    // Derrota um: falta um.
    rt.lua()
        .load("C:hit(C:find_all('bicho')[1], 1, C.objects['Herói'])")
        .exec()
        .unwrap();
    h.run_frames(2);
    assert_eq!(lua::<f64>(&rt, "nn.vars.restantes"), 1.0);
    rt.lua()
        .load("C:hit(C:find_all('bicho')[1], 1, C.objects['Herói'])")
        .exec()
        .unwrap();
    h.run_frames(2);
    assert_eq!(lua::<f64>(&rt, "nn.vars.fim1"), 1.0);
    // A pausa e a onda 2, com 3.
    h.run_frames(60);
    assert_eq!(lua::<f64>(&rt, "nn.vars.onda"), 2.0);
    assert_eq!(bichos(&rt), 3);
    assert_eq!(lua::<f64>(&rt, "nn.vars.comecos"), 2.0);
    rt.lua()
        .load("for _, b in ipairs(C:find_all('bicho')) do C:hit(b, 1, C.objects['Herói']) end")
        .exec()
        .unwrap();
    h.run_frames(3);
    assert_eq!(lua::<f64>(&rt, "nn.vars.venceu or 0"), 1.0);
    assert_eq!(lua::<f64>(&rt, "nn.vars.restantes"), 0.0);
    // Acabou: não vem mais nenhuma.
    h.run_frames(90);
    assert_eq!(bichos(&rt), 0);
    assert_eq!(h.game.0.error(), None);
}

#[test]
fn ondas_comecam_quando_o_jogador_chega_e_gerador_no_3d() {
    let objetos = format!(
        r#"{},
    {{ id = 20, name = "Ondas", z = 12,
      components = {{ {{ type = "behavior", script = "nn:ondas", prefab = "prefabs/bicho.prefab", quando = "chegar",
        distancia = 6, ondas = 1, primeira = 1, intervalo = 0.1 }} }},
      children = {{ {{ id = 21, name = "ponto", x = 4, z = 2 }} }} }},
    {{ id = 30, name = "Gerador", x = -10, z = -10,
      components = {{ {{ type = "behavior", script = "nn:gerador", prefab = "prefabs/bicho.prefab", intervalo = 0.1,
        maximo = 2, raio = 3, comecar_ja = true }} }} }}"#,
        heroi(0.0, 3)
    );
    let cena_ = mundo(&objetos, "");
    let dir = projeto("chegar", &arquivos(&cena_));
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    h.run_frames(40);
    assert_eq!(h.game.0.error(), None);
    // O Gerador fez 2 (o máximo), em volta dele, no chão.
    assert_eq!(lua::<usize>(&rt, "#C:find_all('bicho')"), 2);
    for i in 1..=2 {
        let (x, y, z) = xyz(&rt, &format!("C:find_all('bicho')[{i}]:position3d()"));
        let d = ((x + 10.0).powi(2) + (z + 10.0).powi(2)).sqrt();
        assert!(d <= 3.3 && y.abs() < 0.2, "{x} {y} {z}");
    }
    assert_eq!(lua::<f64>(&rt, "nn.vars.onda"), 0.0);
    // O herói chega perto: começa, e o bicho sai do ponto (o filho).
    rt.lua()
        .load("C.objects['Herói']:move_to(0, 0, 8)")
        .exec()
        .unwrap();
    h.run_frames(20);
    assert_eq!(lua::<f64>(&rt, "nn.vars.onda"), 1.0);
    assert_eq!(lua::<usize>(&rt, "#C:find_all('bicho')"), 3);
    let (x, _, z) = xyz(&rt, "C:find_all('bicho')[3]:position3d()");
    assert!(
        ((x - 4.0).powi(2) + (z - 14.0).powi(2)).sqrt() < 1.0,
        "{x} {z}"
    );
}

/// Um chefe (uma cápsula grande) com só os ataques pedidos.
fn chefe(z: f64, ataques: &str, extra: &str) -> String {
    format!(
        r#"{{ id = 40, name = "Chefe", y = 1.2, z = {z}, scale_x = 2, scale_y = 2.4, scale_z = 2,
      components = {{ {{ type = "shape3d", shape = "capsule" }},
        {{ type = "behavior", script = "nn:vida", vidas = 6, variavel = "", ao_acabar = "nada", invencivel = 0, numeros = true }},
        {{ type = "behavior", script = "nn:chefe", nome = "Besouro-rei", golpe = false, investida = false,
          pulo = false, leque = false, ajudantes = false, aviso = 0.3, pausa = 0.2, distancia = 12,
          solta = "prefabs/estrela.prefab", {ataques} {extra} }} }} }}"#
    )
}

fn estado(rt: &ScriptRuntime) -> String {
    lua(
        rt,
        "C.objects.Chefe and C.objects.Chefe:behavior('chefe').estado or 'sumiu'",
    )
}

fn vida_heroi(rt: &ScriptRuntime) -> f64 {
    lua(rt, "C.objects['Herói']:behavior('vida').atual")
}

/// Roda até `f` valer (no máximo `max` quadros); devolve se valeu.
fn ate(
    h: &mut Headless<Uma>,
    rt: &ScriptRuntime,
    max: u32,
    f: impl Fn(&ScriptRuntime) -> bool,
) -> bool {
    for _ in 0..max {
        h.run_frames(1);
        if f(rt) {
            return true;
        }
    }
    false
}

#[test]
fn chefe_avisa_golpeia_fica_furioso_e_cai_deixando_o_premio() {
    let objetos = format!(
        "{},\n    {}",
        heroi(0.0, 9),
        chefe(4.0, "golpe = true,", "")
    );
    let cena_ = mundo(&objetos, "");
    let dir = projeto("chefe-golpe", &arquivos(&cena_));
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    h.run_frames(5);
    assert_eq!(h.game.0.error(), None);
    // Perto: a luta começa; anda até o herói, avisa (piscando) e golpeia.
    assert!(ate(&mut h, &rt, 200, |rt| estado(rt) == "aviso"), "avisou");
    assert!(lua::<bool>(&rt, "C.objects.Chefe._tint ~= nil"));
    assert!(
        ate(&mut h, &rt, 120, |rt| vida_heroi(rt) < 9.0),
        "o golpe acertou"
    );
    assert_eq!(h.game.0.error(), None);
    // Com metade da vida, fica furioso.
    rt.lua()
        .load("C:hit(C.objects.Chefe, 3, C.objects['Herói'])")
        .exec()
        .unwrap();
    // (Depois da pausa no acerto do golpe dele.)
    h.run_frames(10);
    assert!(lua::<bool>(
        &rt,
        "C.objects.Chefe:behavior('chefe').furioso"
    ));
    // É pesado: o golpe não empurra.
    let (_, _, z0) = xyz(&rt, "C.objects.Chefe:position3d()");
    rt.lua()
        .load("C:hit(C.objects.Chefe, 1, C.objects['Herói'], { push = 10, dx = 0, dz = 1 })")
        .exec()
        .unwrap();
    h.run_frames(10);
    let (_, _, z1) = xyz(&rt, "C.objects.Chefe:position3d()");
    assert!((z1 - z0).abs() < 1.0, "{z0} → {z1}");
    // Derrotado: cai, some e deixa a estrela.
    rt.lua()
        .load("C:hit(C.objects.Chefe, 5, C.objects['Herói'])")
        .exec()
        .unwrap();
    h.run_frames(2);
    assert_eq!(estado(&rt), "derrotado");
    assert!(ate(&mut h, &rt, 200, |rt| estado(rt) == "sumiu"), "sumiu");
    assert_eq!(lua::<usize>(&rt, "#C:find_all('estrela')"), 1);
    assert_eq!(h.game.0.error(), None);
}

#[test]
fn chefe_investida_contra_a_parede_fica_tonto() {
    let objetos = format!(
        r#"{},
    {},
    {{ id = 50, name = "Parede", y = 1.5, z = -9, scale_x = 20, scale_y = 3, scale_z = 0.5,
      components = {{ {{ type = "shape3d", shape = "cube" }}, {{ type = "body3d", body = "static" }} }} }}"#,
        heroi(-5.0, 9),
        chefe(4.0, "investida = true,", "")
    );
    let cena_ = mundo(&objetos, "");
    let dir = projeto("chefe-investida", &arquivos(&cena_));
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    assert!(
        ate(&mut h, &rt, 200, |rt| estado(rt) == "investida"),
        "investiu"
    );
    assert!(
        ate(&mut h, &rt, 200, |rt| estado(rt) == "tonto"),
        "bateu na parede"
    );
    assert!(vida_heroi(&rt) < 9.0, "atropelou o herói no caminho");
    assert_eq!(h.game.0.error(), None);
}

#[test]
fn chefe_pula_com_impacto_e_atira_em_leque() {
    let objetos = format!(
        "{},\n    {}",
        heroi(0.0, 20),
        chefe(6.0, "pulo = true,", "raio_impacto = 4,")
    );
    let cena_ = mundo(&objetos, "");
    let dir = projeto("chefe-pulo", &arquivos(&cena_));
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    assert!(ate(&mut h, &rt, 300, |rt| estado(rt) == "pulo"), "pulou");
    assert!(
        ate(&mut h, &rt, 200, |rt| vida_heroi(rt) < 20.0),
        "o impacto acertou"
    );
    assert_eq!(h.game.0.error(), None);
    drop(h);

    let objetos = format!(
        "{},\n    {}",
        heroi(0.0, 20),
        chefe(
            8.0,
            "leque = true, tiro = \"prefabs/bola.prefab\", tiros = 5,",
            ""
        )
    );
    let cena_ = mundo(&objetos, "");
    let dir = projeto("chefe-leque", &arquivos(&cena_));
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    assert!(
        ate(&mut h, &rt, 300, |rt| lua::<usize>(
            rt,
            "#C:find_all('bola')"
        ) == 5),
        "5 tiros"
    );
    // Os tiros são do chefe: não acertam ele, acertam o herói.
    assert!(lua::<bool>(
        &rt,
        "C:find_all('bola')[1]._dono == C.objects.Chefe"
    ));
    assert!(
        ate(&mut h, &rt, 200, |rt| vida_heroi(rt) < 20.0),
        "um tiro acertou"
    );
    assert_eq!(
        lua::<f64>(&rt, "C.objects.Chefe:behavior('vida').atual"),
        6.0
    );
    assert_eq!(h.game.0.error(), None);
}

/// Ferramenta: imagens do besouro-rei em cada animação, para conferir
/// (`NN_SAIDA=<pasta> cargo test -p nn-script --test chefes poses -- --ignored`).
#[test]
#[ignore]
fn poses_do_besouro_rei() {
    let Some(out) = std::env::var_os("NN_SAIDA").map(PathBuf::from) else {
        return;
    };
    let objetos = r#"{ id = 10, name = "Rei", ry = 35,
      components = { { type = "model", file = "modelos/besourao.glb" },
        { type = "anim3d", animation = "parado", speed = 0 } } },
    { id = 11, name = "Herói", x = 2.2, ry = -60,
      components = { { type = "model", file = "modelos/heroi.glb" } } },
    { id = 12, name = "Câmera", x = 4.2, y = 1.8, z = 2.6, rx = -10, ry = 62,
      components = { { type = "camera3d", fov = 55 } } }"#;
    let cena_ = mundo(objetos, "");
    let dir = projeto("poses", &arquivos(&cena_));
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    h.run_frames(3);
    for (nome, t) in [
        ("parado", 0.0),
        ("andar", 0.2),
        ("golpe", 0.15),
        ("golpe", 0.3),
        ("preparar", 0.1),
        ("pular", 0.25),
        ("tonto", 0.25),
        ("cair", 1.1),
    ] {
        rt.lua()
            .load(format!(
                "local a = C.objects.Rei:animator(); a:play('{nome}', {{ blend = 0, restart = true }}); a.speed = 1; a.time = {t}; a.speed = 0"
            ))
            .exec()
            .unwrap();
        h.run_frames(2);
        h.save_png(out.join(format!("rei-{nome}-{t}.png"))).unwrap();
    }
    assert_eq!(h.game.0.error(), None);
}

/// Muitos inimigos atrás do herói: se espalham em volta dele, sem um empurrar
/// o outro (antes, se espremiam, e a física gastava muito com isso).
#[test]
fn inimigos_em_grupo_se_espalham_sem_se_espremer() {
    let mut objetos = heroi(0.0, 99);
    for i in 0..8 {
        let x = -1.2 + (i % 4) as f64 * 0.8;
        let z = 6.0 + (i / 4) as f64 * 0.8;
        objetos.push_str(&format!(
            r#",
    {{ id = {}, name = "bicho", x = {x}, z = {z},
      components = {{ {{ type = "model", file = "modelos/bicho.glb" }},
        {{ type = "behavior", script = "nn:inimigo3d", andar = "parado", visao = 20, dano = 0, pisar = false }} }} }}"#,
            30 + i
        ));
    }
    let cena_ = mundo(&objetos, "");
    let dir = projeto("grupo", &arquivos(&cena_));
    let Some((mut h, rt)) = cena(&dir, "cenas/mundo.cena") else {
        return;
    };
    h.run_frames(240);
    assert_eq!(h.game.0.error(), None);
    // Todos chegaram perto do herói, e nenhum par está espremido (com um
    // pouco de espaço entre eles).
    let (perto, colados, menor): (u32, u32, f64) = rt
        .lua()
        .load(
            r#"
            local bs = C:find_all("bicho")
            local hx, _, hz = C.objects["Herói"]:position3d()
            local perto, colados, menor = 0, 0, 9
            for i, a in ipairs(bs) do
              local ax, _, az = a:position3d()
              if math.sqrt((ax - hx) ^ 2 + (az - hz) ^ 2) < 3.5 then perto = perto + 1 end
              for j = i + 1, #bs do
                local bx, _, bz = bs[j]:position3d()
                local lim = a._char3.radius + bs[j]._char3.radius
                menor = math.min(menor, math.sqrt((ax - bx) ^ 2 + (az - bz) ^ 2) / lim)
                if math.sqrt((ax - bx) ^ 2 + (az - bz) ^ 2) < lim * 1.1 then colados = colados + 1 end
              end
            end
            return perto, colados, menor
            "#,
        )
        .eval()
        .unwrap();
    println!("perto {perto}, menor distância / soma dos raios {menor:.2}");
    assert!(perto >= 6, "perto do herói: {perto}");
    assert_eq!(colados, 0, "espremidos");
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}
