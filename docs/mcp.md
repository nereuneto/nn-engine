# O servidor MCP: desenvolver com um agente de IA

A ferramenta `nn` tem um **servidor MCP** (Model Context Protocol). Ele liga um agente de IA (Claude Code, Claude Desktop, Cursor ou outro cliente MCP) a um projeto de jogo. O agente conhece o projeto, monta cenas, põe comportamentos e eventos, escreve scripts Lua e confere o que fez, tudo pelas mesmas regras do NN Studio. O que ele grava o editor abre igual, e o contrário também.

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

## Segurança

- O servidor recusa caminhos fora da pasta do jogo (`..`, caminhos absolutos, links que saem dela) e não mexe em `.git`, `dist` e `target`.
- Ele grava cada arquivo de uma vez (num temporário que troca de nome), então o jogo aberto e o NN Studio nunca veem um arquivo pela metade.
- `desfazer` volta as mudanças do agente enquanto o servidor estiver aberto. Se você mudou o mesmo arquivo depois (no NN Studio, à mão), ele só desfaz com `forcar`, para não perder o seu trabalho. Para um histórico de verdade, use o git na pasta do jogo.

## O que vem depois

Esta é a primeira parte (v0.7). As próximas sprints trazem:

- **Rodar e ver:** o agente roda o jogo sem janela, com teclas simuladas, e recebe os erros, os prints, o estado dos objetos e uma captura da tela. Vêm também os **testes de jogo** (`nn testar`).
- **Ao vivo:** ligar no jogo aberto e no NN Studio, ler o estado e mudar valores na hora.
- **Arte, modelagem e animação (v0.8):** pixel art, sons, modelos 3D e animações pelas ferramentas, com prévias em imagem.
