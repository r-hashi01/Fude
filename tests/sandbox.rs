//! Integration tests for the path-sandbox public API:
//! `validate_path`, `is_path_allowed`, `is_dir_allowed`, `atomic_write`,
//! `app_config_dir`, `app_data_dir`, `new_list`, `safe_lock`.

mod common;

use std::fs;
use std::path::PathBuf;

use fude::{
    app_config_dir, app_data_dir, atomic_write, ensure_scratch, is_dir_allowed, is_path_allowed,
    new_list, safe_lock, validate_path,
};

use crate::common::scratch_dir;

// --- validate_path -----------------------------------------------------

#[test]
fn rejects_relative_path() {
    assert!(validate_path("relative/path.md").is_err());
}

#[test]
fn rejects_empty_path() {
    assert!(validate_path("").is_err());
}

#[test]
fn accepts_normal_absolute_path() {
    assert!(validate_path("/tmp/test-file.md").is_ok());
}

#[test]
fn blocks_etc_directory() {
    assert!(validate_path("/etc/passwd").is_err());
}

#[test]
fn blocks_private_etc() {
    assert!(validate_path("/private/etc/hosts").is_err());
}

#[test]
fn blocks_private_var() {
    assert!(validate_path("/private/var/db/secret").is_err());
}

#[test]
fn blocks_macos_system_library() {
    assert!(validate_path("/Library/Preferences/com.apple.plist").is_err());
}

#[test]
fn blocks_usr_bin() {
    assert!(validate_path("/usr/bin/ls").is_err());
}

#[test]
fn blocks_ssh_directory() {
    assert!(validate_path("/Users/alice/.ssh/id_rsa").is_err());
}

#[test]
fn blocks_aws_credentials() {
    assert!(validate_path("/Users/alice/.aws/credentials").is_err());
}

#[test]
fn blocks_gnupg_and_gpg() {
    assert!(validate_path("/Users/alice/.gnupg/secring.gpg").is_err());
    assert!(validate_path("/Users/alice/.gpg/whatever").is_err());
}

#[test]
fn blocks_kube_config() {
    assert!(validate_path("/Users/alice/.kube/config").is_err());
}

#[test]
fn blocks_docker_config() {
    assert!(validate_path("/Users/alice/.docker/config.json").is_err());
}

#[test]
fn blocks_netrc_and_npmrc() {
    assert!(validate_path("/Users/alice/.netrc").is_err());
    assert!(validate_path("/Users/alice/.npmrc").is_err());
}

#[test]
fn blocks_keychains_dir() {
    assert!(validate_path("/Users/alice/Library/Keychains/login.keychain").is_err());
}

#[test]
fn does_not_false_positive_on_etc_in_name() {
    assert!(validate_path("/tmp/etcetera/notes.md").is_ok());
}

#[test]
fn does_not_false_positive_on_usr_substring() {
    assert!(validate_path("/Users/alice/doc.md").is_ok());
}

#[test]
fn blocks_git_internal_paths() {
    assert!(validate_path("/Users/alice/repo/.git/config").is_err());
}

#[test]
fn ssh_match_is_case_insensitive() {
    assert!(validate_path("/Users/alice/.SSH/id_rsa").is_err());
}

#[test]
#[cfg(unix)]
fn blocks_symlink_pointing_at_etc() {
    let dir = scratch_dir();
    let link = dir.path().join("trap");
    if std::os::unix::fs::symlink("/etc", &link).is_ok() {
        let target = link.join("passwd");
        assert!(validate_path(&target.to_string_lossy()).is_err());
    }
}

// --- is_dir_allowed ----------------------------------------------------

#[test]
fn dir_allowed_when_on_list() {
    let dir = scratch_dir();
    let list = new_list();
    let canonical = fs::canonicalize(dir.path()).unwrap();
    safe_lock(&list).push(canonical.to_string_lossy().to_string());
    assert!(is_dir_allowed(&dir.path().to_string_lossy(), &list).is_ok());
}

#[test]
fn dir_allowed_when_prefix() {
    let dir = scratch_dir();
    let sub = dir.path().join("child");
    fs::create_dir_all(&sub).unwrap();
    let list = new_list();
    let canonical = fs::canonicalize(dir.path()).unwrap();
    safe_lock(&list).push(canonical.to_string_lossy().to_string());
    assert!(is_dir_allowed(&sub.to_string_lossy(), &list).is_ok());
}

#[test]
fn dir_denied_when_not_on_list() {
    let dir = scratch_dir();
    let list = new_list();
    assert!(is_dir_allowed(&dir.path().to_string_lossy(), &list).is_err());
}

#[test]
fn dir_denied_for_nonexistent() {
    let list = new_list();
    let missing = scratch_dir().path().join("does-not-exist");
    assert!(is_dir_allowed(&missing.to_string_lossy(), &list).is_err());
}

// --- is_path_allowed ---------------------------------------------------

#[test]
fn path_allowed_via_explicit_allow_list() {
    let dir = scratch_dir();
    let file = dir.path().join("a.md");
    fs::write(&file, b"x").unwrap();
    let paths = new_list();
    let dirs = new_list();
    let canonical = fs::canonicalize(&file)
        .unwrap()
        .to_string_lossy()
        .to_string();
    safe_lock(&paths).push(canonical);
    assert!(is_path_allowed(&file.to_string_lossy(), &paths, &dirs).is_ok());
}

#[test]
fn path_allowed_via_dir_inheritance() {
    let dir = scratch_dir();
    let file = dir.path().join("a.md");
    fs::write(&file, b"x").unwrap();
    let paths = new_list();
    let dirs = new_list();
    let canonical_dir = fs::canonicalize(dir.path())
        .unwrap()
        .to_string_lossy()
        .to_string();
    safe_lock(&dirs).push(canonical_dir);
    assert!(is_path_allowed(&file.to_string_lossy(), &paths, &dirs).is_ok());
}

#[test]
fn path_denied_when_nothing_allowed() {
    let dir = scratch_dir();
    let file = dir.path().join("a.md");
    fs::write(&file, b"x").unwrap();
    let paths = new_list();
    let dirs = new_list();
    assert!(is_path_allowed(&file.to_string_lossy(), &paths, &dirs).is_err());
}

// --- atomic_write ------------------------------------------------------

#[test]
fn atomic_write_round_trip() {
    let dir = scratch_dir();
    let target = dir.path().join("f.txt");
    atomic_write(&target, b"hello").unwrap();
    assert_eq!(fs::read_to_string(&target).unwrap(), "hello");
}

#[test]
fn atomic_write_overwrites_existing() {
    let dir = scratch_dir();
    let target = dir.path().join("f.txt");
    fs::write(&target, b"old").unwrap();
    atomic_write(&target, b"new").unwrap();
    assert_eq!(fs::read_to_string(&target).unwrap(), "new");
}

#[test]
fn atomic_write_leaves_no_temp_file_on_success() {
    let dir = scratch_dir();
    let target = dir.path().join("f.txt");
    atomic_write(&target, b"content").unwrap();
    let leftovers: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "stray .tmp file: {leftovers:?}");
}

#[test]
fn atomic_write_fails_when_parent_missing() {
    let target = scratch_dir().path().join("missing").join("f.txt");
    assert!(atomic_write(&target, b"nope").is_err());
}

// --- app_config_dir / app_data_dir -------------------------------------

#[cfg(target_os = "macos")]
#[test]
fn app_config_dir_macos_shape() {
    let home = std::env::var("HOME").unwrap();
    let got = app_config_dir("com.example.foo").unwrap();
    let expected = PathBuf::from(home)
        .join("Library/Application Support")
        .join("com.example.foo");
    assert_eq!(got, expected);
}

#[cfg(target_os = "linux")]
#[test]
fn app_config_dir_linux_shape() {
    let got = app_config_dir("com.example.foo").unwrap();
    assert!(got.ends_with("com.example.foo"));
    let parent = got.parent().unwrap();
    let matches_xdg = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(|p| parent == std::path::Path::new(&p))
        .unwrap_or(false);
    let matches_home = std::env::var("HOME")
        .ok()
        .map(|h| parent == PathBuf::from(h).join(".config"))
        .unwrap_or(false);
    assert!(matches_xdg || matches_home);
}

#[test]
fn app_data_dir_matches_config_dir() {
    let a = app_config_dir("com.example.foo").unwrap();
    let b = app_data_dir("com.example.foo").unwrap();
    assert_eq!(a, b);
}

// --- new_list / safe_lock ---------------------------------------------

#[test]
fn new_list_is_empty() {
    let list = new_list();
    assert!(safe_lock(&list).is_empty());
}

#[test]
fn safe_lock_recovers_from_poison() {
    let list = new_list();
    let l2 = list.clone();
    let _ = std::thread::spawn(move || {
        let _g = l2.lock().unwrap();
        panic!("poison");
    })
    .join();
    let mut g = safe_lock(&list);
    g.push("ok".into());
    assert_eq!(g.len(), 1);
}

// --- ensure_scratch ----------------------------------------------------

#[test]
fn ensure_scratch_creates_dir_and_allow_lists() {
    let base = scratch_dir();
    let dirs = new_list();
    let result = ensure_scratch(base.path(), &dirs, "cache").unwrap();
    assert!(result.is_dir());
    let guard = safe_lock(&dirs);
    assert_eq!(guard.len(), 1);
    assert!(guard[0].ends_with("cache"));
}

#[test]
fn ensure_scratch_is_idempotent() {
    let base = scratch_dir();
    let dirs = new_list();
    ensure_scratch(base.path(), &dirs, "cache").unwrap();
    ensure_scratch(base.path(), &dirs, "cache").unwrap();
    let guard = safe_lock(&dirs);
    assert_eq!(guard.len(), 1);
}

#[test]
fn ensure_scratch_creates_parent_data_dir() {
    let base = scratch_dir();
    let deep = base.path().join("not").join("yet").join("there");
    let dirs = new_list();
    let result = ensure_scratch(&deep, &dirs, "stuff").unwrap();
    assert!(result.is_dir());
}

#[test]
fn ensure_scratch_rejects_empty_name() {
    let base = scratch_dir();
    let dirs = new_list();
    assert!(ensure_scratch(base.path(), &dirs, "").is_err());
}

#[test]
fn ensure_scratch_rejects_path_separator() {
    let base = scratch_dir();
    let dirs = new_list();
    assert!(ensure_scratch(base.path(), &dirs, "foo/bar").is_err());
    assert!(ensure_scratch(base.path(), &dirs, "foo\\bar").is_err());
}

#[test]
fn ensure_scratch_rejects_parent_traversal() {
    let base = scratch_dir();
    let dirs = new_list();
    assert!(ensure_scratch(base.path(), &dirs, "..").is_err());
    assert!(ensure_scratch(base.path(), &dirs, "foo..bar").is_err());
}

#[test]
fn ensure_scratch_rejects_hidden_name() {
    let base = scratch_dir();
    let dirs = new_list();
    assert!(ensure_scratch(base.path(), &dirs, ".secret").is_err());
}
