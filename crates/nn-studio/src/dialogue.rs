//! Diálogos (`.dialogo`): o editor visual de conversa.
//!
//! Cada fala é um cartão na tela: quem fala, o texto e para onde a conversa
//! vai depois (a próxima fala, as escolhas do jogador ou o fim). Arrastar da
//! bolinha de um cartão (ou de uma escolha) até outro liga os dois; soltar no
//! vazio cria uma fala nova ali. O painel da direita edita a fala escolhida
//! ou, sem nenhuma escolhida, a conversa (os personagens e a velocidade).
//!
//! O arquivo é uma tabela Lua, lida pela engine (`dialogos.lua`).

use std::path::Path;

use eframe::egui::{self, Color32, Id, Pos2, Rect, Sense, Stroke, Vec2};

use crate::theme;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Character {
    pub name: String,
    /// "#RRGGBB" (a etiqueta do nome).
    pub color: String,
    /// Retrato (imagem) que aparece na caixa de fala.
    pub image: String,
    /// Som que toca enquanto o texto aparece (a "voz").
    pub sound: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Choice {
    pub text: String,
    /// A fala para onde vai (0 = termina).
    pub next: u32,
    /// Ao escolher: muda esta variável do jogo...
    pub var: String,
    /// ... para este valor (número ou texto).
    pub value: String,
    /// Só aparece para quem tem este item no inventário.
    pub need: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Node {
    pub id: u32,
    /// Onde o cartão fica no editor.
    pub x: f32,
    pub y: f32,
    /// Quem fala (o nome de um personagem; vazio = narrador).
    pub who: String,
    pub text: String,
    /// A próxima fala, sem escolhas (0 = termina).
    pub next: u32,
    pub choices: Vec<Choice>,
    /// Ao aparecer: muda esta variável...
    pub var: String,
    pub value: String,
    /// ... e dá este item.
    pub item: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DialogueData {
    pub start: u32,
    /// Letras por segundo (0 = o texto aparece de uma vez).
    pub speed: f64,
    pub characters: Vec<Character>,
    pub nodes: Vec<Node>,
}

impl Default for DialogueData {
    fn default() -> Self {
        Self {
            start: 1,
            speed: 40.0,
            characters: Vec::new(),
            nodes: vec![Node {
                id: 1,
                x: 40.0,
                y: 40.0,
                text: "Olá!".into(),
                ..Node::default()
            }],
        }
    }
}

impl DialogueData {
    pub fn node(&self, id: u32) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn node_mut(&mut self, id: u32) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn next_id(&self) -> u32 {
        self.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1
    }

    /// Cria uma fala em (x, y) e devolve o id.
    pub fn add_node(&mut self, x: f32, y: f32, who: &str) -> u32 {
        let id = self.next_id();
        self.nodes.push(Node {
            id,
            x,
            y,
            who: who.to_string(),
            ..Node::default()
        });
        if self.nodes.len() == 1 {
            self.start = id;
        }
        id
    }

    /// Apaga a fala; o que ia para ela passa a terminar.
    pub fn remove_node(&mut self, id: u32) {
        self.nodes.retain(|n| n.id != id);
        for n in &mut self.nodes {
            if n.next == id {
                n.next = 0;
            }
            for c in &mut n.choices {
                if c.next == id {
                    c.next = 0;
                }
            }
        }
        if self.start == id {
            self.start = self.nodes.first().map(|n| n.id).unwrap_or(0);
        }
    }

    pub fn character(&self, name: &str) -> Option<&Character> {
        self.characters.iter().find(|c| c.name == name)
    }
}

/// O conteúdo de um diálogo novo (Arquivos > ➕ Novo > Diálogo).
pub fn template() -> String {
    let d = DialogueData {
        start: 1,
        speed: 40.0,
        characters: vec![Character {
            name: "Ana".into(),
            color: "#FFD166".into(),
            ..Character::default()
        }],
        nodes: vec![
            Node {
                id: 1,
                x: 40.0,
                y: 60.0,
                who: "Ana".into(),
                text: "Olá! Que bom te ver por aqui.".into(),
                next: 2,
                ..Node::default()
            },
            Node {
                id: 2,
                x: 340.0,
                y: 60.0,
                who: "Ana".into(),
                text: "Posso te ajudar?".into(),
                choices: vec![
                    Choice {
                        text: "Sim, por favor!".into(),
                        next: 3,
                        ..Choice::default()
                    },
                    Choice {
                        text: "Agora não.".into(),
                        next: 0,
                        ..Choice::default()
                    },
                ],
                ..Node::default()
            },
            Node {
                id: 3,
                x: 640.0,
                y: 40.0,
                who: "Ana".into(),
                text: "Procure a chave perto da fogueira.".into(),
                ..Node::default()
            },
        ],
    };
    write(&d)
}

// --------------------------------------------------------------------------
// Ler e gravar

fn get_str(t: &mlua::Table, k: &str) -> String {
    match t.get::<mlua::Value>(k) {
        Ok(mlua::Value::String(s)) => s.to_string_lossy(),
        Ok(mlua::Value::Integer(i)) => i.to_string(),
        Ok(mlua::Value::Number(n)) => crate::scene_data::format_num(n),
        Ok(mlua::Value::Boolean(b)) => b.to_string(),
        _ => String::new(),
    }
}

fn get_num(t: &mlua::Table, k: &str, default: f64) -> f64 {
    match t.get::<mlua::Value>(k) {
        Ok(mlua::Value::Integer(i)) => i as f64,
        Ok(mlua::Value::Number(n)) => n,
        _ => default,
    }
}

/// Lê o texto de um `.dialogo`.
pub fn read(text: &str, rel: &str) -> Result<DialogueData, String> {
    let lua = mlua::Lua::new();
    let t: mlua::Table = lua
        .load(text)
        .set_name(format!("@{rel}"))
        .eval()
        .map_err(|e| nn_script::translate_error(e.to_string().trim()))?;
    let mut d = DialogueData {
        start: get_num(&t, "start", 1.0) as u32,
        speed: get_num(&t, "speed", 40.0),
        characters: Vec::new(),
        nodes: Vec::new(),
    };
    if let Ok(list) = t.get::<mlua::Table>("characters") {
        for c in list.sequence_values::<mlua::Table>().filter_map(Result::ok) {
            d.characters.push(Character {
                name: get_str(&c, "name"),
                color: get_str(&c, "color"),
                image: get_str(&c, "image"),
                sound: get_str(&c, "sound"),
            });
        }
    }
    if let Ok(list) = t.get::<mlua::Table>("nodes") {
        for (i, n) in list
            .sequence_values::<mlua::Table>()
            .filter_map(Result::ok)
            .enumerate()
        {
            let id = get_num(&n, "id", 0.0) as u32;
            if id == 0 {
                return Err(format!("a fala {} não tem id", i + 1));
            }
            let mut node = Node {
                id,
                x: get_num(&n, "x", 40.0 + i as f64 * 300.0) as f32,
                y: get_num(&n, "y", 40.0) as f32,
                who: get_str(&n, "who"),
                text: get_str(&n, "text"),
                next: get_num(&n, "next", 0.0) as u32,
                choices: Vec::new(),
                var: get_str(&n, "var"),
                value: get_str(&n, "value"),
                item: get_str(&n, "item"),
            };
            if let Ok(cs) = n.get::<mlua::Table>("choices") {
                for c in cs.sequence_values::<mlua::Table>().filter_map(Result::ok) {
                    node.choices.push(Choice {
                        text: get_str(&c, "text"),
                        next: get_num(&c, "next", 0.0) as u32,
                        var: get_str(&c, "var"),
                        value: get_str(&c, "value"),
                        need: get_str(&c, "need"),
                    });
                }
            }
            d.nodes.push(node);
        }
    }
    Ok(d)
}

fn lua_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Um valor de variável: número se der, senão texto.
fn lua_value(s: &str) -> String {
    let t = s.trim();
    match t.parse::<f64>() {
        Ok(n) if !t.is_empty() && n.is_finite() => crate::scene_data::format_num(n),
        _ => lua_str(s),
    }
}

/// O texto de um `.dialogo` (o mesmo formato que a engine lê).
pub fn write(d: &DialogueData) -> String {
    let mut s = String::from(
        "-- Diálogo do NN Studio (editor de conversa). Dá para editar à mão;\n\
         -- ao salvar pelo editor, comentários escritos aqui se perdem.\n\
         return {\n  format = 1,\n",
    );
    s.push_str(&format!(
        "  start = {},\n  speed = {},\n",
        d.start,
        crate::scene_data::format_num(d.speed)
    ));
    s.push_str("  characters = {\n");
    for c in &d.characters {
        s.push_str(&format!(
            "    {{ name = {}, color = {}, image = {}, sound = {} }},\n",
            lua_str(&c.name),
            lua_str(&c.color),
            lua_str(&c.image),
            lua_str(&c.sound)
        ));
    }
    s.push_str("  },\n  nodes = {\n");
    for n in &d.nodes {
        s.push_str(&format!(
            "    {{ id = {}, x = {}, y = {}, who = {},\n      text = {},\n",
            n.id,
            n.x.round(),
            n.y.round(),
            lua_str(&n.who),
            lua_str(&n.text)
        ));
        if n.choices.is_empty() {
            s.push_str(&format!("      next = {},\n", n.next));
        } else {
            s.push_str("      choices = {\n");
            for c in &n.choices {
                let mut extra = String::new();
                if !c.var.is_empty() {
                    extra.push_str(&format!(
                        ", var = {}, value = {}",
                        lua_str(&c.var),
                        lua_value(&c.value)
                    ));
                }
                if !c.need.is_empty() {
                    extra.push_str(&format!(", need = {}", lua_str(&c.need)));
                }
                s.push_str(&format!(
                    "        {{ text = {}, next = {}{extra} }},\n",
                    lua_str(&c.text),
                    c.next
                ));
            }
            s.push_str("      },\n");
        }
        if !n.var.is_empty() {
            s.push_str(&format!(
                "      var = {}, value = {},\n",
                lua_str(&n.var),
                lua_value(&n.value)
            ));
        }
        if !n.item.is_empty() {
            s.push_str(&format!("      item = {},\n", lua_str(&n.item)));
        }
        s.push_str("    },\n");
    }
    s.push_str("  },\n}\n");
    s
}

// --------------------------------------------------------------------------
// O documento aberto

#[derive(Debug, Clone, Copy, PartialEq)]
enum Drag {
    Node {
        id: u32,
    },
    Pan,
    /// Ligando: da saída da fala (ou da escolha) até onde o mouse está.
    Link {
        from: u32,
        choice: Option<usize>,
    },
}

/// O que o editor pede ao espaço de trabalho.
#[derive(Debug, Clone, PartialEq)]
pub enum DialogueAction {
    Save,
}

/// Arquivos do projeto que os campos oferecem.
#[derive(Debug, Default, Clone)]
pub struct DialogueEnv {
    pub images: Vec<String>,
    pub sounds: Vec<String>,
}

pub struct DialogueDoc {
    pub rel: String,
    pub data: DialogueData,
    saved: DialogueData,
    /// O arquivo não abriu (o texto fica intocado até corrigir).
    pub error: Option<String>,
    undo: Vec<DialogueData>,
    redo: Vec<DialogueData>,
    pub selected: Option<u32>,
    pan: Vec2,
    drag: Option<Drag>,
    /// O que está sendo editado (mudanças seguidas do mesmo campo viram um
    /// passo só de desfazer).
    group: Option<Id>,
}

impl std::fmt::Debug for DialogueDoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DialogueDoc")
            .field("rel", &self.rel)
            .finish_non_exhaustive()
    }
}

const CARD_W: f32 = 240.0;
const HEADER_H: f32 = 26.0;
const CHOICE_H: f32 = 24.0;
const PORT_R: f32 = 6.0;

impl DialogueDoc {
    pub fn open(root: &Path, rel: &str) -> Self {
        let (data, error) = match std::fs::read_to_string(root.join(rel)) {
            Ok(text) => match read(&text, rel) {
                Ok(d) => (d, None),
                Err(e) => (DialogueData::default(), Some(e)),
            },
            Err(e) => (DialogueData::default(), Some(e.to_string())),
        };
        Self::with(rel, data, error)
    }

    pub fn with(rel: &str, data: DialogueData, error: Option<String>) -> Self {
        Self {
            rel: rel.to_string(),
            saved: data.clone(),
            data,
            error,
            undo: Vec::new(),
            redo: Vec::new(),
            selected: None,
            pan: Vec2::ZERO,
            drag: None,
            group: None,
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.error.is_none() && self.data != self.saved
    }

    /// O arquivo mudou fora: relê (se não houver nada a perder).
    pub fn disk_changed(&mut self, root: &Path) {
        if self.is_dirty() {
            return;
        }
        let fresh = Self::open(root, &self.rel);
        if fresh.data != self.data || fresh.error != self.error {
            self.data = fresh.data.clone();
            self.saved = fresh.data;
            self.error = fresh.error;
            self.fix_selection();
        }
    }

    pub fn save(&mut self, root: &Path) -> Result<(), String> {
        if let Some(e) = &self.error {
            return Err(format!("o arquivo tem um erro: {e}"));
        }
        std::fs::write(root.join(&self.rel), write(&self.data)).map_err(|e| e.to_string())?;
        self.saved = self.data.clone();
        Ok(())
    }

    pub fn undo(&mut self) {
        if let Some(d) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.data, d));
            self.group = None;
            self.fix_selection();
        }
    }

    pub fn redo(&mut self) {
        if let Some(d) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.data, d));
            self.group = None;
            self.fix_selection();
        }
    }

    fn fix_selection(&mut self) {
        if let Some(id) = self.selected
            && self.data.node(id).is_none()
        {
            self.selected = None;
        }
    }

    /// Cria uma fala nova no meio do que está à mostra (e escolhe ela).
    pub fn add_node_at_view(&mut self, view: Rect) -> u32 {
        let c = view.center() - view.min - self.pan;
        // Não em cima de outra.
        let (mut x, mut y) = (c.x - CARD_W / 2.0, c.y - 40.0);
        while self
            .data
            .nodes
            .iter()
            .any(|n| (n.x - x).abs() < 30.0 && (n.y - y).abs() < 30.0)
        {
            x += 40.0;
            y += 40.0;
        }
        let who = self
            .selected
            .and_then(|s| self.data.node(s))
            .map(|n| n.who.clone())
            .unwrap_or_default();
        let id = self.data.add_node(x, y, &who);
        self.selected = Some(id);
        id
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, env: &DialogueEnv) -> Vec<DialogueAction> {
        let mut actions = Vec::new();
        if let Some(e) = &self.error {
            ui.add_space(8.0);
            ui.colored_label(
                theme::ERROR,
                format!("Este diálogo não abriu: {e}. Corrija o arquivo (botão direito na aba > Abrir no programa padrão) e abra de novo."),
            );
            return actions;
        }
        let before = self.data.clone();
        let mut group: Option<Id> = None;

        // Barra de cima.
        let mut add_clicked = false;
        ui.horizontal(|ui| {
            let r = ui
                .button("➕ Fala")
                .on_hover_text("Uma fala nova (ou arraste da bolinha de uma fala para o vazio)");
            crate::tutorial::anchor(ui.ctx(), "dialogo.nova_fala", r.rect);
            add_clicked = r.clicked();
            if ui
                .add_enabled(self.selected.is_some(), egui::Button::new("Apagar fala"))
                .on_hover_text("Del")
                .clicked()
                && let Some(id) = self.selected.take()
            {
                self.data.remove_node(id);
            }
            ui.separator();
            if ui.button("Salvar").on_hover_text("Ctrl+S").clicked() {
                actions.push(DialogueAction::Save);
            }
            if ui
                .button("Enquadrar")
                .on_hover_text("Mostra todas as falas")
                .clicked()
            {
                self.frame_all();
            }
            ui.separator();
            ui.weak("Arraste da bolinha de uma fala até outra para ligar; solte no vazio para criar uma fala.");
        });

        egui::Panel::right(Id::new(("dialogo-painel", &self.rel)))
            .resizable(true)
            .default_size(320.0)
            .min_size(240.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    crate::tutorial::anchor(ui.ctx(), "dialogo.painel", ui.max_rect());
                    if let Some(id) = self.selected {
                        self.node_panel(ui, id, env, &mut group);
                    } else {
                        self.conversation_panel(ui, env, &mut group);
                    }
                });
            });
        egui::CentralPanel::no_frame().show(ui, |ui| {
            let view = ui.max_rect();
            if add_clicked {
                self.add_node_at_view(view);
            }
            self.canvas(ui, view, &mut group);
        });

        // Desfazer: mudanças seguidas no mesmo campo (ou arrastando o mesmo
        // cartão) viram um passo só.
        if self.data != before {
            if group.is_none() || group != self.group {
                self.undo.push(before);
                if self.undo.len() > 200 {
                    self.undo.remove(0);
                }
            }
            self.redo.clear();
            self.group = group;
        }
        actions
    }

    fn frame_all(&mut self) {
        if let Some(min) = self
            .data
            .nodes
            .iter()
            .map(|n| Vec2::new(n.x, n.y))
            .reduce(|a, b| a.min(b))
        {
            self.pan = Vec2::new(30.0, 30.0) - min;
        }
    }

    /// A altura do cartão (o texto quebra em até 4 linhas).
    fn card_rect(&self, origin: Pos2, n: &Node, text_h: f32) -> Rect {
        let rows = if n.choices.is_empty() {
            0.0
        } else {
            n.choices.len() as f32 * CHOICE_H + 6.0
        };
        Rect::from_min_size(
            origin + Vec2::new(n.x, n.y) + self.pan,
            Vec2::new(CARD_W, HEADER_H + text_h + 14.0 + rows),
        )
    }

    fn canvas(&mut self, ui: &mut egui::Ui, view: Rect, group: &mut Option<Id>) {
        let painter = ui.painter_at(view);
        painter.rect_filled(view, 0.0, ui.visuals().extreme_bg_color);
        // Pontinhos de fundo (andam com o arrastar).
        let step = 24.0;
        let off = Vec2::new(self.pan.x.rem_euclid(step), self.pan.y.rem_euclid(step));
        let dot = ui.visuals().weak_text_color().gamma_multiply(0.25);
        let mut y = view.top() + off.y;
        while y < view.bottom() {
            let mut x = view.left() + off.x;
            while x < view.right() {
                painter.circle_filled(Pos2::new(x, y), 1.0, dot);
                x += step;
            }
            y += step;
        }
        let bg = ui.interact(
            view,
            Id::new(("dialogo-fundo", &self.rel)),
            Sense::click_and_drag(),
        );
        crate::tutorial::anchor(ui.ctx(), "dialogo.tela", view);

        // Medidas de cada cartão.
        let origin = view.min;
        let font = egui::FontId::proportional(13.0);
        let text_color = ui.visuals().text_color();
        struct Card {
            id: u32,
            rect: Rect,
            text: std::sync::Arc<egui::Galley>,
        }
        let cards: Vec<Card> = self
            .data
            .nodes
            .iter()
            .map(|n| {
                let mut t = n.text.clone();
                if t.is_empty() {
                    t = "(sem texto)".into();
                }
                let mut job =
                    egui::text::LayoutJob::simple(t, font.clone(), text_color, CARD_W - 20.0);
                job.wrap.max_rows = 4;
                let galley = ui.fonts_mut(|f| f.layout_job(job));
                let rect = self.card_rect(origin, n, galley.size().y);
                Card {
                    id: n.id,
                    rect,
                    text: galley,
                }
            })
            .collect();
        let rect_of = |id: u32| cards.iter().find(|c| c.id == id).map(|c| c.rect);
        // Saídas: (fala, escolha, ponto).
        let mut ports: Vec<(u32, Option<usize>, Pos2)> = Vec::new();
        for (card, n) in cards.iter().zip(&self.data.nodes) {
            if n.choices.is_empty() {
                ports.push((
                    n.id,
                    None,
                    Pos2::new(card.rect.right(), card.rect.top() + HEADER_H / 2.0),
                ));
            } else {
                let top = card.rect.bottom() - n.choices.len() as f32 * CHOICE_H - 3.0;
                for i in 0..n.choices.len() {
                    ports.push((
                        n.id,
                        Some(i),
                        Pos2::new(card.rect.right(), top + (i as f32 + 0.5) * CHOICE_H),
                    ));
                }
            }
        }

        // As ligações (atrás dos cartões).
        let accent = theme::ACCENT;
        let link_color = ui.visuals().text_color().gamma_multiply(0.55);
        for &(id, choice, p) in &ports {
            let Some(n) = self.data.node(id) else {
                continue;
            };
            let next = match choice {
                Some(i) => n.choices[i].next,
                None => n.next,
            };
            if next == 0 {
                continue;
            }
            if let Some(r) = rect_of(next) {
                let q = Pos2::new(r.left(), r.top() + HEADER_H / 2.0);
                let sel = self.selected == Some(id) || self.selected == Some(next);
                draw_link(&painter, p, q, if sel { accent } else { link_color });
            }
        }

        // Os cartões.
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let mut clicked_card = false;
        for card in &cards {
            let Some(n) = self.data.node(card.id).cloned() else {
                continue;
            };
            let resp = ui.interact(
                card.rect,
                Id::new(("dialogo-fala", &self.rel, n.id)),
                Sense::click_and_drag(),
            );
            if resp.clicked() || resp.drag_started() {
                self.selected = Some(n.id);
                clicked_card = true;
            }
            if resp.drag_started() {
                self.drag = Some(Drag::Node { id: n.id });
            }
            if self.drag == Some(Drag::Node { id: n.id }) && resp.dragged() {
                let d = resp.drag_delta();
                if let Some(m) = self.data.node_mut(n.id) {
                    m.x += d.x;
                    m.y += d.y;
                }
                *group = Some(Id::new(("dialogo-arrastar", n.id)));
            }
            crate::tutorial::anchor(ui.ctx(), &format!("dialogo.fala:{}", n.id), card.rect);
            let selected = self.selected == Some(n.id);
            let start = self.data.start == n.id;
            let fill = ui.visuals().panel_fill;
            painter.rect_filled(card.rect, 8.0, fill);
            let border = if selected {
                Stroke::new(2.5, accent)
            } else if start {
                Stroke::new(2.0, Color32::from_rgb(90, 200, 120))
            } else {
                Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
            };
            painter.rect_stroke(card.rect, 8.0, border, egui::StrokeKind::Inside);
            // Cabeçalho: quem fala.
            let who_color = self
                .data
                .character(&n.who)
                .map(|c| hex_color(&c.color))
                .unwrap_or(Color32::from_gray(140));
            let head = Rect::from_min_size(card.rect.min, Vec2::new(CARD_W, HEADER_H));
            painter.rect_filled(
                head.shrink(1.0),
                egui::CornerRadius {
                    nw: 8,
                    ne: 8,
                    sw: 0,
                    se: 0,
                },
                who_color.gamma_multiply(0.35),
            );
            painter.circle_filled(head.left_center() + Vec2::new(12.0, 0.0), 5.0, who_color);
            let who = if n.who.is_empty() {
                "Narrador".to_string()
            } else {
                n.who.clone()
            };
            painter.text(
                head.left_center() + Vec2::new(22.0, 0.0),
                egui::Align2::LEFT_CENTER,
                format!("{}  ·  {who}", n.id),
                egui::FontId::proportional(13.0),
                text_color,
            );
            if start {
                painter.text(
                    head.right_center() - Vec2::new(10.0, 0.0),
                    egui::Align2::RIGHT_CENTER,
                    "Início",
                    egui::FontId::proportional(11.0),
                    Color32::from_rgb(90, 200, 120),
                );
            } else if n.choices.is_empty() && n.next == 0 {
                painter.text(
                    head.right_center() - Vec2::new(14.0, 0.0),
                    egui::Align2::RIGHT_CENTER,
                    "fim",
                    egui::FontId::proportional(11.0),
                    ui.visuals().weak_text_color(),
                );
            }
            // A entrada (à esquerda).
            painter.circle_filled(
                Pos2::new(card.rect.left(), head.center().y),
                PORT_R - 2.0,
                link_color,
            );
            // O texto.
            painter.galley(
                card.rect.min + Vec2::new(10.0, HEADER_H + 6.0),
                card.text.clone(),
                text_color,
            );
            // As escolhas.
            if !n.choices.is_empty() {
                let top = card.rect.bottom() - n.choices.len() as f32 * CHOICE_H - 3.0;
                painter.line_segment(
                    [
                        Pos2::new(card.rect.left() + 6.0, top - 2.0),
                        Pos2::new(card.rect.right() - 6.0, top - 2.0),
                    ],
                    Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
                );
                for (i, c) in n.choices.iter().enumerate() {
                    let y = top + (i as f32 + 0.5) * CHOICE_H;
                    let mut t = format!("{}. {}", i + 1, c.text);
                    if t.chars().count() > 30 {
                        t = t.chars().take(29).collect::<String>() + "…";
                    }
                    painter.text(
                        Pos2::new(card.rect.left() + 12.0, y),
                        egui::Align2::LEFT_CENTER,
                        t,
                        egui::FontId::proportional(12.5),
                        accent,
                    );
                }
            }
        }

        // As saídas (por cima dos cartões, para dá para pegar).
        for &(id, choice, p) in &ports {
            let r = Rect::from_center_size(p, Vec2::splat(PORT_R * 3.0));
            let resp = ui.interact(
                r,
                Id::new(("dialogo-saida", &self.rel, id, choice)),
                Sense::drag(),
            );
            let hot = resp.hovered() || resp.dragged();
            painter.circle_filled(p, if hot { PORT_R + 1.5 } else { PORT_R }, accent);
            painter.circle_stroke(p, PORT_R, Stroke::new(1.5, ui.visuals().panel_fill));
            if choice.is_none() {
                crate::tutorial::anchor(ui.ctx(), &format!("dialogo.saida:{id}"), r);
            }
            if resp.drag_started() {
                self.drag = Some(Drag::Link { from: id, choice });
                self.selected = Some(id);
            }
            if hot {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
        }

        // Ligando: a linha segue o mouse; soltar em cima de uma fala liga,
        // soltar no vazio cria uma.
        if let Some(Drag::Link { from, choice }) = self.drag {
            let start = ports
                .iter()
                .find(|(i, c, _)| *i == from && *c == choice)
                .map(|p| p.2);
            if let (Some(s), Some(p)) = (start, pointer) {
                draw_link(&painter, s, p, accent);
            }
            if ui.input(|i| i.pointer.any_released()) {
                if let Some(p) = pointer.filter(|p| view.contains(*p)) {
                    let target = cards
                        .iter()
                        .find(|c| c.rect.contains(p) && c.id != from)
                        .map(|c| c.id);
                    let target = match target {
                        Some(t) => t,
                        None => {
                            let who = self
                                .data
                                .node(from)
                                .map(|n| n.who.clone())
                                .unwrap_or_default();
                            let at = p - origin - self.pan;
                            self.data.add_node(at.x, at.y - HEADER_H / 2.0, &who)
                        }
                    };
                    if let Some(n) = self.data.node_mut(from) {
                        match choice {
                            Some(i) => {
                                if let Some(c) = n.choices.get_mut(i) {
                                    c.next = target;
                                }
                            }
                            None => n.next = target,
                        }
                    }
                    self.selected = Some(target);
                }
                self.drag = None;
            }
        }

        // O fundo: arrastar anda pela conversa; clicar tira a seleção.
        if bg.drag_started() && self.drag.is_none() {
            self.drag = Some(Drag::Pan);
        }
        if self.drag == Some(Drag::Pan) {
            self.pan += bg.drag_delta();
        }
        if bg.clicked() && !clicked_card {
            self.selected = None;
        }
        if ui.input(|i| i.pointer.any_released())
            && matches!(self.drag, Some(Drag::Pan | Drag::Node { .. }))
        {
            self.drag = None;
        }
        // Del apaga a fala escolhida (com o mouse na tela e nenhum campo de
        // texto com o foco).
        if let Some(id) = self.selected
            && bg.hovered()
            && !ui.ctx().egui_wants_keyboard_input()
            && ui.input(|i| i.key_pressed(egui::Key::Delete))
        {
            self.data.remove_node(id);
            self.selected = None;
        }
        if self.data.nodes.is_empty() {
            painter.text(
                view.center(),
                egui::Align2::CENTER_CENTER,
                "Nenhuma fala ainda: clique em ➕ Fala.",
                egui::FontId::proportional(15.0),
                ui.visuals().weak_text_color(),
            );
        }
    }

    /// Campo de texto que agrupa as mudanças seguidas num passo de desfazer.
    fn text_field(
        ui: &mut egui::Ui,
        value: &mut String,
        hint: &str,
        multiline: bool,
        group: &mut Option<Id>,
    ) -> egui::Response {
        let edit = if multiline {
            egui::TextEdit::multiline(value).desired_rows(4)
        } else {
            egui::TextEdit::singleline(value)
        };
        let r = ui.add(edit.hint_text(hint).desired_width(f32::INFINITY));
        if r.changed() {
            *group = Some(r.id);
        }
        r
    }

    /// "Vai para": uma fala ou o fim.
    fn target_combo(
        ui: &mut egui::Ui,
        id: Id,
        data: &DialogueData,
        own: u32,
        value: &mut u32,
    ) -> bool {
        let label = |n: u32| -> String {
            if n == 0 {
                return "Fim da conversa".into();
            }
            match data.node(n) {
                Some(node) => {
                    let mut t: String = node.text.chars().take(26).collect();
                    if node.text.chars().count() > 26 {
                        t.push('…');
                    }
                    format!("{n}: {t}")
                }
                None => format!("{n} (não existe)"),
            }
        };
        let mut changed = false;
        egui::ComboBox::from_id_salt(id)
            .selected_text(label(*value))
            .width(ui.available_width() - 8.0)
            .show_ui(ui, |ui| {
                changed |= ui.selectable_value(value, 0, label(0)).changed();
                for n in &data.nodes {
                    if n.id != own {
                        changed |= ui.selectable_value(value, n.id, label(n.id)).changed();
                    }
                }
            });
        changed
    }

    fn node_panel(
        &mut self,
        ui: &mut egui::Ui,
        id: u32,
        env: &DialogueEnv,
        group: &mut Option<Id>,
    ) {
        let Some(mut n) = self.data.node(id).cloned() else {
            self.selected = None;
            return;
        };
        let names: Vec<String> = self
            .data
            .characters
            .iter()
            .map(|c| c.name.clone())
            .collect();
        ui.horizontal(|ui| {
            ui.heading(format!("Fala {id}"));
            if self.data.start == id {
                ui.colored_label(Color32::from_rgb(90, 200, 120), "(início)");
            }
        });
        ui.add_space(4.0);
        ui.label("Quem fala");
        egui::ComboBox::from_id_salt(("dialogo-quem", id))
            .selected_text(if n.who.is_empty() {
                "Narrador (sem nome)".to_string()
            } else {
                n.who.clone()
            })
            .width(ui.available_width() - 8.0)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut n.who, String::new(), "Narrador (sem nome)");
                for name in &names {
                    ui.selectable_value(&mut n.who, name.clone(), name);
                }
            })
            .response
            .on_hover_text("Os personagens ficam na conversa: clique no vazio da tela para ver.");
        ui.add_space(4.0);
        ui.label("Texto");
        let r = Self::text_field(ui, &mut n.text, "O que ele diz", true, group);
        crate::tutorial::anchor(ui.ctx(), "dialogo.texto", r.rect);
        ui.weak("{moedas} mostra o valor de uma variável do jogo.");
        ui.add_space(8.0);
        if n.choices.is_empty() {
            ui.label("Depois");
            Self::target_combo(
                ui,
                Id::new(("dialogo-depois", id)),
                &self.data,
                id,
                &mut n.next,
            );
        } else {
            ui.label(RichTextLabel::strong("Escolhas do jogador"));
            let mut remove = None;
            for (i, c) in n.choices.iter_mut().enumerate() {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}.", i + 1));
                        let r = ui.add(
                            egui::TextEdit::singleline(&mut c.text)
                                .hint_text("O que o jogador diz")
                                .desired_width(ui.available_width() - 70.0),
                        );
                        if r.changed() {
                            *group = Some(r.id);
                        }
                        if ui.small_button("Apagar").clicked() {
                            remove = Some(i);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Vai para");
                        Self::target_combo(
                            ui,
                            Id::new(("dialogo-escolha", id, i)),
                            &self.data,
                            id,
                            &mut c.next,
                        );
                    });
                    egui::CollapsingHeader::new("Mais")
                        .id_salt(("dialogo-escolha-mais", id, i))
                        .show(ui, |ui| {
                            ui.label("Muda a variável");
                            ui.horizontal(|ui| {
                                let r = ui.add(
                                    egui::TextEdit::singleline(&mut c.var)
                                        .hint_text("variável")
                                        .desired_width(110.0),
                                );
                                if r.changed() {
                                    *group = Some(r.id);
                                }
                                ui.label("para");
                                let r = ui.add(
                                    egui::TextEdit::singleline(&mut c.value)
                                        .hint_text("1")
                                        .desired_width(80.0),
                                );
                                if r.changed() {
                                    *group = Some(r.id);
                                }
                            });
                            ui.label("Só aparece com o item");
                            let r = ui.add(
                                egui::TextEdit::singleline(&mut c.need)
                                    .hint_text("chave")
                                    .desired_width(f32::INFINITY),
                            );
                            if r.changed() {
                                *group = Some(r.id);
                            }
                        });
                });
            }
            if let Some(i) = remove {
                n.choices.remove(i);
            }
        }
        let r = ui
            .button("➕ Escolha")
            .on_hover_text("O jogador escolhe o que responder; cada escolha leva a uma fala");
        crate::tutorial::anchor(ui.ctx(), "dialogo.escolha", r.rect);
        if r.clicked() {
            let next = if n.choices.is_empty() { n.next } else { 0 };
            let text = if n.choices.is_empty() { "Sim" } else { "Não" };
            n.choices.push(Choice {
                text: text.into(),
                next,
                ..Choice::default()
            });
            n.next = 0;
        }
        ui.add_space(8.0);
        egui::CollapsingHeader::new("Ao aparecer")
            .id_salt(("dialogo-ao-aparecer", id))
            .default_open(!n.var.is_empty() || !n.item.is_empty())
            .show(ui, |ui| {
                ui.label("Muda a variável");
                ui.horizontal(|ui| {
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut n.var)
                            .hint_text("variável")
                            .desired_width(110.0),
                    );
                    if r.changed() {
                        *group = Some(r.id);
                    }
                    ui.label("para");
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut n.value)
                            .hint_text("1")
                            .desired_width(80.0),
                    );
                    if r.changed() {
                        *group = Some(r.id);
                    }
                });
                ui.label("Dá o item (inventário)");
                let r = ui.add(
                    egui::TextEdit::singleline(&mut n.item)
                        .hint_text("chave")
                        .desired_width(f32::INFINITY),
                );
                if r.changed() {
                    *group = Some(r.id);
                }
            });
        let _ = env;
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.data.start != id, egui::Button::new("Começar por esta"))
                .on_hover_text("A conversa começa nesta fala")
                .clicked()
            {
                self.data.start = id;
            }
            if ui.button("Apagar fala").clicked() {
                self.data.remove_node(id);
                self.selected = None;
            }
        });
        if let Some(m) = self.data.node_mut(id)
            && *m != n
        {
            *m = n;
        }
    }

    fn conversation_panel(&mut self, ui: &mut egui::Ui, env: &DialogueEnv, group: &mut Option<Id>) {
        ui.heading("Conversa");
        ui.weak("Clique numa fala para editar. Aqui ficam os personagens e a velocidade do texto.");
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Velocidade do texto");
            let r = ui
                .add(
                    egui::DragValue::new(&mut self.data.speed)
                        .range(0.0..=1000.0)
                        .suffix(" letras/s"),
                )
                .on_hover_text("0 = o texto aparece de uma vez");
            if r.changed() {
                *group = Some(r.id);
            }
        });
        ui.add_space(8.0);
        ui.label(RichTextLabel::strong("Personagens"));
        let mut remove = None;
        let mut rename: Option<(String, String)> = None;
        for (i, c) in self.data.characters.iter_mut().enumerate() {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let mut col = hex_color(&c.color);
                    if ui.color_edit_button_srgba(&mut col).changed() {
                        c.color = color_hex(col);
                        *group = Some(Id::new(("dialogo-cor", i)));
                    }
                    let old = c.name.clone();
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut c.name)
                            .hint_text("Nome")
                            .desired_width(ui.available_width() - 70.0),
                    );
                    if r.changed() {
                        *group = Some(r.id);
                        rename = Some((old, c.name.clone()));
                    }
                    if ui.small_button("Apagar").clicked() {
                        remove = Some(i);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Retrato");
                    file_combo(
                        ui,
                        Id::new(("dialogo-retrato", i)),
                        &env.images,
                        &mut c.image,
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Voz");
                    file_combo(ui, Id::new(("dialogo-voz", i)), &env.sounds, &mut c.sound);
                })
                .response
                .on_hover_text("Um som curto que toca enquanto o texto aparece");
            });
        }
        if let Some((old, new)) = rename
            && !old.is_empty()
        {
            for n in &mut self.data.nodes {
                if n.who == old {
                    n.who = new.clone();
                }
            }
        }
        if let Some(i) = remove {
            self.data.characters.remove(i);
        }
        let r = ui.button("➕ Personagem");
        crate::tutorial::anchor(ui.ctx(), "dialogo.personagem", r.rect);
        if r.clicked() {
            let palette = ["#FFD166", "#06D6A0", "#4CC9F0", "#F78C6B", "#C77DFF"];
            let color = palette[self.data.characters.len() % palette.len()];
            let mut name = "Personagem".to_string();
            let mut k = 2;
            while self.data.character(&name).is_some() {
                name = format!("Personagem {k}");
                k += 1;
            }
            self.data.characters.push(Character {
                name,
                color: color.into(),
                ..Character::default()
            });
        }
        ui.add_space(10.0);
        ui.weak("Na cena, um objeto com o comportamento Diálogo (Fases) abre esta conversa: perto dele aparece \"E falar\". Nos eventos: Conversas > Escolher numa conversa.");
    }
}

/// Um texto em negrito (atalho).
struct RichTextLabel;
impl RichTextLabel {
    fn strong(s: &str) -> egui::RichText {
        egui::RichText::new(s).strong()
    }
}

fn file_combo(ui: &mut egui::Ui, id: Id, files: &[String], value: &mut String) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(if value.is_empty() {
            "(nenhum)".to_string()
        } else {
            value.rsplit('/').next().unwrap_or(value).to_string()
        })
        .width(ui.available_width() - 8.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(value, String::new(), "(nenhum)");
            for f in files {
                ui.selectable_value(value, f.clone(), f);
            }
        });
}

fn draw_link(painter: &egui::Painter, from: Pos2, to: Pos2, color: Color32) {
    let dx = ((to.x - from.x).abs() * 0.5).max(40.0);
    let shape = egui::epaint::CubicBezierShape::from_points_stroke(
        [from, from + Vec2::new(dx, 0.0), to - Vec2::new(dx, 0.0), to],
        false,
        Color32::TRANSPARENT,
        Stroke::new(2.0, color),
    );
    painter.add(shape);
    // A ponta da seta.
    let tip = to;
    painter.add(egui::Shape::convex_polygon(
        vec![tip, tip + Vec2::new(-8.0, -5.0), tip + Vec2::new(-8.0, 5.0)],
        color,
        Stroke::NONE,
    ));
}

pub fn hex_color(s: &str) -> Color32 {
    let c = crate::scene_render::parse_color(s);
    Color32::from_rgba_unmultiplied(
        (c.r * 255.0).round() as u8,
        (c.g * 255.0).round() as u8,
        (c.b * 255.0).round() as u8,
        (c.a * 255.0).round() as u8,
    )
}

fn color_hex(c: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grava_e_le_de_volta() {
        let d = read(&template(), "a.dialogo").unwrap();
        assert_eq!(d.nodes.len(), 3);
        assert_eq!(d.characters[0].name, "Ana");
        assert_eq!(d.nodes[1].choices.len(), 2);
        let mut e = d.clone();
        e.nodes[0].text = "Linha 1\n\"aspas\" \\ fim".into();
        e.nodes[1].choices[0].var = "ajudou".into();
        e.nodes[1].choices[0].value = "1".into();
        e.nodes[1].choices[1].value = String::new();
        e.nodes[2].var = "nome".into();
        e.nodes[2].value = "Bia".into();
        e.nodes[2].item = "chave".into();
        let text = write(&e);
        assert!(text.contains("value = 1"), "{text}");
        assert!(text.contains("value = \"Bia\""), "{text}");
        let f = read(&text, "a.dialogo").unwrap();
        assert_eq!(f, e);
    }

    #[test]
    fn apagar_fala_desliga_quem_ia_para_ela() {
        let mut d = read(&template(), "a.dialogo").unwrap();
        d.remove_node(3);
        assert_eq!(d.nodes[1].choices[0].next, 0);
        d.remove_node(1);
        assert_eq!(d.start, 2);
        let id = d.add_node(0.0, 0.0, "Ana");
        assert_eq!(id, 3);
    }

    #[test]
    fn erro_no_arquivo() {
        assert!(read("return {", "x.dialogo").is_err());
        assert!(read("return { nodes = { { text = 'a' } } }", "x.dialogo").is_err());
    }
}
