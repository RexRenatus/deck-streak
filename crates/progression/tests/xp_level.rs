//! The level info and titles are the predecessor's, and the level is read from both XP tables
//! (SPEC-072 A13, A16; R10, R13).

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_kernel::{Db, StudyDay, Track, UtcMillis};
use deck_streak_progression::grant::{GrantPort, GrantRequest, GrantScope, GrantSource};
use deck_streak_progression::ledger::SqliteXpLedger;
use deck_streak_progression::level::{level_info, level_title};
use deck_streak_progression::settle::{SettleCause, SettleRequest, settle};
use deck_streak_progression::xp::{XpAmount, XpTotal, level_for};

#[test]
fn level_info_and_titles_match_the_parity_goldens() {
    golden::each_case("level_info", |case| {
        let total = case.input["total_xp"].as_u64().expect("a total");
        let info = level_info(XpTotal::new(total));
        let expected = &case.output;
        assert_eq!(info.total_xp, expected["total_xp"].as_u64().expect("total"));
        assert_eq!(
            u64::from(info.level.get()),
            expected["level"].as_u64().expect("level"),
            "the level of {total}"
        );
        assert_eq!(info.title, expected["title"].as_str().expect("title"));
        assert_eq!(info.emoji, expected["emoji"].as_str().expect("emoji"));
        assert_eq!(
            info.xp_into_level,
            expected["xp_into_level"].as_u64().expect("into"),
            "the XP into the level of {total}"
        );
        assert_eq!(
            info.xp_for_next,
            expected["xp_for_next"].as_u64().expect("for next"),
            "the XP the level of {total} spans"
        );
    });
    golden::each_case("level_title", |case| {
        let level = level_for(XpTotal::new(
            // The least total of the level: `50 L^2 - 50 L`.
            50 * case.input["level"].as_u64().expect("a level").pow(2)
                - 50 * case.input["level"].as_u64().expect("a level"),
        ));
        let (title, emoji) = level_title(level);
        let expected = case.output.as_array().expect("a title and an emoji");
        assert_eq!(
            (title, emoji),
            (
                expected[0].as_str().expect("title"),
                expected[1].as_str().expect("emoji")
            ),
            "the title of level {}",
            case.input["level"]
        );
    });
}

#[tokio::test]
async fn the_level_is_read_from_both_xp_tables() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    let ledger = SqliteXpLedger::new(db.clone());
    let at = UtcMillis::from_epoch_millis(1_700_000_000_000);
    let _granted = ledger
        .grant(
            &GrantRequest {
                study_day: StudyDay::from_epoch_day(20_000),
                source: GrantSource::new("reading:read:r1").expect("a source"),
                track: Track::Language,
                amount: XpAmount::new(60),
                scope: GrantScope::PerDay,
            },
            at,
        )
        .await
        .expect("the grant runs");
    assert_eq!(ledger.total().await.expect("total").get(), 60);
    let mut write = db.write().await.expect("a write");
    settle(
        &mut write,
        &SettleRequest {
            study_day: StudyDay::from_epoch_day(20_000),
            source: "reviews_law",
            track: Track::Law,
            amount: 40,
            closed: true,
        },
        SettleCause::Recompute,
        at,
    )
    .await
    .expect("the settlement runs");
    write.commit().await.expect("commit");
    assert_eq!(
        ledger.total().await.expect("total").get(),
        100,
        "both tables"
    );
    assert_eq!(
        ledger.track_total(Track::Law).await.expect("law").get(),
        40,
        "the law track reads the settlement"
    );
    assert_eq!(
        ledger
            .track_total(Track::Language)
            .await
            .expect("language")
            .get(),
        60
    );
    assert_eq!(
        ledger.level().await.expect("level").get(),
        2,
        "100 XP is level 2"
    );
}
