//! Translating one text through the engine's `/completion`, streamed (doc-1 §4.3).
//!
//! The request goes out from here rather than from the screen, so the engine's port and API key
//! never reach the WebView and its CSP keeps talking to the IPC only.
//!
//! HTTP is written by hand over a [`TcpStream`], as the health check in `engine.rs` is: one POST to
//! a server on 127.0.0.1 does not justify an HTTP client crate.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, Shutdown, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

/// The tokens that end a PFML3 message (doc-1 §4.3). The GGUF already treats both as end of
/// generation; naming them in the request guards against a GGUF whose settings differ.
const STOP: [&str; 2] = ["<|plamo:tag|>", "<|plamo:bos|>"];

/// The PFML3 prompt for translating `text` into `target`, with the source language when the app
/// knows it (doc-1 §6.1's 判定なし leaves it to the model). Without the leading `<|plamo:bos|>`,
/// which llama-server adds.
pub fn prompt(text: &str, target: &str, source: Option<&str>) -> String {
    let source = source
        .map(|lang| format!("<|plamo:key|>lang<|plamo:val|>{lang}"))
        .unwrap_or_default();
    format!(
        "<|plamo:tag|>dataset<|plamo:msg|>translation\
         <|plamo:tag|>input{source}<|plamo:msg|>{text}\
         <|plamo:tag|>output<|plamo:key|>lang<|plamo:val|>{target}<|plamo:msg|>"
    )
}

/// The `/completion` request: greedy (temperature 0, as TASK-1 measured), so the same text gives
/// the same translation, and no length limit, so a long translation is not cut off — stopping it
/// is the user's.
fn request_body(prompt: &str) -> String {
    json!({
        "prompt": prompt,
        "stream": true,
        "temperature": 0,
        "stop": STOP,
    })
    .to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The engine finished the translation.
    Done,
    /// [`Translator::stop`] or a newer translation ended it first.
    Stopped,
}

impl Outcome {
    /// The name the screen receives (`src/lib/translate.ts`).
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Done => "done",
            Outcome::Stopped => "stopped",
        }
    }
}

/// Which translation is running, so that it can be stopped. One at a time: the engine has a
/// single slot (`--parallel 1`), and starting a translation stops the one before it.
#[derive(Default)]
pub struct Translator {
    current: Mutex<Option<Arc<Cancel>>>,
}

/// Stops one translation: marks it stopped and closes its connection, which wakes a read blocked on
/// the stream and makes llama-server cancel the generation (it frees the slot at once — TASK-9
/// measured the next request answering within 0.3 s).
#[derive(Default)]
pub struct Cancel {
    stopped: AtomicBool,
    stream: Mutex<Option<TcpStream>>,
}

impl Cancel {
    fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        if let Some(stream) = lock(&self.stream).take() {
            let _ = stream.shutdown(Shutdown::Both);
        }
    }

    fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }

    /// Hands over the connection for [`Cancel::stop`] to close; `false` if the translation was
    /// stopped before it had one, in which case the request is not sent.
    fn attach(&self, stream: &TcpStream) -> io::Result<bool> {
        let mut slot = lock(&self.stream);
        // Checked under the lock that `stop` takes, so a stop is either seen here or finds the
        // stream to close.
        if self.is_stopped() {
            return Ok(false);
        }
        *slot = Some(stream.try_clone()?);
        Ok(true)
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Translator {
    /// Stops the running translation, if any, and makes `cancel` the running one.
    pub fn begin(&self) -> Arc<Cancel> {
        let cancel = Arc::new(Cancel::default());
        if let Some(previous) = lock(&self.current).replace(Arc::clone(&cancel)) {
            previous.stop();
        }
        cancel
    }

    /// Forgets `cancel` once its translation has ended, unless a newer one has taken its place.
    pub fn end(&self, cancel: &Arc<Cancel>) {
        let mut current = lock(&self.current);
        if current.as_ref().is_some_and(|c| Arc::ptr_eq(c, cancel)) {
            *current = None;
        }
    }

    pub fn stop(&self) {
        if let Some(cancel) = lock(&self.current).take() {
            cancel.stop();
        }
    }
}

/// Sends `prompt` to the engine at `port` and hands each piece of the translation to `on_piece` as
/// it is generated.
pub fn stream(
    port: u16,
    api_key: &str,
    prompt: &str,
    cancel: &Cancel,
    on_piece: impl FnMut(&str),
) -> Result<Outcome, String> {
    let result = send(port, api_key, prompt, cancel, on_piece);
    // A stop shows up as a read or write failing on the closed connection; it is not an error.
    if cancel.is_stopped() {
        return Ok(Outcome::Stopped);
    }
    result
}

fn send(
    port: u16,
    api_key: &str,
    prompt: &str,
    cancel: &Cancel,
    on_piece: impl FnMut(&str),
) -> Result<Outcome, String> {
    let failed = |err: io::Error| format!("could not reach the engine: {err}");
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).map_err(failed)?;
    if !cancel.attach(&stream).map_err(failed)? {
        return Ok(Outcome::Stopped);
    }
    let body = request_body(prompt);
    let request = format!(
        "POST /completion HTTP/1.1\r\n\
         Host: 127.0.0.1:{port}\r\n\
         Authorization: Bearer {api_key}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).map_err(failed)?;
    read_response(BufReader::new(stream), on_piece)
}

/// Reads an HTTP response to `/completion`: a 200 carries the translation as server-sent events, and
/// anything else carries llama-server's error.
fn read_response(mut reader: impl BufRead, on_piece: impl FnMut(&str)) -> Result<Outcome, String> {
    let failed = |err: io::Error| format!("lost the engine's answer: {err}");
    let head = read_head(&mut reader).map_err(failed)?;
    let body: Box<dyn Read + '_> = if head.chunked {
        Box::new(Chunked::new(reader))
    } else if let Some(length) = head.content_length {
        Box::new(reader.take(length))
    } else {
        Box::new(reader)
    };
    let mut body = BufReader::new(body);
    if head.status != 200 {
        let mut text = String::new();
        let _ = body.read_to_string(&mut text);
        return Err(format!(
            "the engine refused the translation (HTTP {}): {}",
            head.status,
            error_message(&text)
        ));
    }
    read_events(body, on_piece)
}

struct Head {
    status: u16,
    chunked: bool,
    content_length: Option<u64>,
}

fn read_head(reader: &mut impl BufRead) -> io::Result<Head> {
    let invalid = |what: &str| io::Error::new(io::ErrorKind::InvalidData, what.to_owned());
    let status_line = read_line(reader)?.ok_or_else(|| invalid("no response"))?;
    let status = status_line
        .strip_prefix("HTTP/1.")
        .and_then(|rest| rest.get(2..5))
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| invalid("not an HTTP response"))?;
    let mut head = Head {
        status,
        chunked: false,
        content_length: None,
    };
    loop {
        let line = read_line(reader)?.ok_or_else(|| invalid("the headers were cut off"))?;
        if line.is_empty() {
            return Ok(head);
        }
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if name.eq_ignore_ascii_case("transfer-encoding") {
            head.chunked = value.to_ascii_lowercase().contains("chunked");
        } else if name.eq_ignore_ascii_case("content-length") {
            head.content_length = value.parse().ok();
        }
    }
}

/// One line without its line ending, or `None` at the end of the input.
fn read_line(reader: &mut impl BufRead) -> io::Result<Option<String>> {
    let mut line = Vec::new();
    if reader.read_until(b'\n', &mut line)? == 0 {
        return Ok(None);
    }
    while line.last().is_some_and(|b| *b == b'\n' || *b == b'\r') {
        line.pop();
    }
    String::from_utf8(line)
        .map(Some)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

/// The `data:` events of the stream, each a JSON object whose `content` is the next piece and whose
/// `stop` is true on the last one.
fn read_events(
    mut reader: impl BufRead,
    mut on_piece: impl FnMut(&str),
) -> Result<Outcome, String> {
    loop {
        let line = read_line(&mut reader)
            .map_err(|err| format!("lost the engine's answer: {err}"))?
            .ok_or("the engine ended the answer before the translation was complete")?;
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let event: Value = serde_json::from_str(data.trim())
            .map_err(|err| format!("could not read the engine's answer: {err}"))?;
        if let Some(message) = event.get("error") {
            return Err(format!("the engine failed: {}", error_text(message)));
        }
        if let Some(piece) = event.get("content").and_then(Value::as_str)
            && !piece.is_empty()
        {
            on_piece(piece);
        }
        if event.get("stop").and_then(Value::as_bool) == Some(true) {
            return Ok(Outcome::Done);
        }
    }
}

/// llama-server's `{"error": {"message": …}}`, or the body itself when it is not that.
fn error_message(body: &str) -> String {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| value.get("error").map(error_text))
        .unwrap_or_else(|| body.trim().to_owned())
}

fn error_text(error: &Value) -> String {
    error
        .get("message")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| error.to_string())
}

/// A `Transfer-Encoding: chunked` body, which is how llama-server sends a stream.
struct Chunked<R> {
    inner: R,
    /// Bytes left in the current chunk; `None` once the last chunk has been read.
    left: Option<u64>,
}

impl<R: BufRead> Chunked<R> {
    fn new(inner: R) -> Self {
        Chunked {
            inner,
            left: Some(0),
        }
    }

    /// Reads the next chunk's size line (and, after the last chunk, the trailer).
    fn next_chunk(&mut self) -> io::Result<()> {
        let invalid = |what: &str| io::Error::new(io::ErrorKind::InvalidData, what.to_owned());
        let line = read_line(&mut self.inner)?.ok_or_else(|| invalid("the body was cut off"))?;
        let size = line.split(';').next().unwrap_or_default().trim();
        let size = u64::from_str_radix(size, 16).map_err(|_| invalid("a bad chunk size"))?;
        if size == 0 {
            while read_line(&mut self.inner)?.is_some_and(|line| !line.is_empty()) {}
            self.left = None;
        } else {
            self.left = Some(size);
        }
        Ok(())
    }
}

impl<R: BufRead> Read for Chunked<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.left == Some(0) {
            self.next_chunk()?;
        }
        let Some(left) = self.left else {
            return Ok(0);
        };
        let want = buf.len().min(usize::try_from(left).unwrap_or(usize::MAX));
        let read = self.inner.read(&mut buf[..want])?;
        if read == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        let left = left - read as u64;
        if left == 0 {
            // Each chunk's data ends in a line ending of its own.
            read_line(&mut self.inner)?;
        }
        self.left = Some(left);
        Ok(read)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn prompt_without_a_source_language_leaves_it_to_the_model() {
        assert_eq!(
            prompt("Hello, world.", "ja", None),
            "<|plamo:tag|>dataset<|plamo:msg|>translation\
             <|plamo:tag|>input<|plamo:msg|>Hello, world.\
             <|plamo:tag|>output<|plamo:key|>lang<|plamo:val|>ja<|plamo:msg|>"
        );
    }

    #[test]
    fn prompt_with_a_source_language_names_it_on_the_input() {
        assert_eq!(
            prompt("こんにちは", "en", Some("ja")),
            "<|plamo:tag|>dataset<|plamo:msg|>translation\
             <|plamo:tag|>input<|plamo:key|>lang<|plamo:val|>ja<|plamo:msg|>こんにちは\
             <|plamo:tag|>output<|plamo:key|>lang<|plamo:val|>en<|plamo:msg|>"
        );
    }

    #[test]
    fn prompt_keeps_the_text_as_it_is() {
        let text = "  line one\n\nline \"two\"\t{x}\n";
        let built = prompt(text, "ja", None);
        assert!(built.contains(&format!("<|plamo:msg|>{text}<|plamo:tag|>output")));
    }

    #[test]
    fn request_body_streams_greedily_with_both_stop_tokens() {
        let body: Value = serde_json::from_str(&request_body("p \"q\"\n")).unwrap();
        assert_eq!(
            body,
            json!({
                "prompt": "p \"q\"\n",
                "stream": true,
                "temperature": 0,
                "stop": ["<|plamo:tag|>", "<|plamo:bos|>"],
            })
        );
    }

    /// An SSE stream as llama-server sends it, chunked at arbitrary points.
    fn chunked(body: &str, sizes: &[usize]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut rest = body.as_bytes();
        for size in sizes.iter().copied().chain(std::iter::repeat(7)) {
            if rest.is_empty() {
                break;
            }
            let (chunk, tail) = rest.split_at(size.min(rest.len()));
            out.extend_from_slice(format!("{:x}\r\n", chunk.len()).as_bytes());
            out.extend_from_slice(chunk);
            out.extend_from_slice(b"\r\n");
            rest = tail;
        }
        out.extend_from_slice(b"0\r\n\r\n");
        out
    }

    const EVENTS: &str = "data: {\"content\":\"こんにちは、\",\"stop\":false}\n\n\
                          data: {\"content\":\"世界\",\"stop\":false}\n\n\
                          data: {\"content\":\"\",\"stop\":false}\n\n\
                          data: {\"content\":\"\\u3002\",\"stop\":true,\"stop_type\":\"eos\"}\n\n";

    fn response(status: &str, headers: &str, body: &[u8]) -> Vec<u8> {
        let mut out = format!("HTTP/1.1 {status}\r\n{headers}\r\n").into_bytes();
        out.extend_from_slice(body);
        out
    }

    fn pieces(raw: &[u8]) -> (Result<Outcome, String>, Vec<String>) {
        let mut seen = Vec::new();
        let outcome = read_response(raw, |piece| seen.push(piece.to_owned()));
        (outcome, seen)
    }

    #[test]
    fn a_chunked_stream_is_read_piece_by_piece() {
        // Chunk boundaries that split a multi-byte character and a line ending.
        let raw = response(
            "200 OK",
            "Content-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n",
            &chunked(EVENTS, &[20, 1, 5, 33]),
        );
        let (outcome, seen) = pieces(&raw);
        assert_eq!(outcome, Ok(Outcome::Done));
        assert_eq!(seen, ["こんにちは、", "世界", "。"]);
    }

    #[test]
    fn a_stream_without_chunking_is_read_too() {
        let raw = response("200 OK", "Connection: close\r\n", EVENTS.as_bytes());
        let (outcome, seen) = pieces(&raw);
        assert_eq!(outcome, Ok(Outcome::Done));
        assert_eq!(seen.concat(), "こんにちは、世界。");
    }

    #[test]
    fn a_stream_cut_off_before_the_last_event_is_an_error() {
        let body = "data: {\"content\":\"途中\",\"stop\":false}\n\n";
        let raw = response("200 OK", "Connection: close\r\n", body.as_bytes());
        let (outcome, seen) = pieces(&raw);
        assert!(
            outcome
                .unwrap_err()
                .contains("before the translation was complete")
        );
        assert_eq!(seen, ["途中"]);
    }

    #[test]
    fn a_refusal_reports_llama_servers_message() {
        let body =
            r#"{"error":{"message":"Invalid API Key","type":"authentication_error","code":401}}"#;
        let raw = response(
            "401 Unauthorized",
            &format!("Content-Length: {}\r\n", body.len()),
            body.as_bytes(),
        );
        let (outcome, seen) = pieces(&raw);
        assert_eq!(
            outcome,
            Err("the engine refused the translation (HTTP 401): Invalid API Key".into())
        );
        assert!(seen.is_empty());
    }

    #[test]
    fn an_error_event_in_the_stream_is_reported() {
        let body = "data: {\"error\":{\"message\":\"context is full\"}}\n\n";
        let raw = response("200 OK", "Connection: close\r\n", body.as_bytes());
        assert_eq!(
            pieces(&raw).0,
            Err("the engine failed: context is full".into())
        );
    }

    /// A stand-in engine on a free port: answers one request with `answer` and returns what it
    /// was sent. With `hold`, it then keeps the connection open without sending anything more, as
    /// an engine that is still generating does.
    fn fake_engine(answer: Vec<u8>, hold: bool) -> (u16, thread::JoinHandle<String>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().unwrap();
                }
                request.push_str(&line);
                if line == "\r\n" {
                    break;
                }
            }
            let mut body = vec![0u8; length];
            reader.read_exact(&mut body).unwrap();
            request.push_str(&String::from_utf8(body).unwrap());
            stream.write_all(&answer).unwrap();
            if hold {
                // Until the client closes the connection.
                let _ = reader.read(&mut [0u8; 1]);
            }
            request
        });
        (port, handle)
    }

    #[test]
    fn stream_sends_the_prompt_with_the_api_key() {
        let raw = response(
            "200 OK",
            "Transfer-Encoding: chunked\r\n",
            &chunked(EVENTS, &[]),
        );
        let (port, engine) = fake_engine(raw, false);
        let mut seen = String::new();
        let outcome = stream(port, "k3y", "PROMPT", &Cancel::default(), |piece| {
            seen.push_str(piece)
        });
        assert_eq!(outcome, Ok(Outcome::Done));
        assert_eq!(seen, "こんにちは、世界。");
        let request = engine.join().unwrap();
        assert!(
            request.starts_with("POST /completion HTTP/1.1\r\n"),
            "{request}"
        );
        assert!(
            request.contains("\r\nAuthorization: Bearer k3y\r\n"),
            "{request}"
        );
        let body = request.split("\r\n\r\n").nth(1).unwrap();
        let body: Value = serde_json::from_str(body).unwrap();
        assert_eq!(body["prompt"], "PROMPT");
        assert_eq!(body["stream"], true);
    }

    #[test]
    fn stop_ends_a_translation_that_is_still_generating() {
        let first = "data: {\"content\":\"途中\",\"stop\":false}\n\n";
        let raw = response(
            "200 OK",
            "Transfer-Encoding: chunked\r\n",
            format!("{:x}\r\n{first}\r\n", first.len()).as_bytes(),
        );
        let (port, engine) = fake_engine(raw, true);
        let translator = Arc::new(Translator::default());
        let cancel = translator.begin();
        let stopper = Arc::clone(&translator);
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(200));
            stopper.stop();
        });
        let started = Instant::now();
        let mut seen = Vec::new();
        let outcome = stream(port, "k", "P", &cancel, |piece| seen.push(piece.to_owned()));
        assert_eq!(outcome, Ok(Outcome::Stopped));
        assert_eq!(seen, ["途中"]);
        assert!(started.elapsed() < Duration::from_secs(2));
        // The engine sees the connection close, which is what makes llama-server cancel.
        engine.join().unwrap();
    }

    #[test]
    fn a_translation_stopped_before_it_connects_sends_nothing() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let translator = Translator::default();
        let cancel = translator.begin();
        translator.stop();
        let outcome = stream(port, "k", "P", &cancel, |_| panic!("no piece expected"));
        assert_eq!(outcome, Ok(Outcome::Stopped));
    }

    #[test]
    fn a_new_translation_stops_the_previous_one() {
        let translator = Translator::default();
        let first = translator.begin();
        let second = translator.begin();
        assert!(first.is_stopped());
        assert!(!second.is_stopped());
        // The first one ending late does not forget the second, which stop still reaches.
        translator.end(&first);
        translator.stop();
        assert!(second.is_stopped());
    }
}
