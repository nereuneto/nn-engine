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
    { "font", "" }, { "hud", false }, { "outline", "" },
  },
  shape = {
    { "shape", "rect" }, { "w", 64 }, { "h", 64 }, { "radius", 32 }, { "color", "#3FA7D6" },
    { "hud", false },
  },
  body = {
    { "body", "dynamic" }, { "shape", "box" }, { "w", 32 }, { "h", 32 }, { "radius", 16 },
    -- Deslocamento da colisão em relação ao objeto (pés de um sprite com
    -- espaço vazio embaixo, por exemplo).
    { "ox", 0 }, { "oy", 0 },
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
    -- 3D: o som sai do objeto (mais baixo longe dele), ouvido até `distance`.
    { "spatial", true }, { "distance", 30 },
  },
  -- Partículas: contínuas (fogo, chuva) ou de uma vez (explosão).
  particles = {
    { "mode", "continuo" }, { "rate", 20 }, { "emitting", true }, { "amount", 30 },
    { "on_start", true }, { "interval", 0 }, { "when_done", "nada" },
    { "lifetime", 1 }, { "lifetime_var", 0 }, { "speed_min", 40 }, { "speed_max", 120 },
    { "direction", 90 }, { "spread", 360 }, { "gravity", 0 }, { "drag", 0 },
    { "area", "ponto" }, { "area_w", 100 }, { "area_h", 100 }, { "area_radius", 50 },
    { "size_start", 12 }, { "size_end", 0 }, { "size_var", 0 },
    { "color_start", "#FFFFFF" }, { "color_mid", "" }, { "color_end", "#FFFFFF00" },
    { "image", "" }, { "frame", 1 }, { "frame_w", 0 }, { "frame_h", 0 }, { "spin", 0 },
    { "blend", "alpha" }, { "follow", false }, { "max", 500 }, { "wind", 1 },
    { "pixel", false },
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
  -- 3D: um modelo .glb/.gltf, uma forma pronta e a câmera do jogo.
  model = {
    { "file", "" }, { "color", "#FFFFFF" }, { "flat", false }, { "shadow", true },
    { "glow", 0 }, { "glow_night", false }, { "sway", 0 },
  },
  -- material: "normal" ou "agua" (ondinhas, o céu refletido, o brilho do
  -- sol). sway: quanto balança com o vento (0 a 1).
  shape3d = {
    { "shape", "cube" }, { "color", "#7BC86C" }, { "shadow", true }, { "glow", 0 },
    { "glow_night", false }, { "material", "normal" }, { "sway", 0 },
  },
  -- Luz: uma lâmpada (ilumina em volta, até o alcance) ou um holofote
  -- (ilumina num cone, para a frente do objeto, -Z). "flicker" faz a luz
  -- tremular como fogo (0 a 1).
  light3d = {
    { "kind", "point" }, { "color", "#FFD9A0" }, { "intensity", 1.5 }, { "range", 8 },
    { "angle", 30 }, { "soft", 0.3 }, { "flicker", 0 }, { "on", true }, { "night", false },
  },
  camera3d = {
    { "fov", 60 },
  },
  -- Texto no mundo 3D (placas, nomes, avisos): tamanho das letras em metros,
  -- virado para a câmera ou preso como uma placa (lido de frente pelo +Z).
  text3d = {
    { "text", "Texto" }, { "size", 0.5 }, { "color", "#FFFFFF" }, { "facing", "camera" },
    { "outline", "#000000B0" }, { "background", "" }, { "font", "" },
  },
  -- Terreno: alturas e cores (`data`, feito pelo NN Studio), o tamanho, as
  -- cores das quatro camadas e a água.
  terrain = {
    { "data", "" }, { "cells", 64 }, { "width", 64 }, { "depth", 64 }, { "height", 20 },
    { "color1", "#7BC86C" }, { "color2", "#9C7049" }, { "color3", "#9EA3A8" }, { "color4", "#F2D49B" },
    { "water", false }, { "water_level", -1 }, { "water_color", "#3FA7D6B4" },
  },
  -- A animação que o modelo 3D do objeto toca (as do arquivo .glb).
  anim3d = {
    { "animation", "" }, { "speed", 1 }, { "loop", true },
  },
  -- Preso a um osso do modelo do pai (uma espada na mão do herói): o objeto
  -- acompanha o osso quando o pai anima. A posição, a rotação e a escala do
  -- objeto contam a partir do osso.
  bone = {
    { "bone", "" },
  },
  -- Corpo da física 3D. Forma "auto": parado (ou kinematic) com modelo usa a
  -- malha exata; o resto, uma caixa, esfera, cápsula ou cilindro do tamanho
  -- do modelo ou da forma.
  body3d = {
    { "body", "static" }, { "shape", "auto" }, { "sensor", false }, { "bounce", 0 },
    { "friction", 0.5 }, { "mass", 0 }, { "lock_rotation", false }, { "gravity_scale", 1 },
    { "tag", "" },
  },
}

-- Formas do componente body3d.
M.SHAPES_BODY3D = { "auto", "box", "sphere", "capsule", "cylinder", "mesh" }

-- Formas prontas do componente shape3d (a malha de nn.d3 de cada uma).
M.SHAPES3D = { "cube", "sphere", "cylinder", "cone", "capsule", "plane" }

-- Ambiente de uma cena 3D: céu, sol, luz ambiente, névoa e sombras.
M.ENVIRONMENT = {
  { "sky", true }, { "sky_top", "#3F8FD8" }, { "sky_horizon", "#CDEBF5" },
  { "sky_bottom", "#9AA79A" },
  { "sun_height", 50 }, { "sun_turn", 120 }, { "sun_color", "#FFF1D6" }, { "sun_size", 4 },
  { "ambient_sky", "#93AFCF" }, { "ambient_ground", "#6A5D48" },
  { "fog", true }, { "fog_near", 30 }, { "fog_far", 90 },
  { "shadows", true }, { "shadow_distance", 30 },
  -- Dia e noite: o sol anda no céu e as cores mudam com a hora. Um dia do
  -- jogo dura `day_length` minutos; o jogo começa na hora `start_hour`.
  { "cycle", false }, { "day_length", 4 }, { "start_hour", 9 },
  -- Vento: força (0 = parado, 1 = brisa, 3 = ventania) e para onde sopra
  -- (graus: 0 = para -Z, 90 = para +X).
  { "wind", 0 }, { "wind_turn", 60 },
  -- Halo do brilho: quanto o que brilha ganha de halo (0 = nada, 1 = o normal).
  { "bloom", 1 },
}

-- Ambientes prontos (o NN Studio aplica com um clique): o que muda em cada
-- um. Dia é o padrão.
M.ENVIRONMENT_PRESETS = {
  { "dia", "Dia", {} },
  { "amanhecer", "Amanhecer", {
    sky_top = "#5A6FB0", sky_horizon = "#F4B183", sky_bottom = "#7A6A70",
    sun_height = 8, sun_color = "#FFC79A", sun_size = 5,
    ambient_sky = "#A08EB0", ambient_ground = "#5E5048", fog_near = 25, fog_far = 85,
  } },
  { "entardecer", "Entardecer", {
    sky_top = "#3B4F8C", sky_horizon = "#F7A35C", sky_bottom = "#6B5A5A",
    sun_height = 8, sun_color = "#FFB070", sun_size = 6,
    ambient_sky = "#8C7FA8", ambient_ground = "#5A4638", fog_near = 25, fog_far = 80,
  } },
  { "noite", "Noite", {
    sky_top = "#0B1026", sky_horizon = "#22305A", sky_bottom = "#0E1220",
    sun_height = 35, sun_color = "#3D4A78", sun_size = 2,
    ambient_sky = "#16203A", ambient_ground = "#0C0C12", fog_near = 20, fog_far = 70,
  } },
  { "caverna", "Caverna", {
    sky = false, sun_color = "#000000", sun_size = 0,
    ambient_sky = "#1C1C26", ambient_ground = "#0C0B0A", fog_near = 8, fog_far = 35,
    shadows = false,
  } },
}

--- O ambiente pronto `nome` (dia, entardecer, noite, caverna) completo:
--- os campos dele por cima do padrão. nil se não existir.
function M.environment_preset(name)
  for _, p in ipairs(M.ENVIRONMENT_PRESETS) do
    if p[1] == name then
      local e = {}
      for _, f in ipairs(M.ENVIRONMENT) do e[f[1]] = f[2] end
      for k, v in pairs(p[3]) do e[k] = v end
      return e
    end
  end
end

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

-- Avisos no console (cada um uma vez só): o jogo continua.
local warned = {}
M.warnings = {}
function M.warn(msg)
  if warned[msg] then return end
  warned[msg] = true
  table.insert(M.warnings, msg)
  if nn._warn then nn._warn(msg) else print("aviso: " .. msg) end
end

-- A mensagem de um erro de arquivo, curta: "o arquivo x não existe".
local function file_error(file, e)
  e = tostring(e):match("^[^\n]*") or tostring(e)
  if e:find("não encontrado", 1, true) then
    return ("o arquivo %s não existe"):format(file)
  end
  return e
end
M.file_error = file_error

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
  -- 3D: altura (z é a profundidade), rotação em graus em volta de X, Y e Z
  -- e a escala em Z.
  { "z", 0 }, { "rx", 0 }, { "ry", 0 }, { "rz", 0 }, { "scale_z", 1 },
}

-- Campos da cena.
M.SCENE_FIELDS = {
  { "script", "" }, { "background", "" }, { "gravity", -1800 }, { "overlay", false },
  { "mode", "2d" },
}

-- Comportamento: `script` e as propriedades dele, livres (vêm do script).
M.BEHAVIOR = "behavior"

local function defaults(kind)
  local list = M.FIELDS[kind]
  if not list then
    error(("componente desconhecido: \"%s\" (use %s)"):format(
      tostring(kind), "sprite, text, shape, body, map, camera, sound, particles, panel, "
      .. "button, bar, icons, slider, checkbox, textfield, layout, model, shape3d, light3d, camera3d "
      .. "ou behavior"), 0)
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

--- 3D: a matriz do objeto no mundo (com as dos pais). Com `alpha` (de
--- nn.time.alpha()), quem anda pela física fica entre o passo anterior e o
--- atual, para o movimento ficar liso.
function Object:matrix3d(alpha)
  local x, y, z = self.x, self.y, self.z
  local p = self._p3
  if alpha and p then
    x, y, z = p[1] + (x - p[1]) * alpha, p[2] + (y - p[2]) * alpha, p[3] + (z - p[3]) * alpha
  end
  local pm = self.parent and self.parent:matrix3d(alpha) or false
  -- Preso a um osso: o pai, depois o osso dele agora (muda a cada quadro).
  if self._bone and pm then
    local a = M.model_animator(self.parent)
    local b = a and a:bone(self._bone)
    if b then pm = pm * b end
  end
  -- A última matriz fica guardada: o que não se mexeu (a maioria das coisas
  -- de uma cena) não monta outra a cada quadro.
  local c = self._m3
  local rx, ry, rz, sx, sy, sz = self.rx, self.ry, self.rz, self.scale_x, self.scale_y, self.scale_z
  if c and c[1] == x and c[2] == y and c[3] == z and c[4] == rx and c[5] == ry and c[6] == rz
      and c[7] == sx and c[8] == sy and c[9] == sz and c[10] == pm then
    return c.m
  end
  local m = nn.d3.matrix({ x = x, y = y, z = z, rx = rx, ry = ry, rz = rz, sx = sx, sy = sy, sz = sz })
  if pm then m = pm * m end
  if not c then
    c = {}
    self._m3 = c
  end
  c[1], c[2], c[3], c[4], c[5], c[6], c[7], c[8], c[9], c[10] = x, y, z, rx, ry, rz, sx, sy, sz, pm
  c.m = m
  return m
end

--- 3D: onde o objeto está no mundo (x, y, z).
function Object:position3d(alpha)
  return self:matrix3d(alpha):position()
end

--- Toca o som do componente Som do objeto (no 3D, saindo do objeto).
function Object:play_sound(opts)
  local c = self:component("sound")
  if not (c and self.sound) then
    error(("objeto \"%s\" não tem um som"):format(self.name), 2)
  end
  local o = { loop = c.loop, volume = c.volume, channel = c.channel }
  for k, v in pairs(opts or {}) do o[k] = v end
  if self.scene and self.scene._3d and c.spatial and o.x == nil then
    o.x, o.y, o.z = self:position3d()
    o.distance = o.distance or c.distance
  end
  local s = nn.audio.play(self.sound, o)
  if o.x then c._playing = s end
  return s
end

--- 3D: quem toca as animações do modelo do objeto (nil sem modelo animado).
function M.model_animator(o)
  for _, c in ipairs(o.components) do
    if c.type == "model" and c._anim then return c._anim end
  end
  return nil
end

function Object:animator()
  return M.model_animator(self)
end

--- 3D: toca uma animação do modelo ("andar"); veja modelo:animator().
function Object:play(name, opts)
  local a = M.model_animator(self)
  if not a then
    error(("objeto \"%s\": o modelo dele não tem animações"):format(self.name), 2)
  end
  return a:play(name, opts)
end

--- 3D: a caixa do objeto no espaço dele (min e max), sem escala: do
--- modelo, da forma 3D ou um cubinho.
function Object:local_bounds3d()
  for _, c in ipairs(self.components) do
    if c.type == "model" and c._model then return c._model:bounds() end
    if c.type == "shape3d" and c._mesh then
      if c.shape == "plane" then return -0.5, -0.01, -0.5, 0.5, 0.01, 0.5 end
      if c.shape == "capsule" then return -0.25, -0.5, -0.25, 0.25, 0.5, 0.25 end
      return -0.5, -0.5, -0.5, 0.5, 0.5, 0.5
    end
  end
  return -0.25, -0.25, -0.25, 0.25, 0.25, 0.25
end

--- 3D: a caixa do objeto no mundo: min x, y, z e max x, y, z.
function Object:bounds3d()
  local b, ch = self.body, self._char3
  if ch and b and b.alive then
    local r, h = ch.radius, ch.height
    return b.x - r, b.y, b.z - r, b.x + r, b.y + h, b.z + r
  end
  local x0, y0, z0, x1, y1, z1 = self:local_bounds3d()
  local m = self:matrix3d()
  local inf = math.huge
  local ax, ay, az, bx, by, bz = inf, inf, inf, -inf, -inf, -inf
  for i = 0, 7 do
    local px = (i % 2 == 0) and x0 or x1
    local py = (i % 4 < 2) and y0 or y1
    local pz = (i < 4) and z0 or z1
    local wx, wy, wz = m:transform_point(px, py, pz)
    ax, ay, az = math.min(ax, wx), math.min(ay, wy), math.min(az, wz)
    bx, by, bz = math.max(bx, wx), math.max(by, wy), math.max(bz, wz)
  end
  return ax, ay, az, bx, by, bz
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
  if self.scene then self.scene._targets = nil end
  if self._nav_static and self.scene then self.scene._nav_dirty = true end
  if self.body and self.body.alive then
    if self._s3 and self.scene and self.scene._by_body3 then
      self.scene._by_body3[self.body.id] = nil
    end
    self.body:remove()
  end
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
    M.events.signal(self.scene, { kind = "mensagem", obj = self, msg = msg, arg = (...),
      arg2 = select(2, ...) })
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
  -- Até 100 ícones: com um número enorme (vidas = 1e9), o desenho não trava.
  n = math.min(100, math.max(0, math.floor(n)))
  return n, math.min(100, math.max(n, math.floor(c.max or 0)))
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
  if self.scene and self.scene._3d and not M.is_hud(self) then
    -- 3D: a caixa no mundo (em metros), em X e Y. (O corpo 3D não tem
    -- largura e altura como o 2D.)
    local x0, y0, _, x1, y1 = self:bounds3d()
    return (x1 - x0) / 2, (y1 - y0) / 2
  end
  if b and b.alive and b.width then
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
  if self.scene and self.scene._3d then
    -- 3D: as caixas (em metros) se cruzam?
    margin = margin or 0.05
    local a0, a1, a2, a3, a4, a5 = self:bounds3d()
    local b0, b1, b2, b3, b4, b5 = other:bounds3d()
    return a0 <= b3 + margin and b0 <= a3 + margin and a1 <= b4 + margin
      and b1 <= a4 + margin and a2 <= b5 + margin and b2 <= a5 + margin
  end
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

--- Leva o objeto (e o corpo físico) para x, y (e z, no 3D).
function Object:move_to(x, y, z)
  if self.scene and self.scene._3d then
    self.x, self.y, self.z = x, y, z or self.z
    self._p3 = nil
    local b = self.body
    if b and b.alive then
      local wx, wy, wz = self:position3d()
      b:set_position(wx, wy + (self._char3 and self._char3.feet or 0), wz)
      b:set_velocity(0, 0, 0)
      self._s3 = { self.x, self.y, self.z, self.rx, self.ry, self.rz }
    end
    return
  end
  self.x, self.y = x, y
  if self.body and self.body.alive then
    local wx, wy = self:world_transform()
    self.body:set_position(wx + (self._box or 0), wy + (self._boy or 0))
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
  local layout = false
  for _, c in ipairs(o.components) do
    if c.hud then return true end
    if c.type == "layout" then layout = true end
  end
  -- Um Grupo que arruma filhos presos à tela também fica na tela.
  if layout then
    for _, ch in ipairs(o.children or {}) do
      if M.is_hud(ch) then return true end
    end
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
  "anchor", "z", "rx", "ry", "rz", "scale_z",
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
  o.start_x, o.start_y, o.start_z = o.x, o.y, o.z
  o._rules = data.events
  for _, c in ipairs(data.components or {}) do
    local comp = copy_component(c)
    table.insert(o.components, comp)
    if o[comp.type] == nil and (comp.type == "sprite" or comp.type == "text"
        or comp.type == "shape" or comp.type == "camera" or comp.type == "layout"
        or comp.type == "model" or comp.type == "shape3d" or comp.type == "camera3d"
        or comp.type == "text3d" or comp.type == "light3d" or M.UI_KINDS[comp.type]) then
      o[comp.type] = comp
    end
  end
  table.insert(scene.object_list, o)
  scene._targets = nil
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
    if v == nil then
      v = p.default
      if scene._3d and p.default3d ~= nil then v = p.default3d end
    end
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
    mode = def.mode, body = def.body,
  }
end

--- Quem um comportamento procura: "" é o jogador (objetos com um
--- comportamento de jogador); senão, os objetos com esse nome.
--- A lista é guardada até o próximo passo (ou até um objeto surgir ou ser
--- destruído): muitos comportamentos perguntam a mesma coisa a cada passo.
function M.targets(scene, who)
  who = who or ""
  local cache = scene._targets
  if not cache then
    cache = {}
    scene._targets = cache
  end
  local found = cache[who]
  if not found then
    found = {}
    for _, o in ipairs(scene.object_list) do
      if o.alive then
        if who == "" then
          for _, b in ipairs(o.behaviors) do
            if b.player then table.insert(found, o) break end
          end
        elseif o.name == who then
          table.insert(found, o)
        end
      end
    end
    cache[who] = found
  end
  local out = {}
  for i = 1, #found do out[i] = found[i] end
  return out
end

-- Combate --------------------------------------------------------------------

--- Os inimigos de quem ataca: para o jogador, quem tem Vida ou é um Inimigo
--- 3D (menos os jogadores); para os outros, os jogadores. Sem ninguém (um
--- tiro solto), todos que têm Vida. Com `who`, os objetos com esse nome.
function M.foes(scene, by, who)
  if who and who ~= "" then return M.targets(scene, who) end
  local out = {}
  if by and not by._player then return M.targets(scene, "") end
  for _, o in ipairs(scene.object_list) do
    if o.alive and o ~= by and not (by and o._player) then
      for _, b in ipairs(o.behaviors) do
        local s = b.component.script
        if b.enabled and (type(b.dano) == "function" or s == "nn:inimigo3d") then
          table.insert(out, o)
          break
        end
      end
    end
  end
  return out
end

--- 3D: a frente de `o` no chão (o +Z do modelo, girado com ele): dx, dz.
function M.front3(o)
  local m = o:matrix3d()
  local x0, _, z0 = m:position()
  local x1, _, z1 = m:transform_point(0, 0, 1)
  local dx, dz = x1 - x0, z1 - z0
  local n = math.sqrt(dx * dx + dz * dz)
  if n < 1e-3 then return 0, 1 end
  return dx / n, dz / n
end

--- 3D: quem `by` pode acertar na frente dele, até `range` metros (da borda
--- de cada um) e dentro de `angle` graus (360 = em volta), na altura dele.
--- Do mais perto para o mais longe; com `all` falso, só o mais perto.
function M.in_front(scene, by, range, angle, who, all)
  local x, _, z = by:position3d()
  local _, y0, _, _, y1 = by:bounds3d()
  local fx, fz = M.front3(by)
  local half = math.cos(math.rad(math.min(angle, 360) / 2))
  local found = {}
  for _, a in ipairs(M.foes(scene, by, who)) do
    if a.alive and a ~= by then
      local ax, _, az = a:position3d()
      local bx0, ay0, bz0, bx1, ay1, bz1 = a:bounds3d()
      local r = math.max(bx1 - bx0, bz1 - bz0) / 2
      local dx, dz = ax - x, az - z
      local d = math.sqrt(dx * dx + dz * dz)
      local tall = ay1 >= y0 - 0.3 and ay0 <= y1 + 0.3
      local near = d - r <= range
      local ahead = d <= r + 0.2 or angle >= 360 or (dx * fx + dz * fz) / math.max(d, 1e-6) >= half
      if tall and near and ahead then found[#found + 1] = { a, d } end
    end
  end
  table.sort(found, function(p, q) return p[2] < q[2] end)
  local out = {}
  for i, p in ipairs(found) do
    out[i] = p[1]
    if all == false then break end
  end
  return out
end

--- `by` acerta `target` com `amount` de dano: tira vida (comportamento Vida),
--- empurra para longe de `by` (ou na direção `opts.dx, opts.dz`) e manda
--- "acertou" (alvo, dano) para quem bateu; se a vida acabar, o Vida manda
--- "derrotou". Sem Vida, um Inimigo 3D ou um Perigo é derrotado de uma vez.
--- Devolve true se acertou, false se o alvo estava invencível e nil se não
--- dá para machucar ele.
function M.hit(scene, by, target, amount, opts)
  opts = opts or {}
  if not (target and target.alive) then return nil end
  amount = amount or 1
  local ok = target:send("dano", amount, by)
  if ok == nil then
    if not (target:behavior("inimigo3d") or target:behavior("perigo") or opts.kill) then return nil end
    if by and by.alive then by:send("acertou", target, amount) end
    if scene._3d then
      local x, y, z = target:position3d()
      scene:burst(x, y + 0.4, "#FF6B6B", 20, z)
    else
      local x, y = target:world_transform()
      scene:burst(x, y, "#FF6B6B", 20)
    end
    if by and by.alive then by:send("derrotou", target) end
    target:send("derrotado", by)
    target:destroy()
    return true
  end
  if not ok then return false end
  local push = opts.push or 0
  local b = target.body
  if push > 0 and target.alive and b and b.alive then
    if scene._3d then
      local dx, dz = opts.dx, opts.dz
      if not dx and by then
        local x, _, z = by:position3d()
        local tx, _, tz = target:position3d()
        dx, dz = tx - x, tz - z
      end
      dx, dz = dx or 0, dz or 1
      local n = math.sqrt(dx * dx + dz * dz)
      if n < 1e-3 then dx, dz, n = 0, 1, 1 end
      b:set_velocity(dx / n * push, push * (opts.up or 0.5), dz / n * push)
      target.empurrado = opts.stun or 0.3
    else
      local dx = opts.dx
      if not dx and by then
        local x = by:world_transform()
        local tx = target:world_transform()
        dx = tx >= x and 1 or -1
      end
      b.vx = (dx or 1) >= 0 and push or -push
      b.vy = push * (opts.up or 0.5)
      target.empurrado = opts.stun or 0.25
    end
  end
  if by and by.alive then by:send("acertou", target, amount) end
  -- Pausa no acerto, só quando o jogador bate ou apanha.
  local stop = opts.hitstop or 0
  if stop > 0 and ((by and by._player) or target._player) then M.hitstop(scene, stop) end
  return true
end

--- Para o jogo por `seconds` (uma fração de segundo): a "pausa no acerto"
--- que dá peso aos golpes. O desenho continua (com o tremor da câmera).
function M.hitstop(scene, seconds)
  scene._hitstop = math.max(scene._hitstop or 0, math.min(seconds or 0.05, 0.3))
end

--- 3D: um lugar no chão até `radius` metros de (x, z), onde dá para andar
--- (pela grade dos caminhos; sem ela, um raio procura o chão) e a pelo menos
--- `avoid` metros dos jogadores. Devolve x, y (o chão), z; sem achar, o
--- próprio ponto.
function M.ground_spot(scene, x, y, z, radius, avoid)
  radius, avoid = radius or 0, avoid or 0
  local nav = M.nav(scene)
  local players = M.targets(scene, "")
  local tries = radius > 0 and 40 or 1
  for i = 1, tries do
    local px, pz = x, z
    if radius > 0 then
      local a = math.random() * math.pi * 2
      local r = radius * math.sqrt(math.random())
      px, pz = x + math.cos(a) * r, z + math.sin(a) * r
    end
    local py
    if nav then
      if nav:walkable(px, pz) then py = nav:floor(px, pz) end
    else
      local hit = M.world3(scene):raycast(px, y + 8, pz, 0, -1, 0, 30)
      py = hit and hit.y
    end
    -- Longe do jogador (menos na última tentativa).
    if py and avoid > 0 and i < tries then
      for _, p in ipairs(players) do
        local qx, _, qz = p:position3d()
        if (qx - px) ^ 2 + (qz - pz) ^ 2 < avoid * avoid then py = nil break end
      end
    end
    if py then return px, py, pz end
  end
  return x, y, z
end

--- Treme a câmera (`strength` em pixels; no 3D, 80 pixels = 1 metro) por
--- `seconds` segundos.
function M.shake(scene, strength, seconds)
  seconds = seconds or 0.3
  if scene._3d then
    scene._shake3 = { force = (strength or 8) / 80, t = seconds, total = math.max(seconds, 1e-3) }
  else
    nn.camera.shake(strength or 8, seconds)
  end
end

--- Um número subindo de cima de `o` (o dano que ele levou), que some
--- sozinho. `color`: a cor (padrão: amarelo; vermelho no jogador).
function M.damage_number(scene, o, n, color)
  local num = { n = n, t = 0, dx = math.random() - 0.5,
    color = color or (o._player and "#FF5A5A" or "#FFE066") }
  if scene._3d then
    local x0, _, z0, x1, y1, z1 = o:bounds3d()
    num.x, num.y, num.z = (x0 + x1) / 2, y1 + 0.2, (z0 + z1) / 2
  else
    local x, y = o:world_transform()
    local _, hh = o:bounds()
    num.x, num.y = x, y + hh + 6
  end
  scene._numbers = scene._numbers or {}
  table.insert(scene._numbers, num)
end

local function update_numbers(scene, dt)
  local list = scene._numbers
  if not list then return end
  for i = #list, 1, -1 do
    local n = list[i]
    n.t = n.t + dt
    if n.t > 0.9 then table.remove(list, i) end
  end
end

local function draw_numbers(scene)
  for _, n in ipairs(scene._numbers or {}) do
    local a = math.max(0, math.min(1, (0.9 - n.t) / 0.35))
    local text = tostring(n.n == math.floor(n.n) and math.floor(n.n) or n.n)
    if scene._3d then
      local alpha = string.format("%02X", math.floor(a * 255))
      nn.d3.text(text, { x = n.x + n.dx * 0.3 * n.t, y = n.y + n.t * 1.1, z = n.z,
        size = 0.42, color = n.color:sub(1, 7) .. alpha, outline = "#000000" .. alpha })
    else
      nn.draw.text(text, n.x + n.dx * 20 * n.t, n.y + n.t * 50,
        { size = 22, color = n.color, alpha = a, anchor = "center" })
    end
  end
end

-- No 3D, o tremor mexe a câmera só no desenho deste quadro: no próximo, ela
-- volta para onde estava (se ninguém a mudou).
local function shake3_apply(scene, dt)
  local sk = scene._shake3
  if not sk then return end
  local c = nn.d3.camera()
  local f = sk.force * (sk.t / sk.total)
  local ox, oy, oz = (math.random() * 2 - 1) * f, (math.random() * 2 - 1) * f, (math.random() * 2 - 1) * f
  scene._shake3_base = c
  nn.d3.camera({ x = c.x + ox, y = c.y + oy, z = c.z + oz,
    target = { c.target.x + ox, c.target.y + oy, c.target.z + oz } })
  sk.t = sk.t - dt
  if sk.t <= 0 then scene._shake3 = nil end
end

local function shake3_restore(scene)
  local c = scene._shake3_base
  if not c then return end
  scene._shake3_base = nil
  nn.d3.camera({ x = c.x, y = c.y, z = c.z, target = { c.target.x, c.target.y, c.target.z } })
end

-- Interações ("E para abrir") ------------------------------------------------
--
-- Quem pode ser usado (Interagir, Porta 3D) se candidata a cada quadro com a
-- distância até o jogador. Só o mais perto mostra o aviso e responde à tecla
-- (E, ou o Y do controle).

--- Distância entre as caixas de dois objetos (0 se encostam): metros no 3D,
--- pixels no 2D.
function M.gap(a, b)
  if a.scene and a.scene._3d then
    local a0, a1, a2, a3, a4, a5 = a:bounds3d()
    local b0, b1, b2, b3, b4, b5 = b:bounds3d()
    local dx = math.max(0, b0 - a3, a0 - b3)
    local dy = math.max(0, b1 - a4, a1 - b4)
    local dz = math.max(0, b2 - a5, a2 - b5)
    return math.sqrt(dx * dx + dy * dy + dz * dz)
  end
  local ax, ay = a:world_transform()
  local bx, by = b:world_transform()
  local aw, ah = a:bounds()
  local bw, bh = b:bounds()
  local dx = math.max(0, math.abs(ax - bx) - aw - bw)
  local dy = math.max(0, math.abs(ay - by) - ah - bh)
  return math.sqrt(dx * dx + dy * dy)
end

--- A tecla de interagir foi apertada neste quadro?
function M.interact_pressed()
  return nn.input.pressed("e") or nn.input.pad_pressed("y")
end

--- O nome da tecla de interagir (a do controle, se ele foi o último usado).
function M.interact_key()
  local ok, dev = pcall(nn.input.last_device)
  return (ok and dev == "gamepad") and "Y" or "E"
end

-- Candidata `owner` (um comportamento) a ser usado por quem está a até
-- `dist` dele. Devolve quem apertou a tecla, se ele foi o escolhido.
function M.interact(scene, owner, who, dist, text, locked)
  -- Numa conversa (ou no quadro em que ela fechou), ninguém interage.
  if scene._dlg or scene._dlg_closed then return nil end
  local o = owner.obj
  local best, near
  for _, p in ipairs(M.targets(scene, who)) do
    if p ~= o then
      local d = M.gap(o, p)
      if d <= dist and (not near or d < near) then best, near = p, d end
    end
  end
  if not best then return nil end
  local cur = scene._ia
  if not cur or near < cur.d then
    scene._ia = { owner = owner, obj = o, d = near, text = text, locked = locked }
  end
  local last = scene._ia_last
  if last and last.owner == owner and M.interact_pressed() then
    scene._ia_used = true
    return best
  end
  return nil
end

--- A escala da interface pronta (caixa de fala, avisos) pelo tamanho da
--- tela: 1 numa tela de 720 de altura; numa tela pequena de pixel art
--- (320x180), um quarto.
function M.ui_scale()
  local _, h = nn.window.size()
  return math.max(0.25, math.min(2, h / 720))
end

-- O aviso do escolhido: a tecla e o que ela faz, em cima do objeto.
local function draw_prompt(scene)
  local ia = scene._ia
  if scene._dlg or not (ia and ia.obj.alive) or ia.text == nil or ia.text == "" then return end
  local o = ia.obj
  local key = M.interact_key()
  local font = M.font("")
  local k = M.ui_scale()
  local size = math.max(7, math.floor(20 * k + 0.5))
  local tw = nn.text_size(ia.text, { size = size, font = font })
  local cap = math.floor(size * 1.4 + 0.5)
  local pad = math.max(2, math.floor(size / 2))
  local gap = math.max(2, math.floor(8 * k + 0.5))
  local showkey = not ia.locked
  local w = pad * 2 + tw + (showkey and (cap + gap) or 0)
  local h = cap + math.max(2, math.floor(12 * k + 0.5))
  local radius = math.max(2, math.floor(10 * k + 0.5))
  local panel = { color = M.theme("panel"), radius = radius, border = 1, border_color = M.theme("border") }
  local tcolor = ia.locked and "#FF9A9A" or M.theme("text")
  if scene._3d then
    local x0, _, z0, x1, y1, z1 = o:bounds3d()
    -- Acima dos filhos também (um nome em cima da cabeça).
    for _, ch in ipairs(o.children) do
      if ch.alive and ch:shown() then
        local _, _, _, _, cy1 = ch:bounds3d()
        y1 = math.max(y1, cy1)
      end
    end
    local sx, sy = nn.d3.to_screen((x0 + x1) / 2, y1 + 0.3, (z0 + z1) / 2)
    if not sx then return end
    local left, top = sx - w / 2, sy - h
    nn.screen.panel(left, top, w, h, panel)
    local x = left + pad
    if showkey then
      nn.screen.panel(x, top + (h - cap) / 2, cap, cap, { color = M.theme("accent"), radius = math.max(2, radius * 0.6) })
      nn.screen.text(key, x + cap / 2, top + h / 2, { size = size, color = "#1E2A3A", anchor = "center", font = font })
      x = x + cap + gap
    end
    nn.screen.text(ia.text, x, top + h / 2, { size = size, color = tcolor, anchor = "left", font = font })
  else
    local x, y = o:world_transform()
    local _, hh = o:bounds()
    local cy = y + hh + gap + h / 2
    -- Sempre inteiro na tela (um personagem na beirada).
    local vx0, vy0, vx1, vy1 = nn.camera.view()
    x = math.max(vx0 + w / 2 + 2, math.min(vx1 - w / 2 - 2, x))
    cy = math.max(vy0 + h / 2 + 2, math.min(vy1 - h / 2 - 2, cy))
    nn.draw.panel(x, cy, w, h, panel)
    local left = x - w / 2 + pad
    if showkey then
      nn.draw.panel(left + cap / 2, cy, cap, cap, { color = M.theme("accent"), radius = math.max(2, radius * 0.6) })
      nn.draw.text(key, left + cap / 2, cy, { size = size, color = "#1E2A3A", anchor = "center", font = font })
      left = left + cap + gap
    end
    nn.draw.text(ia.text, left, cy, { size = size, color = tcolor, anchor = "left", font = font })
  end
end

--- Faíscas num ponto (moeda pega, inimigo derrotado...). No 3D, com z.
function M.burst(scene, x, y, color, count, z)
  color = color or "#FFD166"
  scene._fx = scene._fx or {}
  local fx = scene._fx[color]
  if not fx then
    local colors = { "#FFFFFF", color, color:sub(1, 7) .. "00" }
    if scene._3d then
      fx = nn.particles3d({
        emitting = false, lifetime = { 0.3, 0.6 }, speed = { 1.5, 4 }, drag = 4,
        size = { 0.25, 0 }, colors = colors, blend = "add",
      })
    else
      -- Do tamanho da tela: numa tela pequena (pixel art), quadradinhos.
      local _, y0, _, y1 = nn.camera.view()
      local h = math.abs(y1 - y0)
      local k = math.max(0.2, math.min(2, h / 720))
      local pixel = h <= 360
      fx = nn.particles({
        emitting = false, lifetime = { 0.25, 0.55 }, speed = { 90 * k, 240 * k }, drag = 4,
        size = { pixel and math.max(2, math.floor(12 * k + 0.5)) or 12 * k, 0 },
        colors = colors, blend = pixel and "alpha" or "add", pixel = pixel,
      })
    end
    scene._fx[color] = fx
  end
  fx:burst(count or 16, x, y, z)
end

-- Componentes ---------------------------------------------------------------

local function make_body(scene, o, c)
  if not scene.world then return end
  if o.body and o.body.alive then
    scene._by_body[o.body.id] = nil
    o.body:remove()
  end
  local x, y, _, sx, sy = o:world_transform()
  -- O corpo fica deslocado do objeto (ox, oy, na escala do objeto).
  o._box, o._boy = (c.ox or 0) * sx, (c.oy or 0) * sy
  local opts = {
    type = c.body, x = x + o._box, y = y + o._boy, sensor = c.sensor, one_way = c.one_way,
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
  if c.body == "static" and not c.sensor then
    o._nav_static = true
    scene._nav_dirty = true
  end
  o._sx, o._sy, o._px, o._py = o.x, o.y, o.x, o.y
end

-- Física 3D -----------------------------------------------------------------

--- O mundo da física 3D da cena (criado na primeira vez).
function M.world3(scene)
  if not scene._world3 then
    scene._world3 = nn.physics3d.world({ gravity = scene._gravity })
    scene._by_body3 = scene._by_body3 or {}
  end
  return scene._world3
end

--- Registra um corpo 3D (ou personagem) como o corpo do objeto.
function M.attach_body3(scene, o, body)
  if o.body and o.body.alive and o.body ~= body then
    scene._by_body3[o.body.id] = nil
    o.body:remove()
  end
  o.body = body
  scene._by_body3[body.id] = o
  o._s3 = { o.x, o.y, o.z, o.rx, o.ry, o.rz }
  o._p3 = nil
end

-- A forma do corpo: "auto" escolhe pelo tipo do corpo e pelo que o objeto
-- mostra (modelo ou forma 3D).
local function body3_shape(o, c, opts)
  local model, shape3
  for _, k in ipairs(o.components) do
    if k.type == "model" and k._model then model = model or k._model end
    if k.type == "shape3d" then shape3 = shape3 or k.shape end
  end
  local s = c.shape
  if s == "auto" then
    if model then
      s = (c.body == "dynamic") and "box" or "mesh"
    elseif shape3 == "sphere" or shape3 == "capsule" then
      s = shape3
    elseif shape3 == "cylinder" or shape3 == "cone" then
      s = "cylinder"
    else
      s = "box"
    end
  end
  if s == "mesh" then
    if not model then
      error(("objeto \"%s\": a forma \"mesh\" do Corpo 3D usa os triângulos de um Modelo 3D"):format(o.name), 0)
    end
    if c.body == "dynamic" then
      error(("objeto \"%s\": a forma \"mesh\" é só para corpo parado (static) ou kinematic; use caixa (box)"):format(o.name), 0)
    end
  end
  opts.shape = s
  if model then
    local x0, y0, z0, x1, y1, z1 = model:bounds()
    local w, h, d = x1 - x0, y1 - y0, z1 - z0
    opts.model = model
    if s ~= "box" and s ~= "mesh" then
      -- Centrada na caixa do modelo (que pode ter a origem nos pés).
      opts.offset_x = (x0 + x1) / 2 * math.abs(opts.sx)
      opts.offset_y = (y0 + y1) / 2 * math.abs(opts.sy)
      opts.offset_z = (z0 + z1) / 2 * math.abs(opts.sz)
      if s == "sphere" then
        opts.radius = math.max(w, h, d) / 2
      else
        opts.radius, opts.height = math.max(w, d) / 2, h
      end
    end
  elseif s == "box" then
    if shape3 == "plane" then
      opts.w, opts.h, opts.d = 1, 0.02, 1
      opts.offset_y = -0.01
    else
      opts.size = 1
    end
  elseif s == "capsule" then
    opts.radius, opts.height = 0.25, 1
  elseif s == "sphere" or s == "cylinder" then
    opts.radius, opts.height = 0.5, 1
  end
end

local function make_body3(scene, o, c)
  local world = M.world3(scene)
  local x, y, z, rx, ry, rz, sx, sy, sz = o:matrix3d():decompose()
  local opts = {
    type = c.body, x = x, y = y, z = z, rx = rx, ry = ry, rz = rz, sx = sx, sy = sy, sz = sz,
    sensor = c.sensor, bounce = c.bounce, friction = c.friction,
    mass = (c.mass and c.mass > 0) and c.mass or nil, lock_rotation = c.lock_rotation,
    gravity_scale = c.gravity_scale, tag = (c.tag ~= "" and c.tag) or o.name,
  }
  body3_shape(o, c, opts)
  M.attach_body3(scene, o, world:body(opts))
  if c.body == "static" and not c.sensor then
    o._nav_static = true
    scene._nav_dirty = true
  end
end

-- Antes do passo: o que o jogo mudou à mão (x, y, z, rotação) vai para o corpo.
local function sync_bodies3_before(scene)
  for _, o in ipairs(scene.object_list) do
    local b, s = o.body, o._s3
    if o.alive and s and b and b.alive then
      if o.x ~= s[1] or o.y ~= s[2] or o.z ~= s[3] or o.rx ~= s[4] or o.ry ~= s[5] or o.rz ~= s[6] then
        local x, y, z, rx, ry, rz = o:matrix3d():decompose()
        if o._char3 then
          b:set_position(x, y + o._char3.feet, z)
        else
          b:set_position(x, y, z)
          b:set_rotation(rx, ry, rz)
        end
      end
    end
  end
end

-- Depois do passo: quem se move pela física leva o objeto junto.
local function sync_bodies3_after(scene)
  for _, o in ipairs(scene.object_list) do
    local b = o.body
    local s3 = o._s3
    if o.alive and s3 and b and b.alive and not o._char3 and b.type == "static" then
      -- Parado: nada a interpolar (e nada a alocar a cada passo).
      o._p3 = nil
      s3[1], s3[2], s3[3], s3[4], s3[5], s3[6] = o.x, o.y, o.z, o.rx, o.ry, o.rz
    elseif o.alive and s3 and b and b.alive then
      local p3 = o._p3 or {}
      p3[1], p3[2], p3[3] = o.x, o.y, o.z
      o._p3 = p3
      if o._char3 or b.type ~= "static" then
        local x, y, z = b.x, b.y, b.z
        local rx, ry, rz
        if o._char3 then
          y = y - o._char3.feet
        else
          rx, ry, rz = b.rx, b.ry, b.rz
        end
        if o.parent then
          local inv = o.parent:matrix3d():inverse()
          if rx then
            x, y, z, rx, ry, rz = (inv * nn.d3.matrix({ x = x, y = y, z = z, rx = rx, ry = ry, rz = rz })):decompose()
          else
            x, y, z = inv:transform_point(x, y, z)
          end
        end
        o.x, o.y, o.z = x, y, z
        if rx then o.rx, o.ry, o.rz = rx, ry, rz end
      end
      s3[1], s3[2], s3[3], s3[4], s3[5], s3[6] = o.x, o.y, o.z, o.rx, o.ry, o.rz
    end
  end
end

local function step3(scene, dt)
  if not scene._world3 then return {} end
  sync_bodies3_before(scene)
  local events = scene._world3:step(dt)
  -- (As consultas da física, como a grade de caminhos, valem depois do
  -- primeiro passo.)
  scene._stepped3 = true
  sync_bodies3_after(scene)
  for _, e in ipairs(events) do
    e.object_a = e.a and scene._by_body3[e.a.id] or nil
    e.object_b = e.b and scene._by_body3[e.b.id] or nil
  end
  return events
end

--- As opções do nn.particles de um componente de partículas.
function M.particle_options(c)
  local burst = c.mode == "explosao"
  local colors = { c.color_start }
  if c.color_mid and c.color_mid ~= "" then table.insert(colors, c.color_mid) end
  table.insert(colors, c.color_end)
  local opts = {
    rate = burst and 0 or c.rate,
    emitting = (not burst) and c.emitting,
    lifetime = { c.lifetime * (1 - math.max(0, math.min(1, c.lifetime_var))), c.lifetime },
    speed = { c.speed_min, c.speed_max },
    direction = c.direction * DEG, spread = c.spread * DEG,
    gravity = c.gravity, drag = c.drag,
    size = { c.size_start, c.size_end }, size_variation = c.size_var,
    colors = colors, blend = c.blend, follow = c.follow, pixel = c.pixel,
    spin = { -c.spin * DEG, c.spin * DEG }, max = math.max(1, math.floor(c.max)),
  }
  if c.area == "retangulo" then
    opts.area = { w = c.area_w, h = c.area_h }
  elseif c.area == "circulo" then
    opts.area = { radius = c.area_radius }
  end
  if c.image ~= "" then
    if c.frame_w > 0 and c.frame_h > 0 then
      opts.texture = nn.assets.sheet(c.image, c.frame_w, c.frame_h, { pixel_art = true })
      opts.frame = math.max(1, math.floor(c.frame))
    else
      opts.texture = nn.assets.texture(c.image, { pixel_art = true })
    end
  end
  return opts
end

--- 3D: as mesmas opções, em metros. A direção é o "para cima" do objeto
--- (gire o objeto para apontar), a abertura é um cone em graus e as áreas
--- ficam deitadas (retângulo no chão, círculo no chão).
function M.particle_options3d(c)
  local opts = M.particle_options(c)
  opts.direction = nil
  opts.spread = c.spread
  opts.spin = { -c.spin, c.spin }
  if c.area == "retangulo" then
    opts.area = { w = c.area_w, d = c.area_h }
  elseif c.area == "circulo" then
    opts.area = { radius = c.area_radius }
  else
    opts.area = nil
  end
  return opts
end

-- Para onde o "cima" do objeto aponta no mundo.
local function up3d(o)
  local m = o:matrix3d()
  local x0, y0, z0 = m:position()
  local x1, y1, z1 = m:transform_point(0, 1, 0)
  return x1 - x0, y1 - y0, z1 - z0
end

-- Põe o emissor no objeto (no 3D, com a direção).
local function place_particles(o, fx)
  if fx.is3d then
    fx:set_position(o:position3d())
    fx:set_direction(up3d(o))
  else
    local x, y = o:world_transform()
    fx:set_position(x, y)
  end
end

local function make_particles(o, c, live)
  local three_d = o.scene and o.scene._3d
  local ok, fx
  if three_d then
    ok, fx = pcall(nn.particles3d, M.particle_options3d(c))
  else
    ok, fx = pcall(nn.particles, M.particle_options(c))
  end
  if not ok then error(fx, 0) end
  local old = o.particles
  o.particles = fx
  c._fx = fx
  place_particles(o, fx)
  c._wait = c.interval
  c._active = false
  -- Explosão: solta tudo ao começar (no editor ao vivo, só se ainda não soltou).
  if c.mode == "explosao" and c.on_start and not (live and old) then
    fx:burst(math.floor(c.amount))
    c._active = true
  end
end

--- Solta `n` partículas do emissor do objeto (ou a quantidade dele).
function M.burst_particles(o, n)
  local c = o:component("particles")
  if not (c and c._fx) then return end
  place_particles(o, c._fx)
  c._fx:burst(math.floor(n or c.amount))
  c._active = true
end

--- Liga ou desliga a emissão contínua.
function M.set_emitting(o, on)
  local c = o:component("particles")
  if not (c and c._fx) then return end
  c.emitting = on
  c._fx.emitting = on and c.mode ~= "explosao"
end

-- Um quadro de cada emissor: posição, repetir a explosão e o fim.
local function update_particles(scene, dt)
  for _, o in ipairs(scene.object_list) do
    local k = o.alive and o:component("particles")
    if k and k._kill then
      o:destroy()
    elseif o.alive and o.particles then
      local fx = o.particles
      place_particles(o, fx)
      -- 3D: o vento leva as partículas (o "wind" do componente, de 0 a 1).
      local env = scene._3d and scene._environment
      if env then
        local c0 = o:component("particles")
        local k = (tonumber(env.wind) or 0) * 1.5 * (c0 and tonumber(c0.wind) or 1)
        local t = (tonumber(env.wind_turn) or 60) * DEG
        fx:set_wind(math.sin(t) * k, 0, -math.cos(t) * k)
      end
      fx:update(dt)
      local c = o:component("particles")
      if c and c._fx == fx then
        if c.mode == "explosao" and c.interval > 0 then
          c._wait = (c._wait or c.interval) - dt
          if c._wait <= 0 then
            c._wait = c._wait + c.interval
            fx:burst(math.floor(c.amount))
            c._active = true
          end
        end
        if fx.emitting then c._active = true end
        local repeating = c.mode == "explosao" and c.interval > 0
        -- Terminou: soltou alguma coisa, parou de emitir e todas sumiram.
        if c._active and fx.finished and not repeating then
          c._active = false
          if M.events then M.events.signal(scene, { kind = "particulas", obj = o }) end
          -- Some no quadro seguinte, depois dos eventos verem o aviso.
          if c.when_done == "destruir" then c._kill = true end
        end
      end
    end
  end
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
  if scene.overlay then
    nn.scene.pop()
  elseif scene._guardada then
    nn.scene.pop({ fade = 0.5 })
  end
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
      local ok, snd = pcall(nn.assets.sound, c.sound)
      if ok then
        o.sound = snd
      else
        M.warn(("objeto \"%s\" (Som): %s; ele fica sem som"):format(o.name, file_error(c.sound, snd)))
      end
      if o.sound and c.autoplay and not live then
        local opts = { loop = c.loop, volume = c.volume, channel = c.channel }
        if scene._3d and c.spatial then
          opts.x, opts.y, opts.z = o:position3d()
          opts.distance = c.distance
        end
        local playing = nn.audio.play(o.sound, opts)
        table.insert(scene._playing, playing)
        -- Som 3D que anda com o objeto.
        if opts.x then c._playing = playing end
      end
    end
  elseif t == "particles" then
    make_particles(o, c, live)
  elseif M.UI_KINDS[t] then
    prepare_ui(scene, o, c)
  elseif t == "camera" then
    scene._camera = scene._camera or o
  elseif t == "model" then
    c._model, c._anim, c._missing = nil, nil, nil
    if c.file ~= "" then
      local ok, model = pcall(nn.assets.model, c.file, { flat = c.flat })
      if ok then
        c._model = model
        -- Modelo animado: cada objeto toca as animações dele.
        if #c._model:animations() > 0 then c._anim = c._model:animator() end
      else
        -- O jogo continua: no lugar do modelo, um cubo rosa.
        M.warn(("objeto \"%s\" (Modelo 3D): %s; no lugar dele aparece um cubo rosa"):format(
          o.name, file_error(c.file, model)))
        M._missing_mesh = M._missing_mesh or nn.d3.cube()
        c._missing = M._missing_mesh
      end
    end
    local a = o:component("anim3d")
    if live and a then M.prepare(scene, o, a, true) end
  elseif t == "anim3d" then
    local anim = M.model_animator(o)
    if anim then
      anim.speed = c.speed
      if c.animation ~= "" then
        if not anim:has(c.animation) then
          error(("objeto \"%s\": o modelo não tem a animação \"%s\" (tem: %s)"):format(
            o.name, c.animation, table.concat(anim:animations(), ", ")), 0)
        end
        anim:play(c.animation, { loop = c.loop, blend = 0, restart = true })
      end
    end
  elseif t == "shape3d" then
    local make = nn.d3[c.shape]
    local ok = false
    for _, s in ipairs(M.SHAPES3D) do if s == c.shape then ok = true end end
    if not ok then
      error(("objeto \"%s\": forma 3D desconhecida \"%s\" (use %s)"):format(
        o.name, tostring(c.shape), table.concat(M.SHAPES3D, ", ")), 0)
    end
    c._mesh = make()
  elseif t == "bone" then
    o._bone = nil
    if c.bone ~= "" then
      local a = o.parent and M.model_animator(o.parent)
      if not a then
        M.warn(("objeto \"%s\" (Preso a um osso): o pai precisa de um modelo 3D com esqueleto e animações"):format(o.name))
      elseif not a:bone(c.bone) then
        M.warn(("objeto \"%s\" (Preso a um osso): o modelo de \"%s\" não tem o osso \"%s\" (tem: %s)"):format(
          o.name, o.parent.name, c.bone, table.concat(a:bones(), ", ")))
      else
        o._bone = c.bone
      end
    end
  elseif t == "camera3d" then
    scene._camera3d = scene._camera3d or o
  elseif t == "body3d" then
    if scene._3d then make_body3(scene, o, c) end
  elseif t == "terrain" then
    local opts = { data = c.data, cells = c.cells, width = c.width, depth = c.depth, height = c.height,
      colors = { c.color1, c.color2, c.color3, c.color4 } }
    if c._terrain then
      c._terrain:replace(opts)
    else
      c._terrain = nn.d3.terrain(opts)
    end
    o.terrain = c._terrain
    scene._terrains = scene._terrains or {}
    local listed = false
    for _, x in ipairs(scene._terrains) do if x == o then listed = true end end
    if not listed then table.insert(scene._terrains, o) end
    if scene._3d then M.terrain_body(scene, o, c) end
  end
end

--- O chão do terreno: os triângulos dele (só a posição do objeto conta).
function M.terrain_body(scene, o, c)
  local x, y, z = o:position3d()
  M.attach_body3(scene, o, M.world3(scene):body({
    terrain = c._terrain, x = x, y = y, z = z, tag = o.name,
  }))
  c._ver = c._terrain.version
end

--- 3D: a altura da água dos terrenos em (x, z), ou nil se não houver água.
function M.water_level(scene, x, z)
  for _, o in ipairs(scene._terrains or {}) do
    local c = o.alive and o:component("terrain")
    if c and c.water then
      local ox, oy, oz = o:position3d()
      if math.abs(x - ox) <= c.width / 2 and math.abs(z - oz) <= c.depth / 2 then
        return oy + c.water_level
      end
    end
  end
  return nil
end

-- Caminhos -------------------------------------------------------------------

--- A grade de caminhos da cena (montada na primeira vez que alguém pede, e
--- de novo quando um corpo parado aparece ou some). No 3D, a água dos
--- terrenos também bloqueia. nil se a cena não tem física.
function M.nav(scene)
  if scene._nav and not scene._nav_dirty then return scene._nav end
  scene._nav_dirty = false
  local nav
  if scene._3d then
    if not (scene._world3 and scene._stepped3) then
      scene._nav_dirty = true
      return nil
    end
    local opts = {}
    for k, v in pairs(scene._nav_opts or {}) do opts[k] = v end
    -- A água dos terrenos: o chão debaixo dela não vale (uma ponte vale).
    local water = {}
    for _, o in ipairs(scene._terrains or {}) do
      local c = o.alive and o:component("terrain")
      if c and c.water then
        local x, y, z = o:position3d()
        water[#water + 1] = { x - c.width / 2, z - c.depth / 2, x + c.width / 2, z + c.depth / 2,
          y + c.water_level - 0.2 }
      end
    end
    for _, w in ipairs(opts.water or {}) do water[#water + 1] = w end
    opts.water = water
    nav = scene._world3:nav(opts)
  else
    if not scene.world then return nil end
    local opts = {}
    for k, v in pairs(scene._nav_opts or {}) do opts[k] = v end
    if not opts.area then
      -- A cena toda: os corpos parados e os tiles (o padrão) e também onde
      -- estão os objetos, com uma folga.
      local x0, y0, x1, y1
      for _, o in ipairs(scene.object_list) do
        if o.alive and not M.is_hud(o) then
          local x, y = o:world_transform()
          x0, y0 = math.min(x0 or x, x), math.min(y0 or y, y)
          x1, y1 = math.max(x1 or x, x), math.max(y1 or y, y)
        end
      end
      if x0 then
        -- (Uma grade grossa só para saber onde ficam as paredes.)
        local gx0, gy0, gx1, gy1 = scene.world:nav({ cell = 64, radius = 0 }):bounds()
        local m = 256
        opts.area = { math.min(x0 - m, gx0), math.min(y0 - m, gy0),
          math.max(x1 + m, gx1), math.max(y1 + m, gy1) }
      end
    end
    nav = scene.world:nav(opts)
  end
  scene._nav = nav
  return nav
end

-- Um objeto ou um ponto ({x, y} no 2D, {x, z} no 3D) no chão da grade.
local function nav_point(scene, p)
  if type(p) == "table" and p.alive ~= nil and p.position3d then
    if scene._3d then
      local x, _, z = p:position3d()
      return x, z
    end
    local x, y = p:world_transform()
    return x, y
  end
  if scene._3d then return p.x or p[1], p.z or p[3] or p[2] end
  return p.x or p[1], p.y or p[2]
end

--- Um caminho de `a` até `b` (objetos ou pontos): a lista de pontos e se
--- chega mesmo lá. nil se a cena não tem física ou se `a` está longe de
--- qualquer lugar andável.
function M.path(scene, a, b)
  local nav = M.nav(scene)
  if not nav then return nil, false end
  local ax, ay = nav_point(scene, a)
  local bx, by = nav_point(scene, b)
  return nav:path(ax, ay, bx, by)
end

--- Para os comportamentos: a próxima parada de quem está em (x, y) indo para
--- (tx, ty), contornando o que bloqueia. `st` guarda o caminho entre um
--- quadro e outro (uma tabela do comportamento). Devolve o ponto para onde
--- andar agora (o próprio destino, se dá para ir reto).
function M.steer(scene, st, x, y, tx, ty, dt)
  local nav = M.nav(scene)
  if not nav then return tx, ty end
  st.t = (st.t or 0) + (dt or 1 / 60)
  local reach = nav.cell * 0.6
  -- Reto, se nada no meio.
  if nav:sight(x, y, tx, ty) then
    st.path = nil
    return tx, ty
  end
  local moved = st.tx and ((st.tx - tx) ^ 2 + (st.ty - ty) ^ 2) > (nav.cell * 2) ^ 2
  if not st.path or moved or st.t > 0.5 then
    st.t = 0
    st.tx, st.ty = tx, ty
    local p = nav:path(x, y, tx, ty)
    st.path, st.i = p, 2
  end
  local p = st.path
  if not p or not p[st.i] then return tx, ty end
  local q = p[st.i]
  local qy = scene._3d and q.z or q.y
  -- Chegou nesta parada: vai para a próxima (ou pula, se já dá para ver).
  while p[st.i + 1] do
    local n = p[st.i + 1]
    local ny = scene._3d and n.z or n.y
    local near = ((q.x - x) ^ 2 + (qy - y) ^ 2) < reach * reach
    if near or nav:sight(x, y, n.x, ny) then
      st.i = st.i + 1
      q, qy = n, ny
    else
      break
    end
  end
  return q.x, qy
end

-- Mostra a grade e os caminhos (botão Caminhos do NN Studio, NN_CAMINHOS=1).
local function draw_paths(scene)
  if not (nn._show_paths and nn._show_paths()) then return end
  local nav = M.nav(scene)
  if not nav then return end
  if scene._3d then
    nn.d3.draw(nav:mesh(), { color = "#3CC8FF8C", shadow = false })
    scene._nav_dot = scene._nav_dot or nn.d3.sphere(1)
    for _, o in ipairs(scene.object_list) do
      local st = o.alive and o._nav_state
      if st and st.path then
        for i = (st.i or 2), #st.path do
          local q = st.path[i]
          nn.d3.draw(scene._nav_dot, { x = q.x, y = q.y + 0.25, z = q.z, scale = 0.18,
            color = "#FFD166", shadow = false })
        end
      end
    end
  else
    if not scene._nav_cells or scene._nav_cells_of ~= nav then
      scene._nav_cells, scene._nav_cells_of = nav:cells(), nav
    end
    local s = nav.cell * 0.7
    for _, c in ipairs(scene._nav_cells) do
      nn.draw.rect(c.x, c.y, s, s, "#3CC8FF80")
    end
    for _, o in ipairs(scene.object_list) do
      local st = o.alive and o._nav_state
      if st and st.path then
        for i = (st.i or 2), #st.path do
          local q = st.path[i]
          nn.draw.circle(q.x, q.y, 4, "#FFD166")
        end
      end
    end
  end
end
M.draw_paths = draw_paths

--- Aplica o ambiente de uma cena 3D (céu, sol, luz, névoa e sombras) e
--- devolve os valores usados.
function M.apply_environment(env)
  local e = {}
  for _, f in ipairs(M.ENVIRONMENT) do
    local v = env and env[f[1]]
    if v == nil then v = f[2] end
    e[f[1]] = v
  end
  e.stars = env and env.stars or 0
  if e.sky then
    nn.d3.sky({ top = e.sky_top, horizon = e.sky_horizon, bottom = e.sky_bottom, sun_size = e.sun_size,
      stars = e.stars })
  else
    nn.d3.sky(false)
  end
  local h, t = e.sun_height * DEG, e.sun_turn * DEG
  nn.d3.light({
    sun = { -math.cos(h) * math.sin(t), -math.sin(h), -math.cos(h) * math.cos(t) },
    sun_color = e.sun_color, sky = e.ambient_sky, ground = e.ambient_ground,
    fog = e.fog and { near = e.fog_near, far = e.fog_far } or false,
  })
  nn.d3.shadows(e.shadows and { distance = e.shadow_distance } or false)
  nn.d3.wind({ strength = e.wind, turn = e.wind_turn })
  nn.d3.bloom({ strength = e.bloom })
  return e
end

-- Dia e noite ----------------------------------------------------------------

local function hex_rgb(c)
  c = tostring(c or ""):gsub("^#", "")
  if #c < 6 then return 1, 1, 1 end
  return tonumber(c:sub(1, 2), 16) / 255, tonumber(c:sub(3, 4), 16) / 255, tonumber(c:sub(5, 6), 16) / 255
end

local function rgb_hex(r, g, b)
  local function k(v) return math.floor(math.max(0, math.min(1, v)) * 255 + 0.5) end
  return ("#%02X%02X%02X"):format(k(r), k(g), k(b))
end

--- Mistura duas cores "#RRGGBB" (t = 0: a primeira; 1: a segunda).
function M.mix_color(a, b, t)
  local r1, g1, b1 = hex_rgb(a)
  local r2, g2, b2 = hex_rgb(b)
  return rgb_hex(r1 + (r2 - r1) * t, g1 + (g2 - g1) * t, b1 + (b2 - b1) * t)
end

-- Os campos que mudam aos poucos entre dois ambientes (cores e números).
local BLEND_COLORS = { "sky_top", "sky_horizon", "sky_bottom", "sun_color", "ambient_sky", "ambient_ground" }
local BLEND_NUMBERS = { "sun_size", "fog_near", "fog_far", "stars" }

--- O ambiente no meio do caminho entre `a` e `b` (t de 0 a 1). Os campos
--- que não misturam (céu, névoa, sombras ligados) vêm de `a` até a metade.
function M.mix_environment(a, b, t)
  local e = {}
  for k, v in pairs(a) do e[k] = v end
  if t >= 0.5 then
    for k, v in pairs(b) do e[k] = v end
  end
  for _, k in ipairs(BLEND_COLORS) do e[k] = M.mix_color(a[k], b[k], t) end
  for _, k in ipairs(BLEND_NUMBERS) do
    local x, y = tonumber(a[k]) or 0, tonumber(b[k]) or 0
    e[k] = x + (y - x) * t
  end
  for _, k in ipairs({ "sun_height", "sun_turn" }) do
    local x, y = tonumber(a[k]) or 0, tonumber(b[k]) or 0
    e[k] = x + (y - x) * t
  end
  return e
end

-- As cores do dia: em cada hora, um ambiente (o "dia" é o da própria cena).
local DAY_KEYS = {
  { 0, "noite" }, { 5, "noite" }, { 6.5, "amanhecer" }, { 8.5, "dia" }, { 16, "dia" },
  { 18, "entardecer" }, { 19.5, "noite" }, { 24, "noite" },
}
local STARS = { noite = 1, amanhecer = 0.15, dia = 0, entardecer = 0.1 }

--- De noite (das 18h às 6h)?
function M.is_night_hour(h)
  h = h % 24
  return h >= 18 or h < 6
end

local function smooth(a, b, x)
  local t = math.max(0, math.min(1, (x - a) / (b - a)))
  return t * t * (3 - 2 * t)
end

--- O ambiente de uma cena com Dia e noite na hora `h` (0 a 24): as cores
--- entre os ambientes prontos (o dia é o da cena), o sol cruzando o céu de
--- leste a oeste e, de noite, a lua. Sem o ciclo, devolve o ambiente igual.
function M.environment_at(env, h)
  local base = {}
  for _, f in ipairs(M.ENVIRONMENT) do
    local v = env and env[f[1]]
    if v == nil then v = f[2] end
    base[f[1]] = v
  end
  if not base.cycle then return base end
  h = (tonumber(h) or 12) % 24
  local function key(name)
    local e = name == "dia" and base or M.environment_preset(name)
    local c = {}
    for k, v in pairs(e) do c[k] = v end
    -- Os campos de ligar (céu, névoa, sombras) e a distância das sombras
    -- são sempre os da cena.
    c.sky, c.fog, c.shadows, c.shadow_distance = base.sky, base.fog, base.shadows, base.shadow_distance
    c.wind, c.wind_turn, c.bloom = base.wind, base.wind_turn, base.bloom
    c.cycle, c.day_length, c.start_hour = base.cycle, base.day_length, base.start_hour
    c.stars = STARS[name] or 0
    return c
  end
  local e
  for i = 1, #DAY_KEYS - 1 do
    local a, b = DAY_KEYS[i], DAY_KEYS[i + 1]
    if h >= a[1] and h <= b[1] then
      local t = b[1] > a[1] and (h - a[1]) / (b[1] - a[1]) or 0
      e = M.mix_environment(key(a[2]), key(b[2]), t)
      break
    end
  end
  -- O sol: nasce a leste às 6h, fica mais alto ao meio-dia (a altura do
  -- sol da cena) e se põe a oeste às 18h. De noite, a lua faz o mesmo.
  local alto = math.max(20, math.min(85, tonumber(base.sun_height) or 50))
  local lado = tonumber(base.sun_turn) or 120
  local sol = math.sin(math.pi * (h - 6) / 12)
  if h >= 6 and h <= 18 then
    e.sun_height = math.max(1, sol * alto)
    e.sun_turn = lado - 90 + 180 * (h - 6) / 12
    e.sun_color = M.mix_color("#000000", e.sun_color, smooth(0, 0.12, sol))
  else
    local hn = (h - 18) % 24
    local lua = math.sin(math.pi * hn / 12)
    e.sun_height = math.max(1, lua * 45)
    e.sun_turn = lado + 90 + 180 * hn / 12
    e.sun_color = M.mix_color("#000000", key("noite").sun_color, smooth(0, 0.15, lua))
    e.sun_size = 3
  end
  return e
end

--- O ambiente da cena 3D agora (com Dia e noite, o da hora atual).
function M.refresh_environment(scene)
  if not scene._3d or not scene._data then return end
  local env = scene._data.environment
  local e
  if scene._env_now then
    e = scene._env_now
  elseif scene._cycle then
    -- A hora em passos de meio minuto do jogo: o sol anda aos poucos e a
    -- sombra não treme.
    scene._env_q = math.floor(scene._hour * 120)
    e = M.environment_at(env, scene._env_q / 120)
  else
    e = M.environment_at(env, 12)
  end
  if scene._env_blend then
    local b = scene._env_blend
    e = M.mix_environment(b.from, e, math.min(1, b.t / b.dur))
  end
  scene._environment = M.apply_environment(e)
  -- "De noite" vale também sem o ciclo, num ambiente escuro (noite, caverna).
  if scene._cycle and not scene._env_now then
    scene._night = M.is_night_hour(scene._hour)
  else
    local r, g, b = hex_rgb(scene._environment.sun_color)
    scene._night = (0.3 * r + 0.59 * g + 0.11 * b) < 0.5
  end
end

-- As variáveis do relógio: hora (número, 0 a 23), relogio ("14:30") e dia.
local function clock_vars(scene)
  if not nn.vars then return end
  local h = scene._hour or 12
  nn.vars.hora = math.floor(h)
  nn.vars.relogio = ("%02d:%02d"):format(math.floor(h), math.floor((h % 1) * 60))
  nn.vars.dia = scene._day or 1
end

--- Começa o ambiente de uma cena 3D (na hora de começar, com Dia e noite).
function M.start_environment(scene, env)
  env = env or {}
  scene._cycle = env.cycle == true
  scene._hour = tonumber(env.start_hour) or 9
  scene._day = 1
  scene._time_paused = false
  scene._env_now, scene._env_blend = nil, nil
  M.refresh_environment(scene)
  if scene._cycle then clock_vars(scene) end
end

-- A cada quadro: a hora anda e, ao passar das 6h ou das 18h, os eventos
-- de amanhecer e anoitecer.
local function update_day(scene, dt)
  if not scene._3d then return end
  if scene._env_blend then
    scene._env_blend.t = scene._env_blend.t + dt
    if scene._env_blend.t >= scene._env_blend.dur then
      scene._env_blend = nil
      scene._env_dirty = true
    end
  end
  if scene._cycle and not scene._time_paused then
    local len = math.max(0.05, tonumber(scene._data.environment and scene._data.environment.day_length) or 4)
    local antes = scene._hour
    scene._hour = scene._hour + dt * 24 / (len * 60)
    if scene._hour >= 24 then
      scene._hour = scene._hour - 24
      scene._day = scene._day + 1
    end
    local function passou(h)
      if antes <= scene._hour then return antes < h and scene._hour >= h end
      return antes < h or scene._hour >= h
    end
    if M.events then
      if passou(6) then M.events.signal(scene, { kind = "amanheceu" }) end
      if passou(18) then M.events.signal(scene, { kind = "anoiteceu" }) end
    end
    clock_vars(scene)
  end
  if scene._env_blend or scene._env_dirty then
    scene._env_dirty = nil
    M.refresh_environment(scene)
  elseif scene._cycle and not scene._env_now and math.floor(scene._hour * 120) ~= scene._env_q then
    -- (Só quando a hora passa de um passo para o outro.)
    M.refresh_environment(scene)
  end
end
M.update_day = update_day

--- Muda a hora (0 a 24), com Dia e noite.
function M.set_time(scene, h)
  scene._hour = (tonumber(h) or 0) % 24
  clock_vars(scene)
  -- O céu muda no próximo quadro (no update da cena).
  scene._env_dirty = true
  if scene._cycle then scene._night = M.is_night_hour(scene._hour) end
end

--- Troca o ambiente por um pronto (ou uma tabela), aos poucos em `seconds`.
--- Com Dia e noite, o tempo para até continuar (scene:resume_time()).
function M.change_environment(scene, preset, seconds)
  local e = type(preset) == "table" and preset or M.environment_preset(preset)
  if not e then
    error(("ambiente pronto desconhecido \"%s\" (use dia, amanhecer, entardecer, noite ou caverna)"):format(tostring(preset)), 0)
  end
  if preset == "dia" then
    -- O dia é o da própria cena.
    e = M.environment_at(scene._data.environment, 12)
    e.cycle = false
  elseif type(preset) ~= "table" then
    -- Um pronto: o vento continua o da cena.
    local base = scene._data.environment or {}
    e.wind, e.wind_turn, e.bloom = base.wind or 0, base.wind_turn or 60, base.bloom or 1
  end
  local from = scene._environment
  scene._env_now = e
  if scene._cycle then scene._time_paused = true end
  local dur = tonumber(seconds) or 0
  scene._env_blend = (dur > 0 and from) and { from = from, t = 0, dur = dur } or nil
  scene._env_dirty = true
end

--- Para ou continua o tempo (Dia e noite). Continuando depois de um
--- ambiente trocado, as cores voltam às da hora aos poucos.
function M.pause_time(scene, paused)
  scene._time_paused = paused and true or false
  if not paused and scene._env_now then
    local from = scene._environment
    scene._env_now = nil
    scene._env_blend = from and { from = from, t = 0, dur = 1.5 } or nil
    scene._env_dirty = true
  end
end

--- 3D: põe a câmera do jogo onde está o objeto com camera3d (olhando para
--- a frente dele), ou numa vista padrão.
function M.update_camera3d(scene)
  -- O Jogador 3D com a câmera atrás dele cuida dela.
  if scene._player_cam then return end
  local cam = scene._camera3d
  if cam and cam.alive then
    local m = cam:matrix3d()
    local px, py, pz = m:position()
    local fx, fy, fz = m:forward()
    nn.d3.camera({ x = px, y = py, z = pz, target = { px + fx, py + fy, pz + fz },
      fov = cam.camera3d and cam.camera3d.fov or 60 })
  else
    nn.d3.look_at(0, 5, 10, 0, 0, 0)
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
  -- O Jogador 3D prendeu o mouse: solta ao sair da cena.
  if scene._captured then
    scene._captured = false
    nn.window.capture_mouse(false)
  end
end

local PREPARE_ORDER = {
  "map", "sprite", "body", "sound", "particles", "panel", "button", "bar", "icons",
  "slider", "checkbox", "textfield", "camera", "model", "anim3d", "shape3d", "camera3d", "body3d",
  "terrain", "text3d", "bone",
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
    scene._targets = nil
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
  scene._targets = nil
  scene._by_id, scene._by_body = {}, {}
  scene._camera, scene._first_map, scene._camera3d = nil, nil, nil
  scene._3d = data.mode == "3d"
  scene._prefabs, scene._defs, scene._fx, scene._next_id = nil, nil, nil, nil
  scene._anims = nil
  scene._gravity = data.gravity or (scene._3d and -20 or -1800)
  scene._world3, scene._by_body3, scene._player_cam = nil, {}, nil
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

  if scene._3d then M.start_environment(scene, data.environment) end
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
        o.body:set_position(x + (o._box or 0), y + (o._boy or 0))
        o._px, o._py = o.x, o.y
      end
    end
  end
end

local function sync_bodies_after(scene)
  for _, o in ipairs(scene.object_list) do
    if o.alive and o.body and o.body.alive then
      o._px, o._py = o.x, o.y
      local bx, by = o.body.x - (o._box or 0), o.body.y - (o._boy or 0)
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
  scene._targets = nil
  -- Pausa no acerto: tudo para um instante (o golpe ganha peso).
  if (scene._hitstop or 0) > 0 then
    scene._hitstop = scene._hitstop - dt
    return {}
  end
  each_behavior(scene, "fixed_update", dt)
  -- Objetos sem corpo físico com velocidade (vx, vy) andam sozinhos.
  for _, o in ipairs(scene.object_list) do
    if o.alive and not o.body and (o.vx or o.vy or o.vz) then
      o.x = o.x + (o.vx or 0) * dt
      o.y = o.y + (o.vy or 0) * dt
      if o.vz then o.z = o.z + o.vz * dt end
    end
    -- Tiros 3D somem sozinhos depois de um tempo.
    if o.alive and o._some_em then
      o._some_em = o._some_em - dt
      if o._some_em <= 0 then o:destroy() end
    end
  end
  if scene._3d then
    local events = step3(scene, dt)
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
  scene._targets = nil
  if scene._3d then shake3_restore(scene) end
  scene._dt = dt
  if (scene._hitstop or 0) > 0 then return end
  update_day(scene, dt)
  update_numbers(scene, dt)
  refresh_list(scene)
  if M.ui then M.ui.update(scene, dt) end
  -- Interações: o escolhido no quadro anterior responde à tecla neste.
  scene._ia_last, scene._ia, scene._ia_used = scene._ia, nil, false
  -- A conversa aberta vem antes: os jogadores sabem que devem ficar parados.
  if M.dialogue then M.dialogue.update(scene, dt) end
  each_behavior(scene, "update", dt)
  refresh_list(scene)
  M.events.step(scene, dt)
  refresh_list(scene)
  update_animations(scene, dt)
  if scene._3d then
    -- Terreno mudado pelo script: o chão da física acompanha.
    for _, o in ipairs(scene._terrains or {}) do
      local c = o.alive and o:component("terrain")
      if c and c._terrain and c._ver ~= c._terrain.version then M.terrain_body(scene, o, c) end
    end
    -- Sons 3D que andam com o objeto.
    for _, o in ipairs(scene.object_list) do
      local c = o.alive and o.sound and o:component("sound")
      local s = c and c._playing
      if s then s:set_position(o:position3d()) end
    end
    -- Animações dos modelos 3D; as que tocam uma vez avisam ao terminar.
    for _, o in ipairs(scene.object_list) do
      if o.alive then
        for _, c in ipairs(o.components) do
          local a = c._anim
          if a then
            local was = a:finished()
            a:update(dt)
            if not was and a:finished() then o:send("animacao_terminou", a.animation) end
          end
        end
      end
    end
  end
  update_particles(scene, dt)
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

-- Componentes que o draw_object desenha só na parte 3D (ou que não
-- desenham nada).
local ONLY_3D = {
  model = true, shape3d = true, terrain = true, text3d = true, camera3d = true, body3d = true,
  anim3d = true, behavior = true, sound = true, body = true, camera = true, layout = true,
  bone = true, light3d = true,
}

-- Tremular (fogo): um número de -1 a 1 que muda sem repetir, diferente para
-- cada luz.
local function flicker_noise(t, seed)
  return 0.5 * math.sin(t * 11.0 + seed) + 0.3 * math.sin(t * 23.7 + seed * 1.7)
    + 0.2 * math.sin(t * 5.3 + seed * 2.3)
end
M.flicker_noise = flicker_noise

--- A luz de um componente light3d na matriz `m` (a do objeto): manda para
--- o renderer neste quadro.
function M.draw_light(o, c, m)
  if not c.on or c.intensity <= 0 or c.range <= 0 then return end
  -- Só à noite: acesa do anoitecer ao amanhecer (ou num ambiente escuro).
  if c.night and not (o.scene and o.scene._night) then return end
  local x, y, z = m:position()
  local k, r = c.intensity, c.range
  if c.flicker > 0 then
    local n = flicker_noise(nn.time.elapsed(), (o.id or 0) * 7.13)
    k = k * (1 + c.flicker * 0.35 * n)
    r = r * (1 + c.flicker * 0.06 * n)
  end
  local d = c._lamp
  if not d then
    d = {}
    c._lamp = d
  end
  d.x, d.y, d.z, d.color, d.intensity, d.range = x, y, z, c.color, k, r
  if c.kind == "spot" then
    local fx, fy, fz = m:forward()
    d.direction = d.direction or {}
    d.direction[1], d.direction[2], d.direction[3] = fx, fy, fz
    d.angle, d.soft = c.angle, c.soft
  else
    d.direction = nil
  end
  nn.d3.lamp(d)
end

-- As oito direções do contorno de um texto.
local OUTLINE = { { -1, -1 }, { 0, -1 }, { 1, -1 }, { -1, 0 }, { 1, 0 }, { -1, 1 }, { 0, 1 }, { 1, 1 } }

local function draw_object(o, alpha)
  -- 3D: modelos e formas, com a posição, a rotação e a escala 3D.
  local m3
  for _, c in ipairs(o.components) do
    if c.type == "model" and c._missing then
      m3 = m3 or o:matrix3d(alpha)
      nn.d3.draw(c._missing, { matrix = m3, color = "#FF3EC8" })
    elseif (c.type == "model" and c._model) or (c.type == "shape3d" and c._mesh) then
      m3 = m3 or o:matrix3d(alpha)
      -- As opções ficam no componente (sem uma tabela nova a cada quadro).
      local d = c._draw
      if not d then
        d = {}
        c._draw = d
      end
      d.matrix, d.color, d.shadow, d.animator = m3, o._tint or c.color, c.shadow, c._anim
      -- Brilha só à noite: de dia, nem o brilho do arquivo do modelo.
      local dia = c.glow_night and not (o.scene and o.scene._night)
      d.glow = dia and 0 or c.glow
      d.emissive = not dia
      d.sway = c.sway
      d.material = c.material == "agua" and "water" or nil
      nn.d3.draw(c._model or c._mesh, d)
    elseif c.type == "light3d" and o.scene and o.scene._3d then
      m3 = m3 or o:matrix3d(alpha)
      M.draw_light(o, c, m3)
    elseif c.type == "text3d" and o.scene and o.scene._3d then
      m3 = m3 or o:matrix3d(alpha)
      nn.d3.text(c.text, {
        matrix = m3, face_camera = c.facing ~= "placa", size = c.size, color = c.color,
        outline = c.outline ~= "" and c.outline or nil,
        background = c.background ~= "" and c.background or nil,
        font = M.font(c.font), align = "center",
      })
    elseif c.type == "terrain" and c._terrain then
      local x, y, z = o:position3d()
      nn.d3.draw(c._terrain, { x = x, y = y, z = z })
      if c.water then
        M._water_mesh = M._water_mesh or nn.d3.plane()
        nn.d3.draw(M._water_mesh, { x = x, y = y + c.water_level, z = z, sx = c.width, sz = c.depth,
          color = c.water_color, shadow = false, material = "water" })
      end
    end
  end
  -- Só o que é 3D (a maioria dos objetos de uma cena 3D): acabou.
  local more = false
  for _, c in ipairs(o.components) do
    if not ONLY_3D[c.type] then
      more = true
      break
    end
  end
  if not more then return end
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
          color = o._tint or c.color, flip_x = c.flip_x, flip_y = c.flip_y, tile_w = tw, tile_h = th,
        })
      else
        nn.draw.sprite(c._img, x, y, {
          frame = frame,
          w = w * sx, h = h * sy, rotation = rot * DEG,
          color = o._tint or c.color, flip_x = c.flip_x, flip_y = c.flip_y, tile_w = tw, tile_h = th,
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
      local draw, px, py = nn.draw, x, y
      if hud then draw, px, py = nn.screen, hx, hy end
      -- O contorno: o texto na cor dele, em volta (lê bem em cima do céu).
      if (c.outline or "") ~= "" then
        local d = math.max(1, opts.size / 16)
        local main = opts.color
        opts.color = c.outline
        for _, k in ipairs(OUTLINE) do
          draw.text(c.text, px + k[1] * d, py + k[2] * d, opts)
        end
        opts.color = main
      end
      draw.text(c.text, px, py, opts)
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
  if scene._3d then
    M.update_camera3d(scene)
    shake3_apply(scene, scene._dt or 1 / 60)
  end
  for _, o in ipairs(scene._draw_list or {}) do
    if o.alive and o:shown() then draw_object(o, alpha) end
  end
  for _, fx in pairs(scene._fx or {}) do fx:draw() end
  -- Os corpos da física por cima (rodar_jogo com ver_corpos).
  if M.show_bodies then
    if scene.world then scene.world:debug_draw() end
    if scene._world3 then scene._world3:debug_draw() end
  end
  each_behavior(scene, "draw")
  draw_numbers(scene)
  M.draw_paths(scene)
  draw_prompt(scene)
  if M.dialogue then M.dialogue.draw(scene) end
end

--- Ajuste ao vivo vindo do editor: muda um campo de um objeto ou componente.
function M.live(scene, msg)
  if msg.field == "environment" then
    -- O Ambiente de uma cena 3D (a tabela inteira).
    if scene._3d then
      scene._data.environment = msg.value
      M.start_environment(scene, msg.value)
    end
    return
  end
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
  elseif scene._3d then
    -- 3D: o corpo acompanha no próximo passo; a escala refaz o corpo.
    if field == "scale_x" or field == "scale_y" or field == "scale_z" then
      local b = o:component("body3d")
      if b then make_body3(scene, o, b) end
    end
  elseif (field == "x" or field == "y") and o.body and o.body.alive then
    local x, y = o:world_transform()
    o.body:set_position(x + (o._box or 0), y + (o._boy or 0))
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
  -- O inventário, item por item (para aparecer no que mudou).
  if type(nn.vars._itens) == "table" then
    for _, e in ipairs(nn.vars._itens) do
      if type(e) == "table" and type(e.item) == "string" then
        vars["inventário: " .. e.item] = e.count or 1
      end
    end
  end
  return { scene = scene._path, objects = objects, vars = vars, fired = M.events.take_fired(scene) }
end

--- No 3D: o objeto do mundo mais perto da câmera cuja caixa o raio que sai
--- do ponto (sx, sy) da tela acerta (ou nil).
function M.pick3d(scene, sx, sy, filter)
  local ox, oy, oz, dx, dy, dz = nn.d3.ray(sx, sy)
  if not ox then return nil end
  local best, best_t
  for _, o in ipairs(scene.object_list) do
    if o.alive and o:shown() and not M.is_hud(o) and (not filter or filter(o)) then
      local b = { o:bounds3d() }
      -- Raio contra a caixa (método das fatias).
      local t0, t1 = 0, math.huge
      local p, d = { ox, oy, oz }, { dx, dy, dz }
      for k = 1, 3 do
        local lo, hi = b[k], b[k + 3]
        if math.abs(d[k]) < 1e-9 then
          if p[k] < lo or p[k] > hi then t0 = math.huge break end
        else
          local a, c = (lo - p[k]) / d[k], (hi - p[k]) / d[k]
          if a > c then a, c = c, a end
          t0, t1 = math.max(t0, a), math.min(t1, c)
        end
      end
      if t0 <= t1 and (not best_t or t0 < best_t) then best, best_t = o, t0 end
    end
  end
  return best
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
    -- A cena que está rodando agora (as ferramentas e os testes olham aqui).
    M.current = self
    -- "Abrir cena por cima": a de baixo continua aparecendo, parada.
    if type(args) == "table" and args.por_cima then self.overlay = true end
    -- Aberta guardando a de antes ("Ir para cena", guardando esta):
    -- "Fechar esta cena" volta para ela.
    if type(args) == "table" and args.voltar then self._guardada = true end
    M.setup(self, data, path)
    local cp = M.checkpoints[path]
    if args and args.checkpoint and cp then
      for _, p in ipairs(M.targets(self, "")) do p:move_to(cp.x, cp.y, cp.z) end
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
    M.current = self
    if self.background and not self.overlay then nn.window.set_clear_color(self.background) end
    -- 3D: o céu, o sol e a névoa desta cena de novo (a de cima pode ter
    -- trocado, como uma caverna escura).
    if self._3d and self._data and not self.overlay then M.refresh_environment(self) end
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

  --- 3D: a altura da água em (x, z), ou nil.
  function scene:water_level(x, z)
    return M.water_level(self, x, z)
  end

  --- Faíscas num ponto (no 3D: scene:burst(x, y, cor, quantas, z)).
  function scene:burst(x, y, color, count, z)
    M.burst(self, x, y, color, count, z)
  end

  --- Combate: `by` acerta `target` com `amount` de dano (veja M.hit).
  --- `opts`: push (empurrão), dx/dz (direção), up, stun, kill.
  function scene:hit(target, amount, by, opts)
    return M.hit(self, by, target, amount, opts)
  end

  --- 3D: quem `by` pode acertar na frente dele (veja M.in_front).
  function scene:in_front(by, range, angle, who, all)
    return M.in_front(self, by, range, angle, who, all)
  end

  --- 3D: a frente de `o` no chão (dx, dz).
  function scene:front3(o)
    return M.front3(o)
  end

  --- Quem `by` pode acertar ("" = os inimigos dele).
  function scene:foes(by, who)
    return M.foes(self, by, who)
  end

  --- Um número subindo de cima de `o` (o dano que levou).
  function scene:damage_number(o, n, color)
    M.damage_number(self, o, n, color)
  end

  --- 3D: um lugar no chão em volta de (x, y, z), até `raio` metros, onde
  --- dá para andar e longe `longe` metros dos jogadores (x, y, z).
  function scene:ground_spot(x, y, z, raio, longe)
    return M.ground_spot(self, x, y, z, raio, longe)
  end

  --- Para o jogo um instante (a pausa no acerto), em segundos.
  --- Dia e noite: a hora (0 a 24) e o dia (1, 2, ...).
  function scene:time_of_day()
    return self._hour or 12, self._day or 1
  end

  --- Dia e noite: muda a hora.
  function scene:set_time(h)
    M.set_time(self, h)
  end

  --- Troca o ambiente por um pronto ("dia", "amanhecer", "entardecer",
  --- "noite", "caverna") ou uma tabela, aos poucos em `segundos`.
  function scene:environment(preset, seconds)
    M.change_environment(self, preset, seconds)
  end

  --- Dia e noite: para (true) ou continua (false) o tempo.
  function scene:pause_time(paused)
    M.pause_time(self, paused)
  end

  --- De noite? (Com Dia e noite, das 18h às 6h; sem, num ambiente escuro.)
  function scene:is_night()
    return self._night == true
  end

  function scene:hitstop(seconds)
    M.hitstop(self, seconds)
  end

  --- Um aviso no console (filtro Avisos), uma vez por texto.
  function scene:warn(msg)
    M.warn(msg)
  end

  --- Treme a câmera (no 2D e no 3D).
  function scene:shake(strength, seconds)
    M.shake(self, strength, seconds)
  end

  --- Guarda o ponto de reinício desta cena.
  function scene:set_checkpoint(x, y, z)
    M.checkpoints[path] = { x = x, y = y, z = z }
  end

  --- Último ponto de reinício (x, y, e z no 3D), ou nada.
  function scene:checkpoint()
    local c = M.checkpoints[path]
    if c then return c.x, c.y, c.z end
  end

  --- A grade de caminhos da cena (nav:path, nav:walkable, nav:sight...).
  --- Montada na primeira vez; nil se a cena não tem física.
  function scene:nav()
    return M.nav(self)
  end

  --- Um caminho de `a` até `b` (objetos ou pontos {x, y}; no 3D, {x, z}):
  --- os pontos ({x, y}, ou {x, y, z} no 3D) e se chega mesmo lá.
  function scene:path(a, b)
    return M.path(self, a, b)
  end

  --- Para os comportamentos: a próxima parada de quem está em (x, y) indo
  --- para (tx, ty) contornando paredes (no 3D, x e z). `st` é uma tabela
  --- do comportamento, que guarda o caminho de um quadro para o outro; `dt`
  --- o tempo do passo (o caminho é refeito a cada meio segundo).
  function scene:steer(st, x, y, tx, ty, dt)
    return M.steer(self, st, x, y, tx, ty, dt)
  end

  --- O mundo da física 3D da cena (nn.physics3d), criado na primeira vez.
  function scene:world3()
    return M.world3(self)
  end

  function scene:_nn_attach_body3(o, body)
    M.attach_body3(self, o, body)
  end

  --- Para Interagir e Porta 3D: candidata o comportamento `b` a ser usado
  --- por quem (`who`, vazio = o jogador) está a até `dist` dele, mostrando
  --- "E  <text>". Devolve quem apertou a tecla, se ele foi o escolhido.
  function scene:interaction(b, who, dist, text, locked)
    return M.interact(self, b, who, dist, text, locked)
  end

  --- Distância entre as caixas de dois objetos (0 se encostam).
  function scene:gap(a, b)
    return M.gap(a, b)
  end

  --- Começa a conversa do arquivo `.dialogo` (`quem_fala`, um objeto, recebe
  --- as mensagens conversa_comecou, escolheu e conversa_terminou). Devolve
  --- false se já houver uma conversa aberta.
  function scene:start_dialogue(path, quem_fala)
    return M.dialogue.start(self, path, quem_fala)
  end

  --- Fecha a conversa aberta.
  function scene:stop_dialogue()
    M.dialogue.stop(self, false)
  end

  --- O arquivo da conversa aberta (nil se não houver).
  function scene:dialogue()
    return self._dlg and self._dlg.path or nil
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
      if self._3d then
        -- 3D: o que o raio do mouse acerta primeiro (o dono que está no
        -- arquivo, se for parte de um prefab).
        local sx, sy = nn.input.mouse()
        local o = sx and M.pick3d(self, sx, sy)
        while o and o.id <= 0 do o = o.parent end
        if o then picked = o.id end
      else
        local x, y = nn.input.mouse_world()
        if x then picked = M.pick(self, x, y) end
      end
    end
    return state, picked
  end

  return scene
end

return M
