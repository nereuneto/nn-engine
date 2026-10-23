-- Jogador de plataforma: anda para os lados e pula.
local B = {
  title = "Jogador de plataforma",
  description = "Anda para os lados e pula com as setas, WASD, espaço ou o controle.",
  category = "Jogador",
  requires = { "body" },
  player = true,
  props = {
    { "velocidade", 320, label = "Velocidade", min = 0, help = "Unidades do mundo por segundo." },
    { "pulo", 860, label = "Força do pulo", min = 0 },
    { "pulos_no_ar", 0, label = "Pulos no ar", min = 0, max = 5, integer = true,
      help = "1 = pulo duplo." },
    { "coiote", 0.1, label = "Tempo de coiote", min = 0, max = 1, advanced = true,
      help = "Ainda dá para pular logo depois de sair da beirada (segundos)." },
    { "pulo_variavel", true, label = "Pulo mais alto segurando", advanced = true,
      help = "Soltar o botão no meio do pulo deixa o pulo mais baixo (um toque rápido dá um pulinho; segure para o pulo inteiro)." },
    { "som", "", type = "sound", label = "Som do pulo",
      help = "Toca a cada pulo (também o do coiote, o guardado e os no ar)." },
  },
}

local function apertou()
  return nn.input.pressed("space") or nn.input.pressed("up") or nn.input.pressed("w")
    or nn.input.pad_pressed("a")
end

local function segurando()
  return nn.input.down("space") or nn.input.down("up") or nn.input.down("w")
    or nn.input.pad_down("a")
end

local function soltou()
  return nn.input.released("space") or nn.input.released("up") or nn.input.released("w")
    or nn.input.pad_released("a")
end

function B:start()
  self.fora_do_chao = 0
  self.pulos = 0
  self.pedido = 0
  self.cortar = false
  self.subindo = math.huge
end

-- Soltar o botão só corta o pulo depois deste tempo: um toque rápido (no
-- chão, guardado ou num teste) dá sempre o mesmo pulo baixo.
local PULO_MINIMO = 0.1

function B:update(dt)
  -- O aperto fica guardado um pouquinho: apertar logo antes de tocar o chão
  -- também pula.
  -- Numa conversa, fica parado.
  if self.scene._dlg or self.scene._dlg_closed then return end
  if apertou() then self.pedido = 0.12 end
  if self.pulo_variavel and soltou() then self.cortar = true end
end

function B:fixed_update(dt)
  local o = self.obj
  local corpo = o.body
  if not (corpo and corpo.alive) then return end
  if (o.empurrado or 0) > 0 then
    -- Levou um empurrão (dano): o controle volta daqui a pouco.
    o.empurrado = o.empurrado - dt
  else
    local dx = nn.input.move()
    if self.scene._dlg then dx = 0 end
    corpo.vx = dx * self.velocidade
  end
  if corpo.on_ground then
    self.fora_do_chao, self.pulos = 0, 0
  else
    self.fora_do_chao = self.fora_do_chao + dt
  end
  if self.pedido > 0 then
    local pode = self.fora_do_chao <= self.coiote
    if not pode and self.pulos < self.pulos_no_ar then
      self.pulos = self.pulos + 1
      pode = true
    end
    if pode then
      corpo.vy = self.pulo
      self.fora_do_chao = math.huge
      self.pedido, self.subindo = 0, 0
      -- Com o botão já solto (um toque, ou um pulo guardado), o pulo é
      -- baixo.
      self.cortar = self.pulo_variavel and not segurando()
      if self.som ~= "" then
        local s = self.som
        pcall(function() nn.audio.play(nn.assets.sound(s)) end)
      end
      if self.obj.send then self.obj:send("pulou") end
    end
  end
  self.pedido = math.max(0, self.pedido - dt)
  self.subindo = self.subindo + dt
  if self.cortar then
    if corpo.vy <= 0 then
      self.cortar = false
    elseif self.subindo >= PULO_MINIMO then
      corpo.vy = corpo.vy * 0.45
      self.cortar = false
    end
  end
end

return B
