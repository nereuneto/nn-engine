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
    assert_eq!(project.start, "cenas/fase.cena");
    assert!(ScriptRuntime::new(&dir).unwrap().check_syntax().is_empty());

    let rt = ScriptRuntime::new(&dir).unwrap();
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
    assert_eq!(h.game.0.field::<i64>("total"), Some(3));
    // Anda para a direita até a moeda que está no chão.
    h.ctx.input.press_key(KeyCode::KeyD);
    h.run_frames(150);
    h.ctx.input.release_key(KeyCode::KeyD);
    h.run_frames(5);
    assert_eq!(h.game.0.error(), None);
    assert_eq!(
        h.game.0.field::<i64>("pontos"),
        Some(1),
        "pegou a moeda do chão"
    );
    drop(h);
    std::fs::remove_dir_all(&dir).unwrap();
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
