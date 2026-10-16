//! ▶ Jogar: o jogo roda num processo separado (o próprio `nn-studio` com
//! `--jogar <pasta>`), na sua própria janela. A saída dele vem para o console.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Instant;

/// Argumento que faz o executável rodar um jogo em vez do editor.
pub const PLAY_ARG: &str = "--jogar";

#[derive(Debug)]
pub struct GameProcess {
    child: Child,
    lines: Receiver<String>,
    started: Instant,
    project: PathBuf,
}

impl GameProcess {
    /// Abre o jogo da pasta `project` com o executável `exe`.
    /// `wake` é chamado quando chega saída nova (para redesenhar o editor).
    pub fn start(
        exe: &Path,
        project: &Path,
        wake: impl Fn() + Send + Sync + Clone + 'static,
    ) -> std::io::Result<Self> {
        let mut child = Command::new(exe)
            .arg(PLAY_ARG)
            .arg(project)
            .current_dir(project)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let (tx, rx) = channel();
        if let Some(out) = child.stdout.take() {
            spawn_reader(out, tx.clone(), wake.clone());
        }
        if let Some(err) = child.stderr.take() {
            spawn_reader(err, tx, wake);
        }
        Ok(Self {
            child,
            lines: rx,
            started: Instant::now(),
            project: project.to_path_buf(),
        })
    }

    /// Linhas novas da saída do jogo.
    pub fn drain(&self) -> Vec<String> {
        self.lines.try_iter().collect()
    }

    /// `Some` quando o jogo terminou.
    pub fn exited(&mut self) -> Option<ExitStatus> {
        self.child.try_wait().ok().flatten()
    }

    pub fn running_for(&self) -> std::time::Duration {
        self.started.elapsed()
    }

    pub fn project(&self) -> &Path {
        &self.project
    }

    /// Fecha o jogo.
    pub fn stop(&mut self) {
        if self.exited().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

impl Drop for GameProcess {
    fn drop(&mut self) {
        self.stop();
    }
}

fn spawn_reader(
    stream: impl Read + Send + 'static,
    tx: Sender<String>,
    wake: impl Fn() + Send + 'static,
) {
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stream);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match reader.read_until(b'\n', &mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let line = String::from_utf8_lossy(&buf);
                    let line = line.trim_end_matches(['\n', '\r']).to_string();
                    if tx.send(line).is_err() {
                        break;
                    }
                    wake();
                }
            }
        }
    });
}

/// Texto para "jogo rodando há ...".
pub fn format_duration(d: std::time::Duration) -> String {
    let s = d.as_secs();
    format!("{}:{:02}", s / 60, s % 60)
}

/// Explica como o jogo terminou, se não foi normalmente.
pub fn describe_exit(status: ExitStatus) -> Option<String> {
    if status.success() {
        return None;
    }
    Some(match status.code() {
        Some(code) => format!("O jogo fechou com erro (código {code})."),
        None => "O jogo foi encerrado.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duracao() {
        assert_eq!(format_duration(std::time::Duration::from_secs(5)), "0:05");
        assert_eq!(format_duration(std::time::Duration::from_secs(125)), "2:05");
    }

    /// Roda um "jogo" de mentira (um comando do sistema que escreve e sai)
    /// para conferir a leitura da saída.
    #[cfg(unix)]
    #[test]
    fn le_saida_e_fim_do_processo() {
        let dir = std::env::temp_dir().join(format!("nn-studio-game-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("falso.sh");
        std::fs::write(
            &script,
            "#!/bin/sh\necho \"arg: $1\"\necho erro >&2\nexit 3\n",
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut g = GameProcess::start(&script, &dir, || {}).unwrap();
        let start = Instant::now();
        let mut lines = Vec::new();
        let status = loop {
            lines.extend(g.drain());
            if let Some(s) = g.exited() {
                std::thread::sleep(std::time::Duration::from_millis(50));
                lines.extend(g.drain());
                break s;
            }
            assert!(start.elapsed().as_secs() < 10);
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert!(lines.contains(&format!("arg: {PLAY_ARG}")), "{lines:?}");
        assert!(lines.contains(&"erro".to_string()), "{lines:?}");
        assert_eq!(
            describe_exit(status).as_deref(),
            Some("O jogo fechou com erro (código 3).")
        );
    }
}
