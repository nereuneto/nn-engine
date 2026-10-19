-- Cubos 3D: um campo low-poly com árvores, casas, pedras e um herói, sob
-- um céu em degradê com sol e sombras, e com física: os cubos caem e
-- empilham, o herói empurra caixotes e não atravessa casas nem árvores.
--
--   WASD / setas       andar (para onde a câmera olha)
--   espaço             pular
--   arrastar o mouse   girar a câmera
--   rodinha            aproximar e afastar
--   clique             soltar um cubo (ou um caixote) onde o mouse aponta
--   N                  liga e desliga a névoa
--   L                  liga e desliga as sombras
--   C                  mostra as formas de colisão
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
    if (math.abs(x) > 5 or math.abs(z) > 5) and not (x > 4 and x < 14 and z > 2 and z < 12) then
      table.insert(self.arvores, {
        x = x, z = z, tamanho = sorteio(0.9, 1.6),
        cor = FOLHAS[math.floor(sorteio(1, #FOLHAS + 0.99))], giro = sorteio(0, 360),
      })
    end
  end
  self.casas = { { x = 7, z = 5, giro = -30 }, { x = 11.5, z = 8.5, giro = -60 } }
  for _ = 1, 25 do
    local x, z, s, giro = sorteio(-30, 30), sorteio(-30, 30), sorteio(0.4, 1.2), sorteio(0, 360)
    if math.abs(x) > 5 or math.abs(z) > 5 then -- o começo fica livre
      table.insert(self.pedras, { x = x, z = z, s = s, giro = giro })
    end
  end

  -- A física: o chão, o cenário parado e o herói.
  self.mundo = nn.physics3d.world({ gravity = -20 })
  self.mundo:body({ type = "static", w = 64, h = 1, d = 64, y = -0.5, tag = "chao" })
  for _, a in ipairs(self.arvores) do
    -- Só o tronco bloqueia (dá para passar por baixo dos galhos).
    self.mundo:body({ type = "static", shape = "cylinder", radius = 0.22 * a.tamanho,
      height = 1.6 * a.tamanho, x = a.x, y = 0.8 * a.tamanho, z = a.z })
  end
  for _, p in ipairs(self.pedras) do
    -- A caixa em volta do modelo, do tamanho em que ele é desenhado.
    self.mundo:body({ type = "static", model = self.pedra, scale = p.s * 1.6,
      x = p.x, z = p.z, ry = p.giro })
  end
  for _, c in ipairs(self.casas) do
    -- A forma exata, triângulo por triângulo (dá para subir no telhado!).
    self.mundo:body({ type = "static", model = self.casa, shape = "mesh", x = c.x, z = c.z, ry = c.giro })
  end
  self.mundo:body({ type = "static", w = 2.2, h = 1.6, d = 2.2, x = 0, y = 0.8, z = -8 }) -- a pirâmide

  -- Uma pilha de caixotes para derrubar.
  self.cubos = {}
  for andar = 0, 2 do
    for i = 0, 2 - andar do
      self:soltar(-4 + i * 1.02 + andar * 0.51, 0.5 + andar * 1.01, 2.5, true)
    end
  end

  -- O herói é um personagem: anda, pula, sobe degraus e empurra o que é solto.
  self.heroi_corpo = self.mundo:character({ x = 0, z = 0, radius = 0.35, height = 1.9, tag = "heroi" })
  self.jogador = { direcao = 0 }
  self.camera = { yaw = 30, pitch = 28, distancia = 12 }
  self.nevoa = true
  self.sombras = true
  -- Céu em degradê, com o disco do sol na direção da luz.
  nn.d3.sky({ top = "#3F8FD8", horizon = "#CDEBF5", bottom = "#9AA79A", sun_size = 4 })
  nn.d3.shadows({ distance = 25, quality = "media" })
  self:luz()
end

-- Um cubo colorido (ou um caixote) que cai e rola de verdade.
function cena:soltar(x, y, z, caixote)
  local cor = CORES[#self.cubos % #CORES + 1]
  local corpo
  if caixote then
    corpo = self.mundo:body({ model = self.caixote, x = x, y = y - 0.5, z = z, density = 0.6 })
  else
    corpo = self.mundo:body({ size = 1, x = x, y = y, z = z, rx = sorteio(0, 40), ry = sorteio(0, 90) })
  end
  table.insert(self.cubos, { corpo = corpo, cor = cor, caixote = caixote })
end

function cena:fixed_update(dt)
  self.mundo:step(dt)
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
  local vx, vz = 0, 0
  if ix ~= 0 or iy ~= 0 then
    local yaw = math.rad(self.camera.yaw)
    local fx, fz = -math.sin(yaw), -math.cos(yaw) -- frente
    local rx, rz = math.cos(yaw), -math.sin(yaw)  -- direita
    vx = rx * ix + fx * iy
    vz = rz * ix + fz * iy
    local n = math.sqrt(vx * vx + vz * vz)
    vx, vz = vx / n * 6, vz / n * 6
    self.jogador.direcao = math.deg(math.atan(vx, vz))
  end
  self.heroi_corpo:walk(vx, vz)
  if nn.input.pressed("space") then self.heroi_corpo:jump(8) end

  -- Clique (sem arrastar): um cubo cai onde o mouse aponta (no chão, num
  -- telhado ou em cima de outro cubo).
  if nn.input.mouse_released("left") and not self.arrastou and mx then
    local ox, oy, oz, dx, dy, dz = nn.d3.ray(mx, my)
    local hit = self.mundo:raycast(ox, oy, oz, dx, dy, dz, 200, self.heroi_corpo)
    if hit then
      self:soltar(hit.x, hit.y + 5, hit.z, #self.cubos % 3 == 2) -- um em cada três é um caixote
    end
  end

  if nn.input.pressed("c") then self.colisoes = not self.colisoes end
  if nn.input.pressed("n") then
    self.nevoa = not self.nevoa
    self:luz()
  end
  if nn.input.pressed("l") then
    self.sombras = not self.sombras
    nn.d3.shadows(self.sombras)
  end
  if nn.input.pressed("escape") then nn.quit() end

  local h = self.heroi_corpo
  nn.d3.orbit({
    target = { h.x, h.y + 1, h.z },
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

  -- Os cubos e caixotes vão para onde a física manda (posição e rotação).
  for _, c in ipairs(self.cubos) do
    if c.caixote then
      nn.d3.draw(self.caixote, { body = c.corpo })
    else
      nn.d3.draw(self.cubo, { body = c.corpo, color = c.cor })
    end
  end

  -- O herói (virado para +Z no modelo) balança enquanto anda no chão.
  local h = self.heroi_corpo
  local andando = (h.vx ~= 0 or h.vz ~= 0) and h.on_ground
  local pulo = andando and math.abs(math.sin(t * 10)) * 0.08 or 0
  nn.d3.draw(self.heroi, { x = h.x, y = h.y + pulo, z = h.z, scale = 1.2, ry = self.jogador.direcao })

  if self.colisoes then self.mundo:debug_draw() end

  -- HUD: o 2D fica por cima do 3D.
  nn.screen.panel(16, 16, 450, 176, { color = "#00000088", radius = 10 })
  nn.screen.text("Cubos 3D", 32, 28, { size = 28, color = "#FFFFFF" })
  nn.screen.text("WASD anda · espaço pula · arraste gira a câmera", 32, 68, { size = 16, color = "#E0E6EE" })
  nn.screen.text("clique solta um cubo · rodinha aproxima", 32, 90, { size = 16, color = "#E0E6EE" })
  nn.screen.text("N névoa · L sombras · C colisões", 32, 112, { size = 16, color = "#E0E6EE" })
  nn.screen.text(("névoa %s · sombras %s · colisões %s"):format(self.nevoa and "sim" or "não",
    self.sombras and "sim" or "não", self.colisoes and "sim" or "não"), 32, 134, { size = 16, color = "#E0E6EE" })
  nn.screen.text(("%d cubos · %d corpos · %d malhas"):format(#self.cubos, self.mundo.count, nn.d3.count()),
    32, 160, { size = 14, color = "#FFD166" })

  -- Um rótulo 2D preso a um ponto 3D, em cima da pirâmide.
  local sx, sy = nn.d3.to_screen(0, 3.6, -8)
  if sx then
    nn.screen.text("pirâmide feita à mão", sx, sy, { size = 16, color = "#FFFFFF", anchor = "center" })
  end
end

return cena
