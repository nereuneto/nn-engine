-- Interação dos menus das cenas visuais: botões, deslizadores, caixas de
-- marcar e campos de texto (e o comportamento "Botão").
--
-- Cada cena guarda a lista de itens que recebem o foco. O foco anda com as
-- setas (ou WASD, ou o direcional do controle) para o item mais perto naquela
-- direção, vai para o item que o mouse passou por cima, e Enter, espaço ou o
-- A do controle apertam o item com foco. Num deslizador com foco, esquerda e
-- direita mudam o valor; num campo de texto, as letras vão para o texto.

local cena = ...
local U = {}

-- Itens: { obj, kind = "button" | "slider" | "checkbox" | "textfield" | "behavior",
--          comp, enabled = fn, press = fn, step = fn(dir), start_focus }

local function ui(scene)
  scene._ui = scene._ui or { items = {} }
  return scene._ui
end

function U.setup(scene)
  scene._ui = { items = {} }
end

--- Põe um item na lista do foco.
function U.add(scene, item)
  local s = ui(scene)
  item.obj._ui = true
  table.insert(s.items, item)
  -- Vários pedindo o foco: fica o primeiro.
  if item.start_focus and not s.focus then s.focus = item end
  return item
end

local function usable(item)
  local o = item.obj
  return o.alive and o:shown() and (not item.enabled or item.enabled())
end

local function alive(scene)
  local s = ui(scene)
  local out = {}
  for _, it in ipairs(s.items) do
    if it.obj.alive then table.insert(out, it) end
  end
  s.items = out
  local list = {}
  for _, it in ipairs(out) do
    if usable(it) then table.insert(list, it) end
  end
  return list
end

--- O item com foco (ou nil).
function U.focused(scene)
  local s = ui(scene)
  if s.focus and not usable(s.focus) then s.focus = nil end
  return s.focus
end

--- Como desenhar o item: "normal", "hover", "press" ou "disabled".
function U.state(scene, item)
  if not item then return "normal" end
  if item.enabled and not item.enabled() then return "disabled" end
  local s = ui(scene)
  if s.pressing == item or (s.flash == item and (s.flash_until or 0) > nn.time.elapsed()) then
    return "press"
  end
  if s.focus == item then return "hover" end
  return "normal"
end

--- O item do objeto (ou do componente) pedido.
function U.item_of(scene, obj, comp)
  for _, it in ipairs(ui(scene).items) do
    if it.obj == obj and (comp == nil or it.comp == comp) then return it end
  end
end

-- Posição na tela (pixels, y para baixo) e meio tamanho do item.
local function rect(item)
  local o = item.obj
  local x, y = o:world_transform()
  local hw, hh = o:bounds()
  if cena.is_hud(o) then
    local sx, sy = cena.screen_pos(o, x, y)
    return sx, sy, hw, hh, true
  end
  return x, y, hw, hh, false
end

local function mouse_on(item)
  local x, y, hw, hh, hud = rect(item)
  local mx, my
  if hud then
    mx, my = nn.input.mouse()
  else
    mx, my = nn.input.mouse_world()
  end
  if not mx then return false end
  -- Numa lista que rola, o que ficou escondido não responde.
  if cena.scrolled_out(item.obj, mx, my) then return false end
  return math.abs(mx - x) <= hw and math.abs(my - y) <= hh
end

-- O item mais perto na direção (dx, dy), com y para cima.
local function nearest(list, from, dx, dy)
  local fx, fy = from.obj:world_transform()
  local best, best_d
  for _, it in ipairs(list) do
    if it ~= from then
      local x, y = it.obj:world_transform()
      local along = (x - fx) * dx + (y - fy) * dy
      if along > 1 then
        local across = math.abs((x - fx) * dy - (y - fy) * dx)
        local d = along + across * 2
        if not best_d or d < best_d then best, best_d = it, d end
      end
    end
  end
  return best
end

local function pressed(keys, pad)
  for _, k in ipairs(keys) do
    if nn.input.pressed(k) then return true end
  end
  return pad and nn.input.pad_pressed(pad) or false
end

local function first(list)
  table.sort(list, function(a, b)
    local ax, ay = a.obj:world_transform()
    local bx, by = b.obj:world_transform()
    if ay ~= by then return ay > by end
    return ax < bx
  end)
  return list[1]
end

--- Aperta o item (como Enter ou um clique).
function U.activate(scene, item)
  local s = ui(scene)
  s.flash, s.flash_until = item, nn.time.elapsed() + 0.12
  if item.press then item.press() end
end

-- Um quadro: foco, mouse, teclado, controle e digitação.
function U.update(scene, dt)
  local s = ui(scene)
  local list = alive(scene)
  local focus = U.focused(scene)
  if #list == 0 then
    s.focus, s.pressing = nil, nil
    return
  end
  local typing = focus and focus.kind == "textfield"

  -- Teclado e controle.
  local dirs = {
    { { "up" }, "w", "up", 0, 1 }, { { "down" }, "s", "down", 0, -1 },
    { { "left" }, "a", "left", -1, 0 }, { { "right" }, "d", "right", 1, 0 },
  }
  for _, d in ipairs(dirs) do
    local keys = { d[1][1] }
    -- Escrevendo num campo, WASD são letras.
    if not typing then table.insert(keys, d[2]) end
    if pressed(keys, d[3]) then
      local horizontal = d[5] == 0
      if focus and focus.step and horizontal then
        focus.step(d[4])
      elseif not focus then
        focus = first(list)
      else
        focus = nearest(list, focus, d[4], d[5]) or focus
      end
    end
  end
  if nn.input.pressed("tab") then
    -- Tab: o próximo na ordem de leitura (de cima para baixo).
    local order = {}
    for _, it in ipairs(list) do table.insert(order, it) end
    first(order)
    local i = 1
    for k, it in ipairs(order) do
      if it == focus then i = k % #order + 1 end
    end
    focus = order[i]
  end

  -- Mouse: passar por cima dá o foco; apertar e soltar em cima aperta.
  local mx, my = nn.input.mouse()
  local moved = mx and (mx ~= s.mx or my ~= s.my)
  if mx then s.mx, s.my = mx, my end
  local hovered
  for _, it in ipairs(list) do
    if mouse_on(it) then hovered = it end
  end
  local hovered_focus = false
  if moved and hovered and not (typing and hovered.kind ~= "textfield" and focus == s.pressing) then
    focus = hovered
    hovered_focus = true
  end
  if nn.input.mouse_pressed("left") then
    if hovered then
      s.pressing = hovered
      focus = hovered
      if hovered.drag then hovered.drag(true) end
    elseif typing then
      -- Clicou fora do campo: ele perde o foco.
      focus = nil
    end
  end
  if s.pressing and s.pressing.drag and nn.input.mouse_down("left") then
    s.pressing.drag(false)
  end
  if s.pressing and nn.input.mouse_released("left") then
    local it = s.pressing
    s.pressing = nil
    if it == hovered and not it.drag and usable(it) then U.activate(scene, it) end
  end

  -- O foco andou: numa lista que rola, ela rola até ele.
  if focus and focus ~= s.focus and not hovered_focus then cena.scroll_into_view(focus.obj) end
  s.focus = focus
  typing = focus and focus.kind == "textfield"
  if typing then
    focus.type(nn.input.text())
    if nn.input.pressed("backspace") then focus.erase() end
    if nn.input.pressed("enter") then U.activate(scene, focus) end
  elseif focus and pressed({ "enter", "space" }, "a") then
    U.activate(scene, focus)
  end
end

return U
