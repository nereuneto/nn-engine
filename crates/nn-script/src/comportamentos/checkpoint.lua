-- Ponto de reinício.
local B = {
  title = "Checkpoint",
  description = "Ao passar por aqui, o jogador volta para este ponto quando perder.",
  category = "Fases",
  props = {
    { "quem", "", type = "object", label = "Quem ativa", help = "Vazio = o jogador." },
  },
}

function B:fixed_update(dt)
  if self.ativo then return end
  for _, quem in ipairs(self.scene:targets(self.quem)) do
    if self.obj:touching(quem, 0) then
      self.ativo = true
      local x, y = self.obj:world_transform()
      self.scene:set_checkpoint(x, y)
      self.scene:burst(x, y, "#7CFC9A", 20)
      self.obj:send("ativou", quem)
      return
    end
  end
end

return B
