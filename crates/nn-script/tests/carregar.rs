//! Pré-carregar: o que a cena nova usa carrega antes de ela começar, e o
//! primeiro quadro dela não carrega nada.

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Headless, SceneStack};
use nn_script::ScriptRuntime;

fn projeto(name: &str, files: &[(String, Vec<u8>)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-carregar-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (path, content) in files {
        let full = dir.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }
    dir
}

fn png(w: u32, h: u32, seed: u8) -> Vec<u8> {
    let pixels = (0..w * h * 4)
        .map(|i| (i as u8).wrapping_mul(seed))
        .collect();
    nn_script::game_run::encode_png(w, h, pixels).unwrap()
}

/// Um WAV de 16 bits, mono, com `n` amostras de silêncio.
fn wav(n: u32) -> Vec<u8> {
    let data = n * 2;
    let mut b = Vec::new();
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&22_050u32.to_le_bytes());
    b.extend_from_slice(&44_100u32.to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data.to_le_bytes());
    b.resize(b.len() + data as usize, 0);
    b
}

fn pilha(dir: &Path, start: &str) -> Option<(Headless<SceneStack>, ScriptRuntime)> {
    let rt = ScriptRuntime::new(dir).unwrap();
    let rt2 = rt.clone();
    let start = start.to_string();
    let config = AppConfig::default()
        .size(64, 64)
        .assets_dir(dir)
        .hot_reload(false);
    match Headless::new(config, move |ctx| SceneStack::new(ctx, rt.scene(start))) {
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

/// Uma fase com 20 imagens e 5 sons: os sons nos componentes Som (sem
/// tocar sozinhos) e num comportamento, e o script dela usa mais uma imagem
/// só no `draw` (da lista `preload`).
fn fase(files: &mut Vec<(String, Vec<u8>)>) {
    let mut objs = String::new();
    for i in 0..20 {
        files.push((format!("imgs/i{i}.png"), png(32, 32, i as u8 + 1)));
        objs.push_str(&format!(
            "{{ id = {}, name = \"o{i}\", components = {{ {{ type = \"sprite\", image = \"imgs/i{i}.png\" }} }} }},\n",
            i + 1
        ));
    }
    for i in 0..4 {
        files.push((format!("sons/s{i}.wav"), wav(500 + i)));
        objs.push_str(&format!(
            "{{ id = {}, name = \"s{i}\", components = {{ {{ type = \"sound\", sound = \"sons/s{i}.wav\", autoplay = false }} }} }},\n",
            100 + i
        ));
    }
    files.push(("sons/pega.wav".into(), wav(300)));
    files.push(("comps/pega.lua".into(), br#"local B = { props = { { "som", "" } } }
function B:update() if not self.tocou then self.tocou = true nn.audio.play(nn.assets.sound(self.som)) end end
return B
"#.to_vec()));
    objs.push_str(
        "{ id = 200, name = \"pega\", components = { { type = \"behavior\", script = \"comps/pega.lua\", som = \"sons/pega.wav\" } } },\n",
    );
    files.push(("imgs/extra.png".into(), png(16, 16, 7)));
    files.push((
        "cenas/fase.lua".into(),
        br#"local c = { preload = { "imgs/extra.png" } }
function c:enter() entrou = true end
function c:draw() nn.draw.sprite(nn.assets.texture("imgs/extra.png"), 0, 0) end
return c
"#
        .to_vec(),
    ));
    files.push((
        "cenas/fase.cena".into(),
        format!("return {{ script = \"cenas/fase.lua\", objects = {{\n{objs}}} }}").into_bytes(),
    ));
}

#[test]
fn o_primeiro_quadro_da_cena_nova_nao_carrega_nada() {
    let mut files = vec![(
        "cenas/menu.lua".to_string(),
        br#"local c = {}
function c:update() nn.scene.switch("cenas/fase.cena") end
return c
"#
        .to_vec(),
    )];
    fase(&mut files);
    let dir = projeto("primeiro", &files);
    let Some((mut h, rt)) = pilha(&dir, "cenas/menu.lua") else {
        return;
    };
    // O quadro da troca carrega tudo: 21 imagens e 5 sons.
    h.run_frames(1);
    assert!(rt.lua().load("return entrou").eval::<bool>().unwrap());
    // Exatamente uma vez cada: o pré-carregamento usa as mesmas opções que
    // a cena (senão a mesma imagem carregaria duas vezes).
    let troca = h.ctx.perf.last.loads;
    assert_eq!(troca, 26, "carregados na troca");
    // O primeiro quadro da fase (que desenha a imagem extra e toca o som do
    // comportamento) e os seguintes: nada.
    for _ in 0..3 {
        h.run_frames(1);
        assert_eq!(h.ctx.perf.last.loads, 0);
    }
    assert!(rt.take_errors().is_empty());
}

#[test]
fn a_tela_de_carregamento_mostra_o_andamento() {
    let mut files = vec![
        (
            "cenas/menu.lua".to_string(),
            br#"local c = {}
function c:update() nn.scene.switch("cenas/fase.cena", nil, { loading = "cenas/carregando.lua" }) end
return c
"#
            .to_vec(),
        ),
        (
            "cenas/carregando.lua".to_string(),
            br#"local c = {}
function c:enter() andamento = {} end
function c:update() andamento[#andamento + 1] = nn.scene.progress() end
return c
"#
            .to_vec(),
        ),
    ];
    // Imagens grandes: carregar leva mais que um quadro.
    for i in 0..30 {
        files.push((format!("imgs/g{i}.png"), png(512, 512, i as u8 + 3)));
    }
    fase(&mut files);
    let objs: String = (0..30)
        .map(|i| {
            format!(
                "{{ id = {}, name = \"g{i}\", components = {{ {{ type = \"sprite\", image = \"imgs/g{i}.png\" }} }} }},\n",
                300 + i
            )
        })
        .collect();
    let cena = files
        .iter_mut()
        .find(|(p, _)| p == "cenas/fase.cena")
        .unwrap();
    let text = String::from_utf8(cena.1.clone()).unwrap();
    cena.1 = text
        .replace("objects = {\n", &format!("objects = {{\n{objs}"))
        .into_bytes();
    let dir = projeto("tela", &files);
    let Some((mut h, rt)) = pilha(&dir, "cenas/menu.lua") else {
        return;
    };
    let mut quadros = 0;
    while !rt
        .lua()
        .load("return entrou == true")
        .eval::<bool>()
        .unwrap()
    {
        h.run_frames(1);
        quadros += 1;
        assert!(quadros < 600, "a fase não começou");
    }
    // A tela de carregamento rodou e viu o andamento subir até o fim.
    let andamento: Vec<f32> = rt.lua().load("return andamento").eval().unwrap();
    assert!(!andamento.is_empty());
    assert!(andamento.windows(2).all(|w| w[0] <= w[1]), "{andamento:?}");
    assert!(
        andamento.iter().all(|p| (0.0..=1.0).contains(p)),
        "{andamento:?}"
    );
    // Fora da tela de carregamento, não há andamento.
    assert!(
        rt.lua()
            .load("return nn.scene.progress() == nil")
            .eval::<bool>()
            .unwrap()
    );
    // E a fase não carrega nada no primeiro quadro dela.
    h.run_frames(1);
    assert_eq!(h.ctx.perf.last.loads, 0);
    assert!(rt.take_errors().is_empty());
}

#[test]
fn a_tela_padrao_tambem_funciona() {
    let mut files = vec![(
        "cenas/menu.lua".to_string(),
        br#"local c = {}
function c:update() if not pediu then pediu = true nn.scene.switch("cenas/fase.cena", nil, { loading = true }) end end
return c
"#
        .to_vec(),
    )];
    fase(&mut files);
    let dir = projeto("padrao", &files);
    let Some((mut h, rt)) = pilha(&dir, "cenas/menu.lua") else {
        return;
    };
    h.run_frames(10);
    assert!(rt.lua().load("return entrou").eval::<bool>().unwrap());
    assert!(rt.take_errors().is_empty());
}
