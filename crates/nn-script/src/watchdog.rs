//! O vigia do laço sem fim, no jogo com janela.
//!
//! Uma linha separada olha quanto tempo o Lua está rodando sem voltar para a
//! engine. Não custa nada por instrução (um gancho do Lua deixaria o jogo
//! uns 40% mais lento): o jogo só avisa quando entra e sai do Lua e das
//! funções `nn.*`, num relógio que a própria linha do vigia anda.
//!
//! Passou de [`LIMIT`] só no Lua:
//! - se o laço chama alguma função `nn.*`, ela dá erro ali mesmo, e a tela
//!   de erro mostra o arquivo e a linha;
//! - se não chama nada (`while true do end`), não há como parar o Lua de
//!   fora: depois de mais [`GRACE`], o vigia avisa (a cena e o tempo) e fecha
//!   o jogo.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Quanto o Lua pode rodar sem voltar, num quadro, antes de ser cortado.
pub const LIMIT: Duration = Duration::from_secs(5);
/// Depois do corte pedido, quanto esperar o Lua voltar antes de fechar.
pub const GRACE: Duration = Duration::from_secs(3);
/// De quanto em quanto tempo o vigia olha.
const TICK: Duration = Duration::from_millis(100);

/// O jogo travou num laço só de Lua (veja o módulo).
#[derive(Debug, Clone)]
pub struct Hang {
    /// A cena da frente quando travou.
    pub scene: String,
    /// Há quanto tempo o Lua estava rodando sem voltar.
    pub seconds: f32,
}

impl Hang {
    /// A explicação para quem fez o jogo.
    pub fn message(&self) -> String {
        format!(
            "o jogo travou em {}: o Lua rodou {:.0} s sem voltar, num laço que não chama \
             nenhuma função nn.* (como `while true do end`), e o jogo foi fechado. \
             Procure um while ou repeat que nunca termina nessa cena ou nos \
             comportamentos dela.",
            self.scene, self.seconds
        )
    }
}

/// O que o jogo avisa ao vigia (só a linha principal mexe nos contadores; o
/// vigia só lê).
#[derive(Debug, Default)]
pub(crate) struct Watchdog {
    /// Milissegundos desde o começo (+1: zero quer dizer "fora").
    now: AtomicU64,
    /// Quando o Lua começou a rodar sem voltar (0: fora do Lua).
    lua_since: AtomicU64,
    /// Quando entrou numa função `nn.*` (0: fora).
    rust_since: AtomicU64,
    /// Milissegundos dentro de funções `nn.*` desde `lua_since` (não contam:
    /// carregar um modelo grande não é um laço sem fim).
    rust_total: AtomicU64,
    /// Chamadas ao Lua uma dentro da outra.
    depth: AtomicU32,
    /// Pedido de corte: a próxima função `nn.*` dá erro.
    stuck: AtomicBool,
    stop: AtomicBool,
    scene: Mutex<String>,
    /// [`LIMIT`] e [`GRACE`] (menores nos testes).
    limit_ms: u64,
    grace_ms: u64,
}

/// Mantém o vigia rodando; ao sair, a linha dele termina.
#[derive(Debug)]
pub(crate) struct Guard(Arc<Watchdog>);

impl Drop for Guard {
    fn drop(&mut self) {
        self.0.stop.store(true, Relaxed);
    }
}

impl Watchdog {
    /// Começa a linha do vigia. `on_hang` roda nela quando o jogo trava num
    /// laço só de Lua (e deve fechar o jogo: a linha principal não volta).
    pub(crate) fn start(on_hang: Box<dyn FnOnce(Hang) + Send>) -> (Arc<Self>, Guard) {
        Self::start_with(LIMIT, GRACE, on_hang)
    }

    fn start_with(
        limit: Duration,
        grace: Duration,
        on_hang: Box<dyn FnOnce(Hang) + Send>,
    ) -> (Arc<Self>, Guard) {
        let watch = Arc::new(Self {
            now: AtomicU64::new(1),
            limit_ms: limit.as_millis() as u64,
            grace_ms: grace.as_millis() as u64,
            ..Self::default()
        });
        let w = watch.clone();
        let spawned = std::thread::Builder::new()
            .name("nn-vigia".into())
            .spawn(move || w.run(on_hang));
        if let Err(e) = spawned {
            log::warn!("o vigia do laço sem fim não começou: {e}");
        }
        (watch.clone(), Guard(watch))
    }

    fn run(&self, on_hang: Box<dyn FnOnce(Hang) + Send>) {
        let start = Instant::now();
        // A vez (o `lua_since`) em que o corte foi pedido, e quando.
        let mut asked: Option<(u64, u64)> = None;
        loop {
            std::thread::sleep(TICK);
            if self.stop.load(Relaxed) {
                return;
            }
            let now = start.elapsed().as_millis() as u64 + 1;
            self.now.store(now, Relaxed);
            let since = self.lua_since.load(Relaxed);
            if since == 0 {
                asked = None;
                continue;
            }
            if asked.is_some_and(|(s, _)| s != since) {
                asked = None;
            }
            // Dentro de uma função da engine: não é o Lua que está preso.
            if self.rust_since.load(Relaxed) != 0 {
                continue;
            }
            let lua_ms = now
                .saturating_sub(since)
                .saturating_sub(self.rust_total.load(Relaxed));
            if lua_ms <= self.limit_ms {
                continue;
            }
            match asked {
                None => {
                    self.stuck.store(true, Relaxed);
                    asked = Some((since, now));
                }
                Some((_, at)) if now - at > self.grace_ms => {
                    let scene = self.scene.lock().map(|s| s.clone()).unwrap_or_default();
                    on_hang(Hang {
                        scene,
                        seconds: lua_ms as f32 / 1000.0,
                    });
                    return;
                }
                Some(_) => {}
            }
        }
    }

    /// A cena da frente (para o aviso).
    pub(crate) fn set_scene(&self, path: &str) {
        if let Ok(mut s) = self.scene.lock() {
            s.clear();
            s.push_str(path);
        }
    }

    /// O jogo vai rodar Lua (a cena, um comportamento...).
    pub(crate) fn enter_lua(&self) {
        if self.depth.fetch_add(1, Relaxed) == 0 {
            self.rust_total.store(0, Relaxed);
            self.stuck.store(false, Relaxed);
            self.lua_since.store(self.now.load(Relaxed), Relaxed);
        }
    }

    pub(crate) fn leave_lua(&self) {
        if self.depth.fetch_sub(1, Relaxed) == 1 {
            self.lua_since.store(0, Relaxed);
        }
    }

    /// O Lua chamou uma função `nn.*`. `false`: o quadro passou do limite e
    /// a função deve dar erro (veja [`cut_message`]).
    pub(crate) fn enter_rust(&self) -> bool {
        if self.stuck.swap(false, Relaxed) {
            return false;
        }
        self.rust_since.store(self.now.load(Relaxed), Relaxed);
        true
    }

    pub(crate) fn leave_rust(&self) {
        let since = self.rust_since.swap(0, Relaxed);
        if since != 0 {
            let spent = self.now.load(Relaxed).saturating_sub(since);
            self.rust_total.fetch_add(spent, Relaxed);
        }
    }
}

/// O erro da função `nn.*` que cortou o quadro.
pub(crate) fn cut_message() -> String {
    format!(
        "o jogo travou: o Lua rodou mais de {} s sem voltar (um laço sem fim?)",
        LIMIT.as_secs()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_tempo_nas_funcoes_da_engine_nao_conta() {
        let w = Watchdog::default();
        w.now.store(1000, Relaxed);
        w.enter_lua();
        assert!(w.enter_rust());
        w.now.store(9000, Relaxed);
        w.leave_rust();
        assert_eq!(w.rust_total.load(Relaxed), 8000);
        w.leave_lua();
        assert_eq!(w.lua_since.load(Relaxed), 0);
    }

    #[test]
    fn o_corte_vale_uma_vez() {
        let w = Watchdog::default();
        w.enter_lua();
        w.stuck.store(true, Relaxed);
        assert!(!w.enter_rust());
        assert!(w.enter_rust());
        w.leave_rust();
        w.leave_lua();
    }

    #[test]
    fn um_laco_so_de_lua_fecha_o_jogo_com_a_cena() {
        let (tx, rx) = std::sync::mpsc::channel();
        let (limit, grace) = (Duration::from_millis(300), Duration::from_millis(300));
        let (w, _guard) =
            Watchdog::start_with(limit, grace, Box::new(move |h| tx.send(h).unwrap()));
        w.set_scene("cenas/fase.lua");
        w.enter_lua();
        // Sem chamar nada: o vigia pede o corte e, sem resposta, avisa.
        let hang = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("o vigia avisou");
        assert_eq!(hang.scene, "cenas/fase.lua");
        assert!(hang.seconds >= 0.6, "{hang:?}");
        assert!(hang.message().contains("cenas/fase.lua"));
    }

    #[test]
    fn quem_chama_a_engine_e_cortado_e_nao_fecha() {
        let (tx, rx) = std::sync::mpsc::channel();
        let (limit, grace) = (Duration::from_millis(300), Duration::from_millis(5000));
        let (w, _guard) =
            Watchdog::start_with(limit, grace, Box::new(move |h| tx.send(h).unwrap()));
        w.enter_lua();
        let t = Instant::now();
        // Um laço que chama nn.* a cada volta: a chamada dá erro.
        let cut = loop {
            if !w.enter_rust() {
                break t.elapsed();
            }
            w.leave_rust();
            assert!(t.elapsed() < Duration::from_secs(5), "não cortou");
            std::thread::sleep(Duration::from_millis(5));
        };
        assert!(cut >= limit, "{cut:?}");
        // O erro fez o Lua voltar: nada de fechar o jogo.
        w.leave_lua();
        assert!(rx.recv_timeout(Duration::from_millis(500)).is_err());
    }
}
