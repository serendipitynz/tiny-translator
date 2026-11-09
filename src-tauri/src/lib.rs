mod clipboard;
mod document;
mod engine;
mod format;
mod history;
mod html;
mod json;
mod language;
mod markdown;
mod repetition;
mod tags;
mod translate;
mod units;

use std::fs::{self, File};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use clipboard::LaunchMemory;
use engine::{Engine, Status};
use history::History;
use serde_json::{Value, json};
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
            app.manage(open_history(&data_dir));
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
            history_delete,
            history_clear
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

/// Deletes one translation from 翻訳履歴; `false` when it was already gone.
#[tauri::command]
fn history_delete(history: State<'_, HistoryState>, id: i64) -> Result<bool, String> {
    history.with(|history| history.delete(id))
}

#[tauri::command]
fn history_clear(history: State<'_, HistoryState>) -> Result<(), String> {
    history.with(|history| history.clear())
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
}
