#!/usr/bin/env python3
"""Gera os modelos 3D low-poly (.glb) do exemplo cubos3d e dos testes.

Só usa a biblioteca padrão do Python. Os modelos são nossos (sem licença de
terceiros) e servem também de exemplo de arquivo glTF como os do Blender:
várias partes com materiais, nós com posição/rotação/escala e uma textura.

    python3 ferramentas/gerar_modelos.py

Escreve em jogos/cubos3d/modelos/ e crates/nn-render/tests/modelos/.
"""

import json
import math
import os
import struct
import zlib

RAIZ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


# --- Geometria -------------------------------------------------------------

def sub(a, b): return (a[0] - b[0], a[1] - b[1], a[2] - b[2])
def cross(a, b): return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])
def norm(a):
    l = math.sqrt(a[0] ** 2 + a[1] ** 2 + a[2] ** 2) or 1.0
    return (a[0] / l, a[1] / l, a[2] / l)


class Parte:
    """Triângulos com uma cor (material) só; normais por face (low-poly)."""

    def __init__(self, cor, textura=False):
        self.cor = cor
        self.textura = textura
        self.pos, self.nor, self.uv, self.idx = [], [], [], []

    def tri(self, a, b, c, uvs=((0, 0), (1, 0), (0, 1))):
        n = norm(cross(sub(b, a), sub(c, a)))
        base = len(self.pos)
        for p, uv in zip((a, b, c), uvs):
            self.pos.append(p)
            self.nor.append(n)
            self.uv.append(uv)
        self.idx += [base, base + 1, base + 2]

    def quad(self, a, b, c, d):
        """a, b, c, d no sentido anti-horário vistos de fora."""
        self.tri(a, b, c, ((0, 1), (1, 1), (1, 0)))
        self.tri(a, c, d, ((0, 1), (1, 0), (0, 0)))

    def caixa(self, cx, cy, cz, sx, sy, sz):
        x0, x1 = cx - sx / 2, cx + sx / 2
        y0, y1 = cy - sy / 2, cy + sy / 2
        z0, z1 = cz - sz / 2, cz + sz / 2
        self.quad((x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1))  # frente
        self.quad((x1, y0, z0), (x0, y0, z0), (x0, y1, z0), (x1, y1, z0))  # trás
        self.quad((x1, y0, z1), (x1, y0, z0), (x1, y1, z0), (x1, y1, z1))  # direita
        self.quad((x0, y0, z0), (x0, y0, z1), (x0, y1, z1), (x0, y1, z0))  # esquerda
        self.quad((x0, y1, z1), (x1, y1, z1), (x1, y1, z0), (x0, y1, z0))  # cima
        self.quad((x0, y0, z0), (x1, y0, z0), (x1, y0, z1), (x0, y0, z1))  # baixo

    def prisma(self, lados, raio, y0, y1, raio_topo=None, giro=0.0):
        """Cilindro facetado (ou cone/tronco com raio_topo)."""
        rt = raio if raio_topo is None else raio_topo
        def ponto(i, r, y):
            a = giro + 2 * math.pi * i / lados
            return (math.cos(a) * r, y, -math.sin(a) * r)
        for i in range(lados):
            a0, a1 = ponto(i, raio, y0), ponto(i + 1, raio, y0)
            b0, b1 = ponto(i, rt, y1), ponto(i + 1, rt, y1)
            if rt > 1e-6:
                self.quad(a0, a1, b1, b0)
                self.tri((0, y1, 0), b0, b1)
            else:
                self.tri(a0, a1, (0, y1, 0))
            self.tri((0, y0, 0), a1, a0)

    def pedra(self, semente):
        """Icosaedro amassado."""
        t = (1 + math.sqrt(5)) / 2
        v = [(-1, t, 0), (1, t, 0), (-1, -t, 0), (1, -t, 0), (0, -1, t), (0, 1, t),
             (0, -1, -t), (0, 1, -t), (t, 0, -1), (t, 0, 1), (-t, 0, -1), (-t, 0, 1)]
        f = [(0, 11, 5), (0, 5, 1), (0, 1, 7), (0, 7, 10), (0, 10, 11), (1, 5, 9), (5, 11, 4),
             (11, 10, 2), (10, 7, 6), (7, 1, 8), (3, 9, 4), (3, 4, 2), (3, 2, 6), (3, 6, 8),
             (3, 8, 9), (4, 9, 5), (2, 4, 11), (6, 2, 10), (8, 6, 7), (9, 8, 1)]
        s = semente
        pts = []
        for p in v:
            s = (s * 1103515245 + 12345) % 2147483648
            k = 0.75 + 0.35 * (s / 2147483648)
            n = norm(p)
            pts.append((n[0] * 0.5 * k, n[1] * 0.5 * k * 0.7, n[2] * 0.5 * k))
        for a, b, c in f:
            self.tri(pts[a], pts[b], pts[c])


def cor(hexa):
    """#RRGGBB (sRGB) → fator linear do glTF."""
    def lin(c):
        c /= 255
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4
    h = hexa.lstrip('#')
    return [lin(int(h[i:i + 2], 16)) for i in (0, 2, 4)] + [1.0]


# --- PNG ------------------------------------------------------------------

def png(w, h, pixels):
    raw = b''.join(b'\x00' + bytes(sum((list(p) for p in pixels[y * w:(y + 1) * w]), [])) for y in range(h))
    def chunk(t, d):
        return struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b''))


def madeira():
    """Tábuas de caixote, 16×16, em pixel art."""
    claro, medio, escuro = (196, 140, 82, 255), (166, 112, 62, 255), (110, 70, 38, 255)
    px = []
    for y in range(16):
        for x in range(16):
            if x in (0, 15) or y in (0, 15):
                px.append(escuro)
            elif (x + y) % 15 == 0 or x - y == 0:
                px.append(escuro)
            elif y % 5 == 2:
                px.append(medio)
            else:
                px.append(claro)
    return png(16, 16, px)


# --- glTF ------------------------------------------------------------------

class Modelo:
    def __init__(self):
        self.bin = bytearray()
        self.json = {"asset": {"version": "2.0", "generator": "nn-engine gerar_modelos.py"},
                     "scene": 0, "scenes": [{"nodes": []}], "nodes": [], "meshes": [],
                     "materials": [], "accessors": [], "bufferViews": []}
        self.textura = None

    def _view(self, data, target=None):
        while len(self.bin) % 4:
            self.bin.append(0)
        v = {"buffer": 0, "byteOffset": len(self.bin), "byteLength": len(data)}
        if target:
            v["target"] = target
        self.bin += data
        self.json["bufferViews"].append(v)
        return len(self.json["bufferViews"]) - 1

    def _acc(self, valores, tipo, n):
        flat = [c for v in valores for c in v]
        view = self._view(struct.pack('<%df' % len(flat), *flat), 34962)
        a = {"bufferView": view, "componentType": 5126, "count": len(valores), "type": tipo}
        if tipo == "VEC3":
            a["min"] = [min(v[i] for v in valores) for i in range(3)]
            a["max"] = [max(v[i] for v in valores) for i in range(3)]
        self.json["accessors"].append(a)
        return len(self.json["accessors"]) - 1

    def com_textura(self, png_bytes, uri=None):
        if uri:
            self.json["images"] = [{"uri": uri}]
        else:
            self.json["images"] = [{"bufferView": self._view(png_bytes), "mimeType": "image/png"}]
        self.json["samplers"] = [{"magFilter": 9728, "minFilter": 9728}]  # pixel art
        self.json["textures"] = [{"source": 0, "sampler": 0}]

    def malha(self, nome, partes):
        prims = []
        for p in partes:
            # Cada parte é fechada e com os triângulos virados para fora:
            # volume com sinal positivo.
            vol = 0.0
            for i in range(0, len(p.idx), 3):
                a, b, c = (p.pos[p.idx[i + k]] for k in range(3))
                vol += (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                        + a[2] * (b[0] * c[1] - b[1] * c[0])) / 6
            assert vol > 0, "%s: parte %s com faces para dentro (volume %f)" % (nome, p.cor, vol)
            mat = {"name": nome + "-" + str(len(self.json["materials"])),
                   "pbrMetallicRoughness": {"baseColorFactor": cor(p.cor), "metallicFactor": 0.0}}
            if p.textura:
                mat["pbrMetallicRoughness"]["baseColorTexture"] = {"index": 0}
            self.json["materials"].append(mat)
            idx = self._view(struct.pack('<%dI' % len(p.idx), *p.idx), 34963)
            self.json["accessors"].append({"bufferView": idx, "componentType": 5125,
                                           "count": len(p.idx), "type": "SCALAR"})
            indices = len(self.json["accessors"]) - 1
            attrs = {"POSITION": self._acc(p.pos, "VEC3", 3), "NORMAL": self._acc(p.nor, "VEC3", 3)}
            if p.textura:
                attrs["TEXCOORD_0"] = self._acc(p.uv, "VEC2", 2)
            prims.append({"attributes": attrs, "indices": indices,
                          "material": len(self.json["materials"]) - 1})
        self.json["meshes"].append({"name": nome, "primitives": prims})
        return len(self.json["meshes"]) - 1

    def no(self, nome, malha=None, filhos=(), raiz=False, **t):
        n = {"name": nome}
        if malha is not None:
            n["mesh"] = malha
        if filhos:
            n["children"] = list(filhos)
        n.update(t)
        self.json["nodes"].append(n)
        i = len(self.json["nodes"]) - 1
        if raiz:
            self.json["scenes"][0]["nodes"].append(i)
        return i

    def glb(self):
        while len(self.bin) % 4:
            self.bin.append(0)
        self.json["buffers"] = [{"byteLength": len(self.bin)}]
        j = json.dumps(self.json, separators=(',', ':')).encode()
        j += b' ' * ((4 - len(j) % 4) % 4)
        total = 12 + 8 + len(j) + 8 + len(self.bin)
        return (struct.pack('<III', 0x46546C67, 2, total) + struct.pack('<II', len(j), 0x4E4F534A) + j
                + struct.pack('<II', len(self.bin), 0x004E4942) + bytes(self.bin))

    def gltf(self, bin_uri):
        while len(self.bin) % 4:
            self.bin.append(0)
        self.json["buffers"] = [{"byteLength": len(self.bin), "uri": bin_uri}]
        return json.dumps(self.json, indent=1).encode(), bytes(self.bin)


def quat_y(graus):
    a = math.radians(graus) / 2
    return [0.0, math.sin(a), 0.0, math.cos(a)]


# --- Os modelos ---------------------------------------------------------------

def arvore():
    m = Modelo()
    tronco = Parte("#8B5A3C")
    tronco.prisma(6, 0.15, 0.0, 0.9)
    copa = Parte("#3E9B4F")
    copa.prisma(7, 0.75, 0.0, 1.2, raio_topo=0.0)
    copa2 = Parte("#4FB06A")
    copa2.prisma(7, 0.55, 0.0, 0.9, raio_topo=0.0, giro=0.4)
    t = m.no("tronco", m.malha("tronco", [tronco]))
    c1 = m.no("copa", m.malha("copa", [copa]), translation=[0, 0.7, 0])
    c2 = m.no("copa2", m.malha("copa2", [copa2]), translation=[0, 1.35, 0])
    m.no("arvore", filhos=[t, c1, c2], raiz=True)
    return m


def casa():
    m = Modelo()
    paredes = Parte("#F2E3C6")
    paredes.caixa(0, 0.6, 0, 2.0, 1.2, 1.6)
    telhado = Parte("#C8553D")
    # Telhado de duas águas (prisma triangular), um pouco maior que a casa.
    x0, x1, z0, z1, y0, y1 = -1.15, 1.15, -0.95, 0.95, 1.2, 2.0
    telhado.quad((x0, y0, z1), (x1, y0, z1), (x1, y1, 0), (x0, y1, 0))
    telhado.quad((x1, y0, z0), (x0, y0, z0), (x0, y1, 0), (x1, y1, 0))
    telhado.tri((x1, y0, z1), (x1, y0, z0), (x1, y1, 0))
    telhado.tri((x0, y0, z0), (x0, y0, z1), (x0, y1, 0))
    telhado.quad((x0, y0, z0), (x1, y0, z0), (x1, y0, z1), (x0, y0, z1))
    porta = Parte("#6B4226")
    porta.caixa(0, 0.4, 0.81, 0.45, 0.8, 0.04)
    janela = Parte("#7FC8F8")
    janela.caixa(-0.6, 0.75, 0.81, 0.35, 0.3, 0.04)
    janela.caixa(0.6, 0.75, 0.81, 0.35, 0.3, 0.04)
    chamine = Parte("#9A8478")
    chamine.caixa(0.55, 1.85, -0.3, 0.25, 0.6, 0.25)
    corpo = m.no("corpo", m.malha("casa", [paredes, telhado, porta, janela, chamine]))
    # Um nó pai com escala: confere que as transformações dos nós valem.
    m.no("casa", filhos=[corpo], raiz=True, scale=[1.5, 1.5, 1.5])
    return m


def pedra():
    m = Modelo()
    p = Parte("#A3A8B4")
    p.pedra(11)
    m.no("pedra", m.malha("pedra", [p]), raiz=True, translation=[0, 0.25, 0])
    return m


def heroi():
    """Virado para +Z (a frente no glTF)."""
    m = Modelo()
    corpo = Parte("#FF8C42")
    corpo.prisma(8, 0.32, 0.25, 1.1, raio_topo=0.28)
    pernas = Parte("#3D5A80")
    pernas.caixa(-0.13, 0.15, 0, 0.18, 0.3, 0.22)
    pernas.caixa(0.13, 0.15, 0, 0.18, 0.3, 0.22)
    cabeca = Parte("#FFD6A5")
    cabeca.caixa(0, 1.35, 0, 0.5, 0.45, 0.45)
    olhos = Parte("#2B2D42")
    olhos.caixa(-0.11, 1.38, 0.23, 0.08, 0.1, 0.02)
    olhos.caixa(0.11, 1.38, 0.23, 0.08, 0.1, 0.02)
    chapeu = Parte("#E63946")
    chapeu.prisma(8, 0.3, 1.57, 1.67)
    chapeu.prisma(8, 0.2, 1.67, 1.8, raio_topo=0.16)
    m.no("heroi", m.malha("heroi", [corpo, pernas, cabeca, olhos, chapeu]), raiz=True)
    return m


def caixote(uri=None):
    m = Modelo()
    m.com_textura(madeira(), uri)
    p = Parte("#FFFFFF", textura=True)
    p.caixa(0, 0.5, 0, 1, 1, 1)
    m.no("caixote", m.malha("caixote", [p]), raiz=True)
    return m


def main():
    destino = os.path.join(RAIZ, "jogos", "cubos3d", "modelos")
    testes = os.path.join(RAIZ, "crates", "nn-render", "tests", "modelos")
    os.makedirs(destino, exist_ok=True)
    os.makedirs(testes, exist_ok=True)
    for nome, fazer in [("arvore", arvore), ("casa", casa), ("pedra", pedra),
                        ("heroi", heroi), ("caixote", caixote)]:
        with open(os.path.join(destino, nome + ".glb"), "wb") as f:
            f.write(fazer().glb())
    # Um .gltf com o .bin e a imagem em arquivos separados (para os testes).
    j, b = caixote("madeira.png").gltf("caixote.bin")
    with open(os.path.join(testes, "caixote.gltf"), "wb") as f:
        f.write(j)
    with open(os.path.join(testes, "caixote.bin"), "wb") as f:
        f.write(b)
    with open(os.path.join(testes, "madeira.png"), "wb") as f:
        f.write(madeira())
    print("modelos gerados em", destino, "e", testes)


if __name__ == "__main__":
    main()
