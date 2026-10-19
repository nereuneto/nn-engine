-- Diálogo: um personagem que conversa (um arquivo .dialogo, montado no
-- editor de conversa do NN Studio).
local B = {
  title = "Diálogo",
  description = "Um personagem que conversa: perto dele aparece \"E  falar\" e a conversa (um arquivo .dialogo) abre numa caixa de fala.",
  category = "Fases",
  mode = "both",
  props = {
    { "arquivo", "", type = "dialogue", label = "Conversa",
      help = "Um arquivo .dialogo (Arquivos > ➕ Novo > Diálogo), feito no editor de conversa." },
    { "comecar", "tecla", label = "Começa",
      options = { { "tecla", "com a tecla E, perto dele" }, { "chegar", "quando o jogador chega perto" },
        { "cena", "quando a cena começa" }, { "mensagem", "só com a mensagem \"conversar\"" } } },
    { "texto", "falar", label = "Aviso", show_if = { "comecar", "tecla" },
      help = "Aparece depois da tecla: \"E  falar\"." },
    { "distancia", 24, default3d = 1.5, min = 0, label = "Distância", show_if = { "comecar", "tecla", "chegar" },
      help = "Até onde o jogador pode estar (pixels no 2D, metros no 3D), da borda do objeto." },
    { "uma_vez", false, label = "Só uma vez" },
    { "olhar", true, label = "Vira para o jogador", mode = "3d",
      help = "Ao começar a conversa, o personagem vira de frente para quem fala com ele." },
  },
}

function B:start()
  self.usado = false
  self.perto = false
  if self.comecar == "cena" then self.agora = true end
end

--- Começa a conversa (também pela mensagem "conversar").
function B:conversar(quem)
  if self.arquivo == "" or (self.uma_vez and self.usado) then return false end
  if type(quem) ~= "table" then quem = self.scene:targets("")[1] end
  if not self.scene:start_dialogue(self.arquivo, self.obj) then return false end
  self.usado = true
  if self.olhar and self.scene._3d and quem and quem.alive then
    local x, _, z = self.obj:position3d()
    local qx, _, qz = quem:position3d()
    if math.abs(qx - x) + math.abs(qz - z) > 0.01 then
      self.obj.ry = math.deg(math.atan(qx - x, qz - z))
    end
  end
  return true
end

function B:update(dt)
  if self.agora then
    self.agora = false
    self:conversar()
    return
  end
  if self.arquivo == "" or (self.uma_vez and self.usado) then return end
  if self.comecar == "tecla" then
    local quem = self.scene:interaction(self, "", self.distancia, self.texto)
    if quem then self:conversar(quem) end
  elseif self.comecar == "chegar" then
    local quem, perto
    for _, p in ipairs(self.scene:targets("")) do
      if self.scene:gap(self.obj, p) <= self.distancia then quem, perto = p, true end
    end
    if perto and not self.perto and not self.scene:dialogue() then self:conversar(quem) end
    self.perto = perto or false
  end
end

return B
