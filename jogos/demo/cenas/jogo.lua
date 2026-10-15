-- O jogo: colete todas as moedas da arena.
local util = require("lib.util")

local jogo = {}

local ARENA_L, ARENA_A = 1600, 1000
local TILE = 64
local VELOCIDADE = 260
local CORRIDA = 1.7
local TAM_JOGADOR = 64
local TAM_MOEDA = 40

function jogo:enter(args)
  self.total = (args and args.moedas) or 20
  self.personagem = nn.assets.sheet("assets/imagens/personagem.png", 16, 16, { pixel_art = true })
  self.moeda = nn.assets.sheet("assets/imagens/moeda.png", 16, 16, { pixel_art = true })
  self.parado = nn.animation({ 1, 2 }, 2)
  self.andando = nn.animation({ 5, 6, 7, 8 }, 10)
  self.anim = nn.animator(self.parado)
  self.som_moeda = nn.assets.sound("assets/sons/moeda.wav")
  self.som_fim = nn.assets.sound("assets/sons/fanfarra.wav")
  self:reiniciar()
end

function jogo:reiniciar()
  self.x, self.y = 0, 0
  self.ax, self.ay = 0, 0 -- posição no passo anterior, para interpolar no desenho
  self.esquerda = false
  self.pontos = 0
  self.venceu = false
  self.popups = {}
  self.moedas = {}

  local giro = nn.animation({ 1, 2, 3, 4 }, 9, "pingpong")
  math.randomseed(2026)
  while #self.moedas < self.total do
    local x = (math.random() * 2 - 1) * (ARENA_L / 2 - TILE)
    local y = (math.random() * 2 - 1) * (ARENA_A / 2 - TILE)
    local livre = util.dist(0, 0, x, y) > 150
    for _, m in ipairs(self.moedas) do
      if util.dist(m.x, m.y, x, y) < 90 then livre = false end
    end
    if livre then
      local anim = nn.animator(giro)
      anim:update(math.random() * giro.duration) -- cada moeda num ponto do giro
      table.insert(self.moedas, { x = x, y = y, anim = anim })
    end
  end
end

-- Movimento e coleta em passos fixos.
function jogo:fixed_update(dt)
  if self.venceu then return end

  local dx, dy = nn.input.wasd()
  local sx, sy = nn.input.arrows()
  dx, dy = dx + sx, dy + sy
  local len = math.sqrt(dx * dx + dy * dy)
  if len > 0 then dx, dy = dx / len, dy / len end

  local correndo = nn.input.down("shift")
  local v = VELOCIDADE * (correndo and CORRIDA or 1)
  self.ax, self.ay = self.x, self.y
  local lx, ly = ARENA_L / 2 - TAM_JOGADOR / 2, ARENA_A / 2 - TAM_JOGADOR / 2
  self.x = util.clamp(self.x + dx * v * dt, -lx, lx)
  self.y = util.clamp(self.y + dy * v * dt, -ly, ly)
  self.movendo = len > 0
  if dx ~= 0 then self.esquerda = dx < 0 end
  self.anim.speed = correndo and CORRIDA or 1

  for i = #self.moedas, 1, -1 do
    local m = self.moedas[i]
    if util.dist(m.x, m.y, self.x, self.y) < 30 + TAM_MOEDA / 2 then
      table.remove(self.moedas, i)
      self.pontos = self.pontos + 1
      table.insert(self.popups, { x = m.x, y = m.y, idade = 0 })

      -- Tom levemente diferente e som posicionado no estéreo.
      local cx = nn.camera.position()
      local w = nn.window.size()
      nn.audio.play(self.som_moeda, {
        speed = 0.95 + math.random() * 0.15,
        pan = util.clamp((m.x - cx) / (w / 2), -1, 1) * 0.7,
      })
      if self.pontos == self.total then
        self.venceu = true
        nn.audio.play(self.som_fim)
      end
    end
  end
end

function jogo:update(dt)
  if nn.input.pressed("escape") or nn.input.pressed("p") then nn.scene.push("cenas/pausa.lua") end
  if self.venceu and nn.input.pressed("enter") then nn.scene.switch("cenas/menu.lua") end
  if nn.input.pressed("r") then self:reiniciar() end

  self.anim:play(self.movendo and self.andando or self.parado)
  self.anim:update(dt)
  for _, m in ipairs(self.moedas) do m.anim:update(dt) end
  for i = #self.popups, 1, -1 do
    local p = self.popups[i]
    p.idade = p.idade + dt
    if p.idade > 0.8 then table.remove(self.popups, i) end
  end
  if dt > 0 then self.fps = util.lerp(self.fps or 60, 1 / dt, 0.05) end
end

function jogo:draw()
  local a = nn.time.alpha()
  local x, y = util.lerp(self.ax, self.x, a), util.lerp(self.ay, self.y, a)

  -- Câmera segue o personagem com suavização.
  local cx, cy = nn.camera.position()
  local t = 1 - math.exp(-6 * nn.time.delta())
  nn.camera.set_position(util.lerp(cx, x, t), util.lerp(cy, y, t))

  self:desenhar_arena()

  for _, m in ipairs(self.moedas) do
    local sobe = math.sin(nn.time.elapsed() * 3 + m.x) * 3
    nn.draw.rect(m.x, m.y - TAM_MOEDA / 2 - 2, TAM_MOEDA * 0.6, 6, { 0, 0, 0, 0.25 })
    nn.draw.sprite(self.moeda, m.x, m.y + sobe, { frame = m.anim:frame(), w = TAM_MOEDA, h = TAM_MOEDA })
  end

  nn.draw.rect(x, y - 30, 40, 8, { 0, 0, 0, 0.3 })
  nn.draw.sprite(self.personagem, x, y, {
    frame = self.anim:frame(), w = TAM_JOGADOR, h = TAM_JOGADOR, flip_x = self.esquerda,
  })

  for _, p in ipairs(self.popups) do
    local k = p.idade / 0.8
    nn.draw.text("+1", p.x, p.y + 30 + 50 * k,
      { size = 26, color = "#FFE27A", alpha = 1 - k * k, anchor = "center" })
  end

  self:desenhar_hud()
end

function jogo:desenhar_arena()
  local claro, escuro = "#3B4A3F", "#334037"
  for ty = 0, ARENA_A / TILE - 1 do
    for tx = 0, ARENA_L / TILE - 1 do
      local cx = -ARENA_L / 2 + (tx + 0.5) * TILE
      local cy = -ARENA_A / 2 + (ty + 0.5) * TILE
      nn.draw.rect(cx, cy, TILE, TILE, (tx + ty) % 2 == 0 and claro or escuro)
    end
  end
  local parede, e = "#5A4632", 12
  nn.draw.rect(0, ARENA_A / 2 + e / 2, ARENA_L + 2 * e, e, parede)
  nn.draw.rect(0, -ARENA_A / 2 - e / 2, ARENA_L + 2 * e, e, parede)
  nn.draw.rect(ARENA_L / 2 + e / 2, 0, e, ARENA_A, parede)
  nn.draw.rect(-ARENA_L / 2 - e / 2, 0, e, ARENA_A, parede)
end

function jogo:desenhar_hud()
  local w, h = nn.window.size()
  local placar = ("Moedas: %d / %d"):format(self.pontos, self.total)
  local tw, th = nn.text_size(placar, { size = 28 })
  nn.screen.rect(10, 10, tw + 20, th + 12, { 0, 0, 0, 0.35 })
  util.texto_sombra(placar, 20, 16, { size = 28, color = "#FFD24A" })
  util.texto_sombra(("%.0f FPS"):format(self.fps or 0), w - 20, 20,
    { size = 18, color = "#B8C4BB", anchor = "topright" })
  util.texto_sombra("WASD/setas: andar   Shift: correr   P/Esc: pausa   R: recomeçar", 20, h - 20,
    { size = 16, color = "#DDE4DE", anchor = "bottomleft" })

  if self.venceu then
    nn.screen.rect(0, 0, w, h, { 0, 0, 0, 0.4 })
    util.texto_sombra("Você pegou todas!", w / 2, h / 2, { size = 56, color = "#FFD24A", anchor = "bottom" })
    util.texto_sombra("Enter: voltar ao menu   R: jogar de novo", w / 2, h / 2 + 12, { size = 22, anchor = "top" })
  end
end

return jogo
