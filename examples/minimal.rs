//! Minimum viable fude app: a window that serves `./dist/` and exposes a
//! single `ping` IPC command.
//!
//! Run with:
//!
//! ```sh
//! mkdir -p dist
//! echo '<!doctype html><script>window.__shell_ipc("ping").then(console.log)</script>' > dist/index.html
//! cargo run --example minimal
//! ```

use fude::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new("dev.fude.example.minimal")
        .title("fude — minimal")
        .assets("./dist")
        .command("ping", |_ctx, _args| Ok(serde_json::json!("pong")))
        .run()
}
