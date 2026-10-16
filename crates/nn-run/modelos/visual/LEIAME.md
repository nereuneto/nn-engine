# {{TITULO}}

Jogo feito em Lua com a [nn-engine](https://github.com/nereuneto/nn-engine), com a fase montada no **NN Studio**.

## Jogar e editar

Abra esta pasta no NN Studio e dê dois cliques em `cenas/fase.cena`: a fase aparece na aba da cena, com os objetos na Hierarquia e as propriedades no Inspetor. Arraste imagens do painel Arquivos para a cena, mova com o mouse e aperte **F6** para testar a partir dela.

Com o jogo aberto, o que você muda no Inspetor ou arrastando na cena aparece no jogo na hora (indicador **Ao vivo**). Salvar qualquer arquivo também atualiza o jogo.

Sem o editor, com o `nn` instalado, de dentro desta pasta: `nn .`

## O que tem aqui

```
config.lua          título, tamanho da janela e cena inicial
cenas/
  fase.cena         a fase: chão, plataformas, herói, moedas e placar
  fase.lua          o script da fase: andar, pular e pegar moedas
assets/
  imagens/          heroi.png (8 quadros de 16x16), moeda.png (4 quadros)
  sons/             moeda.wav
```

Ideias para começar: mude `VELOCIDADE` e `PULO` no topo de `cenas/fase.lua`, duplique moedas e plataformas na cena (Ctrl+D) e arraste-as para outros lugares.

## Distribuir

No NN Studio, **Jogo → Exportar jogo**. Ou, com o `nn`: `nn exportar .`
