-- Interagir: perto do objeto aparece "E  abrir"; apertando, ele avisa.
local B = {
  title = "Interagir",
  description = "Perto dele aparece \"E  ler\" (ou abrir, falar...); apertando E (ou o Y do controle), ele avisa os eventos e outros objetos.",
  category = "Fases",
  mode = "both",
  props = {
    { "texto", "usar", label = "O que faz", help = "Aparece depois da tecla: \"E  abrir\"." },
    { "distancia", 24, default3d = 1.2, min = 0, label = "Distância",
      help = "Até onde o jogador pode estar (pixels no 2D, metros no 3D), da borda do objeto." },
    { "quem", "", type = "object", label = "Quem usa", help = "Vazio = o jogador." },
    { "alvo", "", type = "object", label = "Avisa o objeto",
      help = "Todos com esse nome recebem a mensagem. Vazio = só este objeto (\"interagiu\")." },
    { "mensagem", "abrir", label = "Mensagem" },
    { "item", "", label = "Precisa do item",
      help = "Um item do inventário (\"chave\"). Vazio = não precisa de nada." },
    { "gastar", true, label = "Gasta o item", help = "O item sai do inventário ao usar." },
    { "texto_sem_item", "Precisa de: {item}", label = "Aviso sem o item", advanced = true },
    { "uma_vez", false, label = "Só uma vez" },
    { "som", "", type = "sound", label = "Som" },
  },
}

function B:start()
  self.usado = false
end

function B:aviso()
  if self.item ~= "" and not nn.inventory.has(self.item) then
    return (self.texto_sem_item:gsub("{item}", self.item)), true
  end
  return self.texto, false
end

function B:update(dt)
  if self.usado then return end
  local texto, falta = self:aviso()
  local quem = self.scene:interaction(self, self.quem, self.distancia, texto, falta)
  if not quem then return end
  if falta then
    self.obj:send("falta_item", quem, self.item)
    return
  end
  if self.item ~= "" and self.gastar then nn.inventory.remove(self.item) end
  if self.som ~= "" then nn.audio.play(nn.assets.sound(self.som)) end
  if self.uma_vez then self.usado = true end
  self.obj:send("interagiu", quem)
  if self.alvo ~= "" and self.mensagem ~= "" then
    for _, t in ipairs(self.scene:targets(self.alvo)) do
      t:send(self.mensagem, quem)
    end
  end
end

return B
