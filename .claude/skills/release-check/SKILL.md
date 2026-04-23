---
name: release-check
description: Run the full pre-release gate for the fude crate (fmt, clippy strict, tests, doctests, rustdoc, publish dry-run, CHANGELOG sanity). Use before tagging or publishing to crates.io.
disable-model-invocation: true
---

# release-check

Runs the pre-publish gate for fude. All steps must pass before cutting a release.

## Steps

Run each in order. Stop on first failure and report which step failed.

1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test --all-targets`
4. `cargo test --doc`
5. `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`
6. `cargo publish --dry-run`
7. Check that `CHANGELOG.md` has an entry for the current `Cargo.toml` version (not just `[Unreleased]`).
8. Check that no dep in `Cargo.toml` was upgraded to a version released <14 days ago (supply-chain quarantine — see CLAUDE.md).

## Output

Report a single-line verdict: `READY` with the version, or `BLOCKED: <step> — <reason>`.

Do NOT run `cargo publish` (without `--dry-run`) or `git tag`. The user cuts the release.
