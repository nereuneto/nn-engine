-- Eventos "Quando... Então..." das cenas visuais.
--
-- Uma regra tem condições (Quando) e ações (Então). Ela pertence à cena
-- (campo `events` do .cena) ou a um objeto (campo `events` do objeto, que vai
-- junto quando o objeto é um prefab):
--
--   {
--     when = { { cond = "encostar", quem = "@este", em = "chave" } },
--     actions = { { act = "variavel", var = "chaves", op = "somar", valor = 1 },
--                 { act = "destruir", alvo = "@outro" } },
--   }
--
-- Campos opcionais da regra: `mode = "enquanto"` (roda a cada quadro enquanto
-- as condições valem; o padrão, "quando", roda uma vez quando elas passam a
-- valer, ou a cada vez que um gatilho acontece), `off = true` (desligada) e
-- `comment = "..."` (uma linha de comentário, sem condições nem ações).
--
-- Quem é quem: "@este" é o objeto dono da regra (numa regra da cena, o
-- jogador), "@outro" é o outro objeto da condição (em quem encostou, o item
-- pego...), "@jogador" são os objetos com comportamento de jogador e
-- qualquer outro texto é o nome de um objeto (todos com esse nome).

local M = ...
local E = {}

-- Teclas -------------------------------------------------------------------

-- Grupos de teclas que valem para teclado e controle.
local KEY_GROUPS = {
  pular = { keys = { "space", "up", "w" }, pad = { "a" } },
  confirmar = { keys = { "enter", "space" }, pad = { "a", "start" } },
  voltar = { keys = { "escape", "backspace" }, pad = { "b", "back" } },
  esquerda = { keys = { "left", "a" }, pad = { "left" } },
  direita = { keys = { "right", "d" }, pad = { "right" } },
  cima = { keys = { "up", "w" }, pad = { "up" } },
  baixo = { keys = { "down", "s" }, pad = { "down" } },
  acao = { keys = { "x", "j" }, pad = { "x" } },
}

E.KEYS = {
  { "pular", "pular (espaço, seta para cima, W, A do controle)" },
  { "confirmar", "confirmar (Enter, espaço, A do controle)" },
  { "voltar", "voltar (Esc, B do controle)" },
  { "acao", "ação (X, J, X do controle)" },
  { "esquerda", "esquerda (seta, A, controle)" },
  { "direita", "direita (seta, D, controle)" },
  { "cima", "cima (seta, W, controle)" },
  { "baixo", "baixo (seta, S, controle)" },
  { "qualquer", "qualquer tecla" },
  { "space", "espaço" }, { "enter", "Enter" }, { "escape", "Esc" }, { "shift", "Shift" },
  { "ctrl", "Ctrl" }, { "tab", "Tab" },
  { "up", "seta para cima" }, { "down", "seta para baixo" }, { "left", "seta para a esquerda" },
  { "right", "seta para a direita" },
}
for c in ("abcdefghijklmnopqrstuvwxyz0123456789"):gmatch(".") do
  table.insert(E.KEYS, { c, c:upper() })
end

local function key_state(name, how)
  local fk = how == "apertar" and nn.input.pressed
    or how == "soltar" and nn.input.released or nn.input.down
  local fp = how == "apertar" and nn.input.pad_pressed
    or how == "soltar" and nn.input.pad_released or nn.input.pad_down
  if name == "qualquer" then
    return how == "apertar" and nn.input.any_pressed() or false
  end
  local g = KEY_GROUPS[name]
  if not g then return fk(name) end
  for _, k in ipairs(g.keys) do
    if fk(k) then return true end
  end
  for _, b in ipairs(g.pad) do
    if fp(b) then return true end
  end
  return false
end

-- Quem é quem ----------------------------------------------------------------

local function alive_list(list)
  local out = {}
  for _, o in ipairs(list) do
    if o.alive then table.insert(out, o) end
  end
  return out
end

--- Objetos que um campo "quem/alvo" quer dizer, para a ligação `b`.
local function resolve(scene, sel, b, keep_dead)
  if sel == nil or sel == "" or sel == "@jogador" then
    return M.targets(scene, "")
  elseif sel == "@este" then
    if b.este then
      if b.este.alive or keep_dead then return { b.este } end
      return {}
    end
    return M.targets(scene, "")
  elseif sel == "@outro" then
    if b.outro and (b.outro.alive or keep_dead) then return { b.outro } end
    return {}
  end
  return alive_list(scene:find_all(sel))
end

-- Texto com {variavel} trocado pelo valor.
local function fill(text)
  return (tostring(text):gsub("{([%w_]+)}", function(k)
    local v = nn.vars[k]
    if v == nil then return "{" .. k .. "}" end
    if type(v) == "number" and v == math.floor(v) then return tostring(math.floor(v)) end
    return tostring(v)
  end))
end

-- Número de um campo que pode ser texto com {variavel}.
local function number(v)
  if type(v) == "number" then return v end
  return tonumber(fill(v or "")) or 0
end

local function is_hud(o)
  return M.is_hud(o)
end
E.is_hud = is_hud

--- O ponto (do mundo, ou da tela para objetos de HUD) está no objeto?
function E.hit(o, wx, wy, sx, sy)
  local x, y = o:world_transform()
  local hw, hh = o:bounds()
  if is_hud(o) then
    if not sx then return false end
    local px, py = M.screen_pos(o, x, y)
    return math.abs(sx - px) <= hw and math.abs(sy - py) <= hh
  end
  if not wx then return false end
  return math.abs(wx - x) <= hw and math.abs(wy - y) <= hh
end

local function on_screen(o)
  if is_hud(o) then return true end
  local x0, y0, x1, y1 = nn.camera.view()
  local x, y = o:world_transform()
  local hw, hh = o:bounds()
  return x + hw >= x0 and x - hw <= x1 and y + hh >= y0 and y - hh <= y1
end

-- Sinais: coisas que aconteceram neste quadro (mensagens, objetos destruídos)

function E.signal(scene, s)
  if not scene._signals then return end
  table.insert(scene._signals, s)
end

local function signals(scene, kind, msg)
  local out = {}
  for _, s in ipairs(scene._cur_signals or {}) do
    if s.kind == kind and (msg == nil or s.msg == msg) then table.insert(out, s) end
  end
  return out
end

-- Liga o resultado de uma condição a objetos: `m.este` (quem fez) e
-- `m.outro` (com quem).
local function matches_from(list, quem_set)
  local out = {}
  for _, m in ipairs(list) do
    if quem_set == nil or quem_set[m.este] then table.insert(out, m) end
  end
  return out
end

local function set_of(list)
  local s = {}
  for _, o in ipairs(list) do s[o] = true end
  return s
end

local function pair_key(a, b)
  return tostring(a) .. "|" .. tostring(b)
end

-- Catálogo -------------------------------------------------------------------
--
-- Cada condição e cada ação tem um texto com {campos} (o NN Studio põe os
-- campos no meio da frase), as propriedades (no mesmo formato dos
-- comportamentos) e, para "Ver como código", um modelo do Lua equivalente.
-- Condições com `trigger = true` são gatilhos: acontecem num instante.

local WHO = { type = "target", label = "Quem" }
local function who(key, default, label, extra)
  local p = { key, default or "@este", type = "target", label = label or WHO.label }
  for k, v in pairs(extra or {}) do p[k] = v end
  return p
end

local COMPARE = {
  { "==", "=" }, { "~=", "≠" }, { "<", "<" }, { ">", ">" }, { "<=", "≤" }, { ">=", "≥" },
}

E.CONDITIONS = {
  -- Cena e tempo
  {
    id = "comecou", name = "A cena (ou o objeto) começa", category = "Cena e tempo", trigger = true,
    text = "começar",
    help = "Na regra da cena: quando a cena começa. Na de um objeto: quando ele aparece.",
    lua = "-- em enter()",
    test = function(ctx) return not ctx.started end,
  },
  {
    id = "a_cada", name = "A cada N segundos", category = "Cena e tempo", trigger = true,
    text = "a cada {segundos} segundos",
    props = { { "segundos", 1, min = 0.05 } },
    lua = "-- a cada {segundos} s (conte o tempo em update)",
    tick = function(ctx, p, st, dt)
      st.t = (st.t or 0) + dt
      st.fire = false
      local every = math.max(0.05, number(p.segundos))
      if st.t >= every then
        st.t = st.t - every
        st.fire = true
      end
    end,
    test = function(ctx, p, st) return st.fire end,
  },
  {
    id = "depois_de", name = "Depois de N segundos", category = "Cena e tempo", trigger = true,
    text = "passarem {segundos} segundos",
    help = "Uma vez, depois desse tempo (contando de quando a cena ou o objeto começou).",
    props = { { "segundos", 2, min = 0 } },
    lua = "-- depois de {segundos} s",
    tick = function(ctx, p, st, dt)
      st.t = (st.t or 0) + dt
      st.fire = not st.done and st.t >= number(p.segundos)
      if st.fire then st.done = true end
    end,
    test = function(ctx, p, st) return st.fire end,
  },
  {
    id = "chance", name = "Sorte (chance em %)", category = "Cena e tempo",
    text = "der sorte ({chance}% de chance)",
    help = "Sorteia toda vez que a regra é conferida.",
    props = { { "chance", 50, min = 0, max = 100 } },
    lua = "math.random() * 100 < {chance}",
    test = function(ctx, p) return math.random() * 100 < number(p.chance) end,
  },
  -- Controles
  {
    id = "tecla", name = "Tecla ou botão", category = "Controles",
    trigger = function(p) return p.modo ~= "segurar" end,
    text = "{modo} {tecla}",
    props = {
      { "modo", "apertar", options = { { "apertar", "apertar" }, { "soltar", "soltar" },
        { "segurar", "estiver segurando" } } },
      { "tecla", "pular", type = "choice", options = E.KEYS },
    },
    lua = "nn.input.{modo}(\"{tecla}\")",
    test = function(ctx, p) return key_state(p.tecla, p.modo) end,
  },
  {
    id = "clicar", name = "Clique no objeto", category = "Controles", trigger = true,
    text = "clicar em {quem}",
    help = "Clique do mouse no objeto. Nos botões e caixas de marcar, também Enter, espaço ou o A do controle; num campo de texto, Enter.",
    props = { who("quem") },
    lua = "nn.input.mouse_pressed(\"left\") -- em cima de {quem}",
    test = function(ctx, p, st, b)
      local out = {}
      local list = resolve(ctx.scene, p.quem, b)
      local pressed = signals(ctx.scene, "mensagem", "clicou")
      local by_signal = {}
      for _, s in ipairs(pressed) do by_signal[s.obj] = true end
      local clicked = ctx.scene._clicked or {}
      for _, o in ipairs(list) do
        if clicked[o] or by_signal[o] then table.insert(out, { este = o }) end
      end
      return out
    end,
  },
  {
    id = "mudou", name = "Mudar de valor", category = "Controles", trigger = true,
    text = "{quem} mudar de valor",
    help = "Um deslizador, uma caixa de marcar ou um campo de texto mudou (pelo jogador).",
    props = { who("quem") },
    lua = "-- o valor de {quem} mudou",
    test = function(ctx, p, st, b)
      local out = {}
      local list = resolve(ctx.scene, p.quem, b)
      local hit = {}
      for _, s in ipairs(signals(ctx.scene, "mudou")) do hit[s.obj] = true end
      for _, o in ipairs(list) do
        if hit[o] then table.insert(out, { este = o }) end
      end
      return out
    end,
  },
  -- Toques
  {
    id = "encostar", name = "Encostar em", category = "Toques", trigger = true,
    text = "{quem} encostar em {em}",
    help = "No momento em que os dois começam a se tocar.",
    props = { who("quem"), who("em", "@jogador", "Em") },
    lua = "{quem}:touching({em}) -- só quando começa a encostar",
    tick = function(ctx, p, st)
      local b0 = ctx.base
      local cur, new = {}, {}
      for _, a in ipairs(resolve(ctx.scene, p.quem, b0)) do
        for _, o in ipairs(resolve(ctx.scene, p.em, b0)) do
          if a:touching(o) then
            local k = pair_key(a, o)
            cur[k] = true
            if not (st.prev and st.prev[k]) then table.insert(new, { este = a, outro = o }) end
          end
        end
      end
      st.prev, st.new = cur, new
    end,
    test = function(ctx, p, st) return st.new or {} end,
  },
  {
    id = "desencostar", name = "Parar de encostar", category = "Toques", trigger = true,
    text = "{quem} parar de encostar em {em}",
    props = { who("quem"), who("em", "@jogador", "Em") },
    lua = "not {quem}:touching({em}) -- só quando para de encostar",
    tick = function(ctx, p, st)
      local b0 = ctx.base
      local cur, pairs_now = {}, {}
      for _, a in ipairs(resolve(ctx.scene, p.quem, b0)) do
        for _, o in ipairs(resolve(ctx.scene, p.em, b0)) do
          if a:touching(o) then
            local k = pair_key(a, o)
            cur[k] = true
            pairs_now[k] = { este = a, outro = o }
          end
        end
      end
      local left = {}
      for k, m in pairs(st.prev_pairs or {}) do
        if not cur[k] and m.este.alive and m.outro.alive then table.insert(left, m) end
      end
      st.prev_pairs, st.left = pairs_now, left
    end,
    test = function(ctx, p, st) return st.left or {} end,
  },
  {
    id = "encostando", name = "Estiver encostando", category = "Toques",
    text = "{quem} estiver encostando em {em}",
    props = { who("quem"), who("em", "@jogador", "Em") },
    lua = "{quem}:touching({em})",
    test = function(ctx, p, st, b)
      local out = {}
      for _, a in ipairs(resolve(ctx.scene, p.quem, b)) do
        for _, o in ipairs(resolve(ctx.scene, p.em, b)) do
          if a:touching(o) then table.insert(out, { este = a, outro = o }) end
        end
      end
      return out
    end,
  },
  {
    mode = "2d",
    id = "tile", name = "Encostar num tile", category = "Toques",
    text = "{quem} estiver encostando num tile {tipo}",
    help = "Tiles de um mapa com esse tipo (marcado no tileset: \"lava\", \"agua\", \"tijolo\"). Vazio = qualquer tile. Com Quando, acontece ao começar a encostar.",
    props = { who("quem"), { "tipo", "lava", type = "tiletype", label = "Tipo", allow_empty = "qualquer" } },
    lua = "-- {quem}:tiles_touching() com properties.tipo == \"{tipo}\"",
    test = function(ctx, p, st, b)
      local out = {}
      for _, o in ipairs(resolve(ctx.scene, p.quem, b)) do
        local hit = {}
        for _, tl in ipairs(o:tiles_touching()) do
          local kind = tl.properties and tl.properties.tipo
          if p.tipo == "" or kind == p.tipo then table.insert(hit, tl) end
        end
        if #hit > 0 then table.insert(out, { este = o, tiles = hit }) end
      end
      return out
    end,
  },
  {
    id = "perto", name = "Estiver perto de", category = "Toques",
    text = "{quem} estiver a menos de {distancia} de {alvo}",
    props = { who("quem"), { "distancia", 200, min = 0, default3d = 3 }, who("alvo", "@jogador", "De") },
    lua = "distancia({quem}, {alvo}) < {distancia}",
    test = function(ctx, p, st, b)
      local out = {}
      local d = number(p.distancia)
      if ctx.scene._3d then
        for _, a in ipairs(resolve(ctx.scene, p.quem, b)) do
          local ax, ay, az = a:position3d()
          for _, o in ipairs(resolve(ctx.scene, p.alvo, b)) do
            if o ~= a then
              local ox, oy, oz = o:position3d()
              local dx, dy, dz = ox - ax, oy - ay, oz - az
              if dx * dx + dy * dy + dz * dz < d * d then table.insert(out, { este = a, outro = o }) end
            end
          end
        end
        return out
      end
      for _, a in ipairs(resolve(ctx.scene, p.quem, b)) do
        local ax, ay = a:world_transform()
        for _, o in ipairs(resolve(ctx.scene, p.alvo, b)) do
          if o ~= a then
            local ox, oy = o:world_transform()
            local dx, dy = ox - ax, oy - ay
            if dx * dx + dy * dy < d * d then table.insert(out, { este = a, outro = o }) end
          end
        end
      end
      return out
    end,
  },
  -- Variáveis
  {
    id = "variavel", name = "Comparar variável", category = "Variáveis",
    text = "a variável {var} for {op} {valor}",
    props = {
      { "var", "moedas", type = "var", label = "Variável" },
      { "op", ">=", options = COMPARE },
      { "valor", 10, type = "value", label = "Valor" },
    },
    lua = "nn.vars.{var} {op} {valor}",
    test = function(ctx, p)
      local a = nn.vars[p.var]
      local v = p.valor
      if type(v) == "string" then v = fill(v) end
      local na, nb = tonumber(a), tonumber(v)
      local op = p.op
      if na and nb then
        a, v = na, nb
      elseif op ~= "==" and op ~= "~=" then
        return false
      else
        a, v = tostring(a), tostring(v)
        if a == "nil" then a = "" end
      end
      if op == "==" then return a == v
      elseif op == "~=" then return a ~= v
      elseif op == "<" then return a < v
      elseif op == ">" then return a > v
      elseif op == "<=" then return a <= v
      else return a >= v end
    end,
  },
  -- Objetos
  {
    id = "acabaram", name = "Não sobrar nenhum", category = "Objetos",
    text = "não sobrar nenhum {nome}",
    help = "Todos os objetos com esse nome sumiram (pegou todas as moedas, venceu todos os inimigos...).",
    props = { { "nome", "", type = "object", label = "Nome" } },
    lua = "#self:find_all(\"{nome}\") == 0",
    test = function(ctx, p)
      if p.nome == "" then return false end
      return #alive_list(ctx.scene:find_all(p.nome)) == 0
    end,
  },
  {
    id = "destruido", name = "Objeto destruído", category = "Objetos", trigger = true,
    text = "{quem} for destruído",
    props = { who("quem") },
    lua = "-- em destroyed() do comportamento",
    test = function(ctx, p, st, b)
      local out = {}
      for _, s in ipairs(signals(ctx.scene, "destruido")) do
        local o = s.obj
        local ok
        if p.quem == "@este" then
          ok = (b.este and b.este == o) or (not b.este and o._player)
        elseif p.quem == "@jogador" or p.quem == "" then
          ok = o._player
        elseif p.quem == "@outro" then
          ok = b.outro == o
        else
          ok = o.name == p.quem
        end
        if ok then table.insert(out, { este = o }) end
      end
      return out
    end,
  },
  {
    mode = "2d",
    id = "saiu_da_tela", name = "Sair da tela", category = "Objetos", trigger = true,
    text = "{quem} sair da tela",
    props = { who("quem") },
    lua = "-- fora de nn.camera.view()",
    tick = function(ctx, p, st)
      local was = st.inside or {}
      local now, out = {}, {}
      for _, o in ipairs(resolve(ctx.scene, p.quem, ctx.base)) do
        local inside = on_screen(o)
        now[o] = inside
        if was[o] == true and not inside then table.insert(out, { este = o }) end
      end
      st.inside, st.out = now, out
    end,
    test = function(ctx, p, st) return st.out or {} end,
  },
  {
    id = "no_chao", name = "No chão ou no ar", category = "Objetos",
    text = "{quem} estiver {onde}",
    props = { who("quem"), { "onde", "chao", options = { { "chao", "no chão" }, { "ar", "no ar" } } } },
    lua = "{quem}.body.on_ground",
    test = function(ctx, p, st, b)
      local out = {}
      for _, o in ipairs(resolve(ctx.scene, p.quem, b)) do
        local ground = o.body and o.body.alive and o.body.on_ground or false
        if ground == (p.onde == "chao") then table.insert(out, { este = o }) end
      end
      return out
    end,
  },
  {
    id = "visivel", name = "Visível ou escondido", category = "Objetos",
    text = "{quem} estiver {estado}",
    props = { who("quem"), { "estado", "visivel", options = { { "visivel", "visível" }, { "escondido", "escondido" } } } },
    lua = "{quem}.visible",
    test = function(ctx, p, st, b)
      local out = {}
      for _, o in ipairs(resolve(ctx.scene, p.quem, b)) do
        if o.visible == (p.estado == "visivel") then table.insert(out, { este = o }) end
      end
      return out
    end,
  },
  -- Mensagens
  {
    id = "pegar", name = "Pegar um coletável", category = "Mensagens", trigger = true,
    text = "{quem} pegar {item}",
    help = "Um Coletável foi pego. Vazio = qualquer um.",
    props = { who("quem"), { "item", "", type = "object", label = "Item", allow_empty = "qualquer um" } },
    lua = "-- coletou(item) no comportamento",
    test = function(ctx, p, st, b)
      local out = {}
      local who_set = set_of(resolve(ctx.scene, p.quem, b, true))
      for _, s in ipairs(signals(ctx.scene, "mensagem", "coletou")) do
        local item = s.arg
        if who_set[s.obj] and (p.item == "" or (type(item) == "table" and item.name == p.item)) then
          table.insert(out, { este = s.obj, outro = type(item) == "table" and item or nil })
        end
      end
      return out
    end,
  },
  {
    mode = "2d",
    id = "particulas_terminaram", name = "Partículas terminarem", category = "Objetos", trigger = true,
    text = "as partículas de {quem} terminarem",
    help = "Uma explosão acabou (ou um emissor desligado ficou sem partículas).",
    props = { who("quem") },
    lua = "{quem}.particles.finished",
    test = function(ctx, p, st, b)
      local set = set_of(resolve(ctx.scene, p.quem, b, true))
      local out = {}
      for _, s in ipairs(signals(ctx.scene, "particulas")) do
        if set[s.obj] then table.insert(out, { este = s.obj }) end
      end
      return out
    end,
  },
  {
    id = "animacao_terminou", name = "Animação terminar", category = "Objetos", trigger = true,
    text = "{quem} terminar a animação {nome}",
    help = "Só animações que tocam uma vez (modo \"uma vez\" no editor de animação). Vazio = qualquer uma.",
    help3d = "Só animações que tocam uma vez (Repetir desligado). Vazio = qualquer uma.",
    props = { who("quem"), { "nome", "", type = "anim", label = "Animação", allow_empty = "qualquer uma" } },
    lua = "{quem}.sprite:finished()",
    test = function(ctx, p, st, b)
      local set = set_of(resolve(ctx.scene, p.quem, b, true))
      local out = {}
      for _, s in ipairs(signals(ctx.scene, "mensagem", "animacao_terminou")) do
        if set[s.obj] and (p.nome == "" or s.arg == p.nome) then
          table.insert(out, { este = s.obj })
        end
      end
      return out
    end,
  },
  {
    id = "levou_dano", name = "Levar dano", category = "Mensagens", trigger = true,
    text = "{quem} levar dano",
    help = "Precisa do comportamento Vida.",
    props = { who("quem") },
    lua = "-- dano() no comportamento Vida",
    test = function(ctx, p, st, b)
      local set = set_of(resolve(ctx.scene, p.quem, b, true))
      local out = {}
      for _, s in ipairs(signals(ctx.scene, "mensagem", "levou_dano")) do
        if set[s.obj] then table.insert(out, { este = s.obj }) end
      end
      return out
    end,
  },
  {
    id = "vida_acabou", name = "Vida acabar", category = "Mensagens", trigger = true,
    text = "a vida de {quem} acabar",
    help = "Precisa do comportamento Vida.",
    props = { who("quem") },
    lua = "-- acabou() no comportamento Vida",
    test = function(ctx, p, st, b)
      local set = set_of(resolve(ctx.scene, p.quem, b, true))
      local out = {}
      for _, s in ipairs(signals(ctx.scene, "mensagem", "vida_acabou")) do
        if set[s.obj] then table.insert(out, { este = s.obj }) end
      end
      return out
    end,
  },
  {
    id = "mensagem", name = "Receber mensagem", category = "Mensagens", trigger = true,
    text = "{quem} receber a mensagem {msg}",
    help = "Mensagens vêm da ação \"Mandar mensagem\", de comportamentos e de scripts (obj:send(\"nome\")).",
    props = { who("quem"), { "msg", "abrir", label = "Mensagem" } },
    lua = "function B:{msg}() ... end",
    test = function(ctx, p, st, b)
      local set = set_of(resolve(ctx.scene, p.quem, b, true))
      local out = {}
      for _, s in ipairs(signals(ctx.scene, "mensagem", p.msg)) do
        if set[s.obj] then
          table.insert(out, { este = s.obj, outro = type(s.arg) == "table" and s.arg.alive ~= nil and s.arg or nil })
        end
      end
      return out
    end,
  },
}

local VAR_OPS = {
  { "somar", "somar" }, { "subtrair", "subtrair" }, { "definir", "mudar para" },
  { "maior", "ficar com o maior entre ela e" }, { "menor", "ficar com o menor entre ela e" },
}

local function each(ctx, sel, b, f)
  for _, o in ipairs(resolve(ctx.scene, sel, b)) do f(o) end
end

local function set_velocity(o, vx, vy, add, vz)
  local b = o.body
  if b and b.alive then
    if vz then
      -- 3D (o personagem também: vy é o pulo, vx e vz um empurrão).
      if add then
        b:set_velocity(b.vx + vx, b.vy + vy, b.vz + vz)
      else
        b:set_velocity(vx, vy, vz)
      end
      if b.is_character and (vx ~= 0 or vz ~= 0) then o.empurrado = 0.3 end
    elseif add then
      b.vx, b.vy = b.vx + vx, b.vy + vy
    else
      b.vx, b.vy = vx, vy
    end
  else
    if add then
      o.vx, o.vy = (o.vx or 0) + vx, (o.vy or 0) + vy
      if vz then o.vz = (o.vz or 0) + vz end
    else
      o.vx, o.vy = vx, vy
      if vz then o.vz = vz end
    end
  end
end

E.ACTIONS = {
  -- Variáveis
  {
    id = "variavel", name = "Mudar variável", category = "Variáveis",
    text = "variável {var}: {op} {valor}",
    props = {
      { "var", "moedas", type = "var", label = "Variável" },
      { "op", "somar", options = VAR_OPS },
      { "valor", 1, type = "value", label = "Valor",
        help = "Um número, um texto ou outra variável entre chaves: {pontos}." },
    },
    lua = "nn.vars.{var} = ... -- {op} {valor}",
    run = function(ctx, p)
      local cur = nn.vars[p.var]
      local v = p.valor
      if type(v) == "string" then
        v = fill(v)
        v = tonumber(v) or v
      end
      if p.op == "definir" then
        nn.vars[p.var] = v
        return
      end
      local a, n = tonumber(cur) or 0, tonumber(v) or 0
      if p.op == "somar" then nn.vars[p.var] = a + n
      elseif p.op == "subtrair" then nn.vars[p.var] = a - n
      elseif p.op == "maior" then nn.vars[p.var] = math.max(a, n)
      elseif p.op == "menor" then nn.vars[p.var] = math.min(a, n) end
    end,
  },
  {
    id = "salvar", name = "Salvar no disco", category = "Variáveis",
    text = "salvar no disco: {var}",
    help = "Guarda a variável (ou todas) para a próxima vez que o jogo abrir.",
    props = { { "var", "", type = "var", label = "Variável", allow_empty = "todas" } },
    lua = "nn.save.set(\"{var}\", nn.vars.{var})",
    run = function(ctx, p)
      if p.var == "" then
        for k, v in pairs(nn.vars) do
          if type(k) == "string" then nn.save.set("vars." .. k, v) end
        end
      else
        nn.save.set("vars." .. p.var, nn.vars[p.var])
      end
      nn.save.flush()
    end,
  },
  {
    id = "carregar", name = "Carregar do disco", category = "Variáveis",
    text = "carregar do disco: {var}",
    help = "Lê o que foi salvo (se ainda não tiver nada salvo, a variável fica como está).",
    props = { { "var", "", type = "var", label = "Variável", allow_empty = "todas" } },
    lua = "nn.vars.{var} = nn.save.get(\"{var}\", nn.vars.{var})",
    run = function(ctx, p)
      if p.var == "" then
        for _, key in ipairs(nn.save.keys()) do
          local k = key:match("^vars%.(.+)$")
          if k then nn.vars[k] = nn.save.get(key) end
        end
      else
        local v = nn.save.get("vars." .. p.var)
        if v ~= nil then nn.vars[p.var] = v end
      end
    end,
  },
  {
    id = "zerar", name = "Zerar as variáveis", category = "Variáveis",
    text = "voltar as variáveis do jogo ao começo",
    help = "Todas voltam aos valores das Configurações do projeto (para começar um jogo novo).",
    lua = "nn.reset_vars()",
    run = function() nn.reset_vars() end,
  },
  -- Cenas
  {
    id = "ir_para_cena", name = "Ir para cena", category = "Cenas",
    text = "ir para a cena {cena}",
    props = { { "cena", "", type = "scene", label = "Cena" },
      { "escurecer", 0.5, min = 0, label = "Escurecer (s)", advanced = true } },
    lua = "nn.scene.switch(\"{cena}\")",
    run = function(ctx, p)
      if p.cena == "" then return end
      local fade = number(p.escurecer)
      nn.scene.switch(p.cena, nil, { fade = fade > 0 and fade or nil })
    end,
  },
  {
    id = "reiniciar", name = "Reiniciar a cena", category = "Cenas",
    text = "reiniciar a cena {como}",
    props = { { "como", "comeco", options = {
      { "comeco", "do começo" }, { "checkpoint", "do último checkpoint" },
      { "zerar", "do começo, zerando as variáveis" } } } },
    lua = "self:restart()",
    run = function(ctx, p)
      if p.como == "zerar" then nn.reset_vars() end
      ctx.scene:restart({ checkpoint = p.como == "checkpoint" })
    end,
  },
  {
    id = "por_cima", name = "Abrir cena por cima", category = "Cenas",
    text = "abrir a cena {cena} por cima",
    help = "Pausa, opções, inventário: a cena aparece por cima e a de baixo fica parada até ela fechar.",
    props = { { "cena", "", type = "scene", label = "Cena" } },
    lua = "nn.scene.push(\"{cena}\", { por_cima = true })",
    run = function(ctx, p) M.open_over(p.cena) end,
  },
  {
    id = "fechar_cena", name = "Fechar esta cena", category = "Cenas",
    text = "fechar esta cena (volta para a de baixo)",
    help = "Numa cena aberta por cima: fecha e o jogo de baixo continua.",
    lua = "nn.scene.pop()",
    run = function(ctx) M.close_over(ctx.scene) end,
  },
  {
    id = "sair", name = "Fechar o jogo", category = "Cenas",
    text = "fechar o jogo",
    lua = "nn.quit()",
    run = function() nn.quit({ fade = 0.4 }) end,
  },
  -- Objetos
  {
    id = "criar", name = "Criar prefab", category = "Objetos",
    text = "criar {prefab} em {onde}",
    props = {
      { "prefab", "", type = "prefab", label = "Prefab" },
      who("onde", "@este", "Onde"),
      { "dx", 0, label = "Deslocar x", advanced = true },
      { "dy", 0, label = "Deslocar y", advanced = true },
      { "dz", 0, label = "Deslocar z", advanced = true, mode = "3d" },
      { "vx", 0, label = "Velocidade x", advanced = true },
      { "vy", 0, label = "Velocidade y", advanced = true },
      { "vz", 0, label = "Velocidade z", advanced = true, mode = "3d" },
    },
    lua = "self:spawn(\"{prefab}\", x, y)",
    run = function(ctx, p, b)
      if p.prefab == "" then return end
      local spots = resolve(ctx.scene, p.onde, b, true)
      if ctx.scene._3d then
        for _, o in ipairs(spots) do
          local x, y, z = o:position3d()
          local n = ctx.scene:spawn(p.prefab, 0, 0)
          n:move_to(x + number(p.dx), y + number(p.dy), z + number(p.dz))
          local vx, vy, vz = number(p.vx), number(p.vy), number(p.vz)
          if vx ~= 0 or vy ~= 0 or vz ~= 0 then set_velocity(n, vx, vy, false, vz) end
        end
        return
      end
      for _, o in ipairs(spots) do
        local x, y = o:world_transform()
        local n = ctx.scene:spawn(p.prefab, x + number(p.dx), y + number(p.dy))
        local vx, vy = number(p.vx), number(p.vy)
        if vx ~= 0 or vy ~= 0 then set_velocity(n, vx, vy) end
      end
    end,
  },
  {
    id = "destruir", name = "Destruir", category = "Objetos",
    text = "destruir {alvo}",
    props = { who("alvo") },
    lua = "{alvo}:destroy()",
    run = function(ctx, p, b) each(ctx, p.alvo, b, function(o) o:destroy() end) end,
  },
  {
    id = "botao_ativo", name = "Ativar ou desativar botão", category = "Objetos",
    text = "{acao} o botão {alvo}",
    help = "Um botão desativado fica cinza e não pode ser apertado.",
    props = { { "acao", "desativar", options = { { "ativar", "ativar" }, { "desativar", "desativar" } } },
      who("alvo") },
    lua = "{alvo}.button.enabled = ...",
    run = function(ctx, p, b)
      each(ctx, p.alvo, b, function(o)
        if o.button then o.button.enabled = p.acao == "ativar" end
      end)
    end,
  },
  {
    id = "mostrar", name = "Mostrar ou esconder", category = "Objetos",
    text = "{acao} {alvo}",
    props = { { "acao", "esconder", options = { { "mostrar", "mostrar" }, { "esconder", "esconder" } } },
      who("alvo") },
    lua = "{alvo}.visible = ...",
    run = function(ctx, p, b)
      each(ctx, p.alvo, b, function(o) o.visible = p.acao == "mostrar" end)
    end,
  },
  {
    id = "mover", name = "Levar para (mover)", category = "Movimento",
    text = "levar {alvo} {modo} {x}, {y}",
    props = { who("alvo"),
      { "modo", "para", options = { { "para", "para a posição" }, { "somar", "andando" } } },
      { "x", 0 }, { "y", 0 }, { "z", 0, mode = "3d" } },
    text3d = "levar {alvo} {modo} {x}, {y}, {z}",
    lua = "{alvo}:move_to(x, y)",
    run = function(ctx, p, b)
      each(ctx, p.alvo, b, function(o)
        local x, y, z = number(p.x), number(p.y), number(p.z)
        if p.modo == "somar" then x, y, z = o.x + x, o.y + y, (o.z or 0) + z end
        o:move_to(x, y, z)
      end)
    end,
  },
  {
    id = "velocidade", name = "Velocidade ou empurrão", category = "Movimento",
    text = "{modo} {alvo}: {vx}, {vy}",
    help = "Velocidade em x (direita +) e y (cima +). Com corpo físico, vale para o corpo.",
    props = { { "modo", "definir", options = { { "definir", "mudar a velocidade de" }, { "somar", "empurrar" } } },
      who("alvo"), { "vx", 0 }, { "vy", 600, default3d = 10 }, { "vz", 0, mode = "3d" } },
    text3d = "{modo} {alvo}: {vx}, {vy}, {vz}",
    help3d = "Velocidade em x, y (cima +) e z, em metros por segundo. Com corpo físico, vale para o corpo; num Jogador 3D, y é um pulo.",
    lua = "{alvo}.body.vx, {alvo}.body.vy = {vx}, {vy}",
    run = function(ctx, p, b)
      local vz = ctx.scene._3d and number(p.vz) or nil
      each(ctx, p.alvo, b, function(o) set_velocity(o, number(p.vx), number(p.vy), p.modo == "somar", vz) end)
    end,
  },
  -- Aparência
  {
    id = "texto", name = "Mudar texto", category = "Aparência",
    text = "mudar o texto de {alvo} para {texto}",
    props = { who("alvo"), { "texto", "Moedas: {moedas}", label = "Texto",
      help = "Variáveis entre chaves viram o valor: {moedas}." } },
    lua = "{alvo}.text.text = \"{texto}\"",
    run = function(ctx, p, b)
      each(ctx, p.alvo, b, function(o)
        local c = o.text or o.button or o.checkbox
        if c then
          c.text = fill(p.texto)
          o._fixed_text = true
        end
      end)
    end,
  },
  {
    mode = "2d",
    id = "imagem", name = "Mudar imagem", category = "Aparência",
    text = "mudar a imagem de {alvo} para {imagem}",
    props = { who("alvo"), { "imagem", "", type = "image", label = "Imagem" } },
    lua = "{alvo}.sprite.image = \"{imagem}\"",
    run = function(ctx, p, b)
      each(ctx, p.alvo, b, function(o)
        local c = o.sprite
        if c and p.imagem ~= "" then
          c.image = p.imagem
          M.prepare(ctx.scene, o, c, true)
        end
      end)
    end,
  },
  {
    mode = "2d",
    id = "quadro", name = "Mudar quadro", category = "Aparência",
    text = "mudar o quadro de {alvo} para {quadro}",
    props = { who("alvo"), { "quadro", 1, min = 1, integer = true } },
    lua = "{alvo}.sprite.frame = {quadro}",
    run = function(ctx, p, b)
      each(ctx, p.alvo, b, function(o)
        if o.sprite then o.sprite.frame = math.floor(number(p.quadro)) end
      end)
    end,
  },
  {
    id = "animacao", name = "Tocar animação", category = "Aparência",
    text = "tocar a animação {nome} em {alvo}",
    help = "Uma animação do .anim da imagem (feita no editor de animação). Se já estiver tocando, continua.",
    help3d = "Uma animação do modelo 3D (as do arquivo .glb). Se já estiver tocando, continua.",
    props = { { "nome", "andar", type = "anim", label = "Animação" }, who("alvo"),
      { "reiniciar", false, label = "Do começo", advanced = true,
        help = "Começa do primeiro quadro mesmo se já estiver tocando." },
      { "repetir", true, label = "Repetir", advanced = true, mode = "3d",
        help = "Desligado: toca uma vez e para no fim (a condição \"Animação terminar\" avisa)." } },
    lua = "{alvo}.sprite:play(\"{nome}\")",
    run = function(ctx, p, b)
      each(ctx, p.alvo, b, function(o)
        if ctx.scene._3d then
          local a = M.model_animator(o)
          if a and a:has(p.nome) then
            a:play(p.nome, { loop = p.repetir ~= false, restart = p.reiniciar })
          end
          return
        end
        local s = o.sprite
        if s and s._anims and s._anims.by_name[p.nome] then s:play(p.nome, p.reiniciar) end
      end)
    end,
  },
  {
    mode = "2d",
    id = "trocar_tile", name = "Trocar o tile tocado", category = "Mapas",
    text = "trocar o tile tocado pelo tile {tile}",
    help = "Os tiles da condição \"Encostar num tile\" (blocos que quebram, moedas no mapa). 0 apaga; a colisão acompanha.",
    props = { { "tile", 0, type = "tile", min = 0, integer = true, label = "Tile" } },
    lua = "mapa:set_tile(col, row, {tile})",
    run = function(ctx, p, b)
      for _, tl in ipairs(b.tiles or {}) do
        pcall(tl.map.set_tile, tl.map, tl.col, tl.row, math.floor(number(p.tile)), tl.layer)
      end
    end,
  },
  {
    mode = "2d",
    id = "trocar_tile_em", name = "Trocar o tile em um objeto", category = "Mapas",
    text = "trocar o tile em {alvo} pelo tile {tile}",
    help = "O tile do mapa no centro do objeto (na camada de cima que tiver tile ali). 0 apaga.",
    props = { who("alvo"), { "tile", 0, type = "tile", min = 0, integer = true, label = "Tile" },
      { "camada", "", label = "Camada", advanced = true, help = "Nome da camada (vazio: a de cima que tiver tile ali, ou a primeira)." } },
    lua = "mapa:set_tile(mapa:cell_at({alvo}.x, {alvo}.y), {tile})",
    run = function(ctx, p, b)
      each(ctx, p.alvo, b, function(o)
        local x, y = o:world_transform()
        for _, m in ipairs(ctx.scene.object_list) do
          if m.alive and m.map then
            local col, row = m.map:cell_at(x, y)
            if col then
              local layer = p.camada ~= "" and p.camada or nil
              if not layer then
                layer = 1
                for li = #m.map.layers, 1, -1 do
                  if m.map:tile(col, row, li) ~= 0 then layer = li break end
                end
              end
              pcall(m.map.set_tile, m.map, col, row, math.floor(number(p.tile)), layer)
            end
          end
        end
      end)
    end,
  },
  {
    id = "cor", name = "Mudar cor", category = "Aparência",
    text = "mudar a cor de {alvo} para {cor}",
    props = { who("alvo"), { "cor", "#FF6B6B", type = "color" } },
    lua = "{alvo}.sprite.color = \"{cor}\"",
    run = function(ctx, p, b)
      each(ctx, p.alvo, b, function(o)
        for _, c in ipairs(o.components) do
          if c.type == "sprite" or c.type == "shape" or c.type == "text" or c.type == "model"
            or c.type == "shape3d" then
            c.color = p.cor
          end
        end
      end)
    end,
  },
  {
    mode = "2d",
    id = "faiscas", name = "Faíscas", category = "Aparência",
    text = "soltar faíscas em {alvo}",
    props = { who("alvo"), { "cor", "#FFD166", type = "color", advanced = true },
      { "quantidade", 16, min = 1, integer = true, advanced = true } },
    lua = "self:burst(x, y, \"{cor}\")",
    run = function(ctx, p, b)
      for _, o in ipairs(resolve(ctx.scene, p.alvo, b, true)) do
        local x, y = o:world_transform()
        ctx.scene:burst(x, y, p.cor, math.floor(number(p.quantidade)))
      end
    end,
  },
  {
    mode = "2d",
    id = "soltar_particulas", name = "Soltar partículas", category = "Aparência",
    text = "soltar {quantidade} partículas de {alvo}",
    help = "O objeto precisa do componente Partículas: solta de uma vez (poeira ao pular, brilho, explosão).",
    props = { who("alvo"), { "quantidade", 20, min = 1, integer = true, label = "Quantidade" } },
    lua = "{alvo}.particles:burst({quantidade})",
    run = function(ctx, p, b)
      for _, o in ipairs(resolve(ctx.scene, p.alvo, b, true)) do
        M.burst_particles(o, number(p.quantidade))
      end
    end,
  },
  {
    mode = "2d",
    id = "emissor", name = "Ligar ou desligar partículas", category = "Aparência",
    text = "{acao} as partículas de {alvo}",
    help = "Para a emissão contínua (as que já saíram terminam a vida) ou volta a emitir.",
    props = { { "acao", "desligar", options = { { "ligar", "ligar" }, { "desligar", "desligar" } } },
      who("alvo") },
    lua = "{alvo}.particles.emitting = ...",
    run = function(ctx, p, b)
      for _, o in ipairs(resolve(ctx.scene, p.alvo, b, true)) do
        M.set_emitting(o, p.acao == "ligar")
      end
    end,
  },
  {
    mode = "2d",
    id = "tremer", name = "Tremer a câmera", category = "Aparência",
    text = "tremer a câmera",
    props = { { "forca", 8, min = 0, label = "Força", advanced = true },
      { "tempo", 0.3, min = 0, label = "Tempo (s)", advanced = true } },
    lua = "nn.camera.shake({forca}, {tempo})",
    run = function(ctx, p) nn.camera.shake(number(p.forca), number(p.tempo)) end,
  },
  -- Som
  {
    id = "som", name = "Tocar som", category = "Som",
    text = "tocar o som {som}",
    props = { { "som", "", type = "sound", label = "Som" },
      { "volume", 1, min = 0, max = 2, advanced = true } },
    lua = "nn.audio.play(nn.assets.sound(\"{som}\"))",
    run = function(ctx, p)
      if p.som == "" then return end
      nn.audio.play(nn.assets.sound(p.som), { volume = number(p.volume) })
    end,
  },
  {
    id = "musica", name = "Música", category = "Som",
    text = "{acao} a música {som}",
    props = { { "acao", "tocar", options = { { "tocar", "tocar" }, { "parar", "parar" } } },
      { "som", "", type = "sound", label = "Música" },
      { "volume", 0.6, min = 0, max = 2, advanced = true } },
    lua = "nn.audio.play(nn.assets.sound(\"{som}\"), { loop = true, channel = \"music\" })",
    run = function(ctx, p)
      local cur = M._music
      if p.acao == "parar" then
        if cur and cur.handle.stop then cur.handle:stop() end
        M._music = nil
        return
      end
      if p.som == "" or (cur and cur.path == p.som) then return end
      if cur and cur.handle.stop then cur.handle:stop() end
      local h = nn.audio.play(nn.assets.sound(p.som), { loop = true, channel = "music", volume = number(p.volume) })
      M._music = { path = p.som, handle = h }
    end,
  },
  {
    id = "volume", name = "Volume", category = "Som",
    text = "volume {canal} em {valor}%",
    help = "Fica guardado: vale também na próxima vez que o jogo abrir.",
    props = { { "canal", "musica", options = { { "musica", "da música" }, { "efeitos", "dos efeitos" }, { "geral", "geral" } } },
      { "valor", 100, type = "value", label = "Volume (%)" } },
    lua = "nn.audio.set_volume({valor} / 100, ({ musica = \"music\", efeitos = \"effects\" })[\"{canal}\"])",
    run = function(ctx, p)
      M.set_volume(p.canal, number(fill(tostring(p.valor))) / 100)
    end,
  },
  -- Comportamentos e mensagens
  {
    id = "comportamento", name = "Ligar ou desligar comportamento", category = "Comportamentos",
    text = "{acao} o comportamento {nome} de {alvo}",
    props = { { "acao", "desligar", options = { { "ligar", "ligar" }, { "desligar", "desligar" } } },
      { "nome", "plataforma", type = "behavior", label = "Comportamento" }, who("alvo") },
    lua = "{alvo}:behavior(\"{nome}\").enabled = ...",
    run = function(ctx, p, b)
      each(ctx, p.alvo, b, function(o)
        local bh = o:behavior(p.nome)
        if bh then bh.enabled = p.acao == "ligar" end
      end)
    end,
  },
  {
    id = "mensagem", name = "Mandar mensagem", category = "Comportamentos",
    text = "mandar a mensagem {msg} para {alvo}",
    props = { { "msg", "abrir", label = "Mensagem" }, who("alvo") },
    lua = "{alvo}:send(\"{msg}\")",
    run = function(ctx, p, b)
      each(ctx, p.alvo, b, function(o) o:send(p.msg, b.este) end)
    end,
  },
  -- Tempo
  {
    id = "esperar", name = "Esperar", category = "Tempo",
    text = "esperar {segundos} segundos",
    help = "As ações seguintes desta regra acontecem depois desse tempo.",
    props = { { "segundos", 1, min = 0 } },
    lua = "-- esperar {segundos} s",
    -- Tratada à parte em run_actions.
  },
  {
    id = "console", name = "Escrever no console", category = "Tempo",
    text = "escrever no console: {texto}",
    props = { { "texto", "moedas = {moedas}", label = "Texto" } },
    lua = "print(\"{texto}\")",
    run = function(ctx, p) print(fill(p.texto)) end,
  },
}

local function index(list)
  local by = {}
  for _, d in ipairs(list) do by[d.id] = d end
  return by
end
E.COND_BY_ID = index(E.CONDITIONS)
E.ACT_BY_ID = index(E.ACTIONS)

local function fill_defaults(defn, item)
  local out = {}
  for k, v in pairs(item) do out[k] = v end
  for _, d in ipairs(defn._props) do
    if out[d.key] == nil then out[d.key] = d.default end
  end
  return out
end

local function prepare_defs()
  if E._ready then return end
  for _, list in ipairs({ E.CONDITIONS, E.ACTIONS }) do
    for _, d in ipairs(list) do d._props = M.normalize_props(d.props or {}, d.id) end
  end
  E._ready = true
end

--- Catálogo para o NN Studio (sem as funções).
function E.catalog()
  prepare_defs()
  local function strip(list)
    local out = {}
    for _, d in ipairs(list) do
      local trig = d.trigger
      if type(trig) == "function" then trig = "depends" end
      table.insert(out, {
        id = d.id, name = d.name, category = d.category, text = d.text, help = d.help,
        lua = d.lua, trigger = trig, props = d._props, mode = d.mode, text3d = d.text3d,
        help3d = d.help3d,
      })
    end
    return out
  end
  return { conditions = strip(E.CONDITIONS), actions = strip(E.ACTIONS), keys = E.KEYS }
end

-- Conferir as regras ----------------------------------------------------------

--- Prepara uma lista de regras (do arquivo) para rodar: preenche os padrões
--- e avisa de condições ou ações desconhecidas.
function E.compile(rules, where)
  prepare_defs()
  local out = {}
  for i, r in ipairs(rules or {}) do
    if type(r) ~= "table" then
      error(("%s: evento %d deve ser uma tabela"):format(where, i), 0)
    end
    local c = { index = i, off = r.off, comment = r.comment, mode = r.mode or "quando",
      when = {}, actions = {} }
    for j, cond in ipairs(r.when or {}) do
      local d = E.COND_BY_ID[cond.cond]
      if not d then
        error(("%s: evento %d: condição desconhecida \"%s\""):format(where, i, tostring(cond.cond)), 0)
      end
      local p = fill_defaults(d, cond)
      local trig = d.trigger
      if type(trig) == "function" then trig = trig(p) end
      table.insert(c.when, { def = d, p = p, trigger = trig and true or false, nao = cond.nao, j = j })
    end
    for j, act in ipairs(r.actions or {}) do
      local d = E.ACT_BY_ID[act.act]
      if not d then
        error(("%s: evento %d: ação desconhecida \"%s\""):format(where, i, tostring(act.act)), 0)
      end
      table.insert(c.actions, { def = d, p = fill_defaults(d, act) })
    end
    c.has_trigger = false
    for _, w in ipairs(c.when) do
      if w.trigger and not w.nao then c.has_trigger = true end
    end
    table.insert(out, c)
  end
  return out
end

-- Rodar -----------------------------------------------------------------------

local function new_ctx(scene, rules, owner, where)
  return {
    scene = scene, owner = owner, rules = E.compile(rules, where), state = {},
    started = false, base = { este = owner }, fired = {},
  }
end

--- Contextos de eventos da cena: um para as regras da cena e um para cada
--- objeto que tem regras.
function E.setup(scene, data, path)
  scene._signals = {}
  scene._jobs = {}
  scene._event_ctx = {}
  scene._fired = {}
  if data.events and #data.events > 0 then
    local ctx = new_ctx(scene, data.events, nil, path)
    ctx.key = "c"
    table.insert(scene._event_ctx, ctx)
  end
end

--- Um objeto com regras entrou na cena (no começo ou criado depois).
function E.add_object(scene, o)
  if not (o._rules and #o._rules > 0) then return end
  scene._event_ctx = scene._event_ctx or {}
  local ctx = new_ctx(scene, o._rules, o, (scene._path or "cena") .. ": objeto " .. o.name)
  ctx.key = tostring(o.id)
  o._event_ctx = ctx
  table.insert(scene._event_ctx, ctx)
end

local function binding_key(b)
  return tostring(b.este) .. "|" .. tostring(b.outro)
end

local function eval_rule(ctx, rule, rs, dt)
  -- Gatilhos que guardam estado (toques, tempo) andam todo quadro.
  for _, w in ipairs(rule.when) do
    if w.def.tick then
      rs[w.j] = rs[w.j] or {}
      w.def.tick(ctx, w.p, rs[w.j], dt)
    end
  end
  local bindings = { { este = ctx.owner } }
  for _, w in ipairs(rule.when) do
    local st = rs[w.j] or {}
    local nextb, seen = {}, {}
    for _, b in ipairs(bindings) do
      local r = w.def.test(ctx, w.p, st, b)
      local list
      if type(r) == "table" then
        -- Só as ligações que combinam com o que já foi escolhido.
        list = {}
        for _, m in ipairs(r) do
          local ok = true
          if b.este and m.este and b.este ~= m.este and (w.p.quem == "@este") then ok = false end
          if ok then table.insert(list, m) end
        end
        r = #list > 0
      end
      if w.nao then r = not r end
      if r then
        if list and not w.nao then
          for _, m in ipairs(list) do
            local nb = { este = b.este or m.este, outro = b.outro or m.outro, tiles = b.tiles or m.tiles }
            local k = binding_key(nb)
            if not seen[k] then
              seen[k] = true
              table.insert(nextb, nb)
            end
          end
        else
          local k = binding_key(b)
          if not seen[k] then
            seen[k] = true
            table.insert(nextb, b)
          end
        end
      end
    end
    bindings = nextb
    if #bindings == 0 then break end
  end
  if rule.mode == "enquanto" or rule.has_trigger then
    rs.prev_true = nil
    return bindings
  end
  -- "Quando" só com estados: dispara quando passa a valer.
  local prev = rs.prev_true or {}
  local now, out = {}, {}
  for _, b in ipairs(bindings) do
    local k = binding_key(b)
    now[k] = true
    if not prev[k] then table.insert(out, b) end
  end
  rs.prev_true = now
  return out
end

local function run_actions(ctx, actions, b, from)
  for i = from or 1, #actions do
    local a = actions[i]
    if a.def.id == "esperar" then
      table.insert(ctx.scene._jobs, { t = number(a.p.segundos), ctx = ctx, actions = actions, b = b, from = i + 1 })
      return
    end
    a.def.run(ctx, a.p, b)
    if ctx.scene._switching then return end
  end
end

local function mark_fired(scene, ctx, rule)
  if ctx.key then scene._fired[ctx.key .. ":" .. rule.index] = true end
end

--- Um quadro dos eventos: confere as regras e roda as ações.
function E.step(scene, dt)
  local list = scene._event_ctx
  if not list then return end
  -- Os sinais deste quadro; os que chegarem enquanto as ações rodam ficam
  -- para o próximo.
  scene._cur_signals = scene._signals or {}
  scene._signals = {}
  -- Cliques do mouse neste quadro.
  scene._clicked = {}
  if nn.input.mouse_pressed("left") then
    local wx, wy = nn.input.mouse_world()
    local sx, sy = nn.input.mouse()
    for _, o in ipairs(scene.object_list) do
      -- Itens de menu avisam o clique ao soltar o botão (como "clicou").
      if o.alive and not o._ui and o:shown() and E.hit(o, wx, wy, sx, sy) then
        scene._clicked[o] = true
      end
    end
  end
  for _, ctx in ipairs(list) do
    if ctx.owner == nil or ctx.owner.alive or not ctx.dead_seen then
      if ctx.owner and not ctx.owner.alive then ctx.dead_seen = true end
      for _, rule in ipairs(ctx.rules) do
        if not rule.off and not rule.comment and #rule.when > 0 then
          ctx.state[rule.index] = ctx.state[rule.index] or {}
          local fire = eval_rule(ctx, rule, ctx.state[rule.index], dt)
          for _, b in ipairs(fire) do
            mark_fired(scene, ctx, rule)
            run_actions(ctx, rule.actions, b)
          end
        end
      end
      ctx.started = true
    end
  end
  -- Ações esperando ("esperar N segundos").
  local jobs = scene._jobs
  scene._jobs = {}
  for _, j in ipairs(jobs) do
    j.t = j.t - dt
    if j.t <= 0 then
      run_actions(j.ctx, j.actions, j.b, j.from)
    else
      table.insert(scene._jobs, j)
    end
  end
  -- Tira os objetos que sumiram.
  local keep = {}
  for _, ctx in ipairs(list) do
    if ctx.owner == nil or ctx.owner.alive or not ctx.dead_seen then table.insert(keep, ctx) end
  end
  scene._event_ctx = keep
  scene._cur_signals = nil
end

--- Regras trocadas pelo editor com o jogo aberto.
function E.live(scene, owner, rules)
  local where = (scene._path or "cena")
  if owner == nil then
    for i, ctx in ipairs(scene._event_ctx or {}) do
      if ctx.owner == nil then table.remove(scene._event_ctx, i) break end
    end
    scene._data_events = rules
    if rules and #rules > 0 then
      local ctx = new_ctx(scene, rules, nil, where)
      ctx.key = "c"
      ctx.started = true
      table.insert(scene._event_ctx, 1, ctx)
    end
  else
    if owner._event_ctx then
      for i, ctx in ipairs(scene._event_ctx or {}) do
        if ctx == owner._event_ctx then table.remove(scene._event_ctx, i) break end
      end
      owner._event_ctx = nil
    end
    owner._rules = rules
    E.add_object(scene, owner)
    if owner._event_ctx then owner._event_ctx.started = true end
  end
end

--- Regras que dispararam desde a última vez (para o editor acender).
function E.take_fired(scene)
  local out = {}
  for k in pairs(scene._fired or {}) do table.insert(out, k) end
  table.sort(out)
  scene._fired = {}
  return out
end

return E
