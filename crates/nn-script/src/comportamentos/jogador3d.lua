-- Jogador 3D: anda, pula e leva a câmera atrás dele.
local B = {
  title = "Jogador 3D",
  description = "Anda com WASD, as setas ou o analógico e pula com espaço ou A. A câmera fica atrás dele e o mouse gira ela.",
  category = "Jogador",
  mode = "3d",
  player = true,
  props = {
    { "velocidade", 6, label = "Velocidade", min = 0, help = "Metros por segundo." },
    { "pulo", 8, label = "Força do pulo", min = 0, help = "8 pula uns 1,6 m com a gravidade -20." },
    { "pulos_no_ar", 0, label = "Pulos no ar", min = 0, max = 5, integer = true,
      help = "1 = pulo duplo." },
    { "camera", "atras", label = "Câmera",
      help = "Atrás: a câmera segue o jogador, olhando para onde ele está virado (o ry dele); a Câmera 3D da cena não vale. Para usar a da cena, escolha \"a Câmera 3D da cena\".",
      options = {
        { "atras", "atrás do jogador (3ª pessoa)" },
        { "cena", "a Câmera 3D da cena" },
      } },
    { "distancia", 7, label = "Distância da câmera", min = 1, max = 50,
      show_if = { "camera", "atras" } },
    { "prender_mouse", true, label = "O mouse gira a câmera", show_if = { "camera", "atras" },
      help = "O cursor some e o mouse gira a câmera. Esc solta o mouse; um clique na janela prende de novo." },
    { "sensibilidade", 0.15, label = "Sensibilidade do mouse", min = 0.01, max = 1,
      advanced = true, show_if = { "camera", "atras" } },
    { "inverter_y", false, label = "Inverter o mouse para cima e para baixo", advanced = true,
      show_if = { "camera", "atras" } },
    { "ombro", 0, label = "Câmera de lado (ombro)", advanced = true, show_if = { "camera", "atras" },
      help = "Metros para a direita: o personagem fica um pouco à esquerda do meio da tela, bom para mirar. Com um Atirar que mira no meio da tela, 0 vira 0,6." },
    { "virar", true, label = "Virar para onde anda",
      help = "O modelo vira para a direção do movimento (a frente do modelo é o +Z)." },
    { "animar", true, label = "Animar o modelo",
      help = "Num modelo com animações: parado, andar, correr, pular e cair, conforme o movimento." },
    { "anim_parado", "parado", type = "anim", label = "Animação: parado", advanced = true,
      show_if = { "animar", true } },
    { "anim_andar", "andar", type = "anim", label = "Animação: andar", advanced = true,
      show_if = { "animar", true } },
    { "anim_correr", "correr", type = "anim", label = "Animação: correr", advanced = true,
      show_if = { "animar", true }, help = "Com a velocidade a partir de 4,5 m/s (sem ela, andar)." },
    { "anim_pular", "pular", type = "anim", label = "Animação: pular", advanced = true,
      show_if = { "animar", true } },
    { "anim_cair", "cair", type = "anim", label = "Animação: cair", advanced = true,
      show_if = { "animar", true } },
    { "agua", "voltar", label = "Na água", options = { { "voltar", "volta ao chão firme" }, { "nada", "anda no fundo" } },
      help = "A água de um Terreno. Entrou nela: recebe a mensagem \"agua\" (e, com \"volta\", volta ao último chão firme)." },
    { "queda", -10, label = "Volta se cair abaixo de", advanced = true,
      help = "Altura (Y). Caiu abaixo dela: volta ao último chão firme e recebe a mensagem \"caiu\"." },
    { "raio", 0, label = "Raio do corpo", min = 0, advanced = true,
      help = "Metros. 0 = pelo tamanho do modelo." },
    { "altura", 0, label = "Altura do corpo", min = 0, advanced = true,
      help = "Metros, dos pés à cabeça. 0 = pelo tamanho do modelo." },
  },
}

local function apertou_pulo()
  return nn.input.pressed("space") or nn.input.pad_pressed("a")
end

-- Tamanho do corpo pelo modelo (ou pela forma) do objeto, com a escala.
local function tamanho(self)
  local o = self.obj
  local x0, y0, z0, x1, y1, z1 = o:local_bounds3d()
  local _, _, _, _, _, _, sx, sy, sz = o:matrix3d():decompose()
  local r = math.max((x1 - x0) * math.abs(sx), (z1 - z0) * math.abs(sz)) / 2
  local h = (y1 - y0) * math.abs(sy)
  if self.raio > 0 then r = self.raio end
  if self.altura > 0 then h = self.altura end
  r = math.max(0.1, math.min(r, 1.5))
  h = math.max(h, r * 2 + 0.1)
  -- Os pés: o fundo do modelo (que pode não ter a origem nos pés).
  return r, h, math.min(0, y0 * math.abs(sy))
end

function B:start()
  local o, scene = self.obj, self.scene
  if not scene._3d then
    error(("objeto \"%s\": o Jogador 3D só funciona numa cena 3D"):format(o.name), 0)
  end
  local mundo = scene:world3()
  local x, y, z = o:position3d()
  local r, h, pes = tamanho(self)
  y = y + pes
  self.corpo = mundo:character({ x = x, y = y, z = z, radius = r, height = h, tag = o.name })
  o._char3 = { radius = r, height = h, feet = pes }
  scene:_nn_attach_body3(o, self.corpo)
  self.seguro = { x, y, z }
  self.firme = 0
  self.pulos, self.no_ar, self.pedido = 0, 0, 0
  if self.camera == "atras" then
    self.cam = nn.d3.third_person({
      distance = self.distancia, pitch = 18, height = h * 0.8,
      yaw = (o.ry or 0) + 180, sensitivity = self.sensibilidade, invert_y = self.inverter_y,
      shoulder = self.ombro,
    })
    scene._player_cam = self
    if self.prender_mouse then
      nn.window.capture_mouse(true)
      scene._captured = true
    end
    self:seguir(0)
  end
end

-- A câmera atrás do jogador (não atravessa o cenário).
function B:seguir(dt)
  if not self.cam then return end
  local x, y, z = self.obj:position3d(nn.time.alpha())
  self.cam:update(dt, x, y, z, { world = self.scene:world3(), ignore = self.corpo })
end

-- Para onde andar: em relação à câmera (a de 3ª pessoa ou a da cena).
function B:direcao(ix, iy)
  if self.cam then return self.cam:direction(ix, iy) end
  local c = nn.d3.camera()
  local fx, fz = c.target.x - c.x, c.target.z - c.z
  local n = math.sqrt(fx * fx + fz * fz)
  if n < 1e-6 then fx, fz, n = 0, -1, 1 end
  fx, fz = fx / n, fz / n
  -- Direita = frente girada 90° (olhando de cima).
  return -fz * ix + fx * iy, fx * ix + fz * iy
end

function B:update(dt)
  local o, c = self.obj, self.corpo
  if not (c and c.alive) then return end
  if self.cam and self.prender_mouse then
    if nn.input.pressed("escape") then
      nn.window.capture_mouse(false)
      self.scene._captured = false
    elseif not nn.window.mouse_captured() and nn.input.mouse_pressed("left") then
      nn.window.capture_mouse(true)
      self.scene._captured = true
      -- Este clique só prende o mouse (não golpeia nem atira).
      self.scene._mouse_preso_em = nn.time.frame()
    end
  end
  local ix, iy = nn.input.move()
  -- Numa conversa, fica parado.
  local conversa = self.scene._dlg ~= nil
  if conversa then ix, iy = 0, 0 end
  local vx, vz = self:direcao(ix, iy)
  -- Golpeando (comportamento Golpe): fica parado.
  if o._golpeando and o._golpeando.parar then vx, vz = 0, 0 end
  if (o.empurrado or 0) > 0 then
    -- Levou um empurrão: o controle volta daqui a pouco.
    vx, vz = 0, 0
  else
    c:walk(vx * self.velocidade, vz * self.velocidade)
  end
  if self.virar and (vx ~= 0 or vz ~= 0) then
    -- Vira aos poucos para a direção do movimento.
    local alvo = math.deg(math.atan(vx, vz))
    local diff = (alvo - o.ry + 540) % 360 - 180
    o.ry = o.ry + diff * math.min(1, dt * 12)
    if o._s3 then o._s3[5] = o.ry end
  end
  if apertou_pulo() and not conversa and not self.scene._dlg_closed then self.pedido = 0.12 end
  self:animar_modelo(vx, vz)
  self:seguir(dt)
end

-- Escolhe a animação do modelo pelo movimento (se ele tiver animações).
function B:animar_modelo(vx, vz)
  local a = self.obj:animator()
  if not (self.animar and a) then return end
  -- O Golpe (ou o Atirar) toca a animação dele.
  if self.obj._golpeando or (self.obj._atirando or 0) > nn.time.elapsed() then return end
  local c = self.corpo
  local nome, repetir = nil, true
  if not c.on_ground and self.no_ar > 0.08 then
    if c.vy > 0.5 then
      nome, repetir = self.anim_pular, false
    else
      nome = a:has(self.anim_cair) and self.anim_cair or self.anim_pular
      repetir = nome == self.anim_cair
    end
  else
    local v = math.sqrt(vx * vx + vz * vz) * self.velocidade
    if (self.obj.empurrado or 0) > 0 or v < 0.1 then
      nome = self.anim_parado
    elseif v >= 4.5 and a:has(self.anim_correr) then
      nome = self.anim_correr
    else
      nome = self.anim_andar
    end
  end
  if nome and a:has(nome) then a:play(nome, { loop = repetir, blend = 0.15 }) end
end

function B:fixed_update(dt)
  local o, c = self.obj, self.corpo
  if not (c and c.alive) then return end
  if (o.empurrado or 0) > 0 then o.empurrado = o.empurrado - dt end
  if c.y < self.queda then
    -- Caiu do mundo: volta ao último chão firme.
    local s = self.seguro
    c:set_position(s[1], s[2] + 0.2, s[3])
    c:set_velocity(0, 0, 0)
    o._p3 = nil
    self.firme, self.ultimo = 0, nil
    o:send("caiu")
    return
  end
  -- Na água de um terreno.
  local agua = self.scene:water_level(c.x, c.z)
  if agua and c.y < agua - 0.5 then
    if not self.molhado then
      self.molhado = true
      o:send("agua")
    end
    if self.agua == "voltar" then
      local s = self.seguro
      c:set_position(s[1], s[2] + 0.2, s[3])
      c:set_velocity(0, 0, 0)
      o._p3 = nil
      self.firme, self.ultimo = 0, nil
      return
    end
  else
    self.molhado = false
  end
  -- Pulou de lugar (levado pelo jogo): o chão de antes não vale.
  local u = self.ultimo
  if u and (c.x - u[1]) ^ 2 + (c.y - u[2]) ^ 2 + (c.z - u[3]) ^ 2 > 1 then self.firme = -0.1 end
  self.ultimo = { c.x, c.y, c.z }
  if c.on_ground then
    self.pulos, self.no_ar = 0, 0
    -- Chão firme há um tempinho: lugar seguro para voltar se cair.
    self.firme = self.firme + dt
    if self.firme > 0.3 then self.seguro = { c.x, c.y, c.z } end
  else
    self.no_ar = self.no_ar + dt
    self.firme = 0
  end
  if self.pedido > 0 then
    local pulou = false
    if self.no_ar <= 0.1 then
      pulou = c:jump(self.pulo) or (self.no_ar > 0 and self:pular_no_ar())
    elseif self.pulos < self.pulos_no_ar then
      self.pulos = self.pulos + 1
      pulou = self:pular_no_ar()
    end
    if pulou then
      self.pedido = 0
      self.no_ar = math.huge
      o:send("pulou")
    end
  end
  self.pedido = math.max(0, self.pedido - dt)
end

function B:pular_no_ar()
  self.corpo.vy = self.pulo
  return true
end

function B:destroyed()
  if self.scene._player_cam == self then self.scene._player_cam = nil end
end

return B
