//! The window (SPEC-023 A8, A9): the floor, the base and its rebase equal the predecessor's, and a
//! recount that shrinks is reported by the self-check.

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;
mod support;

use deck_streak_ingest::state::SqliteIngestState;
use deck_streak_ingest::window::{
    INGEST_REBASE_DAYS, INGEST_WINDOW_DAYS, Rebase, SelfCheck, WindowBase, fresh_floor,
    read_window, rebase, rebased,
};
use deck_streak_kernel::UtcMillis;
use serde_json::Value;
use support::Fixture;
use support::logs::Logs;
use support::synthetic::{self, PlannedCard, PlannedReview};

#[path = "../../../tools/log-capture/capture.rs"]
mod log_capture;

/// An endpoint no test contacts: the reader never syncs.
const ENDPOINT: &str = "http://127.0.0.1:9/";
const DAY_MS: i64 = 86_400_000;
/// The first read's instant: a study day's 05:00 in UTC.
const FIRST_READ: i64 = 21_000 * DAY_MS + 5 * 3_600_000;

fn integer(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("an integer: {value}"))
}

fn base_of(value: &Value) -> Option<WindowBase> {
    (!value.is_null()).then(|| WindowBase {
        floor: integer(&value["before_id_ms"]),
        count: integer(&value["count"]),
    })
}

#[test]
fn the_ingest_window_and_its_rebase_match_the_predecessors_golden() {
    golden::each_case("ingest.constants", |case| {
        let expected = &case.output;
        match case.input["name"].as_str() {
            Some("constants.INGEST_WINDOW_DAYS") => {
                assert_eq!(INGEST_WINDOW_DAYS, integer(expected));
            }
            Some("constants.INGEST_REBASE_DAYS") => {
                assert_eq!(INGEST_REBASE_DAYS, integer(expected));
            }
            // The probe's weights and modulus: the gate's test reads them.
            _ => {}
        }
    });
    golden::each_case("ingest_rebase", |case| {
        let base = base_of(&case.input["base"]);
        let now = UtcMillis::from_epoch_millis(integer(&case.input["now_ms"]));
        let recount = integer(&case.input["recount"]);
        let logs = Logs::default();
        let (after, floor, self_check) =
            log_capture::with_capture(logs.recorder(), || match rebase(base, now) {
                Rebase::Keep(kept) => (kept, None, None),
                Rebase::Recount { floor } => {
                    let (written, self_check) = rebased(base, floor, recount);
                    (written, Some(floor), self_check)
                }
            });
        let expected = &case.output;
        assert_eq!(Some(after), base_of(&expected["base"]), "{}", case.input);
        assert_eq!(
            floor,
            expected["recount_floor"].as_i64(),
            "the floor recounted before: {}",
            case.input
        );
        // A recount is written back; a kept base is not.
        assert_eq!(
            floor.map(|_| after),
            base_of(&expected["persisted"]),
            "{}",
            case.input
        );
        let warnings = integer(&expected["self_check_warnings"]);
        assert_eq!(
            i64::try_from(logs.warnings().len()).unwrap_or(i64::MAX),
            warnings,
            "the self-check's WARN: {}",
            case.input
        );
        assert_eq!(self_check.is_some(), warnings > 0, "{}", case.input);
    });
}

/// Two Law cards with study events before the first read's floor, a Maths card out of scope, and a
/// review inside the window. No review lies in the eight days the floor moves between the reads.
const CARDS: [PlannedCard; 3] = [
    PlannedCard {
        id: 2001,
        deck: "Law::Evidence",
        filtered: false,
    },
    PlannedCard {
        id: 2002,
        deck: "Law::Evidence",
        filtered: false,
    },
    PlannedCard {
        id: 2003,
        deck: "Maths",
        filtered: false,
    },
];

const fn days_before_the_first_read(days: i64, card: i64, kind: i64, ease: i64) -> PlannedReview {
    PlannedReview {
        id: FIRST_READ - days * DAY_MS,
        card,
        kind,
        ease,
    }
}

const REVIEWS: [PlannedReview; 9] = [
    days_before_the_first_read(500, 2001, 0, 3),
    days_before_the_first_read(480, 2001, 1, 3),
    days_before_the_first_read(470, 2001, 4, 0),
    days_before_the_first_read(460, 2001, 1, 1),
    days_before_the_first_read(450, 2002, 0, 3),
    days_before_the_first_read(440, 2002, 1, 4),
    days_before_the_first_read(451, 2003, 1, 3),
    days_before_the_first_read(445, 2003, 1, 3),
    days_before_the_first_read(10, 2001, 1, 3),
];

#[tokio::test]
async fn a_shrinking_pre_window_count_is_reported_by_the_self_check() {
    let fixture = Fixture::new(ENDPOINT);
    synthetic::build_planned(&fixture.copy(), &["Law", "Maths"], &CARDS, &REVIEWS);
    let db = fixture.db().await;
    let state = SqliteIngestState::new(db);
    let reader = synthetic::reader(&fixture.settings(), "Law", None, support::clock_at(0));
    let first_read = UtcMillis::from_epoch_millis(FIRST_READ);
    let first = read_window(&reader, &state, first_read)
        .await
        .expect("the first read");

    // A card with two of the five pre-window study events is deleted, and the next read is a rebase
    // period and a day later.
    synthetic::delete_cards(&fixture.copy(), &[2002]);
    let second_read = UtcMillis::from_epoch_millis(FIRST_READ + (INGEST_REBASE_DAYS + 1) * DAY_MS);
    let logs = Logs::default();
    let second = {
        let _logs = log_capture::hold_capture(logs.recorder());
        read_window(&reader, &state, second_read)
            .await
            .expect("the second read")
    };

    assert_eq!(
        second.self_check,
        Some(SelfCheck {
            stored: 5,
            recounted: 3
        }),
        "the recount after the deletion is lower than the stored count"
    );
    let warnings = logs.warnings();
    assert_eq!(warnings.len(), 1, "one WARN: {warnings:?}");
    assert_eq!(
        (warnings[0].field("stored"), warnings[0].field("recounted")),
        (Some("5"), Some("3"))
    );
    // The first read counted five study events of the Law cards before its floor, and read the one
    // after it; the second wrote the recount.
    assert_eq!(
        first.base,
        WindowBase {
            floor: fresh_floor(first_read),
            count: 5
        }
    );
    assert_eq!(first.self_check, None);
    assert_eq!(
        first
            .data
            .reviews
            .iter()
            .map(|review| review.id)
            .collect::<Vec<_>>(),
        [FIRST_READ - 10 * DAY_MS]
    );
    let written = WindowBase {
        floor: fresh_floor(second_read),
        count: 3,
    };
    assert_eq!(second.base, written);
    assert_eq!(
        state.load().await.expect("the state reads").window_base,
        Some(written)
    );
}
