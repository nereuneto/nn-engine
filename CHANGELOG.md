# O que mudou

As versões seguem o [versionamento semântico](https://semver.org/lang/pt-BR/). Antes da 1.0, uma versão nova pode mudar coisas de um jeito que pede ajustes nos projetos; quando isso acontecer, está escrito aqui.

## Não lançado

- **A engine passa a pedir o Rust 1.89** (antes 1.88): a física 3D usa o `nalgebra` 0.35, que precisa dele. O NN Studio continua pedindo o 1.95.

### 3D pelo código

- **Mundo 3D na engine**, desenhado antes do 2D: o HUD, os sprites e os menus ficam por cima.
- **Formas prontas**: cubo, plano, esfera, cilindro, cone e cápsula, e malhas montadas ponto a ponto.
- **Visual low-poly**: luz do sol, luz ambiente do céu e do chão, uma cor por face e névoa.
- **API Lua `nn.d3`**: desenhar com posição, rotação e escala; câmera que olha para um ponto ou gira em volta dele; achar onde o mouse toca o chão; pôr textos 2D em cima de coisas 3D.
- **Modelos `.glb` e `.gltf`** (do Blender e outros) com `nn.assets.model`: partes, materiais, cores e texturas; recarregam quando o arquivo muda.
- **Sombras do sol**, ligadas por padrão, com qualidade, distância e `shadow = false` para quem não deve projetar.
- **Céu em degradê** com o disco do sol; a névoa usa a cor do horizonte.
- **Exemplo `cubos3d`** com árvores, casas, pedras, caixotes e herói em `.glb` (modelos nossos, gerados por `ferramentas/gerar_modelos.py`). Também entre os exemplos do NN Studio.
- No `cubos3d`, o herói pula e empurra caixotes, os cubos caem e empilham onde você clica (até no telhado) e as casas e árvores bloqueiam.
- No NN Studio, modelos 3D aparecem com ícone próprio e "Copiar nn.assets.model".

### 3D no NN Studio

- **Cena 3D**: em **Nova cena visual**, escolha **Tipo: 3D**. É o mesmo `.cena`, com `mode = "3d"`, e já vem com câmera, chão e um cubo.
- **Vista 3D** desenhada pela engine, com céu, sol, névoa, sombras e uma grade no chão com os eixos X (vermelho) e Z (azul).
- **Navegação como na Unity e no Godot**: botão direito + WASD voa (Q/E descem e sobem, a rodinha muda a velocidade), Alt + arrastar gira em volta, o botão do meio arrasta e F enquadra.
- **Objetos 3D**: formas prontas, modelos `.glb` arrastados do painel Arquivos (caem em pé onde o mouse aponta) e a câmera do jogo, vista como um cone.
- **Alças 3D** para mover (por eixo ou no chão), girar e escalar, com encaixe na grade (Ctrl troca), e clique para selecionar.
- **Inspetor** com posição, rotação e escala em X, Y e Z e os componentes Modelo 3D, Forma 3D e Câmera 3D. Hierarquia, desfazer e refazer, duplicar e prefabs funcionam como no 2D.
- **☀ Ambiente**: céu, altura e lado do sol, luz ambiente, névoa e sombras, mudando na hora na vista.
- **Testar daqui (F6)** roda a cena 3D. No Lua, `nn.d3.matrix` e `obj:matrix3d()` para posições presas a outras.

### 3D sem código

- **Corpo 3D** (`body3d`): parado (chão, paredes, cenário), cai e é empurrado (caixas) ou se move sozinho (plataformas). A forma automática usa a malha exata do modelo no que fica parado e uma caixa, esfera, cápsula ou cilindro do tamanho do objeto no resto. **Colisões**, na barra da cena, mostra os corpos.
- **Jogador 3D**: anda para onde a câmera olha, pula (com pulo duplo opcional), vira para onde anda e leva a câmera em terceira pessoa, com o mouse preso (Esc solta). Caiu do mundo: volta ao último chão firme e recebe a mensagem `caiu`.
- **Comportamentos no 3D**: Girar e flutuar, Plataforma que anda (leva o herói junto), e Coletável, Ir para cena, Checkpoint, Mostrar valor e Botão, que agora funcionam nas duas. O Inspetor só oferece o que funciona no tipo da cena.
- **Eventos no 3D**: encostar e perto em metros, Levar para e Velocidade com Z, Criar prefab no ponto 3D, Mudar cor em modelos e formas. O que é só do 2D some dos menus de uma cena 3D.
- **Ao vivo no 3D**: posição, rotação, escala, componentes e o Ambiente mudam no jogo aberto na hora.
- **Tutorial guiado "Seu primeiro mundo 3D"** e os modelos **3D** (a ilha das estrelas sem código) e **Vazio 3D** (`nn novo --modelo visual3d` e `--modelo vazio3d`). O modelo 3D em código passa a se chamar "3D em código" no NN Studio.
- No Lua: `obj:position3d()`, `obj:bounds3d()`, `obj:move_to(x, y, z)`, `self:world3()`, `m:inverse()` e `m:decompose()` nas matrizes, `modelo:bounds()` e `sx`, `sy`, `sz` nos corpos de `nn.physics3d`. Comportamentos podem declarar `mode = "2d"`, `"3d"` ou `"both"`.
- O sol padrão de uma cena 3D vem de lado (`sun_turn = 120`), para as sombras aparecerem.

### Animação de esqueleto

- **Modelos animados**: `.glb` com ossos e animações (do Blender e do Mixamo) se deformam na GPU, com sombra. Também valem as animações de partes sem ossos (uma roda que gira, uma porta que abre). Até 128 ossos por modelo.
- **No Lua**: `modelo:animations()` e `modelo:animator()`, com `play(nome, { loop, blend, speed, restart })`, troca suave entre animações, `update`, `finished`, `bone` (para prender coisas num osso) e `nn.d3.draw(modelo, { animator = anim })`.
- **Herói e bicho animados nossos** (gerados por `ferramentas/gerar_modelos.py`): o herói tem parado, andar, correr, pular, cair e acenar; o bicho, parado e andar. Os exemplos 3D e os modelos de projeto 3D usam o herói animado.
- **Componente Animação 3D** no NN Studio: escolhe a animação do modelo (o ☰ lista as do arquivo), com velocidade e repetir, e a vista do editor toca a animação.
- O **Jogador 3D** anima o modelo sozinho (parado, andar, correr, pular e cair), com os nomes configuráveis.
- Eventos **Tocar animação** e **Animação terminar** também no 3D; no código, `obj:play(nome)` e a mensagem `animacao_terminou`.
- O tutorial **Seu primeiro mundo 3D** ganhou um bicho que anda.

### Som 3D e partículas 3D

- **Som 3D**: um som com posição (`nn.audio.play(som, { x, y, z, distance })`) fica mais baixo longe da câmera e vai para o lado em que está; o volume e o lado acompanham a câmera a cada quadro, e `som:set_position` leva o som junto. No NN Studio, o componente **Som** de uma cena 3D tem **Som no espaço 3D** (ligado) e **Ouvido até**, e o som anda com o objeto. **Tocar som** nos eventos ganha **saindo de** um objeto.
- **Partículas 3D**: `nn.particles3d`, em metros, com cone de direção, gravidade 3D e áreas (círculo deitado, esfera, caixa). Cada partícula é um quadrado virado para a câmera, na frente e atrás dos objetos certos e com névoa. As de somar luz vão juntas na GPU.
- No NN Studio, o componente **Partículas** funciona nas cenas 3D: oito efeitos prontos do 3D (fogo, fumaça, faíscas, explosão, poeira, brilho, chuva, neve), campos em metros, as partículas saindo para o "cima" do objeto, e a prévia animando na vista 3D (com o 💥 Soltar).
- Eventos **Soltar partículas**, **Ligar ou desligar partículas**, **Faíscas** e **Partículas terminarem** no 3D; o **Coletável** solta um brilho 3D.
- A ilha do modelo **3D** ganhou uma fogueira (fogo, fumaça e o estalar), o barulho das ondas nas praias e poeira nos pés do herói ao pular. Sons novos `fogueira.wav` e `mar.wav` nos modelos 3D (gerados por `ferramentas/gerar_ilha3d.py`, que também gera as cenas da ilha).
- Tutorial guiado **Efeitos no 3D**.

### Terreno

- **Componente Terreno**: uma grade de alturas (até 256 × 256 quadrados) com quatro cores pintadas, desenhada low-poly e com o chão exato da física (os mesmos triângulos). A **água** do terreno tem nível e cor, é meio transparente, e o **Jogador 3D** que cai nela recebe a mensagem `agua` e volta para a margem (ou anda no fundo, se escolher).
- **✏ Pintar terreno** no NN Studio: pincéis Subir, Descer, Alisar, Aplainar, Ruído, Pintar cor e **Espalhar** (modelos ou prefabs com tamanho e giro sorteados, já com Corpo 3D; Ctrl apaga). Tamanho e força, um círculo mostrando o pincel no chão, cada arrasto um passo do Ctrl+Z, e o jogo aberto muda na hora.
- No Lua: `nn.d3.terrain` com `height_at`, `brush`, `paint`, `raycast`, `encode`; `world:body({ terrain = t })`; `self:water_level(x, z)`. Malhas meio transparentes são desenhadas depois das outras, de trás para a frente.
- Modelo **Vale 3D** (`nn novo --modelo vale3d`): morros, lago, trilha, árvores espalhadas, estrelas nos morros e uma fogueira, gerado por `ferramentas/gerar_vale3d.py`.
- Tutorial guiado **Pinte um terreno**.

## 0.1.0

A primeira versão para baixar e instalar: o **NN Studio** (o editor), o `nn` (linha de comando) e o `nn-jogo` (o executável dos jogos exportados), para **Windows** e **Linux**.

### NN Studio, o editor

- **Gerenciador de projetos** com boas-vindas na primeira vez (fazer o tutorial, abrir um jogo pronto ou começar do zero), projetos recentes, exemplos e seis modelos: Em código, Plataforma, Aventura, Nave, Vazio e Mínimo.
- **Cenas visuais** (`.cena`) montadas com o mouse:
  - arrastar imagens, mapas e sons para a cena;
  - mover, girar e escalar com as alças;
  - Hierarquia com filhos e Inspetor com todos os componentes;
  - desfazer e refazer.
- **Comportamentos prontos**, sem código: jogador de plataforma e top-down, vida, patrulha, perseguir, perigo, coletável, checkpoint, ir para cena, movimento automático, animação automática, atirar, gerador e mostrar valor.
- **Prefabs**: um objeto pronto para usar em várias cenas. Mudar o prefab muda todas as cópias.
- **Eventos "Quando... Então..."**, lidos como frases. **Ver como código** mostra o Lua equivalente, e cada regra acende quando dispara no jogo aberto.
- **Testar daqui (F6) e ajuste ao vivo**:
  - o que muda no editor aparece no jogo aberto;
  - o editor mostra onde cada objeto está no jogo, pausa e avança quadro a quadro;
  - Ctrl+clique no jogo seleciona o objeto.
- **Editor de pixel art** (lápis, borracha, balde, formas, seleção, espelhado, paletas, quadros, papel vegetal) e **editor de animação** (`.anim`), com prévia.
- **Mapas de tiles** pintados direto na cena:
  - pincel, balde, retângulo e conta-gotas;
  - tiles girados e espelhados, camadas;
  - terrenos que se encaixam sozinhos;
  - painel Tiles para marcar o que bloqueia, plataformas, tipos e animações;
  - arquivos do Tiled (`.tmj`/`.tsj`).
- **Menus e HUD**:
  - componentes de interface: painel, botão, barra, ícones, deslizador, caixa de marcar, campo de texto e grupo que arruma os filhos;
  - âncoras nos cantos da tela e tema do projeto (cores, fonte `.ttf`, som);
  - cenas que abrem por cima de outra (pausa, opções), com a de baixo parada.
- **Partículas** animando na própria cena:
  - dez efeitos prontos e uma seta para a direção;
  - explosões em prefab que estouram e somem sozinhas;
  - ações para soltar e ligar ou desligar.
- **Editor de código Lua**: cores, erros enquanto digita, autocompletar da API `nn.*` com descrições, buscar e substituir.
- **Seis tutoriais guiados**, que destacam cada botão e avançam sozinhos: *Seu primeiro jogo*, *Eventos*, *Desenhe seu herói*, *Monte uma fase com tiles*, *Monte um menu* e *Efeitos com partículas*.
- **Configurações do projeto** num formulário: tamanho da janela e do jogo, variáveis do jogo e tema da interface.
- **Exportar o jogo** num executável único, para Windows ou Linux.
- **Modo Simples e Completo**, tema escuro e claro, paleta de comandos (Ctrl+Shift+P).
- **Log** em `nn-studio.log` e aviso quando o editor fechou por erro.
- **Aviso de versão nova** (pode ser desligado).

### Engine

- Renderização 2D com wgpu: sprites em lote, spritesheets, texto, painéis, partículas, fundos em camadas, câmera que segue, tremor e efeitos de tela.
- Lua 5.4 com a API `nn.*`: cenas (trocar, abrir por cima, transições), entrada (teclado, mouse, controle, ações), áudio com canais, física arcade, tilemaps, interface, progresso salvo.
- Mensagens de erro do Lua em português.
- Tamanho fixo do jogo, que se ajusta a qualquer janela (ajustar, esticar ou pixel art).
- Jogos também podem ser escritos em Rust, com os crates da engine.

### Para quem vai instalar

- **Windows:** instalador sem pedir administrador ou versão portátil em `.zip`. O instalador ainda não tem assinatura digital: o Windows avisa na primeira vez (veja o [README](README.md#baixar)).
- **Linux:** AppImage ou `.tar.gz`.
