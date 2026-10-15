# nn-engine

Engine de jogos 2D em Rust, construída sobre [wgpu](https://wgpu.rs) e [winit](https://github.com/rust-windowing/winit). Os jogos podem ser escritos em **Lua** ou em **Rust**.

Plataformas suportadas: **Windows** e **Linux**.

## Começando

Pré-requisito: Rust 1.88 ou mais recente, instalado pelo [rustup](https://rustup.rs).

**Windows:** baixe e rode o `rustup-init.exe` do site acima. Se o instalador avisar que faltam as ferramentas de build do Visual Studio (o compilador C++ da Microsoft), escolha a opção de instalação rápida que ele oferece. Depois **abra um terminal novo** para o `cargo` aparecer no PATH e confira com `cargo --version`.

**Linux:** `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`. O áudio usa ALSA, então instale também os cabeçalhos de desenvolvimento: `sudo apt install libasound2-dev pkg-config` (Debian/Ubuntu) ou `sudo dnf install alsa-lib-devel` (Fedora).

A primeira compilação demora alguns minutos (o wgpu é grande); as seguintes são rápidas.

### Jogo de demonstração em Lua

```sh
cargo nn jogos/demo
```

Menu, jogo e pausa, todos em Lua (`jogos/demo`). Com o jogo aberto, edite qualquer arquivo de `jogos/demo/cenas/` e salve: a mudança aparece na hora. A referência completa da API Lua está em **[docs/lua.md](docs/lua.md)**.

### Exemplos em Rust

```sh
cargo run --example coletor      # moedas, animação, texto e áudio
cargo run --example movimento    # o básico: andar com WASD
```

| Exemplo | O que mostra | Controles |
|---|---|---|
| `coletor` | Spritesheets e animação, HUD com texto, texto no mundo, efeitos sonoros e música | WASD/setas, Shift corre, M música, R recomeça, Esc sai |
| `movimento` | Timestep fixo com interpolação, câmera que segue, zoom | WASD/setas, Shift corre, roda do mouse dá zoom, Esc sai |

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
        self.posicao += ctx.input.wasd() * 200.0 * ctx.fixed_dt();
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
| `nn-input` | Estado de teclado e mouse (`key_down`, `key_pressed`, `wasd()`...) |
| `nn-render` | GPU, texturas, câmera 2D, sprites em lote, spritesheets, animação e texto |
| `nn-audio` | Efeitos sonoros e música (sobre o [kira](https://github.com/tesselode/kira)), sintetizador simples |
| `nn-app` | Janela, loop principal, trait `Game`, cenas, pasta de assets e modo sem janela (`Headless`) para testes |
| `nn-script` | Scripting em Lua: API `nn.*`, cenas em Lua, recarga ao salvar, tela de erro |
| `nn-run` | Executável `nn`, que roda um projeto Lua (`cargo nn <pasta>`) |
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

## Assets e recarga ao salvar

Assets ficam numa pasta (`assets/` por padrão, configurável em `AppConfig::assets_dir`), procurada a partir da pasta atual e depois ao lado do executável.

```rust,ignore
let heroi = ctx.load_texture("imagens/heroi.png", TextureOptions::PIXEL_ART)?;
let pulo = ctx.load_sound("sons/pulo.ogg")?;
```

Cada arquivo é carregado uma vez (pedir de novo devolve o mesmo id). Em builds de desenvolvimento, imagens, sons e scripts Lua alterados no disco são recarregados sozinhos, mantendo os ids. Para desligar: `AppConfig::hot_reload(false)`.

## Áudio

```rust,ignore
let pulo = ctx.audio.load("assets/pulo.ogg")?;          // WAV, OGG Vorbis ou MP3
ctx.audio.play(pulo);
let mut musica = ctx.audio.play_with(trilha, PlayOptions::default().looped().volume(0.4));
musica.fade_out(Duration::from_secs(1));
```

Sem dispositivo de som, a engine segue em modo mudo em vez de falhar. Para protótipos, `nn_engine::audio::synth` gera efeitos por código (ondas senoidal, quadrada, triangular, dente de serra e ruído, com deslize de tom e envelope).

## Testes

```sh
cargo test --workspace
```

Os testes de renderização desenham em uma textura fora da tela e conferem os pixels. Para gerar imagens de referência e conferir a olho: `cargo test -p nn-render --test captura -- --ignored` (salva em `capturas/`). Sem GPU disponível eles são pulados; com `NN_REQUIRE_GPU=1` passam a ser obrigatórios. No CI do Linux usamos o driver Vulkan por software (lavapipe), então esses testes rodam em todo PR.
