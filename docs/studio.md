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
    e recorde salvo). É o melhor jeito de começar pelo código.
  - **Cena visual:** uma fase de plataforma montada no editor, com o mouse
    (herói, plataformas, moedas e placar), e um script curto com as regras. O
    jeito de começar pelo editor.
  - **Mínimo:** só uma cena com um quadrado que anda.
- **📂 Abrir projeto:** escolha a pasta do jogo (a que tem o `config.lua` ou o
  `main.lua`). Também dá para **arrastar a pasta** para a janela.
- **🎮 Exemplos:** abre uma cópia do jogo de plataforma ou da demonstração,
  para mexer à vontade sem estragar o original.

## A janela do projeto

Os painéis podem ser arrastados pelas abas e encaixados onde você quiser. O
layout e as abas abertas ficam guardados para cada projeto; para voltar ao
original, use **Exibir > Restaurar layout padrão**.

- **Arquivos** (à esquerda, em cima): a pasta do projeto.
- **Hierarquia** (à esquerda, embaixo): os objetos da cena visual aberta.
- **Início, cenas, código, imagens, sons e mapas** (no centro).
- **Inspetor** (à direita): as propriedades do objeto selecionado.
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
- **➕ Novo:** cena visual (montada com o mouse, veja abaixo), cena em código
  (já vem com um quadrado que anda), script (módulo para usar com `require`)
  ou pasta. Cenas novas vão para `cenas/` e scripts para `lib/`, se essas
  pastas existirem.
- **Arraste** uma imagem, um mapa ou um som do painel para dentro de uma cena
  visual (vira um objeto) ou para um campo de arquivo do Inspetor.
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

## Cenas visuais

Uma cena visual (arquivo `.cena`) é montada com o mouse: você põe objetos,
arruma no lugar e ajusta as propriedades, sem escrever código. Ela roda na
engine como qualquer outra cena (`nn.scene.switch("cenas/fase.cena")`, cena
inicial no `config.lua`, exportação, recarregar ao salvar). Crie uma com
**Ctrl+N** ou **➕ Novo > Cena visual**.

![Cena visual do modelo "Cena visual" aberta no NN Studio](imagens/nn-studio-cena.png)

### A aba da cena

A cena é desenhada pela própria engine, do jeito que vai aparecer no jogo. O
retângulo "tela do jogo" mostra o que cabe na janela (o tamanho do
`config.lua`), a partir da posição da câmera.

- **Navegar:** role o mouse para o zoom (no ponto do cursor); arraste com o
  botão do meio, com o direito ou com **Espaço** segurado para mover a vista.
  **F** (ou ⛶ Enquadrar) enquadra o selecionado ou a cena toda.
- **Selecionar:** clique num objeto; **Shift** ou **Ctrl** somam à seleção;
  arraste num espaço vazio para selecionar com um retângulo. **Esc** limpa.
- **Ferramentas:** **W** mover, **E** girar, **R** escalar. Arraste o objeto ou
  as alças: as setas vermelha e verde movem só em X ou em Y; o círculo gira
  (com **Ctrl**, de 15 em 15 graus); os cantos escalam (com **Shift**,
  proporcional).
- **Encaixar** prende na grade ao mover (segurar **Ctrl** também encaixa). As
  **setas do teclado** movem o selecionado 1 unidade (10 com **Shift**).
- **Ctrl+D** duplica, **Del** apaga. **➕ Objeto** cria um objeto vazio, uma
  imagem, um retângulo, um círculo, um texto, uma câmera, um som ou
  partículas no meio da vista.
- **Arrastar arquivos** do painel Arquivos para a cena: uma imagem vira um
  sprite, um mapa `.tmj` vira um mapa, um som vira um objeto de som.
- **Colisões** mostra os corpos físicos (verde) por cima.

### Hierarquia

A lista dos objetos da cena, na ordem de desenho. Clique para selecionar,
clique duas vezes (ou **F2**) para renomear. O 👁 esconde e o cadeado trava (um
objeto travado não é selecionado clicando na cena). **Arraste** um objeto para
mudar a ordem ou solte em cima de outro para virar filho dele: o filho anda,
gira e escala junto com o pai. O botão direito tem Duplicar, Criar filho
vazio, Tirar do pai e Excluir; o campo 🔍 filtra pelo nome.

### Inspetor

Sem nada selecionado, mostra a cena: o **script** dela, a cor de **fundo** e a
**gravidade**. Com um objeto selecionado, mostra o nome, visível/travado, a
**Transformação** (posição, rotação em graus, escala e camada: camada maior é
desenhada por cima) e os **componentes**:

| Componente | O que faz |
|---|---|
| Imagem (sprite) | Mostra uma imagem ou um quadro de uma spritesheet. Com **É uma tira de quadros**, escolha o quadro clicando na grade. |
| Texto | Escreve um texto no mundo. |
| Forma | Retângulo ou círculo de uma cor. |
| Corpo físico | Cai, colide, para no chão (dinâmico, estático ou cinemático; caixa ou círculo; sensor; plataforma de mão única). |
| Mapa do Tiled | Desenha um mapa `.tmj`, com a colisão dos tiles. |
| Câmera | Onde a câmera começa, ou quem ela segue (com suavidade e limite no mapa). |
| Som | Toca um som ou música quando a cena começa. |
| Partículas | Fumaça, faíscas, poeira, chuva... |

**➕ Adicionar componente** acrescenta outro; o menu ☰ de cada um sobe, desce
ou remove. Campos de arquivo aceitam arrastar do painel Arquivos. No modo
Simples, opções avançadas ficam escondidas.

### Desfazer e salvar

**Ctrl+Z** desfaz e **Ctrl+Y** (ou Ctrl+Shift+Z) refaz o que foi feito na cena,
na Hierarquia ou no Inspetor; arrastar com o mouse conta como um passo só.
**Ctrl+S** salva o `.cena`. A aba mostra "•" enquanto houver alterações.

O `.cena` é um arquivo Lua comum e legível (veja o formato no
[guia do Lua](lua.md#cenas-visuais-cena)). Dá para editá-lo à mão: se ele mudar
fora do editor, a aba recarrega sozinha.

### Testar daqui e ajuste ao vivo

**▶ Testar daqui (F6)** salva tudo e roda o jogo começando nesta cena, em vez
da cena inicial do projeto.

Com o jogo aberto mostrando a cena, aparece **• Ao vivo** na barra da cena: o
que você mudar no Inspetor ou arrastando na cena aparece no jogo **na hora,
sem salvar**. Para guardar a mudança, salve; para descartar, desfaça.

### O script da cena

A cena pode ter um script Lua (no Inspetor, **Script > Criar**). Ele recebe os
mesmos callbacks de uma cena em código (`enter`, `fixed_update`, `update`,
`draw`...) e encontra os objetos em `self.objects`, pelo nome:

```lua
local cena = {}

function cena:fixed_update(dt)
  local heroi = self.objects.heroi
  heroi.body.vx = nn.input.move() * 300
end

-- Dois corpos se tocaram.
function cena:contact(e)
  if e.object_a.name == "moeda" then e.object_a:destroy() end
end

return cena
```

A referência completa está no [guia do Lua](lua.md#cenas-visuais-cena).

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
- **▶ Testar daqui (F6)**, numa cena visual, começa o jogo nela (veja
  [Testar daqui e ajuste ao vivo](#testar-daqui-e-ajuste-ao-vivo)).
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
| F6 | Testar a partir da cena visual aberta |
| Ctrl+S / Ctrl+Shift+S | Salvar / Salvar tudo |
| Ctrl+Z / Ctrl+Y | Desfazer / refazer (na cena visual) |
| Ctrl+N | Nova cena visual |
| Ctrl+Shift+N / Ctrl+O | Novo projeto / Abrir projeto |
| Ctrl+Shift+P / Ctrl+P | Paleta de comandos / Abrir arquivo |
| Ctrl+F / Ctrl+H / Ctrl+G | Buscar / Substituir / Ir para a linha |
| Ctrl+/ | Comentar linhas |
| Tab / Shift+Tab | Recuar / desfazer recuo |
| Ctrl+W | Fechar aba |
| Ctrl+= / Ctrl+- / Ctrl+0 | Aumentar / diminuir / tamanho normal da interface |
| F2 / Del (no painel) | Renomear / mandar para a lixeira |
| W / E / R (na cena) | Mover / girar / escalar |
| F / Esc (na cena) | Enquadrar / limpar a seleção |
| Setas, Shift+setas (na cena) | Mover 1 / 10 unidades |
| Ctrl+D / Del (na cena) | Duplicar / apagar |
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
- As cenas visuais são lidas por um módulo Lua dentro da engine
  (`crates/nn-script/src/cena.lua`). A lista de componentes e dos valores
  padrão dele é a mesma que o Inspetor usa. Na aba da cena, o editor desenha
  com o renderer da engine na GPU do próprio editor.
- **Testar daqui** roda `nn-studio --jogar <pasta> --cena <arquivo>`. O ajuste
  ao vivo vai pela entrada padrão do jogo, uma linha JSON por mudança
  (`{"scene": "cenas/fase.cena", "id": 3, "field": "x", "value": 120}`); o
  jogo avisa no log qual cena está na frente.

## O que vem depois

As próximas etapas do NN Studio trazem comportamentos prontos sem código
(andar, pular, seguir, coletar...), prefabs, as variáveis do script no
Inspetor, eventos "Quando... Então...", o tutorial guiado e editores de pixel
art, animação, mapas, partículas e menus dentro do próprio editor.
