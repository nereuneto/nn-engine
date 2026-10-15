# {{TITULO}}

Jogo feito em Lua com a [nn-engine](https://github.com/nereuneto/nn-engine).

## Jogar

Com o `nn` instalado, de dentro desta pasta:

```sh
nn .
```

Ou, a partir da pasta da nn-engine: `cargo nn <caminho desta pasta>`.

Com o jogo aberto, **salve qualquer arquivo `.lua`, imagem ou som e a mudança aparece na hora**. Se um script tiver erro, ele aparece na janela com o arquivo e a linha; corrija, salve e o jogo continua.

## O que tem aqui

```
config.lua          título, tamanho da janela e cena inicial
cenas/
  titulo.lua        menu principal (Jogar, Sair) e recorde
  jogo.lua          a partida: ande e pegue moedas antes do tempo acabar
  pausa.lua         menu de pausa, por cima do jogo
  fim.lua           resultado da partida
lib/
  jogo.lua          controles (ações) e aparência dos menus
assets/
  imagens/          heroi.png (8 quadros de 16x16), moeda.png
  sons/
```

Ideias para começar: mude `DURACAO` e `VELOCIDADE` no topo de `cenas/jogo.lua`, troque as imagens em `assets/imagens`, acrescente obstáculos ou uma segunda moeda.

## Distribuir

```sh
nn exportar .
```

Gera um único executável em `dist/` com o jogo inteiro dentro, pronto para mandar para qualquer pessoa.

## Referência

A API completa (`nn.draw`, `nn.input`, `nn.ui`, `nn.save`, física, tilemaps, partículas...) está em [docs/lua.md](https://github.com/nereuneto/nn-engine/blob/main/docs/lua.md).
