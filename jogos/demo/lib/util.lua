-- Funções de apoio usadas pelas cenas. Carregue com: local util = require("lib.util")
local util = {}

function util.lerp(a, b, t)
  return a + (b - a) * t
end

function util.clamp(v, min, max)
  return math.max(min, math.min(max, v))
end

function util.dist(x1, y1, x2, y2)
  local dx, dy = x2 - x1, y2 - y1
  return math.sqrt(dx * dx + dy * dy)
end

-- Texto na tela com uma sombra discreta.
function util.texto_sombra(texto, x, y, opts)
  local sombra = {}
  for k, v in pairs(opts or {}) do sombra[k] = v end
  sombra.color = { 0, 0, 0, 0.6 }
  nn.screen.text(texto, x + 2, y + 2, sombra)
  nn.screen.text(texto, x, y, opts)
end

return util
