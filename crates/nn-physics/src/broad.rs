//! Grade espacial (a "fase larga"): diz, depressa, quais corpos podem estar
//! perto de uma caixa, para a física só conferir de verdade esses.
//!
//! Refeita a cada uso a partir das caixas dos corpos: não precisa saber
//! quando um corpo muda de lugar. As células guardam o número do corpo; uma
//! consulta devolve os números (sem repetir) em ordem crescente, a mesma
//! ordem em que a física percorre os corpos, para o resultado não mudar.

use std::collections::HashMap;

use crate::shape::Aabb;

/// Um corpo que cobre mais células que isto fica numa lista à parte, sempre
/// conferida (um chão de 10 mil pixels não entra em 300 células).
const MAX_CELLS: i64 = 64;

#[derive(Debug, Clone, Default)]
pub(crate) struct Broad {
    cell: f32,
    /// (célula, corpo), em ordem: as células de cada uma ficam juntas.
    entries: Vec<(u64, u32)>,
    /// Onde começam (e quantos são) os corpos de cada célula em `entries`.
    starts: HashMap<u64, (u32, u32)>,
    /// Corpos grandes demais para a grade.
    big: Vec<u32>,
    /// Marca de quem já entrou numa consulta (por corpo), para não repetir.
    seen: Vec<u32>,
    stamp: u32,
}

fn key(cx: i32, cy: i32) -> u64 {
    (u64::from(cx as u32) << 32) | u64::from(cy as u32)
}

impl Broad {
    /// Refaz a grade com estas caixas (número do corpo, caixa). `cell`: o
    /// lado da célula.
    pub(crate) fn build(&mut self, cell: f32, items: impl Iterator<Item = (u32, Aabb)>) {
        self.cell = cell.max(1.0);
        self.entries.clear();
        self.starts.clear();
        self.big.clear();
        let mut max_index = 0;
        for (i, a) in items {
            max_index = max_index.max(i as usize + 1);
            let (x0, y0, x1, y1) = self.range(&a);
            if (i64::from(x1) - i64::from(x0) + 1) * (i64::from(y1) - i64::from(y0) + 1) > MAX_CELLS
            {
                self.big.push(i);
                continue;
            }
            for cx in x0..=x1 {
                for cy in y0..=y1 {
                    self.entries.push((key(cx, cy), i));
                }
            }
        }
        self.entries.sort_unstable();
        let mut at = 0;
        while at < self.entries.len() {
            let k = self.entries[at].0;
            let mut end = at + 1;
            while end < self.entries.len() && self.entries[end].0 == k {
                end += 1;
            }
            self.starts.insert(k, (at as u32, (end - at) as u32));
            at = end;
        }
        if self.seen.len() < max_index {
            self.seen.resize(max_index, 0);
        }
    }

    fn range(&self, a: &Aabb) -> (i32, i32, i32, i32) {
        let c = self.cell;
        let f = |v: f32| (v / c).floor().clamp(-1.0e9, 1.0e9) as i32;
        (f(a.min.x), f(a.min.y), f(a.max.x), f(a.max.y))
    }

    /// Os corpos que podem tocar a caixa, sem repetir e em ordem crescente,
    /// em `out` (que é limpo antes).
    pub(crate) fn query(&mut self, a: &Aabb, out: &mut Vec<u32>) {
        out.clear();
        self.stamp = self.stamp.wrapping_add(1);
        if self.stamp == 0 {
            self.seen.fill(0);
            self.stamp = 1;
        }
        let stamp = self.stamp;
        let (x0, y0, x1, y1) = self.range(a);
        let cells = (i64::from(x1) - i64::from(x0) + 1) * (i64::from(y1) - i64::from(y0) + 1);
        let mut add = |i: u32, seen: &mut Vec<u32>| {
            let s = &mut seen[i as usize];
            if *s != stamp {
                *s = stamp;
                out.push(i);
            }
        };
        if cells > self.starts.len() as i64 * 4 + 16 {
            // Uma caixa enorme (um raio longo): mais barato olhar as células
            // que existem do que todas as da caixa.
            let (x0, y0, x1, y1) = (x0 as i64, y0 as i64, x1 as i64, y1 as i64);
            for (&k, &(at, n)) in &self.starts {
                let cx = i64::from((k >> 32) as u32 as i32);
                let cy = i64::from(k as u32 as i32);
                if cx >= x0 && cx <= x1 && cy >= y0 && cy <= y1 {
                    for e in &self.entries[at as usize..(at + n) as usize] {
                        add(e.1, &mut self.seen);
                    }
                }
            }
        } else {
            for cx in x0..=x1 {
                for cy in y0..=y1 {
                    if let Some(&(at, n)) = self.starts.get(&key(cx, cy)) {
                        for e in &self.entries[at as usize..(at + n) as usize] {
                            add(e.1, &mut self.seen);
                        }
                    }
                }
            }
        }
        for &i in &self.big {
            add(i, &mut self.seen);
        }
        out.sort_unstable();
    }
}

/// O lado da célula para estes corpos: umas duas vezes o tamanho típico
/// (a mediana), entre 8 e 1024 unidades.
pub(crate) fn cell_size(sizes: &mut [f32]) -> f32 {
    if sizes.is_empty() {
        return 64.0;
    }
    let mid = sizes.len() / 2;
    let (_, m, _) = sizes.select_nth_unstable_by(mid, f32::total_cmp);
    (*m * 2.0).clamp(8.0, 1024.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    fn b(x: f32, y: f32, s: f32) -> Aabb {
        Aabb::from_center(Vec2::new(x, y), Vec2::splat(s))
    }

    #[test]
    fn acha_so_quem_esta_perto_em_ordem() {
        let mut g = Broad::default();
        let boxes = [
            b(0.0, 0.0, 5.0),
            b(500.0, 0.0, 5.0),
            b(8.0, 0.0, 5.0),
            b(0.0, 0.0, 9000.0),
        ];
        g.build(32.0, boxes.iter().enumerate().map(|(i, a)| (i as u32, *a)));
        let mut out = Vec::new();
        g.query(&b(4.0, 0.0, 6.0), &mut out);
        assert_eq!(out, vec![0, 2, 3], "o grande (3) sempre vem");
        g.query(&b(500.0, 0.0, 1.0), &mut out);
        assert_eq!(out, vec![1, 3]);
        // Uma caixa enorme: o caminho pelas células que existem.
        g.query(&b(0.0, 0.0, 100_000.0), &mut out);
        assert_eq!(out, vec![0, 1, 2, 3]);
        // Negativos e o limite das células.
        g.build(10.0, [(7u32, b(-25.0, -25.0, 1.0))].into_iter());
        g.query(&b(-25.0, -25.0, 0.5), &mut out);
        assert_eq!(out, vec![7]);
        g.query(&b(25.0, 25.0, 0.5), &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn tamanho_da_celula() {
        assert_eq!(cell_size(&mut []), 64.0);
        assert_eq!(cell_size(&mut [10.0, 30.0, 20.0]), 40.0);
        assert_eq!(cell_size(&mut [1.0]), 8.0);
    }
}
