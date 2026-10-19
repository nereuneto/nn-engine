//! Gerenciador de projetos: a primeira tela do NN Studio, com os projetos
//! recentes, "Novo projeto", "Abrir projeto" e os exemplos.

use std::path::{Path, PathBuf};

use eframe::egui::{self, Color32, RichText};
use nn_run::Template;

use crate::commands::Command;
use crate::examples::EXAMPLES;
use crate::settings::{self, Mode, Settings, ThemeChoice};
use crate::theme;

#[derive(Debug, Clone, PartialEq)]
pub enum LauncherAction {
    Open(PathBuf),
    Command(Command),
    /// Criar o projeto de um tutorial e começar.
    StartTutorial(&'static str),
}

#[derive(Debug)]
struct NewProjectForm {
    title: String,
    parent: String,
    template: Template,
    error: Option<String>,
    focus: bool,
    /// Clique duplo num modelo: cria na hora.
    submit: bool,
}

/// Aviso de versão nova (se a última consulta achou uma mais nova que esta
/// e a pessoa não pediu para esquecer dela).
fn update_banner(ui: &mut egui::Ui, settings: &mut Settings) {
    let Some(latest) = settings.latest_version.clone() else {
        return;
    };
    if !settings.check_updates
        || !crate::update::is_newer(&latest, env!("CARGO_PKG_VERSION"))
        || settings.skipped_version.as_deref() == Some(latest.as_str())
    {
        return;
    }
    let url = settings
        .latest_url
        .clone()
        .unwrap_or_else(|| crate::update::RELEASES_PAGE.to_string());
    egui::Frame::new()
        .fill(theme::PLAY.gamma_multiply(0.25))
        .inner_margin(10.0)
        .corner_radius(6.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width().min(760.0));
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("🎉 Saiu o NN Studio {latest}")).strong());
                ui.weak(format!("(você tem a {})", env!("CARGO_PKG_VERSION")));
                ui.hyperlink_to("Ver o que mudou e baixar", url);
                if ui
                    .small_button("Agora não")
                    .on_hover_text("Não avisa mais desta versão")
                    .clicked()
                {
                    settings.skipped_version = Some(latest.clone());
                }
            });
        });
    ui.add_space(10.0);
}

/// Um cartão de modelo: miniatura e nome. Clique escolhe.
fn template_card(
    ui: &mut egui::Ui,
    name: &str,
    thumb: Option<&egui::TextureHandle>,
    selected: bool,
) -> egui::Response {
    let size = egui::vec2(176.0, 99.0);
    let (rect, r) = ui.allocate_exact_size(size + egui::vec2(0.0, 24.0), egui::Sense::click());
    let painter = ui.painter_at(rect.expand(3.0));
    let img = egui::Rect::from_min_size(rect.min, size);
    match thumb {
        Some(t) => {
            painter.image(
                t.id(),
                img,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        None => {
            painter.rect_filled(img, 4.0, ui.visuals().faint_bg_color);
        }
    }
    let stroke = if selected {
        egui::Stroke::new(3.0, theme::ACCENT)
    } else if r.hovered() {
        egui::Stroke::new(1.5, ui.visuals().widgets.hovered.fg_stroke.color)
    } else {
        egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
    };
    painter.rect_stroke(img, 4.0, stroke, egui::StrokeKind::Outside);
    let text = if selected {
        RichText::new(name).strong().color(theme::ACCENT)
    } else {
        RichText::new(name).strong()
    };
    let galley = egui::WidgetText::from(text).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        size.x,
        egui::TextStyle::Body,
    );
    let pos = egui::pos2(rect.center().x - galley.size().x / 2.0, img.bottom() + 4.0);
    painter.galley(pos, galley, ui.visuals().text_color());
    r.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::RadioButton, true, selected, name)
    });
    r
}

#[derive(Default)]
pub struct Launcher {
    new_form: Option<NewProjectForm>,
    examples: bool,
    /// Mensagem de erro mostrada no topo (projeto inválido, por exemplo).
    pub error: Option<String>,
    /// Miniaturas dos modelos (carregadas na primeira vez).
    thumbs: Vec<(Template, egui::TextureHandle)>,
    /// Janela com a lista dos tutoriais.
    tutorials: bool,
}

impl std::fmt::Debug for Launcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Launcher")
            .field("error", &self.error)
            .finish_non_exhaustive()
    }
}

/// Miniatura de cada modelo (uma imagem do jogo que ele cria).
fn thumb_png(t: Template) -> &'static [u8] {
    match t {
        Template::Padrao => include_bytes!("../assets/modelos/padrao.png"),
        Template::Visual => include_bytes!("../assets/modelos/visual.png"),
        Template::Aventura => include_bytes!("../assets/modelos/aventura.png"),
        Template::Nave => include_bytes!("../assets/modelos/nave.png"),
        Template::Vazio => include_bytes!("../assets/modelos/vazio.png"),
        Template::Minimo => include_bytes!("../assets/modelos/minimo.png"),
        Template::TresD => include_bytes!("../assets/modelos/3d.png"),
        Template::Visual3D => include_bytes!("../assets/modelos/visual3d.png"),
        Template::Vazio3D => include_bytes!("../assets/modelos/vazio3d.png"),
        Template::Vale3D => include_bytes!("../assets/modelos/vale3d.png"),
    }
}

/// Descrição curta de cada modelo.
fn template_info(t: Template) -> (&'static str, &'static str) {
    match t {
        Template::Padrao => (
            "Em código",
            "Um jogo pequeno e completo escrito em Lua: título com menu, partida, pausa, fim e recorde salvo.",
        ),
        Template::Visual => (
            "Plataforma",
            "Fase de plataforma montada no editor, sem código: herói, inimigo, moedas, HUD com corações, pausa com Esc, opções de volume e uma saída que só abre com todas as moedas.",
        ),
        Template::Aventura => (
            "Aventura",
            "Visto de cima, sem código: pegue a chave, abra a porta e ache o tesouro, fugindo das lesmas e dos morcegos.",
        ),
        Template::Nave => (
            "Nave",
            "Atire nos asteroides, sem código: explosões de partículas, pontos, vidas e recorde salvo no computador.",
        ),
        Template::Vazio => (
            "Vazio",
            "Uma cena vazia e um pacote de imagens e sons para montar do seu jeito (o tutorial começa daqui).",
        ),
        Template::Minimo => (
            "Mínimo",
            "Só uma cena com um quadrado que anda. Para quem quer começar do zero.",
        ),
        Template::TresD => (
            "3D em código",
            "Plataforma 3D em terceira pessoa, em código Lua: ache as 5 estrelas da ilha. Câmera com o mouse, pulo, física, plataformas que andam, menu, pausa e recorde.",
        ),
        Template::Visual3D => (
            "3D",
            "A ilha das estrelas montada no editor, sem código: herói em terceira pessoa, caixotes que caem, plataformas e elevador que andam, moedas, estrelas e um evento de vitória.",
        ),
        Template::Vale3D => (
            "Vale 3D",
            "Um vale pintado num Terreno, sem código: morros, lago, trilha, árvores espalhadas, estrelas nos morros e uma fogueira. Mude o terreno com o ✏ Pintar terreno.",
        ),
        Template::Vazio3D => (
            "Vazio 3D",
            "Um mundo 3D com chão e um pacote de modelos e sons para montar do seu jeito (o tutorial 3D começa daqui).",
        ),
    }
}

impl Launcher {
    /// Mostra a lista de tutoriais.
    pub fn show_tutorials(&mut self) {
        self.tutorials = true;
    }

    fn tutorials_dialog(
        &mut self,
        ctx: &egui::Context,
        settings: &Settings,
    ) -> Option<LauncherAction> {
        if !self.tutorials {
            return None;
        }
        let mut action = None;
        let mut close = false;
        let r = egui::Modal::new(egui::Id::new("tutoriais")).show(ctx, |ui| {
            ui.set_width(480.0);
            ui.heading("🎓 Tutoriais");
            ui.weak(format!(
                "Cada tutorial cria um projeto novo em {} e mostra o que fazer, passo a passo.",
                settings::display_path(&settings.default_projects_dir())
            ));
            ui.add_space(8.0);
            for t in crate::tutorial::all() {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t.title).size(17.0).strong());
                        ui.weak(format!("cerca de {} minutos", t.minutes));
                    });
                    ui.label(t.summary);
                    if ui
                        .add(egui::Button::new("Começar").fill(theme::ACCENT.gamma_multiply(0.75)))
                        .clicked()
                    {
                        action = Some(LauncherAction::StartTutorial(t.id));
                        close = true;
                    }
                });
                ui.add_space(4.0);
            }
            ui.add_space(4.0);
            if ui.button("Fechar").clicked() {
                close = true;
            }
        });
        if close || r.should_close() {
            self.tutorials = false;
        }
        action
    }

    pub fn open_new_project(&mut self, settings: &Settings) {
        self.open_new_project_with(settings, Template::Padrao, "Meu jogo");
    }

    /// O formulário de projeto novo, já com um modelo escolhido.
    pub fn open_new_project_with(&mut self, settings: &Settings, template: Template, title: &str) {
        self.new_form = Some(NewProjectForm {
            title: title.into(),
            parent: settings::display_path(&settings.default_projects_dir()),
            template,
            submit: false,
            error: None,
            focus: true,
        });
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        settings: &mut Settings,
        logo: Option<&egui::TextureHandle>,
    ) -> Option<LauncherAction> {
        let mut action = None;
        egui::CentralPanel::default().show(ui, |ui| {
            let avail = ui.available_width();
            let left_w = 300.0_f32.min(avail * 0.4);
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(left_w, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.add_space(24.0);
                        ui.horizontal(|ui| {
                            if let Some(tex) = logo {
                                ui.add(egui::Image::new(tex).fit_to_exact_size(egui::vec2(56.0, 56.0)));
                            }
                            ui.vertical(|ui| {
                                ui.label(RichText::new("NN Studio").size(30.0).strong());
                                ui.weak("Crie jogos 2D com a nn-engine");
                            });
                        });
                        ui.add_space(28.0);
                        let big = |text: &str| {
                            egui::Button::new(RichText::new(text).size(16.0))
                                .min_size(egui::vec2(left_w - 24.0, 40.0))
                                .corner_radius(8.0)
                        };
                        if ui.add(big("✨ Novo projeto").fill(theme::ACCENT.gamma_multiply(0.75))).clicked() {
                            self.open_new_project(settings);
                        }
                        ui.add_space(4.0);
                        if ui.add(big("📂 Abrir projeto...")).on_hover_text("Ctrl+O").clicked() {
                            action = Some(LauncherAction::Command(Command::OpenProject));
                        }
                        ui.add_space(4.0);
                        if ui.add(big("🎮 Exemplos")).clicked() {
                            self.examples = true;
                        }
                        ui.add_space(4.0);
                        if ui
                            .add(big("🎓 Tutoriais"))
                            .on_hover_text("Aprenda fazendo: um passo de cada vez")
                            .clicked()
                        {
                            self.tutorials = true;
                        }
                        ui.add_space(28.0);
                        ui.separator();
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label("Tema:");
                            ui.selectable_value(&mut settings.theme, ThemeChoice::Escuro, "🌙 Escuro");
                            ui.selectable_value(&mut settings.theme, ThemeChoice::Claro, "☀ Claro");
                        });
                        ui.horizontal(|ui| {
                            ui.label("Modo:");
                            ui.selectable_value(&mut settings.mode, Mode::Simples, "Simples")
                                .on_hover_text("Mostra só o essencial. Bom para começar.");
                            ui.selectable_value(&mut settings.mode, Mode::Completo, "Completo")
                                .on_hover_text("Mostra todas as opções.");
                        });
                        ui.checkbox(&mut settings.check_updates, "Avisar versões novas")
                            .on_hover_text("Consulta o GitHub no máximo uma vez por dia. Só avisa: nunca baixa nada sozinho.");
                        ui.add_space(6.0);
                        ui.weak(format!("versão {}", env!("CARGO_PKG_VERSION")));
                    },
                );
                ui.separator();
                ui.vertical(|ui| {
                    ui.add_space(24.0);
                    update_banner(ui, settings);
                    if let Some(e) = &self.error {
                        let mut close = false;
                        egui::Frame::new()
                            .fill(theme::ERROR.gamma_multiply(0.2))
                            .inner_margin(8.0)
                            .corner_radius(6.0)
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.colored_label(theme::ERROR, format!("⛔ {e}"));
                                    if ui.small_button("✖").clicked() {
                                        close = true;
                                    }
                                });
                            });
                        if close {
                            self.error = None;
                        }
                        ui.add_space(8.0);
                    }
                    ui.label(RichText::new("Projetos recentes").size(19.0).strong());
                    ui.weak("Dica: arraste a pasta de um projeto para esta janela para abri-lo.");
                    ui.add_space(8.0);
                    if settings.recent.is_empty() {
                        ui.add_space(12.0);
                        // Primeira vez: três jeitos de começar.
                        egui::Frame::group(ui.style())
                            .fill(theme::ACCENT.gamma_multiply(0.15))
                            .inner_margin(14.0)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width().min(760.0));
                                ui.label(RichText::new("Bem-vindo ao NN Studio!").size(20.0).strong());
                                ui.label("Por onde você quer começar?");
                                ui.add_space(8.0);
                                let w = ((ui.available_width() - 16.0) / 3.0).clamp(150.0, 240.0);
                                ui.horizontal_top(|ui| {
                                    let card = |ui: &mut egui::Ui, title: &str, text: &str, accent: bool| {
                                        let b = egui::Button::new(
                                            RichText::new(format!("{title}\n\n{text}")).size(14.0),
                                        )
                                        .wrap()
                                        .min_size(egui::vec2(w, 116.0))
                                        .corner_radius(8.0);
                                        let b = if accent {
                                            b.fill(theme::ACCENT.gamma_multiply(0.6))
                                        } else {
                                            b
                                        };
                                        ui.add_sized(egui::vec2(w, 116.0), b)
                                    };
                                    let r = card(
                                        ui,
                                        "🎓 Fazer o tutorial",
                                        "Seu primeiro jogo: uma fase de plataforma em uns 15 minutos, com o editor mostrando cada passo.",
                                        true,
                                    );
                                    crate::tutorial::anchor_in(ui, "boas-vindas:tutorial", r.rect);
                                    if r.clicked() {
                                        action = Some(LauncherAction::StartTutorial("primeiro-jogo"));
                                    }
                                    let r = card(
                                        ui,
                                        "🎮 Abrir um jogo pronto",
                                        "O modelo Plataforma, completo e sem código, para ver como é feito e mudar o que quiser.",
                                        false,
                                    );
                                    if r.clicked() {
                                        self.open_new_project_with(settings, Template::Visual, "Meu jogo");
                                    }
                                    let r = card(
                                        ui,
                                        "📄 Começar do zero",
                                        "Uma cena vazia e um pacote de imagens e sons para montar do seu jeito.",
                                        false,
                                    );
                                    if r.clicked() {
                                        self.open_new_project_with(settings, Template::Vazio, "Meu jogo");
                                    }
                                });
                            });
                        ui.add_space(12.0);
                        ui.label("Nenhum projeto ainda.");
                        ui.weak("Crie o primeiro com ✨ Novo projeto, ou abra um dos 🎮 Exemplos para ver como um jogo é feito.");
                    }
                    let now = settings::now_secs();
                    let mut remove = None;
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                        for r in &settings.recent {
                            let exists = r.path.is_dir();
                            let frame = egui::Frame::new()
                                .fill(ui.visuals().faint_bg_color)
                                .inner_margin(10.0)
                                .corner_radius(8.0);
                            let resp = frame
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            let title = RichText::new(&r.title).size(16.0).strong();
                                            ui.label(if exists { title } else { title.weak() });
                                            ui.weak(settings::display_path(&r.path));
                                        });
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.small_button("✖").on_hover_text("Tirar da lista (não apaga nada)").clicked() {
                                                remove = Some(r.path.clone());
                                            }
                                            if exists {
                                                ui.weak(settings::time_ago(r.opened_at, now));
                                            } else {
                                                ui.colored_label(theme::WARNING, "pasta não encontrada");
                                            }
                                        });
                                    });
                                })
                                .response;
                            let resp = ui.interact(resp.rect, ui.id().with(("recente", &r.path)), egui::Sense::click());
                            if resp.hovered() {
                                ui.painter().rect_stroke(
                                    resp.rect,
                                    8.0,
                                    egui::Stroke::new(1.5, theme::ACCENT),
                                    egui::StrokeKind::Inside,
                                );
                            }
                            if resp.clicked() && exists {
                                action = Some(LauncherAction::Open(r.path.clone()));
                            }
                            ui.add_space(6.0);
                        }
                    });
                    if let Some(p) = remove {
                        settings.remove_recent(&p);
                    }

                });
            });
        });

        if let Some(a) = self.new_project_dialog(ui.ctx(), settings) {
            action = Some(a);
        }
        if let Some(a) = self.tutorials_dialog(ui.ctx(), settings) {
            action = Some(a);
        }
        if let Some(a) = self.examples_dialog(ui.ctx(), settings) {
            action = Some(a);
        }
        action
    }

    fn new_project_dialog(
        &mut self,
        ctx: &egui::Context,
        settings: &mut Settings,
    ) -> Option<LauncherAction> {
        let form = self.new_form.as_mut()?;
        let mut close = false;
        let mut action = None;
        let response = egui::Modal::new(egui::Id::new("novo-projeto")).show(ctx, |ui| {
            ui.set_width(560.0);
            ui.heading("✨ Novo projeto");
            ui.add_space(8.0);
            ui.label("Nome do jogo:");
            let r =
                ui.add(egui::TextEdit::singleline(&mut form.title).desired_width(f32::INFINITY));
            if form.focus {
                r.request_focus();
                form.focus = false;
            }
            ui.add_space(6.0);
            ui.label("Onde criar:");
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut form.parent).desired_width(340.0));
                if ui.button("Escolher...").clicked()
                    && let Some(dir) = rfd::FileDialog::new()
                        .set_title("Onde criar o projeto")
                        .set_directory(&form.parent)
                        .pick_folder()
                {
                    form.parent = settings::display_path(&dir);
                }
            });
            let folder = nn_app::slug(&form.title);
            let target = Path::new(form.parent.trim()).join(&folder);
            ui.weak(format!(
                "A pasta {} será criada.",
                settings::display_path(&target)
            ));
            ui.add_space(8.0);
            ui.label("Modelo:");
            if self.thumbs.is_empty() {
                for t in Template::ALL {
                    if let Ok(img) = image::load_from_memory(thumb_png(t)) {
                        let tex = ui.ctx().load_texture(
                            format!("modelo-{}", t.name()),
                            crate::viewers::to_color_image(&img.to_rgba8()),
                            egui::TextureOptions::LINEAR,
                        );
                        self.thumbs.push((t, tex));
                    }
                }
            }
            // Cartões: miniatura, nome e (embaixo) o que o escolhido traz.
            egui::Grid::new("modelos")
                .num_columns(3)
                .spacing([8.0, 8.0])
                .show(ui, |ui| {
                    for (i, t) in Template::ALL.into_iter().enumerate() {
                        let (name, desc) = template_info(t);
                        let selected = form.template == t;
                        let thumb = self.thumbs.iter().find(|(x, _)| *x == t).map(|(_, h)| h);
                        let r = template_card(ui, name, thumb, selected).on_hover_text(desc);
                        if r.clicked() {
                            form.template = t;
                        }
                        if r.double_clicked() {
                            form.template = t;
                            form.submit = true;
                        }
                        if i % 3 == 2 {
                            ui.end_row();
                        }
                    }
                });
            let (name, desc) = template_info(form.template);
            ui.add_space(4.0);
            ui.label(RichText::new(name).strong());
            ui.weak(desc);
            if let Some(e) = &form.error {
                ui.add_space(4.0);
                ui.colored_label(theme::ERROR, e.as_str());
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter))
                    || std::mem::take(&mut form.submit);
                if ui
                    .add(
                        egui::Button::new("Criar projeto").fill(theme::ACCENT.gamma_multiply(0.75)),
                    )
                    .clicked()
                    || enter
                {
                    if form.title.trim().is_empty() {
                        form.error = Some("Dê um nome ao jogo.".into());
                    } else if form.parent.trim().is_empty() {
                        form.error = Some("Escolha onde criar o projeto.".into());
                    } else {
                        let opts = nn_run::NewOptions {
                            dir: target.clone(),
                            title: Some(form.title.trim().to_string()),
                            template: form.template,
                        };
                        match std::fs::create_dir_all(form.parent.trim())
                            .map_err(|e| e.to_string())
                            .and_then(|_| nn_run::create_project(&opts))
                        {
                            Ok(report) => {
                                settings.projects_dir = Some(PathBuf::from(form.parent.trim()));
                                action = Some(LauncherAction::Open(report.dir));
                                close = true;
                            }
                            Err(e) => form.error = Some(e),
                        }
                    }
                }
                if ui.button("Cancelar").clicked() {
                    close = true;
                }
            });
        });
        if response.should_close() {
            close = true;
        }
        if close {
            self.new_form = None;
        }
        action
    }

    fn examples_dialog(
        &mut self,
        ctx: &egui::Context,
        settings: &Settings,
    ) -> Option<LauncherAction> {
        if !self.examples {
            return None;
        }
        let mut action = None;
        let mut close = false;
        let parent = settings.default_projects_dir();
        let response = egui::Modal::new(egui::Id::new("exemplos")).show(ctx, |ui| {
            ui.set_width(480.0);
            ui.heading("🎮 Exemplos");
            ui.weak(format!(
                "Abrir um exemplo cria uma cópia em {}, para você mexer à vontade.",
                settings::display_path(&parent)
            ));
            ui.add_space(8.0);
            for ex in EXAMPLES {
                egui::Frame::new()
                    .fill(ui.visuals().faint_bg_color)
                    .inner_margin(10.0)
                    .corner_radius(8.0)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(RichText::new(ex.title).size(16.0).strong());
                        ui.label(ex.description);
                        ui.add_space(4.0);
                        if ui
                            .button(format!("Abrir uma cópia de {}", ex.title))
                            .clicked()
                        {
                            match ex.copy_to(&parent) {
                                Ok(dir) => {
                                    action = Some(LauncherAction::Open(dir));
                                    close = true;
                                }
                                Err(e) => {
                                    self.error =
                                        Some(format!("Não foi possível copiar o exemplo: {e}"))
                                }
                            }
                        }
                    });
                ui.add_space(6.0);
            }
            if ui.button("Fechar").clicked() {
                close = true;
            }
        });
        if response.should_close() {
            close = true;
        }
        if close {
            self.examples = false;
        }
        action
    }
}
