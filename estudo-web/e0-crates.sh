#!/usr/bin/env bash
# E0: quais crates da engine compilam para um alvo web, sem mudar nada nelas.
# Uso: e0-crates.sh <alvo>   (roda na raiz do repositório)
alvo=$1
R=estudo-web/resultados/e0-$alvo.md
L=$(mktemp)
{
  echo "# E0 — as crates da engine em \`$alvo\`"
  echo
  echo "| crate | resultado | tempo |"
  echo "|---|---|---|"
} > "$R"
detalhes=""
for c in nn-core nn-input nn-render nn-audio nn-physics nn-physics3d nn-nav nn-tilemap nn-script nn-app nn-projeto nn-run; do
  s=$(date +%s)
  if cargo build --release --target "$alvo" -p "$c" > "$L" 2>&1; then
    echo "| $c | compila | $(( $(date +%s) - s )) s |" >> "$R"
  else
    echo "| $c | **não compila** | $(( $(date +%s) - s )) s |" >> "$R"
    detalhes+=$'\n'"## $c"$'\n\n```\n'"$(grep -E '^(error|warning: .*failed)' -A14 "$L" | head -70)"$'\n```\n'
  fi
done
echo "$detalhes" >> "$R"
cat "$R"
