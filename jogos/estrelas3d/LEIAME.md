# Ilha das Estrelas

Um jogo de plataforma 3D curto, em terceira pessoa: ache as 5 estrelas o mais
rápido que puder. Feito em Lua com a nn-engine.

- **WASD / setas** ou **analógico esquerdo**: andar (para onde a câmera olha)
- **Espaço** ou **A**: pular
- **Mouse** ou **analógico direito**: girar a câmera (o mouse fica preso; **Esc** pausa e solta)
- **Rodinha**: aproximar e afastar a câmera

## Onde está cada coisa

- `cenas/titulo.lua`, `cenas/jogo.lua`, `cenas/pausa.lua`, `cenas/fim.lua`: as telas.
- `lib/ilha.lua`: a fase. Mude as tabelas `PLATAFORMAS`, `MOVEIS` e `ESTRELAS` para fazer outra.
- `lib/jogo.lua`: controles, menus, céu, luz e opções da câmera.
- `modelos/`: os modelos 3D (`.glb`). Troque por modelos seus, do Blender por exemplo.
