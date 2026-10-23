//! Quanto a cena visual custa por quadro, em números que não dependem da
//! máquina: quantas vezes o Lua chama o Rust e quanta memória ele aloca.
//! (O tempo varia muito entre máquinas e no CI; estes não.)

use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

use mlua::{HookTriggers, VmState};
use nn_script::game_run::{GameRun, RunOptions, encode_png};

const IMAGENS: usize = 200;
const FORMAS: usize = 200;
const TEXTOS: usize = 50;
const CORPOS: usize = 50;
const OBJETOS: usize = IMAGENS + FORMAS + TEXTOS + CORPOS;

fn projeto() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-desempenho-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("cenas")).unwrap();
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    std::fs::write(
        dir.join("config.lua"),
        r#"return { title = "Desempenho", width = 640, height = 360, start = "cenas/fase.cena" }"#,
    )
    .unwrap();
    let png = encode_png(8, 8, vec![255; 8 * 8 * 4]).unwrap();
    std::fs::write(dir.join("assets/bloco.png"), png).unwrap();

    let mut objs = String::new();
    let mut id = 0;
    let mut obj = |x: usize, y: usize, comps: &str| {
        id += 1;
        objs.push_str(&format!(
            "    {{ id = {id}, name = \"o{id}\", x = {}, y = {}, components = {{ {comps} }} }},\n",
            x as i32 * 12 - 300,
            150 - y as i32 * 20
        ));
    };
    for i in 0..IMAGENS {
        obj(
            i % 50,
            i / 50,
            r#"{ type = "sprite", image = "assets/bloco.png" }"#,
        );
    }
    for i in 0..FORMAS {
        obj(
            i % 50,
            4 + i / 50,
            r##"{ type = "shape", w = 8, h = 8, color = "#3FA7D6" },
        { type = "behavior", script = "nn:movimento", tipo = "flutuar", altura = 10, duracao = 2 }"##,
        );
    }
    for i in 0..TEXTOS {
        obj(
            i % 10,
            8 + i / 10,
            r#"{ type = "text", text = "pontos", size = 12 }"#,
        );
    }
    for i in 0..CORPOS {
        obj(
            i % 50,
            14,
            r##"{ type = "shape", w = 8, h = 8, color = "#FFAA00" },
        { type = "body", body = "kinematic", shape = "box", w = 8, h = 8 }"##,
        );
    }
    std::fs::write(
        dir.join("cenas/fase.cena"),
        format!("return {{\n  format = 1,\n  objects = {{\n{objs}  }},\n}}\n"),
    )
    .unwrap();
    dir
}

fn abrir() -> Option<GameRun> {
    match GameRun::open(projeto(), RunOptions::default()) {
        Ok(r) => Some(r),
        Err(e) => {
            if std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1") {
                panic!("GPU obrigatória mas indisponível: {e}");
            }
            eprintln!("pulando teste: {e}");
            None
        }
    }
}

#[test]
fn a_cena_visual_chama_o_rust_uma_vez_por_coisa_desenhada() {
    let Some(mut run) = abrir() else {
        return;
    };
    run.run_frames(5);
    assert!(!run.has_error(), "{:?}", run.first_error());
    let lua = run.runtime().lua().clone();
    // As funções da biblioteca do Lua (type, math.abs, o iterador do
    // ipairs...) também são "C" para o Lua, mas não vão ao Rust da engine.
    let padrao: std::collections::HashSet<usize> = lua
        .load(
            r#"
            local out = {}
            local function add(f) if type(f) == "function" then out[#out + 1] = f end end
            for _, v in pairs(_G) do add(v) end
            for _, lib in ipairs({ math, string, table, utf8, coroutine, os }) do
              for _, v in pairs(lib) do add(v) end
            end
            add((ipairs({})))
            return out
            "#,
        )
        .eval::<Vec<mlua::Function>>()
        .unwrap()
        .iter()
        .map(|f| f.to_pointer() as usize)
        .collect();
    // Conta as funções em Rust chamadas pelo Lua.
    let calls = Rc::new(Cell::new(0u64));
    let c = calls.clone();
    lua.set_global_hook(HookTriggers::new().on_calls(), move |_, debug| {
        if debug.source().what == "C" && !padrao.contains(&(debug.function().to_pointer() as usize))
        {
            c.set(c.get() + 1);
            // Com NN_NOMES=1, lista as chamadas (para achar qual passou do limite).
            if std::env::var_os("NN_NOMES").is_some() {
                eprintln!("CHAMADA {:?}", debug.names().name);
            }
        }
        Ok(VmState::Continue)
    })
    .unwrap();
    const QUADROS: u64 = 10;
    run.run_frames(QUADROS);
    lua.remove_global_hook();
    assert!(!run.has_error(), "{:?}", run.first_error());
    let por_quadro = calls.get() / QUADROS;
    // Uma por imagem, forma ou texto desenhado, mais um tanto fixo por
    // quadro (na v0.8 eram umas 4 por objeto: tamanho da imagem, tamanho da
    // janela, e cada corpo lido campo a campo).
    let limite = OBJETOS as u64 + 150;
    eprintln!("{por_quadro} chamadas por quadro");
    assert!(
        por_quadro <= limite,
        "{por_quadro} chamadas ao Rust por quadro (o limite é {limite})"
    );
}

#[test]
fn a_cena_visual_quase_nao_aloca_memoria_a_cada_quadro() {
    let Some(mut run) = abrir() else {
        return;
    };
    run.run_frames(5);
    assert!(!run.has_error(), "{:?}", run.first_error());
    let lua = run.runtime().lua().clone();
    lua.gc_stop();
    let antes = lua.used_memory();
    const QUADROS: usize = 10;
    run.run_frames(QUADROS as u64);
    let depois = lua.used_memory();
    lua.gc_restart();
    assert!(!run.has_error(), "{:?}", run.first_error());
    let por_quadro = depois.saturating_sub(antes) / QUADROS;
    // Na v0.8, cada imagem criava uma tabela de opções e cada objeto uma
    // função: dezenas de KB por quadro com estes 500 objetos.
    let limite = 8 * 1024;
    eprintln!("{por_quadro} bytes por quadro");
    assert!(
        por_quadro <= limite,
        "{por_quadro} bytes alocados pelo Lua por quadro (o limite é {limite})"
    );
}
