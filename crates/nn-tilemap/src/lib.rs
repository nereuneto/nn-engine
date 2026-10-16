//! Tilemaps da nn-engine.
//!
//! Um [`TileMap`] tem camadas de tiles, tilesets (imagens divididas em
//! tiles) e objetos (pontos e retângulos com nome, tipo e propriedades, para
//! marcar onde nasce o jogador, inimigos, saídas...). Pode ser montado em
//! código ou importado do editor [Tiled](https://www.mapeditor.org) no
//! formato JSON (`.tmj`, com tilesets embutidos ou `.tsj`).
//!
//! Coordenadas: as linhas são contadas de cima para baixo (como no editor),
//! mas o mapa é posicionado no mundo com Y para cima, com o canto inferior
//! esquerdo em [`TileMap::origin`].
//!
//! Colisão: tiles com a propriedade `solid = true` (no tileset) ou em camadas
//! com a propriedade `collision = true` são sólidos; `one_way = true` vira
//! plataforma de mão única. Veja [`TileMap::collision_grid`].

mod tiled;

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use glam::{UVec2, Vec2};
use nn_core::Color;
use nn_physics::{Cell, CollisionGrid};
use nn_render::{Camera2D, Rect, Renderer, Sprite, SpriteSheet, TextureId};

pub use tiled::{load_tiled, read_from};

/// Bits de espelhamento que o Tiled guarda no id do tile.
pub const FLIP_H: u32 = 0x8000_0000;
pub const FLIP_V: u32 = 0x4000_0000;
pub const FLIP_D: u32 = 0x2000_0000;
const FLIP_MASK: u32 = FLIP_H | FLIP_V | FLIP_D | 0x1000_0000;

/// Erros ao montar ou importar mapas.
#[derive(Debug, thiserror::Error)]
pub enum TilemapError {
    #[error("{path}: JSON inválido: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("{path}: {message}")]
    Unsupported { path: PathBuf, message: String },
    #[error("falha ao ler {path}: {message}")]
    Io { path: PathBuf, message: String },
    #[error("{0}")]
    Invalid(String),
}

/// Valor de uma propriedade personalizada.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

impl Property {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

/// Propriedades personalizadas (nome → valor).
pub type Properties = BTreeMap<String, Property>;

/// Animação de um tile: a sequência de tiles (ids locais do mesmo tileset)
/// e quanto tempo cada um fica na tela.
#[derive(Debug, Clone, PartialEq)]
pub struct TileAnimation {
    /// (id local do tile, duração em segundos).
    pub frames: Vec<(u32, f32)>,
}

impl TileAnimation {
    /// Quadros com a mesma duração.
    pub fn uniform(frames: &[u32], seconds_each: f32) -> Self {
        Self {
            frames: frames.iter().map(|&f| (f, seconds_each)).collect(),
        }
    }

    /// Duração de uma volta completa.
    pub fn duration(&self) -> f32 {
        self.frames.iter().map(|(_, d)| d.max(0.0)).sum()
    }

    /// Tile que aparece no instante `time` (em segundos, repetindo).
    pub fn frame_at(&self, time: f32) -> Option<u32> {
        let total = self.duration();
        if total <= 0.0 {
            return self.frames.first().map(|(f, _)| *f);
        }
        let mut t = time.rem_euclid(total);
        for &(frame, d) in &self.frames {
            if t < d {
                return Some(frame);
            }
            t -= d.max(0.0);
        }
        self.frames.last().map(|(f, _)| *f)
    }
}

/// Uma imagem dividida em tiles.
#[derive(Debug, Clone, PartialEq)]
pub struct Tileset {
    pub name: String,
    /// Primeiro id global deste tileset (o Tiled começa em 1; 0 é "vazio").
    pub first_gid: u32,
    /// Caminho da imagem, relativo à pasta de assets.
    pub image: PathBuf,
    pub image_size: UVec2,
    pub tile_size: UVec2,
    pub columns: u32,
    pub tile_count: u32,
    pub margin: u32,
    pub spacing: u32,
    /// Propriedades por tile (id local, a partir de 0).
    pub tile_properties: BTreeMap<u32, Properties>,
    /// Tiles animados (id local → animação). Vem do Tiled ou de [`TileMap::animate`].
    pub animations: BTreeMap<u32, TileAnimation>,
    /// Textura carregada (veja [`TileMap::load_textures`]).
    pub texture: Option<TextureId>,
}

impl Tileset {
    /// Tileset a partir de uma spritesheet já carregada: o quadro `n` (a partir
    /// de 0) vira o id global `first_gid + n`.
    pub fn from_sheet(sheet: &SpriteSheet, texture_size: UVec2, first_gid: u32) -> Self {
        Self {
            name: String::new(),
            first_gid,
            image: PathBuf::new(),
            image_size: texture_size,
            tile_size: sheet.frame_size(),
            columns: sheet.columns(),
            tile_count: sheet.frame_count(),
            margin: 0,
            spacing: 0,
            tile_properties: BTreeMap::new(),
            animations: BTreeMap::new(),
            texture: Some(sheet.texture()),
        }
    }

    fn contains(&self, gid: u32) -> bool {
        gid >= self.first_gid && gid < self.first_gid + self.tile_count
    }

    /// Região da imagem do tile (id local).
    fn region(&self, local: u32) -> Rect {
        let col = local % self.columns.max(1);
        let row = local / self.columns.max(1);
        let x = self.margin + col * (self.tile_size.x + self.spacing);
        let y = self.margin + row * (self.tile_size.y + self.spacing);
        // Recuo mínimo nas bordas: evita "vazar" o tile vizinho da imagem.
        let inset = 0.01;
        let size = self.image_size.as_vec2().max(Vec2::ONE);
        Rect::new(
            (x as f32 + inset) / size.x,
            (y as f32 + inset) / size.y,
            (self.tile_size.x as f32 - 2.0 * inset) / size.x,
            (self.tile_size.y as f32 - 2.0 * inset) / size.y,
        )
    }
}

/// Uma camada de tiles.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub name: String,
    /// Ids globais, linha a linha de cima para baixo (0 = vazio).
    pub tiles: Vec<u32>,
    pub visible: bool,
    pub opacity: f32,
    pub properties: Properties,
}

/// Objeto do mapa (ponto ou retângulo), em coordenadas do mundo.
#[derive(Debug, Clone, PartialEq)]
pub struct MapObject {
    pub id: u32,
    pub name: String,
    /// Tipo/classe definido no editor ("moeda", "inimigo"...).
    pub kind: String,
    /// Centro do objeto no mundo.
    pub position: Vec2,
    pub size: Vec2,
    pub point: bool,
    /// Camada de objetos de onde veio.
    pub layer: String,
    pub properties: Properties,
}

/// Mapa de tiles.
#[derive(Debug, Clone, PartialEq)]
pub struct TileMap {
    /// Largura em tiles.
    pub width: u32,
    /// Altura em tiles.
    pub height: u32,
    /// Tamanho de uma célula no mundo.
    pub tile_size: Vec2,
    /// Tamanho de uma célula em pixels da imagem do tileset. A razão
    /// `tile_size / tile_px` é a escala do desenho (tile de 16 px ocupando 32
    /// unidades = escala 2).
    pub tile_px: Vec2,
    /// Canto inferior esquerdo do mapa no mundo.
    pub origin: Vec2,
    pub layers: Vec<Layer>,
    pub tilesets: Vec<Tileset>,
    pub objects: Vec<MapObject>,
    pub properties: Properties,
    /// Relógio dos tiles animados, em segundos (veja [`TileMap::update`]).
    pub time: f32,
}

/// Definição de um caractere em [`TileMap::from_ascii`].
#[derive(Debug, Clone, PartialEq)]
pub enum AsciiTile {
    /// Tile (quadro da spritesheet, a partir de 0) com o tipo de colisão.
    Tile { frame: u32, cell: Cell },
    /// Célula só de colisão, sem desenho.
    Collision(Cell),
    /// Cria um objeto deste tipo no centro da célula.
    Object(String),
}

impl TileMap {
    /// Mapa vazio com uma camada.
    pub fn new(width: u32, height: u32, tile_size: Vec2) -> Self {
        Self {
            width,
            height,
            tile_size,
            tile_px: tile_size,
            origin: Vec2::ZERO,
            layers: vec![Layer {
                name: "tiles".into(),
                tiles: vec![0; (width * height) as usize],
                visible: true,
                opacity: 1.0,
                properties: Properties::new(),
            }],
            tilesets: Vec::new(),
            objects: Vec::new(),
            properties: Properties::new(),
            time: 0.0,
        }
    }

    /// Avança o relógio dos tiles animados. Chame uma vez por frame.
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
    }

    /// Anima um tile: onde aparecer o tile `gid`, os tiles `frames` (ids
    /// globais do mesmo tileset) se revezam, `fps` por segundo.
    /// Devolve falso se algum id não for do tileset de `gid`.
    pub fn animate(&mut self, gid: u32, frames: &[u32], fps: f32) -> bool {
        let gid = gid & !FLIP_MASK;
        let Some(ts) = self.tilesets.iter_mut().rev().find(|t| t.contains(gid)) else {
            return false;
        };
        if frames.is_empty() || !frames.iter().all(|f| ts.contains(*f)) {
            return false;
        }
        let local: Vec<u32> = frames.iter().map(|f| f - ts.first_gid).collect();
        let seconds = if fps > 0.0 { 1.0 / fps } else { f32::INFINITY };
        ts.animations
            .insert(gid - ts.first_gid, TileAnimation::uniform(&local, seconds));
        true
    }

    /// Monta um mapa a partir de linhas de texto. Cada caractere é procurado
    /// em `legend`; os desconhecidos (e espaços e pontos) ficam vazios.
    ///
    /// ```
    /// # use nn_tilemap::{AsciiTile, TileMap};
    /// # use nn_physics::Cell;
    /// # use glam::Vec2;
    /// let legenda = |c: char| match c {
    ///     '#' => Some(AsciiTile::Tile { frame: 0, cell: Cell::Solid }),
    ///     'C' => Some(AsciiTile::Object("moeda".into())),
    ///     _ => None,
    /// };
    /// let mapa = TileMap::from_ascii(&["#..C", "####"], Vec2::splat(16.0), legenda);
    /// assert_eq!((mapa.width, mapa.height), (4, 2));
    /// assert_eq!(mapa.objects.len(), 1);
    /// ```
    pub fn from_ascii(
        rows: &[&str],
        tile_size: Vec2,
        mut legend: impl FnMut(char) -> Option<AsciiTile>,
    ) -> Self {
        let width = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0) as u32;
        let height = rows.len() as u32;
        let mut map = Self::new(width, height, tile_size);
        let mut collision = Layer {
            name: "colisao".into(),
            tiles: vec![0; (width * height) as usize],
            visible: false,
            opacity: 1.0,
            properties: Properties::new(),
        };
        let mut next_id = 1;
        for (row, line) in rows.iter().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                let index = row * width as usize + col;
                match legend(ch) {
                    Some(AsciiTile::Tile { frame, cell }) => {
                        map.layers[0].tiles[index] = frame + 1;
                        collision.tiles[index] = cell_code(cell);
                    }
                    Some(AsciiTile::Collision(cell)) => {
                        collision.tiles[index] = cell_code(cell);
                    }
                    Some(AsciiTile::Object(kind)) => {
                        let position = map.cell_center(col as u32, row as u32);
                        map.objects.push(MapObject {
                            id: next_id,
                            name: String::new(),
                            kind,
                            position,
                            size: tile_size,
                            point: false,
                            layer: "objetos".into(),
                            properties: Properties::new(),
                        });
                        next_id += 1;
                    }
                    None => {}
                }
            }
        }
        // A camada de colisão guarda o tipo de célula diretamente.
        collision
            .properties
            .insert("nn_collision_codes".into(), Property::Bool(true));
        map.layers.push(collision);
        map
    }

    /// Usa uma spritesheet como tileset: o quadro `n` (a partir de 0) é o
    /// tile `n + 1`. Os tiles são esticados para ocupar a célula inteira.
    pub fn add_sheet(&mut self, sheet: &SpriteSheet, texture_size: UVec2) {
        let first_gid = self
            .tilesets
            .iter()
            .map(|t| t.first_gid + t.tile_count)
            .max()
            .unwrap_or(1);
        if self.tilesets.is_empty() {
            self.tile_px = sheet.frame_size().as_vec2();
        }
        self.tilesets
            .push(Tileset::from_sheet(sheet, texture_size, first_gid));
    }

    /// Escala do desenho: unidades do mundo por pixel da imagem.
    pub fn scale(&self) -> Vec2 {
        self.tile_size / self.tile_px.max(Vec2::ONE)
    }

    /// Tamanho do mapa no mundo.
    pub fn world_size(&self) -> Vec2 {
        Vec2::new(self.width as f32, self.height as f32) * self.tile_size
    }

    /// Retângulo que o mapa ocupa no mundo (bom limite para a câmera).
    pub fn bounds(&self) -> nn_render::Bounds {
        nn_render::Bounds::from_size(self.origin, self.world_size())
    }

    /// Centro da célula (coluna e linha a partir de 0, linha 0 em cima).
    pub fn cell_center(&self, col: u32, row: u32) -> Vec2 {
        self.origin
            + Vec2::new(
                col as f32 + 0.5,
                (self.height - 1 - row.min(self.height.saturating_sub(1))) as f32 + 0.5,
            ) * self.tile_size
    }

    /// Célula (coluna, linha a partir de cima) que contém o ponto do mundo.
    pub fn cell_at(&self, p: Vec2) -> Option<(u32, u32)> {
        let local = (p - self.origin) / self.tile_size;
        if local.x < 0.0 || local.y < 0.0 {
            return None;
        }
        let col = local.x as u32;
        let row_from_bottom = local.y as u32;
        if col >= self.width || row_from_bottom >= self.height {
            return None;
        }
        Some((col, self.height - 1 - row_from_bottom))
    }

    fn index(&self, col: u32, row: u32) -> Option<usize> {
        (col < self.width && row < self.height).then(|| (row * self.width + col) as usize)
    }

    /// Id global do tile (sem bits de espelhamento) na camada `layer`.
    pub fn tile(&self, layer: usize, col: u32, row: u32) -> u32 {
        let Some(i) = self.index(col, row) else {
            return 0;
        };
        self.layers
            .get(layer)
            .map_or(0, |l| l.tiles[i] & !FLIP_MASK)
    }

    /// Troca o tile de uma célula. Devolve falso se estiver fora do mapa.
    pub fn set_tile(&mut self, layer: usize, col: u32, row: u32, gid: u32) -> bool {
        let Some(i) = self.index(col, row) else {
            return false;
        };
        match self.layers.get_mut(layer) {
            Some(l) => {
                l.tiles[i] = gid;
                true
            }
            None => false,
        }
    }

    /// Índice da camada pelo nome.
    pub fn layer_index(&self, name: &str) -> Option<usize> {
        self.layers.iter().position(|l| l.name == name)
    }

    fn tileset_for(&self, gid: u32) -> Option<&Tileset> {
        self.tilesets.iter().rev().find(|t| t.contains(gid))
    }

    /// Propriedades do tile com este id global.
    pub fn tile_properties(&self, gid: u32) -> Option<&Properties> {
        let gid = gid & !FLIP_MASK;
        let ts = self.tileset_for(gid)?;
        ts.tile_properties.get(&(gid - ts.first_gid))
    }

    /// Tipo de colisão de uma célula, combinando todas as camadas.
    pub fn collision_cell(&self, col: u32, row: u32) -> Cell {
        let Some(i) = self.index(col, row) else {
            return Cell::Empty;
        };
        let mut result = Cell::Empty;
        for layer in &self.layers {
            let raw = layer.tiles[i];
            if raw == 0 {
                continue;
            }
            let cell = if layer.properties.contains_key("nn_collision_codes") {
                code_cell(raw)
            } else {
                let gid = raw & !FLIP_MASK;
                let prop = |name: &str| {
                    self.tile_properties(gid)
                        .and_then(|p| p.get(name))
                        .and_then(Property::as_bool)
                        .or_else(|| layer.properties.get(name).and_then(Property::as_bool))
                };
                if prop("one_way") == Some(true) {
                    Cell::OneWay
                } else if prop("solid") == Some(true) || prop("collision") == Some(true) {
                    Cell::Solid
                } else {
                    Cell::Empty
                }
            };
            result = match (result, cell) {
                (Cell::Solid, _) | (_, Cell::Solid) => Cell::Solid,
                (Cell::OneWay, _) | (_, Cell::OneWay) => Cell::OneWay,
                _ => Cell::Empty,
            };
        }
        result
    }

    /// Grade de colisão para o mundo físico.
    pub fn collision_grid(&self) -> CollisionGrid {
        let mut grid = CollisionGrid::new(self.width, self.height, self.tile_size, self.origin);
        for row in 0..self.height {
            for col in 0..self.width {
                let cell = self.collision_cell(col, row);
                if cell != Cell::Empty {
                    grid.set(col as i32, (self.height - 1 - row) as i32, cell);
                }
            }
        }
        grid
    }

    /// Carrega as imagens dos tilesets que ainda não têm textura.
    pub fn load_textures<E>(
        &mut self,
        mut load: impl FnMut(&Path) -> Result<TextureId, E>,
    ) -> Result<(), E> {
        for ts in &mut self.tilesets {
            if ts.texture.is_none() {
                ts.texture = Some(load(&ts.image)?);
            }
        }
        Ok(())
    }

    /// Desenha todas as camadas visíveis, só na área que a câmera enxerga.
    pub fn draw(&self, renderer: &mut Renderer, camera: &Camera2D, viewport: Vec2) {
        for i in 0..self.layers.len() {
            if self.layers[i].visible {
                self.draw_layer(i, renderer, camera, viewport);
            }
        }
    }

    /// Desenha uma camada.
    pub fn draw_layer(
        &self,
        layer: usize,
        renderer: &mut Renderer,
        camera: &Camera2D,
        viewport: Vec2,
    ) {
        let Some(l) = self.layers.get(layer) else {
            return;
        };
        if l.properties.contains_key("nn_collision_codes") {
            return;
        }
        let (c0, r0, c1, r1) = self.visible_cells(camera, viewport);
        let tint = Color::WHITE.with_alpha(l.opacity);
        for row in r0..=r1 {
            for col in c0..=c1 {
                let raw = l.tiles[(row * self.width + col) as usize];
                let gid = raw & !FLIP_MASK;
                if gid == 0 {
                    continue;
                }
                let Some(ts) = self.tileset_for(gid) else {
                    continue;
                };
                let Some(texture) = ts.texture else { continue };
                let mut local = gid - ts.first_gid;
                if let Some(frame) = ts
                    .animations
                    .get(&local)
                    .and_then(|a| a.frame_at(self.time))
                {
                    local = frame;
                }
                let size = ts.tile_size.as_vec2() * self.scale();
                let (flip_x, quarter) = flip_transform(raw);
                let mut sprite_size = size;
                if flip_x {
                    sprite_size.x = -sprite_size.x;
                }
                // Tiles maiores que a célula ficam alinhados pelo canto inferior esquerdo, como no Tiled.
                let bottom_left = self.origin
                    + Vec2::new(col as f32, (self.height - 1 - row) as f32) * self.tile_size;
                renderer.draw(
                    &Sprite::new(texture)
                        .at(bottom_left + size / 2.0)
                        .size(sprite_size)
                        .region(ts.region(local))
                        .rotation(quarter as f32 * std::f32::consts::FRAC_PI_2)
                        .color(tint),
                );
            }
        }
    }

    /// Faixa de células visíveis (inclusiva), com uma célula de folga.
    fn visible_cells(&self, camera: &Camera2D, viewport: Vec2) -> (u32, u32, u32, u32) {
        if self.width == 0 || self.height == 0 {
            return (1, 1, 0, 0);
        }
        // Com rotação, a área visível é o círculo que envolve a tela.
        let half = if camera.rotation == 0.0 {
            viewport / (2.0 * camera.zoom)
        } else {
            Vec2::splat(viewport.length() / (2.0 * camera.zoom))
        };
        let min = (camera.position - half - self.origin) / self.tile_size;
        let max = (camera.position + half - self.origin) / self.tile_size;
        let clamp_x = |v: f32| (v.floor() as i64).clamp(0, self.width as i64 - 1) as u32;
        let clamp_y = |v: f32| (v.floor() as i64).clamp(0, self.height as i64 - 1) as u32;
        let (c0, c1) = (clamp_x(min.x - 1.0), clamp_x(max.x + 1.0));
        let (b0, b1) = (clamp_y(min.y - 1.0), clamp_y(max.y + 1.0));
        // Linhas do mundo (de baixo) para linhas do mapa (de cima).
        (c0, self.height - 1 - b1, c1, self.height - 1 - b0)
    }

    /// Objetos de um tipo.
    pub fn objects_of<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a MapObject> + 'a {
        self.objects.iter().filter(move |o| o.kind == kind)
    }

    /// Primeiro objeto com este nome.
    pub fn object(&self, name: &str) -> Option<&MapObject> {
        self.objects.iter().find(|o| o.name == name)
    }
}

/// Como desenhar os bits de espelhamento do Tiled (H, V e D, a diagonal:
/// girar 90° com espelho) com um sprite: espelhar na horizontal (antes) e
/// quantos quartos de volta girar no sentido anti-horário.
///
/// O Tiled aplica a diagonal primeiro, depois H e V. Com as matrizes que levam
/// os eixos da imagem (direita, baixo) para a tela (direita, baixo), procura a
/// combinação de giro e espelho que dá o mesmo resultado.
pub fn flip_transform(raw: u32) -> (bool, u32) {
    type M = [[i32; 2]; 2];
    fn mul(a: M, b: M) -> M {
        [
            [
                a[0][0] * b[0][0] + a[0][1] * b[1][0],
                a[0][0] * b[0][1] + a[0][1] * b[1][1],
            ],
            [
                a[1][0] * b[0][0] + a[1][1] * b[1][0],
                a[1][0] * b[0][1] + a[1][1] * b[1][1],
            ],
        ]
    }
    const I: M = [[1, 0], [0, 1]];
    const D: M = [[0, 1], [1, 0]];
    const H: M = [[-1, 0], [0, 1]];
    const V: M = [[1, 0], [0, -1]];
    // Um quarto de volta anti-horário no mundo (Y para cima), visto na tela.
    const R: M = [[0, 1], [-1, 0]];
    let mut target = I;
    if raw & FLIP_D != 0 {
        target = mul(D, target);
    }
    if raw & FLIP_H != 0 {
        target = mul(H, target);
    }
    if raw & FLIP_V != 0 {
        target = mul(V, target);
    }
    let mut rot = I;
    for quarter in 0..4 {
        if rot == target {
            return (false, quarter);
        }
        if mul(rot, H) == target {
            return (true, quarter);
        }
        rot = mul(R, rot);
    }
    (false, 0)
}

fn cell_code(cell: Cell) -> u32 {
    match cell {
        Cell::Empty => 0,
        Cell::Solid => 1,
        Cell::OneWay => 2,
    }
}

fn code_cell(code: u32) -> Cell {
    match code {
        1 => Cell::Solid,
        2 => Cell::OneWay,
        _ => Cell::Empty,
    }
}

/// Junta `base` e `relative` e resolve `.` e `..` sem tocar no disco.
pub(crate) fn join_normalized(base: &Path, relative: &Path) -> PathBuf {
    let joined = base.join(relative);
    let mut out: Vec<Component> = Vec::new();
    for c in joined.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(out.last(), Some(Component::Normal(_))) {
                    out.pop();
                } else {
                    out.push(c);
                }
            }
            other => out.push(other),
        }
    }
    out.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legenda(c: char) -> Option<AsciiTile> {
        match c {
            '#' => Some(AsciiTile::Tile {
                frame: 0,
                cell: Cell::Solid,
            }),
            '=' => Some(AsciiTile::Tile {
                frame: 1,
                cell: Cell::OneWay,
            }),
            '~' => Some(AsciiTile::Tile {
                frame: 2,
                cell: Cell::Empty,
            }),
            'x' => Some(AsciiTile::Collision(Cell::Solid)),
            'C' => Some(AsciiTile::Object("moeda".into())),
            _ => None,
        }
    }

    fn mapa() -> TileMap {
        TileMap::from_ascii(&["x...C", "..==.", "#~###"], Vec2::splat(10.0), legenda)
    }

    #[test]
    fn ascii_monta_tiles_colisao_e_objetos() {
        let m = mapa();
        assert_eq!((m.width, m.height), (5, 3));
        assert_eq!(m.tile(0, 0, 2), 1);
        assert_eq!(m.tile(0, 2, 1), 2);
        assert_eq!(m.collision_cell(0, 2), Cell::Solid);
        assert_eq!(m.collision_cell(1, 2), Cell::Empty);
        assert_eq!(m.collision_cell(2, 1), Cell::OneWay);
        assert_eq!(m.collision_cell(0, 0), Cell::Solid); // só colisão
        assert_eq!(m.tile(0, 0, 0), 0);
        let moeda = &m.objects[0];
        assert_eq!(moeda.kind, "moeda");
        assert_eq!(moeda.position, Vec2::new(45.0, 25.0));
    }

    #[test]
    fn grade_com_y_para_cima() {
        let g = mapa().collision_grid();
        // Linha de baixo do mapa é a linha 0 da grade.
        assert_eq!(g.get(0, 0), Cell::Solid);
        assert_eq!(g.get(2, 1), Cell::OneWay);
        assert_eq!(g.get(0, 2), Cell::Solid);
    }

    #[test]
    fn celulas_e_mundo() {
        let mut m = mapa();
        m.origin = Vec2::new(100.0, 0.0);
        assert_eq!(m.cell_center(0, 2), Vec2::new(105.0, 5.0));
        assert_eq!(m.cell_at(Vec2::new(105.0, 5.0)), Some((0, 2)));
        assert_eq!(m.cell_at(Vec2::new(149.0, 29.0)), Some((4, 0)));
        assert_eq!(m.cell_at(Vec2::new(99.0, 5.0)), None);
        assert_eq!(m.world_size(), Vec2::new(50.0, 30.0));
    }

    #[test]
    fn trocar_tile_muda_colisao() {
        let mut m = mapa();
        let colisao = m.layer_index("colisao").unwrap();
        assert!(m.set_tile(colisao, 2, 2, 0));
        assert_eq!(m.collision_cell(2, 2), Cell::Empty);
        assert!(!m.set_tile(0, 99, 0, 1));
    }

    #[test]
    fn giros_do_tiled() {
        assert_eq!(flip_transform(5), (false, 0));
        assert_eq!(flip_transform(5 | FLIP_H), (true, 0));
        // Só V: espelho horizontal e meia volta.
        assert_eq!(flip_transform(5 | FLIP_V), (true, 2));
        assert_eq!(flip_transform(5 | FLIP_H | FLIP_V), (false, 2));
        // Girar 90° no sentido horário, no Tiled, é D + H.
        assert_eq!(flip_transform(5 | FLIP_D | FLIP_H), (false, 3));
        // 90° anti-horário é D + V.
        assert_eq!(flip_transform(5 | FLIP_D | FLIP_V), (false, 1));
        assert_eq!(flip_transform(5 | FLIP_D), (true, 1));
        assert_eq!(flip_transform(5 | FLIP_D | FLIP_H | FLIP_V), (true, 3));
    }

    #[test]
    fn caminhos_normalizados() {
        assert_eq!(
            join_normalized(Path::new("fases"), Path::new("../imagens/tiles.png")),
            PathBuf::from("imagens/tiles.png")
        );
        assert_eq!(
            join_normalized(Path::new(""), Path::new("./a/b.png")),
            PathBuf::from("a/b.png")
        );
    }

    #[test]
    fn celulas_visiveis_recortam_pela_camera() {
        let m = TileMap::new(100, 100, Vec2::splat(10.0));
        let cam = Camera2D::new(Vec2::new(500.0, 500.0));
        let (c0, r0, c1, r1) = m.visible_cells(&cam, Vec2::new(100.0, 60.0));
        assert!(c0 >= 43 && c1 <= 56, "{c0}..{c1}");
        assert!(r0 >= 45 && r1 <= 54, "{r0}..{r1}");
    }
}
