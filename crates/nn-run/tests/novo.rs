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
        assert!(
            ScriptRuntime::new(&dir).unwrap().check_syntax().is_empty(),
            "{}",
            template.name()
        );
        // A cena inicial e todas as cenas visuais (as cenas em código podem
        // depender do que a anterior preparou).
        let mut scenes = vec![Project::open(&dir).unwrap().start];
        if let Ok(list) = std::fs::read_dir(dir.join("cenas")) {
            for e in list.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                let rel = format!("cenas/{name}");
                if name.ends_with(".cena") && !scenes.contains(&rel) {
                    scenes.push(rel);
                }
            }
        }
        assert!(!scenes.is_empty(), "{}", template.name());
        for s in scenes {
            let Some((mut h, _rt)) = cena_do_modelo(&dir, &s) else {
                return;
            };
            h.run_frames(90);
            assert_eq!(h.game.0.error(), None, "{} {s}", template.name());
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
    // Leva o herói até cada estrela: pega as 5 e o evento de vitória
    // dispara (o som e a espera vêm antes de ir para o fim).
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
    assert!(placar.contains("Estrelas 5 / 5"), "{placar}");
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
/// no 3D" (`cargo test -p nn-run --test novo capturas -- --ignored`).
#[test]
#[ignore]
fn capturas_dos_jogos_3d() {
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/imagens");
    for (template, file, frente) in [
        (Template::Visual3D, "jogo-ilha-3d.png", 25),
        (Template::Vale3D, "jogo-vale-3d.png", 45),
    ] {
        let dir = temp(&format!("captura-{}", template.name()));
        create_project(&NewOptions {
            dir: dir.clone(),
            title: Some("Meu jogo".into()),
            template,
        })
        .unwrap();
        let project = Project::open(&dir).unwrap();
        let scene = project.start.clone();
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
