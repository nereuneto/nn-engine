//! Aplicação da nn-engine: abre a janela, conecta a GPU e roda o loop principal.
//!
//! O jogo implementa [`Game`] (ou usa uma [`SceneStack`] de [`Scene`]s) e
//! chama [`run`], passando uma função que cria o jogo depois que a GPU está
//! pronta (para já poder carregar texturas). A cada frame a engine:
//!
//! 1. recarrega assets alterados no disco (se a recarga estiver ligada);
//! 2. mede o tempo real que passou;
//! 3. executa [`Game::fixed_update`] zero ou mais vezes (timestep fixo);
//! 4. executa [`Game::update`] uma vez;
//! 5. executa [`Game::render`] e apresenta a imagem;
//! 6. limpa as transições de entrada ("apertou neste frame").
//!
//! [`Headless`] roda o mesmo ciclo sem janela, para testes.

mod assets;
mod config;
mod context;
mod headless;
mod scene;
mod source;
mod window;

pub use assets::{AssetError, Assets};
pub use config::AppConfig;
pub use context::{Context, Game};
pub use headless::Headless;
pub use scene::{Scene, SceneStack, Transition};
pub use source::{AssetSource, DiskSource, Pack, PackError, list_dir, normalize};
pub use window::{AppError, run};
