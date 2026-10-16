# {{TITULO}}

Jogo de nave feito com a [nn-engine](https://github.com/nereuneto/nn-engine) no **NN Studio**, sem escrever código.

## Como é o jogo

Mova a nave com as setas ou WASD e atire com espaço. Cada asteroide destruído vale 10 pontos; encostar num asteroide tira uma vida. O recorde fica salvo no computador.

## Como foi feito

- `cenas/titulo.cena`: título, recorde e botões (**Botão**). Um evento zera as variáveis e **carrega o recorde do disco**.
- `cenas/jogo.cena` (sem gravidade): a nave tem **Jogador top-down**, **Atirar** (cria o prefab `tiro`), **Vida** e **Animação automática**. Um objeto escondido com **Gerador** cria asteroides; outro cria as estrelas do fundo. Os limites são corpos físicos invisíveis.
- `prefabs/asteroide.prefab`: **Perigo** e **eventos** dele mesmo: começa andando para a esquerda, some quando um tiro encosta (e soma pontos) e quando sai da tela. Mudar o prefab muda todos os asteroides.
- Os eventos da cena guardam o recorde (*ficar com o maior entre ela e {pontos}*), **salvam no disco** e vão para `cenas/fim.cena` quando a vida acaba.

Abra o prefab do asteroide e veja os eventos dele no painel **Eventos**. **Ver como código** mostra as mesmas regras em Lua.

Sem o editor, com o `nn` instalado, de dentro desta pasta: `nn .`
