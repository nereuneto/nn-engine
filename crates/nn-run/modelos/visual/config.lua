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
  -- Tamanho do jogo: a fase sempre aparece inteira, com a janela maior ou
  -- menor (sobra uma faixa preta quando a proporção é outra).
  resolution = { 1280, 720 },
  clear_color = "#14202E",
  -- A fase é uma cena visual: abra-a no NN Studio para mexer com o mouse.
  start = "cenas/fase.cena",
  -- Variáveis do jogo (nn.vars): valem de uma cena para outra. Os
  -- comportamentos Coletável e Vida mexem nelas; Mostrar valor mostra.
  vars = { moedas = 0, vidas = 3 },
}
