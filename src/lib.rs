//! # fude (筆)
//!
//! A minimal wry + tao shell for AI-assisted document editors. The brush:
//! the tool you reach for to write with an AI as co-author. A lightweight
//! alternative to Tauri for this narrow use case.
//!
//! - Boots a single-window webview loading `asset://localhost/` from a local
//!   `dist/` directory
//! - Exposes `window.__shell_ipc(cmd, args) => Promise` to the frontend
//!   with JSON request/reply semantics, plus `window.__shell_listen(name, fn)`
//!   for server-push events
//! - Ships opt-in building blocks for allow-list file I/O and native dialogs
//! - Leaves app-specific commands (PTY for AI CLI panes, ACP clients, custom
//!   business logic) to the consumer via [`App::command`]
//!
//! ## Example
//!
//! ```no_run
//! use fude::App;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     App::new("com.example.my-editor")
//!         .title("My Editor")
//!         .assets("./dist")
//!         .with_fs_sandbox()
//!         .with_dialogs()
//!         .command("ping", |_ctx, _args| Ok(serde_json::json!("pong")))
//!         .run()
//! }
//! ```

pub mod acp;
pub mod acp_commands;
mod assets;
pub mod dialogs;
pub mod events;
pub mod fs;
pub mod pty;
pub mod sandbox;
pub mod settings;
pub mod shell;

pub use acp::AcpAdapterConfig;
pub use events::EventEmitter;
pub use fs::FsState;
pub use sandbox::{
    app_config_dir, app_data_dir, atomic_write, ensure_scratch, is_dir_allowed, is_path_allowed,
    new_list, safe_lock, validate_path, SharedList,
};

/// Create and allow-list `<app_data_dir>/<name>`. Convenience wrapper
/// around [`ensure_scratch`] for app-owned scratch directories
/// (e.g. `"temp-images"`, `"cache"`).
///
/// Requires [`App::with_fs_sandbox`].
pub fn ensure_scratch_dir(ctx: &Ctx, name: &str) -> Result<std::path::PathBuf, String> {
    let fs = ctx
        .fs
        .as_ref()
        .ok_or("ensure_scratch_dir requires with_fs_sandbox")?;
    let data = sandbox::app_data_dir(&ctx.identifier)?;
    sandbox::ensure_scratch(&data, &fs.allowed_dirs, name)
}

/// Returns an `asset://` URL that streams an allow-listed local file to
/// the webview. Equivalent to `window.__shell_asset_url(path)` on the
/// frontend: path is percent-encoded and suffixed onto
/// `asset://localhost/__file/`. The file is served only when its
/// canonical path is in the `FsState` allow-list at request time; calling
/// this function does not grant access on its own.
///
/// The file is only served if its canonical path is in the
/// [`FsState`] allow-list at request time. Calling this function does
/// **not** grant access on its own.
pub fn asset_url_from_file(path: &str) -> String {
    let mut out = String::from("asset://localhost/__file/");
    for b in path.as_bytes() {
        match *b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*b as char);
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

#[doc(hidden)]
pub enum UserEvent {
    IpcReply(String),
    Eval(String),
    RunOnMain(Box<dyn FnOnce() + Send>),
}

/// Dispatches a closure to run on the main (UI) thread and blocks the
/// caller until it completes. Required on macOS for native dialogs —
/// AppKit `NSOpenPanel` / `NSSavePanel` / `NSAlert` refuse to run from
/// background threads.
#[derive(Clone)]
pub struct MainDispatcher {
    proxy: tao::event_loop::EventLoopProxy<UserEvent>,
}

impl MainDispatcher {
    /// Run `f` on the main thread and return its result. Blocks the
    /// caller. Returns `Err` if the event loop has already exited.
    pub fn run<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        let boxed: Box<dyn FnOnce() + Send> = Box::new(move || {
            let _ = tx.send(f());
        });
        self.proxy
            .send_event(UserEvent::RunOnMain(boxed))
            .map_err(|_| "event loop closed".to_string())?;
        rx.recv()
            .map_err(|_| "main thread dropped result".to_string())
    }
}

/// Handler signature for commands registered via [`App::command`].
pub type CommandHandler = Arc<dyn Fn(&Ctx, &Value) -> Result<Value, String> + Send + Sync>;

/// Runtime context passed to every command. Provides the app identifier,
/// the shared [`EventEmitter`] for server-push events, and the allow-list
/// state if `with_fs_sandbox` was called.
pub struct Ctx {
    pub identifier: String,
    pub emitter: EventEmitter,
    pub fs: Option<Arc<FsState>>,
    pub main: MainDispatcher,
}

/// Builder for a fude application.
pub struct App {
    identifier: String,
    title: String,
    asset_root: PathBuf,
    commands: HashMap<String, CommandHandler>,
    fs_state: Option<Arc<FsState>>,
    pty_sessions: Option<Arc<pty::PtySessions>>,
    acp_ctx: Option<Arc<acp_commands::AcpCtx>>,
}

impl App {
    /// Create a new app. `identifier` is a reverse-DNS string used to locate
    /// the config / data directory (e.g. `"com.example.editor"`).
    pub fn new(identifier: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            title: String::from("fude"),
            asset_root: assets::default_root(),
            commands: HashMap::new(),
            fs_state: None,
            pty_sessions: None,
            acp_ctx: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Directory served as the root of `asset://localhost/`. Typically
    /// your Vite/webpack `dist/` output.
    pub fn assets(mut self, root: impl Into<PathBuf>) -> Self {
        self.asset_root = root.into();
        self
    }

    /// Register a custom IPC command.
    pub fn command<F>(mut self, name: impl Into<String>, handler: F) -> Self
    where
        F: Fn(&Ctx, &Value) -> Result<Value, String> + Send + Sync + 'static,
    {
        self.commands.insert(name.into(), Arc::new(handler));
        self
    }

    /// Register the built-in allow-list FS commands:
    /// `allow_path`, `allow_dir`, `list_directory`, `read_file`,
    /// `read_file_binary`, `write_file`, `write_file_binary`, `ensure_dir`.
    ///
    /// The frontend must call `allow_path` / `allow_dir` after a native
    /// dialog selection — no file I/O is permitted until then.
    pub fn with_fs_sandbox(mut self) -> Self {
        let state = Arc::new(FsState {
            allowed_paths: new_list(),
            allowed_dirs: new_list(),
        });
        self.fs_state = Some(state.clone());

        let s = state.clone();
        self = self.command("allow_path", move |_ctx, args| fs::allow_path(&s, args));
        let s = state.clone();
        self = self.command("allow_dir", move |_ctx, args| fs::allow_dir(&s, args));
        let s = state.clone();
        self = self.command("list_directory", move |_ctx, args| {
            fs::list_directory(&s, args)
        });
        let s = state.clone();
        self = self.command("read_file", move |_ctx, args| fs::read_file(&s, args));
        let s = state.clone();
        self = self.command("read_file_binary", move |_ctx, args| {
            fs::read_file_binary(&s, args)
        });
        let s = state.clone();
        self = self.command("write_file", move |_ctx, args| fs::write_file(&s, args));
        let s = state.clone();
        self = self.command("write_file_binary", move |_ctx, args| {
            fs::write_file_binary(&s, args)
        });
        let s = state.clone();
        self = self.command("ensure_dir", move |_ctx, args| fs::ensure_dir(&s, args));
        self
    }

    /// Register `load_settings` / `save_settings` — persist an arbitrary
    /// JSON object at `<app_config_dir>/settings.json` via
    /// [`crate::atomic_write`].
    ///
    /// `load_settings()` returns the parsed object, or `{}` if none saved.
    /// `save_settings({ settings: {...} })` writes atomically. The
    /// payload must be a JSON object; arrays / scalars are rejected.
    pub fn with_settings(mut self) -> Self {
        self = self.command("load_settings", |ctx, _args| settings::load(ctx));
        self = self.command("save_settings", settings::save);
        self
    }

    /// Register `shell_open` — opens URLs (`http`/`https`/`mailto`) or
    /// allow-listed local file paths in the OS default application.
    ///
    /// File-path targets require [`App::with_fs_sandbox`] and must
    /// already be in the allow-list (via a dialog or manual `allow_path`).
    pub fn with_shell_open(mut self) -> Self {
        self = self.command("shell_open", |ctx, args| {
            shell::open(args, ctx.fs.as_deref())
        });
        self
    }

    /// Register native dialog commands: `dialog_open`, `dialog_save`,
    /// `dialog_ask`, `dialog_message`.
    pub fn with_dialogs(mut self) -> Self {
        self = self.command("dialog_open", |ctx, args| dialogs::open(&ctx.main, args));
        self = self.command("dialog_save", |ctx, args| dialogs::save(&ctx.main, args));
        self = self.command("dialog_ask", |ctx, args| dialogs::ask(&ctx.main, args));
        self = self.command("dialog_message", |ctx, args| {
            dialogs::message(&ctx.main, args)
        });
        self
    }

    /// Register PTY commands for spawning CLI agents (e.g. `claude`, `codex`).
    ///
    /// Registers: `pty_spawn`, `pty_write`, `pty_resize`, `pty_kill` and
    /// emits `pty:data` / `pty:exit` events.
    ///
    /// `allowed_tools` is the only set of binaries that may ever be spawned;
    /// anything else is rejected. Requires [`App::with_fs_sandbox`] to have
    /// been called — the PTY cwd must live inside an allow-listed directory.
    pub fn with_pty(mut self, allowed_tools: &[&str]) -> Self {
        let cfg = pty::PtyConfig {
            allowed_tools: allowed_tools.iter().map(|s| s.to_string()).collect(),
        };
        let sessions = Arc::new(pty::PtySessions::new(cfg));
        self.pty_sessions = Some(sessions.clone());

        let s = sessions.clone();
        self = self.command("pty_spawn", move |ctx, args| {
            let fs = ctx
                .fs
                .as_ref()
                .ok_or("pty_spawn requires with_fs_sandbox")?;
            pty::spawn(&s, &fs.allowed_dirs, &ctx.emitter, args)
        });
        let s = sessions.clone();
        self = self.command("pty_write", move |_ctx, args| pty::write(&s, args));
        let s = sessions.clone();
        self = self.command("pty_resize", move |_ctx, args| pty::resize(&s, args));
        let s = sessions.clone();
        self = self.command("pty_kill", move |_ctx, args| pty::kill(&s, args));
        self
    }

    /// Register ACP (Agent Client Protocol) commands.
    ///
    /// Registers 11 `acp_*` commands. Requires [`App::with_fs_sandbox`] — the
    /// agent's `fs/read_text_file` / `fs/write_text_file` calls are
    /// intercepted and reject anything outside the user's allow-list.
    pub fn with_acp(
        mut self,
        adapters: Vec<AcpAdapterConfig>,
        client_name: impl Into<String>,
        client_version: impl Into<String>,
    ) -> Self {
        let state = Arc::new(acp::AcpState::new(
            adapters,
            client_name.into(),
            client_version.into(),
        ));
        let acp_ctx = Arc::new(acp_commands::AcpCtx::new(state));
        self.acp_ctx = Some(acp_ctx.clone());

        let c = acp_ctx.clone();
        self = self.command("acp_get_adapter", move |_ctx, _args| {
            acp_commands::get_adapter(&c)
        });
        let c = acp_ctx.clone();
        self = self.command("acp_set_adapter", move |_ctx, args| {
            acp_commands::set_adapter(&c, args)
        });
        let c = acp_ctx.clone();
        self = self.command("acp_initialize", move |ctx, _args| {
            acp_commands::initialize(&c, ctx.fs.as_ref(), &ctx.emitter)
        });
        let c = acp_ctx.clone();
        self = self.command("acp_new_session", move |ctx, args| {
            acp_commands::new_session(&c, ctx.fs.as_ref(), &ctx.emitter, args)
        });
        let c = acp_ctx.clone();
        self = self.command("acp_prompt", move |ctx, args| {
            acp_commands::prompt(&c, ctx.fs.as_ref(), &ctx.emitter, args)
        });
        let c = acp_ctx.clone();
        self = self.command("acp_cancel", move |ctx, args| {
            acp_commands::cancel(&c, ctx.fs.as_ref(), &ctx.emitter, args)
        });
        let c = acp_ctx.clone();
        self = self.command("acp_set_model", move |ctx, args| {
            acp_commands::set_model(&c, ctx.fs.as_ref(), &ctx.emitter, args)
        });
        let c = acp_ctx.clone();
        self = self.command("acp_set_config", move |ctx, args| {
            acp_commands::set_config(&c, ctx.fs.as_ref(), &ctx.emitter, args)
        });
        let c = acp_ctx.clone();
        self = self.command("acp_list_sessions", move |ctx, args| {
            acp_commands::list_sessions(&c, ctx.fs.as_ref(), &ctx.emitter, args)
        });
        let c = acp_ctx.clone();
        self = self.command("acp_resume_session", move |ctx, args| {
            acp_commands::resume_session(&c, ctx.fs.as_ref(), &ctx.emitter, args)
        });
        let c = acp_ctx.clone();
        self = self.command("acp_shutdown", move |_ctx, _args| {
            acp_commands::shutdown(&c)
        });
        self
    }

    /// Block on the event loop. Returns only when the window is closed.
    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        run(self)
    }
}

#[derive(Deserialize)]
struct IpcRequest {
    id: u64,
    cmd: String,
    #[serde(default)]
    args: Value,
}

#[derive(Serialize)]
struct IpcResponse {
    id: u64,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

const IPC_INIT: &str = r#"
(() => {
  let nextId = 1;
  const pending = new Map();
  const listeners = new Map();
  window.__shell_on_reply = (payload) => {
    try {
      const msg = typeof payload === "string" ? JSON.parse(payload) : payload;
      const p = pending.get(msg.id);
      if (!p) return;
      pending.delete(msg.id);
      if (msg.ok) p.resolve(msg.result); else p.reject(new Error(msg.error || "ipc error"));
    } catch (e) { console.error(e); }
  };
  window.__shell_on_event = (name, payload) => {
    const set = listeners.get(name);
    if (!set) return;
    for (const fn of set) { try { fn(payload); } catch (e) { console.error(e); } }
  };
  window.__shell_ipc = (cmd, args = {}) => new Promise((resolve, reject) => {
    const id = nextId++;
    pending.set(id, { resolve, reject });
    window.ipc.postMessage(JSON.stringify({ id, cmd, args }));
  });
  window.__shell_listen = (name, fn) => {
    if (!listeners.has(name)) listeners.set(name, new Set());
    listeners.get(name).add(fn);
    return () => listeners.get(name)?.delete(fn);
  };
  window.__shell_asset_url = (path) => {
    // Maps an allow-listed absolute path to an asset:// URL the webview
    // can render directly (<img>, <video>, <iframe> src). The file is
    // served only if its canonical path is in the FS allow-list.
    return "asset://localhost/__file/" + encodeURIComponent(path);
  };
})();
"#;

fn run(app: App) -> Result<(), Box<dyn std::error::Error>> {
    let App {
        identifier,
        title,
        asset_root,
        commands,
        fs_state,
        pty_sessions: _pty_sessions,
        acp_ctx: _acp_ctx,
    } = app;
    let asset_root: Arc<Path> = Arc::from(asset_root.as_path());

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let emitter = EventEmitter::new(proxy.clone());

    let ctx = Arc::new(Ctx {
        identifier: identifier.clone(),
        emitter: emitter.clone(),
        fs: fs_state,
        main: MainDispatcher {
            proxy: proxy.clone(),
        },
    });

    let window = WindowBuilder::new().with_title(&title).build(&event_loop)?;

    let commands = Arc::new(commands);
    let commands_for_ipc = commands.clone();
    let ctx_for_ipc = ctx.clone();
    let asset_root_for_protocol = asset_root.clone();
    let fs_for_protocol = ctx.fs.clone();

    let webview = WebViewBuilder::new()
        .with_url("asset://localhost/")
        .with_initialization_script(IPC_INIT)
        .with_custom_protocol("asset".into(), move |_id, req| {
            assets::serve(
                &asset_root_for_protocol,
                fs_for_protocol.as_ref(),
                req.uri().path(),
            )
            .map(|b| b.into())
        })
        .with_ipc_handler(move |req| {
            // wry calls this handler on the main (UI) thread on macOS.
            // Offload to a worker so command handlers can dispatch back to
            // main (e.g. for native dialogs) without deadlocking.
            let body: String = req.body().to_string();
            let commands = commands_for_ipc.clone();
            let ctx = ctx_for_ipc.clone();
            let proxy = proxy.clone();
            std::thread::spawn(move || {
                let parsed: Result<IpcRequest, _> = serde_json::from_str(&body);
                let response = match parsed {
                    Ok(r) => {
                        let (ok, result, error) = match commands.get(&r.cmd) {
                            Some(handler) => match handler(&ctx, &r.args) {
                                Ok(v) => (true, Some(v), None),
                                Err(e) => (false, None, Some(e)),
                            },
                            None => (false, None, Some(format!("unknown cmd: {}", r.cmd))),
                        };
                        IpcResponse {
                            id: r.id,
                            ok,
                            result,
                            error,
                        }
                    }
                    Err(e) => IpcResponse {
                        id: 0,
                        ok: false,
                        result: None,
                        error: Some(format!("bad ipc: {e}")),
                    },
                };
                let json = serde_json::to_string(&response).unwrap_or_else(|_| "{}".into());
                let js = format!("window.__shell_on_reply({json})");
                let _ = proxy.send_event(UserEvent::IpcReply(js));
            });
        })
        .build(&window)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::UserEvent(UserEvent::IpcReply(js)) | Event::UserEvent(UserEvent::Eval(js)) => {
                let _ = webview.evaluate_script(&js);
            }
            Event::UserEvent(UserEvent::RunOnMain(f)) => f(),
            _ => {}
        }
    });
}
