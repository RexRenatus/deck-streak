//! Owner sessions: a new id at every handshake, an idle timeout and an absolute lifetime on the
//! kernel's clock, at most eight live, and an extractor that admits only a live cookie (SPEC-024
//! A10, A11; R5, R7).
//!
//! Every clock is a `ManualClock`, so no test waits for time to pass. The timeouts are the SPEC's
//! numbers, written here as the SPEC gives them, and the crate's constants are held equal to them.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::{COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode};
use axum::routing::get;
use deck_streak_identity::session::{
    ABSOLUTE_LIFETIME, IDLE_TIMEOUT, MAX_LIVE_SESSIONS, SESSION_COOKIE, ended_cookie,
    opening_cookie, presented,
};
use deck_streak_identity::{Owner, OwnerSession, SessionToken, Sessions};
use deck_streak_kernel::{ManualClock, TelegramUserId, UtcMillis};
use tower::ServiceExt;

/// The SPEC's idle timeout: 30 minutes without a request.
const IDLE: Duration = Duration::from_mins(30);
/// The SPEC's absolute lifetime: 8 hours after the session began.
const LIFETIME: Duration = Duration::from_hours(8);
/// The SPEC's bound on live sessions.
const MOST_LIVE: usize = 8;
/// One millisecond, the clock's grain.
const TICK: Duration = Duration::from_millis(1);
/// The most a test reads of a body.
const BODY_READ_LIMIT: usize = 4096;

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

fn owner() -> Owner {
    Owner::new(TelegramUserId::new(4242))
}

/// An empty store on a manual clock stopped at 2025-01-15T12:00:00Z.
fn store() -> (Arc<ManualClock>, Sessions) {
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(
        1_736_942_400_000,
    )));
    let sessions = Sessions::new(clock.clone());
    (clock, sessions)
}

fn open(sessions: &Sessions) -> SessionToken {
    sessions.open(owner()).expect("a session opens")
}

#[test]
fn each_handshake_issues_a_new_session_id() {
    let (_clock, sessions) = store();
    let tokens: Vec<SessionToken> = (0..5).map(|_| open(&sessions)).collect();

    // Five handshakes, five ids: none is ever issued twice.
    let distinct: BTreeSet<&str> = tokens.iter().map(SessionToken::expose).collect();
    assert_eq!(distinct.len(), tokens.len(), "an id was issued twice");

    // Each is 32 bytes as 64 lowercase hex digits, and names its own live session.
    for token in examined("session id(s)", tokens.iter().collect()) {
        let id = token.expose();
        assert_eq!(id.len(), 64, "{} hex digits", id.len());
        assert!(
            id.bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
            "not lowercase hex"
        );
        assert_eq!(sessions.admit(id), Some(owner()));
    }
    assert_eq!(sessions.live(), 5);

    // An id nobody was issued names no session.
    assert_eq!(sessions.admit(&"ab".repeat(32)), None);
}

#[test]
fn an_idle_session_expires_and_an_old_one_ends_at_its_lifetime() {
    assert_eq!((IDLE_TIMEOUT, ABSOLUTE_LIFETIME), (IDLE, LIFETIME));
    let (clock, sessions) = store();

    // A session used within its idle timeout lives on, each request refreshing the timer.
    let busy = open(&sessions);
    for _ in 0..3 {
        clock.advance(IDLE.saturating_sub(TICK));
        assert_eq!(sessions.admit(busy.expose()), Some(owner()), "refreshed");
    }
    // Left idle for the whole timeout, it has ended, and it stays ended.
    clock.advance(IDLE);
    assert_eq!(sessions.admit(busy.expose()), None, "idle for 30 minutes");
    clock.advance(TICK);
    assert_eq!(sessions.admit(busy.expose()), None, "still ended");

    // A session used every twenty minutes ends eight hours after it began.
    let long = open(&sessions);
    let step = Duration::from_mins(20);
    let mut elapsed = Duration::ZERO;
    while elapsed + step < LIFETIME {
        clock.advance(step);
        elapsed += step;
        assert_eq!(
            sessions.admit(long.expose()),
            Some(owner()),
            "{elapsed:?} after it began"
        );
    }
    clock.advance(LIFETIME.saturating_sub(elapsed).saturating_sub(TICK));
    assert_eq!(
        sessions.admit(long.expose()),
        Some(owner()),
        "a millisecond before its lifetime"
    );
    clock.advance(TICK);
    assert_eq!(
        sessions.admit(long.expose()),
        None,
        "at its lifetime, however busy"
    );
    assert_eq!(sessions.live(), 0);
}

#[test]
fn the_oldest_session_is_evicted_past_eight() {
    assert_eq!(MAX_LIVE_SESSIONS, MOST_LIVE);
    let (clock, sessions) = store();
    let mut tokens = Vec::new();
    for _ in 0..MOST_LIVE {
        tokens.push(open(&sessions));
        clock.advance(Duration::from_secs(1));
    }
    assert_eq!(sessions.live(), MOST_LIVE);

    // A ninth evicts the one that began first; the other seven, and the ninth, live on.
    let ninth = open(&sessions);
    assert_eq!(sessions.live(), MOST_LIVE);
    assert_eq!(sessions.admit(tokens[0].expose()), None, "the oldest");
    for token in examined("kept session(s)", tokens[1..].iter().collect()) {
        assert_eq!(sessions.admit(token.expose()), Some(owner()));
    }
    assert_eq!(sessions.admit(ninth.expose()), Some(owner()));
}

#[tokio::test]
async fn the_owner_session_extractor_admits_only_a_live_cookie() {
    let (clock, sessions) = store();
    let app = Router::new()
        .route(
            "/probe",
            get(|session: OwnerSession| async move { session.owner().user().get().to_string() }),
        )
        .with_state(sessions.clone());
    let token = open(&sessions);
    let other = open(&sessions);

    let call = |cookie: Option<String>| {
        let app = app.clone();
        async move {
            let mut request = Request::get("/probe");
            if let Some(cookie) = cookie {
                request = request.header(COOKIE, cookie);
            }
            let response = app
                .oneshot(request.body(Body::empty()).expect("a request"))
                .await
                .expect("the router is infallible");
            let status = response.status();
            let body = to_bytes(response.into_body(), BODY_READ_LIMIT)
                .await
                .expect("a readable body");
            (status, String::from_utf8(body.to_vec()).expect("UTF-8"))
        }
    };
    let live = format!("{SESSION_COOKIE}={}", token.expose());

    // The live cookie is admitted, among other cookies, as its owner.
    let admitted = call(Some(format!("theme=dark; {live}"))).await;
    assert_eq!(admitted, (StatusCode::OK, "4242".to_owned()));

    // No cookie, an id nobody was issued, and two session cookies at once are refused.
    let refusals = [
        None,
        Some(format!("{SESSION_COOKIE}={}", "ab".repeat(32))),
        Some(format!("{live}; {SESSION_COOKIE}={}", other.expose())),
        Some(format!("{SESSION_COOKIE}=not-hex")),
    ];
    for cookie in examined("refused cookie(s)", refusals.to_vec()) {
        assert_eq!(
            call(cookie.clone()).await,
            (
                StatusCode::UNAUTHORIZED,
                "{\"reason\":\"no_session\"}".to_owned()
            ),
            "{cookie:?}"
        );
    }

    // Each admitted request refreshed the idle timer; left idle, the session is refused.
    clock.advance(IDLE.saturating_sub(TICK));
    assert_eq!(call(Some(live.clone())).await.0, StatusCode::OK);
    clock.advance(IDLE);
    assert_eq!(call(Some(live)).await.0, StatusCode::UNAUTHORIZED);
}

/// The handshake's `Set-Cookie`: every attribute the SPEC names, in one place, and the
/// logout's twin that differs only in its value and `Max-Age` (SPEC-024 R6).
#[test]
fn the_opening_and_ending_cookies_carry_every_attribute() {
    let (_clock, sessions) = store();
    let token = open(&sessions);

    let (name, value) = opening_cookie(&token);
    assert_eq!(name, SET_COOKIE);
    assert_eq!(
        value,
        format!(
            "{SESSION_COOKIE}={}; Path=/; Max-Age=28800; Secure; HttpOnly; SameSite=Strict",
            token.expose()
        )
    );
    assert_eq!(ABSOLUTE_LIFETIME.as_secs(), 28_800);

    let (name, value) = ended_cookie();
    assert_eq!(name, SET_COOKIE);
    assert_eq!(
        value,
        "__Host-deckstreak_session=; Path=/; Max-Age=0; Secure; HttpOnly; SameSite=Strict"
    );
    assert!(value.starts_with(SESSION_COOKIE));
}

/// The id a request presents: the one session cookie's value among others, `None` for none, for
/// two (in one header or two), and never a look-alike name.
#[test]
fn the_presented_id_is_the_one_session_cookie_and_nothing_else() {
    let id = "ab".repeat(32);
    let headers = |values: &[&[u8]]| {
        let mut map = HeaderMap::new();
        for value in values {
            map.append(
                COOKIE,
                HeaderValue::from_bytes(value).expect("a header value"),
            );
        }
        map
    };
    let carried = format!("{SESSION_COOKIE}={id}");

    let one = headers(&[format!("theme=dark;   {carried} ; lang=en").as_bytes()]);
    assert_eq!(presented(&one), Some(id.as_str()));

    // A value that is not text is skipped, and the next header still supplies the cookie.
    let skipped = headers(&[b"\xff\xfe=1", carried.as_bytes()]);
    assert_eq!(presented(&skipped), Some(id.as_str()));

    let refused = [
        headers(&[]),
        headers(&[b"theme=dark; flag"]),
        headers(&[format!("x{SESSION_COOKIE}={id}; {SESSION_COOKIE}x={id}").as_bytes()]),
        headers(&[format!("{carried}; {SESSION_COOKIE}={id}").as_bytes()]),
        headers(&[carried.as_bytes(), carried.as_bytes()]),
    ];
    for map in examined("refused header set(s)", refused.into_iter().collect()) {
        assert_eq!(presented(&map), None, "{map:?}");
    }
}

/// Ending a session names whether a live one ended, drops only that one, and an id that is
/// malformed, unknown or already ended ends nothing.
#[test]
fn ending_a_session_ends_that_one_and_reports_whether_it_was_live() {
    let (clock, sessions) = store();
    let first = open(&sessions);
    let second = open(&sessions);
    assert_eq!(sessions.live(), 2);

    for absent in [
        "".to_owned(),
        "ab".repeat(32),
        "AB".repeat(32),
        "ab".repeat(31),
        format!("{}g", "a".repeat(63)),
    ] {
        assert!(!sessions.end_session(&absent), "{absent:?} ended a session");
        assert_eq!(sessions.live(), 2);
    }

    assert!(sessions.end_session(first.expose()));
    assert_eq!(sessions.live(), 1);
    assert_eq!(sessions.admit(first.expose()), None);
    assert_eq!(sessions.admit(second.expose()), Some(owner()));
    assert!(!sessions.end_session(first.expose()), "ended twice");

    // A session whose idle time is up has already ended: there is nothing left to end.
    clock.advance(IDLE);
    assert!(!sessions.end_session(second.expose()));
    assert_eq!(sessions.live(), 0);
}

/// An id is 64 lowercase hex digits and no other text: uppercase, a digit past `f`, the
/// characters either side of each range, and a wrong length all name no session.
#[test]
fn an_id_is_read_only_as_sixty_four_lowercase_hex_digits() {
    let (_clock, sessions) = store();
    let token = open(&sessions);
    let id = token.expose().to_owned();

    assert_eq!(sessions.admit(&id), Some(owner()));
    let upper = id.to_uppercase();
    let expected = (upper == id).then(owner);
    assert_eq!(sessions.admit(&upper), expected, "uppercase digits");
    for stray in ['/', ':', '`', 'g', 'G', '@', ' '] {
        let spoiled = format!("{}{stray}", &id[..63]);
        assert_eq!(sessions.admit(&spoiled), None, "{stray:?} read as a digit");
        let front = format!("{stray}{}", &id[1..]);
        assert_eq!(sessions.admit(&front), None, "{stray:?} read as a digit");
    }
    for wrong in [&id[..62], &id[..63], &format!("{id}0"), &format!("{id}00")] {
        assert_eq!(sessions.admit(wrong), None, "{} digits", wrong.len());
    }
}

/// Neither a token nor the store shows an id in its `Debug`.
#[test]
fn a_token_and_a_store_debug_show_no_id() {
    let (_clock, sessions) = store();
    let token = open(&sessions);
    let _second = open(&sessions);
    assert_eq!(format!("{token:?}"), "SessionToken(..)");
    let shown = format!("{sessions:?}");
    assert!(shown.starts_with("Sessions { live: 2"), "{shown}");
    assert!(!shown.contains(token.expose()));
}
