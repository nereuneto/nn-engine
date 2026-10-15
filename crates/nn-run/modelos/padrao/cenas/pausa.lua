-- Pausa: aparece por cima do jogo (overlay), que fica parado por baixo.
local pausa = { overlay = true }

function pausa:enter()
  nn.ui.set_focus("pausa", 1)
end

function pausa:update()
  -- O mesmo botão que pausa (Esc, P ou Start) também volta ao jogo.
  if nn.input.action_pressed("pausa") then nn.scene.pop() end
end

function pausa:draw()
  local w, h = nn.window.size()
  nn.screen.rect(0, 0, w, h, { 0, 0, 0, 0.5 })
  local ui = nn.ui.panel("pausa", { title = "Pausado" })
  if ui:button("Continuar") or ui:back() then nn.scene.pop() end
  if ui:button("Recomeçar") then nn.scene.switch("cenas/jogo.lua", nil, { fade = 0.4 }) end
  if ui:button("Título") then nn.scene.switch("cenas/titulo.lua", nil, { fade = 0.6 }) end
  ui:finish()
end

return pausa
