# {{TITULO}}

Jogo 3D feito com a [nn-engine](https://github.com/nereuneto/nn-engine) no **NN Studio**, **sem código**: ache as 5 estrelas da ilha, pule nas plataformas e pegue as moedas.

Tudo está montado nas cenas visuais:

- `cenas/ilha.cena`: a ilha. Abra no NN Studio para ver a vista 3D.
  - **Herói**: modelo animado `heroi.glb` com o comportamento **Jogador 3D** (anda, pula e leva a câmera atrás dele, tocando as animações parado, correr, pular e cair sozinho).
  - **Ilha**, **Plataforma**, **Árvore** e **Pedra**: **Corpo 3D** parado. Nas plataformas e nas árvores, a forma automática usa a malha exata do modelo.
  - **Plataforma que anda** e **Elevador**: Corpo 3D que se move sozinho e o comportamento **Plataforma que anda**. Elas levam o herói junto.
  - **Caixote**: Corpo 3D que cai e é empurrado. Empurre com o herói!
  - **Besouro**: dois besouros animados com o **Inimigo 3D** (um patrulha, o outro vaga; os dois perseguem quando você chega perto). Encostar tira vida; pular em cima derrota. O herói tem **Vida**, e os **Corações** (presos à tela) mostram quanto resta.
  - **moeda** e **estrela**: prefabs (`prefabs/`) com **Coletável** e **Girar e flutuar**. Mudar o prefab muda todas.
  - **Fogueira**: partículas de fogo e de fumaça (filhas dela) e um **Som** no espaço 3D que estala mais alto de perto. **Ondas**: o barulho do mar nas praias, do mesmo jeito.
  - **Poeira** (filha do Herói): partículas soltas por um evento do Herói quando ele pula.
  - **Cofre**: uma casinha de pedra com uma estrela dentro. A **Porta do cofre** (`porta.glb`) tem o comportamento **Porta 3D**, trancada: precisa do item *chave*. A **Chave** (`chave.glb`, numa das plataformas) é um **Coletável** que guarda o item no inventário, e o objeto **Inventário** (**Mostrar inventário**) mostra os itens no canto da tela. A **Placa do cofre** é um **Texto 3D** preso como placa.
  - **Pescador**: o herói tingido de azul, com o comportamento **Diálogo**. Perto dele aparece "E falar" e a conversa (`dialogos/pescador.dialogo`, que abre no editor de conversa) dá dicas da ilha. O nome em cima dele é um **Texto 3D** (o filho **Nome**).
  - **Placar**: um texto fixo na tela com **Mostrar valor**. Ele, os Corações e a **Dica** são do HUD: na vista 3D do editor, aparecem por cima da cena, na moldura da tela do jogo (**HUD**, na barra, mostra ou esconde), e dá para clicar e arrastar.
  - **Eventos** da cena (painel Eventos): zerar as variáveis no começo, esconder a dica e ir para o fim com as 5 estrelas. No Herói, os sons de pulo e de queda.
- `cenas/fim.cena`: a tela de fim, com botões.

## Controles

WASD ou as setas andam (para onde a câmera olha), espaço pula, o mouse gira a câmera. Esc solta o mouse; um clique na janela prende de novo. Funciona com controle (analógicos e A).

## Para mudar

- O sol, o céu e a névoa: clique em **☀ Ambiente** na barra da cena.
- Mais uma estrela: arraste `prefabs/estrela.prefab` para a cena e mude o "6" no evento de vitória.
- Velocidade e pulo do herói: no Inspetor, no Jogador 3D.

Sem o editor, com o `nn` instalado, de dentro desta pasta: `nn .`

## Testes

`testes/jogo.lua` joga o jogo sozinho, sem janela, e confere se o principal funciona (andar e pular, as moedas, as estrelas). Com o `nn` instalado, de dentro desta pasta: `nn testar`. Mudou o jogo? Rode de novo para ver se nada quebrou, e escreva testes seus no mesmo jeito (veja "Testes de jogo" na documentação da engine).
