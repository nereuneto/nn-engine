-- Espinho, lava, inimigo: machuca quem encosta.
local B = {
  title = "Perigo",
  description = "Espinho, lava, inimigo: machuca quem encosta.",
  category = "Inimigos",
  mode = "both",
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
    { "empurrao", 520, default3d = 7, label = "Empurrão", min = 0, advanced = true,
      help = "Velocidade do empurrão (no 3D, em metros por segundo)." },
  },
}

function B:fixed_update(dt)
  local margem = self.scene._3d and 0.05 or 1
  self._vy = self._vy or setmetatable({}, { __mode = "k" })
  for _, quem in ipairs(self.scene:targets(self.quem)) do
    if self.obj:touching(quem, margem) then
      self:encostou(quem)
      if not (self.obj.alive and self.enabled) then return end
    end
    self._vy[quem] = quem.body and quem.body.alive and quem.body.vy or 0
  end
end

-- Quem encostou veio de cima, caindo?
function B:pisou(quem)
  local corpo = quem.body
  if not corpo then return false end
  if self.scene._3d then
    local caindo = corpo.vy < -0.5 or ((self._vy and self._vy[quem]) or 0) < -0.5
    if not caindo then return false end
    local _, _, _, _, y1 = self.obj:bounds3d()
    local _, qy = quem:position3d()
    return qy >= y1 - 0.35
  end
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
  local d3 = self.scene._3d
  if self.pisar and self:pisou(quem) then
    quem.body.vy = d3 and 8 or 640
    if self.obj:send("dano", 1, quem) == nil then
      if d3 then
        local x, y, z = self.obj:position3d()
        self.scene:burst(x, y + 0.3, "#FFFFFF", 16, z)
      else
        local x, y = self.obj:world_transform()
        self.scene:burst(x, y)
      end
      self.obj:destroy()
    end
    return
  end
  if self.acao == "reiniciar" then
    self.enabled = false
    self.scene:restart({ checkpoint = true })
  elseif self.acao == "checkpoint" then
    local x, y, z = self.scene:checkpoint()
    if not x then x, y, z = quem.start_x, quem.start_y, quem.start_z end
    quem:move_to(x, y, z)
  else
    local levou = quem:send("dano", self.dano, self.obj)
    if levou == nil then
      -- Quem encostou não tem Vida: reinicia a cena.
      self.enabled = false
      self.scene:restart({ checkpoint = true })
      return
    end
    if levou and quem.body and self.empurrao > 0 then
      if d3 then
        -- Para longe do perigo, e um pouco para cima.
        local x, _, z = self.obj:position3d()
        local qx, _, qz = quem:position3d()
        local dx, dz = qx - x, qz - z
        local n = math.sqrt(dx * dx + dz * dz)
        if n < 1e-3 then dx, dz, n = 0, 1, 1 end
        quem.body:set_velocity(dx / n * self.empurrao, self.empurrao * 0.7, dz / n * self.empurrao)
        quem.empurrado = 0.35
      else
        local x = self.obj:world_transform()
        local qx = quem:world_transform()
        quem.body.vx = (qx >= x and 1 or -1) * self.empurrao
        quem.body.vy = self.empurrao * 0.7
        quem.empurrado = 0.25
      end
    end
  end
  if self.some then self.obj:destroy() end
end

return B
