-- Fim da fase: aparece por cima do jogo com o resultado e os recordes.
local util = require("lib.util")
local jogo = require("lib.jogo")

local vitoria = { overlay = true }

-- Tempo até o menu aparecer (evita confirmar sem querer com o pulo).
local ESPERA_MENU = 0.8

function vitoria:enter(r)
  self.r = r
  self.t = 0
  self.novo, self.recordes = jogo.registrar(r)
  nn.ui.set_focus("vitoria", 1)
end

function vitoria:update(dt)
  self.t = self.t + dt
end

function vitoria:draw()
  local w, h = nn.window.size()
  local s = util.escala()
  local k = math.min(self.t / 0.4, 1)
  nn.screen.rect(0, 0, w, h, { 0.03, 0.05, 0.1, 0.45 * k })

  local y = h * 0.16
  util.texto_sombra("Fase completa!", w / 2, y, { size = 72 * s, anchor = "center", color = "#FFE27A", alpha = k })
  local r, rec, novo = self.r, self.recordes, self.novo
  local linhas = {
    { ("Moedas: %d de %d"):format(r.moedas, r.total), novo.moedas },
    { ("Tempo: %s"):format(util.tempo(r.tempo)), novo.tempo },
    { ("Quedas: %d"):format(r.mortes) },
  }
  for i, linha in ipairs(linhas) do
    local texto = linha[1]
    if linha[2] and rec.vitorias > 1 then texto = texto .. "  — recorde!" end
    util.texto_sombra(texto, w / 2, y + (40 + i * 42) * s,
      { size = 30 * s, anchor = "center", alpha = k, color = linha[2] and rec.vitorias > 1 and "#8CE07A" or "#FFFFFF" })
  end
  util.texto_sombra(("Melhor tempo: %s    Mais moedas: %d"):format(util.tempo(rec.tempo), rec.moedas),
    w / 2, y + 220 * s, { size = 22 * s, anchor = "center", alpha = k, color = "#DDE4DE" })

  if self.t > ESPERA_MENU then
    local ui = nn.ui.panel("vitoria", { x = w / 2, y = h * 0.74, width = 340 })
    if ui:button("Jogar de novo") then nn.scene.switch("cenas/fase.lua", nil, { fade = 0.5 }) end
    if ui:button("Título") then nn.scene.switch("cenas/titulo.lua", nil, { fade = 0.6 }) end
    ui:finish()
  end
end

return vitoria
