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
    { "flip_y", false }, { "pixel_art", true }, { "hud", false },
  },
  text = {
    { "text", "Texto" }, { "size", 32 }, { "color", "#FFFFFF" }, { "anchor", "center" },
    { "hud", false },
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
}

-- Campos do objeto, na ordem de gravação (os filhos e componentes vêm depois).
M.OBJECT_FIELDS = {
  { "id", 0 }, { "name", "Objeto" }, { "x", 0 }, { "y", 0 }, { "rotation", 0 },
  { "scale_x", 1 }, { "scale_y", 1 }, { "visible", true }, { "locked", false },
  { "layer", 0 },
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
      tostring(kind), "sprite, text, shape, body, map, camera, sound, particles ou behavior"), 0)
  end
  return list
end

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
  return c
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

--- Metade da largura e da altura do objeto no mundo: do corpo físico, se
--- houver, senão da imagem ou da forma.
function Object:bounds()
  local b = self.body
  if b and b.alive then
    if b.radius and b.radius > 0 then return b.radius, b.radius end
    return b.width / 2, b.height / 2
  end
  local _, _, _, sx, sy = self:world_transform()
  sx, sy = math.abs(sx), math.abs(sy)
  for _, c in ipairs(self.components) do
    if c.type == "sprite" and c._img then
      local w, h = c.w, c.h
      if w <= 0 then w = c._sheet and c.frame_w or c._img.width end
      if h <= 0 then h = c._sheet and c.frame_h or c._img.height end
      return w * sx / 2, h * sy / 2
    elseif c.type == "shape" then
      if c.shape == "circle" then
        local r = c.radius * math.max(sx, sy)
        return r, r
      end
      return c.w * sx / 2, c.h * sy / 2
    end
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

-- Prefabs -------------------------------------------------------------------

M.PREFAB_EXTENSION = "prefab"

-- Campos próprios de cada instância; o resto vem do prefab.
M.INSTANCE_FIELDS = {
  "id", "name", "x", "y", "rotation", "scale_x", "scale_y", "visible", "locked", "layer",
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
  for _, c in ipairs(data.components or {}) do
    local comp = copy_component(c)
    table.insert(o.components, comp)
    if o[comp.type] == nil and (comp.type == "sprite" or comp.type == "text"
        or comp.type == "shape" or comp.type == "camera") then
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

--- Carrega o que o componente precisa (imagem, corpo, som...).
function M.prepare(scene, o, c, live)
  local t = c.type
  if t == "sprite" then
    c._img, c._sheet = nil, false
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

local PREPARE_ORDER = { "map", "sprite", "body", "sound", "particles", "camera" }

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
  scene._gravity = data.gravity or -1800
  scene.background = (data.background ~= nil and data.background ~= "") and data.background or nil

  for _, od in ipairs(data.objects or {}) do build_object(scene, od, nil) end
  -- Física só se algum objeto precisar.
  scene.world = nil
  if needs_world(scene.object_list) then ensure_world(scene) end
  prepare_all(scene, scene.object_list)
  sort_objects(scene)

  if scene.background then nn.window.set_clear_color(scene.background) end
  nn.camera.set_zoom(1)
  nn.camera.set_position(0, 0)
  M.update_camera(scene, 0, true)
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
  each_behavior(scene, "update", dt)
  refresh_list(scene)
  for _, o in ipairs(scene.object_list) do
    if o.particles then
      local x, y = o:world_transform()
      o.particles:set_position(x, y)
      o.particles:update(dt)
    end
  end
  for _, fx in pairs(scene._fx or {}) do fx:update(dt) end
  M.update_camera(scene, dt, false)
end

local function draw_object(o, alpha)
  local x, y, rot, sx, sy = o:world_transform()
  if o.body and o._px and not o.parent then
    x = o._px + (o.x - o._px) * alpha
    y = o._py + (o.y - o._py) * alpha
  end
  -- HUD: mesma posição de quando a câmera está no começo (0, 0), mas presa
  -- à tela (o centro da janela é o 0, 0).
  local W, H = nn.window.size()
  local hx, hy = W / 2 + x, H / 2 - y
  for _, c in ipairs(o.components) do
    local t = c.type
    if t == "sprite" and c._img then
      local w, h = c.w, c.h
      if w <= 0 then w = c._sheet and c.frame_w or c._img.width end
      if h <= 0 then h = c._sheet and c.frame_h or c._img.height end
      if c.hud then
        nn.screen.sprite(c._img, hx, hy, {
          frame = c._sheet and c.frame or nil, origin_x = 0.5, origin_y = 0.5,
          w = w * sx, h = h * sy, rotation = -rot * DEG,
          color = c.color, flip_x = c.flip_x, flip_y = c.flip_y,
        })
      else
        nn.draw.sprite(c._img, x, y, {
          frame = c._sheet and c.frame or nil,
          w = w * sx, h = h * sy, rotation = rot * DEG,
          color = c.color, flip_x = c.flip_x, flip_y = c.flip_y,
        })
      end
    elseif t == "shape" then
      local draw = c.hud and nn.screen or nn.draw
      local px, py = x, y
      if c.hud then px, py = hx, hy end
      if c.shape == "circle" then
        draw.circle(px, py, c.radius * math.max(math.abs(sx), math.abs(sy)), c.color)
      else
        local w, h = c.w * math.abs(sx), c.h * math.abs(sy)
        if c.hud then
          draw.rect(px - w / 2, py - h / 2, w, h, c.color)
        else
          draw.rect(px, py, w, h, c.color)
        end
      end
    elseif t == "text" then
      local opts = { size = c.size * math.abs(sy), color = c.color, anchor = c.anchor }
      if c.hud then
        nn.screen.text(c.text, hx, hy, opts)
      else
        nn.draw.text(c.text, x, y, opts)
      end
    elseif t == "map" and o.map then
      o.map:draw()
    elseif t == "particles" and o.particles then
      o.particles:draw()
    end
  end
end

function M.draw(scene)
  local alpha = nn.time.alpha()
  for _, o in ipairs(scene._draw_list or {}) do
    if o.alive and o:shown() then draw_object(o, alpha) end
  end
  for _, fx in pairs(scene._fx or {}) do fx:draw() end
  each_behavior(scene, "draw")
end

--- Ajuste ao vivo vindo do editor: muda um campo de um objeto ou componente.
function M.live(scene, msg)
  local o = scene._by_id and scene._by_id[msg.id]
  if not o then return end
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
  return { scene = scene._path, objects = objects, vars = vars }
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
    if self.background then nn.window.set_clear_color(self.background) end
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
