-- Tela de título.
local util = require("lib.util")

local titulo = {}

function titulo:enter()
  self.sheet = nn.assets.sheet("assets/imagens/personagem.png", 16, 16, { pixel_art = true })
  self.anim = nn.animator(nn.animation({ 5, 6, 7, 8 }, 10))
  util.tocar_musica()
  nn.camera.set_position(0, 0)
end

function titulo:update(dt)
  self.anim:update(dt)
  if nn.input.pressed("enter") or nn.input.pressed("space") then
    nn.scene.switch("cenas/fase.lua")
  end
  if nn.input.pressed("escape") then nn.quit() end
end

function titulo:draw()
  local w, h = nn.window.size()
  nn.screen.rect(0, h * 0.72, w, h * 0.28, "#5CAA46")
  nn.screen.rect(0, h * 0.72, w, 10, "#84CC60")
  local x = (nn.time.elapsed() * 220) % (w + 200) - 100
  nn.screen.sprite(self.sheet, x, h * 0.72 - 96, { frame = self.anim:frame(), w = 96, h = 96 })

  util.texto_sombra("Plataforma", w / 2, h * 0.25, { size = 88, anchor = "center", color = "#FFFFFF" })
  util.texto_sombra("feito em Lua com a nn-engine", w / 2, h * 0.25 + 70, { size = 24, anchor = "center", color = "#1E3A4C" })
  local piscar = math.floor(nn.time.elapsed() * 2) % 2 == 0
  if piscar then
    util.texto_sombra("Aperte Enter para jogar", w / 2, h * 0.5, { size = 32, anchor = "center", color = "#FFE27A" })
  end
  nn.screen.text("Setas/AD: andar    Espaço/W: pular    Esc: pausa    F1: ver colisões", w / 2, h - 24,
    { size = 18, anchor = "bottom", color = "#E8F5E0" })
end

return titulo
