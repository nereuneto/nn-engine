-- Pontos de vida: o perigo tira, e quando acabam algo acontece.
local B = {
  title = "Vida",
  description = "Pontos de vida. O Perigo tira vida; quando acaba, a cena reinicia (ou outra coisa).",
  category = "Jogador",
  mode = "both",
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
    { "barra", false, label = "Barra de vida em cima",
      help = "Uma barrinha flutuando em cima do objeto (bom para inimigos e chefes)." },
    { "cor_barra", "#FF5A5A", type = "color", label = "Cor da barra", advanced = true,
      show_if = { "barra", true } },
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
    if self.scene._3d then
      local x, y, z = self.obj:position3d()
      self.scene:burst(x, y + 0.5, "#FF6B6B", 24, z)
    else
      local x, y = self.obj:world_transform()
      self.scene:burst(x, y, "#FF6B6B", 24)
    end
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

-- A barra de vida em cima do objeto.
function B:draw()
  if not self.barra or self.atual <= 0 then return end
  local o = self.obj
  if not o.visible and self.inv <= 0 then return end
  local w, h = 46, 7
  if self.scene._3d then
    local x0, y0, z0, x1, y1, z1 = o:bounds3d()
    local sx, sy = nn.d3.to_screen((x0 + x1) / 2, y1 + 0.35, (z0 + z1) / 2)
    if not sx then return end
    nn.screen.panel(sx - w / 2 - 2, sy - h / 2 - 2, w + 4, h + 4, { color = "#000000AA", radius = 3 })
    nn.screen.panel(sx - w / 2, sy - h / 2, w * self.atual / self.vidas, h,
      { color = self.cor_barra, radius = 2 })
  else
    local x, y = o:world_transform()
    local _, hh = o:bounds()
    local by = y + hh + 10
    nn.draw.rect(x - w / 2 - 2, by - 2, w + 4, h + 4, "#000000AA")
    nn.draw.rect(x - w / 2, by, w * self.atual / self.vidas, h, self.cor_barra)
  end
end

return B
