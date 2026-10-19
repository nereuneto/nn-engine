# {{TITULO}}

Jogo 3D feito com a [nn-engine](https://github.com/nereuneto/nn-engine) no **NN Studio**, **sem código**: ache as 5 estrelas da ilha, pule nas plataformas e pegue as moedas.

Tudo está montado nas cenas visuais:

- `cenas/ilha.cena`: a ilha. Abra no NN Studio para ver a vista 3D.
  - **Herói**: modelo animado `heroi.glb` com o comportamento **Jogador 3D** (anda, pula e leva a câmera atrás dele, tocando as animações parado, correr, pular e cair sozinho).
  - **Ilha**, **Plataforma**, **Árvore** e **Pedra**: **Corpo 3D** parado. Nas plataformas e nas árvores, a forma automática usa a malha exata do modelo.
  - **Plataforma que anda** e **Elevador**: Corpo 3D que se move sozinho e o comportamento **Plataforma que anda**. Elas levam o herói junto.
  - **Caixote**: Corpo 3D que cai e é empurrado. Empurre com o herói!
  - **moeda** e **estrela**: prefabs (`prefabs/`) com **Coletável** e **Girar e flutuar**. Mudar o prefab muda todas.
  - **Placar**: um texto fixo na tela com **Mostrar valor**.
  - **Eventos** da cena (painel Eventos): zerar as variáveis no começo, esconder a dica e ir para o fim com as 5 estrelas. No Herói, os sons de pulo e de queda.
- `cenas/fim.cena`: a tela de fim, com botões.

## Controles

WASD ou as setas andam (para onde a câmera olha), espaço pula, o mouse gira a câmera. Esc solta o mouse; um clique na janela prende de novo. Funciona com controle (analógicos e A).

## Para mudar

- O sol, o céu e a névoa: clique em **☀ Ambiente** na barra da cena.
- Mais uma estrela: arraste `prefabs/estrela.prefab` para a cena e mude o "5" no evento de vitória.
- Velocidade e pulo do herói: no Inspetor, no Jogador 3D.

Sem o editor, com o `nn` instalado, de dentro desta pasta: `nn .`
