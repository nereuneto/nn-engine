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
    assert!(text.starts_with("Regra 1 de 1 na cena: Quando"), "{text}");
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
    let stdin = std::cell::RefCell::new(child.stdin.take().unwrap());
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let raw = |bytes: &[u8]| {
        let mut stdin = stdin.borrow_mut();
        stdin.write_all(bytes).unwrap();
        stdin.flush().unwrap();
    };
    let send = |msg: Value| raw(format!("{msg}\n").as_bytes());
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
    // Lixo na entrada (bytes que não são UTF-8, JSON cortado) vira erro, e o
    // servidor continua.
    raw(b"\xff\xfe{\"jsonrpc\"\n");
    assert_eq!(next()["error"]["code"], -32700);
    raw(b"{\"jsonrpc\": \"2.0\", \"id\": 50, \"method\"\n");
    assert_eq!(next()["error"]["code"], -32700);
    send(json!({ "jsonrpc": "2.0", "id": 51, "method": "tools/call",
        "params": { "name": "ver_cena", "arguments": { "caminho": "cenas/\u{1F600}.cena" } } }));
    let r = next();
    assert_eq!(r["id"], 51);
    assert_eq!(r["result"]["isError"], true, "{r}");
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

/// Pixel art pelo agente: cria a folha, desenha com texto e formas, mexe nos
/// quadros e anima; o NN Studio e o jogo leem igual.
#[test]
fn agente_desenha_um_sprite_e_anima() {
    let dir = temp("pixel");
    let mut s = server(&dir);
    ok(&mut s, "criar_projeto", json!({ "modelo": "vazio" }));
    let img = "assets/imagens/slime.png";
    let t = ok(
        &mut s,
        "criar_imagem",
        json!({ "caminho": img, "largura": 8, "altura": 6, "quadros": 2 }),
    );
    assert!(t.contains("2 quadros lado a lado (folha de 16x6)"), "{t}");
    assert!(fails(&mut s, "criar_imagem", json!({ "caminho": img })).contains("já existe"));

    // Quadro 1: o slime por texto, com contorno; quadro 2: a mesma coisa, achatada.
    let (t, pngs) = images(
        &mut s,
        "desenhar",
        json!({ "caminho": img, "operacoes": [
            { "desenho": ["..GG..", ".GGGG.", "GGWGWG", "GGGGGG"], "cores": { "G": "#38b764", "W": "#f4f4f4" }, "em": [1, 1] },
            { "contorno": true, "cor": "#1a1c2c" },
        ] }),
    );
    assert!(t.contains("2 operação(ões)"), "{t}");
    assert_eq!(pngs.len(), 1, "a prévia do quadro");
    let prev = image::load_from_memory(&pngs[0]).unwrap();
    assert!(prev.width() >= 8 * 16, "ampliada: {}", prev.width());
    ok(
        &mut s,
        "desenhar",
        json!({ "caminho": img, "quadro": 2, "previa": false, "operacoes": [
            { "copiar_quadro": 1 },
            { "deslocar": [0, 1] },
            { "retangulo": [0, 4, 8, 1], "cor": 6, "cheio": true },
        ] }),
    );
    let e = fails(
        &mut s,
        "desenhar",
        json!({ "caminho": img, "operacoes": [{ "desenho": ["X"] }] }),
    );
    assert!(e.contains("o caractere 'X'"), "{e}");
    let e = fails(
        &mut s,
        "desenhar",
        json!({ "caminho": img, "quadro": 3, "operacoes": [{ "balde": [0, 0], "cor": "#ffffff" }] }),
    );
    assert!(e.contains("tem 2 quadro(s)"), "{e}");
    let e = fails(
        &mut s,
        "desenhar",
        json!({ "caminho": img, "operacoes": [{ "pintar": [0, 0] }] }),
    );
    assert!(e.contains("use uma de: pontos"), "{e}");

    // Os pixels de verdade.
    let png = image::open(dir.join(img)).unwrap().to_rgba8();
    assert_eq!(png.dimensions(), (16, 6));
    assert_eq!(png.get_pixel(3, 1).0, [0x38, 0xb7, 0x64, 255]);
    assert_eq!(png.get_pixel(3, 3).0, [0xf4, 0xf4, 0xf4, 255]);
    assert_eq!(png.get_pixel(2, 1).0, [0x1a, 0x1c, 0x2c, 255], "contorno");
    assert_eq!(
        png.get_pixel(8 + 3, 2).0,
        [0x38, 0xb7, 0x64, 255],
        "deslocado no 2º quadro"
    );

    // Ver como texto.
    let t = ok(
        &mut s,
        "ver_imagem",
        json!({ "caminho": img, "quadro": 1, "texto": true }),
    );
    assert!(t.contains("16x6 pixels, 2 quadros de 8x6"), "{t}");
    assert!(t.contains("\n..ABBA..\n"), "{t}");
    assert!(t.contains("A = #1a1c2c, B = #38b764"), "{t}");

    // Um terceiro quadro e a animação.
    ok(
        &mut s,
        "quadros",
        json!({ "caminho": img, "operacoes": [{ "inserir": 3, "copiar": 1 }] }),
    );
    let (t, pngs) = images(
        &mut s,
        "animar",
        json!({ "caminho": img, "nome": "parado", "quadros": [1, 2, 3, 2], "fps": 6, "modo": "repetir" }),
    );
    assert!(
        t.contains("\"parado\": quadros [1, 2, 3, 2], 6 fps, repetir"),
        "{t}"
    );
    assert_eq!(image::load_from_memory(&pngs[0]).unwrap().width() % 4, 0);
    assert_eq!(pngs.len(), 2, "a tira e a casca de cebola");
    assert!(t.contains("Casca de cebola"), "{t}");
    if std::env::var("NN_FX_OUT").is_ok() {
        std::fs::write("/tmp/cebola.png", &pngs[1]).unwrap();
    }
    let e = fails(
        &mut s,
        "animar",
        json!({ "caminho": img, "nome": "x", "quadros": [9] }),
    );
    assert!(e.contains("não há o 9"), "{e}");

    // O .anim é o do NN Studio, e a engine lê.
    let anim = nn_projeto::anim::AnimFile::load(&dir, img).unwrap();
    assert_eq!((anim.frame_w, anim.frame_h), (8, 6));
    assert_eq!(anim.anims[0].name, "parado");
    assert_eq!(anim.anims[0].frames.len(), 4);
    let t = ok(
        &mut s,
        "criar_objeto",
        json!({ "cena": "cenas/fase.cena", "objeto": { "name": "slime", "x": 0, "y": 0,
            "components": [{ "type": "sprite", "image": img, "frame_w": 8, "frame_h": 6, "w": 32, "h": 24, "animation": "parado" }] } }),
    );
    assert!(!t.contains("aviso"), "{t}");
    let t = ok(
        &mut s,
        "rodar_jogo",
        json!({ "cena": "cenas/fase.cena", "segundos": 0.5, "captura_final": false }),
    );
    assert!(t.contains("Erros: nenhum"), "{t}");

    // Vários quadros numa chamada, com as cores da chamada; o texto com as
    // mesmas letras.
    let (t, pngs) = images(
        &mut s,
        "desenhar",
        json!({ "caminho": img, "cores": { "G": "#38b764", "K": "#1a1c2c" }, "operacoes": [
            { "quadro": 1, "apagar": true }, { "quadro": 1, "desenho": ["KGGK"], "em": [2, 2] },
            { "quadro": 3, "apagar": true }, { "quadro": 3, "desenho": ["GKKG"], "em": [2, 2] },
        ] }),
    );
    assert!(t.contains("Os 2 quadros mexidos"), "{t}");
    assert_eq!(image::load_from_memory(&pngs[0]).unwrap().width() % 2, 0);
    let t = ok(
        &mut s,
        "ver_imagem",
        json!({ "caminho": img, "quadro": 3, "texto": true, "cores": { "G": "#38b764", "K": "#1a1c2c" } }),
    );
    assert!(t.contains("\n..GKKG..\n"), "{t}");
    let (t, pngs) = images(
        &mut s,
        "ver_imagem",
        json!({ "caminho": img, "quadro": 1, "repetir": [3, 2] }),
    );
    assert!(t.contains("repetido 3x2"), "{t}");
    let tile = image::load_from_memory(&pngs[0]).unwrap();
    assert_eq!(
        tile.width() * 2,
        tile.height() * 3 * 8 / 6,
        "3 por 2 quadros de 8x6"
    );

    // Um chão repetido com o w da área toda: aviso.
    let t = ok(
        &mut s,
        "criar_objeto",
        json!({ "cena": "cenas/fase.cena", "objeto": { "name": "chao", "x": 0, "y": -100,
            "components": [{ "type": "sprite", "image": img, "frame_w": 8, "frame_h": 6, "w": 320, "h": 6, "fill": "repeat" }] } }),
    );
    assert!(
        t.contains("com fill = \"repeat\", w e h são o tamanho de CADA cópia"),
        "{t}"
    );
    assert!(t.contains("scale_x 40"), "{t}");

    // Pixel art numa tela pequena: as capturas vêm ampliadas.
    ok(
        &mut s,
        "mudar_config",
        json!({ "resolution": [320, 180], "scale": "pixel" }),
    );
    let (t, pngs) = images(
        &mut s,
        "rodar_jogo",
        json!({ "cena": "cenas/fase.cena", "segundos": 0.1 }),
    );
    if !t.contains("GPU") {
        assert!(t.contains("(ampliada 3x)"), "{t}");
        assert_eq!(image::load_from_memory(&pngs[0]).unwrap().width(), 960);
    }

    // Tudo volta com desfazer.
    ok(&mut s, "desfazer", json!({ "passos": 20 }));
    assert!(!dir.join(img).exists());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Mapas de tiles pelo agente: cria o mapa de uma imagem de tiles, pinta
/// (com desenho e com terreno automático), marca os sólidos, e o jogo roda.
#[test]
fn agente_cria_e_pinta_um_mapa() {
    let dir = temp("mapa");
    let mut s = server(&dir);
    ok(&mut s, "criar_projeto", json!({ "modelo": "vazio" }));
    let img = "assets/imagens/tiles.png";
    ok(
        &mut s,
        "criar_imagem",
        json!({ "caminho": img, "largura": 8, "altura": 8, "quadros": 20 }),
    );
    ok(
        &mut s,
        "desenhar",
        json!({ "caminho": img, "previa": false, "operacoes": [
            { "quadro": 1, "balde": [0, 0], "cor": "#5d275d" },
            { "quadro": 2, "retangulo": [0, 0, 8, 2], "cor": "#ef7d57", "cheio": true },
        ] }),
    );
    let map = "mapas/fase.tmj";
    let (t, pngs) = images(
        &mut s,
        "criar_mapa",
        json!({ "caminho": map, "tiles": img, "tile": [8, 8], "tamanho": [20, 6] }),
    );
    assert!(t.contains("20x6 tiles de 8x8"), "{t}");
    assert!(t.contains("tiles 1 a 20"), "{t}");
    assert_eq!(pngs.len(), 1, "o tileset com a grade");
    if std::env::var("NN_FX_OUT").is_ok() {
        std::fs::write("/tmp/tileset.png", &pngs[0]).unwrap();
    }
    assert!(dir.join("assets/imagens/tiles.tsj").is_file());
    assert!(
        fails(
            &mut s,
            "criar_mapa",
            json!({ "caminho": map, "tiles": img })
        )
        .contains("já existe")
    );

    let (t, pngs) = images(
        &mut s,
        "pintar_mapa",
        json!({ "caminho": map, "operacoes": [{
            "desenho": ["####......====..####", "####################"],
            "tiles": { "#": 1, "=": 2 }, "em": [0, 4],
        }] }),
    );
    assert!(t.contains("32 célula(s) mudaram"), "{t}");
    let prev = image::load_from_memory(&pngs[0]).unwrap();
    assert_eq!(prev.width() * 6, prev.height() * 20, "o mapa inteiro");
    assert!(prev.width() >= 960, "ampliado: {}", prev.width());
    let t = ok(
        &mut s,
        "marcar_tiles",
        json!({ "caminho": map, "solidos": [1], "plataformas": [2], "terreno": { "nome": "grama", "primeiro": 5 } }),
    );
    assert!(t.contains("1 sólido(s), 1 plataforma(s)"), "{t}");
    assert!(t.contains("15 das 15"), "{t}");

    let t = ok(&mut s, "ver_mapa", json!({ "caminho": map, "texto": true }));
    assert!(
        t.contains("Sólidos: 1. Plataformas (só por cima): 2."),
        "{t}"
    );
    assert!(t.contains(": grama."), "{t}");
    assert!(t.contains("\n1111......2222..1111\n"), "{t}");
    assert!(t.contains("1 = tile 1, 2 = tile 2"), "{t}");

    // Terreno numa camada nova: as bordas se ajustam.
    let t = ok(
        &mut s,
        "pintar_mapa",
        json!({ "caminho": map, "camada": "grama", "criar_camada": true, "previa": false,
            "operacoes": [{ "terreno": "grama", "retangulo": [2, 1, 3, 2] }] }),
    );
    assert!(t.contains("camada nova \"grama\""), "{t}");
    let doc = nn_projeto::map::MapDoc::open(&dir, map).unwrap();
    let layer = doc.tile_layers()[1];
    assert_eq!(doc.get(layer, 3, 1), 20, "o meio é a peça cheia");
    assert_eq!(
        doc.get(layer, 1, 1),
        5 + 6,
        "a borda da esquerda (cantos da direita)"
    );
    assert_eq!(doc.get(layer, 1, 0), 5 + 2, "o canto de cima à esquerda");
    assert_eq!(doc.get(layer, 0, 0), 0);
    let data = nn_projeto::tileset::TilesetData::load(&dir, "assets/imagens/tiles.tsj").unwrap();
    assert!(data.info(0).solid && data.info(1).one_way);

    // Erros que dizem o que vale.
    let e = fails(
        &mut s,
        "pintar_mapa",
        json!({ "caminho": map, "operacoes": [{ "tile": 1, "pontos": [[25, 0]] }] }),
    );
    assert!(e.contains("fora do mapa (20x6)"), "{e}");
    let e = fails(
        &mut s,
        "pintar_mapa",
        json!({ "caminho": map, "operacoes": [{ "terreno": "areia", "pontos": [[0, 0]] }] }),
    );
    assert!(e.contains("tem: grama"), "{e}");
    let e = fails(
        &mut s,
        "pintar_mapa",
        json!({ "caminho": map, "camada": "x", "operacoes": [] }),
    );
    assert!(e.contains("\"criar_camada\": true"), "{e}");
    let e = fails(
        &mut s,
        "marcar_tiles",
        json!({ "caminho": map, "solidos": [21] }),
    );
    assert!(e.contains("os tiles 1 a 20"), "{e}");

    // Cresce, e o jogo roda com o mapa.
    let t = ok(
        &mut s,
        "pintar_mapa",
        json!({ "caminho": map, "tamanho": [24, 6], "previa": false }),
    );
    assert!(t.contains("24x6 tiles"), "{t}");
    let t = ok(
        &mut s,
        "criar_objeto",
        json!({ "cena": "cenas/fase.cena", "objeto": { "name": "mapa", "x": -96, "y": -48,
            "components": [{ "type": "map", "map": map }] } }),
    );
    assert!(!t.contains("aviso"), "{t}");
    let t = ok(
        &mut s,
        "rodar_jogo",
        json!({ "cena": "cenas/fase.cena", "segundos": 0.3, "captura_final": false }),
    );
    assert!(t.contains("Erros: nenhum"), "{t}");

    ok(&mut s, "desfazer", json!({ "passos": 20 }));
    assert!(!dir.join(map).exists());
    assert!(!dir.join("assets/imagens/tiles.tsj").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Partículas pelo agente: um efeito pronto num objeto novo, a prévia do
/// efeito sozinho, e o mesmo efeito em pixel art numa tela pequena.
#[test]
fn agente_poe_particulas() {
    let dir = temp("particulas");
    let mut s = server(&dir);
    ok(&mut s, "criar_projeto", json!({ "modelo": "vazio" }));
    let cena = "cenas/fase.cena";
    let e = fails(
        &mut s,
        "particulas",
        json!({ "cena": cena, "objeto": "tocha" }),
    );
    assert!(e.contains("Efeitos: fogo"), "{e}");
    let e = fails(
        &mut s,
        "particulas",
        json!({ "cena": cena, "objeto": "tocha", "efeito": "lava" }),
    );
    assert!(e.contains("não há o efeito \"lava\""), "{e}");
    let (t, pngs) = images(
        &mut s,
        "particulas",
        json!({ "cena": cena, "objeto": "tocha", "efeito": "fogo", "x": 100, "y": -50 }),
    );
    assert!(t.contains("Criei o objeto \"tocha\""), "{t}");
    assert!(t.contains("contínuo: 45 por segundo"), "{t}");
    let gpu = !t.contains("Sem prévia");
    if gpu {
        assert_eq!(pngs.len(), 1, "{t}");
        if std::env::var("NN_FX_OUT").is_ok() {
            std::fs::write("/tmp/fx-fogo.png", &pngs[0]).unwrap();
        }
    } else {
        assert!(!gpu_required(), "{t}");
    }
    let doc = std::fs::read_to_string(dir.join(cena)).unwrap();
    assert!(doc.contains("FFE066"), "{doc}");

    // Mudanças de campos e um campo errado.
    let t = ok(
        &mut s,
        "particulas",
        json!({ "cena": cena, "objeto": "tocha", "mudar": { "color_start": "#7ec4ff", "rate": 10 }, "previa": false }),
    );
    assert!(t.contains("10 por segundo") && t.contains("#7ec4ff"), "{t}");
    let e = fails(
        &mut s,
        "particulas",
        json!({ "cena": cena, "objeto": "tocha", "mudar": { "cor": "#fff" } }),
    );
    assert!(e.contains("cor"), "{e}");

    // Pixel art numa tela pequena: a escala vem da tela.
    ok(
        &mut s,
        "mudar_config",
        json!({ "resolution": [320, 180], "scale": "pixel" }),
    );
    let (t, pngs) = images(
        &mut s,
        "particulas",
        json!({ "cena": cena, "objeto": "moeda_fx", "efeito": "faiscas", "pixel": true }),
    );
    assert!(t.contains("pixel art (quadradinhos)"), "{t}");
    assert!(t.contains("× 0.25"), "{t}");
    if gpu {
        let strip = image::load_from_memory(&pngs[0]).unwrap();
        assert!(strip.height() >= 180, "ampliada: {}", strip.height());
        assert!(t.contains("um recorte de"), "{t}");
        if std::env::var("NN_FX_OUT").is_ok() {
            std::fs::write("/tmp/fx-pixel.png", &pngs[0]).unwrap();
        }
    }
    let t = ok(
        &mut s,
        "rodar_jogo",
        json!({ "cena": cena, "segundos": 0.3, "captura_final": false }),
    );
    assert!(t.contains("Erros: nenhum"), "{t}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Efeitos sonoros pelo agente: um tipo pronto, outra variação, um ajuste
/// e o som tocando no jogo.
#[test]
fn agente_cria_sons() {
    let dir = temp("sons");
    let mut s = server(&dir);
    ok(&mut s, "criar_projeto", json!({ "modelo": "vazio" }));
    let wav = "assets/sons/salto.wav";
    let e = fails(&mut s, "criar_som", json!({ "caminho": wav }));
    assert!(e.contains("Tipos: moeda"), "{e}");
    let e = fails(
        &mut s,
        "criar_som",
        json!({ "caminho": wav, "tipo": "miau" }),
    );
    assert!(e.contains("não há o tipo \"miau\""), "{e}");
    let (t, pngs) = images(
        &mut s,
        "criar_som",
        json!({ "caminho": wav, "tipo": "pulo" }),
    );
    assert!(t.contains("Gravei assets/sons/salto.wav"), "{t}");
    assert!(t.contains("Tipo pulo, 0.26 s"), "{t}");
    assert_eq!(pngs.len(), 1);
    let bytes = std::fs::read(dir.join(wav)).unwrap();
    assert_eq!(&bytes[..4], b"RIFF");
    let side = std::fs::read_to_string(dir.join("assets/sons/salto.sfx")).unwrap();
    assert!(side.contains("\"tipo\": \"pulo\""), "{side}");

    // Outra variação, depois um ajuste em cima dela.
    let t = ok(
        &mut s,
        "criar_som",
        json!({ "caminho": wav, "variacao": 5 }),
    );
    assert!(t.contains("Tipo pulo (variação 5)"), "{t}");
    let t = ok(
        &mut s,
        "criar_som",
        json!({ "caminho": wav, "mudar": { "onda": "seno", "queda": 0.5 } }),
    );
    assert!(t.contains("onda: seno") && t.contains("queda: 0.5"), "{t}");
    assert!(t.contains("(variação 5)"), "o tipo e a variação ficam: {t}");
    let e = fails(
        &mut s,
        "criar_som",
        json!({ "caminho": wav, "mudar": { "cor": 1 } }),
    );
    assert!(e.contains("não há o campo \"cor\""), "{e}");
    // Só olhar não grava.
    let before = std::fs::read(dir.join(wav)).unwrap();
    let t = ok(&mut s, "criar_som", json!({ "caminho": wav }));
    assert!(t.starts_with("assets/sons/salto.wav: Tipo pulo"), "{t}");
    assert_eq!(std::fs::read(dir.join(wav)).unwrap(), before);
    // Um .wav que não foi feito aqui não é sobrescrito.
    std::fs::write(dir.join("assets/sons/outro.wav"), b"RIFF").unwrap();
    let e = fails(
        &mut s,
        "criar_som",
        json!({ "caminho": "assets/sons/outro.wav", "tipo": "moeda" }),
    );
    assert!(e.contains("não foi feito aqui"), "{e}");

    if std::env::var("NN_FX_OUT").is_ok() {
        for (k, _, _) in nn_projeto::sfx::KINDS {
            let p = format!("assets/sons/fx_{k}.wav");
            let (_, pngs) = images(&mut s, "criar_som", json!({ "caminho": p, "tipo": k }));
            std::fs::write(format!("/tmp/som-{k}.png"), &pngs[0]).unwrap();
            std::fs::copy(dir.join(&p), format!("/tmp/som-{k}.wav")).unwrap();
        }
    }

    // O jogo toca.
    ok(
        &mut s,
        "criar_objeto",
        json!({ "cena": "cenas/fase.cena", "objeto": { "name": "som", "x": 0, "y": 0,
            "components": [{ "type": "sound", "sound": wav }] } }),
    );
    let t = ok(
        &mut s,
        "rodar_jogo",
        json!({ "cena": "cenas/fase.cena", "segundos": 0.3, "captura_final": false }),
    );
    assert!(t.contains("Erros: nenhum"), "{t}");
    assert!(
        t.contains("Sons que tocaram (sem som aqui; só a lista): assets/sons/salto.wav"),
        "{t}"
    );
    ok(&mut s, "desfazer", json!({ "passos": 30 }));
    assert!(!dir.join(wav).exists());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Diálogos pelo agente: a conversa em JSON, lida de volta no mesmo formato,
/// e o jogo abre a caixa de fala.
#[test]
fn agente_escreve_um_dialogo() {
    let dir = temp("dialogo");
    let mut s = server(&dir);
    ok(&mut s, "criar_projeto", json!({ "modelo": "vazio" }));
    let rel = "dialogos/ana.dialogo";
    let t = ok(
        &mut s,
        "escrever_dialogo",
        json!({ "caminho": rel, "personagens": [{ "nome": "Ana", "voz": "assets/sons/nada.wav" }],
            "falas": [
                { "id": "oi", "quem": "Ana", "texto": "Oi! Viu minha chave?" },
                { "id": "pergunta", "quem": "Ana", "texto": "Me ajuda?", "escolhas": [
                    { "texto": "Claro", "vai": "valeu", "variavel": "ajudou", "valor": 1 },
                    { "texto": "Agora não", "vai": "fim" },
                ] },
                { "id": "valeu", "quem": "Ana", "texto": "Valeu! Tome.", "item": "moeda", "depois": "fim" },
                { "id": "perdida", "quem": "Beto", "texto": "Ninguém me chama." },
            ] }),
    );
    assert!(
        t.contains("Criei dialogos/ana.dialogo: 4 fala(s), 2 personagem(ns)"),
        "{t}"
    );
    assert!(
        t.contains("[1] Ana: Oi! Viu minha chave?\n      → [2]"),
        "{t}"
    );
    assert!(t.contains("1) \"Claro\" (faz ajudou = 1) → [3]"), "{t}");
    assert!(t.contains("nenhum caminho chega em [4]"), "{t}");
    assert!(
        t.contains("voz de Ana: assets/sons/nada.wav não existe"),
        "{t}"
    );
    assert!(t.contains("personagem \"Beto\" criado"), "{t}");

    // Lido de volta no mesmo formato; o que volta reescreve igual.
    let t = ok(&mut s, "ver_dialogo", json!({ "caminho": rel }));
    let start = t.find("{\n").unwrap();
    let end = t.rfind("\n}").unwrap() + 2;
    let mut back: Value = serde_json::from_str(&t[start..end]).unwrap();
    let before = std::fs::read_to_string(dir.join(rel)).unwrap();
    back["caminho"] = json!(rel);
    ok(&mut s, "escrever_dialogo", back);
    assert_eq!(std::fs::read_to_string(dir.join(rel)).unwrap(), before);
    let d = nn_projeto::dialogue::read(&before, rel).unwrap();
    assert_eq!(d.node(2).unwrap().choices.len(), 2);

    let e = fails(
        &mut s,
        "escrever_dialogo",
        json!({ "caminho": rel, "falas": [{ "texto": "a", "depois": "lugar" }] }),
    );
    assert!(e.contains("não há a fala \"lugar\""), "{e}");

    let t = ok(&mut s, "conferir_projeto", json!({}));
    assert!(t.contains("diálogo(s)"), "{t}");
    assert!(
        t.contains("Ana usa assets/sons/nada.wav, que não existe"),
        "{t}"
    );
    let t = ok(&mut s, "ver_dialogo", json!({ "caminho": rel }));
    assert!(t.contains("\"valor\": 1"), "número, como no jogo: {t}");

    // No jogo: a conversa abre quando a cena começa.
    ok(
        &mut s,
        "criar_objeto",
        json!({ "cena": "cenas/fase.cena", "objeto": { "name": "ana", "x": 0, "y": 0,
            "components": [{ "type": "nn:dialogo", "arquivo": rel, "comecar": "cena" }] } }),
    );
    let t = ok(
        &mut s,
        "rodar_jogo",
        json!({ "cena": "cenas/fase.cena", "segundos": 0.5, "captura_final": false }),
    );
    assert!(t.contains("Erros: nenhum"), "{t}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Modelos 3D pelo agente: um pronto, mudado com operações, visto em quatro
/// vistas; e um .glb de fora (com esqueleto e animações) descrito.
#[test]
fn agente_modela_em_3d() {
    let dir = temp("modelos3d");
    let mut s = server(&dir);
    ok(&mut s, "criar_projeto", json!({ "modelo": "3d" }));
    let rel = "modelos/torre.modelo";
    let e = fails(
        &mut s,
        "criar_modelo",
        json!({ "caminho": rel, "a_partir": "castelo" }),
    );
    assert!(e.contains("muro, borda, escada, ponte, placa"), "{e}");
    let (t, pngs) = images(
        &mut s,
        "criar_modelo",
        json!({ "caminho": rel, "a_partir": "muro" }),
    );
    assert!(
        t.contains("Criei modelos/torre.modelo (e modelos/torre.glb)"),
        "{t}"
    );
    assert!(t.contains("Tamanho 2.08 x 1.32 x 0.4 m"), "{t}");
    assert_eq!(pngs.len(), 1);
    assert!(dir.join("modelos/torre.glb").is_file());

    let (t, pngs) = images(
        &mut s,
        "modelar",
        json!({ "caminho": rel, "operacoes": [
            { "criar": "torre", "forma": "cilindro", "lados": 8, "tamanho": [1.2, 3, 1.2], "em": [-1.4, 0, 0], "cor": "#B8B8B8" },
            { "peca": "torre", "extrudar": "cima", "distancia": 1, "afinar": 0 },
            { "peca": "torre", "cor": "#E84A4A", "lado": "cima" },
            { "copiar": "torre", "nome": "torre_d", "espelhar": "x" },
            { "peca": "Muro", "recuar": "frente", "quanto": 0.5, "fundo": -0.1, "cor_dentro": "#1E1E1E" },
        ] }),
    );
    assert!(t.contains("peça \"torre\" (cilindro)"), "{t}");
    assert!(
        t.contains("copiei \"torre\" em \"torre_d\" (espelhada)"),
        "{t}"
    );
    assert!(t.contains("\"torre_d\": em [1.4, 0, 0]"), "{t}");
    assert!(t.contains("vai de y = 0 a 4"), "{t}");
    let img = image::load_from_memory(&pngs[0]).unwrap();
    assert_eq!(img.width(), 602);
    if std::env::var("NN_FX_OUT").is_ok() {
        img.save("/tmp/torre.png").unwrap();
    }
    let e = fails(
        &mut s,
        "modelar",
        json!({ "caminho": rel, "operacoes": [{ "peca": "telhado", "cor": "#fff" }] }),
    );
    assert!(
        e.contains("não há a peça \"telhado\" (tem: Muro, Topo, torre, torre_d)"),
        "{e}"
    );

    // O .glb que o jogo usa é o do modelo.
    let glb = std::fs::read(dir.join("modelos/torre.glb")).unwrap();
    let data = nn_render::d3::ModelData::from_gltf(&glb, &mut |_| None).unwrap();
    let (a, b) = data.bounds();
    assert!((b.y - 4.0).abs() < 0.01 && a.y.abs() < 0.01, "{a} {b}");

    // Um .glb de fora: o herói do modelo 3D, com animações.
    let t = ok(
        &mut s,
        "ver_modelo",
        json!({ "caminho": "modelos/heroi.glb" }),
    );
    assert!(t.contains("Esqueleto:"), "{t}");
    assert!(t.contains("Animações:"), "{t}");
    let anim = t
        .split("Animações: ")
        .nth(1)
        .and_then(|r| r.split(" (").next())
        .unwrap()
        .to_string();
    let (t, pngs) = images(
        &mut s,
        "ver_modelo",
        json!({ "caminho": "modelos/heroi.glb", "animacao": anim, "tempo": 0.3, "vista": "frente" }),
    );
    assert!(t.contains("Na animação"), "{t}");
    assert_eq!(image::load_from_memory(&pngs[0]).unwrap().width(), 600);
    if std::env::var("NN_FX_OUT").is_ok() {
        std::fs::write("/tmp/heroi.png", &pngs[0]).unwrap();
    }
    let e = fails(
        &mut s,
        "ver_modelo",
        json!({ "caminho": "modelos/heroi.glb", "animacao": "voar" }),
    );
    assert!(e.contains("não tem a animação \"voar\""), "{e}");

    // Um valor que não existe num campo de escolha.
    ok(
        &mut s,
        "criar_cena",
        json!({ "caminho": "cenas/teste.cena", "tipo": "3d" }),
    );
    let e = fails(
        &mut s,
        "criar_objeto",
        json!({ "cena": "cenas/teste.cena", "objeto": { "name": "x", "components": [{ "type": "body3d", "shape": "xyz" }] } }),
    );
    assert!(
        e.contains("\"shape\" = \"xyz\" não existe; use \"auto\""),
        "{e}"
    );

    // No jogo.
    let t = ok(
        &mut s,
        "rodar_jogo",
        json!({ "segundos": 0.3, "captura_final": false }),
    );
    assert!(t.contains("Erros: nenhum"), "{t}");
    ok(&mut s, "desfazer", json!({ "passos": 10 }));
    assert!(!dir.join(rel).exists());
    let _ = std::fs::remove_dir_all(&dir);
}
