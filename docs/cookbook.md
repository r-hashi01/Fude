# Cookbook

Practical patterns for building on fude. Each recipe is self-contained
and ~30 lines or less.

## Sharing state between commands

Commands receive `&Ctx` but `Ctx` only holds built-ins. For app state,
capture an `Arc` in each closure:

```rust
use std::sync::{Arc, Mutex};
use fude::App;

struct MyState { counter: Mutex<u64> }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(MyState { counter: Mutex::new(0) });

    let s = state.clone();
    App::new("dev.example")
        .assets("./dist")
        .command("inc", move |_ctx, _args| {
            let mut n = s.counter.lock().unwrap();
            *n += 1;
            Ok(serde_json::json!(*n))
        })
        .run()
}
```

For write-heavy state, prefer `RwLock` over `Mutex`. For cross-command
coordination, use `tokio::sync::mpsc` — tokio is already a fude dep.

## Rendering local images / PDFs in the webview

Do **not** base64 files into HTML; that copies the bytes, locks them into
memory, and blocks for large files. Use the asset URL helper:

```js
// frontend
const picked = await window.__shell_ipc("dialog_open", {
  filters: [{ name: "Images", extensions: ["png", "jpg", "webp"] }],
});
if (picked) {
  await window.__shell_ipc("allow_path", { path: picked });
  document.querySelector("img").src = window.__shell_asset_url(picked);
}
```

The file is streamed by fude's `asset://` handler only if its canonical
path is in the FS allow-list. Non-allow-listed paths return HTTP 403.
Works for any `Content-Type` the mime detector knows (see
[frontend-bridge.md](./frontend-bridge.md)).

## Opening links in the OS default browser

```js
await window.__shell_ipc("shell_open", { target: "https://example.com" });
```

For `mailto:`:

```js
await window.__shell_ipc("shell_open", { target: "mailto:alice@example.com" });
```

For an allow-listed local file (opens in the OS default app — useful for
draw.io / PDFs not renderable inline):

```js
await window.__shell_ipc("shell_open", { target: "/Users/alice/notes/doc.pdf" });
```

Reminder: the path must already be in the allow-list, and only
`http`/`https`/`mailto` URL schemes are accepted. `file://`,
`javascript:`, `data:`, and custom schemes are refused.

## Tauri → fude migration map

If you're porting a Tauri app, most APIs have a one-to-one analogue:

| Tauri                                    | fude                                        |
| ---------------------------------------- | ------------------------------------------- |
| `invoke("cmd", args)`                    | `window.__shell_ipc("cmd", args)`           |
| `listen("event", fn)`                    | `window.__shell_listen("event", fn)`        |
| `convertFileSrc(path)`                   | `window.__shell_asset_url(path)`            |
| `plugin-fs` `readTextFile` / `writeFile` | `read_file` / `write_file`                  |
| `plugin-dialog` `open` / `save`          | `dialog_open` / `dialog_save`               |
| `plugin-dialog` `ask` / `message`        | `dialog_ask` / `dialog_message`             |
| `plugin-shell` `open`                    | `shell_open`                                |
| `plugin-store` / ad-hoc fs               | `load_settings` / `save_settings`           |
| `appConfigDir()` / `appDataDir()`        | `fude::app_config_dir` / `app_data_dir`     |
| `#[tauri::command]` fn                   | `.command("name", handler)`                 |
| `capabilities/*.json`                    | compile-time `with_*` methods on `App`      |
| plugin ecosystem                         | add a Rust crate + register commands        |
| auto-updater                             | (not supported; use Tauri if needed)        |
| tray / global shortcut                   | (not supported; out of scope)               |

Typical migration flow:
1. Replace `@tauri-apps/api/core`'s `invoke` with `window.__shell_ipc`
   everywhere. They accept the same argument shape.
2. Replace `@tauri-apps/plugin-dialog` / `@tauri-apps/plugin-shell`
   imports with the IPC command equivalents above.
3. Replace `src-tauri/src/main.rs` with a fude `App` builder chain.
   Custom `#[tauri::command]` functions become closures registered via
   `.command(...)`.
4. Drop the `capabilities/` directory — fude's model is compile-time
   opt-in via `with_fs_sandbox` etc.
5. Delete Tauri's auto-updater / tray / global-shortcut plugin uses.
   If you need them, fude is the wrong choice — stay on Tauri.

## Using a non-ACP AI (HTTP API, local model, …)

ACP is the supported protocol for agent integration, but you're not
required to use it. Drive a custom backend with a plain `command`:

```rust
use tokio::sync::Mutex;
use std::sync::Arc;

struct Client { api_key: String, http: reqwest::Client }

let client = Arc::new(Mutex::new(Client { … }));

App::new("dev.example")
    .assets("./dist")
    .with_fs_sandbox()
    .command("ask", {
        let c = client.clone();
        move |ctx, args| {
            let prompt = args.get("prompt").and_then(|v| v.as_str())
                .ok_or("missing prompt")?;
            let emitter = ctx.emitter.clone();
            // tokio runtime is already present; use current_thread
            // if the handler doesn't need it, else spawn
            let answer = tokio::runtime::Handle::current().block_on(async {
                // … call your HTTP API, stream chunks via emitter …
                Ok::<_, String>("done".to_string())
            })?;
            Ok(serde_json::json!(answer))
        }
    })
    .run()
```

Streaming: emit partial results via `ctx.emitter.emit("ask:chunk",
payload)` and subscribe on the frontend with `window.__shell_listen`.

## Creating a scratch directory for generated assets

Apps that generate files (screenshots, thumbnails, temp exports) need a
writable directory under the sandbox. `ensure_scratch_dir` creates it
and adds it to the allow-list:

```rust
App::new("dev.example")
    .assets("./dist")
    .with_fs_sandbox()
    .command("save_thumbnail", |ctx, args| {
        let dir = fude::ensure_scratch_dir(ctx, "thumbnails")?;
        // … now dir is allow-listed, write_file / write_file_binary work
        Ok(serde_json::json!(dir.to_string_lossy()))
    })
    .run()
```

`name` is validated — no path separators, no `..`, no leading `.`.

## Validating paths in your own commands

If you register a command that takes a file path from the frontend,
apply the same check the built-in commands use:

```rust
use fude::{is_path_allowed, FsState};

.command("my_read", |ctx, args| {
    let fs: &FsState = ctx.fs.as_ref().ok_or("need fs sandbox")?;
    let path = args.get("path").and_then(|v| v.as_str())
        .ok_or("missing path")?;
    let canonical = is_path_allowed(path, &fs.allowed_paths, &fs.allowed_dirs)?;
    // canonical is safe to read/write
    Ok(serde_json::json!(std::fs::read_to_string(canonical).map_err(|e| e.to_string())?))
})
```

Never use the raw user-supplied path — always pass through
`is_path_allowed` (or `is_dir_allowed`) first. The block-list and
symlink-escape checks run inside.
