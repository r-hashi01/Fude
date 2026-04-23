---
name: new-fude-feature
description: Scaffold a new opt-in feature for the fude crate following the with_* builder convention — new module in src/, App::with_<feature> method, integration tests in tests/, inline tests only for private helpers. Use when adding a new capability like with_tray, with_updater, etc.
---

# new-fude-feature

Scaffolds a new opt-in feature following fude's "narrow on purpose" convention.

## Invariants to preserve

- Public surface grows only through `App::with_<feature>(...)` — never a free function.
- Errors return `Result<_, String>`; no `thiserror`.
- Public API → tests in `tests/<feature>.rs`. Private helpers → inline `#[cfg(test)] mod tests`.
- No new top-level dependency unless there is no reasonable alternative; if added, it must be ≥14 days old (see CLAUDE.md).

## Steps

Given a feature name `<feat>`:

1. **TDD first.** Create `tests/<feat>.rs` with failing tests for the public contract. Run `cargo test --test <feat>` and confirm red.
2. Create `src/<feat>.rs`. Expose only the types / functions the test file needs.
3. Wire into `src/lib.rs`: `mod <feat>;` and add `pub fn with_<feat>(mut self, ...) -> Self` on `App`.
4. Make tests green. Keep diffs minimal.
5. If the feature touches the sandbox (FS, paths, PTY, external binaries), add a section to `tests/<feat>.rs` that exercises the symlink-escape / allow-list invariants.
6. Run the full gate: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --all-targets && cargo test --doc`.
7. Add a `CHANGELOG.md` entry under `[Unreleased]`.

## Anti-patterns (reject)

- Adding `pub` items that aren't reachable from an `App::with_*` method.
- Re-exporting third-party types at the crate root.
- Catch-all error enums. Return `Err("short reason".into())`.
- Mocking the filesystem in tests — use `tempfile` (already in dev-deps) or `$HOME/.fude-test-scratch` on macOS (`/tmp` canonicalizes and trips the sandbox).
