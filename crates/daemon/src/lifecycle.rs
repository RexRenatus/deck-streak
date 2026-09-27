//! The lifecycle every long-running role shares (SPEC-025 R8; ADR-010).
//!
//! STUB for the red-first commit: nothing is sent, no heartbeat is armed, and no signal is heard.

use std::io;
use std::time::Duration;

use deck_streak_kernel::Environment;
use tokio::task::JoinHandle;

/// The datagram socket systemd names for sd_notify(3) messages.
pub const NOTIFY_SOCKET: &str = "NOTIFY_SOCKET";
/// The watchdog timeout systemd armed, in microseconds.
pub const WATCHDOG_USEC: &str = "WATCHDOG_USEC";
/// The process the watchdog timeout is addressed to.
pub const WATCHDOG_PID: &str = "WATCHDOG_PID";

/// The shortest watchdog timeout the heartbeat is armed for: the predecessor's
/// `watchdog.py:_MIN_WATCHDOG_SEC`.
pub const MIN_WATCHDOG: Duration = Duration::from_secs(5);
/// The watchdog timeout over this is the heartbeat's interval: the predecessor's
/// `watchdog.py:_HEARTBEAT_DIVISOR`.
pub const HEARTBEAT_DIVISOR: u32 = 3;

/// A state sd_notify(3) reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotifyState {
    /// The role serves.
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
        ""
    }
}

/// The sd_notify(3) client.
#[derive(Clone, Debug, Default)]
pub struct Notifier {}

impl Notifier {
    /// The client for the socket the environment names.
    #[must_use]
    pub fn from_env(env: &Environment) -> Self {
        let _ = env;
        Self::default()
    }

    /// Sends `state`; whether it was sent.
    #[allow(clippy::must_use_candidate)]
    pub fn notify(&self, state: NotifyState) -> bool {
        let _ = state;
        false
    }
}

/// The watchdog timeout systemd armed for the process `pid`.
#[must_use]
pub fn watchdog_timeout(env: &Environment, pid: u32) -> Option<Duration> {
    let _ = (env, pid);
    None
}

/// The heartbeat's interval for a watchdog timeout.
#[must_use]
pub const fn watchdog_interval(timeout: Duration) -> Option<Duration> {
    let _ = timeout;
    None
}

/// Starts the heartbeat, when systemd armed a watchdog.
#[must_use]
pub fn spawn_heartbeat(notifier: Notifier, env: &Environment) -> Option<JoinHandle<()>> {
    let _ = (notifier, env);
    None
}

/// The shutdown signal.
#[derive(Debug)]
pub struct ShutdownSignal {}

impl ShutdownSignal {
    /// Installs the handlers.
    ///
    /// # Errors
    ///
    /// When a handler cannot be installed.
    pub const fn install() -> io::Result<Self> {
        Ok(Self {})
    }

    /// Resolves when the signal arrives.
    pub async fn received(self) {
        std::future::pending::<()>().await;
    }
}
