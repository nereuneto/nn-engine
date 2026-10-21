//! Conferências do repositório que pegam erros de junção e de lançamento:
//! um crate que sumiu das dependências do workspace, versões diferentes
//! entre o workspace e os crates, e o CHANGELOG sem a seção da versão.

use std::path::Path;

fn raiz() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// A versão do workspace (`[workspace.package]`).
fn versao(cargo: &str) -> String {
    let i = cargo
        .find("[workspace.package]")
        .expect("[workspace.package]");
    cargo[i..]
        .lines()
        .find_map(|l| l.strip_prefix("version = "))
        .expect("version")
        .trim_matches('"')
        .to_string()
}

#[test]
fn todo_crate_esta_nas_dependencias_do_workspace_com_a_mesma_versao() {
    let cargo = std::fs::read_to_string(raiz().join("Cargo.toml")).unwrap();
    let v = versao(&cargo);
    let deps = &cargo[cargo.find("[workspace.dependencies]").unwrap()..];
    for e in std::fs::read_dir(raiz().join("crates")).unwrap().flatten() {
        let nome = e.file_name().to_string_lossy().to_string();
        if !e.path().join("Cargo.toml").is_file() || nome == "nn-studio" {
            continue;
        }
        let linha = deps
            .lines()
            .find(|l| l.starts_with(&format!("{nome} = ")))
            .unwrap_or_else(|| panic!("{nome} falta em [workspace.dependencies] do Cargo.toml"));
        assert!(
            linha.contains(&format!("path = \"crates/{nome}\""))
                && linha.contains(&format!("version = \"{v}\"")),
            "{nome}: {linha} (a versão do workspace é {v})"
        );
    }
}

#[test]
fn changelog_tem_a_versao_atual_ou_nao_lancado_em_cima() {
    let cargo = std::fs::read_to_string(raiz().join("Cargo.toml")).unwrap();
    let v = versao(&cargo);
    let log = std::fs::read_to_string(raiz().join("CHANGELOG.md")).unwrap();
    let secoes: Vec<&str> = log.lines().filter(|l| l.starts_with("## ")).collect();
    assert!(
        secoes.contains(&format!("## {v}").as_str()),
        "o CHANGELOG não tem a seção ## {v}"
    );
    // Só pode haver "Não lançado" em cima da versão atual, nada entre elas.
    let i = secoes.iter().position(|s| *s == format!("## {v}")).unwrap();
    assert!(
        i == 0 || (i == 1 && secoes[0] == "## Não lançado"),
        "seções antes da {v}: {:?}",
        &secoes[..i]
    );
}
