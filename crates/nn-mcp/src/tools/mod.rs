//! As ferramentas que o agente chama.

use serde_json::{Map, Value, json};

use crate::Server;

mod aovivo;
mod arquivos;
mod cenas;
mod dialogos;
mod docs;
mod editor;
mod efeitos;
mod eventos;
mod jogar;
mod mapas;
mod modelos;
mod pixel;
mod projeto;
mod sons;
mod valores;

/// O que uma ferramenta devolve: blocos de texto e imagens.
#[derive(Debug, Default)]
pub struct Output {
    pub content: Vec<Value>,
}

impl Output {
    pub fn text(text: impl Into<String>) -> Self {
        Self::default().and_text(text)
    }

    pub fn and_text(mut self, text: impl Into<String>) -> Self {
        self.content
            .push(json!({ "type": "text", "text": text.into() }));
        self
    }

    /// Uma imagem que já veio em base64.
    pub fn and_image_base64(mut self, data: &str, mime: &str) -> Self {
        self.content.push(json!({
            "type": "image",
            "data": data,
            "mimeType": mime,
        }));
        self
    }

    pub fn and_image(mut self, png: &[u8], mime: &str) -> Self {
        self.content.push(json!({
            "type": "image",
            "data": crate::paths::base64(png),
            "mimeType": mime,
        }));
        self
    }
}

pub type Run = fn(&mut Server, &Value) -> Result<Output, String>;

/// Uma ferramenta: nome, descrição, parâmetros (JSON Schema) e o que faz.
pub struct Tool {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    /// Os parâmetros: (nome, esquema, obrigatório).
    pub params: fn() -> Vec<(&'static str, Value, bool)>,
    /// Só lê (não muda arquivos).
    pub read_only: bool,
    /// Pode apagar ou trocar o que existe.
    pub destructive: bool,
    pub run: Run,
}

/// Todas as ferramentas, na ordem em que aparecem para o agente.
fn all() -> Vec<&'static Tool> {
    let mut v: Vec<&'static Tool> = Vec::new();
    v.extend(projeto::TOOLS);
    v.extend(arquivos::TOOLS);
    v.extend(cenas::TOOLS);
    v.extend(eventos::TOOLS);
    v.extend(pixel::TOOLS);
    v.extend(mapas::TOOLS);
    v.extend(efeitos::TOOLS);
    v.extend(sons::TOOLS);
    v.extend(dialogos::TOOLS);
    v.extend(modelos::TOOLS);
    v.extend(jogar::TOOLS);
    v.extend(aovivo::TOOLS);
    v.extend(editor::TOOLS);
    v.extend(docs::TOOLS);
    v
}

pub fn find(name: &str) -> Option<&'static Tool> {
    all().into_iter().find(|t| t.name == name)
}

/// A lista para `tools/list`.
pub fn list() -> Vec<Value> {
    all()
        .into_iter()
        .map(|t| {
            let mut props = Map::new();
            let mut required = Vec::new();
            for (name, schema, req) in (t.params)() {
                props.insert(name.to_string(), schema);
                if req {
                    required.push(name);
                }
            }
            json!({
                "name": t.name,
                "title": t.title,
                "description": t.description,
                "inputSchema": {
                    "type": "object",
                    "properties": props,
                    "required": required,
                    "additionalProperties": false,
                },
                "annotations": {
                    "title": t.title,
                    "readOnlyHint": t.read_only,
                    "destructiveHint": t.destructive,
                    "idempotentHint": t.read_only,
                    "openWorldHint": false,
                },
            })
        })
        .collect()
}

// Esquemas e leitura dos argumentos.

pub fn s_str(desc: &str) -> Value {
    json!({ "type": "string", "description": desc })
}

pub fn s_num(desc: &str) -> Value {
    json!({ "type": "number", "description": desc })
}

pub fn s_int(desc: &str) -> Value {
    json!({ "type": "integer", "description": desc })
}

pub fn s_bool(desc: &str) -> Value {
    json!({ "type": "boolean", "description": desc })
}

pub fn s_enum(desc: &str, values: &[&str]) -> Value {
    json!({ "type": "string", "enum": values, "description": desc })
}

pub fn s_obj(desc: &str) -> Value {
    json!({ "type": "object", "description": desc })
}

pub fn s_list(desc: &str, items: Value) -> Value {
    json!({ "type": "array", "description": desc, "items": items })
}

/// Objeto da cena: o id (número) ou o nome.
pub fn s_ref(desc: &str) -> Value {
    json!({ "type": ["integer", "string"], "description": desc })
}

pub fn arg_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    match args.get(key) {
        Some(Value::String(s)) => Ok(s),
        Some(_) => Err(format!("\"{key}\" precisa ser um texto")),
        None => Err(format!("falta o parâmetro \"{key}\"")),
    }
}

pub fn opt_str<'a>(args: &'a Value, key: &str) -> Result<Option<&'a str>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        Some(_) => Err(format!("\"{key}\" precisa ser um texto")),
    }
}

pub fn opt_num(args: &Value, key: &str) -> Result<Option<f64>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_f64()
            .map(Some)
            .ok_or_else(|| format!("\"{key}\" precisa ser um número")),
    }
}

pub fn opt_bool(args: &Value, key: &str) -> Result<Option<bool>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(format!("\"{key}\" precisa ser true ou false")),
    }
}

impl Server {
    /// Grava um arquivo do projeto e anota para o `desfazer`.
    pub(crate) fn write_file(&mut self, rel: &str, data: &[u8]) -> Result<(), String> {
        let (rel, full) = crate::paths::resolve(&self.root, rel)?;
        let before = std::fs::read(&full).ok();
        if before.as_deref() == Some(data) {
            return Ok(());
        }
        self.check_unsaved(&rel)?;
        crate::paths::write_atomic(&full, data)
            .map_err(|e| format!("não foi possível gravar {rel}: {e}"))?;
        self.history.record(&rel, before, Some(data.to_vec()));
        Ok(())
    }

    /// Apaga um arquivo do projeto e anota para o `desfazer`.
    pub(crate) fn delete_file(&mut self, rel: &str) -> Result<(), String> {
        let (rel, full) = crate::paths::resolve(&self.root, rel)?;
        if full.is_dir() {
            return Err(format!(
                "{rel} é uma pasta; apague os arquivos dela um a um"
            ));
        }
        self.check_unsaved(&rel)?;
        let before =
            std::fs::read(&full).map_err(|e| format!("não foi possível ler {rel}: {e}"))?;
        std::fs::remove_file(&full).map_err(|e| format!("não foi possível apagar {rel}: {e}"))?;
        self.history.record(&rel, Some(before), None);
        Ok(())
    }

    /// Lê um arquivo de texto do projeto.
    pub(crate) fn read_text(&self, rel: &str) -> Result<(String, String), String> {
        let (rel, full) = crate::paths::resolve(&self.root, rel)?;
        if !full.exists() {
            return Err(format!("{rel} não existe"));
        }
        let text = std::fs::read_to_string(&full)
            .map_err(|e| format!("não foi possível ler {rel}: {e}"))?;
        Ok((rel, text))
    }

    /// Tem um projeto na pasta?
    pub(crate) fn has_project(&self) -> bool {
        self.root.join("config.lua").is_file() || self.root.join("main.lua").is_file()
    }
}
