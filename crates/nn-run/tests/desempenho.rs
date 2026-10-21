//! Desempenho das cenas 3D dos modelos (engine sem janela).
//!
//! `cargo test --release -p nn-run --test desempenho -- --ignored --nocapture`
//! mede quanto tempo cada parte de um quadro leva. Os testes que não são
//! ignorados conferem as contagens (o que vai para a GPU), que não dependem
//! da máquina.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use nn_app::{Context, Game, Headless, Scene};
use nn_run::{NewOptions, Template, create_project};
use nn_script::{Project, ScriptRuntime};

#[derive(Default)]
struct Tempos {
    simular: Duration,
    desenhar: Duration,
}

struct Medida {
    cena: nn_script::LuaScene,
    t: Tempos,
    rt: ScriptRuntime,
}

impl Game for Medida {
    fn fixed_update(&mut self, ctx: &mut Context) {
        let t = Instant::now();
        self.cena.fixed_update(ctx);
        self.t.simular += t.elapsed();
    }
    fn update(&mut self, ctx: &mut Context) {
        let t = Instant::now();
        let _ = self.cena.update(ctx);
        self.t.simular += t.elapsed();
    }
    fn render(&mut self, ctx: &mut Context) {
        let t = Instant::now();
        self.cena.render(ctx);
        self.t.desenhar += t.elapsed();
    }
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-desempenho-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn abrir(template: Template, cena: &str, w: u32, h: u32) -> Option<(Headless<Medida>, PathBuf)> {
    abrir_em(template, cena, w, h, false)
}

fn abrir_em(
    template: Template,
    cena: &str,
    w: u32,
    h: u32,
    existe: bool,
) -> Option<(Headless<Medida>, PathBuf)> {
    let dir = std::env::temp_dir().join(format!(
        "nn-desempenho-{}-{}",
        std::process::id(),
        template.name()
    ));
    if !existe {
        let _ = std::fs::remove_dir_all(&dir);
        create_project(&NewOptions {
            dir: dir.clone(),
            title: None,
            template,
        })
        .unwrap();
    }
    let project = Project::open(&dir).unwrap();
    let rt = ScriptRuntime::new(&dir).unwrap();
    let rt2 = rt.clone();
    let config = project
        .config
        .clone()
        .size(w, h)
        .save_dir(dir.join("dados"))
        .hot_reload(false);
    let cena = cena.to_string();
    let h = Headless::new(config, move |ctx| {
        let mut s = rt.scene(cena);
        s.enter(ctx);
        Medida {
            cena: s,
            t: Tempos::default(),
            rt: rt2,
        }
    });
    match h {
        Ok(h) => Some((h, dir)),
        Err(e) => {
            assert!(
                std::env::var("NN_REQUIRE_GPU").is_err(),
                "GPU obrigatória mas indisponível: {e}"
            );
            None
        }
    }
}

fn medir(template: Template, cena: &str) {
    let t0 = Instant::now();
    let Some((mut h, dir)) = abrir(template, cena, 1280, 720) else {
        return;
    };
    h.run_frames(1);
    println!(
        "{:10} abrir a cena: {:.0} ms",
        template.name(),
        t0.elapsed().as_secs_f64() * 1000.0
    );
    h.run_frames(30);
    h.game.t = Tempos::default();
    let n = 120;
    let t = Instant::now();
    h.run_frames(n);
    let total = t.elapsed();
    let ms = |d: Duration| d.as_secs_f64() * 1000.0 / n as f64;
    let s = h.ctx.renderer.last_stats();
    println!(
        "{:10} {:6.2} ms/quadro: simular {:5.2}, montar o desenho {:5.2}, GPU e o resto {:5.2} | {:?} | {:?}",
        template.name(),
        ms(total),
        ms(h.game.t.simular),
        ms(h.game.t.desenhar),
        ms(total - h.game.t.simular - h.game.t.desenhar),
        s,
        h.ctx.renderer.last_stats3(),
    );
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore]
fn tempos_por_quadro() {
    medir(Template::Visual3D, "cenas/ilha.cena");
    medir(Template::Vale3D, "cenas/vale.cena");
    medir(Template::Aventura, "cenas/mapa.cena");
}

/// Uma cena 3D cheia: `arvores` árvores paradas (com Corpo 3D), `moedas`
/// moedas girando (comportamento por objeto) e `bichos` besouros animados.
fn cena_cheia(arvores: usize, moedas: usize, bichos: usize, sombras: bool) -> String {
    let corpo = if std::env::var("NN_SEM_CORPO").is_ok() {
        ""
    } else {
        r#", { type = "body3d", body = "static" }"#
    };
    let mut objs = String::new();
    let mut id = 10;
    let mut add = |s: String| {
        objs.push_str(&s);
        objs.push_str(",\n");
    };
    add(r##"{ id = 1, name = "Chão", scale_x = 200, scale_z = 200, components = { { type = "shape3d", shape = "plane", color = "#7BC86C" }, { type = "body3d", body = "static" } } }"##.into());
    add(r#"{ id = 2, name = "Câmera", y = 12, z = 30, rx = -20, components = { { type = "camera3d", fov = 60 } } }"#.into());
    let lado = |i: usize, n: usize| -> (f64, f64) {
        let k = (n as f64).sqrt().ceil() as usize;
        let (a, b) = (i % k, i / k);
        let s = 180.0 / k as f64;
        (-90.0 + (a as f64 + 0.5) * s, -90.0 + (b as f64 + 0.5) * s)
    };
    for i in 0..arvores {
        let (x, z) = lado(i, arvores);
        id += 1;
        add(format!(
            r#"{{ id = {id}, name = "arvore", x = {x}, z = {z}, components = {{ {{ type = "model", file = "modelos/arvore.glb" }}{corpo} }} }}"#
        ));
    }
    for i in 0..moedas {
        let (x, z) = lado(i, moedas);
        id += 1;
        add(format!(
            r#"{{ id = {id}, name = "moeda", x = {}, y = 0.6, z = {}, components = {{ {{ type = "model", file = "modelos/moeda.glb" }}, {{ type = "behavior", script = "nn:girar3d" }} }} }}"#,
            x + 2.0,
            z + 1.0
        ));
    }
    for i in 0..bichos {
        let (x, z) = lado(i, bichos);
        id += 1;
        add(format!(
            r#"{{ id = {id}, name = "bicho", x = {}, z = {}, components = {{ {{ type = "model", file = "modelos/bicho.glb" }}, {{ type = "anim3d", animation = "andar" }} }} }}"#,
            x + 3.0,
            z + 3.0
        ));
    }
    format!(
        "return {{ format = 1, mode = \"3d\", gravity = -20,\n  environment = {{ shadows = {sombras}, fog = true }},\n  objects = {{\n{objs}  }} }}\n"
    )
}

/// Cria um projeto vazio com a cena cheia e abre a cena.
fn abrir_cheia(
    arvores: usize,
    moedas: usize,
    bichos: usize,
    sombras: bool,
    w: u32,
    h: u32,
) -> Option<(Headless<Medida>, PathBuf)> {
    let dir = temp("vazio3d");
    create_project(&NewOptions {
        dir: dir.clone(),
        title: None,
        template: Template::Vazio3D,
    })
    .unwrap();
    std::fs::write(
        dir.join("cenas/cheia.cena"),
        cena_cheia(arvores, moedas, bichos, sombras),
    )
    .unwrap();
    abrir_em(Template::Vazio3D, "cenas/cheia.cena", w, h, true)
}

fn medir_cheia(nome: &str, arvores: usize, moedas: usize, bichos: usize, sombras: bool) {
    let Some((mut h, dir)) = abrir_cheia(arvores, moedas, bichos, sombras, 1280, 720) else {
        return;
    };
    h.run_frames(10);
    h.game.t = Tempos::default();
    let n = 60;
    let t = Instant::now();
    h.run_frames(n);
    let total = t.elapsed();
    let ms = |d: Duration| d.as_secs_f64() * 1000.0 / n as f64;
    println!(
        "{nome:28} {:7.2} ms/quadro: simular {:6.2}, montar o desenho {:6.2}, GPU e o resto {:6.2} | {:?}",
        ms(total),
        ms(h.game.t.simular),
        ms(h.game.t.desenhar),
        ms(total - h.game.t.simular - h.game.t.desenhar),
        h.ctx.renderer.last_stats3(),
    );
    assert!(h.game.cena.error().is_none(), "{:?}", h.game.cena.error());
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore]
fn tempos_numa_cena_cheia() {
    medir_cheia("vazia", 0, 0, 0, true);
    medir_cheia("1000 árvores", 1000, 0, 0, true);
    medir_cheia("1000 árvores sem sombra", 1000, 0, 0, false);
    medir_cheia("500 moedas girando", 0, 500, 0, true);
    medir_cheia("50 bichos animados", 0, 0, 50, true);
}

/// Uma luta grande na arena da ilha: `n` besouros (com Vida, Golpe e
/// caminhos) atrás do herói, e o besouro-rei acordado.
fn medir_luta(n: usize) {
    // NN_PEQUENO: uma tela pequena, para ver a simulação sem o peso da GPU de software.
    let (w, hh) = if std::env::var_os("NN_PEQUENO").is_some() {
        (160, 90)
    } else {
        (1280, 720)
    };
    let Some((mut h, dir)) = abrir(Template::Visual3D, "cenas/ilha.cena", w, hh) else {
        return;
    };
    h.run_frames(5);
    let lua = h.game.rt.lua().clone();
    lua.globals()
        .set("C", h.game.cena.table().unwrap())
        .unwrap();
    lua.load(format!(
        r#"
        local h = C.objects["Herói"]
        h:move_to(-27, 0.1, 6)
        h:behavior("vida").vidas = 1e9
        h:behavior("vida").atual = 1e9
        -- As Ondas da arena criam os besouros (uma onda só, com todos).
        local w = C.objects["Ondas da arena"]:behavior("ondas")
        w.primeira, w.ondas, w.intervalo, w.raio, w.longe, w.letreiro, w.som = {n}, 1, 0, 7, 2, false, ""
        if {n} > 0 then w:comecar() end
        C.objects["Besouro-rei"]:send("comecar")
        "#
    ))
    .exec()
    .unwrap();
    let inicio = std::sync::Arc::new(std::sync::Mutex::new(Instant::now()));
    {
        let inicio = inicio.clone();
        let hk = mlua::HookTriggers::new().every_nth_instruction(100_000);
        lua.set_global_hook(hk, move |lua, _| {
            if inicio.lock().unwrap().elapsed().as_secs() > 3 {
                eprintln!(
                    "{}",
                    lua.traceback(Some("preso"), 0).unwrap().to_string_lossy()
                );
                std::process::exit(3);
            }
            Ok(mlua::VmState::Continue)
        })
        .unwrap();
    }
    for i in 0..(40 + n as u32) {
        *inicio.lock().unwrap() = Instant::now();
        let t = Instant::now();
        h.run_frames(1);
        if std::env::var_os("NN_DEPURAR").is_some() {
            println!("quadro {i}: {:.1} ms", t.elapsed().as_secs_f64() * 1000.0);
        }
    }
    eprintln!("contando");
    let vivos: usize = lua.load("return #C:find_all('besouro')").eval().unwrap();
    eprintln!("vivos {vivos}");
    assert_eq!(vivos, n);
    lua.remove_global_hook();
    // NN_PERFIL=1: onde o Lua gasta o tempo (amostras a cada 1000 instruções,
    // o tempo desde a anterior vai para cada função na pilha).
    let perfil = std::sync::Arc::new(std::sync::Mutex::new((
        Instant::now(),
        std::collections::HashMap::<String, f64>::new(),
    )));
    if std::env::var_os("NN_PERFIL").is_some() {
        let perfil = perfil.clone();
        let hk = mlua::HookTriggers::new().every_nth_instruction(1000);
        lua.set_global_hook(hk, move |lua, _| {
            let mut p = perfil.lock().unwrap();
            let mut dt = p.0.elapsed().as_secs_f64();
            if dt > 0.002 {
                dt = 0.0; // entre um quadro e outro (a GPU)
            }
            let mut vistas = std::collections::HashSet::new();
            for nivel in 0..30 {
                let Some(nome) = lua.inspect_stack(nivel, |d| {
                    let s = d.source();
                    format!(
                        "{}:{}{}",
                        s.short_src.as_deref().unwrap_or("?"),
                        s.line_defined.unwrap_or(0),
                        if nivel == 0 {
                            format!(" (linha {})", d.current_line().unwrap_or(0))
                        } else {
                            String::new()
                        }
                    )
                }) else {
                    break;
                };
                if vistas.insert(nome.clone()) {
                    *p.1.entry(nome).or_default() += dt;
                }
            }
            p.0 = Instant::now();
            Ok(mlua::VmState::Continue)
        })
        .unwrap();
    }
    h.game.t = Tempos::default();
    let quadros: u32 = std::env::var("NN_QUADROS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(120);
    let t = Instant::now();
    h.run_frames(quadros);
    let total = t.elapsed();
    let ms = |d: Duration| d.as_secs_f64() * 1000.0 / quadros as f64;
    println!(
        "luta com {n:3} besouros {:7.2} ms/quadro: simular {:6.2}, montar o desenho {:6.2}, GPU e o resto {:6.2}",
        ms(total),
        ms(h.game.t.simular),
        ms(h.game.t.desenhar),
        ms(total - h.game.t.simular - h.game.t.desenhar),
    );
    lua.remove_global_hook();
    let mut lista: Vec<_> = perfil.lock().unwrap().1.clone().into_iter().collect();
    lista.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (nome, t) in lista.iter().take(40) {
        println!("  {:8.2} ms/quadro  {nome}", t * 1000.0 / quadros as f64);
    }
    assert!(h.game.cena.error().is_none(), "{:?}", h.game.cena.error());
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
#[ignore]
fn tempos_de_uma_luta() {
    if let Some(n) = std::env::var("NN_LUTA").ok().and_then(|v| v.parse().ok()) {
        medir_luta(n);
        return;
    }
    medir_luta(0);
    medir_luta(10);
    medir_luta(30);
}

#[test]
fn numa_cena_cheia_so_vai_para_a_gpu_o_que_a_camera_e_o_sol_veem() {
    let Some((mut h, dir)) = abrir_cheia(400, 0, 0, true, 320, 180) else {
        return;
    };
    h.run_frames(3);
    assert!(h.game.cena.error().is_none(), "{:?}", h.game.cena.error());
    let s = h.ctx.renderer.last_stats3();
    println!("cena cheia: {s:?}");
    // As 400 árvores (e o chão) são pedidas, mas a câmera só vê uma parte.
    assert!(s.meshes > 400, "{s:?}");
    assert!(s.visible > 0 && s.visible < s.meshes * 3 / 4, "{s:?}");
    // A sombra só precisa do que fica perto da câmera (o quadro do sol).
    assert!(s.shadow > 0 && s.shadow < s.meshes, "{s:?}");
    // A mesma árvore vira poucas chamadas (instâncias), não uma por árvore.
    assert!(s.draw_calls < 20, "{s:?}");
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn as_cenas_3d_dos_modelos_rodam_e_cortam_o_que_nao_aparece() {
    for (template, cena) in [
        (Template::Visual3D, "cenas/ilha.cena"),
        (Template::Vale3D, "cenas/vale.cena"),
    ] {
        let Some((mut h, dir)) = abrir(template, cena, 320, 180) else {
            return;
        };
        h.run_frames(20);
        assert!(
            h.game.cena.error().is_none(),
            "{cena}: {:?}",
            h.game.cena.error()
        );
        let s = h.ctx.renderer.last_stats3();
        println!("{cena}: {s:?}");
        assert!(s.visible > 0 && s.visible <= s.meshes, "{cena}: {s:?}");
        assert!(s.shadow <= s.meshes, "{cena}: {s:?}");
        drop(h);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// Ferramenta: quanto as luzes pesam, na caverna do Vale 3D (tochas,
/// cristais, lampião e a fresta): com todas acesas, sem o halo do brilho,
/// apagadas e com 32 a mais espalhadas (`cargo test --release -p nn-run --test desempenho
/// tempos_da_caverna -- --ignored --nocapture`).
#[test]
#[ignore]
fn tempos_da_caverna() {
    let Some((mut h, dir)) = abrir(Template::Vale3D, "cenas/caverna.cena", 1280, 720) else {
        return;
    };
    h.run_frames(10);
    let lua = h.game.rt.lua().clone();
    lua.globals()
        .set("C", h.game.cena.table().unwrap())
        .unwrap();
    let mut medir = |nome: &str, codigo: &str| {
        lua.load(codigo).exec().unwrap();
        h.run_frames(5);
        h.game.t = Tempos::default();
        let quadros = 60;
        let t = Instant::now();
        h.run_frames(quadros);
        let total = t.elapsed().as_secs_f64() * 1000.0 / quadros as f64;
        let s = h.ctx.renderer.last_stats3();
        println!(
            "{nome:28} {total:7.2} ms/quadro (lâmpadas na GPU: {}, malhas vistas: {})",
            s.lamps, s.visible
        );
    };
    medir("caverna, luzes acesas", "");
    medir(
        "caverna, sem o halo",
        "C._data.environment.bloom = 0; C._env_dirty = true",
    );
    medir(
        "caverna, luzes apagadas",
        "C._data.environment.bloom = 1; C._env_dirty = true
         for _, o in ipairs(C.object_list) do if o.light3d then o.light3d.on = false end end",
    );
    medir(
        "caverna, 32 lâmpadas a mais",
        "local n = 0
         for _, o in ipairs(C.object_list) do
           if o.light3d then
             o.light3d.on = true
           elseif n < 32 and (o.anchor or '') == '' and o.name ~= 'Herói' then
             n = n + 1
             local c = { type = 'light3d', kind = 'point', color = '#FFC080', intensity = 1,
               range = 5, angle = 30, soft = 0.3, flicker = 0.3, on = true }
             table.insert(o.components, c)
             o.light3d = c
           end
         end",
    );
    assert!(h.game.cena.error().is_none(), "{:?}", h.game.cena.error());
    drop(h);
    let _ = std::fs::remove_dir_all(&dir);
}
