-- Fim da partida: resultado por cima do jogo.
local fim = { overlay = true }

-- Espera antes dos botões responderem, para ninguém confirmar sem querer.
local ESPERA = 0.6

function fim:enter(resultado)
  self.r = resultado
  self.t = 0
  nn.ui.set_focus("fim", 1)
end

function fim:update(dt)
  self.t = self.t + dt
end

function fim:draw()
  local w, h = nn.window.size()
  nn.screen.rect(0, 0, w, h, { 0, 0, 0, 0.5 * math.min(self.t / 0.3, 1) })

  local r = self.r
  local ui = nn.ui.panel("fim", { title = "Tempo esgotado!", width = 440 })
  ui:label(("Você pegou %d moedas"):format(r.pontos))
  if r.novo then
    ui:label("Novo recorde!", { color = "#FFD166" })
  else
    ui:label(("Recorde: %d"):format(r.recorde))
  end
  ui:space(8)
  local pronto = self.t > ESPERA
  if ui:button("Jogar de novo") and pronto then
    nn.scene.switch("cenas/jogo.lua", nil, { fade = 0.5 })
  end
  if ui:button("Título") and pronto then
    nn.scene.switch("cenas/titulo.lua", nil, { fade = 0.6 })
  end
  ui:finish()
end

return fim
