"""Soma o tamanho das funções do wasm por crate (pelo nome, via twiggy)."""
import csv, io, re, subprocess, sys
out = subprocess.run(["twiggy", "top", "-n", "1000000", "--format", "csv", sys.argv[1]],
                     capture_output=True, text=True).stdout
total = {}
for row in csv.DictReader(io.StringIO(out)):
    nome = row.get("Name") or row.get("name") or ""
    try:
        tam = int(row.get("ShallowSize") or row.get("shallow_size") or 0)
    except ValueError:
        continue
    m = re.search(r"([a-z_][a-z0-9_]*)\[[0-9a-f]+\]::", nome)
    chave = m.group(1) if m else ("(seção) " + nome[:40] if "::" not in nome else "(outros)")
    total[chave] = total.get(chave, 0) + tam
for k, v in sorted(total.items(), key=lambda x: -x[1])[:25]:
    print(f"{v/1024:9.0f} KB  {k}")
