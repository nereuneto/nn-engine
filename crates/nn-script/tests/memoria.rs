//! Trocar de cena libera o que a cena nova não usa (imagens, sons, malhas).

use std::path::{Path, PathBuf};

use nn_app::{AppConfig, Headless, SceneStack};
use nn_script::ScriptRuntime;

fn projeto(name: &str, files: &[(&str, Vec<u8>)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-memoria-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (path, content) in files {
        let full = dir.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }
    dir
}

fn png(w: u32, h: u32) -> Vec<u8> {
    nn_script::game_run::encode_png(w, h, vec![200; (w * h * 4) as usize]).unwrap()
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

/// Uma cena visual com estas imagens e o script dela.
fn cena(images: &[&str], script: &str) -> Vec<u8> {
    cena_com_modelo(images, None, script)
}

/// Como `cena`, com um modelo 3D.
fn cena_com_modelo(images: &[&str], model: Option<&str>, script: &str) -> Vec<u8> {
    let mut objs: Vec<String> = images
        .iter()
        .enumerate()
        .map(|(i, img)| {
            format!(
                "{{ id = {}, name = \"o{i}\", components = {{ {{ type = \"sprite\", image = \"{img}\" }} }} }},",
                i + 1
            )
        })
        .collect();
    if let Some(m) = model {
        objs.push(format!(
            "{{ id = 99, name = \"modelo\", components = {{ {{ type = \"model\", file = \"{m}\" }} }} }},"
        ));
    }
    format!(
        "return {{ script = \"{script}\", objects = {{ {} }} }}",
        objs.join(" ")
    )
    .into_bytes()
}

/// O script: cria uma malha 3D (de `lado` de tamanho), pede um som e guarda
/// a música, e troca para `next` no primeiro quadro.
fn script(next: &str, lado: u32) -> Vec<u8> {
    format!(
        r#"local c = {{}}
function c:enter()
  self.malha = nn.d3.mesh({{ points = {{ {{0,0,0}}, {{{lado},0,0}}, {{0,{lado},0}} }}, triangles = {{ {{1,2,3}} }} }})
  self.som = nn.assets.sound("sons/efeito.wav")
  self.musica = nn.assets.keep(nn.assets.sound("sons/musica.wav"))
end
function c:update()
  trocas = (trocas or 0) + 1
  nn.scene.switch("{next}")
end
return c
"#
    )
    .into_bytes()
}

#[test]
fn trocar_de_cena_100_vezes_nao_aumenta_a_memoria() {
    let dir = projeto(
        "trocas",
        &[
            ("imgs/a1.png", png(64, 64)),
            ("imgs/b1.png", png(128, 32)),
            ("imgs/comum.png", png(16, 16)),
            ("sons/efeito.wav", wav(2000)),
            ("sons/musica.wav", wav(4000)),
            ("cenas/a.lua", script("cenas/b.cena", 1)),
            ("cenas/b.lua", script("cenas/a.cena", 2)),
            (
                "cenas/a.cena",
                cena(&["imgs/a1.png", "imgs/comum.png"], "cenas/a.lua"),
            ),
            (
                "cenas/b.cena",
                cena(&["imgs/b1.png", "imgs/comum.png"], "cenas/b.lua"),
            ),
        ],
    );
    let Some((mut h, rt)) = pilha(&dir, "cenas/a.cena") else {
        return;
    };
    let medida = |h: &Headless<SceneStack>| {
        (
            h.ctx.renderer.texture_bytes(),
            h.ctx.renderer.texture_count(),
            h.ctx.renderer.mesh_count(),
            h.ctx.audio.sound_count(),
        )
    };
    let no_comeco = medida(&h);
    // Uma volta: a, b, a.
    h.run_frames(2);
    let depois_de_uma = medida(&h);
    assert_eq!(no_comeco, depois_de_uma, "(bytes, texturas, malhas, sons)");
    h.run_frames(200);
    let trocas: i64 = rt.lua().load("return trocas").eval().unwrap();
    assert!(trocas >= 200, "{trocas}");
    assert!(rt.take_errors().is_empty());
    let depois_de_cem = medida(&h);
    assert_eq!(
        depois_de_uma, depois_de_cem,
        "(bytes, texturas, malhas, sons)"
    );
    // Na cena a: os dois sons (o efeito da cena e a música guardada).
    assert_eq!(depois_de_cem.3, 2);
    // E só as imagens dela (a b1, da outra cena, saiu da memória).
    let base = base_bytes(&h);
    assert_eq!(depois_de_cem.0 - base, (64 * 64 + 16 * 16) * 4);
    h.run_frames(1);
    assert_eq!(
        h.ctx.renderer.texture_bytes() - base,
        (128 * 32 + 16 * 16) * 4
    );
}

#[test]
fn os_modelos_da_outra_cena_saem() {
    let troca = |next: &str| {
        format!("local c = {{}}\nfunction c:update() nn.scene.switch(\"{next}\") end\nreturn c\n")
            .into_bytes()
    };
    let dir = projeto(
        "modelos",
        &[
            ("modelos/caixote.glb", glb("caixote")),
            ("modelos/pedra.glb", glb("pedra")),
            ("cenas/a.lua", troca("cenas/b.cena")),
            ("cenas/b.lua", troca("cenas/a.cena")),
            (
                "cenas/a.cena",
                cena_com_modelo(&[], Some("modelos/caixote.glb"), "cenas/a.lua"),
            ),
            (
                "cenas/b.cena",
                cena_com_modelo(&[], Some("modelos/pedra.glb"), "cenas/b.lua"),
            ),
        ],
    );
    let Some((mut h, rt)) = pilha(&dir, "cenas/a.cena") else {
        return;
    };
    let em_a = (h.ctx.renderer.mesh_count(), h.ctx.renderer.texture_bytes());
    assert!(h.ctx.model("modelos/caixote.glb").is_some());
    h.run_frames(1);
    assert!(h.ctx.model("modelos/caixote.glb").is_none());
    assert!(h.ctx.model("modelos/pedra.glb").is_some());
    h.run_frames(101);
    assert!(rt.take_errors().is_empty());
    assert!(h.ctx.model("modelos/pedra.glb").is_none());
    assert_eq!(
        (h.ctx.renderer.mesh_count(), h.ctx.renderer.texture_bytes()),
        em_a
    );
}

/// Um modelo do jogo de exemplo cubos3d.
fn glb(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../jogos/cubos3d/modelos")
        .join(format!("{name}.glb"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn o_que_outra_cena_ainda_usa_nao_sai() {
    // Uma pausa por cima do jogo: o jogo continua com as imagens dele.
    let dir = projeto(
        "pausa",
        &[
            ("imgs/jogo.png", png(32, 32)),
            ("imgs/pausa.png", png(8, 8)),
            ("cenas/jogo.cena", cena(&["imgs/jogo.png"], "cenas/jogo.lua")),
            (
                "cenas/jogo.lua",
                b"local c = {}\nfunction c:update() if not pausou then pausou = true nn.scene.push(\"cenas/pausa.cena\") end end\nreturn c\n".to_vec(),
            ),
            ("cenas/pausa.cena", cena(&["imgs/pausa.png"], "cenas/pausa.lua")),
            ("cenas/pausa.lua", b"return {}".to_vec()),
        ],
    );
    let Some((mut h, _)) = pilha(&dir, "cenas/jogo.cena") else {
        return;
    };
    h.run_frames(3);
    // As duas imagens: a do jogo (embaixo) e a da pausa.
    assert_eq!(
        h.ctx.renderer.texture_bytes() - base_bytes(&h),
        (32 * 32 + 8 * 8) * 4
    );
}

/// As texturas da própria engine (o branco, o círculo...): o que sobra sem
/// nenhuma imagem do jogo.
fn base_bytes(h: &Headless<SceneStack>) -> u64 {
    let dir = projeto("vazio", &[("main.lua", b"return {}".to_vec())]);
    let Some((h2, _)) = pilha(&dir, "main.lua") else {
        return 0;
    };
    let _ = h;
    h2.ctx.renderer.texture_bytes()
}
