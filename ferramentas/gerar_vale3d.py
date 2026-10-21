#!/usr/bin/env python3
"""Gera o modelo de projeto "vale3d": um vale pintado num Terreno (morros,
um lago, uma trilha, cores), com árvores espalhadas, estrelas nos morros,
moedas na trilha, uma fogueira, besouros que mordem e o herói com uma espada
na mão (Golpe).

    python3 ferramentas/gerar_vale3d.py
    NN_REESCREVER=1 cargo test -p nn-studio --lib modelos_ja_vem

O terreno é gravado no mesmo formato do NN Studio (TerrainData::encode).
"""
import base64, math, os, random, struct, sys

sys.dont_write_bytecode = True
from cenas3d import Cena, ENV, HEADER, besouro, comp, coracoes, env, particulas, som, vida_heroi, write

RAIZ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEST = os.path.join(RAIZ, 'crates', 'nn-run', 'modelos', 'vale3d')

CELLS, W, D, MAXH = 64, 96.0, 96.0, 20.0
SIDE = CELLS + 1
SP = W / CELLS

def ponto(i, j):
    return (-W / 2 + i * SP, -D / 2 + j * SP)

def gauss(x, z, cx, cz, r, h):
    return h * math.exp(-((x - cx) ** 2 + (z - cz) ** 2) / (2 * r * r))

LAGO = (-14.0, 12.0, 9.0)        # centro x, z e raio
TRILHA = [(0, 40), (0, 22), (6, 8), (4, -6), (-6, -18), (-4, -34)]
MORROS = [(24, -18, 9, 9), (-28, -22, 10, 11), (28, 22, 8, 7), (-30, 30, 7, 6), (2, -40, 8, 6)]

def dist_trilha(x, z):
    best = 1e9
    for (ax, az), (bx, bz) in zip(TRILHA, TRILHA[1:]):
        dx, dz = bx - ax, bz - az
        t = max(0.0, min(1.0, ((x - ax) * dx + (z - az) * dz) / (dx * dx + dz * dz)))
        px, pz = ax + dx * t, az + dz * t
        best = min(best, math.hypot(x - px, z - pz))
    return best

def altura(x, z):
    h = 0.0
    for cx, cz, r, a in MORROS:
        h += gauss(x, z, cx, cz, r, a)
    # Uma borda alta em volta (o vale), mais baixa onde a trilha sai.
    borda = max(abs(x), abs(z)) / (W / 2)
    h += max(0.0, borda - 0.72) ** 2 * 140
    # Ondulação suave.
    h += 0.6 * math.sin(x * 0.21) * math.cos(z * 0.17)
    # O lago: um buraco.
    lx, lz, lr = LAGO
    dl = math.hypot(x - lx, z - lz)
    if dl < lr * 1.6:
        k = max(0.0, 1 - dl / (lr * 1.6))
        h -= 3.2 * (k ** 0.8)
    # A trilha: aplaina perto dela.
    dt = dist_trilha(x, z)
    if dt < 3:
        h *= 0.35 + 0.65 * (dt / 3)
    return max(-MAXH, min(MAXH, h))

alturas, camadas = [], []
for j in range(SIDE):
    for i in range(SIDE):
        x, z = ponto(i, j)
        h = altura(x, z)
        alturas.append(h)
        # Cores: grama, terra (trilha), pedra (no alto), areia (no lago).
        g, t, p, a = 255, 0, 0, 0
        if h > 5.5:
            k = min(1.0, (h - 5.5) / 3)
            p, g = int(255 * k), int(255 * (1 - k))
        dt = dist_trilha(x, z)
        if dt < 2.2:
            k = 1 - dt / 2.2
            t = int(255 * min(1, k * 1.5))
            g = max(0, g - t)
        lx, lz, lr = LAGO
        dl = math.hypot(x - lx, z - lz)
        if dl < lr * 1.45:
            k = min(1.0, (lr * 1.45 - dl) / 3)
            a = int(255 * k)
            g, t, p = int(g * (1 - k)), int(t * (1 - k)), int(p * (1 - k))
        camadas.append((g, t, p, a))

# Como o TerrainData::encode: alturas em 16 bits com sinal e as camadas.
q = [max(-32767, min(32767, round(h / MAXH * 32767))) for h in alturas]
dados = struct.pack('<%dh' % len(q), *q) + bytes(v for c in camadas for v in c)
DATA = 't1:' + base64.b64encode(dados).decode()
# As alturas como o jogo vai ler (já arredondadas).
H = [v / 32767 * MAXH for v in q]

def chao(x, z):
    """Altura do terreno em (x, z), com os triângulos (TerrainData::height_at)."""
    fx, fz = (x + W / 2) / SP, (z + D / 2) / SP
    i, j = min(int(fx), CELLS - 1), min(int(fz), CELLS - 1)
    u, v = fx - i, fz - j
    h = lambda a, b: H[b * SIDE + a]
    h00, h10, h01, h11 = h(i, j), h(i + 1, j), h(i, j + 1), h(i + 1, j + 1)
    if u >= v:
        return h00 + (h10 - h00) * u + (h11 - h10) * v
    return h00 + (h11 - h01) * u + (h01 - h00) * v

NIVEL = -1.2  # A água do lago.

c = Cena()
c.obj('Terreno', [comp('terrain', data=DATA, cells=CELLS, width=W, depth=D, height=MAXH,
                       color1='#7BC86C', color2='#B08356', color3='#9EA3A8', color4='#F2D49B',
                       water=True, water_level=NIVEL, water_color='#3FA7D6B4')])

heroi_events = [
    '        { when = {',
    '            { cond = "mensagem", quem = "@este", msg = "pulou" },',
    '          }, actions = {',
    '            { act = "som", som = "assets/sons/pulo.wav", volume = 0.5, onde = "@este", distancia = 30 },',
    '            { act = "soltar_particulas", alvo = "Poeira", quantidade = 14 },',
    '          } },',
    '        { when = {',
    '            { cond = "mensagem", quem = "@este", msg = "agua" },',
    '          }, actions = {',
    '            { act = "som", som = "assets/sons/dano.wav", volume = 1, onde = "@este", distancia = 30 },',
    '          } },',
]
hx, hz = 0.0, 36.0
ESPADA_RX = 20
c.obj('Herói', [comp('model', file='modelos/heroi.glb', color='#FFFFFF', flat=False, shadow=True),
                comp('behavior', script='nn:jogador3d', velocidade=6, pulo=8, pulos_no_ar=0, camera='atras',
                     distancia=7, prender_mouse=True, sensibilidade=0.15, inverter_y=False, virar=True,
                     agua='voltar', queda=-25, raio=0, altura=0),
                vida_heroi(tremer=6),
                comp('behavior', script='nn:golpe', quando='tecla', tecla='clique', alvo='', dano=1,
                     alcance=1.7, angulo=120, todos=True, empurrao=6, intervalo=0.45, animacao='golpe',
                     acerto=0.22, som='assets/sons/golpe.wav', som_acerto='assets/sons/acerto.wav')],
      events=heroi_events, x=hx, y=round(chao(hx, hz) + 0.05, 2), z=hz, ry=180, scale_x=1.1, scale_y=1.1, scale_z=1.1,
      children=[c.child('Espada', [comp('model', file='modelos/espada.glb', color='#FFFFFF', flat=False, shadow=True),
                                   comp('bone', bone='mao_d')], rx=ESPADA_RX),
                c.child('Poeira', [particulas(
          mode="explosao", amount=14, on_start=False, lifetime=0.6, lifetime_var=0.4,
          speed_min=0.8, speed_max=2, spread=200, gravity=0.5, drag=4, area="circulo",
          area_radius=0.3, size_start=0.35, size_end=0.6, color_start="#E8DCC4CC",
          color_end="#C8B89800")], y=0.05)])

# Árvores e pedras espalhadas (longe da trilha e do lago).
rnd = random.Random(21)
postas = []
tentativas = 0
while len(postas) < 46 and tentativas < 4000:
    tentativas += 1
    x, z = rnd.uniform(-40, 40), rnd.uniform(-40, 40)
    if dist_trilha(x, z) < 4.5 or math.hypot(x - LAGO[0], z - LAGO[1]) < LAGO[2] * 1.6:
        continue
    if math.hypot(x - hx, z - hz) < 5:
        continue
    if any(math.hypot(x - a, z - b) < 3.2 for a, b, _ in postas):
        continue
    h = chao(x, z)
    if h > 9:
        continue
    pedra = rnd.random() < 0.22
    postas.append((x, z, pedra))
    s = round(rnd.uniform(0.9, 1.45), 2) if not pedra else round(rnd.uniform(1.2, 2.2), 2)
    ry = rnd.randrange(0, 360)
    if pedra:
        c.obj('pedra', [comp('model', file='modelos/pedra.glb', color='#FFFFFF', flat=False, shadow=True),
                        comp('body3d', body='static')],
              x=round(x, 2), y=round(h + 0.2 * s, 2), z=round(z, 2), ry=ry, scale_x=s, scale_y=s, scale_z=s)
    else:
        c.obj('arvore', [comp('model', file='modelos/arvore.glb', color='#FFFFFF', flat=False, shadow=True),
                         comp('body3d', body='static')],
              x=round(x, 2), y=round(h - 0.05, 2), z=round(z, 2), ry=ry, scale_x=s, scale_y=s, scale_z=s)

# Estrelas: nos topos dos morros.
for (cx, cz, r, a) in MORROS:
    c.obj('estrela', prefab='prefabs/estrela.prefab', x=cx, y=round(chao(cx, cz) + 0.6, 2), z=cz)
# Moedas na trilha.
for (ax, az), (bx, bz) in zip(TRILHA, TRILHA[1:]):
    for k in range(1, 4):
        t = k / 4
        x, z = ax + (bx - ax) * t, az + (bz - az) * t
        c.obj('moeda', prefab='prefabs/moeda.prefab', x=round(x, 2), y=round(chao(x, z) + 0.6, 2), z=round(z, 2))

# A fogueira na beira do lago.
fx, fz = -4.0, 4.0
c.obj('Fogueira', [comp('shape3d', shape='cylinder', color='#5B3A29', shadow=True),
                   comp('body3d', body='static'), som('assets/sons/fogueira.wav', 14)],
      x=fx, y=round(chao(fx, fz) + 0.05, 2), z=fz, scale_x=0.9, scale_y=0.15, scale_z=0.9,
      children=[
          c.child('Fogo', [particulas(
              rate=60, lifetime=0.8, lifetime_var=0.4, speed_min=0.6, speed_max=1.6, spread=25,
              gravity=1.5, area="circulo", area_radius=0.25, size_start=0.5, size_end=0.1,
              size_var=0.3, color_start="#FFE066", color_mid="#FF7A1A", color_end="#B0201000",
              blend="add")], y=1),
          c.child('Fumaça', [particulas(
              rate=8, lifetime=3, lifetime_var=0.3, speed_min=0.5, speed_max=1, spread=30,
              gravity=0.4, drag=0.3, area="circulo", area_radius=0.2, size_start=0.4,
              size_end=1.8, color_start="#9A9A9AAA", color_end="#5A5A5A00", spin=30)], y=8),
      ])
c.obj('Lago', [som('assets/sons/mar.wav', 16, 0.5)], x=LAGO[0], y=NIVEL, z=LAGO[1])

# Uma placa na entrada da trilha (Texto 3D preso como placa).
px, pz = 2.6, 31.0
ph = round(chao(px, pz), 2)
c.obj('Placa', [comp('shape3d', shape='cube', color='#9C6B43', shadow=True), comp('body3d', body='static')],
      x=px, y=round(ph + 1.55, 2), z=pz, scale_x=2.6, scale_y=0.75, scale_z=0.1,
      children=[c.child('Poste', [comp('shape3d', shape='cylinder', color='#7A5232', shadow=True)],
                        y=-1.25, scale_x=0.06, scale_y=2.1, scale_z=1.2),
                c.child('Letreiro', [comp('text3d', text='Vale das Estrelas', size=0.26, color='#FFF1D6',
                                          facing='placa', outline='#3A2E1FC0', background='', font='')],
                        z=0.6, scale_x=0.385, scale_y=1.333, scale_z=10)])

c.obj('Placar', [comp('text', text='', size=30, color='#FFD93D', anchor='left', font='', hud=True),
                 comp('behavior', script='nn:mostrar_valor', texto='Estrelas {estrelas} / 5     Moedas {moedas}')],
      x=40, y=-44, anchor='top_left')
c.obj('Dica', [comp('text', text='Ache as 5 estrelas nos morros! Clique para golpear (Esc solta o mouse)', size=32, color='#FFFFFF', anchor='center', font='', hud=True)],
      y=-120, anchor='top')
coracoes(c)

# Besouros perto da trilha.
for (bx, bz, modo, ax) in [(8, 0, 'vagar', 0), (-8, -26, 'patrulha', 8), (14, -30, 'vagar', 0)]:
    besouro(c, bx, round(chao(bx, bz), 2), bz, andar=modo, ax=ax, az=0, raio=4, combate=True)

eventos = '''  events = {
    { comment = "Começo: zera as estrelas e as moedas; a dica some depois de 4 segundos." },
    { when = {
        { cond = "comecou" },
      }, actions = {
        { act = "variavel", var = "estrelas", op = "definir", valor = 0 },
        { act = "variavel", var = "moedas", op = "definir", valor = 0 },
      } },
    { when = {
        { cond = "depois_de", segundos = 4 },
      }, actions = {
        { act = "mostrar", acao = "esconder", alvo = "Dica" },
      } },
    { comment = "Derrotou um besouro: ganha 3 moedas." },
    { when = {
        { cond = "derrotar", quem = "@jogador", alvo = "Besouro" },
      }, actions = {
        { act = "variavel", var = "moedas", op = "somar", valor = 3 },
      } },
    { comment = "Achou as 5 estrelas: vitória!" },
    { when = {
        { cond = "variavel", var = "estrelas", op = ">=", valor = 5 },
      }, actions = {
        { act = "som", som = "assets/sons/vitoria.wav", volume = 1 },
        { act = "esperar", segundos = 1.2 },
        { act = "ir_para_cena", cena = "cenas/fim.cena", escurecer = 0.8 },
      } },
  },
'''
VALE_ENV = env(sky=True, sky_top="#4A90D9", sky_horizon="#D8EEF5", sky_bottom="#3A6E8E",
               sun_height=40, sun_turn=135, sun_color="#FFEBCC", sun_size=4,
               ambient_sky="#93AFCF", ambient_ground="#5E6B52",
               fog=True, fog_near=45, fog_far=120, shadows=True, shadow_distance=30)
text = HEADER + 'return {\n  format = 1,\n  mode = "3d",\n  gravity = -20,\n  ' + VALE_ENV + ',\n' + eventos + '  objects = {\n' + '\n'.join(c.objs) + '\n  },\n}\n'
write(os.path.join(DEST, 'cenas', 'vale.cena'), text)
print('vale ok:', len(postas), 'árvores e pedras')
