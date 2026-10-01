//! The streak calendar through `GET /api/streak` over the real store (SPEC-076 A58, A59, A60 at the
//! route): after the fold has settled a constructed population, each served day carries exactly
//! its own markers, every freeze the fold spent has its covered day served as `freeze`, and a track
//! is settled on exactly its study days. Every review, card and instant is synthetic.

#![allow(clippy::expect_used, clippy::print_stdout)]

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
use deck_streak_streaks::constants::CALENDAR_DAYS;
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
/// 2025-01-15T03:30:10Z: the owner's study day is the 14th, [`TODAY`].
const STARTED_AT: i64 = 1_736_911_810_000;
const TODAY: i64 = 20_102;
const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;

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
            now: UtcMillis::from_epoch_millis(at(TODAY, 12)),
            synced_in: Some(StudyDay::from_epoch_day(TODAY)),
            courses_digest: Some("0123456789abcdef"),
        },
    )
    .await
    .expect("the fold runs");
    let clock = Arc::new(ManualClock::new(UtcMillis::from_epoch_millis(STARTED_AT)));
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
    let request = Request::builder()
        .method("POST")
        .uri("/api/session")
        .header("content-type", "application/json")
        .header("sec-fetch-site", "same-origin")
        .body(Body::from(
            serde_json::json!({ "init_data": OWNER_PAYLOAD }).to_string(),
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
            assert_eq!(got.len(), CALENDAR_DAYS, "{track}: the window's length");
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
        let first = TODAY - i64::try_from(CALENDAR_DAYS).expect("a small constant") + 1;
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
