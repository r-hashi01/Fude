# Frontend bridge

The JavaScript API that fude exposes to your web frontend. Everything
below is available in the webview after fude's initialization script runs
(before any page `<script>` tag executes).

## Global helpers

### `window.__shell_ipc(cmd, args?) → Promise<any>`

Invoke a command registered in Rust via `App::command` or one of the
`with_*` helpers. Arguments are serialised to JSON; the response is
parsed JSON.

```js
const result = await window.__shell_ipc("read_file", { path });
```

Rejects with `Error(message)` on any Rust-side error.

### `window.__shell_listen(name, fn) → Unlisten`

Subscribe to a server-push event. Returns a function; call it to
unsubscribe.

```js
const off = window.__shell_listen("pty:data", ({ id, data }) => { … });
// later
off();
```

### `window.__shell_asset_url(absolutePath) → string`

Build a URL the webview can render directly. Equivalent to Tauri's
`convertFileSrc`. The file is served only when its canonical path is in
the FS allow-list at request time.

```js
img.src = window.__shell_asset_url("/Users/alice/notes/cover.png");
// → asset://localhost/__file/%2FUsers%2Falice%2Fnotes%2Fcover.png
```

A non-allow-listed path returns HTTP 403 from the `asset://` handler.

## IPC commands

Grouped by the `App::with_*` method that registers them.

### `App::with_fs_sandbox`

All paths must be absolute. Reads / writes are refused unless the path
(or one of its parents) is in the allow-list — see
[sandbox.md](./sandbox.md) for exact semantics.

| Command             | Args                                         | Returns              |
| ------------------- | -------------------------------------------- | -------------------- |
| `allow_path`        | `{ path }`                                   | `null`               |
| `allow_dir`         | `{ path }`                                   | `null`               |
| `list_directory`    | `{ path }`                                   | `[{ name, is_dir }]` |
| `read_file`         | `{ path }`                                   | `string` (utf-8)     |
| `read_file_binary`  | `{ path }`                                   | `string` (base64)    |
| `write_file`        | `{ path, content }`                          | `null`               |
| `write_file_binary` | `{ path, content }` (content: base64)        | `null`               |
| `ensure_dir`        | `{ path }`                                   | `null`               |

### `App::with_dialogs`

All four run on the UI thread via `MainDispatcher` and block the caller
until the user dismisses the dialog.

| Command          | Args                                                       | Returns            |
| ---------------- | ---------------------------------------------------------- | ------------------ |
| `dialog_open`    | `{ directory?, multiple?, filters?, defaultPath? }`        | `string \| string[] \| null` |
| `dialog_save`    | `{ filters?, defaultPath? }`                               | `string \| null`   |
| `dialog_ask`     | `{ message, title?, okLabel?, cancelLabel? }`              | `boolean`          |
| `dialog_message` | `{ message, title?, kind? }` (kind: `info` / `warning` / `error`) | `null`       |

`filters`: `[{ name, extensions: [...] }]`.

### `App::with_pty`

Spawn CLI tools in a PTY. Requires `with_fs_sandbox` — the `cwd` must
live inside an allow-listed directory. Only binary names in the app's
`with_pty(&[...])` allow-list may be spawned.

| Command      | Args                                        | Returns       |
| ------------ | ------------------------------------------- | ------------- |
| `pty_spawn`  | `{ tool, cwd, cols?, rows? }`               | `number` (id) |
| `pty_write`  | `{ id, data }` (raw string, ≤ 1 MiB)        | `null`        |
| `pty_resize` | `{ id, cols, rows }`                        | `null`        |
| `pty_kill`   | `{ id }`                                    | `null`        |

Events:
- `pty:data` → `{ id, data }` where `data` is base64-encoded raw bytes.
- `pty:exit` → `{ id }`.

### `App::with_acp`

Agent Client Protocol client over JSON-RPC stdio. Requires
`with_fs_sandbox`. Adapter binary is located against `PATH` plus a short
list of trusted install dirs.

| Command              | Args                                                      | Returns                     |
| -------------------- | --------------------------------------------------------- | --------------------------- |
| `acp_get_adapter`    | `{}`                                                      | `{ name, bin }`             |
| `acp_set_adapter`    | `{ name }`                                                | `null`                      |
| `acp_initialize`     | `{}`                                                      | adapter `initialize` result |
| `acp_new_session`    | `{ cwd }`                                                 | `{ sessionId }`             |
| `acp_prompt`         | `{ sessionId, prompt }`                                   | `null` (response streams)   |
| `acp_cancel`         | `{ sessionId }`                                           | `null`                      |
| `acp_set_model`      | `{ sessionId, model }`                                    | `null`                      |
| `acp_set_config`     | `{ sessionId, config }`                                   | `null`                      |
| `acp_list_sessions`  | `{}`                                                      | `[{ sessionId, … }]`        |
| `acp_resume_session` | `{ sessionId, cwd }`                                      | `null`                      |
| `acp_shutdown`       | `{}`                                                      | `null`                      |

Events:
- `acp:session-update` → the raw ACP `session/update` notification
  (`{ sessionId, update: { sessionUpdate, … } }`). `agent_message_chunk`
  is the common streaming-text shape; tool calls, thoughts, etc. arrive
  under different `sessionUpdate` values.
- `acp:permission-request` → emitted when the agent requests permission
  for an unsafe kind. fude auto-allows `read` / `edit` / `think` /
  `search`; anything else is surfaced so the UI can decide.

### `App::with_shell_open`

| Command      | Args         | Returns |
| ------------ | ------------ | ------- |
| `shell_open` | `{ target }` | `null`  |

`target` may be:
- `http://…`, `https://…`, `mailto:…` URL — passed as-is to the OS opener.
- An allow-listed absolute file path — requires `with_fs_sandbox` and
  the path must already be in the allow-list.

Any other scheme (`file://`, `javascript:`, `data:`, custom) is refused.

### `App::with_settings`

JSON settings persisted at `<app_config_dir>/settings.json` via atomic
write. The payload must be a JSON object — arrays / scalars are rejected.

| Command         | Args                     | Returns       |
| --------------- | ------------------------ | ------------- |
| `load_settings` | `{}`                     | `object` (`{}` if no file yet) |
| `save_settings` | `{ settings: { … } }`    | `null`        |

Schema-less on purpose — apps validate their own shape before calling
`save_settings`.
