//! Arquivos do projeto: a lista (com atualização automática quando algo muda
//! no disco) e as operações (criar, renomear, importar, mandar para a lixeira).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// Intervalo entre as verificações de mudanças no disco.
pub const POLL_INTERVAL: Duration = Duration::from_millis(800);

/// Projetos maiores que isso param de ser listados (pasta errada?).
pub const MAX_ENTRIES: usize = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileKind {
    Folder,
    Lua,
    /// Cena visual do NN Studio (`.cena`).
    Scene,
    Image,
    Sound,
    Map,
    Tileset,
    Text,
    Font,
    Other,
}

impl FileKind {
    pub fn of(path: &Path) -> Self {
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        match ext.as_str() {
            "lua" => FileKind::Lua,
            "cena" => FileKind::Scene,
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" => FileKind::Image,
            "wav" | "ogg" | "mp3" | "flac" => FileKind::Sound,
            "tmj" | "tmx" => FileKind::Map,
            "tsj" | "tsx" => FileKind::Tileset,
            "md" | "txt" | "json" | "csv" | "toml" | "gitignore" => FileKind::Text,
            "ttf" | "otf" => FileKind::Font,
            _ => FileKind::Other,
        }
    }

    /// Nome do tipo para o usuário.
    pub fn label(self) -> &'static str {
        match self {
            FileKind::Folder => "pasta",
            FileKind::Lua => "script Lua",
            FileKind::Scene => "cena visual",
            FileKind::Image => "imagem",
            FileKind::Sound => "som",
            FileKind::Map => "mapa",
            FileKind::Tileset => "tileset",
            FileKind::Text => "texto",
            FileKind::Font => "fonte",
            FileKind::Other => "arquivo",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileEntry {
    /// Caminho relativo à pasta do projeto, com `/`.
    pub rel: String,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
    pub kind: FileKind,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

/// Mudanças desde a última leitura.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Changes {
    /// Arquivos novos ou alterados.
    pub changed: Vec<String>,
    pub removed: Vec<String>,
}

impl Changes {
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.removed.is_empty()
    }
}

/// Os arquivos do projeto, em ordem de árvore (pastas primeiro).
#[derive(Debug)]
pub struct FileTree {
    root: PathBuf,
    entries: Vec<FileEntry>,
    stamps: HashMap<String, (u64, Option<SystemTime>)>,
    last_scan: Instant,
    pub truncated: bool,
}

impl FileTree {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let mut tree = Self {
            root: root.into(),
            entries: Vec::new(),
            stamps: HashMap::new(),
            last_scan: Instant::now(),
            truncated: false,
        };
        tree.rescan();
        tree
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
    }

    pub fn get(&self, rel: &str) -> Option<&FileEntry> {
        self.entries.iter().find(|e| e.rel == rel)
    }

    pub fn exists(&self, rel: &str) -> bool {
        self.get(rel).is_some()
    }

    /// Caminho completo de um arquivo do projeto.
    pub fn full(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    /// Todos os arquivos (sem pastas) de um tipo.
    pub fn files_of(&self, kind: FileKind) -> impl Iterator<Item = &FileEntry> {
        self.entries.iter().filter(move |e| e.kind == kind)
    }

    /// Lê de novo se já passou o intervalo. Devolve o que mudou.
    pub fn poll(&mut self) -> Changes {
        if self.last_scan.elapsed() < POLL_INTERVAL {
            return Changes::default();
        }
        self.rescan()
    }

    /// Lê a pasta de novo agora.
    pub fn rescan(&mut self) -> Changes {
        self.last_scan = Instant::now();
        let mut entries = Vec::new();
        self.truncated = false;
        walk(&self.root, "", 0, &mut entries, &mut self.truncated);
        let mut stamps = HashMap::with_capacity(entries.len());
        let mut changes = Changes::default();
        for e in entries.iter().filter(|e| !e.is_dir) {
            let stamp = (e.size, e.modified);
            if self.stamps.get(&e.rel) != Some(&stamp) && !self.stamps.is_empty() {
                changes.changed.push(e.rel.clone());
            }
            stamps.insert(e.rel.clone(), stamp);
        }
        changes.removed = self
            .stamps
            .keys()
            .filter(|k| !stamps.contains_key(*k))
            .cloned()
            .collect();
        changes.removed.sort();
        self.stamps = stamps;
        self.entries = entries;
        changes
    }
}

fn walk(dir: &Path, prefix: &str, depth: usize, out: &mut Vec<FileEntry>, truncated: &mut bool) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut items: Vec<(String, std::fs::Metadata)> = read
        .filter_map(Result::ok)
        .filter_map(|d| {
            let name = d.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                return None;
            }
            d.metadata().ok().map(|m| (name, m))
        })
        .collect();
    // Pastas primeiro; depois por nome, sem diferenciar maiúsculas.
    items.sort_by(|(a, ma), (b, mb)| {
        mb.is_dir()
            .cmp(&ma.is_dir())
            .then_with(|| natural_cmp(&a.to_lowercase(), &b.to_lowercase()))
    });
    for (name, meta) in items {
        if out.len() >= MAX_ENTRIES {
            *truncated = true;
            return;
        }
        let rel = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let is_dir = meta.is_dir();
        let child = dir.join(&name);
        out.push(FileEntry {
            kind: if is_dir {
                FileKind::Folder
            } else {
                FileKind::of(Path::new(&name))
            },
            rel: rel.clone(),
            name,
            depth,
            is_dir,
            size: if is_dir { 0 } else { meta.len() },
            modified: meta.modified().ok(),
        });
        if is_dir {
            walk(&child, &rel, depth + 1, out, truncated);
        }
    }
}

/// Ordem "natural": "fase2" vem antes de "fase10".
pub fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    loop {
        match (ai.peek(), bi.peek()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, _) => return std::cmp::Ordering::Less,
            (_, None) => return std::cmp::Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let na: String = std::iter::from_fn(|| ai.next_if(char::is_ascii_digit)).collect();
                let nb: String = std::iter::from_fn(|| bi.next_if(char::is_ascii_digit)).collect();
                let ord = na
                    .trim_start_matches('0')
                    .len()
                    .cmp(&nb.trim_start_matches('0').len())
                    .then_with(|| na.trim_start_matches('0').cmp(nb.trim_start_matches('0')));
                if ord != std::cmp::Ordering::Equal {
                    return ord;
                }
            }
            (Some(x), Some(y)) => {
                let ord = x.cmp(y);
                if ord != std::cmp::Ordering::Equal {
                    return ord;
                }
                ai.next();
                bi.next();
            }
        }
    }
}

/// Confere um nome de arquivo ou pasta digitado pelo usuário.
pub fn validate_name(name: &str) -> Result<(), String> {
    let n = name.trim();
    if n.is_empty() {
        return Err("O nome não pode ficar vazio.".into());
    }
    if n == "." || n == ".." {
        return Err("Nome inválido.".into());
    }
    if let Some(c) = n.chars().find(|c| {
        matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control()
    }) {
        return Err(format!("O nome não pode ter o caractere {c:?}."));
    }
    if n.ends_with('.') {
        return Err("O nome não pode terminar com ponto.".into());
    }
    Ok(())
}

/// Caminho livre em `dir`: "nome.png", "nome (2).png", "nome (3).png"...
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let p = Path::new(name);
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_string());
    let ext = p
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    (2..)
        .map(|i| dir.join(format!("{stem} ({i}){ext}")))
        .find(|p| !p.exists())
        .unwrap_or(first)
}

/// Pasta onde um arquivo importado vai parar quando nenhuma foi escolhida:
/// imagens em `assets/imagens`, sons em `assets/sons`...
pub fn default_import_dir(root: &Path, kind: FileKind) -> String {
    let candidates: &[&str] = match kind {
        FileKind::Image => &["assets/imagens", "assets/images", "assets/img", "imagens"],
        FileKind::Sound => &["assets/sons", "assets/sounds", "assets/audio", "sons"],
        FileKind::Font => &["assets/fontes", "assets/fonts", "fontes"],
        FileKind::Map | FileKind::Tileset => &["fases", "mapas", "maps", "tilesets"],
        _ => &[],
    };
    for c in candidates {
        if root.join(c).is_dir() {
            return (*c).to_string();
        }
    }
    match kind {
        FileKind::Image => "assets/imagens".into(),
        FileKind::Sound => "assets/sons".into(),
        FileKind::Font => "assets/fontes".into(),
        _ => String::new(),
    }
}

/// Copia arquivos (ou pastas) de fora para dentro do projeto. `dest` é
/// relativo à raiz; `None` escolhe a pasta pelo tipo de cada arquivo.
/// Devolve os caminhos criados (relativos) ou o erro de cada um.
pub fn import(root: &Path, sources: &[PathBuf], dest: Option<&str>) -> Vec<Result<String, String>> {
    sources
        .iter()
        .map(|src| {
            let name = src
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .ok_or_else(|| format!("{}: caminho inválido", src.display()))?;
            let kind = if src.is_dir() {
                FileKind::Folder
            } else {
                FileKind::of(src)
            };
            let dir_rel = match dest {
                Some(d) => d.to_string(),
                None => default_import_dir(root, kind),
            };
            let dir = root.join(&dir_rel);
            // Arquivo que já está dentro do projeto, na mesma pasta: nada a fazer.
            if src.parent().and_then(|p| p.canonicalize().ok()) == dir.canonicalize().ok() {
                return Err(format!("{name} já está em {}", show_dir(&dir_rel)));
            }
            std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            let target = unique_path(&dir, &name);
            copy_recursive(src, &target).map_err(|e| format!("{name}: {e}"))?;
            Ok(relative(root, &target))
        })
        .collect()
}

fn show_dir(rel: &str) -> String {
    if rel.is_empty() {
        "a pasta do projeto".into()
    } else {
        rel.to_string()
    }
}

fn copy_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if src.is_dir() {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            copy_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        std::fs::copy(src, dst).map(|_| ())
    }
}

/// Caminho relativo com `/` (ou o caminho inteiro, se estiver fora da raiz).
pub fn relative(root: &Path, path: &Path) -> String {
    let root_c = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let path_c = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    path_c
        .strip_prefix(&root_c)
        .or_else(|_| path.strip_prefix(root))
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.to_string_lossy().into_owned())
}

/// Pasta (relativa) onde algo é criado a partir de uma seleção: a própria
/// pasta, ou a pasta do arquivo selecionado.
pub fn parent_dir(rel: &str, is_dir: bool) -> String {
    if is_dir {
        return rel.to_string();
    }
    rel.rsplit_once('/')
        .map(|(d, _)| d.to_string())
        .unwrap_or_default()
}

/// Junta pasta relativa e nome.
pub fn join_rel(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

/// Cria uma pasta. Devolve o caminho relativo.
pub fn create_folder(root: &Path, parent: &str, name: &str) -> Result<String, String> {
    validate_name(name)?;
    let rel = join_rel(parent, name.trim());
    let full = root.join(&rel);
    if full.exists() {
        return Err(format!("Já existe {rel}."));
    }
    std::fs::create_dir_all(&full).map_err(|e| e.to_string())?;
    Ok(rel)
}

/// Cria um arquivo com conteúdo. Acrescenta `.lua` se o nome não tiver.
pub fn create_file(root: &Path, parent: &str, name: &str, content: &str) -> Result<String, String> {
    validate_name(name)?;
    let mut name = name.trim().to_string();
    if Path::new(&name).extension().is_none() {
        name.push_str(".lua");
    }
    let rel = join_rel(parent, &name);
    let full = root.join(&rel);
    if full.exists() {
        return Err(format!("Já existe {rel}."));
    }
    if let Some(dir) = full.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&full, content).map_err(|e| e.to_string())?;
    Ok(rel)
}

/// Renomeia (na mesma pasta). Devolve o caminho relativo novo.
pub fn rename(root: &Path, rel: &str, new_name: &str) -> Result<String, String> {
    validate_name(new_name)?;
    let parent = rel.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let new_rel = join_rel(parent, new_name.trim());
    if new_rel == rel {
        return Ok(new_rel);
    }
    let to = root.join(&new_rel);
    // Só mudar maiúsculas/minúsculas é permitido mesmo onde o sistema não diferencia.
    if to.exists() && !new_rel.eq_ignore_ascii_case(rel) {
        return Err(format!("Já existe {new_rel}."));
    }
    std::fs::rename(root.join(rel), &to).map_err(|e| e.to_string())?;
    Ok(new_rel)
}

/// Manda para a lixeira do sistema.
pub fn move_to_trash(root: &Path, rel: &str) -> Result<(), String> {
    trash::delete(root.join(rel)).map_err(|e| e.to_string())
}

/// Grava um arquivo de texto sem deixá-lo pela metade se algo der errado.
pub fn write_atomic(path: &Path, content: &str) -> std::io::Result<()> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.nn-tmp"));
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// Nome do módulo para `require` ("lib/util.lua" → "lib.util").
pub fn module_name(rel: &str) -> String {
    rel.trim_end_matches(".lua")
        .trim_end_matches("/init")
        .replace('/', ".")
}

/// Conteúdo de uma cena nova.
pub fn scene_template(rel: &str) -> String {
    format!(
        "-- Cena {rel}. Para ir até ela: nn.scene.switch(\"{rel}\")\n\
         local cena = {{}}\n\
         \n\
         -- A cena começou: prepare tudo aqui.\n\
         function cena:enter(args)\n\
         \x20 self.x, self.y = 0, 0\n\
         end\n\
         \n\
         -- Passos fixos (60 por segundo): movimento e regras.\n\
         function cena:fixed_update(dt)\n\
         \x20 local dx, dy = nn.input.move()   -- WASD, setas ou controle\n\
         \x20 self.x = self.x + dx * 300 * dt\n\
         \x20 self.y = self.y + dy * 300 * dt\n\
         end\n\
         \n\
         -- Desenha a cena (a câmera começa no centro da tela).\n\
         function cena:draw()\n\
         \x20 nn.draw.rect(self.x, self.y, 50, 50, \"#3FA7D6\")\n\
         end\n\
         \n\
         return cena\n"
    )
}

/// Conteúdo de uma cena visual nova: um título e um quadrado.
pub fn visual_scene_template() -> String {
    use crate::scene_data::{Component, Object, Prop, SceneData, write};
    let mut title = Object::new(1, "titulo");
    title.y = 160.0;
    let mut text = Component::new("text");
    text.set("text", Prop::Str("Minha cena".into()));
    text.set("size", Prop::Num(48.0));
    title.components.push(text);
    let mut square = Object::new(2, "quadrado");
    square.components.push(Component::new("shape"));
    let scene = SceneData {
        objects: vec![title, square],
        ..SceneData::default()
    };
    write(&scene)
}

/// Script de uma cena visual: os objetos já existem em `self.objects`.
pub fn scene_script_template(scene_rel: &str) -> String {
    format!(
        "-- Script da cena visual {scene_rel}.\n\
         -- Os objetos montados no editor estão em self.objects (pelo nome).\n\
         local cena = {{}}\n\
         \n\
         -- A cena começou (os objetos já foram criados).\n\
         function cena:enter(args)\n\
         end\n\
         \n\
         -- Passos fixos (60 por segundo): movimento e regras.\n\
         function cena:fixed_update(dt)\n\
         \x20 -- Exemplo: mover o objeto \"quadrado\" com WASD, as setas ou o controle.\n\
         \x20 local q = self.objects.quadrado\n\
         \x20 if q then\n\
         \x20   local dx, dy = nn.input.move()\n\
         \x20   q.x = q.x + dx * 300 * dt\n\
         \x20   q.y = q.y + dy * 300 * dt\n\
         \x20 end\n\
         end\n\
         \n\
         -- Dois corpos se tocaram (e.object_a e e.object_b são os objetos).\n\
         function cena:contact(e)\n\
         end\n\
         \n\
         return cena\n"
    )
}

/// Conteúdo de um módulo novo (código compartilhado entre cenas).
pub fn module_template(rel: &str) -> String {
    let module = module_name(rel);
    let var = module.rsplit('.').next().unwrap_or("m").replace('-', "_");
    format!(
        "-- Módulo {module}. Para usar em outro arquivo:\n\
         --   local {var} = require(\"{module}\")\n\
         local {var} = {{}}\n\
         \n\
         function {var}.exemplo()\n\
         \x20 return 42\n\
         end\n\
         \n\
         return {var}\n"
    )
}

/// Tamanho legível.
pub fn human_size(bytes: u64) -> String {
    nn_run::human_size(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("nn-studio-files-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn write(dir: &Path, rel: &str, content: &str) {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, content).unwrap();
    }

    #[test]
    fn lista_em_ordem_de_arvore_sem_ocultos() {
        let dir = temp("lista");
        write(&dir, "main.lua", "");
        write(&dir, "config.lua", "");
        write(&dir, "cenas/fase10.lua", "");
        write(&dir, "cenas/fase2.lua", "");
        write(&dir, "assets/imagens/heroi.png", "");
        write(&dir, ".git/HEAD", "");
        write(&dir, ".gitignore", "");
        let t = FileTree::new(&dir);
        let rels: Vec<_> = t.entries().iter().map(|e| e.rel.as_str()).collect();
        assert_eq!(
            rels,
            [
                "assets",
                "assets/imagens",
                "assets/imagens/heroi.png",
                "cenas",
                "cenas/fase2.lua",
                "cenas/fase10.lua",
                "config.lua",
                "main.lua"
            ]
        );
        let heroi = t.get("assets/imagens/heroi.png").unwrap();
        assert_eq!((heroi.depth, heroi.kind), (2, FileKind::Image));
        assert_eq!(t.files_of(FileKind::Lua).count(), 4);
    }

    #[test]
    fn percebe_mudancas_no_disco() {
        let dir = temp("mudancas");
        write(&dir, "a.lua", "1");
        write(&dir, "b.lua", "2");
        let mut t = FileTree::new(&dir);
        assert!(t.rescan().is_empty());
        write(&dir, "a.lua", "mudou de tamanho");
        write(&dir, "c.lua", "novo");
        std::fs::remove_file(dir.join("b.lua")).unwrap();
        let c = t.rescan();
        let mut changed = c.changed.clone();
        changed.sort();
        assert_eq!(changed, ["a.lua", "c.lua"]);
        assert_eq!(c.removed, ["b.lua"]);
    }

    #[test]
    fn nomes_validos() {
        assert!(validate_name("heroi.png").is_ok());
        assert!(validate_name("cena de título.lua").is_ok());
        assert!(validate_name("").is_err());
        assert!(validate_name("  ").is_err());
        assert!(validate_name("a/b").is_err());
        assert!(validate_name("a:b").is_err());
        assert!(validate_name("..").is_err());
        assert!(validate_name("x.").is_err());
    }

    #[test]
    fn importa_para_a_pasta_certa_sem_sobrescrever() {
        let proj = temp("importa");
        let fora = temp("importa-fora");
        write(&fora, "heroi.png", "png");
        write(&fora, "pulo.wav", "wav");
        write(&fora, "notas.txt", "txt");
        write(&fora, "pack/a.png", "a");
        std::fs::create_dir_all(proj.join("assets/imagens")).unwrap();
        write(&proj, "assets/imagens/heroi.png", "antigo");

        let r = import(
            &proj,
            &[
                fora.join("heroi.png"),
                fora.join("pulo.wav"),
                fora.join("notas.txt"),
                fora.join("pack"),
            ],
            None,
        );
        let ok: Vec<_> = r.into_iter().map(Result::unwrap).collect();
        assert_eq!(
            ok,
            [
                "assets/imagens/heroi (2).png",
                "assets/sons/pulo.wav",
                "notas.txt",
                "pack"
            ]
        );
        assert_eq!(
            std::fs::read_to_string(proj.join("assets/imagens/heroi.png")).unwrap(),
            "antigo"
        );
        assert!(proj.join("pack/a.png").exists());

        // Pasta escolhida.
        let r = import(&proj, &[fora.join("heroi.png")], Some("cenas"));
        assert_eq!(r[0].as_deref(), Ok("cenas/heroi.png"));
        // Importar de dentro da própria pasta de destino não duplica.
        let r = import(&proj, &[proj.join("cenas/heroi.png")], Some("cenas"));
        assert!(r[0].is_err());
    }

    #[test]
    fn criar_renomear() {
        let proj = temp("criar");
        assert_eq!(create_folder(&proj, "", "cenas").unwrap(), "cenas");
        assert!(create_folder(&proj, "", "cenas").is_err());
        let rel = create_file(&proj, "cenas", "menu", "-- oi").unwrap();
        assert_eq!(rel, "cenas/menu.lua");
        assert!(create_file(&proj, "cenas", "menu.lua", "").is_err());
        assert_eq!(
            rename(&proj, "cenas/menu.lua", "inicio.lua").unwrap(),
            "cenas/inicio.lua"
        );
        assert!(proj.join("cenas/inicio.lua").exists());
        assert!(rename(&proj, "cenas/inicio.lua", "x/y.lua").is_err());
        create_file(&proj, "cenas", "outra", "").unwrap();
        assert!(rename(&proj, "cenas/inicio.lua", "outra.lua").is_err());
        // Só maiúsculas: permitido.
        assert_eq!(
            rename(&proj, "cenas/inicio.lua", "Inicio.lua").unwrap(),
            "cenas/Inicio.lua"
        );
    }

    #[test]
    fn grava_sem_deixar_temporario() {
        let proj = temp("grava");
        let p = proj.join("a.lua");
        write_atomic(&p, "um").unwrap();
        write_atomic(&p, "dois").unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "dois");
        assert_eq!(std::fs::read_dir(&proj).unwrap().count(), 1);
    }

    #[test]
    fn modelos_compilam() {
        let lua = mlua::Lua::new();
        assert_eq!(
            crate::lua_syntax::check(&lua, "x", &scene_template("cenas/menu.lua")),
            None
        );
        let m = module_template("lib/meu-util.lua");
        assert!(m.contains("require(\"lib.meu-util\")"), "{m}");
        assert_eq!(crate::lua_syntax::check(&lua, "x", &m), None);
        assert_eq!(module_name("lib/util/init.lua"), "lib.util");
        assert_eq!(parent_dir("cenas/a.lua", false), "cenas");
        assert_eq!(parent_dir("a.lua", false), "");
        assert_eq!(parent_dir("cenas", true), "cenas");
    }

    #[test]
    fn ordem_natural() {
        let mut v = vec!["fase10", "fase2", "fase1", "a", "fase02b"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, ["a", "fase1", "fase2", "fase02b", "fase10"]);
    }
}
