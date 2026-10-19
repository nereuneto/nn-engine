# nn-engine

Engine de jogos 2D (e, em construção, 3D) em Rust, construída sobre [wgpu](https://wgpu.rs) e [winit](https://github.com/rust-windowing/winit). Os jogos podem ser escritos em **Lua** ou em **Rust**.

Plataformas suportadas: **Windows** e **Linux**.

## Baixar

Para fazer jogos não é preciso instalar Rust nem compilar nada: baixe o **NN Studio** pronto na página da [última versão](https://github.com/nereuneto/nn-engine/releases/latest).

| Sistema | Arquivo | Como usar |
|---|---|---|
| Windows 10 ou 11 | `NN-Studio-<versão>-setup.exe` | Instalador: não pede senha de administrador; cria atalhos no menu Iniciar (e, se quiser, na área de trabalho) e aparece em *Aplicativos instalados* para desinstalar. |
| Windows (portátil) | `NN-Studio-<versão>-windows-portatil.zip` | Extraia numa pasta e abra o `nn-studio.exe`. |
| Linux x86_64 | `NN-Studio-<versão>-x86_64.AppImage` | Um arquivo só: marque como executável (`chmod +x`) e abra. |
| Linux (pasta) | `NN-Studio-<versão>-linux-x86_64.tar.gz` | Extraia e rode o `nn-studio`. |

Junto com o editor vêm o `nn` (ferramenta de linha de comando) e o `nn-jogo` (o executável que vira o seu jogo exportado).

**Requisitos:** no Windows, uma placa de vídeo com DirectX 12 ou Vulkan; no Linux, com Vulkan (os drivers atuais da Intel, AMD e NVIDIA têm). No Linux, o áudio sai pelo ALSA (ou pelo PipeWire/PulseAudio, que o atendem).

**"O Windows protegeu o computador":** o NN Studio ainda não tem assinatura digital (um certificado pago), então o Windows avisa na primeira vez que o instalador abre. Clique em **Mais informações** e depois em **Executar assim mesmo**. Os arquivos são gerados pelo [workflow público](.github/workflows/runtimes.yml) a partir deste repositório.

O NN Studio avisa quando sai uma versão nova (ele consulta esta página no máximo uma vez por dia e nunca baixa nada sozinho; dá para desligar na tela inicial do NN Studio). O que mudou em cada versão está no [CHANGELOG](CHANGELOG.md).

## Licença

O código da nn-engine e do NN Studio pode ser usado sob a licença [MIT](LICENSE-MIT) ou a [Apache-2.0](LICENSE-APACHE), à sua escolha. As imagens e os sons dos modelos e exemplos seguem a mesma licença. Os jogos que você faz com a nn-engine são seus.

## Começando pelo código

Para compilar a engine e o editor (ou fazer jogos em Rust), o pré-requisito é o Rust instalado pelo [rustup](https://rustup.rs). A engine pede o Rust 1.89 ou mais novo; o editor (NN Studio), o 1.95. Com o rustup, `rustup update` deixa tudo em dia.

**Windows:** baixe e rode o `rustup-init.exe` do site acima. Se o instalador avisar que faltam as ferramentas de build do Visual Studio (o compilador C++ da Microsoft), escolha a opção de instalação rápida que ele oferece. Depois **abra um terminal novo** para o `cargo` aparecer no PATH e confira com `cargo --version`.

**Linux:** `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`. O áudio usa ALSA e os controles usam udev, então instale também os cabeçalhos de desenvolvimento: `sudo apt install libasound2-dev libudev-dev pkg-config` (Debian/Ubuntu) ou `sudo dnf install alsa-lib-devel systemd-devel` (Fedora).

A primeira compilação demora alguns minutos (o wgpu é grande); as seguintes são rápidas.

### NN Studio: o editor

```sh
cargo studio
```

![NN Studio](docs/imagens/nn-studio.png)

O NN Studio é o jeito mais fácil de fazer um jogo: crie um projeto a partir de um modelo (ou abra um dos exemplos), edite o código Lua com cores e erros apontados enquanto digita, veja imagens, sons e mapas, arraste arquivos do computador para dentro do projeto e aperte **▶ Jogar (F5)**: o jogo abre na janela dele e, a cada arquivo salvo, a mudança aparece na hora. O console mostra o `print` do jogo e os erros em português, com link direto para a linha. O editor de código tem autocompletar da API `nn.*` (Ctrl+Espaço), com a descrição de cada função, os parâmetros enquanto você digita e os objetos das suas cenas. Tem modo Simples e Completo, tema escuro e claro, paleta de comandos (Ctrl+Shift+P) e exportação do jogo pronto para distribuir.

Fases e telas também podem ser montadas com o mouse, em **cenas visuais** (`.cena`): arraste imagens, mapas e sons para a cena, mova, gire e escale com as alças, organize os objetos na Hierarquia e ajuste tudo no Inspetor (sprite, texto, forma, corpo físico, mapa, câmera, som, partículas), com desfazer e refazer. **Comportamentos prontos** dão vida aos objetos **sem código**: jogador de plataforma ou top-down, vida, patrulha, perseguir, perigo, coletável, checkpoint, ir para outra cena, movimento automático, animação, atirar, gerador, botão de menu e placar com as **variáveis do jogo** (moedas, vidas...) fixo na tela. **Prefabs** guardam um objeto pronto para usar em várias cenas (mude o prefab e todas as cópias mudam). **Eventos "Quando... Então..."** juntam tudo sem código, lidos como frases (*Quando o jogador encostar na porta e a variável chaves for ≥ 1, então...*), com **Ver como código** para quem quiser aprender Lua; com o jogo aberto, cada regra acende quando dispara. **▶ Testar daqui (F6)** abre o jogo direto na cena e, com ele aberto, o que você muda no editor aparece no jogo na hora; no caminho de volta, o editor mostra onde cada objeto está no jogo, pausa e avança quadro a quadro, e Ctrl+clique no jogo seleciona o objeto no editor. Para o que os prontos não cobrem, escreva seus próprios comportamentos em Lua, com propriedades que aparecem no Inspetor, ou um script para a cena. Imagens se desenham no próprio editor: o **editor de pixel art** tem lápis, borracha, balde, linhas, formas, seleção, desenho espelhado, paletas, quadros e papel vegetal, e o **editor de animação** monta animações com nome (*parado*, *andar*, *pular*) numa linha do tempo, com prévia; elas ficam num `.anim` ao lado da imagem e tocam na cena, na Animação automática, nos eventos e no Lua. Uma imagem também pode **repetir** em vez de esticar (chão, paredes). Fases de tiles se **pintam direto na cena**: pincel, balde, retângulo, conta-gotas, tiles girados e espelhados, camadas e **terrenos que se encaixam sozinhos** (autotile de 16 peças, com um molde para desenhar os seus), com o painel Tiles para marcar o que bloqueia, as plataformas, o tipo de cada tile (lava, água...) e as animações; o que você pinta aparece no jogo aberto na hora, e os arquivos são os do Tiled (`.tmj`/`.tsj`). **Menus e HUD** se montam com componentes de interface (painel, botão, barra, ícones, deslizador, caixa de marcar, campo de texto e um grupo que arruma os filhos), que funcionam com mouse, teclado e controle sem código, ficam presos aos cantos da tela e seguem o **tema** do projeto (cores, fonte `.ttf` e som); uma cena pode abrir **por cima** de outra (pausa, opções), com a de baixo parada, e os volumes e a tela cheia escolhidos ficam guardados. **Partículas** animam na própria cena do editor, com dez efeitos prontos (fogo, fumaça, faíscas, explosão, poeira, brilho, bolhas, chuva, neve, confete), uma seta para a direção e explosões em prefab que estouram e somem sozinhas. Para aprender fazendo, os **tutoriais guiados** destacam cada botão e avançam sozinhos (*Seu primeiro jogo*, em uns 15 minutos, *Seu primeiro mundo 3D*, *Efeitos no 3D*, *Pinte um terreno*, *Eventos*, *Desenhe seu herói*, *Monte uma fase com tiles*, *Monte um menu* e *Efeitos com partículas*), e os modelos **Plataforma**, **Aventura** e **Nave** são jogos completos sem código. Cenas também podem ser **3D**: uma vista em perspectiva com céu, sol, névoa e sombras, navegação como na Unity e no Godot, modelos `.glb` arrastados para a cena, alças 3D e o **Ambiente**; o **Corpo 3D** faz chão, paredes, caixas que caem e plataformas que levam o herói, o **Jogador 3D** anda, pula e leva a câmera em terceira pessoa, personagens `.glb` com esqueleto (do Blender ou do Mixamo) se animam com a **Animação 3D**, partículas e sons ficam no mundo 3D (fogo, chuva, um som mais alto de perto), **terrenos** se pintam com o mouse (morros, lagos, cores, árvores espalhadas), e coletáveis, plataformas que andam e os eventos funcionam no 3D sem código (tutorial guiado *Seu primeiro mundo 3D* e o modelo **3D**). O jogo pode ter um **tamanho fixo** (por exemplo 1280 × 720) que se ajusta a qualquer janela. Guia completo em **[docs/studio.md](docs/studio.md)**.

![Cena visual no NN Studio](docs/imagens/nn-studio-cena.png)

Tudo o que o NN Studio faz também dá para fazer pela linha de comando, com a ferramenta `nn`:

### Criar um jogo novo

```sh
cargo nn novo ../meu-jogo   # cria um jogo pequeno e completo, pronto para mudar
cargo nn ../meu-jogo        # joga
```

O modelo traz menu, partida, pausa, tela de fim e recorde salvo, já organizado do jeito recomendado (cenas, ações de controle, menus). `--titulo "Meu Jogo"` escolhe o título; `--modelo visual`, `--modelo aventura` e `--modelo nave` criam jogos montados como cenas visuais, sem nenhuma linha de código; `--modelo 3d` cria um jogo de plataforma 3D em terceira pessoa em código, e `--modelo visual3d` o mesmo jogo montado no NN Studio, sem código; `--modelo vale3d` cria um vale pintado num terreno; `--modelo vazio` cria uma cena vazia com imagens e sons; e `--modelo minimo` cria só uma cena. Detalhes em [docs/lua.md](docs/lua.md#começando).

### Jogos de demonstração em Lua

```sh
cargo nn jogos/plataforma   # plataforma: 2 fases do Tiled, inimigos, fundo em camadas, menus, recordes
cargo nn jogos/demo         # menu, coletar moedas e pausa
cargo nn jogos/cubos3d      # 3D: modelos .glb, física, sombras, céu e câmera que gira com o mouse
cargo nn jogos/estrelas3d   # 3D em terceira pessoa: ache as 5 estrelas da ilha
```

Todos são feitos inteiramente em Lua. O de plataforma tem duas fases (a segunda é liberada ao completar a primeira), inimigos que patrulham e perseguem (pise em cima para derrotar), funciona com teclado, mouse (nos menus) e **controle** (Xbox, PlayStation, Switch Pro e genéricos), e guarda as opções, as fases liberadas e os recordes de cada fase.

### Exportar um jogo

```sh
cargo nn exportar jogos/plataforma
```

Gera um **executável único** em `dist/`, com a engine e todos os arquivos do jogo dentro, pronto para distribuir: quem for jogar não precisa de Rust nem da pasta do projeto. Detalhes em [docs/lua.md](docs/lua.md#distribuindo-o-jogo). Com o jogo aberto, edite qualquer arquivo de `jogos/demo/cenas/` e salve: a mudança aparece na hora. A referência completa da API Lua está em **[docs/lua.md](docs/lua.md)**.

### Exemplos em Rust

```sh
cargo run --example coletor      # moedas, animação, texto e áudio
cargo run --example movimento    # o básico: andar com WASD
```

| Exemplo | O que mostra | Controles |
|---|---|---|
| `coletor` | Spritesheets e animação, HUD com texto, texto no mundo, efeitos sonoros e música | WASD/setas ou controle, Shift/RB corre, M música, R recomeça, Esc sai |
| `movimento` | Timestep fixo com interpolação, câmera que segue, zoom | WASD/setas ou controle, Shift/RB corre, roda do mouse dá zoom, Esc sai |

Use `RUST_LOG=debug` para ver mais detalhes no terminal (GPU escolhida, avisos do wgpu).

## Um jogo mínimo em Lua

Crie uma pasta com um `main.lua`:

```lua
local cena = {}

function cena:enter()
  self.x, self.y = 0, 0
end

function cena:fixed_update(dt)
  local dx, dy = nn.input.wasd()
  self.x = self.x + dx * 200 * dt
  self.y = self.y + dy * 200 * dt
end

function cena:draw()
  nn.draw.rect(self.x, self.y, 50, 50, "#3FA7D6")
  nn.screen.text("Use WASD", 16, 16)
end

return cena
```

E rode com `cargo nn caminho/da/pasta`.

## Um jogo mínimo em Rust

```rust,no_run
use nn_engine::prelude::*;

struct MeuJogo {
    posicao: Vec2,
}

impl Game for MeuJogo {
    // Lógica em passos fixos (60 por segundo por padrão).
    fn fixed_update(&mut self, ctx: &mut Context) {
        // WASD, setas, direcional ou analógico do controle.
        self.posicao += ctx.input.move_vector() * 200.0 * ctx.fixed_dt();
    }

    fn render(&mut self, ctx: &mut Context) {
        ctx.renderer
            .draw_rect(self.posicao, Vec2::splat(50.0), Color::hex(0x3FA7D6));
    }
}

fn main() -> Result<(), AppError> {
    // A função recebe o contexto com a GPU pronta: carregue texturas nela.
    nn_engine::run(AppConfig::new("Meu jogo"), |_ctx| MeuJogo {
        posicao: Vec2::ZERO,
    })
}
```

## Arquitetura

O projeto é um workspace Cargo dividido em crates com responsabilidades separadas:

| Crate | Responsabilidade |
|---|---|
| `nn-core` | Tipos base: `Color`, `Time`, `FixedTimestep` |
| `nn-input` | Estado de teclado, mouse e controles (`key_down`, `pad_pressed`, `move_vector()`...) e ações com nome |
| `nn-render` | GPU, texturas, câmera 2D, sprites em lote, spritesheets, animação, texto, tamanho fixo do jogo ajustado à janela e o mundo 3D (`d3`: malhas, modelos glTF, câmera em perspectiva, sol com sombras, céu e névoa) |
| `nn-audio` | Efeitos sonoros e música (sobre o [kira](https://github.com/tesselode/kira)) com volume por canal, sintetizador simples |
| `nn-app` | Janela, loop principal, trait `Game`, cenas, assets, interface (menus), dados salvos, leitura dos controles ([gilrs](https://gitlab.com/gilrs-project/gilrs)) e modo sem janela (`Headless`) para testes |
| `nn-physics` | Física arcade: corpos retangulares e circulares, grade de tiles, plataformas de mão única, sensores, camadas, raycast |
| `nn-physics3d` | Física 3D sobre o [rapier](https://rapier.rs): corpos que caem e empilham, personagem que anda e pula, sensores, raios |
| `nn-tilemap` | Tilemaps com camadas, tilesets e objetos; importação do editor Tiled (JSON) |
| `nn-script` | Scripting em Lua: API `nn.*` (com a documentação de cada função em `api_docs`), cenas em Lua e cenas visuais (`.cena`), comportamentos prontos, eventos "Quando... Então...", prefabs, variáveis do jogo, recarga ao salvar, canal ao vivo com o editor, tela de erro |
| `nn-run` | Ferramenta `nn` (cria, roda e exporta projetos Lua, com os modelos de `nn novo`) e `nn-jogo`, o executável dos jogos exportados |
| `nn-engine` (raiz) | Fachada que reexporta tudo, com `prelude` |

### Loop principal

A cada frame a engine mede o tempo real e executa:

1. `fixed_update` zero ou mais vezes, sempre com o mesmo passo (`ctx.fixed_dt()`). A simulação fica determinística e igual em máquinas rápidas e lentas.
2. `update` uma vez.
3. `render`, que pode usar `ctx.alpha()` para interpolar entre o passo anterior e o atual e deixar o movimento suave em qualquer taxa de quadros.

Regra prática para entrada: `key_down` (segurando) funciona em qualquer lugar; transições como `key_pressed` (apertou neste frame) devem ser lidas em `update`, porque `fixed_update` pode rodar zero ou várias vezes no mesmo frame.

### Renderização

- Coordenadas do mundo com **Y para cima**; com `zoom = 1.0`, 1 unidade = 1 pixel.
- Sprites são desenhados na ordem em que são enviados: o último fica por cima.
- Sprites consecutivos com a mesma textura viram uma única chamada de desenho (instancing). `ctx.renderer.last_stats()` mostra quantas chamadas foram feitas.
- Cores e texturas em sRGB; a mistura de transparência é feita em espaço linear.
- `TextureOptions::PIXEL_ART` deixa a textura nítida ao ampliar.

### Mundo e tela

Há dois espaços de desenho:

- **Mundo**: `ctx.renderer.draw(...)`, `draw_rect(...)`, `draw_text(...)`. Vistos pela câmera, Y para cima.
- **Tela** (HUD, menus): `ctx.renderer.screen().draw_text(...)` e afins. Pixels da janela, origem no canto superior esquerdo, Y para baixo. Não é afetado pela câmera e é sempre desenhado por cima do mundo.

### Animação

```rust,ignore
// Na criação do jogo:
let sheet = ctx.renderer.sprite_sheet(textura, UVec2::splat(16)); // quadros de 16×16
let andar = Animation::new(4..8, 10.0);          // quadros 4 a 7, 10 por segundo
let piscar = Animation::new([8, 9, 8], 6.0).once();
let mut anim = AnimationPlayer::new(andar.clone());
// Duração própria para cada quadro: Animation::new([0, 1, 2], 10.0).with_times([0.1, 0.5, 0.1])

// A cada frame:
anim.play(&andar);                    // não reinicia se já estiver tocando
anim.update(ctx.time.delta_secs());
ctx.renderer.draw(&sheet.sprite(anim.frame()).at(posicao));
```

Modos: `Loop` (padrão), `once()` e `ping_pong()`.

### Texto

```rust,ignore
ctx.renderer.screen().draw_text(
    &Text::new("Vidas: 3").at(Vec2::new(16.0, 16.0)).size(24.0).color(Color::WHITE),
);
ctx.renderer.draw_text(&Text::new("Baú").at(posicao_do_bau).anchor(Anchor::BottomCenter));
let tamanho = ctx.renderer.measure_text(&Text::new("Pausado").size(48.0));
```

A fonte padrão é a Poppins (licença SIL OFL 1.1, em `crates/nn-render/assets/fonts`). Outras fontes .ttf/.otf podem ser carregadas com `ctx.renderer.load_font(caminho)`. Cada letra é rasterizada uma vez por tamanho e reaproveitada, então texto que muda todo frame (como um placar) é barato.

## Cenas

Jogos com várias telas usam uma pilha de cenas. Só a cena do topo recebe `update`; cenas marcadas como overlay deixam a de baixo visível (pausa, janelas).

```rust,ignore
struct Menu;
impl Scene for Menu {
    fn update(&mut self, ctx: &mut Context) -> Transition {
        if ctx.input.key_pressed(KeyCode::Enter) {
            return Transition::Replace(Box::new(Fase::new(ctx)));
        }
        Transition::None
    }
    fn render(&mut self, ctx: &mut Context) { /* ... */ }
}

nn_engine::run(AppConfig::new("Jogo"), |ctx| SceneStack::new(ctx, Menu))
```

Transições: `Push` (abre por cima), `Pop` (volta), `Replace` (troca a atual), `Switch` (fecha todas e abre uma) e `Quit`. Em Lua, o mesmo vale com `nn.scene.push/pop/replace/switch`.

Qualquer transição pode escurecer a tela, trocar com ela coberta e clarear de novo: `Transition::Switch(Box::new(Fase::new(ctx))).fade(0.6)` (ou `.fade_with(segundos, cor)`). Enquanto escurece, a cena que sai fica parada e os menus não respondem.

## Efeitos de tela

```rust,ignore
ctx.shake(16.0, 0.4);                                  // tremor: pixels, segundos
ctx.flash(Color::hex(0xFF3030).with_alpha(0.4), 0.3);  // flash que some aos poucos
```

O tremor só move o mundo (o HUD e os menus ficam parados) e não muda `ctx.camera.position`. Vários tremores somam e diminuem até parar.

## Controles e ações

Controles funcionam sem configuração e podem ser conectados com o jogo aberto. Dá para consultar botões e eixos diretamente ou, melhor, dar nome às ações e ligar teclado e controle a elas:

```rust,ignore
ctx.input.bind("pular", [
    Binding::Key(KeyCode::Space),
    Binding::Pad(PadButton::South),          // A no Xbox, X no PlayStation
]);
ctx.input.bind("direita", [
    Binding::Key(KeyCode::KeyD),
    Binding::PadAxis(PadAxis::LeftX, AxisDir::Positive),
]);

if ctx.input.action_pressed("pular") { /* ... */ }
let andar = ctx.input.move_vector();         // WASD, setas, direcional e analógico juntos
let gatilho = ctx.input.pad_axis(PadAxis::RightTrigger);
```

`ctx.input.last_device()` diz se o jogador está no teclado, no mouse ou no controle, para mostrar a dica certa na tela. No Linux, compilar precisa do `libudev-dev`; para compilar sem suporte a controles, desligue a feature `gamepad` do `nn-app`.

## Interface (menus)

Painéis com botões, barras e opções, montados a cada frame no `render`. Funcionam com mouse, teclado e controle sem código extra (foco com setas/direcional, confirmar com Enter/A, voltar com Esc/B):

```rust,ignore
let mut menu = ctx.panel("principal", PanelOptions::new().title("Meu jogo"));
if menu.button("Jogar") {
    // ...
}
menu.slider("Música", &mut self.musica, 0.0, 1.0, 0.1);
menu.toggle("Tela cheia", &mut self.tela_cheia);
if menu.button("Sair") || menu.back() {
    menu.ctx().exit();
}
menu.finish();
```

Na pilha de cenas, só a cena do topo responde à interface, e um painel ignora o aperto que o abriu (o Esc que abriu a pausa não fecha a pausa). A aparência fica em `ctx.ui.theme_mut()` e acompanha a altura da janela. Em Lua: [`nn.ui`](docs/lua.md#nnui).

## Dados salvos

```rust,ignore
let recorde = ctx.save.get_as::<u32>("recorde").unwrap_or(0);
if pontos > recorde {
    ctx.save.set("recorde", pontos);
}
ctx.set_fullscreen(true);
ctx.audio.set_channel_volume(Channel::Music, 0.5);
```

`ctx.save` guarda um `save.json` na pasta de dados do usuário (`~/.local/share/<id>/` no Linux, `%APPDATA%\<id>\data\` no Windows; o id vem de `AppConfig::id` ou do título). A engine grava sozinha, no máximo uma vez por segundo e ao fechar, de forma atômica (uma gravação interrompida não estraga o arquivo anterior). Em Lua: [`nn.save`](docs/lua.md#nnsave).

## Física e tilemaps

```rust,ignore
use nn_engine::physics::{Body, Shape, World};
use nn_engine::tilemap::{load_tiled, read_from};

let mut mapa = load_tiled("fases/fase1.tmj".as_ref(), read_from("assets"), 3.0)?;
mapa.load_textures(|p| ctx.load_texture(p, TextureOptions::PIXEL_ART))?;

let mut mundo = World::new(Vec2::new(0.0, -2400.0));
mundo.add_grid(mapa.collision_grid());
let heroi = mundo.add(Body::dynamic(Shape::rect(30.0, 42.0)).at(Vec2::new(100.0, 300.0)));

// fixed_update:
for evento in mundo.step(ctx.fixed_dt()) { /* sensores: Enter / Exit */ }
// render:
mapa.draw(&mut ctx.renderer, &ctx.camera, ctx.viewport().as_vec2());
```

Tiles animados (água, lava, bandeiras) feitos no editor de animação do Tiled vêm junto ao carregar o mapa; em Rust, avance o relógio deles com `mapa.update(dt)` uma vez por frame (em Lua é automático). Em mapas feitos por código, `mapa.animate(gid, &[quadros], fps)`.

Para inimigos que atravessam o jogador sem empurrá-lo, use camadas que não interagem (`Body::with_layer`) e confira o contato com `mundo.intersects(a, b)`, que ignora camadas.

A física é do tipo "arcade": corpos não giram, a resolução é por eixo (sem enroscar na junção de tiles), o chão é detectado direto (`on_ground`) e há subpassos para objetos rápidos não atravessarem paredes. Corpos cinemáticos carregam quem está em cima. Mapas do Tiled devem ser salvos em JSON (`.tmj`) com camadas no formato CSV; a referência completa está em [docs/lua.md](docs/lua.md#nntilemap).

## Câmera e fundos em camadas

```rust,ignore
// Uma vez por frame: segue o jogador, suave e sem sair do mapa.
let seguir = Follow {
    offset: Vec2::new(90.0, 80.0),        // olha um pouco à frente
    deadzone: Vec2::new(0.0, 120.0),      // pular não balança a tela
    bounds: Some(mapa.bounds()),
    ..Default::default()
};
ctx.camera.follow(jogador, &seguir, ctx.time.delta_secs(), ctx.viewport().as_vec2());

// Fundo: montanhas que andam a 20% da velocidade do mundo, repetidas sem fim.
let montanhas = Parallax::new(textura, Vec2::new(1024.0, 384.0))
    .factor(Vec2::new(0.2, 1.0))
    .at(Vec2::new(0.0, 280.0));
montanhas.draw(&mut ctx.renderer, &ctx.camera, ctx.viewport().as_vec2());
```

A suavização da câmera é igual em qualquer taxa de quadros. Num eixo em que o mapa é menor que a tela, a câmera fica centralizada nele. Em Lua: `nn.camera.follow` e `nn.draw.parallax` ([docs/lua.md](docs/lua.md#fundos-em-camadas-parallax)).

## Partículas

```rust,ignore
let mut faiscas = ParticleEmitter::new(ParticleConfig {
    lifetime: (0.3, 0.6),
    speed: (120.0, 280.0),
    drag: 4.0,
    size: (14.0, 0.0),
    colors: vec![Color::WHITE, Color::hex(0xFFE27A), Color::hex(0xFF9A30).with_alpha(0.0)],
    blend: Blend::Add,
    ..Default::default()
});
faiscas.burst_at(18, posicao);
faiscas.update(dt);
faiscas.draw(&mut ctx.renderer);
```

Qualquer sprite pode usar `Blend::Add` (mistura aditiva) para efeitos de luz.

## Assets e recarga ao salvar

Assets ficam numa pasta (`assets/` por padrão, configurável em `AppConfig::assets_dir`), procurada a partir da pasta atual e depois ao lado do executável.

```rust,ignore
let heroi = ctx.load_texture("imagens/heroi.png", TextureOptions::PIXEL_ART)?;
let pulo = ctx.load_sound("sons/pulo.ogg")?;
```

Cada arquivo é carregado uma vez (pedir de novo devolve o mesmo id). Num jogo exportado, os mesmos caminhos são lidos do pacote dentro do executável, sem mudar nada no código. Em builds de desenvolvimento, imagens, sons e scripts Lua alterados no disco são recarregados sozinhos, mantendo os ids. Para desligar: `AppConfig::hot_reload(false)`.

## Áudio

```rust,ignore
let pulo = ctx.audio.load("assets/pulo.ogg")?;          // WAV, OGG Vorbis ou MP3
ctx.audio.play(pulo);
let mut musica = ctx.audio.play_with(trilha, PlayOptions::default().looped().channel(Channel::Music));
musica.fade_out(Duration::from_secs(1));
ctx.audio.set_channel_volume(Channel::Music, 0.4);       // também afeta o que já está tocando
```

Sem dispositivo de som, a engine segue em modo mudo em vez de falhar. Para protótipos, `nn_engine::audio::synth` gera efeitos por código (ondas senoidal, quadrada, triangular, dente de serra e ruído, com deslize de tom e envelope).

## Testes

```sh
cargo test --workspace
```

Os testes de renderização desenham em uma textura fora da tela e conferem os pixels. Para gerar imagens de referência e conferir a olho: `cargo test -p nn-render --test captura -- --ignored` (salva em `capturas/`). Sem GPU disponível eles são pulados; com `NN_REQUIRE_GPU=1` passam a ser obrigatórios. No CI do Linux usamos o driver Vulkan por software (lavapipe), então esses testes rodam em todo PR.
