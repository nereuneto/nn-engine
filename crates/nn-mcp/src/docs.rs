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
             5. Termine com conferir_projeto e conserte o que aparecer.",
            arg("ideia")
        ),
        "melhorar_jogo" => format!(
            "No jogo desta pasta, quero: {}\n\n\
             Comece por ver_projeto e pelas cenas envolvidas (ver_cena). Mude com as ferramentas de cena e eventos, \
             escreva Lua só quando precisar, e termine com conferir_projeto.",
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
         - 2D: pixels, y para cima, (0,0) no meio da tela. 3D: metros, y é a altura.",
    );
    if !server.has_project() {
        s.push_str("\nA pasta ainda não tem jogo: use criar_projeto.");
    }
    s
}
