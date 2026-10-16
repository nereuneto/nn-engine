-- Some depois de um tempo ou ao sair da tela.
local B = {
  title = "Destruir depois",
  description = "Some depois de um tempo ou ao sair da tela (tiros, efeitos).",
  category = "Ações",
  props = {
    { "tempo", 3, label = "Some depois de (segundos)", min = 0, help = "0 = não some pelo tempo." },
    { "fora_da_tela", true, label = "Some ao sair da tela" },
  },
}

function B:start()
  self.t = 0
end

function B:update(dt)
  self.t = self.t + dt
  if self.tempo > 0 and self.t >= self.tempo then
    self.obj:destroy()
    return
  end
  if self.fora_da_tela then
    local x0, y0, x1, y1 = nn.camera.view()
    local x, y = self.obj:world_transform()
    local m = 200
    if x < x0 - m or x > x1 + m or y < y0 - m or y > y1 + m then self.obj:destroy() end
  end
end

return B
