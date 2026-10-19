-- Cria tiros (um prefab) ao apertar uma tecla, ou sozinho.
local B = {
  title = "Atirar",
  description = "Cria um tiro (um prefab) ao apertar uma tecla, ou sozinho de tempos em tempos.",
  category = "Ações",
  mode = "both",
  props = {
    { "prefab", "", type = "prefab", label = "Tiro (prefab)" },
    { "tecla", "x", label = "Tecla", help = "Nome da tecla (x, space, z...). Vazio = atira sozinho. No controle, o botão X." },
    { "intervalo", 0.3, label = "Intervalo (segundos)", min = 0 },
    { "velocidade", 700, default3d = 14, label = "Velocidade do tiro", min = 0,
      help = "No 3D, em metros por segundo." },
    { "direcao", "frente", label = "Direção",
      options = {
        { "frente", "para onde está virado" },
        { "jogador", "na direção do jogador" },
        { "camera", "para onde a câmera olha (3D)" },
        { "direita", "direita" }, { "esquerda", "esquerda" },
        { "cima", "cima" }, { "baixo", "baixo" },
      } },
    { "som", "", type = "sound", label = "Som" },
    { "distancia", 24, default3d = 0.8, label = "Sai a esta distância", min = 0, advanced = true },
    { "altura", 1, label = "Sai nesta altura (3D)", min = 0, advanced = true, mode = "3d",
      help = "Metros acima dos pés." },
    { "duracao", 3, label = "O tiro some depois de (3D)", min = 0, mode = "3d",
      help = "Segundos (0 = não some sozinho)." },
  },
}

function B:start()
  self.espera = self.tecla == "" and self.intervalo or 0
end

function B:update(dt)
  self.espera = math.max(0, self.espera - dt)
  if self.prefab == "" or self.espera > 0 then return end
  local quer = self.tecla == "" or nn.input.down(self.tecla) or nn.input.pad_down("x")
  -- Numa conversa, o jogador não atira.
  if self.tecla ~= "" and self.scene._dlg then quer = false end
  if quer then
    self.espera = self.intervalo
    self:atirar()
  end
end

function B:direcao_do_tiro()
  local d = self.direcao
  if d == "esquerda" then return -1, 0 end
  if d == "direita" then return 1, 0 end
  if d == "cima" then return 0, 1 end
  if d == "baixo" then return 0, -1 end
  if d == "jogador" then
    local x, y = self.obj:world_transform()
    local alvo = self.scene:targets("")[1]
    if alvo then
      local ax, ay = alvo:world_transform()
      local dist = math.sqrt((ax - x) ^ 2 + (ay - y) ^ 2)
      if dist > 0 then return (ax - x) / dist, (ay - y) / dist end
    end
    return 1, 0
  end
  local spr = self.obj.sprite
  return (spr and spr.flip_x) and -1 or 1, 0
end

-- 3D: a direção do tiro (x, y, z), já normalizada.
function B:direcao3d()
  local d = self.direcao
  local o = self.obj
  if d == "jogador" then
    local x, y, z = o:position3d()
    local alvo = self.scene:targets("")[1]
    if alvo then
      local ax, ay, az = alvo:position3d()
      local dx, dy, dz = ax - x, (ay + 0.9) - (y + self.altura), az - z
      local n = math.sqrt(dx * dx + dy * dy + dz * dz)
      if n > 1e-3 then return dx / n, dy / n, dz / n end
    end
  elseif d == "camera" then
    local c = nn.d3.camera()
    local dx, dy, dz = c.target.x - c.x, c.target.y - c.y, c.target.z - c.z
    local n = math.sqrt(dx * dx + dz * dz)
    if n > 1e-3 then return dx / n, 0, dz / n end
  elseif d == "cima" then
    return 0, 1, 0
  end
  -- Para a frente do objeto: o +Z do modelo, girado com ele.
  local m = o:matrix3d()
  local x0, y0, z0 = m:position()
  local x1, y1, z1 = m:transform_point(0, 0, 1)
  local dx, dz = x1 - x0, z1 - z0
  local n = math.sqrt(dx * dx + dz * dz)
  if n < 1e-3 then return 0, 0, 1 end
  return dx / n, 0, dz / n
end

-- 3D: o tiro sai da frente, na altura escolhida, andando reto.
function B:atirar3d()
  local dx, dy, dz = self:direcao3d()
  local x, y, z = self.obj:position3d()
  local d = self.distancia
  local tiro = self.scene:spawn(self.prefab, x + dx * d, y + self.altura + dy * d, { z = z + dz * d })
  tiro.ry = math.deg(math.atan(dx, dz))
  local v = self.velocidade
  local b = tiro.body
  if b and b.alive then
    if not b.is_character then b.gravity_scale = 0 end
    b:set_velocity(dx * v, dy * v, dz * v)
  else
    tiro.vx, tiro.vy, tiro.vz = dx * v, dy * v, dz * v
  end
  if self.duracao > 0 then tiro._some_em = self.duracao end
  if self.som ~= "" then nn.audio.play(nn.assets.sound(self.som)) end
  return tiro
end

--- Cria um tiro agora.
function B:atirar()
  if self.scene._3d then return self:atirar3d() end
  local dx, dy = self:direcao_do_tiro()
  local x, y = self.obj:world_transform()
  local tiro = self.scene:spawn(self.prefab, x + dx * self.distancia, y + dy * self.distancia)
  local vx, vy = dx * self.velocidade, dy * self.velocidade
  if tiro.body and tiro.body.alive then
    tiro.body.gravity_scale = 0
    tiro.body.vx, tiro.body.vy = vx, vy
  else
    tiro.vx, tiro.vy = vx, vy
  end
  if tiro.sprite and dx < 0 then tiro.sprite.flip_x = true end
  if self.som ~= "" then nn.audio.play(nn.assets.sound(self.som)) end
  return tiro
end

return B
