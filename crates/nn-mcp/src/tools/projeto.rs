//! O projeto: ver, criar, configurar e conferir.

use serde_json::{Value, json};

use nn_projeto::config_lua::{self, ConfigValues};
use nn_projeto::scene_data::{self, Prop};

use super::{Output, Tool, arg_str, opt_str, s_bool, s_enum, s_int, s_obj, s_str};
use crate::Server;

impl Server {
    pub(crate) fn config(&self) -> Option<(String, ConfigValues)> {
        let text = std::fs::read_to_string(self.root.join("config.lua")).ok()?;
        let folder = self
            .root
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let cfg = config_lua::read(&text, &folder).ok()?;
        Some((text, cfg))
    }

    /// O jogo é 3D? (A cena inicial, ou alguma cena, é 3D; ou o código usa
    /// `nn.d3`.)
    pub(crate) fn project_is_3d(&self) -> bool {
        let files = crate::paths::list_files(&self.root, "");
        if let Some((_, cfg)) = self.config()
            && cfg.start.ends_with(".cena")
            && let Ok(doc) = self.open_doc(&cfg.start)
        {
            return doc.data.three_d;
        }
        files.iter().any(|f| {
            (f.ends_with(".cena") || f.ends_with(".lua"))
                && std::fs::read_to_string(self.root.join(f))
                    .is_ok_and(|t| t.contains("mode = \"3d\"") || t.contains("nn.d3."))
        })
    }
}

fn group(files: &[String], exts: &[&str]) -> Vec<String> {
    files
        .iter()
        .filter(|f| {
            let lower = f.to_lowercase();
            exts.iter().any(|e| lower.ends_with(e))
        })
        .cloned()
        .collect()
}

fn short_list(list: &[String], max: usize) -> String {
    if list.is_empty() {
        return "(nenhum)".into();
    }
    let mut v: Vec<&str> = list.iter().take(max).map(String::as_str).collect();
    let more;
    if list.len() > max {
        more = format!("... e mais {}", list.len() - max);
        v.push(&more);
    }
    v.join(", ")
}

fn ver_projeto(s: &mut Server, _: &Value) -> Result<Output, String> {
    if !s.has_project() {
        let files = crate::paths::list_files(&s.root, "");
        let mut out = format!(
            "A pasta {} ainda não tem um jogo (falta o config.lua){}. Crie um com criar_projeto. Modelos:\n",
            s.root.display(),
            if files.is_empty() {
                String::new()
            } else {
                format!(" e já tem {} arquivo(s)", files.len())
            }
        );
        for t in &s.hooks.templates {
            out.push_str(&format!(
                "- {}{}: {}\n",
                t.name,
                if t.three_d { " (3D)" } else { "" },
                t.description
            ));
        }
        return Ok(Output::text(out));
    }
    let files = crate::paths::list_files(&s.root, "");
    let mut out = String::new();
    match s.config() {
        Some((_, c)) => {
            out.push_str(&format!(
                "Jogo \"{}\" (id {}), nn-engine {}. Pasta: {}\n",
                c.title,
                c.id,
                crate::VERSION,
                s.root.display()
            ));
            out.push_str(&format!(
                "Tipo: {}. Janela {}x{}{}. Fundo {}. Cena inicial: {}{}\n",
                if s.project_is_3d() { "3D" } else { "2D" },
                c.width,
                c.height,
                match c.resolution {
                    Some((w, h)) => format!(", tamanho do jogo {w}x{h} ({})", c.scale),
                    None => String::new(),
                },
                c.clear_color,
                c.start,
                if s.root.join(&c.start).is_file() {
                    ""
                } else {
                    " (NÃO EXISTE)"
                }
            ));
            let vars: Vec<String> = c
                .vars
                .iter()
                .map(|(k, v)| format!("{k} = {}", v.to_lua()))
                .collect();
            out.push_str(&format!(
                "Variáveis do jogo (nn.vars): {}\n",
                if vars.is_empty() {
                    "(nenhuma)".into()
                } else {
                    vars.join(", ")
                }
            ));
        }
        None => out.push_str("O config.lua tem um erro (veja conferir_projeto).\n"),
    }
    let sections: [(&str, Vec<String>); 10] = [
        ("Cenas visuais", group(&files, &[".cena"])),
        ("Prefabs", group(&files, &[".prefab"])),
        ("Scripts Lua", group(&files, &[".lua"])),
        ("Imagens", group(&files, &[".png", ".jpg", ".jpeg"])),
        ("Animações de imagem", group(&files, &[".anim"])),
        ("Sons", group(&files, &[".wav", ".ogg", ".mp3", ".flac"])),
        ("Mapas", group(&files, &[".tmj", ".tsj"])),
        ("Modelos 3D", group(&files, &[".glb", ".gltf", ".modelo"])),
        ("Diálogos", group(&files, &[".dialogo"])),
        ("Fontes", group(&files, &[".ttf", ".otf"])),
    ];
    for (title, list) in &sections {
        out.push_str(&format!(
            "{title} ({}): {}\n",
            list.len(),
            short_list(list, 60)
        ));
    }
    let docs: Vec<String> = group(&files, &[".md", ".txt"]);
    if !docs.is_empty() {
        out.push_str(&format!(
            "Textos ({}): {}\n",
            docs.len(),
            short_list(&docs, 20)
        ));
    }
    let tests = group(&files, &[".lua"])
        .into_iter()
        .filter(|f| f.starts_with("testes/"))
        .count();
    out.push_str(&if tests > 0 {
        format!("Testes de jogo: {tests} arquivo(s) em testes/ (rodar_testes)\n")
    } else {
        "Testes de jogo: nenhum ainda (escreva em testes/; veja buscar_documentacao \"testes de jogo\")\n".into()
    });
    if s.root.join("AGENTS.md").is_file() {
        out.push_str(
            "\nLeia o AGENTS.md do projeto (ler_arquivo) para saber como ele está organizado.\n",
        );
    }
    Ok(Output::text(out))
}

fn criar_projeto(s: &mut Server, a: &Value) -> Result<Output, String> {
    let template = arg_str(a, "modelo")?;
    let Some(info) = s
        .hooks
        .templates
        .iter()
        .find(|t| t.name == template)
        .cloned()
    else {
        let names: Vec<&str> = s.hooks.templates.iter().map(|t| t.name.as_str()).collect();
        return Err(format!(
            "modelo desconhecido: \"{template}\". Modelos: {}",
            names.join(", ")
        ));
    };
    let Some(create) = s.hooks.create_project else {
        return Err("este servidor não cria projetos".into());
    };
    let title = opt_str(a, "titulo")?;
    let dir = match opt_str(a, "pasta")? {
        Some(p) => {
            let (_, full) = crate::paths::resolve(&s.root, p)?;
            full
        }
        None => {
            if s.has_project() {
                return Err(
                    "esta pasta já tem um jogo. Para criar outro dentro dela, passe \"pasta\""
                        .into(),
                );
            }
            let files = crate::paths::list_files(&s.root, "");
            if !files.is_empty() {
                return Err(format!(
                    "a pasta já tem arquivos ({}); passe \"pasta\" para criar o jogo numa pasta nova dentro dela",
                    short_list(&files, 5)
                ));
            }
            s.root.clone()
        }
    };
    let created = if dir == s.root {
        // A pasta existe vazia (ou só com pastas ocultas): cria num lugar
        // ao lado e traz os arquivos para cá.
        // (Com o nome da pasta, que vira o título e o id do jogo.)
        let folder = s
            .root
            .file_name()
            .map(|n| n.to_os_string())
            .unwrap_or_else(|| "jogo".into());
        let tmp = s.root.join(".nn").join(folder);
        let _ = std::fs::remove_dir_all(&tmp);
        let files = create(&tmp, &info.name, title)?;
        for f in &files {
            let from = tmp.join(f);
            let data = std::fs::read(&from).map_err(|e| format!("{f}: {e}"))?;
            s.write_file(f, &data)?;
        }
        let _ = std::fs::remove_dir_all(s.root.join(".nn"));
        files
    } else {
        let files = create(&dir, &info.name, title)?;
        s.root = crate::paths::canonical(&dir);
        files
    };
    Ok(Output::text(format!(
        "Criei o jogo ({}, modelo {}) em {} com {} arquivos. O servidor agora trabalha nesta pasta. Veja com ver_projeto e leia o AGENTS.md.",
        if info.three_d { "3D" } else { "2D" },
        info.name,
        s.root.display(),
        created.len()
    )))
}

/// Muda o `config.lua` (sem perder os comentários).
pub(crate) fn set_config(s: &mut Server, a: &Value) -> Result<Vec<String>, String> {
    let Some((text, old)) = s.config() else {
        return Err("o config.lua não existe ou tem erro; conserte-o com escrever_arquivo".into());
    };
    let mut new = old.clone();
    let mut changed = Vec::new();
    let map = a.as_object().cloned().unwrap_or_default();
    for (k, v) in &map {
        let str_of = |v: &Value| {
            v.as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("\"{k}\" precisa ser um texto"))
        };
        let int_of = |v: &Value| {
            v.as_u64()
                .filter(|n| *n > 0 && *n < 100_000)
                .map(|n| n as u32)
                .ok_or_else(|| format!("\"{k}\" precisa ser um número inteiro positivo"))
        };
        let bool_of = |v: &Value| {
            v.as_bool()
                .ok_or_else(|| format!("\"{k}\" precisa ser true ou false"))
        };
        match k.as_str() {
            "title" => new.title = str_of(v)?,
            "start" => {
                let rel = crate::paths::clean(&str_of(v)?)?;
                if !s.root.join(&rel).is_file() {
                    return Err(format!("a cena inicial {rel} não existe"));
                }
                new.start = rel;
            }
            "clear_color" => new.clear_color = str_of(v)?,
            "width" => new.width = int_of(v)?,
            "height" => new.height = int_of(v)?,
            "fixed_hz" => new.fixed_hz = int_of(v)?,
            "fullscreen" => new.fullscreen = bool_of(v)?,
            "resizable" => new.resizable = bool_of(v)?,
            "vsync" => new.vsync = bool_of(v)?,
            "scale" => {
                let sc = str_of(v)?;
                if !["fit", "stretch", "pixel"].contains(&sc.as_str()) {
                    return Err("\"scale\" é \"fit\", \"stretch\" ou \"pixel\"".into());
                }
                new.scale = sc;
            }
            "resolution" => {
                new.resolution = match v {
                    Value::Null | Value::Bool(false) => None,
                    Value::Array(wh) if wh.len() == 2 => Some((int_of(&wh[0])?, int_of(&wh[1])?)),
                    _ => return Err("\"resolution\" é [largura, altura] ou null".into()),
                }
            }
            "vars" => {
                let m = v
                    .as_object()
                    .ok_or("\"vars\" é um objeto { nome: valor } (null tira a variável)")?;
                for (name, val) in m {
                    if !config_lua::valid_var_name(name) {
                        return Err(format!(
                            "\"{name}\" não serve de nome de variável (use letras, números e _)"
                        ));
                    }
                    new.vars.retain(|(n, _)| n != name);
                    let p = match val {
                        Value::Null => continue,
                        Value::Number(n) => Prop::Num(n.as_f64().unwrap_or(0.0)),
                        Value::Bool(b) => Prop::Bool(*b),
                        Value::String(t) => Prop::Str(t.clone()),
                        _ => {
                            return Err(format!(
                                "a variável \"{name}\" é número, texto ou true/false"
                            ));
                        }
                    };
                    new.vars.push((name.clone(), p));
                }
                new.vars.sort_by(|a, b| a.0.cmp(&b.0));
            }
            "theme" => {
                let m = v
                    .as_object()
                    .ok_or("\"theme\" é um objeto { chave: valor }")?;
                for (key, val) in m {
                    if !scene_data::THEME_KEYS.contains(&key.as_str()) {
                        return Err(format!(
                            "o tema não tem \"{key}\". Chaves: {}",
                            scene_data::THEME_KEYS.join(", ")
                        ));
                    }
                    new.theme.retain(|(k, _)| k != key);
                    if let Some(t) = val.as_str().filter(|t| !t.is_empty()) {
                        new.theme.push((key.clone(), t.to_string()));
                    }
                }
                new.theme.sort_by(|a, b| a.0.cmp(&b.0));
            }
            other => {
                return Err(format!(
                    "o config.lua não tem \"{other}\" aqui. Use title, start, width, height, resolution, scale, clear_color, fullscreen, resizable, vsync, fixed_hz, vars, theme"
                ));
            }
        }
        changed.push(k.clone());
    }
    let changes = config_lua::changes(&old, &new);
    let updated = config_lua::update(&text, &changes, &new);
    s.write_file("config.lua", updated.as_bytes())?;
    Ok(changed)
}

fn mudar_config(s: &mut Server, a: &Value) -> Result<Output, String> {
    let changed = set_config(s, a)?;
    if changed.is_empty() {
        return Err("nada para mudar: passe title, start, width, height, vars...".into());
    }
    Ok(Output::text(format!(
        "Mudei no config.lua: {}.",
        changed.join(", ")
    )))
}

// conferir_projeto ---------------------------------------------------------

use super::valores::looks_like_file;

fn conferir_projeto(s: &mut Server, _: &Value) -> Result<Output, String> {
    if !s.has_project() {
        return Err("a pasta ainda não tem um jogo; crie com criar_projeto".into());
    }
    let mut problems: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let files = crate::paths::list_files(&s.root, "");
    // config.lua
    match std::fs::read_to_string(s.root.join("config.lua")) {
        Ok(text) => match config_lua::read(&text, "") {
            Ok(c) => {
                if !s.root.join(&c.start).is_file() {
                    problems.push(format!("config.lua: a cena inicial {} não existe", c.start));
                }
            }
            Err(e) => problems.push(format!("config.lua: {e}")),
        },
        Err(_) => warnings.push("sem config.lua: o jogo começa pelo main.lua".into()),
    }
    // Scripts
    let lua = mlua::Lua::new();
    let mut scripts = 0;
    for f in files.iter().filter(|f| f.ends_with(".lua")) {
        scripts += 1;
        if let Ok(text) = std::fs::read_to_string(s.root.join(f))
            && let Some(issue) = nn_projeto::lua_syntax::check(&lua, f, &text)
        {
            problems.push(format!("{f}:{}: {}", issue.line, issue.message));
        }
    }
    // Cenas e prefabs
    let mut scenes = 0;
    for f in files
        .iter()
        .filter(|f| f.ends_with(".cena") || f.ends_with(".prefab"))
    {
        scenes += 1;
        let doc = match s.open_doc(f) {
            Ok(d) => d,
            Err(e) => {
                problems.push(format!("{f}: {e}"));
                continue;
            }
        };
        let three_d = doc.three_d();
        let mut objects = Vec::new();
        scene_data::visit(&doc.data.objects, &mut |o| objects.push(o.clone()));
        for o in &objects {
            let at = format!("{f}: \"{}\" (id {})", o.name, o.id);
            if let Some(link) = &o.prefab
                && !s.root.join(&link.path).is_file()
            {
                problems.push(format!("{at}: o prefab {} não existe", link.path));
            }
            for c in &o.components {
                for (k, v) in &c.props {
                    if let Prop::Str(path) = v
                        && looks_like_file(path)
                        && !s.root.join(path).is_file()
                    {
                        problems.push(format!("{at}: {}.{k} = \"{path}\" não existe", c.kind));
                    }
                }
                if !c.is_behavior() {
                    if valores_kind_3d(&c.kind) && !three_d && !doc.prefab {
                        warnings.push(format!("{at}: \"{}\" é de cenas 3D", c.kind));
                    }
                    continue;
                }
                let script = c.str("script").to_string();
                match s.behavior(&script) {
                    Err(e) => problems.push(format!("{at}: {e}")),
                    Ok(info) => {
                        for (k, _) in &c.props {
                            if k != "script" && info.prop(k).is_none() {
                                warnings.push(format!(
                                    "{at}: {script} não tem mais a propriedade \"{k}\" (fica sem efeito)"
                                ));
                            }
                        }
                        let mut tmp = o.clone();
                        for note in super::valores::add_requirements(&mut tmp, &info, three_d) {
                            warnings
                                .push(format!("{at}: {}", note.replace("ganhou", "precisa de")));
                        }
                    }
                }
            }
        }
        // Regras com ids que não existem mais (a leitura já confere os campos).
        let cat = nn_projeto::events::catalog();
        let mut rules: Vec<(String, nn_projeto::scene_data::Rule)> = doc
            .data
            .events
            .iter()
            .map(|r| ("a cena".to_string(), r.clone()))
            .collect();
        for o in &objects {
            if o.prefab.is_none() {
                rules.extend(
                    o.events
                        .iter()
                        .map(|r| (format!("\"{}\"", o.name), r.clone())),
                );
            }
        }
        for (owner, r) in rules {
            for i in &r.when {
                if cat.condition(&i.id).is_none() {
                    problems.push(format!("{f}: {owner}: condição desconhecida \"{}\"", i.id));
                }
            }
            for i in &r.actions {
                if cat.action(&i.id).is_none() {
                    problems.push(format!("{f}: {owner}: ação desconhecida \"{}\"", i.id));
                }
                for (k, v) in &i.props {
                    if let Prop::Str(path) = v
                        && looks_like_file(path)
                        && !s.root.join(path).is_file()
                    {
                        problems.push(format!(
                            "{f}: {owner}: {}.{k} = \"{path}\" não existe",
                            i.id
                        ));
                    }
                }
            }
        }
    }
    let mut out = format!("Conferi {scripts} script(s) Lua e {scenes} cena(s)/prefab(s). ");
    if problems.is_empty() && warnings.is_empty() {
        out.push_str("Tudo certo.");
    } else {
        if !problems.is_empty() {
            out.push_str(&format!("{} problema(s):\n", problems.len()));
            for p in &problems {
                out.push_str(&format!("- {p}\n"));
            }
        } else {
            out.push_str("Nenhum problema.\n");
        }
        if !warnings.is_empty() {
            out.push_str(&format!("{} aviso(s):\n", warnings.len()));
            for w in &warnings {
                out.push_str(&format!("- {w}\n"));
            }
        }
    }
    out.push_str("\n(Isto confere os arquivos sem rodar o jogo; erros que só aparecem jogando ainda podem existir.)");
    Ok(Output::text(out))
}

fn valores_kind_3d(kind: &str) -> bool {
    super::valores::KINDS_3D.contains(&kind)
}

pub static TOOLS: &[&Tool] = &[
    &Tool {
        name: "ver_projeto",
        title: "Ver projeto",
        description: "Comece por aqui. Mostra o jogo: título, 2D ou 3D, tamanho da tela, cena inicial, variáveis e todos os arquivos (cenas, prefabs, scripts, imagens, sons, mapas, modelos). Se a pasta ainda não tem jogo, mostra os modelos de criar_projeto.",
        params: Vec::new,
        read_only: true,
        destructive: false,
        run: ver_projeto,
    },
    &Tool {
        name: "criar_projeto",
        title: "Criar projeto",
        description: "Cria um jogo novo a partir de um modelo (veja os modelos em ver_projeto): na pasta do servidor, se estiver vazia, ou numa pasta nova dentro dela (\"pasta\"; o servidor passa a trabalhar nela).",
        params: || {
            vec![
                (
                    "modelo",
                    s_str(
                        "Nome do modelo: padrao, visual, aventura, nave, visual3d, vale3d, 3d, vazio, vazio3d, minimo",
                    ),
                    true,
                ),
                ("titulo", s_str("Título do jogo"), false),
                (
                    "pasta",
                    s_str("Criar numa pasta nova dentro desta (ex.: \"meu-jogo\")"),
                    false,
                ),
            ]
        },
        read_only: false,
        destructive: false,
        run: criar_projeto,
    },
    &Tool {
        name: "mudar_config",
        title: "Mudar configuração",
        description: "Muda o config.lua sem perder os comentários: title, start (cena inicial), width/height (janela), resolution ([w, h] do jogo, ou null), scale (fit|stretch|pixel), clear_color, fullscreen, resizable, vsync, fixed_hz, vars (variáveis do jogo, { nome: valor }; null tira) e theme (cores e fonte da interface).",
        params: || {
            vec![
                ("title", s_str("Título do jogo"), false),
                ("start", s_str("Cena (ou script) inicial"), false),
                ("width", s_int("Largura da janela"), false),
                ("height", s_int("Altura da janela"), false),
                (
                    "resolution",
                    json!({ "type": ["array", "null"], "items": { "type": "integer" }, "description": "Tamanho fixo do jogo [largura, altura]; null tira" }),
                    false,
                ),
                (
                    "scale",
                    s_enum(
                        "Como o tamanho fixo se ajusta à janela",
                        &["fit", "stretch", "pixel"],
                    ),
                    false,
                ),
                ("clear_color", s_str("Cor de fundo, \"#RRGGBB\""), false),
                ("fullscreen", s_bool("Abrir em tela cheia"), false),
                ("resizable", s_bool("Janela muda de tamanho"), false),
                ("vsync", s_bool("Sincronia vertical"), false),
                ("fixed_hz", s_int("Passos por segundo da física"), false),
                (
                    "vars",
                    s_obj("Variáveis do jogo { nome: valor } (null tira)"),
                    false,
                ),
                (
                    "theme",
                    s_obj(
                        "Tema da interface { accent: \"#...\", font: \"...\" } (\"\" volta ao padrão)",
                    ),
                    false,
                ),
            ]
        },
        read_only: false,
        destructive: false,
        run: mudar_config,
    },
    &Tool {
        name: "conferir_projeto",
        title: "Conferir projeto",
        description: "Confere o projeto todo sem rodar o jogo: config.lua, sintaxe de todos os scripts Lua, cenas e prefabs que não abrem, arquivos citados que não existem (imagens, sons, prefabs, cenas), comportamentos desconhecidos ou faltando componentes, e eventos com condição ou ação desconhecida. Use depois de cada leva de mudanças.",
        params: Vec::new,
        read_only: true,
        destructive: false,
        run: conferir_projeto,
    },
];
