//! Testes das cenas em Lua, rodando a engine sem janela.

use std::path::{Path, PathBuf};
use std::time::Duration;

use mlua::Value;
use nn_app::{AppConfig, Context, Game, Headless, Scene, SceneStack};
use nn_input::KeyCode;
use nn_script::{LuaScene, ScriptRuntime};

/// Pasta de projeto temporária com os arquivos dados.
fn projeto(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-lua-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (path, content) in files {
        let full = dir.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }
    dir
}

fn config(dir: &Path) -> AppConfig {
    AppConfig::default()
        .size(64, 64)
        .assets_dir(dir)
        .hot_reload(true)
        .clear_color(nn_core::Color::BLACK)
}

/// Uma cena só, para inspecionar o estado.
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

fn headless<G: Game>(dir: &Path, create: impl FnOnce(&mut Context) -> G) -> Option<Headless<G>> {
    match Headless::new(config(dir), create) {
        Ok(h) => Some(h),
        Err(e) => {
            if std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1") {
                panic!("GPU obrigatória mas indisponível: {e}");
            }
            eprintln!("pulando teste: {e}");
            None
        }
    }
}

fn cena(dir: &Path, path: &str) -> Option<Headless<Uma>> {
    let rt = ScriptRuntime::new(dir).unwrap();
    headless(dir, |ctx| {
        let mut s = rt.scene(path);
        s.enter(ctx);
        Uma(s)
    })
}

fn pixel(h: &Headless<impl Game>, x: u32, y: u32) -> [u8; 4] {
    let p = h.pixels().unwrap();
    let i = ((y * 64 + x) * 4) as usize;
    p[i..i + 4].try_into().unwrap()
}

fn global<T: mlua::FromLua>(rt: &ScriptRuntime, name: &str) -> T {
    rt.lua().globals().get(name).unwrap()
}

/// Espera o suficiente para o sistema de arquivos registrar outro horário de modificação.
fn espera_relogio_do_disco() {
    std::thread::sleep(Duration::from_millis(1100));
}

#[test]
fn ciclo_de_vida_e_estado() {
    let dir = projeto(
        "ciclo",
        &[(
            "main.lua",
            r##"
            local cena = {}
            function cena:enter(args) self.frames = 0; self.passos = 0 end
            function cena:fixed_update(dt) self.passos = self.passos + 1 end
            function cena:update(dt) self.frames = self.frames + 1 end
            function cena:draw() nn.draw.rect(0, 0, 32, 32, "#FF0000") end
            return cena
            "##,
        )],
    );
    let Some(mut h) = cena(&dir, "main.lua") else {
        return;
    };
    h.run_frames(10);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(h.game.0.field::<i64>("frames"), Some(10));
    assert!(h.game.0.field::<i64>("passos").unwrap() >= 9);
    assert_eq!(pixel(&h, 32, 32), [255, 0, 0, 255]);
    assert_eq!(pixel(&h, 2, 2), [0, 0, 0, 255]);
}

#[test]
fn entrada_move_o_jogador() {
    let dir = projeto(
        "entrada",
        &[(
            "main.lua",
            r##"
            local c = {}
            function c:enter() self.x = 0; self.pulos = 0 end
            function c:fixed_update(dt)
              local dx, dy = nn.input.wasd()
              self.x = self.x + dx * 60 * dt
            end
            function c:update(dt)
              if nn.input.pressed("space") then self.pulos = self.pulos + 1 end
              self.shift = nn.input.down("shift")
            end
            return c
            "##,
        )],
    );
    let Some(mut h) = cena(&dir, "main.lua") else {
        return;
    };
    h.ctx.input.press_key(KeyCode::KeyD);
    h.ctx.input.press_key(KeyCode::Space);
    h.ctx.input.press_key(KeyCode::ShiftRight);
    h.run_frames(60);
    let x: f64 = h.game.0.field("x").unwrap();
    assert!((x - 60.0).abs() < 2.0, "x = {x}");
    assert_eq!(h.game.0.field::<i64>("pulos"), Some(1));
    assert_eq!(h.game.0.field::<bool>("shift"), Some(true));
}

#[test]
fn erro_de_sintaxe_mostra_tela_de_erro() {
    let dir = projeto(
        "sintaxe",
        &[("main.lua", "local c = {\nfunction c:draw( end\nreturn c")],
    );
    let Some(mut h) = cena(&dir, "main.lua") else {
        return;
    };
    h.run_frames(1);
    let erro = h.game.0.error().expect("deveria haver erro").to_string();
    assert!(erro.contains("main.lua:2"), "{erro}");
    // Faixa vermelha no topo da tela de erro.
    let [r, g, _, _] = pixel(&h, 10, 2);
    assert!(r > 200 && g < 100);
}

#[test]
fn erro_em_tempo_de_execucao_aponta_a_linha() {
    let dir = projeto(
        "execucao",
        &[(
            "main.lua",
            "local c = {}\nfunction c:update(dt)\n  local t = nil\n  return t.campo\nend\nreturn c",
        )],
    );
    let Some(mut h) = cena(&dir, "main.lua") else {
        return;
    };
    h.run_frames(1);
    let erro = h.game.0.error().unwrap().to_string();
    assert!(erro.contains("main.lua:4"), "{erro}");
}

#[test]
fn argumento_errado_da_mensagem_clara() {
    let dir = projeto(
        "argumento",
        &[(
            "main.lua",
            "local c = {}\nfunction c:draw()\n  nn.draw.rect(0, 0, 10, 10, '#XYZ')\nend\nreturn c",
        )],
    );
    let Some(mut h) = cena(&dir, "main.lua") else {
        return;
    };
    h.run_frames(1);
    let erro = h.game.0.error().unwrap().to_string();
    assert!(erro.contains("cor inválida"), "{erro}");
    assert!(erro.contains("main.lua:3"), "{erro}");
}

#[test]
fn recarga_mantem_o_estado_e_troca_o_codigo() {
    let dir = projeto(
        "recarga",
        &[(
            "main.lua",
            r##"
            local c = {}
            function c:enter() self.contador = 0 end
            function c:update(dt) self.contador = self.contador + 1 end
            function c:draw() nn.draw.rect(0, 0, 64, 64, "#FF0000") end
            return c
            "##,
        )],
    );
    let Some(mut h) = cena(&dir, "main.lua") else {
        return;
    };
    h.run_frames(5);
    assert_eq!(pixel(&h, 5, 5), [255, 0, 0, 255]);

    espera_relogio_do_disco();
    std::fs::write(
        dir.join("main.lua"),
        r##"
        local c = {}
        function c:enter() self.contador = 0 end
        function c:update(dt) self.contador = self.contador + 100 end
        function c:reload() self.recarregou = true end
        function c:draw() nn.draw.rect(0, 0, 64, 64, "#0000FF") end
        return c
        "##,
    )
    .unwrap();
    std::thread::sleep(ScriptRuntime::POLL_INTERVAL);
    h.run_frames(1);

    assert_eq!(pixel(&h, 5, 5), [0, 0, 255, 255]);
    assert_eq!(h.game.0.field::<bool>("recarregou"), Some(true));
    // 5 do código antigo + 100 do novo: o estado continuou.
    assert_eq!(h.game.0.field::<i64>("contador"), Some(105));
}

#[test]
fn corrigir_o_erro_e_salvar_recupera_a_cena() {
    let dir = projeto("recupera", &[("main.lua", "isto não é lua")]);
    let Some(mut h) = cena(&dir, "main.lua") else {
        return;
    };
    h.run_frames(1);
    assert!(h.game.0.error().is_some());

    espera_relogio_do_disco();
    std::fs::write(
        dir.join("main.lua"),
        "local c = {}\nfunction c:enter() self.ok = true end\nreturn c",
    )
    .unwrap();
    std::thread::sleep(ScriptRuntime::POLL_INTERVAL);
    h.run_frames(1);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(h.game.0.field::<bool>("ok"), Some(true));
}

#[test]
fn transicoes_entre_cenas_com_argumentos() {
    let dir = projeto(
        "transicoes",
        &[
            (
                "menu.lua",
                r##"
                local m = {}
                function m:enter() log = (log or "") .. "menu.enter;" end
                function m:resume() log = log .. "menu.resume;" end
                function m:update()
                  if nn.input.pressed("enter") then nn.scene.push("jogo.lua", { fase = 3 }) end
                end
                return m
                "##,
            ),
            (
                "jogo.lua",
                r##"
                local j = {}
                function j:enter(args) log = log .. "jogo.enter(" .. args.fase .. ");" end
                function j:exit() log = log .. "jogo.exit;" end
                function j:update()
                  if nn.input.pressed("p") then nn.scene.push("pausa.lua") end
                  if nn.input.pressed("escape") then nn.scene.pop() end
                end
                function j:draw() nn.draw.rect(0, 0, 64, 64, "#00FF00") end
                return j
                "##,
            ),
            (
                "pausa.lua",
                r##"
                local p = { overlay = true }
                function p:enter() log = log .. "pausa.enter;" end
                function p:update() if nn.input.pressed("p") then nn.scene.pop() end end
                function p:draw() nn.screen.rect(0, 0, 8, 8, "#FFFFFF") end
                return p
                "##,
            ),
        ],
    );
    let rt = ScriptRuntime::new(&dir).unwrap();
    let r = rt.clone();
    let Some(mut h) = headless(&dir, move |ctx| SceneStack::new(ctx, r.scene("menu.lua"))) else {
        return;
    };
    let tap = |h: &mut Headless<SceneStack>, k: KeyCode| {
        h.ctx.input.press_key(k);
        h.run_frames(1);
        h.ctx.input.release_key(k);
        h.run_frames(1);
    };

    tap(&mut h, KeyCode::Enter);
    assert_eq!(h.game.len(), 2);
    tap(&mut h, KeyCode::KeyP);
    assert_eq!(h.game.len(), 3);
    // Pausa é overlay: o jogo (verde) continua desenhado por baixo do quadrado branco.
    assert_eq!(pixel(&h, 2, 2), [255, 255, 255, 255]);
    assert_eq!(pixel(&h, 30, 30), [0, 255, 0, 255]);
    tap(&mut h, KeyCode::KeyP);
    tap(&mut h, KeyCode::Escape);
    assert_eq!(h.game.len(), 1);

    let log: String = global(&rt, "log");
    assert_eq!(
        log,
        "menu.enter;jogo.enter(3);pausa.enter;jogo.exit;menu.resume;"
    );
}

#[test]
fn require_de_modulos_do_projeto() {
    let dir = projeto(
        "require",
        &[
            (
                "util/matematica.lua",
                "return { dobro = function(x) return 2 * x end }",
            ),
            (
                "main.lua",
                "local m = require('util.matematica')\nlocal c = {}\nfunction c:enter() self.v = m.dobro(21) end\nreturn c",
            ),
        ],
    );
    let Some(mut h) = cena(&dir, "main.lua") else {
        return;
    };
    h.run_frames(1);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(h.game.0.field::<i64>("v"), Some(42));
}

#[test]
fn imagens_spritesheets_e_animacao() {
    // Imagem 2x1: quadro 1 vermelho, quadro 2 azul.
    let mut png = Vec::new();
    let mut img = image::RgbaImage::new(2, 1);
    img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
    img.put_pixel(1, 0, image::Rgba([0, 0, 255, 255]));
    img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let dir = projeto(
        "imagens",
        &[(
            "main.lua",
            r##"
            local c = {}
            function c:enter()
              self.img = nn.assets.texture("cores.png", { pixel_art = true })
              self.sheet = nn.assets.sheet("cores.png", 1, 1, { pixel_art = true })
              self.anim = nn.animator(nn.animation({ 1, 2 }, 1))
              largura, quadros = self.img.width, self.sheet.count
            end
            function c:update(dt) self.anim:update(dt) end
            function c:draw()
              nn.draw.sprite(self.sheet, 0, 0, { frame = self.anim:frame(), w = 64, h = 64 })
            end
            return c
            "##,
        )],
    );
    std::fs::write(dir.join("cores.png"), png).unwrap();
    let rt = ScriptRuntime::new(&dir).unwrap();
    let r = rt.clone();
    let Some(mut h) = headless(&dir, move |ctx| {
        let mut s = r.scene("main.lua");
        s.enter(ctx);
        Uma(s)
    }) else {
        return;
    };
    h.run_frames(1);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(global::<i64>(&rt, "largura"), 2);
    assert_eq!(global::<i64>(&rt, "quadros"), 2);
    assert_eq!(pixel(&h, 32, 32), [255, 0, 0, 255]);
    h.run_frames(60); // 1 s a 1 fps: segundo quadro
    assert_eq!(pixel(&h, 32, 32), [0, 0, 255, 255]);
}

#[test]
fn asset_inexistente_vira_erro_na_tela() {
    let dir = projeto(
        "faltando",
        &[(
            "main.lua",
            "local c = {}\nfunction c:enter()\n  self.t = nn.assets.texture('sumiu.png')\nend\nreturn c",
        )],
    );
    let Some(mut h) = cena(&dir, "main.lua") else {
        return;
    };
    h.run_frames(1);
    let erro = h.game.0.error().unwrap().to_string();
    assert!(
        erro.contains("sumiu.png") && erro.contains("main.lua:3"),
        "{erro}"
    );
}

#[test]
fn texto_e_medidas() {
    let dir = projeto(
        "texto",
        &[(
            "main.lua",
            r##"
            local c = {}
            function c:draw()
              largura, altura = nn.text_size("Olá", { size = 20 })
              nn.screen.text("HI", 2, 2, { size = 30, color = "#00FF00" })
            end
            return c
            "##,
        )],
    );
    let rt = ScriptRuntime::new(&dir).unwrap();
    let r = rt.clone();
    let Some(mut h) = headless(&dir, move |ctx| {
        let mut s = r.scene("main.lua");
        s.enter(ctx);
        Uma(s)
    }) else {
        return;
    };
    h.run_frames(1);
    assert_eq!(h.game.0.error(), None);
    let w: f64 = global(&rt, "largura");
    let a: f64 = global(&rt, "altura");
    assert!(w > 20.0 && a > 18.0, "{w} x {a}");
    let pixels = h.pixels().unwrap();
    let verdes = pixels.chunks(4).filter(|p| p[1] > 128 && p[0] < 50).count();
    assert!(verdes > 30, "{verdes}");
}

#[test]
fn nn_fora_da_cena_da_erro_claro() {
    let dir = projeto("fora", &[]);
    let rt = ScriptRuntime::new(&dir).unwrap();
    let err = rt
        .lua()
        .load("nn.time.delta()")
        .exec()
        .unwrap_err()
        .to_string();
    assert!(err.contains("durante a execução da cena"), "{err}");
    let _: Value = rt.lua().load("return nn.version").eval().unwrap();
}

#[test]
fn exemplo_minimo_do_readme() {
    let dir = projeto(
        "readme",
        &[(
            "main.lua",
            r##"
            local cena = {}

            function cena:enter()
              self.x, self.y = 0, 0
            end

            function cena:fixed_update(dt)
              local dx, dy = nn.input.wasd()
              self.x = self.x + dx * 200 * dt
              self.y = self.y + dy * 200 * dt
            end

            function cena:draw()
              nn.draw.rect(self.x, self.y, 50, 50, "#3FA7D6")
              nn.screen.text("Use WASD", 16, 16)
            end

            return cena
            "##,
        )],
    );
    let Some(mut h) = cena(&dir, "main.lua") else {
        return;
    };
    h.ctx.input.press_key(KeyCode::KeyW);
    h.run_frames(30);
    assert_eq!(h.game.0.error(), None);
    assert!(h.game.0.field::<f64>("y").unwrap() > 90.0);
}

/// As cenas do jogo de demonstração carregam e rodam sem erro.
#[test]
fn demo_roda_sem_erros() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jogos/demo");
    let project = nn_script::Project::open(&dir).unwrap();
    assert_eq!(project.start, "cenas/menu.lua");
    for cena_path in ["cenas/menu.lua", "cenas/jogo.lua", "cenas/pausa.lua"] {
        let Some(mut h) = cena(&dir, cena_path) else {
            return;
        };
        h.ctx.input.press_key(KeyCode::KeyD);
        h.run_frames(20);
        assert_eq!(h.game.0.error(), None, "{cena_path}");
    }
}

// ----------------------------------------------------------------------
// Sprint 4: física e tilemaps
// ----------------------------------------------------------------------

#[test]
fn fisica_queda_chao_e_sensores() {
    let dir = projeto(
        "fisica",
        &[(
            "main.lua",
            r##"
            local c = {}
            function c:enter()
              self.mundo = nn.physics.world({ gravity = -1000 })
              self.chao = self.mundo:body({ type = "static", x = 0, y = -10, w = 400, h = 20, tag = "chao" })
              self.moeda = self.mundo:body({ type = "static", x = 0, y = 15, radius = 8, sensor = true, tag = "moeda" })
              self.jogador = self.mundo:body({ x = 0, y = 200, w = 20, h = 30, tag = "jogador" })
              self.pegou = 0
            end
            function c:fixed_update(dt)
              for _, e in ipairs(self.mundo:step(dt)) do
                if e.type == "enter" and e.a.tag == "moeda" and e.b == self.jogador then
                  self.pegou = self.pegou + 1
                  e.a:remove()
                end
              end
              self.no_chao = self.jogador.on_ground
              self.y = self.jogador.y
              local hit = self.mundo:raycast(100, 100, 0, -1, 500)
              self.raio_y = hit and hit.y
              self.raio_corpo = hit and hit.body and hit.body.tag
            end
            function c:draw() self.mundo:debug_draw() end
            return c
            "##,
        )],
    );
    let Some(mut h) = cena(&dir, "main.lua") else {
        return;
    };
    h.run_frames(90);
    let s = &h.game.0;
    assert_eq!(s.error(), None);
    assert_eq!(s.field::<bool>("no_chao"), Some(true));
    assert!((s.field::<f64>("y").unwrap() - 15.0).abs() < 0.01);
    assert_eq!(s.field::<i64>("pegou"), Some(1));
    assert!(s.field::<f64>("raio_y").unwrap().abs() < 0.01);
    assert_eq!(s.field::<String>("raio_corpo").as_deref(), Some("chao"));
}

#[test]
fn tilemap_ascii_com_colisao_e_blocos_quebraveis() {
    let mut png = Vec::new();
    image::RgbaImage::from_pixel(2, 1, image::Rgba([90, 60, 40, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let dir = projeto(
        "ascii",
        &[(
            "main.lua",
            r##"
            local c = {}
            function c:enter()
              local sheet = nn.assets.sheet("tiles.png", 1, 1)
              self.mapa = nn.tilemap.new({
                sheet = sheet,
                tile_size = 16,
                rows = {
                  "....",
                  ".J..",
                  "#BB#",
                },
                legend = {
                  ["#"] = { tile = 1, solid = true },
                  ["B"] = { tile = 2, solid = true },
                  ["J"] = { object = "jogador" },
                },
              })
              self.mundo = nn.physics.world()
              self.mundo:add_tilemap(self.mapa)
              local j = self.mapa:objects("jogador")[1]
              self.jogador = self.mundo:body({ x = j.x, y = j.y, w = 10, h = 10 })
              self.inicio_x, self.inicio_y = j.x, j.y
              self.col, self.lin = self.mapa:cell_at(j.x, j.y)
            end
            function c:fixed_update(dt) self.mundo:step(dt) end
            function c:update()
              self.y = self.jogador.y
              if nn.input.pressed("space") then self.mapa:set_tile(2, 3, 0) end
            end
            function c:draw() self.mapa:draw() end
            return c
            "##,
        )],
    );
    std::fs::write(dir.join("tiles.png"), png).unwrap();
    let Some(mut h) = cena(&dir, "main.lua") else {
        return;
    };
    h.run_frames(60);
    let s = &h.game.0;
    assert_eq!(s.error(), None);
    assert_eq!(s.field::<f64>("inicio_x"), Some(24.0));
    assert_eq!(s.field::<f64>("inicio_y"), Some(24.0));
    assert_eq!(
        (s.field::<i64>("col"), s.field::<i64>("lin")),
        (Some(2), Some(2))
    );
    // Parado em cima do bloco (topo em y = 16).
    assert!((s.field::<f64>("y").unwrap() - 21.0).abs() < 0.01);

    // Quebra o bloco embaixo: cai pelo buraco.
    h.ctx.input.press_key(KeyCode::Space);
    h.run_frames(60);
    assert!(h.game.0.field::<f64>("y").unwrap() < 0.0);
}

#[test]
fn tilemap_do_tiled_carrega_desenha_e_lista_objetos() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../nn-tilemap/tests/fixtures");
    let dir = projeto(
        "tiled",
        &[(
            "main.lua",
            r##"
            local c = {}
            function c:enter()
              self.mapa = nn.tilemap.load("fases/teste.tmj", { scale = 1 })
              self.largura = self.mapa.world_width
              self.moedas = #self.mapa:objects("moeda")
              self.valor = self.mapa:objects("moeda")[1].properties.valor
              local inicio = self.mapa:object("inicio")
              self.ix, self.iy = inicio.x, inicio.y
              self.solido = self.mapa:solid(1, 3)
              self.musica = self.mapa.properties.musica
              nn.camera.set_position(self.mapa.world_width / 2, self.mapa.world_height / 2)
            end
            function c:draw() self.mapa:draw() end
            return c
            "##,
        )],
    );
    for f in ["fases/teste.tmj", "tilesets/decoracao.tsj"] {
        std::fs::create_dir_all(dir.join(f).parent().unwrap()).unwrap();
        std::fs::copy(fixtures.join(f), dir.join(f)).unwrap();
    }
    std::fs::create_dir_all(dir.join("imagens")).unwrap();
    image::RgbaImage::from_pixel(32, 32, image::Rgba([0, 200, 0, 255]))
        .save(dir.join("imagens/terreno.png"))
        .unwrap();
    image::RgbaImage::from_pixel(48, 16, image::Rgba([200, 0, 200, 255]))
        .save(dir.join("imagens/decoracao.png"))
        .unwrap();

    let Some(mut h) = cena(&dir, "main.lua") else {
        return;
    };
    h.run_frames(1);
    let s = &h.game.0;
    assert_eq!(s.error(), None);
    assert_eq!(s.field::<f64>("largura"), Some(64.0));
    assert_eq!(s.field::<i64>("moedas"), Some(1));
    assert_eq!(s.field::<i64>("valor"), Some(5));
    assert_eq!(
        (s.field::<f64>("ix"), s.field::<f64>("iy")),
        (Some(8.0), Some(24.0))
    );
    assert_eq!(s.field::<String>("solido").as_deref(), Some("solid"));
    assert_eq!(s.field::<String>("musica").as_deref(), Some("fase1.ogg"));
    // Linha de chão (verde) na parte de baixo do mapa, centrado na tela de 64x64.
    let p = pixel(&h, 40, 50);
    assert_eq!(p, [0, 200, 0, 255]);
}

#[test]
fn circulos_no_mundo_e_na_tela() {
    let dir = projeto(
        "circulos",
        &[(
            "main.lua",
            r##"
            local c = {}
            function c:draw()
              nn.draw.circle(0, 0, 10, "#FF0000")
              nn.screen.circle(56, 8, 6, "#00FF00")
            end
            return c
            "##,
        )],
    );
    let Some(mut h) = cena(&dir, "main.lua") else {
        return;
    };
    h.run_frames(1);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(pixel(&h, 32, 32), [255, 0, 0, 255]);
    assert_eq!(pixel(&h, 56, 8), [0, 255, 0, 255]);
}

/// O jogo de plataforma carrega o mapa do Tiled e roda sem erro.
#[test]
fn plataforma_roda_sem_erros() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jogos/plataforma");
    let project = nn_script::Project::open(&dir).unwrap();
    assert_eq!(project.start, "cenas/titulo.lua");
    for cena_path in ["cenas/titulo.lua", "cenas/fase.lua"] {
        let Some(mut h) = cena(&dir, cena_path) else {
            return;
        };
        h.ctx.input.press_key(KeyCode::KeyD);
        h.ctx.input.press_key(KeyCode::Space);
        h.run_frames(90);
        assert_eq!(h.game.0.error(), None, "{cena_path}");
    }
    // O jogador andou para a direita a partir do início.
    let Some(mut h) = cena(&dir, "cenas/fase.lua") else {
        return;
    };
    h.ctx.input.press_key(KeyCode::KeyD);
    h.run_frames(30);
    use mlua::ObjectLike;
    let corpo: mlua::AnyUserData = h.game.0.field("jogador").unwrap();
    let x: f32 = corpo.get("x").unwrap();
    assert!(x > 150.0, "x = {x}");
    assert!(corpo.get::<bool>("on_ground").unwrap());
}

// ----------------------------------------------------------------------
// Sprint 5: jogos empacotados
// ----------------------------------------------------------------------

/// Monta um pacote com todos os arquivos de uma pasta.
fn empacotar(dir: &Path) -> nn_app::Pack {
    let files: Vec<(String, Vec<u8>)> = nn_app::list_dir(dir)
        .into_iter()
        .map(|(rel, full)| (rel, std::fs::read(full).unwrap()))
        .collect();
    let bytes = nn_app::Pack::build(files.iter().map(|(p, d)| (p.as_str(), d.as_slice())));
    nn_app::Pack::parse(bytes).unwrap()
}

#[test]
fn projeto_roda_a_partir_do_pacote() {
    let mut png = Vec::new();
    image::RgbaImage::from_pixel(1, 1, image::Rgba([0, 0, 255, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let dir = projeto(
        "pacote",
        &[
            (
                "config.lua",
                "return { title = 'Empacotado', start = 'cenas/inicio.lua' }",
            ),
            ("lib/cores.lua", "return { fundo = '#00FF00' }"),
            (
                "cenas/inicio.lua",
                r##"
                local cores = require("lib.cores")
                local c = {}
                function c:enter() self.img = nn.assets.texture("img/azul.png") end
                function c:draw()
                  nn.draw.rect(0, 0, 64, 64, cores.fundo)
                  nn.draw.sprite(self.img, 0, 0, { w = 16, h = 16 })
                end
                return c
                "##,
            ),
        ],
    );
    std::fs::create_dir_all(dir.join("img")).unwrap();
    std::fs::write(dir.join("img/azul.png"), png).unwrap();

    let pack = std::sync::Arc::new(empacotar(&dir));
    // Apaga a pasta: tudo tem que vir do pacote.
    std::fs::remove_dir_all(&dir).unwrap();

    let project = nn_script::Project::from_source(pack.clone(), "padrao").unwrap();
    assert_eq!(project.config.title, "Empacotado");
    assert_eq!(project.start, "cenas/inicio.lua");
    let rt = ScriptRuntime::with_source(project.source()).unwrap();
    assert!(rt.check_syntax().is_empty());

    let config = project.config.clone().size(64, 64);
    let start = project.start.clone();
    let h = Headless::new(config, |ctx| {
        assert!(!ctx.assets.hot_reload(), "pacote não recarrega");
        let mut s = rt.scene(start);
        s.enter(ctx);
        Uma(s)
    });
    let Ok(mut h) = h else {
        if std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1") {
            panic!("GPU obrigatória");
        }
        return;
    };
    h.run_frames(1);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(pixel(&h, 2, 2), [0, 255, 0, 255]);
    assert_eq!(pixel(&h, 32, 32), [0, 0, 255, 255]);
}

#[test]
fn verificacao_de_sintaxe_aponta_arquivo_e_linha() {
    let dir = projeto(
        "sintaxe-geral",
        &[
            ("main.lua", "return {}"),
            ("cenas/quebrada.lua", "local x = \nreturn x"),
        ],
    );
    let rt = ScriptRuntime::new(&dir).unwrap();
    let erros = rt.check_syntax();
    assert_eq!(erros.len(), 1, "{erros:?}");
    assert!(erros[0].contains("cenas/quebrada.lua:2"), "{erros:?}");
}

#[test]
fn particulas_em_lua() {
    let dir = projeto(
        "particulas",
        &[(
            "main.lua",
            r##"
            local c = {}
            function c:enter()
              self.fx = nn.particles({
                lifetime = { 0.5, 0.5 }, speed = 0, size = 30,
                colors = { "#FF0000" }, blend = "add", emitting = false,
              })
              self.chuva = nn.particles({ rate = 60, lifetime = 10, speed = { 10, 20 },
                gravity = -100, area = { w = 100, h = 10 }, color = "#00FF00" })
              self.fx:burst(5, 0, 0)
            end
            function c:update(dt)
              self.fx:update(dt)
              self.chuva:update(dt)
              self.vivas = self.fx.count
              self.chovendo = self.chuva.count
            end
            function c:draw() self.fx:draw() end
            return c
            "##,
        )],
    );
    let Some(mut h) = cena(&dir, "main.lua") else {
        return;
    };
    h.run_frames(1);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(h.game.0.field::<i64>("vivas"), Some(5));
    let [r, g, b, _] = pixel(&h, 32, 32);
    assert!(r > 250 && g < 5 && b < 5);
    h.run_frames(60);
    assert_eq!(h.game.0.field::<i64>("vivas"), Some(0));
    let n = h.game.0.field::<i64>("chovendo").unwrap();
    assert!((59..=61).contains(&n), "{n}");
}

#[test]
fn particulas_com_opcao_invalida_dao_erro_claro() {
    let dir = projeto(
        "particulas-erro",
        &[(
            "main.lua",
            "local c = {}\nfunction c:enter()\n  nn.particles({ blend = 'brilho' })\nend\nreturn c",
        )],
    );
    let Some(mut h) = cena(&dir, "main.lua") else {
        return;
    };
    h.run_frames(1);
    let e = h.game.0.error().unwrap().to_string();
    assert!(
        e.contains("mistura inválida") && e.contains("main.lua:3"),
        "{e}"
    );
}
