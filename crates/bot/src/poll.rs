//! The long poll: the drain at start, the loop, the offset and the backoff (SPEC-026 R2, R3, R13;
//! ADR-026).
//!
//! The bot is the one poller of its token. At start it removes any webhook, registers its menu, and
//! drains: one `getUpdates` at offset -1 returns the newest queued update, and the offset is set
//! past it, so every update queued before the start is confirmed and none is dispatched (the
//! predecessor's `bot.py:CommandBot._drain_offset`). The start's requests are retried until they
//! succeed. Then it polls: each `getUpdates` waits up to [`LONG_POLL_SECONDS`] for updates of the
//! kinds in [`ALLOWED_UPDATES`], each update is handled in turn, and the offset becomes its
//! `update_id` + 1, which the next request confirms. A failed request waits the predecessor's
//! backoff ([`backoff`], `goldens/poll_backoff.json`) before the next, on the transport's waits,
//! which are tokio's timer in the service; the drain and the poll share one count of consecutive
//! failures, which a success resets.
//!
//! The role is ready once the first long poll is issued. When the shutdown future resolves, a poll
//! in flight is abandoned, the batch in hand is finished, the offset is confirmed, and the loop
//! returns.

use std::future::Future;
use std::pin::pin;
use std::time::Duration;

use frankenstein::types::AllowedUpdate;

use crate::commands::{Commands, OwnerSync};
use crate::transport::{Incoming, Transport, TransportError};

/// How long one poll waits for an update, in seconds: the predecessor's `bot.py:CommandBot`
/// default, proved by `goldens/bot.timeouts.json`.
pub const LONG_POLL_SECONDS: u32 = 50;
/// The kinds of update the bot asks for: the owner's messages and the owner's taps. Telegram keeps
/// the list between requests, so every request names it.
pub const ALLOWED_UPDATES: [AllowedUpdate; 2] =
    [AllowedUpdate::Message, AllowedUpdate::CallbackQuery];
/// The drain's offset: the newest queued update, returned without being confirmed.
pub const DRAIN_OFFSET: i64 = -1;
/// A request that waits for nothing: the drain's, and the offset's last confirmation.
const NO_WAIT: u32 = 0;
/// The last confirmation asks for at most one update, which it does not handle.
const CONFIRM_LIMIT: u32 = 1;

/// The backoff's first wait, in seconds, and the ceiling every later one is held under: the
/// predecessor's `bot.py:CommandBot._backoff_delay`, proved by `goldens/poll_backoff.json`.
const BACKOFF_BASE_SECONDS: u64 = 3;
const BACKOFF_CEILING_SECONDS: u64 = 60;
/// The predecessor clamps the exponent before it doubles, so no count of failures overflows.
const BACKOFF_MAX_EXPONENT: u32 = 10;

/// The wait after `consecutive_failures` failed requests in a row: 3 s, doubling, held at 60 s.
#[must_use]
pub fn backoff(consecutive_failures: u32) -> Duration {
    let exponent = consecutive_failures
        .saturating_sub(1)
        .min(BACKOFF_MAX_EXPONENT);
    Duration::from_secs((BACKOFF_BASE_SECONDS << exponent).min(BACKOFF_CEILING_SECONDS))
}

/// The poll's state: the offset, the last one a request carried, and the failures in a row.
#[derive(Debug, Default)]
pub struct Poller {
    offset: i64,
    confirmed: i64,
    failures: u32,
}

impl Poller {
    /// A poller before its drain: offset 0, nothing confirmed, no failure.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Moves the offset past `update_id`, once that update is handled.
    pub fn handled(&mut self, update_id: Option<i64>) {
        if let Some(update_id) = update_id {
            self.offset = self.offset.max(update_id.saturating_add(1));
        }
    }

    /// Counts a failed request, and returns the wait before the next.
    pub fn failed(&mut self) -> Duration {
        self.failures = self.failures.saturating_add(1);
        backoff(self.failures)
    }

    /// Counts a request that succeeded: the failures in a row start again from none.
    pub const fn succeeded(&mut self) {
        self.failures = 0;
    }

    /// The drain: sets the offset past the newest queued update, dispatching nothing.
    ///
    /// # Errors
    ///
    /// The request's [`TransportError`].
    pub async fn drain(&mut self, transport: &Transport) -> Result<(), TransportError> {
        let queued = transport
            .get_updates(DRAIN_OFFSET, NO_WAIT, None, &ALLOWED_UPDATES)
            .await?;
        if let Some(newest) = queued
            .iter()
            .filter_map(|incoming| incoming.update_id)
            .max()
        {
            self.offset = newest.saturating_add(1);
            tracing::info!(
                offset = self.offset,
                "the updates queued before the start were drained"
            );
        }
        Ok(())
    }

    /// One long poll at the offset, which confirms every update before it.
    ///
    /// # Errors
    ///
    /// The request's [`TransportError`].
    pub async fn poll(&mut self, transport: &Transport) -> Result<Vec<Incoming>, TransportError> {
        self.confirmed = self.offset;
        transport
            .get_updates(self.offset, LONG_POLL_SECONDS, None, &ALLOWED_UPDATES)
            .await
    }

    /// Confirms the offset, when an update was handled since the last request carried it: one
    /// request that waits for nothing, whose answer is not handled.
    pub async fn confirm(&mut self, transport: &Transport) {
        if self.offset <= self.confirmed {
            return;
        }
        match transport
            .get_updates(self.offset, NO_WAIT, Some(CONFIRM_LIMIT), &ALLOWED_UPDATES)
            .await
        {
            Ok(_) => {
                self.confirmed = self.offset;
                tracing::info!(offset = self.offset, "the offset was confirmed");
            }
            Err(error) => tracing::warn!(%error, "the offset could not be confirmed"),
        }
    }
}

/// Runs the bot until `shutdown` resolves: the start, the drain, then the long poll, handing each
/// update to `commands`. `ready` is called once, as the first long poll is issued.
pub async fn run<S, F, R>(transport: &Transport, commands: &mut Commands<S>, shutdown: F, ready: R)
where
    S: OwnerSync,
    F: Future<Output = ()>,
    R: FnOnce(),
{
    let mut shutdown = pin!(shutdown);
    let mut poller = Poller::new();
    loop {
        let removed = tokio::select! {
            biased;
            () = &mut shutdown => return,
            removed = transport.delete_webhook() => removed,
        };
        match pause(
            transport,
            &mut poller,
            "deleteWebhook",
            removed,
            &mut shutdown,
        )
        .await
        {
            Step::Done => break,
            Step::Again => {}
            Step::Stop => return,
        }
    }
    commands.register_menu().await;
    loop {
        let drained = tokio::select! {
            biased;
            () = &mut shutdown => return,
            drained = poller.drain(transport) => drained,
        };
        match pause(transport, &mut poller, "drain", drained, &mut shutdown).await {
            Step::Done => break,
            Step::Again => {}
            Step::Stop => return,
        }
    }
    let mut ready = Some(ready);
    loop {
        if let Some(ready) = ready.take() {
            ready();
        }
        let answer = tokio::select! {
            biased;
            () = &mut shutdown => break,
            answer = poller.poll(transport) => answer,
        };
        match answer {
            Ok(batch) => {
                poller.succeeded();
                for incoming in batch {
                    let update_id = incoming.update_id;
                    commands.handle(incoming).await;
                    poller.handled(update_id);
                }
            }
            Err(error) => {
                let wait = poller.failed();
                tracing::warn!(%error, wait_s = wait.as_secs(), "a poll failed");
                tokio::select! {
                    biased;
                    () = &mut shutdown => break,
                    () = transport.wait(wait) => {}
                }
            }
        }
    }
    poller.confirm(transport).await;
}

/// What follows one request of the start.
enum Step {
    /// It succeeded: the start goes on.
    Done,
    /// It failed and its backoff has passed: it goes again.
    Again,
    /// The shutdown came during the backoff.
    Stop,
}

/// After a request of the start: [`Step::Done`] when it succeeded, else the backoff, then
/// [`Step::Again`], unless the shutdown comes first.
async fn pause<T, F>(
    transport: &Transport,
    poller: &mut Poller,
    step: &'static str,
    outcome: Result<T, TransportError>,
    shutdown: &mut std::pin::Pin<&mut F>,
) -> Step
where
    F: Future<Output = ()>,
{
    match outcome {
        Ok(_) => {
            poller.succeeded();
            Step::Done
        }
        Err(error) => {
            let wait = poller.failed();
            tracing::warn!(step, %error, wait_s = wait.as_secs(), "a start request failed");
            tokio::select! {
                biased;
                () = shutdown.as_mut() => Step::Stop,
                () = transport.wait(wait) => Step::Again,
            }
        }
    }
}
