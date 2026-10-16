//! NN Studio: o editor visual da nn-engine.
//!
//! Abre com o gerenciador de projetos (recentes, novo projeto a partir de um
//! modelo, exemplos). Com um projeto aberto, mostra os arquivos, o editor de
//! código Lua, o editor de pixel art e animação, visualizadores de som e
//! mapa, o console e o botão
//! ▶ Jogar, que roda o jogo na própria janela dele.

pub mod anim_file;
pub mod app;
pub mod behaviors;
pub mod code;
pub mod commands;
pub mod complete;
pub mod config_lua;
pub mod console;
pub mod events;
pub mod events_view;
pub mod examples;
pub mod file_panel;
pub mod files;
pub mod game;
pub mod hierarchy;
pub mod inspector;
pub mod launcher;
pub mod lua_syntax;
pub mod map_doc;
pub mod map_editor;
pub mod new_map;
pub mod paint;
pub mod pixel_editor;
pub mod pixel_ops;
pub mod project;
pub mod scene_data;
pub mod scene_render;
pub mod scene_view;
pub mod settings;
pub mod theme;
pub mod tiles_view;
pub mod tileset;
pub mod tutorial;
pub mod viewers;
