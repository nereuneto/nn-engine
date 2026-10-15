-- Tela de título com o menu principal.
local util = require("lib.util")
local jogo = require("lib.jogo")

local titulo = {}

function titulo:enter()
  jogo.preparar()
  self.sheet = nn.assets.sheet("assets/imagens/personagem.png", 16, 16, { pixel_art = true })
  self.anim = nn.animator(nn.animation({ 5, 6, 7, 8 }, 10))
  util.tocar_musica()
  nn.camera.set_position(0, 0)
end

function titulo:update(dt)
  self.anim:update(dt)
end

function titulo:draw()
  local w, h = nn.window.size()
  local s = util.escala()
  local chao = h * 0.8
  nn.screen.rect(0, chao, w, h - chao, "#5CAA46")
  nn.screen.rect(0, chao, w, 10 * s, "#84CC60")
  local tam = 96 * s
  local x = (nn.time.elapsed() * 220 * s) % (w + 2 * tam) - tam
  nn.screen.sprite(self.sheet, x, chao - tam, { frame = self.anim:frame(), w = tam, h = tam })

  util.texto_sombra("Plataforma", w / 2, h * 0.14, { size = 88 * s, anchor = "center", color = "#FFFFFF" })
  util.texto_sombra("feito em Lua com a nn-engine", w / 2, h * 0.14 + 62 * s,
    { size = 24 * s, anchor = "center", color = "#1E3A4C" })

  -- O menu: funciona com mouse, teclado e controle.
  local ui = nn.ui.panel("titulo", { x = w / 2, y = h * 0.48, width = 360 })
  if ui:button("Jogar") then nn.scene.switch("cenas/fase.lua", nil, { fade = 0.6 }) end
  if ui:button("Opções") then nn.scene.push("cenas/opcoes.lua") end
  if ui:button("Sair") then nn.quit({ fade = 0.4 }) end
  local rec = jogo.recordes()
  if rec.tempo then
    ui:label(("Recorde: %s  ·  %d moedas"):format(util.tempo(rec.tempo), rec.moedas or 0), { size = 0.8 })
  end
  ui:finish()

  nn.screen.text(jogo.ajuda(), w / 2, h - 24 * s, { size = 18 * s, anchor = "bottom", color = "#E8F5E0" })
end

return titulo
