# pty-terminal

Spawns an allow-listed CLI tool in a PTY and streams its output to a web
UI. Typed input is sent through `pty_write` and output arrives as
`pty:data` events (base64-encoded raw bytes).

## Prerequisites

At least one of the allow-listed tools (`claude`, `codex`, `bash`) on
your `PATH`. Edit the list in `main.rs` to match what you have.

## Run

From the repo root:

```sh
cargo run --example pty-terminal --release
```

1. **Choose folder…** — selects a cwd and adds it to the sandbox.
2. Pick a **tool** from the dropdown.
3. **Spawn** — fude resolves the tool to a binary in a trusted install
   dir (`/opt/homebrew/bin`, `~/.cargo/bin`, etc.), opens a PTY, and
   launches it. The child's `PATH` is overwritten — a compromised
   frontend cannot inject a malicious binary by pointing `PATH` elsewhere.
4. Type a line, press **Send** or **Enter**.
5. **Kill** — terminates the process.

## What this demonstrates

- **`with_pty(&["claude", "codex", "bash"])`** — only these binary names
  may ever be spawned; anything else is refused at the IPC boundary.
- **Composition** — PTY requires `with_fs_sandbox` because the cwd must
  be inside an allow-listed directory. The guarantee: a PTY session
  cannot touch files outside what the user explicitly granted.
- **Server-push streaming** — `pty:data` arrives as fast as the child
  produces bytes. The example decodes base64 and strips the most common
  ANSI escape sequences for readability; swap in
  [xterm.js](https://xtermjs.org) for a real terminal emulator.

## Files

```
main.rs           ~15 lines: App builder + with_pty
dist/index.html   terminal-ish UI with ANSI stripping
```
