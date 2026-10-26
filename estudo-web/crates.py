"""Soma o tamanho das funções do wasm por crate (pelo nome, via twiggy)."""
import re, subprocess, sys
out = subprocess.run(["twiggy", "top", "-n", "100000", "--format", "csv", sys.argv[1]],
                     capture_output=True, text=True).stdout.splitlines()
total = {}
for line in out[1:]:
    parts = line.split(",", 3)
    if len(parts) < 4:
        continue
    nome, tam = parts[3], int(parts[1])
    m = re.search(r"<?([a-z_][a-z0-9_]*)\[[0-9a-f]+\]::", nome) or re.match(r'"?([a-z_][a-z0-9_]*)::', nome)
    chave = m.group(1) if m else ("(seção) " + nome[:40] if not "::" in nome else "(outros)")
    total[chave] = total.get(chave, 0) + tam
for k, v in sorted(total.items(), key=lambda x: -x[1])[:25]:
    print(f"{v/1024:9.0f} KB  {k}")
