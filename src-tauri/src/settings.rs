//! The settings the settings screen changes (TASK-21), kept in `settings.json` in the app data
//! directory. The two colour axes are not here: the screen keeps them in `localStorage`, where
//! `index.html` reads them before the first paint (doc-1 §8.3).
//!
//! The file is read leniently: a value of the wrong kind falls back to its default on its own and
//! is left in the file until the user saves (snz-design doc-7 §6.1). A file that cannot be read at
//! all is moved aside at launch rather than overwritten (2026-11-09 owner decision), so whatever it
//! held stays for the user to recover.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

/// The file's layout version. A file written by a newer version is neither read nor written: a
/// newer app may give the same fields another meaning, and writing would destroy what it saved
/// (doc-7 §5.2).
pub const VERSION: u64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    /// 第一優先言語, as the language code the prompt names (doc-1 §6.4).
    pub first_language: String,
    /// 第二優先言語.
    pub second_language: String,
    /// 自動解放までの時間 in minutes; 0 never releases (doc-1 §4.5).
    pub idle_release_minutes: u32,
    /// The GGUF's file name in the models directory, or `None` for the default choice
    /// ([`crate::engine::choose_model`]).
    pub model: Option<String>,
    /// Whether 起動時のクリップボード翻訳 runs (doc-1 §6.3).
    pub launch_clipboard: bool,
    /// The 翻訳しないキー of JSON, ARB and .strings (doc-1 §7.1).
    pub untranslated_keys: Vec<String>,
    /// Whether the app asks for a newer version at launch (doc-1 §14). On by default (2026-11-11
    /// owner decision): off, a fix would reach only those who went looking for it.
    pub update_check: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            first_language: "ja".into(),
            second_language: "en".into(),
            idle_release_minutes: 10,
            model: None,
            launch_clipboard: true,
            untranslated_keys: Vec::new(),
            update_check: true,
        }
    }
}

/// How the file stood when it was read at launch, or after the last save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileState {
    /// Read, or saved since.
    Read,
    /// There was no file: the defaults are in use, and the first save creates it.
    Missing,
    /// The file could not be read and was moved to `to`; the defaults are in use.
    MovedAside { to: PathBuf, reason: String },
    /// The file could not be read and could not be moved aside either, so it is not written.
    Unmovable { reason: String },
    /// The file was written by a newer version, so it is not written.
    Newer { version: u64 },
}

impl FileState {
    /// Why the file must not be written, or `None` when it may.
    pub fn write_blocker(&self) -> Option<String> {
        match self {
            FileState::Unmovable { reason } => Some(format!(
                "the settings file could not be read or moved aside: {reason}"
            )),
            FileState::Newer { version } => Some(format!(
                "the settings file was written by a newer version (layout {version})"
            )),
            _ => None,
        }
    }
}

/// Reads the settings at `path`. An unreadable file is renamed to `settings.unreadable-<now>.json`
/// beside it, `now` being the time in milliseconds, so two launches never pick the same name.
pub fn load(path: &Path, now: i64) -> (Settings, FileState) {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return (Settings::default(), FileState::Missing);
        }
        Err(err) => return (Settings::default(), move_aside(path, now, err.to_string())),
    };
    match parse(&text) {
        Ok(Parsed::Current(settings)) => (settings, FileState::Read),
        Ok(Parsed::Newer(version)) => (Settings::default(), FileState::Newer { version }),
        Err(reason) => (Settings::default(), move_aside(path, now, reason)),
    }
}

fn move_aside(path: &Path, now: i64, reason: String) -> FileState {
    let to = path.with_file_name(format!("settings.unreadable-{now}.json"));
    match fs::rename(path, &to) {
        Ok(()) => FileState::MovedAside { to, reason },
        Err(err) => FileState::Unmovable {
            reason: format!("{reason}; moving it aside failed: {err}"),
        },
    }
}

enum Parsed {
    Current(Settings),
    Newer(u64),
}

fn parse(text: &str) -> Result<Parsed, String> {
    let value: Value = serde_json::from_str(text).map_err(|err| err.to_string())?;
    let Value::Object(fields) = value else {
        return Err("the settings file does not hold a JSON object".into());
    };
    let version = match fields.get("version") {
        None => VERSION,
        Some(version) => version
            .as_u64()
            .ok_or_else(|| format!("the settings file's version is not a number: {version}"))?,
    };
    if version > VERSION {
        return Ok(Parsed::Newer(version));
    }
    Ok(Parsed::Current(from_fields(&fields, &Settings::default())))
}

/// The settings `fields` name, each one that is missing or of the wrong kind taken from `fallback`.
fn from_fields(fields: &Map<String, Value>, fallback: &Settings) -> Settings {
    let text = |key: &str| {
        fields
            .get(key)
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    };
    let model = match fields.get("model") {
        Some(Value::Null) => None,
        Some(Value::String(name)) if !name.is_empty() => Some(name.clone()),
        _ => fallback.model.clone(),
    };
    let untranslated_keys = match fields.get("untranslatedKeys") {
        Some(Value::Array(keys)) if keys.iter().all(Value::is_string) => keys
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        _ => fallback.untranslated_keys.clone(),
    };
    Settings {
        first_language: text("firstLanguage").unwrap_or_else(|| fallback.first_language.clone()),
        second_language: text("secondLanguage").unwrap_or_else(|| fallback.second_language.clone()),
        idle_release_minutes: fields
            .get("idleReleaseMinutes")
            .and_then(Value::as_u64)
            .and_then(|minutes| u32::try_from(minutes).ok())
            .unwrap_or(fallback.idle_release_minutes),
        model,
        launch_clipboard: fields
            .get("launchClipboard")
            .and_then(Value::as_bool)
            .unwrap_or(fallback.launch_clipboard),
        untranslated_keys,
        update_check: fields
            .get("updateCheck")
            .and_then(Value::as_bool)
            .unwrap_or(fallback.update_check),
    }
}

/// The settings as the screen receives them, and as the file holds them (without `version`).
pub fn to_value(settings: &Settings) -> Value {
    json!({
        "firstLanguage": settings.first_language,
        "secondLanguage": settings.second_language,
        "idleReleaseMinutes": settings.idle_release_minutes,
        "model": settings.model,
        "launchClipboard": settings.launch_clipboard,
        "untranslatedKeys": settings.untranslated_keys,
        "updateCheck": settings.update_check,
    })
}

/// The settings the screen sent to be saved. Strict where the file is lenient: a value of the
/// wrong kind here is the screen's mistake, not a file edited by hand.
pub fn from_value(value: &Value) -> Result<Settings, String> {
    let Value::Object(fields) = value else {
        return Err("the settings are not an object".into());
    };
    let settings = from_fields(fields, &Settings::default());
    if to_value(&settings) != Value::Object(fields.clone()) {
        return Err(format!("the settings are malformed: {value}"));
    }
    Ok(settings)
}

/// Writes `settings` to `path` through a temporary file beside it, so a write cut short leaves the
/// previous file whole rather than a truncated one the next launch would have to move aside.
pub fn save(path: &Path, settings: &Settings) -> io::Result<()> {
    let mut value = to_value(settings);
    value["version"] = json!(VERSION);
    let mut text = serde_json::to_string_pretty(&value).map_err(io::Error::other)?;
    text.push('\n');
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, text)?;
    fs::rename(&temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Dir(PathBuf);

    impl Dir {
        fn new(name: &str) -> Dir {
            let dir = std::env::temp_dir().join(format!(
                "tiny-translator-settings-{}-{name}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Dir(dir)
        }

        fn file(&self) -> PathBuf {
            self.0.join("settings.json")
        }

        fn names(&self) -> Vec<String> {
            let mut names: Vec<String> = fs::read_dir(&self.0)
                .unwrap()
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn changed() -> Settings {
        Settings {
            first_language: "en".into(),
            second_language: "zh-hans".into(),
            idle_release_minutes: 0,
            model: Some("plamo-3-translate-0.8b-Q4_K_M.gguf".into()),
            launch_clipboard: false,
            untranslated_keys: vec!["id".into(), "url".into()],
            update_check: false,
        }
    }

    #[test]
    fn no_file_is_the_defaults() {
        let dir = Dir::new("missing");
        assert_eq!(
            load(&dir.file(), 1),
            (Settings::default(), FileState::Missing)
        );
        assert!(dir.names().is_empty());
    }

    #[test]
    fn saved_settings_survive_a_reload() {
        let dir = Dir::new("round-trip");
        save(&dir.file(), &changed()).unwrap();
        assert_eq!(load(&dir.file(), 1), (changed(), FileState::Read));
        assert_eq!(dir.names(), ["settings.json"]);
    }

    #[test]
    fn a_file_that_is_not_json_is_moved_aside_and_kept() {
        let dir = Dir::new("not-json");
        fs::write(dir.file(), "{ \"firstLanguage\": ").unwrap();
        let (settings, state) = load(&dir.file(), 1_762_700_000_000);
        assert_eq!(settings, Settings::default());
        let FileState::MovedAside { to, .. } = state else {
            panic!("not moved aside: {state:?}");
        };
        assert_eq!(to, dir.0.join("settings.unreadable-1762700000000.json"));
        assert_eq!(fs::read_to_string(&to).unwrap(), "{ \"firstLanguage\": ");
        assert_eq!(dir.names(), ["settings.unreadable-1762700000000.json"]);
        assert_eq!(state_after_save(&dir), FileState::Read);
    }

    fn state_after_save(dir: &Dir) -> FileState {
        save(&dir.file(), &changed()).unwrap();
        load(&dir.file(), 2).1
    }

    #[test]
    fn json_that_is_not_an_object_is_moved_aside() {
        let dir = Dir::new("array");
        fs::write(dir.file(), "[]").unwrap();
        assert!(matches!(
            load(&dir.file(), 1).1,
            FileState::MovedAside { .. }
        ));
    }

    #[test]
    fn a_file_that_is_not_utf8_is_moved_aside() {
        let dir = Dir::new("binary");
        fs::write(dir.file(), [0xff, 0xfe, 0x00]).unwrap();
        assert!(matches!(
            load(&dir.file(), 1).1,
            FileState::MovedAside { .. }
        ));
    }

    #[test]
    fn a_newer_file_is_left_alone_and_not_to_be_written() {
        let dir = Dir::new("newer");
        let text = "{\"version\": 2, \"firstLanguage\": \"en\"}";
        fs::write(dir.file(), text).unwrap();
        let (settings, state) = load(&dir.file(), 1);
        assert_eq!(settings, Settings::default());
        assert_eq!(state, FileState::Newer { version: 2 });
        assert!(state.write_blocker().is_some());
        assert_eq!(fs::read_to_string(dir.file()).unwrap(), text);
    }

    #[test]
    fn a_value_of_the_wrong_kind_falls_back_on_its_own() {
        let dir = Dir::new("lenient");
        let text = r#"{"firstLanguage": "en", "secondLanguage": 3, "idleReleaseMinutes": -1,
            "model": 5, "launchClipboard": "no", "untranslatedKeys": ["id", 2],
            "updateCheck": "off"}"#;
        fs::write(dir.file(), text).unwrap();
        let (settings, state) = load(&dir.file(), 1);
        assert_eq!(state, FileState::Read);
        assert_eq!(
            settings,
            Settings {
                first_language: "en".into(),
                ..Settings::default()
            }
        );
        // doc-7 §6.1: read past, not rewritten.
        assert_eq!(fs::read_to_string(dir.file()).unwrap(), text);
    }

    #[test]
    fn a_file_without_a_version_reads_as_the_current_one() {
        let dir = Dir::new("no-version");
        fs::write(dir.file(), r#"{"launchClipboard": false}"#).unwrap();
        let (settings, state) = load(&dir.file(), 1);
        assert_eq!(state, FileState::Read);
        assert!(!settings.launch_clipboard);
    }

    #[test]
    fn a_version_that_is_not_a_number_is_unreadable() {
        let dir = Dir::new("bad-version");
        fs::write(dir.file(), r#"{"version": "1"}"#).unwrap();
        assert!(matches!(
            load(&dir.file(), 1).1,
            FileState::MovedAside { .. }
        ));
    }

    #[test]
    fn an_unreadable_file_that_cannot_be_moved_is_not_to_be_written() {
        let dir = Dir::new("unmovable");
        // A directory where the file should be: reading fails, and so does renaming it over the
        // name the moved file would take, which is made a non-empty directory first.
        fs::create_dir(dir.file()).unwrap();
        let target = dir.0.join("settings.unreadable-1.json");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("keep"), "").unwrap();
        let (_, state) = load(&dir.file(), 1);
        assert!(matches!(state, FileState::Unmovable { .. }), "{state:?}");
        assert!(state.write_blocker().is_some());
    }

    #[test]
    fn the_screen_s_settings_are_read_strictly() {
        assert_eq!(from_value(&to_value(&changed())), Ok(changed()));
        let mut wrong = to_value(&changed());
        wrong["idleReleaseMinutes"] = json!("10");
        assert!(from_value(&wrong).is_err());
        let mut extra = to_value(&changed());
        extra["colour"] = json!("red");
        assert!(from_value(&extra).is_err());
        assert!(from_value(&json!([])).is_err());
    }

    #[test]
    fn the_file_carries_its_version() {
        let dir = Dir::new("version");
        save(&dir.file(), &Settings::default()).unwrap();
        let value: Value = serde_json::from_str(&fs::read_to_string(dir.file()).unwrap()).unwrap();
        assert_eq!(value["version"], json!(VERSION));
    }
}
