//! Pré-carregar: o que uma cena vai usar é carregado antes de ela começar,
//! para o jogo não travar no meio, no primeiro uso de uma imagem ou de um som.
//!
//! A lista vem da cena: numa cena visual, dos componentes (veja
//! `assets_of` no `cena.lua`); numa cena em Lua (e no script de uma cena
//! visual), do campo `preload`. Sem tela de carregamento, tudo carrega ao
//! entrar na cena, antes de ela montar os objetos. Com `loading` na troca de
//! cena, a [`LoadingScene`] carrega um pouco a cada quadro e mostra o
//! andamento.

use std::time::{Duration, Instant};

use glam::Vec2;
use mlua::{Function, Table, Value};
use nn_app::{AssetHandle, Context, Scene, Transition};
use nn_core::Color;
use nn_render::{Anchor, Text, TextureOptions};

use crate::LuaScene;
use crate::api::{LoadingScreen, Shared};

/// Uma coisa para carregar antes.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Preload {
    Texture(String, TextureOptions),
    Sound(String),
    Model(String, bool),
    Font(String),
    Map(String),
}

impl Preload {
    /// Um item da lista: um caminho (o tipo vem da extensão) ou uma tabela
    /// `{ "caminho", pixel_art = true }` / `{ kind = ..., path = ... }`.
    fn from_value(v: &Value) -> Option<Self> {
        let (path, kind, pixel_art, flat, mipmaps) = match v {
            Value::String(s) => (s.to_str().ok()?.to_string(), None, false, false, true),
            Value::Table(t) => {
                let path: Option<String> = t
                    .get::<Option<String>>("path")
                    .ok()
                    .flatten()
                    .or_else(|| t.get::<Option<String>>(1).ok().flatten());
                (
                    path?,
                    t.get::<Option<String>>("kind").ok().flatten(),
                    t.get::<Option<bool>>("pixel_art")
                        .ok()
                        .flatten()
                        .unwrap_or(false),
                    t.get::<Option<bool>>("flat")
                        .ok()
                        .flatten()
                        .unwrap_or(false),
                    t.get::<Option<bool>>("mipmaps")
                        .ok()
                        .flatten()
                        .unwrap_or(true),
                )
            }
            _ => return None,
        };
        if path.is_empty() {
            return None;
        }
        let ext = path
            .rsplit_once('.')
            .map(|(_, e)| e.to_ascii_lowercase())
            .unwrap_or_default();
        let kind = kind.unwrap_or_else(|| {
            match ext.as_str() {
                "png" | "jpg" | "jpeg" => "texture",
                "wav" | "ogg" | "mp3" => "sound",
                "glb" | "gltf" => "model",
                "ttf" | "otf" => "font",
                "tmj" | "json" => "map",
                _ => "",
            }
            .to_string()
        });
        Some(match kind.as_str() {
            "texture" => Self::Texture(path, crate::api::image_options(pixel_art, mipmaps)),
            "sound" => Self::Sound(path),
            "model" => Self::Model(path, flat),
            "font" => Self::Font(path),
            "map" => Self::Map(path),
            _ => {
                log::warn!(
                    "preload: não sei carregar {path} (use imagens, sons, modelos, fontes ou mapas)"
                );
                return None;
            }
        })
    }

    /// Carrega (ou acha no cache) e devolve quem segura o que carregou.
    fn load(&self, ctx: &mut Context) -> Result<Vec<AssetHandle>, String> {
        let e = |e: nn_app::AssetError| e.to_string();
        Ok(match self {
            Self::Texture(path, options) => {
                vec![ctx.texture_handle(path, *options).map_err(e)?.1]
            }
            Self::Sound(path) => vec![ctx.sound_handle(path).map_err(e)?.1],
            Self::Model(path, flat) => vec![ctx.model_handle(path, *flat).map_err(e)?.1],
            Self::Font(path) => {
                ctx.load_font(path).map_err(e)?;
                Vec::new()
            }
            Self::Map(path) => crate::tilemap::preload(ctx, path)?,
        })
    }
}

/// A lista de uma cena já avaliada: `_assets()` (as cenas visuais) e o campo
/// `preload` (do script).
pub(crate) fn list_of(module: &Table) -> mlua::Result<Vec<Preload>> {
    let mut out: Vec<Preload> = Vec::new();
    let mut add = |list: Table| {
        for v in list.sequence_values::<Value>().flatten() {
            if let Some(p) = Preload::from_value(&v)
                && !out.contains(&p)
            {
                out.push(p);
            }
        }
    };
    if let Ok(f) = module.get::<Function>("_assets") {
        add(f.call::<Table>(module.clone())?);
    }
    if let Ok(Some(list)) = module.get::<Option<Table>>("preload") {
        add(list);
    }
    Ok(out)
}

/// O que a cena já carregou antes de começar (e segura enquanto existir).
#[derive(Debug, Default)]
pub(crate) struct Preloaded {
    pub items: Vec<Preload>,
    pub next: usize,
    pub handles: Vec<AssetHandle>,
}

impl Preloaded {
    pub fn has_more(&self) -> bool {
        self.next < self.items.len()
    }

    pub fn progress(&self) -> f32 {
        if self.items.is_empty() {
            1.0
        } else {
            self.next as f32 / self.items.len() as f32
        }
    }

    /// Carrega o próximo item. Um que não carrega (arquivo que não existe)
    /// fica para a cena, que mostra o erro onde ele for usado.
    pub fn load_next(&mut self, ctx: &mut Context) {
        let Some(item) = self.items.get(self.next) else {
            return;
        };
        self.next += 1;
        match item.load(ctx) {
            Ok(h) => self.handles.extend(h),
            Err(e) => log::debug!("preload: {e}"),
        }
    }
}

/// Quanto carregar por quadro numa tela de carregamento (o resto do quadro
/// desenha a tela e a barra anda).
const BUDGET: Duration = Duration::from_millis(12);

/// A tela de carregamento: carrega a cena nova um pouco a cada quadro,
/// mostrando o andamento, e troca para ela quando terminar.
pub(crate) struct LoadingScene {
    next: Option<LuaScene>,
    screen: Option<LuaScene>,
    runtime: crate::ScriptRuntime,
}

impl LoadingScene {
    pub(crate) fn new(next: LuaScene, screen: &LoadingScreen) -> Self {
        let screen = match screen {
            LoadingScreen::Default => None,
            LoadingScreen::Scene(path) => Some(next.runtime().scene(path.clone()).keep_on_enter()),
        };
        Self {
            runtime: next.runtime().clone(),
            next: Some(next),
            screen,
        }
    }

    fn set_progress(&self, value: Option<f32>) {
        if let Some(mut shared) = self.runtime.lua().app_data_mut::<Shared>() {
            shared.progress = value;
        }
    }

    fn progress(&self) -> f32 {
        self.next.as_ref().map_or(1.0, |n| n.preloaded().progress())
    }
}

impl Scene for LoadingScene {
    fn enter(&mut self, ctx: &mut Context) {
        if let Some(next) = &mut self.next {
            next.prepare(ctx);
        }
        self.set_progress(Some(self.progress()));
        if let Some(screen) = &mut self.screen {
            screen.enter(ctx);
        }
    }

    fn exit(&mut self, ctx: &mut Context) {
        if let Some(screen) = &mut self.screen {
            screen.exit(ctx);
        }
        self.set_progress(None);
    }

    fn fixed_update(&mut self, ctx: &mut Context) {
        if let Some(screen) = &mut self.screen {
            screen.fixed_update(ctx);
        }
    }

    fn update(&mut self, ctx: &mut Context) -> Transition {
        let start = Instant::now();
        if let Some(next) = &mut self.next {
            while next.preloaded().has_more() && start.elapsed() < BUDGET {
                next.preload_next(ctx);
            }
        }
        self.set_progress(Some(self.progress()));
        if let Some(screen) = &mut self.screen {
            // A tela não troca de cena: quem decide é o fim do carregamento.
            let _ = screen.update(ctx);
        }
        let done = self.next.as_ref().is_none_or(|n| !n.preloaded().has_more());
        match self.next.take() {
            Some(next) if done => Transition::Replace(Box::new(next)),
            other => {
                self.next = other;
                Transition::None
            }
        }
    }

    fn render(&mut self, ctx: &mut Context) {
        match &mut self.screen {
            Some(screen) => screen.render(ctx),
            None => draw_default(ctx, self.progress()),
        }
    }
}

/// A tela de carregamento padrão: "Carregando..." e uma barra.
fn draw_default(ctx: &mut Context, progress: f32) {
    let view = ctx.viewport().as_vec2();
    let k = (view.y / 450.0).clamp(0.5, 2.5);
    let w = (view.x * 0.5).max(40.0);
    let h = 10.0 * k;
    let at = Vec2::new((view.x - w) / 2.0, view.y / 2.0);
    let mut screen = ctx.renderer.screen();
    screen.draw_rect(Vec2::ZERO, view, Color::hex(0x141A26));
    screen.draw_text(
        &Text::new("Carregando...")
            .at(Vec2::new(view.x / 2.0, at.y - 14.0 * k))
            .size(22.0 * k)
            .color(Color::hex(0xC8D0E0))
            .anchor(Anchor::BottomCenter),
    );
    screen.draw_rect(at, Vec2::new(w, h), Color::hex(0x2A3446));
    screen.draw_rect(
        at,
        Vec2::new(w * progress.clamp(0.0, 1.0), h),
        Color::hex(0xE5A84D),
    );
}
