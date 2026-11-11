//! 翻訳履歴 (doc-1 §9): every finished translation, kept in SQLite and searched with FTS5.
//!
//! The source and the translation are indexed by FTS5's trigram tokenizer, which indexes every
//! run of three characters rather than words, so Japanese — written without spaces — is found as
//! easily as English.

use std::path::Path;

use rusqlite::{Connection, Row, params, params_from_iter};

/// 保存件数の上限の既定値 (doc-1 §9).
pub const DEFAULT_LIMIT: usize = 1000;

/// The schema this module writes, kept in SQLite's `user_version`.
const SCHEMA_VERSION: i64 = 1;

/// `AUTOINCREMENT` so that an id is never handed out twice: a list still showing a deleted entry
/// must not end up deleting the translation saved after it.
const SCHEMA: &str = "
CREATE TABLE translations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_text TEXT NOT NULL,
    translation TEXT NOT NULL,
    source_lang TEXT,
    target_lang TEXT NOT NULL,
    format TEXT NOT NULL,
    model TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE VIRTUAL TABLE translations_fts USING fts5(
    source_text,
    translation,
    content = 'translations',
    content_rowid = 'id',
    tokenize = 'trigram'
);
CREATE TRIGGER translations_inserted AFTER INSERT ON translations BEGIN
    INSERT INTO translations_fts (rowid, source_text, translation)
    VALUES (new.id, new.source_text, new.translation);
END;
CREATE TRIGGER translations_deleted AFTER DELETE ON translations BEGIN
    INSERT INTO translations_fts (translations_fts, rowid, source_text, translation)
    VALUES ('delete', old.id, old.source_text, old.translation);
END;
";

const COLUMNS: &str =
    "id, source_text, translation, source_lang, target_lang, format, model, created_at";

/// The shortest term the trigram index can find: a trigram is three characters.
const INDEXED_TERM_CHARS: usize = 3;

/// One translation to save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record<'a> {
    pub source_text: &'a str,
    pub translation: &'a str,
    /// 翻訳元言語, or `None` when it was left to the model (doc-1 §6.1's 判定なし).
    pub source_lang: Option<&'a str>,
    pub target_lang: &'a str,
    /// The format's name as the screen gives it (`"text"`, `"markdown"`, `"json"`, `"arb"`,
    /// `"strings"`, `"html"`).
    pub format: &'a str,
    pub model: &'a str,
    /// Milliseconds since the Unix epoch.
    pub created_at: i64,
}

/// One saved translation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: i64,
    pub source_text: String,
    pub translation: String,
    pub source_lang: Option<String>,
    pub target_lang: String,
    pub format: String,
    pub model: String,
    pub created_at: i64,
}

impl Entry {
    fn from_row(row: &Row<'_>) -> rusqlite::Result<Entry> {
        Ok(Entry {
            id: row.get(0)?,
            source_text: row.get(1)?,
            translation: row.get(2)?,
            source_lang: row.get(3)?,
            target_lang: row.get(4)?,
            format: row.get(5)?,
            model: row.get(6)?,
            created_at: row.get(7)?,
        })
    }
}

pub struct History {
    conn: Connection,
    limit: usize,
}

impl History {
    /// Opens the history at `path`, creating it if it is not there, keeping at most `limit`
    /// translations.
    pub fn open(path: &Path, limit: usize) -> Result<History, String> {
        let conn = Connection::open(path).map_err(|err| err.to_string())?;
        History::with_connection(conn, limit)
    }

    fn with_connection(mut conn: Connection, limit: usize) -> Result<History, String> {
        migrate(&mut conn)?;
        Ok(History { conn, limit })
    }

    /// Saves `record`, then deletes the oldest translations beyond the limit. Answers the new
    /// entry's id.
    pub fn save(&mut self, record: &Record<'_>) -> rusqlite::Result<i64> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO translations
                (source_text, translation, source_lang, target_lang, format, model, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                record.source_text,
                record.translation,
                record.source_lang,
                record.target_lang,
                record.format,
                record.model,
                record.created_at,
            ],
        )?;
        let id = tx.last_insert_rowid();
        // By id rather than created_at: ids only grow, while the clock can be set back.
        tx.execute(
            "DELETE FROM translations WHERE id <= (
                 SELECT id FROM translations ORDER BY id DESC LIMIT 1 OFFSET ?1
             )",
            params![i64::try_from(self.limit).unwrap_or(i64::MAX)],
        )?;
        tx.commit()?;
        Ok(id)
    }

    /// The translations whose source or translation contains every whitespace-separated term of
    /// `query`, ignoring case, newest first. A blank `query` gives every translation.
    ///
    /// Terms of three characters or more go through the FTS5 index. Shorter ones, which hold no
    /// trigram — common in Japanese, where a word is often two characters — are looked for with
    /// `LIKE` instead; with the history capped at [`DEFAULT_LIMIT`]-odd entries, the scan this
    /// costs stays small.
    pub fn search(&self, query: &str) -> rusqlite::Result<Vec<Entry>> {
        let mut conditions = Vec::new();
        let mut values = Vec::new();
        for term in query.split_whitespace() {
            if term.chars().count() >= INDEXED_TERM_CHARS {
                conditions.push(
                    "id IN (SELECT rowid FROM translations_fts WHERE translations_fts MATCH ?)",
                );
                values.push(fts_phrase(term));
            } else {
                conditions
                    .push("(source_text LIKE ? ESCAPE '\\' OR translation LIKE ? ESCAPE '\\')");
                let pattern = like_pattern(term);
                values.push(pattern.clone());
                values.push(pattern);
            }
        }
        let filter = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };
        let sql = format!("SELECT {COLUMNS} FROM translations {filter} ORDER BY id DESC");
        let mut statement = self.conn.prepare(&sql)?;
        let entries = statement.query_map(params_from_iter(values), Entry::from_row)?;
        entries.collect()
    }

    /// How many translations the history holds, whatever a search would narrow them to.
    pub fn count(&self) -> rusqlite::Result<u32> {
        self.conn
            .query_row("SELECT count(*) FROM translations", [], |row| row.get(0))
    }

    /// Deletes one translation; `false` when there was none with `id`.
    pub fn delete(&self, id: i64) -> rusqlite::Result<bool> {
        let deleted = self
            .conn
            .execute("DELETE FROM translations WHERE id = ?1", params![id])?;
        Ok(deleted > 0)
    }

    /// Deletes every translation.
    pub fn clear(&self) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM translations", [])?;
        Ok(())
    }
}

/// Brings the schema up to [`SCHEMA_VERSION`], refusing a history a newer app has written: its
/// schema may hold what this version would lose.
fn migrate(conn: &mut Connection) -> Result<(), String> {
    let failed = |err: rusqlite::Error| err.to_string();
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(failed)?;
    if version > SCHEMA_VERSION {
        return Err(format!(
            "the history was written by a newer version (schema {version})"
        ));
    }
    if version == 0 {
        let tx = conn.transaction().map_err(failed)?;
        tx.execute_batch(SCHEMA).map_err(failed)?;
        tx.pragma_update(None, "user_version", SCHEMA_VERSION)
            .map_err(failed)?;
        tx.commit().map_err(failed)?;
    }
    Ok(())
}

/// `term` as an FTS5 string, which the query syntax reads as a phrase rather than as operators.
fn fts_phrase(term: &str) -> String {
    format!("\"{}\"", term.replace('"', "\"\""))
}

/// A `LIKE` pattern that matches `term` anywhere, with `term`'s own `%` and `_` taken literally.
fn like_pattern(term: &str) -> String {
    let escaped = term
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}

#[cfg(test)]
impl History {
    pub(crate) fn in_memory(limit: usize) -> History {
        History::with_connection(Connection::open_in_memory().unwrap(), limit).unwrap()
    }

    fn indexed(&self) -> i64 {
        // The index's own row count, not the content table's, so a stale index shows up.
        self.conn
            .query_row("SELECT count(*) FROM translations_fts_docsize", [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    fn get(&self, id: i64) -> Option<Entry> {
        use rusqlite::OptionalExtension;
        self.conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM translations WHERE id = ?1"),
                params![id],
                Entry::from_row,
            )
            .optional()
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record<'a>(source_text: &'a str, translation: &'a str) -> Record<'a> {
        Record {
            source_text,
            translation,
            source_lang: Some("English"),
            target_lang: "Japanese",
            format: "text",
            model: "plamo-3-translate-2b-Q4_K_M.gguf",
            created_at: 1_762_646_400_000,
        }
    }

    fn sources(entries: &[Entry]) -> Vec<&str> {
        entries.iter().map(|e| e.source_text.as_str()).collect()
    }

    fn sample() -> History {
        let mut history = History::in_memory(DEFAULT_LIMIT);
        history
            .save(&record("The weather is nice today.", "今日は天気が良い。"))
            .unwrap();
        history
            .save(&Record {
                source_lang: Some("Japanese"),
                target_lang: "English",
                ..record("翻訳履歴を検索する", "Search the translation history")
            })
            .unwrap();
        history
            .save(&record("Release notes", "リリースノート"))
            .unwrap();
        history
    }

    #[test]
    fn saves_every_field() {
        let mut history = History::in_memory(DEFAULT_LIMIT);
        let saved = Record {
            source_lang: None,
            format: "markdown",
            ..record("# Hello", "# こんにちは")
        };
        let id = history.save(&saved).unwrap();
        assert_eq!(
            history.get(id),
            Some(Entry {
                id,
                source_text: "# Hello".into(),
                translation: "# こんにちは".into(),
                source_lang: None,
                target_lang: "Japanese".into(),
                format: "markdown".into(),
                model: "plamo-3-translate-2b-Q4_K_M.gguf".into(),
                created_at: 1_762_646_400_000,
            })
        );
    }

    #[test]
    fn the_history_survives_reopening() {
        let path = std::env::temp_dir().join(format!(
            "tiny-translator-{}-history.sqlite3",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let id = History::open(&path, DEFAULT_LIMIT)
            .unwrap()
            .save(&record("Persisted", "保存された"))
            .unwrap();
        let reopened = History::open(&path, DEFAULT_LIMIT).unwrap();
        assert_eq!(sources(&reopened.search("保存").unwrap()), ["Persisted"]);
        assert_eq!(reopened.get(id).unwrap().translation, "保存された");
        drop(reopened);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn refuses_a_history_from_a_newer_schema() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .unwrap();
        assert!(History::with_connection(conn, DEFAULT_LIMIT).is_err());
    }

    #[test]
    fn a_blank_query_lists_everything_newest_first() {
        let history = sample();
        let expected = [
            "Release notes",
            "翻訳履歴を検索する",
            "The weather is nice today.",
        ];
        assert_eq!(sources(&history.search("").unwrap()), expected);
        assert_eq!(sources(&history.search("  \n").unwrap()), expected);
    }

    #[test]
    fn finds_english_in_the_source_and_the_translation_ignoring_case() {
        let history = sample();
        assert_eq!(
            sources(&history.search("WEATHER").unwrap()),
            ["The weather is nice today."]
        );
        assert_eq!(
            sources(&history.search("history").unwrap()),
            ["翻訳履歴を検索する"]
        );
    }

    #[test]
    fn finds_japanese_in_the_source_and_the_translation() {
        let history = sample();
        assert_eq!(
            sources(&history.search("履歴を検索").unwrap()),
            ["翻訳履歴を検索する"]
        );
        assert_eq!(
            sources(&history.search("リリース").unwrap()),
            ["Release notes"]
        );
    }

    #[test]
    fn finds_terms_too_short_for_a_trigram() {
        let history = sample();
        assert_eq!(
            sources(&history.search("天気").unwrap()),
            ["The weather is nice today."]
        );
        assert_eq!(
            sources(&history.search("履").unwrap()),
            ["翻訳履歴を検索する"]
        );
        assert_eq!(sources(&history.search("NO").unwrap()), ["Release notes"]);
    }

    #[test]
    fn every_term_must_match() {
        let history = sample();
        assert_eq!(
            sources(&history.search("天気 nice").unwrap()),
            ["The weather is nice today."]
        );
        assert!(history.search("天気 Release").unwrap().is_empty());
    }

    #[test]
    fn query_syntax_in_a_term_is_taken_literally() {
        let mut history = History::in_memory(DEFAULT_LIMIT);
        history
            .save(&record("100% \"quoted\" a_b", "100% 引用 a_b"))
            .unwrap();
        history.save(&record("100 percent", "百")).unwrap();
        history.save(&record("aXb", "x")).unwrap();
        assert_eq!(
            sources(&history.search("\"quoted\"").unwrap()),
            ["100% \"quoted\" a_b"]
        );
        assert_eq!(
            sources(&history.search("0%").unwrap()),
            ["100% \"quoted\" a_b"]
        );
        assert_eq!(
            sources(&history.search("a_").unwrap()),
            ["100% \"quoted\" a_b"]
        );
        assert!(history.search("NOT").unwrap().is_empty());
        assert!(history.search("OR*").unwrap().is_empty());
    }

    #[test]
    fn the_oldest_go_once_the_limit_is_passed() {
        let mut history = History::in_memory(3);
        for n in 1..=5 {
            history
                .save(&record(&format!("source {n}"), &format!("訳文 {n}")))
                .unwrap();
        }
        assert_eq!(history.count().unwrap(), 3);
        assert_eq!(history.indexed(), 3);
        assert_eq!(
            sources(&history.search("").unwrap()),
            ["source 5", "source 4", "source 3"]
        );
        assert!(history.search("source 1").unwrap().is_empty());
    }

    #[test]
    fn deletes_one_translation() {
        let mut history = sample();
        let id = history.save(&record("Delete me", "消して")).unwrap();
        assert!(history.delete(id).unwrap());
        assert!(!history.delete(id).unwrap());
        assert!(history.search("Delete me").unwrap().is_empty());
        assert!(history.search("消して").unwrap().is_empty());
        assert_eq!(history.count().unwrap(), 3);
        assert_eq!(history.indexed(), 3);
    }

    #[test]
    fn an_id_is_not_handed_out_again_after_its_entry_is_deleted() {
        let mut history = History::in_memory(DEFAULT_LIMIT);
        let first = history.save(&record("first", "一")).unwrap();
        history.delete(first).unwrap();
        let second = history.save(&record("second", "二")).unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn clears_every_translation() {
        let history = sample();
        history.clear().unwrap();
        assert!(history.search("").unwrap().is_empty());
        assert!(history.search("天気").unwrap().is_empty());
        assert_eq!(history.count().unwrap(), 0);
        assert_eq!(history.indexed(), 0);
    }
}
