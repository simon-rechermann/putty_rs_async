pub mod connections;
pub mod core;
pub mod utils;

// re‑export ergonomic entry points
pub use core::connection_manager::{ConnectionHandle, ConnectionManager};
