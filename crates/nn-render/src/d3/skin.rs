//! Esqueletos e animações de modelos glTF.
//!
//! Um modelo animado tem um [`Skeleton`]: os nós do arquivo (ossos e
//! partes) com a posição de descanso e as animações com nome. Cada vértice
//! segue até 4 ossos ([`SkinVertex`]); a GPU deforma a malha com as matrizes
//! dos ossos ([`Skeleton::joint_matrices`]) de uma [`Pose`].
//!
//! Partes sem ossos de um modelo animado (um chapéu preso na cabeça, uma
//! roda que gira) seguem o nó delas, como se fossem um osso.

use glam::{Mat4, Quat, Vec3, Vec4};

/// Quantos ossos um modelo pode ter (o limite do que vai para a GPU de uma
/// vez).
pub const MAX_JOINTS: usize = 128;

/// Os ossos de um vértice e quanto cada um puxa (a soma dá 1).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkinVertex {
    pub joints: [u32; 4],
    pub weights: [f32; 4],
}

impl SkinVertex {
    /// Preso inteiro a um osso só.
    pub fn rigid(joint: u32) -> Self {
        Self {
            joints: [joint, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        }
    }
}

/// Posição, rotação e escala (de um nó, relativas ao pai).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trs {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Trs {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Trs {
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Entre `self` (t = 0) e `other` (t = 1).
    pub fn lerp(&self, other: &Trs, t: f32) -> Trs {
        Trs {
            translation: self.translation.lerp(other.translation, t),
            rotation: nlerp(self.rotation, other.rotation, t),
            scale: self.scale.lerp(other.scale, t),
        }
    }
}

/// Interpolação de rotações pelo caminho mais curto.
fn nlerp(a: Quat, b: Quat, t: f32) -> Quat {
    let b = if a.dot(b) < 0.0 { -b } else { b };
    a.slerp(b, t).normalize()
}

/// Um nó do esqueleto (osso ou parte).
#[derive(Debug, Clone, PartialEq)]
pub struct SkelNode {
    pub name: String,
    /// O pai vem sempre antes na lista.
    pub parent: Option<usize>,
    pub rest: Trs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Property {
    Translation,
    Rotation,
    Scale,
}

/// Os quadros-chave de uma propriedade de um nó.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Channel {
    pub node: usize,
    pub property: Property,
    /// Sem transição: pula de um quadro para o outro.
    pub step: bool,
    pub times: Vec<f32>,
    /// Vec3 (em xyz) ou rotação (xyzw).
    pub values: Vec<Vec4>,
}

impl Channel {
    fn sample(&self, t: f32) -> Vec4 {
        let n = self.times.len().min(self.values.len());
        if n == 0 {
            return Vec4::ZERO;
        }
        if n == 1 || t <= self.times[0] {
            return self.values[0];
        }
        if t >= self.times[n - 1] {
            return self.values[n - 1];
        }
        // O quadro em que `t` cai.
        let i = self.times[..n].partition_point(|&k| k <= t).max(1) - 1;
        let (t0, t1) = (self.times[i], self.times[i + 1]);
        let (a, b) = (self.values[i], self.values[i + 1]);
        if self.step || t1 <= t0 {
            return a;
        }
        let f = (t - t0) / (t1 - t0);
        if self.property == Property::Rotation {
            let q = nlerp(Quat::from_vec4(a), Quat::from_vec4(b), f);
            Vec4::from(q)
        } else {
            a.lerp(b, f)
        }
    }
}

/// Uma animação do modelo ("parado", "andar"...).
#[derive(Debug, Clone, PartialEq)]
pub struct Animation {
    pub name: String,
    /// Em segundos.
    pub duration: f32,
    pub(crate) channels: Vec<Channel>,
}

/// O jeito do esqueleto num instante: a posição de cada nó em relação ao
/// pai.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pose {
    pub locals: Vec<Trs>,
}

impl Pose {
    /// Mistura com outra pose: `t = 0` fica esta, `t = 1` vira a outra.
    pub fn blend(&mut self, other: &Pose, t: f32) {
        let t = t.clamp(0.0, 1.0);
        for (a, b) in self.locals.iter_mut().zip(&other.locals) {
            *a = a.lerp(b, t);
        }
    }
}

/// O esqueleto e as animações de um modelo.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Skeleton {
    pub nodes: Vec<SkelNode>,
    /// Ossos: o nó e a matriz que leva o vértice da pose de descanso para o
    /// espaço do osso (*inverse bind matrix*).
    pub joints: Vec<(usize, Mat4)>,
    pub animations: Vec<Animation>,
}

impl Skeleton {
    /// A pose de descanso (sem animação).
    pub fn rest_pose(&self) -> Pose {
        Pose {
            locals: self.nodes.iter().map(|n| n.rest).collect(),
        }
    }

    /// Índice da animação com esse nome.
    pub fn animation(&self, name: &str) -> Option<usize> {
        self.animations.iter().position(|a| a.name == name)
    }

    /// A pose da animação `index` no instante `time` (em segundos; fora da
    /// animação, fica no primeiro ou no último quadro).
    pub fn pose(&self, index: usize, time: f32) -> Pose {
        let mut pose = self.rest_pose();
        if let Some(anim) = self.animations.get(index) {
            for c in &anim.channels {
                let Some(local) = pose.locals.get_mut(c.node) else {
                    continue;
                };
                let v = c.sample(time);
                match c.property {
                    Property::Translation => local.translation = v.truncate(),
                    Property::Scale => local.scale = v.truncate(),
                    Property::Rotation => {
                        local.rotation = Quat::from_vec4(v).normalize();
                    }
                }
            }
        }
        pose
    }

    /// As matrizes de cada nó no espaço do modelo.
    pub fn globals(&self, pose: &Pose) -> Vec<Mat4> {
        let mut out: Vec<Mat4> = Vec::with_capacity(self.nodes.len());
        for (i, n) in self.nodes.iter().enumerate() {
            let local = pose.locals.get(i).copied().unwrap_or(n.rest).matrix();
            let m = match n.parent {
                Some(p) => out[p] * local,
                None => local,
            };
            out.push(m);
        }
        out
    }

    /// As matrizes dos ossos para a GPU (uma por osso, na ordem de
    /// [`Skeleton::joints`]).
    pub fn joint_matrices(&self, pose: &Pose) -> Vec<Mat4> {
        let globals = self.globals(pose);
        self.joints
            .iter()
            .map(|(node, inverse_bind)| {
                globals.get(*node).copied().unwrap_or(Mat4::IDENTITY) * *inverse_bind
            })
            .collect()
    }

    /// Onde um nó (osso) está no espaço do modelo nesta pose (para prender
    /// coisas na mão, por exemplo).
    pub fn node_matrix(&self, pose: &Pose, name: &str) -> Option<Mat4> {
        let i = self.nodes.iter().position(|n| n.name == name)?;
        self.globals(pose).get(i).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn braco() -> Skeleton {
        // Ombro na origem; o cotovelo 1 acima. Uma animação que dobra o
        // cotovelo 90° em Z em 1 segundo.
        let nodes = vec![
            SkelNode {
                name: "ombro".into(),
                parent: None,
                rest: Trs::default(),
            },
            SkelNode {
                name: "cotovelo".into(),
                parent: Some(0),
                rest: Trs {
                    translation: Vec3::Y,
                    ..Default::default()
                },
            },
        ];
        let q = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        Skeleton {
            joints: vec![
                (0, Mat4::IDENTITY),
                (1, Mat4::from_translation(Vec3::Y).inverse()),
            ],
            animations: vec![Animation {
                name: "dobrar".into(),
                duration: 1.0,
                channels: vec![Channel {
                    node: 1,
                    property: Property::Rotation,
                    step: false,
                    times: vec![0.0, 1.0],
                    values: vec![Vec4::from(Quat::IDENTITY), Vec4::from(q)],
                }],
            }],
            nodes,
        }
    }

    #[test]
    fn descanso_nao_mexe_nos_vertices() {
        let s = braco();
        for m in s.joint_matrices(&s.rest_pose()) {
            assert!(m.abs_diff_eq(Mat4::IDENTITY, 1e-5), "{m}");
        }
    }

    #[test]
    fn animacao_dobra_o_cotovelo_e_mistura() {
        let s = braco();
        let i = s.animation("dobrar").unwrap();
        // A mão (2 acima do ombro, presa ao cotovelo) vai para o lado.
        let mao = Vec3::new(0.0, 2.0, 0.0);
        let j = s.joint_matrices(&s.pose(i, 1.0));
        let p = j[1].transform_point3(mao);
        assert!(p.abs_diff_eq(Vec3::new(-1.0, 1.0, 0.0), 1e-4), "{p}");
        // Na metade do tempo, 45°.
        let j = s.joint_matrices(&s.pose(i, 0.5));
        let p = j[1].transform_point3(mao);
        let h = std::f32::consts::FRAC_1_SQRT_2;
        assert!(p.abs_diff_eq(Vec3::new(-h, 1.0 + h, 0.0), 1e-4), "{p}");
        // Misturar a pose de descanso com a dobrada pela metade dá 45° também.
        let mut pose = s.rest_pose();
        pose.blend(&s.pose(i, 1.0), 0.5);
        let p = s.joint_matrices(&pose)[1].transform_point3(mao);
        assert!(p.abs_diff_eq(Vec3::new(-h, 1.0 + h, 0.0), 1e-4), "{p}");
        // O nó pelo nome.
        let m = s.node_matrix(&s.pose(i, 1.0), "cotovelo").unwrap();
        assert!(m.transform_point3(Vec3::ZERO).abs_diff_eq(Vec3::Y, 1e-5));
    }

    #[test]
    fn quadros_com_degrau_e_fora_do_tempo() {
        let c = Channel {
            node: 0,
            property: Property::Translation,
            step: true,
            times: vec![0.0, 1.0, 2.0],
            values: vec![Vec4::ZERO, Vec4::ONE, Vec4::splat(2.0)],
        };
        assert_eq!(c.sample(0.9), Vec4::ZERO);
        assert_eq!(c.sample(1.5), Vec4::ONE);
        assert_eq!(c.sample(-1.0), Vec4::ZERO);
        assert_eq!(c.sample(9.0), Vec4::splat(2.0));
    }
}
