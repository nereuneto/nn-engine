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

mod grid;
mod shape;

use std::collections::{HashMap, HashSet};

use glam::Vec2;

pub use grid::{Cell, CollisionGrid};
pub use shape::{Aabb, Shape};

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
        }
    }

    // ------------------------------------------------------------------
    // Corpos e grades
    // ------------------------------------------------------------------

    pub fn add(&mut self, body: Body) -> BodyId {
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

    fn ids(&self) -> Vec<BodyId> {
        self.bodies().map(|(id, _)| id).collect()
    }

    // ------------------------------------------------------------------
    // Simulação
    // ------------------------------------------------------------------

    /// Avança a simulação `dt` segundos. Chame em passos fixos
    /// (`fixed_update`). Devolve os eventos de sensores deste passo.
    pub fn step(&mut self, dt: f32) -> &[Event] {
        self.events.clear();
        if dt <= 0.0 {
            return &self.events;
        }
        let ids = self.ids();

        // 1. Cinemáticos andam e empurram quem estiver no caminho.
        let mut moved: HashMap<BodyId, Vec2> = HashMap::new();
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
                    self.move_body(id, &mut body, dt);
                }
            }
            self.put(id, body);
        }

        // 3. Dinâmicos entre si.
        self.separate_dynamics(&ids);

        // 4. Sensores.
        self.update_overlaps(&ids);
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
    fn push_out_of_kinematics(&self, body: &mut Body, moved: &HashMap<BodyId, Vec2>) {
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
    fn move_body(&self, id: BodyId, body: &mut Body, dt: f32) {
        let delta = body.velocity * dt;
        // Subpassos menores que o próprio corpo: assim ele não atravessa
        // paredes finas em alta velocidade.
        let half = body.shape.half_extents();
        let limit = (half.x.min(half.y) * 0.9).max(1.0);
        let steps = ((delta.abs().max_element() / limit).ceil() as u32).clamp(1, MAX_SUBSTEPS);
        let part = delta / steps as f32;
        for _ in 0..steps {
            match body.shape {
                Shape::Rect { .. } => {
                    self.move_axis(id, body, Vec2::new(part.x, 0.0));
                    self.move_axis(id, body, Vec2::new(0.0, part.y));
                    self.resolve_round_solids(id, body);
                }
                Shape::Circle { .. } => {
                    let before = body.position;
                    body.position += part;
                    self.resolve_mtv(id, body, before);
                }
            }
        }
    }

    /// Movimento num eixo só (retângulos): empurra de volta ao bater.
    fn move_axis(&self, id: BodyId, body: &mut Body, delta: Vec2) {
        if delta == Vec2::ZERO {
            return;
        }
        let half = body.shape.half_extents();
        let prev_bottom = body.position.y - half.y;
        body.position += delta;

        let mut hits: Vec<(Aabb, bool, Option<Ground>)> = Vec::new();
        let aabb = body.aabb();
        for grid in self.grids() {
            for (c, r, cell) in grid.cells_in(&aabb) {
                hits.push((
                    grid.cell_aabb(c, r),
                    cell == Cell::OneWay,
                    Some(Ground::Grid),
                ));
            }
        }
        for (oid, other) in self.bodies() {
            if oid == id || !other.is_solid() || !other.interacts(body) {
                continue;
            }
            if let Shape::Rect { .. } = other.shape {
                hits.push((other.aabb(), other.one_way, Some(Ground::Body(oid))));
            }
        }

        for (solid, one_way, ground) in hits {
            let aabb = body.aabb();
            if !aabb.overlaps(&solid) {
                continue;
            }
            if delta.x != 0.0 {
                if one_way {
                    continue;
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
                    continue;
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
        }
    }

    /// Retângulo dinâmico contra sólidos redondos.
    fn resolve_round_solids(&self, id: BodyId, body: &mut Body) {
        for (oid, other) in self.bodies() {
            if oid == id || !other.is_solid() || !other.interacts(body) {
                continue;
            }
            if let Shape::Circle { .. } = other.shape
                && let Some((n, depth)) =
                    separation(body.position, &body.shape, other.position, &other.shape)
            {
                body.position += n * depth;
                record_touch(body, n, Some(Ground::Body(oid)));
                cancel_velocity(body, n);
            }
        }
    }

    /// Círculo dinâmico: resolve pelo vetor mínimo de separação.
    fn resolve_mtv(&self, id: BodyId, body: &mut Body, before: Vec2) {
        let radius = body.shape.half_extents().y;
        let prev_bottom = before.y - radius;
        for _ in 0..4 {
            let mut any = false;
            let aabb = body.aabb();
            let mut solids: Vec<(Vec2, Shape, bool, Option<Ground>)> = Vec::new();
            for grid in self.grids() {
                for (c, r, cell) in grid.cells_in(&aabb) {
                    let cell_box = grid.cell_aabb(c, r);
                    solids.push((
                        cell_box.center(),
                        Shape::Rect {
                            half: cell_box.half(),
                        },
                        cell == Cell::OneWay,
                        Some(Ground::Grid),
                    ));
                }
            }
            for (oid, other) in self.bodies() {
                if oid != id && other.is_solid() && other.interacts(body) {
                    solids.push((
                        other.position,
                        other.shape,
                        other.one_way,
                        Some(Ground::Body(oid)),
                    ));
                }
            }
            for (pos, shape, one_way, ground) in solids {
                let Some((n, depth)) = separation(body.position, &body.shape, pos, &shape) else {
                    continue;
                };
                if one_way {
                    let top = shape.aabb(pos).max.y;
                    if n.y < 0.7 || body.velocity.y > 0.0 || prev_bottom < top - EPS * 4.0 {
                        continue;
                    }
                }
                body.position += n * depth;
                record_touch(body, n, ground);
                cancel_velocity(body, n);
                any = true;
            }
            if !any {
                break;
            }
        }
    }

    fn separate_dynamics(&mut self, ids: &[BodyId]) {
        let dynamics: Vec<BodyId> = ids
            .iter()
            .copied()
            .filter(|id| {
                self.get(*id)
                    .is_some_and(|b| b.kind == BodyKind::Dynamic && !b.sensor)
            })
            .collect();
        for (i, &a) in dynamics.iter().enumerate() {
            for &b in &dynamics[i + 1..] {
                let (Some(ba), Some(bb)) = (self.get(a), self.get(b)) else {
                    continue;
                };
                if !ba.interacts(bb) {
                    continue;
                }
                let Some((n, depth)) = separation(ba.position, &ba.shape, bb.position, &bb.shape)
                else {
                    continue;
                };
                let push = n * depth / 2.0;
                if let Some(ba) = self.get_mut(a) {
                    ba.position += push;
                    record_touch(ba, n, Some(Ground::Body(b)));
                    cancel_velocity(ba, n);
                }
                if let Some(bb) = self.get_mut(b) {
                    bb.position -= push;
                    record_touch(bb, -n, Some(Ground::Body(a)));
                    cancel_velocity(bb, -n);
                }
            }
        }
    }

    fn update_overlaps(&mut self, ids: &[BodyId]) {
        let sensors: Vec<BodyId> = ids
            .iter()
            .copied()
            .filter(|id| self.get(*id).is_some_and(|b| b.sensor))
            .collect();
        let mut current = HashSet::new();
        for &s in &sensors {
            let Some(sb) = self.get(s) else { continue };
            for (oid, other) in self.bodies() {
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
        self.bodies()
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
        self.bodies()
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
        self.bodies()
            .filter(|(oid, other)| {
                *oid != id
                    && body.interacts(other)
                    && overlap(body.position, &body.shape, other.position, &other.shape)
            })
            .map(|(oid, _)| oid)
            .collect()
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
        for (id, b) in self.bodies() {
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
