"""Ajudas para gerar cenas 3D do NN Studio (as ferramentas gerar_*.py)."""
import os

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

