-- Troca os quadros da imagem conforme o objeto anda, pula ou cai.
local B = {
  title = "Animação automática",
  description = "Troca os quadros da imagem quando o objeto está parado, andando, pulando ou caindo.",
  category = "Aparência",
  requires = { "sprite" },
  props = {
    { "parado", "1", label = "Quadros parado", help = "Separados por vírgula; 1-4 vai do 1 ao 4." },
    { "andando", "", label = "Quadros andando" },
    { "pulando", "", label = "Quadros pulando" },
    { "caindo", "", label = "Quadros caindo" },
    { "fps", 10, label = "Quadros por segundo", min = 0.1 },
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
  self.ultimo_x = self.obj.x
end

function B:quadros(estado)
  while estado do
    local texto = self[estado] or ""
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
  if o.body and o.body.alive then
    vx, vy, chao = o.body.vx, o.body.vy, o.body.on_ground
  elseif dt > 0 then
    vx = (o.x - self.ultimo_x) / dt
  end
  self.ultimo_x = o.x
  local estado = "parado"
  if not chao then
    estado = vy > 0 and "pulando" or "caindo"
  elseif math.abs(vx) > 5 then
    estado = "andando"
  end
  local quadros = self:quadros(estado)
  if #quadros == 0 then return end
  if estado ~= self.estado then self.estado, self.tempo = estado, 0 end
  self.tempo = self.tempo + dt
  spr.frame = quadros[math.floor(self.tempo * self.fps) % #quadros + 1]
  if self.espelhar and math.abs(vx) > 5 then spr.flip_x = vx < 0 end
end

return B
