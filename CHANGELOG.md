# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.2](https://github.com/r-hashi01/Fude/compare/v0.1.1...v0.1.2) - 2026-04-23

### Other

- add ROADMAP with now / next / later / non-goals ([#9](https://github.com/r-hashi01/Fude/pull/9))

## [0.1.1](https://github.com/r-hashi01/Fude/compare/v0.1.0...v0.1.1) - 2026-04-23

### Fixed

- Adapt to the wry 0.55 `WebViewBuilder` API: the window is passed to
  `.build(&window)` instead of `::new(&window)`, and custom-protocol
  handlers receive a `WebViewId` first argument.
- Align the Linux-specific `rfd` target override with the base 0.17
  requirement so the `gtk3` backend feature is actually applied.

### Changed

- Bump `wry` 0.45 → 0.55, `tao` 0.30 → 0.35, `rfd` 0.15 → 0.17.
- Bump `actions/checkout` (pinned to v6.0.2 SHA) in CI.
- Dependabot now ignores all `version-update:semver-major` bumps for both
  `cargo` and `github-actions`; major bumps are a deliberate human action.
- Document `main` branch protection (PR + CI required) in `CLAUDE.md`.

### Added

- `release-plz` GitHub Actions workflow for automated version bumps,
  CHANGELOG generation, tagging, and crates.io publishing driven by
  conventional-commit history.

## [0.1.0] — 2026-04-23

Initial public release. Extracted and generalised from the
[mdeditor](https://github.com/r-hashi01) project into a reusable Rust crate.

### Added

#### Core shell
- `App` builder: window + webview + `asset://localhost/` protocol + JSON IPC
  bridge (`window.__shell_ipc`, `window.__shell_listen`).
- `Ctx` runtime context passed to every command handler. Exposes the app
  identifier, the shared `EventEmitter`, the FS allow-list state, and a
  `MainDispatcher` for running closures on the UI thread.
- `App::command(name, handler)` — register arbitrary app-specific IPC
  commands.

#### FS sandbox (`App::with_fs_sandbox`)
- Allow-list gated file I/O: `allow_path`, `allow_dir`, `list_directory`,
  `read_file`, `read_file_binary`, `write_file`, `write_file_binary`,
  `ensure_dir`.
- `validate_path` / `is_path_allowed` / `is_dir_allowed` primitives with
  a block-list covering system directories and credential stores
  (`.ssh`, `.aws`, `.kube`, `.docker`, `.git`, `.npmrc`, `.netrc`,
  `Keychains`, `/etc`, `/var`, `/usr`, `/private/*`, etc.) — refused
  regardless of allow-list.
- `atomic_write` — sibling-temp-file + rename pattern.
- `app_config_dir` / `app_data_dir` — Tauri-compatible per-OS paths.
- `ensure_scratch(data_dir, allowed_dirs, name)` and the convenience
  wrapper `ensure_scratch_dir(ctx, name)` for app-owned scratch
  directories.
- `write_file_binary` refuses to write when the target is an existing
  symbolic link, preventing rename-over-symlink sandbox escapes.

#### Native dialogs (`App::with_dialogs`)
- `dialog_open`, `dialog_save`, `dialog_ask`, `dialog_message` backed by
  `rfd`. All dialogs are dispatched to the main (UI) thread via
  `MainDispatcher` — required on macOS where AppKit refuses modal
  panels from background threads.

#### PTY (`App::with_pty`)
- Allow-list gated subprocess spawning over a PTY: `pty_spawn`,
  `pty_write`, `pty_resize`, `pty_kill` with `pty:data` / `pty:exit`
  events.
- Only tools in the app-provided allow-list may be spawned; they must
  also resolve to a binary in a trusted install directory
  (`/opt/homebrew/bin`, `~/.cargo/bin`, `~/.local/bin`,
  `~/.npm-global/bin`, …). The child process always runs with an
  overwritten `PATH` — a compromised frontend cannot inject a
  malicious binary.

#### ACP client (`App::with_acp`)
- 11 `acp_*` commands implementing an Agent Client Protocol client over
  JSON-RPC stdio, with sandboxed `fs/read_text_file` and
  `fs/write_text_file` responses.
- Adapter discovery over `PATH` + trusted install dirs.
- Permission auto-selection for safe update kinds (`read`, `edit`,
  `think`, `search`) with a `session:permission` event for anything
  else.

#### Shell open (`App::with_shell_open`)
- `shell_open(target)` IPC command — opens `http://`, `https://`, or
  `mailto:` URLs, or allow-listed file paths, in the OS default
  application. Other schemes (`file://`, `javascript:`, `data:`, custom)
  are refused at the IPC boundary.

#### Settings (`App::with_settings`)
- `load_settings` / `save_settings` IPC commands. Persists an arbitrary
  JSON object at `<app_config_dir>/settings.json` via `atomic_write`.
  Schema-less; apps validate their own shape.

#### Asset rendering
- `asset_url_from_file(path)` — builds an `asset://localhost/__file/…`
  URL for direct rendering of allow-listed local files in the webview
  (`<img>`, `<video>`, `<iframe>` src), analogous to Tauri's
  `convertFileSrc`. Also exposed to the frontend as
  `window.__shell_asset_url(path)`. The file is streamed only when its
  canonical path is in the FS allow-list at request time; otherwise 403.

#### Platform
- Main-thread dispatch via `MainDispatcher` + `UserEvent::RunOnMain`.
- IPC handlers run on a worker thread so command handlers can dispatch
  back to the UI thread (e.g. for dialogs) without deadlocking the wry
  callback.

#### Dev experience
- 118 tests (53 unit, 22 fs integration, 42 sandbox integration, 1
  doctest) covering sandbox invariants, asset serving, percent-decode,
  ACP helpers, PTY validation.
- CI on macOS + Linux, stable + MSRV 1.75, `rustfmt --check`,
  `clippy -D warnings`, `rustdoc -D warnings`.
- Dual MIT / Apache-2.0 license.
- Four example apps: `ipc-hello`, `cowrite` (markdown editor),
  `pty-terminal`, `acp-chat`, plus two single-file cargo examples
  (`minimal.rs`, `editor.rs`).

[Unreleased]: https://github.com/r-hashi01/Fude/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/r-hashi01/Fude/releases/tag/v0.1.0
