-- Anda de um lado para o outro, virando na parede e na beirada.
local B = {
  title = "Patrulha",
  description = "Anda para um lado até bater numa parede (ou chegar na beirada) e volta.",
  category = "Inimigos",
  requires = { "body" },
  props = {
    { "velocidade", 120, label = "Velocidade", min = 0 },
    { "direcao", "direita", label = "Começa indo para",
      options = { { "direita", "a direita" }, { "esquerda", "a esquerda" } } },
    { "beirada", true, label = "Vira na beirada", help = "Não cai das plataformas." },
    { "espelhar", true, label = "Vira a imagem", help = "A imagem olha para onde está indo." },
  },
}

function B:start()
  self.lado = self.direcao == "esquerda" and -1 or 1
end

function B:fixed_update(dt)
  local o = self.obj
  local corpo = o.body
  if not (corpo and corpo.alive) then return end
  if self.lado < 0 and corpo.touching_left then
    self.lado = 1
  elseif self.lado > 0 and corpo.touching_right then
    self.lado = -1
  end
  local mundo = self.scene.world
  if self.beirada and corpo.on_ground and mundo then
    -- Procura chão logo à frente dos pés.
    local hw, hh = o:bounds()
    local x = corpo.x + self.lado * (hw + 2)
    local pe = corpo.y - hh
    if not mundo:raycast(x, pe + 2, 0, -1, 14) then self.lado = -self.lado end
  end
  corpo.vx = self.lado * self.velocidade
  if self.espelhar and o.sprite then o.sprite.flip_x = self.lado < 0 end
end

return B
