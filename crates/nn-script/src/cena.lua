-- Executa as cenas visuais (arquivos .cena) montadas no NN Studio.
--
-- Um .cena é um arquivo Lua que devolve uma tabela com os objetos da cena.
-- Este módulo transforma essa tabela numa cena comum: cria os objetos,
-- carrega imagens e sons, monta a física e desenha tudo sozinho. O script da
-- cena (campo `script`), se houver, recebe os callbacks de sempre (enter,
-- update, fixed_update, draw...) e encontra os objetos em `self.objects`.
--
-- Objetos podem ter comportamentos (componente `behavior`): scripts com
-- propriedades que o NN Studio mostra no Inspetor. Os prontos vêm com a
-- engine ("nn:plataforma", "nn:coletavel"...); os do jogo são arquivos .lua.
-- Objetos também podem ser instâncias de prefabs (arquivos .prefab).

local M = {}

local DEG = math.pi / 180

-- Campos de cada componente, na ordem em que o NN Studio grava, com o valor
-- padrão. O editor lê esta mesma lista: ela é a única fonte dos padrões.
M.FIELDS = {
  sprite = {
    { "image", "" }, { "frame_w", 0 }, { "frame_h", 0 }, { "frame", 1 },
    { "w", 0 }, { "h", 0 }, { "color", "#FFFFFF" }, { "flip_x", false },
    { "flip_y", false }, { "pixel_art", true }, { "fill", "stretch" }, { "animation", "" },
    { "hud", false },
  },
  text = {
    { "text", "Texto" }, { "size", 32 }, { "color", "#FFFFFF" }, { "anchor", "center" },
    { "font", "" }, { "hud", false },
  },
  shape = {
    { "shape", "rect" }, { "w", 64 }, { "h", 64 }, { "radius", 32 }, { "color", "#3FA7D6" },
    { "hud", false },
  },
  body = {
    { "body", "dynamic" }, { "shape", "box" }, { "w", 32 }, { "h", 32 }, { "radius", 16 },
    { "sensor", false }, { "one_way", false }, { "bounce", 0 }, { "gravity_scale", 1 },
    { "tag", "" },
  },
  map = {
    { "map", "" }, { "scale", 1 }, { "collision", true },
  },
  camera = {
    { "follow", "" }, { "smooth", 6 }, { "zoom", 1 }, { "limit_to_map", true },
  },
  sound = {
    { "sound", "" }, { "autoplay", true }, { "loop", false }, { "volume", 1 },
    { "channel", "effects" },
  },
  particles = {
    { "rate", 20 }, { "lifetime", 1 }, { "speed_min", 40 }, { "speed_max", 120 },
    { "direction", 90 }, { "spread", 360 }, { "gravity", 0 }, { "size_start", 12 },
    { "size_end", 0 }, { "color_start", "#FFFFFF" }, { "color_end", "#FFFFFF00" },
    { "blend", "alpha" }, { "emitting", true },
  },
  -- Interface (menus e HUD). Cores vazias ("") usam o tema do projeto
  -- (config.lua, `theme`).
  panel = {
    { "w", 320 }, { "h", 220 }, { "color", "" }, { "radius", 12 }, { "border", 0 },
    { "border_color", "" }, { "image", "" }, { "slice", 4 }, { "slice_scale", 4 },
    { "hud", true },
  },
  button = {
    { "text", "Botão" }, { "size", 28 }, { "w", 240 }, { "h", 60 }, { "color", "" },
    { "hover_color", "" }, { "press_color", "" }, { "text_color", "" }, { "radius", 12 },
    { "border", 0 }, { "border_color", "" }, { "image", "" }, { "slice", 4 },
    { "slice_scale", 4 }, { "icon", "" }, { "font", "" }, { "action", "nada" },
    { "scene", "" }, { "sound", "" }, { "focus", false }, { "enabled", true },
    { "hud", true },
  },
  bar = {
    { "w", 240 }, { "h", 24 }, { "var", "" }, { "value", 1 }, { "max", 1 },
    { "max_var", "" }, { "color", "" }, { "back_color", "#00000080" }, { "radius", 6 },
    { "border", 0 }, { "border_color", "" }, { "direction", "right" }, { "hud", true },
  },
  icons = {
    { "image", "" }, { "frame", 1 }, { "frame_w", 0 }, { "frame_h", 0 }, { "var", "" },
    { "count", 3 }, { "max", 0 }, { "size", 32 }, { "spacing", 6 }, { "color", "#FFFFFF" },
    { "empty_color", "#FFFFFF40" }, { "align", "left" }, { "pixel_art", true },
    { "hud", true },
  },
  slider = {
    { "w", 240 }, { "h", 12 }, { "target", "variavel" }, { "var", "" }, { "value", 0.5 },
    { "min", 0 }, { "max", 1 }, { "step", 0 }, { "color", "" }, { "back_color", "#00000080" },
    { "knob_color", "#FFFFFF" }, { "knob", 28 }, { "hud", true },
  },
  checkbox = {
    { "text", "Opção" }, { "size", 26 }, { "box", 36 }, { "target", "variavel" },
    { "var", "" }, { "checked", false }, { "color", "" }, { "check_color", "" },
    { "text_color", "" }, { "font", "" }, { "hud", true },
  },
  textfield = {
    { "text", "" }, { "placeholder", "Digite aqui" }, { "var", "" }, { "w", 320 },
    { "h", 56 }, { "size", 26 }, { "max", 20 }, { "color", "#00000080" },
    { "text_color", "" }, { "border", 2 }, { "border_color", "" }, { "radius", 8 },
    { "font", "" }, { "hud", true },
  },
  -- Grupo: arruma os filhos sozinho (em coluna, em linha ou em grade).
  layout = {
    { "arrange", "vertical" }, { "spacing", 16 }, { "columns", 2 }, { "align", "center" },
  },
}

-- Componentes de interface (desenhados como painéis e ficam no HUD).
M.UI_KINDS = {
  panel = true, button = true, bar = true, icons = true, slider = true,
  checkbox = true, textfield = true,
}

-- Tema do projeto (`theme` no config.lua): o que vale quando a cor de um
-- componente de interface está vazia.
M.THEME = {
  font = "", text = "#FFFFFF", button = "#3FA7D6", hover = "#5BC0EB",
  press = "#2E86AB", disabled = "#5B6472", panel = "#1E2A3AE6", border = "#FFFFFF40",
  accent = "#FFD166", sound = "",
}

--- O valor do tema (o do config.lua ou o padrão).
function M.theme(key)
  local t = rawget(nn, "theme")
  local v = type(t) == "table" and t[key] or nil
  if v == nil or v == "" then return M.THEME[key] end
  return v
end

-- Cor do componente, ou a do tema quando vazia.
local function themed(value, key)
  if value == nil or value == "" then return M.theme(key) end
  return value
end
M.themed = themed

-- Fontes lidas uma vez (o caminho vazio usa a do tema, ou a padrão).
local fonts = {}
function M.font(path)
  if path == nil or path == "" then path = M.theme("font") end
  if path == nil or path == "" then return nil end
  local f = fonts[path]
  if f == nil then
    local ok, loaded = pcall(nn.assets.font, path)
    f = ok and loaded or false
    if not ok then print(("fonte não encontrada: %s"):format(path)) end
    fonts[path] = f
  end
  return f or nil
end

-- Campos do objeto, na ordem de gravação (os filhos e componentes vêm depois).
M.OBJECT_FIELDS = {
  { "id", 0 }, { "name", "Objeto" }, { "x", 0 }, { "y", 0 }, { "rotation", 0 },
  { "scale_x", 1 }, { "scale_y", 1 }, { "visible", true }, { "locked", false },
  { "layer", 0 }, { "anchor", "" },
}

-- Campos da cena.
M.SCENE_FIELDS = {
  { "script", "" }, { "background", "" }, { "gravity", -1800 }, { "overlay", false },
}

-- Comportamento: `script` e as propriedades dele, livres (vêm do script).
M.BEHAVIOR = "behavior"

local function defaults(kind)
  local list = M.FIELDS[kind]
  if not list then
    error(("componente desconhecido: \"%s\" (use %s)"):format(
      tostring(kind), "sprite, text, shape, body, map, camera, sound, particles, panel, "
      .. "button, bar, icons, slider, checkbox, textfield, layout ou behavior"), 0)
  end
  return list
end

local Sprite = {}
Sprite.__index = Sprite

local function copy_component(src)
  if src.type == M.BEHAVIOR then
    local c = {}
    for k, v in pairs(src) do c[k] = v end
    c.script = c.script or ""
    return c
  end
  local c = { type = src.type }
  for _, f in ipairs(defaults(src.type)) do
    local v = src[f[1]]
    if v == nil then v = f[2] end
    c[f[1]] = v
  end
  if c.type == "sprite" then setmetatable(c, Sprite) end
  return c
end

-- Animações (arquivo .anim ao lado da imagem) ------------------------------
--
-- heroi.png → heroi.anim: o tamanho do quadro e as animações com nome.
--
--   return {
--     format = 1, frame_w = 16, frame_h = 16,
--     animations = {
--       { name = "andar", frames = { 3, 4, 5, 6 }, fps = 10 },
--       { name = "pulo", frames = { 7, 8 }, fps = 8, mode = "once", times = { 0.1, 0.3 } },
--     },
--   }

M.ANIM_EXTENSION = "anim"

--- Caminho do .anim de uma imagem (ou o próprio caminho, se já for .anim).
function M.anim_path(image)
  if image:lower():match("%.anim$") then return image end
  local base = image:gsub("%.[^./\\]*$", "")
  return base .. ".anim"
end

--- Lê e confere um .anim. Devolve { frame_w, frame_h, list, by_name }.
function M.parse_anims(data, path)
  if type(data) ~= "table" then error(path .. ": o .anim deve devolver uma tabela", 0) end
  local out = {
    frame_w = tonumber(data.frame_w) or 0, frame_h = tonumber(data.frame_h) or 0,
    list = {}, by_name = {},
  }
  for i, a in ipairs(data.animations or {}) do
    local name = tostring(a.name or ("animacao" .. i))
    local frames = {}
    for _, f in ipairs(a.frames or {}) do
      local n = math.floor(tonumber(f) or 0)
      if n >= 1 then table.insert(frames, n) end
    end
    if #frames == 0 then frames = { 1 } end
    local fps = tonumber(a.fps) or 10
    if fps <= 0 then fps = 10 end
    local mode = a.mode or "loop"
    if mode ~= "loop" and mode ~= "once" and mode ~= "pingpong" then
      error(("%s: animação \"%s\": modo \"%s\" não existe (use loop, once ou pingpong)")
        :format(path, name, tostring(mode)), 0)
    end
    local times = {}
    for k = 1, #frames do
      local v = a.times and tonumber(a.times[k])
      times[k] = (v and v > 0) and v or 1 / fps
    end
    -- A passada completa: no pingpong, volta pelos do meio.
    local seq = {}
    for k = 1, #frames do table.insert(seq, k) end
    if mode == "pingpong" then
      for k = #frames - 1, 2, -1 do table.insert(seq, k) end
    end
    local total = 0
    for _, k in ipairs(seq) do total = total + times[k] end
    local anim = { name = name, frames = frames, fps = fps, mode = mode, times = times,
      seq = seq, total = total }
    table.insert(out.list, anim)
    out.by_name[name] = anim
  end
  return out
end

--- As animações de uma imagem (lidas uma vez por cena), ou nil sem .anim.
function M.load_anims(scene, image)
  if image == nil or image == "" then return nil end
  local path = M.anim_path(image)
  scene._anims = scene._anims or {}
  local cached = scene._anims[path]
  if cached == nil then
    cached = false
    if nn.assets.exists(path) then
      cached = M.parse_anims(M.load_data(path, "arquivo de animações"), path)
    end
    scene._anims[path] = cached
  end
  return cached or nil
end

--- Quadro da animação no instante t (e se ela já terminou).
function M.anim_frame(anim, t)
  if anim.mode == "once" then
    if t >= anim.total then return anim.frames[#anim.frames], true end
  else
    t = t % anim.total
  end
  for _, k in ipairs(anim.seq) do
    if t < anim.times[k] then return anim.frames[k], false end
    t = t - anim.times[k]
  end
  return anim.frames[anim.seq[#anim.seq]], anim.mode == "once"
end

local function anim_names(anims)
  local names = {}
  for _, a in ipairs(anims.list) do table.insert(names, a.name) end
  return table.concat(names, ", ")
end

--- Toca uma animação do .anim da imagem: obj.sprite:play("andar").
--- Se já estiver tocando, continua (`restart = true` começa de novo).
function Sprite:play(name, restart)
  local anims = self._anims
  if not anims then
    error(("a imagem \"%s\" não tem animações (crie %s no NN Studio)")
      :format(self.image, M.anim_path(self.image)), 2)
  end
  if not anims.by_name[name] then
    error(("a animação \"%s\" não existe em %s (tem: %s)")
      :format(tostring(name), M.anim_path(self.image), anim_names(anims)), 2)
  end
  if restart or self.animation ~= name or self._anim_name ~= name then
    self.animation, self._anim_name, self._anim_time, self._anim_done = name, name, 0, false
    self.frame = anims.by_name[name].frames[1]
  end
end

--- Para a animação (o quadro atual fica).
function Sprite:stop()
  self.animation, self._anim_name = "", nil
end

--- A animação (de modo "once") chegou ao fim?
function Sprite:finished()
  return self._anim_done == true
end

--- Nomes das animações da imagem.
function Sprite:animations()
  local out = {}
  for _, a in ipairs((self._anims or { list = {} }).list) do table.insert(out, a.name) end
  return out
end

-- Avança as animações dos objetos; avisa quem termina ("animacao_terminou").
local function update_animations(scene, dt)
  for _, o in ipairs(scene.object_list) do
    if o.alive then
      for _, c in ipairs(o.components) do
        local anims = c._anims
        if anims and c.animation ~= "" then
          local anim = anims.by_name[c.animation]
          if anim then
            if c._anim_name ~= c.animation then
              -- Mudou por fora (obj.sprite.animation = "x" ou pelo editor).
              c._anim_name, c._anim_time, c._anim_done = c.animation, 0, false
            else
              c._anim_time = (c._anim_time or 0) + dt
            end
            local frame, done = M.anim_frame(anim, c._anim_time)
            c.frame = frame
            if done and not c._anim_done then
              c._anim_done = true
              o:send("animacao_terminou", anim.name)
            end
          end
        end
      end
    end
  end
end

-- Objetos -------------------------------------------------------------------

local Object = {}
Object.__index = Object

--- Primeiro componente do tipo pedido (ou nil).
function Object:component(kind)
  for _, c in ipairs(self.components) do
    if c.type == kind then return c end
  end
  return nil
end

--- Posição, rotação (graus) e escala no mundo, somando as dos pais.
function Object:world_transform()
  local p = self.parent
  if not p then
    return self.x, self.y, self.rotation, self.scale_x, self.scale_y
  end
  local px, py, pr, psx, psy = p:world_transform()
  local lx, ly = self.x * psx, self.y * psy
  local a = pr * DEG
  local c, s = math.cos(a), math.sin(a)
  return px + lx * c - ly * s, py + lx * s + ly * c, pr + self.rotation,
    psx * self.scale_x, psy * self.scale_y
end

--- O objeto e todos os pais estão visíveis?
function Object:shown()
  local o = self
  while o do
    if not o.visible then return false end
    o = o.parent
  end
  return true
end

--- Tira o objeto (e os filhos) da cena.
function Object:destroy()
  if not self.alive then return end
  for _, b in ipairs(self.behaviors) do
    if type(b.destroyed) == "function" then b:destroyed() end
  end
  if self.scene and M.events then
    -- As regras "for destruído" (inclusive as do próprio objeto) ainda rodam
    -- neste quadro.
    M.events.signal(self.scene, { kind = "destruido", obj = self })
  end
  self.alive = false
  if self.body and self.body.alive then self.body:remove() end
  for _, ch in ipairs(self.children) do ch:destroy() end
  local scene = self.scene
  if scene then
    scene._dirty_list = true
    if scene.objects[self.name] == self then
      scene.objects[self.name] = nil
      for _, o in ipairs(scene.object_list) do
        if o.alive and o.name == self.name and o ~= self then
          scene.objects[self.name] = o
          break
        end
      end
    end
  end
end

--- Comportamento do objeto pelo nome ("plataforma", "nn:plataforma" ou o
--- caminho do script).
function Object:behavior(name)
  for _, b in ipairs(self.behaviors) do
    local s = b.component.script
    if s == name or s == "nn:" .. name then return b end
  end
  return nil
end

--- Chama `msg(...)` em todos os comportamentos do objeto que tiverem essa
--- função (ex.: `jogador:send("dano", 1)`). Devolve o primeiro resultado.
function Object:send(msg, ...)
  if self.scene and M.events then
    M.events.signal(self.scene, { kind = "mensagem", obj = self, msg = msg, arg = (...) })
  end
  local result
  for _, b in ipairs(self.behaviors) do
    local f = b[msg]
    if b.enabled and type(f) == "function" then
      local v = f(b, ...)
      if result == nil then result = v end
    end
  end
  return result
end

-- Quantos ícones mostrar: o valor da variável (ou `count`) e, com `max`,
-- os vazios até ele.
local function icons_count(c)
  local n = c.count
  if c.var ~= "" then n = tonumber(nn.vars and nn.vars[c.var]) or 0 end
  n = math.max(0, math.floor(n))
  return n, math.max(n, math.floor(c.max or 0))
end
M.icons_count = icons_count

local function text_opts(c, size)
  return { size = size or c.size, font = M.font(c.font) }
end

--- Largura e altura de um componente (sem a escala do objeto), ou nada.
function M.component_size(c)
  local t = c.type
  if t == "sprite" then
    if not c._img then return nil end
    local w, h = c.w, c.h
    if w <= 0 then w = c._sheet and c.frame_w or c._img.width end
    if h <= 0 then h = c._sheet and c.frame_h or c._img.height end
    return w, h
  elseif t == "shape" then
    if c.shape == "circle" then return c.radius * 2, c.radius * 2 end
    return c.w, c.h
  elseif t == "text" then
    if c.text == "" then return 0, c.size end
    return nn.text_size(c.text, text_opts(c))
  elseif t == "panel" or t == "button" or t == "bar" or t == "textfield" then
    return c.w, c.h
  elseif t == "slider" then
    return c.w + c.knob, math.max(c.h, c.knob)
  elseif t == "checkbox" then
    local tw = 0
    if c.text ~= "" then tw = nn.text_size(c.text, text_opts(c)) + c.box * 0.4 end
    return c.box + tw, math.max(c.box, c.size * 1.2)
  elseif t == "icons" then
    local _, k = icons_count(c)
    k = math.max(k, 1)
    return k * c.size + (k - 1) * c.spacing, c.size
  end
  return nil
end

--- Metade da largura e da altura do objeto no mundo: do corpo físico, se
--- houver, senão da imagem, da forma, do texto ou do componente de interface.
function Object:bounds()
  local b = self.body
  if b and b.alive then
    if b.radius and b.radius > 0 then return b.radius, b.radius end
    return b.width / 2, b.height / 2
  end
  local _, _, _, sx, sy = self:world_transform()
  sx, sy = math.abs(sx), math.abs(sy)
  for _, c in ipairs(self.components) do
    local w, h = M.component_size(c)
    if w and w > 0 and h > 0 then return w * sx / 2, h * sy / 2 end
  end
  return 16 * sx, 16 * sy
end

--- Os dois objetos estão encostando (ou um dentro do outro)?
function Object:touching(other, margin)
  if not (other and other.alive and self.alive) or other == self then return false end
  margin = margin or 2
  local ax, ay = self:world_transform()
  local bx, by = other:world_transform()
  local aw, ah = self:bounds()
  local bw, bh = other:bounds()
  return math.abs(ax - bx) <= aw + bw + margin and math.abs(ay - by) <= ah + bh + margin
end

--- O mouse está em cima do objeto? (Objetos de HUD usam a posição na tela.)
function Object:mouse_over()
  if not (self.alive and self:shown()) then return false end
  local wx, wy = nn.input.mouse_world()
  local sx, sy = nn.input.mouse()
  return M.events.hit(self, wx, wy, sx, sy)
end

--- Tiles dos mapas da cena que o objeto está tocando (a borda dele, com uma
--- margem): { map, owner, col, row, layer, tile, properties }.
function Object:tiles_touching(margin)
  margin = margin or 2
  local out = {}
  if not (self.alive and self.scene) then return out end
  local x, y = self:world_transform()
  local hw, hh = self:bounds()
  for _, m in ipairs(self.scene.object_list or {}) do
    if m.alive and m.map and m ~= self then
      for _, tl in ipairs(m.map:tiles_in(x - hw - margin, y - hh - margin, x + hw + margin, y + hh + margin)) do
        tl.map, tl.owner = m.map, m
        table.insert(out, tl)
      end
    end
  end
  return out
end

--- Leva o objeto (e o corpo físico) para x, y.
function Object:move_to(x, y)
  self.x, self.y = x, y
  if self.body and self.body.alive then
    local wx, wy = self:world_transform()
    self.body:set_position(wx, wy)
    self.body:set_velocity(0, 0)
    self._sx, self._sy, self._px, self._py = x, y, x, y
  end
end

-- Tela (HUD) ----------------------------------------------------------------

-- Pontos da tela para prender o HUD: fração da largura e da altura (do
-- canto de cima à esquerda).
M.ANCHORS = {
  top_left = { 0, 0 }, top = { 0.5, 0 }, top_right = { 1, 0 },
  left = { 0, 0.5 }, center = { 0.5, 0.5 }, right = { 1, 0.5 },
  bottom_left = { 0, 1 }, bottom = { 0.5, 1 }, bottom_right = { 1, 1 },
}

--- O objeto fica preso à tela? (Algum componente de HUD, ou a cena está
--- aberta por cima de outra.)
function M.is_hud(o)
  if o.scene and o.scene.overlay then return true end
  for _, c in ipairs(o.components) do
    if c.hud then return true end
  end
  return false
end

--- Onde fica na tela (pixels, y para baixo) o ponto x, y (do mundo do
--- objeto) de um objeto de HUD: a âncora do objeto da raiz mais x, y.
function M.screen_pos(o, x, y)
  local root = o
  while root.parent do root = root.parent end
  local a = M.ANCHORS[root.anchor or ""] or M.ANCHORS.center
  local W, H = nn.window.size()
  return W * a[1] + x, H * a[2] - y
end

-- Prefabs -------------------------------------------------------------------

M.PREFAB_EXTENSION = "prefab"

-- Campos próprios de cada instância; o resto vem do prefab.
M.INSTANCE_FIELDS = {
  "id", "name", "x", "y", "rotation", "scale_x", "scale_y", "visible", "locked", "layer",
  "anchor",
}

local function load_data(path, what)
  local src = nn.assets.read(path)
  local chunk, err = load(src, "@" .. path)
  if not chunk then error(err, 0) end
  local data = chunk()
  if type(data) ~= "table" then
    error(("%s: o %s deve terminar com `return` de uma tabela"):format(path, what), 0)
  end
  return data
end

M.load_data = function(...) return load_data(...) end

--- O objeto guardado num .prefab (lido uma vez por cena).
function M.load_prefab(scene, path)
  scene._prefabs = scene._prefabs or {}
  local p = scene._prefabs[path]
  if not p then
    local data = load_data(path, "prefab")
    if type(data.object) ~= "table" then
      error(path .. ": o prefab precisa de `object = { ... }`", 0)
    end
    p = data.object
    scene._prefabs[path] = p
  end
  return p
end

local function shallow(t)
  local c = {}
  for k, v in pairs(t) do c[k] = v end
  return c
end

--- Junta o prefab com o que a instância mudou (só o objeto principal e os
--- componentes dele): devolve os dados de um objeto comum.
function M.expand(scene, data, depth)
  if data.prefab == nil or data.prefab == "" then return data end
  depth = depth or 0
  if depth > 8 then
    error(data.prefab .. ": prefabs demais um dentro do outro (um prefab dentro dele mesmo?)", 0)
  end
  local base = M.load_prefab(scene, data.prefab)
  local out = shallow(base)
  for _, k in ipairs(M.INSTANCE_FIELDS) do
    if data[k] ~= nil then out[k] = data[k] end
  end
  out.prefab, out.overrides = nil, nil
  out.components = {}
  for i, c in ipairs(base.components or {}) do out.components[i] = shallow(c) end
  for _, ov in ipairs(data.overrides or {}) do
    local c = out.components[ov.component]
    if c then
      for k, v in pairs(ov) do
        if k ~= "component" then c[k] = v end
      end
    end
  end
  out.children = base.children
  out._depth = depth + 1
  return out
end

local function build_object(scene, data, parent, fresh, depth)
  data = M.expand(scene, data, depth)
  local o = setmetatable({}, Object)
  for _, f in ipairs(M.OBJECT_FIELDS) do
    local v = data[f[1]]
    if v == nil then v = f[2] end
    o[f[1]] = v
  end
  if fresh then
    -- Partes de prefab e objetos criados pelo jogo: ids só desta execução.
    scene._next_id = (scene._next_id or 0) - 1
    o.id = scene._next_id
  end
  o.scene, o.parent, o.alive = scene, parent, true
  o.components, o.children, o.behaviors = {}, {}, {}
  o.start_x, o.start_y = o.x, o.y
  o._rules = data.events
  for _, c in ipairs(data.components or {}) do
    local comp = copy_component(c)
    table.insert(o.components, comp)
    if o[comp.type] == nil and (comp.type == "sprite" or comp.type == "text"
        or comp.type == "shape" or comp.type == "camera" or comp.type == "layout"
        or M.UI_KINDS[comp.type]) then
      o[comp.type] = comp
    end
  end
  table.insert(scene.object_list, o)
  scene._by_id[o.id] = o
  if scene.objects[o.name] == nil then scene.objects[o.name] = o end
  local child_fresh = fresh or data._depth ~= nil
  for _, ch in ipairs(data.children or {}) do
    table.insert(o.children, build_object(scene, ch, o, child_fresh, data._depth or depth))
  end
  return o
end

-- Comportamentos ------------------------------------------------------------

-- Código dos comportamentos prontos ("nn:id" → fonte), preenchido pela engine.
M.BUILTIN = {}

--- Lista de propriedades de um comportamento, no formato do editor:
--- { key, default, type, label, help, min, max, options, advanced, ... }.
function M.normalize_props(props, where)
  local out = {}
  if type(props) ~= "table" then return out end
  if #props > 0 then
    for _, p in ipairs(props) do
      if type(p) ~= "table" or type(p[1]) ~= "string" then
        error(where .. ": cada propriedade é { \"nome\", valor_padrão, ... }", 0)
      end
      local d = { key = p[1], default = p[2] }
      for k, v in pairs(p) do
        if type(k) == "string" then d[k] = v end
      end
      table.insert(out, d)
    end
  else
    local keys = {}
    for k in pairs(props) do
      if type(k) == "string" then table.insert(keys, k) end
    end
    table.sort(keys)
    for _, k in ipairs(keys) do table.insert(out, { key = k, default = props[k] }) end
  end
  for _, d in ipairs(out) do
    if d.default == nil then d.default = "" end
    if d.type == nil then
      local t = type(d.default)
      if t == "number" then
        d.type = "number"
      elseif t == "boolean" then
        d.type = "bool"
      elseif d.options then
        d.type = "choice"
      elseif d.default:match("^#%x%x%x%x%x%x$") or d.default:match("^#%x%x%x%x%x%x%x%x$") then
        d.type = "color"
      else
        d.type = "text"
      end
    end
  end
  return out
end

local function behavior_source(script)
  local id = script:match("^nn:(.+)$")
  if id then
    local src = M.BUILTIN[id]
    if not src then error(("comportamento pronto desconhecido: %s"):format(script), 0) end
    return src, "@nn/comportamentos/" .. id .. ".lua"
  end
  return nn.assets.read(script), "@" .. script
end

local function behavior_def(scene, script)
  scene._defs = scene._defs or {}
  local def = scene._defs[script]
  if def then return def end
  local src, name = behavior_source(script)
  local chunk, err = load(src, name)
  if not chunk then error(err, 0) end
  def = chunk()
  if type(def) ~= "table" then
    error(script .. ": o comportamento deve terminar com `return` de uma tabela", 0)
  end
  def._props = M.normalize_props(def.props, script)
  scene._defs[script] = def
  return def
end

local function make_behavior(scene, o, c)
  if c.script == nil or c.script == "" then return nil end
  local def = behavior_def(scene, c.script)
  local b = setmetatable({}, { __index = def })
  for _, p in ipairs(def._props) do
    local v = c[p.key]
    if v == nil then v = p.default end
    b[p.key] = v
  end
  b.obj, b.scene, b.component, b.enabled = o, scene, c, true
  c._inst = b
  table.insert(o.behaviors, b)
  return b
end

local function start_behaviors(list)
  for _, o in ipairs(list) do
    for _, b in ipairs(o.behaviors) do
      if o.alive and b.enabled and type(b.start) == "function" then b:start() end
    end
  end
end

local function each_behavior(scene, name, ...)
  for _, o in ipairs(scene.object_list) do
    if o.alive then
      for _, b in ipairs(o.behaviors) do
        local f = b[name]
        if b.enabled and type(f) == "function" then f(b, ...) end
        if not o.alive then break end
      end
    end
  end
end

--- Lê as informações de um comportamento sem rodar nada da engine (o NN
--- Studio usa para montar o Inspetor).
function M.describe(src, name)
  local env = setmetatable({}, { __index = _G })
  local chunk, err = load(src, "@" .. name, "t", env)
  if not chunk then return { error = err } end
  local ok, def = pcall(chunk)
  if not ok then return { error = tostring(def) } end
  if type(def) ~= "table" then
    return { error = name .. ": o comportamento deve terminar com `return` de uma tabela" }
  end
  local ok2, props = pcall(M.normalize_props, def.props, name)
  if not ok2 then return { error = tostring(props) } end
  return {
    title = def.title, description = def.description, category = def.category,
    requires = def.requires, player = def.player, props = props,
  }
end

--- Quem um comportamento procura: "" é o jogador (objetos com um
--- comportamento de jogador); senão, os objetos com esse nome.
function M.targets(scene, who)
  local out = {}
  for _, o in ipairs(scene.object_list) do
    if o.alive then
      if who == nil or who == "" then
        for _, b in ipairs(o.behaviors) do
          if b.player then table.insert(out, o) break end
        end
      elseif o.name == who then
        table.insert(out, o)
      end
    end
  end
  return out
end

--- Faíscas num ponto (moeda pega, inimigo derrotado...).
function M.burst(scene, x, y, color, count)
  color = color or "#FFD166"
  scene._fx = scene._fx or {}
  local fx = scene._fx[color]
  if not fx then
    fx = nn.particles({
      emitting = false, lifetime = { 0.25, 0.55 }, speed = { 90, 240 }, drag = 4,
      size = { 12, 0 }, colors = { "#FFFFFF", color, color:sub(1, 7) .. "00" }, blend = "add",
    })
    scene._fx[color] = fx
  end
  fx:burst(count or 16, x, y)
end

-- Componentes ---------------------------------------------------------------

local function make_body(scene, o, c)
  if not scene.world then return end
  if o.body and o.body.alive then
    scene._by_body[o.body.id] = nil
    o.body:remove()
  end
  local x, y, _, sx, sy = o:world_transform()
  local opts = {
    type = c.body, x = x, y = y, sensor = c.sensor, one_way = c.one_way,
    bounce = c.bounce, gravity_scale = c.gravity_scale,
    tag = (c.tag ~= "" and c.tag) or o.name,
  }
  if c.shape == "circle" then
    opts.radius = c.radius * math.max(math.abs(sx), math.abs(sy))
  else
    opts.w = c.w * math.abs(sx)
    opts.h = c.h * math.abs(sy)
  end
  o.body = scene.world:body(opts)
  scene._by_body[o.body.id] = o
  o._sx, o._sy, o._px, o._py = o.x, o.y, o.x, o.y
end

local function make_particles(o, c)
  o.particles = nn.particles({
    rate = c.rate, emitting = c.emitting, lifetime = c.lifetime,
    speed = { c.speed_min, c.speed_max },
    direction = c.direction * DEG, spread = c.spread * DEG, gravity = c.gravity,
    size = { c.size_start, c.size_end },
    colors = { c.color_start, c.color_end },
    blend = c.blend,
  })
  local x, y = o:world_transform()
  o.particles:set_position(x, y)
end

-- Interface ------------------------------------------------------------------

-- Opções do jogador (volume, tela cheia), guardadas entre as partidas.
local OPTIONS_KEY = "nn_opcoes"
local VOLUME_CHANNELS = { musica = "music", efeitos = "effects", geral = false }

local function options()
  local ok, t = pcall(nn.save.get, OPTIONS_KEY)
  return (ok and type(t) == "table") and t or {}
end

local function save_option(key, value)
  local t = options()
  t[key] = value
  pcall(nn.save.set, OPTIONS_KEY, t)
end

local function volume(target)
  local ch = VOLUME_CHANNELS[target]
  if ch == nil then return nil end
  return nn.audio.volume(ch or nil)
end

--- Muda o volume ("musica", "efeitos" ou "geral", de 0 a 1) e guarda.
function M.set_volume(target, v)
  local ch = VOLUME_CHANNELS[target]
  if ch == nil then return end
  v = math.max(0, math.min(1, v))
  nn.audio.set_volume(v, ch or nil)
  save_option(target, v)
end

local options_applied = false
local function apply_options()
  if options_applied then return end
  options_applied = true
  local t = options()
  for target, ch in pairs(VOLUME_CHANNELS) do
    if type(t[target]) == "number" then nn.audio.set_volume(t[target], ch or nil) end
  end
  if type(t.tela_cheia) == "boolean" then nn.window.set_fullscreen(t.tela_cheia) end
end

--- Abre a cena por cima da atual (pausa, opções): a de baixo continua
--- aparecendo e fica parada até esta fechar.
function M.open_over(path)
  if path and path ~= "" then nn.scene.push(path, { por_cima = true }) end
end

--- Fecha a cena aberta por cima (volta para a de baixo).
function M.close_over(scene)
  if scene.overlay then nn.scene.pop() end
end

local function slider_value(c)
  local v = volume(c.target)
  if v == nil then
    v = c.value
    if c.var ~= "" then v = tonumber(nn.vars[c.var]) or c.min end
  end
  return v
end

local function set_slider(c, v)
  local lo, hi = math.min(c.min, c.max), math.max(c.min, c.max)
  v = math.max(lo, math.min(hi, v))
  if c.step > 0 then v = c.min + math.floor((v - c.min) / c.step + 0.5) * c.step end
  if VOLUME_CHANNELS[c.target] ~= nil then
    M.set_volume(c.target, (v - c.min) / ((c.max - c.min) ~= 0 and (c.max - c.min) or 1))
  elseif c.var ~= "" then
    nn.vars[c.var] = v
  else
    c.value = v
  end
end

local function checked(c)
  if c.target == "tela_cheia" then return nn.window.fullscreen() end
  if c.var ~= "" then return nn.vars[c.var] == true end
  return c.checked
end

local function field_text(c)
  if c.var ~= "" then
    local v = nn.vars[c.var]
    return v == nil and "" or tostring(v)
  end
  return c.text
end

local function set_field(c, text)
  if c.var ~= "" then nn.vars[c.var] = text else c.text = text end
end

local function bar_fraction(c)
  local v, max = c.value, c.max
  if c.var ~= "" then v = tonumber(nn.vars[c.var]) or 0 end
  if c.max_var ~= "" then max = tonumber(nn.vars[c.max_var]) or max end
  if max <= 0 then return 0 end
  return math.max(0, math.min(1, v / max))
end
M.bar_fraction = bar_fraction

local function play_click(c)
  local snd = (c.sound ~= nil and c.sound ~= "") and c.sound or M.theme("sound")
  if snd and snd ~= "" then
    pcall(function() nn.audio.play(nn.assets.sound(snd)) end)
  end
end

-- Avisa os eventos que o valor do item mudou ("Mudar de valor").
local function changed(scene, o)
  if M.events then M.events.signal(scene, { kind = "mudou", obj = o }) end
end

--- Aperta o botão: som, o evento "Clique no objeto" e a ação dele.
local function press_button(scene, o, c)
  play_click(c)
  o:send("clicou")
  local a = c.action
  if a == "cena" and c.scene ~= "" then
    nn.scene.switch(c.scene, nil, { fade = 0.4 })
  elseif a == "por_cima" then
    M.open_over(c.scene)
  elseif a == "fechar" then
    M.close_over(scene)
  elseif a == "reiniciar" then
    scene:restart()
  elseif a == "sair" then
    nn.quit({ fade = 0.4 })
  end
end

-- Liga o componente de interface ao foco do teclado e do mouse (uma vez).
local function register_ui(scene, o, c)
  if c._item or not M.ui then return end
  local t = c.type
  local item = { obj = o, comp = c, kind = t }
  if t == "button" then
    item.enabled = function() return c.enabled end
    item.press = function() press_button(scene, o, c) end
    item.start_focus = c.focus
  elseif t == "slider" then
    item.step = function(dir)
      local span = c.max - c.min
      set_slider(c, slider_value(c) + dir * (c.step > 0 and c.step or span / 10))
      changed(scene, o)
    end
    item.drag = function()
      local x, y = o:world_transform()
      local mx
      if M.is_hud(o) then
        x = M.screen_pos(o, x, y)
        mx = nn.input.mouse()
      else
        mx = nn.input.mouse_world()
      end
      if not mx then return end
      local _, _, _, sx = o:world_transform()
      local w = c.w * math.abs(sx)
      local t01 = (mx - (x - w / 2)) / (w ~= 0 and w or 1)
      local before = slider_value(c)
      set_slider(c, c.min + t01 * (c.max - c.min))
      if slider_value(c) ~= before then changed(scene, o) end
    end
  elseif t == "checkbox" then
    item.press = function()
      local v = not checked(c)
      if c.target == "tela_cheia" then
        nn.window.set_fullscreen(v)
        save_option("tela_cheia", v)
      elseif c.var ~= "" then
        nn.vars[c.var] = v
      else
        c.checked = v
      end
      play_click(c)
      o:send("clicou")
      changed(scene, o)
    end
  elseif t == "textfield" then
    item.type = function(str)
      if str == "" then return end
      local cur = field_text(c)
      local room = math.max(0, c.max - (utf8.len(cur) or #cur))
      local add, n = {}, 0
      for _, code in utf8.codes(str) do
        if n >= room then break end
        table.insert(add, utf8.char(code))
        n = n + 1
      end
      if n > 0 then
        set_field(c, cur .. table.concat(add))
        changed(scene, o)
      end
    end
    item.erase = function()
      local cur = field_text(c)
      local last = utf8.offset(cur, -1)
      if last then
        set_field(c, cur:sub(1, last - 1))
        changed(scene, o)
      end
    end
    -- Enter no campo: como clicar num botão (para confirmar).
    item.press = function() o:send("clicou") end
  else
    return
  end
  o._ui = true
  c._item = M.ui.add(scene, item)
end

-- Imagem de um componente de interface (com quadros, se pedir).
local function ui_image(c, path, frame_w, frame_h)
  if path == nil or path == "" then return nil end
  if frame_w and frame_w > 0 and frame_h and frame_h > 0 then
    return nn.assets.sheet(path, frame_w, frame_h, { pixel_art = c.pixel_art ~= false })
  end
  return nn.assets.texture(path, { pixel_art = c.pixel_art ~= false })
end

local function prepare_ui(scene, o, c)
  local t = c.type
  if t == "panel" or t == "button" then
    c._img = ui_image(c, c.image)
  end
  if t == "button" then c._icon = ui_image(c, c.icon) end
  if t == "icons" then c._img = ui_image(c, c.image, c.frame_w, c.frame_h) end
  register_ui(scene, o, c)
end

-- Grupo: põe os filhos em coluna, em linha ou em grade, em volta do objeto.
local function arrange(o, l)
  local list = {}
  for _, ch in ipairs(o.children) do
    if ch.alive and ch.visible then
      local w, h = 0, 0
      for _, c in ipairs(ch.components) do
        local cw, chh = M.component_size(c)
        if cw then
          w, h = math.max(w, cw), math.max(h, chh)
        end
      end
      table.insert(list, { o = ch, w = w * math.abs(ch.scale_x), h = h * math.abs(ch.scale_y) })
    end
  end
  if #list == 0 then return end
  local gap = l.spacing
  local function cross(size, total)
    if l.align == "start" then return -total / 2 + size / 2 end
    if l.align == "end" then return total / 2 - size / 2 end
    return 0
  end
  if l.arrange == "vertical" or l.arrange == "horizontal" then
    local vertical = l.arrange == "vertical"
    local total, widest = 0, 0
    for _, it in ipairs(list) do
      total = total + (vertical and it.h or it.w)
      widest = math.max(widest, vertical and it.w or it.h)
    end
    total = total + gap * (#list - 1)
    local pos = total / 2
    for _, it in ipairs(list) do
      local size = vertical and it.h or it.w
      local along = pos - size / 2
      if vertical then
        it.o.x, it.o.y = cross(it.w, widest), along
      else
        it.o.x, it.o.y = -along, cross(it.h, widest)
      end
      pos = pos - size - gap
    end
  elseif l.arrange == "grid" then
    local cols = math.max(1, math.floor(l.columns))
    local cw, chh = 0, 0
    for _, it in ipairs(list) do cw, chh = math.max(cw, it.w), math.max(chh, it.h) end
    local rows = math.ceil(#list / cols)
    local n = math.min(cols, #list)
    local W = n * cw + (n - 1) * gap
    local H = rows * chh + (rows - 1) * gap
    for i, it in ipairs(list) do
      local col, row = (i - 1) % cols, (i - 1) // cols
      it.o.x = -W / 2 + cw / 2 + col * (cw + gap)
      it.o.y = H / 2 - chh / 2 - row * (chh + gap)
    end
  end
end

--- Arruma os filhos dos objetos com Grupo.
function M.arrange_all(scene)
  for _, o in ipairs(scene.object_list) do
    if o.alive and o.layout and o.layout.arrange ~= "none" then arrange(o, o.layout) end
  end
end

--- Carrega o que o componente precisa (imagem, corpo, som...).
function M.prepare(scene, o, c, live)
  local t = c.type
  if t == "sprite" then
    c._img, c._sheet = nil, false
    c._anims = M.load_anims(scene, c.image)
    if c._anims and c.frame_w <= 0 and c.frame_h <= 0 and c._anims.frame_w > 0 then
      -- Sem tamanho de quadro no componente: o do .anim.
      c.frame_w, c.frame_h = c._anims.frame_w, c._anims.frame_h
    end
    if c._anims and c.animation ~= "" and not c._anims.by_name[c.animation] then
      error(("objeto \"%s\": a animação \"%s\" não existe em %s (tem: %s)")
        :format(o.name, c.animation, M.anim_path(c.image), anim_names(c._anims)), 0)
    end
    if c.image ~= "" then
      if c.frame_w > 0 and c.frame_h > 0 then
        c._img = nn.assets.sheet(c.image, c.frame_w, c.frame_h, { pixel_art = c.pixel_art })
        c._sheet = true
      else
        c._img = nn.assets.texture(c.image, { pixel_art = c.pixel_art })
      end
    end
  elseif t == "body" then
    make_body(scene, o, c)
  elseif t == "map" then
    if c.map ~= "" and not o.map then
      local x, y = o:world_transform()
      o.map = nn.tilemap.load(c.map, { scale = c.scale, x = x, y = y })
      if c.collision and scene.world then scene.world:add_tilemap(o.map) end
      scene._first_map = scene._first_map or o.map
    end
  elseif t == "sound" then
    o.sound = nil
    if c.sound ~= "" then
      o.sound = nn.assets.sound(c.sound)
      if c.autoplay and not live then
        local playing = nn.audio.play(o.sound, { loop = c.loop, volume = c.volume, channel = c.channel })
        table.insert(scene._playing, playing)
      end
    end
  elseif t == "particles" then
    make_particles(o, c)
  elseif M.UI_KINDS[t] then
    prepare_ui(scene, o, c)
  elseif t == "camera" then
    scene._camera = scene._camera or o
  end
end

-- Cena ----------------------------------------------------------------------

local function sort_objects(scene)
  local order = {}
  for i, o in ipairs(scene.object_list) do order[o] = i end
  local list = {}
  for _, o in ipairs(scene.object_list) do
    if o.alive then table.insert(list, o) end
  end
  table.sort(list, function(a, b)
    if a.layer ~= b.layer then return a.layer < b.layer end
    return order[a] < order[b]
  end)
  scene._draw_list = list
end

function M.teardown(scene)
  for _, s in ipairs(scene._playing or {}) do
    if s.stop then s:stop() end
  end
  scene._playing = {}
end

local PREPARE_ORDER = {
  "map", "sprite", "body", "sound", "particles", "panel", "button", "bar", "icons",
  "slider", "checkbox", "textfield", "camera",
}

-- Mapas antes dos corpos: a câmera e a colisão dependem deles.
local function prepare_all(scene, list)
  for _, kind in ipairs(PREPARE_ORDER) do
    for _, o in ipairs(list) do
      for _, c in ipairs(o.components) do
        if c.type == kind then M.prepare(scene, o, c) end
      end
    end
  end
  for _, o in ipairs(list) do
    for _, c in ipairs(o.components) do
      if c.type == M.BEHAVIOR then make_behavior(scene, o, c) end
    end
    for _, b in ipairs(o.behaviors) do
      if b.player then o._player = true end
    end
    M.events.add_object(scene, o)
  end
end

local function needs_world(list)
  for _, o in ipairs(list) do
    for _, c in ipairs(o.components) do
      if c.type == "body" or (c.type == "map" and c.collision) then return true end
    end
  end
  return false
end

local function ensure_world(scene)
  if not scene.world then
    scene.world = nn.physics.world({ gravity = scene._gravity or -1800 })
  end
end

--- Monta (ou remonta) os objetos da cena a partir dos dados do arquivo.
function M.setup(scene, data, path)
  M.teardown(scene)
  scene._data, scene._path = data, path
  scene.objects, scene.object_list = {}, {}
  scene._by_id, scene._by_body = {}, {}
  scene._camera, scene._first_map = nil, nil
  scene._prefabs, scene._defs, scene._fx, scene._next_id = nil, nil, nil, nil
  scene._anims = nil
  scene._gravity = data.gravity or -1800
  apply_options()
  if M.ui then M.ui.setup(scene) end
  M.events.setup(scene, data, path)
  scene.background = (data.background ~= nil and data.background ~= "") and data.background or nil

  for _, od in ipairs(data.objects or {}) do build_object(scene, od, nil) end
  -- Física só se algum objeto precisar.
  scene.world = nil
  if needs_world(scene.object_list) then ensure_world(scene) end
  prepare_all(scene, scene.object_list)
  M.arrange_all(scene)
  sort_objects(scene)

  -- Aberta por cima de outra cena: a câmera e o fundo são os da de baixo.
  if not scene.overlay then
    if scene.background then nn.window.set_clear_color(scene.background) end
    nn.camera.set_zoom(1)
    nn.camera.set_position(0, 0)
    M.update_camera(scene, 0, true)
  end
  start_behaviors(scene.object_list)
end

--- Cria um objeto a partir de um prefab, com a cena já rodando.
function M.spawn(scene, path, x, y, opts)
  local data = { prefab = path, x = x or 0, y = y or 0 }
  for k, v in pairs(opts or {}) do data[k] = v end
  local first = #scene.object_list + 1
  local o = build_object(scene, data, opts and opts.parent or nil, true)
  local list = {}
  for i = first, #scene.object_list do table.insert(list, scene.object_list[i]) end
  if needs_world(list) then ensure_world(scene) end
  prepare_all(scene, list)
  if o.parent then table.insert(o.parent.children, o) end
  scene._dirty_list = true
  start_behaviors(list)
  return o
end

-- Pontos de reinício (checkpoint) por cena; valem ao reiniciar a cena depois
-- de perder.
M.checkpoints = {}

function M.update_camera(scene, dt, snap)
  local cam = scene._camera
  if not (cam and cam.alive and cam:shown()) then return end
  local c = cam.camera
  nn.camera.set_zoom(c.zoom)
  local target = c.follow ~= "" and scene.objects[c.follow] or nil
  local bounds = (c.limit_to_map and scene._first_map) or nil
  if target then
    local x, y = target:world_transform()
    if target.body and target._px then
      local a = nn.time.alpha()
      x = target._px + (target.x - target._px) * a
      y = target._py + (target.y - target._py) * a
    end
    if snap then
      nn.camera.set_position(x, y)
      if bounds then nn.camera.clamp(bounds) end
    else
      nn.camera.follow(x, y, { smooth = c.smooth, bounds = bounds })
    end
  else
    local x, y = cam:world_transform()
    nn.camera.set_position(x, y)
    if bounds then nn.camera.clamp(bounds) end
  end
end

local function sync_bodies_before(scene)
  for _, o in ipairs(scene.object_list) do
    if o.alive and o.body and o.body.alive then
      -- O script mudou x/y à mão: leva o corpo junto.
      if o.x ~= o._sx or o.y ~= o._sy then
        local x, y = o:world_transform()
        o.body:set_position(x, y)
        o._px, o._py = o.x, o.y
      end
    end
  end
end

local function sync_bodies_after(scene)
  for _, o in ipairs(scene.object_list) do
    if o.alive and o.body and o.body.alive then
      o._px, o._py = o.x, o.y
      local bx, by = o.body.x, o.body.y
      if o.parent then
        local px, py = o.parent:world_transform()
        bx, by = bx - px, by - py
      end
      o.x, o.y = bx, by
      o._sx, o._sy = o.x, o.y
    end
  end
end

function M.fixed_update(scene, dt)
  each_behavior(scene, "fixed_update", dt)
  -- Objetos sem corpo físico com velocidade (vx, vy) andam sozinhos.
  for _, o in ipairs(scene.object_list) do
    if o.alive and not o.body and (o.vx or o.vy) then
      o.x = o.x + (o.vx or 0) * dt
      o.y = o.y + (o.vy or 0) * dt
    end
  end
  if not scene.world then return {} end
  sync_bodies_before(scene)
  local events = scene.world:step(dt)
  sync_bodies_after(scene)
  for _, e in ipairs(events) do
    e.object_a = e.a and scene._by_body[e.a.id] or nil
    e.object_b = e.b and scene._by_body[e.b.id] or nil
  end
  -- Comportamentos dos dois objetos recebem o toque: contact(outro, e).
  for _, e in ipairs(events) do
    for _, pair in ipairs({ { e.object_a, e.object_b }, { e.object_b, e.object_a } }) do
      local o, other = pair[1], pair[2]
      if o and o.alive then
        for _, b in ipairs(o.behaviors) do
          if b.enabled and type(b.contact) == "function" then b:contact(other, e) end
        end
      end
    end
  end
  return events
end

-- Tira da lista os objetos destruídos e põe os novos na ordem de desenho.
local function refresh_list(scene)
  if scene._dirty_list then
    scene._dirty_list = false
    local alive = {}
    for _, o in ipairs(scene.object_list) do
      if o.alive then table.insert(alive, o) end
    end
    scene.object_list = alive
    sort_objects(scene)
  end
end

function M.update(scene, dt)
  refresh_list(scene)
  if M.ui then M.ui.update(scene, dt) end
  each_behavior(scene, "update", dt)
  refresh_list(scene)
  M.events.step(scene, dt)
  refresh_list(scene)
  update_animations(scene, dt)
  for _, o in ipairs(scene.object_list) do
    if o.particles then
      local x, y = o:world_transform()
      o.particles:set_position(x, y)
      o.particles:update(dt)
    end
  end
  for _, fx in pairs(scene._fx or {}) do fx:update(dt) end
  M.arrange_all(scene)
  if not scene.overlay then M.update_camera(scene, dt, false) end
end

-- Desenha um painel com centro em (x, y) no mundo, ou em (hx, hy) na tela.
local function panel_at(hud, x, y, hx, hy, w, h, opts)
  if w <= 0 or h <= 0 then return end
  if hud then
    nn.screen.panel(hx - w / 2, hy - h / 2, w, h, opts)
  else
    nn.draw.panel(x, y, w, h, opts)
  end
end

local function text_at(hud, x, y, hx, hy, str, opts)
  if hud then nn.screen.text(str, hx, hy, opts) else nn.draw.text(str, x, y, opts) end
end

local function image_at(hud, img, x, y, hx, hy, opts)
  if hud then
    opts.origin_x, opts.origin_y = 0.5, 0.5
    nn.screen.sprite(img, hx, hy, opts)
  else
    nn.draw.sprite(img, x, y, opts)
  end
end

local function circle_at(hud, x, y, hx, hy, r, color)
  if hud then nn.screen.circle(hx, hy, r, color) else nn.draw.circle(x, y, r, color) end
end

local function ui_state(o, c)
  if not (M.ui and c._item) then return "normal" end
  return M.ui.state(o.scene, c._item)
end

-- Cor de fundo de um botão em cada estado.
local function button_color(c, state)
  if c._img then
    if state == "hover" then return c.hover_color ~= "" and c.hover_color or "#FFF2C2" end
    if state == "press" then return c.press_color ~= "" and c.press_color or "#BBBBBB" end
    if state == "disabled" then return "#888888AA" end
    return c.color ~= "" and c.color or "#FFFFFF"
  end
  if state == "hover" then return themed(c.hover_color, "hover") end
  if state == "press" then return themed(c.press_color, "press") end
  if state == "disabled" then return M.theme("disabled") end
  return themed(c.color, "button")
end

local function panel_opts(c, color, border_color)
  local opts = {
    color = color, radius = c.radius, border = c.border,
    border_color = border_color or themed(c.border_color, "border"),
  }
  if c._img then
    opts.image, opts.slice, opts.slice_scale = c._img, c.slice, c.slice_scale
  end
  return opts
end

-- Desenha um componente de interface. `at(dx, dy)` dá a posição (mundo e
-- tela) de um ponto deslocado dx, dy (y para cima) do centro do objeto.
local function draw_ui(o, c, hud, at, sx, sy)
  local t = c.type
  local x, y, hx, hy = at(0, 0)
  if t == "panel" then
    panel_at(hud, x, y, hx, hy, c.w * sx, c.h * sy, panel_opts(c, themed(c.color, "panel")))
  elseif t == "button" then
    local state = ui_state(o, c)
    local w, h = c.w * sx, c.h * sy
    local bc = state == "hover" and themed(c.border_color, "accent") or nil
    panel_at(hud, x, y, hx, hy, w, h, panel_opts(c, button_color(c, state), bc))
    local tc = themed(c.text_color, "text")
    if state == "disabled" then tc = "#FFFFFF80" end
    local size = c.size * sy
    local topts = { size = size, color = tc, anchor = "center", font = M.font(c.font) }
    local dx = 0
    if c._icon then
      local isz = h * 0.6
      local tw = c.text ~= "" and nn.text_size(c.text, topts) or 0
      local gap = c.text ~= "" and size * 0.4 or 0
      local total = isz + gap + tw
      local ix, iy, ihx, ihy = at(-total / 2 + isz / 2, 0)
      image_at(hud, c._icon, ix, iy, ihx, ihy, { w = isz, h = isz })
      dx = -total / 2 + isz + gap + tw / 2
    end
    if c.text ~= "" then
      local tx, ty, thx, thy = at(dx, 0)
      text_at(hud, tx, ty, thx, thy, c.text, topts)
    end
  elseif t == "bar" then
    local w, h = c.w * sx, c.h * sy
    panel_at(hud, x, y, hx, hy, w, h, {
      color = c.back_color, radius = c.radius, border = c.border,
      border_color = themed(c.border_color, "border"),
    })
    local f = bar_fraction(c)
    if f > 0 then
      local fw, fh, dx, dy = w, h, 0, 0
      if c.direction == "left" then
        fw = w * f; dx = w / 2 - fw / 2
      elseif c.direction == "up" then
        fh = h * f; dy = -h / 2 + fh / 2
      elseif c.direction == "down" then
        fh = h * f; dy = h / 2 - fh / 2
      else
        fw = w * f; dx = -w / 2 + fw / 2
      end
      local fx, fy, fhx, fhy = at(dx, dy)
      panel_at(hud, fx, fy, fhx, fhy, fw, fh, {
        color = themed(c.color, "accent"), radius = math.min(c.radius, fw / 2, fh / 2),
      })
    end
  elseif t == "icons" then
    local n, k = icons_count(c)
    local size, gap = c.size * sy, c.spacing * sx
    local total = k * size + math.max(0, k - 1) * gap
    local start = -total / 2
    if c.align == "left" then start = 0 elseif c.align == "right" then start = -total end
    for i = 1, k do
      local ix, iy, ihx, ihy = at(start + (i - 1) * (size + gap) + size / 2, 0)
      local color = i <= n and c.color or c.empty_color
      if c._img then
        local opts = { w = size, h = size, color = color }
        if c.frame_w > 0 and c.frame_h > 0 then opts.frame = math.max(1, math.floor(c.frame)) end
        image_at(hud, c._img, ix, iy, ihx, ihy, opts)
      else
        circle_at(hud, ix, iy, ihx, ihy, size / 2, i <= n and themed("", "accent") or c.empty_color)
      end
    end
  elseif t == "slider" then
    local state = ui_state(o, c)
    local w, h = c.w * sx, c.h * sy
    panel_at(hud, x, y, hx, hy, w, h, { color = c.back_color, radius = h / 2 })
    local span = c.max - c.min
    local f = span ~= 0 and (slider_value(c) - c.min) / span or 0
    f = math.max(0, math.min(1, f))
    if f > 0 then
      local fx, fy, fhx, fhy = at(-w / 2 + w * f / 2, 0)
      panel_at(hud, fx, fy, fhx, fhy, w * f, h, { color = themed(c.color, "accent"), radius = h / 2 })
    end
    local kx, ky, khx, khy = at(-w / 2 + w * f, 0)
    local r = c.knob * sy / 2
    if state == "hover" or state == "press" then
      circle_at(hud, kx, ky, khx, khy, r * 1.35, "#FFFFFF55")
    end
    circle_at(hud, kx, ky, khx, khy, r, c.knob_color)
  elseif t == "checkbox" then
    local state = ui_state(o, c)
    local box = c.box * sy
    local topts = { size = c.size * sy, color = themed(c.text_color, "text"), anchor = "left", font = M.font(c.font) }
    local tw = c.text ~= "" and (nn.text_size(c.text, topts) + box * 0.4) or 0
    local total = box + tw
    local bx, by, bhx, bhy = at(-total / 2 + box / 2, 0)
    local border = state == "hover" and themed("", "accent") or themed("", "border")
    panel_at(hud, bx, by, bhx, bhy, box, box, {
      color = themed(c.color, "panel"), radius = box * 0.2, border = math.max(2, box * 0.08),
      border_color = border,
    })
    if checked(c) then
      local inner = box * 0.5
      panel_at(hud, bx, by, bhx, bhy, inner, inner, { color = themed(c.check_color, "accent"), radius = inner * 0.25 })
    end
    if c.text ~= "" then
      local tx, ty, thx, thy = at(-total / 2 + box + box * 0.4, 0)
      text_at(hud, tx, ty, thx, thy, c.text, topts)
    end
  elseif t == "textfield" then
    local state = ui_state(o, c)
    local w, h = c.w * sx, c.h * sy
    local focused = state == "hover" and c._item and M.ui.focused(o.scene) == c._item
    panel_at(hud, x, y, hx, hy, w, h, {
      color = c.color, radius = c.radius, border = c.border,
      border_color = focused and themed("", "accent") or themed(c.border_color, "border"),
    })
    local value = field_text(c)
    local pad = math.min(12, w * 0.1)
    local tx, ty, thx, thy = at(-w / 2 + pad, 0)
    local topts = { size = c.size * sy, color = themed(c.text_color, "text"), anchor = "left", font = M.font(c.font) }
    if value == "" and not focused then
      topts.color = "#FFFFFF66"
      text_at(hud, tx, ty, thx, thy, c.placeholder, topts)
    elseif value ~= "" then
      text_at(hud, tx, ty, thx, thy, value, topts)
    end
    if focused and (nn.time.elapsed() % 1) < 0.6 then
      local vw = value ~= "" and nn.text_size(value, topts) or 0
      local cx, cy, chx, chy = at(-w / 2 + pad + vw + 2, 0)
      panel_at(hud, cx, cy, chx, chy, 2, c.size * sy, { color = themed(c.text_color, "text") })
    end
  end
end

local function draw_object(o, alpha)
  local x, y, rot, sx, sy = o:world_transform()
  if o.body and o._px and not o.parent then
    x = o._px + (o.x - o._px) * alpha
    y = o._py + (o.y - o._py) * alpha
  end
  -- HUD: a âncora na tela mais a posição (y para cima); sem âncora, o
  -- centro da tela, como estaria com a câmera no começo (0, 0).
  local hx, hy = M.screen_pos(o, x, y)
  local overlay = o.scene and o.scene.overlay
  local function at(dx, dy)
    return x + dx, y + dy, hx + dx, hy - dy
  end
  for _, c in ipairs(o.components) do
    local t = c.type
    local hud = c.hud or overlay
    if t == "sprite" and c._img then
      local w, h = c.w, c.h
      if w <= 0 then w = c._sheet and c.frame_w or c._img.width end
      if h <= 0 then h = c._sheet and c.frame_h or c._img.height end
      -- Repetir: cada cópia tem o tamanho do componente; a escala do objeto
      -- diz quantas cópias cabem.
      local tw, th
      if c.fill == "repeat" then tw, th = w, h end
      local frame = nil
      if c._sheet then
        frame = math.max(1, math.min(math.floor(c.frame), c._img.count))
      end
      if hud then
        nn.screen.sprite(c._img, hx, hy, {
          frame = frame, origin_x = 0.5, origin_y = 0.5,
          w = w * sx, h = h * sy, rotation = -rot * DEG,
          color = c.color, flip_x = c.flip_x, flip_y = c.flip_y, tile_w = tw, tile_h = th,
        })
      else
        nn.draw.sprite(c._img, x, y, {
          frame = frame,
          w = w * sx, h = h * sy, rotation = rot * DEG,
          color = c.color, flip_x = c.flip_x, flip_y = c.flip_y, tile_w = tw, tile_h = th,
        })
      end
    elseif t == "shape" then
      local draw = hud and nn.screen or nn.draw
      local px, py = x, y
      if hud then px, py = hx, hy end
      if c.shape == "circle" then
        draw.circle(px, py, c.radius * math.max(math.abs(sx), math.abs(sy)), c.color)
      else
        local w, h = c.w * math.abs(sx), c.h * math.abs(sy)
        if hud then
          draw.rect(px - w / 2, py - h / 2, w, h, c.color)
        else
          draw.rect(px, py, w, h, c.color)
        end
      end
    elseif t == "text" then
      local opts = { size = c.size * math.abs(sy), color = c.color, anchor = c.anchor, font = M.font(c.font) }
      if hud then
        nn.screen.text(c.text, hx, hy, opts)
      else
        nn.draw.text(c.text, x, y, opts)
      end
    elseif t == "map" and o.map then
      o.map:draw()
    elseif t == "particles" and o.particles then
      o.particles:draw()
    elseif M.UI_KINDS[t] then
      draw_ui(o, c, hud, at, math.abs(sx), math.abs(sy))
    end
  end
end

function M.draw(scene)
  local alpha = nn.time.alpha()
  -- Por cima de outra cena: o fundo é um véu sobre a cena de baixo.
  if scene.overlay and scene.background then
    local W, H = nn.window.size()
    nn.screen.rect(0, 0, W, H, scene.background)
  end
  for _, o in ipairs(scene._draw_list or {}) do
    if o.alive and o:shown() then draw_object(o, alpha) end
  end
  for _, fx in pairs(scene._fx or {}) do fx:draw() end
  each_behavior(scene, "draw")
end

--- Ajuste ao vivo vindo do editor: muda um campo de um objeto ou componente.
function M.live(scene, msg)
  if msg.field == "events" then
    -- Regras da cena (id 0) ou de um objeto.
    local owner = nil
    if msg.id ~= 0 then
      owner = scene._by_id and scene._by_id[msg.id]
      if not owner then return end
    end
    M.events.live(scene, owner, msg.value or {})
    return
  end
  local o = scene._by_id and scene._by_id[msg.id]
  if not o then return end
  if msg.field == "tiles" then
    -- Pintura no editor: { { layer, col, row, tile }, ... } (col e row a partir de 1).
    if o.map then
      for _, c in ipairs(msg.value or {}) do
        pcall(o.map.set_tile, o.map, c[2], c[3], c[4], c[1])
      end
    end
    return
  end
  if msg.component then
    local c = o.components[msg.component]
    if not c then return end
    c[msg.field] = msg.value
    if c.type == M.BEHAVIOR then
      local inst = c._inst
      if msg.field == "script" or not inst then
        -- Trocou o script: o comportamento começa de novo.
        for i, b in ipairs(o.behaviors) do
          if b == inst then table.remove(o.behaviors, i) break end
        end
        local b = make_behavior(scene, o, c)
        if b and type(b.start) == "function" then b:start() end
      else
        inst[msg.field] = msg.value
      end
    elseif c.type ~= "map" and c.type ~= "camera" then
      M.prepare(scene, o, c, true)
    end
    return
  end
  local field = msg.field
  if field == "name" and scene.objects[o.name] == o then
    scene.objects[o.name] = nil
  end
  o[field] = msg.value
  if field == "name" and scene.objects[o.name] == nil then
    scene.objects[o.name] = o
  elseif field == "layer" then
    sort_objects(scene)
  elseif (field == "x" or field == "y") and o.body and o.body.alive then
    local x, y = o:world_transform()
    o.body:set_position(x, y)
    o._sx, o._sy, o._px, o._py = o.x, o.y, o.x, o.y
  elseif (field == "scale_x" or field == "scale_y") then
    local b = o:component("body")
    if b then make_body(scene, o, b) end
  end
end

-- Editor ao vivo -------------------------------------------------------------

local function simple(v)
  local t = type(v)
  return t == "number" or t == "string" or t == "boolean"
end

--- Onde estão os objetos da cena (e as propriedades dos comportamentos e as
--- variáveis do jogo), para o NN Studio mostrar enquanto o jogo roda.
function M.state(scene)
  local objects = {}
  for _, o in ipairs(scene.object_list or {}) do
    if o.alive and o.id > 0 then
      local wx, wy, wr, wsx, wsy = o:world_transform()
      local e = {
        id = o.id, x = o.x, y = o.y, rotation = o.rotation,
        scale_x = o.scale_x, scale_y = o.scale_y, visible = o.visible,
        wx = wx, wy = wy, wr = wr, wsx = wsx, wsy = wsy,
      }
      local props = {}
      for ci, c in ipairs(o.components) do
        local inst = c._inst
        if inst then
          local p = {}
          for _, d in ipairs(inst._props or {}) do
            local v = inst[d.key]
            if simple(v) then p[d.key] = v end
          end
          props[tostring(ci)] = p
        end
      end
      e.props = props
      table.insert(objects, e)
    end
  end
  local vars = {}
  for k, v in pairs(nn.vars or {}) do
    if type(k) == "string" and simple(v) then vars[k] = v end
  end
  return { scene = scene._path, objects = objects, vars = vars, fired = M.events.take_fired(scene) }
end

--- O objeto da cena (do arquivo) sob o ponto x, y do mundo.
function M.pick(scene, x, y)
  local list = scene._draw_list or {}
  for i = #list, 1, -1 do
    local o = list[i]
    if o.alive and o:shown() then
      local ox, oy = o:world_transform()
      local hw, hh = o:bounds()
      if math.abs(x - ox) <= hw and math.abs(y - oy) <= hh then
        -- Parte de um prefab ou criado pelo jogo: o dono que está no arquivo.
        local owner = o
        while owner and owner.id <= 0 do owner = owner.parent end
        if owner then return owner.id end
      end
    end
  end
  return nil
end

-- Liga tudo numa cena comum ---------------------------------------------------

local function load_script(path)
  local src = nn.assets.read(path)
  local chunk, err = load(src, "@" .. path)
  if not chunk then error(err, 0) end
  local module = chunk()
  if type(module) ~= "table" then
    error(path .. ": o script da cena deve terminar com `return` de uma tabela", 0)
  end
  return module
end

--- Cria a cena a partir dos dados de um .cena.
function M.new(data, path)
  if type(data) ~= "table" then
    error(path .. ": o arquivo da cena deve devolver uma tabela", 0)
  end
  local script = {}
  if data.script and data.script ~= "" then script = load_script(data.script) end
  local scene = setmetatable({}, { __index = script })
  scene.overlay = data.overlay or script.overlay

  local function call(name, self, ...)
    local f = script[name]
    if f then return f(self, ...) end
  end

  function scene:enter(args)
    -- "Abrir cena por cima": a de baixo continua aparecendo, parada.
    if type(args) == "table" and args.por_cima then self.overlay = true end
    M.setup(self, data, path)
    local cp = M.checkpoints[path]
    if args and args.checkpoint and cp then
      for _, p in ipairs(M.targets(self, "")) do p:move_to(cp.x, cp.y) end
    else
      M.checkpoints[path] = nil
    end
    call("enter", self, args)
  end

  function scene:reload()
    -- O arquivo mudou: os objetos voltam ao que está nele; o resto do
    -- estado do script continua.
    M.setup(self, data, path)
    call("reload", self)
  end

  function scene:exit()
    call("exit", self)
    M.teardown(self)
  end

  function scene:resume()
    if self.background and not self.overlay then nn.window.set_clear_color(self.background) end
    call("resume", self)
  end

  function scene:fixed_update(dt)
    call("fixed_update", self, dt)
    local events = M.fixed_update(self, dt)
    if script.contact then
      for _, e in ipairs(events) do script.contact(self, e) end
    end
  end

  function scene:update(dt)
    M.update(self, dt)
    call("update", self, dt)
  end

  function scene:draw()
    M.draw(self)
    call("draw", self)
  end

  --- Objeto pelo nome (o primeiro, se houver vários).
  function scene:find(name)
    return self.objects[name]
  end

  --- Todos os objetos com esse nome.
  function scene:find_all(name)
    local out = {}
    for _, o in ipairs(self.object_list) do
      if o.alive and o.name == name then table.insert(out, o) end
    end
    return out
  end

  --- Cria um objeto a partir de um prefab: scene:spawn("prefabs/tiro.prefab", x, y).
  function scene:spawn(prefab, x, y, opts)
    return M.spawn(self, prefab, x, y, opts)
  end

  --- Objetos que um comportamento procura ("" = o jogador).
  function scene:targets(who)
    return M.targets(self, who)
  end

  --- Faíscas num ponto.
  function scene:burst(x, y, color, count)
    M.burst(self, x, y, color, count)
  end

  --- Guarda o ponto de reinício desta cena.
  function scene:set_checkpoint(x, y)
    M.checkpoints[path] = { x = x, y = y }
  end

  --- Último ponto de reinício (x, y), ou nada.
  function scene:checkpoint()
    local c = M.checkpoints[path]
    if c then return c.x, c.y end
  end

  --- Reinicia a cena (com `checkpoint = true`, o jogador volta ao último
  --- checkpoint).
  function scene:restart(opts)
    opts = opts or {}
    nn.scene.switch(path, { checkpoint = opts.checkpoint }, { fade = opts.fade or 0.5 })
  end

  --- Para o comportamento Botão: entra no foco do teclado e do mouse.
  function scene:_ui_add(item)
    return M.ui and M.ui.add(self, item)
  end

  --- O item de menu com o foco (ou nil).
  function scene:_ui_focused()
    return M.ui and M.ui.focused(self)
  end

  --- Abre uma cena por cima desta (o jogo daqui fica parado embaixo).
  function scene:open_over(p)
    M.open_over(p)
  end

  --- Fecha esta cena, se foi aberta por cima de outra.
  function scene:close_over()
    M.close_over(self)
  end

  function scene:_ui_state(item)
    return M.ui and M.ui.state(self, item) or "normal"
  end

  function scene:_live(msg)
    M.live(self, msg)
  end

  -- Chamado pela engine a cada quadro quando o jogo foi aberto pelo editor.
  function scene:_live_tick()
    local now = nn.time.elapsed()
    local state
    if not self._last_report or now - self._last_report >= 0.1 then
      self._last_report = now
      state = M.state(self)
    end
    local picked
    if nn.input.down("ctrl") and nn.input.mouse_pressed("left") then
      local x, y = nn.input.mouse_world()
      if x then picked = M.pick(self, x, y) end
    end
    return state, picked
  end

  return scene
end

return M
