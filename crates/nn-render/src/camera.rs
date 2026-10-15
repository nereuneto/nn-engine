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

/// Retângulo do mundo (Y para cima), usado como limite da câmera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    /// Canto inferior esquerdo.
    pub min: Vec2,
    /// Canto superior direito.
    pub max: Vec2,
}

impl Bounds {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self {
            min: min.min(max),
            max: min.max(max),
        }
    }

    /// A partir do canto inferior esquerdo e do tamanho.
    pub fn from_size(min: Vec2, size: Vec2) -> Self {
        Self::new(min, min + size)
    }

    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    pub fn center(&self) -> Vec2 {
        (self.min + self.max) / 2.0
    }

    pub fn contains(&self, p: Vec2) -> bool {
        p.cmpge(self.min).all() && p.cmple(self.max).all()
    }
}

/// Como a câmera segue um alvo (veja [`Camera2D::follow`]). Todas as medidas
/// em unidades do mundo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Follow {
    /// Rapidez para alcançar o alvo: quanto maior, mais rápido. Com 6, a
    /// câmera percorre ~95% do caminho em meio segundo. 0 vai direto, sem suavizar.
    pub smoothing: f32,
    /// Largura e altura da zona morta: o alvo anda livre dentro dela sem a
    /// câmera se mexer (evita a tela balançar a cada pulo).
    pub deadzone: Vec2,
    /// Somado ao alvo (ex.: olhar um pouco à frente do personagem).
    pub offset: Vec2,
    /// A câmera não mostra nada fora deste retângulo (ex.: o mapa).
    pub bounds: Option<Bounds>,
}

impl Default for Follow {
    fn default() -> Self {
        Self {
            smoothing: 6.0,
            deadzone: Vec2::ZERO,
            offset: Vec2::ZERO,
            bounds: None,
        }
    }
}

impl Camera2D {
    /// Tamanho da área visível no mundo (sem considerar a rotação).
    pub fn view_size(&self, viewport: Vec2) -> Vec2 {
        viewport.max(Vec2::ONE) / self.zoom.max(f32::EPSILON)
    }

    /// Área visível no mundo (sem considerar a rotação).
    pub fn view_bounds(&self, viewport: Vec2) -> Bounds {
        let half = self.view_size(viewport) / 2.0;
        Bounds::new(self.position - half, self.position + half)
    }

    /// Mantém a área visível dentro de `bounds`. Num eixo em que o limite é
    /// menor que a tela, a câmera fica no centro dele.
    pub fn clamp_to(&mut self, bounds: Bounds, viewport: Vec2) {
        self.position = clamp_center(self.position, bounds, self.view_size(viewport));
    }

    /// Move a câmera em direção a `target`, chamado uma vez por frame com o
    /// tempo do frame `dt`. A suavização não depende da taxa de quadros.
    ///
    /// ```
    /// # use nn_render::{Bounds, Camera2D, Follow};
    /// # use glam::Vec2;
    /// let mut camera = Camera2D::default();
    /// let seguir = Follow {
    ///     deadzone: Vec2::new(0.0, 120.0),
    ///     offset: Vec2::new(90.0, 60.0),
    ///     bounds: Some(Bounds::from_size(Vec2::ZERO, Vec2::new(2880.0, 720.0))),
    ///     ..Default::default()
    /// };
    /// camera.follow(Vec2::new(500.0, 100.0), &seguir, 1.0 / 60.0, Vec2::new(1280.0, 720.0));
    /// ```
    pub fn follow(&mut self, target: Vec2, follow: &Follow, dt: f32, viewport: Vec2) {
        let goal = target + follow.offset;
        // Zona morta: só anda o bastante para o alvo voltar para dentro dela.
        let half = follow.deadzone.max(Vec2::ZERO) / 2.0;
        let mut desired = self.position;
        for axis in 0..2 {
            let d = goal[axis] - desired[axis];
            if d > half[axis] {
                desired[axis] = goal[axis] - half[axis];
            } else if d < -half[axis] {
                desired[axis] = goal[axis] + half[axis];
            }
        }
        let view = self.view_size(viewport);
        if let Some(b) = follow.bounds {
            desired = clamp_center(desired, b, view);
        }
        let t = if follow.smoothing <= 0.0 {
            1.0
        } else {
            1.0 - (-follow.smoothing * dt.max(0.0)).exp()
        };
        self.position = self.position.lerp(desired, t);
        if let Some(b) = follow.bounds {
            self.position = clamp_center(self.position, b, view);
        }
    }
}

/// Posição da câmera mais próxima de `p` que mantém a vista dentro de `b`.
fn clamp_center(p: Vec2, b: Bounds, view: Vec2) -> Vec2 {
    let mut out = p;
    for axis in 0..2 {
        let half = view[axis] / 2.0;
        let (lo, hi) = (b.min[axis] + half, b.max[axis] - half);
        out[axis] = if lo > hi {
            (b.min[axis] + b.max[axis]) / 2.0
        } else {
            p[axis].clamp(lo, hi)
        };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limites_seguram_a_vista_e_centralizam_o_que_e_menor() {
        let mut cam = Camera2D::new(Vec2::new(-1000.0, 5000.0));
        let mapa = Bounds::from_size(Vec2::ZERO, Vec2::new(2000.0, 400.0));
        cam.clamp_to(mapa, VIEW);
        // Horizontal: encosta na borda esquerda. Vertical: mapa menor que a tela.
        assert!(perto(cam.position, Vec2::new(400.0, 200.0)));
        let vista = cam.view_bounds(VIEW);
        assert!(perto(vista.min, Vec2::new(0.0, -100.0)));

        cam.zoom = 2.0; // vista de 400x300: agora cabe na altura
        cam.position = Vec2::new(1990.0, 390.0);
        cam.clamp_to(mapa, VIEW);
        assert!(perto(cam.position, Vec2::new(1800.0, 250.0)));
    }

    #[test]
    fn seguir_suave_nao_depende_da_taxa_de_quadros() {
        let seguir = Follow::default();
        let alvo = Vec2::new(300.0, 0.0);
        let mut a = Camera2D::default();
        let mut b = Camera2D::default();
        for _ in 0..30 {
            a.follow(alvo, &seguir, 1.0 / 60.0, VIEW);
        }
        for _ in 0..15 {
            b.follow(alvo, &seguir, 1.0 / 30.0, VIEW);
        }
        assert!((a.position.x - b.position.x).abs() < 0.01);
        // Em meio segundo, ~95% do caminho.
        assert!(
            a.position.x > 280.0 && a.position.x < 300.0,
            "{}",
            a.position.x
        );

        let mut direto = Camera2D::default();
        let sem_suavizar = Follow {
            smoothing: 0.0,
            offset: Vec2::new(10.0, 20.0),
            ..Default::default()
        };
        direto.follow(alvo, &sem_suavizar, 1.0 / 60.0, VIEW);
        assert!(perto(direto.position, Vec2::new(310.0, 20.0)));
    }

    #[test]
    fn zona_morta_deixa_o_alvo_andar_livre() {
        let seguir = Follow {
            smoothing: 0.0,
            deadzone: Vec2::new(100.0, 200.0),
            ..Default::default()
        };
        let mut cam = Camera2D::default();
        cam.follow(Vec2::new(40.0, -90.0), &seguir, 0.016, VIEW);
        assert_eq!(cam.position, Vec2::ZERO, "dentro da zona morta");
        cam.follow(Vec2::new(80.0, 150.0), &seguir, 0.016, VIEW);
        // Só o necessário para o alvo ficar na borda da zona.
        assert!(perto(cam.position, Vec2::new(30.0, 50.0)));
    }

    #[test]
    fn seguir_respeita_os_limites() {
        let seguir = Follow {
            smoothing: 0.0,
            bounds: Some(Bounds::from_size(Vec2::ZERO, Vec2::new(1000.0, 1000.0))),
            ..Default::default()
        };
        let mut cam = Camera2D::default();
        cam.follow(Vec2::new(-500.0, 2000.0), &seguir, 0.016, VIEW);
        assert!(perto(cam.position, Vec2::new(400.0, 700.0)));
        assert!(Bounds::new(Vec2::ONE, Vec2::ZERO).contains(Vec2::splat(0.5)));
    }

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
