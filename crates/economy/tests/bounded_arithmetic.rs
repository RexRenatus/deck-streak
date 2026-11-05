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

/// The six crates whose sources compute coins, prices, XP, levels, streaks, freezes, chests, pity
/// counters, tokens, sessions and the writing streak (R8).
const CRATES: [&str; 6] = [
    "economy",
    "xp",
    "progression",
    "quests",
    "streaks",
    "habits",
];

/// The attribute whose item the census blanks before it reads a site (R9).
const TEST_ATTRIBUTE: &str = "#[cfg(test)]";

/// The two-character operators, read before one character (longest match), and whether each is a
/// site (R10).
const PAIRS: [(&str, bool); 5] = [
    ("+=", true),
    ("-=", true),
    ("*=", true),
    ("->", false),
    ("=>", false),
];

/// The one-character operators (R10).
const BINARY: [char; 3] = ['+', '-', '*'];

/// The characters other than a word's that end an operand (R10).
const OPERAND_ENDS: [char; 3] = [')', ']', '?'];

/// The words after which a `+`, `-` or `*` is unary, a dereference or a pointer type (R10).
const KEYWORDS: [&str; 22] = [
    "return", "in", "if", "else", "match", "while", "let", "mut", "break", "move", "for", "loop",
    "where", "impl", "dyn", "const", "static", "ref", "as", "yield", "box", "unsafe",
];

/// The calls that sum a sequence (R10).
const SUMS: [&str; 2] = ["sum", "product"];

/// The prefixes of the calls written in a bounded form (R11).
const BOUNDED: [&str; 4] = ["checked_", "saturating_", "wrapping_", "overflowing_"];

/// A function's body: its name, and the span from its `{` to the character after its `}`.
struct Body {
    /// The function's name.
    name: String,
    /// The index of its `{`.
    start: usize,
    /// The index after its `}`.
    end: usize,
}

/// The sites of one source's text (R9 to R11).
fn sites_of(text: &str) -> Sites {
    let mut chars = blank(text);
    let bounded = bounded_calls(&chars);
    blank_test_items(&mut chars);
    let bodies = bodies(&chars);
    let mut sites = Sites {
        bounded,
        ..Sites::default()
    };
    for at in operator_sites(&chars) {
        match innermost(&bodies, at) {
            Some(body) => *sites.by_function.entry(body.name.clone()).or_default() += 1,
            None => sites.item_level += 1,
        }
    }
    sites
}

/// The census of the six crates' sources under `root` (R8, R13).
fn census(root: &Path) -> Result<Census, String> {
    let mut sources = Vec::new();
    for name in CRATES {
        sources_under(&root.join("crates").join(name).join("src"), &mut sources)?;
    }
    sources.sort();
    println!("examined {} source(s)", sources.len());
    if sources.is_empty() {
        return Err(
            "examined 0 source(s): the population is empty, which is VOID and never a pass"
                .to_owned(),
        );
    }
    let mut census = Census {
        examined: sources.len(),
        sites: BTreeMap::new(),
    };
    let (mut item_level, mut bounded) = (0, 0);
    for source in &sources {
        let path = source
            .strip_prefix(root)
            .unwrap_or(source)
            .display()
            .to_string();
        let text =
            fs::read_to_string(source).map_err(|error| format!("cannot read {path}: {error}"))?;
        let found = sites_of(&text);
        item_level += found.item_level;
        bounded += found.bounded;
        for (function, count) in found.by_function {
            *census.sites.entry((path.clone(), function)).or_default() += count;
        }
    }
    println!(
        "sites {} in {} function(s)",
        census.sites.values().sum::<usize>(),
        census.sites.len()
    );
    println!("item-level sites {item_level}, counted and never judged");
    println!("bounded calls {bounded}, counted and never judged");
    Ok(census)
}

/// Every `.rs` file under `dir`, at any depth; a directory that does not exist holds none.
fn sources_under(dir: &Path, sources: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }
    let unreadable = |error: std::io::Error| format!("cannot read {}: {error}", dir.display());
    for entry in fs::read_dir(dir).map_err(unreadable)? {
        let path = entry.map_err(unreadable)?.path();
        if path.is_dir() {
            sources_under(&path, sources)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
    Ok(())
}

/// Holds `sites` equal to `register`, function by function (R12). A refusal names its reason on
/// each of its lines.
fn judge(sites: &BTreeMap<(String, String), usize>, register: &[Listed]) -> Result<(), String> {
    let mut refusals = Vec::new();
    for ((source, function), &count) in sites {
        let listed = register
            .iter()
            .any(|row| row.source == source.as_str() && row.function == function.as_str());
        if !listed {
            refusals.push(format!("unlisted: {source} {function} {count}"));
        }
    }
    for row in register {
        let key = (row.source.to_owned(), row.function.to_owned());
        let found = sites.get(&key).copied().unwrap_or(0);
        if found == 0 {
            refusals.push(format!(
                "stale: {} {} {}",
                row.source, row.function, row.sites
            ));
        } else if found != row.sites {
            refusals.push(format!(
                "recounted: {} {} {found}, listed {}",
                row.source, row.function, row.sites
            ));
        }
    }
    if refusals.is_empty() {
        Ok(())
    } else {
        Err(refusals.join("\n"))
    }
}

/// The text with comments and string literals blanked to spaces and each character literal turned
/// into `0` and spaces, every line break kept (R9).
fn blank(text: &str) -> Vec<char> {
    let chars: Vec<char> = text.chars().collect();
    let mut blanked = chars.clone();
    let mut at = 0;
    while at < chars.len() {
        let next = chars.get(at + 1).copied();
        let (end, literal) = if chars[at] == '/' && next == Some('/') {
            (position(&chars, at, '\n').unwrap_or(chars.len()), false)
        } else if chars[at] == '/' && next == Some('*') {
            (comment_end(&chars, at), false)
        } else if let Some(end) = string_end(&chars, at) {
            (end, false)
        } else if let Some(end) = char_end(&chars, at) {
            (end, true)
        } else {
            at += 1;
            continue;
        };
        for c in &mut blanked[at..end] {
            if *c != '\n' {
                *c = ' ';
            }
        }
        if literal {
            blanked[at] = '0';
        }
        at = end;
    }
    blanked
}

/// The index after the `*/` that closes the comment opening at `open`, nested comments counted.
fn comment_end(chars: &[char], open: usize) -> usize {
    let mut depth = 0_usize;
    let mut at = open;
    while at < chars.len() {
        if starts_with(chars, at, "/*") {
            depth += 1;
            at += 2;
        } else if starts_with(chars, at, "*/") {
            depth -= 1;
            at += 2;
            if depth == 0 {
                return at;
            }
        } else {
            at += 1;
        }
    }
    chars.len()
}

/// The index after the string literal (plain, byte, C or raw) that starts at `at`, if one does.
fn string_end(chars: &[char], at: usize) -> Option<usize> {
    let mut cursor = at;
    if matches!(chars[at], 'b' | 'c' | 'r') && (at == 0 || !is_word(chars[at - 1])) {
        cursor += 1;
        if chars[at] != 'r' && chars.get(cursor) == Some(&'r') {
            cursor += 1;
        }
    }
    if cursor > at && chars[cursor - 1] == 'r' {
        let mut hashes = 0;
        while chars.get(cursor) == Some(&'#') {
            hashes += 1;
            cursor += 1;
        }
        if chars.get(cursor) != Some(&'"') {
            return None;
        }
        let close: Vec<char> = std::iter::once('"')
            .chain(std::iter::repeat_n('#', hashes))
            .collect();
        return Some(
            (cursor + 1..chars.len())
                .find(|&end| chars[end..].starts_with(&close))
                .map_or(chars.len(), |end| end + close.len()),
        );
    }
    if chars.get(cursor) != Some(&'"') {
        return None;
    }
    let mut scan = cursor + 1;
    while scan < chars.len() {
        match chars[scan] {
            '\\' => scan += 2,
            '"' => return Some(scan + 1),
            _ => scan += 1,
        }
    }
    Some(chars.len())
}

/// The index after the character or byte-character literal that starts at `at`, if one does; a
/// lifetime or a label is none.
fn char_end(chars: &[char], at: usize) -> Option<usize> {
    let mut quote = at;
    if chars[at] == 'b' && (at == 0 || !is_word(chars[at - 1])) && chars.get(at + 1) == Some(&'\'')
    {
        quote += 1;
    }
    if chars[quote] != '\'' {
        return None;
    }
    if chars.get(quote + 1) == Some(&'\\') {
        return position(chars, quote + 3, '\'').map(|close| close + 1);
    }
    (chars.get(quote + 2) == Some(&'\'')).then_some(quote + 3)
}

/// Blanks every `#[cfg(test)]` item, from the attribute to the first `;` at bracket depth 0 or to
/// the brace that matches the first `{`, whichever comes first (R9).
fn blank_test_items(chars: &mut [char]) {
    let attribute: Vec<char> = TEST_ATTRIBUTE.chars().collect();
    let mut at = 0;
    while at < chars.len() {
        if !chars[at..].starts_with(&attribute) {
            at += 1;
            continue;
        }
        let end = item_end(chars, at + attribute.len());
        for c in &mut chars[at..end] {
            if *c != '\n' {
                *c = ' ';
            }
        }
        at = end;
    }
}

/// The index after the item that follows an attribute ending at `from`.
fn item_end(chars: &[char], from: usize) -> usize {
    let mut depth = 0_i32;
    for (at, &c) in chars.iter().enumerate().skip(from) {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            ';' if depth == 0 => return at + 1,
            '{' => return matching(chars, at),
            _ => {}
        }
    }
    chars.len()
}

/// The index after the `}` that matches the `{` at `open`.
fn matching(chars: &[char], open: usize) -> usize {
    let mut depth = 0_usize;
    for (at, &c) in chars.iter().enumerate().skip(open) {
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
            if depth == 0 {
                return at + 1;
            }
        }
    }
    chars.len()
}

/// Every function body: the brace-matched block after `fn <name>`, its first `{` at parenthesis
/// and bracket depth 0 before any `;` at that depth (R10).
fn bodies(chars: &[char]) -> Vec<Body> {
    let mut bodies = Vec::new();
    for at in 0..chars.len() {
        let keyword = starts_with(chars, at, "fn")
            && (at == 0 || !is_word(chars[at - 1]))
            && chars.get(at + 2).is_some_and(|c| c.is_whitespace());
        if !keyword {
            continue;
        }
        let name_at = skip_space(chars, at + 2);
        let name = word_from(chars, name_at);
        if name.is_empty() {
            continue;
        }
        let mut depth = 0_i32;
        for (cursor, &c) in chars.iter().enumerate().skip(name_at + name.len()) {
            match c {
                '(' | '[' => depth += 1,
                ')' | ']' => depth -= 1,
                ';' if depth == 0 => break,
                '{' if depth == 0 => {
                    bodies.push(Body {
                        name,
                        start: cursor,
                        end: matching(chars, cursor),
                    });
                    break;
                }
                _ => {}
            }
        }
    }
    bodies
}

/// The innermost body that holds `at`, if any does.
fn innermost(bodies: &[Body], at: usize) -> Option<&Body> {
    bodies
        .iter()
        .filter(|body| body.start <= at && at < body.end)
        .max_by_key(|body| body.start)
}

/// The index of every unchecked operator site, by longest match (R10).
fn operator_sites(chars: &[char]) -> Vec<usize> {
    let mut sites = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        if let Some(&(_, site)) = PAIRS.iter().find(|(pair, _)| starts_with(chars, at, pair)) {
            if site {
                sites.push(at);
            }
            at += 2;
            continue;
        }
        let binary = BINARY.contains(&chars[at]) && ends_operand(chars, at) && !exponent(chars, at);
        if binary || (chars[at] == '.' && sum_call(chars, at)) {
            sites.push(at);
        }
        at += 1;
    }
    sites
}

/// Whether the character before `at` that is not white space ends an operand (R10).
fn ends_operand(chars: &[char], at: usize) -> bool {
    let Some(previous) = chars[..at].iter().rposition(|c| !c.is_whitespace()) else {
        return false;
    };
    if is_word(chars[previous]) {
        let word: String = chars[word_start(chars, previous)..=previous]
            .iter()
            .collect();
        return !KEYWORDS.contains(&word.as_str());
    }
    OPERAND_ENDS.contains(&chars[previous])
}

/// Whether the `+` or `-` at `at` is the sign of a decimal literal's exponent (R10).
fn exponent(chars: &[char], at: usize) -> bool {
    if !matches!(chars[at], '+' | '-') || at == 0 || !matches!(chars[at - 1], 'e' | 'E') {
        return false;
    }
    let word: String = chars[word_start(chars, at - 1)..at]
        .iter()
        .collect::<String>()
        .to_ascii_lowercase();
    word.starts_with(|c: char| c.is_ascii_digit())
        && !["0x", "0o", "0b"]
            .iter()
            .any(|prefix| word.starts_with(prefix))
}

/// Whether the `.` at `dot` opens a `.sum(` or `.product(` call, with or without a turbofish
/// (R10).
fn sum_call(chars: &[char], dot: usize) -> bool {
    let name = word_from(chars, dot + 1);
    if !SUMS.contains(&name.as_str()) {
        return false;
    }
    let mut cursor = skip_space(chars, dot + 1 + name.len());
    if starts_with(chars, cursor, "::<") {
        cursor = skip_space(chars, angle_end(chars, cursor + 2));
    }
    chars.get(cursor) == Some(&'(')
}

/// The index after the `>` that closes the `<` at `open`.
fn angle_end(chars: &[char], open: usize) -> usize {
    let mut depth = 0_usize;
    for (at, &c) in chars.iter().enumerate().skip(open) {
        if c == '<' {
            depth += 1;
        } else if c == '>' {
            depth -= 1;
            if depth == 0 {
                return at + 1;
            }
        }
    }
    chars.len()
}

/// How many `checked_`, `saturating_`, `wrapping_` and `overflowing_` calls the text holds (R11).
fn bounded_calls(chars: &[char]) -> usize {
    let mut calls = 0;
    let mut at = 0;
    while at < chars.len() {
        if !is_word(chars[at]) || (at > 0 && is_word(chars[at - 1])) {
            at += 1;
            continue;
        }
        let word = word_from(chars, at);
        let after = skip_space(chars, at + word.len());
        if BOUNDED.iter().any(|prefix| word.starts_with(prefix))
            && matches!(chars.get(after), Some('(' | ':'))
        {
            calls += 1;
        }
        at += word.len();
    }
    calls
}

/// Whether `c` is a character of a word.
fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The index where the word ending at `end` starts.
fn word_start(chars: &[char], end: usize) -> usize {
    let mut start = end;
    while start > 0 && is_word(chars[start - 1]) {
        start -= 1;
    }
    start
}

/// The word that starts at `at`, empty when none does.
fn word_from(chars: &[char], at: usize) -> String {
    chars
        .get(at..)
        .unwrap_or_default()
        .iter()
        .take_while(|&&c| is_word(c))
        .collect()
}

/// The first index at or after `at` that is not white space.
fn skip_space(chars: &[char], at: usize) -> usize {
    let mut cursor = at;
    while chars.get(cursor).is_some_and(|c| c.is_whitespace()) {
        cursor += 1;
    }
    cursor
}

/// Whether `text` starts at `at`.
fn starts_with(chars: &[char], at: usize, text: &str) -> bool {
    text.chars()
        .enumerate()
        .all(|(offset, c)| chars.get(at + offset) == Some(&c))
}

/// The next index at or after `from` that holds `c`.
fn position(chars: &[char], from: usize, c: char) -> Option<usize> {
    chars
        .get(from..)?
        .iter()
        .position(|&found| found == c)
        .map(|offset| from + offset)
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
