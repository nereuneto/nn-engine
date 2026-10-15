-- O jogo: pegue o máximo de moedas antes do tempo acabar.
-- Bom lugar para começar a mudar as coisas: salve o arquivo com o jogo
-- aberto e a mudança aparece na hora.
local jogo = require("lib.jogo")

local cena = {}

local DURACAO = 30          -- segundos de partida
local VELOCIDADE = 380      -- unidades do mundo por segundo
local ARENA_W, ARENA_H = 1200, 640
local TAMANHO = 64          -- tamanho do herói
local ALCANCE = 44          -- distância para pegar uma moeda

function cena:enter()
  jogo.preparar()
  self.heroi = nn.assets.sheet("assets/imagens/heroi.png", 16, 16, { pixel_art = true })
  self.moeda_img = nn.assets.sheet("assets/imagens/moeda.png", 16, 16, { pixel_art = true })
  self.som_moeda = nn.assets.sound("assets/sons/moeda.wav")
  self.som_fim = nn.assets.sound("assets/sons/fim.wav")

  self.anims = {
    parado = nn.animation({ 1, 2 }, 2),
    andando = nn.animation({ 5, 6, 7, 8 }, 12),
  }
  self.anim = nn.animator(self.anims.parado)
  self.giro = nn.animator(nn.animation({ 1, 2, 3, 4 }, 9, "pingpong"))
  self.brilho = nn.particles({
    emitting = false, lifetime = { 0.3, 0.6 }, speed = { 100, 260 }, drag = 4,
    size = { 14, 0 }, colors = { "#FFFFFF", "#FFD166", "#FF9A3000" }, blend = "add",
  })

  self.x, self.y = 0, 0
  self.ax, self.ay = 0, 0     -- posição no passo anterior (para suavizar no draw)
  self.esquerda = false
  self.andando = false
  self.pontos = 0
  self.tempo = DURACAO
  self.acabou = false
  self:nova_moeda()
end

function cena:nova_moeda()
  local margem = 60
  self.moeda = {
    x = (math.random() - 0.5) * (ARENA_W - 2 * margem),
    y = (math.random() - 0.5) * (ARENA_H - 2 * margem),
  }
end

-- Movimento e regras em passos fixos: igual em qualquer computador.
function cena:fixed_update(dt)
  if self.acabou then return end
  self.ax, self.ay = self.x, self.y

  -- Teclado (WASD ou setas) ou controle (direcional ou analógico).
  local dx, dy = nn.input.move()
  self.x = math.max(-ARENA_W / 2, math.min(ARENA_W / 2, self.x + dx * VELOCIDADE * dt))
  self.y = math.max(-ARENA_H / 2, math.min(ARENA_H / 2, self.y + dy * VELOCIDADE * dt))
  if dx ~= 0 then self.esquerda = dx < 0 end
  self.andando = dx ~= 0 or dy ~= 0

  local m = self.moeda
  if (m.x - self.x) ^ 2 + (m.y - self.y) ^ 2 < ALCANCE ^ 2 then
    self.pontos = self.pontos + 1
    self.brilho:burst(18, m.x, m.y)
    nn.audio.play(self.som_moeda, { speed = 0.95 + math.random() * 0.15 })
    self:nova_moeda()
  end

  self.tempo = self.tempo - dt
  if self.tempo <= 0 then self:terminar() end
end

function cena:terminar()
  self.acabou = true
  self.tempo = 0
  nn.audio.play(self.som_fim)
  nn.camera.shake(14, 0.5)
  nn.screen.flash("#FFFFFF80", 0.3)

  -- O recorde fica salvo mesmo depois de fechar o jogo.
  local recorde = nn.save.get("recorde", 0)
  local novo = self.pontos > recorde
  if novo then nn.save.set("recorde", self.pontos) end
  nn.scene.push("cenas/fim.lua", { pontos = self.pontos, recorde = math.max(recorde, self.pontos), novo = novo })
end

function cena:update(dt)
  if nn.input.action_pressed("pausa") then nn.scene.push("cenas/pausa.lua") end
  self.anim:play(self.andando and self.anims.andando or self.anims.parado)
  self.anim:update(dt)
  self.giro:update(dt)
  self.brilho:update(dt)
end

function cena:draw()
  local w, h = nn.window.size()
  -- A arena inteira sempre cabe na janela, em qualquer tamanho.
  nn.camera.set_position(0, 0)
  nn.camera.set_zoom(math.min(w / (ARENA_W + 80), h / (ARENA_H + 160)))

  nn.draw.rect(0, 0, ARENA_W + 16, ARENA_H + 16, "#2E4A66")
  nn.draw.rect(0, 0, ARENA_W, ARENA_H, "#1D3045")

  local t = nn.time.elapsed()
  nn.draw.sprite(self.moeda_img, self.moeda.x, self.moeda.y + math.sin(t * 4) * 4,
    { frame = self.giro:frame(), w = 40, h = 40 })

  -- Entre dois passos fixos, desenha na posição intermediária: movimento suave.
  local a = nn.time.alpha()
  local x = self.ax + (self.x - self.ax) * a
  local y = self.ay + (self.y - self.ay) * a
  nn.draw.sprite(self.heroi, x, y,
    { frame = self.anim:frame(), w = TAMANHO, h = TAMANHO, flip_x = self.esquerda })
  self.brilho:draw()

  -- HUD: coordenadas da tela, não treme junto com a câmera.
  local s = jogo.escala()
  nn.screen.text(("Moedas: %d"):format(self.pontos), 24 * s, 16 * s, { size = 30 * s, color = "#FFD166" })
  local cor = self.tempo < 5 and "#FF6B6B" or "#FFFFFF"
  nn.screen.text(("%d"):format(math.ceil(self.tempo)), w - 24 * s, 16 * s,
    { size = 30 * s, anchor = "topright", color = cor })
end

return cena
