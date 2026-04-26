//! Library surface so integration tests in `tests/` and `src/main.rs` share
//! one module tree.

pub mod cli;
pub mod config;
pub mod ctx;
pub mod error;
pub mod sync;
pub mod time;
pub mod tui;
pub mod update;
