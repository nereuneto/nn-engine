# O que mudou

As versões seguem o [versionamento semântico](https://semver.org/lang/pt-BR/). Antes da 1.0, uma versão nova pode mudar coisas de um jeito que pede ajustes nos projetos; quando isso acontecer, está escrito aqui.

## Não lançado

### Luzes (Sprint 39)

- **Luzes no 3D:** a **Luz** (componente novo) é uma **lâmpada** (ilumina em volta, até o **Alcance**) ou um **holofote** (ilumina num cone, para a frente do objeto), com cor e força, e pode **tremular** como fogo. Valem as 32 mais perto da câmera; só o sol faz sombra.
- **Brilho:** na **Forma 3D** e no **Modelo 3D**, a própria cor aparece acesa mesmo no escuro. Os modelos `.glb` com emissão no material (`emissiveFactor` e `KHR_materials_emissive_strength`) brilham sozinhos nessas partes.
- **No NN Studio:**
  - **➕ Objeto > Luz**, com **Lâmpada**, **Tocha** (a chama com o fogo e a luz tremulando) e **Holofote**;
  - as luzes acendem na vista 3D, e a escolhida mostra até onde chega;
  - **☀ Ambiente > Pronto**: **Dia**, **Entardecer**, **Noite** e **Caverna**, de uma vez.
- **Eventos:** **Acender ou apagar luz** e **Mudar a cor da luz**. Na ação **Ir para cena**, **Como: guardando esta (para voltar)**: a de agora fica como está, e **Fechar esta cena**, na outra, volta para ela.
- **Modelos novos:** `tocha.glb`, `lampiao.glb` e `cristal.glb`, com partes que brilham, nos modelos de projeto 3D. A pedra do `cajado.glb` também brilha.
- **O Vale 3D ganha uma caverna:** um arco de pedra no pé do morro leva a uma caverna escura, iluminada por tochas, cristais, um lampião e a luz que entra por uma fresta em cima da sexta estrela. O herói entra com uma tocha na mão; voltando, o vale continua de onde parou. Agora são 6 estrelas.
- **Tutorial guiado novo: Uma caverna iluminada** (10 minutos).
- **No Lua:** `nn.d3.lamp(opts)` (uma lâmpada ou um holofote no quadro), a opção `glow` do `nn.d3.draw`, `lamps` no `nn.d3.stats()` e o campo `obj.light3d`.
- Voltando de uma cena guardada (ou de uma aberta por cima), uma cena 3D põe de novo o céu, o sol e a névoa dela.

## 0.3.0

A versão do **combate**: inimigos que contornam paredes, buracos e água; uma espada na mão e golpes; tiros que miram pelo meio da tela; números de dano; ondas de inimigos; e chefes com vários ataques, tudo montado no **NN Studio sem código**. A ilha do modelo 3D ganhou uma arena com o besouro-rei, e há um tutorial guiado novo, **Um chefe**. Para começar: o guia [Comece aqui no 3D](docs/3d.md) e a seção [Combate](docs/studio.md#combate-espada-tiros-e-inimigos-que-atacam) do NN Studio.

### Para quem vem da 0.2

- **Os projetos da 0.2 abrem e rodam.** Algumas coisas ficaram diferentes no jogo:
  - **Atirar no 3D**, num modelo com a animação `atirar` (como o herói dos modelos), agora toca essa animação, e o tiro sai 0,08 s depois, com o braço levantado. Para o jeito de antes: **Animação** = nenhuma, ou **O tiro sai depois de** = 0.
  - **Vida** fica vermelha um instante a cada dano. Para desligar: **Fica vermelho ao levar dano**.
  - **Vida** com a variável padrão (**vidas**) só usa a variável no jogador. Num inimigo, ela é ignorada; antes, ele dividia as vidas com o herói. Para mostrar a vida de um inimigo numa variável, use outro nome.
  - Um **Inimigo 3D com um Golpe** não machuca mais só de encostar (o golpe machuca). Sem Golpe, nada muda.
- Os **Atirar** novos no 3D vêm com **para o meio da tela** e o **botão direito** do mouse. Os que já estão nas cenas continuam como estavam.
- Nenhuma função da API Lua nem comportamento pronto foi tirado ou mudou de nome. A Vida recebe agora `dano(n, quem)`; o `quem` é opcional.

### Revisão

- **Pausa no acerto:** quando um golpe acerta (do herói ou nele), o jogo para por uma fração de segundo, o que dá peso ao golpe. No Golpe, **Pausa no acerto** (0,06 s; 0 = sem pausa). No Lua, `scene:hitstop(segundos)` e a opção `hitstop` do `scene:hit`.
- **Inimigos em grupo se espalham:** vários Inimigos 3D atrás do herói se espalham em volta dele, em vez de um empurrar o outro. Fica mais bonito e mais leve: com 30 besouros na arena, a simulação gasta metade do tempo de antes (com 60, um quarto).
- **Mais leve com muitos objetos:** a lista de quem os comportamentos procuram (o jogador, um nome) é montada uma vez por passo, e não a cada pergunta.
- **Ícones** (o componente que mostra as vidas em corações) desenham no máximo 100. Antes, um número enorme na variável travava o jogo.
- **Sons novos**, nos modelos de projeto 3D: `golpe.wav` (a espada cortando o ar), `acerto.wav` (o baque) e `rugido.wav` (o besouro-rei se preparando). A ilha e o Vale 3D já usam.
- **Testes novos:**
  - em todo modelo de projeto, cada objeto começa no jogo onde o NN Studio mostra (teria pegado o erro das cópias de prefab da 0.2.1);
  - o repositório confere que todo crate está nas dependências do workspace com a mesma versão, e que o CHANGELOG tem a seção da versão.

### Caminhos

- **Inimigos que contornam paredes:** o Inimigo 3D e o Perseguir (voando, ou num jogo visto de cima) acham um caminho em volta de paredes, árvores, cenário e tiles sólidos, em vez de ir reto e ficar presos. No 3D, também não entram em buracos, barrancos nem na água dos terrenos. A opção **Contorna paredes** vem ligada (desligue para o jeito de antes).
- **Grade de caminhos** de cada cena, montada sozinha a partir dos corpos parados (e dos terrenos, no 3D) na primeira vez que alguém pede, e de novo quando um corpo parado aparece ou some. No 3D, vale o chão debaixo de uma plataforma no ar e uma ponte por cima da água.
- **🧭 Caminhos** no NN Studio (na barra da cena, com o jogo aberto): mostra no jogo o chão por onde dá para andar e o caminho que cada inimigo segue. Pelo terminal, `NN_CAMINHOS=1`.
- **No Lua:** `scene:path(a, b)`, `scene:steer(...)` (para comportamentos seus), `scene:nav()` com `nav:path`, `nav:walkable`, `nav:sight`, `nav:floor`, `nav:block` e `nav:unblock`, e `mundo:nav(opts)` no `nn.physics` e no `nn.physics3d`.
- No modelo **Aventura**, as lesmas (que perseguem voando) contornam o lago e a parede de tijolos; no **Vale 3D**, os besouros não entram mais no lago, e na **ilha**, não caem no mar.
- Crate novo `nn-nav` (a grade e o A*, sem depender do resto da engine).

### Combate 3D

- **Golpe** (comportamento novo, 3D): ataque de perto. O jogador golpeia com o clique (ou J, ou o X do controle); um inimigo, sozinho, quando o herói chega perto (para, vira e golpeia). Toca a animação `golpe` e, no instante do acerto, tira vida de quem está na frente (alcance e largura ajustáveis) e empurra. Com um Golpe, o Inimigo 3D não machuca mais só de encostar.
- **Atirar no 3D:** com **para o meio da tela** (o padrão no 3D), o tiro vai para onde a câmera olha, também para cima e para baixo, e pode sair de um osso (a mão). Uma mira aparece no meio da tela, e a câmera do Jogador 3D vai para o ombro (**Câmera de lado**). A tecla padrão no 3D é o botão direito do mouse (no controle, o RT ou o RB), e o herói toca a animação `atirar`.
- **Projétil** (comportamento novo, 3D), para o prefab do tiro: tira vida de quem acerta, empurra, some ao bater numa parede (com faíscas) e não acerta quem atirou; pode cair em arco ou atravessar.
- **Preso a um osso** (componente novo): uma espada ou um cajado na mão do personagem, que acompanha a animação. No NN Studio, a arma já aparece na mão na vista 3D, e a lista mostra os ossos do modelo do pai (vem com a mão direita).
- **Vida:** números do dano subindo (**Mostrar o dano**), o modelo fica vermelho um instante a cada dano e a câmera pode tremer (**Treme a câmera ao levar dano**). A ação **Tremer a câmera** também funciona no 3D.
- **Eventos:** **Acertar** e **Derrotar** (com o alvo); no **Levar dano**, o `@outro` é quem bateu.
- **Modelos:** o herói ganha os ossos das mãos (`mao_e`, `mao_d`) e as animações `golpe` e `atirar`; o besouro, a mordida (`golpe`). Novos: `espada.glb` e `cajado.glb`, nos modelos de projeto 3D.
- **Vale 3D:** o herói vem com a espada (clique para golpear); os besouros têm 2 de vida, com barra e números, e mordem; derrotar um dá 3 moedas. Traz também o `magia.prefab` (uma bola com Projétil, para um Atirar).
- **No Lua:** `scene:hit(alvo, dano, quem, opts)`, `scene:foes(quem)`, `scene:damage_number(obj, n)`, `scene:shake(forca, tempo)`, `anim:bones()`, o componente `bone` e a opção `shoulder` da câmera em terceira pessoa. A Vida recebe `dano(n, quem)` e manda `derrotou` para quem bateu por último.

### Chefes e ondas

- **Ondas de inimigos** (comportamento novo, 3D): cria um prefab de inimigo em ondas (onda 1, 2, 3...), cada uma com mais, com pausa entre elas. Os filhos do objeto são os pontos de onde eles saem (sem filhos, em volta, no chão onde dá para andar e longe do herói). Começa quando o jogador chega perto, com a cena ou por mensagem; mostra um letreiro ("Onda 2") e guarda a onda e quantos faltam em variáveis, para o placar. **➕ Objeto > Ondas de inimigos** já vem com quatro pontos.
- **Chefe** (comportamento novo, 3D): golpe, investida (fica tonto ao bater numa parede), pulo com impacto, tiros em leque e chamar ajudantes, cada um com um aviso antes. Fica furioso com pouca vida, é pesado, mostra o nome e a vida no alto da tela e pode deixar um prêmio ao cair.
- **Gerador no 3D:** cria num lugar do chão até **Espalhar até** metros, longe do jogador.
- **Eventos:** **Começar ou terminar uma onda** e **Vencer todas as ondas**.
- **Modelo novo:** o **besouro-rei** (`besourao.glb`), com parado, andar, golpe, preparar, pular, tonto e cair, nos modelos de projeto 3D.
- **A ilha (modelo 3D) ganhou uma arena** depois da ponte: 3 ondas de besouros e o besouro-rei, que guarda a sexta estrela. O herói da ilha também ganhou a espada.
- **Tutorial guiado novo: Um chefe** (15 minutos).
- **Vida:** a variável padrão ("vidas") só vale no jogador. Antes, um inimigo com Vida e a variável padrão dividia as vidas com o herói.
- **No Lua:** `scene:ground_spot(...)`, `scene:in_front(...)`, `scene:front3(obj)` e `scene:warn(texto)`.

## 0.2.2

Correção da 0.2.1.

- **Nas cenas 3D, as cópias de um prefab iam todas para z = 0** no jogo (no NN Studio elas apareciam no lugar certo). As moedas e as estrelas dos modelos **3D** e **Vale 3D**, por exemplo, ficavam enfileiradas longe da trilha e dos morros. Agora cada cópia fica onde está na cena: valem o Z, a rotação 3D (X, Y e Z) e a escala em Z dela. O mesmo para os prefabs criados pelo jogo com `scene:spawn(prefab, x, y, { z = ... })`: o **Atirar** no 3D soltava o tiro em z = 0 quando quem atirava estava em outro lugar.

## 0.2.1

Correção da 0.2.0.

- **Um clique do mouse numa cena 3D com eventos dava erro** (`nn/cena.lua: ... campo 'width'`) quando a cena tinha algum Corpo 3D. Isso acontecia já no primeiro clique na janela do jogo (o que prende o mouse), por exemplo no tutorial *Uma conversa* e nos modelos 3D. Agora não dá mais erro, e **Clique no objeto** funciona no 3D: vale o objeto que o mouse aponta, o mais perto da câmera (e os pais dele). O mesmo para o **Ctrl+clique** na janela do jogo, que seleciona o objeto no NN Studio.
- `objeto:bounds()` numa cena 3D devolve a metade da caixa 3D do objeto (em X e Y).

## 0.2.0

A versão do **3D**: jogos 3D em terceira pessoa, com visual low-poly colorido, montados no **NN Studio sem código** ou escritos em Lua. Cenas 3D com céu, sol, névoa e sombras; física; personagens animados do Blender e do Mixamo; terrenos pintados com o mouse; som e partículas no mundo; inimigos; portas, chaves e zonas; conversas com um editor visual; e seis tutoriais guiados novos. Para começar: o guia [Comece aqui no 3D](docs/3d.md).

### Para quem vem da 0.1

- **Os projetos da 0.1 abrem e rodam sem mudar nada.** Os seis modelos da 0.1 (Em código, Plataforma, Aventura, Nave, Vazio e Mínimo), criados com o `nn` da 0.1, rodam todas as cenas na 0.2, e o NN Studio grava as cenas e os prefabs deles exatamente como estavam. As preferências do editor também continuam.
- Nenhuma função da API Lua nem comportamento pronto foi tirado ou mudou de nome.
- **Para quem compila do código:** a engine passa a pedir o **Rust 1.89** (antes 1.88), porque a física 3D usa o `nalgebra` 0.35, que precisa dele. O NN Studio continua pedindo o 1.95. Com o rustup, `rustup update` resolve. Quem só baixa o NN Studio pronto não precisa de nada disso.
- Jogos exportados na 0.1 continuam funcionando como estão; para usar o que é novo, exporte de novo com a 0.2.

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

### Inimigos, vida e dano no 3D

- **Inimigo 3D** (`nn:inimigo3d`): patrulha (indo e voltando), vaga ou fica parado; persegue o jogador quando ele chega perto e desiste quando ele foge; machuca e empurra quem encosta; morre se pisarem em cima (com Vida, perde uma vida por pisão); anima o modelo (parado e andar) e troca de rumo quando fica preso.
- **Vida**, **Perigo**, **Atirar** e **Destruir depois** funcionam no 3D. A Vida ganhou a **barra de vida em cima** (2D e 3D). O Atirar 3D sai da frente do objeto (ou para onde a câmera olha), na altura escolhida, e o tiro some sozinho.
- Propriedades de comportamentos podem valer só no 2D ou no 3D (`mode`) e ter outro padrão no 3D (`default3d`); o Inspetor mostra só as que valem na cena.
- A ilha e o vale (modelos 3D) ganharam besouros, vida para o herói e corações no HUD.
- Tutorial guiado **Inimigos no 3D**.

### Portas, chaves e zonas

- **Porta 3D**: abre girando na dobradiça (para longe de quem abre), subindo, descendo ou deslizando; com **E** perto dela, sozinha quando o jogador chega ou só por mensagem. Fechada, bloqueia a passagem. Pode pedir uma chave e fechar sozinha depois de um tempo.
- **Inventário**: o **Coletável** guarda itens pelo nome (*chave*, *poção*), com um ícone; **Mostrar inventário** põe os itens num canto da tela. No Lua, `nn.inventory` (`add`, `remove`, `count`, `has`, `items`), que passa entre as cenas como as variáveis do jogo.
- **Interagir**: perto de um objeto aparece **E usar** (o texto você escolhe), e só no mais perto; apertando, ele avisa os eventos e outros objetos. Pode pedir um item. A Porta 3D usa o mesmo aviso.
- **Zona de gatilho**: uma caixa que aparece no editor e some no jogo; avisa quando o jogador entra ou sai (abre e fecha portas, liga armadilhas).
- **Texto 3D** (componente e `nn.d3.text`): textos no mundo, virados para a câmera ou presos como placas, com contorno e fundo, atrás das paredes e com névoa. O **Mostrar valor** escreve neles também.
- **Eventos novos**: Entrar numa zona, Sair de uma zona, Usar (apertar E), Ter um item, Ganhar item e Gastar item.
- **➕ Objeto > Portas e itens** no NN Studio (porta, chave, zona, Texto 3D, inventário), os modelos `porta.glb` e `chave.glb`, um cofre trancado com a chave numa plataforma no modelo **3D** (a ilha sem código), uma placa no **Vale 3D** e o tutorial guiado **Portas e chaves**.

### Diálogos

- **Editor de conversa** no NN Studio (arquivos `.dialogo`, em **➕ Novo > 💬 Diálogo**): cada fala é um cartão com quem fala e o texto; arrastar da bolinha de uma fala liga com outra (soltar no vazio cria uma fala nova); escolhas do jogador, cada uma levando a uma fala; personagens com cor, retrato e voz; velocidade do texto; desfazer e salvar.
- **Diálogo** (`nn:dialogo`, 2D e 3D): um personagem que conversa, com o aviso **E falar** (ou começando quando o jogador chega, quando a cena começa ou por mensagem); no 3D, vira para quem fala com ele. **➕ Objeto > Portas e itens > Personagem que fala** e o botão **💬 Editar a conversa** no Inspetor.
- **Caixa de fala** com o tema do projeto: o nome numa etiqueta da cor do personagem, o retrato, o texto letra por letra (com `{variavel}`) e as escolhas (setas, números ou mouse). Durante a conversa, o herói fica parado.
- Falas e escolhas podem mudar variáveis, dar itens e pedir um item para aparecer.
- **Eventos:** Começar uma conversa, Terminar uma conversa, Escolher numa conversa e a ação Começar conversa. No Lua: `scene:start_dialogue`, `scene:stop_dialogue` e `scene:dialogue`.
- Um pescador que dá dicas no modelo **3D**, um velho que conta onde está a chave no **Aventura** e o tutorial guiado **Uma conversa**.

### Desempenho e robustez do 3D

- **Só vai para a GPU o que a câmera vê**: o que fica fora da tela é cortado antes de desenhar, e o mapa de sombras só recebe o que fica perto da câmera. Numa cena com mil árvores, menos da metade é desenhada e um sexto vai para as sombras.
- **`nn.d3.stats()`** diz quantas malhas foram pedidas, vistas, postas na sombra e quantas chamadas de desenho houve.
- **Terrenos** não refazem mais a malha a cada quadro (só quando mudam), no jogo e no NN Studio.
- Menos trabalho por objeto a cada quadro: a matriz de quem está parado é reaproveitada e a física 3D não cria tabelas novas.
- **Arquivos que faltam não param o jogo:** um modelo que falta ou não abre vira um cubo rosa, um som fica mudo e uma conversa não abre, cada um com um aviso claro no console (o objeto, o componente e o arquivo).
- Testes de desempenho (`cargo test --release -p nn-run --test desempenho -- --ignored --nocapture`) e, no CI, testes que conferem o que vai para a GPU nas cenas 3D dos modelos e numa cena cheia.

### Revisão e acabamento

- **HUD na vista 3D:** o que fica preso à tela (placar, corações, avisos, menus) aparece por cima da cena, na moldura da tela do jogo, e dá para clicar e arrastar. **HUD**, na barra, mostra ou esconde. Antes, ele não aparecia, e o **Enquadrar** contava as posições dele (em pixels) como metros: a Ilha e o Vale abriam com a cena minúscula, lá longe.
- **➕ Objeto > Na tela (HUD)** na cena 3D: texto na tela e os componentes de interface. O **➕** da Hierarquia, numa cena 3D, mostra o mesmo menu da vista.
- A vista 3D **abre olhando para o Jogador 3D** de perto. De longe, a **Névoa** da vista desliga sozinha (marque de novo na barra; no jogo, a névoa continua). **Colisões** começa desligado no 3D.
- **Gerenciador:** as boas-vindas têm o caminho **3D** (o tutorial *Seu primeiro mundo 3D*, a ilha pronta e o Vazio 3D); os **tutoriais** ficam em abas **Jogos 2D** e **Jogos 3D**, com rolagem (antes, a lista passava da tela); no **Novo projeto**, os modelos ficam em dois grupos, 2D e 3D. As janelas não crescem mais aos poucos ao abrir.
- Barras das cenas e do pincel do terreno quebram a linha certinho (antes, o último controle ficava cortado na beirada). O editor de conversa enquadra conversas maiores, e a dica da barra quebra a linha.
- Partículas na prévia do editor não são mais refeitas a cada quadro (o editor comparava um texto montado com todos os campos).
- Tutorial *Inimigos no 3D*: os corações vêm de **➕ Objeto > Na tela (HUD) > Ícones**.
- **Documentação:**
  - guia novo **[Comece aqui no 3D](docs/3d.md)**: os primeiros 5 minutos, a ordem dos tutoriais, as peças prontas, modelos do Blender e do Mixamo, desempenho e o que fazer quando algo dá errado;
  - `studio.md`: *HUD e menus no 3D* e *O que vem depois*;
  - README com capturas do 3D;
  - os LEIAME dos modelos 3D falam dos besouros, da vida e do HUD.

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
