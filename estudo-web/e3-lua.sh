#!/usr/bin/env bash
# E3: as opções de Lua na web. Uso: e3-lua.sh <parte>  (roda em estudo-web/)
# parte: desktop | piccolo | mlua-unknown | mlua-web | mlua-emscripten | mlua-wasi
set -uo pipefail
R=resultados/e3-lua-$1.md
L=$(mktemp)
kb() { echo "$(( $(stat -c %s "$1") / 1024 )) KB (brotli $(( $(brotli -q 11 -c "$1" | wc -c) / 1024 )) KB)"; }
erro() { echo "não compila:"; echo '```'; grep -E '^(error|warning: .*failed)' -A16 "$L" | head -60; echo '```'; }
case $1 in
  desktop)
    { echo "# E3 — Lua no desktop (base para comparar)"; echo
      cargo run -q --release -p lua-mlua 2>&1; echo
      cargo run -q --release -p lua-piccolo 2>&1; } > "$R" ;;
  piccolo)
    { echo "# E3 — piccolo (Rust puro) no wasm32-unknown-unknown, rodando no Node"; echo
      if RUSTFLAGS='--cfg getrandom_backend="wasm_js"' cargo build --release --target wasm32-unknown-unknown -p lua-piccolo --lib > "$L" 2>&1; then
        wasm-bindgen --target nodejs --out-dir pkg-piccolo target/wasm32-unknown-unknown/release/lua_piccolo.wasm
        echo "wasm: $(kb pkg-piccolo/lua_piccolo_bg.wasm)"; echo
        node -e 'console.log(require("./pkg-piccolo/lua_piccolo.js").rodar_js())' 2>&1
      else erro; fi; } > "$R" ;;
  mlua-unknown)
    { echo "# E3 — mlua (Lua 5.4 em C) no wasm32-unknown-unknown"; echo
      if cargo build --release --target wasm32-unknown-unknown -p lua-mlua > "$L" 2>&1; then echo "compila (inesperado)"; else erro; fi; } > "$R" ;;
  mlua-web)
    { echo "# E3 — mlua (Lua 5.4 em C) no wasm32-unknown-unknown com a libc do wasi-sdk, rodando no Node"; echo
      if cargo build --release --target wasm32-unknown-unknown -p lua-mlua-web > "$L" 2>&1; then
        w=target/wasm32-unknown-unknown/release/lua_mlua_web.wasm
        echo "wasm: $(kb $w)"; echo
        node lua-mlua-web.mjs $w 2>&1 | head -80 || node --experimental-wasm-exnref lua-mlua-web.mjs $w 2>&1 | head -80
      else erro; fi; } > "$R" ;;
  mlua-emscripten)
    { echo "# E3 — mlua (Lua 5.4 em C) no wasm32-unknown-emscripten, rodando no Node"; echo
      echo "emcc: $(emcc --version | head -1)"; echo
      if cargo build --release --target wasm32-unknown-emscripten -p lua-mlua > "$L" 2>&1; then
        d=target/wasm32-unknown-emscripten/release
        for f in $d/*.wasm $d/*.js; do echo "- $(basename $f): $(kb $f)"; done; echo
        node $d/lua-mlua.js 2>&1 | head -60
      else erro; fi; } > "$R" ;;
  mlua-wasi)
    { echo "# E3 — mlua (Lua 5.4 em C) no wasm32-wasip1 (wasi-sdk), rodando no wasmtime"; echo
      if cargo build --release --target wasm32-wasip1 -p lua-mlua > "$L" 2>&1; then
        w=target/wasm32-wasip1/release/lua-mlua.wasm
        echo "wasm: $(kb $w)"; echo
        wasmtime run -W exceptions=y $w 2>&1 | head -60 || wasmtime run $w 2>&1 | head -60
      else erro; fi; } > "$R" ;;
esac
cat "$R"
