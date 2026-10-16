-- Troca os quadros da imagem conforme o objeto anda, pula ou cai. Cada estado
-- aceita o nome de uma animação do .anim da imagem ("andar") ou uma lista de
-- quadros ("3-6").
local B = {
  title = "Animação automática",
  description = "Troca os quadros da imagem quando o objeto está parado, andando, pulando ou caindo.",
  category = "Aparência",
  requires = { "sprite" },
  props = {
    { "parado", "1", type = "anim", label = "Parado",
      help = "Uma animação da imagem, ou quadros separados por vírgula (1-4 vai do 1 ao 4)." },
    { "andando", "", type = "anim", label = "Andando" },
    { "pulando", "", type = "anim", label = "Pulando" },
    { "caindo", "", type = "anim", label = "Caindo" },
    { "fps", 10, label = "Quadros por segundo", min = 0.1,
      help = "Para listas de quadros (as animações da imagem têm o seu próprio ritmo)." },
    { "espelhar", true, label = "Vira a imagem", help = "A imagem olha para onde está indo." },
  },
}

local function lista(texto)
  local out = {}
  for parte in tostring(texto):gmatch("[^,%s]+") do
    local a, b = parte:match("^(%d+)%-(%d+)$")
    if a then
      a, b = tonumber(a), tonumber(b)
      local passo = a <= b and 1 or -1
      for i = a, b, passo do table.insert(out, i) end
    elseif tonumber(parte) then
      table.insert(out, math.floor(tonumber(parte)))
    end
  end
  return out
end

-- Se não houver quadros para um estado, usa os do próximo.
local RESERVA = { caindo = "pulando", pulando = "andando", andando = "parado" }

function B:start()
  self.cache = {}
  self.tempo = 0
  self.ultimo_x, self.ultimo_y = self.obj.x, self.obj.y
end

-- Os quadros do estado (ou o nome da animação), passando para o próximo
-- estado quando este está vazio.
function B:quadros(estado, anims)
  while estado do
    local texto = tostring(self[estado] or "")
    if anims and anims.by_name[texto] then return nil, texto end
    local c = self.cache[estado]
    if not c or c.texto ~= texto then
      c = { texto = texto, lista = lista(texto) }
      self.cache[estado] = c
    end
    if #c.lista > 0 then return c.lista end
    estado = RESERVA[estado]
  end
  return {}
end

function B:update(dt)
  local o = self.obj
  local spr = o.sprite
  if not spr then return end
  local vx, vy, chao = 0, 0, true
  -- Sem gravidade (jogo visto de cima): andar vale para todos os lados.
  local de_cima = false
  if o.body and o.body.alive then
    vx, vy, chao = o.body.vx, o.body.vy, o.body.on_ground
    de_cima = o.body.gravity_scale == 0 or (self.scene._gravity or -1800) == 0
  elseif dt > 0 then
    vx = (o.x - self.ultimo_x) / dt
    vy = (o.y - (self.ultimo_y or o.y)) / dt
    de_cima = true
  end
  self.ultimo_x, self.ultimo_y = o.x, o.y
  local estado = "parado"
  if de_cima then
    if math.abs(vx) > 5 or math.abs(vy) > 5 then estado = "andando" end
  elseif not chao then
    estado = vy > 0 and "pulando" or "caindo"
  elseif math.abs(vx) > 5 then
    estado = "andando"
  end
  if self.espelhar and math.abs(vx) > 5 then spr.flip_x = vx < 0 end
  local quadros, nome = self:quadros(estado, spr._anims)
  if nome then
    spr:play(nome)
    return
  end
  if #quadros == 0 then return end
  if estado ~= self.estado then self.estado, self.tempo = estado, 0 end
  self.tempo = self.tempo + dt
  if spr.animation ~= "" and spr._anims then spr:stop() end
  spr.frame = quadros[math.floor(self.tempo * self.fps) % #quadros + 1]
end

return B
