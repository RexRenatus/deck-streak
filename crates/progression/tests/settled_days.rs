//! `settled_days` answers a track's study days from the settlement: the days its source settled
//! above nothing, oldest first, and no other track's or source's (ADR-302 D4, SPEC-076 A59).

// An integration test is test code: its helpers panic on a failed fixture.
#![allow(clippy::expect_used)]

use deck_streak_kernel::{Db, StudyDay, Track, UtcMillis};
use deck_streak_progression::settle::{SettleCause, SettleRequest, settle};
use deck_streak_progression::settled_days;

const AT: i64 = 1_700_000_000_000;

async fn plant(db: &Db, source: &str, track: Track, amount: u32, day: i64) {
    let request = SettleRequest {
        study_day: StudyDay::from_epoch_day(day),
        source,
        track,
        amount,
        closed: true,
    };
    let mut write = db.write().await.expect("a write");
    settle(
        &mut write,
        &request,
        SettleCause::Recompute,
        UtcMillis::from_epoch_millis(AT),
    )
    .await
    .expect("the settlement runs");
    write.commit().await.expect("commit");
}

#[tokio::test]
async fn the_settled_days_are_the_sources_positive_days_on_the_tracks_own_rows() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let db = Db::open(&directory.path().join("deck_streak.db"))
        .await
        .expect("the database opens");
    plant(&db, "reviews", Track::Language, 12, 300).await;
    plant(&db, "reviews", Track::Language, 0, 299).await;
    plant(&db, "reviews", Track::Language, 7, 298).await;
    plant(&db, "reviews", Track::Law, 9, 297).await;
    plant(&db, "reviews_law", Track::Law, 5, 296).await;
    plant(&db, "reviews_law", Track::Language, 5, 295).await;
    let mut connection = db.reader().acquire().await.expect("a reader");
    let language = settled_days(&mut connection, "reviews", Track::Language)
        .await
        .expect("the language days");
    let law = settled_days(&mut connection, "reviews_law", Track::Law)
        .await
        .expect("the law days");
    let epoch = |days: Vec<StudyDay>| days.iter().map(|day| day.epoch_day()).collect::<Vec<_>>();
    println!(
        "examined {} settled day(s) read",
        epoch(language.clone()).len() + epoch(law.clone()).len()
    );
    assert_eq!(epoch(language), vec![298, 300]);
    assert_eq!(epoch(law), vec![296]);
}
