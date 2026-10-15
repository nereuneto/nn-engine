-- Pausa: aparece por cima do jogo (overlay), que continua desenhado mas parado.
local util = require("lib.util")

local pausa = { overlay = true }

function pausa:enter()
  if musica then musica:set_volume(0.12) end
end

function pausa:exit()
  if musica then musica:set_volume(0.35) end
end

function pausa:update(dt)
  if nn.input.pressed("escape") or nn.input.pressed("p") then nn.scene.pop() end
  if nn.input.pressed("m") then nn.scene.switch("cenas/menu.lua") end
end

function pausa:draw()
  local w, h = nn.window.size()
  nn.screen.rect(0, 0, w, h, { 0.05, 0.06, 0.08, 0.6 })
  util.texto_sombra("Pausado", w / 2, h / 2 - 10, { size = 64, anchor = "bottom" })
  util.texto_sombra("P/Esc: continuar    M: menu", w / 2, h / 2 + 10,
    { size = 22, anchor = "top", color = "#B8C4BB" })
end

return pausa
