//! 3D: malhas com luz, câmera em perspectiva e formas prontas.
//!
//! O mundo 3D é desenhado antes de tudo o que é 2D no mesmo
//! [`Renderer::render`](crate::Renderer::render): sprites, texto e
//! `screen()` (HUD, menus) ficam por cima. O visual mira o *low-poly*: cada
//! face recebe uma cor de luz só (normais por face), com o sol (luz
//! direcional) e uma luz ambiente que vem do céu e do chão.
//!
//! ```no_run
//! # fn exemplo(r: &mut nn_render::Renderer) {
//! use glam::{Mat4, Vec3};
//! use nn_core::Color;
//! use nn_render::d3::{self, Camera3D};
//! let cubo = r.create_mesh(&d3::shapes::cube());
//! r.set_camera3d(Camera3D::looking_at(Vec3::new(3.0, 2.0, 4.0), Vec3::ZERO));
//! r.draw_mesh(cubo, Mat4::IDENTITY, Color::hex(0x3FA7D6));
//! # }
//! ```

mod gpu;
mod mesh;
mod model;
pub mod shapes;

use glam::{Mat4, Quat, Vec2, Vec3};
use nn_core::Color;

pub(crate) use gpu::Renderer3D;
pub use mesh::{Mesh, Vertex3};
pub use model::{CollisionMesh, Model, ModelData, ModelError, ModelImage, ModelPart};

/// Câmera em perspectiva: de onde olha, para onde e o campo de visão.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera3D {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    /// Campo de visão vertical, em graus.
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self::looking_at(Vec3::new(0.0, 4.0, 8.0), Vec3::ZERO)
    }
}

impl Camera3D {
    /// Câmera em `position` olhando para `target` (Y para cima, 60°).
    pub fn looking_at(position: Vec3, target: Vec3) -> Self {
        Self {
            position,
            target,
            up: Vec3::Y,
            fov: 60.0,
            near: 0.1,
            far: 1000.0,
        }
    }

    pub fn view(&self) -> Mat4 {
        let up = if (self.target - self.position)
            .normalize_or_zero()
            .cross(self.up)
            .length()
            < 1e-4
        {
            // Olhando reto para cima ou para baixo: outro "cima".
            Vec3::Z
        } else {
            self.up
        };
        glam::camera::rh::view::look_at_mat4(self.position, self.target, up)
    }

    pub fn projection(&self, aspect: f32) -> Mat4 {
        glam::camera::rh::proj::directx::perspective(
            self.fov.clamp(1.0, 179.0).to_radians(),
            aspect.max(1e-4),
            self.near.max(1e-4),
            self.far.max(self.near + 1e-3),
        )
    }

    /// Matriz que leva do mundo para a tela (com a proporção largura ÷ altura).
    pub fn view_projection(&self, aspect: f32) -> Mat4 {
        self.projection(aspect) * self.view()
    }

    /// Onde o ponto `world` aparece na tela de tamanho `size` (pixels, origem
    /// no canto de cima à esquerda). `None` se estiver atrás da câmera.
    pub fn screen_point(&self, world: Vec3, size: Vec2) -> Option<Vec2> {
        let clip = self.view_projection(size.x / size.y.max(1.0)) * world.extend(1.0);
        if clip.w <= 1e-6 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        Some(Vec2::new(
            (ndc.x * 0.5 + 0.5) * size.x,
            (0.5 - ndc.y * 0.5) * size.y,
        ))
    }

    /// O raio que sai da câmera passando pelo ponto `screen` da tela (para
    /// clicar em coisas 3D): origem e direção (normalizada).
    pub fn ray(&self, screen: Vec2, size: Vec2) -> (Vec3, Vec3) {
        let inv = self.view_projection(size.x / size.y.max(1.0)).inverse();
        let ndc = Vec2::new(
            screen.x / size.x.max(1.0) * 2.0 - 1.0,
            1.0 - screen.y / size.y.max(1.0) * 2.0,
        );
        let near = inv.project_point3(ndc.extend(0.0));
        let far = inv.project_point3(ndc.extend(1.0));
        (near, (far - near).normalize_or_zero())
    }
}

/// Névoa: some com o que está longe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fog {
    /// Cor da névoa. `None`: a cor do horizonte do céu (ou a da luz ambiente
    /// de cima, sem céu).
    pub color: Option<Color>,
    /// Distância em que a névoa começa e em que cobre tudo.
    pub near: f32,
    pub far: f32,
}

/// A luz do mundo 3D: o sol, a luz ambiente e a névoa.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light3D {
    /// Direção para onde a luz do sol vai (de cima para baixo: Y negativo).
    pub sun_direction: Vec3,
    pub sun_color: Color,
    /// Luz ambiente que vem de cima (céu) e de baixo (chão).
    pub sky: Color,
    pub ground: Color,
    pub fog: Option<Fog>,
}

impl Default for Light3D {
    fn default() -> Self {
        Self {
            sun_direction: Vec3::new(-0.4, -1.0, -0.6).normalize(),
            sun_color: Color::rgb(1.0, 0.96, 0.88),
            sky: Color::rgb(0.42, 0.47, 0.58),
            ground: Color::rgb(0.22, 0.2, 0.18),
            fog: None,
        }
    }
}

/// Céu em degradê, desenhado atrás de tudo no mundo 3D, com o disco do sol
/// na direção da luz.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sky3D {
    /// Cor lá em cima.
    pub top: Color,
    /// Cor na linha do horizonte.
    pub horizon: Color,
    /// Cor abaixo do horizonte.
    pub bottom: Color,
    /// Tamanho do sol, em graus (0 = sem disco).
    pub sun_size: f32,
}

impl Default for Sky3D {
    fn default() -> Self {
        Self {
            top: Color::hex(0x4A90D9),
            horizon: Color::hex(0xBFE3F2),
            bottom: Color::hex(0x8C9BA5),
            sun_size: 3.0,
        }
    }
}

/// Sombras do sol.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadows {
    pub enabled: bool,
    /// Até que distância do ponto para onde a câmera olha há sombra.
    pub distance: f32,
    /// Tamanho do mapa de sombras (mais = sombras mais nítidas, mais lento).
    pub resolution: u32,
    /// Quanto da luz do sol a sombra tira (0 a 1).
    pub strength: f32,
}

impl Default for Shadows {
    fn default() -> Self {
        Self {
            enabled: true,
            distance: 30.0,
            resolution: 2048,
            strength: 1.0,
        }
    }
}

/// Posição, rotação e escala de algo no mundo 3D.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform3 {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform3 {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform3 {
    pub fn at(position: Vec3) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }

    /// Rotação em graus em volta de X, Y e Z (aplicada Y, depois X, depois Z:
    /// virar, inclinar e rolar).
    pub fn rotation_degrees(mut self, x: f32, y: f32, z: f32) -> Self {
        self.rotation = Quat::from_euler(
            glam::EulerRot::YXZ,
            y.to_radians(),
            x.to_radians(),
            z.to_radians(),
        );
        self
    }

    pub fn scaled(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

/// Uma malha enviada à GPU por [`Renderer::create_mesh`](crate::Renderer::create_mesh).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MeshId(pub(crate) u32);

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Mat3, Vec4};

    #[test]
    fn ponto_na_frente_da_camera_cai_no_meio_da_tela() {
        let cam = Camera3D::looking_at(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO);
        let size = Vec2::new(800.0, 600.0);
        let p = cam.screen_point(Vec3::ZERO, size).unwrap();
        assert!((p - Vec2::new(400.0, 300.0)).length() < 0.01, "{p}");
        // Acima e à direita no mundo: acima e à direita na tela (Y para baixo).
        let q = cam.screen_point(Vec3::new(1.0, 1.0, 0.0), size).unwrap();
        assert!(q.x > 400.0 && q.y < 300.0, "{q}");
        // Atrás da câmera: nada.
        assert!(cam.screen_point(Vec3::new(0.0, 0.0, 10.0), size).is_none());
    }

    #[test]
    fn raio_do_clique_volta_ao_ponto() {
        let cam = Camera3D::looking_at(Vec3::new(3.0, 4.0, 5.0), Vec3::new(0.0, 1.0, 0.0));
        let size = Vec2::new(640.0, 360.0);
        let world = Vec3::new(0.5, 0.2, -0.7);
        let screen = cam.screen_point(world, size).unwrap();
        let (origin, dir) = cam.ray(screen, size);
        // O ponto fica sobre o raio.
        let t = (world - origin).dot(dir);
        assert!(t > 0.0);
        assert!((origin + dir * t - world).length() < 1e-3);
    }

    #[test]
    fn olhar_reto_para_baixo_nao_quebra() {
        let cam = Camera3D::looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::ZERO);
        let m = cam.view_projection(1.0);
        assert!(m.is_finite());
    }

    #[test]
    fn rotacao_em_graus() {
        let t = Transform3::default().rotation_degrees(0.0, 90.0, 0.0);
        // Virar 90° em Y leva a frente (−Z) para a esquerda (−X).
        let v = t.matrix().transform_vector3(Vec3::NEG_Z);
        assert!((v - Vec3::NEG_X).length() < 1e-5, "{v}");
    }

    #[test]
    fn normais_acompanham_a_escala() {
        // Esticado em X, a normal de um plano inclinado não pode esticar junto.
        let model = Mat4::from_scale(Vec3::new(4.0, 1.0, 1.0));
        let inst = gpu::Instance3::new(model, Color::WHITE, false);
        let n = Mat3::from_cols(
            Vec4::from(inst.normal[0]).truncate(),
            Vec4::from(inst.normal[1]).truncate(),
            Vec4::from(inst.normal[2]).truncate(),
        );
        let normal = (n * Vec3::new(1.0, 1.0, 0.0).normalize()).normalize();
        // A superfície x + y = c esticada em X vira mais "deitada": normal mais para Y.
        assert!(normal.y > normal.x, "{normal}");
    }
}
