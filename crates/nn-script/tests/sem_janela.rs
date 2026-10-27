//! O jogo sem janela (`GameRun`) e os testes de jogo (`testes/*.lua`).

use std::path::PathBuf;
use std::time::Duration;

use nn_script::Project;
use nn_script::capture::LineKind;
use nn_script::game_run::{Action, ActionKind, GameRun, Outcome, Plan, RunOptions};
use nn_script::testing::{TestOptions, run_tests};

fn projeto(name: &str, extra: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-sem-janela-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let files: &[(&str, &str)] = &[
        (
            "config.lua",
            r#"return { title = "Sem janela", width = 160, height = 90,
  start = "cenas/menu.cena", vars = { pontos = 0 } }"#,
        ),
        (
            "cenas/menu.cena",
            r#"return {
  events = {
    { when = { { cond = "tecla", modo = "apertar", tecla = "enter" } },
      actions = { { act = "ir_para_cena", cena = "cenas/fase.cena", escurecer = 0 } } },
  },
  objects = {
    { id = 1, name = "titulo", components = { { type = "text", text = "Menu" } } },
  },
}"#,
        ),
        (
            "cenas/fase.cena",
            r##"return {
  script = "cenas/fase.lua",
  events = {
    { when = { { cond = "tecla", modo = "apertar", tecla = "space" } },
      actions = { { act = "variavel", var = "pontos", op = "somar", valor = 1 },
        { act = "console", texto = "pontos = {pontos}" } } },
  },
  objects = {
    { id = 1, name = "heroi", components = { { type = "shape", w = 10, h = 10, color = "#FF0000" } } },
    { id = 2, name = "moeda", x = 50, components = { { type = "shape", w = 4, h = 4 } } },
  },
}"##,
        ),
        (
            "cenas/fase.lua",
            r#"local cena = {}
function cena:update(dt)
  local h = self.objects.heroi
  if nn.input.down("right") then h.x = h.x + 100 * dt end
  if nn.input.pressed("x") then error("bum") end
  if nn.input.pressed("z") then while true do end end
  if nn.input.pressed("q") then nn.quit() end
end
return cena
"#,
        ),
    ];
    for (path, content) in files.iter().chain(extra) {
        let full = dir.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }
    dir
}

fn abrir(dir: &PathBuf, opts: RunOptions) -> Option<GameRun> {
    match GameRun::open(dir, opts) {
        Ok(r) => Some(r),
        Err(e) => {
            if std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1") {
                panic!("GPU obrigatória mas indisponível: {e}");
            }
            eprintln!("pulando teste: {e}");
            None
        }
    }
}

fn at(at: f64, kind: ActionKind) -> Action {
    Action { at, kind }
}

#[test]
fn roteiro_mostra_o_que_aconteceu() {
    let dir = projeto("roteiro", &[]);
    let Some(mut run) = abrir(&dir, RunOptions::default()) else {
        return;
    };
    assert_eq!(run.front(), "cenas/menu.cena");
    let plan = Plan {
        duration: 2.0,
        actions: vec![
            at(1.5, ActionKind::Press("space".into())),
            at(0.1, ActionKind::Press("enter".into())),
            Action::hold(0.2, "right", 1.0),
            at(1.0, ActionKind::Capture("andando".into())),
            at(1.7, ActionKind::Press("space".into())),
            at(1.8, ActionKind::Lua("self.objects.moeda.y = 7".into())),
        ],
    };
    assert_eq!(run.run_plan(&plan).unwrap(), Outcome::Done);
    assert_eq!(run.frames(), 120);
    assert_eq!(run.front(), "cenas/fase.cena");

    let heroi: f64 = run
        .inspect("self.objects.heroi.x")
        .unwrap()
        .parse()
        .unwrap();
    assert!((heroi - 100.0).abs() < 4.0, "{heroi}");
    assert_eq!(run.inspect("self.objects.moeda.y").unwrap(), "7");
    assert_eq!(run.inspect("nn.vars.pontos, 'a'").unwrap(), "2, \"a\"");
    assert!(run.inspect("nada.aqui").unwrap_err().contains("nada"));

    let state = run.snapshot().unwrap();
    assert_eq!(state["vars"]["pontos"], 2);
    let objects = state["objects"].as_array().unwrap();
    assert_eq!(objects.len(), 2);
    assert_eq!(objects[0]["name"], "heroi");

    let report = run.take_report();
    let scenes: Vec<_> = report.scenes.iter().map(|s| s.scene.as_str()).collect();
    assert_eq!(scenes, ["cenas/menu.cena", "cenas/fase.cena"]);
    assert!((report.scenes[1].time - 0.1).abs() < 0.05);
    let prints: Vec<_> = report
        .lines
        .iter()
        .filter(|l| l.kind == LineKind::Print)
        .map(|l| (l.text.as_str(), (l.time * 10.0).round() / 10.0))
        .collect();
    assert_eq!(prints, [("pontos = 1", 1.5), ("pontos = 2", 1.7)]);
    let fired: Vec<_> = report.fired.iter().map(|f| f.text.as_str()).collect();
    assert!(
        fired.contains(&"regra 1 da cena: quando tecla → ir_para_cena"),
        "{fired:?}"
    );
    let espaco = report
        .fired
        .iter()
        .find(|f| f.scene == "cenas/fase.cena")
        .unwrap();
    assert_eq!(
        espaco.text,
        "regra 1 da cena: quando tecla → variavel, console"
    );
    assert_eq!(espaco.count, 2);
    assert_eq!(report.captures.len(), 1);
    let cap = &report.captures[0];
    assert_eq!(
        (cap.label.as_str(), cap.width, cap.height),
        ("andando", 160, 90)
    );
    assert!(cap.png.starts_with(b"\x89PNG"));
    assert!(report.errors.is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn erro_e_laco_sem_fim_param_o_roteiro() {
    let dir = projeto("erros", &[]);
    let opts = RunOptions {
        scene: Some("cenas/fase.cena".into()),
        frame_limit: Duration::from_millis(300),
        ..RunOptions::default()
    };
    let Some(mut run) = abrir(&dir, opts.clone()) else {
        return;
    };
    let plan = Plan {
        duration: 3.0,
        actions: vec![at(0.5, ActionKind::Press("x".into()))],
    };
    let Outcome::GameError(e) = run.run_plan(&plan).unwrap() else {
        panic!("devia dar erro");
    };
    assert!((e.time - 0.5).abs() < 0.05, "{}", e.time);
    assert_eq!(e.error.scene, "cenas/fase.cena");
    assert!(
        e.error.message.contains("cenas/fase.lua:5"),
        "{}",
        e.error.message
    );
    assert!(e.error.message.contains("bum"));
    assert!(run.time() < 1.0);

    // Um laço sem fim vira erro em vez de travar.
    let mut run = abrir(&dir, opts.clone()).unwrap();
    let plan = Plan {
        duration: 2.0,
        actions: vec![at(0.2, ActionKind::Press("z".into()))],
    };
    let Outcome::GameError(e) = run.run_plan(&plan).unwrap() else {
        panic!("devia travar");
    };
    assert!(e.error.message.contains("travou"), "{}", e.error.message);

    // Uma ação lua com erro para o roteiro com a mensagem.
    let mut run = abrir(&dir, opts.clone()).unwrap();
    let plan = Plan {
        duration: 1.0,
        actions: vec![at(0.2, ActionKind::Lua("self.objects.nada.x = 1".into()))],
    };
    let Outcome::Failed(msg) = run.run_plan(&plan).unwrap() else {
        panic!("devia falhar");
    };
    assert!(msg.contains("ação lua em 0.2 s"), "{msg}");

    // A cena já começa com erro: o roteiro nem roda.
    std::fs::write(
        dir.join("cenas/ruim.lua"),
        "local c = {}\nfunction c:enter() error('no começo') end\nreturn c\n",
    )
    .unwrap();
    let mut run = abrir(
        &dir,
        RunOptions {
            scene: Some("cenas/ruim.lua".into()),
            ..opts.clone()
        },
    )
    .unwrap();
    let Outcome::GameError(e) = run
        .run_plan(&Plan {
            duration: 1.0,
            actions: vec![],
        })
        .unwrap()
    else {
        panic!("devia dar erro");
    };
    assert_eq!((e.time, e.error.scene.as_str()), (0.0, "cenas/ruim.lua"));
    assert!(
        e.error.message.contains("cenas/ruim.lua:2: no começo"),
        "{}",
        e.error.message
    );
    assert_eq!(run.frames(), 0);

    // nn.quit fecha o jogo e o roteiro termina.
    let mut run = abrir(&dir, opts).unwrap();
    let plan = Plan {
        duration: 2.0,
        actions: vec![at(0.3, ActionKind::Press("q".into()))],
    };
    assert_eq!(run.run_plan(&plan).unwrap(), Outcome::Done);
    assert!(run.ended());
    let report = run.take_report();
    assert!(report.quit.is_some_and(|t| (t - 0.3).abs() < 0.05));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn testes_de_jogo_passam_e_falham_com_a_linha() {
    let dir = projeto(
        "testes",
        &[
            (
                "testes/menu.lua",
                r#"test("enter abre a fase", function(t)
  t:check_eq(t:scene_path(), "cenas/menu.cena")
  t:press("enter")
  t:check_eq(t:scene_path(), "cenas/fase.cena")
  t:check_eq(nn.vars.pontos, 0)
  nn.vars.pontos = 5
end)

test("o herói anda", function(t)
  t:check_eq(nn.vars.pontos, 0, "cada teste começa do zero")
  t:open("cenas/fase.cena")
  local h = t:find("heroi")
  t:hold("right", 0.5)
  t:check_near(h.x, 50, 2)
end)

test("falha com a linha", function(t)
  t:open("cenas/fase.cena")
  t:check(t:find("heroi").x > 1000, "longe demais")
end)

test("acha o que não existe", function(t)
  t:open("cenas/fase.cena")
  t:find("dragao")
end)

test("erro do jogo", function(t)
  t:open("cenas/fase.cena")
  t:press("x")
end)

test("espera até", function(t)
  t:open("cenas/fase.cena")
  t:down("right")
  t:wait_until(function() return t:find("heroi").x >= 50 end, 2)
  t:up("right")
  t:screenshot("chegou")
  print("chegou em", t:time())
end)

test("demora demais", function(t)
  t:open("cenas/fase.cena")
  t:wait_until(function() return false end, 0.5, "a porta nunca abriu")
end)
"#,
            ),
            ("testes/quebrado.lua", "test(\"x\", function(t)\n"),
            ("testes/vazio.lua", "-- só ajuda, sem testes\nreturn {}\n"),
        ],
    );
    let project = Project::open(&dir).unwrap();
    let opts = TestOptions {
        capture_failures: true,
        ..TestOptions::default()
    };
    let summary = match run_tests(&project, &opts, |_| {}) {
        Ok(s) => s,
        Err(e) => {
            if std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1") {
                panic!("GPU obrigatória mas indisponível: {e}");
            }
            return;
        }
    };
    assert_eq!(summary.file_errors.len(), 1);
    assert_eq!(summary.file_errors[0].file, "testes/quebrado.lua");
    let by_name = |n: &str| summary.results.iter().find(|r| r.case.name == n).unwrap();
    for ok in ["enter abre a fase", "o herói anda", "espera até"] {
        assert!(by_name(ok).passed(), "{ok}: {:?}", by_name(ok).failure);
    }
    let f = by_name("falha com a linha").failure.clone().unwrap();
    assert!(f.starts_with("testes/menu.lua:19: longe demais"), "{f}");
    assert!(by_name("falha com a linha").last_frame.is_some());
    let f = by_name("acha o que não existe").failure.clone().unwrap();
    assert!(f.contains("testes/menu.lua:24:"), "{f}");
    assert!(
        f.contains("\"dragao\"") && f.contains("tem: heroi, moeda"),
        "{f}"
    );
    let f = by_name("erro do jogo").failure.clone().unwrap();
    assert!(f.contains("o jogo deu erro") && f.contains("bum"), "{f}");
    let f = by_name("demora demais").failure.clone().unwrap();
    assert!(f.contains("testes/menu.lua:43: a porta nunca abriu"), "{f}");

    let r = by_name("espera até");
    assert_eq!(r.report.captures.len(), 1);
    assert_eq!(r.report.captures[0].label, "chegou");
    assert!(r.last_frame.is_none());
    let print = r
        .report
        .lines
        .iter()
        .find(|l| l.kind == LineKind::Print)
        .unwrap();
    assert!(print.text.starts_with("chegou em\t0.5"), "{}", print.text);
    assert_eq!(summary.passed(), 3);
    assert_eq!(summary.failed(), 4);
    assert!(!summary.ok());

    // Filtro pelo nome.
    let opts = TestOptions {
        filter: Some("HERÓI".into()),
        ..TestOptions::default()
    };
    let summary = run_tests(&project, &opts, |_| {}).unwrap();
    assert_eq!(summary.results.len(), 1);
    assert!(summary.results[0].passed());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mouse_preso_gira_a_camera_do_jogador_3d() {
    let dir = projeto(
        "mouse3d",
        &[
            (
                "cenas/mundo.cena",
                r##"return { format = 1, mode = "3d", gravity = -20,
  environment = { fog = false, shadows = false },
  objects = {
    { id = 1, name = "Chão", y = -0.5, scale_x = 40, scale_z = 40,
      components = { { type = "shape3d", shape = "cube" }, { type = "body3d", body = "static" } } },
    { id = 10, name = "Herói", y = 1,
      components = { { type = "shape3d", shape = "capsule", color = "#FF8C42" },
        { type = "behavior", script = "nn:jogador3d" } } },
  } }"##,
            ),
            (
                "testes/camera.lua",
                r#"
local function lado()
  local c = nn.d3.camera()
  return math.atan(c.target.x - c.x, c.target.z - c.z)
end
test("o mouse preso gira a câmera", function(t)
  t:open("cenas/mundo.cena"); t:frames(10)
  t:check(nn.window.mouse_captured(), "o Jogador 3D prende o mouse")
  local antes = lado()
  t:mouse_motion(300, 0, 0.5)
  local giro = math.abs(math.deg(lado() - antes)) % 360
  t:check(giro > 20 and giro < 340, "a câmera devia ter girado, girou " .. giro)
  t:check(t:time() > 0.4, "aos poucos, por meio segundo")
end)
"#,
            ),
        ],
    );
    let project = Project::open(&dir).unwrap();
    let summary = match run_tests(&project, &TestOptions::default(), |_| {}) {
        Ok(s) => s,
        Err(e) => {
            if std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1") {
                panic!("GPU obrigatória mas indisponível: {e}");
            }
            return;
        }
    };
    assert!(summary.file_errors.is_empty(), "{:?}", summary.file_errors);
    assert_eq!(summary.results.len(), 1);
    assert!(
        summary.results[0].passed(),
        "{:?}",
        summary.results[0].failure
    );
}
