//! Painel Eventos: as regras "Quando... Então..." da cena aberta e dos
//! objetos dela, uma por cartão, lidas como frases. Os campos ficam no meio
//! da frase; as condições e ações vêm do catálogo da engine.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, RichText, Stroke};

use crate::behaviors::{PropSpec, PropType};
use crate::events::{self, Catalog, EventDef, Piece};
use crate::files::{FileKind, FileTree};
use crate::inspector::{GAME, PREFAB};
use crate::scene_data::{self, EventItem, Prop, Rule, format_num};
use crate::scene_view::{FileDrag, SceneDoc};
use crate::theme;

/// Quanto tempo uma regra fica acesa depois de disparar no jogo.
const FLASH: Duration = Duration::from_millis(900);

/// O que o painel pede ao espaço de trabalho.
#[derive(Debug, Clone, PartialEq)]
pub enum EventsAction {
    OpenFile(String),
}

/// O que o painel precisa do projeto.
pub struct EventsEnv<'a> {
    pub files: &'a FileTree,
    /// Variáveis do jogo (config.lua).
    pub vars: &'a [String],
    /// Nomes das animações do projeto (arquivos `.anim`).
    pub anims: &'a [String],
    /// Tipos de tile dos tilesets dos mapas da cena ("lava", "agua"...).
    pub tile_kinds: &'a [String],
    /// O tileset do mapa da cena, para escolher tiles.
    pub tile_sheet: Option<&'a TileSheet>,
}

/// A imagem de um tileset para escolher um tile.
#[derive(Clone)]
pub struct TileSheet {
    pub texture: egui::TextureHandle,
    pub size: [u32; 2],
    pub data: crate::tileset::TilesetData,
    pub first_gid: u32,
}

impl std::fmt::Debug for TileSheet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TileSheet")
            .field("image", &self.data.image)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for EventsEnv<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventsEnv").finish_non_exhaustive()
    }
}

#[derive(Debug, Default)]
pub struct EventsPanel {
    /// Regras que dispararam no jogo: cena → (chave, quando).
    flashes: HashMap<String, HashMap<String, Instant>>,
    /// "Ver como código": título e código.
    code: Option<(String, String)>,
    /// Busca dos menus de condições e ações.
    search: String,
    /// Mostra só as regras do objeto selecionado (e as da cena).
    pub only_selected: bool,
}

/// De quem são as regras.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Owner {
    Scene,
    Object(u32),
}

impl Owner {
    /// Prefixo da chave que o jogo manda quando a regra dispara.
    fn key(self) -> String {
        match self {
            Owner::Scene => "c".into(),
            Owner::Object(id) => id.to_string(),
        }
    }
}

/// Mudança feita num quadro: `Some(chave)` junta com as próximas (digitar,
/// arrastar um número); `None` é um passo próprio de desfazer.
#[derive(Debug, Default)]
struct Edit {
    changed: bool,
    merge: Option<String>,
    structural: bool,
}

impl Edit {
    fn field(&mut self, key: String) {
        self.changed = true;
        if !self.structural {
            self.merge = Some(key);
        }
    }

    fn structure(&mut self) {
        self.changed = true;
        self.structural = true;
        self.merge = None;
    }
}

/// Contexto para desenhar as regras de um dono.
struct Ctx<'a> {
    owner: Owner,
    in_object: bool,
    read_only: bool,
    /// Nomes dos objetos da cena (para listas e para avisar de nomes que não
    /// existem). Vazio num prefab: lá os nomes são de outras cenas.
    names: &'a [String],
    check_names: bool,
    env: &'a EventsEnv<'a>,
}

impl EventsPanel {
    /// O jogo contou que estas regras dispararam.
    pub fn flash(&mut self, scene: &str, keys: &[String]) {
        if keys.is_empty() {
            return;
        }
        let now = Instant::now();
        let map = self.flashes.entry(scene.to_string()).or_default();
        for k in keys {
            map.insert(k.clone(), now);
        }
    }

    fn lit(&self, scene: &str, key: &str) -> Option<f32> {
        let t = self.flashes.get(scene)?.get(key)?;
        let e = t.elapsed();
        (e < FLASH).then(|| 1.0 - e.as_secs_f32() / FLASH.as_secs_f32())
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        doc: Option<&mut SceneDoc>,
        env: &EventsEnv,
    ) -> Vec<EventsAction> {
        let mut actions = Vec::new();
        let Some(doc) = doc else {
            ui.add_space(8.0);
            ui.weak(
                "Abra uma cena visual para ver e criar os eventos dela: regras \
                 \"Quando... Então...\", sem código.",
            );
            return actions;
        };
        if doc.error.is_some() {
            ui.weak("A cena tem um erro e não pode ser editada aqui.");
            return actions;
        }
        crate::tutorial::anchor(ui.ctx(), "eventos", ui.max_rect());
        events::set_three_d(doc.data.three_d);
        let cat = events::catalog();
        let is_prefab = doc.is_prefab();
        let mut names: Vec<String> = Vec::new();
        scene_data::visit(&doc.data.objects, &mut |o| {
            if !names.contains(&o.name) {
                names.push(o.name.clone());
            }
        });
        let selected = (doc.selection.len() == 1)
            .then(|| doc.selection[0])
            .and_then(|id| doc.data.find(id))
            .map(|o| (o.id, o.name.clone(), o.prefab.is_some()));

        // Barra de cima.
        let mut new_rule: Option<(Owner, EventItem)> = None;
        ui.horizontal_wrapped(|ui| {
            if !is_prefab
                && let Some(d) = add_menu(ui, "➕ Regra da cena", &cat.conditions, &mut self.search)
            {
                new_rule = Some((Owner::Scene, d.new_item(false)));
            }
            if !is_prefab {
                let r = ui.min_rect();
                crate::tutorial::anchor(ui.ctx(), "eventos.regra_cena", r);
            }
            match &selected {
                Some((id, name, false)) => {
                    let label = format!("➕ Regra de {}", short(name, 18));
                    if let Some(d) = add_menu(ui, &label, &cat.conditions, &mut self.search) {
                        new_rule = Some((Owner::Object(*id), d.new_item(true)));
                    }
                }
                Some((_, _, true)) => {
                    ui.weak("(as regras de uma instância vêm do prefab)");
                }
                None => {}
            }
            ui.separator();
            ui.checkbox(&mut self.only_selected, "Só do selecionado")
                .on_hover_text("Mostra as regras da cena e as do objeto selecionado.");
            let code = ui
                .button("Ver como código")
                .on_hover_text("Mostra em Lua o que as regras fazem (só para ler e aprender).");
            crate::tutorial::anchor_in(ui, "eventos.codigo", code.rect);
            if code.clicked() {
                self.code = Some(code_for(doc, selected.as_ref().map(|s| s.0)));
            }
        });
        ui.separator();

        if let Some((owner, item)) = new_rule {
            doc.record(None);
            let rule = Rule {
                when: vec![item],
                ..Default::default()
            };
            match owner {
                Owner::Scene => doc.data.events.push(rule),
                Owner::Object(id) => {
                    if let Some(o) = doc.data.find_mut(id) {
                        o.events.push(rule);
                    }
                }
            }
        }

        // Quem tem regras (ou está selecionado).
        let mut owners: Vec<(Owner, String, bool)> = Vec::new();
        if !is_prefab {
            owners.push((Owner::Scene, "Cena".into(), false));
        }
        scene_data::visit(&doc.data.objects, &mut |o| {
            let is_sel = selected.as_ref().is_some_and(|s| s.0 == o.id);
            if self.only_selected && !is_sel {
                return;
            }
            if !o.events.is_empty()
                || (is_sel && o.prefab.is_none())
                || (is_prefab && o.id == doc.data.objects[0].id)
            {
                owners.push((Owner::Object(o.id), o.name.clone(), o.prefab.is_some()));
            }
        });

        let total: usize = doc.data.events.len() + {
            let mut n = 0;
            scene_data::visit(&doc.data.objects, &mut |o| n += o.events.len());
            n
        };

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if total == 0 {
                    ui.add_space(4.0);
                    ui.weak(
                        "Nenhuma regra ainda. Use ➕ para criar uma, por exemplo: \
                         Quando o jogador encostar em chave, então variável chaves: somar 1 e \
                         destruir o outro.",
                    );
                    if !is_prefab {
                        ui.weak(
                            "Regras da cena valem para a cena toda; regras de um objeto \
                             vão junto com ele (e com as cópias, se ele for um prefab).",
                        );
                    }
                    ui.add_space(6.0);
                }
                for (owner, title, from_prefab) in owners {
                    let rules = match owner {
                        Owner::Scene => doc.data.events.clone(),
                        Owner::Object(id) => match doc.data.find(id) {
                            Some(o) => o.events.clone(),
                            None => continue,
                        },
                    };
                    if owner == Owner::Scene && rules.is_empty() && total > 0 {
                        continue;
                    }
                    ui.horizontal(|ui| {
                        let text = match owner {
                            Owner::Scene => RichText::new("Regras da cena").strong(),
                            Owner::Object(_) if from_prefab => {
                                RichText::new(format!("📦 {title}")).strong().color(PREFAB)
                            }
                            Owner::Object(_) => RichText::new(title.clone()).strong(),
                        };
                        let r = ui.add(egui::Label::new(text).sense(egui::Sense::click()));
                        if let Owner::Object(id) = owner
                            && r.on_hover_text("Clique para selecionar o objeto").clicked()
                        {
                            doc.selection = vec![id];
                        }
                        ui.weak(match rules.len() {
                            1 => "1 regra".to_string(),
                            n => format!("{n} regras"),
                        });
                        if from_prefab
                            && let Owner::Object(id) = owner
                            && let Some(path) = doc
                                .data
                                .find(id)
                                .and_then(|o| o.prefab.as_ref())
                                .map(|l| l.path.clone())
                            && ui.small_button("Abrir prefab").clicked()
                        {
                            actions.push(EventsAction::OpenFile(path));
                        }
                    });
                    let ctx = Ctx {
                        owner,
                        in_object: owner != Owner::Scene,
                        read_only: from_prefab,
                        names: &names,
                        check_names: !is_prefab,
                        env,
                    };
                    let mut list = rules.clone();
                    let mut edit = Edit::default();
                    self.rules_ui(ui, &mut list, &ctx, &doc.rel, cat, &mut edit);
                    if edit.changed && list != rules && !from_prefab {
                        doc.record(edit.merge.as_deref());
                        match owner {
                            Owner::Scene => doc.data.events = list,
                            Owner::Object(id) => {
                                if let Some(o) = doc.data.find_mut(id) {
                                    o.events = list;
                                }
                            }
                        }
                    }
                    ui.add_space(8.0);
                }
            });

        self.code_window(ui.ctx());
        actions
    }

    fn rules_ui(
        &mut self,
        ui: &mut egui::Ui,
        rules: &mut Vec<Rule>,
        ctx: &Ctx,
        scene: &str,
        cat: &Catalog,
        edit: &mut Edit,
    ) {
        let mut op: Option<(usize, RuleOp)> = None;
        let mut lit_any = false;
        for (i, rule) in rules.iter_mut().enumerate() {
            let flash = self.lit(scene, &format!("{}:{}", ctx.owner.key(), i + 1));
            lit_any |= flash.is_some();
            let id = egui::Id::new(("regra", scene, ctx.owner.key(), i));
            if let Some(o) = rule_card(ui, rule, i, ctx, cat, flash, id, &mut self.search, edit) {
                op = Some((i, o));
            }
            ui.add_space(4.0);
        }
        if lit_any {
            crate::theme::animate(ui.ctx(), Duration::from_millis(30));
        }
        if let Some((i, o)) = op {
            edit.structure();
            match o {
                RuleOp::Delete => {
                    rules.remove(i);
                }
                RuleOp::Duplicate => {
                    let r = rules[i].clone();
                    rules.insert(i + 1, r);
                }
                RuleOp::Up if i > 0 => rules.swap(i, i - 1),
                RuleOp::Down if i + 1 < rules.len() => rules.swap(i, i + 1),
                RuleOp::CommentAbove => rules.insert(
                    i,
                    Rule {
                        comment: Some("Comentário".into()),
                        ..Default::default()
                    },
                ),
                _ => {}
            }
        }
        if !ctx.read_only && !rules.is_empty() {
            ui.horizontal(|ui| {
                if let Some(d) = add_menu(ui, "➕ Regra", &cat.conditions, &mut self.search) {
                    edit.structure();
                    rules.push(Rule {
                        when: vec![d.new_item(ctx.in_object)],
                        ..Default::default()
                    });
                }
                if ui
                    .small_button("➕ Comentário")
                    .on_hover_text("Uma linha de texto para organizar as regras")
                    .clicked()
                {
                    edit.structure();
                    rules.push(Rule {
                        comment: Some("Comentário".into()),
                        ..Default::default()
                    });
                }
            });
        }
    }

    fn code_window(&mut self, ctx: &egui::Context) {
        let Some((title, code)) = &self.code else {
            return;
        };
        let mut open = true;
        egui::Window::new(format!("Como código: {title}"))
            .id(egui::Id::new("eventos-como-codigo"))
            .open(&mut open)
            .default_size([560.0, 420.0])
            .resizable(true)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.weak("Aproximado: mostra em Lua a ideia de cada regra. O jogo usa as regras, não este código.");
                });
                if ui.button("Copiar").clicked() {
                    ui.ctx().copy_text(code.clone());
                }
                ui.separator();
                egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                    let mut text = code.as_str();
                    ui.add(
                        egui::TextEdit::multiline(&mut text)
                            .code_editor()
                            .desired_width(f32::INFINITY),
                    );
                });
            });
        if !open {
            self.code = None;
        }
    }
}

/// O código das regras da cena, ou do objeto selecionado.
fn code_for(doc: &SceneDoc, selected: Option<u32>) -> (String, String) {
    if let Some(o) = selected.and_then(|id| doc.data.find(id))
        && (!o.events.is_empty() || doc.data.events.is_empty())
    {
        return (o.name.clone(), events::to_lua(&o.events, Some(&o.name)));
    }
    if doc.is_prefab()
        && let Some(o) = doc.data.objects.first()
    {
        return (o.name.clone(), events::to_lua(&o.events, Some(&o.name)));
    }
    let name = doc.rel.rsplit('/').next().unwrap_or(&doc.rel).to_string();
    (name, events::to_lua(&doc.data.events, None))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleOp {
    Delete,
    Duplicate,
    Up,
    Down,
    CommentAbove,
}

fn short(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max - 1).collect();
        t.push('…');
        t
    }
}

/// Um cartão de regra. Devolve uma operação na lista (excluir, subir...).
#[allow(clippy::too_many_arguments)]
fn rule_card(
    ui: &mut egui::Ui,
    rule: &mut Rule,
    index: usize,
    ctx: &Ctx,
    cat: &Catalog,
    flash: Option<f32>,
    id: egui::Id,
    search: &mut String,
    edit: &mut Edit,
) -> Option<RuleOp> {
    let mut op = None;
    let visuals = ui.visuals().clone();
    let mut fill = visuals.faint_bg_color;
    if let Some(t) = flash {
        fill = lerp_color(fill, GAME.gamma_multiply(0.45), t);
    }
    let stroke = if flash.is_some() {
        Stroke::new(1.5, GAME)
    } else {
        visuals.widgets.noninteractive.bg_stroke
    };
    let is_comment = rule.comment.is_some() && rule.when.is_empty() && rule.actions.is_empty();
    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add_enabled_ui(!ctx.read_only, |ui| {
                if is_comment {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("#").color(theme::PLAY).strong());
                        let text = rule.comment.get_or_insert_with(String::new);
                        let w = (ui.available_width() - 60.0).max(80.0);
                        let r = ui.add(
                            egui::TextEdit::singleline(text)
                                .desired_width(w)
                                .text_color(theme::PLAY),
                        );
                        if r.changed() {
                            edit.field(format!("{:?}:{index}:comentario", id));
                        }
                        op = rule_menu(ui, true);
                    });
                    return;
                }
                // Cabeça: ligada, Quando/Enquanto, menu.
                ui.horizontal(|ui| {
                    let mut on = !rule.off;
                    if ui
                        .checkbox(&mut on, "")
                        .on_hover_text("Ligada (desmarque para desligar a regra sem apagar)")
                        .changed()
                    {
                        rule.off = !on;
                        edit.structure();
                    }
                    let mode = if rule.repeat { "Enquanto" } else { "Quando" };
                    egui::ComboBox::from_id_salt(id.with("modo"))
                        .selected_text(RichText::new(mode).strong())
                        .width(96.0)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(!rule.repeat, "Quando")
                                .on_hover_text("Uma vez, quando tudo passa a valer (ou a cada vez que o gatilho acontece).")
                                .clicked()
                                && rule.repeat
                            {
                                rule.repeat = false;
                                edit.structure();
                            }
                            if ui
                                .selectable_label(rule.repeat, "Enquanto")
                                .on_hover_text("A cada quadro, enquanto tudo vale.")
                                .clicked()
                                && !rule.repeat
                            {
                                rule.repeat = true;
                                edit.structure();
                            }
                        });
                    if rule.off {
                        ui.weak("(desligada)");
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        op = rule_menu(ui, false);
                    });
                });
                // Condições.
                let mut remove_cond = None;
                let n = rule.when.len();
                for (j, item) in rule.when.iter_mut().enumerate() {
                    let prefix = if j == 0 { "" } else { "e" };
                    if item_line(ui, item, true, j, prefix, ctx, cat, id.with(("c", j)), edit) {
                        remove_cond = Some(j);
                    }
                }
                if let Some(j) = remove_cond {
                    rule.when.remove(j);
                    edit.structure();
                }
                if n == 0 {
                    ui.weak("   (sem condição: nunca acontece)");
                }
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    if let Some(d) = add_menu(ui, "➕ e…", &cat.conditions, search) {
                        rule.when.push(d.new_item(ctx.in_object));
                        edit.structure();
                    }
                });
                // Ações.
                ui.label(RichText::new("Então").strong());
                let mut remove_act = None;
                for (j, item) in rule.actions.iter_mut().enumerate() {
                    if item_line(ui, item, false, j, "•", ctx, cat, id.with(("a", j)), edit) {
                        remove_act = Some(j);
                    }
                }
                if let Some(j) = remove_act {
                    rule.actions.remove(j);
                    edit.structure();
                }
                ui.horizontal(|ui| {
                    ui.add_space(14.0);
                    if let Some(d) = add_menu(ui, "➕ ação", &cat.actions, search) {
                        rule.actions.push(d.new_item(ctx.in_object));
                        edit.structure();
                    }
                });
            });
        });
    op
}

fn rule_menu(ui: &mut egui::Ui, comment: bool) -> Option<RuleOp> {
    let mut op = None;
    ui.menu_button("☰", |ui| {
        if !comment && ui.button("Duplicar").clicked() {
            op = Some(RuleOp::Duplicate);
            ui.close();
        }
        if ui.button("Subir").clicked() {
            op = Some(RuleOp::Up);
            ui.close();
        }
        if ui.button("Descer").clicked() {
            op = Some(RuleOp::Down);
            ui.close();
        }
        if !comment && ui.button("Comentário acima").clicked() {
            op = Some(RuleOp::CommentAbove);
            ui.close();
        }
        ui.separator();
        if ui.button("🗑 Excluir").clicked() {
            op = Some(RuleOp::Delete);
            ui.close();
        }
    })
    .response
    .on_hover_text("Mais opções da regra");
    op
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()), 255)
}

/// Menu para escolher uma condição ou ação (por categoria, com busca).
fn add_menu<'a>(
    ui: &mut egui::Ui,
    label: &str,
    list: &'a [EventDef],
    search: &mut String,
) -> Option<&'a EventDef> {
    let mut chosen = None;
    // Clicar na busca não fecha o menu.
    let config = egui::containers::menu::MenuConfig::new()
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
    let (button, _) = egui::containers::menu::MenuButton::new(label)
        .config(config)
        .ui(ui, |ui| {
            ui.set_min_width(240.0);
            let s = ui.add(
                egui::TextEdit::singleline(search)
                    .hint_text("🔍 Buscar")
                    .desired_width(220.0),
            );
            if !s.has_focus() && search.is_empty() {
                s.request_focus();
            }
            ui.separator();
            let q = crate::commands::fold(search.trim());
            let hover = |d: &EventDef| {
                if d.help_text().is_empty() {
                    d.menu_text()
                } else {
                    format!("{}\n{}", d.menu_text(), d.help_text())
                }
            };
            if q.is_empty() {
                let fitting: Vec<EventDef> = list.iter().filter(|d| d.fits()).cloned().collect();
                for c in Catalog::categories(&fitting) {
                    ui.menu_button(c, |ui| {
                        for d in list.iter().filter(|d| d.category == c && d.fits()) {
                            if ui.button(&d.name).on_hover_text(hover(d)).clicked() {
                                chosen = Some(d);
                                ui.close();
                            }
                        }
                    });
                }
            } else {
                let found: Vec<&EventDef> = list
                    .iter()
                    .filter(|d| d.fits())
                    .filter(|d| {
                        crate::commands::fold(&format!(
                            "{} {} {}",
                            d.name,
                            d.menu_text(),
                            d.category
                        ))
                        .contains(&q)
                    })
                    .collect();
                for d in &found {
                    if ui.button(&d.name).on_hover_text(hover(d)).clicked() {
                        chosen = Some(*d);
                        ui.close();
                    }
                }
                if found.is_empty() {
                    ui.weak("Nada encontrado.");
                } else if s.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    // Enter escolhe o primeiro.
                    chosen = Some(found[0]);
                    ui.close();
                }
            }
        });
    // A busca é de um menu por vez: começa vazia a cada menu aberto.
    if chosen.is_some() || button.clicked() {
        search.clear();
    }
    chosen
}

/// Uma condição ou ação: a frase com os campos. Devolve `true` para tirar.
#[allow(clippy::too_many_arguments)]
fn item_line(
    ui: &mut egui::Ui,
    item: &mut EventItem,
    condition: bool,
    index: usize,
    prefix: &str,
    ctx: &Ctx,
    cat: &Catalog,
    id: egui::Id,
    edit: &mut Edit,
) -> bool {
    let mut remove = false;
    let Some(def) = cat.get(condition, &item.id) else {
        ui.horizontal(|ui| {
            ui.colored_label(
                theme::ERROR,
                format!(
                    "{} desconhecida: \"{}\"",
                    if condition { "Condição" } else { "Ação" },
                    item.id
                ),
            );
            remove = ui.small_button("✖").clicked();
        });
        return remove;
    };
    let adv_id = id.with("avancado");
    let has_advanced = def.props_now().any(|p| p.advanced);
    let mut show_adv = ui.data(|d| d.get_temp::<bool>(adv_id).unwrap_or(false));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.add_space(14.0);
        if !prefix.is_empty() {
            ui.label(RichText::new(prefix).weak());
        }
        if condition && item.not {
            ui.label(RichText::new("não").strong().color(theme::WARNING));
        }
        for piece in def.pieces() {
            match piece {
                Piece::Text(t) => {
                    let t = t.trim();
                    if !t.is_empty() {
                        ui.label(t);
                    }
                }
                Piece::Field(k) => {
                    if let Some(spec) = def.prop(&k) {
                        field(ui, item, spec, condition, index, ctx, id, edit);
                    }
                }
            }
        }
        if !def.help_text().is_empty() {
            ui.weak("(?)").on_hover_text(def.help_text());
        }
        if has_advanced
            && ui
                .selectable_label(show_adv, "🔧")
                .on_hover_text("Mais opções")
                .clicked()
        {
            show_adv = !show_adv;
            ui.data_mut(|d| d.insert_temp(adv_id, show_adv));
        }
        if condition
            && ui
                .selectable_label(item.not, RichText::new("não").small())
                .on_hover_text("Inverte: vale quando isto NÃO acontece")
                .clicked()
        {
            item.not = !item.not;
            edit.structure();
        }
        if ui
            .small_button("✖")
            .on_hover_text(if condition {
                "Tirar esta condição"
            } else {
                "Tirar esta ação"
            })
            .clicked()
        {
            remove = true;
        }
    });
    if has_advanced && show_adv {
        ui.horizontal_wrapped(|ui| {
            ui.add_space(34.0);
            for spec in def.props_now().filter(|p| p.advanced) {
                ui.weak(format!("{}:", spec.label));
                field(ui, item, spec, condition, index, ctx, id, edit);
            }
        });
    }
    remove
}

fn text_width(s: &str) -> f32 {
    (s.chars().count() as f32 * 7.5 + 16.0).clamp(48.0, 240.0)
}

/// O campo no meio da frase.
#[allow(clippy::too_many_arguments)]
fn field(
    ui: &mut egui::Ui,
    item: &mut EventItem,
    spec: &PropSpec,
    condition: bool,
    index: usize,
    ctx: &Ctx,
    id: egui::Id,
    edit: &mut Edit,
) {
    let current = item
        .get(&spec.key)
        .cloned()
        .unwrap_or_else(|| spec.default.clone());
    let salt = id.with(&spec.key);
    let merge = format!("{salt:?}");
    let mut new: Option<Prop> = None;
    let mut structural = false;
    let mut warn: Option<String> = None;
    let r = match spec.kind {
        PropType::Number => {
            let mut v = current.num();
            let range = spec.max.unwrap_or(1e6) - spec.min.unwrap_or(-1e6);
            let speed = if spec.integer {
                0.1
            } else if range <= 10.0 {
                0.01
            } else if v.abs() < 5.0 {
                0.05
            } else {
                1.0
            };
            let mut dv = egui::DragValue::new(&mut v)
                .speed(speed)
                .range(spec.min.unwrap_or(-1e9)..=spec.max.unwrap_or(1e9));
            if spec.integer {
                dv = dv.max_decimals(0);
            }
            let r = ui.add(dv);
            if r.changed() {
                new = Some(Prop::Num(if spec.integer { v.round() } else { v }));
            }
            r
        }
        PropType::Bool => {
            let mut v = current.bool();
            let r = ui.checkbox(&mut v, "");
            if r.changed() {
                new = Some(Prop::Bool(v));
                structural = true;
            }
            r
        }
        PropType::Color => {
            let mut c = crate::scene_render::egui_color(current.str()).to_srgba_unmultiplied();
            let r = ui.color_edit_button_srgba_unmultiplied(&mut c);
            if r.changed() {
                let hex = if c[3] == 255 {
                    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
                } else {
                    format!("#{:02X}{:02X}{:02X}{:02X}", c[0], c[1], c[2], c[3])
                };
                new = Some(Prop::Str(hex));
            }
            r
        }
        PropType::Choice => {
            let v = match &current {
                Prop::Str(s) => s.clone(),
                other => events::value_text(None, other, ctx.in_object),
            };
            let full = events::value_text(Some(spec), &current, ctx.in_object);
            // Na caixa, sem a explicação entre parênteses (ela fica na lista).
            let label = full.split(" (").next().unwrap_or(&full).to_string();
            let mut chosen = None;
            let r = egui::ComboBox::from_id_salt(salt)
                .selected_text(label.clone())
                .width(text_width(&label) + 12.0)
                .show_ui(ui, |ui| {
                    for (k, l) in &spec.options {
                        if ui.selectable_label(*k == v, l).clicked() {
                            chosen = Some(k.clone());
                        }
                    }
                })
                .response;
            if let Some(k) = chosen
                && k != v
            {
                new = Some(Prop::Str(k));
                structural = true;
            }
            r
        }
        PropType::Target => {
            let v = current.str().to_string();
            let label = events::value_text(Some(spec), &current, ctx.in_object);
            let mut options: Vec<(String, String)> = Vec::new();
            if let Some(empty) = &spec.allow_empty {
                options.push((String::new(), empty.clone()));
            }
            if ctx.in_object {
                options.push(("@este".into(), "este objeto".into()));
            }
            if !condition || index > 0 {
                options.push(("@outro".into(), "o outro".into()));
            }
            options.push(("@jogador".into(), "o jogador".into()));
            for n in ctx.names {
                options.push((n.clone(), n.clone()));
            }
            if !v.starts_with('@') && !v.is_empty() && ctx.check_names && !ctx.names.contains(&v) {
                warn = Some(format!("Não há objeto chamado \"{v}\" nesta cena."));
            }
            let mut chosen = None;
            let r = egui::ComboBox::from_id_salt(salt)
                .selected_text(RichText::new(label.clone()).strong())
                .width(text_width(&label) + 12.0)
                .show_ui(ui, |ui| {
                    for (k, l) in &options {
                        if k == "@jogador" && ctx.names.is_empty() {
                            // Num prefab ainda vale (os nomes vêm das cenas).
                        }
                        let text = if k.starts_with('@') {
                            RichText::new(l).italics()
                        } else {
                            RichText::new(l)
                        };
                        if ui.selectable_label(*k == v, text).clicked() {
                            chosen = Some(k.clone());
                        }
                    }
                    if !ctx.check_names {
                        ui.separator();
                        ui.weak("Outro nome: digite no campo de texto ao lado.");
                    }
                })
                .response;
            if !ctx.check_names {
                // Num prefab, os objetos estão nas cenas: deixa digitar o nome.
                let mut text = if v.starts_with('@') {
                    String::new()
                } else {
                    v.clone()
                };
                let t = ui.add(
                    egui::TextEdit::singleline(&mut text)
                        .hint_text("nome")
                        .desired_width(70.0),
                );
                if t.changed() {
                    new = Some(Prop::Str(if text.is_empty() {
                        "@jogador".into()
                    } else {
                        text
                    }));
                }
            }
            if let Some(k) = chosen
                && k != v
            {
                new = Some(Prop::Str(k));
                structural = true;
            }
            r
        }
        PropType::Object => {
            let v = current.str().to_string();
            let empty = spec
                .allow_empty
                .clone()
                .unwrap_or_else(|| "(escolha)".into());
            let label = if v.is_empty() {
                empty.clone()
            } else {
                v.clone()
            };
            if !v.is_empty() && ctx.check_names && !ctx.names.contains(&v) {
                warn = Some(format!("Não há objeto chamado \"{v}\" nesta cena."));
            } else if v.is_empty() && spec.allow_empty.is_none() {
                warn = Some("Escolha um objeto.".into());
            }
            let mut chosen = None;
            let r = egui::ComboBox::from_id_salt(salt)
                .selected_text(RichText::new(label.clone()).strong())
                .width(text_width(&label) + 12.0)
                .show_ui(ui, |ui| {
                    if spec.allow_empty.is_some()
                        && ui.selectable_label(v.is_empty(), &empty).clicked()
                    {
                        chosen = Some(String::new());
                    }
                    for n in ctx.names {
                        if ui.selectable_label(*n == v, n).clicked() {
                            chosen = Some(n.clone());
                        }
                    }
                })
                .response;
            if !ctx.check_names {
                let mut text = v.clone();
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut text)
                            .hint_text("nome")
                            .desired_width(70.0),
                    )
                    .changed()
                {
                    new = Some(Prop::Str(text));
                }
            }
            if let Some(k) = chosen
                && k != v
            {
                new = Some(Prop::Str(k));
                structural = true;
            }
            r
        }
        PropType::Var => {
            let mut v = current.str().to_string();
            let hint = spec
                .allow_empty
                .clone()
                .unwrap_or_else(|| "variável".into());
            let w = text_width(&v).max(70.0);
            let r = ui.add(
                egui::TextEdit::singleline(&mut v)
                    .hint_text(hint.clone())
                    .desired_width(w),
            );
            if r.changed() {
                new = Some(Prop::Str(v.trim().to_string()));
            }
            let mut chosen = None;
            ui.menu_button("…", |ui| {
                if spec.allow_empty.is_some() && ui.button(format!("({hint})")).clicked() {
                    chosen = Some(String::new());
                    ui.close();
                }
                if ctx.env.vars.is_empty() {
                    ui.weak("Nenhuma variável nas Configurações do projeto.");
                }
                for name in ctx.env.vars {
                    if ui.button(name).clicked() {
                        chosen = Some(name.clone());
                        ui.close();
                    }
                }
            })
            .response
            .on_hover_text("Variáveis do jogo (Configurações do projeto)");
            if let Some(c) = chosen {
                new = Some(Prop::Str(c));
                structural = true;
            }
            let shown = new
                .as_ref()
                .map(|p| p.str().to_string())
                .unwrap_or_else(|| current.str().to_string());
            if !shown.is_empty() && !ctx.env.vars.contains(&shown) {
                warn = Some(format!(
                    "\"{shown}\" não está nas variáveis do jogo (Configurações do projeto): ela começa vazia."
                ));
            }
            r
        }
        PropType::Anim => {
            let mut v = current.str().to_string();
            let hint = spec
                .allow_empty
                .clone()
                .unwrap_or_else(|| "animação".into());
            let w = text_width(&v).max(70.0);
            let r = ui.add(
                egui::TextEdit::singleline(&mut v)
                    .hint_text(hint.clone())
                    .desired_width(w),
            );
            if r.changed() {
                new = Some(Prop::Str(v.trim().to_string()));
            }
            let mut chosen = None;
            ui.menu_button("…", |ui| {
                if spec.allow_empty.is_some() && ui.button(format!("({hint})")).clicked() {
                    chosen = Some(String::new());
                    ui.close();
                }
                if ctx.env.anims.is_empty() {
                    ui.weak(if events::three_d() {
                        "Nenhum modelo 3D animado nesta cena."
                    } else {
                        "Nenhuma animação ainda: crie na aba de uma imagem (Animações)."
                    });
                }
                for name in ctx.env.anims {
                    if ui.button(name).clicked() {
                        chosen = Some(name.clone());
                        ui.close();
                    }
                }
            })
            .response
            .on_hover_text(if events::three_d() {
                "Animações dos modelos 3D da cena"
            } else {
                "Animações das imagens do projeto"
            });
            if let Some(c) = chosen {
                new = Some(Prop::Str(c));
                structural = true;
            }
            let shown = new
                .as_ref()
                .map(|p| p.str().to_string())
                .unwrap_or_else(|| current.str().to_string());
            if !shown.is_empty() && !ctx.env.anims.contains(&shown) {
                warn = Some(format!(
                    "Nenhuma imagem do projeto tem a animação \"{shown}\"."
                ));
            } else if shown.is_empty() && spec.allow_empty.is_none() {
                warn = Some("Escolha uma animação.".into());
            }
            r
        }
        PropType::TileType => {
            let mut v = current.str().to_string();
            let hint = spec.allow_empty.clone().unwrap_or_else(|| "tipo".into());
            let w = text_width(&v).max(70.0);
            let r = ui.add(
                egui::TextEdit::singleline(&mut v)
                    .hint_text(hint.clone())
                    .desired_width(w),
            );
            if r.changed() {
                new = Some(Prop::Str(v.trim().to_string()));
            }
            let mut chosen = None;
            ui.menu_button("…", |ui| {
                if spec.allow_empty.is_some() && ui.button(format!("({hint})")).clicked() {
                    chosen = Some(String::new());
                    ui.close();
                }
                if ctx.env.tile_kinds.is_empty() {
                    ui.weak("Nenhum tile com tipo ainda: no painel Tiles, use Tipo para marcar.");
                }
                for k in ctx.env.tile_kinds {
                    if ui.button(k).clicked() {
                        chosen = Some(k.clone());
                        ui.close();
                    }
                }
            })
            .response
            .on_hover_text("Tipos dos tiles dos mapas desta cena");
            if let Some(c) = chosen {
                new = Some(Prop::Str(c));
                structural = true;
            }
            r
        }
        PropType::Tile => {
            let mut v = current.num().max(0.0).round();
            let r = ui.add(
                egui::DragValue::new(&mut v)
                    .speed(0.1)
                    .range(0.0..=1e6)
                    .max_decimals(0),
            );
            if r.changed() {
                new = Some(Prop::Num(v));
            }
            if let Some(sheet) = ctx.env.tile_sheet {
                let mut chosen = None;
                let menu = ui.menu_button("…", |ui| {
                    ui.horizontal(|ui| {
                        ui.weak("Clique num tile.");
                        if ui.small_button("Nenhum (apaga)").clicked() {
                            chosen = Some(0.0);
                            ui.close();
                        }
                    });
                    if let Some(t) = tile_picker(ui, sheet, v as u32) {
                        chosen = Some(t as f64);
                        ui.close();
                    }
                });
                menu.response.on_hover_text("Escolher no tileset do mapa");
                if let Some(c) = chosen {
                    new = Some(Prop::Num(c));
                    structural = true;
                }
            }
            r
        }
        PropType::Value => {
            let mut v = match &current {
                Prop::Num(n) => format_num(*n),
                Prop::Bool(b) => b.to_string(),
                Prop::Str(s) => s.clone(),
            };
            let w = text_width(&v).max(40.0);
            let r = ui.add(egui::TextEdit::singleline(&mut v).desired_width(w));
            if r.changed() {
                let t = v.trim();
                new = Some(match t.replace(',', ".").parse::<f64>() {
                    Ok(n) if !t.is_empty() => Prop::Num(n),
                    _ => Prop::Str(v.clone()),
                });
            }
            r
        }
        PropType::Behavior => {
            let v = current.str().to_string();
            let mut chosen = None;
            let r = egui::ComboBox::from_id_salt(salt)
                .selected_text(v.clone())
                .width(text_width(&v) + 12.0)
                .show_ui(ui, |ui| {
                    for b in crate::behaviors::builtins() {
                        let id = b.script.trim_start_matches("nn:");
                        if ui.selectable_label(v == id, &b.title).clicked() {
                            chosen = Some(id.to_string());
                        }
                    }
                    for e in ctx.env.files.files_of(FileKind::Lua) {
                        if e.rel.starts_with("scripts/")
                            && ui.selectable_label(v == e.rel, &e.rel).clicked()
                        {
                            chosen = Some(e.rel.clone());
                        }
                    }
                })
                .response;
            if let Some(c) = chosen {
                new = Some(Prop::Str(c));
                structural = true;
            }
            r
        }
        PropType::Scene
        | PropType::Prefab
        | PropType::Sound
        | PropType::Image
        | PropType::Dialogue => {
            let v = current.str().to_string();
            let kinds: &[FileKind] = match spec.kind {
                PropType::Scene => &[FileKind::Scene, FileKind::Lua],
                PropType::Prefab => &[FileKind::Prefab],
                PropType::Dialogue => &[FileKind::Dialogue],
                PropType::Sound => &[FileKind::Sound],
                _ => &[FileKind::Image],
            };
            let list: Vec<String> = kinds
                .iter()
                .flat_map(|k| ctx.env.files.files_of(*k).map(|e| e.rel.clone()))
                .filter(|r| r != "config.lua")
                .collect();
            let label = events::value_text(Some(spec), &current, ctx.in_object);
            if v.is_empty() && !matches!(spec.kind, PropType::Sound) {
                warn = Some("Escolha um arquivo (ou arraste do painel Arquivos).".into());
            } else if !v.is_empty() && !list.contains(&v) {
                warn = Some(format!("O arquivo {v} não existe."));
            }
            let mut chosen = None;
            let r = egui::ComboBox::from_id_salt(salt)
                .selected_text(RichText::new(label.clone()).strong())
                .width(text_width(&label) + 12.0)
                .show_ui(ui, |ui| {
                    for rel in &list {
                        if ui.selectable_label(*rel == v, rel).clicked() {
                            chosen = Some(rel.clone());
                        }
                    }
                    if list.is_empty() {
                        ui.weak("Nenhum arquivo desse tipo no projeto.");
                    }
                })
                .response;
            if let Some(p) = r.dnd_release_payload::<FileDrag>()
                && kinds.contains(&FileKind::of(std::path::Path::new(&p.0)))
            {
                chosen = Some(p.0.clone());
            }
            if let Some(c) = chosen {
                new = Some(Prop::Str(c));
                structural = true;
            }
            r.on_hover_text(if v.is_empty() {
                "Escolha na lista ou arraste do painel Arquivos para cá.".to_string()
            } else {
                v.clone()
            })
        }
        PropType::Text => {
            let mut v = current.str().to_string();
            let w = text_width(&v).max(60.0);
            let r = ui.add(egui::TextEdit::singleline(&mut v).desired_width(w));
            if r.changed() {
                new = Some(Prop::Str(v));
            }
            r
        }
    };
    if let Some(w) = &warn {
        ui.painter().rect_stroke(
            r.rect.expand(1.0),
            3.0,
            Stroke::new(1.5, theme::WARNING),
            egui::StrokeKind::Outside,
        );
        r.on_hover_text(w);
    } else if !spec.help.is_empty() {
        r.on_hover_text(&spec.help);
    }
    if let Some(v) = new {
        item.set(&spec.key, v);
        if structural {
            edit.structure();
        } else {
            edit.field(merge);
        }
    }
}

/// A grade do tileset; devolve o tile clicado (número do mapa).
fn tile_picker(ui: &mut egui::Ui, sheet: &TileSheet, current: u32) -> Option<u32> {
    let d = &sheet.data;
    let z = (256.0 / sheet.size[0].max(1) as f32)
        .clamp(1.0, 4.0)
        .floor();
    let size = egui::Vec2::new(sheet.size[0] as f32, sheet.size[1] as f32) * z;
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, egui::Color32::from_gray(0x40));
    p.image(
        sheet.texture.id(),
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    let tile_rect = |id: u32| {
        let (x, y) = d.tile_origin(id);
        egui::Rect::from_min_size(
            rect.min + egui::Vec2::new(x as f32, y as f32) * z,
            egui::Vec2::new(d.tile_w as f32, d.tile_h as f32) * z,
        )
    };
    if current >= sheet.first_gid && current - sheet.first_gid < d.count {
        p.rect_stroke(
            tile_rect(current - sheet.first_gid),
            0.0,
            egui::Stroke::new(2.0, egui::Color32::WHITE),
            egui::StrokeKind::Inside,
        );
    }
    let at = |pos: egui::Pos2| (0..d.count).find(|id| tile_rect(*id).contains(pos));
    if let Some(pos) = resp.hover_pos()
        && let Some(id) = at(pos)
    {
        p.rect_stroke(
            tile_rect(id),
            0.0,
            egui::Stroke::new(1.0, egui::Color32::YELLOW),
            egui::StrokeKind::Inside,
        );
        let info = d.info(id);
        let mut tip = format!("tile {}", id + sheet.first_gid);
        if !info.kind.is_empty() {
            tip.push_str(&format!(" · tipo {}", info.kind));
        }
        resp.clone().on_hover_text_at_pointer(tip);
    }
    if resp.clicked()
        && let Some(pos) = resp.interact_pointer_pos()
    {
        return at(pos).map(|id| id + sheet.first_gid);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edicao_junta_digitacao_e_separa_estrutura() {
        let mut e = Edit::default();
        e.field("a".into());
        assert_eq!(e.merge.as_deref(), Some("a"));
        e.structure();
        e.field("b".into());
        assert!(e.changed && e.merge.is_none());
    }

    #[test]
    fn cor_entre_duas() {
        assert_eq!(
            lerp_color(Color32::BLACK, Color32::WHITE, 0.5),
            Color32::from_rgb(128, 128, 128)
        );
    }
}
