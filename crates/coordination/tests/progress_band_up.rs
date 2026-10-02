//! The band-up's record, grant, badge and celebration (SPEC-077 R6, R7; A7, A8): the progress step,
//! run in phase 4 for the current study day, records a course seen for the first time as a silent
//! baseline, and pays no XP, awards no badge and owes no celebration for it, through phase 7's band
//! badge step as well; a band-up is recorded, paid and badged once, and its celebration is offered
//! between the fold's writes until the router answers, then never again. Every card, deck and course
//! here is synthetic.

// An integration test is test code: its helpers panic on a malformed golden or a failed fixture,
// and it prints the examined counts.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

#[path = "awards_support/mod.rs"]
#[allow(dead_code)]
mod support;

use std::collections::BTreeMap;
use std::sync::Arc;

use deck_streak_analytics::settings::AnalyticsSettings;
use deck_streak_coordination::recompute::band_badges::BandBadgesStep;
use deck_streak_coordination::recompute::progress::ProgressStep;
use deck_streak_coordination::recompute::{AwardOffers, Evaluation, Offers};
use deck_streak_ingest::reader::{Card, CollectionData};
use deck_streak_kernel::{CourseCode, Courses, Db, UtcMillis};
use deck_streak_notifications::ladder::{budget_exempt, requested_tier};
use deck_streak_notifications::{Policy, Tier};
use serde_json::{Value, json};
use support::{D0, Recorder, at, badges, card, day, run_step, scratch};

/// One synthetic course whose first three bands each hold one unit's deck.
const COURSES: &str = r#"{"schema":"deckstreak.courses.v1","courses":[{"code":"be","name":"Beta",
"flag":"b","deck_root":"Beta Course","alias":"b","writing":false,
"unit_bands":{"A1":[1,4],"A2":[5,9],"B1":[10,14]}}]}"#;

/// The owner's courses.
fn courses() -> Courses {
    Courses::parse(COURSES).expect("the synthetic courses parse")
}

/// A window with one mature review card of course `be` in each of units 3, 7 and 12: every card's
/// mastery is 1, so A1, A2 and B1 are achieved and the course's current band is B1.
fn window() -> CollectionData {
    let decks = [(1, "Unit 03"), (2, "Unit 07"), (3, "Unit 12")];
    let cards: Vec<Card> = decks
        .iter()
        .map(|(deck, _)| Card {
            course: CourseCode::new("be"),
            ..card(*deck, *deck)
        })
        .collect();
    let deck_names: BTreeMap<i64, String> = decks
        .iter()
        .map(|(deck, unit)| (*deck, format!("Beta Course\u{1f}{unit}")))
        .collect();
    CollectionData {
        reviews: Vec::new(),
        cards,
        created_at: UtcMillis::from_epoch_millis(at(D0 - 1_000, 12)),
        deck_names,
    }
}

/// Every milestone stored, as `(course, band, study day, baseline, marked)`, by course and band.
async fn milestones(db: &Db) -> Vec<(String, String, i64, i64, bool)> {
    sqlx::query_as(
        "SELECT course, band, study_day, baseline, celebrated_at IS NOT NULL \
         FROM band_milestones ORDER BY course, band",
    )
    .fetch_all(db.reader())
    .await
    .expect("the milestones")
}

/// Every course's stored current band, as `(course, band)`, by course.
async fn stored_bands(db: &Db) -> Vec<(String, String)> {
    sqlx::query_as("SELECT course, current_band FROM language_progress ORDER BY course")
        .fetch_all(db.reader())
        .await
        .expect("the stored progress")
}

/// How many XP rows the ledger holds.
async fn xp_rows(db: &Db) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM xp_ledger")
        .fetch_one(db.reader())
        .await
        .expect("the XP ledger's count")
}

/// One recompute of the current study day at `now`: phase 4's progress step, then phase 7's band
/// badge step, each in its own write as the fold runs them.
async fn recompute(db: &Db, data: &CollectionData, now: i64) {
    let courses = courses();
    let progress = ProgressStep::new(courses.clone(), AnalyticsSettings::default());
    run_step(db, &progress, data, 0, (D0, Evaluation::Current), now).await;
    let band_badges = BandBadgesStep::new(courses);
    run_step(db, &band_badges, data, 0, (D0, Evaluation::Current), now).await;
}

#[tokio::test]
async fn the_first_sighting_of_a_course_is_a_silent_baseline() {
    let scratch = scratch().await;
    let db = &scratch.db;
    let data = window();

    recompute(db, &data, at(D0, 12)).await;

    assert_eq!(
        milestones(db).await,
        [("be".to_owned(), "B1".to_owned(), D0, 1, true)],
        "the first sighting records the current band as a silent baseline, owing nothing"
    );
    assert_eq!(
        stored_bands(db).await,
        [("be".to_owned(), "B1".to_owned())],
        "the course's progress is stored with its current band"
    );
    assert_eq!(xp_rows(db).await, 0, "a first sighting pays no XP");
    assert_eq!(badges(db).await, [], "a first sighting awards no badge");

    // A second recompute of the same day sees the stored band: nothing is recorded or paid.
    recompute(db, &data, at(D0, 18)).await;
    assert_eq!(
        milestones(db).await,
        [("be".to_owned(), "B1".to_owned(), D0, 1, true)],
        "the baseline is recorded once"
    );
    assert_eq!(xp_rows(db).await, 0, "the unchanged band pays no XP");
    assert_eq!(badges(db).await, [], "the unchanged band awards no badge");
}

/// The bands in their order. In a golden case's courses each band is one unit of its own: A1 is
/// unit 1 and C2 is unit 6.
const BANDS: [&str; 6] = ["A1", "A2", "B1", "B2", "C1", "C2"];

/// One course of a golden case: its code, name and flag, and the band its progress reaches.
struct Reach {
    code: String,
    name: String,
    flag: String,
    band: String,
}

/// The text field `key` of `value`.
fn text(value: &Value, key: &str) -> String {
    value[key].as_str().expect("a text field").to_owned()
}

/// A golden list of `[code, band]` pairs.
fn pairs(value: &Value) -> Vec<(String, String)> {
    value
        .as_array()
        .expect("a list of pairs")
        .iter()
        .map(|pair| {
            let part = |index: usize| pair[index].as_str().expect("a pair's text").to_owned();
            (part(0), part(1))
        })
        .collect()
}

/// The band of a golden key `<prefix>:<code>:<BAND>` in lowercase, as the grant source and the
/// router's dedupe key spell it (ADR-077): the predecessor spells it uppercase.
fn lowercase_band(key: &str) -> String {
    let (head, band) = key.rsplit_once(':').expect("a key that ends in its band");
    format!("{head}:{}", band.to_ascii_lowercase())
}

/// The courses a case's progress reaches, in the case's order.
fn reaches(input: &Value) -> Vec<Reach> {
    input["progs"]
        .as_array()
        .expect("the case's progress")
        .iter()
        .map(|prog| Reach {
            code: text(prog, "code"),
            name: text(prog, "name"),
            flag: text(prog, "flag"),
            band: text(prog, "band"),
        })
        .collect()
}

/// The owner's courses of a case: each course of its progress, every band one unit from A1 to C2.
fn case_courses(reaches: &[Reach]) -> Courses {
    let courses: Vec<Value> = reaches
        .iter()
        .zip('a'..='z')
        .map(|(reach, alias)| {
            json!({
                "code": reach.code,
                "name": reach.name,
                "flag": reach.flag,
                "deck_root": format!("Course {}", reach.code),
                "alias": alias.to_string(),
                "writing": false,
                "unit_bands": {
                    "A1": [1, 1], "A2": [2, 2], "B1": [3, 3],
                    "B2": [4, 4], "C1": [5, 5], "C2": [6, 6]
                }
            })
        })
        .collect();
    let file = json!({"schema": "deckstreak.courses.v1", "courses": courses});
    Courses::parse(&file.to_string()).expect("the case's courses parse")
}

/// A window in which each course reaches its band: one mature review card in the unit of every band
/// from A1 to it, so that run is achieved and no later band is.
fn case_window(reaches: &[Reach]) -> CollectionData {
    let mut cards = Vec::new();
    let mut deck_names = BTreeMap::new();
    for (index, reach) in (1_i64..).zip(reaches) {
        let band_index = BANDS
            .iter()
            .position(|band| *band == reach.band)
            .expect("a CEFR band");
        for unit in (1_i64..).take(band_index + 1) {
            let id = 100 * index + unit;
            cards.push(Card {
                course: CourseCode::new(&reach.code),
                ..card(id, id)
            });
            deck_names.insert(id, format!("Course {}\u{1f}Unit {unit:02}", reach.code));
        }
    }
    CollectionData {
        reviews: Vec::new(),
        cards,
        created_at: UtcMillis::from_epoch_millis(at(D0 - 1_000, 12)),
        deck_names,
    }
}

/// Stores a case's stored bands as the courses' progress, and its milestones as bands reached on an
/// earlier day and already celebrated, as the predecessor's store holds them before the cycle.
async fn seed(db: &Db, stored: &[(String, String)], reached: &[(String, String)]) {
    let earlier = at(D0 - 1, 12);
    let mut write = db.write().await.expect("a write");
    for (course, band) in stored {
        sqlx::query(
            "INSERT INTO language_progress (course, name, flag, mastery_pct, current_band, \
             mature_cards, total_cards, current_unit, bands, updated_at, created_at) \
             VALUES (?1, ?1, 'F', 0.0, ?2, 0, 0, NULL, '[]', ?3, ?3)",
        )
        .bind(course)
        .bind(band)
        .bind(earlier)
        .execute(&mut *write)
        .await
        .expect("the stored progress writes");
    }
    for (course, band) in reached {
        sqlx::query(
            "INSERT INTO band_milestones (course, band, study_day, baseline, celebrated_at, \
             created_at) VALUES (?1, ?2, ?3, 0, ?4, ?4)",
        )
        .bind(course)
        .bind(band)
        .bind(D0 - 1)
        .bind(earlier)
        .execute(&mut *write)
        .await
        .expect("the stored milestone writes");
    }
    write.commit().await.expect("the seed commits");
}

/// Every XP grant, as `(source, amount, study day)`, by source.
async fn grants(db: &Db) -> Vec<(String, i64, i64)> {
    sqlx::query_as("SELECT source, amount, study_day FROM xp_ledger ORDER BY source")
        .fetch_all(db.reader())
        .await
        .expect("the XP ledger")
}

/// Every badge, as `(key, name, emoji, tier, marked)`, by key.
async fn named_badges(db: &Db) -> Vec<(String, String, String, i64, bool)> {
    sqlx::query_as(
        "SELECT badge_key, name, emoji, tier, celebrated_at IS NOT NULL FROM badges_earned \
         ORDER BY badge_key, tier",
    )
    .fetch_all(db.reader())
    .await
    .expect("the badges")
}

/// Every celebration handed to `router`, as `(event, dedupe key)`, sorted.
fn offered(router: &Recorder) -> Vec<(String, String)> {
    let mut offered: Vec<(String, String)> = router
        .handed()
        .into_iter()
        .map(|celebration| (celebration.event.to_owned(), celebration.key))
        .collect();
    offered.sort();
    offered
}

/// The fold's offers between its writes, at `now`, through `router`.
async fn offer(db: &Db, router: &Arc<Recorder>, now: i64) {
    let offers = AwardOffers::new(router.clone());
    offers
        .offer(db, UtcMillis::from_epoch_millis(now), day(D0))
        .await
        .expect("the offers never fail the fold");
}

/// One recompute of a case's current study day at `now`: phase 4's progress step, then phase 7's
/// band badge step, each in its own write.
async fn recompute_case(db: &Db, courses: &Courses, data: &CollectionData, now: i64) {
    let progress = ProgressStep::new(courses.clone(), AnalyticsSettings::default());
    run_step(db, &progress, data, 0, (D0, Evaluation::Current), now).await;
    let band_badges = BandBadgesStep::new(courses.clone());
    run_step(db, &band_badges, data, 0, (D0, Evaluation::Current), now).await;
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn band_ups_match_the_predecessors_golden_and_pay_once() {
    let band_ups = golden::read(&golden::committed("band_up")).expect("the band-up golden");
    let policy = Policy::compiled().expect("the compiled policy parses");
    let mut offered_band_ups = 0_usize;
    for (index, case) in band_ups.cases.iter().enumerate() {
        let (input, output) = (&case.input, &case.output);
        let class = case.class.as_deref().unwrap_or("drawn");
        let label = format!("case {index} ({class})");
        assert_eq!(
            input["today"].as_i64(),
            Some(D0),
            "{label}: the golden's study day"
        );
        let scratch = scratch().await;
        let db = &scratch.db;
        let reaches = reaches(input);
        let courses = case_courses(&reaches);
        let data = case_window(&reaches);
        seed(db, &pairs(&input["stored"]), &pairs(&input["milestones"])).await;

        recompute_case(db, &courses, &data, at(D0, 12)).await;

        // The milestones this recompute recorded: the golden's records that were new.
        let mut recorded: Vec<(String, String)> = milestones(db)
            .await
            .into_iter()
            .filter(|(_, _, study_day, _, _)| *study_day == D0)
            .map(|(course, band, _, _, _)| (course, band))
            .collect();
        recorded.sort();
        let mut new_records: Vec<(String, String)> = output["records"]
            .as_array()
            .expect("the golden's records")
            .iter()
            .filter(|record| record["new"].as_bool() == Some(true))
            .map(|record| (text(record, "code"), text(record, "band")))
            .collect();
        new_records.sort();
        assert_eq!(recorded, new_records, "{label}: the milestones recorded");

        let mut paid: Vec<(String, i64, i64)> = output["grants"]
            .as_array()
            .expect("the golden's grants")
            .iter()
            .map(|grant| {
                let amount = grant["amount"].as_i64().expect("a grant's amount");
                let study_day = grant["day"].as_i64().expect("a grant's day");
                (lowercase_band(&text(grant, "source")), amount, study_day)
            })
            .collect();
        paid.sort();
        assert_eq!(grants(db).await, paid, "{label}: each band-up's XP");

        let mut badged: Vec<(String, String, String, i64, bool)> = output["badges"]
            .as_array()
            .expect("the golden's badges")
            .iter()
            .map(|badge| {
                let (key, name, emoji) = (
                    text(badge, "key"),
                    text(badge, "name"),
                    text(badge, "emoji"),
                );
                (key, name, emoji, 0, true)
            })
            .collect();
        badged.sort();
        assert_eq!(
            named_badges(db).await,
            badged,
            "{label}: each band-up's badge, written marked"
        );

        // Every celebration passes through the one router (ADR-041), which owns whether it is sent:
        // the predecessor's cases with no notifier, or with its milestones muted, raise none, and
        // here the band-up is still offered, as each of its paid band-ups is.
        let notified =
            input["notifier"].as_bool() == Some(true) && input["notify"].as_bool() == Some(true);
        let mut owed: Vec<(String, String)> = if notified {
            output["celebrations"]
                .as_array()
                .expect("the golden's celebrations")
                .iter()
                .map(|celebration| {
                    let event = text(celebration, "event_type");
                    assert_eq!(
                        requested_tier(&policy, &event, None),
                        Tier::T5,
                        "{label}: the band-up's celebration resolves to T5"
                    );
                    assert_eq!(
                        celebration["budget_exempt"].as_bool(),
                        Some(budget_exempt(&policy, &event)),
                        "{label}: the band-up's celebration is exempt from the weekly budget"
                    );
                    (event, lowercase_band(&text(celebration, "event_key")))
                })
                .collect()
        } else {
            paid.iter()
                .map(|(source, _, _)| ("band_up".to_owned(), source.clone()))
                .collect()
        };
        owed.sort();

        // A router that does not answer is handed each band-up, and nothing is marked.
        let silent = Arc::new(Recorder::silent());
        offer(db, &silent, at(D0, 12)).await;
        assert_eq!(
            offered(&silent),
            owed,
            "{label}: each band-up is offered to the router"
        );
        let unmarked = milestones(db)
            .await
            .into_iter()
            .filter(|(_, _, _, _, marked)| !marked)
            .count();
        assert_eq!(
            unmarked,
            owed.len(),
            "{label}: an unanswered offer leaves its band-up owed"
        );

        // The router answers: each band-up is offered again and marked.
        let router = Arc::new(Recorder::default());
        offer(db, &router, at(D0, 12)).await;
        assert_eq!(
            offered(&router),
            owed,
            "{label}: the owed band-ups are offered once more"
        );
        assert!(
            milestones(db)
                .await
                .iter()
                .all(|(_, _, _, _, marked)| *marked),
            "{label}: an answered band-up is marked"
        );

        // A second recompute and its offers write, pay and offer nothing more.
        let stored_milestones = milestones(db).await;
        recompute_case(db, &courses, &data, at(D0, 18)).await;
        offer(db, &router, at(D0, 18)).await;
        assert_eq!(
            offered(&router),
            owed,
            "{label}: a second recompute offers nothing"
        );
        assert_eq!(
            milestones(db).await,
            stored_milestones,
            "{label}: each band is recorded once"
        );
        assert_eq!(grants(db).await, paid, "{label}: each band-up is paid once");
        assert_eq!(
            named_badges(db).await,
            badged,
            "{label}: each band badge is awarded once"
        );
        offered_band_ups += owed.len();
    }
    let examined = golden::Examined {
        function: band_ups.function.clone(),
        count: band_ups.cases.len(),
    };
    println!("{examined}");
    println!("examined {offered_band_ups} band-up offer(s)");
    assert!(
        examined.count > 0,
        "{examined}: a golden with no case proves nothing"
    );
    assert!(
        offered_band_ups > 0,
        "no case offered a band-up, so the offers were never judged"
    );
}
