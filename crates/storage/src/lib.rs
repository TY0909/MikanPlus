//! Local persistence, cache, migration, and platform path capabilities.

pub mod cache;
pub mod migrate;
pub mod paths;
mod state;

pub use state::State;
