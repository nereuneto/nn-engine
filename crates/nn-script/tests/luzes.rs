//! Luzes no 3D: o componente Luz (lâmpada, holofote, tremular), o brilho das
//! formas e as ações de acender, apagar e mudar a cor, engine sem janela.

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Context, Game, Headless, Scene};
use nn_script::{LuaScene, ScriptRuntime};

const W: u32 = 64;
const H: u32 = 64;

fn projeto(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-luzes-{}-{name}", std::process::id()));
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
        .size(W, H)
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

fn exec(rt: &ScriptRuntime, code: &str) {
    rt.lua().load(code).exec().unwrap();
}

/// Guarda a cena em `C` e, a cada quadro, os números do 3D em `STATS` e onde
/// o ponto `ALVO` aparece na tela em `TELA` (as funções nn.* só rodam
/// dentro da cena).
const OBSERVA: &str = r#"
local c = {}
function c:enter() C = self end
function c:draw()
  STATS = nn.d3.stats()
  if ALVO then TELA = { nn.d3.to_screen(ALVO[1], ALVO[2], ALVO[3]) } end
end
return c
"#;

/// Onde o ponto aparece na tela (depois de um quadro).
fn na_tela(h: &mut Headless<Uma>, rt: &ScriptRuntime, x: f64, z: f64) -> (u32, u32) {
    exec(rt, &format!("ALVO = {{ {x}, 0, {z} }}"));
    h.run_frames(1);
    let (sx, sy): (f64, f64) = (lua(rt, "TELA[1]"), lua(rt, "TELA[2]"));
    (sx.round() as u32, sy.round() as u32)
}

/// Uma cena no escuro (sem sol, sem luz ambiente, sem céu), com um chão
/// branco grande e a câmera a 10 m de altura olhando para baixo.
fn escuro(objetos: &str, eventos: &str) -> String {
    format!(
        r##"return {{ format = 1, mode = "3d", gravity = -20, script = "cenas/obs.lua",
  environment = {{ sky = false, fog = false, shadows = false, sun_color = "#000000",
    ambient_sky = "#000000", ambient_ground = "#000000" }},
  events = {{ {eventos} }},
  objects = {{
    {{ id = 1, name = "Chão", scale_x = 30, scale_z = 30,
      components = {{ {{ type = "shape3d", shape = "plane", color = "#FFFFFF" }} }} }},
    {{ id = 2, name = "Câmera", y = 10, rx = -90, components = {{ {{ type = "camera3d" }} }} }},
    {objetos}
  }} }}"##
    )
}

fn pixel(img: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * W + x) * 4) as usize;
    img[i..i + 4].try_into().unwrap()
}

fn brilho(p: [u8; 4]) -> u32 {
    u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])
}

#[test]
fn lampada_acende_o_chao_em_volta_e_as_acoes_apagam_e_mudam_a_cor() {
    let objetos = r##"{ id = 10, name = "Lâmpada", y = 1,
      components = { { type = "light3d", color = "#FF8000", range = 4, intensity = 1.5 } } }"##;
    let eventos = r##"
    { when = { { cond = "variavel", var = "apagar", op = "==", valor = 1 } },
      actions = { { act = "luz", acao = "apagar", alvo = "Lâmpada" } } },
    { when = { { cond = "variavel", var = "acender", op = "==", valor = 1 } },
      actions = { { act = "luz", acao = "acender", alvo = "Lâmpada" },
        { act = "cor_luz", alvo = "Lâmpada", cor = "#0040FF", forca = 2 } } }"##;
    let dir = projeto(
        "lampada",
        &[
            ("cenas/obs.lua", OBSERVA),
            ("cenas/c.cena", &escuro(objetos, eventos)),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/c.cena") else {
        return;
    };
    h.run_frames(3);
    assert_eq!(h.game.0.error(), None);
    let img = h.pixels().unwrap();
    let meio = pixel(&img, W / 2, H / 2);
    assert!(
        meio[0] > 150 && meio[2] < 20,
        "laranja embaixo da lâmpada: {meio:?}"
    );
    assert!(
        brilho(pixel(&img, 2, 2)) < 6,
        "longe, escuro: {:?}",
        pixel(&img, 2, 2)
    );
    assert_eq!(lua::<u32>(&rt, "STATS.lamps"), 1);
    // obj.light3d é o componente.
    assert_eq!(lua::<f64>(&rt, "C.objects['Lâmpada'].light3d.range"), 4.0);

    // A ação apaga.
    exec(&rt, "nn.vars.apagar = 1");
    h.run_frames(3);
    let img = h.pixels().unwrap();
    assert!(brilho(pixel(&img, W / 2, H / 2)) < 6, "apagada");
    assert_eq!(lua::<u32>(&rt, "STATS.lamps"), 0);

    // Acende de novo, azul e mais forte.
    exec(&rt, "nn.vars.apagar = 0; nn.vars.acender = 1");
    h.run_frames(3);
    let img = h.pixels().unwrap();
    let meio = pixel(&img, W / 2, H / 2);
    assert!(meio[2] > 150 && meio[0] < 20, "azul: {meio:?}");
    assert_eq!(
        lua::<f64>(&rt, "C.objects['Lâmpada'].light3d.intensity"),
        2.0
    );
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn holofote_aponta_para_a_frente_do_objeto() {
    // Um holofote a 3 m, a 2 m do centro, virado para baixo (-90° em X): o
    // círculo de luz fica embaixo dele, e o centro da tela fica escuro.
    let objetos = r##"{ id = 10, name = "Holofote", x = 3, y = 3, rx = -90,
      components = { { type = "light3d", kind = "spot", angle = 15, range = 10, intensity = 2 } } }"##;
    let dir = projeto(
        "holofote",
        &[
            ("cenas/obs.lua", OBSERVA),
            ("cenas/c.cena", &escuro(objetos, "")),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/c.cena") else {
        return;
    };
    h.run_frames(3);
    assert_eq!(h.game.0.error(), None);
    let img = h.pixels().unwrap();
    // Onde fica x = 3 na tela: o ponto do mundo levado para a tela.
    let (sx, sy) = na_tela(&mut h, &rt, 3.0, 0.0);
    let embaixo = pixel(&img, sx, sy);
    assert!(
        brilho(embaixo) > 300,
        "embaixo do holofote: {embaixo:?} em {sx}, {sy}"
    );
    assert!(brilho(pixel(&img, W / 2, H / 2)) < 6, "fora do cone");
    // Virado para o lado (sem rx), o chão embaixo fica escuro.
    exec(&rt, "C.objects.Holofote.rx = 0");
    h.run_frames(2);
    let img = h.pixels().unwrap();
    assert!(brilho(pixel(&img, sx, sy)) < 30);
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tremular_muda_a_luz_e_brilho_aparece_no_escuro() {
    let objetos = r##"{ id = 10, name = "Tocha", y = 1, x = -3,
      components = { { type = "light3d", range = 3, flicker = 1 } } },
    { id = 11, name = "Cristal", y = 0.5,
      components = { { type = "shape3d", shape = "cube", color = "#00FF80", glow = 1 } } }"##;
    let dir = projeto(
        "tremular",
        &[
            ("cenas/obs.lua", OBSERVA),
            ("cenas/c.cena", &escuro(objetos, "")),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/c.cena") else {
        return;
    };
    h.run_frames(2);
    assert_eq!(h.game.0.error(), None);
    let img = h.pixels().unwrap();
    // O cristal (no meio) brilha com a própria cor, sem luz em cima dele.
    let meio = pixel(&img, W / 2, H / 2);
    assert!(meio[1] > 230 && meio[0] < 10, "cristal: {meio:?}");
    // A luz da tocha muda de um quadro para outro.
    let (sx, sy) = na_tela(&mut h, &rt, -3.0, 0.0);
    let mut vistos = std::collections::BTreeSet::new();
    for _ in 0..12 {
        h.run_frames(1);
        let img = h.pixels().unwrap();
        vistos.insert(brilho(pixel(&img, sx, sy)));
    }
    assert!(vistos.len() >= 4, "a tocha tremula: {vistos:?}");
    assert!(
        *vistos.iter().next().unwrap() > 60,
        "sempre acesa: {vistos:?}"
    );
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn holofote_com_sombra_vale_mesmo_sem_a_sombra_do_sol() {
    // Escuro como uma caverna (sem a sombra do sol), um holofote com sombra
    // a 4 m apontando para baixo e um cubo ao lado, no ar: atrás do cubo, o
    // chão fica escuro; o mesmo holofote sem sombra acende ali.
    let objetos = r##"{ id = 10, name = "Holofote", y = 4, rx = -90,
      components = { { type = "light3d", kind = "spot", angle = 40, range = 12, intensity = 2, shadow = true } } },
    { id = 11, name = "Cubo", x = 1, y = 1, scale_x = 0.6, scale_y = 0.6, scale_z = 0.6,
      components = { { type = "shape3d", shape = "cube", color = "#FFFFFF" } } },"##;
    let dir = projeto(
        "sombra",
        &[
            ("cenas/obs.lua", OBSERVA),
            ("cenas/c.cena", &escuro(objetos, "")),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/c.cena") else {
        return;
    };
    h.run_frames(3);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(lua::<u32>(&rt, "STATS.spot_shadows"), 1);
    let (sx, sy) = na_tela(&mut h, &rt, 1.65, 0.0);
    let (lx, ly) = na_tela(&mut h, &rt, -1.65, 0.0);
    let img = h.pixels().unwrap();
    let (sombra, luz) = (pixel(&img, sx, sy), pixel(&img, lx, ly));
    assert!(
        brilho(sombra) * 3 < brilho(luz),
        "sombra {sombra:?} em {sx}, {sy}; luz {luz:?}"
    );
    exec(&rt, "C.objects.Holofote.light3d.shadow = false");
    h.run_frames(2);
    let img = h.pixels().unwrap();
    assert!(brilho(pixel(&img, sx, sy)) * 4 > brilho(luz) * 3);
    assert_eq!(lua::<u32>(&rt, "STATS.spot_shadows"), 0);
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}
