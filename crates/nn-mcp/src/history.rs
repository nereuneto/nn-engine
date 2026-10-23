//! O que o agente mudou, para desfazer.

use std::path::Path;

/// Quantos passos o servidor lembra.
pub const MAX_STEPS: usize = 100;

/// Um arquivo como estava antes e como o servidor deixou.
#[derive(Debug, Clone)]
struct FileChange {
    rel: String,
    /// `None`: o arquivo não existia.
    before: Option<Vec<u8>>,
    /// `None`: o servidor apagou.
    after: Option<Vec<u8>>,
}

/// Uma chamada de ferramenta que mudou arquivos.
#[derive(Debug, Clone)]
struct Step {
    label: String,
    files: Vec<FileChange>,
}

/// As mudanças feitas pelo servidor, da mais antiga para a mais nova.
#[derive(Debug, Default)]
pub struct History {
    steps: Vec<Step>,
    /// O passo da chamada atual (vários arquivos viram um passo só).
    open: Option<Step>,
}

impl History {
    /// Começa um passo (uma chamada de ferramenta).
    pub fn begin(&mut self, label: &str) {
        self.open = Some(Step {
            label: label.to_string(),
            files: Vec::new(),
        });
    }

    /// Os arquivos que o passo atual mudou até agora.
    pub fn current_files(&self) -> Vec<String> {
        self.open
            .as_ref()
            .map(|s| s.files.iter().map(|f| f.rel.clone()).collect())
            .unwrap_or_default()
    }

    /// Fecha o passo atual (se mudou algo, ele entra na lista).
    pub fn end(&mut self) {
        if let Some(step) = self.open.take()
            && !step.files.is_empty()
        {
            self.steps.push(step);
            if self.steps.len() > MAX_STEPS {
                self.steps.remove(0);
            }
        }
    }

    /// Anota uma mudança no passo atual.
    pub fn record(&mut self, rel: &str, before: Option<Vec<u8>>, after: Option<Vec<u8>>) {
        let step = self.open.get_or_insert_with(|| Step {
            label: String::new(),
            files: Vec::new(),
        });
        // O mesmo arquivo duas vezes no passo: vale o "antes" da primeira.
        if let Some(f) = step.files.iter_mut().find(|f| f.rel == rel) {
            f.after = after;
        } else {
            step.files.push(FileChange {
                rel: rel.to_string(),
                before,
                after,
            });
        }
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Os passos, do mais novo para o mais antigo: (nome, arquivos).
    pub fn list(&self) -> Vec<(String, Vec<String>)> {
        self.steps
            .iter()
            .rev()
            .map(|s| {
                (
                    s.label.clone(),
                    s.files.iter().map(|f| f.rel.clone()).collect(),
                )
            })
            .collect()
    }

    /// Desfaz o passo mais novo. Se algum arquivo mudou por fora depois
    /// (no editor, à mão), só desfaz com `force`.
    pub fn undo(&mut self, root: &Path, force: bool) -> Result<String, String> {
        let Some(step) = self.steps.last() else {
            return Err("não há nada feito por aqui para desfazer".into());
        };
        if !force {
            let changed: Vec<&str> = step
                .files
                .iter()
                .filter(|f| std::fs::read(root.join(&f.rel)).ok() != f.after)
                .map(|f| f.rel.as_str())
                .collect();
            if !changed.is_empty() {
                return Err(format!(
                    "{} mudou depois disso (no editor ou à mão); desfazer perderia essas mudanças. \
                     Use forcar = true para desfazer mesmo assim.",
                    changed.join(", ")
                ));
            }
        }
        let step = self.steps.pop().expect("tem passo");
        let mut done = Vec::new();
        for f in step.files.iter().rev() {
            let full = root.join(&f.rel);
            let r = match &f.before {
                Some(data) => crate::paths::write_atomic(&full, data),
                None if full.exists() => std::fs::remove_file(&full),
                None => Ok(()),
            };
            r.map_err(|e| format!("{}: {e}", f.rel))?;
            done.push(match f.before {
                Some(_) if f.after.is_none() => format!("{} (voltou)", f.rel),
                Some(_) => format!("{} (como estava)", f.rel),
                None => format!("{} (apagado)", f.rel),
            });
        }
        Ok(format!("Desfeito: {}. {}", step.label, done.join(", ")))
    }
}
