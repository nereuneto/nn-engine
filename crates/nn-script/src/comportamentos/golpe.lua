-- Golpe: ataque de perto (espada, soco, mordida). Toca a animação de golpe
-- e, no instante do acerto, machuca quem está na frente.
local B = {
  title = "Golpe",
  description = "Ataque de perto: toca a animação do golpe e acerta quem está na frente (tira vida e empurra). O jogador golpeia com o mouse ou uma tecla; um inimigo, sozinho, quando o alvo chega perto.",
  category = "Ações",
  mode = "3d",
  props = {
    { "quando", "tecla", label = "Golpeia",
      options = {
        { "tecla", "ao apertar (para o jogador)" },
        { "perto", "sozinho, quando o alvo chega perto (para inimigos)" },
      } },
    { "tecla", "clique", label = "Tecla", show_if = { "quando", "tecla" },
      help = "\"clique\" = o botão esquerdo do mouse (e a tecla J); \"clique_direito\", o direito; ou o nome de uma tecla (f, space...). No controle, o X." },
    { "distancia", 1.3, label = "Golpeia com o alvo a", min = 0, show_if = { "quando", "perto" },
      help = "Metros. Ele para, vira para o alvo e golpeia." },
    { "alvo", "", type = "object", label = "Acerta", allow_empty = "os inimigos",
      help = "Vazio: o jogador acerta quem tem Vida (ou é um Inimigo 3D); um inimigo acerta o jogador." },
    { "dano", 1, label = "Dano", min = 0 },
    { "alcance", 1.6, label = "Alcance", min = 0.1, help = "Metros à frente." },
    { "angulo", 120, label = "Largura do golpe", min = 10, max = 360,
      help = "Graus: 120 pega quem está na frente e um pouco dos lados; 360, tudo em volta." },
    { "todos", true, label = "Acerta todos na frente",
      help = "Desligado: só o mais perto." },
    { "empurrao", 6, label = "Empurrão", min = 0, help = "Metros por segundo." },
    { "intervalo", 0.45, label = "Intervalo", min = 0, help = "Segundos entre um golpe e o outro." },
    { "animacao", "golpe", type = "anim", label = "Animação", allow_empty = "nenhuma" },
    { "acerto", 0.22, label = "Acerta depois de", min = 0, advanced = true,
      help = "Segundos depois de começar o golpe: o instante em que a arma passa na frente (0,22 no herói; 0,3 no besouro)." },
    { "parar", true, label = "Para de andar golpeando", advanced = true },
    { "som", "", type = "sound", label = "Som do golpe" },
    { "som_acerto", "", type = "sound", label = "Som ao acertar", advanced = true },
    { "faiscas", true, label = "Faíscas ao acertar", advanced = true },
  },
}

-- Toca um som saindo do objeto.
local function tocar(o, som)
  local x, y, z = o:position3d()
  nn.audio.play(nn.assets.sound(som), { x = x, y = y + 1, z = z, distance = 25 })
end

function B:start()
  if not self.scene._3d then
    error(("objeto \"%s\": o Golpe só funciona numa cena 3D"):format(self.obj.name), 0)
  end
  self.espera = 0
  self.tempo = nil
end

-- O clique vale? (Não vale o que prende o mouse na janela.)
local function clique(scene, botao)
  if not nn.input.mouse_pressed(botao) then return false end
  if scene._mouse_preso_em == nn.time.frame() then return false end
  local cam = scene._player_cam
  if cam and cam.prender_mouse and not nn.window.mouse_captured() then return false end
  return true
end

function B:apertou()
  local t = self.tecla
  if nn.input.pad_pressed("x") then return true end
  if t == "clique" or t == "" then
    return clique(self.scene, "left") or nn.input.pressed("j")
  elseif t == "clique_direito" then
    return clique(self.scene, "right")
  end
  return nn.input.pressed(t)
end

-- A frente do objeto no chão (o +Z do modelo, girado com ele).
function B:frente()
  local m = self.obj:matrix3d()
  local x0, _, z0 = m:position()
  local x1, _, z1 = m:transform_point(0, 0, 1)
  local dx, dz = x1 - x0, z1 - z0
  local n = math.sqrt(dx * dx + dz * dz)
  if n < 1e-3 then return 0, 1 end
  return dx / n, dz / n
end

-- O alvo mais perto e a distância (para o modo "perto").
function B:mais_perto()
  local x, _, z = self.obj:position3d()
  local melhor, dist
  for _, a in ipairs(self.scene:foes(self.obj, self.alvo)) do
    local ax, _, az = a:position3d()
    local d = math.sqrt((ax - x) ^ 2 + (az - z) ^ 2) - self.raio_de(a)
    if not dist or d < dist then melhor, dist = a, d end
  end
  return melhor, dist
end

function B.raio_de(o)
  local x0, _, z0, x1, _, z1 = o:bounds3d()
  return math.max(x1 - x0, z1 - z0) / 2
end

--- Começa um golpe agora (se não estiver golpeando). Devolve true se começou.
function B:golpear()
  if self.tempo or self.espera > 0 then return false end
  self.tempo = 0
  self.ja_acertou = false
  self.espera = self.intervalo
  -- O Jogador 3D e o Inimigo 3D param (e não trocam a animação) enquanto isso.
  self.obj._golpeando = { parar = self.parar }
  local a = self.obj:animator()
  if a and self.animacao ~= "" and a:has(self.animacao) then
    a:play(self.animacao, { loop = false, blend = 0.08, restart = true })
    self.anim = a
    -- O golpe todo: a animação (ou, mais curta, o acerto e um pouco).
    self.duracao = math.max(a.duration, self.acerto + 0.05)
  else
    self.anim = nil
    self.duracao = self.acerto + 0.15
  end
  if self.som ~= "" then tocar(self.obj, self.som) end
  self.obj:send("golpeou")
  return true
end

function B:update(dt)
  self.espera = math.max(0, self.espera - dt)
  local scene = self.scene
  if self.tempo then
    self.tempo = self.tempo + dt
    if not self.ja_acertou and self.tempo >= self.acerto then
      self.ja_acertou = true
      self:acertar()
    end
    if self.tempo >= self.duracao then
      self.tempo = nil
      self.obj._golpeando = nil
    end
    return
  end
  if scene._dlg then return end
  if self.quando == "tecla" then
    if self:apertou() then self:golpear() end
  elseif self.espera <= 0 and (self.obj.empurrado or 0) <= 0 then
    local a, d = self:mais_perto()
    if a and d <= self.distancia then
      -- Vira para o alvo e golpeia.
      local x, _, z = self.obj:position3d()
      local ax, _, az = a:position3d()
      self.obj.ry = math.deg(math.atan(ax - x, az - z))
      self:golpear()
    end
  end
end

function B:fixed_update()
  -- Parado enquanto golpeia (o Jogador 3D e o Inimigo 3D andam antes).
  local c = self.obj.body
  if self.parar and self.tempo and c and c.alive and c.is_character and (self.obj.empurrado or 0) <= 0 then
    c:walk(0, 0)
  end
end

--- Quem está na frente agora, dentro do alcance e da largura do golpe (do
--- mais perto para o mais longe).
function B:na_frente()
  local o = self.obj
  local x, y, z = o:position3d()
  local _, y0, _, _, y1 = o:bounds3d()
  local fx, fz = self:frente()
  local meio = math.cos(math.rad(math.min(self.angulo, 360) / 2))
  local out = {}
  for _, a in ipairs(self.scene:foes(o, self.alvo)) do
    if a.alive and a ~= o then
      local ax, _, az = a:position3d()
      local _, ay0, _, _, ay1 = a:bounds3d()
      local dx, dz = ax - x, az - z
      local r = B.raio_de(a)
      local d = math.sqrt(dx * dx + dz * dz)
      local alto = ay1 >= y0 - 0.3 and ay0 <= y1 + 0.3
      local dentro = d - r <= self.alcance
      local frente = d <= r + 0.2 or self.angulo >= 360 or (dx * fx + dz * fz) / math.max(d, 1e-6) >= meio
      if alto and dentro and frente then table.insert(out, { a, d }) end
    end
  end
  table.sort(out, function(p, q) return p[2] < q[2] end)
  local lista = {}
  for i, p in ipairs(out) do
    lista[i] = p[1]
    if not self.todos then break end
  end
  return lista, y
end

-- O instante do acerto: tira vida de quem está na frente.
function B:acertar()
  local o = self.obj
  local lista = self:na_frente()
  local fx, fz = self:frente()
  local algum = false
  for _, a in ipairs(lista) do
    local ok = self.scene:hit(a, self.dano, o, { push = self.empurrao, dx = fx, dz = fz, up = 0.45 })
    if ok then
      algum = true
      if self.faiscas then
        local ax, ay, az = a:position3d()
        local _, _, _, _, ay1 = a:bounds3d()
        self.scene:burst(ax - fx * 0.3, (ay + ay1) / 2, "#FFFFFF", 10, az - fz * 0.3)
      end
    end
  end
  if algum and self.som_acerto ~= "" then tocar(o, self.som_acerto) end
  return algum
end

function B:destroyed()
  self.obj._golpeando = nil
end

return B
