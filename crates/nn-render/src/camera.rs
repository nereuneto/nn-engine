//! Câmera 2D ortográfica.

use glam::{Mat4, Quat, Vec2, Vec3};

/// Câmera 2D com Y para cima.
///
/// Com `zoom = 1.0`, uma unidade do mundo equivale a um pixel físico da tela
/// e `position` fica no centro da janela.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera2D {
    /// Ponto do mundo que aparece no centro da tela.
    pub position: Vec2,
    /// Fator de ampliação. 2.0 mostra tudo com o dobro do tamanho.
    pub zoom: f32,
    /// Rotação da câmera em radianos (anti-horária).
    pub rotation: f32,
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 1.0,
            rotation: 0.0,
        }
    }
}

impl Camera2D {
    pub fn new(position: Vec2) -> Self {
        Self {
            position,
            ..Self::default()
        }
    }

    /// Matriz que leva coordenadas do mundo para o espaço de recorte da GPU.
    /// `viewport` é o tamanho da área de desenho em pixels.
    pub fn view_projection(&self, viewport: Vec2) -> Mat4 {
        let half = viewport.max(Vec2::ONE) / (2.0 * self.zoom.max(f32::EPSILON));
        // Convenção do wgpu: Y para cima e profundidade de 0 a 1 (igual ao DirectX).
        let projection = glam::camera::rh::proj::directx::orthographic(
            -half.x, half.x, -half.y, half.y, -1.0, 1.0,
        );
        let camera_transform = Mat4::from_rotation_translation(
            Quat::from_rotation_z(self.rotation),
            self.position.extend(0.0),
        );
        projection * camera_transform.inverse()
    }

    /// Converte uma posição da tela (pixels, origem no canto superior esquerdo,
    /// Y para baixo, como a posição do cursor em `nn_input`) para o mundo.
    pub fn screen_to_world(&self, screen: Vec2, viewport: Vec2) -> Vec2 {
        let viewport = viewport.max(Vec2::ONE);
        let ndc = Vec3::new(
            screen.x / viewport.x * 2.0 - 1.0,
            1.0 - screen.y / viewport.y * 2.0,
            0.0,
        );
        self.view_projection(viewport)
            .inverse()
            .project_point3(ndc)
            .truncate()
    }

    /// Converte uma posição do mundo para a tela (inverso de [`Self::screen_to_world`]).
    pub fn world_to_screen(&self, world: Vec2, viewport: Vec2) -> Vec2 {
        let viewport = viewport.max(Vec2::ONE);
        let ndc = self
            .view_projection(viewport)
            .project_point3(world.extend(0.0));
        Vec2::new(
            (ndc.x + 1.0) / 2.0 * viewport.x,
            (1.0 - ndc.y) / 2.0 * viewport.y,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEW: Vec2 = Vec2::new(800.0, 600.0);

    fn perto(a: Vec2, b: Vec2) -> bool {
        a.distance(b) < 1e-3
    }

    #[test]
    fn centro_da_tela_e_a_posicao_da_camera() {
        let cam = Camera2D::new(Vec2::new(100.0, -50.0));
        assert!(perto(
            cam.screen_to_world(VIEW / 2.0, VIEW),
            Vec2::new(100.0, -50.0)
        ));
    }

    #[test]
    fn canto_superior_esquerdo_com_y_para_cima() {
        let cam = Camera2D::default();
        assert!(perto(
            cam.screen_to_world(Vec2::ZERO, VIEW),
            Vec2::new(-400.0, 300.0)
        ));
    }

    #[test]
    fn zoom_reduz_a_area_visivel() {
        let cam = Camera2D {
            zoom: 2.0,
            ..Default::default()
        };
        assert!(perto(
            cam.screen_to_world(Vec2::ZERO, VIEW),
            Vec2::new(-200.0, 150.0)
        ));
    }

    #[test]
    fn ida_e_volta_entre_tela_e_mundo() {
        let cam = Camera2D {
            position: Vec2::new(12.0, 34.0),
            zoom: 1.5,
            rotation: 0.7,
        };
        let tela = Vec2::new(123.0, 456.0);
        let mundo = cam.screen_to_world(tela, VIEW);
        assert!(perto(cam.world_to_screen(mundo, VIEW), tela));
    }
}
