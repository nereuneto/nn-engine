//! Descrição curta de cada função da API `nn.*` (e dos campos dos objetos
//! mais usados), para o autocompletar e as dicas do NN Studio. Um teste
//! confere que toda função de `nn` está aqui.

/// Uma função (ou tabela) da API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApiDoc {
    /// Nome completo: `nn.draw.sprite`. Tabelas também entram (`nn.draw`).
    pub name: &'static str,
    /// Parâmetros, como aparecem na dica: `(img, x, y, opts)`. Vazio numa
    /// tabela ou num valor.
    pub args: &'static str,
    pub doc: &'static str,
}

impl ApiDoc {
    /// É uma função (tem parâmetros, mesmo que nenhum)?
    pub fn is_function(&self) -> bool {
        self.args.starts_with('(')
    }

    /// Última parte do nome (`sprite` em `nn.draw.sprite`).
    pub fn short(&self) -> &'static str {
        self.name.rsplit(['.', ':']).next().unwrap_or(self.name)
    }
}

const fn d(name: &'static str, args: &'static str, doc: &'static str) -> ApiDoc {
    ApiDoc { name, args, doc }
}

/// A API `nn.*`.
pub const API_DOCS: &[ApiDoc] = &[
    d(
        "nn",
        "",
        "A engine: desenho, entrada, sons, física, cenas...",
    ),
    d("nn.version", "", "Versão da engine (texto)."),
    d(
        "nn.vars",
        "",
        "Variáveis do jogo: começam com o `vars` do config.lua e valem entre as cenas.",
    ),
    d(
        "nn.reset_vars",
        "()",
        "Volta as variáveis do jogo (nn.vars) aos valores do config.lua.",
    ),
    d(
        "nn.quit",
        "(opts)",
        "Fecha o jogo. Com { fade = segundos }, escurece antes.",
    ),
    d(
        "nn.animation",
        "(quadros, fps, modo)",
        "Animação: lista de quadros, quadros por segundo e modo (\"loop\", \"once\", \"pingpong\").",
    ),
    d(
        "nn.animator",
        "(animacao)",
        "Toca animações: :play(anim), :update(dt), :frame().",
    ),
    d(
        "nn.frames",
        "(de, ate)",
        "Lista de quadros de `de` até `ate`: nn.frames(1, 4) = {1, 2, 3, 4}.",
    ),
    d(
        "nn.particles",
        "(opts)",
        "Emissor de partículas (fumaça, faíscas...): :update(dt), :draw(), :burst(n, x, y).",
    ),
    d(
        "nn.text_size",
        "(texto, opts)",
        "Largura e altura de um texto, sem desenhar.",
    ),
    // Desenho
    d(
        "nn.draw",
        "",
        "Desenho no mundo (visto pela câmera, Y para cima).",
    ),
    d(
        "nn.draw.sprite",
        "(img, x, y, opts)",
        "Desenha uma imagem ou quadro (opts: frame, w, h, scale, rotation, color, alpha, flip_x, flip_y).",
    ),
    d(
        "nn.draw.rect",
        "(x, y, w, h, cor)",
        "Retângulo de cor sólida (x, y é o centro).",
    ),
    d(
        "nn.draw.circle",
        "(x, y, raio, cor)",
        "Círculo de cor sólida.",
    ),
    d(
        "nn.draw.text",
        "(texto, x, y, opts)",
        "Texto no mundo (opts: size, color, alpha, anchor, align, font).",
    ),
    d(
        "nn.draw.parallax",
        "(img, opts)",
        "Fundo em camadas que anda mais devagar que o mundo (opts: factor, x, y, scale, repeat_x, scroll_x...).",
    ),
    d(
        "nn.screen",
        "",
        "Desenho na tela, por cima do mundo (HUD, menus). Origem no canto superior esquerdo.",
    ),
    d("nn.screen.sprite", "(img, x, y, opts)", "Imagem na tela."),
    d(
        "nn.screen.rect",
        "(x, y, w, h, cor)",
        "Retângulo na tela (x, y é o canto superior esquerdo).",
    ),
    d("nn.screen.circle", "(x, y, raio, cor)", "Círculo na tela."),
    d("nn.screen.text", "(texto, x, y, opts)", "Texto na tela."),
    d(
        "nn.screen.flash",
        "(cor, duracao)",
        "Pinta a tela inteira e vai sumindo (ex.: \"#FF000066\" ao levar dano).",
    ),
    // Entrada
    d("nn.input", "", "Teclado, mouse e controle."),
    d(
        "nn.input.down",
        "(tecla)",
        "A tecla está segurada? (\"a\", \"space\", \"left\"...)",
    ),
    d(
        "nn.input.pressed",
        "(tecla)",
        "Apertou a tecla neste quadro?",
    ),
    d(
        "nn.input.released",
        "(tecla)",
        "Soltou a tecla neste quadro?",
    ),
    d(
        "nn.input.wasd",
        "()",
        "Direção dx, dy com WASD (Y para cima).",
    ),
    d("nn.input.arrows", "()", "Direção dx, dy com as setas."),
    d(
        "nn.input.move",
        "()",
        "Direção dx, dy juntando WASD, setas, direcional e analógico.",
    ),
    d(
        "nn.input.axis",
        "(neg, pos)",
        "-1, 0 ou 1 a partir de duas teclas.",
    ),
    d(
        "nn.input.any_pressed",
        "()",
        "Apertou qualquer tecla, botão do mouse ou do controle?",
    ),
    d(
        "nn.input.last_device",
        "()",
        "Último usado: \"keyboard\", \"mouse\" ou \"gamepad\".",
    ),
    d(
        "nn.input.mouse",
        "()",
        "Posição do mouse na tela (nil fora da janela).",
    ),
    d("nn.input.mouse_world", "()", "Posição do mouse no mundo."),
    d(
        "nn.input.mouse_down",
        "(botao)",
        "Botão do mouse segurado (\"left\", \"right\", \"middle\").",
    ),
    d(
        "nn.input.mouse_pressed",
        "(botao)",
        "Apertou o botão do mouse neste quadro?",
    ),
    d(
        "nn.input.mouse_released",
        "(botao)",
        "Soltou o botão do mouse neste quadro?",
    ),
    d("nn.input.scroll", "()", "Rolagem do mouse neste quadro."),
    d(
        "nn.input.pad_down",
        "(botao, n)",
        "Botão do controle segurado (\"a\", \"b\", \"x\", \"y\", \"start\"...).",
    ),
    d(
        "nn.input.pad_pressed",
        "(botao, n)",
        "Apertou o botão do controle neste quadro?",
    ),
    d(
        "nn.input.pad_released",
        "(botao, n)",
        "Soltou o botão do controle neste quadro?",
    ),
    d(
        "nn.input.pad_axis",
        "(eixo, n)",
        "Valor de um eixo do controle (\"leftx\", \"lefty\", \"lt\"...).",
    ),
    d(
        "nn.input.stick",
        "(lado, n)",
        "Analógico \"left\" ou \"right\" como x, y.",
    ),
    d("nn.input.pads", "()", "Controles conectados."),
    d(
        "nn.input.pad_count",
        "()",
        "Quantos controles estão conectados.",
    ),
    d(
        "nn.input.set_deadzone",
        "(v)",
        "Zona morta dos analógicos (padrão 0.2).",
    ),
    d(
        "nn.input.bind",
        "(acao, entradas)",
        "Cria uma ação: nn.input.bind(\"pular\", { \"space\", \"pad:a\" }).",
    ),
    d("nn.input.action", "(acao)", "A ação está segurada?"),
    d(
        "nn.input.action_pressed",
        "(acao)",
        "A ação começou neste quadro?",
    ),
    d(
        "nn.input.action_released",
        "(acao)",
        "A ação terminou neste quadro?",
    ),
    d(
        "nn.input.action_value",
        "(acao)",
        "Intensidade da ação, de 0 a 1.",
    ),
    d(
        "nn.input.action_axis",
        "(neg, pos)",
        "Eixo de -1 a 1 a partir de duas ações.",
    ),
    // Assets
    d(
        "nn.assets",
        "",
        "Imagens, sons, fontes e arquivos do projeto.",
    ),
    d(
        "nn.assets.texture",
        "(caminho, opts)",
        "Carrega uma imagem PNG ({ pixel_art = true } para não suavizar).",
    ),
    d(
        "nn.assets.sheet",
        "(caminho, largura, altura, opts)",
        "Spritesheet com quadros do tamanho dado.",
    ),
    d(
        "nn.assets.sound",
        "(caminho)",
        "Carrega um som (WAV, OGG, MP3).",
    ),
    d(
        "nn.assets.font",
        "(caminho)",
        "Carrega uma fonte .ttf/.otf.",
    ),
    d(
        "nn.assets.read",
        "(caminho)",
        "Conteúdo de um arquivo de texto.",
    ),
    // Áudio
    d("nn.audio", "", "Sons e música."),
    d(
        "nn.audio.play",
        "(som, opts)",
        "Toca um som (opts: volume, speed, pan, loop, channel).",
    ),
    d(
        "nn.audio.set_volume",
        "(v, canal)",
        "Volume geral, ou de um canal (\"effects\", \"music\").",
    ),
    d("nn.audio.volume", "(canal)", "Volume atual."),
    // Câmera
    d("nn.camera", "", "A câmera do mundo."),
    d("nn.camera.position", "()", "Posição x, y da câmera."),
    d("nn.camera.set_position", "(x, y)", "Põe a câmera em x, y."),
    d("nn.camera.zoom", "()", "Zoom atual."),
    d(
        "nn.camera.set_zoom",
        "(z)",
        "Muda o zoom (2 = tudo duas vezes maior).",
    ),
    d("nn.camera.rotation", "()", "Rotação da câmera."),
    d("nn.camera.set_rotation", "(r)", "Gira a câmera (radianos)."),
    d(
        "nn.camera.follow",
        "(x, y, opts)",
        "Segue o alvo suavemente (opts: smooth, deadzone_w, deadzone_h, offset_x, offset_y, bounds).",
    ),
    d(
        "nn.camera.clamp",
        "(limite)",
        "Não deixa a câmera passar do limite (um mapa ou { x, y, w, h }).",
    ),
    d(
        "nn.camera.view",
        "()",
        "Área visível no mundo: x0, y0, x1, y1.",
    ),
    d("nn.camera.to_world", "(sx, sy)", "Ponto da tela no mundo."),
    d("nn.camera.to_screen", "(wx, wy)", "Ponto do mundo na tela."),
    d(
        "nn.camera.shake",
        "(forca, duracao)",
        "Faz a câmera tremer.",
    ),
    d("nn.camera.stop_shake", "()", "Para o tremor na hora."),
    // Cenas
    d("nn.scene", "", "Trocar de cena."),
    d(
        "nn.scene.switch",
        "(caminho, args, opts)",
        "Fecha todas as cenas e abre esta (opts: { fade = segundos }).",
    ),
    d(
        "nn.scene.push",
        "(caminho, args, opts)",
        "Abre a cena por cima da atual (pausa, janela).",
    ),
    d(
        "nn.scene.pop",
        "(opts)",
        "Fecha a cena atual e volta para a de baixo.",
    ),
    d(
        "nn.scene.replace",
        "(caminho, args, opts)",
        "Troca a cena atual.",
    ),
    // Física
    d("nn.physics", "", "Física arcade (corpos não giram)."),
    d(
        "nn.physics.world",
        "(opts)",
        "Cria um mundo físico ({ gravity = -1800 }): :body(opts), :step(dt), :raycast(...).",
    ),
    // Mapas
    d("nn.tilemap", "", "Mapas de tiles (Tiled)."),
    d(
        "nn.tilemap.load",
        "(caminho, opts)",
        "Carrega um mapa .tmj do Tiled (opts: scale, x, y).",
    ),
    d("nn.tilemap.new", "(opts)", "Mapa vazio feito pelo código."),
    // Tempo e janela
    d("nn.time", "", "Relógio do jogo."),
    d("nn.time.delta", "()", "Segundos desde o quadro anterior."),
    d("nn.time.elapsed", "()", "Segundos desde que o jogo abriu."),
    d("nn.time.fixed_delta", "()", "Duração de um passo fixo."),
    d(
        "nn.time.alpha",
        "()",
        "Fração entre passos fixos (para suavizar posições no draw).",
    ),
    d("nn.time.frame", "()", "Número do quadro."),
    d("nn.window", "", "A janela do jogo."),
    d("nn.window.size", "()", "Largura e altura da janela."),
    d("nn.window.set_clear_color", "(cor)", "Cor de fundo."),
    d("nn.window.fullscreen", "()", "Está em tela cheia?"),
    d(
        "nn.window.set_fullscreen",
        "(sim)",
        "Liga ou desliga a tela cheia.",
    ),
    d("nn.window.toggle_fullscreen", "()", "Alterna a tela cheia."),
    // Dados salvos
    d(
        "nn.save",
        "",
        "Dados que continuam depois de fechar o jogo.",
    ),
    d(
        "nn.save.get",
        "(chave, padrao)",
        "Valor salvo, ou o padrão.",
    ),
    d(
        "nn.save.set",
        "(chave, valor)",
        "Guarda um valor (números, textos, tabelas).",
    ),
    d("nn.save.remove", "(chave)", "Apaga uma chave."),
    d("nn.save.clear", "()", "Apaga tudo."),
    d("nn.save.keys", "()", "Lista das chaves salvas."),
    d("nn.save.flush", "()", "Grava no disco agora."),
    d("nn.save.path", "()", "Onde fica o arquivo."),
    // Interface
    d("nn.ui", "", "Menus com botões (mouse, teclado e controle)."),
    d(
        "nn.ui.panel",
        "(nome, opts)",
        "Abre um painel (dentro do draw): ui:button(texto), ui:slider(...), ui:finish().",
    ),
    d(
        "nn.ui.theme",
        "(opts)",
        "Muda a aparência de todos os painéis.",
    ),
    d(
        "nn.ui.set_focus",
        "(nome, n)",
        "Põe o foco no item n de um painel.",
    ),
    d("nn.ui.reset", "(nome)", "Esquece o foco de um painel."),
];

/// Campos e métodos de um tipo de valor (para completar depois do `.`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Member {
    pub name: &'static str,
    /// Parâmetros, se for um método (`:nome(...)`); vazio num campo.
    pub args: &'static str,
    pub doc: &'static str,
}

const fn m(name: &'static str, args: &'static str, doc: &'static str) -> Member {
    Member { name, args, doc }
}

/// Objeto de uma cena visual.
pub const OBJECT_MEMBERS: &[Member] = &[
    m("x", "", "Posição X (relativa ao pai)."),
    m("y", "", "Posição Y (relativa ao pai, para cima)."),
    m("rotation", "", "Rotação em graus."),
    m("scale_x", "", "Escala horizontal."),
    m("scale_y", "", "Escala vertical."),
    m("visible", "", "Aparece?"),
    m("layer", "", "Camada (maior = por cima)."),
    m("name", "", "Nome do objeto."),
    m("id", "", "Número do objeto na cena."),
    m("alive", "", "Ainda está na cena?"),
    m("parent", "", "O objeto pai (ou nil)."),
    m("children", "", "Lista dos filhos."),
    m("components", "", "Lista dos componentes."),
    m("behaviors", "", "Lista dos comportamentos."),
    m("body", "", "O corpo físico (ou nil)."),
    m("sprite", "", "O componente de imagem (ou nil)."),
    m("text", "", "O componente de texto (ou nil)."),
    m("shape", "", "O componente de forma (ou nil)."),
    m("camera", "", "O componente de câmera (ou nil)."),
    m("map", "", "O mapa carregado (ou nil)."),
    m("particles", "", "O emissor de partículas (ou nil)."),
    m("sound", "", "O som carregado (ou nil)."),
    m(
        "vx",
        "",
        "Velocidade X (objetos sem corpo físico andam sozinhos).",
    ),
    m("vy", "", "Velocidade Y."),
    m("start_x", "", "Onde o objeto começou (X)."),
    m("start_y", "", "Onde o objeto começou (Y)."),
    m(
        "component",
        "(tipo)",
        "Primeiro componente do tipo (\"sprite\", \"body\"...).",
    ),
    m(
        "behavior",
        "(nome)",
        "Comportamento pelo nome (\"plataforma\" ou o caminho do script).",
    ),
    m(
        "send",
        "(mensagem, ...)",
        "Chama a função `mensagem` dos comportamentos do objeto (ex.: obj:send(\"dano\", 1)).",
    ),
    m("destroy", "()", "Tira o objeto (e os filhos) da cena."),
    m(
        "world_transform",
        "()",
        "Posição, rotação e escala no mundo.",
    ),
    m("bounds", "()", "Metade da largura e da altura."),
    m(
        "touching",
        "(outro, margem)",
        "Está encostando no outro objeto?",
    ),
    m(
        "move_to",
        "(x, y)",
        "Leva o objeto (e o corpo físico) para x, y.",
    ),
    m("mouse_over", "()", "O mouse está em cima do objeto?"),
];

/// Corpo físico (`obj.body` ou `mundo:body`).
pub const BODY_MEMBERS: &[Member] = &[
    m("x", "", "Centro X."),
    m("y", "", "Centro Y."),
    m("vx", "", "Velocidade X."),
    m("vy", "", "Velocidade Y (positiva = sobe)."),
    m("gravity_scale", "", "Multiplica a gravidade (0 = flutua)."),
    m("bounce", "", "Quique (0 a 1)."),
    m("sensor", "", "Só avisa, não bloqueia."),
    m("layer", "", "Camada (bits)."),
    m("mask", "", "Com quem interage (bits)."),
    m("tag", "", "Rótulo livre."),
    m("id", "", "Número do corpo."),
    m("type", "", "\"dynamic\", \"static\" ou \"kinematic\"."),
    m("width", "", "Largura."),
    m("height", "", "Altura."),
    m("radius", "", "Raio (círculo)."),
    m("alive", "", "Ainda existe?"),
    m("on_ground", "", "Está apoiado em algo?"),
    m("ground", "", "O corpo em que está apoiado."),
    m("touching_left", "", "Encostando à esquerda?"),
    m("touching_right", "", "Encostando à direita?"),
    m("touching_top", "", "Encostando em cima?"),
    m("touching_bottom", "", "Encostando embaixo?"),
    m("set_position", "(x, y)", "Teleporta o corpo."),
    m("set_velocity", "(vx, vy)", "Muda a velocidade."),
    m("overlaps", "()", "Corpos encostando agora."),
    m("intersects", "(outro)", "Os dois se sobrepõem?"),
    m("remove", "()", "Tira o corpo do mundo."),
];

/// `self` dentro do script de uma cena visual.
pub const SCENE_MEMBERS: &[Member] = &[
    m("objects", "", "Objetos pelo nome: self.objects.heroi."),
    m("object_list", "", "Todos os objetos, em ordem."),
    m("world", "", "O mundo físico da cena (ou nil)."),
    m("find", "(nome)", "Objeto pelo nome."),
    m("find_all", "(nome)", "Todos os objetos com esse nome."),
    m(
        "spawn",
        "(prefab, x, y, opts)",
        "Cria um objeto a partir de um prefab.",
    ),
    m(
        "targets",
        "(quem)",
        "Objetos com esse nome (\"\" = o jogador).",
    ),
    m("burst", "(x, y, cor, n)", "Faíscas num ponto."),
    m(
        "restart",
        "(opts)",
        "Reinicia a cena ({ checkpoint = true } volta ao checkpoint).",
    ),
    m("set_checkpoint", "(x, y)", "Guarda o ponto de reinício."),
    m("checkpoint", "()", "Último ponto de reinício (x, y)."),
];

/// `self` dentro de um comportamento.
pub const BEHAVIOR_MEMBERS: &[Member] = &[
    m("obj", "", "O objeto que tem este comportamento."),
    m("scene", "", "A cena (scene:find, scene:spawn...)."),
    m("enabled", "", "false = o comportamento para de rodar."),
    m(
        "component",
        "",
        "O componente (com os valores do Inspetor).",
    ),
];

/// Componentes de imagem, texto e forma (`obj.sprite.` ...).
pub const SPRITE_MEMBERS: &[Member] = &[
    m("image", "", "Arquivo da imagem."),
    m("frame", "", "Quadro da spritesheet."),
    m("w", "", "Largura (0 = a da imagem)."),
    m("h", "", "Altura."),
    m("color", "", "Tinge a imagem."),
    m("flip_x", "", "Espelhada na horizontal?"),
    m("flip_y", "", "Espelhada na vertical?"),
    m("hud", "", "Fixa na tela?"),
];

pub const TEXT_MEMBERS: &[Member] = &[
    m("text", "", "O texto."),
    m("size", "", "Tamanho."),
    m("color", "", "Cor."),
    m("anchor", "", "Qual ponto fica na posição."),
    m("hud", "", "Fixo na tela?"),
];

pub const SHAPE_MEMBERS: &[Member] = &[
    m("shape", "", "\"rect\" ou \"circle\"."),
    m("w", "", "Largura."),
    m("h", "", "Altura."),
    m("radius", "", "Raio."),
    m("color", "", "Cor."),
    m("hud", "", "Fixa na tela?"),
];

/// Documentação de um nome completo da API.
pub fn api_doc(name: &str) -> Option<&'static ApiDoc> {
    API_DOCS.iter().find(|d| d.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toda_funcao_de_nn_tem_descricao() {
        let rt = crate::ScriptRuntime::new(std::env::temp_dir()).unwrap();
        let names: Vec<String> = rt
            .lua()
            .load(
                r#"
local out = {}
local function walk(t, prefix, depth)
  if depth > 3 then return end
  for k, v in pairs(t) do
    if type(k) == "string" and k:sub(1, 1) ~= "_" then
      local name = prefix .. k
      table.insert(out, name)
      if type(v) == "table" then walk(v, name .. ".", depth + 1) end
    end
  end
end
walk(nn, "nn.", 0)
return out
"#,
            )
            .eval()
            .unwrap();
        for n in &names {
            assert!(api_doc(n).is_some(), "{n} sem descrição em API_DOCS");
        }
        for d in API_DOCS {
            if d.name != "nn" {
                assert!(
                    names.contains(&d.name.to_string()),
                    "{} não existe mais",
                    d.name
                );
            }
            assert!(!d.doc.is_empty(), "{}", d.name);
        }
    }
}
