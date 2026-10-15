# nn-engine

Engine de jogos 2D em Rust, construída sobre [wgpu](https://wgpu.rs) e [winit](https://github.com/rust-windowing/winit).

Plataformas suportadas: **Windows** e **Linux**.

## Começando

Pré-requisito: Rust 1.88 ou mais recente ([rustup.rs](https://rustup.rs)).

```sh
cargo run --example movimento
```

Controles do exemplo: **WASD** ou setas para andar, **Shift** para correr, **roda do mouse** para zoom, **Esc** para sair.

Use `RUST_LOG=debug` para ver mais detalhes no terminal (GPU escolhida, avisos do wgpu).

## Um jogo mínimo

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
| `nn-render` | GPU, texturas, câmera 2D e desenho de sprites em lote |
| `nn-app` | Janela, loop principal e a trait `Game` |
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

## Testes

```sh
cargo test --workspace
```

Os testes de renderização desenham em uma textura fora da tela e conferem os pixels. Sem GPU disponível eles são pulados; com `NN_REQUIRE_GPU=1` passam a ser obrigatórios. No CI do Linux usamos o driver Vulkan por software (lavapipe), então esses testes rodam em todo PR.
