use serde::{Deserialize, Serialize};

/// A user-named connection preset.
///
/// The enum is `#[serde(tag = "kind")]` so JSON looks like:
/// `{ "name":"dbg", "kind":"Serial", "port":"/dev/ttyUSB0", "baud":115200 }`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Profile {
    Serial {
        name: String,
        port: String,
        baud: u32,
    },
    Ssh {
        name: String,
        host: String,
        port: u16,
        username: String,
        password: String,
    },
}

impl Profile {
    /// Returns the unique, human-readable identifier.
    pub fn name(&self) -> &str {
        match self {
            Profile::Serial { name, .. } => name,
            Profile::Ssh { name, .. } => name,
        }
    }
}
