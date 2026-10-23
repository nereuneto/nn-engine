-- Mostra o inventário (chaves, poções...) num canto da tela.
local B = {
  title = "Mostrar inventário",
  description = "Mostra os itens que o jogador carrega (chaves, poções...) num canto da tela, com quantos tem de cada.",
  category = "Itens",
  mode = "both",
  props = {
    { "canto", "baixo_esquerda", label = "Canto",
      options = { { "cima_esquerda", "em cima, à esquerda" }, { "cima_direita", "em cima, à direita" },
        { "baixo_esquerda", "embaixo, à esquerda" }, { "baixo_direita", "embaixo, à direita" } } },
    { "tamanho", 48, label = "Tamanho", min = 6, help = "Pixels de cada quadrinho (numa tela pequena de pixel art, uns 12)." },
    { "margem", 20, label = "Distância da borda", min = 0, advanced = true },
    { "fundo", true, label = "Quadrinho atrás de cada item" },
  },
}

local imagens = {}
local function imagem(caminho)
  if caminho == nil or caminho == "" then return nil end
  local img = imagens[caminho]
  if img == nil then
    local ok, t = pcall(nn.assets.texture, caminho)
    img = ok and t or false
    imagens[caminho] = img
  end
  return img or nil
end

function B:draw()
  local itens = nn.inventory.items()
  if #itens == 0 then return end
  local W, H = nn.window.size()
  local t, gap, m = self.tamanho, math.floor(self.tamanho / 6), self.margem
  local total = #itens * t + (#itens - 1) * gap
  local direita = self.canto == "cima_direita" or self.canto == "baixo_direita"
  local embaixo = self.canto == "baixo_esquerda" or self.canto == "baixo_direita"
  local x = direita and (W - m - total) or m
  local y = embaixo and (H - m - t) or m
  local tema = rawget(nn, "theme") or {}
  local painel = (tema.panel ~= nil and tema.panel ~= "") and tema.panel or "#1E2A3AE6"
  local destaque = (tema.accent ~= nil and tema.accent ~= "") and tema.accent or "#FFD166"
  for _, it in ipairs(itens) do
    if self.fundo then
      nn.screen.panel(x, y, t, t, { color = painel, radius = math.floor(t / 5), border = 1, border_color = "#FFFFFF40" })
    end
    local img = imagem(it.icon)
    local pad = math.floor(t * 0.14)
    if img then
      nn.screen.sprite(img, x + pad, y + pad, { w = t - pad * 2, h = t - pad * 2 })
    else
      -- Sem ícone (um item 3D, ou sem imagem): o nome, se couber num
      -- tamanho legível; senão, a primeira letra.
      local nome, size = it.item, t * 0.3
      local w = nn.text_size(nome, { size = size })
      if w > t - pad * 2 then size = size * (t - pad * 2) / w end
      if size >= 10 then
        nn.screen.text(nome, x + t / 2, y + t / 2, { size = size, color = destaque, anchor = "center" })
      else
        nn.screen.text(it.item:sub(1, 1):upper(), x + t / 2, y + t / 2,
          { size = t * 0.5, color = destaque, anchor = "center" })
      end
    end
    if it.count > 1 then
      nn.screen.text("x" .. it.count, x + t - 3, y + t - 2,
        { size = math.max(12, t * 0.32), color = "#FFFFFF", anchor = "bottomright" })
    end
    x = x + t + gap
  end
end

return B
