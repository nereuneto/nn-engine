-- Ondas de inimigos: onda 1, 2, 3... cada uma com mais inimigos; a próxima
-- vem quando todos da anterior forem derrotados.
local B = {
  title = "Ondas de inimigos",
  description = "Cria inimigos (um prefab) em ondas: onda 1, 2, 3..., cada uma com mais. A próxima vem quando todos da anterior forem derrotados. Os filhos deste objeto são os lugares de onde eles saem (sem filhos, em volta dele, no chão).",
  category = "Inimigos",
  mode = "3d",
  props = {
    { "prefab", "", type = "prefab", label = "Inimigo (prefab)" },
    { "prefab_forte", "", type = "prefab", label = "Inimigo forte (prefab)",
      help = "Opcional: a partir da onda escolhida, um em cada três é este." },
    { "forte_a_partir", 2, label = "Fortes a partir da onda", min = 1, integer = true },
    { "ondas", 3, label = "Quantas ondas", min = 1, integer = true },
    { "primeira", 3, label = "Inimigos na onda 1", min = 1, integer = true },
    { "a_mais", 2, label = "A mais em cada onda", min = 0, integer = true },
    { "quando", "chegar", label = "Começa",
      options = {
        { "chegar", "quando o jogador chega perto" },
        { "cena", "quando a cena começa" },
        { "mensagem", "com a mensagem \"comecar\"" },
      } },
    { "distancia", 6, label = "Perto (metros)", min = 0, show_if = { "quando", "chegar" } },
    { "pausa", 3, label = "Pausa entre as ondas", min = 0, help = "Segundos." },
    { "intervalo", 0.5, label = "Um inimigo a cada", min = 0, advanced = true,
      help = "Segundos entre um inimigo e o próximo da mesma onda." },
    { "raio", 8, label = "Espalhar até", min = 0,
      help = "Sem filhos: metros em volta deste objeto (no chão onde dá para andar). Com filhos, sai de um deles." },
    { "longe", 3, label = "Longe do jogador", min = 0, advanced = true,
      help = "Metros: nenhum inimigo aparece colado no jogador." },
    { "letreiro", true, label = "Letreiro \"Onda 1\" na tela" },
    { "texto", "Onda {onda}", label = "Texto do letreiro", show_if = { "letreiro", true },
      help = "{onda} vira o número; {ondas}, quantas são." },
    { "texto_fim", "Você venceu!", label = "Texto no fim", show_if = { "letreiro", true },
      help = "Vazio = sem letreiro no fim." },
    { "var_onda", "onda", label = "Guardar a onda na variável", advanced = true,
      help = "Para mostrar na tela (Mostrar valor: \"Onda {onda}\"). Vazio = não guarda." },
    { "var_restantes", "restantes", label = "Guardar quantos faltam na variável", advanced = true },
    { "som", "", type = "sound", label = "Som de cada onda" },
    { "cor", "#B36BFF", type = "color", label = "Cor das faíscas ao surgir", advanced = true },
  },
}

function B:start()
  self.onda = 0
  self.estado = "esperando"
  self.vivos = {}
  self.fila = 0
  self.tempo = 0
  self.letreiro_t = 0
  self:guardar()
  if self.quando == "cena" then self:comecar() end
end

-- As variáveis do jogo (onda e quantos faltam).
function B:guardar()
  if self.var_onda ~= "" then nn.vars[self.var_onda] = self.onda end
  if self.var_restantes ~= "" then nn.vars[self.var_restantes] = self:restantes() end
end

--- Quantos inimigos faltam nesta onda (vivos e por surgir).
function B:restantes()
  local n = self.fila or 0
  for _, o in ipairs(self.vivos or {}) do
    if o.alive then n = n + 1 end
  end
  return n
end

--- Começa a primeira onda (é a mensagem "comecar").
function B:comecar()
  if self.estado ~= "esperando" then return end
  self.obj:send("ondas_comecaram")
  self:proxima()
end

--- Para tudo: os inimigos que faltam não vêm mais (mensagem "parar").
function B:parar()
  self.estado = "parado"
  self.fila = 0
  self:guardar()
end

function B:proxima()
  self.onda = self.onda + 1
  self.fila = self.primeira + self.a_mais * (self.onda - 1)
  self.tempo = 0
  self.estado = "surgindo"
  self:mostrar(self.texto)
  if self.som ~= "" then nn.audio.play(nn.assets.sound(self.som)) end
  self:guardar()
  self.obj:send("onda_comecou", self.onda)
end

function B:mostrar(texto)
  if not self.letreiro or texto == "" then return end
  self.letreiro_txt = texto:gsub("{onda}", tostring(self.onda)):gsub("{ondas}", tostring(self.ondas))
  self.letreiro_t = 2.2
end

-- De onde o próximo inimigo sai: um dos filhos, ou em volta, no chão.
function B:lugar()
  local scene = self.scene
  local pontos = {}
  for _, c in ipairs(self.obj.children) do
    if c.alive then pontos[#pontos + 1] = c end
  end
  if #pontos > 0 then
    -- O ponto mais longe do jogador entre dois sorteados.
    local a, b = pontos[math.random(#pontos)], pontos[math.random(#pontos)]
    local p = scene:targets("")[1]
    if p and a ~= b then
      local px, _, pz = p:position3d()
      local ax, _, az = a:position3d()
      local bx, _, bz = b:position3d()
      if (bx - px) ^ 2 + (bz - pz) ^ 2 > (ax - px) ^ 2 + (az - pz) ^ 2 then a = b end
    end
    local x, y, z = a:position3d()
    return scene:ground_spot(x, y, z, 0.8, 0)
  end
  local x, y, z = self.obj:position3d()
  return scene:ground_spot(x, y, z, self.raio, self.longe)
end

function B:surgir()
  local prefab = self.prefab
  local forte = self.prefab_forte ~= "" and self.onda >= self.forte_a_partir
  if forte and (self.fila % 3 == 1) then prefab = self.prefab_forte end
  local x, y, z = self:lugar()
  local o = self.scene:spawn(prefab, x, y, { z = z })
  -- Vira para o jogador.
  local p = self.scene:targets("")[1]
  if p then
    local px, _, pz = p:position3d()
    o.ry = math.deg(math.atan(px - x, pz - z))
  end
  self.scene:burst(x, y + 0.4, self.cor, 16, z)
  table.insert(self.vivos, o)
  self.fila = self.fila - 1
end

function B:update(dt)
  if self.letreiro_t > 0 then self.letreiro_t = self.letreiro_t - dt end
  local estado = self.estado
  if estado == "esperando" then
    if self.quando == "chegar" then
      local x, _, z = self.obj:position3d()
      for _, p in ipairs(self.scene:targets("")) do
        local px, _, pz = p:position3d()
        if (px - x) ^ 2 + (pz - z) ^ 2 <= self.distancia ^ 2 then
          self:comecar()
          break
        end
      end
    end
    return
  end
  if self.prefab == "" or estado == "parado" or estado == "fim" then return end
  if estado == "surgindo" then
    self.tempo = self.tempo - dt
    if self.tempo <= 0 and self.fila > 0 then
      self:surgir()
      self.tempo = self.intervalo
      self:guardar()
    end
    if self.fila <= 0 then self.estado = "lutando" end
  elseif estado == "lutando" then
    local antes = nn.vars[self.var_restantes]
    if self:restantes() == 0 then
      self.vivos = {}
      self.obj:send("onda_terminou", self.onda)
      if self.onda >= self.ondas then
        self.estado = "fim"
        self:mostrar(self.texto_fim)
        self.obj:send("ondas_terminaram")
      else
        self.estado = "pausa"
        self.tempo = self.pausa
      end
    end
    if self.var_restantes ~= "" and antes ~= self:restantes() then self:guardar() end
  elseif estado == "pausa" then
    self.tempo = self.tempo - dt
    if self.tempo <= 0 then self:proxima() end
  end
end

-- O letreiro: grande, no alto da tela, aparecendo e sumindo.
function B:draw()
  if self.letreiro_t <= 0 or not self.letreiro_txt then return end
  local W, H = nn.window.size()
  local t = self.letreiro_t
  local a = math.min(1, t / 0.4, (2.2 - t) / 0.25)
  local size = 64 * (1 + 0.15 * math.max(0, (t - 1.95) / 0.25))
  nn.screen.text(self.letreiro_txt, W / 2 + 3, H * 0.28 + 3,
    { size = size, color = "#000000", alpha = a * 0.6, anchor = "center" })
  nn.screen.text(self.letreiro_txt, W / 2, H * 0.28,
    { size = size, color = "#FFD166", alpha = a, anchor = "center" })
end

return B
