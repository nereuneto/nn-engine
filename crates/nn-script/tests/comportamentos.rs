//! Comportamentos prontos, comportamentos do jogo, prefabs, variáveis do
//! jogo e HUD nas cenas visuais, rodando a engine sem janela.

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Context, Game, Headless, Scene, SceneStack};
use nn_input::KeyCode;
use nn_script::{BEHAVIORS, LuaScene, SCENE_RUNTIME, ScriptRuntime};

fn projeto(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-comp-{}-{name}", std::process::id()));
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
        .hot_reload(false)
        .clear_color(nn_core::Color::BLACK)
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

/// Uma cena só; o estado fica nos campos da cena.
fn cena(dir: &Path, path: &str) -> Option<(Headless<Uma>, ScriptRuntime)> {
    let rt = ScriptRuntime::new(dir).unwrap();
    let rt2 = rt.clone();
    let h = headless(dir, |ctx| {
        let mut s = rt.scene(path);
        s.enter(ctx);
        Uma(s)
    })?;
    Some((h, rt2))
}

/// Pilha de cenas (para reiniciar e trocar de cena).
fn pilha(dir: &Path, path: &str) -> Option<(Headless<SceneStack>, ScriptRuntime)> {
    let rt = ScriptRuntime::new(dir).unwrap();
    let rt2 = rt.clone();
    let path = path.to_string();
    let h = headless(dir, move |ctx| SceneStack::new(ctx, rt.scene(path)))?;
    Some((h, rt2))
}

/// Valor de uma expressão Lua (globais, `nn.vars`...).
fn lua<T: mlua::FromLua>(rt: &ScriptRuntime, expr: &str) -> T {
    rt.lua().load(format!("return {expr}")).eval().unwrap()
}

fn segurar(h: &mut Headless<impl Game>, k: KeyCode, frames: u32) {
    h.ctx.input.press_key(k);
    h.run_frames(frames);
    h.ctx.input.release_key(k);
    h.run_frames(1);
}

fn pixel(h: &Headless<impl Game>, x: u32, y: u32) -> [u8; 4] {
    let p = h.pixels().unwrap();
    let i = ((y * 64 + x) * 4) as usize;
    p[i..i + 4].try_into().unwrap()
}

/// Script de cena que expõe a posição de um objeto como globais.
const OBSERVA: &str = r#"
local c = {}
function c:update(dt)
  local h = self.objects.heroi
  if h then hx, hy, hvivo = h.x, h.y, h.alive end
  moedas_na_cena = #self:find_all("moeda")
end
return c
"#;

const CHAO: &str = r#"{ id = 1, name = "chao", x = 0, y = -40,
  components = { { type = "body", body = "static", w = 2000, h = 20 } } }"#;

#[test]
fn todos_os_comportamentos_prontos_se_descrevem_sem_a_engine() {
    // O NN Studio lê as propriedades sem `nn`: nenhum comportamento pode
    // usar a engine fora das funções.
    let lua = mlua::Lua::new();
    let m: mlua::Table = lua.load(SCENE_RUNTIME).eval().unwrap();
    let describe: mlua::Function = m.get("describe").unwrap();
    for (id, src) in BEHAVIORS {
        let d: mlua::Table = describe.call((*src, format!("nn:{id}"))).unwrap();
        let err: Option<String> = d.get("error").unwrap();
        assert_eq!(err, None, "{id}");
        let title: String = d.get("title").unwrap();
        assert!(!title.is_empty(), "{id} sem título");
        let props: mlua::Table = d.get("props").unwrap();
        for p in props.sequence_values::<mlua::Table>() {
            let p = p.unwrap();
            let key: String = p.get("key").unwrap();
            let label: Option<String> = p.get("label").unwrap();
            assert!(label.is_some(), "{id}.{key} sem rótulo");
            let ty: String = p.get("type").unwrap();
            assert!(
                [
                    "number", "bool", "text", "color", "choice", "object", "scene", "prefab",
                    "sound", "image", "anim"
                ]
                .contains(&ty.as_str()),
                "{id}.{key}: tipo {ty}"
            );
        }
    }
}

#[test]
fn jogador_de_plataforma_anda_e_pula() {
    let dir = projeto(
        "plataforma",
        &[
            ("cenas/obs.lua", OBSERVA),
            (
                "cenas/fase.cena",
                &format!(
                    r#"return {{ script = "cenas/obs.lua", gravity = -2000, objects = {{
  {CHAO},
  {{ id = 2, name = "heroi", x = 0, y = 0, components = {{
    {{ type = "body", w = 10, h = 10 }},
    {{ type = "behavior", script = "nn:plataforma", velocidade = 200, pulo = 600 }},
  }} }},
}} }}"#
                ),
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/fase.cena") else {
        return;
    };
    h.run_frames(60);
    assert_eq!(h.game.0.error(), None);
    let y0: f64 = lua(&rt, "hy");
    assert!((y0 - -25.0).abs() < 1.0, "parado no chão: {y0}");
    segurar(&mut h, KeyCode::KeyD, 30);
    let x: f64 = lua(&rt, "hx");
    assert!((80.0..120.0).contains(&x), "andou meio segundo a 200: {x}");
    h.ctx.input.press_key(KeyCode::Space);
    h.run_frames(10);
    h.ctx.input.release_key(KeyCode::Space);
    let y: f64 = lua(&rt, "hy");
    assert!(y > y0 + 40.0, "pulou: {y}");
}

#[test]
fn coletavel_soma_na_variavel_e_o_placar_mostra_no_hud() {
    let dir = projeto(
        "coletavel",
        &[
            (
                "config.lua",
                "return { vars = { moedas = 10, nome = 'Ana' } }",
            ),
            ("cenas/obs.lua", OBSERVA),
            (
                "cenas/fase.cena",
                r##"return { script = "cenas/obs.lua", objects = {
  { id = 1, name = "heroi", x = 0, y = 0, components = {
    { type = "shape", w = 10, h = 10, color = "#FF0000" },
    { type = "behavior", script = "nn:topdown", velocidade = 300 },
  } },
  { id = 2, name = "moeda", x = 60, y = 0, components = {
    { type = "shape", w = 10, h = 10, color = "#FFFF00" },
    { type = "behavior", script = "nn:coletavel", variavel = "moedas", valor = 5 },
  } },
  { id = 3, name = "camera", x = 5000, y = 0, components = { { type = "camera" } } },
  { id = 4, name = "placar", x = 0, y = 0, components = {
    { type = "shape", w = 20, h = 20, color = "#00FF00", hud = true },
    { type = "text", text = "", size = 8, hud = true },
    { type = "behavior", script = "nn:mostrar_valor", texto = "{nome}: {moedas}" },
  } },
} }"##,
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/fase.cena") else {
        return;
    };
    h.run_frames(2);
    assert_eq!(lua::<i64>(&rt, "nn.vars.moedas"), 10);
    assert_eq!(lua::<i64>(&rt, "moedas_na_cena"), 1);
    // A câmera está longe: o que é HUD aparece no meio da tela mesmo assim.
    assert_eq!(pixel(&h, 32, 24), [0, 255, 0, 255]);
    segurar(&mut h, KeyCode::KeyD, 20);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(lua::<i64>(&rt, "nn.vars.moedas"), 15);
    assert_eq!(lua::<i64>(&rt, "moedas_na_cena"), 0, "a moeda sumiu");
    let texto: String = h
        .game
        .0
        .field::<mlua::Table>("objects")
        .unwrap()
        .get::<mlua::Table>("placar")
        .unwrap()
        .get::<mlua::Table>("text")
        .unwrap()
        .get("text")
        .unwrap();
    assert_eq!(texto, "Ana: 15");
}

#[test]
fn perigo_tira_vida_e_ao_acabar_reinicia_a_cena_e_as_variaveis() {
    let dir = projeto(
        "vida",
        &[
            (
                "config.lua",
                "return { start = 'cenas/fase.cena', vars = { vidas = 2, moedas = 0 } }",
            ),
            ("cenas/obs.lua", OBSERVA),
            (
                "cenas/fase.cena",
                r##"return { script = "cenas/obs.lua", objects = {
  { id = 1, name = "heroi", x = 0, y = 0, components = {
    { type = "shape", w = 10, h = 10 },
    { type = "behavior", script = "nn:topdown", velocidade = 100 },
    { type = "behavior", script = "nn:vida", vidas = 2, variavel = "vidas", invencivel = 0.2 },
  } },
  { id = 2, name = "espinho", x = 30, y = 0, components = {
    { type = "shape", w = 10, h = 10 },
    { type = "behavior", script = "nn:perigo", dano = 1 },
  } },
} }"##,
            ),
        ],
    );
    let Some((mut h, rt)) = pilha(&dir, "cenas/fase.cena") else {
        return;
    };
    h.run_frames(2);
    rt.lua().load("nn.vars.moedas = 7").exec().unwrap();
    // Anda até o espinho e fica lá.
    h.ctx.input.press_key(KeyCode::KeyD);
    h.run_frames(14);
    assert_eq!(lua::<i64>(&rt, "nn.vars.vidas"), 1, "levou dano uma vez");
    h.run_frames(20);
    h.ctx.input.release_key(KeyCode::KeyD);
    // Acabaram as vidas: a cena recomeça (com escurecimento) e as
    // variáveis voltam ao config.lua.
    h.run_frames(90);
    assert_eq!(lua::<i64>(&rt, "nn.vars.vidas"), 2);
    assert_eq!(lua::<i64>(&rt, "nn.vars.moedas"), 0);
    let x: f64 = lua(&rt, "hx");
    assert!(x.abs() < 1.0, "voltou ao começo: {x}");
}

#[test]
fn checkpoint_e_perigo_que_manda_de_volta() {
    let dir = projeto(
        "checkpoint",
        &[
            ("cenas/obs.lua", OBSERVA),
            (
                "cenas/fase.cena",
                r##"return { script = "cenas/obs.lua", objects = {
  { id = 1, name = "heroi", x = 0, y = 0, components = {
    { type = "shape", w = 10, h = 10 },
    { type = "behavior", script = "nn:topdown", velocidade = 300 },
  } },
  { id = 2, name = "bandeira", x = 40, y = 0, components = {
    { type = "shape", w = 6, h = 6 },
    { type = "behavior", script = "nn:checkpoint" },
  } },
  { id = 3, name = "lava", x = 100, y = 0, components = {
    { type = "shape", w = 10, h = 10 },
    { type = "behavior", script = "nn:perigo", acao = "checkpoint" },
  } },
} }"##,
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/fase.cena") else {
        return;
    };
    h.ctx.input.press_key(KeyCode::KeyD);
    h.run_frames(21);
    h.ctx.input.release_key(KeyCode::KeyD);
    h.run_frames(1);
    assert_eq!(h.game.0.error(), None);
    let x: f64 = lua(&rt, "hx");
    // Passou da bandeira, encostou na lava e voltou para a bandeira (e
    // ainda andou mais um pouco com a tecla apertada).
    assert!((40.0..70.0).contains(&x), "voltou para a bandeira: {x}");
}

#[test]
fn patrulha_vira_na_beirada_e_animacao_troca_os_quadros() {
    let mut png = Vec::new();
    image::RgbaImage::from_pixel(32, 8, image::Rgba([255, 255, 255, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let dir = projeto(
        "patrulha",
        &[(
            "cenas/fase.cena",
            r##"return { gravity = -2000, objects = {
  { id = 1, name = "plataforma", x = 0, y = -20, components = {
    { type = "body", body = "static", w = 200, h = 10 } } },
  { id = 2, name = "lesma", x = 0, y = 0, components = {
    { type = "sprite", image = "lesma.png", frame_w = 8, frame_h = 8 },
    { type = "body", w = 10, h = 10 },
    { type = "behavior", script = "nn:patrulha", velocidade = 150 },
    { type = "behavior", script = "nn:animacao", parado = "1", andando = "2-4", fps = 10 },
  } },
} }"##,
        )],
    );
    std::fs::write(dir.join("lesma.png"), &png).unwrap();
    let Some((mut h, rt)) = cena(&dir, "cenas/fase.cena") else {
        return;
    };
    rt.lua()
        .load("xmin, xmax, quadros = 0, 0, {}")
        .exec()
        .unwrap();
    let observa = rt
        .lua()
        .load(
            r#"return function(scene)
  local l = scene.objects.lesma
  xmin, xmax = math.min(xmin, l.x), math.max(xmax, l.x)
  quadros[l.sprite.frame] = true
  vira = l.sprite.flip_x
end"#,
        )
        .eval::<mlua::Function>()
        .unwrap();
    let mut virou_alguma_vez = false;
    for _ in 0..240 {
        h.run_frames(1);
        observa.call::<()>(h.game.0.table().unwrap()).unwrap();
        virou_alguma_vez |= lua::<bool>(&rt, "vira");
    }
    assert_eq!(h.game.0.error(), None);
    let (xmin, xmax): (f64, f64) = (lua(&rt, "xmin"), lua(&rt, "xmax"));
    assert!(xmin > -100.0 && xmax < 100.0, "não caiu: {xmin}..{xmax}");
    assert!(xmin < -50.0 && xmax > 50.0, "foi e voltou: {xmin}..{xmax}");
    assert!(virou_alguma_vez, "a imagem virou");
    for q in 2..=4 {
        assert!(
            lua::<bool>(&rt, &format!("quadros[{q}] == true")),
            "quadro {q}"
        );
    }
}

#[test]
fn prefabs_instancia_com_mudancas_e_criados_pelo_jogo() {
    let dir = projeto(
        "prefab",
        &[
            (
                "prefabs/moeda.prefab",
                r##"return { format = 1, object = {
  id = 1, name = "moeda", x = 0, y = 0,
  components = {
    { type = "shape", w = 10, h = 10, color = "#FFFF00" },
    { type = "behavior", script = "scripts/gira.lua", velocidade = 90 },
  },
  children = {
    { id = 2, name = "brilho", x = 0, y = 8, components = { { type = "shape", w = 2, h = 2 } } },
  },
} }"##,
            ),
            (
                "scripts/gira.lua",
                r#"local G = { props = { { "velocidade", 30, label = "Velocidade" } } }
function G:start() self.comecou = true end
function G:update(dt) self.obj.rotation = self.obj.rotation + self.velocidade * dt end
function G:ola(n) return "ola " .. n end
return G"#,
            ),
            (
                "cenas/fase.lua",
                r#"local c = {}
function c:enter()
  self.criada = self:spawn("prefabs/moeda.prefab", 100, 50, { name = "nova" })
end
return c"#,
            ),
            (
                "cenas/fase.cena",
                r##"return { script = "cenas/fase.lua", objects = {
  { id = 5, name = "moeda", prefab = "prefabs/moeda.prefab", x = -20, y = 0,
    overrides = {
      { component = 1, color = "#FF0000" },
      { component = 2, velocidade = 180 },
    },
  },
} }"##,
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/fase.cena") else {
        return;
    };
    h.run_frames(30);
    assert_eq!(h.game.0.error(), None);
    // Expõe a cena para as expressões.
    rt.lua()
        .globals()
        .set("cena", h.game.0.table().unwrap())
        .unwrap();
    let s = |e: &str| -> String { lua(&rt, e) };
    let n = |e: &str| -> f64 { lua(&rt, e) };
    assert_eq!(
        s("cena.objects.moeda.shape.color"),
        "#FF0000",
        "mudança da instância"
    );
    assert_eq!(
        s("cena.criada.shape.color"),
        "#FFFF00",
        "o original do prefab"
    );
    assert_eq!(n("cena.objects.moeda.x"), -20.0);
    assert_eq!(n("cena.criada.x"), 100.0);
    assert_eq!(s("cena.criada.name"), "nova");
    assert_eq!(s("cena.objects.moeda.children[1].name"), "brilho");
    assert!(n("cena.criada.children[1].id") < 0.0, "ids só da execução");
    // Comportamento do jogo: propriedade da instância, start e mensagens.
    let r1 = n("cena.objects.moeda.rotation");
    let r2 = n("cena.criada.rotation");
    assert!(
        (r1 - 90.0).abs() < 10.0,
        "gira a 180 por meio segundo: {r1}"
    );
    assert!((r2 - 45.0).abs() < 10.0, "gira a 90 por meio segundo: {r2}");
    assert!(lua::<bool>(
        &rt,
        "cena.objects.moeda:behavior('scripts/gira.lua').comecou"
    ));
    assert_eq!(s("cena.objects.moeda:send('ola', 3)"), "ola 3");
}

#[test]
fn atirar_gerador_e_destruir_depois() {
    let dir = projeto(
        "atirar",
        &[
            (
                "prefabs/tiro.prefab",
                r##"return { object = { id = 1, name = "tiro", components = {
  { type = "shape", w = 4, h = 4 },
  { type = "behavior", script = "nn:temporario", tempo = 0.25, fora_da_tela = false },
} } }"##,
            ),
            (
                "cenas/fase.cena",
                r##"return { objects = {
  { id = 1, name = "canhao", x = 0, y = 0, components = {
    { type = "behavior", script = "nn:atirar", prefab = "prefabs/tiro.prefab", tecla = "", intervalo = 0.1, velocidade = 100, direcao = "direita" },
  } },
  { id = 2, name = "ninho", x = 0, y = 50, components = {
    { type = "behavior", script = "nn:gerador", prefab = "prefabs/tiro.prefab", intervalo = 0.05, maximo = 2, comecar_ja = true },
  } },
} }"##,
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/fase.cena") else {
        return;
    };
    rt.lua()
        .globals()
        .set("cena", h.game.0.table().unwrap())
        .unwrap();
    h.run_frames(60);
    assert_eq!(h.game.0.error(), None);
    let tiros: i64 = lua(&rt, "#cena:find_all('tiro')");
    // Canhão: um a cada 0,1 s, cada um dura 0,25 s (2 ou 3 vivos). Ninho: até 2.
    assert!((3..=6).contains(&tiros), "{tiros} tiros");
    // Tiros sem corpo físico andam pela velocidade (vx).
    let maior_x: f64 = lua(
        &rt,
        "(function() local m = 0 for _, t in ipairs(cena:find_all('tiro')) do m = math.max(m, t.x) end return m end)()",
    );
    assert!(maior_x > 10.0, "{maior_x}");
}

#[test]
fn ir_para_cena_e_movimento_automatico() {
    let dir = projeto(
        "porta",
        &[
            (
                "cenas/fim.lua",
                "local c = {} function c:enter() chegou = true end return c",
            ),
            ("cenas/obs.lua", OBSERVA),
            (
                "cenas/fase.cena",
                r##"return { script = "cenas/obs.lua", objects = {
  { id = 1, name = "heroi", x = 0, y = 0, components = {
    { type = "shape", w = 10, h = 10 },
    { type = "behavior", script = "nn:topdown", velocidade = 200 },
  } },
  { id = 2, name = "porta", x = 0, y = 0, components = {
    { type = "shape", w = 10, h = 10 },
    { type = "behavior", script = "nn:movimento", tipo = "ir_e_voltar", distancia_x = 60, duracao = 1 },
    { type = "behavior", script = "nn:ir_para_cena", cena = "cenas/fim.lua", escurecer = 0 },
  } },
} }"##,
            ),
        ],
    );
    let Some((mut h, rt)) = pilha(&dir, "cenas/fase.cena") else {
        return;
    };
    // A porta começa em cima do herói: a troca de cena acontece logo.
    h.run_frames(5);
    assert!(lua::<bool>(&rt, "chegou == true"), "foi para a outra cena");
}

#[test]
fn propriedade_do_comportamento_muda_ao_vivo() {
    let dir = projeto(
        "ao-vivo",
        &[
            (
                "scripts/gira.lua",
                r#"return { props = { velocidade = 0 },
  update = function(self, dt) self.obj.rotation = self.obj.rotation + self.velocidade * dt end }"#,
            ),
            (
                "cenas/roda-ao-vivo.cena",
                r##"return { objects = {
  { id = 7, name = "roda", components = {
    { type = "shape" },
    { type = "behavior", script = "scripts/gira.lua" },
  } },
} }"##,
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/roda-ao-vivo.cena") else {
        return;
    };
    // (Cena com nome só deste teste: os outros, rodando junto, não pegam a
    // mensagem ao vivo.)
    rt.lua()
        .globals()
        .set("cena", h.game.0.table().unwrap())
        .unwrap();
    h.run_frames(5);
    assert_eq!(lua::<f64>(&rt, "cena.objects.roda.rotation"), 0.0);
    assert!(nn_script::live::push(
        r#"{"scene":"cenas/roda-ao-vivo.cena","id":7,"component":2,"field":"velocidade","value":120}"#
    ));
    h.run_frames(30);
    let r: f64 = lua(&rt, "cena.objects.roda.rotation");
    assert!((r - 60.0).abs() < 6.0, "{r}");
}

#[test]
fn interagir_inventario_e_zona_no_2d() {
    let dir = projeto(
        "interagir",
        &[
            (
                "cenas/obs.lua",
                "local c = {} function c:enter() C = self end return c",
            ),
            (
                "cenas/fase.cena",
                r##"return { script = "cenas/obs.lua", objects = {
  { id = 1, name = "heroi", x = 0, y = 0, components = {
    { type = "shape", w = 10, h = 10, color = "#FF0000" },
    { type = "behavior", script = "nn:topdown", velocidade = 300 },
  } },
  { id = 2, name = "pocao", x = 40, y = 0, components = {
    { type = "shape", w = 10, h = 10, color = "#FF00FF" },
    { type = "behavior", script = "nn:coletavel", variavel = "", item = "poção" },
  } },
  { id = 3, name = "bau", x = 0, y = 60, components = {
    { type = "shape", w = 20, h = 20, color = "#996633" },
    { type = "behavior", script = "nn:interagir", texto = "abrir", item = "poção", alvo = "porta", mensagem = "sumir", distancia = 10 },
  } },
  { id = 4, name = "porta", x = 300, y = 0, components = {
    { type = "shape", w = 20, h = 40, color = "#FFFFFF" },
    { type = "behavior", script = "sumir.lua" },
  } },
  { id = 5, name = "zona", x = -60, y = 0, components = {
    { type = "shape", w = 30, h = 30, color = "#00FFFF40" },
    { type = "behavior", script = "nn:gatilho", alvo = "porta", mensagem = "piscar", uma_vez = true },
  } },
  { id = 6, name = "itens", components = { { type = "behavior", script = "nn:inventario" } } },
} }"##,
            ),
            (
                "sumir.lua",
                "local B = {} function B:sumir() self.obj:destroy() end function B:piscar(q) PISCOU = (PISCOU or 0) + 1; QUEM = q.name end return B",
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/fase.cena") else {
        return;
    };
    h.run_frames(3);
    assert_eq!(h.game.0.error(), None);
    // A zona some no jogo.
    assert!(!lua::<bool>(&rt, "C.objects.zona.visible"));
    // Entrar na zona avisa a porta (só uma vez).
    for _ in 0..2 {
        rt.lua()
            .load("C.objects.heroi:move_to(-60, 0)")
            .exec()
            .unwrap();
        h.run_frames(3);
        rt.lua()
            .load("C.objects.heroi:move_to(0, -100)")
            .exec()
            .unwrap();
        h.run_frames(3);
    }
    assert_eq!(lua::<i64>(&rt, "PISCOU or 0"), 1);
    assert_eq!(lua::<String>(&rt, "QUEM"), "heroi");
    // Perto do baú sem a poção: o aviso pede o item; E não faz nada.
    rt.lua()
        .load("C.objects.heroi:move_to(0, 42)")
        .exec()
        .unwrap();
    h.run_frames(3);
    assert_eq!(lua::<String>(&rt, "C._ia.text"), "Precisa de: poção");
    assert!(lua::<bool>(&rt, "C._ia.locked"));
    segurar(&mut h, KeyCode::KeyE, 2);
    assert!(lua::<bool>(&rt, "C.objects.porta ~= nil"));
    // Pega a poção (vai para o inventário) e volta: E abre e gasta.
    rt.lua()
        .load("C.objects.heroi:move_to(40, 0)")
        .exec()
        .unwrap();
    h.run_frames(3);
    assert_eq!(lua::<i64>(&rt, "nn.inventory.count('poção')"), 1);
    rt.lua()
        .load("C.objects.heroi:move_to(0, 42)")
        .exec()
        .unwrap();
    h.run_frames(3);
    assert_eq!(lua::<String>(&rt, "C._ia.text"), "abrir");
    segurar(&mut h, KeyCode::KeyE, 2);
    assert!(
        lua::<bool>(&rt, "C.objects.porta == nil"),
        "a mensagem chegou"
    );
    assert_eq!(lua::<i64>(&rt, "nn.inventory.count('poção')"), 0);
    // Longe de tudo: nenhum aviso.
    rt.lua()
        .load("C.objects.heroi:move_to(0, -200)")
        .exec()
        .unwrap();
    h.run_frames(3);
    assert!(lua::<bool>(&rt, "C._ia == nil"));
    assert_eq!(h.game.0.error(), None);
}

#[test]
fn inventario_guarda_tira_e_volta_com_as_variaveis() {
    let dir = projeto(
        "inventario",
        &[("cenas/vazia.cena", "return { objects = {} }")],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/vazia.cena") else {
        return;
    };
    h.run_frames(1);
    let l = rt.lua();
    assert_eq!(
        l.load("return nn.inventory.add('chave', 1, 'chave.png')")
            .eval::<i64>()
            .unwrap(),
        1
    );
    assert_eq!(
        l.load("return nn.inventory.add('chave', 2)")
            .eval::<i64>()
            .unwrap(),
        3
    );
    l.load("nn.inventory.add('poção')").exec().unwrap();
    assert!(
        l.load("return nn.inventory.has('chave', 3)")
            .eval::<bool>()
            .unwrap()
    );
    assert!(
        !l.load("return nn.inventory.has('chave', 4)")
            .eval::<bool>()
            .unwrap()
    );
    // Não tira se não tiver tudo.
    assert!(
        !l.load("return nn.inventory.remove('chave', 5)")
            .eval::<bool>()
            .unwrap()
    );
    assert!(
        l.load("return nn.inventory.remove('chave', 3)")
            .eval::<bool>()
            .unwrap()
    );
    let itens: String = l
        .load("local t = {} for _, i in ipairs(nn.inventory.items()) do t[#t+1] = i.item .. i.count end return table.concat(t, ',')")
        .eval()
        .unwrap();
    assert_eq!(itens, "poção1");
    // Mora em nn.vars: nn.reset_vars esvazia.
    l.load("nn.reset_vars()").exec().unwrap();
    assert_eq!(
        l.load("return nn.inventory.count('poção')")
            .eval::<i64>()
            .unwrap(),
        0
    );
    let e = l
        .load("nn.inventory.add('')")
        .exec()
        .unwrap_err()
        .to_string();
    assert!(e.contains("o item é um nome"), "{e}");
}
