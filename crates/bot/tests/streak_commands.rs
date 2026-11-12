//! The owner's `/streak` (SPEC-076 A21; R22): both tracks, the law track first when it has activity,
//! with the language track's heat and freezes.
//!
//! Each `/streak` goes through the bot's own handlers to a fake Bot API, over a temporary database
//! holding synthetic streak rows, and the bench's manual clock decides the study day.

// An integration test is test code: its helpers panic on a failed check.
#![allow(clippy::expect_used, clippy::print_stdout)]

#[path = "support/fake_bot_api.rs"]
mod fake_bot_api;

use deck_streak_kernel::{Db, UtcMillis};
use fake_bot_api::{Bench, ScriptedSync, incoming, owner_says, payload};
use serde_json::Value;

/// The bench's study day at its start: 2025-01-14, as an epoch day.
const TODAY: i64 = 20_102;
/// A day in milliseconds.
const DAY_MS: i64 = 86_400_000;

/// The streak crate's band table, compiled into this test from the streak crate's own source, so
/// A50's population is derived from the table the reply reads, and a band added to the table grows
/// it. The bot crate has no edge to the streak crate (docs/CONTEXT-MAP.md); this reads the table's
/// constants as data and calls none of that crate's code.
#[allow(dead_code)]
#[path = "../../streaks/src/constants.rs"]
mod streak_table;

/// A50's oracle: each heat band's first run and the heat the language line shows through the band,
/// written out here and never read from the streak crate, so a band the reply gets wrong cannot
/// agree with the oracle it is judged by.
const HEAT_ORACLE: [(u32, &str); 6] = [
    (0, ""),
    (1, " \u{1f525}"),
    (7, " \u{1f525}\u{1f525}"),
    (30, " \u{1f525}\u{1f525}\u{1f525}"),
    (100, " \u{2604}\u{fe0f}"),
    (365, " \u{1f31f}\u{2604}\u{fe0f}\u{1f31f}"),
];
/// A50's leads: a law run of zero (the language line leads) and of two (the law line leads), each
/// with a best of its own.
const LEADS: [(i64, i64); 2] = [(0, 4), (2, 3)];
/// A50's freeze counts: none, one (the singular noun) and two.
const FREEZES: [i64; 3] = [0, 1, 2];
/// Round 4's eight replies, kept in A50's population beside the derived ones: language runs 0, 3,
/// 9 and 45, each with a best in the band above, at one freeze and at two, beside a law run of 2
/// with a best of 2. Each one's heat is read from [`HEAT_ORACLE`], as the derived runs' heats are.
const ROUND_4_REPLIES: [(i64, i64); 4] = [(0, 3), (3, 9), (9, 45), (45, 120)];
/// Round 4's freeze counts: one and two.
const ROUND_4_FREEZES: [i64; 2] = [1, 2];
/// Round 4's law run and its best.
const ROUND_4_LAW: (i64, i64) = (2, 2);

/// One reply of A50's population: the language run, its best and the heat the oracle gives the
/// run, the law run and its best, and the freezes the language track holds.
#[derive(Clone, Copy, Debug)]
struct HeatCase {
    run: i64,
    best: i64,
    heat: &'static str,
    law: i64,
    law_best: i64,
    freezes: i64,
}

/// A50's language runs, derived from the streak crate's band table: each band's first and last
/// run, and the open top band's first and the run after it, each with its band's heat from
/// [`HEAT_ORACLE`]. The count is printed before the oracle is checked, so a band added to the table
/// shows the population it grew to, and a band with no oracle entry, or an entry with no band,
/// fails before any reply is read.
fn heat_runs() -> Vec<(i64, &'static str)> {
    let mut firsts: Vec<u32> = streak_table::STREAK_HEAT
        .iter()
        .map(|(days, _)| *days)
        .collect();
    firsts.sort_unstable();
    firsts.dedup();
    let mut runs: Vec<u32> = Vec::new();
    for (index, first) in firsts.iter().enumerate() {
        let last = firsts.get(index + 1).map_or(first + 1, |next| next - 1);
        for run in [*first, last] {
            if runs.last() != Some(&run) {
                runs.push(run);
            }
        }
    }
    println!(
        "streak-reply heat population derived: {} band(s) of the streak table, {} run(s) x {} \
         leads x {} freeze counts = {} replies",
        firsts.len(),
        runs.len(),
        LEADS.len(),
        FREEZES.len(),
        runs.len() * LEADS.len() * FREEZES.len()
    );
    let oracle: Vec<u32> = HEAT_ORACLE.iter().map(|(first, _)| *first).collect();
    assert_eq!(
        firsts, oracle,
        "every band of the streak table has an oracle entry, and every entry a band"
    );
    runs.into_iter()
        .map(|run| (i64::from(run), oracle_heat(run)))
        .collect()
}

/// The heat [`HEAT_ORACLE`] gives a run: the heat of the last band whose first run it reaches.
fn oracle_heat(run: u32) -> &'static str {
    HEAT_ORACLE
        .iter()
        .rev()
        .find(|(first, _)| run >= *first)
        .map(|(_, heat)| *heat)
        .expect("run 0 opens the first band")
}

/// A50's population: each derived run of `runs` at each of [`LEADS`] and [`FREEZES`], with a
/// best a thousand days above the run, then round 4's eight replies, [`ROUND_4_REPLIES`] at
/// [`ROUND_4_FREEZES`] beside [`ROUND_4_LAW`].
fn heat_cases(runs: &[(i64, &'static str)]) -> Vec<HeatCase> {
    let mut cases = Vec::new();
    for &(run, heat) in runs {
        for (law, law_best) in LEADS {
            for freezes in FREEZES {
                cases.push(HeatCase {
                    run,
                    best: run + 1_000,
                    heat,
                    law,
                    law_best,
                    freezes,
                });
            }
        }
    }
    for (run, best) in ROUND_4_REPLIES {
        let heat = oracle_heat(u32::try_from(run).expect("a run of days"));
        for freezes in ROUND_4_FREEZES {
            cases.push(HeatCase {
                run,
                best,
                heat,
                law: ROUND_4_LAW.0,
                law_best: ROUND_4_LAW.1,
                freezes,
            });
        }
    }
    cases
}

/// Writes the two tracks: the language streak at `language` days with two freezes, and the law
/// streak at `law` days, each last studied the day before.
async fn seed(db: &Db, language: i64, law: i64) {
    let mut write = db.write().await.expect("a write");
    for (track, current, freezes) in [("language", language, 2), ("law", law, 0)] {
        sqlx::query(
            "INSERT INTO streak_state \
             (track, current_days, longest_days, freezes, last_study_day, comeback_armed, created_at) \
             VALUES (?1, ?2, ?2, ?3, ?4, 0, 1000)",
        )
        .bind(track)
        .bind(current)
        .bind(freezes)
        .bind(TODAY - 1)
        .execute(&mut *write)
        .await
        .expect("the synthetic streak row is written");
    }
    write.commit().await.expect("the commit");
}

/// The text of the last `sendMessage`.
fn last_text(bench: &Bench) -> String {
    let sent: Value = payload(&bench.fake.calls_of("sendMessage").pop().expect("a send"));
    sent["text"].as_str().expect("a text").to_owned()
}

#[tokio::test]
async fn streak_shows_both_tracks_law_first_when_law_is_active() {
    let bench = Bench::start().await;
    seed(&bench.db, 12, 4).await;
    bench
        .clock
        .set(UtcMillis::from_epoch_millis(TODAY * DAY_MS + 5 * 3_600_000));
    let mut commands = bench.commands(ScriptedSync::default());

    // The law track is active, so it comes first, and the language track shows its freezes.
    commands.handle(incoming(owner_says(1, "/streak"))).await;
    let text = last_text(&bench);
    let law = text.find("Law: 4").expect("the law track is shown");
    let language = text
        .find("Language: 12")
        .expect("the language track is shown");
    assert!(law < language, "law first in {text}");
    assert!(text.contains("2 freezes"), "the freezes in {text}");
}

/// A rule (A36): the law track leads exactly when its run is above zero, and the language track's
/// noun is singular exactly at one freeze. Law runs 0, 1 and 2 (the boundary -1, 0, +1 of "above
/// zero") over every language freeze count from zero to three, each read through `/streak`.
#[tokio::test]
async fn the_law_leads_only_above_zero_and_the_noun_follows_the_freezes() {
    let mut cases = 0_u32;
    for law in [0_i64, 1, 2] {
        for freezes in 0..=3_i64 {
            let bench = Bench::start().await;
            {
                let mut write = bench.db.write().await.expect("a write");
                for (track, current, held) in [("language", 9_i64, freezes), ("law", law, 0)] {
                    sqlx::query(
                        "INSERT INTO streak_state \
                         (track, current_days, longest_days, freezes, last_study_day, comeback_armed, created_at) \
                         VALUES (?1, ?2, ?2, ?3, ?4, 0, 1000)",
                    )
                    .bind(track)
                    .bind(current)
                    .bind(held)
                    .bind(TODAY - 1)
                    .execute(&mut *write)
                    .await
                    .expect("the synthetic streak row is written");
                }
                write.commit().await.expect("the commit");
            }
            bench
                .clock
                .set(UtcMillis::from_epoch_millis(TODAY * DAY_MS + 5 * 3_600_000));
            let mut commands = bench.commands(ScriptedSync::default());
            commands.handle(incoming(owner_says(1, "/streak"))).await;
            let text = last_text(&bench);
            let law_at = text.find("Law: ").expect("the law line");
            let language_at = text.find("Language: 9").expect("the language line");
            assert_eq!(law_at < language_at, law > 0, "law {law} in {text}");
            let noun = if freezes == 1 {
                "1 freeze)"
            } else {
                "freezes)"
            };
            assert!(text.contains(noun), "freezes {freezes} in {text}");
            assert!(text.contains(&format!("Law: {law} (best {law})")), "{text}");
            assert!(text.starts_with("<b>Streaks</b>\n"), "{text}");
            cases += 1;
        }
    }
    println!("streak-reply population: {cases} replies");
    assert_eq!(cases, 12);
}

/// A rule (A42): each track's line names its own run and its own best, and the law track leads by
/// its run, never by its best. Law runs 0, 1 and 2, each with a best of the run, one more and seven
/// more, over language bests of 9, 10 and 16 on a language run of 9, each pair stored as its own row
/// and read through `/streak`. A run equal to its best hides a swap of the two, and a law run of zero
/// with a best above zero is the state in which leading by the best differs from leading by the run.
#[tokio::test]
async fn each_line_names_its_own_run_and_best_and_the_law_leads_by_its_run() {
    let mut cases = 0_u32;
    let mut idle_law_with_a_best = 0_u32;
    for law in [0_i64, 1, 2] {
        for law_more in [0_i64, 1, 7] {
            for language_more in [0_i64, 1, 7] {
                let law_best = law + law_more;
                let language_best = 9 + language_more;
                let bench = Bench::start().await;
                {
                    let mut write = bench.db.write().await.expect("a write");
                    for (track, current, longest, held) in [
                        ("language", 9_i64, language_best, 2_i64),
                        ("law", law, law_best, 0),
                    ] {
                        sqlx::query(
                            "INSERT INTO streak_state \
                             (track, current_days, longest_days, freezes, last_study_day, comeback_armed, created_at) \
                             VALUES (?1, ?2, ?3, ?4, ?5, 0, 1000)",
                        )
                        .bind(track)
                        .bind(current)
                        .bind(longest)
                        .bind(held)
                        .bind(TODAY - 1)
                        .execute(&mut *write)
                        .await
                        .expect("the synthetic streak row is written");
                    }
                    write.commit().await.expect("the commit");
                }
                bench
                    .clock
                    .set(UtcMillis::from_epoch_millis(TODAY * DAY_MS + 5 * 3_600_000));
                let mut commands = bench.commands(ScriptedSync::default());
                commands.handle(incoming(owner_says(1, "/streak"))).await;
                let text = last_text(&bench);
                let lines: Vec<&str> = text.split('\n').collect();
                assert_eq!(lines.len(), 3, "{text}");
                assert_eq!(lines[0], "<b>Streaks</b>", "{text}");
                let (law_line, language_line) = if law > 0 {
                    (lines[1], lines[2])
                } else {
                    (lines[2], lines[1])
                };
                assert_eq!(
                    law_line,
                    format!("Law: {law} (best {law_best})"),
                    "law {law} best {law_best} in {text}"
                );
                assert!(
                    language_line.starts_with("Language: 9"),
                    "language best {language_best} in {text}"
                );
                assert!(
                    language_line.ends_with(&format!(" (best {language_best}, 2 freezes)")),
                    "language best {language_best} in {text}"
                );
                cases += 1;
                if law == 0 && law_best > 0 {
                    idle_law_with_a_best += 1;
                }
            }
        }
    }
    println!(
        "streak-reply best population: {cases} replies, {idle_law_with_a_best} with an idle law \
         run and a best"
    );
    assert_eq!(cases, 27);
    assert_eq!(idle_law_with_a_best, 6);
}

/// A rule (A50): the language line carries its run's heat in the heat's own place, between the run
/// and the best, and nothing there at a run of zero. The population is derived from the streak
/// crate's band table: each band's first and last run, the open top band's first and the run after
/// it, whichever line leads (a law run of zero and of two), at no freeze, one and two, and round 4's
/// eight replies are kept in it. Each value on the line differs from the heat, and the whole of
/// both lines is read. A band added to the table adds its runs here; a band with no oracle entry
/// fails before any reply is read.
#[tokio::test]
async fn the_language_line_carries_its_runs_heat_in_the_heats_own_place() {
    let runs = heat_runs();
    let population = heat_cases(&runs);
    let mut cases = 0_usize;
    let mut heats = std::collections::BTreeSet::new();
    for HeatCase {
        run,
        best,
        heat,
        law,
        law_best,
        freezes,
    } in population
    {
        let bench = Bench::start().await;
        {
            let mut write = bench.db.write().await.expect("a write");
            for (track, current, longest, held) in [
                ("language", run, best, freezes),
                ("law", law, law_best, 0_i64),
            ] {
                sqlx::query(
                    "INSERT INTO streak_state \
                     (track, current_days, longest_days, freezes, last_study_day, comeback_armed, created_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, 0, 1000)",
                )
                .bind(track)
                .bind(current)
                .bind(longest)
                .bind(held)
                .bind(TODAY - 1)
                .execute(&mut *write)
                .await
                .expect("the synthetic streak row is written");
            }
            write.commit().await.expect("the commit");
        }
        bench
            .clock
            .set(UtcMillis::from_epoch_millis(TODAY * DAY_MS + 5 * 3_600_000));
        let mut commands = bench.commands(ScriptedSync::default());
        commands.handle(incoming(owner_says(1, "/streak"))).await;
        let text = last_text(&bench);
        let lines: Vec<&str> = text.split('\n').collect();
        assert_eq!(lines.len(), 3, "{text}");
        // The law line leads exactly when the law run is above zero.
        let (law_line, language_line) = if law > 0 {
            (lines[1], lines[2])
        } else {
            (lines[2], lines[1])
        };
        assert_eq!(law_line, format!("Law: {law} (best {law_best})"), "{text}");
        let noun = if freezes == 1 { "freeze" } else { "freezes" };
        assert_eq!(
            language_line,
            format!("Language: {run}{heat} (best {best}, {freezes} {noun})"),
            "language run {run}, law run {law}, {freezes} freeze(s) in {text}"
        );
        heats.insert(heat);
        cases += 1;
    }
    let kept = ROUND_4_REPLIES.len() * ROUND_4_FREEZES.len();
    println!(
        "streak-reply heat population: {cases} replies over {} heats, {kept} of them round 4's",
        heats.len()
    );
    assert_eq!(
        cases,
        runs.len() * LEADS.len() * FREEZES.len() + kept,
        "every derived run, lead and freeze count was read, and round 4's replies"
    );
    assert_eq!(heats.len(), HEAT_ORACLE.len(), "every band's heat was read");
}

/// Verifier round 4's killer (item 2): the language line carries its run's heat in the heat's own
/// place over every heat band of the streak crate's table, at each band's first and last run (the
/// top band's first and one beyond), whichever line leads (a law run of zero and of two), at no
/// freeze, one and two. The heats are written out here, one per band, not read from the streak
/// crate.
#[tokio::test]
async fn every_heat_band_is_carried_in_the_heats_own_place_whichever_line_leads() {
    let fire = "\u{1f525}";
    let comet = " \u{2604}\u{fe0f}".to_owned();
    let star = " \u{1f31f}\u{2604}\u{fe0f}\u{1f31f}".to_owned();
    let runs: [(i64, String); 11] = [
        (0, String::new()),
        (1, format!(" {fire}")),
        (6, format!(" {fire}")),
        (7, format!(" {fire}{fire}")),
        (29, format!(" {fire}{fire}")),
        (30, format!(" {fire}{fire}{fire}")),
        (99, format!(" {fire}{fire}{fire}")),
        (100, comet.clone()),
        (364, comet),
        (365, star.clone()),
        (400, star),
    ];
    let mut cases = 0_u32;
    let mut heats = std::collections::BTreeSet::new();
    for (run, heat) in &runs {
        let best = run + 1_000;
        for (law, law_best) in [(0_i64, 4_i64), (2, 3)] {
            for freezes in [0_i64, 1, 2] {
                let bench = Bench::start().await;
                {
                    let mut write = bench.db.write().await.expect("a write");
                    for (track, current, longest, held) in [
                        ("language", *run, best, freezes),
                        ("law", law, law_best, 0_i64),
                    ] {
                        sqlx::query(
                            "INSERT INTO streak_state \
                             (track, current_days, longest_days, freezes, last_study_day, comeback_armed, created_at) \
                             VALUES (?1, ?2, ?3, ?4, ?5, 0, 1000)",
                        )
                        .bind(track)
                        .bind(current)
                        .bind(longest)
                        .bind(held)
                        .bind(TODAY - 1)
                        .execute(&mut *write)
                        .await
                        .expect("the synthetic streak row is written");
                    }
                    write.commit().await.expect("the commit");
                }
                bench
                    .clock
                    .set(UtcMillis::from_epoch_millis(TODAY * DAY_MS + 5 * 3_600_000));
                let mut commands = bench.commands(ScriptedSync::default());
                commands.handle(incoming(owner_says(1, "/streak"))).await;
                let text = last_text(&bench);
                let lines: Vec<&str> = text.split('\n').collect();
                assert_eq!(lines.len(), 3, "{text}");
                let (law_line, language_line) = if law > 0 {
                    (lines[1], lines[2])
                } else {
                    (lines[2], lines[1])
                };
                assert_eq!(law_line, format!("Law: {law} (best {law_best})"), "{text}");
                let noun = if freezes == 1 { "freeze" } else { "freezes" };
                assert_eq!(
                    language_line,
                    format!("Language: {run}{heat} (best {best}, {freezes} {noun})"),
                    "language run {run}, law run {law}, {freezes} freeze(s) in {text}"
                );
                heats.insert(heat.clone());
                cases += 1;
            }
        }
    }
    println!(
        "streak-reply heat population (verifier): {cases} replies over {} heats",
        heats.len()
    );
    assert_eq!(cases, 66);
    assert_eq!(heats.len(), 6, "the empty heat and the five bands");
}
