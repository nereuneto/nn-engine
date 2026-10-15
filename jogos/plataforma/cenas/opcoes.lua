-- Opções: volume, tela cheia e apagar recordes. Abre por cima do título ou
-- da pausa; as escolhas ficam salvas para a próxima vez que o jogo abrir.
local jogo = require("lib.jogo")

local opcoes = { overlay = true }

function opcoes:enter()
  self.o = jogo.opcoes()
  self.amostra = nn.assets.sound("assets/sons/moeda.wav")
  self.confirmar = false
  nn.ui.set_focus("opcoes", 1)
end

function opcoes:draw()
  local w, h = nn.window.size()
  nn.screen.rect(0, 0, w, h, { 0.03, 0.06, 0.1, 0.45 })

  local o = self.o
  local mudou, m = false, false
  local ui = nn.ui.panel("opcoes", { title = "Opções", width = 520 })
  o.musica, m = ui:slider("Música", o.musica, { step = 0.1 })
  mudou = mudou or m
  o.efeitos, m = ui:slider("Efeitos", o.efeitos, { step = 0.1 })
  if m then
    mudou = true
    -- Toca um efeito para o jogador ouvir o volume novo.
    nn.audio.set_volume(o.efeitos, "effects")
    nn.audio.play(self.amostra, { volume = 0.8 })
  end
  o.tela_cheia, m = ui:toggle("Tela cheia", o.tela_cheia)
  mudou = mudou or m

  local texto = self.confirmar and "Confirmar: apagar recordes?" or "Apagar recordes"
  if ui:button(texto) then
    if self.confirmar then
      nn.save.remove("recordes")
      self.confirmar = false
    else
      self.confirmar = true
    end
  end
  if ui:focus() ~= 4 then self.confirmar = false end

  if ui:button("Voltar") or ui:back() then nn.scene.pop() end
  ui:finish()

  if mudou then jogo.salvar_opcoes(o) end
end

return opcoes
