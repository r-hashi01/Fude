//! ipc-hello — the absolute minimum fude app.
//!
//! A window, a web page, and two custom commands. No filesystem, no dialogs,
//! no agents. This is what `fude` gives you before you opt in to anything.
//!
//! Run from the repo root:
//!
//! ```sh
//! cargo run --example ipc-hello --release
//! ```

use fude::App;
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let counter = Arc::new(AtomicU64::new(0));
    let counter_for_cmd = counter.clone();

    App::new("dev.example.ipc-hello")
        .title("ipc-hello")
        .assets(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/ipc-hello/dist"
        ))
        .command("ping", |_ctx, _args| Ok(json!("pong")))
        .command("echo", |_ctx, args| Ok(args.clone()))
        .command("increment", move |_ctx, _args| {
            let n = counter_for_cmd.fetch_add(1, Ordering::Relaxed) + 1;
            Ok(json!(n))
        })
        .run()
}
