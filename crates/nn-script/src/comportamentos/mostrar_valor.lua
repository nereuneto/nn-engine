-- Mostra variáveis do jogo num texto (placar, vidas...).
local B = {
  title = "Mostrar valor",
  description = "Mostra variáveis do jogo no texto do objeto (Texto ou Texto 3D): \"Moedas: {moedas}\".",
  category = "Aparência",
  mode = "both",
  requires = { "text" },
  props = {
    { "texto", "Moedas: {moedas}", label = "Texto",
      help = "{nome} é trocado pelo valor da variável do jogo (nn.vars.nome). Ele escreve quando um valor muda: um texto posto por uma regra fica até a variável mudar." },
  },
}

local function valor(nome)
  local v = nn.vars[nome]
  if v == nil then return "" end
  if type(v) == "number" and v == math.floor(v) then return string.format("%d", v) end
  return tostring(v)
end

-- Escreve só quando o valor muda: um texto posto por uma regra ("mudar o
-- texto") fica até a variável mudar de novo.
function B:update(dt)
  local texto = self.obj.text or self.obj.text3d
  if not texto then return end
  local novo = (self.texto:gsub("{([%w_]+)}", valor))
  if novo ~= self._escrito then
    texto.text = novo
    self._escrito = novo
  end
end

-- Já certo no primeiro quadro (antes, mostrava o texto do arquivo), com o
-- que os outros comportamentos escreveram ao começar (uma contagem).
function B:start()
  self:update(0)
end

function B:after_start()
  self:update(0)
end

return B
