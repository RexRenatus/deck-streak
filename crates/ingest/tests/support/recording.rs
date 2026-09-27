//! The recording layer the no-upload census runs through (SPEC-022 R14, A15).
//!
//! It stands between the syncer and the engine's sync server: every request is kept, its body
//! decompressed (the engine compresses each one with zstd) and read as JSON, then relayed to the
//! server unchanged; the server's answer is relayed back. Each connection carries one request, and
//! the answer tells the client to close it, so a request is never split across a reused
//! connection. It runs on its own thread and runtime, so it answers whatever the test's runtime is
//! doing.

use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

/// One request the layer relayed.
#[derive(Clone, Debug)]
pub struct Recorded {
    /// The sync method, the last segment of the request's path (`meta`, `applyChunk`, `upload`).
    pub method: String,
    /// The body, decompressed and read as JSON; `null` when it had none.
    pub body: Value,
}

/// The recording layer in front of one sync server.
pub struct Recording {
    endpoint: String,
    requests: Arc<Mutex<Vec<Recorded>>>,
    stop: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Recording {
    /// Starts the layer in front of the server at `upstream` (`http://127.0.0.1:<port>/`).
    ///
    /// # Panics
    ///
    /// When the upstream is not a loopback `http:` endpoint, or no port can be bound.
    #[must_use]
    pub fn start(upstream: &str) -> Self {
        let upstream: SocketAddr = upstream
            .strip_prefix("http://")
            .and_then(|rest| rest.strip_suffix('/'))
            .and_then(|address| address.parse().ok())
            .unwrap_or_else(|| panic!("{upstream} is not http://<loopback address>/"));
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        listener
            .set_nonblocking(true)
            .expect("a non-blocking listener");
        let endpoint = format!("http://{}/", listener.local_addr().expect("its address"));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let (stop, stopped) = oneshot::channel();
        let kept = Arc::clone(&requests);
        let thread = thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("the recording layer's runtime");
            runtime.block_on(serve(listener, upstream, kept, stopped));
        });
        Self {
            endpoint,
            requests,
            stop: Some(stop),
            thread: Some(thread),
        }
    }

    /// The layer's endpoint, which the syncer is pointed at.
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Every request relayed so far, in order.
    #[must_use]
    pub fn requests(&self) -> Vec<Recorded> {
        self.requests.lock().expect("the recorded requests").clone()
    }

    /// Forgets the requests relayed so far, so a scenario reads only its own.
    pub fn clear(&self) {
        self.requests.lock().expect("the recorded requests").clear();
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

async fn serve(
    listener: std::net::TcpListener,
    upstream: SocketAddr,
    requests: Arc<Mutex<Vec<Recorded>>>,
    mut stopped: oneshot::Receiver<()>,
) {
    let listener = TcpListener::from_std(listener).expect("a tokio listener");
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                if let Ok((client, _)) = accepted {
                    let requests = Arc::clone(&requests);
                    tokio::spawn(async move {
                        let _ = relay(client, upstream, &requests).await;
                    });
                }
            }
            _ = &mut stopped => return,
        }
    }
}

/// Relays one request from `client` to `upstream`, keeping it, and relays the answer back.
async fn relay(
    client: TcpStream,
    upstream: SocketAddr,
    requests: &Mutex<Vec<Recorded>>,
) -> io::Result<()> {
    let mut client = BufReader::new(client);
    let head = read_head(&mut client).await?;
    let body = read_body(&mut client, &head).await?;
    let request_line = head.lines().next().unwrap_or_default().to_owned();
    let path = request_line.split_whitespace().nth(1).unwrap_or_default();
    requests
        .lock()
        .expect("the recorded requests")
        .push(Recorded {
            method: path.rsplit('/').next().unwrap_or_default().to_owned(),
            body: decoded(&body),
        });

    let mut server = TcpStream::connect(upstream).await?;
    let mut forwarded = format!("{request_line}\r\n");
    for line in head.lines().skip(1).filter(|line| !line.is_empty()) {
        let name = line
            .split(':')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if !matches!(
            name.as_str(),
            "transfer-encoding" | "content-length" | "connection"
        ) {
            forwarded.push_str(line);
            forwarded.push_str("\r\n");
        }
    }
    forwarded.push_str(&format!(
        "content-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    ));
    server.write_all(forwarded.as_bytes()).await?;
    server.write_all(&body).await?;
    let mut answer = Vec::new();
    server.read_to_end(&mut answer).await?;

    let client = client.get_mut();
    client.write_all(&closing(answer)).await?;
    client.shutdown().await
}

/// The request's head, up to and including the blank line.
async fn read_head(reader: &mut BufReader<TcpStream>) -> io::Result<String> {
    let mut head = String::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await? == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        let blank = line == "\r\n";
        head.push_str(line.trim_end_matches(['\r', '\n']));
        head.push('\n');
        if blank {
            return Ok(head);
        }
    }
}

fn header<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    head.lines().skip(1).find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.trim().eq_ignore_ascii_case(name).then(|| value.trim())
    })
}

/// The request's body: its declared length, or its chunks joined.
async fn read_body(reader: &mut BufReader<TcpStream>, head: &str) -> io::Result<Vec<u8>> {
    let mut body = Vec::new();
    if header(head, "transfer-encoding").is_some_and(|value| value.contains("chunked")) {
        loop {
            let mut size = String::new();
            reader.read_line(&mut size).await?;
            let size = usize::from_str_radix(size.trim().split(';').next().unwrap_or("0"), 16)
                .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;
            if size == 0 {
                // Trailers, if any, end with a blank line.
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).await? == 0 || line == "\r\n" {
                        return Ok(body);
                    }
                }
            }
            let start = body.len();
            body.resize(start + size, 0);
            reader.read_exact(&mut body[start..]).await?;
            let mut crlf = [0; 2];
            reader.read_exact(&mut crlf).await?;
        }
    }
    if let Some(length) = header(head, "content-length").and_then(|value| value.parse().ok()) {
        body.resize(length, 0);
        reader.read_exact(&mut body).await?;
    }
    Ok(body)
}

/// A body as the server reads it: zstd-decompressed JSON, or `null` when there is none.
fn decoded(body: &[u8]) -> Value {
    if body.is_empty() {
        return Value::Null;
    }
    let json = zstd::decode_all(body).expect("the engine compresses every request body with zstd");
    serde_json::from_slice(&json).expect("every request body is JSON")
}

/// The server's answer with `connection: close` in its head, so the client never reuses the
/// connection this layer is about to close.
fn closing(answer: Vec<u8>) -> Vec<u8> {
    let Some(end) = answer.windows(4).position(|window| window == b"\r\n\r\n") else {
        return answer;
    };
    let head = String::from_utf8_lossy(&answer[..end]);
    if head
        .lines()
        .any(|line| line.to_ascii_lowercase().starts_with("connection:"))
    {
        return answer;
    }
    let mut closed = answer[..end].to_vec();
    closed.extend_from_slice(b"\r\nconnection: close");
    closed.extend_from_slice(&answer[end..]);
    closed
}
