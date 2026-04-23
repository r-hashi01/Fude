//! Event emitter: background threads push named events to the webview
//! via the tao event loop proxy.

use serde_json::Value;
use tao::event_loop::EventLoopProxy;

use crate::UserEvent;

#[derive(Clone)]
pub struct EventEmitter {
    proxy: EventLoopProxy<UserEvent>,
}

impl EventEmitter {
    pub(crate) fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        Self { proxy }
    }

    /// Emit a named event. The frontend receives it via
    /// `window.__shell_listen(name, fn)`.
    pub fn emit(&self, name: &str, payload: Value) {
        let payload_json = serde_json::to_string(&payload).unwrap_or_else(|_| "null".into());
        let name_json = serde_json::to_string(name).unwrap_or_else(|_| "\"\"".into());
        let js = format!(
            "window.__shell_on_event && window.__shell_on_event({name_json}, {payload_json})"
        );
        let _ = self.proxy.send_event(UserEvent::Eval(js));
    }
}
