# acp-chat

A chat UI that drives an [ACP (Agent Client Protocol)][acp] agent. Lets
the user pick a project folder, starts an ACP session rooted there, and
streams `session-update` events back to the web UI while the agent works.

The agent's own `fs/read_text_file` / `fs/write_text_file` calls are
intercepted by fude and rejected unless the target is inside the folder
the user explicitly chose — the same sandbox that guards the user's own
IPC calls applies to the agent.

## Prerequisites

You need an ACP adapter binary on your `PATH`. This example is configured
to look for `claude-code-acp`:

```rust
vec![AcpAdapterConfig {
    name: "claude-code".into(),
    candidate_bin_names: vec!["claude-code-acp".into()],
}]
```

### Installing `claude-code-acp`

Install via npm:

```sh
npm install -g @zed-industries/claude-code-acp
```

Verify it resolves:

```sh
which claude-code-acp
# → /opt/homebrew/bin/claude-code-acp  (or similar)
```

Authenticate Claude Code the usual way (the adapter reuses your existing
Claude Code session — if `claude` CLI works, the adapter will too).

### Using a different adapter

ACP is a spec, not a single binary. If you're running a different agent
(Zed's own adapter, a local model wrapper, etc.), edit `main.rs`:

```rust
vec![AcpAdapterConfig {
    name: "my-agent".into(),
    candidate_bin_names: vec!["my-agent-acp".into(), "my-agent".into()],
}]
```

fude tries each name in order against `PATH` plus a short list of trusted
install dirs (`/opt/homebrew/bin`, `~/.cargo/bin`, `~/.npm-global/bin`, …).

## Run

From the repo root:

```sh
cargo run --example acp-chat --release
```

### Flow

1. **Choose project folder…** — pick a directory. This becomes the
   sandbox root; the agent cannot read or write outside it.
2. **Start session** — fude locates the adapter, spawns it as a child
   process, sends `initialize` and `session/new` over JSON-RPC stdio.
   The UI turns on the input box when the session is ready.
3. **Type a prompt, Send.** fude forwards it via `session/prompt`.
4. **Watch the chat pane.** The agent streams `session/update`
   notifications back — `agent_message_chunk` updates render as chat
   text; tool calls, thoughts, and other update types are shown raw as
   JSON so you can see the full protocol.
5. Ask the agent to edit a file in the folder. Fude will intercept its
   `fs/write_text_file` call and apply to disk under the allow-list. Try
   asking it to edit `/etc/hosts` — you'll see the sandbox reject it.

## What this demonstrates

- **`with_acp(adapters, client_name, version)`** — registers the 11
  `acp_*` commands the frontend uses.
- **Adapter resolution** — fude tries each `candidate_bin_names` against
  `PATH` and trusted install dirs.
- **Sandbox composition** — `with_acp` requires `with_fs_sandbox`; the
  agent's filesystem calls pass through the same allow-list as the user's
  own `read_file` / `write_file`.
- **Server-push events** — `window.__shell_listen("acp:session-update",
  fn)` subscribes to the streaming agent output.

## Files

```
main.rs           ~25 lines: App builder + with_acp
dist/index.html   chat UI, vanilla HTML/JS
```

[acp]: https://agentclientprotocol.com
