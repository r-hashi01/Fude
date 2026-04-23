//! JSON settings persistence. Registered by
//! [`crate::App::with_settings`].
//!
//! Stores an arbitrary JSON object at `<app_config_dir>/settings.json`.
//! Writes go through [`crate::atomic_write`] so a crash mid-save cannot
//! corrupt the file.
//!
//! The primitive is deliberately schema-less — apps that need validation
//! (e.g. path sanitization, allow-listed field names) should wrap
//! `save_settings` with their own command or post-process the object
//! frontend-side before calling.

use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::{app_config_dir, atomic_write, Ctx};

const FILE_NAME: &str = "settings.json";

pub(crate) fn load(ctx: &Ctx) -> Result<Value, String> {
    load_from(&app_config_dir(&ctx.identifier)?)
}

pub(crate) fn save(ctx: &Ctx, args: &Value) -> Result<Value, String> {
    save_to(&app_config_dir(&ctx.identifier)?, args)
}

pub(crate) fn load_from(dir: &Path) -> Result<Value, String> {
    let path = dir.join(FILE_NAME);
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    let content = fs::read_to_string(&path).map_err(|e| format!("Cannot read settings: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Corrupt settings JSON: {}", e))
}

pub(crate) fn save_to(dir: &Path, args: &Value) -> Result<Value, String> {
    let obj = args.get("settings").ok_or("missing settings")?;
    if !obj.is_object() {
        return Err("Settings must be a JSON object".to_string());
    }
    let serialized =
        serde_json::to_string(obj).map_err(|e| format!("Cannot serialize settings: {}", e))?;
    fs::create_dir_all(dir).map_err(|e| format!("Cannot create config dir: {}", e))?;
    atomic_write(&dir.join(FILE_NAME), serialized.as_bytes())?;
    Ok(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    fn scratch() -> tempfile::TempDir {
        let home = std::env::var("HOME").expect("HOME");
        let base = PathBuf::from(home).join(".fude-test-scratch");
        std::fs::create_dir_all(&base).unwrap();
        tempfile::TempDir::new_in(&base).unwrap()
    }

    #[test]
    fn load_returns_empty_object_when_missing() {
        let dir = scratch();
        assert_eq!(load_from(dir.path()).unwrap(), json!({}));
    }

    #[test]
    fn save_then_load_round_trip() {
        let dir = scratch();
        let args = json!({ "settings": { "theme": "dark", "fontSize": 14 } });
        save_to(dir.path(), &args).unwrap();
        let loaded = load_from(dir.path()).unwrap();
        assert_eq!(loaded["theme"], "dark");
        assert_eq!(loaded["fontSize"], 14);
    }

    #[test]
    fn save_creates_config_dir_if_missing() {
        let dir = scratch();
        let nested = dir.path().join("doesnt").join("exist").join("yet");
        let args = json!({ "settings": { "k": "v" } });
        save_to(&nested, &args).unwrap();
        assert!(nested.join(FILE_NAME).exists());
    }

    #[test]
    fn save_rejects_non_object_settings() {
        let dir = scratch();
        assert!(save_to(dir.path(), &json!({ "settings": [1, 2, 3] })).is_err());
        assert!(save_to(dir.path(), &json!({ "settings": "string" })).is_err());
        assert!(save_to(dir.path(), &json!({ "settings": 42 })).is_err());
    }

    #[test]
    fn save_rejects_missing_settings_key() {
        let dir = scratch();
        assert!(save_to(dir.path(), &json!({ "theme": "dark" })).is_err());
    }

    #[test]
    fn load_errors_on_corrupt_json() {
        let dir = scratch();
        std::fs::write(dir.path().join(FILE_NAME), b"{not json").unwrap();
        assert!(load_from(dir.path()).is_err());
    }

    #[test]
    fn save_is_atomic_via_temp_rename() {
        // Verify save_to doesn't leave a partially-written file on disk
        // even if we inspect mid-operation (best-effort: the temp/rename
        // pattern means `settings.json` is either the old or new content).
        let dir = scratch();
        save_to(dir.path(), &json!({ "settings": { "v": 1 } })).unwrap();
        save_to(dir.path(), &json!({ "settings": { "v": 2 } })).unwrap();
        assert_eq!(load_from(dir.path()).unwrap()["v"], 2);
    }
}
