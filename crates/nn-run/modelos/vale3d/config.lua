-- Configuração do jogo. Todas as opções estão na documentação da nn-engine
-- (docs/lua.md, seção "config.lua").
return {
  title = "{{TITULO}}",
  -- Nome da pasta onde o progresso é salvo. Não mude depois de distribuir
  -- o jogo.
  id = "{{ID}}",
  width = 1280,
  height = 720,
  resolution = { 1280, 720 },
  clear_color = "#BFE3F2",
  -- O 3D com as bordas lisas (antisserrilhado) e sem a luz forte estourar
  -- em branco (tonemap). Em Configurações do projeto.
  antialias = true,
  tonemap = true,
  start = "cenas/vale.cena",
  -- Variáveis do jogo (nn.vars): crie em Configurações do projeto.
  vars = { estrelas = 0, moedas = 0, vidas = 3 },
}
