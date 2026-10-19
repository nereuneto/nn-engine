-- Configuração do jogo. Todas as opções estão na documentação da nn-engine
-- (docs/lua.md, seção "config.lua").
return {
  title = "{{TITULO}}",
  -- Nome da pasta onde o recorde e as opções são salvos. Não mude depois de
  -- distribuir o jogo, ou os jogadores perdem os recordes.
  id = "{{ID}}",
  width = 1280,
  height = 720,
  clear_color = "#BFE3F2",
  start = "cenas/titulo.lua",
}
