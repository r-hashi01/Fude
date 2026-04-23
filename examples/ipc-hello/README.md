# ipc-hello

The absolute minimum fude app: a window, a web page, and two custom
commands. No filesystem, no dialogs, no agents. Use this as the mental
model for what `fude` gives you before you opt in to anything.

## Run

From the repo root:

```sh
cargo run --example ipc-hello --release
```

Three buttons:

- **Ping** — calls `ping` command, prints `"pong"`.
- **Echo** — sends JSON to Rust, prints it back unchanged.
- **Increment** — a counter that lives in Rust (`AtomicU64`); the value
  survives across clicks and is authoritative.

## What this demonstrates

- **Core shell** — `App::new(..).command(..).run()` is enough. `asset://`
  serves the HTML, `window.__shell_ipc` calls into Rust.
- **Stateful commands** — closures can capture `Arc<…>` to share state
  across invocations (see the counter).
- **JSON round-trip** — `echo` proves arbitrary JSON traverses the bridge
  without loss.

## Files

```
main.rs           ~25 lines
dist/index.html   vanilla HTML/JS
```
