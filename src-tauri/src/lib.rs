mod clipboard;
mod engine;
mod language;
mod translate;

use std::fs::{self, File};
use std::path::PathBuf;
use std::process::Stdio;

use clipboard::LaunchMemory;
use engine::{Engine, Status};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_window_state::StateFlags;
use translate::Translator;

/// Carries every change of the translation engine's state to the screen, as [`status_payload`].
const ENGINE_STATE_EVENT: &str = "engine-state";

/// Tells the screen the app was launched again while running, to do 起動時のクリップボード翻訳
/// once more (doc-1 §5).
const LAUNCHED_AGAIN_EVENT: &str = "launched-again";

pub fn run() {
    tauri::Builder::default()
        // First, so a second launch hands over to the running app before anything else starts.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            launched_again(app);
        }))
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(StateFlags::SIZE | StateFlags::POSITION | StateFlags::MAXIMIZED)
                .build(),
        )
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let engine = start_engine(app.handle())?;
            app.manage(engine);
            app.manage(Translator::default());
            let data_dir = app.path().app_data_dir()?;
            app.manage(LaunchMemory::new(data_dir.join("launch-clipboard")));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            engine_state,
            detect_language,
            translate,
            stop_translation,
            launch_clipboard,
            paste_clipboard,
            copy_text
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| match event {
            RunEvent::Exit => app.state::<Engine>().shutdown(),
            // Opening the app from Finder, the Dock or Spotlight while it runs does not start a
            // second process on macOS — the running one is asked to reopen instead.
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => launched_again(app),
            _ => {}
        });
}

fn launched_again(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
    let _ = app.emit(LAUNCHED_AGAIN_EVENT, ());
}

/// `[state, error]`: a pair rather than an object, so the crate needs no serde derive for it.
fn status_payload(status: &Status) -> (&'static str, Option<String>) {
    (status.state.as_str(), status.error.clone())
}

#[tauri::command]
fn engine_state(engine: State<'_, Engine>) -> (&'static str, Option<String>) {
    status_payload(&engine.status())
}

/// The source language of `text` as PLaMo's language code, `"zh"` for Chinese, or `None` when the
/// app leaves it undecided (doc-1 §6.1).
#[tauri::command]
fn detect_language(text: String) -> Option<&'static str> {
    language::detect(&text)
}

/// Translates `text` into `target`, sending the translation to `on_piece` as it is generated, and
/// answers `"done"` or `"stopped"`. Waits for the engine to start when it is 停止中, and keeps it from
/// being released until the translation ends.
#[tauri::command]
async fn translate(
    app: AppHandle,
    text: String,
    target: String,
    source: Option<String>,
    on_piece: Channel<String>,
) -> Result<&'static str, String> {
    // Registered before anything waits, so a stop pressed while the engine is still loading
    // already reaches this translation.
    let cancel = app.state::<Translator>().begin();
    // The wait for the engine and the stream are blocking, and would otherwise hold one of the
    // async runtime's few threads for the whole translation.
    tauri::async_runtime::spawn_blocking(move || {
        let result = app
            .state::<Engine>()
            .acquire()
            .map_err(|err| err.to_string());
        let outcome = result.and_then(|lease| {
            let prompt = translate::prompt(&text, &target, source.as_deref());
            translate::stream(lease.port(), lease.api_key(), &prompt, &cancel, |piece| {
                let _ = on_piece.send(piece.to_owned());
            })
        });
        app.state::<Translator>().end(&cancel);
        outcome.map(translate::Outcome::as_str)
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
fn stop_translation(translator: State<'_, Translator>) {
    translator.stop();
}

/// What 起動時のクリップボード翻訳 found (doc-1 §6.3): `["text", text]` to translate, `["none",
/// null]` when there is nothing new, or `["not-allowed", null]` when reading would make macOS ask
/// the user, so the clipboard was left unread.
#[tauri::command]
fn launch_clipboard(app: AppHandle) -> (&'static str, Option<String>) {
    if !clipboard::readable_without_asking() {
        return ("not-allowed", None);
    }
    match app.clipboard().read_text() {
        Ok(text) if app.state::<LaunchMemory>().take(&text) => ("text", Some(text)),
        _ => ("none", None),
    }
}

/// The clipboard's text for the 貼り付け button, or `None` when it holds none. Read even where
/// macOS asks the user first (doc-1 §6.3).
#[tauri::command]
async fn paste_clipboard(app: AppHandle) -> Option<String> {
    // Off the async runtime: where macOS asks, the read waits for the user's answer.
    tauri::async_runtime::spawn_blocking(move || app.clipboard().read_text().ok())
        .await
        .ok()
        .flatten()
}

#[tauri::command]
fn copy_text(app: AppHandle, text: String) -> Result<(), String> {
    app.clipboard()
        .write_text(text)
        .map_err(|err| err.to_string())
}

/// Starts the engine at app launch, so that loading overlaps the time the user takes to bring in
/// the text to translate (doc-1 §5).
fn start_engine(app: &AppHandle) -> Result<Engine, Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    let model_dir = data_dir.join("models");
    fs::create_dir_all(&model_dir)?;
    let log_dir = app.path().app_log_dir()?;
    fs::create_dir_all(&log_dir)?;
    let pid_file = data_dir.join("engine.pid");
    let program = sidecar_path()?;

    engine::kill_leftover(&pid_file, &program);

    let spawn: engine::Spawn = Box::new(move |port, api_key| {
        let model = engine::choose_model(&model_dir)?;
        // Truncated on every start: the log is for looking into the run that just went wrong.
        let log = File::create(log_dir.join("llama-server.log"))?;
        engine::windowless(&program)
            .args(engine::server_args(&model, port))
            .env(engine::API_KEY_ENV, api_key)
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log)
            .spawn()
    });
    let handle = app.clone();
    let notify: engine::Notify = Box::new(move |status| {
        let _ = handle.emit(ENGINE_STATE_EVENT, status_payload(status));
    });

    let engine = Engine::new(spawn, notify, Some(pid_file));
    engine.start();
    Ok(engine)
}

/// Where Tauri puts the `bundle.externalBin` sidecar: next to the app's own executable, with the
/// target triple taken off its name — in `target/<profile>/` under `tauri dev` and in the bundle
/// alike. Absolute, which is also what lets [`engine::kill_leftover`] recognise the process.
fn sidecar_path() -> std::io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    Ok(exe.with_file_name(format!("llama-server{}", std::env::consts::EXE_SUFFIX)))
}
