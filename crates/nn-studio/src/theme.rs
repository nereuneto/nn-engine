//! Aparência do NN Studio: tema escuro ou claro, tamanho do texto e cores.

use eframe::egui::{self, Color32, FontId, TextStyle};

use crate::settings::ThemeChoice;

/// Cor de destaque do NN Studio.
pub const ACCENT: Color32 = Color32::from_rgb(0x3F, 0xA7, 0xD6);
/// Verde do botão ▶ Jogar.
pub const PLAY: Color32 = Color32::from_rgb(0x3C, 0xA5, 0x5C);
pub const ERROR: Color32 = Color32::from_rgb(0xF2, 0x6D, 0x6D);
pub const WARNING: Color32 = Color32::from_rgb(0xE8, 0xB3, 0x39);

/// Tamanho base do texto do código.
pub const CODE_SIZE: f32 = 14.0;

pub fn apply(ctx: &egui::Context, theme: ThemeChoice, zoom: f32) {
    let dark = theme == ThemeChoice::Escuro;
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.selection.bg_fill = if dark {
        Color32::from_rgb(0x2A, 0x6F, 0x92)
    } else {
        Color32::from_rgb(0xA8, 0xD8, 0xEE)
    };
    visuals.selection.stroke.color = if dark { Color32::WHITE } else { Color32::BLACK };
    visuals.hyperlink_color = if dark {
        Color32::from_rgb(0x6C, 0xC4, 0xEE)
    } else {
        Color32::from_rgb(0x1A, 0x6E, 0x9A)
    };
    if dark {
        visuals.panel_fill = Color32::from_rgb(0x1E, 0x21, 0x27);
        visuals.window_fill = Color32::from_rgb(0x25, 0x29, 0x30);
        visuals.extreme_bg_color = Color32::from_rgb(0x16, 0x18, 0x1D);
        visuals.faint_bg_color = Color32::from_rgb(0x23, 0x26, 0x2D);
    }
    let theme_kind = if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    ctx.set_theme(theme_kind);
    ctx.set_visuals_of(theme_kind, visuals);
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Heading, FontId::proportional(21.0)),
            (TextStyle::Body, FontId::proportional(14.0)),
            (TextStyle::Button, FontId::proportional(14.0)),
            (TextStyle::Small, FontId::proportional(11.0)),
            (TextStyle::Monospace, FontId::monospace(13.0)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(8.0, 5.0);
        style.spacing.button_padding = egui::vec2(8.0, 3.0);
        style.spacing.interact_size.y = 22.0;
    });
    ctx.options_mut(|o| o.zoom_with_keyboard = false);
    if (ctx.zoom_factor() - zoom).abs() > 0.001 {
        ctx.set_zoom_factor(zoom);
    }
}

/// Próximo passo de zoom da interface.
pub fn zoom_step(zoom: f32, up: bool) -> f32 {
    const STEPS: [f32; 7] = [0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75];
    if up {
        STEPS
            .iter()
            .copied()
            .find(|s| *s > zoom + 0.01)
            .unwrap_or(1.75)
    } else {
        STEPS
            .iter()
            .rev()
            .copied()
            .find(|s| *s < zoom - 0.01)
            .unwrap_or(0.8)
    }
}

/// Botão grande e colorido (▶ Jogar).
pub fn big_button(text: &str, fill: Color32) -> egui::Button<'_> {
    egui::Button::new(
        egui::RichText::new(text)
            .color(Color32::WHITE)
            .size(15.0)
            .strong(),
    )
    .fill(fill)
    .min_size(egui::vec2(96.0, 28.0))
    .corner_radius(6.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passos_de_zoom() {
        assert_eq!(zoom_step(1.0, true), 1.1);
        assert_eq!(zoom_step(1.0, false), 0.9);
        assert_eq!(zoom_step(1.75, true), 1.75);
        assert_eq!(zoom_step(0.8, false), 0.8);
    }
}
