-- Testes de jogo: rode com `nn testar` (ou a ferramenta rodar_testes do
-- servidor MCP). Cada teste joga o jogo sem janela, começando na tela de
-- título, e confere o resultado. Veja "Testes de jogo" em docs/lua.md.

test("o botão Jogar começa a partida", function(t)
  t:click("jogar")
  t:wait_until(function() return t:scene_path() == "cenas/jogo.cena" end, 2)
  t:check_eq(nn.vars.pontos, 0)
end)

test("a nave anda e atira", function(t)
  t:open("cenas/jogo.cena")
  local nave = t:find("nave")
  local y0 = nave.y
  t:hold("up", 0.3)
  t:check(nave.y > y0 + 50, "a nave devia subir")
  t:hold("space", 0.3)
  t:check(#t:find_all("tiro") > 0, "segurar espaço devia atirar")
end)

test("acertar um asteroide vale 10 pontos", function(t)
  t:open("cenas/jogo.cena")
  local nave = t:find("nave")
  -- Um asteroide bem na frente da nave.
  t:scene():spawn("prefabs/asteroide.prefab", nave.x + 250, nave.y)
  t:down("space")
  t:wait_until(function() return nn.vars.pontos >= 10 end, 2, "o tiro não acertou o asteroide")
  t:up("space")
end)

test("sem vidas, a partida acaba", function(t)
  t:open("cenas/jogo.cena")
  t:find("nave"):send("dano", 3)
  t:wait_until(function() return t:scene_path() == "cenas/fim.cena" end, 3)
end)
