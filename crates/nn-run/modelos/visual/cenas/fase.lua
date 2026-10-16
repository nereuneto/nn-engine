-- Script da cena visual cenas/fase.cena: as regras do jogo.
-- Os objetos montados no NN Studio estão em self.objects, pelo nome
-- (self.objects.heroi), e self:find_all("moeda") devolve todos com o nome.
local cena = {}

local VELOCIDADE = 320   -- unidades do mundo por segundo
local PULO = 860         -- velocidade para cima ao pular

function cena:enter()
  self.pontos = 0
  self.total = #self:find_all("moeda")
  self.parado = nn.animation({ 1, 2 }, 2)
  self.andando = nn.animation({ 5, 6, 7, 8 }, 12)
  self.anim = nn.animator(self.parado)
  self.giro = nn.animator(nn.animation({ 1, 2, 3, 4 }, 9, "pingpong"))
  self:mostrar_placar()
end

function cena:mostrar_placar()
  self.objects.placar.text.text = ("Moedas: %d de %d"):format(self.pontos, self.total)
end

-- Passos fixos (60 por segundo): movimento e regras.
function cena:fixed_update(dt)
  local heroi = self.objects.heroi
  local corpo = heroi.body
  local dx = nn.input.move()
  corpo.vx = dx * VELOCIDADE
  if self.quer_pular and corpo.on_ground then
    corpo.vy = PULO
  end
  self.quer_pular = false
  -- Caiu do mapa: volta para o começo.
  if heroi.y < -600 then
    corpo:set_position(0, 0)
    corpo:set_velocity(0, 0)
  end
end

-- Uma vez por quadro: teclas apertadas e animação.
function cena:update(dt)
  if nn.input.pressed("space") or nn.input.pressed("up") or nn.input.pressed("w")
      or nn.input.pad_pressed("a") then
    self.quer_pular = true
  end
  local heroi = self.objects.heroi
  local vx = heroi.body.vx
  if vx ~= 0 then heroi.sprite.flip_x = vx < 0 end
  self.anim:play(vx ~= 0 and self.andando or self.parado)
  self.anim:update(dt)
  heroi.sprite.frame = self.anim:frame()
  -- Todas as moedas giram juntas.
  self.giro:update(dt)
  for _, m in ipairs(self:find_all("moeda")) do
    m.sprite.frame = self.giro:frame()
  end
end

-- Dois corpos se tocaram: e.object_a e e.object_b são os objetos.
-- (O sensor, como a moeda, vem em object_a.)
function cena:contact(e)
  local moeda, quem = e.object_a, e.object_b
  if e.type == "enter" and moeda and moeda.name == "moeda" and quem == self.objects.heroi then
    moeda:destroy()
    self.pontos = self.pontos + 1
    nn.audio.play(self.objects.som_moeda.sound)
    self:mostrar_placar()
    if self.pontos == self.total then
      self.objects.placar.text.text = "Você pegou todas!"
    end
  end
end

return cena
