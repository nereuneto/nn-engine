//! Arquivo de animações de uma imagem (`heroi.png` → `heroi.anim`): o
//! tamanho do quadro e as animações com nome. O formato é o mesmo que a
//! engine lê (`cena.lua`, `M.parse_anims`).

use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

use mlua::{Lua, Table, Value};

use crate::scene_data::format_num;

/// Como a animação toca.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Loop,
    Once,
    PingPong,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Loop, Mode::Once, Mode::PingPong];

    pub fn id(self) -> &'static str {
        match self {
            Mode::Loop => "loop",
            Mode::Once => "once",
            Mode::PingPong => "pingpong",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::Loop => "Repetir",
            Mode::Once => "Uma vez",
            Mode::PingPong => "Ida e volta",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.id() == s)
    }
}

/// Um quadro da animação: o número na folha (a partir de 1) e, se tiver, a
/// duração própria em segundos.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub index: u32,
    pub time: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Anim {
    pub name: String,
    pub frames: Vec<Frame>,
    pub fps: f64,
    pub mode: Mode,
}

impl Anim {
    pub fn new(name: &str, frames: impl IntoIterator<Item = u32>) -> Self {
        Self {
            name: name.to_string(),
            frames: frames
                .into_iter()
                .map(|index| Frame { index, time: None })
                .collect(),
            fps: 10.0,
            mode: Mode::Loop,
        }
    }

    /// Duração do quadro `i` (em segundos).
    pub fn frame_time(&self, i: usize) -> f64 {
        self.frames
            .get(i)
            .and_then(|f| f.time)
            .filter(|t| *t > 0.0)
            .unwrap_or(1.0 / self.fps.max(0.01))
    }

    /// Passada completa (posições na lista de quadros).
    fn sequence(&self) -> Vec<usize> {
        let n = self.frames.len();
        let mut seq: Vec<usize> = (0..n).collect();
        if self.mode == Mode::PingPong && n > 2 {
            seq.extend((1..n - 1).rev());
        }
        seq
    }

    pub fn duration(&self) -> f64 {
        self.sequence().iter().map(|&i| self.frame_time(i)).sum()
    }

    /// Posição na lista de quadros no instante `t` (para a prévia).
    pub fn position_at(&self, t: f64) -> Option<usize> {
        if self.frames.is_empty() {
            return None;
        }
        let total = self.duration();
        let mut t = t.max(0.0);
        if self.mode == Mode::Once {
            if t >= total {
                return Some(self.frames.len() - 1);
            }
        } else if total > 0.0 {
            t %= total;
        }
        let seq = self.sequence();
        for &i in &seq {
            let d = self.frame_time(i);
            if t < d {
                return Some(i);
            }
            t -= d;
        }
        seq.last().copied()
    }
}

/// O conteúdo de um `.anim`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimFile {
    pub frame_w: u32,
    pub frame_h: u32,
    pub anims: Vec<Anim>,
}

/// Caminho do `.anim` de uma imagem.
pub fn anim_path(image: &str) -> String {
    if image.to_lowercase().ends_with(".anim") {
        return image.to_string();
    }
    let slash = image.rfind('/').map_or(0, |i| i + 1);
    match image[slash..].rfind('.') {
        Some(dot) => format!("{}.anim", &image[..slash + dot]),
        None => format!("{image}.anim"),
    }
}

fn num(t: &Table, key: &str) -> Option<f64> {
    match t.get::<Value>(key) {
        Ok(Value::Integer(i)) => Some(i as f64),
        Ok(Value::Number(n)) => Some(n),
        _ => None,
    }
}

impl AnimFile {
    pub fn read(text: &str, name: &str) -> Result<Self, String> {
        let lua = Lua::new();
        let value: Value = lua
            .load(text)
            .set_name(format!("@{name}"))
            .eval()
            .map_err(|e| nn_script::translate_error(e.to_string().trim()))?;
        let Value::Table(t) = value else {
            return Err(format!("{name}: o .anim deve devolver uma tabela"));
        };
        let mut file = AnimFile {
            frame_w: num(&t, "frame_w").unwrap_or(0.0).max(0.0) as u32,
            frame_h: num(&t, "frame_h").unwrap_or(0.0).max(0.0) as u32,
            anims: Vec::new(),
        };
        if let Ok(Value::Table(list)) = t.get::<Value>("animations") {
            for (i, a) in list.sequence_values::<Table>().enumerate() {
                let a = a.map_err(|_| format!("{name}: cada animação é uma tabela"))?;
                let anim_name = match a.get::<Value>("name") {
                    Ok(Value::String(s)) => s.to_string_lossy(),
                    _ => format!("animacao{}", i + 1),
                };
                let times: Vec<Option<f64>> = match a.get::<Value>("times") {
                    Ok(Value::Table(tt)) => (1..=tt.raw_len())
                        .map(|k| match tt.get::<Value>(k) {
                            Ok(Value::Integer(v)) => Some(v as f64),
                            Ok(Value::Number(v)) => Some(v),
                            _ => None,
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                let mut frames = Vec::new();
                if let Ok(Value::Table(ft)) = a.get::<Value>("frames") {
                    for (k, v) in ft.sequence_values::<Value>().enumerate() {
                        let n = match v {
                            Ok(Value::Integer(n)) => n as f64,
                            Ok(Value::Number(n)) => n,
                            _ => continue,
                        };
                        if n >= 1.0 {
                            frames.push(Frame {
                                index: n as u32,
                                time: times.get(k).copied().flatten().filter(|t| *t > 0.0),
                            });
                        }
                    }
                }
                let mode = match a.get::<Value>("mode") {
                    Ok(Value::String(s)) => {
                        let s = s.to_string_lossy();
                        Mode::parse(&s).ok_or_else(|| {
                            format!(
                                "{name}: animação \"{anim_name}\": modo \"{s}\" não existe (use loop, once ou pingpong)"
                            )
                        })?
                    }
                    _ => Mode::Loop,
                };
                let fps = num(&a, "fps").filter(|f| *f > 0.0).unwrap_or(10.0);
                // Duração igual ao ritmo da animação não é duração própria.
                for f in &mut frames {
                    if f.time.is_some_and(|t| (t - 1.0 / fps).abs() < 1e-6) {
                        f.time = None;
                    }
                }
                file.anims.push(Anim {
                    name: anim_name,
                    frames,
                    fps,
                    mode,
                });
            }
        }
        Ok(file)
    }

    pub fn load(root: &Path, rel: &str) -> Result<Self, String> {
        let path = anim_path(rel);
        let text = std::fs::read_to_string(root.join(&path)).map_err(|e| format!("{path}: {e}"))?;
        Self::read(&text, &path)
    }

    /// O texto do arquivo, no formato que a engine lê.
    pub fn write(&self, image: &str) -> String {
        let mut s = String::new();
        let short = image.rsplit('/').next().unwrap_or(image);
        s.push_str(&format!(
            "-- Animações de {short} (feitas no NN Studio).\nreturn {{\n  format = 1,\n  frame_w = {},\n  frame_h = {},\n  animations = {{\n",
            self.frame_w, self.frame_h
        ));
        for a in &self.anims {
            let frames: Vec<String> = a.frames.iter().map(|f| f.index.to_string()).collect();
            s.push_str(&format!(
                "    {{ name = {}, frames = {{ {} }}, fps = {}",
                lua_string(&a.name),
                frames.join(", "),
                format_num(a.fps)
            ));
            if a.mode != Mode::Loop {
                s.push_str(&format!(", mode = \"{}\"", a.mode.id()));
            }
            if a.frames.iter().any(|f| f.time.is_some()) {
                let times: Vec<String> = (0..a.frames.len())
                    .map(|i| format_num(a.frame_time(i)))
                    .collect();
                s.push_str(&format!(", times = {{ {} }}", times.join(", ")));
            }
            s.push_str(" },\n");
        }
        s.push_str("  },\n}\n");
        s
    }

    pub fn find(&self, name: &str) -> Option<&Anim> {
        self.anims.iter().find(|a| a.name == name)
    }

    /// Um nome que ainda não existe ("andar", "andar2"...).
    pub fn free_name(&self, base: &str) -> String {
        if self.find(base).is_none() {
            return base.to_string();
        }
        (2..)
            .map(|i| format!("{base}{i}"))
            .find(|n| self.find(n).is_none())
            .unwrap()
    }
}

fn lua_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Os `.anim` do projeto, relidos quando o arquivo muda (para as listas de
/// nomes do Inspetor e dos eventos e para o tamanho do quadro na cena).
#[derive(Debug, Default)]
pub struct AnimCache {
    files: HashMap<String, (Option<SystemTime>, Option<AnimFile>)>,
}

impl AnimCache {
    /// As animações da imagem `image` (ou `None` sem `.anim`).
    pub fn get(&mut self, root: &Path, image: &str) -> Option<&AnimFile> {
        if image.is_empty() {
            return None;
        }
        let rel = anim_path(image);
        let full = root.join(&rel);
        let modified = std::fs::metadata(&full).and_then(|m| m.modified()).ok();
        let stale = self.files.get(&rel).is_none_or(|(m, _)| *m != modified);
        if stale {
            let loaded = modified.and_then(|_| AnimFile::load(root, image).ok());
            self.files.insert(rel.clone(), (modified, loaded));
        }
        self.files.get(&rel).and_then(|(_, f)| f.as_ref())
    }

    /// Nomes das animações da imagem.
    pub fn names(&mut self, root: &Path, image: &str) -> Vec<String> {
        self.get(root, image)
            .map(|f| f.anims.iter().map(|a| a.name.clone()).collect())
            .unwrap_or_default()
    }

    /// Todos os nomes de animação do projeto (sem repetir), para campos que
    /// não sabem de qual imagem é.
    pub fn all_names(&mut self, root: &Path, anim_files: &[String]) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for rel in anim_files {
            for n in self.names(root, rel) {
                if !out.contains(&n) {
                    out.push(n);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caminho_ao_lado_da_imagem() {
        assert_eq!(
            anim_path("assets/imagens/heroi.png"),
            "assets/imagens/heroi.anim"
        );
        assert_eq!(anim_path("a.b/heroi"), "a.b/heroi.anim");
        assert_eq!(anim_path("x/heroi.anim"), "x/heroi.anim");
    }

    #[test]
    fn le_e_grava_no_mesmo_formato() {
        let mut f = AnimFile {
            frame_w: 16,
            frame_h: 16,
            anims: vec![
                Anim::new("parado", [1, 2]),
                Anim::new("pulo \"alto\"", [5, 6, 7]),
            ],
        };
        f.anims[1].mode = Mode::Once;
        f.anims[1].fps = 8.0;
        f.anims[1].frames[1].time = Some(0.5);
        let text = f.write("assets/heroi.png");
        assert!(text.contains("frame_w = 16"), "{text}");
        assert!(text.contains("times = { 0.125, 0.5, 0.125 }"), "{text}");
        let back = AnimFile::read(&text, "heroi.anim").unwrap();
        assert_eq!(back, f);
        assert!((back.anims[1].duration() - 0.75).abs() < 1e-9);
    }

    #[test]
    fn previa_segue_o_modo() {
        let mut a = Anim::new("x", [1, 2, 3]);
        a.fps = 10.0;
        a.mode = Mode::PingPong;
        let pos: Vec<usize> = (0..6)
            .map(|i| a.position_at(i as f64 * 0.1 + 0.05).unwrap())
            .collect();
        assert_eq!(pos, [0, 1, 2, 1, 0, 1]);
        a.mode = Mode::Once;
        assert_eq!(a.position_at(10.0), Some(2));
    }

    #[test]
    fn modo_errado_explica() {
        let e = AnimFile::read(
            r#"return { animations = { { name = "a", frames = { 1 }, mode = "vaivem" } } }"#,
            "x.anim",
        )
        .unwrap_err();
        assert!(e.contains("\"vaivem\" não existe"), "{e}");
    }
}
