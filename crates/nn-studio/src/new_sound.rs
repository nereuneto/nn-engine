//! Janela "Novo som": um efeito sonoro sintetizado (moeda, pulo, tiro...),
//! com variações, ajustes e prévia, gravado como `.wav` (e os números num
//! `.sfx` ao lado, para ajustar depois). É o mesmo gerador do `criar_som` do
//! servidor MCP ([`nn_projeto::sfx`]).

use std::path::Path;

use eframe::egui::{self, Color32, Pos2, RichText, Sense, Stroke, Vec2};

use nn_projeto::sfx::{self, FIELDS, KINDS, Sfx, Wave};

use crate::files::FileTree;

/// O som gravado.
#[derive(Debug, Clone, PartialEq)]
pub struct CreatedSound {
    pub wav: String,
    pub sfx: String,
    /// Era um som que já existia (o visualizador dele precisa recarregar).
    pub replaced: bool,
}

pub struct NewSoundForm {
    pub name: String,
    pub dir: String,
    /// Ajustando um som que já existe (o `.wav`).
    pub editing: Option<String>,
    pub sfx: Sfx,
    pub kind: String,
    pub variation: u32,
    pub error: Option<String>,
    /// O nome segue o tipo (até a pessoa mudar).
    auto_name: bool,
    focus: bool,
    samples: Vec<f32>,
    peaks: Vec<(f32, f32)>,
    sound: Option<nn_audio::SoundId>,
    playing: Option<nn_audio::Sound>,
    /// Tocar quando der (depois de mudar alguma coisa).
    play_next: bool,
    next_variation: u32,
}

impl std::fmt::Debug for NewSoundForm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NewSoundForm")
            .field("rel", &self.rel())
            .field("kind", &self.kind)
            .field("variation", &self.variation)
            .finish_non_exhaustive()
    }
}

/// Uma pasta para os sons: a escolhida, ou `assets/sons`.
pub fn sound_dir(chosen: &str) -> String {
    if chosen.starts_with("assets") {
        chosen.to_string()
    } else {
        "assets/sons".into()
    }
}

impl NewSoundForm {
    pub fn new(files: &FileTree, dir: &str) -> Self {
        let mut f = Self {
            name: String::new(),
            dir: sound_dir(dir),
            editing: None,
            sfx: Sfx::preset("moeda", 0).unwrap_or_default(),
            kind: "moeda".into(),
            variation: 0,
            error: None,
            auto_name: true,
            focus: true,
            samples: Vec::new(),
            peaks: Vec::new(),
            sound: None,
            playing: None,
            play_next: false,
            next_variation: seed(),
        };
        f.name = f.free_name(files, "moeda");
        f.refresh();
        f
    }

    /// Ajustar o som `wav` (que tem o `.sfx` ao lado).
    pub fn open(root: &Path, wav: &str) -> Result<Self, String> {
        let side = sfx::sfx_for(wav);
        let text = std::fs::read_to_string(root.join(&side)).map_err(|e| format!("{side}: {e}"))?;
        let v: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("{side}: {e}"))?;
        let (fx, kind, variation) = Sfx::from_json(&v)?;
        let (dir, file) = wav.rsplit_once('/').unwrap_or(("", wav));
        let mut f = Self {
            name: file.trim_end_matches(".wav").to_string(),
            dir: dir.to_string(),
            editing: Some(wav.to_string()),
            sfx: fx,
            kind,
            variation,
            error: None,
            auto_name: false,
            focus: false,
            samples: Vec::new(),
            peaks: Vec::new(),
            sound: None,
            playing: None,
            play_next: false,
            next_variation: seed(),
        };
        f.refresh();
        Ok(f)
    }

    fn free_name(&self, files: &FileTree, base: &str) -> String {
        let mut name = base.to_string();
        let mut n = 1;
        while files.exists(&self.rel_for(&name)) {
            n += 1;
            name = format!("{base}{n}");
        }
        name
    }

    fn rel_for(&self, name: &str) -> String {
        let file = format!("{}.wav", name.trim().trim_end_matches(".wav"));
        if self.dir.is_empty() {
            file
        } else {
            format!("{}/{file}", self.dir)
        }
    }

    /// O `.wav` que vai ser gravado.
    pub fn rel(&self) -> String {
        self.editing
            .clone()
            .unwrap_or_else(|| self.rel_for(&self.name))
    }

    fn refresh(&mut self) {
        self.samples = self.sfx.render();
        self.peaks = sfx::peaks(&self.samples, 360);
    }

    fn changed(&mut self) {
        self.refresh();
        self.play_next = true;
    }

    fn pick(&mut self, files: &FileTree, kind: &str, variation: u32) {
        if let Some(fx) = Sfx::preset(kind, variation) {
            self.sfx = fx;
            if self.auto_name && self.editing.is_none() && self.kind != kind {
                self.name = self.free_name(files, kind);
            }
            self.kind = kind.to_string();
            self.variation = variation;
            self.changed();
        }
    }

    fn play(&mut self, audio: &mut Option<nn_audio::Audio>) {
        let audio = audio.get_or_insert_with(nn_audio::Audio::new);
        if let Some(mut s) = self.playing.take() {
            s.stop();
        }
        let id = match self.sound {
            Some(id) => {
                if audio
                    .replace_bytes(id, sfx::wav_bytes(&self.samples))
                    .is_err()
                {
                    return;
                }
                id
            }
            None => {
                let id = audio.add_samples(sfx::RATE, &self.samples);
                self.sound = Some(id);
                id
            }
        };
        self.playing = Some(audio.play(id));
    }

    /// Grava o `.wav` e o `.sfx`.
    pub fn save(&self, root: &Path, files: &FileTree) -> Result<CreatedSound, String> {
        let name = self.name.trim();
        if self.editing.is_none() {
            if name.is_empty() {
                return Err("Dê um nome ao som.".into());
            }
            if name.contains(['/', '\\']) {
                return Err("O nome não pode ter / nem \\.".into());
            }
        }
        let wav = self.rel();
        if self.editing.is_none() && files.exists(&wav) {
            return Err(format!("{wav} já existe."));
        }
        let side = sfx::sfx_for(&wav);
        let full = root.join(&wav);
        if let Some(p) = full.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        std::fs::write(&full, sfx::wav_bytes(&self.samples)).map_err(|e| format!("{wav}: {e}"))?;
        std::fs::write(root.join(&side), self.sfx.text(&self.kind, self.variation))
            .map_err(|e| format!("{side}: {e}"))?;
        Ok(CreatedSound {
            wav,
            sfx: side,
            replaced: self.editing.is_some(),
        })
    }

    /// Desenha a janela: (som gravado, fechar).
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        files: &FileTree,
        audio: &mut Option<nn_audio::Audio>,
    ) -> (Option<CreatedSound>, bool) {
        let mut cancel = false;
        let mut created = None;
        ui.set_width(520.0);
        ui.heading(match &self.editing {
            Some(r) => format!("Ajustar o som {r}"),
            None => "Novo som (efeito sonoro)".into(),
        });
        ui.add_space(4.0);
        ui.weak("Escolha um tipo, aperte Variar até gostar e ajuste se quiser. O som toca a cada mudança.");
        ui.add_space(6.0);

        // Os tipos.
        let mut picked = None;
        ui.horizontal_wrapped(|ui| {
            for (id, label, tip) in KINDS {
                let on = self.kind == *id;
                if ui
                    .selectable_label(on, *label)
                    .on_hover_text(*tip)
                    .clicked()
                {
                    picked = Some((*id, 0));
                }
            }
        });
        ui.horizontal(|ui| {
            let can_vary = Sfx::preset(&self.kind, 0).is_some();
            if ui
                .add_enabled(can_vary, egui::Button::new("🎲 Variar"))
                .on_hover_text("Outra versão do mesmo tipo")
                .clicked()
            {
                self.next_variation = self
                    .next_variation
                    .wrapping_mul(1_103_515_245)
                    .wrapping_add(12_345)
                    % 1_000_000
                    + 1;
                picked = Some(("", self.next_variation));
            }
            if self.variation > 0 {
                ui.weak(format!("variação {}", self.variation));
            }
            ui.separator();
            let playing = self
                .playing
                .as_ref()
                .is_some_and(nn_audio::Sound::is_playing);
            if ui
                .button(RichText::new(if playing { "⏹ Parar" } else { "▶ Tocar" }).strong())
                .clicked()
            {
                if playing {
                    if let Some(mut s) = self.playing.take() {
                        s.stop();
                    }
                } else {
                    self.play(audio);
                }
            }
            ui.label(format!("{:.2} s", self.sfx.duration()));
            if audio.as_ref().is_some_and(|a| !a.is_enabled()) {
                ui.weak("(sem saída de som neste computador)");
            }
        });
        if let Some((kind, v)) = picked {
            let kind = if kind.is_empty() {
                self.kind.clone()
            } else {
                kind.to_string()
            };
            self.pick(files, &kind, v);
        }

        // A onda.
        ui.add_space(4.0);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 90.0), Sense::click());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 6.0, ui.visuals().extreme_bg_color);
        let n = self.peaks.len().max(1) as f32;
        let mid = rect.center().y;
        let half = rect.height() * 0.45;
        let color = ui.visuals().selection.bg_fill;
        for (i, (lo, hi)) in self.peaks.iter().enumerate() {
            let x = rect.left() + (i as f32 + 0.5) / n * rect.width();
            painter.line_segment(
                [
                    Pos2::new(x, mid - hi * half),
                    Pos2::new(x, mid - lo * half + 1.0),
                ],
                Stroke::new((rect.width() / n).max(1.0), color),
            );
        }
        // A nota ao longo do tempo (de 30 Hz a 4 kHz).
        let (lo, hi) = (30f32.ln(), 4000f32.ln());
        let dur = self.sfx.duration();
        let pts: Vec<Pos2> = (0..=80)
            .map(|i| {
                let t = i as f32 / 80.0;
                let f = self.sfx.freq_at(t * dur).clamp(30.0, 4000.0);
                let k = (f.ln() - lo) / (hi - lo);
                Pos2::new(
                    rect.left() + t * rect.width(),
                    rect.bottom() - 4.0 - k * (rect.height() - 8.0),
                )
            })
            .collect();
        painter.add(egui::Shape::line(
            pts,
            Stroke::new(1.5, Color32::from_rgb(0xff, 0xcd, 0x75)),
        ));
        if response
            .on_hover_text("A onda (verde: o volume) e a nota ao longo do tempo (amarelo: mais alto = mais agudo). Clique para tocar.")
            .clicked()
        {
            self.play(audio);
        }

        // Os ajustes.
        ui.add_space(4.0);
        let mut changed = false;
        egui::CollapsingHeader::new("Ajustes")
            .default_open(self.editing.is_some())
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        egui::Grid::new("som-ajustes")
                            .num_columns(2)
                            .spacing([10.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("Onda");
                                egui::ComboBox::from_id_salt("som-onda")
                                    .selected_text(self.sfx.wave.label())
                                    .show_ui(ui, |ui| {
                                        for w in Wave::ALL {
                                            if ui
                                                .selectable_label(self.sfx.wave == w, w.label())
                                                .clicked()
                                            {
                                                self.sfx.wave = w;
                                                changed = true;
                                            }
                                        }
                                    });
                                ui.end_row();
                                for f in FIELDS {
                                    if matches!(f.key, "pulso" | "pulso_fim")
                                        && self.sfx.wave != Wave::Square
                                    {
                                        continue;
                                    }
                                    let mut v = self.sfx.get(f.key).unwrap_or(0.0);
                                    ui.label(f.label).on_hover_text(f.tip);
                                    let log = matches!(
                                        f.key,
                                        "freq" | "freq_fim" | "passa_baixa" | "passa_alta"
                                    );
                                    let r = ui.add(
                                        egui::Slider::new(&mut v, f.min..=f.max)
                                            .logarithmic(log)
                                            .max_decimals(3),
                                    );
                                    if r.drag_stopped() || (r.changed() && !r.dragged()) {
                                        changed = true;
                                    }
                                    if r.changed() {
                                        let _ = self.sfx.set(f.key, v);
                                        self.refresh();
                                    }
                                    ui.end_row();
                                }
                            });
                    });
            });
        if changed {
            self.changed();
        }
        if self.play_next {
            self.play_next = false;
            self.play(audio);
        }

        // Nome e gravar.
        ui.add_space(6.0);
        if self.editing.is_none() {
            ui.horizontal(|ui| {
                ui.label("Nome:");
                let r = ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(180.0));
                if self.focus {
                    r.request_focus();
                    self.focus = false;
                }
                if r.changed() {
                    self.auto_name = false;
                }
                ui.weak(format!("({})", self.rel()));
            });
        }
        if let Some(e) = &self.error {
            ui.colored_label(crate::theme::ERROR, e);
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let label = if self.editing.is_some() {
                "Gravar"
            } else {
                "Criar"
            };
            if ui.button(RichText::new(label).strong()).clicked() {
                match self.save(files.root(), files) {
                    Ok(c) => created = Some(c),
                    Err(e) => self.error = Some(e),
                }
            }
            if ui.button("Cancelar").clicked() {
                cancel = true;
            }
        });
        if (created.is_some() || cancel)
            && let Some(mut s) = self.playing.take()
        {
            s.stop();
        }
        (created, cancel)
    }
}

/// Um número para começar as variações (diferente a cada janela).
fn seed() -> u32 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() % 1000 + 1)
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cria_e_ajusta_um_som() {
        let dir = std::env::temp_dir().join(format!("nn-novo-som-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let files = FileTree::new(&dir);
        let mut f = NewSoundForm::new(&files, "");
        assert_eq!(f.rel(), "assets/sons/moeda.wav");
        f.pick(&files, "pulo", 0);
        assert_eq!(f.name, "pulo", "o nome segue o tipo");
        f.pick(&files, "pulo", 9);
        let c = f.save(&dir, &files).unwrap();
        assert_eq!(c.sfx, "assets/sons/pulo.sfx");
        let files = FileTree::new(&dir);
        assert!(f.save(&dir, &files).unwrap_err().contains("já existe"));

        let mut g = NewSoundForm::open(&dir, "assets/sons/pulo.wav").unwrap();
        assert_eq!((g.kind.as_str(), g.variation), ("pulo", 9));
        assert_eq!(g.sfx.freq, f.sfx.freq);
        g.sfx.set("queda", 0.9).unwrap();
        g.refresh();
        assert!(g.save(&dir, &files).unwrap().replaced);
        let bytes = std::fs::read(dir.join("assets/sons/pulo.wav")).unwrap();
        assert_eq!(bytes.len(), 44 + g.samples.len() * 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
