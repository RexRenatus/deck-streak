//! The freeze port and its drop gate (SPEC-076 A7, A8; R7 to R9): a freeze is held at most
//! three at a time, drop-style income is one a calendar month, and the shop is never a drop.

#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_kernel::StudyDay;
use deck_streak_streaks::freeze::{
    EpicPrize, FreezeEvent, FreezeReason, Refusal, admit, drops_in_month, pick_epic_prize,
};
use serde_json::Value;

fn day(number: i64) -> StudyDay {
    StudyDay::from_epoch_day(number)
}

fn int(value: &Value) -> i64 {
    value
        .as_i64()
        .unwrap_or_else(|| panic!("an integer: {value}"))
}

fn event(number: i64, delta: i32, reason: FreezeReason) -> FreezeEvent {
    FreezeEvent {
        day: day(number),
        delta,
        reason,
    }
}

#[test]
fn the_freeze_port_holds_the_hold_cap_and_the_monthly_drop_cap() {
    let today = day(20_000);
    // Under both caps a chest freeze is admitted.
    assert_eq!(admit(2, FreezeReason::Chest, today, &[]), Ok(()));
    // At the hold cap every reason is refused, the shop included, naming the hold.
    for reason in [FreezeReason::Chest, FreezeReason::Shop] {
        assert_eq!(
            admit(3, reason, today, &[]),
            Err(Refusal::HoldCap),
            "{reason:?} at the hold cap"
        );
    }
    // A drop this month refuses a second drop, naming the monthly cap.
    let drop = [event(19_997, 1, FreezeReason::WeeklyQuest)];
    assert_eq!(
        admit(1, FreezeReason::Season, today, &drop),
        Err(Refusal::MonthlyDropCap)
    );
    // The shop is capped by the hold alone.
    assert_eq!(admit(1, FreezeReason::Shop, today, &drop), Ok(()));
    // A drop last month does not count.
    let old = [event(19_994, 1, FreezeReason::Chest)];
    assert_eq!(admit(1, FreezeReason::Chest, today, &old), Ok(()));
    assert_eq!(drops_in_month(&drop, today), 1);
    assert_eq!(drops_in_month(&old, today), 0);
}

#[test]
fn the_drop_gate_matches_the_parity_golden() {
    let examined = golden::each_case("freeze_drop_gate", |case| {
        let held = u32::try_from(int(&case.input["held"])).expect("held");
        let events: Vec<FreezeEvent> = case.input["events"]
            .as_array()
            .expect("events")
            .iter()
            .map(|row| FreezeEvent {
                day: day(int(&row["day"])),
                delta: i32::try_from(int(&row["delta"])).expect("delta"),
                reason: FreezeReason::parse(row["reason"].as_str().expect("reason"))
                    .expect("a known reason"),
            })
            .collect();
        let asked = match case.input["choice"].as_str() {
            Some("freeze") => EpicPrize::Freeze,
            _ => EpicPrize::Token,
        };
        let paid = pick_epic_prize(asked, held, day(int(&case.input["today"])), &events);
        let want = case.output["choice"].as_str();
        let got = match paid {
            EpicPrize::Freeze => "freeze",
            EpicPrize::Token => "token",
        };
        assert_eq!(
            Some(got),
            want,
            "class {:?}, input {}",
            case.class,
            case.input
        );
    });
    println!("{examined}");
    for (name, drop) in [
        ("chest", true),
        ("weekly_quest", true),
        ("season", true),
        ("shop", false),
        ("streak_earn", false),
        ("consumed", false),
        ("streak_break", false),
    ] {
        let reason = FreezeReason::parse(name).expect("a known reason");
        assert_eq!(reason.is_drop(), drop, "{name}");
        assert_eq!(reason.as_str(), name);
    }
}
