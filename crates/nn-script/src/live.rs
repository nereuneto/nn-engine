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
//! salvar. No sentido contrário, o jogo avisa no log (alvo [`LOG_TARGET`])
//! qual cena está na frente, para o editor saber se está "conectado".

use std::collections::VecDeque;
use std::io::BufRead;
use std::sync::Mutex;

/// Alvo do log com a cena que está na frente ("cena: caminho").
pub const LOG_TARGET: &str = "nn_script::live";

/// Mensagens que ainda não foram entregues a nenhuma cena.
const MAX_PENDING: usize = 4096;

static INBOX: Mutex<VecDeque<(String, serde_json::Value)>> = Mutex::new(VecDeque::new());

/// Coloca uma mensagem (uma linha JSON) na fila. Devolve `false` se a linha
/// não for um ajuste válido.
pub fn push(line: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
        return false;
    };
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
}
