//! Painel Hierarquia: os objetos da cena em árvore. Arrastar muda a ordem
//! (e de quem o objeto é filho); o olho esconde, o cadeado trava.

use eframe::egui::{self, Color32, Key, Modifiers, Pos2, Rect, RichText, Sense, Stroke, Vec2};

use crate::scene_data::Object;
use crate::scene_view::{ObjectDrag, SceneDoc, add_object_menu};

/// Onde um objeto arrastado vai parar em relação à linha de destino.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropZone {
    Before,
    Inside,
    After,
}

/// Zona pela altura do ponteiro dentro da linha.
pub fn zone(row: Rect, y: f32) -> DropZone {
    let t = (y - row.top()) / row.height().max(1.0);
    if t < 0.25 {
        DropZone::Before
    } else if t > 0.75 {
        DropZone::After
    } else {
        DropZone::Inside
    }
}

/// O que a Hierarquia pede ao espaço de trabalho.
#[derive(Debug, Clone, PartialEq)]
pub enum HierarchyAction {
    /// Transformar o objeto num prefab (na pasta dada, ou em `prefabs/`).
    CreatePrefab {
        id: u32,
        dir: Option<String>,
    },
    OpenFile(String),
    /// Criar um mapa de tiles (abre a janela).
    NewMap,
}

/// Aplica o soltar: devolve `false` se não deu (ex.: soltar dentro do filho).
pub fn apply_drop(doc: &mut SceneDoc, dragged: u32, target: u32, zone: DropZone) -> bool {
    if dragged == target || doc.prefab_root() == Some(dragged) {
        return false;
    }
    // Num prefab, tudo fica dentro do objeto principal.
    let zone = if doc.prefab_root() == Some(target) {
        DropZone::Inside
    } else {
        zone
    };
    let Some(parent) = doc.data.parent_of(target) else {
        return false;
    };
    let siblings_index = |doc: &SceneDoc| -> usize {
        let list = match parent {
            None => &doc.data.objects,
            Some(p) => match doc.data.find(p) {
                Some(o) => &o.children,
                None => return 0,
            },
        };
        list.iter().position(|o| o.id == target).unwrap_or(0)
    };
    let (new_parent, index) = match zone {
        DropZone::Before => (parent, siblings_index(doc)),
        DropZone::After => (parent, siblings_index(doc) + 1),
        DropZone::Inside => (
            Some(target),
            doc.data.find(target).map(|o| o.children.len()).unwrap_or(0),
        ),
    };
    if new_parent == Some(dragged) || new_parent.is_some_and(|p| doc.data.is_ancestor(dragged, p)) {
        return false;
    }
    doc.record(None);
    let ok = doc.data.move_object(dragged, new_parent, index);
    if ok && let Some(p) = new_parent {
        doc.collapsed.remove(&p);
    }
    ok
}

#[derive(Debug, Default)]
pub struct HierarchyPanel {
    pub filter: String,
}

impl HierarchyPanel {
    pub fn ui(&mut self, ui: &mut egui::Ui, doc: Option<&mut SceneDoc>) -> Vec<HierarchyAction> {
        let mut out = Vec::new();
        crate::tutorial::anchor(ui.ctx(), "hierarquia", ui.max_rect());
        let Some(doc) = doc else {
            ui.add_space(8.0);
            ui.weak("Abra uma cena visual (.cena) para ver os objetos dela aqui.");
            return out;
        };
        if doc.error.is_some() {
            ui.weak("A cena não abriu (veja a aba dela).");
            return out;
        }
        ui.horizontal(|ui| {
            ui.menu_button("➕", |ui| {
                if let Some(o) = add_object_menu(ui) {
                    let (x, y) = doc.view_center();
                    doc.add_prepared(o, x, y);
                    ui.close();
                }
                if ui.button("Mapa de tiles...").clicked() {
                    out.push(HierarchyAction::NewMap);
                    ui.close();
                }
            })
            .response
            .on_hover_text("Adicionar objeto");
            ui.add(
                egui::TextEdit::singleline(&mut self.filter)
                    .hint_text("🔍 Filtrar")
                    .desired_width(ui.available_width()),
            );
        });
        ui.separator();

        let mut drop: Option<(u32, u32, DropZone)> = None;
        let scroll = egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                if doc.data.objects.is_empty() {
                    ui.weak("A cena está vazia. Use ➕ ou arraste imagens do painel Arquivos para a cena.");
                }
                if self.filter.trim().is_empty() {
                    let objects = doc.data.objects.clone();
                    rows(ui, doc, &objects, 0, &mut drop, &mut out);
                } else {
                    let mut found = Vec::new();
                    crate::scene_data::visit(&doc.data.objects, &mut |o| {
                        if crate::commands::fuzzy_score(&self.filter, &o.name).is_some() {
                            found.push(o.clone());
                        }
                    });
                    for o in found {
                        row(ui, doc, &o, 0, &mut drop, &mut out);
                    }
                }
                // Espaço livre no fim: soltar aqui leva para a raiz.
                let (rect, resp) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), ui.available_height().max(40.0)),
                    Sense::click(),
                );
                if resp.dnd_hover_payload::<ObjectDrag>().is_some() {
                    ui.painter().line_segment(
                        [rect.left_top(), rect.right_top()],
                        Stroke::new(2.0, ui.visuals().selection.bg_fill),
                    );
                }
                if let Some(payload) = resp.dnd_release_payload::<ObjectDrag>() {
                    let id = payload.0;
                    match doc.prefab_root() {
                        // Num prefab, o fim da lista é o fim do objeto principal.
                        Some(root) if root != id => {
                            doc.record(None);
                            let n = doc.data.find(root).map(|r| r.children.len()).unwrap_or(0);
                            doc.data.move_object(id, Some(root), n);
                        }
                        Some(_) => {}
                        None => {
                            doc.record(None);
                            let n = doc.data.objects.len();
                            doc.data.move_object(id, None, n);
                        }
                    }
                }
                if resp.clicked() {
                    doc.selection.clear();
                }
            });
        if let Some((dragged, target, z)) = drop {
            apply_drop(doc, dragged, target, z);
        }

        // Atalhos com o mouse sobre o painel.
        if ui.rect_contains_pointer(scroll.inner_rect) && !ui.ctx().text_edit_focused() {
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F2))
                && let Some(&id) = doc.selection.last()
                && let Some(o) = doc.data.find(id)
            {
                doc.renaming = Some((id, o.name.clone(), true));
            }
            if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Delete)) {
                doc.delete_selection();
            }
            if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::D)) {
                doc.duplicate_selection();
            }
        }
        out
    }
}

fn rows(
    ui: &mut egui::Ui,
    doc: &mut SceneDoc,
    list: &[Object],
    depth: usize,
    drop: &mut Option<(u32, u32, DropZone)>,
    out: &mut Vec<HierarchyAction>,
) {
    for o in list {
        row(ui, doc, o, depth, drop, out);
        if !o.children.is_empty() && !doc.collapsed.contains(&o.id) {
            rows(ui, doc, &o.children, depth + 1, drop, out);
        }
    }
}

fn row(
    ui: &mut egui::Ui,
    doc: &mut SceneDoc,
    o: &Object,
    depth: usize,
    drop: &mut Option<(u32, u32, DropZone)>,
    out: &mut Vec<HierarchyAction>,
) {
    let id = o.id;
    let selected = doc.selection.contains(&id);
    let row = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        ui.add_space(depth as f32 * 14.0);
        if o.children.is_empty() {
            ui.add_space(14.0);
        } else {
            let open = !doc.collapsed.contains(&id);
            if ui
                .add(
                    egui::Button::new(if open { "⏷" } else { "⏵" })
                        .frame(false)
                        .small(),
                )
                .clicked()
            {
                if open {
                    doc.collapsed.insert(id);
                } else {
                    doc.collapsed.remove(&id);
                }
            }
        }
        // Olho (visível) e cadeado (travado).
        if toggle_icon(ui, o.visible, Icon::Eye)
            .on_hover_text(if o.visible { "Esconder" } else { "Mostrar" })
            .clicked()
        {
            doc.record(None);
            if let Some(obj) = doc.data.find_mut(id) {
                obj.visible = !obj.visible;
            }
        }
        if toggle_icon(ui, o.locked, Icon::Lock)
            .on_hover_text(if o.locked {
                "Destravar"
            } else {
                "Travar (não seleciona clicando na cena)"
            })
            .clicked()
        {
            doc.record(None);
            if let Some(obj) = doc.data.find_mut(id) {
                obj.locked = !obj.locked;
            }
        }

        if let Some((rid, name, focus)) = &mut doc.renaming
            && *rid == id
        {
            let r =
                ui.add(egui::TextEdit::singleline(name).desired_width(ui.available_width() - 4.0));
            if *focus {
                *focus = false;
                r.request_focus();
            }
            let esc = ui.input(|i| i.key_pressed(Key::Escape));
            if esc {
                doc.renaming = None;
            } else if r.lost_focus() {
                let (rid, name, _) = doc.renaming.take().expect("renomeando");
                let name = name.trim().to_string();
                if !name.is_empty() && doc.data.find(rid).is_some_and(|o| o.name != name) {
                    doc.record(None);
                    if let Some(obj) = doc.data.find_mut(rid) {
                        obj.name = name;
                    }
                }
            }
            return None;
        }

        let mut text = RichText::new(&o.name);
        if o.prefab.is_some() {
            text = text.color(crate::inspector::PREFAB);
        }
        if !o.visible {
            text = text.weak();
        }
        let r = ui.add(
            egui::Button::selectable(selected, (text, egui::Atom::grow()))
                .min_size(Vec2::new(ui.available_width(), 0.0))
                .sense(Sense::click_and_drag()),
        );
        crate::tutorial::anchor_first(ui, &format!("hierarquia:{}", o.name), r.rect);
        Some(r)
    });
    let Some(label) = row.inner else { return };
    let label = match &o.prefab {
        Some(link) => label.on_hover_text(format!("Instância do prefab {}", link.path)),
        None => label,
    };

    let modifiers = ui.input(|i| i.modifiers);
    if label.clicked() {
        if modifiers.shift || modifiers.command {
            if let Some(i) = doc.selection.iter().position(|s| *s == id) {
                doc.selection.remove(i);
            } else {
                doc.selection.push(id);
            }
        } else {
            doc.selection = vec![id];
        }
    }
    if label.double_clicked() {
        doc.renaming = Some((id, o.name.clone(), true));
    }
    label.context_menu(|ui| {
        if !doc.selection.contains(&id) {
            doc.selection = vec![id];
        }
        if ui.button("✏ Renomear (F2)").clicked() {
            doc.renaming = Some((id, o.name.clone(), true));
            ui.close();
        }
        if ui.button("Duplicar (Ctrl+D)").clicked() {
            doc.duplicate_selection();
            ui.close();
        }
        if let Some(link) = &o.prefab {
            if ui.button("Abrir prefab").clicked() {
                out.push(HierarchyAction::OpenFile(link.path.clone()));
                ui.close();
            }
        } else if doc.prefab_root() != Some(id)
            && ui
                .button("📦 Criar prefab")
                .on_hover_text("Guarda o objeto (com os filhos) em prefabs/, para usar em outras cenas. Também dá para arrastar o objeto para uma pasta do painel Arquivos.")
                .clicked()
        {
            out.push(HierarchyAction::CreatePrefab { id, dir: None });
            ui.close();
        }
        if ui.button("Criar filho vazio").clicked() {
            doc.record(None);
            let new = doc.data.next_id();
            let n = doc.data.find(id).map(|o| o.children.len()).unwrap_or(0);
            doc.data.insert(Some(id), n, Object::new(new, "filho"));
            doc.collapsed.remove(&id);
            doc.selection = vec![new];
            ui.close();
        }
        if doc.data.parent_of(id).flatten().is_some() && ui.button("Tirar do pai").clicked() {
            doc.record(None);
            let n = doc.data.objects.len();
            doc.data.move_object(id, None, n);
            ui.close();
        }
        ui.separator();
        if ui.button("🗑 Excluir (Del)").clicked() {
            doc.delete_selection();
            ui.close();
        }
    });

    // Arrastar para reorganizar.
    let full = Rect::from_min_max(
        Pos2::new(row.response.rect.left(), label.rect.top()),
        Pos2::new(row.response.rect.right(), label.rect.bottom()),
    );
    label.dnd_set_drag_payload(ObjectDrag(id));
    if let (Some(payload), Some(p)) = (
        label.dnd_hover_payload::<ObjectDrag>(),
        ui.input(|i| i.pointer.latest_pos()),
    ) && payload.0 != id
    {
        let color = ui.visuals().selection.bg_fill;
        match zone(full, p.y) {
            DropZone::Before => {
                ui.painter()
                    .line_segment([full.left_top(), full.right_top()], Stroke::new(2.0, color));
            }
            DropZone::After => {
                ui.painter().line_segment(
                    [full.left_bottom(), full.right_bottom()],
                    Stroke::new(2.0, color),
                );
            }
            DropZone::Inside => {
                ui.painter().rect_stroke(
                    full,
                    3.0,
                    Stroke::new(1.5, color),
                    egui::StrokeKind::Inside,
                );
            }
        }
    }
    if let (Some(payload), Some(p)) = (
        label.dnd_release_payload::<ObjectDrag>(),
        ui.input(|i| i.pointer.latest_pos()),
    ) {
        *drop = Some((payload.0, id, zone(full, p.y)));
    }
}

#[derive(Clone, Copy)]
enum Icon {
    Eye,
    Lock,
}

/// Botãozinho desenhado (olho ou cadeado).
fn toggle_icon(ui: &mut egui::Ui, on: bool, icon: Icon) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::click());
    let strong = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color().gamma_multiply(0.6);
    let p = ui.painter();
    let c = rect.center();
    match icon {
        Icon::Eye => {
            let color = if on { strong } else { weak };
            p.add(egui::Shape::ellipse_stroke(
                c,
                Vec2::new(6.5, 4.0),
                Stroke::new(1.2, color),
            ));
            if on {
                p.circle_filled(c, 2.0, color);
            } else {
                p.line_segment(
                    [c + Vec2::new(-6.0, 4.0), c + Vec2::new(6.0, -4.0)],
                    Stroke::new(1.2, color),
                );
            }
        }
        Icon::Lock => {
            let color = if on {
                Color32::from_rgb(0xE8, 0xB3, 0x39)
            } else {
                weak
            };
            let body = Rect::from_center_size(c + Vec2::new(0.0, 2.0), Vec2::new(9.0, 7.0));
            p.rect_filled(body, 1.5, color);
            let top = body.top();
            p.line_segment(
                [Pos2::new(c.x - 3.0, top), Pos2::new(c.x - 3.0, top - 3.0)],
                Stroke::new(1.3, color),
            );
            p.line_segment(
                [
                    Pos2::new(c.x + 3.0, top),
                    Pos2::new(c.x + 3.0, top - if on { 3.0 } else { 1.5 }),
                ],
                Stroke::new(1.3, color),
            );
            p.line_segment(
                [
                    Pos2::new(c.x - 3.0, top - 3.0),
                    Pos2::new(c.x + 3.0, top - 3.0),
                ],
                Stroke::new(1.3, color),
            );
        }
    }
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> SceneDoc {
        let dir = std::env::temp_dir();
        let mut d = SceneDoc::open(&dir, "nao-existe-hierarquia.cena");
        d.error = None;
        let mut a = Object::new(1, "a");
        a.children.push(Object::new(2, "a1"));
        d.data.objects = vec![a, Object::new(3, "b"), Object::new(4, "c")];
        d
    }

    #[test]
    fn zonas() {
        let r = Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 20.0));
        assert_eq!(zone(r, 2.0), DropZone::Before);
        assert_eq!(zone(r, 10.0), DropZone::Inside);
        assert_eq!(zone(r, 19.0), DropZone::After);
    }

    #[test]
    fn soltar_reorganiza() {
        let mut d = doc();
        // c antes de a.
        assert!(apply_drop(&mut d, 4, 1, DropZone::Before));
        assert_eq!(
            d.data.objects.iter().map(|o| o.id).collect::<Vec<_>>(),
            [4, 1, 3]
        );
        // b depois de c (mesma lista, para trás).
        assert!(apply_drop(&mut d, 3, 4, DropZone::After));
        assert_eq!(
            d.data.objects.iter().map(|o| o.id).collect::<Vec<_>>(),
            [4, 3, 1]
        );
        // c vira filho de a.
        assert!(apply_drop(&mut d, 4, 1, DropZone::Inside));
        assert_eq!(d.data.parent_of(4), Some(Some(1)));
        // a não pode ir para dentro do próprio filho.
        assert!(!apply_drop(&mut d, 1, 2, DropZone::Inside));
        assert!(!apply_drop(&mut d, 1, 1, DropZone::After));
        // Tudo dá para desfazer.
        assert!(d.undo());
        assert_eq!(d.data.parent_of(4), Some(None));
    }
}
