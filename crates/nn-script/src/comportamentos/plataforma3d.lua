-- Plataforma, elevador ou porta que vai e volta sozinha.
local B = {
  title = "Plataforma que anda",
  description = "Vai e volta sozinha (plataforma, elevador, porta). Quem está em cima vai junto.",
  category = "Movimento",
  mode = "3d",
  -- No NN Studio, ganha um Corpo 3D que se move sozinho (kinematic), que
  -- leva junto quem está em cima.
  requires = { "body3d" },
  body = "kinematic",
  props = {
    { "x", 4, label = "Andar em X", help = "Metros, a partir de onde ela está na cena." },
    { "y", 0, label = "Andar em Y (altura)" },
    { "z", 0, label = "Andar em Z" },
    { "duracao", 3, label = "Segundos para ir", min = 0.1 },
    { "espera", 0.5, label = "Espera nas pontas", min = 0, help = "Segundos parada em cada ponta." },
  },
}

function B:start()
  local o = self.obj
  self.t = 0
  self.x0, self.y0, self.z0 = o.x, o.y, o.z
end

-- De 0 a 1 e de volta, suave, com espera nas pontas.
function B:fracao(t)
  local ida, espera = math.max(self.duracao, 0.1), math.max(self.espera, 0)
  local volta = 2 * (ida + espera)
  t = t % volta
  if t < espera then return 0 end
  t = t - espera
  if t < ida then return (1 - math.cos(t / ida * math.pi)) / 2 end
  t = t - ida
  if t < espera then return 1 end
  t = t - espera
  return (1 + math.cos(t / ida * math.pi)) / 2
end

function B:fixed_update(dt)
  local o = self.obj
  self.t = self.t + dt
  local f = self:fracao(self.t)
  local x, y, z = self.x0 + self.x * f, self.y0 + self.y * f, self.z0 + self.z * f
  local corpo = o.body
  if corpo and corpo.alive and corpo.type == "kinematic" and dt > 0 and not o.parent then
    -- Anda pela velocidade: quem está em cima vai junto.
    corpo:set_velocity((x - o.x) / dt, (y - o.y) / dt, (z - o.z) / dt)
  else
    o.x, o.y, o.z = x, y, z
  end
end

return B
