//! Grade de colisão: o lado sólido de um tilemap.

use glam::Vec2;

use crate::shape::Aabb;

/// Conteúdo de uma célula da grade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cell {
    #[default]
    Empty,
    /// Bloqueia por todos os lados.
    Solid,
    /// Plataforma de mão única: só bloqueia quem vem de cima, caindo.
    OneWay,
}

/// Grade regular de células, com Y para cima. A linha 0 é a de baixo.
#[derive(Debug, Clone, PartialEq)]
pub struct CollisionGrid {
    /// Canto inferior esquerdo da grade no mundo.
    pub origin: Vec2,
    cell_size: Vec2,
    width: u32,
    height: u32,
    cells: Vec<Cell>,
}

impl CollisionGrid {
    /// Grade vazia com `width × height` células.
    ///
    /// # Panics
    /// Se o tamanho da célula não for positivo.
    pub fn new(width: u32, height: u32, cell_size: Vec2, origin: Vec2) -> Self {
        assert!(
            cell_size.x > 0.0 && cell_size.y > 0.0,
            "tamanho de célula inválido"
        );
        Self {
            origin,
            cell_size,
            width,
            height,
            cells: vec![Cell::Empty; (width * height) as usize],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn cell_size(&self) -> Vec2 {
        self.cell_size
    }

    /// Célula na coluna `col` e linha `row` (linha 0 embaixo). Fora da grade é vazio.
    pub fn get(&self, col: i32, row: i32) -> Cell {
        if col < 0 || row < 0 || col >= self.width as i32 || row >= self.height as i32 {
            return Cell::Empty;
        }
        self.cells[(row as u32 * self.width + col as u32) as usize]
    }

    /// Altera uma célula. Fora da grade é ignorado.
    pub fn set(&mut self, col: i32, row: i32, cell: Cell) {
        if col < 0 || row < 0 || col >= self.width as i32 || row >= self.height as i32 {
            return;
        }
        self.cells[(row as u32 * self.width + col as u32) as usize] = cell;
    }

    /// Coluna e linha que contêm o ponto `p` do mundo (podem estar fora da grade).
    pub fn cell_at(&self, p: Vec2) -> (i32, i32) {
        let local = (p - self.origin) / self.cell_size;
        (local.x.floor() as i32, local.y.floor() as i32)
    }

    /// Caixa da célula no mundo.
    pub fn cell_aabb(&self, col: i32, row: i32) -> Aabb {
        let min = self.origin + Vec2::new(col as f32, row as f32) * self.cell_size;
        Aabb::new(min, min + self.cell_size)
    }

    /// Células não vazias que tocam a caixa.
    pub fn cells_in(&self, aabb: &Aabb) -> impl Iterator<Item = (i32, i32, Cell)> + '_ {
        let (c0, r0) = self.cell_at(aabb.min);
        let (c1, r1) = self.cell_at(aabb.max);
        let c0 = c0.max(0);
        let r0 = r0.max(0);
        let c1 = c1.min(self.width as i32 - 1);
        let r1 = r1.min(self.height as i32 - 1);
        (r0..=r1)
            .flat_map(move |r| (c0..=c1).map(move |c| (c, r)))
            .filter_map(move |(c, r)| match self.get(c, r) {
                Cell::Empty => None,
                cell => Some((c, r, cell)),
            })
    }

    /// Raio pela grade (algoritmo DDA). Plataformas de mão única só contam
    /// quando o raio desce e atinge o topo.
    pub(crate) fn raycast(&self, origin: Vec2, dir: Vec2, max: f32) -> Option<(f32, Vec2)> {
        let (mut col, mut row) = self.cell_at(origin);
        let step_x = if dir.x > 0.0 { 1 } else { -1 };
        let step_y = if dir.y > 0.0 { 1 } else { -1 };
        let next_boundary = |cell: i32, step: i32, axis: usize| {
            let o = if axis == 0 {
                self.origin.x
            } else {
                self.origin.y
            };
            let s = if axis == 0 {
                self.cell_size.x
            } else {
                self.cell_size.y
            };
            o + (cell + i32::from(step > 0)) as f32 * s
        };
        let mut t_max_x = if dir.x.abs() < 1e-9 {
            f32::INFINITY
        } else {
            (next_boundary(col, step_x, 0) - origin.x) / dir.x
        };
        let mut t_max_y = if dir.y.abs() < 1e-9 {
            f32::INFINITY
        } else {
            (next_boundary(row, step_y, 1) - origin.y) / dir.y
        };
        let t_delta_x = if dir.x.abs() < 1e-9 {
            f32::INFINITY
        } else {
            self.cell_size.x / dir.x.abs()
        };
        let t_delta_y = if dir.y.abs() < 1e-9 {
            f32::INFINITY
        } else {
            self.cell_size.y / dir.y.abs()
        };

        // Limite de passos para não percorrer infinitamente fora da grade.
        let max_steps = (self.width + self.height) as usize * 2 + 4;
        for _ in 0..max_steps {
            let (t, normal) = if t_max_x < t_max_y {
                col += step_x;
                let t = t_max_x;
                t_max_x += t_delta_x;
                (t, Vec2::new(-step_x as f32, 0.0))
            } else {
                row += step_y;
                let t = t_max_y;
                t_max_y += t_delta_y;
                (t, Vec2::new(0.0, -step_y as f32))
            };
            if t > max {
                return None;
            }
            match self.get(col, row) {
                Cell::Solid => return Some((t, normal)),
                Cell::OneWay if normal == Vec2::Y => return Some((t, normal)),
                _ => {}
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid() -> CollisionGrid {
        let mut g = CollisionGrid::new(10, 5, Vec2::splat(16.0), Vec2::ZERO);
        for c in 0..10 {
            g.set(c, 0, Cell::Solid);
        }
        g.set(5, 3, Cell::OneWay);
        g
    }

    #[test]
    fn celulas_e_coordenadas() {
        let g = grid();
        assert_eq!(g.cell_at(Vec2::new(17.0, 1.0)), (1, 0));
        assert_eq!(g.get(3, 0), Cell::Solid);
        assert_eq!(g.get(-1, 0), Cell::Empty);
        assert_eq!(g.cell_aabb(1, 2).min, Vec2::new(16.0, 32.0));
        let caixa = Aabb::new(Vec2::new(10.0, 10.0), Vec2::new(40.0, 20.0));
        let solidas: Vec<_> = g.cells_in(&caixa).collect();
        assert_eq!(solidas.len(), 3);
    }

    #[test]
    fn raio_para_baixo_acha_o_chao() {
        let g = grid();
        let (t, n) = g.raycast(Vec2::new(8.0, 40.0), Vec2::NEG_Y, 100.0).unwrap();
        assert!((t - 24.0).abs() < 1e-4, "{t}");
        assert_eq!(n, Vec2::Y);
    }

    #[test]
    fn mao_unica_so_bloqueia_de_cima() {
        let g = grid();
        // De cima para baixo, atinge a plataforma na linha 3 (topo em y=64).
        let (t, _) = g
            .raycast(Vec2::new(88.0, 75.0), Vec2::NEG_Y, 100.0)
            .unwrap();
        assert!((t - 11.0).abs() < 1e-4, "{t}");
        // De baixo para cima, atravessa.
        assert!(g.raycast(Vec2::new(88.0, 20.0), Vec2::Y, 100.0).is_none());
    }
}
