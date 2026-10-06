//! The link code, the session's proof, linking and unlinking a passkey, the refusals' statuses, and
//! the logs the linking flow leaves (SPEC-359 R2 to R9, R13; A1 to A7, A19 to A25, A29, A36).

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use std::sync::Arc;

use deck_streak_identity::{Owner, Sessions};
use deck_streak_kernel::{ManualClock, TelegramUserId, UtcMillis};

use support::Captured;

/// The synthetic owner, a user id of fewer than seven digits.
const OWNER: i64 = 4242;
/// 2025-01-15T03:30:10Z, in milliseconds.
const STARTED_AT: i64 = 1_736_911_810_000;
/// The audit events the linking flow emits, each named by its `event` field (R13).
const AUDIT_EVENTS: [&str; 5] = [
    "link_code_minted",
    "link_code_redeemed",
    "passkey_registered",
    "passkey_signed_in",
    "passkey_removed",
];
/// The only fields an audit event carries: its message, its name and the row id (R13).
const AUDIT_FIELDS: [&str; 3] = ["message", "event", "row"];

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// A29: the linking flow's secrets never reach a log line, beside a positive control that the
/// capture saw each audit event, and each audit event names the event and the row id alone.
#[tokio::test]
async fn no_linking_secret_reaches_a_log() {
    let captured = Captured::default();
    let _subscriber = tracing::subscriber::set_default(captured.clone());
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let sessions = Sessions::new(clock);
    let owner = Owner::new(TelegramUserId::new(OWNER));
    let telegram = sessions.open(owner).expect("a Telegram session opens");
    let secrets = vec![("the Telegram session's id", telegram.expose().to_owned())];

    let lines = captured.lines();
    for event in AUDIT_EVENTS {
        assert!(
            lines.iter().any(|line| line.field("event") == Some(event)),
            "the capture saw no audit event `{event}`: the positive control found none"
        );
    }
    for line in lines.iter().filter(|line| line.field("event").is_some()) {
        for (name, _) in &line.fields {
            assert!(
                AUDIT_FIELDS.contains(&name.as_str()),
                "the audit event `{}` carries the field `{name}`",
                line.field("event").unwrap_or_default()
            );
        }
    }
    let lines = examined("captured log lines", lines);
    for (what, secret) in examined("linking secrets", secrets) {
        for line in &lines {
            assert!(
                !line.rendered().contains(&secret),
                "{what} reached a log line from {}",
                line.target
            );
        }
    }
}
