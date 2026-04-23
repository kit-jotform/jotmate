//! Library surface for the `jotmate` binary — makes modules reachable from
//! integration tests in `tests/`. The binary (`src/main.rs`) imports from
//! here rather than declaring its own module tree, so production and tests
//! share one definition of every type.

pub mod cli;
pub mod config;
pub mod ctx;
pub mod error;
pub mod sync;
pub mod time;
pub mod tui;
