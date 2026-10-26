#!/usr/bin/env bash
# E1: a prova (janela, desenho 2D, teclado e, com "som", o som) no navegador.
# Uso: e1-prova.sh [som]   (roda em estudo-web/)
set -euo pipefail
feat=${1:-}
nome=sem-som
args=()
if [ -n "$feat" ]; then nome=com-som; args=(--features "$feat"); fi
R=resultados
S=site/$nome
mkdir -p "$S"
s=$(date +%s)
if ! cargo build --release --target wasm32-unknown-unknown -p prova "${args[@]}" > "$R/e1-$nome-build.log" 2>&1; then
  echo "# E1 ($nome): não compila" > "$R/e1-$nome.md"
  grep -E '^error' -A14 "$R/e1-$nome-build.log" | head -80 >> "$R/e1-$nome.md"
  exit 1
fi
tempo=$(( $(date +%s) - s ))
wasm=target/wasm32-unknown-unknown/release/prova.wasm
wasm-bindgen --target web --no-typescript --out-dir "$S" "$wasm"
cru=$(stat -c %s "$S/prova_bg.wasm")
wasm-opt -Oz --all-features --strip-debug --strip-producers "$S/prova_bg.wasm" -o "$S/opt.wasm"
mv "$S/opt.wasm" "$S/prova_bg.wasm"
cp index.html "$S/"
tam() { stat -c %s "$1"; }
gz() { gzip -9c "$1" | wc -c; }
br() { brotli -q 11 -c "$1" | wc -c; }
kb() { echo "$(( $1 / 1024 )) KB"; }
w=$S/prova_bg.wasm
j=$S/prova.js
{
  echo "# E1 — prova ($nome)"
  echo
  echo "Compilou em $tempo s (sem cache: inclui as dependências)."
  echo
  echo "| arquivo | tamanho | gzip -9 | brotli 11 |"
  echo "|---|---|---|---|"
  echo "| wasm (do wasm-bindgen) | $(kb "$cru") | | |"
  echo "| wasm (wasm-opt -Oz) | $(kb "$(tam "$w")") | $(kb "$(gz "$w")") | $(kb "$(br "$w")") |"
  echo "| js | $(kb "$(tam "$j")") | $(kb "$(gz "$j")") | $(kb "$(br "$j")") |"
  echo
  echo "Os maiores pedaços do wasm final (twiggy; sem nomes, então só os tamanhos por seção e as maiores funções):"
  echo
  echo '```'
  twiggy top -n 12 "$w" 2>&1 | head -18 || true
  echo '```'
  echo
  echo "As maiores crates (twiggy no wasm antes do wasm-opt, que ainda tem os nomes):"
  echo
  echo '```'
  python3 crates.py "$wasm" 2>&1 | head -25 || true
  echo '```'
} > "$R/e1-$nome.md"
cat "$R/e1-$nome.md"
