-- Carga parecida com a de um quadro de jogo, repetida: 2000 objetos andando
-- por 300 quadros, recursão e montagem de texto. Quem roda mede o tempo.
local objs = {}
for i = 1, 2000 do objs[i] = { x = i, y = 0, vx = 1, vy = 2 } end
for _ = 1, 300 do
  for i = 1, #objs do
    local o = objs[i]
    o.x = o.x + o.vx * 0.016
    o.y = o.y + o.vy * 0.016
    if o.x > 800 then o.x = 0 end
  end
end
local function fib(n) if n < 2 then return n end return fib(n - 1) + fib(n - 2) end
local f = fib(25)
local n = 0
for i = 1, 20000 do n = n + #tostring(i) end
return n + f
