pub mod connection;
pub mod errors;
#[cfg(feature = "serial")]
pub mod serial;
#[cfg(feature = "ssh")]
pub mod ssh;

// Re-export the modules here for easy import elsewhere.
pub use connection::*;
pub use errors::*;
