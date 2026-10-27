//! Interface das cenas visuais (menus e HUD), rodando a engine sem janela:
//! botões com teclado e mouse, deslizador, caixa de marcar, campo de texto,
//! âncoras na tela, grupos e cenas abertas por cima.

use std::path::{Path, PathBuf};

use glam::Vec2;
use nn_app::{AppConfig, Context, Game, Headless, Scene, SceneStack};
use nn_input::{KeyCode, MouseButton};
use nn_script::{LuaScene, ScriptRuntime};

const W: f32 = 320.0;
const H: f32 = 240.0;

fn projeto(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nn-interface-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (path, content) in files {
        let full = dir.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, content).unwrap();
    }
    dir
}

fn config(dir: &Path) -> AppConfig {
    AppConfig::default()
        .size(W as u32, H as u32)
        .assets_dir(dir)
        .save_dir(dir.join("dados"))
        .hot_reload(false)
}

struct Uma(LuaScene);
impl Game for Uma {
    fn fixed_update(&mut self, ctx: &mut Context) {
        self.0.fixed_update(ctx);
    }
    fn update(&mut self, ctx: &mut Context) {
        let _ = self.0.update(ctx);
    }
    fn render(&mut self, ctx: &mut Context) {
        self.0.render(ctx);
    }
}

fn headless<G: Game>(dir: &Path, create: impl FnOnce(&mut Context) -> G) -> Option<Headless<G>> {
    match Headless::new(config(dir), create) {
        Ok(h) => Some(h),
        Err(e) => {
            assert!(
                !std::env::var("NN_REQUIRE_GPU").is_ok_and(|v| v == "1"),
                "GPU obrigatória mas indisponível: {e}"
            );
            eprintln!("pulando teste: {e}");
            None
        }
    }
}

fn cena(dir: &Path, path: &str) -> Option<(Headless<Uma>, ScriptRuntime)> {
    let rt = ScriptRuntime::new(dir).unwrap();
    let rt2 = rt.clone();
    let h = headless(dir, |ctx| {
        let mut s = rt.scene(path);
        s.enter(ctx);
        Uma(s)
    })?;
    rt2.lua()
        .globals()
        .set("cena", h.game.0.table().unwrap())
        .unwrap();
    Some((h, rt2))
}

fn lua<T: mlua::FromLua>(rt: &ScriptRuntime, expr: &str) -> T {
    rt.lua().load(format!("return {expr}")).eval().unwrap()
}

fn apertar(h: &mut Headless<impl Game>, k: KeyCode) {
    h.ctx.input.press_key(k);
    h.run_frames(1);
    h.ctx.input.release_key(k);
    h.run_frames(1);
}

/// O ponto (x, y) do HUD (y para cima, centro da tela no 0, 0) na tela.
fn tela(x: f32, y: f32) -> Vec2 {
    Vec2::new(W / 2.0 + x, H / 2.0 - y)
}

fn mouse(h: &mut Headless<impl Game>, p: Vec2) {
    h.ctx.input.set_cursor(Some(p));
    h.run_frames(1);
}

fn clicar(h: &mut Headless<impl Game>, p: Vec2) {
    mouse(h, p);
    h.ctx.input.press_mouse(MouseButton::Left);
    h.run_frames(1);
    h.ctx.input.release_mouse(MouseButton::Left);
    h.run_frames(1);
}

const CONFIG: &str = r#"return {
  title = "Menu", width = 320, height = 240,
  vars = { apertos = 0, saidas = 0, volume = 0.5, som = false, nome = "", mudou = 0, vida = 5 },
}"#;

fn foco(rt: &ScriptRuntime) -> String {
    lua(rt, "cena._ui.focus and cena._ui.focus.obj.name or ''")
}

#[test]
fn botoes_com_teclado_mouse_e_eventos() {
    let dir = projeto(
        "botoes",
        &[
            ("config.lua", CONFIG),
            (
                "menu.cena",
                r#"return { format = 1, objects = {
                  { id = 1, name = "jogar", x = 0, y = 40,
                    components = { { type = "button", text = "Jogar", w = 160, h = 40, focus = true } },
                    events = { { when = { { cond = "clicar", quem = "@este" } },
                      actions = { { act = "variavel", var = "apertos", op = "somar", valor = 1 } } } } },
                  { id = 2, name = "sair", x = 0, y = -40,
                    components = { { type = "button", text = "Sair", w = 160, h = 40 } },
                    events = { { when = { { cond = "clicar", quem = "@este" } },
                      actions = { { act = "variavel", var = "saidas", op = "somar", valor = 1 } } } } },
                  { id = 3, name = "desligado", x = 120, y = -40,
                    components = { { type = "button", text = "X", w = 40, h = 40, enabled = false } } },
                } }"#,
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "menu.cena") else {
        return;
    };
    h.run_frames(2);
    assert_eq!(foco(&rt), "jogar", "o botão com \"Começa selecionado\"");
    // Setas e WASD andam entre os botões; o desativado fica de fora.
    apertar(&mut h, KeyCode::ArrowDown);
    assert_eq!(foco(&rt), "sair");
    apertar(&mut h, KeyCode::KeyD);
    assert_eq!(foco(&rt), "sair", "o botão desativado não recebe o foco");
    apertar(&mut h, KeyCode::KeyW);
    assert_eq!(foco(&rt), "jogar");
    apertar(&mut h, KeyCode::Enter);
    assert_eq!(lua::<i64>(&rt, "nn.vars.apertos"), 1);
    // O mouse por cima dá o foco; clicar (apertar e soltar) aperta, uma vez.
    mouse(&mut h, tela(0.0, -40.0));
    assert_eq!(foco(&rt), "sair");
    clicar(&mut h, tela(10.0, -45.0));
    h.run_frames(2);
    assert_eq!(lua::<i64>(&rt, "nn.vars.saidas"), 1);
    assert_eq!(lua::<i64>(&rt, "nn.vars.apertos"), 1);
    // Clicar no desativado não faz nada; ativar pelo Lua.
    clicar(&mut h, tela(120.0, -40.0));
    assert_eq!(foco(&rt), "sair");
    rt.lua()
        .load("cena.objects.desligado.button.enabled = true")
        .exec()
        .unwrap();
    mouse(&mut h, tela(118.0, -40.0));
    assert_eq!(foco(&rt), "desligado");
    assert_eq!(h.game.0.error(), None);
}

#[test]
fn deslizador_caixa_de_marcar_e_campo_de_texto() {
    let dir = projeto(
        "controles",
        &[
            ("config.lua", CONFIG),
            (
                "opcoes.cena",
                r#"return { format = 1,
                  events = { { when = { { cond = "mudou", quem = "campo" } },
                    actions = { { act = "variavel", var = "mudou", op = "somar", valor = 1 } } } },
                  objects = {
                  { id = 1, name = "volume", x = 0, y = 70,
                    components = { { type = "slider", var = "volume", w = 200, step = 0.1 } } },
                  { id = 2, name = "som", x = 0, y = 0,
                    components = { { type = "checkbox", text = "Som", var = "som" } } },
                  { id = 3, name = "campo", x = 0, y = -70,
                    components = { { type = "textfield", var = "nome", max = 5 } } },
                } }"#,
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "opcoes.cena") else {
        return;
    };
    h.run_frames(2);
    assert_eq!(foco(&rt), "");
    // A primeira seta escolhe o item de cima; no deslizador, esquerda e
    // direita mudam o valor.
    apertar(&mut h, KeyCode::ArrowDown);
    assert_eq!(foco(&rt), "volume");
    apertar(&mut h, KeyCode::ArrowRight);
    assert!((lua::<f64>(&rt, "nn.vars.volume") - 0.6).abs() < 1e-6);
    // Arrastar com o mouse: no fim da barra, o máximo.
    mouse(&mut h, tela(0.0, 70.0));
    h.ctx.input.press_mouse(MouseButton::Left);
    h.run_frames(1);
    mouse(&mut h, tela(140.0, 70.0));
    h.ctx.input.release_mouse(MouseButton::Left);
    h.run_frames(1);
    assert!((lua::<f64>(&rt, "nn.vars.volume") - 1.0).abs() < 1e-6);
    // Caixa de marcar: Enter liga e desliga.
    apertar(&mut h, KeyCode::ArrowDown);
    assert_eq!(foco(&rt), "som");
    apertar(&mut h, KeyCode::Enter);
    assert!(lua::<bool>(&rt, "nn.vars.som"));
    clicar(&mut h, tela(0.0, 0.0));
    assert!(!lua::<bool>(&rt, "nn.vars.som"), "o clique desmarca");
    // Campo de texto: com o foco, as letras (até W, A, S, D) vão para ele.
    apertar(&mut h, KeyCode::ArrowDown);
    assert_eq!(foco(&rt), "campo");
    h.ctx.input.press_key(KeyCode::KeyW);
    h.ctx.input.type_text("Zé w");
    h.run_frames(1);
    h.ctx.input.release_key(KeyCode::KeyW);
    h.run_frames(1);
    assert_eq!(foco(&rt), "campo", "W é letra, não seta");
    assert_eq!(lua::<String>(&rt, "nn.vars.nome"), "Zé w");
    h.ctx.input.type_text("abc");
    h.run_frames(1);
    assert_eq!(
        lua::<String>(&rt, "nn.vars.nome"),
        "Zé wa",
        "no máximo 5 letras"
    );
    apertar(&mut h, KeyCode::Backspace);
    assert_eq!(lua::<String>(&rt, "nn.vars.nome"), "Zé w");
    assert!(
        lua::<i64>(&rt, "nn.vars.mudou") >= 3,
        "o evento Mudar de valor"
    );
    assert_eq!(h.game.0.error(), None);
}

#[test]
fn volume_pelo_deslizador_fica_guardado() {
    let dir = projeto(
        "volume",
        &[
            ("config.lua", CONFIG),
            (
                "opcoes.cena",
                r#"return { format = 1, script = "ver.lua", objects = {
                  { id = 1, name = "musica", components = { { type = "slider", target = "musica", w = 200, step = 0.25 } } },
                } }"#,
            ),
            (
                "ver.lua",
                "return { update = function() nn.vars.vol = nn.audio.volume('music'); \
                 nn.vars.salvo = (nn.save.get('nn_opcoes') or {}).musica end }",
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "opcoes.cena") else {
        return;
    };
    h.run_frames(1);
    apertar(&mut h, KeyCode::ArrowDown);
    apertar(&mut h, KeyCode::ArrowLeft);
    apertar(&mut h, KeyCode::ArrowLeft);
    h.run_frames(1);
    assert!((lua::<f64>(&rt, "nn.vars.vol") - 0.5).abs() < 1e-6);
    assert!((lua::<f64>(&rt, "nn.vars.salvo") - 0.5).abs() < 1e-6);
}

#[test]
fn ancoras_na_tela_e_grupo_que_arruma_os_filhos() {
    let dir = projeto(
        "ancoras",
        &[
            ("config.lua", CONFIG),
            (
                "hud.cena",
                r#"return { format = 1, objects = {
                  { id = 1, name = "canto", x = 30, y = -20, anchor = "top_left",
                    components = { { type = "panel", w = 40, h = 20 } },
                    events = { { when = { { cond = "clicar", quem = "@este" } },
                      actions = { { act = "variavel", var = "apertos", op = "somar", valor = 1 } } } } },
                  { id = 2, name = "menu", x = 0, y = 0,
                    components = { { type = "layout", arrange = "vertical", spacing = 10 } },
                    children = {
                      { id = 3, name = "a", components = { { type = "button", w = 100, h = 30 } } },
                      { id = 4, name = "b", components = { { type = "button", w = 100, h = 30 } } },
                      { id = 5, name = "c", visible = false, components = { { type = "button", w = 100, h = 30 } } },
                      { id = 6, name = "d", components = { { type = "button", w = 60, h = 50 } } },
                    } },
                  { id = 7, name = "linha", x = 0, y = -90,
                    components = { { type = "layout", arrange = "horizontal", spacing = 4, align = "start" } },
                    children = {
                      { id = 8, name = "e", components = { { type = "panel", w = 20, h = 10 } } },
                      { id = 9, name = "f", components = { { type = "panel", w = 30, h = 20 } } },
                    } },
                } }"#,
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "hud.cena") else {
        return;
    };
    h.run_frames(1);
    // Preso ao canto de cima à esquerda: 30 para a direita, 20 para baixo.
    clicar(&mut h, tela(30.0, -20.0));
    assert_eq!(
        lua::<i64>(&rt, "nn.vars.apertos"),
        0,
        "não está no meio da tela"
    );
    clicar(&mut h, Vec2::new(30.0, 20.0));
    assert_eq!(lua::<i64>(&rt, "nn.vars.apertos"), 1);
    // Coluna: 30 + 10 + 30 + 10 + 50 = 130 de altura, centrada (o
    // escondido não conta).
    let y = |n: &str| lua::<f64>(&rt, &format!("cena.objects.{n}.y"));
    assert_eq!((y("a"), y("b"), y("d")), (50.0, 10.0, -40.0));
    // Linha: 20 + 4 + 30 = 54 de largura; "start" alinha por baixo... no
    // eixo de cruz (altura 20): o menor fica 5 abaixo do centro.
    let x = |n: &str| lua::<f64>(&rt, &format!("cena.objects.{n}.x"));
    assert_eq!((x("e"), x("f")), (-17.0, 12.0));
    assert_eq!((y("e"), y("f")), (-5.0, 0.0));
    assert_eq!(h.game.0.error(), None);
}

#[test]
fn paineis_barras_e_icones_aparecem() {
    let dir = projeto(
        "desenho",
        &[
            ("config.lua", CONFIG),
            (
                "hud.cena",
                r##"return { format = 1, background = "#000000", objects = {
                  { id = 1, name = "painel", x = -100, y = 60,
                    components = { { type = "panel", w = 60, h = 40, color = "#FF0000", radius = 8 } } },
                  { id = 2, name = "vida", x = 40, y = 60,
                    components = { { type = "bar", var = "vida", max = 10, w = 100, h = 20,
                      color = "#00FF00", back_color = "#0000FF", radius = 0 } } },
                  { id = 3, name = "coracoes", x = -60, y = -60,
                    components = { { type = "icons", count = 2, max = 3, size = 20, spacing = 10,
                      align = "left", empty_color = "#FFFFFF" } } },
                } }"##,
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "hud.cena") else {
        return;
    };
    h.run_frames(2);
    let px = |h: &Headless<Uma>, p: Vec2| {
        let img = h.pixels().unwrap();
        let i = ((p.y as usize) * W as usize + p.x as usize) * 4;
        [img[i], img[i + 1], img[i + 2]]
    };
    assert_eq!(px(&h, tela(-100.0, 60.0)), [255, 0, 0], "painel");
    assert_eq!(px(&h, tela(-129.0, 79.0)), [0, 0, 0], "canto arredondado");
    // Vida 5 de 10: metade da barra verde, o resto azul.
    assert_eq!(px(&h, tela(10.0, 60.0)), [0, 255, 0]);
    assert_eq!(px(&h, tela(70.0, 60.0)), [0, 0, 255]);
    rt.lua().load("nn.vars.vida = 10").exec().unwrap();
    h.run_frames(1);
    assert_eq!(px(&h, tela(70.0, 60.0)), [0, 255, 0]);
    // Ícones: 2 cheios (cor do tema) e 1 vazio (branco), da esquerda.
    let cheio = px(&h, tela(-50.0, -60.0));
    assert_ne!(cheio, [0, 0, 0]);
    assert_ne!(cheio, [255, 255, 255]);
    assert_eq!(px(&h, tela(-50.0 + 60.0, -60.0)), [255, 255, 255]);
    assert_eq!(h.game.0.error(), None);
}

#[test]
fn cena_por_cima_pausa_o_jogo_e_fecha() {
    let dir = projeto(
        "pausa",
        &[
            ("config.lua", CONFIG),
            (
                "jogo.lua",
                "return { update = function(self, dt) nn.vars.apertos = nn.vars.apertos + 1 end }",
            ),
            (
                "jogo.cena",
                r#"return { format = 1, script = "jogo.lua",
                  events = { { when = { { cond = "tecla", modo = "apertar", tecla = "voltar" } },
                    actions = { { act = "por_cima", cena = "pausa.cena" } } } },
                  objects = {} }"#,
            ),
            (
                "pausa.cena",
                r##"return { format = 1, background = "#00000080",
                  events = { { when = { { cond = "tecla", modo = "apertar", tecla = "voltar" } },
                    actions = { { act = "fechar_cena" } } } },
                  objects = {
                    { id = 1, name = "continuar", components = {
                      { type = "button", text = "Continuar", action = "fechar", focus = true } } },
                  } }"##,
            ),
        ],
    );
    let rt = ScriptRuntime::new(&dir).unwrap();
    let rt2 = rt.clone();
    let Some(mut h) = headless(&dir, move |ctx| SceneStack::new(ctx, rt.scene("jogo.cena"))) else {
        return;
    };
    h.run_frames(5);
    let antes: i64 = lua(&rt2, "nn.vars.apertos");
    assert!(antes >= 4);
    // Esc: a pausa abre por cima; o jogo de baixo para.
    apertar(&mut h, KeyCode::Escape);
    h.run_frames(5);
    assert_eq!(h.game.len(), 2);
    let parado: i64 = lua(&rt2, "nn.vars.apertos");
    h.run_frames(10);
    assert_eq!(
        lua::<i64>(&rt2, "nn.vars.apertos"),
        parado,
        "o jogo está pausado"
    );
    // Enter no botão "Continuar" fecha a pausa e o jogo continua.
    apertar(&mut h, KeyCode::Enter);
    h.run_frames(5);
    assert_eq!(h.game.len(), 1);
    assert!(lua::<i64>(&rt2, "nn.vars.apertos") > parado);
    // Esc abre e Esc fecha (o evento da própria pausa).
    apertar(&mut h, KeyCode::Escape);
    h.run_frames(3);
    assert_eq!(h.game.len(), 2);
    apertar(&mut h, KeyCode::Escape);
    h.run_frames(3);
    assert_eq!(h.game.len(), 1);
}

fn pixel(h: &Headless<impl Game>, p: Vec2) -> [u8; 4] {
    let px = h.pixels().unwrap();
    let i = ((p.y as u32 * W as u32 + p.x as u32) * 4) as usize;
    [px[i], px[i + 1], px[i + 2], px[i + 3]]
}

/// Um Painel com Rolar e dez botões: vira uma lista que rola.
#[test]
fn painel_que_rola_mostra_so_o_que_cabe() {
    let mut botoes = String::new();
    for i in 1..=10 {
        botoes.push_str(&format!(
            r##"{{ id = {}, name = "b{i}", components = {{ {{ type = "button", text = "B{i}", w = 160, h = 40, color = "#3060C0" }} }},
               events = {{ {{ when = {{ {{ cond = "clicar", quem = "@este" }} }},
                 actions = {{ {{ act = "variavel", var = "apertos", op = "somar", valor = {i} }} }} }} }} }},
"##,
            i + 1
        ));
    }
    let cena_txt = format!(
        r##"return {{ format = 1, background = "#000000", objects = {{
          {{ id = 1, name = "lista", x = 0, y = 0,
            components = {{ {{ type = "panel", w = 200, h = 120, color = "#202020", scroll = true }},
                            {{ type = "layout", arrange = "vertical", spacing = 8 }} }},
            children = {{ {botoes} }} }},
        }} }}"##
    );
    let dir = projeto(
        "rolar",
        &[("config.lua", CONFIG), ("lista.cena", &cena_txt)],
    );
    let Some((mut h, rt)) = cena(&dir, "lista.cena") else {
        return;
    };
    h.run_frames(2);
    assert_eq!(h.game.0.error(), None);
    let y = |rt: &ScriptRuntime, n: &str| {
        lua::<f64>(
            rt,
            &format!("select(2, cena.objects.{n}:world_transform())"),
        )
    };
    // O primeiro encosta no alto do painel (60 - 8 de margem - 20).
    assert_eq!(y(&rt, "b1"), 32.0);
    // Dentro do painel, os botões; fora dele, nada (o b4 estaria em -100).
    assert_eq!(pixel(&h, tela(-60.0, 30.0))[..3], [0x30, 0x60, 0xC0]);
    assert_eq!(pixel(&h, tela(0.0, -100.0))[..3], [0, 0, 0]);
    assert_eq!(
        pixel(&h, tela(0.0, 58.0))[..3],
        [0x20, 0x20, 0x20],
        "a margem de cima"
    );

    // O teclado leva o foco para baixo e a lista rola até ele.
    for _ in 0..6 {
        apertar(&mut h, KeyCode::ArrowDown);
    }
    assert_eq!(foco(&rt), "b6");
    let y6 = y(&rt, "b6");
    assert!(y6 - 20.0 >= -52.5 && y6 + 20.0 <= 52.5, "b6 aparece: {y6}");

    // Escondido não responde ao mouse: em -100 estaria um botão.
    clicar(&mut h, tela(0.0, -100.0));
    assert_eq!(lua::<i64>(&rt, "nn.vars.apertos"), 0);

    // A roda rola 40 por linha.
    let antes = y(&rt, "b1");
    mouse(&mut h, tela(0.0, 0.0));
    h.ctx.input.add_scroll(Vec2::new(0.0, 1.0));
    h.run_frames(1);
    assert_eq!(y(&rt, "b1"), antes - 40.0);

    // Arrastar rola e não aperta o botão.
    let antes = y(&rt, "b1");
    h.ctx.input.press_mouse(MouseButton::Left);
    h.run_frames(1);
    for k in 1..=5 {
        h.ctx.input.set_cursor(Some(tela(0.0, k as f32 * 6.0)));
        h.run_frames(1);
    }
    h.ctx.input.release_mouse(MouseButton::Left);
    h.run_frames(2);
    assert_eq!(y(&rt, "b1"), antes + 30.0, "a lista anda com o mouse");
    assert_eq!(
        lua::<i64>(&rt, "nn.vars.apertos"),
        0,
        "arrastar não é clique"
    );

    // Um clique de verdade aperta o botão embaixo do mouse.
    clicar(&mut h, tela(0.0, 0.0));
    assert!(lua::<i64>(&rt, "nn.vars.apertos") > 0);

    // Sem Rolar, nada muda: os filhos ficam onde o Grupo pôs.
    rt.lua()
        .load("cena.objects.lista.panel.scroll = false")
        .exec()
        .unwrap();
    h.run_frames(1);
    assert_eq!(y(&rt, "b1"), 216.0);
    assert_eq!(h.game.0.error(), None);
}

/// A camada com "ordenar por y": quem está mais embaixo fica na frente.
#[test]
fn camada_ordenada_por_y() {
    let dir = projeto(
        "ordem",
        &[
            ("config.lua", CONFIG),
            (
                "fase.cena",
                r##"return { format = 1, background = "#000000", sort_y = { 1 }, objects = {
                  { id = 1, name = "heroi", x = 0, y = -20, layer = 1,
                    components = { { type = "shape", w = 20, h = 20, color = "#00FF00" } } },
                  { id = 2, name = "arvore", x = 0, y = 0, layer = 1,
                    components = { { type = "shape", w = 40, h = 100, color = "#FF0000" } } },
                  { id = 3, name = "chao", x = 0, y = -20, layer = 0,
                    components = { { type = "shape", w = 300, h = 10, color = "#0000FF" } } },
                } }"##,
            ),
        ],
    );
    let Some((mut h, rt)) = cena(&dir, "fase.cena") else {
        return;
    };
    h.run_frames(2);
    assert_eq!(h.game.0.error(), None);
    // O pé da árvore (-50) está mais embaixo que o do herói (-30): ela na
    // frente.
    assert_eq!(pixel(&h, tela(0.0, -20.0))[..3], [255, 0, 0]);
    // O herói desce (pé em -70): agora ele fica na frente.
    rt.lua().load("cena.objects.heroi.y = -60").exec().unwrap();
    h.run_frames(1);
    assert_eq!(pixel(&h, tela(0.0, -55.0))[..3], [0, 255, 0]);
    // A camada 0 continua embaixo de tudo.
    assert_eq!(pixel(&h, tela(100.0, -20.0))[..3], [0, 0, 255]);
    // Sem ordenar, vale a ordem do arquivo: a árvore vem depois, por cima.
    rt.lua().load("cena.objects.heroi.y = -40").exec().unwrap();
    h.run_frames(1);
    assert_eq!(pixel(&h, tela(0.0, -45.0))[..3], [0, 255, 0], "ordenada");
    rt.lua().load("cena._sort_y = nil").exec().unwrap();
    h.run_frames(1);
    assert_eq!(
        pixel(&h, tela(0.0, -45.0))[..3],
        [255, 0, 0],
        "na ordem do arquivo"
    );
    assert_eq!(h.game.0.error(), None);
}

/// O componente Efeito: no objeto, na camada e na tela.
#[test]
fn componente_efeito_no_objeto_na_camada_e_na_tela() {
    let cena_de = |on: &str| {
        format!(
            r##"return {{ format = 1, background = "#000000", objects = {{
              {{ id = 1, name = "vermelho", x = -60, y = 0, layer = 1,
                components = {{ {{ type = "sprite", image = "v.png", w = 40, h = 40 }},
                                {{ type = "effect", effect = "cinza", on = "{on}" }} }} }},
              {{ id = 2, name = "outro", x = 60, y = 0, layer = 1,
                components = {{ {{ type = "sprite", image = "v.png", w = 40, h = 40 }} }} }},
              {{ id = 3, name = "fundo", x = 0, y = -80, layer = 0,
                components = {{ {{ type = "sprite", image = "v.png", w = 300, h = 20 }} }} }},
            }} }}"##
        )
    };
    let png = nn_script::game_run::encode_png(2, 2, [255, 0, 0, 255].repeat(4)).unwrap();
    let cinza = |p: [u8; 4]| p[0] == p[1] && p[1] == p[2] && p[0] > 0;
    let verm = |p: [u8; 4]| p[..3] == [255, 0, 0];
    // (no objeto, no outro da camada, no fundo de outra camada)
    for (on, esperado) in [
        ("objeto", [true, false, false]),
        ("camada", [true, true, false]),
        ("tela", [true, true, true]),
    ] {
        let dir = projeto(
            &format!("efeito-{on}"),
            &[("config.lua", CONFIG), ("fase.cena", &cena_de(on))],
        );
        std::fs::write(dir.join("v.png"), &png).unwrap();
        let Some((mut h, _rt)) = cena(&dir, "fase.cena") else {
            return;
        };
        h.run_frames(2);
        assert_eq!(h.game.0.error(), None);
        let pontos = [tela(-60.0, 0.0), tela(60.0, 0.0), tela(0.0, -80.0)];
        for (p, cinza_esperado) in pontos.iter().zip(esperado) {
            let px = pixel(&h, *p);
            if cinza_esperado {
                assert!(cinza(px), "{on} em {p}: {px:?}");
            } else {
                assert!(verm(px), "{on} em {p}: {px:?}");
            }
        }
    }
}
