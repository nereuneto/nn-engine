-- Gira e flutua sozinho: estrelas, moedas, enfeites.
local B = {
  title = "Girar e flutuar",
  description = "Gira em volta de si e sobe e desce devagar (itens, estrelas, enfeites).",
  category = "Movimento",
  mode = "3d",
  props = {
    { "velocidade", 90, label = "Graus por segundo", help = "Negativo gira para o outro lado. 0 = não gira." },
    { "eixo", "y", label = "Em volta de",
      options = { { "y", "Y (em pé, como um pião)" }, { "x", "X" }, { "z", "Z" } } },
    { "altura", 0.2, label = "Altura do flutuar", min = 0, help = "Metros. 0 = não flutua." },
    { "duracao", 2, label = "Segundos por subida e descida", min = 0.1 },
  },
}

function B:start()
  self.t = 0
  self.y0 = self.obj.y
end

function B:fixed_update(dt)
  local o = self.obj
  self.t = self.t + dt
  local giro = "r" .. self.eixo
  o[giro] = (o[giro] + self.velocidade * dt) % 360
  if self.altura > 0 then
    o.y = self.y0 + math.sin(self.t / self.duracao * 2 * math.pi) * self.altura
  end
end

return B
