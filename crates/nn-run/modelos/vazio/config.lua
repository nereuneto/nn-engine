-- Configuração do jogo. Todas as opções estão na documentação da nn-engine
-- (docs/lua.md, seção "config.lua").
return {
  title = "{{TITULO}}",
  -- Nome da pasta onde o progresso é salvo. Não mude depois de distribuir
  -- o jogo, ou os jogadores perdem os recordes.
  id = "{{ID}}",
  -- Tamanho da janela ao abrir.
  width = 1280,
  height = 720,
  -- Tamanho do jogo: aparece inteiro com a janela maior ou menor.
  resolution = { 1280, 720 },
  clear_color = "#5FB3E0",
  start = "cenas/fase.cena",
  -- Variáveis do jogo (nn.vars): crie em Configurações do projeto.
  vars = { moedas = 0, vidas = 3 },
}
