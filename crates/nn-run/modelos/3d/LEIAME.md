# {{TITULO}}

Um jogo de plataforma 3D em terceira pessoa, já jogando: ache as 5 estrelas
da ilha o mais rápido que puder. Comece daqui e transforme no seu jogo.

- **WASD / setas** ou **analógico esquerdo**: andar (para onde a câmera olha)
- **Espaço** ou **A**: pular
- **Mouse** ou **analógico direito**: girar a câmera (o mouse fica preso; **Esc** pausa e solta)

## Onde mexer primeiro

- `lib/ilha.lua`: a fase. As tabelas `PLATAFORMAS`, `MOVEIS` (plataformas que
  andam) e `ESTRELAS` no começo do arquivo; mude os números e salve, o jogo
  aberto atualiza na hora.
- `cenas/jogo.lua`: as regras. `VELOCIDADE` e `PULO` no começo; o que acontece
  ao pegar moeda e estrela está em `cena:fixed_update`.
- `lib/jogo.lua`: céu, luz, névoa, sombras e os controles.
- `modelos/`: os modelos 3D. Exporte do Blender como **glTF Binary (.glb)** e
  troque o arquivo (por exemplo, `modelos/heroi.glb` pelo seu personagem).

O guia "Seu primeiro jogo 3D" na documentação (docs/lua.md) explica tudo passo a passo.
