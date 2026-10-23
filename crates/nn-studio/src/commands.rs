//! Comandos do NN Studio: o que aparece nos menus, nos atalhos e na paleta
//! de comandos (Ctrl+Shift+P), e a busca aproximada da paleta.

use eframe::egui::{Key, KeyboardShortcut, Modifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    Play,
    TestHere,
    Stop,
    RunTests,
    ConnectAgent,
    Undo,
    Redo,
    Save,
    SaveAll,
    NewScene,
    NewCodeScene,
    NewScript,
    NewFolder,
    ImportFiles,
    NewProject,
    OpenProject,
    CloseProject,
    ProjectSettings,
    Export,
    Find,
    Replace,
    GoToLine,
    CloseTab,
    OpenExternal,
    RevealProject,
    ShowFiles,
    ShowConsole,
    ShowHome,
    ShowHierarchy,
    ShowInspector,
    ShowEvents,
    ShowTiles,
    ClearConsole,
    ResetLayout,
    ToggleTheme,
    ToggleMode,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    Palette,
    QuickOpen,
    Docs,
    Tutorials,
    Shortcuts,
    Preferences,
    About,
    Quit,
}

impl Command {
    pub const ALL: &'static [Command] = &[
        Command::Play,
        Command::TestHere,
        Command::Stop,
        Command::RunTests,
        Command::ConnectAgent,
        Command::Undo,
        Command::Redo,
        Command::Save,
        Command::SaveAll,
        Command::NewScene,
        Command::NewCodeScene,
        Command::NewScript,
        Command::NewFolder,
        Command::ImportFiles,
        Command::NewProject,
        Command::OpenProject,
        Command::CloseProject,
        Command::ProjectSettings,
        Command::Export,
        Command::Find,
        Command::Replace,
        Command::GoToLine,
        Command::CloseTab,
        Command::OpenExternal,
        Command::RevealProject,
        Command::ShowFiles,
        Command::ShowConsole,
        Command::ShowHome,
        Command::ShowHierarchy,
        Command::ShowInspector,
        Command::ShowEvents,
        Command::ShowTiles,
        Command::ClearConsole,
        Command::ResetLayout,
        Command::ToggleTheme,
        Command::ToggleMode,
        Command::ZoomIn,
        Command::ZoomOut,
        Command::ZoomReset,
        Command::Palette,
        Command::QuickOpen,
        Command::Docs,
        Command::Tutorials,
        Command::Shortcuts,
        Command::Preferences,
        Command::About,
        Command::Quit,
    ];

    /// Nome do comando, como aparece no menu e na paleta.
    pub fn title(self) -> &'static str {
        match self {
            Command::Play => "Jogar",
            Command::TestHere => "Testar a partir desta cena",
            Command::Undo => "Desfazer (cena)",
            Command::Redo => "Refazer (cena)",
            Command::Stop => "Parar o jogo",
            Command::RunTests => "Conferir com os testes",
            Command::ConnectAgent => "Ligar um agente de IA...",
            Command::Save => "Salvar",
            Command::SaveAll => "Salvar tudo",
            Command::NewScene => "Nova cena visual",
            Command::NewCodeScene => "Nova cena em código",
            Command::NewScript => "Novo script (módulo)",
            Command::NewFolder => "Nova pasta",
            Command::ImportFiles => "Importar arquivos...",
            Command::NewProject => "Novo projeto...",
            Command::OpenProject => "Abrir projeto...",
            Command::CloseProject => "Fechar projeto",
            Command::ProjectSettings => "Configurações do projeto",
            Command::Export => "Exportar jogo...",
            Command::Find => "Buscar no arquivo",
            Command::Replace => "Substituir no arquivo",
            Command::GoToLine => "Ir para a linha...",
            Command::CloseTab => "Fechar aba",
            Command::OpenExternal => "Abrir arquivo no editor externo",
            Command::RevealProject => "Mostrar a pasta do projeto",
            Command::ShowFiles => "Mostrar painel Arquivos",
            Command::ShowConsole => "Mostrar Console",
            Command::ShowHome => "Mostrar Início",
            Command::ShowHierarchy => "Mostrar Hierarquia",
            Command::ShowInspector => "Mostrar Inspetor",
            Command::ShowEvents => "Mostrar Eventos",
            Command::ShowTiles => "Mostrar Tiles",
            Command::ClearConsole => "Limpar o console",
            Command::ResetLayout => "Restaurar layout padrão",
            Command::ToggleTheme => "Alternar tema claro/escuro",
            Command::ToggleMode => "Alternar modo Simples/Completo",
            Command::ZoomIn => "Aumentar a interface",
            Command::ZoomOut => "Diminuir a interface",
            Command::ZoomReset => "Tamanho normal da interface",
            Command::Palette => "Paleta de comandos",
            Command::QuickOpen => "Abrir arquivo do projeto...",
            Command::Docs => "Guia do Lua (documentação)",
            Command::Shortcuts => "Atalhos do teclado",
            Command::Preferences => "Preferências...",
            Command::About => "Sobre o NN Studio",
            Command::Tutorials => "Tutoriais (passo a passo)",
            Command::Quit => "Sair",
        }
    }

    /// Palavras extras que também acham o comando na paleta.
    pub fn keywords(self) -> &'static str {
        match self {
            Command::Play => "rodar executar testar iniciar f5",
            Command::TestHere => "rodar jogar cena atual daqui f6",
            Command::Undo => "voltar ctrl z",
            Command::Redo => "refazer ctrl y",
            Command::Stop => "fechar encerrar",
            Command::RunTests => "rodar testes de jogo testar verificar nn testar automatico",
            Command::ConnectAgent => {
                "ia inteligencia artificial claude cursor mcp assistente agente chat"
            }
            Command::ProjectSettings => "config titulo janela tamanho tela cheia",
            Command::Export => "executavel distribuir publicar build exe",
            Command::Find => "procurar pesquisar",
            Command::Replace => "trocar",
            Command::ToggleTheme => "cores escuro claro",
            Command::ToggleMode => "avancado iniciante",
            Command::NewScene => "criar cena tela fase visual objetos",
            Command::NewCodeScene => "criar cena lua script programar",
            Command::NewScript => "criar arquivo lua modulo biblioteca",
            Command::ShowHierarchy => "objetos arvore lista cena",
            Command::ShowInspector => "propriedades componentes objeto",
            Command::ShowEvents => "quando entao regras sem codigo",
            Command::ShowTiles => "mapa pintar tileset terreno autotile",
            Command::Tutorials => "aprender tutorial passo a passo primeiro jogo ajuda",
            Command::ImportFiles => "adicionar imagem som",
            Command::QuickOpen => "arquivo ir",
            Command::Docs => "ajuda manual api funcoes",
            Command::Shortcuts => "teclas teclado",
            Command::Preferences => {
                "cor destaque sons animacoes comemorar tema configuracoes do studio"
            }
            Command::ResetLayout => "paineis janelas abas",
            _ => "",
        }
    }

    pub fn shortcut(self) -> Option<KeyboardShortcut> {
        let ctrl = Modifiers::COMMAND;
        let ctrl_shift = Modifiers::COMMAND | Modifiers::SHIFT;
        let k = |m, key| Some(KeyboardShortcut::new(m, key));
        match self {
            Command::Play => k(Modifiers::NONE, Key::F5),
            Command::Stop => k(Modifiers::SHIFT, Key::F5),
            Command::TestHere => k(Modifiers::NONE, Key::F6),
            Command::RunTests => k(ctrl, Key::F5),
            Command::Undo => k(ctrl, Key::Z),
            Command::Redo => k(ctrl, Key::Y),
            Command::Save => k(ctrl, Key::S),
            Command::SaveAll => k(ctrl_shift, Key::S),
            Command::NewScene => k(ctrl, Key::N),
            Command::NewProject => k(ctrl_shift, Key::N),
            Command::OpenProject => k(ctrl, Key::O),
            Command::Find => k(ctrl, Key::F),
            Command::Replace => k(ctrl, Key::H),
            Command::GoToLine => k(ctrl, Key::G),
            Command::CloseTab => k(ctrl, Key::W),
            Command::ZoomIn => k(ctrl, Key::Equals),
            Command::ZoomOut => k(ctrl, Key::Minus),
            Command::ZoomReset => k(ctrl, Key::Num0),
            Command::Palette => k(ctrl_shift, Key::P),
            Command::QuickOpen => k(ctrl, Key::P),
            Command::Docs => k(Modifiers::NONE, Key::F1),
            Command::Quit => k(ctrl, Key::Q),
            _ => None,
        }
    }

    /// Só faz sentido com um projeto aberto.
    pub fn needs_project(self) -> bool {
        !matches!(
            self,
            Command::NewProject
                | Command::OpenProject
                | Command::ToggleTheme
                | Command::ToggleMode
                | Command::ZoomIn
                | Command::ZoomOut
                | Command::ZoomReset
                | Command::Palette
                | Command::Docs
                | Command::Tutorials
                | Command::Shortcuts
                | Command::Preferences
                | Command::About
                | Command::Quit
        )
    }

    /// Comandos avançados ficam fora da paleta e dos menus no modo Simples.
    pub fn advanced(self) -> bool {
        matches!(self, Command::OpenExternal | Command::RevealProject)
    }
}

/// Texto sem acentos e em minúsculas, para comparar ("Configurações" →
/// "configuracoes").
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            c => c,
        })
        .collect()
}

/// Busca aproximada: todas as letras da busca aparecem em ordem no texto.
/// Devolve a pontuação (maior = melhor) ou `None` se não combina. Letras
/// seguidas e começos de palavra valem mais.
pub fn fuzzy_score(query: &str, text: &str) -> Option<i32> {
    let q: Vec<char> = fold(query).chars().filter(|c| !c.is_whitespace()).collect();
    if q.is_empty() {
        return Some(0);
    }
    let t: Vec<char> = fold(text).chars().collect();
    let mut score = 0;
    let mut qi = 0;
    let mut last: Option<usize> = None;
    for (i, &c) in t.iter().enumerate() {
        if qi < q.len() && c == q[qi] {
            score += 1;
            if last.is_some_and(|l| l + 1 == i) {
                score += 5;
            }
            let word_start = i == 0 || !t[i - 1].is_alphanumeric();
            if word_start {
                score += 8;
            }
            last = Some(i);
            qi += 1;
        }
    }
    if qi < q.len() {
        return None;
    }
    // Textos curtos ganham um pouco (a busca cobre mais dele).
    Some(score * 10 - t.len() as i32)
}

/// Uma opção da paleta.
#[derive(Debug, Clone, PartialEq)]
pub enum PaletteItem {
    Command(Command),
    /// Arquivo do projeto (caminho relativo).
    File(String),
}

/// Filtra e ordena as opções pela busca.
pub fn search(query: &str, items: &[PaletteItem]) -> Vec<PaletteItem> {
    let mut scored: Vec<(i32, usize, &PaletteItem)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| {
            let score = match item {
                PaletteItem::Command(c) => {
                    let by_title = fuzzy_score(query, c.title());
                    let by_keywords = fuzzy_score(query, c.keywords()).map(|s| s / 2);
                    // Comandos vêm antes de arquivos com nota parecida.
                    by_title.max(by_keywords)? + 40
                }
                PaletteItem::File(path) => {
                    let name = path.rsplit('/').next().unwrap_or(path);
                    fuzzy_score(query, name)
                        .map(|s| s + 20)
                        .max(fuzzy_score(query, path))?
                }
            };
            Some((score, i, item))
        })
        .collect();
    // Maior pontuação primeiro; empate fica na ordem original.
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored
        .into_iter()
        .map(|(_, _, item)| item.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sem_acentos() {
        assert_eq!(fold("Configurações AÇÃO"), "configuracoes acao");
    }

    #[test]
    fn busca_aproximada() {
        assert!(fuzzy_score("cfg", "Configurações do projeto").is_some());
        assert!(fuzzy_score("configuracoes", "Configurações do projeto").is_some());
        assert!(fuzzy_score("xyz", "Configurações").is_none());
        // Começo de palavra e letras seguidas valem mais.
        let a = fuzzy_score("ex", "Exportar jogo...").unwrap();
        let b = fuzzy_score("ex", "Abrir arquivo no editor externo").unwrap();
        assert!(a > b, "{a} {b}");
        assert_eq!(fuzzy_score("", "qualquer"), Some(0));
    }

    #[test]
    fn paleta_ordena_pelo_melhor() {
        let mut items: Vec<_> = Command::ALL
            .iter()
            .map(|c| PaletteItem::Command(*c))
            .collect();
        items.push(PaletteItem::File("cenas/jogo.lua".into()));
        items.push(PaletteItem::File("lib/jogador.lua".into()));
        let r = search("jogar", &items);
        assert_eq!(r[0], PaletteItem::Command(Command::Play));
        let r = search("export", &items);
        assert_eq!(r[0], PaletteItem::Command(Command::Export));
        // Palavras extras: "rodar" acha Jogar.
        let r = search("rodar", &items);
        assert_eq!(r[0], PaletteItem::Command(Command::Play));
        let r = search("testes", &items);
        assert_eq!(r[0], PaletteItem::Command(Command::RunTests));
        // Arquivo pelo nome.
        let r = search("jogador", &items);
        assert_eq!(r[0], PaletteItem::File("lib/jogador.lua".into()));
        // Comando antes do arquivo de nome parecido.
        items.push(PaletteItem::File("config.lua".into()));
        let r = search("config", &items);
        assert_eq!(r[0], PaletteItem::Command(Command::ProjectSettings));
        // Busca vazia: tudo, na ordem.
        assert_eq!(search("", &items).len(), items.len());
    }

    #[test]
    fn atalhos_nao_se_repetem() {
        let mut vistos = std::collections::HashSet::new();
        for c in Command::ALL {
            if let Some(s) = c.shortcut() {
                assert!(vistos.insert((s.modifiers, s.logical_key)), "{c:?}");
            }
        }
    }
}
