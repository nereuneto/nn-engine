# Estudo da versão web (S67)

Experimentos para decidir se a web entra no plano (v0.13). Ficam só neste
ramo; o relatório e as correções genéricas vão para o main.

- `e0-crates.sh`: quais crates da engine compilam para `wasm32-unknown-unknown`
  e `wasm32-unknown-emscripten`, sem mudar nada.
- `prova/`: janela (canvas), desenho 2D do nn-render, teclado e som (feature
  `som`) no navegador; `navegador.mjs` abre no Chromium sem tela (WebGL2 e
  WebGPU), usa o teclado e o mouse e tira fotos.
- `lua-*` + `teste.lua` + `bench.lua`: as opções de Lua (piccolo em Rust puro;
  mlua em C no emscripten, no wasi e no wasm32-unknown-unknown).

O workflow `.github/workflows/estudo-web.yml` roda tudo a cada push neste ramo
e grava os resultados em `resultados/`.
