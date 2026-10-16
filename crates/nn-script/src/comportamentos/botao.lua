-- Botão de menu: destaca com o mouse ou com as setas e responde ao clique,
-- ao Enter e ao A do controle.
local B = {
  title = "Botão",
  description = "Para menus: destaca ao passar o mouse (ou com as setas) e faz algo ao clicar ou apertar Enter.",
  category = "Interface",
  props = {
    { "acao", "nada", label = "Ao clicar",
      options = {
        { "nada", "nada (um evento decide)" },
        { "cena", "ir para outra cena" },
        { "por_cima", "abrir uma cena por cima (pausa)" },
        { "fechar", "fechar esta cena (a aberta por cima)" },
        { "reiniciar", "reiniciar a cena" },
        { "sair", "fechar o jogo" },
      } },
    { "cena", "", type = "scene", label = "Cena", show_if = { "acao", "cena", "por_cima" } },
    { "foco", false, label = "Começa selecionado",
      help = "O botão que já vem destacado para quem usa teclado ou controle." },
    { "cor", "#FFD166", type = "color", label = "Cor ao destacar" },
    { "aumentar", 1.1, label = "Aumentar ao destacar", min = 1, max = 2 },
    { "som", "", type = "sound", label = "Som ao clicar", advanced = true },
  },
}

local function visual(o)
  for _, c in ipairs(o.components) do
    if c.type == "text" or c.type == "sprite" or c.type == "shape" then return c end
  end
end

function B:start()
  local c = visual(self.obj)
  self.cor_original = c and c.color
  self.sx, self.sy = self.obj.scale_x, self.obj.scale_y
  -- O foco (setas, mouse, controle) é o mesmo dos botões de interface.
  self.item = self.scene:_ui_add({
    obj = self.obj, kind = "behavior",
    enabled = function() return self.enabled end,
    press = function() self:apertar() end,
    start_focus = self.foco,
  })
end

function B:update(dt)
  local state = self.scene:_ui_state(self.item)
  local on = state == "hover" or state == "press"
  local c = visual(self.obj)
  if c then c.color = on and self.cor or self.cor_original end
  local k = on and self.aumentar or 1
  self.obj.scale_x, self.obj.scale_y = self.sx * k, self.sy * k
end

function B:apertar()
  if self.som ~= "" then nn.audio.play(nn.assets.sound(self.som)) end
  self.obj:send("clicou")
  if self.acao == "cena" and self.cena ~= "" then
    nn.scene.switch(self.cena, nil, { fade = 0.4 })
  elseif self.acao == "por_cima" then
    self.scene:open_over(self.cena)
  elseif self.acao == "fechar" then
    self.scene:close_over()
  elseif self.acao == "reiniciar" then
    self.scene:restart()
  elseif self.acao == "sair" then
    nn.quit({ fade = 0.4 })
  end
end

return B
