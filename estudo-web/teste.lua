-- Testes de compatibilidade: o que os jogos da nn-engine usam do Lua 5.4.
-- Devolve um texto com uma linha por teste ("ok" ou "FALHA").
local out = {}
local function t(nome, f)
  local ok, err = pcall(f)
  out[#out + 1] = (ok and "ok    " or "FALHA ") .. nome .. (ok and "" or (": " .. tostring(err)))
end

t("inteiros e //", function()
  assert(assert(load("return 7 // 2"))() == 3)
  assert(math.type(1) == "integer" and math.type(1.0) == "float")
end)
t("goto (via load)", function()
  local f = assert(load("local i = 0 ::de_novo:: i = i + 1 if i < 3 then goto de_novo end return i"))
  assert(f() == 3)
end)
t("string.format", function()
  assert(string.format("%5.2f|%d|%s|%x", 3.14159, 42, "x", 255) == " 3.14|42|x|ff")
end)
t("padroes (gsub/find/match)", function()
  local s = ("a1b22c333"):gsub("%d+", function(d) return "<" .. #d .. ">" end)
  assert(s == "a<1>b<2>c<3>")
  assert(("cenas/fase.cena"):match("^(.-)%.(%a+)$") == "cenas/fase")
  assert(("x = 10"):find("=", 1, true) == 3)
end)
t("table.sort/insert/remove/concat", function()
  local v = { 5, 2, 8, 1 }
  table.sort(v, function(a, b) return a > b end)
  table.insert(v, 1, 9)
  assert(table.remove(v) == 1)
  assert(table.concat(v, ",") == "9,8,5,2")
end)
t("table.unpack/pack", function()
  local p = table.pack(1, nil, 3)
  assert(p.n == 3 and select(3, table.unpack(p, 1, p.n)) == 3)
end)
t("corrotinas", function()
  local co = coroutine.wrap(function(a)
    local b = coroutine.yield(a + 1)
    return b * 2
  end)
  assert(co(1) == 2 and co(5) == 10)
end)
t("metatabelas", function()
  local V = {}
  V.__index = V
  V.__add = function(a, b) return setmetatable({ x = a.x + b.x }, V) end
  function V:len() return self.x end
  local a = setmetatable({ x = 1 }, V) + setmetatable({ x = 2 }, V)
  assert(a:len() == 3)
end)
t("pcall/error com tabela", function()
  local ok, e = pcall(error, { codigo = 7 })
  assert(not ok and e.codigo == 7)
end)
t("load com <const>", function()
  local f = assert(load("local x <const> = 5; return x * 2"))
  assert(f() == 10)
end)
t("utf8", function()
  assert(utf8.char(227) == "ã" and utf8.len("ação") == 4)
end)
t("string.pack", function()
  assert(#string.pack("i4", 7) == 4)
end)
t("math", function()
  assert(math.floor(3.7) == 3 and math.max(1, 5, 3) == 5 and math.tointeger(3.0) == 3)
  assert(math.random(1, 1) == 1 and math.huge > 1e300)
end)
t("os.clock/os.time", function()
  assert(type(os.clock()) == "number" and type(os.time()) == "number")
end)
t("select e ...", function()
  local function f(...) return select("#", ...), select(2, ...) end
  local n, b = f(1, nil, 3)
  assert(n == 3 and b == nil)
end)
t("tostring", function()
  assert(tostring(1.5) == "1.5" and tostring(10) == "10" and tostring(2 ^ 53) == "9.007199254741e+15")
end)
t("string.rep/sub/byte/upper", function()
  assert(("ab"):rep(3, "-") == "ab-ab-ab" and ("nn-engine"):sub(-6) == "engine")
  assert(("A"):byte() == 65 and ("ação"):upper() == "AçãO")
end)
t("ipairs/pairs/next", function()
  local n = 0
  for _, v in ipairs({ 1, 2, 3 }) do n = n + v end
  for k in pairs({ a = 1, b = 2 }) do n = n + #k end
  assert(n == 8 and next({}) == nil)
end)
t("<close> (sintaxe 5.4, via load)", function()
  local f = assert(load("local fechou = false; do local x <close> = setmetatable({}, { __close = function() fechou = true end }) end; return fechou"))
  assert(f() == true)
end)
t("rawget/rawset/rawequal/rawlen", function()
  local m = setmetatable({}, { __index = function() return 1 end })
  assert(m.x == 1 and rawget(m, "x") == nil and rawlen({ 1, 2 }) == 2)
end)

-- Sem table.concat (nem todo Lua tem).
local texto = ""
for i = 1, #out do texto = texto .. out[i] .. "\n" end
return texto
