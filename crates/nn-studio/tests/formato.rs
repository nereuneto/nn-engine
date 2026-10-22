//! As cenas, prefabs e diálogos dos modelos de projeto já vêm no formato que
//! o NN Studio grava (eles moram na `nn-run`, e o arranjo dos Grupos é do
//! editor).

use nn_studio::scene_data::{read, read_prefab, write, write_prefab};

/// Ferramenta: regrava as cenas e prefabs dos modelos no formato do
/// editor (`cargo test -p nn-studio --test formato reescrever_modelos -- --ignored`).
#[test]
#[ignore]
fn reescrever_modelos() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../nn-run/modelos");
    fn walk(dir: &std::path::Path) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p);
                continue;
            }
            let name = p.to_string_lossy().to_string();
            let text = std::fs::read_to_string(&p).unwrap_or_default();
            let out = if name.ends_with(".cena") {
                let mut scene = read(&text, &name).unwrap();
                // Os Grupos já arrumados, como o editor deixa.
                let dir = p.parent().and_then(|d| d.parent()).unwrap();
                nn_studio::scene_render::arrange_all(&mut scene, dir, &mut Default::default());
                write(&scene)
            } else if name.ends_with(".prefab") {
                write_prefab(&read_prefab(&text, &name).unwrap())
            } else {
                continue;
            };
            std::fs::write(&p, out).unwrap();
        }
    }
    walk(&root);
}

#[test]
fn modelos_ja_vem_no_formato_do_editor() {
    // Abrir e salvar pelo editor não muda nada nas cenas e prefabs que
    // vêm prontos nos modelos.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../nn-run/modelos");
    let mut files = Vec::new();
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out);
            } else if p
                .extension()
                .is_some_and(|x| x == "cena" || x == "prefab" || x == "dialogo")
            {
                out.push(p);
            }
        }
    }
    walk(&root, &mut files);
    assert!(files.len() >= 12, "{files:?}");
    for p in files {
        // (No Windows, o git pode trazer o arquivo com CRLF.)
        let text = std::fs::read_to_string(&p).unwrap().replace("\r\n", "\n");
        let name = p.to_string_lossy().to_string();
        let written = if name.ends_with(".prefab") {
            write_prefab(&read_prefab(&text, &name).unwrap())
        } else if name.ends_with(".dialogo") {
            nn_studio::dialogue::write(&nn_studio::dialogue::read(&text, &name).unwrap())
        } else {
            let mut scene = read(&text, &name).unwrap();
            let dir = p.parent().and_then(|d| d.parent()).unwrap();
            nn_studio::scene_render::arrange_all(&mut scene, dir, &mut Default::default());
            write(&scene)
        };
        // NN_REESCREVER=1: grava no formato do editor (para modelos
        // gerados por script).
        if std::env::var_os("NN_REESCREVER").is_some() && written != text {
            std::fs::write(&p, &written).unwrap();
            continue;
        }
        assert_eq!(written, text, "{name}");
    }
    let template = nn_studio::files::visual_scene_template();
    assert_eq!(write(&read(&template, "novo").unwrap()), template);
}
