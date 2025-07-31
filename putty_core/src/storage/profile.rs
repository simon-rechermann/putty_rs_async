use serde::{Deserialize, Serialize};

/// A user-named connection preset.
///
/// For SSH, the password is **never** serialized; instead we keep a reference
/// (`keyring_id`) to the OS keyring entry that holds the secret.
///
/// `{ "kind":"Ssh", "name":"prodbox", "host":"10.0.0.5", ...,
///    "keyring_id":"putty_rs:prodbox" }`
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
        #[serde(default, skip_serializing)]
        password: String, // do not save this in json
        keyring_id: Option<String>,
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
