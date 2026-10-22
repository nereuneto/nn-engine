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

/// As boas-vindas têm um caminho 3D; os tutoriais ficam em 2D e 3D; os
/// modelos aparecem em dois grupos.
#[test]
fn gerenciador_com_caminho_3d() {
    let dir = temp("gerenciador-3d");
    let mut h = studio(&dir, None);
    // Boas-vindas: 3D troca os três cartões.
    h.get_by_label_contains("Seu primeiro jogo: uma fase de plataforma");
    h.get_by_label("3D").click();
    h.run();
    h.get_by_label_contains("Seu primeiro mundo 3D");
    h.get_by_label_contains("A ilha 3D, completa e sem código");
    tela(&mut h, "gerenciador-3d");
    // Tutoriais: na aba do 3D (escolhido nas boas-vindas), só os de 3D.
    h.get_by_label("🎓 Tutoriais").click();
    h.run();
    h.get_by_label("Uma conversa");
    assert!(h.query_by_label("Seu primeiro jogo").is_none());
    let p = h.get_by_label("Jogos 2D (6)").rect().center();
    h.hover_at(p);
    h.run();
    h.drag_at(p);
    h.run();
    h.drop_at(p);
    h.run();
    h.get_by_label("Seu primeiro jogo");
    assert!(h.query_by_label("Uma conversa").is_none());
    tela(&mut h, "tutoriais");
    h.get_by_label("Fechar").click();
    h.run();
    // "Abrir um jogo pronto" já vem com o modelo 3D (a ilha).
    h.get_by_label_contains("🎮 Abrir um jogo pronto").click();
    h.run();
    h.get_by_label("Jogos 2D");
    h.get_by_label("Jogos 3D");
    tela(&mut h, "novo-projeto-3d");
    h.get_by_label_contains("A ilha das estrelas montada no editor");
    h.get_by_label("Cancelar").click();
    h.run();
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
    // Caminhos pede para o jogo mostrar a grade dos caminhos.
    h.get_by_label("🧭 Caminhos").click();
    h.run_steps(3);
    let start = std::time::Instant::now();
    loop {
        let t = std::fs::read_to_string(&arquivo).unwrap_or_default();
        if t.contains(r#"{"cmd":"show_paths"}"#) {
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

/// Com um projeto aberto, a versão nova aparece na barra de cima; o Sobre
/// leva às novidades da versão instalada.
#[test]
fn aviso_de_versao_nova_com_projeto_aberto() {
    let dir = temp("versao-projeto");
    std::fs::write(
        dir.join("preferencias.json"),
        r#"{ "latest_version": "9.0.0", "latest_url": "https://example.com/v9", "last_update_check": 1 }"#,
    )
    .unwrap();
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj));
    h.get_by_label("🎉 Versão 9.0.0");
    tela(&mut h, "versao-nova-projeto");
    // Dispensada no gerenciador ("Agora não"), some daqui também.
    h.state_mut().settings.skipped_version = Some("9.0.0".into());
    h.run();
    assert!(h.query_by_label("🎉 Versão 9.0.0").is_none());
    // Uma versão igual ou mais velha não aparece.
    h.state_mut().settings.skipped_version = None;
    h.state_mut().settings.latest_version = Some(env!("CARGO_PKG_VERSION").into());
    h.run();
    assert!(h.query_by_label_contains("🎉 Versão").is_none());
    // Ajuda > Sobre: o link das novidades desta versão.
    paleta(&mut h, "Sobre");
    h.get_by_label(&format!(
        "Novidades da versão {}",
        env!("CARGO_PKG_VERSION")
    ));
}

// Cenas 3D ---------------------------------------------------------------

fn com_cena3d(h: &mut Harness<'_, StudioApp>, proj: &Path) {
    std::fs::write(
        proj.join("cenas/teste.cena"),
        nn_studio::files::scene3d_template(),
    )
    .unwrap();
    let ws = h.state_mut().project_mut().unwrap();
    ws.files.rescan();
    ws.open_file("cenas/teste.cena", None);
    h.run();
}

/// Onde um ponto do mundo 3D aparece na vista 3D do editor.
fn na_vista3d(h: &Harness<'_, StudioApp>, p: glam::Vec3) -> egui::Pos2 {
    let rect = h.get_by_label("Vista da cena 3D").rect();
    let s = cena(h)
        .cam3
        .camera()
        .screen_point(p, glam::Vec2::new(rect.width(), rect.height()))
        .expect("na frente da câmera");
    rect.min + egui::vec2(s.x, s.y)
}

fn clicar(h: &mut Harness<'_, StudioApp>, p: egui::Pos2) {
    h.hover_at(p);
    h.run();
    h.drag_at(p);
    h.run();
    h.drop_at(p);
    h.run();
}

#[test]
fn nova_cena_3d_pelo_dialogo() {
    let dir = temp("cena-3d-nova");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    atalho(&mut h, Modifiers::COMMAND, Key::N);
    h.get_by_label("Nova cena visual");
    selecionar_tudo(&mut h);
    digitar(&mut h, "mundo");
    h.get_by_label("3D").click();
    h.run();
    h.get_by_label_contains("Botão direito + WASD");
    tela(&mut h, "cena-3d-dialogo");
    h.get_by_label("Criar").click();
    h.run();
    let text = std::fs::read_to_string(proj.join("cenas/mundo.cena")).unwrap();
    assert!(text.contains("mode = \"3d\""), "{text}");
    // A vista é 3D e a cena já vem com câmera, chão e um cubo.
    h.get_by_label("Vista da cena 3D");
    h.get_by_label("Câmera");
    h.get_by_label("Chão");
    h.get_by_label("Cubo");
    // O botão na barra da vista e o Ambiente no Inspetor da cena.
    assert_eq!(h.get_all_by_label("☀ Ambiente").count(), 2);
    tela(&mut h, "cena-3d-nova");
}

#[test]
fn cena_3d_selecionar_apagar_adicionar_e_ambiente() {
    let dir = temp("cena-3d-editar");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    com_cena3d(&mut h, &proj);
    let cubo = objeto(&h, "Cubo").clone();

    // Clicar no cubo seleciona; o Inspetor mostra a transformação 3D.
    let p = na_vista3d(
        &h,
        glam::Vec3::new(cubo.x as f32, cubo.y as f32, cubo.z as f32),
    );
    clicar(&mut h, p);
    assert_eq!(cena(&h).selection, vec![cubo.id]);
    h.get_by_label("Forma 3D");
    h.get_by_label("Posição");
    h.get_by_label("Rotação");
    h.get_by_label("Escala");
    tela(&mut h, "cena-3d-selecionado");

    // Arrastar a seta vermelha (X) leva o cubo só em X; Ctrl+Z desfaz.
    let c = glam::Vec3::new(cubo.x as f32, cubo.y as f32, cubo.z as f32);
    let len = (c - cena(&h).cam3.pos).length() * 0.16;
    let a = na_vista3d(&h, c + glam::Vec3::X * len * 0.75);
    let b = na_vista3d(&h, c + glam::Vec3::X * len * 2.0);
    arrastar(&mut h, a, b);
    let movido = objeto(&h, "Cubo").clone();
    assert!(movido.x > cubo.x + 0.3, "x {} → {}", cubo.x, movido.x);
    assert!((movido.y - cubo.y).abs() < 1e-6 && (movido.z - cubo.z).abs() < 1e-6);
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    assert_eq!(objeto(&h, "Cubo").x, cubo.x);

    // Delete apaga, Ctrl+Z traz de volta.
    h.hover_at(p);
    h.run();
    h.key_press(Key::Delete);
    h.run();
    assert!(cena(&h).data.objects.iter().all(|o| o.name != "Cubo"));
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    assert_eq!(objeto(&h, "Cubo").id, cubo.id);

    // ➕ Objeto → Esfera: entra na cena, selecionada.
    h.get_by_label("➕ Objeto").click();
    h.run();
    h.get_by_label("Esfera").click();
    h.run();
    let esfera = objeto(&h, "Esfera").clone();
    assert_eq!(esfera.component("shape3d").unwrap().str("shape"), "sphere");
    assert_eq!(cena(&h).selection, vec![esfera.id]);

    // ☀ Ambiente: o Inspetor mostra o céu, o sol, a névoa e as sombras.
    h.get_by_label("☀ Ambiente").click();
    h.run();
    assert!(cena(&h).selection.is_empty());
    h.get_by_label("Céu:");
    h.get_by_label("Névoa:");
    h.get_by_label("Sombras:");
    tela(&mut h, "cena-3d-ambiente");

    // Ctrl+S grava a cena 3D com a esfera.
    atalho(&mut h, Modifiers::COMMAND, Key::S);
    let text = std::fs::read_to_string(proj.join("cenas/teste.cena")).unwrap();
    assert!(
        text.contains("mode = \"3d\"") && text.contains("Esfera"),
        "{text}"
    );
}

/// Com GPU, a vista 3D mostra o céu e o cubo laranja onde ele está.
#[test]
fn cena_3d_desenhada_pela_engine() {
    if std::env::var_os("NN_REQUIRE_GPU").is_none() {
        eprintln!("sem NN_REQUIRE_GPU: pulando o teste com GPU");
        return;
    }
    let dir = temp("cena-3d-gpu");
    let proj = plataforma(&dir);
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), true);
    com_cena3d(&mut h, &proj);
    h.run();
    let rect = h.get_by_label("Vista da cena 3D").rect();
    let img = h.render().expect("render");
    if let Some(d) = std::env::var_os("NN_STUDIO_TELAS") {
        let _ = img.save(PathBuf::from(d).join("cena-3d-gpu.png"));
    }
    let ppp = img.width() as f32 / 1280.0;
    let px = |p: egui::Pos2| *img.get_pixel((p.x * ppp) as u32, (p.y * ppp) as u32);
    // Lá em cima, o céu: azul.
    let ceu = px(rect.center_top() + egui::vec2(0.0, 6.0));
    assert!(ceu[2] > ceu[0] + 30, "céu {ceu:?}");
    // O cubo (#FF8C42): mais vermelho que azul.
    let cubo = objeto(&h, "Cubo").clone();
    let c = px(na_vista3d(
        &h,
        glam::Vec3::new(cubo.x as f32, cubo.y as f32, cubo.z as f32),
    ));
    assert!(c[0] > c[2] + 60, "cubo {c:?}");

    // Arrastar um .glb do painel Arquivos: vira um objeto com o modelo,
    // em pé no chão onde o mouse soltou.
    let modelos = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jogos/estrelas3d/modelos");
    std::fs::create_dir_all(proj.join("modelos")).unwrap();
    for m in ["arvore.glb", "pedra.glb"] {
        std::fs::write(
            proj.join("modelos").join(m),
            std::fs::read(modelos.join(m)).unwrap(),
        )
        .unwrap();
    }
    {
        let ws = h.state_mut().project_mut().unwrap();
        ws.files.rescan();
        ws.file_panel.reveal("modelos/arvore.glb");
    }
    h.run();
    for (m, x, z) in [("arvore.glb", -4.0, -2.0), ("pedra.glb", 3.0, 1.5)] {
        let from = h.get_by_label(m).rect().center();
        let to = na_vista3d(&h, glam::Vec3::new(x, 0.0, z));
        arrastar(&mut h, from, to);
        let name = m.trim_end_matches(".glb");
        let o = objeto(&h, name).clone();
        assert_eq!(
            o.component("model").unwrap().str("file"),
            format!("modelos/{m}")
        );
        assert!(
            (o.x - x as f64).abs() < 0.3 && (o.z - z as f64).abs() < 0.3,
            "{m}: {o:?}"
        );
        assert!(o.y > -0.01 && o.y < 0.5, "{m} em pé no chão: y = {}", o.y);
    }
    // O sol mais baixo e o cubo selecionado (a imagem da documentação).
    {
        let doc = cena_mut(&mut h);
        doc.data
            .set_env("sun_height", nn_studio::scene_data::Prop::Num(35.0));
        doc.cam3.look_at(
            glam::Vec3::new(5.5, 4.0, 7.5),
            glam::Vec3::new(-0.5, 0.6, 0.0),
        );
        doc.cam3_set = true;
    }
    h.run();
    let p = na_vista3d(
        &h,
        glam::Vec3::new(cubo.x as f32, cubo.y as f32, cubo.z as f32),
    );
    clicar(&mut h, p);
    h.hover_at(egui::pos2(1100.0, 700.0));
    h.run();
    tela(&mut h, "nn-studio-3d");
}

fn cena_mut<'a>(h: &'a mut Harness<'_, StudioApp>) -> &'a mut nn_studio::scene_view::SceneDoc {
    h.state_mut()
        .project_mut()
        .unwrap()
        .scenes
        .get_mut("cenas/teste.cena")
        .unwrap()
}

#[test]
fn cena_3d_oferece_so_componentes_e_comportamentos_do_3d() {
    let dir = temp("cena-3d-componentes");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    com_cena3d(&mut h, &proj);
    let cubo = objeto(&h, "Cubo").id;
    cena_mut(&mut h).selection = vec![cubo];
    h.run();
    h.get_by_label("➕ Adicionar componente").click();
    h.run();
    h.get_by_label("Corpo 3D");
    assert!(
        h.query_by_label("Corpo físico").is_none(),
        "o corpo 2D não vale no 3D"
    );
    assert!(h.query_by_label("Imagem (sprite)").is_none());
    h.get_by_label_contains("Jogador").click();
    h.run();
    h.get_by_label("Jogador 3D");
    assert!(h.query_by_label("Jogador de plataforma").is_none());
    tela(&mut h, "cena-3d-componentes");
    // Plataforma que anda já ganha um Corpo 3D que se move sozinho.
    h.get_by_label_contains("Movimento").click();
    h.run();
    h.get_by_label("Plataforma que anda").click();
    h.run();
    let o = objeto(&h, "Cubo").clone();
    assert_eq!(o.component("body3d").unwrap().str("body"), "kinematic");
    assert!(
        o.components
            .iter()
            .any(|c| c.str("script") == "nn:plataforma3d")
    );
    h.get_by_label("Corpo 3D");
    tela(&mut h, "cena-3d-corpo");
}

#[test]
fn tutorial_mundo_3d_avanca_sozinho_ate_o_fim() {
    use nn_studio::scene_data::{Component, Object, Prop, Rule};
    let dir = temp("tutorial-3d");
    let mut h = studio(&dir, None);
    h.state_mut().settings.projects_dir = Some(dir.join("projetos"));
    h.state_mut().start_tutorial("primeiro-mundo-3d");
    h.run_steps(4);
    let proj = dir.join("projetos/primeiro-mundo-3d");
    assert!(proj.join("cenas/mundo.cena").is_file());
    assert!(proj.join("modelos/heroi.glb").is_file());
    assert!(proj.join("modelos/bicho.glb").is_file());
    h.state_mut().tutorial.as_mut().unwrap().pause = std::time::Duration::ZERO;
    // A cena aberta é 3D.
    h.get_by_label("Vista da cena 3D");
    tela(&mut h, "tutorial-3d-1");
    for passo in 1..=2 {
        h.get_by_label("Próximo").click();
        h.run_steps(4);
        assert_eq!(h.state().tutorial.as_ref().map(|t| t.step), Some(passo));
    }
    fn doc<'a>(h: &'a mut Harness<'_, StudioApp>) -> &'a mut nn_studio::scene_view::SceneDoc {
        let ws = h.state_mut().project_mut().unwrap();
        ws.scenes.get_mut("cenas/mundo.cena").unwrap()
    }
    fn modelo(id: u32, name: &str, file: &str) -> Object {
        let mut o = Object::new(id, name);
        let mut c = Component::new("model");
        c.set("file", Prop::Str(file.into()));
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
    let corpo = |kind: &str| {
        let mut c = Component::new("body3d");
        c.set("body", Prop::Str(kind.into()));
        c
    };
    // O botão direito no chão > Cubo (como o menu faz).
    let mut cubo = Object::new(9, "Cubo");
    let mut forma = Component::new("shape3d");
    forma.set("shape", Prop::Str("cube".into()));
    cubo.components.push(forma);
    doc(&mut h).data.objects.push(cubo);
    espera(&mut h, 3);
    doc(&mut h)
        .data
        .objects
        .push(modelo(10, "heroi", "modelos/heroi.glb"));
    espera(&mut h, 4);
    let n = doc(&mut h).data.objects.len() - 1;
    doc(&mut h).data.objects[n]
        .components
        .push(Component::behavior("nn:jogador3d", &[]));
    espera(&mut h, 5);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 6);
    let mut arvore = modelo(11, "arvore", "modelos/arvore.glb");
    arvore.components.push(corpo("static"));
    doc(&mut h).data.objects.push(arvore.clone());
    espera(&mut h, 7);
    // A fileira (Alt+arrastar e Ctrl+D, como a cena faz).
    for id in 30..33 {
        let mut a = arvore.clone();
        a.id = id;
        doc(&mut h).data.objects.push(a);
    }
    espera(&mut h, 8);
    let mut caixote = modelo(12, "caixote", "modelos/caixote.glb");
    caixote.components.push(corpo("dynamic"));
    doc(&mut h).data.objects.push(caixote);
    espera(&mut h, 9);
    let mut bicho = modelo(15, "bicho", "modelos/bicho.glb");
    let mut anim = Component::new("anim3d");
    anim.set("animation", Prop::Str("andar".into()));
    bicho.components.push(anim);
    doc(&mut h).data.objects.push(bicho);
    espera(&mut h, 10);
    let mut estrela = modelo(13, "estrela", "modelos/estrela.glb");
    estrela
        .components
        .push(Component::behavior("nn:coletavel", &[]));
    doc(&mut h).data.objects.push(estrela);
    espera(&mut h, 11);
    let mut plat = modelo(14, "plataforma", "modelos/plataforma.glb");
    plat.components.push(corpo("kinematic"));
    plat.components
        .push(Component::behavior("nn:plataforma3d", &[]));
    doc(&mut h).data.objects.push(plat);
    espera(&mut h, 12);
    // O passo do Ambiente: o sol mais baixo.
    doc(&mut h).data.set_env("sun_height", Prop::Num(15.0));
    espera(&mut h, 13);
    assert!(tabs(&h).contains(&Tab::Events));
    let cat = nn_studio::events::catalog();
    let mut acabou = cat.condition("acabaram").unwrap().new_item(false);
    acabou.set("nome", Prop::Str("estrela".into()));
    doc(&mut h).data.events.push(Rule {
        when: vec![acabou],
        actions: vec![cat.action("reiniciar").unwrap().new_item(false)],
        ..Default::default()
    });
    espera(&mut h, 14);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 15);
    let text = std::fs::read_to_string(proj.join("cenas/mundo.cena")).unwrap();
    assert!(
        text.contains("nn:jogador3d") && text.contains("sun_height = 15"),
        "{text}"
    );
    tela(&mut h, "tutorial-3d-fim");
    h.get_by_label("Concluir").click();
    h.run_steps(4);
    assert!(h.state().tutorial.is_none());
}

/// Com GPU: a vista 3D anima o modelo com a Animação 3D (a prévia), e o
/// Inspetor oferece as animações do modelo.
#[test]
fn cena_3d_previa_da_animacao_e_inspetor() {
    use nn_studio::scene_data::{Component, Object, Prop};
    if std::env::var_os("NN_REQUIRE_GPU").is_none() {
        eprintln!("sem NN_REQUIRE_GPU: pulando o teste com GPU");
        return;
    }
    let dir = temp("cena-3d-animacao");
    let proj = plataforma(&dir);
    let modelos = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jogos/estrelas3d/modelos");
    std::fs::create_dir_all(proj.join("modelos")).unwrap();
    std::fs::write(
        proj.join("modelos/heroi.glb"),
        std::fs::read(modelos.join("heroi.glb")).unwrap(),
    )
    .unwrap();
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), true);
    com_cena3d(&mut h, &proj);
    // (Com animação na cena, a vista se redesenha sem parar: passos contados.)
    {
        let doc = cena_mut(&mut h);
        let mut o = Object::new(20, "Herói");
        let mut m = Component::new("model");
        m.set("file", Prop::Str("modelos/heroi.glb".into()));
        o.components.push(m);
        let mut a = Component::new("anim3d");
        a.set("animation", Prop::Str("correr".into()));
        o.components.push(a);
        doc.data.objects.push(o);
        doc.selection = vec![20];
        doc.cam3.look_at(
            glam::Vec3::new(0.0, 1.0, 3.0),
            glam::Vec3::new(0.0, 0.9, 0.0),
        );
        doc.cam3_set = true;
    }
    h.run_steps(4);
    // O Inspetor mostra a Animação 3D com as animações do modelo no ☰.
    h.get_by_label("Animação 3D");
    let rect = h.get_by_label("Vista da cena 3D").rect();
    let a = h.render().expect("render");
    // Um pouco depois, a pose é outra (a vista se redesenha sozinha).
    std::thread::sleep(std::time::Duration::from_millis(120));
    h.step();
    let b = h.render().expect("render");
    tela(&mut h, "cena-3d-animacao");
    let ppp = a.width() as f32 / 1280.0;
    let mut diff = 0;
    for y in (rect.top() as u32)..(rect.bottom() as u32) {
        for x in (rect.left() as u32)..(rect.right() as u32) {
            let (px, py) = ((x as f32 * ppp) as u32, (y as f32 * ppp) as u32);
            if a.get_pixel(px, py) != b.get_pixel(px, py) {
                diff += 1;
            }
        }
    }
    assert!(diff > 30, "a prévia animou: {diff} pixels");
}

/// Uma espada como filha do herói: "Preso a um osso" já vem com a mão
/// direita, o inspetor lista os ossos do modelo do pai e, no editor, a
/// espada fica na mão (na pose de descanso).
#[test]
fn cena_3d_espada_presa_na_mao() {
    use nn_studio::scene_data::{Component, Object, Prop};
    let dir = temp("cena-3d-espada");
    let proj = plataforma(&dir);
    let modelos = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jogos/estrelas3d/modelos");
    std::fs::create_dir_all(proj.join("modelos")).unwrap();
    for m in ["heroi.glb", "espada.glb"] {
        std::fs::copy(modelos.join(m), proj.join("modelos").join(m)).unwrap();
    }
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), true);
    com_cena3d(&mut h, &proj);
    {
        let doc = cena_mut(&mut h);
        let mut o = Object::new(20, "Herói");
        o.ry = 90.0;
        let mut m = Component::new("model");
        m.set("file", Prop::Str("modelos/heroi.glb".into()));
        o.components.push(m);
        let mut espada = Object::new(21, "Espada");
        let mut m = Component::new("model");
        m.set("file", Prop::Str("modelos/espada.glb".into()));
        espada.components.push(m);
        o.children.push(espada);
        doc.data.objects.push(o);
        doc.selection = vec![21];
        doc.cam3.look_at(
            glam::Vec3::new(2.5, 1.4, 2.5),
            glam::Vec3::new(0.0, 0.8, 0.0),
        );
        doc.cam3_set = true;
    }
    h.run_steps(3);
    // Sem o componente, a espada fica nos pés do herói.
    let p = cena_mut(&mut h)
        .data
        .world_matrix3(21)
        .unwrap()
        .transform_point3(glam::Vec3::ZERO);
    assert!(p.length() < 1e-4, "{p}");
    h.get_by_label("➕ Adicionar componente").click();
    h.run_steps(2);
    h.get_by_label("Preso a um osso").click();
    h.run_steps(3);
    let doc = cena_mut(&mut h);
    let o = doc.data.find(21).unwrap();
    assert_eq!(o.component("bone").unwrap().str("bone"), "mao_d");
    // Na mão direita: o -X do herói, virado 90° para +X, vira +Z.
    let p = doc
        .data
        .world_matrix3(21)
        .unwrap()
        .transform_point3(glam::Vec3::ZERO);
    assert!(p.abs_diff_eq(glam::Vec3::new(0.0, 0.53, 0.41), 1e-3), "{p}");
    tela(&mut h, "cena-3d-espada");
}

/// Partículas e som numa cena 3D: os efeitos prontos do 3D, os campos em
/// metros (sem a direção do 2D), o som no espaço 3D e a prévia animando.
#[test]
fn cena_3d_particulas_e_som() {
    use nn_studio::scene_data::{Component, Prop};
    let gpu = std::env::var_os("NN_REQUIRE_GPU").is_some();
    let dir = temp("cena-3d-efeitos");
    let proj = plataforma(&dir);
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), gpu);
    com_cena3d(&mut h, &proj);
    // ➕ Objeto > Partículas > Fogo (o efeito do 3D, em metros).
    h.get_by_label("➕ Objeto").click();
    h.run_steps(2);
    h.get_by(|n| n.label().is_some_and(|l| l.starts_with("Partículas")))
        .hover();
    h.run_steps(3);
    h.get_by(|n| n.role() == Role::Button && n.label().as_deref() == Some("Fogo"))
        .click();
    h.run_steps(4);
    let fogo = objeto(&h, "fogo").clone();
    let c = fogo.component("particles").unwrap();
    assert!(
        c.num("size_start") < 2.0,
        "em metros: {}",
        c.num("size_start")
    );
    assert_eq!(cena_mut(&mut h).selection, vec![fogo.id]);
    // O Inspetor: sem a direção do 2D, com o fundo da área.
    h.get_by_label("Partículas");
    assert!(h.query_by_label("Direção").is_none());
    h.get_by_label_contains("Efeito pronto");
    // Um som no fogo: "no espaço 3D" e a distância.
    {
        let doc = cena_mut(&mut h);
        let o = doc.data.find_mut(fogo.id).unwrap();
        let mut s = Component::new("sound");
        s.set("sound", Prop::Str("sons/pulo.wav".into()));
        o.components.push(s);
    }
    h.run_steps(4);
    h.get_by_label("Som no espaço 3D");
    h.get_by_label("Ouvido até");
    tela(&mut h, "cena-3d-efeitos");
    if !gpu {
        return;
    }
    // A prévia anima: duas imagens seguidas são diferentes.
    let rect = h.get_by_label("Vista da cena 3D").rect();
    let a = h.render().expect("render");
    std::thread::sleep(std::time::Duration::from_millis(150));
    h.step();
    let b = h.render().expect("render");
    let ppp = a.width() as f32 / 1280.0;
    let mut diff = 0;
    for y in (rect.top() as u32)..(rect.bottom() as u32) {
        for x in (rect.left() as u32)..(rect.right() as u32) {
            let (px, py) = ((x as f32 * ppp) as u32, (y as f32 * ppp) as u32);
            if a.get_pixel(px, py) != b.get_pixel(px, py) {
                diff += 1;
            }
        }
    }
    assert!(diff > 20, "o fogo anima na vista: {diff} pixels");
}

#[test]
fn tutorial_efeitos_3d_avanca_sozinho_ate_o_fim() {
    use nn_studio::scene_data::{Component, Object, Prop, Rule};
    let dir = temp("tutorial-efeitos-3d");
    let mut h = studio(&dir, None);
    h.state_mut().settings.projects_dir = Some(dir.join("projetos"));
    h.state_mut().start_tutorial("efeitos-3d");
    h.run_steps(4);
    let proj = dir.join("projetos/efeitos-3d");
    assert!(proj.join("assets/sons/fogueira.wav").is_file());
    h.state_mut().tutorial.as_mut().unwrap().pause = std::time::Duration::ZERO;
    h.get_by_label("Próximo").click();
    h.run_steps(4);
    fn doc<'a>(h: &'a mut Harness<'_, StudioApp>) -> &'a mut nn_studio::scene_view::SceneDoc {
        let ws = h.state_mut().project_mut().unwrap();
        ws.scenes.get_mut("cenas/mundo.cena").unwrap()
    }
    fn espera(h: &mut Harness<'_, StudioApp>, esperado: usize) {
        h.run_steps(6);
        assert_eq!(
            h.state().tutorial.as_ref().map(|t| t.step),
            Some(esperado),
            "o tutorial devia estar no passo {esperado}"
        );
    }
    let efeito = |id: u32, name: &str, preset: &str| {
        let mut o = Object::new(id, name);
        o.components.push(
            nn_studio::particle_presets::find_in(preset, true)
                .unwrap()
                .component(),
        );
        o
    };
    espera(&mut h, 1);
    doc(&mut h).data.objects.push(efeito(10, "fogo", "fogo"));
    espera(&mut h, 2);
    let mut fumaca = efeito(11, "fumaca", "fumaca");
    fumaca.y = 1.5;
    doc(&mut h).data.objects.push(fumaca);
    espera(&mut h, 3);
    let mut som = Object::new(12, "fogueira");
    let mut c = Component::new("sound");
    c.set("sound", Prop::Str("assets/sons/fogueira.wav".into()));
    c.set("loop", Prop::Bool(true));
    som.components.push(c);
    doc(&mut h).data.objects.push(som);
    espera(&mut h, 4);
    let mut jogador = Object::new(13, "Jogador");
    jogador
        .components
        .push(Component::behavior("nn:jogador3d", &[]));
    doc(&mut h).data.objects.push(jogador);
    espera(&mut h, 5);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 6);
    let n = doc(&mut h).data.objects.len() - 1;
    doc(&mut h).data.objects[n]
        .children
        .push(efeito(14, "poeira", "poeira"));
    espera(&mut h, 7);
    let cat = nn_studio::events::catalog();
    let mut msg = cat.condition("mensagem").unwrap().new_item(true);
    msg.set("msg", Prop::Str("pulou".into()));
    let mut soltar = cat.action("soltar_particulas").unwrap().new_item(true);
    soltar.set("alvo", Prop::Str("poeira".into()));
    doc(&mut h).data.objects[n].events.push(Rule {
        when: vec![msg],
        actions: vec![soltar],
        ..Default::default()
    });
    espera(&mut h, 8);
    let mut chuva = efeito(15, "chuva", "chuva");
    chuva.y = 10.0;
    doc(&mut h).data.objects.push(chuva);
    espera(&mut h, 9);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 10);
    let text = std::fs::read_to_string(proj.join("cenas/mundo.cena")).unwrap();
    assert!(
        text.contains("spatial = true") && text.contains("soltar_particulas"),
        "{text}"
    );
    tela(&mut h, "tutorial-efeitos-3d-fim");
    h.get_by_label("Concluir").click();
    h.run_steps(4);
    assert!(h.state().tutorial.is_none());
}

/// Terreno: ➕ Objeto > Terreno, ✏ Pintar terreno, subir com o mouse,
/// desfazer, pintar uma cor, espalhar árvores e a água.
#[test]
fn cena_3d_terreno_pintado_com_o_mouse() {
    use nn_studio::scene_data::Prop;
    let gpu = std::env::var_os("NN_REQUIRE_GPU").is_some();
    let dir = temp("cena-3d-terreno");
    let proj = plataforma(&dir);
    let modelos = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../jogos/estrelas3d/modelos");
    std::fs::create_dir_all(proj.join("modelos")).unwrap();
    std::fs::write(
        proj.join("modelos/arvore.glb"),
        std::fs::read(modelos.join("arvore.glb")).unwrap(),
    )
    .unwrap();
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), gpu);
    com_cena3d(&mut h, &proj);
    // Tira o chão e o cubo do modelo: só o terreno.
    cena_mut(&mut h)
        .data
        .objects
        .retain(|o| o.component("camera3d").is_some());
    h.get_by_label("➕ Objeto").click();
    h.run_steps(2);
    h.get_by_label("Terreno").click();
    h.run_steps(3);
    let id = objeto(&h, "Terreno").id;
    {
        let doc = cena_mut(&mut h);
        doc.selection = vec![id];
        // De cima, olhando o meio do terreno.
        doc.cam3
            .look_at(glam::Vec3::new(0.0, 40.0, 25.0), glam::Vec3::ZERO);
        doc.cam3_set = true;
    }
    h.run_steps(3);
    // O Inspetor mostra o terreno; a barra, o 🖌.
    h.get_by_label("Nível da água");
    h.get_by_label("✏ Pintar terreno").click();
    h.run_steps(3);
    h.get_by_label("⬆ Subir");
    let altura = |h: &Harness<'_, StudioApp>| -> f32 {
        let o = objeto(h, "Terreno");
        let t = nn_studio::terrain_tool::terrain_of(o.component("terrain").unwrap());
        t.height_at(0.0, 0.0).unwrap()
    };
    assert_eq!(altura(&h), 0.0);
    // Segura o botão no meio da vista: o chão sobe.
    let rect = h.get_by_label("Vista da cena 3D").rect();
    let meio = rect.center();
    h.hover_at(meio);
    h.run_steps(2);
    h.drag_at(meio);
    for _ in 0..20 {
        h.hover_at(meio);
        h.step();
    }
    h.drop_at(meio);
    h.run_steps(3);
    let subiu = altura(&h);
    assert!(subiu > 0.3, "subiu: {subiu}");
    // A seleção não mudou (o clique pintou, não selecionou).
    assert_eq!(cena_mut(&mut h).selection, vec![id]);
    tela(&mut h, "cena-3d-terreno");
    // Ctrl+Z desfaz o arrasto inteiro.
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    h.run_steps(2);
    assert_eq!(altura(&h), 0.0);
    // Pintar a cor 4 (areia).
    h.get_by_label("Pintar cor").click();
    h.run_steps(2);
    h.get_by_label_contains("4 Areia").click();
    h.run_steps(2);
    h.drag_at(meio);
    for _ in 0..10 {
        h.hover_at(meio);
        h.step();
    }
    h.drop_at(meio);
    h.run_steps(3);
    {
        let o = objeto(&h, "Terreno");
        let t = nn_studio::terrain_tool::terrain_of(o.component("terrain").unwrap());
        let k = (t.side() / 2 * t.side() + t.side() / 2) as usize;
        assert!(t.layers[k][3] > 200, "areia: {:?}", t.layers[k]);
    }
    // Espalhar árvores.
    {
        let doc = cena_mut(&mut h);
        doc.terrain_tool.brush = nn_studio::terrain_tool::TerrainBrush::Scatter;
        doc.terrain_tool.scatter = "modelos/arvore.glb".into();
        doc.terrain_tool.radius = 8.0;
    }
    h.drag_at(meio);
    for _ in 0..30 {
        h.hover_at(meio);
        h.step();
    }
    h.drop_at(meio);
    h.run_steps(3);
    let arvores = cena_mut(&mut h)
        .data
        .objects
        .iter()
        .filter(|o| o.name == "arvore")
        .count();
    assert!(arvores >= 2, "espalhou: {arvores}");
    // Água ligada.
    {
        let doc = cena_mut(&mut h);
        let o = doc.data.find_mut(id).unwrap();
        o.components[0].set("water", Prop::Bool(true));
        o.components[0].set("water_level", Prop::Num(0.5));
    }
    h.run_steps(3);
    tela(&mut h, "cena-3d-terreno-agua");
    // Salva: a cena tem o terreno com os dados. (Com a água, a vista
    // anima sem parar: passos contados, não `run`.)
    h.key_press_modifiers(Modifiers::COMMAND, Key::S);
    h.run_steps(3);
    let text = std::fs::read_to_string(proj.join("cenas/teste.cena")).unwrap();
    assert!(
        text.contains("type = \"terrain\"") && text.contains("data = \"t1:"),
        "salvou"
    );
}

#[test]
fn tutorial_pinte_um_terreno_avanca_sozinho_ate_o_fim() {
    use nn_studio::scene_data::{Component, Object, Prop};
    use nn_studio::terrain_tool::{TerrainBrush, terrain_of};
    let dir = temp("tutorial-terreno");
    let mut h = studio(&dir, None);
    h.state_mut().settings.projects_dir = Some(dir.join("projetos"));
    h.state_mut().start_tutorial("pinte-um-terreno");
    h.run_steps(4);
    h.state_mut().tutorial.as_mut().unwrap().pause = std::time::Duration::ZERO;
    h.get_by_label("Próximo").click();
    h.run_steps(4);
    fn doc<'a>(h: &'a mut Harness<'_, StudioApp>) -> &'a mut nn_studio::scene_view::SceneDoc {
        let ws = h.state_mut().project_mut().unwrap();
        ws.scenes.get_mut("cenas/mundo.cena").unwrap()
    }
    fn espera(h: &mut Harness<'_, StudioApp>, esperado: usize) {
        h.run_steps(6);
        assert_eq!(
            h.state().tutorial.as_ref().map(|t| t.step),
            Some(esperado),
            "o tutorial devia estar no passo {esperado}"
        );
    }
    // Mexe no terreno como os pincéis fazem.
    fn terreno(h: &mut Harness<'_, StudioApp>, f: impl FnOnce(&mut nn_render::d3::TerrainData)) {
        let d = doc(h);
        let o = d
            .data
            .objects
            .iter_mut()
            .find(|o| o.name == "Terreno")
            .unwrap();
        let c = o
            .components
            .iter_mut()
            .find(|c| c.kind == "terrain")
            .unwrap();
        let mut t = terrain_of(c);
        f(&mut t);
        c.set("data", Prop::Str(t.encode()));
    }
    espera(&mut h, 1);
    let mut t = Object::new(20, "Terreno");
    t.components.push(Component::new("terrain"));
    doc(&mut h).data.objects.push(t);
    espera(&mut h, 2);
    doc(&mut h).selection = vec![20];
    doc(&mut h).terrain_tool.active = true;
    espera(&mut h, 3);
    use glam::Vec2;
    use nn_render::d3::BrushOp;
    terreno(&mut h, |t| {
        for _ in 0..10 {
            t.brush(BrushOp::Raise, Vec2::ZERO, 6.0, 0.5);
        }
    });
    espera(&mut h, 4);
    terreno(&mut h, |t| {
        for _ in 0..8 {
            t.brush(BrushOp::Lower, Vec2::new(15.0, 15.0), 4.0, 0.5);
        }
    });
    espera(&mut h, 5);
    {
        let d = doc(&mut h);
        d.terrain_tool.brush = TerrainBrush::Smooth;
        d.terrain_tool.stroke = 3;
    }
    espera(&mut h, 6);
    terreno(&mut h, |t| {
        t.paint(2, Vec2::ZERO, 3.0, 1.0);
    });
    espera(&mut h, 7);
    {
        let d = doc(&mut h);
        let o = d.data.find_mut(20).unwrap();
        o.components[0].set("water", Prop::Bool(true));
    }
    espera(&mut h, 8);
    for i in 0..5 {
        let mut a = Object::new(30 + i, "arvore");
        let mut m = Component::new("model");
        m.set("file", Prop::Str("modelos/arvore.glb".into()));
        a.components.push(m);
        doc(&mut h).data.objects.push(a);
    }
    espera(&mut h, 9);
    let mut j = Object::new(40, "Jogador");
    j.components.push(Component::behavior("nn:jogador3d", &[]));
    doc(&mut h).data.objects.push(j);
    espera(&mut h, 10);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 11);
    tela(&mut h, "tutorial-terreno-fim");
    h.get_by_label("Concluir").click();
    h.run_steps(4);
    assert!(h.state().tutorial.is_none());
}

#[test]
fn tutorial_inimigos_3d_avanca_sozinho_ate_o_fim() {
    use nn_studio::scene_data::{Component, Object, Prop};
    let dir = temp("tutorial-inimigos-3d");
    let mut h = studio(&dir, None);
    h.state_mut().settings.projects_dir = Some(dir.join("projetos"));
    h.state_mut().start_tutorial("inimigos-3d");
    h.run_steps(4);
    assert!(
        dir.join("projetos/inimigos-3d/assets/imagens/coracao.png")
            .is_file()
    );
    h.state_mut().tutorial.as_mut().unwrap().pause = std::time::Duration::ZERO;
    h.get_by_label("Próximo").click();
    h.run_steps(4);
    fn doc<'a>(h: &'a mut Harness<'_, StudioApp>) -> &'a mut nn_studio::scene_view::SceneDoc {
        let ws = h.state_mut().project_mut().unwrap();
        ws.scenes.get_mut("cenas/mundo.cena").unwrap()
    }
    fn espera(h: &mut Harness<'_, StudioApp>, esperado: usize) {
        h.run_steps(6);
        assert_eq!(
            h.state().tutorial.as_ref().map(|t| t.step),
            Some(esperado),
            "o tutorial devia estar no passo {esperado}"
        );
    }
    let modelo = |id: u32, name: &str, file: &str| {
        let mut o = Object::new(id, name);
        let mut c = Component::new("model");
        c.set("file", Prop::Str(file.into()));
        o.components.push(c);
        o
    };
    espera(&mut h, 1);
    let mut heroi = modelo(10, "heroi", "modelos/heroi.glb");
    heroi
        .components
        .push(Component::behavior("nn:jogador3d", &[]));
    doc(&mut h).data.objects.push(heroi);
    espera(&mut h, 2);
    let n = doc(&mut h).data.objects.len() - 1;
    doc(&mut h).data.objects[n]
        .components
        .push(Component::behavior("nn:vida", &[]));
    espera(&mut h, 3);
    let mut hud = Object::new(11, "coracoes");
    let mut icons = Component::new("icons");
    icons.set("var", Prop::Str("vidas".into()));
    hud.components.push(icons);
    doc(&mut h).data.objects.push(hud);
    espera(&mut h, 4);
    let mut bicho = modelo(12, "bicho", "modelos/bicho.glb");
    bicho
        .components
        .push(Component::behavior("nn:inimigo3d", &[]));
    doc(&mut h).data.objects.push(bicho);
    espera(&mut h, 5);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 6);
    let n = doc(&mut h).data.objects.len() - 1;
    doc(&mut h).data.objects[n]
        .components
        .push(Component::behavior(
            "nn:vida",
            &[
                ("ao_acabar".into(), Prop::Str("destruir".into())),
                ("barra".into(), Prop::Bool(true)),
            ],
        ));
    espera(&mut h, 7);
    let mut lava = Object::new(13, "lava");
    lava.components.push(Component::new("shape3d"));
    lava.components.push(Component::behavior("nn:perigo", &[]));
    doc(&mut h).data.objects.push(lava);
    espera(&mut h, 8);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 9);
    h.get_by_label("Concluir").click();
    h.run_steps(4);
    assert!(h.state().tutorial.is_none());
}

#[test]
fn tutorial_um_chefe_avanca_sozinho_ate_o_fim() {
    use nn_studio::scene_data::{Component, EventItem, Object, Prop, Rule};
    let dir = temp("tutorial-chefe");
    let mut h = studio(&dir, None);
    h.state_mut().settings.projects_dir = Some(dir.join("projetos"));
    h.state_mut().start_tutorial("um-chefe");
    h.run_steps(4);
    let projeto = dir.join("projetos/um-chefe");
    for f in [
        "modelos/besourao.glb",
        "modelos/espada.glb",
        "modelos/bicho.glb",
    ] {
        assert!(projeto.join(f).is_file(), "o modelo vazio3d traz {f}");
    }
    h.state_mut().tutorial.as_mut().unwrap().pause = std::time::Duration::ZERO;
    h.get_by_label("Próximo").click();
    h.run_steps(4);
    fn doc<'a>(h: &'a mut Harness<'_, StudioApp>) -> &'a mut nn_studio::scene_view::SceneDoc {
        let ws = h.state_mut().project_mut().unwrap();
        ws.scenes.get_mut("cenas/mundo.cena").unwrap()
    }
    fn espera(h: &mut Harness<'_, StudioApp>, esperado: usize) {
        h.run_steps(6);
        assert_eq!(
            h.state().tutorial.as_ref().map(|t| t.step),
            Some(esperado),
            "o tutorial devia estar no passo {esperado}"
        );
    }
    let modelo = |id: u32, name: &str, file: &str| {
        let mut o = Object::new(id, name);
        let mut c = Component::new("model");
        c.set("file", Prop::Str(file.into()));
        o.components.push(c);
        o
    };
    espera(&mut h, 1);
    let mut heroi = modelo(10, "heroi", "modelos/heroi.glb");
    heroi
        .components
        .push(Component::behavior("nn:jogador3d", &[]));
    heroi.components.push(Component::behavior("nn:vida", &[]));
    doc(&mut h).data.objects.push(heroi);
    espera(&mut h, 2);
    // A espada, filha do herói, presa à mão.
    let mut espada = modelo(11, "espada", "modelos/espada.glb");
    let mut osso = Component::new("bone");
    osso.set("bone", Prop::Str("mao_d".into()));
    espada.components.push(osso);
    doc(&mut h).data.find_mut(10).unwrap().children.push(espada);
    espera(&mut h, 3);
    doc(&mut h)
        .data
        .find_mut(10)
        .unwrap()
        .components
        .push(Component::behavior("nn:golpe", &[]));
    espera(&mut h, 4);
    let mut bicho = modelo(12, "besouro", "modelos/bicho.glb");
    bicho
        .components
        .push(Component::behavior("nn:inimigo3d", &[]));
    bicho.components.push(Component::behavior("nn:vida", &[]));
    doc(&mut h).data.objects.push(bicho);
    espera(&mut h, 5);
    // O prefab (como o 📦 Criar prefab faz: um arquivo em prefabs/).
    std::fs::create_dir_all(projeto.join("prefabs")).unwrap();
    std::fs::write(
        projeto.join("prefabs/besouro.prefab"),
        r#"return { format = 1, object = { id = 1, name = "besouro", components = {
  { type = "model", file = "modelos/bicho.glb" }, { type = "behavior", script = "nn:inimigo3d" } } } }"#,
    )
    .unwrap();
    h.state_mut().project_mut().unwrap().files.rescan();
    espera(&mut h, 6);
    let mut ondas = Object::new(13, "Ondas");
    ondas.components.push(Component::behavior(
        "nn:ondas",
        &[("prefab".into(), Prop::Str("prefabs/besouro.prefab".into()))],
    ));
    doc(&mut h).data.objects.push(ondas);
    espera(&mut h, 7);
    let mut placar = Object::new(14, "placar");
    placar.components.push(Component::behavior(
        "nn:mostrar_valor",
        &[(
            "texto".into(),
            Prop::Str("Onda {onda}   Faltam {restantes}".into()),
        )],
    ));
    doc(&mut h).data.objects.push(placar);
    espera(&mut h, 8);
    let mut rei = modelo(15, "rei", "modelos/besourao.glb");
    rei.components.push(Component::behavior("nn:vida", &[]));
    rei.components.push(Component::behavior("nn:chefe", &[]));
    doc(&mut h).data.objects.push(rei);
    espera(&mut h, 9);
    let item = |id: &str, props: &[(&str, &str)]| EventItem {
        id: id.into(),
        not: false,
        props: props
            .iter()
            .map(|(k, v)| (k.to_string(), Prop::Str(v.to_string())))
            .collect(),
    };
    doc(&mut h).data.events.push(Rule {
        when: vec![item("ondas_terminaram", &[("quem", "Ondas")])],
        actions: vec![item("mensagem", &[("msg", "comecar"), ("alvo", "rei")])],
        ..Default::default()
    });
    espera(&mut h, 10);
    doc(&mut h).data.events.push(Rule {
        when: vec![item("vida_acabou", &[("quem", "rei")])],
        actions: vec![item("som", &[("som", "assets/sons/vitoria.wav")])],
        ..Default::default()
    });
    espera(&mut h, 11);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 12);
    h.get_by_label("Concluir").click();
    h.run_steps(4);
    assert!(h.state().tutorial.is_none());
}

#[test]
fn tutorial_portas_e_chaves_avanca_sozinho_ate_o_fim() {
    use nn_studio::scene_data::{Component, Prop};
    let dir = temp("tutorial-portas");
    let mut h = studio(&dir, None);
    h.state_mut().settings.projects_dir = Some(dir.join("projetos"));
    h.state_mut().start_tutorial("portas-e-chaves");
    h.run_steps(4);
    let projeto = dir.join("projetos/portas-e-chaves");
    for f in [
        "modelos/porta.glb",
        "modelos/chave.glb",
        "assets/imagens/chave.png",
    ] {
        assert!(projeto.join(f).is_file(), "o modelo vazio3d traz {f}");
    }
    h.state_mut().tutorial.as_mut().unwrap().pause = std::time::Duration::ZERO;
    h.get_by_label("Próximo").click();
    h.run_steps(4);
    const MUNDO: &str = "cenas/mundo.cena";
    fn doc<'a>(h: &'a mut Harness<'_, StudioApp>) -> &'a mut nn_studio::scene_view::SceneDoc {
        let ws = h.state_mut().project_mut().unwrap();
        ws.scenes.get_mut(MUNDO).unwrap()
    }
    fn espera(h: &mut Harness<'_, StudioApp>, esperado: usize) {
        h.run_steps(6);
        assert_eq!(
            h.state().tutorial.as_ref().map(|t| t.step),
            Some(esperado),
            "o tutorial devia estar no passo {esperado}"
        );
    }
    // ➕ Objeto > Portas e itens > (item).
    let novo = |h: &mut Harness<'_, StudioApp>, nome: &str| {
        h.get_by_label("➕ Objeto").click();
        h.run_steps(2);
        h.get_by(|n| n.label().is_some_and(|l| l.starts_with("Portas e itens")))
            .hover();
        h.run_steps(3);
        h.get_by(|n| n.role() == Role::Button && n.label().as_deref() == Some(nome))
            .click();
        h.run_steps(2);
    };
    let ultimo = |h: &mut Harness<'_, StudioApp>| {
        let d = doc(h);
        let id = d.selection[0];
        d.data.find(id).unwrap().clone()
    };
    espera(&mut h, 1);
    h.get_by_label("➕ Objeto").click();
    h.run_steps(2);
    h.get_by(|n| n.role() == Role::Button && n.label().as_deref() == Some("Jogador 3D"))
        .click();
    espera(&mut h, 2);
    novo(&mut h, "Porta");
    let porta = ultimo(&mut h);
    assert_eq!(porta.name, "Porta");
    assert!((porta.y - 1.1).abs() < 0.01, "em pé no chão: {}", porta.y);
    let body = porta.component("body3d").expect("a porta vem com corpo");
    assert_eq!(body.str("body"), "kinematic");
    espera(&mut h, 3);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 4);
    {
        let d = doc(&mut h);
        let o = d.data.find_mut(porta.id).unwrap();
        let c = o
            .components
            .iter_mut()
            .find(|c| c.str("script") == "nn:porta3d")
            .unwrap();
        c.set("chave", Prop::Str("chave".into()));
    }
    espera(&mut h, 5);
    novo(&mut h, "Chave");
    let chave = ultimo(&mut h);
    let col = chave
        .components
        .iter()
        .find(|c| c.str("script") == "nn:coletavel")
        .unwrap();
    assert_eq!(col.str("item"), "chave");
    assert_eq!(col.str("variavel"), "");
    espera(&mut h, 6);
    novo(&mut h, "Mostrar inventário");
    espera(&mut h, 7);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 8);
    novo(&mut h, "Texto 3D");
    let texto = ultimo(&mut h);
    assert_eq!(texto.component("text3d").unwrap().str("text"), "Olá!");
    espera(&mut h, 9);
    novo(&mut h, "Zona de gatilho");
    let zona = ultimo(&mut h);
    {
        let d = doc(&mut h);
        let o = d.data.find_mut(zona.id).unwrap();
        let c = o
            .components
            .iter_mut()
            .find(|c| c.str("script") == "nn:gatilho")
            .unwrap();
        c.set("alvo", Prop::Str("Porta".into()));
        c.set("mensagem", Prop::Str("fechar".into()));
    }
    espera(&mut h, 10);
    {
        // Cada um no seu lugar, para a foto.
        let d = doc(&mut h);
        for (nome, x, y, z, ry) in [
            ("Porta", 0.0, 1.1, 0.0, 0.0),
            ("Chave", 2.5, 0.5, 1.5, 0.0),
            ("Texto", 0.0, 2.7, 0.1, 0.0),
            ("Zona", 0.0, 1.0, -2.2, 0.0),
            ("Jogador", -1.5, 1.0, 2.5, 0.0),
        ] {
            let id = d.data.objects.iter().find(|o| o.name == nome).unwrap().id;
            let o = d.data.find_mut(id).unwrap();
            (o.x, o.y, o.z, o.ry) = (x, y, z, ry);
        }
        let id = d
            .data
            .objects
            .iter()
            .find(|o| o.name == "Texto")
            .unwrap()
            .id;
        let o = d.data.find_mut(id).unwrap();
        o.components[0].set("text", Prop::Str("Cuidado!".into()));
        o.components[0].set("facing", Prop::Str("placa".into()));
        o.components[0].set("background", Prop::Str("#F2D49B".into()));
        o.components[0].set("color", Prop::Str("#3A2E1F".into()));
        o.components[0].set("outline", Prop::Str(String::new()));
        d.cam3.look_at(
            glam::Vec3::new(4.5, 3.5, 6.5),
            glam::Vec3::new(0.0, 1.0, 0.0),
        );
        d.cam3_set = true;
    }
    h.run_steps(3);
    tela(&mut h, "tutorial-portas");
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 11);
    h.get_by_label("Concluir").click();
    h.run_steps(4);
    assert!(h.state().tutorial.is_none());
    let _ = Component::new("text3d");
}

#[test]
fn tutorial_uma_conversa_e_o_editor_de_conversa() {
    use nn_studio::scene_data::{Component, EventItem, Object, Prop, Rule};
    let dir = temp("tutorial-conversa");
    let mut h = studio(&dir, None);
    h.state_mut().settings.projects_dir = Some(dir.join("projetos"));
    h.state_mut().start_tutorial("uma-conversa");
    h.run_steps(4);
    let projeto = dir.join("projetos/uma-conversa");
    h.state_mut().tutorial.as_mut().unwrap().pause = std::time::Duration::ZERO;
    h.get_by_label("Próximo").click();
    fn espera(h: &mut Harness<'_, StudioApp>, esperado: usize) {
        h.run_steps(6);
        assert_eq!(
            h.state().tutorial.as_ref().map(|t| t.step),
            Some(esperado),
            "o tutorial devia estar no passo {esperado}"
        );
    }
    fn conversa<'a>(h: &'a mut Harness<'_, StudioApp>) -> &'a mut nn_studio::dialogue::DialogueDoc {
        let ws = h.state_mut().project_mut().unwrap();
        ws.dialogues.values_mut().next().unwrap()
    }
    espera(&mut h, 1);
    // Arquivos > ➕ Novo > Diálogo > Criar.
    h.get_by_label("➕ Novo").click();
    h.run_steps(2);
    h.get_by_label("💬 Diálogo (conversa)").click();
    h.run_steps(2);
    h.key_press(Key::Enter);
    espera(&mut h, 2);
    assert!(projeto.join("dialogos/conversa.dialogo").is_file());
    assert_eq!(conversa(&mut h).data.nodes.len(), 3);
    // Clicar no cartão escolhe a fala; o texto muda no painel.
    assert!(nn_studio::tutorial::anchor_rect(&h.ctx, "dialogo.fala:1").is_some());
    {
        let d = conversa(&mut h);
        d.selected = Some(1);
        d.data.node_mut(1).unwrap().text = "Oi! Tudo bem por aí?".into();
    }
    espera(&mut h, 3);
    // Arrastar da bolinha da fala 3 para o vazio cria a fala 4, ligada.
    let saida = nn_studio::tutorial::anchor_rect(&h.ctx, "dialogo.saida:3")
        .unwrap()
        .center();
    let destino = saida + egui::vec2(-60.0, 130.0);
    h.hover_at(saida);
    h.run_steps(1);
    h.drag_at(saida);
    h.run_steps(1);
    for i in 1..=6 {
        h.hover_at(saida + (destino - saida) * (i as f32 / 6.0));
        h.run_steps(1);
    }
    h.drop_at(destino);
    h.run_steps(2);
    {
        let d = conversa(&mut h);
        assert_eq!(d.data.nodes.len(), 4, "nasceu uma fala");
        assert_eq!(d.data.node(3).unwrap().next, 4, "já ligada");
        assert_eq!(d.selected, Some(4));
        d.data.node_mut(4).unwrap().text = "Quer uma dica?".into();
    }
    espera(&mut h, 4);
    // ➕ Escolha duas vezes na fala nova.
    for _ in 0..2 {
        h.get_by_label("➕ Escolha").click();
        h.run_steps(2);
    }
    assert_eq!(conversa(&mut h).data.node(4).unwrap().choices.len(), 2);
    espera(&mut h, 5);
    tela(&mut h, "editor-de-conversa");
    // Desfazer tira a segunda escolha; refazer põe de volta.
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    assert_eq!(conversa(&mut h).data.node(4).unwrap().choices.len(), 1);
    atalho(&mut h, Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
    assert_eq!(conversa(&mut h).data.node(4).unwrap().choices.len(), 2);
    // Sem fala escolhida, o painel mostra a conversa: ➕ Personagem.
    conversa(&mut h).selected = None;
    h.run_steps(2);
    h.get_by_label("➕ Personagem").click();
    h.run_steps(2);
    {
        let d = conversa(&mut h);
        assert_eq!(d.data.characters.len(), 2);
        let nome = d.data.characters[1].name.clone();
        d.data.node_mut(4).unwrap().who = nome;
    }
    espera(&mut h, 6);
    h.get_by_label("Salvar").click();
    espera(&mut h, 7);
    let salvo = std::fs::read_to_string(projeto.join("dialogos/conversa.dialogo")).unwrap();
    let lido = nn_studio::dialogue::read(&salvo, "conversa.dialogo").unwrap();
    assert_eq!(lido.nodes.len(), 4);
    assert_eq!(lido.node(4).unwrap().choices.len(), 2);
    // Na cena: o herói e quem fala.
    h.state_mut()
        .project_mut()
        .unwrap()
        .open_file("cenas/mundo.cena", None);
    h.run_steps(2);
    fn doc<'a>(h: &'a mut Harness<'_, StudioApp>) -> &'a mut nn_studio::scene_view::SceneDoc {
        let ws = h.state_mut().project_mut().unwrap();
        ws.scenes.get_mut("cenas/mundo.cena").unwrap()
    }
    let mut heroi = Object::new(50, "Jogador");
    heroi
        .components
        .push(Component::behavior("nn:jogador3d", &[]));
    doc(&mut h).data.objects.push(heroi);
    espera(&mut h, 8);
    let mut ana = Object::new(51, "Ana");
    ana.components.push(Component::behavior(
        "nn:dialogo",
        &[(
            "arquivo".into(),
            Prop::Str("dialogos/conversa.dialogo".into()),
        )],
    ));
    doc(&mut h).data.objects.push(ana);
    doc(&mut h).selection = vec![51];
    espera(&mut h, 9);
    // O Inspetor tem o botão para abrir a conversa.
    h.get_by_label("💬 Editar a conversa");
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 10);
    {
        let mut quando = EventItem {
            id: "escolheu".into(),
            ..Default::default()
        };
        quando.set("opcao", Prop::Str("Sim".into()));
        let mut entao = EventItem {
            id: "dar_item".into(),
            ..Default::default()
        };
        entao.set("item", Prop::Str("mapa".into()));
        doc(&mut h).data.events.push(Rule {
            when: vec![quando],
            actions: vec![entao],
            ..Default::default()
        });
    }
    espera(&mut h, 11);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 12);
    h.get_by_label("Concluir").click();
    h.run_steps(4);
    assert!(h.state().tutorial.is_none());
}

/// Numa cena 3D, o Inspetor de um comportamento só mostra as propriedades
/// que valem nela, com o padrão do 3D (o empurrão do Perigo em m/s).
#[test]
fn cena_3d_comportamentos_com_padroes_do_3d() {
    let dir = temp("cena-3d-padroes");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    com_cena3d(&mut h, &proj);
    let cubo = objeto(&h, "Cubo").id;
    cena_mut(&mut h).selection = vec![cubo];
    h.run();
    h.get_by_label("➕ Adicionar componente").click();
    h.run();
    h.get_by_label_contains("Inimigos").click();
    h.run();
    h.get_by_label("Inimigo 3D");
    h.get_by_label("Perigo").click();
    h.run();
    let o = objeto(&h, "Cubo").clone();
    let p = o
        .components
        .iter()
        .find(|c| c.str("script") == "nn:perigo")
        .unwrap();
    assert_eq!(p.num("empurrao"), 7.0, "o padrão do 3D");
    // Atirar: "Some ao sair da tela" (só do 2D) não aparece no 3D; a altura
    // de saída (só do 3D) aparece.
    h.get_by_label("➕ Adicionar componente").click();
    h.run();
    h.get_by_label_contains("Ações").click();
    h.run();
    h.get_by_label("Destruir depois").click();
    h.run();
    assert!(h.query_by_label("Some ao sair da tela").is_none());
    tela(&mut h, "cena-3d-inimigos");
}

/// Numa cena 3D com HUD (o Vale 3D): a vista abre perto do herói, o HUD
/// aparece por cima do mundo e dá para clicar e arrastar ele; Enquadrar não
/// conta o HUD; ➕ Objeto > Na tela (HUD) põe um texto no meio da tela.
#[test]
fn cena_3d_com_hud_por_cima() {
    let dir = temp("cena-3d-hud");
    let proj = dir.join("jogo");
    nn_run::create_project(&nn_run::NewOptions {
        dir: proj.clone(),
        title: None,
        template: nn_run::Template::Vale3D,
    })
    .unwrap();
    // (Os ajudantes daqui leem a cena teste.cena.)
    std::fs::copy(proj.join("cenas/vale.cena"), proj.join("cenas/teste.cena")).unwrap();
    // (A cena tem animações: a tela redesenha sempre; nada de `run`.)
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
        .open_file("cenas/teste.cena", None);
    h.run_steps(4);
    let passo = |h: &mut Harness<'_, StudioApp>| h.run_steps(2);

    // Abre olhando para o herói de perto (não para a cena toda, de longe).
    let heroi = objeto(&h, "Herói").clone();
    let pos = glam::Vec3::new(heroi.x as f32, heroi.y as f32, heroi.z as f32);
    let dist = (cena(&h).cam3.pos - pos).length();
    assert!(dist < 20.0, "a câmera ficou a {dist} m do herói");
    assert!(cena(&h).show_fog3);
    h.get_by_label("HUD");
    h.get_by_label("Névoa");
    tela(&mut h, "cena-3d-hud");

    // O texto da dica, no HUD: um clique seleciona.
    let dica = objeto(&h, "Dica").clone();
    let rect = h.get_by_label("Vista da cena 3D").rect();
    let w = cena(&h).data.world_transform(dica.id).unwrap();
    let p = cena(&h).camera.to_screen(rect, w.x, w.y);
    assert!(
        rect.contains(p),
        "a dica fica dentro da vista: {p:?} {rect:?}"
    );
    h.hover_at(p);
    passo(&mut h);
    h.drag_at(p);
    passo(&mut h);
    h.drop_at(p);
    passo(&mut h);
    assert_eq!(cena(&h).selection, vec![dica.id]);

    // Arrastar leva a dica pela tela do jogo (direita e para baixo).
    let q = p + egui::vec2(60.0, 30.0);
    h.hover_at(p);
    passo(&mut h);
    h.drag_at(p);
    passo(&mut h);
    for k in 1..=4 {
        h.hover_at(p + (q - p) * (k as f32 / 4.0));
        passo(&mut h);
    }
    h.drop_at(q);
    passo(&mut h);
    let movida = objeto(&h, "Dica").clone();
    assert!(movida.x > dica.x + 20.0, "x {} → {}", dica.x, movida.x);
    assert!(movida.y < dica.y - 10.0, "y {} → {}", dica.y, movida.y);
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    passo(&mut h);
    assert_eq!(objeto(&h, "Dica").x, dica.x);

    // Enquadrar a cena toda: o HUD (em pixels) não conta, e de longe a
    // vista desliga a névoa para dar para ver.
    h.key_press(Key::Escape);
    passo(&mut h);
    cena_mut(&mut h).selection.clear();
    h.get_by_label("⛶ Enquadrar").click();
    passo(&mut h);
    let cam = cena(&h).cam3;
    let alvo = cam.pos + cam.forward() * cam.pos.length();
    assert!(alvo.y > -10.0 && alvo.y < 20.0, "olhando para {alvo}");
    assert!(!cena(&h).show_fog3);

    // ➕ Objeto > Na tela (HUD) > Texto na tela.
    h.get_by_label("➕ Objeto").click();
    passo(&mut h);
    h.get_by(|n| n.label().is_some_and(|l| l.starts_with("Na tela (HUD)")))
        .hover();
    h.run_steps(3);
    h.get_by_label("Texto na tela").click();
    passo(&mut h);
    let texto = objeto(&h, "texto").clone();
    assert!(texto.component("text").unwrap().bool("hud"));
    assert_eq!(cena(&h).selection, vec![texto.id]);
    let t = cena(&h).data.world_transform(texto.id).unwrap();
    let c = cena(&h).camera.to_screen(rect, t.x, t.y);
    assert!(
        (c - rect.center()).length() < 4.0,
        "{c:?} {:?}",
        rect.center()
    );
}

/// Luzes numa cena 3D: ➕ Objeto > Luz (Tocha e Holofote), o ambiente pronto
/// Caverna, o Inspetor da Luz (a abertura só no holofote) e, na vista, o chão
/// aceso em volta da tocha no escuro.
#[test]
fn cena_3d_luzes_e_ambiente_caverna() {
    use nn_studio::scene_data::Prop;
    let gpu = std::env::var_os("NN_REQUIRE_GPU").is_some();
    let dir = temp("cena-3d-luzes");
    let proj = plataforma(&dir);
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), gpu);
    com_cena3d(&mut h, &proj);
    let luz = |h: &mut Harness<'_, StudioApp>, item: &str| {
        h.get_by_label("➕ Objeto").click();
        h.run_steps(2);
        h.get_by(|n| {
            n.label()
                .is_some_and(|l| l.starts_with("Luz") && l != "Luz")
        })
        .hover();
        h.run_steps(3);
        h.get_by(|n| n.role() == Role::Button && n.label().as_deref() == Some(item))
            .click();
        h.run_steps(4);
    };
    luz(&mut h, "Tocha");
    let tocha = objeto(&h, "Tocha").clone();
    // A tocha: o cabo e a chama (a luz tremulando e o fogo), com a brasa
    // que brilha dentro.
    let chama = tocha.children.iter().find(|c| c.name == "Chama").unwrap();
    let l = chama.component("light3d").unwrap();
    assert!(l.num("flicker") > 0.0);
    assert!(chama.component("particles").is_some());
    assert_eq!(
        chama.children[0].component("shape3d").unwrap().num("glow"),
        1.0
    );
    assert!(tocha.children.iter().any(|c| c.name == "Cabo"));
    // O Inspetor da chama: uma lâmpada, sem a abertura do holofote.
    cena_mut(&mut h).selection = vec![chama.id];
    h.run_steps(3);
    h.get_by_label("Tremular");
    h.get_by_label("Alcance");
    assert!(h.query_by_label("Abertura do cone").is_none());
    luz(&mut h, "Holofote");
    let holofote = objeto(&h, "Holofote").clone();
    assert_eq!(holofote.component("light3d").unwrap().str("kind"), "spot");
    assert_eq!(cena_mut(&mut h).selection, vec![holofote.id]);
    h.get_by_label("Abertura do cone");
    // O ambiente pronto Caverna: sem céu e sem sol, com o fundo escuro.
    cena_mut(&mut h).selection.clear();
    h.run_steps(3);
    h.get_by_label("Caverna").click();
    h.run_steps(3);
    {
        let doc = cena_mut(&mut h);
        assert_eq!(doc.data.env("sky"), Prop::Bool(false));
        assert_eq!(doc.data.env("sun_color").str(), "#000000");
        assert_eq!(doc.data.background, "#0A0A0E");
        // Só a tocha, no meio da vista.
        doc.data.objects.retain(|o| o.name != "Holofote");
        doc.cam3
            .look_at(glam::Vec3::new(0.0, 6.0, 6.0), glam::Vec3::ZERO);
        doc.cam3_set = true;
    }
    h.run_steps(4);
    tela(&mut h, "cena-3d-luzes");
    if !gpu {
        return;
    }
    // Na vista: o chão perto da tocha aceso (laranja); longe, escuro.
    let rect = h.get_by_label("Vista da cena 3D").rect();
    let img = h.render().expect("render");
    let ppp = img.width() as f32 / 1280.0;
    let px = |x: f32, y: f32| {
        let p = img.get_pixel((x * ppp) as u32, (y * ppp) as u32);
        (p[0] as u32, p[1] as u32, p[2] as u32)
    };
    let c = rect.center();
    let perto = px(c.x + 30.0, c.y + 25.0);
    let canto = px(rect.left() + 6.0, rect.bottom() - 30.0);
    assert!(
        perto.0 > 60 && perto.0 > perto.2 + 20,
        "perto da tocha: {perto:?}"
    );
    assert!(canto.0 + canto.1 + canto.2 < 90, "longe, escuro: {canto:?}");
}

#[test]
fn tutorial_uma_caverna_avanca_sozinho_ate_o_fim() {
    use nn_studio::scene_data::{Component, EventItem, Object, Prop, Rule};
    let dir = temp("tutorial-caverna");
    let mut h = studio(&dir, None);
    h.state_mut().settings.projects_dir = Some(dir.join("projetos"));
    h.state_mut().start_tutorial("uma-caverna");
    h.run_steps(4);
    let projeto = dir.join("projetos/uma-caverna");
    for f in [
        "modelos/cristal.glb",
        "modelos/tocha.glb",
        "modelos/lampiao.glb",
    ] {
        assert!(projeto.join(f).is_file(), "o modelo vazio3d traz {f}");
    }
    h.state_mut().tutorial.as_mut().unwrap().pause = std::time::Duration::ZERO;
    h.get_by_label("Próximo").click();
    h.run_steps(4);
    fn doc<'a>(h: &'a mut Harness<'_, StudioApp>) -> &'a mut nn_studio::scene_view::SceneDoc {
        let ws = h.state_mut().project_mut().unwrap();
        ws.scenes.get_mut("cenas/mundo.cena").unwrap()
    }
    fn espera(h: &mut Harness<'_, StudioApp>, esperado: usize) {
        h.run_steps(6);
        assert_eq!(
            h.state().tutorial.as_ref().map(|t| t.step),
            Some(esperado),
            "o tutorial devia estar no passo {esperado}"
        );
    }
    espera(&mut h, 1);
    // O ambiente pronto Caverna, pelo botão do Inspetor da cena.
    doc(&mut h).selection.clear();
    h.run_steps(3);
    h.get_by_label("Caverna").click();
    espera(&mut h, 2);
    // A tocha (como o ➕ Objeto > Luz > Tocha faz).
    doc(&mut h).data.objects.push({
        let mut t = nn_studio::scene_view::scene3d::torch();
        t.id = 20;
        for (k, c) in t.children.iter_mut().enumerate() {
            c.id = 21 + k as u32;
            for (j, g) in c.children.iter_mut().enumerate() {
                g.id = 30 + j as u32;
            }
        }
        t
    });
    espera(&mut h, 3);
    let chama = doc(&mut h).data.find_mut(22).unwrap();
    chama
        .components
        .iter_mut()
        .find(|c| c.kind == "light3d")
        .unwrap()
        .set("range", Prop::Num(10.0));
    espera(&mut h, 4);
    let mut cristal = Object::new(40, "cristal");
    let mut m = Component::new("model");
    m.set("file", Prop::Str("modelos/cristal.glb".into()));
    cristal.components.push(m);
    doc(&mut h).data.objects.push(cristal);
    espera(&mut h, 5);
    let mut esfera = Object::new(41, "Esfera");
    let mut f = Component::new("shape3d");
    f.set("shape", Prop::Str("sphere".into()));
    f.set("glow", Prop::Num(1.0));
    esfera.components.push(f);
    doc(&mut h).data.objects.push(esfera);
    espera(&mut h, 6);
    let mut holofote = Object::new(42, "Holofote");
    let mut l = Component::new("light3d");
    l.set("kind", Prop::Str("spot".into()));
    holofote.components.push(l);
    doc(&mut h).data.objects.push(holofote);
    espera(&mut h, 7);
    let mut heroi = Object::new(43, "Jogador");
    heroi
        .components
        .push(Component::behavior("nn:jogador3d", &[]));
    doc(&mut h).data.objects.push(heroi);
    espera(&mut h, 8);
    let item = |id: &str, props: &[(&str, &str)]| EventItem {
        id: id.into(),
        not: false,
        props: props
            .iter()
            .map(|(k, v)| (k.to_string(), Prop::Str(v.to_string())))
            .collect(),
    };
    doc(&mut h).data.find_mut(42).unwrap().events.push(Rule {
        when: vec![item("tecla", &[("modo", "apertar"), ("tecla", "f")])],
        actions: vec![item("luz", &[("acao", "alternar"), ("alvo", "@este")])],
        ..Default::default()
    });
    espera(&mut h, 9);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 10);
    h.get_by_label("Concluir").click();
    h.run_steps(3);
    assert!(h.state().tutorial.is_none());
}

/// Dia e noite no NN Studio: o Ambiente liga o ciclo e mostra a hora em que
/// o jogo começa; às 22h a vista fica escura e o poste "Só à noite" acende
/// o chão em volta; ao meio-dia, ele fica apagado.
#[test]
fn cena_3d_dia_e_noite_na_vista() {
    use nn_studio::scene_data::{Component, Object, Prop};
    let gpu = std::env::var_os("NN_REQUIRE_GPU").is_some();
    let dir = temp("cena-3d-dia");
    let proj = plataforma(&dir);
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), gpu);
    com_cena3d(&mut h, &proj);
    {
        let doc = cena_mut(&mut h);
        doc.selection.clear();
        let mut poste = Object::new(50, "Poste");
        poste.y = 1.5;
        poste.x = 3.0;
        let mut l = Component::new("light3d");
        l.set("night", Prop::Bool(true));
        l.set("range", Prop::Num(4.0));
        poste.components.push(l);
        doc.data.objects.push(poste);
        doc.cam3.look_at(
            glam::Vec3::new(3.0, 8.0, 7.0),
            glam::Vec3::new(3.0, 0.0, 0.0),
        );
        doc.cam3_set = true;
    }
    h.run_steps(3);
    // Sem o ciclo, não há "Começa às".
    assert!(h.query_by_label("Começa às:").is_none());
    h.get_by_label("Dia e noite:");
    cena_mut(&mut h).data.set_env("cycle", Prop::Bool(true));
    h.run_steps(3);
    h.get_by_label("Começa às:");
    if !gpu {
        return;
    }
    let foto = |h: &mut Harness<'_, StudioApp>, hora: f64| {
        cena_mut(h).data.set_env("start_hour", Prop::Num(hora));
        h.run_steps(3);
        let rect = h.get_by_label("Vista da cena 3D").rect();
        let img = h.render().expect("render");
        let ppp = img.width() as f32 / 1280.0;
        // O chão embaixo do poste e um pedaço longe dele.
        let v = |x: f32, y: f32| {
            let p = img.get_pixel(
                ((rect.left() + x) * ppp) as u32,
                ((rect.top() + y) * ppp) as u32,
            );
            p[0] as u32 + p[1] as u32 + p[2] as u32
        };
        (
            v(rect.width() / 2.0, rect.height() * 0.62),
            v(12.0, rect.height() - 12.0),
        )
    };
    let (perto_dia, longe_dia) = foto(&mut h, 12.0);
    let (perto_noite, longe_noite) = foto(&mut h, 22.0);
    tela(&mut h, "cena-3d-noite");
    assert!(
        longe_noite * 2 < longe_dia,
        "de noite, escuro: {longe_noite} (dia {longe_dia})"
    );
    assert!(
        perto_noite > longe_noite * 2,
        "o poste acende o chão: {perto_noite} / {longe_noite}"
    );
    assert!(
        perto_dia < longe_dia * 2,
        "de dia, o poste fica apagado: {perto_dia} / {longe_dia}"
    );
}

#[test]
fn tutorial_dia_e_noite_avanca_sozinho_ate_o_fim() {
    use nn_studio::scene_data::{Component, EventItem, Object, Prop, Rule};
    let dir = temp("tutorial-dia");
    let mut h = studio(&dir, None);
    h.state_mut().settings.projects_dir = Some(dir.join("projetos"));
    h.state_mut().start_tutorial("dia-e-noite");
    h.run_steps(4);
    h.state_mut().tutorial.as_mut().unwrap().pause = std::time::Duration::ZERO;
    h.get_by_label("Próximo").click();
    h.run_steps(4);
    fn doc<'a>(h: &'a mut Harness<'_, StudioApp>) -> &'a mut nn_studio::scene_view::SceneDoc {
        let ws = h.state_mut().project_mut().unwrap();
        ws.scenes.get_mut("cenas/mundo.cena").unwrap()
    }
    fn espera(h: &mut Harness<'_, StudioApp>, esperado: usize) {
        h.run_steps(6);
        assert_eq!(
            h.state().tutorial.as_ref().map(|t| t.step),
            Some(esperado),
            "o tutorial devia estar no passo {esperado}"
        );
    }
    espera(&mut h, 1);
    doc(&mut h).data.set_env("cycle", Prop::Bool(true));
    doc(&mut h).data.set_env("day_length", Prop::Num(2.0));
    espera(&mut h, 2);
    doc(&mut h).data.set_env("start_hour", Prop::Num(21.0));
    espera(&mut h, 3);
    let mut poste = Object::new(40, "Lâmpada");
    let mut l = Component::new("light3d");
    l.set("night", Prop::Bool(true));
    poste.components.push(l);
    doc(&mut h).data.objects.push(poste);
    espera(&mut h, 4);
    let mut janela = Object::new(41, "Cubo");
    let mut f = Component::new("shape3d");
    f.set("glow", Prop::Num(1.0));
    f.set("glow_night", Prop::Bool(true));
    janela.components.push(f);
    doc(&mut h).data.objects.push(janela);
    espera(&mut h, 5);
    let mut heroi = Object::new(42, "Jogador");
    heroi
        .components
        .push(Component::behavior("nn:jogador3d", &[]));
    doc(&mut h).data.objects.push(heroi);
    espera(&mut h, 6);
    let mut placar = Object::new(43, "relógio");
    placar.components.push(Component::behavior(
        "nn:mostrar_valor",
        &[("texto".into(), Prop::Str("{relogio}".into()))],
    ));
    doc(&mut h).data.objects.push(placar);
    espera(&mut h, 7);
    let item = |id: &str, props: &[(&str, &str)]| EventItem {
        id: id.into(),
        not: false,
        props: props
            .iter()
            .map(|(k, v)| (k.to_string(), Prop::Str(v.to_string())))
            .collect(),
    };
    doc(&mut h).data.events.push(Rule {
        when: vec![item("amanhecer", &[("quando", "anoitecer")])],
        actions: vec![item("som", &[("som", "assets/sons/vitoria.wav")])],
        ..Default::default()
    });
    espera(&mut h, 8);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 9);
    h.get_by_label("Concluir").click();
    h.run_steps(3);
    assert!(h.state().tutorial.is_none());
}

/// Água, vento e halo no NN Studio: ➕ Objeto > Água (lago) traz um plano com
/// o Material Água; o Ambiente tem Vento e Halo do brilho, e um pronto
/// (Entardecer) não mexe neles nem no Dia e noite. Na vista, uma esfera que
/// brilha no escuro tem um halo em volta, que some com o Halo em 0.
#[test]
fn cena_3d_agua_vento_e_halo() {
    use nn_studio::scene_data::{Component, Object, Prop};
    let gpu = std::env::var_os("NN_REQUIRE_GPU").is_some();
    let dir = temp("cena-3d-agua");
    let proj = plataforma(&dir);
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), gpu);
    com_cena3d(&mut h, &proj);
    h.get_by_label("➕ Objeto").click();
    h.run_steps(2);
    h.get_by_label("Água (lago)").click();
    h.run_steps(4);
    let lago = objeto(&h, "Lago").clone();
    let f = lago.component("shape3d").unwrap();
    assert_eq!(f.str("material"), "agua");
    assert_eq!(f.str("shape"), "plane");
    assert!(lago.component("body3d").is_none(), "dá para entrar no lago");
    assert!(
        (lago.y - 0.1).abs() < 0.01,
        "um pouco acima do chão: {}",
        lago.y
    );
    // O Inspetor da Forma 3D mostra o Material.
    h.get_by_label("Material");
    h.get_by_label("Balança com o vento");
    // O Ambiente: Vento e Halo do brilho.
    cena_mut(&mut h).selection.clear();
    h.run_steps(3);
    h.get_by_label("Vento:");
    h.get_by_label("Halo do brilho:");
    {
        let doc = cena_mut(&mut h);
        doc.data.set_env("wind", Prop::Num(1.5));
        doc.data.set_env("cycle", Prop::Bool(true));
        doc.data.set_env("bloom", Prop::Num(2.0));
    }
    h.run_steps(3);
    h.get_by_label("Entardecer").click();
    h.run_steps(3);
    {
        let doc = cena_mut(&mut h);
        assert_eq!(doc.data.env("sky_horizon").str(), "#F7A35C");
        assert_eq!(
            doc.data.env("wind").num(),
            1.5,
            "o pronto não mexe no vento"
        );
        assert_eq!(doc.data.env("bloom").num(), 2.0, "nem no halo");
        assert!(doc.data.env("cycle").bool(), "nem no Dia e noite");
        // O lago de lado, contra o sol.
        let l = glam::Vec3::new(lago.x as f32, lago.y as f32, lago.z as f32);
        doc.cam3.look_at(l + glam::Vec3::new(-6.0, 2.0, 8.0), l);
        doc.cam3_set = true;
    }
    h.run_steps(3);
    tela(&mut h, "cena-3d-agua");
    if !gpu {
        return;
    }
    {
        // No meio da vista, a água (azulada, com o céu), não o chão verde.
        let rect = h.get_by_label("Vista da cena 3D").rect();
        let img = h.render().expect("render");
        let ppp = img.width() as f32 / 1280.0;
        let c = rect.center();
        let p = img.get_pixel((c.x * ppp) as u32, ((c.y + 20.0) * ppp) as u32);
        assert!(p[2] > p[1], "a água no meio da vista: {p:?}");
    }
    // O halo: no escuro (Caverna), uma esfera que brilha, no meio da vista.
    h.get_by_label("Caverna").click();
    h.run_steps(3);
    {
        let doc = cena_mut(&mut h);
        doc.data.objects.retain(|o| o.name != "Lago");
        let mut esfera = Object::new(60, "Esfera");
        let mut f = Component::new("shape3d");
        f.set("shape", Prop::Str("sphere".into()));
        f.set("color", Prop::Str("#FFD166".into()));
        f.set("glow", Prop::Num(1.0));
        esfera.components.push(f);
        esfera.y = 3.0;
        doc.data.objects.push(esfera);
        doc.cam3.look_at(
            glam::Vec3::new(0.0, 3.0, 6.0),
            glam::Vec3::new(0.0, 3.0, 0.0),
        );
        doc.cam3_set = true;
    }
    // A linha do meio da vista, do centro para a direita (brilho de cada ponto).
    let linha = |h: &mut Harness<'_, StudioApp>, halo: f64| -> Vec<u32> {
        cena_mut(h).data.set_env("bloom", Prop::Num(halo));
        h.run_steps(3);
        let rect = h.get_by_label("Vista da cena 3D").rect();
        let img = h.render().expect("render");
        let ppp = img.width() as f32 / 1280.0;
        let c = rect.center();
        (0..(rect.width() * 0.45) as u32)
            .map(|dx| {
                let p = img.get_pixel(((c.x + dx as f32) * ppp) as u32, (c.y * ppp) as u32);
                p[0] as u32 + p[1] as u32 + p[2] as u32
            })
            .collect()
    };
    let com = linha(&mut h, 1.5);
    tela(&mut h, "cena-3d-halo");
    let sem = linha(&mut h, 0.0);
    assert!(sem[0] > 300, "a esfera brilha no meio: {}", sem[0]);
    // A borda da esfera (sem halo) e, logo depois dela, o escuro.
    let borda = sem
        .iter()
        .position(|&v| v < 120)
        .expect("a borda da esfera");
    let soma = |l: &[u32]| l[borda + 4..borda + 24].iter().sum::<u32>();
    assert!(
        soma(&com) > soma(&sem) + 300,
        "o halo clareia em volta: {} (sem halo {})",
        soma(&com),
        soma(&sem)
    );
}

#[test]
fn tutorial_um_lago_avanca_sozinho_ate_o_fim() {
    use nn_studio::scene_data::{Component, Object, Prop};
    let dir = temp("tutorial-lago");
    let mut h = studio(&dir, None);
    h.state_mut().settings.projects_dir = Some(dir.join("projetos"));
    h.state_mut().start_tutorial("um-lago-com-vida");
    h.run_steps(4);
    let projeto = dir.join("projetos/um-lago");
    for f in ["modelos/arvore.glb", "modelos/grama.glb"] {
        assert!(projeto.join(f).is_file(), "o modelo vazio3d traz {f}");
    }
    h.state_mut().tutorial.as_mut().unwrap().pause = std::time::Duration::ZERO;
    h.get_by_label("Próximo").click();
    h.run_steps(4);
    fn doc<'a>(h: &'a mut Harness<'_, StudioApp>) -> &'a mut nn_studio::scene_view::SceneDoc {
        let ws = h.state_mut().project_mut().unwrap();
        ws.scenes.get_mut("cenas/mundo.cena").unwrap()
    }
    fn espera(h: &mut Harness<'_, StudioApp>, esperado: usize) {
        h.run_steps(6);
        assert_eq!(
            h.state().tutorial.as_ref().map(|t| t.step),
            Some(esperado),
            "o tutorial devia estar no passo {esperado}"
        );
    }
    fn modelo(id: u32, file: &str, sway: f64) -> Object {
        let mut o = Object::new(id, file);
        let mut m = Component::new("model");
        m.set("file", Prop::Str(format!("modelos/{file}")));
        m.set("sway", Prop::Num(sway));
        o.components.push(m);
        o
    }
    espera(&mut h, 1);
    let mut lago = nn_studio::scene_view::scene3d::water();
    lago.id = 40;
    doc(&mut h).data.objects.push(lago);
    espera(&mut h, 2);
    doc(&mut h).data.set_env("wind", Prop::Num(1.0));
    espera(&mut h, 3);
    doc(&mut h).data.objects.push(modelo(41, "arvore.glb", 0.0));
    espera(&mut h, 4);
    doc(&mut h).data.objects.push(modelo(42, "arvore.glb", 1.0));
    espera(&mut h, 5);
    for id in 43..46 {
        doc(&mut h).data.objects.push(modelo(id, "grama.glb", 1.5));
    }
    espera(&mut h, 6);
    let mut fx = Object::new(46, "brilho");
    fx.components.push(Component::new("particles"));
    doc(&mut h).data.objects.push(fx);
    espera(&mut h, 7);
    // O pronto Entardecer, pelo botão do Inspetor da cena: o vento fica.
    doc(&mut h).selection.clear();
    h.run_steps(3);
    h.get_by_label("Entardecer").click();
    espera(&mut h, 8);
    assert_eq!(doc(&mut h).data.env("wind").num(), 1.0);
    doc(&mut h).data.set_env("bloom", Prop::Num(2.0));
    espera(&mut h, 9);
    let mut heroi = Object::new(47, "Jogador");
    heroi
        .components
        .push(Component::behavior("nn:jogador3d", &[]));
    doc(&mut h).data.objects.push(heroi);
    espera(&mut h, 10);
    h.get_by_label("▶ Testar daqui").click();
    espera(&mut h, 11);
    h.get_by_label("Concluir").click();
    h.run_steps(3);
    assert!(h.state().tutorial.is_none());
}

// Botão direito e busca rápida (S43) ------------------------------------------

/// Botão direito com o mouse parado em `p` (aperta e solta).
fn clique_direito(h: &mut Harness<'_, StudioApp>, p: egui::Pos2) {
    h.hover_at(p);
    h.run();
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        h.step();
    }
    h.run();
}

/// Arrasta com o botão direito (move a câmera, não abre o menu).
fn arrastar_direito(h: &mut Harness<'_, StudioApp>, from: egui::Pos2, to: egui::Pos2) {
    h.hover_at(from);
    h.run();
    h.event(egui::Event::PointerButton {
        pos: from,
        button: egui::PointerButton::Secondary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    h.step();
    for i in 1..=6 {
        let t = i as f32 / 6.0;
        h.hover_at(from + (to - from) * t);
        h.step();
    }
    h.event(egui::Event::PointerButton {
        pos: to,
        button: egui::PointerButton::Secondary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run();
}

/// Passa o mouse num submenu (o nome dele vem com a setinha).
fn submenu(h: &mut Harness<'_, StudioApp>, nome: &str) {
    h.get_by(|n| n.label().is_some_and(|l| l.starts_with(nome) && l != nome))
        .hover();
    h.run_steps(3);
}

fn perto(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

/// Cena 3D: o botão direito no chão cria ali; num objeto, o menu dele
/// (duplicar, copiar e colar a posição); arrastar com ele só gira a câmera;
/// F2 renomeia pela Hierarquia.
#[test]
fn botao_direito_na_cena_3d() {
    let dir = temp("menu-3d");
    let proj = plataforma(&dir);
    let gpu = std::env::var_os("NN_REQUIRE_GPU").is_some();
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), gpu);
    com_cena3d(&mut h, &proj);
    h.run();

    // No chão: o menu de criar. A esfera nasce em pé onde foi o clique.
    let alvo = glam::Vec3::new(3.0, 0.0, 2.0);
    let pa = na_vista3d(&h, alvo);
    clique_direito(&mut h, pa);
    h.get_by_label("Criar aqui");
    h.get_by_label("Chão");
    tela(&mut h, "menu-3d-chao");
    h.get_by_label("Esfera").click();
    h.run();
    assert!(h.query_by_label("Criar aqui").is_none(), "o menu fecha");
    let esfera = objeto(&h, "Esfera").clone();
    assert!(
        perto(esfera.x, 3.0, 0.15) && perto(esfera.z, 2.0, 0.15) && perto(esfera.y, 0.5, 0.05),
        "esfera em ({}, {}, {})",
        esfera.x,
        esfera.y,
        esfera.z
    );
    assert_eq!(cena(&h).selection, vec![esfera.id]);
    // O menu do chão (no fim) age no chão, e não na esfera selecionada.
    let pchao = na_vista3d(&h, glam::Vec3::new(-2.0, 0.0, 3.0));
    clique_direito(&mut h, pchao);
    assert_eq!(
        cena(&h).selection,
        vec![esfera.id],
        "o chão não é selecionado"
    );
    submenu(&mut h, "Chão");
    h.get_by_label_contains("Excluir").click();
    h.run();
    assert!(cena(&h).data.objects.iter().all(|o| o.name != "Chão"));
    objeto(&h, "Esfera");
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    objeto(&h, "Chão");

    // No cubo: o menu dele, com o nome em cima.
    let cubo = objeto(&h, "Cubo").clone();
    let pc = na_vista3d(
        &h,
        glam::Vec3::new(cubo.x as f32, cubo.y as f32, cubo.z as f32),
    );
    clique_direito(&mut h, pc);
    assert_eq!(
        cena(&h).selection,
        vec![cubo.id],
        "o clique seleciona o cubo"
    );
    h.get_by_label_contains("Renomear");
    h.get_by(|n| n.label().is_some_and(|l| l.starts_with("➕ Criar aqui")));
    tela(&mut h, "menu-3d-objeto");
    h.get_by_label_contains("Duplicar").click();
    h.run();
    let cubos = cena(&h)
        .data
        .objects
        .iter()
        .filter(|o| o.name.starts_with("Cubo"))
        .count();
    assert_eq!(cubos, 2, "duplicou");
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    // Copiar a posição do cubo e colar na esfera.
    clique_direito(&mut h, pc);
    h.get_by_label("Copiar posição").click();
    h.run();
    let pe = na_vista3d(
        &h,
        glam::Vec3::new(esfera.x as f32, esfera.y as f32, esfera.z as f32),
    );
    clique_direito(&mut h, pe);
    h.get_by_label("Colar posição").click();
    h.run();
    let colada = objeto(&h, "Esfera").clone();
    assert!(
        perto(colada.x, cubo.x, 1e-3)
            && perto(colada.y, cubo.y, 1e-3)
            && perto(colada.z, cubo.z, 1e-3),
        "esfera colada em ({}, {}, {})",
        colada.x,
        colada.y,
        colada.z
    );
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    assert_eq!(objeto(&h, "Esfera").x, esfera.x);

    // ➕ Criar aqui > Luz > Lâmpada, em cima do cubo: no alto.
    clique_direito(&mut h, pc);
    submenu(&mut h, "➕ Criar aqui");
    submenu(&mut h, "Luz");
    h.get_by_label("Lâmpada").click();
    h.run();
    let lampada = objeto(&h, "Lâmpada").clone();
    assert!(lampada.y > 2.0, "a lâmpada fica no alto: y = {}", lampada.y);

    // Arrastar com o botão direito gira a câmera e não abre o menu.
    let antes = cena(&h).cam3;
    let rect = h.get_by_label("Vista da cena 3D").rect();
    arrastar_direito(&mut h, rect.center(), rect.center() + egui::vec2(80.0, 0.0));
    assert!(h.query_by_label("Criar aqui").is_none());
    assert!(h.query_by_label_contains("Renomear").is_none());
    assert_ne!(cena(&h).cam3.yaw, antes.yaw, "a câmera girou");

    // Esc fecha o menu (e não só limpa a seleção); com o menu aberto, Shift+A
    // não abre a busca por trás dele.
    clique_direito(&mut h, pa);
    h.get_by_label("Criar aqui");
    h.key_press_modifiers(Modifiers::SHIFT, Key::A);
    h.run();
    assert!(cena(&h).quick_add.is_none(), "a busca espera o menu");
    h.key_press(Key::Escape);
    h.run();
    assert!(
        h.query_by_label("Criar aqui").is_none(),
        "Esc fechou o menu"
    );
    // F2 com o mouse na vista: renomeia pela Hierarquia.
    cena_mut(&mut h).selection = vec![cubo.id];
    h.hover_at(rect.center());
    h.run();
    h.key_press(Key::F2);
    h.run();
    assert_eq!(
        cena(&h).renaming.as_ref().map(|r| r.0),
        Some(cubo.id),
        "renomeando o cubo"
    );
    // O campo de renomear da Hierarquia está com o foco, pronto para digitar.
    h.get_by(|n| {
        n.role() == Role::TextInput && n.value().as_deref() == Some("Cubo") && n.is_focused()
    });
}

/// Shift+A na vista 3D: a busca rápida cria onde o mouse está, acha com
/// erro de digitação e lembra o último usado.
#[test]
fn busca_rapida_shift_a_na_cena_3d() {
    let dir = temp("busca-rapida-3d");
    let proj = plataforma(&dir);
    let gpu = std::env::var_os("NN_REQUIRE_GPU").is_some();
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), gpu);
    com_cena3d(&mut h, &proj);
    h.run();
    let alvo = glam::Vec3::new(-3.0, 0.0, 1.0);
    let p = na_vista3d(&h, alvo);
    h.hover_at(p);
    h.run();
    atalho(&mut h, Modifiers::SHIFT, Key::A);
    assert!(cena(&h).quick_add.is_some(), "a busca abriu");
    tela(&mut h, "busca-rapida");
    digitar(&mut h, "tocah");
    h.get_by_label("Luz › Tocha");
    h.key_press(Key::Enter);
    h.run_steps(4);
    assert!(cena(&h).quick_add.is_none(), "Enter cria e fecha");
    let tocha = objeto(&h, "Tocha").clone();
    assert!(
        perto(tocha.x, -3.0, 0.15) && perto(tocha.z, 1.0, 0.15),
        "tocha em ({}, {})",
        tocha.x,
        tocha.z
    );
    // De novo, sem digitar: o último usado vem primeiro.
    h.hover_at(p);
    h.run_steps(2);
    h.key_press_modifiers(Modifiers::SHIFT, Key::A);
    h.run_steps(3);
    h.key_press(Key::Enter);
    h.run_steps(4);
    let tochas = cena(&h)
        .data
        .objects
        .iter()
        .filter(|o| o.name.starts_with("Tocha"))
        .count();
    assert_eq!(tochas, 2);
    // Esc fecha sem criar.
    h.hover_at(p);
    h.run_steps(2);
    h.key_press_modifiers(Modifiers::SHIFT, Key::A);
    h.run_steps(3);
    h.event(egui::Event::Text("cubo".into()));
    h.run_steps(3);
    h.key_press(Key::Escape);
    h.run_steps(3);
    assert!(cena(&h).quick_add.is_none());
    let n = cena(&h).data.objects.len();
    assert_eq!(n, 5, "câmera, chão, cubo e as duas tochas");
}

/// Cena 2D: o botão direito no vazio cria no ponto; arrastar com ele move a
/// vista; num objeto, o menu dele. Na Hierarquia: o vazio cria, e o objeto
/// tem ➕ Criar dentro. No Inspetor: o botão direito no componente restaura.
#[test]
fn botao_direito_na_cena_2d_na_hierarquia_e_no_inspetor() {
    let dir = temp("menu-2d");
    let proj = plataforma(&dir);
    let gpu = std::env::var_os("NN_REQUIRE_GPU").is_some();
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), gpu);
    com_cena(&mut h, &proj);
    let rect = h.get_by_label("Vista da cena").rect();

    // No vazio: criar ali.
    let p = rect.center() + egui::vec2(200.0, 120.0);
    let (wx, wy) = cena(&h).camera.to_world(rect, p);
    clique_direito(&mut h, p);
    h.get_by_label("Criar aqui");
    tela(&mut h, "menu-2d-vazio");
    h.get_by_label("Círculo").click();
    h.run();
    let circulo = objeto(&h, "circulo").clone();
    assert!(
        perto(circulo.x, wx, 1.0) && perto(circulo.y, wy, 1.0),
        "círculo em ({}, {}), clique em ({wx}, {wy})",
        circulo.x,
        circulo.y
    );

    // Com o menu aberto, o botão direito em outro lugar abre o menu ali.
    clique_direito(&mut h, rect.left_top() + egui::vec2(40.0, 40.0));
    h.get_by_label("Criar aqui");
    let q0 = objeto(&h, "quadrado").clone();
    let pq0 = cena(&h).camera.to_screen(rect, q0.x, q0.y);
    clique_direito(&mut h, pq0);
    h.run_steps(2);
    h.get_by_label_contains("Renomear");
    assert_eq!(
        cena(&h).selection,
        vec![q0.id],
        "o menu reabriu no quadrado"
    );
    h.key_press(Key::Escape);
    h.run();
    // Arrastar com o direito move a vista, sem menu.
    let antes = cena(&h).camera.x;
    arrastar_direito(&mut h, p, p + egui::vec2(-100.0, 0.0));
    assert!(h.query_by_label("Criar aqui").is_none());
    assert_ne!(cena(&h).camera.x, antes, "a vista andou");

    // No quadrado: o menu dele. Excluir tira, Ctrl+Z traz de volta.
    let q = objeto(&h, "quadrado").clone();
    let pq = cena(&h).camera.to_screen(rect, q.x, q.y);
    clique_direito(&mut h, pq);
    h.get_by_label_contains("Excluir").click();
    // (O mouse parou em cima do aviso "apagado", que espera: sem h.run().)
    h.run_steps(3);
    assert!(cena(&h).data.objects.iter().all(|o| o.name != "quadrado"));
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run_steps(3);
    objeto(&h, "quadrado");
    h.hover_at(rect.center());
    h.run_steps(30);

    // Hierarquia: o botão direito no vazio cria (no meio da vista).
    let painel = nn_studio::tutorial::anchor_rect(&h.ctx, "hierarquia").expect("hierarquia");
    clique_direito(&mut h, painel.left_bottom() + egui::vec2(40.0, -30.0));
    h.get_by_label("Texto").click();
    h.run();
    objeto(&h, "texto");

    // Hierarquia: ➕ Criar dentro do título.
    h.get_by_label("titulo").click_secondary();
    h.run();
    submenu(&mut h, "➕ Criar dentro");
    h.get_by_label("Retângulo").click();
    h.run();
    let titulo = objeto(&h, "titulo").clone();
    assert_eq!(titulo.children.len(), 1);
    assert_eq!(titulo.children[0].name, "retangulo");
    assert_eq!(cena(&h).selection, vec![titulo.children[0].id]);

    // Inspetor: botão direito no nome do componente > Restaurar o padrão.
    let id = q.id;
    {
        let doc = cena_mut(&mut h);
        doc.selection = vec![id];
        let c = &mut doc.data.find_mut(id).unwrap().components[0];
        c.set("color", nn_studio::scene_data::Prop::Str("#FF0000".into()));
        c.set("shape", nn_studio::scene_data::Prop::Str("circle".into()));
    }
    h.run();
    h.get_all_by_label("Forma")
        .next()
        .unwrap()
        .click_secondary();
    h.run();
    h.get_by_label("↺ Restaurar o padrão").click();
    h.run();
    let c = objeto(&h, "quadrado").components[0].clone();
    let padrao = nn_studio::scene_data::Component::new("shape");
    assert_eq!(c.str("color"), padrao.str("color"), "a cor voltou");
    assert_eq!(c.str("shape"), "circle", "a forma continua");
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    assert_eq!(objeto(&h, "quadrado").components[0].str("color"), "#FF0000");
}

// Menos trabalho repetido (S44) ------------------------------------------------

/// Arrasta segurando Alt.
fn arrastar_com_alt(h: &mut Harness<'_, StudioApp>, from: egui::Pos2, to: egui::Pos2) {
    h.event(egui::Event::ModifiersChanged(Modifiers::ALT));
    h.run();
    arrastar(h, from, to);
    h.event(egui::Event::ModifiersChanged(Modifiers::NONE));
    h.run();
}

fn quantos(h: &Harness<'_, StudioApp>, prefixo: &str) -> usize {
    let mut n = 0;
    nn_studio::scene_data::visit(&cena(h).data.objects, &mut |o| {
        if o.name.starts_with(prefixo) {
            n += 1;
        }
    });
    n
}

/// Cena 2D: Alt+arrastar leva uma cópia; o Ctrl+D seguinte repete o passo;
/// cada um desfaz de uma vez.
#[test]
fn duplicar_arrastando_e_ctrl_d_repete_o_passo() {
    let dir = temp("duplicar-2d");
    let proj = plataforma(&dir);
    let gpu = std::env::var_os("NN_REQUIRE_GPU").is_some();
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), gpu);
    com_cena(&mut h, &proj);
    let rect = h.get_by_label("Vista da cena").rect();
    let q = objeto(&h, "quadrado").clone();
    let from = cena(&h).camera.to_screen(rect, q.x, q.y);
    let to = from + egui::vec2(120.0, 0.0);
    arrastar_com_alt(&mut h, from, to);
    assert_eq!(quantos(&h, "quadrado"), 2, "Alt+arrastar duplicou");
    assert_eq!(objeto(&h, "quadrado").x, q.x, "o original ficou");
    let copia = cena(&h).selection[0];
    let passo = cena(&h).data.find(copia).unwrap().x - q.x;
    assert!(passo > 20.0, "a cópia andou: {passo}");
    // Ctrl+D: a próxima vai o mesmo tanto adiante.
    h.hover_at(rect.center());
    h.run();
    atalho(&mut h, Modifiers::COMMAND, Key::D);
    assert_eq!(quantos(&h, "quadrado"), 3);
    let terceira = cena(&h).selection[0];
    let x3 = cena(&h).data.find(terceira).unwrap().x;
    let x2 = cena(&h).data.find(copia).unwrap().x;
    assert!(
        (x3 - x2 - passo).abs() < 0.01,
        "passo repetido: {x2} → {x3}"
    );
    // De novo.
    atalho(&mut h, Modifiers::COMMAND, Key::D);
    assert_eq!(quantos(&h, "quadrado"), 4);
    // Cada duplicação desfaz de uma vez (o Alt+arrastar também).
    for n in [3, 2, 1] {
        atalho(&mut h, Modifiers::COMMAND, Key::Z);
        assert_eq!(quantos(&h, "quadrado"), n);
    }
}

/// Cena 3D: Shift+D leva a cópia com o mouse até o clique; Esc desiste.
/// Alt+arrastar a seta do Mover leva uma cópia; Alt+arrastar no vazio só
/// gira a câmera.
#[test]
fn shift_d_e_alt_arrastar_na_cena_3d() {
    let dir = temp("duplicar-3d");
    let proj = plataforma(&dir);
    let gpu = std::env::var_os("NN_REQUIRE_GPU").is_some();
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), gpu);
    com_cena3d(&mut h, &proj);
    h.run();
    let cubo = objeto(&h, "Cubo").clone();
    cena_mut(&mut h).selection = vec![cubo.id];
    h.run();
    let a = na_vista3d(&h, glam::Vec3::new(0.0, 0.5, 0.0));
    let b = na_vista3d(&h, glam::Vec3::new(3.0, 0.5, -2.0));
    h.hover_at(a);
    h.run();
    h.key_press_modifiers(Modifiers::SHIFT, Key::D);
    h.run();
    assert!(cena(&h).carrying(), "a cópia segue o mouse");
    assert_eq!(quantos(&h, "Cubo"), 2);
    h.hover_at(b);
    h.run();
    tela(&mut h, "shift-d");
    clique(&mut h, b);
    assert!(!cena(&h).carrying(), "o clique soltou");
    let copia = cena(&h).selection[0];
    let c = cena(&h).data.find(copia).unwrap().clone();
    assert!(
        perto(c.x, 3.0, 0.2) && perto(c.z, -2.0, 0.2) && perto(c.y, cubo.y, 1e-6),
        "a cópia ficou em ({}, {}, {})",
        c.x,
        c.y,
        c.z
    );
    assert_eq!(
        cena(&h).selection,
        vec![copia],
        "o clique não trocou a seleção"
    );
    // Esc desiste: a cópia some.
    h.hover_at(a);
    h.run();
    h.key_press_modifiers(Modifiers::SHIFT, Key::D);
    h.run();
    assert_eq!(quantos(&h, "Cubo"), 3);
    h.hover_at(b);
    h.run();
    h.key_press(Key::Escape);
    h.run();
    assert!(!cena(&h).carrying());
    assert_eq!(quantos(&h, "Cubo"), 2, "Esc tirou a cópia");

    // Alt + arrastar a seta X do Mover: uma cópia andando em X.
    cena_mut(&mut h).selection = vec![cubo.id];
    h.run();
    let centro = glam::Vec3::new(cubo.x as f32, cubo.y as f32, cubo.z as f32);
    let len = (centro - cena(&h).cam3.pos).length() * 0.16;
    let s0 = na_vista3d(&h, centro + glam::Vec3::X * len * 0.75);
    let s1 = na_vista3d(&h, centro + glam::Vec3::X * len * 2.0);
    arrastar_com_alt(&mut h, s0, s1);
    assert_eq!(quantos(&h, "Cubo"), 3, "Alt + seta duplicou");
    assert_eq!(objeto(&h, "Cubo").x, cubo.x, "o original ficou");
    let nova = cena(&h).data.find(cena(&h).selection[0]).unwrap().clone();
    assert!(nova.x > cubo.x + 0.3 && perto(nova.z, cubo.z, 1e-6));
    // Alt + arrastar no vazio: gira a câmera, sem copiar.
    let antes = cena(&h).cam3;
    let rect = h.get_by_label("Vista da cena 3D").rect();
    arrastar_com_alt(
        &mut h,
        rect.left_top() + egui::vec2(60.0, 60.0),
        rect.left_top() + egui::vec2(140.0, 60.0),
    );
    assert_eq!(quantos(&h, "Cubo"), 3);
    assert_ne!(cena(&h).cam3.yaw, antes.yaw);
}

/// Ctrl+C / Ctrl+V cola onde o mouse está; Ctrl+X recorta; colar em outra
/// cena; o menu do botão direito tem Colar aqui.
#[test]
fn copiar_recortar_e_colar_objetos() {
    let dir = temp("copiar-colar");
    let proj = plataforma(&dir);
    let gpu = std::env::var_os("NN_REQUIRE_GPU").is_some();
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), gpu);
    com_cena(&mut h, &proj);
    let rect = h.get_by_label("Vista da cena").rect();
    let q = objeto(&h, "quadrado").clone();
    cena_mut(&mut h).selection = vec![q.id];
    h.hover_at(rect.center());
    h.run();
    h.event(egui::Event::Copy);
    h.run();
    let texto = nn_studio::scene_view::SceneDoc::copied_objects(&h.ctx).expect("copiou");
    // Colar com o mouse em outro ponto: o quadrado vai para lá.
    let p = rect.center() + egui::vec2(150.0, -90.0);
    let (wx, wy) = cena(&h).camera.to_world(rect, p);
    h.hover_at(p);
    h.run();
    h.event(egui::Event::Paste(texto.clone()));
    h.run();
    assert_eq!(quantos(&h, "quadrado"), 2);
    let colado = cena(&h).data.find(cena(&h).selection[0]).unwrap().clone();
    assert_ne!(colado.id, q.id);
    assert!(
        perto(colado.x, wx, 1.0) && perto(colado.y, wy, 1.0),
        "colado em ({}, {}), mouse em ({wx}, {wy})",
        colado.x,
        colado.y
    );
    assert_eq!(colado.components, q.components);
    // Ctrl+X recorta o colado; Ctrl+Z traz de volta.
    h.event(egui::Event::Cut);
    h.run();
    assert_eq!(quantos(&h, "quadrado"), 1);
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    assert_eq!(quantos(&h, "quadrado"), 2);
    // Botão direito no vazio > Colar aqui.
    let p2 = rect.center() + egui::vec2(-160.0, 100.0);
    clique_direito(&mut h, p2);
    h.get_by_label_contains("Colar aqui").click();
    h.run_steps(3);
    assert_eq!(quantos(&h, "quadrado"), 3);
    h.hover_at(rect.center());
    h.run_steps(30);
    // Em outra cena 2D: colado no mesmo lugar (pela Hierarquia não tem mouse).
    std::fs::write(
        proj.join("cenas/outra.cena"),
        nn_studio::files::visual_scene_template(),
    )
    .unwrap();
    {
        let ws = h.state_mut().project_mut().unwrap();
        ws.files.rescan();
        ws.open_file("cenas/outra.cena", None);
    }
    h.run();
    let outra = {
        let ws = h.state_mut().project_mut().unwrap();
        let doc = ws.scenes.get_mut("cenas/outra.cena").unwrap();
        doc.data.objects.retain(|o| o.name != "quadrado");
        doc.paste_objects(&texto, &proj, None);
        doc.data.objects.clone()
    };
    let q2 = outra
        .iter()
        .find(|o| o.name == "quadrado")
        .expect("colou na outra cena");
    assert_eq!((q2.x, q2.y), (q.x, q.y));
    // Objetos 2D não colam numa cena 3D.
    std::fs::write(
        proj.join("cenas/mundo.cena"),
        nn_studio::files::scene3d_template(),
    )
    .unwrap();
    let mut doc3 = nn_studio::scene_view::SceneDoc::open(&proj, "cenas/mundo.cena");
    assert!(doc3.paste_objects(&texto, &proj, None).is_empty());
    assert!(doc3.paste_objects("print('oi')", &proj, None).is_empty());
}

/// Vários selecionados: o Inspetor mostra o que todos têm, marca o que
/// difere e muda em todos; componentes copiados colam em todos.
#[test]
fn inspetor_com_varios_objetos_e_componentes_copiados() {
    use nn_studio::scene_data::{Component, Object, Prop};
    let dir = temp("varios");
    let proj = plataforma(&dir);
    let gpu = std::env::var_os("NN_REQUIRE_GPU").is_some();
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), gpu);
    com_cena3d(&mut h, &proj);
    h.run();
    let cores = ["#FF0000", "#00FF00", "#0000FF"];
    {
        let doc = cena_mut(&mut h);
        doc.data
            .objects
            .retain(|o| o.component("camera3d").is_some());
        for (i, cor) in cores.iter().enumerate() {
            let mut o = Object::new(10 + i as u32, &format!("bloco{i}"));
            o.x = i as f64 * 2.0;
            let mut c = Component::new("shape3d");
            c.set("color", Prop::Str((*cor).into()));
            o.components.push(c);
            if i == 2 {
                // Só o último tem uma luz: ela não aparece com os três.
                o.components.push(Component::new("light3d"));
            }
            doc.data.objects.push(o);
        }
        doc.selection = vec![10, 11, 12];
    }
    h.run();
    h.get_by_label("3 objetos");
    h.get_by_label("Posição ≠");
    h.get_by_label("Cor ≠");
    assert!(h.query_by_label("Luz").is_none(), "a luz não é de todos");
    tela(&mut h, "inspetor-varios");
    // Visível: desmarcar esconde os três (um passo de desfazer).
    h.get_by_label("Visível").click();
    h.run();
    assert!(
        cena(&h)
            .data
            .objects
            .iter()
            .filter(|o| o.id >= 10)
            .all(|o| !o.visible)
    );
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    assert!(
        cena(&h)
            .data
            .objects
            .iter()
            .filter(|o| o.id >= 10)
            .all(|o| o.visible)
    );
    // Restaurar o padrão da Forma 3D: a cor volta nos três; a forma fica.
    h.get_all_by_label("Forma 3D")
        .next()
        .unwrap()
        .click_secondary();
    h.run();
    h.get_by_label("↺ Restaurar o padrão").click();
    h.run();
    let padrao = Component::new("shape3d").str("color").to_string();
    for o in cena(&h).data.objects.iter().filter(|o| o.id >= 10) {
        assert_eq!(o.components[0].str("color"), padrao, "{}", o.name);
    }
    h.get_by_label("Cor");
    // Copiar a luz do bloco2 e colar como componente novo nos dois outros.
    cena_mut(&mut h).selection = vec![12];
    h.run();
    h.get_all_by_label("Luz").next().unwrap().click_secondary();
    h.run();
    h.get_by_label("📋 Copiar componente").click();
    h.run();
    cena_mut(&mut h).selection = vec![10, 11];
    h.run();
    h.get_by_label_contains("Colar componente").click();
    h.run();
    for id in [10, 11] {
        let o = cena(&h).data.find(id).unwrap();
        assert!(o.component("light3d").is_some(), "{} ganhou a luz", o.name);
    }
    // Colar valores: a cor do bloco0 vira a do bloco1.
    {
        let doc = cena_mut(&mut h);
        doc.data.find_mut(10).unwrap().components[0].set("color", Prop::Str("#123456".into()));
        doc.selection = vec![10];
    }
    h.run();
    h.get_all_by_label("Forma 3D")
        .next()
        .unwrap()
        .click_secondary();
    h.run();
    h.get_by_label("📋 Copiar componente").click();
    h.run();
    cena_mut(&mut h).selection = vec![11];
    h.run();
    h.get_all_by_label("Forma 3D")
        .next()
        .unwrap()
        .click_secondary();
    h.run();
    h.get_by_label("📋 Colar valores").click();
    h.run();
    assert_eq!(
        cena(&h).data.find(11).unwrap().components[0].str("color"),
        "#123456"
    );
}

/// Favoritos: guardar pelo botão direito; criar de novo pelo ➕ Objeto e
/// pelo Shift+A; cada cópia fica livre.
#[test]
fn favoritos_do_projeto() {
    let dir = temp("favoritos");
    let proj = plataforma(&dir);
    let gpu = std::env::var_os("NN_REQUIRE_GPU").is_some();
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), gpu);
    com_cena(&mut h, &proj);
    {
        let doc = cena_mut(&mut h);
        let q = doc
            .data
            .objects
            .iter_mut()
            .find(|o| o.name == "quadrado")
            .unwrap();
        q.components[0].set("color", nn_studio::scene_data::Prop::Str("#FF00FF".into()));
    }
    h.run();
    h.get_by_label("quadrado").click_secondary();
    h.run();
    h.get_by_label("⭐ Guardar nos favoritos").click();
    h.run();
    assert!(proj.join("favoritos/quadrado.favorito").is_file());
    assert!(
        console_text(&h).contains("favoritos"),
        "{}",
        console_text(&h)
    );
    // ➕ Objeto > ⭐ Favoritos > quadrado.
    h.get_by_label("➕ Objeto").click();
    h.run();
    submenu(&mut h, "⭐ Favoritos");
    tela(&mut h, "favoritos");
    // (O da Hierarquia fica à esquerda; o do menu, na cena.)
    h.get_all_by(|n| n.role() == Role::Button && n.label().as_deref() == Some("quadrado"))
        .max_by(|a, b| a.rect().left().total_cmp(&b.rect().left()))
        .unwrap()
        .click();
    h.run();
    assert_eq!(quantos(&h, "quadrado"), 2);
    let novo = cena(&h).data.find(cena(&h).selection[0]).unwrap().clone();
    assert_eq!(novo.components[0].str("color"), "#FF00FF");
    assert!(novo.prefab.is_none(), "favorito não é prefab");
    // Shift+A acha o favorito.
    let rect = h.get_by_label("Vista da cena").rect();
    h.hover_at(rect.center() + egui::vec2(100.0, 0.0));
    h.run();
    atalho(&mut h, Modifiers::SHIFT, Key::A);
    digitar(&mut h, "quadr");
    h.get_by_label("⭐ Favoritos › quadrado");
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(quantos(&h, "quadrado"), 3);
    // Um 3D não vê os favoritos do 2D.
    let fav = nn_studio::catalog::favorites(&proj);
    assert_eq!(fav.len(), 1);
    assert!(!fav[0].three_d);
    // ⭐ Favoritos > 🗑 Tirar dos favoritos: pergunta antes de mandar para a lixeira.
    h.get_by_label("➕ Objeto").click();
    h.run();
    submenu(&mut h, "⭐ Favoritos");
    submenu(&mut h, "🗑 Tirar dos favoritos");
    h.get_all_by(|n| n.role() == Role::Button && n.label().as_deref() == Some("quadrado"))
        .max_by(|a, b| a.rect().left().total_cmp(&b.rect().left()))
        .unwrap()
        .click();
    h.run_steps(3);
    h.get_by_label("Mandar para a lixeira?");
}

// Amigável e divertido (S45) ----------------------------------------------------

fn avisos(h: &Harness<'_, StudioApp>) -> Vec<String> {
    h.state().project().unwrap().toasts.texts()
}

/// Apagar e colar mostram um aviso com Desfazer (que some se a cena mudar
/// depois); salvar mostra o aviso e o ✓ na aba.
#[test]
fn avisos_rapidos_com_desfazer_e_o_salvo() {
    let dir = temp("avisos");
    let proj = plataforma(&dir);
    let gpu = std::env::var_os("NN_REQUIRE_GPU").is_some();
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), gpu);
    com_cena(&mut h, &proj);
    let rect = h.get_by_label("Vista da cena").rect();
    let q = objeto(&h, "quadrado").clone();
    cena_mut(&mut h).selection = vec![q.id];
    h.hover_at(rect.center());
    h.run();
    h.key_press(Key::Delete);
    h.run_steps(3);
    assert!(
        avisos(&h).contains(&"1 objeto apagado".to_string()),
        "{:?} {:?}",
        avisos(&h),
        cena(&h)
            .data
            .objects
            .iter()
            .map(|o| o.name.clone())
            .collect::<Vec<_>>()
    );
    tela(&mut h, "aviso-apagado");
    h.get_by_label("Desfazer").click();
    h.run_steps(3);
    objeto(&h, "quadrado");
    assert!(avisos(&h).is_empty() || !avisos(&h).contains(&"1 objeto apagado".to_string()));
    // Apagar e mexer em outra coisa: o Desfazer some (desfaria a outra).
    cena_mut(&mut h).selection = vec![q.id];
    h.hover_at(rect.center());
    h.run();
    h.key_press(Key::Delete);
    h.run_steps(3);
    let t = objeto(&h, "titulo").id;
    cena_mut(&mut h).selection = vec![t];
    h.hover_at(rect.center());
    h.run();
    h.key_press(Key::ArrowRight);
    h.run_steps(3);
    assert!(
        h.query_by_label("Desfazer").is_none(),
        "a cena mudou depois"
    );
    // Salvar: o aviso e o ✓ na aba. (Sem h.run(): ele roda até o aviso sumir.)
    h.key_press_modifiers(Modifiers::COMMAND, Key::S);
    h.run_steps(2);
    assert!(
        avisos(&h).iter().any(|a| a == "Salvo: teste.cena"),
        "{:?}",
        avisos(&h)
    );
    // (O ✓ na aba não aparece na árvore de acessibilidade: confira na tela.)
    tela(&mut h, "aviso-salvo");
}

/// A cena vazia ensina a começar; o Console vazio tem ▶ Jogar; a primeira
/// vez que o jogo roda é comemorada (só uma vez); o Início tem a dica do dia.
#[test]
fn cena_vazia_console_vazio_comemoracao_e_dica() {
    let dir = temp("vazios");
    let proj = plataforma(&dir);
    let gpu = std::env::var_os("NN_REQUIRE_GPU").is_some();
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), gpu);
    com_cena(&mut h, &proj);
    cena_mut(&mut h).data.objects.clear();
    h.run();
    h.get_by_label("Sua cena está vazia");
    tela(&mut h, "cena-vazia");
    h.get_by_label("🔍 Buscar o que criar").click();
    h.run();
    assert!(cena(&h).quick_add.is_some());
    digitar(&mut h, "retangulo");
    h.key_press(Key::Enter);
    h.run_steps(1);
    tela(&mut h, "brilho-do-novo");
    h.run_steps(3);
    let r = objeto(&h, "retangulo").clone();
    let (cx, cy) = cena(&h).view_center();
    assert!(perto(r.x, cx, 1.0) && perto(r.y, cy, 1.0));
    assert!(h.query_by_label("Sua cena está vazia").is_none());

    // Console vazio: ▶ Jogar.
    {
        let ws = h.state_mut().project_mut().unwrap();
        ws.console.clear();
        ws.show_tab(Tab::Console);
    }
    h.run();
    h.get_all_by_label("▶ Jogar")
        .max_by(|a, b| a.rect().top().total_cmp(&b.rect().top()))
        .unwrap()
        .click();
    h.run_steps(3);
    assert_eq!(h.state().project().unwrap().plays, 1);
    // A comemoração vem quando o jogo abre (o "jogo" falso só existe fora do
    // Windows; lá o jogo não abre e não há o que comemorar).
    if cfg!(unix) {
        let festa = |h: &Harness<'_, StudioApp>| {
            avisos(h)
                .iter()
                .filter(|a| a.contains("rodou pela primeira vez"))
                .count()
        };
        assert_eq!(festa(&h), 1, "{:?}", avisos(&h));
        tela(&mut h, "comemoracao");
        assert!(
            h.state()
                .settings
                .celebrated
                .iter()
                .any(|k| k.starts_with("jogar:"))
        );
        // De novo: sem festa.
        h.state_mut().project_mut().unwrap().toasts = Default::default();
        atalho(&mut h, Modifiers::NONE, Key::F5);
        h.run_steps(3);
        assert_eq!(festa(&h), 0);
    } else {
        assert!(
            !avisos(&h)
                .iter()
                .any(|a| a.contains("rodou pela primeira vez")),
            "sem jogo aberto, sem festa"
        );
    }

    // A dica do dia, no Início.
    h.state_mut().project_mut().unwrap().show_tab(Tab::Home);
    h.run();
    h.get_by_label("💡 Você sabia?");
    tela(&mut h, "dica-do-dia");
    let antes = nn_studio::tips::current(&h.ctx);
    h.get_by_label("Próxima dica ⏵").click();
    h.run();
    assert_ne!(nn_studio::tips::current(&h.ctx), antes);
}

/// Preferências: a cor de destaque muda a seleção; sem animações, os menus
/// abrem na hora; sons e comemorações ligam e desligam.
#[test]
fn preferencias_cor_animacoes_e_sons() {
    let dir = temp("preferencias");
    let proj = plataforma(&dir);
    let gpu = std::env::var_os("NN_REQUIRE_GPU").is_some();
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), gpu);
    paleta(&mut h, "preferencias");
    h.get_by_label("Preferências");
    tela(&mut h, "preferencias");
    h.get_by_label("Roxo").click();
    h.run();
    assert_eq!(h.state().settings.accent, nn_studio::settings::Accent::Roxo);
    let roxo = nn_studio::theme::accent_colors(nn_studio::settings::Accent::Roxo)[0];
    assert_eq!(h.ctx.global_style().visuals.selection.bg_fill, roxo);
    assert!(!h.state().settings.ui_sounds, "sons desligados por padrão");
    h.get_by_label_contains("ao salvar e exportar").click();
    h.run();
    assert!(h.state().settings.ui_sounds);
    h.get_by_label_contains("o brilho dos objetos novos")
        .click();
    h.run();
    assert!(!h.state().settings.animations);
    assert_eq!(h.ctx.global_style().animation_time, 0.0);
    h.get_by_label("Voltar ao padrão").click();
    h.run();
    let s = &h.state().settings;
    assert!(s.animations && !s.ui_sounds && s.accent == nn_studio::settings::Accent::Azul);
    // As preferências ficam gravadas.
    h.get_by_label("Fechar").click();
    h.run();
    let gravado = std::fs::read_to_string(dir.join("preferencias.json")).unwrap();
    assert!(gravado.contains("\"accent\""), "{gravado}");
}

// Modelador (S47) -------------------------------------------------------------

fn modelo<'a>(h: &'a mut Harness<'_, StudioApp>) -> &'a mut nn_studio::model_editor::ModelDoc {
    h.state_mut()
        .project_mut()
        .unwrap()
        .models
        .values_mut()
        .next()
        .expect("um modelo aberto")
}

/// Onde um ponto do modelo aparece na vista do modelador.
fn na_vista_modelo(h: &mut Harness<'_, StudioApp>, p: glam::Vec3) -> egui::Pos2 {
    let rect = h.get_by_label("Vista do modelo").rect();
    let s = modelo(h)
        .cam
        .camera()
        .screen_point(p, glam::Vec2::new(rect.width(), rect.height()))
        .expect("na frente da câmera");
    rect.min + egui::vec2(s.x, s.y)
}

fn novo_modelo(h: &mut Harness<'_, StudioApp>) {
    h.get_by_label("➕ Novo").click();
    h.run_steps(2);
    h.get_by_label("🎲 Modelo 3D").click();
    h.run_steps(2);
    h.key_press(Key::Enter);
    h.run_steps(4);
}

#[test]
fn modelador_puxa_pinta_salva_e_vai_para_a_cena() {
    use nn_studio::model_editor::Select;
    let dir = temp("modelador");
    let proj = plataforma(&dir);
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), true);
    novo_modelo(&mut h);
    assert!(proj.join("assets/modelos/modelo.modelo").is_file());
    assert!(
        proj.join("assets/modelos/modelo.glb").is_file(),
        "o .glb nasce junto"
    );
    assert!(tabs(&h).contains(&Tab::Model("assets/modelos/modelo.modelo".into())));
    h.get_by_label("Vista do modelo");
    assert_eq!(modelo(&mut h).mode, Select::Faces);

    // Clicar na face de cima do cubo escolhe ela.
    let topo = na_vista_modelo(&mut h, glam::Vec3::new(0.0, 1.0, 0.0));
    clicar(&mut h, topo);
    let (peca, face) = *modelo(&mut h).sel_faces.first().expect("face escolhida");
    {
        let d = modelo(&mut h);
        let n = nn_studio::model_data::face_normal(
            &d.data.parts[peca].verts,
            &d.data.parts[peca].faces[face].v,
        );
        assert!(n.y > 0.9, "a de cima");
    }
    tela(&mut h, "modelador-face");

    // P puxa: o mouse sobe e o clique termina. Vira um bloco mais alto.
    h.hover_at(topo);
    h.run();
    atalho(&mut h, Modifiers::NONE, Key::P);
    for i in 1..=5 {
        h.hover_at(topo - egui::vec2(0.0, 12.0 * i as f32));
        h.run();
    }
    tela(&mut h, "modelador-puxando");
    clicar(&mut h, topo - egui::vec2(0.0, 60.0));
    {
        let d = modelo(&mut h);
        assert_eq!(d.data.parts[0].faces.len(), 10, "o bloco ganhou 4 paredes");
        let (_, hi) = d.data.bounds().unwrap();
        assert!(hi.y > 1.2, "mais alto: {hi}");
    }

    // Uma cor da paleta pinta a face escolhida.
    h.get_by_label("#3FA7D6").click();
    h.run();
    assert_eq!(modelo(&mut h).data.parts[0].faces[face].color, "#3FA7D6");

    // Desfazer volta a cor, refazer põe de volta.
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    assert_ne!(modelo(&mut h).data.parts[0].faces[face].color, "#3FA7D6");
    atalho(&mut h, Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
    assert_eq!(modelo(&mut h).data.parts[0].faces[face].color, "#3FA7D6");

    // Botão direito no vazio: ➕ Peça aqui > Cone.
    let rect = h.get_by_label("Vista do modelo").rect();
    clique_direito(&mut h, rect.left_top() + egui::vec2(60.0, 60.0));
    submenu(&mut h, "➕ Peça aqui");
    h.get_by_label("🔺 Cone").click();
    h.run();
    {
        let d = modelo(&mut h);
        assert_eq!(d.data.parts.len(), 2);
        assert_eq!(d.data.parts[1].name, "Cone");
        assert_eq!(d.mode, Select::Parts);
        assert_eq!(d.sel_parts, vec![1]);
    }
    h.get_by_label("Cone");
    tela(&mut h, "modelador-cone");

    // Ctrl+S grava o .modelo e o .glb, que a engine abre com as cores.
    atalho(&mut h, Modifiers::COMMAND, Key::S);
    assert!(!modelo(&mut h).is_dirty());
    let texto = std::fs::read_to_string(proj.join("assets/modelos/modelo.modelo")).unwrap();
    assert!(texto.contains("\"Cone\""), "{texto}");
    let glb = std::fs::read(proj.join("assets/modelos/modelo.glb")).unwrap();
    let lido = nn_render::d3::ModelData::from_gltf(&glb, &mut |_| None).unwrap();
    assert_eq!(lido.triangle_count(), modelo(&mut h).data.triangle_count());

    // Soltar o .modelo numa cena 3D cria o objeto com o .glb.
    com_cena3d(&mut h, &proj);
    let id = {
        let ws = h.state_mut().project_mut().unwrap();
        let root = ws.root.clone();
        let mut sizes = nn_studio::scene_render::Sizes::default();
        let doc = ws.scenes.get_mut("cenas/teste.cena").unwrap();
        let mut maps = std::collections::HashMap::new();
        let mut tilesets = std::collections::HashMap::new();
        let mut paint = nn_studio::paint::PaintState::default();
        let mut env = nn_studio::scene_view::SceneEnv {
            root: &root,
            sizes: &mut sizes,
            renderer: None,
            design: (640.0, 360.0),
            clear_color: "#000000",
            connected: false,
            time: 0.0,
            game: None,
            maps: &mut maps,
            tilesets: &mut tilesets,
            paint: &mut paint,
            animate: false,
        };
        doc.drop_file3(&mut env, "assets/modelos/modelo.modelo", glam::Vec3::ZERO)
            .expect("criou o objeto")
    };
    h.run();
    let o = cena(&h).data.find(id).unwrap();
    assert_eq!(o.name, "modelo");
    assert_eq!(
        o.component("model").unwrap().str("file"),
        "assets/modelos/modelo.glb"
    );
}

#[test]
fn modelador_pontos_espelho_caixa_e_alcas() {
    use nn_studio::model_editor::Select;
    let dir = temp("modelador-pontos");
    let proj = plataforma(&dir);
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), true);
    novo_modelo(&mut h);

    // Espelho X: só a metade da direita fica de verdade.
    h.get_by_label("↔ Espelho X").click();
    h.run();
    {
        let d = modelo(&mut h);
        assert!(d.data.parts[0].mirror);
        assert!(d.data.parts[0].verts.iter().all(|v| v.x >= 0.0));
    }

    let vista = h.get_by_label("Vista do modelo").rect();
    h.hover_at(vista.center() + egui::vec2(0.0, 150.0));
    h.run();
    // 1: pontos. A caixa de seleção pega os quatro de cima (os da metade e os
    // do meio).
    atalho(&mut h, Modifiers::NONE, Key::Num1);
    assert_eq!(modelo(&mut h).mode, Select::Points);
    let a = na_vista_modelo(&mut h, glam::Vec3::new(-0.7, 1.15, -0.7));
    let b = na_vista_modelo(&mut h, glam::Vec3::new(0.7, 0.85, 0.7));
    let caixa = egui::Rect::from_two_pos(a, b);
    let (de, ate) = (
        caixa.left_top() - egui::vec2(20.0, 20.0),
        caixa.right_bottom() + egui::vec2(20.0, 8.0),
    );
    arrastar(&mut h, de, ate);
    let escolhidos = modelo(&mut h).sel_verts.clone();
    {
        let d = modelo(&mut h);
        assert!(!escolhidos.is_empty());
        for &(p, k) in &escolhidos {
            assert!(d.data.parts[p].verts[k as usize].y > 0.99, "só os de cima");
        }
    }
    tela(&mut h, "modelador-pontos");

    // A seta verde (Y) sobe os pontos; os do meio continuam no meio.
    let c = modelo(&mut h).selection_center().unwrap();
    let base = na_vista_modelo(&mut h, c);
    let len = (c - modelo(&mut h).cam.pos).length() * 0.16;
    let ponta = na_vista_modelo(&mut h, c + glam::Vec3::Y * len * 0.8);
    arrastar(&mut h, ponta, ponta - egui::vec2(0.0, 40.0));
    {
        let d = modelo(&mut h);
        let p = &d.data.parts[0];
        let altos: Vec<_> = escolhidos
            .iter()
            .map(|&(_, k)| p.verts[k as usize])
            .collect();
        assert!(altos.iter().all(|v| v.y > 1.1), "subiram: {altos:?}");
        assert!(altos.iter().any(|v| v.x == 0.0), "os do meio ficam no meio");
        let _ = base;
    }
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    assert!(
        modelo(&mut h).data.parts[0]
            .verts
            .iter()
            .all(|v| v.y <= 1.0 + 1e-4)
    );

    // M junta os pontos escolhidos: o cubo vira um telhado.
    h.hover_at(vista.center() + egui::vec2(0.0, 150.0));
    h.run();
    atalho(&mut h, Modifiers::NONE, Key::M);
    {
        let d = modelo(&mut h);
        assert!(
            d.data.parts[0].verts.len() < 8,
            "{} pontos, escolhidos {:?}, modo {:?}",
            d.data.parts[0].verts.len(),
            d.sel_verts,
            d.mode
        );
    }

    // 3: peças. Ctrl+D duplica ao lado; Delete apaga a cópia.
    h.hover_at(vista.center() + egui::vec2(0.0, 150.0));
    h.run();
    atalho(&mut h, Modifiers::NONE, Key::Num3);
    modelo(&mut h).sel_parts = vec![0];
    atalho(&mut h, Modifiers::COMMAND, Key::D);
    {
        let d = modelo(&mut h);
        assert_eq!(d.data.parts.len(), 2);
        assert_eq!(d.data.parts[1].name, "Cubo 2");
        assert!(d.data.parts[1].pos.x > 0.5);
        assert_eq!(d.sel_parts, vec![1]);
    }
    tela(&mut h, "modelador-pecas");
    atalho(&mut h, Modifiers::NONE, Key::Delete);
    assert_eq!(modelo(&mut h).data.parts.len(), 1);

    // Desligar o espelho transforma a metade copiada em faces de verdade.
    modelo(&mut h).sel_parts = vec![0];
    h.get_by_label("↔ Espelho X").click();
    h.run();
    {
        let d = modelo(&mut h);
        assert!(!d.data.parts[0].mirror);
        assert!(d.data.parts[0].verts.iter().any(|v| v.x < 0.0));
    }
}

fn clicar_passos(h: &mut Harness<'_, StudioApp>, p: egui::Pos2) {
    h.hover_at(p);
    h.run_steps(1);
    h.drag_at(p);
    h.run_steps(1);
    h.drop_at(p);
    h.run_steps(2);
}

fn arrastar_passos(h: &mut Harness<'_, StudioApp>, from: egui::Pos2, to: egui::Pos2) {
    h.hover_at(from);
    h.run_steps(1);
    h.drag_at(from);
    h.run_steps(1);
    for i in 1..=6 {
        h.hover_at(from + (to - from) * (i as f32 / 6.0));
        h.run_steps(1);
    }
    h.drop_at(to);
    h.run_steps(2);
}

#[test]
fn tutorial_modele_uma_casinha() {
    use nn_studio::model_editor::Select;
    let dir = temp("tutorial-casinha");
    let mut h = studio_with(&dir, None, fake_game(&dir), true);
    h.state_mut().settings.projects_dir = Some(dir.join("projetos"));
    h.state_mut().start_tutorial("modele-uma-casinha");
    h.run_steps(4);
    let projeto = dir.join("projetos/uma-casinha");
    h.state_mut().tutorial.as_mut().unwrap().pause = std::time::Duration::ZERO;
    h.get_by_label("Próximo").click();
    fn espera(h: &mut Harness<'_, StudioApp>, esperado: usize) {
        h.run_steps(6);
        assert_eq!(
            h.state().tutorial.as_ref().map(|t| t.step),
            Some(esperado),
            "o tutorial devia estar no passo {esperado}"
        );
    }
    espera(&mut h, 1);
    // ➕ Novo > Modelo 3D, com o nome casinha.
    h.get_by_label("➕ Novo").click();
    h.run_steps(2);
    h.get_by_label("🎲 Modelo 3D").click();
    h.run_steps(2);
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.run_steps(2);
    h.event(egui::Event::Text("casinha".into()));
    h.run_steps(2);
    h.key_press(Key::Enter);
    espera(&mut h, 2);
    assert!(projeto.join("assets/modelos/casinha.modelo").is_file());
    assert!(nn_studio::tutorial::anchor_rect(&h.ctx, "modelo").is_some());

    // A parede da frente (+Z): clicar, I, mouse para o meio, clicar.
    let frente = na_vista_modelo(&mut h, glam::Vec3::new(0.0, 0.5, 0.5));
    clicar_passos(&mut h, frente);
    {
        let d = modelo(&mut h);
        assert_eq!(d.mode, Select::Faces);
        let (p, f) = d.sel_faces[0];
        let n =
            nn_studio::model_data::face_normal(&d.data.parts[p].verts, &d.data.parts[p].faces[f].v);
        assert!(n.z > 0.9, "a parede da frente: {n}");
    }
    let canto = na_vista_modelo(&mut h, glam::Vec3::new(0.45, 0.95, 0.5));
    h.hover_at(canto);
    h.run_steps(1);
    atalho_passos(&mut h, Modifiers::NONE, Key::I);
    for i in 1..=4 {
        h.hover_at(canto + (frente - canto) * (i as f32 * 0.15));
        h.run_steps(1);
    }
    clicar_passos(&mut h, canto + (frente - canto) * 0.6);
    espera(&mut h, 3);

    // A porta marrom.
    h.get_by_label("#6B4423").click();
    espera(&mut h, 4);

    // P para dentro da casa.
    let porta = na_vista_modelo(&mut h, glam::Vec3::new(0.0, 0.5, 0.5));
    h.hover_at(porta);
    h.run_steps(1);
    atalho_passos(&mut h, Modifiers::NONE, Key::P);
    for i in 1..=4 {
        h.hover_at(porta - egui::vec2(0.0, -6.0 * i as f32));
        h.run_steps(1);
    }
    clicar_passos(&mut h, porta + egui::vec2(0.0, 24.0));
    espera(&mut h, 5);
    {
        let d = modelo(&mut h);
        let (lo, _) = d.data.bounds().unwrap();
        assert!(lo.z >= -0.5 - 1e-4, "afundou para dentro, não para trás");
    }
    tela(&mut h, "tutorial-casinha-porta");

    // ➕ Peça > Pirâmide.
    h.get_by_label("➕ Peça").click();
    h.run_steps(3);
    h.get_by_label("🏠 Pirâmide").click();
    espera(&mut h, 6);

    // A seta verde leva o telhado para cima.
    let c = modelo(&mut h).selection_center().unwrap();
    let len = (c - modelo(&mut h).cam.pos).length() * 0.16;
    let ponta = na_vista_modelo(&mut h, c + glam::Vec3::Y * len * 0.8);
    let alvo = na_vista_modelo(&mut h, c + glam::Vec3::Y * (len * 0.8 + 1.0));
    arrastar_passos(&mut h, ponta, alvo);
    espera(&mut h, 7);

    // Vermelho na peça inteira.
    h.get_by_label("#E84A4A").click();
    espera(&mut h, 8);
    tela(&mut h, "tutorial-casinha-telhado");

    // Salvar.
    let meio = na_vista_modelo(&mut h, glam::Vec3::ZERO);
    h.hover_at(meio);
    atalho_passos(&mut h, Modifiers::COMMAND, Key::S);
    espera(&mut h, 9);
    assert!(projeto.join("assets/modelos/casinha.glb").is_file());

    // Na cena: soltar o .modelo.
    {
        let ws = h.state_mut().project_mut().unwrap();
        ws.open_file("cenas/mundo.cena", None);
    }
    h.run_steps(3);
    let id = {
        let ws = h.state_mut().project_mut().unwrap();
        let root = ws.root.clone();
        let mut sizes = nn_studio::scene_render::Sizes::default();
        let mut maps = std::collections::HashMap::new();
        let mut tilesets = std::collections::HashMap::new();
        let mut paint = nn_studio::paint::PaintState::default();
        let mut env = nn_studio::scene_view::SceneEnv {
            root: &root,
            sizes: &mut sizes,
            renderer: None,
            design: (640.0, 360.0),
            clear_color: "#000000",
            connected: false,
            time: 0.0,
            game: None,
            maps: &mut maps,
            tilesets: &mut tilesets,
            paint: &mut paint,
            animate: false,
        };
        let doc = ws.scenes.get_mut("cenas/mundo.cena").unwrap();
        doc.drop_file3(
            &mut env,
            "assets/modelos/casinha.modelo",
            glam::Vec3::new(3.0, 0.0, 0.0),
        )
    };
    assert!(id.is_some());
    espera(&mut h, 10);
    tela(&mut h, "tutorial-casinha-cena");
    h.get_by_label("Próximo").click();
    espera(&mut h, 11);
}

// Modelar sem repetir (S48) ---------------------------------------------------

#[test]
fn modelador_repetir_em_linha_e_em_circulo() {
    use nn_studio::model_editor::Select;
    let dir = temp("modelador-repetir");
    let proj = plataforma(&dir);
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), true);
    novo_modelo(&mut h);
    {
        let d = modelo(&mut h);
        d.set_mode(Select::Parts);
        d.sel_parts = vec![0];
    }
    h.run();
    h.get_by_label("🔁 Repetir").click();
    h.run();
    // A prévia: 3 cópias em linha, encostadas (o cubo tem 1 m).
    {
        let d = modelo(&mut h);
        assert_eq!(d.data.parts.len(), 4);
        assert_eq!(d.data.parts[3].name, "Cubo 4");
        assert!(
            (d.data.parts[3].pos.x - 3.0).abs() < 1e-4,
            "{}",
            d.data.parts[3].pos
        );
    }
    tela(&mut h, "modelador-repetir-linha");
    // Em círculo, 5 cópias: 6 cubos na mesma distância do meio.
    h.get_by_label("🔄 Em círculo").click();
    h.run();
    {
        let d = modelo(&mut h);
        assert_eq!(d.data.parts.len(), 4);
    }
    // Esc cancela: volta a ter só o cubo.
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(modelo(&mut h).data.parts.len(), 1);
    // De novo, em círculo, e Aplicar: um passo de desfazer.
    h.get_by_label("🔁 Repetir").click();
    h.run();
    h.get_by_label("🔄 Em círculo").click();
    h.run();
    tela(&mut h, "modelador-repetir-circulo");
    h.get_by_label("✔ Aplicar").click();
    h.run();
    {
        let d = modelo(&mut h);
        assert_eq!(d.data.parts.len(), 4);
        let c = d.data.parts[0].pos;
        let raio = |p: glam::Vec3| ((p.x - (c.x - 1.5)).powi(2) + (p.z - c.z).powi(2)).sqrt();
        for p in &d.data.parts {
            assert!((raio(p.pos) - 1.5).abs() < 0.05, "no círculo: {}", p.pos);
        }
        assert_eq!(d.sel_parts.len(), 4);
    }
    atalho(&mut h, Modifiers::COMMAND, Key::Z);
    assert_eq!(modelo(&mut h).data.parts.len(), 1);
}

#[test]
fn cena_3d_repetir_pelo_menu_do_objeto() {
    let dir = temp("cena-3d-repetir");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    com_cena3d(&mut h, &proj);
    let cubo = objeto(&h, "Cubo").clone();
    let antes = cena(&h).data.objects.len();
    let p = na_vista3d(
        &h,
        glam::Vec3::new(cubo.x as f32, cubo.y as f32, cubo.z as f32),
    );
    clique_direito(&mut h, p);
    h.get_by_label("🔁 Repetir…").click();
    h.run();
    assert_eq!(cena(&h).data.objects.len(), antes + 3, "a prévia");
    h.get_by_label("🔄 Em círculo").click();
    h.run();
    tela(&mut h, "cena-3d-repetir");
    h.get_by_label("✔ Aplicar").click();
    h.run_steps(4);
    assert_eq!(cena(&h).data.objects.len(), antes + 3);
    assert_eq!(cena(&h).selection.len(), 4);
    // Desfazer tira as três de uma vez.
    atalho_passos(&mut h, Modifiers::COMMAND, Key::Z);
    assert_eq!(cena(&h).data.objects.len(), antes);
}

#[test]
fn cena_2d_repetir_em_linha() {
    let dir = temp("cena-2d-repetir");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    com_cena(&mut h, &proj);
    let id = cena(&h).data.objects[0].id;
    let antes = cena(&h).data.objects.len();
    let x0 = cena(&h).data.world_transform(id).unwrap().x;
    {
        let ws = h.state_mut().project_mut().unwrap();
        let doc = ws.scenes.values_mut().next().unwrap();
        doc.selection = vec![id];
        assert!(doc.start_repeat());
    }
    h.run();
    h.get_by_label("✔ Aplicar").click();
    h.run_steps(4);
    let d = cena(&h);
    assert_eq!(d.data.objects.len(), antes + 3);
    let xs: Vec<f64> = d
        .selection
        .iter()
        .map(|i| d.data.world_transform(*i).unwrap().x)
        .collect();
    assert_eq!(xs[0], x0);
    assert!(xs[3] > xs[2] && xs[2] > xs[1] && xs[1] > xs[0], "{xs:?}");
}

#[test]
fn modelador_encaixe_kits_biblioteca_e_variacao() {
    use nn_studio::model_editor::Select;
    let dir = temp("modelador-kits");
    let proj = plataforma(&dir);
    let mut h = studio_with(&dir, Some(proj.clone()), fake_game(&dir), true);
    novo_modelo(&mut h);
    // Um cubo mais largo, para o telhado encaixar.
    modelo(&mut h).data.parts[0].scale = glam::Vec3::new(2.0, 1.0, 1.6);
    h.run();

    // Botão direito na face de cima > ➕ Peça aqui > Casa > Telhado: nasce em
    // cima, do tamanho da face.
    let topo = na_vista_modelo(&mut h, glam::Vec3::new(0.0, 1.0, 0.0));
    clique_direito(&mut h, topo);
    submenu(&mut h, "➕ Peça aqui");
    submenu(&mut h, "🏡 Casa");
    h.get_by_label("Telhado").click();
    h.run();
    {
        let d = modelo(&mut h);
        assert_eq!(d.data.parts.len(), 2);
        let t = &d.data.parts[1];
        assert_eq!(t.name, "Telhado");
        assert!(
            (t.pos - glam::Vec3::new(0.0, 1.0, 0.0)).length() < 1e-3,
            "{}",
            t.pos
        );
        assert!(
            (t.scale.x - 2.0).abs() < 1e-3 && (t.scale.z - 1.6).abs() < 1e-3,
            "{}",
            t.scale
        );
        assert!(t.rot.length() < 1e-3, "{}", t.rot);
    }
    tela(&mut h, "modelador-telhado-encaixado");

    // Numa parede do lado (+X): a peça nasce de lado, virada para fora.
    let lado = na_vista_modelo(&mut h, glam::Vec3::new(1.0, 0.5, 0.0));
    clique_direito(&mut h, lado);
    submenu(&mut h, "➕ Peça aqui");
    h.get_by_label("◎ Cilindro").click();
    h.run();
    {
        let d = modelo(&mut h);
        let c = d.data.parts.last().unwrap();
        assert!(
            (c.pos - glam::Vec3::new(1.0, 0.5, 0.0)).length() < 1e-3,
            "{}",
            c.pos
        );
        let up = c.rotation() * glam::Vec3::Y;
        assert!(up.x > 0.99, "virado para +X: {up}");
    }
    atalho(&mut h, Modifiers::COMMAND, Key::Z);

    // Trocar cor: o laranja do cubo vira azul no modelo todo.
    let laranja = h
        .get_all_by_label("#FF8C42")
        .next()
        .unwrap()
        .rect()
        .center();
    clique_direito(&mut h, laranja);
    submenu(&mut h, "🎨 Trocar por…");
    h.get_by_label("trocar por #3FA7D6").click();
    h.run();
    {
        let d = modelo(&mut h);
        assert!(d.data.parts[0].faces.iter().all(|f| f.color == "#3FA7D6"));
    }

    // Guardar o telhado na biblioteca e pôr de novo pelo menu.
    {
        let d = modelo(&mut h);
        d.set_mode(Select::Parts);
        d.sel_parts = vec![1];
    }
    h.run();
    h.get_by_label("📚 Biblioteca").click();
    h.run();
    h.get_by_label("⭐ Guardar a peça na biblioteca…").click();
    h.run_steps(3);
    h.key_press(Key::Enter);
    h.run_steps(3);
    let guardado = dir.join("biblioteca/Telhado.modelo");
    assert!(guardado.is_file(), "guardado ao lado das preferências");
    h.get_by_label("➕ Peça").click();
    h.run();
    submenu(&mut h, "📚 Biblioteca");
    h.get_by_label("⭐ Telhado").click();
    h.run();
    assert_eq!(modelo(&mut h).data.parts.len(), 3);
    assert_eq!(modelo(&mut h).data.parts[2].name, "Telhado 2");

    // Um modelo pronto da biblioteca, como peças.
    h.get_by_label("➕ Peça").click();
    h.run();
    submenu(&mut h, "📚 Biblioteca");
    h.get_by_label("Árvore").click();
    h.run();
    assert_eq!(modelo(&mut h).data.parts.len(), 5);

    // 🔀 Criar variação: grava "modelo 2" com o .glb e abre.
    h.get_by_label("📚 Biblioteca").click();
    h.run();
    h.get_by_label("🔀 Criar variação").click();
    h.run_steps(4);
    assert!(proj.join("assets/modelos/modelo 2.modelo").is_file());
    assert!(proj.join("assets/modelos/modelo 2.glb").is_file());
    assert!(tabs(&h).contains(&Tab::Model("assets/modelos/modelo 2.modelo".into())));
}

#[test]
fn novo_modelo_a_partir_de_um_pronto() {
    let dir = temp("modelo-pronto");
    let proj = plataforma(&dir);
    let mut h = studio(&dir, Some(proj.clone()));
    h.get_by_label("➕ Novo").click();
    h.run();
    h.get_by_label("🎲 Modelo 3D").click();
    h.run();
    h.get_by(|n| {
        n.value().is_some_and(|v| v == "Um cubo") || n.label().is_some_and(|l| l == "Um cubo")
    })
    .click();
    h.run();
    h.get_by_label("Cerca").click();
    h.run();
    h.get_by_label("Criar").click();
    h.run();
    let texto = std::fs::read_to_string(proj.join("assets/modelos/cerca.modelo")).unwrap();
    let m = nn_studio::model_data::ModelData::from_lua(&texto, "cerca.modelo").unwrap();
    assert_eq!(m.parts.len(), 5, "três estacas e duas ripas");
    assert!(proj.join("assets/modelos/cerca.glb").is_file());
}

#[test]
fn tutorial_uma_cerca_e_uma_fogueira() {
    let dir = temp("tutorial-fogueira");
    let mut h = studio_with(&dir, None, fake_game(&dir), true);
    h.state_mut().settings.projects_dir = Some(dir.join("projetos"));
    h.state_mut().start_tutorial("uma-cerca-e-uma-fogueira");
    h.run_steps(4);
    let projeto = dir.join("projetos/uma-cerca-e-uma-fogueira");
    h.state_mut().tutorial.as_mut().unwrap().pause = std::time::Duration::ZERO;
    h.get_by_label("Próximo").click();
    fn espera(h: &mut Harness<'_, StudioApp>, esperado: usize) {
        h.run_steps(6);
        assert_eq!(
            h.state().tutorial.as_ref().map(|t| t.step),
            Some(esperado),
            "o tutorial devia estar no passo {esperado}"
        );
    }
    fn novo_de(h: &mut Harness<'_, StudioApp>, pronto: &str) {
        h.get_by_label("➕ Novo").click();
        h.run_steps(2);
        h.get_by_label("🎲 Modelo 3D").click();
        h.run_steps(6);
        h.get_by(|n| {
            n.value().is_some_and(|v| v == "Um cubo") || n.label().is_some_and(|l| l == "Um cubo")
        })
        .click();
        h.run_steps(3);
        h.get_by_label(pronto).click();
        h.run_steps(2);
        h.key_press(Key::Enter);
        h.run_steps(4);
    }
    espera(&mut h, 1);
    novo_de(&mut h, "Fogueira");
    espera(&mut h, 2);
    assert!(projeto.join("assets/modelos/fogueira.modelo").is_file());

    // Botão direito no chão, a um passo da fogueira: Natureza > Pedra.
    let chao = na_vista_modelo(&mut h, glam::Vec3::new(1.5, 0.0, 0.0));
    h.hover_at(chao);
    h.run_steps(1);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: chao,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        h.step();
    }
    h.run_steps(3);
    for nome in ["➕ Peça aqui", "🌳 Natureza"] {
        h.get_by(|n| n.label().is_some_and(|l| l.starts_with(nome) && l != nome))
            .hover();
        h.run_steps(3);
    }
    h.get_by_label("Pedra").click();
    espera(&mut h, 3);

    // 🔁 Repetir > Em círculo > em volta do meio > 7 cópias > Aplicar.
    h.get_by_label("🔁 Repetir").click();
    h.run_steps(2);
    h.get_by_label("🔄 Em círculo").click();
    h.run_steps(2);
    h.get_by_label("Em volta do meio do modelo").click();
    h.run_steps(2);
    {
        let ws = h.state_mut().project_mut().unwrap();
        let d = ws.models.values_mut().next().unwrap();
        // (O número de cópias, como se tivesse digitado.)
        d.set_repeat_count(7);
    }
    h.run_steps(2);
    tela(&mut h, "tutorial-fogueira-pedras");
    h.get_by_label("✔ Aplicar").click();
    espera(&mut h, 4);
    {
        let d = modelo(&mut h);
        let pedras: Vec<_> = d
            .data
            .parts
            .iter()
            .filter(|p| p.name.starts_with("Pedra"))
            .collect();
        assert_eq!(pedras.len(), 8);
        let r0 = glam::Vec2::new(pedras[0].pos.x, pedras[0].pos.z).length();
        for p in &pedras {
            let r = glam::Vec2::new(p.pos.x, p.pos.z).length();
            assert!((r - r0).abs() < 0.05, "em volta da fogueira: {r} {r0}");
        }
    }
    let meio = na_vista_modelo(&mut h, glam::Vec3::ZERO);
    h.hover_at(meio);
    atalho_passos(&mut h, Modifiers::COMMAND, Key::S);
    espera(&mut h, 5);
    assert!(projeto.join("assets/modelos/fogueira.glb").is_file());

    novo_de(&mut h, "Cerca");
    espera(&mut h, 6);
    assert!(projeto.join("assets/modelos/cerca.glb").is_file());

    // Na cena: a fogueira e a cerca.
    {
        let ws = h.state_mut().project_mut().unwrap();
        ws.open_file("cenas/mundo.cena", None);
    }
    h.run_steps(3);
    let cerca = {
        let ws = h.state_mut().project_mut().unwrap();
        let root = ws.root.clone();
        let mut sizes = nn_studio::scene_render::Sizes::default();
        let mut maps = std::collections::HashMap::new();
        let mut tilesets = std::collections::HashMap::new();
        let mut paint = nn_studio::paint::PaintState::default();
        let mut env = nn_studio::scene_view::SceneEnv {
            root: &root,
            sizes: &mut sizes,
            renderer: None,
            design: (640.0, 360.0),
            clear_color: "#000000",
            connected: false,
            time: 0.0,
            game: None,
            maps: &mut maps,
            tilesets: &mut tilesets,
            paint: &mut paint,
            animate: false,
        };
        let doc = ws.scenes.get_mut("cenas/mundo.cena").unwrap();
        doc.drop_file3(&mut env, "assets/modelos/fogueira.modelo", glam::Vec3::ZERO)
            .unwrap();
        doc.drop_file3(
            &mut env,
            "assets/modelos/cerca.modelo",
            glam::Vec3::new(-3.0, 0.0, 4.0),
        )
        .unwrap()
    };
    espera(&mut h, 7);

    // Botão direito na cerca > Repetir… > Aplicar.
    {
        let ws = h.state_mut().project_mut().unwrap();
        let doc = ws.scenes.get_mut("cenas/mundo.cena").unwrap();
        doc.selection = vec![cerca];
        assert!(doc.start_repeat());
    }
    h.run_steps(3);
    tela(&mut h, "tutorial-fogueira-cerca");
    h.get_by_label("✔ Aplicar").click();
    espera(&mut h, 8);
}
