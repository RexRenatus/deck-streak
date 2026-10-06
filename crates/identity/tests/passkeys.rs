//! Passkey ceremonies and the counter rule (SPEC-359 R5 to R8; A8 to A18, A35).
//!
//! Every response comes from the software authenticator in `support`, against the configured
//! origin `https://app.example`, on a manual clock.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

mod support;

use std::time::Duration;

use deck_streak_identity::Refusal;
use deck_streak_identity::passkeys::{USER_NAME, counter_advances};
use deck_streak_identity::{LinkingConfig, Owner, PasskeyError, Passkeys};
use deck_streak_kernel::TelegramUserId;
use serde_json::Value;

use support::ORIGIN;
use support::{Authenticator, ELSEWHERE, Fixture, OWNER, Presented, USER_PRESENT, fixture, unb64};

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// Starts a sign-in and finishes it with `authenticator`'s assertion as `presented` shapes it.
async fn sign_in(
    world: &Fixture,
    authenticator: &Authenticator,
    presented: &Presented,
) -> Result<i64, Option<Refusal>> {
    let started = world
        .passkeys
        .start_sign_in(&world.db)
        .await
        .map_err(|error| error.refusal())?;
    finish(
        world,
        started.flow.expose(),
        &started.options,
        authenticator,
        presented,
    )
    .await
}

/// Finishes the sign-in `flow` names with `authenticator`'s assertion to `options`.
async fn finish(
    world: &Fixture,
    flow: &str,
    options: &Value,
    authenticator: &Authenticator,
    presented: &Presented,
) -> Result<i64, Option<Refusal>> {
    let response = authenticator.assert(options, presented);
    world
        .passkeys
        .finish_sign_in(&world.db, None, flow, &response)
        .await
        .map(|signed_in| signed_in.row)
        .map_err(|error| error.refusal())
}

/// Starts a registration in `session` and finishes it with `authenticator` as `presented` shapes
/// it.
async fn register(
    world: &Fixture,
    session: &str,
    authenticator: &Authenticator,
    presented: &Presented,
) -> Result<i64, Option<Refusal>> {
    let started = world
        .passkeys
        .start_registration(&world.db, session)
        .await
        .map_err(|error| error.refusal())?;
    let response = authenticator.register(&started.options, presented);
    world
        .passkeys
        .finish_registration(&world.db, session, started.flow.expose(), &response)
        .await
        .map_err(|error| error.refusal())
}

/// A8: a response naming another origin is refused `origin_mismatch`, at registration and at
/// sign-in, and writes nothing.
#[tokio::test]
async fn a_passkey_from_another_origin_is_refused() {
    let world = fixture().await;
    let elsewhere = Presented {
        origin: ELSEWHERE.to_owned(),
        ..Presented::default()
    };
    let session = world.link_session();
    assert_eq!(
        register(&world, &session, &Authenticator::new(1), &elsewhere).await,
        Err(Some(Refusal::OriginMismatch)),
        "a registration from another origin"
    );
    assert_eq!(
        world.rows().await,
        0,
        "the refused registration wrote a row"
    );
    let row = world.seed(OWNER, &Authenticator::new(2), 5).await;
    assert_eq!(
        sign_in(
            &world,
            &Authenticator::new(2),
            &Presented {
                counter: 6,
                ..elsewhere
            }
        )
        .await,
        Err(Some(Refusal::OriginMismatch)),
        "a sign-in from another origin"
    );
    assert_eq!(
        world.row(row).await,
        Some((5, 0, None)),
        "the refused sign-in wrote"
    );
}

/// A9: a response without user verification is refused `uv_required`, at registration and at
/// sign-in.
#[tokio::test]
async fn a_passkey_without_user_verification_is_refused() {
    let world = fixture().await;
    let unverified = Presented {
        flags: USER_PRESENT,
        ..Presented::default()
    };
    let session = world.link_session();
    assert_eq!(
        register(&world, &session, &Authenticator::new(1), &unverified).await,
        Err(Some(Refusal::UvRequired)),
        "a registration without user verification"
    );
    assert_eq!(
        world.rows().await,
        0,
        "the refused registration wrote a row"
    );
    let row = world.seed(OWNER, &Authenticator::new(2), 5).await;
    assert_eq!(
        sign_in(
            &world,
            &Authenticator::new(2),
            &Presented {
                counter: 6,
                ..unverified
            }
        )
        .await,
        Err(Some(Refusal::UvRequired)),
        "a sign-in without user verification"
    );
    assert_eq!(
        world.row(row).await,
        Some((5, 0, None)),
        "the refused sign-in wrote"
    );
}

/// A10: a ceremony is used once: its second finish, and a flow id no ceremony has, are refused
/// `challenge_invalid`.
#[tokio::test]
async fn a_reused_passkey_challenge_is_refused() {
    let world = fixture().await;
    let authenticator = Authenticator::new(2);
    world.seed(OWNER, &authenticator, 0).await;
    let started = world
        .passkeys
        .start_sign_in(&world.db)
        .await
        .expect("a sign-in starts");
    let flow = started.flow.expose().to_owned();
    assert!(
        finish(
            &world,
            &flow,
            &started.options,
            &authenticator,
            &Presented::counter(1)
        )
        .await
        .is_ok(),
        "the first finish signs in"
    );
    assert_eq!(
        finish(
            &world,
            &flow,
            &started.options,
            &authenticator,
            &Presented::counter(2)
        )
        .await,
        Err(Some(Refusal::ChallengeInvalid)),
        "the ceremony's second finish"
    );
    assert_eq!(
        finish(
            &world,
            &"0".repeat(64),
            &started.options,
            &authenticator,
            &Presented::counter(3)
        )
        .await,
        Err(Some(Refusal::ChallengeInvalid)),
        "a flow id no ceremony has"
    );
}

/// A11: a ceremony finished 299 seconds after it started is accepted, and one finished at 300
/// seconds is refused `challenge_expired`.
#[tokio::test]
async fn a_passkey_challenge_expires_at_three_hundred_seconds() {
    let world = fixture().await;
    let authenticator = Authenticator::new(2);
    world.seed(OWNER, &authenticator, 0).await;
    let started = world
        .passkeys
        .start_sign_in(&world.db)
        .await
        .expect("a start");
    world.clock.advance(Duration::from_secs(299));
    let first = finish(
        &world,
        started.flow.expose(),
        &started.options,
        &authenticator,
        &Presented::counter(1),
    )
    .await;
    assert!(
        first.is_ok(),
        "a ceremony 299 seconds old was refused: {first:?}"
    );
    let started = world
        .passkeys
        .start_sign_in(&world.db)
        .await
        .expect("a start");
    world.clock.advance(Duration::from_mins(5));
    assert_eq!(
        finish(
            &world,
            started.flow.expose(),
            &started.options,
            &authenticator,
            &Presented::counter(2)
        )
        .await,
        Err(Some(Refusal::ChallengeExpired)),
        "a ceremony 300 seconds old"
    );
}

/// A12: an assertion whose counter does not advance, equal or lower, is refused
/// `counter_regressed`, and the stored row is unchanged.
#[tokio::test]
async fn a_regressed_counter_is_refused() {
    let world = fixture().await;
    let authenticator = Authenticator::new(2);
    let row = world.seed(OWNER, &authenticator, 5).await;
    for presented in [5, 4] {
        assert_eq!(
            sign_in(&world, &authenticator, &Presented::counter(presented)).await,
            Err(Some(Refusal::CounterRegressed)),
            "a counter of {presented} over a stored 5"
        );
        assert_eq!(
            world.row(row).await,
            Some((5, 0, None)),
            "the refused sign-in wrote"
        );
    }
}

/// A13: a sign-in stores the presented counter, the backup state and the instant of use.
#[tokio::test]
async fn a_passkey_sign_in_stores_its_counter() {
    let world = fixture().await;
    let authenticator = Authenticator::new(2);
    let row = world.seed(OWNER, &authenticator, 5).await;
    world.clock.advance(Duration::from_secs(10));
    let signed_in = sign_in(&world, &authenticator, &Presented::counter(7)).await;
    assert_eq!(signed_in, Ok(row), "the sign-in");
    assert_eq!(
        world.row(row).await,
        Some((7, 0, Some(support::STARTED_AT + 10_000))),
        "the row after the sign-in: counter, backup state, last use"
    );
}

/// A14: the user handle is a random version-4 UUID, the one every passkey of the owner shares,
/// and the user name and display name carry no personal data.
#[tokio::test]
async fn the_user_handle_carries_no_personal_data() {
    let world = fixture().await;
    let session = world.link_session();
    let mut handles = Vec::new();
    for seed in [1, 2] {
        let started = world
            .passkeys
            .start_registration(&world.db, &session)
            .await
            .expect("a registration starts");
        let user = &started.options["publicKey"]["user"];
        assert_eq!(user["name"], USER_NAME, "the user name");
        assert_eq!(user["displayName"], USER_NAME, "the display name");
        handles.push(unb64(user["id"].as_str().expect("a user handle")));
        let response = Authenticator::new(seed).register(&started.options, &Presented::default());
        world
            .passkeys
            .finish_registration(&world.db, &session, started.flow.expose(), &response)
            .await
            .expect("the registration finishes");
    }
    assert_eq!(handles[0], handles[1], "the second registration's handle");
    let handle = &handles[0];
    assert_eq!(handle.len(), 16, "the handle's length");
    assert_eq!(handle[6] >> 4, 4, "the handle's version");
    assert_eq!(handle[8] >> 6, 0b10, "the handle's variant");
    assert!(
        !String::from_utf8_lossy(handle).contains(&OWNER.to_string()),
        "the handle carries the owner's id"
    );
}

/// A15: the counter rule accepts two zeros and an advance, and refuses an equal, a lower and a
/// zero after a non-zero counter, over its literal table.
#[test]
fn the_counter_rule_accepts_only_an_advance_or_two_zeros() {
    let table: [(u32, u32, bool); 9] = [
        (0, 0, true),
        (0, 1, true),
        (5, 6, true),
        (u32::MAX - 1, u32::MAX, true),
        (1, 1, false),
        (5, 5, false),
        (5, 4, false),
        (5, 0, false),
        (u32::MAX, 0, false),
    ];
    for (stored, presented, accepted) in examined("counter cases", table.to_vec()) {
        assert_eq!(
            counter_advances(stored, presented),
            accepted,
            "stored {stored}, presented {presented}"
        );
    }
}

/// A16: a ninth live ceremony evicts the oldest, which is refused `challenge_invalid`, and the
/// others still finish.
#[tokio::test]
async fn a_ninth_ceremony_evicts_the_oldest() {
    let world = fixture().await;
    let authenticator = Authenticator::new(2);
    world.seed(OWNER, &authenticator, 0).await;
    let mut started = Vec::new();
    for _ in 0..9 {
        started.push(
            world
                .passkeys
                .start_sign_in(&world.db)
                .await
                .expect("a start"),
        );
        world.clock.advance(Duration::from_millis(1));
    }
    let oldest = &started[0];
    assert_eq!(
        finish(
            &world,
            oldest.flow.expose(),
            &oldest.options,
            &authenticator,
            &Presented::counter(1)
        )
        .await,
        Err(Some(Refusal::ChallengeInvalid)),
        "the evicted ceremony"
    );
    assert_eq!(world.passkeys.ceremonies().live(), 8, "the live ceremonies");
    let ninth = &started[8];
    let finished = finish(
        &world,
        ninth.flow.expose(),
        &ninth.options,
        &authenticator,
        &Presented::counter(2),
    )
    .await;
    assert!(
        finished.is_ok(),
        "the ninth ceremony was refused: {finished:?}"
    );
}

/// A17: a registration ceremony finished in another `link` session is refused
/// `challenge_invalid`, and writes nothing.
#[tokio::test]
async fn a_ceremony_finishes_only_in_its_own_session() {
    let world = fixture().await;
    let started_in = world.link_session();
    let other = world.link_session();
    let started = world
        .passkeys
        .start_registration(&world.db, &started_in)
        .await
        .expect("a registration starts");
    let response = Authenticator::new(1).register(&started.options, &Presented::default());
    let finished = world
        .passkeys
        .finish_registration(&world.db, &other, started.flow.expose(), &response)
        .await
        .map_err(|error| error.refusal());
    assert_eq!(
        finished,
        Err(Some(Refusal::ChallengeInvalid)),
        "the finish in another session"
    );
    assert_eq!(
        world.rows().await,
        0,
        "the refused registration wrote a row"
    );
}

/// A18: an assertion whose signature does not verify is refused `passkey_invalid`, and the row is
/// unchanged.
#[tokio::test]
async fn a_passkey_with_a_foreign_signature_is_refused() {
    let world = fixture().await;
    let authenticator = Authenticator::new(2);
    let row = world.seed(OWNER, &authenticator, 5).await;
    let forged = Presented {
        foreign_signature: true,
        ..Presented::counter(6)
    };
    assert_eq!(
        sign_in(&world, &authenticator, &forged).await,
        Err(Some(Refusal::PasskeyInvalid)),
        "a foreign signature"
    );
    assert_eq!(
        world.row(row).await,
        Some((5, 0, None)),
        "the refused sign-in wrote"
    );
}

/// A35: two ceremonies read one counter; the first advances it, and the second, whose counter
/// advances only the value it read, is refused `counter_regressed`: the row keeps the newer
/// counter.
#[tokio::test]
async fn a_counter_moved_since_its_read_is_refused() {
    let world = fixture().await;
    let authenticator = Authenticator::new(2);
    let row = world.seed(OWNER, &authenticator, 5).await;
    let first = world
        .passkeys
        .start_sign_in(&world.db)
        .await
        .expect("a start");
    let second = world
        .passkeys
        .start_sign_in(&world.db)
        .await
        .expect("a start");
    let moved = finish(
        &world,
        first.flow.expose(),
        &first.options,
        &authenticator,
        &Presented::counter(9),
    )
    .await;
    assert_eq!(moved, Ok(row), "the first sign-in");
    assert_eq!(
        finish(
            &world,
            second.flow.expose(),
            &second.options,
            &authenticator,
            &Presented::counter(6)
        )
        .await,
        Err(Some(Refusal::CounterRegressed)),
        "the second sign-in, over the counter it read"
    );
    assert_eq!(
        world.row(row).await.map(|(counter, ..)| counter),
        Some(9),
        "the row's counter after the refused sign-in"
    );
}

/// A started ceremony's flow id `Debug`s as `FlowId(..)`, and the ceremony store and the use cases
/// as the live count, so no flow id reaches a log (SPEC-359 R13).
#[tokio::test]
async fn a_ceremony_and_its_stores_debug_without_the_flow_id() {
    let world = fixture().await;
    let session = world.link_session();
    let started = world
        .passkeys
        .start_registration(&world.db, &session)
        .await
        .expect("a registration starts");
    let rendered = [
        format!("{:?}", started.flow),
        format!("{:?}", world.passkeys.ceremonies()),
        format!("{:?}", world.passkeys),
    ];
    assert_eq!(
        rendered,
        [
            "FlowId(..)",
            "Ceremonies { live: 1, .. }",
            "Passkeys { ceremonies: Ceremonies { live: 1, .. }, .. }",
        ],
        "a flow id or a store does not debug as its redacted form"
    );
    let flow = started.flow.expose();
    assert_eq!(flow.len(), 64, "the flow id is 32 bytes as hex");
    for text in examined("Debug renderings", rendered.to_vec()) {
        assert!(!text.contains(flow), "the flow id reached `{text}`");
    }
}

/// An assertion signed over another ceremony's challenge is refused `challenge_invalid`, and the
/// row is unchanged (SPEC-359 R8).
#[tokio::test]
async fn an_assertion_over_another_ceremonys_challenge_is_refused() {
    let world = fixture().await;
    let authenticator = Authenticator::new(2);
    let row = world.seed(OWNER, &authenticator, 5).await;
    let signed = world
        .passkeys
        .start_sign_in(&world.db)
        .await
        .expect("a start");
    let presented = world
        .passkeys
        .start_sign_in(&world.db)
        .await
        .expect("a start");
    assert_eq!(
        finish(
            &world,
            presented.flow.expose(),
            &signed.options,
            &authenticator,
            &Presented::counter(6)
        )
        .await,
        Err(Some(Refusal::ChallengeInvalid)),
        "an assertion over another ceremony's challenge"
    );
    assert_eq!(
        world.row(row).await,
        Some((5, 0, None)),
        "the refused sign-in wrote"
    );
}

/// A registration the store refuses for a reason other than a duplicate credential answers a store
/// failure, never `already_linked`: owner id 0, which the table's check refuses (SPEC-359 R5).
#[tokio::test]
async fn a_registration_the_store_refuses_is_not_already_linked() {
    let world = fixture().await;
    let config = LinkingConfig::from_setting(Some(ORIGIN)).expect("the origin is an https origin");
    let refused = Owner::new(TelegramUserId::new(0));
    let passkeys = Passkeys::new(config, world.sessions.clone(), refused, world.clock.clone());
    let session = world.link_session();
    let started = passkeys
        .start_registration(&world.db, &session)
        .await
        .expect("a registration starts");
    let response = Authenticator::new(2).register(&started.options, &Presented::default());
    let answered = passkeys
        .finish_registration(&world.db, &session, started.flow.expose(), &response)
        .await;
    assert!(
        matches!(answered, Err(PasskeyError::Store(_))),
        "the store's refusal answered otherwise: {answered:?}"
    );
    assert_eq!(
        answered.err().and_then(|error| error.refusal()),
        None,
        "a store failure answered a refusal"
    );
    assert_eq!(world.rows().await, 0, "a refused registration wrote a row");
}
