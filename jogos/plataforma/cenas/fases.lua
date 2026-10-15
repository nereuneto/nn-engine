-- Escolha de fase: aparece por cima do título quando há mais de uma liberada.
local util = require("lib.util")
local jogo = require("lib.jogo")

local fases = { overlay = true }

function fases:enter()
  -- Começa com o foco na fase mais avançada já liberada.
  nn.ui.set_focus("fases", jogo.liberadas())
end

function fases:draw()
  local w, h = nn.window.size()
  nn.screen.rect(0, 0, w, h, { 0.03, 0.06, 0.1, 0.45 })

  local liberadas = jogo.liberadas()
  local ui = nn.ui.panel("fases", { title = "Escolha a fase", width = 560 })
  for n, f in ipairs(jogo.FASES) do
    if n <= liberadas then
      local texto = ("%d. %s"):format(n, f.nome)
      local rec = jogo.recordes(n)
      if rec.tempo then texto = texto .. ("   ·   %s"):format(util.tempo(rec.tempo)) end
      if ui:button(texto) then
        nn.scene.switch("cenas/fase.lua", { fase = n }, { fade = 0.6 })
      end
    else
      ui:label(("%d. %s  (complete a fase %d)"):format(n, f.nome, n - 1))
    end
  end
  ui:space(6)
  if ui:button("Voltar") or ui:back() then nn.scene.pop() end
  ui:finish()
end

return fases
