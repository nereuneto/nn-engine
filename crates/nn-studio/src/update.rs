//! Aviso de versão nova: o NN Studio consulta a última release do
//! repositório no GitHub (no máximo uma vez por dia, numa thread à parte) e,
//! se ela for mais nova, mostra um aviso com o link para baixar. Ele nunca
//! baixa nem instala nada sozinho.

use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

/// Endereço da última release (API do GitHub).
pub const LATEST_URL: &str = "https://api.github.com/repos/nereuneto/nn-engine/releases/latest";
/// Página das versões, para quando a resposta não traz o link.
pub const RELEASES_PAGE: &str = "https://github.com/nereuneto/nn-engine/releases/latest";
/// De quanto em quanto tempo consultar.
pub const INTERVAL_SECS: u64 = 24 * 60 * 60;

/// Uma versão publicada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// "0.2.0" (sem o "v").
    pub version: String,
    /// Página da release.
    pub url: String,
}

/// "v0.2.1" ou "0.2.1" → (0, 2, 1). `None` se não for uma versão.
pub fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let s = s.trim().trim_start_matches('v');
    // "0.2.0-beta" conta como 0.2.0.
    let core = s.split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    let a = parts.next()??;
    let b = parts.next().unwrap_or(Some(0))?;
    let c = parts.next().unwrap_or(Some(0))?;
    Some((a, b, c))
}

/// `other` é mais nova que `current`?
pub fn is_newer(other: &str, current: &str) -> bool {
    match (parse_version(other), parse_version(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

/// Lê a resposta da API do GitHub.
pub fn parse_release(json: &str) -> Option<Release> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let tag = v.get("tag_name")?.as_str()?;
    parse_version(tag)?;
    // Rascunhos e pré-versões não contam.
    if v.get("draft").and_then(|d| d.as_bool()) == Some(true)
        || v.get("prerelease").and_then(|d| d.as_bool()) == Some(true)
    {
        return None;
    }
    let url = v
        .get("html_url")
        .and_then(|u| u.as_str())
        .unwrap_or(RELEASES_PAGE)
        .to_string();
    Some(Release {
        version: tag.trim_start_matches('v').to_string(),
        url,
    })
}

/// Consulta a última release numa thread. O resultado chega pelo canal
/// (`None`: sem internet, sem release ou resposta estranha).
pub fn check() -> Receiver<Option<Release>> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let _ = tx.send(fetch());
    });
    rx
}

fn fetch() -> Option<Release> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(8)))
        .build()
        .into();
    let mut resp = agent
        .get(LATEST_URL)
        .header(
            "User-Agent",
            &format!("nn-studio/{}", env!("CARGO_PKG_VERSION")),
        )
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| log::info!("não deu para ver se há versão nova: {e}"))
        .ok()?;
    let body = resp.body_mut().read_to_string().ok()?;
    parse_release(&body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compara_versoes() {
        assert_eq!(parse_version("v0.2.1"), Some((0, 2, 1)));
        assert_eq!(parse_version("1.0"), Some((1, 0, 0)));
        assert_eq!(parse_version("0.3.0-beta.1"), Some((0, 3, 0)));
        assert_eq!(parse_version("nada"), None);
        assert!(is_newer("v0.1.1", "0.1.0"));
        assert!(is_newer("0.10.0", "0.9.9"));
        assert!(!is_newer("v0.1.0", "0.1.0"));
        assert!(!is_newer("v0.0.9", "0.1.0"));
        assert!(!is_newer("estranho", "0.1.0"));
    }

    #[test]
    fn le_a_resposta_do_github() {
        let r = parse_release(
            r#"{"tag_name":"v0.2.0","html_url":"https://github.com/x/y/releases/tag/v0.2.0","draft":false,"prerelease":false}"#,
        )
        .unwrap();
        assert_eq!(r.version, "0.2.0");
        assert!(r.url.ends_with("v0.2.0"));
        assert_eq!(
            parse_release(r#"{"tag_name":"v0.3.0","prerelease":true}"#),
            None
        );
        assert_eq!(parse_release(r#"{"message":"Not Found"}"#), None);
        assert_eq!(parse_release("não é json"), None);
    }
}

#[cfg(test)]
mod rede {
    /// Consulta de verdade (precisa de internet):
    /// `cargo test -p nn-studio --lib consulta_o_github -- --ignored`
    #[test]
    #[ignore]
    fn consulta_o_github() {
        let r = super::check().recv().unwrap();
        eprintln!("{r:?}");
    }
}
