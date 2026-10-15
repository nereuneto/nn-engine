//! Utilidades dos testes: contexto sem janela.

use nn_app::{AppConfig, Context, Game, Headless};

/// Cria um `Headless`, ou `None` se não houver GPU (a menos que `NN_REQUIRE_GPU=1`).
pub fn headless<G: Game>(
    config: AppConfig,
    create: impl FnOnce(&mut Context) -> G,
) -> Option<Headless<G>> {
    match Headless::new(config, create) {
        Ok(h) => Some(h),
        Err(e) => {
            if std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1") {
                panic!("GPU obrigatória mas indisponível: {e}");
            }
            eprintln!("pulando teste: {e}");
            None
        }
    }
}

/// Pasta temporária exclusiva para um teste.
#[allow(dead_code)]
pub fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-teste-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
