//! The rarity roll, the Epic odds, the payout and the chest constants equal the predecessor's
//! (SPEC-081 A2, A3, A15).

// An integration test is test code: its helpers panic on a malformed golden, and the reader prints
// the examined count on purpose.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

use deck_streak_quests::chests::{self, Rarity};
use deck_streak_quests::{sessions, tokens};
use serde_json::{Value, json};

/// `economy.json`, embedded at build time.
const ECONOMY_FILE: &str = include_str!("../../../economy.json");

/// A whole number under `key`.
fn whole(value: &Value, key: &str) -> i64 {
    value[key].as_i64().expect("a whole number in the case")
}

/// A float under `key`.
fn real(value: &Value, key: &str) -> f64 {
    value[key].as_f64().expect("a number in the case")
}

#[test]
fn the_rarity_roll_matches_the_predecessors_golden() {
    let rolls = golden::each_case("roll_rarity", |case| {
        let ours = chests::roll_rarity(
            real(&case.input, "u"),
            whole(&case.input, "since_epic"),
            whole(&case.input, "since_legendary"),
            real(&case.input, "buff_pts"),
        );
        assert_eq!(
            json!(ours.name()),
            case.output,
            "the roll of {}",
            case.input
        );
    });
    let odds = golden::each_case("epic_odds_pts", |case| {
        let ours = chests::epic_odds_pts(
            whole(&case.input, "since_epic"),
            real(&case.input, "buff_pts"),
        );
        assert_eq!(json!(ours), case.output, "the Epic odds of {}", case.input);
    });
    assert!(
        rolls.count > 0 && odds.count > 0,
        "a roll golden examined nothing"
    );
}

#[test]
fn the_payout_matches_the_predecessors_golden() {
    let examined = golden::each_case("payout_xp", |case| {
        let name = case.input["rarity"].as_str().expect("a rarity");
        let rarity = Rarity::from_name(name);
        assert!(rarity.is_some(), "the port does not know the rarity {name}");
        let ours = chests::payout_xp(
            rarity.unwrap_or(Rarity::Common),
            real(&case.input, "u"),
            whole(&case.input, "session_xp"),
        );
        assert_eq!(json!(ours), case.output, "the payout of {}", case.input);
    });
    assert!(examined.count > 0, "the payout golden examined nothing");
}

/// The value of a dotted path in `economy.json`.
fn declared<'a>(file: &'a Value, path: &[&str]) -> &'a Value {
    path.iter().fold(file, |node, key| &node[*key])
}

/// Whether two numbers are the same value, whichever way JSON writes them.
fn same(left: &Value, right: &Value) -> bool {
    left.as_f64() == right.as_f64() && left.as_f64().is_some()
}

#[test]
fn the_chest_constants_equal_the_golden_and_economy_json() {
    let file: Value = serde_json::from_str(ECONOMY_FILE).expect("economy.json parses");
    let mut compared = 0_usize;
    let mut left_to_streaks = 0_usize;
    let examined = golden::each_case("chests.constants", |case| {
        let name = case.input["name"].as_str().expect("a constant's name");
        let ours: Value = match name {
            "gamification.chests.BASE_ODDS" => json!({
                "common": chests::BASE_ODDS_COMMON,
                "rare": chests::BASE_ODDS_RARE,
                "epic": chests::BASE_ODDS_EPIC,
                "legendary": chests::BASE_ODDS_LEGENDARY,
            }),
            "gamification.chests.EPIC_ODDS_CEILING_PCT" => json!(chests::EPIC_ODDS_CEILING_PCT),
            "gamification.chests.PITY_EPIC_RAMP_AFTER" => json!(chests::PITY_EPIC_RAMP_AFTER),
            "gamification.chests.PITY_EPIC_RAMP_PTS" => json!(chests::PITY_EPIC_RAMP_PTS),
            "gamification.chests.PITY_EPIC_GUARANTEE" => json!(chests::PITY_EPIC_GUARANTEE),
            "gamification.chests.PITY_LEGENDARY_GUARANTEE" => {
                json!(chests::PITY_LEGENDARY_GUARANTEE)
            }
            "gamification.chests.SESSION_GAP_MS" => json!(sessions::SESSION_GAP_MS),
            "gamification.chests.COMMON_XP" => json!([chests::COMMON_XP.0, chests::COMMON_XP.1]),
            "gamification.chests.RARE_XP" => json!([chests::RARE_XP.0, chests::RARE_XP.1]),
            "gamification.chests.LEGENDARY_XP" => json!(chests::LEGENDARY_XP),
            "gamification.chests.EPIC_FALLBACK_XP" => json!(chests::EPIC_FALLBACK_XP),
            "gamification.chests.PAYOUT_SESSION_FRAC" => json!(chests::PAYOUT_SESSION_FRAC),
            "constants.REAL_EFFORT_CHEST_MIN_DISTINCT" => {
                json!(sessions::REAL_EFFORT_CHEST_MIN_DISTINCT)
            }
            // The streaks context holds these two caps (SPEC-081 section 10, T9): quests declares
            // neither, so there is nothing here to compare.
            "constants.STREAK_FREEZE_CAP" | "constants.FREEZE_DROP_MONTHLY_CAP" => {
                left_to_streaks += 1;
                return;
            }
            other => panic!("a constant the golden holds and this test does not know: {other}"),
        };
        compared += 1;
        assert_eq!(ours, case.output, "the constant {name}");
    });
    assert_eq!(examined.count, compared + left_to_streaks);
    assert_eq!(left_to_streaks, 2, "the golden lists the two freeze caps");
    assert_eq!(compared, 13, "the chest constants the golden lists");

    // economy.json's chest values equal the engine's (SPEC-081 R17).
    let chest_pairs: [(&[&str], Value); 18] = [
        (&["odds_percent", "common"], json!(chests::BASE_ODDS_COMMON)),
        (&["odds_percent", "rare"], json!(chests::BASE_ODDS_RARE)),
        (&["odds_percent", "epic"], json!(chests::BASE_ODDS_EPIC)),
        (
            &["odds_percent", "legendary"],
            json!(chests::BASE_ODDS_LEGENDARY),
        ),
        (
            &["pity", "epic_ramp_after"],
            json!(chests::PITY_EPIC_RAMP_AFTER),
        ),
        (
            &["pity", "epic_ramp_points"],
            json!(chests::PITY_EPIC_RAMP_PTS),
        ),
        (
            &["pity", "epic_ceiling_points"],
            json!(chests::EPIC_ODDS_CEILING_PCT),
        ),
        (
            &["pity", "epic_guaranteed_at"],
            json!(chests::PITY_EPIC_GUARANTEE),
        ),
        (
            &["pity", "legendary_guaranteed_at"],
            json!(chests::PITY_LEGENDARY_GUARANTEE),
        ),
        (
            &["effort_floor_distinct_cards"],
            json!(sessions::REAL_EFFORT_CHEST_MIN_DISTINCT),
        ),
        (&["payout_xp", "common_min"], json!(chests::COMMON_XP.0)),
        (&["payout_xp", "common_max"], json!(chests::COMMON_XP.1)),
        (&["payout_xp", "rare_min"], json!(chests::RARE_XP.0)),
        (&["payout_xp", "rare_max"], json!(chests::RARE_XP.1)),
        (
            &["payout_xp", "cap_floor"],
            json!(chests::PAYOUT_CAP_FLOOR_XP),
        ),
        (
            &["payout_xp", "cap_session_fraction"],
            json!(chests::PAYOUT_SESSION_FRAC),
        ),
        (&["payout_xp", "legendary"], json!(chests::LEGENDARY_XP)),
        (
            &["payout_xp", "epic_fallback"],
            json!(chests::EPIC_FALLBACK_XP),
        ),
    ];
    for (path, ours) in &chest_pairs {
        assert!(
            same(declared(&file["chests"], path), ours),
            "economy.json chests.{} is {}, the engine's is {ours}",
            path.join("."),
            declared(&file["chests"], path)
        );
    }
    assert_eq!(
        real(declared(&file, &["chests"]), "session_gap_minutes") * 60_000.0,
        json!(sessions::SESSION_GAP_MS).as_f64().expect("a number"),
        "economy.json's session gap in minutes is the engine's gap in milliseconds"
    );

    // The token's two constants have no golden: economy.json alone holds them (SPEC-081 section 1).
    let token = declared(&file, &["xp", "bonuses", "double_xp_token"]);
    assert!(same(
        &token["window_hours"],
        &json!(tokens::TOKEN_WINDOW_HOURS)
    ));
    assert!(same(&token["cap"], &json!(tokens::TOKEN_BONUS_CAP_XP)));
}
