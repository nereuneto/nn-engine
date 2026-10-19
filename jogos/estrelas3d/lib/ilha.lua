-- A fase: a ilha, as plataformas (paradas e que andam), as moedas e as
-- estrelas. Cria a física e sabe se desenhar. O título usa a mesma ilha
-- como fundo.
local ilha = {}
ilha.__index = ilha

-- Plataformas paradas: centro do topo (x, y, z), largura e profundidade.
local PLATAFORMAS = {
  { -4, 1, 2, 3, 3 }, { -8, 2, -2, 3, 3 }, { -6, 3.2, -7, 3, 3 },  -- escada da esquerda
  { 10, 4.5, -10, 3, 3 },                                           -- depois da que anda
  { 11, 5.8, -5.5, 2.5, 2.5 }, { 8, 7.2, -2, 2.5, 2.5 }, { 10.5, 8.5, 1.5, 2.5, 2.5 }, -- torre
  { 0, 8, 0, 4, 4 },                                                -- ilha no céu
  { -15.5, 1, 6, 2.5, 2.5 }, { -18.5, 1.8, 9.5, 2.5, 2.5 }, { -20, 3, 23, 3, 3 }, -- no mar
}

-- Plataformas que andam: de (a) até (b), velocidade, largura e profundidade.
local MOVEIS = {
  { a = { 0, 3.5, -10 }, b = { 6, 3.5, -10 }, v = 1.6, w = 3, d = 3 },
  { a = { 3.2, 0.3, 0 }, b = { 3.2, 7.9, 0 }, v = 1.3, w = 2, d = 2 },     -- elevador
  { a = { -20, 2.2, 13 }, b = { -20, 2.2, 19 }, v = 1.4, w = 2.5, d = 2.5 },
}

-- As estrelas (em cima das plataformas mais difíceis).
local ESTRELAS = {
  { -6, 4, -7 }, { 10, 5.3, -10 }, { 10.5, 9.3, 1.5 }, { 0, 8.8, 0 }, { -20, 3.8, 23 },
}

-- Moedas: no chão em arco e em cima das plataformas.
local function moedas()
  local lista = {}
  for i = 0, 7 do
    local a = math.rad(200 + i * 18)
    table.insert(lista, { math.cos(a) * 9, 0.6, math.sin(a) * 9 })
  end
  for _, p in ipairs({ PLATAFORMAS[1], PLATAFORMAS[2], PLATAFORMAS[5], PLATAFORMAS[6],
    PLATAFORMAS[9], PLATAFORMAS[10] }) do
    table.insert(lista, { p[1], p[2] + 0.6, p[3] })
  end
  for i = 0, 3 do
    table.insert(lista, { 6 + i * 1.5, 0.6, 9 - i * 1.2 })
  end
  return lista
end

-- Números "aleatórios" sempre iguais, para a ilha não mudar a cada vez.
local semente = 3
local function sorteio(a, b)
  semente = (semente * 1103515245 + 12345) % 2147483648
  return a + (b - a) * (semente / 2147483648)
end

function ilha.nova()
  semente = 3
  local self = setmetatable({}, ilha)
  self.modelos = {
    arvore = nn.assets.model("modelos/arvore.glb"),
    pedra = nn.assets.model("modelos/pedra.glb"),
    plataforma = nn.assets.model("modelos/plataforma.glb"),
    estrela = nn.assets.model("modelos/estrela.glb"),
    moeda = nn.assets.model("modelos/moeda.glb"),
  }
  self.formas = { chao = nn.d3.cylinder(14), agua = nn.d3.plane(), areia = nn.d3.cylinder(14) }

  self.mundo = nn.physics3d.world({ gravity = -20 })
  -- A ilha: um cilindro baixo com o topo em y = 0.
  self.mundo:body({ type = "static", shape = "cylinder", radius = 14, height = 4, y = -2, tag = "ilha" })

  self.plataformas = {}
  for _, p in ipairs(PLATAFORMAS) do
    local x, y, z, w, d = table.unpack(p)
    self.mundo:body({ type = "static", w = w, h = 1, d = d, x = x, y = y - 0.5, z = z })
    table.insert(self.plataformas, { x = x, y = y, z = z, w = w, d = d })
  end

  self.moveis = {}
  for _, m in ipairs(MOVEIS) do
    -- O corpo fica no topo da plataforma; a caixa vai meio metro para baixo.
    local corpo = self.mundo:body({ type = "kinematic", w = m.w, h = 0.5, d = m.d,
      offset_y = -0.25, x = m.a[1], y = m.a[2], z = m.a[3] })
    table.insert(self.moveis, { corpo = corpo, a = m.a, b = m.b, v = m.v, w = m.w, d = m.d, indo = true })
  end

  self.enfeites = {}
  for _ = 1, 26 do
    local a, r = sorteio(0, math.pi * 2), sorteio(6, 13)
    local x, z = math.cos(a) * r, math.sin(a) * r
    local longe_da_trilha = (x * x + (z - 9) ^ 2) > 9 and (x - 3.2) ^ 2 + z * z > 6
    if longe_da_trilha then
      local arvore = sorteio(0, 1) < 0.7
      local s = arvore and sorteio(0.9, 1.5) or sorteio(0.6, 1.4)
      local e = { arvore = arvore, x = x, z = z, s = s, giro = sorteio(0, 360) }
      if arvore then
        self.mundo:body({ type = "static", shape = "cylinder", radius = 0.22 * s, height = 1.6 * s,
          x = x, y = 0.8 * s, z = z })
      else
        self.mundo:body({ type = "static", model = self.modelos.pedra, scale = s * 1.6, x = x, z = z, ry = e.giro })
      end
      table.insert(self.enfeites, e)
    end
  end

  -- Moedas e estrelas: sensores (não bloqueiam, só avisam quem encosta).
  self.itens = {}
  for _, p in ipairs(moedas()) do
    self:item("moeda", p, 0.5)
  end
  for _, p in ipairs(ESTRELAS) do
    self:item("estrela", p, 0.7)
  end
  self.tempo = 0
  return self
end

function ilha:item(tipo, p, raio)
  local corpo = self.mundo:body({ type = "static", sensor = true, shape = "sphere", radius = raio,
    x = p[1], y = p[2], z = p[3], tag = tipo })
  self.itens[corpo.id] = { tipo = tipo, corpo = corpo, x = p[1], y = p[2], z = p[3] }
end

-- Quantos itens de um tipo ainda estão na ilha.
function ilha:restam(tipo)
  local n = 0
  for _, it in pairs(self.itens) do
    if it.tipo == tipo then n = n + 1 end
  end
  return n
end

-- Tira o item que tem este corpo (pegou). Devolve o item ou nil.
function ilha:pegar(corpo)
  local it = self.itens[corpo.id]
  if it then
    self.itens[corpo.id] = nil
    corpo:remove()
  end
  return it
end

-- As plataformas que andam vão e voltam.
function ilha:animar(dt)
  self.tempo = self.tempo + dt
  for _, m in ipairs(self.moveis) do
    local alvo = m.indo and m.b or m.a
    local c = m.corpo
    local dx, dy, dz = alvo[1] - c.x, alvo[2] - c.y, alvo[3] - c.z
    local dist = math.sqrt(dx * dx + dy * dy + dz * dz)
    if dist < 0.05 then
      m.indo = not m.indo
      c:set_velocity(0, 0, 0)
    else
      c:set_velocity(dx / dist * m.v, dy / dist * m.v, dz / dist * m.v)
    end
  end
end

function ilha:passo(dt)
  self:animar(dt)
  return self.mundo:step(dt)
end

function ilha:desenhar()
  local t = self.tempo
  local m = self.modelos
  -- O mar (sem sombra, meio transparente) e a ilha com uma faixa de areia.
  nn.d3.draw(self.formas.agua, { y = -0.4, scale = 400, color = "#3FA7D6", shadow = false })
  nn.d3.draw(self.formas.areia, { y = -0.55, sx = 29.5, sz = 29.5, sy = 1, color = "#F2D49B", shadow = false })
  nn.d3.draw(self.formas.chao, { y = -2, sx = 28, sz = 28, sy = 4, color = "#7BC86C" })

  for _, e in ipairs(self.enfeites) do
    if e.arvore then
      nn.d3.draw(m.arvore, { x = e.x, z = e.z, ry = e.giro, scale = e.s })
    else
      nn.d3.draw(m.pedra, { x = e.x, z = e.z, ry = e.giro, scale = e.s * 1.6 })
    end
  end
  for _, p in ipairs(self.plataformas) do
    nn.d3.draw(m.plataforma, { x = p.x, y = p.y, z = p.z, sx = p.w, sz = p.d })
  end
  for _, mv in ipairs(self.moveis) do
    nn.d3.draw(m.plataforma, { body = mv.corpo, sx = mv.w, sz = mv.d, sy = 0.5, color = "#FFE6B0" })
  end
  for _, it in pairs(self.itens) do
    local giro = t * (it.tipo == "estrela" and 90 or 180)
    local sobe = math.sin(t * 2 + it.x) * 0.12
    if it.tipo == "estrela" then
      nn.d3.draw(m.estrela, { x = it.x, y = it.y - 0.5 + sobe, z = it.z, ry = giro, scale = 1.4 })
    else
      nn.d3.draw(m.moeda, { x = it.x, y = it.y - 0.4 + sobe, z = it.z, ry = giro, shadow = false })
    end
  end
end

return ilha
