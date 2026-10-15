local util = {}

function util.lerp(a, b, t) return a + (b - a) * t end

function util.clamp(v, min, max) return math.max(min, math.min(max, v)) end

-- Aproxima `v` de `alvo` em no máximo `passo`.
function util.approach(v, alvo, passo)
  if v < alvo then return math.min(v + passo, alvo) end
  return math.max(v - passo, alvo)
end

function util.texto_sombra(texto, x, y, opts)
  local sombra = {}
  for k, v in pairs(opts or {}) do sombra[k] = v end
  sombra.color = { 0, 0, 0, 0.55 }
  nn.screen.text(texto, x + 3, y + 3, sombra)
  nn.screen.text(texto, x, y, opts)
end

function util.tempo(segundos)
  return ("%d:%05.2f"):format(math.floor(segundos / 60), segundos % 60)
end

-- Música global: continua entre as cenas.
function util.tocar_musica()
  if not (musica and musica:playing()) then
    musica = nn.audio.play(nn.assets.sound("assets/sons/musica.wav"), { loop = true, volume = 0.3 })
  end
end

return util
