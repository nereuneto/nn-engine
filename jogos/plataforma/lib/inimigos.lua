-- Inimigos: a lesma patrulha (vira na parede e na beirada) e o morcego
-- persegue o jogador quando ele chega perto, voltando para casa depois.
-- Pisar em cima derrota; encostar de lado derruba o jogador.
local util = require("lib.util")

local inimigos = {}

-- Camadas da física: inimigos não empurram o jogador nem uns aos outros,
-- só colidem com o cenário. O contato com o jogador é conferido à parte,
-- com corpo:intersects(outro).
inimigos.CAMADA = 4
inimigos.CAMADA_JOGADOR = 2
local MASCARA = 1

local LESMA = { vel = 70, w = 36, h = 27 }
local MORCEGO = { vel = 160, raio = 16, alcance = 330, coleira = 520 }
local QUIQUE, QUIQUE_ALTO = 640, 980   -- impulso ao pisar (segurando o pulo, mais alto)
local TAM = 48                          -- tamanho do sprite

-- Cria os inimigos marcados no mapa (objetos "lesma" e "morcego").
function inimigos.criar(fase)
  fase.sheet_inimigos = nn.assets.sheet("assets/imagens/inimigos.png", 16, 16, { pixel_art = true })
  fase.som_pisar = nn.assets.sound("assets/sons/pisar.wav")
  local anims = {
    lesma = nn.animation({ 1, 2 }, 4, "pingpong"),
    morcego = nn.animation({ 5, 6 }, 10),
  }
  local lista = {}
  for _, tipo in ipairs({ "lesma", "morcego" }) do
    for _, o in ipairs(fase.mapa:objects(tipo)) do
      local opcoes = { x = o.x, y = o.y, tag = tipo }
      if tipo == "lesma" then
        opcoes.w, opcoes.h = LESMA.w, LESMA.h
      else
        opcoes.radius = MORCEGO.raio
      end
      local corpo = fase.mundo:body(opcoes)
      corpo.layer, corpo.mask = inimigos.CAMADA, MASCARA
      if tipo == "morcego" then corpo.gravity_scale = 0 end
      table.insert(lista, {
        tipo = tipo, corpo = corpo, vivo = true,
        dir = -1, casa_x = o.x, casa_y = o.y, ax = o.x, ay = o.y,
        anim = nn.animator(anims[tipo]), fase_oscila = math.random() * 6,
      })
    end
  end
  fase.inimigos = lista
end

local function lesma(fase, e, dt)
  local c = e.corpo
  if c.touching_left then e.dir = 1 elseif c.touching_right then e.dir = -1 end
  if c.on_ground then
    -- Sem chão logo à frente: vira (não cai da beirada).
    local col, lin = fase.mapa:cell_at(c.x + e.dir * (c.width / 2 + 4), c.y - c.height / 2 - 4)
    if not col or not fase.mapa:solid(col, lin) then e.dir = -e.dir end
  end
  c.vx = e.dir * LESMA.vel
end

local function morcego(fase, e, dt)
  local c, j = e.corpo, fase.jogador
  local para_casa = math.sqrt((c.x - e.casa_x) ^ 2 + (c.y - e.casa_y) ^ 2)
  local dx, dy = j.x - c.x, (j.y + 10) - c.y
  local dist = math.sqrt(dx * dx + dy * dy)
  local alvo_x, alvo_y, vel
  if dist < MORCEGO.alcance and para_casa < MORCEGO.coleira and fase.piscar == 0 then
    alvo_x, alvo_y, vel = j.x, j.y + 10, MORCEGO.vel
  else
    -- Em casa: flutua devagar para cima e para baixo.
    e.fase_oscila = e.fase_oscila + dt
    alvo_x, alvo_y, vel = e.casa_x, e.casa_y + math.sin(e.fase_oscila * 2) * 24, MORCEGO.vel * 0.6
  end
  local tx, ty = alvo_x - c.x, alvo_y - c.y
  local len = math.sqrt(tx * tx + ty * ty)
  local vx, vy = 0, 0
  if len > 2 then vx, vy = tx / len * vel, ty / len * vel end
  c.vx = util.approach(c.vx, vx, 500 * dt)
  c.vy = util.approach(c.vy, vy, 500 * dt)
  if math.abs(c.vx) > 5 then e.dir = c.vx < 0 and -1 or 1 end
end

local function derrotar(fase, e)
  e.vivo = false
  e.morte = 0
  e.x, e.y = e.corpo.x, e.corpo.y
  e.vy = 0
  e.corpo:remove()
  nn.audio.play(fase.som_pisar)
  nn.camera.shake(5, 0.12)
  fase.efeitos.poeira:burst(14, e.x, e.y)
  fase.derrotados = (fase.derrotados or 0) + 1
end

-- Passo fixo: movimento dos inimigos e contato com o jogador. Chamado
-- depois de mundo:step.
function inimigos.atualizar(fase, dt)
  local j = fase.jogador
  for i = #fase.inimigos, 1, -1 do
    local e = fase.inimigos[i]
    if e.vivo then
      e.ax, e.ay = e.corpo.x, e.corpo.y
      if e.tipo == "lesma" then lesma(fase, e, dt) else morcego(fase, e, dt) end
      if e.corpo.y < -200 then
        e.corpo:remove()
        table.remove(fase.inimigos, i)
      elseif fase.piscar == 0 and not fase.venceu and j:intersects(e.corpo) then
        local pes = j.y - j.height / 2
        if j.vy < 0 and pes > e.corpo.y - 6 then
          -- Veio de cima: pisou.
          derrotar(fase, e)
          j.vy = nn.input.action("pular") and QUIQUE_ALTO or QUIQUE
        else
          fase:cair()
          return
        end
      end
    else
      -- Derrotado: a lesma fica achatada; o morcego cai girando.
      e.morte = e.morte + dt
      if e.tipo == "morcego" then
        e.vy = e.vy - 1600 * dt
        e.y = e.y + e.vy * dt
      end
      if e.morte > 0.8 then table.remove(fase.inimigos, i) end
    end
  end
end

function inimigos.animar(fase, dt)
  for _, e in ipairs(fase.inimigos) do
    if e.vivo then e.anim:update(dt) end
  end
end

function inimigos.desenhar(fase)
  local a = nn.time.alpha()
  for _, e in ipairs(fase.inimigos) do
    if e.vivo then
      local c = e.corpo
      local x = util.lerp(e.ax, c.x, a)
      local y = util.lerp(e.ay, c.y, a)
      -- A lesma tem o corpo mais baixo que o sprite: alinha pela base.
      if e.tipo == "lesma" then y = y - c.height / 2 + TAM / 2 end
      nn.draw.sprite(fase.sheet_inimigos, x, y, {
        frame = e.anim:frame(), w = TAM, h = TAM, flip_x = e.dir > 0,
      })
    else
      local k = math.min(e.morte / 0.8, 1)
      local frame = e.tipo == "lesma" and 3 or 7
      local y = e.tipo == "lesma" and (e.y - LESMA.h / 2 + TAM / 2) or e.y
      nn.draw.sprite(fase.sheet_inimigos, e.x, y, {
        frame = frame, w = TAM, h = TAM, alpha = 1 - k * k,
        rotation = e.tipo == "morcego" and e.morte * 8 or 0,
      })
    end
  end
end

return inimigos
