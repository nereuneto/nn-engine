-- Cubos 3D: um campo low-poly com árvores, pedras e um herói em forma de
-- cápsula.
--
--   WASD / setas       andar (para onde a câmera olha)
--   arrastar o mouse   girar a câmera
--   rodinha            aproximar e afastar
--   clique             soltar um cubo no chão
--   N                  liga e desliga a névoa

local cena = {}

local CEU = "#9FD3E8"
local GRAMA = "#7BC86C"
local TRONCO = "#8B5A3C"
local FOLHAS = { "#3E9B4F", "#4FB06A", "#2F8A4A" }
local PEDRA = "#A3A8B4"
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
  self.cone = nn.d3.cone(7)
  self.tronco = nn.d3.cylinder(6)
  self.pedra = nn.d3.sphere(0)
  self.heroi = nn.d3.capsule(10)

  -- Uma pirâmide feita à mão: pontos e triângulos (anti-horário vistos de fora).
  self.piramide = nn.d3.mesh({
    points = { { -0.5, 0, -0.5 }, { 0.5, 0, -0.5 }, { 0.5, 0, 0.5 }, { -0.5, 0, 0.5 }, { 0, 1, 0 } },
    triangles = { { 4, 3, 5 }, { 3, 2, 5 }, { 2, 1, 5 }, { 1, 4, 5 }, { 1, 2, 3 }, { 1, 3, 4 } },
    colors = { "#FFD166", "#F4A259", "#FFD166", "#F4A259", "#C9853A", "#C9853A" },
  })

  self.arvores, self.pedras = {}, {}
  for _ = 1, 40 do
    local x, z = sorteio(-28, 28), sorteio(-28, 28)
    if math.abs(x) > 4 or math.abs(z) > 4 then
      table.insert(self.arvores, {
        x = x, z = z, altura = sorteio(1.6, 3.2),
        cor = FOLHAS[math.floor(sorteio(1, #FOLHAS + 0.99))], giro = sorteio(0, 360),
      })
    end
  end
  for _ = 1, 25 do
    table.insert(self.pedras, {
      x = sorteio(-30, 30), z = sorteio(-30, 30), s = sorteio(0.4, 1.2), giro = sorteio(0, 360),
    })
  end

  self.cubos = {}
  self.jogador = { x = 0, z = 0, direcao = 0 }
  self.camera = { yaw = 30, pitch = 28, distancia = 12 }
  self.nevoa = true
  self:luz()
end

function cena:luz()
  nn.d3.light({
    sun = { -0.5, -1, -0.3 },
    sun_color = "#FFF4DD",
    sky = "#8FA9C8",
    ground = "#5A5040",
    fog = self.nevoa and { color = CEU, near = 18, far = 55 } or false,
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
    nn.d3.draw(self.tronco, { x = a.x, y = 0.4, z = a.z, sx = 0.3, sz = 0.3, sy = 0.8, color = TRONCO })
    nn.d3.draw(self.cone, {
      x = a.x, y = 0.8 + a.altura / 2, z = a.z, ry = a.giro,
      sx = a.altura * 0.7, sz = a.altura * 0.7, sy = a.altura, color = a.cor,
    })
  end
  for _, p in ipairs(self.pedras) do
    nn.d3.draw(self.pedra, { x = p.x, y = p.s * 0.2, z = p.z, scale = p.s, sy = p.s * 0.6, ry = p.giro, color = PEDRA })
  end

  -- A pirâmide girando no meio do campo.
  nn.d3.draw(self.piramide, { x = 0, y = 0, z = -8, scale = 3, ry = t * 20 })

  for _, c in ipairs(self.cubos) do
    nn.d3.draw(self.cubo, { x = c.x, y = c.y, z = c.z, ry = c.giro, color = c.cor })
  end

  local j = self.jogador
  local pulo = math.abs(math.sin(t * 6)) * 0.08
  nn.d3.draw(self.heroi, { x = j.x, y = 1 + pulo, z = j.z, scale = 2, ry = j.direcao, color = "#FF8C42" })
  -- Os "olhos" mostram para onde ele está virado.
  local d = math.rad(j.direcao)
  for _, lado in ipairs({ -1, 1 }) do
    local ox = math.sin(d) * 0.42 + math.cos(d) * 0.13 * lado
    local oz = math.cos(d) * 0.42 - math.sin(d) * 0.13 * lado
    nn.d3.draw(self.cubo, { x = j.x + ox, y = 1.45 + pulo, z = j.z + oz, scale = 0.12, ry = j.direcao, color = "#2B2D42" })
  end

  -- HUD: o 2D fica por cima do 3D.
  nn.screen.panel(16, 16, 410, 132, { color = "#00000088", radius = 10 })
  nn.screen.text("Cubos 3D", 32, 28, { size = 28, color = "#FFFFFF" })
  nn.screen.text("WASD anda · arraste gira · rodinha aproxima", 32, 68, { size = 16, color = "#E0E6EE" })
  nn.screen.text("clique solta um cubo · N liga/desliga a névoa", 32, 90, { size = 16, color = "#E0E6EE" })
  nn.screen.text(("%d cubos · %d malhas"):format(#self.cubos, nn.d3.count()), 32, 116,
    { size = 14, color = "#FFD166" })

  -- Um rótulo 2D preso a um ponto 3D, em cima da pirâmide.
  local sx, sy = nn.d3.to_screen(0, 3.6, -8)
  if sx then
    nn.screen.text("pirâmide feita à mão", sx, sy, { size = 16, color = "#FFFFFF", anchor = "center" })
  end
end

return cena
