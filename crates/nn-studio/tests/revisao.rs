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
                // Preso a um osso (ou filho de quem está): o jogo já toca a
                // animação (alguns cm).
                let mut tela = false;
                let mut osso = false;
                let mut cur = Some(id);
                while let Some(c) = cur {
                    let x = data.find(c).unwrap();
                    tela |= data.is_screen(x);
                    osso |= x.component("bone").is_some();
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
                let tol: f64 = if osso { 0.1 } else { 1e-3 };
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

/// Uso de ponta a ponta da etapa "Studio mais gostoso de usar" (v0.5): monta
/// uma cena 3D e uma 2D só com o que entrou nela, gravando cada tela.
#[test]
#[ignore]
fn uso_de_ponta_a_ponta_v05() {
    use egui::{Key, Modifiers};
    let dir = temp("ponta-a-ponta");
    let proj = dir.join("ilha");
    create_project(&NewOptions {
        dir: proj.clone(),
        title: None,
        template: Template::Visual3D,
    })
    .unwrap();
    let mut h = studio(&dir, Some(proj.clone()), egui::vec2(1280.0, 720.0));
    let ws = h.state_mut().project_mut().unwrap();
    ws.open_file("cenas/ilha.cena", None);
    passos(&mut h);
    passos(&mut h);
    fn cena<'a>(h: &'a mut Harness<'_, StudioApp>) -> &'a mut nn_studio::scene_view::SceneDoc {
        h.state_mut()
            .project_mut()
            .unwrap()
            .scenes
            .get_mut("cenas/ilha.cena")
            .unwrap()
    }
    let rect = h.get_by_label("Vista da cena 3D").rect();
    let direito = |h: &mut Harness<'_, StudioApp>, p: egui::Pos2| {
        h.hover_at(p);
        h.step();
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: p,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: Modifiers::NONE,
            });
            h.step();
        }
        passos(h);
    };
    // (A Hierarquia e o Inspetor também têm "Luz...": fica o do meio.)
    let meio = (rect.left(), rect.right() + 40.0);
    let submenu = |h: &mut Harness<'_, StudioApp>, nome: &str| {
        h.get_all_by(|n| n.label().is_some_and(|l| l.starts_with(nome) && l != nome))
            .find(|n| n.rect().left() > meio.0 && n.rect().left() < meio.1)
            .unwrap()
            .hover();
        passos(h);
    };
    let ponto = |_: &mut Harness<'_, StudioApp>, x: f32, y: f32| rect.min + egui::vec2(x, y);
    tela(&mut h, "e2e-3d-01-aberta");

    // Botão direito no chão: Luz > Tocha.
    let p = ponto(&mut h, 330.0, 215.0);
    direito(&mut h, p);
    tela(&mut h, "e2e-3d-02-menu");
    submenu(&mut h, "Luz");
    h.get_by_label("Tocha").click();
    passos(&mut h);
    tela(&mut h, "e2e-3d-03-tocha");
    // Ctrl+D duas vezes: uma fileira.
    h.hover_at(p);
    h.step();
    for _ in 0..2 {
        h.key_press_modifiers(Modifiers::COMMAND, Key::D);
        passos(&mut h);
    }
    tela(&mut h, "e2e-3d-04-fileira");
    // Shift+A: a busca.
    let p2 = ponto(&mut h, 250.0, 240.0);
    h.hover_at(p2);
    h.step();
    h.key_press_modifiers(Modifiers::SHIFT, Key::A);
    passos(&mut h);
    h.event(egui::Event::Text("lamp".into()));
    passos(&mut h);
    tela(&mut h, "e2e-3d-05-busca");
    h.key_press(Key::Enter);
    passos(&mut h);
    // As três tochas selecionadas: o Inspetor de vários.
    let tochas: Vec<u32> = cena(&mut h)
        .data
        .objects
        .iter()
        .filter(|o| o.name.starts_with("Tocha"))
        .map(|o| o.id)
        .collect();
    cena(&mut h).selection = tochas.clone();
    passos(&mut h);
    tela(&mut h, "e2e-3d-06-varios");
    // Guardar uma nos favoritos, pelo botão direito na Hierarquia.
    cena(&mut h).selection = vec![tochas[0]];
    cena(&mut h).reveal(tochas[0]);
    passos(&mut h);
    h.get_all_by_label("Tocha")
        .next()
        .unwrap()
        .click_secondary();
    passos(&mut h);
    tela(&mut h, "e2e-3d-07-menu-objeto");
    h.get_by_label("⭐ Guardar nos favoritos").click();
    passos(&mut h);
    tela(&mut h, "e2e-3d-08-favorito");
    // Shift+A sem digitar: o favorito e os recentes primeiro.
    h.hover_at(p2);
    h.step();
    h.key_press_modifiers(Modifiers::SHIFT, Key::A);
    passos(&mut h);
    tela(&mut h, "e2e-3d-09-busca-vazia");
    h.key_press(Key::Escape);
    passos(&mut h);
    // Apagar: o aviso com Desfazer.
    cena(&mut h).selection = tochas.clone();
    h.hover_at(p);
    h.step();
    h.key_press(Key::Delete);
    passos(&mut h);
    tela(&mut h, "e2e-3d-10-apagou");

    // Tema claro e cor roxa: os avisos, o menu e a busca.
    h.state_mut().settings.theme = nn_studio::settings::ThemeChoice::Claro;
    h.state_mut().settings.accent = nn_studio::settings::Accent::Roxo;
    passos(&mut h);
    direito(&mut h, p);
    tela(&mut h, "e2e-3d-11-claro-menu");
    h.key_press(Key::Escape);
    passos(&mut h);
    h.hover_at(p2);
    h.step();
    h.key_press_modifiers(Modifiers::SHIFT, Key::A);
    passos(&mut h);
    tela(&mut h, "e2e-3d-12-claro-busca");
    h.key_press(Key::Escape);
    passos(&mut h);
    h.state_mut()
        .project_mut()
        .unwrap()
        .show_tab(nn_studio::project::Tab::Home);
    passos(&mut h);
    tela(&mut h, "e2e-3d-13-claro-inicio");
}

/// Quanto custa um quadro do NN Studio com a ilha (cena 3D do modelo, uns
/// 220 objetos) aberta, com o mouse passando por cima da vista.
#[test]
#[ignore]
fn ilha_quadros_por_segundo() {
    let dir = temp("ilha-tempo");
    let proj = dir.join("ilha");
    create_project(&NewOptions {
        dir: proj.clone(),
        title: None,
        template: Template::Visual3D,
    })
    .unwrap();
    let mut h = studio(&dir, Some(proj.clone()), egui::vec2(1600.0, 900.0));
    h.state_mut()
        .project_mut()
        .unwrap()
        .open_file("cenas/ilha.cena", None);
    passos(&mut h);
    passos(&mut h);
    let rect = h.get_by_label("Vista da cena 3D").rect();
    let n = 40;
    let start = std::time::Instant::now();
    for i in 0..n {
        let t = i as f32 / n as f32;
        h.hover_at(rect.min + egui::vec2(rect.width() * t, rect.height() * 0.6));
        h.step();
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / n as f64;
    eprintln!("ilha: {ms:.1} ms por quadro");
}

/// As imagens da documentação da v0.5: o botão direito, a busca rápida, o
/// Inspetor com vários objetos e as Preferências (na ilha do modelo 3D).
#[test]
#[ignore]
fn capturas_da_v05_para_a_documentacao() {
    use egui::{Key, Modifiers};
    let dir = temp("capturas-v05");
    let proj = dir.join("jogo");
    create_project(&NewOptions {
        dir: proj.clone(),
        title: Some("Meu jogo".into()),
        template: Template::Visual3D,
    })
    .unwrap();
    let mut h = studio(&dir, Some(proj.clone()), egui::vec2(1280.0, 800.0));
    h.state_mut()
        .project_mut()
        .unwrap()
        .open_file("cenas/ilha.cena", None);
    passos(&mut h);
    passos(&mut h);
    fn cena<'a>(h: &'a mut Harness<'_, StudioApp>) -> &'a mut nn_studio::scene_view::SceneDoc {
        h.state_mut()
            .project_mut()
            .unwrap()
            .scenes
            .get_mut("cenas/ilha.cena")
            .unwrap()
    }
    let rect = h.get_by_label("Vista da cena 3D").rect();
    let p = rect.min + egui::vec2(rect.width() * 0.42, rect.height() * 0.62);
    // O botão direito no chão da ilha.
    h.hover_at(p);
    h.step();
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        h.step();
    }
    passos(&mut h);
    tela(&mut h, "nn-studio-botao-direito");
    h.key_press(Key::Escape);
    passos(&mut h);
    // A busca rápida.
    h.hover_at(p);
    h.step();
    h.key_press_modifiers(Modifiers::SHIFT, Key::A);
    passos(&mut h);
    h.event(egui::Event::Text("luz".into()));
    passos(&mut h);
    tela(&mut h, "nn-studio-busca-rapida");
    h.key_press(Key::Escape);
    passos(&mut h);
    // Três árvores selecionadas: o Inspetor de vários.
    let arvores: Vec<u32> = {
        let mut v = Vec::new();
        nn_studio::scene_data::visit(&cena(&mut h).data.objects, &mut |o| {
            if o.component("model")
                .is_some_and(|m| m.str("file").ends_with("arvore.glb"))
            {
                v.push(o.id);
            }
        });
        v.into_iter().take(3).collect()
    };
    cena(&mut h).selection = arvores;
    passos(&mut h);
    tela(&mut h, "nn-studio-varios");
    // As Preferências.
    cena(&mut h).selection.clear();
    h.state_mut().preferences = true;
    passos(&mut h);
    tela(&mut h, "nn-studio-preferencias");
}

/// A imagem do modelador na documentação: uma casinha com porta, janela,
/// telhado de duas águas e chaminé, com a porta escolhida.
#[test]
#[ignore]
fn captura_do_modelador_para_a_documentacao() {
    use glam::Vec3;
    use nn_studio::model_data::{Face, ModelData, Part, face_normal};
    let dir = temp("captura-modelador");
    let proj = dir.join("jogo");
    create_project(&NewOptions {
        dir: proj.clone(),
        title: Some("Meu jogo".into()),
        template: Template::Vazio3D,
    })
    .unwrap();
    // A casa: paredes, porta (encolher + puxar para dentro) e janela.
    let mut paredes = Part::cube("Paredes", "#F2C9A0");
    paredes.scale = Vec3::new(2.0, 1.4, 1.6);
    let face = |p: &Part, n: Vec3| {
        p.faces
            .iter()
            .position(|f| face_normal(&p.verts, &f.v).dot(n) > 0.9)
            .unwrap()
    };
    let frente = face(&paredes, Vec3::Z);
    paredes.inset(&[frente], 0.35);
    // A porta embaixo: o pedaço de dentro desce até o chão.
    let porta: Vec<u32> = paredes.faces[frente].v.clone();
    for &k in &porta {
        let v = &mut paredes.verts[k as usize];
        if v.y < 0.5 {
            v.y = 0.0;
        } else {
            v.y = 0.62;
        }
        v.x *= 0.6;
    }
    let (moved, _) = paredes.extrude(&[frente]);
    for k in moved {
        paredes.verts[k as usize].z -= 0.08;
    }
    paredes.faces[frente].color = "#6B4423".into();
    let lado = face(&paredes, Vec3::X);
    paredes.inset(&[lado], 0.3);
    let (moved, _) = paredes.extrude(&[lado]);
    for k in moved {
        paredes.verts[k as usize].x -= 0.05;
    }
    paredes.faces[lado].color = "#3FA7D6".into();
    // O telhado de duas águas.
    let telhado = Part {
        name: "Telhado".into(),
        verts: vec![
            Vec3::new(-1.15, 0.0, -0.95),
            Vec3::new(1.15, 0.0, -0.95),
            Vec3::new(1.15, 0.0, 0.95),
            Vec3::new(-1.15, 0.0, 0.95),
            Vec3::new(-1.15, 0.8, 0.0),
            Vec3::new(1.15, 0.8, 0.0),
        ],
        faces: vec![
            Face {
                v: vec![0, 1, 2, 3],
                color: "#9C6B43".into(),
            },
            Face {
                v: vec![3, 2, 5, 4],
                color: "#E84A4A".into(),
            },
            Face {
                v: vec![1, 0, 4, 5],
                color: "#E84A4A".into(),
            },
            Face {
                v: vec![2, 1, 5],
                color: "#F2C9A0".into(),
            },
            Face {
                v: vec![0, 3, 4],
                color: "#F2C9A0".into(),
            },
        ],
        pos: Vec3::new(0.0, 1.4, 0.0),
        ..Part::cube("Telhado", "#E84A4A")
    };
    let mut chamine = Part::cube("Chaminé", "#B8B8B8");
    chamine.scale = Vec3::new(0.3, 0.7, 0.3);
    chamine.pos = Vec3::new(0.55, 1.6, -0.35);
    let modelo = ModelData {
        parts: vec![paredes, telhado, chamine],
        ..ModelData::new_default()
    };
    let pasta = proj.join("assets/modelos");
    std::fs::create_dir_all(&pasta).unwrap();
    std::fs::write(pasta.join("casinha.modelo"), modelo.to_lua()).unwrap();
    std::fs::write(pasta.join("casinha.glb"), modelo.to_glb()).unwrap();

    let mut h = studio(&dir, Some(proj.clone()), egui::vec2(1280.0, 800.0));
    h.state_mut()
        .project_mut()
        .unwrap()
        .open_file("assets/modelos/casinha.modelo", None);
    passos(&mut h);
    {
        let ws = h.state_mut().project_mut().unwrap();
        let d = ws.models.values_mut().next().unwrap();
        d.sel_faces = vec![(0, frente)];
        d.cam
            .look_at(Vec3::new(2.3, 1.9, 3.1), Vec3::new(0.0, 0.95, 0.0));
    }
    passos(&mut h);
    passos(&mut h);
    tela(&mut h, "nn-studio-modelador");
}
