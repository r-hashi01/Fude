# Contributing to fude

Thanks for considering a contribution. `fude` is narrow on purpose, so the
first question for most changes is **"does this belong in `fude`, or in
the app on top of it?"**. Please read
[`docs/ROADMAP.md`](./docs/ROADMAP.md) before filing a large change.

## Before you start

- **Bug?** File an issue with the Bug Report template.
- **New feature, new `with_*` layer, or a scope change?** Please open a
  [Discussion](https://github.com/r-hashi01/Fude/discussions) first so we
  can agree on whether it fits the crate's scope before anyone writes
  code. Scope-changing PRs without a prior discussion will usually be
  asked to start one.
- **Small docs / typo / obvious bug fix?** A PR is fine, no prior
  discussion needed.

## Dev setup

- Rust **1.93+** (the MSRV is set in `Cargo.toml`; edition 2024).
- macOS and Linux are the supported dev platforms. Windows is on the
  roadmap but not yet in CI.
- Linux needs WebKit / GTK headers:

  ```sh
  sudo apt-get install -y \
    libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev \
    libayatana-appindicator3-dev librsvg2-dev libjavascriptcoregtk-4.1-dev
  ```

## The loop

Before pushing, run the four commands CI runs:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets && cargo test --doc
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
```

All four must pass. `main` is branch-protected on these checks — your PR
won't merge until they go green.

## Conventions

- **Commit messages**: [Conventional Commits](https://www.conventionalcommits.org/).
  `release-plz` parses the history to derive version bumps and the
  CHANGELOG, so the prefix matters. Common ones:
  - `feat:` — new user-visible functionality (triggers a minor bump).
  - `fix:` — bug fix (triggers a patch bump).
  - `docs:`, `refactor:`, `test:`, `ci:`, `chore:` — no version bump.
- **Errors**: handlers return `Result<_, String>`. Don't introduce
  `thiserror` or a custom error enum.
- **Tests**: public-API behaviour goes in `tests/` as integration tests.
  Private helpers use inline `#[cfg(test)] mod tests`.
- **Dependencies**: prefer recent versions and editions, but never adopt
  a release under 14 days old — supply-chain quarantine.
- **GitHub Actions**: `uses:` refs must be pinned to a full commit SHA
  with the version as a trailing comment (`# v6.0.2`). Tags and branches
  are mutable; SHAs are not.
- **New `pub` items**: these grow the stable surface. Expect a review
  comment asking *why* something needs to be public before it lands.

## Gotchas worth knowing

- macOS `/tmp` canonicalises to `/private/tmp`, which the sandbox
  allow-list rejects. Tests that touch the real filesystem should use
  `$HOME/.fude-test-scratch`.
- Security invariant: no write path may follow a symlink to a location
  outside the allow-list. `write_file_binary` and `acp_write_file` have
  had this bug before — canonicalize the final target, don't just check
  the parent.

## CHANGELOG

Don't edit `CHANGELOG.md` by hand. `release-plz` rewrites it from the
commit history when a release PR is opened. The only exception is
correcting a previously shipped entry after the fact (that has happened
exactly once — see the v0.1.1 backfill PR if you're curious).

## Releasing

Maintainer-only. `release-plz` opens a Release PR once there are
semver-relevant commits on `main`; merging that PR tags and publishes.
No manual `cargo publish`.

## Licensing of contributions

`fude` is dual-licensed under `MIT OR Apache-2.0`. By contributing, you
agree that your contribution may be distributed under those same terms,
as stated in the [Apache-2.0 License section 5]. No CLA.

[Apache-2.0 License section 5]: https://www.apache.org/licenses/LICENSE-2.0#contributions
