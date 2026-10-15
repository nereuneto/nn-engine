-- Configuração compartilhada entre as cenas: controles, aparência dos
-- menus, opções do jogador e recordes (guardados com nn.save).
local jogo = {}

local OPCOES_PADRAO = { musica = 0.7, efeitos = 0.8, tela_cheia = false }

-- Chamado ao entrar no título e na fase. Pode ser chamado várias vezes.
function jogo.preparar()
  -- Cada ação aceita teclado e controle ao mesmo tempo.
  nn.input.bind("esquerda", { "left", "a", "pad:left", "pad:leftx-" })
  nn.input.bind("direita", { "right", "d", "pad:right", "pad:leftx+" })
  nn.input.bind("pular", { "space", "up", "w", "pad:a" })
  nn.input.bind("pausa", { "escape", "p", "pad:start" })

  nn.ui.theme({
    panel_color = "#14303F",
    button_color = "#FFFFFF14",
    focus_color = "#FFE27A",
    focus_text_color = "#1E3A4C",
    accent_color = "#FFE27A",
    label_color = "#CFE3EC",
    move_sound = nn.assets.sound("assets/sons/menu.wav"),
    confirm_sound = nn.assets.sound("assets/sons/confirmar.wav"),
  })

  jogo.fundos = {
    nuvens = nn.assets.texture("assets/imagens/nuvens.png", { pixel_art = true }),
    montanhas = nn.assets.texture("assets/imagens/montanhas.png", { pixel_art = true }),
    colinas = nn.assets.texture("assets/imagens/colinas.png", { pixel_art = true }),
  }

  jogo.aplicar_opcoes(jogo.opcoes())
end

-- Fundo em camadas (parallax): quanto mais longe, mais devagar acompanha a
-- câmera. Chame depois de posicionar a câmera e antes do resto do mundo.
-- O chão da fase fica em y = 144, então as camadas terminam um pouco abaixo.
function jogo.desenhar_fundo()
  local f = jogo.fundos
  nn.draw.parallax(f.nuvens, { scale = 3, factor_x = 0.08, factor_y = 1, y = 560, scroll_x = 12 })
  nn.draw.parallax(f.montanhas, { scale = 4, factor_x = 0.2, factor_y = 1, y = 276 })
  nn.draw.parallax(f.colinas, { scale = 4, factor_x = 0.45, factor_y = 1, y = 212 })
end

-- Opções salvas, com os valores padrão no que faltar.
function jogo.opcoes()
  local o = nn.save.get("opcoes", {})
  for k, v in pairs(OPCOES_PADRAO) do
    if o[k] == nil then o[k] = v end
  end
  return o
end

function jogo.aplicar_opcoes(o)
  nn.audio.set_volume(o.musica, "music")
  nn.audio.set_volume(o.efeitos, "effects")
  if nn.window.fullscreen() ~= o.tela_cheia then
    nn.window.set_fullscreen(o.tela_cheia)
  end
end

function jogo.salvar_opcoes(o)
  nn.save.set("opcoes", o)
  jogo.aplicar_opcoes(o)
end

-- { tempo = melhor tempo, moedas = mais moedas, vitorias = quantas vezes chegou }
function jogo.recordes()
  return nn.save.get("recordes", {})
end

-- Registra o resultado de uma partida. Devolve o que foi recorde e os recordes.
function jogo.registrar(r)
  local rec = jogo.recordes()
  local novo = {}
  if rec.tempo == nil or r.tempo < rec.tempo then
    rec.tempo = r.tempo
    novo.tempo = true
  end
  if rec.moedas == nil or r.moedas > rec.moedas then
    rec.moedas = r.moedas
    novo.moedas = true
  end
  rec.vitorias = (rec.vitorias or 0) + 1
  nn.save.set("recordes", rec)
  return novo, rec
end

-- Dica de controles conforme o que o jogador está usando.
function jogo.ajuda()
  if nn.input.last_device() == "gamepad" then
    return "Analógico ou direcional: andar    A: pular    Start: pausa"
  end
  return "Setas/AD: andar    Espaço/W: pular    Esc: pausa    F1: ver colisões"
end

return jogo
