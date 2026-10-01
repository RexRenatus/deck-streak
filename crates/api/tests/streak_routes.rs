//! The owner's streak and governor routes over the shell's layers (SPEC-076 A20; R20, R21): both
//! answer the owner's live session alone, with 401 or 403 and no data otherwise, and the owner reads
//! both tracks and the governor's verdict.
//!
//! The launch payloads are SPEC-024's, the same synthetic ones the level routes' tests use, dated
//! 2025-01-15T03:30:00Z; the clock starts ten seconds later, so the study day is the 14th.

// An integration test is test code: its helpers panic on a failed fixture, and the examined counts
// are printed on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::{CONTENT_TYPE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode};
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_kernel::{Db, ManualClock, StudyDayRule, TelegramUserId, UtcMillis};
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

/// The synthetic bot token Python signed the payloads for.
const BOT_TOKEN: &str = "synthetic-webapp-signing-token";
/// The synthetic owner.
const OWNER: i64 = 4242;
/// The owner's launch data, signed for [`BOT_TOKEN`].
const OWNER_PAYLOAD: &str = concat!(
    "auth_date=1736911800&query_id=synthetic-query",
    "&user=%7B%22id%22%3A4242%2C%22first_name%22%3A%22Ada%22%2C",
    "%22username%22%3A%22synthetic_owner%22%7D",
    "&hash=79502d49032542c5030e80b666d03f5af53e053f97d387806adb0aa843ddeb2d",
);
/// Another user's launch data, validly signed for [`BOT_TOKEN`].
const STRANGER_PAYLOAD: &str = concat!(
    "auth_date=1736911800&query_id=synthetic-query",
    "&user=%7B%22id%22%3A777%2C%22first_name%22%3A%22Ada%22%2C",
    "%22username%22%3A%22synthetic_owner%22%7D",
    "&hash=2c30fb14ffed3eb8ddf5d9836d760774b91a30db863140cb57e05ea512f308dd",
);
/// The owner's fields, signed for another bot's token: forged, for this bot.
const FORGED_PAYLOAD: &str = concat!(
    "auth_date=1736911800&query_id=synthetic-query",
    "&user=%7B%22id%22%3A4242%2C%22first_name%22%3A%22Ada%22%2C",
    "%22username%22%3A%22synthetic_owner%22%7D",
    "&hash=eb0bcf2041fa3ec65b71d7b00947ed06e6898d26f8f9591aca35caf1e10fdc3f",
);
/// 2025-01-15T03:30:10Z, ten seconds after the payloads were signed, in milliseconds.
const STARTED_AT: i64 = 1_736_911_810_000;
/// The server's study day at [`STARTED_AT`]: 2025-01-14, as an epoch day.
const TODAY: i64 = 20_102;
/// The streak view.
const STREAK_PATH: &str = "/api/streak";
/// The governor view.
const GOVERNOR_PATH: &str = "/api/governor";
/// The most a test reads of a body.
const BODY_READ_LIMIT: usize = 256 * 1024;

/// Prints how many items a check examined and refuses zero (the tdd pack's examined contract).
fn examined<T>(what: &str, items: Vec<T>) -> Vec<T> {
    println!("examined {} {what}", items.len());
    assert!(
        !items.is_empty(),
        "examined 0 {what}: the population is empty, so nothing was judged"
    );
    items
}

/// Writes the two tracks' rows and the governor's state the views read.
async fn seed(db: &Db) {
    let mut write = db.write().await.expect("a write");
    for (track, current, longest, freezes) in [("language", 5, 9, 2), ("law", 2, 4, 0)] {
        sqlx::query(
            "INSERT INTO streak_state \
             (track, current_days, longest_days, freezes, last_study_day, comeback_armed, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 1000)",
        )
        .bind(track)
        .bind(current)
        .bind(longest)
        .bind(freezes)
        .bind(TODAY - 1)
        .execute(&mut *write)
        .await
        .expect("the synthetic streak row is written");
    }
    write.commit().await.expect("the commit");
}

/// The API as the daemon builds it, over a migrated database holding the two tracks, for the
/// synthetic owner and bot, on a manual clock.
async fn app(scratch: &TempDir) -> (Db, Router) {
    app_with(scratch, true).await
}

/// The same app; when `open` is false the readiness never learns of the database.
async fn app_with(scratch: &TempDir, open: bool) -> (Db, Router) {
    let db = Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    seed(&db).await;
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        Owner::new(TelegramUserId::new(OWNER)),
        Freshness::default(),
    );
    let access = OwnerAccess::new(gate, clock, StudyDayRule::default());
    let readiness = Readiness::new();
    if open {
        readiness.database_opened(db.clone());
    }
    (db, router(ApiState::new(readiness).with_owner(access)))
}

/// An answer: its status, its headers and its body.
struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

impl Answer {
    /// The body as JSON.
    fn json(&self) -> Value {
        serde_json::from_str(&self.body).expect("a JSON body")
    }
}

/// `method` on `path` with `headers` and `body`, through the whole app.
async fn send(
    app: &Router,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: String,
) -> Answer {
    let mut request = Request::builder().method(method).uri(path);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = app
        .clone()
        .oneshot(
            request
                .body(Body::from(body))
                .expect("a well-formed request"),
        )
        .await
        .expect("the router is infallible");
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), BODY_READ_LIMIT)
        .await
        .expect("a readable body");
    Answer {
        status,
        headers,
        body: String::from_utf8(bytes.to_vec()).expect("a UTF-8 body"),
    }
}

/// A same-origin JSON handshake with `init_data`.
async fn handshake(app: &Router, init_data: &str) -> Answer {
    send(
        app,
        "POST",
        "/api/session",
        &[
            ("content-type", "application/json"),
            ("sec-fetch-site", "same-origin"),
        ],
        json!({ "init_data": init_data }).to_string(),
    )
    .await
}

/// The cookie a browser sends back after `answer`: its `name=value`.
fn cookie_of(answer: &Answer) -> String {
    let value = answer
        .headers
        .get(SET_COOKIE)
        .expect("a Set-Cookie")
        .to_str()
        .expect("a text header");
    value
        .split(';')
        .next()
        .expect("a name and value")
        .trim()
        .to_owned()
}

/// `GET path` carrying `cookie`, or no cookie.
async fn get(app: &Router, path: &str, cookie: Option<&str>) -> Answer {
    let headers: Vec<(&str, &str)> = cookie
        .map(|cookie| ("cookie", cookie))
        .into_iter()
        .collect();
    send(app, "GET", path, &headers, String::new()).await
}

#[tokio::test]
async fn the_streak_routes_answer_only_the_owner() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (db, app) = app(&scratch).await;
    let paths = examined("streak route(s)", vec![STREAK_PATH, GOVERNOR_PATH]);

    // No session, a cookie that names none, and a session the owner logged out of: 401, and the
    // body names the reason alone.
    let owner = cookie_of(&handshake(&app, OWNER_PAYLOAD).await);
    let ended = cookie_of(&handshake(&app, OWNER_PAYLOAD).await);
    let logout = send(
        &app,
        "DELETE",
        "/api/session",
        &[
            ("content-type", "application/json"),
            ("sec-fetch-site", "same-origin"),
            ("cookie", &ended),
        ],
        String::new(),
    )
    .await;
    assert_eq!(logout.status, StatusCode::NO_CONTENT);
    let unknown = format!("__Host-deckstreak_session={}", "0".repeat(64));
    for path in &paths {
        for cookie in [None, Some(unknown.as_str()), Some(ended.as_str())] {
            let refused = get(&app, path, cookie).await;
            assert_eq!(
                refused.status,
                StatusCode::UNAUTHORIZED,
                "{path} {cookie:?}"
            );
            assert_eq!(refused.json(), json!({"reason": "no_session"}), "{path}");
        }
    }

    // Another user, and the owner's fields forged for another bot, never get a session.
    for (payload, status) in [
        (STRANGER_PAYLOAD, StatusCode::FORBIDDEN),
        (FORGED_PAYLOAD, StatusCode::UNAUTHORIZED),
    ] {
        let refused = handshake(&app, payload).await;
        assert_eq!(refused.status, status, "{}", refused.body);
        assert!(
            refused.headers.get(SET_COOKIE).is_none(),
            "no session opens"
        );
    }

    // The owner's live session reads both tracks: the language track's freezes and their cap, the
    // law track's none, and what is at stake for the day still open.
    let streak = get(&app, STREAK_PATH, Some(&owner)).await;
    assert_eq!(streak.status, StatusCode::OK, "{}", streak.body);
    assert_eq!(
        streak.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes),
        Some(&b"application/json"[..])
    );
    let view = streak.json();
    assert_eq!(view["study_day"], "2025-01-14");
    assert_eq!(view["language"]["current"], 5);
    assert_eq!(view["language"]["longest"], 9);
    assert_eq!(view["language"]["freezes"], 2);
    assert_eq!(view["language"]["freeze_cap"], 3);
    assert_eq!(view["law"]["current"], 2);
    assert_eq!(view["law"]["longest"], 4);
    assert!(
        view["law"].get("freezes").is_none(),
        "the law track holds no freezes"
    );
    assert_eq!(view["at_stake"]["language"], "freeze");
    assert_eq!(view["at_stake"]["law"], "break");

    // The governor reads its verdict, its strength and, when disarmed, why.
    let governor = get(&app, GOVERNOR_PATH, Some(&owner)).await;
    assert_eq!(governor.status, StatusCode::OK, "{}", governor.body);
    let verdict = governor.json();
    assert_eq!(verdict["verdict"], "armed");
    assert!(verdict.get("strength").is_some(), "the strength is served");
    db.close().await;
}

/// A rule (A37): a streak route answers 503 `database_not_open` while the database is not open and
/// 500 `streak_unreadable` when its table cannot be read, each as JSON, for both routes.
#[tokio::test]
async fn the_streak_routes_name_why_they_cannot_answer() {
    let mut refusals = 0_u32;
    for (path, table) in [
        (STREAK_PATH, "streak_state"),
        (GOVERNOR_PATH, "governor_state"),
    ] {
        let scratch = tempfile::tempdir().expect("a temporary directory");
        let (_db, closed) = app_with(&scratch, false).await;
        let owner = cookie_of(&handshake(&closed, OWNER_PAYLOAD).await);
        let refused = get(&closed, path, Some(&owner)).await;
        assert_eq!(refused.status, StatusCode::SERVICE_UNAVAILABLE, "{path}");
        assert_eq!(
            refused.json(),
            json!({"reason": "database_not_open"}),
            "{path}"
        );
        assert_eq!(
            refused.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes),
            Some(&b"application/json"[..])
        );

        let scratch = tempfile::tempdir().expect("a temporary directory");
        let (db, open) = app(&scratch).await;
        let owner = cookie_of(&handshake(&open, OWNER_PAYLOAD).await);
        let mut write = db.write().await.expect("a write");
        let rename = match table {
            "streak_state" => "ALTER TABLE streak_state RENAME TO gone_streak_state",
            _ => "ALTER TABLE governor_state RENAME TO gone_governor_state",
        };
        sqlx::query(rename)
            .execute(&mut *write)
            .await
            .expect("the table is renamed away");
        write.commit().await.expect("the commit");
        let broken = get(&open, path, Some(&owner)).await;
        assert_eq!(broken.status, StatusCode::INTERNAL_SERVER_ERROR, "{path}");
        assert_eq!(
            broken.json(),
            json!({"reason": "streak_unreadable"}),
            "{path}"
        );
        assert_eq!(
            broken.headers.get(CONTENT_TYPE).map(HeaderValue::as_bytes),
            Some(&b"application/json"[..])
        );
        refusals += 2;
    }
    println!("streak-route refusals: {refusals}");
    assert_eq!(refusals, 4);
}

/// The heat thresholds of R5: a track's heat is how many of them its run has reached.
const HEAT_THRESHOLDS: [u32; 5] = [1, 7, 30, 100, 365];
/// The runs the population walks, one at each heat threshold and one below the first.
const RUNS: [u32; 6] = [0, 1, 7, 30, 100, 365];
/// The lapse anchors the population stores, with their dates as the routes serve them.
const ANCHORS: [(i64, &str); 3] = [
    (TODAY - 4, "2025-01-10"),
    (TODAY - 5, "2025-01-09"),
    (TODAY - 6, "2025-01-08"),
];

/// One track's stored row, as the population writes it.
#[derive(Clone, Copy, Debug)]
struct TrackRow {
    current: u32,
    longest: u32,
    freezes: u32,
    studied_today: bool,
}

/// One generated fixture: every input the two routes read.
#[derive(Clone, Copy, Debug)]
struct Fixture {
    language: TrackRow,
    law: TrackRow,
    lapse: bool,
    standby: bool,
    anchor: usize,
    strength: f64,
}

/// The population, built so that every pair of values a route serves differs in some fixture: each
/// track takes every run, its best is above its run by an amount the other track's run moves, the
/// freezes, the day studied, the lapse, the standby, the anchor and the strength each cycle on
/// their own period.
fn population() -> Vec<Fixture> {
    let mut fixtures = Vec::new();
    for (i, language) in RUNS.into_iter().enumerate() {
        for (j, law) in RUNS.into_iter().enumerate() {
            let k = 6 * i + j;
            let (lapse, standby) =
                [(false, false), (false, true), (true, false), (true, true)][k % 4];
            fixtures.push(Fixture {
                language: TrackRow {
                    current: language,
                    longest: language + 1 + u32::try_from(j).expect("a small index"),
                    freezes: u32::try_from(k % 4).expect("a small index"),
                    studied_today: k % 3 == 0,
                },
                law: TrackRow {
                    current: law,
                    longest: law + 2 + u32::try_from(i).expect("a small index"),
                    freezes: 0,
                    studied_today: k % 3 == 1,
                },
                lapse,
                standby,
                anchor: k % 3,
                strength: f64::from(u32::try_from(k % 7).expect("a small index")) / 8.0,
            });
        }
    }
    fixtures
}

/// The tier R5 names for a run.
fn tier(run: u32) -> u32 {
    let reached = HEAT_THRESHOLDS
        .iter()
        .filter(|threshold| run >= **threshold)
        .count();
    u32::try_from(reached).expect("at most five thresholds")
}

/// What R20 says a missed day costs a track: nothing when the day already has a study review or
/// there is no run, a freeze when one is held, else the run.
fn at_stake(track: &TrackRow) -> &'static str {
    if track.studied_today || track.current == 0 {
        "none"
    } else if track.freezes > 0 {
        "freeze"
    } else {
        "break"
    }
}

/// Every value R20 and R21 say each route serves for `fixture`, leaf by leaf, from the SPEC's words.
fn oracle(fixture: &Fixture) -> [Vec<(String, Value)>; 2] {
    let (language, law) = (&fixture.language, &fixture.law);
    let (_, anchor_date) = ANCHORS[fixture.anchor];
    let streak = vec![
        ("study_day", json!("2025-01-14")),
        ("language.current", json!(language.current)),
        ("language.longest", json!(language.longest)),
        ("language.heat", json!(tier(language.current))),
        ("language.freezes", json!(language.freezes)),
        ("language.freeze_cap", json!(3)),
        ("law.current", json!(law.current)),
        ("law.longest", json!(law.longest)),
        ("law.heat", json!(tier(law.current))),
        ("at_stake.language", json!(at_stake(language))),
        ("at_stake.law", json!(at_stake(law))),
    ];
    let verdict = if fixture.lapse {
        "lapse"
    } else if fixture.standby {
        "standby"
    } else {
        "armed"
    };
    let governor = vec![
        ("verdict", json!(verdict)),
        ("strength", json!(fixture.strength)),
        ("standby", json!(fixture.standby)),
        ("lapse", json!(fixture.lapse)),
        (
            "lapse_since",
            if fixture.lapse {
                json!(anchor_date)
            } else {
                Value::Null
            },
        ),
        (
            "relight_cards",
            if fixture.lapse { json!(3) } else { Value::Null },
        ),
    ];
    [streak, governor].map(|leaves| {
        leaves
            .into_iter()
            .map(|(leaf, value)| (leaf.to_owned(), value))
            .collect()
    })
}

/// Every leaf of `value`, named by its path.
fn leaves(prefix: &str, value: &Value, out: &mut Vec<(String, Value)>) {
    if let Value::Object(fields) = value {
        for (key, field) in fields {
            // The calendar's days are read where their markers are decided, by the daemon's
            // calendar test (SPEC-076 A58); this population separates the scalar values.
            if prefix.is_empty() && key == "calendar" {
                continue;
            }
            let path = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            leaves(&path, field, out);
        }
    } else {
        out.push((prefix.to_owned(), value.clone()));
    }
}

/// Writes `fixture`'s rows: both tracks, the governor's row and the latest strength.
async fn store(db: &Db, fixture: &Fixture) {
    let mut write = db.write().await.expect("a write");
    for (track, row) in [("language", fixture.language), ("law", fixture.law)] {
        let last = if row.studied_today { TODAY } else { TODAY - 1 };
        sqlx::query(
            "UPDATE streak_state SET current_days = ?2, longest_days = ?3, freezes = ?4, \
             last_study_day = ?5 WHERE track = ?1",
        )
        .bind(track)
        .bind(i64::from(row.current))
        .bind(i64::from(row.longest))
        .bind(i64::from(row.freezes))
        .bind(last)
        .execute(&mut *write)
        .await
        .expect("a streak row");
    }
    let (anchor, _) = ANCHORS[fixture.anchor];
    sqlx::query("UPDATE governor_state SET lapse_since = ?1, standby = ?2")
        .bind(fixture.lapse.then_some(anchor))
        .bind(i64::from(fixture.standby))
        .execute(&mut *write)
        .await
        .expect("the governor row");
    sqlx::query(
        "INSERT INTO habit_strength (study_day, strength, created_at) VALUES (?1, ?2, 1000) \
         ON CONFLICT (study_day) DO UPDATE SET strength = excluded.strength",
    )
    .bind(TODAY - 1)
    .bind(fixture.strength)
    .execute(&mut *write)
    .await
    .expect("the strength row");
    write.commit().await.expect("the commit");
}

/// A rule (A47, the served-pairs class closed by construction): every value `GET /api/streak` and
/// `GET /api/governor` serve is read from a generated population in which each pair of served
/// values differs in some fixture, against the SPEC's words for each value. The population's own
/// check refuses a pair it does not separate, and a served value the words do not name, so a route
/// that serves one value in another's place is red.
#[tokio::test]
async fn every_served_value_is_read_where_each_pair_of_served_values_differs() {
    let fixtures = examined("served-value fixture(s)", population());
    let expected: Vec<[Vec<(String, Value)>; 2]> = fixtures.iter().map(oracle).collect();

    // The population separates every pair of values each route serves.
    let mut separated = 0_u32;
    let mut unseparated = Vec::new();
    for route in 0..2 {
        let names: Vec<&String> = expected[0][route].iter().map(|(leaf, _)| leaf).collect();
        for (a, first) in names.iter().enumerate() {
            for second in names.iter().skip(a + 1) {
                let differs = expected.iter().any(|leaves| {
                    let value = |name: &String| {
                        leaves[route]
                            .iter()
                            .find(|(leaf, _)| leaf == name)
                            .map(|(_, value)| value.clone())
                    };
                    value(first) != value(second)
                });
                if differs {
                    separated += 1;
                } else {
                    unseparated.push(format!("{first} and {second}"));
                }
            }
        }
    }
    assert!(
        unseparated.is_empty(),
        "the population never separates {unseparated:?}"
    );

    // Each fixture's served values are the words' values, and no route serves a value the words do
    // not name.
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let (db, app) = app(&scratch).await;
    let owner = cookie_of(&handshake(&app, OWNER_PAYLOAD).await);
    for (fixture, want) in fixtures.iter().zip(&expected) {
        store(&db, fixture).await;
        for (route, path) in [STREAK_PATH, GOVERNOR_PATH].into_iter().enumerate() {
            let answer = get(&app, path, Some(&owner)).await;
            assert_eq!(answer.status, StatusCode::OK, "{path}: {}", answer.body);
            let mut served = Vec::new();
            leaves("", &answer.json(), &mut served);
            served.sort_by(|a, b| a.0.cmp(&b.0));
            let mut words = want[route].clone();
            words.sort_by(|a, b| a.0.cmp(&b.0));
            assert_eq!(served, words, "{path} for {fixture:?}");
        }
    }
    println!(
        "served-value population: {} fixture(s), {separated} served pair(s) separated",
        fixtures.len()
    );
    assert_eq!(separated, 55 + 15);
    db.close().await;
}
