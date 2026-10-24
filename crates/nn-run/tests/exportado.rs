//! O jogo exportado de verdade (o `nn-jogo` com o pacote anexado), sem
//! janela: erros de propósito geram o arquivo `<nome>-erro.txt` e a tela de
//! erro para quem joga.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::time::{Duration, Instant};

use nn_run::{EXE_SUFFIX, ExportOptions, export};

const NOME: &str = "jogo-teste";

struct Jogo {
    dir: PathBuf,
    exe: PathBuf,
}

impl Jogo {
    /// Exporta um projeto com este `main.lua`.
    fn exportar(caso: &str, main: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nn-exportado-{}-{caso}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let proj = dir.join("projeto");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(
            proj.join("config.lua"),
            r#"return { title = "Jogo de Teste", width = 400, height = 240 }"#,
        )
        .unwrap();
        std::fs::write(proj.join("main.lua"), main).unwrap();
        let r = export(&ExportOptions {
            project: proj,
            output_dir: dir.join("saida"),
            runtime: Some(PathBuf::from(env!("CARGO_BIN_EXE_nn-jogo"))),
            name: Some(NOME.into()),
        })
        .unwrap();
        assert_eq!(
            r.output,
            dir.join("saida").join(format!("{NOME}{EXE_SUFFIX}"))
        );
        Self { dir, exe: r.output }
    }

    fn arquivo_de_erro(&self) -> PathBuf {
        self.exe.with_file_name(format!("{NOME}-erro.txt"))
    }

    fn tela(&self) -> PathBuf {
        self.dir.join("tela.png")
    }

    /// Roda sem janela por `quadros`, com variáveis a mais. `None`: sem GPU
    /// aqui (o teste é pulado).
    fn rodar(&self, quadros: u32, env: &[(&str, &str)]) -> Option<ExitStatus> {
        let mut cmd = Command::new(&self.exe);
        cmd.env("NN_JOGO_SEM_JANELA", quadros.to_string())
            .env("NN_JOGO_TELA", self.tela());
        for (k, v) in env {
            cmd.env(k, v);
        }
        let mut child = cmd.spawn().unwrap();
        let t = Instant::now();
        let status = loop {
            if let Some(s) = child.try_wait().unwrap() {
                break s;
            }
            if t.elapsed() > Duration::from_secs(90) {
                let _ = child.kill();
                panic!("o jogo não terminou em 90 s");
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        let texto = std::fs::read_to_string(self.arquivo_de_erro()).unwrap_or_default();
        if texto.contains("não pôde ser iniciado") && texto.contains("GPU") {
            if std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1") {
                panic!("GPU obrigatória mas indisponível:\n{texto}");
            }
            eprintln!("pulando teste: sem GPU");
            return None;
        }
        Some(status)
    }

    fn erro(&self) -> String {
        std::fs::read_to_string(self.arquivo_de_erro())
            .unwrap_or_else(|e| panic!("sem {}: {e}", self.arquivo_de_erro().display()))
    }

    /// Espera a tela aparecer (a tela de erro, num processo novo, grava depois
    /// que o jogo fechou).
    fn esperar_tela(&self) {
        let t = Instant::now();
        while !self.tela().exists() {
            assert!(
                t.elapsed() < Duration::from_secs(60),
                "a tela de erro não apareceu"
            );
            std::thread::sleep(Duration::from_millis(100));
        }
        // Terminando de gravar.
        std::thread::sleep(Duration::from_millis(500));
    }
}

/// A cor de um pixel da tela gravada.
fn pixel(png: &Path, x: u32, y: u32) -> [u8; 3] {
    let img = image::open(png).unwrap().to_rgba8();
    let p = img.get_pixel(x, y);
    [p[0], p[1], p[2]]
}

fn perto(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 3)
}

/// O fundo e a faixa de cima da tela de erro para quem joga.
const FUNDO: [u8; 3] = [0x14, 0x1A, 0x26];
const FAIXA: [u8; 3] = [0xE5, 0xA8, 0x4D];

fn confere_tela_de_erro(png: &Path) {
    assert!(
        perto(pixel(png, 2, 1), FAIXA),
        "faixa: {:?}",
        pixel(png, 2, 1)
    );
    assert!(
        perto(pixel(png, 2, 200), FUNDO),
        "fundo: {:?}",
        pixel(png, 2, 200)
    );
}

#[test]
fn erro_de_script_gera_o_arquivo_e_a_tela_para_quem_joga() {
    let jogo = Jogo::exportar(
        "script",
        r#"local cena = {}
function cena:update(dt)
  error("bum de propósito")
end
return cena
"#,
    );
    let Some(status) = jogo.rodar(10, &[]) else {
        return;
    };
    // O jogo segue aberto, com a tela de erro (Esc fecha).
    assert!(status.success(), "{status:?}");
    let texto = jogo.erro();
    assert!(
        texto.starts_with("O jogo \"Jogo de Teste\" encontrou um problema."),
        "{texto}"
    );
    assert!(texto.contains("main.lua:3"), "{texto}");
    assert!(texto.contains("bum de propósito"), "{texto}");
    confere_tela_de_erro(&jogo.tela());
}

#[test]
fn laco_que_chama_a_engine_e_cortado_com_a_linha() {
    let jogo = Jogo::exportar(
        "corte",
        r#"local cena = {}
function cena:update(dt)
  while true do nn.input.down("left") end
end
return cena
"#,
    );
    let Some(status) = jogo.rodar(5, &[]) else {
        return;
    };
    assert!(status.success(), "{status:?}");
    let texto = jogo.erro();
    assert!(texto.contains("main.lua:3: o jogo travou"), "{texto}");
    confere_tela_de_erro(&jogo.tela());
}

#[test]
fn laco_so_de_lua_fecha_o_jogo_e_abre_a_tela() {
    let jogo = Jogo::exportar(
        "laco",
        r#"local cena = {}
function cena:update(dt)
  while true do end
end
return cena
"#,
    );
    let Some(status) = jogo.rodar(5, &[]) else {
        return;
    };
    assert_eq!(status.code(), Some(3), "{status:?}");
    let texto = jogo.erro();
    assert!(texto.contains("O jogo travou e foi fechado"), "{texto}");
    assert!(texto.contains("main.lua"), "{texto}");
    jogo.esperar_tela();
    confere_tela_de_erro(&jogo.tela());
}

#[test]
fn panico_gera_o_arquivo_e_abre_a_tela() {
    let jogo = Jogo::exportar("panico", "return {}");
    let Some(status) = jogo.rodar(5, &[("NN_JOGO_TESTE_PANICO", "1")]) else {
        return;
    };
    assert!(!status.success(), "{status:?}");
    let texto = jogo.erro();
    assert!(texto.contains("erro interno"), "{texto}");
    assert!(texto.contains("pânico de teste"), "{texto}");
    jogo.esperar_tela();
    confere_tela_de_erro(&jogo.tela());
}
