//! Ferramenta de revisão: grava as telas principais do NN Studio em dois
//! tamanhos de janela, com cada modelo aberto, para conferir a aparência.
//!
//! `NN_STUDIO_TELAS=<pasta> cargo test -p nn-studio --test revisao -- --ignored`

use std::path::{Path, PathBuf};

use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use nn_run::{NewOptions, Template, create_project};
use nn_studio::app::{StudioApp, StudioOptions};

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("nn-revisao-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn studio(dir: &Path, open: Option<PathBuf>, size: egui::Vec2) -> Harness<'static, StudioApp> {
    let options = StudioOptions {
        settings_path: Some(dir.join("preferencias.json")),
        open,
        exe: None,
    };
    let mut h = Harness::builder()
        .with_size(size)
        .wgpu()
        .build_eframe(move |cc| {
            let mut app = StudioApp::new(&cc.egui_ctx, options);
            app.set_render_state(cc.wgpu_render_state.clone());
            app
        });
    passos(&mut h);
    h
}

/// Alguns quadros, com tempo de relógio (animações e partículas).
fn passos(h: &mut Harness<'_, StudioApp>) {
    for _ in 0..4 {
        std::thread::sleep(std::time::Duration::from_millis(60));
        h.run_steps(1);
    }
}

fn tela(h: &mut Harness<'_, StudioApp>, name: &str) {
    let Some(dir) = std::env::var_os("NN_STUDIO_TELAS") else {
        return;
    };
    let dir = PathBuf::from(dir).join("revisao");
    let _ = std::fs::create_dir_all(&dir);
    match h.render() {
        Ok(img) => img.save(dir.join(format!("{name}.png"))).unwrap(),
        Err(e) => eprintln!("sem imagem da tela {name}: {e}"),
    }
}

#[test]
#[ignore]
fn telas_para_revisar() {
    for (w, hgt) in [(1280.0, 720.0), (1920.0, 1080.0)] {
        let size = egui::vec2(w, hgt);
        let tag = format!("{}", w as u32);
        // Sem projeto: o gerenciador.
        let dir = temp(&format!("gerenciador-{tag}"));
        let mut h = studio(&dir, None, size);
        tela(&mut h, &format!("{tag}-gerenciador"));
        drop(h);
        for t in Template::ALL {
            let dir = temp(&format!("{}-{tag}", t.name()));
            let proj = dir.join("jogo");
            create_project(&NewOptions {
                dir: proj.clone(),
                title: None,
                template: t,
            })
            .unwrap();
            let mut h = studio(&dir, Some(proj.clone()), size);
            let project = nn_script::Project::open(&proj).unwrap();
            let start = project.start.clone();
            h.state_mut().project_mut().unwrap().open_file(&start, None);
            passos(&mut h);
            tela(&mut h, &format!("{tag}-{}-inicio", t.name()));
            // Uma cena de jogo (a segunda cena visual, se houver).
            let scenes: Vec<String> = h
                .state()
                .project()
                .unwrap()
                .files
                .entries()
                .iter()
                .filter(|e| e.rel.ends_with(".cena") && e.rel != start)
                .map(|e| e.rel.clone())
                .collect();
            if let Some(s) = scenes.first() {
                h.state_mut().project_mut().unwrap().open_file(s, None);
                passos(&mut h);
                // Seleciona o primeiro objeto da Hierarquia.
                let first = h.state().project().unwrap().scenes[s.as_str()]
                    .data
                    .objects
                    .first()
                    .map(|o| o.name.clone());
                if let Some(n) = first {
                    if let Some(node) = h.get_all_by_label(&n).next() {
                        node.click();
                    }
                    passos(&mut h);
                }
                tela(&mut h, &format!("{tag}-{}-cena", t.name()));
            }
            drop(h);
        }
    }
}

/// Quanto tempo o editor leva por quadro com uma cena grande (2000 objetos,
/// 20 emissores de partículas).
#[test]
#[ignore]
fn cena_grande_quadros_por_segundo() {
    let dir = temp("grande");
    let proj = dir.join("jogo");
    create_project(&NewOptions {
        dir: proj.clone(),
        title: None,
        template: Template::Vazio,
    })
    .unwrap();
    let mut objs = String::new();
    for i in 0..2000 {
        let (x, y) = (
            (i % 50) as f64 * 40.0 - 1000.0,
            (i / 50) as f64 * 40.0 - 800.0,
        );
        objs.push_str(&format!(
            "{{ id = {}, name = \"b{i}\", x = {x}, y = {y}, components = {{ {{ type = \"sprite\", image = \"assets/imagens/bloco.png\", w = 32, h = 32 }} }} }},\n",
            i + 1
        ));
    }
    for i in 0..20 {
        objs.push_str(&format!(
            "{{ id = {}, name = \"fx{i}\", x = {}, y = 300, components = {{ {{ type = \"particles\", rate = 60 }} }} }},\n",
            3000 + i,
            i as f64 * 80.0 - 800.0
        ));
    }
    std::fs::write(
        proj.join("cenas/fase.cena"),
        format!("return {{ format = 1, objects = {{ {objs} }} }}"),
    )
    .unwrap();
    let mut h = studio(&dir, Some(proj.clone()), egui::vec2(1600.0, 900.0));
    h.state_mut()
        .project_mut()
        .unwrap()
        .open_file("cenas/fase.cena", None);
    passos(&mut h);
    let start = std::time::Instant::now();
    let n = 60;
    h.run_steps(n);
    let ms = start.elapsed().as_secs_f64() * 1000.0 / n as f64;
    eprintln!("cena grande: {ms:.1} ms por quadro");
}
