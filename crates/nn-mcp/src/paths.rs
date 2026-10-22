//! Caminhos dentro do projeto, gravação segura e utilidades de texto.

use std::path::{Component, Path, PathBuf};

/// Pastas que não aparecem nas listas (e onde o agente não escreve).
pub const SKIP_DIRS: [&str; 4] = [".git", "dist", "target", ".nn"];

/// Caminho relativo à pasta do projeto, com `/` (o jeito dos arquivos de
/// cena). Recusa caminhos absolutos e `..`.
pub fn clean(rel: &str) -> Result<String, String> {
    let rel = rel.trim().replace('\\', "/");
    let rel = rel.trim_start_matches("./");
    if rel.is_empty() {
        return Err("caminho vazio".into());
    }
    let p = Path::new(rel);
    let mut parts = Vec::new();
    for c in p.components() {
        match c {
            Component::Normal(s) => parts.push(s.to_string_lossy().into_owned()),
            Component::CurDir => {}
            _ => {
                return Err(format!(
                    "{rel}: use um caminho relativo à pasta do projeto, sem \"..\" (ex.: cenas/fase.cena)"
                ));
            }
        }
    }
    if parts.is_empty() {
        return Err("caminho vazio".into());
    }
    Ok(parts.join("/"))
}

/// O arquivo no disco, garantindo que fica dentro do projeto (nem por link
/// simbólico ele sai).
pub fn resolve(root: &Path, rel: &str) -> Result<(String, PathBuf), String> {
    let rel = clean(rel)?;
    if rel
        .split('/')
        .next()
        .is_some_and(|first| SKIP_DIRS.contains(&first))
    {
        return Err(format!("{rel}: esta pasta não é do jogo"));
    }
    let full = root.join(&rel);
    // O pedaço que já existe precisa continuar dentro da pasta.
    let mut existing = full.as_path();
    while !existing.exists() {
        match existing.parent() {
            Some(p) => existing = p,
            None => break,
        }
    }
    if let (Ok(real), Ok(base)) = (existing.canonicalize(), root.canonicalize())
        && !real.starts_with(&base)
    {
        return Err(format!("{rel}: fica fora da pasta do projeto"));
    }
    Ok((rel, full))
}

/// O caminho completo da pasta, sem o `\\?\` que o Windows põe (para
/// mostrar ao agente um caminho que ele reconhece).
pub fn canonical(p: &Path) -> PathBuf {
    let c = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    if cfg!(windows) {
        let s = c.to_string_lossy();
        if let Some(rest) = s.strip_prefix(r"\\?\")
            && !rest.starts_with("UNC")
        {
            return PathBuf::from(rest.to_string());
        }
    }
    c
}

/// Grava num arquivo temporário e troca de nome: quem lê (o jogo rodando, o
/// NN Studio) nunca vê o arquivo pela metade.
pub fn write_atomic(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!(
        "{}.nn-tmp",
        path.extension()
            .map(|e| e.to_string_lossy().into_owned())
            .unwrap_or_default()
    ));
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// Todos os arquivos do projeto (relativos, com `/`), em ordem.
pub fn list_files(root: &Path, under: &str) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if p.is_dir() {
                if dir == root && SKIP_DIRS.contains(&name.as_str()) {
                    continue;
                }
                walk(root, &p, out);
            } else if !name.ends_with(".nn-tmp")
                && let Ok(rel) = p.strip_prefix(root)
            {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    let start = if under.is_empty() {
        root.to_path_buf()
    } else {
        root.join(under)
    };
    walk(root, &start, &mut out);
    out
}

/// Texto sem acentos e em minúsculas (para buscar).
pub fn fold(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' => 'a',
            'é' | 'ê' | 'è' | 'É' | 'Ê' => 'e',
            'í' | 'î' | 'Í' => 'i',
            'ó' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ô' | 'Õ' => 'o',
            'ú' | 'ü' | 'Ú' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

/// Base64 (para mandar imagens ao agente).
pub fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caminhos_ficam_dentro_do_projeto() {
        assert_eq!(clean("cenas/fase.cena").unwrap(), "cenas/fase.cena");
        assert_eq!(clean("./cenas\\fase.cena").unwrap(), "cenas/fase.cena");
        assert!(clean("../fora.lua").is_err());
        assert!(clean("cenas/../../fora.lua").is_err());
        assert!(clean("/etc/passwd").is_err());
        assert!(clean("").is_err());
        let root = std::env::temp_dir();
        assert!(resolve(&root, ".git/config").is_err());
    }

    #[test]
    fn base64_igual_ao_padrao() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn busca_sem_acento() {
        assert_eq!(fold("Câmera Ação"), "camera acao");
    }
}
