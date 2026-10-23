//! Diálogos (`.dialogo`): as conversas que o editor de conversa do NN
//! Studio monta e o comportamento Diálogo (`dialogos.lua`) mostra no jogo.
//!
//! Cada fala tem quem fala, o texto e para onde a conversa vai depois (a
//! próxima fala, as escolhas do jogador ou o fim). O arquivo é uma tabela
//! Lua.

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Character {
    pub name: String,
    /// "#RRGGBB" (a etiqueta do nome).
    pub color: String,
    /// Retrato (imagem) que aparece na caixa de fala.
    pub image: String,
    /// Som que toca enquanto o texto aparece (a "voz").
    pub sound: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Choice {
    pub text: String,
    /// A fala para onde vai (0 = termina).
    pub next: u32,
    /// Ao escolher: muda esta variável do jogo...
    pub var: String,
    /// ... para este valor (número ou texto).
    pub value: String,
    /// Só aparece para quem tem este item no inventário.
    pub need: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Node {
    pub id: u32,
    /// Onde o cartão fica no editor.
    pub x: f32,
    pub y: f32,
    /// Quem fala (o nome de um personagem; vazio = narrador).
    pub who: String,
    pub text: String,
    /// A próxima fala, sem escolhas (0 = termina).
    pub next: u32,
    pub choices: Vec<Choice>,
    /// Ao aparecer: muda esta variável...
    pub var: String,
    pub value: String,
    /// ... e dá este item.
    pub item: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DialogueData {
    pub start: u32,
    /// Letras por segundo (0 = o texto aparece de uma vez).
    pub speed: f64,
    pub characters: Vec<Character>,
    pub nodes: Vec<Node>,
}

impl Default for DialogueData {
    fn default() -> Self {
        Self {
            start: 1,
            speed: 40.0,
            characters: Vec::new(),
            nodes: vec![Node {
                id: 1,
                x: 40.0,
                y: 40.0,
                text: "Olá!".into(),
                ..Node::default()
            }],
        }
    }
}

impl DialogueData {
    pub fn node(&self, id: u32) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn node_mut(&mut self, id: u32) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn next_id(&self) -> u32 {
        self.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1
    }

    /// Cria uma fala em (x, y) e devolve o id.
    pub fn add_node(&mut self, x: f32, y: f32, who: &str) -> u32 {
        let id = self.next_id();
        self.nodes.push(Node {
            id,
            x,
            y,
            who: who.to_string(),
            ..Node::default()
        });
        if self.nodes.len() == 1 {
            self.start = id;
        }
        id
    }

    /// Apaga a fala; o que ia para ela passa a terminar.
    pub fn remove_node(&mut self, id: u32) {
        self.nodes.retain(|n| n.id != id);
        for n in &mut self.nodes {
            if n.next == id {
                n.next = 0;
            }
            for c in &mut n.choices {
                if c.next == id {
                    c.next = 0;
                }
            }
        }
        if self.start == id {
            self.start = self.nodes.first().map(|n| n.id).unwrap_or(0);
        }
    }

    pub fn character(&self, name: &str) -> Option<&Character> {
        self.characters.iter().find(|c| c.name == name)
    }
}

/// O conteúdo de um diálogo novo (Arquivos > ➕ Novo > Diálogo).
pub fn template() -> String {
    let d = DialogueData {
        start: 1,
        speed: 40.0,
        characters: vec![Character {
            name: "Ana".into(),
            color: "#FFD166".into(),
            ..Character::default()
        }],
        nodes: vec![
            Node {
                id: 1,
                x: 30.0,
                y: 30.0,
                who: "Ana".into(),
                text: "Olá! Que bom te ver por aqui.".into(),
                next: 2,
                ..Node::default()
            },
            Node {
                id: 2,
                x: 300.0,
                y: 30.0,
                who: "Ana".into(),
                text: "Posso te ajudar?".into(),
                choices: vec![
                    Choice {
                        text: "Sim, por favor!".into(),
                        next: 3,
                        ..Choice::default()
                    },
                    Choice {
                        text: "Agora não.".into(),
                        next: 0,
                        ..Choice::default()
                    },
                ],
                ..Node::default()
            },
            Node {
                id: 3,
                x: 300.0,
                y: 200.0,
                who: "Ana".into(),
                text: "Procure a chave perto da fogueira.".into(),
                ..Node::default()
            },
        ],
    };
    write(&d)
}

// --------------------------------------------------------------------------
// Ler e gravar

fn get_str(t: &mlua::Table, k: &str) -> String {
    match t.get::<mlua::Value>(k) {
        Ok(mlua::Value::String(s)) => s.to_string_lossy(),
        Ok(mlua::Value::Integer(i)) => i.to_string(),
        Ok(mlua::Value::Number(n)) => crate::scene_data::format_num(n),
        Ok(mlua::Value::Boolean(b)) => b.to_string(),
        _ => String::new(),
    }
}

fn get_num(t: &mlua::Table, k: &str, default: f64) -> f64 {
    match t.get::<mlua::Value>(k) {
        Ok(mlua::Value::Integer(i)) => i as f64,
        Ok(mlua::Value::Number(n)) => n,
        _ => default,
    }
}

/// Lê o texto de um `.dialogo`.
pub fn read(text: &str, rel: &str) -> Result<DialogueData, String> {
    let lua = crate::quiet_lua();
    let t: mlua::Table = lua
        .load(text)
        .set_name(format!("@{rel}"))
        .eval()
        .map_err(|e| nn_script::translate_error(e.to_string().trim()))?;
    let mut d = DialogueData {
        start: get_num(&t, "start", 1.0) as u32,
        speed: get_num(&t, "speed", 40.0),
        characters: Vec::new(),
        nodes: Vec::new(),
    };
    if let Ok(list) = t.get::<mlua::Table>("characters") {
        for c in list.sequence_values::<mlua::Table>().filter_map(Result::ok) {
            d.characters.push(Character {
                name: get_str(&c, "name"),
                color: get_str(&c, "color"),
                image: get_str(&c, "image"),
                sound: get_str(&c, "sound"),
            });
        }
    }
    if let Ok(list) = t.get::<mlua::Table>("nodes") {
        for (i, n) in list
            .sequence_values::<mlua::Table>()
            .filter_map(Result::ok)
            .enumerate()
        {
            let id = get_num(&n, "id", 0.0) as u32;
            if id == 0 {
                return Err(format!("a fala {} não tem id", i + 1));
            }
            let mut node = Node {
                id,
                x: get_num(&n, "x", 40.0 + i as f64 * 300.0) as f32,
                y: get_num(&n, "y", 40.0) as f32,
                who: get_str(&n, "who"),
                text: get_str(&n, "text"),
                next: get_num(&n, "next", 0.0) as u32,
                choices: Vec::new(),
                var: get_str(&n, "var"),
                value: get_str(&n, "value"),
                item: get_str(&n, "item"),
            };
            if let Ok(cs) = n.get::<mlua::Table>("choices") {
                for c in cs.sequence_values::<mlua::Table>().filter_map(Result::ok) {
                    node.choices.push(Choice {
                        text: get_str(&c, "text"),
                        next: get_num(&c, "next", 0.0) as u32,
                        var: get_str(&c, "var"),
                        value: get_str(&c, "value"),
                        need: get_str(&c, "need"),
                    });
                }
            }
            d.nodes.push(node);
        }
    }
    Ok(d)
}

fn lua_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Um valor de variável: número se der, senão texto.
fn lua_value(s: &str) -> String {
    let t = s.trim();
    match t.parse::<f64>() {
        Ok(n) if !t.is_empty() && n.is_finite() => crate::scene_data::format_num(n),
        _ => lua_str(s),
    }
}

/// O texto de um `.dialogo` (o mesmo formato que a engine lê).
pub fn write(d: &DialogueData) -> String {
    let mut s = String::from(
        "-- Diálogo do NN Studio (editor de conversa). Dá para editar à mão;\n\
         -- ao salvar pelo editor, comentários escritos aqui se perdem.\n\
         return {\n  format = 1,\n",
    );
    s.push_str(&format!(
        "  start = {},\n  speed = {},\n",
        d.start,
        crate::scene_data::format_num(d.speed)
    ));
    s.push_str("  characters = {\n");
    for c in &d.characters {
        s.push_str(&format!(
            "    {{ name = {}, color = {}, image = {}, sound = {} }},\n",
            lua_str(&c.name),
            lua_str(&c.color),
            lua_str(&c.image),
            lua_str(&c.sound)
        ));
    }
    s.push_str("  },\n  nodes = {\n");
    for n in &d.nodes {
        s.push_str(&format!(
            "    {{ id = {}, x = {}, y = {}, who = {},\n      text = {},\n",
            n.id,
            n.x.round(),
            n.y.round(),
            lua_str(&n.who),
            lua_str(&n.text)
        ));
        if n.choices.is_empty() {
            s.push_str(&format!("      next = {},\n", n.next));
        } else {
            s.push_str("      choices = {\n");
            for c in &n.choices {
                let mut extra = String::new();
                if !c.var.is_empty() {
                    extra.push_str(&format!(
                        ", var = {}, value = {}",
                        lua_str(&c.var),
                        lua_value(&c.value)
                    ));
                }
                if !c.need.is_empty() {
                    extra.push_str(&format!(", need = {}", lua_str(&c.need)));
                }
                s.push_str(&format!(
                    "        {{ text = {}, next = {}{extra} }},\n",
                    lua_str(&c.text),
                    c.next
                ));
            }
            s.push_str("      },\n");
        }
        if !n.var.is_empty() {
            s.push_str(&format!(
                "      var = {}, value = {},\n",
                lua_str(&n.var),
                lua_value(&n.value)
            ));
        }
        if !n.item.is_empty() {
            s.push_str(&format!("      item = {},\n", lua_str(&n.item)));
        }
        s.push_str("    },\n");
    }
    s.push_str("  },\n}\n");
    s
}
