//! Canal "ao vivo" entre o NN Studio e o jogo rodando.
//!
//! O editor escreve na entrada padrão do jogo uma linha JSON por ajuste:
//!
//! ```json
//! {"scene": "cenas/fase.cena", "id": 3, "field": "x", "value": 120}
//! {"scene": "cenas/fase.cena", "id": 3, "component": 1, "field": "color", "value": "#FF0000"}
//! ```
//!
//! e a cena visual aberta com aquele arquivo aplica a mudança na hora, sem
//! salvar. Também chegam comandos: `{"cmd": "pause"}`, `"resume"`,
//! `"step"` (avança um quadro com o jogo pausado) e `"show_paths"` /
//! `"hide_paths"` (mostra a grade dos caminhos e o caminho de cada inimigo;
//! também `{"cmd": "paths", "on": true}`).
//!
//! No sentido contrário, o jogo escreve no log (alvo [`LOG_TARGET`]):
//!
//! - `cena: <caminho>`: a cena que está na frente;
//! - `estado: <json>`: dez vezes por segundo, onde estão os objetos da cena
//!   da frente, as propriedades dos comportamentos e as variáveis do jogo;
//! - `selecionar: <id>`: Ctrl+clique num objeto da janela do jogo.

use std::collections::VecDeque;
use std::io::BufRead;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Alvo do log com a cena que está na frente ("cena: caminho").
pub const LOG_TARGET: &str = "nn_script::live";

/// Mensagens que ainda não foram entregues a nenhuma cena.
const MAX_PENDING: usize = 4096;

static INBOX: Mutex<VecDeque<(String, serde_json::Value)>> = Mutex::new(VecDeque::new());

/// O jogo foi aberto pelo editor (escuta a entrada padrão).
static ENABLED: AtomicBool = AtomicBool::new(false);
static PAUSED: AtomicBool = AtomicBool::new(false);
static STEPS: AtomicU32 = AtomicU32::new(0);
static PATHS: AtomicBool = AtomicBool::new(false);

/// Mostrar os caminhos? (Pedido pelo editor, ou `NN_CAMINHOS=1`.)
pub fn show_paths() -> bool {
    PATHS.load(Ordering::Relaxed) || std::env::var("NN_CAMINHOS").is_ok_and(|v| v == "1")
}

/// O canal com o editor está ligado?
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Liga o canal (para testes; `listen_stdin` já liga).
pub fn enable() {
    ENABLED.store(true, Ordering::Relaxed);
}

/// O editor pausou o jogo?
pub fn paused() -> bool {
    PAUSED.load(Ordering::Relaxed)
}

/// Com o jogo pausado, o editor pediu para avançar um quadro? (Consome o
/// pedido.)
pub(crate) fn take_step() -> bool {
    // Laço de compare_exchange em vez de `fetch_update`, que foi renomeado
    // (e marcado como obsoleto) depois da versão mínima do Rust da engine.
    let mut n = STEPS.load(Ordering::Relaxed);
    while n > 0 {
        match STEPS.compare_exchange_weak(n, n - 1, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return true,
            Err(atual) => n = atual,
        }
    }
    false
}

/// Coloca uma mensagem (uma linha JSON) na fila. Devolve `false` se a linha
/// não for um ajuste válido.
pub fn push(line: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
        return false;
    };
    if let Some(cmd) = value.get("cmd").and_then(|c| c.as_str()) {
        match cmd {
            "pause" => PAUSED.store(true, Ordering::Relaxed),
            "resume" => {
                PAUSED.store(false, Ordering::Relaxed);
                STEPS.store(0, Ordering::Relaxed);
            }
            "step" => {
                PAUSED.store(true, Ordering::Relaxed);
                STEPS.fetch_add(1, Ordering::Relaxed);
            }
            "paths" => {
                let on = value.get("on").and_then(|v| v.as_bool()).unwrap_or(true);
                PATHS.store(on, Ordering::Relaxed);
            }
            "show_paths" => PATHS.store(true, Ordering::Relaxed),
            "hide_paths" => PATHS.store(false, Ordering::Relaxed),
            _ => return false,
        }
        return true;
    }
    let Some(scene) = value
        .get("scene")
        .and_then(|s| s.as_str())
        .map(str::to_string)
    else {
        return false;
    };
    if value.get("id").is_none() || value.get("field").is_none() {
        return false;
    }
    let mut inbox = INBOX.lock().unwrap_or_else(|e| e.into_inner());
    inbox.push_back((scene, value));
    while inbox.len() > MAX_PENDING {
        inbox.pop_front();
    }
    true
}

/// Tira da fila as mensagens para a cena `path`.
pub(crate) fn take(path: &str) -> Vec<serde_json::Value> {
    let mut inbox = INBOX.lock().unwrap_or_else(|e| e.into_inner());
    if inbox.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    inbox.retain(|(scene, value)| {
        if scene == path {
            out.push(value.clone());
            false
        } else {
            true
        }
    });
    out
}

/// Escuta a entrada padrão numa thread separada (modo `--jogar` do editor).
pub fn listen_stdin() {
    enable();
    std::thread::spawn(|| {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            if !line.trim().is_empty() && !push(&line) {
                log::debug!("ajuste ao vivo ignorado: {line}");
            }
        }
    });
}

/// Avisa o editor que `path` é a cena da frente.
pub(crate) fn announce(path: &str) {
    log::info!(target: LOG_TARGET, "cena: {path}");
}

/// Manda o estado da cena da frente para o editor.
pub(crate) fn report(state: &serde_json::Value) {
    log::info!(target: LOG_TARGET, "estado: {state}");
}

/// Ctrl+clique num objeto da janela do jogo.
pub(crate) fn select(id: i64) {
    log::info!(target: LOG_TARGET, "selecionar: {id}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fila_por_cena() {
        assert!(push(r#"{"scene":"a.cena","id":1,"field":"x","value":3}"#));
        assert!(push(r#"{"scene":"b.cena","id":2,"field":"y","value":4}"#));
        assert!(!push("não é json"));
        assert!(!push(r#"{"id":1,"field":"x"}"#));
        let a = take("a.cena");
        assert_eq!(a.len(), 1);
        assert_eq!(a[0]["value"], 3);
        assert!(take("a.cena").is_empty());
        assert_eq!(take("b.cena").len(), 1);
    }

    #[test]
    fn pausar_e_avancar() {
        assert!(push(r#"{"cmd":"step"}"#));
        assert!(paused());
        assert!(take_step());
        assert!(!take_step());
        assert!(push(r#"{"cmd":"resume"}"#));
        assert!(!paused());
        assert!(!push(r#"{"cmd":"dançar"}"#));
    }
}
