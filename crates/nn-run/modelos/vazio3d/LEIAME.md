# {{TITULO}}

Jogo 3D feito com a [nn-engine](https://github.com/nereuneto/nn-engine) no **NN Studio**.

Este projeto começa com um mundo 3D vazio: uma cena (`cenas/mundo.cena`) com câmera e chão, e um pacote de modelos e sons para você montar o seu jogo.

## Por onde começar

No NN Studio, **Ajuda > Tutoriais > Seu primeiro mundo 3D** mostra, passo a passo, como montar um mundo aqui mesmo: um herói animado que anda e pula, árvores, uma caixa que cai, um bicho que anda, uma estrela para pegar, uma plataforma que anda e um evento de vitória.

## O que tem aqui

- `modelos/` (modelos 3D `.glb`):
  - `heroi.glb`: o herói (1,8 m, com a origem nos pés), animado: parado, andar, correr, pular, cair e acenar;
  - `bicho.glb`: um besouro animado (parado e andar);
  - `arvore.glb` e `pedra.glb`: enfeites;
  - `caixote.glb`: um caixote de 1 m;
  - `plataforma.glb`: plataforma de 1 × 1 m, com o topo na origem (escale em X e Z);
  - `estrela.glb` e `moeda.glb`: itens.
- `assets/sons/`: moeda, pulo, dano e vitória.

Arraste um modelo do painel Arquivos para a cena para criar um objeto com ele.

Sem o editor, com o `nn` instalado, de dentro desta pasta: `nn .`
