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
    // O jogo rodando sem janela: os prints vão para a resposta, não para a
    // saída.
    send(json!({ "jsonrpc": "2.0", "id": 6, "method": "tools/call",
        "params": { "name": "rodar_jogo", "arguments": { "segundos": 0.5, "captura_final": false,
            "acoes": [ { "em": 0.1, "lua": "print('oi do jogo')" } ] } } }));
    let r = next();
    assert_eq!(r["id"], 6, "{r}");
    let text = r["result"]["content"][0]["text"].as_str().unwrap();
    if r["result"]["isError"] == false {
        assert!(text.contains("0.10 s  oi do jogo"), "{text}");
    } else {
        assert!(!gpu_required(), "{text}");
    }
    drop(stdin);
    assert!(child.wait().unwrap().success());
    let _ = std::fs::remove_dir_all(&dir);
}

fn gpu_required() -> bool {
    std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1")
}

/// As imagens de uma resposta (PNG).
fn images(s: &mut Server, tool: &str, args: Value) -> (String, Vec<Vec<u8>>) {
    let r = s
        .handle(json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": { "name": tool, "arguments": args },
        }))
        .unwrap();
    let content = r["result"]["content"].as_array().unwrap();
    let text = content
        .iter()
        .filter_map(|c| c["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let mut pngs = Vec::new();
    for c in content {
        if c["type"] == "image" {
            assert_eq!(c["mimeType"], "image/png");
            let data = c["data"].as_str().unwrap();
            pngs.push(decode_base64(data));
        }
    }
    (text, pngs)
}

fn decode_base64(s: &str) -> Vec<u8> {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::new();
    let mut buf = 0u32;
    let mut bits = 0;
    for &c in s.as_bytes() {
        if c == b'=' {
            break;
        }
        let v = T.iter().position(|&x| x == c).unwrap() as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    out
}

#[test]
fn agente_roda_o_jogo_e_os_testes() {
    let dir = temp("rodar");
    let mut s = server(&dir);
    ok(&mut s, "criar_projeto", json!({ "modelo": "visual" }));
    // Antes de tudo: a pasta tem um jogo e uma placa de vídeo?
    let (err, text) = call(&mut s, "rodar_jogo", json!({ "segundos": 0 }));
    if err {
        assert!(!gpu_required(), "{text}");
        assert!(text.contains("sem placa de vídeo"), "{text}");
        return;
    }
    assert!(
        text.contains("Rodou 0.02 s de jogo (1 quadros), começando em cenas/titulo.cena"),
        "{text}"
    );

    // O jogador anda, pula e pega uma moeda.
    let (text, pngs) = images(
        &mut s,
        "rodar_jogo",
        json!({
            "cena": "cenas/fase.cena",
            "segundos": 2.5,
            "acoes": [
                { "em": 0.2, "segurar": "right", "por": 1.5 },
                { "em": 1.0, "apertar": "space" },
                { "em": 1.2, "capturar": "pulando" },
            ],
            "objetos": ["heroi"],
            "olhar": ["nn.vars.moedas", "self.objects.nada.x"],
        }),
    );
    assert!(
        text.contains("Rodou 2.50 s de jogo (150 quadros), começando em cenas/fase.cena"),
        "{text}"
    );
    assert!(text.contains("Erros: nenhum."), "{text}");
    assert!(
        text.contains("regra 4 da cena: quando tecla e no_chao → soltar_particulas"),
        "{text}"
    );
    assert!(text.contains("heroi (5): x 0 → "), "{text}");
    assert!(
        text.contains("moedas (15): text.text \"Moedas: 0\" → \"Moedas: 1\""),
        "{text}"
    );
    assert!(
        text.contains("Variáveis que mudaram: moedas 0 → 1"),
        "{text}"
    );
    assert!(text.contains("heroi: {\"id\":5"), "{text}");
    assert!(text.contains("nn.vars.moedas = 1"), "{text}");
    assert!(text.contains("self.objects.nada.x: erro:"), "{text}");
    assert!(text.contains("pulando — 1.20 s, 1280x720"), "{text}");
    assert_eq!(pngs.len(), 2);
    assert!(pngs.iter().all(|p| p.starts_with(b"\x89PNG")));

    // Um erro no script de um comportamento: o jogo para e diz onde.
    ok(
        &mut s,
        "escrever_arquivo",
        json!({ "caminho": "scripts/quebra.lua", "conteudo": "local B = { title = \"Quebra\" }\nfunction B:update(dt)\n  if nn.input.pressed(\"x\") then local v = nil; v.campo = 1 end\nend\nreturn B\n" }),
    );
    ok(
        &mut s,
        "mudar_objeto",
        json!({ "cena": "cenas/fase.cena", "objeto": "heroi", "componentes": [ { "type": "behavior", "script": "scripts/quebra.lua" } ] }),
    );
    let text = ok(
        &mut s,
        "rodar_jogo",
        json!({ "cena": "cenas/fase.cena", "segundos": 2, "acoes": [ { "em": 0.5, "apertar": "x" } ], "captura_final": false }),
    );
    assert!(text.contains("O JOGO PAROU COM ERRO"), "{text}");
    assert!(text.contains("0.50 s  em cenas/fase.cena:"), "{text}");
    assert!(text.contains("scripts/quebra.lua:3"), "{text}");
    assert!(text.starts_with("Rodou 0.52 s"), "{text}");
    ok(&mut s, "desfazer", json!({}));
    ok(&mut s, "desfazer", json!({}));

    // Ações erradas são recusadas antes de rodar.
    let e = fails(
        &mut s,
        "rodar_jogo",
        json!({ "acoes": [ { "em": 1, "apertar": "pulo" } ] }),
    );
    assert!(e.contains("ação 1: tecla desconhecida: \"pulo\""), "{e}");
    let e = fails(&mut s, "rodar_jogo", json!({ "cena": "cenas/nada.cena" }));
    assert!(e.contains("cenas/nada.cena não existe"), "{e}");

    // Os testes que vêm no modelo passam; um teste novo que falha mostra a
    // linha e a tela.
    let text = ok(&mut s, "rodar_testes", json!({}));
    assert!(
        text.starts_with("4 testes: 4 passaram, 0 falharam"),
        "{text}"
    );
    ok(
        &mut s,
        "escrever_arquivo",
        json!({ "caminho": "testes/vidas.lua", "conteudo": "test(\"o herói começa com 5 vidas\", function(t)\n  t:open(\"cenas/fase.cena\")\n  print(\"vidas:\", nn.vars.vidas)\n  t:check_eq(nn.vars.vidas, 5)\nend)\n" }),
    );
    let (text, pngs) = images(
        &mut s,
        "rodar_testes",
        json!({ "arquivo": "testes/vidas.lua" }),
    );
    assert!(
        text.starts_with("1 testes: 0 passaram, 1 falharam"),
        "{text}"
    );
    assert!(
        text.contains("✗ testes/vidas.lua › o herói começa com 5 vidas"),
        "{text}"
    );
    assert!(
        text.contains("testes/vidas.lua:4: esperava 5, veio 3"),
        "{text}"
    );
    assert!(text.contains("vidas:\t3"), "{text}");
    assert_eq!(pngs.len(), 1);
    let text = ok(&mut s, "rodar_testes", json!({ "filtro": "moeda" }));
    assert!(text.starts_with("2 testes: 2 passaram"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn os_testes_dos_modelos_passam() {
    for t in nn_run::Template::ALL {
        let dir = temp(&format!("testes-{}", t.name()));
        nn_run::create_project(&nn_run::NewOptions {
            dir: dir.clone(),
            title: None,
            template: t,
        })
        .unwrap();
        let project = Project::open(&dir).unwrap();
        let opts = nn_script::testing::TestOptions {
            // Pequeno: sem placa de vídeo, desenhar é o que mais demora.
            run: nn_script::game_run::RunOptions {
                size: Some((320, 180)),
                ..Default::default()
            },
            ..Default::default()
        };
        let summary = match nn_script::testing::run_tests(&project, &opts, |_| {}) {
            Ok(s) => s,
            Err(e) => {
                assert!(!gpu_required(), "{e}");
                return;
            }
        };
        let has_tests = dir.join("testes").is_dir();
        assert_eq!(!summary.results.is_empty(), has_tests, "{}", t.name());
        for r in &summary.results {
            assert!(
                r.passed(),
                "{} › {}: {:?}",
                t.name(),
                r.case.name,
                r.failure
            );
        }
        assert!(
            summary.file_errors.is_empty(),
            "{}: {:?}",
            t.name(),
            summary.file_errors
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn nn_testar_diz_o_que_falhou() {
    let dir = temp("testar");
    nn_run::create_project(&nn_run::NewOptions {
        dir: dir.clone(),
        title: None,
        template: nn_run::Template::Visual,
    })
    .unwrap();
    std::fs::write(
        dir.join("testes/quebra.lua"),
        "test(\"cai no abismo\", function(t)\n  t:open(\"cenas/fase.cena\")\n  t:check(false, \"caiu\")\nend)\n",
    )
    .unwrap();
    let caps = dir.join("capturas");
    let out = Command::new(env!("CARGO_BIN_EXE_nn"))
        .args([
            "testar",
            dir.to_str().unwrap(),
            "--tamanho",
            "320x180",
            "--capturas",
        ])
        .arg(&caps)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if stderr.contains("sem placa de vídeo") {
        assert!(!gpu_required(), "{stderr}");
        return;
    }
    assert!(!out.status.success(), "{stdout}");
    assert!(
        stdout.contains("  ✓ testes/jogo.lua › o herói anda e pula"),
        "{stdout}"
    );
    assert!(
        stdout.contains("  ✗ testes/quebra.lua › cai no abismo"),
        "{stdout}"
    );
    assert!(
        stdout.contains("      testes/quebra.lua:3: caiu"),
        "{stdout}"
    );
    assert!(
        stdout.contains("5 testes: 4 passaram, 1 falhou"),
        "{stdout}"
    );
    let pngs = std::fs::read_dir(&caps).unwrap().count();
    assert_eq!(pngs, 1);

    // Só os que passam: sai sem erro.
    let out = Command::new(env!("CARGO_BIN_EXE_nn"))
        .args([
            "testar",
            dir.to_str().unwrap(),
            "--filtro",
            "moeda",
            "--tamanho",
            "320x180",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(
        stdout.contains("2 testes: 2 passaram, 0 falharam"),
        "{stdout}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
