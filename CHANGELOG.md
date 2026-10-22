# O que mudou

As versões seguem o [versionamento semântico](https://semver.org/lang/pt-BR/). Antes da 1.0, uma versão nova pode mudar coisas de um jeito que pede ajustes nos projetos; quando isso acontecer, está escrito aqui.

## Não lançado

### O modelador (Sprint 47)

- **Modelos 3D feitos no NN Studio:** **Arquivos > ➕ Novo > 🎲 Modelo 3D** cria um `.modelo` e abre a aba do **modelador**, com um cubo para começar. O modelo é feito de **peças** (paredes, telhado, cabeça...), cada uma com pontos e faces coloridas, no estilo low-poly.
- **Ao salvar, o `.glb` sai junto**, com o mesmo nome: é ele que vai para as cenas e para o jogo, e abre no Blender. Cada peça vira uma parte do arquivo, e cada cor, um material. A engine não mudou.
- **A vista anda como a da cena 3D:** botão direito + WASD voa, Alt + arrastar gira em volta, a rodinha aproxima e **F** enquadra.
- **Escolher** pontos (**1**), faces (**2**) ou peças inteiras (**3**): clique, **Shift** junta, e arrastar no vazio faz uma **caixa de seleção**. As alças são as da cena: **W** move, **E** gira e **R** escala. Arrastar o que está escolhido também move, de frente para a câmera.
- **Ferramentas:**
  - **⬆ Puxar (P):** tira um bloco das faces escolhidas, para fora ou para dentro. O mouse manda, o clique termina e o **Esc** desfaz.
  - **▣ Encolher (I):** faz uma face menor dentro de cada face (portas, janelas).
  - **🔗 Juntar (M):** junta os pontos escolhidos num só.
  - **🗑 Apagar (Del)**, **Duplicar peça (Ctrl+D)** e **Escolher tudo (Ctrl+A)**.
- **Formas prontas** para começar uma peça: cubo, cilindro, cone, pirâmide, esfera, plano e rampa, no **➕ Peça**, no botão direito da vista ou no **Shift+A**.
- **Cores:** clicar numa cor da paleta pinta o que está escolhido (faces ou a peça toda). Com **🎨 Pintar (B)**, cada clique pinta uma face, e com **Shift**, a peça inteira. Dá para guardar cores novas na paleta do modelo.
- **↔ Espelho X:** modela só uma metade, e a outra aparece copiada. Os pontos do meio ficam no meio. Desligar transforma a cópia em pontos e faces de verdade.
- **Peças:** a lista à direita mostra, esconde e renomeia as peças (duplo clique ou F2).
- **Na cena:** arrastar o `.modelo` do painel Arquivos para uma cena 3D cria o objeto com o `.glb`. Salvar o modelo de novo atualiza as cenas abertas. Trocar um `.glb` por fora (exportado do Blender, por exemplo) também atualiza agora.
- **Ctrl+Z / Ctrl+Y** desfazem e refazem no modelador, e a contagem de triângulos aparece na barra.
- **Tutorial novo:** **Modele uma casinha**, com paredes, uma porta afundada, um telhado vermelho e a casinha na cena.

### Modelar sem repetir (Sprint 48)

- **🔁 Repetir em linha e em círculo:**
  - **No modelador**, com as peças escolhidas: cercas, escadas, raios de roda, pedras em volta da fogueira.
  - **Na cena 2D e 3D**, pelo botão direito de um objeto: fileiras de árvores, moedas em arco.
  - A prévia aparece enquanto você mexe na quantidade, na distância, no raio, no eixo e em quanto da volta. **Aplicar** fica com as cópias num passo só de desfazer.
  - No modelador, o círculo pode ser **em volta do meio do modelo**.
- **Peças que se encaixam:** com o botão direito (ou o Shift+A) em cima de uma face, a peça nova nasce no meio dela, virada para fora.
- **Kits** no ➕ Peça: **Casa**, **Natureza**, **Personagem** e **Veículo**.
  - O telhado da Casa fica do tamanho da face.
  - Os braços e as pernas do Personagem giram no ombro e no quadril, prontos para o animador.
- **📚 Biblioteca com 10 modelos prontos:** árvore, pinheiro, pedra, cerca, casinha, baú, lampião, fogueira, personagem e carrinho.
  - O **Começar com** do ➕ Novo > Modelo 3D cria um modelo a partir de um deles.
  - No ➕ Peça, eles entram no modelo aberto, como peças.
- **⭐ Guardar na biblioteca:** as suas peças ficam para usar em qualquer modelo e qualquer projeto, numa pasta ao lado das preferências.
- **🔀 Criar variação:** uma cópia do modelo (com o `.glb`), aberta numa aba nova.
- **Cores:** o botão direito numa cor da paleta **troca ela no modelo todo** ou **escolhe as faces dela**.
- **Tutorial novo:** **Uma cerca e uma fogueira**.

### Esqueleto e animador (Sprint 49)

- **💀 Esqueleto** no modelador: **Humanoide**, **Quadrúpede** ou **Simples**.
  - O esqueleto se ajusta ao tamanho do modelo.
  - Cada peça vai para o osso do nome dela (ou para o mais perto) e **segue o osso inteira**, sem entortar.
  - O personagem da biblioteca e o kit Personagem já saem ligados.
- **💀 Ossos (4):** mostra os ossos e move as juntas. **Segue** troca o osso de uma peça, e a lista **Ossos** escolhe e renomeia.
- **🎬 Animar:** a linha do tempo, as animações do modelo, ▶ Tocar (Espaço), repetir ou não, e a duração.
  - **Posar:** girar um osso com as alças grava um quadro-chave no instante.
  - **Linha do tempo:** arrastar os quadros muda o lugar deles, e o botão direito apaga, copia e cola a pose.
- **Animações prontas:**
  - **Humanoide:** parado, andar, correr, pular, cair, golpe e acenar, com os nomes que o Jogador 3D, o Inimigo 3D e o Chefe já procuram;
  - **Quadrúpede:** parado, andar e correr;
  - **Simples:** girar, balançar e abrir.
- **Ajustes rápidos:** 🐇 Mais rápida, 🐢 Mais lenta, 💪 Mais forte e Mais fraca.
- **O `.glb` leva o esqueleto e as animações:** a Animação 3D e o Jogador 3D tocam elas na cena e no jogo, e a engine não mudou. O `.modelo` guarda os ossos e os quadros-chave.
- **O kit Personagem trocou os lados:** o "Braço esquerdo" e a "Perna esquerda" agora ficam no +X, que é o lado esquerdo de quem olha para a frente (+Z). Os modelos já feitos não mudam.
- **Tutorial novo:** **Anime seu personagem**.

## 0.5.0

A versão do **NN Studio mais gostoso de usar**. O **botão direito** funciona em todo lugar e cria onde você clicou. A **busca rápida (Shift+A)** acha o que criar pelo nome, até com erro de digitação. Duplicar ficou sem trabalho repetido (**Ctrl+D que repete o passo**, **Alt+arrastar**, **Shift+D**), e dá para **copiar e colar** objetos e componentes. O Inspetor edita **vários objetos de uma vez**, os **⭐ favoritos** guardam o que você monta toda hora, os **avisos rápidos com Desfazer** deixam errar sem medo, os painéis vazios ensinam e as **Preferências** trazem a cor de destaque, as animações e os sons. A engine não mudou: os jogos rodam igual. Para começar: [A aba da cena](docs/studio.md#a-aba-da-cena), [Inspetor](docs/studio.md#inspetor) e [Preferências](docs/studio.md#preferências).

### Para quem vem da 0.4

- **Os projetos da 0.4 abrem e rodam.** Conferido com os dez modelos de projeto da 0.4, também depois de abrir e gravar cada cena no NN Studio da 0.5. A 0.4 também roda as cenas gravadas pela 0.5.
- **A engine, a API Lua, os comportamentos prontos e as crates do Rust não mudaram**: só o NN Studio.
- **As preferências da 0.4 abrem na 0.5**, com o novo no padrão. Voltando para a 0.4, ela abre as da 0.5 e ignora o que não conhece.
- **O que muda sem você mexer:**
  - o botão direito na cena abre um menu (segurar e arrastar continua movendo a vista);
  - **Alt + arrastar** um objeto no 2D faz uma cópia;
  - o **Ctrl+D** repete o passo da duplicação anterior;
  - a Lâmpada e o Holofote do ➕ Objeto nascem no alto;
  - os avisos rápidos aparecem embaixo, no meio da janela.
- **Pasta nova, só se você usar:** os ⭐ favoritos ficam em `favoritos/`, dentro do projeto.

### Revisão (Sprint 46)

- **Chão grande no botão direito:** algo grande e baixo (como a ilha do modelo 3D, um cilindro de 28 × 4 × 28 m) conta como chão: o botão direito nele abre o menu de criar, e o menu dele vai para o fim. Passar o mouse por cima também não desenha mais a caixa dele, que cobria a vista inteira.
- **O menu do botão direito reabre** no ponto novo quando você clica com o botão direito em outro lugar com ele aberto. Antes, esse clique só fechava o menu.
- **Esc fecha o menu.** Antes, com o mouse na cena, o Esc limpava a seleção e o menu ficava aberto. Com um menu aberto, os atalhos da cena (Shift+A, Del, Ctrl+C...) também esperam.
- **🗑 Tirar dos favoritos**, no submenu ⭐ Favoritos: o arquivo vai para a lixeira, perguntando antes.
- O aviso de favorito ficou curto ("tocha guardado nos favoritos ⭐"); o caminho completo continua no Console.
- **Desempenho:** o NN Studio mede o mesmo da 0.4 numa cena 2D com 2.000 objetos e na ilha com o mouse passando por cima.
- **Documentação:** imagens novas do botão direito, da busca rápida, do Inspetor com vários objetos e das Preferências, e o "O que vem depois" agora fala do modelador e do animador.
- **Ferramentas de revisão** (`tests/revisao.rs`):
  - `uso_de_ponta_a_ponta_v05` monta uma cena só com o que é novo e grava as telas;
  - `ilha_quadros_por_segundo` mede o tempo de um quadro com a ilha aberta;
  - `capturas_da_v05_para_a_documentacao` grava as imagens da documentação.

### Amigável e divertido (Sprint 45)

- **Avisos rápidos** no canto de baixo: "1 objeto apagado", "3 objetos colados", "Salvo: fase.cena", "📦 Prefab criado", "⭐ Guardado nos favoritos" e os erros. Os de apagar, recortar e colar têm **Desfazer**, que só aparece enquanto a cena não mudou depois. Com o mouse em cima, o aviso espera.
- **Ao salvar**, a aba mostra um ✔ no lugar do • por um instante.
- **Objetos novos brilham**: o que acabou de nascer (criado, colado, duplicado) ganha um contorno amarelo que some em menos de um segundo, para você ver onde ele caiu.
- **Painéis vazios que ensinam:**
  - uma cena vazia mostra como começar, com **🔍 Buscar o que criar**;
  - o Console vazio explica o que aparece ali e tem **▶ Jogar**;
  - a Hierarquia de uma cena vazia também tem a busca.
- **💡 Dica do dia** no Início, com **Próxima dica**.
- **Comemorações:** a primeira vez que o jogo de um projeto roda, e a primeira exportação, ganham um aviso e um confete curtinho. Uma vez por projeto.
- **Preferências** (Exibir > Preferências):
  - tema e **cor de destaque** (azul, verde, roxo, laranja, rosa);
  - modo e tamanho da interface;
  - **animações**: desligadas, menus, avisos e brilhos ficam parados;
  - **comemorações**;
  - **sons da interface**, desligados no começo e gerados pelo próprio NN Studio;
  - limpar o Console ao jogar e avisar versões novas.
- Os rótulos com ✓ (que apareciam como um quadradinho) agora usam ✔.

### Menos trabalho repetido (Sprint 44)

- **Ctrl+D repete o passo:** duplicou e moveu a cópia? O próximo Ctrl+D põe a nova o mesmo tanto adiante. Uma cerca ou uma fileira sai com Ctrl+D, Ctrl+D, Ctrl+D…
- **Alt + arrastar** leva uma cópia e deixa o original. No 2D, arrastando o objeto ou a seta; no 3D, arrastando uma seta do Mover (Alt + arrastar no resto continua girando a câmera).
- **Shift+D:** a cópia segue o mouse até o clique (com Encaixar, na grade). Esc ou o botão direito desistem.
- **Copiar, recortar e colar objetos** (Ctrl+C, Ctrl+X, Ctrl+V), com os filhos, na cena e na Hierarquia. Na cena, o colado vai para onde o mouse está. Cola em outra cena do projeto e em outro NN Studio aberto. O botão direito no vazio tem **📋 Colar aqui**, e o menu de um objeto tem Copiar e Recortar.
- **Copiar e colar componentes:** o menu do componente (☰ ou o botão direito no nome) tem **📋 Copiar componente** e **📋 Colar valores**. Embaixo do ➕ Adicionar componente, **📋 Colar componente** põe uma cópia no objeto.
- **Vários objetos no Inspetor:** com vários selecionados, ele mostra a Transformação e os componentes que todos têm. O que você muda vale para todos, só no campo mexido. Um **≠** marca os campos com valores diferentes. Adicionar, colar ou remover um componente também vale para todos.
- **⭐ Favoritos:** no botão direito de um objeto, **⭐ Guardar nos favoritos** guarda um molde em `favoritos/`. Ele aparece no topo do ➕ Objeto, do botão direito e do Shift+A. Diferente do prefab, cada cópia é livre.
- **Tutorial:** o **Seu primeiro mundo 3D** tem um passo novo, **Uma fileira de árvores**.

### Botão direito e criar rápido (Sprint 43)

- **Botão direito na cena** (2D e 3D): clicar e soltar, sem arrastar, abre um menu. No vazio, ele cria o objeto **onde você clicou**, e não mais no meio da vista. Em cima de um objeto, mostra o menu dele: Renomear, Duplicar, ⛶ Focar, **Copiar e Colar posição**, **➕ Criar dentro**, Criar filho vazio, 📦 Criar prefab, Excluir e **➕ Criar aqui**. Num chão (plano, terreno, mapa de tiles), o menu é de criar em cima. Arrastar com o botão direito continua movendo a vista (e, no 3D, voando); pintando tiles, ele continua pegando o tile.
- **Busca rápida para criar (Shift+A):** com o mouse na cena, abre uma caixinha ali. Digite *luz*, *cubo*, *inimigo*, o nome de um prefab, de um modelo ou de uma imagem e aperte **Enter**: o objeto nasce onde o mouse estava. Acha sem acento, por outras palavras (*bloco* acha o Cubo; *lago*, a Água) e com um erro de digitação (*tocah* acha a Tocha). Os últimos que você usou vêm primeiro.
- **Hierarquia:** o botão direito no espaço vazio cria um objeto, e o menu de um objeto é o mesmo da cena, com **➕ Criar dentro** (o objeto novo já nasce filho).
- **Inspetor:** o botão direito no nome de um componente abre o menu dele (o mesmo do ☰), que agora também tem **↺ Restaurar o padrão**: os valores voltam ao começo, sem mudar o arquivo, a forma ou o tipo.
- **F2 na cena** renomeia o objeto selecionado, na Hierarquia (que rola até ele).
- **Os atalhos aparecem à direita** nos menus (F2, Ctrl+D, F, Del), e a lista de **Ajuda > Atalhos do teclado** tem os novos.
- A **Lâmpada** e o **Holofote** do ➕ Objeto nascem no alto (2 m e 4 m acima do chão), e não mais no chão.
- **Tutorial:** o **Seu primeiro mundo 3D** tem um passo novo, **Criar com o botão direito**.

## 0.4.0

A versão dos **mundos 3D mais vivos**: **luzes** (tochas que tremulam, lâmpadas e holofotes) e coisas que **brilham** no escuro; **dia e noite**, com o sol e a lua cruzando o céu, estrelas e luzes que acendem sozinhas; **água** com ondas que reflete o céu; **vento** que balança árvores, grama e bandeiras; e o **halo** em volta do que brilha. Tudo no **NN Studio sem código**. O Vale 3D ganhou uma caverna escura, a ilha tem dia e noite, mar com ondas e uma bandeira na arena, e há três tutoriais guiados novos: **Uma caverna iluminada**, **Dia e noite** e **Um lago com vida**. Para começar: [Luzes e brilho](docs/studio.md#luzes-e-brilho), [Ambiente](docs/studio.md#ambiente) e [Água, vento e halo](docs/studio.md#água-vento-e-halo).

### Para quem vem da 0.3

- **Os projetos da 0.3 abrem e rodam.** Conferido com os nove modelos de projeto da 0.3, também depois de abrir e gravar cada cena no NN Studio da 0.4. Algumas coisas ficam diferentes no jogo sem você mexer:
  - o **Halo do brilho** vem ligado: o fogo e o disco do sol ganham um halo suave. Para o jeito de antes, **Halo do brilho** = 0 (☀ Ambiente);
  - a **água dos Terrenos** tem ondas e reflete o céu;
  - **partículas no 3D** que não somam luz (fumaça, poeira) recebem a luz do mundo: num ambiente escuro, ficam escuras.
- Uma cena gravada pela 0.4 pode usar coisas que a 0.3 não conhece (a **Luz**, o **Dia e noite**). Para voltar à 0.3, use uma cópia do projeto.
- Nenhuma função da API Lua nem comportamento pronto foi tirado ou mudou de nome.
- **No Rust:** `ModelPart`, `ParticleConfig3`, `Sky3D` e `Stats3` ganharam campos (`emissive`, `wind`, `stars`, `lamps` e `halo`). Quem monta essas structs com `..Default::default()` não precisa mudar nada.

### Revisão (Sprint 42)

- **O halo só é desenhado quando algo brilha na tela:** o Brilho, a água, o fogo, o sol ou a lua. Numa cena sem nada disso, ele não pesa. `nn.d3.stats().halo` diz se foi desenhado.
- **Mais leve:** a água faz a conta da sombra uma vez só, e o que está todo na névoa (o mar lá longe) não é mais iluminado. Sem placa de vídeo, a ilha ficou uns 10 ms mais rápida por quadro, em 1280 × 720.
- **Contorno nos textos da tela:** no Inspetor do **Texto**, **Contorno** põe uma borda em volta das letras, para o texto ler bem em cima do céu (`outline` no componente). Os placares e as dicas da ilha e do Vale 3D usam.
- **O fogo inclina com o vento**, em vez de voar com ele. O pronto **Fogo** e a **Tocha** vêm com **Levadas pelo vento** = 0,3, e o fogo tem um halo um pouco mais fraco. As fogueiras da ilha e do vale ficaram menores, sem virar uma bola branca.
- **Ferramentas novas:**
  - `tempos_da_ilha` mede quanto cada coisa pesa na ilha: o halo, o vento, a água, a grama e as sombras;
  - `tempos_da_caverna` mede também sem o halo.
- **Documentação:** o "O que vem depois" do guia do NN Studio foi atualizado, e há imagens novas da ilha, do vale, da caverna e do NN Studio.

### Água, vento e brilho (Sprint 41)

- **Água:** na Forma 3D, **Material: Água**, com ondinhas que se mexem, o céu refletido (mais forte olhando de lado) e o brilho do sol batendo. **➕ Objeto > Água (lago)** traz um plano assim. A água dos **Terrenos** também ganhou ondas e reflexo.
- **Vento** (☀ Ambiente): a força e o lado para onde sopra. O que tem **Balança com o vento** (na Forma 3D e no Modelo 3D) balança com rajadas: embaixo parado, em cima mais. As **partículas** no 3D são levadas pelo vento (**Levadas pelo vento**, no Inspetor delas).
- **Halo do brilho** (☀ Ambiente): um halo suave em volta do que brilha (o Brilho, as partes que brilham nos `.glb`, o fogo, o sol e a lua). Vem ligado; 0 tira.
- Os **Ambientes prontos** não mexem mais no Dia e noite, no Vento nem no Halo.
- **Modelo novo:** `grama.glb` (um tufo), nos modelos de projeto 3D.
- **A ilha (modelo 3D):** o mar tem ondas e reflete o céu; árvores e grama balançam com o vento, e há uma bandeira na arena. **No Vale 3D:** o lago tem ondas, e a grama balança. Na caverna, tochas e cristais ganham o halo.
- **Tutorial guiado novo: Um lago com vida** (10 minutos).
- **No Lua:** `nn.d3.wind(opts)`, `nn.d3.bloom(v)`, as opções `material = "water"` e `sway` do `nn.d3.draw`, `emissor:set_wind(x, y, z)`, os campos `wind`, `wind_turn` e `bloom` do `environment`, `material` e `sway` nas formas e modelos e `wind` nas partículas.
- **No Rust:** `Wind3D`, `Bloom` e `Material3D` no `nn-render`, com `Renderer::set_wind3d`, `set_bloom3d`, `draw_mesh_material` e `draw_model_material`.

### Dia e noite (Sprint 40)

- **Dia e noite** (☀ Ambiente): a hora anda no jogo. O sol nasce a leste às 6h, cruza o céu e se põe a oeste às 18h; de noite, a lua dá uma luz azulada fraca, e aparecem estrelas. As cores passam pelo amanhecer, pelo dia (as cores da cena), pelo entardecer e pela noite. **Um dia dura** quantos minutos, e **Começa às**, a hora do começo. No NN Studio, a vista mostra a cena nessa hora.
- **O relógio:** as variáveis **relogio** ("21:30"), **hora** (o número) e **dia**.
- **Só à noite**, na Luz, e **Brilha só à noite**, na Forma 3D e no Modelo 3D (o brilho do arquivo também): acendem sozinhos ao anoitecer. Sem o ciclo, "de noite" é um ambiente escuro, como a Noite e a Caverna.
- **Eventos:** **Amanhecer ou anoitecer** e **Estiver de dia ou de noite** (Quando); **Mudar a hora**, **Parar ou continuar o tempo** e **Mudar o ambiente** (para um pronto, aos poucos), em Então.
- **Ambiente pronto novo:** **Amanhecer**. A **Noite** ficou um pouco mais escura, para as luzes aparecerem melhor.
- **A ilha (modelo 3D) tem dia e noite:** um dia de 4 minutos, começando às 9h. Postes com lampião no caminho e na ponte acendem sozinhos de noite, o pescador acende o lampião na mão, a fogueira ilumina em volta e o placar mostra a hora.
- **Partículas no 3D** (fumaça, poeira) recebem a luz do mundo: de noite, ficam escuras (ou da cor da tocha perto). O fogo, que soma luz, continua aceso.
- **Tutorial guiado novo: Dia e noite** (10 minutos).
- **No Lua:** `scene:time_of_day()`, `scene:set_time(h)`, `scene:pause_time(sim)`, `scene:environment(pronto, segundos)` e `scene:is_night()`; o `environment` ganha `cycle`, `day_length` e `start_hour`; `nn.d3.sky` ganha `stars`; e `nn.d3.draw`, `emissive = false`.

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
