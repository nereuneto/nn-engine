//! Dia e noite: a hora andando, o sol e a lua, as estrelas, os eventos de
//! amanhecer e anoitecer, as luzes "Só à noite" e as ações de mudar a hora,
//! parar o tempo e mudar o ambiente, engine sem janela.

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Context, Game, Headless, Scene};
use nn_script::{LuaScene, ScriptRuntime};

fn projeto(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-dia-noite-{}-{name}", std::process::id()));
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
function c:draw() STATS = nn.d3.stats() end
return c
"#;

/// Uma cena com Dia e noite: um dia de `minutos`, começando na hora `hora`.
fn com_ciclo(minutos: f64, hora: f64, eventos: &str) -> String {
    format!(
        r##"return {{ format = 1, mode = "3d", gravity = -20, script = "cenas/obs.lua",
  environment = {{ cycle = true, day_length = {minutos}, start_hour = {hora}, sun_height = 60 }},
  events = {{ {eventos} }},
  objects = {{
    {{ id = 1, name = "Chão", scale_x = 20, scale_z = 20,
      components = {{ {{ type = "shape3d", shape = "plane", color = "#FFFFFF" }} }} }},
    {{ id = 2, name = "Poste", y = 2,
      components = {{ {{ type = "light3d", night = true, range = 6 }} }} }},
    {{ id = 3, name = "Janela", y = 1, x = 3,
      components = {{ {{ type = "shape3d", shape = "cube", color = "#FFD27A", glow = 1, glow_night = true }} }} }}
  }} }}"##
    )
}

#[test]
fn o_sol_anda_a_lua_vem_de_noite_e_as_luzes_acendem_sozinhas() {
    // Um dia de 12 segundos (cada segundo do jogo, 2 horas do relógio),
    // começando ao meio-dia.
    let eventos = r#"
    { when = { { cond = "amanhecer", quando = "anoitecer" } },
      actions = { { act = "variavel", var = "anoiteceu", op = "somar", valor = 1 } } },
    { when = { { cond = "amanhecer", quando = "amanhecer" } },
      actions = { { act = "variavel", var = "amanheceu", op = "somar", valor = 1 } } }"#;
    let dir = projeto(
        "ciclo",
        &[
            ("cenas/obs.lua", OBSERVA),
            ("cenas/c.cena", &com_ciclo(0.2, 12.0, eventos)),
        ],
    );
    let Some((mut h, rt)) = cena(&dir) else {
        return;
    };
    h.run_frames(2);
    assert_eq!(h.game.0.error(), None);
    // Meio-dia: o sol lá em cima, forte; sem estrelas; o poste apagado.
    let sol = h.ctx.renderer.light3d();
    assert!(
        sol.sun_direction.y < -0.8,
        "sol alto: {:?}",
        sol.sun_direction
    );
    assert!(sol.sun_color.r > 0.8, "{:?}", sol.sun_color);
    assert_eq!(h.ctx.renderer.sky3d().unwrap().stars, 0.0);
    assert_eq!(lua::<u32>(&rt, "STATS.lamps"), 0);
    assert_eq!(lua::<i64>(&rt, "nn.vars.hora"), 12);
    assert_eq!(lua::<String>(&rt, "nn.vars.relogio").len(), 5);
    assert!(!lua::<bool>(&rt, "C:is_night()"));
    // 4 segundos depois: 20h, de noite. A lua, fraca e azulada; as
    // estrelas; o poste aceso; o anoitecer disparou uma vez.
    h.run_frames(60 * 4);
    let (hora, dia): (f64, i64) = rt.lua().load("return C:time_of_day()").eval().unwrap();
    assert!((19.0..21.5).contains(&hora), "{hora}");
    assert_eq!(dia, 1);
    assert!(lua::<bool>(&rt, "C:is_night()"));
    let lua_ = h.ctx.renderer.light3d();
    assert!(
        lua_.sun_color.r < 0.35 && lua_.sun_color.b >= lua_.sun_color.r,
        "{:?}",
        lua_.sun_color
    );
    assert!(h.ctx.renderer.sky3d().unwrap().stars > 0.5);
    assert_eq!(lua::<u32>(&rt, "STATS.lamps"), 1);
    assert_eq!(lua::<i64>(&rt, "nn.vars.anoiteceu"), 1);
    assert_eq!(lua::<Option<i64>>(&rt, "nn.vars.amanheceu"), None);
    // Mais 6 segundos: 8h do dia 2; amanheceu uma vez, o poste apagou.
    h.run_frames(60 * 6);
    let (hora, dia): (f64, i64) = rt.lua().load("return C:time_of_day()").eval().unwrap();
    assert!((7.0..9.5).contains(&hora), "{hora}");
    assert_eq!(dia, 2);
    assert_eq!(lua::<i64>(&rt, "nn.vars.dia"), 2);
    assert_eq!(lua::<i64>(&rt, "nn.vars.amanheceu"), 1);
    assert_eq!(lua::<i64>(&rt, "nn.vars.anoiteceu"), 1);
    assert_eq!(lua::<u32>(&rt, "STATS.lamps"), 0);
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn o_sol_nasce_a_leste_e_se_poe_a_oeste_aos_poucos() {
    let dir = projeto(
        "sol",
        &[
            ("cenas/obs.lua", OBSERVA),
            ("cenas/c.cena", &com_ciclo(4.0, 6.5, "")),
        ],
    );
    let Some((mut h, rt)) = cena(&dir) else {
        return;
    };
    h.run_frames(2);
    // O sol de manhã vem de um lado; à tarde, do outro.
    let manha = h.ctx.renderer.light3d().sun_direction;
    exec(&rt, "C:set_time(17.5)");
    h.run_frames(1);
    let tarde = h.ctx.renderer.light3d().sun_direction;
    assert!(
        manha.x * tarde.x + manha.z * tarde.z < 0.0,
        "lados opostos: {manha} {tarde}"
    );
    assert!(
        manha.y > -0.4 && tarde.y > -0.4,
        "baixo no céu: {manha} {tarde}"
    );
    // A direção muda pouco de um quadro para o outro (a sombra não pula).
    let mut antes = h.ctx.renderer.light3d().sun_direction;
    for _ in 0..30 {
        h.run_frames(1);
        let agora = h.ctx.renderer.light3d().sun_direction;
        assert!(
            antes.angle_between(agora).to_degrees() < 0.5,
            "{antes} {agora}"
        );
        antes = agora;
    }
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn acoes_mudam_a_hora_param_o_tempo_e_trocam_o_ambiente() {
    let eventos = r#"
    { when = { { cond = "variavel", var = "dormir", op = "==", valor = 1 } },
      actions = { { act = "mudar_hora", hora = 22 }, { act = "variavel", var = "dormir", op = "definir", valor = 0 } } },
    { when = { { cond = "variavel", var = "parar", op = "==", valor = 1 } },
      actions = { { act = "tempo", acao = "parar" }, { act = "variavel", var = "parar", op = "definir", valor = 0 } } },
    { when = { { cond = "variavel", var = "caverna", op = "==", valor = 1 } },
      actions = { { act = "mudar_ambiente", pronto = "caverna", segundos = 1 }, { act = "variavel", var = "caverna", op = "definir", valor = 0 } } },
    { when = { { cond = "variavel", var = "voltar", op = "==", valor = 1 } },
      actions = { { act = "tempo", acao = "continuar" }, { act = "variavel", var = "voltar", op = "definir", valor = 0 } } },
    { mode = "enquanto", when = { { cond = "de_noite", quando = "noite" } },
      actions = { { act = "variavel", var = "noite", op = "definir", valor = 1 } } }"#;
    let dir = projeto(
        "acoes",
        &[
            ("cenas/obs.lua", OBSERVA),
            ("cenas/c.cena", &com_ciclo(2.0, 10.0, eventos)),
        ],
    );
    let Some((mut h, rt)) = cena(&dir) else {
        return;
    };
    h.run_frames(2);
    // Dormir: pula para as 22h, de noite.
    exec(&rt, "nn.vars.dormir = 1");
    h.run_frames(3);
    let hora: f64 = lua(&rt, "(C:time_of_day())");
    assert!((22.0..22.5).contains(&hora), "{hora}");
    assert_eq!(lua::<i64>(&rt, "nn.vars.noite"), 1);
    // Parar o tempo: a hora não anda.
    exec(&rt, "nn.vars.parar = 1");
    h.run_frames(3);
    let a: f64 = lua(&rt, "(C:time_of_day())");
    h.run_frames(120);
    let b: f64 = lua(&rt, "(C:time_of_day())");
    assert_eq!(a, b);
    // O ambiente Caverna, aos poucos (1 s): sem céu no fim.
    exec(&rt, "nn.vars.caverna = 1");
    h.run_frames(3);
    assert!(h.ctx.renderer.sky3d().is_some(), "ainda no meio");
    h.run_frames(70);
    assert!(h.ctx.renderer.sky3d().is_none(), "caverna: sem céu");
    assert_eq!(h.ctx.renderer.light3d().sun_color.r, 0.0);
    // Continuar: volta para a hora (com o céu da noite) e anda de novo.
    exec(&rt, "nn.vars.voltar = 1");
    h.run_frames(120);
    assert!(h.ctx.renderer.sky3d().is_some_and(|s| s.stars > 0.5));
    let c: f64 = lua(&rt, "(C:time_of_day())");
    assert!(c > b, "andou: {b} → {c}");
    assert_eq!(h.game.0.error(), None);
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sem_o_ciclo_de_noite_e_um_ambiente_escuro() {
    // Sem Dia e noite, mas com o ambiente Noite: a luz "Só à noite" acende
    // e a janela brilha; com o Dia, não.
    let cena_ = |sol: &str| {
        format!(
            r##"return {{ format = 1, mode = "3d", gravity = -20, script = "cenas/obs.lua",
  environment = {{ sun_color = "{sol}" }},
  objects = {{
    {{ id = 1, name = "Chão", scale_x = 20, scale_z = 20,
      components = {{ {{ type = "shape3d", shape = "plane", color = "#FFFFFF" }} }} }},
    {{ id = 2, name = "Poste", y = 2, components = {{ {{ type = "light3d", night = true }} }} }}
  }} }}"##
        )
    };
    for (sol, noite) in [("#4F6290", true), ("#FFF1D6", false)] {
        let dir = projeto(
            &format!("sem-ciclo-{noite}"),
            &[("cenas/obs.lua", OBSERVA), ("cenas/c.cena", &cena_(sol))],
        );
        let Some((mut h, rt)) = cena(&dir) else {
            return;
        };
        h.run_frames(2);
        assert_eq!(lua::<bool>(&rt, "C:is_night()"), noite, "{sol}");
        assert_eq!(lua::<u32>(&rt, "STATS.lamps"), u32::from(noite), "{sol}");
        drop(h);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn com_a_hora_andando_o_ambiente_nao_desfaz_as_sombras_do_jogo() {
    // Uma opção "sombras" de um menu: nn.d3.shadows(false) uma vez. Com o Dia
    // e noite, o ambiente é aplicado a cada quadro, mas não religa as
    // sombras (nem o halo, nem o vento).
    let obs = r#"
local c = {}
function c:enter() C = self end
function c:update()
  if DESLIGAR then nn.d3.shadows(false); nn.d3.bloom(false); DESLIGAR = nil end
end
function c:draw() STATS = nn.d3.stats(); LIGADAS = nn.d3.shadows(); HALO = nn.d3.bloom() end
return c
"#;
    let dir = projeto(
        "sombras-menu",
        &[
            ("cenas/obs.lua", obs),
            ("cenas/c.cena", &com_ciclo(0.2, 12.0, "")),
        ],
    );
    let Some((mut h, rt)) = cena(&dir) else {
        return;
    };
    h.run_frames(3);
    assert_eq!(h.game.0.error(), None);
    assert!(lua::<u32>(&rt, "STATS.shadow") > 0, "o sol faz sombra");
    exec(&rt, "DESLIGAR = true");
    h.run_frames(30);
    assert_eq!(h.game.0.error(), None);
    assert!(
        !lua::<bool>(&rt, "LIGADAS"),
        "continuam desligadas com a hora andando"
    );
    assert!(!lua::<bool>(&rt, "HALO"), "o halo também");
    assert_eq!(lua::<u32>(&rt, "STATS.shadow"), 0);
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}
