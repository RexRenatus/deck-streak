//! The archive's listing for the snapshot answer (SPEC-377 R14; ADR-388 D16): the list command the
//! deployment names, run with its arguments and no shell, bounded in time and in output.
//!
//! The command runs with an empty environment and the path of the archive's list-only credential
//! as its last argument, so the credential reaches it through the service's credential directory
//! and never through an environment line. It is killed at its time bound, its output is read to a
//! byte bound, and a refusal, an exit other than success, output past the bound or output that is
//! not UTF-8 answers nothing, which the route answers as unknown. Its lines are the names listed.

use std::future::Future;
use std::io::Read;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use deck_streak_api::snapshot_routes::SnapshotLister;
use deck_streak_kernel::{CredentialsDirectory, Environment, Setting, SettingsError};

/// The setting naming the list command: an absolute program and its arguments, split on
/// whitespace. It carries no credential.
pub const ARCHIVE_LIST_COMMAND: &str = "DECKSTREAK_ARCHIVE_LIST_COMMAND";
/// The role name of the archive's list-only credential in the service's credential directory.
pub const ARCHIVE_LIST_CREDENTIAL: &str = "archive-list";
/// The longest the command runs before it is killed: under the stack's request timeout.
pub const TIME_BOUND: Duration = Duration::from_secs(8);
/// The most bytes of the command's output read.
pub const OUTPUT_BOUND: usize = 65_536;
/// How often the running command is looked at.
const POLL: Duration = Duration::from_millis(10);

/// The list command's words: an absolute program and its arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListCommand {
    program: PathBuf,
    arguments: Vec<String>,
}

impl Setting for ListCommand {
    const SHAPE: &'static str = "an absolute program and its arguments";

    fn parse(text: &str) -> Option<Self> {
        let mut words = text.split_whitespace();
        let program = PathBuf::from(words.next()?);
        program.is_absolute().then(|| Self {
            program,
            arguments: words.map(str::to_owned).collect(),
        })
    }
}

/// The list command as the snapshot route's lister.
#[derive(Clone, Debug)]
pub struct CommandLister {
    command: ListCommand,
    credential: PathBuf,
    time: Duration,
    output: usize,
}

impl CommandLister {
    /// The lister running `command` with the credential file `credential` as its last argument,
    /// killed at `time` and read to `output` bytes.
    #[must_use]
    pub const fn bounded(
        command: ListCommand,
        credential: PathBuf,
        time: Duration,
        output: usize,
    ) -> Self {
        Self {
            command,
            credential,
            time,
            output,
        }
    }

    /// The lister the role's settings and credentials configure: none while the list command's
    /// setting or the archive's list-only credential is absent.
    ///
    /// # Errors
    ///
    /// [`SettingsError::Malformed`] when the setting is set and its program is not absolute.
    pub fn configured(
        env: &Environment,
        credentials: &CredentialsDirectory,
    ) -> Result<Option<Self>, SettingsError> {
        let Some(command) = env.optional::<ListCommand>(ARCHIVE_LIST_COMMAND)? else {
            tracing::info!("no list command is set, so the snapshot answers unknown");
            return Ok(None);
        };
        let credential = credentials.path().join(ARCHIVE_LIST_CREDENTIAL);
        if !credential.is_file() {
            tracing::info!("no list credential is held, so the snapshot answers unknown");
            return Ok(None);
        }
        Ok(Some(Self::bounded(
            command,
            credential,
            TIME_BOUND,
            OUTPUT_BOUND,
        )))
    }

    /// The command's lines, run now and waited for on this thread; none on any refusal.
    fn run(&self) -> Option<Vec<String>> {
        let mut child = Command::new(&self.command.program)
            .args(&self.command.arguments)
            .arg(&self.credential)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let stdout = child.stdout.take()?;
        let mut running = Reaped(child);
        // One byte past the bound is read, so output past it is seen rather than cut short.
        let limit = u64::try_from(self.output).map_or(u64::MAX, |bound| bound.saturating_add(1));
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let mut read = Vec::new();
            let answer = stdout.take(limit).read_to_end(&mut read).map(|_| read);
            // A lister that stopped waiting has dropped the receiver; the answer goes nowhere.
            let _ = sender.send(answer);
        });
        let started = Instant::now();
        let mut read: Option<Vec<u8>> = None;
        let status = loop {
            if started.elapsed() >= self.time {
                return None;
            }
            if read.is_none() {
                read = match receiver.try_recv() {
                    Ok(answer) => Some(answer.ok()?),
                    Err(TryRecvError::Empty) => None,
                    Err(TryRecvError::Disconnected) => return None,
                };
            }
            if let Some(read) = &read {
                if read.len() > self.output {
                    return None;
                }
                if let Some(status) = running.0.try_wait().ok()? {
                    break status;
                }
            }
            thread::sleep(POLL);
        };
        if !status.success() {
            return None;
        }
        let text = String::from_utf8(read?).ok()?;
        Some(
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect(),
        )
    }
}

/// A running command, killed and waited for when it is dropped: a command past its bound, or one
/// the lister stopped reading, never outlives the listing.
struct Reaped(Child);

impl Drop for Reaped {
    fn drop(&mut self) {
        // Either may find the command already gone, which is what was wanted.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl SnapshotLister for CommandLister {
    fn list(&self) -> Pin<Box<dyn Future<Output = Option<Vec<String>>> + Send + '_>> {
        let lister = self.clone();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || lister.run())
                .await
                .ok()
                .flatten()
        })
    }
}
