# Fude

Rust "brush" framework — minimal shell for AI-native document editors. Narrow on purpose; keep dependencies and API surface small.

## Conventions
- License: `MIT OR Apache-2.0` (dual). MSRV: `1.93` (edition2024 required by transitive deps).
- Errors: return `Result<_, String>`. Do not introduce `thiserror` / custom error enums.
- Tests: public API → `tests/` integration. Private helpers → inline `#[cfg(test)] mod tests`.
- TDD: write/adjust tests first; aim for publishable quality (clippy strict, fmt clean).
- Deps: prefer recent versions/editions, but never adopt a release under 2 weeks old (supply-chain quarantine).

## Gotchas
- macOS `/tmp` canonicalizes to `/private/tmp` → sandbox allow-list rejects it. Use `$HOME/.fude-test-scratch` for tests that touch real FS.
- Security invariant: no write path may follow a symlink to a location outside the allow-list. `write_file_binary` and `acp_write_file` have had this bug before — canonicalize the final target, don't just check the parent.

## Commands
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all-targets && cargo test --doc`
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`

## Git
- Local commits only. Do not `git push`; the user publishes to `github.com/r-hashi01/Fude`.

## Claude tooling
- Skills: `/release-check` (pre-publish gate), `/new-fude-feature` (scaffold a `with_*` feature).
- Subagents auto-invoked: `sandbox-invariant-reviewer` (FS/ACP/PTY diffs), `public-api-surface-reviewer` (any new `pub`).
- MCP: `.mcp.json` declares `context7` + `github`. GitHub MCP needs `GITHUB_PERSONAL_ACCESS_TOKEN` in env.
- Hooks: `.rs` edits auto-run `cargo fmt`. `Cargo.lock`, `LICENSE-*`, `.github/workflows/**` are deny-listed.
