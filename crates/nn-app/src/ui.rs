//! Interface: painéis com botões, opções e barras de volume (veja [`Panel`]).

use std::collections::HashMap;

use glam::{UVec2, Vec2};
use nn_audio::SoundId;
use nn_core::Color;
use nn_input::{Device, KeyCode, MouseButton, PadButton};
use nn_render::{Anchor, FontId, Text};

use crate::Context;

/// Espera antes de repetir a navegação com a direção segurada.
const REPEAT_DELAY: f32 = 0.35;
/// Intervalo entre repetições da navegação.
const REPEAT_INTERVAL: f32 = 0.1;
/// Altura de tela para a qual o tema foi pensado (escala automática).
const REFERENCE_HEIGHT: f32 = 720.0;

/// Aparência da interface. Os tamanhos são para uma janela de 720 pixels de
/// altura e crescem ou diminuem com ela (veja [`Theme::scale`]).
#[derive(Debug, Clone)]
pub struct Theme {
    /// Fonte do texto (`None` usa a fonte padrão da engine).
    pub font: Option<FontId>,
    pub text_size: f32,
    pub title_size: f32,
    pub text_color: Color,
    /// Cor dos textos informativos ([`Panel::label`]).
    pub label_color: Color,
    /// Fundo do painel. Com alfa zero, o painel não tem fundo.
    pub panel_color: Color,
    pub button_color: Color,
    /// Fundo do item com foco.
    pub focus_color: Color,
    pub focus_text_color: Color,
    /// Preenchimento das barras.
    pub accent_color: Color,
    /// Largura padrão dos painéis.
    pub width: f32,
    pub padding: f32,
    pub spacing: f32,
    pub row_height: f32,
    /// Escala fixa. `None` (padrão) acompanha a altura da janela, para a
    /// interface ocupar o mesmo espaço em qualquer resolução.
    pub scale: Option<f32>,
    /// Textos do [`Panel::toggle`] ligado e desligado.
    pub on_text: String,
    pub off_text: String,
    /// Som ao mudar o foco ou um valor.
    pub move_sound: Option<SoundId>,
    /// Som ao confirmar (botões e opções liga/desliga).
    pub confirm_sound: Option<SoundId>,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            font: None,
            text_size: 26.0,
            title_size: 44.0,
            text_color: Color::WHITE,
            label_color: Color::hex(0xC9D1DE),
            panel_color: Color::hex(0x0E1422),
            button_color: Color::WHITE.with_alpha(0.08),
            focus_color: Color::hex(0x3FA7D6),
            focus_text_color: Color::WHITE,
            accent_color: Color::hex(0x3FA7D6),
            width: 420.0,
            padding: 24.0,
            spacing: 10.0,
            row_height: 52.0,
            scale: None,
            on_text: "Sim".into(),
            off_text: "Não".into(),
            move_sound: None,
            confirm_sound: None,
        }
    }
}

/// Posição e título de um painel.
#[derive(Debug, Clone, Default)]
pub struct PanelOptions {
    /// Título, em letras maiores no topo.
    pub title: Option<String>,
    /// Largura (na escala do tema). Padrão: [`Theme::width`].
    pub width: Option<f32>,
    /// Ponto da tela (pixels, Y para baixo) onde o painel fica. Padrão: centro.
    pub position: Option<Vec2>,
    /// Que ponto do painel fica em `position`. Padrão: o centro.
    pub anchor: Option<Anchor>,
    /// Fundo deste painel, no lugar do [`Theme::panel_color`].
    pub background: Option<Color>,
}

impl PanelOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn at(mut self, position: Vec2) -> Self {
        self.position = Some(position);
        self
    }

    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = Some(anchor);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }
}

/// Retângulo em pixels (canto superior esquerdo e tamanho).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Area {
    pos: Vec2,
    size: Vec2,
}

impl Area {
    fn contains(&self, p: Vec2) -> bool {
        p.x >= self.pos.x
            && p.y >= self.pos.y
            && p.x < self.pos.x + self.size.x
            && p.y < self.pos.y + self.size.y
    }

    fn center(&self) -> Vec2 {
        self.pos + self.size * 0.5
    }

    fn offset(self, by: Vec2) -> Self {
        Self {
            pos: self.pos + by,
            ..self
        }
    }
}

/// O que cada painel lembra entre um frame e outro.
#[derive(Debug, Default)]
struct Memory {
    focus: usize,
    /// Itens focáveis no último frame, em pixels da tela.
    items: Vec<Area>,
    last_frame: Option<u64>,
    dragging: Option<usize>,
}

#[derive(Debug, Clone, Copy, Default)]
struct NavInput {
    /// -1 sobe, 1 desce.
    vertical: i32,
    /// -1 esquerda, 1 direita.
    horizontal: i32,
    confirm: bool,
    back: bool,
}

#[derive(Debug, Default)]
struct Nav {
    frame: Option<u64>,
    held_v: i32,
    held_h: i32,
    timer_v: f32,
    timer_h: f32,
    out: NavInput,
}

#[derive(Debug)]
enum Op {
    Rect {
        area: Area,
        color: Color,
    },
    Circle {
        center: Vec2,
        radius: f32,
        color: Color,
    },
    Text {
        content: String,
        pos: Vec2,
        size: f32,
        color: Color,
        anchor: Anchor,
    },
}

/// Painel sendo montado neste frame.
#[derive(Debug)]
struct Open {
    id: String,
    scale: f32,
    width: f32,
    options: PanelOptions,
    y: f32,
    /// Espaço deixado depois do último item (sai da altura final).
    trailing: f32,
    index: usize,
    items: Vec<Area>,
    prev_items: Vec<Area>,
    ops: Vec<Op>,
    focus: usize,
    nav: NavInput,
    interactive: bool,
    clicked: Option<(usize, Vec2)>,
    drag: Option<(usize, Vec2)>,
    /// Item que começou a ser arrastado neste frame.
    start_drag: Option<usize>,
}

impl Open {
    fn activated(&self, i: usize) -> bool {
        (self.focus == i && self.nav.confirm) || self.clicked.is_some_and(|(j, _)| j == i)
    }

    fn horizontal(&self, i: usize) -> i32 {
        if self.focus == i {
            self.nav.horizontal
        } else {
            0
        }
    }

    fn text(
        &mut self,
        content: impl Into<String>,
        pos: Vec2,
        size: f32,
        color: Color,
        anchor: Anchor,
    ) {
        self.ops.push(Op::Text {
            content: content.into(),
            pos,
            size,
            color,
            anchor,
        });
    }
}

/// Estado da interface: tema, foco de cada painel e navegação.
#[derive(Debug, Default)]
pub struct Ui {
    theme: Theme,
    memory: HashMap<String, Memory>,
    open: Option<Open>,
    nav: Nav,
    disabled: bool,
}

impl Ui {
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    pub fn theme_mut(&mut self) -> &mut Theme {
        &mut self.theme
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// Liga ou desliga a entrada da interface. A pilha de cenas desliga para
    /// as cenas que estão por baixo de uma sobreposição (como a pausa).
    pub fn set_interactive(&mut self, interactive: bool) {
        self.disabled = !interactive;
    }

    pub fn is_interactive(&self) -> bool {
        !self.disabled
    }

    /// Item com foco num painel (contando a partir de 0).
    pub fn focus(&self, panel: &str) -> Option<usize> {
        self.memory.get(panel).map(|m| m.focus)
    }

    /// Muda o foco de um painel (ex.: começar no botão "Continuar").
    pub fn set_focus(&mut self, panel: &str, index: usize) {
        self.memory.entry(panel.to_owned()).or_default().focus = index;
    }

    /// Esquece o foco de um painel: na próxima vez ele começa no primeiro item.
    pub fn reset(&mut self, panel: &str) {
        self.memory.remove(panel);
    }

    /// Há um painel aberto esperando [`Panel::finish`]?
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// Nome do painel aberto, se houver.
    pub fn open_id(&self) -> Option<&str> {
        self.open.as_ref().map(|o| o.id.as_str())
    }

    /// Escala atual para uma janela de `viewport` pixels.
    pub fn scale(&self, viewport: UVec2) -> f32 {
        self.theme
            .scale
            .unwrap_or_else(|| (viewport.y as f32 / REFERENCE_HEIGHT).max(0.5))
    }
}

/// Repetição da navegação com a direção segurada.
fn repeat(held: &mut i32, timer: &mut f32, dir: i32, dt: f32, continuous: bool) -> i32 {
    if dir == 0 {
        *held = 0;
        return 0;
    }
    if dir != *held || !continuous {
        *held = dir;
        *timer = REPEAT_DELAY;
        return dir;
    }
    *timer -= dt;
    if *timer <= 0.0 {
        *timer = (*timer + REPEAT_INTERVAL).max(0.0);
        return dir;
    }
    0
}

fn anchor_fraction(anchor: Anchor) -> Vec2 {
    match anchor {
        Anchor::TopLeft => Vec2::new(0.0, 0.0),
        Anchor::TopCenter => Vec2::new(0.5, 0.0),
        Anchor::TopRight => Vec2::new(1.0, 0.0),
        Anchor::CenterLeft => Vec2::new(0.0, 0.5),
        Anchor::Center => Vec2::new(0.5, 0.5),
        Anchor::CenterRight => Vec2::new(1.0, 0.5),
        Anchor::BottomLeft => Vec2::new(0.0, 1.0),
        Anchor::BottomCenter => Vec2::new(0.5, 1.0),
        Anchor::BottomRight => Vec2::new(1.0, 1.0),
    }
}

impl Context {
    /// Abre um painel de interface. `id` identifica o painel entre frames
    /// (para lembrar o foco); use nomes diferentes para painéis diferentes.
    ///
    /// Se outro painel estiver aberto, ele é fechado (desenhado) antes.
    pub fn panel(&mut self, id: impl Into<String>, options: PanelOptions) -> Panel<'_> {
        self.finish_panel();
        self.update_ui_nav();

        let id = id.into();
        let frame = self.time.frame();
        let scale = self.ui.scale(self.viewport);
        let width = options.width.unwrap_or(self.ui.theme.width) * scale;
        let enabled = !self.ui.disabled;
        let nav = self.ui.nav.out;
        let theme_move = self.ui.theme.move_sound;

        let mem = self.ui.memory.entry(id.clone()).or_default();
        // Só responde se já estava na tela no frame anterior.
        let shown_before = mem
            .last_frame
            .is_some_and(|f| f == frame || f.wrapping_add(1) == frame);
        mem.last_frame = Some(frame);
        let interactive = enabled && shown_before;
        let count = mem.items.len();

        let mut clicked = None;
        let mut drag = None;
        let mut moved = false;
        if interactive {
            let cursor = self.input.cursor_position();
            let hit = cursor.and_then(|c| mem.items.iter().position(|a| a.contains(c)));
            if let (Some(j), Some(c)) = (hit, cursor) {
                if self.input.last_device() == Device::Mouse {
                    mem.focus = j;
                }
                if self.input.mouse_pressed(MouseButton::Left) {
                    clicked = Some((j, c));
                }
            }
            if self.input.mouse_down(MouseButton::Left) {
                if let (Some(j), Some(c)) = (mem.dragging, cursor) {
                    drag = Some((j, c));
                }
            } else {
                mem.dragging = None;
            }
            if nav.vertical != 0 && count > 0 {
                mem.focus = (mem.focus as i32 + nav.vertical).rem_euclid(count as i32) as usize;
                moved = count > 1;
            }
        }
        if count > 0 {
            mem.focus = mem.focus.min(count - 1);
        }
        let focus = mem.focus;
        let prev_items = std::mem::take(&mut mem.items);

        let padding = self.ui.theme.padding * scale;
        let mut open = Open {
            id,
            scale,
            width,
            y: padding,
            trailing: 0.0,
            index: 0,
            items: Vec::new(),
            prev_items,
            ops: Vec::new(),
            focus,
            nav: if interactive {
                nav
            } else {
                NavInput::default()
            },
            interactive,
            clicked,
            drag,
            start_drag: None,
            options,
        };
        if let Some(title) = open.options.title.clone() {
            let size = self.ui.theme.title_size * scale;
            let color = self.ui.theme.text_color;
            open.text(
                title,
                Vec2::new(width * 0.5, open.y),
                size,
                color,
                Anchor::TopCenter,
            );
            open.trailing = self.ui.theme.spacing * scale;
            open.y += size * 1.3 + open.trailing;
        }
        self.ui.open = Some(open);
        if moved {
            self.play_ui_sound(theme_move);
        }
        Panel { ctx: self }
    }

    /// O painel aberto, se houver (para continuar a montá-lo em outro trecho
    /// de código).
    pub fn open_panel(&mut self) -> Option<Panel<'_>> {
        if self.ui.open.is_some() {
            Some(Panel { ctx: self })
        } else {
            None
        }
    }

    /// Desenha e fecha o painel aberto. A engine chama isto sozinha depois do
    /// `render` de cada cena, então esquecer de fechar não some com o painel.
    pub fn finish_panel(&mut self) {
        let Some(open) = self.ui.open.take() else {
            return;
        };
        let theme = &self.ui.theme;
        let padding = theme.padding * open.scale;
        let height = open.y - open.trailing + padding;
        let size = Vec2::new(open.width, height);
        let position = open
            .options
            .position
            .unwrap_or(self.viewport.as_vec2() * 0.5);
        let anchor = open.options.anchor.unwrap_or(Anchor::Center);
        let origin = (position - size * anchor_fraction(anchor)).round();
        let background = open.options.background.unwrap_or(theme.panel_color);
        let font = theme.font;

        let mut screen = self.renderer.screen();
        if background.a > 0.0 {
            screen.draw_rect(origin, size, background);
        }
        for op in &open.ops {
            match op {
                Op::Rect { area, color } => {
                    screen.draw_rect(origin + area.pos, area.size, *color);
                }
                Op::Circle {
                    center,
                    radius,
                    color,
                } => {
                    screen.draw_circle(origin + *center, *radius, *color);
                }
                Op::Text {
                    content,
                    pos,
                    size,
                    color,
                    anchor,
                } => {
                    let mut text = Text::new(content)
                        .at(origin + *pos)
                        .size(*size)
                        .color(*color)
                        .anchor(*anchor);
                    if let Some(font) = font {
                        text = text.font(font);
                    }
                    screen.draw_text(&text);
                }
            }
        }

        let mem = self.ui.memory.entry(open.id).or_default();
        mem.items = open.items.iter().map(|a| a.offset(origin)).collect();
        if open.start_drag.is_some() {
            mem.dragging = open.start_drag;
        }
        if !mem.items.is_empty() {
            mem.focus = mem.focus.min(mem.items.len() - 1);
        }
    }

    fn update_ui_nav(&mut self) {
        let frame = self.time.frame();
        let nav = &mut self.ui.nav;
        if nav.frame == Some(frame) {
            return;
        }
        let continuous = nav.frame.is_some_and(|f| f.wrapping_add(1) == frame);
        nav.frame = Some(frame);
        let dt = self.time.delta_secs();
        let input = &self.input;
        let stick = input.left_stick();
        // Segurado agora (para repetir) e apertado neste frame (para não
        // perder um toque rápido que apertou e soltou dentro do mesmo frame).
        let dir = |keys: [KeyCode; 2], pad: PadButton, stick: bool| {
            let held = keys.iter().any(|k| input.key_down(*k)) || input.pad_down(pad) || stick;
            let tapped = keys.iter().any(|k| input.key_pressed(*k)) || input.pad_pressed(pad);
            (held, tapped)
        };
        let up = dir(
            [KeyCode::ArrowUp, KeyCode::KeyW],
            PadButton::DPadUp,
            stick.y > 0.5,
        );
        let down = dir(
            [KeyCode::ArrowDown, KeyCode::KeyS],
            PadButton::DPadDown,
            stick.y < -0.5,
        );
        let left = dir(
            [KeyCode::ArrowLeft, KeyCode::KeyA],
            PadButton::DPadLeft,
            stick.x < -0.5,
        );
        let right = dir(
            [KeyCode::ArrowRight, KeyCode::KeyD],
            PadButton::DPadRight,
            stick.x > 0.5,
        );
        let axis = |neg: (bool, bool), pos: (bool, bool)| {
            (
                i32::from(pos.0) - i32::from(neg.0),
                i32::from(pos.1) - i32::from(neg.1),
            )
        };
        let (v, v_tap) = axis(up, down);
        let (h, h_tap) = axis(left, right);
        let step = |moved: i32, tap: i32| if moved == 0 { tap } else { moved };
        nav.out = NavInput {
            vertical: step(
                repeat(&mut nav.held_v, &mut nav.timer_v, v, dt, continuous),
                v_tap,
            ),
            horizontal: step(
                repeat(&mut nav.held_h, &mut nav.timer_h, h, dt, continuous),
                h_tap,
            ),
            confirm: [KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::Space]
                .iter()
                .any(|k| input.key_pressed(*k))
                || input.pad_pressed(PadButton::South),
            back: [KeyCode::Escape, KeyCode::Backspace]
                .iter()
                .any(|k| input.key_pressed(*k))
                || input.pad_pressed(PadButton::East),
        };
    }

    fn play_ui_sound(&mut self, sound: Option<SoundId>) {
        if let Some(id) = sound {
            self.audio.play(id);
        }
    }
}

/// Um painel de interface aberto com [`Context::panel`]: botões, opções e
/// barras de volume que funcionam com mouse, teclado e controle sem nenhum
/// código extra. Os itens aparecem um embaixo do outro, na ordem em que são
/// chamados.
///
/// A interface é "imediata": a cada frame o jogo descreve o painel e já
/// recebe o resultado (o botão foi apertado? o valor mudou?). Não há objetos
/// para criar nem eventos para registrar.
///
/// ```no_run
/// use nn_app::{Context, PanelOptions};
///
/// struct Opcoes { volume: f32, tela_cheia: bool }
///
/// fn desenhar_menu(ctx: &mut Context, opcoes: &mut Opcoes) {
///     let mut menu = ctx.panel("menu", PanelOptions::new().title("Meu jogo"));
///     if menu.button("Jogar") {
///         // começar
///     }
///     menu.slider("Volume", &mut opcoes.volume, 0.0, 1.0, 0.1);
///     menu.toggle("Tela cheia", &mut opcoes.tela_cheia);
///     if menu.button("Sair") {
///         menu.ctx().exit();
///     }
///     menu.finish();
/// }
/// ```
///
/// Navegação: setas, WASD, direcional ou analógico mudam o foco (cima e
/// baixo) e o valor (esquerda e direita); Enter, Espaço ou o botão de baixo
/// do controle (A no Xbox) confirmam; o mouse foca ao passar por cima e
/// confirma com clique. [`Panel::back`] avisa quando o jogador pede para
/// voltar (Esc, Backspace ou o botão da direita do controle).
///
/// Para não reagir a um aperto que já foi usado para abrir o painel (como o
/// Esc que abriu a pausa), um painel ignora a entrada no frame em que aparece.
#[derive(Debug)]
pub struct Panel<'a> {
    ctx: &'a mut Context,
}

/// Uma linha do painel recém-criada.
struct Row {
    index: usize,
    area: Area,
    focused: bool,
}

impl Panel<'_> {
    /// Acesso ao contexto com o painel aberto.
    pub fn ctx(&mut self) -> &mut Context {
        self.ctx
    }

    fn open(&mut self) -> &mut Open {
        self.ctx
            .ui
            .open
            .as_mut()
            .expect("o painel foi fechado (finish) e não pode mais receber itens")
    }

    fn row(&mut self) -> Row {
        let theme = &self.ctx.ui.theme;
        let (padding, spacing, height) = (theme.padding, theme.spacing, theme.row_height);
        let open = self.open();
        let s = open.scale;
        let area = Area {
            pos: Vec2::new(padding * s, open.y),
            size: Vec2::new(open.width - 2.0 * padding * s, height * s),
        };
        open.trailing = spacing * s;
        open.y += area.size.y + open.trailing;
        let index = open.index;
        open.index += 1;
        open.items.push(area);
        Row {
            index,
            area,
            focused: open.focus == index,
        }
    }

    /// Fundo da linha e cor do texto conforme o foco.
    fn row_background(&mut self, row: &Row) -> Color {
        let theme = &self.ctx.ui.theme;
        let (bg, fg) = if row.focused {
            (theme.focus_color, theme.focus_text_color)
        } else {
            (theme.button_color, theme.text_color)
        };
        self.open().ops.push(Op::Rect {
            area: row.area,
            color: bg,
        });
        fg
    }

    fn text_size(&self) -> f32 {
        let s = self.ctx.ui.open.as_ref().map_or(1.0, |o| o.scale);
        self.ctx.ui.theme.text_size * s
    }

    fn inner_padding(&self) -> f32 {
        let s = self.ctx.ui.open.as_ref().map_or(1.0, |o| o.scale);
        16.0 * s
    }

    /// Texto à esquerda e valor à direita, centralizados na altura da linha.
    fn label_and_value(&mut self, row: &Row, label: &str, value: Option<&str>, color: Color) {
        let size = self.text_size();
        let inner = self.inner_padding();
        let mid = row.area.center().y;
        let left = Vec2::new(row.area.pos.x + inner, mid);
        let right = Vec2::new(row.area.pos.x + row.area.size.x - inner, mid);
        let open = self.open();
        open.text(label, left, size, color, Anchor::CenterLeft);
        if let Some(value) = value {
            open.text(value, right, size, color, Anchor::CenterRight);
        }
    }

    fn sound(&mut self, confirm: bool) {
        let theme = &self.ctx.ui.theme;
        let sound = if confirm {
            theme.confirm_sound
        } else {
            theme.move_sound
        };
        self.ctx.play_ui_sound(sound);
    }

    /// Botão. Devolve `true` no frame em que foi acionado.
    pub fn button(&mut self, label: &str) -> bool {
        let row = self.row();
        let fg = self.row_background(&row);
        let size = self.text_size();
        let center = row.area.center();
        self.open().text(label, center, size, fg, Anchor::Center);
        let pressed = self.open().activated(row.index);
        if pressed {
            self.sound(true);
        }
        pressed
    }

    /// Opção liga/desliga, mostrando [`Theme::on_text`] ou [`Theme::off_text`].
    /// Devolve `true` se o valor mudou.
    pub fn toggle(&mut self, label: &str, value: &mut bool) -> bool {
        let theme = &self.ctx.ui.theme;
        let (on, off) = (theme.on_text.clone(), theme.off_text.clone());
        self.toggle_with(label, value, &on, &off)
    }

    /// Como [`Panel::toggle`], com os textos de ligado e desligado.
    pub fn toggle_with(&mut self, label: &str, value: &mut bool, on: &str, off: &str) -> bool {
        let row = self.row();
        let open = self.open();
        let changed = open.activated(row.index) || open.horizontal(row.index) != 0;
        if changed {
            *value = !*value;
            self.sound(true);
        }
        let fg = self.row_background(&row);
        let shown = if *value { on } else { off };
        self.label_and_value(&row, label, Some(shown), fg);
        changed
    }

    /// Escolha entre opções com esquerda e direita (ou clique: no terço
    /// esquerdo volta, no resto avança). Devolve `true` se mudou.
    pub fn choice<S: AsRef<str>>(&mut self, label: &str, options: &[S], index: &mut usize) -> bool {
        let row = self.row();
        let n = options.len();
        let mut changed = false;
        if n > 0 {
            let open = self.open();
            let mut step = open.horizontal(row.index);
            if open.focus == row.index && open.nav.confirm {
                step = 1;
            }
            if let Some((j, c)) = open.clicked
                && j == row.index
            {
                let prev = open.prev_items.get(row.index).copied();
                let left_third = prev.is_some_and(|a| c.x < a.pos.x + a.size.x / 3.0);
                step = if left_third { -1 } else { 1 };
            }
            let old = (*index).min(n - 1);
            let new = (old as i32 + step).rem_euclid(n as i32) as usize;
            *index = new;
            changed = new != old;
            if changed {
                self.sound(false);
            }
        }
        let fg = self.row_background(&row);
        let shown = options
            .get(*index)
            .map(|o| format!("‹ {} ›", o.as_ref()))
            .unwrap_or_default();
        self.label_and_value(&row, label, Some(&shown), fg);
        changed
    }

    /// Barra de valor entre `min` e `max`, mudando de `step` em `step` pelas
    /// setas ou arrastando com o mouse. Valores entre 0 e 1 aparecem em
    /// porcentagem. Devolve `true` se mudou.
    pub fn slider(&mut self, label: &str, value: &mut f32, min: f32, max: f32, step: f32) -> bool {
        let row = self.row();
        let (lo, hi) = if min <= max { (min, max) } else { (max, min) };
        let step = if step > 0.0 { step } else { (hi - lo) / 10.0 };
        let snap = |v: f32| {
            let snapped = if step > 0.0 {
                lo + ((v - lo) / step).round() * step
            } else {
                v
            };
            snapped.clamp(lo, hi)
        };
        let old = value.clamp(lo, hi);
        let mut new = old;
        let (bar_x0, bar_x1) = self.slider_bar(row.area);

        let open = self.open();
        let horizontal = open.horizontal(row.index);
        if horizontal != 0 {
            new = snap(old + horizontal as f32 * step);
        }
        if let Some(prev) = open.prev_items.get(row.index).copied() {
            // Mesma geometria do frame anterior, que é o que o jogador viu.
            let offset = prev.pos.x - row.area.pos.x;
            let (x0, x1) = (bar_x0 + offset, bar_x1 + offset);
            let grab = 14.0 * open.scale;
            let mut pointer = open.drag.filter(|(j, _)| *j == row.index).map(|(_, c)| c);
            // Clique só mexe no valor se for em cima da barra (no texto, só foca).
            if let Some((j, c)) = open.clicked
                && j == row.index
                && c.x >= x0 - grab
                && c.x <= x1 + grab
            {
                pointer = Some(c);
                open.start_drag = Some(row.index);
            }
            if let Some(c) = pointer
                && x1 > x0
            {
                let t = ((c.x - x0) / (x1 - x0)).clamp(0.0, 1.0);
                new = snap(lo + t * (hi - lo));
            }
        }
        let changed = (new - old).abs() > f32::EPSILON;
        *value = if changed { new } else { old };
        if changed && horizontal != 0 {
            self.sound(false);
        }

        let fg = self.row_background(&row);
        let shown = if lo == 0.0 && hi == 1.0 {
            format!("{:.0}%", *value * 100.0)
        } else if step >= 1.0 && step.fract() == 0.0 {
            format!("{:.0}", *value)
        } else {
            format!("{:.1}", *value)
        };
        self.label_and_value(&row, label, Some(&shown), fg);

        let s = self.ctx.ui.open.as_ref().map_or(1.0, |o| o.scale);
        let theme = &self.ctx.ui.theme;
        let track = fg.with_alpha(0.3);
        let fill = if row.focused {
            theme.focus_text_color
        } else {
            theme.accent_color
        };
        let t = if hi > lo {
            (*value - lo) / (hi - lo)
        } else {
            0.0
        };
        let mid = row.area.center().y;
        let thickness = 6.0 * s;
        let knob = 9.0 * s;
        let fill_x = bar_x0 + (bar_x1 - bar_x0) * t;
        let open = self.open();
        open.ops.push(Op::Rect {
            area: Area {
                pos: Vec2::new(bar_x0, mid - thickness * 0.5),
                size: Vec2::new(bar_x1 - bar_x0, thickness),
            },
            color: track,
        });
        open.ops.push(Op::Rect {
            area: Area {
                pos: Vec2::new(bar_x0, mid - thickness * 0.5),
                size: Vec2::new(fill_x - bar_x0, thickness),
            },
            color: fill,
        });
        open.ops.push(Op::Circle {
            center: Vec2::new(fill_x, mid),
            radius: knob,
            color: fill,
        });
        changed
    }

    /// Onde fica a barra do slider dentro da linha (x inicial e final).
    fn slider_bar(&self, area: Area) -> (f32, f32) {
        let s = self.ctx.ui.open.as_ref().map_or(1.0, |o| o.scale);
        let x0 = area.pos.x + area.size.x * 0.45;
        let x1 = area.pos.x + area.size.x - 16.0 * s - 70.0 * s;
        (x0, x1.max(x0))
    }

    /// Texto informativo, centralizado e sem foco.
    pub fn label(&mut self, text: &str) {
        let color = self.ctx.ui.theme.label_color;
        self.label_with(text, color, 1.0);
    }

    /// Texto com cor e tamanho próprios (`size` multiplica o tamanho do tema).
    pub fn label_with(&mut self, text: &str, color: Color, size: f32) {
        let theme = &self.ctx.ui.theme;
        let spacing = theme.spacing;
        let base = theme.text_size;
        let font = theme.font;
        let s = self.ctx.ui.open.as_ref().map_or(1.0, |o| o.scale);
        let size = base * size * s;
        let measured = {
            let mut t = Text::new(text).size(size);
            if let Some(font) = font {
                t = t.font(font);
            }
            self.ctx.renderer.measure_text(&t)
        };
        let open = self.open();
        let pos = Vec2::new(open.width * 0.5, open.y);
        open.text(text, pos, size, color, Anchor::TopCenter);
        open.trailing = spacing * s;
        open.y += measured.y.max(size) + open.trailing;
    }

    /// Espaço vazio (na escala do tema).
    pub fn space(&mut self, height: f32) {
        let open = self.open();
        open.y += height * open.scale;
        open.trailing = 0.0;
    }

    /// O jogador pediu para voltar neste frame (Esc, Backspace ou o botão
    /// da direita do controle).
    pub fn back(&mut self) -> bool {
        let open = self.open();
        open.interactive && open.nav.back
    }

    /// Índice do item com foco (contando a partir de 0).
    pub fn focus(&mut self) -> usize {
        self.open().focus
    }

    /// Desenha e fecha o painel.
    pub fn finish(self) {
        self.ctx.finish_panel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeticao_da_navegacao() {
        let (mut held, mut timer) = (0, 0.0);
        // Aperto: move na hora.
        assert_eq!(repeat(&mut held, &mut timer, 1, 0.016, true), 1);
        // Segurando: espera o atraso inicial.
        let mut moves = 0;
        let mut t = 0.0;
        while t < REPEAT_DELAY - 0.02 {
            moves += repeat(&mut held, &mut timer, 1, 0.016, true);
            t += 0.016;
        }
        assert_eq!(moves, 0);
        // Depois repete em intervalos.
        let mut moves = 0;
        for _ in 0..60 {
            moves += repeat(&mut held, &mut timer, 1, 0.016, true);
        }
        assert!((8..=11).contains(&moves), "{moves}");
        // Soltar e apertar de novo move na hora.
        assert_eq!(repeat(&mut held, &mut timer, 0, 0.016, true), 0);
        assert_eq!(repeat(&mut held, &mut timer, -1, 0.016, true), -1);
    }

    #[test]
    fn area_contem_ponto() {
        let a = Area {
            pos: Vec2::new(10.0, 10.0),
            size: Vec2::new(20.0, 10.0),
        };
        assert!(a.contains(Vec2::new(10.0, 10.0)));
        assert!(a.contains(Vec2::new(29.9, 19.9)));
        assert!(!a.contains(Vec2::new(30.0, 15.0)));
        assert_eq!(a.center(), Vec2::new(20.0, 15.0));
    }
}
