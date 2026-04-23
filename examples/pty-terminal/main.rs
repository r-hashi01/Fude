//! pty-terminal — spawns an allow-listed CLI tool in a PTY and streams
//! its output to a web UI.
//!
//! Pick a project folder, choose a tool from the allow-list, and fude
//! spawns it with that folder as cwd. Output streams back as `pty:data`
//! events (base64). Input typed in the web UI is sent through `pty_write`.
//!
//! Run from the repo root:
//!
//! ```sh
//! cargo run --example pty-terminal --release
//! ```
//!
//! Requires at least one of the allow-listed tools (`claude`, `codex`,
//! `bash`) on your `PATH`.

use fude::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new("dev.example.pty-terminal")
        .title("pty-terminal")
        .assets(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/pty-terminal/dist"
        ))
        .with_fs_sandbox()
        .with_dialogs()
        .with_pty(&["claude", "codex", "bash"])
        .run()
}
