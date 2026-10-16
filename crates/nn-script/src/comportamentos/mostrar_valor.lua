-- Mostra variáveis do jogo num texto (placar, vidas...).
local B = {
  title = "Mostrar valor",
  description = "Mostra variáveis do jogo no texto do objeto: \"Moedas: {moedas}\".",
  category = "Aparência",
  requires = { "text" },
  props = {
    { "texto", "Moedas: {moedas}", label = "Texto",
      help = "{nome} é trocado pelo valor da variável do jogo (nn.vars.nome)." },
  },
}

local function valor(nome)
  local v = nn.vars[nome]
  if v == nil then return "" end
  if type(v) == "number" and v == math.floor(v) then return string.format("%d", v) end
  return tostring(v)
end

function B:update(dt)
  local texto = self.obj.text
  if texto then texto.text = (self.texto:gsub("{([%w_]+)}", valor)) end
end

return B
