//! The served stack's test support, shared by the server, tools and guard tests (SPEC-119 section
//! 14): a client that speaks HTTP/1.1 to a loopback listener with every wait bounded, a scripted
//! law track that counts its reads and can hold them, and the guard's tokens, built from parts at
//! run time. Test support only.
#![allow(dead_code, clippy::expect_used)]

use std::fs;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use deck_streak_coordination::law::LawBlock;
use deck_streak_kernel::{
    CredentialLoader, CredentialsDirectory, ManualClock, Redactor, UtcMillis,
};
use deck_streak_mcp::server;
use deck_streak_mcp::{Grants, Guard, LawTrackFuture, LawTrackSource};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;

/// How long any one wait of a test may take before the test fails: a request that queues or hangs
/// fails the test rather than holding the suite (section 5 of the amendment's brief, S11920).
pub const BOUND: Duration = Duration::from_secs(5);

/// The core credential's token, built from parts.
pub fn core_token() -> String {
    format!("served-core-{}", "c".repeat(30))
}

/// The law-track credential's token, built from parts.
pub fn law_token() -> String {
    format!("served-law-{}", "l".repeat(30))
}

/// A guard over the core and law-track credentials, its limiter on a clock that never moves.
pub fn guard() -> Arc<Guard> {
    let directory = tempfile::tempdir().expect("a temporary directory");
    for (id, value) in [
        ("mcp-core-token", core_token()),
        ("mcp-law-track-token", law_token()),
    ] {
        fs::write(directory.path().join(id), format!("{value}\n")).expect("a credential");
    }
    let path = CredentialsDirectory::new(directory.path()).expect("an absolute path");
    let grants =
        Grants::load(&CredentialLoader::new(path, Redactor::new())).expect("the grants load");
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(
        1_760_000_000_000,
    )));
    Arc::new(Guard::new(grants, clock))
}

/// A law block with every number present.
pub const fn full_block() -> LawBlock {
    LawBlock {
        streak: 12,
        xp_today: 340,
        total_xp: 98_765,
        level: 7,
        dues: Some(23),
        leech_active: Some(4),
        mastery: Some(0.125),
    }
}

/// A law track that answers one block, counts its reads, and holds each read on a gate when it
/// has one.
pub struct ScriptedLaw {
    block: LawBlock,
    reads: AtomicUsize,
    gate: Option<Arc<Semaphore>>,
}

impl ScriptedLaw {
    /// A law track answering `block` at once.
    pub fn answering(block: LawBlock) -> Arc<Self> {
        Arc::new(Self {
            block,
            reads: AtomicUsize::new(0),
            gate: None,
        })
    }

    /// A law track answering `block` once `gate` gives the read a permit.
    pub fn holding(block: LawBlock, gate: Arc<Semaphore>) -> Arc<Self> {
        Arc::new(Self {
            block,
            reads: AtomicUsize::new(0),
            gate: Some(gate),
        })
    }

    /// How many reads have started.
    pub fn reads(&self) -> usize {
        self.reads.load(Ordering::SeqCst)
    }

    /// Whether `count` reads have started within [`BOUND`].
    pub async fn entered(&self, count: usize) -> bool {
        tokio::time::timeout(BOUND, async {
            while self.reads() < count {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .is_ok()
    }
}

impl LawTrackSource for ScriptedLaw {
    fn law_track(&self) -> LawTrackFuture<'_> {
        Box::pin(async move {
            self.reads.fetch_add(1, Ordering::SeqCst);
            if let Some(gate) = &self.gate {
                let _permit = gate.acquire().await.expect("the gate stays open");
            }
            Ok(self.block)
        })
    }
}

/// The served stack on a loopback listener.
pub struct Served {
    /// The listener's address.
    pub address: SocketAddr,
    /// The guard the stack serves behind.
    pub guard: Arc<Guard>,
}

impl Served {
    /// The production router over `law`, served on a fresh loopback port.
    pub async fn start(law: Arc<ScriptedLaw>) -> Self {
        let guard = guard();
        let router = server::router(law, Arc::clone(&guard));
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))
            .await
            .expect("a loopback port");
        let address = listener.local_addr().expect("the bound address");
        tokio::spawn(async move { axum::serve(listener, router).await });
        Self { address, guard }
    }

    /// The `Host` a local client sends: a loopback name and the port.
    pub fn host(&self) -> String {
        format!("localhost:{}", self.address.port())
    }

    /// A POST of `body` to `path`, with the bearer `token` when there is one.
    pub async fn post(&self, path: &str, token: Option<&str>, body: &[u8]) -> Reply {
        self.post_as(&self.host(), path, token, body).await
    }

    /// A POST of `body` to `path` naming `host`, with the bearer `token` when there is one.
    pub async fn post_as(&self, host: &str, path: &str, token: Option<&str>, body: &[u8]) -> Reply {
        let mut head = format!(
            "POST {path} HTTP/1.1\r\nhost: {host}\r\nconnection: close\r\ncontent-type: \
             application/json\r\naccept: application/json, text/event-stream\r\ncontent-length: \
             {}\r\n",
            body.len()
        );
        if let Some(token) = token {
            head.push_str("authorization: Bearer ");
            head.push_str(token);
            head.push_str("\r\n");
        }
        head.push_str("\r\n");
        let mut request = head.into_bytes();
        request.extend_from_slice(body);
        exchange(self.address, request).await
    }
}

/// Sends `request` and reads the whole answer, writing and reading at once so an answer sent
/// before the body is read is still received; every step bounded by [`BOUND`].
async fn exchange(address: SocketAddr, request: Vec<u8>) -> Reply {
    let stream = tokio::time::timeout(BOUND, TcpStream::connect(address))
        .await
        .expect("the connection opens within the bound")
        .expect("the listener accepts");
    let (mut reader, mut writer) = stream.into_split();
    let writing = tokio::spawn(async move {
        // The server may answer and close before it reads the whole body (a refused one), so a
        // failed write is not the test's failure: the answer read below is.
        let _ = writer.write_all(&request).await;
        let _ = writer.flush().await;
        writer
    });
    let mut raw = Vec::new();
    let read = tokio::time::timeout(BOUND, async {
        let mut chunk = [0_u8; 8192];
        loop {
            match reader.read(&mut chunk).await {
                Ok(0) | Err(_) => break,
                Ok(count) => raw.extend_from_slice(&chunk[..count]),
            }
        }
    })
    .await;
    writing.abort();
    assert!(
        read.is_ok(),
        "no complete answer within {BOUND:?}; read so far: {}",
        String::from_utf8_lossy(&raw)
    );
    Reply::parse(&raw)
}

/// An answer: its status, its headers (names in lower case) and its body, de-chunked.
#[derive(Debug, Clone)]
pub struct Reply {
    /// The status code.
    pub status: u16,
    /// The headers, in order, each name in lower case.
    pub headers: Vec<(String, String)>,
    /// The body.
    pub body: Vec<u8>,
}

impl Reply {
    fn parse(raw: &[u8]) -> Self {
        let split = raw
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap_or_else(|| panic!("no header block in {:?}", String::from_utf8_lossy(raw)));
        let head = String::from_utf8_lossy(&raw[..split]).into_owned();
        let mut lines = head.split("\r\n");
        let status = lines
            .next()
            .and_then(|line| line.split(' ').nth(1))
            .and_then(|code| code.parse().ok())
            .unwrap_or_else(|| panic!("no status line in {head:?}"));
        let headers: Vec<(String, String)> = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
            .collect();
        let rest = &raw[split + 4..];
        let chunked = headers
            .iter()
            .any(|(name, value)| name == "transfer-encoding" && value.contains("chunked"));
        let body = if chunked {
            dechunk(rest)
        } else {
            rest.to_vec()
        };
        Self {
            status,
            headers,
            body,
        }
    }

    /// The first value of the header `name` (lower case).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header == name)
            .map(|(_, value)| value.as_str())
    }

    /// The body as text.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// The body as JSON.
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body)
            .unwrap_or_else(|error| panic!("the body is not JSON ({error}): {}", self.text()))
    }

    /// The JSON-RPC answer's `result`.
    pub fn result(&self) -> Value {
        let answer = self.json();
        assert!(
            answer.get("error").is_none(),
            "a JSON-RPC error, not a result: {answer}"
        );
        answer["result"].clone()
    }
}

/// The body of a chunked transfer.
fn dechunk(mut rest: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    while let Some(end) = rest.windows(2).position(|window| window == b"\r\n") {
        let size_text = String::from_utf8_lossy(&rest[..end]);
        let size = usize::from_str_radix(size_text.split(';').next().unwrap_or("").trim(), 16)
            .unwrap_or(0);
        rest = &rest[end + 2..];
        if size == 0 || rest.len() < size {
            break;
        }
        body.extend_from_slice(&rest[..size]);
        rest = rest.get(size + 2..).unwrap_or_default();
    }
    body
}

/// A JSON-RPC request of `method` with `params`, as bytes.
pub fn rpc(id: u64, method: &str, params: &Value) -> Vec<u8> {
    serde_json::to_vec(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))
        .expect("a request serializes")
}

/// An `initialize` request.
pub fn initialize() -> Vec<u8> {
    rpc(
        1,
        "initialize",
        &json!({
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "deck-streak-tests", "version": "0"},
        }),
    )
}

/// A `tools/list` request.
pub fn list_tools() -> Vec<u8> {
    rpc(2, "tools/list", &json!({}))
}

/// A `tools/call` request of `get_law_track`, with no argument.
pub fn call_law_track() -> Vec<u8> {
    rpc(
        3,
        "tools/call",
        &json!({"name": "get_law_track", "arguments": {}}),
    )
}
