//! Mapas de tiles nas cenas visuais: tiles com tipo nos eventos, trocar
//! tiles e pintura ao vivo pelo editor.

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Context, Game, Headless, Scene};
use nn_script::{LuaScene, ScriptRuntime};

fn projeto(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-mapas-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (path, content) in files {
        let full = dir.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }
    // Tileset de 2 tiles de 8×8: terra e lava.
    let mut img = image::RgbaImage::new(16, 8);
    for (x, _, p) in img.enumerate_pixels_mut() {
        *p = if x < 8 {
            image::Rgba([120, 80, 40, 255])
        } else {
            image::Rgba([255, 80, 0, 255])
        };
    }
    std::fs::create_dir_all(dir.join("tiles")).unwrap();
    img.save(dir.join("tiles/terreno.png")).unwrap();
    dir
}

const TSJ: &str = r#"{ "name": "terreno", "image": "terreno.png", "imagewidth": 16, "imageheight": 8,
  "tilewidth": 8, "tileheight": 8, "columns": 2, "tilecount": 2, "margin": 0, "spacing": 0,
  "tiles": [
    { "id": 0, "properties": [ { "name": "solid", "type": "bool", "value": true } ] },
    { "id": 1, "properties": [ { "name": "solid", "type": "bool", "value": true },
                               { "name": "tipo", "type": "string", "value": "lava" } ] }
  ] }"#;

/// 6 × 3 tiles: chão de terra com lava no meio.
const TMJ: &str = r#"{ "width": 6, "height": 3, "tilewidth": 8, "tileheight": 8, "orientation": "orthogonal",
  "tilesets": [ { "firstgid": 1, "source": "../tiles/terreno.tsj" } ],
  "layers": [ { "type": "tilelayer", "name": "chao", "width": 6, "height": 3, "opacity": 1, "visible": true,
    "data": [0,0,0,0,0,0, 0,0,0,0,0,0, 1,1,2,2,1,1] } ] }"#;

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
        .hot_reload(false);
    let h = match Headless::new(config, |ctx| {
        let mut s = rt.scene(path);
        s.enter(ctx);
        Uma(s)
    }) {
        Ok(h) => h,
        Err(e) => {
            assert!(
                !std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1"),
                "GPU obrigatória: {e}"
            );
            return None;
        }
    };
    rt2.lua()
        .globals()
        .set("cena", h.game.0.table().unwrap())
        .unwrap();
    Some((h, rt2))
}

fn lua<T: mlua::FromLua>(rt: &ScriptRuntime, expr: &str) -> T {
    rt.lua().load(format!("return {expr}")).eval().unwrap()
}

#[test]
fn encostar_na_lava_e_trocar_o_tile() {
    // O mapa começa em (0, 0): a lava fica nas colunas 3 e 4 (x de 16 a 32).
    let cena_txt = r#"return {
  gravity = -600,
  events = {
    { when = { { cond = "tile", quem = "bloco", tipo = "lava" } },
      actions = { { act = "variavel", var = "queimou", op = "somar", valor = 1 },
                  { act = "trocar_tile", tile = 0 } } },
    { when = { { cond = "tecla", modo = "apertar", tecla = "x" } },
      actions = { { act = "trocar_tile_em", alvo = "marca", tile = 2 } } },
  },
  objects = {
    { id = 1, name = "mapa", components = { { type = "map", map = "mapas/fase.tmj" } } },
    { id = 2, name = "bloco", x = 24, y = 40, components = {
      { type = "shape", w = 6, h = 6 }, { type = "body", body = "dynamic", w = 6, h = 6 } } },
    { id = 3, name = "marca", x = 4, y = 20, components = { { type = "shape", w = 2, h = 2 } } },
  },
}"#;
    let dir = projeto(
        "lava",
        &[
            ("tiles/terreno.tsj", TSJ),
            ("mapas/fase.tmj", TMJ),
            ("cenas/fase.cena", cena_txt),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/fase.cena") else {
        return;
    };
    // O bloco cai em cima da lava (sólida): queima e a lava some.
    h.run_frames(90);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(
        lua::<f64>(&rt, "nn.vars.queimou"),
        1.0,
        "uma vez só, ao começar a encostar"
    );
    assert_eq!(lua::<u32>(&rt, "cena.objects.mapa.map:tile(3, 3)"), 0);
    let tipos: Vec<String> = lua(
        &rt,
        "(function() local t = {} for _, tl in ipairs(cena.objects.mapa.map:tiles_in(0, 0, 47, 7)) do t[#t+1] = (tl.properties and tl.properties.tipo) or '-' end return t end)()",
    );
    // A lava tocada (3 ou 4, conforme onde o bloco caiu) virou buraco.
    assert!(tipos.len() >= 4 && tipos.len() <= 5, "{tipos:?}");
    // Trocar o tile em um objeto: o tile embaixo da "marca" vira lava.
    h.ctx.input.press_key(nn_input::KeyCode::KeyX);
    h.run_frames(1);
    h.ctx.input.release_key(nn_input::KeyCode::KeyX);
    h.run_frames(1);
    assert_eq!(lua::<u32>(&rt, "cena.objects.mapa.map:tile(1, 1)"), 2);
    let props: String = lua(&rt, "cena.objects.mapa.map:tile_properties(1, 1).tipo");
    assert_eq!(props, "lava");
}

#[test]
fn pintura_ao_vivo_muda_tiles_e_colisao() {
    let dir = projeto(
        "ao-vivo",
        &[
            ("tiles/terreno.tsj", TSJ),
            ("mapas/fase.tmj", TMJ),
            (
                "cenas/fase.cena",
                r#"return { objects = { { id = 7, name = "mapa", components = { { type = "map", map = "mapas/fase.tmj" } } } } }"#,
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/fase.cena") else {
        return;
    };
    h.run_frames(1);
    assert_eq!(
        lua::<Option<String>>(&rt, "cena.objects.mapa.map:solid(1, 1)"),
        None
    );
    assert!(nn_script::live::push(
        r#"{"scene":"cenas/fase.cena","id":7,"field":"tiles","value":[["chao",1,1,1],["chao",2,1,2147483649]]}"#
    ));
    h.run_frames(1);
    assert_eq!(lua::<u32>(&rt, "cena.objects.mapa.map:tile(1, 1)"), 1);
    // Tile espelhado (bit do Tiled): o mesmo tile, com o espelho.
    assert_eq!(lua::<u32>(&rt, "cena.objects.mapa.map:tile(2, 1)"), 1);
    assert_eq!(
        lua::<Option<String>>(&rt, "cena.objects.mapa.map:solid(1, 1)"),
        Some("solid".into())
    );
}
