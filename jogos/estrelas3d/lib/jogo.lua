-- O que as cenas compartilham: controles, aparência dos menus, opções da
-- câmera e o céu. Para usar: local jogo = require("lib.jogo")
local jogo = {}

function jogo.preparar()
  -- Ações: o jogo pergunta por "pular", e não pela tecla. Teclado e
  -- controle funcionam juntos.
  nn.input.bind("pular", { "space", "pad:a" })
  nn.input.bind("pausa", { "escape", "p", "pad:start" })

  nn.ui.theme({
    focus_color = "#FFD93D",
    focus_text_color = "#2B2D42",
    accent_color = "#FFD93D",
    move_sound = nn.assets.sound("assets/sons/menu.wav"),
    confirm_sound = nn.assets.sound("assets/sons/confirmar.wav"),
  })

  nn.d3.sky({ top = "#3F8FD8", horizon = "#CDEBF5", bottom = "#2E6F9E", sun_size = 4 })
  nn.d3.light({
    sun = { -0.45, -0.8, -0.4 },
    sun_color = "#FFF1D6",
    sky = "#93AFCF",
    ground = "#5E6B52",
    fog = { near = 35, far = 90 },
  })
  nn.d3.shadows({ distance = 25, quality = "media" })
end

-- Opções da câmera (salvas): sensibilidade do mouse e Y invertido.
function jogo.opcoes()
  return {
    sensibilidade = nn.save.get("sensibilidade", 0.15),
    inverter = nn.save.get("inverter_y", false),
  }
end

-- Escala da tela: 1 numa janela de 720 pixels de altura.
function jogo.escala()
  local _, h = nn.window.size()
  return h / 720
end

-- 83.4 → "1:23.4"
function jogo.tempo(segundos)
  return ("%d:%04.1f"):format(math.floor(segundos / 60), segundos % 60)
end

return jogo
