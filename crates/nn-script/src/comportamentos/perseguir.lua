-- Vai atrás de um alvo quando ele chega perto.
local B = {
  title = "Perseguir",
  description = "Vai atrás do alvo quando ele chega perto.",
  category = "Inimigos",
  props = {
    { "alvo", "", type = "object", label = "Alvo", help = "Vazio = o jogador." },
    { "velocidade", 150, label = "Velocidade", min = 0 },
    { "visao", 320, label = "Distância de visão", min = 0, help = "0 = sempre persegue." },
    { "voar", false, label = "Voa", help = "Anda em todas as direções, sem gravidade (também num jogo visto de cima)." },
    { "contornar", true, label = "Contorna paredes", show_if = { "voar", true },
      help = "Acha um caminho em volta das paredes e dos tiles sólidos, em vez de ir reto e ficar preso." },
    { "espelhar", true, label = "Vira a imagem" },
  },
}

function B:start()
  if self.voar and self.obj.body then self.obj.body.gravity_scale = 0 end
  self.rota = {}
  self.obj._nav_state = self.rota
end

function B:fixed_update(dt)
  local o = self.obj
  local x, y = o:world_transform()
  local melhor, dx, dy, dist
  for _, alvo in ipairs(self.scene:targets(self.alvo)) do
    local ax, ay = alvo:world_transform()
    local d = math.sqrt((ax - x) ^ 2 + (ay - y) ^ 2)
    if not dist or d < dist then melhor, dx, dy, dist = alvo, ax - x, ay - y, d end
  end
  local vx, vy = 0, 0
  if melhor and dist > 2 and (self.visao <= 0 or dist <= self.visao) then
    if self.voar then
      if self.contornar then
        local ax, ay = melhor:world_transform()
        local px, py = self.scene:steer(self.rota, x, y, ax, ay, dt)
        dx, dy = px - x, py - y
        local n = math.sqrt(dx * dx + dy * dy)
        if n > 1e-3 then dist = n end
      end
      vx, vy = dx / dist * self.velocidade, dy / dist * self.velocidade
    elseif math.abs(dx) > 4 then
      vx = (dx > 0 and 1 or -1) * self.velocidade
    end
  end
  local corpo = o.body
  if corpo and corpo.alive then
    corpo.vx = vx
    if self.voar then corpo.vy = vy end
  else
    o.x, o.y = o.x + vx * dt, o.y + vy * dt
  end
  if self.espelhar and o.sprite and vx ~= 0 then o.sprite.flip_x = vx < 0 end
end

return B
