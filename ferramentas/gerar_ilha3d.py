#!/usr/bin/env python3
"""Gera as cenas dos modelos de projeto "visual3d" (a ilha sem código, com a
arena do besouro-rei do outro lado da ponte) e "vazio3d", e os sons da
fogueira e do mar.

    python3 ferramentas/gerar_ilha3d.py

Depois, para deixar as cenas no formato que o NN Studio salva:

    NN_REESCREVER=1 cargo test -p nn-studio --lib modelos_ja_vem
"""
import math, os, random, struct, sys

sys.dont_write_bytecode = True

from cenas3d import Cena, ENV, HEADER, besouro, comp, coracoes, env, lua, particulas, som, vida_heroi, write

RAIZ = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BASE = os.path.join(RAIZ, 'crates', 'nn-run', 'modelos')

# ---------------------------------------------------------------- visual3d
c = Cena()
# O mar: Material Água (ondinhas, o céu refletido e o brilho do sol).
c.obj('Mar', [comp('shape3d', shape='plane', color='#2E86C1', shadow=False, material='agua')], y=-0.4, scale_x=400, scale_z=400)
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
                     queda=-3, raio=0, altura=0),
                vida_heroi(tremer=6),
                comp('behavior', script='nn:golpe', quando='tecla', tecla='clique', alvo='', dano=1,
                     alcance=1.7, angulo=120, todos=True, empurrao=6, intervalo=0.45, animacao='golpe',
                     acerto=0.22, som='assets/sons/golpe.wav', som_acerto='assets/sons/acerto.wav')],
      events=heroi_events, z=9, ry=180, scale_x=1.1, scale_y=1.1, scale_z=1.1,
      children=[c.child('Espada', [comp('model', file='modelos/espada.glb', color='#FFFFFF', flat=False, shadow=True),
                                   comp('bone', bone='mao_d')], rx=20),
                c.child('Poeira', [particulas(
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
              rate=40, lifetime=0.8, lifetime_var=0.4, speed_min=0.6, speed_max=1.6, spread=25,
              gravity=1.5, area="circulo", area_radius=0.3, size_start=0.4, size_end=0.1,
              size_var=0.3, color_start="#FFE066", color_mid="#FF7A1A", color_end="#B0201000",
              blend="add", wind=0.3), comp('light3d', kind='point', color='#FF9A3C', intensity=1.6, range=8,
              angle=30, soft=0.3, flicker=0.6, on=True, night=False)], y=6),
          c.child('Fumaça', [particulas(
              rate=8, lifetime=3, lifetime_var=0.3, speed_min=0.5, speed_max=1, spread=30,
              gravity=0.4, drag=0.3, area="circulo", area_radius=0.2, size_start=0.4,
              size_end=1.8, color_start="#9A9A9AAA", color_end="#5A5A5A00", spin=30)], y=8),
      ])

# Postes com lampião: acendem sozinhos ao anoitecer (Luz e Brilho "só à
# noite"; a ilha tem Dia e noite, em ☀ Ambiente).
def poste(x, z, ry=0):
    return c.obj('Poste', [], x=x, z=z, ry=ry, children=[
        c.child('Haste', [comp('shape3d', shape='cylinder', color='#3A3A42', shadow=True, glow=0, glow_night=False),
                          comp('body3d', body='static')],
                y=1.3, scale_x=0.12, scale_y=2.6, scale_z=0.12),
        c.child('Braço', [comp('shape3d', shape='cube', color='#3A3A42', shadow=True, glow=0, glow_night=False)],
                y=2.55, z=0.25, scale_x=0.06, scale_y=0.06, scale_z=0.55),
        c.child('Lampião', [comp('model', file='modelos/lampiao.glb', color='#FFFFFF', flat=False, shadow=True,
                                 glow=0, glow_night=True)],
                y=2.08, z=0.48, scale_x=1.1, scale_y=1.1, scale_z=1.1,
                children=[c.child('Luz do poste', [comp('light3d', kind='point', color='#FFD27A', intensity=1.5,
                                                        range=8, angle=30, soft=0.3, flicker=0.08, on=True,
                                                        night=True)], y=0.18)]),
    ])
for (x, z, ry) in [(3.4, 3.2, 200), (4.3, 6.4, 120), (-11.8, 1.9, 90), (-11.8, -1.9, 90),
                   (0.5, -6.5, 0), (8.5, -3.5, 300), (-6.5, -7.5, 40)]:
    poste(x, z, ry)

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
    c.obj('Árvore', [comp('model', file='modelos/arvore.glb', color='#FFFFFF', flat=False, shadow=True, sway=1),
                     comp('body3d', body='static')], x=x, z=z, ry=r, scale_x=s, scale_y=s, scale_z=s)
# Grama: tufos espalhados, balançando com o vento.
rg = random.Random(5)
gramas = 0
while gramas < 70:
    a, r = rg.uniform(0, 2 * math.pi), 13 * math.sqrt(rg.random())
    x, z = round(math.cos(a) * r, 2), round(math.sin(a) * r, 2)
    # Longe do cofre, da fogueira, do pescador e do começo do herói.
    if any(math.hypot(x - px, z - pz) < d for px, pz, d in [(7.5, 7.5, 3.2), (-4, 6.5, 1.5), (1.8, 4.8, 1.2), (0, 9, 1.5)]):
        continue
    gramas += 1
    s = round(rg.uniform(0.9, 1.6), 2)
    c.obj('Grama', [comp('model', file='modelos/grama.glb', color='#FFFFFF', flat=False, shadow=False, sway=1.5)],
          x=x, z=z, ry=rg.randrange(0, 360), scale_x=s, scale_y=s, scale_z=s)
for (x, z, s, r) in [(-7, 6, 1.8, 30), (3.5, 11.5, 1.5, 90), (11.5, 3, 2.2, 150), (-10, -9, 1.7, 220)]:
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
for (x, y, z) in [(-4, 1, 2), (11, 5.8, -5.5), (8, 7.2, -2)]:
    moedas.append((x, y + 0.6, z))
# Um caminho de moedas até a porta do cofre.
for i in range(3):
    moedas.append((1.5 + i * 1.3, 0.6, 8.6 - i * 0.3))
for (x, y, z) in moedas:
    c.obj('moeda', prefab='prefabs/moeda.prefab', x=x, y=y, z=z)
for (x, y, z) in [(-6, 4, -7), (10, 5.3, -10), (10.5, 9.3, 1.5), (0, 8.8, 0), (7.5, 0.9, 7.5)]:
    c.obj('estrela', prefab='prefabs/estrela.prefab', x=x, y=y - 0.4, z=z)

# A arena do besouro-rei: outra ilha a oeste, ligada por uma ponte. Entrando,
# o portão fecha e vêm 3 ondas de besouros; depois, o besouro-rei. Derrotado,
# ele deixa a sexta estrela e o portão abre.
AX, AR = -27.0, 9.0
c.obj('Areia da arena', [comp('shape3d', shape='cylinder', color='#F2D49B', shadow=False)],
      x=AX, y=-0.55, scale_x=AR * 2 + 1.5, scale_z=AR * 2 + 1.5)
c.obj('Arena', [comp('shape3d', shape='cylinder', color='#C9B48A'), comp('body3d', body='static')],
      x=AX, y=-2, scale_x=AR * 2, scale_y=4, scale_z=AR * 2)
for i in range(4):
    c.obj('Ponte', [comp('model', file='modelos/plataforma.glb', color='#C89B6D', flat=False, shadow=True),
                    comp('body3d', body='static')], x=-14.6 - i * 0.95, z=0, scale_x=0.9, scale_y=0.5, scale_z=2.6)
# A cerca: pilares de pedra em volta, menos no portão (virado para a ponte, em +X).
for i in range(28):
    a = 2 * math.pi * i / 28
    if math.cos(a) > 0.95:
        continue
    c.obj('Pilar', [comp('shape3d', shape='cube', color='#8E939C', shadow=True), comp('body3d', body='static')],
          x=round(AX + math.cos(a) * (AR - 0.5), 2), y=1.1, z=round(math.sin(a) * (AR - 0.5), 2),
          ry=round(-math.degrees(a), 1), scale_x=0.7, scale_y=2.2, scale_z=0.9)
c.obj('Portão da arena', [
    comp('shape3d', shape='cube', color='#6B4226', shadow=True),
    comp('body3d', body='kinematic'),
    comp('behavior', script='nn:porta3d', abre='descendo', lado='esquerda', angulo=100, tempo=0.8,
         abrir_com='mensagem', distancia=1.2, chave='', gastar_chave=True, fechar_depois=0,
         texto='abrir', texto_fechar='fechar', texto_trancada='Precisa de: {item}',
         som='assets/sons/porta.wav', som_trancada='assets/sons/dano.wav'),
], x=AX + AR - 0.5, y=1.2, z=0, scale_x=0.5, scale_y=2.4, scale_z=3.4)
c.obj('Zona da arena', [comp('shape3d', shape='cube', color='#FFD16640', shadow=False),
                        comp('behavior', script='nn:gatilho', quem='', alvo='', mensagem='', mensagem_saida='',
                             uma_vez=False, som='', invisivel=True)],
      x=AX, y=1.5, z=0, scale_x=12, scale_y=3, scale_z=12)
# Uma bandeira no alto de um mastro, na arena (balança com o vento).
c.obj('Mastro', [comp('shape3d', shape='cylinder', color='#5A5462', shadow=True), comp('body3d', body='static')],
      x=AX - 6, y=2.5, z=-4, scale_x=0.12, scale_y=5, scale_z=0.12)
c.obj('Bandeira', [comp('shape3d', shape='cube', color='#E63946', shadow=True, sway=2)],
      x=AX - 6, y=4.55, z=-3.35, scale_x=0.04, scale_y=0.8, scale_z=1.2)
pontos = [c.child('Saída', [], x=round(math.cos(a) * 6, 2), z=round(math.sin(a) * 6, 2))
          for a in (math.radians(d) for d in (60, 130, 230, 300))]
c.obj('Ondas da arena', [comp('behavior', script='nn:ondas', prefab='prefabs/besouro.prefab', prefab_forte='',
                     forte_a_partir=2, ondas=3, primeira=2, a_mais=1, quando='mensagem', distancia=6,
                     pausa=3, intervalo=0.6, raio=7, longe=3, letreiro=True, texto='Onda {onda}',
                     texto_fim='', var_onda='onda', var_restantes='restantes', som='assets/sons/porta.wav',
                     cor='#B36BFF')],
      x=AX, z=0, children=pontos)
c.obj('Besouro-rei', [
    comp('model', file='modelos/besourao.glb', color='#FFFFFF', flat=False, shadow=True),
    comp('behavior', script='nn:vida', vidas=14, variavel='', invencivel=0.25, ao_acabar='nada', cena='',
         piscar=False, barra=False, cor_barra='#FF5A5A', numeros=True),
    comp('behavior', script='nn:chefe', nome='Besouro-rei', quando='mensagem', distancia=10, velocidade=2.4,
         pausa=1.4, aviso=0.8, dano=1, golpe=True, alcance=3, investida=True, velocidade_investida=11,
         tonto=1.8, pulo=True, raio_impacto=4.5, leque=True, tiro='prefabs/magia.prefab', tiros=5,
         abertura=70, ajudantes=True, ajudante='prefabs/besouro.prefab', quantos=2, furia=50, barra=True,
         cor_barra='#FF5A5A', solta='prefabs/estrela.prefab', som_aviso='assets/sons/rugido.wav', som_impacto='assets/sons/explosao.wav'),
], x=AX - 4, z=0, ry=90, scale_x=1.5, scale_y=1.5, scale_z=1.5)
c.obj('Placar da arena', [comp('text', text='', size=30, color='#FFFFFF', anchor='right', font='', hud=True, outline='#00000099'),
                          comp('behavior', script='nn:mostrar_valor', texto='Onda {onda} / 3     Faltam {restantes}')],
      x=-40, y=-44, anchor='top_right', visible=False)

# O cofre: uma casinha de pedra com uma porta trancada e uma estrela dentro.
# A chave fica numa plataforma (vai para o inventário quando o herói encosta).
CX, CZ, LADO, ALTO, ESP = 7.5, 7.5, 3.4, 2.6, 0.3
PEDRA = '#A9AEB8'
def parede(x, y, z, sx, sy, sz, cor=PEDRA):
    c.obj('Parede', [comp('shape3d', shape='cube', color=cor, shadow=True), comp('body3d', body='static')],
          x=x, y=y, z=z, scale_x=sx, scale_y=sy, scale_z=sz)
meio = LADO / 2 - ESP / 2
parede(CX, ALTO / 2, CZ - meio, LADO, ALTO, ESP)
parede(CX, ALTO / 2, CZ + meio, LADO, ALTO, ESP)
parede(CX + meio, ALTO / 2, CZ, ESP, ALTO, LADO - 2 * ESP)
# A frente (virada para o herói, em -X) com o vão da porta (1,2 m) no meio.
vao, dentro = 1.2, LADO - 2 * ESP
pedaco = (dentro - vao) / 2
for lado in (-1, 1):
    parede(CX - meio, ALTO / 2, CZ + lado * (vao / 2 + pedaco / 2), ESP, ALTO, pedaco)
parede(CX - meio, 2.2 + (ALTO - 2.2) / 2, CZ, ESP, ALTO - 2.2, vao)
parede(CX, ALTO + 0.12, CZ, LADO + 0.4, 0.25, LADO + 0.4, '#8B5A3C')
c.obj('Porta do cofre', [
    comp('model', file='modelos/porta.glb', color='#FFFFFF', flat=False, shadow=True),
    comp('body3d', body='kinematic'),
    comp('behavior', script='nn:porta3d', abre='girando', lado='esquerda', angulo=100, tempo=0.6,
         abrir_com='tecla', distancia=1.2, chave='chave', gastar_chave=True, fechar_depois=0,
         texto='abrir', texto_fechar='fechar', texto_trancada='Precisa de: {item}',
         som='assets/sons/porta.wav', som_trancada='assets/sons/dano.wav'),
], x=CX - meio, z=CZ, ry=-90)
c.obj('Placa do cofre', [comp('text3d', text='COFRE', size=0.32, color='#3A2E1F', facing='placa',
                              outline='', background='#F2D49B', font='')],
      x=CX - meio - ESP / 2 - 0.02, y=2.4, z=CZ, ry=-90)
c.obj('Chave', [
    comp('model', file='modelos/chave.glb', color='#FFFFFF', flat=False, shadow=True),
    comp('behavior', script='nn:coletavel', quem='', variavel='', valor=1, item='chave',
         icone='assets/imagens/chave.png', som='assets/sons/chave.wav', brilho=True, cor_brilho='#FFD166'),
    comp('behavior', script='nn:girar3d', velocidade=120, eixo='y', altura=0.15, duracao=2),
], x=-8, y=2.1, z=-2, scale_x=1.6, scale_y=1.6, scale_z=1.6)

# O pescador: conversa com o herói (comportamento Diálogo, dialogos/pescador.dialogo).
c.obj('Pescador', [
    comp('model', file='modelos/heroi.glb', color='#9FD3FF', flat=False, shadow=True),
    comp('anim3d', animation='parado', speed=1, loop=True),
    comp('body3d', body='static'),
    comp('behavior', script='nn:dialogo', arquivo='dialogos/pescador.dialogo', comecar='tecla',
         texto='falar', distancia=1.5, uma_vez=False, olhar=True),
], x=1.8, z=4.8, ry=-23,
   children=[c.child('Nome', [comp('text3d', text='Pescador', size=0.22, color='#FFFFFF', facing='camera',
                                    outline='#000000B0', background='', font='')], y=2.15),
             # De noite, ele acende o lampião (na mão esquerda).
             c.child('Lampião do pescador', [comp('model', file='modelos/lampiao.glb', color='#FFFFFF', flat=False,
                                                  shadow=True, glow=0, glow_night=True),
                                             comp('bone', bone='mao_e')],
                     y=-0.42, children=[c.child('Luz do lampião', [comp('light3d', kind='point', color='#FFD27A',
                                                                         intensity=1.2, range=6, angle=30, soft=0.3,
                                                                         flicker=0.1, on=True, night=True)], y=0.18)])])

c.obj('Placar', [comp('text', text='', size=30, color='#FFD93D', anchor='left', font='', hud=True, outline='#00000099'),
                 comp('behavior', script='nn:mostrar_valor', texto='Estrelas {estrelas} / 6     Moedas {moedas}     {relogio}')],
      x=40, y=-44, anchor='top_left')
c.obj('Dica', [comp('text', text='Ache as 6 estrelas! Uma é do besouro-rei, na arena. (Esc solta o mouse)', size=30, color='#FFFFFF', anchor='center', font='', hud=True, outline='#00000099')],
      y=-120, anchor='top')
coracoes(c)
c.obj('Inventário', [comp('behavior', script='nn:inventario', canto='baixo_esquerda', tamanho=56, margem=24, fundo=True)])

# Besouros: andam pela ilha, perseguem o herói e morrem se ele pular em cima.
besouro(c, -9, 0, 5, andar='patrulha', ax=0, az=-5)
besouro(c, 6, 0, -2, andar='vagar', raio=3)

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
    { comment = "A arena: o portão começa aberto." },
    { when = {
        { cond = "comecou" },
      }, actions = {
        { act = "mensagem", msg = "abrir", alvo = "Portão da arena" },
        { act = "variavel", var = "arena", op = "definir", valor = 0 },
      } },
    { comment = "Entrou na arena: o portão fecha e as ondas começam." },
    { when = {
        { cond = "entrar_zona", quem = "Zona da arena", por = "@jogador" },
        { cond = "variavel", var = "arena", op = "==", valor = 0 },
      }, actions = {
        { act = "variavel", var = "arena", op = "definir", valor = 1 },
        { act = "mensagem", msg = "fechar", alvo = "Portão da arena" },
        { act = "mostrar", acao = "mostrar", alvo = "Placar da arena" },
        { act = "esperar", segundos = 1 },
        { act = "mensagem", msg = "comecar", alvo = "Ondas da arena" },
      } },
    { comment = "Venceu as ondas: o besouro-rei acorda." },
    { when = {
        { cond = "ondas_terminaram", quem = "Ondas da arena" },
      }, actions = {
        { act = "mostrar", acao = "esconder", alvo = "Placar da arena" },
        { act = "tremer", forca = 10, tempo = 0.6 },
        { act = "mensagem", msg = "comecar", alvo = "Besouro-rei" },
      } },
    { comment = "Derrotou o besouro-rei: o portão abre." },
    { when = {
        { cond = "vida_acabou", quem = "Besouro-rei" },
      }, actions = {
        { act = "esperar", segundos = 1.5 },
        { act = "mensagem", msg = "abrir", alvo = "Portão da arena" },
      } },
    { comment = "Achou as 6 estrelas: vitória!" },
    { when = {
        { cond = "variavel", var = "estrelas", op = ">=", valor = 6 },
      }, actions = {
        { act = "som", som = "assets/sons/vitoria.wav", volume = 1 },
        { act = "esperar", segundos = 1.2 },
        { act = "ir_para_cena", cena = "cenas/fim.cena", escurecer = 0.8 },
      } },
  },
'''
# A ilha tem Dia e noite: um dia de 4 minutos, começando às 9h.
ILHA_ENV = env(sky=True, sky_top="#3F8FD8", sky_horizon="#CDEBF5", sky_bottom="#2E6F9E",
               sun_height=53, sun_turn=48, sun_color="#FFF1D6", sun_size=4,
               ambient_sky="#93AFCF", ambient_ground="#5E6B52",
               fog=True, fog_near=35, fog_far=90, shadows=True, shadow_distance=25,
               cycle=True, day_length=4, start_hour=9, wind=1, wind_turn=60)
text = HEADER + 'return {\n  format = 1,\n  mode = "3d",\n  gravity = -20,\n  ' + ILHA_ENV + ',\n' + scene_events + '  objects = {\n' + '\n'.join(c.objs) + '\n  },\n}\n'
write(BASE + '/visual3d/cenas/ilha.cena', text)

PREFAB_HEADER = '''-- Prefab do NN Studio: um objeto pronto para usar em várias cenas.
-- Dá para editar à mão; ao salvar pelo editor, comentários se perdem.
'''
def prefab(path, name, comps, escala=1):
    lines = [PREFAB_HEADER + 'return {', '  format = 1,', '  object = {',
             '    id = 1, name = %s,' % lua(name),
             '    x = 0, y = 0, z = 0, rx = 0, ry = 0, rz = 0,',
             '    scale_x = %s, scale_y = %s, scale_z = %s, visible = true, layer = 0,' % (lua(escala), lua(escala), lua(escala)),
             '    components = {']
    for cc in comps: lines.append('      %s,' % cc)
    lines += ['    },', '  },', '}', '']
    write(path, '\n'.join(lines))

prefab(BASE + '/visual3d/prefabs/moeda.prefab', 'moeda', [
    comp('model', file='modelos/moeda.glb', color='#FFFFFF', flat=False, shadow=False),
    comp('behavior', script='nn:coletavel', quem='', variavel='moedas', valor=1, som='assets/sons/moeda.wav', brilho=True, cor_brilho='#FFD166'),
    comp('behavior', script='nn:girar3d', velocidade=180, eixo='y', altura=0.12, duracao=3),
])
prefab(BASE + '/visual3d/prefabs/besouro.prefab', 'besouro', [
    comp('model', file='modelos/bicho.glb', color='#FFFFFF', flat=False, shadow=True),
    comp('behavior', script='nn:inimigo3d', andar='parado', x=0, z=0, raio=4, velocidade=1.8,
         perseguir=True, visao=20, correr=3.2, contornar=True, dano=1, empurrao=7, pisar=True, virar=True,
         animar=True, anim_parado='parado', anim_andar='andar'),
    comp('behavior', script='nn:vida', vidas=2, variavel='', invencivel=0.3, ao_acabar='destruir', cena='',
         piscar=False, barra=True, cor_barra='#FF5A5A', numeros=True),
    comp('behavior', script='nn:golpe', quando='perto', distancia=1.1, alvo='', dano=1, alcance=1.3,
         angulo=100, empurrao=7, intervalo=1.2, animacao='golpe', acerto=0.3, som='',
         som_acerto='assets/sons/acerto.wav'),
])
prefab(BASE + '/visual3d/prefabs/magia.prefab', 'magia', [
    comp('shape3d', shape='sphere', color='#FF7A1A', shadow=False),
    comp('behavior', script='nn:projetil', alvo='', atravessa=False, cor='#FF7A1A', dano=1, dura=3,
         empurrao=4, gravidade=0, parede=True),
], escala=0.35)
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

def golpe(rate=22050, segundos=0.28):
    """Espada cortando o ar: um chiado que sobe e desce de tom, rápido."""
    rnd = random.Random(3)
    n = int(rate * segundos)
    out, lp, hp = [], 0.0, 0.0
    for i in range(n):
        t = i / n
        # O filtro abre e fecha (o "vuuush"); o volume vem e vai.
        k = 0.05 + 0.5 * math.sin(math.pi * t) ** 2
        x = rnd.uniform(-1, 1)
        lp += (x - lp) * k
        hp = lp - hp * 0.0
        env = math.sin(math.pi * min(1.0, t * 1.15)) ** 1.5
        out.append(hp * env * 0.55)
    return out

def acerto(rate=22050, segundos=0.22):
    """Um baque: um tom grave que cai rápido, com um estalo no começo."""
    rnd = random.Random(4)
    n = int(rate * segundos)
    out, fase = [], 0.0
    for i in range(n):
        t = i / rate
        f = 150 * math.exp(-t * 18) + 55
        fase += 2 * math.pi * f / rate
        tom = math.sin(fase) * math.exp(-t * 16)
        estalo = rnd.uniform(-1, 1) * math.exp(-t * 90) * 0.7
        out.append((tom * 0.9 + estalo) * 0.5)
    return out

def rugido(rate=22050, segundos=1.0):
    """O besouro-rei se preparando: um ronco grave, áspero, que cresce."""
    rnd = random.Random(8)
    n = int(rate * segundos)
    out, fase, lp = [], 0.0, 0.0
    for i in range(n):
        t = i / n
        f = 70 + 25 * t + 6 * math.sin(2 * math.pi * 7 * t * segundos)
        fase += 2 * math.pi * f / rate
        serra = (fase / (2 * math.pi)) % 1.0 * 2 - 1
        lp += (serra + rnd.uniform(-0.6, 0.6) - lp) * 0.12
        env = min(1.0, t * 4) * (1 - max(0.0, t - 0.8) / 0.2)
        out.append(lp * env * 0.35)
    return out

comum = os.path.join(BASE, 'comum', 'sons')
wav(os.path.join(comum, 'fogueira.wav'), 16000, fogueira())
wav(os.path.join(comum, 'mar.wav'), 16000, mar())
wav(os.path.join(comum, 'golpe.wav'), 22050, golpe())
wav(os.path.join(comum, 'acerto.wav'), 22050, acerto())
wav(os.path.join(comum, 'rugido.wav'), 22050, rugido())
print('sons ok')
