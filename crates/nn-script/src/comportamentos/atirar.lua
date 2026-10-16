-- Cria tiros (um prefab) ao apertar uma tecla, ou sozinho.
local B = {
  title = "Atirar",
  description = "Cria um tiro (um prefab) ao apertar uma tecla, ou sozinho de tempos em tempos.",
  category = "Ações",
  props = {
    { "prefab", "", type = "prefab", label = "Tiro (prefab)" },
    { "tecla", "x", label = "Tecla", help = "Nome da tecla (x, space, z...). Vazio = atira sozinho. No controle, o botão X." },
    { "intervalo", 0.3, label = "Intervalo (segundos)", min = 0 },
    { "velocidade", 700, label = "Velocidade do tiro", min = 0 },
    { "direcao", "frente", label = "Direção",
      options = {
        { "frente", "para onde está virado" },
        { "jogador", "na direção do jogador" },
        { "direita", "direita" }, { "esquerda", "esquerda" },
        { "cima", "cima" }, { "baixo", "baixo" },
      } },
    { "som", "", type = "sound", label = "Som" },
    { "distancia", 24, label = "Sai a esta distância", min = 0, advanced = true },
  },
}

function B:start()
  self.espera = self.tecla == "" and self.intervalo or 0
end

function B:update(dt)
  self.espera = math.max(0, self.espera - dt)
  if self.prefab == "" or self.espera > 0 then return end
  local quer = self.tecla == "" or nn.input.down(self.tecla) or nn.input.pad_down("x")
  if quer then
    self.espera = self.intervalo
    self:atirar()
  end
end

function B:direcao_do_tiro()
  local d = self.direcao
  if d == "esquerda" then return -1, 0 end
  if d == "direita" then return 1, 0 end
  if d == "cima" then return 0, 1 end
  if d == "baixo" then return 0, -1 end
  if d == "jogador" then
    local x, y = self.obj:world_transform()
    local alvo = self.scene:targets("")[1]
    if alvo then
      local ax, ay = alvo:world_transform()
      local dist = math.sqrt((ax - x) ^ 2 + (ay - y) ^ 2)
      if dist > 0 then return (ax - x) / dist, (ay - y) / dist end
    end
    return 1, 0
  end
  local spr = self.obj.sprite
  return (spr and spr.flip_x) and -1 or 1, 0
end

--- Cria um tiro agora.
function B:atirar()
  local dx, dy = self:direcao_do_tiro()
  local x, y = self.obj:world_transform()
  local tiro = self.scene:spawn(self.prefab, x + dx * self.distancia, y + dy * self.distancia)
  local vx, vy = dx * self.velocidade, dy * self.velocidade
  if tiro.body and tiro.body.alive then
    tiro.body.gravity_scale = 0
    tiro.body.vx, tiro.body.vy = vx, vy
  else
    tiro.vx, tiro.vy = vx, vy
  end
  if tiro.sprite and dx < 0 then tiro.sprite.flip_x = true end
  if self.som ~= "" then nn.audio.play(nn.assets.sound(self.som)) end
  return tiro
end

return B
