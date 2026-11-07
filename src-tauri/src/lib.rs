mod engine;

use std::fs::{self, File};
use std::path::PathBuf;
use std::process::Stdio;

use engine::{Engine, Status};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State};

/// Carries every change of the translation engine's state to the screen, as [`status_payload`].
const ENGINE_STATE_EVENT: &str = "engine-state";

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let engine = start_engine(app.handle())?;
            app.manage(engine);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![engine_state])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                app.state::<Engine>().shutdown();
            }
        });
}

/// `[state, error]`: a pair rather than an object, so the crate needs no serde derive for it.
fn status_payload(status: &Status) -> (&'static str, Option<String>) {
    (status.state.as_str(), status.error.clone())
}

#[tauri::command]
fn engine_state(engine: State<'_, Engine>) -> (&'static str, Option<String>) {
    status_payload(&engine.status())
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

    let spawn: engine::Spawn = Box::new(move |port| {
        let model = engine::choose_model(&model_dir)?;
        // Truncated on every start: the log is for looking into the run that just went wrong.
        let log = File::create(log_dir.join("llama-server.log"))?;
        engine::windowless(&program)
            .args(engine::server_args(&model, port))
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
