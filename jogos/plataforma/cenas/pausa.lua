local util = require("lib.util")

local pausa = { overlay = true }

function pausa:enter() if musica then musica:set_volume(0.1) end end
function pausa:exit() if musica then musica:set_volume(0.3) end end

function pausa:update()
  if nn.input.pressed("escape") or nn.input.pressed("p") then nn.scene.pop() end
  if nn.input.pressed("r") then nn.scene.switch("cenas/fase.lua") end
  if nn.input.pressed("m") then nn.scene.switch("cenas/titulo.lua") end
end

function pausa:draw()
  local w, h = nn.window.size()
  nn.screen.rect(0, 0, w, h, { 0.04, 0.06, 0.1, 0.55 })
  util.texto_sombra("Pausado", w / 2, h / 2 - 10, { size = 64, anchor = "bottom" })
  util.texto_sombra("Esc: continuar    R: recomeçar    M: título", w / 2, h / 2 + 10,
    { size = 22, anchor = "top", color = "#DDE4DE" })
end

return pausa
