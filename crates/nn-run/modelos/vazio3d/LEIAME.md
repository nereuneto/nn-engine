# {{TITULO}}

Jogo 3D feito com a [nn-engine](https://github.com/nereuneto/nn-engine) no **NN Studio**.

Este projeto começa com um mundo 3D vazio: uma cena (`cenas/mundo.cena`) com câmera e chão, e um pacote de modelos e sons para você montar o seu jogo.

## Por onde começar

No NN Studio, **Ajuda > Tutoriais > Seu primeiro mundo 3D** mostra, passo a passo, como montar um mundo aqui mesmo: um herói animado que anda e pula, árvores, uma caixa que cai, um bicho que anda, uma estrela para pegar, uma plataforma que anda e um evento de vitória. Os outros tutoriais 3D usam este mesmo modelo, cada um com uma parte:

- **Efeitos no 3D**: fogueira com fogo, fumaça e som, poeira ao pular e chuva;
- **Pinte um terreno**: morros, um lago, cores e árvores espalhadas com o pincel;
- **Inimigos no 3D**: vida, corações na tela, um besouro que persegue e lava;
- **Portas e chaves**: uma porta trancada, a chave, o inventário e uma zona de gatilho;
- **Uma conversa**: um personagem que fala, no editor de conversa.

Para um jogo inteiro já pronto, crie um projeto com o modelo **3D** (a ilha) ou o **Vale 3D**.

## O que tem aqui

- `modelos/` (modelos 3D `.glb`):
  - `heroi.glb`: o herói (1,8 m, com a origem nos pés), animado: parado, andar, correr, pular, cair e acenar;
  - `bicho.glb`: um besouro animado (parado e andar);
  - `arvore.glb` e `pedra.glb`: enfeites;
  - `caixote.glb`: um caixote de 1 m;
  - `plataforma.glb`: plataforma de 1 × 1 m, com o topo na origem (escale em X e Z);
  - `estrela.glb`, `moeda.glb` e `chave.glb`: itens;
  - `porta.glb`: uma porta de madeira de 1,2 × 2,2 m, com a base na origem (use com o comportamento **Porta 3D**).
- `assets/imagens/`: `coracao.png` e `chave.png` (ícones para o HUD e o inventário).
- `assets/sons/`: moeda, pulo, dano, vitória, chave, porta, fogueira e mar.

Arraste um modelo do painel Arquivos para a cena para criar um objeto com ele.

Sem o editor, com o `nn` instalado, de dentro desta pasta: `nn .`
