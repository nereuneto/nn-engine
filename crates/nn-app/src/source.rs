//! De onde vêm os assets: uma pasta no disco ou um pacote dentro do executável.

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

/// Origem dos arquivos do jogo.
///
/// Os caminhos são sempre relativos à raiz do jogo e usam `/`.
pub trait AssetSource: Debug {
    /// Conteúdo do arquivo.
    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>>;

    /// Última modificação, se a origem permitir saber (usado na recarga).
    fn modified(&self, path: &Path) -> Option<SystemTime>;

    /// Arquivos com a extensão dada (sem o ponto), com a data de modificação.
    fn files(&self, extension: &str) -> Vec<(String, Option<SystemTime>)>;

    /// Nome do arquivo para mensagens de erro (caminho completo no disco,
    /// ou o caminho dentro do pacote).
    fn describe(&self, path: &Path) -> PathBuf;

    /// Os arquivos vêm de um pacote (não mudam, sem recarga).
    fn is_packed(&self) -> bool;
}

/// Caminho relativo normalizado: `/` como separador, sem `.` e com `..` resolvido.
pub fn normalize(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for c in path.components() {
        match c {
            Component::Normal(p) => parts.push(p.to_string_lossy().into_owned()),
            Component::ParentDir => {
                parts.pop();
            }
            _ => {}
        }
    }
    parts.join("/")
}

/// Arquivos numa pasta do disco.
#[derive(Debug, Clone)]
pub struct DiskSource {
    root: PathBuf,
}

impl DiskSource {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl AssetSource for DiskSource {
    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        std::fs::read(self.root.join(path))
    }

    fn modified(&self, path: &Path) -> Option<SystemTime> {
        std::fs::metadata(self.root.join(path))
            .and_then(|m| m.modified())
            .ok()
    }

    fn files(&self, extension: &str) -> Vec<(String, Option<SystemTime>)> {
        let mut out = Vec::new();
        walk(&self.root, &self.root, 0, &mut |rel, full| {
            if full.extension().is_some_and(|e| e == extension) {
                let modified = std::fs::metadata(full).and_then(|m| m.modified()).ok();
                out.push((rel, modified));
            }
        });
        out
    }

    fn describe(&self, path: &Path) -> PathBuf {
        self.root.join(path)
    }

    fn is_packed(&self) -> bool {
        false
    }
}

/// Percorre a pasta, pulando arquivos ocultos e pastas de build.
fn walk(root: &Path, dir: &Path, depth: u32, f: &mut impl FnMut(String, &Path)) {
    if depth > 16 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "target" {
            continue;
        }
        let path = entry.path();
        match entry.file_type() {
            Ok(t) if t.is_dir() => walk(root, &path, depth + 1, f),
            Ok(t) if t.is_file() => {
                if let Ok(rel) = path.strip_prefix(root) {
                    f(normalize(rel), &path);
                }
            }
            _ => {}
        }
    }
}

/// Todos os arquivos de uma pasta (caminho relativo normalizado, caminho completo).
pub fn list_dir(root: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    walk(root, root, 0, &mut |rel, full| {
        out.push((rel, full.to_path_buf()))
    });
    out
}

/// Erros ao ler ou montar pacotes.
#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("pacote inválido: {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Pacote de arquivos do jogo, guardado no fim do executável.
///
/// Formato: `NNPACK01`, quantidade de arquivos (u32), e para cada arquivo o
/// tamanho do caminho (u16), o caminho (UTF-8), o deslocamento e o tamanho
/// dos dados (u64); depois, os dados. Tudo em little-endian. Quando anexado
/// a um executável, vem seguido do tamanho do pacote (u64) e de `NNPAKEND`.
#[derive(Debug, Clone)]
pub struct Pack {
    data: Arc<[u8]>,
    entries: BTreeMap<String, (usize, usize)>,
}

const MAGIC: &[u8; 8] = b"NNPACK01";
const TRAILER: &[u8; 8] = b"NNPAKEND";

impl Pack {
    /// Monta os bytes de um pacote com os arquivos dados.
    pub fn build<'a>(files: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> Vec<u8> {
        let files: Vec<_> = files.into_iter().collect();
        let mut header = Vec::new();
        header.extend_from_slice(MAGIC);
        header.extend_from_slice(&(files.len() as u32).to_le_bytes());
        let mut offset = 0u64;
        for (path, data) in &files {
            header.extend_from_slice(&(path.len() as u16).to_le_bytes());
            header.extend_from_slice(path.as_bytes());
            header.extend_from_slice(&offset.to_le_bytes());
            header.extend_from_slice(&(data.len() as u64).to_le_bytes());
            offset += data.len() as u64;
        }
        let mut out = header;
        for (_, data) in files {
            out.extend_from_slice(data);
        }
        out
    }

    /// Lê um pacote a partir dos seus bytes.
    pub fn parse(bytes: Vec<u8>) -> Result<Self, PackError> {
        let mut cur = 0usize;
        let take = |cur: &mut usize, n: usize| -> Result<&[u8], PackError> {
            let s = bytes
                .get(*cur..*cur + n)
                .ok_or(PackError::Invalid("cabeçalho cortado"))?;
            *cur += n;
            Ok(s)
        };
        if take(&mut cur, 8)? != MAGIC {
            return Err(PackError::Invalid("assinatura desconhecida"));
        }
        let count = u32::from_le_bytes(take(&mut cur, 4)?.try_into().unwrap_or_default());
        let mut raw = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let len = u16::from_le_bytes(take(&mut cur, 2)?.try_into().unwrap_or_default());
            let path = std::str::from_utf8(take(&mut cur, len as usize)?)
                .map_err(|_| PackError::Invalid("caminho não é UTF-8"))?
                .to_string();
            let offset = u64::from_le_bytes(take(&mut cur, 8)?.try_into().unwrap_or_default());
            let size = u64::from_le_bytes(take(&mut cur, 8)?.try_into().unwrap_or_default());
            raw.push((path, offset as usize, size as usize));
        }
        let data_start = cur;
        let mut entries = BTreeMap::new();
        for (path, offset, size) in raw {
            let start = data_start
                .checked_add(offset)
                .ok_or(PackError::Invalid("deslocamento inválido"))?;
            if start.checked_add(size).is_none_or(|end| end > bytes.len()) {
                return Err(PackError::Invalid("arquivo além do fim do pacote"));
            }
            entries.insert(path, (start, size));
        }
        Ok(Self {
            data: bytes.into(),
            entries,
        })
    }

    /// Junta um executável e um pacote num arquivo só.
    pub fn append_to(executable: &[u8], pack: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(executable.len() + pack.len() + 16);
        out.extend_from_slice(executable);
        out.extend_from_slice(pack);
        out.extend_from_slice(&(pack.len() as u64).to_le_bytes());
        out.extend_from_slice(TRAILER);
        out
    }

    /// Lê o pacote anexado ao fim de um executável, se houver.
    pub fn read_appended(executable: &Path) -> Result<Option<Self>, PackError> {
        let mut file = std::fs::File::open(executable)?;
        let len = file.metadata()?.len();
        if len < 16 {
            return Ok(None);
        }
        file.seek(SeekFrom::Start(len - 16))?;
        let mut trailer = [0u8; 16];
        file.read_exact(&mut trailer)?;
        if &trailer[8..] != TRAILER {
            return Ok(None);
        }
        let pack_len = u64::from_le_bytes(trailer[..8].try_into().unwrap_or_default());
        if pack_len > len - 16 {
            return Err(PackError::Invalid("tamanho do pacote inválido"));
        }
        file.seek(SeekFrom::Start(len - 16 - pack_len))?;
        let mut bytes = vec![0u8; pack_len as usize];
        file.read_exact(&mut bytes)?;
        Self::parse(bytes).map(Some)
    }

    /// Caminhos de todos os arquivos.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, path: &str) -> Option<&[u8]> {
        self.entries
            .get(path)
            .map(|&(start, size)| &self.data[start..start + size])
    }
}

impl AssetSource for Pack {
    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        self.get(&normalize(path))
            .map(<[u8]>::to_vec)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "não está no pacote"))
    }

    fn modified(&self, _path: &Path) -> Option<SystemTime> {
        None
    }

    fn files(&self, extension: &str) -> Vec<(String, Option<SystemTime>)> {
        let suffix = format!(".{extension}");
        self.paths()
            .filter(|p| p.ends_with(&suffix))
            .map(|p| (p.to_string(), None))
            .collect()
    }

    fn describe(&self, path: &Path) -> PathBuf {
        PathBuf::from(format!("(pacote)/{}", normalize(path)))
    }

    fn is_packed(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normaliza_caminhos() {
        assert_eq!(normalize(Path::new("./a/b/../c.png")), "a/c.png");
        assert_eq!(
            normalize(Path::new("fases/../imagens/x.png")),
            "imagens/x.png"
        );
    }

    #[test]
    fn pacote_ida_e_volta() {
        let bytes = Pack::build([
            ("main.lua", b"return {}".as_slice()),
            ("a/b.png", &[1, 2, 3]),
        ]);
        let pack = Pack::parse(bytes).unwrap();
        assert_eq!(pack.len(), 2);
        assert_eq!(pack.get("a/b.png"), Some([1u8, 2, 3].as_slice()));
        assert_eq!(pack.read(Path::new("./main.lua")).unwrap(), b"return {}");
        assert!(pack.read(Path::new("nada")).is_err());
        assert_eq!(pack.files("lua").len(), 1);
    }

    #[test]
    fn pacote_corrompido_e_rejeitado() {
        assert!(Pack::parse(b"lixo".to_vec()).is_err());
        let mut bytes = Pack::build([("x", b"12345".as_slice())]);
        bytes.truncate(bytes.len() - 2);
        assert!(Pack::parse(bytes).is_err());
    }

    #[test]
    fn anexado_ao_executavel() {
        let dir = std::env::temp_dir().join(format!("nn-pack-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("jogo");
        let pack = Pack::build([("main.lua", b"ok".as_slice())]);
        std::fs::write(&exe, Pack::append_to(b"EXECUTAVEL FALSO", &pack)).unwrap();
        let lido = Pack::read_appended(&exe).unwrap().unwrap();
        assert_eq!(lido.get("main.lua"), Some(b"ok".as_slice()));

        std::fs::write(&exe, b"sem pacote nenhum aqui").unwrap();
        assert!(Pack::read_appended(&exe).unwrap().is_none());
    }
}
