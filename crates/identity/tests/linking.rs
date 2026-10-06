//! The link code, the session's proof, linking and unlinking a passkey, the refusals' statuses, and
//! the logs the linking flow leaves (SPEC-359 R2 to R9, R13; A1 to A7, A19 to A25, A29, A36).
//!
//! Every response comes from the software authenticator in `support`, against the configured
//! origin `https://app.example`, on a manual clock.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use std::time::Duration;

use axum::body::to_bytes;
use axum::response::IntoResponse as _;
use deck_streak_identity::linking::{LINK_CODE_BYTES, MAX_LIVE_LINK_CODES, TELEGRAM_METHOD};
use deck_streak_identity::session::Proof;
use deck_streak_identity::{Refusal, SignedIn};
use sha2::{Digest as _, Sha256};

use support::{Authenticator, Captured, Fixture, Presented, STRANGER, b64, fixture, unb64};

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
/// Every refusal of identity and the status it answers, written out by hand (R8, R9; SPEC-024).
const STATUSES: [(Refusal, u16); 18] = [
    (Refusal::InitDataInvalid, 401),
    (Refusal::InitDataStale, 401),
    (Refusal::NotOwner, 403),
    (Refusal::NoSession, 401),
    (Refusal::ReauthRequired, 401),
    (Refusal::LinkCodeInvalid, 401),
    (Refusal::LinkCodeExpired, 401),
    (Refusal::ChallengeInvalid, 401),
    (Refusal::ChallengeExpired, 401),
    (Refusal::OriginMismatch, 401),
    (Refusal::UvRequired, 401),
    (Refusal::PasskeyInvalid, 401),
    (Refusal::NotLinked, 401),
    (Refusal::CounterRegressed, 401),
    (Refusal::AlreadyLinked, 409),
    (Refusal::LastMethod, 409),
    (Refusal::LinkingOff, 404),
    (Refusal::IdentityUnknown, 404),
];
/// The most a test reads of a refusal's body.
const BODY_READ_LIMIT: usize = 1024;

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// A NEW `telegram` session for the owner, its handshake now.
fn open_telegram(world: &Fixture) -> String {
    world
        .sessions
        .open(world.owner)
        .expect("a telegram session opens")
        .expose()
        .to_owned()
}

/// A NEW `linked` session, opened by the `passkeys` row `row`.
fn open_linked(world: &Fixture, row: i64) -> String {
    world
        .sessions
        .open_linked(world.owner, row)
        .expect("a linked session opens")
        .expose()
        .to_owned()
}

/// What `token` proves, while it names a live session.
fn proof(world: &Fixture, token: &str) -> Option<Proof> {
    world
        .sessions
        .admit_proof(token)
        .map(|admitted| admitted.proof)
}

/// Mints a link code inside `session`: the code, or the refusal.
fn mint(world: &Fixture, session: &str) -> Result<String, Option<Refusal>> {
    world
        .passkeys
        .mint_link_code(session)
        .map(|code| code.expose().to_owned())
        .map_err(|error| error.refusal())
}

/// Redeems `code` with no session arriving: the `link` session it opens, or the refusal.
fn redeem(world: &Fixture, code: &str) -> Result<String, Option<Refusal>> {
    world
        .passkeys
        .redeem_link_code(code, None)
        .map(|session| session.expose().to_owned())
        .map_err(|error| error.refusal())
}

/// Starts a sign-in and finishes it with `authenticator`'s assertion at `counter`, arriving with
/// the session `arriving`.
async fn sign_in(
    world: &Fixture,
    authenticator: &Authenticator,
    counter: u32,
    arriving: Option<&str>,
) -> Result<SignedIn, Option<Refusal>> {
    let started = world
        .passkeys
        .start_sign_in(&world.db)
        .await
        .map_err(|error| error.refusal())?;
    let response = authenticator.assert(&started.options, &Presented::counter(counter));
    world
        .passkeys
        .finish_sign_in(&world.db, arriving, started.flow.expose(), &response)
        .await
        .map_err(|error| error.refusal())
}

/// Removes the method `id` inside `session`.
async fn remove(world: &Fixture, session: &str, id: i64) -> Result<(), Option<Refusal>> {
    world
        .passkeys
        .remove(&world.db, session, id)
        .await
        .map_err(|error| error.refusal())
}

/// A1: a link code is 16 random bytes, answered once, and the store keeps only its SHA-256.
#[tokio::test]
async fn a_link_code_is_random_and_kept_hashed() {
    let world = fixture().await;
    let session = open_telegram(&world);
    let first = mint(&world, &session).expect("a code is minted");
    let second = mint(&world, &session).expect("a second code is minted");
    assert_ne!(
        first, second,
        "two mints answered one code: it is not random"
    );
    let codes = [unb64(&first), unb64(&second)];
    for code in &codes {
        assert_eq!(code.len(), 16, "a code is not 16 bytes");
    }
    let digests: Vec<Vec<u8>> = codes
        .iter()
        .map(|code| Sha256::digest(code).to_vec())
        .collect();
    assert_eq!(
        world.passkeys.link_codes().digests(),
        digests,
        "the store keeps something other than each code's SHA-256"
    );
}

/// A2: a code redeemed 599 seconds after its mint is accepted, and at 600 refused
/// `link_code_expired`.
#[tokio::test]
async fn a_link_code_expires_at_six_hundred_seconds() {
    let world = fixture().await;
    let session = open_telegram(&world);
    let accepted = mint(&world, &session).expect("a code is minted");
    let expired = mint(&world, &session).expect("a second code is minted");
    world.clock.advance(Duration::from_secs(599));
    assert!(
        redeem(&world, &accepted).is_ok(),
        "a code 599 seconds old was refused"
    );
    world.clock.advance(Duration::from_secs(1));
    assert_eq!(
        redeem(&world, &expired).err(),
        Some(Some(Refusal::LinkCodeExpired)),
        "a code 600 seconds old was not refused link_code_expired"
    );
}

/// A3: a code redeems once; a second redeem, an unknown code and a malformed one are refused
/// `link_code_invalid`.
#[tokio::test]
async fn a_link_code_redeems_once() {
    let world = fixture().await;
    let session = open_telegram(&world);
    let code = mint(&world, &session).expect("a code is minted");
    assert!(redeem(&world, &code).is_ok(), "a fresh code was refused");
    assert_eq!(
        redeem(&world, &code).err(),
        Some(Some(Refusal::LinkCodeInvalid)),
        "a second redeem was not refused link_code_invalid"
    );
    let unknown = b64(&[7_u8; LINK_CODE_BYTES]);
    for presented in [unknown.as_str(), "not a link code"] {
        assert_eq!(
            redeem(&world, presented).err(),
            Some(Some(Refusal::LinkCodeInvalid)),
            "a code never minted was not refused link_code_invalid"
        );
    }
}

/// A4: a code is minted inside a Telegram session whose handshake is 300 seconds old, and refused
/// `reauth_required` at 301 seconds and inside a `link` or `linked` session.
#[tokio::test]
async fn a_link_code_needs_a_fresh_telegram_session() {
    let world = fixture().await;
    let session = open_telegram(&world);
    world.clock.advance(Duration::from_mins(5));
    assert!(
        mint(&world, &session).is_ok(),
        "a Telegram session of 300 seconds minted no code"
    );
    world.clock.advance(Duration::from_secs(1));
    assert_eq!(
        mint(&world, &session).err(),
        Some(Some(Refusal::ReauthRequired)),
        "a Telegram session of 301 seconds minted a code"
    );
    let link = world.link_session();
    assert_eq!(
        mint(&world, &link).err(),
        Some(Some(Refusal::ReauthRequired)),
        "a link session minted a code"
    );
    let row = world.register(&link, &Authenticator::new(7)).await;
    let linked = open_linked(&world, row);
    assert_eq!(
        mint(&world, &linked).err(),
        Some(Some(Refusal::ReauthRequired)),
        "a linked session minted a code"
    );
}

/// A5: a ninth live code evicts the oldest, which is refused `link_code_invalid`; the eight kept
/// still redeem.
#[tokio::test]
async fn a_ninth_link_code_evicts_the_oldest() {
    let world = fixture().await;
    let session = open_telegram(&world);
    let codes: Vec<String> = (0..9)
        .map(|_| mint(&world, &session).expect("a code is minted"))
        .collect();
    assert_eq!(
        redeem(&world, &codes[0]).err(),
        Some(Some(Refusal::LinkCodeInvalid)),
        "the oldest code survived a ninth mint"
    );
    assert_eq!(
        world.passkeys.link_codes().live(),
        8,
        "the store does not hold exactly eight codes"
    );
    assert!(
        redeem(&world, &codes[1]).is_ok(),
        "the oldest code kept was refused"
    );
    assert!(
        redeem(&world, &codes[MAX_LIVE_LINK_CODES - 1]).is_ok(),
        "the eighth code was refused"
    );
}

/// A6: a `link` session is never admitted as an owner session, and the linking use cases refuse it
/// 600 seconds after it opened and not at 599.
#[tokio::test]
async fn a_link_session_is_not_an_owner_session() {
    let world = fixture().await;
    let link = world.link_session();
    assert!(
        world.sessions.admit(&link).is_none(),
        "a link session was admitted as an owner session"
    );
    world.clock.advance(Duration::from_secs(599));
    assert!(
        world
            .passkeys
            .start_registration(&world.db, &link)
            .await
            .is_ok(),
        "a link session of 599 seconds was refused"
    );
    world.clock.advance(Duration::from_secs(1));
    assert_eq!(
        world
            .passkeys
            .start_registration(&world.db, &link)
            .await
            .err()
            .map(|error| error.refusal()),
        Some(Some(Refusal::NoSession)),
        "a link session of 600 seconds still reached a linking use case"
    );
}

/// A7: a `linked` session is admitted as an owner session and mints no link code.
#[tokio::test]
async fn a_linked_session_is_an_owner_session_that_mints_nothing() {
    let world = fixture().await;
    let link = world.link_session();
    let row = world.register(&link, &Authenticator::new(7)).await;
    let linked = open_linked(&world, row);
    assert!(
        world.sessions.admit(&linked).is_some(),
        "a linked session was refused as an owner session"
    );
    assert_eq!(
        mint(&world, &linked).err(),
        Some(Some(Refusal::ReauthRequired)),
        "a linked session minted a link code"
    );
    assert_eq!(world.passkeys.link_codes().live(), 0, "a link code is live");
}

/// A19: a credential registered twice is refused `already_linked` and writes nothing. Both
/// ceremonies start before either finishes, so the exclude list cannot catch the second: only the
/// table's unique key can.
#[tokio::test]
async fn linking_twice_is_refused() {
    let world = fixture().await;
    let link = world.link_session();
    let authenticator = Authenticator::new(7);
    let first = world
        .passkeys
        .start_registration(&world.db, &link)
        .await
        .expect("a registration starts");
    let second = world
        .passkeys
        .start_registration(&world.db, &link)
        .await
        .expect("a second registration starts");
    let response = authenticator.register(&first.options, &Presented::default());
    assert!(
        world
            .passkeys
            .finish_registration(&world.db, &link, first.flow.expose(), &response)
            .await
            .is_ok(),
        "the first registration was refused"
    );
    let response = authenticator.register(&second.options, &Presented::default());
    assert_eq!(
        world
            .passkeys
            .finish_registration(&world.db, &link, second.flow.expose(), &response)
            .await
            .err()
            .map(|error| error.refusal()),
        Some(Some(Refusal::AlreadyLinked)),
        "a credential registered twice was not refused already_linked"
    );
    assert_eq!(world.rows().await, 1, "a second row was written");
}

/// A20: a sign-in with no passkey held, or with a credential no row holds, is refused `not_linked`
/// and writes nothing: no ceremony, no session, no row.
#[tokio::test]
async fn a_sign_in_with_an_unlinked_identity_creates_nothing() {
    let world = fixture().await;
    assert_eq!(
        world
            .passkeys
            .start_sign_in(&world.db)
            .await
            .err()
            .map(|error| error.refusal()),
        Some(Some(Refusal::NotLinked)),
        "a sign-in with no passkey held was not refused not_linked"
    );
    assert_eq!(
        world.passkeys.ceremonies().live(),
        0,
        "a ceremony started with no passkey held"
    );
    let link = world.link_session();
    let row = world.register(&link, &Authenticator::new(7)).await;
    let before = (
        world.rows().await,
        world.sessions.live(),
        world.row(row).await,
    );
    assert_eq!(
        sign_in(&world, &Authenticator::new(9), 1, None).await.err(),
        Some(Some(Refusal::NotLinked)),
        "a credential no row holds was not refused not_linked"
    );
    assert_eq!(
        (
            world.rows().await,
            world.sessions.live(),
            world.row(row).await
        ),
        before,
        "a refused sign-in wrote something"
    );
}

/// A21: a credential whose row belongs to another Telegram user is refused `not_owner`.
#[tokio::test]
async fn a_linked_identity_of_another_user_is_refused() {
    let world = fixture().await;
    let link = world.link_session();
    let _owned = world.register(&link, &Authenticator::new(7)).await;
    let theirs = world.seed(STRANGER, &Authenticator::new(9), 0).await;
    let before = (world.sessions.live(), world.row(theirs).await);
    assert_eq!(
        sign_in(&world, &Authenticator::new(9), 1, None).await.err(),
        Some(Some(Refusal::NotOwner)),
        "another user's passkey was not refused not_owner"
    );
    assert_eq!(
        (world.sessions.live(), world.row(theirs).await),
        before,
        "a refused sign-in wrote something"
    );
}

/// A22: a sign-in ends the session it arrived with and opens a NEW `linked` one for its row.
#[tokio::test]
async fn a_sign_in_rotates_the_session() {
    let world = fixture().await;
    let link = world.link_session();
    let authenticator = Authenticator::new(7);
    let row = world.register(&link, &authenticator).await;
    let arriving = open_telegram(&world);
    let signed_in = sign_in(&world, &authenticator, 1, Some(&arriving))
        .await
        .expect("the sign-in succeeds");
    assert_eq!(
        proof(&world, &arriving),
        None,
        "the session the sign-in arrived with is still live"
    );
    assert_eq!(
        proof(&world, signed_in.session.expose()),
        Some(Proof::Linked(row)),
        "the sign-in opened no linked session for its row"
    );
}

/// A23: the Telegram method is never removed: `last_method`.
#[tokio::test]
async fn unlinking_telegram_is_refused() {
    let world = fixture().await;
    let session = open_telegram(&world);
    assert_eq!(
        remove(&world, &session, TELEGRAM_METHOD).await.err(),
        Some(Some(Refusal::LastMethod)),
        "removing Telegram was not refused last_method"
    );
    let methods = world
        .passkeys
        .methods(&world.db)
        .await
        .expect("the methods are listed");
    assert_eq!(
        methods.first().map(|method| (method.id, method.kind)),
        Some((TELEGRAM_METHOD, "telegram")),
        "Telegram is not the first method"
    );
}

/// A24: a passkey is removed only inside a Telegram session whose handshake is at most 300 seconds
/// old; a `linked`, a `link` or an older Telegram session is refused `reauth_required`.
#[tokio::test]
async fn unlinking_needs_a_fresh_telegram_session() {
    let world = fixture().await;
    let link = world.link_session();
    let row = world.register(&link, &Authenticator::new(7)).await;
    let linked = open_linked(&world, row);
    let stale = open_telegram(&world);
    for (session, what) in [(&linked, "a linked session"), (&link, "a link session")] {
        assert_eq!(
            remove(&world, session, row).await.err(),
            Some(Some(Refusal::ReauthRequired)),
            "{what} removed a passkey"
        );
    }
    world.clock.advance(Duration::from_secs(301));
    assert_eq!(
        remove(&world, &stale, row).await.err(),
        Some(Some(Refusal::ReauthRequired)),
        "a Telegram session of 301 seconds removed a passkey"
    );
    assert_eq!(world.rows().await, 1, "a refused removal deleted the row");
    let fresh = open_telegram(&world);
    world.clock.advance(Duration::from_mins(5));
    assert!(
        remove(&world, &fresh, row).await.is_ok(),
        "a Telegram session of 300 seconds could not remove a passkey"
    );
    assert_eq!(world.rows().await, 0, "the removal deleted no row");
}

/// A25: removing a passkey ends the `linked` sessions that passkey opened, and no other session.
#[tokio::test]
async fn removing_a_passkey_ends_its_sessions() {
    let world = fixture().await;
    let link = world.link_session();
    let removed = world.register(&link, &Authenticator::new(7)).await;
    let kept = world.register(&link, &Authenticator::new(9)).await;
    let opened = open_linked(&world, removed);
    let other = open_linked(&world, kept);
    let session = open_telegram(&world);
    remove(&world, &session, removed)
        .await
        .expect("the passkey is removed");
    assert_eq!(
        proof(&world, &opened),
        None,
        "a session the removed passkey opened is still live"
    );
    assert_eq!(
        proof(&world, &other),
        Some(Proof::Linked(kept)),
        "a session another passkey opened was ended"
    );
    assert_eq!(
        proof(&world, &session),
        Some(Proof::Telegram),
        "the Telegram session was ended"
    );
}

/// A29: no secret of the linking flow reaches a log line or an error, beside a positive control
/// that the capture saw each audit event; each audit event names the event and the row id alone,
/// and a ceremony's `Debug` is its redacted form.
#[allow(
    clippy::too_many_lines,
    reason = "one census: every linking step runs under one capture beside its positive control"
)]
#[tokio::test]
async fn no_linking_secret_reaches_a_log() {
    let captured = Captured::default();
    let _subscriber = tracing::subscriber::set_default(captured.clone());
    let world = fixture().await;
    let authenticator = Authenticator::new(7);
    let telegram = open_telegram(&world);
    let code = mint(&world, &telegram).expect("a code is minted");
    let link = world
        .passkeys
        .redeem_link_code(&code, Some(&telegram))
        .expect("the code redeems")
        .expose()
        .to_owned();
    // A second redeem's error, the one error of the flow that has the code in hand.
    let refused = world
        .passkeys
        .redeem_link_code(&code, None)
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
    let registration = world
        .passkeys
        .start_registration(&world.db, &link)
        .await
        .expect("a registration starts");
    let response = authenticator.register(&registration.options, &Presented::default());
    let row = world
        .passkeys
        .finish_registration(&world.db, &link, registration.flow.expose(), &response)
        .await
        .expect("the registration finishes");
    let started = world
        .passkeys
        .start_sign_in(&world.db)
        .await
        .expect("a sign-in starts");
    let assertion = authenticator.assert(&started.options, &Presented::counter(1));
    let signed_in = world
        .passkeys
        .finish_sign_in(&world.db, Some(&link), started.flow.expose(), &assertion)
        .await
        .expect("the sign-in succeeds");
    let spare = world
        .passkeys
        .start_sign_in(&world.db)
        .await
        .expect("a spare sign-in starts");
    let ceremony = world
        .passkeys
        .ceremonies()
        .take(spare.flow.expose())
        .expect("the spare ceremony is live");
    let fresh = open_telegram(&world);
    remove(&world, &fresh, row)
        .await
        .expect("the passkey is removed");
    let challenge = |options: &serde_json::Value| {
        options["publicKey"]["challenge"]
            .as_str()
            .expect("the options carry a challenge")
            .to_owned()
    };
    let secrets = vec![
        ("the link code", code.clone()),
        ("the Telegram session's id", telegram.clone()),
        ("the link session's id", link.clone()),
        (
            "the linked session's id",
            signed_in.session.expose().to_owned(),
        ),
        ("the second Telegram session's id", fresh.clone()),
        (
            "the registration's flow id",
            registration.flow.expose().to_owned(),
        ),
        ("the sign-in's flow id", started.flow.expose().to_owned()),
        (
            "the spare sign-in's flow id",
            spare.flow.expose().to_owned(),
        ),
        (
            "the registration's challenge",
            challenge(&registration.options),
        ),
        ("the sign-in's challenge", challenge(&started.options)),
        ("the credential id", b64(authenticator.credential_id())),
        ("the public key", b64(&authenticator.cose_key())),
        ("the user handle", signed_in.user_handle.clone()),
    ];

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
    assert_eq!(
        format!("{ceremony:?}"),
        "Ceremony { .. }",
        "a ceremony's Debug is not its redacted form"
    );
    let lines = examined("captured log lines", lines);
    for (what, secret) in examined("linking secrets", secrets) {
        assert!(!secret.is_empty(), "{what} is empty, so nothing was judged");
        for line in &lines {
            assert!(
                !line.rendered().contains(&secret),
                "{what} reached a log line from {}",
                line.target
            );
        }
        assert!(!refused.contains(&secret), "{what} reached an error");
    }
}

/// A36: each refusal of identity answers its status, by a literal table of every reason code, as
/// SPEC-024's `{"reason": ...}` body.
#[tokio::test]
async fn each_refusal_answers_its_status() {
    for (refusal, status) in examined("refusals", STATUSES.to_vec()) {
        let response = refusal.into_response();
        let answered = response.status().as_u16();
        let body = to_bytes(response.into_body(), BODY_READ_LIMIT)
            .await
            .expect("a readable body");
        assert_eq!(
            (refusal.reason(), answered),
            (refusal.reason(), status),
            "a refusal answers another status"
        );
        assert_eq!(
            String::from_utf8_lossy(&body),
            format!("{{\"reason\":\"{}\"}}", refusal.reason()),
            "a refusal's body is not its reason alone"
        );
    }
}
