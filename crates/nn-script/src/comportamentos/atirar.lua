-- Cria tiros (um prefab) ao apertar uma tecla, ou sozinho.
local B = {
  title = "Atirar",
  description = "Cria um tiro (um prefab) ao apertar uma tecla, ou sozinho de tempos em tempos.",
  category = "Ações",
  mode = "both",
  props = {
    { "prefab", "", type = "prefab", label = "Tiro (prefab)" },
    { "tecla", "x", default3d = "clique_direito", label = "Tecla",
      help = "Nome da tecla (x, space, z...), \"clique\" (o botão esquerdo do mouse) ou \"clique_direito\". Vazio = atira sozinho. No controle, o X (no 3D, o RT ou o RB)." },
    { "intervalo", 0.3, label = "Intervalo (segundos)", min = 0 },
    { "velocidade", 700, default3d = 14, label = "Velocidade do tiro", min = 0,
      help = "No 3D, em metros por segundo." },
    { "direcao", "frente", default3d = "camera", label = "Direção",
      options = {
        { "frente", "para onde está virado" },
        { "jogador", "na direção do jogador" },
        { "camera", "para o meio da tela, onde a câmera olha (3D)" },
        { "direita", "direita" }, { "esquerda", "esquerda" },
        { "cima", "cima" }, { "baixo", "baixo" },
      } },
    { "som", "", type = "sound", label = "Som" },
    { "distancia", 24, default3d = 0.8, label = "Sai a esta distância", min = 0, advanced = true },
    { "altura", 1, label = "Sai nesta altura (3D)", min = 0, advanced = true, mode = "3d",
      help = "Metros acima dos pés." },
    { "osso", "", type = "bone", label = "Sai do osso", mode = "3d", allow_empty = "(da frente)",
      help = "Um osso do modelo (\"mao_d\": a mão direita do herói): o tiro sai dele, em vez da frente do objeto." },
    { "mira", true, label = "Mira no meio da tela", mode = "3d", show_if = { "direcao", "camera" },
      help = "Uma cruzinha no meio da tela (só no jogador)." },
    { "animacao", "atirar", type = "anim", label = "Animação", mode = "3d", allow_empty = "nenhuma",
      help = "Toca no modelo ao atirar (se ele tiver)." },
    { "atraso", 0.08, label = "O tiro sai depois de", min = 0, advanced = true, mode = "3d",
      help = "Segundos depois de começar a animação (o braço sobe antes). Sem animação, sai na hora." },
    { "duracao", 3, label = "O tiro some depois de (3D)", min = 0, mode = "3d",
      help = "Segundos (0 = não some sozinho)." },
  },
}

function B:start()
  self.espera = self.tecla == "" and self.intervalo or 0
end

-- A tecla (ou o botão do mouse, ou do controle) está apertada?
function B:quer_atirar()
  local t = self.tecla
  if t == "" then return true end
  local d3 = self.scene._3d
  if d3 then
    if nn.input.pad_down("rt") or nn.input.pad_down("rb") then return true end
  elseif nn.input.pad_down("x") then
    return true
  end
  if t == "clique" or t == "clique_direito" then
    if not nn.input.mouse_down(t == "clique" and "left" or "right") then return false end
    -- O clique que prende o mouse na janela não atira.
    local cam = self.scene._player_cam
    if self.scene._mouse_preso_em == nn.time.frame() then return false end
    return not (cam and cam.prender_mouse and not nn.window.mouse_captured())
  end
  return nn.input.down(t)
end

function B:update(dt)
  -- Mirando no meio da tela: a câmera do jogador vai para o ombro (senão a
  -- mira fica em cima da cabeça dele).
  if not self.ombro_feito and self.scene._3d then
    self.ombro_feito = true
    local pc = self.scene._player_cam
    if self.mira and self.direcao == "camera" and pc and pc.obj == self.obj and pc.cam
        and pc.cam.shoulder == 0 then
      pc.cam.shoulder = 0.6
    end
  end
  self.espera = math.max(0, self.espera - dt)
  -- Esperando o braço subir para o tiro sair.
  if self.pendente then
    self.pendente = self.pendente - dt
    if self.pendente <= 0 then
      self.pendente = nil
      if self.obj.alive then self:soltar3d() end
    end
  end
  if self.prefab == "" or self.espera > 0 then return end
  local quer = self:quer_atirar()
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

-- 3D: de onde o tiro sai: o osso escolhido, ou a frente na altura escolhida.
function B:saida3d(fx, fz)
  local o = self.obj
  if self.osso ~= "" then
    local a = o:animator()
    local m = a and a:bone(self.osso)
    if m then return (o:matrix3d() * m):position() end
  end
  local x, y, z = o:position3d()
  local d = self.distancia
  return x + fx * d, y + self.altura, z + fz * d
end

-- 3D: a frente do objeto no chão (o +Z do modelo, girado com ele).
function B:frente3d()
  local m = self.obj:matrix3d()
  local x0, _, z0 = m:position()
  local x1, _, z1 = m:transform_point(0, 0, 1)
  local dx, dz = x1 - x0, z1 - z0
  local n = math.sqrt(dx * dx + dz * dz)
  if n < 1e-3 then return 0, 1 end
  return dx / n, dz / n
end

-- 3D: onde a câmera mira: o primeiro que o raio do meio da tela acerta (sem
-- contar quem atira), ou um ponto longe.
function B:alvo_da_camera()
  local c = nn.d3.camera()
  local dx, dy, dz = c.target.x - c.x, c.target.y - c.y, c.target.z - c.z
  local n = math.sqrt(dx * dx + dy * dy + dz * dz)
  if n < 1e-6 then return nil end
  dx, dy, dz = dx / n, dy / n, dz / n
  local hit = self.scene:world3():raycast(c.x, c.y, c.z, dx, dy, dz, 80, self.obj.body)
  if hit and hit.distance > 0.5 then return hit.x, hit.y, hit.z end
  return c.x + dx * 80, c.y + dy * 80, c.z + dz * 80
end

-- 3D: de onde sai e para onde vai (x, y, z, e a direção já normalizada).
function B:tiro3d()
  local d = self.direcao
  local o = self.obj
  local fx, fz = self:frente3d()
  local sx, sy, sz = self:saida3d(fx, fz)
  local function para(ax, ay, az)
    local dx, dy, dz = ax - sx, ay - sy, az - sz
    local n = math.sqrt(dx * dx + dy * dy + dz * dz)
    if n > 1e-3 then return dx / n, dy / n, dz / n end
    return fx, 0, fz
  end
  if d == "jogador" then
    local alvo = self.scene:targets("")[1]
    if alvo then
      local ax, ay, az = alvo:position3d()
      return sx, sy, sz, para(ax, ay + 0.9, az)
    end
  elseif d == "camera" and o._player then
    local ax, ay, az = self:alvo_da_camera()
    if ax then
      local dx, dy, dz = para(ax, ay, az)
      -- Mirando bem perto (ou atrás de quem atira): reto para a frente.
      if dx * fx + dz * fz > 0.2 then return sx, sy, sz, dx, dy, dz end
    end
  elseif d == "cima" then
    return sx, sy, sz, 0, 1, 0
  end
  return sx, sy, sz, fx, 0, fz
end

-- 3D: toca a animação; o tiro sai agora ou, com animação, depois do atraso.
function B:atirar3d()
  local a = self.obj:animator()
  if self.animacao ~= "" and a and a:has(self.animacao) then
    a:play(self.animacao, { loop = false, blend = 0.05, restart = true })
    -- O Jogador 3D não troca a animação por cima desta um tempinho.
    self.obj._atirando = nn.time.elapsed() + math.min(a.duration, 0.5)
    if self.atraso > 0 then
      self.pendente = self.atraso
      return nil
    end
  end
  return self:soltar3d()
end

-- 3D: o tiro sai e anda reto.
function B:soltar3d()
  local x, y, z, dx, dy, dz = self:tiro3d()
  local tiro = self.scene:spawn(self.prefab, x, y, { z = z })
  tiro._dono = self.obj
  tiro.ry = math.deg(math.atan(dx, dz))
  tiro.rx = -math.deg(math.atan(dy, math.sqrt(dx * dx + dz * dz)))
  local v = self.velocidade
  local b = tiro.body
  if b and b.alive then
    if not b.is_character then b.gravity_scale = 0 end
    b:set_velocity(dx * v, dy * v, dz * v)
  else
    tiro.vx, tiro.vy, tiro.vz = dx * v, dy * v, dz * v
  end
  if self.duracao > 0 then tiro._some_em = self.duracao end
  if self.som ~= "" then nn.audio.play(nn.assets.sound(self.som), { x = x, y = y, z = z, distance = 25 }) end
  self.obj:send("atirou", tiro)
  return tiro
end

-- 3D: a mira no meio da tela.
function B:draw()
  if not (self.scene._3d and self.mira and self.direcao == "camera" and self.obj._player) then return end
  if self.scene._player_cam == nil or self.scene._dlg then return end
  local c = nn.d3.camera()
  local sx, sy = nn.d3.to_screen(c.target.x, c.target.y, c.target.z)
  if not sx then return end
  local cor, sombra = "#FFFFFFDD", "#00000080"
  for _, r in ipairs({ { -9, -1, 6, 2 }, { 3, -1, 6, 2 }, { -1, -9, 2, 6 }, { -1, 3, 2, 6 } }) do
    nn.screen.rect(sx + r[1] - 1, sy + r[2] - 1, r[3] + 2, r[4] + 2, sombra)
    nn.screen.rect(sx + r[1], sy + r[2], r[3], r[4], cor)
  end
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
