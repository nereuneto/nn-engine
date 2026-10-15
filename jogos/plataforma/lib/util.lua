local util = {}

function util.lerp(a, b, t) return a + (b - a) * t end

function util.clamp(v, min, max) return math.max(min, math.min(max, v)) end

-- Aproxima `v` de `alvo` em no máximo `passo`.
function util.approach(v, alvo, passo)
  if v < alvo then return math.min(v + passo, alvo) end
  return math.max(v - passo, alvo)
end

-- Escala da tela: 1 numa janela de 720 pixels de altura. Usada para o
-- jogo ocupar o mesmo espaço em qualquer resolução (inclusive tela cheia).
function util.escala()
  local _, h = nn.window.size()
  return h / 720
end

function util.texto_sombra(texto, x, y, opts)
  local sombra = {}
  for k, v in pairs(opts or {}) do sombra[k] = v end
  local d = 3 * util.escala()
  sombra.color = { 0, 0, 0, 0.55 * (opts and opts.alpha or 1) }
  sombra.alpha = nil
  nn.screen.text(texto, x + d, y + d, sombra)
  nn.screen.text(texto, x, y, opts)
end

function util.tempo(segundos)
  return ("%d:%05.2f"):format(math.floor(segundos / 60), segundos % 60)
end

util.VOLUME_MUSICA = 0.5

-- Música global: continua entre as cenas. Toca no canal de música, então
-- o volume escolhido nas opções vale para ela.
function util.tocar_musica()
  if not (musica and musica:playing()) then
    musica = nn.audio.play(nn.assets.sound("assets/sons/musica.wav"),
      { loop = true, volume = util.VOLUME_MUSICA, channel = "music" })
  end
end

return util
