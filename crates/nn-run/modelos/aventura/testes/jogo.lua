-- Testes de jogo: rode com `nn testar` (ou a ferramenta rodar_testes do
-- servidor MCP). Cada teste joga o jogo sem janela, começando na tela de
-- título, e confere o resultado. Veja "Testes de jogo" em docs/lua.md.

test("Enter no título abre o mapa", function(t)
  t:press("enter")
  t:wait_until(function() return t:scene_path() == "cenas/mapa.cena" end, 2)
end)

test("o herói anda para cima e para a direita", function(t)
  t:open("cenas/mapa.cena")
  local heroi = t:find("heroi")
  local x0, y0 = heroi.x, heroi.y
  t:hold("up", 0.4)
  t:check(heroi.y > y0 + 50, "o herói devia subir")
  t:hold("right", 0.4)
  t:check(heroi.x > x0 + 50, "o herói devia ir para a direita")
end)

test("sem a chave, a porta avisa que está trancada", function(t)
  t:open("cenas/mapa.cena")
  local heroi, porta = t:find("heroi"), t:find("porta")
  heroi:move_to(porta.x - 60, porta.y)
  t:hold("right", 0.5)
  t:check(porta.alive, "a porta não devia abrir sem a chave")
  t:check(t:find("aviso").text.text:find("trancada"), "devia avisar que a porta está trancada")
end)

test("com a chave, a porta abre", function(t)
  t:open("cenas/mapa.cena")
  local heroi, porta = t:find("heroi"), t:find("porta")
  local chave = t:find("chave")
  heroi:move_to(chave.x, chave.y)
  t:wait_until(function() return nn.vars.chaves == 1 end, 1, "a chave não foi pega")
  heroi:move_to(porta.x - 60, porta.y)
  t:hold("right", 0.5)
  t:check(not porta.alive, "a porta devia abrir")
  t:check_eq(nn.vars.chaves, 0, "abrir a porta gasta a chave")
end)

test("o baú leva ao fim", function(t)
  t:open("cenas/mapa.cena")
  local bau = t:find("bau")
  t:find("heroi"):move_to(bau.x - 60, bau.y)
  t:hold("right", 0.3)
  t:wait_until(function() return t:scene_path() == "cenas/fim.cena" end, 4)
end)
