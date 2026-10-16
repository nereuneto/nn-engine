//! Pintar mapas de tiles: as ferramentas (pincel, borracha, balde,
//! retângulo, conta-gotas, terreno), o carimbo escolhido no painel Tiles e o
//! traço em si, usado tanto na cena quanto na aba do mapa.

use std::collections::HashMap;

use crate::map_doc::{MapDoc, Stamp, TsKind};
use crate::tileset::{TilesetData, TilesetDoc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintTool {
    Brush,
    Eraser,
    Fill,
    Rect,
    Picker,
    Terrain,
}

impl PaintTool {
    pub const ALL: [PaintTool; 6] = [
        PaintTool::Brush,
        PaintTool::Eraser,
        PaintTool::Fill,
        PaintTool::Rect,
        PaintTool::Picker,
        PaintTool::Terrain,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PaintTool::Brush => "Pincel",
            PaintTool::Eraser => "Borracha",
            PaintTool::Fill => "Balde",
            PaintTool::Rect => "Retângulo",
            PaintTool::Picker => "Conta-gotas",
            PaintTool::Terrain => "Terreno",
        }
    }

    pub fn key(self) -> egui::Key {
        use egui::Key;
        match self {
            PaintTool::Brush => Key::B,
            PaintTool::Eraser => Key::E,
            PaintTool::Fill => Key::G,
            PaintTool::Rect => Key::U,
            PaintTool::Picker => Key::I,
            PaintTool::Terrain => Key::T,
        }
    }

    pub fn help(self) -> &'static str {
        match self {
            PaintTool::Brush => {
                "Pinta com os tiles escolhidos no tileset (arraste no tileset para pegar vários). R gira, F espelha."
            }
            PaintTool::Eraser => "Apaga os tiles da camada.",
            PaintTool::Fill => "Pinta a área ligada que tem o mesmo tile.",
            PaintTool::Rect => "Arraste para encher um retângulo com os tiles escolhidos.",
            PaintTool::Picker => {
                "Pega o tile do mapa (com o giro). Com qualquer ferramenta: Alt+clique."
            }
            PaintTool::Terrain => "Pinta um terreno: as bordas e os cantos se encaixam sozinhos.",
        }
    }
}

use eframe::egui;

/// O que se pinta, escolhido no painel Tiles (vale para todos os mapas).
#[derive(Debug, Clone)]
pub struct PaintState {
    pub tool: PaintTool,
    /// Tiles do pincel (ids globais do mapa atual, com os giros).
    pub stamp: Stamp,
    /// Tileset mostrado no painel (índice nos tilesets do mapa).
    pub tileset: usize,
    /// Terreno do pincel de terreno: (conjunto, cor; 0 = apagar terreno).
    pub terrain: Option<(usize, u8)>,
    /// Mostra quais tiles bloqueiam.
    pub show_collision: bool,
    /// Grade de tiles por cima do mapa.
    pub show_grid: bool,
}

impl Default for PaintState {
    fn default() -> Self {
        Self {
            tool: PaintTool::Brush,
            stamp: Stamp::single(1),
            tileset: 0,
            terrain: None,
            show_collision: true,
            show_grid: true,
        }
    }
}

/// Os dados de um tileset do mapa (do `.tsj` aberto ou de dentro do mapa).
pub fn tileset_data<'a>(
    map: &'a MapDoc,
    i: usize,
    docs: &'a HashMap<String, TilesetDoc>,
) -> Option<&'a TilesetData> {
    match &map.tilesets.get(i)?.kind {
        TsKind::External(rel) => docs.get(rel).map(|d| &d.data),
        TsKind::Embedded(d) => Some(d),
    }
}

/// Uma mudança de tile (camada, coluna, linha, tile), para o jogo aberto.
pub type Change = (String, i64, i64, u32);

/// Resultado de um passo do traço.
#[derive(Debug, Default)]
pub struct StrokeResult {
    pub changes: Vec<Change>,
    /// O mapa cresceu (o jogo aberto só vê ao salvar).
    pub grew: bool,
}

/// Um traço em andamento (do apertar ao soltar o botão).
#[derive(Debug, Clone)]
pub struct Stroke {
    pub start: (i64, i64),
    pub last: (i64, i64),
    /// Retângulo: a camada antes, para refazer enquanto arrasta.
    before: Option<Vec<u32>>,
    /// Ctrl: arrastar copia um pedaço do mapa para o carimbo.
    pub copy: bool,
}

fn layer_name(map: &MapDoc) -> String {
    map.tile_layer(map.layer)
        .map(|l| l.name.clone())
        .unwrap_or_default()
}

fn locked(map: &MapDoc) -> bool {
    map.tile_layer(map.layer).is_none_or(|l| l.locked)
}

/// Pinta o carimbo com o canto de cima à esquerda em (col, row), crescendo
/// o mapa se precisar. Devolve quanto o mapa cresceu à esquerda e em cima.
fn stamp_at(
    map: &mut MapDoc,
    stamp: &Stamp,
    col: i64,
    row: i64,
    out: &mut StrokeResult,
) -> (i64, i64) {
    let layer = map.layer;
    let Some((c0, r0)) = map.grow_to(col, row) else {
        return (0, 0);
    };
    let first = map.last_grow;
    let far = (c0 + stamp.w as i64 - 1, r0 + stamp.h as i64 - 1);
    if map.grow_to(far.0, far.1).is_none() {
        return first;
    }
    if first != (0, 0) || (map.width as i64) <= far.0 || (map.height as i64) <= far.1 {
        out.grew = true;
    }
    if map.last_grow != (0, 0) {
        out.grew = true;
    }
    let name = layer_name(map);
    for r in 0..stamp.h {
        for c in 0..stamp.w {
            let gid = stamp.get(c, r);
            if gid == 0 {
                continue;
            }
            let (x, y) = (c0 + c as i64, r0 + r as i64);
            if map.set(layer, x, y, gid) {
                out.changes.push((name.clone(), x, y, gid));
            }
        }
    }
    first
}

impl PaintState {
    /// Apertou o botão na célula (col, row) (pode estar fora do mapa).
    pub fn begin(
        &mut self,
        map: &mut MapDoc,
        docs: &HashMap<String, TilesetDoc>,
        cell: (i64, i64),
        copy: bool,
        pick: bool,
    ) -> (Stroke, StrokeResult) {
        let mut stroke = Stroke {
            start: cell,
            last: cell,
            before: None,
            copy,
        };
        let mut out = StrokeResult::default();
        if copy {
            return (stroke, out);
        }
        if pick || self.tool == PaintTool::Picker {
            self.pick(map, cell);
            return (stroke, out);
        }
        if locked(map) {
            return (stroke, out);
        }
        map.checkpoint();
        if self.tool == PaintTool::Rect {
            stroke.before = map.tile_layer(map.layer).map(|l| l.data.clone());
        }
        let cell = self.apply(map, docs, &mut stroke, cell, &mut out);
        stroke.last = cell;
        (stroke, out)
    }

    /// O mouse andou até a célula, segurando o botão.
    pub fn drag(
        &mut self,
        map: &mut MapDoc,
        docs: &HashMap<String, TilesetDoc>,
        stroke: &mut Stroke,
        cell: (i64, i64),
    ) -> StrokeResult {
        let mut out = StrokeResult::default();
        if cell == stroke.last || stroke.copy || self.tool == PaintTool::Picker || locked(map) {
            stroke.last = cell;
            return out;
        }
        match self.tool {
            PaintTool::Rect => {
                stroke.last = self.apply(map, docs, stroke, cell, &mut out);
            }
            PaintTool::Fill => stroke.last = cell,
            _ => {
                // Todas as células no caminho (o mouse pode pular várias).
                let line = crate::pixel_ops::line(
                    (stroke.last.0 as i32, stroke.last.1 as i32),
                    (cell.0 as i32, cell.1 as i32),
                );
                let mut fix = (0, 0);
                for (c, r) in line.into_iter().skip(1) {
                    let at = (c as i64 + fix.0, r as i64 + fix.1);
                    let moved = self.apply(map, docs, stroke, at, &mut out);
                    // O mapa cresceu à esquerda ou em cima: o resto do
                    // caminho anda junto.
                    fix.0 += moved.0 - at.0;
                    fix.1 += moved.1 - at.1;
                }
                stroke.last = (cell.0 + fix.0, cell.1 + fix.1);
            }
        }
        out
    }

    /// Soltou o botão.
    pub fn end(&mut self, map: &mut MapDoc, stroke: Stroke, cell: (i64, i64)) {
        if stroke.copy {
            let (c0, c1) = (stroke.start.0.min(cell.0), stroke.start.0.max(cell.0));
            let (r0, r1) = (stroke.start.1.min(cell.1), stroke.start.1.max(cell.1));
            let (w, h) = ((c1 - c0 + 1) as u32, (r1 - r0 + 1) as u32);
            if w * h <= 64 * 64 {
                let mut tiles = Vec::new();
                for r in r0..=r1 {
                    for c in c0..=c1 {
                        tiles.push(map.get(map.layer, c, r));
                    }
                }
                if tiles.iter().any(|t| *t != 0) {
                    self.stamp = Stamp { w, h, tiles };
                    self.tool = PaintTool::Brush;
                }
            }
            return;
        }
        if self.tool != PaintTool::Picker {
            map.drop_checkpoint_if_unchanged();
        }
    }

    fn pick(&mut self, map: &MapDoc, cell: (i64, i64)) {
        let mut gid = map.get(map.layer, cell.0, cell.1);
        if gid == 0 {
            // Na camada atual não tem: o de cima que tiver.
            for l in map.tile_layers().into_iter().rev() {
                gid = map.get(l, cell.0, cell.1);
                if gid != 0 {
                    break;
                }
            }
        }
        if gid != 0 {
            self.stamp = Stamp::single(gid);
            if let Some((ts, _)) = map.tileset_of(gid) {
                self.tileset = ts;
            }
            self.tool = PaintTool::Brush;
        }
    }

    /// Aplica a ferramenta na célula. Devolve onde a célula foi parar (o
    /// mapa pode ter crescido à esquerda ou em cima).
    fn apply(
        &mut self,
        map: &mut MapDoc,
        docs: &HashMap<String, TilesetDoc>,
        stroke: &mut Stroke,
        cell: (i64, i64),
        out: &mut StrokeResult,
    ) -> (i64, i64) {
        let layer = map.layer;
        let name = layer_name(map);
        let mut cell = cell;
        match self.tool {
            PaintTool::Brush => {
                let stamp = self.stamp.clone();
                // Carimbos grandes andam de carimbo em carimbo (como no Tiled).
                let (w, h) = (stamp.w.max(1) as i64, stamp.h.max(1) as i64);
                let (sc, sr) = (
                    stroke.start.0 + (cell.0 - stroke.start.0).div_euclid(w) * w,
                    stroke.start.1 + (cell.1 - stroke.start.1).div_euclid(h) * h,
                );
                let grew = stamp_at(map, &stamp, sc, sr, out);
                stroke.start.0 += grew.0;
                stroke.start.1 += grew.1;
                cell = (cell.0 + grew.0, cell.1 + grew.1);
            }
            PaintTool::Eraser => {
                if map.set(layer, cell.0, cell.1, 0) {
                    out.changes.push((name, cell.0, cell.1, 0));
                }
            }
            PaintTool::Fill => {
                let gid = self.stamp.tiles.first().copied().unwrap_or(0);
                for (c, r) in map.flood(layer, cell.0, cell.1, gid) {
                    out.changes.push((name.clone(), c, r, gid));
                }
            }
            PaintTool::Rect => {
                // O retângulo fica dentro do mapa; refaz a partir da camada de antes.
                let clamp = |v: i64, n: u32| v.clamp(0, n as i64 - 1);
                let cell2 = (clamp(cell.0, map.width), clamp(cell.1, map.height));
                let start = (
                    clamp(stroke.start.0, map.width),
                    clamp(stroke.start.1, map.height),
                );
                if let (Some(before), Some(l)) = (&stroke.before, map.tile_layer_mut(layer)) {
                    l.data.clone_from(before);
                }
                let (c0, c1) = (start.0.min(cell2.0), start.0.max(cell2.0));
                let (r0, r1) = (start.1.min(cell2.1), start.1.max(cell2.1));
                let stamp = self.stamp.clone();
                for r in r0..=r1 {
                    for c in c0..=c1 {
                        let gid = stamp.get(
                            ((c - c0) % stamp.w.max(1) as i64) as u32,
                            ((r - r0) % stamp.h.max(1) as i64) as u32,
                        );
                        if gid != 0 {
                            map.set(layer, c, r, gid);
                        }
                    }
                }
                // O jogo aberto recebe a área toda (e o que voltou ao que era).
                let (a0, a1) = (
                    start.0.min(stroke.last.0).min(cell2.0),
                    start.0.max(stroke.last.0).max(cell2.0),
                );
                let (b0, b1) = (
                    start.1.min(stroke.last.1).min(cell2.1),
                    start.1.max(stroke.last.1).max(cell2.1),
                );
                for r in b0.max(0)..=b1.min(map.height as i64 - 1) {
                    for c in a0.max(0)..=a1.min(map.width as i64 - 1) {
                        out.changes.push((name.clone(), c, r, map.get(layer, c, r)));
                    }
                }
                cell = cell2;
            }
            PaintTool::Terrain => {
                let Some((set_i, color)) = self.terrain else {
                    return cell;
                };
                let ts = self.tileset;
                let Some(data) = tileset_data(map, ts, docs).cloned() else {
                    return cell;
                };
                let Some(set) = data.wangsets.get(set_i).cloned() else {
                    return cell;
                };
                if color != 0 {
                    match map.grow_to(cell.0, cell.1) {
                        Some(c) => {
                            if c != cell || map.last_grow != (0, 0) {
                                out.grew = true;
                            }
                            stroke.start.0 += map.last_grow.0;
                            stroke.start.1 += map.last_grow.1;
                            cell = c;
                        }
                        None => return cell,
                    }
                }
                for (x, y) in map.paint_terrain(layer, ts, &data, &set, color, cell.0, cell.1) {
                    out.changes.push((name.clone(), x, y, map.get(layer, x, y)));
                }
            }
            PaintTool::Picker => {}
        }
        if !out.changes.is_empty() || out.grew {
            map.touch();
        }
        cell
    }
}

/// Abre o mapa (e os tilesets `.tsj` dele) se ainda não estiver aberto.
pub fn ensure_open(
    maps: &mut HashMap<String, MapDoc>,
    tilesets: &mut HashMap<String, TilesetDoc>,
    root: &std::path::Path,
    rel: &str,
) -> Result<(), String> {
    if !maps.contains_key(rel) {
        let doc = MapDoc::open(root, rel)?;
        maps.insert(rel.to_string(), doc);
    }
    let mut err = None;
    for ts in maps[rel].external_tilesets() {
        if let std::collections::hash_map::Entry::Vacant(slot) = tilesets.entry(ts) {
            match TilesetDoc::open(root, slot.key()) {
                Ok(d) => {
                    slot.insert(d);
                }
                Err(e) => err = Some(e),
            }
        }
    }
    match err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// Versão do mapa junto com a dos tilesets dele (para saber quando
/// desenhar de novo).
pub fn combined_rev(map: &MapDoc, tilesets: &HashMap<String, TilesetDoc>) -> u64 {
    map.external_tilesets()
        .iter()
        .filter_map(|t| tilesets.get(t))
        .fold(map.rev().wrapping_mul(1_000_003), |acc, t| {
            acc.wrapping_mul(31).wrapping_add(t.rev)
        })
}

/// Os textos do mapa e dos tilesets, como iriam para o disco.
pub fn source_files(
    map: &MapDoc,
    tilesets: &HashMap<String, TilesetDoc>,
) -> HashMap<String, String> {
    let mut files = HashMap::new();
    files.insert(map.rel.clone(), map.text());
    for t in map.external_tilesets() {
        if let Some(d) = tilesets.get(&t) {
            files.insert(t.clone(), d.text());
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapa() -> MapDoc {
        MapDoc::new("a.tmj", 4, 4, 16, 16, "t.tsj")
    }

    #[test]
    fn pincel_arrastado_e_desfazer() {
        let mut m = mapa();
        let docs = HashMap::new();
        let mut p = PaintState {
            stamp: Stamp::single(3),
            ..Default::default()
        };
        let (mut s, out) = p.begin(&mut m, &docs, (0, 0), false, false);
        assert_eq!(out.changes.len(), 1);
        // Pulou da coluna 0 para a 3: as do meio também.
        let out = p.drag(&mut m, &docs, &mut s, (3, 0));
        assert_eq!(out.changes.len(), 3);
        p.end(&mut m, s, (3, 0));
        assert!((0..4).all(|c| m.get(0, c, 0) == 3));
        assert!(m.undo());
        assert!((0..4).all(|c| m.get(0, c, 0) == 0), "um traço = um passo");
    }

    #[test]
    fn pintar_fora_cresce_e_o_traco_acompanha() {
        let mut m = mapa();
        let docs = HashMap::new();
        let mut p = PaintState {
            stamp: Stamp::single(2),
            ..Default::default()
        };
        let (mut s, out) = p.begin(&mut m, &docs, (0, 1), false, false);
        assert!(!out.grew);
        // Arrasta para duas colunas à esquerda do mapa.
        let out = p.drag(&mut m, &docs, &mut s, (-2, 1));
        assert!(out.grew);
        assert_eq!(m.width, 6);
        assert_eq!(m.take_shift(), (2, 0));
        // A célula do começo andou duas colunas.
        assert_eq!(m.get(0, 2, 1), 2);
        assert_eq!(m.get(0, 0, 1), 2);
        assert_eq!(s.last, (0, 1));
        p.end(&mut m, s, (0, 1));
    }

    #[test]
    fn retangulo_balde_conta_gotas_e_copiar() {
        let mut m = mapa();
        let docs = HashMap::new();
        let mut p = PaintState {
            tool: PaintTool::Rect,
            stamp: Stamp::single(4),
            ..Default::default()
        };
        let (mut s, _) = p.begin(&mut m, &docs, (0, 0), false, false);
        p.drag(&mut m, &docs, &mut s, (3, 3));
        // Voltou: o retângulo encolhe.
        p.drag(&mut m, &docs, &mut s, (1, 1));
        p.end(&mut m, s, (1, 1));
        let n = m
            .tile_layer(0)
            .unwrap()
            .data
            .iter()
            .filter(|t| **t == 4)
            .count();
        assert_eq!(n, 4);
        p.tool = PaintTool::Fill;
        p.stamp = Stamp::single(5);
        let (s, _) = p.begin(&mut m, &docs, (3, 3), false, false);
        p.end(&mut m, s, (3, 3));
        assert_eq!(m.get(0, 3, 3), 5);
        assert_eq!(m.get(0, 0, 0), 4);
        // Conta-gotas (Alt+clique): pega o tile e volta ao pincel.
        let (s, _) = p.begin(&mut m, &docs, (0, 0), false, true);
        p.end(&mut m, s, (0, 0));
        assert_eq!(p.stamp, Stamp::single(4));
        assert_eq!(p.tool, PaintTool::Brush);
        // Ctrl+arrastar copia um pedaço do mapa.
        let (s, _) = p.begin(&mut m, &docs, (1, 1), true, false);
        p.end(&mut m, s, (2, 1));
        assert_eq!(
            p.stamp,
            Stamp {
                w: 2,
                h: 1,
                tiles: vec![4, 5]
            }
        );
    }

    #[test]
    fn camada_trancada_nao_pinta() {
        let mut m = mapa();
        m.edit_layer(0, |l| l.locked = true);
        let docs = HashMap::new();
        let mut p = PaintState::default();
        let (s, out) = p.begin(&mut m, &docs, (0, 0), false, false);
        p.end(&mut m, s, (0, 0));
        assert!(out.changes.is_empty());
        assert_eq!(m.get(0, 0, 0), 0);
    }
}
