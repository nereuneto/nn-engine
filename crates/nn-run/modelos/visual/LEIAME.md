# {{TITULO}}

Jogo feito com a [nn-engine](https://github.com/nereuneto/nn-engine), montado no **NN Studio** sem escrever código.

## Jogar e editar

Abra esta pasta no NN Studio: a fase (`cenas/fase.cena`) já abre na aba da cena, com os objetos na Hierarquia e as propriedades no Inspetor. Arraste imagens do painel Arquivos para a cena, mova com o mouse e aperte **F6** para testar a partir dela.

Cada objeto faz o que faz por causa dos **comportamentos** dele (no Inspetor):

- `heroi`: Jogador de plataforma (anda e pula), Animação automática e Vida.
- `moeda`: é um **prefab** (`prefabs/moeda.prefab`) com Coletável e Animação. Arraste o prefab do painel Arquivos para a cena para pôr mais moedas; mudar o prefab muda todas.
- `inimigo`: Patrulha e Perigo (dá para derrotar pulando em cima).
- `plataforma_movel`: Movimento automático (sobe e desce, e leva o herói junto).
- `saida`: Ir para cena (`cenas/fim.cena`).
- `placar`: texto fixo na tela (HUD) com Mostrar valor.

As variáveis do jogo (`moedas`, `vidas`) ficam no `config.lua` e valem de uma cena para outra.

Com o jogo aberto, o que você muda no Inspetor ou arrastando na cena aparece no jogo na hora (indicador **Ao vivo**).

Sem o editor, com o `nn` instalado, de dentro desta pasta: `nn .`

## Quando quiser programar

No Inspetor, **Adicionar componente > Script do jogo > Novo script** cria um comportamento seu em `scripts/`, com propriedades que aparecem no Inspetor. Uma cena também pode ter um script (no Inspetor da cena).

## Distribuir

No NN Studio, **Jogo → Exportar jogo**. Ou, com o `nn`: `nn exportar .`
