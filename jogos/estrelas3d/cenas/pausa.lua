-- Pausa: por cima do jogo (que fica parado), com as opções da câmera.
local jogo = require("lib.jogo")

local pausa = { overlay = true }

function pausa:enter()
  nn.ui.set_focus("pausa", 1)
  local op = jogo.opcoes()
  self.sensibilidade, self.inverter = op.sensibilidade, op.inverter
end

function pausa:update()
  if nn.input.action_pressed("pausa") then nn.scene.pop() end
end

function pausa:draw()
  local w, h = nn.window.size()
  nn.screen.rect(0, 0, w, h, { 0, 0, 0, 0.45 })
  local ui = nn.ui.panel("pausa", { title = "Pausado", width = 420 })
  if ui:button("Continuar") or ui:back() then nn.scene.pop() end
  local s, mudou = ui:slider("Sensibilidade do mouse", self.sensibilidade, { min = 0.05, max = 0.4, step = 0.01 })
  if mudou then
    self.sensibilidade = s
    nn.save.set("sensibilidade", s)
  end
  local inv, mudou_inv = ui:toggle("Inverter o Y da câmera", self.inverter)
  if mudou_inv then
    self.inverter = inv
    nn.save.set("inverter_y", inv)
  end
  if ui:button("Recomeçar") then nn.scene.switch("cenas/jogo.lua", nil, { fade = 0.4 }) end
  if ui:button("Título") then nn.scene.switch("cenas/titulo.lua", nil, { fade = 0.6 }) end
  ui:finish()
end

return pausa
