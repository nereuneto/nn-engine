-- Espinho, lava, inimigo: machuca quem encosta.
local B = {
  title = "Perigo",
  description = "Espinho, lava, inimigo: machuca quem encosta.",
  category = "Inimigos",
  props = {
    { "quem", "", type = "object", label = "Machuca", help = "Vazio = o jogador." },
    { "acao", "dano", label = "Ao encostar",
      options = {
        { "dano", "tira vida (comportamento Vida)" },
        { "checkpoint", "volta ao checkpoint" },
        { "reiniciar", "reinicia a cena" },
      } },
    { "dano", 1, label = "Dano", min = 0, show_if = { "acao", "dano" } },
    { "pisar", false, label = "Morre se pisarem em cima",
      help = "Como num jogo de plataforma: quem cai em cima derrota o perigo." },
    { "some", false, label = "Some ao acertar", help = "Para tiros e projéteis." },
    { "empurrao", 520, label = "Empurrão", min = 0, advanced = true },
  },
}

function B:fixed_update(dt)
  for _, quem in ipairs(self.scene:targets(self.quem)) do
    if self.obj:touching(quem, 1) then
      self:encostou(quem)
      if not (self.obj.alive and self.enabled) then return end
    end
  end
end

-- Quem encostou veio de cima, caindo?
function B:pisou(quem)
  local corpo = quem.body
  if not corpo then return false end
  -- Já está em pé em cima (a física parou a queda no passo anterior).
  local meu = self.obj.body
  if meu and corpo.ground ~= nil and corpo.ground == meu then return true end
  if corpo.vy >= 0 then return false end
  local _, y = self.obj:world_transform()
  local _, qy = quem:world_transform()
  local _, qh = quem:bounds()
  return qy - qh * 0.5 > y
end

function B:encostou(quem)
  if self.pisar and self:pisou(quem) then
    quem.body.vy = 640
    if self.obj:send("dano", 1, quem) == nil then
      local x, y = self.obj:world_transform()
      self.scene:burst(x, y, "#FFFFFF")
      self.obj:destroy()
    end
    return
  end
  if self.acao == "reiniciar" then
    self.enabled = false
    self.scene:restart({ checkpoint = true })
  elseif self.acao == "checkpoint" then
    local x, y = self.scene:checkpoint()
    if not x then x, y = quem.start_x, quem.start_y end
    quem:move_to(x, y)
  else
    local levou = quem:send("dano", self.dano, self.obj)
    if levou == nil then
      -- Quem encostou não tem Vida: reinicia a cena.
      self.enabled = false
      self.scene:restart({ checkpoint = true })
      return
    end
    if levou and quem.body and self.empurrao > 0 then
      local x = self.obj:world_transform()
      local qx = quem:world_transform()
      quem.body.vx = (qx >= x and 1 or -1) * self.empurrao
      quem.body.vy = self.empurrao * 0.7
      quem.empurrado = 0.25
    end
  end
  if self.some then self.obj:destroy() end
end

return B
