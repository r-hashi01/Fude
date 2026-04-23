# fude (筆)

[![crates.io](https://img.shields.io/crates/v/fude.svg)](https://crates.io/crates/fude)
[![docs.rs](https://docs.rs/fude/badge.svg)](https://docs.rs/fude)
[![CI](https://github.com/r-hashi01/Fude/actions/workflows/ci.yml/badge.svg)](https://github.com/r-hashi01/Fude/actions/workflows/ci.yml)
[![MIT/Apache 2.0 licensed](https://img.shields.io/crates/l/fude.svg)](#license)

**The brush for AI-native document editors.** A minimal Rust shell that gives a
web frontend exactly what it needs to co-write with an AI — and nothing else.

```rust
use fude::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new("com.example.my-editor")
        .title("My Editor")
        .assets("./dist")
        .with_fs_sandbox()
        .with_dialogs()
        .command("ping", |_ctx, _args| Ok(serde_json::json!("pong")))
        .run()
}
```

## Why this exists

Tauri and Electron are general-purpose. They carry runtime for every kind of
desktop app. For the specific shape of *"an editor where a human and an AI
agent edit the same document together"*, 90% of that weight is dead code.

Fude is the opposite move: pick one use case, ship only its primitives, stay
under **~1 MB**.

|                   | Tauri 2                           | Electron   | **Fude**          |
| ----------------- | --------------------------------- | ---------- | ----------------- |
| Binary (this app) | ~4.5 MB                           | ~80 MB     | **~1.0 MB**       |
| Scope             | general                           | general    | AI × docs only    |
| Plugins           | yes                               | yes        | compile-time only |
| Agent primitives  | via plugins                       | via IPC    | **built-in**      |
| Config surface    | `tauri.conf.json` + capabilities  | `main.js`  | Rust builder      |

## The use case, precisely

An app where:

1. A **human edits a document** (markdown, prose, code — doesn't matter) in a
   web UI.
2. An **AI agent also edits** the same document, via a CLI (PTY) or a protocol
   (ACP).
3. Both sides need **scoped filesystem access** the user has explicitly
   granted. Never more.

Everything in Fude exists to serve those three.

## Philosophy

- **Narrow on purpose.** Not a generic desktop framework. Features that don't
  serve *human + AI + document* stay out.
- **Sandboxed by default.** File I/O is allow-list only. No path touches disk
  until the user selects it through a native dialog. System dirs and
  credential stores (`~/.ssh`, `~/.aws`, `.git`, …) are blocked regardless of
  allow-list.
- **Agents are first-class.** PTY for CLI agents (`claude`, `codex`, …) and
  ACP (Agent Client Protocol) clients are core primitives, not plugins. The
  agent's `fs/read` / `fs/write` calls pass through the same sandbox.
- **Web frontend, unopinionated.** Ship a `dist/`. Fude serves it over
  `asset://` and exposes `window.__shell_ipc` / `window.__shell_listen`. Pick
  any framework.
- **Builder, not DSL.** One `App::new().with_x().with_y().command(…).run()`
  chain. No config files, no macros.
- **Small enough to read.** The whole crate fits in a long afternoon.

## What's in the box

Built on [`wry`](https://github.com/tauri-apps/wry) +
[`tao`](https://github.com/tauri-apps/tao) — the same webview/window pair
Tauri uses, but wired directly.

| Module      | What it provides                                                                                           | Opt-in via          |
| ----------- | ---------------------------------------------------------------------------------------------------------- | ------------------- |
| Core shell  | Single window, `asset://` protocol, `window.__shell_ipc` JSON bridge, server-push events                   | `App::new`          |
| Sandbox     | Allow-list, atomic writes, blocked system/credential paths, symlink-escape checks                          | `with_fs_sandbox()` |
| Dialogs     | File / folder pickers, message / question boxes (via `rfd`)                                                | `with_dialogs()`    |
| PTY agents  | Spawn allow-listed CLI tools from trusted install dirs; stream output as base64                            | `with_pty(&[…])`    |
| ACP client  | JSON-RPC over stdio to Agent Client Protocol servers, with sandboxed `fs/*` handlers and permission prompts | `with_acp(…)`       |

Each one is opt-in. `App::new()` alone is a webview and an IPC channel,
period.

## Security model

Fude's sandbox has three layers:

1. **Absolute block-list** — even with explicit user consent, Fude refuses to
   read or write paths under system directories (`/etc`, `/var`, `/usr`,
   `/Library`, `/private/*`, …) or credential stores (`.ssh`, `.gnupg`,
   `.aws`, `.kube`, `.docker`, `.git`, `.netrc`, `.npmrc`, `Keychains`).
   Comparison is case-insensitive and component-wise; `/tmp/etcetera/notes.md`
   is fine, `/Users/alice/.SSH/id_rsa` is not.
2. **Symlink-escape detection** — every path is canonicalized before checks.
   A symlink inside an allowed dir that resolves outside is rejected.
   `write_file_binary` additionally refuses to write to an existing symlink at
   all.
3. **Allow-list only for everything else** — nothing reads or writes until the
   user selects a file or folder through a native dialog, and even then only
   the chosen path (or its children, for directories) is accessible.

PTY spawning has its own layer: only tool names in your `with_pty(&[…])`
allow-list may be spawned, they must resolve to a binary in a trusted install
directory (`/opt/homebrew/bin`, `~/.cargo/bin`, etc.), and the child's `PATH`
is overwritten — a compromised frontend cannot inject a malicious binary.

ACP client wraps the same sandbox: when an ACP agent asks to
`fs/read_text_file` or `fs/write_text_file`, Fude handles it on the agent's
behalf, applying the allow-list before touching disk.

## Example: markdown editor

```rust
use fude::{acp::AcpAdapterConfig, App};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new("com.example.editor")
        .title("Co-write")
        .assets("./dist")
        .with_fs_sandbox()
        .with_dialogs()
        .with_pty(&["claude", "codex"])
        .with_acp(
            vec![AcpAdapterConfig {
                name: "claude-code".into(),
                candidate_bin_names: vec!["claude-code-acp".into()],
            }],
            "co-write",
            env!("CARGO_PKG_VERSION"),
        )
        .command("hello", |_ctx, args| {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("world");
            Ok(serde_json::json!(format!("hi, {name}")))
        })
        .run()
}
```

From the frontend:

```js
await window.__shell_ipc("dialog_open", { directory: true });
await window.__shell_ipc("allow_dir", { path: "/Users/alice/notes" });
await window.__shell_ipc("read_file", { path: "/Users/alice/notes/today.md" });

window.__shell_listen("acp:session-update", (payload) => {
  // agent streamed a session update
});
```

See [`examples/`](./examples) for runnable samples.

## Docs

- [`docs/frontend-bridge.md`](./docs/frontend-bridge.md) — full JS API and IPC command reference
- [`docs/sandbox.md`](./docs/sandbox.md) — sandbox threat model and exact guarantees
- [`docs/cookbook.md`](./docs/cookbook.md) — practical patterns (state sharing, image rendering, Tauri migration, custom AI, …)

## Non-goals

- Multi-window management beyond one root window.
- Tray icons, global shortcuts, OS integration beyond dialogs.
- Plugin ecosystem — no runtime plugin loader. Add things in Rust at compile
  time.
- Platform surface beyond what a markdown / prose / notebook editor needs.
- Hiding the webview. You're expected to know you're shipping web code.
- Replacing Tauri. For general apps, Tauri is correct. Fude is for one shape.

## Target user

You if:

- You're building a note, markdown, prose, or notebook editor.
- You want an AI to edit the document alongside the user.
- You'd rather read ~2 000 lines of Rust than configure a framework.
- A 1 MB binary matters to you.

Not you if:

- You need an app shell for a general product (use Tauri).
- You need many windows, tray, background services, notifications (use
  Tauri).
- You want a plugin marketplace.

## Naming

**Fude (筆)** — the brush. In Japanese calligraphy, the tool that touches the
paper. Tauri chose 鳥居 (the gate) to name *"the entry point into a
webview"*; Fude names *"the implement you write with"*. Narrower, more
intimate, more honest about scope.

## Status

Pre-1.0. Extracted from a production app, but API may shift before 1.0.
Feedback and issues welcome.

## Minimum supported Rust version

Fude targets Rust **1.93** and newer (edition2024 required by transitive
deps). MSRV bumps are treated as a `minor`-version change.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE))
- MIT license ([LICENSE-MIT](./LICENSE-MIT))

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual-licensed as above, without any additional terms or
conditions.
