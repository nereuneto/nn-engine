-- Tela de título com o menu principal.
local jogo = require("lib.jogo")

local titulo = {}

function titulo:enter()
  jogo.preparar()
  self.heroi = nn.assets.sheet("assets/imagens/heroi.png", 16, 16, { pixel_art = true })
  self.anim = nn.animator(nn.animation({ 5, 6, 7, 8 }, 10))
end

function titulo:update(dt)
  self.anim:update(dt)
end

function titulo:draw()
  local w, h = nn.window.size()
  local s = jogo.escala()

  nn.screen.text("{{TITULO}}", w / 2, h * 0.2, { size = 72 * s, anchor = "center", color = "#FFFFFF" })
  nn.screen.sprite(self.heroi, w / 2 - 48 * s, h * 0.2 + 50 * s,
    { frame = self.anim:frame(), w = 96 * s, h = 96 * s })

  -- Menu: funciona com mouse, teclado e controle.
  local ui = nn.ui.panel("titulo", { x = w / 2, y = h * 0.66, width = 340 })
  if ui:button("Jogar") then
    nn.scene.switch("cenas/jogo.lua", nil, { fade = 0.6 })
  end
  if ui:button("Sair") then
    nn.quit({ fade = 0.4 })
  end
  local recorde = nn.save.get("recorde", 0)
  if recorde > 0 then
    ui:label(("Recorde: %d moedas"):format(recorde), { size = 0.8 })
  end
  ui:finish()

  local dica = "WASD ou setas: andar    Esc: pausa"
  if nn.input.last_device() == "gamepad" then
    dica = "Analógico ou direcional: andar    Start: pausa"
  end
  nn.screen.text(dica, w / 2, h - 24 * s, { size = 18 * s, anchor = "bottom", color = "#9FB3C8" })
end

return titulo
