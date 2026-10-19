-- Título: a ilha girando atrás do menu.
local jogo = require("lib.jogo")
local ilha = require("lib.ilha")

local titulo = {}

function titulo:enter()
  jogo.preparar()
  nn.window.capture_mouse(false)
  self.ilha = ilha.nova()
  self.giro = 20
end

function titulo:update(dt)
  self.giro = self.giro + dt * 6
  nn.d3.orbit({ target = { 0, 2, 0 }, distance = 34, yaw = self.giro, pitch = 22 })
  self.ilha:animar(dt)
end

function titulo:draw()
  self.ilha:desenhar()

  local w, h = nn.window.size()
  local s = jogo.escala()
  nn.screen.text(nn.window.title(), w / 2 + 3 * s, h * 0.2 + 3 * s,
    { size = 72 * s, anchor = "center", color = "#00000055" })
  nn.screen.text(nn.window.title(), w / 2, h * 0.2, { size = 72 * s, anchor = "center", color = "#FFFFFF" })
  nn.screen.text("Ache as 5 estrelas o mais rápido que puder", w / 2, h * 0.2 + 56 * s,
    { size = 22 * s, anchor = "center", color = "#2B2D42" })

  local ui = nn.ui.panel("titulo", { x = w / 2, y = h * 0.64, width = 340 })
  if ui:button("Jogar") then
    nn.scene.switch("cenas/jogo.lua", nil, { fade = 0.6 })
  end
  if ui:button("Sair") then
    nn.quit({ fade = 0.4 })
  end
  local recorde = nn.save.get("recorde")
  if recorde then
    ui:label("Recorde: " .. jogo.tempo(recorde), { size = 0.8 })
  end
  ui:finish()

  local dica = "WASD: andar   Espaço: pular   Mouse: câmera   Esc: pausa"
  if nn.input.last_device() == "gamepad" then
    dica = "Analógico esquerdo: andar   A: pular   Analógico direito: câmera   Start: pausa"
  end
  nn.screen.text(dica, w / 2, h - 24 * s, { size = 18 * s, anchor = "bottom", color = "#2B2D42" })
end

return titulo
