-- Porta 3D: abre girando na dobradiça, subindo, descendo ou deslizando.
-- Pode pedir uma chave do inventário.
local B = {
  title = "Porta 3D",
  description = "Abre girando, subindo ou deslizando: com a tecla E perto dela, sozinha quando o jogador chega ou por mensagem (\"abrir\"). Pode pedir uma chave.",
  category = "Fases",
  mode = "3d",
  -- No NN Studio, ganha um Corpo 3D que se move sozinho (kinematic): a porta
  -- fechada bloqueia a passagem.
  requires = { "body3d" },
  body = "kinematic",
  props = {
    { "abre", "girando", label = "Abre",
      options = { { "girando", "girando (dobradiça)" }, { "subindo", "subindo" },
        { "descendo", "descendo (some no chão)" }, { "deslizando", "deslizando para o lado" } } },
    { "lado", "esquerda", label = "Lado", show_if = { "abre", "girando", "deslizando" },
      options = { { "esquerda", "esquerda" }, { "direita", "direita" } },
      help = "Girando: de que lado fica a dobradiça. Deslizando: para que lado ela vai (olhando a frente da porta)." },
    { "angulo", 100, label = "Quanto gira", min = 10, max = 180, show_if = { "abre", "girando" },
      help = "Graus. Ela gira para longe de quem abre." },
    { "tempo", 0.6, label = "Segundos para abrir", min = 0.05 },
    { "abrir_com", "tecla", label = "Abre com",
      options = { { "tecla", "a tecla E, perto dela" }, { "chegar", "o jogador chegando perto" },
        { "mensagem", "só mensagem (zona, alavanca, eventos)" } } },
    { "distancia", 1.2, label = "Distância", min = 0, show_if = { "abrir_com", "tecla", "chegar" },
      help = "Metros, da borda da porta até o jogador." },
    { "chave", "", label = "Precisa do item",
      help = "Um item do inventário (\"chave\"), pego com um Coletável. Vazio = não tranca. Mensagens (\"abrir\") abrem mesmo trancada." },
    { "gastar_chave", true, label = "Gasta a chave",
      help = "A chave sai do inventário quando destranca (depois disso a porta fica destrancada)." },
    { "fechar_depois", 0, label = "Fecha sozinha depois de", min = 0,
      help = "Segundos (com o jogador longe dela). 0 = fica aberta." },
    { "texto", "abrir", label = "Aviso", show_if = { "abrir_com", "tecla" }, advanced = true },
    { "texto_fechar", "fechar", label = "Aviso para fechar", show_if = { "abrir_com", "tecla" }, advanced = true,
      help = "Vazio = não fecha com a tecla." },
    { "texto_trancada", "Precisa de: {item}", label = "Aviso trancada", advanced = true },
    { "som", "", type = "sound", label = "Som ao abrir e fechar" },
    { "som_trancada", "", type = "sound", label = "Som trancada", advanced = true },
  },
}

local function suave(t)
  t = math.max(0, math.min(1, t))
  return t * t * (3 - 2 * t)
end

function B:start()
  local o = self.obj
  self.x0, self.y0, self.z0, self.ry0 = o.x, o.y, o.z, o.ry
  local x0, y0, z0, x1, y1, z1 = o:local_bounds3d()
  local sx, sy, sz = math.abs(o.scale_x), math.abs(o.scale_y), math.abs(o.scale_z)
  self.largura, self.altura = (x1 - x0) * sx, (y1 - y0) * sy
  -- A dobradiça: a borda do lado escolhido, no meio da espessura.
  local hx = (self.lado == "direita") and x1 or x0
  local fx = (self.lado == "direita") and x0 or x1
  local hz = (z0 + z1) / 2
  self.dobra = { hx * sx, hz * sz }
  self.ponta = { fx * sx, hz * sz }
  local m = nn.d3.matrix({ ry = self.ry0 })
  local px, _, pz = m:transform_point(self.dobra[1], 0, self.dobra[2])
  self.hx, self.hz = self.x0 + px, self.z0 + pz
  self.f = 0         -- 0 fechada, 1 aberta
  self.alvo = 0
  self.sentido = 1   -- para que lado gira
  self.destrancada = (self.chave == "")
  self.longe = 0
end

--- Está aberta (ou abrindo)?
function B:aberta()
  return self.alvo == 1
end

function B:tocar(som)
  if som == "" then return end
  local x, y, z = self.obj:position3d()
  nn.audio.play(nn.assets.sound(som), { x = x, y = y, z = z, distance = 25 })
end

-- Gira para longe de `quem`: o lado em que a ponta solta vai é o contrário
-- do lado em que ele está.
function B:escolher_sentido(quem)
  if self.abre ~= "girando" or not quem then return end
  local nx, _, nz = nn.d3.matrix({ ry = self.ry0 }):transform_point(0, 0, 1)
  local qx, _, qz = quem:position3d()
  local lado = (qx - self.hx) * nx + (qz - self.hz) * nz
  local m = nn.d3.matrix({ ry = self.ry0 + 30 })
  local ax, _, az = m:transform_point(self.ponta[1] - self.dobra[1], 0, self.ponta[2] - self.dobra[2])
  local vai = ax * nx + az * nz
  self.sentido = ((vai > 0) == (lado > 0)) and -1 or 1
end

--- Abre (`quem` diz para que lado girar). Por mensagem, abre mesmo
--- trancada; a chave vale para quem usa a porta (tecla ou chegando).
function B:abrir(quem)
  if self.alvo == 1 then return end
  if type(quem) ~= "table" then quem = nil end
  self:escolher_sentido(quem)
  self.alvo = 1
  self.longe = 0
  self:tocar(self.som)
  self.obj:send("abriu", quem)
end

-- O jogador usou a porta (tecla ou chegando): a chave conta.
function B:usar(quem)
  if self.alvo == 1 then
    self:fechar()
    return
  end
  if not self.destrancada then
    if not nn.inventory.has(self.chave) then
      if not self.avisou then
        self.avisou = true
        self:tocar(self.som_trancada)
        self.obj:send("trancada", quem)
      end
      return
    end
    if self.gastar_chave then nn.inventory.remove(self.chave) end
    self.destrancada = true
  end
  self:abrir(quem)
end

--- Fecha.
function B:fechar()
  if self.alvo == 0 then return end
  self.alvo = 0
  self:tocar(self.som)
  self.obj:send("fechou")
end

--- Abre se estiver fechada; fecha se estiver aberta.
function B:alternar(quem)
  if self.alvo == 1 then self:fechar() else self:abrir(quem) end
end

--- Destranca sem precisar da chave.
function B:destrancar()
  self.destrancada = true
end

function B:update(dt)
  if self.abrir_com == "tecla" then
    local texto, trancada
    if self.alvo == 1 then
      texto = self.texto_fechar
    elseif not self.destrancada and not nn.inventory.has(self.chave) then
      texto, trancada = (self.texto_trancada:gsub("{item}", self.chave)), true
    else
      texto = self.texto
    end
    if texto ~= "" then
      local quem = self.scene:interaction(self, "", self.distancia, texto, trancada)
      if quem then
        self.avisou = false
        self:usar(quem)
      end
    end
  end
end

-- O jogador mais perto e a distância (da borda da porta).
function B:perto()
  local melhor, dist
  for _, p in ipairs(self.scene:targets("")) do
    local d = self.scene:gap(self.obj, p)
    if not dist or d < dist then melhor, dist = p, d end
  end
  return melhor, dist
end

function B:fixed_update(dt)
  local quem, dist = self:perto()
  local perto = dist and dist <= self.distancia
  if self.abrir_com == "chegar" then
    if perto and self.alvo == 0 then
      self:usar(quem)
    elseif not perto then
      self.avisou = false
    end
  end
  -- Fecha sozinha depois de um tempo aberta, com ninguém perto.
  if self.alvo == 1 and self.f >= 1 and self.fechar_depois > 0 then
    if perto then
      self.longe = 0
    else
      self.longe = self.longe + dt
      if self.longe >= self.fechar_depois then self:fechar() end
    end
  end
  -- Anda até o alvo.
  local passo = dt / math.max(self.tempo, 0.05)
  if self.f < self.alvo then
    self.f = math.min(self.alvo, self.f + passo)
  elseif self.f > self.alvo then
    self.f = math.max(self.alvo, self.f - passo)
  else
    return
  end
  self:posicionar(suave(self.f))
end

function B:posicionar(t)
  local o = self.obj
  if self.abre == "girando" then
    local ry = self.ry0 + self.angulo * t * self.sentido
    local m = nn.d3.matrix({ ry = ry })
    local px, _, pz = m:transform_point(-self.dobra[1], 0, -self.dobra[2])
    o.x, o.z, o.ry = self.hx + px, self.hz + pz, ry
  elseif self.abre == "subindo" then
    o.y = self.y0 + (self.altura - 0.05) * t
  elseif self.abre == "descendo" then
    o.y = self.y0 - (self.altura + 0.02) * t
  else
    local dir = (self.lado == "direita") and 1 or -1
    local m = nn.d3.matrix({ ry = self.ry0 })
    local dx, _, dz = m:transform_point(dir * (self.largura - 0.05) * t, 0, 0)
    o.x, o.z = self.x0 + dx, self.z0 + dz
  end
end

return B
