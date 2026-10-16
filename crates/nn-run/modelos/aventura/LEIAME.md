# {{TITULO}}

Aventura vista de cima, feita com a [nn-engine](https://github.com/nereuneto/nn-engine) no **NN Studio**, sem escrever código.

## Como é o jogo

Ande com as setas ou WASD. Pegue a chave, abra a porta e encontre o tesouro. As lesmas perseguem você e os morcegos patrulham a sala do tesouro: encostar neles tira uma vida.

## Como foi feito

- `cenas/titulo.cena`: título com botões (comportamento **Botão**). Um evento zera as variáveis a cada jogo novo.
- `cenas/mapa.cena`: a fase, **sem gravidade** (no Inspetor da cena). O herói tem **Jogador top-down**, **Animação automática** e **Vida**; as lesmas, **Perseguir** e **Perigo**; os morcegos, **Patrulha** e **Perigo**; a chave, **Coletável** (soma em `chaves`).
- A **porta** e o **baú** têm **eventos** (painel Eventos): com uma chave, a porta abre e gasta a chave; sem chave, aparece um aviso por 2 segundos. Encostar no baú abre o baú, toca a vitória e, depois de 1,5 segundo, vai para `cenas/fim.cena`.
- As variáveis do jogo (`chaves`, `vidas`) ficam no `config.lua`.

Selecione a porta e veja os eventos dela no painel **Eventos**. **Ver como código** mostra as mesmas regras em Lua.

Sem o editor, com o `nn` instalado, de dentro desta pasta: `nn .`
