//! The award port writes one row per key and tier (SPEC-073 A3 to A5; R1, R3, R4): a second award
//! answers `AlreadyAwarded`, a band key names a configured course and a band from A1 to C2, and
//! any other key is refused before a write by an error that names the rule, never the value.

// An integration test is test code: its helpers panic on a failed fixture, and it prints the
// examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_kernel::{Courses, Db, StudyDay, UtcMillis};
use deck_streak_progression::badges::award::{
    Award, AwardError, BANDS, KeyKind, NewBadge, award, key_kind, unmarked,
};
use tempfile::TempDir;

/// Two synthetic courses.
const COURSES: &str = r#"{
  "schema": "deckstreak.courses.v1",
  "courses": [
    {"code": "qaa", "name": "Course Qaa", "flag": "F", "deck_root": "Qaa", "alias": "a",
     "writing": false, "unit_bands": {"A1": [1, 5]}},
    {"code": "qab", "name": "Course Qab", "flag": "F", "deck_root": "Qab Deck", "alias": "b",
     "writing": true, "unit_bands": {}}
  ],
  "focus_subjects": []
}"#;
/// The services clock's instant every award is made at.
const AT: i64 = 1_736_911_810_000;
/// The study day every award is earned on.
const DAY: i64 = 20_102;

fn courses() -> Courses {
    Courses::parse(COURSES).expect("the synthetic courses parse")
}

async fn database(scratch: &TempDir) -> Db {
    Db::open(&scratch.path().join("deck_streak.db"))
        .await
        .expect("the database opens")
}

/// Awards `key` at tier 0 in one write of its own.
async fn award_once(db: &Db, key: &str) -> Result<Award, AwardError> {
    let mut write = db.write().await.expect("a write");
    let answer = award(
        &mut write,
        &courses(),
        &NewBadge {
            key,
            tier: 0,
            name: "Synthetic",
            emoji: "S",
            study_day: StudyDay::from_epoch_day(DAY),
            at: UtcMillis::from_epoch_millis(AT),
        },
    )
    .await;
    write.commit().await.expect("the commit");
    answer
}

/// The rows `badges_earned` holds.
async fn rows(db: &Db) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM badges_earned")
        .fetch_one(db.reader())
        .await
        .expect("the count")
}

#[tokio::test]
async fn awarding_a_badge_twice_writes_one_row() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let db = database(&scratch).await;
    let first = award_once(&db, "first_steps").await.expect("a catalog key");
    let second = award_once(&db, "first_steps").await.expect("a catalog key");
    assert_eq!(first, Award::Awarded);
    assert_eq!(second, Award::AlreadyAwarded);
    assert_eq!(rows(&db).await, 1, "one row for one key and tier");
    // The catalog badge's celebration is still owed: its mark is unset.
    let mut connection = db.reader().acquire().await.expect("a connection");
    let owed = unmarked(&mut connection)
        .await
        .expect("the unmarked badges");
    assert_eq!(
        owed.iter()
            .map(|badge| badge.key.as_str())
            .collect::<Vec<_>>(),
        ["first_steps"]
    );
    // The pool's close waits for every connection, so the one held here goes back first.
    drop(connection);
    db.close().await;
}

#[tokio::test]
async fn a_band_badge_key_names_a_configured_course_and_a_band() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let db = database(&scratch).await;
    let courses = courses();
    let mut accepted = Vec::new();
    for code in ["qaa", "qab"] {
        for band in BANDS {
            accepted.push(format!("band_{code}_{band}"));
        }
    }
    let refused = [
        "band_qzz_B1",
        "band_qaa_D1",
        "band_qaa_b1",
        "band_qaa",
        "band__B1",
        "band_qaa_B1_x",
        "Band_qaa_B1",
    ];
    println!(
        "examined {} accepted and {} refused band key(s)",
        accepted.len(),
        refused.len()
    );
    assert_eq!(accepted.len(), 12, "two courses, six bands");
    for key in &accepted {
        assert_eq!(key_kind(&courses, key), Some(KeyKind::Band), "{key}");
        assert_eq!(
            award_once(&db, key).await.expect(key),
            Award::Awarded,
            "{key}"
        );
    }
    for key in refused {
        assert_eq!(key_kind(&courses, key), None, "{key}");
        assert!(
            matches!(award_once(&db, key).await, Err(AwardError::UnknownKey)),
            "{key} is refused"
        );
    }
    assert_eq!(rows(&db).await, 12, "only the accepted keys are written");
    // A band badge is celebrated by the band-up: it is written marked, so no offer raises it.
    let mut connection = db.reader().acquire().await.expect("a connection");
    assert!(
        unmarked(&mut connection)
            .await
            .expect("the unmarked badges")
            .is_empty()
    );
    // The pool's close waits for every connection, so the one held here goes back first.
    drop(connection);
    db.close().await;
}

#[tokio::test]
async fn an_unknown_badge_key_is_refused_before_a_write() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let db = database(&scratch).await;
    let unknown = ["no_such_badge", "", "first_steps ", "FIRST_STEPS"];
    println!("examined {} unknown key(s)", unknown.len());
    for key in unknown {
        let answer = award_once(&db, key).await;
        assert!(
            matches!(answer, Err(AwardError::UnknownKey)),
            "{key} is refused: {answer:?}"
        );
        let words = answer.map_or_else(|refusal| refusal.to_string(), |_| String::new());
        assert!(
            words.contains("band_<code>_<band>"),
            "the rule is named: {words}"
        );
        if !key.trim().is_empty() {
            assert!(
                !words.contains(key.trim()),
                "the value is not named: {words}"
            );
        }
    }
    assert_eq!(rows(&db).await, 0, "nothing is written");
    // A catalog key is accepted, so the refusal above is the key's and not the port's.
    assert_eq!(
        award_once(&db, "first_steps").await.expect("a catalog key"),
        Award::Awarded
    );
    db.close().await;
}
