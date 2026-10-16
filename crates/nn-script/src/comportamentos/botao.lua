-- Botão de menu: destaca com o mouse ou com as setas e responde ao clique,
-- ao Enter e ao A do controle.
local B = {
  title = "Botão",
  description = "Para menus: destaca ao passar o mouse (ou com as setas) e faz algo ao clicar ou apertar Enter.",
  category = "Interface",
  props = {
    { "acao", "nada", label = "Ao clicar",
      options = {
        { "nada", "nada (um evento decide)" },
        { "cena", "ir para outra cena" },
        { "reiniciar", "reiniciar a cena" },
        { "sair", "fechar o jogo" },
      } },
    { "cena", "", type = "scene", label = "Cena", show_if = { "acao", "cena" } },
    { "foco", false, label = "Começa selecionado",
      help = "O botão que já vem destacado para quem usa teclado ou controle." },
    { "cor", "#FFD166", type = "color", label = "Cor ao destacar" },
    { "aumentar", 1.1, label = "Aumentar ao destacar", min = 1, max = 2 },
    { "som", "", type = "sound", label = "Som ao clicar", advanced = true },
  },
}

local function visual(o)
  for _, c in ipairs(o.components) do
    if c.type == "text" or c.type == "sprite" or c.type == "shape" then return c end
  end
end

function B:start()
  local c = visual(self.obj)
  self.cor_original = c and c.color
  self.sx, self.sy = self.obj.scale_x, self.obj.scale_y
  local s = self.scene
  s._buttons = s._buttons or {}
  table.insert(s._buttons, self)
  if self.foco then s._button_focus = self end
end

local function alive_buttons(scene)
  local out = {}
  for _, b in ipairs(scene._buttons or {}) do
    if b.obj.alive and b.enabled and b.obj:shown() then table.insert(out, b) end
  end
  scene._buttons = out
  return out
end

local function pressed(names, pad)
  for _, k in ipairs(names) do
    if nn.input.pressed(k) then return true end
  end
  return pad and nn.input.pad_pressed(pad) or false
end

-- O botão mais perto na direção (dx, dy).
local function nearest(list, from, dx, dy)
  local fx, fy = from.obj:world_transform()
  local best, best_d
  for _, b in ipairs(list) do
    if b ~= from then
      local x, y = b.obj:world_transform()
      local along = (x - fx) * dx + (y - fy) * dy
      if along > 1 then
        local across = math.abs((x - fx) * dy - (y - fy) * dx)
        local d = along + across * 2
        if not best_d or d < best_d then best, best_d = b, d end
      end
    end
  end
  return best
end

-- Teclado e controle: um botão por cena cuida disso a cada quadro.
local function navigate(scene)
  local frame = nn.time.frame()
  if scene._button_frame == frame then return end
  scene._button_frame = frame
  local list = alive_buttons(scene)
  if #list == 0 then return end
  local focus = scene._button_focus
  if focus and not (focus.obj.alive and focus.enabled) then focus = nil end
  local dirs = {
    { { "up", "w" }, "up", 0, 1 }, { { "down", "s" }, "down", 0, -1 },
    { { "left", "a" }, "left", -1, 0 }, { { "right", "d" }, "right", 1, 0 },
  }
  for _, d in ipairs(dirs) do
    if pressed(d[1], d[2]) then
      if not focus then
        table.sort(list, function(a, b)
          local _, ay = a.obj:world_transform()
          local _, by = b.obj:world_transform()
          return ay > by
        end)
        focus = list[1]
      else
        focus = nearest(list, focus, d[3], d[4]) or focus
      end
    end
  end
  -- O mouse andou por cima de um botão: ele fica com o destaque.
  local mx, my = nn.input.mouse()
  if mx and (mx ~= scene._button_mx or my ~= scene._button_my) then
    scene._button_mx, scene._button_my = mx, my
    for _, b in ipairs(list) do
      if b.obj:mouse_over() then focus = b end
    end
  end
  scene._button_focus = focus
  if focus and pressed({ "enter", "space" }, "a") then focus:apertar() end
end

function B:update(dt)
  navigate(self.scene)
  if nn.input.mouse_pressed("left") and self.obj:mouse_over() then self:apertar() end
  local on = self.scene._button_focus == self
  local c = visual(self.obj)
  if c then c.color = on and self.cor or self.cor_original end
  local k = on and self.aumentar or 1
  self.obj.scale_x, self.obj.scale_y = self.sx * k, self.sy * k
end

function B:apertar()
  if self.som ~= "" then nn.audio.play(nn.assets.sound(self.som)) end
  self.obj:send("clicou")
  if self.acao == "cena" and self.cena ~= "" then
    nn.scene.switch(self.cena, nil, { fade = 0.4 })
  elseif self.acao == "reiniciar" then
    self.scene:restart()
  elseif self.acao == "sair" then
    nn.quit({ fade = 0.4 })
  end
end

return B
