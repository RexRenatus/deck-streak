//! The law block (SPEC-077 R10 to R13; A12 to A15): its fields over the law streak, both XP
//! tables, the stored law dues and the leech port equal the predecessor's law track summary; it is
//! omitted without law activity and its lines follow the predecessor's shown rule; the dues are
//! pending before the first recompute stores them, and the leeches and the pillar are pending until
//! the leech port is wired. Every number here is synthetic.

// An integration test is test code: its helpers panic on a malformed golden or a failed fixture,
// and it prints the examined counts.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "../../../tools/parity-oracle/golden.rs"]
mod golden;

#[path = "awards_support/mod.rs"]
#[allow(dead_code)]
mod support;

use deck_streak_coordination::law::{LawBlock, LawLine, LawLines, law_block};
use deck_streak_curriculum::law::LawDues;
use deck_streak_curriculum::store::put_law_dues;
use deck_streak_kernel::{Db, UtcMillis};
use deck_streak_streaks::store::upsert_state;
use deck_streak_streaks::streak::StreakState;
use serde_json::Value;
use support::{D0, at, day, scratch};

/// Stores the law streak as `current` days long, last studied on `today`.
async fn seed_law_streak(db: &Db, current: u32, today: i64) {
    let mut write = db.write().await.expect("a write");
    let state = StreakState {
        current,
        longest: current,
        freezes: 0,
        last_study_day: Some(day(today)),
        comeback_armed: false,
    };
    upsert_state(
        &mut write,
        "law",
        &state,
        UtcMillis::from_epoch_millis(at(today, 12)),
    )
    .await
    .expect("the law streak writes");
    write.commit().await.expect("the law streak commits");
}

/// Stores the law dues for `today` as `backlog` overdue and `due_today` due.
async fn seed_dues(db: &Db, today: i64, backlog: u32, due_today: u32) {
    let mut write = db.write().await.expect("a write");
    put_law_dues(
        &mut write,
        &LawDues {
            study_day: day(today),
            backlog,
            due_today,
        },
        UtcMillis::from_epoch_millis(at(today, 12)),
    )
    .await
    .expect("the law dues write");
    write.commit().await.expect("the law dues commit");
}

/// Writes one XP row: an even `index` goes to the grants' table and an odd one to the settled
/// table, so the block is read over both. Answers whether the table took the row: neither takes a
/// negative amount (CHARTER 5).
async fn seed_xp(db: &Db, index: usize, study_day: i64, track: &str, amount: i64) -> bool {
    let source = format!("law_block_case_row_{index}");
    let mut write = db.write().await.expect("a write");
    let sql = if index.is_multiple_of(2) {
        "INSERT INTO xp_ledger (study_day, source, track, amount, scope, created_at) \
         VALUES (?1, ?2, ?3, ?4, 'per-day', ?5)"
    } else {
        "INSERT INTO xp_settlement (study_day, source, track, amount, closed, created_at) \
         VALUES (?1, ?2, ?3, ?4, 0, ?5)"
    };
    let written = sqlx::query(sql)
        .bind(study_day)
        .bind(source)
        .bind(track)
        .bind(amount)
        .bind(at(study_day, 12))
        .execute(&mut *write)
        .await;
    match written {
        Ok(_) => {
            write.commit().await.expect("the XP row commits");
            true
        }
        Err(_) => false,
    }
}

/// A count of the golden, or none for null or a value that is not a whole number.
fn whole(value: &Value) -> Option<i64> {
    value.as_i64()
}

/// The golden's line names, as this block's lines.
fn golden_lines(value: &Value) -> Vec<LawLine> {
    value
        .as_array()
        .expect("a list of fields")
        .iter()
        .map(|field| match field.as_str().expect("a field name") {
            "total_xp" => LawLine::TotalXp,
            "streak" => LawLine::Streak,
            "xp_today" => LawLine::XpToday,
            "dues" => LawLine::Dues,
            "mastery" => LawLine::Mastery,
            "leeches" => LawLine::Leeches,
            other => panic!("the golden names a field this block has no line for: {other}"),
        })
        .collect()
}

/// A12: every case's store seeded as the case's stub store holds it, read back through the block.
#[tokio::test]
async fn the_law_block_matches_the_predecessors_golden() {
    let summary = golden::read(&golden::committed("law_track_summary")).expect("the golden reads");
    let mut refused_cases = Vec::new();
    for (number, case) in summary.cases.iter().enumerate() {
        let input = &case.input;
        let today = input["today"].as_i64().expect("a study day");
        let scratch = scratch().await;
        let db = &scratch.db;
        let streak = u32::try_from(input["streak"].as_u64().expect("a streak")).expect("fits");
        seed_law_streak(db, streak, today).await;
        let mut refused_today = 0;
        let mut refused_total = 0;
        for (index, row) in input["ledger"]
            .as_array()
            .expect("ledger rows")
            .iter()
            .enumerate()
        {
            let row_day = row["day"].as_i64().expect("a row day");
            let amount = row["amount"].as_i64().expect("an amount");
            let track = row["track"].as_str().expect("a track");
            if !seed_xp(db, index, row_day, track, amount).await {
                assert!(amount < 0, "case {number}: a non-negative row was refused");
                if track == "law" {
                    refused_total += amount;
                    if row_day == today {
                        refused_today += amount;
                    }
                }
            }
        }
        if let Some(dues) = whole(&input["dues"]) {
            let dues = u32::try_from(dues).expect("a dues count");
            seed_dues(db, today, dues / 2, dues - dues / 2).await;
        }
        let law_leeches = input["leeches"]
            .as_array()
            .expect("leech rows")
            .iter()
            .filter(|track| track.as_str() == Some("law"))
            .count();
        let leeches = u32::try_from(law_leeches).expect("fits");
        let block = law_block(db, day(today), Some(leeches))
            .await
            .expect("the block reads");
        let theirs = &case.output;
        // SPEC-077 T17: no XP table here holds a negative amount, so the case whose ledger holds one
        // reads its XP without the refused rows.
        if refused_total != 0 || refused_today != 0 {
            refused_cases.push(case.class.clone().unwrap_or_default());
        }
        let total = theirs["total_xp"].as_i64().expect("a total") - refused_total;
        let xp_today = theirs["xp_today"].as_i64().expect("a day's XP") - refused_today;
        assert_eq!(block.total_xp, total, "case {number}: lifetime law XP");
        assert_eq!(block.xp_today, xp_today, "case {number}: today's law XP");
        assert_eq!(
            Some(block.level),
            theirs["level"].as_i64(),
            "case {number}: the level of the lifetime law XP"
        );
        assert_eq!(
            Some(block.streak),
            theirs["streak"].as_i64(),
            "case {number}: the law streak"
        );
        assert_eq!(
            block.dues,
            theirs["dues"].as_i64(),
            "case {number}: the dues"
        );
        assert_eq!(
            block.leech_active,
            theirs["leech_active"].as_i64(),
            "case {number}: the active law leeches"
        );
        let mastery = block
            .mastery
            .expect("the pillar, with the leeches answered");
        let pillar = theirs["mastery"].as_f64().expect("a pillar");
        assert!(
            (mastery - pillar).abs() <= 1e-9,
            "case {number}: the pillar, ours {mastery}, theirs {pillar}"
        );
    }
    assert_eq!(
        refused_cases,
        vec!["negative_ledger".to_owned()],
        "only the golden's negative ledger holds a row no XP table takes"
    );
    println!("examined {} law_track_summary case(s)", summary.cases.len());
    assert!(!summary.cases.is_empty(), "examined 0 cases");
}

/// The block a payload of the shown rule's golden describes: a missing or null count reads as
/// zero, and a missing or null dues, leech count or pillar as pending.
fn payload_block(payload: &Value) -> LawBlock {
    let zero = |key: &str| payload[key].as_i64().unwrap_or(0);
    LawBlock {
        streak: zero("streak"),
        xp_today: zero("xp_today"),
        total_xp: zero("total_xp"),
        level: zero("level"),
        dues: payload["dues"].as_i64(),
        leech_active: payload["leech_active"].as_i64(),
        mastery: payload["mastery"].as_f64(),
    }
}

/// A13: omitted without activity over a real store, then every case of the shown rule's golden.
#[tokio::test]
async fn the_law_block_is_omitted_without_law_activity() {
    let scratch = scratch().await;
    let db = &scratch.db;
    seed_dues(db, D0, 9, 3).await;
    seed_law_streak(db, 0, D0).await;
    seed_xp(db, 0, D0, "language", 400).await;
    let quiet = law_block(db, day(D0), Some(0))
        .await
        .expect("the block reads");
    assert_eq!(
        quiet.lines(),
        None,
        "no law streak, law XP or leech: the block is omitted, dues and language XP or not"
    );
    seed_law_streak(db, 2, D0).await;
    let active = law_block(db, day(D0), Some(0))
        .await
        .expect("the block reads");
    assert_eq!(
        active.lines(),
        Some(LawLines {
            lines: vec![LawLine::Streak, LawLine::Dues],
            level_shown: false,
        }),
        "a law streak shows the block, with the dues above zero"
    );
    let shown = golden::read(&golden::committed("law_block_shown")).expect("the golden reads");
    for (number, case) in shown.cases.iter().enumerate() {
        let block = payload_block(&case.input["payload"]);
        let theirs = &case.output;
        let rendered = theirs["rendered"].as_bool().expect("rendered or not");
        let expected = rendered.then(|| LawLines {
            lines: golden_lines(&theirs["fields"]),
            level_shown: theirs["level_shown"].as_bool().expect("level shown or not"),
        });
        assert_eq!(block.lines(), expected, "case {number}: {:?}", case.class);
        if !rendered {
            assert_eq!(theirs["fields"], Value::Array(Vec::new()), "case {number}");
        }
    }
    println!("examined {} law_block_shown case(s)", shown.cases.len());
    assert!(!shown.cases.is_empty(), "examined 0 cases");
}

/// A14: no dues row is pending, never 0; a stored 0 reads as 0 and a stored count as its sum.
#[tokio::test]
async fn law_dues_are_pending_before_the_first_recompute() {
    let scratch = scratch().await;
    let db = &scratch.db;
    seed_law_streak(db, 1, D0).await;
    let before = law_block(db, day(D0), None).await.expect("the block reads");
    assert_eq!(
        before.dues, None,
        "before the first recompute the dues are pending"
    );
    assert_eq!(
        before.lines().expect("a law streak shows the block").lines,
        vec![LawLine::Streak],
        "pending dues draw no dues line"
    );
    seed_dues(db, D0, 0, 0).await;
    let zero = law_block(db, day(D0), None).await.expect("the block reads");
    assert_eq!(zero.dues, Some(0), "a recompute that counted none stores 0");
    seed_dues(db, D0, 3, 4).await;
    let counted = law_block(db, day(D0), None).await.expect("the block reads");
    assert_eq!(
        counted.dues,
        Some(7),
        "the dues are the backlog plus the cards due today"
    );
}

/// A15: the leeches and the pillar stay pending while the leech port answers nothing.
#[tokio::test]
async fn law_leeches_are_pending_until_the_leech_port_is_wired() {
    let scratch = scratch().await;
    let db = &scratch.db;
    seed_law_streak(db, 2, D0).await;
    seed_xp(db, 0, D0, "law", 30).await;
    let unwired = law_block(db, day(D0), None).await.expect("the block reads");
    assert_eq!(
        unwired.leech_active, None,
        "the leeches are pending, never 0"
    );
    assert_eq!(
        unwired.mastery, None,
        "the pillar is not computed while they are pending"
    );
    assert_eq!(
        unwired.lines().expect("law activity shows the block").lines,
        vec![LawLine::TotalXp, LawLine::Streak, LawLine::XpToday],
        "pending leeches draw neither the pillar's line nor the leeches' line"
    );
    let none = law_block(db, day(D0), Some(0))
        .await
        .expect("the block reads");
    assert_eq!(
        (none.leech_active, none.mastery),
        (Some(0), Some(100.0)),
        "a wired port that answers no leech"
    );
    let three = law_block(db, day(D0), Some(3))
        .await
        .expect("the block reads");
    assert_eq!(
        (three.leech_active, three.mastery),
        (Some(3), Some(91.0)),
        "three active law leeches take nine points off the pillar"
    );
}
