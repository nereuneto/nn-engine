-- Fim: o tempo, as moedas e o recorde (o menor tempo, salvo).
local jogo = require("lib.jogo")
local ilha = require("lib.ilha")

local fim = {}

function fim:enter(args)
  jogo.preparar()
  nn.window.capture_mouse(false)
  args = args or {}
  self.tempo = args.tempo or 0
  self.moedas = args.moedas or 0
  local recorde = nn.save.get("recorde")
  self.novo = recorde == nil or self.tempo < recorde
  if self.novo then
    nn.save.set("recorde", self.tempo)
    recorde = self.tempo
  end
  self.recorde = recorde
  self.ilha = ilha.nova()
  self.giro = 0
end

function fim:update(dt)
  self.giro = self.giro + dt * 10
  nn.d3.orbit({ target = { 0, 2, 0 }, distance = 30, yaw = self.giro, pitch = 30 })
  self.ilha:animar(dt)
end

function fim:draw()
  self.ilha:desenhar()
  local w, h = nn.window.size()
  local s = jogo.escala()
  nn.screen.text("Todas as estrelas!", w / 2, h * 0.2, { size = 64 * s, anchor = "center", color = "#FFD93D" })
  local ui = nn.ui.panel("fim", { x = w / 2, y = h * 0.6, width = 380 })
  ui:label("Tempo: " .. jogo.tempo(self.tempo))
  ui:label(("Moedas: %d"):format(self.moedas))
  ui:label(self.novo and "Novo recorde!" or ("Recorde: " .. jogo.tempo(self.recorde)),
    { color = self.novo and "#FFD93D" or "#FFFFFF" })
  ui:space(8)
  if ui:button("Jogar de novo") then nn.scene.switch("cenas/jogo.lua", nil, { fade = 0.5 }) end
  if ui:button("Título") then nn.scene.switch("cenas/titulo.lua", nil, { fade = 0.5 }) end
  ui:finish()
end

return fim
