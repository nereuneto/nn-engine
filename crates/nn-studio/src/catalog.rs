//! O que dá para criar numa cena, num lugar só: os itens do ➕ Objeto, do
//! menu do botão direito na cena e na Hierarquia, e da busca rápida
//! (Shift+A). Os menus são desenhados a partir desta lista, e a busca
//! procura nela (e nos prefabs e modelos do projeto).

use eframe::egui::{self, Key, Modifiers, Pos2, RichText};

use crate::commands::{fold, fuzzy_score};
use crate::scene_data::{Component, Object, Prop};

/// O que um item cria.
pub enum Make {
    Object(Box<dyn Fn() -> Object>),
    /// Abre a janela de mapa de tiles.
    NewMap,
    /// Um arquivo do projeto (prefab ou modelo), como se fosse solto na cena.
    File(String),
}

impl std::fmt::Debug for Make {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Make::Object(_) => f.write_str("Object"),
            Make::NewMap => f.write_str("NewMap"),
            Make::File(rel) => f.debug_tuple("File").field(rel).finish(),
        }
    }
}

/// Uma opção para criar.
#[derive(Debug)]
pub struct Item {
    pub label: String,
    pub tip: String,
    /// Outras palavras que acham o item na busca ("bloco" acha o Cubo).
    pub words: &'static str,
    /// Marca para o tutorial (`cena.objeto:...`).
    pub anchor: Option<String>,
    pub make: Make,
}

impl Item {
    fn new(label: &str, make: impl Fn() -> Object + 'static) -> Self {
        Self {
            label: label.into(),
            tip: String::new(),
            words: "",
            anchor: None,
            make: Make::Object(Box::new(make)),
        }
    }

    fn tip(mut self, tip: &str) -> Self {
        self.tip = tip.into();
        self
    }

    fn words(mut self, words: &'static str) -> Self {
        self.words = words;
        self
    }

    fn anchor(mut self, anchor: impl Into<String>) -> Self {
        self.anchor = Some(anchor.into());
        self
    }

    pub fn pick(&self) -> Picked {
        match &self.make {
            Make::Object(f) => Picked::Object(f()),
            Make::NewMap => Picked::NewMap,
            Make::File(rel) => Picked::File(rel.clone()),
        }
    }
}

/// Uma linha do menu.
#[derive(Debug)]
pub enum Entry {
    Item(Item),
    Sub {
        label: &'static str,
        anchor: Option<&'static str>,
        entries: Vec<Entry>,
    },
    Sep,
}

/// O que foi escolhido. (Dura um quadro: o objeto não precisa de caixa.)
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum Picked {
    Object(Object),
    NewMap,
    File(String),
}

fn item(label: &str, make: impl Fn() -> Object + 'static) -> Item {
    Item::new(label, make)
}

fn sub(label: &'static str, anchor: Option<&'static str>, entries: Vec<Entry>) -> Entry {
    Entry::Sub {
        label,
        anchor,
        entries,
    }
}

impl From<Item> for Entry {
    fn from(i: Item) -> Self {
        Entry::Item(i)
    }
}

fn with(name: &str, kind: &str) -> Object {
    let mut o = Object::new(0, name);
    o.components.push(Component::new(kind));
    o
}

fn shape3(name: &str, shape: &str, color: &str) -> Object {
    let mut o = Object::new(0, name);
    let mut c = Component::new("shape3d");
    c.set("shape", Prop::Str(shape.into()));
    c.set("color", Prop::Str(color.into()));
    o.components.push(c);
    o
}

/// Um comportamento pronto (`nn:...`) com os valores de começo, trocando alguns.
fn behavior(script: &str, three_d: bool, set: &[(&str, Prop)]) -> Component {
    let mut c = match crate::behaviors::builtins()
        .iter()
        .find(|b| b.script == script)
    {
        Some(info) => Component::behavior(script, &info.defaults_for(three_d)),
        None => Component::behavior(script, &[]),
    };
    for (k, v) in set {
        c.set(k, v.clone());
    }
    c
}

/// Os elementos de menu e HUD (submenu Interface no 2D, Na tela no 3D).
fn interface() -> Vec<Entry> {
    let items: [(&str, &str, &str, &str, &'static str); 8] = [
        (
            "Painel",
            "painel",
            "panel",
            "Fundo de menu ou de HUD",
            "fundo menu hud",
        ),
        (
            "Botão",
            "botao",
            "button",
            "Aperta com mouse, teclado ou controle",
            "menu clicar",
        ),
        (
            "Barra",
            "barra",
            "bar",
            "Vida, energia, tempo",
            "vida energia hp tempo",
        ),
        (
            "Ícones",
            "icones",
            "icons",
            "Corações, vidas, moedas",
            "coracoes vidas moedas",
        ),
        (
            "Deslizador",
            "deslizador",
            "slider",
            "Volume, dificuldade",
            "volume slider",
        ),
        (
            "Caixa de marcar",
            "caixa",
            "checkbox",
            "Liga e desliga",
            "checkbox marcar",
        ),
        (
            "Campo de texto",
            "campo",
            "textfield",
            "Para digitar um nome",
            "digitar nome",
        ),
        (
            "Grupo (arrumar filhos)",
            "grupo",
            "layout",
            "Põe os filhos em coluna, linha ou grade",
            "coluna linha grade",
        ),
    ];
    let mut out: Vec<Entry> = items
        .into_iter()
        .map(|(label, name, kind, tip, words)| {
            item(label, move || with(name, kind))
                .tip(tip)
                .words(words)
                .anchor(format!("cena.objeto:{kind}"))
                .into()
        })
        .collect();
    out.push(Entry::Sep);
    out.push(
        item("Menu com botões", crate::scene_view::menu_with_buttons)
            .tip("Um painel com um grupo de 3 botões: Jogar, Opções e Sair")
            .words("tela inicial jogar opcoes sair")
            .anchor("cena.objeto:menu")
            .into(),
    );
    out
}

/// O ➕ Objeto das cenas 2D.
pub fn catalog2() -> Vec<Entry> {
    let mut out = catalog2_objects();
    out.push(Entry::Item(Item {
        label: "Mapa de tiles...".into(),
        tip: "Um mapa para pintar com tiles: chão, paredes, plataformas".into(),
        words: "tilemap chao paredes plataformas fase",
        anchor: Some("cena.objeto:mapa".into()),
        make: Make::NewMap,
    }));
    out
}

/// O ➕ Objeto das cenas 2D, só o que vira objeto na hora (sem o mapa, que
/// abre uma janela).
pub fn catalog2_objects() -> Vec<Entry> {
    let mut out: Vec<Entry> = vec![
        item("Vazio", || Object::new(0, "objeto"))
            .words("grupo pasta")
            .into(),
        item("Imagem (sprite)", || with("sprite", "sprite"))
            .words("figura desenho personagem")
            .into(),
        item("Retângulo", || with("retangulo", "shape"))
            .words("quadrado caixa bloco forma")
            .into(),
        item("Círculo", || {
            let mut o = with("circulo", "shape");
            if let Some(c) = o.components.first_mut() {
                c.set("shape", Prop::Str("circle".into()));
            }
            o
        })
        .words("bola redondo forma")
        .into(),
        item("Texto", || with("texto", "text"))
            .words("placar letra escrita")
            .into(),
        item("Câmera", || with("camera", "camera")).into(),
        item("Som", || with("som", "sound"))
            .words("musica audio efeito")
            .into(),
    ];
    let mut fx: Vec<Entry> = vec![
        item("Vazias", || with("particulas", "particles"))
            .tip("Um emissor para ajustar do zero no Inspetor")
            .words("particulas emissor")
            .into(),
        Entry::Sep,
    ];
    for p in crate::particle_presets::PRESETS {
        fx.push(
            item(p.label, move || {
                let mut o = Object::new(0, p.id);
                o.components.push(p.component());
                o
            })
            .tip(p.tip)
            .words("particulas efeito")
            .anchor(format!("cena.objeto:fx:{}", p.id))
            .into(),
        );
    }
    out.push(sub("Partículas", Some("cena.objeto:particulas"), fx));
    out.push(sub("Interface", Some("cena.objeto:interface"), interface()));
    out
}

/// O ➕ Objeto das cenas 3D.
pub fn catalog3() -> Vec<Entry> {
    let mut out: Vec<Entry> = Vec::new();
    for (label, s, color, words) in [
        ("Cubo", "cube", "#FF8C42", "caixa bloco forma"),
        ("Esfera", "sphere", "#3FA7D6", "bola forma"),
        ("Cilindro", "cylinder", "#C77DFF", "tubo coluna forma"),
        ("Cone", "cone", "#FFD166", "forma"),
        ("Cápsula", "capsule", "#06D6A0", "forma pilula"),
    ] {
        out.push(
            item(label, move || shape3(label, s, color))
                .words(words)
                .into(),
        );
    }
    out.push(
        item("Plano (chão)", || {
            let mut o = shape3("Plano", "plane", "#7BC86C");
            o.scale_x = 10.0;
            o.scale_z = 10.0;
            // Um chão: já vem com um Corpo 3D parado.
            o.components.push(Component::new("body3d"));
            o
        })
        .words("chao piso forma")
        .into(),
    );
    out.push(
        item("Água (lago)", crate::scene_view::scene3d::water)
            .tip("Um plano azul com o Material Água: ondinhas, o céu refletido e o brilho do sol. Ponha um pouco acima do chão, ou num buraco.")
            .words("agua lago rio mar")
            .anchor("cena.objeto:agua")
            .into(),
    );
    out.push(Entry::Sep);
    out.push(
        item("Modelo (.glb)", || with("Modelo", "model"))
            .tip("Um objeto com um modelo 3D: escolha o arquivo no Inspetor, ou arraste um .glb do painel Arquivos para a cena.")
            .words("glb gltf malha")
            .into(),
    );
    out.push(
        item("Câmera 3D", || with("Câmera", "camera3d"))
            .tip("A câmera do jogo: ele mostra o que ela vê.")
            .into(),
    );
    out.push(
        item("Vazio", || Object::new(0, "Grupo"))
            .tip("Um objeto sem nada, para agrupar outros")
            .words("grupo pasta")
            .into(),
    );
    out.push(sub("Luz", Some("cena.objeto:luz"), lights()));
    out.push(Entry::Sep);
    out.push(
        item("Terreno", || with("Terreno", "terrain"))
            .tip("Um chão de 64 × 64 m para pintar morros, vales, cores e água (✏ Pintar terreno)")
            .words("chao morro montanha ilha")
            .into(),
    );
    let mut fx: Vec<Entry> = vec![
        item("Vazias", || {
            let mut o = Object::new(0, "particulas");
            o.components.push(crate::particle_presets::component_3d());
            o
        })
        .tip("Um emissor para ajustar do zero no Inspetor")
        .words("particulas emissor")
        .into(),
        Entry::Sep,
    ];
    for p in crate::particle_presets::PRESETS_3D {
        fx.push(
            item(p.label, move || {
                let mut o = Object::new(0, p.id);
                o.components.push(p.component());
                o
            })
            .tip(p.tip)
            .words("particulas efeito")
            .anchor(format!("cena.objeto:fx:{}", p.id))
            .into(),
        );
    }
    out.push(sub("Partículas", None, fx));
    out.push(sub("Portas e itens", None, doors_and_items()));
    out.push(
        item("Ondas de inimigos", || {
            let mut o = Object::new(0, "Ondas");
            o.components.push(behavior("nn:ondas", true, &[]));
            for (x, z) in [(5.0, 0.0), (-5.0, 0.0), (0.0, 5.0), (0.0, -5.0)] {
                let mut p = Object::new(0, "Saída");
                (p.x, p.z) = (x, z);
                o.children.push(p);
            }
            o
        })
        .tip("Cria inimigos (um prefab) em ondas: onda 1, 2, 3... Os quatro filhos são de onde eles saem: arraste-os pela arena.")
        .words("inimigos monstros arena spawn")
        .anchor("cena.objeto:ondas")
        .into(),
    );
    out.push(Entry::Sep);
    out.push(
        item("Jogador 3D", || {
            let mut o = shape3("Jogador", "capsule", "#FF8C42");
            if let Some(info) = crate::behaviors::builtins()
                .iter()
                .find(|b| b.script == "nn:jogador3d")
            {
                o.components
                    .push(Component::behavior(&info.script, &info.defaults_for(true)));
            }
            o
        })
        .tip("Uma cápsula que anda, pula e leva a câmera atrás dela. Troque a forma por um modelo .glb no Inspetor.")
        .words("personagem heroi player boneco")
        .into(),
    );
    out.push(Entry::Sep);
    let mut hud: Vec<Entry> = vec![
        item("Texto na tela", || {
            let mut o = Object::new(0, "texto");
            let mut c = Component::new("text");
            c.set("hud", Prop::Bool(true));
            o.components.push(c);
            o
        })
        .tip("Um texto preso à tela: placar, dicas, avisos")
        .words("placar pontos hud aviso")
        .into(),
        Entry::Sep,
    ];
    hud.extend(interface());
    out.push(sub("Na tela (HUD)", Some("cena.objeto:hud"), hud));
    out
}

/// O submenu Luz: lâmpada, tocha (com a chama) e holofote.
fn lights() -> Vec<Entry> {
    let light = |set: &[(&str, Prop)]| {
        let mut c = Component::new("light3d");
        for (k, v) in set {
            c.set(k, v.clone());
        }
        c
    };
    vec![
        item("Lâmpada", move || {
            let mut o = Object::new(0, "Lâmpada");
            o.y = 2.0;
            o.components.push(light(&[]));
            o
        })
        .tip("Ilumina para todos os lados, até o Alcance.")
        .words("luz lampada ponto")
        .anchor("cena.objeto:luz:lampada")
        .into(),
        item("Tocha", crate::scene_view::scene3d::torch)
            .tip("Um cabo com a chama em cima: a luz tremula como fogo, com partículas de fogo.")
            .words("luz fogo chama")
            .anchor("cena.objeto:luz:tocha")
            .into(),
        item("Holofote", move || {
            let mut o = Object::new(0, "Holofote");
            o.y = 4.0;
            o.rx = -70.0;
            o.components.push(light(&[
                ("kind", Prop::Str("spot".into())),
                ("color", Prop::Str("#FFF4D6".into())),
                ("intensity", Prop::Num(2.5)),
                ("range", Prop::Num(14.0)),
                ("angle", Prop::Num(25.0)),
            ]));
            o
        })
        .tip("Ilumina num cone, para a frente do objeto (a seta azul). Gire para apontar.")
        .words("luz spot lanterna")
        .anchor("cena.objeto:luz:holofote")
        .into(),
    ]
}

/// O submenu Portas e itens.
fn doors_and_items() -> Vec<Entry> {
    vec![
        item("Porta", || {
            let mut o = shape3("Porta", "cube", "#9C6B43");
            (o.scale_x, o.scale_y, o.scale_z) = (1.2, 2.2, 0.15);
            let mut body = Component::new("body3d");
            body.set("body", Prop::Str("kinematic".into()));
            o.components.push(body);
            o.components.push(behavior("nn:porta3d", true, &[]));
            o
        })
        .tip("Uma porta que abre com E (girando na dobradiça). Ponha um nome de item em \"Precisa do item\" para trancar.")
        .words("abrir trancar")
        .anchor("cena.objeto:porta")
        .into(),
        item("Chave", || {
            let mut o = shape3("Chave", "cube", "#FFD166");
            (o.scale_x, o.scale_y, o.scale_z) = (0.35, 0.35, 0.35);
            o.components.push(behavior(
                "nn:coletavel",
                true,
                &[
                    ("variavel", Prop::Str(String::new())),
                    ("item", Prop::Str("chave".into())),
                ],
            ));
            o.components.push(behavior("nn:girar3d", true, &[]));
            o
        })
        .tip("Um item que vai para o inventário (\"chave\") quando o jogador encosta. Abre portas trancadas.")
        .words("item coletavel pegar")
        .anchor("cena.objeto:chave")
        .into(),
        item("Zona de gatilho", || {
            let mut o = shape3("Zona", "cube", "#3FA7D640");
            (o.scale_x, o.scale_y, o.scale_z) = (3.0, 2.0, 3.0);
            if let Some(c) = o.components.first_mut() {
                c.set("shadow", Prop::Bool(false));
            }
            o.components.push(behavior("nn:gatilho", true, &[]));
            o
        })
        .tip("Uma área invisível no jogo que avisa quando o jogador entra ou sai (abre uma porta, liga uma armadilha...).")
        .words("trigger area armadilha")
        .anchor("cena.objeto:zona")
        .into(),
        item("Texto 3D", || {
            let mut o = Object::new(0, "Texto");
            let mut c = Component::new("text3d");
            c.set("text", Prop::Str("Olá!".into()));
            o.components.push(c);
            o
        })
        .tip("Um texto no mundo: placa, nome, aviso. Sempre virado para a câmera (ou preso como placa).")
        .words("placa letreiro nome")
        .anchor("cena.objeto:texto3d")
        .into(),
        item("Mostrar inventário", || {
            let mut o = Object::new(0, "Inventário");
            o.components.push(behavior("nn:inventario", true, &[]));
            o
        })
        .tip("Os itens que o jogador carrega, num canto da tela.")
        .words("itens mochila")
        .anchor("cena.objeto:inventario")
        .into(),
        item("Personagem que fala", || {
            let mut o = shape3("Personagem", "capsule", "#4CC9F0");
            o.components.push(behavior("nn:dialogo", true, &[]));
            o
        })
        .tip("Alguém para conversar: perto dele aparece \"E falar\" e abre uma conversa (.dialogo) numa caixa de fala.")
        .words("npc conversa dialogo falar")
        .anchor("cena.objeto:personagem")
        .into(),
    ]
}

// Favoritos -----------------------------------------------------------------

/// A pasta dos favoritos, no projeto.
pub const FAVORITES_DIR: &str = "favoritos";
/// A extensão de um favorito (um pedaço de cena com um objeto).
pub const FAVORITE_EXT: &str = "favorito";
const FAVORITE_MARK: &str =
    "-- Favorito do NN Studio: um molde para criar o objeto de novo (⭐ no ➕ Objeto).";

/// Um objeto guardado nos favoritos do projeto.
#[derive(Debug, Clone, PartialEq)]
pub struct Favorite {
    pub name: String,
    pub rel: String,
    pub three_d: bool,
    pub object: Object,
}

type FavCache = Option<(
    std::path::PathBuf,
    Vec<(String, Option<std::time::SystemTime>)>,
    Vec<Favorite>,
)>;
static FAV_CACHE: std::sync::Mutex<FavCache> = std::sync::Mutex::new(None);

/// Os favoritos do projeto (relidos só quando os arquivos mudam).
pub fn favorites(root: &std::path::Path) -> Vec<Favorite> {
    let dir = root.join(FAVORITES_DIR);
    let mut stamp: Vec<(String, Option<std::time::SystemTime>)> = std::fs::read_dir(&dir)
        .map(|list| {
            list.flatten()
                .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some(FAVORITE_EXT))
                .map(|e| {
                    let t = e.metadata().and_then(|m| m.modified()).ok();
                    (e.file_name().to_string_lossy().to_string(), t)
                })
                .collect()
        })
        .unwrap_or_default();
    stamp.sort();
    let mut cache = FAV_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((r, st, favs)) = cache.as_ref()
        && r == root
        && *st == stamp
    {
        return favs.clone();
    }
    let mut favs = Vec::new();
    for (file, _) in &stamp {
        let rel = format!("{FAVORITES_DIR}/{file}");
        let Ok(text) = std::fs::read_to_string(root.join(&rel)) else {
            continue;
        };
        let Ok(mut piece) = crate::scene_data::read(&text, &rel) else {
            continue;
        };
        piece.link_prefabs(root);
        let Some(object) = piece.objects.into_iter().next() else {
            continue;
        };
        let name = std::path::Path::new(file)
            .file_stem()
            .map(|s| s.to_string_lossy().replace(['_', '-'], " "))
            .unwrap_or_default();
        favs.push(Favorite {
            name,
            rel,
            three_d: piece.three_d,
            object,
        });
    }
    *cache = Some((root.to_path_buf(), stamp, favs.clone()));
    favs
}

/// Guarda o objeto (com os filhos) nos favoritos do projeto, com o nome
/// dele. Devolve o caminho do arquivo.
pub fn save_favorite(
    root: &std::path::Path,
    obj: &Object,
    three_d: bool,
) -> Result<String, String> {
    let mut o = obj.clone();
    // Um molde: a posição vem de onde ele for criado.
    (o.x, o.y, o.z) = (0.0, 0.0, 0.0);
    let base = {
        let s = nn_app::slug(&o.name);
        if s.is_empty() {
            "favorito".to_string()
        } else {
            s
        }
    };
    let dir = root.join(FAVORITES_DIR);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut name = base.clone();
    let mut n = 2;
    while dir.join(format!("{name}.{FAVORITE_EXT}")).exists() {
        name = format!("{base}-{n}");
        n += 1;
    }
    let piece = crate::scene_data::SceneData {
        three_d,
        objects: vec![o],
        ..Default::default()
    };
    let text = format!("{FAVORITE_MARK}\n{}", crate::scene_data::write(&piece));
    let rel = format!("{FAVORITES_DIR}/{name}.{FAVORITE_EXT}");
    std::fs::write(root.join(&rel), text).map_err(|e| e.to_string())?;
    Ok(rel)
}

/// O menu com os favoritos do projeto (desta dimensão) no topo.
pub fn with_favorites(
    mut entries: Vec<Entry>,
    root: &std::path::Path,
    three_d: bool,
) -> Vec<Entry> {
    let favs: Vec<Entry> = favorites(root)
        .into_iter()
        .filter(|f| f.three_d == three_d)
        .map(|f| {
            let object = f.object.clone();
            let mut it = item(&f.name, move || object.clone())
                .tip("Um favorito: cria uma cópia livre (mudar uma não muda as outras)")
                .words("favorito");
            it.anchor = Some(format!("cena.objeto:favorito:{}", f.name));
            it.into()
        })
        .collect();
    if !favs.is_empty() {
        entries.insert(0, sub("⭐ Favoritos", Some("cena.objeto:favoritos"), favs));
        entries.insert(1, Entry::Sep);
    }
    entries
}

// Menus ---------------------------------------------------------------------

/// Desenha o menu (o ➕ Objeto, ou dentro do menu do botão direito). Fecha o
/// menu e guarda nos recentes o que foi escolhido.
pub fn menu(ui: &mut egui::Ui, entries: &[Entry], recents: &str) -> Option<Picked> {
    let picked = menu_at(ui, entries, "");
    if let Some((key, p)) = picked {
        remember(ui.ctx(), recents, &key);
        ui.close();
        return Some(p);
    }
    None
}

fn menu_at(ui: &mut egui::Ui, entries: &[Entry], path: &str) -> Option<(String, Picked)> {
    let mut out = None;
    for e in entries {
        match e {
            Entry::Sep => {
                ui.separator();
            }
            Entry::Item(it) => {
                let mut r = ui.button(&it.label);
                if !it.tip.is_empty() {
                    r = r.on_hover_text(&it.tip);
                }
                if let Some(a) = &it.anchor {
                    crate::tutorial::anchor(ui.ctx(), a, r.rect);
                }
                if r.clicked() {
                    out = Some((format!("{path}{}", it.label), it.pick()));
                    ui.close();
                }
            }
            Entry::Sub {
                label,
                anchor,
                entries,
            } => {
                let r = ui.menu_button(*label, |ui| {
                    if let Some(p) = menu_at(ui, entries, &format!("{path}{label}/")) {
                        out = Some(p);
                        ui.close();
                    }
                });
                if let Some(a) = anchor {
                    crate::tutorial::anchor(ui.ctx(), a, r.response.rect);
                }
            }
        }
    }
    out
}

// Busca rápida ----------------------------------------------------------------

/// Um item na lista da busca, com o caminho no menu ("Luz › ").
#[derive(Debug)]
pub struct Flat<'a> {
    pub key: String,
    pub path: String,
    pub item: &'a Item,
}

/// Todos os itens, em ordem de menu.
pub fn flatten(entries: &[Entry]) -> Vec<Flat<'_>> {
    fn walk<'a>(entries: &'a [Entry], key: &str, path: &str, out: &mut Vec<Flat<'a>>) {
        for e in entries {
            match e {
                Entry::Item(it) => out.push(Flat {
                    key: format!("{key}{}", it.label),
                    path: path.to_string(),
                    item: it,
                }),
                Entry::Sub { label, entries, .. } => walk(
                    entries,
                    &format!("{key}{label}/"),
                    &format!("{path}{label} › "),
                    out,
                ),
                Entry::Sep => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(entries, "", "", &mut out);
    out
}

/// Os arquivos do projeto com estas extensões (caminhos relativos, em
/// ordem), sem pastas escondidas e sem o que foi exportado.
pub fn project_files(root: &std::path::Path, exts: &[&str]) -> Vec<String> {
    fn walk(
        root: &std::path::Path,
        dir: &std::path::Path,
        depth: u32,
        exts: &[&str],
        out: &mut Vec<String>,
    ) {
        let Ok(list) = std::fs::read_dir(dir) else {
            return;
        };
        for e in list.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "target" || name == "exportado" {
                continue;
            }
            if p.is_dir() {
                if depth < 4 {
                    walk(root, &p, depth + 1, exts, out);
                }
            } else if p
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| exts.contains(&x.to_lowercase().as_str()))
                && let Ok(rel) = p.strip_prefix(root)
            {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, 0, exts, &mut out);
    out.sort();
    out
}

/// Que tipo de arquivo é, para a busca: (caminho no menu, outras palavras).
fn file_kind(rel: &str) -> (&'static str, &'static str) {
    let ext = std::path::Path::new(rel)
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "prefab" => ("Prefab › ", "prefab"),
        "glb" | "gltf" => ("Modelo › ", "modelo glb"),
        _ => ("Imagem › ", "imagem sprite"),
    }
}

/// Os arquivos do projeto (prefabs, modelos, imagens) como itens da busca.
pub fn file_items(files: &[String]) -> Vec<Item> {
    files
        .iter()
        .map(|rel| {
            let stem = std::path::Path::new(rel)
                .file_stem()
                .map(|s| s.to_string_lossy().replace(['_', '-'], " "))
                .unwrap_or_default();
            Item {
                label: stem,
                tip: rel.clone(),
                words: file_kind(rel).1,
                anchor: None,
                make: Make::File(rel.clone()),
            }
        })
        .collect()
}

/// Distância de edição (trocar, pôr, tirar ou inverter duas letras vizinhas).
fn edits(a: &[char], b: &[char]) -> usize {
    let (n, m) = (a.len(), b.len());
    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=n {
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            d[i][j] = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[n][m]
}

/// Combina com um erro de digitação ("lus" acha "luz", "tocah" acha
/// "Tocha"): compara a busca com o começo de cada palavra.
fn typo_score(query: &str, text: &str) -> Option<i32> {
    let q: Vec<char> = fold(query)
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect();
    if q.len() < 3 {
        return None;
    }
    let allowed = if q.len() >= 6 { 2 } else { 1 };
    let folded = fold(text);
    let mut best: Option<usize> = None;
    for word in folded.split(|c: char| !c.is_alphanumeric()) {
        let w: Vec<char> = word.chars().collect();
        if w.len() + 1 < q.len() {
            continue;
        }
        // O começo da palavra, do tamanho da busca (com uma letra a mais ou
        // a menos), ou a palavra inteira.
        let mut d = edits(&q, &w);
        for len in [q.len().saturating_sub(1), q.len(), q.len() + 1] {
            if len > 0 && len <= w.len() {
                d = d.min(edits(&q, &w[..len]));
            }
        }
        best = Some(best.map_or(d, |b| b.min(d)));
    }
    let d = best?;
    (d <= allowed).then(|| 100 - d as i32 * 40)
}

/// A pontuação de um item na busca (maior = melhor).
pub fn score(query: &str, f: &Flat<'_>) -> Option<i32> {
    let q = query.trim();
    if q.is_empty() {
        return Some(0);
    }
    let label = &f.item.label;
    // O nome conta mais; o caminho no menu e as outras palavras, menos.
    let best = [
        fuzzy_score(q, label).map(|s| s + 2000),
        fuzzy_score(q, f.item.words).map(|s| s + 1000),
        fuzzy_score(q, &format!("{} {label}", f.path)),
    ]
    .into_iter()
    .flatten()
    .max();
    if best.is_some() {
        return best;
    }
    typo_score(q, &format!("{label} {}", f.item.words))
}

/// Os itens que combinam com a busca, do melhor para o pior. Sem busca: os
/// usados por último primeiro, depois o resto na ordem do menu.
pub fn rank(query: &str, list: &[Flat<'_>], recents: &[String]) -> Vec<usize> {
    let recent_pos = |key: &str| recents.iter().position(|r| r == key);
    if query.trim().is_empty() {
        let mut idx: Vec<usize> = (0..list.len()).collect();
        idx.sort_by_key(|&i| (recent_pos(&list[i].key).unwrap_or(usize::MAX), i));
        return idx;
    }
    let mut scored: Vec<(i32, usize)> = list
        .iter()
        .enumerate()
        .filter_map(|(i, f)| {
            let s = score(query, f)?;
            // Um pouco de vantagem para os usados há pouco.
            let bonus = recent_pos(&f.key).map_or(0, |p| 30 - p.min(8) as i32 * 3);
            Some((s + bonus, i))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.into_iter().map(|(_, i)| i).collect()
}

const MAX_RECENTS: usize = 8;

fn recents_id(kind: &str) -> egui::Id {
    egui::Id::new(("criar-recentes", kind))
}

/// O que foi criado por último (`kind`: "2d" ou "3d"), do mais novo ao mais velho.
pub fn recents(ctx: &egui::Context, kind: &str) -> Vec<String> {
    ctx.data_mut(|d| d.get_persisted::<Vec<String>>(recents_id(kind)))
        .unwrap_or_default()
}

pub fn remember(ctx: &egui::Context, kind: &str, key: &str) {
    ctx.data_mut(|d| {
        let list = d.get_persisted_mut_or_default::<Vec<String>>(recents_id(kind));
        list.retain(|k| k != key);
        list.insert(0, key.to_string());
        list.truncate(MAX_RECENTS);
    });
}

/// A caixinha da busca rápida (Shift+A), aberta no ponto do mouse.
#[derive(Debug, Clone)]
pub struct QuickAdd {
    pub query: String,
    pub selected: usize,
    /// Onde a caixa aparece na tela.
    pub pos: Pos2,
    /// Prefabs e modelos do projeto (lidos ao abrir).
    pub files: Vec<String>,
    /// "2d" ou "3d".
    pub kind: &'static str,
    /// Onde criar, no mundo (quem abriu decide; no 2D, z = 0).
    pub at: [f64; 3],
    focus: bool,
    /// Quadros desde que abriu (o clique que abriu não fecha).
    frames: u32,
}

impl QuickAdd {
    pub fn new(pos: Pos2, files: Vec<String>, kind: &'static str) -> Self {
        Self {
            query: String::new(),
            selected: 0,
            pos,
            files,
            kind,
            at: [0.0; 3],
            focus: true,
            frames: 0,
        }
    }
}

/// O resultado de um quadro da busca rápida.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum QuickResult {
    Open,
    Close,
    Pick(Picked),
}

const SHOWN: usize = 10;

/// Desenha a busca rápida.
pub fn quick_add_ui(ctx: &egui::Context, state: &mut QuickAdd, entries: &[Entry]) -> QuickResult {
    let files = file_items(&state.files);
    let mut list = flatten(entries);
    for f in &files {
        let Make::File(rel) = &f.make else { continue };
        list.push(Flat {
            key: format!("arquivo:{rel}"),
            path: file_kind(rel).0.into(),
            item: f,
        });
    }
    let recents = recents(ctx, state.kind);
    let found = rank(&state.query, &list, &recents);
    let n = found.len().min(SHOWN);
    state.frames += 1;

    // Setas e Enter antes da caixa de texto (ela usaria as setas).
    let (up, down, enter, esc) = ctx.input_mut(|i| {
        (
            i.consume_key(Modifiers::NONE, Key::ArrowUp),
            i.consume_key(Modifiers::NONE, Key::ArrowDown),
            i.consume_key(Modifiers::NONE, Key::Enter),
            i.consume_key(Modifiers::NONE, Key::Escape),
        )
    });
    if esc {
        return QuickResult::Close;
    }
    if n > 0 {
        if down {
            state.selected = (state.selected + 1) % n;
        }
        if up {
            state.selected = (state.selected + n - 1) % n;
        }
    }
    state.selected = state.selected.min(n.saturating_sub(1));
    let mut result = QuickResult::Open;
    let pick = |i: usize| {
        let f = &list[found[i]];
        remember(ctx, state.kind, &f.key);
        QuickResult::Pick(f.item.pick())
    };
    if enter {
        if n > 0 {
            return pick(state.selected);
        }
        return QuickResult::Close;
    }

    let area = egui::Area::new(egui::Id::new("busca-rapida"))
        .order(egui::Order::Foreground)
        .fixed_pos(state.pos)
        .constrain(true)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(280.0);
                let r = ui.add(
                    egui::TextEdit::singleline(&mut state.query)
                        .hint_text("Criar: luz, cubo, inimigo...")
                        .desired_width(f32::INFINITY),
                );
                crate::tutorial::anchor(ui.ctx(), "cena.busca", r.rect);
                if state.focus {
                    state.focus = false;
                    r.request_focus();
                }
                if r.changed() {
                    state.selected = 0;
                }
                ui.add_space(2.0);
                if n == 0 {
                    ui.weak("Nada com esse nome.");
                }
                for (row, &i) in found.iter().take(SHOWN).enumerate() {
                    let f = &list[i];
                    let text = if f.path.is_empty() {
                        RichText::new(&f.item.label)
                    } else {
                        RichText::new(format!("{}{}", f.path, f.item.label))
                    };
                    let mut resp = ui.add(
                        egui::Button::selectable(row == state.selected, text)
                            .min_size(egui::vec2(ui.available_width(), 0.0)),
                    );
                    if !f.item.tip.is_empty() {
                        resp = resp.on_hover_text(&f.item.tip);
                    }
                    if resp.clicked() {
                        result = pick(row);
                    }
                }
                ui.add_space(2.0);
                ui.weak("Setas escolhem · Enter cria · Esc fecha");
            });
        });
    if result != QuickResult::Open {
        return result;
    }
    // Clicou fora: fecha.
    let outside = ctx.input(|i| {
        i.pointer.any_pressed()
            && i.pointer
                .interact_pos()
                .is_some_and(|p| !area.response.rect.contains(p))
    });
    if outside && state.frames > 1 {
        return QuickResult::Close;
    }
    QuickResult::Open
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find<'a>(list: &'a [Flat<'a>], query: &str) -> Vec<&'a str> {
        rank(query, list, &[])
            .into_iter()
            .map(|i| list[i].item.label.as_str())
            .collect()
    }

    #[test]
    fn os_menus_tem_tudo_que_tinham() {
        let c3 = catalog3();
        let f3 = flatten(&c3);
        let keys: Vec<&str> = f3.iter().map(|f| f.key.as_str()).collect();
        for k in [
            "Cubo",
            "Plano (chão)",
            "Água (lago)",
            "Luz/Tocha",
            "Partículas/Fogo",
            "Portas e itens/Porta",
            "Portas e itens/Personagem que fala",
            "Ondas de inimigos",
            "Jogador 3D",
            "Na tela (HUD)/Texto na tela",
            "Na tela (HUD)/Menu com botões",
        ] {
            assert!(keys.contains(&k), "falta {k} no 3D");
        }
        let c2 = catalog2();
        let f2 = flatten(&c2);
        let keys: Vec<&str> = f2.iter().map(|f| f.key.as_str()).collect();
        for k in [
            "Vazio",
            "Círculo",
            "Partículas/Vazias",
            "Interface/Botão",
            "Mapa de tiles...",
        ] {
            assert!(keys.contains(&k), "falta {k} no 2D");
        }
        // Cada item cria o que deve.
        let tocha = f3.iter().find(|f| f.key == "Luz/Tocha").unwrap();
        let Picked::Object(o) = tocha.item.pick() else {
            panic!("a tocha é um objeto")
        };
        assert_eq!(o.name, "Tocha");
        assert_eq!(o.children.len(), 2);
        let circulo = f2.iter().find(|f| f.key == "Círculo").unwrap();
        let Picked::Object(o) = circulo.item.pick() else {
            panic!()
        };
        assert_eq!(o.components[0].str("shape"), "circle");
        let mapa = f2.iter().find(|f| f.key == "Mapa de tiles...").unwrap();
        assert_eq!(mapa.item.pick(), Picked::NewMap);
    }

    #[test]
    fn busca_acha_pelo_nome_pelas_palavras_e_com_erro() {
        let c3 = catalog3();
        let f3 = flatten(&c3);
        assert_eq!(find(&f3, "cubo")[0], "Cubo");
        assert_eq!(find(&f3, "tocha")[0], "Tocha");
        // Outras palavras: "bloco" acha o Cubo; "inimigo", as Ondas.
        assert_eq!(find(&f3, "bloco")[0], "Cubo");
        assert_eq!(find(&f3, "inimigo")[0], "Ondas de inimigos");
        assert_eq!(find(&f3, "lago")[0], "Água (lago)");
        // Sem acento e com erro de digitação.
        assert_eq!(find(&f3, "agua")[0], "Água (lago)");
        assert_eq!(find(&f3, "tocah")[0], "Tocha");
        let luz = find(&f3, "lus");
        assert!(
            ["Lâmpada", "Tocha", "Holofote"].contains(&luz[0]),
            "lus acha uma luz: {luz:?}"
        );
        assert!(find(&f3, "xyzw").is_empty());
    }

    #[test]
    fn recentes_vem_primeiro() {
        let c3 = catalog3();
        let f3 = flatten(&c3);
        let recents = vec!["Luz/Holofote".to_string(), "Terreno".to_string()];
        let order = rank("", &f3, &recents);
        assert_eq!(f3[order[0]].item.label, "Holofote");
        assert_eq!(f3[order[1]].item.label, "Terreno");
        assert_eq!(f3[order[2]].item.label, "Cubo");
        // Com busca, o recente ganha um pouco: "luz" escolhe o Holofote.
        let order = rank("luz", &f3, &recents);
        assert_eq!(f3[order[0]].item.label, "Holofote");

        let ctx = egui::Context::default();
        remember(&ctx, "3d", "Cubo");
        remember(&ctx, "3d", "Terreno");
        remember(&ctx, "3d", "Cubo");
        assert_eq!(super::recents(&ctx, "3d"), ["Cubo", "Terreno"]);
        assert!(super::recents(&ctx, "2d").is_empty());
    }

    #[test]
    fn prefabs_e_modelos_na_busca() {
        let files = file_items(&[
            "prefabs/inimigo_lobo.prefab".into(),
            "modelos/arvore.glb".into(),
        ]);
        assert_eq!(files[0].label, "inimigo lobo");
        let list: Vec<Flat<'_>> = files
            .iter()
            .map(|f| Flat {
                key: f.label.clone(),
                path: String::new(),
                item: f,
            })
            .collect();
        assert_eq!(find(&list, "lobo"), ["inimigo lobo"]);
        assert_eq!(
            list[1].item.pick(),
            Picked::File("modelos/arvore.glb".into())
        );
    }

    #[test]
    fn distancia_de_edicao() {
        let c = |s: &str| s.chars().collect::<Vec<_>>();
        assert_eq!(edits(&c("tocha"), &c("tocah")), 1);
        assert_eq!(edits(&c("luz"), &c("lus")), 1);
        assert_eq!(edits(&c("cubo"), &c("cubo")), 0);
        assert_eq!(edits(&c(""), &c("abc")), 3);
    }
}
