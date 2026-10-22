# {{TITULO}}

Aventura vista de cima, feita com a [nn-engine](https://github.com/nereuneto/nn-engine) no **NN Studio**, sem escrever código.

## Como é o jogo

Ande com as setas ou WASD. Pegue a chave, abra a porta e encontre o tesouro. As lesmas perseguem você e os morcegos patrulham a sala do tesouro: encostar neles tira uma vida.

## Como foi feito

- `cenas/titulo.cena`: título com botões (componente **Botão**, que funciona com mouse, teclado e controle). Um evento zera as variáveis a cada jogo novo.
- `cenas/mapa.cena`: a fase, **sem gravidade** (no Inspetor da cena). O herói tem **Jogador top-down**, **Animação automática** e **Vida**; as lesmas, **Perseguir** e **Perigo**; os morcegos, **Patrulha** e **Perigo**; a chave, **Coletável** (soma em `chaves`).
- O chão, a água e as paredes são um **mapa de tiles** (`mapas/mapa.tmj`). Selecione o objeto `mapa` e clique em **✏ Pintar**: no painel **Tiles**, os terrenos **grama** e **agua** pintam o lago com as margens certas; as paredes ficam na camada `paredes`. A água e as pedras bloqueiam (marcadas como **Sólido** no tileset `assets/imagens/terreno.tsj`).
- A **porta** e o **baú** têm **eventos** (painel Eventos): com uma chave, a porta abre e gasta a chave; sem chave, aparece um aviso por 2 segundos. Encostar no baú abre o baú, toca a vitória e, depois de 1,5 segundo, vai para `cenas/fim.cena`.
- O **velho**, perto do lago, tem o comportamento **Diálogo**: perto dele aparece "E falar" e a conversa (`dialogos/velho.dialogo`, que abre no editor de conversa) conta onde está a chave.
- As variáveis do jogo (`chaves`, `vidas`) ficam no `config.lua`.
- As **tochas** ao lado da porta têm um filho com **Partículas** (o efeito pronto Fogo, um pouco menor).
- O **HUD** fica preso à tela: `vidas` (componente **Ícones**, um coração para cada vida) no canto de cima à esquerda e `chaves` (texto com Mostrar valor) no canto de cima à direita.

Selecione a porta e veja os eventos dela no painel **Eventos**. **Ver como código** mostra as mesmas regras em Lua.

Sem o editor, com o `nn` instalado, de dentro desta pasta: `nn .`

## Testes

`testes/jogo.lua` joga o jogo sozinho, sem janela, e confere se o principal funciona (andar, a chave e a porta, o baú). Com o `nn` instalado, de dentro desta pasta: `nn testar`. Mudou o jogo? Rode de novo para ver se nada quebrou, e escreva testes seus no mesmo jeito (veja "Testes de jogo" na documentação da engine).
