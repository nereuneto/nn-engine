# Jogos em Lua

A nn-engine roda jogos escritos inteiramente em Lua (5.4). Não é preciso instalar Lua: ela vem compilada dentro da engine.

## Começando

Para criar um jogo novo, pronto para rodar:

```sh
cargo nn novo ../meu-jogo     # cria a pasta com um jogo pequeno e completo
cargo nn ../meu-jogo          # joga
```

O modelo padrão já vem com menu, partida, pausa, tela de fim e recorde salvo, usando o jeito recomendado de organizar um jogo; é só ir mudando. Opções: `--titulo "Meu Jogo"`, `--modelo visual` (uma fase de plataforma montada no [NN Studio](studio.md), sem código, com [comportamentos](#comportamentos) prontos e um [prefab](#prefabs)) e `--modelo minimo` (só uma cena com um quadrado que anda). A pasta não pode existir com arquivos dentro. Com o `nn` instalado (sem o código da engine), o comando é `nn novo meu-jogo`.

Para ver jogos prontos:

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
  id = "meu-jogo",        -- nome da pasta onde o progresso é salvo (padrão: o título)
  width = 1280,           -- tamanho inicial da janela
  height = 720,
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
}
```

Todas as opções são opcionais. As `vars` viram as [variáveis do jogo](#variáveis-do-jogo-nnvars). Rodando da pasta do projeto (`nn` ou ▶ Jogar no NN Studio), o jogo recarrega ao salvar, a não ser que o `config.lua` diga `hot_reload = false`; o jogo exportado não recarrega. O NN Studio também edita estas opções num formulário (**Editar > Configurações do projeto**), sem mexer nos comentários do arquivo.

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

Cada objeto tem `id` (único na cena), `name`, `x`, `y`, `rotation` (em graus), `scale_x`, `scale_y`, `visible`, `locked` (só para o editor), `layer` (camada maior é desenhada por cima), `components` e `children`. Campos que faltam usam o valor padrão. Os componentes:

| `type` | Campos (padrão) |
|---|---|
| `sprite` | `image`, `frame_w`, `frame_h` (0 = imagem inteira), `frame` (1), `w`, `h` (0 = tamanho da imagem), `color` ("#FFFFFF"), `flip_x`, `flip_y`, `pixel_art` (true) |
| `text` | `text`, `size` (32), `color`, `anchor` ("center", "left", "right"...) |
| `shape` | `shape` ("rect" ou "circle"), `w`, `h` (64), `radius` (32), `color` |
| `body` | `body` ("dynamic", "static" ou "kinematic"), `shape` ("box" ou "circle"), `w`, `h` (32), `radius` (16), `sensor`, `one_way`, `bounce` (0), `gravity_scale` (1), `tag` (padrão: o nome do objeto) |
| `map` | `map` (arquivo .tmj), `scale` (1), `collision` (true) |
| (sprite, text, shape) | `hud` (false): fixo na tela, em vez de andar com o mundo. A posição conta a partir do centro da tela (y para cima). |
| `camera` | `follow` (nome do objeto a seguir), `smooth` (6), `zoom` (1), `limit_to_map` (true) |
| `sound` | `sound`, `autoplay` (true), `loop`, `volume` (1), `channel` ("effects" ou "music") |
| `behavior` | `script` (`"nn:plataforma"` para um [pronto](#comportamentos-prontos) ou o caminho de um `.lua`) e as propriedades dele, soltas: `{ type = "behavior", script = "nn:patrulha", velocidade = 60 }` |
| `particles` | `rate` (20), `lifetime` (1), `speed_min`, `speed_max`, `direction` (90, em graus), `spread` (360), `gravity`, `size_start`, `size_end`, `color_start`, `color_end`, `blend`, `emitting` |

**O script da cena** é um arquivo `.lua` que devolve uma tabela com os callbacks de sempre (`enter`, `fixed_update`, `update`, `draw`, `exit`, `resume`, `reload`), chamados depois do que a engine faz sozinha. Os objetos já existem quando o `enter` roda:

| No script | O que é |
|---|---|
| `self.objects.nome` | O objeto com esse nome (o primeiro, se houver vários). |
| `self:find(nome)`, `self:find_all(nome)` | Um objeto / a lista de todos com o nome. |
| `obj.x`, `obj.y`, `obj.rotation`, `obj.scale_x`, `obj.scale_y`, `obj.visible`, `obj.layer` | Leitura e escrita. Com corpo físico, `x` e `y` acompanham o corpo. |
| `obj.sprite`, `obj.text`, `obj.shape`, `obj.camera` | O componente (primeiro do tipo): mude os campos à vontade (`obj.sprite.frame = 3`, `obj.text.text = "Fim"`, `obj.shape.color = "#FF0000"`). |
| `obj.body` | O corpo físico ([nn.physics](#nnphysics)): `vx`, `vy`, `on_ground`, `:set_position`... |
| `obj.map`, `obj.particles`, `obj.sound` | O mapa carregado, o emissor de partículas e o som. |
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
| `nn:vida` | Pontos de vida; recebe `dano(n)` e `curar(n)`. | `vidas` (3), `variavel` ("vidas": espelha numa variável do jogo), `invencivel` (1 s), `ao_acabar` ("reiniciar", "checkpoint", "cena", "destruir"), `cena`, `piscar` (true) |
| `nn:patrulha` | Anda até uma parede (ou a beirada) e volta. Precisa de corpo. | `velocidade` (120), `direcao` ("direita"), `beirada` (true), `espelhar` (true) |
| `nn:perseguir` | Vai atrás do alvo quando ele chega perto. | `alvo` ("" = jogador), `velocidade` (150), `visao` (320), `voar` (false), `espelhar` (true) |
| `nn:perigo` | Machuca quem encosta. | `quem` (""), `acao` ("dano", "checkpoint", "reiniciar"), `dano` (1), `pisar` (false: pular em cima destrói), `some` (false), `empurrao` (520) |
| `nn:coletavel` | Some ao ser tocado e soma numa variável. | `quem` (""), `variavel` ("moedas"), `valor` (1), `som`, `brilho` (true), `cor_brilho` |
| `nn:checkpoint` | Ponto de reinício. | `quem` ("") |
| `nn:ir_para_cena` | Leva para outra cena. | `cena`, `quem` (""), `tecla` (false: precisa apertar para cima / Enter), `escurecer` (0.5 s) |
| `nn:movimento` | Gira, flutua ou vai e volta (com corpo cinemático, carrega quem está em cima). | `tipo` ("ir_e_voltar", "flutuar", "girar"), `distancia_x` (200), `distancia_y` (0), `altura` (12), `duracao` (2), `velocidade` (90) |
| `nn:animacao` | Troca os quadros conforme o estado. Precisa de imagem. | `parado` ("1"), `andando`, `pulando`, `caindo` (listas como `"1-4"` ou `"5,6"`; vazio usa o anterior), `fps` (10), `espelhar` (true) |
| `nn:mostrar_valor` | Escreve variáveis num texto. Precisa de texto. | `texto` ("Moedas: {moedas}") |
| `nn:atirar` | Cria um prefab andando. | `prefab`, `tecla` ("x"; vazio = sozinho), `intervalo` (0.3), `velocidade` (700), `direcao` ("frente", "jogador", "direita", "cima"), `som`, `distancia` (24) |
| `nn:gerador` | Cria um prefab de tempos em tempos. | `prefab`, `intervalo` (2), `maximo` (10), `espalhar_x`, `espalhar_y` (0), `comecar_ja` (false) |
| `nn:temporario` | Some depois de um tempo ou fora da tela. | `tempo` (3), `fora_da_tela` (true) |

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

Outros métodos e campos: `width`, `height` (em tiles), `tile_width`, `tile_height`, `world_width`, `world_height`, `layers`, `properties`, `:tile(col, lin, camada)`, `:set_tile(col, lin, tile, camada)` (a colisão acompanha: ótimo para blocos quebráveis), `:set_solid(col, lin, "solid" | "one_way" | nil)`, `:solid(col, lin)`, `:cell_at(x, y)` e `:cell_center(col, lin)`. Colunas e linhas começam em 1, com a linha 1 no topo, como no editor.

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

O emissor tem `x`, `y`, `emitting` e `rate` (leitura e escrita), `count` e `finished`, e os métodos `:burst(n, x, y)`, `:set_position(x, y)`, `:update(dt)`, `:draw()`, `:draw_screen()` e `:clear()`.

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

### Outros

`print(...)` escreve no terminal (log da engine). `nn.version` é a versão da engine. `nn.vars` e `nn.reset_vars()`: veja [variáveis do jogo](#variáveis-do-jogo-nnvars). Variáveis globais são compartilhadas entre as cenas e sobrevivem à recarga.
