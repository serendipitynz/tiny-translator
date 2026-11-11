//! アプリの自動更新 (doc-1 §14, TASK-30): asks the release's `latest.json` whether a newer version
//! is published, and, when the user says so, downloads it, checks its signature and puts it in
//! place. The asking and the replacing are tauri-plugin-updater's; what is here is when the app asks
//! and what the screen is told.
//!
//! The check is the one network use R1 allows besides fetching a model (doc-1 §14), so it is made
//! at most once a day at launch, can be turned off in the settings, and never reports a failure the
//! user did not ask for: no network is the normal case on a machine R1 is written for.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use serde_json::{Value, json};
use tauri_plugin_updater::Update;

/// How long a check at launch is good for. tanakh/plamo-translate-desktop asks once a day too
/// (TASK-30): often enough that a fix arrives within a day of the user's next launch, rarely
/// enough that the request is not a trace of every launch.
pub const CHECK_INTERVAL_MS: i64 = 24 * 60 * 60 * 1000;

/// Whether a check at launch is due, the question having last been settled at `last` (milliseconds
/// since the Unix epoch). A `last` in the future means the clock was set back, and is not trusted
/// to put the next check off for however long that is.
pub fn due(last: Option<i64>, now: i64) -> bool {
    match last {
        None => true,
        Some(last) => now < last || now - last >= CHECK_INTERVAL_MS,
    }
}

/// Where in the update the app stands, as the screen draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    /// No newer version is known, or the user set the known one aside.
    None,
    Available,
    Downloading {
        received: u64,
        total: Option<u64>,
    },
    /// Downloaded and verified; being put in place, after which the app restarts.
    Installing,
    /// The download, the signature check or the replacing failed; the update can be tried again.
    Failed {
        reason: String,
    },
}

/// What the `update-state` event and `update_state` carry: `{ phase }`, with the version for every
/// phase but `none`, `received` and `total` (null when the server did not say) while downloading,
/// and `reason` for `failed`.
pub fn payload(phase: &Phase, version: Option<&str>) -> Value {
    match (phase, version) {
        (Phase::None, _) | (_, None) => json!({ "phase": "none" }),
        (Phase::Available, Some(version)) => json!({ "phase": "available", "version": version }),
        (Phase::Downloading { received, total }, Some(version)) => json!({
            "phase": "downloading",
            "version": version,
            "received": received,
            "total": total,
        }),
        (Phase::Installing, Some(version)) => json!({ "phase": "installing", "version": version }),
        (Phase::Failed { reason }, Some(version)) => {
            json!({ "phase": "failed", "version": version, "reason": reason })
        }
    }
}

/// The update the last check found, and where the app stands with it.
pub struct Updates {
    inner: Mutex<Inner>,
    /// The file holding when the question was last settled, which spaces checks across launches:
    /// when a check found no newer version, or the user set the one it found aside. A check that
    /// finds one does not write it, so the offer comes back at the next launch until the user
    /// either takes it or sets it aside; one that fails does not either, so the first launch with
    /// network checks.
    stamp: PathBuf,
}

struct Inner {
    pending: Option<Update>,
    phase: Phase,
}

impl Updates {
    pub fn new(data_dir: &Path) -> Updates {
        Updates {
            inner: Mutex::new(Inner {
                pending: None,
                phase: Phase::None,
            }),
            stamp: data_dir.join("update-checked"),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn payload(&self) -> Value {
        let inner = self.lock();
        payload(
            &inner.phase,
            inner.pending.as_ref().map(|update| update.version.as_str()),
        )
    }

    /// Whether a check at launch is due at `now`.
    pub fn launch_check_due(&self, now: i64) -> bool {
        due(read_stamp(&self.stamp), now)
    }

    /// Whether an update is being downloaded or put in place, when a check must leave it alone.
    pub fn busy(&self) -> bool {
        matches!(
            self.lock().phase,
            Phase::Downloading { .. } | Phase::Installing
        )
    }

    /// Records what a check that succeeded at `now` found. An update already offered stays as it
    /// was when the check finds the same version, so a failure the screen shows is not wiped by a
    /// second check. Answers whether the screen has to be told.
    pub fn found(&self, update: Option<Update>, now: i64) -> bool {
        if update.is_none() {
            self.settle(now);
        }
        let mut inner = self.lock();
        if matches!(inner.phase, Phase::Downloading { .. } | Phase::Installing) {
            return false;
        }
        let same = match (&inner.pending, &update) {
            (Some(known), Some(new)) => known.version == new.version,
            (None, None) => true,
            _ => false,
        };
        if same {
            return false;
        }
        inner.phase = if update.is_some() {
            Phase::Available
        } else {
            Phase::None
        };
        inner.pending = update;
        true
    }

    /// The update to install, now marked as downloading; an error when there is none or one is
    /// already under way.
    pub fn begin(&self) -> Result<Update, String> {
        let mut inner = self.lock();
        if matches!(inner.phase, Phase::Downloading { .. } | Phase::Installing) {
            return Err("the update is already being installed".into());
        }
        let update = inner
            .pending
            .clone()
            .ok_or_else(|| "there is no update to install".to_owned())?;
        inner.phase = Phase::Downloading {
            received: 0,
            total: None,
        };
        Ok(update)
    }

    pub fn set_phase(&self, phase: Phase) {
        self.lock().phase = phase;
    }

    /// Sets the offered update aside at `now`: a check at launch offers it again a day later.
    pub fn dismiss(&self, now: i64) {
        let mut inner = self.lock();
        if matches!(inner.phase, Phase::Downloading { .. } | Phase::Installing) {
            return;
        }
        inner.pending = None;
        inner.phase = Phase::None;
        drop(inner);
        self.settle(now);
    }

    fn settle(&self, now: i64) {
        if let Err(err) = fs::write(&self.stamp, now.to_string()) {
            eprintln!("could not record the update check: {err}");
        }
    }
}

fn read_stamp(path: &Path) -> Option<i64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_762_800_000_000;

    #[test]
    fn a_first_launch_checks() {
        assert!(due(None, NOW));
    }

    #[test]
    fn a_check_lasts_a_day() {
        assert!(!due(Some(NOW - CHECK_INTERVAL_MS + 1), NOW));
        assert!(due(Some(NOW - CHECK_INTERVAL_MS), NOW));
    }

    #[test]
    fn a_clock_set_back_does_not_put_the_check_off() {
        assert!(due(Some(NOW + 1), NOW));
    }

    #[test]
    fn the_stamp_spaces_checks_across_launches() {
        let dir =
            std::env::temp_dir().join(format!("tiny-translator-update-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let updates = Updates::new(&dir);
        assert!(updates.launch_check_due(NOW));
        assert!(!updates.found(None, NOW));
        assert!(!Updates::new(&dir).launch_check_due(NOW + 1));
        assert!(Updates::new(&dir).launch_check_due(NOW + CHECK_INTERVAL_MS));
        fs::write(dir.join("update-checked"), "not a time").unwrap();
        assert!(Updates::new(&dir).launch_check_due(NOW));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn setting_an_update_aside_puts_the_next_check_off_a_day() {
        let dir = std::env::temp_dir().join(format!(
            "tiny-translator-update-dismiss-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let updates = Updates::new(&dir);
        updates.dismiss(NOW);
        assert!(!updates.launch_check_due(NOW + 1));
        assert!(updates.launch_check_due(NOW + CHECK_INTERVAL_MS));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn nothing_to_install_is_an_error() {
        let updates = Updates::new(Path::new("/nonexistent"));
        assert!(updates.begin().is_err());
        assert_eq!(updates.payload(), json!({ "phase": "none" }));
    }

    #[test]
    fn the_screen_is_told_the_version_with_every_phase() {
        let version = Some("0.2.0");
        assert_eq!(
            payload(&Phase::Available, version),
            json!({ "phase": "available", "version": "0.2.0" })
        );
        assert_eq!(
            payload(
                &Phase::Downloading {
                    received: 10,
                    total: None
                },
                version
            ),
            json!({ "phase": "downloading", "version": "0.2.0", "received": 10, "total": null })
        );
        assert_eq!(
            payload(&Phase::Installing, version),
            json!({ "phase": "installing", "version": "0.2.0" })
        );
        assert_eq!(
            payload(
                &Phase::Failed {
                    reason: "bad signature".into()
                },
                version
            ),
            json!({ "phase": "failed", "version": "0.2.0", "reason": "bad signature" })
        );
        assert_eq!(payload(&Phase::Available, None), json!({ "phase": "none" }));
    }
}
