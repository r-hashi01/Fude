//! Shared fixtures for integration tests.
//!
//! `/tmp` and `/var` on macOS canonicalize into `/private/*`, which is
//! blocked by `validate_path`. So scratch directories live under
//! `$HOME/.fude-test-scratch/` which is not on any block-list.

use std::path::PathBuf;

pub fn scratch_dir() -> tempfile::TempDir {
    let home = std::env::var("HOME").expect("HOME set");
    let base = PathBuf::from(home).join(".fude-test-scratch");
    std::fs::create_dir_all(&base).expect("create scratch base");
    tempfile::TempDir::new_in(&base).expect("create scratch tempdir")
}
