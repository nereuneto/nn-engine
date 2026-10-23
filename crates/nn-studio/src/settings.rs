//! Preferências do NN Studio, guardadas na pasta de configuração do usuário:
//! projetos recentes, tema, modo (Simples/Completo) e o layout dos painéis de
//! cada projeto.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::project::Tab;

/// Quantos projetos aparecem em "Recentes".
pub const MAX_RECENT: usize = 12;

/// Nome do arquivo de preferências.
pub const SETTINGS_FILE: &str = "preferencias.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ThemeChoice {
    #[default]
    Escuro,
    Claro,
}

/// A cor de destaque do NN Studio (seleção, links, abas).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Accent {
    #[default]
    Azul,
    Verde,
    Roxo,
    Laranja,
    Rosa,
}

impl Accent {
    pub const ALL: [Accent; 5] = [
        Accent::Azul,
        Accent::Verde,
        Accent::Roxo,
        Accent::Laranja,
        Accent::Rosa,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Accent::Azul => "Azul",
            Accent::Verde => "Verde",
            Accent::Roxo => "Roxo",
            Accent::Laranja => "Laranja",
            Accent::Rosa => "Rosa",
        }
    }
}

/// Modo Simples esconde opções avançadas; o Completo mostra tudo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Simples,
    Completo,
}

impl Mode {
    pub fn is_full(self) -> bool {
        self == Mode::Completo
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::Simples => "Simples",
            Mode::Completo => "Completo",
        }
    }
}

/// Um projeto aberto recentemente.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecentProject {
    pub path: PathBuf,
    pub title: String,
    /// Quando foi aberto pela última vez (segundos desde 1970).
    pub opened_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub recent: Vec<RecentProject>,
    pub theme: ThemeChoice,
    pub mode: Mode,
    /// Tamanho da interface (1.0 = normal).
    pub zoom: f32,
    /// Pasta onde os novos projetos são criados.
    pub projects_dir: Option<PathBuf>,
    /// Limpa o console a cada vez que o jogo começa.
    pub clear_console_on_play: bool,
    /// Layout dos painéis (e abas abertas) de cada projeto, pela pasta.
    pub layouts: BTreeMap<String, egui_dock::DockState<Tab>>,
    /// Avisar quando sai uma versão nova (consulta o GitHub uma vez por dia).
    pub check_updates: bool,
    /// Última consulta (segundos desde 1970).
    pub last_update_check: u64,
    /// A versão mais nova vista na última consulta, e a página dela.
    pub latest_version: Option<String>,
    pub latest_url: Option<String>,
    /// Versão que a pessoa pediu para não lembrar mais.
    pub skipped_version: Option<String>,
    /// A cor de destaque.
    pub accent: Accent,
    /// Sons da interface (um "plim" ao salvar e exportar, um clique ao criar).
    pub ui_sounds: bool,
    /// Pequenas animações (avisos que aparecem e somem, o brilho dos objetos
    /// novos, o confete). Desligadas, tudo fica parado.
    pub animations: bool,
    /// Comemorar os marcos (o primeiro ▶ do projeto, a primeira exportação).
    pub celebrations: bool,
    /// Os marcos já comemorados (`jogar:<pasta>`, `exportar:<pasta>`).
    pub celebrated: std::collections::BTreeSet<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            recent: Vec::new(),
            theme: ThemeChoice::Escuro,
            mode: Mode::Simples,
            zoom: 1.0,
            projects_dir: None,
            clear_console_on_play: true,
            layouts: BTreeMap::new(),
            check_updates: true,
            last_update_check: 0,
            latest_version: None,
            latest_url: None,
            skipped_version: None,
            accent: Accent::Azul,
            ui_sounds: false,
            animations: true,
            celebrations: true,
            celebrated: Default::default(),
        }
    }
}

impl Settings {
    /// Lê as preferências. Arquivo ausente ou estragado vira o padrão (o
    /// estragado é guardado ao lado, para não perder nada).
    pub fn load(path: &Path) -> Self {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => return Self::default(),
        };
        match serde_json::from_str(&text) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("preferências inválidas em {}: {e}", path.display());
                let _ = std::fs::rename(path, path.with_extension("json.corrompido"));
                Self::default()
            }
        }
    }

    /// Grava as preferências (escreve num temporário e troca, para não deixar
    /// o arquivo pela metade).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, path)
    }

    /// Põe o projeto no topo dos recentes.
    pub fn touch_recent(&mut self, path: &Path, title: &str, now: u64) {
        let key = path_key(path);
        self.recent.retain(|r| path_key(&r.path) != key);
        self.recent.insert(
            0,
            RecentProject {
                path: path.to_path_buf(),
                title: title.to_string(),
                opened_at: now,
            },
        );
        self.recent.truncate(MAX_RECENT);
    }

    pub fn remove_recent(&mut self, path: &Path) {
        let key = path_key(path);
        self.recent.retain(|r| path_key(&r.path) != key);
    }

    pub fn layout(&self, project: &Path) -> Option<&egui_dock::DockState<Tab>> {
        self.layouts.get(&path_key(project))
    }

    pub fn set_layout(&mut self, project: &Path, mut layout: egui_dock::DockState<Tab>) {
        // As áreas na tela são recalculadas ao abrir; vazias, viram valores
        // infinitos que o JSON não guarda.
        for (_, node) in layout.iter_all_nodes_mut() {
            node.set_rect(eframe::egui::Rect::ZERO);
            if let Some(leaf) = node.get_leaf_mut() {
                leaf.viewport = eframe::egui::Rect::ZERO;
            }
        }
        self.layouts.insert(path_key(project), layout);
    }

    pub fn remove_layout(&mut self, project: &Path) {
        self.layouts.remove(&path_key(project));
    }

    /// Pasta padrão dos projetos novos: a escolhida antes, ou
    /// "Documentos/NN Studio".
    pub fn default_projects_dir(&self) -> PathBuf {
        if let Some(dir) = &self.projects_dir {
            return dir.clone();
        }
        let base = directories::UserDirs::new()
            .and_then(|u| {
                u.document_dir()
                    .map(Path::to_path_buf)
                    .or_else(|| Some(u.home_dir().to_path_buf()))
            })
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("NN Studio")
    }
}

/// Chave estável de uma pasta (caminho absoluto, quando possível).
pub fn path_key(path: &Path) -> String {
    absolute(path).to_string_lossy().into_owned()
}

/// Tira o prefixo `\\?\` que o Windows põe em caminhos "canônicos"
/// (caminhos de rede, `\\?\UNC\...`, ficam como estão).
fn strip_verbatim(s: &str) -> &str {
    match s.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC") => rest,
        _ => s,
    }
}

/// Caminho absoluto e sem o prefixo `\\?\` do Windows (alguns programas, e a
/// pasta de trabalho de um processo, não aceitam esse formato).
pub fn absolute(path: &Path) -> PathBuf {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    PathBuf::from(strip_verbatim(&abs.to_string_lossy()))
}

/// Caminho para mostrar ao usuário (sem o prefixo `\\?\` do Windows).
pub fn display_path(path: &Path) -> String {
    strip_verbatim(&path.to_string_lossy()).to_string()
}

/// Pasta das preferências: `NN_STUDIO_CONFIG` (para testes) ou a pasta de
/// configuração do usuário.
pub fn settings_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("NN_STUDIO_CONFIG") {
        return Some(PathBuf::from(dir));
    }
    directories::ProjectDirs::from("", "", "NN Studio").map(|d| d.config_dir().to_path_buf())
}

/// Segundos desde 1970.
pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// "agora há pouco", "há 3 horas", "ontem", "há 5 dias"...
pub fn time_ago(then: u64, now: u64) -> String {
    let s = now.saturating_sub(then);
    match s {
        0..60 => "agora há pouco".into(),
        60..3600 => plural(s / 60, "minuto", "minutos"),
        3600..86_400 => plural(s / 3600, "hora", "horas"),
        86_400..172_800 => "ontem".into(),
        172_800..2_592_000 => plural(s / 86_400, "dia", "dias"),
        2_592_000..31_536_000 => plural(s / 2_592_000, "mês", "meses"),
        _ => plural(s / 31_536_000, "ano", "anos"),
    }
}

fn plural(n: u64, one: &str, many: &str) -> String {
    if n == 1 {
        format!("há 1 {one}")
    } else {
        format!("há {n} {many}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("nn-studio-cfg-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// As preferências gravadas pelo NN Studio 0.4 abrem na 0.5 sem perder
    /// nada, com o novo no padrão.
    #[test]
    fn preferencias_da_0_4_abrem() {
        let dir = temp("da-0-4");
        let path = dir.join(SETTINGS_FILE);
        std::fs::write(&path, include_str!("../tests/dados/preferencias-0.4.json")).unwrap();
        let s = Settings::load(&path);
        assert_eq!(s.recent.len(), 1);
        assert_eq!(s.recent[0].title, "Visual");
        assert_eq!(s.layouts.len(), 1, "o layout do projeto ficou");
        assert_eq!(s.accent, Accent::Azul);
        assert!(s.animations && s.celebrations && !s.ui_sounds);
        assert!(s.celebrated.is_empty());
        // Nada foi guardado ao lado como estragado.
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    }

    #[test]
    fn preferencias_da_0_5_abrem() {
        let dir = temp("da-0-5");
        let path = dir.join(SETTINGS_FILE);
        std::fs::write(&path, include_str!("../tests/dados/preferencias-0.5.json")).unwrap();
        let s = Settings::load(&path);
        assert_eq!(s.recent.len(), 1);
        assert_eq!(s.recent[0].title, "Ilha");
        assert_eq!(s.layouts.len(), 1, "o layout do projeto ficou");
        assert_eq!(s.accent, Accent::Verde);
        assert!(s.celebrated.contains("primeiro-jogo"));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    }

    /// Gravadas pelo NN Studio 0.6.0 (com um projeto 3D aberto e a cor, os
    /// sons e as comemorações mudados).
    #[test]
    fn preferencias_da_0_6_abrem() {
        let dir = temp("da-0-6");
        let path = dir.join(SETTINGS_FILE);
        std::fs::write(&path, include_str!("../tests/dados/preferencias-0.6.json")).unwrap();
        let s = Settings::load(&path);
        assert_eq!(s.recent.len(), 1);
        assert_eq!(s.recent[0].title, "Ilha 3D");
        assert_eq!(s.layouts.len(), 1, "o layout do projeto ficou");
        assert_eq!(s.accent, Accent::Laranja);
        assert!(s.ui_sounds);
        assert!(s.celebrated.contains("primeiro-modelo"));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    }

    #[test]
    fn recentes_sem_repetir_e_com_limite() {
        let dir = temp("recentes");
        let mut s = Settings::default();
        for i in 0..15 {
            let p = dir.join(format!("p{i}"));
            std::fs::create_dir_all(&p).unwrap();
            s.touch_recent(&p, &format!("Jogo {i}"), i);
        }
        assert_eq!(s.recent.len(), MAX_RECENT);
        assert_eq!(s.recent[0].title, "Jogo 14");
        // Abrir de novo um que já está na lista: sobe para o topo, sem duplicar.
        s.touch_recent(&dir.join("p10"), "Jogo 10", 100);
        assert_eq!(s.recent.len(), MAX_RECENT);
        assert_eq!(s.recent[0].title, "Jogo 10");
        assert_eq!(s.recent.iter().filter(|r| r.title == "Jogo 10").count(), 1);
        // Caminho diferente para a mesma pasta conta como o mesmo projeto.
        s.touch_recent(&dir.join("p10/../p10"), "Jogo 10", 101);
        assert_eq!(s.recent.iter().filter(|r| r.title == "Jogo 10").count(), 1);
        s.remove_recent(&dir.join("p10"));
        assert!(s.recent.iter().all(|r| r.title != "Jogo 10"));
    }

    #[test]
    fn grava_e_le_de_volta() {
        let dir = temp("grava");
        let path = dir.join(SETTINGS_FILE);
        let mut s = Settings {
            theme: ThemeChoice::Claro,
            mode: Mode::Completo,
            zoom: 1.25,
            ..Settings::default()
        };
        s.touch_recent(&dir, "Teste", 5);
        s.set_layout(&dir, crate::project::default_layout());
        s.save(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        if let Err(e) = serde_json::from_str::<Settings>(&text) {
            panic!("{e}\n{text}");
        }
        let lido = Settings::load(&path);
        assert_eq!(lido.theme, ThemeChoice::Claro);
        assert_eq!(lido.mode, Mode::Completo);
        assert_eq!(lido.zoom, 1.25);
        assert_eq!(lido.recent, s.recent);
        assert!(lido.layout(&dir).is_some());
    }

    #[test]
    fn arquivo_estragado_vira_padrao_e_fica_guardado() {
        let dir = temp("estragado");
        let path = dir.join(SETTINGS_FILE);
        std::fs::write(&path, "{ isso não é json").unwrap();
        let s = Settings::load(&path);
        assert!(s.recent.is_empty());
        assert!(dir.join("preferencias.json.corrompido").exists());
        // Sem arquivo: padrão, sem erro.
        let s = Settings::load(&dir.join("nao-existe.json"));
        assert_eq!(s.mode, Mode::Simples);
    }

    #[test]
    fn caminhos_do_windows_sem_prefixo() {
        assert_eq!(strip_verbatim(r"\\?\C:\jogos\a"), r"C:\jogos\a");
        assert_eq!(strip_verbatim(r"\\?\UNC\srv\x"), r"\\?\UNC\srv\x");
        assert_eq!(strip_verbatim("/home/a"), "/home/a");
        assert!(absolute(Path::new(".")).is_absolute());
    }

    #[test]
    fn tempo_relativo() {
        assert_eq!(time_ago(100, 130), "agora há pouco");
        assert_eq!(time_ago(0, 60), "há 1 minuto");
        assert_eq!(time_ago(0, 7200), "há 2 horas");
        assert_eq!(time_ago(0, 90_000), "ontem");
        assert_eq!(time_ago(0, 5 * 86_400), "há 5 dias");
        assert_eq!(time_ago(0, 70 * 86_400), "há 2 meses");
        assert_eq!(time_ago(0, 800 * 86_400), "há 2 anos");
    }
}
