# fude (筆)

[![crates.io](https://img.shields.io/crates/v/fude-rs.svg)](https://crates.io/crates/fude-rs)
[![docs.rs](https://docs.rs/fude-rs/badge.svg)](https://docs.rs/fude-rs)
[![CI](https://github.com/r-hashi01/Fude/actions/workflows/ci.yml/badge.svg)](https://github.com/r-hashi01/Fude/actions/workflows/ci.yml)
[![MIT/Apache 2.0 licensed](https://img.shields.io/crates/l/fude-rs.svg)](#license)

**The brush for AI-native document editors.** A Rust shell that pre-wires the
three things every "human + agent + document" app needs — a sandboxed
filesystem, safe PTY spawning for CLI agents, and an ACP client — and ships
nothing you didn't ask for.

```text
~3,000 lines of Rust in src/. ~1 MB binary with everything turned on.
Webview + sandboxed FS + PTY agents + ACP client. That is the whole scope.
```

```rust
use fude::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new("com.example.hello").assets("./dist").run()
}
```

That's a working webview app. Every other feature is one `.with_*()` away.

## Why pick fude

Tauri and Electron are general-purpose: their runtime covers every kind of
desktop app. Fude takes the opposite bet — pick one shape of app, ship only
its primitives, stay small enough to audit.

|                   | Tauri 2                           | Electron   | **fude**          |
| ----------------- | --------------------------------- | ---------- | ----------------- |
| Binary (this app) | ~4.5 MB                           | ~80 MB     | **~1.0 MB**       |
| Shell core LOC    | ~100k+ (workspace)                | N/A (JS)   | **~3k**           |
| Scope             | general                           | general    | AI × docs only    |
| Plugins           | yes                               | yes        | compile-time only |
| Agent primitives  | via plugins                       | via IPC    | **built-in**      |
| Config surface    | `tauri.conf.json` + capabilities  | `main.js`  | Rust builder      |
| Maintainers       | 200+ contributors                 | 1000+      | 1 (be aware)      |

The last row is honest: fude is a one-person library, pre-1.0. The small
surface is its strength *and* its fragility. If you need hundreds of eyes
on your shell code today, use Tauri. If you'd rather read the shell code
yourself before shipping it, read on.

## What fude wires for you

Three things an AI-native editor needs that every team builds wrong at least
once. Each is already in the box, tested, and gated by an opt-in builder.

```text
Without fude                                With fude
─────────────                               ─────────
Write an FS allow-list:                     .with_fs_sandbox()
  canonicalize every path,
  block-list ~/.ssh, ~/.aws, …,
  symlink-escape detection,
  refuse write to existing symlinks           (230 LOC you don't write)

Spawn a CLI agent safely:                   .with_pty(&["claude", "codex"])
  allow-list by name,
  resolve inside trusted dirs only,
  scrub PATH before exec,
  stream PTY output back to UI                (350 LOC you don't write)

Talk to an ACP agent:                       .with_acp(…)
  JSON-RPC over stdio,
  route fs/read, fs/write through sandbox,
  permission prompts from agent,
  session-update streaming to UI              (900 LOC you don't write)
```

If any of those three rows describes a problem you're actively solving, fude
is pitched *at you*. If none of them do, fude is probably not the right
tool.

## Read it yourself

Not a black box. The "narrow on purpose" claim is verifiable in minutes:

| File                  | LOC   | What it's responsible for                                |
| --------------------- | ----- | -------------------------------------------------------- |
| `src/sandbox.rs`      | 228   | Allow-list, block-list, canonicalization, symlink guard  |
| `src/fs.rs`           | 225   | FS command handlers layered on `sandbox`                 |
| `src/pty.rs`          | 350   | Allow-listed PTY spawning + streaming                    |
| `src/acp.rs`          | 614   | ACP client (JSON-RPC, stdio, session bookkeeping)        |
| `src/acp_commands.rs` | 287   | ACP command handlers that bridge to the sandbox          |
| `src/assets.rs`       | 394   | `asset://` protocol serving `./dist` or sandboxed files  |
| `src/lib.rs`          | 530   | Builder, IPC bridge, event loop                          |
| (plus 5 smaller)      | 463   | dialogs, shell, settings, events, …                      |
| **Total**             | **3,091** |                                                      |

If you plan to ship an app that touches a user's filesystem, being able to
audit the 228 lines that decide what `write_file` will and won't accept is
the feature.

## 5-minute quickstart

From empty directory to a running, sandboxed editor.

### 1. A new crate
```sh
cargo new --bin my-editor
cd my-editor
cargo add fude-rs
```

### 2. The frontend (one file)
```sh
mkdir dist
cat > dist/index.html <<'HTML'
<!doctype html>
<textarea id="t" rows=20 cols=60></textarea>
<button onclick="pick()">Open…</button>
<script>
  async function pick() {
    const dir = await window.__shell_ipc("dialog_open", { directory: true });
    if (!dir) return;
    await window.__shell_ipc("allow_dir", { path: dir });
    const path = `${dir}/notes.md`;
    const text = await window.__shell_ipc("read_file", { path })
      .catch(() => "");
    t.value = text;
    t.oninput = () => window.__shell_ipc("write_file", { path, content: t.value });
  }
</script>
HTML
```

### 3. The backend (seven lines)
```rust
// src/main.rs
use fude::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new("com.example.my-editor")
        .title("My Editor")
        .assets("./dist")
        .with_fs_sandbox()
        .with_dialogs()
        .run()
}
```

### 4. Run
```sh
cargo run
```

You now have a desktop app where: the user picks a directory through a
native dialog, anything under that directory (and *only* that directory) is
readable and writable, and the sandbox refuses symlinks that escape, refuses
paths under `~/.ssh`, `~/.aws`, `.git`, and so on — regardless of what the
frontend asks for.

Layering on a CLI agent is one more line: `.with_pty(&["claude"])`.
Layering on an ACP agent is one more: `.with_acp(…)`. Full examples in
[`examples/`](./examples).

## Philosophy

- **Narrow on purpose.** Features that don't serve *human + AI + document*
  stay out, even if they'd be easy.
- **Sandboxed by default.** File I/O is allow-list only. No path touches
  disk until the user selects it through a native dialog. System dirs and
  credential stores (`~/.ssh`, `~/.aws`, `.git`, …) are blocked regardless
  of allow-list.
- **Agents are first-class.** PTY for CLI agents and ACP clients are core
  primitives, not plugins.
- **Web frontend, unopinionated.** Ship a `dist/`. Fude serves it over
  `asset://` and exposes `window.__shell_ipc` / `window.__shell_listen`.
  Pick any framework.
- **Builder, not DSL.** One `App::new().with_x().with_y().command(…).run()`
  chain. No config files. No macros.

## What's in the box

Built on [`wry`](https://github.com/tauri-apps/wry) +
[`tao`](https://github.com/tauri-apps/tao) — the same webview/window pair
Tauri uses, but wired directly.

| Module      | What it provides                                                                                | Opt-in via          |
| ----------- | ------------------------------------------------------------------------------------------------| ------------------- |
| Core shell  | Single window, `asset://` protocol, `window.__shell_ipc` JSON bridge, server-push events        | `App::new`          |
| Sandbox     | Allow-list, atomic writes, blocked system/credential paths, symlink-escape checks               | `with_fs_sandbox()` |
| Dialogs     | File / folder pickers, message / question boxes (via `rfd`)                                     | `with_dialogs()`    |
| PTY agents  | Spawn allow-listed CLI tools from trusted install dirs; stream output as base64                 | `with_pty(&[…])`    |
| ACP client  | JSON-RPC over stdio to ACP servers, with sandboxed `fs/*` handlers and permission prompts       | `with_acp(…)`       |

Each one is opt-in. `App::new()` alone is a webview and an IPC channel,
period.

## Security model

Fude's sandbox has three layers:

1. **Absolute block-list** — even with explicit user consent, Fude refuses
   to read or write paths under system directories (`/etc`, `/var`, `/usr`,
   `/Library`, `/private/*`, …) or credential stores (`.ssh`, `.gnupg`,
   `.aws`, `.kube`, `.docker`, `.git`, `.netrc`, `.npmrc`, `Keychains`).
   Comparison is case-insensitive and component-wise; `/tmp/etcetera/notes.md`
   is fine, `/Users/alice/.SSH/id_rsa` is not.
2. **Symlink-escape detection** — every path is canonicalized before checks.
   A symlink inside an allowed dir that resolves outside is rejected.
   `write_file_binary` additionally refuses to write to an existing symlink
   at all.
3. **Allow-list only for everything else** — nothing reads or writes until
   the user selects a file or folder through a native dialog, and even then
   only the chosen path (or its children, for directories) is accessible.

PTY spawning has its own layer: only tool names in your `with_pty(&[…])`
allow-list may be spawned, they must resolve to a binary in a trusted
install directory (`/opt/homebrew/bin`, `~/.cargo/bin`, etc.), and the
child's `PATH` is overwritten — a compromised frontend cannot inject a
malicious binary.

ACP wraps the same sandbox: when an ACP agent asks to `fs/read_text_file`
or `fs/write_text_file`, Fude handles it on the agent's behalf, applying
the allow-list before touching disk.

## Docs

- [`docs/frontend-bridge.md`](./docs/frontend-bridge.md) — JS API and IPC
  command reference
- [`docs/sandbox.md`](./docs/sandbox.md) — sandbox threat model and exact
  guarantees
- [`docs/cookbook.md`](./docs/cookbook.md) — state sharing, image rendering,
  Tauri migration, custom AI integration, …
- [`docs/ROADMAP.md`](./docs/ROADMAP.md) — where fude is headed and what it
  won't grow into
- [`examples/`](./examples) — `minimal`, `ipc-hello`, `pty-terminal`,
  `cowrite`, `acp-chat`

## Is this for you?

**Yes if:**
- You're building a note / markdown / prose / notebook editor where an AI
  agent touches the user's files.
- A ~1 MB binary and ~3k LOC of shell you can read matter to you.
- You'd rather write Rust than fight a config file.

**No if:**
- You need a general-purpose app shell — [Tauri](https://tauri.app) is the
  right call.
- You need tabs, multiple windows, tray icons, background services, plugin
  marketplaces. None of those are coming.
- You need many contributors, auditors, or an SLA today. Pre-1.0,
  one-maintainer — vendor it or wait.

## Non-goals (explicit)

- Multi-window / tab management beyond one root window.
- Tray icons, global shortcuts, OS integration beyond dialogs.
- Runtime plugin loader. Add things in Rust at compile time.
- Hiding the webview. You're shipping web code; fude won't pretend
  otherwise.
- Replacing Tauri for general apps.

## Naming

**Fude (筆)** — the brush. In Japanese calligraphy, the tool that touches
the paper. Tauri named itself after the gate (鳥居); fude names itself
after the implement. Narrower, more intimate, more honest about scope.

## Status & MSRV

Pre-1.0, extracted from a production app. The public API may still shift
before 1.0 — see [`docs/ROADMAP.md`](./docs/ROADMAP.md). Rust **1.93+**
(edition2024 required by transitive deps). MSRV bumps are a minor-version
change.

## License

Licensed under either of Apache License, Version 2.0
([LICENSE-APACHE](./LICENSE-APACHE)) or MIT
([LICENSE-MIT](./LICENSE-MIT)) at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual-licensed as above, without any additional terms or
conditions.

See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for the dev loop,
[`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md) for community expectations,
and [`SECURITY.md`](./SECURITY.md) for reporting vulnerabilities.
