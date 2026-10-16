# NN Studio

O NN Studio é o editor da nn-engine: um programa com janela para criar, editar,
testar e exportar jogos sem usar o terminal. Ele trabalha com os mesmos
projetos em Lua da ferramenta `nn`, então dá para alternar entre os dois.

![NN Studio com o jogo de plataforma aberto](imagens/nn-studio.png)

## Abrindo

- **Versão pronta:** baixe o pacote do seu sistema na página de *releases*
  (`nn-windows.zip` ou `nn-linux.tar.gz`) e abra o `nn-studio`. Deixe o
  `nn-jogo` na mesma pasta: ele é usado para exportar os jogos.
- **A partir do código:** `cargo studio` (precisa do Rust 1.95 ou mais novo).
  Para abrir direto um projeto: `cargo studio jogos/plataforma`.

## Gerenciador de projetos

É a primeira tela.

- **Projetos recentes:** clique para abrir. O ✖ só tira da lista (não apaga
  nada). Pastas que sumiram aparecem como "pasta não encontrada".
- **✨ Novo projeto:** escolha o nome do jogo, onde criar e o modelo (cada
  cartão mostra uma imagem do jogo que ele cria; clique duas vezes para criar
  na hora):
  - **Em código:** um jogo pequeno e completo escrito em Lua (título com
    menu, partida, pausa, fim e recorde salvo). O melhor jeito de começar pelo
    código.
  - **Plataforma:** uma fase montada no editor, **sem nenhuma linha de
    código**: tela de título, herói, plataformas (uma delas móvel), moedas (um
    prefab), inimigo, abismo, placar e uma saída que só abre depois de pegar
    todas as moedas (um evento).
  - **Aventura:** visto de cima, sem código: pegue a chave, abra a porta e ache
    o tesouro, fugindo das lesmas e dos morcegos. A porta e o baú funcionam
    com eventos.
  - **Nave:** sem código: tiros, asteroides que vêm de um gerador, pontos,
    vidas e recorde salvo no computador.
  - **Vazio:** uma cena vazia e um pacote de imagens e sons, para montar do
    seu jeito (o tutorial começa daqui).
  - **Mínimo:** só uma cena com um quadrado que anda.

  ![Novo projeto: os modelos](imagens/nn-studio-modelos.png)
- **🎓 Tutoriais:** veja [Tutoriais](#tutoriais). Quem abre o NN Studio pela
  primeira vez já vê o convite para o tutorial *Seu primeiro jogo*.
- **📂 Abrir projeto:** escolha a pasta do jogo (a que tem o `config.lua` ou o
  `main.lua`). Também dá para **arrastar a pasta** para a janela.
- **🎮 Exemplos:** abre uma cópia do jogo de plataforma ou da demonstração,
  para mexer à vontade sem estragar o original.

## Tutoriais

Um tutorial cria um projeto novo (na pasta de projetos) e abre uma janelinha
que mostra um passo de cada vez. O NN Studio **destaca** o painel ou botão de
cada passo e **avança sozinho** quando você faz o que o passo pede: ele confere
o projeto (se o chão tem corpo físico, se o herói tem o Jogador de plataforma,
se a regra existe...). Dá para voltar, pular e sair a qualquer momento; o
projeto continua aberto.

![Tutorial "Seu primeiro jogo" destacando o Adicionar componente](imagens/nn-studio-tutorial.png)

- **Seu primeiro jogo** (uns 15 minutos), a partir do modelo Vazio: chão com
  corpo físico, herói que anda e pula, animação, moedas (e um prefab delas),
  placar no HUD e um evento para vencer.
- **Eventos** (uns 10 minutos), com o modelo Aventura: ler as regras da porta,
  testar e criar regras suas com *Quando* e *Enquanto*.
- **Desenhe seu herói** (uns 10 minutos), a partir do modelo Vazio: desenhar
  um personagem no [editor de pixel art](#editor-de-pixel-art), fazer um
  segundo quadro, animar e pôr na cena.
- **Monte uma fase com tiles** (uns 12 minutos), a partir do modelo Vazio:
  pintar o chão com um terreno que se encaixa sozinho, uma plataforma, uma
  camada de enfeites, lava animada marcada com um tipo e o evento que faz a
  lava queimar ([mapas de tiles](#mapas-de-tiles)).
- **Monte um menu** (uns 10 minutos), a partir do modelo Vazio: corações no
  HUD presos no canto da tela, uma cena de pausa que abre por cima do jogo
  com Esc, com botões e um volume ([interface](#interface-menus-e-hud)).
- **Efeitos com partículas** (uns 6 minutos), a partir do modelo Vazio: fogo
  e explosão prontos, a direção na seta, a explosão num prefab e um evento
  que solta ela com uma tecla ([partículas](#partículas)).

Eles ficam no Início (**🎓 Tutoriais**) e em **Ajuda > Tutoriais**.

## A janela do projeto

Os painéis podem ser arrastados pelas abas e encaixados onde você quiser. O
layout e as abas abertas ficam guardados para cada projeto; para voltar ao
original, use **Exibir > Restaurar layout padrão**.

- **Arquivos** (à esquerda, em cima): a pasta do projeto.
- **Hierarquia** (à esquerda, embaixo): os objetos da cena visual aberta.
- **Eventos** (embaixo, ao lado do Console): as regras "Quando... Então..."
  da cena aberta (**Exibir > Mostrar Eventos**).
- **Início, cenas, código, imagens, sons e mapas** (no centro).
- **Inspetor** (à direita): as propriedades do objeto selecionado.
- **Console** (embaixo): mensagens do jogo e do editor.

### Modo Simples e modo Completo

No alto, à direita. O **Simples** mostra só o essencial; o **Completo** mostra
todas as opções (mais configurações do projeto, detalhes internos da engine no
console, pastas de saída como `dist`, abrir no editor externo...).

### Paleta de comandos

**Ctrl+Shift+P** abre uma busca com todos os comandos e arquivos: digite parte
do nome ("export", "config", "tema", "fase1") e aperte Enter. **Ctrl+P** busca
só arquivos. A busca ignora acentos e aceita letras soltas ("cfg" acha
"Configurações").

## Arquivos

- **Clique** num arquivo para abrir. Imagens mostram uma miniatura na lista;
  sons têm um ▶ para ouvir.
- **➕ Novo:** cena visual (montada com o mouse, veja abaixo), cena em código
  (já vem com um quadrado que anda), **imagem** (para desenhar no
  [editor de pixel art](#editor-de-pixel-art): escolha o tamanho e quantos
  quadros), script (módulo para usar com `require`) ou pasta. Cenas novas vão
  para `cenas/`, imagens para `assets/imagens` e scripts para `lib/`, se
  essas pastas existirem.
- **Arraste** uma imagem, um mapa, um som ou um prefab do painel para dentro
  de uma cena visual (vira um objeto) ou para um campo de arquivo do Inspetor.
  Arraste um objeto da Hierarquia para uma pasta para criar um
  [prefab](#prefabs).
- **📥 Importar** ou **arrastar arquivos** do computador para a janela:
  imagens vão para `assets/imagens`, sons para `assets/sons` e fontes para
  `assets/fontes`. Com uma pasta selecionada no painel, vão para ela. Nada é
  sobrescrito: um arquivo com o mesmo nome vira "nome (2).png".
- **Botão direito** num arquivo:
  - **Copiar caminho**: copia `"assets/imagens/heroi.png"`, pronto para colar
    no código;
  - **Copiar require** (em scripts): copia `require("lib.util")`;
  - **Renomear** (F2), **Mandar para a lixeira** (Del), **Mostrar na pasta do
    sistema**, **Abrir no programa padrão**.
- A lista se atualiza sozinha quando algo muda na pasta (por exemplo, ao salvar
  um mapa no Tiled).

## Editor de código

- Cores para o Lua, números de linha, a linha do cursor destacada.
- **Erros de sintaxe enquanto você digita**: a linha com erro fica vermelha e a
  mensagem, em português, aparece embaixo. Clique nela para ir até a linha.
- **Recuo automático:** Enter mantém o recuo e soma um nível depois de `then`,
  `do`, `function(...)`, `{`... Tab e Shift+Tab recuam as linhas selecionadas.
- **Autocompletar:** ao digitar `nn.`, `self.`, `obj.`, `self.obj.`,
  `self.objects.`, `nn.vars.` (e outros), aparece a lista do que existe ali,
  com a descrição de cada item. **Ctrl+Espaço** abre a lista a qualquer
  momento; setas escolhem, **Enter** ou **Tab** completam, **Esc** fecha.
  A lista conhece os objetos das cenas que usam o script, as variáveis do
  jogo e, dentro de aspas, os arquivos do projeto (`"assets/imagens/...`).
- Ao abrir parênteses de uma função (`nn.draw.rect(`), os parâmetros aparecem
  acima do cursor; passar o mouse sobre um nome mostra a descrição.
- **Ctrl+/** comenta ou descomenta as linhas.
- **Ctrl+F** busca (Enter vai para a próxima, Shift+Enter para a anterior;
  "Aa" diferencia maiúsculas). **Ctrl+H** substitui. **Ctrl+G** vai para uma
  linha.
- **Ctrl+S** salva; **Ctrl+Shift+S** salva tudo. Abas com alterações mostram
  "•" no nome. Ao fechar uma aba, o projeto ou o NN Studio com alterações não
  salvas, ele pergunta antes.
- Se o arquivo mudar fora do NN Studio (em outro editor, por exemplo), ele é
  recarregado sozinho; se você também tiver alterações, ele pergunta qual
  versão manter.

## Cenas visuais

Uma cena visual (arquivo `.cena`) é montada com o mouse: você põe objetos,
arruma no lugar, ajusta as propriedades, dá a eles **comportamentos prontos**
(andar e pular, patrulhar, ser coletado, machucar, levar para outra fase...)
e junta tudo com **eventos** ("Quando o jogador encostar na chave, então...").
Sem escrever código. Ela roda na
engine como qualquer outra cena (`nn.scene.switch("cenas/fase.cena")`, cena
inicial no `config.lua`, exportação, recarregar ao salvar). Crie uma com
**Ctrl+N** ou **➕ Novo > Cena visual**.

![Cena visual do modelo Plataforma aberta no NN Studio](imagens/nn-studio-cena.png)

### A aba da cena

A cena é desenhada pela própria engine, do jeito que vai aparecer no jogo. O
retângulo "tela do jogo" mostra o que cabe na janela (o tamanho do
`config.lua`), a partir da posição da câmera.

- **Navegar:** role o mouse para o zoom (no ponto do cursor); arraste com o
  botão do meio, com o direito ou com **Espaço** segurado para mover a vista.
  **F** (ou ⛶ Enquadrar) enquadra o selecionado ou a cena toda.
- **Selecionar:** clique num objeto; **Shift** ou **Ctrl** somam à seleção;
  arraste num espaço vazio para selecionar com um retângulo. **Esc** limpa.
- **Ferramentas:** **W** mover, **E** girar, **R** escalar. Arraste o objeto ou
  as alças: as setas vermelha e verde movem só em X ou em Y; o círculo gira
  (com **Ctrl**, de 15 em 15 graus); os cantos escalam (com **Shift**,
  proporcional).
- **Encaixar** prende na grade ao mover (segurar **Ctrl** também encaixa). As
  **setas do teclado** movem o selecionado 1 unidade (10 com **Shift**).
- **Ctrl+D** duplica, **Del** apaga. **➕ Objeto** cria um objeto vazio, uma
  imagem, um retângulo, um círculo, um texto, uma câmera, um som,
  [partículas](#partículas) (vazias ou um efeito pronto) ou um componente de
  [interface](#interface-menus-e-hud) no meio da vista.
- **Arrastar arquivos** do painel Arquivos para a cena: uma imagem vira um
  sprite, um mapa `.tmj` vira um mapa, um som vira um objeto de som e um
  `.prefab` vira uma instância do prefab.
- **Colisões** mostra os corpos físicos (verde) por cima.

### Hierarquia

A lista dos objetos da cena, na ordem de desenho. Clique para selecionar,
clique duas vezes (ou **F2**) para renomear. O 👁 esconde e o cadeado trava (um
objeto travado não é selecionado clicando na cena). **Arraste** um objeto para
mudar a ordem ou solte em cima de outro para virar filho dele: o filho anda,
gira e escala junto com o pai. O botão direito tem Duplicar, Criar filho
vazio, Tirar do pai, **📦 Criar prefab** e Excluir; o campo 🔍 filtra pelo
nome. Instâncias de prefab aparecem em azul.

### Inspetor

Sem nada selecionado, mostra a cena: o **script** dela, a cor de **fundo**, a
**gravidade** e **Por cima** (a cena abre por cima de outra, como uma pausa:
veja [Interface](#interface-menus-e-hud)). Com um objeto selecionado, mostra o nome, visível/travado, a
**Transformação** (posição, rotação em graus, escala e camada: camada maior é
desenhada por cima) e os **componentes**:

| Componente | O que faz |
|---|---|
| Imagem (sprite) | Mostra uma imagem ou um quadro de uma spritesheet. Com **É uma tira de quadros**, escolha o quadro clicando na grade. **Animação** toca uma das [animações](#animações) da imagem (escolha no ☰), já na cena do editor. **Preencher: Repetir** repete a imagem lado a lado em vez de esticar: com escala 20, um bloco vira 20 blocos (chão, paredes, fundos). **🎨 Editar imagem** abre o [editor de pixel art](#editor-de-pixel-art); sem imagem, **🎨 Desenhar uma imagem nova** cria uma e já põe no objeto. |
| Texto | Escreve um texto no mundo. |
| Forma | Retângulo ou círculo de uma cor. |
| Corpo físico | Cai, colide, para no chão (dinâmico, estático ou cinemático; caixa ou círculo; sensor; plataforma de mão única). |
| Mapa de tiles | Desenha um mapa `.tmj`, com a colisão dos tiles. **✏ Pintar** pinta o mapa ali na cena ([mapas de tiles](#mapas-de-tiles)); **Abrir o mapa** abre a aba dele. |
| Câmera | Onde a câmera começa, ou quem ela segue (com suavidade e limite no mapa). |
| Som | Toca um som ou música quando a cena começa. |
| Partículas | Fogo, fumaça, faíscas, explosões, chuva... animando na cena ([partículas](#partículas)). |
| Interface | Painel, Botão, Barra, Ícones, Deslizador, Caixa de marcar, Campo de texto e Grupo: menus e HUD ([interface](#interface-menus-e-hud)). |
| Comportamento | Um comportamento pronto ou um script seu (veja abaixo). |

**➕ Adicionar componente** acrescenta outro; o menu ☰ de cada um sobe, desce
ou remove. Campos de arquivo aceitam arrastar do painel Arquivos. No modo
Simples, opções avançadas ficam escondidas.

Imagem, texto e forma têm a opção **HUD**: o objeto fica **fixo na tela**
(placar, vidas, botões), em vez de andar com o mundo. Na cena, ele aparece
dentro do retângulo da tela do jogo. A posição conta a partir do ponto da
tela escolhido em **Transformação > Preso em** (o centro, se você não
escolher): preso no canto de cima à esquerda, ele continua no canto em
qualquer tamanho de janela. Os componentes de interface já vêm fixos na tela.

### Comportamentos (sem código)

Um comportamento é uma peça pronta de lógica que você põe num objeto pelo
Inspetor: **➕ Adicionar componente** mostra os comportamentos por categoria.
Cada um tem suas propriedades (velocidade, altura do pulo, quantas vidas...),
com explicação ao passar o mouse.

![Herói do modelo Plataforma com os comportamentos no Inspetor](imagens/nn-studio-comportamentos.png)

| Categoria | Comportamento | O que faz |
|---|---|---|
| Jogador | Jogador de plataforma | Anda para os lados e pula (setas, WASD, espaço ou controle), com tolerância na beirada e pulo mais alto segurando. Pede um corpo físico. |
| Jogador | Jogador top-down | Anda em todas as direções (jogo visto de cima). |
| Jogador | Vida | Pontos de vida. Quando acabam: reinicia a cena, volta ao checkpoint, vai para outra cena ou o objeto some. |
| Inimigos | Patrulha | Anda até bater numa parede (ou chegar na beirada) e volta. |
| Inimigos | Perseguir | Vai atrás do jogador (ou de outro objeto) quando ele chega perto; andando ou voando. |
| Inimigos | Perigo | Espinho, lava, inimigo: tira vida, manda de volta ao checkpoint ou reinicia. Com **Pisar derrota**, pular em cima destrói o inimigo. |
| Itens | Coletável | Some quando o jogador encosta e soma numa variável do jogo (com som e brilho). |
| Fases | Checkpoint | Ao passar, o jogador volta para cá quando perder. |
| Fases | Ir para cena | Porta, saída, bandeira: leva para outra cena (ao encostar ou apertando uma tecla). |
| Movimento | Movimento automático | Gira, flutua ou vai e volta sozinho (plataformas móveis, enfeites). |
| Aparência | Animação automática | Troca a animação quando o objeto está parado, andando, pulando ou caindo. Em cada estado, escolha uma das [animações](#animações) da imagem no ☰ (ou escreva quadros: `1-4`, `5,6`...). |
| Aparência | Mostrar valor | Mostra variáveis do jogo num texto: `Moedas: {moedas}`. |
| Ações | Atirar | Cria um tiro (um prefab) ao apertar uma tecla ou sozinho, de tempos em tempos. |
| Ações | Gerador | Cria objetos (um prefab) de tempos em tempos. |
| Ações | Destruir depois | Some depois de um tempo ou ao sair da tela. |
| Interface | Botão | Para menus: destaca ao passar o mouse (ou com as setas) e, ao clicar ou apertar Enter, vai para uma cena, reinicia ou fecha o jogo (ou deixa um evento decidir). |

"Quem" e "Alvo", quando ficam vazios, querem dizer **o jogador** (o objeto com
um comportamento de jogador). Se um comportamento precisa de algo que o objeto
não tem (um corpo físico, uma imagem), o Inspetor avisa e oferece adicionar; ao
escolher um comportamento que precisa de corpo, ele já vem junto.

**Seus próprios comportamentos:** **➕ Adicionar componente > Script do jogo >
Novo script...** cria `scripts/<nome>.lua` com um modelo comentado e já põe no
objeto. As propriedades declaradas no script (`props`) aparecem no Inspetor do
mesmo jeito que as dos prontos, e cada objeto pode ter valores diferentes. O
formato está no [guia do Lua](lua.md#comportamentos).

### Prefabs

Um prefab (arquivo `.prefab`) é um objeto pronto para usar várias vezes, em
várias cenas: uma moeda, um inimigo, um tiro.

- **Criar:** botão direito no objeto da Hierarquia > **📦 Criar prefab** (vai
  para `prefabs/`), ou arraste o objeto para uma pasta do painel Arquivos.
  O objeto vira uma instância do prefab novo.
- **Usar:** arraste o `.prefab` do painel Arquivos para a cena.
- **Editar:** clique no `.prefab` no painel Arquivos (ou **Abrir prefab** no
  Inspetor). Ele abre numa aba como uma cena; ao salvar, **todas as
  instâncias** mudam, em todas as cenas.
- **Mudar só uma instância:** mude a posição, o nome ou qualquer propriedade
  do objeto principal no Inspetor. O que for diferente do prefab fica em
  **negrito**, com um ↺ para voltar ao valor do prefab. Os filhos do prefab
  seguem sempre o prefab.
- **Desvincular** (no Inspetor) transforma a instância num objeto comum, com
  uma cópia de tudo.
- No jogo, `scene:spawn("prefabs/tiro.prefab", x, y)` cria um na hora (é o que
  Atirar e Gerador fazem).

### Variáveis do jogo e HUD

As **variáveis do jogo** guardam o que vale para o jogo todo: moedas, vidas,
pontos, chaves. Elas são criadas em **Configurações do projeto > Variáveis do
jogo** (ficam no `config.lua`, em `vars`) e:

- passam de uma cena para outra e voltam ao valor inicial quando o jogo
  reinicia (ou quando a Vida acaba e a cena reinicia);
- são usadas pelos comportamentos: Coletável soma, Vida mostra as vidas que
  restam, Mostrar valor escreve no texto;
- no código, ficam em `nn.vars` (`nn.vars.moedas = nn.vars.moedas + 1`).

Para um placar: um objeto de texto com **HUD** marcado e o comportamento
**Mostrar valor** com `Moedas: {moedas}  Vidas: {vidas}`. Para as vidas, o
componente **Ícones** com a variável `vidas` mostra um coração para cada uma;
a **Barra** mostra uma variável enchendo (vida, energia, tempo). Veja
[Interface](#interface-menus-e-hud).

### Interface: menus e HUD

![HUD do modelo Plataforma: corações (Ícones) presos no canto de cima à esquerda](imagens/nn-studio-interface.png)

**➕ Objeto > Interface** (e **➕ Adicionar componente > Interface**) traz os
componentes de menu e HUD:

| Componente | O que faz |
|---|---|
| Painel | Fundo de menu ou de HUD: cor, cantos arredondados e borda, ou uma **imagem** de moldura em 9 fatias (os cantos não esticam, o meio estica). |
| Botão | Texto (e ícone) num painel que muda de cor com o foco e ao apertar. **Ao apertar**: trocar de cena, abrir uma cena **por cima**, fechar esta cena, reiniciar ou sair do jogo; para outras coisas, a condição **Clique no objeto** nos eventos. **Começa com foco** escolhe o primeiro botão para quem joga com teclado ou controle. |
| Barra | Mostra uma variável do jogo enchendo, até um máximo (ou outra variável, como a vida máxima). Enche para a direita, a esquerda, cima ou baixo. |
| Ícones | Uma imagem para cada ponto de uma variável: corações para as vidas. Com **Máximo**, os que faltam aparecem apagados. |
| Deslizador | Escolhe um valor arrastando ou com as setas: uma variável, ou o **volume** da música, dos efeitos ou geral. |
| Caixa de marcar | Liga e desliga uma variável, ou a **tela cheia**. |
| Campo de texto | Para digitar (o nome do jogador): o texto vai para uma variável. |
| Grupo (arrumar) | Arruma os filhos sozinho, em coluna, em linha ou em grade, com um espaço entre eles. Opcional: sem ele, cada objeto fica onde você puser. |

**➕ Objeto > Interface > Menu com botões** faz um menu pronto: um painel com
um Grupo e três botões (Jogar, Opções e Sair).

Os menus funcionam **sem código** com o mouse, com as setas (ou WASD) e Enter
ou espaço, e com o controle (direcional e A). O foco anda para o item mais
perto na direção apertada; Tab vai para o próximo. Num deslizador com foco, a
esquerda e a direita mudam o valor. Os volumes e a tela cheia escolhidos ficam
guardados e valem da próxima vez que o jogo abrir. Quando um valor muda, a
condição **Mudar de valor** dispara nos eventos.

**Cenas por cima.** Uma cena com **Por cima** marcado (no Inspetor da cena, ou
**Abre por cima de outra cena** ao criar uma cena visual nova) abre por cima
da que está rodando: a de baixo continua aparecendo, **parada**, e o fundo da
cena por cima vira um véu (use uma cor transparente). Abra com a ação
**Abrir cena por cima** (por exemplo, quando apertar a tecla *voltar*, que é
Esc ou o B do controle) ou com um botão; feche com **Fechar esta cena** ou um
botão **Fechar**. Uma cena por cima pode abrir outra (as opções por cima da
pausa).

**Tema da interface.** As cores vazias dos componentes (marcadas **do tema**
no Inspetor) vêm do tema do projeto, em **Configurações do projeto > Tema da
interface**: texto, botão (normal, com foco, apertado e desligado), painel,
bordas e destaque, além da **fonte** (um `.ttf` ou `.otf` do projeto) e do
**som** dos botões. Mude o tema e todos os menus mudam juntos; escolher uma
cor no Inspetor muda só aquele componente (**↺ tema** volta). O componente
Texto também tem **Fonte**.

**Telas de outros formatos.** O **📐** na barra da cena mostra a tela do jogo
em outras proporções (16:9, 4:3, 21:9, celular em pé), para conferir se o HUD
preso nos cantos fica no lugar.

### Eventos: "Quando... Então..."

Os comportamentos fazem cada objeto funcionar; os **eventos** são as regras do
seu jogo: *quando* o jogador pegar todas as moedas, *então* a saída abre;
*quando* a vida acabar, *então* guarde o recorde e vá para a tela de fim. Ficam
no painel **Eventos** (embaixo, ao lado do Console), uma regra por cartão,
lidas como frases, com os campos no meio da frase.

![Eventos da porta no modelo Aventura](imagens/nn-studio-eventos.png)

- **➕ Regra da cena** cria uma regra que vale para a cena toda. Com um objeto
  selecionado, **➕ Regra de <objeto>** cria uma regra **do objeto**: ela vai
  junto com ele (e com todas as cópias, se ele for um prefab). Na regra de um
  objeto, *este objeto* é ele mesmo.
- O menu de cada **➕** tem as condições ou ações por categoria e uma busca
  (digite e aperte Enter para a primeira).
- Uma regra pode ter várias condições (**e**: todas precisam valer) e várias
  ações (feitas em ordem). O **não** no fim da linha inverte uma condição.
- **Quando** acontece uma vez, no momento em que as condições passam a valer
  (ou a cada vez que um gatilho, como "apertar uma tecla", acontece).
  **Enquanto** acontece a cada quadro, enquanto tudo vale.
- **O outro** é com quem a condição aconteceu: em quem o jogador encostou, o
  item que ele pegou. Ex.: *Quando o jogador encostar em chave, então destruir
  o outro*.
- **Esperar N segundos** atrasa as ações seguintes da regra.
- O ☑ desliga a regra sem apagar; o ☰ duplica, sobe, desce, põe um
  **comentário** acima ou exclui. Desfazer e refazer valem para tudo.
- Nomes de objetos, variáveis ou arquivos que não existem ficam com um
  contorno amarelo e a explicação ao passar o mouse.
- **Ver como código** mostra o Lua que faz o mesmo que as regras (só para ler
  e aprender: o jogo usa as regras).

| Condições (Quando) | |
|---|---|
| Cena e tempo | começar, a cada N segundos, depois de N segundos, sorte (chance em %) |
| Controles | apertar, soltar ou estar segurando uma tecla (ou um grupo: pular, confirmar, voltar, setas...), clicar num objeto |
| Toques | encostar em, parar de encostar, estar encostando, estar perto de, encostar num tile (de um tipo) |
| Variáveis | comparar uma variável do jogo (=, ≠, <, >, ≤, ≥) |
| Objetos | não sobrar nenhum (com um nome), ser destruído, sair da tela, estar no chão ou no ar, estar visível ou escondido |
| Mensagens | pegar um coletável, levar dano, a vida acabar, receber uma mensagem |

| Ações (Então) | |
|---|---|
| Variáveis | mudar (somar, subtrair, mudar para, ficar com o maior/menor), salvar no disco, carregar do disco, zerar todas |
| Cenas | ir para outra cena, reiniciar (do começo ou do checkpoint), fechar o jogo |
| Objetos | criar um prefab, destruir, mostrar ou esconder |
| Movimento | levar para uma posição, mudar a velocidade ou empurrar |
| Aparência | mudar texto (com `{variavel}`), imagem, quadro ou cor, soltar faíscas, tremer a câmera |
| Som | tocar um som, tocar ou parar a música |
| Comportamentos | ligar ou desligar um comportamento, mandar uma mensagem |
| Tempo | esperar, escrever no console |
| Mapas | trocar o tile tocado, trocar o tile em um objeto (0 apaga; a colisão acompanha) |

Algumas ações têm opções a mais no 🔧 (deslocamento ao criar, volume, força do
tremor...). O formato no arquivo está no
[guia do Lua](lua.md#eventos-quando-então).

### Partículas

![Tochas do modelo Aventura com fogo de partículas animando na cena](imagens/nn-studio-particulas.png)

**➕ Objeto > Partículas** cria um emissor vazio ou um **efeito pronto**:
fogo, fumaça, faíscas, explosão, poeira, brilho, bolhas, chuva, neve e
confete. No Inspetor, **✨ Efeito pronto** troca os valores pelos de outro
efeito (Ctrl+Z desfaz).

As partículas **animam na cena do editor**, como no jogo. O botão **✨
Partículas**, na barra da cena, pausa a prévia. Com o objeto selecionado, a
**seta laranja** mostra a direção e a abertura (arraste a ponta para mudar a
direção; com Ctrl, de 15 em 15 graus), e a área onde as partículas nascem
aparece tracejada.

O componente tem três grupos:

- **Emissão.** *Contínuo* solta sem parar (por segundo); *De uma vez* solta
  uma quantidade junta, ao começar (*Soltar ao começar*), repetindo (*Repetir
  a cada*) ou só quando um evento pedir. *Ao terminar: destruir o objeto*
  faz o objeto sumir quando todas as partículas acabam. A área onde nascem
  pode ser um ponto, um retângulo (chuva e neve no alto da tela) ou um
  círculo.
- **Movimento.** Vida (com variação), velocidade, direção, abertura,
  gravidade (negativa cai, positiva sobe, como a fumaça), freio e
  **Acompanham o objeto** (o jato de uma nave anda junto com ela; desligado,
  vira um rastro).
- **Aparência.** Tamanho inicial e final (com variação), cores inicial,
  do meio (opcional) e final, **Luz** (soma as cores: fogo e faíscas mais
  fortes), uma **imagem** ou um quadro de spritesheet no lugar do círculo
  suave, e giro.

**💥 Soltar**, no Inspetor, solta uma leva na prévia. Uma explosão no
editor repete sozinha, para dar para ver.

**Efeitos em prefab.** Um efeito que aparece quando algo acontece (uma
explosão, um brilho ao pegar uma moeda) fica bem num [prefab](#prefabs): um
objeto com Partículas *De uma vez* e *Ao terminar: destruir o objeto*. A
ação **Criar prefab** dos eventos põe ele onde precisa; ele estoura e some.

Nos [eventos](#eventos-quando-então): a ação **Soltar partículas** solta uma
quantidade de um objeto (poeira ao pular), **Ligar ou desligar partículas**
para ou volta a emissão contínua, e a condição **Partículas terminarem** avisa
quando uma explosão acabou.

Os modelos têm exemplos: na **Nave**, o jato atrás da nave e a explosão dos
asteroides (`prefabs/explosao.prefab`); na **Plataforma**, poeira ao pular e
um brilho ao pegar moeda; na **Aventura**, tochas com fogo.

### Desfazer e salvar

**Ctrl+Z** desfaz e **Ctrl+Y** (ou Ctrl+Shift+Z) refaz o que foi feito na cena,
na Hierarquia ou no Inspetor; arrastar com o mouse conta como um passo só.
**Ctrl+S** salva o `.cena`. A aba mostra "•" enquanto houver alterações.

O `.cena` é um arquivo Lua comum e legível (veja o formato no
[guia do Lua](lua.md#cenas-visuais-cena)). Dá para editá-lo à mão: se ele mudar
fora do editor, a aba recarrega sozinha.

### Testar daqui e ajuste ao vivo

**▶ Testar daqui (F6)** salva tudo e roda o jogo começando nesta cena, em vez
da cena inicial do projeto.

Com o jogo aberto mostrando a cena, aparece **• Ao vivo** na barra da cena: o
que você mudar no Inspetor ou arrastando na cena aparece no jogo **na hora,
sem salvar**. Para guardar a mudança, salve; para descartar, desfaça.

E o caminho de volta, do jogo para o editor:

- **Fantasmas:** onde um objeto está agora no jogo (se andou para longe do
  lugar onde você o pôs), a cena mostra o contorno tracejado dele.
- **No jogo:** no Inspetor, a posição atual do objeto no jogo e os valores dos
  comportamentos que mudaram (as vidas que restam, por exemplo) aparecem em
  ciano. **Manter** traz o valor para a cena (e dá para desfazer). Sem nada
  selecionado, o Inspetor mostra as variáveis do jogo agora.
- **Pausar / Continuar / Avançar 1 quadro**, na barra da cena, congelam o
  jogo para olhar com calma.
- **Ctrl+clique** num objeto, na janela do jogo, seleciona ele no editor.
- **Eventos ao vivo:** o que você muda nas regras também vale na hora, e cada
  regra **acende** no painel Eventos quando dispara no jogo. Ótimo para
  entender por que algo aconteceu (ou não).

### O script da cena

Para regras que nenhum comportamento cobre, a cena pode ter um script Lua (no
Inspetor, **Script > Criar**). Ele recebe os mesmos callbacks de uma cena em
código (`enter`, `fixed_update`, `update`,
`draw`...) e encontra os objetos em `self.objects`, pelo nome:

```lua
local cena = {}

function cena:fixed_update(dt)
  local heroi = self.objects.heroi
  heroi.body.vx = nn.input.move() * 300
end

-- Dois corpos se tocaram.
function cena:contact(e)
  if e.object_a.name == "moeda" then e.object_a:destroy() end
end

return cena
```

A referência completa está no [guia do Lua](lua.md#cenas-visuais-cena).

## Editor de pixel art

Clicar numa imagem (PNG) abre o editor de pixel art. Ele grava PNG comum, que
abre em qualquer programa; ao salvar, a cena do editor e o jogo aberto mostram
o desenho novo na hora.

![Editor de pixel art com a animação "andar" do herói](imagens/nn-studio-pixel-art.png)

- **Ferramentas** (com atalho): **Lápis** (B), **Borracha** (E), **Balde**
  (G: pinta a área da mesma cor), **Linha** (L), **Retângulo** (R),
  **Círculo** (O), **Conta-gotas** (I) e **Seleção** (M). Botão esquerdo
  pinta com a cor principal; o direito, com a secundária (que começa
  transparente: apaga). Alt+clique pega a cor com qualquer ferramenta.
  Shift deixa a linha reta e o retângulo e o círculo, iguais dos lados;
  Shift+clique com o lápis faz uma linha desde o último ponto.
- À direita: o tamanho do **pincel** ([ e ] mudam), **Preenchido** (retângulo
  e círculo) e **Espelhado**, que desenha dos dois lados do quadro ao mesmo
  tempo (bom para personagens de frente).
- **Cores:** principal e secundária (X troca), e as paletas: as cores
  desta imagem, a **paleta do projeto** (arquivo `paleta.hex`, para usar as
  mesmas cores em todas as imagens) e paletas prontas (Sweetie 16, PICO-8,
  Endesga 32).
- **Seleção:** arraste para selecionar; arraste a seleção para mover.
  Ctrl+C, Ctrl+X, Ctrl+V (entre imagens também), Delete apaga, Enter ou Esc
  solta. **↔ / ↕ / Girar** espelham ou giram a seleção (ou o quadro todo).
- **Vista:** role o mouse para aproximar (no ponto do mouse); arraste com o
  botão do meio ou com Espaço para mover. **Ajustar** cabe tudo na tela.
  **Grade** mostra os pixels de perto.
- **Tamanho…** muda o tamanho da imagem (o desenho fica no canto de cima, à
  esquerda). **Copiar código** copia o `nn.assets.texture(...)` ou o
  `nn.assets.sheet(...)`.
- Ctrl+Z / Ctrl+Y desfazem e refazem; Ctrl+S salva. Uma aba com • tem
  alterações não salvas.

### Quadros

Marque **Quadros** e o tamanho de cada um (16 × 16, por exemplo) para dividir
a imagem numa folha de sprites. **Folha** mostra todos e **Quadro** só o atual,
maior. No painel de baixo (**Quadros**), clique num quadro para desenhar nele;
**+ Novo quadro** e **Duplicar** criam o próximo (duplicar é o jeito mais
rápido de fazer o passo seguinte de uma animação), **Excluir** tira, e ◀ ▶
mudam de lugar. As animações acompanham: tirar o quadro 3 faz o 4 virar o 3
nelas também. **Papel vegetal** mostra o quadro anterior clarinho por baixo
(o anterior na animação escolhida, se o quadro estiver nela).

O tamanho do quadro fica no arquivo de animações da imagem (abaixo); uma
imagem sem esse arquivo usa o tamanho que as cenas já usam para ela.

### Animações

Uma animação é uma sequência de quadros com nome: "parado", "andar",
"pular". Elas ficam num arquivo ao lado da imagem (`heroi.png` → `heroi.anim`)
e servem para qualquer cena ou prefab que use a imagem. No painel de baixo,
**Animações**:

- **+ Nova animação** cria uma (a primeira já vem com todos os quadros).
  Clique num nome para escolher; o ☰ usa todos os quadros, tira todos,
  duplica ou exclui.
- **Ritmo** (quadros por segundo) e **modo**: Repetir, Uma vez ou Ida e
  volta.
- A **linha do tempo** mostra os quadros da animação. Clique num **quadro da
  folha** (embaixo) para pôr no fim, ou arraste para o lugar certo; arraste
  dentro da linha do tempo para mudar a ordem. Botão direito num quadro: uma
  **duração** própria (um quadro mais longo, como num golpe) ou tirar da
  animação.
- A **Prévia** (à direita) toca a animação escolhida.

Para usar: no Inspetor, **Imagem > Animação** toca uma animação desde o
começo; na **Animação automática**, cada estado (parado, andando, pulando,
caindo) escolhe uma; os eventos têm **Tocar animação** e **Animação
terminar**; no Lua, `obj.sprite:play("andar")`
([guia do Lua](lua.md#animações-anim)). Os modelos Plataforma, Aventura e
Vazio já vêm com `heroi.anim` e `moeda.anim`.

## Mapas de tiles

Um mapa de tiles é uma fase montada com quadradinhos (tiles) de uma imagem só,
pintados numa grade. O NN Studio pinta os mapas **direto na cena** (ou na aba
do mapa) e grava no formato do [Tiled](https://www.mapeditor.org/) (`.tmj`, com
o tileset num `.tsj` ao lado da imagem): dá para abrir os mesmos arquivos no
Tiled, e o que você faz lá aparece aqui.

![Pintando o chão com o terreno "terra" na cena, com o painel Tiles à direita](imagens/nn-studio-tiles.png)

- **Mapa novo:** na cena, **➕ Objeto > Mapa de tiles...** (ou no painel Tiles).
  Escolha os tiles: um tileset do projeto, uma imagem (o `.tsj` é criado ao
  lado dela) ou uma imagem nova em branco para desenhar. O mapa vai para
  `mapas/` e o objeto já entra no modo Pintar. Os modelos Plataforma, Aventura
  e Vazio trazem `assets/imagens/terreno.png` e `terreno.tsj`.
- **✏ Pintar:** selecione o objeto do mapa e clique em **✏ Pintar** (na barra
  da cena ou no Inspetor). O mouse passa a pintar tiles; **Esc** sai. Rolar o
  mouse aproxima, botão do meio ou Espaço + arrastar move a vista. Pintar
  fora do mapa faz ele **crescer** (à esquerda e embaixo, o objeto anda junto
  para o desenho ficar no lugar).
- **Ferramentas** (painel **Tiles**): **Pincel** (B), **Borracha** (E),
  **Balde** (G), **Retângulo** (U), **Conta-gotas** (I, ou botão direito em
  qualquer ferramenta) e **Terreno** (T). Clique num tile do tileset, ou
  arraste para pegar vários: o pincel pinta o pedaço inteiro. **Ctrl +
  arrastar** no mapa copia um pedaço para o pincel. **R** gira o pincel 90°,
  **X** espelha (Shift+X, na vertical): os tiles ficam girados no mapa, como
  no Tiled.
- **Terrenos (autotile):** escolha um terreno (terra, grama, água) e pinte: as
  bordas e os cantos se encaixam sozinhos, com 16 peças (cantos do Tiled).
  **apagar** tira o terreno e acerta as bordas em volta. Uma célula pintada
  com o terreno é o terreno inteiro; as peças das vizinhas só levam a borda
  (a grama por cima, a areia na margem), então o que bloqueia é exatamente o
  que você pintou. Peças com mais de uma opção são sorteadas pela
  probabilidade de cada uma (uma grama com flores de vez em quando).
- **+ Terreno** cria um terreno seu: as 16 peças entram no fim da imagem do
  tileset, já com os cantos marcados e pintadas como guia. **Abrir a imagem**
  leva ao [editor de pixel art](#editor-de-pixel-art) para desenhar por cima.
- **Camadas:** o mapa tem camadas pintadas uma por cima da outra (o chão, os
  enfeites, a frente). Cada uma pode ficar escondida, **travada** (não pinta)
  ou ser uma camada de **colisão** (tudo nela bloqueia). Clique duas vezes
  para renomear; **Subir**, **Descer**, **Excluir** e a **opacidade**. O
  pincel pinta na camada escolhida (também no alto do painel).
- **Marcar os tiles:** **Sólido** (bloqueia), **Plataforma** (de mão única:
  dá para pular por baixo), **Tipo** (um nome como `lava`, `agua`, `tijolo`,
  para os eventos), **Animação** (clique num tile e Ctrl+clique nos que vêm
  depois; cada quadro tem a sua duração) e **Terreno** (clique nos cantos de
  cada tile para dizer de qual terreno é cada canto). A marcação fica no
  tileset e vale para todos os mapas que usam ele. **Colisão** mostra o que
  bloqueia (vermelho) e as plataformas (azul) por cima do mapa.
- **Ao vivo:** com o jogo aberto (F6), o que você pinta dentro do mapa aparece
  no jogo na hora, com a colisão. Quando o mapa cresce, o jogo vê ao salvar.
- **Aba do mapa:** clicar num `.tmj` abre o mapa sozinho, com o mesmo pincel.
  **Abrir no Tiled** abre no programa padrão. Mapas que o editor não sabe
  pintar (infinitos, camadas compactadas em base64, tilesets `.tsx`) mostram
  a prévia e o motivo. Tilesets que estão dentro do mapa (do Tiled) se pintam
  normalmente; para marcar tiles neles, **Separar num arquivo .tsj**.
- **Desfazer e salvar:** Ctrl+Z desfaz cada pincelada (na cena, enquanto
  pinta, ou na aba do mapa). Ctrl+S na cena salva a cena, os mapas dela e os
  tilesets juntos; a aba da cena ganha o • quando o mapa tem alterações.

Nos [eventos](#eventos-quando-então), **Encostar num tile** (de um tipo, ou
qualquer um) e **Trocar o tile tocado** fazem lava, espinhos, água e blocos
que quebram; **Trocar o tile em um objeto** troca o tile onde um objeto está.
O campo de tile tem um **…** para escolher no tileset. No Lua:
`obj:tiles_touching()`, `mapa:tiles_in(...)` e `mapa:tile_properties(...)`
([guia do Lua](lua.md#nntilemap)).

## Imagens, sons e mapas

- **Imagens:** abrem no [editor de pixel art](#editor-de-pixel-art). Imagens
  que não são PNG (a engine só lê PNG) mostram um aviso.
- **Sons:** forma de onda, ▶ Tocar e o código `nn.assets.sound(...)`.
- **Mapas do Tiled (.tmj):** abrem no [editor de mapas](#mapas-de-tiles).
  Ao salvar no Tiled, o editor e o jogo aberto atualizam.

## Jogar

- **▶ Jogar (F5)** salva tudo e abre o jogo na janela dele. Com o jogo aberto,
  salve um arquivo: a mudança aparece na hora, sem fechar o jogo.
- **▶ Testar daqui (F6)**, numa cena visual, começa o jogo nela (veja
  [Testar daqui e ajuste ao vivo](#testar-daqui-e-ajuste-ao-vivo)).
- **⟳ Reiniciar (F5)** fecha e abre o jogo de novo; **⏹ Parar (Shift+F5)**
  fecha.
- O **Console** recebe o que o jogo escreve com `print()`, os avisos e os
  erros. Erros aparecem em português, e cada `arquivo.lua:12` é um link: clique
  para abrir o arquivo nessa linha. Se o console estiver escondido quando
  chegar um erro, ele aparece sozinho.
- Filtros do console: **Erros**, **Avisos** e **Mensagens** (e **Engine**, no
  modo Completo). **Limpar ao jogar** começa cada partida com o console limpo.

## Configurações do projeto

**Editar > Configurações do projeto** (ou o botão no Início) mostra um
formulário com o título, o tamanho da janela, o **tamanho do jogo**, tela
cheia, cor de fundo e a cena inicial (uma cena em código ou uma cena visual),
além das
[variáveis do jogo](#variáveis-do-jogo-e-hud): nome e valor inicial (número,
texto ou sim/não), e o [tema da interface](#interface-menus-e-hud). No modo Completo também o identificador dos saves, VSync,
janela redimensionável, recarregar ao salvar e passos fixos por segundo. Tudo
fica no `config.lua`: o NN Studio troca só os valores, sem mexer nos seus
comentários.

**Tamanho do jogo:** com **Fixo** marcado, o jogo enxerga sempre o mesmo
tamanho (por exemplo 1280 × 720 ou 640 × 360) e se ajusta à janela, maior ou
menor, sem cortar nada: **Ajustar** mantém a proporção (com faixas pretas
quando a janela é de outra proporção), **Esticar** ocupa tudo e **Pixel art**
só aumenta em números inteiros (2×, 3×...), para os pixels ficarem nítidos. O
retângulo "tela do jogo" na cena passa a ter esse tamanho. Os modelos novos já
vêm com 1280 × 720.

## Exportar

**Jogo > Exportar jogo...** gera o executável único do jogo (o mesmo do
`nn exportar`), na pasta `dist` do projeto ou na que você escolher. Antes,
os scripts são conferidos: com erro de sintaxe, a exportação não acontece e a
mensagem diz onde está o erro.

## Atalhos

| Atalho | Comando |
|---|---|
| F5 / Shift+F5 | Jogar (ou reiniciar) / Parar |
| F6 | Testar a partir da cena visual aberta |
| Ctrl+S / Ctrl+Shift+S | Salvar / Salvar tudo |
| Ctrl+Z / Ctrl+Y | Desfazer / refazer (na cena visual, nos mapas e no editor de pixel art) |
| Ctrl+N | Nova cena visual |
| Ctrl+Shift+N / Ctrl+O | Novo projeto / Abrir projeto |
| Ctrl+Shift+P / Ctrl+P | Paleta de comandos / Abrir arquivo |
| Ctrl+F / Ctrl+H / Ctrl+G | Buscar / Substituir / Ir para a linha |
| Ctrl+/ | Comentar linhas |
| Ctrl+Espaço | Autocompletar (no editor de código) |
| Tab / Shift+Tab | Recuar / desfazer recuo |
| Ctrl+W | Fechar aba |
| Ctrl+= / Ctrl+- / Ctrl+0 | Aumentar / diminuir / tamanho normal da interface |
| F2 / Del (no painel) | Renomear / mandar para a lixeira |
| W / E / R (na cena) | Mover / girar / escalar |
| F / Esc (na cena) | Enquadrar / limpar a seleção |
| Setas, Shift+setas (na cena) | Mover 1 / 10 unidades |
| Ctrl+D / Del (na cena) | Duplicar / apagar |
| Ctrl+clique (na janela do jogo) | Selecionar o objeto no editor |
| B E G L R O I M (na imagem) | Lápis, borracha, balde, linha, retângulo, círculo, conta-gotas, seleção |
| X / [ ] / H (na imagem) | Trocar as cores / tamanho do pincel / espelhar |
| , / . (na imagem) | Quadro anterior / próximo |
| B E G U I T (pintando um mapa) | Pincel, borracha, balde, retângulo, conta-gotas, terreno |
| R / X / Shift+X (pintando um mapa) | Girar / espelhar / espelhar na vertical o pincel |
| Ctrl + arrastar / botão direito (pintando) | Copiar um pedaço / pegar o tile |
| F1 | Guia do Lua |

A lista também está em **Ajuda > Atalhos do teclado**.

## Onde ficam as preferências

Projetos recentes, tema, modo, tamanho da interface e layouts ficam num
arquivo `preferencias.json` na pasta de configuração do usuário
(`%APPDATA%\NN Studio\config` no Windows, `~/.config/nnstudio` no Linux). Os
projetos não ganham nenhum arquivo do editor: a pasta do jogo continua só com
o jogo.

## Como funciona por dentro

- O editor é feito com [egui](https://github.com/emilk/egui) e
  [egui_dock](https://github.com/Adanos020/egui_dock), desenhados pelo mesmo
  wgpu da engine.
- ▶ Jogar roda o próprio `nn-studio` com `--jogar <pasta>` num processo
  separado. O jogo escreve o log numa linha por mensagem (`@nn {json}`) e o
  editor lê. Se o jogo travar, o editor continua de pé.
- As mensagens de erro do Lua são traduzidas pela própria engine
  (`nn_script::translate_error`), então a tela de erro do jogo também fica em
  português.
- As cenas visuais são lidas por um módulo Lua dentro da engine
  (`crates/nn-script/src/cena.lua`). A lista de componentes e dos valores
  padrão dele é a mesma que o Inspetor usa. Na aba da cena, o editor desenha
  com o renderer da engine na GPU do próprio editor.
- **Testar daqui** roda `nn-studio --jogar <pasta> --cena <arquivo>`. O ajuste
  ao vivo vai pela entrada padrão do jogo, uma linha JSON por mudança
  (`{"scene": "cenas/fase.cena", "id": 3, "field": "x", "value": 120}`, ou
  `{"cmd": "pause"}`, `"resume"`, `"step"`). O jogo responde pelo log: qual
  cena está na frente, o estado dos objetos (10 vezes por segundo, em JSON) e
  o objeto do Ctrl+clique.
- Os comportamentos prontos são módulos Lua dentro da engine
  (`crates/nn-script/src/comportamentos/`), escritos com a mesma API que um
  script seu usaria. O Inspetor lê a lista de propriedades de cada um rodando
  o módulo num Lua separado, sem o jogo.
- O autocompletar e as descrições vêm de `nn_script::api_docs`, a mesma lista
  que um teste confere contra a API de verdade.
- Os eventos rodam num módulo Lua da engine
  (`crates/nn-script/src/eventos.lua`), que também descreve cada condição e
  ação (frase, campos e o Lua equivalente). O painel Eventos monta os menus e
  as frases a partir dessa descrição. Mudanças nas regras vão para o jogo
  aberto como `{"id": 0, "field": "events", "value": [...]}` (id 0 = regras
  da cena), e o estado que o jogo manda diz quais regras dispararam.
- O tutorial confere o projeto a cada quadro; os painéis marcam onde estão os
  botões que ele destaca.
- O editor de pixel art guarda o desenho na memória e grava o PNG com a
  biblioteca `image`. Os arquivos `.anim` são tabelas Lua lidas pela própria
  engine (`cena.lua`, `parse_anims`) e pelo NN Studio no mesmo formato.
- Os mapas são lidos e gravados no formato JSON do Tiled (o que o editor não
  conhece, como objetos e propriedades, fica como estava). Na cena, o mapa
  aberto é desenhado pelo renderer da engine a partir do que está na memória,
  sem salvar; na aba do mapa, o egui desenha os tiles. Os giros seguem os bits
  do Tiled (diagonal, horizontal, vertical). O ajuste ao vivo dos tiles vai
  como `{"field": "tiles", "value": [["camada", coluna, linha, tile], ...]}`
  e o jogo chama `set_tile`, que também acerta a colisão.
- Repetir a imagem é feito no shader de sprites: cada sprite tem quantas
  vezes a região se repete, então um chão de 20 blocos continua sendo um
  desenho só.

## O que vem depois

A próxima etapa é a **v0.1**: acabamento, revisão dos tutoriais e dos
modelos, e o NN Studio pronto para instalar.
