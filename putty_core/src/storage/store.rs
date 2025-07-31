//! A very small profile store
//!
//! * Each SSH profile keeps its secret in the system key-ring under the single
//!   **service** “`putty_rs`” and user  **`putty_rs:<profile-name>`**..
//! * Serial profiles contain no secret.

use std::{fs, io, path::Path, path::PathBuf};

use directories::ProjectDirs;
use keyring::{Entry, Error as KrError};
use log::{debug, warn};
use serde_json::Error as SerdeError;

use super::profile::Profile;

/// Small wrapper that stores JSON files on disk **and** secrets in the key-ring.
#[derive(Debug, Clone)]
pub struct ProfileStore {
    dir: PathBuf,
}

/* ───────────────────────────────────────────────────────── helpers ── */

/// canonical key-ring user name:  `putty_rs:<profile-name>`
fn key_id(name: &str) -> String {
    format!("putty_rs:{name}")
}

/// open key-ring entry (logs every access so we see what happens)
fn open_entry(id: &str) -> io::Result<Entry> {
    debug!("key-ring open  service='putty_rs'  user='{id}'");
    Entry::new("putty_rs", id).map_err(io::Error::other)
}

/// compute `<config>/profiles/<name>.json`
fn json_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.json"))
}

/* ─────────────────────────────────────────────── public impl ─────── */

impl ProfileStore {
    /// Locate (or create) the *profiles* directory under the user’s config dir.
    pub fn new() -> io::Result<Self> {
        let dir = ProjectDirs::from("", "", "putty_rs")
            .ok_or_else(|| io::Error::other("no config dir"))?
            .config_dir()
            .join("profiles");
        fs::create_dir_all(&dir)?;
        debug!("profile store dir = {dir:?}");
        Ok(Self { dir })
    }

    /* --------------------------- save -------------------------------- */
    ///
    /// * Serial → copied 1:1 to JSON  
    /// * SSH    → secret put in key-ring, redacted JSON on disk
    pub fn save(&self, profile: &Profile) -> io::Result<()> {
        debug!("save {}", profile.name());

        let sanitized = match profile {
            Profile::Serial { .. } => profile.clone(),

            Profile::Ssh {
                name,
                host,
                port,
                username,
                password,
                ..
            } => {
                let id = key_id(name);
                debug!("write secret len={}  to id='{id}'", password.len());

                if !password.is_empty() {
                    open_entry(&id)?
                        .set_password(password)
                        .map_err(io::Error::other)?;
                }

                // clone WITHOUT the secret
                Profile::Ssh {
                    name: name.clone(),
                    host: host.clone(),
                    port: *port,
                    username: username.clone(),
                    password: String::new(),
                    keyring_id: Some(id),
                }
            }
        };

        serde_json::to_writer_pretty(
            fs::File::create(json_path(&self.dir, profile.name()))?,
            &sanitized,
        )
        .map_err(SerdeError::into)
    }

    /* --------------------------- list ------------------------------- */
    ///
    /// Loads every JSON file; SSH profiles get their secret filled in
    /// from the key-ring (if present).
    pub fn list(&self) -> io::Result<Vec<Profile>> {
        debug!("KEYRING_BACKEND = {:?}", std::env::var("KEYRING_BACKEND"));
        let mut out = Vec::new();

        for entry in fs::read_dir(&self.dir)? {
            let path = entry?.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }

            match fs::File::open(&path)
                .and_then(|f| serde_json::from_reader::<_, Profile>(f).map_err(SerdeError::into))
            {
                Ok(mut profile) => {
                    if let Profile::Ssh {
                        password,
                        keyring_id,
                        name,
                        ..
                    } = &mut profile
                    {
                        if password.is_empty() {
                            let id = keyring_id.clone().unwrap_or_else(|| key_id(name));
                            match open_entry(&id)?.get_password() {
                                Ok(sec) => {
                                    *password = sec.trim_end_matches(['\r', '\n']).to_owned();
                                }
                                Err(KrError::NoEntry) => {
                                    debug!("no secret stored under id='{id}'");
                                }
                                Err(e) => warn!("key-ring read error: {e}"),
                            }
                        }
                    }
                    out.push(profile);
                }
                Err(e) => warn!("bad profile {path:?}: {e}"),
            }
        }
        Ok(out)
    }

    /* --------------------------- delete ----------------------------- */
    ///
    /// Removes the JSON file **and** the associated key-ring secret.
    pub fn delete(&self, name: &str) -> io::Result<bool> {
        let id = key_id(name);
        let _ = open_entry(&id)?.delete_credential(); // ignore NoEntry

        match fs::remove_file(json_path(&self.dir, name)) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }

    pub fn dir(&self) -> &PathBuf {
        &self.dir
    }
}
