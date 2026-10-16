//! Console: mensagens do editor e do jogo (o `print` do Lua, avisos e erros),
//! com filtros, busca e posições `arquivo.lua:12` clicáveis.

use std::collections::{HashSet, VecDeque};

use eframe::egui::{self, Color32, RichText};
use serde::{Deserialize, Serialize};

use crate::lua_syntax::find_locations;

/// Máximo de mensagens guardadas (as mais antigas saem).
pub const MAX_ENTRIES: usize = 2000;

/// Quantas mensagens aparecem de uma vez.
const SHOWN_ENTRIES: usize = 400;

/// Começo das linhas de log que o jogo manda para o editor.
pub const LOG_PREFIX: &str = "@nn ";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Editor,
    Game,
}

/// Grupo do filtro do console.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Error,
    Warning,
    /// `print` do jogo e avisos do editor.
    Message,
    /// Informações internas da engine (só no modo Completo).
    Engine,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub id: u64,
    pub source: Source,
    pub level: Level,
    pub category: Category,
    pub text: String,
    /// Quantas vezes seguidas a mesma mensagem apareceu.
    pub count: u32,
}

/// Registro de log no formato que o jogo escreve (`@nn {json}`).
#[derive(Debug, Serialize, Deserialize)]
struct Record {
    #[serde(rename = "n")]
    level: Level,
    #[serde(rename = "a")]
    target: String,
    #[serde(rename = "m")]
    message: String,
}

/// Linha de log do jogo para o editor (usada no modo `--jogar`).
pub fn format_record(level: log::Level, target: &str, message: &str) -> String {
    let level = match level {
        log::Level::Error => Level::Error,
        log::Level::Warn => Level::Warn,
        log::Level::Info => Level::Info,
        _ => Level::Debug,
    };
    let rec = Record {
        level,
        target: target.to_string(),
        message: message.to_string(),
    };
    format!(
        "{LOG_PREFIX}{}",
        serde_json::to_string(&rec).unwrap_or_default()
    )
}

/// Interpreta uma linha da saída do jogo: (nível, categoria, texto).
/// Linha em que o jogo avisa qual cena está na frente (para o ajuste ao
/// vivo): devolve o caminho da cena.
pub fn live_scene(line: &str) -> Option<String> {
    match live_message(line)? {
        LiveMessage::Scene(s) => Some(s),
        _ => None,
    }
}

/// O que o jogo aberto pelo editor manda pelo canal ao vivo.
#[derive(Debug, Clone, PartialEq)]
pub enum LiveMessage {
    /// A cena da frente.
    Scene(String),
    /// Onde estão os objetos, propriedades e variáveis (JSON).
    State(serde_json::Value),
    /// Ctrl+clique num objeto da janela do jogo.
    Select(u32),
}

/// Linha do canal ao vivo (não vai para o console).
pub fn live_message(line: &str) -> Option<LiveMessage> {
    let json = line.strip_prefix(LOG_PREFIX)?;
    let rec = serde_json::from_str::<Record>(json).ok()?;
    if rec.target != nn_script::live::LOG_TARGET {
        return None;
    }
    let m = rec.message;
    if let Some(s) = m.strip_prefix("cena: ") {
        return Some(LiveMessage::Scene(s.to_string()));
    }
    if let Some(s) = m.strip_prefix("estado: ") {
        return serde_json::from_str(s).ok().map(LiveMessage::State);
    }
    if let Some(s) = m.strip_prefix("selecionar: ") {
        return s.trim().parse().ok().map(LiveMessage::Select);
    }
    None
}

pub fn parse_game_line(line: &str) -> (Level, Category, String) {
    if let Some(json) = line.strip_prefix(LOG_PREFIX)
        && let Ok(rec) = serde_json::from_str::<Record>(json)
    {
        if let Some(text) = rec.message.strip_prefix("[lua] ") {
            return (Level::Info, Category::Message, text.to_string());
        }
        // Avisos de bibliotecas de fora (janela, GPU) não interessam a quem
        // faz o jogo: ficam com os detalhes da engine.
        let ours = rec.target.starts_with("nn_");
        let category = match rec.level {
            Level::Error => Category::Error,
            Level::Warn if ours => Category::Warning,
            _ => Category::Engine,
        };
        let text = rec
            .message
            .strip_prefix("erro em ")
            .and_then(|r| r.split_once(":\n"))
            .map(|(_, rest)| rest.to_string())
            .unwrap_or(rec.message);
        return (rec.level, category, text);
    }
    // Linha solta (algo escrito direto na saída, como avisos do driver de
    // vídeo): detalhe interno, a não ser que seja um erro ou uma falha.
    let lower = line.to_lowercase();
    if lower.starts_with("erro:") || lower.starts_with("error") || lower.contains("panicked") {
        (Level::Error, Category::Error, line.to_string())
    } else {
        (Level::Info, Category::Engine, line.to_string())
    }
}

/// O que o usuário pediu ao clicar no console.
#[derive(Debug, Clone, PartialEq)]
pub enum ConsoleAction {
    OpenLocation { path: String, line: usize },
}

#[derive(Debug)]
pub struct Console {
    entries: VecDeque<Entry>,
    next_id: u64,
    expanded: HashSet<u64>,
    pub show_errors: bool,
    pub show_warnings: bool,
    pub show_messages: bool,
    pub show_engine: bool,
    pub search: String,
    /// Erros que chegaram desde a última vez que o console foi visto.
    pub unseen_errors: usize,
}

impl Default for Console {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            next_id: 0,
            expanded: HashSet::new(),
            show_errors: true,
            show_warnings: true,
            show_messages: true,
            show_engine: false,
            search: String::new(),
            unseen_errors: 0,
        }
    }
}

impl Console {
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.expanded.clear();
        self.unseen_errors = 0;
    }

    pub fn push(
        &mut self,
        source: Source,
        level: Level,
        category: Category,
        text: impl Into<String>,
    ) {
        let text = text.into();
        if category == Category::Error {
            self.unseen_errors += 1;
        }
        if let Some(last) = self.entries.back_mut()
            && last.text == text
            && last.source == source
            && last.level == level
        {
            last.count += 1;
            return;
        }
        self.next_id += 1;
        self.entries.push_back(Entry {
            id: self.next_id,
            source,
            level,
            category,
            text,
            count: 1,
        });
        while self.entries.len() > MAX_ENTRIES {
            if let Some(old) = self.entries.pop_front() {
                self.expanded.remove(&old.id);
            }
        }
    }

    /// Mensagem do próprio editor.
    pub fn info(&mut self, text: impl Into<String>) {
        self.push(Source::Editor, Level::Info, Category::Message, text);
    }

    pub fn warn(&mut self, text: impl Into<String>) {
        self.push(Source::Editor, Level::Warn, Category::Warning, text);
    }

    pub fn error(&mut self, text: impl Into<String>) {
        self.push(Source::Editor, Level::Error, Category::Error, text);
    }

    /// Uma linha da saída do jogo.
    pub fn push_game_line(&mut self, line: &str) {
        let (level, category, text) = parse_game_line(line);
        if text.trim().is_empty() {
            return;
        }
        self.push(Source::Game, level, category, text);
    }

    pub fn count(&self, category: Category) -> usize {
        self.entries
            .iter()
            .filter(|e| e.category == category)
            .map(|e| e.count as usize)
            .sum()
    }

    fn visible(&self, e: &Entry, full_mode: bool) -> bool {
        let by_category = match e.category {
            Category::Error => self.show_errors,
            Category::Warning => self.show_warnings,
            Category::Message => self.show_messages,
            Category::Engine => full_mode && self.show_engine,
        };
        by_category
            && (self.search.is_empty()
                || crate::commands::fold(&e.text).contains(&crate::commands::fold(&self.search)))
    }

    /// Texto de todas as mensagens visíveis (para copiar).
    pub fn visible_text(&self, full_mode: bool) -> String {
        self.entries
            .iter()
            .filter(|e| self.visible(e, full_mode))
            .map(|e| {
                if e.count > 1 {
                    format!("{} (x{})", e.text, e.count)
                } else {
                    e.text.clone()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Desenha o console. Devolve o que foi clicado.
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        full_mode: bool,
        clear_on_play: &mut bool,
    ) -> Option<ConsoleAction> {
        self.unseen_errors = 0;
        let mut action = None;
        ui.horizontal_wrapped(|ui| {
            if ui
                .button("🗑 Limpar")
                .on_hover_text("Apaga todas as mensagens")
                .clicked()
            {
                self.clear();
            }
            if ui
                .button("📋 Copiar")
                .on_hover_text("Copia as mensagens visíveis")
                .clicked()
            {
                ui.ctx().copy_text(self.visible_text(full_mode));
            }
            ui.separator();
            let errors = self.count(Category::Error);
            let warnings = self.count(Category::Warning);
            let messages = self.count(Category::Message);
            ui.toggle_value(&mut self.show_errors, format!("⛔ Erros {errors}"))
                .on_hover_text("Mostrar erros");
            ui.toggle_value(&mut self.show_warnings, format!("⚠ Avisos {warnings}"))
                .on_hover_text("Mostrar avisos");
            ui.toggle_value(&mut self.show_messages, format!("💬 Mensagens {messages}"))
                .on_hover_text("Mostrar mensagens do jogo (print) e do editor");
            if full_mode {
                let engine = self.count(Category::Engine);
                ui.toggle_value(&mut self.show_engine, format!("⚙ Engine {engine}"))
                    .on_hover_text("Mostrar detalhes internos da engine");
            }
            ui.separator();
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("🔍 Filtrar")
                    .desired_width(140.0),
            );
            ui.checkbox(clear_on_play, "Limpar ao jogar");
        });
        ui.separator();

        let visible: Vec<usize> = (0..self.entries.len())
            .filter(|&i| self.visible(&self.entries[i], full_mode))
            .collect();
        if visible.is_empty() {
            ui.weak(if self.entries.is_empty() {
                "Nenhuma mensagem. O que o jogo escrever com print() aparece aqui."
            } else {
                "Nenhuma mensagem com esses filtros."
            });
            return None;
        }
        // Só as últimas mensagens são desenhadas: o console continua leve
        // mesmo com um jogo que escreve muito.
        let skip = visible.len().saturating_sub(SHOWN_ENTRIES);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                if skip > 0 {
                    ui.weak(format!(
                        "({skip} mensagens mais antigas escondidas; use o filtro)"
                    ));
                }
                for &i in &visible[skip..] {
                    if let Some(a) = self.entry_ui(ui, i) {
                        action = Some(a);
                    }
                }
            });
        action
    }

    fn entry_ui(&mut self, ui: &mut egui::Ui, index: usize) -> Option<ConsoleAction> {
        let e = &self.entries[index];
        let (icon, color) = match e.category {
            Category::Error => ("⛔", Color32::from_rgb(0xF2, 0x6D, 0x6D)),
            Category::Warning => ("⚠", Color32::from_rgb(0xE8, 0xB3, 0x39)),
            Category::Message => match e.source {
                Source::Game => ("💬", ui.visuals().text_color()),
                Source::Editor => ("✏", ui.visuals().weak_text_color()),
            },
            Category::Engine => ("⚙", ui.visuals().weak_text_color()),
        };
        let id = e.id;
        let mut lines = e.text.lines();
        let first = lines.next().unwrap_or("").to_string();
        let rest: Vec<String> = lines.map(str::to_string).collect();
        let count = e.count;
        let expanded = self.expanded.contains(&id);
        let mut action = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            if rest.is_empty() {
                ui.label(RichText::new(format!("{icon} ")).color(color));
            } else {
                let arrow = if expanded { "⏷" } else { "⏵" };
                if ui
                    .add(
                        egui::Button::new(RichText::new(format!("{arrow}{icon} ")).color(color))
                            .frame(false),
                    )
                    .on_hover_text(if expanded {
                        "Esconder detalhes"
                    } else {
                        "Mostrar detalhes"
                    })
                    .clicked()
                {
                    if expanded {
                        self.expanded.remove(&id);
                    } else {
                        self.expanded.insert(id);
                    }
                }
            }
            if let Some(a) = text_with_links(ui, &first, color) {
                action = Some(a);
            }
            if count > 1 {
                ui.label(RichText::new(format!("  ×{count}")).small().weak());
            }
        });
        if expanded {
            ui.indent(("console-detalhes", id), |ui| {
                for line in &rest {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        if let Some(a) = text_with_links(ui, line, ui.visuals().weak_text_color()) {
                            action = Some(a);
                        }
                    });
                }
            });
        }
        action
    }
}

/// Mostra um texto em que `arquivo.lua:12` vira link.
fn text_with_links(ui: &mut egui::Ui, text: &str, color: Color32) -> Option<ConsoleAction> {
    let text = text.replace('\t', "    ");
    let mut action = None;
    let mut pos = 0;
    for loc in find_locations(&text) {
        if loc.range.start > pos {
            ui.label(
                RichText::new(&text[pos..loc.range.start])
                    .monospace()
                    .color(color),
            );
        }
        if ui
            .link(RichText::new(&text[loc.range.clone()]).monospace())
            .on_hover_text("Abrir o arquivo nesta linha")
            .clicked()
        {
            action = Some(ConsoleAction::OpenLocation {
                path: loc.path.clone(),
                line: loc.line,
            });
        }
        pos = loc.range.end;
    }
    if pos < text.len() || text.is_empty() {
        ui.label(RichText::new(&text[pos..]).monospace().color(color));
    }
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linhas_do_jogo() {
        let l = format_record(log::Level::Info, "nn_script::api", "[lua] olá\tmundo");
        assert!(l.starts_with(LOG_PREFIX));
        assert_eq!(
            parse_game_line(&l),
            (Level::Info, Category::Message, "olá\tmundo".into())
        );

        let l = format_record(
            log::Level::Error,
            "nn_script",
            "erro em cenas/fase.lua:\ncenas/fase.lua:3: tentou chamar\ncaminho",
        );
        assert_eq!(
            parse_game_line(&l),
            (
                Level::Error,
                Category::Error,
                "cenas/fase.lua:3: tentou chamar\ncaminho".into()
            )
        );

        let l = format_record(log::Level::Info, "nn_app::window", "janela aberta");
        assert_eq!(parse_game_line(&l).1, Category::Engine);
        let l = format_record(log::Level::Warn, "nn_app", "cuidado");
        assert_eq!(parse_game_line(&l).1, Category::Warning);
        // Aviso de biblioteca de fora: detalhe da engine.
        let l = format_record(log::Level::Warn, "winit::x11", "XRandR");
        assert_eq!(parse_game_line(&l).1, Category::Engine);

        // Linhas soltas.
        assert_eq!(parse_game_line("libEGL warning: DRI3").1, Category::Engine);
        assert_eq!(parse_game_line("erro: não abriu").1, Category::Error);
        assert_eq!(
            parse_game_line("thread 'main' panicked at x").1,
            Category::Error
        );
        // JSON quebrado não some.
        assert_eq!(parse_game_line("@nn {quebrado").1, Category::Engine);
    }

    #[test]
    fn repete_conta_e_limite() {
        let mut c = Console::default();
        c.info("a");
        c.info("a");
        c.info("b");
        assert_eq!(c.len(), 2);
        assert_eq!(c.entries().next().unwrap().count, 2);
        assert_eq!(c.count(Category::Message), 3);
        for i in 0..(MAX_ENTRIES + 10) {
            c.info(format!("m{i}"));
        }
        assert_eq!(c.len(), MAX_ENTRIES);
        c.error("x");
        c.error("y");
        assert_eq!(c.unseen_errors, 2);
        c.clear();
        assert!(c.is_empty());
        assert_eq!(c.unseen_errors, 0);
    }

    #[test]
    fn filtros_e_texto_visivel() {
        let mut c = Console::default();
        c.push(Source::Game, Level::Info, Category::Engine, "interno");
        c.push(Source::Game, Level::Info, Category::Message, "Olá jogador");
        c.error("Deu errado");
        c.warn("Atenção");
        assert_eq!(c.visible_text(false), "Olá jogador\nDeu errado\nAtenção");
        c.show_engine = true;
        assert!(c.visible_text(true).starts_with("interno"));
        assert!(!c.visible_text(false).contains("interno"));
        c.show_errors = false;
        assert!(!c.visible_text(false).contains("Deu errado"));
        c.search = "jogador".into();
        assert_eq!(c.visible_text(false), "Olá jogador");
        c.search = "ATENCAO".into();
        assert_eq!(c.visible_text(false), "Atenção");
    }
}
