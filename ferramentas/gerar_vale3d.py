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
ESPADA_RX = 20

def chama(c, y, cor='#FFA347', forca=1.6, alcance=7, tremular=0.5, fogo=True, tamanho=1.0):
    """A chama de uma tocha: a luz tremulando e o fogo em partículas."""
    comps = [comp('light3d', kind='point', color=cor, intensity=forca, range=alcance, angle=30,
                  soft=0.3, flicker=tremular, on=True)]
    if fogo:
        comps.append(particulas(
            rate=30, lifetime=0.5, lifetime_var=0.4, speed_min=0.3, speed_max=0.8, spread=25,
            gravity=1.5, area="circulo", area_radius=0.06 * tamanho, size_start=0.22 * tamanho,
            size_end=0.04, size_var=0.3, color_start="#FFE066", color_mid="#FF7A1A",
            color_end="#B0201000", blend="add"))
    return c.child('Chama', comps, y=y)

def heroi(c, x, y, z, ry=180, tocha=False, distancia=7):
    """O herói com a espada na mão direita (Golpe com o clique) e, com
    `tocha`, uma tocha acesa na mão esquerda."""
    filhos = [c.child('Espada', [comp('model', file='modelos/espada.glb', color='#FFFFFF', flat=False, shadow=True),
                                 comp('bone', bone='mao_d')], rx=ESPADA_RX)]
    if tocha:
        filhos.append(c.child('Tocha na mão', [comp('model', file='modelos/tocha.glb', color='#FFFFFF', flat=False, shadow=True, glow=0),
                                              comp('bone', bone='mao_e')],
                              y=-0.32, rx=15, scale_x=0.75, scale_y=0.75, scale_z=0.75,
                              children=[chama(c, 1.15, forca=1.5, alcance=8, tamanho=0.8)]))
    filhos.append(c.child('Poeira', [particulas(
        mode="explosao", amount=14, on_start=False, lifetime=0.6, lifetime_var=0.4,
        speed_min=0.8, speed_max=2, spread=200, gravity=0.5, drag=4, area="circulo",
        area_radius=0.3, size_start=0.35, size_end=0.6, color_start="#E8DCC4CC",
        color_end="#C8B89800")], y=0.05))
    return c.obj('Herói', [comp('model', file='modelos/heroi.glb', color='#FFFFFF', flat=False, shadow=True),
                           comp('behavior', script='nn:jogador3d', velocidade=6, pulo=8, pulos_no_ar=0, camera='atras',
                                distancia=distancia, prender_mouse=True, sensibilidade=0.15, inverter_y=False, virar=True,
                                agua='voltar', queda=-25, raio=0, altura=0),
                           vida_heroi(tremer=6),
                           comp('behavior', script='nn:golpe', quando='tecla', tecla='clique', alvo='', dano=1,
                                alcance=1.7, angulo=120, todos=True, empurrao=6, intervalo=0.45, animacao='golpe',
                                acerto=0.22, som='assets/sons/golpe.wav', som_acerto='assets/sons/acerto.wav')],
                 events=heroi_events, x=x, y=y, z=z, ry=ry, scale_x=1.1, scale_y=1.1, scale_z=1.1,
                 children=filhos)

hx, hz = 0.0, 36.0
heroi(c, hx, round(chao(hx, hz) + 0.05, 2), hz)

# A entrada da caverna, no pé do morro do leste.
EX, EZ = 15.0, -11.0

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
    if math.hypot(x - EX, z - EZ) < 7:
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

def tocha_de_pe(c, nome, x, y, z, rx=0, rz=0, ry=0):
    """Uma tocha (o modelo) com a chama acesa em cima."""
    return c.obj(nome, [comp('model', file='modelos/tocha.glb', color='#FFFFFF', flat=False, shadow=True, glow=0)],
                 x=x, y=y, z=z, rx=rx, ry=ry, rz=rz, children=[chama(c, 1.15)])

# A entrada da caverna: um arco de pedra no pé do morro, virado para a
# trilha, com uma tocha de cada lado. Entrar leva para a cena da caverna
# (guardando o vale como está).
tx, tz = 4.0, -6.0
ery = round(math.degrees(math.atan2(tx - EX, tz - EZ)))
eh = round(chao(EX, EZ), 2)
PEDRA = '#6E6A72'
def bloco(nome, x, y, z, sx, sy, sz, cor=PEDRA):
    return c.child(nome, [comp('shape3d', shape='cube', color=cor, shadow=True, glow=0),
                          comp('body3d', body='static')],
                   x=x, y=y, z=z, scale_x=sx, scale_y=sy, scale_z=sz)
c.obj('Caverna', [], x=EX, y=eh, z=EZ, ry=ery, children=[
    bloco('Pilar esquerdo', -1.9, 1.4, 0, 1.3, 4.2, 2.2),
    bloco('Pilar direito', 1.9, 1.4, 0, 1.3, 4.2, 2.2),
    bloco('Topo', 0, 3.9, 0, 5.2, 1.2, 2.4),
    bloco('Fundo', 0, 1.4, -1.2, 2.6, 4.2, 0.4, cor='#050507'),
    bloco('Encosta', 0, 1.0, -3.4, 7.5, 6.0, 4.0, cor='#5E5A63'),
    # Pedras em volta, para o arco parecer parte do morro.
    *[c.child('pedra', [comp('model', file='modelos/pedra.glb', color='#FFFFFF', flat=False, shadow=True, glow=0),
                        comp('body3d', body='static')],
              x=px, y=py, z=pz, ry=pr, scale_x=ps, scale_y=ps, scale_z=ps)
      for (px, py, pz, ps, pr) in [(-3.4, 0.5, -0.6, 2.6, 30), (3.5, 0.4, -0.3, 2.3, 200),
                                   (-2.2, 4.6, -1.3, 2.4, 80), (2.3, 4.5, -1.5, 2.8, 140),
                                   (0.2, 5.3, -3.0, 3.4, 260), (-3.8, 2.6, -2.6, 3.0, 10),
                                   (3.9, 2.4, -2.8, 3.2, 300)]],
    c.child('Placa da caverna', [comp('text3d', text='Caverna', size=0.55, color='#FFE2B8', facing='placa',
                                      outline='#2A1E14C0', background='', font='')], y=4.0, z=1.25),
    c.child('Entrada da caverna', [comp('shape3d', shape='cube', color='#FFD16640', shadow=False, glow=0),
                                   comp('behavior', script='nn:gatilho', alvo='', invisivel=True, mensagem='',
                                        mensagem_saida='', quem='', som='', uma_vez=False)],
            y=1.5, z=0.4, scale_x=2.4, scale_y=3, scale_z=1.2),
])
# As tochas dos lados (fora do arco, viradas para a trilha).
for lado in (-1, 1):
    a = math.radians(ery)
    ox, oz = math.cos(a) * 3.3 * lado + math.sin(a) * 1.4, -math.sin(a) * 3.3 * lado + math.cos(a) * 1.4
    x, z = round(EX + ox, 2), round(EZ + oz, 2)
    tocha_de_pe(c, 'Tocha da entrada', x, round(chao(x, z), 2), z)

c.obj('Placar', [comp('text', text='', size=30, color='#FFD93D', anchor='left', font='', hud=True),
                 comp('behavior', script='nn:mostrar_valor', texto='Estrelas {estrelas} / 6     Moedas {moedas}')],
      x=40, y=-44, anchor='top_left')
c.obj('Dica', [comp('text', text='Ache as 6 estrelas (uma na caverna)! Clique para golpear (Esc solta o mouse)', size=30, color='#FFFFFF', anchor='center', font='', hud=True)],
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
    { comment = "Entrou na caverna: vai para a cena dela, guardando o vale como está." },
    { when = {
        { cond = "entrar_zona", quem = "Entrada da caverna", por = "@jogador" },
      }, actions = {
        { act = "ir_para_cena", cena = "cenas/caverna.cena", guardar = true, escurecer = 0.6 },
      } },
    { comment = "Achou as 6 estrelas (uma na caverna): vitória!" },
    { when = {
        { cond = "variavel", var = "estrelas", op = ">=", valor = 6 },
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


# --- A caverna ----------------------------------------------------------------
# Um túnel escuro (ambiente Caverna: sem céu e sem sol), iluminado por tochas
# nas paredes, cristais que brilham, um lampião e um feixe de luz que entra
# por uma fresta no teto, em cima da estrela. O herói entra com uma tocha na
# mão. A saída volta para o vale (Fechar esta cena).
c = Cena()
FUNDO = -40.0
ROCHA, ROCHA2, TETO = '#4A4550', '#3E3A44', '#2E2B33'
def parede(nome, x, y, z, sx, sy, sz, cor=ROCHA2):
    return c.obj(nome, [comp('shape3d', shape='cube', color=cor, shadow=True, glow=0),
                        comp('body3d', body='static')],
                 x=x, y=y, z=z, scale_x=sx, scale_y=sy, scale_z=sz)
meio = (FUNDO + 2) / 2
parede('Chão', 0, -0.25, meio, 12, 0.5, -FUNDO + 6, cor=ROCHA)
parede('Parede esquerda', -6.5, 3, meio, 1, 7, -FUNDO + 6)
parede('Parede direita', 6.5, 3, meio, 1, 7, -FUNDO + 6)
parede('Teto', 0, 6.25, meio, 14, 0.5, -FUNDO + 6, cor=TETO)
parede('Parede do fundo', 0, 3, FUNDO - 1.5, 14, 7, 1)
# Na frente, a saída: uma abertura com a luz do dia lá fora.
parede('Frente esquerda', -4, 3, 3.5, 5, 7, 1)
parede('Frente direita', 4, 3, 3.5, 5, 7, 1)
parede('Frente de cima', 0, 5, 3.5, 3, 3, 1)
c.obj('Luz do dia', [comp('shape3d', shape='cube', color='#DDEEFF', shadow=False, glow=1)],
      x=0, y=1.75, z=4.2, scale_x=3, scale_y=3.5, scale_z=0.2)
c.obj('Claridade da saída', [comp('light3d', kind='spot', color='#CFE4FF', intensity=1.2, range=12, angle=40,
                                  soft=0.6, flicker=0, on=True)],
      x=0, y=3.2, z=3.6, rx=-20)

# Pedras e pontas de rocha pelo caminho (perto das paredes).
rnd = random.Random(7)
for i in range(14):
    lado = -1 if i % 2 == 0 else 1
    z = round(-3 - i * 2.6 + rnd.uniform(-0.8, 0.8), 2)
    x = round(lado * rnd.uniform(4.3, 5.4), 2)
    s = round(rnd.uniform(1.4, 2.6), 2)
    c.obj('pedra', [comp('model', file='modelos/pedra.glb', color='#8C8496', flat=False, shadow=True, glow=0),
                    comp('body3d', body='static')],
          x=x, y=round(0.15 * s, 2), z=z, ry=rnd.randrange(0, 360), scale_x=s, scale_y=s, scale_z=s)
for (x, z, h) in [(-2.8, -11, 1.6), (3.2, -17, 2.2), (-3.4, -27, 1.8), (2.6, -31, 1.3)]:
    c.obj('Ponta de rocha', [comp('shape3d', shape='cone', color='#5A5462', shadow=True, glow=0),
                             comp('body3d', body='static')],
          x=x, y=h / 2, z=z, scale_x=0.9, scale_y=h, scale_z=0.9)
    c.obj('Ponta do teto', [comp('shape3d', shape='cone', color='#4A4552', shadow=True, glow=0)],
          x=-x * 0.8, y=round(6 - h / 2, 2), z=z - 2, rx=180, scale_x=0.7, scale_y=h * 0.8, scale_z=0.7)

# Tochas nas paredes, inclinadas para o meio.
for k, z in enumerate([-5, -13, -21, -29]):
    for lado in (-1, 1):
        if (k + (lado > 0)) % 2:
            continue
        tocha_de_pe(c, 'Tocha', round(lado * 5.75, 2), 1.4, z, rz=lado * 22)

# Cristais que brilham, cada um com uma luz azul fraca.
for (x, z, s, ry) in [(-4.3, -17.5, 1.2, 20), (4.4, -25, 1.4, 160), (-4.2, -33.5, 1.0, 80), (4.1, -9, 0.8, 300)]:
    c.obj('Cristal', [comp('model', file='modelos/cristal.glb', color='#FFFFFF', flat=False, shadow=True, glow=0),
                      comp('body3d', body='static')],
          x=x, y=0, z=z, ry=ry, scale_x=s, scale_y=s, scale_z=s,
          children=[c.child('Luz do cristal', [comp('light3d', kind='point', color='#5CE1FF', intensity=1.1,
                                                    range=4.5, angle=30, soft=0.3, flicker=0, on=True)], y=0.7)])

# O fundo: a estrela num pedestal, embaixo da fresta, e um lampião ao lado.
c.obj('Pedestal', [comp('shape3d', shape='cylinder', color='#6E6A72', shadow=True, glow=0),
                   comp('body3d', body='static')],
      x=0, y=0.4, z=FUNDO + 3, scale_x=1.4, scale_y=0.8, scale_z=1.4)
c.obj('estrela', prefab='prefabs/estrela.prefab', x=0, y=1.4, z=FUNDO + 3)
c.obj('Fresta', [comp('light3d', kind='spot', color='#DDEBFF', intensity=4, range=9, angle=16, soft=0.4,
                      flicker=0, on=True)],
      x=0, y=6.1, z=FUNDO + 3, rx=-90)
# O feixe de luz: um cone claro e transparente, da fresta até o chão, com
# poeirinha brilhando dentro.
c.obj('Feixe', [comp('shape3d', shape='cone', color='#DDEBFF26', shadow=False, glow=1)],
      x=0, y=3.0, z=FUNDO + 3, scale_x=2.2, scale_y=6.0, scale_z=2.2,
      children=[c.child('Poeira no feixe', [particulas(
          rate=10, lifetime=3, lifetime_var=0.3, speed_min=0.05, speed_max=0.15, direction=90, spread=180,
          gravity=-0.03, area="circulo", area_radius=0.4, size_start=0.05, size_end=0.05,
          color_start="#FFFFFF00", color_mid="#F4F8FFCC", color_end="#FFFFFF00", blend="add")],
          y=-0.1, scale_x=0.4545, scale_y=0.1667, scale_z=0.4545)])
c.obj('Lampião', [comp('model', file='modelos/lampiao.glb', color='#FFFFFF', flat=False, shadow=True, glow=0)],
      x=2.4, y=0, z=FUNDO + 4.5, ry=20, scale_x=1.6, scale_y=1.6, scale_z=1.6,
      children=[c.child('Luz do lampião', [comp('light3d', kind='point', color='#FFD27A', intensity=1.3,
                                                range=6, angle=30, soft=0.3, flicker=0.15, on=True)], y=0.18)])

# Dois besouros no caminho.
besouro(c, -1.5, 0, -19, andar='patrulha', ax=3, combate=True)
besouro(c, 1.5, 0, -27, andar='vagar', raio=3, combate=True)

heroi(c, 0, 0.05, -3.5, ry=180, tocha=True, distancia=5)

c.obj('Saída', [comp('shape3d', shape='cube', color='#FFD16640', shadow=False, glow=0),
                comp('behavior', script='nn:gatilho', alvo='', invisivel=True, mensagem='', mensagem_saida='',
                     quem='', som='', uma_vez=False)],
      x=0, y=1.5, z=3.0, scale_x=3, scale_y=3, scale_z=0.6)

c.obj('Placar', [comp('text', text='', size=30, color='#FFD93D', anchor='left', font='', hud=True),
                 comp('behavior', script='nn:mostrar_valor', texto='Estrelas {estrelas} / 6     Moedas {moedas}')],
      x=40, y=-44, anchor='top_left')
c.obj('Dica', [comp('text', text='Uma caverna escura! A estrela está lá no fundo. (Para sair, volte para a luz.)',
                    size=32, color='#FFFFFF', anchor='center', font='', hud=True)],
      y=-120, anchor='top')
coracoes(c)

eventos = '''  events = {
    { comment = "A dica some depois de 5 segundos." },
    { when = {
        { cond = "depois_de", segundos = 5 },
      }, actions = {
        { act = "mostrar", acao = "esconder", alvo = "Dica" },
      } },
    { comment = "Derrotou um besouro: ganha 3 moedas." },
    { when = {
        { cond = "derrotar", quem = "@jogador", alvo = "Besouro" },
      }, actions = {
        { act = "variavel", var = "moedas", op = "somar", valor = 3 },
      } },
    { comment = "Voltou para a luz: fecha a caverna e o vale continua de onde parou." },
    { when = {
        { cond = "entrar_zona", quem = "Saída", por = "@jogador" },
      }, actions = {
        { act = "fechar_cena" },
      } },
  },
'''
CAVERNA_ENV = env(sky=False, sky_top="#3F8FD8", sky_horizon="#CDEBF5", sky_bottom="#9AA79A",
                  sun_height=50, sun_turn=120, sun_color="#000000", sun_size=0,
                  ambient_sky="#1C1C26", ambient_ground="#0C0B0A",
                  fog=True, fog_near=8, fog_far=35, shadows=False, shadow_distance=30)
text = (HEADER + 'return {\n  format = 1,\n  mode = "3d",\n  gravity = -20,\n  background = "#0A0A0E",\n  '
        + CAVERNA_ENV + ',\n' + eventos + '  objects = {\n' + '\n'.join(c.objs) + '\n  },\n}\n')
write(os.path.join(DEST, 'cenas', 'caverna.cena'), text)
print('caverna ok')
