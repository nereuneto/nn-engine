//! Avisos rápidos embaixo, no meio da janela: "Objeto apagado · Desfazer",
//! "Salvo", "⭐ Guardado nos favoritos"... Somem sozinhos (parando enquanto o
//! mouse está em cima). Os erros também vão para o Console.

use std::time::Duration;

use eframe::egui::{self, Color32, RichText};

/// Que tipo de aviso.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Info,
    Success,
    Error,
    /// Um marco (o primeiro ▶ do projeto, a primeira exportação).
    Celebrate,
}

/// O que o Desfazer de um aviso desfaz: o último passo da cena `rel`, se
/// ela ainda está no passo `version` (se mudou depois, o botão some).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Undo {
    pub rel: String,
    pub version: u64,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub text: String,
    pub kind: Kind,
    pub undo: Option<Undo>,
    /// Quanto tempo ele já ficou à mostra.
    shown: f32,
    id: u64,
}

impl Toast {
    fn life(&self) -> f32 {
        match (self.kind, &self.undo) {
            (Kind::Error, _) => 8.0,
            (_, Some(_)) => 6.0,
            (Kind::Celebrate, _) => 6.0,
            _ => 3.5,
        }
    }
}

/// Os avisos à mostra.
#[derive(Debug, Default)]
pub struct Toasts {
    list: Vec<Toast>,
    next: u64,
}

/// Quantos ficam à mostra ao mesmo tempo (os mais velhos saem antes).
const MAX: usize = 4;

impl Toasts {
    pub fn push(&mut self, text: impl Into<String>, kind: Kind) {
        self.add(text.into(), kind, None);
    }

    /// Um aviso com o botão Desfazer.
    pub fn push_undo(&mut self, text: impl Into<String>, undo: Undo) {
        self.add(text.into(), Kind::Info, Some(undo));
    }

    fn add(&mut self, text: String, kind: Kind, undo: Option<Undo>) {
        // O mesmo aviso de novo (salvar duas vezes): só renova.
        if let Some(t) = self
            .list
            .iter_mut()
            .find(|t| t.text == text && t.kind == kind && t.undo.is_none() && undo.is_none())
        {
            t.shown = 0.0;
            return;
        }
        self.next += 1;
        self.list.push(Toast {
            text,
            kind,
            undo,
            shown: 0.0,
            id: self.next,
        });
        while self.list.len() > MAX {
            self.list.remove(0);
        }
    }

    /// Os textos à mostra (do mais velho ao mais novo).
    pub fn texts(&self) -> Vec<String> {
        self.list.iter().map(|t| t.text.clone()).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Desenha os avisos. `version` diz o passo atual de desfazer de uma
    /// cena (para mostrar o Desfazer só se nada mudou depois). Devolve o
    /// Desfazer clicado.
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        version: impl Fn(&str) -> Option<u64>,
        animate: bool,
    ) -> Option<Undo> {
        if self.list.is_empty() {
            return None;
        }
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        let mut clicked = None;
        let mut close = Vec::new();
        let mut hovered = false;
        let area = egui::Area::new(egui::Id::new("avisos-rapidos"))
            .order(egui::Order::Foreground)
            // Embaixo, no meio: os cantos são do tutorial guiado.
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -34.0))
            .interactable(true)
            .show(ctx, |ui| {
                ui.set_max_width(360.0);
                // Do mais velho (em cima) ao mais novo (embaixo).
                ui.vertical_centered(|ui| {
                    for t in self.list.iter() {
                        let fade = if animate {
                            let out = (t.life() - t.shown).clamp(0.0, 0.4) / 0.4;
                            let inn = (t.shown / 0.15).clamp(0.0, 1.0);
                            out.min(inn)
                        } else {
                            1.0
                        };
                        let (bar, icon) = match t.kind {
                            Kind::Info => (ui.visuals().selection.bg_fill, ""),
                            Kind::Success => (crate::theme::PLAY, "✔ "),
                            Kind::Error => (crate::theme::ERROR, "⚠ "),
                            Kind::Celebrate => (Color32::from_rgb(0xFF, 0xC8, 0x3D), ""),
                        };
                        let frame = egui::Frame::popup(ui.style())
                            .inner_margin(egui::Margin::symmetric(10, 7))
                            .multiply_with_opacity(fade);
                        let r = frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(3.0, 18.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter()
                                    .rect_filled(rect, 1.5, bar.gamma_multiply(fade));
                                ui.label(RichText::new(format!("{icon}{}", t.text)));
                                if let Some(u) = &t.undo
                                    && version(&u.rel) == Some(u.version)
                                    && ui
                                        .add(egui::Button::new(RichText::new("Desfazer").strong()))
                                        .clicked()
                                {
                                    clicked = Some(u.clone());
                                    close.push(t.id);
                                }
                                if ui
                                    .add(egui::Button::new("×").frame(false))
                                    .on_hover_text("Fechar")
                                    .clicked()
                                {
                                    close.push(t.id);
                                }
                            });
                        });
                        hovered |= r.response.contains_pointer();
                        ui.add_space(6.0);
                    }
                });
            });
        hovered |= area.response.contains_pointer();
        // O tempo para enquanto o mouse está em cima.
        if !hovered {
            for t in &mut self.list {
                t.shown += dt;
            }
        }
        self.list
            .retain(|t| !close.contains(&t.id) && t.shown < t.life());
        if !self.list.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(if animate { 33 } else { 250 }));
        }
        clicked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avisos_repetidos_renovam_e_o_limite() {
        let mut t = Toasts::default();
        t.push("Salvo: a.cena", Kind::Success);
        t.push("Salvo: a.cena", Kind::Success);
        assert_eq!(t.texts().len(), 1);
        for i in 0..6 {
            t.push(format!("aviso {i}"), Kind::Info);
        }
        assert_eq!(t.texts().len(), MAX);
        assert_eq!(t.texts().last().unwrap(), "aviso 5");
    }
}
