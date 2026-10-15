-- Configuração compartilhada entre as cenas. Para usar em uma cena:
--   local jogo = require("lib.jogo")
local jogo = {}

-- Chamado ao entrar em cada cena principal. Pode ser chamado várias vezes.
function jogo.preparar()
  -- Ações: o jogo pergunta por "pausa", e não por "Esc". Assim o teclado e
  -- o controle funcionam juntos. Acrescente as suas aqui (ex.: "pular").
  nn.input.bind("pausa", { "escape", "p", "pad:start" })

  -- Aparência dos menus (nn.ui).
  nn.ui.theme({
    focus_color = "#FFD166",
    focus_text_color = "#14202E",
    accent_color = "#FFD166",
    move_sound = nn.assets.sound("assets/sons/menu.wav"),
    confirm_sound = nn.assets.sound("assets/sons/confirmar.wav"),
  })
end

-- Escala da tela: 1 numa janela de 720 pixels de altura. Multiplique os
-- tamanhos do que vai na tela (textos, HUD) por ela para o jogo ficar igual
-- em qualquer resolução, inclusive em tela cheia.
function jogo.escala()
  local _, h = nn.window.size()
  return h / 720
end

return jogo
