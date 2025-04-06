pub mod connection;
pub mod errors;
pub mod serial;
pub mod ssh;

// Re-export the modules here for easy import elsewhere.
pub use connection::*;
pub use errors::*;
