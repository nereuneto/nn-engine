//! Caminhos da nn-engine.
//!
//! Uma [`NavGrid`] divide o chão em células quadradas e guarda, para cada
//! uma, se dá para andar nela e a altura do chão (no 3D). [`NavGrid::path`]
//! acha um caminho com A* (8 direções, sem cortar quina de parede) e depois
//! tira os zigue-zagues: sobram só os pontos onde é preciso virar.
//!
//! Quem monta a grade é a física: no 2D, os corpos parados e os tiles
//! sólidos bloqueiam; no 3D, um raio procura o chão de cada célula e uma
//! caixa confere se cabe alguém em pé ali. Entre duas células, a diferença
//! de altura do chão não pode passar do degrau ([`NavGrid::max_step`]):
//! assim um buraco ou um barranco também bloqueiam.
//!
//! As coordenadas são as do chão: `x` e `y` no 2D, `x` e `z` no 3D (o `y` de
//! [`glam::Vec2`] faz o papel do `z`).

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use glam::Vec2;

/// Quantas células o A* pode abrir num pedido (para um caminho impossível
/// não travar o jogo).
pub const MAX_EXPANDED: usize = 60_000;

/// Uma célula da grade: coluna e linha.
pub type CellPos = (u32, u32);

/// A grade de caminhos.
#[derive(Debug, Clone, PartialEq)]
pub struct NavGrid {
    /// Canto de menor x e menor y (ou z) da grade.
    pub origin: Vec2,
    /// Lado de cada célula.
    pub cell: f32,
    width: u32,
    height: u32,
    walkable: Vec<bool>,
    floor: Vec<f32>,
    /// Maior diferença de altura do chão entre duas células vizinhas que
    /// ainda dá para atravessar (`INFINITY` no 2D).
    pub max_step: f32,
}

/// Um caminho achado por [`NavGrid::path`].
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    /// Os pontos, do começo ao fim (o primeiro é a origem pedida).
    pub points: Vec<Vec2>,
    /// A altura do chão em cada ponto (0 no 2D).
    pub heights: Vec<f32>,
    /// Chega ao destino? (`false`: o destino não tem caminho; os pontos
    /// levam o mais perto possível.)
    pub complete: bool,
}

#[derive(Clone, Copy, PartialEq)]
struct Open {
    f: f32,
    g: f32,
    i: u32,
}

impl Eq for Open {}

impl Ord for Open {
    fn cmp(&self, o: &Self) -> Ordering {
        // BinaryHeap é de máximo: o menor f (e, empatado, o maior g) primeiro.
        o.f.total_cmp(&self.f).then(self.g.total_cmp(&o.g))
    }
}

impl PartialOrd for Open {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

const DIRS: [(i32, i32); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];

impl NavGrid {
    /// Uma grade cobrindo de `min` a `max`, com células de lado `cell`, toda
    /// andável e com o chão na altura 0.
    pub fn new(min: Vec2, max: Vec2, cell: f32) -> Self {
        let cell = cell.max(1e-3);
        let size = (max - min).max(Vec2::splat(cell));
        let width = ((size.x / cell).ceil() as u32).clamp(1, 4096);
        let height = ((size.y / cell).ceil() as u32).clamp(1, 4096);
        let n = (width * height) as usize;
        Self {
            origin: min.min(max),
            cell,
            width,
            height,
            walkable: vec![true; n],
            floor: vec![0.0; n],
            max_step: f32::INFINITY,
        }
    }

    /// Colunas.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Linhas.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// O canto oposto à origem.
    pub fn max(&self) -> Vec2 {
        self.origin + Vec2::new(self.width as f32, self.height as f32) * self.cell
    }

    fn index(&self, (cx, cy): CellPos) -> usize {
        (cy * self.width + cx) as usize
    }

    /// A célula de um ponto (`None` fora da grade).
    pub fn cell_of(&self, p: Vec2) -> Option<CellPos> {
        let q = (p - self.origin) / self.cell;
        if q.x < 0.0 || q.y < 0.0 {
            return None;
        }
        let (cx, cy) = (q.x as u32, q.y as u32);
        (cx < self.width && cy < self.height).then_some((cx, cy))
    }

    /// O centro de uma célula.
    pub fn center(&self, (cx, cy): CellPos) -> Vec2 {
        self.origin + (Vec2::new(cx as f32, cy as f32) + 0.5) * self.cell
    }

    /// Dá para andar nesta célula?
    pub fn is_walkable(&self, c: CellPos) -> bool {
        c.0 < self.width && c.1 < self.height && self.walkable[self.index(c)]
    }

    /// Dá para andar neste ponto? (Fora da grade: não.)
    pub fn walkable_at(&self, p: Vec2) -> bool {
        self.cell_of(p).is_some_and(|c| self.is_walkable(c))
    }

    pub fn set_walkable(&mut self, c: CellPos, walkable: bool) {
        if c.0 < self.width && c.1 < self.height {
            let i = self.index(c);
            self.walkable[i] = walkable;
        }
    }

    /// A altura do chão da célula.
    pub fn floor(&self, c: CellPos) -> f32 {
        if c.0 < self.width && c.1 < self.height {
            self.floor[self.index(c)]
        } else {
            0.0
        }
    }

    /// A altura do chão num ponto (a da célula dele; 0 fora da grade).
    pub fn floor_at(&self, p: Vec2) -> f32 {
        self.cell_of(p).map_or(0.0, |c| self.floor(c))
    }

    pub fn set_floor(&mut self, c: CellPos, h: f32) {
        if c.0 < self.width && c.1 < self.height {
            let i = self.index(c);
            self.floor[i] = h;
        }
    }

    /// As células cujo centro fica dentro do retângulo de `min` a `max`.
    pub fn cells_in(&self, min: Vec2, max: Vec2) -> impl Iterator<Item = CellPos> + '_ {
        let (lo, hi) = (min.min(max), min.max(max));
        let a = ((lo - self.origin) / self.cell - 0.5)
            .ceil()
            .max(Vec2::ZERO);
        let b = ((hi - self.origin) / self.cell - 0.5).floor();
        let (x0, y0) = (a.x as i64, a.y as i64);
        let x1 = (b.x as i64).min(self.width as i64 - 1);
        let y1 = (b.y as i64).min(self.height as i64 - 1);
        (y0..=y1).flat_map(move |y| (x0..=x1).map(move |x| (x as u32, y as u32)))
    }

    /// Bloqueia as células cujo centro fica no retângulo.
    pub fn block_rect(&mut self, min: Vec2, max: Vec2) {
        let cells: Vec<CellPos> = self.cells_in(min, max).collect();
        for c in cells {
            self.set_walkable(c, false);
        }
    }

    /// Bloqueia as células cujo centro fica a menos de `radius` de `center`.
    pub fn block_circle(&mut self, center: Vec2, radius: f32) {
        let cells: Vec<CellPos> = self
            .cells_in(center - radius, center + radius)
            .filter(|&c| self.center(c).distance(center) <= radius)
            .collect();
        for c in cells {
            self.set_walkable(c, false);
        }
    }

    /// Bloqueia, no retângulo, as células com o chão abaixo de `level` (água,
    /// lava).
    pub fn block_below(&mut self, min: Vec2, max: Vec2, level: f32) {
        let cells: Vec<CellPos> = self
            .cells_in(min, max)
            .filter(|&c| self.floor(c) < level)
            .collect();
        for c in cells {
            self.set_walkable(c, false);
        }
    }

    /// Quantas células são andáveis.
    pub fn walkable_count(&self) -> usize {
        self.walkable.iter().filter(|w| **w).count()
    }

    /// As células andáveis (para desenhar a grade).
    pub fn walkable_cells(&self) -> impl Iterator<Item = CellPos> + '_ {
        (0..self.height).flat_map(move |y| {
            (0..self.width)
                .map(move |x| (x, y))
                .filter(move |&c| self.is_walkable(c))
        })
    }

    /// Dá para passar de `a` para a vizinha `b`? (As duas andáveis e o
    /// chão sem um degrau alto demais.)
    fn step_ok(&self, a: CellPos, b: CellPos) -> bool {
        self.is_walkable(b) && (self.floor(a) - self.floor(b)).abs() <= self.max_step
    }

    fn neighbor(&self, c: CellPos, d: (i32, i32)) -> Option<CellPos> {
        let x = c.0 as i64 + d.0 as i64;
        let y = c.1 as i64 + d.1 as i64;
        (x >= 0 && y >= 0 && x < self.width as i64 && y < self.height as i64)
            .then_some((x as u32, y as u32))
    }

    /// Dá para ir de `a` para a vizinha na direção `d`? Na diagonal, as duas
    /// laterais também precisam ser passáveis (não corta quina).
    fn can_move(&self, a: CellPos, d: (i32, i32)) -> Option<CellPos> {
        let b = self.neighbor(a, d)?;
        if !self.step_ok(a, b) {
            return None;
        }
        if d.0 != 0 && d.1 != 0 {
            let s1 = self.neighbor(a, (d.0, 0))?;
            let s2 = self.neighbor(a, (0, d.1))?;
            if !self.step_ok(a, s1) || !self.step_ok(a, s2) {
                return None;
            }
        }
        Some(b)
    }

    /// A célula andável mais perto de `p`, até `max_cells` células de
    /// distância. Um ponto fora da grade usa a borda mais perto.
    pub fn nearest_walkable(&self, p: Vec2, max_cells: u32) -> Option<CellPos> {
        let q = ((p - self.origin) / self.cell).floor();
        let cx = (q.x as i64).clamp(0, self.width as i64 - 1);
        let cy = (q.y as i64).clamp(0, self.height as i64 - 1);
        let start = (cx as u32, cy as u32);
        if self.is_walkable(start) {
            return Some(start);
        }
        let mut best: Option<(f32, CellPos)> = None;
        for r in 1..=max_cells as i64 {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let (x, y) = (cx + dx, cy + dy);
                    if x < 0 || y < 0 || x >= self.width as i64 || y >= self.height as i64 {
                        continue;
                    }
                    let c = (x as u32, y as u32);
                    if self.is_walkable(c) {
                        let d = self.center(c).distance_squared(p);
                        if best.is_none_or(|(bd, _)| d < bd) {
                            best = Some((d, c));
                        }
                    }
                }
            }
            if best.is_some() {
                break;
            }
        }
        best.map(|(_, c)| c)
    }

    /// Dá para ir de `a` até `b` em linha reta, sem passar por célula
    /// bloqueada nem por um degrau alto demais? (A largura de quem anda já
    /// está na grade: as células perto demais das paredes vêm bloqueadas.)
    pub fn line_of_sight(&self, a: Vec2, b: Vec2) -> bool {
        let Some(mut prev) = self.cell_of(a) else {
            return false;
        };
        if !self.is_walkable(prev) {
            return false;
        }
        let len = a.distance(b);
        let steps = ((len / (self.cell * 0.25)).ceil() as usize).max(1);
        for k in 1..=steps {
            let p = a.lerp(b, k as f32 / steps as f32);
            let Some(c) = self.cell_of(p) else {
                return false;
            };
            if c != prev {
                let d = (c.0 as i32 - prev.0 as i32, c.1 as i32 - prev.1 as i32);
                if d.0.abs() > 1 || d.1.abs() > 1 || self.can_move(prev, d).is_none() {
                    return false;
                }
                prev = c;
            }
        }
        true
    }

    /// Um caminho de `from` até `to`. Os pontos começam em `from` e terminam
    /// em `to` (ou, se não houver caminho até lá, no lugar alcançável mais
    /// perto dele). `None`: `from` não fica perto de nenhum lugar andável.
    pub fn path(&self, from: Vec2, to: Vec2) -> Option<Path> {
        let start = self.nearest_walkable(from, 6)?;
        let goal = self
            .nearest_walkable(to, 6)
            .unwrap_or_else(|| self.nearest_walkable(to, 64).unwrap_or(start));
        let n = (self.width * self.height) as usize;
        let mut g = vec![f32::INFINITY; n];
        let mut came: Vec<u32> = vec![u32::MAX; n];
        let mut closed = vec![false; n];
        let h = |c: CellPos| {
            let dx = (c.0 as f32 - goal.0 as f32).abs();
            let dy = (c.1 as f32 - goal.1 as f32).abs();
            // Distância octil (8 direções).
            (dx.max(dy) + (std::f32::consts::SQRT_2 - 1.0) * dx.min(dy)) * self.cell
        };
        let si = self.index(start);
        g[si] = 0.0;
        let mut open = BinaryHeap::new();
        open.push(Open {
            f: h(start),
            g: 0.0,
            i: si as u32,
        });
        let mut best = (h(start), si);
        let mut reached = start == goal;
        let mut expanded = 0;
        while let Some(Open { g: gc, i, .. }) = open.pop() {
            let i = i as usize;
            if closed[i] || gc > g[i] {
                continue;
            }
            closed[i] = true;
            let c = ((i as u32) % self.width, (i as u32) / self.width);
            if c == goal {
                reached = true;
                best = (0.0, i);
                break;
            }
            let hc = h(c);
            if hc < best.0 {
                best = (hc, i);
            }
            expanded += 1;
            if expanded > MAX_EXPANDED {
                break;
            }
            for d in DIRS {
                let Some(nb) = self.can_move(c, d) else {
                    continue;
                };
                let ni = self.index(nb);
                if closed[ni] {
                    continue;
                }
                let step = if d.0 != 0 && d.1 != 0 {
                    std::f32::consts::SQRT_2
                } else {
                    1.0
                } * self.cell;
                let ng = gc + step;
                if ng < g[ni] {
                    g[ni] = ng;
                    came[ni] = i as u32;
                    open.push(Open {
                        f: ng + h(nb),
                        g: ng,
                        i: ni as u32,
                    });
                }
            }
        }
        // As células do caminho, do começo ao fim.
        let mut cells = Vec::new();
        let mut i = best.1;
        loop {
            cells.push(((i as u32) % self.width, (i as u32) / self.width));
            if i == si || came[i] == u32::MAX {
                break;
            }
            i = came[i] as usize;
        }
        cells.reverse();
        let mut pts: Vec<Vec2> = Vec::with_capacity(cells.len() + 2);
        pts.push(from);
        pts.extend(cells.iter().map(|&c| self.center(c)));
        if reached && self.walkable_at(to) && to != from {
            pts.push(to);
        }
        // Tira os pontos do meio que dá para pular em linha reta.
        let mut out = vec![pts[0]];
        let mut k = 0;
        while k < pts.len() - 1 {
            let mut far = k + 1;
            for j in (k + 2..pts.len()).rev() {
                if self.line_of_sight(pts[k], pts[j]) {
                    far = j;
                    break;
                }
            }
            out.push(pts[far]);
            k = far;
        }
        out.dedup_by(|a, b| a.distance_squared(*b) < 1e-8);
        let heights = out.iter().map(|&p| self.floor_at(p)).collect();
        Some(Path {
            points: out,
            heights,
            complete: reached,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Uma grade 10 × 10 (células de 1) com uma parede em x = 5, aberta só
    /// em cima (y = 9).
    fn parede() -> NavGrid {
        let mut g = NavGrid::new(Vec2::ZERO, Vec2::splat(10.0), 1.0);
        for y in 0..9 {
            g.set_walkable((5, y), false);
        }
        g
    }

    fn comprimento(p: &Path) -> f32 {
        p.points.windows(2).map(|w| w[0].distance(w[1])).sum()
    }

    #[test]
    fn linha_reta_quando_nao_ha_nada_no_meio() {
        let g = NavGrid::new(Vec2::ZERO, Vec2::splat(10.0), 1.0);
        let p = g.path(Vec2::new(0.5, 0.5), Vec2::new(9.5, 7.5)).unwrap();
        assert!(p.complete);
        assert_eq!(p.points, vec![Vec2::new(0.5, 0.5), Vec2::new(9.5, 7.5)]);
    }

    #[test]
    fn contorna_a_parede_pela_abertura() {
        let g = parede();
        let (a, b) = (Vec2::new(1.5, 1.5), Vec2::new(8.5, 1.5));
        assert!(!g.line_of_sight(a, b));
        let p = g.path(a, b).unwrap();
        assert!(p.complete);
        assert_eq!(*p.points.first().unwrap(), a);
        assert_eq!(*p.points.last().unwrap(), b);
        // Passa pela abertura (y = 9), e cada trecho é em linha reta.
        assert!(p.points.iter().any(|q| q.y > 8.0), "{:?}", p.points);
        for w in p.points.windows(2) {
            assert!(g.line_of_sight(w[0], w[1]), "{w:?}");
        }
        // Os zigue-zagues foram tirados: poucos pontos, e perto do mais curto.
        assert!(p.points.len() <= 5, "{:?}", p.points);
        assert!(comprimento(&p) < 19.0, "{}", comprimento(&p));
    }

    #[test]
    fn nao_corta_quina_na_diagonal() {
        let mut g = NavGrid::new(Vec2::ZERO, Vec2::splat(3.0), 1.0);
        g.set_walkable((1, 0), false);
        g.set_walkable((0, 1), false);
        // (0,0) → (1,1) só pela diagonal, entre duas paredes: não passa.
        let p = g.path(Vec2::new(0.5, 0.5), Vec2::new(1.5, 1.5)).unwrap();
        assert!(!p.complete);
    }

    #[test]
    fn sem_caminho_leva_ao_lugar_mais_perto() {
        let mut g = parede();
        g.set_walkable((5, 9), false);
        let p = g.path(Vec2::new(1.5, 4.5), Vec2::new(8.5, 4.5)).unwrap();
        assert!(!p.complete);
        let fim = *p.points.last().unwrap();
        assert!((fim.x - 4.5).abs() < 0.01, "{fim}");
    }

    #[test]
    fn degrau_alto_demais_bloqueia() {
        let mut g = NavGrid::new(Vec2::ZERO, Vec2::new(10.0, 3.0), 1.0);
        g.max_step = 0.5;
        // Um barranco de 2 m na coluna 5, menos na linha 2 (uma rampa).
        for y in 0..3 {
            for x in 5..10 {
                g.set_floor((x, y), 2.0);
            }
        }
        for (x, h) in [(3, 0.5), (4, 1.0), (5, 1.5), (6, 2.0)] {
            g.set_floor((x, 2), h);
        }
        let (a, b) = (Vec2::new(0.5, 0.5), Vec2::new(8.5, 0.5));
        assert!(!g.line_of_sight(a, b));
        let p = g.path(a, b).unwrap();
        assert!(p.complete);
        assert!(
            p.points.iter().any(|q| q.y > 2.0),
            "pela rampa: {:?}",
            p.points
        );
        assert_eq!(*p.heights.last().unwrap(), 2.0);
    }

    #[test]
    fn agua_e_retangulos_bloqueiam() {
        let mut g = NavGrid::new(Vec2::ZERO, Vec2::splat(10.0), 1.0);
        for y in 0..10 {
            for x in 0..10 {
                g.set_floor((x, y), if (3..7).contains(&x) { -1.0 } else { 0.0 });
            }
        }
        g.block_below(Vec2::ZERO, Vec2::new(10.0, 8.0), -0.5);
        assert!(!g.walkable_at(Vec2::new(4.5, 2.5)));
        assert!(g.walkable_at(Vec2::new(4.5, 8.5)));
        g.block_rect(Vec2::new(0.0, 0.0), Vec2::new(1.9, 1.9));
        assert!(!g.walkable_at(Vec2::new(0.5, 0.5)));
        assert!(!g.walkable_at(Vec2::new(1.5, 1.5)));
        assert!(g.walkable_at(Vec2::new(2.5, 2.5)));
        g.block_circle(Vec2::new(8.0, 5.0), 1.0);
        assert!(!g.walkable_at(Vec2::new(8.5, 5.5)));
        assert!(g.walkable_at(Vec2::new(9.5, 5.5)));
    }

    #[test]
    fn comeco_dentro_da_parede_sai_pelo_lugar_mais_perto() {
        let g = parede();
        let p = g.path(Vec2::new(5.5, 4.5), Vec2::new(1.5, 4.5)).unwrap();
        assert!(p.complete);
        assert_eq!(p.points[0], Vec2::new(5.5, 4.5));
    }

    #[test]
    fn labirinto_grande_e_rapido() {
        // 200 × 200 com paredes compridas alternadas: o caminho serpenteia.
        let mut g = NavGrid::new(Vec2::ZERO, Vec2::splat(200.0), 1.0);
        for k in 0..19 {
            let x = 10 + k * 10;
            for y in 0..200 {
                let aberta = if k % 2 == 0 { y >= 195 } else { y < 5 };
                if !aberta {
                    g.set_walkable((x, y), false);
                }
            }
        }
        let t = std::time::Instant::now();
        let p = g.path(Vec2::new(0.5, 0.5), Vec2::new(199.5, 0.5)).unwrap();
        assert!(p.complete);
        assert!(comprimento(&p) > 19.0 * 190.0, "{}", comprimento(&p));
        // Folga larga: em debug, num CI lento.
        assert!(t.elapsed().as_secs_f32() < 5.0, "{:?}", t.elapsed());
    }
}
