//! SPEC-435 A6 to A9 (R8 to R13, ADR-449 D2): every unchecked operator in the money and counter
//! paths is listed with its bound.
//!
//! The population is every `.rs` file under `src/` of the six crates that compute coins, prices,
//! XP, levels, streaks, freezes, chests, pity counters, tokens, sessions and the writing streak,
//! read as text, and the census names no file to skip. It blanks `//` comments, nested `/* */`
//! comments and string literals to spaces, turns a character literal into `0` and spaces so it
//! reads as an operand, keeps every line break, and then blanks every `#[cfg(test)]` item.
//!
//! A site is a `+=`, `-=` or `*=`; a `+`, `-` or `*` whose previous character that is not white
//! space ends an operand (a letter, a digit, `_`, `)`, `]` or `?`) and whose previous word, when it
//! ends one, is no keyword of SPEC-435 R10; or a `.sum(` or `.product(` call, with or without a
//! turbofish. `->` and `=>` are never sites, and the sign of a decimal literal's exponent is part
//! of the literal. A site inside a function body is named by its innermost function; a site
//! outside every body is item level, counted and never judged. `checked_`, `saturating_`,
//! `wrapping_` and `overflowing_` calls are counted and never judged.
//!
//! The register is [`LISTED`]: one row per source and function, with its count of sites, its class
//! and its bound in words. The census holds its sites equal to the register function by function,
//! and refuses a function no row lists, a row whose count differs and a row it does not reach. It
//! prints `examined N source(s)` and its counts, never a line's text, and refuses zero examined,
//! which is VOID. Every red is planted in `tests/fixtures/bounded_arithmetic/plants.rs.fixture` or
//! in a temporary tree, never in the product tree; the tree test reads the six crates and runs in
//! CI.

// An integration test is test code: its helpers panic on an unreadable file, and it prints the
// examined count on purpose. clippy.toml's in-test allowances cover only `#[test]` bodies.
#![allow(clippy::expect_used, clippy::print_stdout)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// What bounds a listed function's sites, as a reviewer reads it (R12).
enum Class {
    /// An operand is a float, which the text cannot type.
    Float,
    /// Each operand is widened before the sum.
    Widened,
    /// A constant, cap, ladder or guarantee the code enforces bounds it.
    Domain,
    /// A real date or time bounds it.
    Calendar,
    /// A count over a finite set bounds it.
    Count,
}

/// One row of the register: one function's sites, and what bounds them.
#[expect(
    dead_code,
    reason = "a reviewer reads each row's class and bound; the census reads its counts"
)]
struct Listed {
    /// The source, from the repository's root.
    source: &'static str,
    /// The function, by its name.
    function: &'static str,
    /// How many unchecked operator sites the function holds.
    sites: usize,
    /// What bounds them.
    class: Class,
    /// The bound, in words.
    bound: &'static str,
}

/// Every unchecked operator site of the six crates, by its function (SPEC-435 R12).
const LISTED: [Listed; 46] = [
    Listed {
        source: "crates/economy/src/rules.rs",
        function: "daily_loss_cap",
        sites: 1,
        class: Class::Float,
        bound: "a float share of the wallet",
    },
    Listed {
        source: "crates/economy/src/rules.rs",
        function: "scaled_fine",
        sites: 1,
        class: Class::Float,
        bound: "a float share of the wallet",
    },
    Listed {
        source: "crates/economy/src/wallet.rs",
        function: "debit_capped_on",
        sites: 2,
        class: Class::Domain,
        bound: "the day's cap of 0 to 100 less the day's non-negative debits; a balance less its floor of 0",
    },
    Listed {
        source: "crates/economy/src/wallet.rs",
        function: "debit_floored_on",
        sites: 1,
        class: Class::Domain,
        bound: "a balance less its floor of 0",
    },
    Listed {
        source: "crates/economy/src/wallet.rs",
        function: "movements",
        sites: 1,
        class: Class::Domain,
        bound: "a page constant plus 1",
    },
    Listed {
        source: "crates/economy/src/wallet.rs",
        function: "purchase_on",
        sites: 1,
        class: Class::Domain,
        bound: "a positive price from a balance kept at or above its floor (`formal/tla/WalletFloor`)",
    },
    Listed {
        source: "crates/economy/src/wallet.rs",
        function: "settle_mint_on",
        sites: 2,
        class: Class::Domain,
        bound: "a balance less its floor of 0; the day's mint of at most 40 less a non-negative room",
    },
    Listed {
        source: "crates/habits/src/minutes.rs",
        function: "week_end",
        sites: 2,
        class: Class::Calendar,
        bound: "a week's day numbers",
    },
    Listed {
        source: "crates/habits/src/minutes.rs",
        function: "week_start",
        sites: 2,
        class: Class::Calendar,
        bound: "a week's day numbers",
    },
    Listed {
        source: "crates/habits/src/writing.rs",
        function: "streak_ending",
        sites: 3,
        class: Class::Count,
        bound: "the days of a finite set, walked back by a cursor",
    },
    Listed {
        source: "crates/progression/src/badges/hours.rs",
        function: "local_hour",
        sites: 2,
        class: Class::Widened,
        bound: "widened to `i128`",
    },
    Listed {
        source: "crates/progression/src/consistency.rs",
        function: "consistency_multiplier",
        sites: 2,
        class: Class::Float,
        bound: "a float multiplier",
    },
    Listed {
        source: "crates/progression/src/consistency.rs",
        function: "day_base_xp",
        sites: 1,
        class: Class::Widened,
        bound: "`u32` terms widened to `i64`, over one day's rows",
    },
    Listed {
        source: "crates/progression/src/consistency.rs",
        function: "day_bonuses",
        sites: 4,
        class: Class::Count,
        bound: "two review counts of one day summed; the rest float",
    },
    Listed {
        source: "crates/progression/src/consistency.rs",
        function: "projected_multiplier_drop",
        sites: 1,
        class: Class::Domain,
        bound: "a run of at least 0 less a configured step, floored",
    },
    Listed {
        source: "crates/progression/src/consistency.rs",
        function: "tier_down_run",
        sites: 2,
        class: Class::Count,
        bound: "a fold over a finite list, 1 per day; less a step floored at 0",
    },
    Listed {
        source: "crates/progression/src/exchange.rs",
        function: "exchange_rates",
        sites: 2,
        class: Class::Count,
        bound: "sums of per-row XP and per-day graduations over the rows read",
    },
    Listed {
        source: "crates/progression/src/level.rs",
        function: "level_info",
        sites: 3,
        class: Class::Domain,
        bound: "100 times a level; a total less its level's start, which is at most the total",
    },
    Listed {
        source: "crates/progression/src/milestone.rs",
        function: "candidate",
        sites: 2,
        class: Class::Domain,
        bound: "a target less a value below it; the share is float",
    },
    Listed {
        source: "crates/progression/src/milestone.rs",
        function: "next_milestone",
        sites: 1,
        class: Class::Domain,
        bound: "a non-empty constant ladder's length less 1",
    },
    Listed {
        source: "crates/progression/src/milestone.rs",
        function: "smaller",
        sites: 4,
        class: Class::Domain,
        bound: "the ladder's targets and the progress",
    },
    Listed {
        source: "crates/progression/src/records.rs",
        function: "chase",
        sites: 1,
        class: Class::Domain,
        bound: "two non-negative counts",
    },
    Listed {
        source: "crates/progression/src/xp.rs",
        function: "level_for",
        sites: 6,
        class: Class::Widened,
        bound: "widened to `u128`",
    },
    Listed {
        source: "crates/progression/src/xp.rs",
        function: "xp_to_reach",
        sites: 3,
        class: Class::Domain,
        bound: "a level from `level_for`, whose start is at most the total",
    },
    Listed {
        source: "crates/quests/src/chests.rs",
        function: "epic_odds_pts",
        sites: 4,
        class: Class::Domain,
        bound: "a counter below its guarantee less a constant; float odds",
    },
    Listed {
        source: "crates/quests/src/chests.rs",
        function: "grant_session_chests_on",
        sites: 2,
        class: Class::Count,
        bound: "a session's start difference in milliseconds; chests held at most the day's cap",
    },
    Listed {
        source: "crates/quests/src/chests.rs",
        function: "payout_xp",
        sites: 5,
        class: Class::Domain,
        bound: "a constant payout range; float shares",
    },
    Listed {
        source: "crates/quests/src/chests.rs",
        function: "roll_rarity",
        sites: 6,
        class: Class::Domain,
        bound: "the pity counters below their guarantees plus 1; float odds",
    },
    Listed {
        source: "crates/quests/src/chests.rs",
        function: "sweep_stale_chests_on",
        sites: 1,
        class: Class::Calendar,
        bound: "a real time in milliseconds",
    },
    Listed {
        source: "crates/quests/src/draw.rs",
        function: "fraction",
        sites: 1,
        class: Class::Float,
        bound: "a float fraction",
    },
    Listed {
        source: "crates/quests/src/pity.rs",
        function: "after",
        sites: 4,
        class: Class::Domain,
        bound: "each counter stays below its guarantee: an Epic by the 14th, a Legendary by the 40th, reset by `roll_rarity`",
    },
    Listed {
        source: "crates/quests/src/sessions.rs",
        function: "measure",
        sites: 2,
        class: Class::Count,
        bound: "the reviews of one session; seconds are float",
    },
    Listed {
        source: "crates/quests/src/sessions.rs",
        function: "session_bounds",
        sites: 1,
        class: Class::Calendar,
        bound: "a real time in milliseconds",
    },
    Listed {
        source: "crates/quests/src/tokens.rs",
        function: "activate_token_on",
        sites: 2,
        class: Class::Calendar,
        bound: "the time now in milliseconds plus a constant window",
    },
    Listed {
        source: "crates/streaks/src/calendar.rs",
        function: "first_day",
        sites: 3,
        class: Class::Calendar,
        bound: "a real day number",
    },
    Listed {
        source: "crates/streaks/src/calendar.rs",
        function: "language",
        sites: 1,
        class: Class::Calendar,
        bound: "a real day number",
    },
    Listed {
        source: "crates/streaks/src/calendar.rs",
        function: "law",
        sites: 1,
        class: Class::Calendar,
        bound: "a real day number",
    },
    Listed {
        source: "crates/streaks/src/freeze.rs",
        function: "civil_month",
        sites: 15,
        class: Class::Calendar,
        bound: "the civil date of a real day number",
    },
    Listed {
        source: "crates/streaks/src/freeze.rs",
        function: "freeze_events_for",
        sites: 2,
        class: Class::Widened,
        bound: "`u32` freezes widened to `i64`",
    },
    Listed {
        source: "crates/streaks/src/governor.rs",
        function: "silence_walk",
        sites: 1,
        class: Class::Calendar,
        bound: "a bounded walk back over real days",
    },
    Listed {
        source: "crates/streaks/src/governor.rs",
        function: "standby_notice",
        sites: 1,
        class: Class::Calendar,
        bound: "a real day number",
    },
    Listed {
        source: "crates/streaks/src/law.rs",
        function: "bridged_streak",
        sites: 1,
        class: Class::Calendar,
        bound: "a real day number",
    },
    Listed {
        source: "crates/streaks/src/store.rs",
        function: "add_freeze",
        sites: 1,
        class: Class::Domain,
        bound: "`admit` refuses at the cap of 3 in the same write",
    },
    Listed {
        source: "crates/streaks/src/strength.rs",
        function: "advance",
        sites: 4,
        class: Class::Float,
        bound: "float strengths",
    },
    Listed {
        source: "crates/streaks/src/strength.rs",
        function: "persists",
        sites: 1,
        class: Class::Calendar,
        bound: "a real day number",
    },
    Listed {
        source: "crates/xp/src/review_xp.rs",
        function: "review_xp",
        sites: 5,
        class: Class::Domain,
        bound: "`ease - 1` inside 1 to 4; four float products",
    },
];

/// The sites one source holds (R10, R11).
#[derive(Debug, Default)]
struct Sites {
    /// Each function's unchecked operator sites, by its name.
    by_function: BTreeMap<String, usize>,
    /// Sites outside every function body: counted, never judged.
    item_level: usize,
    /// `checked_`, `saturating_`, `wrapping_` and `overflowing_` calls: counted, never judged.
    bounded: usize,
}

/// What the census read.
#[derive(Debug, Default)]
struct Census {
    /// How many sources it read.
    examined: usize,
    /// Each function's sites, by its source and its name.
    sites: BTreeMap<(String, String), usize>,
}

/// The sites of one source's text (R9 to R11).
fn sites_of(text: &str) -> Sites {
    let _ = text;
    Sites::default()
}

/// The census of the six crates' sources under `root` (R8, R13).
#[expect(
    clippy::unnecessary_wraps,
    reason = "the red commit's stub: it lists and reads nothing, finds nothing and refuses nothing"
)]
fn census(root: &Path) -> Result<Census, String> {
    let _ = root;
    Ok(Census::default())
}

/// Holds `sites` equal to `register`, function by function (R12). A refusal names its reason on
/// each of its lines.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the red commit's stub: it accepts whatever it is given"
)]
fn judge(sites: &BTreeMap<(String, String), usize>, register: &[Listed]) -> Result<(), String> {
    let _ = (sites, register);
    Ok(())
}

/// The repository's root, two levels above this crate.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The register's rows by source and function, with the count each lists.
fn listed(register: &[Listed]) -> BTreeMap<(String, String), usize> {
    register
        .iter()
        .map(|row| ((row.source.to_owned(), row.function.to_owned()), row.sites))
        .collect()
}

#[test]
fn the_matcher_finds_each_unchecked_operator_in_a_planted_source_and_nothing_else() {
    let plants = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/bounded_arithmetic/plants.rs.fixture");
    let text = fs::read_to_string(&plants).expect("the planted source is readable");
    let sites = sites_of(&text);
    let by_function: Vec<(&str, usize)> = sites
        .by_function
        .iter()
        .map(|(function, &count)| (function.as_str(), count))
        .collect();

    assert_eq!(
        by_function,
        [
            ("binary", 6),
            ("compound", 3),
            ("inner", 1),
            ("literals", 1),
            ("outer", 1),
            ("sums", 2),
        ],
        "each planted site by its innermost function, and none in `nothing` or the test module"
    );
    assert_eq!(
        sites.item_level, 1,
        "the planted constant expression is item level"
    );
    assert_eq!(
        sites.bounded, 1,
        "the planted `checked_add` call is counted"
    );
}

#[test]
fn an_unlisted_recounted_or_stale_fn_is_refused() {
    let sites = BTreeMap::from([
        (
            ("crates/planted/src/lib.rs".to_owned(), "settle".to_owned()),
            2,
        ),
        (
            ("crates/planted/src/lib.rs".to_owned(), "spend".to_owned()),
            1,
        ),
    ]);
    let row = |function: &'static str, sites: usize| Listed {
        source: "crates/planted/src/lib.rs",
        function,
        sites,
        class: Class::Domain,
        bound: "a planted bound",
    };

    assert_eq!(
        judge(&sites, &[row("settle", 2), row("spend", 1)]),
        Ok(()),
        "the positive control: every function listed with its count"
    );
    assert_eq!(
        judge(&sites, &[row("settle", 2)]).err().as_deref(),
        Some("unlisted: crates/planted/src/lib.rs spend 1"),
        "a function no row lists"
    );
    assert_eq!(
        judge(&sites, &[row("settle", 3), row("spend", 1)])
            .err()
            .as_deref(),
        Some("recounted: crates/planted/src/lib.rs settle 2, listed 3"),
        "a row whose count differs"
    );
    assert_eq!(
        judge(&sites, &[row("settle", 2), row("spend", 1), row("gone", 1)])
            .err()
            .as_deref(),
        Some("stale: crates/planted/src/lib.rs gone 1"),
        "a row the census does not reach"
    );
}

#[test]
fn every_unchecked_operator_in_the_money_and_counter_paths_is_listed_with_its_bound() {
    let census = census(&root()).expect("the census reads the six crates");

    assert_eq!(
        judge(&census.sites, &LISTED),
        Ok(()),
        "every unchecked operator site is listed with its bound"
    );
    assert_eq!(
        census.sites,
        listed(&LISTED),
        "the sites equal LISTED, function by function"
    );
    assert_eq!(census.examined, 57, "the six crates hold 57 sources");
}

#[test]
fn an_empty_population_is_refused() {
    let crates = [
        "economy",
        "xp",
        "progression",
        "quests",
        "streaks",
        "habits",
    ];
    let empty = tempfile::tempdir().expect("a temporary directory");
    for name in crates {
        fs::create_dir_all(empty.path().join("crates").join(name).join("src"))
            .expect("an empty src/ is made");
    }

    assert_eq!(
        census(empty.path()).err().as_deref(),
        Some("examined 0 source(s): the population is empty, which is VOID and never a pass"),
        "six crates with no source are refused"
    );
    let planted = tempfile::tempdir().expect("a temporary directory");
    for name in crates {
        let src = planted.path().join("crates").join(name).join("src");
        fs::create_dir_all(&src).expect("src/ is made");
        fs::write(
            src.join("lib.rs"),
            "fn planted(a: u32) -> u32 {\n    a + 1\n}\n",
        )
        .expect("a planted source is written");
    }
    let census = census(planted.path()).expect("the census reads the planted sources");
    assert_eq!(
        (census.examined, census.sites.len()),
        (6, 6),
        "one source in each of the six crates is read, each with its planted function"
    );
}
