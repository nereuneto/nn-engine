//! Editor de código Lua: cores, números de linha, erros de sintaxe enquanto
//! se digita, buscar e substituir, ir para a linha e recuo automático.

use std::ops::Range;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Color32, FontId, Key, Modifiers, Pos2, Rect, RichText, Stroke, TextFormat, Vec2,
    text::{ByteIndex, CCursor, CCursorRange, LayoutJob},
};

use crate::complete::{self, Completion};
use crate::lua_syntax::{self, SyntaxIssue, TokenKind};

/// Recuo usado pelo editor (Tab).
pub const INDENT: &str = "  ";

/// Espera depois de digitar antes de conferir a sintaxe.
const CHECK_DELAY: Duration = Duration::from_millis(300);

#[derive(Debug, Default)]
pub struct FindState {
    pub open: bool,
    pub replace: bool,
    pub query: String,
    pub replacement: String,
    pub match_case: bool,
    /// Índice da ocorrência atual.
    current: usize,
    focus: bool,
}

#[derive(Debug)]
pub struct CodeDoc {
    pub rel: String,
    pub text: String,
    saved: String,
    crlf: bool,
    /// O arquivo mudou no disco enquanto havia alterações não salvas aqui.
    pub external_change: bool,
    pub issue: Option<SyntaxIssue>,
    checked: Option<String>,
    last_edit: Instant,
    pub find: FindState,
    goto_open: bool,
    goto_text: String,
    goto_focus: bool,
    /// Linha para onde ir no próximo quadro (a partir de 1).
    jump: Option<usize>,
    /// Trecho (em caracteres) a selecionar no próximo quadro.
    select: Option<Range<usize>>,
    /// Linha e coluna do cursor (a partir de 1).
    pub cursor: (usize, usize),
    /// Sugestões abertas (autocompletar).
    pub suggest: Option<Suggest>,
    /// Posição do cursor (em bytes) no quadro anterior.
    last_cursor: Option<usize>,
}

/// Lista de sugestões aberta.
#[derive(Debug, Clone)]
pub struct Suggest {
    pub completion: Completion,
    pub selected: usize,
}

/// Pedido do editor para quem o contém.
#[derive(Debug, Clone, PartialEq)]
pub enum CodeAction {
    Save,
    /// Descartar as alterações e ler o arquivo do disco.
    Reload,
}

/// Cores do código.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub keyword: Color32,
    pub builtin: Color32,
    pub literal: Color32,
    pub number: Color32,
    pub string: Color32,
    pub comment: Color32,
    pub function: Color32,
    pub operator: Color32,
    pub name: Color32,
}

impl Palette {
    pub fn for_theme(dark: bool) -> Self {
        let c = Color32::from_rgb;
        if dark {
            Self {
                keyword: c(0xC5, 0x86, 0xC0),
                builtin: c(0x4E, 0xC9, 0xB0),
                literal: c(0x56, 0x9C, 0xD6),
                number: c(0xB5, 0xCE, 0xA8),
                string: c(0xCE, 0x91, 0x78),
                comment: c(0x6A, 0x99, 0x55),
                function: c(0xDC, 0xDC, 0xAA),
                operator: c(0xC8, 0xC8, 0xC8),
                name: c(0x9C, 0xDC, 0xFE),
            }
        } else {
            Self {
                keyword: c(0xAF, 0x00, 0xDB),
                builtin: c(0x26, 0x7F, 0x99),
                literal: c(0x00, 0x00, 0xFF),
                number: c(0x09, 0x86, 0x58),
                string: c(0xA3, 0x15, 0x15),
                comment: c(0x00, 0x80, 0x00),
                function: c(0x79, 0x5E, 0x26),
                operator: c(0x30, 0x30, 0x30),
                name: c(0x00, 0x10, 0x80),
            }
        }
    }

    fn color(&self, kind: TokenKind) -> Color32 {
        match kind {
            TokenKind::Keyword => self.keyword,
            TokenKind::Builtin => self.builtin,
            TokenKind::Literal => self.literal,
            TokenKind::Number => self.number,
            TokenKind::String => self.string,
            TokenKind::Comment => self.comment,
            TokenKind::Function => self.function,
            TokenKind::Operator | TokenKind::Space => self.operator,
            TokenKind::Name => self.name,
        }
    }
}

/// Monta o texto colorido. `marks` são trechos (bytes) com fundo destacado.
pub fn highlight(
    text: &str,
    font: &FontId,
    palette: &Palette,
    marks: &[(Range<usize>, Color32)],
) -> LayoutJob {
    let mut job = LayoutJob {
        text: text.to_string(),
        ..Default::default()
    };
    job.wrap.max_width = f32::INFINITY;
    let mut push = |range: Range<usize>, color: Color32, italics: bool| {
        if range.is_empty() {
            return;
        }
        // Divide o pedaço onde começam/terminam marcações de fundo.
        let mut cuts = vec![range.start, range.end];
        for (m, _) in marks {
            for p in [m.start, m.end] {
                if p > range.start && p < range.end {
                    cuts.push(p);
                }
            }
        }
        cuts.sort_unstable();
        cuts.dedup();
        for w in cuts.windows(2) {
            let bg = marks
                .iter()
                .rev()
                .find(|(m, _)| m.start <= w[0] && w[1] <= m.end)
                .map(|(_, c)| *c)
                .unwrap_or(Color32::TRANSPARENT);
            job.sections.push(egui::text::LayoutSection {
                leading_space: 0.0,
                byte_range: ByteIndex(w[0])..ByteIndex(w[1]),
                format: TextFormat {
                    font_id: font.clone(),
                    color,
                    italics,
                    background: bg,
                    ..Default::default()
                },
            });
        }
    };
    for (range, kind) in lua_syntax::tokenize(text) {
        push(range, palette.color(kind), kind == TokenKind::Comment);
    }
    if text.is_empty() {
        job.sections.push(egui::text::LayoutSection {
            leading_space: 0.0,
            byte_range: ByteIndex(0)..ByteIndex(0),
            format: TextFormat {
                font_id: font.clone(),
                color: palette.operator,
                ..Default::default()
            },
        });
    }
    job
}

/// Ocorrências de `query` em `text` (trechos em bytes).
pub fn find_all(text: &str, query: &str, match_case: bool) -> Vec<Range<usize>> {
    if query.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let q = query.as_bytes();
    let t = text.as_bytes();
    let mut i = 0;
    while i + q.len() <= t.len() {
        let window = &t[i..i + q.len()];
        let found = if match_case {
            window == q
        } else {
            window.eq_ignore_ascii_case(q)
        };
        if found && text.is_char_boundary(i) && text.is_char_boundary(i + q.len()) {
            out.push(i..i + q.len());
            i += q.len();
        } else {
            i += 1;
        }
    }
    out
}

/// Byte → índice de caractere.
pub fn char_index(text: &str, byte: usize) -> usize {
    let mut byte = byte.min(text.len());
    while !text.is_char_boundary(byte) {
        byte -= 1;
    }
    text[..byte].chars().count()
}

/// Índice de caractere → byte.
pub fn byte_index(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map(|(i, _)| i)
        .unwrap_or(text.len())
}

/// Byte onde começa a linha `line` (a partir de 1).
pub fn line_start(text: &str, line: usize) -> usize {
    if line <= 1 {
        return 0;
    }
    text.match_indices('\n')
        .nth(line - 2)
        .map(|(i, _)| i + 1)
        .unwrap_or(text.len())
}

/// Linha e coluna (a partir de 1) de um byte.
pub fn line_col(text: &str, byte: usize) -> (usize, usize) {
    let mut byte = byte.min(text.len());
    while !text.is_char_boundary(byte) {
        byte -= 1;
    }
    let before = &text[..byte];
    let line = before.matches('\n').count() + 1;
    let col = before
        .rsplit_once('\n')
        .map(|(_, l)| l)
        .unwrap_or(before)
        .chars()
        .count()
        + 1;
    (line, col)
}

/// Edição feita pelo próprio editor (não pelo campo de texto).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edit {
    /// Enter com recuo automático.
    Newline,
    Indent,
    Unindent,
    ToggleComment,
}

/// Aplica uma edição. `sel` é a seleção em bytes (início, fim); devolve a
/// seleção nova (em bytes).
pub fn apply_edit(text: &mut String, sel: Range<usize>, edit: Edit) -> Range<usize> {
    let (start, end) = (sel.start.min(text.len()), sel.end.min(text.len()));
    match edit {
        Edit::Newline => {
            let ls = text[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
            // A linha até o cursor decide o recuo (Enter no meio da linha).
            let indent = lua_syntax::next_indent(&text[ls..start], INDENT);
            let insert = format!("\n{indent}");
            text.replace_range(start..end, &insert);
            let p = start + insert.len();
            p..p
        }
        Edit::Indent if start == end || !text[start..end].contains('\n') => {
            text.replace_range(start..end, INDENT);
            let p = start + INDENT.len();
            p..p
        }
        Edit::Indent | Edit::Unindent | Edit::ToggleComment => {
            let ls = text[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
            // Seleção que termina no começo de uma linha não inclui essa linha.
            let end_adj = if end > start && text[..end].ends_with('\n') {
                end - 1
            } else {
                end
            };
            let le = text[end_adj..]
                .find('\n')
                .map(|i| end_adj + i)
                .unwrap_or(text.len());
            let block = text[ls..le].to_string();
            let lines: Vec<&str> = block.split('\n').collect();
            let new_lines: Vec<String> = match edit {
                Edit::Indent => lines
                    .iter()
                    .map(|l| {
                        if l.trim().is_empty() {
                            l.to_string()
                        } else {
                            format!("{INDENT}{l}")
                        }
                    })
                    .collect(),
                Edit::Unindent => lines
                    .iter()
                    .map(|l| {
                        l.strip_prefix(INDENT)
                            .or_else(|| l.strip_prefix('\t'))
                            .or_else(|| l.strip_prefix(' '))
                            .unwrap_or(l)
                            .to_string()
                    })
                    .collect(),
                _ => {
                    let all_commented = lines
                        .iter()
                        .filter(|l| !l.trim().is_empty())
                        .all(|l| l.trim_start().starts_with("--"));
                    let min_indent = lines
                        .iter()
                        .filter(|l| !l.trim().is_empty())
                        .map(|l| l.len() - l.trim_start().len())
                        .min()
                        .unwrap_or(0);
                    lines
                        .iter()
                        .map(|l| {
                            if l.trim().is_empty() {
                                l.to_string()
                            } else if all_commented {
                                let i = l.len() - l.trim_start().len();
                                let rest = &l[i..];
                                let rest = rest
                                    .strip_prefix("-- ")
                                    .or_else(|| rest.strip_prefix("--"))
                                    .unwrap_or(rest);
                                format!("{}{rest}", &l[..i])
                            } else {
                                format!("{}-- {}", &l[..min_indent], &l[min_indent..])
                            }
                        })
                        .collect()
                }
            };
            let new_block = new_lines.join("\n");
            text.replace_range(ls..le, &new_block);
            if start == end {
                // Cursor sem seleção: acompanha a mudança da própria linha.
                let delta = new_block.len() as isize - block.len() as isize;
                let p = (start as isize + delta).clamp(ls as isize, (ls + new_block.len()) as isize)
                    as usize;
                p..p
            } else {
                ls..ls + new_block.len()
            }
        }
    }
}

impl CodeDoc {
    pub fn open(root: &Path, rel: &str) -> Result<Self, String> {
        let bytes = std::fs::read(root.join(rel)).map_err(|e| e.to_string())?;
        let raw = String::from_utf8(bytes).map_err(|_| {
            "o arquivo não está em UTF-8; abra no editor externo e salve como UTF-8".to_string()
        })?;
        let crlf = raw.contains("\r\n");
        let text = raw.replace("\r\n", "\n");
        Ok(Self {
            rel: rel.to_string(),
            saved: text.clone(),
            text,
            crlf,
            external_change: false,
            issue: None,
            checked: None,
            last_edit: Instant::now() - CHECK_DELAY,
            find: FindState::default(),
            goto_open: false,
            goto_text: String::new(),
            goto_focus: false,
            jump: None,
            select: None,
            cursor: (1, 1),
            suggest: None,
            last_cursor: None,
        })
    }

    pub fn is_dirty(&self) -> bool {
        self.text != self.saved
    }

    pub fn is_lua(&self) -> bool {
        self.rel.ends_with(".lua")
    }

    pub fn save(&mut self, root: &Path) -> Result<(), String> {
        let content = if self.crlf {
            self.text.replace('\n', "\r\n")
        } else {
            self.text.clone()
        };
        crate::files::write_atomic(&root.join(&self.rel), &content).map_err(|e| e.to_string())?;
        self.saved = self.text.clone();
        self.external_change = false;
        Ok(())
    }

    /// Lê o arquivo de novo (descarta alterações não salvas).
    pub fn reload(&mut self, root: &Path) -> Result<(), String> {
        let fresh = Self::open(root, &self.rel)?;
        self.text = fresh.text;
        self.saved = fresh.saved;
        self.crlf = fresh.crlf;
        self.external_change = false;
        self.checked = None;
        Ok(())
    }

    /// O arquivo mudou no disco: sem alterações aqui, recarrega; com
    /// alterações, avisa e deixa o usuário escolher.
    pub fn disk_changed(&mut self, root: &Path) {
        let Ok(on_disk) = std::fs::read_to_string(root.join(&self.rel)) else {
            return;
        };
        let on_disk = on_disk.replace("\r\n", "\n");
        if on_disk == self.saved {
            return;
        }
        if self.is_dirty() {
            if on_disk != self.text {
                self.external_change = true;
            }
        } else {
            self.text = on_disk.clone();
            self.saved = on_disk;
            self.checked = None;
        }
    }

    /// Vai para uma linha (a partir de 1) no próximo quadro.
    pub fn go_to_line(&mut self, line: usize) {
        self.jump = Some(line.max(1));
    }

    /// Abre a barra de busca. O código perde o foco já, para nada do que for
    /// digitado em seguida cair nele.
    pub fn open_find(&mut self, ctx: &egui::Context, replace: bool, selection: Option<String>) {
        ctx.memory_mut(|m| m.surrender_focus(self.editor_id()));
        self.find.open = true;
        self.find.replace = replace;
        self.find.focus = true;
        if let Some(s) = selection.filter(|s| !s.is_empty() && !s.contains('\n')) {
            self.find.query = s;
        }
        self.goto_open = false;
    }

    pub fn open_goto(&mut self, ctx: &egui::Context) {
        ctx.memory_mut(|m| m.surrender_focus(self.editor_id()));
        self.goto_open = true;
        self.goto_focus = true;
        self.goto_text.clear();
        self.find.open = false;
    }

    pub fn editor_id(&self) -> egui::Id {
        egui::Id::new(("codigo", &self.rel))
    }

    fn find_id(&self) -> egui::Id {
        egui::Id::new(("buscar", &self.rel))
    }

    fn goto_id(&self) -> egui::Id {
        egui::Id::new(("ir-para-linha", &self.rel))
    }

    /// Texto selecionado no editor.
    pub fn selected_text(&self, ctx: &egui::Context) -> Option<String> {
        let state = egui::TextEdit::load_state(ctx, self.editor_id())?;
        let r = state.cursor.char_range()?;
        let (a, b) = (
            r.primary.index.0.min(r.secondary.index.0),
            r.primary.index.0.max(r.secondary.index.0),
        );
        let (a, b) = (byte_index(&self.text, a), byte_index(&self.text, b));
        Some(self.text[a..b].to_string())
    }

    /// Confere a sintaxe, se o texto mudou e a pessoa parou de digitar.
    fn check_syntax(&mut self, lua: &mlua::Lua, ctx: &egui::Context) {
        if !self.is_lua() || self.checked.as_deref() == Some(self.text.as_str()) {
            return;
        }
        let wait = CHECK_DELAY.saturating_sub(self.last_edit.elapsed());
        if !wait.is_zero() {
            ctx.request_repaint_after(wait);
            return;
        }
        self.issue = lua_syntax::check(lua, &self.rel, &self.text);
        self.checked = Some(self.text.clone());
    }

    /// Verifica agora (ao abrir e ao salvar).
    pub fn check_now(&mut self, lua: &mlua::Lua) {
        if self.is_lua() {
            self.issue = lua_syntax::check(lua, &self.rel, &self.text);
            self.checked = Some(self.text.clone());
        }
    }

    fn apply(&mut self, ctx: &egui::Context, edit: Edit) {
        let id = self.editor_id();
        let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
        let range = state
            .cursor
            .char_range()
            .unwrap_or_else(|| CCursorRange::one(CCursor::new(self.text.chars().count())));
        let (a, b) = (
            range.primary.index.0.min(range.secondary.index.0),
            range.primary.index.0.max(range.secondary.index.0),
        );
        let sel = byte_index(&self.text, a)..byte_index(&self.text, b);
        let new = apply_edit(&mut self.text, sel, edit);
        let (na, nb) = (
            char_index(&self.text, new.start),
            char_index(&self.text, new.end),
        );
        state
            .cursor
            .set_char_range(Some(CCursorRange::two(CCursor::new(na), CCursor::new(nb))));
        state.store(ctx, id);
        self.last_edit = Instant::now();
    }

    /// Posição do cursor em bytes (do estado do campo de texto).
    fn cursor_byte(&self, ctx: &egui::Context) -> Option<usize> {
        let state = egui::TextEdit::load_state(ctx, self.editor_id())?;
        let r = state.cursor.char_range()?;
        Some(byte_index(&self.text, r.primary.index.0))
    }

    /// Põe o cursor em `byte`.
    fn set_cursor(&self, ctx: &egui::Context, byte: usize) {
        let id = self.editor_id();
        let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
        let c = char_index(&self.text, byte);
        state
            .cursor
            .set_char_range(Some(CCursorRange::one(CCursor::new(c))));
        state.store(ctx, id);
    }

    /// Aceita a sugestão escolhida.
    fn accept(&mut self, ctx: &egui::Context) {
        let Some(sug) = self.suggest.take() else {
            return;
        };
        let Some(item) = sug.completion.items.get(sug.selected) else {
            return;
        };
        let (text, cursor) = complete::apply(&self.text, &sug.completion, item);
        self.text = text;
        self.set_cursor(ctx, cursor);
        self.last_cursor = Some(cursor);
        self.last_edit = Instant::now();
    }

    /// Abre (ou atualiza) as sugestões no cursor.
    pub fn refresh_suggest(&mut self, ctx: &egui::Context, env: &complete::Env, forced: bool) {
        let Some(cursor) = self.cursor_byte(ctx) else {
            self.suggest = None;
            return;
        };
        let keep = self
            .suggest
            .as_ref()
            .and_then(|s| s.completion.items.get(s.selected))
            .map(|i| i.label.clone());
        self.suggest = complete::complete(&self.text, cursor, env, forced).map(|completion| {
            let selected = keep
                .and_then(|k| completion.items.iter().position(|i| i.label == k))
                .unwrap_or(0);
            Suggest {
                completion,
                selected,
            }
        });
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        lua: &mlua::Lua,
        font_size: f32,
        env: Option<&complete::Env>,
    ) -> Vec<CodeAction> {
        let mut actions = Vec::new();
        let id = self.editor_id();
        let dark = ui.visuals().dark_mode;
        let palette = Palette::for_theme(dark);
        let font = FontId::monospace(font_size);

        if self.external_change {
            egui::Frame::new()
                .fill(Color32::from_rgb(0x5A, 0x45, 0x10).gamma_multiply(if dark {
                    1.0
                } else {
                    0.35
                }))
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("⚠ Este arquivo foi alterado fora do NN Studio.");
                        if ui.button("Usar a versão do disco").clicked() {
                            actions.push(CodeAction::Reload);
                        }
                        if ui.button("Manter a minha").clicked() {
                            self.external_change = false;
                        }
                    });
                });
        }

        // Atalhos do editor, antes do campo de texto ver as teclas.
        let has_focus = ui.memory(|m| m.has_focus(id));
        if !has_focus {
            self.suggest = None;
        }
        let mut forced = false;
        if has_focus && env.is_some() {
            if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Space)) {
                forced = true;
            }
            if let Some(sug) = &mut self.suggest {
                let n = sug.completion.items.len().max(1);
                let (up, down, accept, close) = ui.input_mut(|i| {
                    (
                        i.consume_key(Modifiers::NONE, Key::ArrowUp),
                        i.consume_key(Modifiers::NONE, Key::ArrowDown),
                        i.consume_key(Modifiers::NONE, Key::Enter)
                            || i.consume_key(Modifiers::NONE, Key::Tab),
                        i.consume_key(Modifiers::NONE, Key::Escape),
                    )
                });
                if up {
                    sug.selected = (sug.selected + n - 1) % n;
                }
                if down {
                    sug.selected = (sug.selected + 1) % n;
                }
                if close {
                    self.suggest = None;
                } else if accept {
                    self.accept(ui.ctx());
                }
            }
        }
        if has_focus {
            let edit = ui.input_mut(|i| {
                if i.consume_key(Modifiers::COMMAND, Key::Slash) {
                    Some(Edit::ToggleComment)
                } else if i.consume_key(Modifiers::SHIFT, Key::Tab) {
                    Some(Edit::Unindent)
                } else if i.consume_key(Modifiers::NONE, Key::Tab) {
                    Some(Edit::Indent)
                } else if !i.modifiers.shift && i.consume_key(Modifiers::NONE, Key::Enter) {
                    Some(Edit::Newline)
                } else {
                    None
                }
            });
            if let Some(edit) = edit {
                self.apply(ui.ctx(), edit);
            }
        }

        egui::Panel::bottom(egui::Id::new(("status-codigo", &self.rel)))
            .resizable(false)
            .show_separator_line(true)
            .show(ui, |ui| self.status_line(ui, &mut actions));

        if self.find.open {
            self.find_bar(ui, id);
        }
        if self.goto_open {
            self.goto_bar(ui);
        }

        // Marcações de fundo: ocorrências da busca.
        let mut marks = Vec::new();
        if self.find.open && !self.find.query.is_empty() {
            let found = find_all(&self.text, &self.find.query, self.find.match_case);
            let hl = if dark {
                Color32::from_rgba_unmultiplied(0xE8, 0xB3, 0x39, 60)
            } else {
                Color32::from_rgba_unmultiplied(0xFF, 0xD0, 0x40, 110)
            };
            let cur = if dark {
                Color32::from_rgba_unmultiplied(0xE8, 0x8B, 0x10, 150)
            } else {
                Color32::from_rgba_unmultiplied(0xFF, 0x96, 0x00, 170)
            };
            for (i, r) in found.into_iter().enumerate() {
                marks.push((r, if i == self.find.current { cur } else { hl }));
            }
        }

        let line_count = self.text.matches('\n').count() + 1;
        let digits = line_count.to_string().len().max(2);
        let char_w = ui.fonts_mut(|f| f.glyph_width(&font, '0'));
        let gutter_w = digits as f32 * char_w + 22.0;
        let issue_line = self.issue.as_ref().map(|i| i.line);
        let jump = self.jump.take();
        let select = self.select.take();

        if jump.is_some() || select.is_some() {
            let mut state = egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
            let range = if let Some(line) = jump {
                let b = line_start(&self.text, line);
                let c = char_index(&self.text, b);
                CCursorRange::one(CCursor::new(c))
            } else {
                let r = select.clone().unwrap_or(0..0);
                CCursorRange::two(CCursor::new(r.start), CCursor::new(r.end))
            };
            state.cursor.set_char_range(Some(range));
            state.store(ui.ctx(), id);
            if jump.is_some() {
                ui.memory_mut(|m| m.request_focus(id));
            }
        }

        let editor_h = ui.available_height().max(60.0);
        let mut layouter =
            |ui: &egui::Ui, buf: &dyn egui::TextBuffer, _w: f32| -> Arc<egui::Galley> {
                let job = highlight(buf.as_str(), &font, &palette, &marks);
                ui.fonts_mut(|f| f.layout_job(job))
            };
        let before = self.text.clone();
        let mut scroll_target = None;
        let mut popup_pos: Option<Rect> = None;
        egui::ScrollArea::both()
            .id_salt(("rolagem-codigo", &self.rel))
            .auto_shrink([false, false])
            .max_height(editor_h)
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let (gutter, _) = ui.allocate_exact_size(
                        Vec2::new(gutter_w, editor_h - 4.0),
                        egui::Sense::hover(),
                    );
                    let out = egui::TextEdit::multiline(&mut self.text)
                        .id(id)
                        .font(font.clone())
                        .code_editor()
                        .lock_focus(true)
                        .frame(egui::Frame::NONE)
                        .margin(egui::Margin::symmetric(6, 4))
                        .desired_width(f32::INFINITY)
                        .desired_rows(((editor_h - 8.0) / font_size.max(1.0) / 1.2) as usize)
                        .layouter(&mut layouter)
                        .show(ui);
                    let painter = ui.painter().clone();
                    let galley = out.galley.clone();
                    let origin = out.galley_pos;
                    let gutter_x = gutter.right() - 10.0;
                    let weak = ui.visuals().weak_text_color();
                    let strong = ui.visuals().strong_text_color();
                    let cursor_line = out.cursor_range.map(|r| {
                        let b = byte_index(&self.text, r.primary.index.0);
                        line_col(&self.text, b)
                    });
                    if let Some(lc) = cursor_line {
                        self.cursor = lc;
                    }
                    let mut line = 1;
                    let mut new_line = true;
                    for row in &galley.rows {
                        let rect = row.rect().translate(origin.to_vec2());
                        if new_line {
                            let is_cursor = cursor_line.is_some_and(|(l, _)| l == line)
                                && out.response.response.has_focus();
                            if is_cursor {
                                painter.rect_filled(
                                    Rect::from_min_max(
                                        Pos2::new(gutter.left(), rect.top()),
                                        Pos2::new(ui.clip_rect().right(), rect.bottom()),
                                    ),
                                    0.0,
                                    ui.visuals().text_color().gamma_multiply(0.05),
                                );
                            }
                            if issue_line == Some(line) {
                                painter.rect_filled(
                                    Rect::from_min_max(
                                        Pos2::new(gutter.left(), rect.top()),
                                        Pos2::new(ui.clip_rect().right(), rect.bottom()),
                                    ),
                                    0.0,
                                    Color32::from_rgba_unmultiplied(0xE5, 0x48, 0x4D, 40),
                                );
                                painter.circle_filled(
                                    Pos2::new(gutter.left() + 7.0, rect.center().y),
                                    3.5,
                                    Color32::from_rgb(0xE5, 0x48, 0x4D),
                                );
                            }
                            painter.text(
                                Pos2::new(gutter_x, rect.top()),
                                egui::Align2::RIGHT_TOP,
                                line.to_string(),
                                font.clone(),
                                if is_cursor { strong } else { weak },
                            );
                            if jump == Some(line) {
                                scroll_target = Some(rect);
                            }
                        }
                        new_line = row.ends_with_newline;
                        if new_line {
                            line += 1;
                        }
                    }
                    painter.line_segment(
                        [
                            Pos2::new(gutter.right() - 3.0, gutter.top()),
                            Pos2::new(
                                gutter.right() - 3.0,
                                gutter.top().max(origin.y + galley.size().y),
                            ),
                        ],
                        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
                    );
                    if let Some(r) = &select {
                        let c = galley.pos_from_cursor(CCursor::new(r.start));
                        scroll_target = Some(c.translate(origin.to_vec2()));
                    }
                    // Onde fica a lista de sugestões e a dica da função.
                    if let Some(range) = out.cursor_range
                        && out.response.response.has_focus()
                    {
                        let c = galley
                            .pos_from_cursor(range.primary)
                            .translate(origin.to_vec2());
                        popup_pos = Some(c);
                    }
                    // Dica ao passar o mouse num nome da API.
                    if let Some(p) = out.response.response.hover_pos() {
                        let c = galley.cursor_from_pos(p - origin);
                        let b = byte_index(&self.text, c.index.0);
                        if let Some(doc) = complete::doc_at(&self.text, b) {
                            out.response.response.clone().on_hover_ui_at_pointer(|ui| {
                                ui.label(
                                    RichText::new(format!("{}{}", doc.name, doc.args))
                                        .monospace()
                                        .strong(),
                                );
                                ui.label(doc.doc);
                            });
                        }
                    }
                    if let Some(t) = scroll_target {
                        ui.scroll_to_rect(t.expand(20.0), Some(egui::Align::Center));
                    }
                });
            });
        if self.text != before {
            self.last_edit = Instant::now();
        }
        // Sugestões: abrem sozinhas ao digitar e somem quando o cursor anda.
        if let Some(env) = env {
            let cursor = self.cursor_byte(ui.ctx());
            if forced {
                self.refresh_suggest(ui.ctx(), env, true);
            } else if self.text != before {
                let typed = cursor
                    .and_then(|c| self.text[..c.min(self.text.len())].chars().last())
                    .is_some_and(|ch| {
                        ch.is_alphanumeric() || matches!(ch, '_' | '.' | ':' | '"' | '\'' | '/')
                    });
                if typed {
                    self.refresh_suggest(ui.ctx(), env, self.suggest.is_some());
                } else {
                    self.suggest = None;
                }
            } else if cursor != self.last_cursor && self.suggest.is_some() {
                self.suggest = None;
            }
            self.last_cursor = cursor;
            if let Some(pos) = popup_pos {
                self.suggest_ui(ui, pos, &font);
            }
        }
        self.check_syntax(lua, ui.ctx());

        actions
    }

    /// A lista de sugestões (ou a dica da função) embaixo do cursor.
    fn suggest_ui(&mut self, ui: &mut egui::Ui, cursor: Rect, font: &FontId) {
        let ctx = ui.ctx().clone();
        let Some(sug) = &self.suggest else {
            // Sem lista: mostra os parâmetros da função em que está o cursor.
            let Some(b) = self.cursor_byte(&ctx) else {
                return;
            };
            if let Some(doc) = complete::signature_at(&self.text, b) {
                egui::Area::new(egui::Id::new(("assinatura", &self.rel)))
                    .order(egui::Order::Tooltip)
                    .fixed_pos(cursor.left_top() - Vec2::new(0.0, 26.0))
                    .interactable(false)
                    .show(&ctx, |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("{}{}", doc.name, doc.args))
                                        .font(font.clone())
                                        .strong(),
                                );
                                ui.weak(doc.doc);
                            });
                        });
                    });
            }
            return;
        };
        let mut clicked = None;
        let selected = sug.selected;
        egui::Area::new(egui::Id::new(("sugestoes", &self.rel)))
            .order(egui::Order::Foreground)
            .fixed_pos(cursor.left_bottom() + Vec2::new(0.0, 2.0))
            .show(&ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        egui::ScrollArea::vertical()
                            .max_height(220.0)
                            .max_width(240.0)
                            .id_salt(("lista-sugestoes", &self.rel))
                            .show(ui, |ui| {
                                ui.set_width(240.0);
                                for (i, item) in sug.completion.items.iter().enumerate() {
                                    let text = RichText::new(format!(
                                        "{}  {}",
                                        item.kind.badge(),
                                        item.label
                                    ))
                                    .font(font.clone());
                                    let r = ui.add(
                                        egui::Button::selectable(
                                            i == selected,
                                            (text, egui::Atom::grow()),
                                        )
                                        .min_size(Vec2::new(240.0, 0.0)),
                                    );
                                    if i == selected {
                                        r.scroll_to_me(None);
                                    }
                                    if r.clicked() {
                                        clicked = Some(i);
                                    }
                                }
                            });
                        if let Some(item) = sug.completion.items.get(selected)
                            && (!item.detail.is_empty() || !item.doc.is_empty())
                        {
                            ui.separator();
                            ui.vertical(|ui| {
                                ui.set_width(260.0);
                                ui.label(
                                    RichText::new(format!("{}{}", item.label, item.detail))
                                        .font(font.clone())
                                        .strong(),
                                );
                                if !item.doc.is_empty() {
                                    ui.label(&item.doc);
                                }
                            });
                        }
                    });
                    ui.weak("Enter ou Tab: usar · Esc: fechar");
                });
            });
        if let Some(i) = clicked
            && let Some(s) = &mut self.suggest
        {
            s.selected = i;
            self.accept(&ctx);
            ctx.memory_mut(|m| m.request_focus(self.editor_id()));
        }
    }

    fn status_line(&mut self, ui: &mut egui::Ui, actions: &mut Vec<CodeAction>) {
        ui.horizontal(|ui| {
            match &self.issue {
                Some(issue) => {
                    let text = RichText::new(format!("⛔ Linha {}: {}", issue.line, issue.message))
                        .color(Color32::from_rgb(0xF2, 0x6D, 0x6D));
                    if ui
                        .link(text)
                        .on_hover_text("Ir para a linha do erro")
                        .clicked()
                    {
                        self.go_to_line(issue.line);
                    }
                }
                None if self.is_lua() => {
                    ui.weak("✔ Sem erros de sintaxe");
                }
                None => {}
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(format!(
                    "Linha {}, coluna {} · {}",
                    self.cursor.0,
                    self.cursor.1,
                    if self.crlf { "CRLF" } else { "LF" }
                ));
                if self.is_dirty() {
                    if ui
                        .small_button("💾 Salvar")
                        .on_hover_text("Ctrl+S")
                        .clicked()
                    {
                        actions.push(CodeAction::Save);
                    }
                    ui.label(
                        RichText::new("• não salvo").color(Color32::from_rgb(0xE8, 0xB3, 0x39)),
                    );
                }
            });
        });
    }

    fn find_bar(&mut self, ui: &mut egui::Ui, editor: egui::Id) {
        let find_id = self.find_id();
        let found = find_all(&self.text, &self.find.query, self.find.match_case);
        if self.find.current >= found.len() {
            self.find.current = 0;
        }
        let mut go: Option<isize> = None;
        let mut close = false;
        ui.horizontal_wrapped(|ui| {
            ui.label("🔍");
            // O foco vem antes do campo, para ele já receber o que for digitado.
            if self.find.focus {
                ui.memory_mut(|m| m.request_focus(find_id));
                self.find.focus = false;
            }
            let r = ui.add(
                egui::TextEdit::singleline(&mut self.find.query)
                    .id(find_id)
                    .hint_text("Buscar")
                    .desired_width(180.0),
            );
            if r.changed() {
                self.find.current = 0;
                if !self.find.query.is_empty() {
                    go = Some(0);
                }
            }
            if r.lost_focus() && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
                go = Some(if ui.input(|i| i.modifiers.shift) {
                    -1
                } else {
                    1
                });
                r.request_focus();
            }
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                close = true;
            }
            let count = if self.find.query.is_empty() {
                String::new()
            } else if found.is_empty() {
                "nada encontrado".into()
            } else {
                format!("{} de {}", self.find.current + 1, found.len())
            };
            ui.weak(count);
            if ui
                .small_button("⏶")
                .on_hover_text("Anterior (Shift+Enter)")
                .clicked()
            {
                go = Some(-1);
            }
            if ui
                .small_button("⏷")
                .on_hover_text("Próxima (Enter)")
                .clicked()
            {
                go = Some(1);
            }
            ui.toggle_value(&mut self.find.match_case, "Aa")
                .on_hover_text("Diferenciar maiúsculas e minúsculas");
            if self.find.replace {
                ui.separator();
                ui.add(
                    egui::TextEdit::singleline(&mut self.find.replacement)
                        .hint_text("Substituir por")
                        .desired_width(160.0),
                );
                if ui.button("Substituir").clicked()
                    && let Some(r) = found.get(self.find.current).cloned()
                {
                    self.text.replace_range(r.clone(), &self.find.replacement);
                    self.last_edit = Instant::now();
                    go = Some(0);
                }
                if ui.button("Substituir tudo").clicked() && !found.is_empty() {
                    for r in found.iter().rev() {
                        self.text.replace_range(r.clone(), &self.find.replacement);
                    }
                    self.last_edit = Instant::now();
                }
            } else if ui.small_button("Substituir...").clicked() {
                self.find.replace = true;
            }
            if ui.small_button("✖").on_hover_text("Fechar (Esc)").clicked() {
                close = true;
            }
        });
        if close {
            self.find.open = false;
            ui.memory_mut(|m| m.request_focus(editor));
            return;
        }
        if let Some(step) = go {
            let found = find_all(&self.text, &self.find.query, self.find.match_case);
            if !found.is_empty() {
                let n = found.len() as isize;
                self.find.current = ((self.find.current as isize + step).rem_euclid(n)) as usize;
                let r = &found[self.find.current];
                self.select = Some(char_index(&self.text, r.start)..char_index(&self.text, r.end));
            }
        }
    }

    fn goto_bar(&mut self, ui: &mut egui::Ui) {
        let goto_id = self.goto_id();
        let lines = self.text.matches('\n').count() + 1;
        let mut close = false;
        ui.horizontal(|ui| {
            ui.label(format!("Ir para a linha (1 a {lines}):"));
            if self.goto_focus {
                ui.memory_mut(|m| m.request_focus(goto_id));
                self.goto_focus = false;
            }
            let r = ui.add(
                egui::TextEdit::singleline(&mut self.goto_text)
                    .id(goto_id)
                    .desired_width(80.0),
            );
            if r.lost_focus() && ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) {
                if let Ok(n) = self.goto_text.trim().parse::<usize>() {
                    self.go_to_line(n.clamp(1, lines));
                }
                close = true;
            }
            if ui.input(|i| i.key_pressed(Key::Escape)) || ui.small_button("✖").clicked() {
                close = true;
            }
        });
        if close {
            self.goto_open = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn busca() {
        assert_eq!(find_all("aXa xa", "xa", false), vec![1..3, 4..6]);
        assert_eq!(find_all("aXa xa", "xa", true), vec![4..6]);
        assert_eq!(find_all("ação ação", "ção", true), vec![1..6, 8..13]);
        assert!(find_all("abc", "", false).is_empty());
    }

    #[test]
    fn indices() {
        let t = "olá\nmundo\n\nfim";
        assert_eq!(line_start(t, 1), 0);
        assert_eq!(line_start(t, 2), 5);
        assert_eq!(line_start(t, 4), 12);
        assert_eq!(line_start(t, 99), t.len());
        assert_eq!(line_col(t, 6), (2, 2));
        assert_eq!(line_col(t, 4), (1, 4));
        // No meio de um caractere de vários bytes: arredonda para trás.
        assert_eq!(line_col(t, 3), (1, 3));
        assert_eq!(char_index(t, 5), 4);
        assert_eq!(byte_index(t, 4), 5);
        assert_eq!(byte_index(t, 999), t.len());
    }

    #[test]
    fn enter_com_recuo() {
        let mut t = String::from("function a()\nend");
        let sel = apply_edit(&mut t, 12..12, Edit::Newline);
        assert_eq!(t, "function a()\n  \nend");
        assert_eq!(sel, 15..15);
        let mut t = String::from("  x = 1");
        let sel = apply_edit(&mut t, 7..7, Edit::Newline);
        assert_eq!(t, "  x = 1\n  ");
        assert_eq!(sel, 10..10);
        // Com seleção: troca a seleção.
        let mut t = String::from("abcdef");
        apply_edit(&mut t, 2..4, Edit::Newline);
        assert_eq!(t, "ab\nef");
    }

    #[test]
    fn tab_e_shift_tab() {
        let mut t = String::from("a\nb\nc");
        let sel = apply_edit(&mut t, 0..3, Edit::Indent);
        assert_eq!(t, "  a\n  b\nc");
        assert_eq!(sel, 0..7);
        let sel = apply_edit(&mut t, sel, Edit::Unindent);
        assert_eq!(t, "a\nb\nc");
        assert_eq!(sel, 0..3);
        // Tab sem seleção insere espaços.
        let mut t = String::from("xy");
        let sel = apply_edit(&mut t, 1..1, Edit::Indent);
        assert_eq!((t.as_str(), sel), ("x  y", 3..3));
        // Shift+Tab sem seleção tira o recuo da linha.
        let mut t = String::from("    z");
        let sel = apply_edit(&mut t, 5..5, Edit::Unindent);
        assert_eq!((t.as_str(), sel), ("  z", 3..3));
    }

    #[test]
    fn comentar_e_descomentar() {
        let mut t = String::from("  a = 1\n    b = 2\n\nc");
        let sel = apply_edit(&mut t, 0..17, Edit::ToggleComment);
        assert_eq!(t, "  -- a = 1\n  --   b = 2\n\nc");
        apply_edit(&mut t, sel, Edit::ToggleComment);
        assert_eq!(t, "  a = 1\n    b = 2\n\nc");
        let mut t = String::from("x");
        apply_edit(&mut t, 0..0, Edit::ToggleComment);
        assert_eq!(t, "-- x");
    }

    #[test]
    fn cores_cobrem_o_texto() {
        let font = FontId::monospace(14.0);
        let p = Palette::for_theme(true);
        let text = "local x = 1 -- oi\nprint(x)";
        let job = highlight(text, &font, &p, &[(6..7, Color32::RED)]);
        let mut pos = 0;
        for s in &job.sections {
            assert_eq!(s.byte_range.start.0, pos);
            pos = s.byte_range.end.0;
        }
        assert_eq!(pos, text.len());
        assert!(
            job.sections
                .iter()
                .any(|s| s.byte_range == (ByteIndex(6)..ByteIndex(7))
                    && s.format.background == Color32::RED)
        );
        assert_eq!(highlight("", &font, &p, &[]).sections.len(), 1);
    }

    #[test]
    fn abre_salva_e_percebe_mudancas() {
        let dir = std::env::temp_dir().join(format!("nn-studio-code-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.lua"), "x = 1\r\ny = 2\r\n").unwrap();
        let mut d = CodeDoc::open(&dir, "a.lua").unwrap();
        assert_eq!(d.text, "x = 1\ny = 2\n");
        assert!(!d.is_dirty());
        d.text.push_str("z = 3\n");
        assert!(d.is_dirty());
        d.save(&dir).unwrap();
        assert!(!d.is_dirty());
        // Mantém o fim de linha original (CRLF).
        assert_eq!(
            std::fs::read_to_string(dir.join("a.lua")).unwrap(),
            "x = 1\r\ny = 2\r\nz = 3\r\n"
        );

        // Mudou fora, sem alterações aqui: recarrega.
        std::fs::write(dir.join("a.lua"), "novo = true\n").unwrap();
        d.disk_changed(&dir);
        assert_eq!(d.text, "novo = true\n");
        assert!(!d.external_change);
        // Mudou fora, com alterações aqui: avisa e não perde nada.
        d.text.push_str("meu = 1\n");
        std::fs::write(dir.join("a.lua"), "outro = 2\n").unwrap();
        d.disk_changed(&dir);
        assert!(d.external_change);
        assert!(d.text.contains("meu = 1"));
        d.reload(&dir).unwrap();
        assert_eq!(d.text, "outro = 2\n");
        assert!(!d.is_dirty());

        std::fs::write(dir.join("b.lua"), [0xFF, 0xFE, 0x00]).unwrap();
        assert!(CodeDoc::open(&dir, "b.lua").is_err());
    }
}
