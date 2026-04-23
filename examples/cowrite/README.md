# cowrite

A minimal markdown editor built on fude — FS sandbox + native dialogs + one
custom IPC command.

## Run

From the repo root:

```sh
cargo run --example cowrite --release
```

Click **Open…**, pick a `.md` file, edit, **Save**. The footer word-count
proves the custom `stat` IPC command round-trips between JS and Rust.

## What this demonstrates

- **Builder API** — `App::new(..).with_fs_sandbox().with_dialogs().command(..).run()`.
- **Sandbox flow** — the frontend cannot read any file until it receives a
  path from a native dialog and calls `allow_path`. Hard-coding a path and
  calling `read_file` from devtools returns an error.
- **Custom command** — `stat` is the pattern for exposing app-specific Rust
  logic (here: word/char/line count) to the web UI.

## Files

```
main.rs           ~20 lines: App builder + one command
dist/index.html   vanilla HTML/JS, no build step
```
