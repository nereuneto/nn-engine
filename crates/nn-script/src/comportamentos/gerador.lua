-- Cria objetos (um prefab) de tempos em tempos.
local B = {
  title = "Gerador",
  description = "Cria objetos (um prefab) de tempos em tempos: inimigos, gotas, moedas...",
  category = "Ações",
  props = {
    { "prefab", "", type = "prefab", label = "Prefab" },
    { "intervalo", 2, label = "A cada (segundos)", min = 0.05 },
    { "maximo", 10, label = "No máximo ao mesmo tempo", min = 0, integer = true, help = "0 = sem limite." },
    { "espalhar_x", 0, label = "Espalhar em X", min = 0, help = "Sorteia a posição até esta distância.", mode = "2d" },
    { "espalhar_y", 0, label = "Espalhar em Y", min = 0, mode = "2d" },
    { "raio", 0, label = "Espalhar até", min = 0, mode = "3d",
      help = "Metros em volta do Gerador: sorteia um lugar no chão onde dá para andar (0 = no próprio Gerador)." },
    { "longe", 0, label = "Longe do jogador", min = 0, mode = "3d", advanced = true,
      help = "Metros: não cria colado no jogador." },
    { "comecar_ja", false, label = "Cria um logo no começo" },
  },
}

function B:start()
  self.espera = self.comecar_ja and 0 or self.intervalo
  self.vivos = {}
end

function B:update(dt)
  if self.prefab == "" then return end
  self.espera = self.espera - dt
  if self.espera > 0 then return end
  self.espera = self.intervalo
  local vivos = {}
  for _, o in ipairs(self.vivos) do
    if o.alive then table.insert(vivos, o) end
  end
  self.vivos = vivos
  if self.maximo > 0 and #vivos >= self.maximo then return end
  if self.scene._3d then
    local x, y, z = self.obj:position3d()
    if self.raio > 0 or self.longe > 0 then
      x, y, z = self.scene:ground_spot(x, y, z, self.raio, self.longe)
    end
    table.insert(self.vivos, self.scene:spawn(self.prefab, x, y, { z = z }))
    return
  end
  local x, y = self.obj:world_transform()
  x = x + (math.random() * 2 - 1) * self.espalhar_x
  y = y + (math.random() * 2 - 1) * self.espalhar_y
  table.insert(self.vivos, self.scene:spawn(self.prefab, x, y))
end

return B
