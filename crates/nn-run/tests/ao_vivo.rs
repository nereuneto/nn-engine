//! O agente (servidor MCP) liga no jogo aberto: vê, muda na hora, pausa,
//! avança e fecha. O jogo roda sem janela numa thread e o agente fala com
//! ele pela porta local, como com um jogo de verdade.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use nn_app::{Headless, SceneStack};
use nn_mcp::Server;
use nn_script::{Project, ScriptRuntime};
use serde_json::{Value, json};

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-ao-vivo-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn gpu_required() -> bool {
    std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1")
}

/// Chama uma ferramenta: (deu erro?, texto, quantas imagens).
fn call(s: &mut Server, tool: &str, args: Value) -> (bool, String, usize) {
    let r = s
        .handle(json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": tool, "arguments": args },
        }))
        .unwrap();
    let content = r["result"]["content"].as_array().unwrap().clone();
    let text = content
        .iter()
        .filter_map(|c| c["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let images = content.iter().filter(|c| c["type"] == "image").count();
    (r["result"]["isError"] == true, text, images)
}

fn project(dir: &Path) {
    nn_run::create_project(&nn_run::NewOptions {
        dir: dir.to_path_buf(),
        title: None,
        template: nn_run::Template::Visual,
    })
    .unwrap();
}

#[test]
fn agente_ve_e_mexe_no_jogo_aberto() {
    let dir = temp("jogo");
    project(&dir);
    let listener = nn_script::remote::listen(&dir).unwrap();
    assert!(nn_script::remote::connection_file(&dir).is_file());

    // O jogo, nesta thread.
    let p = Project::open(&dir).unwrap();
    let rt = ScriptRuntime::with_source(p.source()).unwrap();
    let config = p.config.clone().size(320, 180).hot_reload(false);
    let mut h = match Headless::new(config, move |ctx| {
        SceneStack::new(ctx, rt.scene("cenas/fase.cena"))
    }) {
        Ok(h) => h,
        Err(e) => {
            assert!(!gpu_required(), "{e}");
            return;
        }
    };
    h.run_frames(2);

    // O agente, noutra.
    let d = dir.clone();
    let agent = std::thread::spawn(move || {
        let mut s = Server::new(d, nn_run::mcp_hooks());
        let mut out = Vec::new();
        out.push(call(&mut s, "ver_jogo_aberto", json!({})));
        out.push(call(
            &mut s,
            "mexer_no_jogo",
            json!({
                "mudar": [ { "objeto": "heroi", "componente": "plataforma", "campo": "velocidade", "valor": 900 },
                           { "objeto": "saida", "campo": "visible", "valor": false } ],
                "variaveis": { "vidas": 7 },
                "lua": "print('oi do agente'); return self.objects.heroi.name",
            }),
        ));
        out.push(call(
            &mut s,
            "ver_jogo_aberto",
            json!({ "objetos": ["heroi"], "olhar": ["nn.vars.vidas", "nada.x"], "captura": false }),
        ));
        out.push(call(&mut s, "mexer_no_jogo", json!({ "pausar": true })));
        // O inimigo patrulha a 60 px/s: pausado, não sai do lugar.
        let x = "return self.objects.inimigo.x";
        out.push(call(&mut s, "mexer_no_jogo", json!({ "lua": x })));
        std::thread::sleep(Duration::from_millis(300));
        out.push(call(&mut s, "mexer_no_jogo", json!({ "lua": x })));
        out.push(call(&mut s, "mexer_no_jogo", json!({ "avancar": 30 })));
        out.push(call(&mut s, "mexer_no_jogo", json!({ "lua": x })));
        out.push(call(&mut s, "ver_jogo_aberto", json!({ "captura": false })));
        // Teclas: pausado, o herói anda para a direita só nos quadros avançados.
        let hx = "return self.objects.heroi.x";
        out.push(call(&mut s, "mexer_no_jogo", json!({ "lua": hx })));
        out.push(call(
            &mut s,
            "mexer_no_jogo",
            json!({ "teclas": [ { "segurar": "right", "por": 0.5 } ], "avancar": 40 }),
        ));
        out.push(call(&mut s, "mexer_no_jogo", json!({ "lua": hx })));
        out.push(call(
            &mut s,
            "mexer_no_jogo",
            json!({ "mudar": [ { "objeto": "dragao", "campo": "x", "valor": 1 },
                               { "objeto": "heroi", "componente": "nn:voar", "campo": "x", "valor": 1 } ] }),
        ));
        out.push(call(
            &mut s,
            "mexer_no_jogo",
            json!({ "lua": "error('ops')" }),
        ));
        out.push(call(
            &mut s,
            "mexer_no_jogo",
            json!({ "pausar": false, "fechar": true }),
        ));
        out
    });
    let started = Instant::now();
    while !agent.is_finished() {
        h.frame(Headless::<SceneStack>::FRAME);
        std::thread::sleep(Duration::from_millis(2));
        assert!(
            started.elapsed() < Duration::from_secs(120),
            "o agente não terminou"
        );
    }
    let out = agent.join().unwrap();
    assert!(h.ctx.is_exiting(), "fechar devia fechar o jogo");

    let (err, text, images) = &out[0];
    assert!(!err, "{text}");
    assert!(
        text.starts_with("Jogo aberto em cenas/fase.cena:"),
        "{text}"
    );
    assert!(text.contains("rodando."), "{text}");
    assert!(text.contains("Os objetos de cenas/fase.cena:"), "{text}");
    assert!(text.contains("heroi (5): x 0"), "{text}");
    assert!(text.contains("A tela agora (320x180)"), "{text}");
    assert_eq!(*images, 1);

    let (err, text, _) = &out[1];
    assert!(!err, "{text}");
    assert!(
        text.contains("✓ heroi.plataforma.velocidade = 900"),
        "{text}"
    );
    assert!(text.contains("✓ saida.visible = false"), "{text}");
    assert!(text.contains("✓ nn.vars.vidas = 7"), "{text}");
    assert!(text.contains("✓ lua: \"heroi\""), "{text}");
    assert!(text.contains("Nada disso foi gravado"), "{text}");

    let (err, text, images) = &out[2];
    assert!(!err, "{text}");
    assert!(
        text.contains("O que mudou em cenas/fase.cena desde a última olhada:"),
        "{text}"
    );
    assert!(
        text.contains("nn:plataforma.velocidade 320 → 900"),
        "{text}"
    );
    assert!(text.contains("saida (12): visible true → false"), "{text}");
    assert!(
        text.contains("Variáveis que mudaram: vidas 3 → 7"),
        "{text}"
    );
    assert!(text.contains("oi do agente"), "{text}");
    assert!(text.contains("\"velocidade\":900"), "{text}");
    assert!(text.contains("nn.vars.vidas = 7"), "{text}");
    assert!(text.contains("nada.x: erro:"), "{text}");
    assert_eq!(*images, 0);

    assert!(out[3].1.contains("✓ jogo pausado"), "{}", out[3].1);
    let x = |t: &str| -> f64 {
        t.lines()
            .find_map(|l| l.strip_prefix("✓ lua: "))
            .unwrap()
            .parse()
            .unwrap()
    };
    let (x1, x2, x3) = (x(&out[4].1), x(&out[5].1), x(&out[7].1));
    assert_eq!(x1, x2, "pausado, o inimigo não devia andar");
    assert!(
        out[6].1.contains("✓ avançou 30 quadro(s) (0.50 s)"),
        "{}",
        out[6].1
    );
    // Meio segundo a 60 px/s: até 30 px (menos se ele virou na beirada no
    // meio do caminho; onde ele está quando pausa depende da velocidade da
    // máquina).
    let andou = (x3 - x2).abs();
    assert!(andou > 0.5 && andou <= 35.0, "{x2} → {x3}");
    assert!(out[8].1.contains("PAUSADO"), "{}", out[8].1);

    let (err, text, _) = &out[10];
    assert!(!err, "{text}");
    assert!(
        text.contains("✓ teclas: segurar right de 0.00 s por 0.50 s"),
        "{text}"
    );
    let (hx1, hx2) = (x(&out[9].1), x(&out[11].1));
    assert!(hx2 - hx1 > 20.0, "segurar right devia andar: {hx1} → {hx2}");
    let (err, text, _) = &out[12];
    assert!(err);
    assert!(
        text.contains("não achei o objeto \"dragao\" (tem: "),
        "{text}"
    );
    assert!(
        text.contains("não tem o componente \"nn:voar\" (tem: "),
        "{text}"
    );
    let (err, text, _) = &out[13];
    assert!(err);
    assert!(text.contains("✗ lua: lua:1: ops"), "{text}");
    assert!(
        out[14].1.contains("✓ jogo continuando\n✓ jogo fechado"),
        "{}",
        out[14].1
    );

    // Ao sair, o arquivo de conexão vai embora.
    drop(h);
    drop(listener);
    assert!(!nn_script::remote::connection_file(&dir).exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sem_jogo_aberto_o_agente_sabe_o_que_fazer() {
    let dir = temp("fechado");
    project(&dir);
    let mut s = Server::new(dir.clone(), nn_run::mcp_hooks());
    let (err, text, _) = call(&mut s, "ver_jogo_aberto", json!({}));
    assert!(err);
    assert!(
        text.contains("não está aberto") && text.contains("abrir_jogo"),
        "{text}"
    );
    // Um arquivo de conexão que sobrou de um jogo que não fechou direito.
    let file = nn_script::remote::connection_file(&dir);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, r#"{"porta": 1, "token": "x"}"#).unwrap();
    let (err, text, _) = call(&mut s, "mexer_no_jogo", json!({ "pausar": true }));
    assert!(err && text.contains("não está aberto"), "{text}");
    assert!(!file.exists(), "o arquivo velho devia sumir");

    // Um jogo que fecha ao abrir: o erro mostra o fim do log.
    let mut hooks = nn_run::mcp_hooks();
    hooks.play = Some(|dir, _, log| {
        std::process::Command::new(env!("CARGO_BIN_EXE_nn"))
            .arg(dir)
            .arg("--opcao-que-nao-existe")
            .stdout(std::fs::File::create(log)?)
            .stderr(std::fs::File::create(log)?)
            .spawn()
    });
    let mut s = Server::new(dir.clone(), hooks);
    let (err, text, _) = call(&mut s, "abrir_jogo", json!({ "cena": "cenas/nada.cena" }));
    assert!(err && text.contains("cenas/nada.cena não existe"), "{text}");
    let (err, text, _) = call(&mut s, "abrir_jogo", json!({}));
    assert!(err, "{text}");
    assert!(text.contains("o jogo fechou logo ao abrir"), "{text}");
    assert!(
        text.contains("opção desconhecida: --opcao-que-nao-existe"),
        "{text}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
