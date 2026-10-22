//! Recursos (a documentação), prompts e as instruções para o agente.

use serde_json::{Value, json};

use crate::Server;

/// Os guias da engine, dentro do executável.
pub const DOCS: [(&str, &str); 4] = [
    ("lua.md", include_str!("../../../docs/lua.md")),
    ("3d.md", include_str!("../../../docs/3d.md")),
    ("studio.md", include_str!("../../../docs/studio.md")),
    ("mcp.md", include_str!("../../../docs/mcp.md")),
];

const DOC_TITLES: [(&str, &str, &str); 4] = [
    (
        "lua.md",
        "Guia de Lua e das cenas",
        "Projeto, config.lua, cenas visuais, comportamentos, prefabs, eventos e a referência da API nn.*",
    ),
    (
        "3d.md",
        "Guia do 3D",
        "Cenas 3D, modelos, luzes, física 3D, Jogador 3D",
    ),
    (
        "studio.md",
        "Guia do NN Studio",
        "O editor visual: o que cada painel faz e os formatos dos arquivos",
    ),
    (
        "mcp.md",
        "O servidor MCP",
        "As ferramentas deste servidor e o jeito de trabalhar com elas",
    ),
];

pub fn resources(server: &Server) -> Vec<Value> {
    let mut out: Vec<Value> = DOC_TITLES
        .iter()
        .map(|(name, title, desc)| {
            json!({
                "uri": format!("nn://docs/{name}"),
                "name": name,
                "title": title,
                "description": desc,
                "mimeType": "text/markdown",
            })
        })
        .collect();
    out.push(json!({
        "uri": "nn://projeto/AGENTS.md",
        "name": "AGENTS.md",
        "title": "Como trabalhar neste projeto",
        "description": if server.root.join("AGENTS.md").is_file() {
            "O AGENTS.md do projeto"
        } else {
            "O AGENTS.md que a engine põe nos projetos novos"
        },
        "mimeType": "text/markdown",
    }));
    out
}

pub fn read(server: &Server, uri: &str) -> Option<String> {
    if uri == "nn://projeto/AGENTS.md" {
        return std::fs::read_to_string(server.root.join("AGENTS.md"))
            .ok()
            .or_else(|| {
                (!server.hooks.agents_md.is_empty()).then(|| server.hooks.agents_md.clone())
            });
    }
    let name = uri.strip_prefix("nn://docs/")?;
    DOCS.iter()
        .find(|(n, _)| *n == name)
        .map(|(_, t)| t.to_string())
}

pub fn prompts() -> Vec<Value> {
    vec![
        json!({
            "name": "criar_jogo",
            "title": "Criar um jogo",
            "description": "Faz um jogo do começo a partir de uma ideia",
            "arguments": [
                { "name": "ideia", "description": "O jogo que você quer (gênero, tema, o que o jogador faz)", "required": true },
            ],
        }),
        json!({
            "name": "melhorar_jogo",
            "title": "Melhorar o jogo",
            "description": "Olha o projeto aberto e faz uma melhoria pedida",
            "arguments": [
                { "name": "pedido", "description": "O que mudar ou acrescentar", "required": true },
            ],
        }),
    ]
}

pub fn prompt(params: &Value) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or("falta \"name\"")?;
    let arg = |k: &str| {
        params
            .pointer(&format!("/arguments/{k}"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let text = match name {
        "criar_jogo" => format!(
            "Quero fazer este jogo com a nn-engine: {}\n\n\
             1. Use ver_projeto. Se a pasta não tem jogo, escolha o modelo mais próximo (os \"visual\" são montados sem código, com cenas, comportamentos prontos e eventos) e use criar_projeto.\n\
             2. Leia o AGENTS.md e veja as cenas que vieram (ver_cena).\n\
             3. Planeje em poucos passos e me mostre o plano.\n\
             4. Monte: prefira comportamentos prontos (listar_comportamentos) e eventos (listar_eventos); escreva Lua só para o que eles não fazem (buscar_documentacao antes).\n\
             5. Jogue o que fez com rodar_jogo (ações de teclado e mouse, capturas da tela) e conserte o que não funcionar.\n\
             6. Escreva testes de jogo em testes/ para o principal (rodar_testes) e termine com conferir_projeto.",
            arg("ideia")
        ),
        "melhorar_jogo" => format!(
            "No jogo desta pasta, quero: {}\n\n\
             Comece por ver_projeto e pelas cenas envolvidas (ver_cena). Mude com as ferramentas de cena e eventos, \
             escreva Lua só quando precisar. Confira jogando (rodar_jogo), rode os testes do projeto (rodar_testes) \
             e acrescente um teste para o que mudou. Se o jogo estiver aberto numa janela, veja o efeito nele \
             (ver_jogo_aberto). Termine com conferir_projeto.",
            arg("pedido")
        ),
        other => return Err(format!("prompt desconhecido: {other}")),
    };
    Ok(json!({
        "description": name,
        "messages": [{ "role": "user", "content": { "type": "text", "text": text } }],
    }))
}

/// As instruções que o servidor manda ao agente ao conectar.
pub fn instructions(server: &Server) -> String {
    let mut s = String::from(
        "Servidor da nn-engine: um projeto de jogo 2D/3D em Lua, com cenas visuais (.cena) que o NN Studio também edita. \
         Tudo (nomes de ferramentas, documentação, mensagens) é em português.\n\
         Jeito de trabalhar:\n\
         - Comece com ver_projeto (e leia o AGENTS.md do projeto, se houver).\n\
         - Cenas e prefabs: ver_cena, criar_objeto, mudar_objeto, copiar_objeto, apagar_objeto, criar_prefab. Os objetos vão no mesmo formato do arquivo, e cada campo é conferido.\n\
         - Comportamento sem código: comportamentos prontos (listar_comportamentos) e eventos \"Quando... Então...\" (listar_eventos, por_regra).\n\
         - Código: buscar_documentacao antes; escrever_arquivo confere a sintaxe antes de gravar.\n\
         - Depois de uma leva de mudanças, conferir_projeto. Errou? desfazer.\n\
         - Veja funcionando: rodar_jogo roda sem janela, com teclas e cliques simulados, e mostra erros, prints, regras que dispararam, o que mudou e capturas da tela.\n\
         - Testes de jogo: arquivos Lua em testes/ com test(\"nome\", function(t) ... end); rodar_testes roda todos. Rode antes e depois de mudar.\n\
         - Jogo aberto numa janela (abrir_jogo, ou o ▶ Jogar do NN Studio): ver_jogo_aberto mostra o que acontece enquanto a pessoa joga; mexer_no_jogo muda valores na hora (sem gravar), pausa e avança quadros. Bom para ajustar números junto com a pessoa; o que ficar, grave nos arquivos.\n\
         - 2D: pixels, y para cima, (0,0) no meio da tela. 3D: metros, y é a altura.",
    );
    if !server.has_project() {
        s.push_str("\nA pasta ainda não tem jogo: use criar_projeto.");
    }
    s
}
