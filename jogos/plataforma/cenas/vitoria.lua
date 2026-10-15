-- Fim da fase: aparece por cima do jogo com o resultado.
local util = require("lib.util")

local vitoria = { overlay = true }

function vitoria:enter(r)
  self.r = r
  self.t = 0
end

function vitoria:update(dt)
  self.t = self.t + dt
  if self.t > 0.6 and (nn.input.pressed("enter") or nn.input.pressed("space")) then
    nn.scene.switch("cenas/fase.lua")
  end
  if nn.input.pressed("escape") then nn.scene.switch("cenas/titulo.lua") end
end

function vitoria:draw()
  local w, h = nn.window.size()
  local k = math.min(self.t / 0.4, 1)
  nn.screen.rect(0, 0, w, h, { 0.03, 0.05, 0.1, 0.45 * k })
  local y = h * 0.3
  util.texto_sombra("Fase completa!", w / 2, y, { size = 72, anchor = "center", color = "#FFE27A", alpha = k })
  local linhas = {
    ("Moedas: %d de %d"):format(self.r.moedas, self.r.total),
    ("Tempo: %s"):format(util.tempo(self.r.tempo)),
    ("Quedas: %d"):format(self.r.mortes),
  }
  for i, linha in ipairs(linhas) do
    util.texto_sombra(linha, w / 2, y + 60 + i * 44, { size = 30, anchor = "center", alpha = k })
  end
  if self.r.moedas == self.r.total then
    util.texto_sombra("Todas as moedas!", w / 2, y + 60 + 4 * 44 + 10, { size = 26, anchor = "center", color = "#8CE07A", alpha = k })
  end
  nn.screen.text("Enter: jogar de novo    Esc: título", w / 2, h - 40,
    { size = 20, anchor = "bottom", color = "#DDE4DE", alpha = k })
end

return vitoria
