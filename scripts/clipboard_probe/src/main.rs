//! Check whether reading the clipboard at launch makes macOS ask the user (doc-1 §6.3).
//!
//! The app reads the clipboard text once in `setup`, the way 起動時のクリップボード翻訳 will,
//! through tauri-plugin-clipboard-manager's Rust API. It also offers the two later reads the
//! app could fall back to: a button that calls the plugin's JS command (a read that follows a
//! click, not a paste), and a text area to paste into with Cmd+V (a read the WebView makes in
//! response to the user's paste). Every result is appended to `probe.log` in the app's log
//! directory together with NSPasteboard's `accessBehavior`, so a run can be judged from the
//! log as well as from what appeared on screen.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

struct Log {
    path: PathBuf,
    lines: Mutex<Vec<String>>,
}

impl Log {
    fn record(&self, line: String) {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let line = format!("{millis} {line}");
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = writeln!(file, "{line}");
        }
        self.lines.lock().unwrap().push(line);
    }
}

#[cfg(target_os = "macos")]
fn access_behavior() -> String {
    use objc2_app_kit::{NSPasteboard, NSPasteboardAccessBehavior};
    let behavior = NSPasteboard::generalPasteboard().accessBehavior();
    let name = match behavior {
        NSPasteboardAccessBehavior::Default => "Default",
        NSPasteboardAccessBehavior::Ask => "Ask",
        NSPasteboardAccessBehavior::AlwaysAllow => "AlwaysAllow",
        NSPasteboardAccessBehavior::AlwaysDeny => "AlwaysDeny",
        _ => "unknown",
    };
    format!("{name} ({})", behavior.0)
}

#[cfg(not(target_os = "macos"))]
fn access_behavior() -> String {
    "n/a".into()
}

fn describe_read(result: Result<String, impl std::fmt::Display>) -> String {
    match result {
        Ok(text) => {
            let head: String = text.chars().take(40).collect();
            format!("ok chars={} head={head:?}", text.chars().count())
        }
        Err(error) => format!("err {error}"),
    }
}

#[tauri::command]
fn record(log: State<'_, Log>, line: String) {
    log.record(format!("[{}] {line}", access_behavior()));
}

#[tauri::command]
fn log_lines(log: State<'_, Log>) -> Vec<String> {
    log.lines.lock().unwrap().clone()
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![record, log_lines])
        .setup(|app| {
            let dir = app.path().app_log_dir()?;
            fs::create_dir_all(&dir)?;
            let log = Log {
                path: dir.join("probe.log"),
                lines: Mutex::new(Vec::new()),
            };
            log.record(format!("launch accessBehavior={}", access_behavior()));
            let result = app.clipboard().read_text();
            log.record(format!("setup read_text {}", describe_read(result)));
            log.record(format!(
                "after setup read accessBehavior={}",
                access_behavior()
            ));
            app.manage(log);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run clipboard_probe");
}
