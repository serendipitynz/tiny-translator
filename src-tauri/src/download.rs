//! モデルの入手 (doc-1 §10.4): fetches a GGUF from PFN's Hugging Face repository, at a commit the
//! app pins, checks it against the SHA-256 recorded with that commit, and only then puts it in the
//! models directory under its published name.
//!
//! The transfer is the OS's own curl run as a child process (2026-11-10 owner decision): macOS and
//! Windows 10 1803+ both ship one, so HTTPS needs no TLS stack in the app, and stopping a transfer
//! is ending a process, as it is for the engine. The file grows as `<name>.part`, which the models
//! listing does not take for a GGUF, so a stopped or failed transfer resumes where it left off.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

/// A GGUF the app can fetch (doc-1 §4.4's three sizes, Q4_K_M).
pub struct Catalogued {
    /// The file's name in the repository, which is also its name in the models directory.
    pub name: &'static str,
    /// The size the model is told apart by (doc-1 §4.4).
    pub size: &'static str,
    /// The Hugging Face repository the file is fetched from.
    pub repo: &'static str,
    /// The commit the file is fetched at. Pinned with `bytes` and `sha256`, so what is fetched is
    /// what was checked here, and with the licence the app shows before it (doc-1 §10.3's D3):
    /// these commits carry the 2026-11-09 licence that `src/licenses/` holds.
    revision: &'static str,
    pub bytes: u64,
    /// The LFS object's SHA-256, as the repository's API reports it for `revision`.
    sha256: &'static str,
}

pub const CATALOG: [Catalogued; 3] = [
    Catalogued {
        name: "plamo-3-translate-0.8b-Q4_K_M.gguf",
        size: "0.8B",
        repo: "pfnet/plamo-3-translate-0.8b-GGUF",
        revision: "1cf96ae02133958a20074bfdfab76517bb737d00",
        bytes: 573_126_848,
        sha256: "70891ec70dc7ab4205745a38307867c2881b99239f20327927633abfe4bc9ec6",
    },
    Catalogued {
        name: "plamo-3-translate-2b-Q4_K_M.gguf",
        size: "2B",
        repo: "pfnet/plamo-3-translate-2b-GGUF",
        revision: "4f672106b847200bfdc0f6af6c321cd94eba1aed",
        bytes: 1_807_203_040,
        sha256: "1ffd9073afcc70e35ccc942428aa2b73d7aa2b9ea779bf504b775369e0276c13",
    },
    Catalogued {
        name: "plamo-3-translate-8b-Q4_K_M.gguf",
        size: "8B",
        repo: "pfnet/plamo-3-translate-8b-GGUF",
        revision: "c67efa4eefdc14d3c97bf8f4f35ee58e2ecd2f42",
        bytes: 5_280_445_728,
        sha256: "94e5d3f91b421f7c8900ddaac27b11a19f7441426c954bbb25a2fd60a1640187",
    },
];

impl Catalogued {
    pub fn url(&self) -> String {
        format!(
            "https://huggingface.co/{}/resolve/{}/{}",
            self.repo, self.revision, self.name
        )
    }

    pub fn expected(&self) -> Expected<'static> {
        Expected {
            bytes: self.bytes,
            sha256: self.sha256,
        }
    }
}

pub fn catalogued(name: &str) -> Option<&'static Catalogued> {
    CATALOG.iter().find(|model| model.name == name)
}

/// What a fetched file must be before it is put in the models directory.
#[derive(Clone, Copy)]
pub struct Expected<'a> {
    pub bytes: u64,
    /// Lower-case hex.
    pub sha256: &'a str,
}

/// Where `name` grows while it is fetched.
pub fn part_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.part"))
}

/// How a transfer is made: which curl, and which protocols it may use.
pub struct Fetcher {
    program: PathBuf,
    protocols: &'static str,
}

impl Fetcher {
    /// The OS's own curl, by absolute path so that no other `curl` on the PATH stands in for it,
    /// held to HTTPS for the request and for every redirect (Hugging Face sends the file from a
    /// CDN on another host).
    pub fn system() -> Fetcher {
        Fetcher {
            program: system_curl(),
            protocols: "=https",
        }
    }
}

#[cfg(target_os = "macos")]
fn system_curl() -> PathBuf {
    PathBuf::from("/usr/bin/curl")
}

#[cfg(windows)]
fn system_curl() -> PathBuf {
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
    Path::new(&root).join("System32").join("curl.exe")
}

#[cfg(not(any(target_os = "macos", windows)))]
fn system_curl() -> PathBuf {
    PathBuf::from("/usr/bin/curl")
}

/// How often the transfer is looked at: for its progress, for having ended, and for a stop.
const POLL: Duration = Duration::from_millis(200);

/// How often progress is reported while the file is hashed, which reads far faster than POLL.
const REPORT_EVERY: Duration = Duration::from_millis(200);

/// One fetch, which another thread can stop with [`Job::cancel`].
#[derive(Default)]
pub struct Job {
    cancelled: AtomicBool,
    child: Mutex<Option<Child>>,
}

impl Job {
    /// Stops the fetch. The curl process is ended here, not left for the fetching thread to notice:
    /// at app exit that thread may never run again, and an orphaned curl would go on downloading.
    /// What was fetched so far stays, so the next fetch resumes from it.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Some(child) = self.lock().as_mut() {
            let _ = child.kill();
        }
    }

    fn cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    fn lock(&self) -> MutexGuard<'_, Option<Child>> {
        self.child
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// 取得中: the bytes are the `.part` file's size.
    Fetching,
    /// 照合中: the bytes are how much of the file has been hashed.
    Verifying,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The GGUF is in the models directory.
    Done,
    /// Stopped by [`Job::cancel`]; the `.part` file stays for the next fetch to resume.
    Cancelled,
}

/// Fetches `url` into `dir` as `name`, reporting progress as it goes. The file is put in place only
/// once its size and SHA-256 are `expected`; a file that turns out otherwise is deleted, since
/// resuming from it would only reproduce it.
pub fn fetch(
    job: &Job,
    fetcher: &Fetcher,
    url: &str,
    expected: Expected<'_>,
    dir: &Path,
    name: &str,
    progress: &mut dyn FnMut(Phase, u64),
) -> Result<Outcome, String> {
    let target = dir.join(name);
    if target.exists() {
        return Err(format!("{name} is already in {}", dir.display()));
    }
    fs::create_dir_all(dir).map_err(|err| format!("could not make {}: {err}", dir.display()))?;
    let part = part_path(dir, name);

    if size_of(&part) > expected.bytes {
        remove(&part)?;
    }
    if size_of(&part) < expected.bytes {
        if let Some(outcome) = transfer(job, fetcher, url, &part, progress)? {
            return Ok(outcome);
        }
        let fetched = size_of(&part);
        if fetched != expected.bytes {
            if fetched > expected.bytes {
                remove(&part)?;
            }
            return Err(format!(
                "fetched {fetched} bytes where {} were expected",
                expected.bytes
            ));
        }
    }

    progress(Phase::Verifying, 0);
    let Some(digest) = sha256(job, &part, expected.bytes, progress)? else {
        return Ok(Outcome::Cancelled);
    };
    if !digest.eq_ignore_ascii_case(expected.sha256) {
        remove(&part)?;
        return Err(format!(
            "the file's SHA-256 is {digest}, not {}; it was deleted",
            expected.sha256
        ));
    }
    fs::rename(&part, &target).map_err(|err| format!("could not put {name} in place: {err}"))?;
    Ok(Outcome::Done)
}

/// Runs curl until it ends. `Some(Cancelled)` when it was stopped; `None` when it finished.
fn transfer(
    job: &Job,
    fetcher: &Fetcher,
    url: &str,
    part: &Path,
    progress: &mut dyn FnMut(Phase, u64),
) -> Result<Option<Outcome>, String> {
    let mut command = crate::engine::windowless(&fetcher.program);
    command
        .args([
            "--silent",
            "--show-error",
            "--fail",
            "--location",
            "--proto",
            fetcher.protocols,
            "--proto-redir",
            fetcher.protocols,
            // Resumes from the `.part` file's end, also on curl's own retries.
            "--continue-at",
            "-",
            "--retry",
            "3",
            "--connect-timeout",
            "30",
            // A transfer that has stalled for a minute counts as failed (and is retried), rather
            // than leaving the progress standing still for good.
            "--speed-limit",
            "1",
            "--speed-time",
            "60",
            "--output",
        ])
        // By name from inside the directory: the path to it holds the user's name, which curl.exe
        // may not read correctly outside the ANSI code page, while the catalog's names are ASCII.
        .arg(part.file_name().expect("a part file has a name"))
        .current_dir(part.parent().expect("a part file has a directory"))
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|err| format!("could not run {}: {err}", fetcher.program.display()))?;
    // Read after the exit: with --silent, curl writes one line at most, which the pipe holds.
    let stderr = child.stderr.take();
    {
        let mut slot = job.lock();
        if job.cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(Some(Outcome::Cancelled));
        }
        *slot = Some(child);
    }

    let status = loop {
        progress(Phase::Fetching, size_of(part));
        let ended = {
            let mut slot = job.lock();
            let child = slot.as_mut().expect("the transfer's process went missing");
            match child.try_wait() {
                Ok(Some(status)) => Some(Ok(status)),
                Ok(None) => None,
                Err(err) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    Some(Err(format!("could not check the transfer: {err}")))
                }
            }
        };
        if let Some(ended) = ended {
            job.lock().take();
            break ended?;
        }
        thread::sleep(POLL);
    };

    if job.cancelled() {
        return Ok(Some(Outcome::Cancelled));
    }
    progress(Phase::Fetching, size_of(part));
    if status.success() {
        return Ok(None);
    }
    // 33: the server would not resume. The `.part` is of no use to the next try either.
    if status.code() == Some(33) {
        remove(part)?;
    }
    let mut message = String::new();
    if let Some(mut stderr) = stderr {
        let _ = stderr.read_to_string(&mut message);
    }
    let message = message.trim();
    Err(if message.is_empty() {
        format!("curl ended with {status}")
    } else {
        message.to_owned()
    })
}

/// The file's SHA-256 as lower-case hex, or `None` when the job was stopped.
fn sha256(
    job: &Job,
    path: &Path,
    total: u64,
    progress: &mut dyn FnMut(Phase, u64),
) -> Result<Option<String>, String> {
    let failed = |err: io::Error| format!("could not read {}: {err}", path.display());
    let mut file = File::open(path).map_err(failed)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 1 << 20];
    let mut hashed = 0u64;
    let mut reported = Instant::now();
    loop {
        if job.cancelled() {
            return Ok(None);
        }
        let read = file.read(&mut buffer).map_err(failed)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        hashed += read as u64;
        if reported.elapsed() >= REPORT_EVERY {
            progress(Phase::Verifying, hashed.min(total));
            reported = Instant::now();
        }
    }
    progress(Phase::Verifying, hashed.min(total));
    Ok(Some(
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    ))
}

fn size_of(path: &Path) -> u64 {
    fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

fn remove(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("could not delete {}: {err}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// curl over `file://`: the same transfer, resume and failure handling as over HTTPS, without
    /// the network.
    fn local() -> Fetcher {
        Fetcher {
            program: system_curl(),
            protocols: "=file",
        }
    }

    fn file_url(path: &Path) -> String {
        let path = path.to_string_lossy().replace('\\', "/");
        if path.starts_with('/') {
            format!("file://{path}")
        } else {
            format!("file:///{path}")
        }
    }

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Scratch {
            let dir = std::env::temp_dir().join(format!(
                "tiny-translator-download-{}-{name}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(dir.join("models")).unwrap();
            Scratch(dir)
        }

        fn models(&self) -> PathBuf {
            self.0.join("models")
        }

        /// A source file of `bytes` bytes, with the digest that makes it what is expected.
        fn source(&self, bytes: usize) -> (PathBuf, String) {
            let content: Vec<u8> = (0..bytes).map(|i| (i % 251) as u8).collect();
            let path = self.0.join("source.bin");
            fs::write(&path, &content).unwrap();
            let digest = Sha256::digest(&content)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            (path, digest)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const NAME: &str = "stand-in.gguf";

    fn run(
        job: &Job,
        scratch: &Scratch,
        url: &str,
        expected: Expected<'_>,
    ) -> (Result<Outcome, String>, Vec<(Phase, u64)>) {
        let mut seen = Vec::new();
        let result = fetch(
            job,
            &local(),
            url,
            expected,
            &scratch.models(),
            NAME,
            &mut |phase, bytes| seen.push((phase, bytes)),
        );
        (result, seen)
    }

    #[test]
    fn a_matching_file_is_put_in_place_under_its_name() {
        let scratch = Scratch::new("match");
        let (source, digest) = scratch.source(3 << 20);
        let expected = Expected {
            bytes: 3 << 20,
            sha256: &digest,
        };

        let (result, seen) = run(&Job::default(), &scratch, &file_url(&source), expected);

        assert_eq!(result, Ok(Outcome::Done));
        assert_eq!(
            fs::read(scratch.models().join(NAME)).unwrap(),
            fs::read(&source).unwrap()
        );
        assert!(!part_path(&scratch.models(), NAME).exists());
        assert_eq!(seen.last(), Some(&(Phase::Verifying, 3 << 20)));
        assert!(seen.contains(&(Phase::Fetching, 3 << 20)));
    }

    #[test]
    fn a_partial_file_is_resumed_from_its_end() {
        let scratch = Scratch::new("resume");
        let (source, digest) = scratch.source(2 << 20);
        let content = fs::read(&source).unwrap();
        let part = part_path(&scratch.models(), NAME);
        fs::write(&part, &content[..700_000]).unwrap();

        let (result, seen) = run(
            &Job::default(),
            &scratch,
            &file_url(&source),
            Expected {
                bytes: 2 << 20,
                sha256: &digest,
            },
        );

        assert_eq!(result, Ok(Outcome::Done));
        assert_eq!(fs::read(scratch.models().join(NAME)).unwrap(), content);
        assert_eq!(seen.first(), Some(&(Phase::Fetching, 700_000)));
    }

    #[test]
    fn a_complete_partial_file_is_only_verified() {
        let scratch = Scratch::new("complete-part");
        let (source, digest) = scratch.source(1 << 20);
        fs::copy(&source, part_path(&scratch.models(), NAME)).unwrap();

        // An address curl cannot read: the transfer must not be attempted at all.
        let missing = file_url(&scratch.0.join("missing.bin"));
        let (result, seen) = run(
            &Job::default(),
            &scratch,
            &missing,
            Expected {
                bytes: 1 << 20,
                sha256: &digest,
            },
        );

        assert_eq!(result, Ok(Outcome::Done));
        assert!(seen.iter().all(|(phase, _)| *phase == Phase::Verifying));
    }

    #[test]
    fn a_file_that_does_not_match_is_deleted() {
        let scratch = Scratch::new("mismatch");
        let (source, _) = scratch.source(1 << 20);
        let wrong = "0".repeat(64);

        let (result, _) = run(
            &Job::default(),
            &scratch,
            &file_url(&source),
            Expected {
                bytes: 1 << 20,
                sha256: &wrong,
            },
        );

        let error = result.unwrap_err();
        assert!(error.contains("SHA-256"), "{error}");
        assert!(!scratch.models().join(NAME).exists());
        assert!(!part_path(&scratch.models(), NAME).exists());
    }

    #[test]
    fn a_partial_file_longer_than_expected_is_fetched_again() {
        let scratch = Scratch::new("too-long");
        let (source, digest) = scratch.source(1 << 20);
        fs::write(
            part_path(&scratch.models(), NAME),
            vec![7u8; (1 << 20) + 10],
        )
        .unwrap();

        let (result, _) = run(
            &Job::default(),
            &scratch,
            &file_url(&source),
            Expected {
                bytes: 1 << 20,
                sha256: &digest,
            },
        );

        assert_eq!(result, Ok(Outcome::Done));
        assert_eq!(
            fs::read(scratch.models().join(NAME)).unwrap(),
            fs::read(&source).unwrap()
        );
    }

    #[test]
    fn a_file_of_the_wrong_size_is_not_verified() {
        let scratch = Scratch::new("short");
        let (source, digest) = scratch.source(1 << 20);

        let (result, seen) = run(
            &Job::default(),
            &scratch,
            &file_url(&source),
            Expected {
                bytes: 2 << 20,
                sha256: &digest,
            },
        );

        assert!(result.unwrap_err().contains("expected"));
        assert!(seen.iter().all(|(phase, _)| *phase == Phase::Fetching));
        assert!(!scratch.models().join(NAME).exists());
    }

    #[test]
    fn a_failed_transfer_says_why_and_puts_nothing_in_place() {
        let scratch = Scratch::new("failed");
        let missing = file_url(&scratch.0.join("missing.bin"));

        let (result, _) = run(
            &Job::default(),
            &scratch,
            &missing,
            Expected {
                bytes: 10,
                sha256: "",
            },
        );

        assert!(result.unwrap_err().starts_with("curl:"));
        assert!(!scratch.models().join(NAME).exists());
    }

    #[test]
    fn https_only_refuses_another_protocol() {
        let scratch = Scratch::new("https-only");
        let (source, digest) = scratch.source(1024);

        let result = fetch(
            &Job::default(),
            &Fetcher::system(),
            &file_url(&source),
            Expected {
                bytes: 1024,
                sha256: &digest,
            },
            &scratch.models(),
            NAME,
            &mut |_, _| {},
        );

        assert!(result.is_err());
        assert!(!scratch.models().join(NAME).exists());
    }

    #[test]
    fn a_model_already_in_place_is_not_fetched_over() {
        let scratch = Scratch::new("present");
        let (source, digest) = scratch.source(1024);
        fs::write(scratch.models().join(NAME), b"mine").unwrap();

        let (result, _) = run(
            &Job::default(),
            &scratch,
            &file_url(&source),
            Expected {
                bytes: 1024,
                sha256: &digest,
            },
        );

        assert!(result.is_err());
        assert_eq!(fs::read(scratch.models().join(NAME)).unwrap(), b"mine");
    }

    #[test]
    fn a_cancelled_job_keeps_what_it_fetched() {
        let scratch = Scratch::new("cancel");
        let (source, digest) = scratch.source(4 << 20);
        let job = Arc::new(Job::default());
        let canceller = Arc::clone(&job);

        // Cancelled at the first report, while hashing a complete `.part`: the point at which a
        // stop comes is then the same on every run.
        fs::copy(&source, part_path(&scratch.models(), NAME)).unwrap();
        let result = fetch(
            &job,
            &local(),
            &file_url(&source),
            Expected {
                bytes: 4 << 20,
                sha256: &digest,
            },
            &scratch.models(),
            NAME,
            &mut |_, _| canceller.cancel(),
        );

        assert_eq!(result, Ok(Outcome::Cancelled));
        assert!(part_path(&scratch.models(), NAME).exists());
        assert!(!scratch.models().join(NAME).exists());
    }

    #[test]
    fn a_job_cancelled_before_the_transfer_starts_none() {
        let scratch = Scratch::new("cancel-first");
        let (source, digest) = scratch.source(1024);
        let job = Job::default();
        job.cancel();

        let (result, seen) = run(
            &job,
            &scratch,
            &file_url(&source),
            Expected {
                bytes: 1024,
                sha256: &digest,
            },
        );

        assert_eq!(result, Ok(Outcome::Cancelled));
        assert!(seen.is_empty());
    }

    /// The whole path against Hugging Face: `cargo test -- --ignored fetches_the_real`. Fetches
    /// 0.8B (about 550 MB), so it is not part of the usual run.
    #[test]
    #[ignore]
    fn fetches_the_real_smallest_model() {
        let scratch = Scratch::new("real");
        let model = &CATALOG[0];
        let mut last = None;
        let started = Instant::now();

        let result = fetch(
            &Job::default(),
            &Fetcher::system(),
            &model.url(),
            model.expected(),
            &scratch.models(),
            model.name,
            &mut |phase, bytes| last = Some((phase, bytes)),
        );

        eprintln!("{} in {:?}", model.name, started.elapsed());
        assert_eq!(result, Ok(Outcome::Done));
        assert_eq!(last, Some((Phase::Verifying, model.bytes)));
        assert_eq!(
            fs::metadata(scratch.models().join(model.name))
                .unwrap()
                .len(),
            model.bytes
        );
    }

    #[test]
    fn the_licence_the_screen_shows_is_the_pinned_commits_own() {
        // The SHA-256 of `LICENSE/ja` and `LICENSE/en`, the same at each commit in CATALOG: the
        // screen shows these bytes before a fetch (doc-1 §10.3's D3), so they must not drift.
        let texts: [(&[u8], &str); 2] = [
            (
                include_bytes!("../../src/licenses/plamo-community-license.ja.txt"),
                "7dd74339bbf9a39fe33e271a89657aacf06f452b082fcb1273fb792b8d6b7db2",
            ),
            (
                include_bytes!("../../src/licenses/plamo-community-license.en.txt"),
                "ca0c5e90f6de6799fc2d2d4f2b2647cc088444c19f286f689e525d4ad28f9ed6",
            ),
        ];
        for (text, expected) in texts {
            let digest: String = Sha256::digest(text)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            assert_eq!(digest, expected);
        }
    }

    #[test]
    fn the_catalog_holds_the_default_model_and_pins_each_file() {
        assert!(catalogued(crate::engine::DEFAULT_MODEL).is_some());
        for model in &CATALOG {
            assert_eq!(model.revision.len(), 40, "{}", model.name);
            assert_eq!(model.sha256.len(), 64, "{}", model.name);
            assert!(model.url().starts_with("https://huggingface.co/pfnet/"));
            assert!(
                model
                    .url()
                    .ends_with(&format!("/resolve/{}/{}", model.revision, model.name))
            );
        }
    }
}
