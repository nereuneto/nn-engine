-- Cubos 3D: um campo low-poly com árvores, casas, pedras e um herói, sob
-- um céu em degradê com sol e sombras.
--
--   WASD / setas       andar (para onde a câmera olha)
--   arrastar o mouse   girar a câmera
--   rodinha            aproximar e afastar
--   clique             soltar um cubo (ou um caixote) no chão
--   N                  liga e desliga a névoa
--   L                  liga e desliga as sombras
--
-- Os modelos em modelos/ são arquivos .glb (como os que o Blender exporta),
-- gerados por ferramentas/gerar_modelos.py no código da engine.

local cena = {}

local GRAMA = "#7BC86C"
local FOLHAS = { "#FFFFFF", "#E2F5D0", "#C9E8B0" } -- tons por cima do verde do modelo
local CORES = { "#FF6B6B", "#FFD166", "#06D6A0", "#118AB2", "#C77DFF" }

-- Números "aleatórios" sempre iguais, para o campo não mudar a cada vez.
local semente = 7
local function sorteio(a, b)
  semente = (semente * 1103515245 + 12345) % 2147483648
  return a + (b - a) * (semente / 2147483648)
end

function cena:enter()
  self.cubo = nn.d3.cube()
  self.chao = nn.d3.plane()
  self.arvore = nn.assets.model("modelos/arvore.glb")
  self.pedra = nn.assets.model("modelos/pedra.glb")
  self.casa = nn.assets.model("modelos/casa.glb")
  self.heroi = nn.assets.model("modelos/heroi.glb")
  self.caixote = nn.assets.model("modelos/caixote.glb")

  -- Uma pirâmide feita à mão: pontos e triângulos (anti-horário vistos de fora).
  self.piramide = nn.d3.mesh({
    points = { { -0.5, 0, -0.5 }, { 0.5, 0, -0.5 }, { 0.5, 0, 0.5 }, { -0.5, 0, 0.5 }, { 0, 1, 0 } },
    triangles = { { 4, 3, 5 }, { 3, 2, 5 }, { 2, 1, 5 }, { 1, 4, 5 }, { 1, 2, 3 }, { 1, 3, 4 } },
    colors = { "#FFD166", "#F4A259", "#FFD166", "#F4A259", "#C9853A", "#C9853A" },
  })

  self.arvores, self.pedras = {}, {}
  for _ = 1, 40 do
    local x, z = sorteio(-28, 28), sorteio(-28, 28)
    if (math.abs(x) > 4 or math.abs(z) > 4) and not (x > 4 and x < 14 and z > 2 and z < 12) then
      table.insert(self.arvores, {
        x = x, z = z, tamanho = sorteio(0.9, 1.6),
        cor = FOLHAS[math.floor(sorteio(1, #FOLHAS + 0.99))], giro = sorteio(0, 360),
      })
    end
  end
  self.casas = { { x = 7, z = 5, giro = -30 }, { x = 11.5, z = 8.5, giro = -60 } }
  for _ = 1, 25 do
    table.insert(self.pedras, {
      x = sorteio(-30, 30), z = sorteio(-30, 30), s = sorteio(0.4, 1.2), giro = sorteio(0, 360),
    })
  end

  self.cubos = {}
  self.jogador = { x = 0, z = 0, direcao = 0 }
  self.camera = { yaw = 30, pitch = 28, distancia = 12 }
  self.nevoa = true
  self.sombras = true
  -- Céu em degradê, com o disco do sol na direção da luz.
  nn.d3.sky({ top = "#3F8FD8", horizon = "#CDEBF5", bottom = "#9AA79A", sun_size = 4 })
  nn.d3.shadows({ distance = 25, quality = "media" })
  self:luz()
end

function cena:luz()
  nn.d3.light({
    sun = { -0.5, -0.8, -0.35 },
    sun_color = "#FFF1D6",
    sky = "#93AFCF",
    ground = "#6A5D48",
    -- Sem "color": a névoa usa a cor do horizonte do céu.
    fog = self.nevoa and { near = 20, far = 60 } or false,
  })
end

function cena:update(dt)
  -- Girar a câmera arrastando o mouse.
  local mx, my = nn.input.mouse()
  if mx and nn.input.mouse_down("left") and self.ultimo_mouse then
    local dx, dy = mx - self.ultimo_mouse[1], my - self.ultimo_mouse[2]
    if math.abs(dx) + math.abs(dy) > 0 then self.arrastou = true end
    self.camera.yaw = self.camera.yaw - dx * 0.3
    self.camera.pitch = math.max(5, math.min(80, self.camera.pitch + dy * 0.3))
  end
  if nn.input.mouse_pressed("left") then self.arrastou = false end
  self.ultimo_mouse = mx and { mx, my } or nil

  local _, rodinha = nn.input.scroll()
  self.camera.distancia = math.max(4, math.min(40, self.camera.distancia - rodinha * 1.5))

  -- Andar em relação à câmera: "frente" é para onde ela olha.
  local ix, iy = nn.input.wasd()
  if ix ~= 0 or iy ~= 0 then
    local yaw = math.rad(self.camera.yaw)
    local fx, fz = -math.sin(yaw), -math.cos(yaw) -- frente
    local rx, rz = math.cos(yaw), -math.sin(yaw)  -- direita
    local vx = rx * ix + fx * iy
    local vz = rz * ix + fz * iy
    local n = math.sqrt(vx * vx + vz * vz)
    local j = self.jogador
    j.x = math.max(-30, math.min(30, j.x + vx / n * 6 * dt))
    j.z = math.max(-30, math.min(30, j.z + vz / n * 6 * dt))
    j.direcao = math.deg(math.atan(vx, vz))
  end

  -- Clique (sem arrastar): um cubo colorido cai onde o mouse toca o chão.
  if nn.input.mouse_released("left") and not self.arrastou and mx then
    local x, _, z = nn.d3.ground_point(mx, my)
    if x then
      table.insert(self.cubos, {
        x = x, y = 6, z = z, vy = 0, giro = sorteio(0, 90),
        cor = CORES[#self.cubos % #CORES + 1],
        caixote = #self.cubos % 3 == 2, -- um em cada três é um caixote
      })
    end
  end
  for _, c in ipairs(self.cubos) do
    c.vy = c.vy - 20 * dt
    c.y = math.max(0.5, c.y + c.vy * dt)
    if c.y == 0.5 then c.vy = 0 end
  end

  if nn.input.pressed("n") then
    self.nevoa = not self.nevoa
    self:luz()
  end
  if nn.input.pressed("l") then
    self.sombras = not self.sombras
    nn.d3.shadows(self.sombras)
  end
  if nn.input.pressed("escape") then nn.quit() end

  local j = self.jogador
  nn.d3.orbit({
    target = { j.x, 1, j.z },
    distance = self.camera.distancia,
    yaw = self.camera.yaw,
    pitch = self.camera.pitch,
  })
end

function cena:draw()
  local t = nn.time.elapsed()

  nn.d3.draw(self.chao, { scale = 64, color = GRAMA })

  for _, a in ipairs(self.arvores) do
    nn.d3.draw(self.arvore, { x = a.x, z = a.z, ry = a.giro, scale = a.tamanho, color = a.cor })
  end
  for _, p in ipairs(self.pedras) do
    nn.d3.draw(self.pedra, { x = p.x, z = p.z, scale = p.s * 1.6, ry = p.giro })
  end
  for _, c in ipairs(self.casas) do
    nn.d3.draw(self.casa, { x = c.x, z = c.z, ry = c.giro })
  end

  -- A pirâmide girando no meio do campo.
  nn.d3.draw(self.piramide, { x = 0, y = 0, z = -8, scale = 3, ry = t * 20 })

  for _, c in ipairs(self.cubos) do
    if c.caixote then
      nn.d3.draw(self.caixote, { x = c.x, y = c.y - 0.5, z = c.z, ry = c.giro })
    else
      nn.d3.draw(self.cubo, { x = c.x, y = c.y, z = c.z, ry = c.giro, color = c.cor })
    end
  end

  -- O herói (virado para +Z no modelo) anda balançando.
  local j = self.jogador
  local pulo = math.abs(math.sin(t * 6)) * 0.08
  nn.d3.draw(self.heroi, { x = j.x, y = pulo, z = j.z, scale = 1.2, ry = j.direcao })

  -- HUD: o 2D fica por cima do 3D.
  nn.screen.panel(16, 16, 430, 154, { color = "#00000088", radius = 10 })
  nn.screen.text("Cubos 3D", 32, 28, { size = 28, color = "#FFFFFF" })
  nn.screen.text("WASD anda · arraste gira · rodinha aproxima", 32, 68, { size = 16, color = "#E0E6EE" })
  nn.screen.text("clique solta um cubo · N névoa · L sombras", 32, 90, { size = 16, color = "#E0E6EE" })
  nn.screen.text(("névoa %s · sombras %s"):format(self.nevoa and "sim" or "não", self.sombras and "sim" or "não"),
    32, 112, { size = 16, color = "#E0E6EE" })
  nn.screen.text(("%d cubos · %d malhas"):format(#self.cubos, nn.d3.count()), 32, 138,
    { size = 14, color = "#FFD166" })

  -- Um rótulo 2D preso a um ponto 3D, em cima da pirâmide.
  local sx, sy = nn.d3.to_screen(0, 3.6, -8)
  if sx then
    nn.screen.text("pirâmide feita à mão", sx, sy, { size = 16, color = "#FFFFFF", anchor = "center" })
  end
end

return cena
