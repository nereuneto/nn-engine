# Estudo: a nn-engine no navegador (S67)

Pergunta: dá para rodar os jogos da nn-engine no navegador, quanto custa e a
web fica no plano (v0.13)?

**Resposta curta: dá, e a recomendação é manter a web na v0.13.** O desenho
2D e 3D, o teclado, o mouse e o som já rodam no navegador com o código da
engine (WebGL2 e WebGPU). O Lua 5.4 que usamos hoje (em C) também roda lá,
passando nos testes de compatibilidade, sem trocar de linguagem nem de
versão. O que falta é encaixar as partes: o laço da janela, os arquivos do
jogo, os saves e a exportação. A estimativa é de 4 sprints.

Os experimentos ficam no ramo `estudo-web` (pasta `estudo-web/`). O workflow
`estudo-web.yml` roda tudo no GitHub Actions a cada push e grava os
resultados em `estudo-web/resultados/`. Lá estão os números abaixo, as fotos
do navegador e o console de cada teste.

## Como foi testado

- **E0**: cada crate da engine compilada, sem mudanças, para
  `wasm32-unknown-unknown` (o alvo do winit, do wgpu e do web-sys) e para
  `wasm32-unknown-emscripten`.
- **E1**: uma prova (`estudo-web/prova`) com o nn-render (retângulos, imagem,
  texto e um cubo 3D com céu), o teclado (setas), o mouse e o som do nn-audio
  (kira + WebAudio). O Chromium a abre sem tela (Playwright, com placa de
  vídeo por software), aperta as teclas, clica e tira fotos. Foi testada em
  WebGL2 e em WebGPU.
- **E3**: as opções de Lua. O mesmo `teste.lua` (20 testes do que os jogos
  usam do Lua 5.4) e o mesmo `bench.lua` (2000 objetos andando por 300
  quadros, recursão e texto) rodaram em cada opção.

## O que já funciona

| | WebGL2 | WebGPU |
|---|---|---|
| Janela (canvas), redimensionar | sim | sim |
| Desenho 2D: retângulos, imagens, texto | sim | sim (com a vista sRGB, veja abaixo) |
| 3D: malhas, luz, céu | sim | sim, depois das correções |
| Teclado e mouse | sim | sim |
| Som (nn-audio no WebAudio) | sim (AudioContext "running", 48 kHz) | sim |
| Pronto para desenhar | ~0,3 s depois de baixar | ~0,1 s |

A prova com 2000 imagens a mais por quadro gasta ~1,7 ms de CPU por quadro no
navegador. A placa de vídeo do teste é simulada (SwiftShader), então os
quadros por segundo de lá não valem para uma máquina de verdade.

### Crates (E0)

| crate | wasm32-unknown-unknown | emscripten |
|---|---|---|
| nn-core, nn-render, nn-audio, nn-physics, nn-physics3d, nn-nav, nn-tilemap | compilam | compilam |
| nn-input | compila | não (o winit não tem emscripten) |
| nn-app | compila (depois do relógio novo, abaixo) | não (winit) |
| nn-script, nn-projeto, nn-run | não: o Lua em C (veja o Lua) | não (winit) |

O emscripten fica descartado: o winit não roda nele, e a engine teria de ter
uma camada própria de janela e entrada só para isso.

## O Lua

| opção | testes (de 20) | bench | tamanho (brotli) | observação |
|---|---|---|---|---|
| mlua no desktop (base) | 20 | 47 ms | — | o de hoje |
| **mlua no wasm32-unknown-unknown + libc do wasi-sdk** | 19 | 124 ms | 140 KB | **o caminho recomendado** |
| mlua no emscripten | 20 | 122 ms | 138 KB | mas o resto da engine não roda no emscripten |
| mlua no wasm32-wasip1 (wasmtime) | 19 | 75 ms | 143 KB | fora do navegador; só para comparar |
| piccolo (Lua em Rust puro) | 6 | 1095 ms | 165 KB | descartado: falta muito e é 9x mais lento |
| mlua no wasm32-unknown-unknown puro | — | — | — | não compila (o lua-src não conhece o alvo) |

O caminho recomendado mantém o mesmo Lua 5.4 e o mesmo mlua. Ele funciona
assim:

- O Lua em C é compilado com o clang e a libc do wasi-sdk. Para isso, o
  `lua-src` precisa de um patch de uma linha, que aceita o alvo
  `wasm32-unknown-unknown` como se fosse WASI.
- O que a libc pede ao "sistema" (relógio, sorteio, escrever no console)
  vira import do wasm. A página fornece isso com umas 50 linhas de JS
  (`estudo-web/lua-mlua-web.mjs` faz isso à mão, igual seria na página).
- Os erros do Lua (`setjmp`/`longjmp`) usam o tratamento de exceções do
  WebAssembly, que os navegadores atuais têm. As versões mínimas ainda
  precisam ser conferidas na v0.13.
- O único teste que falhou é `os.time`/`os.clock` ("not supported on
  WASI"). É só ligar ao relógio da página.
- No navegador, o Lua fica ~2,6x mais lento que no desktop. É parecido com o
  emscripten e aceitável para a lógica dos jogos: o pesado é o desenho e a
  física, que são Rust.

## Tamanho do download

A prova (render 2D e 3D, texto, som e janela), depois do `wasm-opt -Oz`:

| | wasm | brotli |
|---|---|---|
| sem som | 2,5 MB | 820 KB |
| com som | 2,6 MB | 844 KB |
| + o Lua (estimado) | | +140 KB |

Pelas medidas, um jogo completo (com o nn-script, a física e o Lua) deve
ficar entre 1,3 e 1,6 MB comprimido, mais os arquivos do jogo. O maior pedaço
é o **naga** (~600 KB antes de comprimir). Ele traduz os shaders de WGSL para
GLSL no navegador, porque o WebGL2 não lê WGSL. Ideias para a v0.13:

- Traduzir os shaders na exportação. Só vale se o wgpu deixar passar o GLSL
  pronto.
- Um build só WebGPU, que dispensa o naga e o tradutor de GLSL. Mas o WebGPU
  ainda não está em todo navegador.
- Tirar o decodificador de JPEG (~110 KB) quando o jogo não tiver JPEG.

## Correções que já foram para o main

São correções genéricas, sem mudança no desktop: as 52 fotos dos jogos e
modelos saíram iguais byte a byte às de antes.

- **Limites da GPU**: onde a placa não tem os limites comuns (WebGL2, OpenGL
  ES 3.0), a engine pede os do WebGL2. O renderer só usa uniforms.
- **Relógio**: o nn-render e o nn-app usam o `web-time`. No desktop ele é o
  próprio `std::time`; no navegador, o `std::time::Instant` trava o jogo.
- **3D no WebGL2**: o brilho (segundo alvo) mistura igual à cor quando a
  placa não deixa misturar cada alvo de um jeito.
- **3D no WebGPU**: três ajustes.
  - A sombra com ossos usa grupos vazios em vez de buracos na lista.
  - O céu estrelado usa o `smoothstep` na ordem que o WGSL do navegador
    aceita.
  - A água amostra a textura fora do desvio, porque o WGSL do navegador só
    deixa amostrar em controle uniforme.

## O que falta (estimativa para a v0.13)

1. **Laço da janela no navegador** (nn-app), cerca de 1 sprint:
   - o `spawn_app` do winit no lugar do `run_app`;
   - a GPU iniciada sem `pollster` (assíncrona);
   - o tamanho e a densidade do canvas e a tela cheia;
   - a vista sRGB no canvas do WebGPU, que não tem formato sRGB e deixa as
     cores escuras sem ela (a prova já faz isso).
2. **Arquivos e saves**, cerca de 1 sprint:
   - o jogo exportado como um pacote baixado com `fetch`. A engine já lê
     jogos de um pacote (`AssetSource`), então basta uma origem em memória;
   - os saves no `localStorage`/IndexedDB;
   - a recarga ao vivo desligada.
3. **Lua no navegador** (nn-script), cerca de 1 sprint:
   - o `lua-src` com o patch (manter a cópia ou propor ao projeto);
   - o shim WASI em JS e o `os.time`/`os.clock`;
   - o vigia de travamento sem thread: o contador de instruções do Lua só
     na web, ou desligado.
4. **Exportar para a web e testar**, cerca de 1 sprint:
   - "Exportar para a web" no Studio e no MCP, que gera o `index.html`, o
     wasm otimizado e o pacote;
   - o som que só liga depois do primeiro clique (regra dos navegadores);
   - o controle (gamepad);
   - um teste no Chromium sem tela no CI, como o deste estudo;
   - a medida com jogos de verdade.

**Total: ~4 sprints de trabalho, mais a revisão.** Os riscos:

- o patch do `lua-src` (precisa ser mantido até entrar no projeto);
- as versões mínimas dos navegadores para as exceções do WebAssembly;
- o WebGPU, que ainda pede opções no Linux. O WebGL2 cobre esses casos.
