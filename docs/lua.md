# Jogos em Lua

A nn-engine roda jogos escritos inteiramente em Lua (5.4). Não é preciso instalar Lua: ela vem compilada dentro da engine.

## Começando

```sh
cargo nn jogos/demo         # menu, coletar moedas, pausa
cargo nn jogos/plataforma   # plataforma com mapa do Tiled e física
```

`cargo nn <pasta>` roda o projeto Lua da pasta. Durante o desenvolvimento, **salve qualquer `.lua`, imagem ou som e o jogo atualiza na hora**, sem fechar a janela e sem perder o estado. Se um script tiver erro, ele aparece na própria janela com o arquivo e a linha; corrija, salve e o jogo continua.

### Estrutura de um projeto

```
meu-jogo/
├── config.lua          (opcional) título, tamanho da janela, cena inicial
├── main.lua            cena inicial (ou a indicada em config.lua)
├── cenas/
│   ├── jogo.lua
│   └── pausa.lua
├── lib/
│   └── util.lua        módulos: local util = require("lib.util")
└── assets/
    ├── imagens/
    └── sons/
```

Todos os caminhos (`nn.assets.*`, `nn.scene.*`, `require`) são relativos à pasta do projeto.

### config.lua

```lua
return {
  title = "Meu jogo",
  width = 1280,           -- tamanho inicial da janela
  height = 720,
  start = "main.lua",     -- cena inicial
  clear_color = "#1A1F1C",
  vsync = true,
  fixed_hz = 60,          -- passos de fixed_update por segundo
  resizable = true,
  hot_reload = true,      -- recarregar ao salvar
}
```

## Distribuindo o jogo

```sh
cargo nn exportar jogos/plataforma
```

Gera `dist/plataforma` (ou `dist/plataforma.exe` no Windows): **um único arquivo** com a engine e todos os arquivos do projeto dentro. Quem for jogar só precisa dele: não precisa de Rust, Lua, nem da pasta do projeto. No Windows, o jogo abre sem janela de console.

Antes de empacotar, a sintaxe de todos os `.lua` é verificada; com erro, nada é gerado e o erro aparece com arquivo e linha. A primeira exportação compila a versão otimizada da engine e demora alguns minutos; as próximas são rápidas.

Opções:

| Opção | Descrição |
|---|---|
| `--saida <pasta>` | Onde criar o executável (padrão: `dist`). |
| `--nome <nome>` | Nome do arquivo (padrão: nome da pasta do projeto). |
| `--runtime <arquivo>` | Executável `nn-jogo` a usar. Para gerar o jogo de Windows estando no Linux (ou o contrário), baixe o `nn-jogo` do outro sistema nas *releases* do repositório e passe aqui. |

Se o jogo exportado não abrir (por exemplo, por falta de suporte de vídeo), ele deixa um arquivo `<nome>-erro.txt` ao lado do executável explicando o motivo.

## Cenas

Cada cena é um arquivo que devolve uma tabela. Todas as funções são opcionais e recebem `self`, a tabela de estado da cena.

```lua
local cena = {}

function cena:enter(args)     -- a cena começou; args vem de nn.scene.push/replace/switch
  self.x, self.y = 0, 0
  self.heroi = nn.assets.texture("assets/imagens/heroi.png", { pixel_art = true })
end

function cena:fixed_update(dt) -- passos fixos (física, movimento); dt é sempre o mesmo
  local dx, dy = nn.input.wasd()
  self.x = self.x + dx * 200 * dt
  self.y = self.y + dy * 200 * dt
end

function cena:update(dt)       -- uma vez por frame (menus, teclas apertadas, animações)
  if nn.input.pressed("escape") then nn.scene.push("cenas/pausa.lua") end
end

function cena:draw()           -- desenho
  nn.draw.sprite(self.heroi, self.x, self.y)
  nn.screen.text("Olá!", 16, 16, { size = 24 })
end

function cena:exit() end       -- a cena saiu da pilha
function cena:resume() end     -- a cena voltou ao topo (a de cima foi fechada)
function cena:reload() end     -- o arquivo foi salvo e recarregado

return cena
```

Coloque `overlay = true` na tabela para a cena de baixo continuar desenhada por trás (pausas, janelas):

```lua
local pausa = { overlay = true }
```

**Dica:** use `nn.input.down` (tecla segurada) em `fixed_update` e `nn.input.pressed` (apertou agora) em `update`. O `fixed_update` pode rodar zero ou várias vezes no mesmo frame.

**Recarga:** ao salvar, as funções da cena são trocadas mas `self` é mantido. O `enter` não roda de novo; se precisar refazer algo, use `reload`.

## Referência da API

Convenções:

- **Mundo** (`nn.draw.*`): coordenadas do jogo, vistas pela câmera, **Y para cima**. `x, y` é o centro do sprite ou retângulo.
- **Tela** (`nn.screen.*`): pixels da janela, origem no canto superior esquerdo, **Y para baixo**, por cima do mundo e sem câmera. `x, y` é o canto superior esquerdo.
- **Cores** podem ser `"#RRGGBB"`, `"#RRGGBBAA"`, `0xRRGGBB` ou tabelas `{r, g, b, a}` com valores de 0 a 1.
- **Quadros** de spritesheets e animações começam em **1**.
- Ângulos em radianos.

### nn.draw / nn.screen

As duas tabelas têm as mesmas funções; muda só o espaço.

| Função | Descrição |
|---|---|
| `sprite(img, x, y, opts)` | Desenha uma imagem ou um quadro de spritesheet. |
| `rect(x, y, w, h, cor)` | Retângulo de cor sólida. |
| `text(texto, x, y, opts)` | Texto. |
| `circle(x, y, raio, cor)` | Círculo de cor sólida (`x, y` é o centro nos dois espaços). |

Opções de `sprite`: `frame` (para spritesheets), `w`, `h`, `scale`, `rotation`, `origin_x`, `origin_y` (0 a 1; padrão 0.5 no mundo e 0 na tela), `color` (tinge), `alpha`, `flip_x`, `flip_y`.

Opções de `text`: `size` (padrão 20), `color`, `alpha`, `anchor`, `align`, `font`.
`anchor` diz qual ponto do texto fica em `x, y`: `topleft` (padrão), `top`, `topright`, `left`, `center`, `right`, `bottomleft`, `bottom`, `bottomright`.
`align` alinha as linhas de um texto com `\n`: `left`, `center`, `right`.

`nn.text_size(texto, opts)` devolve `largura, altura` sem desenhar.

### nn.input

| Função | Descrição |
|---|---|
| `down(tecla)` | Tecla segurada. |
| `pressed(tecla)` | Apertou neste frame. |
| `released(tecla)` | Soltou neste frame. |
| `wasd()` | Direção `dx, dy` (Y para cima), normalizada. |
| `arrows()` | O mesmo, com as setas. |
| `axis(neg, pos)` | -1, 0 ou 1 a partir de duas teclas. |
| `mouse()` | Posição do cursor na tela (`nil` fora da janela). |
| `mouse_world()` | Posição do cursor no mundo. |
| `mouse_down(b)`, `mouse_pressed(b)`, `mouse_released(b)` | Botão `"left"` (padrão), `"right"` ou `"middle"`. |
| `scroll()` | Rolagem do mouse neste frame. |

Nomes de teclas: `"a"` a `"z"`, `"0"` a `"9"`, `"space"`, `"enter"`, `"escape"`, `"tab"`, `"backspace"`, `"delete"`, `"shift"`, `"ctrl"`, `"alt"`, `"left"`, `"right"`, `"up"`, `"down"`, `"f1"` a `"f12"`.

### nn.assets

| Função | Devolve |
|---|---|
| `texture(caminho, { pixel_art = true })` | Imagem PNG, com campos `width` e `height`. |
| `sheet(caminho_ou_imagem, largura_quadro, altura_quadro, opts)` | Spritesheet, com `count`, `columns`, `rows`. Opções: `margin`, `spacing`, `pixel_art`. |
| `sound(caminho)` | Som (WAV, OGG Vorbis, MP3), com `duration` e `:play(opts)`. |
| `font(caminho)` | Fonte .ttf/.otf, para a opção `font` do texto. |
| `read(caminho)` | Conteúdo de um arquivo de texto. |

Cada arquivo é carregado uma vez; pedir de novo devolve o mesmo. Imagens e sons alterados no disco são recarregados sozinhos.

### nn.audio

`nn.audio.play(som, opts)` toca e devolve o som tocando. Opções: `volume` (1 = normal), `speed` (2 = uma oitava acima), `pan` (-1 esquerda a 1 direita), `loop`.

O som tocando tem `:stop()`, `:fade_out(segundos)`, `:pause()`, `:resume()`, `:set_volume(v)`, `:set_speed(v)`, `:set_pan(v)` e `:playing()`.

`nn.audio.set_volume(v)` e `nn.audio.volume()` controlam o volume geral.

### Animação

```lua
local andar = nn.animation({ 5, 6, 7, 8 }, 10)          -- quadros, quadros por segundo
local girar = nn.animation(nn.frames(1, 4), 9, "pingpong") -- modos: "loop" (padrão), "once", "pingpong"
local anim = nn.animator(andar)

-- a cada frame:
anim:play(andar)          -- troca de animação; não reinicia se já for a mesma
anim:update(dt)
nn.draw.sprite(sheet, x, y, { frame = anim:frame() })
```

O animador tem `:frame()`, `:finished()`, `:restart()` e os campos `speed` (multiplicador) e `time`. A animação tem `duration`, `length` e `mode`.

### nn.physics

Física arcade: corpos não giram, as colisões são previsíveis e o chão é detectado direto. Ideal para plataforma, top-down e afins.

```lua
local mundo = nn.physics.world({ gravity = -2400 })   -- gravidade em Y (negativa = cai)

local chao = mundo:body({ type = "static", x = 0, y = -10, w = 800, h = 20 })
local heroi = mundo:body({ x = 0, y = 100, w = 30, h = 42, tag = "heroi" })
local moeda = mundo:body({ type = "static", x = 200, y = 40, radius = 16, sensor = true, tag = "moeda" })

function cena:fixed_update(dt)
  heroi.vx = nn.input.axis("left", "right") * 300
  if heroi.on_ground and nn.input.down("space") then heroi.vy = 900 end

  for _, e in ipairs(mundo:step(dt)) do      -- avança a física e devolve os eventos
    if e.type == "enter" and e.a.tag == "moeda" and e.b == heroi then
      e.a:remove()
    end
  end
end

function cena:draw()
  mundo:debug_draw()                          -- mostra os corpos (ótimo para ajustar colisões)
end
```

Opções de `mundo:body`:

| Opção | Descrição |
|---|---|
| `type` | `"dynamic"` (padrão: cai e colide), `"static"` (parede, chão) ou `"kinematic"` (move-se pela velocidade e carrega quem está em cima: plataformas móveis, elevadores). |
| `x`, `y` | Centro do corpo. |
| `w`, `h` ou `radius` | Retângulo ou círculo. |
| `vx`, `vy` | Velocidade inicial. |
| `gravity_scale` | Multiplicador da gravidade (0 = flutua). |
| `sensor` | Não colide, só avisa quando algo entra ou sai (moedas, espinhos, portas). |
| `one_way` | Plataforma de mão única: só bloqueia quem vem de cima. |
| `bounce` | 0 = para ao bater, 1 = quica sem perder velocidade. |
| `layer`, `mask` | Camadas (bits): dois corpos interagem se a camada de um estiver na máscara do outro. |
| `tag` | Rótulo livre. |

O corpo tem os campos `x`, `y`, `vx`, `vy`, `gravity_scale`, `bounce`, `sensor`, `layer`, `mask` e `tag` (leitura e escrita), `id`, `type`, `width`, `height`, `radius`, `alive`, `on_ground`, `touching_left`, `touching_right`, `touching_top`, `touching_bottom` e `ground` (o corpo em que está apoiado). Métodos: `:set_position(x, y)`, `:set_velocity(x, y)`, `:overlaps()` (corpos encostando agora) e `:remove()`. Dá para comparar corpos com `==`.

O mundo tem `:step(dt)` (chame em `fixed_update`), `:body(opts)`, `:add_tilemap(mapa)`, `:raycast(x, y, dx, dy, distancia, mascara)` (devolve `{ x, y, nx, ny, distance, body }` ou `nil`), `:query_point(x, y)`, `:query_rect(x, y, w, h)`, `:bodies(tag)`, `:debug_draw()` e os campos `gravity_x`, `gravity_y` e `count`.

Os eventos de `step` são tabelas `{ type = "enter" | "exit", a = corpo, b = corpo }`. Pelo menos um dos dois é sensor; se só um for, ele vem em `a`.

### nn.tilemap

Mapas de tiles, feitos no editor [Tiled](https://www.mapeditor.org) ou escritos direto em Lua.

**Do Tiled** (salve como JSON, `.tmj`; tilesets embutidos ou `.tsj`):

```lua
local mapa = nn.tilemap.load("fases/fase1.tmj", { scale = 3 })   -- tiles de 16 px viram 48
mundo:add_tilemap(mapa)                                            -- colisão dos tiles

local inicio = mapa:object("inicio")                               -- objeto pelo nome
for _, m in ipairs(mapa:objects("moeda")) do ... end               -- objetos pelo tipo/classe

function cena:draw()
  mapa:draw()                     -- todas as camadas visíveis
  -- ou camada por camada, para desenhar coisas entre elas:
  mapa:draw_layer("fundo")
end
```

No Tiled, marque nos tiles do tileset a propriedade booleana `solid` (bloqueia) ou `one_way` (plataforma de mão única). Uma camada com a propriedade `collision = true` torna todos os seus tiles sólidos (bom para uma camada invisível só de colisão). Objetos trazem `name`, `type` (o tipo ou a classe), `x`, `y` (centro, já em coordenadas do mundo), `w`, `h`, `point`, `layer` e `properties`.

Opções de `load`: `scale`, `x`, `y` (posição do canto inferior esquerdo do mapa) e `pixel_art` (padrão `true`).

**Em Lua**, com uma linha de texto por fileira:

```lua
local mapa = nn.tilemap.new({
  sheet = nn.assets.sheet("assets/tiles.png", 16, 16, { pixel_art = true }),
  tile_size = 48,
  rows = {
    "..........",
    "..C....C..",
    ".J..====..",
    "##########",
  },
  legend = {
    ["#"] = { tile = 1, solid = true },
    ["="] = { tile = 4, one_way = true },
    ["C"] = { object = "moeda" },
    ["J"] = { object = "jogador" },
  },
})
```

Outros métodos e campos: `width`, `height` (em tiles), `tile_width`, `tile_height`, `world_width`, `world_height`, `layers`, `properties`, `:tile(col, lin, camada)`, `:set_tile(col, lin, tile, camada)` (a colisão acompanha: ótimo para blocos quebráveis), `:set_solid(col, lin, "solid" | "one_way" | nil)`, `:solid(col, lin)`, `:cell_at(x, y)` e `:cell_center(col, lin)`. Colunas e linhas começam em 1, com a linha 1 no topo, como no editor.

### nn.particles

```lua
local brilho = nn.particles({
  emitting = false,                         -- só solta com :burst
  lifetime = { 0.3, 0.6 },                  -- vida em segundos (mínimo, máximo)
  speed = { 120, 280 },
  drag = 4,                                 -- freia com o tempo
  size = { 14, 0 },                         -- tamanho no início e no fim
  colors = { "#FFFFFF", "#FFE27A", "#FF9A3000" },  -- gradiente ao longo da vida
  blend = "add",                            -- soma as cores: efeito de luz
})

brilho:burst(18, moeda.x, moeda.y)          -- solta 18 de uma vez

local fumaca = nn.particles({ rate = 20, direction = math.pi / 2, spread = 0.6, gravity = 40 })
fumaca:set_position(chamine.x, chamine.y)   -- emite sozinho, 20 por segundo

function cena:update(dt) brilho:update(dt); fumaca:update(dt) end
function cena:draw() brilho:draw(); fumaca:draw() end
```

| Opção | Descrição |
|---|---|
| `rate` | Partículas por segundo enquanto `emitting` for verdadeiro. |
| `emitting` | Começa emitindo (padrão `true`). |
| `lifetime`, `speed`, `size`, `spin` | Número ou faixa `{mínimo, máximo}` (`size` é `{início, fim}`). |
| `direction`, `spread` | Direção central e abertura total, em radianos (padrão: para cima, em todas as direções). |
| `gravity` | Número (Y) ou `{x, y}`. |
| `drag` | Perda de velocidade por segundo. |
| `size_variation` | Variação aleatória do tamanho (0.3 = até 30%). |
| `colors` ou `color` | Lista de cores ao longo da vida, ou uma cor que some aos poucos. |
| `texture`, `frame` | Imagem ou quadro de spritesheet (padrão: círculo suave). |
| `area` | Onde nascem: `{ w =, h = }` ou `{ radius = }` (padrão: um ponto). |
| `blend` | `"alpha"` (padrão) ou `"add"` (luz, fogo, faíscas). |
| `max` | Limite de partículas vivas (padrão 1000). |

O emissor tem `x`, `y`, `emitting` e `rate` (leitura e escrita), `count` e `finished`, e os métodos `:burst(n, x, y)`, `:set_position(x, y)`, `:update(dt)`, `:draw()`, `:draw_screen()` e `:clear()`.

### nn.camera

`position()`, `set_position(x, y)`, `zoom()`, `set_zoom(z)`, `rotation()`, `set_rotation(r)`, `to_world(sx, sy)`, `to_screen(wx, wy)`.

### nn.scene

| Função | Efeito |
|---|---|
| `push(caminho, args)` | Abre a cena por cima da atual. |
| `pop()` | Fecha a cena atual e volta para a de baixo. |
| `replace(caminho, args)` | Troca a cena atual. |
| `switch(caminho, args)` | Fecha todas e abre esta (ex.: voltar ao menu). |
| `nn.quit()` | Fecha o jogo. |

A troca acontece ao final do `update`.

### nn.time e nn.window

`nn.time.delta()`, `elapsed()`, `fixed_delta()`, `alpha()` (fração entre passos fixos, para interpolar posições no `draw`), `frame()`.

`nn.window.size()` devolve `largura, altura`; `nn.window.set_clear_color(cor)` muda a cor de fundo.

### Outros

`print(...)` escreve no terminal (log da engine). `nn.version` é a versão da engine. Variáveis globais são compartilhadas entre as cenas e sobrevivem à recarga.
