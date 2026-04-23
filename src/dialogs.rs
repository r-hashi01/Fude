//! Native dialogs backed by rfd. Registered by [`crate::App::with_dialogs`].
//!
//! On macOS, `NSOpenPanel` / `NSSavePanel` / `NSAlert` must run on the
//! main (UI) thread. Because fude's IPC handler runs on a background
//! thread, every dialog call is dispatched via [`crate::MainDispatcher`]
//! and blocks the caller until the user dismisses the dialog.

use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use serde_json::Value;

use crate::MainDispatcher;

fn arg_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

fn with_filters(mut dialog: FileDialog, args: &Value) -> FileDialog {
    if let Some(filters) = args.get("filters").and_then(|v| v.as_array()) {
        for f in filters {
            let name = f.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let exts: Vec<&str> = f
                .get("extensions")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            if !exts.is_empty() {
                dialog = dialog.add_filter(name, &exts);
            }
        }
    }
    if let Some(dir) = arg_str(args, "defaultPath") {
        dialog = dialog.set_directory(dir);
    }
    dialog
}

pub(crate) fn open(main: &MainDispatcher, args: &Value) -> Result<Value, String> {
    let args = args.clone();
    main.run(move || {
        let directory = args
            .get("directory")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let multiple = args
            .get("multiple")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let dialog = with_filters(FileDialog::new(), &args);

        if directory {
            if multiple {
                dialog
                    .pick_folders()
                    .map(|v| {
                        Value::Array(
                            v.into_iter()
                                .map(|p| Value::from(p.to_string_lossy().to_string()))
                                .collect(),
                        )
                    })
                    .unwrap_or(Value::Null)
            } else {
                dialog
                    .pick_folder()
                    .map(|p| Value::from(p.to_string_lossy().to_string()))
                    .unwrap_or(Value::Null)
            }
        } else if multiple {
            dialog
                .pick_files()
                .map(|v| {
                    Value::Array(
                        v.into_iter()
                            .map(|p| Value::from(p.to_string_lossy().to_string()))
                            .collect(),
                    )
                })
                .unwrap_or(Value::Null)
        } else {
            dialog
                .pick_file()
                .map(|p| Value::from(p.to_string_lossy().to_string()))
                .unwrap_or(Value::Null)
        }
    })
}

pub(crate) fn save(main: &MainDispatcher, args: &Value) -> Result<Value, String> {
    let args = args.clone();
    main.run(move || {
        with_filters(FileDialog::new(), &args)
            .save_file()
            .map(|p| Value::from(p.to_string_lossy().to_string()))
            .unwrap_or(Value::Null)
    })
}

pub(crate) fn ask(main: &MainDispatcher, args: &Value) -> Result<Value, String> {
    let args = args.clone();
    main.run(move || {
        let message = arg_str(&args, "message").unwrap_or("");
        let title = arg_str(&args, "title").unwrap_or("");
        let ok_label = arg_str(&args, "okLabel");
        let cancel_label = arg_str(&args, "cancelLabel");
        let buttons = match (ok_label, cancel_label) {
            (Some(ok), Some(cancel)) => {
                MessageButtons::OkCancelCustom(ok.to_string(), cancel.to_string())
            }
            _ => MessageButtons::YesNo,
        };
        let result = MessageDialog::new()
            .set_title(title)
            .set_description(message)
            .set_level(MessageLevel::Info)
            .set_buttons(buttons)
            .show();
        let ok = matches!(
            result,
            MessageDialogResult::Yes | MessageDialogResult::Ok | MessageDialogResult::Custom(_)
        );
        Value::from(ok)
    })
}

pub(crate) fn message(main: &MainDispatcher, args: &Value) -> Result<Value, String> {
    let args = args.clone();
    main.run(move || {
        let body = arg_str(&args, "message").unwrap_or("");
        let title = arg_str(&args, "title").unwrap_or("");
        let kind = arg_str(&args, "kind").unwrap_or("info");
        let level = match kind {
            "error" => MessageLevel::Error,
            "warning" => MessageLevel::Warning,
            _ => MessageLevel::Info,
        };
        MessageDialog::new()
            .set_title(title)
            .set_description(body)
            .set_level(level)
            .set_buttons(MessageButtons::Ok)
            .show();
        Value::Null
    })
}
