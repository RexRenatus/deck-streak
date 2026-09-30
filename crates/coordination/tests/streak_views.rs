//! The streak and governor views (SPEC-076 A34; R20, R21): what is at stake, the heat and the
//! verdict, generated over every combination of the rows they read and compared with an oracle
//! written from the SPEC's words. Every row is synthetic.

#![allow(clippy::expect_used, clippy::print_stdout)]

use deck_streak_coordination::streak_views::{AtStake, GovernorView, governor_view, streak_view};
use deck_streak_kernel::{Db, StudyDay};
use tempfile::TempDir;

const TODAY: i64 = 20_000;

async fn database(scratch: &TempDir) -> Db {
    Db::open(&scratch.path().join("deckstreak.db"))
        .await
        .expect("the database opens")
}

fn heat_of(days: u32) -> &'static str {
    // The heat table's words: 365, 100, 30, 7 and 1 days wear their emoji; fewer wear none.
    match days {
        365.. => "\u{1f31f}\u{2604}\u{fe0f}\u{1f31f}",
        100.. => "\u{2604}\u{fe0f}",
        30.. => "\u{1f525}\u{1f525}\u{1f525}",
        7.. => "\u{1f525}\u{1f525}",
        1.. => "\u{1f525}",
        0 => "",
    }
}

fn tier_of(days: u32) -> u32 {
    [1_u32, 7, 30, 100, 365]
        .iter()
        .map(|threshold| u32::from(days >= *threshold))
        .sum()
}

/// A rule (A34): a missed day costs a freeze on the language track while it holds one and the run
/// is live, breaks the run when it holds none, and costs nothing when there is no run; the law
/// track holds no freezes, so a live run breaks and no run costs nothing. Every language run length
/// at each heat edge, every freeze count, and three law runs, over stored rows.
#[tokio::test]
async fn the_view_names_what_is_at_stake_and_the_heat_for_every_stored_pair() {
    let mut cases = 0_u32;
    for current in [0_u32, 1, 6, 7, 29, 30, 99, 100, 364, 365] {
        for freezes in 0..=3_u32 {
            for law in [0_u32, 1, 30] {
                let scratch = TempDir::new().expect("a scratch directory");
                let db = database(&scratch).await;
                let mut write = db.write().await.expect("a write");
                for (track, run, held) in [("language", current, freezes), ("law", law, 0)] {
                    sqlx::query(
                        "INSERT INTO streak_state (track, current_days, longest_days, freezes, \
                         last_study_day, comeback_armed, created_at) VALUES (?1, ?2, ?3, ?4, 19999, 0, 1)",
                    )
                    .bind(track)
                    .bind(i64::from(run))
                    .bind(i64::from(run) + 2)
                    .bind(i64::from(held))
                    .execute(&mut *write)
                    .await
                    .expect("a streak row");
                }
                write.commit().await.expect("the commit");
                let view = streak_view(&db, StudyDay::from_epoch_day(TODAY))
                    .await
                    .expect("the view");
                let language_at_stake = if current == 0 {
                    AtStake::Nothing
                } else if freezes == 0 {
                    AtStake::Break
                } else {
                    AtStake::Freeze
                };
                let law_at_stake = if law == 0 {
                    AtStake::Nothing
                } else {
                    AtStake::Break
                };
                assert_eq!(
                    view.language_at_stake, language_at_stake,
                    "{current}/{freezes}"
                );
                assert_eq!(view.law_at_stake, law_at_stake, "law {law}");
                assert_eq!(view.language_heat, heat_of(current));
                assert_eq!(view.language_tier, tier_of(current));
                assert_eq!(view.law_tier, tier_of(law));
                assert_eq!(view.freeze_cap, 3);
                assert_eq!(view.study_day.epoch_day(), TODAY);
                assert_eq!(
                    (
                        view.language.current,
                        view.language.longest,
                        view.language.freezes
                    ),
                    (current, current + 2, freezes)
                );
                assert_eq!((view.law.current, view.law.longest), (law, law + 2));
                cases += 1;
                db.close().await;
            }
        }
    }
    println!("streak-view population: {cases} stored pairs");
    assert_eq!(cases, 10 * 4 * 3);
}

/// A rule (A34): with no stored row a track reads as its start state, one freeze and no run, and
/// nothing is at stake.
#[tokio::test]
async fn a_track_with_no_row_reads_as_its_start_state() {
    let scratch = TempDir::new().expect("a scratch directory");
    let db = database(&scratch).await;
    let view = streak_view(&db, StudyDay::from_epoch_day(TODAY))
        .await
        .expect("the view");
    assert_eq!((view.language.current, view.language.freezes), (0, 1));
    assert_eq!((view.law.current, view.law.freezes), (0, 1));
    assert_eq!(
        (view.language_at_stake, view.law_at_stake),
        (AtStake::Nothing, AtStake::Nothing)
    );
    assert_eq!(
        (view.language_heat, view.language_tier, view.law_tier),
        ("", 0, 0)
    );
}

/// A rule (A34): the words the views use for what is at stake.
#[test]
fn what_is_at_stake_is_named_in_the_views_words() {
    let words: Vec<&str> = [AtStake::Freeze, AtStake::Break, AtStake::Nothing]
        .into_iter()
        .map(AtStake::as_str)
        .collect();
    assert_eq!(words, ["freeze", "break", "none"]);
}

/// A rule (A34): the verdict is lapse when a lapse is open, else standby when the governor stands
/// by, else armed; the full truth table, then the stored rows through the view: the anchor, the
/// latest strength (zero before the first day settles) and the relight reviews.
#[tokio::test]
async fn the_verdict_follows_the_lapse_and_standby_over_every_stored_row() {
    let mut cases = 0_u32;
    for lapse in [false, true] {
        for standby in [false, true] {
            let expected = if lapse {
                "lapse"
            } else if standby {
                "standby"
            } else {
                "armed"
            };
            let view = GovernorView {
                strength: 0.5,
                lapse,
                lapse_since: None,
                relight_reviews: 3,
                standby,
            };
            assert_eq!(view.verdict(), expected, "lapse {lapse}, standby {standby}");
            for strength in [None, Some(0.4)] {
                let scratch = TempDir::new().expect("a scratch directory");
                let db = database(&scratch).await;
                let mut write = db.write().await.expect("a write");
                sqlx::query("UPDATE governor_state SET lapse_since = ?1, standby = ?2")
                    .bind(lapse.then_some(19_990_i64))
                    .bind(i64::from(standby))
                    .execute(&mut *write)
                    .await
                    .expect("the governor row");
                if let Some(value) = strength {
                    for (day, held) in [(19_998_i64, 0.9), (19_999, value)] {
                        sqlx::query("INSERT INTO habit_strength VALUES (?1, ?2, 1)")
                            .bind(day)
                            .bind(held)
                            .execute(&mut *write)
                            .await
                            .expect("a strength row");
                    }
                }
                write.commit().await.expect("the commit");
                let read = governor_view(&db).await.expect("the view");
                assert_eq!(read.lapse, lapse);
                assert_eq!(read.standby, standby);
                assert_eq!(
                    read.lapse_since.map(StudyDay::epoch_day),
                    lapse.then_some(19_990)
                );
                assert_eq!(read.relight_reviews, 3);
                assert!((read.strength - strength.unwrap_or(0.0)).abs() < f64::EPSILON);
                assert_eq!(read.verdict(), expected);
                cases += 1;
                db.close().await;
            }
        }
    }
    println!("governor-view population: {cases} stored rows");
    assert_eq!(cases, 8);
}
