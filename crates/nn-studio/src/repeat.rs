//! Repetir em linha e em círculo: cercas, colunas, raios de roda, pétalas,
//! fileiras de árvores, pedras em volta da fogueira. O modelador repete
//! peças; a cena, objetos. As duas usam a mesma conta e a mesma janelinha.

use eframe::egui;
use glam::{Mat4, Quat, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatKind {
    Line,
    Circle,
}

/// Como repetir.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RepeatSpec {
    pub kind: RepeatKind,
    /// Quantas cópias novas (o original fica).
    pub count: u32,
    /// Em linha: a distância entre uma e outra.
    pub spacing: f32,
    /// Em linha: a direção (0 = X, 1 = Y, 2 = Z). Em círculo: o eixo do giro.
    pub axis: usize,
    /// Em linha: para o lado negativo do eixo.
    pub backwards: bool,
    /// Em círculo: o raio (o original fica na borda).
    pub radius: f32,
    /// Em círculo: quanto da volta (360 = a volta toda).
    pub arc: f32,
    /// Em círculo: cada cópia gira junto (virada para fora, como o original).
    pub turn: bool,
    /// Em círculo: o meio é o 0 do modelo (no eixo), e não ao lado do
    /// original. Pedras em volta de uma fogueira que está no meio.
    pub around_zero: bool,
}

impl RepeatSpec {
    /// Em linha, `count` cópias a cada `spacing`, na direção X.
    pub fn line(count: u32, spacing: f32) -> Self {
        Self {
            kind: RepeatKind::Line,
            count,
            spacing,
            axis: 0,
            backwards: false,
            radius: spacing.max(0.1) * 2.0,
            arc: 360.0,
            turn: true,
            around_zero: false,
        }
    }

    /// Em círculo, `count` cópias em volta, com raio `radius`.
    pub fn circle(count: u32, radius: f32, axis: usize) -> Self {
        Self {
            kind: RepeatKind::Circle,
            count,
            spacing: radius.max(0.1) / 2.0,
            axis,
            backwards: false,
            radius,
            arc: 360.0,
            turn: true,
            around_zero: false,
        }
    }

    fn axis_vec(&self) -> Vec3 {
        [Vec3::X, Vec3::Y, Vec3::Z][self.axis.min(2)]
    }

    /// A direção do original para o meio do círculo (perpendicular ao eixo).
    fn inward(&self) -> Vec3 {
        match self.axis.min(2) {
            1 => Vec3::NEG_X,
            2 => Vec3::NEG_X,
            _ => Vec3::NEG_Z,
        }
    }

    /// O meio do círculo, com o original (ou o meio do grupo) em `origin`.
    pub fn center(&self, origin: Vec3) -> Vec3 {
        if self.around_zero {
            let a = self.axis_vec();
            return a * origin.dot(a);
        }
        origin + self.inward() * self.radius
    }

    /// O ângulo (graus) da cópia `i` (1..=count) em volta do círculo.
    pub fn angle(&self, i: u32) -> f32 {
        let arc = self.arc.clamp(1.0, 360.0);
        let step = if arc >= 359.9 {
            arc / (self.count + 1) as f32
        } else {
            arc / self.count.max(1) as f32
        };
        step * i as f32
    }

    /// O movimento (no mundo) que leva o original até a cópia `i`
    /// (1..=count). `origin` é onde está o original (ou o meio do grupo).
    pub fn transform(&self, i: u32, origin: Vec3) -> Mat4 {
        match self.kind {
            RepeatKind::Line => {
                let sign = if self.backwards { -1.0 } else { 1.0 };
                Mat4::from_translation(self.axis_vec() * self.spacing * sign * i as f32)
            }
            RepeatKind::Circle => {
                let c = self.center(origin);
                let q = Quat::from_axis_angle(self.axis_vec(), self.angle(i).to_radians());
                if self.turn {
                    Mat4::from_translation(c) * Mat4::from_quat(q) * Mat4::from_translation(-c)
                } else {
                    Mat4::from_translation(q * (origin - c) + c - origin)
                }
            }
        }
    }

    /// No 2D: onde vai um ponto e quanto ele gira (graus), na cópia `i`.
    /// O círculo do 2D gira em volta de Z, e a linha anda em X ou Y.
    pub fn transform2(&self, i: u32, origin: (f64, f64), p: (f64, f64)) -> ((f64, f64), f64) {
        let mut s = *self;
        if s.kind == RepeatKind::Circle {
            s.axis = 2;
        } else {
            s.axis = s.axis.min(1);
        }
        let o = Vec3::new(origin.0 as f32, origin.1 as f32, 0.0);
        let m = s.transform(i, o);
        let q = m.transform_point3(Vec3::new(p.0 as f32, p.1 as f32, 0.0));
        let turn = if s.kind == RepeatKind::Circle && s.turn {
            f64::from(s.angle(i))
        } else {
            0.0
        };
        ((f64::from(q.x), f64::from(q.y)), turn)
    }
}

/// A janelinha das opções. `three_d` mostra o eixo Z; `unit` é "m" ou "px".
/// Devolve se algo mudou.
/// Com `zero`, o círculo pode ser em volta do 0 (do modelo).
pub fn spec_ui(
    ui: &mut egui::Ui,
    spec: &mut RepeatSpec,
    three_d: bool,
    unit: &str,
    zero: bool,
) -> bool {
    let before = *spec;
    let step = if unit == "px" { 1.0 } else { 0.05 };
    ui.horizontal(|ui| {
        let r = ui.selectable_value(&mut spec.kind, RepeatKind::Line, "↔ Em linha");
        crate::tutorial::anchor(ui.ctx(), "repetir.linha", r.rect);
        let r = ui.selectable_value(&mut spec.kind, RepeatKind::Circle, "🔄 Em círculo");
        crate::tutorial::anchor(ui.ctx(), "repetir.circulo", r.rect);
    });
    // Trocar o jeito volta o eixo ao de costume: a linha anda em X, e o
    // círculo fica deitado no chão (em volta de Y).
    if spec.kind != before.kind {
        spec.axis = match spec.kind {
            RepeatKind::Line => 0,
            RepeatKind::Circle => 1,
        };
    }
    egui::Grid::new("repetir-opcoes")
        .num_columns(2)
        .spacing(egui::vec2(8.0, 4.0))
        .show(ui, |ui| {
            ui.label("Cópias");
            ui.add(egui::DragValue::new(&mut spec.count).range(1..=200).speed(0.2));
            ui.end_row();
            let axes: &[(usize, &str)] = if three_d {
                &[(0, "X"), (1, "Y"), (2, "Z")]
            } else {
                &[(0, "X"), (1, "Y")]
            };
            match spec.kind {
                RepeatKind::Line => {
                    ui.label("Distância");
                    ui.add(
                        egui::DragValue::new(&mut spec.spacing)
                            .speed(step)
                            .range(0.0..=100_000.0)
                            .max_decimals(2)
                            .suffix(format!(" {unit}")),
                    );
                    ui.end_row();
                    ui.label("Direção");
                    ui.horizontal(|ui| {
                        if !three_d {
                            spec.axis = spec.axis.min(1);
                        }
                        for &(a, name) in axes {
                            ui.selectable_value(&mut spec.axis, a, name);
                        }
                        ui.checkbox(&mut spec.backwards, "para trás")
                            .on_hover_text("Para o lado negativo do eixo");
                    });
                    ui.end_row();
                }
                RepeatKind::Circle => {
                    if zero {
                        ui.label("Meio");
                        let r = ui
                            .checkbox(&mut spec.around_zero, "Em volta do meio do modelo")
                            .on_hover_text("O círculo em volta do 0 do modelo (onde está a fogueira, a mesa...). Desligado, o meio fica ao lado, à distância do raio.");
                        crate::tutorial::anchor(ui.ctx(), "repetir.meio", r.rect);
                        ui.end_row();
                    }
                    ui.label("Raio");
                    ui.add_enabled(
                        !(zero && spec.around_zero),
                        egui::DragValue::new(&mut spec.radius)
                            .speed(step)
                            .range(0.0..=100_000.0)
                            .max_decimals(2)
                            .suffix(format!(" {unit}")),
                    );
                    ui.end_row();
                    if three_d {
                        ui.label("Eixo");
                        ui.horizontal(|ui| {
                            for &(a, name) in axes {
                                ui.selectable_value(&mut spec.axis, a, name);
                            }
                        })
                        .response
                        .on_hover_text("Em volta de Y fica deitado no chão (pedras em volta da fogueira); X e Z ficam em pé (raios de roda).");
                        ui.end_row();
                    }
                    ui.label("Volta");
                    ui.add(
                        egui::Slider::new(&mut spec.arc, 15.0..=360.0)
                            .step_by(15.0)
                            .suffix("°"),
                    )
                    .on_hover_text("360° é a volta toda; menos faz um arco");
                    ui.end_row();
                    ui.label("");
                    ui.checkbox(&mut spec.turn, "Girar junto")
                        .on_hover_text("Cada cópia gira com o círculo (como pétalas). Desligado, todas ficam viradas para o mesmo lado.");
                    ui.end_row();
                }
            }
        });
    *spec != before
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linha_anda_a_distancia() {
        let s = RepeatSpec::line(3, 2.0);
        let p = s.transform(3, Vec3::ZERO).transform_point3(Vec3::ZERO);
        assert!((p - Vec3::new(6.0, 0.0, 0.0)).length() < 1e-5);
        let s = RepeatSpec {
            backwards: true,
            axis: 2,
            ..s
        };
        let p = s.transform(1, Vec3::ZERO).transform_point3(Vec3::ONE);
        assert!((p - Vec3::new(1.0, 1.0, -1.0)).length() < 1e-5);
    }

    #[test]
    fn circulo_divide_a_volta_e_volta_ao_comeco() {
        // 5 cópias + o original: de 60 em 60 graus, todos no mesmo raio.
        let s = RepeatSpec::circle(5, 2.0, 1);
        let o = Vec3::new(1.0, 0.0, 0.0);
        let c = s.center(o);
        for i in 1..=5 {
            let p = s.transform(i, o).transform_point3(o);
            assert!(((p - c).length() - 2.0).abs() < 1e-4);
            assert!((p.y).abs() < 1e-5, "deitado no chão");
        }
        assert!((s.angle(1) - 60.0).abs() < 1e-4);
        // Um arco de 90 graus com 3 cópias: 30, 60, 90.
        let a = RepeatSpec {
            arc: 90.0,
            ..RepeatSpec::circle(3, 1.0, 1)
        };
        assert!((a.angle(3) - 90.0).abs() < 1e-4);
    }

    #[test]
    fn circulo_em_volta_do_zero() {
        let s = RepeatSpec {
            around_zero: true,
            ..RepeatSpec::circle(3, 9.0, 1)
        };
        let o = Vec3::new(1.5, 0.2, 0.0);
        let p = s.transform(2, o).transform_point3(o);
        assert!((p - Vec3::new(-1.5, 0.2, 0.0)).length() < 1e-4, "{p}");
    }

    #[test]
    fn circulo_sem_girar_so_anda() {
        let s = RepeatSpec {
            turn: false,
            ..RepeatSpec::circle(3, 1.0, 1)
        };
        let m = s.transform(2, Vec3::ZERO);
        let (_, q, _) = m.to_scale_rotation_translation();
        assert!(q.angle_between(Quat::IDENTITY) < 1e-4);
    }

    #[test]
    fn no_2d_gira_em_volta_de_z() {
        let s = RepeatSpec::circle(3, 100.0, 1);
        let ((x, y), turn) = s.transform2(2, (0.0, 0.0), (0.0, 0.0));
        // Meio em (-100, 0): a cópia 2 (180°) vai para (-200, 0).
        assert!((x + 200.0).abs() < 1e-3 && y.abs() < 1e-3, "{x} {y}");
        assert!((turn - 180.0).abs() < 1e-3);
        let l = RepeatSpec {
            axis: 1,
            ..RepeatSpec::line(2, 32.0)
        };
        let ((x, y), turn) = l.transform2(2, (0.0, 0.0), (5.0, 5.0));
        assert_eq!((x, y, turn), (5.0, 69.0, 0.0));
    }
}
