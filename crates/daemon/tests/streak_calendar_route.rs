//! The streak calendar through `GET /api/streak` over the real store (SPEC-076 A58, A59, A60 at the
//! route, A63): after the fold has settled a constructed population, each served day carries
//! exactly its own markers, every freeze the fold spent has its covered day served as `freeze`, a
//! track is settled on exactly its study days, and over the parity golden's cases each track serves
//! the predecessor's window and the two tracks' study days together are the predecessor's studied
//! days. Every review, card and instant is synthetic.
//!
//! The launch payloads were signed by Python's standard `hmac` and `hashlib` for the synthetic bot
//! token below, each dated 03:30:00Z on the day after the study day it serves; every clock is a
//! `ManualClock` started ten seconds later, so the payload is fresh and the study day is still the
//! one before until 04:00.

#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::header::SET_COOKIE;
use axum::http::{Request, StatusCode};
use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_api::{ApiState, OwnerAccess, Readiness, router};
use deck_streak_coordination::recompute::FoldInput;
use deck_streak_daemon::wiring::recompute_fold;
use deck_streak_identity::{Freshness, Owner, OwnerGate, WebAppKey};
use deck_streak_ingest::reader::{Card, CollectionData, Review};
use deck_streak_kernel::{
    Db, ManualClock, StudyDay, StudyDayRule, TelegramUserId, Track, UtcMillis,
};
use deck_streak_progression::settled_days;
use deck_streak_streaks::calendar;
use serde_json::Value;
use sqlx::Row;
use tempfile::TempDir;
use tower::ServiceExt;

const BOT_TOKEN: &str = "synthetic-webapp-signing-token";
const OWNER: i64 = 4242;
const OWNER_PAYLOAD: &str = concat!(
    "auth_date=1736911800&query_id=synthetic-query",
    "&user=%7B%22id%22%3A4242%2C%22first_name%22%3A%22Ada%22%2C",
    "%22username%22%3A%22synthetic_owner%22%7D",
    "&hash=79502d49032542c5030e80b666d03f5af53e053f97d387806adb0aa843ddeb2d",
);
/// The parity golden's served days, each with the hash of the owner's launch payload dated 03:30:00Z
/// on the day after it (`auth_date` is that instant in seconds).
const SIGNED: [(i64, &str); 7] = [
    (
        20_000,
        "b773322caf93b9f1285ef1c73a01531a68c3b8ca56f99dc02e81bd5048079e3c",
    ),
    (
        20_001,
        "0e4acb462a8723e8f6673b16758e90733a71bcaefdf762910bf8a454b15dc342",
    ),
    (
        20_002,
        "2babe0a5d98743c106fda512434403074bdd9a51e752bc23646dd08940bebc84",
    ),
    (
        20_003,
        "8b16fd6f379e2c15fed102581150cf729853129273fa2e3ec01cab240ee91f94",
    ),
    (
        20_004,
        "1be053aa7e706999fd5a4a97290c13b0f520aab26a648ff2c7bc789da7f1d9ae",
    ),
    (
        20_005,
        "dc99af7767d0819320eef3d928914412c49bc2cbc438250227df3060ed6027b4",
    ),
    (
        20_006,
        "7d34c7e0944ac565b9fab5e1b13ac42a3b24f8f5c44a1c603d994dd17c35858c",
    ),
];
/// 2025-01-14, the study day [`OWNER_PAYLOAD`] serves: its clock starts at 2025-01-15T03:30:10Z.
const TODAY: i64 = 20_102;
/// The predecessor's window served on [`TODAY`], a Tuesday: from Monday 19919, the Monday on or
/// before the day 181 days back, through it (`charts.streak_calendar`; SPEC-076 section 27).
const TODAYS_FIRST: i64 = 19_919;
const TODAYS_LENGTH: usize = 184;
const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;
/// Ten seconds past 03:30:00Z, where every payload is dated.
const SIGNED_AT_MS: i64 = 3 * HOUR_MS + 30 * 60_000 + 10_000;

const fn at(day: i64, hour: i64) -> i64 {
    day * DAY_MS + hour * HOUR_MS
}

const fn review(day: i64, card: i64) -> Review {
    Review {
        id: at(day, 9) + card,
        card_id: card,
        ease: 3,
        interval: 25,
        last_interval: 15,
        factor: 2500,
        taken_ms: 9_000,
        kind: 1,
    }
}

const fn card(id: i64, track: Track) -> Card {
    Card {
        id,
        note_id: id,
        deck_id: 1,
        original_deck_id: 0,
        queue: 2,
        kind: 2,
        due: 1_090,
        interval: 30,
        factor: 2500,
        reps: 3,
        lapses: 0,
        track,
        course: None,
        tier: None,
    }
}

/// The language track's study days, by construction: a run long enough to earn a freeze, one real
/// miss the freeze covers, a second run, two misses that break it, a third run.
fn language_days(today_studied: bool) -> BTreeSet<i64> {
    let mut days: BTreeSet<i64> = BTreeSet::new();
    days.extend(TODAY - 30..=TODAY - 23);
    days.extend(TODAY - 21..=TODAY - 15);
    days.extend(TODAY - 12..=TODAY - 10);
    days.extend(TODAY - 5..=TODAY - 4);
    if today_studied {
        days.insert(TODAY);
    }
    days
}

/// The law track's study days, by construction: they overlap the language days on some days and
/// stand alone on others, so a track filter that is dropped changes the answer.
fn law_days(today_studied: bool) -> BTreeSet<i64> {
    let mut days: BTreeSet<i64> = BTreeSet::new();
    days.extend([TODAY - 30, TODAY - 29, TODAY - 27]);
    days.extend(TODAY - 22..=TODAY - 20);
    days.extend([TODAY - 14, TODAY - 3]);
    if today_studied {
        days.insert(TODAY - 1);
        days.insert(TODAY);
    }
    days
}

fn collection(language: &BTreeSet<i64>, law: &BTreeSet<i64>) -> CollectionData {
    let reviews = language
        .iter()
        .map(|day| review(*day, 1))
        .chain(law.iter().map(|day| review(*day, 2)))
        .collect();
    CollectionData {
        reviews,
        cards: vec![card(1, Track::Language), card(2, Track::Law)],
        created_at: UtcMillis::from_epoch_millis(at(TODAY - 1_000, 12)),
        deck_names: BTreeMap::from([(1, "Synthetic".to_owned())]),
    }
}

/// The composed API over a database the fold has settled for `language` and `law`.
async fn settled(scratch: &TempDir, language: &BTreeSet<i64>, law: &BTreeSet<i64>) -> (Db, Router) {
    settled_on(scratch, language, law, TODAY).await
}

/// The same, with the fold run at noon on `today` and the owner's clock on `today`'s study day.
async fn settled_on(
    scratch: &TempDir,
    language: &BTreeSet<i64>,
    law: &BTreeSet<i64>,
    today: i64,
) -> (Db, Router) {
    let db = Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let data = collection(language, law);
    let fold = recompute_fold(AnalyticsSettings::default()).expect("the fold registers");
    fold.run(
        &db,
        &FoldInput {
            data: &data,
            rule: StudyDayRule::default(),
            now: UtcMillis::from_epoch_millis(at(today, 12)),
            synced_in: Some(StudyDay::from_epoch_day(today)),
            courses_digest: Some("0123456789abcdef"),
        },
    )
    .await
    .expect("the fold runs");
    let started = (today + 1) * DAY_MS + SIGNED_AT_MS;
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(started)));
    let gate = OwnerGate::new(
        WebAppKey::from_bot_token(BOT_TOKEN),
        Owner::new(TelegramUserId::new(OWNER)),
        Freshness::default(),
    );
    let readiness = Readiness::new();
    readiness.database_opened(db.clone());
    let access = OwnerAccess::new(gate, clock, StudyDayRule::default());
    (db, router(ApiState::new(readiness).with_owner(access)))
}

async fn owner_cookie(app: &Router) -> String {
    owner_cookie_from(app, OWNER_PAYLOAD).await
}

/// The owner's launch payload for the study day `today`, from [`SIGNED`].
fn payload_on(today: i64) -> String {
    let (_, hash) = SIGNED
        .iter()
        .find(|(day, _)| *day == today)
        .expect("a payload is signed for every served day of the golden");
    let auth_date = (today + 1) * DAY_MS / 1_000 + 3 * 3_600 + 30 * 60;
    format!(
        "auth_date={auth_date}&query_id=synthetic-query\
         &user=%7B%22id%22%3A4242%2C%22first_name%22%3A%22Ada%22%2C\
         %22username%22%3A%22synthetic_owner%22%7D&hash={hash}"
    )
}

async fn owner_cookie_from(app: &Router, payload: &str) -> String {
    let request = Request::builder()
        .method("POST")
        .uri("/api/session")
        .header("content-type", "application/json")
        .header("sec-fetch-site", "same-origin")
        .body(Body::from(
            serde_json::json!({ "init_data": payload }).to_string(),
        ))
        .expect("a request");
    let response = app.clone().oneshot(request).await.expect("infallible");
    response
        .headers()
        .get(SET_COOKIE)
        .expect("a Set-Cookie")
        .to_str()
        .expect("text")
        .split(';')
        .next()
        .expect("a pair")
        .trim()
        .to_owned()
}

async fn streak_of(app: &Router, cookie: Option<&str>) -> (StatusCode, Value) {
    let mut request = Request::builder().method("GET").uri("/api/streak");
    if let Some(cookie) = cookie {
        request = request.header("cookie", cookie);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).expect("a request"))
        .await
        .expect("infallible");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1 << 20)
        .await
        .expect("a body");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn day_set(days: &BTreeSet<i64>) -> BTreeSet<StudyDay> {
    days.iter()
        .map(|day| StudyDay::from_epoch_day(*day))
        .collect()
}

/// A served calendar as `(day, studied, markers)`.
fn served(body: &Value, track: &str) -> Vec<(String, bool, Vec<String>)> {
    body["calendar"][track]
        .as_array()
        .expect("the calendar is an array")
        .iter()
        .map(|day| {
            (
                day["day"].as_str().expect("a day").to_owned(),
                day["studied"].as_bool().expect("a flag"),
                day["markers"]
                    .as_array()
                    .expect("markers")
                    .iter()
                    .map(|marker| marker.as_str().expect("a marker").to_owned())
                    .collect(),
            )
        })
        .collect()
}

/// The pure function's calendar for the same days, in the route's shape.
fn expected(days: &[CalendarRow]) -> Vec<(String, bool, Vec<String>)> {
    days.iter()
        .map(|day| (day.0.clone(), day.1, day.2.clone()))
        .collect()
}

type CalendarRow = (String, bool, Vec<String>);

fn pure(track: &str, days: &BTreeSet<i64>) -> Vec<CalendarRow> {
    let served_day = StudyDay::from_epoch_day(TODAY);
    let none = BTreeSet::new();
    let window = if track == "language" {
        calendar::language(&day_set(days), &none, served_day)
    } else {
        calendar::law(&day_set(days), &none, served_day)
    };
    window
        .into_iter()
        .map(|day| {
            (
                day.day.to_string(),
                day.studied,
                day.markers.iter().map(|m| m.as_str().to_owned()).collect(),
            )
        })
        .collect()
}

#[tokio::test]
async fn the_route_serves_each_days_markers_after_the_fold() {
    let mut examined_days = 0_usize;
    let mut spent_freezes = 0_usize;
    for today_studied in [false, true] {
        let scratch = TempDir::new().expect("a scratch directory");
        let language = language_days(today_studied);
        let law = law_days(today_studied);
        let (db, app) = settled(&scratch, &language, &law).await;
        let cookie = owner_cookie(&app).await;
        let (status, body) = streak_of(&app, Some(&cookie)).await;
        assert_eq!(status, StatusCode::OK, "the owner reads the calendar");

        for (track, days) in [("language", &language), ("law", &law)] {
            let got = served(&body, track);
            assert_eq!(got.len(), TODAYS_LENGTH, "{track}: the window's length");
            assert_eq!(
                got.first().map(|day| day.0.clone()),
                Some(StudyDay::from_epoch_day(TODAYS_FIRST).to_string()),
                "{track}: the window opens on the predecessor's Monday"
            );
            assert_eq!(
                got.last().map(|day| day.0.clone()),
                Some(StudyDay::from_epoch_day(TODAY).to_string()),
                "{track}: the served day is the window's last"
            );
            assert_eq!(
                got.last().map(|day| day.1),
                Some(today_studied),
                "{track}: the open day is studied exactly when the population studied it"
            );
            assert_eq!(
                got,
                expected(&pure(track, days)),
                "{track}: the route's days"
            );
            examined_days += got.len();
        }

        // Every freeze the fold spent on a return day in the window has its covered day, the day
        // after the previous study day, served as a freeze, and no other day is.
        let mut write = db.write().await.expect("a write");
        let events: Vec<i64> = sqlx::query(
            "SELECT study_day FROM freeze_events WHERE reason = 'consumed' ORDER BY study_day",
        )
        .fetch_all(&mut *write)
        .await
        .expect("the events read")
        .into_iter()
        .map(|row| row.get(0))
        .collect();
        drop(write);
        let first = TODAYS_FIRST;
        let covered: BTreeSet<String> = events
            .iter()
            .filter_map(|ret| {
                let previous = language.range(..*ret).next_back().copied()?;
                let covered = previous + 1;
                (covered >= first).then(|| StudyDay::from_epoch_day(covered).to_string())
            })
            .collect();
        spent_freezes += events.len();
        let frozen: BTreeSet<String> = served(&body, "language")
            .into_iter()
            .filter(|day| day.2.iter().any(|marker| marker == "freeze"))
            .map(|day| day.0)
            .collect();
        assert_eq!(
            frozen, covered,
            "the served freezes are the spent freezes' covered days"
        );
        let law_frozen = served(&body, "law")
            .into_iter()
            .any(|day| day.2.iter().any(|marker| marker == "freeze"));
        assert!(!law_frozen, "the law track serves no freeze");
    }
    println!("examined {examined_days} served calendar day(s), {spent_freezes} spent freeze(s)");
    assert!(examined_days > 0, "examined 0 days");
    assert!(
        spent_freezes > 0,
        "no freeze was spent, so the freeze marker was never judged"
    );

    // R20: a caller with no session reads nothing.
    let scratch = TempDir::new().expect("a scratch directory");
    let (_db, app) = settled(&scratch, &language_days(false), &law_days(false)).await;
    let (status, body) = streak_of(&app, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body.get("calendar").is_none(), "no data without a session");
}

#[tokio::test]
async fn a_track_is_settled_on_exactly_its_study_days() {
    let mut examined_days = 0_usize;
    for today_studied in [false, true] {
        let scratch = TempDir::new().expect("a scratch directory");
        let language = language_days(today_studied);
        let law = law_days(today_studied);
        let (db, _app) = settled(&scratch, &language, &law).await;
        let mut write = db.write().await.expect("a write");
        for (source, track, days) in [
            ("reviews", "language", &language),
            ("reviews_law", "law", &law),
        ] {
            let settled_days: BTreeSet<i64> = sqlx::query(
                "SELECT study_day FROM xp_settlement \
                 WHERE source = ?1 AND track = ?2 AND amount > 0",
            )
            .bind(source)
            .bind(track)
            .fetch_all(&mut *write)
            .await
            .expect("the settlement reads")
            .into_iter()
            .map(|row| row.get(0))
            .collect();
            assert_eq!(
                &settled_days, days,
                "{track}: settled days equal the study days"
            );
            examined_days += days.len();
        }
        // The reader's two conjuncts, judged by planted rows the fold never writes: a settled row
        // of amount 0 is no study day, and another track's row under the same source is not this
        // track's. The fold writes neither, so only a plant can separate the conjuncts.
        for (day, source, track, amount) in [
            (TODAY - 100, "reviews", "language", 0),
            (TODAY - 101, "reviews", "law", 7),
            (TODAY - 102, "reviews_law", "language", 7),
            (TODAY - 103, "reviews_law", "law", 0),
        ] {
            sqlx::query(
                "INSERT INTO xp_settlement (study_day, source, track, amount, closed, created_at) \
                 VALUES (?1, ?2, ?3, ?4, 1, 1000)",
            )
            .bind(day)
            .bind(source)
            .bind(track)
            .bind(amount)
            .execute(&mut *write)
            .await
            .expect("the planted row is written");
            examined_days += 1;
        }
        for (source, track, kind, days) in [
            ("reviews", "language", Track::Language, &language),
            ("reviews_law", "law", Track::Law, &law),
        ] {
            let read: BTreeSet<i64> = settled_days(&mut write, source, kind)
                .await
                .expect("the settled days read")
                .into_iter()
                .map(StudyDay::epoch_day)
                .collect();
            assert_eq!(
                &read, days,
                "{track}: the reader keeps only this track's days above zero"
            );
        }
        // The two tracks differ, so a dropped track filter would be seen.
        assert_ne!(language, law, "the population separates the tracks");
        assert!(
            language.difference(&law).next().is_some()
                && law.difference(&language).next().is_some(),
            "each track has a day the other lacks"
        );
    }
    println!("examined {examined_days} study day(s) against their settlements");
    assert!(examined_days > 0, "examined 0 days");
}

/// One golden case as the predecessor served it: its served day, its window in order and its
/// studied days, every day an epoch day number.
struct PredecessorCase {
    today: i64,
    window: Vec<i64>,
    studied: BTreeSet<i64>,
}

/// One golden case as the route served it after the fold: each track's `(day, studied)` in order.
#[derive(Clone)]
struct ServedCase {
    today: i64,
    language: Vec<(String, bool)>,
    law: Vec<(String, bool)>,
}

/// The union judge's counts (the verifier's `judge_parity.py` rule).
#[derive(Debug, Default, PartialEq, Eq)]
struct Judged {
    cases: usize,
    days: usize,
    window_mismatches: usize,
    unnamed: usize,
    named: usize,
}

impl Judged {
    const fn refuses(&self) -> bool {
        self.cases == 0 || self.window_mismatches > 0 || self.unnamed > 0
    }
}

/// The differences ADR-302 names, `(served day, day, reason)`. None arises in this population, so
/// the list is empty: no review is deleted after its day closed (the predecessor would recount the
/// day unstudied, while a closed day's settlement is only ever raised), and the fold settles the
/// open day before the route reads it (the predecessor's rollup would lag its own writer the same
/// way). A difference absent from this list fails the judge.
const NAMED: &[(i64, i64, &str)] = &[];

/// Each track's served days must be the predecessor's window in order, and the days either track
/// served as studied must be the predecessor's studied days; a difference is counted named when
/// `named` gives its reason, and unnamed otherwise.
fn judge(golden: &[PredecessorCase], served: &[ServedCase], named: &[(i64, i64, &str)]) -> Judged {
    let mut judged = Judged::default();
    for (case, got) in golden.iter().zip(served) {
        assert_eq!(case.today, got.today, "the served case is the golden's");
        judged.cases += 1;
        let window: Vec<String> = case
            .window
            .iter()
            .map(|day| StudyDay::from_epoch_day(*day).to_string())
            .collect();
        let mut union: BTreeSet<String> = BTreeSet::new();
        for track in [&got.language, &got.law] {
            judged.days += track.len();
            let days: Vec<String> = track.iter().map(|day| day.0.clone()).collect();
            if days != window {
                judged.window_mismatches += 1;
            }
            union.extend(track.iter().filter(|day| day.1).map(|day| day.0.clone()));
        }
        let studied: BTreeSet<String> = case
            .studied
            .iter()
            .map(|day| StudyDay::from_epoch_day(*day).to_string())
            .collect();
        for day in union.symmetric_difference(&studied) {
            let is_named = named.iter().any(|(today, named_day, _)| {
                *today == case.today && StudyDay::from_epoch_day(*named_day).to_string() == *day
            });
            if is_named {
                judged.named += 1;
            } else {
                judged.unnamed += 1;
            }
        }
    }
    judged
}

fn epoch_days(value: &Value) -> Vec<i64> {
    value
        .as_array()
        .expect("a list of days")
        .iter()
        .map(|day| day.as_i64().expect("an epoch day number"))
        .collect()
}

fn served_flags(body: &Value, track: &str) -> Vec<(String, bool)> {
    served(body, track)
        .into_iter()
        .map(|day| (day.0, day.1))
        .collect()
}

#[tokio::test]
async fn the_route_serves_the_predecessors_window_and_its_studied_days() {
    let golden = golden::read(&golden::committed("streak_calendar"))
        .unwrap_or_else(|refusal| panic!("{refusal}"));
    let mut predecessor = Vec::new();
    let mut served_cases = Vec::new();
    for case in &golden.cases {
        let today = case.input["today"].as_i64().expect("a served day");
        // A rollup with study reviews is a study day; the store at the served day holds none dated
        // after it. The days are split over the tracks by a fixed rule, so each track holds days
        // the other lacks and some days are both tracks'.
        let mut language = BTreeSet::new();
        let mut law = BTreeSet::new();
        for rollup in case.input["rollups"].as_array().expect("rollups") {
            let day = rollup["day"].as_i64().expect("an epoch day");
            let reviews = rollup["reviews"].as_i64().expect("a review count");
            if reviews > 0 && day <= today {
                match day.rem_euclid(3) {
                    0 => {
                        language.insert(day);
                    }
                    1 => {
                        law.insert(day);
                    }
                    _ => {
                        language.insert(day);
                        law.insert(day);
                    }
                }
            }
        }
        let scratch = TempDir::new().expect("a scratch directory");
        let (_db, app) = settled_on(&scratch, &language, &law, today).await;
        let cookie = owner_cookie_from(&app, &payload_on(today)).await;
        let (status, body) = streak_of(&app, Some(&cookie)).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "the owner reads the calendar on {today}"
        );
        served_cases.push(ServedCase {
            today,
            language: served_flags(&body, "language"),
            law: served_flags(&body, "law"),
        });
        predecessor.push(PredecessorCase {
            today,
            window: epoch_days(&case.output["window"]),
            studied: epoch_days(&case.output["studied"]).into_iter().collect(),
        });
    }
    let judged = judge(&predecessor, &served_cases, NAMED);
    println!(
        "examined {} case(s), {} served day(s); window mismatches {}; unnamed studied differences \
         {}; named {}",
        judged.cases, judged.days, judged.window_mismatches, judged.unnamed, judged.named
    );
    assert_eq!(
        judged.cases,
        golden.cases.len(),
        "every golden case is judged"
    );
    assert!(
        !judged.refuses(),
        "the route differs from the predecessor: {judged:?}"
    );

    // Control 1: one studied day dropped from both tracks, the first case's first studied day.
    let mut dropped = served_cases.clone();
    let day = predecessor[0]
        .studied
        .first()
        .copied()
        .expect("the first case studies a day");
    let word = StudyDay::from_epoch_day(day).to_string();
    let first_case = &mut dropped[0];
    for track in [&mut first_case.language, &mut first_case.law] {
        for served_day in track.iter_mut().filter(|served_day| served_day.0 == word) {
            served_day.1 = false;
        }
    }
    let control = judge(&predecessor, &dropped, NAMED);
    println!(
        "control, a dropped studied day: unnamed studied differences {} (refused: {})",
        control.unnamed,
        control.refuses()
    );
    assert_eq!(
        control.unnamed, 1,
        "the dropped day is an unnamed difference"
    );
    assert!(control.refuses(), "the judge refuses a dropped studied day");
    // The same difference under a planted reason is counted named, and not refused.
    let planted = [(predecessor[0].today, day, "a planted reason")];
    let named = judge(&predecessor, &dropped, &planted);
    println!(
        "control, the dropped day named: named {}, unnamed {}",
        named.named, named.unnamed
    );
    assert_eq!((named.named, named.unnamed), (1, 0));
    assert!(
        !named.refuses(),
        "a named difference is reported, not refused"
    );

    // Control 2: every track cut to the 35 days ending at its served day.
    let cut: Vec<ServedCase> = served_cases
        .iter()
        .map(|case| {
            let last = |track: &[(String, bool)]| track[track.len().saturating_sub(35)..].to_vec();
            ServedCase {
                today: case.today,
                language: last(&case.language),
                law: last(&case.law),
            }
        })
        .collect();
    let control = judge(&predecessor, &cut, NAMED);
    println!(
        "control, a 35-day cut: window mismatches {} (refused: {})",
        control.window_mismatches,
        control.refuses()
    );
    assert_eq!(
        control.window_mismatches,
        2 * golden.cases.len(),
        "a 35-day cut differs from every window"
    );
    assert!(control.refuses(), "the judge refuses a 35-day window");
}
