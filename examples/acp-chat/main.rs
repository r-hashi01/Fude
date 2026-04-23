//! acp-chat — a chat UI that drives an ACP (Agent Client Protocol) agent.
//!
//! Lets the user pick a project folder, starts an ACP session rooted there,
//! and streams `session-update` events back to the web UI while the agent
//! works. The agent's own `fs/*` calls are sandboxed by fude to the
//! allow-listed folder.
//!
//! Run from the repo root:
//!
//! ```sh
//! cargo run --example acp-chat --release
//! ```
//!
//! Requires an ACP adapter binary on `PATH` (e.g. `claude-code-acp`).

use fude::{acp::AcpAdapterConfig, App};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new("dev.example.acp-chat")
        .title("acp-chat")
        .assets(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/acp-chat/dist"
        ))
        .with_fs_sandbox()
        .with_dialogs()
        .with_acp(
            vec![AcpAdapterConfig {
                name: "claude-code".into(),
                candidate_bin_names: vec!["claude-code-acp".into()],
            }],
            "fude-example-acp-chat",
            env!("CARGO_PKG_VERSION"),
        )
        .run()
}
