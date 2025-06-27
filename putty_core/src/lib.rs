pub mod connections;
pub mod core;
pub mod storage;
pub mod utils;

// re‑export ergonomic entry point
pub use core::connection_manager::ConnectionManager;
pub use storage::{Profile, ProfileStore};
