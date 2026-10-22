-- Testes de jogo: rode com `nn testar` (ou a ferramenta rodar_testes do
-- servidor MCP). Cada teste joga o jogo sem janela, começando na tela de
-- título, e confere o resultado. Veja "Testes de jogo" em docs/lua.md.

test("o botão Jogar abre a fase", function(t)
  t:click("jogar")
  t:wait_until(function() return t:scene_path() == "cenas/fase.cena" end, 2)
end)

test("o herói anda e pula", function(t)
  t:open("cenas/fase.cena")
  local heroi = t:find("heroi")
  t:wait(0.5) -- cai até o chão
  local x0, y0 = heroi.x, heroi.y
  t:hold("right", 0.5)
  t:check(heroi.x > x0 + 50, "o herói devia andar para a direita")
  t:press("space")
  t:wait(0.2)
  t:check(heroi.y > y0 + 30, "o herói devia pular")
end)

test("pegar uma moeda soma 1 em moedas", function(t)
  t:open("cenas/fase.cena")
  local moeda = t:find("moeda")
  t:find("heroi"):move_to(moeda.x, moeda.y)
  t:wait_until(function() return nn.vars.moedas == 1 end, 1, "a moeda não foi pega")
  t:check(not moeda.alive, "a moeda devia sumir")
end)

test("com todas as moedas, a saída leva ao fim", function(t)
  t:open("cenas/fase.cena")
  local heroi = t:find("heroi")
  for _, moeda in ipairs(t:find_all("moeda")) do
    heroi:move_to(moeda.x, moeda.y)
    t:wait(0.2)
  end
  t:check_eq(nn.vars.moedas, 4)
  local saida = t:find("saida")
  heroi:move_to(saida.x, saida.y)
  t:wait_until(function() return t:scene_path() == "cenas/fim.cena" end, 3)
end)
