//! cowrite — a minimal markdown editor built on fude.
//!
//! Demonstrates a realistic consumer setup:
//! - `with_fs_sandbox()` for allow-list FS access
//! - `with_dialogs()` for native file pickers
//! - one custom `stat` command that returns word/char counts
//!
//! Run from the repo root:
//!
//! ```sh
//! cargo run --example cowrite --release
//! ```

use fude::App;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    App::new("dev.example.cowrite")
        .title("cowrite")
        .assets(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/cowrite/dist"
        ))
        .with_fs_sandbox()
        .with_dialogs()
        .command("stat", |_ctx, args| {
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let chars = text.chars().count();
            let words = text.split_whitespace().count();
            let lines = text.lines().count();
            Ok(json!({ "chars": chars, "words": words, "lines": lines }))
        })
        .run()
}
