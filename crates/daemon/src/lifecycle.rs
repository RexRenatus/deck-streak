//! The lifecycle every long-running role of `deckstreakd` shares (SPEC-025 R8; ADR-010): it tells
//! systemd the role is ready, keeps the watchdog fed, says when the role stops, and turns SIGTERM
//! (or SIGINT at a terminal) into the role's shutdown.
//!
//! It ports the predecessor's `watchdog.py:SdNotifier`, `watchdog.py:watchdog_interval_s`,
//! `watchdog.py:create_heartbeat` and `watchdog.py:run_sd_watchdog` at `27ee2bc`: a hand-written
//! `sd_notify(3)` datagram client that never blocks and never fails the role, `READY=1` once the
//! role serves, then `WATCHDOG=1` at once and every `WatchdogSec=` over [`HEARTBEAT_DIVISOR`],
//! never armed below [`MIN_WATCHDOG`]. Outside systemd, with no `NOTIFY_SOCKET`, every message is a
//! no-op.
//! The heartbeat runs on the role's own runtime and is deliberately coupled to nothing else: a
//! wedged runtime stops it, and systemd restarts the role; a stale sync is the dead-man watch's
//! page, not a restart.
//!
//! Every variable here is systemd's, read from the environment `main` handed in.

use std::io;
use std::os::unix::net::UnixDatagram;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use deck_streak_kernel::{Environment, Setting};
use tokio::signal::unix::{Signal, SignalKind, signal};
use tokio::task::JoinHandle;

/// The datagram socket systemd names for `sd_notify(3)` messages (`Type=notify`).
pub const NOTIFY_SOCKET: &str = "NOTIFY_SOCKET";
/// The watchdog timeout systemd armed (`WatchdogSec=`), in microseconds.
pub const WATCHDOG_USEC: &str = "WATCHDOG_USEC";
/// The process the watchdog timeout is addressed to; unset, it is this one.
pub const WATCHDOG_PID: &str = "WATCHDOG_PID";

/// The shortest watchdog timeout the heartbeat is armed for: below it, a ping every third of the
/// timeout cannot reliably beat the deadline. The predecessor's `watchdog.py:_MIN_WATCHDOG_SEC`,
/// proved by `goldens/watchdog.constants.json`.
pub const MIN_WATCHDOG: Duration = Duration::from_secs(5);
/// The watchdog timeout over this is the heartbeat's interval, so systemd kills the role only when
/// its runtime missed at least two pings in a row, never for one slow operation. The predecessor's
/// `watchdog.py:_HEARTBEAT_DIVISOR`, proved by `goldens/watchdog.constants.json`.
pub const HEARTBEAT_DIVISOR: u32 = 3;

/// A state `sd_notify(3)` reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotifyState {
    /// The role serves: systemd's start-up is complete, and its watchdog is armed from now.
    Ready,
    /// The role's runtime still runs.
    Watchdog,
    /// The role is stopping.
    Stopping,
}

impl NotifyState {
    /// The datagram's text.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::Ready => "READY=1",
            Self::Watchdog => "WATCHDOG=1",
            Self::Stopping => "STOPPING=1",
        }
    }
}

/// Where `sd_notify(3)` messages go: the two forms the predecessor spoke.
#[derive(Debug)]
enum NotifyAddress {
    /// A socket in the file system, `/run/systemd/notify` under a system manager.
    Path(PathBuf),
    /// A socket in Linux's abstract namespace, given as `@name`.
    Abstract(Vec<u8>),
}

impl Setting for NotifyAddress {
    const SHAPE: &'static str = "an absolute socket path, or @ and an abstract socket name";

    fn parse(text: &str) -> Option<Self> {
        if let Some(name) = text.strip_prefix('@') {
            Some(Self::Abstract(name.as_bytes().to_vec()))
        } else if text.starts_with('/') {
            Some(Self::Path(PathBuf::from(text)))
        } else {
            None
        }
    }
}

/// The `sd_notify(3)` client. It sends each message as one datagram, never blocks, and never fails
/// the role: a failed send is logged once per episode of failures, and the role carries on.
#[derive(Clone, Debug)]
pub struct Notifier {
    address: Option<Arc<NotifyAddress>>,
    warned: Arc<AtomicBool>,
}

impl Notifier {
    /// The client for the socket [`NOTIFY_SOCKET`] names; with none, a client whose every message
    /// is a no-op. A socket of a form this client does not speak disables it, with one WARN.
    #[must_use]
    pub fn from_env(env: &Environment) -> Self {
        let address = env
            .optional::<NotifyAddress>(NOTIFY_SOCKET)
            .unwrap_or_else(|_| {
                tracing::warn!("the notify socket is of a form this service does not speak");
                None
            });
        Self {
            address: address.map(Arc::new),
            warned: Arc::default(),
        }
    }

    /// Whether messages reach a socket at all.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.address.is_some()
    }

    /// Sends `state`, and says whether it was sent.
    #[allow(
        clippy::must_use_candidate,
        reason = "sending is the effect; whether it was sent is only a report"
    )]
    pub fn notify(&self, state: NotifyState) -> bool {
        let Some(address) = &self.address else {
            return false;
        };
        match send(address, state.message().as_bytes()) {
            Ok(()) => {
                self.warned.store(false, Ordering::Relaxed);
                true
            }
            Err(error) => {
                if !self.warned.swap(true, Ordering::Relaxed) {
                    let state = state.message();
                    tracing::warn!(%error, state, "an sd_notify datagram was not sent");
                }
                false
            }
        }
    }
}

fn send(address: &NotifyAddress, message: &[u8]) -> io::Result<()> {
    let socket = UnixDatagram::unbound()?;
    // A full queue answers WouldBlock at once: a notification never blocks the role.
    socket.set_nonblocking(true)?;
    match address {
        NotifyAddress::Path(path) => socket.send_to(message, path).map(|_sent| ()),
        NotifyAddress::Abstract(name) => send_abstract(&socket, name, message),
    }
}

#[cfg(target_os = "linux")]
fn send_abstract(socket: &UnixDatagram, name: &[u8], message: &[u8]) -> io::Result<()> {
    use std::os::linux::net::SocketAddrExt;
    let address = std::os::unix::net::SocketAddr::from_abstract_name(name)?;
    socket.send_to_addr(message, &address).map(|_sent| ())
}

#[cfg(not(target_os = "linux"))]
fn send_abstract(_socket: &UnixDatagram, _name: &[u8], _message: &[u8]) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "an abstract socket name is Linux's",
    ))
}

/// A whole number of microseconds, as `WATCHDOG_USEC` holds one.
struct Micros(u64);

impl Setting for Micros {
    const SHAPE: &'static str = "a whole number of microseconds";

    fn parse(text: &str) -> Option<Self> {
        text.parse().ok().map(Self)
    }
}

/// A variable's text, for a comparison the predecessor made on the text itself.
struct Text(String);

impl Setting for Text {
    const SHAPE: &'static str = "text";

    fn parse(text: &str) -> Option<Self> {
        Some(Self(text.to_owned()))
    }
}

/// The watchdog timeout systemd armed for the process `pid`: [`WATCHDOG_USEC`], unless it is unset,
/// not a positive whole number of microseconds, or [`WATCHDOG_PID`] names another process
/// (`sd_watchdog_enabled(3)`; the predecessor's `watchdog.py:watchdog_interval_s`).
#[must_use]
pub fn watchdog_timeout(env: &Environment, pid: u32) -> Option<Duration> {
    if let Ok(Some(Text(addressed))) = env.optional::<Text>(WATCHDOG_PID)
        && addressed != pid.to_string()
    {
        return None;
    }
    let Micros(micros) = env.optional::<Micros>(WATCHDOG_USEC).ok().flatten()?;
    (micros > 0).then(|| Duration::from_micros(micros))
}

/// How often the heartbeat pings for a watchdog `timeout`: the timeout over [`HEARTBEAT_DIVISOR`];
/// `None` below [`MIN_WATCHDOG`], where no heartbeat is armed, as the predecessor refuses
/// (`watchdog.py:create_heartbeat`). The predecessor's one-second floor on the interval can never
/// bind above that minimum (five seconds over three), so it is not written out.
#[must_use]
pub fn watchdog_interval(timeout: Duration) -> Option<Duration> {
    (timeout >= MIN_WATCHDOG).then(|| timeout / HEARTBEAT_DIVISOR)
}

/// Starts the heartbeat, when systemd armed a watchdog for this process: `WATCHDOG=1` at once, then
/// every [`watchdog_interval`]. Returns `None` when no watchdog is armed, and when the timeout is
/// below [`MIN_WATCHDOG`], which it logs once as a unit to fix.
#[must_use]
pub fn spawn_heartbeat(notifier: Notifier, env: &Environment) -> Option<JoinHandle<()>> {
    let timeout = watchdog_timeout(env, std::process::id())?;
    let Some(interval) = watchdog_interval(timeout) else {
        tracing::warn!(
            watchdog_ms = millis(timeout),
            minimum_ms = millis(MIN_WATCHDOG),
            "the watchdog timeout is below the safe minimum, so no heartbeat is armed: raise the \
             unit's WatchdogSec="
        );
        return None;
    };
    Some(tokio::spawn(async move {
        loop {
            notifier.notify(NotifyState::Watchdog);
            tokio::time::sleep(interval).await;
        }
    }))
}

fn millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// The shutdown signal: SIGTERM, with which systemd stops a unit (`KillSignal=`'s default), or
/// SIGINT at a terminal.
///
/// Its handlers are installed by [`ShutdownSignal::install`], before the role tells systemd it is
/// ready: once a listener is registered, tokio replaces the signal's default action for the rest of
/// the process and holds a signal that arrives before the listener is awaited, so a SIGTERM the
/// moment after `READY=1` drains the role rather than killing it.
#[derive(Debug)]
pub struct ShutdownSignal {
    terminate: Signal,
    interrupt: Signal,
}

impl ShutdownSignal {
    /// Installs the SIGTERM and SIGINT handlers, now.
    ///
    /// # Errors
    ///
    /// The operating system's reason when a handler cannot be installed.
    pub fn install() -> io::Result<Self> {
        Ok(Self {
            terminate: signal(SignalKind::terminate())?,
            interrupt: signal(SignalKind::interrupt())?,
        })
    }

    /// Resolves when either signal arrives.
    pub async fn received(mut self) {
        tokio::select! {
            _ = self.terminate.recv() => {}
            _ = self.interrupt.recv() => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Micros, NotifyAddress, Setting, Text};

    #[test]
    fn each_private_setting_states_its_shape() {
        assert_eq!(
            NotifyAddress::SHAPE,
            "an absolute socket path, or @ and an abstract socket name"
        );
        assert_eq!(Micros::SHAPE, "a whole number of microseconds");
        assert_eq!(Text::SHAPE, "text");
    }
}
