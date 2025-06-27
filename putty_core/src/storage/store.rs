use std::{fs, io, path::PathBuf};

use directories::ProjectDirs;
use serde_json::Error as SerdeError;

use super::profile::Profile;

#[derive(Debug, Clone)]
pub struct ProfileStore {
    dir: PathBuf,
}

impl ProfileStore {
    /// `~/.config/putty_rs/profiles` on Linux, `%APPDATA%\putty_rs\profiles` on Windows, etc.
    pub fn new() -> io::Result<Self> {
        let proj = ProjectDirs::from("", "", "putty_rs")
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Unable to locate config dir"))?;
        let dir = proj.config_dir().join("profiles");
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    fn file_for(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.json"))
    }

    /// Returns every stored profile (silently skips malformed files).
    pub fn list(&self) -> io::Result<Vec<Profile>> {
        let mut out = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let path = entry?.path();
            if !path.extension().is_some_and(|e| e == "json") {
                continue;
            }
            match fs::File::open(&path)
                .and_then(|f| serde_json::from_reader(f).map_err(SerdeError::into))
            {
                Ok(profile) => out.push(profile),
                Err(e) => eprintln!("Warning: could not read {:?}: {e}", path),
            }
        }
        Ok(out)
    }

    /// Create or overwrite a profile.
    pub fn save(&self, profile: &Profile) -> io::Result<()> {
        let file = fs::File::create(self.file_for(profile.name()))?;
        serde_json::to_writer_pretty(file, profile).map_err(SerdeError::into)
    }

    /// Delete a preset (`Ok(true)` if removed, `Ok(false)` if it didn’t exist).
    pub fn delete(&self, name: &str) -> io::Result<bool> {
        match fs::remove_file(self.file_for(name)) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }
}
