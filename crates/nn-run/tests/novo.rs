//! `nn novo`: os projetos criados rodam de primeira.

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Headless, Scene, SceneStack};
use nn_input::KeyCode;
use nn_run::{NewOptions, Template, create_project};
use nn_script::{Project, ScriptRuntime};

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-novo-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn headless(dir: &Path, save_dir: &Path) -> Option<Headless<SceneStack>> {
    let project = Project::open(dir).unwrap();
    let rt = ScriptRuntime::new(dir).unwrap();
    let config: AppConfig = project
        .config
        .clone()
        .size(320, 180)
        .save_dir(save_dir)
        .hot_reload(false);
    let start = project.start.clone();
    match Headless::new(config, move |ctx| SceneStack::new(ctx, rt.scene(start))) {
        Ok(h) => Some(h),
        Err(e) => {
            if std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1") {
                panic!("GPU obrigatória: {e}");
            }
            None
        }
    }
}

fn toque(h: &mut Headless<SceneStack>, k: KeyCode) {
    h.ctx.input.press_key(k);
    h.run_frames(1);
    h.ctx.input.release_key(k);
    h.run_frames(1);
}

#[test]
fn modelo_padrao_cria_um_jogo_que_roda_do_titulo_ao_fim() {
    let base = temp("padrao");
    let dir = base.join("caca-moedas");
    let r = create_project(&NewOptions {
        dir: dir.clone(),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(r.title, "Caca moedas");
    assert_eq!(r.id, "caca-moedas");
    assert!(r.files.contains(&"cenas/jogo.lua".to_string()));
    assert!(dir.join(".gitignore").exists());
    assert!(dir.join("assets/imagens/heroi.png").exists());

    let config = std::fs::read_to_string(dir.join("config.lua")).unwrap();
    assert!(config.contains("title = \"Caca moedas\"") && config.contains("id = \"caca-moedas\""));
    let project = Project::open(&dir).unwrap();
    assert_eq!(project.config.title, "Caca moedas");
    assert_eq!(project.config.save_id(), "caca-moedas");
    assert!(ScriptRuntime::new(&dir).unwrap().check_syntax().is_empty());

    let save_dir = base.join("dados");
    let Some(mut h) = headless(&dir, &save_dir) else {
        return;
    };
    h.run_frames(2);
    assert_eq!(h.game.len(), 1);

    // Título → Jogar (com escurecimento).
    toque(&mut h, KeyCode::Enter);
    assert!(h.game.is_fading());
    h.run_frames(60);
    assert!(!h.game.is_fading());

    // Pausa e volta.
    toque(&mut h, KeyCode::Escape);
    assert_eq!(h.game.len(), 2, "pausou");
    toque(&mut h, KeyCode::Escape);
    assert_eq!(h.game.len(), 1, "despausou");

    // Anda um pouco e deixa o tempo acabar: aparece o fim da partida.
    h.ctx.input.press_key(KeyCode::KeyD);
    h.run_frames(60);
    h.ctx.input.release_key(KeyCode::KeyD);
    h.run_frames(30 * 60);
    assert_eq!(h.game.len(), 2, "fim da partida por cima do jogo");

    // "Jogar de novo" recomeça.
    toque(&mut h, KeyCode::Enter);
    h.run_frames(60);
    assert_eq!(h.game.len(), 1);
    drop(h);
    std::fs::remove_dir_all(&base).unwrap();
}

#[test]
fn modelo_minimo_e_titulo_escolhido() {
    let dir = temp("minimo");
    let r = create_project(&NewOptions {
        dir: dir.clone(),
        title: Some("Meu \"Primeiro\" Jogo".into()),
        template: Template::Minimo,
    })
    .unwrap();
    assert_eq!(r.files.len(), 3);
    let project = Project::open(&dir).unwrap();
    assert_eq!(project.config.title, "Meu \"Primeiro\" Jogo");
    assert_eq!(project.start, "main.lua");
    assert!(ScriptRuntime::new(&dir).unwrap().check_syntax().is_empty());

    let Some(mut h) = headless(&dir, &dir.join("dados")) else {
        return;
    };
    h.run_frames(5);
    assert_eq!(h.game.len(), 1);
    drop(h);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Uma cena só, para ler o estado do script.
struct Uma(nn_script::LuaScene);
impl nn_app::Game for Uma {
    fn fixed_update(&mut self, ctx: &mut nn_app::Context) {
        self.0.fixed_update(ctx);
    }
    fn update(&mut self, ctx: &mut nn_app::Context) {
        let _ = self.0.update(ctx);
    }
    fn render(&mut self, ctx: &mut nn_app::Context) {
        self.0.render(ctx);
    }
}

#[test]
fn modelo_visual_roda_a_fase_e_pega_moedas() {
    let dir = temp("visual");
    let r = create_project(&NewOptions {
        dir: dir.clone(),
        title: None,
        template: Template::Visual,
    })
    .unwrap();
    assert!(r.files.contains(&"cenas/fase.cena".to_string()));
    let project = Project::open(&dir).unwrap();
    assert_eq!(project.start, "cenas/titulo.cena");
    assert!(ScriptRuntime::new(&dir).unwrap().check_syntax().is_empty());

    let rt = ScriptRuntime::new(&dir).unwrap();
    let vars = rt.clone();
    let check = rt.clone();
    let var = move |name: &str| -> i64 {
        vars.lua()
            .load(format!("return nn.vars.{name}"))
            .eval()
            .unwrap()
    };
    let config = project
        .config
        .clone()
        .size(320, 180)
        .save_dir(dir.join("dados"))
        .hot_reload(false);
    let h = Headless::new(config, move |ctx| {
        let mut s = rt.scene("cenas/fase.cena");
        s.enter(ctx);
        Uma(s)
    });
    let mut h = match h {
        Ok(h) => h,
        Err(e) => {
            assert!(
                !std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1"),
                "GPU obrigatória: {e}"
            );
            return;
        }
    };
    h.run_frames(30);
    assert_eq!(h.game.0.error(), None);
    // O chão é um mapa de tiles pintado: o herói para em cima dele (o topo
    // da terra fica em y = -240; o herói tem 60 de altura).
    check
        .lua()
        .globals()
        .set("cena", h.game.0.table().unwrap())
        .unwrap();
    let y = lua::<f64>(&check, "return cena.objects.heroi.y");
    assert!((y - -210.0).abs() < 3.0, "herói no chão de tiles: y = {y}");
    assert_eq!(var("moedas"), 0);
    assert_eq!(var("vidas"), 3);
    // Anda para a direita até a moeda que está no chão. Não tem nenhum
    // código: só comportamentos prontos e um prefab.
    h.ctx.input.press_key(KeyCode::KeyD);
    h.run_frames(120);
    h.ctx.input.release_key(KeyCode::KeyD);
    h.run_frames(5);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(var("moedas"), 1, "pegou a moeda do chão");
    assert_eq!(var("vidas"), 3);
    // Pular do chão solta a poeira nos pés (um evento da cena).
    assert_eq!(
        lua::<i64>(&check, "return cena:find('poeira').particles.count"),
        0
    );
    h.ctx.input.press_key(KeyCode::Space);
    h.run_frames(1);
    h.ctx.input.release_key(KeyCode::Space);
    h.run_frames(1);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(
        lua::<i64>(&check, "return cena:find('poeira').particles.count"),
        10
    );
    drop(h);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn modelo_visual_tem_menus_pausa_e_opcoes() {
    let base = temp("visual-menus");
    let dir = base.join("jogo");
    let r = create_project(&NewOptions {
        dir: dir.clone(),
        title: None,
        template: Template::Visual,
    })
    .unwrap();
    for f in [
        "cenas/pausa.cena",
        "cenas/opcoes.cena",
        "assets/imagens/coracao.png",
    ] {
        assert!(r.files.contains(&f.to_string()), "{f}");
    }
    let Some(mut h) = headless(&dir, &base.join("dados")) else {
        return;
    };
    h.run_frames(3);
    // Título: Jogar já vem com foco; Enter entra na fase.
    toque(&mut h, KeyCode::Enter);
    h.run_frames(60);
    assert_eq!(h.game.len(), 1);
    // Esc: a pausa abre por cima.
    toque(&mut h, KeyCode::Escape);
    h.run_frames(2);
    assert_eq!(h.game.len(), 2, "pausou");
    // Para baixo: Opções; Enter abre as opções por cima da pausa.
    toque(&mut h, KeyCode::ArrowDown);
    toque(&mut h, KeyCode::Enter);
    h.run_frames(2);
    assert_eq!(h.game.len(), 3, "opções por cima da pausa");
    // O foco começa em Voltar; duas vezes para cima chega no volume dos
    // efeitos, e a esquerda abaixa (e guarda).
    toque(&mut h, KeyCode::ArrowUp);
    toque(&mut h, KeyCode::ArrowUp);
    toque(&mut h, KeyCode::ArrowLeft);
    h.run_frames(2);
    let efeitos = h
        .ctx
        .save
        .get("nn_opcoes")
        .and_then(|v| v.get("efeitos"))
        .and_then(|v| v.as_f64());
    assert!(
        efeitos.is_some_and(|v| (v - 0.9).abs() < 1e-6),
        "{efeitos:?}"
    );
    // Esc fecha as opções e depois a pausa: volta à fase.
    toque(&mut h, KeyCode::Escape);
    h.run_frames(2);
    assert_eq!(h.game.len(), 2);
    toque(&mut h, KeyCode::Escape);
    h.run_frames(2);
    assert_eq!(h.game.len(), 1, "voltou para a fase");
    drop(h);
    std::fs::remove_dir_all(&base).unwrap();
}

#[test]
fn nao_sobrescreve_pasta_com_arquivos() {
    let dir = temp("ocupada");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("importante.txt"), "não apague").unwrap();
    let e = create_project(&NewOptions {
        dir: dir.clone(),
        ..Default::default()
    })
    .unwrap_err();
    assert!(e.contains("não está vazia"), "{e}");
    assert_eq!(
        std::fs::read_to_string(dir.join("importante.txt")).unwrap(),
        "não apague"
    );

    // Pasta vazia pode.
    let vazia = temp("vazia");
    std::fs::create_dir_all(&vazia).unwrap();
    assert!(
        create_project(&NewOptions {
            dir: vazia.clone(),
            ..Default::default()
        })
        .is_ok()
    );
    std::fs::remove_dir_all(&dir).unwrap();
    std::fs::remove_dir_all(&vazia).unwrap();
}

/// Uma cena visual do projeto rodando sozinha, com a tabela dela na global
/// `cena` do Lua.
fn cena_do_modelo(dir: &Path, path: &str) -> Option<(Headless<Uma>, ScriptRuntime)> {
    let project = Project::open(dir).unwrap();
    let rt = ScriptRuntime::new(dir).unwrap();
    let rt2 = rt.clone();
    let config = project
        .config
        .clone()
        .size(320, 180)
        .save_dir(dir.join("dados"))
        .hot_reload(false);
    let path = path.to_string();
    let h = Headless::new(config, move |ctx| {
        let mut s = rt.scene(path);
        s.enter(ctx);
        Uma(s)
    });
    match h {
        Ok(h) => {
            rt2.lua()
                .globals()
                .set("cena", h.game.0.table().unwrap())
                .unwrap();
            Some((h, rt2))
        }
        Err(e) => {
            assert!(
                !std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1"),
                "GPU obrigatória: {e}"
            );
            None
        }
    }
}

fn lua<T: mlua::FromLua>(rt: &ScriptRuntime, code: &str) -> T {
    rt.lua().load(code).eval().unwrap()
}

/// Roda a cena inicial e todas as cenas visuais do projeto por uns quadros:
/// nenhuma pode dar erro nem aviso (arquivo que falta). `false` sem GPU.
fn roda_todas_as_cenas(dir: &Path, nome: &str) -> bool {
    assert!(
        ScriptRuntime::new(dir).unwrap().check_syntax().is_empty(),
        "{nome}"
    );
    // A cena inicial e todas as cenas visuais (as cenas em código podem
    // depender do que a anterior preparou).
    let mut scenes = vec![Project::open(dir).unwrap().start];
    if let Ok(list) = std::fs::read_dir(dir.join("cenas")) {
        for e in list.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let rel = format!("cenas/{name}");
            if name.ends_with(".cena") && !scenes.contains(&rel) {
                scenes.push(rel);
            }
        }
    }
    assert!(!scenes.is_empty(), "{nome}");
    for s in scenes {
        let Some((mut h, rt)) = cena_do_modelo(dir, &s) else {
            return false;
        };
        h.run_frames(90);
        assert_eq!(h.game.0.error(), None, "{nome} {s}");
        // Um clique no meio da tela (no 3D, é o que prende o mouse no jogo).
        h.ctx.input.set_cursor(Some(glam::Vec2::new(160.0, 90.0)));
        h.ctx.input.press_mouse(nn_input::MouseButton::Left);
        h.run_frames(1);
        h.ctx.input.release_mouse(nn_input::MouseButton::Left);
        h.run_frames(2);
        assert_eq!(h.game.0.error(), None, "{nome} {s}, depois de um clique");
        let avisos: Vec<String> = rt
            .lua()
            .named_registry_value::<mlua::Table>("nn.cena")
            .ok()
            .and_then(|m| m.get::<Vec<String>>("warnings").ok())
            .unwrap_or_default();
        assert!(avisos.is_empty(), "{nome} {s}: {avisos:?}");
    }
    true
}

/// Ferramenta: um projeto feito em outra versão (por exemplo, com o `nn` da
/// v0.1.0) roda nesta: `NN_PROJETO=<pasta> cargo test -p nn-run --test novo
/// projeto_de_fora -- --ignored`. O projeto é copiado antes (nada muda nele).
#[test]
#[ignore]
fn projeto_de_fora_roda_todas_as_cenas() {
    let Some(src) = std::env::var_os("NN_PROJETO") else {
        return;
    };
    let src = PathBuf::from(src);
    let dir = temp("de-fora");
    fn copiar(a: &Path, b: &Path) {
        std::fs::create_dir_all(b).unwrap();
        for e in std::fs::read_dir(a).unwrap().flatten() {
            let to = b.join(e.file_name());
            if e.file_type().unwrap().is_dir() {
                copiar(&e.path(), &to);
            } else {
                std::fs::copy(e.path(), to).unwrap();
            }
        }
    }
    copiar(&src, &dir);
    roda_todas_as_cenas(&dir, &src.display().to_string());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn todos_os_modelos_rodam_todas_as_cenas() {
    for template in Template::ALL {
        let dir = temp(&format!("todas-{}", template.name()));
        create_project(&NewOptions {
            dir: dir.clone(),
            title: Some("Teste \"Aspas\"".into()),
            template,
        })
        .unwrap();
        if !roda_todas_as_cenas(&dir, template.name()) {
            return;
        }
        // O título vai para dentro das cenas também.
        if template == Template::Aventura {
            let t = std::fs::read_to_string(dir.join("cenas/titulo.cena")).unwrap();
            assert!(t.contains("text = \"Teste \\\"Aspas\\\"\""), "{t}");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[test]
fn modelo_aventura_chave_abre_a_porta_e_o_bau_termina() {
    let dir = temp("aventura");
    create_project(&NewOptions {
        dir: dir.clone(),
        title: None,
        template: Template::Aventura,
    })
    .unwrap();
    let Some((mut h, rt)) = cena_do_modelo(&dir, "cenas/mapa.cena") else {
        return;
    };
    // A grade de caminhos: a parede de tijolos do meio (os tiles sólidos)
    // e o lago não são andáveis; o gramado é.
    h.run_frames(2);
    let anda = |rt: &ScriptRuntime, x: f64, y: f64| -> bool {
        lua(rt, &format!("return cena:nav():walkable({x}, {y})"))
    };
    assert!(!anda(&rt, 96.0, 240.0), "a parede do meio");
    assert!(anda(&rt, -160.0, -160.0), "o gramado");
    assert!(!anda(&rt, -310.0, 0.0), "o lago");
    // Sem inimigos, para o teste não depender de onde eles andam.
    let sem_inimigos = "for _, n in ipairs({ 'lesma', 'morcego' }) do \
        for _, o in ipairs(cena:find_all(n)) do o:destroy() end end";
    rt.lua().load(sem_inimigos).exec().unwrap();
    h.run_frames(2);
    // As paredes são tiles do mapa: andando para a direita, o herói para na
    // parede de tijolos do meio (que começa em x = 72; ele tem 40 de largura).
    h.ctx.input.press_key(KeyCode::KeyD);
    h.run_frames(150);
    h.ctx.input.release_key(KeyCode::KeyD);
    h.run_frames(2);
    let x = lua::<f64>(&rt, "return cena.objects.heroi.x");
    assert!(
        (40.0..=53.0).contains(&x),
        "parou na parede de tiles: x = {x}"
    );
    assert_eq!(lua::<i64>(&rt, "return nn.vars.chaves"), 0);
    // O velho conversa: E perto dele abre a conversa; a escolha "E a
    // chave?" leva à dica, e a conversa termina.
    rt.lua()
        .load("cena.objects.heroi:move_to(-380, -160)")
        .exec()
        .unwrap();
    h.run_frames(3);
    assert_eq!(lua::<String>(&rt, "return cena._ia.text"), "falar");
    let tecla = |h: &mut Headless<Uma>, k: KeyCode| {
        h.ctx.input.press_key(k);
        h.run_frames(1);
        h.ctx.input.release_key(k);
        h.run_frames(2);
    };
    tecla(&mut h, KeyCode::KeyE);
    assert_eq!(
        lua::<String>(&rt, "return cena:dialogue()"),
        "dialogos/velho.dialogo"
    );
    // Enter mostra o texto todo; Enter de novo passa.
    for _ in 0..2 {
        tecla(&mut h, KeyCode::Enter);
    }
    assert_eq!(lua::<i64>(&rt, "return cena._dlg.node.id"), 2);
    tecla(&mut h, KeyCode::Enter);
    tecla(&mut h, KeyCode::Digit1);
    assert_eq!(lua::<i64>(&rt, "return cena._dlg.node.id"), 3);
    tecla(&mut h, KeyCode::Enter);
    tecla(&mut h, KeyCode::Enter);
    assert!(lua::<bool>(&rt, "return cena:dialogue() == nil"));
    assert_eq!(h.game.0.error(), None);
    // Sem chave, a porta avisa.
    rt.lua()
        .load("cena.objects.heroi:move_to(60, 0)")
        .exec()
        .unwrap();
    h.run_frames(3);
    assert!(lua::<bool>(&rt, "return cena.objects.porta ~= nil"));
    assert!(lua::<String>(&rt, "return cena.objects.aviso.text.text").contains("trancada"));
    // Pega a chave e volta à porta: ela abre e gasta a chave.
    rt.lua()
        .load("cena.objects.heroi:move_to(-480, 200)")
        .exec()
        .unwrap();
    h.run_frames(3);
    assert_eq!(lua::<i64>(&rt, "return nn.vars.chaves"), 1);
    rt.lua()
        .load("cena.objects.heroi:move_to(60, 0)")
        .exec()
        .unwrap();
    h.run_frames(3);
    assert!(
        lua::<bool>(&rt, "return cena.objects.porta == nil"),
        "a porta abriu"
    );
    assert_eq!(lua::<i64>(&rt, "return nn.vars.chaves"), 0);
    // O baú abre.
    rt.lua()
        .load("cena.objects.heroi:move_to(430, 0)")
        .exec()
        .unwrap();
    h.run_frames(3);
    assert_eq!(lua::<i64>(&rt, "return cena.objects.bau.sprite.frame"), 2);
    h.run_frames(120);
    assert_eq!(h.game.0.error(), None);
    drop(h);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn modelo_nave_tiro_da_pontos_e_o_recorde_vai_para_o_disco() {
    let dir = temp("nave");
    create_project(&NewOptions {
        dir: dir.clone(),
        title: None,
        template: Template::Nave,
    })
    .unwrap();
    let Some((mut h, rt)) = cena_do_modelo(&dir, "cenas/jogo.cena") else {
        return;
    };
    h.run_frames(2);
    // Sem asteroides sorteados: só o que o teste cria.
    rt.lua()
        .load("cena.objects.gerador:behavior('gerador').enabled = false")
        .exec()
        .unwrap();
    rt.lua()
        .load("for _, o in ipairs(cena:find_all('asteroide')) do o:destroy() end")
        .exec()
        .unwrap();
    // Asteroides sob encomenda: K cria um na frente da nave, J em cima dela
    // (regras mandadas pelo canal ao vivo, como o editor faz).
    assert!(nn_script::live::push(
        r#"{"scene":"cenas/jogo.cena","id":3,"field":"events","value":[
          {"when":[{"cond":"tecla","modo":"apertar","tecla":"k"}],
           "actions":[{"act":"criar","prefab":"prefabs/asteroide.prefab","onde":"nave","dx":420}]},
          {"when":[{"cond":"tecla","modo":"apertar","tecla":"j"}],
           "actions":[{"act":"criar","prefab":"prefabs/asteroide.prefab","onde":"nave"}]}]}"#
    ));
    h.run_frames(1);
    toque_uma(&mut h, KeyCode::KeyK);
    h.ctx.input.press_key(KeyCode::Space);
    h.run_frames(40);
    h.ctx.input.release_key(KeyCode::Space);
    h.run_frames(2);
    assert_eq!(lua::<i64>(&rt, "return nn.vars.pontos"), 10);
    // Três asteroides na nave: a vida acaba e o recorde é salvo.
    for _ in 0..3 {
        toque_uma(&mut h, KeyCode::KeyJ);
        h.run_frames(90);
    }
    assert_eq!(lua::<i64>(&rt, "return nn.vars.vidas"), 0);
    assert_eq!(lua::<i64>(&rt, "return nn.vars.recorde"), 10);
    assert_eq!(h.ctx.save.get_as::<i64>("vars.recorde"), Some(10));
    // Gravado no arquivo (o evento "salvar no disco" grava na hora).
    let path = h.ctx.save.path().unwrap().to_path_buf();
    assert!(!h.ctx.save.is_dirty());
    assert!(
        std::fs::read_to_string(path)
            .unwrap()
            .contains("vars.recorde")
    );
    assert!(lua::<bool>(&rt, "return cena.objects.nave == nil"));
    assert_eq!(h.game.0.error(), None);
    drop(h);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn modelo_visual3d_ilha_sem_codigo_anda_pega_estrelas_e_termina() {
    let dir = temp("visual3d");
    create_project(&NewOptions {
        dir: dir.clone(),
        title: None,
        template: Template::Visual3D,
    })
    .unwrap();
    let Some((mut h, rt)) = cena_do_modelo(&dir, "cenas/ilha.cena") else {
        return;
    };
    h.run_frames(30);
    assert_eq!(h.game.0.error(), None);
    // Dois besouros andando e três corações no HUD.
    assert_eq!(lua::<i64>(&rt, "return #cena:find_all('Besouro')"), 2);
    assert_eq!(lua::<i64>(&rt, "return nn.vars.vidas"), 3);
    // Os besouros andam pela ilha (pelos caminhos) sem cair no mar.
    let mut menor = f64::MAX;
    for _ in 0..20 {
        h.run_frames(15);
        let m: f64 = lua(
            &rt,
            "local m = 99 for _, b in ipairs(cena:find_all('Besouro')) do
               local _, y = b:position3d() m = math.min(m, y) end return m",
        );
        menor = menor.min(m);
    }
    assert!(menor > -0.5, "um besouro caiu: {menor}");
    assert!(
        !lua::<bool>(&rt, "return cena:nav():walkable(0, 40)"),
        "o mar"
    );
    // O herói está em pé na ilha e a câmera atrás dele: W anda para −Z.
    let y: f64 = lua(&rt, "return cena.objects['Herói'].y");
    assert!(y.abs() < 0.1, "em pé no chão: {y}");
    h.ctx.input.press_key(KeyCode::KeyW);
    h.run_frames(40);
    h.ctx.input.release_key(KeyCode::KeyW);
    let z: f64 = lua(&rt, "return cena.objects['Herói'].z");
    assert!(z < 6.5, "andou para a ilha: {z}");
    // Pula: poeira nos pés (um evento no Herói solta as partículas).
    h.ctx.input.press_key(KeyCode::Space);
    h.run_frames(2);
    h.ctx.input.release_key(KeyCode::Space);
    let poeira: i64 = lua(&rt, "return cena.objects.Poeira.particles.count");
    assert!(poeira > 0, "poeira ao pular");
    // A fogueira está acesa e o estalar é um som 3D.
    assert!(lua::<i64>(&rt, "return cena.objects.Fogo.particles.count") > 10);
    assert!(lua::<bool>(
        &rt,
        "return cena.objects.Fogueira:component('sound')._playing ~= nil"
    ));
    // O elevador sobe e a plataforma anda.
    h.run_frames(120);
    let ey: f64 = lua(&rt, "return cena.objects.Elevador.y");
    assert!(ey > 1.0, "elevador subiu: {ey}");
    // O cofre: a porta está trancada; a chave (numa plataforma) vai para o
    // inventário e, com ela, E abre a porta.
    rt.lua()
        .load(
            "local c = cena.objects.Chave
             local x, y, z = c:position3d()
             cena.objects['Herói']:move_to(x, y, z)",
        )
        .exec()
        .unwrap();
    h.run_frames(3);
    assert_eq!(lua::<i64>(&rt, "return nn.inventory.count('chave')"), 1);
    rt.lua()
        .load("cena.objects['Herói']:move_to(4.6, 0.1, 7.5)")
        .exec()
        .unwrap();
    h.run_frames(5);
    assert_eq!(lua::<String>(&rt, "return cena._ia.text"), "abrir");
    h.ctx.input.press_key(KeyCode::KeyE);
    h.run_frames(2);
    h.ctx.input.release_key(KeyCode::KeyE);
    h.run_frames(50);
    let ry: f64 = lua(&rt, "return cena.objects['Porta do cofre'].ry");
    assert!((ry + 90.0).abs() > 80.0, "a porta abriu: {ry}");
    assert_eq!(lua::<i64>(&rt, "return nn.inventory.count('chave')"), 0);
    // O pescador conversa: "Vim sim!" leva às dicas, e a última muda a
    // variável dica.
    rt.lua()
        .load("cena.objects['Herói']:move_to(1.7, 0.1, 6.2)")
        .exec()
        .unwrap();
    h.run_frames(5);
    assert_eq!(lua::<String>(&rt, "return cena._ia.text"), "falar");
    let tecla = |h: &mut Headless<Uma>, k: KeyCode| {
        h.ctx.input.press_key(k);
        h.run_frames(1);
        h.ctx.input.release_key(k);
        h.run_frames(2);
    };
    tecla(&mut h, KeyCode::KeyE);
    assert_eq!(
        lua::<String>(&rt, "return cena:dialogue()"),
        "dialogos/pescador.dialogo"
    );
    for k in [
        KeyCode::Enter,
        KeyCode::Digit1,
        KeyCode::Enter,
        KeyCode::Enter,
        KeyCode::Enter,
        KeyCode::Enter,
    ] {
        tecla(&mut h, k);
    }
    assert!(lua::<bool>(&rt, "return cena:dialogue() == nil"));
    assert_eq!(lua::<f64>(&rt, "return nn.vars.dica"), 1.0);
    // Leva o herói até cada estrela da ilha: pega as 5 (a sexta é do
    // besouro-rei).
    let estrelas: Vec<(f64, f64, f64)> = rt
        .lua()
        .load(
            "local out = {}
             for _, e in ipairs(cena:find_all('estrela')) do
               local x, y, z = e:position3d()
               table.insert(out, { x, y, z })
             end
             return out",
        )
        .eval::<Vec<Vec<f64>>>()
        .unwrap()
        .into_iter()
        .map(|p| (p[0], p[1], p[2]))
        .collect();
    assert_eq!(estrelas.len(), 5);
    for (x, y, z) in estrelas {
        rt.lua()
            .load(format!(
                "cena.objects['Herói']:move_to({x}, {}, {z})",
                y - 0.5
            ))
            .exec()
            .unwrap();
        h.run_frames(3);
    }
    assert_eq!(lua::<i64>(&rt, "return nn.vars.estrelas"), 5);
    // Cada estrela pega solta um brilho (partículas 3D).
    assert!(lua::<bool>(&rt, "return cena._fx['#FFD166'].is3d"));
    assert!(lua::<i64>(&rt, "return #cena:find_all('estrela')") == 0);
    // O placar mostra.
    let placar: String = lua(&rt, "return cena.objects.Placar.text.text");
    assert!(placar.contains("Estrelas 5 / 6"), "{placar}");
    assert_eq!(h.game.0.error(), None);
    // A arena: o portão começou aberto (desceu); entrando, ele fecha e as
    // ondas começam.
    let portao =
        |rt: &ScriptRuntime| -> f64 { lua(rt, "return cena.objects['Portão da arena'].y") };
    assert!(portao(&rt) < 0.0, "aberto: {}", portao(&rt));
    rt.lua()
        .load("cena.objects['Herói']:move_to(-22, 0.1, 0)")
        .exec()
        .unwrap();
    h.run_frames(120);
    assert!(portao(&rt) > 1.0, "fechou: {}", portao(&rt));
    assert_eq!(lua::<f64>(&rt, "return nn.vars.onda"), 1.0);
    assert!(lua::<bool>(
        &rt,
        "return cena.objects['Placar da arena'].visible"
    ));
    // Derrota cada onda (o herói fica longe, sem levar dano).
    let mut ondas = 0;
    for _ in 0..40 {
        rt.lua()
            .load(
                "cena.objects['Herói']:move_to(-19.5, 0.1, 0)
                 for _, b in ipairs(cena:find_all('besouro')) do cena:hit(b, 5, cena.objects['Herói']) end",
            )
            .exec()
            .unwrap();
        h.run_frames(30);
        ondas = lua::<i64>(&rt, "return nn.vars.onda");
        if lua::<String>(
            &rt,
            "return cena.objects['Besouro-rei']:behavior('chefe').estado",
        ) != "dormindo"
        {
            break;
        }
    }
    assert_eq!(ondas, 3);
    assert_eq!(h.game.0.error(), None);
    assert!(!lua::<bool>(
        &rt,
        "return cena.objects['Placar da arena'].visible"
    ));
    // O besouro-rei acordou; derrotado, deixa a sexta estrela e o portão abre.
    rt.lua()
        .load("cena:hit(cena.objects['Besouro-rei'], 99, cena.objects['Herói'])")
        .exec()
        .unwrap();
    h.run_frames(200);
    assert_eq!(h.game.0.error(), None);
    assert!(lua::<bool>(
        &rt,
        "return cena.objects['Besouro-rei'] == nil"
    ));
    assert!(portao(&rt) < 0.0, "abriu de novo: {}", portao(&rt));
    let (x, y, z): (f64, f64, f64) = {
        let v: Vec<f64> = lua(&rt, "return { cena:find_all('estrela')[1]:position3d() }");
        (v[0], v[1], v[2])
    };
    rt.lua()
        .load(format!(
            "cena.objects['Herói']:move_to({x}, {}, {z})",
            y - 0.5
        ))
        .exec()
        .unwrap();
    h.run_frames(5);
    assert_eq!(lua::<i64>(&rt, "return nn.vars.estrelas"), 6);
    assert_eq!(h.game.0.error(), None);
    drop(h);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn modelo_vale3d_terreno_pisa_lago_volta_e_estrelas() {
    let dir = temp("vale3d");
    create_project(&NewOptions {
        dir: dir.clone(),
        title: None,
        template: Template::Vale3D,
    })
    .unwrap();
    let Some((mut h, rt)) = cena_do_modelo(&dir, "cenas/vale.cena") else {
        return;
    };
    h.run_frames(30);
    assert_eq!(h.game.0.error(), None);
    // Em pé no terreno, na altura do chão.
    let (y, chao): (f64, f64) = (
        lua(&rt, "return cena.objects['Herói'].y"),
        lua(&rt, "return cena.objects.Terreno.terrain:height_at(0, 36)"),
    );
    assert!((y - chao).abs() < 0.2, "no chão: {y} ({chao})");
    // Anda pela trilha.
    h.ctx.input.press_key(KeyCode::KeyW);
    h.run_frames(40);
    h.ctx.input.release_key(KeyCode::KeyW);
    let z: f64 = lua(&rt, "return cena.objects['Herói'].z");
    assert!(z < 33.0, "andou: {z}");
    // No lago: volta para a margem.
    rt.lua()
        .load("cena.objects['Herói']:move_to(-14, -0.5, 12)")
        .exec()
        .unwrap();
    h.run_frames(40);
    let (x, z): (f64, f64) = (
        lua(&rt, "return cena.objects['Herói'].x"),
        lua(&rt, "return cena.objects['Herói'].z"),
    );
    assert!(
        ((x + 14.0).powi(2) + (z - 12.0).powi(2)).sqrt() > 5.0,
        "saiu do lago: {x} {z}"
    );
    // Os caminhos: o lago (com a água do terreno) não é andável, a trilha
    // é; e os besouros andam sem entrar no lago.
    rt.lua().load("cena._nav_dirty = true").exec().unwrap();
    let t = std::time::Instant::now();
    let andaveis: i64 = lua(&rt, "return cena:nav().walkable_count");
    eprintln!(
        "grade de caminhos do vale: {andaveis} células, {:?}",
        t.elapsed()
    );
    assert!(andaveis > 5000, "{andaveis}");
    assert!(
        !lua::<bool>(&rt, "return cena:nav():walkable(-14, 12)"),
        "o lago"
    );
    assert!(
        lua::<bool>(&rt, "return cena:nav():walkable(0, 30)"),
        "a trilha"
    );
    let mut fundo = f64::MAX;
    for _ in 0..30 {
        h.run_frames(10);
        let m: f64 = lua(
            &rt,
            "local m = 99 for _, b in ipairs(cena:find_all('Besouro')) do
               local x, y, z = b:position3d()
               local a = cena:water_level(x, z)
               if a then m = math.min(m, y - a) end
             end return m",
        );
        fundo = fundo.min(m);
    }
    assert!(fundo > -0.3, "um besouro entrou no lago: {fundo}");
    // As 5 estrelas ficam em cima dos morros, e dá para pegar.
    let n: i64 = lua(&rt, "return #cena:find_all('estrela')");
    assert_eq!(n, 5);
    for k in 1..=5 {
        rt.lua()
            .load(format!(
                "local e = cena:find_all('estrela')[1]
                 local x, y, z = e:position3d()
                 assert(y > cena.objects.Terreno.terrain:height_at(x, z), 'estrela {k} acima do chão')
                 cena.objects['Herói']:move_to(x, y - 0.5, z)"
            ))
            .exec()
            .unwrap();
        h.run_frames(3);
    }
    assert_eq!(lua::<i64>(&rt, "return nn.vars.estrelas"), 5);
    assert_eq!(h.game.0.error(), None);
    drop(h);
    std::fs::remove_dir_all(&dir).unwrap();
}

fn toque_uma(h: &mut Headless<Uma>, k: KeyCode) {
    h.ctx.input.press_key(k);
    h.run_frames(1);
    h.ctx.input.release_key(k);
    h.run_frames(1);
}

/// Ferramenta: grava as miniaturas dos modelos que o NN Studio mostra no
/// "Novo projeto" (`cargo test -p nn-run --test novo miniaturas -- --ignored`).
/// A do modelo Vazio (que não tem nada na cena) é montada à parte.
#[test]
#[ignore]
fn miniaturas_dos_modelos() {
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../nn-studio/assets/modelos");
    std::fs::create_dir_all(&out).unwrap();
    // NN_MODELO=3d: só a miniatura desse modelo.
    let so = std::env::var("NN_MODELO").ok();
    for template in Template::ALL {
        if template == Template::Vazio || so.as_deref().is_some_and(|m| m != template.name()) {
            continue;
        }
        let dir = temp(&format!("miniatura-{}", template.name()));
        create_project(&NewOptions {
            dir: dir.clone(),
            title: Some("Meu jogo".into()),
            template,
        })
        .unwrap();
        let project = Project::open(&dir).unwrap();
        let scene = if template == Template::TresD {
            "cenas/jogo.lua".to_string()
        } else {
            ["cenas/fase.cena", "cenas/mapa.cena", "cenas/jogo.cena"]
                .into_iter()
                .map(str::to_string)
                .find(|r| dir.join(r).is_file())
                .unwrap_or(project.start.clone())
        };
        let rt = ScriptRuntime::new(&dir).unwrap();
        // Todos do mesmo jeito: o jogo em 1280×720 reduzido para 320×180.
        let config = project
            .config
            .clone()
            .size(320, 180)
            .resolution(1280, 720)
            .save_dir(dir.join("dados"))
            .hot_reload(false);
        let Ok(mut h) = Headless::new(config, move |ctx| {
            let mut s = rt.scene(scene);
            s.enter(ctx);
            Uma(s)
        }) else {
            return;
        };
        h.run_frames(if template == Template::Nave { 150 } else { 40 });
        if template == Template::Visual3D || template == Template::Vale3D {
            // A dica do começo some depois de 4 segundos.
            h.run_frames(220);
        }
        if template == Template::TresD
            || template == Template::Visual3D
            || template == Template::Vale3D
        {
            // Anda um pouco para a frente, para a ilha aparecer melhor.
            h.ctx.input.press_key(KeyCode::KeyW);
            h.run_frames(25);
            h.ctx.input.release_key(KeyCode::KeyW);
            h.run_frames(40);
        }
        h.save_png(out.join(format!("{}.png", template.name())))
            .unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// Ferramenta: grava as imagens dos jogos 3D do README e do guia "Comece aqui
/// no 3D" (`cargo test -p nn-run --test novo capturas -- --ignored`). Com
/// `NN_CAMINHOS=1`, aparecem a grade dos caminhos e o caminho dos inimigos.
#[test]
#[ignore]
fn capturas_dos_jogos_3d() {
    // NN_SAIDA=<pasta>: grava lá (para conferir sem mexer nas da documentação).
    let out = std::env::var_os("NN_SAIDA")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/imagens"));
    let mut lista = vec![
        (Template::Visual3D, "jogo-ilha-3d.png", 25),
        (Template::Vale3D, "jogo-vale-3d.png", 45),
    ];
    if std::env::var_os("NN_SAIDA").is_some() {
        // Só para conferir (não vai para a documentação).
        lista.push((Template::Aventura, "jogo-aventura.png", 0));
    }
    for (template, file, frente) in lista {
        let dir = temp(&format!("captura-{}", template.name()));
        create_project(&NewOptions {
            dir: dir.clone(),
            title: Some("Meu jogo".into()),
            template,
        })
        .unwrap();
        let project = Project::open(&dir).unwrap();
        let scene = if template == Template::Aventura {
            "cenas/mapa.cena".to_string()
        } else {
            project.start.clone()
        };
        let rt = ScriptRuntime::new(&dir).unwrap();
        let config = project
            .config
            .clone()
            .size(1280, 720)
            .save_dir(dir.join("dados"))
            .hot_reload(false);
        let Ok(mut h) = Headless::new(config, move |ctx| {
            let mut s = rt.scene(scene);
            s.enter(ctx);
            Uma(s)
        }) else {
            return;
        };
        // A dica do começo some depois de 4 segundos; depois, anda um pouco.
        h.run_frames(260);
        h.ctx.input.press_key(KeyCode::KeyW);
        h.run_frames(frente);
        h.ctx.input.release_key(KeyCode::KeyW);
        h.run_frames(40);
        h.save_png(out.join(file)).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// Ferramenta: imagens do combate no Vale 3D (o herói golpeando um besouro,
/// o número do dano e a barra de vida): `NN_SAIDA=<pasta> cargo test -p
/// nn-run --test novo capturas_do_combate -- --ignored`.
#[test]
#[ignore]
fn capturas_do_combate() {
    let Some(out) = std::env::var_os("NN_SAIDA").map(PathBuf::from) else {
        return;
    };
    let dir = temp("captura-combate");
    create_project(&NewOptions {
        dir: dir.clone(),
        title: Some("Meu jogo".into()),
        template: Template::Vale3D,
    })
    .unwrap();
    let project = Project::open(&dir).unwrap();
    let rt = ScriptRuntime::new(&dir).unwrap();
    let rt2 = rt.clone();
    let config = project
        .config
        .clone()
        .size(1280, 720)
        .save_dir(dir.join("dados"))
        .hot_reload(false);
    let start = project.start.clone();
    let Ok(mut h) = Headless::new(config, move |ctx| {
        let mut s = rt.scene(start);
        s.enter(ctx);
        Uma(s)
    }) else {
        return;
    };
    h.run_frames(30);
    let lua = rt2.lua();
    lua.globals().set("C", h.game.0.table().unwrap()).unwrap();
    // O herói de frente para o primeiro besouro, a câmera mais perto e de lado.
    lua.load(
        r#"
        local b = C:find_all("Besouro")[1]
        local bx, by, bz = b:position3d()
        b:behavior("inimigo3d").enabled = false
        b:behavior("golpe").enabled = false
        local h = C.objects["Herói"]
        h:move_to(bx, by + 0.1, bz + 1.6)
        h.ry = 180
        local cam = C._player_cam.cam
        cam.distance = 3.2
        cam.yaw = 120
        cam.pitch = 12
        "#,
    )
    .exec()
    .unwrap();
    h.run_frames(60);
    h.save_png(out.join("combate-parado.png")).unwrap();
    h.ctx.input.press_key(KeyCode::KeyJ);
    h.run_frames(1);
    h.ctx.input.release_key(KeyCode::KeyJ);
    h.run_frames(6);
    h.save_png(out.join("combate-golpe-alto.png")).unwrap();
    h.run_frames(8);
    h.save_png(out.join("combate-golpe-acerto.png")).unwrap();
    h.run_frames(12);
    h.save_png(out.join("combate-numero.png")).unwrap();
    assert_eq!(h.game.0.error(), None);
    drop(h);
    std::fs::remove_dir_all(&dir).unwrap();

    // O herói com um cajado e um Atirar com a magia (mira no meio da tela).
    create_project(&NewOptions {
        dir: dir.clone(),
        title: Some("Meu jogo".into()),
        template: Template::Vale3D,
    })
    .unwrap();
    let cena = dir.join("cenas/vale.cena");
    let texto = std::fs::read_to_string(&cena)
        .unwrap()
        .replace("modelos/espada.glb", "modelos/cajado.glb")
        .replacen(
            r#"{ type = "behavior", script = "nn:golpe","#,
            r#"{ type = "behavior", script = "nn:atirar", prefab = "prefabs/magia.prefab", osso = "mao_d", som = "assets/sons/tiro.wav" },
        { type = "behavior", script = "nn:golpe","#,
            1,
        );
    std::fs::write(&cena, texto).unwrap();
    let project = Project::open(&dir).unwrap();
    let rt = ScriptRuntime::new(&dir).unwrap();
    let rt2 = rt.clone();
    let config = project
        .config
        .clone()
        .size(1280, 720)
        .save_dir(dir.join("dados"))
        .hot_reload(false);
    let start = project.start.clone();
    let Ok(mut h) = Headless::new(config, move |ctx| {
        let mut s = rt.scene(start);
        s.enter(ctx);
        Uma(s)
    }) else {
        return;
    };
    h.run_frames(30);
    let lua = rt2.lua();
    lua.globals().set("C", h.game.0.table().unwrap()).unwrap();
    lua.load("local cam = C._player_cam.cam; cam.pitch = 6")
        .exec()
        .unwrap();
    h.run_frames(30);
    h.ctx.input.press_mouse(nn_input::MouseButton::Right);
    h.run_frames(1);
    h.ctx.input.release_mouse(nn_input::MouseButton::Right);
    h.run_frames(14);
    h.save_png(out.join("combate-magia.png")).unwrap();
    assert_eq!(h.game.0.error(), None);
    drop(h);
    std::fs::remove_dir_all(&dir).unwrap();
}

/// Ferramenta: imagens da arena da ilha (o letreiro da onda, os besouros e
/// o besouro-rei com a barra no alto): `NN_SAIDA=<pasta> cargo test -p
/// nn-run --test novo capturas_da_arena -- --ignored`.
#[test]
#[ignore]
fn capturas_da_arena() {
    let Some(out) = std::env::var_os("NN_SAIDA").map(PathBuf::from) else {
        return;
    };
    let dir = temp("captura-arena");
    create_project(&NewOptions {
        dir: dir.clone(),
        title: Some("Meu jogo".into()),
        template: Template::Visual3D,
    })
    .unwrap();
    let project = Project::open(&dir).unwrap();
    let rt = ScriptRuntime::new(&dir).unwrap();
    let rt2 = rt.clone();
    let config = project
        .config
        .clone()
        .size(1280, 720)
        .save_dir(dir.join("dados"))
        .hot_reload(false);
    let start = project.start.clone();
    let Ok(mut h) = Headless::new(config, move |ctx| {
        let mut s = rt.scene(start);
        s.enter(ctx);
        Uma(s)
    }) else {
        return;
    };
    h.run_frames(30);
    let lua = rt2.lua();
    lua.globals().set("C", h.game.0.table().unwrap()).unwrap();
    // Entra na arena, olhando para o meio dela.
    lua.load(
        r#"
        local h = C.objects["Herói"]
        h:move_to(-22, 0.1, 0)
        h.ry = -90
        local cam = C._player_cam.cam
        cam.yaw = 90
        cam.pitch = 20
        cam.distance = 3
        "#,
    )
    .exec()
    .unwrap();
    h.run_frames(100);
    h.save_png(out.join("arena-onda.png")).unwrap();
    // Pula as ondas: o besouro-rei acorda.
    for _ in 0..40 {
        lua.load("C.objects['Herói']:behavior('vida').inv = 1e9; for _, b in ipairs(C:find_all('besouro')) do b:destroy() end")
            .exec()
            .unwrap();
        h.run_frames(30);
    }
    lua.load(
        r#"
        local h = C.objects["Herói"]
        h:move_to(-27, 0.1, 3)
        h.ry = 180
        local cam = C._player_cam.cam
        cam.yaw = 20
        cam.pitch = 22
        cam.distance = 6
        "#,
    )
    .exec()
    .unwrap();
    h.run_frames(30);
    h.save_png(out.join("arena-rei.png")).unwrap();
    for i in 0..6 {
        h.run_frames(25);
        let info: String = lua
            .load(
                "local r = C.objects['Besouro-rei']; local h = C.objects['Herói']; local c = C._player_cam.cam
                 local rx, ry, rz = r:position3d(); local hx, hy, hz = h:position3d()
                 return string.format('rei %.1f %.1f %.1f %s | heroi %.1f %.1f %.1f vida %s | cam %.1f %.1f %.1f',
                   rx, ry, rz, r:behavior('chefe').estado, hx, hy, hz, tostring(h:behavior('vida').atual), c.x, c.y, c.z)",
            )
            .eval()
            .unwrap();
        eprintln!("{info}");
        h.save_png(out.join(format!("arena-luta-{i}.png"))).unwrap();
    }
    assert_eq!(h.game.0.error(), None);
    drop(h);
    std::fs::remove_dir_all(&dir).unwrap();
}
