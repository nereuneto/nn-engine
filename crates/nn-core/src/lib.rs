//! Tipos fundamentais compartilhados por todos os módulos da nn-engine.

mod color;
mod time;

pub use color::Color;
pub use time::{FixedTimestep, Time};
