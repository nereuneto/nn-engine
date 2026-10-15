-- A fase: mapa feito no Tiled (fases/fase1.tmj), física e colisões.
local util = require("lib.util")

local fase = {}

local ESCALA = 3            -- tiles de 16 px viram 48 unidades
local GRAVIDADE = -2400
local VEL_ANDAR = 340
local ACEL_CHAO = 2800
local ACEL_AR = 1600
local VEL_PULO = 930
local CORTE_PULO = 0.45     -- soltar o pulo cedo deixa o salto mais baixo
local COYOTE = 0.1          -- ainda dá para pular logo depois de sair da beirada
local BUFFER = 0.12         -- apertar o pulo um pouco antes de tocar o chão também vale
local TAM_SPRITE = 48

function fase:enter()
  self.mapa = nn.tilemap.load("fases/fase1.tmj", { scale = ESCALA })
  self.mundo = nn.physics.world({ gravity = GRAVIDADE })
  self.mundo:add_tilemap(self.mapa)

  self.sheet = nn.assets.sheet("assets/imagens/personagem.png", 16, 16, { pixel_art = true })
  self.moeda_sheet = nn.assets.sheet("assets/imagens/moeda.png", 16, 16, { pixel_art = true })
  self.img_plataforma = nn.assets.texture("assets/imagens/plataforma.png", { pixel_art = true })
  self.anims = {
    parado = nn.animation({ 1, 2 }, 2),
    andando = nn.animation({ 5, 6, 7, 8 }, 12),
    pulo = nn.animation({ 3 }, 1),
  }
  self.anim = nn.animator(self.anims.parado)
  self.giro = nn.animator(nn.animation({ 1, 2, 3, 4 }, 9, "pingpong"))
  self.sons = {
    pulo = nn.assets.sound("assets/sons/pulo.wav"),
    moeda = nn.assets.sound("assets/sons/moeda.wav"),
    dano = nn.assets.sound("assets/sons/dano.wav"),
    vitoria = nn.assets.sound("assets/sons/vitoria.wav"),
  }
  util.tocar_musica()

  -- Objetos colocados no editor viram corpos.
  local inicio = self.mapa:object("inicio")
  self.inicio = { x = inicio.x, y = inicio.y }
  self.jogador = self.mundo:body({ x = inicio.x, y = inicio.y, w = 30, h = 42, tag = "jogador" })

  self.moedas = {}
  for _, o in ipairs(self.mapa:objects("moeda")) do
    local corpo = self.mundo:body({ type = "static", x = o.x, y = o.y, radius = 18, sensor = true, tag = "moeda" })
    self.moedas[corpo.id] = { corpo = corpo, x = o.x, y = o.y }
  end
  self.total = #self.mapa:objects("moeda")

  for _, o in ipairs(self.mapa:objects("espinho")) do
    self.mundo:body({ type = "static", x = o.x, y = o.y, w = o.w, h = o.h, sensor = true, tag = "espinho" })
  end
  for _, o in ipairs(self.mapa:objects("chegada")) do
    self.mundo:body({ type = "static", x = o.x, y = o.y, w = o.w, h = o.h, sensor = true, tag = "chegada" })
  end

  self.plataformas = {}
  for _, o in ipairs(self.mapa:objects("plataforma")) do
    local vel = (o.properties.velocidade or 60) * ESCALA
    local corpo = self.mundo:body({
      type = "kinematic", x = o.x, y = o.y, w = o.w, h = o.h, one_way = true, vx = vel, tag = "plataforma",
    })
    table.insert(self.plataformas, {
      corpo = corpo, x0 = o.x, distancia = (o.properties.distancia or 64) * ESCALA, vel = vel,
    })
  end

  self.pegas, self.mortes, self.tempo = 0, 0, 0
  self.coyote, self.buffer = 0, 0
  self.esquerda = false
  self.piscar = 0
  self.popups = {}
  self.venceu = false
  self.debug = false
  self.ax, self.ay = inicio.x, inicio.y
  nn.camera.set_position(inicio.x, inicio.y)
end

function fase:fixed_update(dt)
  if self.venceu then return end
  local j = self.jogador
  self.ax, self.ay = j.x, j.y

  -- Plataformas móveis vão e voltam.
  for _, p in ipairs(self.plataformas) do
    if p.corpo.x > p.x0 + p.distancia then
      p.corpo.vx = -p.vel
    elseif p.corpo.x < p.x0 - p.distancia then
      p.corpo.vx = p.vel
    end
  end

  -- Andar com aceleração (mais solta no ar).
  local dir = util.clamp(nn.input.axis("left", "right") + nn.input.axis("a", "d"), -1, 1)
  local acel = j.on_ground and ACEL_CHAO or ACEL_AR
  j.vx = util.approach(j.vx, dir * VEL_ANDAR, acel * dt)
  if dir ~= 0 then self.esquerda = dir < 0 end

  -- Pulo com tolerâncias.
  if j.on_ground then self.coyote = COYOTE else self.coyote = self.coyote - dt end
  self.buffer = self.buffer - dt
  if self.buffer > 0 and self.coyote > 0 then
    j.vy = VEL_PULO
    self.buffer, self.coyote = 0, 0
    nn.audio.play(self.sons.pulo, { volume = 0.6 })
  end

  self.tempo = self.tempo + dt
  for _, e in ipairs(self.mundo:step(dt)) do
    if e.type == "enter" and e.b == j then
      if e.a.tag == "moeda" then
        self:pegar(e.a)
      elseif e.a.tag == "espinho" then
        self:cair()
      elseif e.a.tag == "chegada" then
        self:vencer()
      end
    end
  end

  if j.y < -200 then self:cair() end
end

function fase:pegar(corpo)
  local m = self.moedas[corpo.id]
  if not m then return end
  self.moedas[corpo.id] = nil
  corpo:remove()
  self.pegas = self.pegas + 1
  table.insert(self.popups, { x = m.x, y = m.y, idade = 0 })
  nn.audio.play(self.sons.moeda, { speed = 0.95 + math.random() * 0.15, volume = 0.8 })
end

function fase:cair()
  self.mortes = self.mortes + 1
  nn.audio.play(self.sons.dano, { volume = 0.7 })
  local j = self.jogador
  j:set_position(self.inicio.x, self.inicio.y)
  j:set_velocity(0, 0)
  self.ax, self.ay = j.x, j.y
  self.piscar = 1.0
end

function fase:vencer()
  if self.venceu then return end
  self.venceu = true
  self.jogador:set_velocity(0, 0)
  nn.audio.play(self.sons.vitoria)
  nn.scene.push("cenas/vitoria.lua", {
    moedas = self.pegas, total = self.total, tempo = self.tempo, mortes = self.mortes,
  })
end

function fase:update(dt)
  if nn.input.pressed("escape") or nn.input.pressed("p") then nn.scene.push("cenas/pausa.lua") end
  if nn.input.pressed("f1") then self.debug = not self.debug end

  if nn.input.pressed("space") or nn.input.pressed("up") or nn.input.pressed("w") then
    self.buffer = BUFFER
  end
  local soltou = nn.input.released("space") or nn.input.released("up") or nn.input.released("w")
  if soltou and self.jogador.vy > 0 then
    self.jogador.vy = self.jogador.vy * CORTE_PULO
  end

  local j = self.jogador
  if not j.on_ground then
    self.anim:play(self.anims.pulo)
  elseif math.abs(j.vx) > 20 then
    self.anim:play(self.anims.andando)
  else
    self.anim:play(self.anims.parado)
  end
  self.anim:update(dt)
  self.giro:update(dt)
  self.piscar = math.max(0, self.piscar - dt)

  for i = #self.popups, 1, -1 do
    local p = self.popups[i]
    p.idade = p.idade + dt
    if p.idade > 0.7 then table.remove(self.popups, i) end
  end
end

function fase:draw()
  local a = nn.time.alpha()
  local j = self.jogador
  local x, y = util.lerp(self.ax, j.x, a), util.lerp(self.ay, j.y, a)

  -- Câmera: segue olhando um pouco à frente, sem sair do mapa.
  local w, h = nn.window.size()
  local zoom = nn.camera.zoom()
  local vw, vh = w / zoom, h / zoom
  local cx, cy = nn.camera.position()
  local t = 1 - math.exp(-5 * nn.time.delta())
  cx = util.lerp(cx, x + (self.esquerda and -90 or 90), t)
  cy = util.lerp(cy, y + 80, t)
  cx = util.clamp(cx, vw / 2, math.max(vw / 2, self.mapa.world_width - vw / 2))
  cy = util.clamp(cy, vh / 2, math.max(vh / 2, self.mapa.world_height - vh / 2))
  nn.camera.set_position(cx, cy)

  self.mapa:draw_layer("ceu")
  self.mapa:draw_layer("decoracao")
  for _, p in ipairs(self.plataformas) do
    nn.draw.sprite(self.img_plataforma, p.corpo.x, p.corpo.y, { w = p.corpo.width, h = p.corpo.height })
  end
  self.mapa:draw_layer("terreno")
  self.mapa:draw_layer("perigos")

  local tempo = nn.time.elapsed()
  for _, m in pairs(self.moedas) do
    nn.draw.sprite(self.moeda_sheet, m.x, m.y + math.sin(tempo * 3 + m.x) * 4,
      { frame = self.giro:frame(), w = 40, h = 40 })
  end

  -- Pisca depois de cair.
  if self.piscar == 0 or math.floor(self.piscar * 12) % 2 == 0 then
    nn.draw.sprite(self.sheet, x, y - 21 + TAM_SPRITE / 2, {
      frame = self.anim:frame(), w = TAM_SPRITE, h = TAM_SPRITE, flip_x = self.esquerda,
    })
  end

  for _, p in ipairs(self.popups) do
    local k = p.idade / 0.7
    nn.draw.text("+1", p.x, p.y + 24 + 40 * k, { size = 28, color = "#FFE27A", alpha = 1 - k * k, anchor = "center" })
  end

  if self.debug then self.mundo:debug_draw() end
  self:hud()
end

function fase:hud()
  local w, h = nn.window.size()
  nn.screen.rect(14, 14, 300, 50, { 0, 0, 0, 0.3 })
  nn.screen.sprite(self.moeda_sheet, 26, 21, { frame = 1, w = 36, h = 36 })
  util.texto_sombra(("%d / %d"):format(self.pegas, self.total), 72, 22, { size = 28, color = "#FFE27A" })
  util.texto_sombra(util.tempo(self.tempo), 180, 22, { size = 28 })
  if self.mortes > 0 then
    util.texto_sombra(("Quedas: %d"):format(self.mortes), w - 20, 22, { size = 22, anchor = "topright" })
  end
  if self.debug then
    nn.screen.text("F1: esconder colisões", 20, h - 20, { size = 16, anchor = "bottomleft", color = "#FFFFFF" })
  end
end

return fase
