//! An editor shell with the full set of opt-ins: FS sandbox, native dialogs,
//! a PTY for CLI agents, and an ACP client.
//!
//! Build your web frontend into `./dist/`, then:
//!
//! ```sh
//! cargo run --example editor
//! ```
//!
//! From the frontend you can call e.g.
//!
//! ```js
//! await window.__shell_ipc("dialog_open", { directory: true });
//! await window.__shell_ipc("allow_dir", { path });
//! await window.__shell_ipc("acp_initialize");
//! await window.__shell_ipc("acp_new_session", { cwd: path });
//! await window.__shell_ipc("acp_prompt", { sessionId, prompt: "hello" });
//! ```

use fude::{acp::AcpAdapterConfig, App};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new("dev.fude.example.editor")
        .title("fude — editor")
        .assets("./dist")
        .with_fs_sandbox()
        .with_dialogs()
        .with_pty(&["claude", "codex"])
        .with_acp(
            vec![AcpAdapterConfig {
                name: "claude-code".into(),
                candidate_bin_names: vec!["claude-code-acp".into()],
            }],
            "fude-example-editor",
            env!("CARGO_PKG_VERSION"),
        )
        .command("echo", |_ctx, args| Ok(args.clone()))
        .run()
}
