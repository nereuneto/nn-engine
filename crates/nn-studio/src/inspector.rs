//! Painel Inspetor: as propriedades do objeto selecionado (ou da cena, sem
//! seleção). Cada componente tem os campos com o controle certo, e dá para
//! arrastar imagens, sons e mapas do painel Arquivos direto para os campos.

use std::collections::HashMap;
use std::time::SystemTime;

use eframe::egui::{self, Color32, RichText, Vec2};

use crate::behaviors::{self, BehaviorInfo, Catalog, PropType};
use crate::files::{FileKind, FileTree};
use crate::scene_data::{Component, Object, Prop};
use crate::scene_view::{FileDrag, GameState, SceneDoc};
use crate::settings::Mode;

/// Tipo de controle de um campo.
#[derive(Debug, Clone, Copy)]
enum W {
    Num {
        speed: f64,
        min: f64,
        max: f64,
        suffix: &'static str,
    },
    Int {
        min: f64,
        max: f64,
    },
    Bool,
    Text,
    Multiline,
    Color,
    Choice(&'static [(&'static str, &'static str)]),
    File(FileKind),
    Object,
    /// Nome de uma animação da imagem do objeto.
    Anim,
    /// Um osso do modelo do pai (componente Preso a um osso).
    Bone,
    /// Cor que, vazia, vem do tema da interface (a chave do tema).
    ThemeColor(&'static str),
    /// Variável do jogo.
    Var,
    /// Fonte (.ttf); vazia = a do tema.
    Font,
    /// Cena (.cena ou .lua).
    Scene,
    /// Cor opcional (vazia = sem).
    OptColor,
}

#[derive(Debug, Clone, Copy)]
struct F {
    key: &'static str,
    label: &'static str,
    w: W,
    advanced: bool,
    help: &'static str,
}

const fn f(key: &'static str, label: &'static str, w: W) -> F {
    F {
        key,
        label,
        w,
        advanced: false,
        help: "",
    }
}

const fn adv(key: &'static str, label: &'static str, w: W) -> F {
    F {
        key,
        label,
        w,
        advanced: true,
        help: "",
    }
}

const fn help(mut x: F, help: &'static str) -> F {
    x.help = help;
    x
}

const SIZE: W = W::Num {
    speed: 1.0,
    min: 0.0,
    max: 100_000.0,
    suffix: "",
};
const ANY: W = W::Num {
    speed: 1.0,
    min: -100_000.0,
    max: 100_000.0,
    suffix: "",
};
const ANCHORS: &[(&str, &str)] = &[
    ("center", "Centro"),
    ("topleft", "Canto superior esquerdo"),
    ("top", "Em cima"),
    ("topright", "Canto superior direito"),
    ("left", "Esquerda"),
    ("right", "Direita"),
    ("bottomleft", "Canto inferior esquerdo"),
    ("bottom", "Embaixo"),
    ("bottomright", "Canto inferior direito"),
];

const SPRITE_FIELDS: &[F] = &[
    f("image", "Imagem", W::File(FileKind::Image)),
    help(
        f(
            "frame_w",
            "Largura do quadro",
            W::Int {
                min: 0.0,
                max: 4096.0,
            },
        ),
        "Para spritesheets: o tamanho de cada quadro. 0 = a imagem inteira.",
    ),
    f(
        "frame_h",
        "Altura do quadro",
        W::Int {
            min: 0.0,
            max: 4096.0,
        },
    ),
    f(
        "frame",
        "Quadro",
        W::Int {
            min: 1.0,
            max: 100_000.0,
        },
    ),
    help(
        f("w", "Largura", SIZE),
        "0 = o tamanho da imagem (ou do quadro).",
    ),
    f("h", "Altura", SIZE),
    help(
        f("color", "Cor", W::Color),
        "Tinge a imagem. Branco = cores originais.",
    ),
    help(
        f("animation", "Animação", W::Anim),
        "Animação da imagem que toca ao começar (feita no editor de animação, na aba da imagem).",
    ),
    help(
        f(
            "fill",
            "Preencher",
            W::Choice(&[("stretch", "Esticar"), ("repeat", "Repetir")]),
        ),
        "Repetir: a imagem se repete lado a lado em vez de esticar; a escala do objeto diz quantas vezes (chão, paredes, fundos).",
    ),
    f("flip_x", "Espelhar na horizontal", W::Bool),
    adv("flip_y", "Espelhar na vertical", W::Bool),
    adv("pixel_art", "Pixel art (sem suavizar)", W::Bool),
    HUD,
];

const HUD: F = help(
    f("hud", "Fixo na tela (HUD)", W::Bool),
    "Não anda com a câmera: placar, vidas, botões. A posição conta a partir do ponto da tela escolhido em Transformação (Preso em).",
);

const TEXT_FIELDS: &[F] = &[
    f("text", "Texto", W::Multiline),
    f("size", "Tamanho", SIZE),
    f("color", "Cor", W::Color),
    help(
        f("anchor", "Âncora", W::Choice(ANCHORS)),
        "Qual ponto do texto fica na posição do objeto.",
    ),
    FONT,
    help(
        f("outline", "Contorno", W::OptColor),
        "Uma borda em volta das letras, nesta cor: o texto lê bem em cima de qualquer fundo (um placar em cima do céu).",
    ),
    HUD,
];

const SHAPE_FIELDS: &[F] = &[
    f(
        "shape",
        "Forma",
        W::Choice(&[("rect", "Retângulo"), ("circle", "Círculo")]),
    ),
    f("w", "Largura", SIZE),
    f("h", "Altura", SIZE),
    f("radius", "Raio", SIZE),
    f("color", "Cor", W::Color),
    HUD,
];

const BODY_FIELDS: &[F] = &[
    f(
        "body",
        "Tipo",
        W::Choice(&[
            ("dynamic", "Dinâmico (cai e colide)"),
            ("static", "Estático (chão, parede)"),
            ("kinematic", "Cinemático (plataforma móvel)"),
        ]),
    ),
    f(
        "shape",
        "Forma",
        W::Choice(&[("box", "Caixa"), ("circle", "Círculo")]),
    ),
    f("w", "Largura", SIZE),
    f("h", "Altura", SIZE),
    f("radius", "Raio", SIZE),
    help(
        f("sensor", "Sensor", W::Bool),
        "Não bloqueia: só avisa quando algo entra (moedas, portas, espinhos).",
    ),
    help(
        f("one_way", "Mão única", W::Bool),
        "Só bloqueia quem vem de cima (plataformas que dá para pular por baixo).",
    ),
    help(
        f(
            "bounce",
            "Quique",
            W::Num {
                speed: 0.01,
                min: 0.0,
                max: 1.0,
                suffix: "",
            },
        ),
        "0 = para ao bater, 1 = quica sem perder velocidade.",
    ),
    help(
        adv(
            "gravity_scale",
            "Gravidade",
            W::Num {
                speed: 0.05,
                min: -10.0,
                max: 10.0,
                suffix: "×",
            },
        ),
        "Multiplica a gravidade da cena (0 = flutua).",
    ),
    help(
        adv("tag", "Rótulo", W::Text),
        "Rótulo do corpo (padrão: o nome do objeto).",
    ),
];

const MAP_FIELDS: &[F] = &[
    f("map", "Mapa (.tmj)", W::File(FileKind::Map)),
    help(
        f(
            "scale",
            "Escala",
            W::Num {
                speed: 0.1,
                min: 0.1,
                max: 32.0,
                suffix: "×",
            },
        ),
        "Tiles de 16 px com escala 3 viram 48 unidades.",
    ),
    help(
        f("collision", "Colisão dos tiles", W::Bool),
        "Os tiles marcados como sólidos no Tiled bloqueiam os corpos físicos.",
    ),
];

const CAMERA_FIELDS: &[F] = &[
    help(
        f("follow", "Seguir", W::Object),
        "Objeto que a câmera acompanha. Sem nenhum, ela fica parada aqui.",
    ),
    f(
        "zoom",
        "Zoom",
        W::Num {
            speed: 0.05,
            min: 0.05,
            max: 20.0,
            suffix: "×",
        },
    ),
    help(
        f("limit_to_map", "Não passar do mapa", W::Bool),
        "Não mostra nada fora do primeiro mapa da cena.",
    ),
    help(
        adv(
            "smooth",
            "Suavidade",
            W::Num {
                speed: 0.1,
                min: 0.0,
                max: 50.0,
                suffix: "",
            },
        ),
        "Rapidez para alcançar o alvo; 0 vai direto.",
    ),
];

const SOUND_FIELDS: &[F] = &[
    f("sound", "Som", W::File(FileKind::Sound)),
    f("autoplay", "Tocar ao começar", W::Bool),
    f("loop", "Repetir", W::Bool),
    f(
        "volume",
        "Volume",
        W::Num {
            speed: 0.01,
            min: 0.0,
            max: 2.0,
            suffix: "",
        },
    ),
    f(
        "channel",
        "Canal",
        W::Choice(&[("effects", "Efeitos"), ("music", "Música")]),
    ),
    help(
        f("spatial", "Som no espaço 3D", W::Bool),
        "O som sai do objeto: mais baixo longe dele e do lado em que ele está (fogueira, cachoeira, rádio). Desligado: igual em todo lugar (música).",
    ),
    help(
        f(
            "distance",
            "Ouvido até",
            W::Num {
                speed: 0.5,
                min: 1.0,
                max: 1000.0,
                suffix: " m",
            },
        ),
        "Mais longe que isso, não se ouve. Até um quarto dessa distância, o volume é cheio.",
    ),
];

/// Título de um grupo de campos (não é um campo).
const fn section(label: &'static str) -> F {
    F {
        key: "",
        label,
        w: W::Bool,
        advanced: false,
        help: "",
    }
}

const DEGREES: W = W::Num {
    speed: 1.0,
    min: -360.0,
    max: 360.0,
    suffix: "°",
};
const FRACTION: W = W::Num {
    speed: 0.01,
    min: 0.0,
    max: 1.0,
    suffix: "",
};

const PARTICLES_FIELDS: &[F] = &[
    section("Emissão"),
    help(
        f(
            "mode",
            "Modo",
            W::Choice(&[
                ("continuo", "Contínuo (fogo, chuva)"),
                ("explosao", "De uma vez (explosão)"),
            ]),
        ),
        "Contínuo solta sem parar; de uma vez solta várias juntas (ao começar, por um evento ou repetindo).",
    ),
    f("rate", "Por segundo", SIZE),
    help(
        f("emitting", "Emitindo", W::Bool),
        "Desligado, só começa com a ação \"Ligar ou desligar partículas\".",
    ),
    f(
        "amount",
        "Quantidade",
        W::Int {
            min: 1.0,
            max: 5000.0,
        },
    ),
    help(
        f("on_start", "Soltar ao começar", W::Bool),
        "Desligado, solta só com a ação \"Soltar partículas\" (poeira ao pular).",
    ),
    help(
        adv(
            "interval",
            "Repetir a cada",
            W::Num {
                speed: 0.05,
                min: 0.0,
                max: 600.0,
                suffix: " s",
            },
        ),
        "0 = não repete.",
    ),
    help(
        f(
            "when_done",
            "Ao terminar",
            W::Choice(&[("nada", "Nada"), ("destruir", "Destruir o objeto")]),
        ),
        "Para efeitos em prefab: a explosão aparece, termina e some sozinha.",
    ),
    help(
        adv(
            "area",
            "Nascem em",
            W::Choice(&[
                ("ponto", "Um ponto"),
                ("retangulo", "Um retângulo"),
                ("circulo", "Um círculo"),
            ]),
        ),
        "Chuva e neve: um retângulo largo no alto da tela.",
    ),
    adv("area_w", "Largura da área", SIZE),
    adv("area_h", "Altura da área", SIZE),
    adv("area_radius", "Raio da área", SIZE),
    help(
        adv(
            "max",
            "Máximo vivas",
            W::Int {
                min: 1.0,
                max: 20_000.0,
            },
        ),
        "Limite de partículas ao mesmo tempo.",
    ),
    section("Movimento"),
    f(
        "lifetime",
        "Vida",
        W::Num {
            speed: 0.05,
            min: 0.0,
            max: 60.0,
            suffix: " s",
        },
    ),
    help(
        adv("lifetime_var", "Variação da vida", FRACTION),
        "0,5 = umas vivem até a metade do tempo.",
    ),
    f("speed_min", "Velocidade mínima", ANY),
    f("speed_max", "Velocidade máxima", ANY),
    help(
        f("direction", "Direção", DEGREES),
        "90 = para cima. Dá para arrastar a seta na cena.",
    ),
    help(
        f(
            "spread",
            "Abertura",
            W::Num {
                speed: 1.0,
                min: 0.0,
                max: 360.0,
                suffix: "°",
            },
        ),
        "360 = para todos os lados.",
    ),
    help(
        f("gravity", "Gravidade", ANY),
        "Negativa = caem; positiva = sobem (fumaça).",
    ),
    help(
        adv(
            "drag",
            "Freio",
            W::Num {
                speed: 0.05,
                min: 0.0,
                max: 50.0,
                suffix: "",
            },
        ),
        "Quanto perdem de velocidade por segundo (explosões: 3 ou 4).",
    ),
    help(
        adv("follow", "Acompanham o objeto", W::Bool),
        "Ligado, as partículas andam junto com o objeto (jato de uma nave). Desligado, ficam onde nasceram (rastro).",
    ),
    help(
        adv("wind", "Levadas pelo vento", FRACTION),
        "No 3D, com Vento (☀ Ambiente): quanto o vento leva as partículas (0 = nada, 1 = tudo).",
    ),
    section("Aparência"),
    f("size_start", "Tamanho inicial", SIZE),
    f("size_end", "Tamanho final", SIZE),
    adv("size_var", "Variação do tamanho", FRACTION),
    f("color_start", "Cor inicial", W::Color),
    help(
        f("color_mid", "Cor do meio", W::OptColor),
        "Opcional: uma cor no meio da vida (fogo: amarelo, laranja, vermelho).",
    ),
    f("color_end", "Cor final", W::Color),
    help(
        f(
            "blend",
            "Mistura",
            W::Choice(&[("alpha", "Normal"), ("add", "Luz (somar cores)")]),
        ),
        "Luz deixa fogo, faíscas e brilho mais fortes.",
    ),
    help(
        adv("image", "Imagem", W::File(FileKind::Image)),
        "Vazio = um círculo suave. Com imagem: folhas, estrelas, pixel art.",
    ),
    adv(
        "frame_w",
        "Largura do quadro",
        W::Int {
            min: 0.0,
            max: 4096.0,
        },
    ),
    adv(
        "frame_h",
        "Altura do quadro",
        W::Int {
            min: 0.0,
            max: 4096.0,
        },
    ),
    adv(
        "frame",
        "Quadro",
        W::Int {
            min: 1.0,
            max: 100_000.0,
        },
    ),
    help(
        adv(
            "spin",
            "Giro",
            W::Num {
                speed: 1.0,
                min: 0.0,
                max: 3600.0,
                suffix: "°/s",
            },
        ),
        "Cada partícula gira até essa velocidade, para um lado ou para o outro.",
    ),
];

// Interface (menus e HUD) ------------------------------------------------

const SLICE: F = help(
    adv(
        "slice",
        "Margem das fatias",
        W::Num {
            speed: 0.2,
            min: 0.0,
            max: 512.0,
            suffix: " px",
        },
    ),
    "Com imagem: quantos pixels de cada canto não esticam (9 fatias).",
);
const SLICE_SCALE: F = help(
    adv(
        "slice_scale",
        "Escala das fatias",
        W::Num {
            speed: 0.05,
            min: 0.0,
            max: 64.0,
            suffix: "",
        },
    ),
    "Com imagem: quanto cada pixel dos cantos cresce. Pixel art pequena: 3 ou 4.",
);
const UI_HUD: F = help(
    adv("hud", "Fixo na tela", W::Bool),
    "Fica preso à tela e não anda com a câmera. Desligado, fica no mundo do jogo.",
);
const FONT: F = help(
    adv("font", "Fonte", W::Font),
    "Arquivo .ttf ou .otf. Vazio = a fonte do tema (ou a da engine).",
);
const RADIUS: F = f("radius", "Cantos arredondados", SIZE);
const BORDER: F = f("border", "Borda", SIZE);
const BORDER_COLOR: F = f("border_color", "Cor da borda", W::ThemeColor("border"));
const VAR: F = help(
    f("var", "Variável", W::Var),
    "Variável do jogo (Configurações do projeto) que o componente mostra ou muda.",
);
const PERCENT: W = W::Num {
    speed: 0.01,
    min: -100_000.0,
    max: 100_000.0,
    suffix: "",
};

const PANEL_FIELDS: &[F] = &[
    f("w", "Largura", SIZE),
    f("h", "Altura", SIZE),
    f("color", "Cor", W::ThemeColor("panel")),
    RADIUS,
    BORDER,
    BORDER_COLOR,
    help(
        f("image", "Imagem", W::File(FileKind::Image)),
        "Uma moldura desenhada: os cantos ficam do mesmo tamanho e o meio estica.",
    ),
    SLICE,
    SLICE_SCALE,
    UI_HUD,
];

const BUTTON_ACTIONS: &[(&str, &str)] = &[
    ("nada", "Nada (use os eventos)"),
    ("cena", "Trocar de cena"),
    ("por_cima", "Abrir cena por cima"),
    ("fechar", "Fechar esta cena (por cima)"),
    ("reiniciar", "Reiniciar a cena"),
    ("sair", "Sair do jogo"),
];

const BUTTON_FIELDS: &[F] = &[
    f("text", "Texto", W::Text),
    help(
        f("action", "Ao apertar", W::Choice(BUTTON_ACTIONS)),
        "O que o botão faz. Para outras coisas, use a condição \"Clicar\" nos eventos.",
    ),
    help(
        f("scene", "Cena", W::Scene),
        "Para \"Trocar de cena\" e \"Abrir cena por cima\".",
    ),
    f("size", "Tamanho do texto", SIZE),
    f("w", "Largura", SIZE),
    f("h", "Altura", SIZE),
    f("color", "Cor", W::ThemeColor("button")),
    help(
        f("hover_color", "Cor com foco", W::ThemeColor("hover")),
        "Com o mouse em cima, ou escolhido pelo teclado ou controle.",
    ),
    f("press_color", "Cor apertado", W::ThemeColor("press")),
    f("text_color", "Cor do texto", W::ThemeColor("text")),
    RADIUS,
    adv("border", "Borda", SIZE),
    adv("border_color", "Cor da borda", W::ThemeColor("border")),
    help(
        f("icon", "Ícone", W::File(FileKind::Image)),
        "Imagem pequena antes do texto.",
    ),
    adv("image", "Imagem do fundo", W::File(FileKind::Image)),
    SLICE,
    SLICE_SCALE,
    FONT,
    help(
        adv("sound", "Som", W::File(FileKind::Sound)),
        "Toca ao apertar. Vazio = o som do tema.",
    ),
    help(
        f("focus", "Começa com foco", W::Bool),
        "O primeiro escolhido quando o menu abre (para jogar com teclado ou controle).",
    ),
    f("enabled", "Ativo", W::Bool),
    UI_HUD,
];

const BAR_FIELDS: &[F] = &[
    VAR,
    help(
        f("value", "Valor", PERCENT),
        "Sem variável: o valor mostrado.",
    ),
    help(f("max", "Máximo", PERCENT), "O valor da barra cheia."),
    help(
        adv("max_var", "Variável do máximo", W::Var),
        "Se o máximo muda no jogo (vida máxima, por exemplo).",
    ),
    f("w", "Largura", SIZE),
    f("h", "Altura", SIZE),
    f("color", "Cor", W::ThemeColor("accent")),
    f("back_color", "Cor do fundo", W::Color),
    RADIUS,
    adv("border", "Borda", SIZE),
    adv("border_color", "Cor da borda", W::ThemeColor("border")),
    f(
        "direction",
        "Enche para",
        W::Choice(&[
            ("right", "Direita"),
            ("left", "Esquerda"),
            ("up", "Cima"),
            ("down", "Baixo"),
        ]),
    ),
    UI_HUD,
];

const ICONS_FIELDS: &[F] = &[
    help(
        f("image", "Imagem", W::File(FileKind::Image)),
        "Um coração, uma moeda, uma estrela...",
    ),
    help(
        adv(
            "frame_w",
            "Largura do quadro",
            W::Int {
                min: 0.0,
                max: 4096.0,
            },
        ),
        "Para spritesheets. 0 = a imagem inteira.",
    ),
    adv(
        "frame_h",
        "Altura do quadro",
        W::Int {
            min: 0.0,
            max: 4096.0,
        },
    ),
    adv(
        "frame",
        "Quadro",
        W::Int {
            min: 1.0,
            max: 100_000.0,
        },
    ),
    help(
        VAR,
        "Variável do jogo com quantos ícones mostrar (vidas, por exemplo).",
    ),
    help(
        f(
            "count",
            "Quantidade",
            W::Int {
                min: 0.0,
                max: 100.0,
            },
        ),
        "Sem variável: quantos ícones mostrar.",
    ),
    help(
        f(
            "max",
            "Máximo",
            W::Int {
                min: 0.0,
                max: 100.0,
            },
        ),
        "Mostra os que faltam apagados (3 de 5 corações). 0 = só os que tem.",
    ),
    f("size", "Tamanho", SIZE),
    f("spacing", "Espaço entre eles", SIZE),
    f("color", "Cor", W::Color),
    f("empty_color", "Cor dos que faltam", W::Color),
    f(
        "align",
        "Alinhar",
        W::Choice(&[
            ("left", "À esquerda"),
            ("center", "No meio"),
            ("right", "À direita"),
        ]),
    ),
    adv("pixel_art", "Pixel art", W::Bool),
    UI_HUD,
];

const SLIDER_FIELDS: &[F] = &[
    help(
        f(
            "target",
            "Muda",
            W::Choice(&[
                ("variavel", "Uma variável"),
                ("musica", "Volume da música"),
                ("efeitos", "Volume dos efeitos"),
                ("geral", "Volume geral"),
            ]),
        ),
        "Os volumes ficam guardados para a próxima vez que o jogo abrir.",
    ),
    VAR,
    help(
        f("value", "Valor", PERCENT),
        "O valor inicial (sem variável).",
    ),
    f("min", "Mínimo", PERCENT),
    f("max", "Máximo", PERCENT),
    help(
        f(
            "step",
            "Passo",
            W::Num {
                speed: 0.01,
                min: 0.0,
                max: 100_000.0,
                suffix: "",
            },
        ),
        "Quanto anda com as setas. 0 = um décimo.",
    ),
    f("w", "Largura", SIZE),
    f("h", "Altura", SIZE),
    f("color", "Cor", W::ThemeColor("accent")),
    f("back_color", "Cor do fundo", W::Color),
    f("knob_color", "Cor do botão", W::Color),
    f("knob", "Tamanho do botão", SIZE),
    UI_HUD,
];

const CHECKBOX_FIELDS: &[F] = &[
    f("text", "Texto", W::Text),
    f(
        "target",
        "Muda",
        W::Choice(&[("variavel", "Uma variável"), ("tela_cheia", "Tela cheia")]),
    ),
    VAR,
    help(
        f("checked", "Marcada", W::Bool),
        "Como começa (sem variável).",
    ),
    f("size", "Tamanho do texto", SIZE),
    f("box", "Tamanho da caixa", SIZE),
    f("color", "Cor da caixa", W::ThemeColor("button")),
    f("check_color", "Cor do ✔", W::ThemeColor("accent")),
    f("text_color", "Cor do texto", W::ThemeColor("text")),
    FONT,
    UI_HUD,
];

const TEXTFIELD_FIELDS: &[F] = &[
    help(f("text", "Texto", W::Text), "O texto que já vem escrito."),
    help(
        f("placeholder", "Dica", W::Text),
        "Aparece apagada enquanto o campo está vazio.",
    ),
    help(
        VAR,
        "Variável do jogo que guarda o que foi digitado (o nome do jogador, por exemplo).",
    ),
    help(
        f(
            "max",
            "Máximo de letras",
            W::Int {
                min: 1.0,
                max: 1000.0,
            },
        ),
        "",
    ),
    f("w", "Largura", SIZE),
    f("h", "Altura", SIZE),
    f("size", "Tamanho do texto", SIZE),
    f("color", "Cor do fundo", W::Color),
    f("text_color", "Cor do texto", W::ThemeColor("text")),
    BORDER,
    f("border_color", "Cor da borda", W::ThemeColor("accent")),
    RADIUS,
    FONT,
    UI_HUD,
];

const LAYOUT_FIELDS: &[F] = &[
    help(
        f(
            "arrange",
            "Arrumar os filhos",
            W::Choice(&[
                ("vertical", "Em coluna"),
                ("horizontal", "Em linha"),
                ("grid", "Em grade"),
                ("none", "Não arrumar"),
            ]),
        ),
        "Os filhos ficam um ao lado do outro, sem você posicionar cada um.",
    ),
    f("spacing", "Espaço", SIZE),
    help(
        f(
            "columns",
            "Colunas",
            W::Int {
                min: 1.0,
                max: 50.0,
            },
        ),
        "Na grade.",
    ),
    f(
        "align",
        "Alinhar",
        W::Choice(&[
            ("start", "No começo"),
            ("center", "No meio"),
            ("end", "No fim"),
        ]),
    ),
];

const MODEL_FIELDS: &[F] = &[
    help(
        f("file", "Modelo", W::File(FileKind::Model)),
        "Um arquivo .glb ou .gltf (exporte do Blender como glTF Binary). Também dá para arrastar o arquivo para a cena.",
    ),
    help(
        f("color", "Cor", W::Color),
        "Multiplica as cores do modelo (branco = como ele é).",
    ),
    help(
        adv("flat", "Faces retas (low-poly)", W::Bool),
        "Cada face com um tom só, mesmo que o modelo tenha sombreado suave.",
    ),
    f("shadow", "Projeta sombra", W::Bool),
    help(
        f(
            "glow",
            "Brilho",
            W::Num {
                speed: 0.01,
                min: 0.0,
                max: 1.0,
                suffix: "",
            },
        ),
        "Quanto a própria cor brilha (0 a 1): aparece acesa mesmo no escuro, como um cristal ou uma janela à noite. Não ilumina em volta (para isso, uma Luz).",
    ),
    help(
        f("glow_night", "Brilha só à noite", W::Bool),
        "Com Dia e noite (☀ Ambiente), o brilho acende ao anoitecer e apaga ao amanhecer: janelas, postes. Sem o ciclo, vale num ambiente escuro (Noite, Caverna).",
    ),
    help(
        f(
            "sway",
            "Balança com o vento",
            W::Num {
                speed: 0.01,
                min: 0.0,
                max: 2.0,
                suffix: "",
            },
        ),
        "Com Vento (☀ Ambiente): embaixo fica parado e em cima balança (1 = uma árvore; 0 = nada). Bom para árvores, grama, bandeiras.",
    ),
];

const SHAPE3D_FIELDS: &[F] = &[
    f(
        "shape",
        "Forma",
        W::Choice(&[
            ("cube", "Cubo"),
            ("sphere", "Esfera"),
            ("cylinder", "Cilindro"),
            ("cone", "Cone"),
            ("capsule", "Cápsula"),
            ("plane", "Plano (chão)"),
        ]),
    ),
    f("color", "Cor", W::Color),
    f("shadow", "Projeta sombra", W::Bool),
    help(
        f(
            "glow",
            "Brilho",
            W::Num {
                speed: 0.01,
                min: 0.0,
                max: 1.0,
                suffix: "",
            },
        ),
        "Quanto a própria cor brilha (0 a 1): aparece acesa mesmo no escuro, como um cristal ou uma janela à noite. Não ilumina em volta (para isso, uma Luz).",
    ),
    help(
        f("glow_night", "Brilha só à noite", W::Bool),
        "Com Dia e noite (☀ Ambiente), o brilho acende ao anoitecer e apaga ao amanhecer: janelas, postes. Sem o ciclo, vale num ambiente escuro (Noite, Caverna).",
    ),
    help(
        f(
            "material",
            "Material",
            W::Choice(&[("normal", "Normal"), ("agua", "Água (ondas e reflexo)")]),
        ),
        "Água: ondinhas que se mexem, o céu refletido (mais forte olhando de lado) e o brilho do sol. Use num Plano com uma cor um pouco transparente.",
    ),
    help(
        f(
            "sway",
            "Balança com o vento",
            W::Num {
                speed: 0.01,
                min: 0.0,
                max: 2.0,
                suffix: "",
            },
        ),
        "Com Vento (☀ Ambiente): embaixo fica parado e em cima balança (1 = uma árvore; 0 = nada). Bom para árvores, grama, bandeiras.",
    ),
];

const LIGHT3D_FIELDS: &[F] = &[
    help(
        f(
            "kind",
            "Tipo",
            W::Choice(&[
                ("point", "Lâmpada (em volta)"),
                ("spot", "Holofote (num cone)"),
            ]),
        ),
        "Lâmpada: ilumina para todos os lados, como uma tocha. Holofote: ilumina num cone, para a frente do objeto (a seta azul, -Z); gire o objeto para apontar.",
    ),
    f("on", "Acesa", W::Bool),
    help(
        f("night", "Só à noite", W::Bool),
        "Com Dia e noite (☀ Ambiente), acende sozinha ao anoitecer e apaga ao amanhecer. Sem o ciclo, fica acesa num ambiente escuro (Noite, Caverna).",
    ),
    f("color", "Cor", W::Color),
    help(
        f(
            "intensity",
            "Força",
            W::Num {
                speed: 0.05,
                min: 0.0,
                max: 20.0,
                suffix: "",
            },
        ),
        "1 é como o sol bem perto da luz; 2 ou 3 para um holofote forte.",
    ),
    help(
        f(
            "range",
            "Alcance",
            W::Num {
                speed: 0.1,
                min: 0.1,
                max: 200.0,
                suffix: " m",
            },
        ),
        "Até onde a luz chega (vai sumindo aos poucos até lá).",
    ),
    help(
        f(
            "angle",
            "Abertura do cone",
            W::Num {
                speed: 0.5,
                min: 1.0,
                max: 89.0,
                suffix: "°",
            },
        ),
        "Holofote: metade da abertura do cone.",
    ),
    help(
        adv(
            "soft",
            "Borda suave",
            W::Num {
                speed: 0.01,
                min: 0.0,
                max: 1.0,
                suffix: "",
            },
        ),
        "Holofote: quanto da borda do cone vai apagando aos poucos (0 = borda dura).",
    ),
    help(
        f(
            "flicker",
            "Tremular",
            W::Num {
                speed: 0.01,
                min: 0.0,
                max: 1.0,
                suffix: "",
            },
        ),
        "Faz a luz oscilar como fogo (0 = parada; 0,5 para uma tocha).",
    ),
];

const CAMERA3D_FIELDS: &[F] = &[help(
    f(
        "fov",
        "Campo de visão",
        W::Num {
            speed: 0.5,
            min: 10.0,
            max: 150.0,
            suffix: "°",
        },
    ),
    "Quanto ela enxerga de cima a baixo. 60° é o normal; mais = mais aberto.",
)];

const TEXT3D_FIELDS: &[F] = &[
    help(
        f("text", "Texto", W::Multiline),
        "Escrito no mundo 3D: placas, nomes em cima dos personagens, avisos. O Mostrar valor troca {variavel} pelo valor.",
    ),
    help(
        f(
            "size",
            "Tamanho",
            W::Num {
                speed: 0.01,
                min: 0.01,
                max: 100.0,
                suffix: " m",
            },
        ),
        "A altura das letras, em metros (a escala do objeto também conta).",
    ),
    f("color", "Cor", W::Color),
    help(
        f(
            "facing",
            "Virado para",
            W::Choice(&[
                ("camera", "A câmera (sempre de frente)"),
                ("placa", "A frente do objeto (placa)"),
            ]),
        ),
        "Placa: fica preso como uma placa na parede, lido de frente pelo lado +Z do objeto (gire o objeto para virar).",
    ),
    help(
        f("outline", "Contorno", W::OptColor),
        "Uma borda em volta das letras: lê em qualquer fundo. Vazio = sem.",
    ),
    help(
        f("background", "Fundo", W::OptColor),
        "Um retângulo atrás do texto. Vazio = sem.",
    ),
    adv("font", "Fonte", W::Font),
];

const TERRAIN_FIELDS: &[F] = &[
    help(
        f(
            "width",
            "Largura",
            W::Num {
                speed: 0.5,
                min: 1.0,
                max: 4000.0,
                suffix: " m",
            },
        ),
        "De um lado ao outro (X). Mudar estica o que foi pintado.",
    ),
    help(
        f(
            "depth",
            "Fundo",
            W::Num {
                speed: 0.5,
                min: 1.0,
                max: 4000.0,
                suffix: " m",
            },
        ),
        "De trás para a frente (Z).",
    ),
    help(
        adv(
            "cells",
            "Detalhe",
            W::Int {
                min: 2.0,
                max: 256.0,
            },
        ),
        "Quadrados de cada lado: mais detalhe nos morros (e mais triângulos). 64 é um bom começo.",
    ),
    help(
        adv(
            "height",
            "Altura máxima",
            W::Num {
                speed: 0.5,
                min: 0.5,
                max: 1000.0,
                suffix: " m",
            },
        ),
        "O chão vai desta altura para baixo até esta para cima.",
    ),
    section("Cores (Pintar cor)"),
    f("color1", "Cor 1 (grama)", W::Color),
    f("color2", "Cor 2 (terra)", W::Color),
    f("color3", "Cor 3 (pedra)", W::Color),
    f("color4", "Cor 4 (areia)", W::Color),
    section("Água"),
    help(
        f("water", "Água", W::Bool),
        "Um plano de água por cima: o que estiver mais baixo vira lago ou mar. O Jogador 3D que cair nela recebe a mensagem \"agua\".",
    ),
    help(
        f(
            "water_level",
            "Nível da água",
            W::Num {
                speed: 0.05,
                min: -1000.0,
                max: 1000.0,
                suffix: " m",
            },
        ),
        "Em relação ao terreno (0 = a altura do chão plano).",
    ),
    f("water_color", "Cor da água", W::Color),
];

const BONE_FIELDS: &[F] = &[help(
    f("bone", "Osso", W::Bone),
    "O osso do modelo do pai em que o objeto fica preso (\"mao_d\" é a mão direita do herói). A posição, a rotação e a escala do objeto contam a partir do osso: ajuste com o mouse até a arma ficar certa na mão.",
)];

const ANIM3D_FIELDS: &[F] = &[
    help(
        f("animation", "Animação", W::Anim),
        "A animação que o modelo toca desde o começo (as do arquivo .glb). Vazia: parado na pose do modelo. Um Jogador 3D escolhe sozinho.",
    ),
    help(
        f(
            "speed",
            "Velocidade",
            W::Num {
                speed: 0.05,
                min: -10.0,
                max: 10.0,
                suffix: "×",
            },
        ),
        "1 = normal; 2 = o dobro; negativo toca ao contrário.",
    ),
    help(
        f("loop", "Repetir", W::Bool),
        "Desligado: toca uma vez e para no fim (a condição \"Animação terminar\" avisa).",
    ),
];

const BODY3D_FIELDS: &[F] = &[
    help(
        f(
            "body",
            "Tipo",
            W::Choice(&[
                ("static", "Parado"),
                ("dynamic", "Cai e é empurrado"),
                ("kinematic", "Se move sozinho"),
            ]),
        ),
        "Parado nunca sai do lugar (chão, paredes, cenário). \"Cai e é empurrado\" segue a física (caixas, bolas). \"Se move sozinho\" é movido pelo jogo (Plataforma que anda, portas) e leva junto quem está em cima.",
    ),
    help(
        f(
            "shape",
            "Forma",
            W::Choice(&[
                ("auto", "Automática"),
                ("box", "Caixa"),
                ("sphere", "Esfera"),
                ("capsule", "Cápsula"),
                ("cylinder", "Cilindro"),
                ("mesh", "Malha exata do modelo"),
            ]),
        ),
        "Automática: parado com um modelo usa a malha exata (dá para subir no telhado); o resto, uma caixa, esfera, cápsula ou cilindro do tamanho do objeto. A malha exata não serve para \"cai e é empurrado\".",
    ),
    help(
        f("sensor", "Só detecta (sensor)", W::Bool),
        "Não bloqueia: só avisa quando algo entra (moedas, portas, zonas).",
    ),
    help(
        f(
            "bounce",
            "Quique",
            W::Num {
                speed: 0.01,
                min: 0.0,
                max: 1.0,
                suffix: "",
            },
        ),
        "0 = para ao bater, 1 = quica sem perder velocidade.",
    ),
    help(
        adv(
            "friction",
            "Atrito",
            W::Num {
                speed: 0.01,
                min: 0.0,
                max: 2.0,
                suffix: "",
            },
        ),
        "0 = gelo; 0,5 = o normal.",
    ),
    help(
        adv(
            "mass",
            "Massa",
            W::Num {
                speed: 0.1,
                min: 0.0,
                max: 10000.0,
                suffix: " kg",
            },
        ),
        "0 = pelo tamanho. Mais pesado empurra mais e é menos empurrado.",
    ),
    help(
        adv("lock_rotation", "Não gira (fica em pé)", W::Bool),
        "Para coisas que caem mas não devem tombar.",
    ),
    help(
        adv(
            "gravity_scale",
            "Gravidade",
            W::Num {
                speed: 0.05,
                min: -10.0,
                max: 10.0,
                suffix: "×",
            },
        ),
        "Multiplica a gravidade da cena (0 = flutua).",
    ),
    help(
        adv("tag", "Rótulo", W::Text),
        "Para achar o corpo no código (padrão: o nome do objeto).",
    ),
];

/// O "Ambiente" de uma cena 3D: céu, sol, luz ambiente, névoa e sombras.
fn environment_ui(ui: &mut egui::Ui, doc: &mut SceneDoc) {
    use crate::scene_data::Prop;
    let r = egui::CollapsingHeader::new(RichText::new("☀ Ambiente").strong())
        .id_salt(("ambiente", &doc.rel))
        .default_open(true)
        .show(ui, |ui| {
            let color = |ui: &mut egui::Ui, doc: &mut SceneDoc, key: &str| {
                let mut c = hex_to_rgba(doc.data.env(key).str());
                if ui.color_edit_button_srgba_unmultiplied(&mut c).changed() {
                    doc.record(Some(&format!("ambiente:{key}")));
                    doc.data
                        .set_env(key, Prop::Str(rgba_to_hex([c[0], c[1], c[2], 255])));
                }
            };
            let check = |ui: &mut egui::Ui, doc: &mut SceneDoc, key: &str, text: &str| -> bool {
                let mut b = doc.data.env(key).num() != 0.0;
                if ui.checkbox(&mut b, text).changed() {
                    doc.record(None);
                    doc.data.set_env(key, Prop::Bool(b));
                }
                b
            };
            let num = |ui: &mut egui::Ui,
                       doc: &mut SceneDoc,
                       key: &str,
                       range: std::ops::RangeInclusive<f64>,
                       suffix: &str| {
                let mut v = doc.data.env(key).num();
                if ui
                    .add(egui::DragValue::new(&mut v).range(range).speed(0.5).suffix(suffix))
                    .changed()
                {
                    doc.record(Some(&format!("ambiente:{key}")));
                    doc.data.set_env(key, Prop::Num(v));
                }
            };
            // Os prontos: tudo de uma vez (céu, sol, luz ambiente, névoa).
            ui.horizontal_wrapped(|ui| {
                ui.label("Pronto:").on_hover_text(
                    "Troca o céu, o sol, a luz ambiente e a névoa de uma vez (Ctrl+Z desfaz). O Dia e noite, o Vento e o Halo ficam como estão. Na Noite e na Caverna, as Luzes (➕ Objeto > Luz) aparecem bem.",
                );
                for (id, name, values) in &crate::scene_data::schema().environment_presets {
                    let r = ui.small_button(name.as_str());
                    crate::tutorial::anchor_in(ui, &format!("inspetor:ambiente:{id}"), r.rect);
                    if r.clicked() {
                        doc.record(None);
                        // (O tempo, o vento e o halo ficam como estão.)
                        const KEEP: [&str; 6] =
                            ["cycle", "day_length", "start_hour", "wind", "wind_turn", "bloom"];
                        for (k, v) in values {
                            if !KEEP.contains(&k.as_str()) {
                                doc.data.set_env(k, v.clone());
                            }
                        }
                        // Sem céu (a caverna), o fundo também fica escuro.
                        if doc.data.env("sky").num() == 0.0 {
                            doc.data.background = "#0A0A0E".to_string();
                        }
                    }
                }
            });
            egui::Grid::new(("grade-ambiente", &doc.rel))
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Céu:")
                        .on_hover_text("Degradê atrás de tudo: em cima, no horizonte e embaixo. Sem céu, aparece o fundo.");
                    ui.horizontal(|ui| {
                        if check(ui, doc, "sky", "") {
                            color(ui, doc, "sky_top");
                            color(ui, doc, "sky_horizon");
                            color(ui, doc, "sky_bottom");
                        }
                    });
                    ui.end_row();

                    ui.label("Sol:").on_hover_text(
                        "Altura (90° = meio-dia, perto de 0° = fim de tarde, sombras compridas) e para que lado ele está.",
                    );
                    ui.horizontal(|ui| {
                        num(ui, doc, "sun_height", 1.0..=90.0, "° alto");
                        num(ui, doc, "sun_turn", -360.0..=360.0, "° lado");
                    });
                    ui.end_row();

                    ui.label("Luz do sol:");
                    ui.horizontal(|ui| {
                        color(ui, doc, "sun_color");
                        ui.weak("disco");
                        num(ui, doc, "sun_size", 0.0..=20.0, "°");
                    });
                    ui.end_row();

                    ui.label("Luz ambiente:").on_hover_text(
                        "A luz que chega onde o sol não bate: a de cima (céu) e a de baixo (chão).",
                    );
                    ui.horizontal(|ui| {
                        color(ui, doc, "ambient_sky");
                        color(ui, doc, "ambient_ground");
                    });
                    ui.end_row();

                    ui.label("Névoa:")
                        .on_hover_text("O que está longe some na cor do horizonte: começa em uma distância e cobre tudo na outra.");
                    ui.horizontal(|ui| {
                        if check(ui, doc, "fog", "") {
                            num(ui, doc, "fog_near", 0.0..=1000.0, " m");
                            num(ui, doc, "fog_far", 1.0..=2000.0, " m");
                        }
                    });
                    ui.end_row();

                    ui.label("Sombras:")
                        .on_hover_text("Sombra do sol até esta distância da câmera. Para um objeto não fazer sombra, desligue \"Sombra\" no componente dele.");
                    ui.horizontal(|ui| {
                        if check(ui, doc, "shadows", "") {
                            num(ui, doc, "shadow_distance", 5.0..=200.0, " m");
                        }
                    });
                    ui.end_row();

                    let r = ui.label("Vento:");
                    crate::tutorial::anchor_in(ui, "inspetor:ambiente:vento", r.rect);
                    r.on_hover_text(
                        "O que tem \"Balança com o vento\" (árvores, grama, bandeiras) balança, com rajadas, e as partículas são levadas. 0 = parado, 1 = brisa, 3 = ventania. O lado é para onde ele sopra (0° = para -Z, 90° = para +X).",
                    );
                    ui.horizontal(|ui| {
                        let mut v = doc.data.env("wind").num();
                        if ui
                            .add(egui::DragValue::new(&mut v).range(0.0..=5.0).speed(0.05))
                            .changed()
                        {
                            doc.record(Some("ambiente:wind"));
                            doc.data.set_env("wind", Prop::Num(v));
                        }
                        if v > 0.0 {
                            num(ui, doc, "wind_turn", -360.0..=360.0, "° lado");
                        }
                    });
                    ui.end_row();

                    let r = ui.label("Halo do brilho:");
                    crate::tutorial::anchor_in(ui, "inspetor:ambiente:halo", r.rect);
                    r.on_hover_text(
                        "O que brilha (Brilho, a chama das tochas, o fogo, o sol e a lua) ganha um halo suave em volta. 0 = sem halo, 1 = o normal.",
                    );
                    ui.horizontal(|ui| {
                        let mut v = doc.data.env("bloom").num();
                        if ui
                            .add(egui::DragValue::new(&mut v).range(0.0..=3.0).speed(0.02))
                            .changed()
                        {
                            doc.record(Some("ambiente:bloom"));
                            doc.data.set_env("bloom", Prop::Num(v));
                        }
                    });
                    ui.end_row();

                    let r = ui.label("Dia e noite:").on_hover_text(
                        "O sol anda no céu e as cores mudam com a hora: amanhecer, dia, entardecer e noite (com a lua e as estrelas). O dia é o desta cena. A vista mostra a hora em que o jogo começa: mude para ver a cena de noite.",
                    );
                    crate::tutorial::anchor_in(ui, "inspetor:ambiente:ciclo", r.rect);
                    ui.horizontal(|ui| {
                        if check(ui, doc, "cycle", "") {
                            ui.weak("um dia dura");
                            let mut v = doc.data.env("day_length").num();
                            if ui
                                .add(
                                    egui::DragValue::new(&mut v)
                                        .range(0.1..=240.0)
                                        .speed(0.1)
                                        .suffix(" min"),
                                )
                                .changed()
                            {
                                doc.record(Some("ambiente:day_length"));
                                doc.data.set_env("day_length", Prop::Num(v));
                            }
                        }
                    });
                    ui.end_row();
                    if doc.data.env("cycle").bool() {
                        ui.label("Começa às:").on_hover_text(
                            "A hora em que o jogo começa (6 = o sol nascendo, 12 = meio-dia, 18 = o pôr do sol, 22 = noite). A vista da cena mostra essa hora.",
                        );
                        ui.horizontal(|ui| {
                            let mut v = doc.data.env("start_hour").num();
                            let r = ui.add(
                                egui::Slider::new(&mut v, 0.0..=23.99)
                                    .step_by(0.25)
                                    .custom_formatter(|n, _| {
                                        format!("{:02}:{:02}", n.floor() as u32, ((n % 1.0) * 60.0).round() as u32)
                                    }),
                            );
                            crate::tutorial::anchor_in(ui, "inspetor:ambiente:hora", r.rect);
                            if r.changed() {
                                doc.record(Some("ambiente:start_hour"));
                                doc.data.set_env("start_hour", Prop::Num(v));
                            }
                        });
                        ui.end_row();
                    }
                });
        });
    crate::tutorial::anchor_in(ui, "inspetor:ambiente", r.header_response.rect);
}

/// Posição, rotação e escala de um objeto numa cena 3D.
fn transform3_ui(
    ui: &mut egui::Ui,
    doc: &mut SceneDoc,
    id: u32,
    o: &crate::scene_data::Object,
    differs: [bool; 4],
) {
    egui::CollapsingHeader::new(RichText::new("Transformação").strong())
        .id_salt(("transformacao3", id))
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new(("grade-transformacao3", id))
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    let mut t = [
                        o.x, o.y, o.z, o.rx, o.ry, o.rz, o.scale_x, o.scale_y, o.scale_z,
                    ];
                    let row = |ui: &mut egui::Ui,
                               label: &str,
                               tip: &str,
                               v: &mut [f64],
                               speed: f64,
                               suffix: &str,
                               differs: bool| {
                        transform_label(ui, label, tip, differs);
                        ui.horizontal(|ui| {
                            for (k, axis) in ["x ", "y ", "z "].into_iter().enumerate() {
                                ui.add(
                                    egui::DragValue::new(&mut v[k])
                                        .speed(speed)
                                        .max_decimals(3)
                                        .prefix(axis)
                                        .suffix(suffix),
                                );
                            }
                        });
                        ui.end_row();
                    };
                    row(
                        ui,
                        "Posição",
                        "Em metros. Y é a altura.",
                        &mut t[0..3],
                        0.05,
                        "",
                        differs[0],
                    );
                    row(
                        ui,
                        "Rotação",
                        "Graus em volta de cada eixo: Y vira para os lados, X inclina, Z rola.",
                        &mut t[3..6],
                        1.0,
                        "°",
                        differs[1],
                    );
                    row(
                        ui,
                        "Escala",
                        "1 = o tamanho do modelo.",
                        &mut t[6..9],
                        0.01,
                        "",
                        differs[2],
                    );
                    let now = doc.data.find(id).map(|x| {
                        [
                            x.x, x.y, x.z, x.rx, x.ry, x.rz, x.scale_x, x.scale_y, x.scale_z,
                        ]
                    });
                    if Some(t) != now {
                        doc.record(Some(&format!("{id}:transformacao")));
                        if let Some(obj) = doc.data.find_mut(id) {
                            [
                                obj.x,
                                obj.y,
                                obj.z,
                                obj.rx,
                                obj.ry,
                                obj.rz,
                                obj.scale_x,
                                obj.scale_y,
                                obj.scale_z,
                            ] = t;
                        }
                    }
                });
        });
}

/// Componentes que só fazem sentido numa cena 3D.
pub fn is_3d_kind(kind: &str) -> bool {
    matches!(
        kind,
        "model"
            | "shape3d"
            | "light3d"
            | "camera3d"
            | "body3d"
            | "anim3d"
            | "terrain"
            | "text3d"
            | "bone"
    )
}

/// Componentes de cena 2D que não valem numa cena 3D (física 2D, mapa de
/// tiles, câmera 2D, partículas 2D).
pub fn is_2d_only_kind(kind: &str) -> bool {
    matches!(kind, "body" | "map" | "camera" | "sprite" | "shape")
}

fn fields(kind: &str) -> &'static [F] {
    match kind {
        "model" => MODEL_FIELDS,
        "shape3d" => SHAPE3D_FIELDS,
        "light3d" => LIGHT3D_FIELDS,
        "camera3d" => CAMERA3D_FIELDS,
        "body3d" => BODY3D_FIELDS,
        "anim3d" => ANIM3D_FIELDS,
        "bone" => BONE_FIELDS,
        "terrain" => TERRAIN_FIELDS,
        "text3d" => TEXT3D_FIELDS,
        "sprite" => SPRITE_FIELDS,
        "text" => TEXT_FIELDS,
        "shape" => SHAPE_FIELDS,
        "body" => BODY_FIELDS,
        "map" => MAP_FIELDS,
        "camera" => CAMERA_FIELDS,
        "sound" => SOUND_FIELDS,
        "particles" => PARTICLES_FIELDS,
        "panel" => PANEL_FIELDS,
        "button" => BUTTON_FIELDS,
        "bar" => BAR_FIELDS,
        "icons" => ICONS_FIELDS,
        "slider" => SLIDER_FIELDS,
        "checkbox" => CHECKBOX_FIELDS,
        "textfield" => TEXTFIELD_FIELDS,
        "layout" => LAYOUT_FIELDS,
        _ => &[],
    }
}

/// Campos de interface que não valem com as outras escolhas (a cena de um
/// botão que não troca de cena, a variável de um deslizador de volume...).
fn hidden_ui_field(c: &Component, key: &str) -> bool {
    match (c.kind.as_str(), key) {
        ("button", "scene") => !matches!(c.str("action"), "cena" | "por_cima"),
        ("slider" | "checkbox", "var") => c.str("target") != "variavel",
        ("slider", "value") | ("checkbox", "checked") => {
            c.str("target") != "variavel" || !c.str("var").is_empty()
        }
        ("textfield", "text") => !c.str("var").is_empty(),
        ("bar", "value") => !c.str("var").is_empty(),
        ("icons", "count") => !c.str("var").is_empty(),
        ("icons", "frame") => c.num("frame_w") <= 0.0,
        ("panel" | "button", "slice" | "slice_scale") => c.str("image").is_empty(),
        ("layout", "columns") => c.str("arrange") != "grid",
        ("particles", "rate" | "emitting") => c.str("mode") == "explosao",
        ("particles", "amount" | "on_start" | "interval") => c.str("mode") != "explosao",
        ("particles", "area_w" | "area_h") => c.str("area") != "retangulo",
        ("particles", "area_radius") => c.str("area") != "circulo",
        ("particles", "frame_w" | "frame_h") => c.str("image").is_empty(),
        ("particles", "frame") => c.str("image").is_empty() || c.num("frame_w") <= 0.0,
        ("layout", "spacing" | "align") => c.str("arrange") == "none",
        _ => false,
    }
}

/// Nome e descrição de cada componente.
pub fn component_info(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "sprite" => (
            "Imagem (sprite)",
            "Mostra uma imagem ou um quadro de spritesheet.",
        ),
        "text" => ("Texto", "Escreve um texto no mundo."),
        "shape" => ("Forma", "Retângulo ou círculo de uma cor."),
        "body" => (
            "Corpo físico",
            "Cai, colide, para no chão. Mexa nele pelo script: objeto.body.vx.",
        ),
        "map" => (
            "Mapa de tiles",
            "Desenha um mapa .tmj (do editor ou do Tiled), com a colisão dos tiles.",
        ),
        "camera" => ("Câmera", "Onde a câmera começa, ou quem ela segue."),
        "sound" => ("Som", "Toca um som ou música quando a cena começa."),
        "particles" => ("Partículas", "Fumaça, faíscas, poeira, chuva..."),
        "panel" => (
            "Painel",
            "Fundo de menu ou de HUD: cantos arredondados, borda ou uma moldura.",
        ),
        "button" => (
            "Botão",
            "Aperta com o mouse, o teclado ou o controle. Troca de cena, abre por cima, fecha...",
        ),
        "bar" => (
            "Barra",
            "Vida, energia, tempo: mostra uma variável do jogo.",
        ),
        "icons" => (
            "Ícones",
            "Corações, vidas, moedas: um ícone para cada ponto da variável.",
        ),
        "slider" => (
            "Deslizador",
            "Escolhe um valor arrastando: volume, dificuldade...",
        ),
        "checkbox" => (
            "Caixa de marcar",
            "Liga e desliga: tela cheia, uma opção do jogo.",
        ),
        "textfield" => (
            "Campo de texto",
            "Para digitar: o nome do jogador, um código.",
        ),
        "layout" => (
            "Grupo (arrumar)",
            "Arruma os filhos sozinho, em coluna, em linha ou em grade.",
        ),
        "model" => (
            "Modelo 3D",
            "Um modelo .glb ou .gltf (do Blender e outros).",
        ),
        "shape3d" => (
            "Forma 3D",
            "Cubo, esfera, cilindro, cone, cápsula ou plano, de uma cor.",
        ),
        "light3d" => (
            "Luz",
            "Uma lâmpada, uma tocha ou um holofote: ilumina o que está em volta (as luzes não fazem sombra; só o sol).",
        ),
        "camera3d" => (
            "Câmera 3D",
            "A câmera do jogo: ele mostra o que ela vê, de onde ela está.",
        ),
        "terrain" => (
            "Terreno",
            "Um chão com morros, vales, cores e água, pintado com o ✏ Pintar terreno da vista 3D.",
        ),
        "text3d" => (
            "Texto 3D",
            "Escreve no mundo 3D: placas, nomes em cima dos personagens, avisos.",
        ),
        "anim3d" => (
            "Animação 3D",
            "Toca uma animação do modelo 3D (andar, acenar...), com velocidade e repetir.",
        ),
        "bone" => (
            "Preso a um osso",
            "Para segurar: uma espada ou um cajado na mão do personagem, que acompanha a animação. Ponha o objeto como filho do personagem.",
        ),
        "body3d" => (
            "Corpo 3D",
            "Física 3D: chão e paredes que bloqueiam, caixas que caem, plataformas que andam.",
        ),
        "behavior" => (
            "Comportamento",
            "Um script com propriedades: pronto (andar, pular, coletar...) ou do jogo.",
        ),
        _ => ("Componente", ""),
    }
}

/// O que o inspetor pede ao espaço de trabalho.
#[derive(Debug, Clone, PartialEq)]
pub enum InspectorAction {
    OpenFile(String),
    /// Criar o script da cena.
    CreateScript,
    /// Criar um script de comportamento novo e pôr neste objeto.
    NewBehaviorScript(u32),
    /// Mostrar o painel Eventos.
    ShowEvents,
    /// Criar uma imagem nova (pixel art) para o componente `ci` do objeto.
    NewImage {
        id: u32,
        ci: usize,
        name: String,
    },
}

/// Resumo das regras de eventos, com o botão para editar. Devolve `true` se
/// pediu para abrir o painel Eventos.
fn events_summary(ui: &mut egui::Ui, rules: &[crate::scene_data::Rule], in_object: bool) -> bool {
    ui.add_space(8.0);
    let mut open = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Eventos").strong());
        ui.weak(match rules.len() {
            0 => "nenhuma regra".to_string(),
            1 => "1 regra".to_string(),
            n => format!("{n} regras"),
        });
        open = ui
            .button(if rules.is_empty() {
                "➕ Criar"
            } else {
                "Editar"
            })
            .on_hover_text("Regras \"Quando... Então...\": abre o painel Eventos")
            .clicked();
    });
    for r in rules.iter().take(4) {
        let text = crate::events::rule_summary(r, in_object);
        let short: String = if text.chars().count() > 70 {
            text.chars().take(69).chain(std::iter::once('…')).collect()
        } else {
            text.clone()
        };
        ui.add(egui::Label::new(RichText::new(short).small().weak()).truncate())
            .on_hover_text(text);
    }
    if rules.len() > 4 {
        ui.weak(format!("... e mais {}", rules.len() - 4));
    }
    open
}

#[derive(Default)]
pub struct Inspector {
    images: HashMap<String, (Option<SystemTime>, egui::TextureHandle)>,
    /// Informações dos comportamentos (lidas dos scripts).
    pub catalog: Catalog,
    /// Pedidos feitos enquanto desenha.
    actions_out: Vec<InspectorAction>,
    /// O jogo aberto (se mostra a cena deste inspetor).
    game: Option<GameState>,
    /// Componente novo: no próximo quadro, rola até o fim para mostrá-lo.
    scroll_to_end: bool,
    /// Vários objetos selecionados: o que eles têm em comum e o que difere.
    multi: Option<Multi>,
}

/// Vários objetos no Inspetor: ele mostra o último selecionado (o "líder"),
/// só com os componentes que todos têm, e o que muda nele muda em todos.
#[derive(Debug, Default, Clone)]
struct Multi {
    /// Os componentes do líder que todos têm (pelo índice).
    common: Vec<bool>,
    /// Campos com valores diferentes entre eles: (componente, chave).
    differs: std::collections::HashSet<(usize, String)>,
    /// Posição, rotação, escala e camada diferentes.
    pos: bool,
    rot: bool,
    scale: bool,
    layer: bool,
}

/// O que identifica um componente para comparar objetos: o tipo e, num
/// comportamento, o script.
fn comp_key(c: &Component) -> (String, String) {
    let script = if c.is_behavior() {
        c.str("script").to_string()
    } else {
        String::new()
    };
    (c.kind.clone(), script)
}

/// O componente de `o` que corresponde ao componente `ci` de `lead` (o mesmo
/// tipo, na mesma ordem entre os do mesmo tipo).
fn matching(o: &Object, lead: &Object, ci: usize) -> Option<usize> {
    let key = comp_key(lead.components.get(ci)?);
    let k = lead.components[..ci]
        .iter()
        .filter(|c| comp_key(c) == key)
        .count();
    o.components
        .iter()
        .enumerate()
        .filter(|(_, c)| comp_key(c) == key)
        .nth(k)
        .map(|(i, _)| i)
}

impl Multi {
    fn of(lead: &Object, others: &[Object], three_d: bool) -> Self {
        let mut m = Multi::default();
        for (ci, c) in lead.components.iter().enumerate() {
            let matched: Vec<Option<&Component>> = others
                .iter()
                .map(|o| matching(o, lead, ci).map(|j| &o.components[j]))
                .collect();
            let common = matched.iter().all(Option::is_some);
            m.common.push(common);
            if !common {
                continue;
            }
            for other in matched.into_iter().flatten() {
                let keys = c
                    .props
                    .iter()
                    .map(|(k, _)| k)
                    .chain(other.props.iter().map(|(k, _)| k));
                for k in keys {
                    let same = match (c.get(k), other.get(k)) {
                        (Some(a), Some(b)) => same_value(a, b),
                        (None, None) => true,
                        _ => false,
                    };
                    if !same {
                        m.differs.insert((ci, k.clone()));
                    }
                }
            }
        }
        for o in others {
            if three_d {
                m.pos |= (o.x, o.y, o.z) != (lead.x, lead.y, lead.z);
                m.rot |= (o.rx, o.ry, o.rz) != (lead.rx, lead.ry, lead.rz);
                m.scale |=
                    (o.scale_x, o.scale_y, o.scale_z) != (lead.scale_x, lead.scale_y, lead.scale_z);
            } else {
                m.pos |= (o.x, o.y) != (lead.x, lead.y);
                m.rot |= o.rotation != lead.rotation;
                m.scale |= (o.scale_x, o.scale_y) != (lead.scale_x, lead.scale_y);
            }
            m.layer |= o.layer != lead.layer;
        }
        m
    }

    fn differs(&self, ci: usize, key: &str) -> bool {
        self.differs.contains(&(ci, key.to_string()))
    }
}

/// Leva para os outros objetos o que mudou no líder (`before` → agora): os
/// campos da transformação que mudaram, os valores mudados dos componentes
/// em comum, componentes novos e componentes tirados.
fn propagate(doc: &mut SceneDoc, before: &Object, others: &[u32]) {
    let Some(after) = doc.data.find(before.id).cloned() else {
        return;
    };
    if &after == before {
        return;
    }
    for &oid in others {
        let Some(other) = doc.data.find(oid).cloned() else {
            continue;
        };
        let mut o2 = other.clone();
        macro_rules! field {
            ($($f:ident),*) => {
                $( if before.$f != after.$f { o2.$f = after.$f.clone(); } )*
            };
        }
        field!(
            x, y, z, rotation, rx, ry, rz, scale_x, scale_y, scale_z, visible, locked, layer
        );
        let (nb, na) = (before.components.len(), after.components.len());
        if na == nb {
            for ci in 0..na {
                let (b, a) = (&before.components[ci], &after.components[ci]);
                if b == a || comp_key(b) != comp_key(a) {
                    continue;
                }
                let Some(j) = matching(&other, before, ci) else {
                    continue;
                };
                let keys: Vec<String> = b
                    .props
                    .iter()
                    .chain(a.props.iter())
                    .map(|(k, _)| k.clone())
                    .collect();
                for k in keys {
                    if b.get(&k) != a.get(&k) {
                        match a.get(&k) {
                            Some(v) => o2.components[j].set(&k, v.clone()),
                            None => o2.components[j].remove(&k),
                        }
                    }
                }
            }
        } else if na > nb {
            // Componentes novos (vão para o fim).
            for c in &after.components[nb..] {
                o2.components.push(c.clone());
            }
        } else {
            // Um componente saiu: o primeiro que não bate mais.
            let ci = (0..nb)
                .find(|&i| {
                    i >= na || comp_key(&before.components[i]) != comp_key(&after.components[i])
                })
                .unwrap_or(nb - 1);
            if let Some(j) = matching(&other, before, ci) {
                o2.components.remove(j);
            }
        }
        if o2 != other
            && let Some(slot) = doc.data.find_mut(oid)
        {
            *slot = o2;
        }
    }
}

/// O componente copiado (☰ > Copiar componente).
fn copied_component_id() -> egui::Id {
    egui::Id::new("componente-copiado")
}

fn copied_component(ctx: &egui::Context) -> Option<Component> {
    ctx.data(|d| d.get_temp::<Component>(copied_component_id()))
}

/// O rótulo de uma linha da Transformação, marcado quando os objetos
/// selecionados têm valores diferentes.
fn transform_label(ui: &mut egui::Ui, text: &str, tip: &str, differs: bool) -> egui::Response {
    if differs {
        ui.label(RichText::new(format!("{text} ≠")).color(crate::theme::WARNING))
            .on_hover_text(format!("{tip}\n\nOs objetos têm valores diferentes: o que você mudar aqui fica igual em todos."))
    } else {
        let r = ui.label(text);
        if tip.is_empty() {
            r
        } else {
            r.on_hover_text(tip)
        }
    }
}

/// Cor do que vem do jogo aberto.
pub const GAME: Color32 = Color32::from_rgb(0x5A, 0xD8, 0xE8);

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Valor como texto curto.
fn prop_text(p: &Prop) -> String {
    match p {
        Prop::Num(n) => crate::scene_data::format_num(round2(*n)),
        Prop::Bool(b) => if *b { "sim" } else { "não" }.to_string(),
        Prop::Str(s) => format!("\"{s}\""),
    }
}

/// Mesmo valor (números com uma folga pequena)?
fn same_value(a: &Prop, b: &Prop) -> bool {
    match (a, b) {
        (Prop::Num(x), Prop::Num(y)) => (x - y).abs() < 1e-6,
        _ => a == b,
    }
}

/// Cor das instâncias de prefab.
pub const PREFAB: Color32 = Color32::from_rgb(0x6A, 0xB0, 0xF5);

/// Um `.lua` que pode ser um comportamento (não é cena nem configuração).
fn is_behavior_candidate(rel: &str) -> bool {
    rel != "config.lua" && rel != "main.lua" && !rel.starts_with("cenas/")
}

/// Rótulo de um campo (em negrito e com ↺ quando a instância mudou o valor
/// do prefab).
fn field_row(
    ui: &mut egui::Ui,
    spec: &Spec,
    overridden: bool,
    differs: bool,
    reverts: &mut Vec<String>,
) -> bool {
    ui.horizontal(|ui| {
        let text = if differs {
            RichText::new(format!("{} ≠", spec.label)).color(crate::theme::WARNING)
        } else if overridden {
            RichText::new(&spec.label).strong().color(PREFAB)
        } else {
            RichText::new(&spec.label)
        };
        let mut r = ui.label(text);
        if !spec.help.is_empty() {
            r = r.on_hover_text(&spec.help);
        }
        if differs {
            r.on_hover_text(
                "Os objetos têm valores diferentes: o que você mudar aqui fica igual em todos.",
            );
        }
        if overridden
            && ui
                .small_button("↺")
                .on_hover_text("Voltar ao valor do prefab")
                .clicked()
        {
            reverts.push(spec.key.clone());
        }
    });
    true
}

/// Aplica as mudanças de um componente (um passo de desfazer).
fn apply_changes(
    doc: &mut SceneDoc,
    o: &Object,
    ci: usize,
    changes: Vec<(String, Prop)>,
    reverts: Vec<String>,
) {
    if changes.is_empty() && reverts.is_empty() {
        return;
    }
    let first = changes
        .first()
        .map(|c| c.0.clone())
        .or_else(|| reverts.first().cloned())
        .unwrap_or_default();
    let key = format!("{}:{ci}:{first}", o.id);
    doc.record(if reverts.is_empty() { Some(&key) } else { None });
    let base: Option<Component> = o
        .prefab
        .as_ref()
        .and_then(|l| l.base.as_ref())
        .and_then(|b| b.components.get(ci).cloned());
    if let Some(c) = doc
        .data
        .find_mut(o.id)
        .and_then(|x| x.components.get_mut(ci))
    {
        for (k, v) in changes {
            // Terreno: mais quadrados ou outra altura máxima refazem os dados
            // (senão o que foi pintado se perde ou muda de altura).
            if c.kind == "terrain" && (k == "cells" || k == "height") {
                let old = crate::terrain_tool::terrain_of(c);
                c.set(&k, v);
                let new = if k == "cells" {
                    old.resample(c.num("cells").clamp(2.0, 256.0) as u32)
                } else {
                    let mut t = old;
                    t.max_height = (c.num("height") as f32).max(0.1);
                    t
                };
                c.set("data", Prop::Str(new.encode()));
                continue;
            }
            c.set(&k, v);
        }
        for k in reverts {
            match base.as_ref().and_then(|b| b.get(&k)) {
                Some(v) => c.set(&k, v.clone()),
                None => c.remove(&k),
            }
        }
    }
}

/// Um campo pronto para mostrar: do componente (fixo) ou de um
/// comportamento (vem do script).
#[derive(Debug, Clone)]
struct Spec {
    key: String,
    label: String,
    w: Wd,
    help: String,
}

#[derive(Debug, Clone)]
enum Wd {
    Num {
        speed: f64,
        min: f64,
        max: f64,
        suffix: &'static str,
    },
    Int {
        min: f64,
        max: f64,
    },
    Bool,
    Text,
    Multiline,
    Color,
    Choice(Vec<(String, String)>),
    File(FileKind),
    /// Cena (.cena ou .lua).
    Scene,
    /// Objeto da cena; o texto é o que mostrar quando vazio.
    Object(&'static str),
    /// Animação da imagem do objeto (ou, num comportamento, também uma
    /// lista de quadros).
    Anim,
    /// Um osso de um modelo 3D; o texto é o que mostrar quando vazio.
    Bone(String),
    /// Cor que, vazia, vem do tema da interface.
    ThemeColor(&'static str),
    /// Variável do jogo.
    Var,
    /// Fonte (.ttf ou .otf).
    Font,
    /// Cor opcional (vazia = sem).
    OptColor,
}

impl From<&F> for Spec {
    fn from(f: &F) -> Self {
        let w = match f.w {
            W::Num {
                speed,
                min,
                max,
                suffix,
            } => Wd::Num {
                speed,
                min,
                max,
                suffix,
            },
            W::Int { min, max } => Wd::Int { min, max },
            W::Bool => Wd::Bool,
            W::Text => Wd::Text,
            W::Multiline => Wd::Multiline,
            W::Color => Wd::Color,
            W::Choice(list) => Wd::Choice(
                list.iter()
                    .map(|(k, l)| (k.to_string(), l.to_string()))
                    .collect(),
            ),
            W::File(k) => Wd::File(k),
            W::Object => Wd::Object("(nenhum)"),
            W::Anim => Wd::Anim,
            W::Bone => Wd::Bone("(nenhum)".into()),
            W::ThemeColor(k) => Wd::ThemeColor(k),
            W::Var => Wd::Var,
            W::Font => Wd::Font,
            W::Scene => Wd::Scene,
            W::OptColor => Wd::OptColor,
        };
        Spec {
            key: f.key.to_string(),
            label: f.label.to_string(),
            w,
            help: f.help.to_string(),
        }
    }
}

/// As partículas de uma cena 3D: em metros (o arrastar anda devagar) e com
/// as dicas do 3D.
fn particles3d_spec(spec: &mut Spec) {
    if let Wd::Num { speed, suffix, .. } = &mut spec.w
        && !matches!(
            spec.key.as_str(),
            "rate" | "spread" | "spin" | "interval" | "lifetime"
        )
    {
        *speed = 0.02;
        if matches!(spec.key.as_str(), "speed_min" | "speed_max") {
            *suffix = " m/s";
        } else if matches!(
            spec.key.as_str(),
            "size_start" | "size_end" | "area_w" | "area_h" | "area_radius"
        ) {
            *suffix = " m";
        }
    }
    match spec.key.as_str() {
        "spread" => {
            spec.help = "Um cone em volta do \"cima\" do objeto: 0 = um raio só, 360 = todos os lados. Gire o objeto para apontar.".into();
        }
        "gravity" => {
            spec.help = "Em m/s²: negativa = caem; positiva = sobem (fumaça).".into();
        }
        "area" => {
            spec.help = "Deitada no chão: um retângulo (largura × fundo) ou um círculo. Chuva e neve: um retângulo largo no alto.".into();
        }
        "area_h" => spec.label = "Fundo da área".into(),
        _ => {}
    }
}

/// O campo de uma propriedade de comportamento.
fn behavior_spec(info: &BehaviorInfo, p: &behaviors::PropSpec) -> Spec {
    let w = match p.kind {
        PropType::Number if p.integer => Wd::Int {
            min: p.min.unwrap_or(-1_000_000.0),
            max: p.max.unwrap_or(1_000_000.0),
        },
        PropType::Number => {
            let range = p.max.unwrap_or(1000.0) - p.min.unwrap_or(0.0);
            let base = p.default.num().abs();
            Wd::Num {
                speed: if base < 5.0 && range <= 20.0 {
                    0.01
                } else {
                    1.0
                },
                min: p.min.unwrap_or(-1_000_000.0),
                max: p.max.unwrap_or(1_000_000.0),
                suffix: "",
            }
        }
        PropType::Bool => Wd::Bool,
        PropType::Text => Wd::Text,
        PropType::Color => Wd::Color,
        PropType::Choice => Wd::Choice(p.options.clone()),
        PropType::Object => Wd::Object(if info.is_builtin() {
            "(o jogador)"
        } else {
            "(nenhum)"
        }),
        PropType::Scene => Wd::Scene,
        PropType::Prefab => Wd::File(FileKind::Prefab),
        PropType::Sound => Wd::File(FileKind::Sound),
        PropType::Image => Wd::File(FileKind::Image),
        PropType::Dialogue => Wd::File(FileKind::Dialogue),
        PropType::Anim => Wd::Anim,
        PropType::Bone => Wd::Bone(p.allow_empty.clone().unwrap_or_else(|| "(nenhum)".into())),
        PropType::Tile => Wd::Int {
            min: 0.0,
            max: 1_000_000.0,
        },
        PropType::Var => Wd::Var,
        PropType::Target | PropType::Value | PropType::Behavior | PropType::TileType => Wd::Text,
    };
    Spec {
        key: p.key.clone(),
        label: p.label.clone(),
        w,
        help: p.help.clone(),
    }
}

impl std::fmt::Debug for Inspector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inspector").finish_non_exhaustive()
    }
}

/// Cor "#RRGGBB(AA)" ↔ bytes.
fn hex_to_rgba(s: &str) -> [u8; 4] {
    let c = crate::scene_render::egui_color(s);
    c.to_srgba_unmultiplied()
}

fn rgba_to_hex(c: [u8; 4]) -> String {
    if c[3] == 255 {
        format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
    } else {
        format!("#{:02X}{:02X}{:02X}{:02X}", c[0], c[1], c[2], c[3])
    }
}

impl Inspector {
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        doc: Option<&mut SceneDoc>,
        files: &FileTree,
        sizes: &mut crate::scene_render::Sizes,
        mode: Mode,
        game: Option<&GameState>,
    ) -> Vec<InspectorAction> {
        let mut actions = Vec::new();
        self.game = game.cloned();
        let Some(doc) = doc else {
            ui.add_space(8.0);
            ui.weak("Selecione um objeto numa cena visual para ver as propriedades dele aqui.");
            return actions;
        };
        if doc.error.is_some() {
            return actions;
        }
        crate::tutorial::anchor(ui.ctx(), "inspetor", ui.max_rect());
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match doc.selection.len() {
                0 => self.scene_ui(ui, doc, files, &mut actions),
                1 => {
                    let id = doc.selection[0];
                    if doc.data.find(id).is_some() {
                        self.object_ui(ui, doc, id, files, sizes, mode);
                    }
                }
                n => {
                    let lead = *doc.selection.last().expect("seleção");
                    let mut others: Vec<u32> = Vec::new();
                    for id in &doc.selection {
                        if *id != lead && !others.contains(id) {
                            others.push(*id);
                        }
                    }
                    let Some(before) = doc.data.find(lead).cloned() else {
                        return;
                    };
                    let objs: Vec<Object> = others
                        .iter()
                        .filter_map(|id| doc.data.find(*id).cloned())
                        .collect();
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(format!("{n} objetos")).strong());
                            ui.weak(format!("(mostrando {})", before.name));
                        });
                        ui.weak("O que você mudar aqui muda em todos. ≠ marca valores diferentes entre eles; só aparecem os componentes que todos têm.");
                        ui.horizontal(|ui| {
                            if ui.button("Duplicar").clicked() {
                                doc.duplicate_selection();
                            }
                            if ui.button("🗑 Excluir").clicked() {
                                doc.delete_selection();
                            }
                        });
                    });
                    if doc.selection.len() < 2 || doc.data.find(lead).is_none() {
                        return;
                    }
                    ui.add_space(4.0);
                    self.multi = Some(Multi::of(&before, &objs, doc.data.three_d));
                    self.object_ui(ui, doc, lead, files, sizes, mode);
                    self.multi = None;
                    propagate(doc, &before, &others);
                }
            });
        actions.append(&mut self.actions_out);
        actions
    }

    fn scene_ui(
        &mut self,
        ui: &mut egui::Ui,
        doc: &mut SceneDoc,
        files: &FileTree,
        actions: &mut Vec<InspectorAction>,
    ) {
        crate::events::set_three_d(doc.data.three_d);
        ui.label(RichText::new("Cena").size(17.0).strong());
        ui.weak(format!("{} · {} objetos", doc.rel, doc.data.ids().len()));
        ui.add_space(6.0);
        egui::Grid::new(("grade-cena", &doc.rel))
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                ui.label("Script:")
                    .on_hover_text("Código Lua da cena: recebe os objetos em self.objects.");
                ui.horizontal(|ui| {
                    let mut script = doc.data.script.clone();
                    egui::ComboBox::from_id_salt(("script", &doc.rel))
                        .width((ui.available_width() - 64.0).clamp(70.0, 180.0))
                        .truncate()
                        .selected_text(if script.is_empty() {
                            "(nenhum)".to_string()
                        } else {
                            script.clone()
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut script, String::new(), "(nenhum)");
                            for e in files
                                .files_of(FileKind::Lua)
                                .filter(|e| e.rel != "config.lua")
                            {
                                ui.selectable_value(&mut script, e.rel.clone(), &e.rel);
                            }
                        });
                    if script != doc.data.script {
                        doc.record(None);
                        doc.data.script = script;
                    }
                    if doc.data.script.is_empty() {
                        if ui
                            .button("Criar")
                            .on_hover_text("Cria um script para esta cena")
                            .clicked()
                        {
                            actions.push(InspectorAction::CreateScript);
                        }
                    } else if ui.button("Abrir").clicked() {
                        actions.push(InspectorAction::OpenFile(doc.data.script.clone()));
                    }
                });
                ui.end_row();

                ui.label("Fundo:");
                ui.horizontal(|ui| {
                    let mut own = !doc.data.background.is_empty();
                    if ui
                        .checkbox(&mut own, "Cor própria")
                        .on_hover_text("Sem cor própria, vale a do projeto (config.lua).")
                        .changed()
                    {
                        doc.record(None);
                        doc.data.background = match (own, doc.data.overlay) {
                            (false, _) => String::new(),
                            (true, true) => "#00000099".into(),
                            (true, false) => "#14202E".into(),
                        };
                    }
                    if own {
                        let mut c = hex_to_rgba(&doc.data.background);
                        if ui.color_edit_button_srgba_unmultiplied(&mut c).changed() {
                            doc.record(Some("fundo"));
                            // Por cima, o fundo é um véu: pode ser transparente.
                            let a = if doc.data.overlay { c[3] } else { 255 };
                            doc.data.background = rgba_to_hex([c[0], c[1], c[2], a]);
                        }
                    }
                });
                ui.end_row();

                ui.label("Gravidade:")
                    .on_hover_text("Para corpos físicos. Negativa = cai.");
                let mut g = doc.data.gravity;
                if ui.add(egui::DragValue::new(&mut g).speed(10.0)).changed() {
                    doc.record(Some("gravidade"));
                    doc.data.gravity = g;
                }
                ui.end_row();

                if doc.data.three_d {
                    return;
                }
                ui.label("Por cima:");
                let mut o = doc.data.overlay;
                let r = ui
                    .checkbox(&mut o, "Pausa ou menu")
                    .on_hover_text("Abre por cima de outra cena (pausa, opções). A cena de baixo continua aparecendo, parada, até esta fechar. Os objetos ficam presos à tela e o fundo vira um véu (use uma cor transparente).");
                crate::tutorial::anchor_in(ui, "inspetor:por-cima", r.rect);
                if r.changed() {
                    doc.record(None);
                    doc.data.overlay = o;
                    // O véu escuro por cima da cena de baixo.
                    if o && (doc.data.background.is_empty() || hex_to_rgba(&doc.data.background)[3] == 255) {
                        doc.data.background = "#00000099".into();
                    }
                }
                ui.end_row();
            });
        if doc.data.three_d {
            ui.add_space(6.0);
            environment_ui(ui, doc);
        }
        if !doc.is_prefab() && events_summary(ui, &doc.data.events, false) {
            self.actions_out.push(InspectorAction::ShowEvents);
        }
        if let Some(game) = &self.game {
            ui.add_space(10.0);
            ui.label(RichText::new("No jogo agora").strong().color(GAME));
            if game.vars.is_empty() {
                ui.weak("Nenhuma variável do jogo.");
            }
            egui::Grid::new("vars-no-jogo")
                .num_columns(2)
                .show(ui, |ui| {
                    for (k, v) in &game.vars {
                        ui.label(k);
                        ui.label(RichText::new(prop_text(v)).color(GAME));
                        ui.end_row();
                    }
                });
            if game.paused {
                ui.weak("O jogo está pausado.");
            }
        }
        ui.add_space(10.0);
        ui.weak("Selecione um objeto na cena ou na Hierarquia para ver as propriedades dele.");
    }

    fn object_ui(
        &mut self,
        ui: &mut egui::Ui,
        doc: &mut SceneDoc,
        id: u32,
        files: &FileTree,
        sizes: &mut crate::scene_render::Sizes,
        mode: Mode,
    ) {
        crate::events::set_three_d(doc.data.three_d);
        let full = mode.is_full();
        let o = doc.data.find(id).cloned().expect("objeto selecionado");
        let multi = self.multi.clone();
        // Nome, visível, travado. (Com vários, o nome fica de fora.)
        if multi.is_none() {
            ui.horizontal(|ui| {
                let mut name = o.name.clone();
                let r = ui.add(
                    egui::TextEdit::singleline(&mut name).desired_width(ui.available_width() - 4.0),
                );
                if r.changed() {
                    doc.record(Some(&format!("{id}:name")));
                    if let Some(obj) = doc.data.find_mut(id) {
                        obj.name = name;
                    }
                }
            });
        }
        ui.horizontal(|ui| {
            let mut v = o.visible;
            if ui.checkbox(&mut v, "Visível").changed() {
                doc.record(None);
                if let Some(obj) = doc.data.find_mut(id) {
                    obj.visible = v;
                }
            }
            let mut l = o.locked;
            if ui
                .checkbox(&mut l, "Travado")
                .on_hover_text("Não seleciona clicando na cena.")
                .changed()
            {
                doc.record(None);
                if let Some(obj) = doc.data.find_mut(id) {
                    obj.locked = l;
                }
            }
            if full {
                ui.weak(format!("id {id}"));
            }
        });
        ui.add_space(4.0);

        let d = multi
            .as_ref()
            .map_or([false; 4], |m| [m.pos, m.rot, m.scale, m.layer]);
        if doc.data.three_d {
            transform3_ui(ui, doc, id, &o, d);
        } else {
            egui::CollapsingHeader::new(RichText::new("Transformação").strong())
            .id_salt(("transformacao", id))
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new(("grade-transformacao", id))
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        let mut t = (o.x, o.y, o.rotation, o.scale_x, o.scale_y, o.layer);
                        transform_label(ui, "Posição", "", d[0]);
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut t.0).speed(1.0).prefix("x "));
                            ui.add(egui::DragValue::new(&mut t.1).speed(1.0).prefix("y "));
                        });
                        ui.end_row();
                        transform_label(ui, "Rotação", "", d[1]);
                        ui.add(egui::DragValue::new(&mut t.2).speed(1.0).suffix("°"));
                        ui.end_row();
                        transform_label(ui, "Escala", "", d[2]);
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut t.3).speed(0.01).prefix("x "));
                            ui.add(egui::DragValue::new(&mut t.4).speed(0.01).prefix("y "));
                        });
                        ui.end_row();
                        transform_label(
                            ui,
                            "Camada",
                            "Objetos de camada maior são desenhados por cima.",
                            d[3],
                        );
                        ui.add(egui::DragValue::new(&mut t.5).speed(0.1));
                        ui.end_row();
                        // Na tela (HUD ou cena por cima), os objetos da raiz
                        // ficam presos a um ponto da tela.
                        let root = doc.data.parent_of(id) == Some(None);
                        if root && doc.data.is_screen(&o) {
                            let r = ui.label("Preso em").on_hover_text(
                                "Ponto da tela de onde a posição conta. Num canto, o objeto continua no canto em qualquer tamanho de tela.",
                            );
                            crate::tutorial::anchor_in(ui, "inspetor:ancora", r.rect);
                            if let Some(a) = anchor_grid(ui, id, &o.anchor) {
                                // Vai para dentro daquele canto (ou lado), com
                                // uma margem; depois dá para ajustar.
                                let (b, _) = crate::scene_render::local_bounds(&o, files.root(), sizes, 0.0);
                                let (sx, sy) = (o.scale_x.abs(), o.scale_y.abs());
                                let (x0, x1) = (b.min.0 * sx, b.max.0 * sx);
                                let (y0, y1) = (b.min.1 * sy, b.max.1 * sy);
                                const MARGIN: f64 = 24.0;
                                let (_, _, fx, fy) = crate::scene_data::SCREEN_ANCHORS
                                    .iter()
                                    .copied()
                                    .find(|p| p.0 == a)
                                    .unwrap_or(("center", "", 0.5, 0.5));
                                t.0 = round2(if fx < 0.25 {
                                    MARGIN - x0
                                } else if fx > 0.75 {
                                    -MARGIN - x1
                                } else {
                                    -(x0 + x1) / 2.0
                                });
                                t.1 = round2(if fy < 0.25 {
                                    -MARGIN - y1
                                } else if fy > 0.75 {
                                    MARGIN - y0
                                } else {
                                    -(y0 + y1) / 2.0
                                });
                                doc.record(None);
                                if let Some(obj) = doc.data.find_mut(id) {
                                    obj.anchor = a;
                                    obj.x = t.0;
                                    obj.y = t.1;
                                }
                            }
                            ui.end_row();
                        }
                        // Onde está no jogo aberto, se mudou.
                        if multi.is_none()
                            && let Some(g) = self.game.as_ref().and_then(|g| g.objects.get(&id))
                        {
                            let l = g.local;
                            let differs = (l.x - o.x).abs() > 0.01
                                || (l.y - o.y).abs() > 0.01
                                || (l.rotation - o.rotation).abs() > 0.01
                                || (l.scale_x - o.scale_x).abs() > 0.001
                                || (l.scale_y - o.scale_y).abs() > 0.001;
                            if differs {
                                ui.label(RichText::new("No jogo").color(GAME));
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "x {}  y {}  {}°",
                                            crate::scene_data::format_num(round2(l.x)),
                                            crate::scene_data::format_num(round2(l.y)),
                                            crate::scene_data::format_num(round2(l.rotation)),
                                        ))
                                        .color(GAME),
                                    );
                                    if ui
                                        .small_button("Manter")
                                        .on_hover_text("Traz a posição, a rotação e a escala do jogo para a cena")
                                        .clicked()
                                    {
                                        t = (
                                            round2(l.x),
                                            round2(l.y),
                                            round2(l.rotation),
                                            l.scale_x,
                                            l.scale_y,
                                            t.5,
                                        );
                                    }
                                });
                                ui.end_row();
                            }
                        }
                let now = doc
                            .data
                            .find(id)
                            .map(|x| (x.x, x.y, x.rotation, x.scale_x, x.scale_y, x.layer))
                            .unwrap_or_default();
                        if t != now {
                            doc.record(Some(&format!("{id}:transformacao")));
                            if let Some(obj) = doc.data.find_mut(id) {
                                (
                                    obj.x,
                                    obj.y,
                                    obj.rotation,
                                    obj.scale_x,
                                    obj.scale_y,
                                    obj.layer,
                                ) = t;
                            }
                        }
                    });
            });
        }

        if o.component("body").is_some() {
            if doc.data.parent_of(id).flatten().is_some() {
                ui.colored_label(
                    crate::theme::WARNING,
                    "⚠ Objetos com corpo físico funcionam melhor na raiz (sem pai).",
                );
            }
            if o.rotation != 0.0 {
                ui.colored_label(
                    crate::theme::WARNING,
                    "⚠ O corpo físico não gira (só a imagem).",
                );
            }
        }

        // Instância de prefab: os componentes vêm dele.
        let instance = o.prefab.clone();
        if let Some(link) = instance.as_ref().filter(|_| multi.is_none()) {
            let mut unlink = false;
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Prefab").strong().color(PREFAB));
                    ui.label(&link.path);
                });
                if link.base.is_none() {
                    ui.colored_label(
                        crate::theme::ERROR,
                        "O arquivo do prefab não foi encontrado (ou tem erro).",
                    );
                } else {
                    ui.weak("Mudanças aqui valem só para este objeto (aparecem em negrito, com ↺ para voltar ao do prefab).");
                }
                ui.horizontal(|ui| {
                    if ui.button("Abrir prefab").clicked() {
                        self.actions_out.push(InspectorAction::OpenFile(link.path.clone()));
                    }
                    if ui
                        .button("Desvincular")
                        .on_hover_text("Vira um objeto comum: os filhos do prefab passam a ser dele e mudanças no prefab não chegam mais.")
                        .clicked()
                    {
                        unlink = true;
                    }
                });
            });
            if unlink {
                doc.record(None);
                let mut next = doc.data.next_id();
                if let Some(obj) = doc.data.find_mut(id) {
                    let mut kids: Vec<Object> = obj.prefab_children().to_vec();
                    fn renumber(o: &mut Object, next: &mut u32) {
                        o.id = *next;
                        *next += 1;
                        for c in &mut o.children {
                            renumber(c, next);
                        }
                    }
                    for k in &mut kids {
                        renumber(k, &mut next);
                    }
                    kids.append(&mut obj.children);
                    obj.children = kids;
                    obj.prefab = None;
                }
                return;
            }
        }

        let names: Vec<String> = {
            let mut v = Vec::new();
            crate::scene_data::visit(&doc.data.objects, &mut |x| v.push(x.name.clone()));
            v
        };
        let root = files.root().to_path_buf();
        let infos: Vec<Option<BehaviorInfo>> = o
            .components
            .iter()
            .map(|c| {
                (c.is_behavior() && !c.str("script").is_empty())
                    .then(|| self.catalog.get(&root, c.str("script")))
            })
            .collect();
        let mut remove = None;
        let mut swap = None;
        let mut reset: Option<usize> = None;
        let mut paste_values: Option<usize> = None;
        let mut add: Vec<Component> = Vec::new();
        let count = o.components.len();
        for (ci, comp) in o.components.iter().enumerate() {
            if multi
                .as_ref()
                .is_some_and(|m| !m.common.get(ci).copied().unwrap_or(false))
            {
                continue;
            }
            let (mut title, mut desc) = {
                let (t, d) = component_info(&comp.kind);
                (t.to_string(), d.to_string())
            };
            if let Some(info) = &infos[ci] {
                title = info.title.clone();
                desc = info.description.clone();
            }
            let state = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                ui.id().with(("componente", id, ci, &comp.kind)),
                true,
            );
            state
                .show_header(ui, |ui| {
                    let mut r = ui.add(
                        egui::Label::new(RichText::new(&title).strong()).sense(egui::Sense::click()),
                    );
                    crate::tutorial::anchor_in(
                        ui,
                        &format!("inspetor:componente:{}", comp.kind),
                        r.rect,
                    );
                    if !desc.is_empty() {
                        r = r.on_hover_text(&desc);
                    }
                    if instance.is_some() {
                        return;
                    }
                    // O mesmo menu no botão direito do nome e no ☰.
                    let mut menu = |ui: &mut egui::Ui| {
                        if ci > 0 && ui.button("⏶ Subir").clicked() {
                            swap = Some((ci, ci - 1));
                            ui.close();
                        }
                        if ci + 1 < count && ui.button("⏷ Descer").clicked() {
                            swap = Some((ci, ci + 1));
                            ui.close();
                        }
                        if ui
                            .button("📋 Copiar componente")
                            .on_hover_text("Para colar os valores em outro igual, ou como componente novo em outro objeto")
                            .clicked()
                        {
                            ui.ctx().data_mut(|d| {
                                d.insert_temp(copied_component_id(), comp.clone());
                            });
                            ui.close();
                        }
                        let can_paste = copied_component(ui.ctx())
                            .is_some_and(|c| comp_key(&c) == comp_key(comp));
                        if ui
                            .add_enabled(can_paste, egui::Button::new("📋 Colar valores"))
                            .on_hover_text("Põe aqui os valores do componente copiado")
                            .on_disabled_hover_text("Copie um componente deste mesmo tipo antes")
                            .clicked()
                        {
                            paste_values = Some(ci);
                            ui.close();
                        }
                        ui.separator();
                        if ui
                            .button("↺ Restaurar o padrão")
                            .on_hover_text("Volta os valores ao começo (o arquivo, a forma e o tipo continuam)")
                            .clicked()
                        {
                            reset = Some(ci);
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("🗑 Remover componente").clicked() {
                            remove = Some(ci);
                            ui.close();
                        }
                    };
                    r.context_menu(&mut menu);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.menu_button("☰", &mut menu)
                            .response
                            .on_hover_text("Subir, descer, restaurar ou remover (ou o botão direito no nome)");
                    });
                })
                .body(|ui| {
                    if comp.is_behavior() {
                        self.behavior_ui(
                            ui,
                            doc,
                            &o,
                            ci,
                            comp,
                            infos[ci].as_ref(),
                            files,
                            sizes,
                            &names,
                            full,
                            &mut add,
                        );
                    } else {
                        self.component_ui(ui, doc, id, ci, comp, files, sizes, &names, full);
                    }
                });
        }
        if let Some(ci) = remove {
            doc.record(None);
            if let Some(obj) = doc.data.find_mut(id) {
                obj.components.remove(ci);
            }
        }
        if let Some(ci) = paste_values
            && let Some(c) = copied_component(ui.ctx())
            && o.components.get(ci).is_some_and(|x| *x != c)
        {
            doc.record(None);
            if let Some(obj) = doc.data.find_mut(id) {
                obj.components[ci] = c;
            }
        }
        if let Some(ci) = reset
            && let Some(comp) = o.components.get(ci)
        {
            let fresh = default_component(comp, doc.data.three_d, infos[ci].as_ref());
            if &fresh != comp {
                doc.record(None);
                if let Some(obj) = doc.data.find_mut(id) {
                    obj.components[ci] = fresh;
                }
            }
        }
        if let Some((a, b)) = swap {
            doc.record(None);
            if let Some(obj) = doc.data.find_mut(id) {
                obj.components.swap(a, b);
            }
        }
        if !add.is_empty() {
            doc.record(None);
            if let Some(obj) = doc.data.find_mut(id) {
                obj.components.extend(add);
            }
        }
        if multi.is_none()
            && let Some(obj) = doc.data.find(id)
            && events_summary(ui, &obj.events, true)
        {
            self.actions_out.push(InspectorAction::ShowEvents);
        }
        ui.add_space(8.0);
        if instance.is_some() {
            ui.weak("Para pôr ou tirar componentes, abra o prefab: a mudança vale para todas as cópias.");
            return;
        }
        let mut chosen: Vec<Component> = Vec::new();
        // Os ossos do modelo do pai (Preso a um osso já vem com uma mão).
        let parent_bones = doc
            .data
            .parent_of(id)
            .flatten()
            .and_then(|p| doc.data.find(p))
            .and_then(|p| p.component("model"))
            .map(|m| sizes.model_bones(files.root(), m.str("file")))
            .unwrap_or_default();
        let add_menu = ui.menu_button("➕ Adicionar componente", |ui| {
            let order = &crate::scene_data::schema().order;
            let is_ui = |k: &str| {
                matches!(
                    k,
                    "panel"
                        | "button"
                        | "bar"
                        | "icons"
                        | "slider"
                        | "checkbox"
                        | "textfield"
                        | "layout"
                )
            };
            let mut item = |ui: &mut egui::Ui, kind: &str| {
                let (title, desc) = component_info(kind);
                let exists = o.component(kind).is_some();
                let unique = matches!(
                    kind,
                    "body" | "camera" | "map" | "layout" | "body3d" | "anim3d" | "terrain" | "bone"
                );
                if ui
                    .add_enabled(!(exists && unique), egui::Button::new(title))
                    .on_hover_text(desc)
                    .clicked()
                {
                    let mut c = if kind == "particles" && doc.data.three_d {
                        crate::particle_presets::component_3d()
                    } else {
                        Component::new(kind)
                    };
                    if kind == "text" && doc.data.three_d {
                        // No 3D, texto é do HUD (fica na tela).
                        c.set("hud", Prop::Bool(true));
                    }
                    if kind == "bone"
                        && let Some(b) = hand_bone(&parent_bones)
                    {
                        c.set("bone", Prop::Str(b));
                    }
                    if kind == "body" {
                        // O corpo já vem do tamanho do que aparece.
                        if let Some((w, h)) = visible_size(&o, sizes, files) {
                            c.set("w", Prop::Num(w.round()));
                            c.set("h", Prop::Num(h.round()));
                        }
                    }
                    chosen.push(c);
                    ui.close();
                }
            };
            let three_d = doc.data.three_d;
            let fits = |k: &str| {
                if three_d {
                    !is_2d_only_kind(k)
                } else {
                    !is_3d_kind(k)
                }
            };
            for kind in order.iter().filter(|k| !is_ui(k) && fits(k)) {
                item(ui, kind);
            }
            ui.menu_button("Interface", |ui| {
                for kind in order.iter().filter(|k| is_ui(k)) {
                    item(ui, kind);
                }
            });
            ui.separator();
            ui.label(RichText::new("Comportamentos").weak());
            // Só os prontos que funcionam nesta cena (2D ou 3D).
            let cats: Vec<&str> = behaviors::categories()
                .into_iter()
                .filter(|c| {
                    behaviors::builtins()
                        .iter()
                        .any(|b| b.category == *c && b.fits(three_d))
                })
                .collect();
            for cat in cats {
                ui.menu_button(cat, |ui| {
                    for b in behaviors::builtins()
                        .iter()
                        .filter(|b| b.category == cat && b.fits(three_d))
                    {
                        if ui.button(&b.title).on_hover_text(&b.description).clicked() {
                            // Quem precisa de corpo 3D já ganha um, do tipo
                            // que o comportamento pede.
                            if b.requires.iter().any(|r| r == "body3d")
                                && o.component("body3d").is_none()
                            {
                                let mut body = Component::new("body3d");
                                if !b.body.is_empty() {
                                    body.set("body", Prop::Str(b.body.clone()));
                                }
                                chosen.push(body);
                            }
                            // Quem precisa de corpo físico já ganha um.
                            if b.requires.iter().any(|r| r == "body")
                                && o.component("body").is_none()
                            {
                                let mut body = Component::new("body");
                                if let Some((w, h)) = visible_size(&o, sizes, files) {
                                    body.set("w", Prop::Num(w.round()));
                                    body.set("h", Prop::Num(h.round()));
                                }
                                chosen.push(body);
                            }
                            chosen.push(Component::behavior(
                                &b.script,
                                &b.defaults_for(doc.data.three_d),
                            ));
                            ui.close();
                        }
                    }
                });
            }
            ui.menu_button("Script do jogo", |ui| {
                if ui
                    .button("➕ Novo script...")
                    .on_hover_text("Cria um arquivo em scripts/ com o modelo de comportamento")
                    .clicked()
                {
                    self.actions_out
                        .push(InspectorAction::NewBehaviorScript(id));
                    ui.close();
                }
                let scripts: Vec<String> = files
                    .files_of(FileKind::Lua)
                    .map(|e| e.rel.clone())
                    .filter(|r| is_behavior_candidate(r))
                    .collect();
                if !scripts.is_empty() {
                    ui.separator();
                }
                for rel in scripts {
                    if ui.button(&rel).clicked() {
                        let info = self.catalog.get(&root, &rel);
                        chosen.push(Component::behavior(
                            &rel,
                            &info.defaults_for(doc.data.three_d),
                        ));
                        ui.close();
                    }
                }
            });
        });
        crate::tutorial::anchor_in(ui, "inspetor.adicionar", add_menu.response.rect);
        // O componente copiado (☰ > Copiar componente) entra como novo.
        if let Some(c) = copied_component(ui.ctx()) {
            let (title, _) = component_info(&c.kind);
            let unique = matches!(
                c.kind.as_str(),
                "body" | "camera" | "map" | "layout" | "body3d" | "anim3d" | "terrain" | "bone"
            );
            let fits = if doc.data.three_d {
                !is_2d_only_kind(&c.kind)
            } else {
                !is_3d_kind(&c.kind)
            };
            let blocked = unique && o.component(&c.kind).is_some();
            if fits
                && ui
                    .add_enabled(
                        !blocked,
                        egui::Button::new(format!("📋 Colar componente: {title}")),
                    )
                    .on_hover_text(
                        "Põe aqui uma cópia do componente copiado, com os mesmos valores",
                    )
                    .on_disabled_hover_text("Este objeto já tem um deste (só pode um)")
                    .clicked()
            {
                chosen.push(c);
            }
        }
        if !chosen.is_empty() {
            doc.record(None);
            if let Some(obj) = doc.data.find_mut(id) {
                obj.components.extend(chosen);
            }
            self.scroll_to_end = true;
        } else if std::mem::take(&mut self.scroll_to_end) {
            ui.scroll_to_cursor(Some(egui::Align::Max));
        }
    }

    /// Um comportamento: escolher o script e as propriedades dele.
    #[allow(clippy::too_many_arguments)]
    fn behavior_ui(
        &mut self,
        ui: &mut egui::Ui,
        doc: &mut SceneDoc,
        o: &Object,
        ci: usize,
        comp: &Component,
        info: Option<&BehaviorInfo>,
        files: &FileTree,
        sizes: &mut crate::scene_render::Sizes,
        names: &[String],
        full: bool,
        add: &mut Vec<Component>,
    ) {
        let id = o.id;
        // Para campos de animação: as animações da imagem do objeto (ou, no
        // 3D, as do modelo).
        let anims = o
            .components
            .iter()
            .find(|c| c.kind == "sprite")
            .map(|c| sizes.anims.names(files.root(), c.str("image")))
            .or_else(|| {
                o.components
                    .iter()
                    .find(|c| c.kind == "model")
                    .map(|c| sizes.model_anims(files.root(), c.str("file")))
            })
            .unwrap_or_default();
        // Campos de osso: os do modelo 3D do objeto.
        let bones = o
            .component("model")
            .map(|c| sizes.model_bones(files.root(), c.str("file")))
            .unwrap_or_default();
        let mut changes: Vec<(String, Prop)> = Vec::new();
        let mut reverts: Vec<String> = Vec::new();
        let script = comp.str("script").to_string();
        let Some(info) = info else {
            ui.weak("Escolha um comportamento em ➕ Adicionar componente.");
            return;
        };
        if let Some(e) = &info.error {
            ui.colored_label(crate::theme::ERROR, e);
        }
        if !info.is_builtin() {
            ui.horizontal(|ui| {
                ui.weak(&script);
                if ui.small_button("Abrir script").clicked() {
                    self.actions_out
                        .push(InspectorAction::OpenFile(script.clone()));
                }
            });
        }
        // Do que ele precisa.
        for req in &info.requires {
            // Numa cena 3D, quem precisa de texto usa o Texto 3D.
            let req = if req == "text" && doc.data.three_d {
                "text3d"
            } else {
                req.as_str()
            };
            if o.component(req).is_none() && !(req == "text3d" && o.component("text").is_some()) {
                let (title, _) = component_info(req);
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(crate::theme::WARNING, format!("⚠ Precisa de: {title}"));
                    if o.prefab.is_none() && ui.small_button("Adicionar").clicked() {
                        add.push(Component::new(req));
                    }
                });
            }
        }
        let three_d = doc.data.three_d;
        let value = |key: &str| -> Prop {
            comp.get(key)
                .cloned()
                .or_else(|| info.prop(key).map(|p| p.default_for(three_d)))
                .unwrap_or(Prop::Str(String::new()))
        };
        egui::Grid::new(("grade-comportamento", id, ci))
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                for p in &info.props {
                    if (p.advanced && !full) || !p.fits(three_d) {
                        continue;
                    }
                    if let Some((other, values)) = &p.show_if {
                        let v = value(other);
                        let v = match &v {
                            Prop::Str(s) => s.clone(),
                            other => other.to_lua(),
                        };
                        if !values.contains(&v) {
                            continue;
                        }
                    }
                    let spec = behavior_spec(info, p);
                    let overridden = o.is_overridden(ci, &p.key);
                    let differs = self.multi.as_ref().is_some_and(|m| m.differs(ci, &p.key));
                    if field_row(ui, &spec, overridden, differs, &mut reverts) {
                        // (a linha mostrou o rótulo)
                    }
                    let current = value(&p.key);
                    ui.horizontal(|ui| {
                        if let Some(new) = field_widget(
                            ui,
                            &spec,
                            &current,
                            files,
                            names,
                            Lists {
                                anims: &anims,
                                bones: &bones,
                            },
                            (id, ci),
                        ) {
                            changes.push((p.key.clone(), new));
                        }
                        // Valor no jogo aberto, se mudou (o jogo pode mexer nele).
                        if let Some(g) = self.game.as_ref().and_then(|g| g.prop(id, ci, &p.key))
                            && !same_value(g, &current)
                        {
                            ui.label(RichText::new(format!("jogo: {}", prop_text(g))).color(GAME));
                            if ui
                                .small_button("Manter")
                                .on_hover_text("Usa o valor do jogo na cena")
                                .clicked()
                            {
                                changes.push((p.key.clone(), g.clone()));
                            }
                        }
                    });
                    ui.end_row();
                }
            });
        if info.props.is_empty() && info.error.is_none() {
            ui.weak("Este comportamento não tem propriedades.");
        }
        // O Diálogo: abrir a conversa no editor.
        let conversa = value("arquivo");
        if info.script == "nn:dialogo"
            && let Prop::Str(rel) = &conversa
            && !rel.is_empty()
        {
            let r = ui
                .button("💬 Editar a conversa")
                .on_hover_text(format!("Abre {rel} no editor de conversa"));
            crate::tutorial::anchor(ui.ctx(), "inspetor.editar_conversa", r.rect);
            if r.clicked() {
                self.actions_out
                    .push(InspectorAction::OpenFile(rel.clone()));
            }
        }
        // Valores guardados de propriedades que o script não tem mais.
        let stale: Vec<String> = comp
            .props
            .iter()
            .map(|(k, _)| k.clone())
            .filter(|k| k != "script" && info.prop(k).is_none() && info.error.is_none())
            .collect();
        if !stale.is_empty() && o.prefab.is_none() {
            ui.horizontal_wrapped(|ui| {
                ui.weak(format!(
                    "Valores que o script não usa mais: {}",
                    stale.join(", ")
                ));
                if ui.small_button("Limpar").clicked() {
                    doc.record(None);
                    if let Some(c) = doc.data.find_mut(id).and_then(|x| x.components.get_mut(ci)) {
                        for k in &stale {
                            c.remove(k);
                        }
                    }
                }
            });
        }
        apply_changes(doc, o, ci, changes, reverts);
    }

    #[allow(clippy::too_many_arguments)]
    fn component_ui(
        &mut self,
        ui: &mut egui::Ui,
        doc: &mut SceneDoc,
        id: u32,
        ci: usize,
        comp: &Component,
        files: &FileTree,
        sizes: &mut crate::scene_render::Sizes,
        names: &[String],
        full: bool,
    ) {
        let mut changes: Vec<(String, Prop)> = Vec::new();
        let mut reverts: Vec<String> = Vec::new();
        let object = doc.data.find(id).cloned();
        let obj = object.as_ref();
        let is_instance = obj.is_some_and(|x| x.prefab.is_some());
        let circle = comp.str("shape") == "circle";
        let anims = if comp.kind == "sprite" {
            sizes.anims.names(files.root(), comp.str("image"))
        } else if comp.kind == "anim3d" {
            obj.and_then(|x| x.component("model"))
                .map(|m| sizes.model_anims(files.root(), m.str("file")))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        // Preso a um osso: os ossos do modelo do pai.
        let bones = if comp.kind == "bone" {
            doc.data
                .parent_of(id)
                .flatten()
                .and_then(|p| doc.data.find(p))
                .and_then(|p| p.component("model"))
                .map(|m| sizes.model_bones(files.root(), m.str("file")))
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        if comp.kind == "bone" && bones.is_empty() {
            ui.colored_label(
                crate::theme::WARNING,
                "Ponha este objeto como filho de um personagem com esqueleto (arraste para cima dele na Hierarquia).",
            );
        }
        // Partículas: efeitos prontos e soltar na prévia.
        if comp.kind == "particles" {
            ui.horizontal_wrapped(|ui| {
                let r = ui.menu_button("✨ Efeito pronto", |ui| {
                    for p in crate::particle_presets::presets(doc.data.three_d) {
                        let r = ui.button(p.label).on_hover_text(p.tip);
                        crate::tutorial::anchor_in(ui, &format!("inspetor:fx:{}", p.id), r.rect);
                        if r.clicked() {
                            let c = p.component();
                            for (k, v) in &c.props {
                                changes.push((k.clone(), v.clone()));
                            }
                            ui.close();
                        }
                    }
                });
                r.response
                    .on_hover_text("Troca os valores pelos de um efeito pronto (Ctrl+Z desfaz)");
                let r = ui
                    .button("💥 Soltar")
                    .on_hover_text("Solta partículas de uma vez na prévia da cena");
                crate::tutorial::anchor_in(ui, "inspetor:soltar", r.rect);
                if r.clicked() {
                    doc.burst_requests.push(id);
                }
            });
        }
        egui::Grid::new(("grade-componente", id, ci))
            .num_columns(2)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                for spec in fields(&comp.kind) {
                    if spec.advanced && !full {
                        continue;
                    }
                    // Raio só para círculos; largura/altura só para os outros.
                    if matches!(comp.kind.as_str(), "shape" | "body") {
                        if spec.key == "radius" && !circle {
                            continue;
                        }
                        if (spec.key == "w" || spec.key == "h") && circle {
                            continue;
                        }
                    }
                    if comp.kind == "sprite" && spec.key == "frame" && comp.num("frame_w") <= 0.0 {
                        continue;
                    }
                    // A abertura e a borda só valem no holofote.
                    if comp.kind == "light3d"
                        && matches!(spec.key, "angle" | "soft")
                        && comp.str("kind") != "spot"
                    {
                        continue;
                    }
                    if hidden_ui_field(comp, spec.key) {
                        continue;
                    }
                    // No 3D, as partículas saem para o "cima" do objeto (sem
                    // a direção do 2D).
                    let particles3d = comp.kind == "particles" && doc.data.three_d;
                    // Som 3D: só numa cena 3D (e a distância, com ele ligado).
                    if comp.kind == "sound"
                        && ((spec.key == "spatial" && !doc.data.three_d)
                            || (spec.key == "distance"
                                && !(doc.data.three_d && comp.bool("spatial"))))
                    {
                        continue;
                    }
                    if particles3d && spec.key == "direction" {
                        continue;
                    }
                    // O vento só existe no 3D.
                    if comp.kind == "particles" && spec.key == "wind" && !doc.data.three_d {
                        continue;
                    }
                    if spec.key.is_empty() {
                        ui.label(RichText::new(spec.label).small().strong().weak());
                        ui.end_row();
                        continue;
                    }
                    let mut spec = Spec::from(spec);
                    if particles3d {
                        particles3d_spec(&mut spec);
                    }
                    let overridden =
                        is_instance && obj.is_some_and(|x| x.is_overridden(ci, &spec.key));
                    let differs = self
                        .multi
                        .as_ref()
                        .is_some_and(|m| m.differs(ci, &spec.key));
                    field_row(ui, &spec, overridden, differs, &mut reverts);
                    let current = comp.get(&spec.key).cloned().unwrap_or(Prop::Num(0.0));
                    if let Some(new) = field_widget(
                        ui,
                        &spec,
                        &current,
                        files,
                        names,
                        Lists {
                            anims: &anims,
                            bones: &bones,
                        },
                        (id, ci),
                    ) {
                        changes.push((spec.key.clone(), new));
                    }
                    ui.end_row();
                }
            });

        // Desenhar no editor de pixel art.
        if comp.kind == "sprite" {
            ui.horizontal_wrapped(|ui| {
                if comp.str("image").is_empty() {
                    let r = ui
                        .button("🎨 Desenhar uma imagem nova")
                        .on_hover_text("Cria uma imagem e abre no editor de pixel art");
                    crate::tutorial::anchor_in(ui, "inspetor:desenhar", r.rect);
                    if r.clicked() {
                        self.actions_out.push(InspectorAction::NewImage {
                            id,
                            ci,
                            name: obj.map(|o| o.name.clone()).unwrap_or_default(),
                        });
                    }
                } else {
                    let r = ui.button("🎨 Editar imagem").on_hover_text(
                        "Abre a imagem no editor de pixel art (e as animações dela)",
                    );
                    crate::tutorial::anchor_in(ui, "inspetor:editar-imagem", r.rect);
                    if r.clicked() {
                        self.actions_out
                            .push(InspectorAction::OpenFile(comp.str("image").to_string()));
                    }
                }
            });
        }
        // Pintar o mapa aqui na cena.
        if comp.kind == "map" && !comp.str("map").is_empty() {
            ui.horizontal_wrapped(|ui| {
                let on = doc.painting == Some(id);
                let r = ui
                    .selectable_label(on, "✏ Pintar")
                    .on_hover_text("Pinta os tiles direto na cena (o pincel fica no painel Tiles)");
                crate::tutorial::anchor_in(ui, "inspetor:pintar", r.rect);
                if r.clicked() {
                    doc.painting = if on { None } else { Some(id) };
                }
                if ui
                    .button("Abrir o mapa")
                    .on_hover_text("Na aba do mapa")
                    .clicked()
                {
                    self.actions_out
                        .push(InspectorAction::OpenFile(comp.str("map").to_string()));
                }
            });
        }
        // Ajudas da imagem: tamanho original, ×2, quadros.
        if comp.kind == "sprite" && !comp.str("image").is_empty() {
            let natural = sizes.image(files.root(), comp.str("image"));
            match natural {
                None => {
                    ui.colored_label(crate::theme::ERROR, "Imagem não encontrada (ou não é PNG).");
                }
                Some((iw, ih)) => {
                    let (fw, fh) = if comp.num("frame_w") > 0.0 && comp.num("frame_h") > 0.0 {
                        (comp.num("frame_w"), comp.num("frame_h"))
                    } else {
                        (iw as f64, ih as f64)
                    };
                    ui.horizontal_wrapped(|ui| {
                        ui.weak(format!("{iw}×{ih} px"));
                        if ui
                            .small_button("Original")
                            .on_hover_text("Tamanho da imagem (ou do quadro)")
                            .clicked()
                        {
                            changes.push(("w".into(), Prop::Num(0.0)));
                            changes.push(("h".into(), Prop::Num(0.0)));
                        }
                        for k in [2.0, 3.0, 4.0] {
                            if ui.small_button(format!("×{k}")).clicked() {
                                changes.push(("w".into(), Prop::Num(fw * k)));
                                changes.push(("h".into(), Prop::Num(fh * k)));
                            }
                        }
                        if comp.num("frame_w") <= 0.0
                            && iw > ih
                            && ih > 0.0
                            && (iw % ih) == 0.0
                            && ui
                                .small_button("É uma tira de quadros")
                                .on_hover_text(format!("Divide em quadros de {ih}×{ih}"))
                                .clicked()
                        {
                            changes.push(("frame_w".into(), Prop::Num(ih as f64)));
                            changes.push(("frame_h".into(), Prop::Num(ih as f64)));
                            changes.push(("frame".into(), Prop::Num(1.0)));
                        }
                    });
                    if comp.num("frame_w") > 0.0
                        && comp.num("frame_h") > 0.0
                        && let Some(frame) = self.frame_picker(ui, files, comp, (iw, ih))
                    {
                        changes.push(("frame".into(), Prop::Num(frame as f64)));
                    }
                }
            }
        }
        if comp.kind == "body"
            && let Some(o) = doc.data.find(id)
            && let Some((w, h)) = visible_size(o, sizes, files)
            && ui.small_button("Ajustar ao que aparece").clicked()
        {
            changes.push(("w".into(), Prop::Num(w.round())));
            changes.push(("h".into(), Prop::Num(h.round())));
            changes.push(("radius".into(), Prop::Num((w.max(h) / 2.0).round())));
        }

        if let Some(o) = obj {
            apply_changes(doc, o, ci, changes, reverts);
        }
    }

    /// Grade da spritesheet para escolher o quadro com um clique.
    fn frame_picker(
        &mut self,
        ui: &mut egui::Ui,
        files: &FileTree,
        comp: &Component,
        size: (f32, f32),
    ) -> Option<u32> {
        let rel = comp.str("image").to_string();
        let modified = files.get(&rel).and_then(|e| e.modified);
        let stale = self.images.get(&rel).is_none_or(|(m, _)| *m != modified);
        if stale {
            let img = crate::viewers::load_rgba(&files.full(&rel)).ok()?;
            let tex = ui.ctx().load_texture(
                format!("inspetor:{rel}"),
                crate::viewers::to_color_image(&img),
                egui::TextureOptions::NEAREST,
            );
            self.images.insert(rel.clone(), (modified, tex));
        }
        let tex = self.images.get(&rel)?.1.clone();
        let (fw, fh) = (comp.num("frame_w") as f32, comp.num("frame_h") as f32);
        let cols = (size.0 / fw).floor().max(1.0) as u32;
        let rows = (size.1 / fh).floor().max(1.0) as u32;
        let width = ui.available_width().min(260.0);
        let scale = (width / size.0).min(160.0 / size.1).max(0.25);
        let (rect, resp) = ui.allocate_exact_size(
            Vec2::new(size.0 * scale, size.1 * scale),
            egui::Sense::click(),
        );
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 0.0, Color32::from_gray(0x40));
        p.image(
            tex.id(),
            rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        let grid = egui::Stroke::new(1.0, Color32::from_white_alpha(70));
        for c in 0..=cols {
            let x = rect.left() + c as f32 * fw * scale;
            p.line_segment(
                [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
                grid,
            );
        }
        for r in 0..=rows {
            let y = rect.top() + r as f32 * fh * scale;
            p.line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                grid,
            );
        }
        let current = comp.num("frame").max(1.0) as u32 - 1;
        let cell = |n: u32| {
            egui::Rect::from_min_size(
                rect.min
                    + Vec2::new(
                        (n % cols) as f32 * fw * scale,
                        (n / cols) as f32 * fh * scale,
                    ),
                Vec2::new(fw * scale, fh * scale),
            )
        };
        p.rect_stroke(
            cell(current),
            0.0,
            egui::Stroke::new(2.0, Color32::from_rgb(0xFF, 0xA9, 0x2E)),
            egui::StrokeKind::Inside,
        );
        if let Some(pos) = resp.hover_pos() {
            let col = ((pos.x - rect.left()) / (fw * scale)) as u32;
            let row = ((pos.y - rect.top()) / (fh * scale)) as u32;
            if col < cols && row < rows {
                let n = row * cols + col;
                p.rect_stroke(
                    cell(n),
                    0.0,
                    egui::Stroke::new(1.0, Color32::WHITE),
                    egui::StrokeKind::Inside,
                );
                if resp.clicked() {
                    return Some(n + 1);
                }
                resp.on_hover_text(format!("quadro {}", n + 1));
            }
        }
        None
    }
}

/// Tamanho do que aparece (imagem ou forma), já com a escala.
fn visible_size(
    o: &crate::scene_data::Object,
    sizes: &mut crate::scene_render::Sizes,
    files: &FileTree,
) -> Option<(f64, f64)> {
    let mut only_visual = o.clone();
    only_visual
        .components
        .retain(|c| matches!(c.kind.as_str(), "sprite" | "shape"));
    if only_visual.components.is_empty() {
        return None;
    }
    let (r, _) = crate::scene_render::local_bounds(&only_visual, files.root(), sizes, 32.0);
    Some((
        (r.max.0 - r.min.0) * o.scale_x.abs(),
        (r.max.1 - r.min.1) * o.scale_y.abs(),
    ))
}

/// Mostra o controle de um campo. Devolve o valor novo, se mudou.
fn field_widget(
    ui: &mut egui::Ui,
    spec: &Spec,
    current: &Prop,
    files: &FileTree,
    names: &[String],
    lists: Lists<'_>,
    salt: (u32, usize),
) -> Option<Prop> {
    let Lists { anims, bones } = lists;
    let width = ui.available_width().clamp(80.0, 180.0);
    match &spec.w {
        Wd::Num {
            speed,
            min,
            max,
            suffix,
        } => {
            let mut v = current.num();
            ui.add(
                egui::DragValue::new(&mut v)
                    .speed(*speed)
                    .range(*min..=*max)
                    .suffix(*suffix),
            )
            .changed()
            .then_some(Prop::Num(v))
        }
        Wd::Int { min, max } => {
            let mut v = current.num().round();
            ui.add(
                egui::DragValue::new(&mut v)
                    .speed(0.1)
                    .range(*min..=*max)
                    .max_decimals(0),
            )
            .changed()
            .then_some(Prop::Num(v.round()))
        }
        Wd::Bool => {
            let mut v = current.bool();
            ui.checkbox(&mut v, "").changed().then_some(Prop::Bool(v))
        }
        Wd::Text => {
            let mut v = current.str().to_string();
            ui.add(egui::TextEdit::singleline(&mut v).desired_width(width))
                .changed()
                .then_some(Prop::Str(v))
        }
        Wd::Multiline => {
            let mut v = current.str().to_string();
            ui.add(
                egui::TextEdit::multiline(&mut v)
                    .desired_rows(2)
                    .desired_width(width),
            )
            .changed()
            .then_some(Prop::Str(v))
        }
        Wd::Color => {
            let mut c = hex_to_rgba(current.str());
            ui.color_edit_button_srgba_unmultiplied(&mut c)
                .changed()
                .then(|| Prop::Str(rgba_to_hex(c)))
        }
        Wd::Choice(options) => {
            let mut v = current.str().to_string();
            let text = options
                .iter()
                .find(|(k, _)| *k == v)
                .map(|(_, l)| l.clone())
                .unwrap_or_else(|| v.clone());
            let mut changed = false;
            egui::ComboBox::from_id_salt(("escolha", salt, &spec.key))
                .width(width)
                .truncate()
                .selected_text(text)
                .show_ui(ui, |ui| {
                    for (k, l) in options {
                        if ui.selectable_value(&mut v, k.clone(), l).changed() {
                            changed = true;
                        }
                    }
                });
            changed.then_some(Prop::Str(v))
        }
        Wd::Object(empty) => {
            let mut v = current.str().to_string();
            let mut changed = false;
            egui::ComboBox::from_id_salt(("objeto", salt, &spec.key))
                .width(width)
                .truncate()
                .selected_text(if v.is_empty() {
                    empty.to_string()
                } else {
                    v.clone()
                })
                .show_ui(ui, |ui| {
                    changed |= ui.selectable_value(&mut v, String::new(), *empty).changed();
                    let mut seen = std::collections::HashSet::new();
                    for n in names {
                        if seen.insert(n) {
                            changed |= ui.selectable_value(&mut v, n.clone(), n).changed();
                        }
                    }
                });
            changed.then_some(Prop::Str(v))
        }
        Wd::Anim => anim_field(ui, spec, current, anims, salt, width),
        Wd::Bone(empty) => bone_field(ui, spec, current, bones, empty, salt, width),
        Wd::ThemeColor(key) => theme_color_field(ui, current, key),
        Wd::Var => var_field(ui, current, width),
        Wd::OptColor => {
            let value = current.str();
            let mut out = None;
            ui.horizontal(|ui| {
                let mut on = !value.is_empty();
                if ui.checkbox(&mut on, "").changed() {
                    out = Some(Prop::Str(if on { "#FFFFFF".into() } else { String::new() }));
                }
                if on {
                    let mut c = hex_to_rgba(value);
                    if ui.color_edit_button_srgba_unmultiplied(&mut c).changed() {
                        out = Some(Prop::Str(rgba_to_hex(c)));
                    }
                }
            });
            out
        }
        Wd::Font => {
            let list: Vec<String> = files
                .files_of(FileKind::Font)
                .map(|e| e.rel.clone())
                .collect();
            file_combo(ui, spec, current, &list, salt, "(do tema)", |p| {
                FileKind::of(std::path::Path::new(p)) == FileKind::Font
            })
        }
        Wd::Scene => {
            let list: Vec<String> = files
                .files_of(FileKind::Scene)
                .chain(files.files_of(FileKind::Lua))
                .map(|e| e.rel.clone())
                .filter(|r| r != "config.lua")
                .collect();
            file_combo(ui, spec, current, &list, salt, "(nenhum)", |p| {
                matches!(
                    FileKind::of(std::path::Path::new(p)),
                    FileKind::Scene | FileKind::Lua
                )
            })
        }
        Wd::File(kind) => {
            let kind = *kind;
            let list: Vec<String> = files
                .files_of(kind)
                .map(|e| e.rel.clone())
                .filter(|r| kind != FileKind::Map || r.to_lowercase().ends_with(".tmj"))
                .collect();
            file_combo(ui, spec, current, &list, salt, "(nenhum)", |p| {
                FileKind::of(std::path::Path::new(p)) == kind
            })
        }
    }
}

/// Os 9 pontos da tela para prender um objeto: um retângulo (a tela) com um
/// ponto em cada canto, no meio de cada lado e no centro. Devolve o escolhido.
fn anchor_grid(ui: &mut egui::Ui, id: u32, current: &str) -> Option<String> {
    let current = if current.is_empty() {
        "center"
    } else {
        current
    };
    let size = Vec2::new(72.0, 46.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let screen = rect.shrink(7.0);
    let visuals = ui.visuals().clone();
    ui.painter().rect_stroke(
        screen,
        2.0,
        egui::Stroke::new(1.0, visuals.weak_text_color()),
        egui::StrokeKind::Middle,
    );
    let mut out = None;
    for (name, label, fx, fy) in crate::scene_data::SCREEN_ANCHORS {
        let p = screen.min + Vec2::new(screen.width() * fx as f32, screen.height() * fy as f32);
        let hit = egui::Rect::from_center_size(p, Vec2::splat(14.0));
        let r = ui
            .interact(
                hit,
                ui.id().with(("ancora", id, name)),
                egui::Sense::click(),
            )
            .on_hover_text(format!("Preso {label}"));
        let on = name == current;
        r.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::RadioButton,
                true,
                on,
                format!("Preso {label}"),
            )
        });
        let (radius, color) = if on {
            (5.0, visuals.selection.bg_fill)
        } else if r.hovered() {
            (4.0, visuals.strong_text_color())
        } else {
            (3.0, visuals.weak_text_color())
        };
        ui.painter().circle_filled(p, radius, color);
        if r.clicked() && !on {
            out = Some(name.to_string());
        }
    }
    out
}

/// Cor que pode vir do tema: vazia, mostra a do tema (e mudar a cor a
/// separa do tema); com uma cor, ↺ volta para a do tema.
fn theme_color_field(ui: &mut egui::Ui, current: &Prop, key: &str) -> Option<Prop> {
    let value = current.str();
    let themed = value.is_empty();
    let mut c = hex_to_rgba(&crate::scene_data::themed(value, key));
    let mut out = None;
    ui.horizontal(|ui| {
        if ui
            .color_edit_button_srgba_unmultiplied(&mut c)
            .on_hover_text(if themed {
                "Cor do tema da interface (Configurações do projeto). Mude para usar outra só aqui."
            } else {
                "Cor só deste componente"
            })
            .changed()
        {
            out = Some(Prop::Str(rgba_to_hex(c)));
        }
        if themed {
            ui.weak("do tema");
        } else if ui
            .small_button("↺ tema")
            .on_hover_text("Voltar a usar a cor do tema da interface")
            .clicked()
        {
            out = Some(Prop::Str(String::new()));
        }
    });
    out
}

/// Variável do jogo: escrever o nome ou escolher entre as do projeto.
fn var_field(ui: &mut egui::Ui, current: &Prop, width: f32) -> Option<Prop> {
    let mut v = current.str().to_string();
    let mut out = None;
    let vars = crate::scene_data::var_names();
    ui.horizontal(|ui| {
        let r = ui.add(
            egui::TextEdit::singleline(&mut v)
                .hint_text("(nenhuma)")
                .desired_width((width - 30.0).max(60.0)),
        );
        if r.changed() {
            out = Some(Prop::Str(v.trim().to_string()));
        }
        ui.menu_button("…", |ui| {
            if ui.button("(nenhuma)").clicked() {
                out = Some(Prop::Str(String::new()));
                ui.close();
            }
            if vars.is_empty() {
                ui.weak("Nenhuma variável nas Configurações do projeto.");
            }
            for name in &vars {
                if ui.button(name).clicked() {
                    out = Some(Prop::Str(name.clone()));
                    ui.close();
                }
            }
        })
        .response
        .on_hover_text("Variáveis do jogo (Configurações do projeto)");
    });
    let shown = out
        .as_ref()
        .map(|p: &Prop| p.str().to_string())
        .unwrap_or_else(|| current.str().to_string());
    if !shown.is_empty() && !vars.contains(&shown) {
        ui.end_row();
        ui.label("");
        ui.colored_label(
            crate::theme::WARNING,
            "⚠ Não está nas variáveis do jogo: começa vazia.",
        );
    }
    out
}

/// Nome de animação: escrever ou escolher entre as da imagem.
fn anim_field(
    ui: &mut egui::Ui,
    spec: &Spec,
    current: &Prop,
    anims: &[String],
    salt: (u32, usize),
    width: f32,
) -> Option<Prop> {
    let mut v = current.str().to_string();
    let mut changed = false;
    ui.horizontal(|ui| {
        let w = if anims.is_empty() {
            width
        } else {
            width - 28.0
        };
        let r = ui.add(
            egui::TextEdit::singleline(&mut v)
                .desired_width(w.max(40.0))
                .hint_text(if anims.is_empty() { "" } else { "(nenhuma)" }),
        );
        changed |= r.changed();
        if spec.key == "animation" {
            crate::tutorial::anchor_in(ui, "inspetor.animacao", r.rect);
        }
        if !v.is_empty()
            && !anims.is_empty()
            && !anims.contains(&v)
            && !v.chars().next().is_some_and(|c| c.is_ascii_digit())
        {
            r.on_hover_text(format!(
                "A imagem não tem a animação \"{v}\". Tem: {}",
                anims.join(", ")
            ));
        }
        if !anims.is_empty() {
            ui.push_id(("anim", salt, &spec.key), |ui| {
                ui.menu_button("☰", |ui| {
                    if ui.button("(nenhuma)").clicked() {
                        v.clear();
                        changed = true;
                        ui.close();
                    }
                    for n in anims {
                        if ui.selectable_label(v == *n, n).clicked() {
                            v = n.clone();
                            changed = true;
                            ui.close();
                        }
                    }
                })
                .response
                .on_hover_text("Animações da imagem");
            });
        }
    });
    changed.then_some(Prop::Str(v))
}

/// A mão direita entre os ossos de um modelo: "mao_d" (os nossos), a do
/// Mixamo ("mixamorig:RightHand") ou um nome parecido.
pub fn hand_bone(bones: &[String]) -> Option<String> {
    let low = |b: &String| b.to_lowercase();
    bones
        .iter()
        .find(|b| low(b) == "mao_d")
        .or_else(|| bones.iter().find(|b| low(b).ends_with("righthand")))
        .or_else(|| {
            bones.iter().find(|b| {
                let l = low(b);
                (l.contains("hand") || l.contains("mao") || l.contains("mão"))
                    && (l.contains("right")
                        || l.ends_with("_r")
                        || l.ends_with(".r")
                        || l.ends_with("_d"))
            })
        })
        .cloned()
}

/// As listas que alguns campos oferecem: as animações (da imagem ou do modelo)
/// e os ossos de um modelo 3D.
#[derive(Debug, Clone, Copy)]
struct Lists<'a> {
    anims: &'a [String],
    bones: &'a [String],
}

/// Um osso de um modelo: a lista dos ossos (ou texto, se o editor não
/// conhece o modelo).
fn bone_field(
    ui: &mut egui::Ui,
    spec: &Spec,
    current: &Prop,
    bones: &[String],
    empty: &str,
    salt: (u32, usize),
    width: f32,
) -> Option<Prop> {
    let mut v = current.str().to_string();
    if bones.is_empty() {
        let r = ui.add(
            egui::TextEdit::singleline(&mut v)
                .desired_width(width)
                .hint_text(empty),
        );
        return r.changed().then_some(Prop::Str(v));
    }
    let mut changed = false;
    let r = egui::ComboBox::from_id_salt(("osso", salt, &spec.key))
        .width(width)
        .truncate()
        .selected_text(if v.is_empty() {
            empty.to_string()
        } else {
            v.clone()
        })
        .show_ui(ui, |ui| {
            changed |= ui.selectable_value(&mut v, String::new(), empty).changed();
            for b in bones {
                changed |= ui.selectable_value(&mut v, b.clone(), b).changed();
            }
        });
    if !v.is_empty() && !bones.contains(&v) {
        r.response.on_hover_text(format!(
            "O modelo não tem o osso \"{v}\". Tem: {}",
            bones.join(", ")
        ));
    }
    changed.then_some(Prop::Str(v))
}

/// Lista de arquivos, aceitando arrastar do painel Arquivos.
fn file_combo(
    ui: &mut egui::Ui,
    spec: &Spec,
    current: &Prop,
    list: &[String],
    salt: (u32, usize),
    empty: &str,
    accepts: impl Fn(&str) -> bool,
) -> Option<Prop> {
    let mut v = current.str().to_string();
    let mut changed = false;
    // Na caixa, só o nome do arquivo (o caminho todo fica na dica).
    let short = v.rsplit('/').next().unwrap_or(&v).to_string();
    let r = egui::ComboBox::from_id_salt(("arquivo", salt, &spec.key))
        .selected_text(if v.is_empty() {
            empty.to_string()
        } else {
            short
        })
        .width(ui.available_width().clamp(80.0, 180.0))
        .truncate()
        .show_ui(ui, |ui| {
            changed |= ui.selectable_value(&mut v, String::new(), empty).changed();
            for rel in list {
                changed |= ui.selectable_value(&mut v, rel.clone(), rel).changed();
            }
        })
        .response
        .on_hover_text(if v.is_empty() {
            "Escolha na lista ou arraste do painel Arquivos para cá.".to_string()
        } else {
            format!("{v}\nEscolha na lista ou arraste do painel Arquivos para cá.")
        });
    if let Some(p) = r.dnd_release_payload::<FileDrag>()
        && accepts(&p.0)
    {
        v = p.0.clone();
        changed = true;
    }
    changed.then_some(Prop::Str(v))
}

/// O componente com os valores do começo, mas sem mudar o que diz o que ele é
/// (o arquivo, a forma, o tipo de luz, o script...).
pub fn default_component(
    comp: &Component,
    three_d: bool,
    info: Option<&BehaviorInfo>,
) -> Component {
    let mut fresh = if comp.is_behavior() {
        match info {
            Some(info) => Component::behavior(comp.str("script"), &info.defaults_for(three_d)),
            None => return comp.clone(),
        }
    } else if comp.kind == "particles" && three_d {
        crate::particle_presets::component_3d()
    } else {
        Component::new(&comp.kind)
    };
    for key in [
        "script", "shape", "file", "image", "sound", "map", "kind", "hud", "bone", "model",
    ] {
        if let Some(v) = comp.get(key) {
            fresh.set(key, v.clone());
        }
    }
    fresh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todos_os_campos_existem_na_engine() {
        // Cada campo do inspetor precisa existir no esquema da engine (e
        // todo campo da engine aparece no inspetor).
        let schema = crate::scene_data::schema();
        for kind in &schema.order {
            let engine: Vec<&str> = schema
                .fields(kind)
                .unwrap()
                .iter()
                .map(|(k, _)| k.as_str())
                .collect();
            let ours: Vec<&str> = fields(kind)
                .iter()
                .map(|f| f.key)
                .filter(|k| !k.is_empty())
                .collect();
            for k in &ours {
                assert!(engine.contains(k), "{kind}.{k} não existe na engine");
            }
            for k in &engine {
                // As alturas do terreno são pintadas na vista (✏ Pintar
                // terreno), não editadas no inspetor.
                if (kind.as_str(), *k) == ("terrain", "data") {
                    continue;
                }
                assert!(ours.contains(k), "{kind}.{k} não aparece no inspetor");
            }
            assert_ne!(component_info(kind).0, "Componente");
        }
    }

    #[test]
    fn cores_hex() {
        assert_eq!(rgba_to_hex(hex_to_rgba("#12AB34")), "#12AB34");
        assert_eq!(rgba_to_hex(hex_to_rgba("#12AB3480")), "#12AB3480");
    }
}
