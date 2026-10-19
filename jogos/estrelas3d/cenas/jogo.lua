-- A partida: ache as 5 estrelas. Moedas são extras; cair no mar volta para
-- o último lugar seguro.
local jogo = require("lib.jogo")
local ilha = require("lib.ilha")

local cena = {}

local INICIO = { 0, 0, 9 }
local VELOCIDADE = 6
local PULO = 8

function cena:enter()
  jogo.preparar()
  self.ilha = ilha.nova()
  self.heroi_modelo = nn.assets.model("modelos/heroi.glb")
  -- As animações do herói: parado, andar, correr, pular, cair e acenar.
  self.anim = self.heroi_modelo:animator()
  self.anim:play("parado")
  self.heroi = self.ilha.mundo:character({ x = INICIO[1], y = INICIO[2], z = INICIO[3], tag = "heroi" })
  self.direcao = 180 -- de costas para a câmera
  self.seguro = { table.unpack(INICIO) }

  -- A câmera atrás do herói: o mouse (preso) e o analógico direito giram.
  local op = jogo.opcoes()
  self.camera = nn.d3.third_person({ distance = 7, pitch = 18, height = 1.4,
    sensitivity = op.sensibilidade, invert_y = op.inverter })
  nn.window.capture_mouse(true)

  self.sons = {
    moeda = nn.assets.sound("assets/sons/moeda.wav"),
    estrela = nn.assets.sound("assets/sons/vitoria.wav"),
    pulo = nn.assets.sound("assets/sons/pulo.wav"),
    agua = nn.assets.sound("assets/sons/dano.wav"),
  }
  self.estrelas, self.moedas, self.tempo = 0, 0, 0
  self.total = self.ilha:restam("estrela")
  self.aviso, self.aviso_tempo = "Ache as 5 estrelas!", 3
end

-- Voltou da pausa: prende o mouse de novo e pega as opções novas.
function cena:resume()
  local op = jogo.opcoes()
  self.camera.sensitivity = op.sensibilidade
  self.camera.invert_y = op.inverter
  nn.window.capture_mouse(true)
end

function cena:avisar(texto)
  self.aviso, self.aviso_tempo = texto, 2.5
end

function cena:fixed_update(dt)
  for _, e in ipairs(self.ilha:passo(dt)) do
    -- O sensor (moeda, estrela) vem em "a".
    if e.type == "enter" and e.b == self.heroi then
      local it = self.ilha:pegar(e.a)
      if it and it.tipo == "moeda" then
        self.moedas = self.moedas + 1
        nn.audio.play(self.sons.moeda, { volume = 0.6 })
      elseif it and it.tipo == "estrela" then
        self.estrelas = self.estrelas + 1
        nn.audio.play(self.sons.estrela)
        self:avisar(("Estrela! %d de %d"):format(self.estrelas, self.total))
        if self.estrelas == self.total then self.venceu = 0 end
      end
    end
  end
end

function cena:update(dt)
  if nn.input.action_pressed("pausa") then
    nn.window.capture_mouse(false)
    nn.scene.push("cenas/pausa.lua")
    return
  end
  if self.venceu then
    self.venceu = self.venceu + dt
    if self.venceu > 1.5 then
      nn.scene.switch("cenas/fim.lua", { tempo = self.tempo, moedas = self.moedas }, { fade = 0.8 })
    end
  else
    self.tempo = self.tempo + dt
  end
  self.aviso_tempo = self.aviso_tempo - dt

  -- Andar para onde a câmera olha (teclado ou analógico).
  local ix, iy = nn.input.move()
  local vx, vz = self.camera:direction(ix, iy)
  if self.venceu then vx, vz = 0, 0 end
  self.heroi:walk(vx * VELOCIDADE, vz * VELOCIDADE)
  if vx ~= 0 or vz ~= 0 then
    -- Vira aos poucos para a direção do movimento.
    local alvo = math.deg(math.atan(vx, vz))
    local diff = (alvo - self.direcao + 540) % 360 - 180
    self.direcao = self.direcao + diff * math.min(1, dt * 12)
  end
  if nn.input.action_pressed("pular") and self.heroi:jump(PULO) then
    nn.audio.play(self.sons.pulo, { volume = 0.5 })
  end

  -- A animação conforme o movimento (a troca é suave). Venceu: acena.
  local hh = self.heroi
  if self.venceu then
    self.anim:play("acenar")
  elseif not hh.on_ground then
    self.anim:play(hh.vy > 0 and "pular" or "cair", { loop = hh.vy <= 0 })
  elseif vx ~= 0 or vz ~= 0 then
    self.anim:play(math.sqrt(vx * vx + vz * vz) > 0.7 and "correr" or "andar")
  else
    self.anim:play("parado")
  end
  self.anim:update(dt)

  -- Último lugar seguro: no chão da ilha.
  local h = self.heroi
  if h.on_ground and h.y > -0.1 and h.y < 0.1 and h.x * h.x + h.z * h.z < 12 * 12 then
    self.seguro = { h.x, h.y, h.z }
  end
  -- Caiu no mar: volta.
  if h.y < -3 then
    nn.audio.play(self.sons.agua)
    h:set_position(self.seguro[1], self.seguro[2] + 0.5, self.seguro[3])
    self:avisar("Splash! De volta à ilha")
  end

  -- A câmera segue o herói e não atravessa árvores nem plataformas.
  self.camera:update(dt, h.x, h.y, h.z, { world = self.ilha.mundo, ignore = h })
end

function cena:draw()
  self.ilha:desenhar()
  local h = self.heroi
  nn.d3.draw(self.heroi_modelo, { x = h.x, y = h.y, z = h.z, ry = self.direcao, scale = 1.1,
    animator = self.anim })

  -- HUD
  local w, hh = nn.window.size()
  local s = jogo.escala()
  nn.screen.panel(16 * s, 16 * s, 300 * s, 92 * s, { color = "#00000066", radius = 12 * s })
  nn.screen.text(("Estrelas  %d / %d"):format(self.estrelas, self.total), 32 * s, 26 * s,
    { size = 28 * s, color = "#FFD93D" })
  nn.screen.text(("Moedas  %d     %s"):format(self.moedas, jogo.tempo(self.tempo)), 32 * s, 66 * s,
    { size = 22 * s, color = "#FFFFFF" })

  if self.aviso_tempo > 0 then
    local a = math.min(1, self.aviso_tempo)
    nn.screen.text(self.aviso, w / 2, hh * 0.3, { size = 40 * s, anchor = "center", color = "#FFFFFF", alpha = a })
  end

  -- Uma seta em cima da estrela mais perto, para ajudar a achar.
  local perto, melhor = nil, math.huge
  for _, it in pairs(self.ilha.itens) do
    if it.tipo == "estrela" then
      local d = (it.x - h.x) ^ 2 + (it.z - h.z) ^ 2
      if d < melhor then perto, melhor = it, d end
    end
  end
  if perto then
    local sx, sy = nn.d3.to_screen(perto.x, perto.y + 1.4, perto.z)
    if sx then
      nn.screen.text("▼", sx, sy, { size = 26 * s, anchor = "center", color = "#FFD93D", alpha = 0.9 })
    end
  end
end

function cena:exit()
  nn.window.capture_mouse(false)
end

return cena
