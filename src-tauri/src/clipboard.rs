//! 起動時のクリップボード翻訳 (doc-1 §6.3): whether the clipboard may be read at launch, and
//! whether what it holds was already translated at the previous launch.

use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::Mutex;

/// Whether reading the clipboard now would go ahead without macOS asking the user first. Asking
/// is checked, not triggered: only reading asks.
#[cfg(target_os = "macos")]
pub fn readable_without_asking() -> bool {
    use objc2_app_kit::{NSPasteboard, NSPasteboardAccessBehavior};
    // Before macOS 15.4 there is no `accessBehavior`, and no asking either; calling the missing
    // selector would abort the app.
    if !objc2::available!(macos = 15.4) {
        return true;
    }
    NSPasteboard::generalPasteboard().accessBehavior() == NSPasteboardAccessBehavior::AlwaysAllow
}

/// Windows has nothing like macOS' `accessBehavior`: reading never asks.
#[cfg(not(target_os = "macos"))]
pub fn readable_without_asking() -> bool {
    true
}

/// The clipboard text taken at the previous launch, kept as a digest so that whatever the
/// clipboard held — a password, say — is never written to disk.
pub struct LaunchMemory {
    path: PathBuf,
    /// `None` until first read from `path`. Behind the lock so a first launch and a second one
    /// that arrives meanwhile cannot both take the same text.
    last: Mutex<Option<String>>,
}

impl LaunchMemory {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            last: Mutex::new(None),
        }
    }

    /// Whether `text` is this launch's text to translate: more than whitespace, and not the text
    /// taken at the previous launch. Remembers it when it is.
    pub fn take(&self, text: &str) -> bool {
        if text.trim().is_empty() {
            return false;
        }
        let mut last = self.last.lock().unwrap_or_else(|e| e.into_inner());
        let previous = last.get_or_insert_with(|| {
            fs::read_to_string(&self.path)
                .map(|s| s.trim().to_owned())
                .unwrap_or_default()
        });
        let current = digest(text);
        if *previous == current {
            return false;
        }
        // A write that fails only costs translating the same text again at the next launch.
        let _ = fs::write(&self.path, &current);
        *previous = current;
        true
    }
}

/// `DefaultHasher::new` hashes with fixed keys, so the digest is the same from launch to launch.
/// Rust may change its algorithm in a later release, which only costs one repeated translation —
/// not worth a hash crate.
fn digest(text: &str) -> String {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_in(dir: &std::path::Path) -> LaunchMemory {
        LaunchMemory::new(dir.join("launch-clipboard"))
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "tiny-translator-clipboard-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn takes_new_text_once() {
        let dir = temp_dir("once");
        let memory = memory_in(&dir);
        assert!(memory.take("Hello"));
        assert!(!memory.take("Hello"));
        assert!(memory.take("World"));
        assert!(memory.take("Hello"));
    }

    #[test]
    fn remembers_across_launches() {
        let dir = temp_dir("across");
        assert!(memory_in(&dir).take("Hello"));
        assert!(!memory_in(&dir).take("Hello"));
        assert!(memory_in(&dir).take("Hello, world"));
    }

    #[test]
    fn leaves_whitespace_alone_and_does_not_remember_it() {
        let dir = temp_dir("blank");
        let memory = memory_in(&dir);
        assert!(memory.take("Hello"));
        assert!(!memory.take(""));
        assert!(!memory.take(" \n\t"));
        assert!(!memory.take("Hello"));
    }

    #[test]
    fn does_not_write_the_text_itself() {
        let dir = temp_dir("digest");
        assert!(memory_in(&dir).take("secret-password"));
        let stored = fs::read_to_string(dir.join("launch-clipboard")).unwrap();
        assert!(!stored.contains("secret"));
    }

    #[test]
    fn takes_text_when_nothing_can_be_stored() {
        let dir = temp_dir("unwritable");
        let memory = LaunchMemory::new(dir.join("missing").join("launch-clipboard"));
        assert!(memory.take("Hello"));
        assert!(!memory.take("Hello"));
    }
}
