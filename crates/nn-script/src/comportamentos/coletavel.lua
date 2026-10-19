-- Some ao ser tocado e soma pontos numa variável do jogo.
local B = {
  title = "Coletável",
  description = "Moeda, item, vida extra: some quando o jogador encosta e soma numa variável do jogo.",
  category = "Itens",
  mode = "both",
  props = {
    { "quem", "", type = "object", label = "Quem pega", help = "Vazio = o jogador." },
    { "variavel", "moedas", label = "Soma na variável",
      help = "Variável do jogo (nn.vars). Vazio = não soma nada." },
    { "valor", 1, label = "Quanto soma" },
    { "som", "", type = "sound", label = "Som" },
    { "brilho", true, label = "Brilho ao pegar" },
    { "cor_brilho", "#FFD166", type = "color", label = "Cor do brilho", advanced = true },
  },
}

function B:fixed_update(dt)
  for _, quem in ipairs(self.scene:targets(self.quem)) do
    if self.obj:touching(quem, 0) then
      self:pegar(quem)
      return
    end
  end
end

function B:pegar(quem)
  if self.variavel ~= "" then
    nn.vars[self.variavel] = (tonumber(nn.vars[self.variavel]) or 0) + self.valor
  end
  if self.som ~= "" then nn.audio.play(nn.assets.sound(self.som)) end
  if self.brilho and not self.scene._3d then
    local x, y = self.obj:world_transform()
    self.scene:burst(x, y, self.cor_brilho)
  end
  quem:send("coletou", self.obj)
  self.obj:destroy()
end

return B
