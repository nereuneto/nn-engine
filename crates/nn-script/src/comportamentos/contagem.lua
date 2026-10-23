-- Contagem regressiva: o tempo da fase, um cronômetro de corrida, "sobreviva
-- 60 segundos". O tempo fica numa variável do jogo (para o HUD mostrar) e,
-- ao acabar, avisa os eventos e faz o que for escolhido.
local B = {
  title = "Contagem regressiva",
  description = "Conta o tempo para trás numa variável do jogo (mostre no HUD com {tempo}) e, ao chegar a zero, avisa os eventos, reinicia a cena ou vai para outra.",
  category = "Fases",
  mode = "both",
  props = {
    { "segundos", 60, label = "Segundos", min = 0, help = "De onde começa." },
    { "variavel", "tempo", label = "Variável", help = "Variável do jogo com os segundos que faltam (um número inteiro). Mudar a variável (um evento que soma 10) muda o tempo." },
    { "comecar", true, label = "Começa sozinha",
      help = "Desligado, começa com a mensagem \"comecar\" (eventos: Mandar mensagem)." },
    { "ao_acabar", "nada", label = "Ao acabar",
      options = {
        { "nada", "só avisa (condição Contagem acabar)" },
        { "reiniciar", "reinicia a cena" },
        { "cena", "vai para outra cena" },
      } },
    { "cena", "", type = "scene", label = "Cena", show_if = { "ao_acabar", "cena" } },
  },
}

function B:start()
  self.resta = self.segundos
  self.andando = self.comecar
  self.acabou = false
  self:mostrar()
end

function B:mostrar()
  if self.variavel ~= "" then
    self.escrito = math.max(0, math.ceil(self.resta))
    nn.vars[self.variavel] = self.escrito
  end
end

--- Mensagens: "comecar", "parar", "reiniciar" e "somar" (com os segundos).
function B:comecar() self.andando = true end
function B:parar() self.andando = false end
function B:reiniciar()
  self.resta, self.acabou, self.andando = self.segundos, false, true
  self:mostrar()
end
function B:somar(s)
  self.resta = self.resta + (tonumber(s) or 0)
  self:mostrar()
end

function B:update(dt)
  if not self.andando or self.acabou then return end
  -- Numa conversa, o tempo para.
  if self.scene._dlg then return end
  -- Alguém mudou a variável (um evento que soma tempo, um bônus): vale.
  local v = self.variavel ~= "" and nn.vars[self.variavel]
  if type(v) == "number" and v ~= self.escrito then self.resta = v end
  self.resta = self.resta - dt
  self:mostrar()
  if self.resta <= 0 then
    self.acabou = true
    self.obj:send("acabou_o_tempo")
    if self.ao_acabar == "reiniciar" then
      self.scene:restart({ checkpoint = true })
    elseif self.ao_acabar == "cena" and self.cena ~= "" then
      nn.scene.switch(self.cena, nil, { fade = 0.4 })
    end
  end
end

return B
