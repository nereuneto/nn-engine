//! Física arcade 2D da nn-engine.
//!
//! Feita para jogos de plataforma, top-down e afins: corpos não giram,
//! colisões são resolvidas de forma previsível e estável, e o chão é
//! detectado direto ([`Body::on_ground`]). Não é uma simulação de corpos
//! rígidos realista (sem rotação, atrito ou empilhamento físico complexo).
//!
//! - [`Body`]: retângulo ou círculo, estático, dinâmico ou cinemático.
//! - [`CollisionGrid`]: tiles sólidos e plataformas de mão única.
//! - Sensores: não colidem, mas geram [`Event::Enter`] / [`Event::Exit`].
//! - Consultas: [`World::raycast`], [`World::query_point`], [`World::query_aabb`].
//! - Caminhos: [`World::nav_grid`] monta a grade por onde dá para andar
//!   (para inimigos que contornam paredes num jogo visto de cima).

mod broad;
mod grid;
mod shape;

use std::collections::{BTreeMap, HashSet};

use glam::Vec2;

pub use grid::{Cell, CollisionGrid};
pub use nn_nav::{NavGrid, Path as NavPath};
pub use shape::{Aabb, Shape};

use broad::Broad;
use shape::{EPS, overlap, raycast_circle, separation};

/// Identificador de um corpo no [`World`]. Continua válido (e único) mesmo
/// depois que outros corpos são removidos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BodyId {
    index: u32,
    generation: u32,
}

impl BodyId {
    /// Número inteiro único do corpo, útil como chave em tabelas.
    pub fn to_bits(self) -> u64 {
        (u64::from(self.generation) << 32) | u64::from(self.index)
    }

    pub fn from_bits(bits: u64) -> Self {
        Self {
            index: bits as u32,
            generation: (bits >> 32) as u32,
        }
    }
}

/// Identificador de uma grade de colisão no [`World`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridId(u32);

/// Como o corpo se move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyKind {
    /// Não se move. Paredes, chão.
    Static,
    /// Sofre gravidade e colide. Jogador, inimigos, caixas.
    Dynamic,
    /// Move-se só pela velocidade que você define, ignorando colisões, e
    /// empurra/carrega corpos dinâmicos. Plataformas móveis, elevadores.
    Kinematic,
}

/// Lados em que o corpo encostou em algo sólido no último passo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Touching {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    /// Apoiado no chão.
    pub bottom: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ground {
    Grid,
    Body(BodyId),
}

/// Um corpo físico.
#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    pub kind: BodyKind,
    pub shape: Shape,
    /// Centro do corpo no mundo.
    pub position: Vec2,
    pub velocity: Vec2,
    /// Multiplicador da gravidade do mundo (0 = flutua).
    pub gravity_scale: f32,
    /// Sensores não colidem; só avisam quando algo entra ou sai deles.
    pub sensor: bool,
    /// Para corpos estáticos e cinemáticos retangulares: só bloqueia quem vem
    /// de cima (plataforma de mão única).
    pub one_way: bool,
    /// Quanto da velocidade volta ao bater (0 = para, 1 = quica sem perder).
    pub bounce: f32,
    /// Camadas a que o corpo pertence (bits).
    pub layer: u32,
    /// Camadas com que o corpo interage (bits).
    pub mask: u32,
    /// Rótulo livre ("jogador", "moeda"...).
    pub tag: String,
    touching: Touching,
    ground: Option<Ground>,
}

impl Body {
    fn new(kind: BodyKind, shape: Shape) -> Self {
        Self {
            kind,
            shape,
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            gravity_scale: 1.0,
            sensor: false,
            one_way: false,
            bounce: 0.0,
            layer: 1,
            mask: u32::MAX,
            tag: String::new(),
            touching: Touching::default(),
            ground: None,
        }
    }

    /// Corpo dinâmico: cai, anda e colide.
    pub fn dynamic(shape: Shape) -> Self {
        Self::new(BodyKind::Dynamic, shape)
    }

    /// Corpo estático: parede, chão.
    pub fn fixed(shape: Shape) -> Self {
        Self::new(BodyKind::Static, shape)
    }

    /// Corpo cinemático: move-se pela velocidade e carrega quem está em cima.
    pub fn kinematic(shape: Shape) -> Self {
        Self::new(BodyKind::Kinematic, shape)
    }

    pub fn at(mut self, position: Vec2) -> Self {
        self.position = position;
        self
    }

    pub fn with_velocity(mut self, velocity: Vec2) -> Self {
        self.velocity = velocity;
        self
    }

    pub fn with_gravity_scale(mut self, scale: f32) -> Self {
        self.gravity_scale = scale;
        self
    }

    pub fn as_sensor(mut self) -> Self {
        self.sensor = true;
        self
    }

    pub fn as_one_way(mut self) -> Self {
        self.one_way = true;
        self
    }

    pub fn with_bounce(mut self, bounce: f32) -> Self {
        self.bounce = bounce;
        self
    }

    pub fn with_layer(mut self, layer: u32, mask: u32) -> Self {
        self.layer = layer;
        self.mask = mask;
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        self
    }

    /// Lados que encostaram em algo sólido no último passo.
    pub fn touching(&self) -> Touching {
        self.touching
    }

    /// Está apoiado em algo (chão, plataforma, outro corpo)?
    pub fn on_ground(&self) -> bool {
        self.touching.bottom
    }

    /// Corpo em que está apoiado, se for um corpo (e não a grade).
    pub fn ground_body(&self) -> Option<BodyId> {
        match self.ground {
            Some(Ground::Body(id)) => Some(id),
            _ => None,
        }
    }

    pub fn aabb(&self) -> Aabb {
        self.shape.aabb(self.position)
    }

    fn interacts(&self, other: &Body) -> bool {
        (self.layer & other.mask) != 0 && (other.layer & self.mask) != 0
    }

    fn is_solid(&self) -> bool {
        !self.sensor && self.kind != BodyKind::Dynamic
    }
}

/// Algo que aconteceu durante um [`World::step`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Dois corpos começaram a se sobrepor; pelo menos um é sensor
    /// (o primeiro, se só um for).
    Enter(BodyId, BodyId),
    /// Deixaram de se sobrepor.
    Exit(BodyId, BodyId),
}

/// Resultado de um raio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayHit {
    /// Corpo atingido, ou `None` se foi uma grade.
    pub body: Option<BodyId>,
    pub point: Vec2,
    /// Direção da superfície atingida.
    pub normal: Vec2,
    pub distance: f32,
}

#[derive(Debug, Clone)]
struct Slot {
    generation: u32,
    body: Option<Body>,
}

/// Mundo físico: corpos, grades e gravidade.
#[derive(Debug, Clone)]
pub struct World {
    /// Gravidade em unidades por segundo². Y para cima, então cair é negativo.
    pub gravity: Vec2,
    slots: Vec<Slot>,
    free: Vec<u32>,
    grids: Vec<Option<CollisionGrid>>,
    overlaps: HashSet<(BodyId, BodyId)>,
    events: Vec<Event>,
    /// A grade espacial (refeita em cada fase do passo) e os vetores de
    /// trabalho, guardados para não criar outros a cada passo.
    broad: Broad,
    scratch: Scratch,
    stats: std::cell::Cell<StepStats>,
    /// Nos testes: uma célula enorme põe todos juntos (o mesmo que conferir
    /// todos contra todos, como antes da grade).
    cell_override: Option<f32>,
    /// A grade das consultas (raios, pontos, caixas), refeita na primeira
    /// consulta depois que algum corpo pode ter mudado.
    query: std::cell::RefCell<QueryIndex>,
}

#[derive(Debug, Clone, Default)]
struct QueryIndex {
    fresh: bool,
    broad: Broad,
    near: Vec<u32>,
}

/// Com menos corpos que isto, as consultas olham todos (é mais rápido que
/// montar a grade).
const QUERY_GRID_MIN: usize = 48;

/// Quanto trabalho o último [`World::step`] deu (para medir e para testes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StepStats {
    /// Corpos no mundo.
    pub bodies: u32,
    /// Vezes que um corpo foi conferido contra outro (o resto a grade
    /// espacial descartou).
    pub checks: u64,
}

/// Os sólidos perto de um corpo dinâmico enquanto ele anda (veja
/// [`World::near_solids`]).
struct NearSolids<'a> {
    id: BodyId,
    broad: &'a mut Broad,
    near: &'a mut Vec<u32>,
    solids: &'a mut Vec<BodyId>,
    region: Option<Aabb>,
}

#[derive(Debug, Clone, Default)]
struct Scratch {
    ids: Vec<BodyId>,
    near: Vec<u32>,
    solids: Vec<BodyId>,
    sizes: Vec<f32>,
}

impl Default for World {
    fn default() -> Self {
        Self::new(Vec2::new(0.0, -1500.0))
    }
}

/// Limite de subpassos por corpo em cada passo.
const MAX_SUBSTEPS: u32 = 256;

/// Velocidade abaixo da qual um quique é descartado (evita tremer no chão).
const MIN_BOUNCE_SPEED: f32 = 30.0;

impl World {
    pub fn new(gravity: Vec2) -> Self {
        Self {
            gravity,
            slots: Vec::new(),
            free: Vec::new(),
            grids: Vec::new(),
            overlaps: HashSet::new(),
            events: Vec::new(),
            broad: Broad::default(),
            scratch: Scratch::default(),
            stats: std::cell::Cell::new(StepStats::default()),
            cell_override: None,
            query: std::cell::RefCell::new(QueryIndex::default()),
        }
    }

    // ------------------------------------------------------------------
    // Corpos e grades
    // ------------------------------------------------------------------

    pub fn add(&mut self, body: Body) -> BodyId {
        self.query.get_mut().fresh = false;
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.body = Some(body);
            BodyId {
                index,
                generation: slot.generation,
            }
        } else {
            self.slots.push(Slot {
                generation: 0,
                body: Some(body),
            });
            BodyId {
                index: self.slots.len() as u32 - 1,
                generation: 0,
            }
        }
    }

    /// Remove o corpo. Sobreposições com ele terminam sem gerar `Exit`.
    pub fn remove(&mut self, id: BodyId) -> Option<Body> {
        self.query.get_mut().fresh = false;
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        let body = slot.body.take()?;
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(id.index);
        self.overlaps.retain(|(a, b)| *a != id && *b != id);
        Some(body)
    }

    pub fn get(&self, id: BodyId) -> Option<&Body> {
        let slot = self.slots.get(id.index as usize)?;
        (slot.generation == id.generation)
            .then_some(slot.body.as_ref())
            .flatten()
    }

    pub fn get_mut(&mut self, id: BodyId) -> Option<&mut Body> {
        self.query.get_mut().fresh = false;
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        slot.body.as_mut()
    }

    pub fn contains(&self, id: BodyId) -> bool {
        self.get(id).is_some()
    }

    /// Todos os corpos.
    pub fn bodies(&self) -> impl Iterator<Item = (BodyId, &Body)> {
        self.slots.iter().enumerate().filter_map(|(i, s)| {
            s.body.as_ref().map(|b| {
                (
                    BodyId {
                        index: i as u32,
                        generation: s.generation,
                    },
                    b,
                )
            })
        })
    }

    pub fn len(&self) -> usize {
        self.slots.iter().filter(|s| s.body.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn add_grid(&mut self, grid: CollisionGrid) -> GridId {
        self.grids.push(Some(grid));
        GridId(self.grids.len() as u32 - 1)
    }

    pub fn grid(&self, id: GridId) -> Option<&CollisionGrid> {
        self.grids.get(id.0 as usize)?.as_ref()
    }

    pub fn grid_mut(&mut self, id: GridId) -> Option<&mut CollisionGrid> {
        self.grids.get_mut(id.0 as usize)?.as_mut()
    }

    pub fn remove_grid(&mut self, id: GridId) -> Option<CollisionGrid> {
        self.grids.get_mut(id.0 as usize)?.take()
    }

    pub fn grids(&self) -> impl Iterator<Item = &CollisionGrid> {
        self.grids.iter().flatten()
    }

    /// Os corpos que podem tocar a caixa, em ordem; `None` = olhe todos
    /// (poucos corpos).
    fn query_near(&self, area: &Aabb) -> Option<Vec<BodyId>> {
        let count = self.slots.len();
        if count < QUERY_GRID_MIN {
            return None;
        }
        let mut q = self.query.borrow_mut();
        let q = &mut *q;
        if !q.fresh {
            let mut sizes: Vec<f32> = self
                .bodies()
                .map(|(_, b)| b.shape.half_extents().max_element() * 2.0)
                .collect();
            let cell = self
                .cell_override
                .unwrap_or_else(|| broad::cell_size(&mut sizes));
            q.broad.build(
                cell,
                self.bodies().map(|(id, b)| {
                    let a = b.aabb();
                    (id.index, Aabb::new(a.min - EPS, a.max + EPS))
                }),
            );
            q.fresh = true;
        }
        q.broad.query(area, &mut q.near);
        Some(q.near.iter().map(|&i| self.id_at(i)).collect())
    }

    /// Os corpos (com o id) que podem tocar a caixa, na ordem de sempre.
    fn bodies_near(&self, area: Aabb) -> impl Iterator<Item = (BodyId, &Body)> {
        let near = self.query_near(&area);
        let all = near.is_none().then(|| self.bodies());
        let some = near.map(|v| v.into_iter().filter_map(|id| self.get(id).map(|b| (id, b))));
        all.into_iter().flatten().chain(some.into_iter().flatten())
    }

    /// O trabalho do último passo.
    pub fn stats(&self) -> StepStats {
        self.stats.get()
    }

    fn count(&self, n: u64) {
        let mut s = self.stats.get();
        s.checks += n;
        self.stats.set(s);
    }

    fn id_at(&self, index: u32) -> BodyId {
        BodyId {
            index,
            generation: self.slots[index as usize].generation,
        }
    }

    /// O lado das células da grade espacial para os corpos de agora.
    fn cell_size(&mut self) -> f32 {
        let mut sizes = std::mem::take(&mut self.scratch.sizes);
        sizes.clear();
        sizes.extend(self.bodies().map(|(_, b)| {
            let h = b.shape.half_extents();
            h.x.max(h.y) * 2.0
        }));
        let c = broad::cell_size(&mut sizes);
        self.scratch.sizes = sizes;
        c
    }

    /// Refaz a grade espacial com os corpos que passam no filtro, cada caixa
    /// crescida por `grow(corpo)`.
    fn build_broad(
        &mut self,
        cell: f32,
        keep: impl Fn(&Body) -> bool,
        grow: impl Fn(&Body) -> f32,
    ) {
        let mut broad = std::mem::take(&mut self.broad);
        broad.build(
            cell,
            self.bodies().filter(|(_, b)| keep(b)).map(|(id, b)| {
                let a = b.aabb();
                let g = grow(b);
                (id.index, Aabb::new(a.min - g, a.max + g))
            }),
        );
        self.broad = broad;
    }

    // ------------------------------------------------------------------
    // Simulação
    // ------------------------------------------------------------------

    /// Avança a simulação `dt` segundos. Chame em passos fixos
    /// (`fixed_update`). Devolve os eventos de sensores deste passo.
    pub fn step(&mut self, dt: f32) -> &[Event] {
        self.events.clear();
        self.stats.set(StepStats::default());
        self.query.get_mut().fresh = false;
        if dt <= 0.0 {
            return &self.events;
        }
        let mut ids = std::mem::take(&mut self.scratch.ids);
        ids.clear();
        ids.extend(self.bodies().map(|(id, _)| id));
        let cell = self.cell_override.unwrap_or_else(|| self.cell_size());

        // 1. Cinemáticos andam e empurram quem estiver no caminho.
        let mut moved: BTreeMap<BodyId, Vec2> = BTreeMap::new();
        for &id in &ids {
            let Some(body) = self.get_mut(id) else {
                continue;
            };
            if body.kind != BodyKind::Kinematic {
                continue;
            }
            let d = body.velocity * dt;
            body.position += d;
            moved.insert(id, d);
        }

        // 2. Dinâmicos: carona em plataformas, gravidade, movimento e colisão.
        // Os sólidos (parados e cinemáticos) não se mexem nesta fase: a grade
        // deles vale para todos os dinâmicos.
        self.build_broad(cell, Body::is_solid, |_| EPS * 4.0);
        let mut near = std::mem::take(&mut self.scratch.near);
        let mut solids = std::mem::take(&mut self.scratch.solids);
        let mut broad = std::mem::take(&mut self.broad);
        for &id in &ids {
            let Some(mut body) = self.take(id) else {
                continue;
            };
            if body.kind == BodyKind::Dynamic {
                if let Some(Ground::Body(g)) = body.ground
                    && let Some(d) = moved.get(&g)
                {
                    body.position += *d;
                }
                body.velocity += self.gravity * body.gravity_scale * dt;
                body.touching = Touching::default();
                body.ground = None;
                if body.sensor {
                    body.position += body.velocity * dt;
                } else {
                    self.push_out_of_kinematics(&mut body, &moved);
                    let mut near_solids = NearSolids {
                        id,
                        broad: &mut broad,
                        near: &mut near,
                        solids: &mut solids,
                        region: None,
                    };
                    self.move_body(&mut body, dt, &mut near_solids);
                }
            }
            self.put(id, body);
        }
        self.broad = broad;

        // 3. Dinâmicos entre si.
        self.separate_dynamics(cell, &mut near);

        // 4. Sensores.
        self.update_overlaps(cell, &ids, &mut near);
        self.stats.set(StepStats {
            bodies: ids.len() as u32,
            ..self.stats.get()
        });
        self.scratch.ids = ids;
        self.scratch.near = near;
        self.scratch.solids = solids;
        &self.events
    }

    fn take(&mut self, id: BodyId) -> Option<Body> {
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        slot.body.take()
    }

    fn put(&mut self, id: BodyId, body: Body) {
        self.slots[id.index as usize].body = Some(body);
    }

    /// Um cinemático que andou para cima de um corpo o empurra para fora.
    fn push_out_of_kinematics(&self, body: &mut Body, moved: &BTreeMap<BodyId, Vec2>) {
        for (&kid, &d) in moved {
            if d == Vec2::ZERO {
                continue;
            }
            let Some(k) = self.get(kid) else { continue };
            if k.sensor || k.one_way || !k.interacts(body) {
                continue;
            }
            if let Some((n, depth)) = separation(body.position, &body.shape, k.position, &k.shape) {
                body.position += n * depth;
                record_touch(body, n, Some(Ground::Body(kid)));
            }
        }
    }

    /// Move o corpo dinâmico resolvendo colisões com sólidos e grades.
    fn move_body(&self, body: &mut Body, dt: f32, near: &mut NearSolids) {
        let delta = body.velocity * dt;
        // Subpassos menores que o próprio corpo: assim ele não atravessa
        // paredes finas em alta velocidade.
        let half = body.shape.half_extents();
        let limit = (half.x.min(half.y) * 0.9).max(1.0);
        let steps = ((delta.abs().max_element() / limit).ceil() as u32).clamp(1, MAX_SUBSTEPS);
        let part = delta / steps as f32;
        // `left`: o caminho que falta (os sólidos perto dele valem até lá).
        let mut left = delta;
        for _ in 0..steps {
            match body.shape {
                Shape::Rect { .. } => {
                    self.near_solids(near, body, left);
                    self.move_axis(body, Vec2::new(part.x, 0.0), near);
                    self.near_solids(near, body, left);
                    self.move_axis(body, Vec2::new(0.0, part.y), near);
                    self.near_solids(near, body, left);
                    self.resolve_round_solids(body, near);
                }
                Shape::Circle { .. } => {
                    self.near_solids(near, body, left);
                    let before = body.position;
                    body.position += part;
                    self.resolve_mtv(body, before, near);
                }
            }
            left -= part;
        }
    }

    /// Garante que `near.solids` tem todos os sólidos que o corpo pode tocar
    /// andando `left` a mais (com uma folga do tamanho dele), na ordem dos
    /// corpos.
    fn near_solids(&self, near: &mut NearSolids, body: &Body, left: Vec2) {
        let a = body.aabb();
        let reach = left.abs() + body.shape.half_extents() * 0.5 + Vec2::splat(EPS * 8.0);
        if near
            .region
            .is_some_and(|r| inside(&Aabb::new(a.min - reach, a.max + reach), &r))
        {
            return;
        }
        let margin = left.abs() + body.shape.half_extents() + Vec2::splat(1.0);
        let region = Aabb::new(a.min - margin, a.max + margin);
        near.broad.query(&region, near.near);
        near.solids.clear();
        for &i in near.near.iter() {
            let oid = self.id_at(i);
            if oid != near.id
                && self
                    .get(oid)
                    .is_some_and(|o| o.is_solid() && o.interacts(body))
            {
                near.solids.push(oid);
            }
        }
        near.region = Some(region);
    }

    /// Passa por cada sólido que o corpo pode tocar, na ordem dos corpos:
    /// os da área de `near` e, se um empurrão tirar o corpo dela no meio, os
    /// outros que vêm depois (como se não houvesse a grade).
    fn each_solid(
        &self,
        body: &mut Body,
        near: &NearSolids,
        mut f: impl FnMut(&mut Body, BodyId, &Body),
    ) {
        let mut f = |body: &mut Body, oid: BodyId, other: &Body| {
            self.count(1);
            f(body, oid, other);
        };
        let region = near.region.unwrap_or_else(|| body.aabb());
        let mut after = None;
        for &oid in near.solids.iter() {
            let Some(other) = self.get(oid) else { continue };
            f(body, oid, other);
            if !inside(&body.aabb(), &region) {
                after = Some(oid.index);
                break;
            }
        }
        let Some(after) = after else { return };
        for (oid, other) in self.bodies() {
            if oid.index > after && oid != near.id && other.is_solid() && other.interacts(body) {
                f(body, oid, other);
            }
        }
    }

    /// Movimento num eixo só (retângulos): empurra de volta ao bater.
    fn move_axis(&self, body: &mut Body, delta: Vec2, near: &NearSolids) {
        if delta == Vec2::ZERO {
            return;
        }
        let half = body.shape.half_extents();
        let prev_bottom = body.position.y - half.y;
        body.position += delta;

        let hit = |body: &mut Body, solid: Aabb, one_way: bool, ground: Option<Ground>| {
            if !body.aabb().overlaps(&solid) {
                return;
            }
            if delta.x != 0.0 {
                if one_way {
                    return;
                }
                if delta.x > 0.0 {
                    body.position.x = solid.min.x - half.x;
                    body.touching.right = true;
                } else {
                    body.position.x = solid.max.x + half.x;
                    body.touching.left = true;
                }
                body.velocity.x = bounce(body.velocity.x, body.bounce);
            } else {
                if one_way && (delta.y > 0.0 || prev_bottom < solid.max.y - EPS * 4.0) {
                    return;
                }
                if delta.y > 0.0 {
                    body.position.y = solid.min.y - half.y;
                    body.touching.top = true;
                } else {
                    body.position.y = solid.max.y + half.y;
                    body.touching.bottom = true;
                    body.ground = ground;
                }
                body.velocity.y = bounce(body.velocity.y, body.bounce);
            }
        };
        // Primeiro os tiles que ele toca agora, depois os corpos retangulares.
        let aabb = body.aabb();
        for grid in self.grids() {
            for (c, r, cell) in grid.cells_in(&aabb) {
                hit(
                    body,
                    grid.cell_aabb(c, r),
                    cell == Cell::OneWay,
                    Some(Ground::Grid),
                );
            }
        }
        self.each_solid(body, near, |body, oid, other| {
            if let Shape::Rect { .. } = other.shape {
                hit(body, other.aabb(), other.one_way, Some(Ground::Body(oid)));
            }
        });
    }

    /// Retângulo dinâmico contra sólidos redondos.
    fn resolve_round_solids(&self, body: &mut Body, near: &NearSolids) {
        self.each_solid(body, near, |body, oid, other| {
            if let Shape::Circle { .. } = other.shape
                && let Some((n, depth)) =
                    separation(body.position, &body.shape, other.position, &other.shape)
            {
                body.position += n * depth;
                record_touch(body, n, Some(Ground::Body(oid)));
                cancel_velocity(body, n);
            }
        });
    }

    /// Círculo dinâmico: resolve pelo vetor mínimo de separação.
    fn resolve_mtv(&self, body: &mut Body, before: Vec2, near: &NearSolids) {
        let radius = body.shape.half_extents().y;
        let prev_bottom = before.y - radius;
        let push =
            |body: &mut Body, pos: Vec2, shape: &Shape, one_way: bool, ground: Option<Ground>| {
                let Some((n, depth)) = separation(body.position, &body.shape, pos, shape) else {
                    return false;
                };
                if one_way {
                    let top = shape.aabb(pos).max.y;
                    if n.y < 0.7 || body.velocity.y > 0.0 || prev_bottom < top - EPS * 4.0 {
                        return false;
                    }
                }
                body.position += n * depth;
                record_touch(body, n, ground);
                cancel_velocity(body, n);
                true
            };
        for _ in 0..4 {
            let mut any = false;
            let aabb = body.aabb();
            for grid in self.grids() {
                for (c, r, cell) in grid.cells_in(&aabb) {
                    let b = grid.cell_aabb(c, r);
                    let shape = Shape::Rect { half: b.half() };
                    any |= push(
                        body,
                        b.center(),
                        &shape,
                        cell == Cell::OneWay,
                        Some(Ground::Grid),
                    );
                }
            }
            self.each_solid(body, near, |body, oid, other| {
                any |= push(
                    body,
                    other.position,
                    &other.shape,
                    other.one_way,
                    Some(Ground::Body(oid)),
                );
            });
            if !any {
                break;
            }
        }
    }

    fn separate_dynamics(&mut self, cell: f32, near: &mut Vec<u32>) {
        // A grade dos dinâmicos, com as caixas crescidas: os empurrões deste
        // laço mexem neles. Quem sai da caixa guardada entra na lista
        // `escaped` e passa a ser conferido por todos (assim nenhum par fica
        // de fora, como sem a grade).
        let is_dyn = |b: &Body| b.kind == BodyKind::Dynamic && !b.sensor;
        let grow = |b: &Body| b.shape.half_extents().max_element() * 0.5 + 1.0;
        self.build_broad(cell, is_dyn, grow);
        let n = self.slots.len();
        let mut stored: Vec<Option<Aabb>> = vec![None; n];
        for (id, b) in self.bodies() {
            if is_dyn(b) {
                let (a, g) = (b.aabb(), grow(b));
                stored[id.index as usize] = Some(Aabb::new(a.min - g, a.max + g));
            }
        }
        let mut escaped: Vec<u32> = Vec::new();
        let mut is_escaped = vec![false; n];
        let mut check = |world: &World, i: u32, escaped: &mut Vec<u32>| {
            let i = i as usize;
            if is_escaped[i] {
                return;
            }
            if let (Some(b), Some(box_)) = (world.slots[i].body.as_ref(), stored[i])
                && !inside(&b.aabb(), &box_)
            {
                is_escaped[i] = true;
                escaped.push(i as u32);
            }
        };
        for ai in 0..n as u32 {
            let a = self.id_at(ai);
            let Some(ba) = self.get(a).filter(|b| is_dyn(b)) else {
                continue;
            };
            let (aa, g) = (ba.aabb(), grow(ba));
            let region = Aabb::new(aa.min - g, aa.max + g);
            let mut broad = std::mem::take(&mut self.broad);
            broad.query(&region, near);
            self.broad = broad;
            if !escaped.is_empty() {
                near.extend(escaped.iter().copied());
                near.sort_unstable();
                near.dedup();
            }
            // Os pares de `a` com quem vem depois: os da grade e, se os
            // empurrões tirarem `a` da área, todos os que faltam.
            let mut k = 0;
            let mut brute_after: Option<u32> = None;
            loop {
                let bi = if let Some(after) = brute_after {
                    let next = after + 1;
                    if next as usize >= n {
                        break;
                    }
                    brute_after = Some(next);
                    next
                } else {
                    let Some(&bi) = near.get(k) else { break };
                    k += 1;
                    if bi <= ai {
                        continue;
                    }
                    bi
                };
                let b = self.id_at(bi);
                self.count(1);
                let (Some(ba), Some(bb)) = (self.get(a), self.get(b)) else {
                    continue;
                };
                if !is_dyn(bb) || !ba.interacts(bb) {
                    continue;
                }
                let Some((nrm, depth)) = separation(ba.position, &ba.shape, bb.position, &bb.shape)
                else {
                    continue;
                };
                let push = nrm * depth / 2.0;
                if let Some(ba) = self.get_mut(a) {
                    ba.position += push;
                    record_touch(ba, nrm, Some(Ground::Body(b)));
                    cancel_velocity(ba, nrm);
                }
                if let Some(bb) = self.get_mut(b) {
                    bb.position -= push;
                    record_touch(bb, -nrm, Some(Ground::Body(a)));
                    cancel_velocity(bb, -nrm);
                }
                check(self, ai, &mut escaped);
                check(self, bi, &mut escaped);
                if brute_after.is_none()
                    && self.get(a).is_some_and(|ba| !inside(&ba.aabb(), &region))
                {
                    brute_after = Some(bi);
                }
            }
        }
    }

    fn update_overlaps(&mut self, cell: f32, ids: &[BodyId], near: &mut Vec<u32>) {
        let mut current = HashSet::new();
        if ids.iter().any(|id| self.get(*id).is_some_and(|b| b.sensor)) {
            self.build_broad(cell, |_| true, |_| EPS);
            for &s in ids {
                let Some(area) = self.get(s).filter(|b| b.sensor).map(Body::aabb) else {
                    continue;
                };
                let mut broad = std::mem::take(&mut self.broad);
                broad.query(&area, near);
                self.broad = broad;
                for &oi in near.iter() {
                    let oid = self.id_at(oi);
                    self.count(1);
                    let (Some(sb), Some(other)) = (self.get(s), self.get(oid)) else {
                        continue;
                    };
                    if oid == s || !sb.interacts(other) {
                        continue;
                    }
                    if overlap(sb.position, &sb.shape, other.position, &other.shape) {
                        // Par ordenado: o sensor primeiro; se os dois forem, o menor id.
                        let pair = if other.sensor && oid < s {
                            (oid, s)
                        } else {
                            (s, oid)
                        };
                        current.insert(pair);
                    }
                }
            }
        }
        let mut entered: Vec<_> = current.difference(&self.overlaps).copied().collect();
        let mut exited: Vec<_> = self.overlaps.difference(&current).copied().collect();
        entered.sort();
        exited.sort();
        self.events
            .extend(entered.into_iter().map(|(a, b)| Event::Enter(a, b)));
        self.events
            .extend(exited.into_iter().map(|(a, b)| Event::Exit(a, b)));
        self.overlaps = current;
    }

    // ------------------------------------------------------------------
    // Consultas
    // ------------------------------------------------------------------

    /// Corpos que contêm o ponto.
    pub fn query_point(&self, p: Vec2) -> Vec<BodyId> {
        self.bodies_near(Aabb::new(p, p))
            .filter(|(_, b)| match b.shape {
                Shape::Rect { .. } => b.aabb().contains(p),
                Shape::Circle { radius } => b.position.distance_squared(p) <= radius * radius,
            })
            .map(|(id, _)| id)
            .collect()
    }

    /// Corpos que se sobrepõem à caixa.
    pub fn query_aabb(&self, aabb: Aabb) -> Vec<BodyId> {
        let shape = Shape::Rect { half: aabb.half() };
        let center = aabb.center();
        self.bodies_near(aabb)
            .filter(|(_, b)| overlap(center, &shape, b.position, &b.shape))
            .map(|(id, _)| id)
            .collect()
    }

    /// Os dois corpos se sobrepõem agora? Só a forma conta: camadas e
    /// máscaras são ignoradas. Útil para inimigos que atravessam o jogador
    /// (sem se empurrarem) mas precisam saber quando encostam nele.
    pub fn intersects(&self, a: BodyId, b: BodyId) -> bool {
        match (self.get(a), self.get(b)) {
            (Some(ba), Some(bb)) if a != b => {
                overlap(ba.position, &ba.shape, bb.position, &bb.shape)
            }
            _ => false,
        }
    }

    /// Corpos que se sobrepõem a `id` agora (respeitando camadas e máscaras).
    pub fn overlapping(&self, id: BodyId) -> Vec<BodyId> {
        let Some(body) = self.get(id) else {
            return Vec::new();
        };
        self.bodies_near(body.aabb())
            .filter(|(oid, other)| {
                *oid != id
                    && body.interacts(other)
                    && overlap(body.position, &body.shape, other.position, &other.shape)
            })
            .map(|(oid, _)| oid)
            .collect()
    }

    /// A grade de caminhos: as células onde cabe alguém de raio `radius`
    /// sem encostar em corpos parados (sólidos, não sensores) nem em tiles
    /// sólidos. Plataformas de mão única e corpos que se movem não bloqueiam.
    /// `area`: o pedaço do mundo coberto (sem ela, o que cobre os corpos
    /// parados e as grades, com uma folga).
    pub fn nav_grid(&self, area: Option<Aabb>, cell: f32, radius: f32) -> NavGrid {
        let cell = cell.max(1.0);
        let r = radius.max(0.0);
        let walls = || {
            self.bodies()
                .map(|(_, b)| b)
                .filter(|b| b.kind == BodyKind::Static && !b.sensor && !b.one_way)
        };
        let area = area.unwrap_or_else(|| {
            let mut lo = Vec2::splat(f32::MAX);
            let mut hi = Vec2::splat(f32::MIN);
            for b in walls() {
                let a = b.aabb();
                lo = lo.min(a.min);
                hi = hi.max(a.max);
            }
            for g in self.grids() {
                let size = g.cell_size() * Vec2::new(g.width() as f32, g.height() as f32);
                lo = lo.min(g.origin);
                hi = hi.max(g.origin + size);
            }
            if lo.x > hi.x {
                lo = Vec2::splat(-512.0);
                hi = Vec2::splat(512.0);
            }
            Aabb::new(lo - cell * 2.0, hi + cell * 2.0)
        });
        let mut nav = NavGrid::new(area.min, area.max, cell);
        for b in walls() {
            match b.shape {
                Shape::Circle { radius } => nav.block_circle(b.position, radius + r),
                Shape::Rect { .. } => {
                    let a = b.aabb();
                    nav.block_rect(a.min - r, a.max + r);
                }
            }
        }
        for g in self.grids() {
            for row in 0..g.height() as i32 {
                for col in 0..g.width() as i32 {
                    if g.get(col, row) == Cell::Solid {
                        let a = g.cell_aabb(col, row);
                        nav.block_rect(a.min - r, a.max + r);
                    }
                }
            }
        }
        nav
    }

    /// Lança um raio de `origin` na direção `dir` até `max_distance`.
    /// Atinge grades e corpos que não são sensores, cujas camadas cruzam `mask`.
    pub fn raycast(&self, origin: Vec2, dir: Vec2, max_distance: f32, mask: u32) -> Option<RayHit> {
        let dir = dir.normalize_or_zero();
        if dir == Vec2::ZERO || max_distance <= 0.0 {
            return None;
        }
        let mut best: Option<RayHit> = None;
        let mut consider = |t: f32, normal: Vec2, body: Option<BodyId>| {
            if best.is_none_or(|b| t < b.distance) {
                best = Some(RayHit {
                    body,
                    point: origin + dir * t,
                    normal,
                    distance: t,
                });
            }
        };
        for grid in self.grids() {
            if let Some((t, n)) = grid.raycast(origin, dir, max_distance) {
                consider(t, n, None);
            }
        }
        let end = origin + dir * max_distance;
        for (id, b) in self.bodies_near(Aabb::new(origin.min(end), origin.max(end))) {
            if b.sensor || (b.layer & mask) == 0 {
                continue;
            }
            let hit = match b.shape {
                Shape::Rect { .. } => b.aabb().raycast(origin, dir, max_distance),
                Shape::Circle { radius } => {
                    raycast_circle(b.position, radius, origin, dir, max_distance)
                }
            };
            if let Some((t, n)) = hit {
                if b.one_way && n != Vec2::Y {
                    continue;
                }
                consider(t, n, Some(id));
            }
        }
        best
    }
}

/// A caixa `a` está toda dentro de `b`?
fn inside(a: &Aabb, b: &Aabb) -> bool {
    a.min.x >= b.min.x && a.min.y >= b.min.y && a.max.x <= b.max.x && a.max.y <= b.max.y
}

fn bounce(v: f32, factor: f32) -> f32 {
    let r = -v * factor;
    if r.abs() < MIN_BOUNCE_SPEED { 0.0 } else { r }
}

/// Registra de que lado o corpo encostou, a partir da normal de separação.
fn record_touch(body: &mut Body, n: Vec2, ground: Option<Ground>) {
    if n.y > 0.7 {
        body.touching.bottom = true;
        body.ground = ground;
    } else if n.y < -0.7 {
        body.touching.top = true;
    } else if n.x > 0.7 {
        body.touching.left = true;
    } else if n.x < -0.7 {
        body.touching.right = true;
    }
}

/// Remove (ou inverte, com quique) a velocidade que empurra contra a superfície.
fn cancel_velocity(body: &mut Body, n: Vec2) {
    let vn = body.velocity.dot(n);
    if vn < 0.0 {
        let keep = if -vn * body.bounce < MIN_BOUNCE_SPEED {
            0.0
        } else {
            body.bounce
        };
        body.velocity -= n * vn * (1.0 + keep);
    }
}

#[cfg(test)]
mod tests;
