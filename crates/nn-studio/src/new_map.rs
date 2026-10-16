//! Janela "Novo mapa de tiles": escolhe o tileset (ou uma imagem, ou uma
//! imagem nova em branco), o tamanho dos tiles e do mapa, e cria o `.tmj`.

use std::path::Path;

use eframe::egui;

use crate::files::{FileKind, FileTree};
use crate::map_doc::MapDoc;
use crate::tileset::{TilesetData, TilesetDoc, tsj_for};

/// De onde vêm os tiles do mapa novo.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    /// Um `.tsj` que já existe.
    Tileset(String),
    /// Uma imagem (o `.tsj` é criado ao lado dela).
    Image(String),
    /// Uma imagem nova, em branco, para desenhar os tiles.
    Blank,
}

impl Source {
    fn label(&self) -> String {
        match self {
            Source::Tileset(r) => format!("{r} (tileset)"),
            Source::Image(r) => r.clone(),
            Source::Blank => "Imagem nova, em branco (desenhar os tiles)".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewMapForm {
    pub name: String,
    pub source: Source,
    pub tile: [u32; 2],
    pub size: [u32; 2],
    pub scale: f64,
    pub error: Option<String>,
    focus: bool,
}

/// O mapa criado.
#[derive(Debug, Clone, PartialEq)]
pub struct Created {
    pub map: String,
    pub scale: f64,
    /// Tamanho do mapa no mundo (com a escala).
    pub world: (f64, f64),
    /// Arquivos criados (para o console).
    pub files: Vec<String>,
}

/// As opções de tiles do projeto: tilesets, depois imagens sem tileset.
pub fn sources(files: &FileTree) -> Vec<Source> {
    let tsj: Vec<String> = files
        .files_of(FileKind::Tileset)
        .map(|e| e.rel.clone())
        .filter(|r| r.to_lowercase().ends_with(".tsj"))
        .collect();
    let mut out: Vec<Source> = tsj.iter().cloned().map(Source::Tileset).collect();
    for e in files.files_of(FileKind::Image) {
        if !tsj.contains(&tsj_for(&e.rel)) {
            out.push(Source::Image(e.rel.clone()));
        }
    }
    out.push(Source::Blank);
    out
}

impl NewMapForm {
    pub fn new(files: &FileTree) -> Self {
        let mut name = "fase".to_string();
        let mut n = 1;
        while files.exists(&format!("mapas/{name}.tmj")) {
            n += 1;
            name = format!("fase{n}");
        }
        let source = sources(files).into_iter().next().unwrap_or(Source::Blank);
        let mut f = Self {
            name,
            source: Source::Blank,
            tile: [16, 16],
            size: [40, 15],
            scale: 3.0,
            error: None,
            focus: true,
        };
        f.pick(files, source);
        f
    }

    fn pick(&mut self, files: &FileTree, s: Source) {
        if let Source::Tileset(r) = &s
            && let Ok(d) = TilesetData::load(files.root(), r)
        {
            self.tile = [d.tile_w, d.tile_h];
        }
        self.source = s;
    }

    /// Desenha o formulário. `Some` quando criou o mapa.
    pub fn ui(&mut self, ui: &mut egui::Ui, files: &FileTree) -> (Option<Created>, bool) {
        let mut cancel = false;
        let mut created = None;
        ui.set_width(420.0);
        ui.heading("Novo mapa de tiles");
        ui.add_space(6.0);
        ui.label("Nome:");
        let r = ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(f32::INFINITY));
        if self.focus {
            r.request_focus();
            self.focus = false;
        }
        ui.add_space(4.0);
        ui.label("Tiles de:");
        let list = sources(files);
        let mut picked = None;
        egui::ComboBox::from_id_salt("novo-mapa-tiles")
            .width(400.0)
            .selected_text(self.source.label())
            .show_ui(ui, |ui| {
                for s in list {
                    if ui.selectable_label(self.source == s, s.label()).clicked() {
                        picked = Some(s);
                    }
                }
            });
        if let Some(s) = picked {
            self.pick(files, s);
        }
        let from_tileset = matches!(self.source, Source::Tileset(_));
        ui.horizontal(|ui| {
            ui.label("Tamanho do tile:");
            ui.add_enabled(
                !from_tileset,
                egui::DragValue::new(&mut self.tile[0])
                    .range(2..=256)
                    .suffix(" px"),
            );
            ui.label("×");
            ui.add_enabled(
                !from_tileset,
                egui::DragValue::new(&mut self.tile[1])
                    .range(2..=256)
                    .suffix(" px"),
            );
            if !from_tileset {
                for s in [8, 16, 32] {
                    if ui.small_button(s.to_string()).clicked() {
                        self.tile = [s, s];
                    }
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Mapa:");
            ui.add(
                egui::DragValue::new(&mut self.size[0])
                    .range(1..=crate::map_doc::MAX_SIZE)
                    .suffix(" colunas"),
            );
            ui.label("×");
            ui.add(
                egui::DragValue::new(&mut self.size[1])
                    .range(1..=crate::map_doc::MAX_SIZE)
                    .suffix(" linhas"),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Escala na cena:");
            ui.add(
                egui::DragValue::new(&mut self.scale)
                    .range(0.25..=16.0)
                    .speed(0.1)
                    .suffix("×"),
            );
        });
        ui.weak("O mapa cresce sozinho quando você pinta fora dele.");
        let rel = self.map_rel();
        ui.weak(format!("Será criado: {rel}"));
        if let Some(e) = &self.error {
            ui.colored_label(crate::theme::ERROR, e);
        }
        ui.add_space(8.0);
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
        ui.horizontal(|ui| {
            let create = ui
                .add(egui::Button::new("Criar").fill(crate::theme::ACCENT.gamma_multiply(0.7)))
                .clicked();
            if create || enter {
                match self.create(files) {
                    Ok(c) => created = Some(c),
                    Err(e) => self.error = Some(e),
                }
            }
            cancel = ui.button("Cancelar").clicked();
        });
        (created, cancel)
    }

    fn map_rel(&self) -> String {
        format!("mapas/{}.tmj", crate::files::safe_name(self.name.trim()))
    }

    pub fn create(&self, files: &FileTree) -> Result<Created, String> {
        let root = files.root();
        let name = crate::files::safe_name(self.name.trim());
        if name.is_empty() {
            return Err("Dê um nome ao mapa.".into());
        }
        let rel = self.map_rel();
        if root.join(&rel).exists() {
            return Err(format!("Já existe {rel}."));
        }
        let mut made = Vec::new();
        let [mut tw, mut th] = self.tile;
        let tsj = match &self.source {
            Source::Tileset(r) => {
                let d = TilesetData::load(root, r)?;
                (tw, th) = (d.tile_w, d.tile_h);
                r.clone()
            }
            Source::Image(img) => {
                let tsj = tsj_for(img);
                if !root.join(&tsj).is_file() {
                    let (w, h) = image::image_dimensions(root.join(img))
                        .map_err(|e| format!("{img}: {e}"))?;
                    if w < tw || h < th {
                        return Err(format!("A imagem tem {w} × {h}: menor que um tile."));
                    }
                    new_tileset(root, &tsj, img, (w, h), tw, th)?;
                    made.push(tsj.clone());
                }
                tsj
            }
            Source::Blank => {
                let dir = crate::file_panel::image_dir(files, "");
                let mut img = crate::files::join_rel(&dir, &format!("{name}_tiles.png"));
                let mut n = 1;
                while root.join(&img).exists() {
                    n += 1;
                    img = crate::files::join_rel(&dir, &format!("{name}_tiles{n}.png"));
                }
                let (w, h) = (tw * 8, th * 4);
                let full = root.join(&img);
                if let Some(d) = full.parent() {
                    std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
                }
                image::RgbaImage::new(w, h)
                    .save_with_format(&full, image::ImageFormat::Png)
                    .map_err(|e| e.to_string())?;
                made.push(img.clone());
                let tsj = tsj_for(&img);
                new_tileset(root, &tsj, &img, (w, h), tw, th)?;
                made.push(tsj.clone());
                tsj
            }
        };
        let mut map = MapDoc::new(&rel, self.size[0], self.size[1], tw, th, &tsj);
        map.save(root)?;
        made.push(rel.clone());
        Ok(Created {
            world: (
                (self.size[0] * tw) as f64 * self.scale,
                (self.size[1] * th) as f64 * self.scale,
            ),
            map: rel,
            scale: self.scale,
            files: made,
        })
    }
}

fn new_tileset(
    root: &Path,
    tsj: &str,
    image: &str,
    size: (u32, u32),
    tw: u32,
    th: u32,
) -> Result<(), String> {
    let name = image.rsplit('/').next().unwrap_or(image);
    let name = name.rsplit_once('.').map_or(name, |(a, _)| a);
    TilesetDoc::from_data(tsj, TilesetData::new(name, image, size, tw, th)).save(root)
}
