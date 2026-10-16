-- Porta, saída, bandeira de fim de fase.
local B = {
  title = "Ir para cena",
  description = "Porta, saída, bandeira de fim de fase: leva para outra cena.",
  category = "Fases",
  props = {
    { "cena", "", type = "scene", label = "Cena" },
    { "quem", "", type = "object", label = "Quem ativa", help = "Vazio = o jogador." },
    { "tecla", false, label = "Precisa apertar para cima", help = "Como uma porta: encoste e aperte para cima." },
    { "escurecer", 0.5, label = "Escurecer (segundos)", min = 0, advanced = true },
  },
}

function B:update(dt)
  if self.indo or self.cena == "" then return end
  for _, quem in ipairs(self.scene:targets(self.quem)) do
    if self.obj:touching(quem, 0) then
      local apertou = nn.input.pressed("up") or nn.input.pressed("w") or nn.input.pad_pressed("up")
      if not self.tecla or apertou then
        self.indo = true
        nn.scene.switch(self.cena, nil, { fade = self.escurecer > 0 and self.escurecer or nil })
        return
      end
    end
  end
end

return B
