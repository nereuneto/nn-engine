-- Chefe: um inimigo grande que luta com vários ataques (golpe, investida,
-- pulo com impacto, tiros em leque, chamar ajudantes), sempre com um aviso
-- antes, fica furioso com pouca vida e mostra a barra no alto da tela.
local B = {
  title = "Chefe",
  description = "Um inimigo grande com vários ataques, cada um com um aviso antes (dá para desviar): golpe, investida, pulo com impacto, tiros em leque e chamar ajudantes. Com pouca vida fica furioso. Mostra o nome e a vida no alto da tela. Precisa de Vida.",
  category = "Inimigos",
  mode = "3d",
  props = {
    { "nome", "Chefe", label = "Nome (na barra)" },
    { "quando", "chegar", label = "A luta começa",
      options = {
        { "chegar", "quando o jogador chega perto" },
        { "cena", "quando a cena começa" },
        { "mensagem", "com a mensagem \"comecar\"" },
      } },
    { "distancia", 10, label = "Perto (metros)", min = 0, show_if = { "quando", "chegar" } },
    { "velocidade", 2.5, label = "Velocidade", min = 0, help = "Metros por segundo, andando atrás do jogador." },
    { "pausa", 1.4, label = "Pausa entre ataques", min = 0, help = "Segundos." },
    { "aviso", 0.7, label = "Aviso antes de atacar", min = 0,
      help = "Segundos em que ele se prepara (pisca e toca \"preparar\"): o tempo de desviar." },
    { "dano", 1, label = "Dano", min = 0, help = "De cada ataque e ao encostar." },
    { "golpe", true, label = "Ataque: golpe", help = "Bate na frente, de perto." },
    { "alcance", 2.4, label = "Alcance do golpe", min = 0.5, show_if = { "golpe", true },
      help = "Metros do meio dele até quem leva (num chefe largo, aumente)." },
    { "investida", true, label = "Ataque: investida",
      help = "Corre reto para onde o jogador estava. Se bater numa parede, fica tonto." },
    { "velocidade_investida", 11, label = "Velocidade da investida", min = 1, advanced = true,
      show_if = { "investida", true } },
    { "tonto", 1.6, label = "Tonto depois de bater", min = 0, advanced = true, show_if = { "investida", true },
      help = "Segundos parado depois de bater numa parede: a hora de acertar ele." },
    { "pulo", true, label = "Ataque: pulo com impacto",
      help = "Pula, cai perto do jogador e solta uma onda no chão em volta (pule para escapar)." },
    { "raio_impacto", 4, label = "Raio do impacto", min = 0.5, show_if = { "pulo", true } },
    { "leque", false, label = "Ataque: tiros em leque" },
    { "tiro", "", type = "prefab", label = "Tiro (prefab)", show_if = { "leque", true },
      help = "Um prefab com Projétil (como a magia do Vale 3D)." },
    { "tiros", 5, label = "Tiros no leque", min = 1, max = 15, integer = true, show_if = { "leque", true } },
    { "abertura", 70, label = "Abertura do leque", min = 0, max = 360, advanced = true, show_if = { "leque", true },
      help = "Graus." },
    { "ajudantes", false, label = "Ataque: chamar ajudantes" },
    { "ajudante", "", type = "prefab", label = "Ajudante (prefab)", show_if = { "ajudantes", true } },
    { "quantos", 2, label = "Ajudantes de cada vez", min = 1, integer = true, show_if = { "ajudantes", true } },
    { "furia", 50, label = "Furioso com a vida em", min = 0, max = 100,
      help = "Porcentagem: com essa vida ou menos, fica mais rápido e ataca com menos pausa (0 = nunca)." },
    { "barra", true, label = "Barra no alto da tela" },
    { "cor_barra", "#FF5A5A", type = "color", label = "Cor da barra", advanced = true, show_if = { "barra", true } },
    { "solta", "", type = "prefab", label = "Ao ser derrotado, deixa",
      help = "Um prefab que aparece onde ele caiu (uma estrela, uma chave). Vazio = nada." },
    { "som_aviso", "", type = "sound", label = "Som do aviso", advanced = true },
    { "som_impacto", "", type = "sound", label = "Som do impacto", advanced = true },
    { "anim_parado", "parado", type = "anim", label = "Animação: parado", advanced = true },
    { "anim_andar", "andar", type = "anim", label = "Animação: andar", advanced = true },
    { "anim_golpe", "golpe", type = "anim", label = "Animação: golpe", advanced = true },
    { "anim_aviso", "preparar", type = "anim", label = "Animação: aviso", advanced = true },
    { "anim_pulo", "pular", type = "anim", label = "Animação: pulo", advanced = true },
    { "anim_tonto", "tonto", type = "anim", label = "Animação: tonto", advanced = true },
    { "anim_cair", "cair", type = "anim", label = "Animação: derrotado", advanced = true },
  },
}

-- O corpo pelo tamanho do modelo (como no Inimigo 3D), um pouco mais largo.
local function tamanho(o)
  local x0, y0, z0, x1, y1, z1 = o:local_bounds3d()
  local _, _, _, _, _, _, sx, sy, sz = o:matrix3d():decompose()
  local r = math.max((x1 - x0) * math.abs(sx), (z1 - z0) * math.abs(sz)) / 2
  local h = (y1 - y0) * math.abs(sy)
  r = math.max(0.2, math.min(r, 3))
  h = math.max(h, r * 2 + 0.05)
  return r, h, math.min(0, y0 * math.abs(sy))
end

function B:start()
  local o, scene = self.obj, self.scene
  if not scene._3d then
    error(("objeto \"%s\": o Chefe só funciona numa cena 3D"):format(o.name), 0)
  end
  local x, y, z = o:position3d()
  local r, h, pes = tamanho(o)
  self.corpo = scene:world3():character({ x = x, y = y + pes, z = z, radius = r, height = h, tag = o.name })
  o._char3 = { radius = r, height = h, feet = pes }
  scene:_nn_attach_body3(o, self.corpo)
  self.raio = r
  self.estado = "dormindo"
  self.t = 0
  self.espera = 0
  self.furioso = false
  self.mostrada = 1
  self.ajudantes_vivos = {}
  self.rota = {}
  o._nav_state = self.rota
  self.vida = o:behavior("vida")
  if not self.vida then
    scene:warn(("objeto \"%s\": o Chefe precisa do comportamento Vida (senão ninguém derrota ele)"):format(o.name))
  end
  if self.quando == "cena" then self:comecar() end
end

--- Começa a luta (a mensagem "comecar").
function B:comecar()
  if self.estado ~= "dormindo" then return end
  self.estado = "andando"
  self.espera = self.pausa
  self.obj:send("luta_comecou")
end

-- Multiplica as velocidades e divide as pausas na fúria.
function B:ritmo()
  return self.furioso and 1.4 or 1
end

function B:tocar(nome, loop, reiniciar)
  local a = self.obj:animator()
  if a and nome ~= "" and a:has(nome) then
    a:play(nome, { loop = loop ~= false, blend = 0.12, restart = reiniciar })
    return a
  end
end

-- O jogador mais perto: ele, a distância e a direção no chão.
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

function B:olhar_para(px, pz, dt)
  local x, _, z = self.obj:position3d()
  local alvo = math.deg(math.atan(px - x, pz - z))
  local o = self.obj
  if not dt then
    o.ry = alvo
    return
  end
  local d = (alvo - (o.ry or 0) + 540) % 360 - 180
  o.ry = (o.ry or 0) + d * math.min(1, dt * 6 * self:ritmo())
end

-- Escolhe o próximo ataque entre os ligados que fazem sentido agora.
function B:escolher(dist)
  local op = {}
  local function pode(nome, peso) for _ = 1, peso do op[#op + 1] = nome end end
  -- O golpe só quando o jogador já está ao alcance (senão, ele chega mais perto).
  if self.golpe and dist <= self.alcance then pode("golpe", 3) end
  if self.investida and dist > 4 then pode("investida", 2) end
  if self.pulo and dist > 2 then pode("pulo", 2) end
  if self.leque and self.tiro ~= "" and dist > 3 then pode("leque", 2) end
  if self.ajudantes and self.ajudante ~= "" then
    local vivos = 0
    for _, a in ipairs(self.ajudantes_vivos) do if a.alive then vivos = vivos + 1 end end
    if vivos == 0 then pode("ajudantes", 1) end
  end
  if #op == 0 then return nil end
  return op[math.random(#op)]
end

function B:avisar(ataque)
  self.estado = "aviso"
  self.proximo = ataque
  self.t = self.aviso / self:ritmo()
  self.obj._tint = "#FFFFFF"
  self:tocar(self.anim_aviso, true, true)
  if self.som_aviso ~= "" then
    local x, y, z = self.obj:position3d()
    nn.audio.play(nn.assets.sound(self.som_aviso), { x = x, y = y + 1, z = z, distance = 30 })
  end
  self.obj:send("aviso", ataque)
end

function B:atacar(ataque)
  local o, c = self.obj, self.corpo
  local p = self:jogador()
  self.estado = ataque
  self.t = 0
  self.feito = false
  if ataque == "golpe" then
    local a = self:tocar(self.anim_golpe, false, true)
    self.duracao = a and math.max(a.duration, 0.4) or 0.6
  elseif ataque == "investida" then
    local x, _, z = o:position3d()
    local px, pz = x, z + 1
    if p then px, _, pz = p:position3d() end
    local dx, dz = px - x, pz - z
    local n = math.max(1e-3, math.sqrt(dx * dx + dz * dz))
    self.dir = { dx / n, dz / n }
    self:olhar_para(px, pz)
    self:tocar(self.anim_andar, true)
    local a = o:animator()
    if a then a.speed = 2 end
    self.duracao = math.min(2.5, (n + 3) / self.velocidade_investida)
    self.ultimo = { x, z }
    self.acertou_investida = false
  elseif ataque == "pulo" then
    local x, _, z = o:position3d()
    local px, pz = x, z
    if p then
      -- Cai na frente do jogador (não em cima dele).
      px, _, pz = p:position3d()
      local dx, dz = px - x, pz - z
      local n = math.sqrt(dx * dx + dz * dz)
      local antes = math.min(n, self.raio + 0.6)
      if n > 1e-3 then px, pz = px - dx / n * antes, pz - dz / n * antes end
    end
    local vy = 9
    local tempo = 2 * vy / 20
    self.salto = { (px - x) / tempo, (pz - z) / tempo }
    self:olhar_para(px, pz)
    c:jump(vy)
    c.vy = vy
    self:tocar(self.anim_pulo, false, true)
    self.no_ar = false
  elseif ataque == "leque" then
    self:tocar(self.anim_golpe, false, true)
    self.duracao = 0.6
  elseif ataque == "ajudantes" then
    self:tocar(self.anim_aviso, false, true)
    self.duracao = 0.6
  end
  o:send("atacou", ataque)
end

function B:fixed_update(dt)
  local o, c = self.obj, self.corpo
  if not (c and c.alive) or self.estado == "derrotado" then return end
  -- Pesado: um golpe não empurra o chefe.
  if (o.empurrado or 0) > 0 then
    o.empurrado = 0
    c:set_velocity(0, math.min(c.vy, 0), 0)
  end
  self:ver_furia()
  local p, dist = self:jogador()
  local estado = self.estado
  local vx, vz = 0, 0
  if estado == "dormindo" then
    self:tocar(self.anim_parado, true)
    if self.quando == "chegar" and p and dist <= self.distancia then self:comecar() end
  elseif estado == "andando" then
    self.espera = self.espera - dt
    if p then
      local px, _, pz = p:position3d()
      local x, _, z = o:position3d()
      -- Para um pouco antes de encostar (ou já ao alcance do golpe).
      local perto = self.raio + 1.2
      if self.golpe then perto = math.max(self.raio + 0.6, math.min(self.alcance * 0.8, perto)) end
      if dist > perto then
        local tx, tz = self.scene:steer(self.rota, x, z, px, pz, dt)
        local dx, dz = tx - x, tz - z
        local n = math.sqrt(dx * dx + dz * dz)
        local v = self.velocidade * self:ritmo()
        if n > 0.05 then vx, vz = dx / n * v, dz / n * v end
      end
      self:olhar_para(px, pz, dt)
      if self.espera <= 0 then
        local ataque = self:escolher(dist)
        if ataque then
          self:avisar(ataque)
          vx, vz = 0, 0
        else
          self.espera = 0.3
        end
      end
    end
    if self.estado == "andando" then
      self:tocar((vx ~= 0 or vz ~= 0) and self.anim_andar or self.anim_parado, true)
    end
  elseif estado == "aviso" then
    self.t = self.t - dt
    if p then
      local px, _, pz = p:position3d()
      self:olhar_para(px, pz, dt)
    end
    -- Pisca em branco enquanto se prepara.
    o._tint = (math.floor(self.t * 12) % 2 == 0) and "#FFFFFF" or "#FFE0A0"
    if self.t <= 0 then
      o._tint = nil
      self:atacar(self.proximo)
    end
  elseif estado == "golpe" then
    self.t = self.t + dt
    if not self.feito and self.t >= 0.3 then
      self.feito = true
      local fx, fz = self.scene:front3(o)
      for _, a in ipairs(self.scene:in_front(o, self.alcance, 140, "", true)) do
        self.scene:hit(a, self.dano, o, { push = 9, dx = fx, dz = fz, up = 0.5 })
      end
      self.scene:shake(5, 0.2)
    end
    if self.t >= self.duracao then self:depois() end
  elseif estado == "investida" then
    self.t = self.t + dt
    local v = self.velocidade_investida * self:ritmo()
    vx, vz = self.dir[1] * v, self.dir[2] * v
    -- Atropela o jogador (uma vez).
    if p and not self.acertou_investida and o:touching(p, 0.15) then
      self.acertou_investida = true
      self.scene:hit(p, self.dano, o, { push = 11, dx = self.dir[1], dz = self.dir[2], up = 0.6 })
    end
    -- Bateu numa parede: parou de andar.
    local x, _, z = o:position3d()
    local andou = math.sqrt((x - self.ultimo[1]) ^ 2 + (z - self.ultimo[2]) ^ 2)
    self.ultimo = { x, z }
    if self.t > 0.25 and andou < v * dt * 0.25 then
      vx, vz = 0, 0
      local a = o:animator()
      if a then a.speed = 1 end
      self.scene:shake(8, 0.3)
      self.scene:burst(x + self.dir[1] * self.raio, select(2, o:position3d()) + 0.8,
        "#D9C3A5", 20, z + self.dir[2] * self.raio)
      self.estado = "tonto"
      self.t = self.tonto
      self:tocar(self.anim_tonto, true, true)
      o:send("tonto")
    elseif self.t >= self.duracao then
      local a = o:animator()
      if a then a.speed = 1 end
      self:depois()
    end
  elseif estado == "tonto" then
    self.t = self.t - dt
    if self.t <= 0 then self:depois() end
  elseif estado == "pulo" then
    self.t = self.t + dt
    vx, vz = self.salto[1], self.salto[2]
    if not c.on_ground then self.no_ar = true end
    if self.no_ar and c.on_ground and self.t > 0.15 then
      vx, vz = 0, 0
      self:impacto()
      self:depois()
    elseif self.t > 2 then
      self:depois()
    end
  elseif estado == "leque" then
    self.t = self.t + dt
    if not self.feito and self.t >= 0.3 then
      self.feito = true
      self:atirar_leque()
    end
    if self.t >= self.duracao then self:depois() end
  elseif estado == "ajudantes" then
    self.t = self.t + dt
    if not self.feito and self.t >= 0.3 then
      self.feito = true
      self:chamar_ajudantes()
    end
    if self.t >= self.duracao then self:depois() end
  end
  c:walk(vx, vz)
  -- Encostar machuca (menos dormindo e tonto).
  if p and estado ~= "dormindo" and estado ~= "tonto" and estado ~= "investida" and o:touching(p, 0.05) then
    if self.dano > 0 then self.scene:hit(p, self.dano, o, { push = 8, up = 0.6 }) end
  end
end

-- Volta a andar, esperando a pausa.
function B:depois()
  self.estado = "andando"
  self.espera = self.pausa / self:ritmo()
  self.obj._tint = nil
end

-- Caiu do pulo: uma onda no chão em volta; quem está no chão perto leva.
function B:impacto()
  local o = self.obj
  local x, y, z = o:position3d()
  local r = self.raio_impacto
  for i = 0, 11 do
    local a = i / 12 * math.pi * 2
    self.scene:burst(x + math.cos(a) * r * 0.8, y + 0.2, "#D9C3A5", 4, z + math.sin(a) * r * 0.8)
  end
  self.scene:shake(12, 0.35)
  if self.som_impacto ~= "" then
    nn.audio.play(nn.assets.sound(self.som_impacto), { x = x, y = y, z = z, distance = 40 })
  end
  for _, p in ipairs(self.scene:foes(o, "")) do
    local px, py, pz = p:position3d()
    local d = math.sqrt((px - x) ^ 2 + (pz - z) ^ 2)
    local no_chao = not p.body or p.body.on_ground or py < y + 0.4
    if d <= r + self.raio and no_chao then
      self.scene:hit(p, self.dano, o, { push = 9, up = 0.7 })
    end
  end
  o:send("impactou")
end

function B:atirar_leque()
  local o = self.obj
  local x, _, z = o:position3d()
  local _, y0, _, _, y1 = o:bounds3d()
  local fx, fz = self.scene:front3(o)
  local base = math.atan(fx, fz)
  local n = self.tiros
  local ab = math.rad(self.abertura)
  for i = 1, n do
    local a = base + (n == 1 and 0 or (i - 1) / (n - 1) - 0.5) * ab
    local dx, dz = math.sin(a), math.cos(a)
    -- Na altura do peito de quem está no chão (um metro), não da cabeça dele.
    local sx, sy, sz = x + dx * (self.raio + 0.4), y0 + math.min(1.0, (y1 - y0) * 0.5), z + dz * (self.raio + 0.4)
    local t = self.scene:spawn(self.tiro, sx, sy, { z = sz })
    t._dono = o
    t.ry = math.deg(a)
    local v = 9
    if t.body and t.body.alive then
      t.body.gravity_scale = 0
      t.body:set_velocity(dx * v, 0, dz * v)
    else
      t.vx, t.vy, t.vz = dx * v, 0, dz * v
    end
  end
end

function B:chamar_ajudantes()
  local x, y, z = self.obj:position3d()
  for _ = 1, self.quantos do
    local ax, ay, az = self.scene:ground_spot(x, y, z, 4, 2)
    local a = self.scene:spawn(self.ajudante, ax, ay, { z = az })
    self.scene:burst(ax, ay + 0.4, "#B36BFF", 14, az)
    table.insert(self.ajudantes_vivos, a)
  end
end

function B:ver_furia()
  if self.furioso or self.furia <= 0 or not self.vida then return end
  local v = self.vida
  if v.atual and v.vidas and v.atual / v.vidas * 100 <= self.furia and v.atual > 0 then
    self.furioso = true
    local x, y, z = self.obj:position3d()
    self.scene:burst(x, y + 1, "#FF4A4A", 30, z)
    self.scene:shake(8, 0.4)
    self.obj:send("ficou_furioso")
  end
end

function B:update(dt)
  -- A barra acompanha a vida devagar (a parte que sumiu fica branca um pouco).
  local v = self.vida
  if v and v.vidas and v.vidas > 0 then
    local alvo = math.max(0, v.atual / v.vidas)
    if self.mostrada > alvo then
      self.mostrada = math.max(alvo, self.mostrada - dt * 0.6)
    else
      self.mostrada = alvo
    end
  end
  -- Furioso: um brilho vermelho de vez em quando.
  if self.furioso and self.estado ~= "aviso" and self.estado ~= "derrotado" and (self.obj._tint == nil
      or self.obj._tint == "#FF9A9A") then
    self.obj._tint = (math.floor(nn.time.elapsed() * 3) % 2 == 0) and "#FF9A9A" or nil
  end
  if self.estado == "derrotado" then
    self.t = self.t - dt
    if self.t <= 0 and self.obj.alive then self.obj:destroy() end
  end
end

-- A Vida acabou (com "Quando acabar: nada"): cai, solta faíscas e some.
function B:vida_acabou()
  if self.estado == "derrotado" then return end
  self.estado = "derrotado"
  self.t = 1.6
  self.obj._tint = nil
  if self.corpo and self.corpo.alive then self.corpo:walk(0, 0) end
  local x, y, z = self.obj:position3d()
  self.scene:burst(x, y + 1, "#FFD166", 40, z)
  self.scene:shake(14, 0.5)
  if not self:tocar(self.anim_cair, false, true) then self.t = 0.4 end
end

-- Some (derrotado ou destruído): solta o prêmio uma vez.
function B:destroyed()
  if self.soltou then return end
  self.soltou = true
  local x, y, z = self.obj:position3d()
  self.scene:burst(x, y + 0.8, "#FFD166", 30, z)
  for _, a in ipairs(self.ajudantes_vivos or {}) do
    if a.alive then a:destroy() end
  end
  local caiu = self.estado == "derrotado" or (self.vida and (self.vida.atual or 1) <= 0)
  if self.solta ~= "" and caiu then
    local gx, gy, gz = self.scene:ground_spot(x, y, z, 0, 0)
    self.scene:spawn(self.solta, gx, gy, { z = gz })
  end
end

-- A barra do chefe, no alto da tela.
function B:draw()
  if not self.barra or not self.vida or self.estado == "dormindo" then return end
  if not self.obj.alive then return end
  local W = nn.window.size()
  local w = math.min(640, W * 0.6)
  local h = 18
  local x, y = (W - w) / 2, 64
  nn.screen.text(self.nome, W / 2, y - 22, { size = 26, color = "#FFFFFF", anchor = "center" })
  nn.screen.panel(x - 4, y - 4, w + 8, h + 8, { color = "#000000B0", radius = 6 })
  local v = self.vida
  local frac = v.vidas > 0 and math.max(0, v.atual / v.vidas) or 0
  if self.mostrada > frac then
    nn.screen.panel(x, y, w * self.mostrada, h, { color = "#FFFFFFCC", radius = 4 })
  end
  if frac > 0 then
    nn.screen.panel(x, y, w * frac, h, { color = self.furioso and "#FF2E2E" or self.cor_barra, radius = 4 })
  end
end

return B
