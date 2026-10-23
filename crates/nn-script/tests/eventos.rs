//! Eventos "Quando... Então..." das cenas visuais, rodando a engine sem
//! janela.

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Context, Game, Headless, Scene, SceneStack};
use nn_input::KeyCode;
use nn_script::{EVENTS_RUNTIME, LuaScene, SCENE_RUNTIME, ScriptRuntime};

fn projeto(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-eventos-{}-{name}", std::process::id()));
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

/// Uma cena só, guardada na global `cena` do Lua.
fn cena(dir: &Path, path: &str) -> Option<(Headless<Uma>, ScriptRuntime)> {
    let rt = ScriptRuntime::new(dir).unwrap();
    let rt2 = rt.clone();
    let h = headless(dir, |ctx| {
        let mut s = rt.scene(path);
        s.enter(ctx);
        Uma(s)
    })?;
    rt2.lua()
        .globals()
        .set("cena", h.game.0.table().unwrap())
        .unwrap();
    Some((h, rt2))
}

fn pilha(dir: &Path, path: &str) -> Option<(Headless<SceneStack>, ScriptRuntime)> {
    let rt = ScriptRuntime::new(dir).unwrap();
    let rt2 = rt.clone();
    let path = path.to_string();
    let h = headless(dir, move |ctx| SceneStack::new(ctx, rt.scene(path)))?;
    Some((h, rt2))
}

fn lua<T: mlua::FromLua>(rt: &ScriptRuntime, expr: &str) -> T {
    rt.lua().load(format!("return {expr}")).eval().unwrap()
}

fn apertar(h: &mut Headless<impl Game>, k: KeyCode) {
    h.ctx.input.press_key(k);
    h.run_frames(1);
    h.ctx.input.release_key(k);
    h.run_frames(1);
}

#[test]
fn catalogo_se_descreve_sem_a_engine_e_as_frases_tem_os_campos() {
    let lua = mlua::Lua::new();
    let m: mlua::Table = lua.load(SCENE_RUNTIME).eval().unwrap();
    let e: mlua::Table = lua.load(EVENTS_RUNTIME).call(m).unwrap();
    let catalog: mlua::Table = e
        .get::<mlua::Function>("catalog")
        .unwrap()
        .call(())
        .unwrap();
    for list in ["conditions", "actions"] {
        let items: Vec<mlua::Table> = catalog
            .get::<mlua::Table>(list)
            .unwrap()
            .sequence_values()
            .map(Result::unwrap)
            .collect();
        assert!(items.len() >= 15, "{list}: {}", items.len());
        for item in items {
            let id: String = item.get("id").unwrap();
            let text: String = item.get("text").unwrap();
            let category: String = item.get("category").unwrap();
            assert!(!category.is_empty(), "{id}");
            let props: Vec<mlua::Table> = item
                .get::<mlua::Table>("props")
                .unwrap()
                .sequence_values()
                .map(Result::unwrap)
                .collect();
            // Cada {campo} da frase é uma propriedade; propriedades fora da
            // frase só podem ser avançadas. As só do 3D ficam na frase do 3D.
            let text3d: Option<String> = item.get("text3d").unwrap();
            for p in &props {
                let key: String = p.get("key").unwrap();
                let advanced: bool = p.get::<Option<bool>>("advanced").unwrap().unwrap_or(false);
                let mode: Option<String> = p.get("mode").unwrap();
                let field = format!("{{{key}}}");
                let in_text = if mode.as_deref() == Some("3d") {
                    text3d.as_deref().is_some_and(|t| t.contains(&field))
                } else {
                    text.contains(&field)
                };
                assert!(in_text || advanced, "{list} {id}: {key} fora da frase");
            }
            let mut rest = text.as_str();
            while let Some(i) = rest.find('{') {
                let j = rest[i..].find('}').unwrap() + i;
                let key = &rest[i + 1..j];
                assert!(
                    props.iter().any(|p| p.get::<String>("key").unwrap() == key),
                    "{list} {id}: {{{key}}} sem propriedade"
                );
                rest = &rest[j + 1..];
            }
        }
    }
}

const HEROI: &str = r#"{ id = 1, name = "heroi", x = 0, y = 0,
  components = { { type = "shape", w = 20, h = 20 }, { type = "behavior", script = "nn:topdown", velocidade = 0 } } }"#;

#[test]
fn regras_da_cena_comecar_tecla_tempo_e_variavel() {
    let cena_txt = format!(
        r#"return {{
  events = {{
    {{ when = {{ {{ cond = "comecou" }} }}, actions = {{ {{ act = "variavel", var = "pontos", op = "definir", valor = 10 }} }} }},
    {{ when = {{ {{ cond = "tecla", modo = "apertar", tecla = "x" }} }}, actions = {{ {{ act = "variavel", var = "pontos", op = "somar", valor = 5 }} }} }},
    {{ when = {{ {{ cond = "a_cada", segundos = 0.5 }} }}, actions = {{ {{ act = "variavel", var = "tiques", op = "somar", valor = 1 }} }} }},
    {{ comment = "Quando passa a valer: só uma vez" }},
    {{ when = {{ {{ cond = "variavel", var = "pontos", op = ">=", valor = 15 }} }},
      actions = {{ {{ act = "variavel", var = "premios", op = "somar", valor = 1 }} }} }},
    {{ mode = "enquanto", when = {{ {{ cond = "variavel", var = "pontos", op = ">=", valor = 15 }} }},
      actions = {{ {{ act = "variavel", var = "quadros", op = "somar", valor = 1 }} }} }},
    {{ off = true, when = {{ {{ cond = "comecou" }} }}, actions = {{ {{ act = "variavel", var = "desligada", op = "definir", valor = 1 }} }} }},
    {{ comment = "Uma regra com nome roda", when = {{ {{ cond = "comecou" }} }}, actions = {{ {{ act = "variavel", var = "com_nome", op = "definir", valor = 1 }} }} }},
    {{ when = {{ {{ cond = "tecla", modo = "apertar", tecla = "z" }}, {{ cond = "variavel", var = "pontos", op = ">", valor = 100, nao = true }} }},
      actions = {{ {{ act = "variavel", var = "texto", op = "definir", valor = "tem {{pontos}}" }} }} }},
  }},
  objects = {{ {HEROI} }},
}}"#
    );
    let dir = projeto("cena", &[("cenas/regras.cena", &cena_txt)]);
    let Some((mut h, rt)) = cena(&dir, "cenas/regras.cena") else {
        return;
    };
    h.run_frames(2);
    assert_eq!(lua::<f64>(&rt, "nn.vars.pontos"), 10.0);
    assert_eq!(lua::<Option<f64>>(&rt, "nn.vars.desligada"), None);
    assert_eq!(
        lua::<f64>(&rt, "nn.vars.com_nome"),
        1.0,
        "regra com comment roda"
    );
    apertar(&mut h, KeyCode::KeyX);
    assert_eq!(lua::<f64>(&rt, "nn.vars.pontos"), 15.0);
    h.run_frames(10);
    assert_eq!(
        lua::<f64>(&rt, "nn.vars.premios"),
        1.0,
        "Quando: uma vez só"
    );
    assert!(
        lua::<f64>(&rt, "nn.vars.quadros") >= 10.0,
        "Enquanto: todo quadro"
    );
    apertar(&mut h, KeyCode::KeyX);
    h.run_frames(5);
    assert_eq!(lua::<f64>(&rt, "nn.vars.premios"), 1.0);
    apertar(&mut h, KeyCode::KeyZ);
    assert_eq!(lua::<String>(&rt, "nn.vars.texto"), "tem 20");
    h.run_frames(60);
    let tiques: f64 = lua(&rt, "nn.vars.tiques");
    assert!((2.0..=4.0).contains(&tiques), "{tiques}");
}

#[test]
fn encostar_pegar_o_outro_e_acabaram_os_objetos() {
    let cena_txt = format!(
        r##"return {{
  events = {{
    {{ when = {{ {{ cond = "encostar", quem = "@jogador", em = "chave" }} }},
      actions = {{ {{ act = "variavel", var = "chaves", op = "somar", valor = 1 }}, {{ act = "destruir", alvo = "@outro" }} }} }},
    {{ when = {{ {{ cond = "acabaram", nome = "chave" }} }},
      actions = {{ {{ act = "mostrar", acao = "mostrar", alvo = "porta" }}, {{ act = "cor", alvo = "porta", cor = "#00FF00" }} }} }},
  }},
  objects = {{
    {HEROI},
    {{ id = 2, name = "chave", x = 100, components = {{ {{ type = "shape", w = 10, h = 10 }} }} }},
    {{ id = 3, name = "chave", x = 200, components = {{ {{ type = "shape", w = 10, h = 10 }} }} }},
    {{ id = 4, name = "porta", x = 400, visible = false, components = {{ {{ type = "shape", w = 10, h = 10 }} }} }},
  }},
}}"##
    );
    let dir = projeto("encostar", &[("cenas/chaves.cena", &cena_txt)]);
    let Some((mut h, rt)) = cena(&dir, "cenas/chaves.cena") else {
        return;
    };
    h.run_frames(3);
    assert_eq!(lua::<Option<f64>>(&rt, "nn.vars.chaves"), None);
    rt.lua()
        .load("cena.objects.heroi:move_to(100, 0)")
        .exec()
        .unwrap();
    h.run_frames(2);
    assert_eq!(lua::<f64>(&rt, "nn.vars.chaves"), 1.0);
    assert_eq!(lua::<usize>(&rt, "#cena:find_all('chave')"), 1);
    assert!(!lua::<bool>(&rt, "cena.objects.porta.visible"));
    rt.lua()
        .load("cena.objects.heroi:move_to(200, 0)")
        .exec()
        .unwrap();
    h.run_frames(2);
    assert_eq!(lua::<f64>(&rt, "nn.vars.chaves"), 2.0);
    assert!(lua::<bool>(&rt, "cena.objects.porta.visible"));
    assert_eq!(
        lua::<String>(&rt, "cena.objects.porta.shape.color"),
        "#00FF00"
    );
}

#[test]
fn regras_do_objeto_vao_no_prefab_com_esperar_e_destruido() {
    let dir = projeto(
        "prefab",
        &[
            (
                "prefabs/bomba.prefab",
                r#"return { format = 1, object = {
  id = 1, name = "bomba",
  components = { { type = "shape", w = 10, h = 10 } },
  events = {
    { when = { { cond = "comecou" } }, actions = {
      { act = "variavel", var = "bombas", op = "somar", valor = 1 },
      { act = "esperar", segundos = 0.5 },
      { act = "destruir", alvo = "@este" } } },
    { when = { { cond = "destruido", quem = "@este" } }, actions = {
      { act = "variavel", var = "explosoes", op = "somar", valor = 1 } } },
  },
} }"#,
            ),
            (
                "cenas/bombas.cena",
                r#"return {
  events = {
    { when = { { cond = "tecla", modo = "apertar", tecla = "b" } },
      actions = { { act = "criar", prefab = "prefabs/bomba.prefab", onde = "heroi", dx = 30 } } },
  },
  objects = {
    { id = 1, name = "heroi", components = { { type = "shape" } } },
    { id = 2, name = "bomba_da_cena", prefab = "prefabs/bomba.prefab", x = -50 },
  },
}"#,
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/bombas.cena") else {
        return;
    };
    h.run_frames(2);
    assert_eq!(lua::<f64>(&rt, "nn.vars.bombas"), 1.0);
    apertar(&mut h, KeyCode::KeyB);
    apertar(&mut h, KeyCode::KeyB);
    h.run_frames(1);
    assert_eq!(lua::<f64>(&rt, "nn.vars.bombas"), 3.0);
    assert_eq!(lua::<usize>(&rt, "#cena:find_all('bomba')"), 2);
    assert_eq!(lua::<f64>(&rt, "cena:find_all('bomba')[1].x"), 30.0);
    h.run_frames(40);
    assert_eq!(lua::<usize>(&rt, "#cena:find_all('bomba')"), 0);
    assert!(!lua::<bool>(&rt, "cena.objects.bomba_da_cena ~= nil"));
    assert_eq!(lua::<f64>(&rt, "nn.vars.explosoes"), 3.0);
}

#[test]
fn vida_acabou_e_botao_levam_para_outra_cena() {
    let dir = projeto(
        "vida",
        &[
            (
                "cenas/jogo.cena",
                r#"return {
  events = {
    { when = { { cond = "tecla", modo = "apertar", tecla = "d" } },
      actions = { { act = "mensagem", msg = "dano", alvo = "@jogador" } } },
    { when = { { cond = "levou_dano", quem = "@jogador" } },
      actions = { { act = "variavel", var = "danos", op = "somar", valor = 1 } } },
    { when = { { cond = "vida_acabou", quem = "@jogador" } },
      actions = { { act = "ir_para_cena", cena = "cenas/fim.cena", escurecer = 0 } } },
  },
  objects = {
    { id = 1, name = "heroi", components = { { type = "shape" },
      { type = "behavior", script = "nn:topdown" },
      { type = "behavior", script = "nn:vida", vidas = 2, invencivel = 0, ao_acabar = "nada", variavel = "" } } },
  },
}"#,
            ),
            (
                "cenas/fim.cena",
                r#"return {
  events = { { when = { { cond = "comecou" } }, actions = { { act = "variavel", var = "no_fim", op = "definir", valor = 1 } } },
             { when = { { cond = "clicar", quem = "jogar" } }, actions = { { act = "variavel", var = "clicou", op = "definir", valor = 1 } } } },
  objects = {
    { id = 1, name = "jogar", components = { { type = "text", text = "Jogar", hud = true },
      { type = "behavior", script = "nn:botao", foco = true, acao = "cena", cena = "cenas/jogo.cena" } } },
  },
}"#,
            ),
        ],
    );
    let Some((mut h, rt)) = pilha(&dir, "cenas/jogo.cena") else {
        return;
    };
    h.run_frames(2);
    apertar(&mut h, KeyCode::KeyD);
    assert_eq!(lua::<f64>(&rt, "nn.vars.danos"), 1.0);
    apertar(&mut h, KeyCode::KeyD);
    h.run_frames(3);
    assert_eq!(lua::<f64>(&rt, "nn.vars.danos"), 2.0);
    assert_eq!(lua::<f64>(&rt, "nn.vars.no_fim"), 1.0);
    // Botão com foco: Enter aperta, avisa o evento e vai para a cena.
    apertar(&mut h, KeyCode::Enter);
    assert_eq!(lua::<f64>(&rt, "nn.vars.clicou"), 1.0);
    h.run_frames(40);
    apertar(&mut h, KeyCode::KeyD);
    assert_eq!(lua::<f64>(&rt, "nn.vars.danos"), 3.0, "voltou para o jogo");
}

#[test]
fn regras_mudam_ao_vivo_e_o_estado_conta_quais_dispararam() {
    let dir = projeto(
        "ao-vivo",
        &[(
            "cenas/viva.cena",
            r#"return { objects = { { id = 5, name = "luz", components = { { type = "shape" } } } } }"#,
        )],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/viva.cena") else {
        return;
    };
    h.run_frames(2);
    assert!(nn_script::live::push(
        r#"{"scene":"cenas/viva.cena","id":0,"field":"events","value":[
          {"when":[{"cond":"tecla","modo":"apertar","tecla":"k"}],
           "actions":[{"act":"variavel","var":"k","op":"somar","valor":1}]}]}"#
    ));
    assert!(nn_script::live::push(
        r#"{"scene":"cenas/viva.cena","id":5,"field":"events","value":[
          {"mode":"enquanto","when":[{"cond":"tecla","modo":"segurar","tecla":"k"}],
           "actions":[{"act":"mostrar","acao":"esconder","alvo":"@este"}]}]}"#
    ));
    h.run_frames(1);
    apertar(&mut h, KeyCode::KeyK);
    assert_eq!(lua::<f64>(&rt, "nn.vars.k"), 1.0);
    assert!(!lua::<bool>(&rt, "cena.objects.luz.visible"));
    // O estado mandado ao editor diz quais regras dispararam.
    let m: mlua::Table = rt.lua().named_registry_value("nn.cena").unwrap();
    let state: mlua::Table = m
        .get::<mlua::Function>("state")
        .unwrap()
        .call(h.game.0.table().unwrap())
        .unwrap();
    let fired: Vec<String> = state
        .get::<mlua::Table>("fired")
        .unwrap()
        .sequence_values()
        .map(Result::unwrap)
        .collect();
    assert_eq!(fired, vec!["5:1".to_string(), "c:1".to_string()]);
}
