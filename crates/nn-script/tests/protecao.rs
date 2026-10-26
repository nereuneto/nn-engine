//! Erros nos comportamentos não derrubam a cena, e o Lua dos jogos não mexe
//! no computador de quem joga.

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Context, Game, Headless, Scene};
use nn_script::{LuaScene, Project, ScriptRuntime};

fn projeto(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-protecao-{}-{name}", std::process::id()));
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

#[test]
fn um_comportamento_com_erro_e_desligado_e_a_cena_segue() {
    let dir = projeto(
        "comportamento",
        &[
            (
                "comps/conta.lua",
                "local B = {}\nfunction B:update(dt) contagem = (contagem or 0) + 1 end\nreturn B\n",
            ),
            (
                "comps/quebra.lua",
                "local B = {}\nfunction B:update(dt)\n  quebrou = (quebrou or 0) + 1\n  local x = nil\n  x.y = 1\nend\nreturn B\n",
            ),
            (
                "comps/ai.lua",
                "local B = {}\nfunction B:dano(n)\n  error(\"ai \" .. n)\nend\nreturn B\n",
            ),
            (
                "cenas/fase.lua",
                "local c = {}\nfunction c:update(dt)\n  resposta = self.objects.ferido:send(\"dano\", 3)\n  enviados = (enviados or 0) + 1\nend\nreturn c\n",
            ),
            (
                "cenas/fase.cena",
                r#"return { script = "cenas/fase.lua", objects = {
  { id = 1, name = "bom", components = { { type = "behavior", script = "comps/conta.lua" } } },
  { id = 2, name = "ruim", components = { { type = "behavior", script = "comps/quebra.lua" } } },
  { id = 3, name = "ferido", components = { { type = "behavior", script = "comps/ai.lua" } } },
} }"#,
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/fase.cena") else {
        return;
    };
    h.run_frames(10);
    // A cena não parou: o outro comportamento e o script da cena seguem.
    assert_eq!(h.game.0.error(), None);
    assert_eq!(lua::<i64>(&rt, "contagem"), 10);
    assert_eq!(lua::<i64>(&rt, "enviados"), 10);
    // O que deu erro rodou uma vez só (foi desligado), e quem pediu recebeu nil.
    assert_eq!(lua::<i64>(&rt, "quebrou"), 1);
    assert!(lua::<bool>(&rt, "resposta == nil"));
    let errors = rt.take_errors();
    assert_eq!(errors.len(), 2, "{errors:#?}");
    let ruim = &errors[0].message;
    assert!(
        ruim.contains("comps/quebra.lua do objeto \"ruim\""),
        "{ruim}"
    );
    assert!(ruim.contains("desligado"), "{ruim}");
    assert!(ruim.contains("comps/quebra.lua:5:"), "{ruim}");
    // A mensagem do Lua já em português, com o caminho até o erro.
    assert!(ruim.contains("tentou"), "{ruim}");
    assert!(ruim.contains("caminho até o erro"), "{ruim}");
    let ferido = &errors[1].message;
    assert!(ferido.contains("(em dano)"), "{ferido}");
    assert!(ferido.contains("comps/ai.lua:3: ai 3"), "{ferido}");
    assert_eq!(errors[1].scene, "cenas/fase.cena");
}

#[test]
fn erro_ao_carregar_e_no_start_tambem_so_desligam() {
    let dir = projeto(
        "carregar",
        &[
            (
                "comps/sintaxe.lua",
                "local B = {\nfunction B:update( end\nreturn B\n",
            ),
            (
                "comps/start.lua",
                "local B = {}\nfunction B:start() error(\"no start\") end\nfunction B:update() rodou = true end\nreturn B\n",
            ),
            (
                "comps/conta.lua",
                "local B = {}\nfunction B:update(dt) contagem = (contagem or 0) + 1 end\nreturn B\n",
            ),
            (
                "cenas/fase.cena",
                r#"return { objects = {
  { id = 1, name = "a", components = { { type = "behavior", script = "comps/sintaxe.lua" } } },
  { id = 2, name = "b", components = { { type = "behavior", script = "comps/start.lua" } } },
  { id = 3, name = "c", components = { { type = "behavior", script = "comps/conta.lua" } } },
} }"#,
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "cenas/fase.cena") else {
        return;
    };
    h.run_frames(3);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(lua::<i64>(&rt, "contagem"), 3);
    assert!(
        lua::<bool>(&rt, "rodou == nil"),
        "o do start com erro ficou desligado"
    );
    let errors: Vec<String> = rt.take_errors().into_iter().map(|e| e.message).collect();
    assert_eq!(errors.len(), 2, "{errors:#?}");
    assert!(
        errors[0].contains("carregar o comportamento comps/sintaxe.lua"),
        "{errors:#?}"
    );
    assert!(errors[0].contains("comps/sintaxe.lua:2"), "{errors:#?}");
    assert!(errors[1].contains("(em start)"), "{errors:#?}");
}

#[test]
fn no_desenvolvimento_o_erro_aparece_por_cima_do_jogo() {
    let dir = projeto(
        "tela",
        &[
            (
                "comps/quebra.lua",
                "local B = {}\nfunction B:update() error(\"bum\") end\nreturn B\n",
            ),
            (
                "cenas/fase.cena",
                r#"return { objects = {
  { id = 1, name = "a", components = { { type = "behavior", script = "comps/quebra.lua" } } },
} }"#,
            ),
            ("cenas/vazia.cena", "return { objects = {} }"),
        ],
    );
    let Some((mut h, _)) = cena(&dir, "cenas/vazia.cena") else {
        return;
    };
    h.run_frames(2);
    let limpa = h.pixels().unwrap();
    assert_eq!(&limpa[limpa.len() - 4..limpa.len() - 1], [0, 0, 0]);
    let Some((mut h, _)) = cena(&dir, "cenas/fase.cena") else {
        return;
    };
    h.run_frames(2);
    // A faixa do erro embaixo, por cima do fundo preto.
    let p = h.pixels().unwrap();
    let canto = &p[p.len() - 4..p.len() - 1];
    assert!(canto != [0, 0, 0], "{canto:?}");
}

#[test]
fn o_lua_dos_jogos_nao_mexe_no_computador() {
    let dir = projeto("lua", &[("main.lua", "return {}")]);
    let rt = ScriptRuntime::new(&dir).unwrap();
    let erro = |code: &str| -> String {
        let (ok, msg): (bool, String) = rt
            .lua()
            .load(format!("return pcall(function() {code} end)"))
            .eval()
            .unwrap();
        assert!(!ok, "{code} funcionou");
        msg
    };
    assert!(erro("return io.open('x')").contains("nn.assets.read"));
    assert!(erro("io.write('oi')").contains("io.write não existe"));
    assert!(erro("os.execute('ls')").contains("os.execute não existe"));
    assert!(erro("os.remove('x')").contains("não mexe nos arquivos"));
    assert!(erro("os.exit()").contains("nn.quit()"));
    assert!(erro("os.getenv('HOME')").contains("os.time"));
    assert!(erro("dofile('x.lua')").contains("require"));
    assert!(erro("loadfile('x.lua')").contains("require"));
    // O que fica.
    assert!(lua::<bool>(
        &rt,
        "type(os.time()) == 'number' and type(os.clock()) == 'number'"
    ));
    assert!(lua::<bool>(
        &rt,
        "os.date('%Y') ~= nil and os.difftime(2, 1) == 1"
    ));
    assert!(lua::<bool>(
        &rt,
        "require('os') == os and package.loadlib == nil"
    ));
    assert_eq!(lua::<i64>(&rt, "load('return 1 + 1')()"), 2);
    assert_eq!(
        lua::<i64>(&rt, "load('return x', 'c', 't', { x = 5 })()"),
        5
    );
    // Um trecho já compilado (que pode quebrar a memória) não carrega.
    let (f, msg): (mlua::Value, String) = rt
        .lua()
        .load("return load(string.dump(function() return 1 end))")
        .eval()
        .unwrap();
    assert!(f.is_nil() && msg.contains("binary"), "{msg}");
}

#[test]
fn o_config_lua_tambem_roda_sem_o_os() {
    let dir = projeto(
        "config",
        &[
            (
                "config.lua",
                "os.execute('echo oi')\nreturn { title = 'x' }",
            ),
            ("main.lua", "return {}"),
        ],
    );
    let e = Project::open(&dir).unwrap_err().to_string();
    assert!(e.contains("os.execute não existe"), "{e}");
}
