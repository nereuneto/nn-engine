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
        check_updates: false,
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

/// Os diálogos do gerenciador (tutoriais, projeto novo, exemplos) e as
/// boas-vindas no 3D.
#[test]
#[ignore]
fn dialogos_do_gerenciador() {
    let size = egui::vec2(1280.0, 720.0);
    for (label, name) in [
        ("🎓 Tutoriais", "tutoriais"),
        ("✨ Novo projeto", "novo"),
        ("🎮 Exemplos", "exemplos"),
    ] {
        let dir = temp(&format!("gerenciador-{name}"));
        let mut h = studio(&dir, None, size);
        h.get_by_label(label).click();
        passos(&mut h);
        tela(&mut h, &format!("gerenciador-{name}"));
    }
    let dir = temp("gerenciador-3d");
    let mut h = studio(&dir, None, size);
    h.get_by_label("3D").click();
    passos(&mut h);
    tela(&mut h, "gerenciador-boas-vindas-3d");
    h.get_by_label("🎓 Tutoriais").click();
    passos(&mut h);
    tela(&mut h, "gerenciador-tutoriais-3d");
}

/// Ferramenta: grava as imagens do NN Studio no 3D usadas no README e nos
/// guias (`cargo test -p nn-studio --test revisao capturas -- --ignored`).
#[test]
#[ignore]
fn capturas_do_3d_para_a_documentacao() {
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/imagens");
    let size = egui::vec2(1280.0, 800.0);
    let abrir = |t: Template, name: &str| {
        let dir = temp(&format!("captura-{name}"));
        let proj = dir.join("jogo");
        create_project(&NewOptions {
            dir: proj.clone(),
            title: Some("Meu jogo".into()),
            template: t,
        })
        .unwrap();
        let h = studio(&dir, Some(proj), size);
        (h, dir)
    };
    let selecionar = |h: &mut Harness<'_, StudioApp>, cena: &str, nome: &str| {
        let doc = h
            .state_mut()
            .project_mut()
            .unwrap()
            .scenes
            .get_mut(cena)
            .unwrap();
        let id = doc.data.objects.iter().find(|o| o.name == nome).unwrap().id;
        doc.selection = vec![id];
    };
    let gravar = |h: &mut Harness<'_, StudioApp>, file: &str| {
        passos(h);
        let img = h.render().expect("imagem");
        img.save(out.join(file)).unwrap();
    };

    // A ilha, com o herói selecionado (o Jogador 3D no Inspetor).
    let (mut h, _d) = abrir(Template::Visual3D, "ilha");
    selecionar(&mut h, "cenas/ilha.cena", "Herói");
    gravar(&mut h, "nn-studio-3d.png");
    drop(h);

    // O vale, com o pincel do terreno.
    let (mut h, _d) = abrir(Template::Vale3D, "vale");
    selecionar(&mut h, "cenas/vale.cena", "Terreno");
    passos(&mut h);
    h.get_by_label("✏ Pintar terreno").click();
    gravar(&mut h, "nn-studio-terreno.png");
    drop(h);

    // O editor de conversa, com a conversa do pescador.
    let (mut h, _d) = abrir(Template::Visual3D, "conversa");
    h.state_mut()
        .project_mut()
        .unwrap()
        .open_file("dialogos/pescador.dialogo", None);
    gravar(&mut h, "nn-studio-conversa.png");
}

/// Ferramenta: um projeto feito em outra versão (por exemplo, com o `nn` da
/// v0.1.0) abre no NN Studio, e cada cena e prefab é gravado de novo pelo
/// editor numa cópia: `NN_PROJETO=<pasta> NN_SAIDA=<pasta nova> cargo test
/// -p nn-studio --test revisao projeto_de_fora -- --ignored`. Depois, o
/// teste `projeto_de_fora` do nn-run confere que a cópia ainda roda.
#[test]
#[ignore]
fn projeto_de_fora_abre_e_grava_no_editor() {
    let (Some(src), Some(out)) = (std::env::var_os("NN_PROJETO"), std::env::var_os("NN_SAIDA"))
    else {
        return;
    };
    let (src, out) = (PathBuf::from(src), PathBuf::from(out));
    fn copiar(a: &Path, b: &Path) {
        std::fs::create_dir_all(b).unwrap();
        for e in std::fs::read_dir(a).unwrap().flatten() {
            let to = b.join(e.file_name());
            if e.file_type().unwrap().is_dir() {
                copiar(&e.path(), &to);
            } else {
                std::fs::copy(e.path(), to).unwrap();
            }
        }
    }
    let _ = std::fs::remove_dir_all(&out);
    copiar(&src, &out);
    let dir = temp("de-fora");
    let mut h = studio(&dir, Some(out.clone()), egui::vec2(1280.0, 720.0));
    let files: Vec<String> = h
        .state()
        .project()
        .unwrap()
        .files
        .entries()
        .iter()
        .filter(|e| e.rel.ends_with(".cena") || e.rel.ends_with(".prefab"))
        .map(|e| e.rel.clone())
        .collect();
    for rel in &files {
        h.state_mut().project_mut().unwrap().open_file(rel, None);
        passos(&mut h);
        let ws = h.state_mut().project_mut().unwrap();
        let doc = ws.scenes.get_mut(rel.as_str()).expect("abriu");
        assert_eq!(doc.error, None, "{rel}");
        doc.save(&out).unwrap();
        eprintln!("gravado: {rel}");
    }
}

/// O jogo começa igual ao editor: em cada cena de cada modelo de projeto,
/// cada objeto do arquivo (e os filhos dele) fica no jogo onde o NN Studio
/// mostra. Pega erros como o da 0.2.1, em que as cópias de prefab de uma
/// cena 3D iam para z = 0 só no jogo.
#[test]
fn modelos_o_jogo_comeca_igual_ao_editor() {
    use nn_app::{AppConfig, Context, Game, Headless, Scene};
    use nn_studio::scene_data::{Object, SceneData};
    struct Uma(nn_script::LuaScene);
    impl Game for Uma {
        fn update(&mut self, ctx: &mut Context) {
            let _ = self.0.update(ctx);
        }
        fn render(&mut self, ctx: &mut Context) {
            self.0.render(ctx);
        }
    }
    fn todos(list: &[Object], out: &mut Vec<u32>) {
        for o in list {
            out.push(o.id);
            todos(&o.children, out);
        }
    }
    let mut conferidos = 0;
    for template in Template::ALL {
        let dir = temp(&format!("igual-{}", template.name()));
        create_project(&NewOptions {
            dir: dir.clone(),
            title: None,
            template,
        })
        .unwrap();
        let Ok(list) = std::fs::read_dir(dir.join("cenas")) else {
            continue;
        };
        let mut cenas: Vec<String> = list
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.ends_with(".cena"))
            .collect();
        cenas.sort();
        for nome in cenas {
            let rel = format!("cenas/{nome}");
            let data: SceneData = nn_studio::scene_data::read(
                &std::fs::read_to_string(dir.join(&rel)).unwrap(),
                &rel,
            )
            .unwrap();
            // Os ossos dos modelos (para quem está preso a um osso).
            let mut sizes = nn_studio::scene_render::Sizes::default();
            let mut ids = Vec::new();
            todos(&data.objects, &mut ids);
            for &id in &ids {
                if let Some(m) = data.find(id).and_then(|o| o.component("model")) {
                    sizes.model_bones(&dir, m.str("file"));
                }
            }
            let rt = nn_script::ScriptRuntime::new(&dir).unwrap();
            let rt2 = rt.clone();
            let config = AppConfig::default()
                .size(320, 180)
                .assets_dir(&dir)
                .hot_reload(false);
            let r2 = rel.clone();
            let Ok(h) = Headless::new(config, move |ctx| {
                let mut s = rt.scene(r2);
                s.enter(ctx);
                Uma(s)
            }) else {
                assert!(std::env::var_os("NN_REQUIRE_GPU").is_none());
                return;
            };
            assert_eq!(h.game.0.error(), None, "{} {rel}", template.name());
            rt2.lua()
                .globals()
                .set("C", h.game.0.table().unwrap())
                .unwrap();
            for &id in &ids {
                let o = data.find(id).unwrap();
                // O que fica preso à tela (HUD) é desenhado de outro jeito.
                let mut tela = false;
                let mut cur = Some(id);
                while let Some(c) = cur {
                    let x = data.find(c).unwrap();
                    tela |= data.is_screen(x);
                    cur = data.parent_of(c).flatten();
                }
                if tela {
                    continue;
                }
                let jogo: Option<Vec<f64>> = rt2
                    .lua()
                    .load(format!(
                        "local o = C._by_id[{id}]
                         if not o then return nil end
                         if C._3d then return {{ o:position3d() }} end
                         local x, y = o:world_transform()
                         return {{ x, y, 0 }}"
                    ))
                    .eval()
                    .unwrap();
                let jogo = jogo.unwrap_or_else(|| {
                    panic!(
                        "{} {rel}: \"{}\" ({id}) não está no jogo",
                        template.name(),
                        o.name
                    )
                });
                let (ex, ey, ez) = if data.three_d {
                    let p = data
                        .world_matrix3(id)
                        .unwrap()
                        .transform_point3(glam::Vec3::ZERO);
                    (f64::from(p.x), f64::from(p.y), f64::from(p.z))
                } else {
                    let t = data.world_transform(id).unwrap();
                    (t.x, t.y, 0.0)
                };
                // Preso a um osso: o jogo já toca a animação (alguns cm).
                let tol: f64 = if o.component("bone").is_some() {
                    0.1
                } else {
                    1e-3
                };
                let d = ((jogo[0] - ex).powi(2) + (jogo[1] - ey).powi(2) + (jogo[2] - ez).powi(2))
                    .sqrt();
                assert!(
                    d <= tol.max(ex.abs().max(ey.abs()).max(ez.abs()) * 1e-5),
                    "{} {rel}: \"{}\" ({id}) no editor em ({ex:.3}, {ey:.3}, {ez:.3}), no jogo em ({:.3}, {:.3}, {:.3})",
                    template.name(),
                    o.name,
                    jogo[0],
                    jogo[1],
                    jogo[2]
                );
                conferidos += 1;
            }
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
    assert!(conferidos > 200, "{conferidos}");
}
