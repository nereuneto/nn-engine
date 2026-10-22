-- Testes de jogo: rode com `nn testar` (ou a ferramenta rodar_testes do
-- servidor MCP). Cada teste joga o jogo sem janela e confere o resultado.
-- No 3D, as posições são em metros e y é a altura. Veja "Testes de jogo"
-- em docs/lua.md.

test("o herói anda e pula", function(t)
  local heroi = t:find("Herói")
  t:wait(0.5) -- cai até o chão
  local x0, y0, z0 = heroi.x, heroi.y, heroi.z
  t:hold("w", 0.6)
  local andou = math.sqrt((heroi.x - x0) ^ 2 + (heroi.z - z0) ^ 2)
  t:check(andou > 1, "o herói devia andar (andou " .. andou .. " m)")
  t:press("space")
  t:wait(0.2)
  t:check(heroi.y > y0 + 0.5, "o herói devia pular")
end)

test("pegar uma moeda soma 1 em moedas", function(t)
  local moeda = t:find("moeda")
  t:find("Herói"):move_to(moeda.x, moeda.y, moeda.z)
  t:wait_until(function() return nn.vars.moedas == 1 end, 1, "a moeda não foi pega")
end)

test("pegar as estrelas soma em estrelas", function(t)
  local heroi = t:find("Herói")
  local estrelas = t:find_all("estrela")
  for _, e in ipairs(estrelas) do
    heroi:move_to(e.x, e.y, e.z)
    t:wait(0.2)
  end
  t:check_eq(nn.vars.estrelas, #estrelas, "devia ter pegado todas")
end)

test("com 6 estrelas, a ilha leva ao fim", function(t)
  nn.vars.estrelas = 6
  t:wait_until(function() return t:scene_path() == "cenas/fim.cena" end, 3)
end)
