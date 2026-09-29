//! The runner port and the process runner (SPEC-043 R1, R5, R6, R12).
//!
//! The runner is a shell script (`agent/run-headless.sh`): this context launches it as a
//! subprocess with the prompt on a file, the caps in its environment and nothing else added. The
//! device key never passes through this code: the script reads it from the unit's credential.

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde_json::Value;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use crate::duty::DutyCaps;
use crate::verdict::{Cause, Telemetry};

/// How much longer than the run's wall clock this side waits before it stops the script itself.
pub const WALL_CLOCK_GRACE: Duration = Duration::from_secs(45);
/// Numbers each prompt file, so two runs in one process never share one.
static RUN_SEQUENCE: AtomicU64 = AtomicU64::new(0);
/// The result subtype the CLI reports for a run that reached its turn cap.
pub const SUBTYPE_TURN_CAP: &str = "error_max_turns";
/// The result subtype the CLI reports for a run that reached its budget cap.
pub const SUBTYPE_BUDGET_CAP: &str = "error_max_budget_usd";
/// What the script's wall-clock refusal says.
pub const WALL_CLOCK_MARK: &str = "wall clock";

/// A future a runner returns.
pub type RunFuture<'a> = Pin<Box<dyn Future<Output = Result<RunReply, Cause>> + Send + 'a>>;

/// What a successful run returned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunReply {
    /// The model's reply text.
    pub result: String,
    /// What the CLI measured.
    pub telemetry: Telemetry,
}

/// Something that runs one prompt under a duty's caps.
pub trait Runner: Send + Sync {
    /// Runs `prompt` under `caps`.
    ///
    /// # Errors
    ///
    /// The closed [`Cause`] the run failed for.
    fn run<'a>(&'a self, prompt: &'a str, caps: &'a DutyCaps) -> RunFuture<'a>;
}

/// The runner that launches `agent/run-headless.sh`.
#[derive(Clone, Debug)]
pub struct ProcessRunner {
    script: PathBuf,
    work_dir: PathBuf,
    grace: Duration,
}

impl ProcessRunner {
    /// A runner for `script`, writing each prompt under `work_dir` (a directory this service owns).
    #[must_use]
    pub const fn new(script: PathBuf, work_dir: PathBuf) -> Self {
        Self {
            script,
            work_dir,
            grace: WALL_CLOCK_GRACE,
        }
    }

    /// The same runner, waiting `grace` past a run's wall clock before it stops the script itself.
    #[must_use]
    pub const fn with_grace(mut self, grace: Duration) -> Self {
        self.grace = grace;
        self
    }
}

impl Runner for ProcessRunner {
    fn run<'a>(&'a self, prompt: &'a str, caps: &'a DutyCaps) -> RunFuture<'a> {
        Box::pin(async move {
            let file = self.work_dir.join(format!(
                "prompt-{}-{}.md",
                std::process::id(),
                RUN_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            write_private(&file, prompt).map_err(|_| Cause::RunFailed)?;
            let outcome = self.launch(&file, caps).await;
            let _ = std::fs::remove_file(&file);
            outcome
        })
    }
}

impl ProcessRunner {
    async fn launch(&self, file: &std::path::Path, caps: &DutyCaps) -> Result<RunReply, Cause> {
        let budget = format!(
            "{}.{:06}",
            caps.max_budget_micro_usd / 1_000_000,
            caps.max_budget_micro_usd % 1_000_000
        );
        let mut child = Command::new("bash")
            .arg(&self.script)
            .arg(file)
            .env("DECKSTREAK_AGENT_MAX_TURNS", caps.max_turns.to_string())
            .env("DECKSTREAK_AGENT_MAX_BUDGET_USD", budget)
            .env(
                "DECKSTREAK_AGENT_WALL_SECONDS",
                caps.wall_clock.as_secs().to_string(),
            )
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| Cause::RunFailed)?;
        let mut stdout = child.stdout.take().ok_or(Cause::RunFailed)?;
        let mut stderr = child.stderr.take().ok_or(Cause::RunFailed)?;
        let started = tokio::time::Instant::now();
        let waited = tokio::time::timeout(caps.wall_clock + self.grace, async {
            let (mut out, mut err) = (String::new(), String::new());
            let (read_out, read_err) = tokio::join!(
                stdout.read_to_string(&mut out),
                stderr.read_to_string(&mut err)
            );
            let status = child.wait().await;
            (status, read_out.and(read_err), out, err)
        })
        .await;
        let Ok((status, read, out, err)) = waited else {
            return Err(Cause::TimeCap);
        };
        let code = status.map_err(|_| Cause::RunFailed)?.code();
        read.map_err(|_| Cause::RunFailed)?;
        let elapsed = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        match code {
            Some(0) => parse_reply(&out, elapsed),
            Some(1) => Err(Cause::KeyMissing),
            Some(2) => Err(Cause::RefusedShape),
            Some(3) => Err(Cause::KeyRejected),
            Some(4) => Err(Cause::CapacityExhausted),
            Some(5) => Err(Cause::ProxyUnreachable),
            Some(6) => Err(failure_cause(&out, &err)),
            _ => Err(Cause::RunFailed),
        }
    }
}

fn write_private(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(text.as_bytes())
}

/// The cause of a runner exit 6: the CLI's result subtype first, then the wall-clock refusal.
fn failure_cause(stdout: &str, stderr: &str) -> Cause {
    let subtype = serde_json::from_str::<Value>(stdout.trim())
        .ok()
        .and_then(|value| {
            value
                .get("subtype")
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
    match subtype.as_deref() {
        Some(SUBTYPE_TURN_CAP) => Cause::TurnCap,
        Some(SUBTYPE_BUDGET_CAP) => Cause::BudgetCap,
        _ if stderr.contains(WALL_CLOCK_MARK) => Cause::TimeCap,
        _ => Cause::RunFailed,
    }
}

fn parse_reply(stdout: &str, duration_ms: u64) -> Result<RunReply, Cause> {
    let value: Value = serde_json::from_str(stdout.trim()).map_err(|_| Cause::RunFailed)?;
    let result = value
        .get("result")
        .and_then(Value::as_str)
        .ok_or(Cause::RunFailed)?
        .to_owned();
    let count = |value: Option<&Value>| value.and_then(Value::as_u64).unwrap_or(0);
    let usage = value.get("usage");
    let cost = value
        .get("total_cost_usd")
        .and_then(Value::as_f64)
        .and_then(|usd| format!("{usd:.6}").replace('.', "").parse::<u64>().ok())
        .unwrap_or(0);
    Ok(RunReply {
        result,
        telemetry: Telemetry {
            turns: u32::try_from(count(value.get("num_turns"))).unwrap_or(u32::MAX),
            input_tokens: count(usage.and_then(|u| u.get("input_tokens"))),
            output_tokens: count(usage.and_then(|u| u.get("output_tokens"))),
            cost_micro_usd: cost,
            duration_ms,
        },
    })
}
