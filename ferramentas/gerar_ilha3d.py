#!/usr/bin/env python3
"""Gera as cenas dos modelos de projeto "visual3d" (a ilha sem código) e
"vazio3d", e os sons da fogueira e do mar.

    python3 ferramentas/gerar_ilha3d.py

Depois, para deixar as cenas no formato que o NN Studio salva:

    NN_REESCREVER=1 cargo test -p nn-studio --lib modelos_ja_vem
"""
import math, os, random, struct

RAIZ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BASE = os.path.join(RAIZ, 'crates', 'nn-run', 'modelos')

def lua(v):
    if isinstance(v, bool): return 'true' if v else 'false'
    if isinstance(v, (int, float)):
        if isinstance(v, float):
            v = round(v, 3)
            if v == int(v): v = int(v)
        return str(v)
    if isinstance(v, str): return '"' + v.replace('"', '\\"') + '"'
    raise TypeError(v)

def comp(kind, **kw):
    return '{ type = %s%s }' % (lua(kind), ''.join(', %s = %s' % (k, lua(v)) for k, v in kw.items()))

class Cena:
    def __init__(self):
        self.objs = []
        self.next = 1
    def obj(self, name, comps=(), prefab=None, events=None, children=(), **t):
        i = self.next; self.next += 1
        anchor = t.pop('anchor', None)
        tr = dict(x=0, y=0, z=0, rx=0, ry=0, rz=0, scale_x=1, scale_y=1, scale_z=1)
        tr.update({k: v for k, v in t.items() if not k.startswith('_')})
        lines = ['    {', '      id = %d, name = %s,' % (i, lua(name))]
        lines.append('      ' + ', '.join('%s = %s' % (k, lua(tr[k])) for k in ['x', 'y', 'z', 'rx', 'ry', 'rz']) + ',')
        lines.append('      ' + ', '.join('%s = %s' % (k, lua(tr[k])) for k in ['scale_x', 'scale_y', 'scale_z']) + ', visible = true, layer = 0,')
        if anchor:
            lines.append('      anchor = %s,' % lua(anchor))
        if prefab:
            lines.append('      prefab = %s,' % lua(prefab))
        if comps:
            lines.append('      components = {')
            for c in comps: lines.append('        %s,' % c)
            lines.append('      },')
        if events:
            lines.append('      events = {')
            for e in events: lines.append(e)
            lines.append('      },')
        if children:
            # Os filhos já vêm prontos (texto de cada um), com recuo a mais.
            lines.append('      children = {')
            for ch in children:
                lines.append('\n'.join('    ' + l for l in ch.split('\n')))
            lines.append('      },')
        lines.append('    },')
        text = '\n'.join(lines)
        if t.get('_child'):
            return text
        self.objs.append(text)
        return i

    def child(self, name, comps=(), **t):
        """Um filho: devolve o texto (vai em children=[...] do pai)."""
        t['_child'] = True
        return self.obj(name, comps, **t)

def env(**kw):
    return 'environment = { ' + ', '.join('%s = %s' % (k, lua(v)) for k, v in kw.items()) + ' }'

ENV = env(sky=True, sky_top="#3F8FD8", sky_horizon="#CDEBF5", sky_bottom="#2E6F9E",
          sun_height=53, sun_turn=48, sun_color="#FFF1D6", sun_size=4,
          ambient_sky="#93AFCF", ambient_ground="#5E6B52",
          fog=True, fog_near=35, fog_far=90, shadows=True, shadow_distance=25)

def particulas(**kw):
    """Componente Partículas (os campos que faltam ficam no padrão)."""
    base = dict(mode="continuo", rate=20, emitting=True, amount=30, on_start=True, interval=0,
                when_done="nada", lifetime=1, lifetime_var=0, speed_min=1, speed_max=3,
                direction=90, spread=60, gravity=0, drag=0, area="ponto", area_w=2, area_h=2,
                area_radius=0.5, size_start=0.2, size_end=0, size_var=0, color_start="#FFFFFF",
                color_mid="", color_end="#FFFFFF00", image="", frame=1, frame_w=0, frame_h=0,
                spin=0, blend="alpha", follow=False, max=500)
    base.update(kw)
    return comp('particles', **base)

def som(arquivo, distancia, volume=1):
    return comp('sound', sound=arquivo, autoplay=True, loop=True, volume=volume, channel="effects",
                spatial=True, distance=distancia)

HEADER = '''-- Cena visual do NN Studio. Dá para editar à mão; ao salvar pelo editor,
-- comentários escritos aqui se perdem.
'''

def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    open(path, 'w').write(text)

# ---------------------------------------------------------------- visual3d
c = Cena()
c.obj('Mar', [comp('shape3d', shape='plane', color='#3FA7D6', shadow=False)], y=-0.4, scale_x=400, scale_z=400)
c.obj('Areia', [comp('shape3d', shape='cylinder', color='#F2D49B', shadow=False)], y=-0.55, scale_x=29.5, scale_z=29.5)
c.obj('Ilha', [comp('shape3d', shape='cylinder', color='#7BC86C'), comp('body3d', body='static')],
      y=-2, scale_x=28, scale_y=4, scale_z=28)
heroi_events = [
    '        { when = {',
    '            { cond = "mensagem", quem = "@este", msg = "pulou" },',
    '          }, actions = {',
    '            { act = "som", som = "assets/sons/pulo.wav", volume = 0.5, onde = "@este", distancia = 30 },',
    '            { act = "soltar_particulas", alvo = "Poeira", quantidade = 14 },',
    '          } },',
    '        { when = {',
    '            { cond = "mensagem", quem = "@este", msg = "caiu" },',
    '          }, actions = {',
    '            { act = "som", som = "assets/sons/dano.wav", volume = 1, onde = "@este", distancia = 30 },',
    '          } },',
]
c.obj('Herói', [comp('model', file='modelos/heroi.glb', color='#FFFFFF', flat=False, shadow=True),
                comp('behavior', script='nn:jogador3d', velocidade=6, pulo=8, pulos_no_ar=0, camera='atras',
                     distancia=7, prender_mouse=True, sensibilidade=0.15, inverter_y=False, virar=True,
                     queda=-3, raio=0, altura=0)],
      events=heroi_events, z=9, ry=180, scale_x=1.1, scale_y=1.1, scale_z=1.1,
      children=[c.child('Poeira', [particulas(
          mode="explosao", amount=14, on_start=False, lifetime=0.6, lifetime_var=0.4,
          speed_min=0.8, speed_max=2, spread=200, gravity=0.5, drag=4, area="circulo",
          area_radius=0.3, size_start=0.35, size_end=0.6, color_start="#E8DCC4CC",
          color_end="#C8B89800")], y=0.05)])

# A fogueira: troncos, fogo, fumaça e o estalar (som 3D).
c.obj('Fogueira', [comp('shape3d', shape='cylinder', color='#5B3A29', shadow=True),
                   comp('body3d', body='static'), som('assets/sons/fogueira.wav', 14)],
      x=-4, z=6.5, scale_x=0.9, scale_y=0.15, scale_z=0.9,
      children=[
          # (Os filhos ficam dentro da escala da fogueira: 1 em y = 0,15 m.)
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

# O barulho do mar nas praias (som 3D; cada um se ouve de perto).
for (x, z) in [(16, -6), (-14, 10), (6, 17)]:
    c.obj('Ondas', [som('assets/sons/mar.wav', 18, 0.7)], x=x, z=z)

def plat(name, x, y, z, w, d, color='#FFFFFF', body='static', extra=()):
    comps = [comp('model', file='modelos/plataforma.glb', color=color, flat=False, shadow=True),
             comp('body3d', body=body)] + list(extra)
    return c.obj(name, comps, x=x, y=y, z=z, scale_x=w, scale_z=d)

for (x, y, z, w, d) in [(-4, 1, 2, 3, 3), (-8, 2, -2, 3, 3), (-6, 3.2, -7, 3, 3), (10, 4.5, -10, 3, 3),
                        (11, 5.8, -5.5, 2.5, 2.5), (8, 7.2, -2, 2.5, 2.5), (10.5, 8.5, 1.5, 2.5, 2.5), (0, 8, 0, 4, 4)]:
    plat('Plataforma', x, y, z, w, d)
plat('Plataforma que anda', 0, 3.5, -10, 3, 3, '#FFE6B0', 'kinematic',
     [comp('behavior', script='nn:plataforma3d', x=6, y=0, z=0, duracao=3.75, espera=0.5)])
plat('Elevador', 3.2, 0.3, 0, 2, 2, '#FFE6B0', 'kinematic',
     [comp('behavior', script='nn:plataforma3d', x=0, y=7.6, z=0, duracao=5.8, espera=1)])

# Árvores e pedras (fixas).
arvores = [(-9, -6, 1.3, 20), (-11, 3, 1.1, 80), (-5, 9, 1.4, 140), (6, -5, 1.2, 200), (9, 4, 1.0, 260),
           (-2, -11, 1.5, 300), (12, -2, 1.3, 40), (-12, -2, 1.2, 110)]
for (x, z, s, r) in arvores:
    c.obj('Árvore', [comp('model', file='modelos/arvore.glb', color='#FFFFFF', flat=False, shadow=True),
                     comp('body3d', body='static')], x=x, z=z, ry=r, scale_x=s, scale_y=s, scale_z=s)
for (x, z, s, r) in [(-7, 6, 1.8, 30), (5, 6, 1.5, 90), (11, 9, 2.2, 150), (-10, -9, 1.7, 220)]:
    c.obj('Pedra', [comp('model', file='modelos/pedra.glb', color='#FFFFFF', flat=False, shadow=True),
                    comp('body3d', body='static')], x=x, y=0.26 * s, z=z, ry=r, scale_x=s, scale_y=s, scale_z=s)
for (x, z) in [(2, 4), (2.2, 5.3), (2.1, 4.65)]:
    pass
c.obj('Caixote', [comp('model', file='modelos/caixote.glb', color='#FFFFFF', flat=False, shadow=True),
                  comp('body3d', body='dynamic', mass=8)], x=-2, z=5)
c.obj('Caixote', [comp('model', file='modelos/caixote.glb', color='#FFFFFF', flat=False, shadow=True),
                  comp('body3d', body='dynamic', mass=8)], x=-0.8, z=5.2, ry=15)
c.obj('Caixote', [comp('model', file='modelos/caixote.glb', color='#FFFFFF', flat=False, shadow=True),
                  comp('body3d', body='dynamic', mass=8)], x=-1.4, y=1, z=5.1, ry=40)

# Moedas (prefab) no chão e nas plataformas.
moedas = []
for i in range(8):
    a = math.radians(200 + i * 18)
    moedas.append((math.cos(a) * 9, 0.6, math.sin(a) * 9))
for (x, y, z) in [(-4, 1, 2), (-8, 2, -2), (11, 5.8, -5.5), (8, 7.2, -2)]:
    moedas.append((x, y + 0.6, z))
for i in range(4):
    moedas.append((6 + i * 1.5, 0.6, 9 - i * 1.2))
for (x, y, z) in moedas:
    c.obj('moeda', prefab='prefabs/moeda.prefab', x=x, y=y, z=z)
for (x, y, z) in [(-6, 4, -7), (10, 5.3, -10), (10.5, 9.3, 1.5), (0, 8.8, 0), (-3, 0.9, 5)]:
    c.obj('estrela', prefab='prefabs/estrela.prefab', x=x, y=y - 0.4, z=z)

c.obj('Placar', [comp('text', text='', size=30, color='#FFD93D', anchor='left', font='', hud=True),
                 comp('behavior', script='nn:mostrar_valor', texto='Estrelas {estrelas} / 5     Moedas {moedas}')],
      x=40, y=-44, anchor='top_left')
c.obj('Dica', [comp('text', text='Ache as 5 estrelas! (Esc solta o mouse)', size=34, color='#FFFFFF', anchor='center', font='', hud=True)],
      y=-120, anchor='top')

def anchor_fix(text):
    # "anchor" é campo do objeto: a função obj não sabe dele, vai como transformação extra.
    return text

scene_events = '''  events = {
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
text = HEADER + 'return {\n  format = 1,\n  mode = "3d",\n  gravity = -20,\n  ' + ENV + ',\n' + scene_events + '  objects = {\n' + '\n'.join(c.objs) + '\n  },\n}\n'
write(BASE + '/visual3d/cenas/ilha.cena', text)

PREFAB_HEADER = '''-- Prefab do NN Studio: um objeto pronto para usar em várias cenas.
-- Dá para editar à mão; ao salvar pelo editor, comentários se perdem.
'''
def prefab(path, name, comps):
    lines = [PREFAB_HEADER + 'return {', '  format = 1,', '  object = {',
             '    id = 1, name = %s,' % lua(name),
             '    x = 0, y = 0, z = 0, rx = 0, ry = 0, rz = 0,',
             '    scale_x = 1, scale_y = 1, scale_z = 1, visible = true, layer = 0,',
             '    components = {']
    for cc in comps: lines.append('      %s,' % cc)
    lines += ['    },', '  },', '}', '']
    write(path, '\n'.join(lines))

prefab(BASE + '/visual3d/prefabs/moeda.prefab', 'moeda', [
    comp('model', file='modelos/moeda.glb', color='#FFFFFF', flat=False, shadow=False),
    comp('behavior', script='nn:coletavel', quem='', variavel='moedas', valor=1, som='assets/sons/moeda.wav', brilho=True, cor_brilho='#FFD166'),
    comp('behavior', script='nn:girar3d', velocidade=180, eixo='y', altura=0.12, duracao=3),
])
prefab(BASE + '/visual3d/prefabs/estrela.prefab', 'estrela', [
    comp('model', file='modelos/estrela.glb', color='#FFFFFF', flat=False, shadow=True),
    comp('behavior', script='nn:coletavel', quem='', variavel='estrelas', valor=1, som='assets/sons/vitoria.wav', brilho=True, cor_brilho='#FFD166'),
    comp('behavior', script='nn:girar3d', velocidade=90, eixo='y', altura=0.15, duracao=3),
])

# ---------------------------------------------------------------- vazio3d
v = Cena()
v.obj('Câmera', [comp('camera3d', fov=60)], y=5, z=10, rx=-22)
v.obj('Chão', [comp('shape3d', shape='plane', color='#7BC86C', shadow=True), comp('body3d', body='static')],
      scale_x=30, scale_z=30)
text = HEADER + 'return {\n  format = 1,\n  mode = "3d",\n  gravity = -20,\n  ' + ENV + ',\n  objects = {\n' + '\n'.join(v.objs) + '\n  },\n}\n'
write(BASE + '/vazio3d/cenas/mundo.cena', text)
print('ok')


# ---------------------------------------------------------------- sons
def wav(path, rate, samples):
    data = b''.join(struct.pack('<h', max(-32767, min(32767, int(v * 32767)))) for v in samples)
    head = (b'RIFF' + struct.pack('<I', 36 + len(data)) + b'WAVEfmt ' +
            struct.pack('<IHHIIHH', 16, 1, 1, rate, rate * 2, 2, 16) + b'data' + struct.pack('<I', len(data)))
    os.makedirs(os.path.dirname(path), exist_ok=True)
    open(path, 'wb').write(head + data)

def fogueira(rate=16000, segundos=3.0):
    """Fogo estalando: um chiado grave e estalos soltos (dá para repetir)."""
    rnd = random.Random(5)
    n = int(rate * segundos)
    out, lp = [], 0.0
    estalos = [rnd.randrange(n) for _ in range(int(segundos * 9))]
    for i in range(n):
        lp += (rnd.uniform(-1, 1) - lp) * 0.04
        out.append(lp * 0.9)
    for e in estalos:
        forte = rnd.uniform(0.3, 0.9)
        dur = rnd.randrange(80, 400)
        for k in range(dur):
            j = (e + k) % n
            out[j] += rnd.uniform(-1, 1) * forte * (1 - k / dur) ** 3
    return out

def mar(rate=16000, segundos=6.0):
    """Ondas: um chiado que cresce e some (o ciclo fecha no fim)."""
    rnd = random.Random(9)
    n = int(rate * segundos)
    out, lp, lp2 = [], 0.0, 0.0
    for i in range(n):
        lp += (rnd.uniform(-1, 1) - lp) * 0.08
        lp2 += (lp - lp2) * 0.3
        t = i / n
        onda = 0.15 + 0.85 * (0.5 - 0.5 * math.cos(2 * math.pi * t)) ** 2
        out.append(lp2 * 2.2 * onda)
    return out

comum = os.path.join(BASE, 'comum', 'sons')
wav(os.path.join(comum, 'fogueira.wav'), 16000, fogueira())
wav(os.path.join(comum, 'mar.wav'), 16000, mar())
print('sons ok')
