# Jogos em Lua

A nn-engine roda jogos escritos inteiramente em Lua (5.4). Não é preciso instalar Lua: ela vem compilada dentro da engine.

## Começando

Para criar um jogo novo, pronto para rodar:

```sh
cargo nn novo ../meu-jogo     # cria a pasta com um jogo pequeno e completo
cargo nn ../meu-jogo          # joga
```

O modelo padrão já vem com menu, partida, pausa, tela de fim e recorde salvo, usando o jeito recomendado de organizar um jogo; é só ir mudando. Opções: `--titulo "Meu Jogo"` e `--modelo`, que escolhe o modelo: `visual` (fase de plataforma montada no [NN Studio](studio.md), sem código, com [comportamentos](#comportamentos), [eventos](#eventos-quando-então) e um [prefab](#prefabs)), `aventura` (visto de cima: chave, porta e tesouro), `nave` (tiros, asteroides e recorde salvo), `3d` (plataforma 3D em terceira pessoa, veja [Seu primeiro jogo 3D](#seu-primeiro-jogo-3d)), `vazio` (cena vazia com um pacote de imagens e sons) ou `minimo` (só uma cena com um quadrado que anda). A pasta não pode existir com arquivos dentro. Com o `nn` instalado (sem o código da engine), o comando é `nn novo meu-jogo`.

Para ver jogos prontos:

```sh
cargo nn jogos/demo         # menu, coletar moedas, pausa
cargo nn jogos/plataforma   # plataforma com mapa do Tiled e física
cargo nn jogos/cubos3d      # 3D: modelos, física, sombras e céu
cargo nn jogos/estrelas3d   # 3D em terceira pessoa: ache as 5 estrelas da ilha
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
    ├── imagens/        heroi.png e, ao lado, heroi.anim (as animações dela)
    └── sons/
```

Todos os caminhos (`nn.assets.*`, `nn.scene.*`, `require`) são relativos à pasta do projeto.

### config.lua

```lua
return {
  title = "Meu jogo",
  id = "meu-jogo",        -- nome da pasta onde o progresso é salvo (padrão: o título)
  width = 1280,           -- tamanho inicial da janela
  height = 720,
  resolution = { 1280, 720 }, -- tamanho fixo do jogo (opcional, veja abaixo)
  scale = "fit",          -- como ele se ajusta à janela: "fit", "stretch" ou "pixel"
  fullscreen = false,     -- começar em tela cheia
  start = "main.lua",     -- cena inicial
  clear_color = "#1A1F1C",
  vsync = true,
  fixed_hz = 60,          -- passos de fixed_update por segundo
  resizable = true,
  hot_reload = true,      -- recarregar ao salvar
  vars = {                -- variáveis do jogo (nn.vars)
    moedas = 0,
    vidas = 3,
  },
  theme = {               -- tema da interface (menus e HUD das cenas visuais)
    button = "#E76F51",
    font = "assets/fontes/minha.ttf",
  },
}
```

Todas as opções são opcionais. As `vars` viram as [variáveis do jogo](#variáveis-do-jogo-nnvars).

**Tema da interface** (`theme`): as cores que os [componentes de interface](#interface-menus-e-hud) usam quando a cor deles está vazia: `text` (#FFFFFF), `button` (#3FA7D6), `hover` (#5BC0EB, botão com foco), `press` (#2E86AB, apertado), `disabled` (#5B6472), `panel` (#1E2A3AE6), `border` (#FFFFFF40) e `accent` (#FFD166, barras e marcas), além de `font` (um `.ttf` ou `.otf`; vazio = a fonte da engine) e `sound` (som dos botões). Só o que você muda precisa estar no arquivo. No código, o tema completo fica em `nn.theme`.

**Tamanho fixo do jogo** (`resolution`): sem ele, o jogo enxerga o tamanho da janela (janela maior mostra mais coisa). Com `resolution = { largura, altura }`, a câmera, `nn.screen`, `nn.window.size()`, `nn.camera.view()` e o mouse usam sempre esse tamanho, e a imagem se ajusta à janela conforme `scale`: `"fit"` (padrão) aumenta ou diminui mantendo a proporção, com faixas pretas quando a janela tem outra proporção; `"stretch"` ocupa a janela toda, mesmo distorcendo; `"pixel"` só aumenta em números inteiros (2×, 3×...), para pixel art nítida. O texto é desenhado no tamanho real da tela, então continua nítido. Os modelos visuais vêm com `resolution = { 1280, 720 }`. Rodando da pasta do projeto (`nn` ou ▶ Jogar no NN Studio), o jogo recarrega ao salvar, a não ser que o `config.lua` diga `hot_reload = false`; o jogo exportado não recarrega. O NN Studio também edita estas opções num formulário (**Editar > Configurações do projeto**), sem mexer nos comentários do arquivo.

As mensagens de erro do Lua aparecem em português ("tentou chamar um valor nil (vazio) (método 'pular')"), na tela de erro do jogo e no console do NN Studio.

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

**Dica:** use `nn.input.down` (tecla segurada) em `fixed_update` e `nn.input.pressed` (apertou agora) em `update`. O `fixed_update` pode rodar zero ou várias vezes no mesmo frame. Para o jogo funcionar com teclado e controle ao mesmo tempo, use [ações](#ações).

**Recarga:** ao salvar, as funções da cena são trocadas mas `self` é mantido. O `enter` não roda de novo; se precisar refazer algo, use `reload`.

### Cenas visuais (.cena)

Uma cena também pode ser um arquivo `.cena`, montado com o mouse no [NN Studio](studio.md#cenas-visuais). Ela é usada como qualquer cena: `start = "cenas/fase.cena"` no `config.lua`, `nn.scene.switch("cenas/fase.cena")`, `nn.scene.push(...)`. A engine cria os objetos, carrega as imagens e os sons, monta a física, segue a câmera e desenha tudo sozinha.

O arquivo é uma tabela Lua legível, com um objeto por bloco e um componente por linha:

```lua
return {
  format = 1,
  script = "cenas/fase.lua",     -- opcional: o código da cena
  background = "#5FB3E0",        -- opcional: cor de fundo (senão, a do config.lua)
  gravity = -1800,               -- gravidade da física (negativa = cai)
  overlay = false,               -- abre por cima de outra cena (pausa, opções)
  objects = {
    {
      id = 4, name = "heroi",
      x = 0, y = -180, rotation = 0, scale_x = 1, scale_y = 1,
      visible = true, layer = 2,
      components = {
        { type = "sprite", image = "assets/imagens/heroi.png", frame_w = 16, frame_h = 16, frame = 1, w = 64, h = 64 },
        { type = "body", body = "dynamic", shape = "box", w = 40, h = 60 },
      },
      children = { ... },          -- filhos andam, giram e escalam junto
    },
  },
}
```

Cada objeto tem `id` (único na cena), `name`, `x`, `y`, `rotation` (em graus), `scale_x`, `scale_y`, `visible`, `locked` (só para o editor), `layer` (camada maior é desenhada por cima), `anchor` (num objeto da raiz que fica na tela: o ponto da tela de onde a posição conta, veja abaixo), `components` e `children`. Campos que faltam usam o valor padrão. Os componentes:

| `type` | Campos (padrão) |
|---|---|
| `sprite` | `image`, `frame_w`, `frame_h` (0 = imagem inteira, ou o tamanho do `.anim` da imagem), `frame` (1), `w`, `h` (0 = tamanho da imagem), `color` ("#FFFFFF"), `flip_x`, `flip_y`, `pixel_art` (true), `fill` ("stretch" ou "repeat": repete a imagem lado a lado; `w` e `h` são o tamanho de cada cópia e a escala do objeto diz quantas cabem), `animation` ("": uma [animação](#animações-anim) da imagem para tocar desde o começo) |
| `text` | `text`, `size` (32), `color`, `anchor` ("center", "left", "right"...), `font` ("": a do tema) |
| `shape` | `shape` ("rect" ou "circle"), `w`, `h` (64), `radius` (32), `color` |
| `body` | `body` ("dynamic", "static" ou "kinematic"), `shape` ("box" ou "circle"), `w`, `h` (32), `radius` (16), `sensor`, `one_way`, `bounce` (0), `gravity_scale` (1), `tag` (padrão: o nome do objeto) |
| `map` | `map` (arquivo .tmj), `scale` (1), `collision` (true) |
| (sprite, text, shape) | `hud` (false): fixo na tela, em vez de andar com o mundo. A posição conta a partir do centro da tela, ou do ponto escolhido em `anchor` (y para cima). |

A cena e cada objeto também podem ter `events`, as regras [Quando... Então...](#eventos-quando-então).

#### Interface: menus e HUD

Os componentes de interface ficam presos à tela (`hud = true`, o padrão deles). Cores vazias (`""`) usam o [tema](#configlua) do projeto.

| `type` | Campos (padrão) |
|---|---|
| `panel` | `w` (320), `h` (220), `color` ("": tema `panel`), `radius` (12), `border` (0), `border_color`, `image` (moldura em 9 fatias), `slice` (4: pixels dos cantos da imagem), `slice_scale` (4) |
| `button` | `text` ("Botão"), `size` (28), `w` (240), `h` (60), `color`, `hover_color`, `press_color`, `text_color`, `radius` (12), `border`, `border_color`, `image`, `slice`, `slice_scale`, `icon`, `font`, `action` ("nada", "cena", "por_cima", "fechar", "reiniciar", "sair"), `scene`, `sound`, `focus` (false: começa com o foco), `enabled` (true) |
| `bar` | `w` (240), `h` (24), `var` (variável do jogo), `value` (1), `max` (1), `max_var`, `color` ("": tema `accent`), `back_color`, `radius` (6), `border`, `border_color`, `direction` ("right", "left", "up", "down") |
| `icons` | `image`, `frame`, `frame_w`, `frame_h`, `var`, `count` (3, sem variável), `max` (0: só os que tem; com máximo, os que faltam ficam com `empty_color`), `size` (32), `spacing` (6), `color`, `empty_color`, `align` ("left", "center", "right"), `pixel_art` |
| `slider` | `w` (240), `h` (12), `target` ("variavel", "musica", "efeitos", "geral"), `var`, `value` (0.5), `min` (0), `max` (1), `step` (0: um décimo), `color`, `back_color`, `knob_color`, `knob` (28) |
| `checkbox` | `text` ("Opção"), `size` (26), `box` (36), `target` ("variavel" ou "tela_cheia"), `var`, `checked`, `color`, `check_color`, `text_color`, `font` |
| `textfield` | `text`, `placeholder` ("Digite aqui"), `var` (guarda o texto), `w` (320), `h` (56), `size` (26), `max` (20 letras), `color`, `text_color`, `border` (2), `border_color`, `radius` (8), `font` |
| `layout` | `arrange` ("vertical", "horizontal", "grid", "none"): arruma os filhos do objeto, com `spacing` (16), `columns` (2, na grade) e `align` ("start", "center", "end") |

**Preso na tela.** A posição de um objeto da raiz que fica na tela conta a partir do ponto `anchor`: `"top_left"`, `"top"`, `"top_right"`, `"left"`, `"center"` (padrão), `"right"`, `"bottom_left"`, `"bottom"` ou `"bottom_right"`, com y para cima. `{ anchor = "top_left", x = 40, y = -40 }` fica a 40 pixels do canto em qualquer tamanho de janela.

**Foco.** Botões, deslizadores, caixas de marcar e campos de texto recebem o foco com o mouse (passando por cima), as setas ou WASD (para o item mais perto naquela direção), Tab e o direcional do controle. Enter, espaço ou o A do controle apertam; num deslizador, esquerda e direita mudam o valor; num campo de texto, as letras vão para o texto (Backspace apaga). Um botão apertado manda a mensagem `clicou` ao objeto (a condição `clicar` dos eventos); um valor que muda dispara a condição `mudou`. Os volumes e a tela cheia escolhidos ficam guardados (chave `nn_opcoes` do [nn.save](#nnsave)) e valem de novo ao abrir o jogo.

**Cenas por cima.** Com `overlay = true`, ou aberta com `nn.scene.push(cena, { por_cima = true })` (a ação `por_cima`), a cena aparece por cima da de baixo, que continua desenhada e **parada** até ela fechar (`nn.scene.pop()`, a ação `fechar_cena` ou um botão `fechar`). O `background` dela vira um véu sobre a de baixo (use uma cor transparente, como `"#00000099"`), e os objetos ficam na tela.

No script, `obj.button`, `obj.bar`, `obj.slider`... são os componentes (`obj.button.enabled = false`, `obj.bar.max = 5`), e `self:open_over(cena)` e `self:close_over()` abrem e fecham cenas por cima.
| `camera` | `follow` (nome do objeto a seguir), `smooth` (6), `zoom` (1), `limit_to_map` (true) |
| `sound` | `sound`, `autoplay` (true), `loop`, `volume` (1), `channel` ("effects" ou "music") |
| `behavior` | `script` (`"nn:plataforma"` para um [pronto](#comportamentos-prontos) ou o caminho de um `.lua`) e as propriedades dele, soltas: `{ type = "behavior", script = "nn:patrulha", velocidade = 60 }` |
| `particles` | `mode` ("continuo" ou "explosao"), `rate` (20 por segundo), `emitting` (true), `amount` (30, de uma vez), `on_start` (true: solta ao começar), `interval` (0: repetir a explosão a cada N s), `when_done` ("nada" ou "destruir": o objeto some quando as partículas acabam), `lifetime` (1), `lifetime_var` (0 a 1), `speed_min`, `speed_max`, `direction` (90, em graus), `spread` (360), `gravity` (negativa cai), `drag`, `area` ("ponto", "retangulo", "circulo"), `area_w`, `area_h`, `area_radius`, `size_start`, `size_end`, `size_var`, `color_start`, `color_mid` ("" = sem), `color_end`, `image`, `frame`, `frame_w`, `frame_h`, `spin` (graus por segundo), `blend` ("alpha" ou "add"), `follow` (false: as partículas acompanham o objeto), `max` (500) |

**O script da cena** é um arquivo `.lua` que devolve uma tabela com os callbacks de sempre (`enter`, `fixed_update`, `update`, `draw`, `exit`, `resume`, `reload`), chamados depois do que a engine faz sozinha. Os objetos já existem quando o `enter` roda:

| No script | O que é |
|---|---|
| `self.objects.nome` | O objeto com esse nome (o primeiro, se houver vários). |
| `self:find(nome)`, `self:find_all(nome)` | Um objeto / a lista de todos com o nome. |
| `obj.x`, `obj.y`, `obj.rotation`, `obj.scale_x`, `obj.scale_y`, `obj.visible`, `obj.layer` | Leitura e escrita. Com corpo físico, `x` e `y` acompanham o corpo. |
| `obj.sprite`, `obj.text`, `obj.shape`, `obj.camera` | O componente (primeiro do tipo): mude os campos à vontade (`obj.sprite.frame = 3`, `obj.text.text = "Fim"`, `obj.shape.color = "#FF0000"`). |
| `obj.sprite:play(nome, reiniciar)` | Toca uma [animação](#animações-anim) da imagem (se já estiver tocando, continua; `reiniciar = true` começa de novo). Também dá para mudar `obj.sprite.animation`. |
| `obj.sprite:stop()`, `:finished()`, `:animations()` | Para (o quadro atual fica) / a animação de modo "once" chegou ao fim? / os nomes das animações da imagem. Ao terminar, o objeto recebe a mensagem `animacao_terminou` (com o nome). |
| `obj.body` | O corpo físico ([nn.physics](#nnphysics)): `vx`, `vy`, `on_ground`, `:set_position`... |
| `obj.map`, `obj.particles`, `obj.sound` | O mapa carregado, o emissor de partículas ([nn.particles](#nnparticles): `obj.particles:burst(20)`, `obj.particles.emitting = false`) e o som. |
| `obj:component(tipo)`, `obj.components` | Qualquer componente, pelo tipo / todos. |
| `obj:world_transform()` | Posição, rotação e escala no mundo (somando os pais). |
| `obj:destroy()` | Tira o objeto (e os filhos) da cena. |
| `obj.parent`, `obj.children`, `obj.name`, `obj.id` | A árvore de objetos. |
| `obj.vx`, `obj.vy` | Num objeto **sem** corpo físico, velocidade: ele anda sozinho (tiros, enfeites). |
| `obj:behavior(nome)` | O comportamento (`"plataforma"`, `"nn:plataforma"` ou `"scripts/x.lua"`), ou `nil`. |
| `obj:send(msg, ...)` | Chama `msg(...)` em todos os comportamentos do objeto que tiverem essa função: `heroi:send("dano", 1)`. |
| `obj:bounds()` | Metade da largura e da altura (do corpo, da imagem ou da forma). |
| `obj:touching(outro, margem)` | Os dois estão encostando? |
| `obj:move_to(x, y)` | Leva o objeto (e o corpo) para lá, parado. |
| `obj:mouse_over()` | O mouse está em cima do objeto? (Num objeto de HUD, confere na tela.) |
| `obj:tiles_touching(margem)` | Os tiles dos mapas da cena que o objeto toca (com a `margem` em volta, padrão 2): uma lista de `{ col, row, layer, tile, properties, map }`. `properties.tipo` é o tipo marcado no painel Tiles. |

A cena também tem:

| Na cena | O que faz |
|---|---|
| `self:spawn(prefab, x, y, opts)` | Cria um objeto a partir de um [prefab](#prefabs), com a cena rodando, e devolve o objeto. `opts` muda campos (`{ name = "tiro2", layer = 5 }`). |
| `self:restart(opts)` | Reinicia a cena. `{ checkpoint = true }` põe o jogador no último checkpoint. |
| `self:set_checkpoint(x, y)`, `self:checkpoint()` | Guarda / lê o ponto de reinício da cena. |
| `self:targets(quem)` | Objetos com esse nome; `""` = o jogador (objetos com comportamento de jogador). |
| `self:burst(x, y, cor, n)` | Faíscas num ponto. |

Para reagir a toques entre corpos, defina `contact`: ele recebe os eventos da física com os objetos em `e.object_a` e `e.object_b` (o sensor, se houver, vem em `a`):

```lua
function cena:contact(e)
  if e.type == "enter" and e.object_a.name == "moeda" and e.object_b == self.objects.heroi then
    e.object_a:destroy()
  end
end
```

Ao salvar o `.cena` ou o script com o jogo aberto, os objetos voltam ao que está no arquivo e o resto do estado do script (`self.pontos`...) continua.

### Comportamentos

Um comportamento é um componente `behavior`: um pedaço de lógica preso a um objeto, com propriedades que aparecem no Inspetor do NN Studio. Pode ser um dos [prontos](#comportamentos-prontos) (`script = "nn:plataforma"`) ou um arquivo seu, que devolve uma tabela:

```lua
-- scripts/girar.lua
local B = {
  title = "Girar",                                   -- nome no Inspetor
  description = "Gira o objeto sem parar.",
  category = "Movimento",                            -- grupo no menu
  requires = { "sprite" },                           -- componentes de que precisa (opcional)
  props = {
    { "velocidade", 90, label = "Velocidade", help = "Graus por segundo.", min = -720, max = 720 },
    { "sentido", "horario", options = { { "horario", "horário" }, { "anti", "anti-horário" } } },
    { "som", "", type = "sound", advanced = true },
  },
}

function B:start()                -- todos os objetos da cena já existem
  self.giros = 0
end

function B:update(dt)
  local s = self.sentido == "horario" and -1 or 1
  self.obj.rotation = self.obj.rotation + s * self.velocidade * dt
end

return B
```

- **Propriedades** (`props`): lista de `{ "nome", padrão, ... }`. Cada uma vira `self.<nome>`, com o valor que o objeto tem no `.cena` (ou o padrão). Opções: `label`, `help`, `min`, `max`, `integer = true`, `options = { { valor, "texto" }, ... }` (escolha), `advanced = true` (só no modo Completo), `show_if = { "outra", valores... }` (só aparece quando outra propriedade tem um desses valores) e `type`, que normalmente é deduzido do padrão (número, sim/não, cor `"#RRGGBB"`, texto). Para o Inspetor mostrar a lista certa, `type` pode ser `"object"` (nome de um objeto da cena), `"scene"`, `"prefab"`, `"sound"`, `"image"`, `"color"` ou `"choice"`. Também vale um mapa simples: `props = { velocidade = 90 }`.
- **Callbacks** (todos opcionais): `start()`, `update(dt)`, `fixed_update(dt)`, `draw()` (depois dos objetos), `contact(outro, e)` (o corpo do objeto tocou o de `outro`; `e` é o evento da física) e `destroyed()`.
- **Campos:** `self.obj` (o objeto), `self.scene` (a cena: `self.scene:find`, `self.scene:spawn`...), `self.enabled` (`false` para de rodar) e `self.component` (o componente, com os valores do arquivo).
- `player = true` marca um comportamento de jogador: é quem os outros procuram quando "Quem" ou "Alvo" fica vazio.
- Qualquer função do comportamento pode ser chamada por mensagem: `obj:send("dano", 1)` chama `dano(1)` em todos os comportamentos do objeto (é assim que o Perigo tira vida da Vida).

No NN Studio, **Adicionar componente > Script do jogo > Novo script...** cria um arquivo com esse formato. Ao salvar o script com o jogo aberto, os comportamentos são recriados.

#### Comportamentos prontos

| `script` | O que faz | Propriedades (padrão) |
|---|---|---|
| `nn:plataforma` | Jogador de plataforma: anda e pula (setas, WASD, espaço, controle). Precisa de corpo dinâmico. | `velocidade` (320), `pulo` (860), `pulos_no_ar` (0), `coiote` (0.1), `pulo_variavel` (true) |
| `nn:topdown` | Jogador visto de cima: anda nas 4 direções. | `velocidade` (260) |
| `nn:vida` | Pontos de vida; recebe `dano(n)` e `curar(n)`. | `vidas` (3), `variavel` ("vidas": espelha numa variável do jogo), `invencivel` (1 s), `ao_acabar` ("reiniciar", "checkpoint", "cena", "destruir", "nada"), `cena`, `piscar` (true) |
| `nn:patrulha` | Anda até uma parede (ou a beirada) e volta. Precisa de corpo. | `velocidade` (120), `direcao` ("direita"), `beirada` (true), `espelhar` (true) |
| `nn:perseguir` | Vai atrás do alvo quando ele chega perto. | `alvo` ("" = jogador), `velocidade` (150), `visao` (320), `voar` (false), `espelhar` (true) |
| `nn:perigo` | Machuca quem encosta. | `quem` (""), `acao` ("dano", "checkpoint", "reiniciar"), `dano` (1), `pisar` (false: pular em cima destrói), `some` (false), `empurrao` (520) |
| `nn:coletavel` | Some ao ser tocado e soma numa variável. | `quem` (""), `variavel` ("moedas"), `valor` (1), `som`, `brilho` (true), `cor_brilho` |
| `nn:checkpoint` | Ponto de reinício. | `quem` ("") |
| `nn:ir_para_cena` | Leva para outra cena. | `cena`, `quem` (""), `tecla` (false: precisa apertar para cima / Enter), `escurecer` (0.5 s) |
| `nn:movimento` | Gira, flutua ou vai e volta (com corpo cinemático, carrega quem está em cima). | `tipo` ("ir_e_voltar", "flutuar", "girar"), `distancia_x` (200), `distancia_y` (0), `altura` (12), `duracao` (2), `velocidade` (90) |
| `nn:animacao` | Troca a animação conforme o estado. Precisa de imagem. | `parado` ("1"), `andando`, `pulando`, `caindo` (o nome de uma [animação](#animações-anim) da imagem, como `"andar"`, ou quadros como `"1-4"` e `"5,6"`; vazio usa o anterior), `fps` (10, para quadros), `espelhar` (true) |
| `nn:mostrar_valor` | Escreve variáveis num texto. Precisa de texto. | `texto` ("Moedas: {moedas}") |
| `nn:atirar` | Cria um prefab andando. | `prefab`, `tecla` ("x"; vazio = sozinho), `intervalo` (0.3), `velocidade` (700), `direcao` ("frente", "jogador", "direita", "cima"), `som`, `distancia` (24) |
| `nn:gerador` | Cria um prefab de tempos em tempos. | `prefab`, `intervalo` (2), `maximo` (10), `espalhar_x`, `espalhar_y` (0), `comecar_ja` (false) |
| `nn:temporario` | Some depois de um tempo ou fora da tela. | `tempo` (3), `fora_da_tela` (true) |
| `nn:botao` | Botão de menu: destaca com o mouse ou as setas (teclado e controle); clique, Enter ou A apertam. Manda a mensagem `clicou` e faz a ação. | `acao` ("nada", "cena", "reiniciar", "sair"), `cena`, `foco` (false: começa selecionado), `cor` ("#FFD166": cor ao destacar), `aumentar` (1.1), `som` |

A **Vida** também tem `ao_acabar = "nada"` (um evento decide o que fazer) e avisa por mensagem: `levou_dano` a cada dano e `vida_acabou` no fim. O **Coletável** manda `coletou` (com o item) para quem pegou. Essas mensagens chegam aos comportamentos (como funções) e aos [eventos](#eventos-quando-então).

Os prontos são escritos em Lua, com a mesma API dos seus: o código deles está em `crates/nn-script/src/comportamentos/` e é um bom ponto de partida para um comportamento novo.

### Prefabs

Um prefab (`.prefab`) é um objeto guardado num arquivo próprio, para usar em várias cenas:

```lua
return {
  format = 1,
  object = {
    id = 1, name = "moeda",
    components = {
      { type = "sprite", image = "assets/imagens/moeda.png", frame_w = 16, frame_h = 16, w = 40, h = 40 },
      { type = "behavior", script = "nn:coletavel", variavel = "moedas", valor = 1 },
    },
    children = { ... },
  },
}
```

Numa cena, uma instância é um objeto com `prefab`, a posição dela e, se for o caso, o que muda em relação ao prefab:

```lua
{
  id = 7, name = "moeda_rara", prefab = "prefabs/moeda.prefab",
  x = 200, y = 40, rotation = 0, scale_x = 1, scale_y = 1, visible = true, layer = 1,
  overrides = {
    { component = 2, valor = 5 },   -- o 2º componente do prefab, com valor = 5
  },
}
```

Os componentes e os filhos vêm do prefab, então mudar o arquivo muda todas as instâncias. Em código, `self:spawn("prefabs/moeda.prefab", x, y)` cria uma na hora.

### Variáveis do jogo (nn.vars)

`nn.vars` guarda o que vale para o jogo todo: começa com as `vars` do `config.lua`, **passa de uma cena para outra** e volta ao começo quando o jogo é aberto de novo. `nn.reset_vars()` volta tudo aos valores do `config.lua` (a Vida faz isso ao reiniciar a cena). Para guardar no disco entre uma partida e outra, use [nn.save](#nnsave).

```lua
nn.vars.moedas = nn.vars.moedas + 1
if nn.vars.vidas <= 0 then nn.scene.switch("cenas/fim.cena") end
```

Variáveis novas no `config.lua` aparecem no jogo aberto ao salvar.

### Eventos: "Quando... Então..."

Eventos são regras escritas no `.cena` (no campo `events` da cena) ou num objeto (no campo `events` dele, que vai junto quando o objeto é um prefab). No [NN Studio](studio.md#eventos-quando-então) elas são montadas no painel Eventos, lidas como frases; no arquivo, ficam assim:

```lua
events = {
  { comment = "Com uma chave, a porta abre." },
  { when = {
      { cond = "encostando", quem = "@jogador", em = "@este" },
      { cond = "variavel", var = "chaves", op = ">=", valor = 1 },
    }, actions = {
      { act = "variavel", var = "chaves", op = "subtrair", valor = 1 },
      { act = "som", som = "assets/sons/porta.wav" },
      { act = "destruir", alvo = "@este" },
    } },
  { mode = "enquanto", off = true, when = { ... }, actions = { ... } },
},
```

- **`when`**: as condições (todas precisam valer). `nao = true` inverte uma condição.
- **`actions`**: as ações, em ordem. `esperar` atrasa as seguintes.
- **`mode`**: sem ele (ou `"quando"`), a regra roda uma vez quando as condições **passam a valer**, ou a cada vez que um gatilho (apertar uma tecla, encostar...) acontece. Com `"enquanto"`, roda a cada quadro enquanto tudo vale.
- **`off = true`** desliga a regra; **`comment`** sozinho é uma linha de comentário.
- **Quem é quem**, nos campos `quem`, `em` e `alvo`: `"@este"` é o dono da regra (numa regra da cena, o jogador), `"@outro"` é o outro objeto da condição (em quem encostou, o item pego, quem mandou a mensagem), `"@jogador"` são os objetos com comportamento de jogador, e qualquer outro texto é o nome de um objeto (todos com esse nome).
- Textos aceitam `{variavel}` (trocado pelo valor): `"Pontos: {pontos}"`, e o `valor` de uma variável pode ser outra: `valor = "{pontos}"`.
- As regras rodam a cada quadro, depois dos comportamentos. Com o jogo aberto pelo editor, mudar as regras vale na hora.

Condições (`cond`):

| `cond` | Campos |
|---|---|
| `comecou` | — (a cena começou; numa regra de objeto, o objeto apareceu) |
| `a_cada` / `depois_de` | `segundos` |
| `chance` | `chance` (%) |
| `tecla` | `modo` ("apertar", "soltar", "segurar"), `tecla` (uma tecla como "space", "x", "shift", ou um grupo: "pular", "confirmar", "voltar", "acao", "esquerda", "direita", "cima", "baixo", "qualquer") |
| `clicar` | `quem` (clique do mouse no objeto, ou Enter num Botão) |
| `encostar` / `desencostar` / `encostando` | `quem`, `em` |
| `perto` | `quem`, `distancia`, `alvo` |
| `variavel` | `var`, `op` ("==", "~=", "<", ">", "<=", ">="), `valor` |
| `acabaram` | `nome` (não sobrou nenhum objeto com esse nome) |
| `destruido` / `saiu_da_tela` | `quem` |
| `no_chao` | `quem`, `onde` ("chao", "ar") |
| `visivel` | `quem`, `estado` ("visivel", "escondido") |
| `pegar` | `quem`, `item` ("" = qualquer coletável) |
| `levou_dano` / `vida_acabou` | `quem` (precisa do comportamento Vida) |
| `animacao_terminou` | `quem`, `nome` ("" = qualquer uma; só animações de modo "once") |
| `particulas_terminaram` | `quem` (uma explosão acabou, ou um emissor desligado ficou sem partículas) |
| `mensagem` | `quem`, `msg` (de `obj:send("msg")`, de comportamentos ou da ação `mensagem`) |
| `mudou` | `quem` (um deslizador, caixa de marcar ou campo de texto mudou pelo jogador) |

Ações (`act`):

| `act` | Campos |
|---|---|
| `variavel` | `var`, `op` ("somar", "subtrair", "definir", "maior", "menor"), `valor` |
| `salvar` / `carregar` | `var` ("" = todas): grava no disco com [nn.save](#nnsave) (chave `vars.<nome>`) / lê de volta |
| `zerar` | — (`nn.reset_vars()`) |
| `ir_para_cena` | `cena`, `escurecer` (0.5 s) |
| `reiniciar` | `como` ("comeco", "checkpoint", "zerar": do começo, zerando as variáveis) |
| `por_cima` | `cena` (abre por cima: a de baixo fica parada) |
| `fechar_cena` | — (fecha a cena aberta por cima) |
| `sair` | — (fecha o jogo) |
| `criar` | `prefab`, `onde` (na posição de quem), `dx`, `dy`, `vx`, `vy` |
| `destruir` | `alvo` |
| `mostrar` | `acao` ("mostrar", "esconder"), `alvo` |
| `botao_ativo` | `acao` ("ativar", "desativar"), `alvo` (um botão desativado fica cinza) |
| `mover` | `alvo`, `modo` ("para": a posição; "somar": andando), `x`, `y` |
| `velocidade` | `modo` ("definir", "somar": empurrão), `alvo`, `vx`, `vy` (no corpo físico, se houver) |
| `texto` / `imagem` / `quadro` / `cor` | `alvo` e `texto` / `imagem` / `quadro` / `cor` |
| `animacao` | `nome`, `alvo`, `reiniciar` (false) |
| `faiscas` | `alvo`, `cor`, `quantidade` |
| `soltar_particulas` | `alvo`, `quantidade` (do componente Partículas do objeto) |
| `emissor` | `acao` ("ligar", "desligar"), `alvo` |
| `tremer` | `forca`, `tempo` |
| `som` | `som`, `volume` |
| `musica` | `acao` ("tocar", "parar"), `som`, `volume` |
| `volume` | `canal` ("musica", "efeitos", "geral"), `valor` (%; fica guardado) |
| `comportamento` | `acao` ("ligar", "desligar"), `nome` (ex.: "plataforma"), `alvo` |
| `mensagem` | `msg`, `alvo` |
| `esperar` | `segundos` |
| `console` | `texto` |

O catálogo completo (com as frases que o NN Studio mostra e o Lua equivalente de cada item) está em `crates/nn-script/src/eventos.lua`.

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
| `panel(x, y, w, h, opts)` | Painel: retângulo com cantos arredondados e borda, ou uma imagem em 9 fatias. |

Opções de `sprite`: `frame` (para spritesheets), `w`, `h`, `scale`, `rotation`, `origin_x`, `origin_y` (0 a 1; padrão 0.5 no mundo e 0 na tela), `color` (tinge), `alpha`, `flip_x`, `flip_y`, `tile_w`, `tile_h` (repetir em vez de esticar: o tamanho de cada cópia; `{ w = 640, h = 32, tile_w = 32, tile_h = 32 }` desenha 20 blocos).

Opções de `text`: `size` (padrão 20), `color`, `alpha`, `anchor`, `align`, `font`.
`anchor` diz qual ponto do texto fica em `x, y`: `topleft` (padrão), `top`, `topright`, `left`, `center`, `right`, `bottomleft`, `bottom`, `bottomright`.
`align` alinha as linhas de um texto com `\n`: `left`, `center`, `right`.

Opções de `panel`: `color`, `radius` (cantos), `border` (largura) e `border_color`, ou `image` (uma imagem, ou uma spritesheet com `frame`) com `slice` (pixels de cada canto que não esticam) e `slice_scale` (quanto cada pixel do canto cresce; 3 ou 4 para pixel art). No mundo, `x, y` é o centro; na tela, o canto de cima à esquerda. Cores com transparência ficam certas, sem a borda escurecer o fundo.

```lua
nn.screen.panel(20, 20, 300, 120, { color = "#1E2A3AE6", radius = 12, border = 2, border_color = "#FFFFFF40" })
nn.screen.panel(20, 160, 300, 120, { image = moldura, slice = 6, slice_scale = 4 })
```

`nn.text_size(texto, opts)` devolve `largura, altura` sem desenhar.

`nn.screen.flash(cor, duração)` pinta a tela inteira com a cor, sumindo aos poucos (padrão 0,3 s). Use uma cor com transparência para um efeito mais leve: `nn.screen.flash("#FF000066", 0.25)` ao levar dano.

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
| `mouse_delta()` | Quanto o mouse andou neste frame (`dx, dy`, em pixels), mesmo com ele preso (`nn.window.capture_mouse`). É o que gira uma câmera em terceira ou primeira pessoa. |
| `text()` | O texto digitado neste frame, com acentos e maiúsculas (para campos de texto). |

Nomes de teclas: `"a"` a `"z"`, `"0"` a `"9"`, `"space"`, `"enter"`, `"escape"`, `"tab"`, `"backspace"`, `"delete"`, `"shift"`, `"ctrl"`, `"alt"`, `"left"`, `"right"`, `"up"`, `"down"`, `"f1"` a `"f12"`.

| Função | Descrição |
|---|---|
| `move()` | Direção de movimento `dx, dy` juntando WASD, setas, direcional e analógico esquerdo. Comprimento de 0 a 1 (o analógico permite andar devagar). |
| `any_pressed()` | Alguma tecla, botão do mouse ou botão do controle foi apertado neste frame ("aperte qualquer tecla"). |
| `last_device()` | Último dispositivo usado: `"keyboard"`, `"mouse"` ou `"gamepad"`. Útil para mostrar "aperte Enter" ou "aperte A". |

#### Controles

Controles (gamepads) funcionam sem configuração: Xbox, PlayStation, Switch Pro e a maioria dos genéricos. Podem ser conectados e desconectados com o jogo aberto.

| Função | Descrição |
|---|---|
| `pad_down(botão, n)` | Botão segurado. Sem `n`, vale qualquer controle; com `n`, só o controle `n` (1, 2...). |
| `pad_pressed(botão, n)`, `pad_released(botão, n)` | Apertou / soltou neste frame. |
| `pad_axis(eixo, n)` | Valor de um eixo, já com zona morta. |
| `stick(lado, n)` | Analógico `"left"` (padrão) ou `"right"` como `x, y` (Y para cima), com zona morta. |
| `pads()` | Lista dos controles conectados: `{ { slot = 1, name = "Xbox Controller" }, ... }`. |
| `pad_count()` | Quantos controles estão conectados. |
| `set_deadzone(v)` | Zona morta dos analógicos (padrão 0.2). |

Botões, pela posição (nomes do Xbox): `"a"` (baixo), `"b"` (direita), `"x"` (esquerda), `"y"` (cima), `"lb"`, `"rb"`, `"lt"`, `"rt"`, `"back"`, `"start"`, `"ls"`, `"rs"` (apertar os analógicos) e o direcional `"up"`, `"down"`, `"left"`, `"right"`. No controle de PlayStation, `"a"` é o X (cruz); no do Switch, é o B.

Eixos: `"leftx"`, `"lefty"`, `"rightx"`, `"righty"` (de -1 a 1, Y para cima) e os gatilhos `"lt"`, `"rt"` (de 0 a 1).

#### Ações

Uma ação dá nome a algo que o jogador faz e liga várias entradas a ela. O jogo pergunta pela ação, não pela tecla, e funciona igual no teclado e no controle:

```lua
nn.input.bind("pular", { "space", "w", "pad:a" })
nn.input.bind("esquerda", { "left", "a", "pad:left", "pad:leftx-" })
nn.input.bind("direita", { "right", "d", "pad:right", "pad:leftx+" })

-- no update:
if nn.input.action_pressed("pular") then pular() end
-- no fixed_update (com o analógico, inclinar pouco anda devagar):
local dir = nn.input.action_axis("esquerda", "direita")
```

Entradas: nomes de teclas, `"mouse:left"` (ou `right`, `middle`), `"pad:<botão>"` e `"pad:<eixo>+"` / `"pad:<eixo>-"` (o eixo conta como apertado a partir da metade).

| Função | Descrição |
|---|---|
| `bind(ação, entradas)` | Cria ou redefine a ação. As ações valem para todas as cenas. |
| `action(ação)` | Alguma entrada da ação está segurada. |
| `action_pressed(ação)`, `action_released(ação)` | A ação começou / terminou neste frame. Segurar uma segunda entrada da mesma ação não conta como novo aperto. |
| `action_value(ação)` | Intensidade de 0 a 1: 1 para teclas e botões, o quanto o eixo está inclinado. |
| `action_axis(neg, pos)` | Eixo de -1 a 1 a partir de duas ações. |

Usar uma ação que não foi criada com `bind` é um erro (para pegar erros de digitação).

### nn.assets

| Função | Devolve |
|---|---|
| `texture(caminho, { pixel_art = true })` | Imagem PNG, com campos `width` e `height`. |
| `sheet(caminho_ou_imagem, largura_quadro, altura_quadro, opts)` | Spritesheet, com `count`, `columns`, `rows`. Opções: `margin`, `spacing`, `pixel_art`. |
| `sound(caminho)` | Som (WAV, OGG Vorbis, MP3), com `duration` e `:play(opts)`. |
| `font(caminho)` | Fonte .ttf/.otf, para a opção `font` do texto. |
| `read(caminho)` | Conteúdo de um arquivo de texto. |
| `exists(caminho)` | O arquivo existe? |
| `animations(imagem)` | As [animações](#animações-anim) do `.anim` da imagem, por nome: `nn.assets.animations("assets/imagens/heroi.png").andar` (prontas para `nn.animator`). |

Cada arquivo é carregado uma vez; pedir de novo devolve o mesmo. Imagens e sons alterados no disco são recarregados sozinhos.

### nn.audio

`nn.audio.play(som, opts)` toca e devolve o som tocando. Opções: `volume` (1 = normal), `speed` (2 = uma oitava acima), `pan` (-1 esquerda a 1 direita), `loop` e `channel` (`"effects"`, o padrão, ou `"music"`).

O som tocando tem `:stop()`, `:fade_out(segundos)`, `:pause()`, `:resume()`, `:set_volume(v)`, `:set_speed(v)`, `:set_pan(v)` e `:playing()`.

`nn.audio.set_volume(v)` e `nn.audio.volume()` controlam o volume geral. Com um canal, controlam só ele, inclusive os sons que já estão tocando: `nn.audio.set_volume(0.5, "music")`. É o que uma tela de opções precisa para separar música e efeitos.

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

O animador tem `:frame()`, `:finished()`, `:restart()` e os campos `speed` (multiplicador) e `time`. A animação tem `duration`, `length` e `mode`. Um quarto parâmetro dá a cada quadro a sua duração, em segundos: `nn.animation({ 1, 2, 3 }, 10, "once", { 0.1, 0.5, 0.1 })`.

#### Animações (.anim)

As animações feitas no [editor de pixel art do NN Studio](studio.md#animações) ficam num arquivo ao lado da imagem: `heroi.png` → `heroi.anim`. Ele guarda o tamanho do quadro e as animações com nome:

```lua
return {
  format = 1,
  frame_w = 16, frame_h = 16,
  animations = {
    { name = "parado", frames = { 1, 2 }, fps = 3 },
    { name = "andar", frames = { 5, 6, 7, 8 }, fps = 10 },
    { name = "golpe", frames = { 3, 4 }, fps = 10, mode = "once", times = { 0.1, 0.3 } },
  },
}
```

`frames` são números de quadro (a partir de 1), `fps` o ritmo, `mode` é `"loop"` (padrão), `"once"` ou `"pingpong"`, e `times` (opcional) a duração de cada quadro em segundos. Nas cenas visuais, o componente `sprite` usa as animações da imagem dele (`animation = "andar"`, `obj.sprite:play("andar")`, a Animação automática e os eventos); em código, `nn.assets.animations(imagem)` devolve as animações prontas para `nn.animator`:

```lua
local folha = nn.assets.sheet("assets/imagens/heroi.png", 16, 16, { pixel_art = true })
local anims = nn.assets.animations("assets/imagens/heroi.png")
local anim = nn.animator(anims.parado)

function cena:update(dt)
  anim:play(nn.input.down("direita") and anims.andar or anims.parado)
  anim:update(dt)
end

function cena:draw()
  nn.draw.sprite(folha, 0, 0, { frame = anim:frame(), scale = 4 })
end
```

Ao salvar o `.anim` com o jogo aberto, as cenas recarregam com as animações novas.

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

O corpo tem os campos `x`, `y`, `vx`, `vy`, `gravity_scale`, `bounce`, `sensor`, `layer`, `mask` e `tag` (leitura e escrita), `id`, `type`, `width`, `height`, `radius`, `alive`, `on_ground`, `touching_left`, `touching_right`, `touching_top`, `touching_bottom` e `ground` (o corpo em que está apoiado). Métodos: `:set_position(x, y)`, `:set_velocity(x, y)`, `:overlaps()` (corpos encostando agora, respeitando camadas e máscaras), `:intersects(outro)` (os dois se sobrepõem? ignora camadas) e `:remove()`. Dá para comparar corpos com `==`.

**Inimigos que atravessam o jogador:** para o jogador e os inimigos não se empurrarem, ponha-os em camadas que não interagem e confira o contato com `:intersects`. Pisar em cima é o jogador caindo com os pés acima do centro do inimigo:

```lua
jogador.layer = 2
inimigo.layer, inimigo.mask = 4, 1     -- inimigo só colide com o cenário

-- no fixed_update, depois de mundo:step(dt):
if jogador:intersects(inimigo) then
  local pes = jogador.y - jogador.height / 2
  if jogador.vy < 0 and pes > inimigo.y then
    inimigo:remove()                   -- pisou
    jogador.vy = 640                   -- quica
  else
    perder_vida()                      -- encostou de lado
  end
end
```

O jogo de plataforma (`jogos/plataforma/lib/inimigos.lua`) tem uma lesma que patrulha (vira na parede com `touching_left`/`touching_right` e na beirada com `mapa:solid`) e um morcego que persegue o jogador quando ele chega perto.

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

Outros métodos e campos: `width`, `height` (em tiles), `tile_width`, `tile_height`, `world_width`, `world_height`, `layers`, `properties`, `:tile(col, lin, camada)`, `:set_tile(col, lin, tile, camada)` (a colisão acompanha: ótimo para blocos quebráveis), `:set_solid(col, lin, "solid" | "one_way" | nil)`, `:solid(col, lin)`, `:cell_at(x, y)`, `:cell_center(col, lin)`, `:tile_properties(col, lin, camada)` (as propriedades do tile ali no tileset, como `{ tipo = "lava", solid = true }`) e `:tiles_in(x0, y0, x1, y1)` (os tiles não vazios dentro de um retângulo do mundo, em todas as camadas: `{ col, row, layer, tile, properties }`). Colunas e linhas começam em 1, com a linha 1 no topo, como no editor.

**Tiles girados:** como no Tiled, os três bits de cima do número do tile dizem se ele está espelhado na horizontal (`0x80000000`), na vertical (`0x40000000`) ou na diagonal (`0x20000000`; diagonal + horizontal = girado 90° no sentido horário). `mapa:tile(...)` devolve o número com os bits; as propriedades e a colisão são as do tile sem eles.

**No NN Studio**, os mapas são pintados direto na cena (**✏ Pintar**), com terrenos que se encaixam sozinhos e as marcações `solid`, `one_way`, `tipo` e as animações feitas no painel Tiles (veja o [guia do NN Studio](studio.md#mapas-de-tiles)). Os arquivos são os mesmos do Tiled.

**Tiles animados** (água, lava, tochas, bandeiras): no Tiled, abra o tileset, escolha o tile e use o *Editor de Animação de Tiles* para montar a sequência de quadros. Ao carregar o mapa, a animação vem junto e roda sozinha, no ritmo do jogo. Em mapas feitos em Lua, use `mapa:animate(tile, { quadros }, fps)`: onde aparecer `tile`, os `quadros` se revezam (`mapa:animate(5, { 5, 6, 7, 8 }, 6)`). Os quadros precisam ser do mesmo tileset que o tile.

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
| `follow` | As partículas acompanham o emissor quando ele anda (o jato de uma nave). Padrão `false`: ficam onde nasceram (um rastro). |

O emissor tem `x`, `y`, `emitting` e `rate` (leitura e escrita), `count` e `finished`, e os métodos `:burst(n, x, y)`, `:set_position(x, y)`, `:update(dt)`, `:draw()`, `:draw_screen()` e `:clear()`.

### nn.d3 (3D)

O mundo 3D é desenhado **antes** de todo o 2D: sprites, `nn.screen` e menus ficam por cima, então o HUD de um jogo 3D é feito como sempre. O visual é *low-poly*: cada face recebe um tom só de luz, vindo do sol (que faz sombras) e de uma luz ambiente do céu e do chão. As coordenadas usam **Y para cima** e a unidade é livre (pense em metros: o cubo pronto tem 1 de lado).

#### Seu primeiro mundo 3D, passo a passo

**1. Uma forma e uma câmera.** As formas prontas são criadas uma vez (no `enter`) e desenhadas quantas vezes quiser:

```lua
local cena = {}

function cena:enter()
  self.cubo = nn.d3.cube()
  nn.d3.look_at(3, 2, 5, 0, 0, 0)   -- câmera em (3, 2, 5) olhando para o meio
end

function cena:draw()
  nn.d3.draw(self.cubo, { color = "#FF6B6B" })
end

return cena
```

**2. Chão e posições.** `x`, `y`, `z` levam a forma para outro lugar; `scale` (ou `sx`, `sy`, `sz`) muda o tamanho:

```lua
self.chao = nn.d3.plane()
-- no draw:
nn.d3.draw(self.chao, { scale = 20, color = "#7BC86C" })   -- um gramado de 20 × 20
nn.d3.draw(self.cubo, { x = 2, y = 0.5, z = -1 })          -- y = 0.5: apoiado no chão
```

**3. Girar.** `rx`, `ry` e `rz` são graus em volta de cada eixo. `ry` é o mais usado: virar para os lados.

```lua
nn.d3.draw(self.cubo, { y = 0.5, ry = nn.time.elapsed() * 90 })   -- uma volta a cada 4 segundos
```

**4. Céu, luz e névoa.** O céu é um degradê com o disco do sol. O sol tem uma direção (para onde a luz vai, de cima para baixo) e faz sombras sozinho. A névoa esconde o que está longe e, sem `color`, pega a cor do horizonte, então o mundo some no céu:

```lua
nn.d3.sky({ top = "#3F8FD8", horizon = "#CDEBF5", bottom = "#9AA79A", sun_size = 4 })
nn.d3.light({
  sun = { -0.5, -0.8, -0.35 },
  sky = "#93AFCF", ground = "#6A5D48",
  fog = { near = 20, far = 60 },
})
```

**5. Modelos do Blender.** Exporte do Blender como **glTF Binary (.glb)**, ponha na pasta do jogo e carregue com `nn.assets.model`. As cores, os materiais e as texturas do modelo vêm junto:

```lua
self.casa = nn.assets.model("modelos/casa.glb")
-- no draw:
nn.d3.draw(self.casa, { x = 6, z = 4, ry = -30 })
```

No Blender, a frente do modelo é o eixo **−Y** (que vira **+Z** no glTF). Com o jogo aberto, exporte de novo por cima do arquivo e o modelo muda na hora.

**6. Câmera que segue.** `nn.d3.orbit` põe a câmera girando em volta de um ponto, perfeito para seguir o jogador visto de trás:

```lua
nn.d3.orbit({ target = { jogador.x, 1, jogador.z }, distance = 10, yaw = self.yaw, pitch = 25 })
```

**7. Clicar no chão.** `nn.d3.ground_point` diz onde o mouse toca o chão:

```lua
local mx, my = nn.input.mouse()
if mx and nn.input.mouse_pressed("left") then
  local x, y, z = nn.d3.ground_point(mx, my)
  if x then destino = { x = x, z = z } end
end
```

O exemplo completo, com árvores, casas e pedras em `.glb`, um herói que anda para onde a câmera olha, sombras, céu e cubos que caem onde você clica, está em `jogos/cubos3d`. Os modelos dele foram gerados pelo script `ferramentas/gerar_modelos.py` (no código da engine), que mostra como montar um `.glb` à mão.

#### Formas

| Função | Forma |
|---|---|
| `nn.d3.cube()` | Cubo de lado 1, centrado na origem. |
| `nn.d3.plane()` | Quadrado de lado 1 deitado (o chão), virado para cima. |
| `nn.d3.sphere(detalhe)` | Esfera de diâmetro 1. `detalhe` de 0 (20 faces, uma pedra) a 5; padrão 2. |
| `nn.d3.cylinder(lados)` | Cilindro de altura 1 e diâmetro 1 (padrão 16 lados). |
| `nn.d3.cone(lados)` | Cone de altura 1, base de diâmetro 1, ponta para cima. Com poucos lados, um pinheiro. |
| `nn.d3.capsule(lados)` | Cápsula de altura 1 e diâmetro 0,5: o corpo de um personagem simples. |
| `nn.d3.mesh(opts)` | Uma malha sua (abaixo). |

Chamar a mesma forma de novo devolve a mesma malha (não gasta memória). A malha tem `triangles`, `width`, `height` e `depth`.

Para montar uma forma ponto a ponto, dê os pontos e os triângulos (cada um com três números de pontos, no **sentido anti-horário visto de fora**; o lado de dentro não é desenhado). `colors` dá uma cor para cada triângulo:

```lua
local piramide = nn.d3.mesh({
  points = { {-0.5, 0, -0.5}, {0.5, 0, -0.5}, {0.5, 0, 0.5}, {-0.5, 0, 0.5}, {0, 1, 0} },
  triangles = { {4, 3, 5}, {3, 2, 5}, {2, 1, 5}, {1, 4, 5}, {1, 2, 3}, {1, 3, 4} },
  colors = { "#FFD166", "#F4A259", "#FFD166", "#F4A259", "#C9853A", "#C9853A" },
})
```

#### Modelos (.glb e .gltf)

`nn.assets.model(caminho, opts)` carrega um modelo glTF 2.0: o `.glb` (um arquivo só, o mais prático) ou o `.gltf` com o `.bin` e as imagens ao lado. Vêm junto as partes, as posições, rotações e escalas dos objetos, a cor de cada material, as cores dos vértices e a imagem de cor (*base color*) dos materiais. Carregar o mesmo arquivo de novo não gasta memória.

| Opção | Descrição |
|---|---|
| `flat` | `true`: cada face com um tom só (o visual low-poly), mesmo que o modelo tenha sombreado suave. |

O modelo tem `triangles`, `width`, `height` e `depth`. Ele é desenhado com `nn.d3.draw`, como as formas. Ainda não entram: animações de esqueleto, luzes e câmeras do arquivo (por enquanto só modelos parados).

#### Desenhar

`nn.d3.draw(malha_ou_modelo, opts)`, chamado no `draw` da cena:

| Opção | Descrição |
|---|---|
| `x`, `y`, `z` | Posição (padrão 0). |
| `rx`, `ry`, `rz` | Rotação em graus: inclinar, virar e rolar. |
| `scale` | Tamanho (padrão 1). `sx`, `sy`, `sz` mudam um eixo só. |
| `color` | Cor (multiplica a da malha; padrão branco). |
| `image` | Uma imagem de `nn.assets.texture` aplicada nas faces (nas formas prontas e nas malhas). |
| `shadow` | `false`: não projeta sombra (boa para o chão, efeitos e coisas pequenas). Padrão `true`. |
| `body` | Um corpo de [`nn.physics3d`](#nnphysics3d-física-3d): desenha na posição e rotação dele (a escala continua vindo de `scale`). |

#### Câmera

| Função | Descrição |
|---|---|
| `nn.d3.look_at(x, y, z, ax, ay, az)` | Câmera em `x, y, z` olhando para o ponto `ax, ay, az`. |
| `nn.d3.orbit(opts)` | Câmera em volta de `target` (`{x, y, z}`), a `distance`, girada `yaw` graus para os lados e `pitch` graus para cima. |
| `nn.d3.camera(opts)` | Muda só o que for dado (`x`, `y`, `z`, `target`, `fov` em graus, `near`, `far`) e devolve a câmera como ficou. Sem nada, só devolve. |
| `nn.d3.third_person(opts)` | Uma câmera em terceira pessoa pronta (abaixo). |

A câmera continua a mesma ao trocar de cena; cada cena 3D costuma ajustá-la no `enter`.

#### Câmera em terceira pessoa

`nn.d3.third_person(opts)` cria uma câmera que fica atrás do personagem, gira com o **mouse preso** e com o **analógico direito**, aproxima e afasta com a rodinha e chega mais perto quando uma parede ou árvore fica no caminho, para nunca atravessar o cenário:

```lua
function cena:enter()
  self.camera = nn.d3.third_person({ distance = 7, pitch = 18 })
  nn.window.capture_mouse(true)          -- o cursor some; o mouse gira a câmera
end

function cena:update(dt)
  -- Andar para onde a câmera olha: a entrada (X para a direita, Y para a frente)
  -- vira uma direção no chão.
  local ix, iy = nn.input.move()
  local vx, vz = self.camera:direction(ix, iy)
  self.heroi:walk(vx * 6, vz * 6)

  self.camera:update(dt, self.heroi.x, self.heroi.y, self.heroi.z,
    { world = self.mundo, ignore = self.heroi })
end
```

| Opção | Descrição |
|---|---|
| `distance` | Distância até o personagem (padrão 6). `min_distance` e `max_distance` limitam a rodinha. |
| `yaw`, `pitch` | Ângulo inicial em graus: em volta (0 = atrás, olhando para −Z) e acima do horizonte (padrão 20). `min_pitch` e `max_pitch` limitam (padrão −30 e 70). |
| `height` | Altura do ponto olhado acima dos pés (padrão 1,5). |
| `sensitivity` | Graus por pixel do mouse (padrão 0,15). |
| `stick_speed` | Graus por segundo com o analógico no fim (padrão 150). |
| `invert_y` | Inverte o movimento vertical. |
| `zoom` | `false` desliga a rodinha. |
| `follow` | Quão grudada ela segue o personagem (padrão 12; maior = mais grudada, 0 = sem suavizar). |

`camera:update(dt, x, y, z, opts)` vai no `update`. Ela lê o mouse (só quando ele está preso), o analógico e a rodinha, e põe a câmera do `nn.d3`. As opções são:
- `world`: um mundo de `nn.physics3d`, para não atravessar paredes;
- `ignore`: o corpo do personagem;
- `control = false`: ignora a entrada, para cenas cinematográficas.

Os campos `yaw`, `pitch`, `distance`, `sensitivity` e `invert_y` podem ser lidos e mudados (por exemplo, numa tela de opções), e `x`, `y`, `z` dizem onde a câmera está. Os métodos são:
- `camera:direction(ix, iy)`: transforma a entrada em `vx, vz` no chão;
- `camera:forward()`: para onde ela olha, `fx, fz`;
- `camera:rotate(graus_lado, graus_cima)`.

#### Luz

`nn.d3.light(opts)` muda só o que for dado:

| Opção | Descrição |
|---|---|
| `sun` | Direção da luz do sol, `{x, y, z}` (para baixo: `y` negativo). |
| `sun_color` | Cor e força do sol (mais escuro = mais fraco; `"#000000"` desliga). |
| `sky`, `ground` | Luz ambiente que vem de cima e de baixo: deixa as sombras coloridas, nunca pretas. |
| `fog` | `{ near =, far =, color = }`: a névoa começa em `near` e cobre tudo em `far`. Sem `color`, usa a cor do horizonte do céu. `false` desliga. |

#### Céu

`nn.d3.sky(opts)` desenha um céu em degradê atrás do mundo 3D, no lugar da cor de fundo. `nn.d3.sky(true)` usa o céu padrão (azul) e `nn.d3.sky(false)` tira o céu.

| Opção | Descrição |
|---|---|
| `top`, `horizon`, `bottom` | Cor lá em cima, na linha do horizonte e abaixo dela. |
| `sun_size` | Tamanho do disco do sol em graus (padrão 3; 0 = sem disco). Ele fica na direção de onde vem a luz do sol, com a cor de `sun_color`. |

#### Sombras

O sol faz sombras sozinho: tudo projeta e recebe sombra. Para uma coisa não projetar, desenhe com `shadow = false`.

| Chamada | Descrição |
|---|---|
| `nn.d3.shadows(false)` / `nn.d3.shadows(true)` | Desliga ou liga todas as sombras (bom para uma opção "sombras" no menu, para computadores mais fracos). |
| `nn.d3.shadows({ distance =, quality =, strength = })` | `distance`: até onde há sombra em volta do ponto para onde a câmera olha (padrão 30). `quality`: `"baixa"`, `"media"` (padrão) ou `"alta"`. `strength`: de 0 a 1, quanto a sombra escurece (padrão 1). |

#### Da tela para o 3D e de volta

| Função | Descrição |
|---|---|
| `nn.d3.to_screen(x, y, z)` | Onde o ponto aparece na tela (`x, y`), ou `nil` se estiver atrás da câmera. Bom para nomes e barras de vida em cima de coisas 3D. |
| `nn.d3.ground_point(x, y, altura)` | Onde o ponto da tela toca o chão na `altura` dada (padrão 0): `x, y, z`, ou `nil` se olhar para o céu. |
| `nn.d3.ray(x, y)` | O raio que sai da câmera pelo ponto da tela: origem e direção (seis números). |
| `nn.d3.count()` | Quantas malhas foram desenhadas no último quadro. |

Em breve: o 3D no NN Studio.

### Seu primeiro jogo 3D

O modelo `3d` cria um jogo de plataforma 3D em terceira pessoa, já jogando: a **Ilha das Estrelas**. São 5 estrelas escondidas, moedas, plataformas que andam, cronômetro, pausa com opções da câmera e recorde salvo. Este guia mostra como transformá-lo no seu jogo.

**1. Crie e jogue.** No NN Studio, escolha **Novo projeto > 3D**, ou pelo terminal:

```sh
cargo nn novo ../minha-ilha --modelo 3d
cargo nn ../minha-ilha
```

O mouse fica preso e gira a câmera. **Esc** pausa e solta o mouse. Na pausa dá para ajustar a sensibilidade e inverter o Y.

**2. Entenda as peças.** Tudo é código Lua curto:
- `lib/ilha.lua` monta a fase (a física e o desenho);
- `cenas/jogo.lua` tem as regras: o herói, a câmera, as moedas, as estrelas, cair no mar e o HUD;
- `cenas/titulo.lua`, `cenas/pausa.lua` e `cenas/fim.lua` são as outras telas;
- `lib/jogo.lua` tem os controles, o céu, a luz e as opções.

**3. Mude a fase.** No começo de `lib/ilha.lua` ficam três tabelas, e cada linha é uma coisa:

```lua
local PLATAFORMAS = {
  { -4, 1, 2, 3, 3 },   -- x, y (altura do topo), z, largura, profundidade
  ...
}
local MOVEIS = {
  { a = { 0, 3.5, -10 }, b = { 6, 3.5, -10 }, v = 1.6, w = 3, d = 3 },  -- vai de a até b
  ...
}
local ESTRELAS = { { -6, 4, -7 }, ... }   -- onde fica cada estrela
```

Acrescente uma plataforma, salve e o jogo aberto recarrega. O herói pula cerca de 1,6 de altura e alcança uns 4 de distância. Lembre-se disso para a fase continuar possível. Para saber onde você está, ponha `print(self.heroi.x, self.heroi.y, self.heroi.z)` no `update` e veja no terminal.

**4. Ajuste o herói.** Em `cenas/jogo.lua`, `VELOCIDADE` e `PULO` mudam o jeito de andar. A gravidade é o `gravity` do mundo em `lib/ilha.lua`: mais negativa deixa o pulo mais "pesado".

**5. Troque os modelos.** Os modelos ficam em `modelos/`. Faça um personagem no Blender, exporte como **glTF Binary (.glb)** com a frente virada para −Y do Blender, e salve por cima de `modelos/heroi.glb`. Ele aparece na hora. Veja também [Modelos](#modelos-glb-e-gltf).

**6. Uma regra nova.** Exemplo: uma estrela vermelha que dá 10 segundos de bônus. Em `lib/ilha.lua`, crie o item com outro tipo, `self:item("bonus", { 2, 1, 6 }, 0.6)`, e desenhe-o em `ilha:desenhar` (copie o desenho da estrela com `color = "#FF5A5A"`). Em `cenas/jogo.lua`, no `fixed_update`, junto das moedas:

```lua
elseif it and it.tipo == "bonus" then
  self.tempo = math.max(0, self.tempo - 10)
  self:avisar("-10 segundos!")
```

**7. Distribua.** `cargo nn exportar ../minha-ilha` gera um executável único, como em qualquer jogo da engine (veja [Distribuindo o jogo](#distribuindo-o-jogo)).

As referências estão em [nn.d3](#nnd3-3d), [nn.physics3d](#nnphysics3d-física-3d) e [Câmera em terceira pessoa](#câmera-em-terceira-pessoa).

### nn.physics3d (física 3D)

Física de verdade em 3D: as coisas caem, quicam, rolam, empilham e são empurradas; o personagem anda, pula, sobe degraus e rampas e não atravessa paredes. É separada da [física 2D](#nnphysics) e mede em "metros" (a gravidade da Terra é -9,81; jogos costumam usar mais, como -20, para pulos menos flutuantes).

#### Física 3D, passo a passo

**1. Um mundo e um chão.** O mundo avança no `fixed_update`; o chão é um corpo `static` (não se move):

```lua
function cena:enter()
  self.mundo = nn.physics3d.world({ gravity = -20 })
  self.mundo:body({ type = "static", w = 40, h = 1, d = 40, y = -0.5 })   -- chão com o topo em y = 0
end

function cena:fixed_update(dt)
  self.mundo:step(dt)
end
```

**2. Uma caixa que cai.** Sem `type`, o corpo é `dynamic`: cai, rola e empurra. Para desenhar, passe o corpo em `body` e o desenho segue a posição e a rotação dele:

```lua
self.caixa = self.mundo:body({ size = 1, x = 0, y = 5, z = 0 })
self.cubo = nn.d3.cube()
-- no draw:
nn.d3.draw(self.cubo, { body = self.caixa, color = "#FF6B6B" })
```

**3. Um herói que anda e pula.** `mundo:character` cria um personagem; a posição dele é a dos **pés**. Diga a velocidade com `:walk(vx, vz)` e pule com `:jump(velocidade)` (só pula se estiver no chão):

```lua
self.heroi = self.mundo:character({ x = 0, z = 0 })
-- no update:
local ix, iy = nn.input.wasd()
self.heroi:walk(ix * 5, -iy * 5)
if nn.input.pressed("space") then self.heroi:jump(8) end
-- no draw:
nn.d3.draw(self.modelo_heroi, { x = self.heroi.x, y = self.heroi.y, z = self.heroi.z })
```

**4. Cenário com modelos.** Um modelo vira corpo com `model`: por padrão, uma caixa em volta dele. Para cenário parado, `shape = "mesh"` usa os triângulos exatos (dá para subir no telhado):

```lua
local casa = nn.assets.model("modelos/casa.glb")
self.mundo:body({ type = "static", model = casa, shape = "mesh", x = 7, z = 5, ry = -30 })
```

**5. Moedas e portas: sensores.** Um corpo com `sensor = true` não bloqueia, só avisa quem entra e sai. Os avisos vêm de `mundo:step`:

```lua
self.moeda = self.mundo:body({ type = "static", sensor = true, shape = "sphere", radius = 0.5, x = 4, y = 1 })
-- no fixed_update:
for _, e in ipairs(self.mundo:step(dt)) do
  if e.type == "enter" and e.a == self.moeda and e.b == self.heroi then
    self.moeda:remove()
  end
end
```

**6. Ver as colisões.** `mundo:debug_draw()` (no `draw`) mostra as formas de colisão por cima de tudo, meio transparentes. Ajuda muito a ajustar tamanhos. No `jogos/cubos3d`, a tecla C liga e desliga.

#### Corpos

`mundo:body(opts)`:

| Opção | Descrição |
|---|---|
| `type` | `"dynamic"` (padrão: cai e rola), `"static"` (chão, paredes, cenário) ou `"kinematic"` (movido pelo jogo pela velocidade: plataformas, portas; empurra os dinâmicos). |
| `shape` | `"box"` (padrão), `"sphere"`, `"capsule"`, `"cylinder"` ou `"mesh"` (os triângulos de um `model`, só para `static` e `kinematic`). |
| `w`, `h`, `d` ou `size` | Caixa: largura, altura e profundidade (ou as três iguais). |
| `radius`, `height` | Esfera, cápsula e cilindro (em pé). |
| `model` | Um modelo de `nn.assets.model`: a caixa em volta dele, ou os triângulos com `shape = "mesh"`. |
| `scale` | Multiplica o tamanho (use o mesmo `scale` do desenho). |
| `x`, `y`, `z`, `rx`, `ry`, `rz` | Posição e rotação inicial (graus). |
| `vx`, `vy`, `vz` | Velocidade inicial. |
| `mass` ou `density` | Massa (ou massa por volume; padrão 1). Mais pesado empurra mais e é menos empurrado. |
| `friction` | Atrito (0 = gelo; padrão 0,5). |
| `bounce` | Quique (0 a 1). |
| `sensor` | Não colide: só avisa quem entra e sai. |
| `lock_rotation` | Não gira (fica sempre em pé). |
| `gravity_scale` | Multiplicador da gravidade (0 = flutua). |
| `damping`, `angular_damping` | Freio do movimento e do giro (resistência do ar). |
| `ccd` | Para coisas muito rápidas (tiros) não atravessarem paredes finas. |
| `offset_x`, `offset_y`, `offset_z` | Desloca a forma em relação à posição do corpo. |
| `tag` | Rótulo livre. |

O corpo tem os campos `x`, `y`, `z`, `vx`, `vy`, `vz`, `rx`, `ry`, `rz`, `gravity_scale` e `tag` (leitura e escrita), e `id`, `type`, `alive`, `mass`, `sensor`, `sleeping` (parado, sem gastar tempo da física) e `is_character`.

Métodos do corpo:
- `:set_position(x, y, z)`, `:set_velocity(x, y, z)` e `:set_rotation(rx, ry, rz)`;
- `:set_spin(x, y, z)`: giro em graus por segundo;
- `:impulse(x, y, z)`: um empurrão;
- `:impulse_at(x, y, z, px, py, pz)`: um empurrão num ponto, que faz girar;
- `:torque(x, y, z)`;
- `:touching()`: os corpos encostando agora;
- `:remove()`.

Dá para comparar corpos com `==`.

#### Personagem

`mundo:character(opts)` cria uma cápsula em pé, controlada pelo jogo:

| Opção | Descrição |
|---|---|
| `x`, `y`, `z` | Posição dos pés. |
| `radius`, `height` | Tamanho da cápsula (padrão 0,35 e 1,8). |
| `step` | Altura de degrau que ele sobe sozinho (padrão 0,35). |
| `slope` | Rampa mais inclinada que ele sobe, em graus (padrão 45). |
| `mass` | Força para empurrar as coisas (padrão 70). |

Além do que todo corpo tem, o personagem tem:
- `:walk(vx, vz)`: a velocidade de andar, que vale até mudar;
- `:jump(velocidade)`: devolve se pulou;
- os campos `on_ground`, `hit_wall` e `hit_ceiling`.

A gravidade do mundo cuida da queda. Ele empurra os corpos dinâmicos e entra nos sensores.

#### O mundo

`mundo:step(dt)` avança a física. Chame no `fixed_update`. Ele devolve os eventos do passo: tabelas `{ type = "enter" | "exit", a = corpo, b = corpo }`, para corpos que começaram ou pararam de se tocar, ou que entraram ou saíram de um sensor. Se um dos dois for sensor, ele vem em `a`.

Outros métodos do mundo:
- `:raycast(x, y, z, dx, dy, dz, distancia, ignorar)`: devolve `{ x, y, z, nx, ny, nz, distance, body }` ou `nil`. Ignora os sensores e o corpo `ignorar` (quem atira). Com `nn.d3.ray(mx, my)`, diz o que está embaixo do mouse.
- `:query_sphere(x, y, z, raio)`: os corpos dentro da esfera.
- `:bodies(tag)`: os corpos com esse rótulo, ou todos.
- `:debug_draw()`.

Campos do mundo: `gravity` (o Y da gravidade) e `count`.

### nn.ui

Menus com botões, opções e barras de volume. Funcionam com **mouse, teclado e controle** sem nenhum código extra: setas, WASD, direcional ou analógico mudam o foco (cima e baixo) e o valor (esquerda e direita); Enter, Espaço ou o botão A confirmam; o mouse foca ao passar por cima e confirma com clique.

A interface é montada **a cada frame, dentro do `draw`**, e cada item já devolve o resultado:

```lua
function menu:draw()
  local ui = nn.ui.panel("principal", { title = "Meu jogo" })
  if ui:button("Jogar") then nn.scene.switch("cenas/fase.lua") end
  self.volume = ui:slider("Volume", self.volume)
  self.tela_cheia = ui:toggle("Tela cheia", self.tela_cheia)
  if ui:button("Sair") then nn.quit() end
  ui:finish()
end
```

`nn.ui.panel(nome, opts)` abre um painel. O `nome` faz o painel lembrar qual item estava com foco. Opções: `title`, `width`, `x` e `y` (ponto da tela; padrão: centro), `anchor` (que ponto do painel fica em `x, y`; padrão `"center"`), `background` (cor).

| Método | Devolve |
|---|---|
| `ui:button(texto)` | `true` no frame em que foi acionado. |
| `ui:toggle(texto, valor, opts)` | `novo_valor, mudou`. Mostra "Sim"/"Não"; mude com `{ on = "Ligado", off = "Desligado" }`. |
| `ui:slider(texto, valor, opts)` | `novo_valor, mudou`. Opções `min` (0), `max` (1) e `step` (um décimo do intervalo). De 0 a 1 aparece em porcentagem. Também dá para clicar e arrastar a barra. |
| `ui:choice(texto, opções, índice)` | `novo_índice, mudou`. Escolhe entre textos com esquerda e direita: `ui:choice("Dificuldade", { "Fácil", "Normal" }, self.dif)`. |
| `ui:label(texto, opts)` | Texto informativo, sem foco. Opções `color` e `size` (multiplica o tamanho do tema). |
| `ui:space(altura)` | Espaço vazio. |
| `ui:back()` | `true` se o jogador pediu para voltar (Esc, Backspace ou o botão B do controle). |
| `ui:focus()` | Número do item com foco (1, 2...). |
| `ui:finish()` | Desenha o painel. Se esquecer, a engine desenha sozinha no fim do `draw`. |

Detalhes que já vêm resolvidos:

- **Só a cena do topo responde.** Numa pausa por cima do jogo, o menu de baixo continua aparecendo, mas só o de cima recebe teclas e cliques.
- **O aperto que abriu o painel não conta.** O Esc que abriu a pausa não aciona o "Voltar" da pausa no mesmo frame.
- **Trocar de cena num botão** (`nn.scene.*` dentro do `draw`) acontece no frame seguinte.
- **Escala:** a interface acompanha a altura da janela (pensada para 720 pixels), então fica do mesmo tamanho relativo em tela cheia.

`nn.ui.theme(opts)` muda a aparência de todos os painéis. Opções: `font`, `text_size`, `title_size`, cores `text_color`, `label_color`, `panel_color`, `button_color`, `focus_color`, `focus_text_color`, `accent_color` (barras), medidas `width`, `padding`, `spacing`, `row_height`, `scale` (número ou `"auto"`), textos `on_text`, `off_text` e sons `move_sound` (mudar foco ou valor) e `confirm_sound` (`false` remove).

`nn.ui.set_focus(nome, n)` põe o foco no item `n` de um painel (ex.: "Continuar" ao abrir a pausa); `nn.ui.reset(nome)` esquece o foco.

### nn.save

Dados que continuam existindo depois de fechar o jogo: recordes, opções, progresso.

```lua
local recorde = nn.save.get("recorde", 0)    -- 0 se ainda não existe
if pontos > recorde then nn.save.set("recorde", pontos) end

nn.save.set("opcoes", { musica = 0.7, tela_cheia = true })
local opcoes = nn.save.get("opcoes", {})
```

| Função | Descrição |
|---|---|
| `get(chave, padrão)` | Valor salvo, ou `padrão` se não existir. |
| `set(chave, valor)` | Guarda números, textos, booleanos e tabelas com eles. `nil` apaga. |
| `remove(chave)`, `clear()` | Apaga uma chave / tudo. |
| `keys()` | Lista das chaves salvas. |
| `flush()` | Grava no disco agora. Normalmente não é preciso: a engine grava sozinha (no máximo uma vez por segundo) e ao fechar o jogo. |
| `path()` | Onde fica o arquivo. |

O arquivo é um `save.json` legível em `~/.local/share/<id>/` no Linux e em `%APPDATA%\<id>\data\` no Windows, onde `<id>` vem do `id` do `config.lua` (ou do título). Defina o `id` para o progresso não se perder se o título mudar. A gravação é segura: se o computador desligar no meio, o arquivo anterior continua inteiro.

### Fundos em camadas (parallax)

`nn.draw.parallax(imagem, opts)` desenha uma imagem repetida que acompanha a câmera mais devagar que o mundo, dando profundidade: montanhas ao longe andam pouco, colinas mais perto andam mais. Chame no `draw`, **depois de posicionar a câmera** e antes do resto do mundo, do mais longe para o mais perto:

```lua
nn.draw.parallax(self.nuvens, { factor_x = 0.1, y = 560, scroll_x = 12 })
nn.draw.parallax(self.montanhas, { factor_x = 0.2, factor_y = 1, y = 280, scale = 4 })
nn.draw.parallax(self.colinas, { factor_x = 0.5, factor_y = 1, y = 210, scale = 4 })
```

| Opção | Descrição |
|---|---|
| `factor` | Quanto acompanha a câmera, nos dois eixos: 0 = parada na tela (céu), 1 = junto com o mundo. Padrão 0.5. `factor_x` e `factor_y` mudam um eixo só. |
| `x`, `y` | Centro da imagem quando a câmera está em (0, 0). |
| `scale`, `w`, `h` | Tamanho no mundo (padrão: o da imagem). |
| `repeat_x`, `repeat_y` | Repetir na horizontal (padrão `true`) e na vertical (padrão `false`). |
| `scroll_x`, `scroll_y` | Velocidade de rolagem sozinha, em unidades por segundo (nuvens, neblina). |
| `color`, `alpha` | Tinge e deixa transparente. |

Para a imagem emendar sem costura, a borda esquerda dela deve continuar na direita.

### nn.camera

`position()`, `set_position(x, y)`, `zoom()`, `set_zoom(z)`, `rotation()`, `set_rotation(r)`, `to_world(sx, sy)`, `to_screen(wx, wy)`.

**Seguir o jogador:** `nn.camera.follow(x, y, opts)`, chamado uma vez por frame (no `update` ou no `draw`), leva a câmera até o alvo de forma suave, igual em qualquer taxa de quadros.

```lua
nn.camera.follow(jogador.x, jogador.y, {
  bounds = mapa,                          -- não mostra nada fora do mapa
  offset_x = olhando_esquerda and -90 or 90, -- olha um pouco à frente
  deadzone_h = 120,                       -- pular não balança a tela
})
```

| Opção | Descrição |
|---|---|
| `smooth` | Rapidez para alcançar o alvo (padrão 6: ~95% do caminho em meio segundo). 0 vai direto. |
| `deadzone_w`, `deadzone_h` | Zona morta: o alvo anda livre dentro dela sem a câmera se mexer. |
| `offset_x`, `offset_y` | Somado ao alvo. |
| `bounds` | Limite: um mapa (`nn.tilemap`) ou `{ x =, y =, w =, h = }` (canto inferior esquerdo e tamanho). Se o limite for menor que a tela num eixo, a câmera fica no centro dele. |

`nn.camera.clamp(limite)` só aplica o limite, sem seguir nada. `nn.camera.view()` devolve a área visível no mundo: `x0, y0, x1, y1` (útil para saber o que está na tela).

`nn.camera.shake(força, duração)` faz a câmera tremer: `força` é o deslocamento máximo em pixels da tela e o tremor diminui até parar em `duração` segundos (padrão 0,3). Tremores somam; `nn.camera.stop_shake()` para na hora. Só o mundo treme: o que é desenhado com `nn.screen` (HUD, menus) fica parado, e `nn.camera.position()` continua devolvendo a posição sem o tremor.

### nn.scene

| Função | Efeito |
|---|---|
| `push(caminho, args, opts)` | Abre a cena por cima da atual. |
| `pop(opts)` | Fecha a cena atual e volta para a de baixo. |
| `replace(caminho, args, opts)` | Troca a cena atual. |
| `switch(caminho, args, opts)` | Fecha todas e abre esta (ex.: voltar ao menu). |
| `nn.quit(opts)` | Fecha o jogo. |

A troca acontece ao final do `update`. Pedidos feitos no `draw` (como num botão de menu) valem no frame seguinte.

**Escurecer e clarear:** com `opts = { fade = segundos }`, a tela escurece, a cena troca com a tela coberta e a tela clareia de novo (metade do tempo para cada lado). `color` muda a cor (padrão preto); `fade = true` usa 0,5 s.

```lua
nn.scene.switch("cenas/fase.lua", nil, { fade = 0.6 })
nn.scene.switch("cenas/titulo.lua", nil, { fade = 1, color = "#FFFFFF" })
nn.quit({ fade = 0.4 })
```

Enquanto a tela escurece, a cena que está saindo continua desenhada mas fica parada (sem `update`) e os menus não respondem, então um segundo aperto não dispara outra troca. A cena nova já roda enquanto a tela clareia.

### nn.time e nn.window

`nn.time.delta()`, `elapsed()`, `fixed_delta()`, `alpha()` (fração entre passos fixos, para interpolar posições no `draw`), `frame()`.

`nn.window.size()` devolve `largura, altura`; `nn.window.set_clear_color(cor)` muda a cor de fundo.

`nn.window.fullscreen()` diz se está em tela cheia; `nn.window.set_fullscreen(true/false)` e `nn.window.toggle_fullscreen()` mudam (no fim do frame). A janela muda de tamanho, então use `nn.window.size()` a cada frame em vez de guardar o tamanho.

`nn.window.capture_mouse(true)` prende o mouse na janela e esconde o cursor (para girar a câmera com `nn.input.mouse_delta()`); `nn.window.capture_mouse(false)` solta. Solte ao abrir a pausa e os menus, e prenda de novo ao voltar (o `resume` da cena é um bom lugar). Ao sair da janela (Alt+Tab) o mouse é solto sozinho e volta a ser preso quando ela recebe o foco. `nn.window.mouse_captured()` diz se está preso.

`nn.window.title()` devolve o título do jogo (o `title` do config.lua).

### Outros

`print(...)` escreve no terminal (log da engine). Com [tamanho fixo do jogo](#configlua), `nn.window.size()` devolve esse tamanho. `nn.version` é a versão da engine. `nn.vars` e `nn.reset_vars()`: veja [variáveis do jogo](#variáveis-do-jogo-nnvars). Variáveis globais são compartilhadas entre as cenas e sobrevivem à recarga.
