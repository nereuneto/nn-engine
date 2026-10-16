//! `nn-studio`: o editor da nn-engine.
//!
//! ```text
//! nn-studio                 abre o gerenciador de projetos
//! nn-studio <pasta>         abre o projeto da pasta
//! nn-studio --jogar <pasta> roda o jogo (é assim que o ▶ Jogar abre o jogo)
//! nn-studio --jogar <pasta> --cena <arquivo>   começa nessa cena (Testar daqui)
//! ```

// No Windows, abrir o NN Studio não mostra uma janela de console.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use nn_studio::app::{StudioOptions, run};
use nn_studio::console::format_record;
use nn_studio::game::{PLAY_ARG, SCENE_ARG};
use nn_studio::settings::{SETTINGS_FILE, settings_dir};

const HELP: &str = "\
uso:
  nn-studio                  abre o NN Studio
  nn-studio <pasta>          abre o projeto da pasta
  nn-studio --jogar <pasta>  roda o jogo da pasta numa janela
  nn-studio --jogar <pasta> --cena <arquivo>
                             começa o jogo nessa cena";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some(PLAY_ARG) => {
            let dir = args.get(1).map(String::as_str).unwrap_or(".");
            let scene = args
                .iter()
                .position(|a| a == SCENE_ARG)
                .and_then(|i| args.get(i + 1))
                .map(String::as_str);
            play(dir, scene)
        }
        Some("-h" | "--help" | "--ajuda") => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        other => {
            // Log no terminal e em nn-studio.log; erro que fecha o editor
            // fica guardado para o aviso da próxima vez.
            nn_studio::crash::init(settings_dir().as_deref());
            let options = StudioOptions {
                settings_path: settings_dir().map(|d| d.join(SETTINGS_FILE)),
                open: other.map(PathBuf::from),
                exe: None,
            };
            match run(options) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    log::error!("{e}");
                    eprintln!("erro: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

/// Modo `--jogar`: roda o jogo e manda o log para o console do editor.
/// A entrada padrão traz os ajustes ao vivo do editor.
fn play(dir: &str, scene: Option<&str>) -> ExitCode {
    let filter = "warn,nn_app=info,nn_script=info,nn_render=info,nn_audio=info,nn_physics=info,nn_tilemap=info";
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(filter))
        .format(|buf, record| {
            writeln!(
                buf,
                "{}",
                format_record(record.level(), record.target(), &record.args().to_string())
            )
        })
        .target(env_logger::Target::Stdout)
        .init();
    nn_script::live::listen_stdin();
    let result = nn_script::Project::open(dir)
        .map_err(Into::into)
        .and_then(|mut project| {
            if let Some(scene) = scene {
                project.start = scene.to_string();
            }
            nn_script::run(project)
        });
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log::error!("{e}");
            ExitCode::FAILURE
        }
    }
}
