-- Jogador visto de cima: anda nas 8 direções.
local B = {
  title = "Jogador top-down",
  description = "Anda em todas as direções com as setas, WASD ou o controle (jogo visto de cima).",
  category = "Jogador",
  player = true,
  props = {
    { "velocidade", 260, label = "Velocidade", min = 0, help = "Unidades do mundo por segundo." },
  },
}

function B:start()
  local corpo = self.obj.body
  if corpo then corpo.gravity_scale = 0 end
end

function B:fixed_update(dt)
  local o = self.obj
  local dx, dy = nn.input.move()
  -- Numa conversa, fica parado.
  if self.scene._dlg then dx, dy = 0, 0 end
  if (o.empurrado or 0) > 0 then
    o.empurrado = o.empurrado - dt
    return
  end
  local corpo = o.body
  if corpo and corpo.alive then
    corpo.vx, corpo.vy = dx * self.velocidade, dy * self.velocidade
  else
    o.x = o.x + dx * self.velocidade * dt
    o.y = o.y + dy * self.velocidade * dt
  end
end

return B
