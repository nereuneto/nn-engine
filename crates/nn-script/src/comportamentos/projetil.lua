-- Projétil: o tiro anda reto (ou caindo), machuca quem acerta e some ao bater
-- numa parede. Para o prefab do tiro do Atirar no 3D.
local B = {
  title = "Projétil",
  description = "Para o prefab de um tiro (bola de fogo, flecha, pedra): anda, acerta quem tem Vida (tira vida e empurra) e some ao bater numa parede. Não acerta quem atirou.",
  category = "Ações",
  mode = "3d",
  props = {
    { "dano", 1, label = "Dano", min = 0 },
    { "alvo", "", type = "object", label = "Acerta", allow_empty = "os inimigos de quem atirou",
      help = "Vazio: o tiro do jogador acerta quem tem Vida (ou é um Inimigo 3D); o de um inimigo acerta o jogador." },
    { "empurrao", 4, label = "Empurrão", min = 0, help = "Metros por segundo." },
    { "atravessa", false, label = "Atravessa quem acerta",
      help = "Ligado: continua andando depois de acertar (cada um uma vez só)." },
    { "parede", true, label = "Some ao bater numa parede" },
    { "gravidade", 0, label = "Cai", min = 0, max = 3,
      help = "0 = reto; 1 = cai como uma pedra jogada (a gravidade da cena)." },
    { "dura", 4, label = "Some depois de", min = 0, help = "Segundos (0 = não some sozinho)." },
    { "cor", "#FFD166", type = "color", label = "Cor das faíscas", advanced = true },
  },
}

function B:start()
  if not self.scene._3d then
    error(("objeto \"%s\": o Projétil só funciona numa cena 3D"):format(self.obj.name), 0)
  end
  self.vida = self.dura
  self.acertados = {}
  self.v = { 0, 0, 0 }
  local x0, y0, z0, x1, y1, z1 = self.obj:local_bounds3d()
  local _, _, _, _, _, _, sx, sy, sz = self.obj:matrix3d():decompose()
  self.raio = math.max(0.05, math.min((x1 - x0) * math.abs(sx), (y1 - y0) * math.abs(sy), (z1 - z0) * math.abs(sz)) / 2)
end

-- A velocidade: a do corpo, ou a que o Atirar deu (vx, vy, vz), que este
-- comportamento passa a cuidar.
function B:velocidade()
  local o = self.obj
  local b = o.body
  if b and b.alive then return b.vx, b.vy, b.vz end
  if o.vx or o.vy or o.vz then
    self.v = { o.vx or 0, o.vy or 0, o.vz or 0 }
    o.vx, o.vy, o.vz = nil, nil, nil
  end
  local v = self.v
  return v[1], v[2], v[3]
end

-- Quem atirou (e as coisas presas nele) não conta.
function B:e_do_dono(o)
  local dono = self.obj._dono
  while o do
    if o == dono or o == self.obj then return true end
    o = o.parent
  end
  return false
end

-- Bateu em `quem` (um objeto, ou nil = o cenário). Devolve true se o tiro
-- acabou.
function B:impacto(quem, x, y, z, dx, dz)
  local scene = self.scene
  if quem and self.acertados[quem] then return false end
  local alvos = {}
  for _, a in ipairs(scene:foes(self.obj._dono, self.alvo)) do alvos[a] = true end
  if quem and alvos[quem] then
    self.acertados[quem] = true
    scene:hit(quem, self.dano, self.obj._dono or self.obj, { push = self.empurrao, dx = dx, dz = dz, up = 0.35 })
    scene:burst(x, y, self.cor, 10, z)
    if self.atravessa then return false end
    self.obj:send("acertou", quem)
    self.obj:destroy()
    return true
  end
  if not self.parede then return false end
  scene:burst(x, y, self.cor, 8, z)
  self.obj:send("bateu")
  self.obj:destroy()
  return true
end

function B:fixed_update(dt)
  local o = self.obj
  if not o.alive then return end
  if self.dura > 0 then
    self.vida = self.vida - dt
    if self.vida <= 0 then
      o:destroy()
      return
    end
  end
  local scene = self.scene
  local vx, vy, vz = self:velocidade()
  local b = o.body
  local corpo = b and b.alive
  local g = self.gravidade * (scene._gravity or -20)
  if g ~= 0 then
    vy = vy + g * dt
    if corpo then b:set_velocity(vx, vy, vz) else self.v[2] = vy end
  end
  local x, y, z = o:position3d()
  local passo = math.sqrt(vx * vx + vy * vy + vz * vz) * dt
  -- Para onde aponta: a frente (+Z) do modelo vai junto com o tiro.
  local chao = math.sqrt(vx * vx + vz * vz)
  if passo > 1e-6 then
    o.ry = math.deg(math.atan(vx, vz))
    o.rx = -math.deg(math.atan(vy, chao))
  end
  -- O raio até onde o tiro vai neste passo: o primeiro que ele acerta.
  if passo > 1e-6 then
    local n = passo / dt
    local dx, dy, dz = vx / n, vy / n, vz / n
    local mundo = scene:world3()
    local de = 0
    -- O raio não vê o próprio tiro (ou, sem corpo, quem atirou).
    local dono = o._dono
    local ignora = corpo and b or (dono and dono.body and dono.body.alive and dono.body) or nil
    for _ = 1, 4 do
      local hit = mundo:raycast(x + dx * de, y + dy * de, z + dz * de, dx, dy, dz,
        passo + self.raio - de, ignora)
      if not hit then break end
      local quem = scene._by_body3 and scene._by_body3[hit.body.id]
      if quem and self:e_do_dono(quem) then
        -- Atravessa quem atirou: continua o raio depois dele.
        de = de + math.max(hit.distance + 0.05, 0.25)
        if de >= passo + self.raio then break end
      else
        if self:impacto(quem, hit.x, hit.y, hit.z, dx, dz) then return end
        if quem then self.acertados[quem] = true end
        break
      end
    end
  end
  if not corpo then
    local v = self.v
    o.x, o.y, o.z = o.x + v[1] * dt, o.y + v[2] * dt, o.z + v[3] * dt
  end
  -- Quem tem Vida e não tem corpo (o raio não acha): encostou.
  for _, a in ipairs(scene:foes(o._dono, self.alvo)) do
    if not self.acertados[a] and not (a.body and a.body.alive) and o:touching(a, 0.05) then
      local ax, ay, az = a:position3d()
      if self:impacto(a, ax, ay, az, vx, vz) then return end
    end
  end
end

return B
