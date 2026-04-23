//! Opens URLs or allow-listed files in the OS default application.
//! Registered by [`crate::App::with_shell_open`].
//!
//! Two classes of target are accepted:
//!
//! - URLs with scheme `http://`, `https://`, or `mailto:` — passed to the
//!   OS opener as-is.
//! - Absolute local file paths — must already be allow-listed via
//!   `allow_path` or `allow_dir` before `shell_open` will accept them.
//!
//! Any other scheme (`file://`, `javascript:`, `vbscript:`, custom
//! schemes) is refused — the goal is to prevent a compromised frontend
//! from tricking the shell into running arbitrary handlers or reading
//! paths outside the sandbox.

use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

use crate::{is_path_allowed, FsState};

#[cfg_attr(test, derive(Debug))]
pub(crate) enum Target {
    Url(String),
    Path(PathBuf),
}

pub(crate) fn classify(input: &str, fs: Option<&FsState>) -> Result<Target, String> {
    if input.is_empty() {
        return Err("shell_open target is empty".to_string());
    }
    let lower = input.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:")
    {
        return Ok(Target::Url(input.to_string()));
    }
    if lower.contains("://") || lower.starts_with("javascript:") || lower.starts_with("data:") {
        return Err(format!(
            "shell_open refused scheme in target: {}",
            input.split_once(':').map(|(s, _)| s).unwrap_or("unknown")
        ));
    }
    let fs = fs.ok_or_else(|| {
        "shell_open for file paths requires with_fs_sandbox on the App".to_string()
    })?;
    let canonical = is_path_allowed(input, &fs.allowed_paths, &fs.allowed_dirs)?;
    Ok(Target::Path(PathBuf::from(canonical)))
}

pub(crate) fn open(args: &Value, fs: Option<&FsState>) -> Result<Value, String> {
    let target = args
        .get("target")
        .and_then(|v| v.as_str())
        .ok_or("missing target")?;
    let classified = classify(target, fs)?;
    let arg = match classified {
        Target::Url(u) => u,
        Target::Path(p) => p.to_string_lossy().to_string(),
    };
    spawn_opener(&arg)
}

#[cfg(target_os = "macos")]
fn spawn_opener(arg: &str) -> Result<Value, String> {
    Command::new("open")
        .arg(arg)
        .spawn()
        .map_err(|e| format!("open failed: {}", e))?;
    Ok(Value::Null)
}

#[cfg(target_os = "linux")]
fn spawn_opener(arg: &str) -> Result<Value, String> {
    Command::new("xdg-open")
        .arg(arg)
        .spawn()
        .map_err(|e| format!("xdg-open failed: {}", e))?;
    Ok(Value::Null)
}

#[cfg(target_os = "windows")]
fn spawn_opener(arg: &str) -> Result<Value, String> {
    Command::new("cmd")
        .args(["/C", "start", "", arg])
        .spawn()
        .map_err(|e| format!("start failed: {}", e))?;
    Ok(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{new_list, FsState};
    use std::sync::Arc;

    fn empty_fs() -> Arc<FsState> {
        Arc::new(FsState {
            allowed_paths: new_list(),
            allowed_dirs: new_list(),
        })
    }

    #[test]
    fn empty_input_rejected() {
        assert!(classify("", None).is_err());
    }

    #[test]
    fn http_url_passes_without_fs() {
        let r = classify("http://example.com", None).unwrap();
        assert!(matches!(r, Target::Url(_)));
    }

    #[test]
    fn https_url_passes_without_fs() {
        assert!(matches!(
            classify("https://example.com/foo?bar", None).unwrap(),
            Target::Url(_)
        ));
    }

    #[test]
    fn mailto_passes_without_fs() {
        assert!(matches!(
            classify("mailto:alice@example.com", None).unwrap(),
            Target::Url(_)
        ));
    }

    #[test]
    fn scheme_casing_is_ignored() {
        assert!(matches!(
            classify("HTTPS://example.com", None).unwrap(),
            Target::Url(_)
        ));
    }

    #[test]
    fn javascript_scheme_rejected() {
        assert!(classify("javascript:alert(1)", None).is_err());
    }

    #[test]
    fn data_scheme_rejected() {
        assert!(classify("data:text/html,<script>", None).is_err());
    }

    #[test]
    fn file_scheme_rejected() {
        assert!(classify("file:///etc/passwd", None).is_err());
    }

    #[test]
    fn custom_scheme_rejected() {
        assert!(classify("vscode://path/to/file", None).is_err());
    }

    #[test]
    fn file_path_requires_fs_sandbox() {
        let err = classify("/tmp/x.md", None).unwrap_err();
        assert!(err.contains("with_fs_sandbox"));
    }

    #[test]
    fn file_path_not_on_allow_list_rejected() {
        let fs = empty_fs();
        assert!(classify("/etc/passwd", Some(&fs)).is_err());
    }
}
