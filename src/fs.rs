//! Sandbox-backed file I/O commands. Registered by [`crate::App::with_fs_sandbox`].

use std::fs;
use std::io::Read;
use std::path::Path;

use base64::Engine;
use serde::Serialize;
use serde_json::Value;

use crate::sandbox::{
    atomic_write, is_dir_allowed, is_path_allowed, safe_lock, validate_path, SharedList,
    DEFAULT_MAX_ALLOWED_PATHS, DEFAULT_MAX_FILE_SIZE,
};

pub struct FsState {
    pub allowed_paths: SharedList,
    pub allowed_dirs: SharedList,
}

fn arg_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("missing arg: {}", key))
}

pub fn allow_path(state: &FsState, args: &Value) -> Result<Value, String> {
    let path = arg_str(args, "path")?;
    validate_path(path)?;
    let canonical = fs::canonicalize(path)
        .map_err(|_| "Invalid file path".to_string())?
        .to_string_lossy()
        .to_string();
    let mut paths = safe_lock(&state.allowed_paths);
    if paths.len() >= DEFAULT_MAX_ALLOWED_PATHS {
        return Err("Too many allowed paths".to_string());
    }
    if !paths.contains(&canonical) {
        paths.push(canonical);
    }
    Ok(Value::Null)
}

pub fn allow_dir(state: &FsState, args: &Value) -> Result<Value, String> {
    let path = arg_str(args, "path")?;
    validate_path(path)?;
    let canonical = fs::canonicalize(path)
        .map_err(|_| "Invalid directory path".to_string())?
        .to_string_lossy()
        .to_string();
    let mut dirs = safe_lock(&state.allowed_dirs);
    if dirs.len() >= DEFAULT_MAX_ALLOWED_PATHS {
        return Err("Too many allowed directories".to_string());
    }
    if !dirs.contains(&canonical) {
        dirs.push(canonical);
    }
    Ok(Value::Null)
}

#[derive(Serialize)]
struct DirEntry {
    name: String,
    path: String,
    is_dir: bool,
}

pub fn list_directory(state: &FsState, args: &Value) -> Result<Value, String> {
    let path = arg_str(args, "path")?;
    validate_path(path)?;
    is_dir_allowed(path, &state.allowed_dirs)?;

    let entries = fs::read_dir(path).map_err(|e| format!("Cannot read directory: {}", e))?;
    let mut result: Vec<DirEntry> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Cannot read entry: {}", e))?;
        let metadata = entry
            .metadata()
            .map_err(|e| format!("Cannot read metadata: {}", e))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        result.push(DirEntry {
            name,
            path: entry.path().to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
        });
    }
    serde_json::to_value(&result).map_err(|e| e.to_string())
}

pub fn read_file(state: &FsState, args: &Value) -> Result<Value, String> {
    let path = arg_str(args, "path")?;
    validate_path(path)?;
    let canonical = is_path_allowed(path, &state.allowed_paths, &state.allowed_dirs)?;

    let metadata = fs::metadata(&canonical).map_err(|_| "Cannot read file".to_string())?;
    if metadata.len() > DEFAULT_MAX_FILE_SIZE {
        return Err(format!(
            "File too large: {} bytes (max {})",
            metadata.len(),
            DEFAULT_MAX_FILE_SIZE
        ));
    }
    let mut buf = String::with_capacity(metadata.len() as usize);
    fs::File::open(&canonical)
        .map_err(|_| "Cannot read file".to_string())?
        .take(DEFAULT_MAX_FILE_SIZE + 1)
        .read_to_string(&mut buf)
        .map_err(|_| "Cannot read file".to_string())?;
    if buf.len() as u64 > DEFAULT_MAX_FILE_SIZE {
        return Err(format!(
            "File too large: exceeds {} bytes",
            DEFAULT_MAX_FILE_SIZE
        ));
    }
    Ok(Value::from(buf))
}

pub fn read_file_binary(state: &FsState, args: &Value) -> Result<Value, String> {
    let path = arg_str(args, "path")?;
    validate_path(path)?;
    let canonical = is_path_allowed(path, &state.allowed_paths, &state.allowed_dirs)?;

    let metadata = fs::metadata(&canonical).map_err(|_| "Cannot read file".to_string())?;
    if metadata.len() > DEFAULT_MAX_FILE_SIZE {
        return Err(format!(
            "File too large: {} bytes (max {})",
            metadata.len(),
            DEFAULT_MAX_FILE_SIZE
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    fs::File::open(&canonical)
        .map_err(|_| "Cannot read file".to_string())?
        .take(DEFAULT_MAX_FILE_SIZE + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read file".to_string())?;
    if bytes.len() as u64 > DEFAULT_MAX_FILE_SIZE {
        return Err(format!(
            "File too large: exceeds {} bytes",
            DEFAULT_MAX_FILE_SIZE
        ));
    }
    Ok(Value::from(
        base64::engine::general_purpose::STANDARD.encode(&bytes),
    ))
}

pub fn write_file(state: &FsState, args: &Value) -> Result<Value, String> {
    let path = arg_str(args, "path")?;
    let content = arg_str(args, "content")?;
    validate_path(path)?;
    let canonical = is_path_allowed(path, &state.allowed_paths, &state.allowed_dirs)?;
    if content.len() as u64 > DEFAULT_MAX_FILE_SIZE {
        return Err(format!(
            "Content too large: {} bytes (max {})",
            content.len(),
            DEFAULT_MAX_FILE_SIZE
        ));
    }
    atomic_write(Path::new(&canonical), content.as_bytes())?;
    Ok(Value::Null)
}

pub fn write_file_binary(state: &FsState, args: &Value) -> Result<Value, String> {
    let path = arg_str(args, "path")?;
    let data = arg_str(args, "data")?;
    validate_path(path)?;
    let parent = Path::new(path)
        .parent()
        .ok_or("Invalid file path")?
        .to_string_lossy()
        .to_string();
    let canonical_parent = is_dir_allowed(&parent, &state.allowed_dirs)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .map_err(|e| format!("base64 decode error: {}", e))?;
    if bytes.len() as u64 > DEFAULT_MAX_FILE_SIZE {
        return Err(format!(
            "File too large: {} bytes (max {})",
            bytes.len(),
            DEFAULT_MAX_FILE_SIZE
        ));
    }
    let filename = Path::new(path).file_name().ok_or("Invalid file name")?;
    let canonical_path = Path::new(&canonical_parent).join(filename);
    if let Ok(meta) = fs::symlink_metadata(&canonical_path) {
        if meta.file_type().is_symlink() {
            return Err("Write rejected: target is a symlink".to_string());
        }
    }
    atomic_write(&canonical_path, &bytes)?;
    let final_canonical = fs::canonicalize(&canonical_path)
        .map_err(|e| format!("Cannot resolve written file: {}", e))?;
    validate_path(&final_canonical.to_string_lossy())?;
    if !final_canonical.starts_with(Path::new(&canonical_parent)) {
        let _ = fs::remove_file(&canonical_path);
        return Err("Write rejected: symlink escape detected".to_string());
    }
    Ok(Value::Null)
}

pub fn ensure_dir(state: &FsState, args: &Value) -> Result<Value, String> {
    let path = arg_str(args, "path")?;
    validate_path(path)?;
    let canonical_parent = is_dir_allowed(path, &state.allowed_dirs).or_else(|_| {
        let parent = Path::new(path)
            .parent()
            .ok_or("Invalid path")?
            .to_string_lossy()
            .to_string();
        is_dir_allowed(&parent, &state.allowed_dirs)
    })?;
    let dir_name = Path::new(path)
        .file_name()
        .ok_or("Invalid directory name")?;
    let target = Path::new(&canonical_parent).join(dir_name);
    fs::create_dir_all(&target).map_err(|e| format!("Cannot create directory: {}", e))?;
    let canonical_target = fs::canonicalize(&target)
        .map_err(|e| format!("Cannot resolve created directory: {}", e))?;
    validate_path(&canonical_target.to_string_lossy())?;
    Ok(Value::Null)
}
