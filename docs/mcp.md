# O servidor MCP: desenvolver com um agente de IA

A ferramenta `nn` tem um **servidor MCP** (Model Context Protocol). Ele liga um agente de IA (Claude Code, Claude Desktop, Cursor ou outro cliente MCP) a um projeto de jogo. O agente conhece o projeto, monta cenas, põe comportamentos e eventos, escreve scripts Lua e confere o que fez, tudo pelas mesmas regras do NN Studio. O que ele grava o editor abre igual, e o contrário também. Ele também **joga o jogo sem janela**, com teclas e cliques simulados, e vê os erros, o que mudou e a tela, roda os **testes de jogo** do projeto e acompanha o **jogo aberto** enquanto você joga, mudando valores na hora.

```
nn mcp <pasta-do-jogo>
```

O servidor conversa com o agente pela entrada e saída padrão: quem abre o `nn mcp` é o próprio cliente, configurado como mostrado abaixo. Ele só mexe em arquivos **dentro da pasta do jogo**, e **cada mudança pode ser desfeita** (ferramenta `desfazer`).

## Ligar no seu agente

O `nn` vem nos pacotes da release:

- **Windows (instalador):** `C:\Users\<você>\AppData\Local\Programs\NN Studio\nn.exe`.
- **Windows (zip portátil) e Linux (tar.gz):** o `nn` fica na pasta que você extraiu.
- **Linux (AppImage):** `NN-Studio-<versão>-x86_64.AppImage nn mcp <pasta>` roda o `nn` que vem dentro dele.

Nos exemplos abaixo, troque `nn` pelo caminho completo se ele não estiver no PATH.

### Claude Code

De dentro da pasta do jogo:

```
claude mcp add nn -- nn mcp .
```

Com `--scope project`, a configuração vai para um `.mcp.json` na pasta. Quem abrir o projeto com o Claude Code ganha o servidor também. Para ver se ligou, use `/mcp` dentro do Claude Code.

No Windows, com o instalador:

```
claude mcp add nn -- "C:\Users\<você>\AppData\Local\Programs\NN Studio\nn.exe" mcp .
```

### Claude Desktop

Em **Configurações > Desenvolvedor > Editar configuração**, no `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "nn": {
      "command": "C:\\Users\\<você>\\AppData\\Local\\Programs\\NN Studio\\nn.exe",
      "args": ["mcp", "C:\\Users\\<você>\\Documents\\meu-jogo"]
    }
  }
}
```

Reabra o Claude Desktop depois de salvar.

### Cursor e outros

Qualquer cliente MCP que abre servidores locais (stdio) serve. O comando é o `nn`, e os argumentos são `mcp` e a pasta do jogo. No Cursor, num `.cursor/mcp.json` dentro do projeto:

```json
{ "mcpServers": { "nn": { "command": "nn", "args": ["mcp", "."] } } }
```

## Começar

Peça ao agente o que você quer, do jeito que pediria a uma pessoa: *"faz uma fase de plataforma com três moedas e uma porta que só abre com todas"*. O servidor diz ao agente como trabalhar, e todo projeto novo vem com um `AGENTS.md` que explica a organização dele.

Numa pasta vazia, o agente cria o jogo a partir de um modelo (`criar_projeto`). Há também dois **prompts** prontos:

- `criar_jogo`: faz um jogo do começo, a partir de uma ideia;
- `melhorar_jogo`: faz uma mudança pedida no jogo aberto.

No Claude Code eles aparecem como `/mcp__nn__criar_jogo` e `/mcp__nn__melhorar_jogo`.

## As ferramentas

| Ferramenta | O que faz |
|---|---|
| `ver_projeto` | O jogo: título, 2D ou 3D, tela, cena inicial, variáveis e todos os arquivos. Numa pasta vazia, os modelos. |
| `criar_projeto` | Cria um jogo a partir de um modelo (`visual`, `aventura`, `nave`, `visual3d`, `vale3d`, `padrao`, `3d`, `vazio`, `vazio3d`, `minimo`). |
| `mudar_config` | Muda o `config.lua` sem perder os comentários: título, cena inicial, tamanho, variáveis do jogo e tema. |
| `conferir_projeto` | Confere tudo sem rodar o jogo: sintaxe dos scripts, cenas que não abrem, arquivos citados que não existem, comportamentos e eventos desconhecidos, componentes que faltam. |
| `listar_arquivos`, `ler_arquivo` | Os arquivos do projeto. Uma imagem volta como imagem, para o agente ver (e o tamanho, para os quadros de uma spritesheet). |
| `escrever_arquivo`, `apagar_arquivo` | Scripts, mapas, diálogos. Um Lua com erro de sintaxe **não é gravado**: volta a linha e o erro em português. |
| `desfazer` | Volta as últimas mudanças feitas pelo agente. |
| `ver_cena` | Uma cena ou um prefab em JSON, no formato do arquivo, com as regras de eventos em frases. |
| `criar_cena`, `mudar_cena` | Cenas 2D ou 3D, a cor do fundo, a gravidade, o ambiente 3D (céu, sol, névoa). |
| `criar_objeto`, `mudar_objeto`, `copiar_objeto`, `apagar_objeto` | Objetos com componentes e comportamentos, cada campo conferido. Também filhos, fileiras de cópias e trocar de pai. |
| `criar_prefab` | Transforma um objeto num prefab. Mudar o prefab muda todas as cópias. |
| `listar_componentes`, `listar_comportamentos` | Os componentes (com os campos e padrões) e os comportamentos prontos e do projeto (com as propriedades, opções e ajuda). |
| `listar_eventos`, `por_regra`, `tirar_regra` | Os eventos "Quando... Então..." sem código. |
| `buscar_documentacao` | A API Lua `nn.*` e os guias (este, o de Lua, o de 3D e o do NN Studio). |
| `conferir_lua` | A sintaxe de um trecho de Lua, sem gravar. |
| `rodar_jogo` | Roda o jogo **sem janela**, com teclas e cliques simulados, e mostra os erros (com arquivo e linha), avisos, prints, as regras que dispararam, as trocas de cena, o que mudou nos objetos e variáveis, e **capturas da tela**. Veja [Rodar e ver](#rodar-e-ver). |
| `rodar_testes` | Roda os [testes de jogo](lua.md#testes-de-jogo) do projeto (`testes/*.lua`) e mostra quem falhou, a linha, os prints e a tela na hora da falha. |
| `abrir_jogo` | Abre o jogo **numa janela** no seu computador (você vê e pode jogar junto) e liga nele. Veja [O jogo aberto](#o-jogo-aberto). |
| `ver_jogo_aberto` | Olha o jogo aberto (por `abrir_jogo`, pelo ▶ Jogar do NN Studio ou por `nn .`) enquanto alguém joga: a cena, os erros, prints e regras desde a última olhada, o que mudou e a tela. |
| `mexer_no_jogo` | Muda o jogo aberto **na hora, sem gravar**: campos de objetos e comportamentos, variáveis, Lua; pausa, avança quadros e fecha. |

Os guias também vão para o agente como **recursos** (`nn://docs/lua.md`, `nn://docs/3d.md`, `nn://docs/studio.md`, `nn://docs/mcp.md` e `nn://projeto/AGENTS.md`).

## Objetos no formato do arquivo

`ver_cena` mostra os objetos como estão no `.cena`, só com o que difere do padrão (`completo: true` mostra tudo). `criar_objeto` aceita o mesmo formato:

```json
{
  "name": "heroi", "x": -400, "y": -200, "layer": 2,
  "components": [
    {"type": "sprite", "image": "assets/imagens/heroi.png", "frame_w": 16, "frame_h": 16, "w": 64, "h": 64},
    {"type": "behavior", "script": "nn:plataforma", "velocidade": 300},
    {"type": "behavior", "script": "nn:vida", "vidas": 3}
  ]
}
```

- `type` é um componente (`sprite`, `shape`, `text`, `body`, `camera`, `particles`, `button`, `model`, `body3d`...) ou um comportamento: `"behavior"` com o `script` (`"nn:plataforma"`, ou o caminho de um `.lua` do projeto). Também vale só `"type": "nn:plataforma"`.
- Os campos que faltam ficam com o padrão. Cada campo é conferido: um nome errado, um tipo errado, uma cor fora do formato ou uma opção que não existe voltam como erro, com a lista do que vale.
- Um comportamento que precisa de corpo físico (`nn:plataforma`, `nn:patrulha`...) já ganha um, como no NN Studio.
- Uma cópia de prefab: `{"name": "moeda2", "prefab": "prefabs/moeda.prefab", "x": 100, "y": 0}`.
- No 2D, as medidas são em pixels, com y para cima e (0, 0) no meio da tela. No 3D, são em metros, e y é a altura.

As regras de eventos também vão no formato do arquivo:

```json
{
  "when": [{"cond": "pegar", "quem": "@jogador", "item": "moeda"}],
  "actions": [{"act": "som", "som": "assets/sons/moeda.wav"}]
}
```

## Rodar e ver

`rodar_jogo` roda o jogo sem janela: 60 quadros por segundo de jogo, o mais rápido que der (não no tempo do relógio), sem som, com o `math.random` sempre na mesma sequência e com o salvo (`nn.save`) só na memória. Rodar não mexe nos recordes de quem joga.

O jogador é simulado com **ações**, cada uma no seu tempo (`em`, em segundos desde o começo):

```json
{
  "cena": "cenas/fase.cena",
  "segundos": 3,
  "acoes": [
    {"em": 0.2, "segurar": "right", "por": 1.5},
    {"em": 1.0, "apertar": "space"},
    {"em": 1.2, "capturar": "no pulo"}
  ],
  "objetos": ["heroi"],
  "olhar": ["nn.vars.moedas"]
}
```

| Ação | O que faz |
|---|---|
| `"apertar": "space"` | Aperta e solta a tecla. |
| `"segurar": "right", "por": 1.5` | Segura a tecla (sem `por`, até o fim). |
| `"soltar": "right"` | Solta uma tecla segurada. |
| `"clicar": [x, y]` | Clica num ponto da tela (pixels, y para baixo). Com `"botao": "right"`, o botão direito. |
| `"clicar": "jogar"` | Clica no objeto com esse nome (um botão do menu, por exemplo). |
| `"mouse": [x, y]` | Põe o mouse num ponto, sem clicar. |
| `"digitar": "Nereu"` | Escreve num campo de texto. |
| `"capturar": true` ou `"capturar": "nome"` | Guarda uma captura da tela agora. |
| `"lua": "self.objects.heroi:move_to(0, 0)"` | Roda código com `self` = a cena da frente: para preparar a situação (levar o herói até a porta, mudar uma variável). |

As teclas têm os nomes do `nn.input`: `a` a `z`, `0` a `9`, `space`, `enter`, `escape`, `left`, `right`, `up`, `down`, `shift`, `ctrl`, `alt`, `tab`... Sem `cena`, o jogo começa na cena inicial do `config.lua`. Com `"segundos": 0`, só mostra a primeira tela.

A resposta conta o que aconteceu, com o tempo de cada coisa:

```
Rodou 3.00 s de jogo (180 quadros), começando em cenas/fase.cena.

Erros: nenhum.

Regras dos eventos que dispararam:
  1.00 s  cenas/fase.cena: regra 4 da cena: quando tecla e no_chao → soltar_particulas
  1.70 s  cenas/fase.cena: regra 5 da cena: quando pegar → criar

O que mudou em cenas/fase.cena do começo ao fim:
  heroi (5): x 0 → 480, y -180 → -210
  moeda (8): sumiu (destruído)
  moedas (15): text.text "Moedas: 0" → "Moedas: 1"
Variáveis que mudaram: moedas 0 → 1
```

e traz as capturas (a do fim e as pedidas) como imagens. Se um script dá erro, o jogo para ali, e a resposta mostra o erro com o arquivo e a linha. Um laço sem fim (`while true do end`) vira erro depois de 5 segundos, em vez de travar o servidor.

Para conferir o que importa de novo depois de cada mudança, o agente escreve **testes de jogo** em `testes/` e roda com `rodar_testes`. Os testes estão no [guia de Lua](lua.md#testes-de-jogo); `nn testar` roda os mesmos testes no terminal.

## O jogo aberto

Com o jogo aberto numa janela, o agente acompanha você jogando. Ele liga no jogo aberto pelo **▶ Jogar** do NN Studio, por `nn .` na pasta ou por ele mesmo, com `abrir_jogo` (que também aceita uma `cena` para começar).

- `ver_jogo_aberto` mostra a cena da frente, o tempo, se está pausado, os erros, avisos e prints e as regras que dispararam **desde a última olhada**, o que mudou nos objetos e nas variáveis desde a última olhada, e a **tela agora** como imagem. Com `objetos` e `olhar`, os objetos por inteiro e expressões Lua.
- `mexer_no_jogo` muda o jogo **na hora, sem gravar nada**. Serve para experimentar um valor com você antes de mudar o arquivo:

```json
{
  "mudar": [{"objeto": "heroi", "componente": "nn:plataforma", "campo": "pulo", "valor": 1000}],
  "variaveis": {"vidas": 9},
  "lua": "self.objects.heroi:move_to(400, 0)",
  "pausar": true,
  "avancar": 30
}
```

  `avancar` anda um número de quadros (60 = 1 s) e deixa o jogo pausado; `"pausar": false` continua, e `"fechar": true` fecha o jogo.
- O que fica, o agente grava nos arquivos (`mudar_objeto`, `escrever_arquivo`...): o jogo aberto recarrega sozinho, e o NN Studio mostra a mudança.

O jogo aberto atende numa porta do próprio computador (`127.0.0.1`), com uma senha num arquivo que só o seu usuário lê. Um programa de fora do computador não fala com ele. Um jogo exportado não atende ninguém.

## Segurança

- O servidor recusa caminhos fora da pasta do jogo (`..`, caminhos absolutos, links que saem dela) e não mexe em `.git`, `dist` e `target`.
- Ele grava cada arquivo de uma vez (num temporário que troca de nome), então o jogo aberto e o NN Studio nunca veem um arquivo pela metade.
- `desfazer` volta as mudanças do agente enquanto o servidor estiver aberto. Se você mudou o mesmo arquivo depois (no NN Studio, à mão), ele só desfaz com `forcar`, para não perder o seu trabalho. Para um histórico de verdade, use o git na pasta do jogo.

## O que vem depois

Depois desta, vem a revisão e o lançamento da v0.7. Depois, na v0.8:

- **Arte, modelagem e animação (v0.8):** pixel art, sons, modelos 3D e animações pelas ferramentas, com prévias em imagem.
