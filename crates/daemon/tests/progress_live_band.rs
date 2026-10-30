//! The live band the persona engine reads (SPEC-077 R8, A9): a language mentor whose course is
//! configured and stored writes at the stored current band, not the roster's; with the course not
//! configured, or nothing stored yet, the roster's band stands; a law persona carries no band.
//!
//! The roster is synthetic, built over the public templates as the agent's own tests build it,
//! and the stored course progress is written as the progress step writes it.

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use deck_streak_agent::{
    CefrBand, Duty, LiveBand, Persona, Roster, Subject, TemplateSet, TopicKey, resolve_band,
};
use deck_streak_curriculum::progress::CourseProgress;
use deck_streak_curriculum::store::put_progress;
use deck_streak_daemon::wiring::CurriculumLiveBand;
use deck_streak_kernel::{CourseCode, Courses, Db, UtcMillis};
use serde_json::json;
use tempfile::TempDir;

/// Synthetic courses: `es` among them, beside one more.
const WITH_ES: &str = r#"{"schema":"deckstreak.courses.v1","courses":[
{"code":"es","name":"Spanish","flag":"e","deck_root":"Spanish Course","alias":"e","writing":false,
"unit_bands":{"A1":[1,4]}},
{"code":"be","name":"Beta","flag":"b","deck_root":"Beta Course","alias":"b","writing":false,
"unit_bands":{"A1":[1,4]}}]}"#;

/// Synthetic courses that do not name `es`.
const WITHOUT_ES: &str = r#"{"schema":"deckstreak.courses.v1","courses":[
{"code":"be","name":"Beta","flag":"b","deck_root":"Beta Course","alias":"b","writing":false,
"unit_bands":{"A1":[1,4]}}]}"#;

/// The courses `text`.
fn courses(text: &str) -> Courses {
    Courses::parse(text).expect("the synthetic courses parse")
}

/// A synthetic roster: an invented professor and an invented Spanish mentor at A2.
fn roster() -> Roster {
    let roster = json!({
        "schema": "deckstreak.agent.roster.v1",
        "personas": {
            "law-evidence": {
                "name": "Professor Quill Testwell",
                "bio": "A synthetic bio of an invented professor.",
                "voice": "A synthetic voice, measured and exact.",
                "personality": "A synthetic personality that asks before it tells."
            },
            "language-mentor-es": {
                "name": "Mentor Vela Sample",
                "bio": "A synthetic bio of an invented mentor.",
                "voice": "A synthetic voice, warm and clear.",
                "personality": "A synthetic personality that praises what went right."
            }
        },
        "topics": {
            "synthetic/law-topic": {"template": "law-evidence"},
            "synthetic/language-topic": {"template": "language-mentor-es", "cefr": "A2"}
        }
    });
    let public = TemplateSet::public().expect("the public templates load");
    Roster::parse(&roster.to_string(), public).expect("the roster reads")
}

/// The persona the roster gives `topic`'s daily reading.
fn persona(roster: &Roster, topic: &str) -> Persona {
    let topic = TopicKey::parse(topic).expect("a synthetic topic key");
    roster
        .persona(&topic, Duty::DailyReading)
        .expect("the roster names a persona for the topic")
}

/// A scratch database, every migration applied.
async fn scratch() -> (TempDir, Db) {
    let dir = TempDir::new().expect("a scratch directory");
    let db = Db::open(&dir.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    (dir, db)
}

/// Stores `code`'s course progress at `band`, as the progress step writes it.
async fn store_band(db: &Db, code: &str, band: &'static str) {
    let progress = CourseProgress {
        code: CourseCode::new(code).expect("a course code"),
        name: code.to_owned(),
        flag: code.to_owned(),
        total_cards: 10,
        mature_cards: 4,
        mastery_pct: 40.0,
        current_band: band,
        bands: Vec::new(),
        current_unit: Some(5),
    };
    let mut write = db.write().await.expect("a write");
    put_progress(&mut write, &progress, UtcMillis::from_epoch_millis(1_000))
        .await
        .expect("the progress writes");
    write.commit().await.expect("the progress commits");
}

#[tokio::test]
async fn the_persona_engine_reads_the_live_band() {
    let roster = roster();
    let mentor = persona(&roster, "synthetic/language-topic");
    let professor = persona(&roster, "synthetic/law-topic");
    assert_eq!(
        mentor.subject().as_str(),
        "language/es",
        "the mentor's subject"
    );

    let (_dir, db) = scratch().await;
    store_band(&db, "es", "B1").await;
    store_band(&db, "be", "C1").await;
    let live = CurriculumLiveBand::new(db.clone(), courses(WITH_ES));
    assert_eq!(
        resolve_band(&mentor, Some(&live))
            .await
            .expect("the live band reads"),
        Some(CefrBand::B1),
        "the stored current band of the configured course the subject names"
    );

    let unconfigured = CurriculumLiveBand::new(db.clone(), courses(WITHOUT_ES));
    assert_eq!(
        resolve_band(&mentor, Some(&unconfigured))
            .await
            .expect("the live band reads"),
        Some(CefrBand::A2),
        "the roster's band, when the subject's course is not configured"
    );

    let (_empty_dir, empty) = scratch().await;
    let unstored = CurriculumLiveBand::new(empty, courses(WITH_ES));
    assert_eq!(
        resolve_band(&mentor, Some(&unstored))
            .await
            .expect("the live band reads"),
        Some(CefrBand::A2),
        "the roster's band, before the course's progress is stored"
    );

    assert_eq!(
        resolve_band(&professor, Some(&live))
            .await
            .expect("no band to read"),
        None,
        "a law persona carries no band"
    );
    let law = Subject::parse("law/es").expect("a law subject");
    assert_eq!(
        live.band(&law).await.expect("the port answers"),
        None,
        "the port answers no band for a subject of another kind, whatever its area"
    );
}
