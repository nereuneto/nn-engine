-- Configuração do jogo. Todas as opções estão na documentação da nn-engine
-- (docs/lua.md, seção "config.lua").
return {
  title = "{{TITULO}}",
  -- Nome da pasta onde o progresso é salvo. Não mude depois de distribuir
  -- o jogo, ou os jogadores perdem os recordes.
  id = "{{ID}}",
  width = 1280,
  height = 720,
  clear_color = "#14202E",
  start = "cenas/titulo.lua",
}
