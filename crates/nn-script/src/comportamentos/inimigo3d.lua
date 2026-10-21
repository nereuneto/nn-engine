-- Inimigo 3D: anda sozinho (patrulha ou vaga), persegue o jogador quando ele
-- chega perto, machuca quem encosta e pode morrer se pisarem em cima.
local B = {
  title = "Inimigo 3D",
  description = "Anda sozinho (indo e voltando ou sem rumo), persegue o jogador quando ele chega perto e machuca quem encosta.",
  category = "Inimigos",
  mode = "3d",
  props = {
    { "andar", "patrulha", label = "Anda",
      options = { { "patrulha", "indo e voltando" }, { "vagar", "sem rumo, por perto" }, { "parado", "fica parado" } } },
    { "x", 6, label = "Até onde em X", show_if = { "andar", "patrulha" },
      help = "Metros a partir de onde ele começa (a volta é para o começo)." },
    { "z", 0, label = "Até onde em Z", show_if = { "andar", "patrulha" } },
    { "raio", 6, label = "Vaga até", min = 1, show_if = { "andar", "vagar" },
      help = "Metros em volta de onde ele começa." },
    { "velocidade", 2, label = "Velocidade", min = 0, help = "Metros por segundo." },
    { "perseguir", true, label = "Persegue o jogador" },
    { "visao", 8, label = "Vê o jogador a", min = 0, show_if = { "perseguir", true },
      help = "Metros. Longe assim e mais um pouco, ele desiste e volta." },
    { "correr", 3.5, label = "Velocidade perseguindo", min = 0, show_if = { "perseguir", true } },
    { "contornar", true, label = "Contorna paredes e buracos",
      help = "Acha um caminho em volta de paredes, árvores, buracos e água, em vez de ir reto e ficar preso." },
    { "dano", 1, label = "Dano", min = 0,
      help = "Vida que tira do jogador ao encostar (comportamento Vida). Com um Golpe no mesmo objeto, só o golpe machuca." },
    { "empurrao", 7, label = "Empurrão", min = 0, advanced = true },
    { "pisar", true, label = "Morre se pisarem em cima",
      help = "Como num jogo de plataforma. Com Vida, perde uma vida a cada pisão." },
    { "virar", true, label = "Virar para onde anda", advanced = true },
    { "animar", true, label = "Animar o modelo", advanced = true,
      help = "Num modelo com animações: parado e andar, conforme o movimento." },
    { "anim_parado", "parado", type = "anim", label = "Animação: parado", advanced = true,
      show_if = { "animar", true } },
    { "anim_andar", "andar", type = "anim", label = "Animação: andar", advanced = true,
      show_if = { "animar", true } },
  },
}

-- O corpo pelo tamanho do modelo (ou da forma), como no Jogador 3D.
local function tamanho(o)
  local x0, y0, z0, x1, y1, z1 = o:local_bounds3d()
  local _, _, _, _, _, _, sx, sy, sz = o:matrix3d():decompose()
  local r = math.max((x1 - x0) * math.abs(sx), (z1 - z0) * math.abs(sz)) / 2
  local h = (y1 - y0) * math.abs(sy)
  r = math.max(0.1, math.min(r, 1.5))
  h = math.max(h, r * 2 + 0.05)
  return r, h, math.min(0, y0 * math.abs(sy))
end

function B:start()
  local o, scene = self.obj, self.scene
  if not scene._3d then
    error(("objeto \"%s\": o Inimigo 3D só funciona numa cena 3D"):format(o.name), 0)
  end
  local x, y, z = o:position3d()
  local r, h, pes = tamanho(o)
  self.corpo = scene:world3():character({ x = x, y = y + pes, z = z, radius = r, height = h, tag = o.name })
  o._char3 = { radius = r, height = h, feet = pes }
  scene:_nn_attach_body3(o, self.corpo)
  self.inicio = { x, z }
  self.ida = true
  self.espera = 0
  self.preso = 0
  self.ultimo = { x, z }
  self.alvo = nil
  self.caca = nil
  -- O caminho em volta do que bloqueia (scene:steer guarda aqui).
  self.rota = {}
  o._nav_state = self.rota
  -- Os inimigos 3D da cena (para um não entrar no outro).
  local todos = {}
  for _, b in ipairs(scene._inimigos3d or {}) do
    if b.obj.alive then todos[#todos + 1] = b end
  end
  todos[#todos + 1] = self
  scene._inimigos3d = todos
end

-- Em grupo: não empurra o inimigo que está colado na frente (escorrega para o
-- lado) e se afasta um pouco dos vizinhos. Assim eles se espalham em volta do
-- jogador em vez de se empilhar, e a física não precisa separar quem está
-- se espremendo (com muitos inimigos, era o que mais pesava).
function B:espalhar(x, z, vx, vz, v)
  local r = self.obj._char3.radius
  local sx, sz = 0, 0
  for _, b in ipairs(self.scene._inimigos3d) do
    local outro = b.obj
    if b ~= self and outro.alive and outro._char3 then
      local dx, dz = b.ultimo[1] - x, b.ultimo[2] - z
      local d2 = dx * dx + dz * dz
      local lim = r + outro._char3.radius + 0.3
      if d2 < lim * lim then
        local d = math.sqrt(d2)
        if d < 1e-3 then
          -- Um em cima do outro: cada um para um lado.
          local a = (outro.id or 0) < (self.obj.id or 0) and 0 or math.pi
          dx, dz, d = math.cos(a), math.sin(a), 1
        else
          dx, dz = dx / d, dz / d
        end
        local frente = vx * dx + vz * dz
        if frente > 0 then vx, vz = vx - frente * dx, vz - frente * dz end
        local f = 1 - d / lim
        sx, sz = sx - dx * f, sz - dz * f
      end
    end
  end
  vx, vz = vx + sx * v, vz + sz * v
  local m = math.sqrt(vx * vx + vz * vz)
  if m > v then vx, vz = vx / m * v, vz / m * v end
  return vx, vz
end

-- O jogador mais perto (e a distância no chão).
function B:jogador()
  local x, _, z = self.obj:position3d()
  local melhor, dist
  for _, p in ipairs(self.scene:targets("")) do
    local px, _, pz = p:position3d()
    local d = math.sqrt((px - x) ^ 2 + (pz - z) ^ 2)
    if not dist or d < dist then melhor, dist = p, d end
  end
  return melhor, dist
end

-- Para onde ir agora (nil = fica parado).
function B:destino(dt)
  local x, _, z = self.obj:position3d()
  if self.perseguir then
    local p, d = self:jogador()
    if p and d <= self.visao then
      self.caca = p
    elseif p and self.caca and d > self.visao * 1.6 then
      self.caca = nil
    end
    if self.caca and self.caca.alive then
      local px, _, pz = self.caca:position3d()
      return px, pz, self.correr
    end
    self.caca = nil
  end
  if self.andar == "patrulha" then
    local ax, az = self.inicio[1], self.inicio[2]
    local bx, bz = ax + self.x, az + self.z
    local tx, tz = ax, az
    if self.ida then tx, tz = bx, bz end
    if math.sqrt((tx - x) ^ 2 + (tz - z) ^ 2) < 0.3 then
      self.ida = not self.ida
      self.espera = 0.6
      return nil
    end
    return tx, tz, self.velocidade
  elseif self.andar == "vagar" then
    if not self.alvo or math.sqrt((self.alvo[1] - x) ^ 2 + (self.alvo[2] - z) ^ 2) < 0.4 then
      if self.alvo then
        self.alvo = nil
        self.espera = 0.5 + math.random() * 1.5
        return nil
      end
      local a = math.random() * math.pi * 2
      local r = self.raio * math.sqrt(math.random())
      self.alvo = { self.inicio[1] + math.cos(a) * r, self.inicio[2] + math.sin(a) * r }
    end
    return self.alvo[1], self.alvo[2], self.velocidade
  end
  return nil
end

function B:fixed_update(dt)
  local o, c = self.obj, self.corpo
  if not (c and c.alive) then return end
  -- Levou um empurrão (um golpe, um tiro): a física leva, ele não anda.
  if (o.empurrado or 0) > 0 then
    o.empurrado = o.empurrado - dt
    self.ultimo = { o.x, o.z }
    return
  end
  local vx, vz = 0, 0
  -- Golpeando (comportamento Golpe): parado, e a animação é a do golpe.
  local golpe = o._golpeando
  if golpe then
    if golpe.parar then c:walk(0, 0) end
    self:tocar_em_quem_encosta()
    return
  end
  if self.espera > 0 then
    self.espera = self.espera - dt
  else
    local tx, tz, v = self:destino(dt)
    if tx then
      local x, _, z = o:position3d()
      if self.contornar then
        tx, tz = self.scene:steer(self.rota, x, z, tx, tz, dt)
      else
        self.rota.path = nil
      end
      local dx, dz = tx - x, tz - z
      local n = math.sqrt(dx * dx + dz * dz)
      if n > 0.05 then vx, vz = dx / n * v, dz / n * v end
      if vx ~= 0 or vz ~= 0 then vx, vz = self:espalhar(x, z, vx, vz, v) end
    end
  end
  c:walk(vx, vz)
  -- Preso numa parede (ou numa árvore): troca de rumo.
  local x, _, z = o:position3d()
  local andou = math.sqrt((x - self.ultimo[1]) ^ 2 + (z - self.ultimo[2]) ^ 2)
  self.ultimo = { x, z }
  if (vx ~= 0 or vz ~= 0) and andou < 0.2 * dt * math.sqrt(vx * vx + vz * vz) then
    self.preso = self.preso + dt
    if self.preso > 0.8 then
      self.preso = 0
      self.ida = not self.ida
      self.alvo = nil
    end
  else
    self.preso = 0
  end
  if self.virar and (vx ~= 0 or vz ~= 0) then
    local alvo = math.deg(math.atan(vx, vz))
    local d = (alvo - (o.ry or 0) + 180) % 360 - 180
    o.ry = (o.ry or 0) + d * math.min(1, dt * 8)
  end
  self:animar_modelo(vx, vz)
  self:tocar_em_quem_encosta()
end

-- Encostou no jogador.
function B:tocar_em_quem_encosta()
  local o = self.obj
  self._vy = self._vy or setmetatable({}, { __mode = "k" })
  for _, p in ipairs(self.scene:targets("")) do
    if o.alive and o:touching(p, 0.05) then self:encostou(p) end
    self._vy[p] = p.body and p.body.alive and p.body.vy or 0
  end
end

function B:animar_modelo(vx, vz)
  local a = self.obj:animator()
  if not (self.animar and a) then return end
  local nome = (vx * vx + vz * vz) > 0.01 and self.anim_andar or self.anim_parado
  if a:has(nome) then a:play(nome, { blend = 0.2 }) end
end

-- O jogador caiu em cima? (A física pode ter parado a queda neste passo:
-- vale a velocidade do passo anterior.)
function B:pisou(p)
  local b = p.body
  if not b then return false end
  local caindo = b.vy < -0.5 or ((self._vy and self._vy[p]) or 0) < -0.5
  if not caindo then return false end
  local _, _, _, _, y1 = self.obj:bounds3d()
  local _, py = p:position3d()
  return py >= y1 - 0.35
end

function B:encostou(p)
  local o = self.obj
  if self.pisar and self:pisou(p) then
    p.body.vy = 8
    if o:send("dano", 1, p) == nil then
      local x, y, z = o:position3d()
      self.scene:burst(x, y + 0.3, "#FFFFFF", 18, z)
      p:send("derrotou", o)
      o:send("derrotado", p)
      o:destroy()
    end
    return
  end
  -- Com um Golpe, quem machuca é o golpe.
  local g = o:behavior("golpe")
  if self.dano <= 0 or (g and g.enabled) then return end
  local levou = p:send("dano", self.dano, o)
  if levou == nil then
    -- O jogador não tem Vida: volta ao checkpoint (ou reinicia a cena).
    self.scene:restart({ checkpoint = true })
    return
  end
  if levou and p.body and self.empurrao > 0 then
    local x, _, z = o:position3d()
    local px, _, pz = p:position3d()
    local dx, dz = px - x, pz - z
    local n = math.sqrt(dx * dx + dz * dz)
    if n < 1e-3 then dx, dz, n = 0, 1, 1 end
    p.body:set_velocity(dx / n * self.empurrao, self.empurrao * 0.6, dz / n * self.empurrao)
    p.empurrado = 0.35
    self.espera = 0.6
  end
end

return B
