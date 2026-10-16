//! Testes da interface do NN Studio, clicando e digitando como uma pessoa
//! (egui_kittest), sem abrir janela.
//!
//! Com `NN_STUDIO_TELAS=<pasta>`, cada teste também grava imagens das telas
//! (precisa de GPU ou de um driver por software).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{self, Key, Modifiers};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use nn_studio::app::{StudioApp, StudioOptions};
use nn_studio::project::Tab;

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("nn-studio-ui-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Cópia do jogo de plataforma, para os testes mexerem à vontade.
fn plataforma(dir: &Path) -> PathBuf {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jogos/plataforma");
    let dst = dir.join("plataforma");
    copy_dir(&src, &dst);
    dst
}

fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for e in std::fs::read_dir(src).unwrap() {
        let e = e.unwrap();
        let to = dst.join(e.file_name());
        if e.path().is_dir() {
            copy_dir(&e.path(), &to);
        } else {
            std::fs::copy(e.path(), to).unwrap();
        }
    }
}

/// Um "jogo" de mentira: escreve como o jogo de verdade escreve e sai.
fn fake_game(dir: &Path) -> PathBuf {
    #[cfg(unix)]
    {
        let p = dir.join("jogo-falso.sh");
        std::fs::write(
            &p,
            "#!/bin/sh\n\
             echo \"recebi: $1 $2\"\n\
             printf '%s\\n' '@nn {\"n\":\"Info\",\"a\":\"nn_script::api\",\"m\":\"[lua] olá do jogo\"}'\n\
             printf '%s\\n' '@nn {\"n\":\"Error\",\"a\":\"nn_script\",\"m\":\"erro em main.lua:\\nmain.lua:2: tentou chamar\"}'\n\
             exit 0\n",
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        p
    }
    #[cfg(not(unix))]
    {
        dir.join("nao-existe.exe")
    }
}

fn studio(dir: &Path, open: Option<PathBuf>) -> Harness<'static, StudioApp> {
    studio_with(dir, open, fake_game(dir), false)
}

/// NN Studio com outro "jogo" e, se `gpu`, com a GPU de teste (as cenas
/// visuais são desenhadas pelo renderer da engine).
fn studio_with(
    dir: &Path,
    open: Option<PathBuf>,
    exe: PathBuf,
    gpu: bool,
) -> Harness<'static, StudioApp> {
    let options = StudioOptions {
        settings_path: Some(dir.join("preferencias.json")),
        open,
        exe: Some(exe),
    };
    let mut builder = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_max_steps(200);
    if gpu {
        builder = builder.wgpu();
    }
    let mut h = builder.build_eframe(move |cc| {
        let mut app = StudioApp::new(&cc.egui_ctx, options);
        app.set_render_state(cc.wgpu_render_state.clone());
        app
    });
    h.run();
    h
}

/// Grava a tela, se pedido (para conferir a aparência).
fn tela(h: &mut Harness<'_, StudioApp>, name: &str) {
    if let Some(dir) = std::env::var_os("NN_STUDIO_TELAS") {
        let dir = PathBuf::from(dir);
        let _ = std::fs::create_dir_all(&dir);
        match h.render() {
            Ok(img) => img.save(dir.join(format!("{name}.png"))).unwrap(),
            Err(e) => eprintln!("sem imagem da tela {name}: {e}"),
        }
    }
}

fn tabs(h: &Harness<'_, StudioApp>) -> Vec<Tab> {
    h.state()
        .project()
        .map(|p| p.dock.iter_all_tabs().map(|(_, t)| t.clone()).collect())
        .unwrap_or_default()
}

fn console_text(h: &Harness<'_, StudioApp>) -> String {
    h.state()
        .project()
        .map(|p| {
            p.console
                .entries()
                .map(|e| e.text.clone())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

/// Atalho de teclado.
fn atalho(h: &mut Harness<'_, StudioApp>, modifiers: Modifiers, key: Key) {
    h.key_press_modifiers(modifiers, key);
    h.run();
}

/// Campo de texto com este conteúdo.
fn campo<'a>(h: &'a Harness<'_, StudioApp>, value: &str) -> egui_kittest::Node<'a> {
    let value = value.to_string();
    h.get_by(move |n| n.role() == Role::TextInput && n.value().as_deref() == Some(value.as_str()))
}

/// Digita no campo com o foco.
fn digitar(h: &mut Harness<'_, StudioApp>, texto: &str) {
    h.event(egui::Event::Text(texto.into()));
    h.run();
}

/// Seleciona todo o texto do campo com o foco.
fn selecionar_tudo(h: &mut Harness<'_, StudioApp>) {
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.run();
}

/// Abre a paleta, digita e escolhe a primeira opção.
fn paleta(h: &mut Harness<'_, StudioApp>, texto: &str) {
    atalho(h, Modifiers::COMMAND | Modifiers::SHIFT, Key::P);
    digitar(h, texto);
    h.key_press(Key::Enter);
    h.run();
}

#[test]
fn gerenciador_cria_projeto_novo() {
    let dir = temp("novo");
    let mut h = studio(&dir, None);
    tela(&mut h, "gerenciador");
    h.get_by_label("Projetos recentes");
    h.get_by_label("✨ Novo projeto").click();
    h.run();
    tela(&mut h, "novo-projeto");

    // Nome do jogo e pasta.
    let inputs = h.get_all_by_role(Role::TextInput).count();
    assert_eq!(inputs, 2);
    // O nome já vem com o foco.
    selecionar_tudo(&mut h);
    digitar(&mut h, "Corrida Maluca");
    h.get_all_by_role(Role::TextInput).nth(1).unwrap().focus();
    selecionar_tudo(&mut h);
    digitar(&mut h, &dir.join("jogos").to_string_lossy());
    h.get_by_label_contains("corrida-maluca será criada");
    h.get_by_label("Criar projeto").click();
    h.run();

    let p = h.state().project().expect("o projeto abriu");
    assert_eq!(p.title, "Corrida Maluca");
    assert!(dir.join("jogos/corrida-maluca/config.lua").exists());
    assert!(p.files.exists("cenas/jogo.lua"));
    // Entrou nos recentes, gravados nas preferências.
    let prefs = std::fs::read_to_string(dir.join("preferencias.json")).unwrap();
    assert!(prefs.contains("corrida-maluca"), "{prefs}");
    tela(&mut h, "projeto-novo");
}

#[test]
fn pasta_que_nao_e_projeto_mostra_erro() {
    let dir = temp("invalido");
    let h = studio(&dir, Some(dir.join("nada")));
    assert!(h.state().project().is_none());
    assert!(h.state().launcher_error().unwrap().contains("não existe"));
    h.get_by_label_contains("não existe");
}

#[test]
fn abre_arquivo_edita_e_salva() {
    let dir = temp("edita");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    tela(&mut h, "projeto");

    // Ctrl+P: abrir arquivo pelo nome.
    atalho(&mut h, Modifiers::COMMAND, Key::P);
    digitar(&mut h, "titulo");
    h.key_press(Key::Enter);
    h.run();
    assert!(tabs(&h).contains(&Tab::Code("cenas/titulo.lua".into())));
    tela(&mut h, "codigo");

    // Digita no editor.
    let editor = h.get_by_role(Role::MultilineTextInput);
    editor.focus();
    editor.type_text("-- mudança feita no teste\n");
    h.run();
    let doc = &h.state().project().unwrap().docs["cenas/titulo.lua"];
    assert!(doc.is_dirty());
    assert!(
        doc.text.contains("-- mudança feita no teste"),
        "{}",
        doc.text
    );
    // A barra de status avisa.
    h.get_by_label_contains("1 arquivo não salvo");

    // Ctrl+S grava no disco.
    atalho(&mut h, Modifiers::COMMAND, Key::S);
    let disk = std::fs::read_to_string(proj.join("cenas/titulo.lua")).unwrap();
    assert!(disk.contains("-- mudança feita no teste"));
    assert!(!h.state().project().unwrap().docs["cenas/titulo.lua"].is_dirty());
    assert!(console_text(&h).contains("Salvo: cenas/titulo.lua"));
}

#[test]
fn erro_de_sintaxe_aparece_e_leva_a_linha() {
    let dir = temp("sintaxe");
    let proj = plataforma(&dir);
    std::fs::write(
        proj.join("cenas/quebrada.lua"),
        "local c = {}\n\nfunction c:draw(\n\nreturn c\n",
    )
    .unwrap();
    let mut h = studio(&dir, Some(proj));
    h.state_mut()
        .project_mut()
        .unwrap()
        .open_file("cenas/quebrada.lua", None);
    h.run();
    let issue = h.state().project().unwrap().docs["cenas/quebrada.lua"]
        .issue
        .clone()
        .expect("erro de sintaxe");
    assert!(issue.line >= 3, "{issue:?}");
    // A mensagem aparece em português na barra do editor; clicar vai até a linha.
    h.get_by_label_contains(&format!("Linha {}:", issue.line))
        .click();
    h.run();
    assert_eq!(
        h.state().project().unwrap().docs["cenas/quebrada.lua"]
            .cursor
            .0,
        issue.line
    );
    tela(&mut h, "erro-de-sintaxe");
}

#[test]
fn erro_do_jogo_no_console_abre_o_arquivo_na_linha() {
    let dir = temp("console");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj));
    let line = nn_studio::console::format_record(
        log::Level::Error,
        "nn_script",
        "erro em cenas/fase.lua:\ncenas/fase.lua:12: tentou chamar um valor nil (vazio) (método 'pular')\ncaminho até o erro (stack traceback):\n\tcenas/fase.lua:12: na função 'cena:update'",
    );
    h.state_mut()
        .project_mut()
        .unwrap()
        .console
        .push_game_line(&line);
    h.run();
    tela(&mut h, "console-com-erro");
    h.get_by_label("cenas/fase.lua:12").click();
    h.run();
    assert!(tabs(&h).contains(&Tab::Code("cenas/fase.lua".into())));
    h.run();
    assert_eq!(
        h.state().project().unwrap().docs["cenas/fase.lua"].cursor.0,
        12
    );
}

#[derive(Debug)]
struct Solto(PathBuf);

impl egui::DroppedFile for Solto {
    fn path(&self) -> &Path {
        &self.0
    }

    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

#[test]
fn arrastar_imagem_e_som_importa_para_as_pastas_certas() {
    let dir = temp("arrastar");
    let proj = plataforma(&dir);
    let fora = dir.join("fora");
    std::fs::create_dir_all(&fora).unwrap();
    let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 0, 0, 255]));
    img.save(fora.join("bau.png")).unwrap();
    std::fs::copy(proj.join("assets/sons/pulo.wav"), fora.join("abrir.wav")).unwrap();

    let mut h = studio(&dir, Some(proj.clone()));
    h.input_mut()
        .dropped_files
        .push(Arc::new(Solto(fora.join("bau.png"))));
    h.input_mut()
        .dropped_files
        .push(Arc::new(Solto(fora.join("abrir.wav"))));
    h.run();
    assert!(proj.join("assets/imagens/bau.png").exists());
    assert!(proj.join("assets/sons/abrir.wav").exists());
    let text = console_text(&h);
    assert!(text.contains("Importado: assets/imagens/bau.png"), "{text}");
    // O arquivo novo aparece no painel.
    assert!(
        h.state()
            .project()
            .unwrap()
            .files
            .exists("assets/imagens/bau.png")
    );
    h.get_by_label("bau.png");
}

#[test]
fn nova_cena_pelo_dialogo() {
    let dir = temp("cena");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    paleta(&mut h, "cena em codigo");
    h.get_by_label("Nova cena em código");
    selecionar_tudo(&mut h);
    digitar(&mut h, "chefe");
    h.get_by_label("Será criado: cenas/chefe.lua");
    tela(&mut h, "nova-cena");
    h.get_by_label("Criar").click();
    h.run();
    assert!(proj.join("cenas/chefe.lua").exists());
    assert!(tabs(&h).contains(&Tab::Code("cenas/chefe.lua".into())));
    let doc = &h.state().project().unwrap().docs["cenas/chefe.lua"];
    assert_eq!(doc.issue, None, "a cena nova não tem erros");

    // Nome repetido: erro no diálogo, nada é sobrescrito.
    paleta(&mut h, "cena em codigo");
    selecionar_tudo(&mut h);
    digitar(&mut h, "chefe");
    h.get_by_label("Criar").click();
    h.run();
    h.get_by_label_contains("Já existe cenas/chefe.lua");
}

#[test]
fn renomear_pelo_painel() {
    let dir = temp("renomear");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    // Abre o arquivo (o painel rola até ele) e volta para o Início: assim só
    // o painel mostra o nome.
    h.state_mut()
        .project_mut()
        .unwrap()
        .open_file("lib/inimigos.lua", None);
    h.run();
    h.state_mut().project_mut().unwrap().show_tab(Tab::Home);
    h.run();
    h.get_by_label("inimigos.lua").click_secondary();
    h.run();
    h.get_by_label_contains("Renomear").click();
    h.run();
    // O nome aparece para editar, com "inimigos" selecionado.
    assert!(campo(&h, "inimigos.lua").is_focused());
    digitar(&mut h, "monstros");
    h.key_press(Key::Enter);
    h.run();
    assert!(proj.join("lib/monstros.lua").exists());
    assert!(!proj.join("lib/inimigos.lua").exists());
    // A aba aberta acompanha o nome novo.
    assert!(tabs(&h).contains(&Tab::Code("lib/monstros.lua".into())));
    assert!(
        h.state()
            .project()
            .unwrap()
            .docs
            .contains_key("lib/monstros.lua")
    );
}

#[test]
fn fechar_com_alteracoes_pergunta_e_salva() {
    let dir = temp("fechar");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    h.state_mut()
        .project_mut()
        .unwrap()
        .open_file("lib/util.lua", None);
    h.run();
    h.state_mut()
        .project_mut()
        .unwrap()
        .docs
        .get_mut("lib/util.lua")
        .unwrap()
        .text
        .push_str("-- antes de fechar\n");
    h.run();
    // Pede para fechar a janela.
    h.input_mut()
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .events
        .push(egui::ViewportEvent::Close);
    h.run();
    h.get_by_label("Salvar as alterações?");
    h.get_by_label_contains("lib/util.lua");
    tela(&mut h, "salvar-antes-de-fechar");
    h.get_by_label("Salvar tudo").click();
    h.run();
    let disk = std::fs::read_to_string(proj.join("lib/util.lua")).unwrap();
    assert!(disk.contains("-- antes de fechar"));
}

#[test]
fn configuracoes_gravam_o_config_lua_sem_perder_comentarios() {
    let dir = temp("config");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    paleta(&mut h, "configuracoes do projeto");
    assert!(tabs(&h).contains(&Tab::Settings));
    tela(&mut h, "configuracoes");
    campo(&h, "Plataforma — nn-engine").focus();
    selecionar_tudo(&mut h);
    digitar(&mut h, "Plataforma 2");
    h.get_by_label_contains("Salvar configurações").click();
    h.run();
    let config = std::fs::read_to_string(proj.join("config.lua")).unwrap();
    assert!(config.contains("title = \"Plataforma 2\""), "{config}");
    assert!(config.contains("-- Identifica o jogo"), "{config}");
    // O título novo vale para o projeto (ao perceber a mudança no disco).
    std::thread::sleep(nn_studio::files::POLL_INTERVAL);
    h.run();
    assert_eq!(h.state().project().unwrap().title, "Plataforma 2");
}

#[test]
fn paleta_troca_tema_e_modo_e_grava_as_preferencias() {
    let dir = temp("tema");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj));
    paleta(&mut h, "tema");
    assert_eq!(
        h.state().settings.theme,
        nn_studio::settings::ThemeChoice::Claro
    );
    paleta(&mut h, "modo");
    assert_eq!(h.state().settings.mode, nn_studio::settings::Mode::Completo);
    tela(&mut h, "tema-claro-completo");
    let prefs = std::fs::read_to_string(dir.join("preferencias.json")).unwrap();
    assert!(
        prefs.contains("\"Claro\"") && prefs.contains("\"Completo\""),
        "{prefs}"
    );
    // No modo Completo o console tem o filtro da engine.
    h.get_by_label_contains("Engine");
}

#[test]
fn layout_e_abas_voltam_ao_reabrir() {
    let dir = temp("layout");
    let proj = plataforma(&dir);
    {
        let mut h = studio(&dir, Some(proj.clone()));
        h.state_mut()
            .project_mut()
            .unwrap()
            .open_file("cenas/fase.lua", None);
        h.run();
        h.state_mut().persist();
    }
    let h = studio(&dir, Some(proj));
    assert!(tabs(&h).contains(&Tab::Code("cenas/fase.lua".into())));
}

#[cfg(unix)]
#[test]
fn jogar_roda_o_jogo_e_traz_a_saida_para_o_console() {
    let dir = temp("jogar");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    atalho(&mut h, Modifiers::NONE, Key::F5);
    // Espera o "jogo" escrever e sair.
    let start = std::time::Instant::now();
    while h.state().project().unwrap().is_playing() {
        assert!(start.elapsed().as_secs() < 20, "o jogo não terminou");
        std::thread::sleep(std::time::Duration::from_millis(50));
        h.step();
    }
    h.run();
    let text = console_text(&h);
    assert!(text.contains("olá do jogo"), "{text}");
    assert!(text.contains("main.lua:2: tentou chamar"), "{text}");
    let root = proj.canonicalize().unwrap();
    assert!(
        text.contains(&format!("recebi: --jogar {}", root.display())),
        "{text}"
    );
    tela(&mut h, "jogo-terminou");
    // Erro do jogo: o console mostra o arquivo e a linha como link.
    h.get_by_label("main.lua:2");
}

#[test]
fn arrastar_pasta_para_o_gerenciador_abre_o_projeto() {
    let dir = temp("pasta");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, None);
    assert!(h.state().project().is_none());
    h.input_mut().dropped_files.push(Arc::new(Solto(proj)));
    h.run();
    assert_eq!(h.state().project().unwrap().title, "Plataforma — nn-engine");
}

#[test]
fn buscar_e_ir_para_a_linha_pelo_teclado() {
    let dir = temp("buscar");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj));
    h.state_mut()
        .project_mut()
        .unwrap()
        .open_file("cenas/titulo.lua", None);
    h.run();
    h.get_by_role(Role::MultilineTextInput).focus();
    h.run();
    // Ctrl+G e já digita: nada cai no código.
    h.key_press_modifiers(Modifiers::COMMAND, Key::G);
    h.event(egui::Event::Text("16".into()));
    h.key_press(Key::Enter);
    h.run();
    let doc = &h.state().project().unwrap().docs["cenas/titulo.lua"];
    assert!(!doc.is_dirty(), "o texto não pode mudar");
    assert_eq!(doc.cursor.0, 16);

    // Ctrl+F: acha e conta as ocorrências.
    h.key_press_modifiers(Modifiers::COMMAND, Key::F);
    h.event(egui::Event::Text("nn.camera".into()));
    h.run();
    h.get_by_label("1 de 3");
    h.key_press(Key::Enter);
    h.run();
    h.get_by_label("2 de 3");
    assert!(!h.state().project().unwrap().docs["cenas/titulo.lua"].is_dirty());
    tela(&mut h, "buscar");
}

#[test]
fn exemplos_abrem_uma_copia() {
    let dir = temp("exemplos");
    // Os projetos novos (e as cópias dos exemplos) vão para esta pasta.
    let projetos = dir.join("projetos");
    std::fs::write(
        dir.join("preferencias.json"),
        format!("{{\"projects_dir\": {:?}}}", projetos.to_string_lossy()),
    )
    .unwrap();
    let mut h = studio(&dir, None);
    h.get_by_label("🎮 Exemplos").click();
    h.run();
    tela(&mut h, "exemplos");
    h.get_by_label("Abrir uma cópia de Plataforma").click();
    h.run();
    let p = h.state().project().expect("o exemplo abriu");
    assert!(p.root.ends_with("plataforma"));
    assert!(projetos.join("plataforma/fases/fase1.tmj").exists());
    // O original não é tocado: abrir de novo cria outra cópia.
    h.state_mut().close_project();
    h.run();
    h.get_by_label("🎮 Exemplos").click();
    h.run();
    h.get_by_label("Abrir uma cópia de Plataforma").click();
    h.run();
    assert!(projetos.join("plataforma-2/config.lua").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn mandar_para_a_lixeira_pergunta_antes() {
    let dir = temp("lixeira");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    h.state_mut()
        .project_mut()
        .unwrap()
        .open_file("lib/inimigos.lua", None);
    h.run();
    h.state_mut().project_mut().unwrap().show_tab(Tab::Home);
    h.run();
    h.get_by_label("inimigos.lua").click_secondary();
    h.run();
    h.get_by_label_contains("Mandar para a lixeira").click();
    h.run();
    h.get_by_label("Mandar para a lixeira?");
    h.get_by_label("Cancelar").click();
    h.run();
    assert!(proj.join("lib/inimigos.lua").exists(), "cancelar não apaga");
    h.get_by_label("inimigos.lua").click_secondary();
    h.run();
    h.get_by_label_contains("Mandar para a lixeira").click();
    h.run();
    h.get_by_label("🗑 Mandar para a lixeira").click();
    h.run();
    let text = console_text(&h);
    if text.contains("Mandado para a lixeira") {
        assert!(!proj.join("lib/inimigos.lua").exists());
        assert!(
            !tabs(&h).contains(&Tab::Code("lib/inimigos.lua".into())),
            "a aba fecha"
        );
    } else {
        // Sem lixeira no sistema (alguns ambientes de teste): o erro aparece e nada some.
        assert!(text.contains("Não foi possível mandar"), "{text}");
        assert!(proj.join("lib/inimigos.lua").exists());
    }
}

// ---------------------------------------------------------------------------
// Cenas visuais

/// Projeto com uma cena visual nova (título e quadrado) aberta.
fn com_cena(h: &mut Harness<'_, StudioApp>, proj: &Path) {
    std::fs::write(
        proj.join("cenas/teste.cena"),
        nn_studio::files::visual_scene_template(),
    )
    .unwrap();
    let ws = h.state_mut().project_mut().unwrap();
    ws.files.rescan();
    ws.open_file("cenas/teste.cena", None);
    h.run();
}

fn cena<'a>(h: &'a Harness<'_, StudioApp>) -> &'a nn_studio::scene_view::SceneDoc {
    &h.state().project().unwrap().scenes["cenas/teste.cena"]
}

fn objeto<'a>(h: &'a Harness<'_, StudioApp>, name: &str) -> &'a nn_studio::scene_data::Object {
    cena(h)
        .data
        .objects
        .iter()
        .find(|o| o.name == name)
        .unwrap_or_else(|| panic!("sem o objeto {name}"))
}

/// Arrasta com o mouse de um ponto a outro.
fn arrastar(h: &mut Harness<'_, StudioApp>, from: egui::Pos2, to: egui::Pos2) {
    h.hover_at(from);
    h.run();
    h.drag_at(from);
    h.run();
    for i in 1..=6 {
        let t = i as f32 / 6.0;
        h.hover_at(from + (to - from) * t);
        h.run();
    }
    h.drop_at(to);
    h.run();
}

#[test]
fn nova_cena_visual_pelo_dialogo() {
    let dir = temp("cena-visual");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    atalho(&mut h, Modifiers::COMMAND, Key::N);
    h.get_by_label("Nova cena visual");
    selecionar_tudo(&mut h);
    digitar(&mut h, "fase3");
    h.get_by_label("Será criado: cenas/fase3.cena");
    h.get_by_label("Criar").click();
    h.run();
    assert!(proj.join("cenas/fase3.cena").exists());
    let t = tabs(&h);
    assert!(t.contains(&Tab::Scene("cenas/fase3.cena".into())), "{t:?}");
    assert!(t.contains(&Tab::Hierarchy) && t.contains(&Tab::Inspector));
    // A Hierarquia mostra os objetos do modelo; o Inspetor, a cena.
    h.get_by_label("titulo");
    h.get_by_label("quadrado");
    h.get_by_label("Vista da cena");
    h.get_by_label_contains("2 objetos");
    tela(&mut h, "cena-visual-nova");
    // A cena nova roda na engine (sem erros de sintaxe).
    let text = std::fs::read_to_string(proj.join("cenas/fase3.cena")).unwrap();
    assert!(text.contains("format = 1"), "{text}");
}

#[test]
fn cena_visual_arrastar_imagem_mover_desfazer_e_salvar() {
    let dir = temp("cena-editar");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    com_cena(&mut h, &proj);
    h.state_mut()
        .project_mut()
        .unwrap()
        .file_panel
        .reveal("assets/imagens/moeda.png");
    h.run();

    // Arrasta a imagem do painel Arquivos para a cena: vira um sprite.
    let from = h.get_by_label("moeda.png").rect().center();
    let to = h.get_by_label("Vista da cena").rect().center() + egui::vec2(120.0, 0.0);
    arrastar(&mut h, from, to);
    let moeda = objeto(&h, "moeda").clone();
    let sprite = moeda.component("sprite").expect("sprite");
    assert_eq!(sprite.str("image"), "assets/imagens/moeda.png");
    assert_eq!(
        cena(&h).selection,
        vec![moeda.id],
        "o objeto novo fica selecionado"
    );
    assert!(cena(&h).is_dirty());
    h.get_by_label("Imagem (sprite)");
    tela(&mut h, "cena-imagem-solta");

    // Setas movem o selecionado (com o mouse em cima da cena).
    let centro = h.get_by_label("Vista da cena").rect().center();
    h.hover_at(centro);
    h.run();
    for _ in 0..3 {
        h.key_press(Key::ArrowRight);
        h.run();
    }
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowUp);
    h.run();
    let movida = objeto(&h, "moeda").clone();
    assert_eq!(movida.x, moeda.x + 3.0);
    assert_eq!(movida.y, moeda.y + 10.0);

    // Ctrl+Z desfaz as setas de uma vez; outro Ctrl+Z tira a imagem.
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    assert_eq!(objeto(&h, "moeda").x, moeda.x);
    assert_eq!(objeto(&h, "moeda").y, moeda.y);
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    assert!(cena(&h).data.objects.iter().all(|o| o.name != "moeda"));
    assert!(!cena(&h).is_dirty(), "voltou ao que está salvo");
    atalho(&mut h, Modifiers::COMMAND, Key::Y);
    assert_eq!(objeto(&h, "moeda").x, moeda.x);

    // Ctrl+S grava o .cena.
    atalho(&mut h, Modifiers::COMMAND, Key::S);
    assert!(!cena(&h).is_dirty());
    let text = std::fs::read_to_string(proj.join("cenas/teste.cena")).unwrap();
    assert!(
        text.contains("image = \"assets/imagens/moeda.png\""),
        "{text}"
    );
    let lida = nn_studio::scene_data::read(&text, "teste").unwrap();
    assert_eq!(
        lida,
        cena(&h).data,
        "o que foi gravado é o que está no editor"
    );
}

#[test]
fn cena_visual_hierarquia_e_inspetor() {
    let dir = temp("cena-inspetor");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    com_cena(&mut h, &proj);

    // Clicar na Hierarquia seleciona; o Inspetor mostra o objeto.
    h.get_by_label("quadrado").click();
    h.run();
    let id = objeto(&h, "quadrado").id;
    assert_eq!(cena(&h).selection, vec![id]);
    h.get_by_label("Transformação");

    // Renomear pelo Inspetor.
    campo(&h, "quadrado").focus();
    h.run();
    selecionar_tudo(&mut h);
    digitar(&mut h, "jogador");
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(cena(&h).data.find(id).unwrap().name, "jogador");

    // Adicionar um corpo físico.
    h.get_by_label_contains("Adicionar componente").click();
    h.run();
    h.get_by_label("Corpo físico").click();
    h.run();
    assert!(cena(&h).data.find(id).unwrap().component("body").is_some());
    tela(&mut h, "cena-inspetor");

    // Arrastar "jogador" para dentro de "titulo" na Hierarquia vira filho.
    let from = h.get_by_label("jogador").rect().center();
    let to = h.get_by_label("titulo").rect().center();
    arrastar(&mut h, from, to);
    let titulo = objeto(&h, "titulo");
    assert_eq!(titulo.children.len(), 1, "{:?}", cena(&h).data.objects);
    assert_eq!(titulo.children[0].name, "jogador");

    // Desfazer volta tudo, um passo por vez.
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    assert!(objeto(&h, "titulo").children.is_empty());
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    assert!(objeto(&h, "jogador").component("body").is_none());
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    objeto(&h, "quadrado");
}

#[test]
fn cena_visual_seta_do_gizmo_e_selecao_por_retangulo() {
    let dir = temp("cena-gizmo");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    com_cena(&mut h, &proj);
    h.get_by_label("quadrado").click();
    h.run();
    let antes = objeto(&h, "quadrado").clone();
    let vista = h.get_by_label("Vista da cena").rect();
    let centro = cena(&h).camera.to_screen(vista, antes.x, antes.y);
    let zoom = cena(&h).camera.zoom;

    // Arrastar a seta vermelha (eixo X) move só em X, mesmo com o mouse
    // saindo um pouco da linha.
    let seta = centro + egui::vec2(40.0, 0.0);
    arrastar(&mut h, seta, seta + egui::vec2(100.0, 15.0));
    let depois = objeto(&h, "quadrado").clone();
    assert!(
        (depois.x - (antes.x + 100.0 / zoom)).abs() < 0.01,
        "x {} -> {}",
        antes.x,
        depois.x
    );
    assert_eq!(depois.y, antes.y);
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    assert_eq!(objeto(&h, "quadrado").x, antes.x, "um passo só");

    // Retângulo num espaço vazio seleciona o que fica todo dentro dele.
    let canto = vista.left_top() + egui::vec2(5.0, 5.0);
    arrastar(&mut h, canto, vista.right_bottom() - egui::vec2(5.0, 5.0));
    let mut sel = cena(&h).selection.clone();
    sel.sort();
    assert_eq!(sel, vec![1, 2], "título e quadrado");
}

/// "Jogo" que avisa a cena da frente e guarda o que recebe na entrada.
#[cfg(unix)]
fn jogo_ao_vivo(dir: &Path) -> PathBuf {
    let p = dir.join("jogo-ao-vivo.sh");
    let recebido = dir.join("recebido.txt");
    std::fs::write(
        &p,
        format!(
            "#!/bin/sh\n\
             echo \"args: $*\"\n\
             printf '%s\\n' '@nn {{\"n\":\"Info\",\"a\":\"nn_script::live\",\"m\":\"cena: cenas/teste.cena\"}}'\n\
             while read -r linha; do echo \"$linha\" >> '{}'; done\n",
            recebido.display()
        ),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

#[cfg(unix)]
#[test]
fn testar_daqui_abre_na_cena_e_manda_ajustes_ao_vivo() {
    let dir = temp("ao-vivo");
    let proj = plataforma(&dir);
    let mut h = studio_with(&dir, Some(proj.clone()), jogo_ao_vivo(&dir), false);
    com_cena(&mut h, &proj);
    h.get_by_label("quadrado").click();
    h.run();
    let id = objeto(&h, "quadrado").id;

    // F6: roda o jogo começando nesta cena.
    // (Com o jogo aberto o editor redesenha sempre: `run_steps` em vez de `run`.)
    h.key_press(Key::F6);
    h.run_steps(3);
    let start = std::time::Instant::now();
    while h.state().project().unwrap().game_scene.is_none() {
        assert!(start.elapsed().as_secs() < 20, "o jogo não avisou a cena");
        std::thread::sleep(std::time::Duration::from_millis(50));
        h.step();
    }
    h.run_steps(3);
    let text = console_text(&h);
    assert!(text.contains("args: --jogar"), "{text}");
    assert!(text.contains("--cena cenas/teste.cena"), "{text}");
    assert!(
        !text.contains("nn_script::live"),
        "o aviso da cena não suja o console"
    );
    h.get_by_label("• Ao vivo");

    // Mexer no objeto manda o ajuste para o jogo, sem salvar.
    let centro = h.get_by_label("Vista da cena").rect().center();
    h.hover_at(centro);
    h.run_steps(2);
    h.key_press(Key::ArrowRight);
    h.run_steps(2);
    assert!(cena(&h).is_dirty());
    let recebido = dir.join("recebido.txt");
    let start = std::time::Instant::now();
    let linhas = loop {
        let t = std::fs::read_to_string(&recebido).unwrap_or_default();
        if !t.is_empty() {
            break t;
        }
        assert!(
            start.elapsed().as_secs() < 10,
            "o jogo não recebeu o ajuste"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    let msg: serde_json::Value = serde_json::from_str(linhas.lines().next().unwrap()).unwrap();
    assert_eq!(msg["scene"], "cenas/teste.cena");
    assert_eq!(msg["id"], id);
    assert_eq!(msg["field"], "x");
    assert_eq!(msg["value"], 1.0);
    tela(&mut h, "cena-ao-vivo");
    h.key_press_modifiers(Modifiers::SHIFT, Key::F5);
    h.run_steps(3);
    assert!(!h.state().project().unwrap().is_playing());
}

/// Com GPU, a cena é desenhada pelo renderer da engine dentro do editor.
#[test]
fn cena_visual_desenhada_pela_engine() {
    if std::env::var_os("NN_REQUIRE_GPU").is_none() {
        eprintln!("sem NN_REQUIRE_GPU: pulando o teste com GPU");
        return;
    }
    let dir = temp("cena-gpu");
    let proj = plataforma(&dir);
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), true);
    com_cena(&mut h, &proj);
    let mut doc_rect = h.get_by_label("Vista da cena").rect();
    let img = h.render().expect("render");
    // O quadrado azul (#3FA7D6) aparece onde ele está na cena.
    let q = objeto(&h, "quadrado");
    let c = cena(&h).camera.to_screen(doc_rect, q.x, q.y);
    let mut found = false;
    for dy in -20..=20 {
        for dx in -20..=20 {
            let p = img.get_pixel((c.x as i32 + dx) as u32, (c.y as i32 + dy) as u32);
            if (p[0] as i32 - 0x3F).abs() < 12
                && (p[1] as i32 - 0xA7).abs() < 12
                && (p[2] as i32 - 0xD6).abs() < 12
            {
                found = true;
            }
        }
    }
    doc_rect = doc_rect.shrink(4.0);
    let fundo = img.get_pixel(doc_rect.left() as u32 + 4, doc_rect.bottom() as u32 - 4);
    if let Some(d) = std::env::var_os("NN_STUDIO_TELAS") {
        let _ = img.save(PathBuf::from(d).join("cena-gpu.png"));
    }
    assert!(found, "o quadrado não apareceu; fundo {fundo:?}");
}
