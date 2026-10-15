-- Menu principal.
local util = require("lib.util")

local menu = {}

local OPCOES = { "Jogar", "Sair" }

function menu:enter()
  self.selecionada = 1
  self.moeda = nn.assets.sheet("assets/imagens/moeda.png", 16, 16, { pixel_art = true })
  self.giro = nn.animator(nn.animation({ 1, 2, 3, 4 }, 9, "pingpong"))
  self.som_menu = nn.assets.sound("assets/sons/menu.wav")
  self.som_ok = nn.assets.sound("assets/sons/confirmar.wav")

  -- A música é global: continua tocando entre as cenas.
  if not (musica and musica:playing()) then
    musica = nn.audio.play(nn.assets.sound("assets/sons/musica.wav"), { loop = true, volume = 0.35 })
  end
  nn.camera.set_position(0, 0)
end

function menu:update(dt)
  self.giro:update(dt)

  local antes = self.selecionada
  if nn.input.pressed("up") or nn.input.pressed("w") then self.selecionada = self.selecionada - 1 end
  if nn.input.pressed("down") or nn.input.pressed("s") then self.selecionada = self.selecionada + 1 end
  self.selecionada = (self.selecionada - 1) % #OPCOES + 1
  if self.selecionada ~= antes then nn.audio.play(self.som_menu) end

  if nn.input.pressed("enter") or nn.input.pressed("space") then
    nn.audio.play(self.som_ok)
    if OPCOES[self.selecionada] == "Jogar" then
      nn.scene.switch("cenas/jogo.lua", { moedas = 20 })
    else
      nn.quit()
    end
  end
  if nn.input.pressed("escape") then nn.quit() end
end

function menu:draw()
  local w, h = nn.window.size()
  local t = nn.time.elapsed()

  -- Moedas girando no rodapé.
  for i = 0, 7 do
    local x = w * (i + 0.5) / 8
    local y = h * 0.82 + math.sin(t * 2 + i) * 10
    nn.screen.sprite(self.moeda, x, y, {
      frame = self.giro:frame(), scale = 3, origin_x = 0.5, origin_y = 0.5, alpha = 0.5,
    })
  end

  util.texto_sombra("nn-engine", w / 2, h * 0.22, { size = 72, color = "#FFD24A", anchor = "center" })
  util.texto_sombra("demo em Lua", w / 2, h * 0.22 + 64, { size = 24, color = "#B8C4BB", anchor = "center" })

  for i, nome in ipairs(OPCOES) do
    local y = h * 0.48 + (i - 1) * 64
    local selecionada = i == self.selecionada
    if selecionada then
      local tw, th = nn.text_size(nome, { size = 40 })
      nn.screen.rect(w / 2 - tw / 2 - 24, y - th / 2 - 6, tw + 48, th + 12, { 1, 1, 1, 0.08 })
    end
    nn.screen.text(nome, w / 2, y, {
      size = 40, anchor = "center", color = selecionada and "#FFFFFF" or "#7C8A80",
    })
  end

  nn.screen.text("setas: escolher    Enter: confirmar", w / 2, h - 24,
    { size = 16, anchor = "bottom", color = "#7C8A80" })
end

return menu
