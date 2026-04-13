//! Round-trip test for `ProfileStore` with whatever key-ring back-end
//! `keyring` selects by default. We sandbox the JSON side by pointing
//! `XDG_CONFIG_HOME` to a temporary directory, so existing user profiles
//! never interfere.

use std::{fs, path::PathBuf};

use keyring::{Entry, Error as KrError};
use putty_storage::{Profile, ProfileStore};
use serde_json::Value;
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn profile_store_roundtrip_default_backend() -> anyhow::Result<()> {
    let sandbox = TempDir::new()?;
    let profiles_dir = sandbox.path().join("profiles");
    let store = ProfileStore::in_dir(profiles_dir.clone())?;

    let profile_name = format!("probe-{}", Uuid::new_v4());
    let key_id = format!("putty_rs:{profile_name}");
    let pw = "s3cr3t!";

    store.save(&Profile::Ssh {
        name: profile_name.clone(),
        host: "host".into(),
        port: 22,
        username: "user".into(),
        password: pw.into(),
        keyring_id: None,
    })?;

    let json_path: PathBuf = profiles_dir.join(format!("{profile_name}.json"));
    let doc: Value = serde_json::from_str(&fs::read_to_string(&json_path)?)?;
    match &doc["password"] {
        Value::String(s) => assert!(s.is_empty(), "password leaked into JSON"),
        Value::Null => {}
        other => panic!("unexpected JSON value {other:?}"),
    }

    assert_eq!(Entry::new("putty_rs", &key_id)?.get_password()?, pw);

    let profiles_vec = store.list()?;
    let restored_pw = match &profiles_vec[..] {
        [Profile::Ssh { password, .. }] => password,
        other => panic!("unexpected list content {other:?}"),
    };
    assert_eq!(restored_pw, pw);

    assert!(store.delete(&profile_name)?);
    assert!(store.list()?.is_empty());
    assert!(!json_path.exists());
    assert!(matches!(
        Entry::new("putty_rs", &key_id)?.get_password(),
        Err(KrError::NoEntry)
    ));

    Ok(())
}
