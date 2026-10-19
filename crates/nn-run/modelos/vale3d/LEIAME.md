# {{TITULO}}

Jogo 3D feito com a [nn-engine](https://github.com/nereuneto/nn-engine) no **NN Studio**, **sem código**: ache as 5 estrelas nos morros do vale, siga a trilha das moedas e não caia no lago.

Tudo está montado nas cenas visuais:

- `cenas/vale.cena`: o vale. Abra no NN Studio para ver a vista 3D.
  - **Terreno**: os morros, o lago, a trilha e as cores são um componente **Terreno**. Selecione e clique em **✏ Pintar terreno** na barra da vista para mudar: subir, descer, alisar, aplainar, ruído, pintar as quatro cores e espalhar árvores. A água é do próprio terreno (Inspetor > Água).
  - **Herói**: modelo animado com o **Jogador 3D**. Se cair no lago, volta para a margem (e um evento toca um som).
  - **arvore** e **pedra**: espalhadas, com **Corpo 3D** parado.
  - **estrela** e **moeda**: prefabs (`prefabs/`) com **Coletável** e **Girar e flutuar**.
  - **Fogueira**: fogo, fumaça e o estalar (som 3D). **Lago**: o barulho da água.
  - **Eventos** da cena: zerar as variáveis, esconder a dica e ir para o fim com as 5 estrelas.
- `cenas/fim.cena`: a tela de fim, com botões.

## Controles

WASD ou as setas andam (para onde a câmera olha), espaço pula, o mouse gira a câmera. Esc solta o mouse; um clique na janela prende de novo. Funciona com controle (analógicos e A).

Sem o editor, com o `nn` instalado, de dentro desta pasta: `nn .`
