local util = require("lib.util")

local pausa = { overlay = true }

function pausa:enter(args)
  self.fase = (args and args.fase) or 1
  if musica then musica:set_volume(util.VOLUME_MUSICA * 0.3) end
  nn.ui.set_focus("pausa", 1)
end

function pausa:exit()
  if musica then musica:set_volume(util.VOLUME_MUSICA) end
end

function pausa:update()
  -- O mesmo botão que pausa (Esc, P ou Start) também volta ao jogo.
  if nn.input.action_pressed("pausa") then nn.scene.pop() end
end

function pausa:draw()
  local w, h = nn.window.size()
  nn.screen.rect(0, 0, w, h, { 0.04, 0.06, 0.1, 0.55 })
  local ui = nn.ui.panel("pausa", { title = "Pausado" })
  if ui:button("Continuar") or ui:back() then nn.scene.pop() end
  if ui:button("Recomeçar") then nn.scene.switch("cenas/fase.lua", { fase = self.fase }, { fade = 0.4 }) end
  if ui:button("Opções") then nn.scene.push("cenas/opcoes.lua") end
  if ui:button("Título") then nn.scene.switch("cenas/titulo.lua", nil, { fade = 0.6 }) end
  ui:finish()
end

return pausa
