-- Zona de gatilho: uma área (invisível no jogo) que avisa quando alguém
-- entra ou sai dela.
local B = {
  title = "Zona de gatilho",
  description = "Uma área invisível que avisa quando o jogador entra ou sai: abre uma porta, liga uma armadilha, mostra um aviso.",
  category = "Fases",
  mode = "both",
  props = {
    { "quem", "", type = "object", label = "Quem ativa", help = "Vazio = o jogador." },
    { "alvo", "", type = "object", label = "Avisa o objeto",
      help = "Todos com esse nome recebem a mensagem. Vazio = só a própria zona (para os eventos)." },
    { "mensagem", "abrir", label = "Mensagem ao entrar",
      help = "\"abrir\" abre uma Porta 3D; a mensagem chega aos eventos (Receber mensagem) e aos scripts." },
    { "mensagem_saida", "", label = "Mensagem ao sair",
      help = "Vazio = não avisa ao sair (\"fechar\" fecha a porta de novo)." },
    { "uma_vez", false, label = "Só uma vez" },
    { "som", "", type = "sound", label = "Som ao entrar" },
    { "invisivel", true, label = "Invisível no jogo",
      help = "A forma da zona aparece no editor, mas não no jogo." },
  },
}

function B:start()
  self.dentro = {}
  self.usada = false
  if self.invisivel then self.obj.visible = false end
end

function B:avisar(msg, quem)
  if self.alvo == "" or msg == "" then return end
  for _, t in ipairs(self.scene:targets(self.alvo)) do
    t:send(msg, quem)
  end
end

function B:fixed_update(dt)
  if self.usada then return end
  local agora = {}
  for _, quem in ipairs(self.scene:targets(self.quem)) do
    if quem ~= self.obj and self.obj:touching(quem, 0) then
      agora[quem] = true
      if not self.dentro[quem] then
        self.dentro[quem] = true
        if self.som ~= "" then nn.audio.play(nn.assets.sound(self.som)) end
        self.obj:send("entrou", quem)
        self:avisar(self.mensagem, quem)
        if self.uma_vez then
          self.usada = true
          return
        end
      end
    end
  end
  for quem in pairs(self.dentro) do
    if not agora[quem] then
      self.dentro[quem] = nil
      self.obj:send("saiu", quem)
      self:avisar(self.mensagem_saida, quem)
    end
  end
end

return B
