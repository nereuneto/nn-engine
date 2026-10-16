//! NN Studio: o editor visual da nn-engine.
//!
//! Abre com o gerenciador de projetos (recentes, novo projeto a partir de um
//! modelo, exemplos). Com um projeto aberto, mostra os arquivos, o editor de
//! código Lua, visualizadores de imagem, som e mapa, o console e o botão
//! ▶ Jogar, que roda o jogo na própria janela dele.

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
pub mod project;
pub mod scene_data;
pub mod scene_render;
pub mod scene_view;
pub mod settings;
pub mod theme;
pub mod viewers;
