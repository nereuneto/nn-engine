# Jogos em Lua

A nn-engine roda jogos escritos inteiramente em Lua (5.4). Não é preciso instalar Lua: ela vem compilada dentro da engine.

## Começando

Para criar um jogo novo, pronto para rodar:

```sh
cargo nn novo ../meu-jogo     # cria a pasta com um jogo pequeno e completo
cargo nn ../meu-jogo          # joga
```

O modelo padrão já vem com menu, partida, pausa, tela de fim e recorde salvo, usando o jeito recomendado de organizar um jogo; é só ir mudando. Opções: `--titulo "Meu Jogo"` e `--modelo`, que escolhe o modelo: `visual` (fase de plataforma montada no [NN Studio](studio.md), sem código, com [comportamentos](#comportamentos), [eventos](#eventos-quando-então) e um [prefab](#prefabs)), `aventura` (visto de cima: chave, porta e tesouro), `nave` (tiros, asteroides e recorde salvo), `3d` (plataforma 3D em terceira pessoa em código, veja [Seu primeiro jogo 3D](#seu-primeiro-jogo-3d)), `visual3d` (a mesma ilha montada no NN Studio, sem código: [cenas 3D](#cenas-3d)), `vazio` (cena vazia com um pacote de imagens e sons), `vazio3d` (mundo 3D com chão e um pacote de modelos e sons), `vale3d` (um vale pintado num Terreno, sem código) ou `minimo` (só uma cena com um quadrado que anda). A pasta não pode existir com arquivos dentro. Com o `nn` instalado (sem o código da engine), o comando é `nn novo meu-jogo`.

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

Os projetos criados pelo `nn novo` (ou pelo NN Studio) também trazem um `AGENTS.md`: ele explica o projeto a um agente de IA. Para desenvolver com um agente, veja [O servidor MCP](mcp.md).

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
| `text` | `text`, `size` (32), `color`, `anchor` ("center", "left", "right"...), `font` ("": a do tema), `outline` ("": uma cor para a borda em volta das letras, que deixa o texto legível em cima de qualquer fundo) |
| `shape` | `shape` ("rect" ou "circle"), `w`, `h` (64), `radius` (32), `color` |
| `body` | `body` ("dynamic", "static" ou "kinematic"), `shape` ("box" ou "circle"), `w`, `h` (32), `radius` (16), `sensor`, `one_way`, `bounce` (0), `gravity_scale` (1), `tag` (padrão: o nome do objeto) |
| `map` | `map` (arquivo .tmj), `scale` (1), `collision` (true) |
| (sprite, text, shape) | `hud` (false): fixo na tela, em vez de andar com o mundo. A posição conta a partir do centro da tela, ou do ponto escolhido em `anchor` (y para cima). |

A cena e cada objeto também podem ter `events`, as regras [Quando... Então...](#eventos-quando-então).

#### Cenas 3D

Com `mode = "3d"`, a cena é um mundo 3D (montado na [vista 3D do NN Studio](studio.md#cenas-3d)). Cada objeto ganha `z`, a rotação `rx`, `ry`, `rz` (em graus: inclinar, virar e rolar, como no [`nn.d3.draw`](#desenhar)) e `scale_z`; X e Z são o chão e Y é a altura. A cena também tem um `environment`, que monta o céu, o sol, a névoa e as sombras (campos que faltam usam o padrão):

```lua
return {
  format = 1, mode = "3d",
  environment = {
    sky = true, sky_top = "#3F8FD8", sky_horizon = "#CDEBF5", sky_bottom = "#9AA79A",
    sun_height = 50, sun_turn = 120, sun_color = "#FFF1D6", sun_size = 4,
    ambient_sky = "#93AFCF", ambient_ground = "#6A5D48",
    fog = true, fog_near = 30, fog_far = 90,
    shadows = true, shadow_distance = 30,
    cycle = false, day_length = 4, start_hour = 9,
    wind = 0, wind_turn = 60, bloom = 1,
  },
  objects = {
    { id = 1, name = "Câmera", y = 5, z = 10, rx = -22, components = { { type = "camera3d", fov = 60 } } },
    { id = 2, name = "Cubo", y = 0.5, components = { { type = "shape3d", shape = "cube", color = "#FF8C42" } } },
  },
}
```

`sun_height` é a altura do sol em graus (90 = meio-dia) e `sun_turn` o lado de onde ele vem. `wind` é a força do vento (0 = parado, 1 = brisa, 3 = ventania) e `wind_turn` o lado para onde ele sopra, em graus (0 = para -Z, 90 = para +X): balança o que tem `sway` e leva as partículas. `bloom` é o halo em volta do que brilha (0 = sem, 1 = o normal). Mudar o ambiente (`scene:environment`) não mexe no vento nem no halo.

**Dia e noite.** Com `cycle = true`, a hora anda: um dia do jogo dura `day_length` minutos, e o jogo começa na hora `start_hour` (0 a 24). O sol nasce às 6h, fica na altura `sun_height` ao meio-dia e se põe às 18h; de noite, a lua faz o mesmo, com uma luz azulada fraca, e aparecem estrelas. As cores passam pelos ambientes prontos *amanhecer* (por volta das 6h30), o da própria cena (o dia, das 8h30 às 16h), *entardecer* (18h) e *noite* (das 19h30 às 5h). O relógio vai para as variáveis `hora` (o número, de 0 a 23), `relogio` ("14:30", para o placar) e `dia` (1, 2, ...). De noite, das 18h às 6h, acendem as Luzes com `night = true` e os brilhos com `glow_night = true`; sem o ciclo, "de noite" é um ambiente escuro (o pronto *noite* ou *caverna*).

| Na cena | O que faz |
|---|---|
| `scene:time_of_day()` | A hora (0 a 24, com os minutos na parte decimal) e o dia. |
| `scene:set_time(h)` | Pula para a hora `h` (o céu muda no próximo quadro). |
| `scene:pause_time(true/false)` | Para ou continua a hora. |
| `scene:environment(pronto, segundos)` | Troca o ambiente por um pronto (`"dia"`, `"amanhecer"`, `"entardecer"`, `"noite"`, `"caverna"`) ou por uma tabela como a `environment`, aos poucos. Com o ciclo, o tempo para; `scene:pause_time(false)` volta para as cores da hora. |
| `scene:is_night()` | De noite? |

Os componentes 3D:

| `type` | Campos (padrão) |
|---|---|
| `model` | `file` (um `.glb` ou `.gltf`), `color` ("#FFFFFF": multiplica as cores do modelo), `flat` (false: cada face com um tom só), `shadow` (true), `glow` (0 a 1: a própria cor brilha, mesmo no escuro; as partes que o `.glb` faz brilhar, com `emissiveFactor`, brilham sempre), `glow_night` (false: o brilho, e o do arquivo, só de noite), `sway` (0: quanto balança com o vento; 1 = uma árvore) |
| `shape3d` | `shape` ("cube", "sphere", "cylinder", "cone", "capsule" ou "plane"), `color`, `shadow` (true), `glow` (0 a 1: a própria cor brilha, mesmo no escuro; não ilumina em volta), `glow_night` (false: só de noite), `material` ("normal" ou "agua": ondinhas, o céu refletido e o brilho do sol), `sway` (0: quanto balança com o vento) |
| `light3d` | Uma luz: `kind` ("point": ilumina em volta; "spot": um holofote, num cone para a frente do objeto, −Z), `color`, `intensity` (1.5), `range` (8 m: até onde chega), `angle` (30°: metade da abertura do holofote), `soft` (0.3: a borda do cone que apaga aos poucos), `flicker` (0 a 1: tremula como fogo), `on` (true), `night` (false: acesa só de noite). Valem as 32 mais perto da câmera; só o sol faz sombra. `obj.light3d.on = false` apaga. |
| `camera3d` | `fov` (60): o jogo olha de onde este objeto está, para onde ele aponta (−Z). Sem câmera, de (0, 5, 10) para o centro. |
| `anim3d` | `animation` ("": nenhuma), `speed` (1), `loop` (true): a animação que o `model` do objeto toca desde o começo |
| `bone` | `bone` (""): preso a um osso do modelo do pai (uma espada na mão do herói: o osso `mao_d`). O objeto acompanha o osso quando o pai anima; a posição, a rotação e a escala dele contam a partir do osso. Um osso que não existe dá um aviso com a lista dos que existem. |
| `body3d` | `body` ("static", "dynamic" ou "kinematic"), `shape` ("auto", "box", "sphere", "capsule", "cylinder", "mesh"), `sensor`, `bounce` (0), `friction` (0.5), `mass` (0 = pelo tamanho), `lock_rotation`, `gravity_scale` (1), `tag` |
| `text3d` | `text` ("Texto"), `size` (0.5: a altura das letras em metros, vezes a escala do objeto), `color`, `facing` ("camera": sempre de frente; "placa": preso como uma placa, lido de frente pelo +Z do objeto e espelhado quando visto de trás), `outline` ("#000000B0": contorno; "" = sem), `background` ("": um retângulo atrás), `font`. Aparece atrás das paredes e com névoa; `obj.text3d.text` muda o texto (o `nn:mostrar_valor` também). |

O `body3d` é um corpo de [`nn.physics3d`](#nnphysics3d-física-3d) no mundo da cena (com a `gravity` dela; numa cena 3D, -20 por padrão). A forma `"auto"` usa a malha exata de um `model` num corpo `static` ou `kinematic`; nos outros casos, uma caixa (ou esfera, cápsula e cilindro, conforme o `shape3d`) do tamanho do modelo ou da forma, com a escala do objeto. Os comportamentos que funcionam no 3D são `nn:jogador3d`, `nn:inimigo3d`, `nn:golpe`, `nn:projetil`, `nn:porta3d`, `nn:girar3d`, `nn:plataforma3d`, `nn:vida`, `nn:perigo`, `nn:atirar`, `nn:temporario`, `nn:coletavel`, `nn:gatilho`, `nn:interagir`, `nn:inventario`, `nn:ir_para_cena`, `nn:checkpoint`, `nn:mostrar_valor` e `nn:botao` (veja os [prontos](#comportamentos-prontos)); um comportamento seu pode dizer `mode = "3d"` (ou `"2d"`, ou `"both"`) para o NN Studio só oferecer ele nesse tipo de cena. Uma propriedade também pode ter `mode = "2d"` ou `"3d"` (só aparece nesse tipo de cena) e `default3d` (o padrão numa cena 3D, como metros em vez de pixels).

No script:

| No script | O que é |
|---|---|
| `obj.z`, `obj.rx`, `obj.ry`, `obj.rz`, `obj.scale_z` | Leitura e escrita. Com corpo, a posição acompanha o corpo; mudar à mão leva o corpo junto. |
| `obj.body` | O corpo 3D (`body3d`) ou o personagem do Jogador 3D: `vx`, `vy`, `vz`, `:set_velocity`, `:impulse`, `on_ground`... |
| `obj:matrix3d()` | A [matriz](#desenhar) do objeto no mundo (somando os pais). |
| `obj:position3d()` | Onde ele está no mundo: `x, y, z`. |
| `obj:bounds3d()` | A caixa dele no mundo: `x0, y0, z0, x1, y1, z1`. |
| `obj:touching(outro, margem)` | As caixas se tocam (margem padrão 0,05 m)? |
| `obj:move_to(x, y, z)` | Leva o objeto (e o corpo) para lá, parado. |
| `self:world3()` | O mundo da física 3D da cena (`raycast`, `query_sphere`...). |
| `obj:play(nome, opts)`, `obj:animator()` | Toca uma animação do modelo do objeto (as opções do [animator](#animações-personagens-com-esqueleto)) / o animator dele. Uma animação sem `loop` manda a mensagem `animacao_terminou` (com o nome) ao terminar. |

Os eventos de toque da física 3D chegam ao `contact(e)` da cena e ao `contact(outro, e)` dos comportamentos, com `e.object_a` e `e.object_b`.

#### Interface: menus e HUD

Os componentes de interface ficam presos à tela (`hud = true`, o padrão deles). Cores vazias (`""`) usam o [tema](#configlua) do projeto.

| `type` | Campos (padrão) |
|---|---|
| `panel` | `w` (320), `h` (220), `color` ("": tema `panel`), `radius` (12), `border` (0), `border_color`, `image` (moldura em 9 fatias), `slice` (4: pixels dos cantos da imagem), `slice_scale` (4) |
| `button` | `text` ("Botão"), `size` (28), `w` (240), `h` (60), `color`, `hover_color`, `press_color`, `text_color`, `radius` (12), `border`, `border_color`, `image`, `slice`, `slice_scale`, `icon`, `font`, `action` ("nada", "cena", "por_cima", "fechar", "reiniciar", "sair"), `scene`, `sound`, `focus` (false: começa com o foco), `enabled` (true) |
| `bar` | `w` (240), `h` (24), `var` (variável do jogo), `value` (1), `max` (1), `max_var`, `color` ("": tema `accent`), `back_color`, `radius` (6), `border`, `border_color`, `direction` ("right", "left", "up", "down") |
| `icons` | `image`, `frame`, `frame_w`, `frame_h`, `var`, `count` (3, sem variável), `max` (0: só os que tem; com máximo, os que faltam ficam com `empty_color`; no máximo 100 ícones), `size` (32), `spacing` (6), `color`, `empty_color`, `align` ("left", "center", "right"), `pixel_art` |
| `slider` | `w` (240), `h` (12), `target` ("variavel", "musica", "efeitos", "geral"), `var`, `value` (0.5), `min` (0), `max` (1), `step` (0: um décimo), `color`, `back_color`, `knob_color`, `knob` (28) |
| `checkbox` | `text` ("Opção"), `size` (26), `box` (36), `target` ("variavel" ou "tela_cheia"), `var`, `checked`, `color`, `check_color`, `text_color`, `font` |
| `textfield` | `text`, `placeholder` ("Digite aqui"), `var` (guarda o texto), `w` (320), `h` (56), `size` (26), `max` (20 letras), `color`, `text_color`, `border` (2), `border_color`, `radius` (8), `font` |
| `layout` | `arrange` ("vertical", "horizontal", "grid", "none"): arruma os filhos do objeto, com `spacing` (16), `columns` (2, na grade) e `align` ("start", "center", "end") |

**Preso na tela.** A posição de um objeto da raiz que fica na tela conta a partir do ponto `anchor`: `"top_left"`, `"top"`, `"top_right"`, `"left"`, `"center"` (padrão), `"right"`, `"bottom_left"`, `"bottom"` ou `"bottom_right"`, com y para cima. `{ anchor = "top_left", x = 40, y = -40 }` fica a 40 pixels do canto em qualquer tamanho de janela.

**Foco.** Botões, deslizadores, caixas de marcar e campos de texto recebem o foco com o mouse (passando por cima), as setas ou WASD (para o item mais perto naquela direção), Tab e o direcional do controle. Enter, espaço ou o A do controle apertam; num deslizador, esquerda e direita mudam o valor; num campo de texto, as letras vão para o texto (Backspace apaga). Um botão apertado manda a mensagem `clicou` ao objeto (a condição `clicar` dos eventos); um valor que muda dispara a condição `mudou`. Os volumes e a tela cheia escolhidos ficam guardados (chave `nn_opcoes` do [nn.save](#nnsave)) e valem de novo ao abrir o jogo.

**Cenas por cima.** Com `overlay = true`, ou aberta com `nn.scene.push(cena, { por_cima = true })` (a ação `por_cima`), a cena aparece por cima da de baixo, que continua desenhada e **parada** até ela fechar (`nn.scene.pop()`, a ação `fechar_cena` ou um botão `fechar`). O `background` dela vira um véu sobre a de baixo (use uma cor transparente, como `"#00000099"`), e os objetos ficam na tela.

No script, `obj.button`, `obj.bar`, `obj.slider`... são os componentes (`obj.button.enabled = false`, `obj.bar.max = 5`), e `self:open_over(cena)` e `self:close_over()` abrem e fecham cenas por cima.
| `camera` | `follow` (nome do objeto a seguir), `smooth` (6), `zoom` (1), `limit_to_map` (true) |
| `sound` | `sound`, `autoplay` (true), `loop`, `volume` (1), `channel` ("effects" ou "music"), e numa cena 3D `spatial` (true: o som sai do objeto) e `distance` (30 m: mais longe, não se ouve) |
| `behavior` | `script` (`"nn:plataforma"` para um [pronto](#comportamentos-prontos) ou o caminho de um `.lua`) e as propriedades dele, soltas: `{ type = "behavior", script = "nn:patrulha", velocidade = 60 }` |
| `particles` | `mode` ("continuo" ou "explosao"), `rate` (20 por segundo), `emitting` (true), `amount` (30, de uma vez), `on_start` (true: solta ao começar), `interval` (0: repetir a explosão a cada N s), `when_done` ("nada" ou "destruir": o objeto some quando as partículas acabam), `lifetime` (1), `lifetime_var` (0 a 1), `speed_min`, `speed_max`, `direction` (90, em graus), `spread` (360), `gravity` (negativa cai), `drag`, `area` ("ponto", "retangulo", "circulo"), `area_w`, `area_h`, `area_radius`, `size_start`, `size_end`, `size_var`, `color_start`, `color_mid` ("" = sem), `color_end`, `image`, `frame`, `frame_w`, `frame_h`, `spin` (graus por segundo), `blend` ("alpha" ou "add"), `follow` (false: as partículas acompanham o objeto), `max` (500), `wind` (1: no 3D, quanto o vento do ambiente leva as partículas; 0 = nada). Numa cena 3D, os números são em metros, as partículas saem para o "cima" do objeto (gire o objeto para apontar; `direction` não conta), `spread` é um cone em graus e as áreas ficam deitadas (`area_w` × `area_h` no chão, ou um círculo no chão) |

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

- **Propriedades** (`props`): lista de `{ "nome", padrão, ... }`. Cada uma vira `self.<nome>`, com o valor que o objeto tem no `.cena` (ou o padrão). Opções: `label`, `help`, `min`, `max`, `integer = true`, `options = { { valor, "texto" }, ... }` (escolha), `advanced = true` (só no modo Completo), `show_if = { "outra", valores... }` (só aparece quando outra propriedade tem um desses valores) e `type`, que normalmente é deduzido do padrão (número, sim/não, cor `"#RRGGBB"`, texto). Para o Inspetor mostrar a lista certa, `type` pode ser `"object"` (nome de um objeto da cena), `"scene"`, `"prefab"`, `"sound"`, `"image"`, `"dialogue"` (um `.dialogo`), `"anim"` (uma animação da imagem ou do modelo do objeto), `"bone"` (um osso do modelo 3D do objeto), `"color"` ou `"choice"`. Também vale um mapa simples: `props = { velocidade = 90 }`.
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
| `nn:vida` | (2D e 3D) Pontos de vida; recebe `dano(n, quem)` (quem bateu) e `curar(n)`. | `vidas` (3), `variavel` ("vidas": espelha numa variável do jogo; "vidas" só vale no jogador, num inimigo use outro nome), `invencivel` (1 s), `ao_acabar` ("reiniciar", "checkpoint", "cena", "destruir", "nada"), `cena`, `piscar` (true), `barra` (false: uma barra de vida flutuando em cima), `cor_barra`, `numeros` (false: um número sobe de quem leva dano, com quanto tirou), `vermelho` (true: fica vermelho um instante ao levar dano), `tremer` (0: a força do tremor da câmera ao levar dano; 6 é leve) |
| `nn:patrulha` | Anda até uma parede (ou a beirada) e volta. Precisa de corpo. | `velocidade` (120), `direcao` ("direita"), `beirada` (true), `espelhar` (true) |
| `nn:perseguir` | Vai atrás do alvo quando ele chega perto. Voando (ou num jogo visto de cima), contorna as paredes pelos [caminhos](#caminhos-inimigos-que-contornam-paredes). | `alvo` ("" = jogador), `velocidade` (150), `visao` (320), `voar` (false), `contornar` (true, só voando), `espelhar` (true) |
| `nn:perigo` | (2D e 3D) Machuca quem encosta. | `quem` (""), `acao` ("dano", "checkpoint", "reiniciar"), `dano` (1), `pisar` (false: pular em cima destrói), `some` (false), `empurrao` (520; no 3D, 7 m/s) |
| `nn:inimigo3d` | (3D) Anda sozinho, persegue o jogador quando ele chega perto e machuca quem encosta. Cria o próprio corpo (como o Jogador 3D) e, num modelo animado, toca parado e andar. Pisado: manda `dano` para a Vida dele (sem Vida, some), `derrotado` e, para quem pisou, `derrotou`. Com um [Golpe](#combate-golpes-tiros-e-dano) no mesmo objeto, só o golpe machuca (encostar não). Empurrado (por um golpe ou um tiro), para de andar um instante. | `andar` ("patrulha": de onde começa até `x`, `z` e volta; "vagar": sem rumo até `raio` m; "parado"), `x` (6), `z` (0), `raio` (6), `velocidade` (2), `perseguir` (true), `visao` (8 m; desiste a 1,6 × isso), `correr` (3.5), `contornar` (true: acha um [caminho](#caminhos-inimigos-que-contornam-paredes) em volta de paredes, árvores, buracos e água), `dano` (1), `empurrao` (7), `pisar` (true), `virar` (true), `animar` (true), `anim_parado`, `anim_andar` |
| `nn:coletavel` | Some ao ser tocado e soma numa variável; pode guardar um item no [inventário](#inventário-nninventory). | `quem` (""), `variavel` ("moedas"; "" = não soma), `valor` (1), `item` ("": o nome do item que vai para o inventário, como "chave"), `icone` (a imagem do item; vazio = a do objeto, no 2D), `som`, `brilho` (true), `cor_brilho` |
| `nn:gatilho` | (2D e 3D) Zona de gatilho: avisa quando alguém entra ou sai dela (pela caixa do objeto) e some no jogo. Manda `entrou` e `saiu` (com quem) para a própria zona e a mensagem escolhida para os objetos `alvo`. | `quem` (""), `alvo` (""), `mensagem` ("abrir"), `mensagem_saida` (""), `uma_vez` (false), `som`, `invisivel` (true) |
| `nn:interagir` | (2D e 3D) Perto do objeto aparece "E  usar"; apertando E (ou o Y do controle) ele manda `interagiu` (com quem) e a `mensagem` para os objetos `alvo`. Pode pedir um item do inventário (sem ele, o aviso diz o que falta e manda `falta_item`). Só o objeto mais perto do jogador mostra o aviso. | `texto` ("usar"), `distancia` (24 px; no 3D, 1.2 m, da borda), `quem` (""), `alvo` (""), `mensagem` ("abrir"), `item` (""), `gastar` (true), `texto_sem_item` ("Precisa de: {item}"), `uma_vez` (false), `som` |
| `nn:porta3d` | (3D) Porta: abre girando na dobradiça (para longe de quem abre), subindo, descendo ou deslizando. Abre com E perto dela, sozinha quando o jogador chega ou só por mensagem. Pode pedir uma chave do inventário (mensagens abrem mesmo trancada). Precisa de um `body3d` kinematic (que bloqueia a passagem). Recebe `abrir`, `fechar`, `alternar` e `destrancar`; manda `abriu`, `fechou` e `trancada`. | `abre` ("girando", "subindo", "descendo", "deslizando"), `lado` ("esquerda", "direita"), `angulo` (100), `tempo` (0.6 s), `abrir_com` ("tecla", "chegar", "mensagem"), `distancia` (1.2 m), `chave` (""), `gastar_chave` (true), `fechar_depois` (0 s = fica aberta), `texto` ("abrir"), `texto_fechar` ("fechar"; vazio = não fecha com a tecla), `texto_trancada`, `som`, `som_trancada` |
| `nn:dialogo` | (2D e 3D) Um personagem que conversa: abre um [diálogo](#diálogos-dialogo) numa caixa de fala. Com `comecar = "tecla"`, perto dele aparece "E  falar" (o mesmo aviso do Interagir). Também começa com a mensagem `conversar`. | `arquivo` (o `.dialogo`), `comecar` ("tecla", "chegar": quando o jogador chega perto, "cena": quando a cena começa, "mensagem"), `texto` ("falar"), `distancia` (24 px; no 3D, 1.5 m), `uma_vez` (false), `olhar` (3D, true: vira para quem fala com ele) |
| `nn:inventario` | (2D e 3D) Mostra os itens do inventário num canto da tela, com quantos tem de cada (sem ícone, a primeira letra). | `canto` ("baixo_esquerda", "baixo_direita", "cima_esquerda", "cima_direita"), `tamanho` (48), `margem` (20), `fundo` (true) |
| `nn:checkpoint` | Ponto de reinício. | `quem` ("") |
| `nn:ir_para_cena` | Leva para outra cena. | `cena`, `quem` (""), `tecla` (false: precisa apertar para cima / Enter), `escurecer` (0.5 s) |
| `nn:movimento` | Gira, flutua ou vai e volta (com corpo cinemático, carrega quem está em cima). | `tipo` ("ir_e_voltar", "flutuar", "girar"), `distancia_x` (200), `distancia_y` (0), `altura` (12), `duracao` (2), `velocidade` (90) |
| `nn:animacao` | Troca a animação conforme o estado. Precisa de imagem. | `parado` ("1"), `andando`, `pulando`, `caindo` (o nome de uma [animação](#animações-anim) da imagem, como `"andar"`, ou quadros como `"1-4"` e `"5,6"`; vazio usa o anterior), `fps` (10, para quadros), `espelhar` (true) |
| `nn:mostrar_valor` | Escreve variáveis num texto (`text` ou `text3d`). | `texto` ("Moedas: {moedas}") |
| `nn:atirar` | (2D e 3D) Cria um prefab andando. No 3D, sai da frente do objeto (o +Z do modelo), na altura escolhida, ou de um osso; com Corpo 3D, sem gravidade. No 3D, com "camera", vai para o meio da tela (também para cima e para baixo), e a câmera do Jogador 3D vai para o ombro. Manda `atirou` (com o tiro). | `prefab`, `tecla` ("x"; no 3D, "clique_direito"; "clique" é o botão esquerdo; vazio = sozinho; no controle, o X, e no 3D o RT ou o RB), `intervalo` (0.3), `velocidade` (700; no 3D, 14 m/s), `direcao` ("frente"; no 3D, "camera"; "jogador", "direita", "cima"), `som`, `distancia` (24; no 3D, 0.8 m), `altura` (3D: 1 m), `osso` (3D, "": sai desse osso do modelo, como "mao_d"), `mira` (3D, true: uma cruzinha no meio da tela), `animacao` (3D, "atirar"), `atraso` (3D, 0.08 s: o tiro sai depois que a animação começa), `duracao` (3D: o tiro some depois de 3 s) |
| `nn:golpe` | (3D) Ataque de perto: toca a animação do golpe e, no instante do acerto, acerta quem está na frente (dentro do alcance e da largura), tira vida e empurra. O jogador golpeia com o clique (ou J, ou o X do controle); um inimigo, sozinho, quando o alvo chega perto (ele para, vira e golpeia). Manda `golpeou` ao começar e `acertou` (alvo, dano) a cada acerto. | `quando` ("tecla", "perto"), `tecla` ("clique"), `distancia` (1.3 m, no "perto"), `alvo` ("": os inimigos; veja [Combate](#combate-golpes-tiros-e-dano)), `dano` (1), `alcance` (1.6 m), `angulo` (120°), `todos` (true; false = só o mais perto), `empurrao` (6 m/s), `intervalo` (0.45 s), `animacao` ("golpe"), `acerto` (0.22 s), `parar` (true: para de andar golpeando), `som`, `som_acerto`, `faiscas` (true), `pausa_acerto` (0.06 s: o jogo para um instante quando o golpe acerta, só com o jogador) |
| `nn:projetil` | (3D) Para o prefab de um tiro: anda (com a velocidade que o Atirar deu, ou a do corpo), acerta quem tem Vida (tira vida e empurra), some ao bater numa parede (com faíscas) e não acerta quem atirou. Manda `acertou` e `bateu`. | `dano` (1), `alvo` ("": os inimigos de quem atirou), `empurrao` (4 m/s), `atravessa` (false), `parede` (true: some ao bater), `gravidade` (0: reto; 1 = cai como uma pedra), `dura` (4 s), `cor` (das faíscas) |
| `nn:gerador` | (2D e 3D) Cria um prefab de tempos em tempos. No 3D, num lugar do chão onde dá para andar. | `prefab`, `intervalo` (2), `maximo` (10), `espalhar_x`, `espalhar_y` (0, só 2D), `raio` (3D, 0 m: espalha até essa distância), `longe` (3D, 0 m: não cria colado no jogador), `comecar_ja` (false) |
| `nn:ondas` | (3D) Ondas de inimigos: cria um prefab em ondas (onda 1, 2, 3...), cada uma com mais; a próxima vem depois de uma pausa, quando todos da anterior foram derrotados. Os filhos do objeto são de onde eles saem (sem filhos, em volta, no chão). Mostra um letreiro ("Onda 2") e guarda a onda e quantos faltam em variáveis. Recebe `comecar` e `parar`; manda `ondas_comecaram`, `onda_comecou` (n), `onda_terminou` (n) e `ondas_terminaram`. | `prefab`, `prefab_forte` (""; a partir de `forte_a_partir` = 2, um em cada três), `ondas` (3), `primeira` (3 inimigos), `a_mais` (2 por onda), `quando` ("chegar": o jogador a `distancia` = 6 m; "cena"; "mensagem"), `pausa` (3 s), `intervalo` (0.5 s entre um inimigo e outro), `raio` (8 m), `longe` (3 m do jogador), `letreiro` (true), `texto` ("Onda {onda}"; `{ondas}` = quantas são), `texto_fim` ("Você venceu!"), `var_onda` ("onda"), `var_restantes` ("restantes"), `som`, `cor` (das faíscas ao surgir) |
| `nn:chefe` | (3D) Um inimigo grande com vários ataques, cada um com um aviso antes (pisca em branco e toca "preparar"): golpe na frente, investida (corre reto; se bater numa parede, fica tonto), pulo com impacto (cai na frente do jogador e acerta quem está no chão em volta), tiros em leque (um prefab com Projétil) e chamar ajudantes. Cria o próprio corpo (como o Inimigo 3D), é pesado (golpes não empurram), anda atrás do jogador contornando paredes e machuca ao encostar. Com pouca vida, fica furioso (mais rápido, menos pausa). Mostra o nome e a vida no alto da tela. Precisa de Vida (de preferência com "Quando acabar: nada": ele toca "cair" e some). Começa a lutar quando o jogador chega perto, com a mensagem `comecar` ou ao levar dano. Manda `luta_comecou`, `aviso` (o ataque), `atacou` (o ataque), `tonto`, `impactou` e `ficou_furioso`. | `nome` ("Chefe"), `quando` ("chegar", "cena", "mensagem"), `distancia` (10 m), `velocidade` (2.5), `pausa` (1.4 s), `aviso` (0.7 s), `dano` (1), `golpe` (true), `alcance` (2.4 m, do meio dele), `investida` (true), `velocidade_investida` (11), `tonto` (1.6 s), `pulo` (true), `raio_impacto` (4 m), `leque` (false), `tiro` (o prefab), `tiros` (5), `abertura` (70°), `ajudantes` (false), `ajudante` (o prefab), `quantos` (2), `furia` (50: com essa porcentagem da vida), `barra` (true), `cor_barra`, `solta` (um prefab que aparece onde ele caiu), `som_aviso`, `som_impacto`, `anim_parado`, `anim_andar`, `anim_golpe`, `anim_aviso` ("preparar"), `anim_pulo` ("pular"), `anim_tonto`, `anim_cair` |
| `nn:temporario` | (2D e 3D) Some depois de um tempo (no 2D, também ao sair da tela). | `tempo` (3), `fora_da_tela` (true, só 2D) |
| `nn:jogador3d` | (3D) Jogador 3D: anda para onde a câmera olha (WASD, setas, analógico), pula (espaço, A) e leva a câmera em 3ª pessoa. Cria o próprio corpo (uma cápsula do tamanho do modelo). Manda `pulou` e `caiu`. | `velocidade` (6), `pulo` (8), `pulos_no_ar` (0), `camera` ("atras" ou "cena"), `distancia` (7), `prender_mouse` (true), `sensibilidade` (0.15), `inverter_y` (false), `ombro` (0 m: a câmera vai para a direita, para mirar), `virar` (true), `agua` ("voltar": na água de um Terreno, volta ao último chão firme; "nada": anda no fundo; nos dois, recebe a mensagem `agua`), `animar` (true: parado, andar, correr, pular e cair num modelo animado), `anim_parado`, `anim_andar`, `anim_correr`, `anim_pular`, `anim_cair` (os nomes das animações), `queda` (-10), `raio`, `altura` (0 = pelo modelo) |
| `nn:girar3d` | (3D) Gira em volta de um eixo e flutua. | `velocidade` (90 graus/s), `eixo` ("y", "x", "z"), `altura` (0.2 m), `duracao` (2 s) |
| `nn:plataforma3d` | (3D) Vai e volta a partir de onde está; com um `body3d` kinematic, leva quem está em cima. | `x` (4), `y` (0), `z` (0), `duracao` (3 s), `espera` (0.5 s) |
| `nn:botao` | Botão de menu: destaca com o mouse ou as setas (teclado e controle); clique, Enter ou A apertam. Manda a mensagem `clicou` e faz a ação. | `acao` ("nada", "cena", "reiniciar", "sair"), `cena`, `foco` (false: começa selecionado), `cor` ("#FFD166": cor ao destacar), `aumentar` (1.1), `som` |

A **Vida** também tem `ao_acabar = "nada"` (um evento decide o que fazer) e avisa por mensagem: `levou_dano` (o dano e quem bateu) a cada dano e `vida_acabou` no fim; quem bateu por último recebe `derrotou` (com o derrotado). O **Coletável** manda `coletou` (com o item) para quem pegou. Essas mensagens chegam aos comportamentos (como funções) e aos [eventos](#eventos-quando-então).

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

### Inventário (nn.inventory)

Os itens que o jogador carrega (chaves, poções, um mapa) ficam no inventário. Ele mora em `nn.vars._itens`: passa de uma cena para outra, volta ao começo com `nn.reset_vars()` e vai junto quando o jogo salva as variáveis. O [Coletável](#comportamentos-prontos) guarda itens (`item = "chave"`), a Porta 3D e o Interagir pedem itens pelo nome, e o `nn:inventario` mostra o que tem na tela.

| Função | O que faz |
|---|---|
| `nn.inventory.add(item, n, icone)` | Guarda `n` (1) do item; `icone` é a imagem que o inventário mostra. Devolve quantos tem agora. |
| `nn.inventory.remove(item, n)` | Tira `n` (1), se tiver tudo isso; devolve `true` se tirou. |
| `nn.inventory.count(item)` / `nn.inventory.has(item, n)` | Quantos tem (0 se nenhum) / tem pelo menos `n` (1)? |
| `nn.inventory.items()` | Os itens, na ordem em que foram pegos: `{ {item =, count =, icon =}, ... }`. |
| `nn.inventory.clear()` | Esvazia. |

```lua
if nn.inventory.has("chave") then
  nn.inventory.remove("chave")
  porta:send("abrir")
end
```

### Diálogos (.dialogo)

Conversas com falas, escolhas e personagens ficam em arquivos `.dialogo`, montados no [editor de conversa](studio.md#editor-de-conversa) do NN Studio. Uma conversa abre numa **caixa de fala** no pé da tela, com o tema do projeto (`theme` do `config.lua`: painel, borda, texto, destaque, fonte e som): o nome de quem fala numa etiqueta da cor do personagem, o retrato (se houver) e o texto aparecendo letra por letra. **E**, Enter, espaço, o A do controle ou um clique mostram o texto todo e passam para a próxima fala; as setas (ou W e S), os números e o mouse escolhem. Enquanto ela está aberta, os jogadores prontos (Jogador 3D, de plataforma e top-down) ficam parados e o Interagir e as portas não respondem.

```lua
return {
  format = 1,
  start = 1,        -- a primeira fala
  speed = 40,       -- letras por segundo (0 = o texto todo de uma vez)
  characters = {
    { name = "Ana", color = "#FFD166", image = "assets/imagens/ana.png", sound = "assets/sons/voz.wav" },
  },
  nodes = {
    { id = 1, x = 40, y = 60, who = "Ana", text = "Oi, {nome}! Viu minha chave?", next = 2 },
    { id = 2, x = 340, y = 60, who = "", text = "Ela parece preocupada.",
      choices = {
        { text = "Vi sim", next = 3, var = "ajudou", value = 1 },
        { text = "Não", next = 0 },
        { text = "Toma, achei", next = 3, need = "chave" },
      } },
    { id = 3, x = 640, y = 60, who = "Ana", text = "Obrigada!", item = "moeda rara", next = 0 },
  },
}
```

- **Falas** (`nodes`): `who` é o nome de um personagem (vazio = narrador, sem etiqueta), `text` aceita `{variavel}`, `next` é a próxima fala (0 = termina) e `x`, `y` são a posição do cartão no editor.
- **Escolhas** (`choices`): o jogador escolhe uma; cada uma leva a uma fala (`next`), pode mudar uma variável (`var`, `value`) e só aparece para quem tem um item no [inventário](#inventário-nninventory) (`need`).
- **Ao aparecer**, uma fala pode mudar uma variável (`var`, `value`) e dar um item (`item`).
- **Personagens**: `color` é a cor da etiqueta do nome, `image` o retrato e `sound` um som curto que toca enquanto o texto aparece.
- **Mensagens** para quem fala (o objeto com o Diálogo): `conversa_comecou` e `conversa_terminou` (com o arquivo), `escolheu` (com o texto da escolha) e `fala` (com o id). Nos [eventos](#eventos-quando-então): `conversa_comecou`, `conversa_terminou` e `escolheu`.

No script da cena, `self:start_dialogue("dialogos/ana.dialogo", ana)` abre uma conversa (`ana`, quem fala, é opcional; devolve `false` se já houver uma aberta), `self:stop_dialogue()` fecha e `self:dialogue()` é o arquivo da conversa aberta (ou `nil`). Num comportamento, `self.scene:start_dialogue(...)`.

### Interações ("E para abrir")

O Interagir e a Porta 3D usam o mesmo aviso: perto do objeto aparece a tecla (E, ou Y se o controle foi o último usado) e o que ela faz. De tudo o que está perto, só o mais perto do jogador mostra o aviso e responde à tecla. Um comportamento seu pode usar o mesmo sistema: chame `self.scene:interaction(self, quem, distancia, texto, trancado)` no `update`; ele devolve o objeto que apertou a tecla (ou `nil`). `self.scene:gap(a, b)` é a distância entre as caixas de dois objetos (0 se encostam).

### Caminhos (inimigos que contornam paredes)

Cada cena com física tem uma **grade de caminhos**: o chão dividido em quadradinhos, cada um marcado como andável ou não. Ela é montada sozinha na primeira vez que alguém pede (e de novo quando um corpo parado aparece ou some):

- **No 3D**, células de 0,5 m. Um raio procura o chão de cada uma (os corpos parados: chão, terreno, cenário) e confere se cabe alguém em pé ali. Debaixo de uma plataforma no ar vale o chão; uma ponte por cima da água vale. Não é andável: paredes, árvores, o que não tem chão (buracos), a água dos terrenos e um desnível maior que um degrau (45 cm) entre duas células (barrancos).
- **No 2D** (jogos vistos de cima), células de 16 unidades. Bloqueiam os corpos parados (menos sensores e plataformas de mão única) e os tiles sólidos, com uma folga de 12.
- Corpos que se movem (portas, plataformas que andam, caixas) não entram na grade: quem anda desvia deles pela física.

O Inimigo 3D e o Perseguir (voando) usam a grade com **Contorna paredes** ligado: vão em linha reta quando dá e, quando não dá, seguem o caminho, refeito a cada meio segundo. No NN Studio, com o jogo aberto, **🧭 Caminhos** (na barra da cena) mostra no jogo as células andáveis e o caminho de cada inimigo; `NN_CAMINHOS=1` faz o mesmo ao abrir o jogo pelo terminal.

No Lua:

| Função | O que faz |
|---|---|
| `scene:path(a, b)` | Um caminho de `a` até `b` (objetos ou pontos `{x, y}`; no 3D, `{x, z}`): a lista de pontos (`{x, y}`, ou `{x, y, z}` no 3D, com o y do chão) e `true` se chega mesmo lá (`false`: leva ao lugar mais perto). |
| `scene:steer(estado, x, y, tx, ty, dt)` | Para um comportamento seu: para onde andar agora, indo de (x, y) para (tx, ty) e contornando o que bloqueia (no 3D, x e z). `estado` é uma tabela sua, que guarda o caminho entre um quadro e outro. Guarde-a também em `self.obj._nav_state` para o botão Caminhos mostrar. |
| `scene:nav()` | A grade da cena (ou `nil` sem física). |
| `nav:path(x1, y1, x2, y2)` | Como `scene:path`, com números. |
| `nav:walkable(x, y)`, `nav:sight(x1, y1, x2, y2)` | Dá para andar ali? Dá para ir reto de um ponto ao outro? |
| `nav:floor(x, z)` | A altura do chão (3D). |
| `nav:block(x1, y1, x2, y2)`, `nav:unblock(...)` | Bloqueia ou libera um retângulo (uma ponte que caiu, uma porta trancada). |
| `nav.cell`, `nav.columns`, `nav.rows`, `nav.walkable_count`, `nav:bounds()` | O tamanho das células, da grade e o pedaço coberto. |

Para montar uma grade com outras medidas, ponha antes `scene._nav_opts = { cell = 0.25, radius = 0.3, height = 1.8, step = 0.3 }` e marque `scene._nav_dirty = true`. Fora das cenas visuais, `mundo:nav(opts)` (de [`nn.physics`](#nnphysics) ou [`nn.physics3d`](#nnphysics3d-física-3d)) devolve uma grade nova; no 3D, chame depois de um `mundo:step`.

### Combate (golpes, tiros e dano)

No 3D, o **Golpe** (de perto) e o **Atirar** com um prefab que tem **Projétil** (de longe) tiram vida do comportamento **Vida**. Quem pode ser acertado ("Acerta" vazio):

- **Quem ataca é o jogador:** quem tem Vida ou é um Inimigo 3D (menos os jogadores). Um Inimigo 3D (ou um Perigo) sem Vida é derrotado de uma vez.
- **Quem ataca é outro (um inimigo):** os jogadores.
- **Com um nome em "Acerta":** só os objetos com esse nome.

A cada acerto, o alvo é empurrado para longe de quem bateu, quem bateu recebe a mensagem `acertou` (o alvo e o dano) e, se a vida acabou, `derrotou`. Os eventos **Acertar** e **Derrotar** usam essas mensagens, e o **Levar dano** sabe quem bateu (o `@outro`).

Para segurar uma arma, ponha o objeto da arma como **filho** do personagem e dê a ele o componente **Preso a um osso** (`bone`) com a mão (`mao_d` no herói dos modelos). Os modelos de projeto 3D trazem a `espada.glb` (o cabo na origem, a lâmina para a frente) e o `cajado.glb` (para cima). O herói tem as animações `golpe` (o acerto é perto de 0,22 s) e `atirar`, e o besouro (`bicho.glb`) tem um `golpe` (a mordida, perto de 0,3 s). O Vale 3D já vem com a espada na mão do herói, besouros que mordem e o prefab `magia.prefab` (uma bola com Projétil, para um Atirar).

No Lua:

| Função | O que faz |
|---|---|
| `scene:hit(alvo, dano, quem, opts)` | `quem` acerta o `alvo`: tira vida, empurra e manda as mensagens acima. Opções: `push` (o empurrão em m/s), `dx`, `dz` (a direção; sem elas, para longe de `quem`), `up` (0.5: quanto do empurrão vai para cima), `stun` (0.3 s sem andar), `kill` (sem Vida, derrota mesmo assim), `hitstop` (segundos de pausa no acerto, quando o jogador bate ou apanha). Devolve `true` se acertou, `false` se o alvo estava invencível e `nil` se não dá para machucar ele. |
| `scene:foes(quem, nome)` | Quem `quem` pode acertar (as regras acima; com `nome`, os objetos com esse nome). |
| `scene:damage_number(obj, n, cor)` | Um número subindo de cima do objeto, que some sozinho (amarelo; vermelho no jogador). |
| `scene:shake(forca, tempo)` | Treme a câmera (a ação Tremer também). No 3D, 80 de força = 1 metro. |
| `scene:hitstop(segundos)` | Para o jogo um instante (no máximo 0,3 s): a "pausa no acerto". O desenho continua. |
| `scene:in_front(quem, alcance, angulo, nome, todos)` | Quem `quem` pode acertar na frente dele (até `alcance` m da borda de cada um, dentro de `angulo` graus), do mais perto para o mais longe; com `todos = false`, só o mais perto. É o que o Golpe usa. |
| `scene:front3(obj)` | A frente do objeto no chão (o +Z do modelo, girado com ele): `dx, dz`. |
| `scene:ground_spot(x, y, z, raio, longe)` | Um lugar no chão até `raio` m de (x, z), onde dá para andar (pela grade dos [caminhos](#caminhos-inimigos-que-contornam-paredes)) e a pelo menos `longe` m dos jogadores: `x, y, z`. As Ondas e o Gerador usam. |
| `scene:warn(texto)` | Um aviso no console do NN Studio (filtro Avisos), uma vez por texto. |
| `obj._tint` | Uma cor que pinta o modelo, a forma ou a imagem do objeto por cima da dele (`nil` = a dele); a Vida usa para o vermelho. |

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
| `amanhecer` | `quando` ("anoitecer": às 18h; "amanhecer": às 6h), com Dia e noite |
| `de_noite` | `quando` ("noite" ou "dia") |
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
| `levou_dano` / `vida_acabou` | `quem` (precisa do comportamento Vida). No `levou_dano`, o `@outro` é quem bateu. |
| `acertar` / `derrotar` | `quem` (quem bateu), `alvo` ("" = qualquer um): um golpe ou um tiro tirou vida / acabou com a vida de alguém. O `@outro` é quem levou. |
| `onda` | `quem` (o objeto com Ondas de inimigos), `quando` ("comecar", "terminar"), `onda` (0 = qualquer uma) |
| `ondas_terminaram` | `quem`: a última onda foi derrotada |
| `animacao_terminou` | `quem`, `nome` ("" = qualquer uma; só animações de modo "once") |
| `particulas_terminaram` | `quem` (uma explosão acabou, ou um emissor desligado ficou sem partículas) |
| `mensagem` | `quem`, `msg` (de `obj:send("msg")`, de comportamentos ou da ação `mensagem`) |
| `entrar_zona` / `sair_zona` | `quem` (a zona, com o comportamento Zona de gatilho), `por` (quem entra ou sai; "@outro" nas ações) |
| `interagir` | `quem` (o objeto com Interagir), `por` (quem apertou E) |
| `tem_item` | `item`, `qtd` (o jogador tem pelo menos isso no [inventário](#inventário-nninventory)) |
| `conversa_comecou` / `conversa_terminou` | `quem` (quem fala; "" = qualquer um) |
| `escolheu` | `opcao` (o texto da escolha; "" = qualquer uma), `quem` |
| `mudou` | `quem` (um deslizador, caixa de marcar ou campo de texto mudou pelo jogador) |

Ações (`act`):

| `act` | Campos |
|---|---|
| `variavel` | `var`, `op` ("somar", "subtrair", "definir", "maior", "menor"), `valor` |
| `salvar` / `carregar` | `var` ("" = todas): grava no disco com [nn.save](#nnsave) (chave `vars.<nome>`) / lê de volta |
| `zerar` | — (`nn.reset_vars()`) |
| `ir_para_cena` | `cena`, `como` ("trocar"; "guardar": a cena de agora fica como está e `fechar_cena`, na outra, volta para ela), `escurecer` (0.5 s) |
| `reiniciar` | `como` ("comeco", "checkpoint", "zerar": do começo, zerando as variáveis) |
| `por_cima` | `cena` (abre por cima: a de baixo fica parada) |
| `fechar_cena` | — (fecha a cena aberta por cima, ou aberta com `guardar`, e volta para a de antes) |
| `luz` | `acao` ("acender", "apagar" ou "alternar"), `alvo` (precisa do componente `light3d`) |
| `mudar_hora` | `hora` (0 a 24, com Dia e noite) |
| `tempo` | `acao` ("parar" ou "continuar") |
| `mudar_ambiente` | `pronto` ("dia", "amanhecer", "entardecer", "noite" ou "caverna"), `segundos` (2) |
| `cor_luz` | `alvo`, `cor`, `forca` (0 = a mesma) |
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
| `conversa` | `arquivo` (um `.dialogo`), `quem` (quem fala) |
| `dar_item` / `tirar_item` | `item`, `qtd`, `icone` (só no `dar_item`): guarda no inventário / gasta (se não tiver tudo, não tira nada) |
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

**Som 3D:** com `x`, `y` e `z`, o som sai daquele ponto do mundo: fica mais baixo longe da câmera 3D (quem ouve) e vai para o lado esquerdo ou direito conforme onde está. `distance` (padrão 30) é a distância em que ele some; até um quarto dela, o volume é cheio. O volume e o lado acompanham a câmera a cada quadro, e `som:set_position(x, y, z)` leva o som junto com algo que anda:

```lua
self.fogo = nn.audio.play(nn.assets.sound("sons/fogueira.wav"), { x = 4, y = 0, z = -2, distance = 15, loop = true })
-- se a fogueira andar:
self.fogo:set_position(x, y, z)
```

O som 3D tem também `:position()` e `:distance_volume()` (de 0 a 1: quanto a distância deixa).

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

O mundo tem `:step(dt)` (chame em `fixed_update`), `:body(opts)`, `:add_tilemap(mapa)`, `:raycast(x, y, dx, dy, distancia, mascara)` (devolve `{ x, y, nx, ny, distance, body }` ou `nil`), `:query_point(x, y)`, `:query_rect(x, y, w, h)`, `:bodies(tag)`, `:debug_draw()`, `:nav({ cell = 16, radius = 12, area = { x1, y1, x2, y2 } })` (a [grade de caminhos](#caminhos-inimigos-que-contornam-paredes)) e os campos `gravity_x`, `gravity_y` e `count`.

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

#### Terreno

`nn.d3.terrain(opts)` cria um terreno: uma grade de alturas com quatro cores pintadas por cima (o componente `terrain` das cenas usa o mesmo). Ele fica centrado na posição em que é desenhado: vai de `-width/2` a `width/2` em X e de `-depth/2` a `depth/2` em Z.

```lua
local chao = nn.d3.terrain({ width = 64, depth = 64, cells = 64, height = 20 })
for i = 1, 20 do chao:brush("subir", 10, -5, 8, 0.5) end   -- um morro
chao:paint(3, 10, -5, 3)                                    -- pedra no topo
local mundo = nn.physics3d.world()
mundo:body({ terrain = chao })                              -- o chão da física

function cena:draw() nn.d3.draw(chao) end
```

| Opção | Descrição |
|---|---|
| `width`, `depth` | Tamanho em metros (padrão 64). |
| `cells` | Quadrados de cada lado (padrão 64; até 256). |
| `height` | As alturas vão de `-height` a `height` (padrão 20). |
| `data` | As alturas e as cores guardadas por `:encode()` (o NN Studio grava assim). Vazio: plano. |
| `colors` | As quatro cores: `{ grama, terra, pedra, areia }`. |

| No terreno | O que faz |
|---|---|
| `t:height_at(x, z)` | A altura do chão (nil fora). |
| `t:brush(pincel, x, z, raio, força, altura)` | `"subir"`, `"descer"`, `"alisar"`, `"aplainar"` (até `altura`; sem ela, a do ponto) ou `"ruido"`. A força é em metros no meio do pincel (alisar e aplainar: 0 a 1). |
| `t:paint(camada, x, z, raio, força)` | Pinta a camada 1 a 4 (força 0 a 1, padrão 1). |
| `t:raycast(ox, oy, oz, dx, dy, dz, max)` | Onde um raio bate no chão: `x, y, z`. |
| `t:set_colors({...})`, `t:encode()` | Troca as cores / guarda tudo num texto. |
| `t.width`, `t.depth`, `t.cells`, `t.max_height`, `t.version` | `version` conta as mudanças. |

`nn.d3.draw(t, opts)` desenha (low-poly: cada triângulo com a sua cor) e `world:body({ terrain = t, x, y, z })` cria o chão exato da física (parado). Nas cenas, o componente `terrain` tem `data`, `cells`, `width`, `depth`, `height`, `color1` a `color4`, `water` (false), `water_level` (-1, em relação ao terreno) e `water_color` ("#3FA7D6B4", meio transparente); o terreno usa só a posição do objeto, `obj.terrain` é o terreno (mudar ele pelo script refaz o chão da física) e `self:water_level(x, z)` diz a altura da água ali (ou nil). Malhas com cor meio transparente (como a água) são desenhadas depois das outras, de trás para a frente.

#### Partículas 3D

`nn.particles3d(opts)` faz o mesmo no mundo 3D, em metros: cada partícula é um quadrado sempre virado para a câmera, que aparece na frente e atrás dos objetos certos e pega a névoa (sem sombra).

```lua
local fogo = nn.particles3d({
  x = 0, y = 0.2, z = -3,
  rate = 60, lifetime = { 0.4, 0.8 }, speed = { 0.6, 1.6 },
  direction = { 0, 1, 0 }, spread = 25,     -- para cima, num cone de 25°
  gravity = 1.5,                            -- sobe (número = y; ou {x, y, z})
  area = { radius = 0.25 },                 -- num círculo deitado
  size = { 0.5, 0.1 },
  colors = { "#FFE066", "#FF7A1A", "#B0201000" }, blend = "add",
})

function cena:update(dt) fogo:update(dt) end
function cena:draw() fogo:draw() end
```

As opções são as do 2D, com estas diferenças: `direction` é `{x, y, z}` (padrão: para cima), `spread` e `spin` são em **graus** (`spread = 360`: todos os lados), `gravity` é um número (o y) ou `{x, y, z}`, e `area` é `{ radius = }` (um círculo deitado), `{ radius = , sphere = true }` (uma esfera) ou `{ w = , h = , d = }` (uma caixa). O emissor tem também `z`, `:burst(n, x, y, z)`, `:set_position(x, y, z)` e `:set_direction(x, y, z)`.

### nn.d3 (3D)

> Começando no 3D? O guia [Comece aqui no 3D](3d.md) mostra os caminhos (sem código no NN Studio ou em Lua), os tutoriais e onde está cada peça.

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

Numa [cena 3D](#cenas-3d), um arquivo que falta ou não abre não para o jogo: um modelo vira um cubo rosa no lugar, um som fica mudo e uma conversa não abre. Cada caso vira um aviso (uma vez só) no console, dizendo o objeto, o componente e o arquivo, por exemplo `objeto "Casa" (Modelo 3D): o arquivo modelos/casa.glb não existe; no lugar dele aparece um cubo rosa`. Já `nn.assets.model` no seu código continua dando erro, para você ver na hora.

| Opção | Descrição |
|---|---|
| `flat` | `true`: cada face com um tom só (o visual low-poly), mesmo que o modelo tenha sombreado suave. |

`modelo:bounds()` devolve a caixa em volta do modelo: `x0, y0, z0, x1, y1, z1`.

O modelo tem `triangles`, `width`, `height` e `depth`. Ele é desenhado com `nn.d3.draw`, como as formas. Luzes e câmeras do arquivo não entram.

#### Animações (personagens com esqueleto)

Um modelo com esqueleto (ossos) e animações, como um personagem do Blender ou do Mixamo, se mexe na GPU, com sombra. Cada objeto que usa o modelo tem o seu **animator**, que diz qual animação toca e em que instante:

```lua
function cena:enter()
  self.heroi = nn.assets.model("modelos/heroi.glb")
  print(table.concat(self.heroi:animations(), ", "))   -- parado, andar, correr, pular, cair, acenar, golpe, atirar
  self.anim = self.heroi:animator()
  self.anim:play("parado")
end

function cena:update(dt)
  if nn.input.down("w") then self.anim:play("andar") else self.anim:play("parado") end
  self.anim:update(dt)                 -- avança o tempo da animação
end

function cena:draw()
  nn.d3.draw(self.heroi, { x = 0, z = 0, animator = self.anim })
end
```

| No animator | O que faz |
|---|---|
| `anim:play(nome, opts)` | Começa a tocar `nome`. Se já estiver tocando, continua de onde está (dá para chamar todo quadro). Opções: `loop` (padrão `true`; `false` toca uma vez e para no fim), `blend` (segundos de transição suave da anterior para a nova, padrão 0,2; 0 = troca na hora), `speed` e `restart` (`true`: começa do zero mesmo se já estiver tocando). Uma animação que não existe dá erro com a lista das que existem. |
| `anim:update(dt)` | Avança o tempo (no `update`). |
| `anim:stop()` | Volta para a pose do modelo, sem animação. |
| `anim:finished()` | Uma animação sem `loop` chegou ao fim? |
| `anim:animations()`, `anim:has(nome)` | Os nomes / se tem essa. |
| `anim:bones()` | Os nomes dos ossos (e das partes) do modelo. |
| `anim:bone(nome)` | Onde está um osso (ou parte) agora, como [matriz](#desenhar) no espaço do modelo: para prender uma espada na mão, `nn.d3.draw(espada, { matrix = matriz_do_heroi * anim:bone("braco_d") })`. |
| `anim.animation`, `anim.time`, `anim.duration` | A animação tocando (ou `nil`), o instante e a duração (segundos). `time` pode ser mudado. |
| `anim.speed`, `anim.looping` | Velocidade (1 = normal; negativo toca ao contrário) e se repete. |

Os modelos dos exemplos e dos modelos de projeto 3D trazem o `heroi.glb` (as animações acima, mais `golpe` e `atirar`; a frente dele é o +Z; as mãos são os ossos `mao_e` e `mao_d`) e o `bicho.glb` (`parado`, `andar` e `golpe`). Numa cena visual, o componente `bone` prende um objeto num osso sem código (veja [Combate](#combate-golpes-tiros-e-dano)).

**Do Blender:** cada *Action* do personagem vira uma animação com o mesmo nome. Exporte em *Arquivo > Exportar > glTF 2.0*, formato *glb*, com *Animação* marcada. Um personagem olhando para −Y no Blender (a vista de frente) olha para +Z no jogo, como os nossos. Use 1 unidade = 1 metro.

**Do Mixamo:** baixe o personagem e as animações em FBX, importe no Blender, dê às *Actions* nomes como `parado` e `andar` (no Mixamo elas vêm como `mixamo.com`) e exporte em glb como acima. Confira a escala depois de importar. Até 128 ossos por modelo; com mais, ele aparece parado.

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
| `matrix` | Uma matriz de `nn.d3.matrix` (no lugar de posição, rotação e escala). |
| `animator` | Um modelo animado na pose de `modelo:animator()` ([animações](#animações-personagens-com-esqueleto)). |
| `glow` | De 0 a 1: a própria cor brilha, mesmo no escuro ([brilho](#lâmpadas-holofotes-e-brilho)). `emissive = false` apaga o brilho que vem no arquivo do modelo. |
| `material` | `"water"` (ou `"agua"`): ondinhas que se mexem, o céu refletido (mais forte olhando de lado) e o brilho do sol. Fica bonito num plano com uma cor um pouco transparente. Padrão `"normal"`. |
| `sway` | Quanto balança com o [vento](#vento-e-halo) (0 = nada; 1 = uma árvore; 1.5 = grama). Embaixo (o y 0 da malha) fica parado e, quanto mais alto, mais balança. |

`nn.d3.text(texto, opts)` escreve no mundo 3D: nomes em cima dos personagens, placas, avisos. Sempre virado para a câmera, em `x`, `y`, `z`, com `size` (0.5 m: a altura das letras), `color`, `font`, `anchor` ("center"), `align`, `background` (uma cor: um retângulo atrás) e `outline` (uma cor: contorno em volta das letras). Com `matrix = nn.d3.matrix({...})`, fica preso como uma placa (nos eixos X e Y da matriz, com a escala; lido de frente pelo +Z e espelhado quando visto de trás); `face_camera = true` usa só a posição e a escala da matriz. Como as partículas, aparece atrás das paredes e com névoa.

```lua
nn.d3.text("Ana", { x = heroi.x, y = heroi.y + 2.2, z = heroi.z, size = 0.3 })
nn.d3.text("Saída", { matrix = nn.d3.matrix({ x = 4, y = 2, ry = 90 }), background = "#F2D49B", color = "#3A2E1F" })
```

`nn.d3.matrix(opts)` guarda posição, rotação e escala (as mesmas opções do `draw`) numa matriz. `pai * filho` junta duas, para coisas presas a outras (uma espada na mão, uma roda no carro): `nn.d3.draw(roda, { matrix = carro * nn.d3.matrix({ x = 1, y = -0.3 }) })`. `m:position()` devolve onde ela está, `m:forward()` para onde aponta (−Z), `m:transform_point(x, y, z)` leva um ponto para o mundo, `m:inverse()` desfaz a matriz e `m:decompose()` devolve posição, rotação (graus) e escala: `x, y, z, rx, ry, rz, sx, sy, sz`.

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
| `shoulder` | Metros para a direita (câmera "no ombro", padrão 0): o personagem fica um pouco à esquerda do meio da tela, bom para mirar. |

`camera:update(dt, x, y, z, opts)` vai no `update`. Ela lê o mouse (só quando ele está preso), o analógico e a rodinha, e põe a câmera do `nn.d3`. As opções são:
- `world`: um mundo de `nn.physics3d`, para não atravessar paredes;
- `ignore`: o corpo do personagem;
- `control = false`: ignora a entrada, para cenas cinematográficas.

Os campos `yaw`, `pitch`, `distance`, `shoulder`, `sensitivity` e `invert_y` podem ser lidos e mudados (por exemplo, numa tela de opções), e `x`, `y`, `z` dizem onde a câmera está. Os métodos são:
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

#### Lâmpadas, holofotes e brilho

Além do sol, até **32 luzes** iluminam o mundo em cada quadro (com mais, valem as mais perto da câmera). Elas não fazem sombra. Peça a luz **a cada quadro**, no `draw`, como os desenhos:

```lua
function cena:draw()
  -- Uma tocha: laranja, até 7 m, tremulando.
  local k = 1 + 0.3 * math.sin(nn.time.elapsed() * 13)
  nn.d3.lamp({ x = 2, y = 1.3, z = 0, color = "#FFA347", intensity = 1.6 * k, range = 7 })
  -- Um holofote apontando para baixo.
  nn.d3.lamp({ x = 0, y = 6, z = -4, direction = { 0, -1, 0 }, angle = 18, intensity = 3, range = 9 })
  -- Um cristal que brilha sozinho (aparece aceso mesmo no escuro).
  nn.d3.draw(self.cristal, { x = -2, glow = 1 })
end
```

| Opção de `nn.d3.lamp` | Descrição |
|---|---|
| `x`, `y`, `z` | Onde a luz está. |
| `color`, `intensity` | A cor e a força (1 = como o sol bem perto da luz). |
| `range` | Até onde a luz chega, em metros (padrão 8): ela some aos poucos até lá. |
| `direction` | Com uma direção `{x, y, z}`, vira um **holofote**: só ilumina dentro do cone. |
| `angle`, `soft` | No holofote: metade da abertura, em graus (padrão 30), e quanto da borda apaga aos poucos (0 a 1, padrão 0.25). |

O `glow` do `nn.d3.draw` (de 0 a 1) faz a própria cor brilhar: a malha aparece acesa no escuro, mas não ilumina em volta (para isso, uma lâmpada). Um modelo `.glb` com emissão no material (`emissiveFactor`, e a extensão `KHR_materials_emissive_strength`) brilha sozinho nessas partes, como a chama da `tocha.glb` e os cristais da `cristal.glb`. Numa cena do NN Studio, use o componente `light3d` e o `glow` das formas e dos modelos; `nn.d3.stats().lamps` diz quantas luzes valeram no último quadro.

Para escurecer o mundo, apague o sol e a luz ambiente (o ambiente pronto **Caverna** faz isso):

```lua
nn.d3.sky(false)
nn.d3.light({ sun_color = "#000000", sky = "#1C1C26", ground = "#0C0B0A", fog = { near = 8, far = 35 } })
```

#### Vento e halo

O **vento** balança o que é desenhado com `sway` (árvores, grama, bandeiras), com rajadas, e é o mesmo para o mundo todo. O **halo** (*bloom*) é uma luz suave em volta do que brilha: o `glow`, as partes do `.glb` que brilham, o fogo e as faíscas, o sol e a lua. Os dois ficam valendo até mudar (não é a cada quadro, como as lâmpadas).

```lua
function cena:enter()
  nn.d3.wind({ strength = 1.2, turn = 90 })  -- uma brisa soprando para +X
  nn.d3.bloom({ strength = 1.5 })            -- halo um pouco mais forte
end

function cena:draw()
  nn.d3.draw(self.arvore, { x = 3, sway = 1 })
  nn.d3.draw(nn.d3.plane(), { y = 0.1, scale = 12, color = "#2E86C1D9", material = "water", shadow = false })
end
```

| Função | Descrição |
|---|---|
| `nn.d3.wind(opts)` | `strength` (0 = parado, 1 = brisa, 3 = ventania), `turn` (para onde sopra, em graus: 0 = para -Z, 90 = para +X) ou `direction` (`{x, y, z}`). Sem `opts`, só lê. Devolve `{ strength =, x =, z = }`. |
| `nn.d3.bloom(v)` | `false` desliga o halo, `true` liga; `{ strength =, enabled = }` muda a força (0 a 4, padrão 1). Devolve se está ligado. Bom para uma opção no menu: sem placa de vídeo, o halo pesa. |
| `emissor:set_wind(x, y, z)` | O vento que leva as partículas de um emissor 3D, em m/s (nas cenas do NN Studio, vem do ambiente). |

Numa cena do NN Studio, o vento e o halo vêm do `environment` (`wind`, `wind_turn` e `bloom`) e o balanço, do `sway` das formas e dos modelos.

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
| `nn.d3.count()` | Quantas malhas foram pedidas no último quadro (com as que ficaram fora da tela). |
| `nn.d3.stats()` | O que o último quadro 3D desenhou: `meshes` (pedidas), `visible` (na frente da câmera), `shadow` (no mapa de sombras), `draw_calls` (chamadas de desenho), `lamps` (as lâmpadas que valeram) e `halo` (se o halo do brilho foi desenhado: sem nada brilhando na tela, ele é pulado). |

O que fica fora da vista da câmera nem vai para a GPU, e o mapa de sombras só recebe o que o sol "vê" perto da câmera: dá para pedir milhares de coisas por quadro e pagar só pelas que aparecem. Cópias da mesma malha (mil árvores iguais) viram uma chamada de desenho só. Para conferir, mostre `nn.d3.stats()` na tela.

Para montar o mundo com o mouse, use uma [cena 3D](#cenas-3d) no NN Studio.

### Seu primeiro jogo 3D

O modelo `3d` cria um jogo de plataforma 3D em terceira pessoa, já jogando: a **Ilha das Estrelas**. São 5 estrelas escondidas, moedas, plataformas que andam, cronômetro, pausa com opções da câmera e recorde salvo. Este guia mostra como transformá-lo no seu jogo.

**1. Crie e jogue.** No NN Studio, escolha **Novo projeto > Jogos 3D > 3D em código**, ou pelo terminal:

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
| `scale` | Multiplica o tamanho (use o mesmo `scale` do desenho). `sx`, `sy`, `sz` multiplicam um eixo só. |
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
- `:nav({ cell = 0.5, radius = 0.35, height = 1, step = 0.45, slope = 50, area = { x1, z1, x2, z2 }, water = { { x1, z1, x2, z2, nível }, ... } })`: a [grade de caminhos](#caminhos-inimigos-que-contornam-paredes) do chão (depois de um `:step`).
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
