-- Porta, saída, bandeira de fim de fase.
local B = {
  title = "Ir para cena",
  description = "Porta, saída, bandeira de fim de fase: leva para outra cena.",
  category = "Fases",
  mode = "both",
  props = {
    { "cena", "", type = "scene", label = "Cena" },
    { "quem", "", type = "object", label = "Quem ativa", help = "Vazio = o jogador." },
    { "tecla", false, label = "Precisa apertar uma tecla",
      help = "Como uma porta: encoste e aperte para cima (no 3D, E ou o Y do controle)." },
    { "escurecer", 0.5, label = "Escurecer (segundos)", min = 0, advanced = true },
  },
}

function B:update(dt)
  if self.indo or self.cena == "" then return end
  for _, quem in ipairs(self.scene:targets(self.quem)) do
    if self.obj:touching(quem, 0) then
      local apertou
      if self.scene._3d then
        apertou = nn.input.pressed("e") or nn.input.pad_pressed("y")
      else
        apertou = nn.input.pressed("up") or nn.input.pressed("w") or nn.input.pad_pressed("up")
      end
      if not self.tecla or apertou then
        self.indo = true
        nn.scene.switch(self.cena, nil, { fade = self.escurecer > 0 and self.escurecer or nil })
        return
      end
    end
  end
end

return B
