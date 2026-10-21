//! The personal board's pure rules (SPEC-075 R1, A11; #79): the best day is the highest stored
//! score, a tie going to the most recent of the tied days as Python's `max` keeps the first
//! maximum it meets in the most-recent-first rollups; and each row names its kind, emoji, label
//! and value the way the API answers them.

// An integration test is test code: it prints the examined count on purpose.
#![allow(clippy::print_stdout)]

use deck_streak_kernel::StudyDay;
use deck_streak_progression::board::{BoardRow, BoardStreak, DayScore, best_day, board_rows};
use deck_streak_progression::level::level_info;
use deck_streak_progression::xp::XpTotal;

/// The study day with epoch day number `day`.
const fn day(day: i64) -> StudyDay {
    StudyDay::from_epoch_day(day)
}

/// One stored day's score.
const fn scored(on: i64, score: i64) -> DayScore {
    DayScore {
        day: day(on),
        score,
    }
}

#[test]
fn the_best_day_on_a_tie_is_the_most_recent() {
    // Most recent first: 20_000 scores 40, then 19_998 and 19_996 tie at 88 around a 12.
    let totals = [
        scored(20_000, 40),
        scored(19_998, 88),
        scored(19_997, 12),
        scored(19_996, 88),
    ];
    assert_eq!(best_day(&totals), Some(scored(19_998, 88)));
    let rows = board_rows(
        &totals,
        day(20_000),
        BoardStreak {
            current: 3,
            longest: 9,
        },
        &level_info(XpTotal::new(4_000)),
    );
    assert_eq!(
        rows.first(),
        Some(&BoardRow::BestDay {
            score: 88,
            day: day(19_998),
        })
    );
}

#[test]
fn each_board_row_names_its_kind_emoji_label_and_value() {
    let rows = board_rows(
        &[scored(20_000, 36), scored(19_999, 91)],
        day(20_000),
        BoardStreak {
            current: 3,
            longest: 9,
        },
        &level_info(XpTotal::new(4_000)),
    );
    let read: Vec<_> = rows
        .iter()
        .map(|row| {
            (
                row.kind(),
                row.emoji(),
                row.label(),
                row.value(),
                row.study_day(),
                row.longest(),
                row.title(),
            )
        })
        .collect();
    println!("examined {} board row(s)", read.len());
    assert_eq!(
        read,
        vec![
            (
                "best_day",
                "\u{1f3c5}",
                "Best day",
                91,
                Some(day(19_999)),
                None,
                None
            ),
            (
                "today",
                "\u{1f4c5}",
                "Today",
                36,
                Some(day(20_000)),
                None,
                None
            ),
            ("streak", "\u{1f525}", "Streak", 3, None, Some(9), None),
            (
                "level",
                "\u{26a1}",
                "Level",
                9,
                None,
                None,
                Some("Seedling")
            ),
        ]
    );
}

#[test]
fn with_no_rollup_the_board_is_the_streak_and_the_level() {
    let rows = board_rows(
        &[],
        day(20_000),
        BoardStreak {
            current: 0,
            longest: 5,
        },
        &level_info(XpTotal::new(0)),
    );
    assert_eq!(
        rows,
        vec![
            BoardRow::Streak {
                current: 0,
                longest: 5,
            },
            BoardRow::Level {
                level: 1,
                title: "Sprout",
            },
        ]
    );
}

#[test]
fn a_day_with_no_rollup_today_reads_today_as_zero() {
    let rows = board_rows(
        &[scored(19_997, 48), scored(19_994, 75)],
        day(20_000),
        BoardStreak {
            current: 0,
            longest: 2,
        },
        &level_info(XpTotal::new(310)),
    );
    assert_eq!(
        rows.get(..2),
        Some(
            &[
                BoardRow::BestDay {
                    score: 75,
                    day: day(19_994),
                },
                BoardRow::Today {
                    score: 0,
                    day: day(20_000),
                },
            ][..]
        )
    );
}
