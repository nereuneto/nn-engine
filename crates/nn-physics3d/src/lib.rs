//! Física 3D da nn-engine, sobre o [rapier](https://rapier.rs).
//!
//! - [`World3`]: o mundo, com gravidade, que avança em passos fixos.
//! - Corpos ([`BodyDesc`]): estáticos (chão, paredes, cenário), dinâmicos
//!   (caem, rolam, empilham e são empurrados) e cinemáticos (movidos pelo
//!   jogo: plataformas, portas).
//! - Formas ([`Shape3`]): caixa, esfera, cápsula, cilindro e malha exata de
//!   triângulos (para cenário parado).
//! - Personagens ([`CharacterDesc`]): andam, pulam, sobem degraus e rampas,
//!   não atravessam paredes e empurram o que é dinâmico.
//! - Sensores, eventos de toque ([`Event3`]) e raios ([`World3::raycast`]).
//!
//! Y para cima; a unidade é livre (pense em metros).

use std::collections::HashMap;

use glam::{Quat, Vec3};
use rapier3d::control::{CharacterAutostep, CharacterLength, KinematicCharacterController};
use rapier3d::prelude::*;

type RVec = rapier3d::math::Vector;
type RRot = rapier3d::math::Rotation;
type RPose = rapier3d::math::Pose;

fn rv(v: Vec3) -> RVec {
    RVec::new(v.x, v.y, v.z)
}
fn gv(v: RVec) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}
fn rq(q: Quat) -> RRot {
    RRot::from_xyzw(q.x, q.y, q.z, q.w)
}
fn gq(q: RRot) -> Quat {
    Quat::from_xyzw(q.x, q.y, q.z, q.w)
}

/// Identificador de um corpo no [`World3`]. Continua único mesmo depois
/// que outros corpos são removidos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyId3(RigidBodyHandle);

impl PartialOrd for BodyId3 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BodyId3 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_bits().cmp(&other.to_bits())
    }
}

impl BodyId3 {
    /// Um número que identifica o corpo (para guardar ou comparar).
    pub fn to_bits(self) -> u64 {
        let (i, g) = self.0.into_raw_parts();
        (u64::from(g) << 32) | u64::from(i)
    }

    pub fn from_bits(bits: u64) -> Self {
        Self(RigidBodyHandle::from_raw_parts(
            bits as u32,
            (bits >> 32) as u32,
        ))
    }
}

/// Como o corpo se move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BodyKind3 {
    /// Não se move: chão, paredes, cenário.
    Static,
    /// Cai, rola, empilha e é empurrado.
    #[default]
    Dynamic,
    /// Movido pelo jogo (posição ou velocidade); empurra os dinâmicos.
    Kinematic,
}

/// A forma de colisão, centrada na posição do corpo (mais `offset`).
#[derive(Debug, Clone, PartialEq)]
pub enum Shape3 {
    /// Caixa com largura, altura e profundidade.
    Box(Vec3),
    Sphere {
        radius: f32,
    },
    /// Em pé (eixo Y). `height` é a altura total, com as pontas redondas.
    Capsule {
        radius: f32,
        height: f32,
    },
    /// Em pé (eixo Y).
    Cylinder {
        radius: f32,
        height: f32,
    },
    /// Os triângulos exatos (só para corpos estáticos ou cinemáticos:
    /// cenário, morros, a casa).
    Mesh {
        vertices: Vec<Vec3>,
        triangles: Vec<[u32; 3]>,
    },
}

impl Shape3 {
    /// A forma como o rapier entende, ou um erro em português.
    fn collider(&self) -> Result<ColliderBuilder, PhysicsError> {
        Ok(match self {
            Shape3::Box(size) => {
                let h = size.max(Vec3::splat(1e-3)) / 2.0;
                ColliderBuilder::cuboid(h.x, h.y, h.z)
            }
            Shape3::Sphere { radius } => ColliderBuilder::ball(radius.max(1e-3)),
            Shape3::Capsule { radius, height } => {
                let r = radius.max(1e-3);
                ColliderBuilder::capsule_y((height / 2.0 - r).max(0.0), r)
            }
            Shape3::Cylinder { radius, height } => {
                ColliderBuilder::cylinder((height / 2.0).max(1e-3), radius.max(1e-3))
            }
            Shape3::Mesh {
                vertices,
                triangles,
            } => {
                if triangles.is_empty() {
                    return Err(PhysicsError::EmptyMesh);
                }
                let n = vertices.len() as u32;
                if triangles.iter().flatten().any(|&i| i >= n) {
                    return Err(PhysicsError::BadMesh("índice fora da malha".into()));
                }
                ColliderBuilder::trimesh(
                    vertices.iter().copied().map(rv).collect(),
                    triangles.clone(),
                )
                .map_err(|e| PhysicsError::BadMesh(e.to_string()))?
            }
        })
    }
}

/// Erros ao criar corpos.
#[derive(Debug, Clone, PartialEq)]
pub enum PhysicsError {
    EmptyMesh,
    BadMesh(String),
    MeshNotStatic,
}

impl std::error::Error for PhysicsError {}

impl std::fmt::Display for PhysicsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PhysicsError::EmptyMesh => write!(f, "a malha de colisão não tem triângulos"),
            PhysicsError::BadMesh(e) => write!(f, "malha de colisão inválida: {e}"),
            PhysicsError::MeshNotStatic => write!(
                f,
                "a forma \"mesh\" (triângulos exatos) só vale para corpos static ou kinematic; \
                 para o que cai e rola, use box, sphere, capsule ou cylinder"
            ),
        }
    }
}

/// Como criar um corpo.
#[derive(Debug, Clone, PartialEq)]
pub struct BodyDesc {
    pub kind: BodyKind3,
    pub shape: Shape3,
    /// Deslocamento da forma em relação à posição do corpo (por exemplo,
    /// a caixa de um modelo que começa no chão).
    pub offset: Vec3,
    pub position: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
    /// Massa por volume (padrão 1). Ignorado se `mass` for dado.
    pub density: f32,
    pub mass: Option<f32>,
    /// Atrito (0 = gelo; padrão 0,5).
    pub friction: f32,
    /// Quique (0 = não quica, 1 = quica sem perder velocidade).
    pub bounce: f32,
    /// Não colide: só avisa quando algo entra ou sai.
    pub sensor: bool,
    /// Não gira (fica em pé).
    pub lock_rotation: bool,
    pub gravity_scale: f32,
    /// Freio do movimento e do giro (resistência do ar).
    pub linear_damping: f32,
    pub angular_damping: f32,
    /// Detecção contínua: para coisas rápidas (tiros) não atravessarem paredes finas.
    pub ccd: bool,
}

impl Default for BodyDesc {
    fn default() -> Self {
        Self {
            kind: BodyKind3::Dynamic,
            shape: Shape3::Box(Vec3::ONE),
            offset: Vec3::ZERO,
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            velocity: Vec3::ZERO,
            density: 1.0,
            mass: None,
            friction: 0.5,
            bounce: 0.0,
            sensor: false,
            lock_rotation: false,
            gravity_scale: 1.0,
            linear_damping: 0.0,
            angular_damping: 0.05,
            ccd: false,
        }
    }
}

/// Como criar um personagem: uma cápsula em pé que anda, pula e sobe
/// degraus. A posição é a dos **pés**.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharacterDesc {
    pub position: Vec3,
    pub radius: f32,
    pub height: f32,
    /// Altura máxima de degrau que ele sobe sozinho.
    pub step: f32,
    /// Rampa mais inclinada que ele sobe, em graus.
    pub max_slope: f32,
    /// Massa usada para empurrar o que é dinâmico.
    pub mass: f32,
}

impl Default for CharacterDesc {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            radius: 0.35,
            height: 1.8,
            step: 0.35,
            max_slope: 45.0,
            mass: 70.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Character {
    collider: ColliderHandle,
    desc: CharacterDesc,
    /// Velocidade de andar pedida para o próximo passo (horizontal).
    walk: Vec3,
    /// Velocidade vertical (gravidade e pulo).
    vy: f32,
    on_ground: bool,
    /// Pulo pedido (velocidade), aplicado no próximo passo.
    jump: Option<f32>,
    /// Bateu a cabeça / encostou numa parede no último passo.
    hit_ceiling: bool,
    hit_wall: bool,
}

/// Algo que aconteceu num passo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event3 {
    /// Começaram a se tocar (ou um entrou num sensor). Se um dos dois for
    /// sensor, ele vem primeiro.
    Enter(BodyId3, BodyId3),
    /// Pararam de se tocar (ou um saiu do sensor).
    Exit(BodyId3, BodyId3),
}

/// O que um raio acertou.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayHit3 {
    pub body: BodyId3,
    pub point: Vec3,
    pub normal: Vec3,
    pub distance: f32,
}

/// Dados de cada corpo que o rapier não guarda.
#[derive(Debug, Clone)]
struct Info {
    kind: BodyKind3,
    shape: Shape3,
    offset: Vec3,
    sensor: bool,
}

/// O mundo da física 3D.
pub struct World3 {
    world: PhysicsWorld,
    infos: HashMap<RigidBodyHandle, Info>,
    characters: HashMap<RigidBodyHandle, Character>,
    controller: KinematicCharacterController,
    /// Lista de corpos vazia, para o controle do personagem (veja
    /// `move_characters`).
    no_bodies: RigidBodySet,
    events: Vec<Event3>,
    collision_rx: std::sync::mpsc::Receiver<CollisionEvent>,
    collision_tx: std::sync::mpsc::Sender<CollisionEvent>,
    force_tx: std::sync::mpsc::Sender<ContactForceEvent>,
    tear_tx: std::sync::mpsc::Sender<SoftBodyTearEvent>,
    // (Os receptores de força e de rasgo não são usados, mas precisam
    // existir para o envio não falhar.)
    _force_rx: std::sync::mpsc::Receiver<ContactForceEvent>,
    _tear_rx: std::sync::mpsc::Receiver<SoftBodyTearEvent>,
}

impl std::fmt::Debug for World3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("World3")
            .field("bodies", &self.infos.len())
            .field("gravity", &self.gravity())
            .finish_non_exhaustive()
    }
}

impl Default for World3 {
    fn default() -> Self {
        Self::new(Vec3::new(0.0, -9.81, 0.0))
    }
}

impl World3 {
    pub fn new(gravity: Vec3) -> Self {
        let world = PhysicsWorld {
            gravity: rv(gravity),
            ..Default::default()
        };
        let (collision_tx, collision_rx) = std::sync::mpsc::channel();
        let (force_tx, force_rx) = std::sync::mpsc::channel();
        let (tear_tx, tear_rx) = std::sync::mpsc::channel();
        Self {
            world,
            infos: HashMap::new(),
            characters: HashMap::new(),
            controller: KinematicCharacterController::default(),
            no_bodies: RigidBodySet::new(),
            events: Vec::new(),
            collision_rx,
            collision_tx,
            force_tx,
            tear_tx,
            _force_rx: force_rx,
            _tear_rx: tear_rx,
        }
    }

    pub fn gravity(&self) -> Vec3 {
        gv(self.world.gravity)
    }

    pub fn set_gravity(&mut self, g: Vec3) {
        self.world.gravity = rv(g);
    }

    /// Quantos corpos há no mundo.
    pub fn len(&self) -> usize {
        self.infos.len()
    }

    pub fn is_empty(&self) -> bool {
        self.infos.is_empty()
    }

    pub fn contains(&self, id: BodyId3) -> bool {
        self.infos.contains_key(&id.0)
    }

    /// Todos os corpos.
    pub fn ids(&self) -> Vec<BodyId3> {
        let mut v: Vec<BodyId3> = self.infos.keys().map(|h| BodyId3(*h)).collect();
        v.sort();
        v
    }

    /// Cria um corpo.
    pub fn add(&mut self, desc: &BodyDesc) -> Result<BodyId3, PhysicsError> {
        if matches!(desc.shape, Shape3::Mesh { .. }) && desc.kind == BodyKind3::Dynamic {
            return Err(PhysicsError::MeshNotStatic);
        }
        let builder = match desc.kind {
            BodyKind3::Static => RigidBodyBuilder::fixed(),
            BodyKind3::Dynamic => RigidBodyBuilder::dynamic(),
            BodyKind3::Kinematic => RigidBodyBuilder::kinematic_velocity_based(),
        };
        let mut builder = builder
            .pose(RPose::from_parts(
                rv(desc.position),
                rq(desc.rotation.normalize()),
            ))
            .linvel(rv(desc.velocity))
            .gravity_scale(desc.gravity_scale)
            .linear_damping(desc.linear_damping.max(0.0))
            .angular_damping(desc.angular_damping.max(0.0))
            .ccd_enabled(desc.ccd);
        if desc.lock_rotation {
            builder = builder.lock_rotations();
        }
        let mut collider = desc
            .shape
            .collider()?
            .translation(rv(desc.offset))
            .friction(desc.friction.max(0.0))
            .restitution(desc.bounce.clamp(0.0, 1.0))
            // O quique de um vale mesmo batendo em algo que não quica.
            .restitution_combine_rule(CoefficientCombineRule::Max)
            .sensor(desc.sensor)
            .active_events(ActiveEvents::COLLISION_EVENTS)
            .active_collision_types(ActiveCollisionTypes::all());
        collider = match desc.mass {
            Some(m) => collider.mass(m.max(1e-3)),
            None => collider.density(desc.density.max(1e-3)),
        };
        let (handle, _) = self.world.insert(builder, collider);
        self.infos.insert(
            handle,
            Info {
                kind: desc.kind,
                shape: desc.shape.clone(),
                offset: desc.offset,
                sensor: desc.sensor,
            },
        );
        Ok(BodyId3(handle))
    }

    /// Cria um personagem (veja [`CharacterDesc`]).
    pub fn add_character(&mut self, desc: &CharacterDesc) -> BodyId3 {
        let r = desc.radius.max(0.05);
        let h = desc.height.max(r * 2.0 + 0.01);
        let body = RigidBodyBuilder::kinematic_position_based()
            .translation(rv(desc.position + Vec3::Y * h / 2.0));
        let collider = ColliderBuilder::capsule_y(h / 2.0 - r, r)
            .active_events(ActiveEvents::COLLISION_EVENTS)
            .active_collision_types(ActiveCollisionTypes::all());
        let (handle, colliders) = self.world.insert(body, collider);
        let desc = CharacterDesc {
            radius: r,
            height: h,
            ..*desc
        };
        self.infos.insert(
            handle,
            Info {
                kind: BodyKind3::Kinematic,
                shape: Shape3::Capsule {
                    radius: r,
                    height: h,
                },
                offset: Vec3::ZERO,
                sensor: false,
            },
        );
        self.characters.insert(
            handle,
            Character {
                collider: colliders,
                desc,
                walk: Vec3::ZERO,
                vy: 0.0,
                on_ground: false,
                jump: None,
                hit_ceiling: false,
                hit_wall: false,
            },
        );
        BodyId3(handle)
    }

    pub fn is_character(&self, id: BodyId3) -> bool {
        self.characters.contains_key(&id.0)
    }

    /// Remove o corpo. `false` se ele já não existia.
    pub fn remove(&mut self, id: BodyId3) -> bool {
        self.characters.remove(&id.0);
        if self.infos.remove(&id.0).is_none() {
            return false;
        }
        self.world.remove_body(id.0);
        true
    }

    pub fn kind(&self, id: BodyId3) -> Option<BodyKind3> {
        self.infos.get(&id.0).map(|i| i.kind)
    }

    pub fn shape(&self, id: BodyId3) -> Option<&Shape3> {
        self.infos.get(&id.0).map(|i| &i.shape)
    }

    pub fn shape_offset(&self, id: BodyId3) -> Option<Vec3> {
        self.infos.get(&id.0).map(|i| i.offset)
    }

    pub fn is_sensor(&self, id: BodyId3) -> bool {
        self.infos.get(&id.0).is_some_and(|i| i.sensor)
    }

    fn body(&self, id: BodyId3) -> Option<&RigidBody> {
        self.world.bodies.get(id.0)
    }

    fn body_mut(&mut self, id: BodyId3) -> Option<&mut RigidBody> {
        self.world.bodies.get_mut(id.0)
    }

    /// Posição do corpo (de um personagem: dos pés).
    pub fn position(&self, id: BodyId3) -> Option<Vec3> {
        let p = gv(self.body(id)?.translation());
        Some(match self.characters.get(&id.0) {
            Some(c) => p - Vec3::Y * c.desc.height / 2.0,
            None => p,
        })
    }

    pub fn set_position(&mut self, id: BodyId3, p: Vec3) {
        let p = match self.characters.get_mut(&id.0) {
            Some(c) => {
                c.vy = 0.0;
                p + Vec3::Y * c.desc.height / 2.0
            }
            None => p,
        };
        if let Some(b) = self.body_mut(id) {
            b.set_translation(rv(p), true);
        }
    }

    pub fn rotation(&self, id: BodyId3) -> Option<Quat> {
        Some(gq(*self.body(id)?.rotation()))
    }

    pub fn set_rotation(&mut self, id: BodyId3, q: Quat) {
        if let Some(b) = self.body_mut(id) {
            b.set_rotation(rq(q.normalize()), true);
        }
    }

    pub fn velocity(&self, id: BodyId3) -> Option<Vec3> {
        if let Some(c) = self.characters.get(&id.0) {
            return Some(Vec3::new(c.walk.x, c.vy, c.walk.z));
        }
        Some(gv(self.body(id)?.linvel()))
    }

    pub fn set_velocity(&mut self, id: BodyId3, v: Vec3) {
        if let Some(c) = self.characters.get_mut(&id.0) {
            c.walk = Vec3::new(v.x, 0.0, v.z);
            c.vy = v.y;
            return;
        }
        if let Some(b) = self.body_mut(id) {
            b.set_linvel(rv(v), true);
        }
    }

    /// Velocidade de giro (radianos por segundo em volta de cada eixo).
    pub fn angular_velocity(&self, id: BodyId3) -> Option<Vec3> {
        Some(gv(self.body(id)?.angvel()))
    }

    pub fn set_angular_velocity(&mut self, id: BodyId3, w: Vec3) {
        if let Some(b) = self.body_mut(id) {
            b.set_angvel(rv(w), true);
        }
    }

    /// Um empurrão instantâneo (muda a velocidade em `impulso / massa`).
    pub fn apply_impulse(&mut self, id: BodyId3, impulse: Vec3) {
        if let Some(b) = self.body_mut(id) {
            b.apply_impulse(rv(impulse), true);
        }
    }

    /// Um empurrão num ponto do mundo (faz girar).
    pub fn apply_impulse_at(&mut self, id: BodyId3, impulse: Vec3, point: Vec3) {
        if let Some(b) = self.body_mut(id) {
            b.apply_impulse_at_point(rv(impulse), rv(point), true);
        }
    }

    pub fn apply_torque_impulse(&mut self, id: BodyId3, torque: Vec3) {
        if let Some(b) = self.body_mut(id) {
            b.apply_torque_impulse(rv(torque), true);
        }
    }

    pub fn mass(&self, id: BodyId3) -> Option<f32> {
        Some(self.body(id)?.mass())
    }

    pub fn gravity_scale(&self, id: BodyId3) -> Option<f32> {
        Some(self.body(id)?.gravity_scale())
    }

    pub fn set_gravity_scale(&mut self, id: BodyId3, s: f32) {
        if let Some(b) = self.body_mut(id) {
            b.set_gravity_scale(s, true);
        }
    }

    /// Está parado, "dormindo" (a física não gasta tempo com ele).
    pub fn is_sleeping(&self, id: BodyId3) -> bool {
        self.body(id).is_some_and(|b| b.is_sleeping())
    }

    // --- Personagens -----------------------------------------------------

    /// Velocidade de andar do personagem para os próximos passos (só X e Z
    /// contam; a gravidade cuida do Y).
    pub fn walk(&mut self, id: BodyId3, velocity: Vec3) {
        if let Some(c) = self.characters.get_mut(&id.0) {
            c.walk = Vec3::new(velocity.x, 0.0, velocity.z);
        }
    }

    /// Pula com a velocidade `speed` (para cima), se estiver no chão.
    /// Devolve se pulou.
    pub fn jump(&mut self, id: BodyId3, speed: f32) -> bool {
        match self.characters.get_mut(&id.0) {
            Some(c) if c.on_ground => {
                c.jump = Some(speed);
                c.on_ground = false;
                true
            }
            _ => false,
        }
    }

    /// O personagem está no chão?
    pub fn on_ground(&self, id: BodyId3) -> bool {
        self.characters.get(&id.0).is_some_and(|c| c.on_ground)
    }

    /// O personagem bateu a cabeça no último passo?
    pub fn hit_ceiling(&self, id: BodyId3) -> bool {
        self.characters.get(&id.0).is_some_and(|c| c.hit_ceiling)
    }

    /// O personagem encostou numa parede no último passo?
    pub fn hit_wall(&self, id: BodyId3) -> bool {
        self.characters.get(&id.0).is_some_and(|c| c.hit_wall)
    }

    /// Velocidade do que está embaixo dos pés do personagem (zero se for
    /// parado ou nada).
    fn ground_velocity(&self, handle: RigidBodyHandle, center: Vec3, desc: &CharacterDesc) -> Vec3 {
        let feet = center - Vec3::Y * (desc.height / 2.0 - desc.radius);
        let filter = QueryFilter::default()
            .exclude_rigid_body(handle)
            .exclude_sensors();
        let ray = Ray::new(rv(feet), rv(Vec3::NEG_Y));
        let Some((h, _)) = self.world.query_pipeline_with_filter(filter).cast_ray(
            &ray,
            desc.radius + desc.step.max(0.05) + 0.1,
            true,
        ) else {
            return Vec3::ZERO;
        };
        let Some(body) = self
            .world
            .colliders
            .get(h)
            .and_then(|c| c.parent())
            .and_then(|b| self.world.bodies.get(b))
        else {
            return Vec3::ZERO;
        };
        if body.is_fixed() {
            Vec3::ZERO
        } else {
            gv(body.linvel())
        }
    }

    fn move_characters(&mut self, dt: f32) {
        let gravity = self.world.gravity.y;
        let handles: Vec<RigidBodyHandle> = self.characters.keys().copied().collect();
        for handle in handles {
            let Some(mut c) = self.characters.get(&handle).copied() else {
                continue;
            };
            let Some(body) = self.world.bodies.get(handle) else {
                continue;
            };
            let pos = *body.position();
            if let Some(speed) = c.jump.take() {
                c.vy = speed;
            } else if c.on_ground && c.vy <= 0.0 {
                // Encostado no chão: só o suficiente para continuar grudado.
                c.vy = gravity * dt;
            } else {
                c.vy += gravity * dt;
            }
            // (Em cima de uma plataforma que se move, o controle do rapier já
            // leva o personagem junto.)
            let desired = Vec3::new(c.walk.x * dt, c.vy * dt, c.walk.z * dt);
            // Em cima de algo que se move (plataforma): vai junto. (O controle
            // do rapier também tentaria, mas só em alguns quadros; por isso
            // ele recebe a lista de corpos vazia e não vê a velocidade.)
            let carried = if c.vy <= 0.0 {
                self.ground_velocity(handle, gv(pos.translation), &c.desc)
            } else {
                Vec3::ZERO
            };
            let controller = KinematicCharacterController {
                autostep: Some(CharacterAutostep {
                    max_height: CharacterLength::Absolute(c.desc.step.max(0.0)),
                    min_width: CharacterLength::Absolute(c.desc.radius * 0.5),
                    include_dynamic_bodies: false,
                }),
                max_slope_climb_angle: c.desc.max_slope.clamp(0.0, 89.0).to_radians(),
                min_slope_slide_angle: c.desc.max_slope.clamp(0.0, 89.0).to_radians(),
                snap_to_ground: if c.vy > 0.0 {
                    None
                } else {
                    Some(CharacterLength::Absolute(c.desc.step.max(0.05)))
                },
                offset: CharacterLength::Absolute(0.02),
                ..self.controller
            };
            let shape = self.world.colliders[c.collider].shared_shape().clone();
            let mut collisions = Vec::new();
            let filter = QueryFilter::default()
                .exclude_rigid_body(handle)
                .exclude_sensors();
            let movement = {
                let queries = self.world.broad_phase.as_query_pipeline(
                    self.world.narrow_phase.query_dispatcher(),
                    &self.no_bodies,
                    &self.world.colliders,
                    filter,
                );
                controller.move_shape(dt, &queries, shape.as_ref(), &pos, rv(desired), |hit| {
                    collisions.push(hit)
                })
            };
            let moved = gv(movement.translation);
            c.on_ground = movement.grounded;
            c.hit_ceiling = c.vy > 0.0 && moved.y < desired.y * 0.5;
            if c.hit_ceiling {
                c.vy = 0.0;
            }
            let horizontal = Vec3::new(desired.x, 0.0, desired.z);
            let moved_h = Vec3::new(moved.x, 0.0, moved.z);
            c.hit_wall = horizontal.length() > 1e-5
                && moved_h.length() < horizontal.length() * 0.5
                && collisions.iter().any(|h| gv(h.hit.normal1).y.abs() < 0.3);
            if c.on_ground && c.vy < 0.0 {
                c.vy = 0.0;
            }
            // Empurra o que é dinâmico.
            {
                let mut queries = self.world.broad_phase.as_query_pipeline_mut(
                    self.world.narrow_phase.query_dispatcher(),
                    &mut self.world.bodies,
                    &mut self.world.colliders,
                    filter,
                );
                controller.solve_character_collision_impulses(
                    dt,
                    &mut queries,
                    shape.as_ref(),
                    c.desc.mass,
                    collisions.iter(),
                );
            }
            if let Some(body) = self.world.bodies.get_mut(handle) {
                body.set_next_kinematic_translation(pos.translation + rv(moved + carried * dt));
            }
            self.characters.insert(handle, c);
        }
    }

    /// Avança a simulação `dt` segundos (use o passo fixo do jogo). Os
    /// eventos do passo ficam em [`World3::events`].
    pub fn step(&mut self, dt: f32) {
        let dt = dt.clamp(1e-4, 0.1);
        self.world.integration_parameters.dt = dt;
        self.move_characters(dt);
        let collector = ChannelEventCollector::new(
            self.collision_tx.clone(),
            self.force_tx.clone(),
            self.tear_tx.clone(),
        );
        self.world.step_with_events(&(), &collector);
        self.events.clear();
        while let Ok(e) = self.collision_rx.try_recv() {
            let (a, b, started) = match e {
                CollisionEvent::Started(a, b, _) => (a, b, true),
                CollisionEvent::Stopped(a, b, _) => (a, b, false),
            };
            let body = |h: ColliderHandle| {
                self.world
                    .colliders
                    .get(h)
                    .and_then(|c| c.parent())
                    .map(BodyId3)
            };
            let (Some(mut x), Some(mut y)) = (body(a), body(b)) else {
                continue;
            };
            if !self.is_sensor(x) && self.is_sensor(y) {
                std::mem::swap(&mut x, &mut y);
            }
            self.events.push(if started {
                Event3::Enter(x, y)
            } else {
                Event3::Exit(x, y)
            });
        }
    }

    /// Os eventos do último [`World3::step`].
    pub fn events(&self) -> &[Event3] {
        &self.events
    }

    /// O primeiro corpo que o raio acerta, saindo de `origin` na direção
    /// `dir`, até `max` de distância. Sensores não contam. `ignore`: um corpo
    /// que o raio atravessa (quem atira).
    pub fn raycast(
        &self,
        origin: Vec3,
        dir: Vec3,
        max: f32,
        ignore: Option<BodyId3>,
    ) -> Option<RayHit3> {
        let dir = dir.normalize_or_zero();
        if dir == Vec3::ZERO {
            return None;
        }
        let mut filter = QueryFilter::default().exclude_sensors();
        if let Some(i) = ignore {
            filter = filter.exclude_rigid_body(i.0);
        }
        let ray = Ray::new(rv(origin), rv(dir));
        let (h, hit) = self
            .world
            .query_pipeline_with_filter(filter)
            .cast_ray_and_get_normal(&ray, max.max(0.0), true)?;
        let body = self.world.colliders.get(h)?.parent()?;
        Some(RayHit3 {
            body: BodyId3(body),
            point: origin + dir * hit.time_of_impact,
            normal: gv(hit.normal),
            distance: hit.time_of_impact,
        })
    }

    /// Os corpos que encostam numa esfera (sensores incluídos).
    pub fn query_sphere(&self, center: Vec3, radius: f32) -> Vec<BodyId3> {
        let shape = Ball::new(radius.max(1e-3));
        let mut out: Vec<BodyId3> = self
            .world
            .query_pipeline()
            .intersect_shape(RPose::from_translation(rv(center)), &shape)
            .filter_map(|(_, c)| c.parent().map(BodyId3))
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// Os corpos que estão tocando `id` agora (contato ou sensor).
    pub fn touching(&self, id: BodyId3) -> Vec<BodyId3> {
        let Some(body) = self.body(id) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for &c in body.colliders() {
            for pair in self.world.narrow_phase.contact_pairs_with(c) {
                if pair.has_any_active_contact() {
                    let other = if pair.collider1 == c {
                        pair.collider2
                    } else {
                        pair.collider1
                    };
                    if let Some(p) = self.world.colliders.get(other).and_then(|o| o.parent()) {
                        out.push(BodyId3(p));
                    }
                }
            }
            for (a, b, hit) in self.world.narrow_phase.intersection_pairs_with(c) {
                if hit {
                    let other = if a == c { b } else { a };
                    if let Some(p) = self.world.colliders.get(other).and_then(|o| o.parent()) {
                        out.push(BodyId3(p));
                    }
                }
            }
        }
        out.sort();
        out.dedup();
        out
    }
}

#[cfg(test)]
mod tests;
