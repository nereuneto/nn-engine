-- Pontos de vida: o perigo tira, e quando acabam algo acontece.
local B = {
  title = "Vida",
  description = "Pontos de vida. O Perigo tira vida; quando acaba, a cena reinicia (ou outra coisa).",
  category = "Jogador",
  props = {
    { "vidas", 3, label = "Vidas", min = 1, integer = true },
    { "variavel", "vidas", label = "Guardar na variável",
      help = "Variável do jogo (nn.vars), para mostrar na tela e passar de uma cena para outra. Vazio = não guarda." },
    { "invencivel", 1, label = "Invencível depois do dano", min = 0, help = "Segundos." },
    { "ao_acabar", "reiniciar", label = "Quando acabar",
      options = {
        { "reiniciar", "reinicia a cena (e as variáveis do jogo)" },
        { "checkpoint", "volta ao checkpoint" },
        { "cena", "vai para outra cena" },
        { "destruir", "some (para inimigos)" },
        { "nada", "nada (um evento decide)" },
      } },
    { "cena", "", type = "scene", label = "Cena", show_if = { "ao_acabar", "cena" } },
    { "piscar", true, label = "Pisca quando invencível", advanced = true },
  },
}

function B:start()
  self.atual = self.vidas
  if self.variavel ~= "" then
    local v = tonumber(nn.vars[self.variavel])
    if v and v > 0 then self.atual = v else nn.vars[self.variavel] = self.vidas end
  end
  self.inv = 0
end

function B:update(dt)
  if self.inv > 0 then
    self.inv = self.inv - dt
    if self.piscar then self.obj.visible = self.inv <= 0 or math.floor(self.inv * 15) % 2 == 0 end
  end
end

--- Tira `n` vidas. Devolve false se estava invencível.
function B:dano(n)
  if self.inv > 0 or self.atual <= 0 then return false end
  self:mudar(self.atual - (n or 1))
  self.obj:send("levou_dano", n or 1)
  if self.atual <= 0 then
    self:acabou()
  else
    self.inv = self.invencivel
  end
  return true
end

--- Devolve `n` vidas (até o máximo).
function B:curar(n)
  self:mudar(math.min(self.vidas, self.atual + (n or 1)))
  return true
end

function B:mudar(v)
  self.atual = v
  if self.variavel ~= "" then nn.vars[self.variavel] = v end
end

function B:acabou()
  self.obj.visible = true
  -- Para os eventos ("Quando a vida de ... acabar").
  self.obj:send("vida_acabou")
  if self.ao_acabar == "nada" then
    return
  elseif self.ao_acabar == "destruir" then
    local x, y = self.obj:world_transform()
    self.scene:burst(x, y, "#FF6B6B", 24)
    self.obj:destroy()
  elseif self.ao_acabar == "cena" and self.cena ~= "" then
    nn.scene.switch(self.cena, nil, { fade = 0.6 })
  elseif self.ao_acabar == "checkpoint" then
    self:mudar(self.vidas)
    self.scene:restart({ checkpoint = true })
  else
    nn.reset_vars()
    self.scene:restart()
  end
end

return B
