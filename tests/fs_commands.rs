//! Integration tests for the sandbox-backed FS command surface
//! (`fude::fs::allow_path`, `read_file`, `write_file`, …).

mod common;

use std::fs;

use base64::Engine;
use fude::fs::{
    allow_dir, allow_path, ensure_dir, list_directory, read_file, read_file_binary, write_file,
    write_file_binary, FsState,
};
use fude::{new_list, safe_lock};
use serde_json::json;

use crate::common::scratch_dir;

fn fresh_state() -> FsState {
    FsState {
        allowed_paths: new_list(),
        allowed_dirs: new_list(),
    }
}

// --- allow_path / allow_dir --------------------------------------------

#[test]
fn allow_path_rejects_relative() {
    let s = fresh_state();
    let err = allow_path(&s, &json!({ "path": "relative.md" })).unwrap_err();
    assert!(err.contains("absolute"), "got: {err}");
}

#[test]
fn allow_path_rejects_blocked_system_path() {
    let s = fresh_state();
    assert!(allow_path(&s, &json!({ "path": "/etc/passwd" })).is_err());
}

#[test]
fn allow_path_deduplicates() {
    let dir = scratch_dir();
    let file = dir.path().join("a.md");
    fs::write(&file, b"x").unwrap();
    let s = fresh_state();
    allow_path(&s, &json!({ "path": file.to_string_lossy() })).unwrap();
    allow_path(&s, &json!({ "path": file.to_string_lossy() })).unwrap();
    assert_eq!(safe_lock(&s.allowed_paths).len(), 1);
}

#[test]
fn allow_path_missing_arg() {
    let s = fresh_state();
    assert!(allow_path(&s, &json!({})).is_err());
}

#[test]
fn allow_dir_adds_canonical_path() {
    let dir = scratch_dir();
    let s = fresh_state();
    allow_dir(&s, &json!({ "path": dir.path().to_string_lossy() })).unwrap();
    let list = safe_lock(&s.allowed_dirs);
    assert_eq!(list.len(), 1);
    let canonical = fs::canonicalize(dir.path()).unwrap();
    assert_eq!(list[0], canonical.to_string_lossy().to_string());
}

#[test]
fn allow_dir_deduplicates() {
    let dir = scratch_dir();
    let s = fresh_state();
    allow_dir(&s, &json!({ "path": dir.path().to_string_lossy() })).unwrap();
    allow_dir(&s, &json!({ "path": dir.path().to_string_lossy() })).unwrap();
    assert_eq!(safe_lock(&s.allowed_dirs).len(), 1);
}

// --- list_directory ----------------------------------------------------

#[test]
fn list_directory_requires_allow_list() {
    let dir = scratch_dir();
    fs::write(dir.path().join("a.md"), b"x").unwrap();
    let s = fresh_state();
    assert!(list_directory(&s, &json!({ "path": dir.path().to_string_lossy() })).is_err());
}

#[test]
fn list_directory_lists_entries_and_hides_dotfiles() {
    let dir = scratch_dir();
    fs::write(dir.path().join("visible.md"), b"x").unwrap();
    fs::write(dir.path().join(".hidden"), b"x").unwrap();
    fs::create_dir(dir.path().join("subdir")).unwrap();

    let s = fresh_state();
    allow_dir(&s, &json!({ "path": dir.path().to_string_lossy() })).unwrap();

    let value = list_directory(&s, &json!({ "path": dir.path().to_string_lossy() })).unwrap();
    let entries = value.as_array().unwrap();
    let names: Vec<String> = entries
        .iter()
        .map(|e| e["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"visible.md".to_string()));
    assert!(names.contains(&"subdir".to_string()));
    assert!(!names.iter().any(|n| n == ".hidden"));

    let subdir = entries.iter().find(|e| e["name"] == "subdir").unwrap();
    assert_eq!(subdir["is_dir"], json!(true));
}

// --- read_file ---------------------------------------------------------

#[test]
fn read_file_roundtrip() {
    let dir = scratch_dir();
    let file = dir.path().join("a.md");
    fs::write(&file, b"hello world").unwrap();

    let s = fresh_state();
    allow_path(&s, &json!({ "path": file.to_string_lossy() })).unwrap();
    let got = read_file(&s, &json!({ "path": file.to_string_lossy() })).unwrap();
    assert_eq!(got.as_str().unwrap(), "hello world");
}

#[test]
fn read_file_denied_without_allow() {
    let dir = scratch_dir();
    let file = dir.path().join("a.md");
    fs::write(&file, b"x").unwrap();
    let s = fresh_state();
    assert!(read_file(&s, &json!({ "path": file.to_string_lossy() })).is_err());
}

#[test]
fn read_file_rejects_oversize() {
    let dir = scratch_dir();
    let file = dir.path().join("big.bin");
    // 10 MiB + 1 byte; DEFAULT_MAX_FILE_SIZE is 10 MiB.
    let data = vec![b'a'; (10 * 1024 * 1024) + 1];
    fs::write(&file, &data).unwrap();
    let s = fresh_state();
    allow_path(&s, &json!({ "path": file.to_string_lossy() })).unwrap();
    assert!(read_file(&s, &json!({ "path": file.to_string_lossy() })).is_err());
}

#[test]
fn read_file_binary_round_trip() {
    let dir = scratch_dir();
    let file = dir.path().join("bin");
    let raw = vec![0u8, 1, 2, 0xff, 0xfe];
    fs::write(&file, &raw).unwrap();
    let s = fresh_state();
    allow_path(&s, &json!({ "path": file.to_string_lossy() })).unwrap();
    let got = read_file_binary(&s, &json!({ "path": file.to_string_lossy() })).unwrap();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(got.as_str().unwrap())
        .unwrap();
    assert_eq!(decoded, raw);
}

// --- write_file --------------------------------------------------------

#[test]
fn write_file_round_trip() {
    let dir = scratch_dir();
    let file = dir.path().join("out.md");
    fs::write(&file, b"").unwrap();

    let s = fresh_state();
    allow_path(&s, &json!({ "path": file.to_string_lossy() })).unwrap();
    write_file(
        &s,
        &json!({ "path": file.to_string_lossy(), "content": "new content" }),
    )
    .unwrap();
    assert_eq!(fs::read_to_string(&file).unwrap(), "new content");
}

#[test]
fn write_file_denied_without_allow() {
    let dir = scratch_dir();
    let file = dir.path().join("out.md");
    fs::write(&file, b"").unwrap();
    let s = fresh_state();
    let err = write_file(
        &s,
        &json!({ "path": file.to_string_lossy(), "content": "x" }),
    )
    .unwrap_err();
    assert!(err.contains("Access denied"), "got: {err}");
}

#[test]
fn write_file_missing_args() {
    let s = fresh_state();
    assert!(write_file(&s, &json!({})).is_err());
    assert!(write_file(&s, &json!({ "path": "/tmp/x" })).is_err()); // no content
}

// --- write_file_binary -------------------------------------------------

#[test]
fn write_file_binary_round_trip() {
    let dir = scratch_dir();
    let s = fresh_state();
    allow_dir(&s, &json!({ "path": dir.path().to_string_lossy() })).unwrap();
    let raw = vec![0u8, 1, 2, 3, 255];
    let encoded = base64::engine::general_purpose::STANDARD.encode(&raw);
    let target = dir.path().join("blob.bin");
    write_file_binary(
        &s,
        &json!({ "path": target.to_string_lossy(), "data": encoded }),
    )
    .unwrap();
    assert_eq!(fs::read(&target).unwrap(), raw);
}

#[test]
fn write_file_binary_rejects_bad_base64() {
    let dir = scratch_dir();
    let s = fresh_state();
    allow_dir(&s, &json!({ "path": dir.path().to_string_lossy() })).unwrap();
    let target = dir.path().join("blob.bin");
    let err = write_file_binary(
        &s,
        &json!({ "path": target.to_string_lossy(), "data": "@@@not_base64@@@" }),
    )
    .unwrap_err();
    assert!(err.contains("base64"), "got: {err}");
}

#[test]
fn write_file_binary_requires_allowed_parent() {
    let dir = scratch_dir();
    let s = fresh_state();
    let target = dir.path().join("blob.bin");
    let encoded = base64::engine::general_purpose::STANDARD.encode(b"hi");
    assert!(write_file_binary(
        &s,
        &json!({ "path": target.to_string_lossy(), "data": encoded }),
    )
    .is_err());
}

#[test]
#[cfg(unix)]
fn write_file_binary_rejects_symlink_escape() {
    // parent dir is allow-listed, but target file is a symlink pointing
    // outside of it → must be rejected.
    let allowed = scratch_dir();
    let outside = scratch_dir();
    let outside_target = outside.path().join("escaped.bin");
    let link = allowed.path().join("blob.bin");
    std::os::unix::fs::symlink(&outside_target, &link).unwrap();

    let s = fresh_state();
    allow_dir(&s, &json!({ "path": allowed.path().to_string_lossy() })).unwrap();

    let encoded = base64::engine::general_purpose::STANDARD.encode(b"should-not-land");
    let err = write_file_binary(
        &s,
        &json!({ "path": link.to_string_lossy(), "data": encoded }),
    )
    .unwrap_err();
    assert!(
        err.contains("symlink") || err.contains("Access denied") || err.contains("not allowed"),
        "got: {err}"
    );
    assert!(
        !outside_target.exists(),
        "data landed outside allowed dir via symlink"
    );
}

// --- ensure_dir --------------------------------------------------------

#[test]
fn ensure_dir_creates_child_of_allowed_dir() {
    let parent = scratch_dir();
    let s = fresh_state();
    allow_dir(&s, &json!({ "path": parent.path().to_string_lossy() })).unwrap();
    let child = parent.path().join("new-folder");
    ensure_dir(&s, &json!({ "path": child.to_string_lossy() })).unwrap();
    assert!(child.is_dir());
}

#[test]
fn ensure_dir_idempotent() {
    let parent = scratch_dir();
    let s = fresh_state();
    allow_dir(&s, &json!({ "path": parent.path().to_string_lossy() })).unwrap();
    let child = parent.path().join("new-folder");
    ensure_dir(&s, &json!({ "path": child.to_string_lossy() })).unwrap();
    ensure_dir(&s, &json!({ "path": child.to_string_lossy() })).unwrap();
    assert!(child.is_dir());
}

#[test]
fn ensure_dir_denies_outside_allow_list() {
    let parent = scratch_dir();
    let s = fresh_state();
    let child = parent.path().join("new-folder");
    assert!(ensure_dir(&s, &json!({ "path": child.to_string_lossy() })).is_err());
    assert!(!child.exists());
}
