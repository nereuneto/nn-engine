# O que mudou

As versões seguem o [versionamento semântico](https://semver.org/lang/pt-BR/). Antes da 1.0, uma versão nova pode mudar coisas de um jeito que pede ajustes nos projetos; quando isso acontecer, está escrito aqui.

## Não lançado

### Estudo da versão web (Sprint 67)

- **A engine roda no navegador:** o desenho 2D e 3D, o teclado, o mouse e o som, em WebGL2 e WebGPU, e o mesmo Lua 5.4 que usamos hoje. A recomendação é manter a web na v0.13, com cerca de 4 sprints de trabalho. O relatório completo está em [`docs/estudo-web.md`](docs/estudo-web.md), e os experimentos no ramo `estudo-web`.
- **Correções que preparam a web, sem mudança no desktop** (as fotos dos jogos saíram iguais byte a byte):
  - onde a placa não tem os limites comuns, a engine pede os do WebGL2;
  - os relógios do nn-render e do nn-app funcionam no navegador;
  - o 3D funciona onde a placa não mistura cada alvo de um jeito (WebGL2);
  - três shaders foram ajustados ao WGSL mais rígido dos navegadores: a sombra com ossos, o céu estrelado e a água.

### Pré-carregar e a tela de carregamento (Sprint 66)

- **Ao começar, a cena carrega antes o que vai usar.** Antes, uma imagem ou um som carregava no primeiro uso, e o jogo dava uma travada no meio: o som do item na primeira vez que tocava, a fonte do placar no primeiro desenho.
  - Numa cena visual, a lista sai sozinha dos componentes: imagens, sons, modelos, fontes e mapas, também os dos prefabs e os sons das propriedades dos comportamentos.
  - Numa cena em Lua, ou no script de uma cena visual, há o campo `preload`, com os caminhos (`{ "caminho", pixel_art = true }` para pixel art).
- **Tela de carregamento**, com a opção `loading` na troca de cena: `loading = true` mostra a tela padrão, com uma barra; `loading = "cenas/carregando.lua"` usa uma cena sua, que lê o andamento em **`nn.scene.progress()`**. A cena nova carrega um pouco a cada quadro e começa quando terminar.
- **F3 e `nn.debug.stats()`** contam os arquivos carregados e mostram o último quadro que carregou alguma coisa: um quadro que carrega no meio do jogo é uma travada à vista.
- **Testes:** trocar para uma cena com 21 imagens e 5 sons (um deles tocado por um comportamento, e uma imagem usada só no `draw`) carrega tudo no quadro da troca e nada no primeiro quadro da cena nova; a tela de carregamento mostra o andamento subindo até o fim.
- **Para quem usa as crates:** `FrameTimes::loads` e `load`, e `Perf::add_load`, `total_loads` e `last_load`.

### Memória: trocar de cena libera o que não se usa mais (Sprint 65)

- **Ao entrar numa cena, o que nenhuma cena usa mais sai da memória:**
  - as imagens (inclusive as dos tilesets e as das partículas);
  - os sons;
  - os modelos 3D, com as malhas e as texturas deles;
  - as malhas do `nn.d3.mesh` e dos terrenos.

  Antes, tudo o que o jogo carregava ficava até ele fechar: um jogo com muitas fases ia enchendo a memória da placa de vídeo.
- **O que fica:**
  - o que outra cena ainda aberta usa (o jogo embaixo de uma pausa);
  - o que um módulo do `require` guarda;
  - o que for guardado com **`nn.assets.keep(asset)`**, para o que toda cena usa, como a música.
- **Pedir de novo** algo que saiu carrega outra vez, sem erro.
- **`nn.d3.mesh`** com os mesmos pontos e triângulos reaproveita a malha já criada, em vez de ocupar mais memória a cada chamada.
- **O atlas de letras tem um limite** de 4 páginas (16 MB). Cheio, a página usada há mais tempo é reaproveitada (nunca uma desenhada no quadro atual). Antes, um jogo com muitos tamanhos de letra crescia sem parar.
- **Testes:**
  - trocar 200 vezes entre duas cenas com imagens, sons, malhas e modelos diferentes deixa a memória igual à da primeira volta, só com o que a cena atual usa;
  - uma pausa não descarrega o jogo de baixo;
  - o atlas com 80 tamanhos de letra fica em 4 páginas.
- **Para quem usa as crates:**
  - `Context::texture_handle`, `sound_handle` e `model_handle` devolvem um `AssetHandle`; sem nenhum vivo, `Context::unload_unused` descarrega o asset. Pedido com `load_texture` e afins, o asset nunca é descarregado (como antes);
  - `keep_texture`, `keep_sound` e `keep_model`;
  - `Renderer::free_texture`, `free_mesh`, `free_model` e `mesh_count`;
  - `Audio::free` e `sound_count`.

### Comportamentos com erro e o Lua sem `io` (Sprint 64)

- **Um comportamento com erro não para mais a cena.** Antes, o primeiro erro num comportamento parava tudo. Agora só aquele comportamento é desligado, naquele objeto; o resto da cena, os outros comportamentos e o script da cena seguem. Vale para:
  - os erros em `start`, `update`, `contact` e nas mensagens (`obj:send`);
  - um script de comportamento que nem carrega: o objeto fica sem ele.
- **O erro continua aparecendo:**
  - no jogo em desenvolvimento, numa faixa embaixo, por cima do jogo, com o objeto, o arquivo, a linha e o caminho até o erro;
  - no console do NN Studio;
  - no `rodar_jogo` e nos testes de jogo, como erro;
  - no jogo exportado, no arquivo de erro, sem interromper quem joga.
- **O Lua dos jogos não mexe mais no computador de quem joga:**
  - sem `io`;
  - do `os`, só `os.time`, `os.clock`, `os.date` e `os.difftime`;
  - sem `dofile`, `loadfile` e `package.loadlib`;
  - o `load` só aceita texto (um trecho compilado podia quebrar a memória do Lua).

  Quem usar o que saiu recebe um erro dizendo o que usar no lugar: `nn.assets.read` para ler um arquivo do jogo, `nn.save` para guardar, `nn.quit()` para fechar, `require` para outro script. O `config.lua` e o Lua que o NN Studio e o servidor MCP usam para ler os arquivos do projeto também rodam assim.
- **Ajuste nos projetos:** um script que usava `io` ou `os.execute` precisa trocar pelo equivalente acima.

### Erros no jogo exportado e laço sem fim (Sprint 63)

- **Quando o jogo exportado dá erro, quem joga vê uma tela simples:** "O jogo encontrou um problema", com o caminho do arquivo para mandar a quem fez o jogo, e Esc fecha. Antes aparecia a tela de desenvolvedor, com a mensagem do script.
- **O arquivo `<nome>-erro.txt`** fica ao lado do executável (ou na pasta temporária, se ali não der para escrever). Ele traz:
  - o nome do jogo e a versão da engine;
  - a mensagem com o arquivo e a linha;
  - onde está o log completo.

  Antes ele só existia quando o jogo nem abria. Agora vale também para:
  - erro de script e laço sem fim: a tela aparece no próprio jogo;
  - erro interno da engine: o jogo fecha e a tela abre numa janela nova.
- **Laço sem fim também com janela.** Um quadro em que o Lua passa de 5 segundos sem voltar não trava mais o jogo:
  - se o laço chama alguma função `nn.*`, ela dá erro ali mesmo, e a tela de erro mostra o arquivo e a linha (por exemplo, `while not nn.input.pressed("space") do end`, que nunca termina porque a tecla só é lida entre quadros);
  - um laço só de Lua (`while true do end`) não tem como ser parado por fora: depois de mais 3 segundos, o jogo fecha dizendo a cena.

  O vigia é uma linha de execução separada e não custa nada por quadro. Medimos a alternativa, um gancho do Lua que dá arquivo e linha em todo caso, e ela deixava o Lua uns 40% mais lento.
- **No jogo em desenvolvimento** (`nn`, ▶ Jogar) a tela de erro segue a mesma, com a mensagem inteira.
- **Testes de ponta a ponta com o executável exportado de verdade:** erro de script, laço que chama a engine, laço só de Lua e erro interno geram o arquivo e a tela. Eles rodam sem janela, com `NN_JOGO_SEM_JANELA`, e por isso rodam também no CI.
- **`rodar_jogo`** avisa que ali o Lua roda mais devagar que no jogo de verdade, por causa da checagem de laço sem fim desse modo.
- **Para quem usa as crates:** `Project::report` e `nn_script::report` (`ErrorReport`, a tela de erro), `nn_script::run_headless` e `nn_script::watchdog`.

### Cenas visuais mais leves (Sprint 62)

- **As cenas visuais gastam bem menos tempo por quadro**, com o mesmo resultado na tela. Com 2.000 objetos, medido sem janela:

  | Cena | Antes | Agora |
  |---|---|---|
  | 2.000 imagens paradas | 16,4 ms | 3,6 ms |
  | 2.000 formas paradas | 7,2 ms | 3,0 ms |
  | 2.000 formas com **Movimento automático** | 10,1 ms | 5,0 ms |

- **Por quê:**
  - cada imagem pedia ao Rust o tamanho dela e o da janela, e montava uma tabela de opções nova a cada quadro: agora é uma chamada só, sem tabela;
  - a posição na tela só é calculada para o que é HUD;
  - os laços que passam por todos os objetos (comportamentos, animações, partículas, desenho) não chamam mais uma função por item;
  - os corpos da física são lidos todos de uma vez depois de cada passo, não campo a campo.
- **O Lua quase não aloca memória por quadro:** numa cena de teste com 500 objetos, de uns 200 KB para menos de 1 KB, o que deixa o coletor de lixo bem mais quieto. As chamadas do Lua ao Rust caíram de umas 1.600 para umas 500 por quadro (uma por coisa desenhada).
- **Texto e painéis:** o layout de um texto que não mudou é reaproveitado de um quadro para o outro, e os painéis e as partículas não montam mais listas a cada quadro. 500 textos e 100 painéis: de 1,8 ms para 0,9 ms.
- **Testes:** dois testes novos falham se a cena visual voltar a chamar o Rust mais de uma vez por coisa desenhada ou a alocar memória a cada quadro (números que não dependem da máquina, ao contrário do tempo).

### Limite de quadros e o medidor F3 (Sprint 61)

- **Limite de quadros** (`fps_max` no `config.lua`, ou **Limite de quadros** nas configurações do projeto no NN Studio):
  - sem ele, o jogo segue o monitor com VSync e fica em até 240 quadros por segundo sem VSync; antes, sem VSync, o jogo rodava sem parar e ocupava um núcleo inteiro do processador;
  - um número limita (60, 120...), e `0` tira o limite;
  - o limite segue uma hora marcada para cada quadro, então um atraso do sistema num quadro é compensado no seguinte, e a média fica no número pedido.
- **Janela minimizada:** o jogo para de simular e de desenhar (a música continua) e, ao voltar, segue de onde estava, sem um salto no tempo.
- **F3: a tela de desempenho.** No jogo aberto da pasta do projeto, F3 mostra no canto:
  - os quadros por segundo e o pior quadro;
  - o tempo dos passos fixos, da física, do `update` e do desenho;
  - os sprites e as chamadas de desenho (e as malhas, no 3D);
  - a memória das texturas e do Lua, e os objetos da cena.

  No jogo exportado, F3 não faz nada.
- **`nn.debug.stats()`** devolve os mesmos números para o Lua, e `nn.debug.overlay(true)` abre a tela pelo código.
- **`rodar_jogo`** (servidor MCP) diz quanto o jogo gastou por quadro, em média, e qual foi o quadro mais pesado.
- **Para quem usa as crates:** `Context::perf` (o medidor, com `FrameTimes` e `add_physics`), `AppConfig::fps_max` e `dev_tools`, e `Renderer::texture_bytes` e `texture_count`.

### Física 2D com grade espacial (Sprint 60)

- **A física 2D ficou muito mais rápida com muitos corpos.** Antes, cada corpo que se movia conferia todos os outros, em cada subpasso; agora uma grade espacial deixa conferir só quem está perto. Vale para o movimento, para os corpos que se empurram, para os sensores e para os raios e as consultas (`raycast`, `query_point`, `query_aabb`, `overlapping`).
  - 500 corpos caindo: de uns 33 ms para uns 5 ms por quadro no jogo, contando o Lua.
  - 1.500 corpos numa cena cheia: de 12,8 ms para 2,6 ms por passo de física.
- **O resultado é o mesmo de antes:** um teste roda cenas cheias com e sem a grade e confere que as posições, os eventos e as consultas saem iguais.
- **O passo ficou previsível:** as plataformas móveis eram percorridas numa ordem que mudava de uma execução para outra, e a mesma cena podia dar posições um pouco diferentes. Agora elas vão sempre na mesma ordem.
- **Para quem usa as crates:** `World::stats()` diz quantos corpos e quantas conferências o último passo teve.

## 0.8.0

A versão da **arte, da modelagem e da animação pelo agente**. O servidor MCP passa de 30 para **48 ferramentas**, e o agente de IA faz sozinho o que antes pedia um artista:
- a **pixel art** e as animações dos sprites;
- os **mapas de tiles** com o tileset dele;
- as **partículas**;
- os **efeitos sonoros** (sintetizados, como o sfxr; também no NN Studio, em **➕ Novo > 🔊 Som**);
- os **diálogos**;
- os **modelos 3D**, com **esqueleto e animações**.

Cada passo volta como imagem (os sprites ampliados, o mapa com a colisão, o modelo em quatro vistas, a tira de uma animação), porque o agente vê, mas não ouve nem joga com as mãos. Na revisão, dois agentes fizeram cada um um jogo do zero só pelo servidor, com toda a arte e os sons feitos por eles, e com testes:
- **"Musgo"**: plataforma 2D em pixel art;
- **"A Torre do Vale"**: aventura 3D com herói, lobo e sábio modelados e animados.

Como usar: [O servidor MCP](docs/mcp.md).

### Para quem vem da 0.7

- **Os projetos da 0.7 abrem e rodam.** Conferido com os dez modelos de projeto da 0.7.0 publicada e com os dois jogos feitos por agentes na revisão da 0.7:
  - abertos e gravados no editor novo, os arquivos dos modelos saem iguais, byte a byte (os campos novos, como o deslocamento da colisão e as partículas de pixel art, só vão para o arquivo quando mudam);
  - rodam no `nn` novo sem erro nem aviso, e os testes deles passam;
  - o servidor MCP vê, confere e roda todos.

  Os projetos novos da 0.8 também rodam no `nn` da 0.7.
- **As preferências da 0.7 abrem na 0.8** como estão.
- **Arquivos novos, só se você usar:**
  - `.sfx`, ao lado do `.wav` de um som sintetizado;
  - `.dialogo`, das conversas;
  - os ossos e as animações dentro do `.modelo`.

  O jogo exportado leva só o `.wav` e o `.glb`, como antes.
- **O que muda sem você mexer:**
  - **Um botão com foco** (teclado, controle ou mouse) ganha uma borda na cor de destaque do tema, mesmo sem borda própria. Se vários pedem o foco ao começar, fica o primeiro; antes, ficava o último.
  - **O Mostrar valor só escreve quando o valor muda.** Um texto que uma regra põe no mesmo objeto fica até a variável mudar; antes, sumia no quadro seguinte. Ele também já mostra no primeiro quadro o que outro comportamento escreveu ao começar (uma contagem).
  - **A âncora do texto aceita `top_left`**, como a do objeto (e a do objeto aceita `topleft`).
  - **Esqueleto pronto:** as juntas ficam em cima das peças com o nome do osso. Num modelo que já tem esqueleto nada muda; só ao pôr de novo.
  - **Animações prontas do esqueleto simples:** vão para o osso do modelo quando ele tem outro nome ("Dobradiça").
  - **Duplicar um objeto** não repete um nome que já existe na cena ("Árvore 2" já existe: a cópia é "Árvore 3").
  - **Um pulo guardado no `nn:plataforma`** (apertado logo antes de cair) com o botão já solto fica baixo, como um toque.
- **Para quem usa as crates:**
  - a API do Lua e os formatos dos arquivos continuam os mesmos, com campos novos opcionais;
  - os comportamentos podem ter `after_start`, chamado depois que todos rodaram o `start`;
  - o que saiu do NN Studio para o `nn-projeto` está nas seções abaixo.

### Revisão (Sprint 59)

- **A prova: dois agentes fazendo jogos do zero, com toda a arte, só pelo servidor.** Sem acesso ao código, os dois cumpriram os sete itens pedidos, com testes que passam e o `conferir_projeto` limpo.
  - **"Musgo"** (2D, 320x180), em cerca de 100 chamadas:
    - herói com 8 quadros e lesmas animadas;
    - um tileset de 7 tiles e um mapa de 60x12 em duas camadas;
    - poeira no pulo, faíscas e confete;
    - 6 sons;
    - uma placa que conversa, com uma escolha;
    - vitória, derrota e 9 testes.
  - **"A Torre do Vale"** (3D), em cerca de 145 chamadas:
    - herói, lobo (com uma mandíbula), sábio, torre, ponte, casa e baú modelados;
    - um golpe e uma mordida com quadros-chave próprios;
    - um baú que abre a tampa;
    - 7 sons, fumaça e brilho;
    - um diálogo que dá a chave;
    - vitória, derrota e 8 testes.

  O que atrapalhou os dois virou os acertos abaixo.
- **Tela pequena:** `mudar_config` avisa, ao mudar o tamanho da tela, os números das cenas que ficaram no padrão de 720 de altura, como a gravidade, a velocidade, o pulo e os botões, com o número na escala nova.
  - O **Perigo** ganhou **Pulo ao pisar** (`quique`); antes ele era fixo e jogava o herói para fora de uma tela pequena.
- **Campos de escolha:** a ação de um botão que não existe é recusada, e `listar_componentes` mostra as opções de cada campo de escolha.
- **Um texto preso num canto da tela** (pelo servidor) fica alinhado a esse canto; antes, o meio dele ficava no ponto e um texto comprido saía da tela.
- **Partículas contínuas** que começam desligadas: `on_start: false` num contínuo vira `emitting: false`, com o aviso de como ligar.
- **Vida:** o servidor avisa quando o `config.lua` já começa a variável com outro número (que vale primeiro).
- **Modelos:**
  - `recuar` com `no_chao`, para portas;
  - os kits dizem o tamanho de cada peça;
  - a descrição de `lados` diz que o tamanho vai de face a face;
  - `ver_modelo` no `.glb` de um `.modelo` diz quais nós são os ossos.
- **Jogador 3D:** **Inclinação da câmera** (`inclinacao`), para ver por cima do personagem o que está à frente.
- **`rodar_jogo`:**
  - avisa quando a cena reiniciou, e aí mostra o estado no fim em vez de comparar com o começo;
  - a descrição explica `self:find_all` para objetos com o mesmo nome.
- **Inventário:** um item sem ícone (um item 3D) mostra o nome em vez de só a primeira letra.
- **Mensagens:**
  - `mudar_objeto` mostra o nome novo;
  - o `esqueleto` diz quantas peças foram ligadas por `ligar`;
  - a dica do `escrever_dialogo` mostra o componente inteiro;
  - as `cores` de cada desenho do `desenhar` estão explicadas.

### Pixel art pelo agente (Sprint 55)

- **O agente de IA desenha a arte do jogo.** Cinco ferramentas novas no servidor MCP, com as mesmas operações do editor de pixel art do NN Studio. Cada passo volta como **imagem ampliada**, com pixels grandes, xadrez no transparente e uma linha entre os quadros. Como usar: [Pixel art](docs/mcp.md#pixel-art).
  - **`criar_imagem`:** uma imagem vazia (ou com fundo), com quadros lado a lado para animar.
  - **`desenhar`:** várias operações numa chamada, num quadro.
    - O jeito mais prático é o **desenho como texto**, um caractere por pixel, com a cor de cada caractere.
    - Também: pontos, linha, retângulo, elipse, balde, apagar, trocar cor, espelhar, girar, **simetria** (uma metade copiada espelhada na outra), deslocar, **contorno** e copiar outro quadro.
    - As cores vão em `#rrggbb`, `transparente` ou pelo número numa paleta: a do projeto (`paleta.hex`), Sweetie 16, PICO-8 ou Endesga 32.
    - Uma chamada mexe em vários quadros (cada operação com o seu `quadro`), e as `cores` da chamada valem para todos os desenhos dela.
  - **`ver_imagem`:** a imagem, o tamanho, os quadros, as animações e as cores usadas. Com `texto`, o quadro como texto, para conferir pixel a pixel (nos mesmos caracteres do desenho). Com `repetir`, o tile lado a lado, para ver as emendas.
  - **`quadros`:** põe, tira e troca quadros (as animações acompanham) e divide uma imagem em quadros.
  - **`animar`:** as animações com nome do `.anim` (quadros, fps, repetir, uma vez ou ida e volta), com a tira dos quadros na ordem em que tocam.
- **Pixel art no jogo:**
  - num jogo de tela pequena (320x180 com pixels inteiros), as capturas do `rodar_jogo` e dos testes vêm ampliadas, e `ampliar` escolhe quanto;
  - um sprite com `fill = "repeat"` e o `w` da área toda ganha um aviso, e a ajuda do sprite explica que `w` e `h` são o tamanho de cada cópia;
  - `criar_imagem` sugere a tela pequena quando o jogo é de 1280x720.
- **NN Studio:** o editor de pixel art ganhou **Contorno** (uma borda com a cor principal em volta do desenho) e **Trocar cor** (a secundária pela principal no quadro todo).
- **Para quem usa as crates:** as operações de pixel art e o `.anim` saíram do NN Studio para o `nn-projeto` (`pixel`, `anim`), usado pelo editor e pelo servidor. O `pixel` ganhou `flip`, `rotate`, `shift`, `replace_color`, `outline`, `draw_text_art`, `as_text_art` e `preview`.

### Mapas, partículas, sons e diálogos pelo agente (Sprint 56)

- **Mapas de tiles pelo agente.** Quatro ferramentas novas no servidor MCP, com o mesmo `.tmj` e `.tsj` do editor de mapas do NN Studio. Como usar: [Mapas, partículas, sons e diálogos](docs/mcp.md#mapas-partículas-sons-e-diálogos).
  - **`criar_mapa`:** o mapa a partir de uma imagem de tiles (o tileset é criado ao lado) e os tiles numerados com a grade.
  - **`pintar_mapa`:** várias operações numa camada: tile em retângulo, linha, pontos ou balde, apagar, **desenho como texto** (um caractere por tile) e **terreno automático** (as bordas e os cantos se ajustam sozinhos). Também cria camadas, liga a colisão de uma camada e aumenta o mapa. Volta o mapa desenhado.
  - **`marcar_tiles`:** os tiles sólidos, as plataformas de mão única, o tipo de cada um (`lava`, `agua`) e as 16 peças de um terreno.
  - **`ver_mapa`:** o mapa desenhado, as camadas, os tilesets e os terrenos; uma camada como texto.
- **`particulas`:** um efeito pronto num objeto (cria o objeto se precisar), ajustes e uma **prévia do efeito sozinho** em quatro momentos, recortada em volta dele.
- **Efeitos sonoros sintetizados**, como o sfxr: moeda, pulo, tiro, laser, explosão, power-up, dano, clique e blip, com variações e ajustes (onda, nota, deslize, vibrato, arpejo, volume ao longo do tempo, filtros).
  - No servidor, **`criar_som`** grava o `.wav` e volta a forma da onda e a nota ao longo do tempo, já que o agente não ouve.
  - No NN Studio, **Arquivos > ➕ Novo > 🔊 Som** (e **Arquivo > Novo som**): escolha o tipo, aperte **🎲 Variar** até gostar e ajuste ouvindo. Os números ficam num `.sfx` ao lado do `.wav`, e o som abre de novo em **🎛 Ajustar efeito**.
- **Diálogos pelo agente:** `escrever_dialogo` grava uma conversa inteira em JSON (falas com nomes, escolhas, variáveis, itens e personagens) e conta de volta os caminhos, avisando de falas que nunca aparecem; `ver_dialogo` devolve no mesmo formato, para mudar.
- **Partículas de pixel art:** o componente Partículas ganhou **Pixel art** (`pixel`): quadradinhos de tamanho inteiro, sem girar. As faíscas de pegar uma moeda (`nn:coletavel`) seguem o tamanho da tela e, numa tela pequena, viram quadradinhos.
- **Colisão deslocada:** o corpo 2D ganhou **Deslocar x** e **Deslocar y** (`ox`, `oy`), para a caixa de colisão ficar nos pés de um sprite com espaço vazio. O NN Studio desenha a caixa no lugar certo.
- **`animar`** também volta a **casca de cebola**: cada quadro sobre a sombra do anterior, para ver o movimento.
- **Jogos de tela pequena (pixel art):**
  - a **caixa de fala** dos diálogos, as escolhas e o aviso **"E falar"** seguem o tamanho da tela (em 320x180 a caixa cobria dois terços dela e a primeira escolha saía por cima); o retrato é desenhado como pixel art e o aviso não sai da tela;
  - o servidor MCP avisa quando velocidades, pulos e letras ficaram no padrão, que é pensado para uma tela de 720 de altura, e sugere os números na escala da tela; o corpo que vem junto com um comportamento tem o tamanho do quadro do sprite.
- **`nn:plataforma`:** **Som do pulo** e a condição **Pular** nos eventos (para um som ou poeira a cada pulo). Um pulo guardado (apertado logo antes de cair) com o botão já solto fica baixo, como um toque.
- **`nn:animacao`** com os campos no padrão usa sozinho as animações da imagem chamadas `parado`, `andar`, `pular` e `cair` (e a que o sprite já tocava).
- **No servidor MCP:** `rodar_jogo` lista os **sons que tocaram**, os tempos de cada regra e o inventário no que mudou; `conferir_projeto` também confere mapas, tilesets, diálogos e `.sfx`; os mapas vêm ampliados com a **colisão por cima** (a borda do que bloqueia em vermelho, as plataformas em amarelo) e o tileset com o número de cada tile.
- **Para quem usa as crates:** os mapas, tilesets, efeitos de partículas e diálogos saíram do NN Studio para o `nn-projeto` (`map`, `tileset`, `particles`, `dialogue`), e o gerador de sons é o `nn_projeto::sfx`. O `ParticleConfig` da `nn-render` ganhou `pixel`.

### Modelagem 3D pelo agente (Sprint 57)

- **O agente de IA faz os modelos 3D do jogo.** Três ferramentas novas no servidor MCP, com o mesmo `.modelo` do modelador do NN Studio. Cada passo volta desenhado em **quatro vistas** (frente, lado, de cima e na diagonal), com uma grade de 1 m, o chão e a origem, sem precisar de placa de vídeo. Como usar: [Modelos 3D](docs/mcp.md#modelos-3d).
  - **`criar_modelo`:** um modelo vazio, de um modelo pronto ou da cópia de outro. Grava junto o `.glb` que o jogo usa.
  - **`modelar`:** várias operações numa chamada:
    - peças de uma forma (cubo, cilindro, cone, pirâmide, esfera, plano, rampa) com tamanho, posição, giro e cor, e as peças dos kits;
    - mover, girar, escalar ou dar um tamanho;
    - pintar a peça ou só as faces de um lado;
    - extrudar (com a ponta afinando), recuar (portas e janelas) e afinar;
    - simetria, copiar espelhado, juntar, apagar e renomear.
  - **`ver_modelo`:** um `.modelo` ou **qualquer `.glb`** (Blender, Mixamo):
    - o tamanho em metros e onde ficam a base e a origem, com um aviso quando o modelo está em centímetros, afunda ou flutua;
    - as peças, as cores, o brilho, os ossos e as animações;
    - o desenho em quatro vistas ou numa só, também num quadro de uma animação.
- **Brilho por peça:** uma peça do modelo pode acender com a própria cor (no modelador, **Brilho**; no `modelar`, `brilho`), e o `.glb` leva isso.
- **Peças de cenário prontas** (no modelador e no `criar_modelo`): **muro**, **borda**, **escada**, **ponte** e **placa**, de 2 m para emendar.
- **A cor de um modelo tinge o brilho dele:** o brilho que vem do arquivo (*emissive*) também pega a `color` do objeto.
- **Câmera girando em volta** (`nn:camera_orbita`): numa Câmera 3D, dá voltas devagar em volta de um objeto ou do meio da cena, olhando para ele, para telas de título e de menu. No NN Studio: **➕ Objeto > Câmera de menu (girando)**.
- **Contagem regressiva** (`nn:contagem`, 2D e 3D): conta os segundos para trás numa variável do jogo (para o HUD mostrar) e, no zero, avisa a condição nova **Contagem acabar**, reinicia a cena ou vai para outra. Recebe `comecar`, `parar`, `reiniciar` e `somar`.
- **A prova:** um agente fez um jogo 3D só pelo servidor, com os modelos dele (um cavaleiro, uma gema que brilha, uma torre e um portal), um menu com a câmera girando e uma contagem regressiva. Saíram estes acertos:
  - `modelar`: espelhar também em Z, cilindros e cones com uma face para a frente e no tamanho exato, pirâmide com `lados`, `abertura` no recuar, lados pelos eixos e a direita e a esquerda explicadas;
  - `ver_modelo`: as costas, um traço para a frente, `metros` para comparar e de onde a onde cada peça vai;
  - `ver_corpos` no 3D desenha também o cenário de malha exata (o chão, as paredes, a escada);
  - os campos de escolha dos componentes (a forma de um corpo, o tipo de uma luz) são conferidos, também no `conferir_projeto`;
  - a contagem regressiva segue a variável (um bônus de tempo funciona), e as cópias com **Girar e flutuar** não sobem e descem todas juntas.
- **Para quem usa as crates:** o modelo do modelador, as peças e os modelos prontos saíram do NN Studio para o `nn-projeto` (`model`, `model_kits`). O desenho sem placa de vídeo é o `nn_projeto::model_view`.

### Ossos e animações pelo agente (Sprint 58)

- **O agente de IA anima os modelos 3D.** Duas ferramentas novas no servidor MCP, com o mesmo esqueleto e animador do modelador do NN Studio. Como usar: [Ossos e animações](docs/mcp.md#ossos-e-animações).
  - **`esqueleto`:** um esqueleto pronto (humanoide, quadrúpede ou simples) do tamanho do modelo, com cada peça ligada ao osso do nome dela ou ao mais perto; ossos próprios (a junta e a ponta); ligar uma peça a um osso. Volta as vistas com os ossos desenhados.
  - **`animar_modelo`:** as animações prontas, com força e velocidade, ou quadros-chave próprios por osso (giro e deslocamento). Volta uma **tira** com o modelo em vários momentos da animação, no mesmo enquadramento.
  - **`ver_modelo`** desenha os ossos por cima num modelo com esqueleto.
- **Esqueleto pronto mais certo:** as juntas das pernas, dos braços, das patas, da cabeça e do rabo ficam em cima das peças com esses nomes (antes, num quadrúpede feito de cubos, as patas giravam fora do lugar). Vale também para o NN Studio.
- **Abrir porta:** uma animação pronta nova do esqueleto simples. As do esqueleto simples agora vão para o osso do modelo mesmo quando ele tem outro nome.
- **A prova:** um agente fez **"A Chave do Lobo"** em cerca de 80 chamadas, com o herói, o lobo e o baú modelados e animados por ele (o "parado" respirando e o pulo de braços para cima com quadros-chave próprios). Do que ele achou, isto foi acertado:
  - **Abrir num modelo com vários ossos:** a pronta girava o osso de baixo, e o baú inteiro tombava. Agora ela vai para o osso do nome ("Tampa", "Porta") ou para o último osso, e `osso` escolhe outro. A resposta diz que ossos a animação move.
  - **Âncora:** a do objeto é `top_left` e a do texto era `topleft`, e uma valia onde a outra quebrava o jogo. Agora as duas valem nos dois lugares, e um valor que não existe na âncora do texto é recusado ao criar o objeto (antes só quebrava ao jogar).
  - **Mostrar valor** apagava, no quadro seguinte, o texto que uma regra punha no mesmo objeto. Agora ele só escreve quando o valor muda.
  - **`rodar_jogo`** diz que animações 3D começaram a tocar em cada modelo, e quando.
  - **Tiras maiores** com menos quadros, e `"ossos": false` para ver só o modelo.
  - **Descrições:** a câmera "atrás" do Jogador 3D (gira com o mouse, não quando o jogador vira); andar e correr no teclado; a distância do "estiver perto de" (das posições, não das bordas); o limite de 5 imagens do `rodar_jogo`.
  - **`buscar_documentacao`** com `secao` também acha os parágrafos com título em negrito ("Preso na tela").
- **Para quem usa as crates:** o esqueleto e as animações prontas saíram do NN Studio para o `nn-projeto` (`model_rig`), e o `model_view` ganhou `render_bones` e `render_strip`. `ModelData::bone_lines` dá a junta e a ponta de cada osso num instante da animação.

## 0.7.0

A versão do **desenvolvimento guiado por IA**. O **servidor MCP** (`nn mcp`) liga um agente de IA (Claude Code, Claude Desktop, Cursor...) ao projeto, com **30 ferramentas em português**. O agente monta cenas, objetos, prefabs e eventos, escreve Lua e confere tudo antes de gravar. Ele também **joga o jogo sem janela**, com teclas e cliques simulados, e vê os erros, o que mudou e a tela. Ele escreve e roda os **testes de jogo** (`testes/*.lua`, também com `nn testar` e no NN Studio) e acompanha o **jogo aberto** enquanto você joga, mudando valores na hora. Com o **NN Studio** aberto, ele vê os objetos que você escolheu, não grava por cima do que você não salvou e o que ele muda aparece no Console. Para ligar: **Jogo > Ligar um agente de IA…** no NN Studio. Na revisão, dois agentes fizeram cada um um jogo do zero só pelo servidor, um de plataforma 2D e um de coletar em 3D, com testes. Como usar: [O servidor MCP](docs/mcp.md) e [Testes de jogo](docs/lua.md#testes-de-jogo).

### Para quem vem da 0.6

- **Os projetos da 0.6 abrem e rodam.** Conferido com os dez modelos de projeto da 0.6.0 publicada:
  - abertos e gravados no NN Studio novo, saem iguais, byte a byte;
  - rodam no `nn` novo sem erro nem aviso;
  - o servidor MCP vê, confere e roda todos.

  Os projetos novos da 0.7 também rodam no `nn` da 0.6.
- **As preferências da 0.6 abrem na 0.7** como estão.
- **Arquivos novos, só se você usar:**
  - `testes/` (os testes de jogo) e o `AGENTS.md` (o guia do agente) vêm nos projetos novos;
  - `.mcp.json` e `.cursor/mcp.json` só se você ligar um agente;
  - nada disso vai para o jogo exportado.
- **O que muda sem você mexer:**
  - **Uma regra de evento com `comment` e condições agora roda:** o `comment` vira o nome dela. Antes, ela era ignorada; o NN Studio não faz regras assim, mas um arquivo editado à mão pode ter.
  - **O Jogador 3D (e todo personagem do `nn.physics3d`) nasce 2 cm acima e fica a 2 cm do chão.** Antes, posto com os pés no chão, ele afundava devagar.
  - O `nn:mostrar_valor` já mostra os valores no primeiro quadro.
  - O `debug_draw` do 3D usa as cores do 2D: verde para o que cai e amarelo para os sensores.
- **Para quem usa as crates:**
  - a API do Lua, os comportamentos prontos e os formatos dos arquivos são os mesmos;
  - o que é novo está nas seções abaixo;
  - no `nn-app`, o `Headless` ganhou `frame_undrawn` e `redraw` (rodar sem desenhar na GPU);
  - no `nn-render`, o `Renderer` ganhou `discard_frame`.

### Revisão (Sprint 54)

- **A prova: agentes fazendo jogos do zero só pelo servidor.** Dois agentes, sem acesso ao código, fizeram cada um um jogo pelas ferramentas, com testes que passam e `conferir_projeto` limpo:
  - um de **plataforma 2D**: moedas, inimigo, placar, vitória e derrota, e o pulo ajustado ao vivo;
  - um de **coletar cristais em 3D**: obstáculos, cronômetro, vitória e derrota.

  O que atrapalhou os dois virou as correções abaixo.
- **NN Studio: o agente de IA junto com você.**
  - **Jogo > Ligar um agente de IA…** (ou "claude" na paleta) mostra como ligar o Claude Code, o Claude Desktop, o Cursor ou outro cliente, já com o caminho do `nn` e da pasta, com botões para copiar. Ele também grava a configuração: o `.mcp.json`, a do Claude Desktop (com uma cópia da de antes) ou o `.cursor/mcp.json`.
  - **`ver_editor`**, a ferramenta nova, mostra ao agente a aba da frente, os **objetos escolhidos** (*"deixa este inimigo mais rápido"*), o que não foi gravado e a imagem da janela.
  - **O agente não grava por cima do que você não salvou:** com mudanças suas numa cena, as ferramentas que gravam nela (e o `desfazer`) pedem para salvar ou descartar antes.
  - **O que o agente muda aparece no Console** com ✨, a ferramenta e o arquivo (clique para abrir).
- **Defeitos:**
  - **Uma regra com `comment` nunca rodava**, sem aviso nenhum. Agora roda, e o `por_regra` mostra o nome dela e se está desligada.
  - **O Jogador 3D afundava devagar quando ficava parado**, se começasse com os pés exatamente no chão (o normal no editor). O personagem agora mantém a folga do chão e das paredes.
  - **No Inspetor, as opções que dependem de um sim/não nunca apareciam**, por exemplo as animações do Jogador 3D quando "Animar" está ligado.
  - Alguns ícones apareciam como quadradinhos no NN Studio: 🧪 (os testes), 🧭 (Caminhos), ✓ e ✗. Agora são 🔬, 🗺, ✔ e ✖.
  - `abrir_jogo` não abria mais o jogo depois que o `nn` era atualizado com o servidor aberto. O erro agora diz o caminho do arquivo, e `reabrir` não fecha o jogo antes de saber que dá para abrir de novo.
- **Para o agente:**
  - **`mexer_no_jogo` aperta e segura teclas** no jogo aberto (`teclas`), contando o tempo de jogo: pausado, elas esperam o `avancar`. Assim dá para ajustar o pulo sem ninguém jogando.
  - **`rodar_jogo` com `ver_corpos`** desenha os corpos da física nas capturas (2D e 3D).
  - **`buscar_documentacao`** devolve só os trechos que citam a busca (antes, seções inteiras de até 27 KB). Se nada cita todas as palavras, mostra o que cita parte delas. Com `secao`, mostra uma seção inteira.
  - **Avisos novos:**
    - um corpo (`body`) bem maior ou menor que o sprite: o `w` e o `h` dos dois são multiplicados pela escala;
    - uma regra que cita um arquivo que ainda não existe;
    - o jogo aberto lento.
  - O `ver_jogo_aberto` diz os objetos que voltaram ao recarregar a cena, e o `mexer_no_jogo` explica o que recarrega e o que fica.
  - O `ver_projeto` lista os textos (`AGENTS.md`, `LEIAME.md`) e os testes.
  - O **`nn:perigo`** ganhou "ir para outra cena" (uma tela de fim de jogo).
  - A documentação ficou mais clara em alguns pontos:
    - o encostar;
    - os valores de `anchor`;
    - o `comment`;
    - a câmera do Jogador 3D;
    - a pasta dos modelos no `AGENTS.md`.
- **Os testes de jogo e o `rodar_jogo` ficaram cerca de 9 vezes mais rápidos** sem placa de vídeo: o jogo sem janela só desenha na GPU quando alguém pede a tela.
- **Robustez:**
  - o servidor não cai mais com bytes que não são UTF-8 na entrada, nem com um defeito numa ferramenta, que vira uma resposta de erro;
  - o canal do jogo aberto confere a senha sem deixar o tempo dizer quanto acertou, recusa pedidos acima de 1 MB e ligações que não mandam a senha em 5 segundos, e aceita até 8 ligações.
- **`nn mcp` sem pasta** usa a do projeto do Claude Code (`CLAUDE_PROJECT_DIR`), ou a atual.
- O `AGENTS.md` não vai mais para o jogo exportado.
- **Documentação:**
  - o guia do servidor ganhou *Junto com o NN Studio*, as teclas e o `ver_corpos`;
  - o do NN Studio ganhou *Agente de IA*;
  - README e "O que vem depois" atualizados.

### O servidor MCP (Sprint 51)

- **`nn mcp <pasta>`: um agente de IA desenvolve o jogo.** O servidor MCP liga o Claude Code, o Claude Desktop, o Cursor ou outro cliente MCP ao projeto. O agente vê o projeto, monta cenas, põe comportamentos e eventos, escreve scripts Lua e confere o que fez. Como ligar e o que cada ferramenta faz: [O servidor MCP](docs/mcp.md).
- **24 ferramentas em português:**
  - **o projeto:** `ver_projeto`, `criar_projeto` (pelos modelos), `mudar_config` (sem perder os comentários do `config.lua`) e `conferir_projeto`;
  - **arquivos:** `listar_arquivos`, `ler_arquivo` (uma imagem volta como imagem), `escrever_arquivo`, `apagar_arquivo` e `desfazer`;
  - **cenas e prefabs:** `ver_cena`, `criar_cena`, `mudar_cena`, `criar_objeto`, `mudar_objeto`, `copiar_objeto`, `apagar_objeto` e `criar_prefab`;
  - **o que existe:** `listar_componentes`, `listar_comportamentos` e `listar_eventos`, com campos, padrões, opções e ajuda;
  - **eventos sem código:** `por_regra` e `tirar_regra`;
  - **código:** `buscar_documentacao` (a API `nn.*` e os guias) e `conferir_lua`.
- **Tudo é conferido antes de gravar.** Um campo com nome ou tipo errado, uma cor fora do formato ou uma opção que não existe voltam como erro, com a lista do que vale (e os nomes parecidos). Um Lua com erro de sintaxe não é gravado: volta a linha e o erro em português. Um arquivo citado que não existe vira aviso.
- **O mesmo formato do NN Studio.** As cenas e prefabs que o agente grava são idênticas às que o editor grava, e o agente vê os objetos no formato do arquivo `.cena`. Um comportamento que precisa de corpo físico já ganha um, como no editor.
- **Seguro:**
  - o servidor não sai da pasta do jogo (nem por `..`, caminho absoluto ou link) e não mexe em `.git`, `dist` e `target`;
  - cada arquivo é gravado de uma vez, então o jogo aberto e o editor nunca veem um arquivo pela metade;
  - `desfazer` volta as mudanças do agente e não passa por cima do que você mudou depois no editor.
- **A documentação vai junto:** os guias de Lua, de 3D, do NN Studio e do MCP são recursos do servidor, e há dois prompts prontos: `criar_jogo` e `melhorar_jogo`.
- **Todo projeto novo vem com um `AGENTS.md`,** que explica a um agente como o projeto se organiza e como ligar o servidor.
- **No AppImage:** `NN-Studio-<versão>-x86_64.AppImage nn mcp <pasta>` roda o `nn` que vem dentro dele.
- **Para quem usa as crates:** as cenas, prefabs, eventos, comportamentos e o `config.lua` saíram do NN Studio para a crate nova **`nn-projeto`**, usada pelo editor e pelo servidor. A crate **`nn-mcp`** tem o servidor.

### Rodar e ver (Sprint 52)

- **O agente joga o jogo: `rodar_jogo`.** A ferramenta nova do servidor MCP roda o jogo **sem janela** e conta o que aconteceu. O jogador é simulado com ações no tempo certo: apertar e segurar teclas, clicar num ponto ou num botão pelo nome, escrever e capturar a tela. A resposta traz:
  - os erros, com o arquivo e a linha (o jogo para ali);
  - os avisos e os `print`;
  - as regras dos eventos que dispararam;
  - as trocas de cena;
  - o que mudou nos objetos e nas variáveis;
  - as capturas da tela, como imagens.

  Uma ação `lua` prepara a situação (levar o herói até a porta). Como usar: [Rodar e ver](docs/mcp.md#rodar-e-ver).
- **Testes de jogo.** Arquivos Lua em `testes/` que jogam o jogo e conferem o resultado: `t:hold("right", 1)`, `t:press("space")`, `t:click("jogar")`, `t:find("heroi")`, `t:check(...)`, `t:wait_until(...)`. Cada teste roda num jogo novo, e o que falha diz a linha, o motivo e o que o jogo escreveu. Como escrever: [Testes de jogo](docs/lua.md#testes-de-jogo).
  - **`nn testar [pasta]`** roda os testes no terminal e sai com erro se algum falhar. Opções: `--filtro`, `--capturas` (grava a tela de quem falhou) e `--tamanho`.
  - **`rodar_testes`** roda os mesmos testes pelo servidor MCP, com a tela na hora da falha.
  - Os modelos `visual`, `aventura`, `nave`, `visual3d` e `padrao` vêm com **`testes/jogo.lua`**, que confere o principal de cada jogo.
  - A pasta `testes/` não vai para o jogo exportado.
- **O jogo sem janela é igual toda vez:**
  - 60 quadros por segundo de jogo, o mais rápido que der;
  - sem som;
  - o `math.random` sempre na mesma sequência;
  - o `nn.save` só na memória: rodar e testar não mexem nos recordes de quem joga.

  Um laço sem fim num script vira erro depois de 5 segundos, em vez de travar.
- **O servidor diz ao agente para jogar e testar.** As instruções, os prompts `criar_jogo` e `melhorar_jogo` e o `AGENTS.md` dos projetos novos pedem para conferir jogando e para escrever um teste para o que mudou.
- O erro de uma tecla desconhecida (`nn.input.down("pulo")`) agora lista os nomes que valem.
- **Para quem usa as crates:** o `nn-script` ganhou o jogo sem janela (`game_run::GameRun`: roteiro de ações, estado da cena, expressões, capturas), os testes de jogo (`testing`) e a captura do log por thread (`capture`). O `ScriptRuntime` diz a cena da frente (`front`) e os erros das cenas (`take_errors`).

### Ao vivo (Sprint 53)

- **O agente acompanha o jogo aberto.** Com o jogo aberto numa janela (pelo ▶ Jogar do NN Studio, por `nn .` ou pela ferramenta nova `abrir_jogo`), o agente de IA vê e mexe nele enquanto você joga. Como usar: [O jogo aberto](docs/mcp.md#o-jogo-aberto).
  - **`ver_jogo_aberto`:**
    - a cena da frente e se está pausado;
    - os erros, avisos e prints e as regras que dispararam desde a última olhada;
    - o que mudou nos objetos e nas variáveis;
    - a imagem da tela agora.
  - **`mexer_no_jogo`**, na hora e sem gravar:
    - muda campos de objetos e comportamentos (o pulo do herói, a velocidade do inimigo) e as variáveis;
    - roda Lua;
    - pausa, avança quadros e fecha o jogo.

    Bom para acertar um número junto com você antes de gravar no arquivo.
  - **`abrir_jogo`** abre o jogo numa janela (também numa `cena`) e liga nele; se o jogo já estiver aberto, usa ele.
- **Seguro:** o jogo aberto atende numa porta do próprio computador, com uma senha num arquivo que só o seu usuário lê. Um jogo exportado não atende ninguém.
- **NN Studio: Jogo > Conferir com os testes (Ctrl+F5).** Roda os testes de `testes/` em segundo plano e mostra no Console ✔ para o que passou e ✖ com o motivo para o que falhou, com link para a linha.
- **`nn <pasta> --cena <arquivo>`** abre o jogo começando numa cena.
- **Para quem usa as crates:**
  - o `nn-script` ganhou o canal do jogo aberto (`remote`: `listen`, `Client`) e `run_project_from`;
  - o `nn-app` tira a imagem do quadro da janela (`Context::request_screenshot` e `take_screenshot`);
  - o `nn-render` lê qualquer textura (`read_texture_rgba`, também BGRA).

## 0.6.0

A versão do **modelar e animar no NN Studio**. O **modelador** monta modelos 3D low-poly de peças coloridas. Ele tem formas prontas, puxar e encolher faces, pintar com a paleta e o **espelho X**. O **Repetir em linha e em círculo** (também na cena), as **peças que se encaixam**, os **kits** e uma **biblioteca** de modelos prontos acabam com o trabalho repetido. O **animador** põe um **esqueleto pronto** (humanoide, quadrúpede ou simples), que liga as peças sozinho. Depois, é posar quadro a quadro numa **linha do tempo** ou usar as **animações prontas** (parado, andar, correr, pular...), que o Jogador 3D toca sozinho. Ao salvar, sai o `.glb` que as cenas e o jogo usam: a engine não mudou. Três tutoriais novos: **Modele uma casinha**, **Uma cerca e uma fogueira** e **Anime seu personagem**. Para começar: [Modelador](docs/studio.md#modelador), [Modelar sem repetir](docs/studio.md#modelar-sem-repetir) e [Esqueleto e animações](docs/studio.md#esqueleto-e-animações).

### Para quem vem da 0.5

- **Os projetos da 0.5 abrem e rodam.** Conferido com os dez modelos de projeto da 0.5, também depois de abrir e gravar cada cena no NN Studio da 0.6. A 0.5 também roda as cenas gravadas pela 0.6.
- **A engine, a API Lua, os comportamentos prontos e as crates do Rust não mudaram**: só o NN Studio.
- **As preferências da 0.5 abrem na 0.6** como estão.
- **Arquivos novos, só se você usar:**
  - os modelos do modelador são `.modelo` (texto, como as cenas), com o `.glb` do mesmo nome ao lado;
  - a biblioteca pessoal de peças fica na pasta `biblioteca`, ao lado das preferências.
- **O que muda sem você mexer:**
  - um `.glb` trocado por fora (exportado do Blender, por exemplo) agora atualiza as cenas abertas na hora;
  - o botão direito de um objeto da cena ganhou o **🔁 Repetir…**.

### Revisão (Sprint 50)

- **O Inspetor mostra o modelo aberto:** com a aba de um `.modelo` na frente, ele mostra as peças, os triângulos, o tamanho, os ossos e as animações, em vez da última cena.
- **Pontos em modelos grandes:** num modelo com 180 peças e 3.360 triângulos, escolher pontos passou de quase 3 s para uns 30 ms por quadro. O NN Studio agora sabe o que aparece por um mapa de profundidade da vista, refeito só quando o modelo ou a câmera mudam.
- **A barra do modelador não passa mais do lado da aba** com o esqueleto ligado (a grade ia para fora, e o painel da direita cobria o Inspetor).
- **Um `.modelo` com erro** mostra o motivo, não é gravado por cima e abre sozinho quando é corrigido fora.
- **Documentação:** o modelador entrou no README, e o "O que vem depois" foi atualizado.

### O modelador (Sprint 47)

- **Modelos 3D feitos no NN Studio:** **Arquivos > ➕ Novo > 🎲 Modelo 3D** cria um `.modelo` e abre a aba do **modelador**, com um cubo para começar. O modelo é feito de **peças** (paredes, telhado, cabeça...), cada uma com pontos e faces coloridas, no estilo low-poly.
- **Ao salvar, o `.glb` sai junto**, com o mesmo nome: é ele que vai para as cenas e para o jogo, e abre no Blender. Cada peça vira uma parte do arquivo, e cada cor, um material. A engine não mudou.
- **A vista anda como a da cena 3D:** botão direito + WASD voa, Alt + arrastar gira em volta, a rodinha aproxima e **F** enquadra.
- **Escolher** pontos (**1**), faces (**2**) ou peças inteiras (**3**): clique, **Shift** junta, e arrastar no vazio faz uma **caixa de seleção**. As alças são as da cena: **W** move, **E** gira e **R** escala. Arrastar o que está escolhido também move, de frente para a câmera.
- **Ferramentas:**
  - **⬆ Puxar (P):** tira um bloco das faces escolhidas, para fora ou para dentro. O mouse manda, o clique termina e o **Esc** desfaz.
  - **▣ Encolher (I):** faz uma face menor dentro de cada face (portas, janelas).
  - **🔗 Juntar (M):** junta os pontos escolhidos num só.
  - **🗑 Apagar (Del)**, **Duplicar peça (Ctrl+D)** e **Escolher tudo (Ctrl+A)**.
- **Formas prontas** para começar uma peça: cubo, cilindro, cone, pirâmide, esfera, plano e rampa, no **➕ Peça**, no botão direito da vista ou no **Shift+A**.
- **Cores:** clicar numa cor da paleta pinta o que está escolhido (faces ou a peça toda). Com **🎨 Pintar (B)**, cada clique pinta uma face, e com **Shift**, a peça inteira. Dá para guardar cores novas na paleta do modelo.
- **↔ Espelho X:** modela só uma metade, e a outra aparece copiada. Os pontos do meio ficam no meio. Desligar transforma a cópia em pontos e faces de verdade.
- **Peças:** a lista à direita mostra, esconde e renomeia as peças (duplo clique ou F2).
- **Na cena:** arrastar o `.modelo` do painel Arquivos para uma cena 3D cria o objeto com o `.glb`. Salvar o modelo de novo atualiza as cenas abertas. Trocar um `.glb` por fora (exportado do Blender, por exemplo) também atualiza agora.
- **Ctrl+Z / Ctrl+Y** desfazem e refazem no modelador, e a contagem de triângulos aparece na barra.
- **Tutorial novo:** **Modele uma casinha**, com paredes, uma porta afundada, um telhado vermelho e a casinha na cena.

### Modelar sem repetir (Sprint 48)

- **🔁 Repetir em linha e em círculo:**
  - **No modelador**, com as peças escolhidas: cercas, escadas, raios de roda, pedras em volta da fogueira.
  - **Na cena 2D e 3D**, pelo botão direito de um objeto: fileiras de árvores, moedas em arco.
  - A prévia aparece enquanto você mexe na quantidade, na distância, no raio, no eixo e em quanto da volta. **Aplicar** fica com as cópias num passo só de desfazer.
  - No modelador, o círculo pode ser **em volta do meio do modelo**.
- **Peças que se encaixam:** com o botão direito (ou o Shift+A) em cima de uma face, a peça nova nasce no meio dela, virada para fora.
- **Kits** no ➕ Peça: **Casa**, **Natureza**, **Personagem** e **Veículo**.
  - O telhado da Casa fica do tamanho da face.
  - Os braços e as pernas do Personagem giram no ombro e no quadril, prontos para o animador.
- **📚 Biblioteca com 10 modelos prontos:** árvore, pinheiro, pedra, cerca, casinha, baú, lampião, fogueira, personagem e carrinho.
  - O **Começar com** do ➕ Novo > Modelo 3D cria um modelo a partir de um deles.
  - No ➕ Peça, eles entram no modelo aberto, como peças.
- **⭐ Guardar na biblioteca:** as suas peças ficam para usar em qualquer modelo e qualquer projeto, numa pasta ao lado das preferências.
- **🔀 Criar variação:** uma cópia do modelo (com o `.glb`), aberta numa aba nova.
- **Cores:** o botão direito numa cor da paleta **troca ela no modelo todo** ou **escolhe as faces dela**.
- **Tutorial novo:** **Uma cerca e uma fogueira**.

### Esqueleto e animador (Sprint 49)

- **💀 Esqueleto** no modelador: **Humanoide**, **Quadrúpede** ou **Simples**.
  - O esqueleto se ajusta ao tamanho do modelo.
  - Cada peça vai para o osso do nome dela (ou para o mais perto) e **segue o osso inteira**, sem entortar.
  - O personagem da biblioteca e o kit Personagem já saem ligados.
- **💀 Ossos (4):** mostra os ossos e move as juntas. **Segue** troca o osso de uma peça, e a lista **Ossos** escolhe e renomeia.
- **🎬 Animar:** a linha do tempo, as animações do modelo, ▶ Tocar (Espaço), repetir ou não, e a duração.
  - **Posar:** girar um osso com as alças grava um quadro-chave no instante.
  - **Linha do tempo:** arrastar os quadros muda o lugar deles, e o botão direito apaga, copia e cola a pose.
- **Animações prontas:**
  - **Humanoide:** parado, andar, correr, pular, cair, golpe e acenar, com os nomes que o Jogador 3D, o Inimigo 3D e o Chefe já procuram;
  - **Quadrúpede:** parado, andar e correr;
  - **Simples:** girar, balançar e abrir.
- **Ajustes rápidos:** 🐇 Mais rápida, 🐢 Mais lenta, 💪 Mais forte e Mais fraca.
- **O `.glb` leva o esqueleto e as animações:** a Animação 3D e o Jogador 3D tocam elas na cena e no jogo, e a engine não mudou. O `.modelo` guarda os ossos e os quadros-chave.
- **O kit Personagem trocou os lados:** o "Braço esquerdo" e a "Perna esquerda" agora ficam no +X, que é o lado esquerdo de quem olha para a frente (+Z). Os modelos já feitos não mudam.
- **Tutorial novo:** **Anime seu personagem**.

## 0.5.0

A versão do **NN Studio mais gostoso de usar**. O **botão direito** funciona em todo lugar e cria onde você clicou. A **busca rápida (Shift+A)** acha o que criar pelo nome, até com erro de digitação. Duplicar ficou sem trabalho repetido (**Ctrl+D que repete o passo**, **Alt+arrastar**, **Shift+D**), e dá para **copiar e colar** objetos e componentes. O Inspetor edita **vários objetos de uma vez**, os **⭐ favoritos** guardam o que você monta toda hora, os **avisos rápidos com Desfazer** deixam errar sem medo, os painéis vazios ensinam e as **Preferências** trazem a cor de destaque, as animações e os sons. A engine não mudou: os jogos rodam igual. Para começar: [A aba da cena](docs/studio.md#a-aba-da-cena), [Inspetor](docs/studio.md#inspetor) e [Preferências](docs/studio.md#preferências).

### Para quem vem da 0.4

- **Os projetos da 0.4 abrem e rodam.** Conferido com os dez modelos de projeto da 0.4, também depois de abrir e gravar cada cena no NN Studio da 0.5. A 0.4 também roda as cenas gravadas pela 0.5.
- **A engine, a API Lua, os comportamentos prontos e as crates do Rust não mudaram**: só o NN Studio.
- **As preferências da 0.4 abrem na 0.5**, com o novo no padrão. Voltando para a 0.4, ela abre as da 0.5 e ignora o que não conhece.
- **O que muda sem você mexer:**
  - o botão direito na cena abre um menu (segurar e arrastar continua movendo a vista);
  - **Alt + arrastar** um objeto no 2D faz uma cópia;
  - o **Ctrl+D** repete o passo da duplicação anterior;
  - a Lâmpada e o Holofote do ➕ Objeto nascem no alto;
  - os avisos rápidos aparecem embaixo, no meio da janela.
- **Pasta nova, só se você usar:** os ⭐ favoritos ficam em `favoritos/`, dentro do projeto.

### Revisão (Sprint 46)

- **Chão grande no botão direito:** algo grande e baixo (como a ilha do modelo 3D, um cilindro de 28 × 4 × 28 m) conta como chão: o botão direito nele abre o menu de criar, e o menu dele vai para o fim. Passar o mouse por cima também não desenha mais a caixa dele, que cobria a vista inteira.
- **O menu do botão direito reabre** no ponto novo quando você clica com o botão direito em outro lugar com ele aberto. Antes, esse clique só fechava o menu.
- **Esc fecha o menu.** Antes, com o mouse na cena, o Esc limpava a seleção e o menu ficava aberto. Com um menu aberto, os atalhos da cena (Shift+A, Del, Ctrl+C...) também esperam.
- **🗑 Tirar dos favoritos**, no submenu ⭐ Favoritos: o arquivo vai para a lixeira, perguntando antes.
- O aviso de favorito ficou curto ("tocha guardado nos favoritos ⭐"); o caminho completo continua no Console.
- **Desempenho:** o NN Studio mede o mesmo da 0.4 numa cena 2D com 2.000 objetos e na ilha com o mouse passando por cima.
- **Documentação:** imagens novas do botão direito, da busca rápida, do Inspetor com vários objetos e das Preferências, e o "O que vem depois" agora fala do modelador e do animador.
- **Ferramentas de revisão** (`tests/revisao.rs`):
  - `uso_de_ponta_a_ponta_v05` monta uma cena só com o que é novo e grava as telas;
  - `ilha_quadros_por_segundo` mede o tempo de um quadro com a ilha aberta;
  - `capturas_da_v05_para_a_documentacao` grava as imagens da documentação.

### Amigável e divertido (Sprint 45)

- **Avisos rápidos** no canto de baixo: "1 objeto apagado", "3 objetos colados", "Salvo: fase.cena", "📦 Prefab criado", "⭐ Guardado nos favoritos" e os erros. Os de apagar, recortar e colar têm **Desfazer**, que só aparece enquanto a cena não mudou depois. Com o mouse em cima, o aviso espera.
- **Ao salvar**, a aba mostra um ✔ no lugar do • por um instante.
- **Objetos novos brilham**: o que acabou de nascer (criado, colado, duplicado) ganha um contorno amarelo que some em menos de um segundo, para você ver onde ele caiu.
- **Painéis vazios que ensinam:**
  - uma cena vazia mostra como começar, com **🔍 Buscar o que criar**;
  - o Console vazio explica o que aparece ali e tem **▶ Jogar**;
  - a Hierarquia de uma cena vazia também tem a busca.
- **💡 Dica do dia** no Início, com **Próxima dica**.
- **Comemorações:** a primeira vez que o jogo de um projeto roda, e a primeira exportação, ganham um aviso e um confete curtinho. Uma vez por projeto.
- **Preferências** (Exibir > Preferências):
  - tema e **cor de destaque** (azul, verde, roxo, laranja, rosa);
  - modo e tamanho da interface;
  - **animações**: desligadas, menus, avisos e brilhos ficam parados;
  - **comemorações**;
  - **sons da interface**, desligados no começo e gerados pelo próprio NN Studio;
  - limpar o Console ao jogar e avisar versões novas.
- Os rótulos com ✓ (que apareciam como um quadradinho) agora usam ✔.

### Menos trabalho repetido (Sprint 44)

- **Ctrl+D repete o passo:** duplicou e moveu a cópia? O próximo Ctrl+D põe a nova o mesmo tanto adiante. Uma cerca ou uma fileira sai com Ctrl+D, Ctrl+D, Ctrl+D…
- **Alt + arrastar** leva uma cópia e deixa o original. No 2D, arrastando o objeto ou a seta; no 3D, arrastando uma seta do Mover (Alt + arrastar no resto continua girando a câmera).
- **Shift+D:** a cópia segue o mouse até o clique (com Encaixar, na grade). Esc ou o botão direito desistem.
- **Copiar, recortar e colar objetos** (Ctrl+C, Ctrl+X, Ctrl+V), com os filhos, na cena e na Hierarquia. Na cena, o colado vai para onde o mouse está. Cola em outra cena do projeto e em outro NN Studio aberto. O botão direito no vazio tem **📋 Colar aqui**, e o menu de um objeto tem Copiar e Recortar.
- **Copiar e colar componentes:** o menu do componente (☰ ou o botão direito no nome) tem **📋 Copiar componente** e **📋 Colar valores**. Embaixo do ➕ Adicionar componente, **📋 Colar componente** põe uma cópia no objeto.
- **Vários objetos no Inspetor:** com vários selecionados, ele mostra a Transformação e os componentes que todos têm. O que você muda vale para todos, só no campo mexido. Um **≠** marca os campos com valores diferentes. Adicionar, colar ou remover um componente também vale para todos.
- **⭐ Favoritos:** no botão direito de um objeto, **⭐ Guardar nos favoritos** guarda um molde em `favoritos/`. Ele aparece no topo do ➕ Objeto, do botão direito e do Shift+A. Diferente do prefab, cada cópia é livre.
- **Tutorial:** o **Seu primeiro mundo 3D** tem um passo novo, **Uma fileira de árvores**.

### Botão direito e criar rápido (Sprint 43)

- **Botão direito na cena** (2D e 3D): clicar e soltar, sem arrastar, abre um menu. No vazio, ele cria o objeto **onde você clicou**, e não mais no meio da vista. Em cima de um objeto, mostra o menu dele: Renomear, Duplicar, ⛶ Focar, **Copiar e Colar posição**, **➕ Criar dentro**, Criar filho vazio, 📦 Criar prefab, Excluir e **➕ Criar aqui**. Num chão (plano, terreno, mapa de tiles), o menu é de criar em cima. Arrastar com o botão direito continua movendo a vista (e, no 3D, voando); pintando tiles, ele continua pegando o tile.
- **Busca rápida para criar (Shift+A):** com o mouse na cena, abre uma caixinha ali. Digite *luz*, *cubo*, *inimigo*, o nome de um prefab, de um modelo ou de uma imagem e aperte **Enter**: o objeto nasce onde o mouse estava. Acha sem acento, por outras palavras (*bloco* acha o Cubo; *lago*, a Água) e com um erro de digitação (*tocah* acha a Tocha). Os últimos que você usou vêm primeiro.
- **Hierarquia:** o botão direito no espaço vazio cria um objeto, e o menu de um objeto é o mesmo da cena, com **➕ Criar dentro** (o objeto novo já nasce filho).
- **Inspetor:** o botão direito no nome de um componente abre o menu dele (o mesmo do ☰), que agora também tem **↺ Restaurar o padrão**: os valores voltam ao começo, sem mudar o arquivo, a forma ou o tipo.
- **F2 na cena** renomeia o objeto selecionado, na Hierarquia (que rola até ele).
- **Os atalhos aparecem à direita** nos menus (F2, Ctrl+D, F, Del), e a lista de **Ajuda > Atalhos do teclado** tem os novos.
- A **Lâmpada** e o **Holofote** do ➕ Objeto nascem no alto (2 m e 4 m acima do chão), e não mais no chão.
- **Tutorial:** o **Seu primeiro mundo 3D** tem um passo novo, **Criar com o botão direito**.

## 0.4.0

A versão dos **mundos 3D mais vivos**: **luzes** (tochas que tremulam, lâmpadas e holofotes) e coisas que **brilham** no escuro; **dia e noite**, com o sol e a lua cruzando o céu, estrelas e luzes que acendem sozinhas; **água** com ondas que reflete o céu; **vento** que balança árvores, grama e bandeiras; e o **halo** em volta do que brilha. Tudo no **NN Studio sem código**. O Vale 3D ganhou uma caverna escura, a ilha tem dia e noite, mar com ondas e uma bandeira na arena, e há três tutoriais guiados novos: **Uma caverna iluminada**, **Dia e noite** e **Um lago com vida**. Para começar: [Luzes e brilho](docs/studio.md#luzes-e-brilho), [Ambiente](docs/studio.md#ambiente) e [Água, vento e halo](docs/studio.md#água-vento-e-halo).

### Para quem vem da 0.3

- **Os projetos da 0.3 abrem e rodam.** Conferido com os nove modelos de projeto da 0.3, também depois de abrir e gravar cada cena no NN Studio da 0.4. Algumas coisas ficam diferentes no jogo sem você mexer:
  - o **Halo do brilho** vem ligado: o fogo e o disco do sol ganham um halo suave. Para o jeito de antes, **Halo do brilho** = 0 (☀ Ambiente);
  - a **água dos Terrenos** tem ondas e reflete o céu;
  - **partículas no 3D** que não somam luz (fumaça, poeira) recebem a luz do mundo: num ambiente escuro, ficam escuras.
- Uma cena gravada pela 0.4 pode usar coisas que a 0.3 não conhece (a **Luz**, o **Dia e noite**). Para voltar à 0.3, use uma cópia do projeto.
- Nenhuma função da API Lua nem comportamento pronto foi tirado ou mudou de nome.
- **No Rust:** `ModelPart`, `ParticleConfig3`, `Sky3D` e `Stats3` ganharam campos (`emissive`, `wind`, `stars`, `lamps` e `halo`). Quem monta essas structs com `..Default::default()` não precisa mudar nada.

### Revisão (Sprint 42)

- **O halo só é desenhado quando algo brilha na tela:** o Brilho, a água, o fogo, o sol ou a lua. Numa cena sem nada disso, ele não pesa. `nn.d3.stats().halo` diz se foi desenhado.
- **Mais leve:** a água faz a conta da sombra uma vez só, e o que está todo na névoa (o mar lá longe) não é mais iluminado. Sem placa de vídeo, a ilha ficou uns 10 ms mais rápida por quadro, em 1280 × 720.
- **Contorno nos textos da tela:** no Inspetor do **Texto**, **Contorno** põe uma borda em volta das letras, para o texto ler bem em cima do céu (`outline` no componente). Os placares e as dicas da ilha e do Vale 3D usam.
- **O fogo inclina com o vento**, em vez de voar com ele. O pronto **Fogo** e a **Tocha** vêm com **Levadas pelo vento** = 0,3, e o fogo tem um halo um pouco mais fraco. As fogueiras da ilha e do vale ficaram menores, sem virar uma bola branca.
- **Ferramentas novas:**
  - `tempos_da_ilha` mede quanto cada coisa pesa na ilha: o halo, o vento, a água, a grama e as sombras;
  - `tempos_da_caverna` mede também sem o halo.
- **Documentação:** o "O que vem depois" do guia do NN Studio foi atualizado, e há imagens novas da ilha, do vale, da caverna e do NN Studio.

### Água, vento e brilho (Sprint 41)

- **Água:** na Forma 3D, **Material: Água**, com ondinhas que se mexem, o céu refletido (mais forte olhando de lado) e o brilho do sol batendo. **➕ Objeto > Água (lago)** traz um plano assim. A água dos **Terrenos** também ganhou ondas e reflexo.
- **Vento** (☀ Ambiente): a força e o lado para onde sopra. O que tem **Balança com o vento** (na Forma 3D e no Modelo 3D) balança com rajadas: embaixo parado, em cima mais. As **partículas** no 3D são levadas pelo vento (**Levadas pelo vento**, no Inspetor delas).
- **Halo do brilho** (☀ Ambiente): um halo suave em volta do que brilha (o Brilho, as partes que brilham nos `.glb`, o fogo, o sol e a lua). Vem ligado; 0 tira.
- Os **Ambientes prontos** não mexem mais no Dia e noite, no Vento nem no Halo.
- **Modelo novo:** `grama.glb` (um tufo), nos modelos de projeto 3D.
- **A ilha (modelo 3D):** o mar tem ondas e reflete o céu; árvores e grama balançam com o vento, e há uma bandeira na arena. **No Vale 3D:** o lago tem ondas, e a grama balança. Na caverna, tochas e cristais ganham o halo.
- **Tutorial guiado novo: Um lago com vida** (10 minutos).
- **No Lua:** `nn.d3.wind(opts)`, `nn.d3.bloom(v)`, as opções `material = "water"` e `sway` do `nn.d3.draw`, `emissor:set_wind(x, y, z)`, os campos `wind`, `wind_turn` e `bloom` do `environment`, `material` e `sway` nas formas e modelos e `wind` nas partículas.
- **No Rust:** `Wind3D`, `Bloom` e `Material3D` no `nn-render`, com `Renderer::set_wind3d`, `set_bloom3d`, `draw_mesh_material` e `draw_model_material`.

### Dia e noite (Sprint 40)

- **Dia e noite** (☀ Ambiente): a hora anda no jogo. O sol nasce a leste às 6h, cruza o céu e se põe a oeste às 18h; de noite, a lua dá uma luz azulada fraca, e aparecem estrelas. As cores passam pelo amanhecer, pelo dia (as cores da cena), pelo entardecer e pela noite. **Um dia dura** quantos minutos, e **Começa às**, a hora do começo. No NN Studio, a vista mostra a cena nessa hora.
- **O relógio:** as variáveis **relogio** ("21:30"), **hora** (o número) e **dia**.
- **Só à noite**, na Luz, e **Brilha só à noite**, na Forma 3D e no Modelo 3D (o brilho do arquivo também): acendem sozinhos ao anoitecer. Sem o ciclo, "de noite" é um ambiente escuro, como a Noite e a Caverna.
- **Eventos:** **Amanhecer ou anoitecer** e **Estiver de dia ou de noite** (Quando); **Mudar a hora**, **Parar ou continuar o tempo** e **Mudar o ambiente** (para um pronto, aos poucos), em Então.
- **Ambiente pronto novo:** **Amanhecer**. A **Noite** ficou um pouco mais escura, para as luzes aparecerem melhor.
- **A ilha (modelo 3D) tem dia e noite:** um dia de 4 minutos, começando às 9h. Postes com lampião no caminho e na ponte acendem sozinhos de noite, o pescador acende o lampião na mão, a fogueira ilumina em volta e o placar mostra a hora.
- **Partículas no 3D** (fumaça, poeira) recebem a luz do mundo: de noite, ficam escuras (ou da cor da tocha perto). O fogo, que soma luz, continua aceso.
- **Tutorial guiado novo: Dia e noite** (10 minutos).
- **No Lua:** `scene:time_of_day()`, `scene:set_time(h)`, `scene:pause_time(sim)`, `scene:environment(pronto, segundos)` e `scene:is_night()`; o `environment` ganha `cycle`, `day_length` e `start_hour`; `nn.d3.sky` ganha `stars`; e `nn.d3.draw`, `emissive = false`.

### Luzes (Sprint 39)

- **Luzes no 3D:** a **Luz** (componente novo) é uma **lâmpada** (ilumina em volta, até o **Alcance**) ou um **holofote** (ilumina num cone, para a frente do objeto), com cor e força, e pode **tremular** como fogo. Valem as 32 mais perto da câmera; só o sol faz sombra.
- **Brilho:** na **Forma 3D** e no **Modelo 3D**, a própria cor aparece acesa mesmo no escuro. Os modelos `.glb` com emissão no material (`emissiveFactor` e `KHR_materials_emissive_strength`) brilham sozinhos nessas partes.
- **No NN Studio:**
  - **➕ Objeto > Luz**, com **Lâmpada**, **Tocha** (a chama com o fogo e a luz tremulando) e **Holofote**;
  - as luzes acendem na vista 3D, e a escolhida mostra até onde chega;
  - **☀ Ambiente > Pronto**: **Dia**, **Entardecer**, **Noite** e **Caverna**, de uma vez.
- **Eventos:** **Acender ou apagar luz** e **Mudar a cor da luz**. Na ação **Ir para cena**, **Como: guardando esta (para voltar)**: a de agora fica como está, e **Fechar esta cena**, na outra, volta para ela.
- **Modelos novos:** `tocha.glb`, `lampiao.glb` e `cristal.glb`, com partes que brilham, nos modelos de projeto 3D. A pedra do `cajado.glb` também brilha.
- **O Vale 3D ganha uma caverna:** um arco de pedra no pé do morro leva a uma caverna escura, iluminada por tochas, cristais, um lampião e a luz que entra por uma fresta em cima da sexta estrela. O herói entra com uma tocha na mão; voltando, o vale continua de onde parou. Agora são 6 estrelas.
- **Tutorial guiado novo: Uma caverna iluminada** (10 minutos).
- **No Lua:** `nn.d3.lamp(opts)` (uma lâmpada ou um holofote no quadro), a opção `glow` do `nn.d3.draw`, `lamps` no `nn.d3.stats()` e o campo `obj.light3d`.
- Voltando de uma cena guardada (ou de uma aberta por cima), uma cena 3D põe de novo o céu, o sol e a névoa dela.

## 0.3.0

A versão do **combate**: inimigos que contornam paredes, buracos e água; uma espada na mão e golpes; tiros que miram pelo meio da tela; números de dano; ondas de inimigos; e chefes com vários ataques, tudo montado no **NN Studio sem código**. A ilha do modelo 3D ganhou uma arena com o besouro-rei, e há um tutorial guiado novo, **Um chefe**. Para começar: o guia [Comece aqui no 3D](docs/3d.md) e a seção [Combate](docs/studio.md#combate-espada-tiros-e-inimigos-que-atacam) do NN Studio.

### Para quem vem da 0.2

- **Os projetos da 0.2 abrem e rodam.** Algumas coisas ficaram diferentes no jogo:
  - **Atirar no 3D**, num modelo com a animação `atirar` (como o herói dos modelos), agora toca essa animação, e o tiro sai 0,08 s depois, com o braço levantado. Para o jeito de antes: **Animação** = nenhuma, ou **O tiro sai depois de** = 0.
  - **Vida** fica vermelha um instante a cada dano. Para desligar: **Fica vermelho ao levar dano**.
  - **Vida** com a variável padrão (**vidas**) só usa a variável no jogador. Num inimigo, ela é ignorada; antes, ele dividia as vidas com o herói. Para mostrar a vida de um inimigo numa variável, use outro nome.
  - Um **Inimigo 3D com um Golpe** não machuca mais só de encostar (o golpe machuca). Sem Golpe, nada muda.
- Os **Atirar** novos no 3D vêm com **para o meio da tela** e o **botão direito** do mouse. Os que já estão nas cenas continuam como estavam.
- Nenhuma função da API Lua nem comportamento pronto foi tirado ou mudou de nome. A Vida recebe agora `dano(n, quem)`; o `quem` é opcional.

### Revisão

- **Pausa no acerto:** quando um golpe acerta (do herói ou nele), o jogo para por uma fração de segundo, o que dá peso ao golpe. No Golpe, **Pausa no acerto** (0,06 s; 0 = sem pausa). No Lua, `scene:hitstop(segundos)` e a opção `hitstop` do `scene:hit`.
- **Inimigos em grupo se espalham:** vários Inimigos 3D atrás do herói se espalham em volta dele, em vez de um empurrar o outro. Fica mais bonito e mais leve: com 30 besouros na arena, a simulação gasta metade do tempo de antes (com 60, um quarto).
- **Mais leve com muitos objetos:** a lista de quem os comportamentos procuram (o jogador, um nome) é montada uma vez por passo, e não a cada pergunta.
- **Ícones** (o componente que mostra as vidas em corações) desenham no máximo 100. Antes, um número enorme na variável travava o jogo.
- **Sons novos**, nos modelos de projeto 3D: `golpe.wav` (a espada cortando o ar), `acerto.wav` (o baque) e `rugido.wav` (o besouro-rei se preparando). A ilha e o Vale 3D já usam.
- **Testes novos:**
  - em todo modelo de projeto, cada objeto começa no jogo onde o NN Studio mostra (teria pegado o erro das cópias de prefab da 0.2.1);
  - o repositório confere que todo crate está nas dependências do workspace com a mesma versão, e que o CHANGELOG tem a seção da versão.

### Caminhos

- **Inimigos que contornam paredes:** o Inimigo 3D e o Perseguir (voando, ou num jogo visto de cima) acham um caminho em volta de paredes, árvores, cenário e tiles sólidos, em vez de ir reto e ficar presos. No 3D, também não entram em buracos, barrancos nem na água dos terrenos. A opção **Contorna paredes** vem ligada (desligue para o jeito de antes).
- **Grade de caminhos** de cada cena, montada sozinha a partir dos corpos parados (e dos terrenos, no 3D) na primeira vez que alguém pede, e de novo quando um corpo parado aparece ou some. No 3D, vale o chão debaixo de uma plataforma no ar e uma ponte por cima da água.
- **🧭 Caminhos** no NN Studio (na barra da cena, com o jogo aberto): mostra no jogo o chão por onde dá para andar e o caminho que cada inimigo segue. Pelo terminal, `NN_CAMINHOS=1`.
- **No Lua:** `scene:path(a, b)`, `scene:steer(...)` (para comportamentos seus), `scene:nav()` com `nav:path`, `nav:walkable`, `nav:sight`, `nav:floor`, `nav:block` e `nav:unblock`, e `mundo:nav(opts)` no `nn.physics` e no `nn.physics3d`.
- No modelo **Aventura**, as lesmas (que perseguem voando) contornam o lago e a parede de tijolos; no **Vale 3D**, os besouros não entram mais no lago, e na **ilha**, não caem no mar.
- Crate novo `nn-nav` (a grade e o A*, sem depender do resto da engine).

### Combate 3D

- **Golpe** (comportamento novo, 3D): ataque de perto. O jogador golpeia com o clique (ou J, ou o X do controle); um inimigo, sozinho, quando o herói chega perto (para, vira e golpeia). Toca a animação `golpe` e, no instante do acerto, tira vida de quem está na frente (alcance e largura ajustáveis) e empurra. Com um Golpe, o Inimigo 3D não machuca mais só de encostar.
- **Atirar no 3D:** com **para o meio da tela** (o padrão no 3D), o tiro vai para onde a câmera olha, também para cima e para baixo, e pode sair de um osso (a mão). Uma mira aparece no meio da tela, e a câmera do Jogador 3D vai para o ombro (**Câmera de lado**). A tecla padrão no 3D é o botão direito do mouse (no controle, o RT ou o RB), e o herói toca a animação `atirar`.
- **Projétil** (comportamento novo, 3D), para o prefab do tiro: tira vida de quem acerta, empurra, some ao bater numa parede (com faíscas) e não acerta quem atirou; pode cair em arco ou atravessar.
- **Preso a um osso** (componente novo): uma espada ou um cajado na mão do personagem, que acompanha a animação. No NN Studio, a arma já aparece na mão na vista 3D, e a lista mostra os ossos do modelo do pai (vem com a mão direita).
- **Vida:** números do dano subindo (**Mostrar o dano**), o modelo fica vermelho um instante a cada dano e a câmera pode tremer (**Treme a câmera ao levar dano**). A ação **Tremer a câmera** também funciona no 3D.
- **Eventos:** **Acertar** e **Derrotar** (com o alvo); no **Levar dano**, o `@outro` é quem bateu.
- **Modelos:** o herói ganha os ossos das mãos (`mao_e`, `mao_d`) e as animações `golpe` e `atirar`; o besouro, a mordida (`golpe`). Novos: `espada.glb` e `cajado.glb`, nos modelos de projeto 3D.
- **Vale 3D:** o herói vem com a espada (clique para golpear); os besouros têm 2 de vida, com barra e números, e mordem; derrotar um dá 3 moedas. Traz também o `magia.prefab` (uma bola com Projétil, para um Atirar).
- **No Lua:** `scene:hit(alvo, dano, quem, opts)`, `scene:foes(quem)`, `scene:damage_number(obj, n)`, `scene:shake(forca, tempo)`, `anim:bones()`, o componente `bone` e a opção `shoulder` da câmera em terceira pessoa. A Vida recebe `dano(n, quem)` e manda `derrotou` para quem bateu por último.

### Chefes e ondas

- **Ondas de inimigos** (comportamento novo, 3D): cria um prefab de inimigo em ondas (onda 1, 2, 3...), cada uma com mais, com pausa entre elas. Os filhos do objeto são os pontos de onde eles saem (sem filhos, em volta, no chão onde dá para andar e longe do herói). Começa quando o jogador chega perto, com a cena ou por mensagem; mostra um letreiro ("Onda 2") e guarda a onda e quantos faltam em variáveis, para o placar. **➕ Objeto > Ondas de inimigos** já vem com quatro pontos.
- **Chefe** (comportamento novo, 3D): golpe, investida (fica tonto ao bater numa parede), pulo com impacto, tiros em leque e chamar ajudantes, cada um com um aviso antes. Fica furioso com pouca vida, é pesado, mostra o nome e a vida no alto da tela e pode deixar um prêmio ao cair.
- **Gerador no 3D:** cria num lugar do chão até **Espalhar até** metros, longe do jogador.
- **Eventos:** **Começar ou terminar uma onda** e **Vencer todas as ondas**.
- **Modelo novo:** o **besouro-rei** (`besourao.glb`), com parado, andar, golpe, preparar, pular, tonto e cair, nos modelos de projeto 3D.
- **A ilha (modelo 3D) ganhou uma arena** depois da ponte: 3 ondas de besouros e o besouro-rei, que guarda a sexta estrela. O herói da ilha também ganhou a espada.
- **Tutorial guiado novo: Um chefe** (15 minutos).
- **Vida:** a variável padrão ("vidas") só vale no jogador. Antes, um inimigo com Vida e a variável padrão dividia as vidas com o herói.
- **No Lua:** `scene:ground_spot(...)`, `scene:in_front(...)`, `scene:front3(obj)` e `scene:warn(texto)`.

## 0.2.2

Correção da 0.2.1.

- **Nas cenas 3D, as cópias de um prefab iam todas para z = 0** no jogo (no NN Studio elas apareciam no lugar certo). As moedas e as estrelas dos modelos **3D** e **Vale 3D**, por exemplo, ficavam enfileiradas longe da trilha e dos morros. Agora cada cópia fica onde está na cena: valem o Z, a rotação 3D (X, Y e Z) e a escala em Z dela. O mesmo para os prefabs criados pelo jogo com `scene:spawn(prefab, x, y, { z = ... })`: o **Atirar** no 3D soltava o tiro em z = 0 quando quem atirava estava em outro lugar.

## 0.2.1

Correção da 0.2.0.

- **Um clique do mouse numa cena 3D com eventos dava erro** (`nn/cena.lua: ... campo 'width'`) quando a cena tinha algum Corpo 3D. Isso acontecia já no primeiro clique na janela do jogo (o que prende o mouse), por exemplo no tutorial *Uma conversa* e nos modelos 3D. Agora não dá mais erro, e **Clique no objeto** funciona no 3D: vale o objeto que o mouse aponta, o mais perto da câmera (e os pais dele). O mesmo para o **Ctrl+clique** na janela do jogo, que seleciona o objeto no NN Studio.
- `objeto:bounds()` numa cena 3D devolve a metade da caixa 3D do objeto (em X e Y).

## 0.2.0

A versão do **3D**: jogos 3D em terceira pessoa, com visual low-poly colorido, montados no **NN Studio sem código** ou escritos em Lua. Cenas 3D com céu, sol, névoa e sombras; física; personagens animados do Blender e do Mixamo; terrenos pintados com o mouse; som e partículas no mundo; inimigos; portas, chaves e zonas; conversas com um editor visual; e seis tutoriais guiados novos. Para começar: o guia [Comece aqui no 3D](docs/3d.md).

### Para quem vem da 0.1

- **Os projetos da 0.1 abrem e rodam sem mudar nada.** Os seis modelos da 0.1 (Em código, Plataforma, Aventura, Nave, Vazio e Mínimo), criados com o `nn` da 0.1, rodam todas as cenas na 0.2, e o NN Studio grava as cenas e os prefabs deles exatamente como estavam. As preferências do editor também continuam.
- Nenhuma função da API Lua nem comportamento pronto foi tirado ou mudou de nome.
- **Para quem compila do código:** a engine passa a pedir o **Rust 1.89** (antes 1.88), porque a física 3D usa o `nalgebra` 0.35, que precisa dele. O NN Studio continua pedindo o 1.95. Com o rustup, `rustup update` resolve. Quem só baixa o NN Studio pronto não precisa de nada disso.
- Jogos exportados na 0.1 continuam funcionando como estão; para usar o que é novo, exporte de novo com a 0.2.

### 3D pelo código

- **Mundo 3D na engine**, desenhado antes do 2D: o HUD, os sprites e os menus ficam por cima.
- **Formas prontas**: cubo, plano, esfera, cilindro, cone e cápsula, e malhas montadas ponto a ponto.
- **Visual low-poly**: luz do sol, luz ambiente do céu e do chão, uma cor por face e névoa.
- **API Lua `nn.d3`**: desenhar com posição, rotação e escala; câmera que olha para um ponto ou gira em volta dele; achar onde o mouse toca o chão; pôr textos 2D em cima de coisas 3D.
- **Modelos `.glb` e `.gltf`** (do Blender e outros) com `nn.assets.model`: partes, materiais, cores e texturas; recarregam quando o arquivo muda.
- **Sombras do sol**, ligadas por padrão, com qualidade, distância e `shadow = false` para quem não deve projetar.
- **Céu em degradê** com o disco do sol; a névoa usa a cor do horizonte.
- **Exemplo `cubos3d`** com árvores, casas, pedras, caixotes e herói em `.glb` (modelos nossos, gerados por `ferramentas/gerar_modelos.py`). Também entre os exemplos do NN Studio.
- No `cubos3d`, o herói pula e empurra caixotes, os cubos caem e empilham onde você clica (até no telhado) e as casas e árvores bloqueiam.
- No NN Studio, modelos 3D aparecem com ícone próprio e "Copiar nn.assets.model".

### 3D no NN Studio

- **Cena 3D**: em **Nova cena visual**, escolha **Tipo: 3D**. É o mesmo `.cena`, com `mode = "3d"`, e já vem com câmera, chão e um cubo.
- **Vista 3D** desenhada pela engine, com céu, sol, névoa, sombras e uma grade no chão com os eixos X (vermelho) e Z (azul).
- **Navegação como na Unity e no Godot**: botão direito + WASD voa (Q/E descem e sobem, a rodinha muda a velocidade), Alt + arrastar gira em volta, o botão do meio arrasta e F enquadra.
- **Objetos 3D**: formas prontas, modelos `.glb` arrastados do painel Arquivos (caem em pé onde o mouse aponta) e a câmera do jogo, vista como um cone.
- **Alças 3D** para mover (por eixo ou no chão), girar e escalar, com encaixe na grade (Ctrl troca), e clique para selecionar.
- **Inspetor** com posição, rotação e escala em X, Y e Z e os componentes Modelo 3D, Forma 3D e Câmera 3D. Hierarquia, desfazer e refazer, duplicar e prefabs funcionam como no 2D.
- **☀ Ambiente**: céu, altura e lado do sol, luz ambiente, névoa e sombras, mudando na hora na vista.
- **Testar daqui (F6)** roda a cena 3D. No Lua, `nn.d3.matrix` e `obj:matrix3d()` para posições presas a outras.

### 3D sem código

- **Corpo 3D** (`body3d`): parado (chão, paredes, cenário), cai e é empurrado (caixas) ou se move sozinho (plataformas). A forma automática usa a malha exata do modelo no que fica parado e uma caixa, esfera, cápsula ou cilindro do tamanho do objeto no resto. **Colisões**, na barra da cena, mostra os corpos.
- **Jogador 3D**: anda para onde a câmera olha, pula (com pulo duplo opcional), vira para onde anda e leva a câmera em terceira pessoa, com o mouse preso (Esc solta). Caiu do mundo: volta ao último chão firme e recebe a mensagem `caiu`.
- **Comportamentos no 3D**: Girar e flutuar, Plataforma que anda (leva o herói junto), e Coletável, Ir para cena, Checkpoint, Mostrar valor e Botão, que agora funcionam nas duas. O Inspetor só oferece o que funciona no tipo da cena.
- **Eventos no 3D**: encostar e perto em metros, Levar para e Velocidade com Z, Criar prefab no ponto 3D, Mudar cor em modelos e formas. O que é só do 2D some dos menus de uma cena 3D.
- **Ao vivo no 3D**: posição, rotação, escala, componentes e o Ambiente mudam no jogo aberto na hora.
- **Tutorial guiado "Seu primeiro mundo 3D"** e os modelos **3D** (a ilha das estrelas sem código) e **Vazio 3D** (`nn novo --modelo visual3d` e `--modelo vazio3d`). O modelo 3D em código passa a se chamar "3D em código" no NN Studio.
- No Lua: `obj:position3d()`, `obj:bounds3d()`, `obj:move_to(x, y, z)`, `self:world3()`, `m:inverse()` e `m:decompose()` nas matrizes, `modelo:bounds()` e `sx`, `sy`, `sz` nos corpos de `nn.physics3d`. Comportamentos podem declarar `mode = "2d"`, `"3d"` ou `"both"`.
- O sol padrão de uma cena 3D vem de lado (`sun_turn = 120`), para as sombras aparecerem.

### Animação de esqueleto

- **Modelos animados**: `.glb` com ossos e animações (do Blender e do Mixamo) se deformam na GPU, com sombra. Também valem as animações de partes sem ossos (uma roda que gira, uma porta que abre). Até 128 ossos por modelo.
- **No Lua**: `modelo:animations()` e `modelo:animator()`, com `play(nome, { loop, blend, speed, restart })`, troca suave entre animações, `update`, `finished`, `bone` (para prender coisas num osso) e `nn.d3.draw(modelo, { animator = anim })`.
- **Herói e bicho animados nossos** (gerados por `ferramentas/gerar_modelos.py`): o herói tem parado, andar, correr, pular, cair e acenar; o bicho, parado e andar. Os exemplos 3D e os modelos de projeto 3D usam o herói animado.
- **Componente Animação 3D** no NN Studio: escolhe a animação do modelo (o ☰ lista as do arquivo), com velocidade e repetir, e a vista do editor toca a animação.
- O **Jogador 3D** anima o modelo sozinho (parado, andar, correr, pular e cair), com os nomes configuráveis.
- Eventos **Tocar animação** e **Animação terminar** também no 3D; no código, `obj:play(nome)` e a mensagem `animacao_terminou`.
- O tutorial **Seu primeiro mundo 3D** ganhou um bicho que anda.

### Som 3D e partículas 3D

- **Som 3D**: um som com posição (`nn.audio.play(som, { x, y, z, distance })`) fica mais baixo longe da câmera e vai para o lado em que está; o volume e o lado acompanham a câmera a cada quadro, e `som:set_position` leva o som junto. No NN Studio, o componente **Som** de uma cena 3D tem **Som no espaço 3D** (ligado) e **Ouvido até**, e o som anda com o objeto. **Tocar som** nos eventos ganha **saindo de** um objeto.
- **Partículas 3D**: `nn.particles3d`, em metros, com cone de direção, gravidade 3D e áreas (círculo deitado, esfera, caixa). Cada partícula é um quadrado virado para a câmera, na frente e atrás dos objetos certos e com névoa. As de somar luz vão juntas na GPU.
- No NN Studio, o componente **Partículas** funciona nas cenas 3D: oito efeitos prontos do 3D (fogo, fumaça, faíscas, explosão, poeira, brilho, chuva, neve), campos em metros, as partículas saindo para o "cima" do objeto, e a prévia animando na vista 3D (com o 💥 Soltar).
- Eventos **Soltar partículas**, **Ligar ou desligar partículas**, **Faíscas** e **Partículas terminarem** no 3D; o **Coletável** solta um brilho 3D.
- A ilha do modelo **3D** ganhou uma fogueira (fogo, fumaça e o estalar), o barulho das ondas nas praias e poeira nos pés do herói ao pular. Sons novos `fogueira.wav` e `mar.wav` nos modelos 3D (gerados por `ferramentas/gerar_ilha3d.py`, que também gera as cenas da ilha).
- Tutorial guiado **Efeitos no 3D**.

### Terreno

- **Componente Terreno**: uma grade de alturas (até 256 × 256 quadrados) com quatro cores pintadas, desenhada low-poly e com o chão exato da física (os mesmos triângulos). A **água** do terreno tem nível e cor, é meio transparente, e o **Jogador 3D** que cai nela recebe a mensagem `agua` e volta para a margem (ou anda no fundo, se escolher).
- **✏ Pintar terreno** no NN Studio: pincéis Subir, Descer, Alisar, Aplainar, Ruído, Pintar cor e **Espalhar** (modelos ou prefabs com tamanho e giro sorteados, já com Corpo 3D; Ctrl apaga). Tamanho e força, um círculo mostrando o pincel no chão, cada arrasto um passo do Ctrl+Z, e o jogo aberto muda na hora.
- No Lua: `nn.d3.terrain` com `height_at`, `brush`, `paint`, `raycast`, `encode`; `world:body({ terrain = t })`; `self:water_level(x, z)`. Malhas meio transparentes são desenhadas depois das outras, de trás para a frente.
- Modelo **Vale 3D** (`nn novo --modelo vale3d`): morros, lago, trilha, árvores espalhadas, estrelas nos morros e uma fogueira, gerado por `ferramentas/gerar_vale3d.py`.
- Tutorial guiado **Pinte um terreno**.

### Inimigos, vida e dano no 3D

- **Inimigo 3D** (`nn:inimigo3d`): patrulha (indo e voltando), vaga ou fica parado; persegue o jogador quando ele chega perto e desiste quando ele foge; machuca e empurra quem encosta; morre se pisarem em cima (com Vida, perde uma vida por pisão); anima o modelo (parado e andar) e troca de rumo quando fica preso.
- **Vida**, **Perigo**, **Atirar** e **Destruir depois** funcionam no 3D. A Vida ganhou a **barra de vida em cima** (2D e 3D). O Atirar 3D sai da frente do objeto (ou para onde a câmera olha), na altura escolhida, e o tiro some sozinho.
- Propriedades de comportamentos podem valer só no 2D ou no 3D (`mode`) e ter outro padrão no 3D (`default3d`); o Inspetor mostra só as que valem na cena.
- A ilha e o vale (modelos 3D) ganharam besouros, vida para o herói e corações no HUD.
- Tutorial guiado **Inimigos no 3D**.

### Portas, chaves e zonas

- **Porta 3D**: abre girando na dobradiça (para longe de quem abre), subindo, descendo ou deslizando; com **E** perto dela, sozinha quando o jogador chega ou só por mensagem. Fechada, bloqueia a passagem. Pode pedir uma chave e fechar sozinha depois de um tempo.
- **Inventário**: o **Coletável** guarda itens pelo nome (*chave*, *poção*), com um ícone; **Mostrar inventário** põe os itens num canto da tela. No Lua, `nn.inventory` (`add`, `remove`, `count`, `has`, `items`), que passa entre as cenas como as variáveis do jogo.
- **Interagir**: perto de um objeto aparece **E usar** (o texto você escolhe), e só no mais perto; apertando, ele avisa os eventos e outros objetos. Pode pedir um item. A Porta 3D usa o mesmo aviso.
- **Zona de gatilho**: uma caixa que aparece no editor e some no jogo; avisa quando o jogador entra ou sai (abre e fecha portas, liga armadilhas).
- **Texto 3D** (componente e `nn.d3.text`): textos no mundo, virados para a câmera ou presos como placas, com contorno e fundo, atrás das paredes e com névoa. O **Mostrar valor** escreve neles também.
- **Eventos novos**: Entrar numa zona, Sair de uma zona, Usar (apertar E), Ter um item, Ganhar item e Gastar item.
- **➕ Objeto > Portas e itens** no NN Studio (porta, chave, zona, Texto 3D, inventário), os modelos `porta.glb` e `chave.glb`, um cofre trancado com a chave numa plataforma no modelo **3D** (a ilha sem código), uma placa no **Vale 3D** e o tutorial guiado **Portas e chaves**.

### Diálogos

- **Editor de conversa** no NN Studio (arquivos `.dialogo`, em **➕ Novo > 💬 Diálogo**): cada fala é um cartão com quem fala e o texto; arrastar da bolinha de uma fala liga com outra (soltar no vazio cria uma fala nova); escolhas do jogador, cada uma levando a uma fala; personagens com cor, retrato e voz; velocidade do texto; desfazer e salvar.
- **Diálogo** (`nn:dialogo`, 2D e 3D): um personagem que conversa, com o aviso **E falar** (ou começando quando o jogador chega, quando a cena começa ou por mensagem); no 3D, vira para quem fala com ele. **➕ Objeto > Portas e itens > Personagem que fala** e o botão **💬 Editar a conversa** no Inspetor.
- **Caixa de fala** com o tema do projeto: o nome numa etiqueta da cor do personagem, o retrato, o texto letra por letra (com `{variavel}`) e as escolhas (setas, números ou mouse). Durante a conversa, o herói fica parado.
- Falas e escolhas podem mudar variáveis, dar itens e pedir um item para aparecer.
- **Eventos:** Começar uma conversa, Terminar uma conversa, Escolher numa conversa e a ação Começar conversa. No Lua: `scene:start_dialogue`, `scene:stop_dialogue` e `scene:dialogue`.
- Um pescador que dá dicas no modelo **3D**, um velho que conta onde está a chave no **Aventura** e o tutorial guiado **Uma conversa**.

### Desempenho e robustez do 3D

- **Só vai para a GPU o que a câmera vê**: o que fica fora da tela é cortado antes de desenhar, e o mapa de sombras só recebe o que fica perto da câmera. Numa cena com mil árvores, menos da metade é desenhada e um sexto vai para as sombras.
- **`nn.d3.stats()`** diz quantas malhas foram pedidas, vistas, postas na sombra e quantas chamadas de desenho houve.
- **Terrenos** não refazem mais a malha a cada quadro (só quando mudam), no jogo e no NN Studio.
- Menos trabalho por objeto a cada quadro: a matriz de quem está parado é reaproveitada e a física 3D não cria tabelas novas.
- **Arquivos que faltam não param o jogo:** um modelo que falta ou não abre vira um cubo rosa, um som fica mudo e uma conversa não abre, cada um com um aviso claro no console (o objeto, o componente e o arquivo).
- Testes de desempenho (`cargo test --release -p nn-run --test desempenho -- --ignored --nocapture`) e, no CI, testes que conferem o que vai para a GPU nas cenas 3D dos modelos e numa cena cheia.

### Revisão e acabamento

- **HUD na vista 3D:** o que fica preso à tela (placar, corações, avisos, menus) aparece por cima da cena, na moldura da tela do jogo, e dá para clicar e arrastar. **HUD**, na barra, mostra ou esconde. Antes, ele não aparecia, e o **Enquadrar** contava as posições dele (em pixels) como metros: a Ilha e o Vale abriam com a cena minúscula, lá longe.
- **➕ Objeto > Na tela (HUD)** na cena 3D: texto na tela e os componentes de interface. O **➕** da Hierarquia, numa cena 3D, mostra o mesmo menu da vista.
- A vista 3D **abre olhando para o Jogador 3D** de perto. De longe, a **Névoa** da vista desliga sozinha (marque de novo na barra; no jogo, a névoa continua). **Colisões** começa desligado no 3D.
- **Gerenciador:** as boas-vindas têm o caminho **3D** (o tutorial *Seu primeiro mundo 3D*, a ilha pronta e o Vazio 3D); os **tutoriais** ficam em abas **Jogos 2D** e **Jogos 3D**, com rolagem (antes, a lista passava da tela); no **Novo projeto**, os modelos ficam em dois grupos, 2D e 3D. As janelas não crescem mais aos poucos ao abrir.
- Barras das cenas e do pincel do terreno quebram a linha certinho (antes, o último controle ficava cortado na beirada). O editor de conversa enquadra conversas maiores, e a dica da barra quebra a linha.
- Partículas na prévia do editor não são mais refeitas a cada quadro (o editor comparava um texto montado com todos os campos).
- Tutorial *Inimigos no 3D*: os corações vêm de **➕ Objeto > Na tela (HUD) > Ícones**.
- **Documentação:**
  - guia novo **[Comece aqui no 3D](docs/3d.md)**: os primeiros 5 minutos, a ordem dos tutoriais, as peças prontas, modelos do Blender e do Mixamo, desempenho e o que fazer quando algo dá errado;
  - `studio.md`: *HUD e menus no 3D* e *O que vem depois*;
  - README com capturas do 3D;
  - os LEIAME dos modelos 3D falam dos besouros, da vida e do HUD.

## 0.1.0

A primeira versão para baixar e instalar: o **NN Studio** (o editor), o `nn` (linha de comando) e o `nn-jogo` (o executável dos jogos exportados), para **Windows** e **Linux**.

### NN Studio, o editor

- **Gerenciador de projetos** com boas-vindas na primeira vez (fazer o tutorial, abrir um jogo pronto ou começar do zero), projetos recentes, exemplos e seis modelos: Em código, Plataforma, Aventura, Nave, Vazio e Mínimo.
- **Cenas visuais** (`.cena`) montadas com o mouse:
  - arrastar imagens, mapas e sons para a cena;
  - mover, girar e escalar com as alças;
  - Hierarquia com filhos e Inspetor com todos os componentes;
  - desfazer e refazer.
- **Comportamentos prontos**, sem código: jogador de plataforma e top-down, vida, patrulha, perseguir, perigo, coletável, checkpoint, ir para cena, movimento automático, animação automática, atirar, gerador e mostrar valor.
- **Prefabs**: um objeto pronto para usar em várias cenas. Mudar o prefab muda todas as cópias.
- **Eventos "Quando... Então..."**, lidos como frases. **Ver como código** mostra o Lua equivalente, e cada regra acende quando dispara no jogo aberto.
- **Testar daqui (F6) e ajuste ao vivo**:
  - o que muda no editor aparece no jogo aberto;
  - o editor mostra onde cada objeto está no jogo, pausa e avança quadro a quadro;
  - Ctrl+clique no jogo seleciona o objeto.
- **Editor de pixel art** (lápis, borracha, balde, formas, seleção, espelhado, paletas, quadros, papel vegetal) e **editor de animação** (`.anim`), com prévia.
- **Mapas de tiles** pintados direto na cena:
  - pincel, balde, retângulo e conta-gotas;
  - tiles girados e espelhados, camadas;
  - terrenos que se encaixam sozinhos;
  - painel Tiles para marcar o que bloqueia, plataformas, tipos e animações;
  - arquivos do Tiled (`.tmj`/`.tsj`).
- **Menus e HUD**:
  - componentes de interface: painel, botão, barra, ícones, deslizador, caixa de marcar, campo de texto e grupo que arruma os filhos;
  - âncoras nos cantos da tela e tema do projeto (cores, fonte `.ttf`, som);
  - cenas que abrem por cima de outra (pausa, opções), com a de baixo parada.
- **Partículas** animando na própria cena:
  - dez efeitos prontos e uma seta para a direção;
  - explosões em prefab que estouram e somem sozinhas;
  - ações para soltar e ligar ou desligar.
- **Editor de código Lua**: cores, erros enquanto digita, autocompletar da API `nn.*` com descrições, buscar e substituir.
- **Seis tutoriais guiados**, que destacam cada botão e avançam sozinhos: *Seu primeiro jogo*, *Eventos*, *Desenhe seu herói*, *Monte uma fase com tiles*, *Monte um menu* e *Efeitos com partículas*.
- **Configurações do projeto** num formulário: tamanho da janela e do jogo, variáveis do jogo e tema da interface.
- **Exportar o jogo** num executável único, para Windows ou Linux.
- **Modo Simples e Completo**, tema escuro e claro, paleta de comandos (Ctrl+Shift+P).
- **Log** em `nn-studio.log` e aviso quando o editor fechou por erro.
- **Aviso de versão nova** (pode ser desligado).

### Engine

- Renderização 2D com wgpu: sprites em lote, spritesheets, texto, painéis, partículas, fundos em camadas, câmera que segue, tremor e efeitos de tela.
- Lua 5.4 com a API `nn.*`: cenas (trocar, abrir por cima, transições), entrada (teclado, mouse, controle, ações), áudio com canais, física arcade, tilemaps, interface, progresso salvo.
- Mensagens de erro do Lua em português.
- Tamanho fixo do jogo, que se ajusta a qualquer janela (ajustar, esticar ou pixel art).
- Jogos também podem ser escritos em Rust, com os crates da engine.

### Para quem vai instalar

- **Windows:** instalador sem pedir administrador ou versão portátil em `.zip`. O instalador ainda não tem assinatura digital: o Windows avisa na primeira vez (veja o [README](README.md#baixar)).
- **Linux:** AppImage ou `.tar.gz`.
