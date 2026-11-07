//! 翻訳エンジン (doc-1 §1): the llama-server child process that serves PLaMo 3 Translate.
//!
//! The engine is a child process so that ending it returns its memory to the OS (doc-1 §4.1), and
//! it is ended whenever nothing has used it for 自動解放までの時間 (doc-1 §4.5). Everything that
//! talks to it holds a [`Lease`] for as long as it does, so the release never cuts off a
//! translation, and asks for one through [`Engine::acquire`], which starts the engine again when it
//! has been released.
//!
//! Nothing here knows about Tauri: what the process is ([`Spawn`]) and who hears about state
//! changes ([`Notify`]) are handed in, which is what lets the tests drive a stand-in process.

use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

/// 自動解放までの時間の既定値 (doc-1 §4.5).
pub const DEFAULT_IDLE_RELEASE: Duration = Duration::from_secs(10 * 60);

/// How long loading may take before the engine is given up on. TASK-1 measured 1.3–3.3 s for a
/// GGUF already in the page cache, and about 10 s for the first start after a download; this
/// leaves room for an 8B read from a slow disk without leaving a stuck process up indefinitely.
const LOAD_TIMEOUT: Duration = Duration::from_secs(180);

/// How many processes one start may spawn. The OS can keep a port it has just handed out by
/// [`free_port`] busy for a moment after it is released (seen on macOS as `EADDRINUSE` that clears
/// within 10 ms), and llama-server then exits at once; a fresh port almost always works.
const START_ATTEMPTS: u32 = 3;

const HEALTH_POLL: Duration = Duration::from_millis(100);
const HEALTH_TIMEOUT: Duration = Duration::from_millis(500);

/// How often a running engine is checked for having exited on its own, so a crash shows as 停止中
/// instead of a 待機中 that the next translation would find dead.
const EXIT_POLL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineState {
    /// 停止中
    Stopped,
    /// 読み込み中
    Loading,
    /// 待機中
    Ready,
}

impl EngineState {
    /// The name the screen receives (`src/lib/engine.ts`).
    pub fn as_str(self) -> &'static str {
        match self {
            EngineState::Stopped => "stopped",
            EngineState::Loading => "loading",
            EngineState::Ready => "ready",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub state: EngineState,
    /// Why the last start failed or the engine stopped on its own. Cleared once it is 待機中 again.
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcquireError {
    /// The engine could not be started or did not finish loading; the text is [`Status::error`].
    Failed(String),
    ShuttingDown,
}

impl fmt::Display for AcquireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AcquireError::Failed(reason) => write!(f, "{reason}"),
            AcquireError::ShuttingDown => write!(f, "the app is shutting down"),
        }
    }
}

/// Starts the engine process listening on 127.0.0.1 at the given port.
pub type Spawn = Box<dyn Fn(u16) -> io::Result<Child> + Send + Sync>;
/// Hears every change of [`Status`]. Called with the engine's lock held, so it must not call back
/// into the [`Engine`].
pub type Notify = Box<dyn Fn(&Status) + Send + Sync>;

pub struct Engine {
    shared: Arc<Shared>,
}

/// Proof that the engine is 待機中 and stays up: the release waits until every lease is dropped.
pub struct Lease {
    shared: Arc<Shared>,
    port: u16,
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "translating (TASK-9) is the first caller")
)]
impl Lease {
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for Lease {
    fn drop(&mut self) {
        let mut inner = self.shared.lock();
        inner.leases -= 1;
        if inner.leases == 0 && inner.state == EngineState::Ready {
            // 自動解放までの時間 counts from the end of the last translation (doc-1 §1).
            inner.idle_since = Some(Instant::now());
            self.shared.changed.notify_all();
        }
    }
}

struct Shared {
    inner: Mutex<Inner>,
    changed: Condvar,
    spawn: Spawn,
    notify: Notify,
    pid_file: Option<PathBuf>,
}

struct Inner {
    state: EngineState,
    error: Option<String>,
    process: Option<Running>,
    /// Bumped on every spawned process, so a loader thread that outlived its process can tell.
    generation: u64,
    /// Bumped on every start (which may spawn several processes), so [`Engine::acquire`] can tell
    /// whether 停止中 is the end of its own start.
    start_id: u64,
    leases: usize,
    idle_since: Option<Instant>,
    /// `None` is 自動解放までの時間 = 0, "never release".
    idle_release: Option<Duration>,
    shutting_down: bool,
}

struct Running {
    child: Child,
    port: u16,
}

impl Engine {
    /// `pid_file` records the running engine's PID so that the next launch can end a process an
    /// abnormal exit left behind ([`kill_leftover`]).
    pub fn new(spawn: Spawn, notify: Notify, pid_file: Option<PathBuf>) -> Engine {
        let shared = Arc::new(Shared {
            inner: Mutex::new(Inner {
                state: EngineState::Stopped,
                error: None,
                process: None,
                generation: 0,
                start_id: 0,
                leases: 0,
                idle_since: None,
                idle_release: Some(DEFAULT_IDLE_RELEASE),
                shutting_down: false,
            }),
            changed: Condvar::new(),
            spawn,
            notify,
            pid_file,
        });
        let supervisor = Arc::clone(&shared);
        thread::spawn(move || supervise(&supervisor));
        Engine { shared }
    }

    pub fn status(&self) -> Status {
        self.shared.lock().status()
    }

    /// Starts loading without waiting for it, unless the engine is already up or loading. A failed
    /// start is reported through [`Status::error`].
    pub fn start(&self) {
        let mut inner = self.shared.lock();
        if inner.state == EngineState::Stopped && !inner.shutting_down {
            let _ = self.shared.start(&mut inner);
        }
    }

    /// Waits until the engine is 待機中, starting it if it is 停止中, and holds it there until the
    /// lease is dropped.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "translating (TASK-9) is the first caller")
    )]
    pub fn acquire(&self) -> Result<Lease, AcquireError> {
        let mut inner = self.shared.lock();
        let mut started = None;
        loop {
            if inner.shutting_down {
                return Err(AcquireError::ShuttingDown);
            }
            match inner.state {
                EngineState::Ready => {
                    let port = inner
                        .process
                        .as_ref()
                        .map(|p| p.port)
                        .expect("ready without a process");
                    inner.leases += 1;
                    inner.idle_since = None;
                    return Ok(Lease {
                        shared: Arc::clone(&self.shared),
                        port,
                    });
                }
                EngineState::Loading => {
                    inner = self.shared.wait(inner);
                }
                EngineState::Stopped => {
                    // Our own start ended in 停止中: report it rather than retry forever. A start
                    // somebody else made since then (a newer start_id) is still worth waiting on.
                    if started == Some(inner.start_id) {
                        let reason = inner
                            .error
                            .clone()
                            .unwrap_or_else(|| "the engine stopped".into());
                        return Err(AcquireError::Failed(reason));
                    }
                    if let Err(reason) = self.shared.start(&mut inner) {
                        return Err(AcquireError::Failed(reason));
                    }
                    started = Some(inner.start_id);
                }
            }
        }
    }

    /// Changes 自動解放までの時間; `None` never releases. Takes effect on the running engine too.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the settings screen (TASK-21) is the first caller"
        )
    )]
    pub fn set_idle_release(&self, after: Option<Duration>) {
        let mut inner = self.shared.lock();
        inner.idle_release = after;
        self.shared.changed.notify_all();
    }

    /// Ends the engine for good: on app exit.
    pub fn shutdown(&self) {
        let mut inner = self.shared.lock();
        inner.shutting_down = true;
        self.shared.stop(&mut inner, None);
        self.shared.changed.notify_all();
    }
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        // A panic while the lock was held leaves the fields consistent (each update is a single
        // assignment), so the engine keeps working rather than taking the app down with it.
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn wait<'a>(&self, guard: MutexGuard<'a, Inner>) -> MutexGuard<'a, Inner> {
        self.changed
            .wait(guard)
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn set_state(&self, inner: &mut Inner, state: EngineState) {
        if inner.state != state {
            inner.state = state;
            (self.notify)(&inner.status());
            self.changed.notify_all();
        }
    }

    fn start(self: &Arc<Self>, inner: &mut Inner) -> Result<(), String> {
        inner.start_id += 1;
        if let Err(reason) = self.spawn_process(inner, 1) {
            inner.error = Some(reason.clone());
            (self.notify)(&inner.status());
            self.changed.notify_all();
            return Err(reason);
        }
        inner.error = None;
        inner.idle_since = None;
        self.set_state(inner, EngineState::Loading);
        Ok(())
    }

    /// Spawns attempt `attempt` of the current start and the thread that waits for it to load.
    fn spawn_process(self: &Arc<Self>, inner: &mut Inner, attempt: u32) -> Result<(), String> {
        let spawned = free_port().and_then(|port| (self.spawn)(port).map(|child| (child, port)));
        let (child, port) = spawned.map_err(|err| format!("could not start the engine: {err}"))?;
        if let Some(pid_file) = &self.pid_file {
            let _ = fs::write(pid_file, child.id().to_string());
        }
        inner.process = Some(Running { child, port });
        inner.generation += 1;

        let shared = Arc::clone(self);
        let generation = inner.generation;
        thread::spawn(move || load(&shared, generation, port, attempt));
        Ok(())
    }

    fn discard_process(&self, inner: &mut Inner) {
        if let Some(mut running) = inner.process.take() {
            // SIGKILL rather than a polite SIGTERM: the engine holds nothing that needs flushing,
            // and its memory goes back to the OS either way (TASK-1 AC#3).
            let _ = running.child.kill();
            let _ = running.child.wait();
            if let Some(pid_file) = &self.pid_file {
                let _ = fs::remove_file(pid_file);
            }
        }
    }

    fn stop(&self, inner: &mut Inner, error: Option<String>) {
        self.discard_process(inner);
        inner.idle_since = None;
        if error.is_some() {
            inner.error = error;
        }
        if inner.state == EngineState::Stopped {
            (self.notify)(&inner.status());
            self.changed.notify_all();
        } else {
            self.set_state(inner, EngineState::Stopped);
        }
    }
}

impl Inner {
    fn status(&self) -> Status {
        Status {
            state: self.state,
            error: self.error.clone(),
        }
    }

    fn loading(&self, generation: u64) -> bool {
        self.generation == generation && self.state == EngineState::Loading
    }

    /// Whether the process exited on its own; reaps it if so.
    fn exited(&mut self) -> Option<String> {
        let running = self.process.as_mut()?;
        match running.child.try_wait() {
            Ok(Some(status)) => Some(format!("the engine exited ({status})")),
            Ok(None) => None,
            Err(err) => Some(format!("could not check the engine: {err}")),
        }
    }
}

/// Waits for `/health` to answer 200, which llama-server does only once the model is loaded.
fn load(shared: &Arc<Shared>, generation: u64, port: u16, attempt: u32) {
    let deadline = Instant::now() + LOAD_TIMEOUT;
    loop {
        {
            let mut inner = shared.lock();
            if !inner.loading(generation) {
                return;
            }
            if let Some(reason) = inner.exited() {
                let reason = format!("{reason} while loading");
                if attempt < START_ATTEMPTS {
                    shared.discard_process(&mut inner);
                    if let Err(spawn_error) = shared.spawn_process(&mut inner, attempt + 1) {
                        shared.stop(&mut inner, Some(spawn_error));
                    }
                } else {
                    shared.stop(&mut inner, Some(reason));
                }
                return;
            }
        }
        let healthy = health(port);
        let mut inner = shared.lock();
        if !inner.loading(generation) {
            return;
        }
        if healthy {
            if inner.leases == 0 {
                // A start nobody is waiting on (the one at app launch) is released like any other.
                inner.idle_since = Some(Instant::now());
            }
            shared.set_state(&mut inner, EngineState::Ready);
            return;
        }
        if Instant::now() >= deadline {
            let reason = format!(
                "the engine did not finish loading in {} s",
                LOAD_TIMEOUT.as_secs()
            );
            shared.stop(&mut inner, Some(reason));
            return;
        }
        drop(inner);
        thread::sleep(HEALTH_POLL);
    }
}

/// Records that a 待機中 engine exited on its own. One that exits while loading is left to its
/// loader, which may start it again ([`START_ATTEMPTS`]); stopping it here would end the start
/// with the retries unused.
fn stop_if_exited(shared: &Shared, inner: &mut Inner) -> bool {
    if inner.state != EngineState::Ready {
        return false;
    }
    let Some(reason) = inner.exited() else {
        return false;
    };
    shared.stop(inner, Some(reason));
    true
}

/// Ends the engine once 自動解放までの時間 has passed with no lease, and notices one that exited.
fn supervise(shared: &Shared) {
    let mut inner = shared.lock();
    loop {
        if inner.shutting_down {
            return;
        }
        if stop_if_exited(shared, &mut inner) {
            continue;
        }
        let now = Instant::now();
        let release_at = match (
            inner.state,
            inner.leases,
            inner.idle_release,
            inner.idle_since,
        ) {
            (EngineState::Ready, 0, Some(after), Some(since)) => Some(since + after),
            _ => None,
        };
        if release_at.is_some_and(|at| now >= at) {
            shared.stop(&mut inner, None);
            continue;
        }
        let poll = inner.process.is_some().then_some(EXIT_POLL);
        let timeout = match (release_at.map(|at| at - now), poll) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        inner = match timeout {
            Some(timeout) => {
                shared
                    .changed
                    .wait_timeout(inner, timeout)
                    .unwrap_or_else(|p| p.into_inner())
                    .0
            }
            None => shared.wait(inner),
        };
    }
}

/// A free port on 127.0.0.1, chosen by the OS. It is not held until the engine binds it, so the
/// engine can find it taken and exit at once; [`START_ATTEMPTS`] covers that.
fn free_port() -> io::Result<u16> {
    Ok(TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?
        .local_addr()?
        .port())
}

fn health(port: u16) -> bool {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let Ok(mut stream) = TcpStream::connect_timeout(&address, HEALTH_TIMEOUT) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(HEALTH_TIMEOUT));
    let _ = stream.set_write_timeout(Some(HEALTH_TIMEOUT));
    let request =
        format!("GET /health HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }
    let mut head = [0u8; 12];
    stream.read_exact(&mut head).is_ok() && is_ok_status(&head)
}

/// Whether an HTTP response starts with a 200 status line (`HTTP/1.1 200`).
fn is_ok_status(head: &[u8]) -> bool {
    head.starts_with(b"HTTP/1.") && head.get(8..12) == Some(b" 200")
}

/// The GGUF the engine loads (doc-1 §10): the default model if it is in `dir`, otherwise the
/// first GGUF by name. Choosing one is the settings screen's (TASK-21).
pub fn choose_model(dir: &Path) -> io::Result<PathBuf> {
    let mut models: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("gguf"))
        })
        .collect();
    models.sort();
    let default = models
        .iter()
        .find(|path| path.file_name().is_some_and(|name| name == DEFAULT_MODEL));
    default.or(models.first()).cloned().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("no GGUF in {}", dir.display()),
        )
    })
}

/// 既定のモデル (doc-1 §4.4): 2B Q4_K_M, under the name Hugging Face publishes it as.
pub const DEFAULT_MODEL: &str = "plamo-3-translate-2b-Q4_K_M.gguf";

/// llama-server's arguments (doc-1 §4.5).
pub fn server_args(model: &Path, port: u16) -> Vec<std::ffi::OsString> {
    let mut args: Vec<std::ffi::OsString> = vec!["-m".into(), model.into()];
    let port = port.to_string();
    for arg in [
        "-c",
        "8192",
        "--parallel",
        "1",
        "--host",
        "127.0.0.1",
        "--port",
        &port,
        // The prompt cache keeps up to 8 GiB of earlier prompts in RAM by default (R6), and
        // translations rarely share a prefix long enough to gain from it.
        "--cache-ram",
        "0",
        // Only the app talks to the engine, so the browser UI llama-server serves stays off.
        "--no-webui",
    ] {
        args.push(arg.into());
    }
    args
}

/// Ends a llama-server that an abnormal exit of the previous launch left running (doc-1 §5). The
/// PID is ended only if it still belongs to `program`, so a PID the OS has since given to another
/// process — LM Studio's own llama-server included — is left alone.
pub fn kill_leftover(pid_file: &Path, program: &Path) {
    let Ok(text) = fs::read_to_string(pid_file) else {
        return;
    };
    if let Ok(pid) = text.trim().parse::<u32>()
        && process_is(pid, program)
    {
        kill(pid);
    }
    let _ = fs::remove_file(pid_file);
}

#[cfg(unix)]
fn process_is(pid: u32, program: &Path) -> bool {
    // `comm` is the argv[0] the process was started with, which is the absolute path the app
    // spawns the sidecar by.
    windowless("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .is_ok_and(|out| Path::new(String::from_utf8_lossy(&out.stdout).trim()) == program)
}

#[cfg(windows)]
fn process_is(pid: u32, program: &Path) -> bool {
    // tasklist reports only the image name, not the path.
    let Some(name) = program
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
    else {
        return false;
    };
    windowless("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
        .is_ok_and(|out| {
            String::from_utf8_lossy(&out.stdout)
                .to_lowercase()
                .starts_with(&format!("\"{name}\""))
        })
}

#[cfg(unix)]
fn kill(pid: u32) {
    let _ = windowless("kill")
        .args(["-KILL", &pid.to_string()])
        .status();
}

#[cfg(windows)]
fn kill(pid: u32) {
    let _ = windowless("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .status();
}

/// A `Command` that opens no console window on Windows, where a GUI app's child would otherwise get
/// one.
pub fn windowless(program: impl AsRef<std::ffi::OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    /// A stand-in engine: a `sleep` process for the process side, and a thread that answers
    /// `/health` on the chosen port once `ready` is set — or never, while it is clear. `spawns`
    /// counts the ones that came up, not the attempts [`START_ATTEMPTS`] allows.
    struct Fake {
        ready: Arc<AtomicBool>,
        spawns: Arc<AtomicUsize>,
        states: Arc<Mutex<Vec<EngineState>>>,
    }

    impl Fake {
        fn new() -> Fake {
            Fake {
                ready: Arc::new(AtomicBool::new(true)),
                spawns: Arc::new(AtomicUsize::new(0)),
                states: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn engine(&self, pid_file: Option<PathBuf>) -> Engine {
            let ready = Arc::clone(&self.ready);
            let spawns = Arc::clone(&self.spawns);
            let states = Arc::clone(&self.states);
            Engine::new(
                Box::new(move |port| {
                    if !serve_health(port, Arc::clone(&ready)) {
                        return Command::new("false").spawn();
                    }
                    spawns.fetch_add(1, Ordering::SeqCst);
                    Command::new("sleep").arg("60").spawn()
                }),
                Box::new(move |status| states.lock().unwrap().push(status.state)),
                pid_file,
            )
        }

        fn states(&self) -> Vec<EngineState> {
            self.states.lock().unwrap().clone()
        }
    }

    /// Whether the fake could take the port; like llama-server, it exits at once when it cannot.
    fn serve_health(port: u16, ready: Arc<AtomicBool>) -> bool {
        let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, port)) else {
            return false;
        };
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut buf = [0u8; 512];
                let _ = stream.read(&mut buf);
                let line: &[u8] = if ready.load(Ordering::SeqCst) {
                    b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n"
                } else {
                    b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n"
                };
                let _ = stream.write_all(line);
            }
        });
        true
    }

    fn wait_for(engine: &Engine, state: EngineState) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while engine.status().state != state {
            assert!(Instant::now() < deadline, "engine never reached {state:?}");
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("tiny-translator-{}-{name}", std::process::id()))
    }

    #[test]
    fn start_reports_loading_then_ready() {
        let fake = Fake::new();
        let engine = fake.engine(None);
        engine.start();
        wait_for(&engine, EngineState::Ready);
        assert_eq!(fake.states(), [EngineState::Loading, EngineState::Ready]);
        engine.shutdown();
    }

    #[test]
    fn acquire_waits_for_loading_to_finish() {
        let fake = Fake::new();
        fake.ready.store(false, Ordering::SeqCst);
        let engine = fake.engine(None);
        engine.start();
        let ready = Arc::clone(&fake.ready);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(300));
            ready.store(true, Ordering::SeqCst);
        });
        let started = Instant::now();
        let lease = engine.acquire().unwrap();
        assert!(started.elapsed() >= Duration::from_millis(250));
        assert_eq!(engine.status().state, EngineState::Ready);
        assert!(lease.port() > 0);
        assert_eq!(fake.spawns.load(Ordering::SeqCst), 1);
        drop(lease);
        engine.shutdown();
    }

    #[test]
    fn idle_release_stops_the_engine_and_acquire_starts_it_again() {
        let fake = Fake::new();
        let engine = fake.engine(None);
        engine.set_idle_release(Some(Duration::from_millis(200)));
        drop(engine.acquire().unwrap());
        wait_for(&engine, EngineState::Stopped);

        let lease = engine.acquire().unwrap();
        assert_eq!(fake.spawns.load(Ordering::SeqCst), 2);
        drop(lease);
        assert_eq!(
            fake.states(),
            [
                EngineState::Loading,
                EngineState::Ready,
                EngineState::Stopped,
                EngineState::Loading,
                EngineState::Ready,
            ]
        );
        engine.shutdown();
    }

    #[test]
    fn a_held_lease_keeps_the_engine_up() {
        let fake = Fake::new();
        let engine = fake.engine(None);
        engine.set_idle_release(Some(Duration::from_millis(100)));
        let lease = engine.acquire().unwrap();
        thread::sleep(Duration::from_millis(400));
        assert_eq!(engine.status().state, EngineState::Ready);
        drop(lease);
        wait_for(&engine, EngineState::Stopped);
        engine.shutdown();
    }

    #[test]
    fn no_idle_release_keeps_the_engine_up() {
        let fake = Fake::new();
        let engine = fake.engine(None);
        engine.set_idle_release(Some(Duration::from_millis(100)));
        engine.set_idle_release(None);
        drop(engine.acquire().unwrap());
        thread::sleep(Duration::from_millis(400));
        assert_eq!(engine.status().state, EngineState::Ready);
        engine.shutdown();
    }

    #[test]
    fn an_early_exit_while_loading_is_retried() {
        let ports = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&ports);
        let states = Arc::new(Mutex::new(Vec::new()));
        let heard = Arc::clone(&states);
        let ready = Arc::new(AtomicBool::new(true));
        let engine = Engine::new(
            Box::new(move |port| {
                let mut seen = seen.lock().unwrap();
                seen.push(port);
                if seen.len() == 1 || !serve_health(port, Arc::clone(&ready)) {
                    return Command::new("false").spawn();
                }
                Command::new("sleep").arg("60").spawn()
            }),
            Box::new(move |status: &Status| heard.lock().unwrap().push(status.state)),
            None,
        );
        let lease = engine.acquire().unwrap();
        let ports = ports.lock().unwrap().clone();
        assert!(ports.len() >= 2, "{ports:?}");
        assert_eq!(lease.port(), *ports.last().unwrap());
        // The retries stay inside 読み込み中: the screen never sees the engine stop in between.
        assert_eq!(
            *states.lock().unwrap(),
            [EngineState::Loading, EngineState::Ready]
        );
        drop(lease);
        engine.shutdown();
    }

    #[test]
    fn the_supervisor_leaves_an_exit_while_loading_to_the_loader() {
        let ready = Arc::new(AtomicBool::new(false));
        let engine = Engine::new(
            Box::new(move |port| {
                serve_health(port, Arc::clone(&ready));
                Command::new("false").spawn()
            }),
            Box::new(|_| {}),
            None,
        );
        engine.start();
        // Holding the lock keeps the loader from seeing the exit first, which is the order that
        // used to end the start without its retries.
        let mut inner = engine.shared.lock();
        assert_eq!(inner.state, EngineState::Loading);
        thread::sleep(Duration::from_millis(200));
        assert!(!stop_if_exited(&engine.shared, &mut inner));
        assert_eq!(inner.state, EngineState::Loading);
        assert!(inner.process.is_some());
        drop(inner);
        engine.shutdown();
    }

    #[test]
    fn a_ready_engine_that_exits_is_reported_as_stopped() {
        let fake = Fake::new();
        let engine = fake.engine(None);
        engine.start();
        wait_for(&engine, EngineState::Ready);
        let pid = engine.shared.lock().process.as_ref().unwrap().child.id();
        Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .status()
            .unwrap();
        wait_for(&engine, EngineState::Stopped);
        let error = engine.status().error.unwrap();
        assert!(error.contains("exited"), "{error}");
        engine.shutdown();
    }

    #[test]
    fn a_failed_spawn_is_reported() {
        let engine = Engine::new(
            Box::new(|_| {
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "no GGUF in /models",
                ))
            }),
            Box::new(|_| {}),
            None,
        );
        let err = engine.acquire().err().unwrap();
        assert_eq!(
            err,
            AcquireError::Failed("could not start the engine: no GGUF in /models".into())
        );
        assert_eq!(engine.status().state, EngineState::Stopped);
        assert_eq!(
            engine.status().error.as_deref(),
            Some("could not start the engine: no GGUF in /models")
        );
    }

    #[test]
    fn an_engine_that_exits_while_loading_is_reported() {
        let engine = Engine::new(
            Box::new(|_| Command::new("false").spawn()),
            Box::new(|_| {}),
            None,
        );
        let err = engine.acquire().err().unwrap();
        let AcquireError::Failed(reason) = err else {
            panic!("{err:?}")
        };
        assert!(reason.contains("exited"), "{reason}");
        assert!(reason.contains("while loading"), "{reason}");
    }

    #[test]
    fn shutdown_ends_the_process_and_removes_the_pid_file() {
        let pid_file = temp_path("shutdown.pid");
        let fake = Fake::new();
        let engine = fake.engine(Some(pid_file.clone()));
        engine.start();
        wait_for(&engine, EngineState::Ready);
        let pid: u32 = fs::read_to_string(&pid_file).unwrap().parse().unwrap();
        assert!(alive(pid));
        engine.shutdown();
        assert!(!alive(pid));
        assert!(!pid_file.exists());
        assert_eq!(engine.acquire().err(), Some(AcquireError::ShuttingDown));
    }

    #[test]
    fn kill_leftover_ends_only_the_named_program() {
        let sleep = Path::new("/bin/sleep");
        let pid_file = temp_path("leftover.pid");

        let mut other = Command::new(sleep).arg("60").spawn().unwrap();
        fs::write(&pid_file, other.id().to_string()).unwrap();
        kill_leftover(&pid_file, Path::new("/opt/elsewhere/llama-server"));
        assert!(
            other.try_wait().unwrap().is_none(),
            "a process of another program was ended"
        );
        assert!(!pid_file.exists());
        other.kill().unwrap();
        other.wait().unwrap();

        let mut leftover = Command::new(sleep).arg("60").spawn().unwrap();
        fs::write(&pid_file, leftover.id().to_string()).unwrap();
        kill_leftover(&pid_file, sleep);
        assert!(
            leftover.wait().unwrap().code().is_none(),
            "the leftover was not killed"
        );
        assert!(!pid_file.exists());
    }

    fn alive(pid: u32) -> bool {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success()
    }

    #[test]
    fn status_line_must_be_200() {
        assert!(is_ok_status(b"HTTP/1.1 200 OK"));
        assert!(!is_ok_status(b"HTTP/1.1 503 Service"));
        assert!(!is_ok_status(b"garbage"));
    }

    #[test]
    fn choose_model_prefers_the_default_then_name_order() {
        let dir = temp_path("models");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        assert!(choose_model(&dir).is_err());
        for name in [
            "notes.txt",
            "plamo-3-translate-8b-Q4_K_M.gguf",
            "plamo-3-translate-0.8b-Q4_K_M.GGUF",
        ] {
            fs::write(dir.join(name), "").unwrap();
        }
        assert_eq!(
            choose_model(&dir).unwrap(),
            dir.join("plamo-3-translate-0.8b-Q4_K_M.GGUF")
        );
        fs::write(dir.join(DEFAULT_MODEL), "").unwrap();
        assert_eq!(choose_model(&dir).unwrap(), dir.join(DEFAULT_MODEL));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn server_args_bind_localhost_with_one_slot() {
        let args = server_args(Path::new("/m/model.gguf"), 4321);
        let args: Vec<_> = args.iter().map(|a| a.to_str().unwrap()).collect();
        assert_eq!(
            args,
            [
                "-m",
                "/m/model.gguf",
                "-c",
                "8192",
                "--parallel",
                "1",
                "--host",
                "127.0.0.1",
                "--port",
                "4321",
                "--cache-ram",
                "0",
                "--no-webui",
            ]
        );
    }
}
