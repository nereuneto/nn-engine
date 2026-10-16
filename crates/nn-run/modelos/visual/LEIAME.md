# {{TITULO}}

Jogo feito com a [nn-engine](https://github.com/nereuneto/nn-engine), montado no **NN Studio** sem escrever código.

## Jogar e editar

Abra esta pasta no NN Studio e abra a fase (`cenas/fase.cena`): ela aparece na aba da cena, com os objetos na Hierarquia e as propriedades no Inspetor. Arraste imagens do painel Arquivos para a cena, mova com o mouse e aperte **F6** para testar a partir dela.

O chão e as plataformas são um **mapa de tiles** (`mapas/fase.tmj`, objeto `mapa`). Selecione o mapa e clique em **✏ Pintar**: no painel **Tiles**, escolha o terreno **terra** e pinte; as bordas e a grama se encaixam sozinhas. A tábua é uma plataforma de mão única (dá para pular por baixo). Os tiles vêm de `assets/imagens/terreno.png`; o painel Tiles também marca quais bloqueiam, o tipo (lava, água...) e as animações.

Cada objeto faz o que faz por causa dos **comportamentos** dele (no Inspetor):

- `heroi`: Jogador de plataforma (anda e pula), Animação automática e Vida.
- `moeda`: é um **prefab** (`prefabs/moeda.prefab`) com Coletável e Animação. Arraste o prefab do painel Arquivos para a cena para pôr mais moedas; mudar o prefab muda todas.
- `inimigo`: Patrulha e Perigo (dá para derrotar pulando em cima).
- `plataforma_movel`: Movimento automático (sobe e desce, e leva o herói junto).
- `saida`: um **evento** (painel Eventos) leva para `cenas/fim.cena` quando o herói encosta nela, mas só depois de pegar todas as moedas. Outro evento, da cena, deixa a saída verde nessa hora.
- **Partículas:** o herói tem um filho `poeira` (Partículas de uma vez, sem soltar ao começar); um evento da cena solta a poeira quando ele pula do chão. Outro evento cria o prefab `brilho` (`prefabs/brilho.prefab`) onde a moeda estava: ele brilha e some sozinho.

As variáveis do jogo (`moedas`, `vidas`) ficam no `config.lua` e valem de uma cena para outra.

## Interface: HUD e menus

- O **HUD** da fase fica preso à tela: `vidas` (componente **Ícones**, um coração para cada vida, preso no canto de cima à esquerda), `moedas` (texto com Mostrar valor) e `progresso` (componente **Barra**, que enche com as moedas), presos no canto de cima à direita. No Inspetor, **Preso em** escolhe o canto: o HUD continua no canto em qualquer tamanho de janela.
- O título (`cenas/titulo.cena`) tem um **Grupo** que arruma três **Botões** em coluna: Jogar, Opções e Sair. Dá para escolher com o mouse, as setas ou o controle. O evento da cena zera as variáveis a cada jogo novo.
- **Esc** na fase abre a **pausa** (`cenas/pausa.cena`) **por cima**: a fase continua aparecendo, parada, até a pausa fechar.
- **Opções** (`cenas/opcoes.cena`) abre por cima do título ou da pausa, com **Deslizadores** para o volume da música e dos efeitos e uma **Caixa de marcar** para a tela cheia. As escolhas ficam guardadas para a próxima vez.
- As cores dos botões e painéis vêm do **tema da interface** (Configurações do projeto). Mude o tema e todos os menus mudam juntos.

## Eventos: "Quando... Então..."

No painel **Eventos** (ao lado do Console) ficam as regras do jogo, lidas como frases: *Quando o jogador encostar em este objeto e não sobrar nenhuma moeda, então ir para a cena fim*. Crie as suas com **➕ Regra**, sem escrever código. **Ver como código** mostra a mesma regra em Lua.

Com o jogo aberto, o que você muda no Inspetor ou arrastando na cena aparece no jogo na hora (indicador **Ao vivo**).

Sem o editor, com o `nn` instalado, de dentro desta pasta: `nn .`

## Quando quiser programar

No Inspetor, **Adicionar componente > Script do jogo > Novo script** cria um comportamento seu em `scripts/`, com propriedades que aparecem no Inspetor. Uma cena também pode ter um script (no Inspetor da cena).

## Distribuir

No NN Studio, **Jogo > Exportar jogo**. Ou, com o `nn`: `nn exportar .`
