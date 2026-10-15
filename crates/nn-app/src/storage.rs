//! Dados salvos do jogo (recordes, opções, progresso), num arquivo JSON na
//! pasta de dados do usuário.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

/// Nome do arquivo de dados dentro da pasta do jogo.
pub const SAVE_FILE: &str = "save.json";

/// Dados salvos do jogo: um dicionário de chave → valor JSON.
///
/// Alterações são gravadas no disco sozinhas (no máximo uma vez por
/// [`Storage::AUTO_FLUSH`] e ao fechar o jogo); [`Storage::flush`] grava na hora.
/// A gravação é atômica: um arquivo novo é escrito ao lado e só então
/// substitui o antigo, então faltar luz no meio não corrompe o save.
///
/// ```
/// use nn_app::Storage;
///
/// let mut save = Storage::in_memory();
/// save.set("recorde", 1200);
/// assert_eq!(save.get_as::<u32>("recorde"), Some(1200));
/// ```
#[derive(Debug)]
pub struct Storage {
    path: Option<PathBuf>,
    data: Map<String, Value>,
    dirty: bool,
    last_flush: Option<Instant>,
}

impl Default for Storage {
    fn default() -> Self {
        Self::in_memory()
    }
}

impl Storage {
    /// Intervalo mínimo entre gravações automáticas.
    pub const AUTO_FLUSH: Duration = Duration::from_secs(1);

    /// Dados só na memória (nada é gravado). Usado em testes.
    pub fn in_memory() -> Self {
        Self {
            path: None,
            data: Map::new(),
            dirty: false,
            last_flush: None,
        }
    }

    /// Dados gravados em `path`. Se o arquivo existir mas estiver corrompido,
    /// ele é guardado com a extensão `.corrompido` e o jogo começa do zero.
    pub fn open(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let data = match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<Value>(&bytes) {
                Ok(Value::Object(map)) => map,
                Ok(_) | Err(_) => {
                    let backup = path.with_extension("json.corrompido");
                    log::warn!(
                        "dados salvos inválidos em {}; guardados em {} e começando do zero",
                        path.display(),
                        backup.display()
                    );
                    let _ = std::fs::rename(&path, &backup);
                    Map::new()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Map::new(),
            Err(e) => {
                log::warn!("não foi possível ler {}: {e}", path.display());
                Map::new()
            }
        };
        Self {
            path: Some(path),
            data,
            dirty: false,
            last_flush: None,
        }
    }

    /// Dados do jogo `id` na pasta de dados do usuário:
    /// `~/.local/share/<id>/` no Linux e `%APPDATA%\<id>\data\` no Windows.
    /// Sem pasta de dados disponível, os dados ficam só na memória.
    pub fn for_game(id: &str) -> Self {
        match data_dir(id) {
            Some(dir) => Self::open(dir.join(SAVE_FILE)),
            None => {
                log::warn!("pasta de dados do usuário indisponível; o progresso não será salvo");
                Self::in_memory()
            }
        }
    }

    /// Arquivo onde os dados são gravados (`None` se só na memória).
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Valor de uma chave.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    /// Valor de uma chave convertido para um tipo (`None` se não existe ou
    /// se o tipo não bate).
    pub fn get_as<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Muda o valor de uma chave. `null` apaga a chave.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<Value>) {
        let key = key.into();
        let value = value.into();
        if value.is_null() {
            self.remove(&key);
            return;
        }
        if self.data.get(&key) != Some(&value) {
            self.data.insert(key, value);
            self.dirty = true;
        }
    }

    /// Muda o valor de uma chave a partir de qualquer tipo serializável.
    pub fn set_as<T: Serialize>(
        &mut self,
        key: impl Into<String>,
        value: &T,
    ) -> Result<(), serde_json::Error> {
        let value = serde_json::to_value(value)?;
        self.set(key, value);
        Ok(())
    }

    /// Apaga uma chave.
    pub fn remove(&mut self, key: &str) {
        if self.data.remove(key).is_some() {
            self.dirty = true;
        }
    }

    /// Apaga tudo.
    pub fn clear(&mut self) {
        if !self.data.is_empty() {
            self.data.clear();
            self.dirty = true;
        }
    }

    /// Chaves salvas.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.data.keys().map(String::as_str)
    }

    /// Há alterações ainda não gravadas?
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Grava as alterações no disco agora.
    pub fn flush(&mut self) -> std::io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        self.last_flush = Some(Instant::now());
        let Some(path) = &self.path else {
            self.dirty = false;
            return Ok(());
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_vec_pretty(&self.data).map_err(std::io::Error::other)?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, path)?;
        self.dirty = false;
        Ok(())
    }

    /// Grava se houver alterações e já passou [`Storage::AUTO_FLUSH`] desde a
    /// última gravação. A engine chama isto a cada frame.
    pub fn auto_flush(&mut self) {
        let due = self
            .last_flush
            .is_none_or(|t| t.elapsed() >= Self::AUTO_FLUSH);
        if self.dirty && due {
            self.flush_logged();
        }
    }

    /// [`Storage::flush`] registrando o erro no log em vez de devolvê-lo.
    pub fn flush_logged(&mut self) {
        if let Err(e) = self.flush() {
            let path = self.path.as_deref().unwrap_or(Path::new("?"));
            log::error!("não foi possível salvar em {}: {e}", path.display());
        }
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        // Garantia extra: nada que foi alterado fica sem gravar.
        if self.dirty && self.path.is_some() {
            self.flush_logged();
        }
    }
}

/// Pasta de dados do usuário para o jogo `id`.
pub fn data_dir(id: &str) -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", id).map(|d| d.data_dir().to_path_buf())
}

/// Transforma um título em identificador para pasta: minúsculas, sem
/// acentos nem espaços ("Meu Jogo!" → "meu-jogo").
pub fn slug(title: &str) -> String {
    let mut out = String::new();
    for c in title.chars() {
        let c = match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'ñ' | 'Ñ' => 'n',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        "nn-jogo".into()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nn-save-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn grava_e_le_de_volta() {
        let dir = temp("ida-volta");
        let path = dir.join("sub").join(SAVE_FILE);
        let mut save = Storage::open(&path);
        assert!(save.get("x").is_none());
        save.set("recorde", 42);
        save.set("nome", "Nereu");
        save.set_as(
            "opcoes",
            &serde_json::json!({"musica": 0.5, "tela_cheia": true}),
        )
        .unwrap();
        assert!(save.is_dirty());
        save.flush().unwrap();
        assert!(!save.is_dirty());
        assert!(!path.with_extension("json.tmp").exists());

        let save = Storage::open(&path);
        assert_eq!(save.get_as::<i32>("recorde"), Some(42));
        assert_eq!(save.get_as::<String>("nome").as_deref(), Some("Nereu"));
        assert_eq!(save.get("opcoes").unwrap()["musica"], 0.5);
        assert_eq!(save.get_as::<String>("recorde"), None);
        let mut keys: Vec<_> = save.keys().collect();
        keys.sort();
        assert_eq!(keys, ["nome", "opcoes", "recorde"]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn mesmo_valor_nao_suja_e_null_apaga() {
        let mut save = Storage::in_memory();
        save.set("a", 1);
        save.flush().unwrap();
        save.set("a", 1);
        assert!(!save.is_dirty());
        save.set("a", Value::Null);
        assert!(save.is_dirty());
        assert!(save.get("a").is_none());
        save.clear();
    }

    #[test]
    fn arquivo_corrompido_vira_backup() {
        let dir = temp("corrompido");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(SAVE_FILE);
        std::fs::write(&path, b"{ isto nao e json").unwrap();
        let mut save = Storage::open(&path);
        assert!(save.keys().next().is_none());
        assert!(dir.join("save.json.corrompido").exists());
        save.set("ok", true);
        save.flush().unwrap();
        assert_eq!(Storage::open(&path).get_as::<bool>("ok"), Some(true));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn gravacao_automatica_espera_o_intervalo() {
        let dir = temp("auto");
        let path = dir.join(SAVE_FILE);
        let mut save = Storage::open(&path);
        save.set("a", 1);
        save.auto_flush();
        assert!(path.exists());
        save.set("a", 2);
        save.auto_flush();
        // Gravou há menos de 1 s: fica para depois.
        assert!(save.is_dirty());
        assert_eq!(Storage::open(&path).get_as::<i32>("a"), Some(1));
        // Ao ser descartado, grava o que faltava.
        drop(save);
        assert_eq!(Storage::open(&path).get_as::<i32>("a"), Some(2));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn titulo_vira_identificador() {
        assert_eq!(slug("Meu Jogo!"), "meu-jogo");
        assert_eq!(slug("  Ação & Reação 2 "), "acao-reacao-2");
        assert_eq!(slug("???"), "nn-jogo");
    }
}
