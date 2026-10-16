-- Movimento sozinho: girar, flutuar ou ir e voltar.
local B = {
  title = "Movimento automático",
  description = "Gira, flutua ou vai e volta sozinho (plataformas móveis, itens, enfeites).",
  category = "Movimento",
  props = {
    { "tipo", "ir_e_voltar", label = "Movimento",
      options = {
        { "ir_e_voltar", "ir e voltar" },
        { "flutuar", "flutuar (sobe e desce)" },
        { "girar", "girar" },
      } },
    { "distancia_x", 200, label = "Distância em X", show_if = { "tipo", "ir_e_voltar" } },
    { "distancia_y", 0, label = "Distância em Y", show_if = { "tipo", "ir_e_voltar" } },
    { "altura", 12, label = "Altura", show_if = { "tipo", "flutuar" } },
    { "duracao", 2, label = "Segundos por volta", min = 0.05,
      show_if = { "tipo", "ir_e_voltar", "flutuar" } },
    { "velocidade", 90, label = "Graus por segundo", show_if = { "tipo", "girar" } },
  },
}

function B:start()
  self.t = 0
  self.x0, self.y0 = self.obj.x, self.obj.y
end

function B:fixed_update(dt)
  local o = self.obj
  self.t = self.t + dt
  if self.tipo == "girar" then
    o.rotation = o.rotation + self.velocidade * dt
    return
  end
  local volta = self.t / math.max(self.duracao, 0.05) * 2 * math.pi
  local x, y = self.x0, self.y0
  if self.tipo == "flutuar" then
    y = self.y0 + math.sin(volta) * self.altura
  else
    local f = (1 - math.cos(volta)) / 2   -- de 0 a 1 e de volta, suave
    x, y = self.x0 + self.distancia_x * f, self.y0 + self.distancia_y * f
  end
  local corpo = o.body
  if corpo and corpo.alive and corpo.type == "kinematic" and dt > 0 then
    -- Cinemático anda pela velocidade e carrega quem estiver em cima.
    corpo.vx, corpo.vy = (x - o.x) / dt, (y - o.y) / dt
  else
    o.x, o.y = x, y
  end
end

return B
