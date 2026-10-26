//! Erros no jogo exportado: o arquivo `<nome>-erro.txt` para quem fez o
//! jogo, e uma tela simples para quem joga.
//!
//! No jogo em desenvolvimento (`nn`, NN Studio) nada disto é usado: a tela de
//! erro mostra a mensagem inteira e a cena recarrega ao salvar.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use glam::Vec2;
use nn_app::{AppConfig, Context, Game};
use nn_core::Color;
use nn_render::{Anchor, Text};

/// O arquivo de erro de um jogo exportado.
#[derive(Debug)]
pub struct ErrorReport {
    file: PathBuf,
    log: Option<PathBuf>,
    title: String,
    /// Onde o arquivo foi escrito (ao lado do executável ou, sem permissão,
    /// na pasta temporária); `None` antes do primeiro erro.
    written: Mutex<Option<PathBuf>>,
    /// Abre a tela de erro num processo novo (quando o jogo vai fechar e não
    /// dá para mostrar a tela nele).
    pub show: Option<fn(&ErrorReport)>,
}

impl ErrorReport {
    /// `file` é onde escrever (`<nome>-erro.txt` ao lado do executável);
    /// `log`, o log completo do jogo, citado no arquivo.
    pub fn new(file: impl Into<PathBuf>, log: Option<PathBuf>, title: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            log,
            title: title.into(),
            written: Mutex::new(None),
            show: None,
        }
    }

    /// O nome do jogo.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Onde o arquivo está, se algum erro já foi escrito.
    pub fn path(&self) -> Option<PathBuf> {
        self.written.lock().ok().and_then(|w| w.clone())
    }

    /// Escreve um erro. O primeiro começa o arquivo (com o nome do jogo e a
    /// versão); os seguintes vão no fim. Devolve onde ficou.
    pub fn write(&self, what: &str) -> Option<PathBuf> {
        let mut written = self.written.lock().unwrap_or_else(|e| e.into_inner());
        let text = match &*written {
            None => self.header() + what.trim_end() + "\n",
            Some(_) => format!(
                "\n----------------------------------------\n\n{}\n",
                what.trim_end()
            ),
        };
        let first = written.is_none();
        let fallback = std::env::temp_dir().join(self.file.file_name().unwrap_or_default());
        let tries = match &*written {
            Some(p) => vec![p.clone()],
            None => vec![self.file.clone(), fallback],
        };
        for path in tries {
            let file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(first)
                .append(!first)
                .open(&path);
            if let Ok(mut f) = file
                && f.write_all(text.as_bytes()).is_ok()
            {
                *written = Some(path.clone());
                return Some(path);
            }
        }
        log::error!(
            "não foi possível escrever o arquivo de erro {}",
            self.file.display()
        );
        None
    }

    fn header(&self) -> String {
        let mut s = format!(
            "O jogo \"{}\" encontrou um problema.\nMande este arquivo para quem fez o jogo.\n\n\
             nn-engine {} ({}/{})\n",
            self.title,
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
        );
        if let Some(log) = &self.log {
            s.push_str(&format!("Log completo: {}\n", log.display()));
        }
        s.push('\n');
        s
    }
}

/// A tela simples de erro, para quem joga: o que aconteceu, onde está o
/// arquivo e como fechar. `file` `None`: não deu para escrever o arquivo.
/// `closed`: o jogo já fechou (é a tela sozinha, num processo novo).
pub(crate) fn draw_player_error(ctx: &mut Context, file: Option<&Path>, closed: bool) {
    let view = ctx.viewport().as_vec2();
    let k = (view.y / 450.0).clamp(0.6, 2.5);
    let margin = 40.0 * k;
    let width = (view.x - margin * 2.0).max(10.0);
    let path = file.map(|p| p.display().to_string());

    // O caminho cabe na largura: diminui a letra se for comprido.
    let mut path_size = 18.0 * k;
    if let Some(p) = &path {
        let w = ctx.renderer.measure_text(&Text::new(p).size(path_size)).x;
        if w > width {
            path_size = (path_size * width / w).max(8.0);
        }
    }

    let mut screen = ctx.renderer.screen();
    screen.draw_rect(Vec2::ZERO, view, Color::hex(0x141A26));
    screen.draw_rect(Vec2::ZERO, Vec2::new(view.x, 6.0 * k), Color::hex(0xE5A84D));
    let mut y = margin + 10.0 * k;
    let mut line =
        |screen: &mut nn_render::ScreenPainter<'_>, text: &str, size: f32, color: u32, gap: f32| {
            screen.draw_text(
                &Text::new(text)
                    .at(Vec2::new(margin, y))
                    .size(size)
                    .color(Color::hex(color)),
            );
            y += size * 1.3 + gap;
        };
    line(
        &mut screen,
        "O jogo encontrou um problema",
        34.0 * k,
        0xFFFFFF,
        18.0 * k,
    );
    match &path {
        Some(p) => {
            line(
                &mut screen,
                "Os detalhes foram salvos neste arquivo:",
                20.0 * k,
                0xC8D0E0,
                6.0 * k,
            );
            line(&mut screen, p, path_size, 0xFFD27F, 14.0 * k);
            line(
                &mut screen,
                "Mande o arquivo para quem fez o jogo.",
                20.0 * k,
                0xC8D0E0,
                0.0,
            );
        }
        None => {
            line(
                &mut screen,
                "Não foi possível salvar os detalhes do erro.",
                20.0 * k,
                0xC8D0E0,
                0.0,
            );
        }
    }
    screen.draw_text(
        &Text::new(if closed {
            "O jogo foi fechado. Esc fecha esta janela."
        } else {
            "Esc fecha o jogo."
        })
        .at(Vec2::new(margin, view.y - margin))
        .size(16.0 * k)
        .color(Color::hex(0x8A94A8))
        .anchor(Anchor::BottomLeft),
    );
}

/// A tela de erro sozinha, num processo novo (o jogo fechou por um erro que
/// não deixava mostrar a tela nele).
#[derive(Debug)]
pub struct ErrorScreen {
    file: Option<PathBuf>,
}

impl ErrorScreen {
    pub fn new(file: Option<PathBuf>) -> Self {
        Self { file }
    }

    /// A configuração da janela da tela de erro.
    pub fn config(title: &str) -> AppConfig {
        AppConfig::new(format!("{title}: erro"))
            .size(800, 450)
            .clear_color(Color::hex(0x141A26))
    }
}

impl Game for ErrorScreen {
    fn update(&mut self, ctx: &mut Context) {
        if ctx.input.key_pressed(nn_input::KeyCode::Escape) {
            ctx.exit();
        }
    }

    fn render(&mut self, ctx: &mut Context) {
        draw_player_error(ctx, self.file.as_deref(), true);
    }
}

/// Abre a janela da tela de erro e espera fechar.
pub fn run_error_screen(title: &str, file: Option<PathBuf>) -> Result<(), nn_app::AppError> {
    nn_app::run(ErrorScreen::config(title), move |_| ErrorScreen::new(file))
}

/// Como [`run_error_screen`], sem janela: desenha alguns quadros e grava a
/// tela em `screenshot` (para testar o jogo exportado onde não há tela).
pub fn run_error_screen_headless(
    title: &str,
    file: Option<PathBuf>,
    screenshot: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut h = nn_app::Headless::new(ErrorScreen::config(title), move |_| ErrorScreen::new(file))?;
    h.run_frames(3);
    h.save_png(screenshot)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_primeiro_erro_comeca_o_arquivo_e_os_outros_vao_no_fim() {
        let dir = std::env::temp_dir().join(format!("nn-relatorio-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("jogo-erro.txt");
        std::fs::write(&file, "de outra vez").unwrap();
        let r = ErrorReport::new(&file, Some(dir.join("jogo.log")), "Meu Jogo");
        assert_eq!(r.path(), None);
        assert_eq!(
            r.write("Erro no script cenas/a.lua:\nbum").as_deref(),
            Some(file.as_path())
        );
        r.write("Erro no script cenas/b.lua:\nbam");
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(!text.contains("de outra vez"), "{text}");
        assert!(
            text.starts_with("O jogo \"Meu Jogo\" encontrou um problema."),
            "{text}"
        );
        assert!(text.contains("jogo.log"), "{text}");
        let a = text.find("cenas/a.lua").unwrap();
        let b = text.find("cenas/b.lua").unwrap();
        assert!(a < b, "{text}");
    }

    #[test]
    fn sem_permissao_vai_para_a_pasta_temporaria() {
        let r = ErrorReport::new("/pasta/que/nao/existe/nn-teste-erro.txt", None, "Jogo");
        let path = r.write("bum").expect("escreveu em algum lugar");
        assert_eq!(path, std::env::temp_dir().join("nn-teste-erro.txt"));
        let _ = std::fs::remove_file(path);
    }
}
