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
        check_updates: false,
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

#[test]
fn autocompletar_no_editor_de_codigo() {
    let dir = temp("autocompletar");
    let proj = plataforma(&dir);
    std::fs::create_dir_all(proj.join("scripts")).unwrap();
    std::fs::write(proj.join("scripts/fase.lua"), "").unwrap();
    std::fs::write(
        proj.join("cenas/teste.cena"),
        "return { script = 'scripts/fase.lua', objects = { { id = 1, name = 'heroi' } } }",
    )
    .unwrap();
    let mut h = studio(&dir, Some(proj.clone()));
    h.state_mut()
        .project_mut()
        .unwrap()
        .open_file("scripts/fase.lua", None);
    h.run();
    let editor = h.get_by_role(Role::MultilineTextInput);
    editor.focus();
    h.run();
    let texto = |h: &Harness<'_, StudioApp>| {
        h.state().project().unwrap().docs["scripts/fase.lua"]
            .text
            .clone()
    };

    // "nn.dr" sugere "draw"; Enter aceita.
    digitar(&mut h, "nn.dr");
    h.get_by_label("{}  draw");
    tela(&mut h, "autocompletar");
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(texto(&h), "nn.draw");
    // Depois do ponto, as funções; aceitar uma função abre o parêntese.
    digitar(&mut h, ".rec");
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(texto(&h), "nn.draw.rect(");
    // A dica mostra os parâmetros enquanto se digita os argumentos.
    h.get_by_label_contains("nn.draw.rect(x, y, w, h, cor)");
    digitar(&mut h, "0, 0, 8, 8)\n");

    // Objetos da cena que usa este script.
    digitar(&mut h, "self.objects.he");
    h.key_press(Key::Tab);
    h.run();
    assert!(texto(&h).ends_with("self.objects.heroi"), "{}", texto(&h));
    // Esc fecha sem mudar nada.
    digitar(&mut h, ".bo");
    h.key_press(Key::Escape);
    h.run();
    assert!(
        texto(&h).ends_with("self.objects.heroi.bo"),
        "{}",
        texto(&h)
    );
    assert!(
        h.state().project().unwrap().docs["scripts/fase.lua"]
            .suggest
            .is_none()
    );
}

/// "Jogo" que conta o estado da cena, pede para selecionar um objeto e
/// guarda o que recebe.
#[cfg(unix)]
fn jogo_que_conta(dir: &Path) -> PathBuf {
    let p = dir.join("jogo-conta.sh");
    let recebido = dir.join("comandos.txt");
    let estado = r#"estado: {\"scene\":\"cenas/teste.cena\",\"paused\":false,\"objects\":[{\"id\":2,\"x\":50,\"y\":0,\"rotation\":0,\"scale_x\":1,\"scale_y\":1,\"wx\":50,\"wy\":0,\"wr\":0,\"wsx\":1,\"wsy\":1,\"visible\":true,\"props\":{}}],\"vars\":{\"moedas\":7}}"#;
    std::fs::write(
        &p,
        format!(
            "#!/bin/sh\n\
             printf '%s\\n' '@nn {{\"n\":\"Info\",\"a\":\"nn_script::live\",\"m\":\"cena: cenas/teste.cena\"}}'\n\
             printf '%s\\n' '@nn {{\"n\":\"Info\",\"a\":\"nn_script::live\",\"m\":\"{estado}\"}}'\n\
             printf '%s\\n' '@nn {{\"n\":\"Info\",\"a\":\"nn_script::live\",\"m\":\"selecionar: 2\"}}'\n\
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
fn jogo_conta_onde_estao_os_objetos_e_o_editor_pausa() {
    let dir = temp("jogo-conta");
    let proj = plataforma(&dir);
    let mut h = studio_with(&dir, Some(proj.clone()), jogo_que_conta(&dir), false);
    com_cena(&mut h, &proj);
    h.key_press(Key::F6);
    h.run_steps(3);
    let start = std::time::Instant::now();
    while h.state().project().unwrap().game_state.is_none() || cena(&h).selection.is_empty() {
        assert!(start.elapsed().as_secs() < 20, "o jogo não contou o estado");
        std::thread::sleep(std::time::Duration::from_millis(50));
        h.step();
    }
    h.run_steps(3);
    // Ctrl+clique no jogo selecionou o quadrado aqui.
    assert_eq!(cena(&h).selection, vec![2]);
    // O Inspetor mostra onde ele está no jogo e deixa trazer para a cena.
    h.get_by_label("No jogo");
    tela(&mut h, "jogo-conta");
    h.get_by_label("Manter").click();
    h.run_steps(3);
    assert_eq!(objeto(&h, "quadrado").x, 50.0);
    // Pausar manda o comando para o jogo.
    h.get_by_label("Pausar").click();
    h.run_steps(3);
    let arquivo = dir.join("comandos.txt");
    let start = std::time::Instant::now();
    loop {
        let t = std::fs::read_to_string(&arquivo).unwrap_or_default();
        if t.contains(r#"{"cmd":"pause"}"#) {
            break;
        }
        assert!(start.elapsed().as_secs() < 10, "não chegou: {t}");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    h.key_press_modifiers(Modifiers::SHIFT, Key::F5);
    h.run_steps(3);
}

#[test]
fn comportamento_pronto_pelo_inspetor() {
    let dir = temp("comportamento");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    com_cena(&mut h, &proj);
    h.get_by_label("quadrado").click();
    h.run();
    // Adicionar componente > Jogador > Jogador de plataforma.
    h.get_by_label_contains("Adicionar componente").click();
    h.run();
    tela(&mut h, "adicionar-componente");
    h.get_by_label_contains("Jogador").click();
    h.run();
    h.get_by_label("Jogador de plataforma").click();
    h.run();
    let q = objeto(&h, "quadrado").clone();
    // Precisa de corpo físico: já veio junto.
    assert!(q.component("body").is_some());
    let b = q
        .components
        .iter()
        .find(|c| c.is_behavior())
        .expect("comportamento");
    assert_eq!(b.str("script"), "nn:plataforma");
    assert_eq!(b.num("velocidade"), 320.0);
    // As propriedades aparecem com os nomes do script.
    h.get_by_label("Força do pulo");
    tela(&mut h, "comportamento");
    // Salva e o arquivo tem uma linha só para o comportamento.
    atalho(&mut h, Modifiers::COMMAND, Key::S);
    let text = std::fs::read_to_string(proj.join("cenas/teste.cena")).unwrap();
    assert!(
        text.contains(
            "{ type = \"behavior\", script = \"nn:plataforma\", coiote = 0.1, pulo = 860,"
        ),
        "{text}"
    );
}

#[test]
fn prefab_criado_pela_hierarquia_e_usado_na_cena() {
    let dir = temp("prefab");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    com_cena(&mut h, &proj);
    // Botão direito no quadrado > Criar prefab.
    h.get_by_label("quadrado").click_secondary();
    h.run();
    h.get_by_label_contains("Criar prefab").click();
    h.run();
    assert!(proj.join("prefabs/quadrado.prefab").exists());
    let q = objeto(&h, "quadrado").clone();
    assert_eq!(q.prefab.as_ref().unwrap().path, "prefabs/quadrado.prefab");
    // Mudar a cor na instância vira uma mudança só dela (com ↺ para voltar).
    let id = q.id;
    {
        let ws = h.state_mut().project_mut().unwrap();
        let doc = ws.scenes.get_mut("cenas/teste.cena").unwrap();
        doc.selection = vec![id];
        doc.data.find_mut(id).unwrap().components[0]
            .set("color", nn_studio::scene_data::Prop::Str("#FF0000".into()));
    }
    h.run();
    h.get_by_label("↺").click();
    h.run();
    assert_eq!(objeto(&h, "quadrado").components[0].str("color"), "#3FA7D6");
    // Arrastar o prefab do painel Arquivos para a cena cria outra cópia.
    h.state_mut()
        .project_mut()
        .unwrap()
        .file_panel
        .reveal("prefabs/quadrado.prefab");
    h.run();
    let from = h.get_by_label("quadrado.prefab").rect().center();
    let to = h.get_by_label("Vista da cena").rect().center() + egui::vec2(100.0, 50.0);
    arrastar(&mut h, from, to);
    let copias: Vec<_> = cena(&h)
        .data
        .objects
        .iter()
        .filter(|o| o.prefab.is_some())
        .collect();
    assert_eq!(copias.len(), 2);
    // Mudar o prefab no disco muda as cópias abertas.
    let text = std::fs::read_to_string(proj.join("prefabs/quadrado.prefab")).unwrap();
    std::fs::write(
        proj.join("prefabs/quadrado.prefab"),
        text.replace("#3FA7D6", "#00FF00"),
    )
    .unwrap();
    let start = std::time::Instant::now();
    loop {
        h.run();
        if objeto(&h, "quadrado").components[0].str("color") == "#00FF00" {
            break;
        }
        assert!(start.elapsed().as_secs() < 10, "as cópias não mudaram");
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    // Salvar: a cópia guarda só o caminho do prefab e a posição.
    atalho(&mut h, Modifiers::COMMAND, Key::S);
    let text = std::fs::read_to_string(proj.join("cenas/teste.cena")).unwrap();
    assert!(
        text.contains("prefab = \"prefabs/quadrado.prefab\""),
        "{text}"
    );
    assert!(!text.contains("#00FF00"), "a cor vem do prefab: {text}");
}

#[test]
fn variaveis_do_jogo_nas_configuracoes() {
    let dir = temp("vars");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    paleta(&mut h, "configuracoes do projeto");
    h.get_by_label("➕ Variável").click();
    h.run();
    h.get_by_label("💾 Salvar configurações").click();
    h.run();
    let text = std::fs::read_to_string(proj.join("config.lua")).unwrap();
    assert!(text.contains("vars = { pontos = 0 }"), "{text}");
}

#[test]
fn eventos_pelo_painel_criar_desfazer_salvar_e_ver_como_codigo() {
    let dir = temp("eventos");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    com_cena(&mut h, &proj);
    h.get_by_label("quadrado").click();
    h.run();
    // O Inspetor mostra o resumo e leva ao painel Eventos.
    h.get_by_label("➕ Criar").click();
    h.run();
    assert!(tabs(&h).contains(&Tab::Events));
    // Nova regra do quadrado: busca "encostar" e Enter escolhe.
    h.get_by_label("➕ Regra de quadrado").click();
    h.run();
    digitar(&mut h, "encostar");
    tela(&mut h, "eventos-menu");
    h.key_press(Key::Enter);
    h.run();
    let regras = objeto(&h, "quadrado").events.clone();
    assert_eq!(regras.len(), 1);
    assert_eq!(regras[0].when[0].id, "encostar");
    assert_eq!(regras[0].when[0].str("quem"), "@este");
    // Então: destruir (o outro).
    h.get_by_label("➕ ação").click();
    h.run();
    digitar(&mut h, "destruir");
    tela(&mut h, "eventos-acao");
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(objeto(&h, "quadrado").events[0].actions[0].id, "destruir");
    tela(&mut h, "eventos");
    // Desfazer tira a ação; refazer volta.
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    assert!(objeto(&h, "quadrado").events[0].actions.is_empty());
    atalho(&mut h, Modifiers::COMMAND, Key::Y);
    assert_eq!(objeto(&h, "quadrado").events[0].actions.len(), 1);
    // Salvar grava as regras no objeto.
    atalho(&mut h, Modifiers::COMMAND, Key::S);
    let text = std::fs::read_to_string(proj.join("cenas/teste.cena")).unwrap();
    assert!(
        text.contains("{ cond = \"encostar\", quem = \"@este\", em = \"@jogador\" },"),
        "{text}"
    );
    assert!(
        text.contains("{ act = \"destruir\", alvo = \"@este\" },"),
        "{text}"
    );
    // Ver como código.
    h.get_by_label("Ver como código").click();
    h.run();
    h.get_by_label_contains("Como código: quadrado");
    let code = h
        .get_by(|n| n.role() == Role::MultilineTextInput)
        .value()
        .unwrap_or_default();
    assert!(code.contains("self.obj:destroy()"), "{code}");
    tela(&mut h, "eventos-codigo");
}

#[test]
fn tutorial_primeiro_jogo_avanca_sozinho_ate_o_fim() {
    use nn_studio::scene_data::{Component, Object, Prop, Rule};
    let dir = temp("tutorial");
    let mut h = studio(&dir, None);
    h.state_mut().settings.projects_dir = Some(dir.join("projetos"));
    h.state_mut().start_tutorial("primeiro-jogo");
    h.run_steps(4);
    assert!(dir.join("projetos/primeiro-jogo/cenas/fase.cena").is_file());
    h.state_mut().tutorial.as_mut().unwrap().pause = std::time::Duration::ZERO;
    let passo = |h: &Harness<'_, StudioApp>| h.state().tutorial.as_ref().map(|t| t.step);
    tela(&mut h, "tutorial-1");
    // Passo de leitura: Próximo.
    h.get_by_label("Próximo").click();
    h.run_steps(4);
    assert_eq!(passo(&h), Some(1));
    // Cada passo confere a cena: muda a cena como a pessoa faria e o
    // tutorial avança sozinho.
    fn cena_mut<'a>(h: &'a mut Harness<'_, StudioApp>) -> &'a mut nn_studio::scene_view::SceneDoc {
        let ws = h.state_mut().project_mut().unwrap();
        ws.scenes.get_mut("cenas/fase.cena").unwrap()
    }
    fn sprite(id: u32, name: &str, image: &str) -> Object {
        let mut o = Object::new(id, name);
        let mut c = Component::new("sprite");
        c.set("image", Prop::Str(image.into()));
        o.components.push(c);
        o
    }
    fn espera(h: &mut Harness<'_, StudioApp>, esperado: usize) {
        h.run_steps(6);
        assert_eq!(
            h.state().tutorial.as_ref().map(|t| t.step),
            Some(esperado),
            "o tutorial devia estar no passo {esperado}"
        );
    }
    cena_mut(&mut h)
        .data
        .objects
        .push(sprite(1, "bloco", "assets/imagens/bloco.png"));
    espera(&mut h, 2);
    cena_mut(&mut h).data.objects[0].scale_x = 20.0;
    espera(&mut h, 3);
    cena_mut(&mut h).data.objects[0]
        .components
        .push(Component::new("body"));
    espera(&mut h, 4);
    cena_mut(&mut h).data.objects[0].components[1].set("body", Prop::Str("static".into()));
    espera(&mut h, 5);
    cena_mut(&mut h)
        .data
        .objects
        .push(sprite(2, "heroi", "assets/imagens/heroi.png"));
    espera(&mut h, 6);
    cena_mut(&mut h).data.objects[1]
        .components
        .push(Component::behavior("nn:plataforma", &[]));
    espera(&mut h, 7);
    // Testar daqui (o "jogo" do teste é um script que sai logo).
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 8);
    cena_mut(&mut h).data.objects[1]
        .components
        .push(Component::behavior("nn:animacao", &[]));
    espera(&mut h, 9);
    let mut moeda = sprite(3, "moeda", "assets/imagens/moeda.png");
    moeda
        .components
        .push(Component::behavior("nn:coletavel", &[]));
    cena_mut(&mut h).data.objects.push(moeda);
    espera(&mut h, 10);
    for id in [4, 5, 6] {
        let mut m = Object::new(id, "moeda");
        m.prefab = Some(nn_studio::scene_data::PrefabLink {
            path: "prefabs/moeda.prefab".into(),
            base: None,
            raw: Vec::new(),
        });
        cena_mut(&mut h).data.objects.push(m);
    }
    espera(&mut h, 11);
    let mut placar = Object::new(7, "placar");
    let mut texto = Component::new("text");
    texto.set("hud", Prop::Bool(true));
    placar.components.push(texto);
    placar
        .components
        .push(Component::behavior("nn:mostrar_valor", &[]));
    cena_mut(&mut h).data.objects.push(placar);
    espera(&mut h, 12);
    // O passo do evento mostra o painel Eventos.
    assert!(tabs(&h).contains(&Tab::Events));
    let cat = nn_studio::events::catalog();
    let mut acabou = cat.condition("acabaram").unwrap().new_item(false);
    acabou.set("nome", Prop::Str("moeda".into()));
    cena_mut(&mut h).data.events.push(Rule {
        when: vec![acabou],
        actions: vec![cat.action("reiniciar").unwrap().new_item(false)],
        ..Default::default()
    });
    espera(&mut h, 13);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 14);
    assert!(!cena_mut(&mut h).is_dirty(), "Testar daqui salvou");
    h.get_by_label("Próximo").click();
    espera(&mut h, 15);
    tela(&mut h, "tutorial-fim");
    h.get_by_label("Concluir").click();
    h.run_steps(4);
    assert!(h.state().tutorial.is_none());
}

#[test]
fn tutorial_de_eventos_avanca_com_as_regras() {
    use nn_studio::scene_data::{Prop, Rule};
    let dir = temp("tutorial-eventos");
    let mut h = studio(&dir, None);
    h.state_mut().settings.projects_dir = Some(dir.join("projetos"));
    h.state_mut().start_tutorial("eventos");
    h.run_steps(4);
    h.state_mut().tutorial.as_mut().unwrap().pause = std::time::Duration::ZERO;
    fn espera(h: &mut Harness<'_, StudioApp>, esperado: usize) {
        h.run_steps(6);
        assert_eq!(h.state().tutorial.as_ref().map(|t| t.step), Some(esperado));
    }
    fn mapa<'a>(h: &'a mut Harness<'_, StudioApp>) -> &'a mut nn_studio::scene_view::SceneDoc {
        let ws = h.state_mut().project_mut().unwrap();
        // A fase abre na frente.
        assert_eq!(ws.active_scene.as_deref(), Some("cenas/mapa.cena"));
        ws.scenes.get_mut("cenas/mapa.cena").unwrap()
    }
    h.get_by_label("Próximo").click();
    espera(&mut h, 1);
    let porta = mapa(&mut h)
        .data
        .objects
        .iter()
        .find(|o| o.name == "porta")
        .unwrap()
        .id;
    mapa(&mut h).selection = vec![porta];
    espera(&mut h, 2);
    h.get_by_label("Próximo").click();
    espera(&mut h, 3);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 4);
    let cat = nn_studio::events::catalog();
    let tecla = |modo: &str| {
        let mut c = cat.condition("tecla").unwrap().new_item(false);
        c.set("modo", Prop::Str(modo.into()));
        c.set("tecla", Prop::Str("shift".into()));
        c
    };
    mapa(&mut h).data.events.push(Rule {
        when: vec![tecla("segurar")],
        repeat: true,
        ..Default::default()
    });
    espera(&mut h, 5);
    let cor = cat.action("cor").unwrap().new_item(false);
    mapa(&mut h)
        .data
        .events
        .last_mut()
        .unwrap()
        .actions
        .push(cor.clone());
    espera(&mut h, 6);
    mapa(&mut h).data.events.push(Rule {
        when: vec![tecla("soltar")],
        actions: vec![cor],
        ..Default::default()
    });
    espera(&mut h, 7);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 8);
    h.get_by_label("Próximo").click();
    espera(&mut h, 9);
    h.get_by_label("Concluir").click();
    h.run_steps(2);
    assert!(h.state().tutorial.is_none());
}

/// Segura o botão do mouse e passa pelos pontos (um traço de lápis).
fn traco(h: &mut Harness<'_, StudioApp>, pontos: &[egui::Pos2]) {
    let ev = |h: &mut Harness<'_, StudioApp>, e: egui::Event| h.input_mut().events.push(e);
    let botao = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    ev(h, egui::Event::PointerMoved(pontos[0]));
    h.step();
    ev(h, botao(pontos[0], true));
    h.step();
    for p in &pontos[1..] {
        ev(h, egui::Event::PointerMoved(*p));
        h.step();
    }
    ev(h, botao(*pontos.last().unwrap(), false));
    h.step();
}

#[test]
fn tutorial_desenhe_seu_heroi_com_o_editor_de_pixel_art() {
    use nn_studio::scene_data::{Component, Object, Prop};
    let dir = temp("tutorial-pixel");
    let mut h = studio(&dir, None);
    h.state_mut().settings.projects_dir = Some(dir.join("projetos"));
    h.state_mut().start_tutorial("desenhe-seu-heroi");
    h.run_steps(4);
    let projeto = dir.join("projetos/desenhe-seu-heroi");
    assert!(projeto.join("cenas/fase.cena").is_file());
    h.state_mut().tutorial.as_mut().unwrap().pause = std::time::Duration::ZERO;
    fn espera(h: &mut Harness<'_, StudioApp>, esperado: usize) {
        h.run_steps(6);
        assert_eq!(
            h.state().tutorial.as_ref().map(|t| t.step),
            Some(esperado),
            "o tutorial devia estar no passo {esperado}"
        );
    }
    h.get_by_label("Próximo").click();
    espera(&mut h, 1);

    // ➕ Novo > Imagem (pixel art): o diálogo cria o PNG e abre o editor.
    h.get_by_label("➕ Novo").click();
    h.run_steps(2);
    h.get_by_label("🎨 Imagem (pixel art)").click();
    h.run_steps(2);
    h.key_press(Key::Enter);
    espera(&mut h, 2);
    let rel = "assets/imagens/desenho.png";
    assert_eq!(h.state().project().unwrap().created_images, [rel]);
    assert!(projeto.join(rel).is_file());
    assert!(tabs(&h).contains(&Tab::Image(rel.into())));
    tela(&mut h, "pixel-novo");

    // Desenhar com o mouse: um traço na tela do editor.
    let tela_rect = nn_studio::tutorial::anchor_rect(&h.ctx, "pixel:tela").expect("tela do editor");
    let c = tela_rect.center();
    let doc = |h: &Harness<'_, StudioApp>| {
        h.state()
            .project()
            .unwrap()
            .pixels
            .get(rel)
            .unwrap()
            .img
            .clone()
    };
    // 16 × 16 ajustado na tela: cada pixel tem vários pontos da tela.
    let passo = (tela_rect.height() - 24.0) / 16.0;
    let linha: Vec<egui::Pos2> = (-6..=6)
        .map(|i| c + egui::vec2(i as f32 * passo, 0.0))
        .collect();
    traco(&mut h, &linha);
    let pintados = doc(&h).pixels().filter(|p| p.0[3] > 0).count();
    assert!(pintados >= 12, "o traço pintou {pintados} pixels");
    // O resto do desenho (e as cores), direto na imagem.
    {
        let ws = h.state_mut().project_mut().unwrap();
        let d = ws.pixels.get_mut(rel).unwrap();
        for (i, cor) in [[255, 0, 0, 255], [0, 0, 255, 255]].into_iter().enumerate() {
            for x in 0..16 {
                d.img.put_pixel(x, 2 + i as u32, image::Rgba(cor));
            }
        }
    }
    espera(&mut h, 4);
    // Duplicar o quadro (botão do painel Quadros).
    h.get_by_label("Duplicar").click();
    espera(&mut h, 5);
    assert_eq!(doc(&h).width(), 32);
    {
        let ws = h.state_mut().project_mut().unwrap();
        let d = ws.pixels.get_mut(rel).unwrap();
        assert_eq!(d.current, 1, "o quadro novo fica selecionado");
        for y in 12..16 {
            d.img.put_pixel(16 + 3, y, image::Rgba([0, 0, 0, 0]));
            d.img.put_pixel(16 + 6, y, image::Rgba([0, 0, 0, 255]));
        }
    }
    espera(&mut h, 6);
    h.get_by_label("Animações (0)").click();
    h.run_steps(2);
    h.get_by_label("+ Nova animação").click();
    h.run_steps(2);
    {
        let ws = h.state_mut().project_mut().unwrap();
        let d = ws.pixels.get_mut(rel).unwrap();
        assert_eq!(
            d.anim.anims[0].frames.len(),
            2,
            "a primeira usa todos os quadros"
        );
        d.anim.anims[0].name = "andar".into();
    }
    espera(&mut h, 7);
    tela(&mut h, "pixel-animacao");
    // Ctrl+S com a imagem em foco (clicar nela deu o foco à aba).
    h.key_press_modifiers(Modifiers::COMMAND, Key::S);
    espera(&mut h, 8);
    let anim = std::fs::read_to_string(projeto.join("assets/imagens/desenho.anim")).unwrap();
    assert!(
        anim.contains("name = \"andar\", frames = { 1, 2 }"),
        "{anim}"
    );
    assert_eq!(image::open(projeto.join(rel)).unwrap().width(), 32);

    // Na cena, com a animação.
    {
        let ws = h.state_mut().project_mut().unwrap();
        ws.open_file("cenas/fase.cena", None);
        let doc = ws.scenes.get_mut("cenas/fase.cena").unwrap();
        let mut o = Object::new(50, "meu_heroi");
        let mut s = Component::new("sprite");
        s.set("image", Prop::Str(rel.into()));
        o.components.push(s);
        doc.data.objects.push(o);
    }
    espera(&mut h, 9);
    {
        let ws = h.state_mut().project_mut().unwrap();
        let doc = ws.scenes.get_mut("cenas/fase.cena").unwrap();
        let o = doc.data.objects.iter_mut().find(|o| o.id == 50).unwrap();
        o.components[0].set("animation", Prop::Str("andar".into()));
    }
    espera(&mut h, 10);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 11);
    h.get_by_label("Concluir").click();
    h.run_steps(4);
    assert!(h.state().tutorial.is_none());
}

/// Centro da célula (col, row) do mapa pintado na cena, na tela.
fn celula(h: &Harness<'_, StudioApp>, scene: &str, col: i64, row: i64) -> egui::Pos2 {
    let ws = h.state().project().unwrap();
    let doc = &ws.scenes[scene];
    let id = doc.painting.expect("pintando");
    let o = doc.data.find(id).unwrap();
    let c = o.component("map").unwrap();
    let map = &ws.map_docs[c.str("map")];
    let s = c.num("scale");
    let (tw, th) = (map.tile_w as f64 * s, map.tile_h as f64 * s);
    let x = o.x + (col as f64 + 0.5) * tw;
    let y = o.y + ((map.height as i64 - 1 - row) as f64 + 0.5) * th;
    let rect = h.get_by_label("Vista da cena").rect();
    doc.camera.to_screen(rect, x, y)
}

/// Um clique (apertar e soltar) num ponto.
fn clique(h: &mut Harness<'_, StudioApp>, p: egui::Pos2) {
    traco(h, &[p, p]);
    h.run_steps(2);
}

#[test]
fn tutorial_monte_uma_fase_com_tiles() {
    use nn_studio::scene_data::{Component, EventItem, Object, Prop, Rule};
    let dir = temp("tutorial-tiles");
    let mut h = studio(&dir, None);
    h.state_mut().settings.projects_dir = Some(dir.join("projetos"));
    h.state_mut().start_tutorial("fase-com-tiles");
    h.run_steps(4);
    let projeto = dir.join("projetos/fase-com-tiles");
    assert!(projeto.join("assets/imagens/terreno.tsj").is_file());
    h.state_mut().tutorial.as_mut().unwrap().pause = std::time::Duration::ZERO;
    fn espera(h: &mut Harness<'_, StudioApp>, esperado: usize) {
        h.run_steps(6);
        assert_eq!(
            h.state().tutorial.as_ref().map(|t| t.step),
            Some(esperado),
            "o tutorial devia estar no passo {esperado}"
        );
    }
    const CENA: &str = "cenas/fase.cena";
    h.get_by_label("Próximo").click();
    espera(&mut h, 1);

    // ➕ Objeto > Mapa de tiles...: a janela já vem com o terreno.tsj.
    h.get_by_label("➕ Objeto").click();
    h.run_steps(2);
    h.get_by_label("Mapa de tiles...").click();
    h.run_steps(2);
    tela(&mut h, "tiles-novo-mapa");
    h.key_press(Key::Enter);
    espera(&mut h, 2);
    assert!(projeto.join("mapas/fase.tmj").is_file());
    assert!(tabs(&h).contains(&Tab::Tiles), "o painel Tiles abriu");
    assert!(h.state().project().unwrap().scenes[CENA].painting.is_some());

    // Terreno terra: um traço de duas linhas embaixo.
    let terra =
        nn_studio::tutorial::anchor_rect(&h.ctx, "tiles.terreno:terra").expect("botão do terreno");
    clique(&mut h, terra.center());
    assert_eq!(
        h.state().project().unwrap().paint.tool,
        nn_studio::paint::PaintTool::Terrain
    );
    for row in [12, 13] {
        let pontos: Vec<egui::Pos2> = (8..=20).map(|c| celula(&h, CENA, c, row)).collect();
        traco(&mut h, &pontos);
    }
    espera(&mut h, 3);
    tela(&mut h, "tiles-chao");
    let mapa = |h: &Harness<'_, StudioApp>| {
        h.state().project().unwrap().map_docs["mapas/fase.tmj"].clone()
    };
    // As células pintadas são terra cheia (o tile 15 do terreno.tsj); a de
    // cima tem a grama (borda de cima).
    assert_eq!(mapa(&h).get(0, 10, 13), 15);
    assert_ne!(mapa(&h).get(0, 10, 11), 0);

    // A tábua, escolhida no tileset (o último tile da segunda linha).
    // (O painel muda de tamanho: a grade é procurada a cada vez.)
    let tile = |h: &Harness<'_, StudioApp>, id: u32| {
        let grade = nn_studio::tutorial::anchor_rect(&h.ctx, "tiles.tileset").expect("tileset");
        let z = grade.width() / 128.0;
        grade.min + egui::vec2((id % 8) as f32 * 16.0 + 8.0, (id / 8) as f32 * 16.0 + 8.0) * z
    };
    let p = tile(&h, 15);
    clique(&mut h, p);
    assert_eq!(h.state().project().unwrap().paint.stamp.tiles, [16]);
    let pontos: Vec<egui::Pos2> = (11..=14).map(|c| celula(&h, CENA, c, 9)).collect();
    traco(&mut h, &pontos);
    espera(&mut h, 4);

    // Camada de enfeites com flores.
    h.get_by_label("+ Camada").click();
    h.run_steps(2);
    assert_eq!(
        mapa(&h).tile_layer(mapa(&h).layer).unwrap().name,
        "enfeites"
    );
    let p = tile(&h, 38);
    clique(&mut h, p);
    for c in [9, 15] {
        let p = celula(&h, CENA, c, 11);
        clique(&mut h, p);
    }
    espera(&mut h, 5);

    // De volta à camada chao (direto), e a lava animada.
    {
        let ws = h.state_mut().project_mut().unwrap();
        ws.map_docs.get_mut("mapas/fase.tmj").unwrap().layer = 0;
    }
    h.run_steps(2);
    let p = tile(&h, 34);
    clique(&mut h, p);
    let pontos: Vec<egui::Pos2> = (16..=18).map(|c| celula(&h, CENA, c, 12)).collect();
    traco(&mut h, &pontos);
    espera(&mut h, 6);

    // Marcar a lava: Tipo, escrever "lava" e clicar no tile.
    h.get_by_label("Tipo").click();
    h.run_steps(2);
    h.get_by(|n| n.role() == Role::TextInput && n.placeholder() == Some("lava"))
        .click();
    h.run_steps(1);
    // (A lava animada redesenha sempre: `run_steps` em vez de `run`.)
    h.event(egui::Event::Text("lava".into()));
    h.run_steps(2);
    let p = tile(&h, 34);
    clique(&mut h, p);
    espera(&mut h, 7);
    tela(&mut h, "tiles-tipo");
    let ts = &h.state().project().unwrap().tilesets["assets/imagens/terreno.tsj"];
    assert_eq!(ts.data.info(34).kind, "lava");
    assert!(ts.is_dirty());

    // Esc sai do Pintar; o herói (direto na cena) e o comportamento.
    let centro = h.get_by_label("Vista da cena").rect().center();
    h.hover_at(centro);
    h.run_steps(1);
    h.key_press(Key::Escape);
    h.run_steps(2);
    assert!(h.state().project().unwrap().scenes[CENA].painting.is_none());
    {
        let ws = h.state_mut().project_mut().unwrap();
        let doc = ws.scenes.get_mut(CENA).unwrap();
        let mut o = Object::new(50, "heroi");
        let mut s = Component::new("sprite");
        s.set("image", Prop::Str("assets/imagens/heroi.png".into()));
        o.components.push(s);
        doc.data.objects.push(o);
    }
    espera(&mut h, 8);
    {
        let ws = h.state_mut().project_mut().unwrap();
        let doc = ws.scenes.get_mut(CENA).unwrap();
        let o = doc.data.objects.iter_mut().find(|o| o.id == 50).unwrap();
        let mut b = Component::new("behavior");
        b.set("script", Prop::Str("nn:plataforma".into()));
        o.components.push(b);
    }
    espera(&mut h, 9);
    // A regra da lava.
    {
        let ws = h.state_mut().project_mut().unwrap();
        let doc = ws.scenes.get_mut(CENA).unwrap();
        let mut quando = EventItem {
            id: "tile".into(),
            ..Default::default()
        };
        quando.set("quem", Prop::Str("@jogador".into()));
        quando.set("tipo", Prop::Str("lava".into()));
        doc.data.events.push(Rule {
            when: vec![quando],
            actions: vec![EventItem {
                id: "reiniciar".into(),
                ..Default::default()
            }],
            ..Default::default()
        });
    }
    espera(&mut h, 10);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 11);
    // Testar salvou tudo: o mapa e o tileset (com a lava marcada).
    let tmj = std::fs::read_to_string(projeto.join("mapas/fase.tmj")).unwrap();
    assert!(
        tmj.contains("\"source\": \"../assets/imagens/terreno.tsj\""),
        "{tmj}"
    );
    assert!(tmj.contains("\"name\": \"enfeites\""), "{tmj}");
    let tsj = std::fs::read_to_string(projeto.join("assets/imagens/terreno.tsj")).unwrap();
    assert!(tsj.contains("\"value\": \"lava\""), "{tsj}");
    assert!(!h.state().project().unwrap().map_docs["mapas/fase.tmj"].is_dirty());
    h.get_by_label("Concluir").click();
    h.run_steps(4);
    assert!(h.state().tutorial.is_none());
}

#[test]
fn aba_do_mapa_pinta_desfaz_e_salva_no_formato_do_tiled() {
    let dir = temp("aba-mapa");
    let proj = plataforma(&dir);
    let original = std::fs::read_to_string(proj.join("fases/fase1.tmj")).unwrap();
    let mut h = studio(&dir, Some(proj.clone()));
    const MAPA: &str = "fases/fase1.tmj";
    h.state_mut().project_mut().unwrap().open_file(MAPA, None);
    h.run_steps(4);
    assert!(tabs(&h).contains(&Tab::Map(MAPA.into())));
    assert!(tabs(&h).contains(&Tab::Tiles), "o painel Tiles abre junto");
    assert_eq!(
        h.state().project().unwrap().tiles_map.as_deref(),
        Some(MAPA)
    );
    tela(&mut h, "aba-mapa");
    let antes = h.state().project().unwrap().map_docs[MAPA].clone();
    let camada = antes.layer;
    // Um tile no canto de cima à esquerda da vista do mapa (céu).
    let vista = nn_studio::tutorial::anchor_rect(&h.ctx, "mapa.vista").expect("vista do mapa");
    let ws = h.state_mut().project_mut().unwrap();
    ws.paint.stamp = nn_studio::map_doc::Stamp::single(3);
    let _ = ws;
    // Acha a célula vazia mais perto do centro, procurando pela tela.
    let mut alvo = None;
    for dy in 0..40 {
        let p = vista.center() + egui::vec2(0.0, -(dy as f32) * 8.0);
        alvo = Some(p);
        clique(&mut h, p);
        if h.state().project().unwrap().map_docs[MAPA].is_dirty() {
            break;
        }
    }
    let depois = h.state().project().unwrap().map_docs[MAPA].clone();
    assert!(depois.is_dirty(), "pintou {alvo:?}");
    let mudou = (0..depois.width as i64 * depois.height as i64)
        .filter(|i| {
            depois.get(camada, i % depois.width as i64, i / depois.width as i64)
                != antes.get(camada, i % antes.width as i64, i / antes.width as i64)
        })
        .count();
    assert_eq!(mudou, 1);
    // Ctrl+Z com a aba do mapa em foco desfaz.
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run_steps(3);
    assert!(!h.state().project().unwrap().map_docs[MAPA].is_dirty());
    h.key_press_modifiers(Modifiers::COMMAND, Key::Y);
    h.run_steps(3);
    assert!(h.state().project().unwrap().map_docs[MAPA].is_dirty());
    // Ctrl+S grava; o que o editor não conhece (objetos, propriedades) fica.
    h.key_press_modifiers(Modifiers::COMMAND, Key::S);
    h.run_steps(3);
    assert!(!h.state().project().unwrap().map_docs[MAPA].is_dirty());
    let salvo = std::fs::read_to_string(proj.join(MAPA)).unwrap();
    assert_ne!(salvo, original);
    let a: serde_json::Value = serde_json::from_str(&original).unwrap();
    let b: serde_json::Value = serde_json::from_str(&salvo).unwrap();
    assert_eq!(
        a["layers"][4], b["layers"][4],
        "a camada de objetos fica igual"
    );
    assert_eq!(a["tilesets"], b["tilesets"]);
    // E a engine lê o mapa gravado.
    let root = proj.clone();
    nn_tilemap::load_tiled(
        Path::new(MAPA),
        |p: &Path| std::fs::read_to_string(root.join(p)).map_err(|e| e.to_string()),
        1.0,
    )
    .expect("a engine lê o mapa");
}

#[cfg(unix)]
#[test]
fn pintar_na_cena_com_o_jogo_aberto_manda_os_tiles() {
    use nn_studio::scene_data::{Component, Object, Prop};
    let dir = temp("tiles-ao-vivo");
    let proj = plataforma(&dir);
    let mut h = studio_with(&dir, Some(proj.clone()), jogo_ao_vivo(&dir), false);
    com_cena(&mut h, &proj);
    {
        let ws = h.state_mut().project_mut().unwrap();
        let doc = ws.scenes.get_mut("cenas/teste.cena").unwrap();
        let mut o = Object::new(70, "mapa");
        // A célula (2, 2) fica no meio da vista.
        o.x = -40.0;
        o.y = -200.0;
        let mut c = Component::new("map");
        c.set("map", Prop::Str("fases/fase1.tmj".into()));
        c.set("scale", Prop::Num(1.0));
        o.components.push(c);
        doc.data.objects.push(o);
        doc.selection = vec![70];
    }
    h.run_steps(2);
    // (O botão está na barra da cena e no Inspetor.)
    h.get_all_by_label("✏ Pintar").next().unwrap().click();
    h.run_steps(3);
    assert_eq!(cena(&h).painting, Some(70));
    assert!(tabs(&h).contains(&Tab::Tiles));
    h.key_press(Key::F6);
    h.run_steps(3);
    let start = std::time::Instant::now();
    while h.state().project().unwrap().game_scene.is_none() {
        assert!(start.elapsed().as_secs() < 20, "o jogo não avisou a cena");
        std::thread::sleep(std::time::Duration::from_millis(50));
        h.step();
    }
    h.run_steps(3);
    // Testar salvou a cena; agora pinta um tile dentro do mapa.
    h.state_mut().project_mut().unwrap().paint.stamp = nn_studio::map_doc::Stamp::single(7);
    let p = celula(&h, "cenas/teste.cena", 2, 2);
    clique(&mut h, p);
    assert!(h.state().project().unwrap().map_docs["fases/fase1.tmj"].is_dirty());
    let recebido = dir.join("recebido.txt");
    let start = std::time::Instant::now();
    let linha = loop {
        let t = std::fs::read_to_string(&recebido).unwrap_or_default();
        if let Some(l) = t.lines().find(|l| l.contains("\"tiles\"")) {
            break l.to_string();
        }
        assert!(
            start.elapsed().as_secs() < 10,
            "o jogo não recebeu os tiles: {t}"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
        h.step();
    };
    let msg: serde_json::Value = serde_json::from_str(&linha).unwrap();
    assert_eq!(msg["id"], 70);
    // [camada, coluna, linha, tile], contando de 1.
    let t = &msg["value"][0];
    assert_eq!((t[1].as_i64(), t[2].as_i64()), (Some(3), Some(3)), "{msg}");
    h.key_press_modifiers(Modifiers::SHIFT, Key::F5);
    h.run_steps(3);
}

#[test]
fn interface_menu_pronto_ancora_e_tema() {
    let dir = temp("interface");
    let proj = plataforma(&dir);
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), true);
    com_cena(&mut h, &proj);

    // ➕ Objeto > Interface > Menu com botões: painel com 3 botões em coluna.
    h.get_by_label("➕ Objeto").click();
    h.run_steps(2);
    submenu_interface(&mut h);
    h.run_steps(3);
    h.get_by_label("Menu com botões").click();
    h.run_steps(3);
    let menu = objeto(&h, "menu").clone();
    assert!(menu.component("panel").is_some() && menu.component("layout").is_some());
    let nomes: Vec<&str> = menu.children.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(nomes, ["jogar", "opcoes", "sair"]);
    // O grupo arruma os botões em coluna, de cima para baixo.
    let ys: Vec<f64> = menu.children.iter().map(|c| c.y).collect();
    assert!(ys[0] > ys[1] && ys[1] > ys[2], "{ys:?}");
    assert_eq!(cena(&h).selection, vec![menu.id]);
    tela(&mut h, "interface-menu");

    // Preso no canto de cima à esquerda: vai para o canto, com margem (o
    // painel tem 320 × 276).
    h.get_by_label("Preso em");
    h.get_by_label("Preso em cima, à esquerda").click();
    h.run();
    let menu2 = objeto(&h, "menu").clone();
    assert_eq!(menu2.anchor, "top_left");
    assert_eq!((menu2.x, menu2.y), (24.0 + 160.0, -24.0 - 138.0));
    // No editor, o canto do painel fica no canto da tela do jogo.
    let t = cena(&h).data.world_transform(menu.id).unwrap();
    assert!((t.x - 160.0 - (-640.0 + 24.0)).abs() < 1.0, "{}", t.x);

    // A cor do botão vem do tema; mudar o tema muda o desenho.
    let botao = menu2.children[0].component("button").unwrap();
    assert_eq!(botao.str("color"), "");
    assert_eq!(
        nn_studio::scene_data::themed(botao.str("color"), "button"),
        "#3FA7D6"
    );
    // Salvar grava a âncora.
    atalho(&mut h, Modifiers::COMMAND, Key::S);
    let text = std::fs::read_to_string(proj.join("cenas/teste.cena")).unwrap();
    assert!(text.contains("anchor = \"top_left\""), "{text}");
    assert!(text.contains("type = \"button\""), "{text}");
}

#[test]
fn tutorial_monte_um_menu() {
    use nn_studio::scene_data::{EventItem, Prop, Rule};
    let dir = temp("tutorial-menu");
    let mut h = studio_with(&dir, None, fake_game(&dir), true);
    h.state_mut().settings.projects_dir = Some(dir.join("projetos"));
    h.state_mut().start_tutorial("monte-um-menu");
    h.run_steps(4);
    let projeto = dir.join("projetos/menu-e-hud");
    assert!(projeto.join("assets/imagens/coracao.png").is_file());
    h.state_mut().tutorial.as_mut().unwrap().pause = std::time::Duration::ZERO;
    fn espera(h: &mut Harness<'_, StudioApp>, esperado: usize) {
        h.run_steps(6);
        assert_eq!(
            h.state().tutorial.as_ref().map(|t| t.step),
            Some(esperado),
            "o tutorial devia estar no passo {esperado}"
        );
    }
    const FASE: &str = "cenas/fase.cena";
    const PAUSA: &str = "cenas/pausa.cena";
    h.get_by_label("Próximo").click();
    espera(&mut h, 1);

    // ➕ Objeto > Interface > Ícones.
    h.get_by_label("➕ Objeto").click();
    h.run_steps(2);
    submenu_interface(&mut h);
    h.run_steps(3);
    h.get_by_label("Ícones").click();
    espera(&mut h, 2);
    let id = h.state().project().unwrap().scenes[FASE].selection[0];
    {
        let ws = h.state_mut().project_mut().unwrap();
        let doc = ws.scenes.get_mut(FASE).unwrap();
        let c = doc
            .data
            .find_mut(id)
            .unwrap()
            .components
            .get_mut(0)
            .unwrap();
        c.set("image", Prop::Str("assets/imagens/coracao.png".into()));
        c.set("var", Prop::Str("vidas".into()));
        c.set("max", Prop::Num(3.0));
    }
    espera(&mut h, 3);
    h.get_by_label("Preso em cima, à esquerda").click();
    espera(&mut h, 4);
    tela(&mut h, "menu-coracoes");

    // ➕ Novo > Cena visual, por cima.
    h.get_by_label("➕ Novo").click();
    h.run_steps(2);
    h.get_by_label("🖼 Cena visual").click();
    h.run_steps(2);
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.run_steps(1);
    h.event(egui::Event::Text("pausa".into()));
    h.run_steps(2);
    h.get_by_label("Abre por cima de outra cena (pausa, opções)")
        .click();
    h.run_steps(1);
    h.get_by_label("Criar").click();
    espera(&mut h, 5);
    let texto = std::fs::read_to_string(projeto.join(PAUSA)).unwrap();
    assert!(texto.contains("overlay = true"), "{texto}");
    assert!(tabs(&h).contains(&Tab::Scene(PAUSA.into())));

    // O deslizador do volume, arrastado para dentro do grupo.
    h.get_by_label("➕ Objeto").click();
    h.run_steps(2);
    submenu_interface(&mut h);
    h.run_steps(3);
    h.get_by_label("Deslizador").click();
    h.run_steps(3);
    {
        let ws = h.state_mut().project_mut().unwrap();
        let doc = ws.scenes.get_mut(PAUSA).unwrap();
        let id = doc.selection[0];
        let c = doc
            .data
            .find_mut(id)
            .unwrap()
            .components
            .get_mut(0)
            .unwrap();
        c.set("target", Prop::Str("musica".into()));
    }
    h.run_steps(2);
    let from = h.get_by_label("deslizador").rect().center();
    let to = h.get_by_label("botoes").rect().center();
    // (Com o tutorial aberto a tela redesenha sempre: passos contados.)
    h.hover_at(from);
    h.run_steps(1);
    h.drag_at(from);
    h.run_steps(1);
    for i in 1..=6 {
        h.hover_at(from + (to - from) * (i as f32 / 6.0));
        h.run_steps(1);
    }
    h.drop_at(to);
    h.run_steps(1);
    espera(&mut h, 6);
    tela(&mut h, "menu-pausa");

    // Na fase, Esc abre a pausa por cima.
    h.state_mut().project_mut().unwrap().open_file(FASE, None);
    h.run_steps(2);
    {
        let ws = h.state_mut().project_mut().unwrap();
        let doc = ws.scenes.get_mut(FASE).unwrap();
        let mut quando = EventItem {
            id: "tecla".into(),
            ..Default::default()
        };
        quando.set("modo", Prop::Str("apertar".into()));
        quando.set("tecla", Prop::Str("voltar".into()));
        let mut entao = EventItem {
            id: "por_cima".into(),
            ..Default::default()
        };
        entao.set("cena", Prop::Str(PAUSA.into()));
        doc.data.events.push(Rule {
            when: vec![quando],
            actions: vec![entao],
            ..Default::default()
        });
    }
    espera(&mut h, 7);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 8);
    h.get_by_label("Próximo").click();
    espera(&mut h, 9);
    // Testar salvou as duas cenas.
    let fase = std::fs::read_to_string(projeto.join(FASE)).unwrap();
    assert!(
        fase.contains("anchor = \"top_left\"") && fase.contains("act = \"por_cima\""),
        "{fase}"
    );
    let pausa = std::fs::read_to_string(projeto.join(PAUSA)).unwrap();
    assert!(pausa.contains("target = \"musica\""), "{pausa}");
    h.get_by_label("Concluir").click();
    h.run_steps(4);
    assert!(h.state().tutorial.is_none());
}

/// Abre o submenu Interface do ➕ Objeto (já aberto).
fn submenu_interface(h: &mut Harness<'_, StudioApp>) {
    h.get_by(|n| n.label().is_some_and(|l| l.starts_with("Interface")))
        .hover();
}

#[test]
fn modelo_plataforma_abre_os_menus_no_editor() {
    let dir = temp("modelo-menus");
    let proj = dir.join("jogo");
    nn_run::create_project(&nn_run::NewOptions {
        dir: proj.clone(),
        title: None,
        template: nn_run::Template::Visual,
    })
    .unwrap();
    // (O título tem um herói animado: a tela redesenha sempre.)
    let options = StudioOptions {
        settings_path: Some(dir.join("preferencias.json")),
        open: Some(proj.clone()),
        exe: Some(fake_game(&dir)),
        check_updates: false,
    };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_eframe(move |cc| {
            let mut app = StudioApp::new(&cc.egui_ctx, options);
            app.set_render_state(cc.wgpu_render_state.clone());
            app
        });
    h.run_steps(4);
    h.state_mut()
        .project_mut()
        .unwrap()
        .open_file("cenas/opcoes.cena", None);
    h.run_steps(4);
    let doc = &h.state().project().unwrap().scenes["cenas/opcoes.cena"];
    assert!(doc.data.overlay);
    // Abrir não muda nada (os grupos já vêm arrumados).
    assert!(!doc.is_dirty());
    h.get_by_label("musica").click();
    h.run_steps(4);
    h.get_by_label("Deslizador");
    tela(&mut h, "modelo-opcoes");
    h.state_mut()
        .project_mut()
        .unwrap()
        .open_file("cenas/fase.cena", None);
    // (As moedas animadas redesenham sempre: passos contados.)
    h.run_steps(4);
    assert!(!h.state().project().unwrap().scenes["cenas/fase.cena"].is_dirty());
    h.get_by_label("vidas").click();
    h.run_steps(4);
    tela(&mut h, "modelo-hud");
}

#[test]
fn particulas_efeitos_prontos_previa_e_seta() {
    let dir = temp("particulas");
    let proj = plataforma(&dir);
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), true);
    com_cena(&mut h, &proj);
    // ➕ Objeto > Partículas > Fogo.
    h.get_by_label("➕ Objeto").click();
    h.run_steps(2);
    h.get_by(|n| n.label().is_some_and(|l| l.starts_with("Partículas")))
        .hover();
    h.run_steps(3);
    h.get_by_label("Fogo").click();
    // (As partículas animam: a tela redesenha sempre.)
    h.run_steps(20);
    let fogo = objeto(&h, "fogo").clone();
    let c = fogo.component("particles").unwrap();
    assert_eq!(c.str("color_mid"), "#FF7A1A");
    assert_eq!(c.str("area"), "circulo");
    h.get_by_label("Emissão");
    h.get_by_label("Aparência");
    h.get_by_label("✨ Partículas");
    tela(&mut h, "particulas-fogo");

    // A ponta da seta muda a direção (para a direita: 0°).
    let rect = h.get_by_label("Vista da cena").rect();
    let t = cena(&h).data.world_transform(fogo.id).unwrap();
    let centro = cena(&h).camera.to_screen(rect, t.x, t.y);
    let ponta = centro + egui::vec2(0.0, -90.0);
    let destino = centro + egui::vec2(90.0, 0.0);
    h.hover_at(ponta);
    h.run_steps(1);
    h.drag_at(ponta);
    h.run_steps(1);
    for i in 1..=6 {
        h.hover_at(ponta + (destino - ponta) * (i as f32 / 6.0));
        h.run_steps(1);
    }
    h.drop_at(destino);
    h.run_steps(2);
    let dir = objeto(&h, "fogo")
        .component("particles")
        .unwrap()
        .num("direction");
    assert!(!(5.0..=355.0).contains(&dir), "direção {dir}");
    atalho_passos(&mut h, Modifiers::COMMAND, Key::Z);
    let dir = objeto(&h, "fogo")
        .component("particles")
        .unwrap()
        .num("direction");
    assert_eq!(dir, 90.0, "desfazer volta a direção");

    // Efeito pronto pelo Inspetor: vira explosão; 💥 Soltar na prévia.
    h.get_by_label("✨ Efeito pronto").click();
    h.run_steps(2);
    h.get_by_label("Explosão").click();
    h.run_steps(3);
    let c = objeto(&h, "fogo").component("particles").unwrap().clone();
    assert_eq!(c.str("mode"), "explosao");
    assert_eq!(c.str("when_done"), "destruir");
    h.get_by_label("💥 Soltar").click();
    h.run_steps(3);
    tela(&mut h, "particulas-explosao");
    // Pausar a prévia.
    h.get_by_label("✨ Partículas").click();
    h.run_steps(2);
    assert!(cena(&h).particles_paused);
    // Salvar grava os campos novos.
    atalho_passos(&mut h, Modifiers::COMMAND, Key::S);
    let text = std::fs::read_to_string(proj.join("cenas/teste.cena")).unwrap();
    assert!(text.contains("mode = \"explosao\""), "{text}");
}

/// Atalho de teclado com a tela redesenhando sempre.
fn atalho_passos(h: &mut Harness<'_, StudioApp>, modifiers: Modifiers, key: Key) {
    h.key_press_modifiers(modifiers, key);
    h.run_steps(3);
}

#[test]
fn tutorial_efeitos_com_particulas() {
    use nn_studio::scene_data::{EventItem, Prop, Rule};
    let dir = temp("tutorial-fx");
    let mut h = studio_with(&dir, None, fake_game(&dir), true);
    h.state_mut().settings.projects_dir = Some(dir.join("projetos"));
    h.state_mut().start_tutorial("efeitos-com-particulas");
    h.run_steps(4);
    let projeto = dir.join("projetos/efeitos");
    h.state_mut().tutorial.as_mut().unwrap().pause = std::time::Duration::ZERO;
    fn espera(h: &mut Harness<'_, StudioApp>, esperado: usize) {
        h.run_steps(6);
        assert_eq!(
            h.state().tutorial.as_ref().map(|t| t.step),
            Some(esperado),
            "o tutorial devia estar no passo {esperado}"
        );
    }
    const FASE: &str = "cenas/fase.cena";
    let fx = |h: &mut Harness<'_, StudioApp>, nome: &str| {
        h.get_by_label("➕ Objeto").click();
        h.run_steps(2);
        h.get_by(|n| n.label().is_some_and(|l| l.starts_with("Partículas")))
            .hover();
        h.run_steps(3);
        h.get_by(|n| n.role() == Role::Button && n.label().as_deref() == Some(nome))
            .click();
    };
    h.get_by_label("Próximo").click();
    espera(&mut h, 1);
    fx(&mut h, "Fogo");
    espera(&mut h, 2);
    // Arrastar a ponta da seta para o lado.
    let id = h.state().project().unwrap().scenes[FASE].selection[0];
    let rect = h.get_by_label("Vista da cena").rect();
    let doc = &h.state().project().unwrap().scenes[FASE];
    let t = doc.data.world_transform(id).unwrap();
    let centro = doc.camera.to_screen(rect, t.x, t.y);
    let (ponta, destino) = (
        centro + egui::vec2(0.0, -90.0),
        centro + egui::vec2(-90.0, -20.0),
    );
    h.hover_at(ponta);
    h.run_steps(1);
    h.drag_at(ponta);
    h.run_steps(1);
    for i in 1..=6 {
        h.hover_at(ponta + (destino - ponta) * (i as f32 / 6.0));
        h.run_steps(1);
    }
    h.drop_at(destino);
    espera(&mut h, 3);
    fx(&mut h, "Explosão");
    espera(&mut h, 4);
    tela(&mut h, "tutorial-fx");
    // O prefab (como o Criar prefab da Hierarquia faria).
    {
        let ws = h.state_mut().project_mut().unwrap();
        let doc = ws.scenes.get_mut(FASE).unwrap();
        let o = doc
            .data
            .objects
            .iter()
            .find(|o| o.name == "explosao")
            .unwrap()
            .clone();
        std::fs::create_dir_all(projeto.join("prefabs")).unwrap();
        std::fs::write(
            projeto.join("prefabs/explosao.prefab"),
            nn_studio::scene_data::write_prefab(&o),
        )
        .unwrap();
        ws.files.rescan();
    }
    espera(&mut h, 5);
    {
        let ws = h.state_mut().project_mut().unwrap();
        let doc = ws.scenes.get_mut(FASE).unwrap();
        let mut quando = EventItem {
            id: "tecla".into(),
            ..Default::default()
        };
        quando.set("modo", Prop::Str("apertar".into()));
        quando.set("tecla", Prop::Str("acao".into()));
        let mut entao = EventItem {
            id: "criar".into(),
            ..Default::default()
        };
        entao.set("prefab", Prop::Str("prefabs/explosao.prefab".into()));
        entao.set("onde", Prop::Str("fogo".into()));
        doc.data.events.push(Rule {
            when: vec![quando],
            actions: vec![entao],
            ..Default::default()
        });
    }
    espera(&mut h, 6);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 7);
    let fase = std::fs::read_to_string(projeto.join(FASE)).unwrap();
    assert!(fase.contains("type = \"particles\""), "{fase}");
    h.get_by_label("Concluir").click();
    h.run_steps(4);
    assert!(h.state().tutorial.is_none());
}

#[test]
fn modelo_aventura_tochas_animam_no_editor() {
    let dir = temp("modelo-tochas");
    let proj = dir.join("jogo");
    nn_run::create_project(&nn_run::NewOptions {
        dir: proj.clone(),
        title: None,
        template: nn_run::Template::Aventura,
    })
    .unwrap();
    let options = StudioOptions {
        settings_path: Some(dir.join("preferencias.json")),
        open: Some(proj.clone()),
        exe: Some(fake_game(&dir)),
        check_updates: false,
    };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .wgpu()
        .build_eframe(move |cc| {
            let mut app = StudioApp::new(&cc.egui_ctx, options);
            app.set_render_state(cc.wgpu_render_state.clone());
            app
        });
    h.run_steps(4);
    h.state_mut()
        .project_mut()
        .unwrap()
        .open_file("cenas/mapa.cena", None);
    h.run_steps(4);
    assert!(!h.state().project().unwrap().scenes["cenas/mapa.cena"].is_dirty());
    h.get_all_by_label("fogo").next().unwrap().click();
    {
        // Perto da porta, para ver o fogo das tochas.
        let ws = h.state_mut().project_mut().unwrap();
        let cam = &mut ws.scenes.get_mut("cenas/mapa.cena").unwrap().camera;
        (cam.x, cam.y, cam.zoom) = (96.0, 10.0, 1.05);
    }
    // As partículas precisam de um tempo de relógio para aparecer.
    for _ in 0..8 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        h.run_steps(1);
    }
    h.get_by_label("✨ Efeito pronto");
    tela(&mut h, "modelo-tochas");
}

#[test]
fn boas_vindas_comecam_um_jogo_pronto() {
    let dir = temp("boas-vindas");
    let mut h = studio(&dir, None);
    h.get_by_label("Bem-vindo ao NN Studio!");
    tela(&mut h, "boas-vindas");
    h.get_by_label_contains("Abrir um jogo pronto").click();
    h.run();
    h.get_all_by_role(Role::TextInput).nth(1).unwrap().focus();
    selecionar_tudo(&mut h);
    digitar(&mut h, &dir.join("jogos").to_string_lossy());
    h.get_by_label("Criar projeto").click();
    h.run_steps(6);
    let p = h.state().project().expect("o projeto abriu");
    // O modelo Plataforma (cenas visuais).
    assert!(p.files.exists("cenas/fase.cena"), "modelo errado");
}

#[test]
fn aviso_quando_o_editor_fechou_por_erro() {
    let dir = temp("travou");
    std::fs::write(
        dir.join(nn_studio::crash::CRASH_FILE),
        "NN Studio 0.1.0 (linux x86_64)\nalgo deu errado\nem src/x.rs:1\n",
    )
    .unwrap();
    let mut h = studio(&dir, None);
    h.get_by_label("O NN Studio fechou por um erro");
    h.get_by_label_contains("algo deu errado");
    tela(&mut h, "travou");
    h.get_by_label("Fechar").click();
    h.run();
    assert!(h.query_by_label("O NN Studio fechou por um erro").is_none());
    // Não avisa de novo.
    drop(h);
    let mut h = studio(&dir, None);
    h.run();
    assert!(h.query_by_label("O NN Studio fechou por um erro").is_none());
}

#[test]
fn aviso_de_versao_nova_no_gerenciador() {
    let dir = temp("versao");
    std::fs::write(
        dir.join("preferencias.json"),
        r#"{ "latest_version": "9.0.0", "latest_url": "https://example.com/v9", "last_update_check": 1 }"#,
    )
    .unwrap();
    let mut h = studio(&dir, None);
    h.get_by_label_contains("Saiu o NN Studio 9.0.0");
    h.get_by_label("Ver o que mudou e baixar");
    tela(&mut h, "versao-nova");
    h.get_by_label("Agora não").click();
    h.run();
    assert!(h.query_by_label_contains("Saiu o NN Studio").is_none());
    assert_eq!(h.state().settings.skipped_version.as_deref(), Some("9.0.0"));
    // Desligado, também não aparece (nem consulta).
    h.state_mut().settings.skipped_version = None;
    h.state_mut().settings.check_updates = false;
    h.run();
    assert!(h.query_by_label_contains("Saiu o NN Studio").is_none());
}
