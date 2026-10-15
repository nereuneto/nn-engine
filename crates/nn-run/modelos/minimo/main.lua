-- Cena inicial. Com o jogo aberto, salve este arquivo: a mudança aparece na hora.
local cena = {}

function cena:enter()
  self.x, self.y = 0, 0
end

-- Passos fixos (60 por segundo): movimento e regras.
function cena:fixed_update(dt)
  local dx, dy = nn.input.move()   -- WASD, setas ou controle
  self.x = self.x + dx * 300 * dt
  self.y = self.y + dy * 300 * dt
end

function cena:draw()
  nn.draw.rect(self.x, self.y, 50, 50, "#3FA7D6")
  nn.screen.text("{{TITULO}}: use WASD, as setas ou o controle", 16, 16, { size = 22 })
end

return cena
