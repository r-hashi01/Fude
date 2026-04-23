# Sandbox model

Fude's sandbox exists so a compromised or malicious frontend cannot read
or write files the user didn't explicitly grant. This document specifies
exactly what fude guarantees and what it deliberately leaves to the app.

## Three layers

All FS-facing commands (`read_file`, `write_file`, `list_directory`,
`ensure_dir`, `acp fs/*` handlers, `pty_spawn`'s cwd, `asset://__file/`
URL) run every target path through the same pipeline:

**1. Absolute-path requirement.** Relative paths are rejected at the
boundary. This blocks `../` traversal tricks before they even enter the
pipeline.

**2. Block-list (pre-canonical and post-canonical).** The path is
rejected if it starts with any of:

```
/etc          /var          /usr          /sys          /proc
/sbin         /bin          /boot         /Library
/private/etc  /private/var  /private/tmp
```

…or contains (component-wise, case-insensitive) any of:

```
.ssh   .gnupg   .gpg    .aws   .kube   .docker
.git   .netrc   .npmrc  .config/gcloud  Keychains
```

This check runs twice: once on the raw input, once on the canonicalized
path. A symlink inside an allow-listed directory that points into `/etc`
or `~/.ssh` is refused.

**3. Allow-list.** The path must be either:
- A file whose canonical path was added via `allow_path`, or
- A file whose canonical path starts with a directory added via
  `allow_dir`.

`allow_path` / `allow_dir` entries are only created through native
dialogs (`dialog_open` / `dialog_save`) selected by the user, or
deliberate app-side `App::command` registrations. A frontend cannot
silently authorize paths.

## `write_file_binary` symlink hardening

`write_file_binary` additionally refuses when the target path is an
existing symlink, regardless of where it points. This closes a
rename-over-symlink escape: without this check, `write_file_binary`
using `rename(tmp, target)` would follow the symlink and place the file
outside the allow-list. Regular `write_file` goes through `atomic_write`
which has the same guard.

## `asset://__file/` protocol

URLs of the form `asset://localhost/__file/<percent-encoded-path>` are
routed through the same allow-list check. Non-allow-listed paths return
HTTP 403 — not 404 — so the frontend can distinguish "missing" from
"refused". If `with_fs_sandbox` is not enabled, every `__file/` request
returns 403.

## PTY sub-sandbox

`pty_spawn` adds two further constraints:

- The `tool` argument must match the app's `App::with_pty(&[...])`
  allow-list (binary names only).
- The resolved binary must live in one of:
  `/opt/homebrew/bin`, `/usr/local/bin`, `/usr/bin`, `/bin`,
  `~/.cargo/bin`, `~/.local/bin`, `~/.volta/bin`, `~/.npm-global/bin`,
  `~/.bun/bin`.
- The child's `PATH` is set to that same list. The user's `PATH` is
  never honored. A frontend that somehow injects `PATH=/tmp/evil` into
  the environment cannot redirect `claude` or `codex` to a malicious
  binary.

The `cwd` passed to `pty_spawn` runs through the full allow-list check
above.

## ACP sub-sandbox

When an ACP agent sends `fs/read_text_file` or `fs/write_text_file`,
fude intercepts and handles it on the agent's behalf. The path goes
through the same three-layer pipeline. An agent that asks to read
`/etc/passwd`, or to write to `~/.ssh/authorized_keys`, gets the same
rejection as a compromised frontend would.

Permission prompts (`session/request_permission`) are auto-approved for
kinds that cannot corrupt data (`read`, `edit`, `think`, `search`) and
surfaced to the frontend as `acp:permission-request` for anything else.

## Threat model

**What fude protects against:**
- A compromised frontend (XSS in web code) reading or writing arbitrary
  files.
- A compromised frontend spawning arbitrary programs.
- An ACP agent requesting access outside the user-chosen folder.
- Rename-over-symlink escapes on write.

**What fude does NOT protect against:**
- A malicious **native** app (the Rust side) — you control that code;
  if you register a command that bypasses the allow-list, the sandbox
  is bypassed.
- Bugs in the user's shell commands (custom `App::command` handlers
  that themselves do unsafe FS work).
- Adversarial `dist/` content shipped by the app author — the sandbox
  is about runtime frontend compromise, not source trust.
- Side-channel / timing attacks on file existence.
- The user actively granting access to sensitive paths via dialogs.
  If the user picks `~/.ssh` in an open dialog, fude still refuses —
  but if they pick `~` and then a pathological file name contains no
  blocked component, that's allowed.

## Edge cases

- **Symlinks inside allow-listed dirs**: honored if the target
  canonicalizes inside the same (or another) allow-listed dir; rejected
  otherwise.
- **`/tmp` on macOS**: canonicalizes to `/private/tmp`, which is
  block-listed. Use `app_data_dir(...)` or `$HOME/…` for scratch files.
- **Case sensitivity**: block-list components compare case-insensitive
  (`.SSH` and `.ssh` both rejected). Prefix block-list compares
  case-sensitive — macOS's case-insensitive default filesystem means a
  path that looks fine may canonicalize to a block-listed form; the
  post-canonical check catches this.
- **Relative symlinks that reach outside after canonicalization**:
  rejected.
- **Non-existent paths**: `read_file` / canonicalization fails with
  `"Invalid file path"`. Writes to non-existent paths succeed only if
  the parent directory is allow-listed.
