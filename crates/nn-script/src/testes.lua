-- Jogo sem janela: o que roda dentro do Lua para os testes de jogo
-- (`testes/*.lua`, `nn testar`) e para o `rodar_jogo` do servidor MCP.
--
-- Recebe `driver` (funções do Rust que mexem no teclado e no mouse do jogo)
-- e `M` (o módulo das cenas visuais). Devolve a tabela `D`.
--
-- O código de um teste roda numa corrotina. Para o jogo andar, ele pede ao
-- Rust com `coroutine.yield("frames", n)`, e uma captura da tela com
-- `coroutine.yield("capture", nome)`. O Rust volta com o número do quadro
-- e se o jogo fechou.

local driver, M = ...
local D = {}

local FPS = 60

-- Mostrar valores ------------------------------------------------------------

local function num(v)
  if math.type(v) == "integer" then return tostring(v) end
  if v ~= v then return "nan" end
  if v == math.huge then return "inf" end
  if v == -math.huge then return "-inf" end
  local s = ("%.2f"):format(v)
  s = s:gsub("0+$", ""):gsub("%.$", "")
  if s == "-0" then s = "0" end
  return s
end

local function is_object(t)
  return type(t) == "table" and rawget(t, "components") ~= nil and rawget(t, "name") ~= nil
end

local function fmt(v, depth)
  depth = depth or 0
  local t = type(v)
  if t == "number" then return num(v) end
  if t == "string" then return (("%q"):format(v):gsub("\\\n", "\\n")) end
  if t ~= "table" then return tostring(v) end
  if is_object(v) then
    return ("<objeto %s (id %s)%s>"):format(v.name, tostring(v.id), v.alive == false and ", destruído" or "")
  end
  if depth >= 2 then return "{...}" end
  local parts, n = {}, 0
  local len = #v
  for i = 1, math.min(len, 10) do
    table.insert(parts, fmt(v[i], depth + 1))
  end
  n = len
  local keys = {}
  for k in pairs(v) do
    if not (math.type(k) == "integer" and k >= 1 and k <= len) then
      if type(k) ~= "string" or k:sub(1, 1) ~= "_" then table.insert(keys, k) end
    end
  end
  table.sort(keys, function(a, b) return tostring(a) < tostring(b) end)
  for _, k in ipairs(keys) do
    n = n + 1
    if #parts < 12 then
      local kv = type(k) == "string" and k:match("^[%a_][%w_]*$") and k or ("[" .. fmt(k, 2) .. "]")
      table.insert(parts, kv .. " = " .. fmt(v[k], depth + 1))
    end
  end
  if n > #parts then table.insert(parts, ("... (%d no total)"):format(n)) end
  return "{ " .. table.concat(parts, ", ") .. " }"
end
D.format = fmt

-- O estado da cena -------------------------------------------------------------

local function simple(v)
  local t = type(v)
  return t == "number" or t == "string" or t == "boolean"
end

local function round(v)
  if type(v) ~= "number" or math.type(v) == "integer" then return v end
  return math.floor(v * 100 + 0.5) / 100
end

local function vars()
  local out = {}
  for k, v in pairs(nn.vars or {}) do
    if type(k) == "string" and simple(v) then out[k] = round(v) end
  end
  return out
end

-- As propriedades dos comportamentos (pelo nome do script) e o texto.
local function props(o)
  local out, seen = {}, {}
  for _, c in ipairs(o.components) do
    local inst = c._inst
    if inst then
      local key = c.script or c.type
      seen[key] = (seen[key] or 0) + 1
      if seen[key] > 1 then key = key .. "#" .. seen[key] end
      local p = {}
      for _, d in ipairs(inst._props or {}) do
        local v = inst[d.key]
        if simple(v) then p[d.key] = round(v) end
      end
      if next(p) then out[key] = p end
    elseif c.type == "text" and type(c.text) == "string" then
      out.text = { text = c.text }
    end
  end
  return out
end

--- O estado da cena da frente: os objetos (do arquivo) e as variáveis do
--- jogo. Numa cena em código, os campos simples do `self`.
function D.snapshot(scene)
  if type(scene) ~= "table" then return nil end
  local out = { vars = vars() }
  if rawget(scene, "object_list") then
    out.visual = true
    out.three_d = scene._3d and true or false
    local objects, spawned = {}, 0
    for _, o in ipairs(scene.object_list) do
      if o.alive then
        if o.id > 0 then
          local e = { id = o.id, name = o.name, x = round(o.x), y = round(o.y), visible = o.visible }
          if scene._3d then
            e.z = round(o.z)
            e.rx, e.ry, e.rz = round(o.rx), round(o.ry), round(o.rz)
          else
            e.rotation = round(o.rotation)
          end
          e.props = props(o)
          table.insert(objects, e)
        else
          spawned = spawned + 1
        end
      end
    end
    out.objects = objects
    out.spawned = spawned
  else
    local fields = {}
    for k, v in pairs(scene) do
      if type(k) == "string" and k:sub(1, 1) ~= "_" then
        if simple(v) then
          fields[k] = round(v)
        elseif type(v) == "table" and not is_object(v) and getmetatable(v) == nil then
          -- Uma tabela pequena de valores simples ({ x = 10, y = 20 }).
          local small, n = {}, 0
          for k2, v2 in pairs(v) do
            n = n + 1
            if n > 12 or not simple(v2) or type(k2) ~= "string" then small = nil break end
            small[k2] = round(v2)
          end
          if small and n > 0 then fields[k] = small end
        end
      end
    end
    out.fields = fields
  end
  return out
end

-- Regras que dispararam -------------------------------------------------------

local function object_name(scene, id)
  for _, o in ipairs(scene.object_list or {}) do
    if tostring(o.id) == id then return o.name end
  end
  return "objeto " .. id
end

local function describe(scene, key)
  local owner, index = key:match("^(.-):(%d+)$")
  index = tonumber(index)
  local where = owner == "c" and "da cena" or ("de " .. object_name(scene, owner))
  local text = ("regra %d %s"):format(index or 0, where)
  for _, ctx in ipairs(scene._event_ctx or {}) do
    if ctx.key == owner then
      for _, rule in ipairs(ctx.rules) do
        if rule.index == index then
          local when, acts = {}, {}
          for _, w in ipairs(rule.when) do
            table.insert(when, (w.nao and "não " or "") .. w.def.id)
          end
          for _, a in ipairs(rule.actions) do table.insert(acts, a.def.id) end
          return text .. ": quando " .. table.concat(when, " e ")
            .. " → " .. (#acts > 0 and table.concat(acts, ", ") or "nada")
        end
      end
    end
  end
  return text
end

--- As regras dos eventos que dispararam desde a última chamada.
function D.fired(scene)
  if type(scene) ~= "table" or not rawget(scene, "_fired") or not M.events then return nil end
  if next(scene._fired) == nil then return nil end
  local out = {}
  for _, k in ipairs(M.events.take_fired(scene)) do
    table.insert(out, { key = k, text = describe(scene, k) })
  end
  return out
end

-- Rodar código com `self` = a cena da frente ----------------------------------

local function chunk(scene, code, name)
  local env = setmetatable({ self = scene }, { __index = _G, __newindex = _G })
  local f = load("return " .. code, "=" .. name, "t", env)
  if f then return f end
  local err
  f, err = load(code, "=" .. name, "t", env)
  if not f then error(err, 0) end
  return f
end

--- O valor de uma expressão (ou o erro), como texto. `name` é o nome do
--- trecho nas mensagens de erro (padrão "olhar").
function D.inspect(scene, code, name)
  local ok, f = pcall(chunk, scene, code, name or "olhar")
  if not ok then return false, f end
  local r = table.pack(pcall(f))
  if not r[1] then return false, tostring(r[2]) end
  if r.n <= 1 then return true, "nil" end
  local parts = {}
  for i = 2, r.n do table.insert(parts, fmt(r[i])) end
  return true, table.concat(parts, ", ")
end

-- O `t` dos testes ------------------------------------------------------------

local T = {}
T.__index = T

local function front()
  local path, scene = driver.front()
  return path, scene
end

--- Roda `n` quadros do jogo (nada, se o jogo já fechou).
function T:frames(n)
  n = math.floor(n or 1)
  if n <= 0 or self._ended then return end
  local frame, ended = coroutine.yield("frames", n)
  self._frame = frame
  self._ended = ended
end

--- Roda o jogo por `s` segundos.
function T:wait(s)
  if type(s) ~= "number" or s < 0 then error("t:wait precisa dos segundos (um número)", 2) end
  self:frames(math.floor(s * FPS + 0.5))
end

--- Roda até o tempo `at` (segundos desde o começo do jogo).
function T:_to(at)
  local target = math.floor(at * FPS + 0.5)
  if target > self._frame then self:frames(target - self._frame) end
end

--- Segundos de jogo desde o começo.
function T:time()
  return self._frame / FPS
end

--- O jogo fechou (nn.quit)?
function T:quit()
  return self._ended and true or false
end

function T:down(key)
  driver.key(key, true)
end

function T:up(key)
  driver.key(key, false)
end

--- Aperta e solta a tecla (apertada por um quadro).
function T:press(key)
  driver.key(key, true)
  self:frames(1)
  driver.key(key, false)
  self:frames(1)
end

--- Segura a tecla por `s` segundos.
function T:hold(key, s)
  if type(s) ~= "number" then error("t:hold precisa dos segundos (um número)", 2) end
  driver.key(key, true)
  self:wait(s)
  driver.key(key, false)
end

--- Escreve um texto (para campos de texto).
function T:type(text)
  driver.type(tostring(text))
  self:frames(1)
end

--- A cena da frente (o `self` dela) e o caminho do arquivo.
function T:scene()
  local path, scene = front()
  return scene, path
end

function T:scene_path()
  return (front())
end

local function names(scene)
  local list, seen = {}, {}
  for _, o in ipairs(scene.object_list or {}) do
    if o.alive and o.id > 0 and not seen[o.name] and #list < 15 then
      seen[o.name] = true
      table.insert(list, o.name)
    end
  end
  return table.concat(list, ", ")
end

--- O objeto `name` da cena da frente (erro se não houver).
function T:find(name)
  local path, scene = front()
  if type(scene) ~= "table" or not scene.find then
    error(("a cena da frente (%s) não é uma cena visual"):format(path), 2)
  end
  local o = scene:find(name)
  if not o then
    error(("não achei o objeto \"%s\" em %s (tem: %s)"):format(tostring(name), path, names(scene)), 2)
  end
  return o
end

--- Todos os objetos com o nome (a lista pode vir vazia).
function T:find_all(name)
  local path, scene = front()
  if type(scene) ~= "table" or not scene.find_all then
    error(("a cena da frente (%s) não é uma cena visual"):format(path), 2)
  end
  return scene:find_all(name)
end

--- Troca para a cena (como nn.scene.switch) e espera ela rodar o primeiro
--- quadro (as regras "Quando começar" já rodaram).
function T:open(path, args)
  nn.scene.switch(path, args)
  self:frames(2)
  local now = front()
  if now ~= path then
    error(("a cena %s não abriu (a da frente é %s)"):format(path, tostring(now)), 2)
  end
end

-- Onde o objeto aparece na tela (pixels, y para baixo).
local function screen_of(o)
  local x, y = o:world_transform()
  if M.is_hud(o) then return M.screen_pos(o, x, y) end
  if o.scene and o.scene._3d then
    local sx, sy = nn.d3.to_screen(o:position3d())
    if not sx then error(("o objeto \"%s\" está atrás da câmera"):format(o.name), 3) end
    return sx, sy
  end
  return nn.camera.to_screen(x, y)
end
D.screen_of = screen_of

--- Clica num ponto da tela (x, y em pixels, y para baixo), num objeto ou
--- no objeto com esse nome.
function T:click(a, b, button)
  local x, y
  if type(a) == "string" then
    a = self:find(a)
  end
  if type(a) == "table" then
    x, y = screen_of(a)
    button = b
  else
    x, y = a, b
  end
  if type(x) ~= "number" or type(y) ~= "number" then
    error("t:click precisa de x, y, de um objeto ou do nome de um objeto", 2)
  end
  button = button or "left"
  driver.mouse_move(x, y)
  driver.mouse_button(button, true)
  self:frames(1)
  driver.mouse_button(button, false)
  self:frames(1)
end

--- Põe o mouse num ponto da tela (sem clicar).
function T:mouse(x, y)
  driver.mouse_move(x, y)
end

--- Move o mouse preso `dx, dy` pixels (a câmera do Jogador 3D gira), aos
-- poucos por `s` segundos (sem `s`, num quadro só).
function T:mouse_motion(dx, dy, s)
  if type(dx) ~= "number" or type(dy) ~= "number" then
    error("t:mouse_motion precisa de dx e dy (os pixels que o mouse anda)", 2)
  end
  local n = math.max(1, math.floor((s or 0) * 60 + 0.5))
  for _ = 1, n do
    driver.mouse_motion(dx / n, dy / n)
    self:frames(1)
  end
end

-- Gira a roda do mouse: `linhas` positivas sobem (a lista volta),
--- negativas descem. Com x, y, põe o mouse ali antes.
function T:scroll(linhas, x, y)
  if type(linhas) ~= "number" then error("t:scroll precisa das linhas (um número)", 2) end
  if x and y then driver.mouse_move(x, y) end
  driver.scroll(linhas)
  self:frames(1)
end

--- Guarda uma captura da tela agora.
function T:screenshot(name)
  coroutine.yield("capture", name and tostring(name) or ("captura em " .. num(self:time()) .. " s"))
end

local function fail(msg, level)
  error(msg, level + 1)
end

--- Confere: se `cond` for falso, o teste falha com a mensagem.
function T:check(cond, msg)
  if not cond then fail(msg or "a conferência falhou", 2) end
  return cond
end

--- Confere que `got` é igual a `want`.
function T:check_eq(got, want, msg)
  if got ~= want then
    fail(("%sesperava %s, veio %s"):format(msg and (msg .. ": ") or "", fmt(want), fmt(got)), 2)
  end
end

--- Confere que dois números estão perto (padrão: 1).
function T:check_near(got, want, tolerance, msg)
  tolerance = tolerance or 1
  if type(got) ~= "number" or math.abs(got - want) > tolerance then
    fail(("%sesperava %s (± %s), veio %s"):format(msg and (msg .. ": ") or "", fmt(want),
      fmt(tolerance), fmt(got)), 2)
  end
end

--- Roda o jogo até `cond()` dar verdadeiro (no máximo `s` segundos, padrão 5).
function T:wait_until(cond, s, msg)
  s = s or 5
  local limit = self._frame + math.floor(s * FPS + 0.5)
  while not cond() do
    if self._frame >= limit then
      fail(msg or ("esperou %s s e não aconteceu"):format(num(s)), 2)
    end
    self:frames(1)
  end
end

function D.new_t(frame)
  return setmetatable({ _frame = frame or 0, _ended = false }, T)
end

-- Erro dentro do teste: a mensagem e as linhas do traceback nos arquivos do
-- projeto.
local function handler(e)
  local msg = tostring(e)
  local lines = {}
  for _, l in ipairs(driver.traceback()) do
    if l:find("%.lua:%d+$") and not l:find("^nn/") and not msg:find(l .. ":", 1, true)
      and #lines < 4 then
      table.insert(lines, "  em " .. l)
    end
  end
  if #lines > 0 then msg = msg .. "\n" .. table.concat(lines, "\n") end
  return msg
end

--- O corpo da corrotina: roda `f(t, ...)` e devolve true ou false, erro.
function D.run(f, t, ...)
  local ok, err = xpcall(f, handler, t, ...)
  if ok then return true end
  return false, err
end

-- O jogo aberto (servidor MCP) ------------------------------------------------

--- As regras que dispararam desde a última olhada, juntas pelo texto:
--- { { texto, primeiro, vezes } }.
function D.live_fired(scene)
  if M.events then M.events.log_fired = true end
  if type(scene) ~= "table" then return {} end
  local log = rawget(scene, "_fired_log")
  scene._fired_log = {}
  local out, by = {}, {}
  for _, e in ipairs(log or {}) do
    local text = describe(scene, e.k)
    local item = by[text]
    if item then
      item.vezes = item.vezes + 1
    else
      item = { texto = text, primeiro = e.t, vezes = 1 }
      by[text] = item
      table.insert(out, item)
    end
  end
  return out
end

--- Muda um campo de um objeto na hora (sem gravar), como o NN Studio faz
--- com o jogo aberto. `ref`: id ou nome; `comp`: o tipo do componente, o
--- script do comportamento ("nn:plataforma", "plataforma") ou o número dele.
function D.live_set(scene, ref, field, value, comp)
  if type(scene) ~= "table" or not rawget(scene, "object_list") then
    return false, "a cena da frente não é uma cena visual (use \"lua\")"
  end
  local o
  if type(ref) == "number" then
    o = scene._by_id and scene._by_id[ref]
  else
    o = scene:find(tostring(ref))
  end
  if not o then
    return false, ("não achei o objeto %s (tem: %s)"):format(fmt(ref), names(scene))
  end
  if o.id <= 0 then
    return false, ("o objeto %s foi criado pelo jogo; mude-o com \"lua\""):format(o.name)
  end
  if type(field) ~= "string" or field == "" then return false, "falta o campo" end
  local msg = { id = o.id, field = field, value = value }
  if comp ~= nil then
    local index
    if type(comp) == "number" then
      index = o.components[comp] and comp
    else
      local want = tostring(comp)
      for i, c in ipairs(o.components) do
        local script = c.script or ""
        if c.type == want or script == want or script == "nn:" .. want then
          index = i
          break
        end
      end
    end
    if not index then
      local have = {}
      for _, c in ipairs(o.components) do table.insert(have, c.script or c.type) end
      return false, ("o objeto %s não tem o componente %s (tem: %s)"):format(o.name, fmt(comp),
        table.concat(have, ", "))
    end
    msg.component = index
  end
  M.live(scene, msg)
  return true, ("%s.%s%s = %s"):format(o.name, comp and (tostring(comp) .. ".") or "", field, fmt(value))
end

-- Arquivos de testes ----------------------------------------------------------

--- Um `test(nome, função)` que guarda os testes na lista `list`.
function D.collector(list)
  return function(name, f)
    if type(name) ~= "string" or type(f) ~= "function" then
      error("use test(\"nome do teste\", function(t) ... end)", 2)
    end
    table.insert(list, { name = name, fn = f })
  end
end

-- O roteiro do rodar_jogo -----------------------------------------------------

--- Roda as ações do roteiro, cada uma no seu tempo, até `duration`.
function D.plan(t, plan)
  local evs = {}
  for i, a in ipairs(plan.actions or {}) do
    a._i = i
    table.insert(evs, a)
  end
  local seq = #evs
  local function sort()
    table.sort(evs, function(a, b)
      if a.at ~= b.at then return a.at < b.at end
      return a._i < b._i
    end)
  end
  sort()
  while #evs > 0 do
    local a = table.remove(evs, 1)
    t:_to(a.at)
    if t:quit() then break end
    local op = a.op
    if op == "press" then
      t:press(a.key)
    elseif op == "hold" then
      t:down(a.key)
      seq = seq + 1
      table.insert(evs, { at = a.at + a.dur, op = "up", key = a.key, _i = seq })
      sort()
    elseif op == "down" then
      t:down(a.key)
    elseif op == "up" then
      t:up(a.key)
    elseif op == "click" then
      if a.name then t:click(a.name, a.button) else t:click(a.x, a.y, a.button) end
    elseif op == "move" then
      t:mouse(a.x, a.y)
    elseif op == "scroll" then
      t:scroll(a.lines)
    elseif op == "mouse_motion" then
      t:mouse_motion(a.x, a.y, a.dur)
    elseif op == "type" then
      t:type(a.text)
    elseif op == "capture" then
      t:screenshot(a.label)
    elseif op == "lua" then
      local _, scene = front()
      local ok, err = pcall(function() chunk(scene, a.code, "lua")() end)
      if not ok then error(("a ação lua em %s s deu erro: %s"):format(num(a.at), tostring(err)), 0) end
    end
  end
  if not t:quit() then t:_to(plan.duration) end
end

return D
