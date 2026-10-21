#!/usr/bin/env python3
"""Gera os modelos 3D low-poly (.glb) do exemplo cubos3d e dos testes.

Só usa a biblioteca padrão do Python. Os modelos são nossos (sem licença de
terceiros) e servem também de exemplo de arquivo glTF como os do Blender:
várias partes com materiais, nós com posição/rotação/escala, uma textura e,
no herói e no bicho, um esqueleto com animações (parado, andar...).

    python3 ferramentas/gerar_modelos.py

Escreve em jogos/cubos3d/modelos/, jogos/estrelas3d/modelos/ e
crates/nn-render/tests/modelos/.
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

    def __init__(self, cor, textura=False, brilho=0.0):
        self.cor = cor
        self.textura = textura
        # Brilho próprio (emissão): aparece aceso mesmo no escuro. Acima de
        # 1, mais forte (a extensão KHR_materials_emissive_strength).
        self.brilho = brilho
        self.pos, self.nor, self.uv, self.idx = [], [], [], []
        # Modelos com esqueleto: o osso de cada vértice (o atual vale para o
        # que for acrescentado depois).
        self.osso = 0
        self.ossos = []

    def tri(self, a, b, c, uvs=((0, 0), (1, 0), (0, 1))):
        n = norm(cross(sub(b, a), sub(c, a)))
        base = len(self.pos)
        for p, uv in zip((a, b, c), uvs):
            self.pos.append(p)
            self.nor.append(n)
            self.uv.append(uv)
            self.ossos.append(self.osso)
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

    def prisma(self, lados, raio, y0, y1, raio_topo=None, giro=0.0, centro=(0, 0)):
        """Cilindro facetado (ou cone/tronco com raio_topo)."""
        rt = raio if raio_topo is None else raio_topo
        cx, cz = centro
        def ponto(i, r, y):
            a = giro + 2 * math.pi * i / lados
            px, pz = math.cos(a) * r, -math.sin(a) * r
            return (px + cx if cx else px, y, pz + cz if cz else pz)
        for i in range(lados):
            a0, a1 = ponto(i, raio, y0), ponto(i + 1, raio, y0)
            b0, b1 = ponto(i, rt, y1), ponto(i + 1, rt, y1)
            if rt > 1e-6:
                self.quad(a0, a1, b1, b0)
                self.tri((cx, y1, cz), b0, b1)
            else:
                self.tri(a0, a1, (cx, y1, cz))
            self.tri((cx, y0, cz), a1, a0)

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
        self.esqueleto = False

    def _view(self, data, target=None):
        while len(self.bin) % 4:
            self.bin.append(0)
        v = {"buffer": 0, "byteOffset": len(self.bin), "byteLength": len(data)}
        if target:
            v["target"] = target
        self.bin += data
        self.json["bufferViews"].append(v)
        return len(self.json["bufferViews"]) - 1

    def _acc(self, valores, tipo, n, alvo=34962):
        flat = [c for v in valores for c in v]
        view = self._view(struct.pack('<%df' % len(flat), *flat), alvo)
        a = {"bufferView": view, "componentType": 5126, "count": len(valores), "type": tipo}
        if tipo in ("VEC3", "SCALAR"):
            a["min"] = [min(v[i] for v in valores) for i in range(n)]
            a["max"] = [max(v[i] for v in valores) for i in range(n)]
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
            if p.brilho > 0:
                mat["emissiveFactor"] = cor(p.cor)[:3]
                if p.brilho != 1.0:
                    mat["extensions"] = {"KHR_materials_emissive_strength":
                                         {"emissiveStrength": p.brilho}}
                    usadas = self.json.setdefault("extensionsUsed", [])
                    if "KHR_materials_emissive_strength" not in usadas:
                        usadas.append("KHR_materials_emissive_strength")
            self.json["materials"].append(mat)
            idx = self._view(struct.pack('<%dI' % len(p.idx), *p.idx), 34963)
            self.json["accessors"].append({"bufferView": idx, "componentType": 5125,
                                           "count": len(p.idx), "type": "SCALAR"})
            indices = len(self.json["accessors"]) - 1
            attrs = {"POSITION": self._acc(p.pos, "VEC3", 3), "NORMAL": self._acc(p.nor, "VEC3", 3)}
            if p.textura:
                attrs["TEXCOORD_0"] = self._acc(p.uv, "VEC2", 2)
            if self.esqueleto:
                # Cada vértice preso inteiro a um osso (peças rígidas, o
                # visual low-poly).
                juntas = [j for o in p.ossos for j in (o, 0, 0, 0)]
                view = self._view(struct.pack('<%dH' % len(juntas), *juntas), 34962)
                self.json["accessors"].append({"bufferView": view, "componentType": 5123,
                                               "count": len(p.ossos), "type": "VEC4"})
                attrs["JOINTS_0"] = len(self.json["accessors"]) - 1
                attrs["WEIGHTS_0"] = self._acc([(1.0, 0.0, 0.0, 0.0)] * len(p.ossos), "VEC4", 4)
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

    def ossos(self, lista):
        """Esqueleto: lista de (nome, pai, posição no mundo) com o pai antes
        do filho. Devolve os índices dos nós; a malha da skin vem depois."""
        self.esqueleto = True
        nos, mundo = [], []
        for nome, pai, pos in lista:
            local = pos if pai is None else sub(pos, mundo[pai])
            no = {"name": nome, "translation": list(local)}
            self.json["nodes"].append(no)
            i = len(self.json["nodes"]) - 1
            if pai is None:
                self.json["scenes"][0]["nodes"].append(i)
            else:
                self.json["nodes"][nos[pai]].setdefault("children", []).append(i)
            nos.append(i)
            mundo.append(pos)
        # Matrizes inversas da pose de descanso (só translação), por coluna.
        ibm = [(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, -p[0], -p[1], -p[2], 1) for p in mundo]
        acc = self._acc(ibm, "MAT4", 16, alvo=None)
        self.json["skins"] = [{"joints": nos, "inverseBindMatrices": acc, "skeleton": nos[0]}]
        self._nos_ossos = nos
        return nos

    def animacao(self, nome, tempos, canais):
        """canais: { (osso, "rotation"|"translation"): [valor por tempo] }."""
        tempo = self._acc([(t,) for t in tempos], "SCALAR", 1, alvo=None)
        samplers, channels = [], []
        for (osso, prop), valores in canais.items():
            tipo = "VEC4" if prop == "rotation" else "VEC3"
            n = 4 if prop == "rotation" else 3
            assert len(valores) == len(tempos), (nome, osso, prop)
            saida = self._acc(valores, tipo, n, alvo=None)
            samplers.append({"input": tempo, "output": saida, "interpolation": "LINEAR"})
            channels.append({"sampler": len(samplers) - 1,
                             "target": {"node": self._nos_ossos[osso], "path": prop}})
        self.json.setdefault("animations", []).append(
            {"name": nome, "samplers": samplers, "channels": channels})

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


def quat(eixo, graus):
    """Rotação em volta de "x", "y" ou "z" (x, y, z, w)."""
    a = math.radians(graus) / 2
    q = [0.0, 0.0, 0.0, math.cos(a)]
    q["xyz".index(eixo)] = math.sin(a)
    return tuple(q)


def quat_xz(gx, gz):
    """Primeiro em volta de Z, depois de X (balançar e abrir um braço)."""
    x, z = quat("x", gx), quat("z", gz)
    # x * z
    return (x[3] * z[0] + x[0] * z[3] + x[1] * z[2] - x[2] * z[1],
            x[3] * z[1] - x[0] * z[2] + x[1] * z[3] + x[2] * z[0],
            x[3] * z[2] + x[0] * z[1] - x[1] * z[0] + x[2] * z[3],
            x[3] * z[3] - x[0] * z[0] - x[1] * z[1] - x[2] * z[2])


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
    """Virado para +Z (a frente no glTF), 1,8 m, com esqueleto e as animações
    parado, andar, correr, pular, cair, acenar, golpe e atirar. A esquerda dele
    é o +X. As mãos (mao_e e mao_d) são ossos sem nada preso: o lugar de
    segurar uma espada ou um cajado (componente "Preso a um osso")."""
    m = Modelo()
    # Ossos (posição no mundo, na pose de descanso).
    (RAIZ, QUADRIL, TRONCO, CABECA, BRACO_E, BRACO_D, PERNA_E, CANELA_E, PERNA_D, CANELA_D,
     MAO_E, MAO_D) = range(12)
    m.ossos([
        ("raiz", None, (0, 0, 0)),
        ("quadril", RAIZ, (0, 0.5, 0)),
        ("tronco", QUADRIL, (0, 0.55, 0)),
        ("cabeca", TRONCO, (0, 1.1, 0)),
        ("braco_e", TRONCO, (0.36, 1.0, 0)),
        ("braco_d", TRONCO, (-0.36, 1.0, 0)),
        ("perna_e", QUADRIL, (0.13, 0.5, 0)),
        ("canela_e", PERNA_E, (0.13, 0.26, 0)),
        ("perna_d", QUADRIL, (-0.13, 0.5, 0)),
        ("canela_d", PERNA_D, (-0.13, 0.26, 0)),
        ("mao_e", BRACO_E, (0.41, 0.53, 0)),
        ("mao_d", BRACO_D, (-0.41, 0.53, 0)),
    ])
    corpo = Parte("#FF8C42")
    corpo.osso = TRONCO
    corpo.prisma(8, 0.32, 0.45, 1.1, raio_topo=0.28)
    calca = Parte("#3D5A80")
    for lado, perna, canela in ((1, PERNA_E, CANELA_E), (-1, PERNA_D, CANELA_D)):
        calca.osso = perna
        calca.caixa(0.13 * lado, 0.38, 0, 0.18, 0.26, 0.22)
        calca.osso = canela
        calca.caixa(0.13 * lado, 0.13, 0.02, 0.17, 0.26, 0.24)
    pele = Parte("#FFD6A5")
    pele.osso = CABECA
    pele.caixa(0, 1.35, 0, 0.5, 0.45, 0.45)
    mangas = Parte("#F07A35")
    for lado, braco in ((1, BRACO_E), (-1, BRACO_D)):
        mangas.osso = braco
        mangas.caixa(0.41 * lado, 0.8, 0, 0.14, 0.42, 0.15)
        pele.osso = braco
        pele.caixa(0.41 * lado, 0.53, 0, 0.12, 0.12, 0.12)
    olhos = Parte("#2B2D42")
    olhos.osso = CABECA
    olhos.caixa(-0.11, 1.38, 0.23, 0.08, 0.1, 0.02)
    olhos.caixa(0.11, 1.38, 0.23, 0.08, 0.1, 0.02)
    chapeu = Parte("#E63946")
    chapeu.osso = CABECA
    chapeu.prisma(8, 0.3, 1.57, 1.67)
    chapeu.prisma(8, 0.2, 1.67, 1.8, raio_topo=0.16)
    malha = m.malha("heroi", [corpo, calca, pele, mangas, olhos, chapeu])
    m.json["nodes"].append({"name": "heroi", "mesh": malha, "skin": 0})
    m.json["scenes"][0]["nodes"].append(len(m.json["nodes"]) - 1)

    q0 = quat("x", 0)
    def x(*graus):
        return [quat("x", g) for g in graus]
    def quadril(*alturas):
        return [(0, a, 0) for a in alturas]

    # Parado: respira (o tronco e os braços mexem um pouco).
    t = [0, 1, 2]
    m.animacao("parado", t, {
        (QUADRIL, "translation"): quadril(0.5, 0.49, 0.5),
        (TRONCO, "rotation"): x(0, 2, 0),
        (CABECA, "rotation"): x(0, -2, 0),
        (BRACO_E, "rotation"): [quat("z", g) for g in (4, 7, 4)],
        (BRACO_D, "rotation"): [quat("z", -g) for g in (4, 7, 4)],
    })
    # Andar: pernas e braços balançam ao contrário (a frente é -X na rotação).
    t = [0, 0.2, 0.4, 0.6, 0.8]
    m.animacao("andar", t, {
        (QUADRIL, "translation"): quadril(0.5, 0.52, 0.5, 0.52, 0.5),
        (PERNA_E, "rotation"): x(-25, 0, 25, 0, -25),
        (PERNA_D, "rotation"): x(25, 0, -25, 0, 25),
        (CANELA_E, "rotation"): x(5, 5, 30, 15, 5),
        (CANELA_D, "rotation"): x(30, 15, 5, 5, 30),
        (BRACO_E, "rotation"): [quat_xz(g, 5) for g in (20, 0, -20, 0, 20)],
        (BRACO_D, "rotation"): [quat_xz(g, -5) for g in (-20, 0, 20, 0, -20)],
        (TRONCO, "rotation"): x(3, 3, 3, 3, 3),
    })
    # Correr: mais rápido, mais aberto e inclinado para a frente.
    t = [0, 0.125, 0.25, 0.375, 0.5]
    m.animacao("correr", t, {
        (QUADRIL, "translation"): quadril(0.47, 0.53, 0.47, 0.53, 0.47),
        (PERNA_E, "rotation"): x(-45, 0, 40, 0, -45),
        (PERNA_D, "rotation"): x(40, 0, -45, 0, 40),
        (CANELA_E, "rotation"): x(15, 20, 70, 40, 15),
        (CANELA_D, "rotation"): x(70, 40, 15, 20, 70),
        (BRACO_E, "rotation"): [quat_xz(g, 8) for g in (45, 0, -45, 0, 45)],
        (BRACO_D, "rotation"): [quat_xz(g, -8) for g in (-45, 0, 45, 0, -45)],
        (TRONCO, "rotation"): x(12, 10, 12, 10, 12),
    })
    # Pular: encolhe uma perna e abre os braços para cima (toca uma vez).
    t = [0, 0.15, 0.3]
    m.animacao("pular", t, {
        (PERNA_E, "rotation"): x(0, -40, -35),
        (CANELA_E, "rotation"): x(0, 60, 55),
        (PERNA_D, "rotation"): x(0, 15, 10),
        (CANELA_D, "rotation"): x(0, 25, 20),
        (BRACO_E, "rotation"): [quat("z", g) for g in (5, 120, 130)],
        (BRACO_D, "rotation"): [quat("z", -g) for g in (5, 120, 130)],
    })
    # Cair: braços para cima, balançando.
    t = [0, 0.3, 0.6]
    m.animacao("cair", t, {
        (BRACO_E, "rotation"): [quat("z", g) for g in (140, 155, 140)],
        (BRACO_D, "rotation"): [quat("z", -g) for g in (140, 125, 140)],
        (PERNA_E, "rotation"): [quat_xz(g, 8) for g in (-10, 5, -10)],
        (PERNA_D, "rotation"): [quat_xz(g, -8) for g in (5, -10, 5)],
        (CANELA_E, "rotation"): x(20, 10, 20),
        (CANELA_D, "rotation"): x(10, 20, 10),
    })
    # Acenar: o braço direito no alto, de um lado para o outro.
    t = [0, 0.3, 0.6, 0.9, 1.2]
    m.animacao("acenar", t, {
        (BRACO_D, "rotation"): [quat("z", -g) for g in (145, 165, 145, 165, 145)],
        (BRACO_E, "rotation"): [quat("z", 5)] * 5,
        (CABECA, "rotation"): [quat("z", g) for g in (0, 4, 0, 4, 0)],
        (QUADRIL, "translation"): quadril(0.5, 0.5, 0.5, 0.5, 0.5),
    })
    # Golpe: o braço direito sobe por cima da cabeça e desce na frente (o
    # acerto é perto de 0,22 s), com o tronco indo junto.
    t = [0, 0.12, 0.22, 0.34, 0.5]
    m.animacao("golpe", t, {
        (BRACO_D, "rotation"): [quat_xz(g, -z) for g, z in ((0, 4), (-165, 12), (-60, 6), (-45, 6), (0, 4))],
        (BRACO_E, "rotation"): [quat_xz(g, 6) for g in (0, 20, -15, -10, 0)],
        (TRONCO, "rotation"): x(0, -8, 16, 12, 0),
        (CABECA, "rotation"): x(0, 4, -8, -6, 0),
        (QUADRIL, "translation"): quadril(0.5, 0.5, 0.46, 0.47, 0.5),
        (PERNA_E, "rotation"): x(0, -10, -22, -20, 0),
        (PERNA_D, "rotation"): x(0, 6, 14, 12, 0),
    })
    # Atirar: o braço direito vai para a frente, fica um tempinho e volta (o
    # tiro sai perto de 0,1 s).
    t = [0, 0.08, 0.25, 0.4]
    m.animacao("atirar", t, {
        (BRACO_D, "rotation"): [quat_xz(g, -4) for g in (0, -88, -86, 0)],
        (TRONCO, "rotation"): x(0, -4, -3, 0),
    })
    del q0
    return m


def bicho():
    """Um besouro de 4 patas (inimigo), virado para +Z, com parado e andar."""
    m = Modelo()
    RAIZ, CORPO, CABECA, P1, P2, P3, P4 = range(7)
    m.ossos([
        ("raiz", None, (0, 0, 0)),
        ("corpo", RAIZ, (0, 0.3, 0)),
        ("cabeca", CORPO, (0, 0.32, 0.32)),
        ("pata_fe", CORPO, (0.25, 0.25, 0.18)),
        ("pata_fd", CORPO, (-0.25, 0.25, 0.18)),
        ("pata_te", CORPO, (0.25, 0.25, -0.2)),
        ("pata_td", CORPO, (-0.25, 0.25, -0.2)),
    ])
    casco = Parte("#8E44AD")
    casco.osso = CORPO
    casco.caixa(0, 0.35, -0.02, 0.5, 0.26, 0.6)
    casco.prisma(6, 0.25, 0.48, 0.58, raio_topo=0.12, centro=(0, -0.02))
    pintas = Parte("#F1C40F")
    pintas.osso = CORPO
    pintas.caixa(0.13, 0.49, 0.08, 0.1, 0.03, 0.1)
    pintas.caixa(-0.13, 0.49, -0.12, 0.1, 0.03, 0.1)
    escuro = Parte("#2B2D42")
    escuro.osso = CABECA
    escuro.caixa(0, 0.33, 0.38, 0.3, 0.2, 0.2)
    for i, (x, z) in enumerate(((0.25, 0.18), (-0.25, 0.18), (0.25, -0.2), (-0.25, -0.2))):
        escuro.osso = P1 + i
        escuro.caixa(x + (0.04 if x > 0 else -0.04), 0.12, z, 0.07, 0.25, 0.07)
    olhos = Parte("#FFFFFF")
    olhos.osso = CABECA
    olhos.caixa(0.07, 0.36, 0.48, 0.07, 0.07, 0.02)
    olhos.caixa(-0.07, 0.36, 0.48, 0.07, 0.07, 0.02)
    malha = m.malha("bicho", [casco, pintas, escuro, olhos])
    m.json["nodes"].append({"name": "bicho", "mesh": malha, "skin": 0})
    m.json["scenes"][0]["nodes"].append(len(m.json["nodes"]) - 1)
    def x(*graus):
        return [quat("x", g) for g in graus]
    t = [0, 0.75, 1.5]
    m.animacao("parado", t, {
        (CORPO, "translation"): [(0, 0.3, 0), (0, 0.28, 0), (0, 0.3, 0)],
        (CABECA, "rotation"): [quat("y", g) for g in (-8, 8, -8)],
    })
    t = [0, 0.15, 0.3, 0.45, 0.6]
    m.animacao("andar", t, {
        (CORPO, "translation"): [(0, 0.3, 0), (0, 0.32, 0), (0, 0.3, 0), (0, 0.32, 0), (0, 0.3, 0)],
        (P1, "rotation"): x(-30, 0, 30, 0, -30),
        (P4, "rotation"): x(-30, 0, 30, 0, -30),
        (P2, "rotation"): x(30, 0, -30, 0, 30),
        (P3, "rotation"): x(30, 0, -30, 0, 30),
    })
    # Golpe: recua e dá uma mordida para a frente (o acerto é perto de 0,3 s).
    t = [0, 0.18, 0.3, 0.45, 0.6]
    m.animacao("golpe", t, {
        (CORPO, "translation"): [(0, 0.3, 0), (0, 0.33, -0.08), (0, 0.27, 0.12), (0, 0.28, 0.08), (0, 0.3, 0)],
        (CABECA, "translation"): [(0, 0.02, 0.32), (0, 0.06, 0.26), (0, -0.02, 0.44), (0, 0, 0.4), (0, 0.02, 0.32)],
        (CABECA, "rotation"): x(0, -18, 14, 8, 0),
        (P1, "rotation"): x(0, 12, -20, -15, 0),
        (P2, "rotation"): x(0, 12, -20, -15, 0),
    })
    return m


def besourao():
    """O besouro-rei (chefe): um besouro grande, de chifres, virado para +Z,
    1,9 m de altura até a ponta do chifre, com 6 patas. Animações: parado,
    andar, golpe (o acerto é perto de 0,3 s), preparar (o aviso antes de
    atacar), pular, tonto e cair."""
    m = Modelo()
    RAIZ, CORPO, CABECA, P1, P2, P3, P4, P5, P6 = range(9)
    patas = [(0.78, 0.6), (-0.78, 0.6), (0.82, 0.0), (-0.82, 0.0), (0.78, -0.6), (-0.78, -0.6)]
    m.ossos([("raiz", None, (0, 0, 0)), ("corpo", RAIZ, (0, 0.9, 0)), ("cabeca", CORPO, (0, 1.0, 0.9))]
            + [("pata%d" % (i + 1), CORPO, (x, 0.85, z)) for i, (x, z) in enumerate(patas)])
    casco = Parte("#A3242F")
    casco.osso = CORPO
    casco.caixa(0, 1.0, -0.12, 1.5, 0.7, 1.8)
    casco.prisma(8, 0.74, 1.35, 1.65, raio_topo=0.42, centro=(0, -0.12))
    faixa = Parte("#5C1018")
    faixa.osso = CORPO
    faixa.caixa(0, 1.66, -0.12, 0.08, 0.04, 1.2)
    pintas = Parte("#F4B400")
    pintas.osso = CORPO
    for x, z in ((0.38, 0.25), (-0.38, 0.25), (0.42, -0.45), (-0.42, -0.45)):
        pintas.caixa(x, 1.36, z, 0.22, 0.04, 0.22)
    escuro = Parte("#2B2D42")
    escuro.osso = CABECA
    escuro.caixa(0, 1.0, 1.12, 0.9, 0.56, 0.5)
    for i, (x, z) in enumerate(patas):
        escuro.osso = P1 + i
        escuro.caixa(x + (0.1 if x > 0 else -0.1), 0.42, z, 0.26, 0.86, 0.26)
    chifres = Parte("#F2E3C6")
    chifres.osso = CABECA
    chifres.prisma(6, 0.15, 1.2, 1.95, raio_topo=0.0, centro=(0, 1.22))
    chifres.prisma(5, 0.09, 1.2, 1.6, raio_topo=0.0, centro=(0.3, 1.18))
    chifres.prisma(5, 0.09, 1.2, 1.6, raio_topo=0.0, centro=(-0.3, 1.18))
    chifres.caixa(0.22, 0.74, 1.42, 0.08, 0.14, 0.2)
    chifres.caixa(-0.22, 0.74, 1.42, 0.08, 0.14, 0.2)
    olhos = Parte("#FFD93D")
    olhos.osso = CABECA
    olhos.caixa(0.27, 1.08, 1.38, 0.15, 0.12, 0.02)
    olhos.caixa(-0.27, 1.08, 1.38, 0.15, 0.12, 0.02)
    malha = m.malha("besourao", [casco, faixa, pintas, escuro, chifres, olhos])
    m.json["nodes"].append({"name": "besourao", "mesh": malha, "skin": 0})
    m.json["scenes"][0]["nodes"].append(len(m.json["nodes"]) - 1)

    def x(*graus):
        return [quat("x", g) for g in graus]
    def z(*graus):
        return [quat("z", g) for g in graus]
    def corpo(*pos):
        return [(0, y, dz) for y, dz in pos]
    cab = (0, 0.1, 0.9)

    t = [0, 1, 2]
    m.animacao("parado", t, {
        (CORPO, "translation"): corpo((0.9, 0), (0.87, 0), (0.9, 0)),
        (CABECA, "rotation"): [quat("y", g) for g in (-6, 6, -6)],
    })
    t = [0, 0.2, 0.4, 0.6, 0.8]
    um, outro = x(-25, 0, 25, 0, -25), x(25, 0, -25, 0, 25)
    m.animacao("andar", t, {
        (CORPO, "translation"): corpo((0.9, 0), (0.94, 0), (0.9, 0), (0.94, 0), (0.9, 0)),
        (P1, "rotation"): um, (P4, "rotation"): um, (P5, "rotation"): um,
        (P2, "rotation"): outro, (P3, "rotation"): outro, (P6, "rotation"): outro,
    })
    # Golpe: levanta a cabeça e bate os chifres para baixo e para a frente.
    t = [0, 0.15, 0.3, 0.5, 0.8]
    m.animacao("golpe", t, {
        (CORPO, "translation"): corpo((0.9, 0), (0.95, -0.15), (0.82, 0.35), (0.85, 0.2), (0.9, 0)),
        (CORPO, "rotation"): x(0, -10, 8, 5, 0),
        (CABECA, "rotation"): x(0, -30, 32, 20, 0),
        (P1, "rotation"): x(0, -20, 15, 10, 0),
        (P2, "rotation"): x(0, -20, 15, 10, 0),
    })
    # Preparar (o aviso): empina a frente e treme.
    t = [0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6]
    m.animacao("preparar", t, {
        (CORPO, "translation"): corpo(*[(0.98, -0.1)] * 7),
        (CORPO, "rotation"): [quat_xz(-14, g) for g in (0, 4, -4, 4, -4, 4, 0)],
        (CABECA, "rotation"): x(-22, -18, -22, -18, -22, -18, -22),
        (P1, "rotation"): x(-30, -30, -30, -30, -30, -30, -30),
        (P2, "rotation"): x(-30, -30, -30, -30, -30, -30, -30),
    })
    # Pular: agacha, estica e abre as patas no ar.
    t = [0, 0.12, 0.25, 0.6, 0.9]
    abre_e, abre_d = z(0, -10, 30, 30, 0), z(0, 10, -30, -30, 0)
    m.animacao("pular", t, {
        (CORPO, "translation"): corpo((0.9, 0), (0.68, 0), (0.98, 0), (0.95, 0), (0.9, 0)),
        (CABECA, "rotation"): x(0, 10, -20, -15, 0),
        (P1, "rotation"): abre_e, (P3, "rotation"): abre_e, (P5, "rotation"): abre_e,
        (P2, "rotation"): abre_d, (P4, "rotation"): abre_d, (P6, "rotation"): abre_d,
    })
    # Tonto: a cabeça balança e o corpo cambaleia.
    t = [0, 0.25, 0.5, 0.75, 1]
    m.animacao("tonto", t, {
        (CORPO, "translation"): corpo((0.84, 0), (0.82, 0), (0.84, 0), (0.82, 0), (0.84, 0)),
        (CORPO, "rotation"): z(0, 6, 0, -6, 0),
        (CABECA, "rotation"): [quat_xz(15, g) for g in (0, 18, 0, -18, 0)],
    })
    # Cair: tomba de lado, com as patas encolhidas (toca uma vez).
    t = [0, 0.3, 0.7, 1.2]
    encolhe = x(0, 30, 50, 50)
    m.animacao("cair", t, {
        (CORPO, "translation"): corpo((0.9, 0), (0.85, 0), (0.6, 0), (0.62, 0)),
        (CORPO, "rotation"): z(0, 20, 75, 80),
        (CABECA, "rotation"): x(0, 15, 25, 25),
        (P1, "rotation"): encolhe, (P2, "rotation"): encolhe, (P3, "rotation"): encolhe,
        (P4, "rotation"): encolhe, (P5, "rotation"): encolhe, (P6, "rotation"): encolhe,
    })
    del cab
    return m


def espada():
    """Espada para segurar: o cabo na origem (onde a mão pega) e a lâmina
    para a frente (+Z), 0,9 m. Presa no osso mao_d do herói, com o braço
    parado, ela aponta para a frente."""
    m = Modelo()
    lamina = Parte("#D9E2EC")
    lamina.caixa(0, 0, 0.47, 0.03, 0.09, 0.62)
    # A ponta: um prisma de 4 lados deitado (fino como a lâmina).
    ponta = Parte("#D9E2EC")
    ponta.tri((0.015, -0.045, 0.78), (0.015, 0.045, 0.78), (0, 0, 0.9))
    ponta.tri((-0.015, 0.045, 0.78), (-0.015, -0.045, 0.78), (0, 0, 0.9))
    ponta.tri((-0.015, -0.045, 0.78), (0.015, -0.045, 0.78), (0, 0, 0.9))
    ponta.tri((0.015, 0.045, 0.78), (-0.015, 0.045, 0.78), (0, 0, 0.9))
    ponta.quad((0.015, -0.045, 0.78), (-0.015, -0.045, 0.78), (-0.015, 0.045, 0.78), (0.015, 0.045, 0.78))
    guarda = Parte("#F4B400")
    guarda.caixa(0, 0, 0.13, 0.06, 0.28, 0.05)
    cabo = Parte("#6B4226")
    cabo.caixa(0, 0, 0.02, 0.05, 0.05, 0.2)
    pomo = Parte("#F4B400")
    pomo.caixa(0, 0, -0.1, 0.07, 0.07, 0.06)
    m.no("espada", m.malha("espada", [lamina, ponta, guarda, cabo, pomo]), raiz=True)
    return m


def cajado():
    """Cajado de mago: a mão pega na origem; a haste sobe (+Y) até 1,1 m,
    com uma pedra brilhante na ponta, e desce 0,35 m."""
    m = Modelo()
    haste = Parte("#8B5A3C")
    haste.prisma(6, 0.03, -0.35, 1.0)
    anel = Parte("#F4B400")
    anel.prisma(6, 0.05, 0.92, 1.0)
    pedra = Parte("#4CC9F0", brilho=0.8)
    pedra.prisma(5, 0.09, 1.0, 1.1)
    pedra.prisma(5, 0.09, 1.1, 1.22, raio_topo=0.0)
    m.no("cajado", m.malha("cajado", [haste, anel, pedra]), raiz=True)
    return m


def tocha():
    """Tocha de pé (ou presa na parede, inclinada): o cabo de madeira, o
    copo de ferro e a chama que brilha. A base na origem; 1,25 m."""
    m = Modelo()
    cabo = Parte("#6B4226")
    cabo.prisma(6, 0.045, 0.0, 0.9, raio_topo=0.055)
    copo = Parte("#3A3A42")
    copo.prisma(6, 0.07, 0.86, 0.98, raio_topo=0.1)
    fora = Parte("#FF8A1F", brilho=1.0)
    fora.prisma(5, 0.085, 0.98, 1.08, raio_topo=0.07)
    fora.prisma(5, 0.07, 1.08, 1.25, raio_topo=0.0, giro=0.3)
    dentro = Parte("#FFE066", brilho=1.4)
    dentro.prisma(4, 0.045, 1.04, 1.16, raio_topo=0.0, giro=0.6, centro=(0.0, 0.07))
    m.no("tocha", m.malha("tocha", [cabo, copo, fora, dentro]), raiz=True)
    return m


def lampiao():
    """Lampião de ferro com o vidro aceso: a base na origem; 0,45 m, com a
    alça em cima (para pendurar ou segurar)."""
    m = Modelo()
    ferro = Parte("#2E2E36")
    ferro.prisma(6, 0.12, 0.0, 0.05)
    ferro.prisma(6, 0.13, 0.27, 0.31, raio_topo=0.06)
    ferro.prisma(6, 0.06, 0.31, 0.35, raio_topo=0.03)
    # As quatro colunas do vidro.
    for x, z in ((0.09, 0.09), (-0.09, 0.09), (0.09, -0.09), (-0.09, -0.09)):
        ferro.caixa(x, 0.16, z, 0.025, 0.22, 0.025)
    # A alça: três barrinhas em arco.
    ferro.caixa(0.0, 0.43, 0.0, 0.12, 0.025, 0.025)
    ferro.caixa(0.065, 0.39, 0.0, 0.025, 0.08, 0.025)
    ferro.caixa(-0.065, 0.39, 0.0, 0.025, 0.08, 0.025)
    vidro = Parte("#FFD27A", brilho=1.2)
    vidro.prisma(6, 0.085, 0.05, 0.27)
    m.no("lampiao", m.malha("lampiao", [ferro, vidro]), raiz=True)
    return m


def cristal():
    """Cristais que brilham saindo de uma pedra (cavernas): pontas de seis
    lados, de alturas diferentes. A base na origem; ~1 m."""
    m = Modelo()
    pedra = Parte("#4A4652")
    pedra.prisma(7, 0.42, 0.0, 0.16, raio_topo=0.3)
    claro = Parte("#7FF5FF", brilho=0.9)
    escuro = Parte("#4CC9F0", brilho=0.6)
    for (x, z, r, h, parte) in ((0.0, 0.0, 0.13, 0.75, claro), (0.2, 0.08, 0.09, 0.45, escuro),
                                (-0.17, 0.12, 0.08, 0.38, claro), (0.05, -0.2, 0.08, 0.5, escuro),
                                (-0.12, -0.12, 0.06, 0.3, escuro)):
        parte.prisma(6, r, 0.1, 0.1 + h, centro=(x, z))
        parte.prisma(6, r, 0.1 + h, 0.1 + h + r * 2.2, raio_topo=0.0, centro=(x, z))
    m.no("cristal", m.malha("cristal", [pedra, claro, escuro]), raiz=True)
    return m


def estrela():
    """Estrela de 5 pontas, gorda (como as de jogo de plataforma), de pé."""
    m = Modelo()
    p = Parte("#FFD93D")
    pontas, r_fora, r_dentro, esp = 5, 0.5, 0.22, 0.14
    contorno = []
    for i in range(pontas * 2):
        a = math.pi / 2 + i * math.pi / pontas
        r = r_fora if i % 2 == 0 else r_dentro
        contorno.append((math.cos(a) * r, math.sin(a) * r))
    frente, tras = (0.0, 0.0, esp), (0.0, 0.0, -esp)
    n = len(contorno)
    for i in range(n):
        (x0, y0), (x1, y1) = contorno[i], contorno[(i + 1) % n]
        a, b = (x0, y0, 0.0), (x1, y1, 0.0)
        p.tri(frente, a, b)   # frente (sentido anti-horário visto de +Z)
        p.tri(tras, b, a)     # trás
    olhos = Parte("#3A2E1F")
    olhos.caixa(-0.07, 0.05, 0.11, 0.05, 0.1, 0.04)
    olhos.caixa(0.07, 0.05, 0.11, 0.05, 0.1, 0.04)
    m.no("estrela", m.malha("estrela", [p, olhos]), raiz=True, translation=[0, 0.5, 0])
    return m


def moeda():
    """Moeda em pé (gira em volta de Y no jogo)."""
    m = Modelo()
    borda = Parte("#F4B400")
    borda.prisma(10, 0.3, -0.05, 0.05)
    meio = Parte("#FFE066")
    meio.caixa(0, 0, 0.0, 0.12, 0.3, 0.12)
    # O prisma é deitado de lado: gira 90° em X para ficar em pé.
    a = math.radians(90) / 2
    m.no("moeda", m.malha("moeda", [borda, meio]), raiz=True,
         rotation=[math.sin(a), 0.0, 0.0, math.cos(a)], translation=[0, 0.4, 0])
    return m


def chave():
    """Chave dourada em pé (gira em volta de Y no jogo): argola, haste e dentes."""
    m = Modelo()
    ouro = Parte("#F4B400")
    claro = Parte("#FFE066")
    esp = 0.05
    # A argola: um anel de 8 lados, deitado de lado (no plano XY).
    lados, r_fora, r_dentro, cy = 8, 0.17, 0.09, 0.5
    for i in range(lados):
        a0 = 2 * math.pi * i / lados
        a1 = 2 * math.pi * (i + 1) / lados
        def p(a, r, z):
            return (math.cos(a) * r, cy + math.sin(a) * r, z)
        f0, f1 = p(a0, r_fora, esp), p(a1, r_fora, esp)
        d0, d1 = p(a0, r_dentro, esp), p(a1, r_dentro, esp)
        t0, t1 = p(a0, r_fora, -esp), p(a1, r_fora, -esp)
        e0, e1 = p(a0, r_dentro, -esp), p(a1, r_dentro, -esp)
        ouro.quad(d0, f0, f1, d1)    # frente
        ouro.quad(f0, d0, e0, t0)    # (lado de fora e de dentro abaixo)
        ouro.quad(t1, e1, d1, f1)
        ouro.quad(e0, e1, t1, t0)    # trás
        ouro.quad(f0, t0, t1, f1)    # borda de fora
        ouro.quad(d1, e1, e0, d0)    # borda de dentro
    # A haste e os dentes.
    claro.caixa(0, 0.17, 0, 0.07, 0.48, esp * 2)
    claro.caixa(0.07, 0.0, 0, 0.1, 0.06, esp * 2)
    claro.caixa(0.06, 0.1, 0, 0.08, 0.05, esp * 2)
    m.no("chave", m.malha("chave", [ouro, claro]), raiz=True, translation=[0, 0.05, 0])
    return m


def porta():
    """Porta de madeira (1,2 × 2,2 m, 15 cm de espessura), com a base no chão:
    tábuas, duas faixas de ferro e a maçaneta (dos dois lados)."""
    m = Modelo()
    madeira_ = Parte("#9C6B43")
    escura = Parte("#7A5232")
    ferro = Parte("#4A4F57")
    w, h, e = 1.2, 2.2, 0.15
    tabuas = 4
    for i in range(tabuas):
        x = -w / 2 + (i + 0.5) * w / tabuas
        (madeira_ if i % 2 == 0 else escura).caixa(x, h / 2, 0, w / tabuas - 0.01, h, e)
    for y in (0.45, 1.75):
        ferro.caixa(0, y, 0, w + 0.01, 0.1, e + 0.02)
    for z in (e / 2 + 0.03, -e / 2 - 0.03):
        ferro.caixa(w / 2 - 0.18, 1.05, z, 0.08, 0.08, 0.06)
    m.no("porta", m.malha("porta", [madeira_, escura, ferro]), raiz=True)
    return m


def plataforma():
    """Bloco de chão 1 × 1 × 1 com grama em cima (estique com escala)."""
    m = Modelo()
    terra = Parte("#9C6B43")
    terra.caixa(0, -0.55, 0, 1.0, 0.9, 1.0)
    grama = Parte("#6CC24A")
    grama.caixa(0, -0.05, 0, 1.04, 0.1, 1.04)
    m.no("plataforma", m.malha("plataforma", [terra, grama]), raiz=True)
    return m


def caixote(uri=None):
    m = Modelo()
    m.com_textura(madeira(), uri)
    p = Parte("#FFFFFF", textura=True)
    p.caixa(0, 0.5, 0, 1, 1, 1)
    m.no("caixote", m.malha("caixote", [p]), raiz=True)
    return m


def roda():
    """Teste: uma parte sem ossos com animação no nó dela (gira em Y, 1 s)."""
    m = Modelo()
    p = Parte("#FF0000")
    p.caixa(0.5, 0, 0, 1, 0.2, 0.2)
    malha = m.malha("barra", [p])
    eixo = m.no("eixo", malha, raiz=True, translation=[0, 1, 0])
    m._nos_ossos = [eixo]
    m.animacao("girar", [0, 0.5, 1], {(0, "rotation"): [quat("y", 0), quat("y", 90), quat("y", 180)]})
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
    # O jogo Ilha das Estrelas usa os mesmos e mais alguns.
    estrelas = os.path.join(RAIZ, "jogos", "estrelas3d", "modelos")
    os.makedirs(estrelas, exist_ok=True)
    for nome, fazer in [("arvore", arvore), ("pedra", pedra), ("heroi", heroi),
                        ("caixote", caixote), ("estrela", estrela), ("moeda", moeda),
                        ("plataforma", plataforma), ("bicho", bicho), ("chave", chave),
                        ("porta", porta), ("espada", espada), ("cajado", cajado),
                        ("besourao", besourao), ("tocha", tocha), ("lampiao", lampiao),
                        ("cristal", cristal)]:
        with open(os.path.join(estrelas, nome + ".glb"), "wb") as f:
            f.write(fazer().glb())
    with open(os.path.join(testes, "roda.glb"), "wb") as f:
        f.write(roda().glb())
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
