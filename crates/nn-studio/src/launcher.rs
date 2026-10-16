//! Gerenciador de projetos: a primeira tela do NN Studio, com os projetos
//! recentes, "Novo projeto", "Abrir projeto" e os exemplos.

use std::path::{Path, PathBuf};

use eframe::egui::{self, RichText};
use nn_run::Template;

use crate::commands::Command;
use crate::examples::EXAMPLES;
use crate::settings::{self, Mode, Settings, ThemeChoice};
use crate::theme;

#[derive(Debug, Clone, PartialEq)]
pub enum LauncherAction {
    Open(PathBuf),
    Command(Command),
}

#[derive(Debug)]
struct NewProjectForm {
    title: String,
    parent: String,
    template: Template,
    error: Option<String>,
    focus: bool,
}

#[derive(Debug, Default)]
pub struct Launcher {
    new_form: Option<NewProjectForm>,
    examples: bool,
    /// Mensagem de erro mostrada no topo (projeto inválido, por exemplo).
    pub error: Option<String>,
}

/// Descrição curta de cada modelo.
fn template_info(t: Template) -> (&'static str, &'static str) {
    match t {
        Template::Padrao => (
            "Padrão",
            "Um jogo pequeno e completo: título com menu, partida, pausa, fim e recorde salvo. O melhor jeito de começar.",
        ),
        Template::Minimo => (
            "Mínimo",
            "Só uma cena com um quadrado que anda. Para quem quer começar do zero.",
        ),
    }
}

impl Launcher {
    pub fn open_new_project(&mut self, settings: &Settings) {
        self.new_form = Some(NewProjectForm {
            title: "Meu jogo".into(),
            parent: settings::display_path(&settings.default_projects_dir()),
            template: Template::Padrao,
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
                        ui.add_space(6.0);
                        ui.weak(format!("versão {}", env!("CARGO_PKG_VERSION")));
                    },
                );
                ui.separator();
                ui.vertical(|ui| {
                    ui.add_space(24.0);
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
                        ui.add_space(20.0);
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
            ui.set_width(460.0);
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
            for t in Template::ALL {
                let (name, desc) = template_info(t);
                ui.radio_value(&mut form.template, t, RichText::new(name).strong());
                ui.indent(("modelo", name), |ui| {
                    ui.weak(desc);
                });
            }
            if let Some(e) = &form.error {
                ui.add_space(4.0);
                ui.colored_label(theme::ERROR, e.as_str());
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
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
