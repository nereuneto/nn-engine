//! `nn mcp`: um agente monta um jogo pelas ferramentas, e o jogo roda.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use nn_app::{AppConfig, Headless, Scene};
use nn_mcp::Server;
use nn_script::{Project, ScriptRuntime};
use serde_json::{Value, json};

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-mcp-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn server(dir: &std::path::Path) -> Server {
    Server::new(dir.to_path_buf(), nn_run::mcp_hooks())
}

/// Chama uma ferramenta: (deu erro?, texto).
fn call(s: &mut Server, tool: &str, args: Value) -> (bool, String) {
    let r = s
        .handle(json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": tool, "arguments": args },
        }))
        .unwrap();
    let result = &r["result"];
    assert!(result.is_object(), "{tool}: {r}");
    let text = result["content"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    (result["isError"].as_bool().unwrap(), text)
}

fn ok(s: &mut Server, tool: &str, args: Value) -> String {
    let (err, text) = call(s, tool, args.clone());
    assert!(!err, "{tool} {args} falhou: {text}");
    text
}

fn fails(s: &mut Server, tool: &str, args: Value) -> String {
    let (err, text) = call(s, tool, args.clone());
    assert!(err, "{tool} {args} devia falhar: {text}");
    text
}

/// Uma cena só, para ver se o script dela deu erro.
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
fn agente_monta_um_jogo_de_plataforma_do_zero() {
    let dir = temp("plataforma");
    let mut s = server(&dir);
    assert!(ok(&mut s, "ver_projeto", json!({})).contains("criar_projeto"));
    ok(
        &mut s,
        "criar_projeto",
        json!({ "modelo": "vazio", "titulo": "Pula Pula" }),
    );
    assert!(dir.join("AGENTS.md").is_file());
    let info = ok(&mut s, "ver_projeto", json!({}));
    assert!(info.contains("\"Pula Pula\""), "{info}");
    let id = Project::open(&dir).unwrap().config.id.unwrap_or_default();
    assert!(id.starts_with("nn-mcp-"), "o id vem do nome da pasta: {id}");

    let fase = "cenas/fase.cena";
    let text = ok(
        &mut s,
        "criar_objeto",
        json!({ "cena": fase, "objetos": [
            { "name": "chao", "y": -300, "components": [
                { "type": "shape", "w": 1280, "h": 60, "color": "#6B4F2A" },
                { "type": "body", "body": "static", "w": 1280, "h": 60 } ] },
            { "name": "heroi", "x": -400, "y": -200, "layer": 2, "components": [
                { "type": "sprite", "image": "assets/imagens/heroi.png", "frame_w": 16, "frame_h": 16, "w": 64, "h": 64 },
                { "type": "nn:plataforma", "velocidade": 300 } ] },
            { "name": "moeda", "x": -200, "y": -200, "components": [
                { "type": "sprite", "image": "assets/imagens/moeda.png", "frame_w": 16, "frame_h": 16, "w": 40, "h": 40 },
                { "type": "behavior", "script": "nn:coletavel", "variavel": "moedas" } ] },
        ] }),
    );
    // O jogador de plataforma precisa de corpo: ganhou um.
    assert!(text.contains("ganhou um body"), "{text}");
    ok(
        &mut s,
        "criar_prefab",
        json!({ "cena": fase, "objeto": "moeda" }),
    );
    assert!(dir.join("prefabs/moeda.prefab").is_file());
    let text = ok(
        &mut s,
        "copiar_objeto",
        json!({ "cena": fase, "objeto": "moeda", "copias": 3, "passo_x": 80 }),
    );
    assert!(text.contains("(id 6)"), "{text}");
    ok(
        &mut s,
        "escrever_arquivo",
        json!({ "caminho": "scripts/girar.lua", "conteudo":
            "local B = {\n  title = \"Girar\",\n  props = { { \"velocidade\", 90 } },\n}\n\
             function B:update(dt)\n  self.obj.rotation = self.obj.rotation + self.velocidade * dt\nend\nreturn B\n" }),
    );
    ok(
        &mut s,
        "mudar_objeto",
        json!({ "cena": "prefabs/moeda.prefab", "objeto": "moeda",
                "componentes": [ { "type": "behavior", "script": "scripts/girar.lua", "velocidade": 180 } ] }),
    );
    let text = ok(
        &mut s,
        "por_regra",
        json!({ "cena": fase, "regra": {
            "when": [ { "cond": "acabaram", "nome": "moeda" } ],
            "actions": [ { "act": "ir_para_cena", "cena": fase } ] } }),
    );
    assert!(text.starts_with("Regra 1 de 1 em a cena: Quando"), "{text}");
    // O exemplo da documentação (docs/mcp.md e a descrição de por_regra).
    let text = ok(
        &mut s,
        "por_regra",
        json!({ "cena": fase, "objeto": "heroi", "regra": {
            "when": [{"cond": "pegar", "quem": "@jogador", "item": "moeda"}],
            "actions": [{"act": "som", "som": "assets/sons/moeda.wav"}] } }),
    );
    assert!(text.contains("em \"heroi\""), "{text}");
    let e = fails(
        &mut s,
        "por_regra",
        json!({ "cena": fase, "regra": { "actions": [{"act": "tocar_som"}] } }),
    );
    assert!(e.contains("parecidos: som"), "{e}");
    ok(
        &mut s,
        "mudar_config",
        json!({ "title": "Pula Pula!", "vars": { "tempo": 60 } }),
    );
    let check = ok(&mut s, "conferir_projeto", json!({}));
    assert!(check.contains("Tudo certo"), "{check}");

    // O NN Studio grava exatamente o mesmo arquivo.
    for f in [fase, "prefabs/moeda.prefab"] {
        let text = std::fs::read_to_string(dir.join(f)).unwrap();
        let again = if f.ends_with(".prefab") {
            nn_projeto::scene_data::write_prefab(
                &nn_projeto::scene_data::read_prefab(&text, f).unwrap(),
            )
        } else {
            nn_projeto::scene_data::write(&nn_projeto::scene_data::read(&text, f).unwrap())
        };
        assert_eq!(again, text, "{f}");
    }
    // ver_cena mostra o que foi feito, no formato do arquivo.
    let scene = ok(
        &mut s,
        "ver_cena",
        json!({ "cena": fase, "objeto": "heroi" }),
    );
    assert!(scene.contains("\"script\": \"nn:plataforma\""), "{scene}");
    assert!(scene.contains("\"velocidade\": 300"), "{scene}");

    // O jogo roda a fase sem erro.
    let project = Project::open(&dir).unwrap();
    assert_eq!(project.config.title, "Pula Pula!");
    let rt = ScriptRuntime::new(&dir).unwrap();
    assert!(rt.check_syntax().is_empty());
    let config: AppConfig = project
        .config
        .clone()
        .size(320, 180)
        .save_dir(dir.join("dados"))
        .hot_reload(false);
    let h = Headless::new(config, move |ctx| {
        let mut scene = rt.scene(fase);
        scene.enter(ctx);
        Uma(scene)
    });
    let mut h = match h {
        Ok(h) => h,
        Err(e) => {
            if std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1") {
                panic!("GPU obrigatória: {e}");
            }
            return;
        }
    };
    h.run_frames(90);
    assert_eq!(h.game.0.error(), None);
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn erros_dizem_o_que_vale() {
    let dir = temp("erros");
    let mut s = server(&dir);
    ok(&mut s, "criar_projeto", json!({ "modelo": "vazio" }));
    let fase = "cenas/fase.cena";
    let e = fails(
        &mut s,
        "criar_objeto",
        json!({ "cena": fase, "objeto": { "name": "x", "components": [ { "type": "sprite", "imagem": "a.png" } ] } }),
    );
    assert!(
        e.contains("\"imagem\"") && e.contains("image, frame_w"),
        "{e}"
    );
    let e = fails(
        &mut s,
        "criar_objeto",
        json!({ "cena": fase, "objeto": { "name": "x", "components": [ { "type": "nn:patrulha", "velocidade": "rapido" } ] } }),
    );
    assert!(e.contains("número"), "{e}");
    let e = fails(
        &mut s,
        "criar_objeto",
        json!({ "cena": fase, "objeto": { "name": "x", "components": [ { "type": "shape", "color": "vermelho" } ] } }),
    );
    assert!(e.contains("#RRGGBB"), "{e}");
    let e = fails(
        &mut s,
        "criar_objeto",
        json!({ "cena": fase, "objeto": { "name": "x", "components": [ { "type": "voar" } ] } }),
    );
    assert!(
        e.contains("componente desconhecido") && e.contains("sprite"),
        "{e}"
    );
    let e = fails(
        &mut s,
        "criar_objeto",
        json!({ "cena": fase, "objeto": { "name": "x", "components": [ { "type": "nn:vida", "ao_acabar": "explodir" } ] } }),
    );
    assert!(e.contains("Opções") && e.contains("\"reiniciar\""), "{e}");
    let e = fails(
        &mut s,
        "por_regra",
        json!({ "cena": fase, "regra": { "when": [ { "cond": "tecla" } ], "actions": [ { "act": "trocar_cena" } ] } }),
    );
    assert!(e.contains("listar_eventos"), "{e}");
    let e = fails(
        &mut s,
        "escrever_arquivo",
        json!({ "caminho": "scripts/ruim.lua", "conteudo": "function x(\n  return 1\nend" }),
    );
    assert!(e.contains("linha 2"), "{e}");
    assert!(!dir.join("scripts/ruim.lua").exists());
    let e = fails(
        &mut s,
        "ver_cena",
        json!({ "cena": fase, "objeto": "ninguem" }),
    );
    assert!(e.contains("não há objeto"), "{e}");
    // Nada disso mudou a cena.
    let steps = ok(&mut s, "desfazer", json!({ "listar": true }));
    assert!(steps.lines().count() == 2, "{steps}");
    // Arquivo que não existe: grava, mas avisa; e a conferência acha.
    let text = ok(
        &mut s,
        "criar_objeto",
        json!({ "cena": fase, "objeto": { "name": "fantasma", "components": [
            { "type": "sprite", "image": "assets/imagens/nao_tem.png" } ] } }),
    );
    assert!(text.contains("nao_tem.png\" não existe"), "{text}");
    let check = ok(&mut s, "conferir_projeto", json!({}));
    assert!(
        check.contains("1 problema(s)") && check.contains("nao_tem.png"),
        "{check}"
    );
    ok(&mut s, "desfazer", json!({}));
    assert!(ok(&mut s, "conferir_projeto", json!({})).contains("Tudo certo"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn desfazer_volta_os_arquivos_como_estavam() {
    let dir = temp("desfazer");
    let mut s = server(&dir);
    ok(&mut s, "criar_projeto", json!({ "modelo": "visual" }));
    let fase = "cenas/fase.cena";
    let before = std::fs::read(dir.join(fase)).unwrap();
    ok(
        &mut s,
        "mudar_objeto",
        json!({ "cena": fase, "objeto": "heroi", "mudar": { "x": 99 } }),
    );
    ok(
        &mut s,
        "criar_prefab",
        json!({ "cena": fase, "objeto": "inimigo" }),
    );
    ok(
        &mut s,
        "apagar_objeto",
        json!({ "cena": fase, "objeto": "mapa" }),
    );
    assert_ne!(std::fs::read(dir.join(fase)).unwrap(), before);
    // Mudou por fora: só desfaz forçando.
    std::fs::write(dir.join(fase), "-- mexi à mão\n").unwrap();
    let e = fails(&mut s, "desfazer", json!({}));
    assert!(e.contains("forcar"), "{e}");
    ok(&mut s, "desfazer", json!({ "forcar": true }));
    ok(&mut s, "desfazer", json!({ "passos": 2 }));
    assert_eq!(std::fs::read(dir.join(fase)).unwrap(), before);
    assert!(!dir.join("prefabs/inimigo.prefab").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn nao_sai_da_pasta_do_projeto() {
    let dir = temp("pasta");
    let mut s = server(&dir);
    ok(&mut s, "criar_projeto", json!({ "modelo": "minimo" }));
    for path in [
        "../fora.lua",
        "/tmp/fora.lua",
        "cenas/../../fora.lua",
        ".git/config",
    ] {
        fails(
            &mut s,
            "escrever_arquivo",
            json!({ "caminho": path, "conteudo": "x = 1" }),
        );
        fails(&mut s, "ler_arquivo", json!({ "caminho": path }));
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(std::env::temp_dir(), dir.join("atalho")).unwrap();
        let e = fails(
            &mut s,
            "escrever_arquivo",
            json!({ "caminho": "atalho/fora.lua", "conteudo": "x = 1" }),
        );
        assert!(e.contains("fora da pasta"), "{e}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn todos_os_modelos_passam_na_conferencia() {
    // Sem alarme falso: o que vem pronto abre e confere sem problemas.
    for t in nn_run::Template::ALL {
        let dir = temp(&format!("modelo-{}", t.name()));
        let mut s = server(&dir);
        ok(&mut s, "criar_projeto", json!({ "modelo": t.name() }));
        let check = ok(&mut s, "conferir_projeto", json!({}));
        assert!(
            check.contains("Tudo certo") || check.contains("Nenhum problema"),
            "{}: {check}",
            t.name()
        );
        for f in std::fs::read_dir(dir.join("cenas")).into_iter().flatten() {
            let f = f.unwrap().file_name().to_string_lossy().into_owned();
            if f.ends_with(".cena") {
                ok(&mut s, "ver_cena", json!({ "cena": format!("cenas/{f}") }));
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn nn_mcp_so_escreve_o_protocolo_na_saida() {
    let dir = temp("stdio");
    // Um comportamento que escreve na saída ao ser lido: não pode atrapalhar
    // o protocolo.
    std::fs::create_dir_all(dir.join("scripts")).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_nn"))
        .args(["mcp", dir.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut send = |msg: Value| {
        writeln!(stdin, "{msg}").unwrap();
        stdin.flush().unwrap();
    };
    let mut next = || -> Value { serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap() };
    send(json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "teste", "version": "1" } } }));
    let r = next();
    assert_eq!(r["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(
        r["result"]["serverInfo"]["version"],
        env!("CARGO_PKG_VERSION")
    );
    send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
    send(json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": { "name": "criar_projeto", "arguments": { "modelo": "vazio" } } }));
    assert_eq!(next()["result"]["isError"], false);
    send(json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call",
        "params": { "name": "escrever_arquivo", "arguments": { "caminho": "scripts/fala.lua",
            "conteudo": "print(\"oi da saída\")\nio = io or {}\nreturn { title = \"Fala\", props = { { \"n\", 1 } } }\n" } } }));
    let r = next();
    assert_eq!(r["id"], 3);
    assert_eq!(r["result"]["isError"], false, "{r}");
    send(json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/call",
        "params": { "name": "listar_comportamentos", "arguments": {} } }));
    let r = next();
    assert_eq!(r["id"], 4, "{r}");
    assert!(
        r["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("scripts/fala.lua")
    );
    send(json!({ "jsonrpc": "2.0", "id": 5, "method": "ping" }));
    assert_eq!(next()["id"], 5);
    drop(stdin);
    assert!(child.wait().unwrap().success());
    let _ = std::fs::remove_dir_all(&dir);
}
