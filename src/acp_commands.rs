//! Synchronous dispatch wrappers for ACP IPC commands.
//! Runs the underlying async ops on a dedicated current-thread tokio runtime.

use std::future::Future;
use std::sync::Arc;

use serde_json::Value;
use tokio::runtime::Handle;

use crate::acp::{ensure_acp, AcpState};
use crate::events::EventEmitter;
use crate::fs::FsState;

pub struct AcpCtx {
    pub state: Arc<AcpState>,
    pub handle: Handle,
}

impl AcpCtx {
    pub fn new(state: Arc<AcpState>) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("fude-acp-rt".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_io()
                    .enable_time()
                    .build()
                    .expect("tokio runtime");
                tx.send(rt.handle().clone()).ok();
                rt.block_on(std::future::pending::<()>());
            })
            .expect("spawn fude-acp-rt");
        let handle = rx.recv().expect("tokio handle");
        Self { state, handle }
    }

    pub fn run<F, T>(&self, fut: F) -> T
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        self.handle.spawn(async move {
            let _ = tx.send(fut.await);
        });
        rx.recv().expect("tokio task panicked")
    }
}

fn arg_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("missing arg: {}", key))
}

fn require_fs(fs: Option<&Arc<FsState>>) -> Result<&Arc<FsState>, String> {
    fs.ok_or_else(|| "ACP commands require App::with_fs_sandbox".to_string())
}

pub fn get_adapter(ctx: &AcpCtx) -> Result<Value, String> {
    let state = ctx.state.clone();
    let name = ctx.run(async move { state.adapter.lock().await.clone() });
    Ok(Value::from(name))
}

pub fn set_adapter(ctx: &AcpCtx, args: &Value) -> Result<Value, String> {
    let next = arg_str(args, "adapter")?.to_string();
    if ctx.state.find_adapter(&next).is_none() {
        return Err(format!("Unknown ACP adapter: {}", next));
    }
    let state = ctx.state.clone();
    let next_for_rt = next.clone();
    ctx.run(async move {
        let mut adapter_guard = state.adapter.lock().await;
        if *adapter_guard != next_for_rt {
            *adapter_guard = next_for_rt;
            let mut process_guard = state.process.lock().await;
            if let Some(acp) = process_guard.take() {
                acp.kill().await;
            }
        }
    });
    Ok(Value::from(next))
}

pub fn initialize(
    ctx: &AcpCtx,
    fs: Option<&Arc<FsState>>,
    emitter: &EventEmitter,
) -> Result<Value, String> {
    let fs = require_fs(fs)?;
    let state = ctx.state.clone();
    let ap = fs.allowed_paths.clone();
    let ad = fs.allowed_dirs.clone();
    let em = emitter.clone();
    let client_name = state.client_name.clone();
    let version = state.client_version.clone();
    ctx.run(async move {
        let acp = ensure_acp(&state, em, ap, ad).await?;
        acp.request(
            "initialize",
            serde_json::json!({
                "protocolVersion": 1,
                "clientInfo": { "name": client_name, "title": client_name, "version": version },
                "clientCapabilities": {
                    "fs": { "readTextFile": true, "writeTextFile": true }
                }
            }),
        )
        .await
    })
}

pub fn new_session(
    ctx: &AcpCtx,
    fs: Option<&Arc<FsState>>,
    emitter: &EventEmitter,
    args: &Value,
) -> Result<Value, String> {
    let fs = require_fs(fs)?;
    let cwd = arg_str(args, "cwd")?.to_string();
    let state = ctx.state.clone();
    let ap = fs.allowed_paths.clone();
    let ad = fs.allowed_dirs.clone();
    let em = emitter.clone();
    ctx.run(async move {
        let acp = ensure_acp(&state, em, ap, ad).await?;
        acp.request(
            "session/new",
            serde_json::json!({ "cwd": cwd, "mcpServers": [] }),
        )
        .await
    })
}

pub fn prompt(
    ctx: &AcpCtx,
    fs: Option<&Arc<FsState>>,
    emitter: &EventEmitter,
    args: &Value,
) -> Result<Value, String> {
    let fs = require_fs(fs)?;
    let session_id = arg_str(args, "sessionId")?.to_string();
    let prompt = arg_str(args, "prompt")?.to_string();
    let state = ctx.state.clone();
    let ap = fs.allowed_paths.clone();
    let ad = fs.allowed_dirs.clone();
    let em = emitter.clone();
    ctx.run(async move {
        let acp = ensure_acp(&state, em, ap, ad).await?;
        acp.request(
            "session/prompt",
            serde_json::json!({
                "sessionId": session_id,
                "prompt": [{ "type": "text", "text": prompt }]
            }),
        )
        .await
    })
}

pub fn cancel(
    ctx: &AcpCtx,
    fs: Option<&Arc<FsState>>,
    emitter: &EventEmitter,
    args: &Value,
) -> Result<Value, String> {
    let fs = require_fs(fs)?;
    let session_id = arg_str(args, "sessionId")?.to_string();
    let state = ctx.state.clone();
    let ap = fs.allowed_paths.clone();
    let ad = fs.allowed_dirs.clone();
    let em = emitter.clone();
    ctx.run(async move {
        let acp = ensure_acp(&state, em, ap, ad).await?;
        acp.notify(
            "session/cancel",
            serde_json::json!({ "sessionId": session_id }),
        )
        .await?;
        Ok(Value::Null)
    })
}

pub fn set_model(
    ctx: &AcpCtx,
    fs: Option<&Arc<FsState>>,
    emitter: &EventEmitter,
    args: &Value,
) -> Result<Value, String> {
    let fs = require_fs(fs)?;
    let session_id = arg_str(args, "sessionId")?.to_string();
    let model_id = arg_str(args, "modelId")?.to_string();
    let state = ctx.state.clone();
    let ap = fs.allowed_paths.clone();
    let ad = fs.allowed_dirs.clone();
    let em = emitter.clone();
    ctx.run(async move {
        let acp = ensure_acp(&state, em, ap, ad).await?;
        acp.request(
            "session/set_model",
            serde_json::json!({ "sessionId": session_id, "modelId": model_id }),
        )
        .await
    })
}

pub fn set_config(
    ctx: &AcpCtx,
    fs: Option<&Arc<FsState>>,
    emitter: &EventEmitter,
    args: &Value,
) -> Result<Value, String> {
    let fs = require_fs(fs)?;
    let session_id = arg_str(args, "sessionId")?.to_string();
    let config_id = arg_str(args, "configId")?.to_string();
    let value = arg_str(args, "value")?.to_string();
    let state = ctx.state.clone();
    let ap = fs.allowed_paths.clone();
    let ad = fs.allowed_dirs.clone();
    let em = emitter.clone();
    ctx.run(async move {
        let acp = ensure_acp(&state, em, ap, ad).await?;
        acp.request(
            "session/set_config_option",
            serde_json::json!({ "sessionId": session_id, "configId": config_id, "value": value }),
        )
        .await
    })
}

pub fn list_sessions(
    ctx: &AcpCtx,
    fs: Option<&Arc<FsState>>,
    emitter: &EventEmitter,
    args: &Value,
) -> Result<Value, String> {
    let fs = require_fs(fs)?;
    let cwd = args
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let state = ctx.state.clone();
    let ap = fs.allowed_paths.clone();
    let ad = fs.allowed_dirs.clone();
    let em = emitter.clone();
    ctx.run(async move {
        let acp = ensure_acp(&state, em, ap, ad).await?;
        acp.request("session/list", serde_json::json!({ "cwd": cwd }))
            .await
    })
}

pub fn resume_session(
    ctx: &AcpCtx,
    fs: Option<&Arc<FsState>>,
    emitter: &EventEmitter,
    args: &Value,
) -> Result<Value, String> {
    let fs = require_fs(fs)?;
    let session_id = arg_str(args, "sessionId")?.to_string();
    let cwd = arg_str(args, "cwd")?.to_string();
    let state = ctx.state.clone();
    let ap = fs.allowed_paths.clone();
    let ad = fs.allowed_dirs.clone();
    let em = emitter.clone();
    ctx.run(async move {
        let acp = ensure_acp(&state, em, ap, ad).await?;
        acp.request(
            "session/resume",
            serde_json::json!({ "sessionId": session_id, "cwd": cwd, "mcpServers": [] }),
        )
        .await
    })
}

pub fn shutdown(ctx: &AcpCtx) -> Result<Value, String> {
    let state = ctx.state.clone();
    ctx.run(async move {
        let mut guard = state.process.lock().await;
        if let Some(acp) = guard.take() {
            acp.kill().await;
        }
    });
    Ok(Value::Null)
}
