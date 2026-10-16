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
- **✨ Novo projeto:** escolha o nome do jogo, onde criar e o modelo:
  - **Padrão:** um jogo pequeno e completo (título com menu, partida, pausa, fim
    e recorde salvo). É o melhor jeito de começar.
  - **Mínimo:** só uma cena com um quadrado que anda.
- **📂 Abrir projeto:** escolha a pasta do jogo (a que tem o `config.lua` ou o
  `main.lua`). Também dá para **arrastar a pasta** para a janela.
- **🎮 Exemplos:** abre uma cópia do jogo de plataforma ou da demonstração,
  para mexer à vontade sem estragar o original.

## A janela do projeto

Os painéis podem ser arrastados pelas abas e encaixados onde você quiser. O
layout e as abas abertas ficam guardados para cada projeto; para voltar ao
original, use **Exibir > Restaurar layout padrão**.

- **Arquivos** (à esquerda): a pasta do projeto.
- **Início, código, imagens, sons e mapas** (no centro).
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
- **➕ Novo:** cena (já vem com um quadrado que anda), script (módulo para usar
  com `require`) ou pasta. Cenas novas vão para `cenas/` e scripts para `lib/`,
  se essas pastas existirem.
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

## Imagens, sons e mapas

- **Imagens:** zoom (botões ou Ctrl + rolar), grade de pixels de perto e
  **Quadros**, que mostra a divisão da spritesheet com o número de cada quadro
  (o mesmo número que se usa no Lua). **Copiar código** copia o
  `nn.assets.texture(...)` ou o `nn.assets.sheet(...)` com o tamanho do quadro.
- **Sons:** forma de onda, ▶ Tocar e o código `nn.assets.sound(...)`.
- **Mapas do Tiled (.tmj):** prévia com os tiles e os objetos marcados (passe
  o mouse para ver o tipo). **Editar no Tiled** abre o mapa no programa
  padrão; ao salvar lá, a prévia atualiza.

## Jogar

- **▶ Jogar (F5)** salva tudo e abre o jogo na janela dele. Com o jogo aberto,
  salve um arquivo: a mudança aparece na hora, sem fechar o jogo.
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
formulário com o título, o tamanho da janela, tela cheia, cor de fundo e a cena
inicial. No modo Completo também o identificador dos saves, VSync, janela
redimensionável, recarregar ao salvar e passos fixos por segundo. Tudo fica no
`config.lua`: o NN Studio troca só os valores, sem mexer nos seus comentários.

## Exportar

**Jogo > Exportar jogo...** gera o executável único do jogo (o mesmo do
`nn exportar`), na pasta `dist` do projeto ou na que você escolher. Antes,
os scripts são conferidos: com erro de sintaxe, a exportação não acontece e a
mensagem diz onde está o erro.

## Atalhos

| Atalho | Comando |
|---|---|
| F5 / Shift+F5 | Jogar (ou reiniciar) / Parar |
| Ctrl+S / Ctrl+Shift+S | Salvar / Salvar tudo |
| Ctrl+N | Nova cena |
| Ctrl+Shift+N / Ctrl+O | Novo projeto / Abrir projeto |
| Ctrl+Shift+P / Ctrl+P | Paleta de comandos / Abrir arquivo |
| Ctrl+F / Ctrl+H / Ctrl+G | Buscar / Substituir / Ir para a linha |
| Ctrl+/ | Comentar linhas |
| Tab / Shift+Tab | Recuar / desfazer recuo |
| Ctrl+W | Fechar aba |
| Ctrl+= / Ctrl+- / Ctrl+0 | Aumentar / diminuir / tamanho normal da interface |
| F2 / Del (no painel) | Renomear / mandar para a lixeira |
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

## O que vem depois

As próximas etapas do NN Studio trazem a cena visual (objetos, hierarquia,
inspetor, mover com o mouse, desfazer), comportamentos prontos sem código,
eventos "Quando... Então...", o tutorial guiado e editores de pixel art,
animação, mapas, partículas e menus dentro do próprio editor.
