-- Diálogos: conversas montadas no editor de conversa do NN Studio (arquivos
-- `.dialogo`) tocando numa caixa de fala no pé da tela.
--
--   return {
--     format = 1, start = 1, speed = 40,
--     characters = { { name = "Ana", color = "#FFD166", image = "", sound = "" } },
--     nodes = {
--       { id = 1, who = "Ana", text = "Oi! Viu minha chave?", next = 2 },
--       { id = 2, who = "", text = "...", choices = {
--           { text = "Vi sim", next = 3, var = "ajudou", value = 1 },
--           { text = "Não", next = 0 },
--       } },
--       { id = 3, who = "Ana", text = "Obrigada! Tome.", item = "moeda", next = 0 },
--     },
--   }
--
-- Cada fala tem quem fala (um personagem, ou "" para o narrador), o texto
-- (com {variavel}), a próxima fala (`next`; 0 = termina) ou as escolhas e,
-- opcionalmente, o que ela faz ao aparecer: mudar uma variável (`var`,
-- `value`) e dar um item (`item`). Uma escolha também pode mudar uma variável
-- e só aparecer para quem tem um item (`need`).
--
-- Enquanto a conversa está aberta, os jogadores prontos ficam parados. E,
-- Enter, espaço, o A do controle ou um clique mostram o texto todo e passam
-- para a próxima fala; as setas (ou W e S) e os números escolhem.

local cena = ...
local D = {}

D.EXTENSION = "dialogo"

local function fill(text)
  return (tostring(text or ""):gsub("{([%w_]+)}", function(k)
    local v = nn.vars[k]
    if v == nil then return "{" .. k .. "}" end
    if type(v) == "number" and v == math.floor(v) then return tostring(math.floor(v)) end
    return tostring(v)
  end))
end

--- Lê e confere um arquivo de diálogo.
function D.load(path)
  local data = cena.load_data(path, "diálogo")
  data.nodes = data.nodes or {}
  data.characters = data.characters or {}
  local by_id = {}
  for i, n in ipairs(data.nodes) do
    if type(n.id) ~= "number" then
      error(("%s: a fala %d não tem id"):format(path, i), 0)
    end
    by_id[n.id] = n
  end
  data.by_id = by_id
  data.by_name = {}
  for _, c in ipairs(data.characters) do
    if c.name and c.name ~= "" then data.by_name[c.name] = c end
  end
  return data
end

local function confirm_down()
  return nn.input.down("e") or nn.input.down("enter") or nn.input.down("space")
    or nn.input.pad_down("a") or nn.input.mouse_down("left")
end

local function confirm_pressed()
  return nn.input.pressed("e") or nn.input.pressed("enter") or nn.input.pressed("space")
    or nn.input.pad_pressed("a") or nn.input.mouse_pressed("left")
end

local function send(d, msg, arg)
  if d.owner and d.owner.alive then
    d.owner:send(msg, arg)
  elseif cena.events then
    cena.events.signal(d.scene, { kind = "mensagem", obj = d.owner, msg = msg, arg = arg })
  end
end

-- As escolhas que aparecem (as com `need` só para quem tem o item).
local function visible_choices(node)
  local out = {}
  for _, c in ipairs(node.choices or {}) do
    if c.need == nil or c.need == "" or nn.inventory.has(c.need) then
      table.insert(out, c)
    end
  end
  return out
end

local function set_var(var, value)
  if var == nil or var == "" then return end
  if type(value) == "string" then
    local n = tonumber(fill(value))
    if n then value = n else value = fill(value) end
  end
  nn.vars[var] = value
end

-- Mostra a fala `id` (0 ou nil: termina).
local function go(d, id)
  local node = id and id ~= 0 and d.data.by_id[id] or nil
  if not node then
    D.stop(d.scene, true)
    return
  end
  d.node = node
  d.text = fill(node.text)
  d.len = utf8.len(d.text) or #d.text
  d.shown = 0
  d.choice = 1
  d.choices = visible_choices(node)
  d.lines = nil
  set_var(node.var, node.value)
  if node.item and node.item ~= "" then nn.inventory.add(node.item, 1) end
  send(d, "fala", node.id)
end

--- Começa a conversa do arquivo `path`. `owner` é quem fala (recebe as
--- mensagens conversa_comecou, escolheu e conversa_terminou).
function D.start(scene, path, owner)
  if scene._dlg then return false end
  local ok, data = pcall(D.load, path)
  if not ok then
    -- Um aviso no console; o jogo continua (sem a conversa).
    cena.warn(("conversa %s: %s"):format(path, cena.file_error(path, data)))
    return false
  end
  local d = { scene = scene, data = data, path = path, owner = owner, armed = false, t = 0 }
  scene._dlg = d
  send(d, "conversa_comecou", path)
  go(d, data.start or (data.nodes[1] and data.nodes[1].id))
  return true
end

--- Fecha a conversa (`finished`: chegou ao fim).
function D.stop(scene, finished)
  local d = scene._dlg
  if not d then return end
  scene._dlg = nil
  -- A tecla que fechou não aperta mais nada neste quadro.
  scene._dlg_closed = true
  send(d, "conversa_terminou", d.path)
end

local function sound(path)
  if path and path ~= "" then pcall(function() nn.audio.play(nn.assets.sound(path)) end) end
end

function D.update(scene, dt)
  scene._dlg_closed = nil
  local d = scene._dlg
  if not d then return end
  d.t = d.t + dt
  -- Só vale a tecla apertada depois de soltar a que abriu a conversa.
  if not d.armed then
    d.armed = not confirm_down()
    return
  end
  local speed = tonumber(d.data.speed) or 40
  local before = math.floor(d.shown)
  if d.shown < d.len then
    d.shown = speed <= 0 and d.len or math.min(d.len, d.shown + speed * dt)
    local who = d.data.by_name[d.node.who or ""]
    if who and who.sound and who.sound ~= "" and math.floor(d.shown) ~= before
        and math.floor(d.shown) % 3 == 0 then
      sound(who.sound)
    end
  end
  local typing = d.shown < d.len
  local n = #d.choices
  if not typing and n > 0 then
    local up = nn.input.pressed("up") or nn.input.pressed("w") or nn.input.pad_pressed("up")
    local down = nn.input.pressed("down") or nn.input.pressed("s") or nn.input.pad_pressed("down")
    if up then d.choice = (d.choice - 2) % n + 1 end
    if down then d.choice = d.choice % n + 1 end
    for i = 1, math.min(n, 9) do
      if nn.input.pressed(tostring(i)) then
        d.choice = i
        d.pick = true
      end
    end
    -- O mouse em cima de uma escolha.
    local mx, my = nn.input.mouse()
    for i, r in ipairs(d.rects or {}) do
      if mx and mx >= r[1] and mx <= r[1] + r[3] and my >= r[2] and my <= r[2] + r[4] then
        if d.last_mouse and (d.last_mouse[1] ~= mx or d.last_mouse[2] ~= my) then d.choice = i end
      end
    end
    d.last_mouse = mx and { mx, my } or nil
  end
  if confirm_pressed() or d.pick then
    d.pick = false
    if typing then
      d.shown = d.len
    elseif n > 0 then
      local c = d.choices[d.choice]
      sound(cena.theme("sound"))
      set_var(c.var, c.value)
      send(d, "escolheu", c.text)
      go(d, c.next)
    else
      sound(cena.theme("sound"))
      go(d, d.node.next)
    end
  end
end

-- Quebra o texto em linhas que cabem em `w` pixels.
local function wrap(text, w, opts)
  local lines = {}
  for para in (text .. "\n"):gmatch("(.-)\n") do
    local line = ""
    for word in para:gmatch("%S+") do
      local try = line == "" and word or (line .. " " .. word)
      if line ~= "" and nn.text_size(try, opts) > w then
        table.insert(lines, line)
        line = word
      else
        line = try
      end
    end
    table.insert(lines, line)
  end
  return lines
end

local images = {}
local function image(path)
  if path == nil or path == "" then return nil end
  local img = images[path]
  if img == nil then
    -- Pixel art (como o sprite): o retrato fica nítido ampliado.
    local ok, t = pcall(nn.assets.texture, path, { pixel_art = true })
    img = ok and t or false
    images[path] = img
  end
  return img or nil
end

-- A caixa de fala no pé da tela, com o tema do projeto.
function D.draw(scene)
  local d = scene._dlg
  if not d or not d.node then return end
  local W, H = nn.window.size()
  local font = cena.font("")
  -- Tudo pelo tamanho da tela: numa tela de pixel art (320x180), a caixa
  -- e as letras ficam pequenas como o resto do jogo.
  local size = math.max(7, math.floor(math.min(W, H * 1.6) / 34))
  local opts = { size = size, font = font }
  local margin = math.max(2, math.floor(math.min(W, H) * 0.04))
  local radius = math.max(2, math.floor(size * 0.6))
  local line = math.max(1, math.floor(size / 9))
  local bw = math.min(W - margin * 2, size * 42)
  local bh = math.floor(size * 5.2)
  local bx, by = math.floor((W - bw) / 2), H - margin - bh
  local panel = cena.theme("panel")
  local border = cena.theme("border")
  local text_color = cena.theme("text")
  local accent = cena.theme("accent")
  nn.screen.panel(bx, by, bw, bh, { color = panel, radius = radius, border = line * 2, border_color = border })
  local who = d.data.by_name[d.node.who or ""]
  local pad = math.floor(size * 0.9)
  local tx = bx + pad
  -- O retrato do personagem, à esquerda.
  local img = who and image(who.image)
  if img then
    local ps = bh - pad * 2
    local f = math.max(1, math.floor(size / 5))
    nn.screen.panel(bx + pad - f, by + pad - f, ps + f * 2, ps + f * 2, { color = "#00000040", radius = radius })
    nn.screen.sprite(img, bx + pad, by + pad, { w = ps, h = ps })
    tx = bx + pad * 2 + ps
  end
  -- O nome, numa etiqueta em cima da caixa.
  local name = d.node.who or ""
  if name ~= "" then
    local nopts = { size = math.floor(size * 0.95), font = font }
    local nw = nn.text_size(name, nopts) + pad * 1.4
    local nh = math.floor(size * 1.7)
    nn.screen.panel(tx - pad * 0.3, by - nh * 0.6, nw, nh,
      { color = (who and who.color ~= "" and who.color) or accent, radius = radius })
    nn.screen.text(name, tx - pad * 0.3 + nw / 2, by - nh * 0.6 + nh / 2,
      { size = nopts.size, color = "#1E2A3A", anchor = "center", font = font })
  end
  -- O texto, aparecendo letra por letra.
  local tw = bx + bw - pad - tx
  if not d.lines or d.lines_w ~= tw then
    d.lines = wrap(d.text, tw, opts)
    d.lines_w = tw
  end
  local left = math.floor(d.shown)
  local ty = by + pad * 0.9
  for _, line in ipairs(d.lines) do
    if left <= 0 then break end
    local n = utf8.len(line) or #line
    local part = line
    if left < n then part = line:sub(1, (utf8.offset(line, left + 1) or (#line + 1)) - 1) end
    nn.screen.text(part, tx, ty, { size = size, color = text_color, anchor = "topleft", font = font })
    left = left - n - 1
    ty = ty + size * 1.35
  end
  local typing = d.shown < d.len
  if typing then return end
  if #d.choices == 0 then
    -- Pronto: um quadradinho piscando no canto (passa para a próxima).
    if (d.t * 2) % 2 < 1.3 then
      local s = math.floor(size * 0.5)
      nn.screen.panel(bx + bw - pad - s, by + bh - pad * 0.8 - s, s, s, { color = accent, radius = math.max(1, s / 4) })
    end
    return
  end
  -- As escolhas, em cima da caixa, à direita.
  local copts = { size = size, font = font }
  local cw = 0
  for i, c in ipairs(d.choices) do
    cw = math.max(cw, nn.text_size(i .. ".  " .. fill(c.text), copts))
  end
  cw = math.min(cw + pad * 2, bw)
  local ch = math.floor(size * 1.9)
  local gap = math.floor(size * 0.35)
  local cx = bx + bw - cw
  local cy = by - gap * 3 - #d.choices * (ch + gap) + gap
  if name ~= "" then cy = cy - math.floor(size * 0.6) end
  -- Muitas escolhas: começam no alto da tela, sem sair dela.
  cy = math.max(margin, cy)
  d.rects = {}
  for i, c in ipairs(d.choices) do
    local on = i == d.choice
    nn.screen.panel(cx, cy, cw, ch, {
      color = panel, radius = radius, border = on and line * 3 or line,
      border_color = on and accent or border,
    })
    nn.screen.text(i .. ".  " .. fill(c.text), cx + pad, cy + ch / 2,
      { size = size, color = on and accent or text_color, anchor = "left", font = font })
    d.rects[i] = { cx, cy, cw, ch }
    cy = cy + ch + gap
  end
end

return D
