//! NN Studio: o editor visual da nn-engine.
//!
//! Abre com o gerenciador de projetos (recentes, novo projeto a partir de um
//! modelo, exemplos). Com um projeto aberto, mostra os arquivos, o editor de
//! código Lua, o editor de pixel art e animação, visualizadores de som e
//! mapa, o console e o botão
//! ▶ Jogar, que roda o jogo na própria janela dele.

pub mod agent;
pub use nn_projeto::{
    anim as anim_file, map as map_doc, particles as particle_presets, pixel as pixel_ops, tileset,
};
pub use nn_projeto::{behaviors, config_lua, events, lua_syntax, scene_data};
pub mod app;
pub mod catalog;
pub mod code;
pub mod commands;
pub mod complete;
pub mod console;
pub mod crash;
pub mod dialogue;
pub mod events_view;
pub mod examples;
pub mod file_panel;
pub mod files;
pub mod game;
pub mod hierarchy;
pub mod inspector;
pub mod launcher;
pub mod map_editor;
pub mod model_data;
pub mod model_editor;
pub mod model_kits;
pub mod model_rig;
pub mod new_map;
pub mod paint;
pub mod pixel_editor;
pub mod project;
pub mod repeat;
pub mod scene_render;
pub mod scene_view;
pub mod settings;
pub mod terrain_tool;
pub mod theme;
pub mod tiles_view;
pub mod tips;
pub mod toast;
pub mod tutorial;
pub mod ui_sounds;
pub mod update;
pub mod viewers;
