-- Executa as cenas visuais (arquivos .cena) montadas no NN Studio.
--
-- Um .cena é um arquivo Lua que devolve uma tabela com os objetos da cena.
-- Este módulo transforma essa tabela numa cena comum: cria os objetos,
-- carrega imagens e sons, monta a física e desenha tudo sozinho. O script da
-- cena (campo `script`), se houver, recebe os callbacks de sempre (enter,
-- update, fixed_update, draw...) e encontra os objetos em `self.objects`.

local M = {}

local DEG = math.pi / 180

-- Campos de cada componente, na ordem em que o NN Studio grava, com o valor
-- padrão. O editor lê esta mesma lista: ela é a única fonte dos padrões.
M.FIELDS = {
  sprite = {
    { "image", "" }, { "frame_w", 0 }, { "frame_h", 0 }, { "frame", 1 },
    { "w", 0 }, { "h", 0 }, { "color", "#FFFFFF" }, { "flip_x", false },
    { "flip_y", false }, { "pixel_art", true },
  },
  text = {
    { "text", "Texto" }, { "size", 32 }, { "color", "#FFFFFF" }, { "anchor", "center" },
  },
  shape = {
    { "shape", "rect" }, { "w", 64 }, { "h", 64 }, { "radius", 32 }, { "color", "#3FA7D6" },
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

local function defaults(kind)
  local list = M.FIELDS[kind]
  if not list then
    error(("componente desconhecido: \"%s\" (use %s)"):format(
      tostring(kind), "sprite, text, shape, body, map, camera, sound ou particles"), 0)
  end
  return list
end

local function copy_component(src)
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

local function build_object(scene, data, parent)
  local o = setmetatable({}, Object)
  for _, f in ipairs(M.OBJECT_FIELDS) do
    local v = data[f[1]]
    if v == nil then v = f[2] end
    o[f[1]] = v
  end
  o.scene, o.parent, o.alive = scene, parent, true
  o.components, o.children = {}, {}
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
  for _, ch in ipairs(data.children or {}) do
    table.insert(o.children, build_object(scene, ch, o))
  end
  return o
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

--- Monta (ou remonta) os objetos da cena a partir dos dados do arquivo.
function M.setup(scene, data, path)
  M.teardown(scene)
  scene._data, scene._path = data, path
  scene.objects, scene.object_list = {}, {}
  scene._by_id, scene._by_body = {}, {}
  scene._camera, scene._first_map = nil, nil
  scene.background = (data.background ~= nil and data.background ~= "") and data.background or nil

  -- Física só se algum objeto precisar.
  local needs_world = false
  local function scan(list)
    for _, od in ipairs(list or {}) do
      for _, c in ipairs(od.components or {}) do
        if c.type == "body" or (c.type == "map" and c.collision ~= false) then needs_world = true end
      end
      scan(od.children)
    end
  end
  scan(data.objects)
  scene.world = needs_world and nn.physics.world({ gravity = data.gravity or -1800 }) or nil

  for _, od in ipairs(data.objects or {}) do build_object(scene, od, nil) end
  -- Mapas antes dos corpos: a câmera e a colisão dependem deles.
  for _, kind in ipairs({ "map", "sprite", "body", "sound", "particles", "camera" }) do
    for _, o in ipairs(scene.object_list) do
      for _, c in ipairs(o.components) do
        if c.type == kind then M.prepare(scene, o, c) end
      end
    end
  end
  sort_objects(scene)

  if scene.background then nn.window.set_clear_color(scene.background) end
  nn.camera.set_zoom(1)
  nn.camera.set_position(0, 0)
  M.update_camera(scene, 0, true)
end

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
  if not scene.world then return {} end
  sync_bodies_before(scene)
  local events = scene.world:step(dt)
  sync_bodies_after(scene)
  for _, e in ipairs(events) do
    e.object_a = e.a and scene._by_body[e.a.id] or nil
    e.object_b = e.b and scene._by_body[e.b.id] or nil
  end
  return events
end

function M.update(scene, dt)
  if scene._dirty_list then
    scene._dirty_list = false
    local alive = {}
    for _, o in ipairs(scene.object_list) do
      if o.alive then table.insert(alive, o) end
    end
    scene.object_list = alive
    sort_objects(scene)
  end
  for _, o in ipairs(scene.object_list) do
    if o.particles then
      local x, y = o:world_transform()
      o.particles:set_position(x, y)
      o.particles:update(dt)
    end
  end
  M.update_camera(scene, dt, false)
end

local function draw_object(o, alpha)
  local x, y, rot, sx, sy = o:world_transform()
  if o.body and o._px and not o.parent then
    x = o._px + (o.x - o._px) * alpha
    y = o._py + (o.y - o._py) * alpha
  end
  for _, c in ipairs(o.components) do
    local t = c.type
    if t == "sprite" and c._img then
      local w, h = c.w, c.h
      if w <= 0 then w = c._sheet and c.frame_w or c._img.width end
      if h <= 0 then h = c._sheet and c.frame_h or c._img.height end
      nn.draw.sprite(c._img, x, y, {
        frame = c._sheet and c.frame or nil,
        w = w * sx, h = h * sy, rotation = rot * DEG,
        color = c.color, flip_x = c.flip_x, flip_y = c.flip_y,
      })
    elseif t == "shape" then
      if c.shape == "circle" then
        nn.draw.circle(x, y, c.radius * math.max(math.abs(sx), math.abs(sy)), c.color)
      else
        nn.draw.rect(x, y, c.w * math.abs(sx), c.h * math.abs(sy), c.color)
      end
    elseif t == "text" then
      nn.draw.text(c.text, x, y, { size = c.size * math.abs(sy), color = c.color, anchor = c.anchor })
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
end

--- Ajuste ao vivo vindo do editor: muda um campo de um objeto ou componente.
function M.live(scene, msg)
  local o = scene._by_id and scene._by_id[msg.id]
  if not o then return end
  if msg.component then
    local c = o.components[msg.component]
    if not c then return end
    c[msg.field] = msg.value
    if c.type ~= "map" and c.type ~= "camera" then M.prepare(scene, o, c, true) end
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

  function scene:_live(msg)
    M.live(self, msg)
  end

  return scene
end

return M
