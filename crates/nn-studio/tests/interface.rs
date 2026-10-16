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
    let options = StudioOptions {
        settings_path: Some(dir.join("preferencias.json")),
        open,
        exe: Some(fake_game(dir)),
    };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .with_max_steps(200)
        .build_eframe(move |cc| StudioApp::new(&cc.egui_ctx, options));
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
    atalho(&mut h, Modifiers::COMMAND, Key::N);
    h.get_by_label("Nova cena");
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
    atalho(&mut h, Modifiers::COMMAND, Key::N);
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
