-- Testes de jogo: rode com `nn testar` (ou a ferramenta rodar_testes do
-- servidor MCP). Cada teste joga o jogo sem janela, começando na tela de
-- título, e confere o resultado. Numa cena em código, t:scene() é o `self`
-- da cena. Veja "Testes de jogo" em docs/lua.md.

test("Enter no título começa a partida", function(t)
  t:press("enter")
  t:wait_until(function() return t:scene_path() == "cenas/jogo.lua" end, 2)
end)

test("o herói anda", function(t)
  t:open("cenas/jogo.lua")
  local cena = t:scene()
  local x0 = cena.x
  t:hold("right", 0.5)
  t:check(cena.x > x0 + 100, "o herói devia andar para a direita")
end)

test("pegar a moeda vale um ponto", function(t)
  t:open("cenas/jogo.lua")
  local cena = t:scene()
  cena.x, cena.y = cena.moeda.x, cena.moeda.y
  t:wait(0.1)
  t:check_eq(cena.pontos, 1)
end)

test("quando o tempo acaba, aparece o fim", function(t)
  t:open("cenas/jogo.lua")
  t:scene().tempo = 0.1
  t:wait_until(function() return t:scene_path() == "cenas/fim.lua" end, 1)
end)
