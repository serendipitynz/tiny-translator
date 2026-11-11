mod clipboard;
mod document;
mod download;
mod engine;
mod format;
mod history;
mod html;
mod json;
mod language;
mod markdown;
mod placeholders;
mod repetition;
mod settings;
mod tags;
mod translate;
mod units;
mod update;

use std::fs::{self, File};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use clipboard::LaunchMemory;
use engine::{Engine, EngineState, Status};
use history::History;
use serde_json::{Value, json};
use settings::{FileState, Settings};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager, RunEvent, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_updater::UpdaterExt;
use tauri_plugin_window_state::StateFlags;
use translate::Translator;
use update::{Phase, Updates};

/// Carries every change of the translation engine's state to the screen, as [`status_payload`].
const ENGINE_STATE_EVENT: &str = "engine-state";

/// Tells the screen the app was launched again while running, to do 起動時のクリップボード翻訳
/// once more (doc-1 §5).
const LAUNCHED_AGAIN_EVENT: &str = "launched-again";

/// Carries every change of the update's phase to the screen, as [`update::payload`].
const UPDATE_STATE_EVENT: &str = "update-state";

/// How long asking for a newer version may take. Without network the request fails at once; this
/// bounds a network that takes the request and never answers.
const UPDATE_CHECK_TIMEOUT: Duration = Duration::from_secs(30);

/// How long downloading the update may take as a whole: the plugin applies its timeout to the
/// entire transfer, and the check's 30 seconds would cut off a 20 MB download on a slow line.
const UPDATE_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// How often the download's progress goes to the screen, rather than once for every chunk.
const UPDATE_PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            #[cfg(target_os = "windows")]
            disable_browser_keys(app);
            let data_dir = app.path().app_data_dir()?;
            fs::create_dir_all(&data_dir)?;
            let settings = Arc::new(SettingsStore::load(&data_dir));
            let engine = start_engine(app.handle(), Arc::clone(&settings))?;
            app.manage(engine);
            app.manage(settings);
            app.manage(Translator::default());
            app.manage(LaunchMemory::new(data_dir.join("launch-clipboard")));
            app.manage(open_history(&data_dir));
            app.manage(Downloads::default());
            app.manage(Updates::new(&data_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            engine_state,
            detect_language,
            detect_format,
            translate,
            stop_translation,
            launch_clipboard,
            paste_clipboard,
            copy_text,
            history_search,
            history_count,
            history_delete,
            history_clear,
            settings_get,
            settings_save,
            models_list,
            models_catalog,
            model_download,
            model_download_cancel,
            engine_memory,
            update_state,
            update_check,
            update_install,
            update_dismiss
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| match event {
            RunEvent::Exit => {
                app.state::<Downloads>().cancel();
                app.state::<Engine>().shutdown();
            }
            // Opening the app from Finder, the Dock or Spotlight while it runs does not start a
            // second process on macOS — the running one is asked to reopen instead.
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => launched_again(app),
            _ => {}
        });
}

/// Turns off WebView2's browser keys: F5 and Ctrl+R reload, Ctrl+P prints, Ctrl+F and F3 find, F12
/// opens DevTools (TASK-27). A reload discards the source text and a translation in progress, and
/// none of these exist in WKWebView. Editing keys stay. The keys are not listed and stopped one by
/// one in the screen's keydown, because WebView2 documents its set as open-ended.
#[cfg(target_os = "windows")]
fn disable_browser_keys(app: &tauri::App) {
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;
    use windows_core::Interface;

    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let result = window.with_webview(|webview| {
        // SAFETY: the controller is this window's own, and `with_webview` runs this on the main
        // thread, where WebView2 has to be called.
        let set = unsafe {
            webview
                .controller()
                .CoreWebView2()
                .and_then(|core| core.Settings())
                .and_then(|settings| settings.cast::<ICoreWebView2Settings3>())
                .and_then(|settings| settings.SetAreBrowserAcceleratorKeysEnabled(false))
        };
        if let Err(err) = set {
            eprintln!("could not turn off WebView2's browser keys: {err}");
        }
    });
    if let Err(err) = result {
        eprintln!("could not reach the main webview to turn off its browser keys: {err}");
    }
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

/// The format `text` is read as when the screen leaves it to the app (doc-1 §7.2): `"json"`,
/// `"html"`, `"markdown"` or `"text"`.
#[tauri::command]
fn detect_format(text: String) -> &'static str {
    format::detect(&text)
}

/// Translates `text`, read as `format` (`"text"`, `"markdown"`, `"json"` or `"html"`), into
/// `target` 翻訳単位 by 翻訳単位 (doc-1 §7.1, §7.3), and answers `"done"` or `"stopped"`. JSON leaves the values under
/// `untranslated_keys` (翻訳しないキー) as they are, and fails when `text` is not JSON. The
/// translation goes to `on_update` as it is generated, as `["append", text]` to add
/// at the end or `["replace", text]` for the whole translation so far. Waits for the engine to
/// start when it is 停止中, and keeps it from being released until the translation ends. A finished
/// translation is saved to 翻訳履歴.
#[tauri::command]
async fn translate(
    app: AppHandle,
    text: String,
    format: String,
    untranslated_keys: Vec<String>,
    target: String,
    source: Option<String>,
    on_update: Channel<(&'static str, String)>,
) -> Result<&'static str, String> {
    let request = Request {
        format: document::Format::from_name(&format, untranslated_keys)
            .ok_or_else(|| format!("unknown format: {format}"))?,
        format_name: format,
        text,
        target,
        source,
    };
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
            let mut model = translate::EngineModel {
                port: lease.port(),
                api_key: lease.api_key(),
                cancel: &cancel,
            };
            translate_and_record(
                &mut model,
                lease.model(),
                &request,
                &app.state::<HistoryState>(),
                |update| {
                    let message = match update {
                        document::Update::Append(text) => ("append", text.to_owned()),
                        document::Update::Replace(text) => ("replace", text.to_owned()),
                    };
                    let _ = on_update.send(message);
                },
            )
        });
        app.state::<Translator>().end(&cancel);
        outcome.map(translate::Outcome::as_str)
    })
    .await
    .map_err(|err| err.to_string())?
}

/// One translation the screen asked for.
struct Request {
    text: String,
    /// The name the screen gave [`Request::format`] by, which the history keeps.
    format_name: String,
    format: document::Format,
    target: String,
    source: Option<String>,
}

/// Translates `request` through `model`, which loaded the model named `model_name`, and saves it
/// to 翻訳履歴 once it is done. A stopped or failed translation is not saved (doc-1 §9).
fn translate_and_record(
    model: &mut impl document::Model,
    model_name: &str,
    request: &Request,
    history: &HistoryState,
    mut emit: impl FnMut(document::Update<'_>),
) -> Result<translate::Outcome, String> {
    let mut translation = String::new();
    let outcome = document::translate(
        model,
        &request.text,
        &request.format,
        &request.target,
        request.source.as_deref(),
        |update| {
            match update {
                document::Update::Append(text) => translation.push_str(text),
                document::Update::Replace(text) => text.clone_into(&mut translation),
            }
            emit(update);
        },
    )?;
    if outcome == translate::Outcome::Done {
        history.save(&history::Record {
            source_text: &request.text,
            translation: &translation,
            source_lang: request.source.as_deref(),
            target_lang: &request.target,
            format: &request.format_name,
            model: model_name,
            created_at: now_millis(),
        });
    }
    Ok(outcome)
}

/// 翻訳履歴, or why it could not be opened. The app runs on without it: a history it cannot read
/// is no reason to stop translating.
struct HistoryState(Mutex<Result<History, String>>);

impl HistoryState {
    fn lock(&self) -> MutexGuard<'_, Result<History, String>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn with<T>(&self, act: impl FnOnce(&mut History) -> rusqlite::Result<T>) -> Result<T, String> {
        match &mut *self.lock() {
            Ok(history) => act(history).map_err(|err| err.to_string()),
            Err(reason) => Err(format!("the history is unavailable: {reason}")),
        }
    }

    /// Saves a finished translation. A failure leaves the translation itself done, so it is only
    /// logged.
    fn save(&self, record: &history::Record<'_>) {
        if let Err(err) = self.with(|history| history.save(record)) {
            eprintln!("could not save the translation to the history: {err}");
        }
    }
}

fn open_history(data_dir: &std::path::Path) -> HistoryState {
    let path = data_dir.join("history.sqlite3");
    let history = fs::create_dir_all(data_dir)
        .map_err(|err| err.to_string())
        .and_then(|()| History::open(&path, history::DEFAULT_LIMIT));
    if let Err(reason) = &history {
        eprintln!("could not open the history at {}: {reason}", path.display());
    }
    HistoryState(Mutex::new(history))
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
        })
}

/// The translations in 翻訳履歴 that contain every whitespace-separated term of `query` in their
/// source or translation, newest first; every translation for a blank `query` (doc-1 §9). Each
/// one is `{ id, sourceText, translation, sourceLang, targetLang, format, model, createdAt }`,
/// `sourceLang` being null for 判定なし and `createdAt` milliseconds since the Unix epoch.
#[tauri::command]
fn history_search(history: State<'_, HistoryState>, query: String) -> Result<Vec<Value>, String> {
    let entries = history.with(|history| history.search(&query))?;
    Ok(entries
        .into_iter()
        .map(|entry| {
            json!({
                "id": entry.id,
                "sourceText": entry.source_text,
                "translation": entry.translation,
                "sourceLang": entry.source_lang,
                "targetLang": entry.target_lang,
                "format": entry.format,
                "model": entry.model,
                "createdAt": entry.created_at,
            })
        })
        .collect())
}

/// How many translations 翻訳履歴 holds, so deleting them all can say how many go.
#[tauri::command]
fn history_count(history: State<'_, HistoryState>) -> Result<u32, String> {
    history.with(|history| history.count())
}

/// Deletes one translation from 翻訳履歴; `false` when it was already gone.
#[tauri::command]
fn history_delete(history: State<'_, HistoryState>, id: i64) -> Result<bool, String> {
    history.with(|history| history.delete(id))
}

#[tauri::command]
fn history_clear(history: State<'_, HistoryState>) -> Result<(), String> {
    history.with(|history| history.clear())
}

/// The settings, and how their file stood, read at launch and changed by the settings screen. Held
/// apart from the engine because the engine's [`engine::Spawn`] reads the chosen model on every
/// start.
struct SettingsStore {
    path: PathBuf,
    current: Mutex<(Settings, FileState)>,
}

impl SettingsStore {
    fn load(data_dir: &std::path::Path) -> SettingsStore {
        let path = data_dir.join("settings.json");
        let loaded = settings::load(&path, now_millis());
        if let FileState::MovedAside { to, reason } = &loaded.1 {
            eprintln!(
                "could not read the settings ({reason}); moved them to {}",
                to.display()
            );
        } else if let Some(reason) = loaded.1.write_blocker() {
            eprintln!("the settings will not be saved: {reason}");
        }
        SettingsStore {
            path,
            current: Mutex::new(loaded),
        }
    }

    fn lock(&self) -> MutexGuard<'_, (Settings, FileState)> {
        self.current
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn get(&self) -> Settings {
        self.lock().0.clone()
    }
}

/// 自動解放までの時間 as the engine takes it: 0 minutes never releases (doc-1 §4.5).
fn idle_release(minutes: u32) -> Option<Duration> {
    (minutes > 0).then(|| Duration::from_secs(u64::from(minutes) * 60))
}

/// `{ settings, file }`: the settings as `settings::to_value` gives them, and how their file
/// stands — `{ state: "read" | "missing" }`, `{ state: "moved", path, reason }` for a file that
/// could not be read and was moved aside, `{ state: "unmovable", reason }` or `{ state: "newer",
/// version }` for one that is not written.
#[tauri::command]
fn settings_get(store: State<'_, Arc<SettingsStore>>) -> Value {
    let current = store.lock();
    let file = match &current.1 {
        FileState::Read => json!({ "state": "read" }),
        FileState::Missing => json!({ "state": "missing" }),
        FileState::MovedAside { to, reason } => {
            json!({ "state": "moved", "path": to.to_string_lossy(), "reason": reason })
        }
        FileState::Unmovable { reason } => json!({ "state": "unmovable", "reason": reason }),
        FileState::Newer { version } => json!({ "state": "newer", "version": version }),
    };
    json!({ "settings": settings::to_value(&current.0), "file": file })
}

/// Puts `settings` into effect at once and saves them. They stay in effect for this launch even
/// when they cannot be saved (snz-design doc-7 §5.3); the error says why they were not.
#[tauri::command]
fn settings_save(
    store: State<'_, Arc<SettingsStore>>,
    engine: State<'_, Engine>,
    settings: Value,
) -> Result<(), String> {
    apply_settings(&store, &engine, settings::from_value(&settings)?)
}

/// [`settings_save`] once the screen's value is read.
fn apply_settings(
    store: &SettingsStore,
    engine: &Engine,
    settings: Settings,
) -> Result<(), String> {
    // The lock is let go before the engine is touched: a reload spawns the engine at once, and the
    // spawn reads the chosen model from this same store.
    let (model_changed, write_blocker) = {
        let mut current = store.lock();
        let model_changed = current.0.model != settings.model;
        current.0 = settings.clone();
        (model_changed, current.1.write_blocker())
    };
    engine.set_idle_release(idle_release(settings.idle_release_minutes));
    if model_changed {
        engine.reload();
    }
    if let Some(reason) = write_blocker {
        return Err(reason);
    }
    settings::save(&store.path, &settings).map_err(|err| err.to_string())?;
    store.lock().1 = FileState::Read;
    Ok(())
}

/// `{ dir, models, chosen }`: the models directory, the GGUFs in it as `{ name, bytes }` by name,
/// and the name of the one the engine loads on its next start (null when there is none).
#[tauri::command]
fn models_list(app: AppHandle, store: State<'_, Arc<SettingsStore>>) -> Result<Value, String> {
    let dir = models_dir(&app).map_err(|err| err.to_string())?;
    let models = engine::list_models(&dir).map_err(|err| err.to_string())?;
    let preferred = store.get().model;
    let chosen = engine::choose_model(&dir, preferred.as_deref())
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        });
    let models: Vec<Value> = models
        .into_iter()
        .map(|model| json!({ "name": model.name, "bytes": model.bytes }))
        .collect();
    Ok(json!({ "dir": dir.to_string_lossy(), "models": models, "chosen": chosen }))
}

/// The engine's memory use in bytes while it runs, or null (doc-1 §4.5's メモリ使用量の目安).
#[tauri::command]
async fn engine_memory(app: AppHandle) -> Option<u64> {
    // Off the async runtime: on macOS it waits for `ps`.
    tauri::async_runtime::spawn_blocking(move || app.state::<Engine>().memory())
        .await
        .ok()
        .flatten()
}

fn models_dir(app: &AppHandle) -> tauri::Result<PathBuf> {
    Ok(app.path().app_data_dir()?.join("models"))
}

/// The model fetch under way (doc-1 §10.4): one at a time, stopped by the screen or by app exit.
#[derive(Default)]
struct Downloads(Mutex<Option<Arc<download::Job>>>);

impl Downloads {
    fn lock(&self) -> MutexGuard<'_, Option<Arc<download::Job>>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn cancel(&self) {
        if let Some(job) = self.lock().as_ref() {
            job.cancel();
        }
    }
}

/// `{ dir, models }`: the models directory, and the models the app can fetch (doc-1 §10.4) as
/// `{ name, size, repo, bytes, present, partial }` — whether the directory already has it, and how many
/// bytes of it an earlier fetch left to resume from.
#[tauri::command]
fn models_catalog(app: AppHandle) -> Result<Value, String> {
    let dir = models_dir(&app).map_err(|err| err.to_string())?;
    let models: Vec<Value> = download::CATALOG
        .iter()
        .map(|model| {
            let partial = fs::metadata(download::part_path(&dir, model.name))
                .map(|meta| meta.len())
                .unwrap_or(0);
            json!({
                "name": model.name,
                "size": model.size,
                "repo": model.repo,
                "bytes": model.bytes,
                "present": dir.join(model.name).exists(),
                "partial": partial,
            })
        })
        .collect();
    Ok(json!({ "dir": dir.to_string_lossy(), "models": models }))
}

/// Fetches the catalog's model `name` into the models directory, sending `{ phase: "fetching" |
/// "verifying", bytes }` as it goes. Answers `"done"`, or `"cancelled"` when
/// [`model_download_cancel`] stopped it; an engine that could not start for want of a model is
/// started with the new one.
#[tauri::command]
async fn model_download(
    app: AppHandle,
    name: String,
    progress: Channel<Value>,
) -> Result<&'static str, String> {
    // Off the async runtime: it waits for curl and reads the whole file to hash it.
    tauri::async_runtime::spawn_blocking(move || {
        let model = download::catalogued(&name)
            .ok_or_else(|| format!("{name} is not a model the app can fetch"))?;
        let dir = models_dir(&app).map_err(|err| err.to_string())?;
        let downloads = app.state::<Downloads>();
        let job = {
            let mut current = downloads.lock();
            if current.is_some() {
                return Err("another model is being fetched".to_owned());
            }
            let job = Arc::new(download::Job::default());
            *current = Some(Arc::clone(&job));
            job
        };
        let outcome = download::fetch(
            &job,
            &download::Fetcher::system(),
            &model.url(),
            model.expected(),
            &dir,
            model.name,
            &mut |phase, bytes| {
                let phase = match phase {
                    download::Phase::Fetching => "fetching",
                    download::Phase::Verifying => "verifying",
                };
                let _ = progress.send(json!({ "phase": phase, "bytes": bytes }));
            },
        );
        downloads.lock().take();
        match outcome? {
            download::Outcome::Done => {
                let engine = app.state::<Engine>();
                let status = engine.status();
                if status.state == EngineState::Stopped && status.error.is_some() {
                    engine.start();
                }
                Ok("done")
            }
            download::Outcome::Cancelled => Ok("cancelled"),
        }
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
fn model_download_cancel(downloads: State<'_, Downloads>) {
    downloads.cancel();
}

/// `{ phase, version?, received?, total?, reason? }` as [`update::payload`] gives it.
#[tauri::command]
fn update_state(updates: State<'_, Updates>) -> Value {
    updates.payload()
}

fn emit_update(app: &AppHandle) {
    let _ = app.emit(UPDATE_STATE_EVENT, app.state::<Updates>().payload());
}

/// Asks the release's `latest.json` whether a newer version is published (doc-1 §14), and answers
/// `"available"` or `"none"`. A check at launch (`manual` false) answers `"skipped"` without asking
/// when the settings turned it off or the last one was less than a day ago; either kind answers
/// `"busy"` while an update is being installed.
#[tauri::command]
async fn update_check(app: AppHandle, manual: bool) -> Result<&'static str, String> {
    let updates = app.state::<Updates>();
    if !manual
        && (!app.state::<Arc<SettingsStore>>().get().update_check
            || !updates.launch_check_due(now_millis()))
    {
        return Ok("skipped");
    }
    if updates.busy() {
        return Ok("busy");
    }
    let found = updater(&app)?.check().await.map_err(|err| {
        let reason = err.to_string();
        eprintln!("could not check for an update: {reason}");
        reason
    })?;
    let available = found.is_some();
    if updates.found(found, now_millis()) {
        emit_update(&app);
    }
    Ok(if available { "available" } else { "none" })
}

fn updater(app: &AppHandle) -> Result<tauri_plugin_updater::Updater, String> {
    let handle = app.clone();
    app.updater_builder()
        .timeout(UPDATE_CHECK_TIMEOUT)
        // Windows only: the plugin starts the installer and ends the process at once, so
        // RunEvent::Exit never comes. Left running, the engine would hold llama-server.exe open in
        // the folder the installer is about to replace.
        .on_before_exit(move || {
            handle.state::<Downloads>().cancel();
            handle.state::<Engine>().shutdown();
        })
        .build()
        .map_err(|err| err.to_string())
}

/// Downloads the update the last check found, checks its signature against the key the app carries
/// and puts it in place, then restarts the app (doc-1 §14). Progress goes out as `update-state`.
/// Returns only when the update failed: on Windows the installer takes over and the app exits.
#[tauri::command]
async fn update_install(app: AppHandle) -> Result<(), String> {
    let updates = app.state::<Updates>();
    let mut update = updates.begin()?;
    emit_update(&app);
    update.timeout = Some(UPDATE_DOWNLOAD_TIMEOUT);
    let mut received = 0u64;
    let mut told = Instant::now();
    let downloaded = update
        .download(
            |chunk, total| {
                received += chunk as u64;
                if told.elapsed() >= UPDATE_PROGRESS_INTERVAL {
                    told = Instant::now();
                    updates.set_phase(Phase::Downloading { received, total });
                    emit_update(&app);
                }
            },
            || {},
        )
        .await;
    // The signature is checked inside the download: a package it does not verify never reaches
    // `install`.
    let bytes = match downloaded {
        Ok(bytes) => bytes,
        Err(err) => return update_failed(&app, &err.to_string()),
    };
    updates.set_phase(Phase::Installing);
    emit_update(&app);
    // Off the async runtime: replacing the app is file work, and on Windows the call never returns.
    let installed = tauri::async_runtime::spawn_blocking(move || update.install(bytes)).await;
    match installed {
        Ok(Ok(())) => app.restart(),
        Ok(Err(err)) => update_failed(&app, &err.to_string()),
        Err(err) => update_failed(&app, &err.to_string()),
    }
}

fn update_failed(app: &AppHandle, reason: &str) -> Result<(), String> {
    eprintln!("could not install the update: {reason}");
    app.state::<Updates>().set_phase(Phase::Failed {
        reason: reason.to_owned(),
    });
    emit_update(app);
    Err(reason.to_owned())
}

/// Sets the offered update aside: a check at launch offers it again a day later, 今すぐ確認 at once.
#[tauri::command]
fn update_dismiss(app: AppHandle) {
    app.state::<Updates>().dismiss(now_millis());
    emit_update(&app);
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
    // Turned off in the settings: as if there were nothing, with no hint about macOS's permission.
    if !app.state::<Arc<SettingsStore>>().get().launch_clipboard {
        return ("none", None);
    }
    if !clipboard::readable_without_asking() {
        return ("not-allowed", None);
    }
    match read_clipboard(&app) {
        Some(text) if app.state::<LaunchMemory>().take(&text) => ("text", Some(text)),
        _ => ("none", None),
    }
}

/// The clipboard's text for the 貼り付け button, or `None` when it holds none. Read even where
/// macOS asks the user first (doc-1 §6.3).
#[tauri::command]
async fn paste_clipboard(app: AppHandle) -> Option<String> {
    // Off the async runtime: where macOS asks, the read waits for the user's answer.
    tauri::async_runtime::spawn_blocking(move || read_clipboard(&app))
        .await
        .ok()
        .flatten()
}

/// The clipboard's text, with its lines broken as the source pane breaks them.
fn read_clipboard(app: &AppHandle) -> Option<String> {
    app.clipboard()
        .read_text()
        .ok()
        .map(clipboard::with_lf_line_breaks)
}

#[tauri::command]
fn copy_text(app: AppHandle, text: String) -> Result<(), String> {
    app.clipboard()
        .write_text(text)
        .map_err(|err| err.to_string())
}

/// Starts the engine at app launch, so that loading overlaps the time the user takes to bring in
/// the text to translate (doc-1 §5).
fn start_engine(
    app: &AppHandle,
    settings: Arc<SettingsStore>,
) -> Result<Engine, Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    let model_dir = models_dir(app)?;
    fs::create_dir_all(&model_dir)?;
    let log_dir = app.path().app_log_dir()?;
    fs::create_dir_all(&log_dir)?;
    let pid_file = data_dir.join("engine.pid");
    let program = sidecar_path()?;
    let idle = idle_release(settings.get().idle_release_minutes);

    engine::kill_leftover(&pid_file, &program);

    let spawn: engine::Spawn = Box::new(move |port, api_key| {
        let model = engine::choose_model(&model_dir, settings.get().model.as_deref())?;
        // Truncated on every start: the log is for looking into the run that just went wrong.
        let log = File::create(log_dir.join("llama-server.log"))?;
        let child = engine::windowless(&program)
            .args(engine::server_args(&model, port))
            .env(engine::API_KEY_ENV, api_key)
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log)
            .spawn()?;
        // The file name alone: the full path would put the user's home directory in the history.
        let name = model.file_name().unwrap_or(model.as_os_str());
        Ok(engine::Spawned {
            child,
            model: name.to_string_lossy().into_owned(),
        })
    });
    let handle = app.clone();
    let notify: engine::Notify = Box::new(move |status| {
        let _ = handle.emit(ENGINE_STATE_EVENT, status_payload(status));
    });

    let engine = Engine::new(spawn, notify, Some(pid_file));
    engine.set_idle_release(idle);
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

#[cfg(test)]
mod tests {
    use super::*;
    use translate::{Completion, Outcome};

    /// A model that streams `pieces` as the translation of every prompt, then ends as `end` says.
    struct Fake {
        pieces: Vec<&'static str>,
        end: Result<Completion, String>,
    }

    impl document::Model for Fake {
        fn tokens(&mut self, text: &str) -> Result<usize, String> {
            Ok(text.chars().count())
        }

        fn complete(
            &mut self,
            _prompt: &str,
            on_piece: &mut dyn FnMut(&str) -> bool,
        ) -> Result<Completion, String> {
            for piece in &self.pieces {
                if !on_piece(piece) {
                    return Ok(Completion::Cut);
                }
            }
            self.end.clone()
        }
    }

    fn request() -> Request {
        Request {
            text: "Good morning.".into(),
            format_name: "text".into(),
            format: document::Format::Text,
            target: "Japanese".into(),
            source: Some("English".into()),
        }
    }

    fn empty_history() -> HistoryState {
        HistoryState(Mutex::new(Ok(History::in_memory(history::DEFAULT_LIMIT))))
    }

    fn saved(history: &HistoryState) -> Vec<history::Entry> {
        history.with(|history| history.search("")).unwrap()
    }

    fn run(end: Result<Completion, String>, history: &HistoryState) -> Result<Outcome, String> {
        let mut model = Fake {
            pieces: vec!["おはよう", "ございます。"],
            end,
        };
        translate_and_record(&mut model, "model.gguf", &request(), history, |_| {})
    }

    #[test]
    fn a_finished_translation_is_saved_with_what_the_screen_was_sent() {
        let history = empty_history();
        let before = now_millis();
        assert_eq!(run(Ok(Completion::Done), &history), Ok(Outcome::Done));
        let entries = saved(&history);
        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.source_text, "Good morning.");
        assert_eq!(entry.translation, "おはようございます。");
        assert_eq!(entry.source_lang.as_deref(), Some("English"));
        assert_eq!(entry.target_lang, "Japanese");
        assert_eq!(entry.format, "text");
        assert_eq!(entry.model, "model.gguf");
        assert!((before..=now_millis()).contains(&entry.created_at));
    }

    #[test]
    fn a_stopped_translation_is_not_saved() {
        let history = empty_history();
        assert_eq!(run(Ok(Completion::Stopped), &history), Ok(Outcome::Stopped));
        assert!(saved(&history).is_empty());
    }

    #[test]
    fn a_failed_translation_is_not_saved() {
        let history = empty_history();
        assert!(run(Err("the engine went away".into()), &history).is_err());
        assert!(saved(&history).is_empty());
    }

    #[test]
    fn a_history_that_could_not_be_opened_leaves_the_translation_done() {
        let history = HistoryState(Mutex::new(Err("unreadable".into())));
        assert_eq!(run(Ok(Completion::Done), &history), Ok(Outcome::Done));
    }

    /// The engine's spawn reads the chosen model from the store, as the app's does; saving a new
    /// model reloads a loading engine, which spawns at once.
    #[cfg(unix)]
    #[test]
    fn saving_a_new_model_reloads_the_engine_without_waiting_on_the_settings() {
        let dir =
            std::env::temp_dir().join(format!("tiny-translator-apply-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let store = Arc::new(SettingsStore::load(&dir));
        let seen = Arc::new(Mutex::new(Vec::new()));
        let (reader, models) = (Arc::clone(&store), Arc::clone(&seen));
        let engine = Engine::new(
            Box::new(move |_, _| {
                models.lock().unwrap().push(reader.get().model);
                // Never answers /health, so the engine stays 読み込み中.
                let child = std::process::Command::new("sleep").arg("60").spawn()?;
                Ok(engine::Spawned {
                    child,
                    model: "stand-in.gguf".into(),
                })
            }),
            Box::new(|_| {}),
            None,
        );
        engine.start();
        let settings = Settings {
            model: Some("other.gguf".into()),
            ..Settings::default()
        };
        let (done, finished) = std::sync::mpsc::channel();
        let (store_in, engine) = (Arc::clone(&store), Arc::new(engine));
        let engine_in = Arc::clone(&engine);
        std::thread::spawn(move || {
            let _ = done.send(apply_settings(&store_in, &engine_in, settings));
        });
        let result = finished
            .recv_timeout(Duration::from_secs(5))
            .expect("saving the settings never returned");
        assert_eq!(result, Ok(()));
        assert_eq!(*seen.lock().unwrap(), [None, Some("other.gguf".to_owned())]);
        assert_eq!(
            settings::load(&store.path, 0).0.model.as_deref(),
            Some("other.gguf")
        );
        engine.shutdown();
        fs::remove_dir_all(&dir).unwrap();
    }
}
