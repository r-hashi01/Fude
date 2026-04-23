//! Path sandbox primitives: an allow-list model where the user grants access
//! to files/directories via native dialogs, and read/write ops refuse to
//! touch anything outside that list. Also blocks well-known credential
//! locations (~/.ssh, ~/.aws, etc.) regardless of allow-list.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

pub const DEFAULT_MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
pub const DEFAULT_MAX_ALLOWED_PATHS: usize = 10_000;

/// Shared, mutex-protected list of absolute canonical paths.
pub type SharedList = Arc<Mutex<Vec<String>>>;

pub fn new_list() -> SharedList {
    Arc::new(Mutex::new(Vec::new()))
}

pub fn safe_lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

fn starts_with_any(path: &Path, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|p| path.starts_with(p))
}

fn has_blocked_component(path: &Path, blocked: &[&str]) -> bool {
    let path_str = path.to_string_lossy();
    for b in blocked {
        if b.contains('/') {
            if path_str.contains(b) {
                return true;
            }
        } else {
            for component in path.components() {
                if let std::path::Component::Normal(name) = component {
                    if name.eq_ignore_ascii_case(b) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Reject paths that point at system directories or credential stores.
/// Applied before *any* allow-list check so a compromised frontend can't
/// whitelist /etc or ~/.ssh.
pub fn validate_path(path: &str) -> Result<(), String> {
    let p = Path::new(path);
    if !p.is_absolute() {
        return Err("Only absolute paths are allowed".to_string());
    }

    let blocked_dirs: &[&str] = &[
        "/etc",
        "/var",
        "/usr",
        "/sys",
        "/proc",
        "/sbin",
        "/bin",
        "/boot",
        "/private/etc",
        "/private/var",
        "/private/tmp",
        "/Library",
    ];
    if starts_with_any(p, blocked_dirs) {
        return Err("Access to system directories is not allowed".to_string());
    }

    let blocked_components: &[&str] = &[
        ".ssh",
        ".gnupg",
        ".gpg",
        ".aws",
        ".kube",
        ".docker",
        ".config/gcloud",
        "Keychains",
        ".git",
        ".npmrc",
        ".netrc",
    ];
    if has_blocked_component(p, blocked_components) {
        return Err("Access to sensitive directories is not allowed".to_string());
    }

    if let Ok(canonical) = fs::canonicalize(path) {
        if starts_with_any(&canonical, blocked_dirs) {
            return Err("Access to system directories is not allowed".to_string());
        }
        if has_blocked_component(&canonical, blocked_components) {
            return Err("Access to sensitive directories is not allowed".to_string());
        }
    }
    Ok(())
}

pub fn is_dir_allowed(path: &str, allowed_dirs: &SharedList) -> Result<String, String> {
    let canonical = fs::canonicalize(path).map_err(|_| "Invalid directory path".to_string())?;
    let dirs = safe_lock(allowed_dirs);
    if dirs
        .iter()
        .any(|allowed| canonical.starts_with(Path::new(allowed)))
    {
        Ok(canonical.to_string_lossy().to_string())
    } else {
        Err("Access denied: directory not selected via dialog".to_string())
    }
}

/// Create `<data_dir>/<name>` (and parents) and add its canonical path to
/// `allowed_dirs`. Idempotent. `name` must be a simple directory name —
/// it cannot contain path separators, `..`, or start with `.`.
///
/// Returns the canonical scratch path. Typically called via
/// [`crate::ensure_scratch_dir`].
pub fn ensure_scratch(
    data_dir: &Path,
    allowed_dirs: &SharedList,
    name: &str,
) -> Result<PathBuf, String> {
    if name.is_empty() {
        return Err("scratch dir name is empty".to_string());
    }
    if name.contains('/') || name.contains('\\') {
        return Err("scratch dir name must not contain path separators".to_string());
    }
    if name.contains("..") || name.starts_with('.') {
        return Err("scratch dir name must not contain '..' or start with '.'".to_string());
    }
    let dir = data_dir.join(name);
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot create scratch dir: {}", e))?;
    let canonical =
        fs::canonicalize(&dir).map_err(|e| format!("Cannot canonicalize scratch dir: {}", e))?;
    let canonical_str = canonical.to_string_lossy().to_string();
    let mut dirs = safe_lock(allowed_dirs);
    if !dirs.contains(&canonical_str) {
        dirs.push(canonical_str);
    }
    Ok(canonical)
}

pub fn is_path_allowed(
    path: &str,
    allowed_paths: &SharedList,
    allowed_dirs: &SharedList,
) -> Result<String, String> {
    let canonical = fs::canonicalize(path).map_err(|_| "Invalid file path".to_string())?;
    let canonical_str = canonical.to_string_lossy().to_string();

    let paths = safe_lock(allowed_paths);
    if paths.contains(&canonical_str) {
        return Ok(canonical_str);
    }
    drop(paths);

    let dirs = safe_lock(allowed_dirs);
    if dirs
        .iter()
        .any(|allowed| canonical.starts_with(Path::new(allowed)))
    {
        return Ok(canonical_str);
    }
    Err("Access denied: file not selected via dialog".to_string())
}

/// Write `data` to `target` atomically via a sibling temp file + rename.
pub fn atomic_write(target: &Path, data: &[u8]) -> Result<(), String> {
    let parent = target.parent().ok_or("Invalid file path")?;
    let base = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let pid = std::process::id();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp_name = format!(".{}.{}.{}.tmp", base, pid, ts);
    let tmp_path = parent.join(&tmp_name);
    fs::write(&tmp_path, data).map_err(|e| format!("Cannot write temp file: {}", e))?;
    fs::rename(&tmp_path, target).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("Cannot rename temp file: {}", e)
    })
}

/// Resolve `~/Library/Application Support/<identifier>` on macOS.
/// Falls back to `$XDG_CONFIG_HOME/<identifier>` or `~/.config/<identifier>`
/// on Linux; `%APPDATA%\<identifier>` on Windows.
pub fn app_config_dir(identifier: &str) -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
        Ok(PathBuf::from(home)
            .join("Library/Application Support")
            .join(identifier))
    }
    #[cfg(target_os = "linux")]
    {
        let base = std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".config"))
            })
            .ok_or_else(|| "HOME not set".to_string())?;
        Ok(base.join(identifier))
    }
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").map_err(|_| "APPDATA not set".to_string())?;
        Ok(PathBuf::from(appdata).join(identifier))
    }
}

pub fn app_data_dir(identifier: &str) -> Result<PathBuf, String> {
    // On all three platforms, Tauri points app_data_dir at the same path as
    // app_config_dir (roaming config) — we mirror that.
    app_config_dir(identifier)
}
