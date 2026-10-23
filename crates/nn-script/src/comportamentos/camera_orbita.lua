-- Câmera que gira em volta: telas de título e de menu em 3D, a vitrine de um
-- personagem, o fim da fase. Ponha num objeto com Câmera 3D.
local B = {
  title = "Câmera girando em volta",
  description = "A câmera dá voltas devagar em volta de um objeto (ou do meio da cena), olhando para ele: telas de título e de menu em 3D.",
  category = "Câmera",
  mode = "3d",
  requires = { "camera3d" },
  props = {
    { "alvo", "", type = "object", label = "Em volta de", allow_empty = "o meio da cena (0, 0, 0)" },
    { "raio", 12, label = "Distância", min = 0.5, help = "Metros até o alvo, no chão." },
    { "altura", 5, label = "Altura", help = "Metros acima do alvo." },
    { "velocidade", 8, label = "Graus por segundo", help = "Negativo gira para o outro lado. 0 = parada." },
    { "olhar_altura", 1, label = "Olha para a altura", advanced = true,
      help = "Metros acima do pé do alvo para onde a câmera olha." },
  },
}

function B:start()
  -- Começa de onde a câmera foi posta no editor (o ângulo em volta do alvo).
  local cx, _, cz = self:centro()
  local x, _, z = self.obj:position3d()
  self.angulo = math.deg(math.atan(x - cx, z - cz))
  self:posicionar()
end

function B:centro()
  if self.alvo ~= "" then
    local o = self.scene:find(self.alvo)
    if o and o.alive then return o:position3d() end
  end
  return 0, 0, 0
end

function B:posicionar()
  local cx, cy, cz = self:centro()
  local a = math.rad(self.angulo)
  local x, y, z = cx + math.sin(a) * self.raio, cy + self.altura, cz + math.cos(a) * self.raio
  local o = self.obj
  o.x, o.y, o.z = x, y, z
  -- A câmera olha para -Z do objeto: vira para o alvo.
  local dx, dy, dz = cx - x, cy + self.olhar_altura - y, cz - z
  o.ry = math.deg(math.atan(-dx, -dz))
  o.rx = math.deg(math.atan(dy, math.sqrt(dx * dx + dz * dz)))
  o.rz = 0
end

function B:update(dt)
  self.angulo = (self.angulo + self.velocidade * dt) % 360
  self:posicionar()
end

return B
